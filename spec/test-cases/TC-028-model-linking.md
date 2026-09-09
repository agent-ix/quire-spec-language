---
id: TC-028
title: "Ambiguous scalar inference"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-006
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: verifies
---

## Description

Ambiguous scalar inference. Type: Integration; priority P1. Traces: FR-006-AC-4.
Planned, not executed. Requires the qualified inputs and actual native APIs
specified by IT-005; a setup refusal is not an observed application outcome.

## Test Procedure

Under the separately qualified rule-model declarations, check the pinned ambiguous-literals expression 1 + 2 = 3 and size-needs-context expression size(self.items) = 2. Compare size-preserves-duplicates, which supplies the explicit compatible Count context.

## Expected Results

The context-free cases return ill_typed. The explicit compatible Count context establishes Boolean typing without choosing an arbitrary numeric type. No hypothetical collection evaluation outcome is counted as a typechecker observation.
