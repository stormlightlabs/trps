//! Text detectors for phrase-based and structural trope signals.

pub(crate) mod char_class;
pub(crate) mod cross_file;
pub(crate) mod dialect;
pub(crate) mod markdown;
pub(crate) mod repetition;
pub(crate) mod structural;

use std::fmt::Display;
use std::path::Path;

use aho_corasick::{AhoCorasick, MatchKind};

use crate::detector::dialect::{Dialect, DialectRule};
use crate::detector::markdown::MarkdownOptions;
use crate::errors::DetectorBuildError;
use crate::patterns::{Pattern, Severity, Thresholds};
use crate::patterns::{bundled_patterns, validate_patterns};
use crate::suppression::Suppressions;

/// Rule ids the detectors compiled into this crate can report.
///
/// The phrase rules come out of the loaded dictionaries, so [`Detector`] holds
/// those; these have no dictionary entry to read. Anything asking whether a
/// rule id exists needs both, which [`Detector::rule_ids`] joins.
pub const BUILTIN_RULE_IDS: &[&str] = &[
    char_class::EM_DASH_ADDICTION.0,
    char_class::UNICODE_DECORATION_RULE_ID,
    cross_file::CROSS_FILE_DUPLICATION.0,
    dialect::DIALECT_SPELLING.0,
    markdown::BOLD_FIRST_LEADS.0,
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

/// Extensions a path carrying Markdown is named with.
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown", "mdx"];

/// What kind of text a scan is reading.
///
/// It decides one thing: whether the code in a document is code. A
/// suppression marker written inside a fence or an inline span is an example
/// of a marker in Markdown, and is a marker anywhere else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    /// Prose carrying no markup. Text read from stdin has no path to read a
    /// format from and is this one.
    #[default]
    PlainText,
    /// Markdown, named by the extension on its path.
    Markdown,
}

impl Format {
    /// The format `path` is named for, from its extension.
    pub fn of(path: &Path) -> Self {
        let markdown = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                MARKDOWN_EXTENSIONS
                    .iter()
                    .any(|known| extension.eq_ignore_ascii_case(known))
            });

        match markdown {
            true => Self::Markdown,
            false => Self::PlainText,
        }
    }
}

