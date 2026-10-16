---
id: SR-11712
title: "Gap analysis of quire-spec-language PR #902: record-field typed comparison"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a3c4b6c28ce233112b7a9d53923a33abfff31ed3; qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/document.rs, qsl-semantics/src/model/observation/frame.rs, qsl-semantics/tests/it/native_snapshot_values.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #902

## Summary

Ticket: QSL-812. A manual spec-row-to-test check scoped to the PR diff. No
matrix run (read-only, no build). Plan completion: not assessed.

Examined:
- FR-106 check 11 equality text ("a record the same most-specific type and
  equal effective fields", each field under its declared type's equality)
- FR-106 value-count text (the record and each contained value count)
- FR-106-AC-37, FR-106-AC-40, FR-056-AC-21
- the tests the diff adds or changes, and the production code they reach

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

No blocking gaps of their own. The trace and test-intent defects are recorded
once, in SR-11711 FND-002 and FND-003, not duplicated here.

- The production change is fully reached: the new test drives
  `record_field_type` and the record arm of `equals_by` through `frame::run`.
- The value count of 17 matches FR-106: the `Address` record and its
  `street` add two values to the previous 15.

Recorded (medium/low, not fixed):
- FR-106 check 11's record clause has no AC row, so the record-field
  comparison is specified only in the body text. SR-11711 FND-003 carries the
  fix.
- No test covers nested records, or Option or Sequence fields inside a
  record.
