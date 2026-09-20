---
title: Suppressing a finding in place
description: Ignore markers that keep a span out of a report and hold the reason next to the text.
sidebar:
  order: 4
---

Mark a span you have decided to keep in the prose rather than in the
dictionary, so the reason sits next to the text. Quoted slop is the case that
needs it. A document quoting a trope to criticize it is graded for the trope it
quotes, and no dictionary entry tells that apart from the real thing.

`trps-ignore-next-line` drops the findings on the line after it:

```markdown
<!-- trps-ignore-next-line -->
The ecosystem is robust, they wrote, and we left it standing.
```

`trps-ignore-start` and `trps-ignore-end` drop everything between them,
including the two marker lines:

```markdown
<!-- trps-ignore-start -->
> Let us delve into this robust ecosystem.
<!-- trps-ignore-end -->
```

## Naming rules on a marker

Name rule ids after a marker to suppress those alone, separated by spaces or
commas. A marker naming none suppresses every rule on the span:

```markdown
<!-- trps-ignore-next-line word_choice.delve -->
Let us delve into this robust ecosystem, which still reports.
```

An id names the rule it spells, or every rule beneath it when it stops at a
dot: `word_choice` covers `word_choice.delve` and `word_choice.magic_adverbs`,
and `word` covers neither.

An id no rule answers to suppresses nothing, so the CLI warns on stderr and
names the line the marker is on. The warning leaves the exit code alone, and
leaves `--json` writing one document to stdout.

## Writing the reason beside the marker

A `--` ends the list, so the note saying why the span was kept can sit beside
the marker:

```text
trps-ignore-start -- transcribed from the 2019 proposal, quoted verbatim
```

A marker counts anywhere on a line, so every comment syntax carries one and a
plain text file can write the bare word. A region left open runs to the end of
the file.
