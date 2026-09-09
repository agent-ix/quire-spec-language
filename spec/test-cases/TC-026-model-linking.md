---
id: TC-026
title: "Presence facts stay with their observation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-006
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: verifies
---

## Description

Presence facts stay with their observation. Type: Integration; priority P1. Traces: FR-006-AC-2.
Planned, not executed. Requires the qualified inputs and actual native APIs
specified by IT-005; a setup refusal is not an observed application outcome.

## Test Procedure

Check the pinned pre-guard-cannot-protect-post and captured-post-value-not-retagged cases under the qualified postcondition context. Compare with same-pre-value-guard and pre-preserves-captured-alias. Keep exact binding/source/anchor identities in each request.

## Expected Results

A presence fact for a different observed value cannot discharge the unwrap: the adverse cases return undefined_expression. The two same-value controls establish Boolean typing. Captured aliases retain the observation of their value; source spelling pre(...) does not retag it.
