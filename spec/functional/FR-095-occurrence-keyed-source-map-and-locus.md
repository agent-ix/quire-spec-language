---
id: FR-095
title: "S-4: occurrence keys, source regions, the package source map and the diagnostic locus"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: traces_to
---
# FR-095: S-4: occurrence keys, source regions, the package source map and the diagnostic locus

## Description

ADR-013 §7 slice **S-4** (Linear QSL-159) has no owning FR before this one.
It carries O-07 (source occurrence identity), O-12 (source locations and
provenance: the occurrence-key-keyed package source map) and T-5 (the
foundation diagnostic `Locus`). Its gate is S-3 (FR-087, FR-088), landed as
PR #397.

The types are layer F's (ADR-011 §6.1): `qsl_foundation::source::provenance`
holds the occurrence key, `RawSourceRef`, `Revision`, `SourceRegion` and
`PackageSourceMap`, and `qsl_foundation::diagnostic` holds `Locus`. The
layer-4 I2 reader builds the package source map from a verified
`quire.checked-package/v2` wire's `source_map` (ADR-013 C-14).

## Inputs

- ADR-013 O-07, O-12, T-5, C-14; R-10 (a wire node id never becomes a
  `NodeKey` by conversion).
- QSpec FR-322 `source_map` and the `RawSourceRef`, `Revision`,
  `SourceRegion` and `SourceMapEntry` schema definitions
  (`ix://agent-ix/quire-specification/FR-322`).
- The kernel `quire_exact::{Origin, Location}` (ADR-013 S-1).
- The `quire.checked-package/v2` wire a caller hands the I2 reader.

## Outputs

- `OccurrenceKey`: (wire node id, role, ordinal).
- `RawSourceRef` (authority, identity, `Revision{namespace, value}`,
  `quire.source.bytes/v1` digest) and `SourceRegion` (`RawSourceRef`, byte
  start, byte end).
- `PackageSourceMap`: occurrence key → ordered, non-empty regions, carried by
  the I2 reader's verified outcome.
- `Locus`: `Region(SourceRegion)`, `Occurrence(Location)` or
  `Artifact{digest, pointer}`, with an RFC 6901 `JsonPointer`.

## Behavior

### An occurrence key and a source region are lexical values

An occurrence key SHALL be (node id, role, ordinal), equal to another exactly
when all three members are equal. Every node of a checked package's semantic
graph SHALL have at least one source occurrence. A `RawSourceRef` SHALL carry
a non-empty authority and identity, a `Revision` with a non-empty namespace
and value, and a `quire.source.bytes/v1` digest. A `SourceRegion` SHALL be a
half-open byte interval `[start, end)` with `start <= end`; an empty region
is a point. Two regions SHALL be equal exactly when their source digests,
starts and ends are equal.

### The package source map is the wire's, keyed by occurrence

When the I2 reader verifies a `quire.checked-package/v2` wire, its outcome
SHALL carry the package source map built from the wire's `source_map`: each
entry's occurrence key maps to that entry's regions, in wire order, each
under the entry's own `RawSourceRef`. The map SHALL hold each key once, with
at least one region. A node's occurrences SHALL be exactly the entries that
name it. An entry that the provenance types refuse SHALL refuse the read as
`invalid_source_map`.

### A location tag resolves through the map or refuses

A kernel `Location` SHALL resolve through the package source map by its node
id and occurrence, in the emission direction (node key → wire node id). A
tag whose node the package does not map SHALL refuse with an unknown-node
cause, and a tag of a mapped node at an unmapped role and ordinal with an
unknown-occurrence cause.

### A locus names a region, an occurrence or an artifact position

A `Locus::Region` SHALL denote its own region, a `Locus::Occurrence` the
regions its location resolves to through the package source map, and a
`Locus::Artifact` no source region. An artifact locus's pointer SHALL be an
RFC 6901 JSON pointer.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-095-AC-1 | Two occurrence keys differing only in node id, only in role or only in ordinal are unequal, and equal members give equal keys. `RawSourceRef` refuses an empty authority, an empty identity and a `quire.definition.bytes/v1` digest, `Revision` an empty namespace and an empty value, each with its own cause; `SourceRegion` refuses start 5, end 4 and admits start 5, end 5. Every node of the semantic graph of a checked package holding a one-parameter function and a caller that calls it inside a conditional has at least one source occurrence. | Test (TC-420) |
| FR-095-AC-2 | A region over digest `d`, start 4, end 9 equals, and orders equal to, a region with the same three members under another authority, identity and revision, and is unequal to regions that change only the digest, only the start or only the end. | Test (TC-420) |
| FR-095-AC-3 | A verified read of a wire holding two declarations, one of whose entries has two regions, carries a package source map in which each entry's key maps to exactly its wire regions in wire order, each with the wire's authority, identity, revision and source digest, and each node's occurrences are exactly its one entry. The map refuses an entry with no region and a key mapped twice. An entry with a reversed region or a non-hex node id refuses as `invalid_source_map`. Over QSpec's published positive `quire.checked-package/v2` fixtures, every `source_map` entry of each IR-admitted fixture looks up to exactly its wire regions and each node's occurrence count equals its entry count. | Test (TC-421) |
| FR-095-AC-4 | A location tag of a mapped occurrence resolves to its regions; a tag naming a node the map does not hold refuses with the unknown-node cause, and one naming a mapped node at an unmapped ordinal with the unknown-occurrence cause. A wire whose `source_map` entry names a node absent from its semantic graph refuses at the read as `invalid_source_map`. | Test (TC-421) |
| FR-095-AC-5 | A region locus denotes exactly its region; an occurrence locus denotes the two regions its mapped key holds, and an unmapped one refuses with the map's unknown-occurrence cause; an artifact locus refuses as naming no source region. | Test (TC-422) |
| FR-095-AC-6 | `""`, `/`, `/source_map/0/regions` and `/a~1b/c~0d` parse as JSON pointers and keep their text; `source_map` refuses for a missing leading `/`, and `/a~2b` and `/a~` refuse for the `~` at byte 2. | Test (TC-422) |

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  §3 O-07, O-12, T-5; §4 C-14, C-21; §7 (the S-4 slice row).
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 (layer F depends on K; the diagnostic locus is foundation).
- [FR-087](FR-087-typestate-and-cross-package-node-key.md): the I2 reader
  and `WireNodeId` this requirement's package source map is read through.
- [US-005](../usecase/US-005-trust-checked-identity-across-packaging.md).
- QSpec FR-322 (`ix://agent-ix/quire-specification/FR-322`).

## Status

Specified and implemented under QSL-159. TC-420, TC-421 and TC-422 pass
locally; TC-421's QSpec-fixture step runs under `make conformance`.

Three parts of O-12 and C-21 are not built. Each needs a `RawSourceRef`
for a source QSL reads itself. [FR-001](FR-001-read-exact-source.md) now
states where it comes from: the caller supplies the authority, identity and
revision namespace and value, and S0 mints the reference. ADR-013 §7 slice
S-4b (QSL-233) builds it, together with two of the parts: replacing
`LocatedSpan` in the canonical S0 to S2 diagnostics with `SourceRegion`, and
C-21's embedded-body span to document region. The third part, check-stage
regions, is [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)'s
`check::Location` resolution in slice S-5b. It also needs the parsed forms'
expression spans (FR-091-AC-10, QSL-141).
