//! Structural detectors for trope signals that are not literal phrase matches.
//!
//! Every rule here comes from the Tropes.fyi list in `meta/tropes.md`.
//! `meta/sources.md` carries the catalog and its license.

/// Three sentences opening with the same words.
pub const ANAPHORA_ABUSE: (&str, &str) = ("sentence_structure.anaphora_abuse", "Anaphora Abuse");

/// A sentence built as a three-part list.
pub const TRICOLON_ABUSE: (&str, &str) = ("sentence_structure.tricolon_abuse", "Tricolon Abuse");

/// A run of short declarative sentences.
pub const SHORT_PUNCHY_FRAGMENTS: (&str, &str) = (
    "paragraph_structure.short_punchy_fragments",
    "Short Punchy Fragments",
);

/// Prose carrying an ordinal list without being a list.
pub const LISTICLE_IN_TRENCH_COAT: (&str, &str) = (
    "paragraph_structure.listicle_in_trench_coat",
    "Listicle in a Trench Coat",
);

/// A summary of the summary of the summary.
pub const FRACTAL_SUMMARIES: (&str, &str) = ("composition.fractal_summaries", "Fractal Summaries");

/// Three analogies to history stacked in a row.
pub const HISTORICAL_ANALOGY_STACKING: (&str, &str) = (
    "composition.historical_analogy_stacking",
    "Historical Analogy Stacking",
);

use serde::Deserialize;

use super::{Finding, paragraph_spans, prose_sentences};

/// The counts the structural rules fire at, each under its own rule id.
///
/// The defaults were calibrated against `meta/examples`, which holds short
/// prose. Every rule here reports a shape a writer reaches for on purpose now
/// and then, so a project writing more of them raises the count rather than
/// silencing the rule.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StructuralLimits {
    /// What counts as a run of sentences opening alike, under
    /// `sentence_structure.anaphora_abuse`.
    pub anaphora_abuse: AnaphoraLimits,
    /// What counts as a sentence built as a list, under
    /// `sentence_structure.tricolon_abuse`.
    pub tricolon_abuse: TricolonLimits,
    /// What counts as a run of short sentences, under
    /// `paragraph_structure.short_punchy_fragments`.
    pub short_punchy_fragments: FragmentLimits,
    /// What counts as prose carrying a list, under
    /// `paragraph_structure.listicle_in_trench_coat`.
    pub listicle_in_trench_coat: ListicleLimits,
    /// What counts as a document summarizing itself, under
    /// `composition.fractal_summaries`.
    pub fractal_summaries: SummaryLimits,
    /// What counts as a stack of analogies, under
    /// `composition.historical_analogy_stacking`.
    pub historical_analogy_stacking: AnalogyLimits,
}

/// What counts as a run of sentences opening alike, under
/// `sentence_structure.anaphora_abuse`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnaphoraLimits {
    /// Sentences in a row opening with the same two words before the run is
    /// reported. A writer repeats an opening once for emphasis; three in a
    /// row is the trope. Below two a single sentence is a run of one, so two
    /// is the floor.
    pub min_sentences: usize,
}

impl Default for AnaphoraLimits {
    fn default() -> Self {
        Self { min_sentences: 3 }
    }
}

/// What counts as a sentence built as a list, under
/// `sentence_structure.tricolon_abuse`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TricolonLimits {
    /// Commas and semicolons a sentence holds before it reads as a list. A
    /// tricolon is three parts, and two separators are what divide a
    /// sentence into three, so two is the floor. Below it the rule reports
    /// `Products help people, products help teams.`, a two-part sentence
    /// that is no tricolon at all.
    pub min_separators: usize,
    /// Clauses in a row opening with the same word. One echo is the
    /// parallelism the rule reads three parts by; at zero the separators
    /// alone report every sentence holding a pair of commas, so one is the
    /// floor.
    pub min_repeated_starts: usize,
}

impl Default for TricolonLimits {
    fn default() -> Self {
        Self {
            min_separators: 2,
            min_repeated_starts: 2,
        }
    }
}

/// What counts as a run of short sentences, under
/// `paragraph_structure.short_punchy_fragments`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FragmentLimits {
    /// Fragments in a row before the run is reported. Below two a single
    /// short sentence is a run of one, so two is the floor.
    pub min_sentences: usize,
    /// Words a sentence can hold and still count as a fragment. At zero no
    /// sentence counts, which silences the rule rather than tightening it,
    /// so one is the floor.
    pub max_words: usize,
}

