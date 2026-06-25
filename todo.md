# CLI / Matcher Plan

## Shape

- `meta/tropes.md` is source material only.
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

## Trope Coverage Checklist

Current phrase coverage: 22 of 33 source sections.
Implemented non-Aho detectors: 1.

- [x] Quietly and Other Magic Adverbs
- [x] Delve and Friends
- [x] Tapestry and Landscape
- [x] The Serves As Dodge
- [x] Negative Parallelism
- [x] Not X. Not Y. Just Z.
- [x] The X? A Y.
- [ ] Anaphora Abuse - structural detector
- [ ] Tricolon Abuse - structural detector
- [x] It's Worth Noting
- [x] Superficial Analyses
- [x] False Ranges
- [ ] Short Punchy Fragments - structural detector
- [ ] Listicle in a Trench Coat - structural detector
- [x] Here's the Kicker
- [x] Think of It As
- [x] Imagine a World Where
- [x] False Vulnerability
- [x] The Truth Is Simple
- [x] Grandiose Stakes Inflation
- [x] Let's Break This Down
- [x] Vague Attributions
- [x] Invented Concept Labels
- [x] Em-Dash Addiction
- [ ] Bold-First Bullets - markdown-aware detector
- [x] Unicode Decoration - character-class detector
- [ ] Fractal Summaries - structural detector
- [ ] The Dead Metaphor - repetition detector
- [ ] Historical Analogy Stacking - structural detector
- [ ] One-Point Dilution - semantic or repetition detector
- [ ] Content Duplication - repetition detector
- [x] The Signposted Conclusion
- [x] Despite Its Challenges

## Non-Aho-Corasick Rules

Aho-Corasick is for literal phrase signals.

Separate rule types are for when the trope depends on structure, repetition,
markdown syntax, or document-level shape.

- Anaphora Abuse: split into sentences and flag repeated sentence starts within
  a short window.
- Tricolon Abuse: detect repeated clause patterns and dense comma/semicolon
  triples, not just fixed phrases.
- Short Punchy Fragments: measure runs of very short sentences or paragraph
  fragments.
- Listicle in a Trench Coat: detect paragraph openings like `The first`, `The
second`, `The third` across adjacent paragraphs.
- Bold-First Bullets: parse markdown list items and flag bullets that start with
  bold text.
- Unicode Decoration: scan for configured Unicode punctuation and symbols.
  Actual Unicode em dashes belong here, not in phrase TOML. Keep ASCII `" -- "`
  as a phrase proxy for typed em-dash style until Em-Dash Addiction gets a count
  or density detector.
- Fractal Summaries: detect repeated summary/conclusion signposts at section
  boundaries.
- The Dead Metaphor: count repeated uncommon nouns or configured metaphor terms
  across a document.
- Historical Analogy Stacking: detect runs of named examples and comparison
  verbs across adjacent sentences.
- One-Point Dilution: likely needs repetition or semantic similarity scoring.
- Content Duplication: compare normalized paragraphs or sentence shingles.

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

Use the `lectito` CLI to extract article text into fixtures when useful:

```text
lectito inspect <url> --text > meta/examples/clean/example.txt
```

### Unit Tests

- loading multiple `crates/core/src/patterns/*.toml` files
- detecting phrases across files with one matcher
- case-insensitive matching
- duplicate pattern ids fail validation
- duplicate phrases fail validation
- empty pattern ids, names, phrase lists, and phrases fail validation
- clean examples produce no findings, or only expected low-noise findings
- slop examples produce expected pattern ids

### Integration Tests

- file input
- stdin input
- `NO_COLOR`
- exit code `0` for clean input
- exit code `1` for matched input
