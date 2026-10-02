---
id: SR-244
title: "Integer IR delivery and remaining Plan-008 acceptance"
type: SpecReview
analysis: gap-analysis
scope: "plan/Plan-008-native-lowering; Task-032; FR-033; TM-006"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-008
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-006
    type: references
---
## Summary

Task-032 is implemented with review corrections at b0c02c7.
Tasks 019/032 are done;
Task-020 retains generated activation qualification. The owner permits this
engineering delivery while preserving incomplete full-plan acceptance.

## Verdict

**FAIL for complete Plan-008 acceptance** — Task-020 remains in progress.
The integer IR PR is ready for review under the owner's delivery direction.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Generated activation remains deferred; numeric/object/graph backend implementation and parity are also incomplete. | Task-020; FR-009-AC-5; IT-002 |
| FND-002 | medium | Compiler issue #28 owns the matrix status-column mismatch; authored Tested marks are not engine-verified. | spec/native-lowering/tests.md |

## Coverage

Quire reports FR-033 5/5 criteria, TM-006 4/4 test cases and global 319/323.
Ten new traced tests cover both pinned IR readers, actual command selection and
filesystem failures, plus target-name/encoding checks and a private wire-domain
refusal control. The correction's focused suite passes 60 tests; SR-243 records
the full local feature lanes. Counts describe trace bindings, not execution.
TC-094 remains visibly deferred in the matrix; empty status_lies is not proof
that this existing status-selector mismatch disappeared.

Reverse mapping covers target selection, primitive translation, pre-serialization wire admission, bounded output,
original read/source identity and command diagnostics under FR-033,
reusing FR-009/029's existing package/intake behavior. No scoped unbacked row,
unowned behavior or stub was found. Optional semantic gap review was declined
and skipped. Neither trace backing nor native runtime success completes backend
assurance or the LC04 epic.
