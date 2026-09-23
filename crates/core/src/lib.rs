#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

pub mod detector;
pub mod errors;
pub mod excludes;
pub mod patterns;
pub mod rules;
pub mod suppression;

pub use crate::detector::cross_file::{CrossFileFinding, Occurrence, scan_cross_file};
pub use crate::detector::{
    BUILTIN_RULE_IDS, Detector, Finding, FindingKind, LineIndex, Location, Span, group_by_span,
};
pub use crate::excludes::Excludes;
pub use crate::patterns::{Dialect, Pattern, Severity, Thresholds};
pub use crate::rules::Rules;
pub use crate::suppression::Suppressions;
