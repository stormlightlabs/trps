//! Text detectors for phrase-based and structural trope signals.

pub mod char_class;
pub mod markdown;
pub mod repetition;
pub mod structural;

use std::fmt::Display;

use aho_corasick::{AhoCorasick, MatchKind};

use crate::errors::DetectorBuildError;
use crate::patterns::{Pattern, Severity};
use crate::patterns::{bundled_patterns, validate_patterns};
use crate::suppression::Suppressions;

/// Rule ids the detectors compiled into this crate can report.
///
/// The phrase rules come out of the loaded dictionaries, so [`Detector`] holds
/// those; these have no dictionary entry to read. Anything asking whether a
/// rule id exists needs both, which [`Detector::rule_ids`] joins.
pub const BUILTIN_RULE_IDS: &[&str] = &[
    char_class::UNICODE_DECORATION_RULE_ID,
    markdown::BOLD_FIRST_BULLETS.0,
    repetition::CONTENT_DUPLICATION.0,
    repetition::DEAD_METAPHOR.0,
    repetition::ONE_POINT_DILUTION.0,
    structural::ANAPHORA_ABUSE.0,
    structural::FRACTAL_SUMMARIES.0,
    structural::HISTORICAL_ANALOGY_STACKING.0,
    structural::LISTICLE_IN_TRENCH_COAT.0,
    structural::SHORT_PUNCHY_FRAGMENTS.0,
    structural::TRICOLON_ABUSE.0,
];

/// Finds trope signals in prose.
#[derive(Debug)]
pub struct Detector {
    phrase_patterns: Vec<Pattern>,
    phrase_to_pattern: Vec<usize>,
    phrase_matcher: AhoCorasick,
}

impl Detector {
    /// Builds a detector from the bundled pattern dictionaries.
    pub fn bundled() -> Result<Self, DetectorBuildError> {
        Self::new(bundled_patterns()?)
    }

    /// Builds a detector from already-loaded patterns.
    pub fn new(patterns: Vec<Pattern>) -> Result<Self, DetectorBuildError> {
        validate_patterns(&patterns)?;

        let mut phrases = Vec::new();
        let mut phrase_to_pattern = Vec::new();

        for (pattern_index, pattern) in patterns.iter().enumerate() {
            for phrase in &pattern.phrases {
                phrases.push(phrase.as_str());
                phrase_to_pattern.push(pattern_index);
            }
        }

        let phrase_matcher = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .match_kind(MatchKind::LeftmostFirst)
            .build(phrases)?;

        Ok(Self {
            phrase_patterns: patterns,
            phrase_to_pattern,
            phrase_matcher,
        })
    }

    /// Every rule id this detector can report.
    pub fn rule_ids(&self) -> impl Iterator<Item = &str> + Clone {
        self.phrase_patterns
            .iter()
            .map(|pattern| pattern.id.as_str())
            .chain(BUILTIN_RULE_IDS.iter().copied())
    }

    /// Scans text with all enabled detectors.
    ///
    /// Findings the text suppressed in place are dropped here rather than by
    /// the caller, so every reader of a scan sees the same document.
    pub fn scan(&self, text: &str) -> Vec<Finding> {
        let suppressions = Suppressions::new(text);
        let mut findings = self.scan_phrases(text);

        findings.extend(char_class::scan_unicode_decoration(text));
        findings.extend(markdown::scan_markdown(text));
        findings.extend(structural::scan_structural(text));
        findings.extend(repetition::scan_repetition(text));
        findings.retain(|finding| !suppressions.covers(finding.span.start(), &finding.rule_id));
        findings.sort_by_key(|finding| finding.span.start());
        findings
    }

    fn scan_phrases(&self, text: &str) -> Vec<Finding> {
        self.phrase_matcher
            .find_iter(text)
            .map(|mat| {
                let phrase_index = mat.pattern().as_usize();
                let pattern = &self.phrase_patterns[self.phrase_to_pattern[phrase_index]];

                Finding {
                    rule_id: pattern.id.clone(),
                    rule_name: pattern.name.clone(),
                    severity: pattern.severity,
                    kind: FindingKind::Phrase,
                    matched: text[mat.start()..mat.end()].to_owned(),
                    span: Span(mat.start(), mat.end()),
                }
            })
            .collect()
    }
}

/// A detector finding with byte offsets into the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Stable rule or pattern id.
    pub rule_id: String,
    /// Human-readable rule or pattern name.
    pub rule_name: String,
    /// Finding severity.
    pub severity: Severity,
    /// Detector category that produced the finding.
    pub kind: FindingKind,
    /// The text that matched, or, for a detector that counts occurrences, the
    /// occurrences themselves: `formatting.unicode_decoration` reports
    /// `— — —` rather than the text between the first dash and the last.
    pub matched: String,
    /// Start & end byte offset.
    pub span: Span,
}

impl Finding {
    pub fn structural(rule: (&str, &str), text: &str, span: Span) -> Finding {
        Finding {
            rule_id: rule.0.to_owned(),
            rule_name: rule.1.to_owned(),
            severity: Severity::Medium,
            kind: FindingKind::Structural,
            matched: text[span.start()..span.end()].to_owned(),
            span,
        }
    }

