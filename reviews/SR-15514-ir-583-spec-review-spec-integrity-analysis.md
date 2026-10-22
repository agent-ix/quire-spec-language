---
id: SR-15514
title: "Integrity review of AD-001 evaluator slice ordering"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/assurance/AD-001-shared-evaluation-core-interface.md; IR-583 PR #1224"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: reviews
---
# SR-15514: Integrity review of AD-001 evaluator slice ordering

## Summary

Ticket: IR-583. I compared PR #1224's changed blocker table and surrounding prose with the structured incoming Linear relations for IR-583 and IR-585 through IR-590. The table's direct edges and the S1-to-S6 transitive IR-269 path match those relations. The new independence statement conflicts with an unchanged ordering decision later in AD-001. No requirement statement or verification method changed, so EARS and evidence analyses were not applicable.

Examined units:

- AD-001 System Boundary, changed S6 and IR-269 ordering text: "The no_std + alloc implementation is IR-590, after S1-S5. IR-269 remains a direct blocker of S1 and therefore an indirect predecessor of S6."
- AD-001 Staged ownership and blockers table, IR-583 and S1-S6 rows (lines 150-158): all seven direct incoming blocker lists were compared with `linear issue relation list`.
- AD-001 Staged ownership and blockers prose (lines 160-166): "The structured graph leaves S2, S3 and S5 independent of the IR-269 MVP gate and the IR-583 interface; each waits for its named spec blocker."
- AD-001 Decisions (lines 208-209): "Checked-term separation follows the interface contract (IR-583 before IR-586), while the no_std implementation still waits for S1-S5."
- AD-001 Risks, changed IR-269 reachability text (lines 230-232): "IR-269 directly blocks S1 and indirectly precedes S6 through S1. It does not block S2-S5 directly."

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Remove or reconcile the stale IR-583-before-IR-586 decision: the new table correctly shows IR-586 blocked only by STD-297, but the Decisions section still imposes IR-583 first. | spec/assurance/AD-001-shared-evaluation-core-interface.md:160; spec/assurance/AD-001-shared-evaluation-core-interface.md:208 | wrong-requirement |

## Verdict

Changes requested for FND-001. The edited table, IR-269 reachability, and S1/S4/S6 ordering otherwise match the measured structured relations. This review covers the Markdown diff and the directly conflicting existing decision only.
