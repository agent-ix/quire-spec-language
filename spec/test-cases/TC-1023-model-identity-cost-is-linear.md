---
id: TC-1023
title: "Model identity cost is linear in types plus edges"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
  - target: ix://agent-ix/quire-spec-language/NFR-012
    type: verifies
---
# TC-1023: Model identity cost is linear in types plus edges

## Description

Verify FR-081-AC-11: hashed bytes, retained facts and work units follow closed
formulas linear in effective types and direct edges.

## Test Procedure

Implement as the Rust integration test
`tests/it/model_identity_cost.rs::cost_follows_closed_formulas`.

1. Generate, in package `test/orders` with nodes named `T<ordinal>`: a chain of
   `n` edges for `n` in 10, 100 and 1000; a width-2 diamond ladder of 500
   layers where each type has both types of the layer below as supertypes; and
   a shape where each of 1,000 types has the previous 10 types as supertypes.
2. Normalize each at the NFR-012 defaults and read the counters: bytes admitted
   to every identity, universe and view hash call, retained facts and work units.
3. Compute the expected hashed bytes independently: twice the sum of the
   preimage lengths, plus the view header, plus 154 bytes of frame per entry
   and one separator between entries; a preimage is 106 bytes plus its key, a
   qualify fact of 96 bytes plus its key, and per parent a separator, 217
   bytes and the parent key, one more byte per ordinal digit past the first.
4. Compare facts (types plus edges) and work units (records, facts, declarations,
   hashes, one cycle-check per edge) with the counters.

## Expected Results

Hashed bytes, facts and work units equal the formulas exactly. The width-2
ladder, whose path count is exponential in its layers, completes at the
defaults. A shape where every type has 50 parents over 10,001 types is refused
`resource_exhausted` at `hashed_bytes` before hashing the denied preimage.
