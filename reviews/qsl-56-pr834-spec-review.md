---
id: SR-9090
title: "Spec review of quire-spec-language PR #834: FR-082 AC-16 and AC-17"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@bd0e216d9804ec6a8da75bd4f2bcd3458fc4027f; spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md (refinement obligation paragraph, reference covariance bullet, AC-16, AC-17)"
review_set: subset
---
# Spec review of quire-spec-language PR #834

## Summary

Ticket: QSL-56. The edited FR-082 text was checked for unique ids, for agreement with the code and tests, and for QSpec citation.

- The AC ids are unique: AC-1 to AC-13, AC-15, AC-16 and AC-17. AC-14 stays unused. The parked covariance AC now carries AC-16, so it no longer clashes with AC-10.
- AC-16's statement and acceptance criterion agree with the covariance bullet. The test that traces AC-16 asserts its three branches.
- AC-17's ordering, single meter, withheld failures and lineage clauses match the code in check/mod.rs and check/model_conformance.rs.
- The new text cites QSpec by id only (FR-146, FR-151). It copies no QSpec text.

## Verdict

PASS for the spec text. The missing test binding for AC-17 is recorded as SR-9089 FND-002, which is a coverage defect and not a spec defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
