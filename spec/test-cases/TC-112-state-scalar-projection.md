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
collision-safe aliases. Materialize unchanged and changed native inputs and
inspect their provenance. Exercise direct state/captured parameters, selection,
mismatched context, frame refusal, excluded forms, exact limits, cancellation
and fresh retries. Export the checked-in example using the actual command.

## Expected Results

IR consumes real checked scalar reads with independently expected values.
Native validation precedes input materialization; stopped requests return no
partial inputs. No numeric backend execution or full graph parity is claimed.
