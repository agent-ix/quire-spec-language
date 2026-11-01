---
id: SR-202610310146
title: "QSL-1419 / QSL-1432 scoped gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1419 and folded QSL-1432; qsl-eval/src/simulation/protocol.rs, protocol/access.rs, protocol/steps.rs; FR-223-AC-5/8 and TC-668"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: references
---

## Summary

The PR changes only internal RA exploration indexing and keeps the exact LT-16 exploration assertions and forbidden-outcome oracle. The scoped static matrix marks FR-223-AC-5 and FR-223-AC-8 tagged by the existing LT-12/LT-13 and LT-16 tests. Full-repository matrix/reverse-gap evidence is incomplete because both repository-wide matrix commands failed to finish in the review window; this is a scoped result, not a repository-wide assurance PASS.

## Verdict

**CONDITIONAL** — the PR-scoped requirements are tagged and no changed behavior lacks an owner, but a full repository gap audit could not be completed. The 60 s debug target also remains unmet; the recorded ticket evidence satisfies the ticket's measured-reason alternative only if the code lead accepts that disposition.

## Coverage

- Scoped `quire matrix --scope /tmp/qsl1419-scope --format json`: FR-223-AC-5 is tagged by `lt12_reaches_its_weak_outcome_under_ra_only` and `lt13_reaches_its_weak_outcome_under_ra_only`; FR-223-AC-8 is tagged by `lt16_collected_sc_predecessors_still_constrain_the_sc_load`.
- Full-root `quoin matrix --repo . --json` and `quire matrix --scope . --format json` did not complete; their partial outputs did not establish repository-wide coverage. The second command was interrupted after more than five minutes without a result.
- Reverse-gap check of the three changed files: the new maps and helper are private implementation indexes; they add no externally observable behavior. FR-223 owns the preserved RA/SC semantics, and QSL-1419/QSL-1432 own the scoped performance/exactness work. No source or test stub, weakened assertion, cap, or changed trace tag was found in the diff.
- No applicable AssuranceProfile was found. The measurement plans under `spec/assurance/` cover separate model/evaluator benchmark apparatus.
- Semantic review was skipped; it is the optional intent↔test↔code pass and was not requested.
- Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
