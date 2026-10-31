---
id: SR-1932004
title: "Gap analysis — QSL-965 FR-280 EN-4 provider path"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-280-AC-5, FR-280-AC-6, FR-281 AnalyzeLimits; qsl-analyze/src/analyze.rs, qsl-analyze/src/provider.rs, qsl-analyze/tests/it/analyze.rs"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-761
    type: reviews
review_set: subset
---

## Summary

The manual acceptance-to-test check confirms the EN-4 Service parity case in FR-280-AC-5, including witness replay. FR-280-AC-6 is only partially evidenced, and the public `AnalyzeLimits.deadline_ms` input has neither implementation enforcement nor a finite-deadline test; the repository-wide matrix audit was unavailable.

## Verdict

**CONDITIONAL** — focused evidence exposes two medium gaps; the complete repository matrix, reverse-gap and stub audit could not be obtained.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | A finite AnalyzeLimits.deadline_ms has no enforcement and no test; the field is never read, so the operation may outlive the caller's deadline. | qsl-analyze/src/analyze.rs:211 | implementation-bug-despite-evidence |
| FND-002 | medium | FR-280-AC-6 has only a one-item incomplete-snapshot test; no test covers the refused undeclared-population binding, reversed result order, or no-engine-run assertions. | qsl-analyze/tests/it/analyze.rs:564 | correct-requirement-no-evidence |

## Coverage

Plan completion: not assessed.

Manual acceptance-to-test check:

- FR-280-AC-5 / TC-761 step 4: `analyze_and_the_provider_entry_settle_one_record_for_one_en4_item` asserts parity with `analyze`, the Service emitted-package refusal before and after, three sampled witnesses, and `Reproduced` replay for each witness.
- FR-280-AC-6 / TC-761 step 5: `provider_entry_settles_an_incomplete_binding_per_item` verifies one incomplete snapshot identity maps to one backend-free declined record with `unavailable_observation/missing-required-artifact`. It does not cover the undeclared-population refusal, order preservation across multiple records, or proof that no engine ran.
- FR-281: `AnalyzeLimits` declares a wall-clock `deadline_ms`; no source reference beyond declaration/default exists in `qsl-analyze/src`, and the new public analyze path passes only the caller's `Cancel` to engines. No finite-deadline test exists.
- FR-280-AC-2 covers every TC-762 item. This QSL-965 slice adds the EN-4 route; the existing ticket record identifies EN-1 provider support as a separate remaining item, so that broader slice is not assessed as a defect in this ticket.

`quoin gap-analysis --help` is unavailable in installed quoin 0.29.1 (`command gap-analysis not found`). The fallback `quoin matrix --repo . --json` was attempted but produced no output for more than 90 seconds before interruption; full matrix and reverse-gap/stub results are therefore unavailable. Optional semantic review was skipped.
