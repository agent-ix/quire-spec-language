---
id: FR-280
title: "Serve the driver's lower, generate and prove operations at the QSL boundary"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: references
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-331"
    type: depends_on
---
# FR-280: Serve the driver's lower, generate and prove operations at the QSL boundary

## Description

The orchestrating driver owns `lower`, `generate` and `prove` (ADR-029 LC-1,
OP-1), because CG depends on QSL and a QSL crate that called CG would close a
package cycle (ADR-011 FB-11). The driver repository specifies those
operations. This requirement states what QSL supplies at that boundary:

- the `EmittedPackage` from the same run's `package` (FR-278), which is the
  input `lower` and `prove` take;
- for each `analyze` engine (FR-281), its FR-331 provider manifest, so the
  registry can route items to it (FR-288);
- an entry that runs the items of an FR-331 provider request routed to a QSL
  engine through `analyze`, and returns FR-331 results whose terminal records
  equal those `analyze` settles for the same items, so `prove` and `analyze`
  produce the same record for an item (ADR-029 OP-2);
- QSL's FR-331 terminal record type (`TerminalRecord`, ADR-013 O-24), the
  one record type both paths write.

## Inputs

For the provider entry: the `EmittedPackage`, the FR-331 provider request
with the items routed to one QSL engine, the provider limits and `&Cancel`.

## Outputs

FR-331 results: one terminal record per routed item.

## Behavior

- QSL shall publish one FR-331 provider manifest for each `analyze` engine,
  listing the (kind, mode) pairs, domains, options and bounds the engine
  accepts.
- When the driver passes an FR-331 provider request routed to a QSL engine,
  the QSL provider entry shall run the routed items through `analyze` and
  return one FR-331 terminal record per routed item.
- The terminal record the provider entry returns for an item shall equal the
  record `analyze` returns for the same item, package, subject binding and
  limits.
- No QSL crate shall depend on a CG, RT or driver crate. An IR crate that
  ADR-011 §6.1 sanctions (`quire-contract-model`) is allowed.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-280-AC-1 | Each QSL engine's provider manifest is read by the FR-331 manifest reader and converted by FR-288's conversion into a `BackendDescriptor` whose advertised (kind, mode) pairs equal the claim kinds the engine settles in TC-762. | Test (TC-761) |
| FR-280-AC-2 | For each TC-762 item, an FR-331 provider request that routes that item to its QSL engine, passed to the provider entry with the `EmittedPackage` of the same source, returns a terminal record equal to the one `analyze` returns for that item. | Test (TC-761) |
| FR-280-AC-3 | The `cargo tree` of every QSL core crate lists no CG, RT or driver crate; an IR crate ADR-011 §6.1 sanctions (`quire-contract-model`) is allowed (FR-284's direction check). | Test (TC-767) |

## Dependencies

- ADR-029 LC-1, OP-1, OP-2: the operation owners and `analyze` inside
  `prove`.
- ADR-011 FB-11: the package direction.
- ADR-013 O-24: the terminal record.
- [FR-278](FR-278-parse-select-check-and-package-as-library-operations.md): `package`.
- [FR-281](FR-281-analyze-claims-with-in-process-engines.md): `analyze`.
- [FR-288](FR-288-build-the-registry-from-provider-manifests.md): the manifest conversion.
- QSpec FR-300: the `lower` and `generate` operations; QSpec FR-331: the
  provider envelope.

## Overlap

The driver repository owns the requirements for `lower`, `generate` and
`prove`, the `Provider` trait (ADR-029 PV-3) and its adapter over this entry.
CG owns the process-provider negotiation arm (ADR-029 PV-4).

## References

- QSL-393 (V1-A06): typed library APIs.
- QSL-390 (ARCH-50).
- QSpec FR-300 (STD-141): the QSpec half.
