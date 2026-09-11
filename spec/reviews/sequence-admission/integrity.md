---
id: SR-257
title: "Integrity review of the native sequence ceiling"
type: SpecReview
analysis: integrity
scope: "FR-015 sequence-ceiling amendment; TC-041; TC-065; Task-034"
review_set: all
---
## Summary

Reviewed consistency of the owner-ruling ceiling, FR-015, TC-041 and the existing runtime exhaustion case. The amendment tightens admission without changing the meaning or encoding of accepted sequence values.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The former 40,000-wide stress fixture is replaced by two 200-wide levels while preserving 40,000 logical occurrences and the original exhaustion assertions. | TC-065; tests/runtime_validation_cases/limits.rs |

## Traceability

US-002 → FR-015 → StR-001; FR-015-AC-2 → TC-041 → public Rust/source boundary tests. NFR-005 retains Rust implementation ownership. The normative response is atomic model refusal; the existing AC groups related admission-negative cases and now names exact sequence boundaries. Caller work limits and declared sequence maxima have distinct purposes. No external command/API assumptions are added. Other earlier native rules conflict with the broader ruling; issue #30 retains those differences and this review makes no whole-profile conformance claim.

