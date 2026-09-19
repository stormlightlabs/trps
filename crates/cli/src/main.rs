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
use tropius_core::{
    detector::{Detector, Finding},
    patterns::{
        Severity, apply_dictionary, bundled_patterns, find_project_dictionary, load_pattern_file,
    },
};

/// Name the JSON report gives to text read from stdin.
const STDIN_NAME: &str = "-";

/// Version of the JSON report shape, raised when a consumer would have to
/// change to keep reading it.
const REPORT_VERSION: u32 = 1;

#[derive(Debug, Parser)]
#[command(about = "Detect AI writing tropes in prose.")]
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
    let sources = read_sources(args.inputs)?;
    let (detector, dictionary) = build_detector(args.dictionary)?;

    let scanned: Vec<(Source, Vec<Finding>)> = sources
        .into_iter()
        .map(|source| {
            let findings = detector.scan(&source.text);
            (source, findings)
        })
        .collect();

    match args.json {
        true => print_json(&scanned, dictionary)?,
        false => print_report(&scanned),
    }

    Ok(scanned.iter().any(|(_, findings)| !findings.is_empty()))
}

/// Builds the one detector a run applies to every path, and returns the
/// dictionary it was built from so the JSON report can name it.
fn build_detector(dictionary: Option<PathBuf>) -> Result<(Detector, Option<PathBuf>), String> {
    let mut patterns = bundled_patterns().map_err(|error| error.to_string())?;

    let dictionary = dictionary.or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|directory| find_project_dictionary(&directory))
    });

    if let Some(path) = &dictionary {
        let file = load_pattern_file(path).map_err(|error| error.to_string())?;
        patterns = apply_dictionary(patterns, &file);
    }

    let detector = Detector::new(patterns).map_err(|error| error.to_string())?;

    Ok((detector, dictionary))
}

/// Reads every path up front so an unreadable one fails before any report is
/// written.
fn read_sources(inputs: Vec<PathBuf>) -> Result<Vec<Source>, String> {
    if inputs.is_empty() {
        return Ok(vec![Source {
            name: STDIN_NAME.to_owned(),
            text: read_stdin()?,
        }]);
    }

    inputs
        .into_iter()
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
    let findings = scanned
        .iter()
        .flat_map(|(source, findings)| {
            findings.iter().map(|finding| {
                let (line, column) = line_and_column(&source.text, finding.span.start());

                ReportFinding {
                    rule_id: &finding.rule_id,
                    rule_name: &finding.rule_name,
                    severity: finding.severity,
                    kind: finding.kind.label(),
                    path: &source.name,
                    line,
                    column,
                    matched: &finding.matched,
                }
            })
        })
        .collect();

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

/// Prints the decorated report, heading each file's findings with its path
/// when a run covers more than one.
fn print_report(scanned: &[(Source, Vec<Finding>)]) {
    let name_the_source = scanned.len() > 1;

    for (source, findings) in scanned {
        if findings.is_empty() {
            continue;
        }

        if name_the_source {
            println!(
                "{}",
                source
                    .name
                    .if_supports_color(Stream::Stdout, |text| text.bold())
            );
        }

        for finding in findings {
            print_finding(finding);
        }
    }
}

/// Returns the 1-based line and character column of a byte offset.
fn line_and_column(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset];
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);

    (
        before.matches('\n').count() + 1,
        before[line_start..].chars().count() + 1,
    )
}

fn print_finding(finding: &Finding) {
    println!(
        "{} {} {}",
        finding.severity.symbol(),
        severity_label(finding.severity),
        finding
            .rule_id
            .if_supports_color(Stream::Stdout, |text| text.bold()),
    );
    println!(
        "  ├─ {} {}:{}",
        finding.kind.label(),
        finding.span.start(),
        finding.span.end(),
    );
    println!(
        "  └─ {}",
        indented_match(&finding.matched).if_supports_color(Stream::Stdout, |text| text.yellow())
    );
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
