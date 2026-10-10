---
id: TC-914
title: "Registry allocation failure preserves its native measurement through QSL admission"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: verifies
  - target: ix://agent-ix/quire-semantic-value/FR-108
    type: references
---
# TC-914: Registry allocation failure preserves its native measurement through QSL admission

## Description

Verify [FR-082](../functional/FR-082-resolve-conformance-subsetting-and-redefinition.md)
AC-10 through AC-13 through QSL's real expression-checking consumer of shared
type-environment admission. The fixture uses valid record, tuple, union and
object declarations with member links and an ancestor chain, sufficient stage
limits, and an uncancelled handle. It observes the native amount and unit at
the production reservation boundary delivered under
[QSV FR-108](ix://agent-ix/quire-semantic-value/FR-108).

## Test Procedure

1. Admit the fixture successfully through QSL and retain the complete
   environment and member-link results. Record its required work and ancestor
   counters for the configured-budget controls.
2. Through an isolated allocator seam at the production reservation boundary,
   deny a registry, index and traversal reservation in separate attempts.
   Capture the actual amount passed and whether its unit is bytes or
   additional elements. Assert the QSL public failure carries the identical
   amount and unit in a type-environment registry payload, catalog
   `resource_exhausted`/`allocation-failed`, through `StageFailure::Refused`.
   Assert absence of bound and setting, no work-counter interpretation, and
   no replacement limit, input/type refusal, Boolean, cancellation or fault.
   Exercise every native unit used by delivered production storage; a
   fabricated upstream error alone does not qualify this check.
3. Select a failure after earlier descriptors or member links have been
   staged. Observe the public admission result and caller-visible environment
   state: no descriptor prefix, index, member key join, environment or checked
   package escapes. Instrument actual evaluation entry and its meter and
   assert both invocation count and charge remain zero. Disable denial, retry
   the identical declarations, and compare the entire admitted environment
   and member links with step 1.
4. Trigger a reservation-size arithmetic or representation overflow through
   the production admission path without allocator denial. Assert the public
   failure preserves the upstream capacity/size reason and is distinct from
   `allocation-failed`; it invents no allocator-denied request measurement.
5. With allocation enabled, run the same fixture with work and ancestor
   limits at each required counter N-1 and N. Assert N-1 reports its existing
   `StageFailure::Limit`, kind, configured bound, actual denied counter and
   setting; N admits. Run independent malformed-declaration, cancellation,
   [FR-259](../functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md)
   AC-5's canonical 4096-byte allocation, and released checked-invariant cause
   controls. Assert their original classifications and payloads; the canonical
   `requested` remains bytes and is never a registry payload.

Tag the implemented consumer tests with `TC-914` and the corresponding
`FR-082-AC-10` through `FR-082-AC-13` criterion identifiers.

## Expected Results

A genuine allocator denial preserves the measured native request and unit in
QSL's registry resource refusal. Capacity/size overflow remains distinct, and
configured limits retain their bounds and counters. Denied admission exposes
no partial result and starts no evaluation; a retry succeeds with the complete
environment. Canonical allocation, malformed input, cancellation and invariant
failures retain their established contracts.

## Status

Planned consumer verification after the reviewed QSL and QSV contracts are
merged and IR-713 delivers its fallible production storage and carrier. This
test case specifies the required evidence; it does not claim an allocator
failure has already been executed or the QSL-500 allocation path delivered.
