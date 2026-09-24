---
title: Sources
description: The published catalogs the bundled rules come from, and what each one contributed.
sidebar:
  order: 5
---

Every bundled pattern names the catalogs its phrases came from, in the
dictionary beside the phrases:

```toml
[[patterns]]
id = "composition.era_framing"
name = "In Today's Fast-Paced Era"
severity = "medium"
sources = ["clearmode"]
phrases = ["in today's fast-paced", "in the age of"]
```

So a rule you want to argue with can be traced to whoever proposed it. A key
nothing registers fails the load. The seven the tool registers are the
sections below, and a project registers its own catalogs in a `[sources]`
table, which the [project
dictionary](/reference/project-dictionary/#citing-a-source) has. Rules written
in Rust instead of TOML, covering sentence and paragraph shape, repetition,
bold-first leads, and Unicode decoration, all come from Tropes.fyi.

## tropes.fyi

[Tropes.fyi](https://tropes.fyi) by [ossama.is](https://ossama.is) is where the
project started, and its list is copied into `meta/tropes.md`. It reads
published prose: word choice, tone, sentence structure, and the shape of a
paragraph. The 22 patterns in the five original dictionaries cite it.

## humanizer

[blader/humanizer](https://github.com/blader/humanizer) is a rewriting skill
built on Wikipedia's
[Signs of AI writing](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing).
Its section on vague connection, where a sentence says two things are related
without saying how, is `composition.vague_connection`. It is a second citation
on chat residue, cutoff disclaimers, stacked qualifiers, and travel-brochure
prose.

## avoid-ai-writing

[conorbronsdon/avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing)
is a skill with an executable detector behind it, at 53 pattern types the
largest of the seven catalogs. Four rules come from it, and three of those
catch marks a chat tool leaves in text somebody pasted: tracking parameters on
a URL, citation markup, and placeholders nobody filled in. Those hold whatever
the writing around them reads like. Its cutoff disclaimers and engagement bait
are the other two citations.

## clearmode

[eugeniughelbur/clearmode](https://github.com/eugeniughelbur/clearmode)
publishes the CLEAR-100 lexicons, which sort slop by kind: words, phrases,
sentence openers, assistant voice, promotional adjectives, and filler.
`composition.era_framing` and `tone.promotional` come from two of those lists,
and its slop words carry part of `word_choice.lexical_spikes`.

## vale-llm-slop

[Syntaf/vale-llm-slop](https://github.com/Syntaf/vale-llm-slop) reads prose
that gets committed instead of published: pull request bodies, commit messages,
comments, and docstrings. Nine rules cite it, more than any other catalog. The
four `technical.*` rules are its work, and they read comments that rate the
code, code given intentions, prose that restates a signature, and reasons that
give no reason. So are three of the `assistant.*` rules,
`composition.throat_clearing`, and `sentence_structure.hedge_stack`.

## vale-ai-slop

The [`research/ai-slop`](https://github.com/stuffbucket/vale/tree/main/research/ai-slop)
notes in [stuffbucket/vale](https://github.com/stuffbucket/vale), a Simplified
Technical English linter, rank candidate rules by the papers behind them and
record a false-positive risk against each. Two of its tier-one rules are
`sentence_structure.impersonal_hedge` and `composition.restatement_markers`.
Its watchlist of words whose use spiked after 2022 overlaps clearmode's slop
list, and `word_choice.lexical_spikes` takes the seven words on both lists plus
seven more from one or the other.

## slop-forensics

[sam-paech/slop-forensics](https://github.com/sam-paech/slop-forensics) counts
which words and phrases a model over-produces, so its lists are measured rather
than noticed. The two `narrative.*` rules come from its bigram and trigram
lists, which were built from generated fiction. Any one of those phrases is
ordinary in a novel, so both rules are graded `low` and say something only
where several land in one passage.

## Licenses

The six catalogs on GitHub are MIT licensed, and `meta/sources.md` in the
repository holds each one's copyright notice and the commit its lists were read
at. Tropes.fyi states no license; it publishes its list as a Markdown file to
copy into a prompt, and `meta/tropes.md` keeps the attribution line that file
carries.
