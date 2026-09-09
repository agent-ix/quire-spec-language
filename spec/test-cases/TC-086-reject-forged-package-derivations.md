---
id: TC-086
title: "Reject forged package derivations"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: verifies
---
# TC-086: Reject forged package derivations

## Description

Property, priority P1. Verifies FR-020-AC-7, FR-020-AC-11. Planned; no implementation or execution is claimed. Setup uses actual admitted models and compiler APIs before the target boundary.

## Test Procedure

Mutate one validly encoded resolution target, span, root ExprId, runtime context/observation/universe/operation/frame flag, model artifact content or projection claim at a time; omit a real known feature or add an unused known feature. Keep externally selected source/models/bindings valid and update the raw package selector.

## Expected Results

Complete regeneration catches every forged/omitted claim. An outer byte digest or successful wire decode never substitutes for correspondence. Semantic comparison retains meaningful ordered arrays and exact model artifact strings.
