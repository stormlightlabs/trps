# Pipeline notes

The request moves through the stages we already had: parse → validate → enrich
→ dispatch. Each hop writes one line to the audit log, and a failure at any hop
leaves the request where it stopped.
