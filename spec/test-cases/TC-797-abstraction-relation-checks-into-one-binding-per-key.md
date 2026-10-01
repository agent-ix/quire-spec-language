---
id: TC-797
title: "An abstraction relation checks into one binding per key, total or partial"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: verifies
---
# TC-797: An abstraction relation checks into one binding per key, total or partial

## Description

Verify the checked form of an admissible relation. Scope: FR-304-AC-1,
FR-304-AC-2.

## Test Procedure

1. Check, against the ConfigVersion domain package, a relation with an
   `ObjectBinding` for ConfigVersion (two fields), a `PopulationBinding`
   for its population, and a `FrameBinding` for `attemptUpdate`.
2. Check a relation holding only the `ObjectBinding`.

## Expected Results

1. A `CheckedAbstractionRelation` with exactly three bindings, keyed by the
   ConfigVersion and population `DeclarationKey`s and `OperationKey {
   ConfigVersion, attemptUpdate }`, each holding the authored value.
2. The relation checks with one binding.
