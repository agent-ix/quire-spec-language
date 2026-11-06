---
id: SR-20261105101032
title: "Gap analysis — QSL-1750 compound conversion frame"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1750 AC1-AC3, QSpec FR-142-AC-10, TC-187 U22/U25; initial review head 42e0353e0f54139c22a2cffca37f9b6d175a7bab; re-review through 41b19b02bba4210255bafcc7b3257237512d3b67"
review_set: subset
---

## Summary

Manually compared QSL-1750 AC1-AC3 and QSpec TC-187 U25 with the changed implementation and tagged focused test at PR #2419 head `42e0353e0f54139c22a2cffca37f9b6d175a7bab`. The test covers a compound source converting to named `J`, but not the compound pivot target and both conversions required by U25.

## Verdict

**FAIL** — AC1's U25 compound pivot sequence is not implemented/evidenced at the reviewed head; the required first conversion into the compound target is absent from the focused test and public check path used by it.

## Coverage

- Ticket AC1, including QSpec TC-187 U25: manually traced to `compound_conversion_frame.rs`; incomplete because the checked conversion is only from compound source to named `J`.
- Ticket AC2: both unknown-compound-unit and dimension-mismatch cases assert typed `CheckCause::IllTyped` refusals.
- Ticket AC3: no claim that QSpec agreement rows run on QSL main; no repository matrix evidence was available.
- New test has `QSpec-FR-142-AC-10` and `QSpec-TC-187` trace tags.
- `quoin matrix --repo . --json` ran for about four minutes without output and was interrupted; full matrix, repository-wide reverse-gap scan, and stub scan were unavailable. This is a scoped manual acceptance check, not a repository-wide PASS.
- Plan completion: not assessed.
- Optional semantic review: not requested and skipped.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC1/U25 requires `N_m → compound kg*m^2*s^-2 → J` as two explicit conversions, each charging identity-read, target-domain, and result-retain. At the reviewed head, the test runs only a single compound-source-to-`J` conversion, so the target-side pivot behavior and agreement row are not covered. | qsl-eval/tests/it/compound_conversion_frame.rs:119 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | AC3 requires both U22 and U25 agreement rows on QSL main. This PR's new compound test traces and exercises U25 only; no QSL test explicitly exercises U22's compound-related vector. | qsl-eval/tests/it/compound_conversion_frame.rs:68-70 |
| FND-003 | low | At reviewed commit 8aa2a5857, the second U25 leg asserts magnitude 1 but not that the output unit equals the checked J target. | qsl-eval/tests/it/compound_conversion_frame.rs:250-258 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Commit 8aa2a5857 now evaluates the compound quantity literal and both U25 conversion steps through checked frame evaluation, with each conversion's expected charge sequence. |
| FND-002 | still-open | The reviewed PR head has a U25 case only. No U22 compound agreement vector is present in the QSL tests examined. |
| FND-003 | still-open | The requested reviewed commit lacks an assertion comparing the second result unit to the checked J target. A matching assertion exists only in an uncommitted worktree edit. |
| FND-002 | rejected | The U22 row is exercised by `qsl-semantics/tests/it/quantities.rs::u22_declared_conversion_requires_one_dimension_node`; U25 is exercised by `u25_compound_pivot_between_nominal_dimensions` and the new retained-frame QSL evaluation test. |
| FND-003 | fixed | Commit 96695ebe9 asserts the second result unit equals the unit on the checked conversion target. |


## Follow-up head review

Reviewed commit `41b19b02bba4210255bafcc7b3257237512d3b67` against `96695ebe948247b663b36498a436d368c624b084`. Its only change is a private `ClauseExpressionTarget` enum and equivalent `match` dispatch for expected-type versus conversion checking. It preserves the two behavior paths already reviewed; no gap or disposition changes. No build or test was run.
