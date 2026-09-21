---
id: TC-205
title: "cargo-deny denies inventory, linkme and ctor"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-080
    type: verifies
---
# TC-205: cargo-deny denies inventory, linkme and ctor

## Description

Verify that the `cargo-deny` configuration actively denies `inventory`,
`linkme` and `ctor` as dependencies, and that the deny rule fires when one
of them is added, rather than passing only because none happens to be
present today. Scope: FR-080-AC-2.

Catches a configuration that never actually lists the three crates (so
`cargo deny check` passes today for the unrelated reason that nobody added
them, not because the gate would catch it), and a `cargo-deny` config that
denies them only in one profile (e.g. release) while a dev/test build can
still pull them in undetected.

## Test Procedure

1. Run `cargo deny check` against the unmodified registry crate's manifest
   and confirm it passes.
2. Temporarily add `inventory` as a dependency of the registry crate (a
   throwaway branch or worktree, not committed), and re-run
   `cargo deny check`.
3. Repeat step 2 for `linkme`, then for `ctor`, each as its own trial.
4. Inspect the `cargo-deny` configuration file directly and confirm each of
   the three crate names appears in a `deny` (not merely `warn` or
   `skip`) list scoped to the registry crate's dependency graph, in every
   build profile the gate runs under.

## Expected Results

Step 1 passes. Steps 2 and 3 each fail `cargo deny check`, naming the added
crate. Step 4 confirms the deny entries exist in configuration, not only as
an observed absence.
