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

The noun passes and the verb still reports. A phrase a bundled pattern already
carries has to be allowed out first, or loading fails on the duplicate.

Declared patterns are matched before the bundled ones, so a project phrase wins
where the two overlap: a rule for `landscape architecture` reports that span
rather than losing it to the bundled `landscape`.

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
