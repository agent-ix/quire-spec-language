---
id: SR-13952
title: 'Gap analysis of PR #1186: quantile field and a buildable symmetry case (STD-252), changed ACs against TC rows'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@91e502f0e; git diff origin/main...91e502f0e (PR #1186, branch std-qsl570-252): FR-189-AC-3, FR-189-AC-15 (new), FR-192-AC-6 (new), TC-624, TC-627; ticket STD-252'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-189
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-192
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-624
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-627
  type: reviews
---

## Summary

Ticket: STD-252. PR: quire-spec-language#1186, head 91e502f0e. Method: gap analysis of the new and changed ACs against their TC rows, done by hand from the spec. This is a spec-only PR, so no matrix run and no cargo tests. Plan completion: not assessed. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

Changed ACs and their TC rows:

| AC | Verification | TC row |
| --- | --- | --- |
| FR-189-AC-3 (second clause reworded) | Test (TC-624) | Step 3 "the symmetric variant with each workload"; expected result 3 "`m = 1` under each workload". Covered, but see SR-13950 FND-003 for the stale fixture line. |
| FR-189-AC-15 (new) | Test (TC-624) | Scope extended to AC-15; step 13 and expected result 13 cover the three integer cases and the `P95` test. The quantity-typed clause and the no-counted-sample clause are not covered (FND-001). |
| FR-192-AC-6 (new) | Test (TC-627) | Scope extended to AC-6; step 5 and expected result 5 cover Value(2), Censored, and absent for `P95`, a mean of a fraction and `LongRun`. Fully covered. |

No test in the tree is tagged with the new ACs yet. The field does not exist in code yet (it is the PR body's remaining code work), so that is expected and not a finding.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-189-AC-3 | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:237 | examined |
| FR-189-AC-15 | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:249 | examined |
| FR-192-AC-6 | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:146 | examined |
| TC-624 scope, fixtures, steps 3 and 13, expected results 3 and 13 | spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md:15,19,22,34,42,52 | examined |
| TC-627 scope, step 5, expected result 5 | spec/test-cases/TC-627-statistical-results-settle-as-measured.md:15,25,35 | examined |
| FR-189-AC-13 (stub sample-count precedent) | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:247 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two clauses of FR-189-AC-15 have no TC-624 step or expected result: "A quantity-typed `M` gives a `Value` carrying `M`'s unit", and "a quantile test with no counted sample [has] `quantile` absent". Step 13 runs only the three integer stubs and the `P95` test, and expected result 13 lists only `Value(2)`, `Censored`, `Value(2)` and absent for `P95`. TC-624's fixture list names no AC-15 stub either. A test written from TC-624 would leave both branches untested. Fix: add the quantity-typed stub and a zero-counted quantile test to step 13, with their expected values (and the AC-15 stubs to the fixture list). | spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md:19,34,52 |
| FND-002 | low | FR-189-AC-15 says the stubbed test runs "over 4 counted samples" but names no method or confidence parameters that end the test at exactly 4. FR-189-AC-13, the precedent for a stubbed test, gives Okamoto with `α = β = 1/3` and `ι = 2/5`, so that `N = 4`. Under SPRT, or under other confidence parameters, the count depends on the thresholds, so the test author has to choose them. Fix: give AC-15 AC-13's method and parameters, or say the stub ends the test after 4 counted samples. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:249 |

## Verdict

Not mergeable as written. FND-001 is medium: two branches of the new AC-15 have no TC row. FND-002 is low. FR-189-AC-3 and FR-192-AC-6 trace cleanly to their TC rows.

## Dispositions

Disposition pass 1 at `agent-ix/quire-spec-language@936786f97` (fix commit `936786f97`, on top of the merge `9b5a81088` of main at `afffa6959`). Each outcome was re-checked against the spec at that head, not taken from the commit message or the PR body. Gates at head: `make check-spec-validation` exit 0 with no warning on a touched file, `tools/check-index-completeness.sh` exit 0; the diff touches nothing under `docs/` and no test-parsed vector table. The head does not contain the current `origin/main` (`b51907381`): two later main commits (`b51907381`, `4bd6f6287`) touch code, `Cargo.lock` and `reviews/qsl-950-*` only, so nothing in this review overlaps them.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | TC-624 fixtures now list "`BehaviourSampler` stubs giving FR-189-AC-15's samples, a `Quantity<ms>` `M` variant of them and a quantile test whose stub returns no counted sample"; step 13 runs the `Quantity<ms>` samples against `3 ms` and the no-sample test; expected result 13 adds "`Value(2)` (the magnitude in `ms`); absent for the no-sample test". AC-15 now states the quantity case with a value (`Value(2)`, not `1/500`). Recomputed by script: sorted 1, 2, 3, 5 ms, rank 2, 2 ms, magnitude 2 in `ms` and 1/500 in `s`. |
| FND-002 | fixed | AC-15 now says "(the stub ends the test after exactly 4 counted samples)", and TC-624 step 13 "from a stub that ends the test at exactly 4 counted samples", so no method or confidence parameter has to be chosen. |