    /// Builds a repetition finding from a byte range in the scanned text.
    pub fn repetition(rule: (&str, &str), severity: Severity, text: &str, span: Span) -> Finding {
        Finding {
            rule_id: rule.0.to_owned(),
            rule_name: rule.1.to_owned(),
            severity,
            kind: FindingKind::Repetition,
            matched: text[span.start()..span.end()].to_owned(),
            span,
        }
    }

    /// Builds a markdown-aware finding from a byte range in the scanned text.
    pub fn markdown(rule: (&str, &str), severity: Severity, text: &str, span: Span) -> Finding {
        Finding {
            rule_id: rule.0.to_owned(),
            rule_name: rule.1.to_owned(),
            severity,
            kind: FindingKind::Markdown,
            matched: text[span.start()..span.end()].to_owned(),
            span,
        }
    }
}

/// The kind of detector that produced a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    /// Literal phrase matched by Aho-Corasick.
    Phrase,
    /// Character-class match from a structural detector.
    CharacterClass,
    /// Document or sentence structure matched by heuristic detectors.
    Structural,
    /// Repeated document content matched by repetition detectors.
    Repetition,
    /// Markdown syntax matched by markdown-aware detectors.
    Markdown,
}

impl Display for FindingKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FindingKind::Phrase => "phrase",
            FindingKind::CharacterClass => "char",
            FindingKind::Structural => "struct",
            FindingKind::Repetition => "repeat",
            FindingKind::Markdown => "markdown",
        })
    }
}

