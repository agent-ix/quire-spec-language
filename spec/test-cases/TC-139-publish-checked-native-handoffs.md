---
id: TC-139
title: "Publish and read checked native handoffs"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-051, type: verifies }
---
# TC-139: Publish and read checked native handoffs

## Description

Owner-boundary checks for the implemented FR-051 public APIs.

## Test Procedure

**Steps for FR-051-AC-1 through FR-051-AC-5 RETIRED by QSL-303 (M-6d).**
`protocol_artifact::checked_predicate` and `protocol_artifact::temporal_subject`
are deleted, along with their only tests, formerly in
`tests/it/compiled_protocol_v2.rs` (that file remains, now scoped to TC-138's
unrelated coverage): with QSL-21d/e/f (QSL-309/300/301) landed, the spine
`ProtocolClause` path no longer needs this producer/consumer round trip. The
procedure below described that deleted surface and no longer runs.

~~Integration, property and compile-fail tests derive both documents from real
parse/link/check/compiled-protocol-v2 admission, read them against the same and
foreign admitted packages, recompute schema/content digests independently, and
mutate every field and boundary described by FR-051. Exact and one-over limits
cover bytes, depth, strings, populations, expressions and visited fields.~~

~~The compile-fail surface attempts public construction and evaluator/callback
injection. Runtime vectors attempt a textual AST, self-asserted trust/total
flags, false/numeric leaves, foreign spans, clock/profile/capture substitutions,
unknown history kinds and noncanonical JSON. A guarded temporal fixture selects
the exact activation guard as a checked predicate while proving that it does not
enter the temporal formula's reachable `holds(...)` population. Guard and
formula document bytes are cross-wired against the other's selection, and
trigger and anchor handles remain invalid predicate selections.~~

**FR-051-AC-6 stays live**, amended to the production-dependency-graph fact:
`arch-lint duplicate-revisions --lockfile Cargo.lock` (FR-061), run by
`make ci`, confirms that `Cargo.lock` holds one copy of each first-party
crate, so no dependency of QSL resolves a second QSL copy and the graph is
cycle-free; the production dependency key `quire-contract-ir` resolves to the
`quire-contract-model` package.

## Expected Results

FR-051-AC-1 through FR-051-AC-5 (retired): no longer applicable; the surface
they described does not exist.

FR-051-AC-6 (live): `arch-lint duplicate-revisions --lockfile Cargo.lock`
exits 0 in `make ci`.

## Status

Passing for `quire-spec-language#90`; activation-guard coverage extended by
`quire-spec-language#98`. FR-051-AC-1 through FR-051-AC-5 retired by QSL-303
(M-6d); FR-051-AC-6 amended to the still-live cycle-free production-graph
check, which `make ci` runs as `arch-lint duplicate-revisions` (QSL-334).
