# Retry helper

This function returns the parsed span. It is a thin wrapper around the
parser, split out for better readability.

The parser is aware of the trailing newline and will complain without one.
The worker retries twice for various reasons, and the second attempt waits
longer. This is by design.

Loop through the results, then return the result the caller asked for. Best
practices say to keep the cast where it is.
