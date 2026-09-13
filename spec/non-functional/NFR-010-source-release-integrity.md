---
id: NFR-010
title: "Publish source releases with exact provenance"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-032"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-033"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-034"
    type: constrains
---
# NFR-010: Publish source releases with exact provenance

## Statement

When a formal-verification MVP source release is published, the
`quire-spec-language` repository shall create an immutable annotated Git tag, a
GitHub source release generated from that tag, and a SHA-256 checksum identifying
the uploaded `git archive`.

## Scope

This applies to the `quire-spec-language` source-only development-release
boundary. It covers the crate version, Cargo publication setting, tag target,
archive, checksum, release notes and the dependency identities needed to repeat
the bounded-scalar verification evidence. It does not publish to crates.io,
expand language support, change `resources/native-v1/`, or turn unsupported,
inconclusive or invalid outcomes into Boolean results.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Cargo manifests that allow registry publication | 0 | 0 | manifest inspection |
| Uploaded archive checksum mismatches | 0 | 0 | independent SHA-256 verification |
| Release tag/asset provenance mismatches | 0 | 0 | Git and GitHub release inspection |

## Verification

Inspect every Cargo manifest for `publish = false`. Verify that the annotated,
unsigned development tag names the selected merged commit without moving an
existing tag. Generate the tarball with `git archive` from that tag, calculate
`SHA256SUMS`, upload both assets to the GitHub release, then download the
archive and checksum independently and compare its top-level revision and
contents to the tag. Release notes identify the tagged commit, pinned
contract-IR/runtime/codegen revisions, admitted bounded-scalar native/oracle/
proptest/Kani profile, and the unsupported object/graph and definedness-bearing
constructs.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-010-AC-1 | The `v0.2.0` release is source-only, every Cargo manifest retains `publish = false`, and no registry publication occurs. | Inspection |
| NFR-010-AC-2 | The release tag is annotated and targets the selected merged MVP commit; no existing tag is deleted, moved, replaced or force-updated. | Inspection |
| NFR-010-AC-3 | The release includes a `git archive` tarball and `SHA256SUMS`; an independently downloaded copy verifies and corresponds to the tagged source tree. | Inspection |
| NFR-010-AC-4 | Release notes state the exact commit and dependency pins, bounded-scalar capability, native/oracle/proptest/Kani agreement, and object/graph and definedness limitations. | Inspection |
