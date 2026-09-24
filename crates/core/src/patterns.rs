//! Pattern dictionary types and bundled TOML loading.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, de};

use crate::detector::char_class::{EM_DASH_ADDICTION, UNICODE_DECORATION_RULE_ID};
use crate::detector::cross_file::CROSS_FILE_DUPLICATION;
use crate::detector::markdown::BOLD_FIRST_LEADS;
use crate::detector::repetition::{CONTENT_DUPLICATION, DEAD_METAPHOR, ONE_POINT_DILUTION};
use crate::detector::structural::{
    ANAPHORA_ABUSE, FRACTAL_SUMMARIES, HISTORICAL_ANALOGY_STACKING, LISTICLE_IN_TRENCH_COAT,
    SHORT_PUNCHY_FRAGMENTS, TRICOLON_ABUSE,
};
use crate::errors::{PatternLoadError, PatternValidationError};

// The counts `Thresholds` holds, down to the leaves, and the dialect
// `PatternFile` names. The detectors that read them are private to the crate,
// so this is where a consumer names them.
pub use crate::detector::char_class::{DashLimits, DecorationLimits};
pub use crate::detector::cross_file::CrossFileLimits;
pub use crate::detector::dialect::Dialect;
pub use crate::detector::markdown::{BoldLeadLimits, MarkdownOptions};
pub use crate::detector::repetition::{
    DeadMetaphorLimits, DilutionLimits, DuplicationLimits, RepetitionLimits,
};
pub use crate::detector::structural::{
    AnalogyLimits, AnaphoraLimits, FragmentLimits, ListicleLimits, StructuralLimits, SummaryLimits,
    TricolonLimits,
};

/// File names a project dictionary is discovered under, in search order.
pub const PROJECT_DICTIONARY_FILES: &[&str] = &["trps.toml", "tropes.toml", "tropius.toml"];

/// A published catalog a bundled pattern takes its phrases from.
///
/// Every bundled pattern names one or more of these in its `sources` list, so
/// a reader asking where a rule came from reads the answer beside the rule.
/// `meta/sources.md` carries each catalog's license and copyright holder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source {
    /// Key a pattern cites, such as `tropes.fyi`.
    pub key: &'static str,
    /// Where the catalog is published.
    pub url: &'static str,
}

/// Catalogs a bundled pattern may cite.
pub const SOURCES: &[Source] = &[
    Source {
        key: "tropes.fyi",
        url: "https://tropes.fyi",
    },
    Source {
        key: "humanizer",
        url: "https://github.com/blader/humanizer",
    },
    Source {
        key: "avoid-ai-writing",
        url: "https://github.com/conorbronsdon/avoid-ai-writing",
    },
    Source {
        key: "clearmode",
        url: "https://github.com/eugeniughelbur/clearmode",
    },
    Source {
        key: "vale-llm-slop",
        url: "https://github.com/Syntaf/vale-llm-slop",
    },
    Source {
        key: "vale-ai-slop",
        url: "https://github.com/stuffbucket/vale/tree/main/research/ai-slop",
    },
    Source {
        key: "slop-forensics",
        url: "https://github.com/sam-paech/slop-forensics",
    },
];

/// Returns the catalog registered under `key`.
pub fn source(key: &str) -> Option<&'static Source> {
    SOURCES.iter().find(|source| source.key == key)
}

/// TOML files bundled into `trps-core`.
pub const BUNDLED_PATTERN_FILES: &[(&str, &str)] = &[
    ("formatting.toml", include_str!("patterns/formatting.toml")),
    (
        "sentence-structure.toml",
        include_str!("patterns/sentence-structure.toml"),
    ),
    (
        "composition.toml",
        include_str!("patterns/composition.toml"),
    ),
    ("tone.toml", include_str!("patterns/tone.toml")),
    (
        "word-choice.toml",
        include_str!("patterns/word-choice.toml"),
    ),
    (
        "assistant-voice.toml",
        include_str!("patterns/assistant-voice.toml"),
    ),
    (
        "technical-prose.toml",
        include_str!("patterns/technical-prose.toml"),
    ),
    ("narrative.toml", include_str!("patterns/narrative.toml")),
];

/// Severity attached to a pattern or detector finding.
///
/// The variants are ordered as a scale, `Low < Medium < High`, so a consumer
/// can keep the findings it cares about with a comparison. That ordering is
/// part of the API: the variants stay in this order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Low-confidence or low-impact signal.
    Low,
    /// Medium-confidence signal.
    Medium,
    /// High-confidence signal.
    High,
}

impl Severity {
    /// Returns the report symbol for this severity.
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Low => "ℹ",
            Self::Medium => "⚠",
            Self::High => "✕",
        }
    }
}

