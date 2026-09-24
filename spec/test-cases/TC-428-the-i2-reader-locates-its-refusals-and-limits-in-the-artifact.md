---
id: TC-428
title: "The I2 reader locates its version refusal and its limits in the artifact"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-428: The I2 reader locates its version refusal and its limits in the artifact

## Description

Verify FR-096's O-22 reader: the I2 reader carries IR's version refusal as
`unknown_wire`/`unsupported-wire` naming the actual and expected versions at
`/contract_version` in the digest-addressed artifact, locates IR's limits
at IR's pointer, and gives its own byte ceiling and a refusal at no value
no locus. This catches an
empty pointer standing in for a located failure.

Scope: FR-096-AC-9, FR-096-AC-10.

## Test Procedure

1. Read bytes whose `contract_version` is `quire.checked-package/v3`.
2. Read a v2 wire whose graph has more nodes than node bound `B`.
3. Read bytes longer than the artifact byte ceiling.
4. Read bytes that are not JSON.

Tag the tests `#[trace("TC-428", "FR-096-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: `StageFailure::Refused`, code `unknown_wire`/`unsupported-wire`,
  `actual` `quire.checked-package/v3` and `expected`
  `quire.checked-package/v2`, locus `Locus::Artifact` with the bytes'
  `raw-artifact-digest` and pointer `/contract_version`.
- Step 2: `StageFailure::Limit`, kind node count, bound `B`, IR's consumed
  counter, locus `Locus::Artifact` with the bytes' `raw-artifact-digest`
  and IR's pointer.
- Step 3: kind input bytes and no locus.
- Step 4: a refusal with no locus.

## Status

Planned. ADR-013 §7 slice S-5b (QSL-160). Steps 1 and 2 need the three IR
reader changes FR-096 lists.
