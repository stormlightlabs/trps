---
title: Project dictionary
description: Adjust the bundled patterns for one repository with a trps.toml, and keep paths out of a scan.
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

A project pattern may carry a `sources` key, the way a bundled one does, but
nothing checks what it says: the citation is yours, and the registry of keys a
bundled pattern may cite is ours. [Sources](/reference/sources/) lists that
registry.

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
⚠ medium word_choice.dialect_spelling
  ├─ spelling docs/guide.md:1:5-13
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

## Tuning what repeats across files

`composition.cross_file_duplication` reports the wording a run's files share.
[Usage](/reference/usage/) says what it reads; here is what a project sets. Six
words in two files is the default, and a project that reuses more of its own
wording than that raises either number:

```toml
[cross_file]
min_words = 10
min_files = 3
```

`min_words` counts the words of a shared run as the comparison reads them.
`min_files` counts the files a run has to appear in, not the times it appears:
a run used twice in one file and once in another is in two files. Two is the
floor for both, and a smaller number is read as two.

Raise `min_words` when what gets reported is house phrasing you mean to keep.
Raise `min_files` when a pair of documents is expected to overlap and a third
would be the signal.

## Excluding paths

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
