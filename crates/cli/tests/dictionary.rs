//! End-to-end checks that a project dictionary reaches the scan.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const INPUT: &str = "The capture harness will delve into the logs.\n";

#[test]
fn a_named_dictionary_drops_an_allowed_phrase() {
    let directory = case_dir("named-dictionary");
    let input = write(&directory, "input.md", INPUT);
    let empty = write(&directory, "empty.toml", "");
    let allowing = write(&directory, "allow.toml", "allow = [\"harness\"]\n");

    let bundled = scan(&directory, &["--dictionary", path(&empty), path(&input)]);
    assert!(bundled.contains("harness"), "{bundled}");

    let extended = scan(&directory, &["--dictionary", path(&allowing), path(&input)]);
    assert!(!extended.contains("harness"), "{extended}");
    assert!(extended.contains("word_choice.delve"), "{extended}");
}

#[test]
fn a_discovered_dictionary_applies_without_a_flag() {
    let directory = case_dir("discovered-dictionary");
    let input = write(&directory, "input.md", INPUT);
    write(&directory, "trps.toml", "allow = [\"harness\"]\n");

    let report = scan(&directory, &[path(&input)]);

    assert!(!report.contains("harness"), "{report}");
    assert!(report.contains("word_choice.delve"), "{report}");
}

#[test]
fn a_declared_pattern_wins_an_overlap_with_a_bundled_phrase() {
    let directory = case_dir("overlapping-dictionary");
    let input = write(
        &directory,
        "input.md",
        "The landscape architecture review is done.\n",
    );
    write(
        &directory,
        "trps.toml",
        r#"
[[patterns]]
id = "project.landscape_architecture"
name = "Landscape Architecture"
severity = "high"
phrases = ["landscape architecture"]
"#,
    );

    let report = scan(&directory, &[path(&input)]);

    assert!(
        report.contains("project.landscape_architecture"),
        "{report}"
    );
    assert!(!report.contains("word_choice.grandiose_nouns"), "{report}");
}

#[test]
fn a_dictionary_repeating_a_pattern_id_is_a_configuration_error() {
    let directory = case_dir("duplicate-id-dictionary");
    let input = write(&directory, "input.md", INPUT);
    write(
        &directory,
        "trps.toml",
        r#"
[[patterns]]
id = "project.dup"
name = "First"
severity = "low"
phrases = ["bounded"]

[[patterns]]
id = "project.dup"
name = "Second"
severity = "high"
phrases = ["contract"]
"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_tropius-cli"))
        .current_dir(&directory)
        .arg(path(&input))
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate pattern id `project.dup`"));
}

#[test]
fn a_missing_dictionary_is_a_configuration_error() {
    let directory = case_dir("missing-dictionary");
    let input = write(&directory, "input.md", INPUT);
    let missing = directory.join("absent.toml");

    let output = Command::new(env!("CARGO_BIN_EXE_tropius-cli"))
        .current_dir(&directory)
        .args(["--dictionary", path(&missing), path(&input)])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("absent.toml"));
}

fn scan(directory: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_tropius-cli"))
        .current_dir(directory)
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));

    String::from_utf8(output.stdout).unwrap()
}

fn case_dir(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);

    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }

    fs::create_dir_all(&directory).unwrap();

    directory
}

fn write(directory: &Path, name: &str, contents: &str) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, contents).unwrap();
    path
}

fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}
