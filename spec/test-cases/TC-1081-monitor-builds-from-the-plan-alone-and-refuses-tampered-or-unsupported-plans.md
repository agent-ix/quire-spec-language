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

Scope: FR-251-AC-4 to FR-251-AC-12, FR-251-AC-14, FR-251-AC-15, FR-251-AC-17, FR-251-AC-19, FR-251-AC-21, FR-251-AC-23 to FR-251-AC-25, FR-251-AC-27 to FR-251-AC-30, FR-251-AC-32 to FR-251-AC-39, FR-251-AC-41, FR-251-AC-42, FR-251-AC-43 through FR-251-AC-48.

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
15. At rate 2 per ms on a 1 MHz counter with uncertainty 0: `once holds(p)` with `p` true at tick 0 and an event at tick 5000, `historically holds(p)` with `p` true at 0 and false at 100; read each buffer and result (AC-23, AC-24).
16. At the start of a trace: `once holds(p)` with `p` never true, `historically holds(p)` with `p` always true, then one witness each (AC-25).
17. After the overflow of TC-706 step 5: `once holds(p)` with `p` true at the lost event and no later `p`, then with a later `p`; `historically holds(p)` with `p` false at the lost event and true afterwards, then with a later false (AC-27).
18. On the 16-bit target: `once` and `historically` with `p` true before a gap and never after, and with `p` true or false after it (AC-28).
19. `p since q` and `p triggered q` over the traces of AC-29, and over the gap traces of AC-30 (AC-29, AC-30).
20. `historically (holds(req) implies eventually[0 ms, 5 ms] holds(ack))` read its capacity; feed `req` at tick 0 with no `ack`, then with `ack` at 2998 (AC-32).
21. `historically (holds(req) implies eventually holds(ack))` with `activation_capacity` 1; two `req` events before any `ack` (AC-33).
22. `once[5 ms, *] holds(p)` read its capacity; `p` true at tick 0; read the result at ticks 4000 and 6000 (AC-34).
23. `always eventually holds(p)` and `eventually always holds(p)` with `activation_capacity` 2 over the traces of AC-35 and AC-36.
24. Derive the plan of AC-1 and variants differing only in an atom body, the guard body, or a called helper's body; build `TimedMonitorPlan` through its public constructor with an atom body swapped (AC-37, AC-38, AC-39).
25. Run the AC-7 differential over the unbounded past traces of steps 15 to 19 against the `HistoryCutoff` evaluator and the infinite-trace evaluator (AC-41).
26. Write the plan-root package over an emission holding rows of every kind (selected model declaration and field, source-unit, model enum declaration and member, tuple declaration, operation-member, equality producer and fact, temporal binding, omitted nodes) with a rename of every node id those rows name, and read every id field of every returned row; write the plan-root package of a unit that selects a model, and read the returned `Emission`'s rows and join its exact rows to the package read back through IR.
27. Write the plan-root package over a unit with two temporal clauses once for each clause, read it back through the strict reader, and select each subject's clause node and claim ordinal through the returned `Emission`'s temporal binding rows; repeat through a row carry that renames a clause declaration, which no current writer path does; select an ordinal the package does not hold for each clause and a clause node it does not hold (AC-43).
28. Write the plan-root package of AC-1's unit, read it back through the strict reader, and join the returned `Emission`'s source record rows of `Instance`, `Activation` and `LimitReached` through Contract IR's source record join; repeat with each renamed member's `admitted_name` set to its declared field name (AC-44).
29. Produce a tick-monitor plan from genuine authored source and an independently derived and strict-read observation-owner progress association. Through the public handoff compare the complete canonical model-step order and owner association VALUE with their original owner values; compare the original source, structural clause node, claim role/ordinal, selected subject and checked model-time facts. Join through the same final emission temporal binding rows and compare its clause binding with the strict reader declaration. Drop temporary compilation inputs and repeat public reads of values and capabilities alongside the plan and obligations. Exercise owner-permitted clone and writer-rename behavior while retaining all old witness rows (AC-45).
30. Independently remove the checked order premise and progress association; substitute a foreign emission; change the original source, structural clause node, claim role and ordinal, selected subject, observation source, clock identity/revision, model, structural model/unit/dimension/digital-operation node, unit, dense/tick kind, digital operation and exact period. Substitute the complete order and association values and the full authority document bytes. Invoke public admission and assert each typed refusal with no partial admitted context. Encode the genuine owner association at its exact observed byte length and one byte less; inspect the original owner resource refusal. Do not fabricate an admitted authority, encode a structural node as a digest, or infer association from labels, clock configuration or numeric positions (AC-46).
31. Drop the original authored source, model documents and request inputs after producing a plan root. Read its retained combined checked unit through the public API; compare the authored source prefix, generated entry names, original selected clause key, model object key, dense source kind and millisecond unit. Separately join the final clause through the emission retained rows, without treating original checker keys as final renamed identities or progress authority (AC-47).
32. Read the public canonical model-step order value from genuine dense and digital model-time roots. Compare the entire object with the checked model, unit and dimension identities; for digital time compare the checked operation anchor and exact period. Change source kind, time-source unit and digital period independently. Encode at the exact observed byte length and one byte less; inspect the original canonical limit refusal. The plan-root accessors preserve immutable ownership of the original plan, obligations and emission (AC-48).

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
- Step 15: the tick-0 entry folds at tick 5000; `once holds(p)` is `true`; `historically holds(p)` is `false`; the buffer holds the tick-5000 entry and the cell.
- Step 16: `indeterminate` (`MissingHistory`), `indeterminate`; then `true` and `false`.
- Step 17: `indeterminate`, then `true`; `indeterminate`, then `false`; never a wrong definite verdict.
- Step 18: `indeterminate` and `indeterminate`; then `true`, `false` as stated; each result retains the subformula and `history_start`.
- Step 19: `since` true and false as AC-29 states, `triggered` `false` and `indeterminate`; after a gap `indeterminate`, `true`, `false` as AC-30 states.
- Step 20: capacity 12; `Pending` at tick 4000 and `false` once progress closes the window; with `ack` the entry folds true and the result stays `indeterminate`.
- Step 21: the second instance settles `Incomplete(LimitReached{limit: activation, value: 1, setting: monitor.max_activations})`; the operator stays `Pending`, never `true`.
- Step 22: capacity 12; `indeterminate` at tick 4000, `true` at tick 6000.
- Step 23: the third instance of each settles `Incomplete(LimitReached{limit: activation, value: 2, setting: monitor.max_activations})`; the first `p` settles the two `eventually` instances `true`; for `eventually always` no instance settles true on a `p` and a false `p` settles both `false`, the outer staying `Pending`.
- Step 24: each variant has a different `PlanId`, the helper variant also a different plan-root key with the helper's `node_id` unchanged; equal bodies give an equal `PlanId`.
- Step 25: monitor and `HistoryCutoff` evaluator equal on every trace; every definite monitor verdict equals the infinite-trace verdict; `indeterminate` verdicts are not compared.
- Step 26: every row kind is carried and non-empty where the acquisition had rows; each node id in each row equals the renamed id, and a row naming no renamed node is unchanged; the declaration and field rows equal the unit's acquisition rows, the source-unit and equality rows each name a node of the package, and the IR join accepts the exact rows.
- Step 27: each selection returns the declaration the plan root's `clause` binding names and the declaration of the `temporal_clause_names` row of that ordinal, never the other clause's; after the rename the selection returns the renamed id and no id absent from the package; the absent ordinal and an absent clause node each select nothing.
- Step 28: the rows name `origin` and `value` at the renamed members and keep `origin_tick` and `amount` as declared names; the join succeeds and projects each record; each row with a declared name as `admitted_name` is refused with `MismatchedIdentityWitness`.
- Step 29: both complete owner values retain the original structural/source/claim components; the same-emission temporal join equals the plan-root and strict-reader declaration. The values and capabilities survive temporary input destruction, follow the owner clone/rename contract and retain every older witness row.
- Step 30: each independently changed or missing selection returns its typed refusal and no partial context. Exact canonical-association byte bounds succeed; one byte less preserves the owner resource refusal. No digest, label, full authority document, clock configuration, matching index or second emission supplies missing authority.
- Step 31: the original combined checked unit retains all compared source/time facts after input destruction; the separate final-emission selection equals the strict reader clause. This is component retention evidence and grants no admitted order/progress claim.
- Step 32: the complete canonical values retain every specified component; source kind, unit and period changes give unequal values. Exact-length encoding succeeds; one less returns Encoding with the native canonical byte limit. The returned plan, obligations and emission remain immutable.

## Qualification boundary

The procedure defines required future evidence; this specification change grants
no implementation or qualification credit. Positive joint-authority steps remain
planned until the observation owner supplies the structural node and original
claim-occurrence association contract (IR-1390). An owner API that accepts only
64-hex node strings does not admit QSL's complete structural identities; no
conversion to a fabricated digest or claim string is permitted.

Component controls independently change the observation source, clock identity,
clock revision, model, unit, dense/tick kind, operation, period numerator and
denominator against genuine checked source facts. These controls provide no
positive joint-authority credit until the original structural node and claim
occurrence comparisons and owner-issued association test also pass.
