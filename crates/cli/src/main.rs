//! Command-line interface for scanning prose with bundled trope detectors.

use std::fs;
use std::{
    io::{self, Read},
    path::PathBuf,
    process::ExitCode,
};

use clap::Parser;
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use trps_core::{
    detector::{
        Detector, Finding, FindingKind, LineIndex, Location,
        cross_file::{CrossFileFinding, scan_cross_file},
        group_by_span,
    },
    excludes::Excludes,
    patterns::Severity,
    rules::Rules,
    suppression::Suppressions,
};

/// Name the JSON report gives to text read from stdin.
const STDIN_NAME: &str = "-";

/// Version of the JSON report shape, raised when a consumer would have to
/// change to keep reading it.
const REPORT_VERSION: u32 = 1;

#[derive(Debug, Parser)]
#[command(name = "trps", about = "Detect AI writing tropes in prose.")]
struct Args {
    /// Files to scan. Reads stdin when none are given.
    inputs: Vec<PathBuf>,
    /// Project dictionary to apply. Defaults to the nearest `trps.toml`,
    /// `tropes.toml`, or `tropius.toml` in the repository.
    #[arg(long, value_name = "PATH")]
    dictionary: Option<PathBuf>,
    /// Report findings as JSON on stdout instead of a decorated report.
    #[arg(long)]
    json: bool,
    /// Do not print the line a clean run writes to stderr.
    #[arg(short, long)]
    quiet: bool,
}

/// Text to scan, under the name the report gives it.
struct Source {
    name: String,
    text: String,
}

/// One run's findings over every path it was given.
#[derive(Serialize)]
struct Report<'a> {
    version: u32,
    dictionary: Option<String>,
    findings: Vec<ReportFinding<'a>>,
    /// Runs shared by several of the scanned files. Absent when a run found
    /// none, which is every run over a single path.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cross_file: Vec<SharedFinding<'a>>,
}

/// A run several files share, located once per place it appears.
#[derive(Serialize)]
struct SharedFinding<'a> {
    rule_id: &'a str,
    rule_name: &'a str,
    severity: Severity,
    kind: String,
    matched: &'a str,
    occurrences: Vec<SharedPlace<'a>>,
}

/// One place a shared run appears.
#[derive(Serialize)]
struct SharedPlace<'a> {
    path: &'a str,
    line: usize,
    column: usize,
}

/// A finding located by path, line, and column rather than byte offset.
#[derive(Serialize)]
struct ReportFinding<'a> {
    rule_id: &'a str,
    rule_name: &'a str,
    severity: Severity,
    kind: String,
    path: &'a str,
    line: usize,
    column: usize,
    matched: &'a str,
    /// The spelling the project's dialect uses. Only the dialect rule sets it.
    #[serde(skip_serializing_if = "Option::is_none")]
    expected: Option<&'a str>,
}

fn main() -> ExitCode {
    if std::env::var_os("NO_COLOR").is_some() {
        owo_colors::set_override(false);
    }

    match run(Args::parse()) {
        Ok(has_findings) => match has_findings {
            true => ExitCode::from(1),
            false => ExitCode::SUCCESS,
        },
        Err(error) => {
            eprintln!(
                "{} {error}",
                "error:".if_supports_color(Stream::Stderr, |text| text.red())
            );
            ExitCode::from(2)
        }
    }
}

fn run(args: Args) -> Result<bool, String> {
    // A working directory that cannot be read is no directory to search from,
    // so the run applies the bundled patterns rather than whatever a relative
    // search would turn up.
    let from = std::env::current_dir().ok();
    let rules = Rules::resolve(args.dictionary.as_deref(), from.as_deref())
        .map_err(|error| error.to_string())?;
    let sources = read_sources(args.inputs, &rules.excludes)?;

    let scanned: Vec<(Source, Vec<Finding>)> = sources
        .into_iter()
        .map(|source| {
            let findings = rules.detector.scan(&source.text);
            (source, findings)
        })
        .collect();

    let texts: Vec<&str> = scanned
        .iter()
        .map(|(source, _)| source.text.as_str())
        .collect();
    let shared = scan_cross_file(&texts, rules.cross_file);

    warn_unknown_rules(&scanned, &rules.detector);

    match args.json {
        true => print_json(&scanned, &shared, rules.dictionary)?,
        false => print_report(&scanned, &shared),
    }

    let has_findings =
        !shared.is_empty() || scanned.iter().any(|(_, findings)| !findings.is_empty());

    if !has_findings && !args.quiet {
        report_clean();
    }

    Ok(has_findings)
}

