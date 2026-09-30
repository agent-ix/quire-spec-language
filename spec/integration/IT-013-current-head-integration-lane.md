---
id: IT-013
title: "Exercise QSL, Contract IR, Runtime and Codegen together at each repository's current head"
type: IT
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-058, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/ADR-011, type: references }
---
# IT-013: Exercise QSL, Contract IR, Runtime and Codegen together at each repository's current head

## Objective

**RETIRED by QSL-335.** QSL-335 deletes `integration/current-head/` and its Makefile targets. The lane ran QSL against the backend repositories' current heads because QSL's own build pinned them to exact revisions. QSL's own build now resolves its one backend dependency, quire-contract-ir, from `branch = "main"`, so `make ci` already builds against IR's current head, and composition with quire-contract-runtime and quire-contract-codegen is tested in agent-ix/quire-integration. The test it describes, `integration/current-head/tests/contract.rs`, is deleted. There is no successor in this repository.

Verify the real integration boundary the current-head lane exists to guard:
that QSL's parse output, quire-contract-ir's identity types,
quire-contract-runtime's identity newtypes and quire-contract-codegen's
generated schema constants still compose when each of the three backend
crates is resolved at its repository's current default-branch head, not at
QSL's own pinned `rev`. Without this test, the lane's manifest could resolve
and build while its four crates' real public surfaces had already drifted
apart in a way only a genuine cross-crate call detects.

## Target Integration

**RETIRED by QSL-335.** The current-head lane and its Makefile targets are deleted; this section no longer describes a runnable procedure.

## Preconditions

**RETIRED by QSL-335.** The current-head lane and its Makefile targets are deleted; this section no longer describes a runnable procedure.

## Inputs

**RETIRED by QSL-335.** The current-head lane and its Makefile targets are deleted; this section no longer describes a runnable procedure.

## Test Procedure

**RETIRED by QSL-335.** The current-head lane and its Makefile targets are deleted; this section no longer describes a runnable procedure.

## Expected Results

**RETIRED by QSL-335.** The current-head lane and its Makefile targets are deleted; this section no longer describes a runnable procedure.

## Status

Retired by QSL-335: `integration/current-head/` is deleted and FR-058, the
requirement this test verified, is retired in full.

## Metadata

- Priority: P1
- Target Integration: quire-contract-ir, quire-contract-runtime and
  quire-contract-codegen at current head, called from the current-head lane
  crate
- Automation: Automated Rust integration test (needs network access to three
  repositories' default branches)

## Dependencies

**Upstream:** [FR-058](../functional/FR-058-detect-current-head-cross-repository-incompatibility.md).
**Downstream:** [TC-159](../test-cases/TC-159-current-head-integration-lane.md).
