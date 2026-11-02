---
id: SR-202610310156
title: "Gap analysis — QSL-1397 producer dispatch emission"
type: SpecReview
analysis: gap-analysis
date: 2026-11-02
scope: "agent-ix/quire-spec-language; PR #2047 changed producer-dispatch source and tests; FR-494-AC-1..4"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-494
    type: reviews
---
# SR-202610310156: Gap analysis — QSL-1397 producer dispatch emission

## Summary

Ticket: QSL-1397. Manual changed-scope trace review found each FR-494 criterion tagged by the corresponding new integration test (`TC-1282` for AC-1/3/4; `TC-1283` for AC-2/4). Comparing assertions with the acceptance text found two evidence gaps: the tests do not compare candidate body/precondition identities element-by-element and do not compile under permuted source declaration order. The full computed matrix and repository-wide reverse-gap scan are unavailable: `quoin matrix --repo <repo> --json` ran for more than 14 minutes without producing a result, then `quire matrix --scope <repo> --format json` ran for more than five minutes without producing JSON; both were stopped. Plan completion: not assessed. Semantic review: skipped (opt-in not requested).

## Verdict

**CONDITIONAL.** Manual review found only medium evidence gaps in the changed scope; the matrix computation and repository-wide reverse-gap audit did not complete, so no whole-repository PASS is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1282 does not verify each candidate's exact operation-body and effective-precondition identities | FR-494-AC-1, FR-494-AC-3, TC-1282 |
| FND-002 | medium | TC-1283 does not test that permuting declarations preserves canonical row order and identities | FR-494-AC-2, TC-1283 |

## Coverage

- Changed-scope trace tags: `TC-1282` binds FR-494-AC-1, FR-494-AC-3 and FR-494-AC-4; `TC-1283` binds FR-494-AC-2 and FR-494-AC-4. The tags are present in `tests/it/dispatch_emission.rs`.
- Manual acceptance checks: FND-001 and FND-002 are the specific uncovered portions of AC-1/3 and AC-2.
- Changed-scope source: `qsl-semantics/src/check/lowering.rs` retains the linked decision by exact application/location; `qsl-package/src/emit.rs` maps the retained operation witness, context, ordered candidate IDs and occurrence to the carrier.
- Matrix verification: not completed. The QSL `quoin matrix` and `quire matrix` invocations exceeded the review run's practical time budget before emitting results.
- Reverse gaps: manually inspected changed production files; no additional changed behavior outside FR-494 was identified.
- Plan completion: not assessed.
