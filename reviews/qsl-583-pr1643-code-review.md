---
id: SR-1643001
title: "code-review and rust-review of PR #1643 (QSL-583b, FR-240 zone liveness and FR-241 lassos)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1643 git diff origin/qsl-583-zone-liveness...d853a40f8: qsl-analyze/src/zone_check/{liveness.rs,lasso.rs,concretise.rs,search.rs,time_lock.rs}, qsl-analyze/src/model_check/{fair_walk.rs,mod.rs}, qsl-eval/src/automaton/{translate.rs,mod.rs}, qsl-analyze/tests/it/zone_liveness.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-240
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-241
    type: reviews
---
# SR-1643001: code-review and rust-review of PR #1643 (QSL-583b, FR-240 zone liveness and FR-241 lassos)

## Summary

Ticket: QSL-583. Head: d853a40f8. The review was led by soundness, checked with probes in the review worktree that were reverted afterwards.

**A wrong `Holds` under fairness (FND-001, high).** FR-232 and the replay count a step as enabled only when its target satisfies its time invariants. The product's fairness enabledness ignores the target's invariants. `enabled_at` tests only the guard parts, and the cells are split only by guard constants (`atoms_of`).

Repro (dense, clock `x` never reset):
- **Model:** `serve` moves phase 0 to phase 1 and sets `retried`; `time invariant when phase = 1 { x <= 1 }`.
- **Claim:** `fair weak serve; eventually holds(c.retried)` (and the same under `fair strong serve`).
- **What happens:** after `x > 1`, `serve` is disabled forever, so the idle tail at phase 0 is weakly and strongly fair and violates the claim.
- **Independent oracle:** FR-237's replay, given the idle-tail payload with the fair clause's fairness set, returns `Reproduced` under both kinds.
- **EN-6 result:** `Holds` under both.

