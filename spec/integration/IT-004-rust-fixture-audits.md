---
id: IT-004
title: "Run the selected fixture audits through Rust"
type: IT
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-012"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/NFR-005"
    type: verifies
---
# IT-004: Run the selected fixture audits through Rust

## Objective

Exercise the Rust verification executable over real selected artifact files,
including negative controls and explicit producer-language refusal.

## Target Integration

The Rust fixture-audit executable, repository checkpoint files, and the selected
private standard review packet. Native syntax uses the real parser library.
The integration crosses command/file boundaries without substituting the audits.

## Preconditions

Build the pinned locked Cargo package with no default features. The local
model-source/model-output fixtures are present. Review/roles/rule-syntax checks
additionally select the standard packet from the reviewed Agent A worktree;
its revision and file digests are recorded with the run. No external producer
language is approved by these checks.

## Inputs

Rust command modes from FR-012; this repository's tests/fixtures; the standard's
proposals/state-core and fixtures directories. Negative tests use isolated
temporary copies and leave original evidence untouched.

## Test Procedure

1. Run self-test and model-bytes locally through the Rust executable (the optional manual CI job has a 10-minute timeout).
   - IT-004-SC-01: Self-test controls and all five selected checkpoint digests pass without Python or Node.
2. Run review and roles against the selected standard fixture directory.
   - IT-004-SC-02: Counts are 23 reviewed files, seven invocation cases, six content/digest controls, 17 role artifacts and four source regions.
3. Run rule-syntax against the selected state-core directory.
   - IT-004-SC-03: The real parser observes 50 parsed cases and one unsupported refusal, with no logical-result claim.
4. Run malformed-input/path/budget controls and model-producer refusal.
   - IT-004-SC-04: Invalid or incomplete checks never emit a success summary; no external producer is launched.
5. Inspect every hosted workflow event declaration without dispatching it.
   - IT-004-SC-05: Each existing workflow exposes only workflow_dispatch; local gate results are recorded separately from historical hosted CI.

## Expected Results

Only the selected byte, identity, correspondence and syntax checks succeed.
Fresh producer reproduction remains unavailable until its separate disposition.
The historical artifact bytes and provenance are unchanged.

## Metadata

Priority: High. Automation: Rust command/file integration tests and explicit
local packet checks. Hosted CI is manual-dispatch only while stabilizing and
needs no private sibling repository or external
producer. Existing IT-001 observations remain historical and separately pinned.

## Dependencies

- [FR-012](../functional/FR-012-audit-fixtures-in-rust.md)
- [NFR-005](../non-functional/NFR-005-rust-verification-paths.md)
