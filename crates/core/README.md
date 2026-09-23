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
use trps_core::{LineIndex, Rules};

let text = "Let us delve into this robust ecosystem.\n\
            It is possible that they reverse.\n";

let rules = Rules::resolve(None, std::env::current_dir().ok().as_deref())
    .expect("the project dictionary loads");
let lines = LineIndex::new(text);

for finding in rules.detector.scan(text) {
    let (start, _) = lines.locate_span(finding.span);

    println!(
        "{}:{}: {} {} {}",
        start.line,
        start.column,
        finding.severity.symbol(),
        finding.rule_id,
        finding.matched,
    );
}
```

`Rules::resolve` applies the nearest `trps.toml`, `tropes.toml`, or
`tropius.toml` at or above the directory it is given, so a consumer gets the
dictionary the CLI would find. Pass `None` for the directory to scan under the
bundled patterns alone.

The detectors themselves are private. What a consumer needs is re-exported from
the crate root, so the paths above are the ones that hold across releases.

## The rest

The [documentation site][site] covers what is not here:

- scanning files
- writing a project dictionary
- suppressing a finding with an ignore marker
- the JSON report
- which published catalog each rule came from

[cli]: https://crates.io/crates/trps-cli
[site]: https://trps.stormlightlabs.org
