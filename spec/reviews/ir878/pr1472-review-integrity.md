---
id: SR-17029
title: "IR-878 PR #1472 integrity review of FR-485"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-485-supplied-admission-and-conversion-owner-allocation.md, spec/reviews/ir878/base.md, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-485
    type: reviews
---

## Summary

Ticket: IR-878. PR: quire-spec-language#1472 (head 5e56adbb7, diff `origin/main...HEAD`). Reviewer pass (integrity) over the spec-only re-derivation of closed #995 as FR-485. Review set: base plus integrity, ears-conformance, evidence, scope-boundary, failure-domain (subset chosen by the dispatching brief; no AssuranceProfile in spec/).

Examined units: FR-485 (examined), FR-485-AC-1 (examined), FR-485-AC-2 (examined), FR-485-AC-3 (examined), spec/spec.md (examined), spec/reviews/ir878/base.md (examined).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-485 is free on main (max FR-484) but open draft PR #1470 (IR-1007, created 9 minutes earlier) also adds spec/functional/FR-485-acquire-and-apply-a-checked-closure-through-the-replay-facade.md with id FR-485. Whichever PR merges second duplicates FR-485 and FR-485-AC-1..3. Re-allocate one of the two before merge. | FR-485:21 |
| FND-002 | medium | The eight committed SpecReview files use SR-17020..SR-17027; draft PR #1470 adds SR-17020..SR-17025 for its own reviews. Six SR ids collide across the two open PRs. | spec/reviews/ir878/base.md:2 |
| FND-003 | high | The Integer/Rational evaluator-charge control was dropped in the re-derivation. #995 FR-394-AC-2 said 'The supplied caller's qualified numeric helper leaves evaluator charges unchanged'; quire-exact FR-380 says 'the evaluator charge control remains with QSL'. FR-485-AC-2 keeps only bound resolution and pass-through, and AC-3 covers only the Decimal fixture, so no FR-485 criterion verifies evaluator-meter invariance for the FR-380 Integer/Rational supplied path. | FR-485-AC-2:41 |
| FND-004 | low | The Description cites quire-exact FR-380-AC-6 and FR-372-AC-5 (AC-3 cites FR-372-AC-5 too) as live criteria, but both ids are retired on quire-exact main and survive only in 'formerly FR-372-AC-5' and 'formerly FR-380-AC-6' allocation notes. Say 'formerly' or cite the FR-level notes. | FR-485:21 |

## Verdict

Request changes: the FR-485 and SR id collisions with draft #1470, and a dropped Integer/Rational evaluator-charge obligation.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in dbb95abc5 against origin/main. |
| FND-002 | fixed | Verified in dbb95abc5 against origin/main. |
| FND-003 | fixed | Verified in dbb95abc5 against origin/main. |
| FND-004 | fixed | Verified in dbb95abc5 against origin/main. |
