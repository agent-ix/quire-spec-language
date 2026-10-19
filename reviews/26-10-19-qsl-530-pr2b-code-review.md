---
id: SR-13298
title: "Code review of quire-spec-language PR #1045: QSL-530 E9, cross-binding liveness ranking (FR-126)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4be524040; PR #1045 (git diff origin/main...HEAD, merge base ed2831ea1): qsl-analyze/src/model_check/liveness.rs, qsl-analyze/tests/it/example_units.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
---
# SR-13298: Code review of quire-spec-language PR #1045

## Summary

Ticket: QSL-530 (E9, PR 2b). Code review with the rust-review lane folded
in. The PR changes `Run::decide` in `liveness.rs` to rank bindings as
FR-126 "Bindings and ranking" (amended by STD-203, #1036) says:

- The first phase runs over every binding before any second phase.
- A finding at depth `d` caps each later binding's product horizon at
  `d - 1`, and its model-only pass bound at `d`. A finding at depth 0 stops
  the loop.
- With no first-phase finding, the shortest lasso across bindings wins. A
  tie keeps the earlier binding.
- The model-only pass no longer creates or charges a root or successor at
  or beyond its bound.

Checks run at 4be524040 in a fresh clone, with my own `CARGO_TARGET_DIR`
(not `make ci`):

- `cargo fmt --all -- --check`: clean.
- `cargo clippy -p qsl-analyze --all-targets -- -D warnings`: clean.
- `cargo test -p qsl-analyze --test it example_units`: 11 of 11 pass.
- `cargo test -p qsl-analyze`: all pass (9 unit tests, 132 integration tests
  in `it`, doc tests).

I measured the engine with probe tests in that scratch clone. They were
never pushed. The probes swept `max_states`, `max_transitions` and
`max_depth`, and an env-gated trace printed each binding's cap, charges and
hits.

- **AC-15 holds on the real engine, for the stated reason.** With versions
  (0,1), `a`'s model-only pass charges exactly 6 states and finds the
  undefined letter at depth 2. `b` then starts with cap 1 and no budget left,
  and its product stops: `Stopped{MaxStates, 6}`. `max_states` 7 and 8 also
  stop in `b`. From 9 up, and at 1,000, the result is `b` undefined at
  position 1 with prefix `upd(b)`, 3 product states. With versions (0,0) at
  1,000, the result is `a` at position 2 with 0 product states. `max_states`
  1 to 5 stop inside `a`'s own pass.
- **The depth-0 case.** For `always eventually holds(6 mod (2 - v) >= 0)`
  with `a` at 2, every budget from 1 up returns `a` at position 0 with 1
  product state, so `b` is not explored.
- **AC-10.** The result is bound to `b`, length 3, loop on `a`, 30 product
  states. Versions (0,2) and (1,0) bind to `a`, so the length tie goes to
  the earlier binding.
- **ADR-018 §6.** The weak `each` run still reports 30 product states, 78
  edges and 2 automaton states, and ends `ComponentCertificateNotBuilt`.
- **Determinism.** Eight requests run twice each gave byte-equal `Debug`
  output.

The rest is sound:

- `best` is replaced without comparing against it. That is correct only
  because the cap keeps every later finding strictly shallower: undefined
  letters and contract findings at depth `cap` are read, and nothing deeper
  is created.
- `depth - 1` cannot underflow, because depth 0 breaks the loop first.
- `is_none_or` with strict `<` keeps the earlier binding on a tie.
- No panic, `unwrap` or `unsafe` was added outside tests.
- There is no compatibility shim. `run` now takes `&dyn ModelLetters`, a
  test-local change.

## Verdict

**Not mergeable yet: two medium findings in the budget-limited path.** On
an unconstrained budget, ranking, the cap, depth 0, the shortest-lasso rule
and the shared budget all behave as FR-126 says. The findings are about
runs where the shared budget runs out:

- FND-001: a binding whose own exploration stops still returns its finding,
  and later bindings are never ranked against it. Measured, this returns a
  depth-2 violation bound to `a` while `b` holds a depth-0 one.
- FND-002: the first phase does not end at its finding. The product is
  explored to completion first, so the budget that FR-126 leaves to later
  bindings goes to states past the finding.

The coverage gaps are in SR-13299, the gap analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | When a binding's product stops (`Ending::Stopped`) or its model-only pass stops, with `best` still `None` and a first-phase hit in hand, `decide` returns that hit at once (`Some((_, hit)) if best.is_none() => hit.outcome(..)`). It skips the later bindings. FR-126 says that once a binding's first phase ends with a finding at depth `d`, each later binding is explored through `d - 1`, and the engine returns the `Stopped` of a later binding that stops before completing that depth. Measured on `always eventually holds(6 mod (2 - c.versionNumber) >= 0)` with WHOLE fairness: versions (0,1) with `max_states` 4 to 8 (or `max_transitions` 3 to 11) return `Violated` bound to `a`, undefined at position 2. A larger budget returns `b` at position 1. Versions (0,2) with `max_states` 4 to 8 return `a` at position 2, though `b` is undefined at position 0 (`max_depth` 0 already returns `b@0`). So the outcome is a deeper, non-canonical counterexample, and it changes with the budget. A partial graph can also miss a shallower contract finding on a depth `d - 1` state that was never expanded. Fix: record the hit as `best` and go on to the later bindings with the cap. The next binding then stops on the spent budget and returns its `Stopped`, as FR-126 says. Return the hit only when no later binding remains or `d` is 0, and only if every state shallower than `d` was expanded. Add a test for this case. | qsl-analyze/src/model_check/liveness.rs:751-759, 769-774 |
| FND-002 | medium | FR-126 says the engine "SHALL end the first phase at the first product state in FR-101 canonical breadth-first order whose letter is undefined or whose monitor state rejects". `explore` instead runs `retain` over the whole product, and `first_phase` only scans the finished graph afterwards. The budget spent on states past the finding is then gone when later bindings need it, under the cross-binding rule this PR adds. Measured, versions (0,1): `a`'s finding sits inside its first 4 product states (`max_states` 4 already returns it), and `b` needs 3 more to reach depth 1. But `a` is explored to all 9 states, so `max_states` 9 to 11 return `Stopped` where FR-126 gives `b`'s finding, and 7 and 8 hit FND-001. Fix: when a product state is created with an undefined letter, or an expansion records an undefined or refused contract, stop the exploration once the BFS level that holds it is complete, so the first finding is still canonical. Then charge only what was explored. A TP-4 run needs the full graph only when no binding has a first-phase finding. | qsl-analyze/src/model_check/liveness.rs:889-903, 994-1041 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | When a binding's product stops while it holds a "canonical" first-phase hit at depth `d`, the new branch returns that hit (`depth == 0 \|\| last`) or makes it `best` and `continue`s. In both cases the binding's own model-only pass never runs. `canonical` checks only that every product state shallower than `d` was expanded. The dead roots recorded below `d` are ignored, though FR-126 ranks the pass's findings with the product's by depth. Reproduced in a scratch probe. The letters are over the joint state: for binding `b`, the automaton dies at (1,0) and (0,2), and the atom is undefined at (2,0) and (2,1). For `a`, there is no finding. An uncancelled run returns `b` undefined at position 2 (prefix `a a`), from the pass. If `b`'s letters cancel the run while reading (1,2), after the depth-3 live hit was created, the result is `Violated` bound to `b` at position 3 (prefix `b a a`). That is a deeper, non-canonical counterexample where FR-126 gives the depth-2 one, or `Stopped{Cancelled}`. The `continue` path has the same hole when the stop does not stop the later bindings, for example an expansion meter stop. Fix: in that branch, count the binding canonical only when no dead root lies below `d`. Otherwise run its model-only pass with bound `d` before ranking, and return `Stopped` if the pass cannot finish. Add a test for this case. | qsl-analyze/src/model_check/liveness.rs:778-803 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 38a4f05c6: a binding that stops returns its `Stopped`, unless it holds a canonical hit. That hit becomes `best`, and the later bindings run under `d - 1` and stop on the spent budget. A pass that stops returns its `Stopped`. Measured at 7333749a2 with an oracle sweep: 18 configurations (two claims, versions {0,1,2}^2), `max_states` and `max_transitions` 0 to 30 each, 1,116 runs. Every run returned either `Stopped` or the unlimited answer, and the threshold was monotone. Versions (0,2) with `max_states` 4 to 8 now stop instead of returning `a@2`. The binding's skipped model-only pass is a separate hole, FND-003. |
| FND-002 | fixed | 38a4f05c6: the product sets `found` when it meets a finding and creates no state at or past that depth, and the pass stops creating states after an undefined letter. With the TP-4 mod claim and versions (0,1), the threshold dropped from 12 to 8 states (from 14 to 6 transitions), and `max_states` 9 to 11 now return `b@1`. The ADR-018 §6 weak `each` run still explores 30 product states. |
| FND-003 | fixed | 78945d64f: in the stopped-binding branch, `decide` now runs the binding's model-only pass with bound `d` before it ranks the finding. If the pass stops, the run returns `Stopped`. A shallower pass hit replaces the product hit. The new test `a_stopped_binding_does_not_return_a_finding_its_own_pass_may_beat` encodes the probe: unlimited gives `b@2`, cancelled gives `Stopped{Cancelled}`. I checked that it fails against 7333749a2's `liveness.rs` (it panics at the `Stopped{Cancelled}` assertion) and passes at 78945d64f. A re-run of the oracle at 78945d64f found no wrong or non-monotone result in 1,116 runs, nor in 242 more runs on the hidden-pass world (thresholds: 17 states, 23 transitions; every run below a threshold stops, and every run at or above it gives `b@2`). |
