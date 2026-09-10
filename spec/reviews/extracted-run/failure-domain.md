---
id: SR-216
title: "failure-domain review of standalone Markdown execution"
type: SpecReview
analysis: failure-domain
scope: "FR-031; FR-030 identity pairing; TC-109; TM-007; Task-030"
review_set: all
---
## Summary

Unsupported command/package combinations and multiple bindings refuse before dependent reads. Stale original bytes refuse before extraction; Quire schema/availability failures remain visible. The consumer checks correspondence before native parsing. Runtime incomplete/refused results retain extraction metadata without truth; other intake failures keep their existing envelope. No callback, shared mutation or new graph traversal is added.

Author PR-readiness review of `0d3d294`, using the owner-selected all set.
No applicable AssuranceProfile was found; timing follows the owner directive.

Three mode refusals now have distinct catalogued codes; wrong-clause-count
details retain the actual count. Selection happens before missing-model controls
can reach I/O, and exactly one binding is retained from one guard. Typed output
preserves preflight details, optional source mapping and original producer data.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-031; TC-109 |
