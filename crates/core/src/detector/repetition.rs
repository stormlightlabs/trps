//! Repetition detectors for document-level trope signals.
//!
//! Every rule here comes from the Tropes.fyi list in `meta/tropes.md`.
//! `meta/sources.md` carries the catalog and its license.

/// A metaphor term repeated until it stops meaning anything.
pub const DEAD_METAPHOR: (&str, &str) = ("composition.dead_metaphor", "The Dead Metaphor");

/// One point stretched across a document.
pub const ONE_POINT_DILUTION: (&str, &str) =
    ("composition.one_point_dilution", "One-Point Dilution");

/// A paragraph or sentence repeated verbatim.
pub const CONTENT_DUPLICATION: (&str, &str) =
    ("composition.content_duplication", "Content Duplication");

use std::collections::HashMap;

use serde::Deserialize;

use crate::patterns::Severity;

use super::{Finding, paragraph_spans, sentence_spans, word_spans};

/// The counts the repetition rules fire at, each under its own rule id.
///
/// The defaults were calibrated against `meta/examples`, which holds short
/// prose. Repetition is how a reference document stays clear, so a project
/// writing one raises these rather than silencing the rules.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RepetitionLimits {
    /// What counts as a metaphor term worn out, under
    /// `composition.dead_metaphor`.
    pub dead_metaphor: DeadMetaphorLimits,
    /// What counts as one point stretched across paragraphs, under
    /// `composition.one_point_dilution`.
    pub one_point_dilution: DilutionLimits,
    /// What counts as a passage long enough to report a second copy of,
    /// under `composition.content_duplication`.
    pub content_duplication: DuplicationLimits,
}

/// What counts as a metaphor term worn out, under
/// `composition.dead_metaphor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DeadMetaphorLimits {
    /// Uses of one term in a document before it is reported. Below two a
    /// single use is a repeat of nothing, so two is the floor.
    pub min_repeats: usize,
}

impl Default for DeadMetaphorLimits {
    fn default() -> Self {
        Self { min_repeats: 5 }
    }
}

/// What counts as one point stretched across paragraphs, under
/// `composition.one_point_dilution`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DilutionLimits {
    /// Top terms three paragraphs in a row have to share. A paragraph
    /// carrying fewer top terms than this can share that many with nobody,
    /// so it is skipped before the comparison. Below two a run of paragraphs
    /// sharing one word is every document on a subject, so two is the floor.
    pub min_shared_terms: usize,
}

impl Default for DilutionLimits {
    fn default() -> Self {
        Self {
            min_shared_terms: 3,
        }
    }
}

/// What counts as a passage long enough to report a second copy of, under
/// `composition.content_duplication`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DuplicationLimits {
    /// Characters a passage holds before a second copy of it is reported,
    /// counted over the words it normalizes to. A short sentence repeats for
    /// good reason, and this length is what keeps the rule off it. At zero a
    /// passage holding no words duplicates every other one, so one is the
    /// floor.
    pub min_length: usize,
}

impl Default for DuplicationLimits {
    fn default() -> Self {
        Self { min_length: 40 }
    }
}

const DEAD_METAPHOR_TERMS: &[&str] = &[
    "ecosystem",
    "ecosystems",
    "wall",
    "walls",
    "door",
    "doors",
    "primitive",
    "primitives",
    "tapestry",
    "landscape",
];

const STOP_WORDS: &[&str] = &[
    "about", "after", "again", "also", "because", "before", "being", "between", "could", "every",
    "from", "have", "into", "more", "much", "over", "same", "should", "that", "their", "there",
    "these", "they", "this", "through", "what", "when", "where", "which", "while", "with", "would",
    "your",
];

/// Finds repetition-based trope signals in text.
pub fn scan_repetition(text: &str, limits: RepetitionLimits) -> Vec<Finding> {
    let paragraphs = paragraph_spans(text);
    let sentences = sentence_spans(text);

    let mut findings = Vec::new();
    findings.extend(scan_dead_metaphor(text, limits.dead_metaphor));
    findings.extend(scan_one_point_dilution(
        text,
        &paragraphs,
        limits.one_point_dilution,
    ));
    findings.extend(scan_content_duplication(
        text,
        &paragraphs,
        &sentences,
        limits.content_duplication,
    ));
    findings
}

