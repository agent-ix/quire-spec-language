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

Task-032 is implemented and locally tested, with generator correction at 7b5b663.
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
| FND-002 | low | Coverage Status is inspected manually because the catalog's coverage selector expects Status. | spec/native-lowering/tests.md |

## Coverage

Quire reports FR-033 5/5 criteria, TM-006 4/4 test cases and global 319/323.
Seven new traced tests pass, including both pinned IR readers, actual commands
and filesystem failure propagation. The unchanged numeric tests retain their
5a7e5db baseline; the 19 affected command tests pass at 7b5b663.
TC-094 remains visibly deferred in the matrix; empty status_lies is not proof
that this existing status-selector mismatch disappeared.

Reverse mapping covers target selection, primitive translation, bounded wire
output, original read/source identity and command diagnostics under FR-033,
reusing FR-009/029's existing package/intake behavior. No scoped unbacked row,
unowned behavior or stub was found. Optional semantic gap review was declined
and skipped. Neither trace backing nor native runtime success completes backend
assurance or the LC04 epic.
