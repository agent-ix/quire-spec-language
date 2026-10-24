---
id: SR-14851
title: "EARS review of STD-256 FR-205, FR-207 and FR-216 statements (PR #1200, QSpec PR #353)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@6e35052ff7072d7486001ff9721d5e5a0bad0fe9; spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md, spec/functional/FR-207-run-compensation-templates-as-protocol-threads.md, spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md; agent-ix/quire-specification@21ff693e1113a6d69e501fab5300552a8ead515b; spec/functional/protocol/FR-425-define-protocol-subjects-and-state-keys.md, spec/functional/protocol/FR-426-apply-protocol-steps.md, spec/functional/protocol/FR-427-check-protocol-compensation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-207
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
---
# SR-14851: EARS review of STD-256 (PR #1200, QSpec PR #353)

## Summary

Ticket: STD-256. These are the changed requirement statements:

- FR-205's `ProtocolKey` bullet, whose third member is the attempt record.
- FR-207's `cattempt` bullet: "SHALL take one `cattempt` step per record of the
  template's attempt type ... The step's record SHALL be the thread's attempt
  record and part of the step's transition identity."
- FR-216's commit-node and `cattempt` footprint bullet.

The ADR-027 rows, the QSpec FR-425/FR-426/FR-427 tables and prose, and the AC
rows are not EARS statements. `make check-spec-validation` reports no EARS
warning on a touched file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. Each changed bullet is ubiquitous, and each names its subject ("The
protocol system", "The key", the step kind). The second SHALL in FR-207 makes
a separate claim about the record and follows the file's existing bullet
style. The wording of the retention condition is recorded in SR-14850
FND-001.
