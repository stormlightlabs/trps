//! The rules one run applies to every path it reads.
//!
//! A run is more than the bundled patterns. A project dictionary can allow
//! phrases away, declare patterns of its own, name a dialect, and keep paths
//! out of the scan, and which dictionary applies depends on where the run was
//! started. [`Rules::resolve`] settles all of it in one call, so a consumer
//! gets what the command line gets without assembling it again.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::detector::Detector;
use crate::errors::{PatternLoadError, RulesError};
use crate::excludes::Excludes;
use crate::patterns::{
    AllowedPhrase, DeclaredPattern, MarkdownOptions, Thresholds, apply_dictionary,
    bundled_patterns, describe_dictionary, find_project_dictionary, load_pattern_file,
    validate_declared_sources,
};

/// What a run's project dictionary decides.
///
/// The file itself is kept so a report can name the dictionary it applied.
#[derive(Debug)]
pub struct Rules {
    /// The detector the resolved patterns, dialect, and thresholds built.
    pub detector: Detector,
    /// The paths the dictionary took out of the scan.
    pub excludes: Excludes,
    /// The dictionary that was applied, where one was.
    pub dictionary: Option<PathBuf>,
    /// What the dictionary changed, for a report that scans nothing.
    pub resolution: Resolution,
}

/// What applying a project dictionary changed.
///
/// A dictionary that does nothing scans exactly like one that works, so the
/// only way to tell them apart is a record of what each decision touched.
/// This is that record: a run reports it rather than asking a reader to infer
/// it from findings that never appeared.
#[derive(Debug, Default)]
pub struct Resolution {
    /// Patterns the run scans with, after the dictionary was applied.
    pub patterns: usize,
    /// Patterns the dictionary declared. See [`DeclaredPattern`].
    pub declared: Vec<DeclaredPattern>,
    /// Phrases the dictionary allowed. See [`AllowedPhrase`].
    pub allowed: Vec<AllowedPhrase>,
    /// The exclude globs, as the dictionary wrote them.
    pub excludes: Vec<String>,
    /// The catalogs the dictionary registered, by key and URL.
    pub sources: BTreeMap<String, String>,
}

impl Rules {
    /// Resolves the one dictionary a run applies to every path.
    ///
    /// `dictionary` names the file to apply. Where it is `None`, the search
    /// [`find_project_dictionary`] describes runs from `from`: the nearest
    /// dictionary in that directory or an ancestor up to the repository root
    /// applies, and outside a repository only `from` itself is read. `from`
    /// is the directory the run was started in, where the caller knows it. A
    /// caller that could not read its working directory passes `None` and no
    /// search happens, rather than one from a directory that is only a guess.
    /// Where neither finds a dictionary, the bundled patterns stand alone.
    ///
    /// A dictionary's excludes are compiled against the directory holding it,
    /// not `from`, so its globs read the way a path in that repository does
    /// however the run was started.
    pub fn resolve(dictionary: Option<&Path>, from: Option<&Path>) -> Result<Self, RulesError> {
        let mut patterns = bundled_patterns()?;
        let mut excludes = Excludes::default();
        let mut thresholds = Thresholds::default();
        let mut markdown = MarkdownOptions::default();
        let mut dialect = None;
        let mut resolution = Resolution::default();

        let dictionary = dictionary
            .map(Path::to_path_buf)
            .or_else(|| from.and_then(find_project_dictionary));

        if let Some(path) = &dictionary {
            let file = load_pattern_file(path)?;
            let root = path.parent().unwrap_or(Path::new("."));

            validate_declared_sources(&file).map_err(PatternLoadError::from)?;

            let (declared, allowed) = describe_dictionary(&patterns, &file);

            resolution.declared = declared;
            resolution.allowed = allowed;
            resolution.excludes = file.exclude.clone();
            resolution.sources = file.sources.clone();

            excludes = Excludes::new(root, &file.exclude)?;
            dialect = file.dialect;
            patterns = apply_dictionary(patterns, &file);
            thresholds = file.thresholds;
            markdown = file.markdown;
        }

        resolution.patterns = patterns.len();

        let mut detector = Detector::new(patterns)?
            .with_thresholds(thresholds)
            .with_markdown(markdown);

        if let Some(dialect) = dialect {
            detector = detector.with_dialect(dialect)?;
        }

        Ok(Self {
            detector,
            excludes,
            dictionary,
            resolution,
        })
    }
}
