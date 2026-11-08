---
id: FR-566
title: "Keep QSL outcomes unchanged by a registered process plugin"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-565
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-305"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-339"
    type: depends_on
---
# FR-566: Keep QSL outcomes unchanged by a registered process plugin

## Description

FR-289 states that installing, removing or reordering providers changes
results only through each item's candidate set, and tests it with
manifests. A third-party provider arrives as a process plugin (ADR-029 PL-1),
whose descriptor has origin `Process` (FR-288). This requirement extends
FR-289 to that case over the FR-565 matrix.

The test plugin manifest is a QSpec FR-331 manifest for the `BackendId`
`qsl.test-plugin` that advertises every (kind, mode) pair the FR-565 cases'
items do not use, and nothing else. Its descriptor is built with
`descriptor_from_manifest` and origin `Process`, as the driver builds one
from a plugin's `hello` frame.

For every `Case` of FR-565, QSL's outcome SHALL be byte-equal across three
configurations:

1. no provider descriptor registered and no provider added to `Engines`;
2. the linked descriptors the first slice registers (ADR-029 SL-1: the Kani
   backend) and `Engines::qsl()`;
3. configuration 2 with the test plugin's `Process` descriptor registered and
   a provider under that descriptor added to `Engines`.

An item whose kind the test plugin does advertise gains exactly
`qsl.test-plugin` in its candidate set, and every other item's candidate set
is unchanged (FR-289 item 3).

## Inputs

The FR-565 cases; the test plugin manifest; registries and `Engines` values
in the three configurations.

## Outputs

FR-286 documents and items; candidate sets (FR-075).

## Behavior

- For each FR-565 case, QSL shall return byte-equal documents or items in
  the three configurations.
- When a `Process` descriptor that does not advertise an item's matching
  capability is registered, layer R shall give that item the same candidate
  set.
- When a `Process` descriptor that advertises an item's matching capability
  is registered, layer R shall add exactly that descriptor's `BackendId` to
  the item's candidate set.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-566-AC-1 | For every `Case` of `lifecycle_matrix()`, `run_case_with` over configurations 1, 2 and 3 returns byte-equal document or item bytes and equal exit codes; FR-275-AC-1's `package` bytes are byte-equal in the three. | Test (TC-1395) |
| FR-566-AC-2 | For FR-281-AC-1's item, the candidate set in configuration 3 equals configuration 2's; for an item of a kind the test plugin advertises, configuration 3's candidate set is configuration 2's plus `qsl.test-plugin`, and registering the plugin's descriptor before or after the linked ones gives equal candidate sets. | Test (TC-1395) |

## Dependencies

- ADR-029 PV-1, PV-2, PL-1, PL-5.
- [FR-288](FR-288-build-the-registry-from-provider-manifests.md): `descriptor_from_manifest` and `ProviderOrigin`.
- [FR-289](FR-289-keep-results-unchanged-by-provider-installation.md): the invariants this extends.
- [FR-281](FR-281-analyze-claims-with-in-process-engines.md): `Engines` and providers under a descriptor.
- [FR-565](FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md): the cases.
- QSpec FR-305, FR-339-AC-3.

## Overlap

The driver repository runs the same cases through `quire` with a real
process plugin executable, speaking `quire.plugin-wire/v1`, named and not
named on the command line, and asserts byte-equal documents and equal exit
codes.

## References

- QSL-393 (V1-A06): provider negotiation that leaves results unchanged.
