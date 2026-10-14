---
id: TC-1024
title: "Model identity cost is linear in types plus edges"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
  - target: ix://agent-ix/quire-spec-language/NFR-012
    type: verifies
---
# TC-1024: Model identity cost is linear in types plus edges

## Description

Verify FR-081-AC-11: hashed bytes, retained facts and work units follow closed
formulas linear in effective types and direct edges.

## Test Procedure

Implement as the Rust integration test
`tests/it/model_identity_cost.rs::cost_follows_closed_formulas`.

1. Generate, in package `test/orders` with nodes named `T<ordinal>`: a chain of
   `n` edges for `n` in 10, 100 and 1000; a width-2 diamond ladder of 500
   layers where each type has both types of the layer below as supertypes; and
   a shape where each of 1,000 types has the previous 10 types as supertypes;
   and the three N13 shapes, with nodes named `T00` upward and `T00a`, `T00b`
   upward for the ladder.
2. Normalize each at the NFR-012 defaults and read the counters: bytes admitted
   to every identity, universe and view hash call, retained facts and work units.
3. Compute the expected hashed bytes independently, from the shapes the
   quire-specification `TC-195` N13 and N19 rows measure. Let `k` be the JCS
   length of a declaration key: 83 bytes for `T0`, one more per extra digit of
   the node name, so 83 to 87 bytes in this package. A type's preimage is 106
   bytes plus its key, plus its qualify fact of 96 bytes plus the key, plus,
   per parent, a separator, 217 bytes and the parent's key, one more byte per
   ordinal digit past the first. Hashed bytes are twice the sum of the
   preimage lengths, plus the 264-byte view header, plus 154 bytes of frame per
   entry and one separator between entries, plus one object universe preimage
   per connected component: 335 bytes with one root type, 125 bytes and a
   separator more for each further root type.
4. Compare facts (types plus edges) and work units (records, facts, declarations,
   hashes, one cycle-check per edge, so `4V + 2E + 2` for `V` types and `E`
   direct edges with one universe) with the counters. The 100-type chain
   `T00` to `T99` is 99 edges, 199 retained facts, 149,894 hashed bytes and 600
   work units; the 50-layer width-2 ladder is 100 types, 196 edges, 296
   facts, 209,400 hashed bytes (its two roots add 126 bytes of universe) and
   794 work units; thirty types each generalizing the previous five (or fewer)
   are 135 edges, 165 facts, 108,988 hashed bytes and 392 work units.
5. Normalize `F2` of quire-specification `TC-195` plus fields `B/x2`, `C/x3`
   and `D/x4` with `B/x2.redefines = A/x` and `D/x4.redefines = A/x`, and
   `C/x3` not redefining (the N12 case): 13 effective declarations and 24
   retained facts. Work units are 76: 8 records, 24 facts, 4 cycle-checks, 13
   declarations and 15 hashes make 64, the `normalize.redefinition-check` of
   `B/x2` (`m` = 2, `r` = 0) and `D/x4` (`m` = 4, `r` = 1) add 7, and the
   `normalize.conflict-check` on `(D, A/x)`, reached by `c` = 2 redefiners
   with `f(B) + f(D)` = 2 + 3 = 5, adds 5.

## Expected Results

Hashed bytes, facts and work units equal the formulas exactly. The width-2
ladder, whose path count is exponential in its layers, completes at the
defaults. A shape where every type has 50 parents over 10,001 types (about 313
million hashed bytes) is refused `resource_exhausted` at `hashed_bytes` before
hashing the denied preimage. Its counters equal the quire-specification
figures where the shapes coincide.
