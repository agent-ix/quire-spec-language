---
id: SR-20261104074840
title: "Code review — QSL-1624 reader projection and QSL-1626 partial unit retention"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2278 reviewed at rebased head c90f70ce52c0c8a4aa1d19e9913b2e21b190ea36: qsl-eval/tests/it/protocol_slots/acceptance.rs, qsl-package/src/checked_v2.rs, qsl-package/src/evaluation_view.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/protocol_obligation.rs, qsl-semantics/src/check/protocol_slots.rs; FR-390-AC-18 and FR-390-AC-25"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: reviews
---

## Summary

Ticket: QSL-1624 is implemented on the reviewed head. The reader projection uses the checked source root for its exact recursive result type while retaining field and declaration identity from the admitted I2 graph; the AC-18 test supplies the matching E-P model document and compares full results and ordered charges. QSL-1626 remains accurately partial. The initial FND-001 trace issue was fixed by removing the AC-25 tag from the in-process-only test; the rebased head preserves that fix and introduces no new findings.

The Rust diff was inspected against repository guidance, the relevant callers, and the acceptance test. No merge gates were run per review dispatch.

## Verdict

**PASS** — re-review at c90f70ce confirms the previous FND-001 fix and found no new defects in the rebased diff. AC-18 reader behavior and exact checked-root type use remain sound in the inspected scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The in-process-only compound-unit test is tagged FR-390-AC-25, so the computed matrix can count it as coverage although the criterion requires reader-backed projection and parity; that reader test remains ignored on QSL-1625 | qsl-eval/tests/it/protocol_slots/acceptance.rs:3949 |

## Scope examined

- FR-390-AC-18: “Reader views resolve Boolean, bounded Integer, record/composite and Reference<Node> identities of TC-1070 fixture E to the exact kernel values/types of the in-process view, including recursively nested Option/Collection values used by a slot. AC-11/15 fail if a required QSL-emitted type is reported unsupported; unsupported cases cannot replace any required result/log comparison.”
- FR-390-AC-25: “A Quantity reader projection preserves its exact declared or compound `UnitId`; two declared units of one dimension remain distinct Quantity types, and only an explicit FR-142 conversion yields the selected target unit type. A dimension name or matching exponent vector alone never selects a unit identity, and a Dimension is not admitted as a standalone runtime value.”
- Changed implementation: reader-side projection and exact type handoff in `qsl-package/src/checked_v2.rs` and `qsl-package/src/evaluation_view.rs`; formed-unit retention in protocol obligation/slot checking and final lowering in `qsl-semantics/src/check/{mod.rs,protocol_obligation.rs,protocol_slots.rs}`.
- Changed acceptance test: `qsl-eval/tests/it/protocol_slots/acceptance.rs`.

## Findings detail

### FND-001 — AC-25 coverage is overstated

`protocol_capture_compound_quantity_identity_is_held` evaluates only the in-process `CheckedPackage` view, yet it carries `#[trace("FR-390-AC-25")]`. AC-25 specifically requires reader projection and reader/in-process parity. The reader-backed compound identity test remains ignored because computed-root support is assigned to QSL-1625. The PR body correctly says QSL-1626 is partial, but this trace tag can make the computed Test Matrix represent AC-25 as covered before its reader behavior is exercised. Keep the in-process test, but bind it only to a criterion it fully verifies, or leave it untagged until the reader-backed acceptance is runnable.

The `checked_root_type` handoff itself is consistent with this boundary: the selected root belongs to the checked source package, its type is taken from that exact checked root, and field operation/identity and field presence selection are still obtained through the admitted graph and its I2 identity rows. The AC-18 test uses the same E-P model document to build and read the package and compares outcomes plus charge usage/logs. No mismatch was found in this path.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Commit 0de46888c removes the FR-390-AC-25 trace from the in-process-only compound-unit test. |

Round 1 after commit 0de46888c: the test remains active as QSL-1626 partial evidence, but no longer claims reader-projection AC-25 coverage.


## Re-review after rebase

Reviewed `c90f70ce52c0c8a4aa1d19e9913b2e21b190ea36` against current `origin/main` (`bb22eded9c0756a38f1b9289564aee371d3de002`). FND-001 remains fixed: `protocol_capture_compound_quantity_identity_is_held` has no FR-390-AC-25 trace. The rebase adds no new findings. QSL-1626 reader-backed compound-unit parity remains intentionally gated on QSL-1625. No merge gates were run.