/// Says that a run matched nothing, and names prose it never read for.
///
/// A run that prints nothing reads as a verdict on the writing, and the
/// catalogue is narrower than that. The line goes to stderr beside the
/// warnings, so a `--json` run still writes one document to stdout, and
/// `--quiet` turns it off for anyone who would otherwise send stderr to
/// `/dev/null`.
fn report_clean() {
    eprintln!(
        "{} nothing in the catalogue matched. Hedges, filler adverbs, \
and editorial asides are not in it.",
        "clean:".if_supports_color(Stream::Stderr, |text| text.green()),
    );
}

/// Reads every path up front so an unreadable one fails before any report is
/// written.
///
/// An excluded path is dropped before it is read, so naming one the project
/// has excluded is not an error even when nothing is there to read.
fn read_sources(inputs: Vec<PathBuf>, excludes: &Excludes) -> Result<Vec<Source>, String> {
    if inputs.is_empty() {
        return Ok(vec![Source {
            name: STDIN_NAME.to_owned(),
            text: read_stdin()?,
        }]);
    }

    inputs
        .into_iter()
        .filter(|path| !excludes.excludes(path))
        .map(|path| {
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read `{}`: {error}", path.display()))?;

            Ok(Source {
                name: path.display().to_string(),
                text,
            })
        })
        .collect()
}

fn read_stdin() -> Result<String, String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("failed to read stdin: {error}"))?;

    Ok(input)
}

fn print_json<'a>(
    scanned: &'a [(Source, Vec<Finding>)],
    shared: &'a [CrossFileFinding],
    dictionary: Option<PathBuf>,
) -> Result<(), String> {
    let indexes = line_indexes(scanned);
    let mut findings = Vec::new();

    for ((source, source_findings), index) in scanned.iter().zip(&indexes) {
        findings.extend(source_findings.iter().map(|finding| {
            let location = index.locate(finding.span.start());

            ReportFinding {
                rule_id: &finding.rule_id,
                rule_name: &finding.rule_name,
                severity: finding.severity,
                kind: finding.kind.label(),
                path: &source.name,
                line: location.line,
                column: location.column,
                matched: &finding.matched,
                expected: finding.expected.as_deref(),
            }
        }));
    }

    let report = Report {
        version: REPORT_VERSION,
        dictionary: dictionary.map(|path| path.display().to_string()),
        findings,
        cross_file: shared
            .iter()
            .map(|finding| SharedFinding {
                rule_id: &finding.rule_id,
                rule_name: &finding.rule_name,
                severity: finding.severity,
                kind: FindingKind::Repetition.label(),
                matched: &finding.matched,
                occurrences: finding
                    .occurrences
                    .iter()
                    .map(|occurrence| {
                        let location = indexes[occurrence.document].locate(occurrence.span.start());

                        SharedPlace {
                            path: &scanned[occurrence.document].0.name,
                            line: location.line,
                            column: location.column,
                        }
                    })
                    .collect(),
            })
            .collect(),
    };

    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to write the JSON report: {error}"))?;

    println!("{json}");

    Ok(())
}

/// Warns about a marker naming a rule nothing reports. The marker suppresses
/// nothing, and nothing else in a run says so.
///
/// Warnings go to stderr, so a `--json` run still writes one document to
/// stdout, and they leave the exit code alone: a typo in a marker does not
/// fail a build.
fn warn_unknown_rules(scanned: &[(Source, Vec<Finding>)], detector: &Detector) {
    for (source, _) in scanned {
        let index = LineIndex::new(&source.text);

        for (rule, offset) in Suppressions::new(&source.text).unknown_rules(detector.rule_ids()) {
            eprintln!(
                "{} {} no rule is named `{rule}`",
                "warning:".if_supports_color(Stream::Stderr, |text| text.yellow()),
                in_file(&source.name, index.locate(offset).to_string()),
            );
        }
    }
}

/// Prints the decorated report. Every finding names its own place, so a run
/// over several paths needs no heading to say which file it is reading.
///
/// What several files share comes last, after each file has been read on its
/// own terms.
fn print_report(scanned: &[(Source, Vec<Finding>)], shared: &[CrossFileFinding]) {
    let indexes = line_indexes(scanned);

    for ((source, findings), index) in scanned.iter().zip(&indexes) {
        for group in group_by_span(findings) {
            print_group(group, &source.name, index);
        }
    }

    for finding in shared {
        print_shared_finding(finding, scanned, &indexes);
    }
}

