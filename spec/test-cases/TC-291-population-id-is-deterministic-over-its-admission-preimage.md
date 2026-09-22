---
id: TC-291
title: "PopulationId is deterministic over its admission preimage and distinguishes distinct admissions"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: verifies
---
# TC-291: PopulationId is deterministic over its admission preimage and distinguishes distinct admissions

## Description

Verify FR-089-AC-1: two admissions of the same `PopulationDocument` against
the same `population_key` and domain package identity mint the same
`PopulationId`, and an admission that differs in any one of `population_key`,
domain package identity, or pre/post anchor side mints a different one.
Scope: FR-089-AC-1.

Known gap: `PopulationId` does not exist yet. `admit_binding`/`admit_invocation`
(`src/model/population.rs:624,1097`) return a `PopulationBinding` with no
minted identity attached, and the kernel `quire-exact` crate defines no
`PopulationId` type. This test case fails against current code. Remaining
work: QSL-131 Slice B.

Catches an implementation that mints `PopulationId` from the document's
content alone (ignoring which population declaration or domain package it
was admitted against, so two distinct population declarations that happen to
admit byte-identical documents would collide), or that mints a fresh
identity on every admission regardless of the preimage (so the same document
admitted twice against the same declaration would never resolve to a shared
identity within one evaluation).

## Test Procedure

1. Admit `PopulationDocument` `D1` against domain package `P1` and
   `population_key` `K1`, twice, independently.
2. Admit `D1` against `P1` and a distinct `population_key` `K2` declared on
   the same domain package.
3. Admit `D1` against a distinct domain package `P2` (same `population_key`
   spelling `K1`, different domain package identity) that declares an
   equivalent population.
4. Within one invocation evaluation, admit `D1` against `P1`/`K1` as a `pre`
   binding via `admit_invocation`, and admit `D1` against `P1`/`K1` again as
   the corresponding `post` binding.

## Expected Results

The two `PopulationId`s from step 1 are equal. The `PopulationId` from step 2
differs from step 1's. The `PopulationId` from step 3 differs from step 1's.
The `pre` and `post` `PopulationId`s from step 4 differ from each other. A
mutant that mints `PopulationId` from the document's content only fails
steps 2 and 3 (both would equal step 1's); a mutant that mints a fresh
identity per call fails step 1 (the two admissions would not match).
