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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
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
/// remove from them.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternFile {
    /// Phrases removed from the patterns this file is applied to, matched
    /// case-insensitively.
    #[serde(default)]
    pub allow: Vec<String>,
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
/// The nearest directory holding one wins, and within a directory the first
/// name in [`PROJECT_DICTIONARY_FILES`] wins.
pub fn find_project_dictionary(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|directory| {
        PROJECT_DICTIONARY_FILES
            .iter()
            .map(|name| directory.join(name))
            .find(|path| path.is_file())
    })
}

/// Applies a project dictionary to `base`.
///
/// Allowed phrases are removed from every pattern in `base`, and a pattern left
/// with no phrases is dropped. A pattern the dictionary declares replaces the
/// one in `base` with the same id, or is appended when no id matches.
pub fn apply_dictionary(base: Vec<Pattern>, dictionary: &PatternFile) -> Vec<Pattern> {
    let allowed: HashSet<String> = dictionary
        .allow
        .iter()
        .map(|phrase| normalize(phrase))
        .collect();

    let mut patterns: Vec<Pattern> = base
        .into_iter()
        .filter_map(|mut pattern| {
            pattern
                .phrases
                .retain(|phrase| !allowed.contains(&normalize(phrase)));

            (!pattern.phrases.is_empty()).then_some(pattern)
        })
        .collect();

    for pattern in &dictionary.patterns {
        match patterns
            .iter_mut()
            .find(|existing| existing.id == pattern.id)
        {
            Some(existing) => *existing = pattern.clone(),
            None => patterns.push(pattern.clone()),
        }
    }

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

    #[test]
    fn project_dictionary_is_found_in_an_ancestor_directory() {
        for name in PROJECT_DICTIONARY_FILES {
            let root = temp_dir(&format!("find-{name}"));
            let nested = root.join("docs/guides");
            std::fs::create_dir_all(&nested).unwrap();
            std::fs::write(root.join(name), "allow = []\n").unwrap();

            assert_eq!(find_project_dictionary(&nested), Some(root.join(name)));

            std::fs::remove_dir_all(&root).unwrap();
        }
    }

    #[test]
    fn the_first_dictionary_name_wins_within_a_directory() {
        let root = temp_dir("dictionary-name-order");
        std::fs::create_dir_all(&root).unwrap();

        for name in PROJECT_DICTIONARY_FILES.iter().rev() {
            std::fs::write(root.join(name), "allow = []\n").unwrap();
        }

        assert_eq!(
            find_project_dictionary(&root),
            Some(root.join(PROJECT_DICTIONARY_FILES[0]))
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn no_project_dictionary_is_found_without_one() {
        let root = temp_dir("no-project-dictionary");
        std::fs::create_dir_all(&root).unwrap();

        assert_eq!(find_project_dictionary(&root), None);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn reading_a_missing_pattern_file_reports_its_path() {
        let path = temp_dir("missing-pattern-file").join("trps.toml");
        let error = load_pattern_file(&path).unwrap_err();

        assert!(error.to_string().contains("trps.toml"));
    }

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("tropius-{name}-{}", std::process::id()))
    }
}
