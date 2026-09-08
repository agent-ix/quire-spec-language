---
id: TC-027
title: "Guarded bounded addition"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-006
    type: verifies
---

## Description

Guarded bounded addition. Type: Integration; priority P1. Traces: FR-006-AC-3.
Planned, not executed. Requires the qualified inputs and actual native APIs
specified by IT-005; a setup refusal is not an observed application outcome.

## Test Procedure

Use a qualified Version declaration with exact inclusive bounds 0..1000 and dimensionless unit, matching guarded-addition in the pinned rule cases. Check self.n < 1000 implies self.n + 1 <= 1000. Compare the unproved-overflow expression and guards at the finite boundary, including <= 1000 which still permits overflow.

## Expected Results

The guarded-addition expression establishes Boolean typing with its arithmetic definedness obligation discharged. Omitting or weakening the boundary guard returns undefined_expression. No model maximum is inferred from example runtime values; this test executes static typing only.
