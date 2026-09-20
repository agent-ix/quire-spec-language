---
id: TC-159
title: "Run the current-head integration lane against real and intentionally incompatible heads"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-058
    type: verifies
---
# TC-159: Run the current-head integration lane against real and intentionally incompatible heads

## Description

Verify that the current-head lane's manifest(s) sit outside the root
`[workspace]` and never change the root `Cargo.lock`, that a real run against
the three backend repositories' current heads resolves and records one commit
per repository, that a run against the intentionally incompatible fixture
fails with a stable diagnostic naming the incompatible dependency, and that
the lane is invocable by one documented local command with a named owner and
update procedure. Scope: FR-058-AC-1 through FR-058-AC-4.

## Test Procedure

1. Inspect the root `Cargo.toml` `[workspace]` `members` list and the lane's
   own manifest(s) under `integration/current-head/`. Record the root
   `Cargo.lock`'s content hash, run `make integration-current-head-prepare`
   and `make integration-current-head`, then re-inspect the root
   `Cargo.lock`.
2. Run `cargo run --manifest-path integration/current-head/tool/Cargo.toml --
   revision-log --qsl . --manifest integration/current-head/Cargo.toml
   --vendor-root integration/current-head/.vendor` against real network
   access to the three backend repositories' default branches.
3. Run `cargo run --manifest-path integration/current-head/tool/Cargo.toml --
   check-incompatible-fixture --manifest
   integration/current-head/fixtures/incompatible/Cargo.toml`.
4. Read `integration/current-head/README.md`.

## Expected Results

- Step 1: the lane's manifest(s) are not listed in the root `[workspace]`
  `members`; the root `Cargo.lock`'s content hash is identical before and
  after both `make` invocations.
- Step 2: the revision log names exactly four lines, one per repository (QSL,
  quire-contract-ir, quire-contract-runtime, quire-contract-codegen), each
  with a real resolved commit hash.
- Step 3: the subcommand exits successfully (it detected the expected
  failure) and prints the stable `FR-058-AC-3` marker line naming the
  incompatible dependency; the underlying `cargo build` of the fixture itself
  fails to compile with unresolved-import errors, not a patch-resolution
  error.
- Step 4: the README names an owner (whoever owns ADR-011 T-12, tracked under
  #215 and successors), a bisection/escalation procedure for a failing run,
  and the exact local invocation commands used in steps 1-3.

## Metadata

- Priority: P1
- Target Integration: `integration/current-head/` lane crate, tool crate and
  fixture; real quire-contract-ir/-runtime/-codegen repositories over network
- Automation: Manual (needs live network access to three repositories'
  default branches); see [IT-013](../integration/IT-013-current-head-integration-lane.md)
  for the automated in-process contract test this lane also runs.

## Dependencies

**Upstream:** [FR-058](../functional/FR-058-detect-current-head-cross-repository-incompatibility.md).
**Downstream:** [IT-013](../integration/IT-013-current-head-integration-lane.md).
