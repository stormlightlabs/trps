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

/// Byte ranges of scanned text a document has asked to keep.
///
/// A marker is recognized anywhere on a line, so every comment syntax carries
/// one: `<!-- trps-ignore-next-line -->` in Markdown reads the same to this as
/// a bare word does in a plain text file.
#[derive(Debug, Default)]
pub struct Suppressions {
    ranges: Vec<Range<usize>>,
}

impl Suppressions {
    /// Reads the markers in `text`.
    ///
    /// A region left open runs to the end of the text, so a missing
    /// [`END_MARKER`] suppresses the rest of the document rather than nothing.
    /// A second [`START_MARKER`] inside an open region is not a second region.
    pub fn new(text: &str) -> Self {
        let mut ranges = Vec::new();
        let mut open: Option<usize> = None;
        let mut next_line = false;
        let mut offset = 0;

        for line in text.split_inclusive('\n') {
            let end = offset + line.len();

            if std::mem::take(&mut next_line) {
                ranges.push(offset..end);
            }

            if line.contains(END_MARKER) {
                if let Some(start) = open.take() {
                    ranges.push(start..end);
                }
            } else if line.contains(START_MARKER) {
                open.get_or_insert(offset);
            } else if line.contains(NEXT_LINE_MARKER) {
                next_line = true;
            }

            offset = end;
        }

        if let Some(start) = open {
            ranges.push(start..text.len());
        }

        Self { ranges }
    }

    /// Reports whether a finding starting at `offset` was suppressed.
    pub fn covers(&self, offset: usize) -> bool {
        self.ranges.iter().any(|range| range.contains(&offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_without_markers_suppresses_nothing() {
        let suppressions = Suppressions::new("one\ntwo\n");

        assert!(!suppressions.covers(0));
        assert!(!suppressions.covers(4));
    }

    #[test]
    fn a_next_line_marker_covers_only_the_line_after_it() {
        let text = "one\n<!-- trps-ignore-next-line -->\ntwo\nthree\n";
        let suppressions = Suppressions::new(text);

        assert!(!suppressions.covers(text.find("one").unwrap()));
        assert!(suppressions.covers(text.find("two").unwrap()));
        assert!(!suppressions.covers(text.find("three").unwrap()));
    }

    #[test]
    fn a_next_line_marker_on_the_last_line_covers_nothing() {
        let suppressions = Suppressions::new("one\ntrps-ignore-next-line\n");

        assert!(!suppressions.covers(0));
    }

    #[test]
    fn a_region_covers_its_markers_and_everything_between_them() {
        let text = "one\ntrps-ignore-start\ntwo\nthree\ntrps-ignore-end\nfour\n";
        let suppressions = Suppressions::new(text);

        assert!(!suppressions.covers(text.find("one").unwrap()));
        assert!(suppressions.covers(text.find("two").unwrap()));
        assert!(suppressions.covers(text.find("three").unwrap()));
        assert!(!suppressions.covers(text.find("four").unwrap()));
    }

    #[test]
    fn an_unclosed_region_runs_to_the_end_of_the_text() {
        let text = "one\ntrps-ignore-start\ntwo\nthree";
        let suppressions = Suppressions::new(text);

        assert!(!suppressions.covers(text.find("one").unwrap()));
        assert!(suppressions.covers(text.find("three").unwrap()));
    }

    #[test]
    fn a_second_start_inside_a_region_does_not_reopen_it() {
        let text = "trps-ignore-start\none\ntrps-ignore-start\ntwo\ntrps-ignore-end\nthree\n";
        let suppressions = Suppressions::new(text);

        assert!(suppressions.covers(text.find("two").unwrap()));
        assert!(!suppressions.covers(text.find("three").unwrap()));
    }

    #[test]
    fn an_end_marker_without_a_start_suppresses_nothing() {
        let text = "one\ntrps-ignore-end\ntwo\n";
        let suppressions = Suppressions::new(text);

        assert!(!suppressions.covers(text.find("one").unwrap()));
        assert!(!suppressions.covers(text.find("two").unwrap()));
    }
}
