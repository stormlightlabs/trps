//! The rules one run applies to every path it reads.
//!
//! A run is more than the bundled patterns. A project dictionary can allow
//! phrases away, declare patterns of its own, name a dialect, and keep paths
//! out of the scan, and which dictionary applies depends on where the run was
//! started. [`Rules::resolve`] settles all of it in one call, so a consumer
//! gets what the command line gets without assembling it again.

use std::path::{Path, PathBuf};

use crate::detector::{Detector, cross_file::CrossFileLimits};
use crate::errors::RulesError;
use crate::excludes::Excludes;
use crate::patterns::{
    apply_dictionary, bundled_patterns, find_project_dictionary, load_pattern_file,
};

/// What a run's project dictionary decides.
///
/// The file itself is kept so a report can name the dictionary it applied.
#[derive(Debug)]
pub struct Rules {
    /// The detector the resolved patterns and dialect built.
    pub detector: Detector,
    /// The paths the dictionary took out of the scan.
    pub excludes: Excludes,
    /// What the dictionary counts as a repetition several files share.
    pub cross_file: CrossFileLimits,
    /// The dictionary that was applied, where one was.
    pub dictionary: Option<PathBuf>,
}

impl Rules {
    /// Resolves the one dictionary a run applies to every path.
    ///
    /// `dictionary` names the file to apply. Where it is `None`, the nearest
    /// project dictionary at or above `from` applies, under the search
    /// [`find_project_dictionary`] describes; `from` is the directory the run
    /// was started in, where the caller knows it. A caller that could not read
    /// its working directory passes `None` and no search happens, rather than
    /// one from a directory that is only a guess. Where neither finds a
    /// dictionary, the bundled patterns stand alone.
    ///
    /// A dictionary's excludes are compiled against the directory holding it,
    /// not `from`, so its globs read the way a path in that repository does
    /// however the run was started.
    pub fn resolve(dictionary: Option<&Path>, from: Option<&Path>) -> Result<Self, RulesError> {
        let mut patterns = bundled_patterns()?;
        let mut excludes = Excludes::default();
        let mut cross_file = CrossFileLimits::default();
        let mut dialect = None;

        let dictionary = dictionary
            .map(Path::to_path_buf)
            .or_else(|| from.and_then(find_project_dictionary));

        if let Some(path) = &dictionary {
            let file = load_pattern_file(path)?;
            let root = path.parent().unwrap_or(Path::new("."));

            excludes = Excludes::new(root, &file.exclude)?;
            cross_file = file.cross_file;
            dialect = file.dialect;
            patterns = apply_dictionary(patterns, &file);
        }

        let mut detector = Detector::new(patterns)?;

        if let Some(dialect) = dialect {
            detector = detector.with_dialect(dialect)?;
        }

        Ok(Self {
            detector,
            excludes,
            cross_file,
            dictionary,
        })
    }
}
