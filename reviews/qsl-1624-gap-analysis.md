---
id: SR-20261104074841
title: "Gap analysis — QSL-1624 reader projection and QSL-1626 partial unit retention"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2278 reviewed at rebased head c90f70ce52c0c8a4aa1d19e9913b2e21b190ea36: FR-390-AC-18 and FR-390-AC-25, qsl-eval/tests/it/protocol_slots/acceptance.rs, qsl-package/src/checked_v2.rs, qsl-package/src/evaluation_view.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/protocol_obligation.rs, qsl-semantics/src/check/protocol_slots.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: reviews
---

## Summary

This scoped acceptance audit found a reader-backed result and ordered-charge comparison for FR-390-AC-18. The initial review found an incorrect AC-25 trace on an in-process-only test; that finding is fixed at the rebased head. QSL-1626 remains explicitly partial, with reader-backed AC-25 parity behind QSL-1625.

The repository-wide `quoin matrix --repo . --json` and `quire matrix --scope . --format json` did not complete within 30 seconds (the latter emitted module warnings only). Per the review brief, this artifact records the scoped AC-to-test check and does not claim a repository-wide matrix, reverse-gap, or stub audit. Plan completion: not assessed.

## Verdict

**CONDITIONAL** — the earlier AC-25 trace finding is fixed and the rebase introduced no new findings. Reader-backed AC-25 parity remains intentionally incomplete under QSL-1625, and the repository-wide matrix/reverse-gap audit was unavailable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The in-process-only compound-unit test is tagged FR-390-AC-25, so the computed matrix can count it as coverage although the criterion requires reader-backed projection and parity; that reader test remains ignored on QSL-1625 | qsl-eval/tests/it/protocol_slots/acceptance.rs:3949 |

## Coverage

- FR-390-AC-18: examined. `reader_backed_projection_preserves_nested_option_and_collection_value_types` is active, carries `FR-390-AC-18`, admits the package with the matching TC-1070 E-P model document, compares the complete checked/reader evaluation result, checks expected nested values, and compares ordered charge log and usage.
- FR-390-AC-25: examined. The reader-backed declared/compound tests remain ignored pending QSL-1625 computed-root support. The active in-process-only test `protocol_capture_compound_quantity_identity_is_held` no longer carries the AC-25 trace at the reviewed head; the earlier inflated-coverage finding is fixed.
- The PR description accurately says QSL-1626 is partial and remains In Progress; the acceptance text and ignored reader test also identify QSL-1625 as the remaining dependency.
- Plan completion: not assessed.

## Findings detail

### FND-001 — AC-25 coverage is overstated

`protocol_capture_compound_quantity_identity_is_held` evaluates only the in-process `CheckedPackage` view, yet it carries `#[trace("FR-390-AC-25")]`. AC-25 requires reader projection and reader/in-process parity. The reader-backed compound identity test remains ignored because computed-root support is assigned to QSL-1625. This tag can make the computed Test Matrix represent AC-25 as covered before its reader behavior is exercised. Keep the in-process test, but bind it only to a criterion it fully verifies, or leave it untagged until the reader-backed acceptance is runnable.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Commit 0de46888c removes the FR-390-AC-25 trace from the in-process-only compound-unit test. |

Round 1 after commit 0de46888c: the test remains active as QSL-1626 partial evidence, but no longer claims reader-projection AC-25 coverage.


## Re-review after rebase

Reviewed `c90f70ce52c0c8a4aa1d19e9913b2e21b190ea36` against current `origin/main` (`bb22eded9c0756a38f1b9289564aee371d3de002`). FND-001 remains fixed: the in-process-only test is untagged, and no new findings arose from the rebase. The reader-backed AC-25 tests remain ignored pending QSL-1625, consistent with QSL-1626's partial scope. Repository-wide matrix/reverse-gap audit remains unavailable as recorded above. No merge gates were run.