A step-loop variant (`ping` resets `y` each unit, with `serve`'s target needing `x <= 1`) wrongly gives `Undecided(NoAdmittedBehaviour)`.

**What I checked and found correct:**

- **Probes against a hand-derived truth.** Every one of these returned the right verdict, and every lasso replayed to `Reproduced`:

  | Probe | Results |
  | --- | --- |
  | `serve` at the instant `x = 1`, with `ping` resetting `x` from `x >= 1` | strong Holds; weak and none Violated |
  | the same with `ping` only below 1 | Violated under strong |
  | an unsatisfiable `ping` guard | `NoAdmittedBehaviour` |
  | idle tail with `serve` from `x >= 2` | weak and strong Holds; none Violated as a terminal-stutter lasso |
  | `serve` enabled on `[1, 2]` each round | weak Violated; strong Holds |
  | Zeno-only model | `NoAdmittedBehaviour` |
  | alternating phases | `always eventually` Holds; `eventually always` Violated; `always phase = 0` Violated; response Holds |
  | two clocks, `serve` needing a free-running `y >= 2` | weak Holds; none Violated, stem extended past the ceiling |
  | tick source | weak Holds; none Violated (a tick and ping lasso); `tick` only after `serve` Holds |

- **Holds certificate.** FR-245's checker rebuilds the item from the recompiled clause. It accepts only a TT-1 atom, a TT-2 `BoundedWindow` or deadlock freedom, and refuses every other clause as `UnsupportedItem`. So a liveness `Holds` cannot settle `proved` before 583c, as claimed.

- **Fourier–Motzkin (`lasso.rs`).** It is exact over the rationals:
  - **Elimination:** combining `b·upper + a·lower` with `a, b > 0` cancels the variable, and the result is strict when either side is.
  - **Back-substitution:** each level holds every projected constraint, so a value chosen for the earlier variables always leaves a non-empty interval.
  - **FR-241's rule on prefix sums:** least value when the lower end is closed, otherwise the midpoint, otherwise lower plus one. This is the delay rule shifted by `P_{k-1}`.
  - **Cyclic reset reading:** `delays_between` covers the full loop when the clock's last reset is its own position.
  - **Never-reset clocks:** an upper bound or equality on one is infeasible, and a lower bound holds.
  - **Limits:** the `MAX_CONSTRAINTS` cap is reported as infeasibility (FND-003).

- **`translate_position_negation` against the timed evaluator.** I ran nine interval-free bodies on every lasso of up to 4 positions over 3 levels: 426 lassos each, all agreeing with `evaluate_timed_at`, with both verdicts occurring. The bodies were `eventually`, `always`, `always eventually`, `eventually always`, response, `until`, `release`, `once` under `always`, and a disjunction. `next` is not admitted under the timed profile.

- **Vacuity.** A claim with no fairness relies on FR-239's time-divergence test. With fairness, the universal-automaton product must have a fair accepting component or an idle tail. The idle tail counts even when it is unfair, which is STD-493 fd83efcd item 3 and follows the text.

- **Idle tails.** Enabledness is read with every cell atom exceeded, which is the steady state that both weak (eventually always) and strong (infinitely often) fairness need, apart from FND-001.

- **Mutual aM identification.** With `L = U = M`, the LU relation is "equal or both above M", a time-preserving bisimulation, and the claim reads only discrete letters. So merging mutually covered zones keeps the fair accepting cycles.

Gates at d853a40f8:
- `zone_` tests: 74/74 passed.
- clippy `-p qsl-analyze -p qsl-eval --all-targets --all-features -D warnings`: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The product reads fairness enabledness from the guard alone. `enabled_at` ignores the step's target time invariants, and `atoms_of` splits cells by guard constants only. FR-232 says a step is enabled only when its target satisfies its time invariants, and FR-237's replay reads it that way. Repro: `serve` goes to a phase whose invariant `x <= 1` holds with `x` never reset. Under `fair weak serve` or `fair strong serve`, `eventually holds(c.retried)` returns `Holds`, but the idle tail at phase 0 is fair (serve is disabled once `x > 1`), violates the claim, and replays to `Reproduced` under the same fairness. A loop variant wrongly returns `NoAdmittedBehaviour`. Fix: in both `enabled_at` and `atoms_of`, conjoin each part with the target invariants on the clocks the step does not reset, drop the step when a reset value breaks a target bound (as `Run::enabled` does), and add those constants as cell atoms. Add the repro as a regression test. | qsl-analyze/src/zone_check/liveness.rs:511-546,831-857 |
| FND-002 | low | FR-240 "Vacuity" admits the subject through any reached idle tail, even one that is unfair under the clause's fairness. `admits_fair` returns true on any idle vertex before the fair analysis runs. That follows the text and is filed (STD-493 fd83efcd item 3). But it is the direction that lets a proof stand over no fair behaviour. Once the spec rules, run the idle vertices through the same fair analysis. | qsl-analyze/src/zone_check/liveness.rs:313-327 |
| FND-003 | low | `solve` reports exceeding `MAX_CONSTRAINTS` as `NotConcretised::Infeasible`, so a budget stop reads as an unsatisfiable loop, and the run can end `Undecided(LassoNotConcretized)` instead of a V-7 limit. Give the cap its own error and map it to a limit stop (or a named fault). | qsl-analyze/src/zone_check/lasso.rs:347-352 |

## Verdict

**Not mergeable** until FND-001 is fixed. A fair, time-divergent behaviour violates the claim, and the engine returns `Holds`. Today a liveness `Holds` settles inconclusive, because FR-245's checker refuses it until 583c. But 583c's component certificate would inherit the same enabledness, and EN-6's own verdict is already wrong. FND-002 and FND-003 are low.

Rust lane: no `unwrap`, `expect` or panicking index in the new production paths, apart from indexing guarded by construction (`vertices[edge.from]`). Every vertex and edge is charged to the search budgets (`charge_vertex`, `charge_edge`), and the kernels honour cancellation. The `fair_walk` visibility widenings to `pub(crate)` are minimal.

## Coverage

Examined:
- FR-240 Behavior (liveness phase, fairness over time, divergence, idle tail, vacuity);
- FR-241 Behavior (loop delays, canonical rule);
- FR-232 "Fairness over time" (context);
- FR-236 "Timed lasso" (context);
- STD-493 comment fd83efcd.

Gap analysis is SR-1643002.

## Dispositions

Round 1, head f3d9e57a9. It is not restacked: f3d9e57a9 is the fix commit on top of d853a40f8.

I checked each fix in a fresh detached worktree and target, under locked-build at jobs 4:
- `zone_` tests: 75/75 passed.
- qsl-eval `timed_trace` tests: 14/14 passed.
- qsl-analyze `lasso` unit tests: 3/3 passed.
- clippy `-p qsl-analyze -p qsl-eval --all-targets --all-features -D warnings`: exit 0.

I re-ran every round-0 liveness probe, then reverted them. All verdicts are unchanged, and every lasso replays. I also ran two new target-invariant probes:
- **Reset breaking the target bound:** `serve` resets `x` to 2 into a phase holding `x <= 1`. With `serve` never enabled, the result is Violated with an idle tail, under weak fairness and with none.
- **Enabled on `[2, 3]` only:** `serve` is guarded `x >= 2` into a phase holding `x <= 3`. The result is Violated with an idle tail under weak and strong fairness.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `Run::enabling_parts` conjoins each guard part with the target time invariants on the clocks the step does not reset, and returns no part when a reset value breaks a target bound. `enabled_at`, `atoms_of` (cell atoms) and `Run::enabled` all use it. The repro is now `enabledness_reads_the_target_time_invariants`: idle-tail and `ping`-loop variants, under weak and strong fairness, each Violated and replayed to `Reproduced`. The changed vacuity fixture keeps its intent. `serve` now resets `x` into phase 1 (`x <= 0`), so its target holds at every instant, and the `ping` loops stay unfair under `fair weak serve`. The old fixture had relied on the bug: without the reset, `serve` would be enabled only at `x = 0`. |
| FND-002 | deferred | The behaviour is kept, and FR-240 "Vacuity" as written supports it: a proof is barred only when the initial state "reaches no fair time-divergent cycle and no quiescent state". The question is filed on STD-493 (comment fd83efcd, item 3), and the code now cites it. |
| FND-003 | fixed | `MAX_CONSTRAINTS` is gone. The elimination is capped by the run's remaining `max_symbolic_transitions` (`transitions_left`), and passing the cap returns `NotConcretised::Exhausted`, which the liveness phase turns into `Stopped(LimitReached(MaxSymbolicTransitions))`. The unit test `an_elimination_past_its_cap_is_exhausted_not_infeasible` covers it. |
