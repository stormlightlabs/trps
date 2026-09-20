//! Character-class detectors that do not need Aho-Corasick.
//!
//! Every rule here comes from the Tropes.fyi list in `meta/tropes.md`.
//! `meta/sources.md` carries the catalog and its license.

use crate::patterns::Severity;

use super::{Finding, FindingKind};

/// Rule id for Unicode decoration findings.
pub const UNICODE_DECORATION_RULE_ID: &str = "formatting.unicode_decoration";

const UNICODE_DECORATION_RULE_NAME: &str = "Unicode Decoration";

const DECORATIVE_CHARS: &[char] = &['→', '←', '↔', '⇒', '“', '”', '‘', '’', '—', '–'];

/// Finds configured Unicode punctuation and decorative symbols.
pub fn scan_unicode_decoration(text: &str) -> Vec<Finding> {
    text.char_indices()
        .filter(|(_, character)| DECORATIVE_CHARS.contains(character))
        .map(|(start, character)| Finding {
            rule_id: UNICODE_DECORATION_RULE_ID.to_owned(),
            rule_name: UNICODE_DECORATION_RULE_NAME.to_owned(),
            severity: Severity::Low,
            kind: FindingKind::CharacterClass,
            matched: character.to_string(),
            span: super::Span(start, start + character.len_utf8()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_unicode_decoration() {
        let findings = scan_unicode_decoration("Input → Processing");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].matched, "→");
        assert_eq!(findings[0].span.start(), 6);
        assert_eq!(findings[0].span.end(), 9);
    }

    #[test]
    fn ignores_ascii_arrow() {
        assert!(scan_unicode_decoration("Input -> Processing").is_empty());
    }
}