/// One line index per scanned source, built once for every pass that reads
/// them.
fn line_indexes(scanned: &[(Source, Vec<Finding>)]) -> Vec<LineIndex<'_>> {
    scanned
        .iter()
        .map(|(source, _)| LineIndex::new(&source.text))
        .collect()
}

/// Prints a run several files share, naming every place it appears so a
/// reader can tell a convention from a tic.
fn print_shared_finding(
    finding: &CrossFileFinding,
    scanned: &[(Source, Vec<Finding>)],
    indexes: &[LineIndex],
) {
    print_heading(finding.severity, FindingKind::Repetition, &finding.rule_id);

    for occurrence in &finding.occurrences {
        println!(
            "  ├─ {}",
            origin(
                &scanned[occurrence.document].0.name,
                indexes[occurrence.document].locate_span(occurrence.span),
            ),
        );
    }

    println!(
        "  └─ {}",
        indented_match(&finding.matched).if_supports_color(Stream::Stdout, |text| text.yellow())
    );
}

/// Prints one passage as one entry: every rule that fired on it, then where
/// it is and what it says.
///
/// Each rule names its own detector, because two rules sharing a span need
/// not have come from the same one. The entry quotes each distinct text once,
/// which is nearly always one. A rule that counts occurrences instead of
/// quoting its span, as `formatting.unicode_decoration` does, is what makes
/// more than one possible.
fn print_group(group: &[Finding], name: &str, index: &LineIndex) {
    for finding in group {
        print_heading(finding.severity, finding.kind, &finding.rule_id);
    }

    let mut quoted: Vec<String> = Vec::new();

    for finding in group {
        let matched = matched_text(finding);

        if !quoted.contains(&matched) {
            quoted.push(matched);
        }
    }

    let (last, rest) = quoted
        .split_last()
        .expect("a group holds at least one finding");

    println!("  ├─ {}", origin(name, index.locate_span(group[0].span)));

    for matched in rest {
        println!("  ├─ {matched}");
    }

    println!("  └─ {last}");
}

/// Prints the first line of a finding: its severity, the detector it came
/// from, and the rule that fired.
///
/// The detector belongs here rather than beside the place, because an entry
/// holding several rules need not have them from one detector.
fn print_heading(severity: Severity, kind: FindingKind, rule_id: &str) {
    println!(
        "{} {} {} {}",
        severity.symbol(),
        severity_label(severity),
        kind.label(),
        rule_id.if_supports_color(Stream::Stdout, |text| text.bold()),
    );
}

/// Renders what a finding matched, and the form it expected where the rule
/// carries one.
fn matched_text(finding: &Finding) -> String {
    let matched = indented_match(&finding.matched)
        .if_supports_color(Stream::Stdout, |text| text.yellow())
        .to_string();

    match &finding.expected {
        Some(expected) => format!(
            "{matched} → {}",
            expected.if_supports_color(Stream::Stdout, |text| text.green())
        ),
        None => matched,
    }
}

/// Renders where a finding is, as `path:line:column-column`, collapsing the
/// end line for a finding that sits on one line.
fn origin(name: &str, (start, end): (Location, Location)) -> String {
    let range = match start.line == end.line {
        true => format!("{start}-{}", end.column),
        false => format!("{start}-{end}"),
    };

    in_file(name, range)
}

/// Prefixes a place in a file with its path. Text read from stdin has no
/// path, so it gets the place alone.
fn in_file(name: &str, place: String) -> String {
    match name == STDIN_NAME {
        true => place,
        false => format!("{name}:{place}"),
    }
}

fn indented_match(value: &str) -> String {
    value.lines().collect::<Vec<_>>().join("\n     ")
}

fn severity_label(severity: Severity) -> String {
    match severity {
        Severity::Low => "low"
            .if_supports_color(Stream::Stdout, |text| text.blue())
            .to_string(),
        Severity::Medium => "medium"
            .if_supports_color(Stream::Stdout, |text| text.yellow())
            .to_string(),
        Severity::High => "high"
            .if_supports_color(Stream::Stdout, |text| text.red())
            .to_string(),
    }
}
