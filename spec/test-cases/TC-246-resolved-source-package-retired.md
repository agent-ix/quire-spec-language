---
id: TC-246
title: "ResolvedSourcePackage is retired, with no dangling caller"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-246: ResolvedSourcePackage is retired, with no dangling caller

## Description

Verify that `ResolvedSourcePackage` (`qsl-semantics/src/complete/package.rs`, complete-V1
lane C2) is removed in the same change that adds `VerifiedPackage` and
`ImportView`, with no compatibility alias, feature-flagged fallback, or
wrapper keeping the old name or its construction path reachable, and no
caller left referencing it. Scope: FR-087-AC-7, FR-087-CON-4.

## Test Procedure

1. Search the whole compiled crate (`src/`, `tests/`, `examples/`) for every
   occurrence of `ResolvedSourcePackage` by name.
2. Search for any re-export, type alias, or `pub use` naming the removed
   type under any spelling.
3. Build the crate. A build that succeeds with `ResolvedSourcePackage`
   reachable under any conditional compilation flag is a duplication
   finding, not a pass (the TC-170/TC-246 shared standard).
4. Confirm every former caller of `resolve_source_package` (or its
   equivalent) now constructs and consumes a `VerifiedPackage`/`ImportView`
   pair instead, with the same or a strictly increased set of covered
   scenarios (no test deleted without a replacement covering the same
   scenario).

## Expected Results

- Steps 1-2: zero occurrences of `ResolvedSourcePackage` anywhere in the
  compiled crate, its tests, or its examples.
- Step 3: the crate builds with no reachable definition of the old type
  under any feature combination.
- Step 4: every former caller's scenario has a `VerifiedPackage`/`ImportView`
  equivalent; a scenario silently dropped rather than migrated fails this
  step.
