---
id: SR-240
title: "risk-complexity review of bounded integer IR lowering"
type: SpecReview
analysis: risk-complexity
scope: "FR-033; TC-111; TM-006; Task-032"
review_set: all
---
## Summary

FR-033 has medium technical risk and medium volatility because numeric definedness and the backend boundary must remain explicit. Existing constructors/checkers, signed bound assertions, exact original source checks and whole-artifact refusals constrain the change. Both pinned IR readers qualify the actual wire shape; current codegen's unsupported-expression result is observed. No new performance guarantee or concurrency is introduced. See failure-domain.md for boundary controls.

Author PR-readiness review of `7b5b663`, including the generator I/O correction,
using the owner-selected all set. Numeric lowering is unchanged from `5a7e5db`.
No applicable AssuranceProfile exists; timing follows the owner directive.

## Findings

The existing example generator had treated user-selected path failures as static
fixture defects. Typed I/O propagation and actual failure/retry checks address
that risk without changing compiler or runtime semantics.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-033; TC-111 |
