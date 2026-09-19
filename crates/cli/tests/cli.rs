//! Integration tests for the `tropius` command line.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

#[test]
fn a_file_argument_is_scanned_and_exits_one() {
    let output = scan(Some(&example("slop/word-choice.txt")), "", &[]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("word_choice.delve"));
}

#[test]
fn a_clean_file_prints_nothing_and_exits_zero() {
    let output = scan(Some(&example("clean/field-notes.txt")), "", &[]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "");
}

#[test]
fn stdin_is_scanned_when_no_file_is_given() {
    let findings = scan(None, "Let us delve into this robust ecosystem.\n", &[]);

    assert_eq!(findings.status.code(), Some(1));
    assert!(stdout(&findings).contains("word_choice.delve"));

    let clean = scan(None, "The parser reads a file once.\n", &[]);

    assert_eq!(clean.status.code(), Some(0));
    assert_eq!(stdout(&clean), "");
}

#[test]
fn an_unreadable_file_exits_two() {
    let output = scan(Some(&example("clean/nothing-here.txt")), "", &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to read"));
}

#[test]
fn no_color_suppresses_the_escape_codes() {
    let path = example("slop/word-choice.txt");

    let colored = scan(Some(&path), "", &[("FORCE_COLOR", "1")]);
    assert!(
        stdout(&colored).contains('\u{1b}'),
        "the report should be colored when color is forced"
    );

    let plain = scan(Some(&path), "", &[("FORCE_COLOR", "1"), ("NO_COLOR", "1")]);
    assert!(!stdout(&plain).contains('\u{1b}'));
}

#[test]
fn a_multiline_match_is_indented_under_its_finding() {
    let output = scan(
        None,
        "We built it fast.\nWe built it wrong.\nWe built it twice.\n",
        &[("NO_COLOR", "1")],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stdout(&output),
        concat!(
            "⚠ medium sentence_structure.anaphora_abuse\n",
            "  ├─ struct 0:55\n",
            "  └─ We built it fast.\n",
            "     We built it wrong.\n",
            "     We built it twice.\n",
            "⚠ medium paragraph_structure.short_punchy_fragments\n",
            "  ├─ struct 0:55\n",
            "  └─ We built it fast.\n",
            "     We built it wrong.\n",
            "     We built it twice.\n",
        )
    );
}

fn example(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../meta/examples")
        .join(relative)
}

/// Runs the binary over a file or stdin, with the color environment cleared
/// so only the variables a test sets are in play.
fn scan(path: Option<&Path>, input: &str, environment: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tropius-cli"));
    command.env_remove("NO_COLOR");
    command.env_remove("FORCE_COLOR");
    command.env_remove("CLICOLOR_FORCE");

    for (key, value) in environment {
        command.env(key, value);
    }

    if let Some(path) = path {
        command.arg(path);
    }

    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run tropius-cli");

    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(input.as_bytes())
        .expect("failed to write stdin");

    child.wait_with_output().expect("failed to collect output")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
}
