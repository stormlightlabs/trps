//! A repetition detector that reads a whole run rather than one file.
//!
//! [`super::repetition`] compares passages inside one text, so a paragraph
//! pasted into three files with the nouns changed is clean in every one of
//! them, and a closing phrase reused across a week of documents is clean in
//! each. This rule compares the files of a run against each other and reports
//! the longest word runs they share.
//!
//! What to do about a repetition is the reader's call. A house convention and
//! a tic look the same from here, so a finding names every place the run
//! appears and says nothing about which one it is.

use std::collections::HashMap;

use serde::Deserialize;

use crate::patterns::Severity;
use crate::suppression::Suppressions;

use super::{Span, markdown, word_spans};

/// A run of words several files share.
pub const CROSS_FILE_DUPLICATION: (&str, &str) = (
    "composition.cross_file_duplication",
    "Cross-File Duplication",
);

/// What a run counts as a repetition worth reporting.
///
/// Shared vocabulary is the common case and belongs below these: a repository
/// naming `status:queued` in nine files has nine correct uses. The defaults
/// are set where a shared run is long enough to have been written once and
/// copied, and a project that disagrees raises them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CrossFileLimits {
    /// Words a shared run needs before it is reported. Below two the rule
    /// would report the vocabulary of the corpus, so two is the floor.
    pub min_words: usize,
    /// Files a shared run has to appear in. Below two it is not a cross-file
    /// repetition, so two is the floor there as well.
    pub min_files: usize,
}

impl Default for CrossFileLimits {
    fn default() -> Self {
        Self {
            min_words: 6,
            min_files: 2,
        }
    }
}

/// A run of words shared by several of the texts a scan was given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossFileFinding {
    /// Stable rule id.
    pub rule_id: String,
    /// Human-readable rule name.
    pub rule_name: String,
    /// Finding severity.
    pub severity: Severity,
    /// The shared run as the first text writes it. The others match it word
    /// for word once case and punctuation are set aside.
    pub matched: String,
    /// Every place the run appears, in the order the texts were given.
    pub occurrences: Vec<Occurrence>,
}

/// One place a shared run appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occurrence {
    /// Index of the text in the slice the scan was given.
    pub document: usize,
    /// Start and end byte offset in that text.
    pub span: Span,
}

/// One word of a text, lowercased for comparison.
struct Word {
    span: Span,
    lowercase: String,
}

/// Where a shared run starts: a text, and a word index into it.
#[derive(Clone, Copy)]
struct Position {
    document: usize,
    word: usize,
}

/// Finds word runs shared by several of `texts`.
///
/// Fenced blocks and front matter are blanked first, so a command quoted in
/// two guides is not a repetition, and a run a file suppressed in place is
/// dropped from that file's side of the finding.
///
/// A run given fewer texts than [`CrossFileLimits::min_files`] reports
/// nothing, which is what keeps a one-file run and a read of stdin behaving
/// as they did before the rule existed.
pub fn scan_cross_file(texts: &[&str], limits: CrossFileLimits) -> Vec<CrossFileFinding> {
    let min_words = limits.min_words.max(2);
    let min_files = limits.min_files.max(2);

    if texts.len() < min_files {
        return Vec::new();
    }

    let masked: Vec<String> = texts
        .iter()
        .map(|text| markdown::mask_non_prose(text))
        .collect();
    let documents: Vec<Vec<Word>> = masked.iter().map(|text| words(text.as_str())).collect();
    let suppressions: Vec<Suppressions> = masked
        .iter()
        .map(|text| Suppressions::new(text.as_str()))
        .collect();

    let mut findings: Vec<CrossFileFinding> = shared_starts(&documents, min_words)
        .into_iter()
        .filter(|positions| files(positions.iter().map(|position| position.document)) >= min_files)
        .filter(|positions| !share_a_preceding_word(&documents, positions))
        .filter_map(|positions| {
            let length = shared_length(&documents, &positions, min_words);
            let occurrences: Vec<Occurrence> = positions
                .iter()
                .map(|position| occurrence(&documents, *position, length))
                .filter(|occurrence| {
                    !suppressions[occurrence.document]
                        .covers(occurrence.span.start(), CROSS_FILE_DUPLICATION.0)
                })
                .collect();

            if files(occurrences.iter().map(|occurrence| occurrence.document)) < min_files {
                return None;
            }

            let first = occurrences[0];

            Some(CrossFileFinding {
                rule_id: CROSS_FILE_DUPLICATION.0.to_owned(),
                rule_name: CROSS_FILE_DUPLICATION.1.to_owned(),
                severity: Severity::Medium,
                matched: texts[first.document][first.span.start()..first.span.end()].to_owned(),
                occurrences,
            })
        })
        .collect();

    findings.sort_by_key(|finding| {
        (
            finding.occurrences[0].document,
            finding.occurrences[0].span.start(),
        )
    });

    findings
}

