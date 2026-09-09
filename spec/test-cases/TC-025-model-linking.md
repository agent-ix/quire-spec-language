---
id: TC-025
title: "Unguarded optional unwrap"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-006
    type: verifies
---

## Description

Unguarded optional unwrap. Type: Integration; priority P1. Traces: FR-006-AC-1.
Planned, not executed. Requires the qualified inputs and actual native APIs
specified by IT-005; a setup refusal is not an observed application outcome.

## Test Procedure

Use IT-005's separately qualified rule-model declarations matching the pinned typing-cases hypotheses. Submit unguarded-unwrap, right-guard-cannot-protect-left and alternative-join-does-not-union-facts to the real native checker. Compare with absent-parent-implication and conditional-guard under the same model.

## Expected Results

The unguarded/control failures return undefined_expression at the relevant expression; the properly guarded cases establish Boolean typing. Refused expressions cannot proceed to evaluation. The test compares static judgments, not the fixture's hypothetical evaluation values.
