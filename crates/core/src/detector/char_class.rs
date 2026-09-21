//! Character-class detectors that do not need Aho-Corasick.
//!
//! Every rule here comes from the Tropes.fyi list in `meta/tropes.md`.
//! `meta/sources.md` carries the catalog and its license.

use serde::Deserialize;

use crate::patterns::Severity;

use super::{Finding, FindingKind, Span, paragraph_spans, reaches_rate, word_spans};

/// Rule id for Unicode decoration findings.
pub const UNICODE_DECORATION_RULE_ID: &str = "formatting.unicode_decoration";

const UNICODE_DECORATION_RULE_NAME: &str = "Unicode Decoration";

/// Dashes used for pauses, asides, and pivots until they stop landing.
pub const EM_DASH_ADDICTION: (&str, &str) = ("formatting.em_dash_addiction", "Em-Dash Addiction");

/// Decorative characters, grouped so that each class is counted on its own.
///
/// One quoted phrase or one arrow reads as ordinary prose, so a class fires
/// only where it repeats inside a section. Counting every class together would
/// report a paragraph carrying one arrow and one quoted phrase.
///
/// Curly single quotes are left out: `’` is how a word processor writes an
/// apostrophe. Dashes are left out because [`scan_em_dash_addiction`] counts
/// them across the whole document rather than inside a section.
const DECORATION_CLASSES: &[&[char]] = &[&['“', '”'], &['→', '←', '↔', '⇒']];

/// What counts as decoration repeated often enough to report, under
/// `formatting.unicode_decoration`.
///
/// The default was calibrated against `meta/examples`, which holds short
/// prose. A project writing quoted terms or arrow diagrams raises it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DecorationLimits {
    /// Occurrences of one class inside one section before it is reported.
    /// Below two the rule would report a single curly quote, so two is the
    /// floor.
    pub min_occurrences: usize,
}

impl Default for DecorationLimits {
    fn default() -> Self {
        Self { min_occurrences: 3 }
    }
}

/// The Unicode dashes the em-dash rule counts. The ASCII `--` proxy needs the
/// characters around it, so [`ascii_dash_spans`] finds that form.
const DASHES: &[char] = &['—', '–'];

/// What counts as dash use dense enough to read as a habit, under
/// `formatting.em_dash_addiction`.
///
/// The defaults were calibrated against `meta/examples`, which holds short
/// prose. How many dashes read as a habit is a house-style call, so a project
/// writing more of them raises these rather than silencing the rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DashLimits {
    /// Dashes below which neither threshold reports. One aside is written
    /// with a dash on each side of it, so density starts above a pair. Below
    /// two the rule would report a lone dash, so two is the floor.
    pub floor: usize,
    /// Dashes that report whatever the document's length. `meta/tropes.md`
    /// puts a human writer at two or three in a piece. Two is the floor here
    /// as well.
    pub count: usize,
    /// Dashes per hundred words that report a document too short to reach
    /// [`DashLimits::count`]. At zero every document is dense, which leaves
    /// no rate at all, so one is the floor.
    pub rate_per_hundred_words: usize,
}

impl Default for DashLimits {
    fn default() -> Self {
        Self {
            floor: 3,
            count: 6,
            rate_per_hundred_words: 2,
        }
    }
}

/// Finds Unicode punctuation and decorative symbols repeated inside a section.
///
/// One finding covers one class in one section. It spans the first occurrence
/// to the last and matches the characters themselves, so it reports how many
/// there were and where.
pub fn scan_unicode_decoration(text: &str, limits: DecorationLimits) -> Vec<Finding> {
    let min_occurrences = limits.min_occurrences.max(2);
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

            if hits.len() < min_occurrences {
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
                expected: None,
                span: Span(first, last + character.len_utf8()),
            });
        }
    }

    findings
}