/// Every word of `text`, lowercased for comparison.
///
/// A link counts as one word rather than the seven its punctuation splits it
/// into. Two pages citing the same source are a convention, and a link read as
/// a run of words cleared the threshold on its own.
fn words(text: &str) -> Vec<Word> {
    let links = link_spans(text);
    let mut words: Vec<Word> = Vec::new();

    for word in word_spans(text) {
        let span = links
            .iter()
            .find(|link| link.start() <= word.start() && word.end() <= link.end())
            .copied()
            .unwrap_or(word);

        if words.last().is_some_and(|last| last.span == span) {
            continue;
        }

        words.push(Word {
            span,
            lowercase: text[span.start()..span.end()].to_lowercase(),
        });
    }

    words
}

/// The byte ranges of the links in `text`, each running to the whitespace
/// that ends it.
fn link_spans(text: &str) -> Vec<Span> {
    const SCHEMES: [&str; 2] = ["http://", "https://"];

    let mut spans = Vec::new();
    let mut offset = 0;

    while let Some(found) = text[offset..].find("http") {
        let start = offset + found;
        let link = &text[start..];

        match SCHEMES.iter().any(|scheme| link.starts_with(scheme)) {
            true => {
                let end = start + link.find(char::is_whitespace).unwrap_or(link.len());
                spans.push(Span(start, end));
                offset = end;
            }
            false => offset = start + "http".len(),
        }
    }

    spans
}

