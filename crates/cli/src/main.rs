//! Command-line interface for scanning prose with bundled trope detectors.

use std::fs;
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::Parser;
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use trps_core::{
    detector::{Detector, Finding, LineIndex, Location},
    excludes::Excludes,
    patterns::{
        Severity, apply_dictionary, bundled_patterns, find_project_dictionary, load_pattern_file,
    },
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
}

/// What a run's project dictionary decides: the detector built from it, the
/// paths it keeps out of the scan, and the file itself so the JSON report can
/// name it.
struct Rules {
    detector: Detector,
    excludes: Excludes,
    dictionary: Option<PathBuf>,
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
    /// The spelling the project's dialect uses, on the one rule that knows.
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
    let rules = build_rules(args.dictionary)?;
    let sources = read_sources(args.inputs, &rules.excludes)?;

    let scanned: Vec<(Source, Vec<Finding>)> = sources
        .into_iter()
        .map(|source| {
            let findings = rules.detector.scan(&source.text);
            (source, findings)
        })
        .collect();

    warn_unknown_rules(&scanned, &rules.detector);

    match args.json {
        true => print_json(&scanned, rules.dictionary)?,
        false => print_report(&scanned),
    }

    Ok(scanned.iter().any(|(_, findings)| !findings.is_empty()))
}

/// Resolves the one dictionary a run applies to every path.
fn build_rules(dictionary: Option<PathBuf>) -> Result<Rules, String> {
    let mut patterns = bundled_patterns().map_err(|error| error.to_string())?;
    let mut excludes = Excludes::default();

    let dictionary = dictionary.or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|directory| find_project_dictionary(&directory))
    });

    let mut dialect = None;

    if let Some(path) = &dictionary {
        let file = load_pattern_file(path).map_err(|error| error.to_string())?;
        let root = path.parent().unwrap_or(Path::new("."));

        excludes = Excludes::new(root, &file.exclude).map_err(|error| error.to_string())?;
        dialect = file.dialect;
        patterns = apply_dictionary(patterns, &file);
    }

    let mut detector = Detector::new(patterns).map_err(|error| error.to_string())?;

    if let Some(dialect) = dialect {
        detector = detector
            .with_dialect(dialect)
            .map_err(|error| error.to_string())?;
    }

    Ok(Rules {
        detector,
        excludes,
        dictionary,
    })
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

fn print_json(
    scanned: &[(Source, Vec<Finding>)],
    dictionary: Option<PathBuf>,
) -> Result<(), String> {
    let mut findings = Vec::new();

    for (source, source_findings) in scanned {
        let index = LineIndex::new(&source.text);

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
    };

    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to write the JSON report: {error}"))?;

    println!("{json}");

    Ok(())
}

/// Warns about a marker naming a rule nothing reports, which suppresses
/// nothing and would otherwise fail in silence.
///
/// Warnings go to stderr, so a `--json` run still writes one document to
/// stdout, and they do not change the exit code: a typo in a marker is worth
/// saying and not worth failing a build over.
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
fn print_report(scanned: &[(Source, Vec<Finding>)]) {
    for (source, findings) in scanned {
        let index = LineIndex::new(&source.text);

        for finding in findings {
            print_finding(finding, &source.name, &index);
        }
    }
}

fn print_finding(finding: &Finding, name: &str, index: &LineIndex) {
    println!(
        "{} {} {}",
        finding.severity.symbol(),
        severity_label(finding.severity),
        finding
            .rule_id
            .if_supports_color(Stream::Stdout, |text| text.bold()),
    );
    println!(
        "  ├─ {} {}",
        finding.kind.label(),
        origin(name, index.locate_span(finding.span)),
    );
    println!("  └─ {}", matched_text(finding));
}

/// Renders what a finding matched, and the spelling it expected where the
/// rule carries one, so a dialect fix needs no lookup.
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

/// Renders where a finding is, as `path:line:column-column`, dropping the
/// path for stdin, which has none, and the end line for a finding that sits
/// on one line.
fn origin(name: &str, (start, end): (Location, Location)) -> String {
    let range = match start.line == end.line {
        true => format!("{start}-{}", end.column),
        false => format!("{start}-{end}"),
    };

    in_file(name, range)
}

/// Prefixes a place in a file with the file, dropping the path for stdin,
/// which has none.
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
