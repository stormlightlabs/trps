# Sources

A bundled pattern cites the catalogs its phrases came from in its `sources`
key, and `SOURCES` in `crates/core/src/patterns.rs` is the list of keys it may
use. `bundled_patterns` rejects a pattern citing anything else, so every
citation resolves to a catalog somebody can open. A project dictionary may
carry the key as well, and nothing checks what it says there.

The [Sources](https://trps.stormlightlabs.org/reference/sources/) page
describes each catalog and what it contributed. To see which rules cite one:

```sh
grep -rl <key> crates/core/src/patterns
```

## Copyright

The six projects on GitHub are MIT licensed, which asks that their copyright
notices travel with the phrases taken from them. The commit named is the one
whose lists were read; all six keep changing.

| Key | Copyright | Read at |
| --- | --- | --- |
| `humanizer` | (c) 2025 Siqi Chen | `9862685` |
| `avoid-ai-writing` | (c) 2026 Conor Bronsdon | `c478346` |
| `clearmode` | (c) 2026 Eugeniu Ghelbur | `8f6b1af` |
| `vale-llm-slop` | (c) 2026 Grant Mercer | `4dda3ec` |
| `vale-ai-slop` | (c) 2026 stuffbucket | `d96df3e` |
| `slop-forensics` | (c) 2025 Sam Paech | `c313f04` |

The seventh, `tropes.fyi`, states no license. The site publishes its list as a
Markdown file to copy into a prompt, and that file carries one attribution
line, which [`tropes.md`](./tropes.md) keeps: "Source: tropes.fyi by
ossama.is". Read 2026-09-19.
