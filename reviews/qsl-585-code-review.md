---
id: SR-1957033
title: "Code review — bounded DBM subtraction"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/src/zone_check/dbm.rs, qsl-analyze/src/zone_check/dbm/tests.rs, FR-238, FR-419-AC-6, TC-693"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-238
    type: references
---

## Summary

Reviewed PR #2038's bounded-subtraction DBM operation and its tests, including Rust correctness, exact arithmetic, error atomicity, canonicality, and acceptance traceability. The transform is mathematically consistent with `x' = x - amount`, but the test oracle and trace claims do not substantiate the full acceptance coverage.

## Verdict

**CONDITIONAL** — the focused implementation is sound on inspected paths, but the test oracle duplicates the production transform and the test claims a task-product AC that it does not verify.

## Scope

- `qsl-analyze/src/zone_check/dbm.rs:354-415` — bounded subtraction API, validation, guard, row/column update, closure.
- `qsl-analyze/src/zone_check/dbm/tests.rs:200-263, 534-632` — reference transform and focused tests.
- FR-238-AC-4 — independent reference, all specified subtraction cases, malformed input atomicity.
- FR-419-AC-6 — exact fixed-work product and bounded/incomplete interval-work routes.
- TC-693 — declared test scope and procedure.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The bounded-subtraction reference duplicates the production row/column shift formula, so the comparison is not an independent oracle for transform errors. | qsl-analyze/src/zone_check/dbm/tests.rs:240-263, 558-581 |
| FND-002 | medium | The test is tagged to FR-419-AC-6 although it tests only one DBM primitive, not the exact task-product/certificate gate or bounded interval-work outcome required by that AC. | qsl-analyze/src/zone_check/dbm/tests.rs:534-540 |

## Review Notes

- The implementation first intersects a cloned zone with `amount <= x <= ceiling`, then shifts only the selected clock's row and column and recloses. The coordinate substitution is correct for `x' = x - amount`; `BigInt` avoids overflow and errors are returned before mutating `self`.
- Focused DBM tests, formatting, Clippy, and dependency bans passed. Aggregate/full-package failures reproduced on the base commit and are recorded separately to the lead.
