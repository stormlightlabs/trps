# trps-core

Finds AI writing tropes in prose. This crate holds the pattern dictionary, the
detectors that read it, and the run that assembles both. [`trps-cli`][cli] is
the command-line front end.

```rust
use trps_core::{LineIndex, Rules};

let text = "Let us delve into this robust ecosystem.\n";

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

`Rules::resolve` applies the nearest `trps.toml` at or above the directory it
is given, so a consumer gets the dictionary the CLI would find. Pass `None` for
the directory to scan under the bundled patterns alone.

The detectors are private to the crate. What a caller needs is re-exported from
the root, and the [API documentation][docs] lists it.

## The rest

The [documentation site][site] covers what is not here:

- scanning files and directories
- writing a project dictionary
- suppressing a finding with an ignore marker
- the JSON report
- which published catalog each rule came from

[cli]: https://crates.io/crates/trps-cli
[docs]: https://docs.rs/trps-core
[site]: https://trps.stormlightlabs.org