/// Finds trope signals in prose.
#[derive(Debug)]
pub struct Detector {
    phrase_patterns: Vec<Pattern>,
    phrase_to_pattern: Vec<usize>,
    phrase_matcher: AhoCorasick,
    dialect: Option<DialectRule>,
    thresholds: Thresholds,
    markdown: MarkdownOptions,
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
                phrases.push(collapse_whitespace(phrase));
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
            dialect: None,
            thresholds: Thresholds::default(),
            markdown: MarkdownOptions::default(),
        })
    }

    /// Turns on the dialect rule, which [`Detector::new`] leaves off. It has
    /// no default because neither spelling is wrong until a project has
    /// chosen one, and a default would report half of a British corpus.
    pub fn with_dialect(mut self, dialect: Dialect) -> Result<Self, DetectorBuildError> {
        self.dialect = Some(DialectRule::new(dialect)?);

        Ok(self)
    }

    /// Applies the counts a project tuned, which [`Detector::new`] leaves at
    /// the ones the tool ships with.
    pub fn with_thresholds(mut self, thresholds: Thresholds) -> Self {
        self.thresholds = thresholds;
        self
    }

    /// Applies what a project decided about reading Markdown, which
    /// [`Detector::new`] leaves at the defaults.
    pub fn with_markdown(mut self, markdown: MarkdownOptions) -> Self {
        self.markdown = markdown;
        self
    }

    /// The counts this detector scans at.
    ///
    /// A rule reading a count inside [`Detector::scan`] takes it from here. A
    /// rule that reads a whole run rather than one text is scanned by the
    /// caller, so `composition.cross_file_duplication` takes its entry
    /// through this.
    pub fn thresholds(&self) -> &Thresholds {
        &self.thresholds
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
    /// Fenced blocks and front matter are blanked before anything reads the
    /// text, so no detector grades a quoted command or a metadata key.
    ///
    /// Findings the text suppressed in place are dropped here rather than by
    /// the caller, so every reader of a scan sees the same document.
    ///
    /// A rule that reports one span twice is reported once.
    /// `composition.content_duplication` reaches a paragraph holding a single
    /// sentence through both of its passes, and two findings agreeing on
    /// every field describe one problem.
    ///
    /// Findings come back ordered by span, so the ones covering the same text
    /// sit together and [`group_by_span`] can collect them.
    pub fn scan(&self, text: &str) -> Vec<Finding> {
        self.scan_as(text, Format::default())
    }

    /// Scans text read as `format`.
    ///
    /// The format decides whether the code in the text is code, which is what
    /// [`Detector::suppressions`] reads it for. Everything [`Detector::scan`]
    /// documents holds here too.
    pub fn scan_as(&self, text: &str, format: Format) -> Vec<Finding> {
        let suppressions = self.suppressions(text, format);
        let masked = markdown::mask_non_prose(text);
        let text = masked.as_str();
        let mut findings = self.scan_phrases(text);

        findings.extend(char_class::scan_em_dash_addiction(
            text,
            self.thresholds.em_dash_addiction,
        ));
        findings.extend(char_class::scan_unicode_decoration(
            text,
            self.thresholds.unicode_decoration,
        ));
        findings.extend(markdown::scan_markdown(
            text,
            self.thresholds.bold_first_leads,
        ));
        findings.extend(structural::scan_structural(
            text,
            self.thresholds.structural,
        ));
        findings.extend(repetition::scan_repetition(
            text,
            self.thresholds.repetition,
        ));

        if let Some(rule) = &self.dialect {
            findings.extend(rule.scan(text));
        }

        findings.retain(|finding| !suppressions.covers(finding.span.start(), &finding.rule_id));
        findings.sort_by_key(|finding| (finding.span.start(), finding.span.end()));
        findings.dedup();
        findings
    }

    /// Reads the suppression markers of `text`, as a scan of it would.
    ///
    /// A caller that reports on the markers rather than scanning, as a run
    /// warning about one that names no rule does, takes them from here. Both
    /// then read one document: a marker this skips suppresses nothing and is
    /// warned about by nobody.
    ///
    /// A fenced block in Markdown is never a marker, whatever the project
    /// decided about the inline spans. Nothing in a fence is graded, so a
    /// marker there suppresses only the prose after it, which is the silent
    /// whole-file suppression the skip exists to stop.
    pub fn suppressions(&self, text: &str, format: Format) -> Suppressions {
        match (format, self.markdown.skip_markers_in_code) {
            (Format::Markdown, true) => Suppressions::in_markdown(text),
            (Format::Markdown, false) => Suppressions::new(&markdown::mask_non_prose(text)),
            (Format::PlainText, _) => Suppressions::new(text),
        }
    }

    /// Finds the phrases, over a copy of the text with its wraps undone.
    ///
    /// A phrase is a run of words, and where the text wraps mid-phrase the
    /// words are the same ones. The match runs over [`reflow`]'s copy and every
    /// span comes back in the offsets of `text`, so a finding quotes the
    /// wrapped text and reports the line and column it was written at.
    fn scan_phrases(&self, text: &str) -> Vec<Finding> {
        let reflowed = reflow(text);

        self.phrase_matcher
            .find_iter(reflowed.text.as_str())
            .map(|mat| {
                let phrase_index = mat.pattern().as_usize();
                let pattern = &self.phrase_patterns[self.phrase_to_pattern[phrase_index]];
                let span = reflowed.span(mat.start(), mat.end());

                Finding {
                    rule_id: pattern.id.clone(),
                    rule_name: pattern.name.clone(),
                    severity: pattern.severity,
                    kind: FindingKind::Phrase,
                    matched: text[span.start()..span.end()].to_owned(),
                    expected: None,
                    span,
                }
            })
            .collect()
    }
}

/// A copy of the scanned text with its line wraps undone, and the offsets
/// that lead back to it.
///
/// The phrase matcher compares bytes, so a phrase written with a space finds
/// nothing where the text broke the line instead. Matching the copy and
/// reporting the original is what lets one paragraph wrapped at two widths
/// report the same rules.
struct Reflowed {
    text: String,
    /// The offset into the scanned text of each byte of `text`, and of the end
    /// of the scanned text at the last entry, so the end of a match maps as
    /// readily as its start.
    offsets: Vec<usize>,
}

impl Reflowed {
    /// The span of the scanned text that `start..end` of the copy covers.
    fn span(&self, start: usize, end: usize) -> Span {
        Span(self.offsets[start], self.offsets[end])
    }
}

