---
id: FR-079
title: "Preserve existing Kani lowering corpora across the registry swap"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-011
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: traces_to
---
# FR-079: Preserve existing Kani lowering corpora across the registry swap

## Description

QSL SHALL NOT change the output of any existing Kani lowering corpus case
when it replaces the fixed `ProjectionTarget` catalog with the registry
(FR-075). QSL SHALL NOT change the set of target names
`src/lowering/target.rs` currently publishes (`boolean-oracle/v1`,
`integer-ir/v1`, `state-scalar-ir/v1`), or their parsing, display and
CLI-selection behavior, as part of that replacement.

Today the registry (FR-075) sits alongside the fixed catalog, the catalog
selects the lowering target, and lowering takes no registry input.
ADR-011 §6.2 (the `lowering` row) and §7.3 M-6b assign #217 the deletion of
the `lowering` targets, `ProjectionTarget` and `--target`, with the backend
chosen only by `BackendId`. How that deletion relates to this requirement's
preserved target names is an open question for the owner (ADR-011, "Open
questions for the owner", ADR-011-OQ-1).

## Inputs

- The existing Kani lowering corpus (fixtures and their recorded expected
  output) as it exists on `main` before this change.
- The three existing target name strings and their current
  `FromStr`/`Display` round-trip behavior.

## Outputs

- Byte-identical lowering output for every existing corpus case.
- Byte-identical `FromStr`/`Display` behavior for all three legacy target
  name strings.

## Behavior

### The swap changes routing, not lowering output

Registering the three legacy targets as `BackendDescriptor` entries (or an
equivalent registration for Kani's existing coverage) in the new registry
SHALL leave the target-neutral IR lowering these targets drive unchanged.
The registry changes how a target is selected and how a capability is
matched to a registrant; it SHALL NOT change what IR or output a selected
target produces.

### Every legacy target name still resolves

Each of `boolean-oracle/v1`, `integer-ir/v1` and `state-scalar-ir/v1` SHALL
continue to be an accepted, published target name after the swap, with the
same `Display` spelling and the same `FromStr` acceptance/rejection
behavior as before. Dropping, renaming, or silently merging any of the
three during the migration violates this requirement even if the merged
behavior is arguably equivalent.

### Corpus regression is corpus-differential, not corpus-approximate

Verification of this requirement compares actual recorded output bytes
before and after the swap for the same corpus inputs; a test that only
asserts "the corpus still passes" without comparing pre- and post-swap
output does not verify this requirement, because a corpus case whose
expected-output fixture was updated alongside the implementation change
would still pass without showing the output is unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-079-AC-1 | For every case in the existing Kani lowering corpus, the lowering output produced through the registry-based routing is byte-identical to the output recorded before the registry swap, compared directly against the pre-swap recording rather than against a fixture updated at the same time as the swap. | Test (TC-203) |
| FR-079-AC-2 | Each of `boolean-oracle/v1`, `integer-ir/v1` and `state-scalar-ir/v1` parses via `FromStr`, round-trips via `Display` to its original spelling, and appears in the CLI's published target list, identically before and after the swap. | Test (TC-204) |

## Dependencies

- [FR-075](FR-075-compute-candidates-from-registered-backends.md) is the
  registry this requirement's non-regression check runs against.
- Ticket exit condition: "existing Kani lowering corpora unchanged"
  ([quire-spec-language#185](https://github.com/agent-ix/quire-spec-language/issues/185)).
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.2 and §7.3 M-6b, and ADR-011-OQ-1, which records their open question
  against FR-079-AC-1 and FR-079-AC-2.

## Status

Specified under
[quire-spec-language#185](https://github.com/agent-ix/quire-spec-language/issues/185).
The registry landed under QSL-46 (PR #305) alongside the unchanged catalog.

By Acceptance Criterion:
- FR-079-AC-1: unbacked. No test compares corpus output across a catalog
  replacement. Open question: ADR-011-OQ-1.
- FR-079-AC-2: backed for the current catalog (`TC-204`):
  `legacy_target_names_parse_round_trip_and_list_identically`
  (`tests/it/lowering_registry_isolation.rs`). Open question: ADR-011-OQ-1.
