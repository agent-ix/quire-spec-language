---
id: TC-979
title: "Every capability match is exhaustive under the seam probe"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-080
    type: verifies
---
# TC-979: Every capability match is exhaustive under the seam probe

## Description

Verify the seam probe reaches every exhaustive match over the canonical
`Capability` type. Scope: FR-080-AC-6.

## Test Procedure

1. Run `xtask seam-probe` with the probe variant enabled.
2. Review its expected-location report for `same_kind`, `families()`,
   `composed_admission_stages`, descriptor advertisement validation and
   candidate matching.
3. Temporarily add a wildcard arm at each listed match site in turn and rerun
   the probe.

## Expected Results

- The probe reports an `E0004` at each listed match site and each location is
  present in the checked-in expected-location list.
- A wildcard arm suppresses that location's `E0004`, and the probe fails
  because the expected location is missing.