/// A deserialized TOML pattern file.
///
/// The bundled dictionaries and a project dictionary share this shape. Bundled
/// files declare patterns only; a project dictionary can also list phrases to
/// remove from them and paths to keep out of a scan.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternFile {
    /// Phrases removed from the patterns this file is applied to, matched
    /// case-insensitively.
    #[serde(default)]
    pub allow: Vec<String>,
    /// The English dialect the project writes in, where it names one.
    ///
    /// Naming one turns on `word_choice.dialect_spelling`, which reports
    /// every spelling the other dialect uses. The rule is off while this is
    /// unset, because neither dialect is the one a project must write in.
    #[serde(default)]
    pub dialect: Option<Dialect>,
    /// Globs naming paths no scan reads, relative to the directory holding
    /// this file. See [`crate::excludes::Excludes`].
    #[serde(default)]
    pub exclude: Vec<String>,
    /// What the project decides about reading Markdown. See
    /// [`MarkdownOptions`].
    #[serde(default)]
    pub markdown: MarkdownOptions,
    /// Catalogs this file's patterns may cite, by key and URL.
    ///
    /// A bundled pattern cites [`SOURCES`], which is compiled in. A project
    /// keeping its own catalog registers it here, and its patterns cite the
    /// key the same way. See [`validate_declared_sources`].
    #[serde(default)]
    pub sources: BTreeMap<String, String>,
    /// Pattern entries declared by the file.
    #[serde(default)]
    pub patterns: Vec<Pattern>,
    /// The counts the project tunes. See [`Thresholds`].
    #[serde(default)]
    pub thresholds: Thresholds,
}

impl PatternFile {
    /// Deserializes a pattern file from TOML text.
    pub fn from_toml(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
    }
}

/// The counts a project tunes, keyed by the rule id a finding prints.
///
/// A rule that fires at a count reads it from here, so a project that finds
/// one too loose or too strict raises it rather than silencing the rule.
/// Every entry defaults to the count the tool ships with, so a dictionary
/// setting none of them scans as it does now.
///
/// A rule id holds a dot, so a header written without quotes is a table
/// nested a segment at a time: `[thresholds.composition.cross_file_duplication]`
/// and `[thresholds."composition.cross_file_duplication"]` are different TOML
/// and the same rule. Both reach the same entry, because a project that writes
/// the obvious one is tuning the rule either way.
///
/// A key naming no rule is kept rather than rejected. The keys are rule ids
/// rather than fields, so `deny_unknown_fields` would fail the dictionary
/// over a typo; [`Thresholds::unknown_rules`] hands the key to a run to warn
/// about instead, spelled as the whole id rather than the segment it starts
/// with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Thresholds {
    /// What counts as a repetition several files share, under
    /// `composition.cross_file_duplication`. See [`CrossFileLimits`].
    pub cross_file_duplication: CrossFileLimits,
    /// What counts as a run of bolded leads, under
    /// `formatting.bold_first_leads`. See [`BoldLeadLimits`].
    pub bold_first_leads: BoldLeadLimits,
    /// What counts as dash use dense enough to read as a habit, under
    /// `formatting.em_dash_addiction`. See [`DashLimits`].
    pub em_dash_addiction: DashLimits,
    /// What counts as decoration repeated often enough to report, under
    /// `formatting.unicode_decoration`. See [`DecorationLimits`].
    pub unicode_decoration: DecorationLimits,
    /// The counts the repetition rules fire at, one field per rule id. They
    /// are grouped because one scan reads them together. See
    /// [`RepetitionLimits`].
    pub repetition: RepetitionLimits,
    /// The counts the structural rules fire at, one field per rule id. They
    /// are grouped for the same reason. See [`StructuralLimits`].
    pub structural: StructuralLimits,
    unknown: Vec<String>,
}

impl Thresholds {
    /// The keys of the table that name no rule.
    pub fn unknown_rules(&self) -> impl Iterator<Item = &str> {
        self.unknown.iter().map(String::as_str)
    }

    /// Reads the entries of one table, where `prefix` is the part of the rule
    /// id its keys hang under and is empty for `[thresholds]` itself.
    ///
    /// A key that finishes a rule id is that rule's entry. A key that finishes
    /// none and holds nothing but tables is a segment of a longer id, so it is
    /// joined to the keys beneath it and read again. Anything else names no
    /// rule and is kept for a run to warn about.
    fn read<E: de::Error>(&mut self, prefix: &str, table: toml::Table) -> Result<(), E> {
        for (key, value) in table {
            let rule = match prefix.is_empty() {
                true => key,
                false => format!("{prefix}.{key}"),
            };

            match rule.as_str() {
                id if id == CROSS_FILE_DUPLICATION.0 => self.cross_file_duplication = entry(value)?,
                id if id == BOLD_FIRST_LEADS.0 => self.bold_first_leads = entry(value)?,
                id if id == EM_DASH_ADDICTION.0 => self.em_dash_addiction = entry(value)?,
                id if id == UNICODE_DECORATION_RULE_ID => self.unicode_decoration = entry(value)?,
                id if id == CONTENT_DUPLICATION.0 => {
                    self.repetition.content_duplication = entry(value)?
                }
                id if id == DEAD_METAPHOR.0 => self.repetition.dead_metaphor = entry(value)?,
                id if id == ONE_POINT_DILUTION.0 => {
                    self.repetition.one_point_dilution = entry(value)?
                }
                id if id == ANAPHORA_ABUSE.0 => self.structural.anaphora_abuse = entry(value)?,
                id if id == FRACTAL_SUMMARIES.0 => {
                    self.structural.fractal_summaries = entry(value)?
                }
                id if id == HISTORICAL_ANALOGY_STACKING.0 => {
                    self.structural.historical_analogy_stacking = entry(value)?
                }
                id if id == LISTICLE_IN_TRENCH_COAT.0 => {
                    self.structural.listicle_in_trench_coat = entry(value)?
                }
                id if id == SHORT_PUNCHY_FRAGMENTS.0 => {
                    self.structural.short_punchy_fragments = entry(value)?
                }
                id if id == TRICOLON_ABUSE.0 => self.structural.tricolon_abuse = entry(value)?,
                _ => match value {
                    toml::Value::Table(nested) if groups_rules(&nested) => {
                        self.read(&rule, nested)?
                    }
                    _ => self.unknown.push(rule),
                },
            }
        }

        Ok(())
    }
}