impl Default for FragmentLimits {
    fn default() -> Self {
        Self {
            min_sentences: 3,
            max_words: 4,
        }
    }
}

/// What counts as prose carrying a list, under
/// `paragraph_structure.listicle_in_trench_coat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ListicleLimits {
    /// Paragraphs in a row opening with an ordinal before the run is
    /// reported. Below two one ordinal opening is a run of one, so two is
    /// the floor.
    pub min_paragraphs: usize,
}

impl Default for ListicleLimits {
    fn default() -> Self {
        Self { min_paragraphs: 3 }
    }
}

/// What counts as a document summarizing itself, under
/// `composition.fractal_summaries`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SummaryLimits {
    /// Paragraphs opening as a summary before the document is reported.
    /// Below two the one summary a piece ends with is the trope, so two is
    /// the floor.
    pub min_openings: usize,
}

impl Default for SummaryLimits {
    fn default() -> Self {
        Self { min_openings: 3 }
    }
}

/// What counts as a stack of analogies, under
/// `composition.historical_analogy_stacking`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnalogyLimits {
    /// Sentences in a row reaching for a well-known company or platform
    /// before the stack is reported. Below two a single analogy is a stack
    /// of one, so two is the floor.
    pub min_sentences: usize,
}

impl Default for AnalogyLimits {
    fn default() -> Self {
        Self { min_sentences: 3 }
    }
}

/// Finds structural trope signals in text.
///
/// The sentence rules run over one block of prose at a time, because every one
/// of them is about sentences sitting next to each other in a paragraph. The
/// paragraph rules read the sections instead, and a list or a table is a
/// section of its own.
pub fn scan_structural(text: &str, limits: StructuralLimits) -> Vec<Finding> {
    let paragraphs = paragraph_spans(text);
    let mut findings = Vec::new();

    for sentences in prose_sentences(text) {
        findings.extend(scan_anaphora(text, &sentences, limits.anaphora_abuse));
        findings.extend(scan_tricolon(text, &sentences, limits.tricolon_abuse));
        findings.extend(scan_short_punchy_fragments(
            text,
            &sentences,
            limits.short_punchy_fragments,
        ));
        findings.extend(scan_historical_analogy_stacking(
            text,
            &sentences,
            limits.historical_analogy_stacking,
        ));
    }

    findings.extend(scan_listicle_in_trench_coat(
        text,
        &paragraphs,
        limits.listicle_in_trench_coat,
    ));
    findings.extend(scan_fractal_summaries(
        text,
        &paragraphs,
        limits.fractal_summaries,
    ));
    findings
}

fn scan_anaphora(text: &str, sentences: &[super::Span], limits: AnaphoraLimits) -> Vec<Finding> {
    let starts: Vec<_> = sentences
        .iter()
        .filter_map(|sentence| sentence_start_key(text, *sentence).map(|key| (*sentence, key)))
        .collect();

    starts
        .windows(limits.min_sentences.max(2))
        .filter(|window| window.windows(2).all(|pair| pair[0].1 == pair[1].1))
        .map(|window| {
            Finding::structural(
                ANAPHORA_ABUSE,
                text,
                super::Span(window[0].0.start(), window[window.len() - 1].0.end()),
            )
        })
        .collect()
}

fn scan_tricolon(text: &str, sentences: &[super::Span], limits: TricolonLimits) -> Vec<Finding> {
    let min_separators = limits.min_separators.max(2);
    let min_repeated_starts = limits.min_repeated_starts.max(1);

    sentences
        .iter()
        .filter(|sentence| {
            let value = &text[sentence.start()..sentence.end()];
            let separators = value.matches(',').count() + value.matches(';').count();
            separators >= min_separators && repeated_clause_starts(value) >= min_repeated_starts
        })
        .map(|sentence| {
            Finding::structural(
                TRICOLON_ABUSE,
                text,
                super::Span(sentence.start(), sentence.end()),
            )
        })
        .collect()
}