/// Finds dashes used often enough to read as a habit.
///
/// Addiction is a density claim, so the count runs across the whole document
/// rather than a section, and the ASCII `--` proxy is counted alongside the
/// Unicode forms. One finding covers the document: it spans the first dash to
/// the last and matches the dashes themselves.
///
/// A long piece reports on the count alone. A short one reports on the rate,
/// because six dashes in a paragraph and six in a chapter are not the same
/// habit. Either way the default leaves two dashes alone, which is one aside.
pub fn scan_em_dash_addiction(text: &str, limits: DashLimits) -> Vec<Finding> {
    let floor = limits.floor.max(2);
    let count = limits.count.max(2);
    let rate = limits.rate_per_hundred_words.max(1);
    let mut spans: Vec<Span> = text
        .char_indices()
        .filter(|(offset, character)| {
            DASHES.contains(character) && !is_numeric_range(text, *offset, *character)
        })
        .map(|(offset, character)| Span(offset, offset + character.len_utf8()))
        .collect();

    spans.extend(ascii_dash_spans(text));
    spans.sort_by_key(Span::start);

    let words = word_spans(text).len();
    let dense = spans.len() >= count || reaches_rate(spans.len(), rate, words);

    if spans.len() < floor || !dense {
        return Vec::new();
    }

    let first = spans[0];
    let last = spans[spans.len() - 1];

    vec![Finding {
        rule_id: EM_DASH_ADDICTION.0.to_owned(),
        rule_name: EM_DASH_ADDICTION.1.to_owned(),
        severity: Severity::Medium,
        kind: FindingKind::CharacterClass,
        matched: spans
            .iter()
            .map(|span| &text[span.start()..span.end()])
            .collect::<Vec<_>>()
            .join(" "),
        expected: None,
        span: Span(first.start(), last.end()),
    }]
}

