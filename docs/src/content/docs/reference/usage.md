---
title: Usage
description: Scan stdin, files, or an article pulled from a URL, and read what the run reports.
sidebar:
  order: 1
---

Scan text from stdin:

```sh
printf 'Let us delve into this robust ecosystem.' | cargo run -q -p trps-cli
```

Scan files, as many as you have:

```sh
cargo run -q -p trps-cli -- README.md docs/guide.md
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
    | cargo run -q -p trps-cli
```

## What a scan reads

A scan grades the prose of a file and skips the parts nobody wrote as prose:

- Fenced code blocks, and everything between the fences. A block holds sample
  output, a command, or a quoted defect, so grading it reports the quoted text
  rather than the writing around it.
- YAML front matter, delimiters included.

The rules that count sentences next to each other read one paragraph at a
time, and a heading, a table row, a block quote, and a list item are not part
of one. Three bullets opening with the same two words are a list rather than
anaphora, and a lead-in line above a list is not a run of fragments with the
list under it.

A sentence ends at a `.`, `!`, or `?` with whitespace or the end of the file
after it. A terminator inside a token does not end one, so `src/lib.rs` and
`v0.1.1` are read as single words.

## Exit codes

The CLI exits `0` when it finds nothing and `1` when it reports a trope
signal. It exits `2` for a usage or configuration error.

## Color

Color output respects [`NO_COLOR`](https://no-color.org/).
