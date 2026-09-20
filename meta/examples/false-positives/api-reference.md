# Record separator

Note that the parser is aware of the trailing newline: a record without one is
a truncated write rather than a short record, and the reader rejects it.

For clarity, the helper function below reports the offset of the last complete
record, not the length of the buffer it read.
