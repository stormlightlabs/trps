//! Integration tests for the `trps` command line.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Prose carrying `harness` as a domain noun beside a genuine trope.
const DOMAIN_PROSE: &str = "The capture harness will delve into the logs.\n";

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
            "  ├─ struct 1:1-3:18\n",
            "  └─ We built it fast.\n",
            "     We built it wrong.\n",
            "     We built it twice.\n",
            "⚠ medium paragraph_structure.short_punchy_fragments\n",
            "  ├─ struct 1:1-3:18\n",
            "  └─ We built it fast.\n",
            "     We built it wrong.\n",
            "     We built it twice.\n",
        )
    );
}

#[test]
fn a_named_dictionary_drops_an_allowed_phrase() {
    let directory = case_dir("named-dictionary");
    write(&directory, "input.md", DOMAIN_PROSE);
    write(&directory, "empty.toml", "");
    write(&directory, "allow.toml", "allow = [\"harness\"]\n");

    let bundled = run(
        Some(&directory),
        &["--dictionary", "empty.toml", "input.md"],
        "",
        &[],
    );

    assert_eq!(bundled.status.code(), Some(1));
    assert!(stdout(&bundled).contains("harness"));

    let extended = run(
        Some(&directory),
        &["--dictionary", "allow.toml", "input.md"],
        "",
        &[],
    );

    assert_eq!(extended.status.code(), Some(1));
    assert!(!stdout(&extended).contains("harness"));
    assert!(stdout(&extended).contains("word_choice.delve"));
}

#[test]
fn a_discovered_dictionary_applies_without_a_flag() {
    let directory = case_dir("discovered-dictionary");
    write(&directory, "input.md", DOMAIN_PROSE);
    write(&directory, "trps.toml", "allow = [\"harness\"]\n");

    let output = run(Some(&directory), &["input.md"], "", &[]);

    assert_eq!(output.status.code(), Some(1));
    assert!(!stdout(&output).contains("harness"));
    assert!(stdout(&output).contains("word_choice.delve"));
}

#[test]
fn a_named_dictionary_is_taken_over_a_discovered_one() {
    let directory = case_dir("dictionary-precedence");
    write(&directory, "input.md", DOMAIN_PROSE);
    write(&directory, "trps.toml", "allow = [\"harness\"]\n");
    write(&directory, "named.toml", "allow = [\"delve into\"]\n");

    let output = run(
        Some(&directory),
        &["--dictionary", "named.toml", "input.md"],
        "",
        &[],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("harness"));
    assert!(!stdout(&output).contains("delve into"));
}

