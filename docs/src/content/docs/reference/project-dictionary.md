---
title: Project dictionary
description: Adjust the bundled patterns for one repository with a trps.toml, tune the counts a rule fires at, and keep paths out of a scan.
sidebar:
  order: 3
---

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

A project pattern cites its sources the way a bundled one does, once the
project has registered them. [Citing a source](#citing-a-source) has the
shape.

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

## Allowing and redeclaring phrases

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

## Choosing a dialect

A corpus drifts between `judgment` and `judgement` unless something holds it to
one. `dialect` names the side the project writes on, and turns on
`word_choice.dialect_spelling`:

```toml
dialect = "american"
```

`american` and `british` are the two values, and neither is the default. The
rule stays off while the key is unset, because a tool that picked a dialect for
you would report half of a British corpus as wrong.

A finding names the spelling it found and the one the dialect uses, in the case
you wrote it:

```
⚠ medium spelling word_choice.dialect_spelling
  ├─ docs/guide.md:1:5-13
  └─ judgement → judgment
```

The word list covers the `-or`/`-our`, `-ize`/`-ise`, `-er`/`-re` and
`-se`/`-ce` families, the doubled consonants of `travelled` and `enrolment`,
and the irregulars that follow no pattern. Nothing is stemmed: `colours` is
reported because it is listed, and a word with no entry is not reported.

A pair is listed only where both spellings are unambiguous, since the rule
reads them in both directions. British English uses `program` for software,
and American English spells `license` and `practice` one way for both the noun
and the verb, so none of the three is listed. British here is the `-ise`
convention rather than the `-ize` spelling Oxford keeps.

Spelling inside a fenced block or front matter is never graded, and a single
finding can be suppressed the way any other is. [Suppressing
findings](/reference/suppressing-findings/) has the markers.

## Tuning rule counts

A rule that fires at a count reads it from `[thresholds]`, keyed by the rule id
the finding prints. A project that finds one too strict raises it instead of
silencing the rule.

`composition.dead_metaphor` counts the word and not the meaning, so it reports
`primitive` at five uses even in a parser's documentation, where the word means
the smallest type a grammar has. Raising the count to twelve gives that
documentation room:

```toml
[thresholds."composition.dead_metaphor"]
min_repeats = 12
```

A count belongs to its rule, not to the term or pattern that tripped it. The
rule still counts each of its terms on its own, but holds every one of them to
that same number, so the twelve above also stops reporting `wall` at seven
uses and `door` at eight. Raise a count to the lowest number that quiets what
you meant to quiet.

A rule id holds a dot, so `[thresholds."composition.dead_metaphor"]` and
`[thresholds.composition.dead_metaphor]` reach the same entry.

A key naming no rule warns on stderr, names the dictionary it read, and leaves
the load and the exit code alone. The fields inside a rule's entry are fixed,
so `min_repeat` under `[thresholds."composition.dead_metaphor"]` fails the
load rather than warning.

### Every count and its floor

| Rule                                          | Key                      | Default | Floor |
| --------------------------------------------- | ------------------------ | ------- | ----- |
| `composition.content_duplication`             | `min_length`             | 40      | 1     |
| `composition.cross_file_duplication`          | `min_words`              | 6       | 2     |
| `composition.cross_file_duplication`          | `min_files`              | 2       | 2     |
| `composition.dead_metaphor`                   | `min_repeats`            | 5       | 2     |
| `composition.fractal_summaries`               | `min_openings`           | 3       | 2     |
| `composition.historical_analogy_stacking`     | `min_sentences`          | 3       | 2     |
| `composition.one_point_dilution`              | `min_shared_terms`       | 3       | 2     |
| `formatting.bold_first_leads`                 | `min_leads`              | 3       | 2     |
| `formatting.em_dash_addiction`                | `floor`                  | 3       | 2     |
| `formatting.em_dash_addiction`                | `count`                  | 6       | 2     |
| `formatting.em_dash_addiction`                | `rate_per_hundred_words` | 2       | 1     |
| `formatting.unicode_decoration`               | `min_occurrences`        | 3       | 2     |
| `paragraph_structure.listicle_in_trench_coat` | `min_paragraphs`         | 3       | 2     |
| `paragraph_structure.short_punchy_fragments`  | `min_sentences`          | 3       | 2     |
| `paragraph_structure.short_punchy_fragments`  | `max_words`              | 4       | 1     |
| `sentence_structure.anaphora_abuse`           | `min_sentences`          | 3       | 2     |
| `sentence_structure.tricolon_abuse`           | `min_separators`         | 2       | 2     |
| `sentence_structure.tricolon_abuse`           | `min_repeated_starts`    | 2       | 1     |

Five of the keys count something the name does not say. In
`formatting.em_dash_addiction`, `floor` is the number of dashes below which the
rule never reports, `count` is the number that reports however long the
document is, and `rate_per_hundred_words` is the density that reports a
document too short to reach `count`.

In `composition.content_duplication`, `min_length` is characters, measured over
the words a passage normalizes to.

In `composition.cross_file_duplication`, `min_files` is the files a shared run
appears in rather than the times it appears, so a run used twice in one file
and once in another is in two files.

A number below a key's floor is read as the floor.

## Excluding paths

Some prose is never worth grading: research captured from elsewhere, imported
reference pages, anything written to a house format the bundled patterns read
as slop. `exclude` names those paths in the same file.

```toml
exclude = ["docs/notebook/", "meta/examples/**"]
```

Patterns are globs matched against the path relative to the directory holding
the dictionary. `*` stops at a `/` and `**` crosses one. A pattern ending in
`/` is the directory and everything under it.

An excluded path is skipped before it is read, so naming one on the command
line reports nothing and is not an error. A path outside the dictionary's
directory is never excluded.

## Citing a source

A pattern says where its phrases came from in a `sources` list. The keys it
may use are the seven [the tool registers](/reference/sources/) and whatever
the project registers in a `[sources]` table:

```toml
[sources]
house-style = "https://wiki.corp.example/style"

[[patterns]]
id = "house.synergize"
name = "Synergize"
severity = "medium"
sources = ["house-style"]
phrases = ["synergize"]
```

A key neither side registers fails the load, which is the error a bundled
pattern gets for the same mistake. A pattern citing nothing still loads: where
a project's phrases came from is the project's business, and what it says
about them has to be true.

## Markers in Markdown code

A page documenting the [ignore markers](/reference/suppressing-findings/)
writes them in code, and a marker in an inline span is an example rather than
an instruction. Markdown is read that way by default. A repository generating
Markdown whose spans carry real markers turns it off:

```toml
[markdown]
skip_markers_in_code = false
```

A marker in a fenced block is never one either way, whatever this says.
Nothing inside a fence is graded, so a marker there can only open a region
over the prose after it, which is the silent suppression worth stopping. An
inline span sits in prose that is graded, which is why the two differ.

Markdown is decided by the extension: `.md`, `.markdown`, and `.mdx`. Plain
text is read the other way, because a backtick in arbitrary prose says nothing
about code.

A span closes on the same line it opened on. Markdown lets one cross a line,
and reading it that way would let a stray backtick pair with the next real
span and swallow every marker between the two.

## Reading what the dictionary did

A dictionary that does nothing scans like one that works. A file above the
repository root is never found, and an allowlist entry matching no phrase
takes nothing out of anything. `--config` reports what a run resolved, having
scanned nothing:

```text
$ trps --config
dictionary: /home/you/project/trps.toml
patterns: 43

declared patterns:
  word_choice.delve replaces the bundled pattern
  house.synergize is new

allowed phrases:
  `framework` left word_choice.grandiose_nouns
  `moreovr` matches no bundled phrase

excluded paths:
  docs/notebook/**

registered sources:
  house-style https://wiki.corp.example/style
```

A section with nothing under it is left out, and a run that found no
dictionary says `dictionary: none found`. `--json` writes the same report as
one document, so a build can diff it; [JSON
output](/reference/json-output/#the-configuration-report) has the shape.
