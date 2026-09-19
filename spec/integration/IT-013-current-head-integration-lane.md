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

Verify the real integration boundary the current-head lane exists to guard:
that QSL's parse output, quire-contract-ir's identity types,
quire-contract-runtime's identity newtypes and quire-contract-codegen's
generated schema constants still compose when each of the three backend
crates is resolved at its repository's current default-branch head, not at
QSL's own pinned `rev`. Without this test, the lane's manifest could resolve
and build while its four crates' real public surfaces had already drifted
apart in a way only a genuine cross-crate call detects.

## Target Integration

The lane crate under test is `integration/current-head/`. Its external
dependencies, each resolved at current head rather than a pinned `rev`:

- `quire-contract-ir` (`quire-contract-model` package), patched via a local
  vendored clone of its default branch (see `integration/current-head/README.md`
  for why a plain `branch = "main"` dependency cannot be used for this one).
- `quire-contract-runtime`, at `branch = "main"`.
- `quire-contract-codegen`, at `branch = "main"`.

QSL itself is a `path` dependency on this repository's own working tree. All
four are real crates; none is faked, stubbed or hand-copied for this test.

## Preconditions

- Network access to fetch quire-contract-ir's, quire-contract-runtime's and
  quire-contract-codegen's default branches.
- `integration/current-head/.vendor/quire-contract-ir` prepared by
  `make integration-current-head-prepare` (`tool/` subcommand `prepare`).

## Inputs

- A QSL source string parsed through `quire_spec_language::parse`.
- `quire_contract_ir::AnchorName`, constructed from that parse's identity
  data.
- `quire_contract_runtime::{ContractIdentity, RequirementId, RevisionId}`,
  constructed from real, unrestricted public constructors.
- `quire_contract_codegen::{BOUND_COVERAGE_SCHEMA, MAX_ANALYSIS_BYTES}`, its
  public generated-schema constants.

## Test Procedure

1. Run `cargo test --manifest-path integration/current-head/Cargo.toml`
   (`tests/contract.rs`), each of the three backend crates resolved at
   current head.
   - IT-013-SC-01: QSL's `parse` succeeds and its output identity data
     constructs a valid `quire_contract_ir::AnchorName`.
   - IT-013-SC-02: `quire_contract_runtime`'s `ContractIdentity`, `RequirementId`
     and `RevisionId` construct from that same data via their real public
     constructors, with no internal or restricted producer bypassed.
   - IT-013-SC-03: `quire_contract_codegen::BOUND_COVERAGE_SCHEMA` and
     `MAX_ANALYSIS_BYTES` are readable and structurally well-formed constants,
     confirming the codegen crate's generated artifacts at head are not stale
     relative to its own source.
2. Inspect the lane's resolved dependency graph after step 1.
   - IT-013-SC-04: none of the three backend crates resolved from a published
     registry release; each resolved from its git default branch (or, for
     quire-contract-ir, the local vendored clone of it), so a resolution
     failure at head cannot silently fall back to a released version.

## Expected Results

All four success criteria hold. The composition exercised is real: QSL's real
parser output feeds quire-contract-runtime's real identity constructors, and
quire-contract-codegen's real generated constants are read directly, all
against each backend repository's current head, not a hand-built stand-in.
This test intentionally does not build a full checked-package v2 round trip
(oracle generation, execution, accounting); quire-contract-runtime's and
quire-contract-codegen's deeper APIs (campaign accounting, verdicts,
bound-oracle generation) are constructible only from their own crates'
internal producers, and wiring QSL's internal fixture machinery through this
external lane crate is out of this test's proportional scope (see
`integration/current-head/README.md`, "What it deliberately does not
attempt").

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
