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
layer-3 `check` core. Scope: FR-088-AC-1. The lane-private
`syntax::ClauseKind` (native-v1) was deleted with the `0-draft` front end in
QSL-1810, so the earlier steps that pinned its variants and consumers are gone.

## Test Procedure

1. Search the whole compiled crate for every definition of an enum whose
   variants correspond to clause kinds (claim, temporal, protocol,
   frame/state-transition forms) and record each one's module path and
   variant list.
2. Confirm exactly one such enum is canonical, defined in the layer-3
   `check` core.

## Expected Results

- Steps 1-2: exactly one canonical clause-kind enum, in `check`.