fn scan_short_punchy_fragments(
    text: &str,
    sentences: &[super::Span],
    limits: FragmentLimits,
) -> Vec<Finding> {
    let min_sentences = limits.min_sentences.max(2);
    let max_words = limits.max_words.max(1);
    let mut findings = Vec::new();
    let mut run_start = None;
    let mut run_end = 0;
    let mut run_len = 0;

    for sentence in sentences {
        if word_count(&text[sentence.start()..sentence.end()]) <= max_words {
            run_start.get_or_insert(sentence.start());
            run_end = sentence.end();
            run_len += 1;
        } else {
            if run_len >= min_sentences {
                findings.push(Finding::structural(
                    SHORT_PUNCHY_FRAGMENTS,
                    text,
                    super::Span(run_start.unwrap(), run_end),
                ));
            }
            run_start = None;
            run_end = 0;
            run_len = 0;
        }
    }

    if run_len >= min_sentences {
        findings.push(Finding::structural(
            SHORT_PUNCHY_FRAGMENTS,
            text,
            super::Span(run_start.unwrap(), run_end),
        ));
    }

    findings
}

fn scan_listicle_in_trench_coat(
    text: &str,
    paragraphs: &[super::Span],
    limits: ListicleLimits,
) -> Vec<Finding> {
    let mut ordinal_hits = Vec::new();

    for paragraph in paragraphs {
        if paragraph_starts_with_ordinal(&text[paragraph.start()..paragraph.end()]) {
            ordinal_hits.push(*paragraph);
        }
    }

    ordinal_hits
        .windows(limits.min_paragraphs.max(2))
        .map(|window| {
            Finding::structural(
                LISTICLE_IN_TRENCH_COAT,
                text,
                super::Span(window[0].start(), window[window.len() - 1].end()),
            )
        })
        .collect()
}

fn scan_fractal_summaries(
    text: &str,
    paragraphs: &[super::Span],
    limits: SummaryLimits,
) -> Vec<Finding> {
    let mut hits = Vec::new();

    for paragraph in paragraphs {
        let value = text[paragraph.start()..paragraph.end()].trim_start();
        if starts_with_any_ci(
            value,
            &[
                "in this section",
                "as we've seen",
                "as we have seen",
                "in summary",
                "to sum up",
                "in conclusion",
            ],
        ) {
            hits.push(*paragraph);
        }
    }

    if hits.len() < limits.min_openings.max(2) {
        return Vec::new();
    }

    vec![Finding::structural(
        FRACTAL_SUMMARIES,
        text,
        super::Span(hits[0].start(), hits[hits.len() - 1].end()),
    )]
}

fn scan_historical_analogy_stacking(
    text: &str,
    sentences: &[super::Span],
    limits: AnalogyLimits,
) -> Vec<Finding> {
    sentences
        .windows(limits.min_sentences.max(2))
        .filter(|window| {
            window
                .iter()
                .all(|sentence| has_analogy_marker(&text[sentence.start()..sentence.end()]))
        })
        .map(|window| {
            Finding::structural(
                HISTORICAL_ANALOGY_STACKING,
                text,
                super::Span(window[0].start(), window[window.len() - 1].end()),
            )
        })
        .collect()
}

fn sentence_start_key(text: &str, sentence: super::Span) -> Option<String> {
    let words = words(&text[sentence.start()..sentence.end()]);

    if words.is_empty() {
        None
    } else {
        Some(words.into_iter().take(2).collect::<Vec<_>>().join(" "))
    }
}

fn repeated_clause_starts(sentence: &str) -> usize {
    let starts: Vec<_> = sentence
        .split([',', ';'])
        .filter_map(|clause| words(clause).into_iter().next())
        .collect();

    starts
        .windows(2)
        .filter(|window| window[0] == window[1])
        .count()
}

fn paragraph_starts_with_ordinal(paragraph: &str) -> bool {
    starts_with_any_ci(
        paragraph.trim_start(),
        &[
            "the first",
            "first,",
            "first ",
            "the second",
            "second,",
            "second ",
            "the third",
            "third,",
        ],
    )
}

fn has_analogy_marker(sentence: &str) -> bool {
    let lower = sentence.to_ascii_lowercase();
    let examples = [
        "apple", "facebook", "stripe", "aws", "spotify", "uber", "airbnb", "shopify", "discord",
        "web", "mobile", "social", "cloud",
    ];

    examples.iter().any(|example| lower.contains(example))
}

