# CLI / Matcher Plan

## Shape

- `meta/tropes.md` is source material only. Do not parse it in the runtime
  pipeline.
- Pattern dictionaries live in `crates/core/src/patterns/*.toml` so contributors
  can add focused rule files without editing one giant dictionary.
- `crates/core` owns:
  - loading all TOML pattern files
  - validating pattern ids and phrases
  - building one Aho-Corasick matcher from all phrase patterns
  - scanning text and returning plain finding structs
- `crates/cli` owns:
  - `clap` argument parsing
  - reading a file or stdin
  - display with the bundled core patterns by default
  - display with `owo_colors`, respecting `NO_COLOR`
  - exit code behavior

## Pattern TOML

Example file: `crates/core/src/patterns/word-choice.toml`

```toml
[[patterns]]
id = "word_choice.delve"
name = "Delve and Friends"
severity = "medium"
phrases = [
  "delve into",
  "delving deeper",
  "certainly",
  "utilize",
  "leverage",
  "robust",
  "streamline",
  "harness",
]
```

Keep the first pass phrase-only. Add structural rules later only when phrase
matching proves insufficient.

## CLI

Initial command:

```text
tropius [FILE]
```

Behavior:

- read stdin when `FILE` is omitted
- print each finding with pattern name, matched phrase, and byte range
- return `0` when no findings are found
- return `1` when findings are found
- return `2` for usage/configuration errors

## Test Bed

Add examples that exercise both obvious matches and normal prose. Keep them as
plain text fixtures so tests do not need network access.

Suggested layout:

```text
meta/examples/
  ai-slop/
  clean/
```

Use the local `lectito` CLI to extract article text into fixtures when useful:

```text
lectito inspect <url> --text > meta/examples/clean/example.txt
```

Do not make tests depend on `lectito` or the network. Use it only to prepare
fixtures that are checked into the repo.

Core tests should cover:

- loading multiple `crates/core/src/patterns/*.toml` files
- detecting phrases across files with one matcher
- case-insensitive matching
- duplicate pattern ids fail validation
- duplicate phrases either fail validation or are reported deterministically
- clean examples produce no findings, or only expected low-noise findings
- slop examples produce expected pattern ids

CLI smoke tests should cover:

- file input
- stdin input
- `NO_COLOR`
- exit code `0` for clean input
- exit code `1` for matched input
