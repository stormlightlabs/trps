//! Text detectors for phrase-based and structural trope signals.

pub mod char_class;

use aho_corasick::{AhoCorasick, MatchKind};

use crate::patterns::{Pattern, PatternLoadError, PatternValidationError, Severity};
use crate::patterns::{bundled_patterns, validate_patterns};

/// Finds trope signals in prose.
#[derive(Debug)]
pub struct Detector {
    phrase_patterns: Vec<Pattern>,
    phrase_to_pattern: Vec<usize>,
    phrase_matcher: AhoCorasick,
}

impl Detector {
    /// Builds a detector from the bundled pattern dictionaries.
    pub fn bundled() -> Result<Self, DetectorBuildError> {
        Self::new(bundled_patterns()?)
    }

    /// Builds a detector from already-loaded patterns.
    pub fn new(patterns: Vec<Pattern>) -> Result<Self, DetectorBuildError> {
        validate_patterns(&patterns)?;

        let mut phrases = Vec::new();
        let mut phrase_to_pattern = Vec::new();

        for (pattern_index, pattern) in patterns.iter().enumerate() {
            for phrase in &pattern.phrases {
                phrases.push(phrase.as_str());
                phrase_to_pattern.push(pattern_index);
            }
        }

        let phrase_matcher = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .match_kind(MatchKind::LeftmostFirst)
            .build(phrases)?;

        Ok(Self {
            phrase_patterns: patterns,
            phrase_to_pattern,
            phrase_matcher,
        })
    }

    /// Scans text with all enabled detectors.
    pub fn scan(&self, text: &str) -> Vec<Finding> {
        let mut findings = self.scan_phrases(text);
        findings.extend(char_class::scan_unicode_decoration(text));
        findings.sort_by_key(|finding| finding.start);
        findings
    }

    fn scan_phrases(&self, text: &str) -> Vec<Finding> {
        self.phrase_matcher
            .find_iter(text)
            .map(|mat| {
                let phrase_index = mat.pattern().as_usize();
                let pattern = &self.phrase_patterns[self.phrase_to_pattern[phrase_index]];

                Finding {
                    rule_id: pattern.id.clone(),
                    rule_name: pattern.name.clone(),
                    severity: pattern.severity,
                    kind: FindingKind::Phrase,
                    matched: text[mat.start()..mat.end()].to_owned(),
                    start: mat.start(),
                    end: mat.end(),
                }
            })
            .collect()
    }
}

/// A detector finding with byte offsets into the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Stable rule or pattern id.
    pub rule_id: String,
    /// Human-readable rule or pattern name.
    pub rule_name: String,
    /// Finding severity.
    pub severity: Severity,
    /// Detector category that produced the finding.
    pub kind: FindingKind,
    /// Matched text slice.
    pub matched: String,
    /// Start byte offset.
    pub start: usize,
    /// End byte offset.
    pub end: usize,
}

/// The kind of detector that produced a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    /// Literal phrase matched by Aho-Corasick.
    Phrase,
    /// Character-class match from a structural detector.
    CharacterClass,
}

/// Errors that can happen while building a detector.
#[derive(Debug)]
pub enum DetectorBuildError {
    /// Bundled pattern files could not be loaded.
    PatternLoad(PatternLoadError),
    /// Pattern dictionaries failed validation.
    PatternValidation(PatternValidationError),
    /// The Aho-Corasick automaton could not be built.
    AhoCorasick(aho_corasick::BuildError),
}

impl From<PatternLoadError> for DetectorBuildError {
    fn from(error: PatternLoadError) -> Self {
        Self::PatternLoad(error)
    }
}

impl From<PatternValidationError> for DetectorBuildError {
    fn from(error: PatternValidationError) -> Self {
        Self::PatternValidation(error)
    }
}

impl From<aho_corasick::BuildError> for DetectorBuildError {
    fn from(error: aho_corasick::BuildError) -> Self {
        Self::AhoCorasick(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_detector_finds_phrase_patterns() {
        let detector = Detector::bundled().unwrap();
        let findings = detector.scan("Let's delve into this robust ecosystem.");

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "word_choice.delve")
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "word_choice.grandiose_nouns")
        );
    }

    #[test]
    fn phrase_matching_is_case_insensitive() {
        let detector = Detector::bundled().unwrap();
        let findings = detector.scan("We should DELVE INTO the details.");

        assert_eq!(findings[0].rule_id, "word_choice.delve");
        assert_eq!(findings[0].matched, "DELVE INTO");
    }

    #[test]
    fn scan_includes_unicode_decoration() {
        let detector = Detector::bundled().unwrap();
        let findings = detector.scan("Input → output");

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == char_class::UNICODE_DECORATION_RULE_ID)
        );
    }
}
