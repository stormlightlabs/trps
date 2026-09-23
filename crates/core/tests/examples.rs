//! Corpus tests over the checked-in examples in `meta/examples`.
//!
//! A file in `clean/` must produce no findings. A file in `slop/` must produce
//! exactly the rule ids listed for it below, so a detector that starts or stops
//! firing on real prose shows up as a failing test rather than as drift.
//!
//! A file in `false-positives/` is prose a careful writer would defend, which
//! a rule reports anyway. The phrase matcher reads literal phrases and cannot
//! see the reason a sentence goes on to give, so those findings are the price
//! of the rule. Recording them here prices each rule and makes a change to one
//! visible: narrowing a rule empties its entry, and widening one fills it.
//!
//! `rules.rs` holds one sample per pattern and fails when a pattern arrives
//! with no test at all. These fixtures are prose where several rules meet, and
//! fail when one of them changes what it reports beside the others.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use trps_core::Detector;

/// Rule ids each slop example is expected to produce.
const SLOP_RULES: &[(&str, &[&str])] = &[
    (
        "assistant-voice.txt",
        &[
            "assistant.chat_residue",
            "assistant.cutoff_disclaimer",
            "assistant.process_narration",
            "assistant.self_reference",
            "assistant.tool_residue",
            "assistant.unfilled_placeholder",
        ],
    ),
    ("bold-leads.md", &["formatting.bold_first_leads"]),
    (
        "composition.txt",
        &[
            "composition.dead_metaphor",
            "composition.despite_challenges",
            "paragraph_structure.listicle_in_trench_coat",
            "word_choice.grandiose_nouns",
        ],
    ),
    ("decoration.md", &["formatting.unicode_decoration"]),
    ("em-dashes.txt", &["formatting.em_dash_addiction"]),
    (
        "formatting.md",
        &[
            "formatting.bold_first_leads",
            "formatting.signposted_conclusion",
        ],
    ),
    (
        "hedging.txt",
        &[
            "composition.restatement_markers",
            "composition.throat_clearing",
            "sentence_structure.hedge_stack",
            "sentence_structure.impersonal_hedge",
        ],
    ),
    (
        "narrative.txt",
        &["narrative.body_beats", "narrative.stock_imagery"],
    ),
    (
        "promotion.txt",
        &[
            "composition.era_framing",
            "composition.vague_connection",
            "tone.engagement_bait",
            "tone.promotional",
            "word_choice.lexical_spikes",
        ],
    ),
    (
        "repetition.txt",
        &[
            "composition.content_duplication",
            "composition.fractal_summaries",
            "composition.one_point_dilution",
            "formatting.signposted_conclusion",
        ],
    ),
    (
        "sentence-structure.txt",
        &[
            "paragraph_structure.short_punchy_fragments",
            "sentence_structure.false_ranges",
            "sentence_structure.filler_transitions",
            "sentence_structure.negative_parallelism",
            "sentence_structure.not_x_not_y",
            "sentence_structure.question_answer",
            "sentence_structure.superficial_analyses",
        ],
    ),
    (
        "structure.txt",
        &[
            "composition.historical_analogy_stacking",
            "paragraph_structure.short_punchy_fragments",
            "sentence_structure.anaphora_abuse",
            "sentence_structure.tricolon_abuse",
        ],
    ),
    (
        "technical-prose.md",
        &[
            "technical.anthropomorphism",
            "technical.restates_code",
            "technical.self_praise",
            "technical.vague_reasons",
        ],
    ),
    (
        "tone.txt",
        &[
            "tone.false_suspense",
            "tone.false_vulnerability",
            "tone.grandiose_stakes",
            "tone.imagine_world",
            "tone.invented_concept_labels",
            "tone.teacher_voice",
            "tone.think_of_it_as",
            "tone.truth_is_simple",
            "tone.vague_attributions",
        ],
    ),
    (
        "word-choice.txt",
        &[
            "word_choice.delve",
            "word_choice.grandiose_nouns",
            "word_choice.magic_adverbs",
            "word_choice.serves_as",
        ],
    ),
];

/// Rule ids each false-positive example is expected to produce.
///
/// Every entry is a finding on prose that says what it means. Treat a change
/// here as a decision about the rule named, not as a test to correct.
const FALSE_POSITIVE_RULES: &[(&str, &[&str])] = &[
    (
        "api-reference.md",
        &[
            "composition.throat_clearing",
            "technical.anthropomorphism",
            "technical.restates_code",
            "technical.self_praise",
        ],
    ),
    (
        "field-guide.txt",
        &["tone.promotional", "word_choice.lexical_spikes"],
    ),
    (
        "postmortem.txt",
        &[
            "composition.restatement_markers",
            "sentence_structure.impersonal_hedge",
            "technical.vague_reasons",
        ],
    ),
    (
        "short-story.txt",
        &["narrative.body_beats", "narrative.stock_imagery"],
    ),
];

#[test]
fn clean_examples_produce_no_findings() {
    for path in examples("clean") {
        assert_eq!(
            rule_ids(&path),
            BTreeSet::new(),
            "{} is meant to be clean",
            path.display()
        );
    }
}

#[test]
fn slop_examples_produce_the_listed_rules() {
    for path in examples("slop") {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap();
        let expected = SLOP_RULES
            .iter()
            .find(|(fixture, _)| *fixture == name)
            .unwrap_or_else(|| panic!("{name} has no expected rules in SLOP_RULES"));

        let expected: BTreeSet<String> = expected.1.iter().map(|id| (*id).to_owned()).collect();

        assert_eq!(rule_ids(&path), expected, "unexpected rules for {name}");
    }
}

#[test]
fn false_positive_examples_report_the_listed_rules() {
    for path in examples("false-positives") {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap();
        let expected = FALSE_POSITIVE_RULES
            .iter()
            .find(|(fixture, _)| *fixture == name)
            .unwrap_or_else(|| panic!("{name} has no expected rules in FALSE_POSITIVE_RULES"));

        let expected: BTreeSet<String> = expected.1.iter().map(|id| (*id).to_owned()).collect();

        assert_eq!(rule_ids(&path), expected, "unexpected rules for {name}");
    }
}

fn examples(kind: &str) -> Vec<PathBuf> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../meta/examples")
        .join(kind);

    let mut paths: Vec<_> = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
        .map(|entry| entry.expect("failed to read directory entry").path())
        .filter(|path| path.is_file())
        .collect();

    paths.sort();
    assert!(
        !paths.is_empty(),
        "{} holds no examples",
        directory.display()
    );
    paths
}

fn rule_ids(path: &Path) -> BTreeSet<String> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    Detector::bundled()
        .expect("bundled patterns build")
        .scan(&text)
        .into_iter()
        .map(|finding| finding.rule_id)
        .collect()
}
