//! Pattern dictionary types and bundled TOML loading.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::{PatternLoadError, PatternValidationError};

/// File names a project dictionary is discovered under, in search order.
pub const PROJECT_DICTIONARY_FILES: &[&str] = &["trps.toml", "tropes.toml", "tropius.toml"];

/// TOML files bundled into `tropius-core`.
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
];

/// Severity attached to a pattern or detector finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
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
    /// Globs naming paths no scan reads, relative to the directory holding
    /// this file. See [`crate::excludes::Excludes`].
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Pattern entries declared by the file.
    #[serde(default)]
    pub patterns: Vec<Pattern>,
}

impl PatternFile {
    /// Deserializes a pattern file from TOML text.
    pub fn from_toml(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
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
    fn rejects_duplicate_pattern_ids() {
        let patterns = vec![
            Pattern {
                id: "word_choice.delve".to_owned(),
                name: "Delve".to_owned(),
                severity: Severity::Medium,
                phrases: vec!["delve into".to_owned()],
            },
            Pattern {
                id: "word_choice.delve".to_owned(),
                name: "Delve Again".to_owned(),
                severity: Severity::Medium,
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
                phrases: vec!["Delve Into".to_owned()],
            },
            Pattern {
                id: "two".to_owned(),
                name: "Two".to_owned(),
                severity: Severity::Medium,
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
            phrases: vec!["harness".to_owned()],
        }];
        let dictionary = PatternFile::from_toml(r#"allow = ["harness"]"#).unwrap();

        assert!(apply_dictionary(base, &dictionary).is_empty());
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
    fn a_dictionary_rejects_an_unknown_field() {
        let error = PatternFile::from_toml("allowed = [\"harness\"]").unwrap_err();

        assert!(error.to_string().contains("unknown field"));
    }
}
