---
id: TC-257
title: "Exactly one closed checked clause-kind enum exists, and syntax::ClauseKind gains no variant"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-257: Exactly one closed checked clause-kind enum exists, and syntax::ClauseKind gains no variant

## Description

Verify that exactly one checked clause-kind enum is defined, in the
layer-3 `check` core, and that the lane-private `syntax::ClauseKind`
(native-v1) is unchanged by this requirement: it gains no new variant, and
no consumer added after this requirement uses it as if it were canonical
(ADR-013 R-09). Scope: FR-088-AC-1.

## Test Procedure

1. Search the whole compiled crate for every definition of an enum whose
   variants correspond to clause kinds (claim, temporal, protocol,
   frame/state-transition forms) and record each one's module path and
   variant list.
2. Confirm exactly one such enum is canonical, defined in the layer-3
   `check` core.
3. Read `syntax::ClauseKind`'s variant list before and after this
   requirement's implementation and confirm it is byte-for-byte identical
   (same variants, same names, same order).
4. Search for any new consumer of `syntax::ClauseKind` introduced by this
   requirement (a caller that did not exist before); confirm none exists,
   per R-09's "no new consumer" rule for lane-private types.

## Expected Results

- Steps 1-2: exactly one canonical clause-kind enum, in `check`.
- Step 3: `syntax::ClauseKind` unchanged.
- Step 4: no new consumer of the lane-private enum.
