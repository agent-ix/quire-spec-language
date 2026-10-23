---
id: SR-16254
title: "spec-review of the restored hybrid features in quire-spec-language PR 1365 (STD-409 to STD-415)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@caf3aab8; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md; spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md; spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md; spec/decisions/ADR-026-dense-time.md; spec/test-cases/TC-1220-hybrid-initial-states-expressions-resets-and-claim-forms.md; spec/test-cases/TC-1221-hybrid-trace-replay.md; spec/test-cases/TC-705-hybrid-solver-results-map-to-verdicts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-450
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: references
---

# Spec review of the restored hybrid features in quire-spec-language PR 1365

## Summary

Tickets: STD-409 to STD-415. This is one spec-review pass over
`git diff origin/main...caf3aab8`. The oracles were QSpec #415 (head
7bbc566d: FR-193, FR-481, FR-370, FR-331 and their vectors), the merged
#400 and #1350, and the open fix-forward #1364 (head 419f1ce7).

Gates at caf3aab8: `make check-spec-validation` with QSPEC_DIR at the QSpec
#415 head exits 0, with no finding on a touched file. `cargo test -p
qsl-package --lib checked_v2` with QSPEC_DIR at the QSpec #415 head passes,
47 tests. origin/main is an ancestor of the head.

Hand derivations over `Braking` (`Moving`: `v' = -a; a' = 0; t' = 1`,
invariant `v` in `[0, 20]` and `a` in `[1, 2]`, guard `v = 0`, reset
`v = 0`; `Stopped`: invariant `v = 0`):

- FR-450-AC-2: `v(s) = 20 - 2s`, so `v(10) = 0` equals `before`, the guard
  and reset hold, and `always [0, 12] holds(h.v > 0)` is false at `t = 10`,
  so the result is `Reproduced` of kind `Formula`. Against `SlowEnough`,
  `v < 30` holds on the prefix, the rest is unknown, and there is no bad
  prefix, so the result is `NotReproduced` at `Claim`. Correct.
- FR-450-AC-3: `before` `v = 1` fails `Dwell`. `before.time` 9 gives
  `v(9) = 2`, not 0, so it also fails `Dwell`. Correct.
- FR-450-AC-4: with `a' = -1/10`, `a(t) = 2 - t/10` and
  `v(t) = 20 - 2t + t^2/20`, so `v(10) = 5` and the trace fails `Dwell`;
  `v' = -v` is cyclic, so the result is `NotExact`. Correct.
- FR-450-AC-1: a `start` box with `v` in `[25, 26]` breaks `v <= 20`, so the
  result is `Initial`. A `before` box with `v` in `[1/200, 1/100]` gives
  `|v| <= 1/100`, which is `Consistent` with `delta` 1/100 and definitely
  false without it, at `Jump`. Correct.
- FR-447-AC-10 reach: `once [2, 3]` gives `max(0, 0 - 2) = 0`, so the
  horizon is 10. `since [0, 3]` over `eventually [0, 4]` gives
  `max(0, max(0, 4 - 0)) = 4`. Unbounded `historically` and `triggered` give
  0. Correct.
- FR-447-AC-11 units: `1500 ms` is `3/2`, `0.5 s` is `1/2`, and `[0, 2]`
  gives `{0, closed, 2, closed}`. Correct.

