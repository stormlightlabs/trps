//! Corpus tests over the checked-in examples in `meta/examples`.
//!
//! A file in `clean/` must produce no findings. A file in `slop/` must produce
//! exactly the rule ids listed for it below, so a detector that starts or stops
//! firing on real prose shows up as a failing test rather than as drift.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use tropius_core::detector::Detector;

/// Rule ids each slop example is expected to produce.
const SLOP_RULES: &[(&str, &[&str])] = &[
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
    (
        "formatting.md",
        &[
            "formatting.bold_first_bullets",
            "formatting.em_dash_addiction",
            "formatting.signposted_conclusion",
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
