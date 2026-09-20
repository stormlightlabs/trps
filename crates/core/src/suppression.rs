//! Suppression markers read out of the scanned text.
//!
//! A repository that has decided to keep a span marks it in the prose rather
//! than in the dictionary, because the reason for keeping it lives next to the
//! text and not at the root of the project. Quoted slop is the case that needs
//! it: a document quoting a trope to criticize it is graded for the trope it
//! quotes, and no dictionary entry can tell that apart from the real thing.

use std::ops::Range;

/// Marker suppressing findings on the line after the one carrying it.
pub const NEXT_LINE_MARKER: &str = "trps-ignore-next-line";

/// Marker opening a suppressed region, closed by [`END_MARKER`].
pub const START_MARKER: &str = "trps-ignore-start";

/// Marker closing a region opened by [`START_MARKER`].
pub const END_MARKER: &str = "trps-ignore-end";

/// Separator between a marker's rule ids and the note explaining it.
const REASON_SEPARATOR: &str = "--";

/// One span of scanned text a document has asked to keep.
#[derive(Debug)]
struct Region {
    span: Range<usize>,
    /// Rule ids the region suppresses, or empty for every rule.
    rules: Vec<String>,
}

impl Region {
    fn covers(&self, offset: usize, rule_id: &str) -> bool {
        self.span.contains(&offset)
            && (self.rules.is_empty() || self.rules.iter().any(|rule| rule == rule_id))
    }
}

/// Spans of scanned text a document has asked to keep.
///
/// A marker is recognized anywhere on a line, so every comment syntax carries
/// one: `<!-- trps-ignore-next-line -->` in Markdown reads the same to this as
/// a bare word does in a plain text file.
///
/// What follows a marker on its line is a list of rule ids, separated by
/// spaces or commas, and a marker naming none suppresses every rule. A `--`
/// ends the list, so the note saying why the span was kept can sit beside the
/// marker without being read as a rule id.
#[derive(Debug, Default)]
pub struct Suppressions {
    regions: Vec<Region>,
}

impl Suppressions {
    /// Reads the markers in `text`.
    ///
    /// A region left open runs to the end of the text, so a missing
    /// [`END_MARKER`] suppresses the rest of the document rather than nothing.
    /// A second [`START_MARKER`] inside an open region is not a second region.
    pub fn new(text: &str) -> Self {
        let mut regions = Vec::new();
        let mut open: Option<(usize, Vec<String>)> = None;
        let mut next_line: Option<Vec<String>> = None;
        let mut offset = 0;

        for line in text.split_inclusive('\n') {
            let end = offset + line.len();

            if let Some(rules) = next_line.take() {
                regions.push(Region {
                    span: offset..end,
                    rules,
                });
            }

            if line.contains(END_MARKER) {
                if let Some((start, rules)) = open.take() {
                    regions.push(Region {
                        span: start..end,
                        rules,
                    });
                }
            } else if line.contains(START_MARKER) {
                open.get_or_insert_with(|| (offset, rules_after(line, START_MARKER)));
            } else if line.contains(NEXT_LINE_MARKER) {
                next_line = Some(rules_after(line, NEXT_LINE_MARKER));
            }

            offset = end;
        }

        if let Some((start, rules)) = open {
            regions.push(Region {
                span: start..text.len(),
                rules,
            });
        }

        Self { regions }
    }

    /// Reports whether a finding for `rule_id` starting at `offset` was
    /// suppressed.
    pub fn covers(&self, offset: usize, rule_id: &str) -> bool {
        self.regions
            .iter()
            .any(|region| region.covers(offset, rule_id))
    }
}

