---
id: TC-773
title: "Installing, removing or reordering providers changes nothing outside the touched candidate sets"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: verifies
---
# TC-773: Installing, removing or reordering providers changes nothing outside the touched candidate sets

## Description

Verify FR-289's front-end, locality and meaning invariants. Order invariance is TC-194's.

Scope: FR-289-AC-1 to FR-289-AC-3.

## Test Procedure

1. Compile `tests/fixtures/spine-compile.native` in one test that builds an empty registry and in one that builds a registry from three manifests.
2. Property-test: for generated manifests and items, add a manifest advertising none of an item's kinds, then one advertising its kind, and compute the item's candidate set each time.
3. Compile the `compile_fail` doctests that build a `refuted` terminal record with no reproducing replay result and a `proved` record with no `BackendId`, beside compiling controls; build a `refuted` record from a reproducing replay result and read it back.

Tag the tests `#[trace("TC-773", "<AC id>")]`.

## Expected Results

- Step 1: the `package` bytes and `check` outcomes are equal.
- Step 2: the first addition leaves the candidate set equal; the second adds exactly that backend.
- Step 3: the doctests fail to compile and the controls compile; the record reads back `refuted` with that replay result.
