#![deny(missing_docs)]
//! The pattern dictionary and detectors that find AI writing tropes in prose.
#![doc = include_str!("../README.md")]

mod detector;

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
pub use crate::patterns::{
    AnalogyLimits, AnaphoraLimits, BoldLeadLimits, CrossFileLimits, DashLimits, DeadMetaphorLimits,
    DecorationLimits, Dialect, DilutionLimits, DuplicationLimits, FragmentLimits, ListicleLimits,
    Pattern, RepetitionLimits, Severity, StructuralLimits, SummaryLimits, Thresholds,
    TricolonLimits,
};
pub use crate::rules::Rules;
pub use crate::suppression::Suppressions;