impl FindingKind {
    pub fn label(self) -> String {
        self.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span(pub usize, pub usize);

impl Span {
    pub fn start(&self) -> usize {
        self.0
    }

    pub fn end(&self) -> usize {
        self.1
    }
}

/// The blank-line separated sections of the text, each trimmed of surrounding
/// whitespace.
///
/// Three detectors count per section, so the split lives here and each of them
/// calls it. A blank line is a line holding nothing but whitespace, which is
/// what makes the split read `\r\n\r\n` as a break; matching `"\n\n"` alone
/// left a CRLF file as one section.
pub(crate) fn paragraph_spans(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut start = 0;
    let mut offset = 0;

    for line in text.split_inclusive('\n') {
        if line.trim().is_empty() {
            push_trimmed_span(text, &mut spans, start, offset);
            start = offset + line.len();
        }

        offset += line.len();
    }

    push_trimmed_span(text, &mut spans, start, text.len());
    spans
}

/// Pushes `start..end` with surrounding whitespace trimmed off, dropping a
/// span that holds only whitespace.
pub(crate) fn push_trimmed_span(text: &str, spans: &mut Vec<Span>, start: usize, end: usize) {
    let value = &text[start..end];

    if value.trim().is_empty() {
        return;
    }

    let leading = value.len() - value.trim_start().len();
    let trailing = value.len() - value.trim_end().len();

    spans.push(Span(start + leading, end - trailing));
}

/// A one-based line and column in the scanned text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    /// One-based line number.
    pub line: usize,
    /// One-based column, counted in characters rather than bytes.
    pub column: usize,
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Resolves byte offsets in scanned text to lines and columns.
///
/// Findings keep byte offsets; a reader needs a line. Building the index once
/// per scan keeps that translation off the hot path of the detectors.
#[derive(Debug)]
pub struct LineIndex<'a> {
    text: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    /// Indexes the line starts of `text`.
    pub fn new(text: &'a str) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(text.match_indices('\n').map(|(offset, _)| offset + 1));

        Self { text, line_starts }
    }

    /// Resolves a byte offset to its line and column.
    ///
    /// An offset past the end of the text resolves to the end of the text.
    /// The carriage return of a `\r\n` pair does not count as a column, so a
    /// span reaching the end of a line reports the same column either way.
    pub fn locate(&self, offset: usize) -> Location {
        let offset = offset.min(self.text.len());
        let line = self.line_starts.partition_point(|start| *start <= offset) - 1;
        let preceding = &self.text[self.line_starts[line]..offset];
        let preceding = preceding.strip_suffix('\r').unwrap_or(preceding);

        Location {
            line: line + 1,
            column: preceding.chars().count() + 1,
        }
    }

    /// Resolves a span to the location of its first and last characters.
    ///
    /// Both ends are inclusive, so the pair covers what a reader would
    /// underline. A span matching nothing reports its start twice.
    pub fn locate_span(&self, span: Span) -> (Location, Location) {
        let last = self.text[..span.end().min(self.text.len())]
            .char_indices()
            .next_back()
            .map(|(offset, _)| offset)
            .filter(|offset| *offset >= span.start())
            .unwrap_or_else(|| span.start());

        (self.locate(span.start()), self.locate(last))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_detector_finds_phrase_patterns() {
        let detector = Detector::bundled().unwrap();
        let findings = detector.scan("Let's delve into this robust ecosystem.");

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "word_choice.delve")
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "word_choice.grandiose_nouns")
        );
    }

    #[test]
    fn phrase_matching_is_case_insensitive() {
        let detector = Detector::bundled().unwrap();
        let findings = detector.scan("We should DELVE INTO the details.");

        assert_eq!(findings[0].rule_id, "word_choice.delve");
        assert_eq!(findings[0].matched, "DELVE INTO");
    }

    #[test]
    fn a_line_index_locates_offsets_on_the_first_line() {
        let index = LineIndex::new("delve into this");

        assert_eq!(index.locate(0), Location { line: 1, column: 1 });
        assert_eq!(index.locate(6), Location { line: 1, column: 7 });
    }

    #[test]
    fn a_line_index_counts_lines_from_one() {
        let text = "first\nsecond\n\nfourth";
        let index = LineIndex::new(text);

        assert_eq!(index.locate(6), Location { line: 2, column: 1 });
        assert_eq!(index.locate(13), Location { line: 3, column: 1 });
        assert_eq!(
            index.locate(text.find("fourth").unwrap()),
            Location { line: 4, column: 1 }
        );
    }

    #[test]
    fn a_column_counts_characters_rather_than_bytes() {
        let text = "a → b";
        let index = LineIndex::new(text);

        assert_eq!(index.locate(text.find('→').unwrap()).column, 3);
        assert_eq!(index.locate(text.find('b').unwrap()).column, 5);
    }

    #[test]
    fn an_offset_past_the_end_resolves_to_the_end() {
        let index = LineIndex::new("one\ntwo");

        assert_eq!(index.locate(999), Location { line: 2, column: 4 });
    }

    #[test]
    fn carriage_returns_do_not_count_as_columns() {
        let text = "first\r\nsecond\r\n";
        let index = LineIndex::new(text);

        assert_eq!(index.locate(0), Location { line: 1, column: 1 });
        assert_eq!(
            index.locate(text.find('\r').unwrap()),
            Location { line: 1, column: 6 }
        );
        assert_eq!(
            index.locate(text.find("second").unwrap()),
            Location { line: 2, column: 1 }
        );
    }

    #[test]
    fn a_span_locates_its_first_and_last_characters() {
        let text = "one two\nthree four\n";
        let index = LineIndex::new(text);

        assert_eq!(&text[4..7], "two");
        assert_eq!(
            index.locate_span(Span(4, 7)),
            (
                Location { line: 1, column: 5 },
                Location { line: 1, column: 7 }
            )
        );
        assert_eq!(
            index.locate_span(Span(0, 13)),
            (
                Location { line: 1, column: 1 },
                Location { line: 2, column: 5 }
            )
        );
    }

    #[test]
    fn an_empty_span_locates_its_start_twice() {
        let index = LineIndex::new("one two\n");
        let start = Location { line: 1, column: 5 };

        assert_eq!(index.locate_span(Span(4, 4)), (start, start));
    }

    #[test]
    fn a_location_displays_as_line_and_column() {
        assert_eq!(
            Location {
                line: 12,
                column: 3
            }
            .to_string(),
            "12:3"
        );
    }

    #[test]
    fn findings_resolve_to_the_line_they_were_found_on() {
        let detector = Detector::bundled().unwrap();
        let text = "A clean opening line.\nLet us delve into this.\n";
        let findings = detector.scan(text);
        let index = LineIndex::new(text);

        let delve = findings
            .iter()
            .find(|finding| finding.rule_id == "word_choice.delve")
            .expect("the second line matches word_choice.delve");

        assert_eq!(
            index.locate(delve.span.start()),
            Location { line: 2, column: 8 }
        );
    }

    #[test]
    fn a_scan_drops_the_findings_the_text_suppressed() {
        let detector = Detector::bundled().unwrap();
        let text = "trps-ignore-next-line\nLet us delve into this.\nLet us delve into that.\n";
        let findings = detector.scan(text);
        let delve: Vec<_> = findings
            .iter()
            .filter(|finding| finding.rule_id == "word_choice.delve")
            .collect();

        assert_eq!(delve.len(), 1);
        assert_eq!(
            LineIndex::new(text).locate(delve[0].span.start()).line,
            3,
            "only the marked line is suppressed"
        );
    }

    #[test]
    fn a_marker_naming_a_rule_leaves_the_other_findings_alone() {
        let detector = Detector::bundled().unwrap();
        let text =
            "trps-ignore-next-line word_choice.delve\nLet us delve into a → b → c → world.\n";
        let findings = detector.scan(text);

        assert!(
            !findings
                .iter()
                .any(|finding| finding.rule_id == "word_choice.delve")
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == char_class::UNICODE_DECORATION_RULE_ID)
        );
    }

    #[test]
    fn sections_split_on_a_blank_line_whatever_the_line_ending() {
        let unix = "One dash.\n\nAnother dash.\n";
        let windows = "One dash.\r\n\r\nAnother dash.\r\n";

        assert_eq!(paragraph_spans(unix).len(), 2);
        assert_eq!(paragraph_spans(windows).len(), 2);
    }

    #[test]
    fn scan_includes_unicode_decoration() {
        let detector = Detector::bundled().unwrap();
        let findings = detector.scan("Input → output → result → done");

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == char_class::UNICODE_DECORATION_RULE_ID)
        );
    }
}