/// Groups the places every `min_words`-long run appears, keeping only the runs
/// that appear more than once.
///
/// Positions come out in the order the texts were given, which is what lets
/// [`files`] count the texts by walking the list once.
fn shared_starts(documents: &[Vec<Word>], min_words: usize) -> Vec<Vec<Position>> {
    let mut runs: HashMap<String, Vec<Position>> = HashMap::new();

    for (document, words) in documents.iter().enumerate() {
        for word in 0..words.len().saturating_sub(min_words - 1) {
            let run = words[word..word + min_words]
                .iter()
                .map(|word| word.lowercase.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            runs.entry(run)
                .or_default()
                .push(Position { document, word });
        }
    }

    runs.into_values()
        .filter(|positions| positions.len() > 1)
        .collect()
}

/// How many distinct texts a run of occurrences covers.
///
/// Both callers hand over document indexes in ascending order, so counting
/// the changes is the whole of it.
fn files(documents: impl Iterator<Item = usize>) -> usize {
    let mut previous = None;

    documents
        .filter(|document| previous.replace(*document) != Some(*document))
        .count()
}

/// Whether the same word precedes every occurrence of the run.
///
/// Such a run is the tail of a longer shared one, which is reported on its
/// own. Dropping it is what leaves one finding per repetition rather than one
/// per word of it.
fn share_a_preceding_word(documents: &[Vec<Word>], positions: &[Position]) -> bool {
    let preceding = |position: &Position| {
        position
            .word
            .checked_sub(1)
            .map(|word| documents[position.document][word].lowercase.as_str())
    };

    preceding(&positions[0]).is_some_and(|first| {
        positions
            .iter()
            .all(|position| preceding(position) == Some(first))
    })
}

/// How far the run reaches: the words every occurrence still agrees on.
fn shared_length(documents: &[Vec<Word>], positions: &[Position], min_words: usize) -> usize {
    let mut length = min_words;

    loop {
        let mut next = positions.iter().map(|position| {
            documents[position.document]
                .get(position.word + length)
                .map(|word| word.lowercase.as_str())
        });

        let Some(Some(first)) = next.next() else {
            return length;
        };

        if !next.all(|word| word == Some(first)) {
            return length;
        }

        length += 1;
    }
}

/// The byte range a run of `length` words covers from `position`.
fn occurrence(documents: &[Vec<Word>], position: Position, length: usize) -> Occurrence {
    let words = &documents[position.document];

    Occurrence {
        document: position.document,
        span: Span(
            words[position.word].span.start(),
            words[position.word + length - 1].span.end(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(texts: &[&str]) -> Vec<CrossFileFinding> {
        scan_cross_file(texts, CrossFileLimits::default())
    }

    fn matched(texts: &[&str]) -> Vec<String> {
        scan(texts)
            .into_iter()
            .map(|finding| finding.matched)
            .collect()
    }

    #[test]
    fn a_phrase_two_files_share_is_reported_once_with_both_places() {
        let first = "The loader reads it, rather than left to be discovered.";
        let second = "A default is written down, rather than left to be discovered.";
        let findings = scan(&[first, second]);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, CROSS_FILE_DUPLICATION.0);
        assert_eq!(findings[0].matched, "rather than left to be discovered");

        let places: Vec<_> = findings[0]
            .occurrences
            .iter()
            .map(|occurrence| occurrence.document)
            .collect();

        assert_eq!(places, vec![0, 1]);
    }

    #[test]
    fn a_finding_names_every_file_the_run_appears_in() {
        let text = "Each copy is clean alone, and the set is the duplication.";
        let findings = scan(&[text, text, text]);

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0]
                .occurrences
                .iter()
                .map(|occurrence| occurrence.document)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn a_shared_run_is_reported_at_its_full_length() {
        let first = "Nobody reads the queue before the loader merges the two files.";
        let second = "The loader merges the two files, whatever the queue says.";

        assert_eq!(
            matched(&[first, second]),
            vec!["the loader merges the two files"]
        );
    }

    #[test]
    fn shared_vocabulary_stays_below_the_threshold() {
        let first = "The board carries status:queued until a run takes it.";
        let second = "An issue reading status:queued is one nobody holds.";

        assert_eq!(matched(&[first, second]), Vec::<String>::new());
    }

    #[test]
    fn two_pages_citing_the_same_link_are_not_a_repetition() {
        let first = "The catalog is at https://tropes.fyi/directory today.";
        let second = "Read https://tropes.fyi/directory for the rest of them.";

        assert_eq!(matched(&[first, second]), Vec::<String>::new());
    }

    #[test]
    fn one_file_is_scanned_as_it_was_before_the_rule() {
        let text = "The same long sentence about the loader twice. The same long sentence about the loader twice.";

        assert_eq!(scan(&[text]), Vec::new());
    }

    #[test]
    fn the_thresholds_are_tunable() {
        let first = "The board carries status:queued until a run takes it.";
        let second = "An issue reading status:queued is one nobody holds.";
        let limits = CrossFileLimits {
            min_words: 2,
            min_files: 2,
        };

        let findings = scan_cross_file(&[first, second], limits);

        assert_eq!(
            findings
                .iter()
                .map(|finding| finding.matched.as_str())
                .collect::<Vec<_>>(),
            vec!["status:queued"]
        );
    }

    #[test]
    fn a_run_in_too_few_files_is_left_alone() {
        let first = "The loader reads the file, merges the two, and records the path.";
        let second = "The loader reads the file, merges the two, and records the path.";
        let third = "Nothing here resembles the other two documents at all.";
        let limits = CrossFileLimits {
            min_words: 6,
            min_files: 3,
        };

        assert_eq!(scan_cross_file(&[first, second, third], limits), Vec::new());
    }

    #[test]
    fn a_command_quoted_in_two_files_is_not_a_repetition() {
        let text = "Run it.\n\n```sh\ncargo run -q -p trps-cli -- README.md docs/guide.md\n```\n";

        assert_eq!(scan(&[text, text]), Vec::new());
    }

    #[test]
    fn a_suppressed_side_drops_out_of_the_finding() {
        let shared = "the loader merges the two files it was given";
        let first = format!("trps-ignore-next-line\nNobody reads the queue before {shared}.\n");
        let second = format!("Whatever the queue says, {shared}.\n");
        let third = format!("The run ends once {shared}.\n");

        let findings = scan(&[&first, &second, &third]);

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0]
                .occurrences
                .iter()
                .map(|occurrence| occurrence.document)
                .collect::<Vec<_>>(),
            vec![1, 2],
            "the suppressed file leaves the finding and the other two keep it"
        );
    }

    #[test]
    fn a_repetition_suppressed_everywhere_but_one_file_is_dropped() {
        let shared = "the loader merges the two files it was given";
        let first = format!("trps-ignore-next-line\nNobody reads the queue before {shared}.\n");
        let second = format!("Whatever the queue says, {shared}.\n");

        assert_eq!(scan(&[&first, &second]), Vec::new());
    }
}