/// Copies `text` with every line wrap collapsed to the single space it stands
/// in for.
///
/// A wrap is any run of whitespace holding at most one newline, so the break
/// itself, the indent that follows it, and the two spaces somebody left after
/// a full stop all read as one space. A block quote repeats its `>` on every
/// line it runs over, so the marker goes with the break it follows and a
/// quoted paragraph wraps like any other.
///
/// A run holding two newlines is a paragraph break and is copied as it was
/// written: the halves either side of it belong to different sentences, and no
/// phrase should reach across them.
fn reflow(text: &str) -> Reflowed {
    let mut reflowed = String::with_capacity(text.len());
    let mut offsets = Vec::with_capacity(text.len() + 1);
    let mut characters = text.char_indices().peekable();

    while let Some((offset, character)) = characters.next() {
        if !character.is_whitespace() {
            offsets.extend(std::iter::repeat_n(offset, character.len_utf8()));
            reflowed.push(character);
            continue;
        }

        let mut end = offset + character.len_utf8();
        let mut newlines = usize::from(character == '\n');

        loop {
            while let Some(&(at, next)) = characters.peek() {
                if !next.is_whitespace() {
                    break;
                }

                newlines += usize::from(next == '\n');
                end = at + next.len_utf8();
                characters.next();
            }

            match characters.peek() {
                Some(&(at, '>')) if newlines > 0 => {
                    end = at + 1;
                    characters.next();
                }
                _ => break,
            }
        }

        match newlines < 2 {
            true => {
                offsets.push(offset);
                reflowed.push(' ');
            }
            false => {
                for (at, character) in text[offset..end].char_indices() {
                    offsets.extend(std::iter::repeat_n(offset + at, character.len_utf8()));
                }

                reflowed.push_str(&text[offset..end]);
            }
        }
    }

    offsets.push(text.len());

    Reflowed {
        text: reflowed,
        offsets,
    }
}

/// Collapses the whitespace inside a phrase, so a dictionary that wrapped one
/// across two lines matches what [`reflow`] produces.
fn collapse_whitespace(phrase: &str) -> String {
    phrase.split_whitespace().collect::<Vec<_>>().join(" ")
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
    /// The form the rule expected in place of `matched`.
    /// `word_choice.dialect_spelling` names the spelling the project's
    /// dialect uses; no other rule sets this.
    pub expected: Option<String>,
    /// Start & end byte offset.
    pub span: Span,
}

impl Finding {
    /// Builds a structural finding from a byte range in the scanned text.
    ///
    /// The structural rules all report at [`Severity::Medium`], so this
    /// takes no severity where [`Finding::repetition`] and
    /// [`Finding::markdown`] do.
    pub(crate) fn structural(rule: (&str, &str), text: &str, span: Span) -> Finding {
        Finding {
            rule_id: rule.0.to_owned(),
            rule_name: rule.1.to_owned(),
            severity: Severity::Medium,
            kind: FindingKind::Structural,
            matched: text[span.start()..span.end()].to_owned(),
            expected: None,
            span,
        }
    }

    /// Builds a repetition finding from a byte range in the scanned text.
    pub(crate) fn repetition(
        rule: (&str, &str),
        severity: Severity,
        text: &str,
        span: Span,
    ) -> Finding {
        Finding {
            rule_id: rule.0.to_owned(),
            rule_name: rule.1.to_owned(),
            severity,
            kind: FindingKind::Repetition,
            matched: text[span.start()..span.end()].to_owned(),
            expected: None,
            span,
        }
    }

    /// Builds a markdown-aware finding from a byte range in the scanned text.
    pub(crate) fn markdown(
        rule: (&str, &str),
        severity: Severity,
        text: &str,
        span: Span,
    ) -> Finding {
        Finding {
            rule_id: rule.0.to_owned(),
            rule_name: rule.1.to_owned(),
            severity,
            kind: FindingKind::Markdown,
            matched: text[span.start()..span.end()].to_owned(),
            expected: None,
            span,
        }
    }
}

