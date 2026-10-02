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

Scope: FR-265-AC-1 to FR-265-AC-6.

## Test Procedure

Compile FR-265's `witness` unit against FR-108's `Config` domain package.
Admit FR-265's `low`, `high` and `tail` snapshots by FR-106 for each
invariant (`self` `child` in `low`, `mid` in `high` and `tail`, unless a
step says otherwise). Evaluate each clause once by FR-107 and pass the
evaluation, its stop reports and the observation to the derivation.

1. `AllBelow` over `high`, over `tail` and over `low`; record the work
   charges of the `high` and `tail` evaluations.
2. `SomeAtLeast` over `high` and `low`; `NotAllBelow` over `high`;
   `EqualsAll` over `high`.
3. `GuardedAll` over `high` with `self` `mid`, then with `self` `root`;
   `LetAll` over `high`; `OrAll` over `high`.
4. `NestedAll` over `high`.
5. `FilteredAll` and `MappedAll` over `high`; `BuiltAll` over `low`.
6. `AllBelow` over `high` with the evaluation budget at zero; `NoneSelected`
   over `low`.

Tag the tests `#[trace("TC-740", "FR-265-AC-n")]`.

## Expected Results

- Step 1: `decisive-counterexample`, record (`AllBelow`'s `forall`
  occurrence key, reference to `mid`, index 1, population-root path, no
  trace position) for `high` and an equal record for `tail`, with equal work
  charges; `closed-scope` and no record for `low`.
- Step 2: `decisive-witness` with `mid` at index 1 naming the `exists`, then
  `closed-scope` with no record; `decisive-witness` naming `NotAllBelow`'s
  `forall`; `closed-scope` with no record for `EqualsAll`.
- Step 3: `AllBelow`-shaped record for `GuardedAll`'s `forall`, then
  `closed-scope`; a record for `LetAll`'s `forall`; `closed-scope` and no
  record for `OrAll`.
- Step 4: one record naming the outer `forall`, the reference to `root`,
  index 0; no inner occurrence appears.
- Step 5: `mid`, index 1, population root (`FilteredAll`); element 600,
  index 1, population root (`MappedAll`); element 600, index 1, path rooted
  at a `built` subject naming the list expression's occurrence key and
  ending in an index step for position 1 (`BuiltAll`).
- Step 6: `incomplete`, no basis, no record; `closed-scope`, no record.
