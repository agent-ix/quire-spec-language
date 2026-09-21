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

- `quire-contract-ir` (`quire-contract-model` package) and
  `quire-contract-runtime`, each patched via a local clone of its
  default branch (see `integration/current-head/README.md` for why a plain
  `branch = "main"` dependency cannot be used for either -- #249 review R3
  added the `quire-contract-runtime` patch to converge this lane's own
  `Cargo.lock` on one revision per repository, alongside IR's pre-existing
  one).
- `quire-contract-codegen`, at `branch = "main"` (not patched; nothing else in
  this lane's graph pins a conflicting revision of it).

QSL itself is a `path` dependency on this repository's own working tree. All
four are real crates; none is faked, stubbed or hand-copied for this test.

## Preconditions

- Network access to fetch quire-contract-ir's, quire-contract-runtime's and
  quire-contract-codegen's default branches.
- `integration/current-head/.deps/quire-contract-ir` and
  `integration/current-head/.deps/quire-contract-runtime`, both prepared by
  `make integration-current-head-prepare` (`tool/` subcommand `prepare`),
  which also refreshes this lane's own `Cargo.lock` to each dependency's
  current head (#249 review HIGH-1).

## Inputs

- A QSL source string parsed through `quire_spec_language::parse`.
- `quire_contract_ir::AnchorName`, constructed from that parse's identity
  data.
- `quire_contract_runtime::{ContractIdentity, RequirementId, RevisionId}`,
  constructed from real, unrestricted public constructors.
- `quire_contract_codegen::BOUND_COVERAGE_SCHEMA`, its public generated-schema
  constant.

## Test Procedure

1. Run `cargo test --manifest-path integration/current-head/Cargo.toml`
   (`tests/contract.rs`), each of the three backend crates resolved at
   current head.
   - IT-013-SC-01: QSL's `parse` succeeds and its output identity data
     constructs a valid `quire_contract_ir::AnchorName`.
   - IT-013-SC-02: `quire_contract_runtime`'s `ContractIdentity`, `RequirementId`
     and `RevisionId` construct from that same data via their real public
     constructors, with no internal or restricted producer bypassed.
   - IT-013-SC-03: `quire_contract_codegen::BOUND_COVERAGE_SCHEMA` is a
     readable, structurally well-formed Draft 2020-12 schema constant,
     confirming the codegen crate's generated artifacts at head are not stale
     relative to its own source.
2. Read this lane's own resolved `integration/current-head/Cargo.lock` after
   step 1 (`tests/contract.rs`'s
   `backend_crates_resolve_from_head_not_a_registry_release`, an automated
   check, not only inspection -- #249 review, LOW).
   - IT-013-SC-04: none of the three backend crates' packages resolved from a
     published registry release; quire-contract-codegen resolved from its git
     default branch, and quire-contract-ir/quire-contract-model/
     quire-contract-runtime resolved through the local clones, so a
     resolution failure at head cannot silently fall back to a released
     version.

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
