---
id: TC-1081
title: "The monitor builds from the plan alone, refuses altered plans, isolates instances, agrees with the reference evaluator and fails closed"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: verifies
---
# TC-1081: The monitor builds from the plan alone, refuses altered plans, isolates instances, agrees with the reference evaluator and fails closed

## Description

Verify that the executable monitor plan of FR-251 is self-sufficient, that admission refuses any plan not derived from the selected clause and target, that built monitors do not share state, that executing the plan agrees with QSL's reference evaluator, and that every build, admission and execution fault is a typed non-verdict result.

Scope: FR-251-AC-4 to FR-251-AC-12, FR-251-AC-14, FR-251-AC-15, FR-251-AC-17, FR-251-AC-19, FR-251-AC-21, FR-251-AC-22, FR-251-AC-24, FR-251-AC-25 and FR-251-AC-26.

## Test Procedure

Fixtures: the deadline claim of FR-251-AC-1 on a 20-bit 1 MHz counter with its admitted plan; a second clause and target for the substitution candidates; a clause with reversed implication operands; a clause whose activation guard is false; clauses with open and closed interval endpoints; a predicate the selected backend cannot execute.

1. Derive and admit the plan, drop the producer and preparation objects, then build a monitor from the plan alone, with the source text replaced by poisoned bytes that fail the test if read, no clause argument supplied, and atom-evaluator callbacks and an RT temporal-translation hook installed that fail the test if invoked. Inspect the plan's program. Remove one transition body from a copy and submit it for admission.
2. For each of an atom, an operand reference, an entry body, the `init` body, an interval association, a buffer owner, an activation binding, the uncertainty and the maximum gap, replace it alone in a copy of the plan with the value from the second clause and target, keeping the original selection, and submit each for admission. Give two subjects the same numeric subformula IDs and resolve each ID against the other. Derive the plan for the second target and admit it.
3. Build two monitors from one plan and feed a `req` event to the first only. Attempt to rebind each live monitor to the second plan. Read the result of each monitor.
4. Run the plan's transition entries and the reference evaluator on the same admitted inputs for the traces of FR-251-AC-1 to FR-251-AC-3, the reversed-operand clause, the false-activation clause and the open and closed endpoint clauses. Then assert progress past a deadline, and separately poll the counter without an assertion.
5. Plan the unsupported predicate. Fail state allocation during build. Submit an event missing a required admitted value. Trigger an executor invariant failure, then submit another input.
6. Read the admitted plan-root node of AC-1: list its entry nodes and their signatures, the `State` type and the capacity premises; change each entry body in turn and compare plan identity; read the initial state and a created obligation's origin.
7. On the 16-bit target of FR-251-AC-3, feed the readings `65530` then `4` and `65530` then `65500`, recording each entry call; then admit a watermark at history boundary tick 3001 and a bare counter poll for AC-1's claim.
8. On the 16-bit target of FR-251-AC-3, keep one instance open across a fault and one decided before it, and start one after it; feed the first reading, then `65530`, `65500`, `65510`, recording each entry call and its inputs.
9. Build the AC-1 plan with a per-step budget too small for one `on_event`; present a `req` event and then an admitted watermark and read the monitor state after each, then raise the budget (or build another monitor with a sufficient budget from the same plan) and re-present the same event and the same watermark, counting executions of the transition body.
10. Build the AC-1 plan with `activation_capacity` 2; start three `req` instances before any `ack`; present watermarks and then the `ack`s; read the root outcomes and the per-instance reports.
11. On the 16-bit target of FR-251-AC-3, fill the window of `once[0 ms, 10 ms] holds(p)`, cause a fault, evaluate before and after the window refills, and poll the counter between readings.
12. Run an "on each" clause over a model-typed value with three admitted events (two in one tick); read each instance's activation position and captures from `State`; add an atom over a model value with no admitted projection.
13. Build `once[0 ms, 10 ms] eventually holds(ack)` with `activation_capacity` 2; present events at ticks 0, 1000 and 2000 with no `ack`; read each instance's `subformula` (the `eventually` node's id).
14. On a 1 MHz counter at rate 2 per ms, build `once[0 ms, 10 ms] eventually[0 ms, 5 ms] holds(p)` and `eventually[0 ms, 5 ms] once[0 ms, 10 ms] holds(p)`; read each operator's capacity; feed `p` at tick 0 and 30 events at the rate across ticks 0 to 14999; read the inner buffer at tick 14999 and the decisions that need the tick-0 entry.
15. Plan `once holds(p)` and `historically holds(p)` at rate 2 per ms on a 1 MHz counter with uncertainty 0; feed `p` true at tick 0, an event at tick 5000, and `p` false at tick 100 for `historically`; read each buffer and the results (AC-22).
16. On the 16-bit target, feed `p` true before a gap fault and read `once holds(p)` and `historically holds(p)` after it, then feed `p` after it (AC-24).
17. Plan `always eventually holds(p)` and `eventually always holds(p)` with `activation_capacity` 2; start three positions before any `p`, then feed `p` (AC-25).
18. Derive the plan of AC-1 and variants that differ only in an atom body, the guard body or a called helper's body; build `TimedMonitorPlan` through its public constructor with an atom body swapped (AC-26).

