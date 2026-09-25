---
id: TC-445
title: "The witness-packet and replay-request readers refuse an unknown package contract version"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: verifies
---
# TC-445: The witness-packet and replay-request readers refuse an unknown package contract version

## Description

Verify that `WitnessEnvelope::reconstruct` and `ReplayRequest::decode` each
refuse a `package_contract_version` that is not exactly the one admitted
package contract version (`quire.checked-package/v2`), with the catalog's
unsupported-wire refusal (`unknown_wire`/`unsupported-wire`, ADR-013 O-22)
naming the actual version supplied. Before this test, both readers only
checked that the member was present, never its value, so any string passed
decode/reconstruction unchecked. Scope: FR-070-AC-8, FR-071-AC-8.

## Test Procedure

1. Build a real, otherwise well-formed witness packet (as
   `WitnessEnvelope::reconstruct` admits it) and mutate its
   `package_contract_version` to an unknown version string; reconstruct it.
2. Build a real, otherwise well-formed replay request wire (as
   `ReplayRequest::decode` admits it) and mutate its
   `package_contract_version` to an unknown version string, alongside a
   malformed byte-provision entry that would only be reached if the version
   check did not run first; decode it.

## Expected Results

- Both mutated inputs refuse with a typed `UnknownPackageContractVersion`
  cause carrying the actual (unknown) version string, rendering as
  `unknown_wire/unsupported-wire: <actual> is not the admitted
  quire.checked-package/v2 package contract version`.
- The request's refusal is observed before the byte provision is read: the
  malformed byte-provision entry never surfaces as `ByteDigestMismatch`.

## Status

Passed locally (QSL-235), `qsl-replay/src/witness.rs::envelope_tests::tc_445_refuses_an_unknown_package_contract_version` and `qsl-replay/src/request.rs::tests::tc_445_refuses_an_unknown_package_contract_version`.
