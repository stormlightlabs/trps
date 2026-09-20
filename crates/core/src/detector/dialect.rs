//! Spelling that does not match the dialect a project writes in.
//!
//! The rule is off until a project dictionary names a dialect, because
//! neither spelling is wrong until a project has chosen one. A default would
//! report half of a British corpus.

/// A word spelled against the dialect the project writes in.
pub const DIALECT_SPELLING: (&str, &str) = ("word_choice.dialect_spelling", "Dialect Spelling");

use aho_corasick::{AhoCorasick, MatchKind};

use crate::errors::{DetectorBuildError, PatternLoadError};
use crate::patterns::Severity;

use super::{Finding, FindingKind, Span};

/// The word-pair file bundled with the crate.
const BUNDLED_SPELLINGS: &str = include_str!("dialect/spellings.toml");

/// The English dialect a project writes in.
///
/// `British` is the `-ise` convention rather than the `-ize` spelling Oxford
/// keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Dialect {
    /// American English.
    American,
    /// British English.
    British,
}

/// One word spelled one way in American English and another in British.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Spelling {
    /// The American form, such as `color`.
    pub american: String,
    /// The British form, such as `colour`.
    pub british: String,
}

impl Spelling {
    /// The form a project writing `dialect` uses.
    fn expected(&self, dialect: Dialect) -> &str {
        match dialect {
            Dialect::American => &self.american,
            Dialect::British => &self.british,
        }
    }

    /// The form the other dialect uses, which is what the rule reports.
    fn reported(&self, dialect: Dialect) -> &str {
        self.expected(dialect.other())
    }
}

impl Dialect {
    fn other(self) -> Self {
        match self {
            Self::American => Self::British,
            Self::British => Self::American,
        }
    }
}

/// The word pairs as `spellings.toml` holds them.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SpellingFile {
    words: Vec<Spelling>,
}

/// Reads the bundled word pairs.
pub fn bundled_spellings() -> Result<Vec<Spelling>, PatternLoadError> {
    Ok(toml::from_str::<SpellingFile>(BUNDLED_SPELLINGS)?.words)
}

/// The dialect rule, compiled over the forms the chosen dialect does not
/// use.
#[derive(Debug)]
pub struct DialectRule {
    matcher: AhoCorasick,
    /// The expected form for each of the matcher's patterns, in that order.
    expected: Vec<String>,
}

impl DialectRule {
    /// Compiles the rule for `dialect` over the bundled word pairs.
    pub fn new(dialect: Dialect) -> Result<Self, DetectorBuildError> {
        Self::from_spellings(dialect, &bundled_spellings()?)
    }

    /// Compiles the rule for `dialect` over `spellings`.
    ///
    /// The longest match at an offset wins, so `colourful` matches as itself
    /// rather than as the `colour` inside it.
    pub fn from_spellings(
        dialect: Dialect,
        spellings: &[Spelling],
    ) -> Result<Self, DetectorBuildError> {
        let matcher = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .match_kind(MatchKind::LeftmostLongest)
            .build(spellings.iter().map(|spelling| spelling.reported(dialect)))?;

        Ok(Self {
            matcher,
            expected: spellings
                .iter()
                .map(|spelling| spelling.expected(dialect).to_owned())
                .collect(),
        })
    }

    /// Reports every whole word in `text` that `dialect` spells differently.
    pub fn scan(&self, text: &str) -> Vec<Finding> {
        self.matcher
            .find_iter(text)
            .filter(|found| is_whole_word(text, found.start(), found.end()))
            .map(|found| {
                let matched = &text[found.start()..found.end()];

                Finding {
                    rule_id: DIALECT_SPELLING.0.to_owned(),
                    rule_name: DIALECT_SPELLING.1.to_owned(),
                    severity: Severity::Medium,
                    kind: FindingKind::Spelling,
                    matched: matched.to_owned(),
                    expected: Some(match_case(
                        matched,
                        &self.expected[found.pattern().as_usize()],
                    )),
                    span: Span(found.start(), found.end()),
                }
            })
            .collect()
    }
}

