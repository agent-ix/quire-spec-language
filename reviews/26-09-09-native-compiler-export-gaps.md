---
id: SR-184
title: "Standalone compiler export delivery gaps"
type: SpecReview
analysis: gap-analysis
scope: "plan/Plan-009-native-workflow; Task-026; FR-027; TM-007"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-009
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-007
    type: references
---
## Summary

Task-021–026 are done; source-only compiler export is implemented at f0cfe7b.
Broader LC05 producer adoption and deferred assurance remain open.

## Verdict

**CONDITIONAL** — compiler export is delivered; broader integration remains ongoing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Actual Quire extraction/consumer adoption and deferred backend assurance remain outside this command. | Plan-009; IT-003; IT-008 |
| FND-002 | medium | The catalog expects Status while matrices use Coverage Status; issue 28 tracks the mismatch. Trace binding counts do not engine-verify authored Tested statuses. | TM-007 |

## Coverage

Quire reports all three FR-027 criteria backed and TM-007 at 11/11 test cases;
global rollup is 284/292. No scoped unbacked row or unowned behavior was found.
Shared command helpers remain owned by FR-026/027 and TC-105 carries real trace
attributes. The scoped controls passed in the actual 300-test run, but matrix
status reconciliation remains unavailable because of the catalog mismatch.
Existing global metric/integration
gaps remain open. Optional semantic gap review was declined and skipped.
