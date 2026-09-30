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

**RETIRED by QSL-335.** QSL-335 deletes `integration/current-head/` and its Makefile targets. The lane ran QSL against the backend repositories' current heads because QSL's own build pinned them to exact revisions. QSL's own build now resolves its one backend dependency, quire-contract-ir, from `branch = "main"`, so `make ci` already builds against IR's current head, and composition with quire-contract-runtime and quire-contract-codegen is tested in agent-ix/quire-integration. FR-058's acceptance criteria this test case verified are retired for the same reason. There is no successor test case.

Verify that the current-head lane's manifest(s) sit outside the root
`[workspace]` and never change the root `Cargo.lock`, that a real run against
the three backend repositories' current heads resolves and records one commit
per repository, that a run against the intentionally incompatible fixture
fails with a stable diagnostic naming the incompatible dependency, and that
the lane is invocable by one documented local command with a named owner and
update procedure. Scope: FR-058-AC-1 through FR-058-AC-4.

## Test Procedure

**RETIRED by QSL-335.** The current-head lane and its Makefile targets are deleted; this section no longer describes a runnable procedure.

## Expected Results

**RETIRED by QSL-335.** The current-head lane and its Makefile targets are deleted; this section no longer describes a runnable procedure.

## Status

Retired by QSL-335: `integration/current-head/` is deleted, and FR-058 (the
requirement this test case verified) is retired in full.

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