Examined, clean: the S1/S2 `initial` and `timed-interval` rules, the S3
initial entries, the expression forms, `pre(x)` lowering and the reset
refusals, the clock, activation, capture, fairness, body, interval and
horizon rules, the checked-record identity additions and the S4 emission of
each restored form, the FR-250 and ADR-026 HY-2/HY-3 refutation path, and
FR-450's settlement rules. The diff has no deferral or narrowing language,
and the restrictions it keeps are the scope the tickets give.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-450-AC-6's altered trace fails `Jump`, not `Loop`. Take the `Bounce` trace with its last state `(Up, time 2, x = 1/2)`. Jump 1 goes from `Down` with `before` `x = 0` and reset `true`, so `x` keeps the value 0 (QSpec FR-193 "Resets"). Exact check 3 ("the reached state satisfies the reset against `before`") fails at `/jumps/1` before check 4 (`Loop`) is reached. FR-450 returns `NotReproduced` "at the first failing check". TC-1221 step 6 also expects `Loop` for this trace. Fix: expect `NotReproduced` at `Jump`, or build the `Loop` vector from a trace whose jumps all replay but whose last state differs from the loop state, for example `loop_to` 1 on this trace. | spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md:168; spec/test-cases/TC-1221-hybrid-trace-replay.md:54-55 | wrong-requirement |
| FND-002 | medium | FR-450-AC-5 expects `UndefinedEvaluation`, but FR-450's own rule order gives `Formula`. On the AC-2 trace, `1 / (h.v - 5) < 100` with `h.v - 5 = 15 - 2t` is false for `t` in `[1499/200, 15/2)`, before `v = 5` at `15/2`. So check 5 holds (the claim is false on the prefix), and FR-450 says "If every check holds, then exact replay SHALL return `Reproduced` with kind `Formula`" before the `UndefinedEvaluation` sentence. The precedence is not stated (QSpec FR-193 has the same gap, SR-16253 FND-006). Fix: state the precedence and which instant `where` names, then either keep AC-5 under a rule that prefers the undefined value, or change the claim to one that is true up to `15/2`, such as `always [0, 12] holds(1 / (h.v - 5) != 0)`. | spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md:137-143,167; spec/test-cases/TC-1221-hybrid-trace-replay.md:52 | wrong-requirement |
| FND-003 | high | Conflicts with the open fix-forward #1364. FR-447 conflicts textually (`git merge-tree`). Ids collide: #1364 adds FR-447-AC-6 (I2 read-back) and FR-447-AC-7 (profile refusal and `true`/`false` encoding), and this PR adds FR-447-AC-6 (`initial`) and FR-447-AC-7 (expressions). The semantics conflict too. #1364 requires a hybrid claim's profile to be `quire.temporal.infinite-trace/v1`, refuses any other at the `using` alias, and emits "each `HybridInterval` in the integer form". This PR emits timed intervals and makes `[q, *]` and fairness depend on the profile. #1364's I2 reader must also read back every restored form (initial entries, reset relations, `each`, captures, fairness, past operators, timed intervals), and this PR does not state that. Fix: rebase onto whichever lands first, renumber the colliding ACs, reconcile the profile rule and the interval form, and extend the I2 read-back rule and its AC to the restored forms. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:545-551 | wrong-requirement |
| FND-004 | medium | QSL has no counterpart for QSpec FR-193's empty initial set. QSpec FR-193 "Initial states" and FR-193-AC-11 settle an item whose initial set is empty `inconclusive`, `no-initial-state`. FR-250's provider outcomes (`Unsat`, `EnclosureDisjoint`, `DeltaSat`, `EnclosureMeetsUnsafe`, `Unknown`, `Stopped`) and its verdict table have no outcome or row that settles `NoInitialState`, and no QSL AC covers it. Several initial entries make an empty set reachable in source. Fix: add the provider outcome and FR-250 table row (`Inconclusive(NoInitialState)`), and an AC in FR-250 or FR-447 with a vector such as `initial { mode Moving when { v = 30 }; }` against `Moving`'s `v <= 20`. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:112-120 | missing-requirement |
| FND-005 | medium | Static replay of a `Call` has no termination rule. FR-450 evaluates `Call` "by evaluating the function's checked body over the argument intervals". FR-447 admits recursive pure functions (checked by FR-062 with their own `decreases` rule). Over interval arguments, the base-case condition of a recursive body can be undecided, so the hull of both branches recurses with no bound, and static replay may not terminate. Fix: bound interval evaluation of a recursive call, for example treat an undecided recursive call as the whole range of its result type, and add a vector. | spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md:104-111 | missing-requirement |
| FND-006 | low | `Braking2`'s source does not state the reset that FR-447-AC-12 and QSpec FR-481-AC-8 emit. The use case gives `Braking2`'s modes "(as Braking)", whose reset is `{ v = 0 }`. AC-12 then emits "`Braking2`'s transition reset `{ v = 0 and a >= 1 }`" as an aside, and QSpec's `restored_nodes` carry it. The QSpec vectors also differ from this source in the initial conditions and the capture type (SR-16253 FND-003). Fix: write `Braking2` in full in the use case, with its reset, so the source, AC-12 and the QSpec vectors are one text. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:88-101,551 | wrong-requirement |

## Verdict

Not mergeable. FND-001 is a hand-derived vector that is wrong as written.
FND-003 is the conflict with #1364, including two colliding AC ids and a
contradictory profile and interval-form rule. FND-002, FND-004 and FND-005
are real gaps. Everything else, including every other hand-derived vector,
the reset and pre-jump rules, the interval conversions, the horizon rule and
the S4 emission of the restored forms, is correct and consistent with QSpec
#415. No scope deferral was found.
