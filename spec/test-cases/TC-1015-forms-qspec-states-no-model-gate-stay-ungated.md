---
id: TC-1015
title: "Forms QSpec states no model gate for stay ungated"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: verifies
---
# TC-1015: Forms QSpec states no model gate for stay ungated

## Description

Verify that this requirement gates only the families of FR-361 item 5. Scope:
FR-361-AC-4. This catches a gate that refuses a state clause, a population,
navigation, a protocol or memory form, or a `hyper`, `relation` or state-graph
clause for lacking the model profile, or that invents a gate for a facet QSL
has no layer for.

## Test Procedure

1. Compile a unit with the header profile `quire.value.complete/v1` holding a
   `pre` clause, a `post` clause and an `invariant` over value operands.
2. Compile a unit with the same header holding a protocol form (FR-308), a
   memory clause and a fence (FR-309).
3. Compile units with the same header holding a `hyper` clause, a `relation`
   clause and a state-graph clause.

## Expected Results

None of the units raises `unsupported_construct` from the model gate. Any
other refusal they raise is the checker's own and names no missing model
profile.