fn scan_dead_metaphor(text: &str, limits: DeadMetaphorLimits) -> Vec<Finding> {
    let min_repeats = limits.min_repeats.max(2);
    let mut hits: HashMap<&str, Vec<super::Span>> = HashMap::new();

    for span in word_spans(text) {
        let word = text[span.start()..span.end()].to_ascii_lowercase();

        if let Some(term) = DEAD_METAPHOR_TERMS
            .iter()
            .find(|term| **term == word.as_str())
        {
            hits.entry(term).or_default().push(span);
        }
    }

    hits.into_values()
        .filter(|spans| spans.len() >= min_repeats)
        .map(|spans| {
            Finding::repetition(
                DEAD_METAPHOR,
                Severity::Medium,
                text,
                super::Span(spans[0].start(), spans[spans.len() - 1].end()),
            )
        })
        .collect()
}

fn scan_one_point_dilution(
    text: &str,
    paragraphs: &[super::Span],
    limits: DilutionLimits,
) -> Vec<Finding> {
    let min_shared_terms = limits.min_shared_terms.max(2);
    let paragraph_terms: Vec<_> = paragraphs
        .iter()
        .map(|paragraph| {
            (
                *paragraph,
                top_terms(&text[paragraph.start()..paragraph.end()]),
            )
        })
        .filter(|(_, terms)| terms.len() >= min_shared_terms)
        .collect();

    paragraph_terms
        .windows(3)
        .filter(|window| {
            let shared = shared_terms(&window[0].1, &window[1].1, &window[2].1);
            shared >= min_shared_terms
        })
        .map(|window| {
            Finding::repetition(
                ONE_POINT_DILUTION,
                Severity::Medium,
                text,
                super::Span(window[0].0.start(), window[2].0.end()),
            )
        })
        .collect()
}

fn scan_content_duplication(
    text: &str,
    paragraphs: &[super::Span],
    sentences: &[super::Span],
    limits: DuplicationLimits,
) -> Vec<Finding> {
    let min_length = limits.min_length.max(1);
    let mut findings =
        duplicate_normalized_spans(text, paragraphs, CONTENT_DUPLICATION, min_length);

    findings.extend(duplicate_normalized_spans(
        text,
        sentences,
        CONTENT_DUPLICATION,
        min_length,
    ));

    findings
}

fn duplicate_normalized_spans(
    text: &str,
    spans: &[super::Span],
    rule: (&str, &str),
    min_length: usize,
) -> Vec<Finding> {
    let mut seen: HashMap<String, super::Span> = HashMap::new();
    let mut findings = Vec::new();

    for span in spans {
        let value = &text[span.start()..span.end()];

        if value.len() < min_length {
            continue;
        }

        let normalized = normalize_text(value);

        if normalized.len() < min_length {
            continue;
        }

        if let Some(previous) = seen.get(&normalized) {
            findings.push(Finding::repetition(
                rule,
                Severity::High,
                text,
                super::Span(previous.start(), span.end()),
            ));
        } else {
            seen.insert(normalized, *span);
        }
    }

    findings
}

fn top_terms(text: &str) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for span in word_spans(text) {
        let word = text[span.start()..span.end()].to_ascii_lowercase();

        if word.len() < 5 || STOP_WORDS.contains(&word.as_str()) {
            continue;
        }

        *counts.entry(word).or_default() += 1;
    }

    let mut counts: Vec<_> = counts.into_iter().collect();
    counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    counts.into_iter().take(5).map(|(word, _)| word).collect()
}

fn shared_terms(first: &[String], second: &[String], third: &[String]) -> usize {
    first
        .iter()
        .filter(|term| second.contains(term) && third.contains(term))
        .count()
}

