//! Repetition detectors for document-level trope signals.

use std::collections::HashMap;

use crate::patterns::Severity;

use super::Finding;

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
pub fn scan_repetition(text: &str) -> Vec<Finding> {
    let paragraphs = paragraph_spans(text);
    let sentences = sentence_spans(text);

    let mut findings = Vec::new();
    findings.extend(scan_dead_metaphor(text));
    findings.extend(scan_one_point_dilution(text, &paragraphs));
    findings.extend(scan_content_duplication(text, &paragraphs, &sentences));
    findings
}

fn scan_dead_metaphor(text: &str) -> Vec<Finding> {
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
        .filter(|spans| spans.len() >= 5)
        .map(|spans| {
            Finding::repetition(
                "composition.dead_metaphor",
                "The Dead Metaphor",
                Severity::Medium,
                text,
                super::Span(spans[0].start(), spans[spans.len() - 1].end()),
            )
        })
        .collect()
}

fn scan_one_point_dilution(text: &str, paragraphs: &[super::Span]) -> Vec<Finding> {
    let paragraph_terms: Vec<_> = paragraphs
        .iter()
        .map(|paragraph| {
            (
                *paragraph,
                top_terms(&text[paragraph.start()..paragraph.end()]),
            )
        })
        .filter(|(_, terms)| terms.len() >= 3)
        .collect();

    paragraph_terms
        .windows(3)
        .filter(|window| {
            let shared = shared_terms(&window[0].1, &window[1].1, &window[2].1);
            shared >= 3
        })
        .map(|window| {
            Finding::repetition(
                "composition.one_point_dilution",
                "One-Point Dilution",
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
) -> Vec<Finding> {
    let mut findings = duplicate_normalized_spans(
        text,
        paragraphs,
        "composition.content_duplication",
        "Content Duplication",
    );

    findings.extend(duplicate_normalized_spans(
        text,
        sentences,
        "composition.content_duplication",
        "Content Duplication",
    ));

    findings
}

fn duplicate_normalized_spans(
    text: &str,
    spans: &[super::Span],
    rule_id: &str,
    rule_name: &str,
) -> Vec<Finding> {
    let mut seen: HashMap<String, super::Span> = HashMap::new();
    let mut findings = Vec::new();

    for span in spans {
        let value = &text[span.start()..span.end()];

        if value.len() < 40 {
            continue;
        }

        let normalized = normalize_text(value);

        if normalized.len() < 40 {
            continue;
        }

        if let Some(previous) = seen.get(&normalized) {
            findings.push(Finding::repetition(
                rule_id,
                rule_name,
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

fn paragraph_spans(text: &str) -> Vec<super::Span> {
    split_spans(text, "\n\n")
}

fn sentence_spans(text: &str) -> Vec<super::Span> {
    let mut spans = Vec::new();
    let mut start = 0;

    for (index, character) in text.char_indices() {
        if matches!(character, '.' | '!' | '?') {
            push_trimmed_span(text, &mut spans, start, index + character.len_utf8());
            start = index + character.len_utf8();
        }
    }

    push_trimmed_span(text, &mut spans, start, text.len());
    spans
}

fn split_spans(text: &str, separator: &str) -> Vec<super::Span> {
    let mut spans = Vec::new();
    let mut start = 0;

    for (index, _) in text.match_indices(separator) {
        push_trimmed_span(text, &mut spans, start, index);
        start = index + separator.len();
    }

    push_trimmed_span(text, &mut spans, start, text.len());
    spans
}

fn push_trimmed_span(text: &str, spans: &mut Vec<super::Span>, start: usize, end: usize) {
    let value = &text[start..end];
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return;
    }

    let leading = value.len() - value.trim_start().len();
    let trailing = value.len() - value.trim_end().len();

    spans.push(super::Span(start + leading, end - trailing));
}

fn word_spans(text: &str) -> Vec<super::Span> {
    let mut spans = Vec::new();
    let mut start = None;

    for (index, character) in text.char_indices() {
        if character.is_ascii_alphanumeric() || character == '\'' {
            start.get_or_insert(index);
        } else if let Some(word_start) = start.take() {
            spans.push(super::Span(word_start, index));
        }
    }

    if let Some(word_start) = start {
        spans.push(super::Span(word_start, text.len()));
    }

    spans
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

    #[test]
    fn detects_dead_metaphor() {
        let findings = scan_repetition(
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
        let findings = scan_repetition(
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
        let findings = scan_repetition(
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
        let findings = scan_repetition(
            "This sentence repeats the same exact claim about adoption and access. Something else happens. This sentence repeats the same exact claim about adoption and access.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "composition.content_duplication")
        );
    }
}
