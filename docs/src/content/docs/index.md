---
title: tropius
description: A CLI that detects AI tropes in prose.
---

`tropius` reads prose and reports the phrases, rhythms, and punctuation habits
that mark text as machine-written. This site is its usage documentation; the
[README](https://github.com/stormlightlabs/trps) says what the tool is and how
it works.

Five pages:

- [Usage](/reference/usage/) scans stdin, files, and an article pulled from a
  URL, and says what the exit codes mean.
- [JSON output](/reference/json-output/) gives the shape of the document
  `--json` writes.
- [Project dictionary](/reference/project-dictionary/) adjusts the bundled
  patterns for one repository and keeps paths out of a scan.
- [Suppressing a finding in place](/reference/suppressing-findings/) marks a
  span to leave alone and records why.
- [Sources](/reference/sources/) names the published catalogs the rules come
  from and what each one contributed.
