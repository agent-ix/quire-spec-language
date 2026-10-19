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

Scope: FR-251-AC-4 to FR-251-AC-8.

## Test Procedure

Fixtures: the deadline claim of FR-251-AC-1 on a 20-bit 1 MHz counter with its admitted plan; a second clause and target for the substitution candidates; a clause with reversed implication operands; a clause whose activation guard is false; clauses with open and closed interval endpoints; a predicate the selected backend cannot execute.

1. Derive and admit the plan, drop the producer and preparation objects, then build a monitor from the plan alone, with the source text replaced by poisoned bytes that fail the test if read, no clause argument supplied, and atom-evaluator callbacks and an RT temporal-translation hook installed that fail the test if invoked. Inspect the plan's program. Remove one transition body from a copy and submit it for admission.
2. For each of an atom, an operand reference, a transition body, a state initializer, an interval association, a buffer owner, an activation binding, the uncertainty and the maximum gap, replace it alone in a copy of the plan with the value from the second clause and target, keeping the original selection, and submit each for admission. Give two subjects the same numeric subformula IDs and resolve each ID against the other. Derive the plan for the second target and admit it.
3. Build two monitors from one plan and feed a `req` event to the first only. Attempt to rebind each live monitor to the second plan. Read the result of each monitor.
4. Run the plan's transition entries and the reference evaluator on the same admitted inputs for the traces of FR-251-AC-1 to FR-251-AC-3, the reversed-operand clause, the false-activation clause and the open and closed endpoint clauses. Then assert progress past a deadline, and separately poll the counter without an assertion.
5. Plan the unsupported predicate. Fail state allocation during build. Submit an event missing a required admitted value. Trigger an executor invariant failure, then submit another input.

Tag the tests `#[trace("TC-1081", "FR-251-AC-n")]`.

## Expected Results

- Step 1: the monitor builds from the plan alone. The program holds both atom bodies, the ordered implication, eventually and always topology, the activation and capture bindings, the state initializers and the executable event and watermark transitions. The poisoned source is never read, no clause argument is needed, and no callback or translation hook is invoked. The copy without a transition body refuses admission, and no initialization runs and no monitor exists.
- Step 2: every substituted copy refuses admission with no initialization. Foreign IDs with matching numbers do not resolve. The plan for the second target is admitted as a distinct plan.
- Step 3: the second monitor has no obligation from the first monitor's `req`. Both rebind attempts fail and neither monitor's state changes. Each result retains its subject, target premises and obligation origin.
- Step 4: the per-obligation outcomes are identical. The progress assertion settles the deadline per QSpec FR-094/FR-160; the poll adds no event and settles no deadline.
- Step 5: `Unsupported` and no monitor; a build failure and no monitor; the event is rejected and state is unchanged; `Failed` with no verdict, and the later input is refused. No step returns `false` as a property result.
