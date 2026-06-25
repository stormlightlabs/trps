pub const BUNDLED_PATTERN_FILES: &[(&str, &str)] = &[
    ("formatting.toml", include_str!("patterns/formatting.toml")),
    (
        "sentence-structure.toml",
        include_str!("patterns/sentence-structure.toml"),
    ),
    ("tone.toml", include_str!("patterns/tone.toml")),
    (
        "word-choice.toml",
        include_str!("patterns/word-choice.toml"),
    ),
];

pub fn bundled_patterns() -> Result<Vec<Pattern>, toml::de::Error> {
    let mut patterns = Vec::new();

    for (_, input) in BUNDLED_PATTERN_FILES {
        patterns.extend(PatternFile::from_toml(input)?.patterns);
    }

    Ok(patterns)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct PatternFile {
    pub patterns: Vec<Pattern>,
}

impl PatternFile {
    pub fn from_toml(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Pattern {
    pub id: String,
    pub name: String,
    pub severity: Severity,
    pub phrases: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
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
}
