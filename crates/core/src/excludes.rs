//! Path excludes declared by a project dictionary.
//!
//! A repository keeps prose it never wants graded: captured research, imported
//! reference pages, anything written under a house format the bundled patterns
//! read as slop. Grading those produces findings nobody will act on, so the
//! dictionary names the paths and a scan skips them.

use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::errors::ExcludeError;

/// Paths a project dictionary has taken out of every scan.
///
/// Patterns are globs matched against the path relative to the directory
/// holding the dictionary, so they read the way a path in the repository does.
/// `*` stops at a separator and `**` crosses one, and a pattern ending in `/`
/// is the directory and everything under it.
#[derive(Debug, Default)]
pub struct Excludes {
    root: PathBuf,
    globs: GlobSet,
}

impl Excludes {
    /// Compiles `patterns`, matched against paths under `root`.
    pub fn new(root: &Path, patterns: &[String]) -> Result<Self, ExcludeError> {
        let mut builder = GlobSetBuilder::new();

        for pattern in patterns {
            builder.add(glob(pattern)?);
        }

        let root = match root.as_os_str().is_empty() {
            true => Path::new("."),
            false => root,
        };

        Ok(Self {
            root: std::path::absolute(root).unwrap_or_else(|_| root.to_path_buf()),
            globs: builder.build()?,
        })
    }

    /// Reports whether `path` was excluded.
    ///
    /// A path outside the dictionary's directory is never excluded. The
    /// excludes belong to one repository, and a path leading out of it is not
    /// one the dictionary was written about.
    pub fn excludes(&self, path: &Path) -> bool {
        if self.globs.is_empty() {
            return false;
        }

        let candidate = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());

        candidate
            .strip_prefix(&self.root)
            .is_ok_and(|relative| self.globs.is_match(relative))
    }
}

fn glob(pattern: &str) -> Result<Glob, ExcludeError> {
    let pattern = match pattern.strip_suffix('/') {
        Some(directory) => format!("{directory}/**"),
        None => pattern.to_owned(),
    };

    Ok(globset::GlobBuilder::new(&pattern)
        .literal_separator(true)
        .build()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn excludes(patterns: &[&str]) -> Excludes {
        let patterns: Vec<String> = patterns
            .iter()
            .map(|pattern| (*pattern).to_owned())
            .collect();

        Excludes::new(Path::new("/project"), &patterns).unwrap()
    }

    #[test]
    fn an_empty_exclude_list_excludes_nothing() {
        assert!(!excludes(&[]).excludes(Path::new("/project/docs/notes.md")));
    }

    #[test]
    fn a_directory_pattern_covers_everything_under_it() {
        let excludes = excludes(&["docs/notebook/"]);

        assert!(excludes.excludes(Path::new("/project/docs/notebook/page.md")));
        assert!(excludes.excludes(Path::new("/project/docs/notebook/deep/page.md")));
        assert!(!excludes.excludes(Path::new("/project/docs/guide.md")));
    }

    #[test]
    fn a_star_stops_at_a_separator_and_a_double_star_crosses_one() {
        let shallow = excludes(&["docs/*.md"]);

        assert!(shallow.excludes(Path::new("/project/docs/guide.md")));
        assert!(!shallow.excludes(Path::new("/project/docs/deep/guide.md")));

        let deep = excludes(&["docs/**/*.md"]);

        assert!(deep.excludes(Path::new("/project/docs/deep/guide.md")));
    }

    #[test]
    fn a_path_outside_the_dictionary_directory_is_never_excluded() {
        assert!(!excludes(&["**/*.md"]).excludes(Path::new("/elsewhere/docs/guide.md")));
    }

    #[test]
    fn a_relative_path_is_matched_against_the_working_directory() {
        let root = std::env::current_dir().unwrap();
        let excludes = Excludes::new(&root, &["notes/*.md".to_owned()]).unwrap();

        assert!(excludes.excludes(Path::new("notes/captured.md")));
        assert!(excludes.excludes(Path::new("./notes/captured.md")));
        assert!(!excludes.excludes(Path::new("notes/captured.txt")));
    }

    #[test]
    fn an_unparseable_pattern_is_an_error() {
        let error = Excludes::new(Path::new("/project"), &["docs/[".to_owned()]).unwrap_err();

        assert!(error.to_string().contains("docs/["), "{error}");
    }
}
