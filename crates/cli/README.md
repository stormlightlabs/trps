# trps-cli

`trps` detects AI writing tropes in prose. It reads Markdown and plain text,
reports what it finds by line and column, and exits non-zero when it finds
something, so it can gate a pull request.

```sh
cargo install trps-cli
```

## Scanning

With no paths, `trps` reads stdin:

```sh
printf 'Let us delve into this robust ecosystem.\n' | trps
```

Otherwise it scans the files it is named:

```sh
trps README.md docs/guide.md
```

| Flag | What it does |
| --- | --- |
| `--dictionary <PATH>` | Applies this project dictionary instead of searching for one. |
| `--json` | Writes the report as JSON on stdout. |
| `-q`, `--quiet` | Drops the line a clean run writes to stderr. |

Without `--dictionary`, `trps` applies the nearest `trps.toml`, `tropes.toml`,
or `tropius.toml` at or above the directory it was started in.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Nothing found. |
| 1 | Findings reported. |
| 2 | A path could not be read, or a dictionary failed to load. |

## The rest

The [documentation site][site] covers what is not here:

- scanning files and directories
- writing a project dictionary
- suppressing a finding with an ignore marker
- the JSON report
- which published catalog each rule came from

[site]: https://trps.stormlightlabs.org
