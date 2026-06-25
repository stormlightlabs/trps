# TODO

## Fixtures

- Use `lectito` to turn selected URLs from `meta/examples.txt` into checked-in
  plain text fixtures.

```text
lectito --format text <url> > meta/examples/clean/example.txt
```

- Add clean examples that produce no findings, or only expected low-noise
  findings.
- Add slop examples that produce expected pattern ids.

## Tests

- Add CLI integration tests for file input.
- Add CLI integration tests for stdin input.
- Add CLI integration tests for `NO_COLOR`.
- Add CLI integration tests for exit code `0` on clean input.
- Add CLI integration tests for exit code `1` on matched input.
- Add snapshot-style tests for multiline CLI report formatting.

## Detector Quality

- Tune detector thresholds against checked-in fixtures.
- Consider grouping or suppressing overlapping findings when one text span
  triggers multiple heuristic detectors.
- Consider summarizing repeated Unicode Decoration findings to reduce report noise.
- Consider turning Em-Dash Addiction into a count or density detector instead
  of relying on the ASCII `" -- "` phrase proxy.
