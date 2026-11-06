---
id: SR-20261105101058
title: "code-review review of QSL-1693 Rational bounds"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1693; PR #2430; FR-505/TC-1300 and FR-517-AC-12/TC-1312; intake.rs, model_intake.rs, checked_handoff.rs, R model and library"
review_set: subset
---

## Summary

The feature parses canonical signed-64 Rational bound strings without widening, accepts safe JSON integers, and tests exact type retention in the emitted handoff. The stale parser comment identified in review was corrected.

## Verdict

**PASS** — No open findings remain. FND-001 was fixed by updating the parser comment to match the safe-number and signed-string contract.

## Scope

- FR-505-AC-1 and FR-505-AC-3: bound representation and typed refusals.
- FR-517-AC-12: producer output retains the authored `Ratio` and `Exact` types and bounds.
- `qsl-semantics/src/model/intake.rs`: bound parsing and numeric/string validation.
- `qsl-semantics/tests/it/model_intake.rs`: exact values, malformed strings, unsafe numbers, and refusal order.
- `tests/it/checked_handoff.rs`: emitted models and independently read typed Rational literal values.
- R model and library fixtures: authored bounds and source reachability.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `bound_operand` doc comment says the integer branch does not authorize numeric wire operands, but the caller accepts safe JSON integers through 2^53 under FR-505 and QSpec FR-154. Update the comment to describe the current contract. | qsl-semantics/src/model/intake.rs:4139-4141 |

## Coverage

The checked parser uses a canonical decimal round-trip and an `i64` conversion for strings; the caller enforces the safe-number magnitude. It validates structure and applicability before missing-key checks, validates keys before operand shapes in keyword order, and checks the domain before the optional unit. Tests exercise those paths and assert the emitted bounded domain for each handoff package. No implementation defect was found. The comment was corrected after the review finding.

## Evidence and limits

Independent source review only. No build, test, or gate was run by this reviewer; the final scoped gate remains with the lead after the lock refresh. No Cargo files or implementation files were modified by this reviewer.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The parser comment now states the accepted safe JSON integer and canonical signed-64 string forms. |

After excerpt: `/// The caller admits safe JSON integers through ±2^53 or canonical signed-64 decimal strings under QSpec FR-154 / TC-981.`
