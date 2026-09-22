---
id: TC-387
title: "The ProtocolClause snapshot cause maps each WrongSnapshotCause to wrong_snapshot"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-387: The ProtocolClause snapshot cause maps each WrongSnapshotCause to wrong_snapshot

## Description

Verify FR-090-AC-6. The `ProtocolClause` family cause that carries
`WrongSnapshotCause` (ProtocolClause owns `Pre`: ADR-012 §4.3, FR-091-AC-8)
has an exhaustive `catalog_code()` with no `_` arm
(ADR-013 T-6, O-17). It returns code `wrong_snapshot` with the variant's own
catalog cause tag. Scope: FR-090-AC-6.

This catches two faults: a `catalog_code()` that returns the wrong cause tag
for a variant, and one that routes through the native-v1
`qsl_foundation::diagnostic::Code` (`Code::WrongSnapshot.as_str()`) instead
of the catalog `CatalogCode`.

## Test Procedure

1. Build the family cause holding `WrongSnapshotCause::WrongAnchor` and call
   `catalog_code()` on it.
2. Build the family cause holding `WrongSnapshotCause::ForbiddenPreRead` and
   call `catalog_code()` on it.
3. Add the family cause's `catalog_code()` match to the checked-in
   seam-probe list (FR-063), so that a probe-only `WrongSnapshotCause`
   variant fails that match with `E0004` under `make seam-probe`.

Tag the test `#[trace("FR-090-AC-6", "TC-387")]`.

## Expected Results

- Step 1 returns `CatalogCode::new("wrong_snapshot", "wrong-anchor")`.
- Step 2 returns `CatalogCode::new("wrong_snapshot", "forbidden-pre-read")`.
- Under step 3, a variant with no arm fails to compile.

## Status

Planned; no test backs this case.
