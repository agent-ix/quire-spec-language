---
id: SR-20261105101066
title: "QSL-1770 PR2456 code-review"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1770 PR2456 at 826c26f159e8e454385a14c4590e8b6faa99f8a5; qsl-foundation/src/diagnostic.rs; qsl-foundation/tests/kernel_refusal_record.rs"
relationships:
  - type: references
    target: ix://agent-ix/quire-spec-language/FR-096
---

## Summary

Ticket: QSL-1770. Urgent exhaustive-match compile fix only; full bounded conversion and strict URI diagnostic integration remain incomplete. Examined both changed files and upstream quire-exact origin/main API. No applicable AssuranceProfile found.

## Verdict

FAIL. The structural spelling policy has two implementations. Remove the duplicate by sharing the existing behavior through the foundation layer. API compatibility: QuantityResultInterval::new(UnitId, Rational, Rational), unit/min/max accessors and boxed refusal target agree with fresh upstream source. The real test checks exact output, reduced bounds, two package identities, declared/compound units, code/cause/category, exact field set, caller locus and absence of actual magnitude.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | New unit_spelling duplicates the existing structural unit formatter in qsl-replay/src/spine.rs:2523; consolidate this policy in one shared home. | qsl-foundation/src/diagnostic.rs:887; qsl-replay/src/spine.rs:2523 | implementation-bug-despite-evidence |

## Coverage

Both changed files reviewed against repository instructions and Rust API, implementation, testing and wire rules. No CI, manifest or lock changes. No source stubs, mocks, suppressions or gate narrowing introduced. Existing unit spelling search covered qsl-* Rust sources and found qsl-replay/src/spine.rs:2523. cargo fmt --check and git diff --check passed. Compilation and tests intentionally not run against known old lock lacking the new variant; focused execution requires Draco's lock bump.

Matrix command: timeout 20s quire matrix --scope . --format json exited 124. Scoped manual fallback inspected the new #[trace("TC-428", "FR-096-AC-8")] binding, owning FR-096 and TC-428, production arm and real assertions. Existing FR-096-AC-8 concerns its thirteen enumerated older causes; new bounded test extends the shared refusal behavior rather than claiming full QSL-1770 acceptance. No global coverage percentage is claimed. Semantic gap-analysis expansion skipped (not requested).

Plan completion: not assessed

Scope records:

```json
[
  {
    "id": "kernel_refusal_record",
    "path": "qsl-foundation/src/diagnostic.rs",
    "role": "examined",
    "excerpt": "Refusal::BoundedQuantityOutOfDomain { target }"
  },
  {
    "id": "unit_spelling",
    "path": "qsl-foundation/src/diagnostic.rs",
    "role": "examined",
    "excerpt": "fn unit_spelling(unit: &quire_exact::UnitId) -> String"
  },
  {
    "id": "a_bounded_quantity_refusal_renders_its_structural_target",
    "path": "qsl-foundation/tests/kernel_refusal_record.rs",
    "role": "examined",
    "excerpt": "expected(&format!(\"Quantity<{spelling}>[1/1, 2/1]\"))"
  },
  {
    "id": "FR-096-AC-8",
    "path": "spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md",
    "role": "context_only",
    "excerpt": "For each of the thirteen kernel causes in the key table, a `RefusalRecord` built from an S6a `Evaluation` whose outcome is that kernel `Refused` carries the table's code and cause, category refusal, exactly the table's field keys and the evaluation's resolved locus, and `Refusal::code()` and `Refusal::cause()` return the same code and cause."
  }
]
```