Tag the tests `#[trace("TC-1081", "FR-251-AC-n")]`.

## Expected Results

- Step 1: the monitor builds from the plan alone. The program holds both atom bodies, the ordered implication, eventually and always topology, the activation and capture bindings, the state initializers and the executable event and watermark transitions. The poisoned source is never read, no clause argument is needed, and no callback or translation hook is invoked. The copy without a transition body refuses admission, and no initialization runs and no monitor exists.
- Step 2: every substituted copy refuses admission with no initialization. Foreign IDs with matching numbers do not resolve. The plan for the second target is admitted as a distinct plan.
- Step 3: the second monitor has no obligation from the first monitor's `req`. Both rebind attempts fail and neither monitor's state changes. Each result retains its subject, target premises and obligation origin.
- Step 4: the per-obligation outcomes are identical. The progress assertion settles the deadline per QSpec FR-094/FR-160; the poll adds no event and settles no deadline.
- Step 5: `Unsupported` and no monitor; a build failure and no monitor; the event is rejected and state is unchanged; `Failed` with no verdict, and the later input is refused. No step returns `false` as a property result.
- Step 6: function entries `init`, `on_event`, `on_watermark` and `on_gap` with the stated signatures and no statement form; each body change gives a different plan identity; the initial state is the `init` result and the origin is the unwrapped tick of the activating event.
- Step 7: delta 10 passed and unwrapped time advances by 10; the second pair calls `on_gap` with 65506 and not `on_event` (the open instance fails `ReadingGapExceeded`, Step 8); the watermark settles the deadline false; the poll calls no entry.
- Step 8: the first reading passes delta 0; the fault calls `on_gap` with 65506 and not `on_event`; only the open instance fails with `ReadingGapExceeded` and is dropped; the decided and later instances are unaffected; `65510` passes delta 10.
- Step 9: the first presentation returns `StepResourceStop` (a typed incomplete outcome, no verdict, no refusal), the state is byte-for-byte unchanged and the event is unconsumed (likewise the watermark); the re-presented input runs the body once and gives the outcome an unstopped step gives; the monitor accepts the next input.
- Step 10: the third live instance is not started and settles `Incomplete(LimitReached{limit: activation, value: 2, setting: monitor.max_activations})`, none dropped silently; the root gives one folded outcome per activation, pending until resolved; each `eventually` instance is reported once on settlement and each pending one at every watermark.
- Step 11: buffers empty and `history_start` at the faulting reading's unwrapped tick; `indeterminate` until the window refills, then decided; the poll leaves the baseline unchanged.
- Step 12: activation positions 1, 2, 3 from `State.event_position`, read back as `status.position` of the three `activations` records, no `Activation` field; captures are typed projections; the unprojectable atom yields `Incomplete(LimitReached{limit: atom_projection, value: <its AtomId>, setting: monitor.atom_projection})` with replayable evidence.
- Step 13: the third instance is not started and settles `Incomplete(LimitReached{limit: activation, value: 2, setting: monitor.max_activations})`; the plan root has no additional binding.
- Step 14: inner capacity 32 in both plans (retention 15002 ticks), outer capacity 22 for the `once` and 12 for the `eventually`; the tick-0 entry is present at tick 14999 and read, no `Incomplete`, and it expires only after tick 15002.
- Step 15: capacity 2, buffer at most 2 entries and the summary cell; the tick-0 entry is folded at tick 5000, `once holds(p)` is `true`, `historically holds(p)` is `false`.
- Step 16: `once` `indeterminate` with `p` only before the gap and `true` after `p`; `historically` `indeterminate`, never `true`, then `false` on a false after the gap; each result retains the subformula and `history_start`.
- Step 17: planned, no history buffer; the third instance settles `Incomplete(LimitReached{limit: activation, value: 2, setting: monitor.max_activations})`; the first `p` settles the two live instances `true`; the root stays `Pending` on the open trace.
- Step 18: each variant has a different `PlanId` and plan-root key; the equal-bodied plan an equal one; the helper's call identity is unchanged.
