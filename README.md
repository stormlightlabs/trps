# tropius

`tropius` is a CLI to detect AI tropes in prose.

## How

We use a pattern dictionary in TOML, drawn from the published trope catalogs
listed in [`sources.md`](./meta/sources.md). Every pattern names the catalogs
its phrases came from, and a bundled pattern that cites one the crate does not
know fails to load.

```text
Text
  ↓
Aho-Corasick phrase matcher
  ↓
Structural, repetition, and character-class detectors
  ↓
Findings report
```

## Quick start

```sh
printf 'Let us delve into this robust ecosystem.' | cargo run -q -p tropius-cli
```

See [`docs/src/content/docs/reference/`](./docs/src/content/docs/reference/)
for the rest: scanning files, the JSON report, the project dictionary, and the
ignore markers.

## Coverage

- an implementation path for every source section in
  [`tropes.md`](https://tropes.fyi/tropes-md)
- phrase patterns for literal trope signals
- chat-assistant residue: narrated next steps, cutoff disclaimers, and the
  tracking parameters and citation markup a chat tool leaves in pasted text
- prose about code: comments that rate it, code given intentions, docstrings
  that restate the signature, and reasons that give no reason
- phrasings measured as over-represented in generated fiction
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
