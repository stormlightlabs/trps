//! Command-line interface for scanning prose with bundled trope detectors.

use std::fs;
use std::{
    io::{self, Read},
    path::PathBuf,
    process::ExitCode,
};

use clap::Parser;
use owo_colors::{OwoColorize, Stream};
use tropius_core::{
    detector::{Detector, Finding},
    patterns::Severity,
};

#[derive(Debug, Parser)]
#[command(about = "Detect AI writing tropes in prose.")]
struct Args {
    /// File to scan. Reads stdin when omitted.
    input: Option<PathBuf>,
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
    let input = read_input(args.input)?;
    let detector = Detector::bundled().map_err(|error| error.to_string())?;
    let findings = detector.scan(&input);

    for finding in &findings {
        print_finding(finding);
    }

    Ok(!findings.is_empty())
}

fn read_input(input: Option<PathBuf>) -> Result<String, String> {
    match input {
        Some(path) => fs::read_to_string(&path)
            .map_err(|error| format!("failed to read `{}`: {error}", path.display())),
        None => {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .map_err(|error| format!("failed to read stdin: {error}"))?;
            Ok(input)
        }
    }
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
