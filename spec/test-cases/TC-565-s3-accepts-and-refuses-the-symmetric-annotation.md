---
id: TC-565
title: "S3 carries the symmetric annotation and refuses a fold over its references"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-150
    type: verifies
---
# TC-565: S3 carries the symmetric annotation and refuses a fold over its references

## Description

Verify that an annotated population that is read only by equality and dereference checks, that the annotation enters the package identity, and that a `fold` over a set of its references is refused at the `fold`.

Scope: FR-150-AC-1 to FR-150-AC-2.

## Test Procedure

Fixtures: ADR-021 §7.1's ConfigVersion unit, with and without `symmetric` on `config_history`; the same unit with `peers: Set<Reference<ConfigVersion>>` and a post clause that folds over `self.peers`.

1. Check the annotated unit; read the `config_history` record and the package identity. Check the unannotated unit and compare package identities.
2. Check the `peers` variant with and without the annotation.

Tag the tests `#[trace("TC-565", "FR-150-AC-n")]`.

## Expected Results

- Step 1: the unit checks, `symmetric: true`, and the two package identities differ.
- Step 2: annotated, `ill_typed`/`identity-observing-form` at the `fold` span naming `fold` and `config_history`, and no checked package; unannotated, the unit checks.
