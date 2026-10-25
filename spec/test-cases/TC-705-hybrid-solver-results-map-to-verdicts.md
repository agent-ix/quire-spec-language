---
id: TC-705
title: "Hybrid solver results map to verdicts"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: verifies
---
# TC-705: Hybrid solver results map to verdicts

## Description

Verify classification of clocks and hybrid variables, routing to an FR-193 provider, and the verdict map.

Scope: FR-250-AC-1 to FR-250-AC-7.

## Test Procedure

Fixtures: a braking model with `v` of rate `-a` and a clock `t`; EN-6 registered beside an in-process fixture hybrid provider and a process fixture hybrid provider. Each descriptor has `dynamics` `["hybrid"]` and `bounds` `{max_solver_steps: 1000000, max_solver_time: 60}`; each provider can return every `SolverOutcome`, and a process provider can overrun its `deadline_ms`, which the host ends at `deadline_ms` and settles as `TimedOut`.

1. Classify the variables and route the claim, with the fixture provider registered and with none.
2. Map each fixture result.
3. For both providers, admit requests with `max_solver_time` 1.5 s, 2 s, 0.2 s and unset, and with `max_solver_steps` unset and 1,000; inspect the process provider's v2 request body, including that `package` decodes to the exact emitted package bytes; stop the provider on time with no `Cancel` raised, again with a provider `deadline_ms` overrun, and again with an FR-276 frontend deadline that fires; stop it on steps; write 1.5 as a wire `max_solver_time`, a zero budget, an unknown `hybrid.limits` member, an empty `hybrid`, and a `hybrid` with an empty `limits`; write a request that sets no budget; compare the canonical key bytes of two requests that differ only in `max_solver_time`.
4. Return an `EnclosureMeetsUnsafe` over FR-447's `Braking` with a trace from `Moving` by transition 0 to `Stopped`, and a `DeltaSat` with none; then that trace with transition 1, with `t` missing from an enclosure, and with a time interval of lower bound 3 and upper bound 2; read a plugin `solver-inconclusive` entry carrying that trace on the `Braking` claim and on an FR-468 refinement item.

5. Return `NoInitialState` over `Braking` with `initial { mode Moving when { v = 30 }; }` through each provider; for the process provider, exercise the emitted v2 package request and its `no-initial-state` result entry. Also read a plugin `no-initial-state` entry on that claim.
6. Return each method, outcome and coverage triple of FR-250-AC-6 over `SlowEnough`, and read a first-party `bounded-solver` entry with `method` `flowpipe-unrounded`.
7. Return a sound `(DeltaComplete, Unsat)` with each coverage of FR-250-AC-7 over each claim it names, an `(InductiveEnclosure, EnclosureDisjoint, Inductive)` over `Slows` and `always holds(h.v < 30)`, and read a plugin's `trusted` `bounded-solver` proof with `horizon` 1 and with `horizon` 10 on `SlowEnough`.

Tag the tests `#[trace("TC-705", "FR-250-AC-n")]`.

## Expected Results

- Step 1: `v` hybrid, `t` a clock; the item carries `hybrid-dynamics` and is routed to the fixture, not EN-6; `unsupported`, `unsupported-requested-capability` with no hybrid provider.
- Step 2: the rows of FR-250's table, with no `refuted`.
- Step 3: solver budgets 2, 2, 1 and 60 s, the run using the rounded value; the process provider's request body carries the exact emitted v2 package bytes as unpadded base64url and the same explicit `hybrid.limits` values as the in-process request; the time stop and the `deadline_ms` overrun are `incomplete` with cause `TimedOut`, the frontend deadline stop `Cancelled{source: deadline}`, the wire 1.5, the zero budget, the unknown member, the empty `hybrid` and the empty `limits` are each refused by the provider before consumption, the unset budgets are omitted from the item, and the stored record holds no wall-time member; the key bytes are equal; the step stop is `LimitReached{max_solver_steps, 1000, hybrid.max_solver_steps}`, the default 1,000,000.
- Step 4: `inconclusive`, `SolverInconclusive{trace: Some(..)}` equal to the fixture's trace, with its FR-450 static replay outcome and no `counterexamples` entry; `SolverInconclusive{trace: None}`; each malformed trace refused `invalid_result`/`invalid-value` at the member. The plugin entry reads `SolverInconclusive{trace: Some(..)}` on the `Braking` claim and is `Malformed` at `/cause/trace` on the refinement item.
- Step 5: `inconclusive`, `NoInitialState`, never `Proved`, with no `counterexamples` entry, from both fixture providers and through the plugin reader; the process request reader admits the decoded emitted package, binds the result index and claim occurrence to the checked hybrid claim, then reads the provider result entry.
- Step 6: the records FR-250-AC-6 states, with their basis bytes; the first-party `flowpipe-unrounded` entry refused `invalid_result`/`invalid-value` at `/basis/method`.
- Step 7: `Proved` or `Inconclusive(InsufficientHorizon{horizon, jumps, exhaustive})` as FR-250-AC-7 states, the inductive results `Proved{InductiveSolver{..}}`, the plugin proofs `InsufficientHorizon{1, 1, {Time}}` and `proved`, `trusted`; no trace and no `counterexamples` entry on an `InsufficientHorizon` record.
