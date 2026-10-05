---
id: FR-288
title: "Build the backend registry from provider manifests by one conversion"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-080
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-331"
    type: depends_on
---
# FR-288: Build the backend registry from provider manifests by one conversion

## Description

The registry is the layer-R `qsl_route::Registry` value, and nothing else
registers backends (ADR-029 PV-1, FR-075, FR-080). It is built from two
sources: the compile-time providers linked into the binary, each giving its
FR-331 provider manifest, and the plugin processes the caller names, each
giving its manifest in its `hello` frame (FR-290). Layer R SHALL convert
every manifest into a `BackendDescriptor` by one conversion, whatever its
source. The registry reaches each consumer as an argument (ADR-012 §7.1).

A provider's `BackendId` is its backend identity alone, the identity
string its manifest states (ADR-013 O-19). The `BackendId` is the backend
member of the cache key (QSpec FR-306, FR-292), and a `proved` record names
the provider that produced it by its `BackendId` (FR-290).

`BackendId` is one type, defined in `qsl-foundation`, holding exactly that
identity. Every QSL crate that names a backend uses it: layer R's
registry and descriptors, and layer 6's replay envelopes and terminal
records. A `BackendDescriptor` holds the `BackendId` and the manifest's
advertised capabilities, and no other identity of the tool.

## Inputs

FR-331 provider manifests.

## Outputs

`BackendDescriptor` values, and the `Registry` built from them.

## Behavior

- Layer R shall convert an FR-331 provider manifest into a
  `BackendDescriptor` by one function, used for compile-time and plugin
  manifests alike.
- The conversion shall take the descriptor's `BackendId` from the
  manifest's backend identity, verbatim.
- Layer R and layer 6 shall name a backend by the one `qsl-foundation`
  `BackendId` type.
- Layer R shall give every `BackendDescriptor` a `ProviderOrigin`, `Linked` or
  `Process`. The conversion shall set `Linked` for a compile-time provider's
  manifest and `Process` for a plugin `hello` frame's manifest, and no
  manifest member shall state or change it.
- If two registrations of one `BackendId` differ in any member, origin
  included, then the registry shall withdraw both and record each
  conflicting manifest digest as `duplicate-backend`, whatever the order of
  registration; a registration equal in every member shall be idempotent.
- If a manifest fails the FR-331 manifest reader, then the conversion shall
  refuse it with the reader's typed cause, and the registry shall not hold
  it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-288-AC-1 | One manifest, passed to the conversion once as a compile-time provider's and once as the body of a plugin `hello` frame, gives two `BackendDescriptor` values that differ in their origin member alone. | Test (TC-772) |
| FR-288-AC-2 | A descriptor's `BackendId` equals the manifest's backend identity, verbatim; changing any one other member of the manifest (an advertised kind, a domain, an option, a bound) changes the descriptor's advertised capabilities and leaves its `BackendId` unchanged. | Test (TC-772) |
| FR-288-AC-3 | A manifest missing its advertised kinds is refused with the FR-331 reader's cause, and a registry built from it and one valid manifest holds only the valid one. | Test (TC-772) |
| FR-288-AC-4 | For an item routed to a provider and settled with a replayed counterexample, the `BackendId` the registry's descriptor holds, the one the replay envelope records and the one the terminal record names are equal values; two manifests that differ only in a tool version member give the same `BackendId`. | Test (TC-772) |
| FR-288-AC-5 | A descriptor built for a compile-time provider holds origin `Linked`, and one built for a plugin `hello` manifest holds `Process`; the `BackendId` is the same for both. | Test (TC-772) |
| FR-288-AC-6 | Registering `Linked` and `Process` descriptors of one `BackendId` and manifest, in either order, withdraws both and records the conflicting manifest digest as `duplicate-backend`; a registration equal in every member, origin included, is idempotent. | Test (TC-772) |

## Dependencies

- ADR-029 PV-1: registration.
- ADR-012 §7.1: the registry as an argument.
- ADR-013 O-19: `BackendId`, the backend identity alone.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md), [FR-080](FR-080-registry-evidence-and-gates.md): the registry.
- QSpec FR-331: the provider manifest.

## Overlap

The driver repository owns the `Provider` trait through which compile-time
providers give their manifests (ADR-029 PV-3).

## References

- QSL-393 (V1-A06): provider negotiation.
- QSpec FR-305, FR-462 (STD-141): the QSpec half.
- Implementation follow-up: replace `qsl_route::BackendId` and
  `qsl_replay::identity::Backend` with the one `qsl-foundation` `BackendId`;
  this record changes the specification only. `BackendDescriptor` already
  carries no tool identity.