/// Reads one rule's entry from the value written under its id.
fn entry<T: de::DeserializeOwned, E: de::Error>(value: toml::Value) -> Result<T, E> {
    value.try_into().map_err(de::Error::custom)
}

/// Whether a table under a key naming no rule holds the rest of longer ids.
///
/// Every count a rule tunes is a plain value, so a table of nothing but tables
/// carries no entry of its own and is the middle of a dotted header. An empty
/// table carries nothing either way, and reads as the entry it was written as
/// so the key it names is warned about rather than dropped.
fn groups_rules(table: &toml::Table) -> bool {
    !table.is_empty() && table.values().all(toml::Value::is_table)
}

impl<'de> Deserialize<'de> for Thresholds {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut thresholds = Self::default();

        thresholds.read("", toml::Table::deserialize(deserializer)?)?;

        Ok(thresholds)
    }
}

/// One phrase-based trope pattern.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Pattern {
    /// Stable dotted id, such as `word_choice.delve`.
    pub id: String,
    /// Human-readable pattern name.
    pub name: String,
    /// Pattern severity.
    pub severity: Severity,
    /// Catalogs the phrases came from, by [`Source::key`].
    ///
    /// A bundled pattern names at least one registered key, under
    /// [`validate_sources`]. A project dictionary may cite whatever it likes,
    /// including nothing.
    #[serde(default)]
    pub sources: Vec<String>,
    /// Literal phrases matched case-insensitively by the phrase detector.
    pub phrases: Vec<String>,
}

/// Loads and validates the pattern dictionaries bundled with the crate.
pub fn bundled_patterns() -> Result<Vec<Pattern>, PatternLoadError> {
    let mut patterns = Vec::new();

    for (_, input) in BUNDLED_PATTERN_FILES {
        patterns.extend(PatternFile::from_toml(input)?.patterns);
    }

    validate_patterns(&patterns)?;
    validate_sources(&patterns)?;

    Ok(patterns)
}

/// Reads a pattern file from `path`.
pub fn load_pattern_file(path: &Path) -> Result<PatternFile, PatternLoadError> {
    let input = fs::read_to_string(path).map_err(|source| PatternLoadError::Read {
        path: path.display().to_string(),
        source,
    })?;

    Ok(PatternFile::from_toml(&input)?)
}

/// Searches `start` and its ancestors for a project dictionary.
///
/// A project dictionary belongs to a repository, so the search stops at the
/// directory holding `.git` and never reads one above it. Outside a repository
/// only `start` is searched, which keeps a file dropped in a shared temporary
/// directory out of every scan run from under it.
///
/// The nearest directory holding one wins, and within a directory the first
/// name in [`PROJECT_DICTIONARY_FILES`] wins.
pub fn find_project_dictionary(start: &Path) -> Option<PathBuf> {
    let repository_root = start
        .ancestors()
        .find(|directory| directory.join(".git").exists());

    for directory in start.ancestors() {
        let found = PROJECT_DICTIONARY_FILES
            .iter()
            .map(|name| directory.join(name))
            .find(|path| path.is_file());

        if found.is_some() {
            return found;
        }

        if repository_root.is_none_or(|root| root == directory) {
            return None;
        }
    }

    None
}

/// Applies a project dictionary to `base`.
///
/// Allowed phrases are removed from every pattern in `base`, and a pattern left
/// with no phrases is dropped. A pattern the dictionary declares takes the
/// place of the one in `base` with the same id.
///
/// Declared patterns come first in the returned list. The phrase matcher
/// resolves two phrases starting at one offset in favour of the earlier
/// pattern, so a declared phrase wins an overlap with a bundled one rather than
/// being shadowed by it. Declared ids and phrases are left as the dictionary
/// wrote them, so [`validate_patterns`] reports a dictionary that repeats an id
/// the way it reports one in a bundled file.
pub fn apply_dictionary(base: Vec<Pattern>, dictionary: &PatternFile) -> Vec<Pattern> {
    let allowed: HashSet<String> = dictionary
        .allow
        .iter()
        .map(|phrase| normalize(phrase))
        .collect();
    let declared: HashSet<&str> = dictionary
        .patterns
        .iter()
        .map(|pattern| pattern.id.as_str())
        .collect();

    let mut patterns = dictionary.patterns.clone();

    patterns.extend(base.into_iter().filter_map(|mut pattern| {
        if declared.contains(pattern.id.as_str()) {
            return None;
        }

        pattern
            .phrases
            .retain(|phrase| !allowed.contains(&normalize(phrase)));

        (!pattern.phrases.is_empty()).then_some(pattern)
    }));

    patterns
}

