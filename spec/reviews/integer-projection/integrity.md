---
id: SR-237
title: "integrity review of bounded integer IR lowering"
type: SpecReview
analysis: integrity
scope: "FR-033; TC-111; TM-006; Task-032"
review_set: all
---
## Summary

FR-033 adds IR translation without redefining signed bounded arithmetic or erasing the original native model's nominal/unit authority. The native checker and actual strict binder both judge definedness. Wire integers use the existing flattened input shape; no second AST or native-predicate evaluation is substituted. Source-only integer artifacts are distinguished from backend acceptance and runtime truth. The scripted CLI selector has a concrete default and unknown-target refusal.

Author PR-readiness review of `7b5b663`, including the generator I/O correction,
using the owner-selected all set. Numeric lowering is unchanged from `5a7e5db`.
No applicable AssuranceProfile exists; timing follows the owner directive.

## Findings

Generator failure is explicitly nontransactional: partial files may remain, but
filesystem errors cannot become a successful exit or a panic. The new criterion
has actual invalid-directory, later-write and fresh-retry controls.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-033; TC-111 |
