---
id: TC-1303
title: "Compare the static subject of two checked units"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-508
    type: verifies
---
# TC-1303: Compare the static subject of two checked units

## Description

Verify the static subject of FR-508. Scope: FR-508-AC-1 to FR-508-AC-5
(AC-6 is by inspection).

## Test Procedure

Fixture `Z0`: a unit importing library `Lib`, selecting domain package `Pkg`
at version 1 as `model M`, header profiles `v` (`quire.value.complete/v1`) and
`pp` (`quire.protocol.finite-global/v1`), one function and one protocol. `S0`
is the subject of checking `Z0` under default limits.

| Row | Change to Z0 | Expected |
| --- | --- | --- |
| 1 | all limits raised | subject equal to `S0`; `differences` empty; `BuildProvenance` unequal |
| 2 | replay the emitted unit with two populations, two windows, two traces and two backends; compute the subject before and after | equal to `S0`, empty |
| 3 | `v` selects `quire.state.core/v1`, source still checks | [profiles] |
| 4 | `Pkg` version 2 | [models] |
| 5 | source text edited (a body literal) | [sources] |
| 6 | supply an extra library the unit never imports | [] |
| 7 | the unit imports a second library | [sources, dependencies] |
| 8 | two declarations each bind a role `x`; then rename one | two role entries with different owners; then [roles] |
| 9 | a unit that fails S3 | the refusal; no subject exists |

Tag the test `#[trace("FR-508-AC-1", "FR-508-AC-2", "FR-508-AC-3", "FR-508-AC-4", "FR-508-AC-5", "TC-1303")]`.

## Expected Results

Each row's Expected column.
