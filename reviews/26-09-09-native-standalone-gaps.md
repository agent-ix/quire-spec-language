---
id: SR-174
title: "Standalone native workflow delivery gaps"
type: SpecReview
analysis: gap-analysis
scope: "plan/Plan-009-native-workflow; Task-025; FR-026; TM-007"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-009
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-007
    type: references
---
## Summary

Task-021–025 are done. The source-to-runtime file command is implemented at
d7437e2; broader LC05 extraction adoption and assurance remain ongoing.

## Verdict

**CONDITIONAL** — standalone execution is delivered; broader integration remains open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | C's real Quire extraction adoption and deferred backend/assurance qualification remain outside the standalone command. | Plan-009; IT-003; IT-008 |
| FND-002 | medium | Defensive evaluation refusal is not exercised end-to-end; validation refusal and stopped evaluation do not close that assurance gap. | FR-026-AC-3; SR-173 |

## Coverage

Quire reports FR-026 at 5/5 criteria backed and TM-007 at 10/10 test cases;
global rollup is 280/288. The four new binary tests carry real trace attributes.
Matrix statuses were checked against actual passing runs because of the known
Coverage Status/Status catalog mismatch. No scoped unbacked row or unowned
requirement was reported by tracing; this is not engine-verified Tested status
(tracked in #28), nor evidence of every outcome branch. The repeated scan remains
FR-026 5/5, TM-007 10/10 and global 280/288. No unowned changed
behavior was found; command decoding, I/O, result views and the example generator
are owned by FR-026. Existing global metric and integration gaps remain open.
Optional semantic gap review was declined and skipped.
