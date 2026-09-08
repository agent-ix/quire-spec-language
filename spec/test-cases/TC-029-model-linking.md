---
id: TC-029
title: "Clause roots are Boolean"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-006
    type: verifies
---

## Description

Clause roots are Boolean. Type: Integration; priority P1. Traces: FR-006-AC-5.
Planned, not executed. Requires the qualified inputs and actual native APIs
specified by IT-005; a setup refusal is not an observed application outcome.

## Test Procedure

Under the qualified rule model, check a current invariant whose entire expression is self.n. Compare a current Boolean comparison self.n = self.n and the pinned Boolean operation-result postcondition under its proper operation context.

## Expected Results

The numeric root returns ill_typed even though self.n resolves and is individually well typed. The two Boolean-root controls establish Boolean clause typing. No rejected root becomes a reference-evaluable clause.