/// Reads the rule ids following `marker` on its line.
///
/// A token carrying no letter or digit is comment syntax rather than a rule
/// id, which is what keeps the `-->` closing a Markdown comment out of the
/// list.
fn rules_after(line: &str, marker: &str) -> Vec<String> {
    let rest = line.split_once(marker).map(|(_, rest)| rest).unwrap_or("");
    let rest = rest
        .split_once(REASON_SEPARATOR)
        .map_or(rest, |(rules, _)| rules);

    rest.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|token| token.chars().any(|character| character.is_alphanumeric()))
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rule id no marker in these tests names.
    const OTHER: &str = "formatting.bold_first_bullets";

    #[test]
    fn text_without_markers_suppresses_nothing() {
        let suppressions = Suppressions::new("one\ntwo\n");

        assert!(!suppressions.covers(0, OTHER));
        assert!(!suppressions.covers(4, OTHER));
    }

    #[test]
    fn a_next_line_marker_covers_only_the_line_after_it() {
        let text = "one\n<!-- trps-ignore-next-line -->\ntwo\nthree\n";
        let suppressions = Suppressions::new(text);

        assert!(!suppressions.covers(text.find("one").unwrap(), OTHER));
        assert!(suppressions.covers(text.find("two").unwrap(), OTHER));
        assert!(!suppressions.covers(text.find("three").unwrap(), OTHER));
    }

    #[test]
    fn a_next_line_marker_on_the_last_line_covers_nothing() {
        let suppressions = Suppressions::new("one\ntrps-ignore-next-line\n");

        assert!(!suppressions.covers(0, OTHER));
    }

    #[test]
    fn a_region_covers_its_markers_and_everything_between_them() {
        let text = "one\ntrps-ignore-start\ntwo\nthree\ntrps-ignore-end\nfour\n";
        let suppressions = Suppressions::new(text);

        assert!(!suppressions.covers(text.find("one").unwrap(), OTHER));
        assert!(suppressions.covers(text.find("two").unwrap(), OTHER));
        assert!(suppressions.covers(text.find("three").unwrap(), OTHER));
        assert!(!suppressions.covers(text.find("four").unwrap(), OTHER));
    }

    #[test]
    fn an_unclosed_region_runs_to_the_end_of_the_text() {
        let text = "one\ntrps-ignore-start\ntwo\nthree";
        let suppressions = Suppressions::new(text);

        assert!(!suppressions.covers(text.find("one").unwrap(), OTHER));
        assert!(suppressions.covers(text.find("three").unwrap(), OTHER));
    }

    #[test]
    fn a_second_start_inside_a_region_does_not_reopen_it() {
        let text = "trps-ignore-start\none\ntrps-ignore-start\ntwo\ntrps-ignore-end\nthree\n";
        let suppressions = Suppressions::new(text);

        assert!(suppressions.covers(text.find("two").unwrap(), OTHER));
        assert!(!suppressions.covers(text.find("three").unwrap(), OTHER));
    }

    #[test]
    fn an_end_marker_without_a_start_suppresses_nothing() {
        let text = "one\ntrps-ignore-end\ntwo\n";
        let suppressions = Suppressions::new(text);

        assert!(!suppressions.covers(text.find("one").unwrap(), OTHER));
        assert!(!suppressions.covers(text.find("two").unwrap(), OTHER));
    }

    #[test]
    fn a_marker_naming_a_rule_suppresses_that_rule_alone() {
        let text = "trps-ignore-next-line word_choice.delve\none\n";
        let suppressions = Suppressions::new(text);
        let one = text.find("one").unwrap();

        assert!(suppressions.covers(one, "word_choice.delve"));
        assert!(!suppressions.covers(one, OTHER));
    }

    #[test]
    fn a_marker_names_several_rules_by_comma_or_space() {
        let commas = "trps-ignore-next-line word_choice.delve,unicode\none\n";
        let spaces = "trps-ignore-next-line word_choice.delve unicode\none\n";

        for text in [commas, spaces] {
            let suppressions = Suppressions::new(text);
            let one = text.find("one").unwrap();

            assert!(suppressions.covers(one, "word_choice.delve"));
            assert!(suppressions.covers(one, "unicode"));
            assert!(!suppressions.covers(one, OTHER));
        }
    }

    #[test]
    fn a_region_naming_a_rule_suppresses_that_rule_alone() {
        let text = "trps-ignore-start word_choice.delve\none\ntrps-ignore-end\n";
        let suppressions = Suppressions::new(text);
        let one = text.find("one").unwrap();

        assert!(suppressions.covers(one, "word_choice.delve"));
        assert!(!suppressions.covers(one, OTHER));
    }

    #[test]
    fn comment_syntax_around_a_marker_is_not_read_as_a_rule() {
        let text = "<!-- trps-ignore-next-line word_choice.delve -->\none\n";
        let suppressions = Suppressions::new(text);
        let one = text.find("one").unwrap();

        assert!(suppressions.covers(one, "word_choice.delve"));
        assert!(!suppressions.covers(one, OTHER));
    }

    #[test]
    fn a_note_after_two_dashes_is_not_read_as_a_rule() {
        let text = "trps-ignore-next-line -- quoting the thing we criticize\none\n";
        let suppressions = Suppressions::new(text);
        let one = text.find("one").unwrap();

        assert!(suppressions.covers(one, OTHER));
        assert!(suppressions.covers(one, "word_choice.delve"));
    }
}
