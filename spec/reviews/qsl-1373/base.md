---
id: SR-13731
title: "Base review of QSL-1373 proof-checking and SMT-normalization changes"
type: SpecReview
analysis: base
scope: "spec/functional/FR-314-check-an-smt-proof-certificate.md, spec/functional/FR-315-encode-the-smt-lib-transition-relation.md, spec/test-cases/TC-893-an-smt-proof-certificate-is-checked.md, spec/test-cases/TC-900-the-smt-lib-encoding-is-canonical.md"
review_set: subset
---

## Summary

Reviewed the four changed FR/TC artifacts against the base checklist, including IDs, validation links, trace coverage, testability and the hash/digest/pin rule. No applicable `AssuranceProfile` exists under `spec/`; the selected review set is base plus failure-domain, integrity and evidence. Follow-up confirmed FND-001 is resolved: AC-18 and TC-893 now include an accepted nonlinear identity, and the new criteria have criterion-level source trace tags.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Resolved: FR-314-AC-18 and TC-893 Step 8 now accept the nonlinear identity `(= (* x y) (* y x))`. Source tags now cover FR-314-AC-18, AC-19, AC-20 and FR-315-AC-57 in `qsl-replay/src/certificate/alethe.rs`, `qsl-replay/tests/it/refinement_smt.rs` and `qsl-eval/src/smt/normal.rs`. | FR-314-AC-18, FR-314-AC-19, FR-314-AC-20, FR-315-AC-57, TC-893, TC-900 |
