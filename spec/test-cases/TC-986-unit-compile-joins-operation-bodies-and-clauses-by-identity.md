---
id: TC-986
title: "Unit compile joins operation bodies and clauses by resolved operation identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: verifies
---
# TC-986: Unit compile joins operation bodies and clauses by resolved operation identity

## Description

Verify that S3 associates retained unit operation bodies and pre/post clauses
with admitted operations by resolved `DeclarationKey`, independent of local
body labels and declaration order, and that an operation without a body is
not a dispatch candidate. Scope: FR-302-AC-7.

## Test Procedure

1. In a `1-draft` unit selecting profile alias `V`, select a domain package
   through model alias `M` with object types `A` and
   `B` (`B` conforms to `A`), and operations `A.size` and `B.size`, where
   `B.size` redefines `A.size`. Give each domain operation `false` as its
   precondition and postcondition text.
2. In the unit, declare `body forB using V on M::B::size() { 22 }` before
   `body forA using V on M::A::size() { 11 }`. Add
   `pre preA using V on M::A::size { true }`,
   `post postA using V on M::A::size { true }`,
   `pre preB using V on M::B::size { true }` and
   `post postB using V on M::B::size { true }`. Add
   `invariant observesSize using V on M::B at current { self.size() = 22 }`.
3. Compile the unit. Inspect the selected model's operation records, retained
   operation-clause data and linked dispatch table.
4. In a separate compile, call `empty()` on concrete type `A`, whose selected
   package declares `A.empty` but for which the unit has no `operation-body`
   declaration.

## Expected Results

1. The first compile succeeds. `has_body` is true for the resolved keys of
   `A.size` and `B.size`; the body keyed to `A.size` retains literal `11`, and
   the body keyed to `B.size` retains literal `22`. Each unit pre/post clause
   is associated with its operation's resolved `DeclarationKey`, and its
   retained source provenance identifies that unit declaration. Reversing
   body declaration order or changing the local labels leaves those
   associations unchanged. The retained unit pre/post expressions are `true`;
   the domain-package `false` texts are kept as each operation's own
   `requires`/`ensures` (FR-103) and do not supply them. The invariant evaluates
   true for an object whose most-specific type is `B`.
2. The linked table contains the candidates whose resolved operations have
   bodies and selects by the receiver's most-specific type.
3. In the second compile, the bodyless operation has `has_body = false` and
   contributes no candidate. With no applicable candidate, S3 refuses under
   FR-151 and returns no checked package or partial dispatch table.