fn normalize(phrase: &str) -> String {
    phrase.trim().to_ascii_lowercase()
}

/// A pattern a project dictionary declared.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DeclaredPattern {
    /// The id the dictionary gave it.
    pub id: String,
    /// Whether it took the place of a bundled pattern with the same id.
    pub replaces_bundled: bool,
}

/// A phrase a project dictionary allowed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AllowedPhrase {
    /// The phrase as the dictionary wrote it.
    pub phrase: String,
    /// The bundled patterns it was taken out of.
    ///
    /// Empty where the phrase matched none, which is the common way a
    /// dictionary entry does nothing.
    pub patterns: Vec<String>,
}

/// Reports what `dictionary` does to `base`, without applying it.
///
/// [`apply_dictionary`] returns the patterns a run scans with, and a pattern
/// it dropped or replaced leaves no trace in them. A reader asking whether the
/// dictionary did anything needs the trace, so this walks the same two
/// decisions, in the order they are applied, and names what each one touched.
/// A phrase allowed out of a pattern the same dictionary replaced took
/// nothing with it, and is reported as having matched nothing.
pub fn describe_dictionary(
    base: &[Pattern],
    dictionary: &PatternFile,
) -> (Vec<DeclaredPattern>, Vec<AllowedPhrase>) {
    let declared = dictionary
        .patterns
        .iter()
        .map(|pattern| DeclaredPattern {
            id: pattern.id.clone(),
            replaces_bundled: base.iter().any(|bundled| bundled.id == pattern.id),
        })
        .collect();

    let replaced: HashSet<&str> = dictionary
        .patterns
        .iter()
        .map(|pattern| pattern.id.as_str())
        .collect();

    let allowed = dictionary
        .allow
        .iter()
        .map(|phrase| {
            let wanted = normalize(phrase);

            AllowedPhrase {
                phrase: phrase.clone(),
                patterns: base
                    .iter()
                    .filter(|pattern| !replaced.contains(pattern.id.as_str()))
                    .filter(|pattern| {
                        pattern
                            .phrases
                            .iter()
                            .any(|candidate| normalize(candidate) == wanted)
                    })
                    .map(|pattern| pattern.id.clone())
                    .collect(),
            }
        })
        .collect();

    (declared, allowed)
}