/// The ASCII `--` a writer types where the keyboard has no em dash.
///
/// Whitespace on both sides is what tells the dash apart from a flag, a
/// comment marker, or the `---` a thematic break is written with. Whitespace
/// rather than a space is what keeps a dash found across a line wrap.
fn ascii_dash_spans(text: &str) -> Vec<Span> {
    text.match_indices("--")
        .filter(|(offset, _)| {
            let before = text[..*offset].chars().next_back();
            let after = text[offset + 2..].chars().next();

            before.is_none_or(char::is_whitespace) && after.is_none_or(char::is_whitespace)
        })
        .map(|(offset, _)| Span(offset, offset + 2))
        .collect()
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

    /// Scans for decoration at the counts the tool ships with.
    fn scan_decoration(text: &str) -> Vec<Finding> {
        scan_unicode_decoration(text, DecorationLimits::default())
    }

    /// Scans for dashes at the counts the tool ships with.
    fn scan_dashes(text: &str) -> Vec<Finding> {
        scan_em_dash_addiction(text, DashLimits::default())
    }

    #[test]
    fn finds_a_repeated_decoration_class() {
        let findings = scan_decoration("A → b → c → d");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].matched, "→ → →");
        assert_eq!(findings[0].span.start(), 2);
        assert_eq!(findings[0].span.end(), 17);
    }

    #[test]
    fn ignores_a_lone_decoration() {
        assert!(scan_decoration("Input → Processing").is_empty());
    }

    #[test]
    fn counts_each_class_on_its_own() {
        let findings = scan_decoration("A “b” c → d");

        assert!(findings.is_empty());
    }

    #[test]
    fn reports_each_class_that_repeats() {
        let findings = scan_decoration("A → b → c → d, “e”, “f”, and “g”");

        let matched: Vec<_> = findings
            .iter()
            .map(|finding| finding.matched.as_str())
            .collect();

        assert_eq!(matched, ["“ ” “ ” “ ”", "→ → →"]);
    }

    #[test]
    fn does_not_count_across_sections() {
        assert!(scan_decoration("One →\n\nTwo →\n\nThree →").is_empty());
    }

    #[test]
    fn ignores_curly_apostrophes() {
        assert!(scan_decoration("It’s the writer’s reader’s call").is_empty());
    }

    #[test]
    fn ignores_ascii_arrows() {
        assert!(scan_decoration("Input -> output -> result -> done").is_empty());
    }

    #[test]
    fn leaves_dashes_to_the_em_dash_rule() {
        assert!(scan_decoration("A — b — c — d").is_empty());
    }

    #[test]
    fn counts_the_ascii_proxy_with_the_unicode_forms() {
        let findings = scan_dashes("A — b -- c – d");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].matched, "— -- –");
        assert_eq!(findings[0].span.start(), 2);
        assert_eq!(findings[0].span.end(), 16);
    }

    #[test]
    fn ignores_one_aside() {
        let aside = "The change — long overdue — shipped on Friday.";

        assert!(scan_dashes(aside).is_empty());
    }

    #[test]
    fn ignores_a_single_dash_in_a_page() {
        let page = format!("{}— and it shipped.", "word ".repeat(400));

        assert!(scan_dashes(&page).is_empty());
    }

    #[test]
    fn counts_across_sections() {
        assert_eq!(scan_dashes("One —\n\nTwo —\n\nThree —").len(), 1);
    }

    #[test]
    fn reports_a_long_document_on_the_count_alone() {
        let document = format!("{}— ", "word ".repeat(60)).repeat(6);

        assert_eq!(scan_dashes(&document).len(), 1);
    }

    #[test]
    fn leaves_the_same_document_alone_one_dash_short() {
        let document = format!("{}— ", "word ".repeat(60)).repeat(5);

        assert!(scan_dashes(&document).is_empty());
    }

    #[test]
    fn ignores_en_dashes_between_digits() {
        let ranges = "The window ran 2019–2021, the audit 2022–2023, and the review 2024–2025.";

        assert!(scan_dashes(ranges).is_empty());
    }

    #[test]
    fn ignores_thematic_breaks() {
        let breaks = "One\n\n---\n\nTwo\n\n---\n\nThree\n\n---\n\nFour";

        assert!(scan_dashes(breaks).is_empty());
    }

    #[test]
    fn ignores_command_flags() {
        let command = "Run cargo test --workspace --all-features --locked";

        assert!(scan_dashes(command).is_empty());
    }

    #[test]
    fn a_tuned_dash_count_moves_where_a_long_document_reports() {
        let six = format!("{}— ", "word ".repeat(60)).repeat(6);
        let five = format!("{}— ", "word ".repeat(60)).repeat(5);
        let raised = DashLimits {
            count: 8,
            ..DashLimits::default()
        };
        let lowered = DashLimits {
            count: 5,
            ..DashLimits::default()
        };

        assert!(scan_em_dash_addiction(&six, raised).is_empty());
        assert_eq!(scan_em_dash_addiction(&five, lowered).len(), 1);
    }

    #[test]
    fn a_tuned_dash_floor_moves_the_dashes_a_document_is_left_alone_at() {
        let aside = "The change — long overdue — shipped on Friday.";
        let three = "A — b — c — d";
        let raised = DashLimits {
            floor: 4,
            ..DashLimits::default()
        };
        let lowered = DashLimits {
            floor: 2,
            ..DashLimits::default()
        };

        assert!(scan_em_dash_addiction(three, raised).is_empty());
        assert_eq!(scan_em_dash_addiction(aside, lowered).len(), 1);
    }

    /// A dictionary is input, so a rate past what the arithmetic holds has to
    /// be an answer rather than a panic or a wrap.
    ///
    /// Four words times a quarter of a `usize`, rounded up, is exactly one
    /// wrap: multiplying the two plainly panics on a debug build and reports
    /// zero on a release one, which reads as the document being dense.
    #[test]
    fn a_rate_no_document_reaches_reports_nothing() {
        let text = "A -- b -- c -- d";
        let unreachable = DashLimits {
            rate_per_hundred_words: usize::MAX / 4 + 1,
            ..DashLimits::default()
        };

        assert_eq!(scan_dashes(text).len(), 1);
        assert!(scan_em_dash_addiction(text, unreachable).is_empty());
    }

    #[test]
    fn a_tuned_dash_rate_moves_where_a_short_document_reports() {
        let dense = format!("{}— ", "word ".repeat(40)).repeat(5);
        let sparse = format!("{}— ", "word ".repeat(60)).repeat(5);
        let raised = DashLimits {
            rate_per_hundred_words: 4,
            ..DashLimits::default()
        };
        let lowered = DashLimits {
            rate_per_hundred_words: 1,
            ..DashLimits::default()
        };

        assert_eq!(scan_dashes(&dense).len(), 1);
        assert!(scan_em_dash_addiction(&dense, raised).is_empty());
        assert!(scan_dashes(&sparse).is_empty());
        assert_eq!(scan_em_dash_addiction(&sparse, lowered).len(), 1);
    }

    #[test]
    fn dash_counts_set_below_two_report_one_aside_rather_than_a_lone_dash() {
        let lone = "The change shipped — late.";
        let aside = "The change — long overdue — shipped.";
        let below = DashLimits {
            floor: 0,
            count: 0,
            rate_per_hundred_words: 0,
        };

        assert!(scan_em_dash_addiction(lone, below).is_empty());
        assert_eq!(scan_em_dash_addiction(aside, below).len(), 1);
    }

    #[test]
    fn a_tuned_decoration_count_moves_where_a_class_reports() {
        let two = "A → b → c";
        let three = "A → b → c → d";

        assert!(scan_decoration(two).is_empty());
        assert_eq!(
            scan_unicode_decoration(two, DecorationLimits { min_occurrences: 2 }).len(),
            1
        );
        assert_eq!(scan_decoration(three).len(), 1);
        assert!(scan_unicode_decoration(three, DecorationLimits { min_occurrences: 4 }).is_empty());
    }

    #[test]
    fn a_decoration_count_set_below_two_reports_no_single_curly_quote() {
        let single = "He said “hello there.";
        let quoted = "He said “hello there.”";
        let below = DecorationLimits { min_occurrences: 1 };

        assert!(scan_unicode_decoration(single, below).is_empty());
        assert_eq!(scan_unicode_decoration(quoted, below).len(), 1);
    }
}
