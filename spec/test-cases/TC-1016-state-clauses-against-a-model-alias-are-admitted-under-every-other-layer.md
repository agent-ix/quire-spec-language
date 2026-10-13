---
id: TC-1016
title: "State clauses against a model alias are admitted under every other layer"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: verifies
---
# TC-1016: State clauses against a model alias are admitted under every other layer

## Description

Verify that the `model` declaration is ungated and that state clauses written against its alias are admitted by the four layers other than the model layer. Scope: FR-361-AC-5.

## Test Procedure

1. For each header profile `quire.state.core/v1`, `quire.state.queries/v1`, `quire.state.graph/v1` and `quire.value.complete/v1`, compile a unit with a `model` declaration, an object type `A` with a field and an operation, and one `invariant`, one `pre` and one `post` against them, using only that layer's forms.

## Expected Results

1. Each unit compiles with no model-gate refusal.
