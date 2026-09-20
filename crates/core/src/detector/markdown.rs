//! Markdown-aware detectors for formatting tropes.
//!
//! Every rule here comes from the Tropes.fyi list in `meta/tropes.md`.
//! `meta/sources.md` carries the catalog and its license.

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

pub(crate) fn list_item_body(line: &str) -> Option<&str> {
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

/// Whether a line carries prose rather than markdown structure.
///
/// A heading, a table row, a block quote, and a list item are structure. The
/// structural detectors count sentences inside a paragraph, and three bullets
/// opening the same way are a list rather than anaphora.
pub(crate) fn is_prose_line(line: &str) -> bool {
    let trimmed = line.trim_start();

    !(trimmed.starts_with('#')
        || trimmed.starts_with('|')
        || trimmed.starts_with('>')
        || list_item_body(trimmed).is_some())
}

/// Blanks the regions of `text` that nobody wrote as prose.
///
/// A fenced block holds sample output, a command, or a quoted defect, and
/// front matter holds metadata. Grading either reports the quoted text rather
/// than the prose around it.
///
/// Every byte but a newline is replaced with a space, so an offset into the
/// result is the same offset in the original and a blanked region reads as
/// blank lines to the detectors that split on them.
pub(crate) fn mask_non_prose(text: &str) -> String {
    let mut masked = text.as_bytes().to_vec();

    for span in non_prose_spans(text) {
        for byte in &mut masked[span.start()..span.end()] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    }

    String::from_utf8(masked).expect("a space in place of a byte leaves the text valid UTF-8")
}

/// The byte ranges of the front matter and the fenced blocks of `text`.
///
/// An unclosed fence runs to the end of the text, which is how a markdown
/// reader treats it.
fn non_prose_spans(text: &str) -> Vec<Span> {
    let front_matter = front_matter_span(text);
    let body = front_matter.map(|span| span.end()).unwrap_or(0);

    let mut spans = Vec::from_iter(front_matter);
    let mut open: Option<(u8, usize, usize)> = None;
    let mut offset = body;

    for line in text[body..].split_inclusive('\n') {
        let end = offset + line.len();

        match (open, fence_marker(line)) {
            (None, Some((character, width, _))) => open = Some((character, width, offset)),
            (Some((character, width, start)), Some((closing, closing_width, closes)))
                if closes && closing == character && closing_width >= width =>
            {
                spans.push(Span(start, end));
                open = None;
            }
            _ => {}
        }

        offset = end;
    }

    if let Some((_, _, start)) = open {
        spans.push(Span(start, text.len()));
    }

    spans
}

/// The fence character of a line, the width of its run, and whether the line
/// carries nothing after it.
///
/// Only a line carrying nothing after the run closes a fence, which is what
/// keeps an opening ```` ```rust ```` from closing the block above it.
fn fence_marker(line: &str) -> Option<(u8, usize, bool)> {
    let rest = line.trim_start();

    if line.len() - rest.len() > 3 {
        return None;
    }

    let character = rest
        .bytes()
        .next()
        .filter(|byte| matches!(byte, b'`' | b'~'))?;
    let width = rest.bytes().take_while(|byte| *byte == character).count();

    (width >= 3).then(|| (character, width, rest[width..].trim().is_empty()))
}

/// The byte range of a YAML front matter block, its delimiters included.
fn front_matter_span(text: &str) -> Option<Span> {
    let mut lines = text.split_inclusive('\n');
    let mut offset = lines.next().filter(|line| line.trim() == "---")?.len();

    for line in lines {
        offset += line.len();

        if matches!(line.trim(), "---" | "...") {
            return Some(Span(0, offset));
        }
    }

    None
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
    fn masking_blanks_a_fenced_block_without_moving_offsets() {
        let text = "Prose above.\n\n```text\nRun the thing.\n```\n\nProse below.\n";
        let masked = mask_non_prose(text);

        assert_eq!(masked.len(), text.len());
        assert!(masked.contains("Prose above."));
        assert!(masked.contains("Prose below."));
        assert!(!masked.contains("Run the thing."));
    }

    #[test]
    fn masking_keeps_offsets_across_a_multibyte_fence() {
        let text = "```\n└─ → ⚠\n```\nAfter.\n";
        let masked = mask_non_prose(text);

        assert_eq!(masked.len(), text.len());
        assert_eq!(
            masked.find("After.").unwrap(),
            text.find("After.").unwrap(),
            "a blanked multi-byte character leaves the bytes after it where they were"
        );
    }

    #[test]
    fn an_info_string_does_not_close_the_fence_it_opens() {
        let masked = mask_non_prose("```rust\nlet delve = 1;\n```\nAfter.\n");

        assert!(!masked.contains("delve"));
        assert!(masked.contains("After."));
    }

    #[test]
    fn an_unclosed_fence_runs_to_the_end() {
        let masked = mask_non_prose("Before.\n\n```\nRun the thing.\n");

        assert!(masked.contains("Before."));
        assert!(!masked.contains("Run the thing."));
    }

    #[test]
    fn masking_blanks_front_matter() {
        let masked = mask_non_prose("---\ntitle: Delve\n---\n\nProse below.\n");

        assert!(!masked.contains("Delve"));
        assert!(masked.contains("Prose below."));
    }

    #[test]
    fn a_thematic_break_is_not_front_matter() {
        let text = "Prose above.\n\n---\n\nProse below.\n";

        assert_eq!(mask_non_prose(text), text);
    }

    #[test]
    fn structure_is_not_prose() {
        assert!(is_prose_line("The loader reads the file."));
        assert!(!is_prose_line("# Config files"));
        assert!(!is_prose_line("| Key | Meaning |"));
        assert!(!is_prose_line("> Quoted."));
        assert!(!is_prose_line("- The loader reads the file."));
        assert!(!is_prose_line("1. The loader reads the file."));
    }

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