/// Collects the findings that cover exactly the same span.
///
/// Several heuristics can see one passage. Three short sentences opening the
/// same way trip both `sentence_structure.anaphora_abuse` and
/// `paragraph_structure.short_punchy_fragments`. Printed apart they show the
/// reader those sentences twice and leave the reader to work out that it is
/// one problem, so a report quotes the passage once under every rule that
/// fired on it.
///
/// Spans have to be equal rather than merely overlap. A paragraph rule covers
/// every phrase finding inside it, and `composition.fractal_summaries` can
/// cover a whole document. Folding a contained finding into the one around it
/// would hide the rules that say the most about the text.
///
/// The findings must be ordered the way [`Detector::scan`] returns them,
/// which puts equal spans next to each other. Each group holds at least one
/// finding.
pub fn group_by_span(findings: &[Finding]) -> impl Iterator<Item = &[Finding]> {
    findings.chunk_by(|left, right| left.span == right.span)
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
    /// A word spelled against the project's dialect.
    Spelling,
}

impl Display for FindingKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FindingKind::Phrase => "phrase",
            FindingKind::CharacterClass => "char",
            FindingKind::Structural => "struct",
            FindingKind::Repetition => "repeat",
            FindingKind::Markdown => "markdown",
            FindingKind::Spelling => "spelling",
        })
    }
}

impl FindingKind {
    /// The short name a report prints for the kind, as [`Display`] writes
    /// it. Owned, for a consumer that needs a `String` rather than a
    /// formatter.
    pub fn label(self) -> String {
        self.to_string()
    }
}

/// A byte range into the text that was scanned: the start offset and the
/// end offset, the same half-open range a slice takes.
///
/// Offsets are bytes rather than characters, so they index the scanned
/// `str` directly. [`LineIndex`] turns one into a line and column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span(pub usize, pub usize);

impl Span {
    /// The byte offset the matched text starts at.
    pub fn start(&self) -> usize {
        self.0
    }

    /// The byte offset one past the matched text.
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

/// The sentences of `text`, grouped by the block of prose they sit in.
///
/// Four detectors count sentences and every one of them counts them inside a
/// paragraph, so both the split and the grouping live here.
///
/// A sentence never reaches across a line markdown reads as structure, and a
/// run of them never spans two blocks. Three bullets opening the same way are
/// a list rather than anaphora, and a one-line lead-in before a list is not a
/// run of fragments with the list's own wrapped lines.
///
/// A terminator inside a token does not end a sentence either: splitting on
/// every `.` read `src/lib.rs` and `v0.1.1` as three sentences each, and a
/// rule counting short sentences saw a path as a run of fragments.
pub(crate) fn prose_sentences(text: &str) -> Vec<Vec<Span>> {
    prose_blocks(text)
        .into_iter()
        .map(|block| split_sentences(text, block))
        .filter(|sentences| !sentences.is_empty())
        .collect()
}

/// Every sentence of `text` in the order it appears, with the block
/// boundaries [`prose_sentences`] keeps dropped.
pub(crate) fn sentence_spans(text: &str) -> Vec<Span> {
    prose_sentences(text).concat()
}

/// Splits one block of prose into sentences.
fn split_sentences(text: &str, block: Span) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut start = block.start();

    for (offset, character) in text[block.start()..block.end()].char_indices() {
        let end = block.start() + offset + character.len_utf8();

        if matches!(character, '.' | '!' | '?') && ends_sentence(text, end) {
            push_trimmed_span(text, &mut spans, start, end);
            start = end;
        }
    }

    push_trimmed_span(text, &mut spans, start, block.end());
    spans
}

/// Whether the terminator ending at `end` closes a sentence rather than
/// sitting inside a token. Nothing after it, or whitespace, closes one.
fn ends_sentence(text: &str, end: usize) -> bool {
    text[end..].chars().next().is_none_or(char::is_whitespace)
}

/// Maximal runs of adjacent prose lines, each trimmed of surrounding
/// whitespace.
///
/// A blank line ends a run and so does a line of markdown structure. An
/// indented line under structure continues it, which is how a bullet that
/// wraps onto a second line stays part of its bullet instead of reading as a
/// paragraph of its own.
fn prose_blocks(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut start = None;
    let mut structure = false;
    let mut offset = 0;

    for line in text.split_inclusive('\n') {
        let blank = line.trim().is_empty();

        if !blank && markdown::is_prose_line(line) && !(structure && is_indented(line)) {
            structure = false;
            start.get_or_insert(offset);
        } else {
            structure |= !blank;

            if let Some(block) = start.take() {
                push_trimmed_span(text, &mut spans, block, offset);
            }
        }

        offset += line.len();
    }

    if let Some(block) = start {
        push_trimmed_span(text, &mut spans, block, text.len());
    }

    spans
}