#[test]
fn a_declared_pattern_wins_an_overlap_with_a_bundled_phrase() {
    let directory = case_dir("overlapping-dictionary");
    write(
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

    let output = run(Some(&directory), &["input.md"], "", &[]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("project.landscape_architecture"));
    assert!(!stdout(&output).contains("word_choice.grandiose_nouns"));
}

#[test]
fn a_dictionary_repeating_a_pattern_id_exits_two() {
    let directory = case_dir("duplicate-id-dictionary");
    write(&directory, "input.md", DOMAIN_PROSE);
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

    let output = run(Some(&directory), &["input.md"], "", &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate pattern id `project.dup`"));
}

#[test]
fn a_missing_dictionary_exits_two() {
    let directory = case_dir("missing-dictionary");
    write(&directory, "input.md", DOMAIN_PROSE);

    let output = run(
        Some(&directory),
        &["--dictionary", "absent.toml", "input.md"],
        "",
        &[],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("absent.toml"));
}

#[test]
fn several_paths_are_scanned_and_each_finding_names_its_file() {
    let output = run(
        None,
        &[
            example("slop/word-choice.txt").to_str().unwrap(),
            example("clean/field-notes.txt").to_str().unwrap(),
            example("slop/repetition.txt").to_str().unwrap(),
        ],
        "",
        &[("NO_COLOR", "1")],
    );

    let report = stdout(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(report.contains("slop/word-choice.txt:1:7-16"));
    assert!(report.contains("slop/repetition.txt:"));
    assert!(
        !report.contains("clean/field-notes.txt"),
        "a file with no findings should not appear at all"
    );
}

#[test]
fn an_unreadable_path_among_several_exits_two_before_reporting() {
    let output = run(
        None,
        &[
            example("slop/word-choice.txt").to_str().unwrap(),
            example("clean/nothing-here.txt").to_str().unwrap(),
        ],
        "",
        &[("NO_COLOR", "1")],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to read"));
}

#[test]
fn json_locates_a_finding_by_path_line_and_column() {
    let directory = case_dir("json-location");
    write(
        &directory,
        "input.md",
        "The parser reads a file once.\nA caf\u{e9} and we delve into the logs.\n",
    );

    let output = run(Some(&directory), &["--json", "input.md"], "", &[]);

    assert_eq!(output.status.code(), Some(1));

    let report: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("the report is JSON");

    assert_eq!(report["version"], 1);

    let finding = report["findings"]
        .as_array()
        .expect("findings is an array")
        .iter()
        .find(|finding| finding["rule_id"] == "word_choice.delve")
        .expect("the delve pattern is reported");

    assert_eq!(finding["kind"], "phrase");
    assert_eq!(finding["path"], "input.md");
    assert_eq!(finding["line"], 2);
    assert_eq!(finding["column"], 15, "columns count characters, not bytes");
    assert_eq!(finding["matched"], "delve into");
    assert!(finding["severity"].is_string());
    assert!(finding["rule_name"].is_string());
}

#[test]
fn json_names_the_dictionary_the_run_used() {
    let directory = case_dir("json-dictionary");
    write(&directory, "input.md", DOMAIN_PROSE);
    write(&directory, "trps.toml", "allow = [\"harness\"]\n");

    let discovered = run(Some(&directory), &["--json", "input.md"], "", &[]);
    let report: serde_json::Value =
        serde_json::from_str(&stdout(&discovered)).expect("the report is JSON");

    assert_eq!(discovered.status.code(), Some(1));
    assert!(
        report["dictionary"]
            .as_str()
            .is_some_and(|path| path.ends_with("trps.toml")),
        "a discovered dictionary is reported by the path the run resolved"
    );
    assert!(!stdout(&discovered).contains("harness"));

    let named = run(
        Some(&directory),
        &["--json", "--dictionary", "trps.toml", "input.md"],
        "",
        &[],
    );
    let report: serde_json::Value =
        serde_json::from_str(&stdout(&named)).expect("the report is JSON");

    assert_eq!(report["dictionary"], "trps.toml");

    let none = run(None, &["--json"], "The parser reads a file once.\n", &[]);
    let report: serde_json::Value =
        serde_json::from_str(&stdout(&none)).expect("the report is JSON");

    assert!(report["dictionary"].is_null());
}

#[test]
fn json_over_several_paths_carries_a_clean_run_as_an_empty_list() {
    let output = run(
        None,
        &[
            "--json",
            example("clean/field-notes.txt").to_str().unwrap(),
            example("clean/changelog.md").to_str().unwrap(),
        ],
        "",
        &[],
    );

    let report: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("the report is JSON");

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(report["findings"].as_array().map(Vec::len), Some(0));
}

#[test]
fn a_file_finding_reports_the_path_line_and_column() {
    let path = example("slop/word-choice.txt");
    let output = scan(Some(&path), "", &[("NO_COLOR", "1")]);

    let expected = format!("  ├─ phrase {}:1:7-16\n", path.display());
    assert!(
        stdout(&output).contains(&expected),
        "expected {expected:?} in:\n{}",
        stdout(&output)
    );
}

#[test]
fn a_stdin_finding_reports_the_line_and_column_alone() {
    let output = scan(
        None,
        "A clean opening line.\nLet us delve into this.\n",
        &[("NO_COLOR", "1")],
    );

    assert_eq!(
        stdout(&output),
        concat!(
            "⚠ medium word_choice.delve\n",
            "  ├─ phrase 2:8-17\n",
            "  └─ delve into\n",
        )
    );
}

#[test]
fn crlf_text_reports_the_same_place_as_lf_text() {
    let prose = "A clean opening line.\nLet us delve into this.\n";
    let environment = [("NO_COLOR", "1")];

    let lf = scan(None, prose, &environment);
    let crlf = scan(None, &prose.replace('\n', "\r\n"), &environment);

    assert!(
        stdout(&lf).contains("  ├─ phrase 2:8-17\n"),
        "{}",
        stdout(&lf)
    );
    assert_eq!(stdout(&crlf), stdout(&lf));
}

#[test]
fn an_excluded_path_is_left_out_of_the_scan() {
    let directory = case_dir("excluded-path");
    write(&directory, "guide.md", DOMAIN_PROSE);
    write(&directory, "notebook/captured.md", DOMAIN_PROSE);
    write(&directory, "trps.toml", "exclude = [\"notebook/\"]\n");

    let output = run(
        Some(&directory),
        &["--json", "guide.md", "notebook/captured.md"],
        "",
        &[],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("guide.md"));
    assert!(
        !stdout(&output).contains("captured.md"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_run_over_only_excluded_paths_finds_nothing() {
    let directory = case_dir("wholly-excluded");
    write(&directory, "notebook/captured.md", DOMAIN_PROSE);
    write(&directory, "trps.toml", "exclude = [\"notebook/**\"]\n");

    let output = run(Some(&directory), &["notebook/captured.md"], "", &[]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "");
}

#[test]
fn an_exclude_pattern_that_is_not_a_glob_exits_two() {
    let directory = case_dir("bad-exclude");
    write(&directory, "input.md", DOMAIN_PROSE);
    write(&directory, "trps.toml", "exclude = [\"notebook/[\"]\n");

    let output = run(Some(&directory), &["input.md"], "", &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("notebook/["));
}

#[test]
fn a_marked_line_is_kept_out_of_the_report() {
    let output = scan(
        None,
        "<!-- trps-ignore-next-line -->\nLet us delve into this.\nThe parser reads a file once.\n",
        &[("NO_COLOR", "1")],
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_marked_region_is_kept_out_of_the_report() {
    let output = scan(
        None,
        concat!(
            "Quoting the thing it criticizes:\n",
            "<!-- trps-ignore-start -->\n",
            "Let us delve into this robust ecosystem.\n",
            "<!-- trps-ignore-end -->\n",
            "Which is why we delve into nothing.\n",
        ),
        &[("NO_COLOR", "1")],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains("  ├─ phrase 5:17-26\n"),
        "{}",
        stdout(&output)
    );
    assert_eq!(stdout(&output).matches("word_choice.delve").count(), 1);
}

#[test]
fn a_marker_naming_a_rule_keeps_the_other_findings() {
    let output = scan(
        None,
        "<!-- trps-ignore-next-line word_choice.delve -->\nLet us delve into a → b → c → world.\n",
        &[("NO_COLOR", "1")],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(
        !stdout(&output).contains("word_choice.delve"),
        "{}",
        stdout(&output)
    );
    assert!(
        stdout(&output).contains("unicode_decoration"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_marker_naming_no_rule_warns_without_failing_the_run() {
    let known = scan(
        None,
        "<!-- trps-ignore-next-line word_choice.delve -->\nLet us delve into this.\n",
        &[("NO_COLOR", "1")],
    );

    assert_eq!(known.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&known.stderr), "");

    let typo = scan(
        None,
        "<!-- trps-ignore-next-line word_choise -->\nLet us delve into this.\n",
        &[("NO_COLOR", "1")],
    );

    assert_eq!(typo.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&typo.stderr),
        "warning: 1:1 no rule is named `word_choise`\n"
    );
    assert!(stdout(&typo).contains("word_choice.delve"));
}

#[test]
fn a_warning_names_its_file_and_leaves_the_json_alone() {
    let directory = case_dir("unknown-rule-warning");
    write(
        &directory,
        "input.md",
        "<!-- trps-ignore-start word_choise -->\nLet us delve into this.\n",
    );

    let output = run(
        Some(&directory),
        &["--json", "input.md"],
        "",
        &[("NO_COLOR", "1")],
    );

    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "warning: input.md:1:1 no rule is named `word_choise`\n"
    );

    let report: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("the report is JSON");

    assert_eq!(report["findings"][0]["rule_id"], "word_choice.delve");
}

#[test]
fn no_dialect_is_enforced_until_a_dictionary_names_one() {
    let output = scan(None, "The judgement was coloured.\n", &[("NO_COLOR", "1")]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_dialect_reports_the_other_spelling_and_the_one_it_expects() {
    let directory = case_dir("american-dialect");
    write(&directory, "input.md", "The judgement was coloured.\n");
    write(&directory, "trps.toml", "dialect = \"american\"\n");

    let output = run(Some(&directory), &["input.md"], "", &[("NO_COLOR", "1")]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stdout(&output),
        concat!(
            "⚠ medium word_choice.dialect_spelling\n",
            "  ├─ spelling input.md:1:5-13\n",
            "  └─ judgement → judgment\n",
            "⚠ medium word_choice.dialect_spelling\n",
            "  ├─ spelling input.md:1:19-26\n",
            "  └─ coloured → colored\n",
        )
    );
}

#[test]
fn a_british_dictionary_reports_the_american_spelling() {
    let directory = case_dir("british-dialect");
    write(&directory, "input.md", "The judgment was colored.\n");
    write(&directory, "trps.toml", "dialect = \"british\"\n");

    let output = run(Some(&directory), &["input.md"], "", &[("NO_COLOR", "1")]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("judgment → judgement"));
    assert!(stdout(&output).contains("colored → coloured"));
}

#[test]
fn json_carries_the_expected_spelling_on_the_dialect_rule_alone() {
    let directory = case_dir("dialect-json");
    write(
        &directory,
        "input.md",
        "The judgement will delve into the logs.\n",
    );
    write(&directory, "trps.toml", "dialect = \"american\"\n");

    let output = run(Some(&directory), &["--json", "input.md"], "", &[]);
    let report: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("the report is JSON");
    let findings = report["findings"].as_array().expect("findings are a list");

    let spelling = findings
        .iter()
        .find(|finding| finding["rule_id"] == "word_choice.dialect_spelling")
        .expect("the dialect rule reported");

    assert_eq!(spelling["kind"], "spelling");
    assert_eq!(spelling["matched"], "judgement");
    assert_eq!(spelling["expected"], "judgment");

    let phrase = findings
        .iter()
        .find(|finding| finding["rule_id"] == "word_choice.delve")
        .expect("the phrase rule reported");

    assert!(phrase.get("expected").is_none());
}

#[test]
fn a_dialect_finding_is_suppressed_like_any_other() {
    let directory = case_dir("dialect-suppression");
    write(
        &directory,
        "input.md",
        "<!-- trps-ignore-next-line word_choice.dialect_spelling -->\nThe judgement stands.\n",
    );
    write(&directory, "trps.toml", "dialect = \"american\"\n");

    let output = run(Some(&directory), &["input.md"], "", &[("NO_COLOR", "1")]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "");
}

fn example(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../meta/examples")
        .join(relative)
}

/// Runs the binary over a file or stdin, with the color environment cleared
/// so only the variables a test sets are in play.
fn scan(path: Option<&Path>, input: &str, environment: &[(&str, &str)]) -> Output {
    let arguments: Vec<&str> = path
        .map(|path| path.to_str().expect("example paths are utf-8"))
        .into_iter()
        .collect();

    run(None, &arguments, input, environment)
}

/// Runs the binary with `arguments`, from `directory` when one is given.
fn run(
    directory: Option<&Path>,
    arguments: &[&str],
    input: &str,
    environment: &[(&str, &str)],
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_trps"));
    command.env_remove("NO_COLOR");
    command.env_remove("FORCE_COLOR");
    command.env_remove("CLICOLOR_FORCE");

    for (key, value) in environment {
        command.env(key, value);
    }

    if let Some(directory) = directory {
        command.current_dir(directory);
    }

    command.args(arguments);

    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run trps-cli");

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

/// An empty directory under the test target directory, one per case so the
/// dictionary a case writes reaches only its own run.
fn case_dir(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);

    if directory.exists() {
        fs::remove_dir_all(&directory).expect("failed to clear the case directory");
    }

    fs::create_dir_all(&directory).expect("failed to create the case directory");

    directory
}

/// Writes a case file, creating the directories `name` reaches through.
fn write(directory: &Path, name: &str, contents: &str) {
    let path = directory.join(name);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create a case directory");
    }

    fs::write(path, contents).expect("failed to write a case file");
}
