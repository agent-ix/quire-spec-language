---
id: TC-173
title: "Refusal split: check causes in check, InputRefusal in value::expression"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: verifies
---
# TC-173: Refusal split: check causes in check, InputRefusal in value::expression

## Description

Verify the exact twelve-versus-one split ADR-011's module table describes
for `value::expression::refusal`: `check` defines every check-cause type
(`CheckCause`, `CheckRefusal`, `Obligation`, `MeasureObligation`,
`CheckingStage`, `CheckingLimitKind`, `DispatchFunctionRole`,
`InvalidDispatchDeclaration`, `Location`, `Origin`, `ProvedInterval`,
`WrongSnapshotCause`) and does not define `InputRefusal`; `value::expression`
defines `InputRefusal`, relocated beside `CheckedPackage::call`'s and
`CheckedPackage::evaluate`'s admission code, and does not define any of the
twelve check-cause types. It also verifies the one mechanical consequence of
relocating `WrongSnapshotCause`: `value::outcome.rs`'s import updates to name
`check`, with no other change to that file. This test catches both an
over-move (`InputRefusal` folded into `check`, collapsing the split ADR-011
requires) and an under-move (one or more check-cause types left behind in
`value::expression`, so `check`'s own checking methods cannot use them
without a reverse edge). Scope: FR-068-AC-4, FR-068-AC-8.

## Test Procedure

1. For each of the twelve check-cause type names, search the compiled crate
   for every defining location and record it.
2. Search the compiled crate for every defining location of `InputRefusal`.
3. Confirm `InputRefusal`'s defining location is textually adjacent to (in
   the same file as, or a file dedicated to) `CheckedPackage::call`'s and
   `CheckedPackage::evaluate`'s admission logic, inside `value::expression`.
4. Read `src/value/outcome.rs`'s import list and confirm its
   `WrongSnapshotCause` import resolves to `crate::check::WrongSnapshotCause`.
5. Diff `src/value/outcome.rs` against its pre-move baseline and confirm the
   only change is the one import path.

## Expected Results

- Step 1: each of the twelve check-cause types has exactly one defining
  location, in `check`; any type also defined in `value::expression`, or
  defined in neither, fails this step, naming the type.
- Step 2: `InputRefusal` has exactly one defining location, in
  `value::expression`; a definition found in `check` fails this step.
- Step 3: `InputRefusal`'s location is adjacent to the admission code, not
  left at its old top-of-`mod.rs` position once that logic has moved
  elsewhere; a definition present but physically stranded away from the
  admission code it now describes is a finding, though not by itself a
  failure of FR-068-AC-4's letter — record it as a note if found without the
  physical relocation, since the criterion's binding requirement is the
  type/module split, not the exact line position.
- Step 4: the import resolves to `crate::check::WrongSnapshotCause`; any
  other resolution, or a compile error, fails this step.
- Step 5: no other line of `value::outcome.rs` differs from the pre-move
  baseline; any other diff fails this step and names it.
