# Sources

Every bundled pattern names where its phrases came from in its `sources` key,
and the keys below are the ones it may name. `crates/core/src/patterns.rs`
carries the same list as the `SOURCES` registry, and loading fails on a bundled
pattern that cites a key nobody registered. To find the rules that cite one:

```sh
grep -rl <key> crates/core/src/patterns
```

Every catalog here is MIT licensed, so each one's copyright line is kept below.
The commit is the one its lists were read at; all of them keep changing.

## tropes.fyi

[Tropes.fyi](https://tropes.fyi) by [ossama.is](https://ossama.is), copied to
[`tropes.md`](./tropes.md). It is where the project started. The structural,
repetition, markdown, and character-class detectors under
`crates/core/src/detector/` come from it alone, and so do the patterns in the
five dictionaries that predate the others below.

## humanizer

[blader/humanizer](https://github.com/blader/humanizer) at `9862685`,
copyright (c) 2025 Siqi Chen. A rewriting skill drawn from Wikipedia's
[Signs of AI writing](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing).
Its section 14 is the whole of `composition.vague_connection`, and its lists
are second citations on chat residue, cutoff disclaimers, stacked qualifiers,
and travel-brochure prose.

## avoid-ai-writing

[conorbronsdon/avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing)
at `c478346`, copyright (c) 2026 Conor Bronsdon. A skill with an executable
detector behind it, at 53 pattern types the largest catalog of the six. What we
took from it is the part no prose rule covers: the fingerprints a chat tool
leaves in pasted text, meaning tracking parameters, citation markup, and
placeholders nobody filled in, plus its cutoff disclaimers and engagement bait.

## clearmode

[eugeniughelbur/clearmode](https://github.com/eugeniughelbur/clearmode) at
`8f6b1af`, copyright (c) 2026 Eugeniu Ghelbur. The CLEAR-100 lexicons, which
sort slop by kind: words, phrases, sentence openers, assistant voice,
promotional adjectives, filler. `composition.era_framing` and
`tone.promotional` come from two of those lists, and its slop words carry half
of `word_choice.lexical_spikes`.

## vale-llm-slop

[Syntaf/vale-llm-slop](https://github.com/Syntaf/vale-llm-slop) at `4dda3ec`,
copyright (c) 2026 Grant Mercer. Vale styles written for committed prose rather
than for publishing: pull request bodies, commit messages, comments, and
docstrings. It is the source of every `technical.*` and `assistant.*` rule but
the two above, and of `composition.throat_clearing`.

## vale-ai-slop

[stuffbucket/vale, `research/ai-slop`](https://github.com/stuffbucket/vale/tree/main/research/ai-slop)
at `d96df3e`, copyright (c) 2026 stuffbucket. Candidate rules grounded in cited
papers rather than in taste, with a false-positive risk recorded against each.
`sentence_structure.impersonal_hedge` and `composition.restatement_markers` are
two of its tier-one rules, and its watchlist of words that spiked after 2022 is
the rest of `word_choice.lexical_spikes`.

## slop-forensics

[sam-paech/slop-forensics](https://github.com/sam-paech/slop-forensics) at
`c313f04`, copyright (c) 2025 Sam Paech. A toolkit that measures which words
and phrases a model over-produces, rather than a list somebody wrote. The
`narrative.*` rules come from its bigram and trigram lists, built from
generated fiction. Any one of those phrases is ordinary in a novel, so both
rules are graded low and mean something only where several land together.
