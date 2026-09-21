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

## One passage, one entry

A passage can trip more than one rule. An entry names every rule that fired,
its severity and the detector behind it, then quotes the passage once:

```
⚠ medium struct sentence_structure.anaphora_abuse
⚠ medium struct paragraph_structure.short_punchy_fragments
  ├─ draft.md:1:1-55
  └─ We built it fast. We built it wrong. We built it twice.
```

Rules share an entry only where they cover the same span. A rule that fires
inside a longer finding keeps an entry of its own, because a paragraph rule
covers every phrase under it and folding those away would hide the rules that
say the most about the text.

`--json` lists each finding on its own, whatever it shares a span with. See
[JSON output](/reference/json-output/).

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

## What several paths share

A run over more than one path also compares the files against each other and
reports the word runs they share. A phrase used once reads as voice, and the
same phrase closing three neighboring documents is a tic neither file can see
from the inside.

Such a finding names every place the run appears, because whether a repetition
is a house convention or a tic is yours to decide:

```
⚠ medium repeat composition.cross_file_duplication
  ├─ docs/one.md:12:22-54
  ├─ docs/two.md:40:28-60
  └─ rather than left to be discovered
```

Case and punctuation do not count against a match, and a link counts as one
word however long it is. The rule is quiet on short runs, so shared vocabulary
stays out of the report. [Tuning what a rule
counts](/reference/project-dictionary/) has the two counts it reads and when to
raise them.

## A clean run

A run that matches nothing says so on stderr:

```text
clean: nothing in the catalogue matched. Hedges, filler adverbs, and editorial asides are not in it.
```

A run that printed nothing would read as a verdict on the prose. Tropius
matches a catalogue of tropes, and most of the ways writing goes wrong are
outside it, so read the text yourself.

The line goes to stderr, so a `--json` pipeline reading stdout never sees it.
Pass `-q` or `--quiet` to drop it.

## Exit codes

The CLI exits `0` when it finds nothing and `1` when it reports a trope
signal. It exits `2` for a usage or configuration error.

## Color

Color output respects [`NO_COLOR`](https://no-color.org/).
