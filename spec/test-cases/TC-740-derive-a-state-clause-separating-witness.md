---
id: TC-740
title: "S6a stop reports and the decision path derive a state clause's basis and witness"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-265
    type: verifies
---
# TC-740: S6a stop reports and the decision path derive a state clause's basis and witness

## Description

Verify FR-265's derivation over FR-265's `witness` unit: a decisive
`forall` or `exists` on the decision path yields one record with its
quantifier, element, index and value path; every other path yields
`closed-scope`; an incomplete evaluation yields no basis.

Scope: FR-265-AC-1 to FR-265-AC-7.

## Test Procedure

Compile FR-265's `witness` unit against FR-108's `Config` domain package.
Admit FR-265's `low` and `high` snapshots by FR-106 for each invariant
(`self` `child` in `low`, `mid` in `high`, unless a step says otherwise).
Evaluate each clause once by FR-107 and pass the evaluation, its stop
reports and the observation to the derivation.

1. `AllBelow` over `high` and over `low`; count the element visits of the
   `high` evaluation.
2. `SomeAtLeast` over `high` and `low`; `NotAllBelow` over `high`;
   `EqualsAll` over `high`.
3. `GuardedAll` over `high` with `self` `mid`, then with `self` `root`;
   `LetAll` over `high`; `OrAll` over `high`.
4. `NestedAll` over `high`.
5. `FilteredAll` and `MappedAll` over `high`.
6. `AllBelow` over `high` with the evaluation budget at zero; `NoneSelected`
   over `low`.
7. Run FR-265's member clauses through `run_clause` with `self` `mid`:
   `MemberAll` with `history` `[0, 600, 700]`; `ConvertedAll` with
   `history` `[700, 0, 600]`; `MemberPre` over a pre-call observation with
   `history` `[0, 600, 700]`, and over an invocation whose pre and post
   snapshots hold that `history`.

Tag the tests `#[trace("TC-740", "FR-265-AC-n")]`.

## Expected Results

- Step 1: `decisive-counterexample`, record (`AllBelow`'s `forall`
  occurrence key, 600, index 1, a path rooted at the built list ending in
  index step 1, no trace position) for `high`, with two element visits;
  `closed-scope` and no record for `low`.
- Step 2: `decisive-witness` with 600 at index 1 naming the `exists`, then
  `closed-scope` with no record; `decisive-witness` naming `NotAllBelow`'s
  `forall`; `closed-scope` with no record for `EqualsAll`.
- Step 3: a record for `GuardedAll`'s `forall` naming the reference to
  `mid` at index 1, then
  `closed-scope`; a record for `LetAll`'s `forall`; `closed-scope` and no
  record for `OrAll`.
- Step 4: one record naming the outer `forall`, element 0, index 0; no
  inner occurrence appears.
- Step 5: element 600, index 2 (`FilteredAll`); element 601, index 1
  (`MappedAll`); each path rooted at a `built` subject naming the list's
  occurrence key and ending in an index step for its index.
- Step 6: `incomplete`, no basis, no record; `closed-scope`, no record.
- Step 7: 600 at index 1 on the path `mid` / member `history` in the current
  observation; 600 at index 2 on the same path; for `MemberPre`, equal
  records in both runs, 600 at index 1, the path naming the pre
  observation.