/// Whether a line opens with enough whitespace to hang off the line above it.
fn is_indented(line: &str) -> bool {
    line.len() - line.trim_start().len() >= 2
}

/// The words of `text`: runs of alphanumerics and apostrophes.
///
/// Three detectors count words, so the split lives here and each of them
/// calls it.
pub(crate) fn word_spans(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut start = None;

    for (index, character) in text.char_indices() {
        if character.is_ascii_alphanumeric() || character == '\'' {
            start.get_or_insert(index);
        } else if let Some(word_start) = start.take() {
            spans.push(Span(word_start, index));
        }
    }

    if let Some(word_start) = start {
        spans.push(Span(word_start, text.len()));
    }

    spans
}

/// Whether `hits` across `words` words is at least `per_hundred` of them in
/// every hundred words.
///
/// The rate comes from the dictionary, which is input, so the products are
/// saturating rather than plain. A `per_hundred` a TOML integer holds overflows
/// `per_hundred * words` on any document worth scanning, and the rule should
/// answer no to a rate no document reaches rather than panic on a debug build
/// and wrap to an answer of its own on a release one. Saturation changes no
/// answer a document can produce: only some hundred quadrillion hits reach the
/// left side of it.
pub(crate) fn reaches_rate(hits: usize, per_hundred: usize, words: usize) -> bool {
    hits.saturating_mul(100) >= per_hundred.saturating_mul(words)
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

    use crate::patterns::PatternFile;

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

    /// Whether a scan of `text` read as `format` reports the phrase rule the
    /// text carries.
    fn reports_delve(detector: &Detector, text: &str, format: Format) -> bool {
        detector
            .scan_as(text, format)
            .iter()
            .any(|finding| finding.rule_id == "word_choice.delve")
    }

    #[test]
    fn a_marker_inside_an_inline_span_is_prose_about_a_marker() {
        let detector = Detector::bundled().unwrap();
        let text = "Write it as `trps-ignore-next-line`\nLet's delve into this.";

        assert!(reports_delve(&detector, text, Format::Markdown));
        assert!(!reports_delve(&detector, text, Format::PlainText));
    }

    #[test]
    fn a_marker_inside_a_fence_is_prose_about_a_marker() {
        let detector = Detector::bundled().unwrap();
        let text = "```text\ntrps-ignore-start\n```\n\nLet's delve into this.";

        assert!(reports_delve(&detector, text, Format::Markdown));
        assert!(!reports_delve(&detector, text, Format::PlainText));
    }

    #[test]
    fn a_backtick_that_never_closes_leaves_the_markers_after_it_alone() {
        let detector = Detector::bundled().unwrap();
        let text = "A ` opens nothing.\n\ntrps-ignore-next-line\nLet's delve into this.";

        assert!(!reports_delve(&detector, text, Format::Markdown));
    }

    #[test]
    fn a_stray_backtick_does_not_reach_the_code_span_below_it() {
        let detector = Detector::bundled().unwrap();
        let text = "<!-- trps-ignore-start -->\nHe said `hello.\n\
<!-- trps-ignore-end -->\nA line with `code` in it.\n\nLet's delve into this.";

        assert!(reports_delve(&detector, text, Format::Markdown));
    }

    #[test]
    fn a_project_can_turn_the_skip_off() {
        let detector = Detector::bundled().unwrap().with_markdown(MarkdownOptions {
            skip_markers_in_code: false,
        });
        let text = "Write it as `trps-ignore-next-line`\nLet's delve into this.";

        assert!(!reports_delve(&detector, text, Format::Markdown));
    }

    #[test]
    fn a_marker_in_a_fence_stays_out_of_markdown_with_the_skip_off() {
        let detector = Detector::bundled().unwrap().with_markdown(MarkdownOptions {
            skip_markers_in_code: false,
        });
        let text = "```text\ntrps-ignore-start\n```\n\nLet's delve into this.";

        assert!(reports_delve(&detector, text, Format::Markdown));
    }

    #[test]
    fn markdown_is_the_extension_on_the_path() {
        assert_eq!(Format::of(Path::new("notes.md")), Format::Markdown);
        assert_eq!(Format::of(Path::new("notes.MARKDOWN")), Format::Markdown);
        assert_eq!(Format::of(Path::new("docs/page.mdx")), Format::Markdown);
        assert_eq!(Format::of(Path::new("notes.txt")), Format::PlainText);
        assert_eq!(Format::of(Path::new("README")), Format::PlainText);
    }

    #[test]
    fn a_phrase_matches_across_a_line_wrap() {
        let detector = Detector::bundled().unwrap();
        let findings = detector.scan("We should delve\ninto the details.");

        assert_eq!(findings[0].rule_id, "word_choice.delve");
        assert_eq!(findings[0].matched, "delve\ninto");
    }

    #[test]
    fn a_phrase_matches_across_a_wrap_and_the_indent_under_it() {
        let detector = Detector::bundled().unwrap();
        let findings = detector.scan("- We should delve\n  into the details.");

        assert_eq!(findings[0].rule_id, "word_choice.delve");
        assert_eq!(findings[0].matched, "delve\n  into");
    }

    #[test]
    fn a_phrase_matches_across_a_crlf_pair() {
        let detector = Detector::bundled().unwrap();
        let findings = detector.scan("We should delve\r\ninto the details.");

        assert_eq!(findings[0].rule_id, "word_choice.delve");
        assert_eq!(findings[0].matched, "delve\r\ninto");
    }

    #[test]
    fn a_phrase_matches_across_a_wrap_inside_a_block_quote() {
        let detector = Detector::bundled().unwrap();
        let findings = detector.scan("> We should delve\n> into the details.");

        assert_eq!(findings[0].rule_id, "word_choice.delve");
        assert_eq!(findings[0].matched, "delve\n> into");
    }

    #[test]
    fn a_phrase_does_not_match_across_a_blank_line_in_a_block_quote() {
        let detector = Detector::bundled().unwrap();
        let findings = detector.scan("> We should delve\n>\n> into the details.");

        assert!(
            !findings
                .iter()
                .any(|finding| finding.rule_id == "word_choice.delve")
        );
    }

    #[test]
    fn a_phrase_does_not_match_across_a_paragraph_break() {
        let detector = Detector::bundled().unwrap();
        let findings = detector.scan("We should delve\n\ninto the details.");

        assert!(
            !findings
                .iter()
                .any(|finding| finding.rule_id == "word_choice.delve")
        );
    }

    #[test]
    fn one_paragraph_wrapped_two_ways_reports_the_same_rules() {
        let detector = Detector::bundled().unwrap();
        let narrow = "We should delve\ninto the details.";
        let wide = "We should delve into the details.";

        let rules = |text: &str| -> Vec<String> {
            detector
                .scan(text)
                .into_iter()
                .map(|finding| finding.rule_id)
                .collect()
        };

        assert_eq!(rules(narrow), rules(wide));

        let finding = &detector.scan(narrow)[0];
        let (start, end) = LineIndex::new(narrow).locate_span(finding.span);

        assert_eq!((start.line, start.column), (1, 11));
        assert_eq!((end.line, end.column), (2, 4));
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
    fn a_terminator_inside_a_token_does_not_end_a_sentence() {
        let text = "The loader reads v0.1.1 from src/lib.rs at startup.";

        assert_eq!(sentence_spans(text), vec![Span(0, text.len())]);
    }

    #[test]
    fn a_sentence_ends_at_a_terminator_whitespace_follows() {
        let text = "One sentence. Two sentences.";
        let spans = sentence_spans(text);

        assert_eq!(spans.len(), 2);
        assert_eq!(&text[spans[0].start()..spans[0].end()], "One sentence.");
        assert_eq!(&text[spans[1].start()..spans[1].end()], "Two sentences.");
    }

    #[test]
    fn a_sentence_does_not_reach_across_markdown_structure() {
        let text = "A lead-in line.\n\n- A bullet.\n\nA closing line.\n";
        let blocks = prose_sentences(text);

        assert_eq!(blocks.len(), 2, "the bullet is structure rather than prose");
        assert_eq!(
            &text[blocks[0][0].start()..blocks[0][0].end()],
            "A lead-in line."
        );
        assert_eq!(
            &text[blocks[1][0].start()..blocks[1][0].end()],
            "A closing line."
        );
    }

    #[test]
    fn a_wrapped_bullet_stays_part_of_its_bullet() {
        let text = "- A bullet that runs on\n  and ends here.\n\nA paragraph.\n";
        let blocks = prose_sentences(text);

        assert_eq!(blocks.len(), 1);
        assert_eq!(
            &text[blocks[0][0].start()..blocks[0][0].end()],
            "A paragraph."
        );
    }

    #[test]
    fn a_fenced_block_is_not_graded() {
        let detector = Detector::bundled().unwrap();
        let text = "Prose above.\n\n```text\nHe published this. Openly. In a book.\n```\n";

        assert_eq!(detector.scan(text), Vec::new());
    }

    #[test]
    fn a_run_of_bullets_sharing_an_opening_is_a_list_rather_than_anaphora() {
        let detector = Detector::bundled().unwrap();
        let text = "- The loader reads the file.\n- The loader merges the two.\n- The loader records the path.\n";

        assert_eq!(detector.scan(text), Vec::new());
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
    fn findings_covering_one_passage_group_into_one_entry() {
        let detector = Detector::bundled().unwrap();
        let text = "We built it fast.\nWe built it wrong.\nWe built it twice.\n";
        let findings = detector.scan(text);
        let groups: Vec<_> = group_by_span(&findings).collect();

        assert_eq!(groups.len(), 1, "one passage is one entry");
        assert_eq!(
            groups[0]
                .iter()
                .map(|finding| finding.rule_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                structural::ANAPHORA_ABUSE.0,
                structural::SHORT_PUNCHY_FRAGMENTS.0,
            ]
        );
    }

    #[test]
    fn a_finding_inside_another_stays_its_own_entry() {
        let detector = Detector::bundled().unwrap();
        let text = "It's not bold. It's backwards. Not a bug. Not a feature. Just a habit.\n";
        let findings = detector.scan(text);
        let groups: Vec<_> = group_by_span(&findings).collect();

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "sentence_structure.not_x_not_y"),
            "the phrase rule fires inside the paragraph the structural rule covers"
        );
        assert_eq!(groups.len(), findings.len(), "no two spans are equal");
    }

    #[test]
    fn one_rule_reporting_a_passage_twice_reports_it_once() {
        let detector = Detector::bundled().unwrap();
        let paragraph = "The loader reads the dictionary and merges it over the bundled one.\n";
        let text = format!("{paragraph}\n{paragraph}");
        let duplication: Vec<_> = detector
            .scan(&text)
            .into_iter()
            .filter(|finding| finding.rule_id == repetition::CONTENT_DUPLICATION.0)
            .collect();

        assert_eq!(
            duplication.len(),
            1,
            "the paragraph pass and the sentence pass describe one repetition"
        );
    }

    #[test]
    fn a_detector_carries_the_counts_a_dictionary_tuned() {
        let thresholds = PatternFile::from_toml(
            r#"
[thresholds."composition.cross_file_duplication"]
min_words = 20
"#,
        )
        .unwrap()
        .thresholds;
        let detector = Detector::bundled().unwrap().with_thresholds(thresholds);

        assert_eq!(
            detector.thresholds().cross_file_duplication.min_words,
            20,
            "the entry the cross-file rule reads travels on the detector"
        );
        assert_eq!(
            Detector::bundled().unwrap().thresholds(),
            &Thresholds::default()
        );
    }

    #[test]
    fn a_lowered_count_reaches_the_rule_that_reads_it() {
        let thresholds = PatternFile::from_toml(
            r#"
[thresholds."formatting.unicode_decoration"]
min_occurrences = 2
"#,
        )
        .unwrap()
        .thresholds;
        let text = "Input → output → result";
        let decoration = |detector: Detector| {
            detector
                .scan(text)
                .iter()
                .any(|finding| finding.rule_id == char_class::UNICODE_DECORATION_RULE_ID)
        };

        assert!(!decoration(Detector::bundled().unwrap()));
        assert!(decoration(
            Detector::bundled().unwrap().with_thresholds(thresholds)
        ));
    }

    #[test]
    fn a_lowered_count_reaches_a_structural_or_repetition_rule_too() {
        let thresholds = PatternFile::from_toml(
            r#"
[thresholds."composition.dead_metaphor"]
min_repeats = 4
"#,
        )
        .unwrap()
        .thresholds;
        let text = "The ecosystem needs ecosystem value. This ecosystem has ecosystem tools.";
        let metaphor = |detector: Detector| {
            detector
                .scan(text)
                .iter()
                .any(|finding| finding.rule_id == repetition::DEAD_METAPHOR.0)
        };

        assert!(!metaphor(Detector::bundled().unwrap()));
        assert!(metaphor(
            Detector::bundled().unwrap().with_thresholds(thresholds)
        ));
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
