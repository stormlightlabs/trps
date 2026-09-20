# Retry behaviour

The worker retries a failed job twice, waiting one second and then four. A
third failure moves the job to the dead letter queue, where it stays until an
operator releases it.

The parser errors when a record has no trailing newline. That rule comes from
the wire format, which reserves the newline as the record separator, so a
truncated write is detected rather than parsed.

Both numbers are read from the job definition at startup. Changing either one
needs a restart, since the scheduler caches the definition.
