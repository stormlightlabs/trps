//! Markdown-aware detectors for formatting tropes.
//!
//! Every rule here comes from the Tropes.fyi list in `meta/tropes.md`.
//! `meta/sources.md` carries the catalog and its license.

/// Bullets and paragraphs opening with a bolded span.
pub const BOLD_FIRST_LEADS: (&str, &str) = ("formatting.bold_first_leads", "Bold-First Leads");

/// Bolded leads in a row before the run is reported.
///
/// A writer reaches for one bolded lead, and sometimes for two. Three in a
/// row is an opening filled in from a template, which is the trope.
const BOLD_LEAD_THRESHOLD: usize = 3;

use crate::patterns::Severity;

use super::{Finding, Span};

/// Finds markdown-specific trope signals.
///
/// A run of bolded leads is the signal, so a list and a sequence of
/// paragraphs are read the same way. One finding covers one run, from the
/// first lead to the last.
pub fn scan_markdown(text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut run = Vec::new();

    for (block, opens_bold) in blocks(text) {
        if opens_bold {
            run.push(block);
            continue;
        }

        findings.extend(report_run(text, &run));
        run.clear();
    }

    findings.extend(report_run(text, &run));
    findings
}

/// The finding a run of bolded leads reports, where the run is long enough.
fn report_run(text: &str, run: &[Span]) -> Option<Finding> {
    let (first, last) = (run.first()?, run.last()?);

    (run.len() >= BOLD_LEAD_THRESHOLD).then(|| {
        Finding::markdown(
            BOLD_FIRST_LEADS,
            Severity::Medium,
            text,
            Span(first.start(), last.end()),
        )
    })
}

/// Every block of `text` that can open with a bolded span, with whether it
/// does.
///
/// A list item is one block, and so is a paragraph, because each carries a
/// single lead. A wrapped line continues the block above it, so a bullet
/// running over two lines is one lead and not a break in the run. A heading,
/// a table row, and a block quote carry no lead and end the run they
/// interrupt.
///
/// A blank line ends a block without ending a run. Paragraphs are separated
/// by blank lines, and [`mask_non_prose`] has already blanked the fenced
/// blocks, so a run holds across a sample sitting between two of its steps.
fn blocks(text: &str) -> Vec<(Span, bool)> {
    let mut blocks: Vec<(Span, bool)> = Vec::new();
    let mut open = false;
    let mut offset = 0;

    for line in text.split_inclusive('\n') {
        let span = Span(offset, offset + line.trim_end().len());
        offset += line.len();

        if line.trim().is_empty() {
            open = false;
        } else if let Some(body) = list_item_body(line.trim_start()) {
            blocks.push((span, starts_with_closed_bold(body.trim_start())));
            open = true;
        } else if !is_prose_line(line) {
            blocks.push((span, false));
            open = false;
        } else if open {
            let block = blocks.last_mut().expect("an open block was pushed");
            block.0 = Span(block.0.start(), span.end());
        } else {
            blocks.push((span, starts_with_closed_bold(line.trim_start())));
            open = true;
        }
    }

    blocks
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
    fn detects_unordered_bold_first_leads() {
        let findings = scan_markdown(
            "- **Security**: keys read at startup\n- **Latency**: down by a third\n- **Cost**: flat",
        );

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "formatting.bold_first_leads");
    }

    #[test]
    fn detects_numbered_bold_first_leads() {
        let findings = scan_markdown(
            "1. __Performance__: lazy loading\n2. __Security__: keys\n3. __Cost__: flat\n",
        );

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].matched,
            "1. __Performance__: lazy loading\n2. __Security__: keys\n3. __Cost__: flat"
        );
    }

    #[test]
    fn detects_bold_first_paragraphs() {
        let findings = scan_markdown(
            "**Starting from nothing.** Run the thing.\n\n**Starting from an issue.** Run the other thing.\n\n**Not sure what to do.** Run triage.\n",
        );

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "formatting.bold_first_leads");
    }

    #[test]
    fn one_bolded_lead_is_emphasis_rather_than_a_template() {
        assert!(scan_markdown("- **Security**: keys read at startup\n- Latency held.").is_empty());
        assert!(
            scan_markdown("**Security.** Keys are read at startup.\n\nLatency held.").is_empty()
        );
    }

    #[test]
    fn a_wrapped_bullet_does_not_end_a_run() {
        let findings = scan_markdown(
            "- **Security**: keys read at startup, which is\n  where the operator sets them\n- **Latency**: down\n- **Cost**: flat\n",
        );

        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn a_block_without_a_bolded_lead_ends_the_run() {
        let findings = scan_markdown(
            "**Security.** Keys are read at startup.\n\nThe rest of the config is read with it.\n\n**Latency.** It fell by a third.\n\n**Cost.** It held flat.\n",
        );

        assert!(findings.is_empty());
    }

    #[test]
    fn a_heading_ends_the_run_it_interrupts() {
        let findings = scan_markdown(
            "**Security.** Keys are read at startup.\n\n**Latency.** It fell by a third.\n\n## Next quarter\n\n**Cost.** It holds flat.\n",
        );

        assert!(findings.is_empty());
    }

    #[test]
    fn ignores_bold_later_in_bullets() {
        let findings = scan_markdown(
            "- The **security** setting\n- The **latency** budget\n- The **cost** ceiling",
        );

        assert!(findings.is_empty());
    }

    #[test]
    fn ignores_unclosed_bold() {
        let findings = scan_markdown("- **Security: keys\n- **Latency: down\n- **Cost: flat");

        assert!(findings.is_empty());
    }
}
