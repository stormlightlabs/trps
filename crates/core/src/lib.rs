#![deny(missing_docs)]

//! Finds AI writing tropes in prose: the pattern dictionary, the detectors
//! that read it, and the run that assembles both.
//!
//! A scan is three steps. [`Rules::resolve`] settles what this run applies —
//! the bundled patterns, whatever a project dictionary adds, allows away, or
//! excludes, and the counts the counting rules fire at. [`Detector::scan`]
//! reads the text and returns a [`Finding`] per hit, each carrying a rule id,
//! a [`Severity`], the text that matched, and a [`Span`] of byte offsets.
//! [`LineIndex`] turns a span into the [`Location`] a report prints.
//!
//! ```
//! use trps_core::{LineIndex, Rules};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let text = "Let us delve into this robust ecosystem.\n\
//!             It is possible that they reverse.\n";
//!
//! // Applies the nearest `trps.toml` at or above the directory the run
//! // started in. Where there is none, the bundled patterns stand alone.
//! let rules = Rules::resolve(None, std::env::current_dir().ok().as_deref())?;
//! let lines = LineIndex::new(text);
//!
//! for finding in rules.detector.scan(text) {
//!     let (start, _) = lines.locate_span(finding.span);
//!
//!     println!(
//!         "{}:{}: {} {} {}",
//!         start.line,
//!         start.column,
//!         finding.severity.symbol(),
//!         finding.rule_id,
//!         finding.matched,
//!     );
//! }
//! # Ok(())
//! # }
//! ```
//!
//! The detectors themselves are private. What a consumer needs is re-exported
//! here, so the paths above are the ones that hold across releases.

pub mod detector;
pub mod errors;
pub mod excludes;
pub mod patterns;
pub mod rules;
pub mod suppression;

pub use crate::detector::cross_file::{CrossFileFinding, scan_cross_file};
pub use crate::detector::{
    BUILTIN_RULE_IDS, Detector, Finding, FindingKind, LineIndex, Location, Span, group_by_span,
};
pub use crate::excludes::Excludes;
pub use crate::patterns::{Pattern, Severity, Thresholds};
pub use crate::rules::Rules;
pub use crate::suppression::Suppressions;
