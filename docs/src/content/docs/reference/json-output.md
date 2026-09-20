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
`phrase`, `char`, `struct`, `repeat`, or `markdown`. `path` is the file as you
named it on the command line, and `-` for text read from stdin. `line` and
`column` are 1-based, and a column counts characters rather than bytes.
`matched` is the text that matched, except where a rule counts occurrences:
`formatting.unicode_decoration` lists the characters it counted, such as
`— — —`.
