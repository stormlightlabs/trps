//! One sample of prose per bundled pattern.
//!
//! A pattern is worth no more than the text it fires on, so every bundled
//! pattern names a sample here and that sample has to produce its rule id. A
//! pattern added without one fails `every_pattern_has_a_sample`.

use std::collections::BTreeSet;

use tropius_core::detector::Detector;
use tropius_core::patterns::{SOURCES, bundled_patterns};

/// Prose each bundled pattern has to report.
///
/// A sample fires other rules too, which is what real prose does; only the
/// rule the sample is listed under is asserted.
const SAMPLES: &[(&str, &str)] = &[
    (
        "formatting.em_dash_addiction",
        "The change -- long overdue -- shipped on Friday.",
    ),
    (
        "formatting.signposted_conclusion",
        "In conclusion, the release shipped a week late.",
    ),
    (
        "sentence_structure.negative_parallelism",
        "It is not a rewrite of the parser.",
    ),
    (
        "sentence_structure.question_answer",
        "The result? Two days lost to a cold cache.",
    ),
    (
        "sentence_structure.filler_transitions",
        "It's worth noting the queue drains only on restart.",
    ),
    (
        "sentence_structure.not_x_not_y",
        "Not a bug. Not a feature. A default nobody read.",
    ),
    (
        "sentence_structure.superficial_analyses",
        "The port landed in March, highlighting its importance to the team.",
    ),
    (
        "sentence_structure.false_ranges",
        "The work ran from innovation to a shipped binary.",
    ),
    (
        "sentence_structure.impersonal_hedge",
        "It could be argued that the format was wrong from the start.",
    ),
    (
        "sentence_structure.hedge_stack",
        "Raising the timeout could potentially drain the queue faster.",
    ),
    (
        "composition.despite_challenges",
        "Despite these challenges, the port shipped.",
    ),
    (
        "composition.throat_clearing",
        "Note that the queue drains only on restart.",
    ),
    (
        "composition.restatement_markers",
        "In other words, the retry never ran.",
    ),
    (
        "composition.era_framing",
        "In today's fast-paced release cycle, the tests lag the code.",
    ),
    (
        "composition.vague_connection",
        "He is associated with the parser rewrite.",
    ),
    (
        "tone.false_suspense",
        "Here's the kicker: the cache was never warm.",
    ),
    ("tone.teacher_voice", "Let's break this down."),
    (
        "tone.think_of_it_as",
        "Think of it as a queue with one reader.",
    ),
    (
        "tone.imagine_world",
        "Imagine a world where a deploy never fails.",
    ),
    (
        "tone.false_vulnerability",
        "And yes, the first attempt failed in staging.",
    ),
    (
        "tone.truth_is_simple",
        "The reality is simpler than the postmortem says.",
    ),
    (
        "tone.grandiose_stakes",
        "The release will fundamentally reshape how the team ships.",
    ),
    (
        "tone.vague_attributions",
        "Experts argue the format will win by 2030.",
    ),
    (
        "tone.invented_concept_labels",
        "Call the whole thing the supervision paradox.",
    ),
    (
        "tone.promotional",
        "The office is nestled in the old mill district.",
    ),
    (
        "tone.engagement_bait",
        "What nobody tells you is that the cache starts cold.",
    ),
    (
        "word_choice.magic_adverbs",
        "The job quietly retried twice.",
    ),
    (
        "word_choice.delve",
        "Let us delve into the queue metrics for March.",
    ),
    (
        "word_choice.grandiose_nouns",
        "A tapestry of services sits behind one load balancer.",
    ),
    (
        "word_choice.serves_as",
        "The table serves as the index for every lookup.",
    ),
    (
        "word_choice.lexical_spikes",
        "A meticulous audit of an intricate codebase.",
    ),
    (
        "assistant.chat_residue",
        "Great question. The queue drains on restart.",
    ),
    (
        "assistant.process_narration",
        "Let me check the queue metrics for March.",
    ),
    (
        "assistant.self_reference",
        "I can see that the retry never ran.",
    ),
    (
        "assistant.cutoff_disclaimer",
        "As of my last update, the wire format was stable.",
    ),
    (
        "assistant.tool_residue",
        "The format is documented at https://example.com/spec?utm_source=chatgpt.com",
    ),
    ("assistant.unfilled_placeholder", "Signed, [Your Name]."),
    (
        "technical.self_praise",
        "The wrapper exists for better readability.",
    ),
    (
        "technical.anthropomorphism",
        "The parser is aware of the trailing newline.",
    ),
    (
        "technical.restates_code",
        "This function returns the parsed span.",
    ),
    (
        "technical.vague_reasons",
        "The retry runs twice for various reasons.",
    ),
    (
        "narrative.body_beats",
        "She took a deep breath and opened the door.",
    ),
    (
        "narrative.stock_imagery",
        "The air was thick with smoke from the mill.",
    ),
];

#[test]
fn every_sample_reports_the_rule_it_is_listed_under() {
    let detector = Detector::bundled().expect("bundled patterns build");

    for (rule_id, sample) in SAMPLES {
        let findings = detector.scan(sample);
        let reported: BTreeSet<&str> = findings
            .iter()
            .map(|finding| finding.rule_id.as_str())
            .collect();

        assert!(
            reported.contains(rule_id),
            "{rule_id} did not fire on {sample:?}; the sample reported {reported:?}"
        );
    }
}

#[test]
fn every_pattern_has_a_sample() {
    let sampled: BTreeSet<&str> = SAMPLES.iter().map(|(rule_id, _)| *rule_id).collect();
    let bundled: BTreeSet<String> = bundled_patterns()
        .expect("bundled patterns load")
        .into_iter()
        .map(|pattern| pattern.id)
        .collect();

    let uncovered: Vec<&String> = bundled
        .iter()
        .filter(|id| !sampled.contains(id.as_str()))
        .collect();
    let unknown: Vec<&&str> = sampled
        .iter()
        .filter(|id| !bundled.contains(**id))
        .collect();

    assert!(
        uncovered.is_empty(),
        "patterns with no sample: {uncovered:?}"
    );
    assert!(unknown.is_empty(), "samples for no pattern: {unknown:?}");
}

#[test]
fn every_registered_source_is_cited_by_a_pattern() {
    let cited: BTreeSet<String> = bundled_patterns()
        .expect("bundled patterns load")
        .into_iter()
        .flat_map(|pattern| pattern.sources)
        .collect();

    let uncited: Vec<&str> = SOURCES
        .iter()
        .map(|source| source.key)
        .filter(|key| !cited.contains(*key))
        .collect();

    assert!(uncited.is_empty(), "sources nothing cites: {uncited:?}");
}