fn normalize_text(text: &str) -> String {
    word_spans(text)
        .into_iter()
        .map(|span| text[span.start()..span.end()].to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scans for repetition signals at the counts the tool ships with.
    fn scan_defaults(text: &str) -> Vec<Finding> {
        scan_repetition(text, RepetitionLimits::default())
    }

    /// Whether a scan reported `rule`, so a tuned count is read apart from
    /// whatever else the fixture happens to trip.
    fn fires(findings: &[Finding], rule: (&str, &str)) -> bool {
        findings.iter().any(|finding| finding.rule_id == rule.0)
    }

    #[test]
    fn detects_dead_metaphor() {
        let findings = scan_defaults(
            "The ecosystem needs ecosystem value. This ecosystem has ecosystem tools for the ecosystem.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "composition.dead_metaphor")
        );
    }

    #[test]
    fn detects_one_point_dilution() {
        let findings = scan_defaults(
            "Platform access pricing blocks builders and adoption.\n\nPlatform access pricing slows builders and adoption.\n\nPlatform access pricing confuses builders and adoption.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "composition.one_point_dilution")
        );
    }

    #[test]
    fn detects_duplicate_paragraphs() {
        let findings = scan_defaults(
            "This paragraph repeats the same exact claim about adoption and access.\n\nThis paragraph repeats the same exact claim about adoption and access.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "composition.content_duplication")
        );
    }

    #[test]
    fn detects_duplicate_sentences() {
        let findings = scan_defaults(
            "This sentence repeats the same exact claim about adoption and access. Something else happens. This sentence repeats the same exact claim about adoption and access.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "composition.content_duplication")
        );
    }

    #[test]
    fn a_tuned_dead_metaphor_count_moves_where_a_term_reports() {
        let four = "The ecosystem needs ecosystem value. This ecosystem has ecosystem tools.";
        let five = "The ecosystem needs ecosystem value. This ecosystem has ecosystem tools for the ecosystem.";
        let lowered = RepetitionLimits {
            dead_metaphor: DeadMetaphorLimits { min_repeats: 4 },
            ..RepetitionLimits::default()
        };
        let raised = RepetitionLimits {
            dead_metaphor: DeadMetaphorLimits { min_repeats: 6 },
            ..RepetitionLimits::default()
        };

        assert!(!fires(&scan_defaults(four), DEAD_METAPHOR));
        assert!(fires(&scan_repetition(four, lowered), DEAD_METAPHOR));
        assert!(fires(&scan_defaults(five), DEAD_METAPHOR));
        assert!(!fires(&scan_repetition(five, raised), DEAD_METAPHOR));
    }

    #[test]
    fn a_tuned_shared_term_count_moves_where_paragraphs_report() {
        let two_shared = "Platform pricing blocks builders.\n\nPlatform pricing slows adoption.\n\nPlatform pricing confuses readers.";
        let four_shared = "Platform access pricing blocks builders and adoption.\n\nPlatform access pricing slows builders and adoption.\n\nPlatform access pricing confuses builders and adoption.";
        let lowered = RepetitionLimits {
            one_point_dilution: DilutionLimits {
                min_shared_terms: 2,
            },
            ..RepetitionLimits::default()
        };
        let raised = RepetitionLimits {
            one_point_dilution: DilutionLimits {
                min_shared_terms: 5,
            },
            ..RepetitionLimits::default()
        };

        assert!(!fires(&scan_defaults(two_shared), ONE_POINT_DILUTION));
        assert!(fires(
            &scan_repetition(two_shared, lowered),
            ONE_POINT_DILUTION
        ));
        assert!(fires(&scan_defaults(four_shared), ONE_POINT_DILUTION));
        assert!(!fires(
            &scan_repetition(four_shared, raised),
            ONE_POINT_DILUTION
        ));
    }

    #[test]
    fn a_tuned_duplication_length_moves_which_passages_report() {
        let short = "Access matters here.\n\nAccess matters here.";
        let long = "This paragraph repeats the same exact claim about adoption and access.\n\nThis paragraph repeats the same exact claim about adoption and access.";
        let lowered = RepetitionLimits {
            content_duplication: DuplicationLimits { min_length: 10 },
            ..RepetitionLimits::default()
        };
        let raised = RepetitionLimits {
            content_duplication: DuplicationLimits { min_length: 200 },
            ..RepetitionLimits::default()
        };

        assert!(!fires(&scan_defaults(short), CONTENT_DUPLICATION));
        assert!(fires(&scan_repetition(short, lowered), CONTENT_DUPLICATION));
        assert!(fires(&scan_defaults(long), CONTENT_DUPLICATION));
        assert!(!fires(&scan_repetition(long, raised), CONTENT_DUPLICATION));
    }

    #[test]
    fn counts_set_to_zero_are_clamped_rather_than_reporting_plain_prose() {
        let limits = RepetitionLimits {
            dead_metaphor: DeadMetaphorLimits { min_repeats: 0 },
            one_point_dilution: DilutionLimits {
                min_shared_terms: 0,
            },
            content_duplication: DuplicationLimits { min_length: 0 },
        };
        let text = "The wind rose over the harbor.\n\nA gull turned above the water.";

        assert!(scan_repetition(text, limits).is_empty());
    }
}
