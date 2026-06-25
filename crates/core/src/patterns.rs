//! Pattern dictionary types and bundled TOML loading.

use std::collections::HashSet;

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

/// Loads and validates the pattern dictionaries bundled with the crate.
pub fn bundled_patterns() -> Result<Vec<Pattern>, PatternLoadError> {
    let mut patterns = Vec::new();

    for (_, input) in BUNDLED_PATTERN_FILES {
        patterns.extend(PatternFile::from_toml(input)?.patterns);
    }

    validate_patterns(&patterns)?;

    Ok(patterns)
}

/// Validates pattern ids and phrases across all loaded pattern files.
pub fn validate_patterns(patterns: &[Pattern]) -> Result<(), PatternValidationError> {
    let mut ids = HashSet::new();
    let mut phrases = HashSet::new();

    for pattern in patterns {
        if !ids.insert(pattern.id.as_str()) {
            return Err(PatternValidationError::DuplicatePatternId {
                id: pattern.id.clone(),
            });
        }

        for phrase in &pattern.phrases {
            let normalized = phrase.to_ascii_lowercase();

            if !phrases.insert(normalized) {
                return Err(PatternValidationError::DuplicatePhrase {
                    phrase: phrase.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Errors that can happen while loading bundled pattern dictionaries.
#[derive(Debug)]
pub enum PatternLoadError {
    /// A TOML file could not be deserialized.
    Toml(toml::de::Error),
    /// Pattern dictionaries parsed but failed semantic validation.
    Validation(PatternValidationError),
}

impl From<toml::de::Error> for PatternLoadError {
    fn from(error: toml::de::Error) -> Self {
        Self::Toml(error)
    }
}

impl From<PatternValidationError> for PatternLoadError {
    fn from(error: PatternValidationError) -> Self {
        Self::Validation(error)
    }
}

/// Validation errors for parsed pattern dictionaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternValidationError {
    /// Two patterns use the same id.
    DuplicatePatternId {
        /// The duplicate pattern id.
        id: String,
    },
    /// Two pattern entries use the same phrase after ASCII case folding.
    DuplicatePhrase {
        /// The duplicate phrase as it appeared in the later entry.
        phrase: String,
    },
}

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

/// A deserialized TOML pattern file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct PatternFile {
    /// Pattern entries declared by the file.
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
}
