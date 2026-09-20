//! Filesystem tests for project dictionary discovery and rule resolution.
//!
//! These build directory trees, so they run under the test target directory
//! rather than a shared temporary one.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use trps_core::patterns::{PROJECT_DICTIONARY_FILES, find_project_dictionary, load_pattern_file};
use trps_core::rules::Rules;

/// Prose carrying a bundled phrase, which a dictionary can allow away.
const DELVE: &str = "Let us delve into this.\n";

#[test]
fn a_dictionary_is_found_in_an_ancestor_directory() {
    for name in PROJECT_DICTIONARY_FILES {
        let root = repository(&format!("find-{name}"));
        let nested = root.join("docs/guides");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join(name), "allow = []\n").unwrap();

        assert_eq!(find_project_dictionary(&nested), Some(root.join(name)));
    }
}

#[test]
fn the_first_dictionary_name_wins_within_a_directory() {
    let root = repository("dictionary-name-order");

    for name in PROJECT_DICTIONARY_FILES.iter().rev() {
        fs::write(root.join(name), "allow = []\n").unwrap();
    }

    assert_eq!(
        find_project_dictionary(&root),
        Some(root.join(PROJECT_DICTIONARY_FILES[0]))
    );
}

#[test]
fn the_search_stops_at_the_repository_root() {
    let outside = case_dir("dictionary-above-repository");
    let root = outside.join("checkout");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(".git"), "gitdir: elsewhere\n").unwrap();
    fs::write(outside.join(PROJECT_DICTIONARY_FILES[0]), "allow = []\n").unwrap();

    assert_eq!(find_project_dictionary(&root), None);
}

#[test]
fn outside_a_repository_only_the_starting_directory_is_searched() {
    let root = outside_any_repository("dictionary-without-repository");
    let nested = root.join("docs");
    fs::create_dir_all(&nested).unwrap();
    fs::write(root.join(PROJECT_DICTIONARY_FILES[0]), "allow = []\n").unwrap();

    assert_eq!(find_project_dictionary(&nested), None);
    assert_eq!(
        find_project_dictionary(&root),
        Some(root.join(PROJECT_DICTIONARY_FILES[0]))
    );

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn no_dictionary_is_found_without_one() {
    let root = repository("no-dictionary");

    assert_eq!(find_project_dictionary(&root), None);
}

#[test]
fn reading_a_missing_dictionary_reports_its_path() {
    let path = case_dir("missing-dictionary").join("trps.toml");
    let error = load_pattern_file(&path).unwrap_err();

    assert!(error.to_string().contains("trps.toml"));
}

#[test]
fn a_named_dictionary_is_resolved_over_one_the_search_would_find() {
    let root = repository("resolve-named-dictionary");
    let named = root.join("named.toml");
    fs::write(&named, "allow = [\"delve into\"]\n").unwrap();
    fs::write(root.join(PROJECT_DICTIONARY_FILES[0]), "allow = []\n").unwrap();

    let rules = Rules::resolve(Some(&named), &root).unwrap();

    assert_eq!(rules.dictionary, Some(named));
    assert!(!reports_delve(&rules));
}

#[test]
fn a_dictionary_the_search_finds_is_resolved_without_being_named() {
    let root = repository("resolve-discovered-dictionary");
    let nested = root.join("docs");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        root.join(PROJECT_DICTIONARY_FILES[0]),
        "allow = [\"delve into\"]\n",
    )
    .unwrap();

    let rules = Rules::resolve(None, &nested).unwrap();

    assert_eq!(
        rules.dictionary,
        Some(root.join(PROJECT_DICTIONARY_FILES[0]))
    );
    assert!(!reports_delve(&rules));
}

#[test]
fn without_a_dictionary_the_bundled_patterns_stand_alone() {
    let root = repository("resolve-no-dictionary");

    let rules = Rules::resolve(None, &root).unwrap();

    assert_eq!(rules.dictionary, None);
    assert!(reports_delve(&rules));
    assert!(!rules.excludes.excludes(&root.join("notebook/page.md")));
}

#[test]
fn excludes_are_resolved_against_the_dictionary_directory() {
    let project = repository("resolve-exclude-root");
    let dictionary = project.join(PROJECT_DICTIONARY_FILES[0]);
    fs::write(&dictionary, "exclude = [\"notebook/\"]\n").unwrap();
    let elsewhere = repository("resolve-exclude-caller");

    let rules = Rules::resolve(Some(&dictionary), &elsewhere).unwrap();

    assert!(rules.excludes.excludes(&project.join("notebook/page.md")));
    assert!(!rules.excludes.excludes(&elsewhere.join("notebook/page.md")));
}

#[test]
fn an_exclude_that_is_not_a_glob_fails_the_resolution() {
    let root = repository("resolve-bad-exclude");
    fs::write(
        root.join(PROJECT_DICTIONARY_FILES[0]),
        "exclude = [\"notebook/[\"]\n",
    )
    .unwrap();

    let error = Rules::resolve(None, &root).unwrap_err();

    assert!(error.to_string().contains("notebook/["), "{error}");
}

/// Whether the resolved detector still reports the bundled phrase in [`DELVE`].
fn reports_delve(rules: &Rules) -> bool {
    rules
        .detector
        .scan(DELVE)
        .iter()
        .any(|finding| finding.rule_id == "word_choice.delve")
}

/// An empty directory under the test target directory, one per case.
fn case_dir(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);

    if directory.exists() {
        fs::remove_dir_all(&directory).expect("failed to clear the case directory");
    }

    fs::create_dir_all(&directory).expect("failed to create the case directory");

    directory
}

/// A case directory with no repository above it, which the test target
/// directory cannot provide: it sits inside this checkout. The name is unique
/// per run and the directory is created rather than reused, so a directory
/// planted in a shared temporary directory fails the test instead of feeding
/// it.
fn outside_any_repository(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is after the epoch")
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("tropius-{name}-{}-{stamp}", std::process::id()));

    fs::create_dir(&directory).expect("failed to create the case directory");

    directory
}

/// A case directory carrying the `.git` marker the search stops at, written as
/// a file the way a worktree checkout carries it.
fn repository(name: &str) -> PathBuf {
    let root = case_dir(name);
    fs::write(root.join(".git"), "gitdir: elsewhere\n").unwrap();
    root
}
