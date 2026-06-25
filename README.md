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
