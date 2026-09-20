//! Markdown-aware detectors for formatting tropes.

/// Bullets opening with a bolded span.
pub const BOLD_FIRST_BULLETS: (&str, &str) =
    ("formatting.bold_first_bullets", "Bold-First Bullets");

use crate::patterns::Severity;

use super::{Finding, Span};

/// Finds markdown-specific trope signals.
pub fn scan_markdown(text: &str) -> Vec<Finding> {
    line_spans(text)
        .into_iter()
        .filter(|line| starts_with_bold_list_item(&text[line.start()..line.end()]))
        .map(|line| Finding::markdown(BOLD_FIRST_BULLETS, Severity::Medium, text, line))
        .collect()
}

fn starts_with_bold_list_item(line: &str) -> bool {
    match list_item_body(line.trim_start()) {
        Some(after_marker) => starts_with_closed_bold(after_marker.trim_start()),
        None => false,
    }
}

fn list_item_body(line: &str) -> Option<&str> {
    if let Some(rest) = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "))
    {
        return Some(rest);
    }

    let (digits, rest) = line.split_at(line.find(|character: char| !character.is_ascii_digit())?);

    if digits.is_empty() {
        return None;
    }

    rest.strip_prefix(". ")
}

fn starts_with_closed_bold(value: &str) -> bool {
    value
        .strip_prefix("**")
        .and_then(|rest| rest.find("**").map(|index| index > 0))
        .unwrap_or(false)
        || value
            .strip_prefix("__")
            .and_then(|rest| rest.find("__").map(|index| index > 0))
            .unwrap_or(false)
}

fn line_spans(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut start = 0;

    for (index, character) in text.char_indices() {
        if character == '\n' {
            push_line_span(text, &mut spans, start, index);
            start = index + character.len_utf8();
        }
    }

    push_line_span(text, &mut spans, start, text.len());
    spans
}

fn push_line_span(text: &str, spans: &mut Vec<Span>, start: usize, end: usize) {
    let line = &text[start..end];
    if !line.trim().is_empty() {
        let trailing = line.len() - line.trim_end().len();
        spans.push(Span(start, end - trailing));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_unordered_bold_first_bullets() {
        let findings = scan_markdown("- **Security**: Environment-based configuration");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "formatting.bold_first_bullets");
    }

    #[test]
    fn detects_numbered_bold_first_bullets() {
        let findings = scan_markdown("1. __Performance__: Lazy loading");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].matched, "1. __Performance__: Lazy loading");
    }

    #[test]
    fn ignores_bold_later_in_bullets() {
        assert!(scan_markdown("- The **security** setting").is_empty());
    }

    #[test]
    fn ignores_unclosed_bold() {
        assert!(scan_markdown("- **Security: Environment").is_empty());
    }
}