/// Validates the citations on the patterns a project dictionary declares.
///
/// A declared pattern cites whatever the dictionary registered under
/// `[sources]`, or one of the bundled [`SOURCES`]. Citing neither is the same
/// mistake a bundled pattern makes when it cites a key nothing resolves, and
/// gets the same error, so a citation stays something a reader can follow.
///
/// A pattern citing nothing at all is left alone. Where a project's phrases
/// came from is the project's business; where they say they came from has to
/// be true.
pub fn validate_declared_sources(dictionary: &PatternFile) -> Result<(), PatternValidationError> {
    for pattern in &dictionary.patterns {
        for key in &pattern.sources {
            if source(key).is_none() && !dictionary.sources.contains_key(key) {
                return Err(PatternValidationError::UnknownSource {
                    id: pattern.id.clone(),
                    key: key.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Validates the citations on patterns bundled with the crate.
///
/// A bundled pattern has to say where its phrases came from, and has to say it
/// in a key [`source`] resolves, so a citation cannot rot into a free-text
/// note nobody can follow. Patterns a project dictionary declares are not
/// checked: their citation, if any, belongs to that project.
pub fn validate_sources(patterns: &[Pattern]) -> Result<(), PatternValidationError> {
    for pattern in patterns {
        if pattern.sources.is_empty() {
            return Err(PatternValidationError::MissingSource {
                id: pattern.id.clone(),
            });
        }

        for key in &pattern.sources {
            if source(key).is_none() {
                return Err(PatternValidationError::UnknownSource {
                    id: pattern.id.clone(),
                    key: key.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Validates pattern ids and phrases across all loaded pattern files.
pub fn validate_patterns(patterns: &[Pattern]) -> Result<(), PatternValidationError> {
    let mut ids = HashSet::new();
    let mut phrases = HashSet::new();

    for pattern in patterns {
        if pattern.id.trim().is_empty() {
            return Err(PatternValidationError::EmptyPatternId);
        }

        if pattern.name.trim().is_empty() {
            return Err(PatternValidationError::EmptyPatternName {
                id: pattern.id.clone(),
            });
        }

        if pattern.phrases.is_empty() {
            return Err(PatternValidationError::EmptyPhraseList {
                id: pattern.id.clone(),
            });
        }

        if !ids.insert(pattern.id.as_str()) {
            return Err(PatternValidationError::DuplicatePatternId {
                id: pattern.id.clone(),
            });
        }

        for phrase in &pattern.phrases {
            if phrase.trim().is_empty() {
                return Err(PatternValidationError::EmptyPhrase {
                    id: pattern.id.clone(),
                });
            }

            let normalized = normalize(phrase);

            if !phrases.insert(normalized) {
                return Err(PatternValidationError::DuplicatePhrase {
                    phrase: phrase.clone(),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::repetition::{DeadMetaphorLimits, DilutionLimits, DuplicationLimits};
    use crate::detector::structural::{
        AnalogyLimits, AnaphoraLimits, FragmentLimits, ListicleLimits, SummaryLimits,
        TricolonLimits,
    };

    #[test]
    fn deserializes_pattern_file() {
        let file = PatternFile::from_toml(
            r#"
[[patterns]]
id = "word_choice.delve"
name = "Delve and Friends"
severity = "medium"
phrases = ["delve into", "robust"]
"#,
        )
        .unwrap();

        assert_eq!(file.patterns.len(), 1);

        let pattern = &file.patterns[0];
        assert_eq!(pattern.id, "word_choice.delve");
        assert_eq!(pattern.name, "Delve and Friends");
        assert_eq!(pattern.severity, Severity::Medium);
        assert_eq!(pattern.phrases, ["delve into", "robust"]);
    }

    #[test]
    fn rejects_unknown_severity() {
        let error = PatternFile::from_toml(
            r#"
[[patterns]]
id = "word_choice.delve"
name = "Delve and Friends"
severity = "extreme"
phrases = ["delve into"]
"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown variant"));
    }

    #[test]
    fn severity_orders_from_low_to_high() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
    }

    #[test]
    fn deserializes_bundled_pattern_files() {
        let patterns = bundled_patterns().unwrap();

        assert!(
            patterns
                .iter()
                .any(|pattern| pattern.id == "word_choice.delve")
        );
        assert!(
            patterns
                .iter()
                .any(|pattern| pattern.id == "sentence_structure.negative_parallelism")
        );
    }

    #[test]
    fn rejects_a_bundled_pattern_that_cites_nothing() {
        let patterns = vec![Pattern {
            id: "word_choice.delve".to_owned(),
            name: "Delve".to_owned(),
            severity: Severity::Medium,
            sources: Vec::new(),
            phrases: vec!["delve into".to_owned()],
        }];

        assert_eq!(
            validate_sources(&patterns),
            Err(PatternValidationError::MissingSource {
                id: "word_choice.delve".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_a_bundled_pattern_that_cites_an_unregistered_source() {
        let patterns = vec![Pattern {
            id: "word_choice.delve".to_owned(),
            name: "Delve".to_owned(),
            severity: Severity::Medium,
            sources: vec!["tropes.fyi".to_owned(), "a-blog-post".to_owned()],
            phrases: vec!["delve into".to_owned()],
        }];

        assert_eq!(
            validate_sources(&patterns),
            Err(PatternValidationError::UnknownSource {
                id: "word_choice.delve".to_owned(),
                key: "a-blog-post".to_owned(),
            })
        );
    }

    #[test]
    fn a_project_pattern_needs_no_citation() {
        let dictionary = PatternFile::from_toml(
            r#"
[[patterns]]
id = "project.bounded"
name = "Bounded Without a Bound"
severity = "high"
phrases = ["bounded"]
"#,
        )
        .unwrap();

        assert!(dictionary.patterns[0].sources.is_empty());
        assert!(validate_patterns(&dictionary.patterns).is_ok());
    }

    #[test]
    fn rejects_duplicate_pattern_ids() {
        let patterns = vec![
            Pattern {
                id: "word_choice.delve".to_owned(),
                name: "Delve".to_owned(),
                severity: Severity::Medium,
                sources: Vec::new(),
                phrases: vec!["delve into".to_owned()],
            },
            Pattern {
                id: "word_choice.delve".to_owned(),
                name: "Delve Again".to_owned(),
                severity: Severity::Medium,
                sources: Vec::new(),
                phrases: vec!["delving deeper".to_owned()],
            },
        ];

        assert_eq!(
            validate_patterns(&patterns),
            Err(PatternValidationError::DuplicatePatternId {
                id: "word_choice.delve".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_duplicate_phrases_case_insensitively() {
        let patterns = vec![
            Pattern {
                id: "one".to_owned(),
                name: "One".to_owned(),
                severity: Severity::Medium,
                sources: Vec::new(),
                phrases: vec!["Delve Into".to_owned()],
            },
            Pattern {
                id: "two".to_owned(),
                name: "Two".to_owned(),
                severity: Severity::Medium,
                sources: Vec::new(),
                phrases: vec!["delve into".to_owned()],
            },
        ];

        assert_eq!(
            validate_patterns(&patterns),
            Err(PatternValidationError::DuplicatePhrase {
                phrase: "delve into".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_empty_pattern_ids() {
        let patterns = vec![Pattern {
            id: " ".to_owned(),
            name: "Empty".to_owned(),
            severity: Severity::Medium,
            sources: Vec::new(),
            phrases: vec!["delve into".to_owned()],
        }];

        assert_eq!(
            validate_patterns(&patterns),
            Err(PatternValidationError::EmptyPatternId)
        );
    }

    #[test]
    fn rejects_empty_pattern_names() {
        let patterns = vec![Pattern {
            id: "word_choice.delve".to_owned(),
            name: " ".to_owned(),
            severity: Severity::Medium,
            sources: Vec::new(),
            phrases: vec!["delve into".to_owned()],
        }];

        assert_eq!(
            validate_patterns(&patterns),
            Err(PatternValidationError::EmptyPatternName {
                id: "word_choice.delve".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_empty_phrase_lists() {
        let patterns = vec![Pattern {
            id: "word_choice.delve".to_owned(),
            name: "Delve".to_owned(),
            severity: Severity::Medium,
            sources: Vec::new(),
            phrases: Vec::new(),
        }];

        assert_eq!(
            validate_patterns(&patterns),
            Err(PatternValidationError::EmptyPhraseList {
                id: "word_choice.delve".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_empty_phrases() {
        let patterns = vec![Pattern {
            id: "word_choice.delve".to_owned(),
            name: "Delve".to_owned(),
            severity: Severity::Medium,
            sources: Vec::new(),
            phrases: vec![" ".to_owned()],
        }];

        assert_eq!(
            validate_patterns(&patterns),
            Err(PatternValidationError::EmptyPhrase {
                id: "word_choice.delve".to_owned(),
            })
        );
    }

    #[test]
    fn allowlist_removes_a_phrase_from_a_bundled_pattern() {
        let dictionary = PatternFile::from_toml(r#"allow = ["harness"]"#).unwrap();
        let patterns = apply_dictionary(bundled_patterns().unwrap(), &dictionary);
        let delve = patterns
            .iter()
            .find(|pattern| pattern.id == "word_choice.delve")
            .unwrap();

        assert!(!delve.phrases.iter().any(|phrase| phrase == "harness"));
        assert!(delve.phrases.iter().any(|phrase| phrase == "delve into"));
    }

    #[test]
    fn allowlist_matching_ignores_case() {
        let dictionary = PatternFile::from_toml(r#"allow = ["Harness"]"#).unwrap();
        let patterns = apply_dictionary(bundled_patterns().unwrap(), &dictionary);
        let delve = patterns
            .iter()
            .find(|pattern| pattern.id == "word_choice.delve")
            .unwrap();

        assert!(!delve.phrases.iter().any(|phrase| phrase == "harness"));
    }

    #[test]
    fn a_pattern_whose_phrases_are_all_allowed_is_dropped() {
        let base = vec![Pattern {
            id: "word_choice.delve".to_owned(),
            name: "Delve".to_owned(),
            severity: Severity::Medium,
            sources: Vec::new(),
            phrases: vec!["harness".to_owned()],
        }];
        let dictionary = PatternFile::from_toml(r#"allow = ["harness"]"#).unwrap();

        assert!(apply_dictionary(base, &dictionary).is_empty());
    }

    #[test]
    fn a_dictionary_registers_a_source_its_patterns_can_cite() {
        let dictionary = PatternFile::from_toml(
            r#"
[sources]
house-style = "https://wiki.example.com/style"

[[patterns]]
id = "house.jargon"
name = "House jargon"
severity = "medium"
sources = ["house-style", "tropes.fyi"]
phrases = ["synergize"]
"#,
        )
        .unwrap();

        assert_eq!(
            dictionary.sources.get("house-style").map(String::as_str),
            Some("https://wiki.example.com/style")
        );
        assert!(validate_declared_sources(&dictionary).is_ok());
    }

    #[test]
    fn a_declared_pattern_citing_an_unregistered_source_is_rejected() {
        let dictionary = PatternFile::from_toml(
            r#"
[[patterns]]
id = "house.jargon"
name = "House jargon"
severity = "medium"
sources = ["wiki"]
phrases = ["synergize"]
"#,
        )
        .unwrap();

        assert!(matches!(
            validate_declared_sources(&dictionary),
            Err(PatternValidationError::UnknownSource { key, .. }) if key == "wiki"
        ));
    }

    #[test]
    fn the_description_names_what_each_dictionary_entry_touched() {
        let base = bundled_patterns().unwrap();
        let dictionary = PatternFile::from_toml(
            r#"
allow = ["delve into", "nothing answers to this"]

[[patterns]]
id = "house.jargon"
name = "House jargon"
severity = "medium"
phrases = ["synergize"]
"#,
        )
        .unwrap();
        let (declared, allowed) = describe_dictionary(&base, &dictionary);

        assert_eq!(declared.len(), 1);
        assert_eq!(declared[0].id, "house.jargon");
        assert!(!declared[0].replaces_bundled);

        assert_eq!(allowed[0].phrase, "delve into");
        assert_eq!(allowed[0].patterns, ["word_choice.delve"]);
        assert_eq!(allowed[1].phrase, "nothing answers to this");
        assert!(allowed[1].patterns.is_empty());
    }

    #[test]
    fn a_declared_pattern_replaces_the_bundled_one_with_its_id() {
        let dictionary = PatternFile::from_toml(
            r#"
[[patterns]]
id = "word_choice.delve"
name = "Project Delve"
severity = "low"
phrases = ["delve into"]
"#,
        )
        .unwrap();
        let patterns = apply_dictionary(bundled_patterns().unwrap(), &dictionary);
        let delve: Vec<_> = patterns
            .iter()
            .filter(|pattern| pattern.id == "word_choice.delve")
            .collect();

        assert_eq!(delve.len(), 1);
        assert_eq!(delve[0].name, "Project Delve");
        assert_eq!(delve[0].severity, Severity::Low);
        assert_eq!(delve[0].phrases, ["delve into"]);
    }

    #[test]
    fn declared_patterns_are_matched_before_the_bundled_ones() {
        let dictionary = PatternFile::from_toml(
            r#"
[[patterns]]
id = "project.landscape_architecture"
name = "Landscape Architecture"
severity = "high"
phrases = ["landscape architecture"]
"#,
        )
        .unwrap();
        let patterns = apply_dictionary(bundled_patterns().unwrap(), &dictionary);
        let detector = crate::detector::Detector::new(patterns).unwrap();
        let findings = detector.scan("The landscape architecture review is done.");

        assert_eq!(findings[0].rule_id, "project.landscape_architecture");
        assert_eq!(findings[0].matched, "landscape architecture");
    }

    #[test]
    fn a_dictionary_repeating_a_pattern_id_fails_validation() {
        let dictionary = PatternFile::from_toml(
            r#"
[[patterns]]
id = "project.dup"
name = "First"
severity = "low"
phrases = ["bounded"]

[[patterns]]
id = "project.dup"
name = "Second"
severity = "high"
phrases = ["contract"]
"#,
        )
        .unwrap();
        let patterns = apply_dictionary(bundled_patterns().unwrap(), &dictionary);

        assert_eq!(
            validate_patterns(&patterns),
            Err(PatternValidationError::DuplicatePatternId {
                id: "project.dup".to_owned(),
            })
        );
    }

    #[test]
    fn a_declared_pattern_with_a_new_id_is_added() {
        let dictionary = PatternFile::from_toml(
            r#"
[[patterns]]
id = "project.bounded"
name = "Bounded Without a Bound"
severity = "high"
phrases = ["bounded"]
"#,
        )
        .unwrap();
        let patterns = apply_dictionary(bundled_patterns().unwrap(), &dictionary);

        assert!(
            patterns
                .iter()
                .any(|pattern| pattern.id == "project.bounded")
        );
        assert!(
            patterns
                .iter()
                .any(|pattern| pattern.id == "word_choice.delve")
        );
    }

    #[test]
    fn a_dictionary_names_the_dialect_the_project_writes_in() {
        assert_eq!(
            PatternFile::from_toml(r#"dialect = "british""#)
                .unwrap()
                .dialect,
            Some(Dialect::British)
        );
        assert_eq!(PatternFile::default().dialect, None);
    }

    #[test]
    fn a_dictionary_tunes_a_count_under_the_rule_that_reads_it() {
        let file = PatternFile::from_toml(
            r#"
[thresholds."composition.cross_file_duplication"]
min_words = 20
"#,
        )
        .unwrap();

        assert_eq!(
            file.thresholds.cross_file_duplication,
            CrossFileLimits {
                min_words: 20,
                min_files: 2,
            }
        );
    }

    #[test]
    fn a_rule_id_written_without_quotes_tunes_the_same_count() {
        let file = PatternFile::from_toml(
            r#"
[thresholds.composition.cross_file_duplication]
min_words = 20
"#,
        )
        .unwrap();

        assert_eq!(
            file.thresholds.cross_file_duplication,
            CrossFileLimits {
                min_words: 20,
                min_files: 2,
            }
        );
        assert_eq!(file.thresholds.unknown_rules().count(), 0);
    }

    #[test]
    fn a_dictionary_tunes_the_character_and_markdown_counts() {
        let file = PatternFile::from_toml(
            r#"
[thresholds."formatting.em_dash_addiction"]
floor = 4
count = 10
rate_per_hundred_words = 3

[thresholds."formatting.unicode_decoration"]
min_occurrences = 5

[thresholds."formatting.bold_first_leads"]
min_leads = 2
"#,
        )
        .unwrap();

        assert_eq!(
            file.thresholds.em_dash_addiction,
            DashLimits {
                floor: 4,
                count: 10,
                rate_per_hundred_words: 3,
            }
        );
        assert_eq!(
            file.thresholds.unicode_decoration,
            DecorationLimits { min_occurrences: 5 }
        );
        assert_eq!(
            file.thresholds.bold_first_leads,
            BoldLeadLimits { min_leads: 2 }
        );
        assert_eq!(file.thresholds.unknown_rules().count(), 0);
    }

    #[test]
    fn the_character_and_markdown_rules_read_an_unquoted_id_too() {
        let file = PatternFile::from_toml(
            r#"
[thresholds.formatting.em_dash_addiction]
count = 10

[thresholds.formatting.unicode_decoration]
min_occurrences = 5

[thresholds.formatting.bold_first_leads]
min_leads = 2
"#,
        )
        .unwrap();

        assert_eq!(file.thresholds.em_dash_addiction.count, 10);
        assert_eq!(file.thresholds.unicode_decoration.min_occurrences, 5);
        assert_eq!(file.thresholds.bold_first_leads.min_leads, 2);
        assert_eq!(file.thresholds.unknown_rules().count(), 0);
    }

    #[test]
    fn a_dictionary_tunes_the_structural_and_repetition_counts() {
        let file = PatternFile::from_toml(
            r#"
[thresholds."sentence_structure.anaphora_abuse"]
min_sentences = 4

[thresholds."sentence_structure.tricolon_abuse"]
min_separators = 3
min_repeated_starts = 3

[thresholds."paragraph_structure.short_punchy_fragments"]
min_sentences = 2
max_words = 6

[thresholds."paragraph_structure.listicle_in_trench_coat"]
min_paragraphs = 4

[thresholds."composition.fractal_summaries"]
min_openings = 2

[thresholds."composition.historical_analogy_stacking"]
min_sentences = 4

[thresholds."composition.dead_metaphor"]
min_repeats = 8

[thresholds."composition.one_point_dilution"]
min_shared_terms = 4

[thresholds."composition.content_duplication"]
min_length = 80
"#,
        )
        .unwrap();

        assert_eq!(
            file.thresholds.structural,
            StructuralLimits {
                anaphora_abuse: AnaphoraLimits { min_sentences: 4 },
                tricolon_abuse: TricolonLimits {
                    min_separators: 3,
                    min_repeated_starts: 3,
                },
                short_punchy_fragments: FragmentLimits {
                    min_sentences: 2,
                    max_words: 6,
                },
                listicle_in_trench_coat: ListicleLimits { min_paragraphs: 4 },
                fractal_summaries: SummaryLimits { min_openings: 2 },
                historical_analogy_stacking: AnalogyLimits { min_sentences: 4 },
            }
        );
        assert_eq!(
            file.thresholds.repetition,
            RepetitionLimits {
                dead_metaphor: DeadMetaphorLimits { min_repeats: 8 },
                one_point_dilution: DilutionLimits {
                    min_shared_terms: 4,
                },
                content_duplication: DuplicationLimits { min_length: 80 },
            }
        );
        assert_eq!(file.thresholds.unknown_rules().count(), 0);
    }

    #[test]
    fn the_structural_and_repetition_rules_read_an_unquoted_id_too() {
        let file = PatternFile::from_toml(
            r#"
[thresholds.sentence_structure.anaphora_abuse]
min_sentences = 4

[thresholds.composition.dead_metaphor]
min_repeats = 8

[thresholds.composition.content_duplication]
min_length = 80
"#,
        )
        .unwrap();

        assert_eq!(file.thresholds.structural.anaphora_abuse.min_sentences, 4);
        assert_eq!(file.thresholds.repetition.dead_metaphor.min_repeats, 8);
        assert_eq!(
            file.thresholds.repetition.content_duplication.min_length,
            80
        );
        assert_eq!(file.thresholds.unknown_rules().count(), 0);
    }

    #[test]
    fn a_dictionary_tuning_nothing_keeps_every_default() {
        let file = PatternFile::from_toml(r#"allow = ["harness"]"#).unwrap();

        assert_eq!(file.thresholds, Thresholds::default());
        assert_eq!(
            file.thresholds.cross_file_duplication,
            CrossFileLimits::default()
        );
        assert_eq!(file.thresholds.unknown_rules().count(), 0);
    }

    #[test]
    fn a_threshold_key_naming_no_rule_is_kept_rather_than_rejected() {
        let file = PatternFile::from_toml(
            r#"
[thresholds."composition.cross_file_duplicaton"]
min_words = 20
"#,
        )
        .unwrap();

        assert_eq!(
            file.thresholds.unknown_rules().collect::<Vec<_>>(),
            ["composition.cross_file_duplicaton"]
        );
        assert_eq!(
            file.thresholds.cross_file_duplication,
            CrossFileLimits::default()
        );
    }

    #[test]
    fn an_unquoted_key_naming_no_rule_is_kept_under_the_id_it_spells() {
        let file = PatternFile::from_toml(
            r#"
[thresholds.composition.cross_file_duplicaton]
min_words = 20

[thresholds.compositon]
min_words = 20
"#,
        )
        .unwrap();

        assert_eq!(
            file.thresholds.unknown_rules().collect::<Vec<_>>(),
            ["composition.cross_file_duplicaton", "compositon"]
        );
        assert_eq!(
            file.thresholds.cross_file_duplication,
            CrossFileLimits::default()
        );
    }

    #[test]
    fn a_threshold_entry_rejects_a_count_its_rule_does_not_have() {
        let error = PatternFile::from_toml(
            r#"
[thresholds."composition.cross_file_duplication"]
min_word = 20
"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn a_dictionary_rejects_an_unknown_field() {
        let error = PatternFile::from_toml("allowed = [\"harness\"]").unwrap_err();

        assert!(error.to_string().contains("unknown field"));
    }
}
