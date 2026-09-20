---
title: Config files
updated: 2026-09-19
---

# Config files

The loader reads `~/.config/trps/trps.toml` first, then `.trps.toml` beside the
repository root. Version `v0.1.1` renamed the second path, and the old name is
read for one more release.

| Key         | Meaning                                       |
| ----------- | --------------------------------------------- |
| `dictionary`| Path to the pattern file the run applies.     |
| `exclude`   | Globs the walker skips before it reads them.  |
| `severity`  | Lowest severity the report prints.            |

Three keys share a prefix, and each one takes a different kind of value.

- The loader reads the global file before it reads the project one.
- The loader merges the two, and the project file wins a conflict.
- The loader records which file each value came from.

Run it against a file to see the shape of the report:

```text
⚠ medium paragraph_structure.short_punchy_fragments
  └─ ** Run the thing. Openly. In a file. As a key.
```

A key the loader does not know is an error rather than a warning, because a
misspelled key the loader drops without a word reads as a setting that had
no effect.

**Severity** is the one key with a default. A run that sets nothing prints
every finding, since a report hiding its own low findings would read as a
file with none.
