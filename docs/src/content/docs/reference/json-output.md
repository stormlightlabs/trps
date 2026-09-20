---
title: JSON output
description: The shape of the document --json writes to stdout, field by field.
sidebar:
  order: 2
---

`--json` writes the findings to stdout as one JSON document instead of the
decorated report.

```json
{
  "version": 1,
  "dictionary": "/home/you/project/trps.toml",
  "findings": [
    {
      "rule_id": "word_choice.delve",
      "rule_name": "Delve and Friends",
      "severity": "medium",
      "kind": "phrase",
      "path": "docs/guide.md",
      "line": 2,
      "column": 15,
      "matched": "delve into"
    }
  ]
}
```

`version` is `1`, and rises when the shape changes enough to break something
reading it. `findings` is always present, and is empty on a clean run.

`dictionary` is the project dictionary the run applied, or `null` when it
applied none. A dictionary found by the search is reported as an absolute path;
`--dictionary` is reported as you wrote it.

`severity` is `low`, `medium`, or `high`. `kind` names the detector that fired:
`phrase`, `char`, `struct`, `repeat`, `markdown`, or `spelling`. `path` is the file as you
named it on the command line, and `-` for text read from stdin. `line` and
`column` are 1-based, and a column counts characters rather than bytes.
`matched` is the text that matched, except where a rule counts occurrences:
`formatting.unicode_decoration` lists the characters it counted, such as
`— — —`.

`expected` is there only on `word_choice.dialect_spelling`, which knows the
word the project's dialect uses and writes it in the case the text spelled it
in. Every other rule reports what it found and leaves the rewrite to you, so
the field is absent rather than null.

## Findings that cross files

`cross_file` carries what several of the scanned files share. A run is one
entry however many files it appears in, and `occurrences` names each place:

```json
{
  "cross_file": [
    {
      "rule_id": "composition.cross_file_duplication",
      "rule_name": "Cross-File Duplication",
      "severity": "medium",
      "kind": "repeat",
      "matched": "rather than left to be discovered",
      "occurrences": [
        { "path": "docs/one.md", "line": 12, "column": 22 },
        { "path": "docs/two.md", "line": 40, "column": 28 }
      ]
    }
  ]
}
```

`matched` is the run as the first file writes it; the others differ from it
only in case and punctuation. The key is absent when a run found nothing to
report, which is every run over a single path.