fn starts_with_any_ci(value: &str, prefixes: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    prefixes.iter().any(|prefix| lower.starts_with(prefix))
}

fn word_count(value: &str) -> usize {
    words(value).len()
}

fn words(value: &str) -> Vec<String> {
    value
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scans for structural signals at the counts the tool ships with.
    fn scan_defaults(text: &str) -> Vec<Finding> {
        scan_structural(text, StructuralLimits::default())
    }

    /// Whether a scan reported `rule`, so a tuned count is read apart from
    /// whatever else the fixture happens to trip.
    fn fires(findings: &[Finding], rule: (&str, &str)) -> bool {
        findings.iter().any(|finding| finding.rule_id == rule.0)
    }

    #[test]
    fn detects_anaphora_abuse() {
        let findings = scan_defaults(
            "They assume users pay. They assume builders arrive. They assume markets form.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "sentence_structure.anaphora_abuse")
        );
    }

    #[test]
    fn detects_tricolon_abuse() {
        let findings = scan_defaults(
            "Products impress people, products empower teams, products create worlds.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "sentence_structure.tricolon_abuse")
        );
    }

    #[test]
    fn detects_short_punchy_fragments() {
        let findings = scan_defaults("He published this. Openly. In a book. As a priest.");

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "paragraph_structure.short_punchy_fragments")
        );
    }

    #[test]
    fn a_run_of_fragments_does_not_reach_across_a_list() {
        let findings = scan_defaults("Ask of each change:\n\n- Is it needed?\n- Is it small?\n");

        assert!(findings.is_empty());
    }

    #[test]
    fn anaphora_does_not_reach_across_a_paragraph_break() {
        let findings = scan_defaults(
            "They assume users pay.\n\nThey assume builders arrive.\n\nThey assume markets form.",
        );

        assert!(findings.is_empty());
    }

    #[test]
    fn a_path_is_one_word_rather_than_a_run_of_fragments() {
        let findings = scan_defaults(
            "The loader reads `~/.config/trps/trps.toml` first, then `.trps.toml` beside it.",
        );

        assert!(findings.is_empty());
    }

    #[test]
    fn detects_listicle_in_trench_coat() {
        let findings = scan_defaults(
            "The first wall is access.\n\nThe second wall is pricing.\n\nThe third wall is trust.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "paragraph_structure.listicle_in_trench_coat")
        );
    }

    #[test]
    fn detects_fractal_summaries() {
        let findings = scan_defaults(
            "In this section, we examine access.\n\nAs we've seen, access matters.\n\nIn summary, access wins.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "composition.fractal_summaries")
        );
    }

    #[test]
    fn detects_historical_analogy_stacking() {
        let findings = scan_defaults(
            "Apple did not build Uber. Facebook did not build Spotify. AWS did not build Airbnb.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "composition.historical_analogy_stacking")
        );
    }

    #[test]
    fn a_tuned_anaphora_count_moves_where_a_run_reports() {
        let two = "They assume users pay. They assume builders arrive.";
        let three = "They assume users pay. They assume builders arrive. They assume markets form.";
        let lowered = StructuralLimits {
            anaphora_abuse: AnaphoraLimits { min_sentences: 2 },
            ..StructuralLimits::default()
        };
        let raised = StructuralLimits {
            anaphora_abuse: AnaphoraLimits { min_sentences: 4 },
            ..StructuralLimits::default()
        };

        assert!(!fires(&scan_defaults(two), ANAPHORA_ABUSE));
        assert!(fires(&scan_structural(two, lowered), ANAPHORA_ABUSE));
        assert!(fires(&scan_defaults(three), ANAPHORA_ABUSE));
        assert!(!fires(&scan_structural(three, raised), ANAPHORA_ABUSE));
    }

    #[test]
    fn a_tuned_tricolon_count_moves_where_a_sentence_reports() {
        let one_echo =
            "Products impress people, products empower teams, and markets create worlds.";
        let three_part = "Products impress people, products empower teams, products create worlds.";
        let lowered = StructuralLimits {
            tricolon_abuse: TricolonLimits {
                min_repeated_starts: 1,
                ..TricolonLimits::default()
            },
            ..StructuralLimits::default()
        };
        let more_separators = StructuralLimits {
            tricolon_abuse: TricolonLimits {
                min_separators: 3,
                ..TricolonLimits::default()
            },
            ..StructuralLimits::default()
        };
        let more_repeats = StructuralLimits {
            tricolon_abuse: TricolonLimits {
                min_repeated_starts: 3,
                ..TricolonLimits::default()
            },
            ..StructuralLimits::default()
        };

        assert!(!fires(&scan_defaults(one_echo), TRICOLON_ABUSE));
        assert!(fires(&scan_structural(one_echo, lowered), TRICOLON_ABUSE));
        assert!(fires(&scan_defaults(three_part), TRICOLON_ABUSE));
        assert!(!fires(
            &scan_structural(three_part, more_separators),
            TRICOLON_ABUSE
        ));
        assert!(!fires(
            &scan_structural(three_part, more_repeats),
            TRICOLON_ABUSE
        ));
    }

    #[test]
    fn a_tuned_fragment_run_moves_where_a_run_reports() {
        let two = "He published this. Openly. The rest of the work took another decade.";
        let four = "He published this. Openly. In a book. As a priest.";
        let lowered = StructuralLimits {
            short_punchy_fragments: FragmentLimits {
                min_sentences: 2,
                ..FragmentLimits::default()
            },
            ..StructuralLimits::default()
        };
        let raised = StructuralLimits {
            short_punchy_fragments: FragmentLimits {
                min_sentences: 5,
                ..FragmentLimits::default()
            },
            ..StructuralLimits::default()
        };

        assert!(!fires(&scan_defaults(two), SHORT_PUNCHY_FRAGMENTS));
        assert!(fires(
            &scan_structural(two, lowered),
            SHORT_PUNCHY_FRAGMENTS
        ));
        assert!(fires(&scan_defaults(four), SHORT_PUNCHY_FRAGMENTS));
        assert!(!fires(
            &scan_structural(four, raised),
            SHORT_PUNCHY_FRAGMENTS
        ));
    }

    #[test]
    fn a_tuned_fragment_length_moves_which_sentences_count() {
        let four_words = "He published this openly. In a small book. As a young priest.";
        let five_words = "The team shipped the change. The users noticed it quickly. The numbers moved up sharply.";
        let lowered = StructuralLimits {
            short_punchy_fragments: FragmentLimits {
                max_words: 3,
                ..FragmentLimits::default()
            },
            ..StructuralLimits::default()
        };
        let raised = StructuralLimits {
            short_punchy_fragments: FragmentLimits {
                max_words: 5,
                ..FragmentLimits::default()
            },
            ..StructuralLimits::default()
        };

        assert!(fires(&scan_defaults(four_words), SHORT_PUNCHY_FRAGMENTS));
        assert!(!fires(
            &scan_structural(four_words, lowered),
            SHORT_PUNCHY_FRAGMENTS
        ));
        assert!(!fires(&scan_defaults(five_words), SHORT_PUNCHY_FRAGMENTS));
        assert!(fires(
            &scan_structural(five_words, raised),
            SHORT_PUNCHY_FRAGMENTS
        ));
    }

    #[test]
    fn a_tuned_listicle_count_moves_where_paragraphs_report() {
        let two = "The first wall is access.\n\nThe second wall is pricing.";
        let three =
            "The first wall is access.\n\nThe second wall is pricing.\n\nThe third wall is trust.";
        let lowered = StructuralLimits {
            listicle_in_trench_coat: ListicleLimits { min_paragraphs: 2 },
            ..StructuralLimits::default()
        };
        let raised = StructuralLimits {
            listicle_in_trench_coat: ListicleLimits { min_paragraphs: 4 },
            ..StructuralLimits::default()
        };

        assert!(!fires(&scan_defaults(two), LISTICLE_IN_TRENCH_COAT));
        assert!(fires(
            &scan_structural(two, lowered),
            LISTICLE_IN_TRENCH_COAT
        ));
        assert!(fires(&scan_defaults(three), LISTICLE_IN_TRENCH_COAT));
        assert!(!fires(
            &scan_structural(three, raised),
            LISTICLE_IN_TRENCH_COAT
        ));
    }

    #[test]
    fn a_tuned_summary_count_moves_where_a_document_reports() {
        let two = "In this section, we examine access.\n\nIn summary, access wins.";
        let three = "In this section, we examine access.\n\nAs we've seen, access matters.\n\nIn summary, access wins.";
        let lowered = StructuralLimits {
            fractal_summaries: SummaryLimits { min_openings: 2 },
            ..StructuralLimits::default()
        };
        let raised = StructuralLimits {
            fractal_summaries: SummaryLimits { min_openings: 4 },
            ..StructuralLimits::default()
        };

        assert!(!fires(&scan_defaults(two), FRACTAL_SUMMARIES));
        assert!(fires(&scan_structural(two, lowered), FRACTAL_SUMMARIES));
        assert!(fires(&scan_defaults(three), FRACTAL_SUMMARIES));
        assert!(!fires(&scan_structural(three, raised), FRACTAL_SUMMARIES));
    }

    #[test]
    fn a_tuned_analogy_count_moves_where_a_stack_reports() {
        let two = "Apple did not build Uber. Facebook did not build Spotify.";
        let three =
            "Apple did not build Uber. Facebook did not build Spotify. AWS did not build Airbnb.";
        let lowered = StructuralLimits {
            historical_analogy_stacking: AnalogyLimits { min_sentences: 2 },
            ..StructuralLimits::default()
        };
        let raised = StructuralLimits {
            historical_analogy_stacking: AnalogyLimits { min_sentences: 4 },
            ..StructuralLimits::default()
        };

        assert!(!fires(&scan_defaults(two), HISTORICAL_ANALOGY_STACKING));
        assert!(fires(
            &scan_structural(two, lowered),
            HISTORICAL_ANALOGY_STACKING
        ));
        assert!(fires(&scan_defaults(three), HISTORICAL_ANALOGY_STACKING));
        assert!(!fires(
            &scan_structural(three, raised),
            HISTORICAL_ANALOGY_STACKING
        ));
    }

    #[test]
    fn a_separator_count_below_two_still_needs_three_parts_for_a_tricolon() {
        let two_part = "Products help people, products help teams.";
        let floor = StructuralLimits {
            tricolon_abuse: TricolonLimits {
                min_separators: 1,
                min_repeated_starts: 1,
            },
            ..StructuralLimits::default()
        };

        assert!(!fires(&scan_structural(two_part, floor), TRICOLON_ABUSE));
    }

    #[test]
    fn a_repeated_start_count_of_zero_is_clamped_rather_than_reporting_any_pair_of_commas() {
        let listed = "Rain arrived at noon, wind followed by three, and the harbor went quiet.";
        let floor = StructuralLimits {
            tricolon_abuse: TricolonLimits {
                min_repeated_starts: 0,
                ..TricolonLimits::default()
            },
            ..StructuralLimits::default()
        };

        assert!(!fires(&scan_structural(listed, floor), TRICOLON_ABUSE));
    }

    #[test]
    fn a_fragment_length_of_zero_is_clamped_rather_than_silencing_the_rule() {
        let fragments = "Openly. Boldly. Truly.";
        let floor = StructuralLimits {
            short_punchy_fragments: FragmentLimits {
                max_words: 0,
                ..FragmentLimits::default()
            },
            ..StructuralLimits::default()
        };

        assert!(fires(
            &scan_structural(fragments, floor),
            SHORT_PUNCHY_FRAGMENTS
        ));
    }

    #[test]
    fn counts_set_to_zero_are_clamped_rather_than_reporting_plain_prose() {
        let limits = StructuralLimits {
            anaphora_abuse: AnaphoraLimits { min_sentences: 0 },
            tricolon_abuse: TricolonLimits {
                min_separators: 0,
                min_repeated_starts: 0,
            },
            short_punchy_fragments: FragmentLimits {
                min_sentences: 0,
                max_words: 0,
            },
            listicle_in_trench_coat: ListicleLimits { min_paragraphs: 0 },
            fractal_summaries: SummaryLimits { min_openings: 0 },
            historical_analogy_stacking: AnalogyLimits { min_sentences: 0 },
        };
        let text = "The wind rose over the harbor.\n\nA gull turned above the water.";

        assert!(scan_structural(text, limits).is_empty());
    }
}
