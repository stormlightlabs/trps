//! Character-class detectors that do not need Aho-Corasick.

use crate::patterns::Severity;

use super::{Finding, FindingKind, Span, paragraph_spans};

/// Rule id for Unicode decoration findings.
pub const UNICODE_DECORATION_RULE_ID: &str = "formatting.unicode_decoration";

const UNICODE_DECORATION_RULE_NAME: &str = "Unicode Decoration";

/// Decorative characters, grouped so that a run of one kind is what fires.
///
/// A single em dash or one quoted phrase reads as a human voice. The tell is
/// the same class used over and over in one section, which is why the classes
/// are counted apart from each other rather than together.
///
/// Curly single quotes are left out. `’` is how a word processor writes an
/// apostrophe, and an apostrophe is not decoration.
const DECORATION_CLASSES: &[&[char]] = &[&['—', '–'], &['“', '”'], &['→', '←', '↔', '⇒']];

/// Occurrences of one class within one section before it is worth reporting.
const DECORATION_THRESHOLD: usize = 3;

/// Finds Unicode punctuation and decorative symbols repeated within a section.
///
/// One finding covers one class in one section, spanning the first occurrence
/// to the last and matching the characters themselves, so a reader sees how
/// many there were and where instead of one glyph per finding.
pub fn scan_unicode_decoration(text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    for section in paragraph_spans(text) {
        let value = &text[section.start()..section.end()];

        for class in DECORATION_CLASSES {
            let hits: Vec<_> = value
                .char_indices()
                .filter(|(_, character)| class.contains(character))
                .map(|(offset, character)| (section.start() + offset, character))
                .collect();

            if hits.len() < DECORATION_THRESHOLD {
                continue;
            }

            let (first, _) = hits[0];
            let (last, character) = hits[hits.len() - 1];

            findings.push(Finding {
                rule_id: UNICODE_DECORATION_RULE_ID.to_owned(),
                rule_name: UNICODE_DECORATION_RULE_NAME.to_owned(),
                severity: Severity::Low,
                kind: FindingKind::CharacterClass,
                matched: hits
                    .iter()
                    .map(|(_, character)| character.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                span: Span(first, last + character.len_utf8()),
            });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_repeated_decoration_class() {
        let findings = scan_unicode_decoration("A — b — c — d");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].matched, "— — —");
        assert_eq!(findings[0].span.start(), 2);
        assert_eq!(findings[0].span.end(), 17);
    }

    #[test]
    fn ignores_a_lone_decoration() {
        assert!(scan_unicode_decoration("Input → Processing").is_empty());
    }

    #[test]
    fn counts_each_class_on_its_own() {
        let findings = scan_unicode_decoration("A — b “c” d → e");

        assert!(findings.is_empty());
    }

    #[test]
    fn reports_each_class_that_repeats() {
        let findings = scan_unicode_decoration("A — b — c — d, “e”, “f”, and “g”");

        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn does_not_count_across_sections() {
        assert!(scan_unicode_decoration("One —\n\nTwo —\n\nThree —").is_empty());
    }

    #[test]
    fn ignores_curly_apostrophes() {
        assert!(scan_unicode_decoration("It’s the writer’s reader’s call").is_empty());
    }

    #[test]
    fn ignores_ascii_arrows() {
        assert!(scan_unicode_decoration("Input -> output -> result -> done").is_empty());
    }
}
