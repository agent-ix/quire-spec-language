---
id: TC-112
title: "Bind state fields and materialize validated primitive inputs"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-034
    type: verifies
---
## Description

Integration, P1: project the concrete ConfigVersion update rule and supply
its primitive inputs through actual native validation and the strict IR readers.

## Test Procedure

Compare bound pre/post references, exact model/source/field correspondence and
collision-safe aliases, requiring the expected field and direct-read counts
before inspecting filtered observations. Materialize unchanged and changed native inputs and
inspect their provenance. Exercise direct state/captured parameters, selection,
mismatched context, frame refusal, excluded forms, exact limits, cancellation
and fresh retries. Export the checked-in example using the actual command.

Use real model values named nativeField0 through nativeField15 to require
seventeen fresh alias candidates; test the exact combined node/candidate ceiling,
exhaustion during candidate search, one-short whole-package work and a fresh retry.
Keep Boolean and signed-integer literal/negation controls across all three targets.
Exercise invariant parameter access, current selection for a postcondition, missing
selected invocation and missing declared parameter through the public linker and
validator. Compare each actual typed predecessor cause and input location; use a
successfully validated invocation to check the captured primitive and independently
counted materialization work.

## Expected Results

IR consumes real checked scalar reads with independently expected values.
Native validation precedes input materialization; stopped requests return no
partial inputs. No numeric backend execution or full graph parity is claimed.
Invalid predecessor inputs yield no ValidatedContext. Post-validation missing
correspondence has a typed invariant reason and original read provenance; tests
do not fabricate a validated context to reach those defensive internal paths.
ContextMismatch, ResourceExhausted and Cancelled remain caller-reachable
materialization stops with completed work and no partial inputs.
