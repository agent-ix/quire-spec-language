---
id: TC-386
title: "F diagnostic maps every family-refusal, snapshot-cause and model-refusal catalog code to category refusal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-386: F diagnostic maps every family-refusal, snapshot-cause and model-refusal catalog code to category refusal

## Description

Verify FR-090-AC-5. F `diagnostic`'s map from a `CatalogCode` to an O-16
`Category` returns `Category::Refusal` for the code of every `FamilyRefusal`
variant. It does the same for every code that the `ProtocolClause`
snapshot cause's `catalog_code()` returns, and for every code
`ModelRefusal::catalog_code()` returns, since a `ModelRefusal` reaches the
caller in `FamilyResult::Refused`, category `refusal`. F never names
`FamilyRefusal` (ADR-013 O-16). Scope: FR-090-AC-5.

This catches two faults: a map that files a family refusal under
`Unsupported` or `InternalFailure`, and a map written by matching on
`FamilyRefusal`, which would need F to depend upward on the `check` core.

## Test Procedure

1. For every `FamilyRefusal` variant, take `catalog_code()` and pass the
   resulting `CatalogCode` to F `diagnostic`'s category map. The test lives
   in the crate that defines `FamilyRefusal`.
2. Do the same for every variant of the `ProtocolClause` family cause
   that carries `WrongSnapshotCause` (`WrongAnchor`, `ForbiddenPreRead`).
3. Do the same for every `ModelRefusal` cause `ModelRefusal::catalog_code()`
   covers.
4. Read `qsl-foundation/Cargo.toml`'s `[dependencies]` table.

Tag the test `#[trace("FR-090-AC-5", "TC-386")]`.

## Expected Results

- Every code in steps 1 to 3 maps to `Category::Refusal`.
- The step-4 table names neither the crate that defines `FamilyRefusal` nor
  any crate at layer 3 or above. It may name `quire-exact`, because ADR-011
  §6.1 lets F depend on K.

## Status

Planned; no test backs this case.
