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

**Steps for FR-051-AC-1 through FR-051-AC-5 RETIRED (M-6d).**
`protocol_artifact::checked_predicate` and `protocol_artifact::temporal_subject`
are deleted, along with their only tests, formerly in
`tests/it/compiled_protocol_v2.rs` (that file remains, now scoped to TC-138's
unrelated coverage): with the spine `ProtocolClause` frame slices landed, the spine
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
`arch-lint direction` (FR-059), run over the four repositories' checkouts,
confirms that no dependency of QSL depends back on QSL, so the graph is
cycle-free; the production dependency key is `quire-contract-model`, the
package's own name.

## Expected Results

FR-051-AC-1 through FR-051-AC-5 (retired): no longer applicable; the surface
they described does not exist.

FR-051-AC-6 (live): `arch-lint direction` reports no FB-05 edge into QSL and
no FB-11 cycle.

## Status

Passing for `quire-spec-language#90`; activation-guard coverage extended by
`quire-spec-language#98`. FR-051-AC-1 through FR-051-AC-5 retired
(M-6d); FR-051-AC-6 amended to the still-live cycle-free production-graph
check, which `arch-lint direction` (FR-059) runs.
