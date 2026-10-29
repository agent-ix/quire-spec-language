---
id: SR-1702001
title: "Code review of quire-spec-language PR #1702: timed refinement liveness on zones (QSL-1005d)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f7bb1931c; PR #1702 diff against its merge base with qsl-1005c-trap-undo (2fd0cedfb): qsl-analyze/src/zone_check/{refine.rs,refine/live.rs,liveness.rs,zone_check.rs}; qsl-replay/src/model_check/{refinement_replay.rs,replay/timed.rs}; tests timed_refinement_liveness"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-430
    type: reviews
---
# Code review of quire-spec-language PR #1702

## Summary

Ticket: QSL-1005, slice 1005d. The Rust review lane is folded into this
review. I reviewed only this PR's diff at f7bb1931c, taken from its merge
base with `qsl-1005c-trap-undo` (2fd0cedfb). The specs are FR-430-AC-5 (as
corrected by #1701), FR-143, FR-241, ADR-026 TS-4/EZ-6/EZ-8 and TC-1170.

Gates: `cargo test -p qsl-analyze -p qsl-replay --all-features` passed at
f7bb1931c (EXIT=0). That covers 1,003 tests in `it`, including every
`zone_*` liveness suite unchanged, plus 90 lib tests and all of qsl-replay.
I did not run clippy.

The five soundness questions from the brief:

1. **Only positive-total-delay loops refute.** A cycle is accepted only
   through the `divergent` acceptance set, which needs a reset of `z`. That
   reset needs `z >= 1` (unit scale), and z's aM bound is that unit. FR-241
   then requires `D > 0` for a dense source when it solves the loop
   (`solve_loop` pushes `-D < 0`). So no zero-delay or Zeno loop refutes,
   which `a_zero_delay_loop_does_not_refute` and its mutation check confirm.
   I found no way a real divergent violation is missed:
   - every time-divergent run passes a `z` reset;
   - delays enter every next cell;
   - steps that `check_step` fails are dropped only because safety holds,
     so their post-states are time-locked;
   - quiescent states get the idle tail;
   - a fair component that does not concretise gives
     `Undecided(LassoNotConcretized)`, never `Holds`.
2. **Shortest stem.** The roll-back pops the stem's last arc while that arc
   has the same `via` (step and part) and the same target vertex as the
   loop's last arc, from a positioned source with equal state, `q` and
   cell. A `Stage` is built from the source and target state and cell only,
   and the cached `moved` for (q, state, step) gives the same `taken`. So
   the rotated loop has the same stages, abstract steps and F_C enabledness
   (cells decide it exactly). FR-241 then concretises the whole lasso
   again. The violation is preserved.
3. **Abstract side.** Abstract fairness classes and enabledness come from
   `abstract_system.expand_state` over each position's candidates, the same
   reading as the untimed FR-143 (`model_check/refine.rs`).
4. **`liveness.rs` refactor.**
   - `Cells` takes over the `constrained`/`atoms` pair unchanged.
   - `enabled_at` now takes a zone.
   - The loop copy keeps every field except `fixed`; `stage(.., Some(d))`
     only set `fixed`, so the copy is equivalent.
   - The empty-stem branch, which used to be `Fault("lasso-stem")`, now
     concretises a loop that enters at the initial state when it resets
     every clock to 0. Otherwise it falls through to the later copy counts,
     which carry an entry constraint.
   - No existing `zone_*` verdict changed: all those suites pass.
5. **Replay.** `timed_prefix` closes a lasso's loop with FR-237's
   `close_loop` (positive total delay, clocks close). With the delay set to
   0, the lasso is refused `LoopZeroDelay` (test 4). The lasso settlement
   reuses the untimed checks: drift, F_C fairness, and the class enabled
   throughout and never taken.

Local probes, not committed:
- **P1:** adding `assume fair weak M::Call::send` gives liveness `HoldsReduced`, because the unguarded `send` loop is F_C-unfair.
- **P2:** a fair weak `tick`, guarded `x <= 0 ms`, gives `Violated` with a loop that takes `tick`, and replay gives `Reproduced`.

## Verdict

**No soundness defect.** One low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The liveness half's `stats` give the safety product's counts. `Run::stats` reads `self.nodes` and `self.edges` from the safety search, while liveness vertices and arcs are charged only through `escape_state`/`escape_edge`. So `TimedLivenessOutcome::{HoldsReduced, Violated, ...}.stats.symbolic_states` is the safety graph's size: 2 for probe P1, although the liveness product is larger. Report the liveness product's own counts, or the sum, and say which. | qsl-analyze/src/zone_check/search.rs:913-923; qsl-analyze/src/zone_check/refine/live.rs:844-888 |
