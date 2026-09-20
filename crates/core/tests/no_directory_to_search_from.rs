//! What [`Rules::resolve`] does when the caller names no directory to search
//! from, which is the one case that needs a process of its own.
//!
//! Passing `None` has to skip the search rather than fall back on a relative
//! one, and the difference only shows from a working directory where a
//! relative search would find something. Setting the working directory is
//! process-wide and a test binary runs its tests in threads, so this file
//! holds the one test that sets it.

use std::fs;
use std::path::Path;

use trps_core::patterns::{PROJECT_DICTIONARY_FILES, find_project_dictionary};
use trps_core::rules::Rules;

#[test]
fn no_directory_to_search_from_finds_no_dictionary() {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-directory-to-search-from");

    if directory.exists() {
        fs::remove_dir_all(&directory).expect("failed to clear the case directory");
    }

    fs::create_dir_all(&directory).expect("failed to create the case directory");

    let dictionary = directory.join(PROJECT_DICTIONARY_FILES[0]);
    fs::write(&dictionary, "allow = [\"delve into\"]\n").unwrap();
    std::env::set_current_dir(&directory).unwrap();

    // The dictionary is there to be found: named as the directory to search
    // from, and again as the relative directory a fallback would search.
    assert_eq!(
        Rules::resolve(None, Some(&directory)).unwrap().dictionary,
        Some(dictionary)
    );
    assert_eq!(
        find_project_dictionary(Path::new(".")),
        Some(Path::new(".").join(PROJECT_DICTIONARY_FILES[0]))
    );

    assert_eq!(Rules::resolve(None, None).unwrap().dictionary, None);
}
