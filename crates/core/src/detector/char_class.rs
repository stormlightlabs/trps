//! Character-class detectors that do not need Aho-Corasick.

use crate::patterns::Severity;

use super::{Finding, FindingKind, Span, paragraph_spans};

/// Rule id for Unicode decoration findings.
pub const UNICODE_DECORATION_RULE_ID: &str = "formatting.unicode_decoration";

const UNICODE_DECORATION_RULE_NAME: &str = "Unicode Decoration";

/// Decorative characters, grouped so that each class is counted on its own.
///
/// One em dash or one quoted phrase reads as ordinary prose, so a class fires
/// only where it repeats inside a section. Counting every class together would
/// report a paragraph carrying one dash, one arrow, and one quoted phrase.
///
/// Curly single quotes are left out: `’` is how a word processor writes an
/// apostrophe. An en dash between two digits is left out by
/// [`is_numeric_range`], which is a range rather than decoration.
const DECORATION_CLASSES: &[&[char]] = &[&['—', '–'], &['“', '”'], &['→', '←', '↔', '⇒']];

/// Occurrences of one class inside one section before it is reported.
const DECORATION_THRESHOLD: usize = 3;

/// Finds Unicode punctuation and decorative symbols repeated inside a section.
///
/// One finding covers one class in one section. It spans the first occurrence
/// to the last and matches the characters themselves, so it reports how many
/// there were and where.
pub fn scan_unicode_decoration(text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    for section in paragraph_spans(text) {
        let value = &text[section.start()..section.end()];

        for class in DECORATION_CLASSES {
            let hits: Vec<_> = value
                .char_indices()
                .filter(|(offset, character)| {
                    class.contains(character) && !is_numeric_range(value, *offset, *character)
                })
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

/// Whether the character at `offset` is an en dash standing between two
/// digits, as `2019–2021` does. A range is what the en dash is for.
fn is_numeric_range(value: &str, offset: usize, character: char) -> bool {
    if character != '–' {
        return false;
    }

    let before = value[..offset].chars().next_back();
    let after = value[offset + character.len_utf8()..].chars().next();

    before.is_some_and(|before| before.is_ascii_digit())
        && after.is_some_and(|after| after.is_ascii_digit())
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

        let matched: Vec<_> = findings
            .iter()
            .map(|finding| finding.matched.as_str())
            .collect();

        assert_eq!(matched, ["— — —", "“ ” “ ” “ ”"]);
    }

    #[test]
    fn ignores_en_dashes_between_digits() {
        let ranges = "The window ran 2019–2021, the audit 2022–2023, and the review 2024–2025.";

        assert!(scan_unicode_decoration(ranges).is_empty());
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
