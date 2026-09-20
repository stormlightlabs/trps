# tropius

`tropius` is a CLI to detect AI tropes in prose.

## How

We use a pattern dictionary in TOML based on a [trope list](./meta/tropes.md)
from [Tropes.fyi](https://tropes.fyi)

```text
Text
  ↓
Aho-Corasick phrase matcher
  ↓
Structural, repetition, and character-class detectors
  ↓
Findings report
```

## Usage

Scan text from stdin:

```sh
printf 'Let us delve into this robust ecosystem.' | cargo run -q -p tropius-cli
```

Scan files, as many as you have:

```sh
cargo run -q -p tropius-cli -- README.md docs/guide.md
```

Every finding is located as `path:line:column-column`, so a run over several
files needs no heading to say which one it is reading. Text from stdin has no
path and reports the line and column alone. Columns are 1-based, count
characters rather than bytes, and both ends are inclusive.

Scan article text extracted from a live URL with
[lectito](https://lectito.stormlightlabs.org/):

```sh
# Install lectito
cargo install lectito-cli

lectito 'https://www.solo.io/blog/what-is-agent-identity-human-workload-a-new-layer' \
    --format text \
    | cargo run -q -p tropius-cli
```

The CLI exits `0` when no findings are found and `1` when it finds trope signals.

It exits `2` for usage or configuration errors.

Color output respects [`NO_COLOR`](https://no-color.org/).

## JSON output

`--json` writes the findings to stdout as one JSON document instead of the
decorated report.

```json
{
  "version": 1,
  "dictionary": "/home/you/project/trps.toml",
  "findings": [
    {
      "rule_id": "word_choice.delve",
      "rule_name": "Delve and Friends",
      "severity": "medium",
      "kind": "phrase",
      "path": "docs/guide.md",
      "line": 2,
      "column": 15,
      "matched": "delve into"
    }
  ]
}
```

`version` is `1`, and rises when the shape changes enough to break something
reading it. `findings` is always present, and is empty on a clean run.

`dictionary` is the project dictionary the run applied, or `null` when it
applied none. A dictionary found by the search is reported as an absolute path;
`--dictionary` is reported as you wrote it.

`severity` is `low`, `medium`, or `high`. `kind` names the detector that fired:
`phrase`, `char`, `struct`, `repeat`, or `markdown`. `path` is the file as you
named it on the command line, and `-` for text read from stdin. `line` and
`column` are 1-based, and a column counts characters rather than bytes.

## Project dictionary

The bundled patterns are tuned for general prose, so a repository whose domain
vocabulary collides with them can adjust the dictionary instead of forking it.
Write a `trps.toml` at the root of the project:

```toml
# Phrases that leave every bundled pattern, matched case-insensitively.
allow = ["harness", "framework"]

# Patterns added in the same shape the bundled files use.
[[patterns]]
id = "project.bounded"
name = "Bounded Without a Bound"
severity = "high"
phrases = ["bounded"]
```

The CLI looks for that file in the working directory and its ancestors, taking
the nearest one it finds and stopping at the repository root, so a dictionary
outside the project never reaches a scan inside it. Outside a repository only
the working directory is searched. `trps.toml`, `tropes.toml`, and
`tropius.toml` all work, and are searched in that order. `--dictionary <path>`
names a file directly and skips the search.

One run uses one dictionary for every path it scans, and the search starts from
the working directory. Scanning a file kept in another project still reports it
under this project's rules, so run the CLI from the root of the project whose
rules you want.

Allowing a phrase removes it from the pattern that carried it and leaves the
rest of that pattern in place: `allow = ["harness"]` stops the `harness`
findings without disabling the other phrases in `word_choice.delve`. A pattern
whose phrases are all allowed drops out entirely, and a pattern declared with
the id of a bundled one replaces it.

To keep a phrase but grade it differently, allow it out and declare it again:

```toml
allow = ["harness"]

[[patterns]]
id = "project.harness"
name = "Harness the Verb"
severity = "low"
phrases = ["harness the", "harnessing"]
```

The noun passes and the verb still reports. A phrase carried by a bundled
pattern you are not replacing has to be allowed out first, or loading fails on
the duplicate. Redeclaring the pattern that carries it needs no allowlist: the
bundled entry goes with the id.

Declared patterns are matched before the bundled ones, so a project phrase wins
where the two overlap: a rule for `landscape architecture` reports that span
rather than losing it to the bundled `landscape`.

### Excluding paths

Some prose is never worth grading: research captured from elsewhere, imported
reference pages, anything written to a house format the bundled patterns read
as slop. `exclude` names those paths in the same file.

```toml
exclude = ["docs/notebook/", "meta/examples/**"]
```

Patterns are globs matched against the path relative to the directory holding
the dictionary. `*` stops at a `/` and `**` crosses one, so `docs/*.md` takes
the Markdown directly under `docs` and `docs/**/*.md` takes it at any depth. A
pattern ending in `/` is the directory and everything under it.

An excluded path is skipped before it is read, so naming one on the command
line reports nothing and is not an error. A path outside the dictionary's
directory is never excluded: the list belongs to one repository and says
nothing about a file kept somewhere else.

## Suppressing a finding in place

Mark a span you have decided to keep in the prose rather than in the
dictionary, so the reason sits next to the text. Quoted slop is the case that
needs it. A document quoting a trope to criticize it is graded for the trope it
quotes, and no dictionary entry tells that apart from the real thing.

`trps-ignore-next-line` drops the findings on the line after it:

```markdown
<!-- trps-ignore-next-line -->
The ecosystem is robust, they wrote, and we left it standing.
```

`trps-ignore-start` and `trps-ignore-end` drop everything between them,
including the two marker lines:

```markdown
<!-- trps-ignore-start -->
> Let us delve into this robust ecosystem.
<!-- trps-ignore-end -->
```

Name rule ids after a marker to suppress those alone, separated by spaces or
commas. A marker naming none suppresses every rule on the span:

```markdown
<!-- trps-ignore-next-line word_choice.delve -->
Let us delve into the em dash — which still reports.
```

An id names the rule it spells, or every rule beneath it when it stops at a
dot: `word_choice` covers `word_choice.delve` and `word_choice.magic_adverbs`,
and `word` covers neither.

An id no rule answers to suppresses nothing, so the CLI warns on stderr and
names the line the marker is on. The warning leaves the exit code alone, and
leaves `--json` writing one document to stdout.

A `--` ends the list, so the note saying why the span was kept can sit beside
the marker:

```text
trps-ignore-start -- transcribed from the 2019 proposal, quoted verbatim
```

A marker counts anywhere on a line, so every comment syntax carries one and a
plain text file can write the bare word. A region left open runs to the end of
the file.

## Coverage

- an implementation path for every source section in
  [`tropes.md`](https://tropes.fyi/tropes-md)
- phrase patterns for literal trope signals
- structural detectors for sentence and paragraph shape
- repetition detectors for repeated metaphor terms and duplicated content
- markdown-aware detection for bold-first bullets
- character-class detection for Unicode decoration

## Inspiration

I got nerd-sniped on [BlueSky](https://bsky.app/profile/samuel.fm/post/3mp3l3cxg622y)

> [Samuel - @samuel.fm](https://bsky.app/profile/samuel.fm) **2026-06-25 04:16**
>
> “Claudesmell” labeller for [standard.site](https://standard.site) records wen

> [owais - @desertthunder.dev](https://bsky.app/profile/desertthunder.dev) **2026-06-25 04:18**
>
> what does claudesmell mean here

> [Samuel - @samuel.fm](https://bsky.app/profile/samuel.fm) **2026-06-25 04:22**
>
> smells of claude, e.g. short, punchy yet needlessly florid prose, excessive claudeisms,
> it’s not just x it’s y etc

> [owais - @desertthunder.dev](https://bsky.app/profile/desertthunder.dev) **2026-06-25 04:26**
>
> You know that feeling of nerd-sniping about to happen? I gotta use aho-corasick +
> [tropes.fyi](https://tropes.fyi) in some way

## Further Reading

[Wikipedia Article](https://en.wikipedia.org/wiki/Wikipedia%3ASigns_of_AI_writing)

[Directory of tropes](https://tropes.fyi/directory)

## License

[unlicense](./LICENSE)
