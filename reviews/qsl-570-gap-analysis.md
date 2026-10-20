---
id: SR-12216
title: "Gap analysis of QSL-570 E17c part 1 EN-4 statistical decide against FR-189 to FR-191 and QSpec FR-408 (PR #1079)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@149fdd4b70d4ea10512bbde90c055f1d23032618; spec/functional/FR-189-*.md, spec/functional/FR-190-*.md, spec/functional/FR-191-*.md, spec/test-cases/TC-624-*.md, spec/test-cases/TC-625-*.md, spec/test-cases/TC-626-*.md, spec/test-cases/TC-640-*.md; qsl-analyze/src/statistical.rs, qsl-analyze/src/statistical/*.rs, qsl-replay/src/statistical.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-190
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-191
    type: reviews
---
# SR-12216: Gap analysis of QSL-570 E17c part 1 EN-4 statistical decide against FR-189 to FR-191 and QSpec FR-408 (PR #1079)

## Summary

Ticket: QSL-570 (E17c), part 1 of 2. PR quire-spec-language#1079 at head 149fdd4b7. The audit covers the diff, the FR-189, FR-190 and FR-191 ACs, the QSpec FR-408 bounds and AC vectors, and TC-624, TC-625, TC-626 and TC-640. Plan completion was not assessed. QSpec FR-408 was read from a local read-only checkout and is cited by id only.

`quoin matrix` (quoin 0.28.3) reports every FR-189, FR-190 and FR-191 AC as `tagged`. The narrow tests pass: 30 in qsl-analyze `statistical`, 2 in qsl-replay `statistical`.

Criterion coverage:

- **FR-189-AC-1 (examined).**
  - Covered: P95 with Okamoto accepts at 5 ms with `N = 23,026`, half-width `1/100` and lower end `≥ 19/20`, and rejects at 2 ms with upper end `< 19/20`. SPRT accepts at 5 ms and rejects at 2 ms, each under 23,026 samples.
  - Not covered: "each of at most 11 positions" is a property of the model sampler, which is declared PR 2 work.
- **FR-189-AC-2 (examined).** Covered: `N` is 23,026 and 26,492, `T± = ±ln 100` outward within `2^-64`, and two runs are equal.
- **FR-189-AC-3 (examined).**
  - Covered: `m = 2`, `α′ = β′ = 1/200`, trace `2i + j`.
  - Not covered: the symmetry clause (`m = 1` under an invariant workload, `m = 2` otherwise). PR 2 owns it.
- **FR-189-AC-4 (examined).**
  - Covered: activation gives `samples = 23,026` with `draws > samples`. The censored and strict `M < 3`, `< 1/2` reductions are checked at the `MeasureEvent` and `Bound` level.
- **FR-189-AC-5 (examined).**
  - Covered: `N = 9,210,341`, `UndecidedSuccessor`, and `NotStatistical` for a mean with SPRT.
  - `NotMarkov` is shown only through a fixed fake. The `Health` model is PR 2 work.
- **FR-189-AC-6, TC-640 (examined).** Covered on a `Coin` fake that draws `b` with `weighted_choice`: rejected at the first `v = 1` trace, no further draw, test 0, trace, position and `division-by-zero`.
- **FR-189-AC-7 (examined).** Covered: `Unsupported(TotalAboveDrawRange)` with total `2^256 + 1`, at the first sample.
- **FR-190-AC-1 (examined).** Covered: LongRun is Accepted, the estimate is within `1/5000` of `1800/1801`, the lower end is `≥ 999/1000`, the run stops at the first `n ≥ 100` recomputed from the recorded cycles, coverage is asymptotic, `<= 0.9995` is Undecided, and two runs are equal.
- **FR-190-AC-2 and QSpec FR-408-AC-12 (examined).** Covered: the transient start stops on `max_cycle_steps` with no decision. `min_cycles` works as a floor. The degenerate walk stops at exactly 11,632.
- **FR-190-AC-3 (examined).** Covered: the `duration` weights are 1 and 3, and the method/form pairs are refused `NotStatistical`.
- **FR-191-AC-1 to AC-4 (examined).**
  - Covered: the `max_samples` stop at 1,000,000 and completion at 9,210,341; the 2^24 default; `max_draws` 1,000; cancellation; the SPRT rerun equality; the 0–99 prefix equality; seed 8; and per-test provenance.
  - Not covered: the obligation-identity clauses of AC-3 and AC-4, which are PR 2 work.
- **QSpec FR-408 bounds (examined).**
  - The `ln`, `√`, `π`, `Φ` and quantile methods are implemented as FR-408 states, and all of them round outward (SR-12215).
  - The `2^-64` precision is not met for small `δ` (quantile) or large `|k|` (`ln`). See SR-12215 FND-001 and FND-002.

The PR has no stubs and no untested production branch on the main paths. The PR body's "Remaining work for PR 2" section lists the model-level remainder.

