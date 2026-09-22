---
id: TC-296
title: "A standalone Direct admission and an invocation's Post binding over the same domain package and population_key mint distinct PopulationIds"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: verifies
---
# TC-296: A standalone Direct admission and an invocation's Post binding over the same domain package and population_key mint distinct PopulationIds

## Description

Verify the closed three-state admission-role discriminator half of
FR-089-AC-1: a `Direct` admission (standalone `admit_binding`) and an
`admit_invocation`-attached `Post` binding, over the same domain package
and `population_key`, mint distinct `PopulationId`s, even when admitting
the same `PopulationDocument`. Scope: FR-089-AC-1.

This is the collision QSL-172's review found under a literal reading of the
preimage: a two-state (`pre`/`post`-only) discriminator names an anchor
side only for the binding an invocation attaches, so a `Direct` admission
and a `Post` binding would carry an identical preimage whenever they share
a domain package and `population_key` -- `admit_binding`
(`src/model/population.rs:624`) cannot see its caller's role, and
`admit_invocation` (`:1105-1124`) calls it twice with identical arguments
apart from `document`, and standalone callers of `admit_binding` also
exist. The three-state discriminator (`Direct`, `Pre`, `Post`) closes this
by naming a role for every admission, not only the two an invocation
attaches.

Implemented: the closed three-state `AdmissionRole` (`Direct`/`Pre`/`Post`)
discriminator is part of every `PopulationId`'s admission preimage
(QSL-131 Slice B). Backed by
`tc_296_standalone_direct_admission_distinct_from_invocation_post`
(`tests/it/model_population.rs`).

Catches an implementation that models the admission-role component as a
two-state `Option<AnchorSide>` (`None` for a standalone admission, `Some(Pre)`/
`Some(Post)` for an invocation's bindings) rather than the closed
three-state discriminator the ruling requires: under `Option<AnchorSide>`,
a standalone admission's `None` and an invocation's `Post` binding still
differ syntactically, but a test that only checks "the type has three
named states" cannot catch a discriminator collapsed to `{Some(Post), None}`
compared by discarding the `Option` wrapper before hashing -- this test
instead asserts the *identity values themselves* stay distinct, which is
robust to either representation.

## Test Procedure

1. Admit `PopulationDocument` `D1` against domain package `P1` and
   `population_key` `K1` directly (standalone `admit_binding`, no
   invocation), producing `PopulationId` `id_direct`.
2. Within one invocation evaluation, admit `D1` against the same `P1`/`K1`
   as the invocation's `Pre` binding, then admit `D1` against `P1`/`K1`
   again as the corresponding `Post` binding, producing `id_pre` and
   `id_post`.
3. Compare `id_direct`, `id_pre` and `id_post` pairwise.

## Expected Results

All three of `id_direct`, `id_pre` and `id_post` are pairwise distinct. In
particular `id_direct != id_post`, even though both admissions share the
same domain package, `population_key` and document content. A mutant that
mints the admission-role component as a two-state discriminator collapsing
`Direct` and `Post` to the same representation (for example, both encoded
as "not `Pre`") fails the `id_direct != id_post` assertion.