/// Whether the match runs to both ends of the word it sits in.
///
/// The word list holds no stems, so a match inside a longer word is not a
/// finding: `colorant` reports nothing.
fn is_whole_word(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();

    !(before.is_some_and(char::is_alphanumeric) || after.is_some_and(char::is_alphanumeric))
}

/// Writes `expected` in the case `matched` was written in, so the suggestion
/// can be pasted over the word it replaces. Every entry in the word list is
/// lowercase.
fn match_case(matched: &str, expected: &str) -> String {
    if matched.chars().any(char::is_uppercase) && !matched.chars().any(char::is_lowercase) {
        return expected.to_uppercase();
    }

    let mut characters = expected.chars();

    match matched.chars().next().is_some_and(char::is_uppercase) {
        true => characters
            .next()
            .map(|first| first.to_uppercase().chain(characters).collect())
            .unwrap_or_default(),
        false => expected.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(dialect: Dialect, text: &str) -> Vec<(String, String)> {
        DialectRule::new(dialect)
            .unwrap()
            .scan(text)
            .into_iter()
            .map(|finding| (finding.matched, finding.expected.unwrap()))
            .collect()
    }

    #[test]
    fn the_bundled_pairs_parse() {
        assert!(bundled_spellings().unwrap().len() > 100);
    }

    #[test]
    fn an_american_project_reports_the_british_spelling() {
        assert_eq!(
            findings(Dialect::American, "The judgement was coloured by rigour."),
            [
                ("judgement".to_owned(), "judgment".to_owned()),
                ("coloured".to_owned(), "colored".to_owned()),
                ("rigour".to_owned(), "rigor".to_owned()),
            ]
        );
    }

    #[test]
    fn a_british_project_reports_the_american_spelling() {
        assert_eq!(
            findings(Dialect::British, "The judgment was colored by rigor."),
            [
                ("judgment".to_owned(), "judgement".to_owned()),
                ("colored".to_owned(), "coloured".to_owned()),
                ("rigor".to_owned(), "rigour".to_owned()),
            ]
        );
    }

    #[test]
    fn the_dialect_the_project_writes_in_reports_nothing() {
        assert!(findings(Dialect::American, "The judgment was colored.").is_empty());
        assert!(findings(Dialect::British, "The judgement was coloured.").is_empty());
    }

    #[test]
    fn every_family_the_rule_covers_is_reported() {
        assert_eq!(
            findings(
                Dialect::American,
                "Colour the centre, analyse the defence, and cancel the travelling."
            ),
            [
                ("Colour".to_owned(), "Color".to_owned()),
                ("centre".to_owned(), "center".to_owned()),
                ("analyse".to_owned(), "analyze".to_owned()),
                ("defence".to_owned(), "defense".to_owned()),
                ("travelling".to_owned(), "traveling".to_owned()),
            ]
        );
    }

    #[test]
    fn a_longer_spelling_wins_over_the_one_inside_it() {
        assert_eq!(
            findings(Dialect::American, "A colourful organisational chart."),
            [
                ("colourful".to_owned(), "colorful".to_owned()),
                ("organisational".to_owned(), "organizational".to_owned()),
            ]
        );
    }

    #[test]
    fn a_spelling_inside_a_longer_word_is_not_a_finding() {
        assert!(findings(Dialect::American, "The colourant and centred.").len() == 1);
        assert!(findings(Dialect::British, "The laboratory ran overnight.").is_empty());
    }

    #[test]
    fn the_suggestion_keeps_the_case_the_word_was_written_in() {
        assert_eq!(
            findings(Dialect::American, "Colour. COLOUR. colour."),
            [
                ("Colour".to_owned(), "Color".to_owned()),
                ("COLOUR".to_owned(), "COLOR".to_owned()),
                ("colour".to_owned(), "color".to_owned()),
            ]
        );
    }

    #[test]
    fn a_possessive_and_a_hyphen_still_end_a_word() {
        assert_eq!(
            findings(Dialect::American, "The colour-coded neighbour's map."),
            [
                ("colour".to_owned(), "color".to_owned()),
                ("neighbour".to_owned(), "neighbor".to_owned()),
            ]
        );
    }
}