No major finding. FND-001 to FND-003 are medium.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | PR #1079 is part 1 of QSL-570, but its Linear attachment has `linkKind: closes`. So does #1082. Merging this PR would move QSL-570 to Done while the declared PR 2 remainder is still open: the model `BehaviourSampler`, symmetry, obligation identity and the `NotMarkov` payload. Fix: the lead keeps QSL-570 open after this merge, or moves the remainder to its own ticket. | QSL-570 attachments (PR #1079, #1082); PR body "Remaining work for PR 2" |
| FND-002 | medium | Some tests are tagged with whole ACs whose clauses they only partly cover, so the matrix shows these ACs as `tagged` although clauses are untested. FR-189-AC-3 lacks the symmetry clause. FR-191-AC-3 and AC-4 lack obligation identity. FR-189-AC-1 lacks the "at most 11 positions" clause. FR-189-AC-5 shows `NotMarkov` only through a fixed fake, not the `Health` model. The PR body declares all of this, but nothing in the matrix or the ticket keeps it from reading as done. Fix: split the compound ACs (part-1 clauses and part-2 clauses), or record the uncovered clauses on QSL-570 so PR 2 must tag them. | qsl-analyze/src/statistical/tests.rs:279,394,1047,1103 |
| FND-003 | medium | QSpec FR-408-AC-7's clause "a sum or half-width within the rounding error of a threshold gives undecided" has no test. Every interval-rule and SPRT test sits far from the threshold. The `2^-64` precision of the quantile is not asserted either: its test checks only `< 10^-12`, which hid SR-12215 FND-001. Fix: add a boundary test, for example an SPRT whose exact sum equals `ln(β′)` within the bound width, or a half-width within `2^-64` of `ι`, asserting Undecided or continue. Tighten the quantile assertion to `2^-64`. | qsl-analyze/src/statistical/method.rs:192-253; qsl-analyze/src/statistical/bounds.rs:378-389 |
| FND-004 | low | The code adds refusals and a budget reading that FR-189 to FR-191 do not state. FR-189's `StatisticalRefusal` lists none of `NoTest`, `TooManyTests`, `TraceIndexRange`, `ConfidenceBeyondPrecision`, `FractionForEvent` or `Internal`. The regenerative method counts walk steps against `max_draws`, while FR-191 defines `max_draws` as samples drawn. Each is a sound meaning, but a second implementation would not know it. Fix: state them in FR-189 and FR-191 (PR #1082 may already cover some). | qsl-replay/src/statistical.rs:423-471,175-177; qsl-analyze/src/statistical.rs:460-462 |
| FND-005 | low | FR-191-AC-2's third clause (the transient variant stopping on `max_cycle_steps` and naming `statistical.max_cycle_steps`) is asserted only in the FR-190-AC-2 test, which is not tagged `FR-191-AC-2`. Fix: add `FR-191-AC-2` to that test's `#[trace]`. | qsl-analyze/src/statistical/tests.rs:852-885 |

## Verdict

**PASS: mergeable. No major finding.** Every part-1 clause of FR-189 to FR-191 is backed by a test that can fail. The QSpec FR-408 vectors (`N` = 23,026, 26,492 and 9,210,341; the SPRT thresholds; 11,632 cycles) are asserted exactly. The engine is sound, rounding outward everywhere.

The medium findings are about bookkeeping and test strength. FND-001 is ticket closure. FND-002 is tags on compound ACs whose part-2 clauses are untested. FND-003 is a missing boundary-rounding test and a weak precision assertion. They can be fixed in this PR or tracked on QSL-570.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | FR-189-AC-10 asks for the least-favourable rule to be shown "through the `BehaviourSampler` seam, with the SPRT method". The test calls `least_favourable` directly on built `TestResult`s. It covers every case in the AC, and `check_statistical` takes the verdict's `estimate` and `interval` from the same function, so the gap is only the AC's stated route. Fix: drive one or two of the cases through `check_statistical` with a `ByInitial` sampler, or reword the AC. | qsl-analyze/src/statistical/tests.rs:1411-1467 |
| FND-007 | low | After SR-12215 FND-003, the code refuses `TraceIndexRange` only for Okamoto and SPRT. FR-189's refusal list, merged in #1082, still says `TraceIndexRange` applies "when `max_draws · m` overflows the trace index", with no method restriction. Fix: add "for the Okamoto and SPRT methods" to FR-189. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:93-95; qsl-analyze/src/statistical.rs:643-652 |

## Dispositions

Round 1, reviewed at a9a693ac0e88a414cd5c067f016507356549657c (fix commit fda45c590, plus #1082 merged from main). No gates were run.

Code was checked against the merged FR-189 to FR-191 text:

- The least-favourable margin rule.
- `estimate` and `interval` presence.
- `NoTest`.
- After an undefined sample, finished tests are kept and unstarted ones are absent.
- A quantile test is not decided before its first counted sample.
- `max_samples` stops before the next draw, and a method that decides exactly at `max_samples` completes.
- `Stopped` and `Unsupported` carry provenance; a refusal carries none.

All of these match. FND-006 and FND-007 are new low findings from this round. They are open, do not block, and go to the lead for PR 2.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | deferred | Ticket closure is the lead's action at merge, not a code change. #1082 merged with `linkKind: closes` and QSL-570 stayed In Progress. #1079 (part 1) and #1098 still carry `closes`, so the lead must re-check QSL-570's state after this merge while PR 2 is open. |
| FND-002 | fixed | fda45c590 |
| FND-003 | fixed | fda45c590 |
| FND-004 | deferred | Half fixed by #1082 (e9a24af94): FR-189 now lists `NoTest`, `TooManyTests`, `TraceIndexRange`, `ConfidenceBeyondPrecision`, `FractionForEvent` and `Internal`. Still unstated: the regenerative method counting walk steps against `max_draws`, which FR-191 defines as samples drawn. That is spec text only, low, and goes to PR 2's spec fit on QSL-570. |
| FND-005 | fixed | fda45c590 |
