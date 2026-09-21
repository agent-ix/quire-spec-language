---
id: TC-215
title: "A dominated redefinition is retained for provenance, not deleted"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-215: A dominated redefinition is retained for provenance, not deleted

## Description

Verify that when two redefinitions of the same member reach one effective
type along a chain, the more specific redefiner is the one a name resolves
to, but the less specific redefiner's own entry remains present in the
correspondence rather than being erased. Scope: FR-081-AC-3.

Catches an implementation that, once it decides which redefinition wins,
discards the loser's derivation entirely (for example, by overwriting a
`HashMap` slot keyed only by the effective member rather than appending to a
per-member list) — a shape that would still make every ordinary "which
redefinition applies" test pass, since the winner is correct, while quietly
destroying the provenance trail the ticket's "original/effective provenance"
exit condition exists to keep.

## Test Procedure

1. Admit a domain package with a three-level chain: object type `A` declares
   field `label`; object type `B` (`supertypes: [A]`) redefines `label`;
   object type `C` (`supertypes: [B]`) also redefines `label`, both
   redefinitions independently declaring `redefines: label` against their
   nearest visible ancestor.
2. Run the model binder's normalization.
3. Read the effective `label` member the correspondence resolves for `C`'s
   effective type, and confirm it derives from `C`'s own redefinition.
4. Separately query the correspondence for `B`'s redefinition of `label` as
   it reaches `C`'s effective type.
5. Assert `B`'s redefinition still has a readable entry: its own original
   declaration key, its derivation facts and its own effective identity are
   all present, and the entry is marked as not the declaration a name
   resolves to.

## Expected Results

Step 3 resolves to `C`'s redefinition. Step 5 finds `B`'s redefinition's
entry intact and marked non-resolving, not absent. An implementation that
deletes or overwrites the losing entry fails step 5 by returning nothing (or
an error) for a query that should find `B`'s retained provenance.
