---
id: SR-1957034
title: "Gap analysis — bounded DBM subtraction"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-analyze/src/zone_check/dbm.rs, qsl-analyze/src/zone_check/dbm/tests.rs, spec/functional/FR-238-represent-zones-as-difference-bound-matrices.md, spec/test-cases/TC-693-dbm-zones-in-exact-integer-arithmetic.md; FR-238-AC-4"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-238
    type: references
---

## Summary

Manual acceptance-criteria-to-test and reverse-gap review of the bounded-subtraction change. The implementation supplies the core DBM update, but the submitted evidence does not yet establish FR-238-AC-4's independent-reference and full subtraction-case obligations.

## Verdict

**CONDITIONAL** — one medium verification gap remains for FR-238-AC-4.

## Coverage

- Plan completion: not assessed.
- FR-238-AC-4 is tagged by two tests; malformed range/index atomicity and one non-strict diagonal fixture are directly exercised.
- The test helper and implementation use the same row/column coordinate shift; the stated independent-reference criterion is therefore not met by this comparison.
- The test case TC-693 still declares scope only through AC-3, and its procedure does not include bounded subtraction.
- Quoin matrix generation did not complete in the available run; this is a manual scoped check of the changed AC and its direct test binding, not a repository-wide matrix claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-238-AC-4 requires agreement with an independent oracle for diagonal constraints and every specified subtraction case, but the added test's reference applies the same row/column shift formula as production and exercises one non-strict fixture rather than the full specified case set. | qsl-analyze/src/zone_check/dbm/tests.rs:240-263, 534-632; spec/functional/FR-238-represent-zones-as-difference-bound-matrices.md:105 |

## Manual Traceability

| Criterion | Test | Result |
| --- | --- | --- |
| FR-238-AC-4 | `bounded_subtraction_matches_reference_with_diagonal_constraints` | Partial: one diagonal fixture, a grid check, canonicality, zero update, and disabled update are covered; oracle independence/full specified cases are not established. |
| FR-238-AC-4 | `bounded_subtraction_refuses_malformed_ranges_and_clock_indices_transactionally` | Covers invalid clock index/reference clock and malformed numeric ranges transactionally. |
