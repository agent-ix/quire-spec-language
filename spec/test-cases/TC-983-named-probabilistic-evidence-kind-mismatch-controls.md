---
id: TC-983
title: "Named probabilistic backends require the exact requested evidence kind"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-983: Named probabilistic backends require the exact requested evidence kind

## Description

Exercise a named registered backend for each admitted probabilistic evidence
kind, with both matching and mismatching descriptor controls. Scope:
FR-075-AC-1/3/13/14.

## Test Procedure

For each row, register one backend `p` advertising
`probabilistic-satisfaction` and exactly the evidence kind shown. Compute a
candidate set for an item naming `p` and requesting the row's evidence kind.
Run each row in a fresh registry.

| Requested item evidence kind | Named backend's admitted evidence kind | Control | Expected named candidate outcome |
|---|---|---|---|
| `statistical` | `statistical` | Exact-match positive control | Candidate set contains only `p`. |
| `statistical` | `exact` | Evidence-kind mismatch | Empty candidate set; it is not an unknown-backend marker. |
| `exact` | `exact` | Exact-match positive control | Candidate set contains only `p`. |
| `exact` | `statistical` | Evidence-kind mismatch | Empty candidate set; it is not an unknown-backend marker. |

## Expected Results

Both named mismatch controls return an empty candidate set and preserve the
registered identity; both exact-match controls return that identity. Neither
evidence kind substitutes for the other. A name not registered remains the
distinct unknown-backend marker under FR-075-AC-3 and TC-195.
