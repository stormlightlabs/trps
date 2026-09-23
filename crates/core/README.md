# trps-core

Finds AI writing tropes in prose: the pattern dictionary, the detectors that
read it, and the run that assembles both. [`trps-cli`][cli] is the command-line
front end.

A scan is three steps. `Rules::resolve` settles what this run applies — the
bundled patterns, whatever a project dictionary adds, allows away, or excludes,
and the counts the counting rules fire at. `Detector::scan` reads the text and
returns a `Finding` per hit, each carrying a rule id, a `Severity`, the text
that matched, and a `Span` of byte offsets. `LineIndex` turns a span into the
`Location` a report prints.

```rust
use std::fs;

use trps_core::{LineIndex, Rules};

// A project whose domain vocabulary collides with a bundled pattern, and
// that has a phrase of its own to report. Yours is already on disk; this
// one is written so the example runs anywhere.
let project = std::env::temp_dir()
    .join(format!("trps-example-{}", std::process::id()));

fs::create_dir_all(&project).expect("the project directory is writable");
fs::write(
    project.join("trps.toml"),
    r#"
allow = ["harness"]

[[patterns]]
id = "project.shipping_soon"
name = "Shipping Soon"
severity = "high"
phrases = ["shipping soon"]
"#,
)
.expect("the dictionary is writable");

let text = "Let us delve into this robust harness.\n\
            The parser is shipping soon.\n";

let rules = Rules::resolve(None, Some(&project))
    .expect("the project dictionary loads");
let lines = LineIndex::new(text);
let mut report = Vec::new();

for finding in rules.detector.scan(text) {
    let (start, _) = lines.locate_span(finding.span);

    report.push(format!(
        "{}:{}: {} {} {}",
        start.line,
        start.column,
        finding.severity.symbol(),
        finding.rule_id,
        finding.matched,
    ));
}

for line in &report {
    println!("{line}");
}

assert_eq!(
    rules.dictionary.as_deref(),
    Some(project.join("trps.toml").as_path()),
);
assert_eq!(
    report,
    [
        "1:8: ⚠ word_choice.delve delve into",
        "1:24: ⚠ word_choice.delve robust",
        "2:15: ✕ project.shipping_soon shipping soon",
    ],
);

fs::remove_dir_all(&project).expect("the project directory is removable");
```

The report shows the dictionary in `project` was applied: the word it allows
away is missing from it, and `project.shipping_soon` is in it because the
dictionary declares that pattern.

`Rules::resolve` searches for the nearest `trps.toml`, `tropes.toml`, or
`tropius.toml` in the directory it is given and its ancestors, stopping at the
repository root so a dictionary outside the project never reaches a scan
inside it. Outside a repository only the given directory is read. Pass `None`
for the directory to scan under the bundled patterns alone.

The detectors themselves are private. What a consumer needs is re-exported from
the crate root, so the paths above are the ones that hold across releases.

## The rest

The [reference documentation][reference] covers what is not here:

- scanning files
- writing a project dictionary
- suppressing a finding with an ignore marker
- the JSON report
- which published catalog each rule came from

[cli]: https://crates.io/crates/trps-cli
[reference]: https://github.com/stormlightlabs/trps/tree/main/docs/src/content/docs/reference
