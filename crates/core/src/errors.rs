//! Error types returned by core pattern loading and detector construction.

use thiserror::Error;

/// Errors that can happen while building a detector.
#[derive(Debug, Error)]
pub enum DetectorBuildError {
    /// Bundled pattern files could not be loaded.
    #[error(transparent)]
    PatternLoad(#[from] PatternLoadError),
    /// Pattern dictionaries failed validation.
    #[error("invalid pattern dictionary: {0}")]
    PatternValidation(#[from] PatternValidationError),
    /// The Aho-Corasick automaton could not be built.
    #[error("failed to build phrase matcher: {0}")]
    AhoCorasick(#[from] aho_corasick::BuildError),
}

/// Errors that can happen while loading bundled pattern dictionaries.
#[derive(Debug, Error)]
pub enum PatternLoadError {
    /// A pattern file could not be read from disk.
    #[error("failed to read `{path}`: {source}")]
    Read {
        /// The path that could not be read.
        path: String,
        /// The underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// A TOML file could not be deserialized.
    #[error("failed to parse pattern TOML: {0}")]
    Toml(#[from] toml::de::Error),
    /// Pattern dictionaries parsed but failed semantic validation.
    #[error("invalid pattern dictionary: {0}")]
    Validation(#[from] PatternValidationError),
}

/// Validation errors for parsed pattern dictionaries.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum PatternValidationError {
    /// A pattern id is empty or only whitespace.
    #[error("pattern id cannot be empty")]
    EmptyPatternId,
    /// A pattern name is empty or only whitespace.
    #[error("pattern `{id}` name cannot be empty")]
    EmptyPatternName {
        /// The id of the invalid pattern.
        id: String,
    },
    /// A pattern has no phrases.
    #[error("pattern `{id}` must have at least one phrase")]
    EmptyPhraseList {
        /// The id of the invalid pattern.
        id: String,
    },
    /// A pattern phrase is empty or only whitespace.
    #[error("pattern `{id}` contains an empty phrase")]
    EmptyPhrase {
        /// The id of the invalid pattern.
        id: String,
    },
    /// Two patterns use the same id.
    #[error("duplicate pattern id `{id}`")]
    DuplicatePatternId {
        /// The duplicate pattern id.
        id: String,
    },
    /// Two pattern entries use the same phrase after ASCII case folding.
    #[error("duplicate phrase `{phrase}`")]
    DuplicatePhrase {
        /// The duplicate phrase as it appeared in the later entry.
        phrase: String,
    },
}
