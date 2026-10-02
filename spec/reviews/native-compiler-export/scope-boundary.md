---
id: SR-181
title: "scope-boundary review of standalone compiler export"
type: SpecReview
analysis: scope-boundary
scope: "FR-027; TC-105; TM-007; Task-026"
review_set: all
---
## Summary

Agent A owns FR-027 command orchestration (core). NativeModel, parser/linker/checker and NativePackage retain semantic ownership; filesystem/stdout are local I/O boundaries. Serde and OS behavior are assumed dependencies, with shape/digest/output contracts tested at the native binary. B's evidence envelopes and C's Quire/producer adapters remain separate. Test helpers share one Rust module instance rather than duplicate semantic setup.

Author PR-readiness review of `f0cfe7b` with the owner-selected all set.
No applicable AssuranceProfile was found; review timing follows the owner directive.

The compilation module owns model admission and parse/link/check/package orchestration; Intake owns bounded file/source reading. The binary owns argv and output streams. RunError naming remains compatible pending the cross-cutting error API ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-027; TC-105 |
