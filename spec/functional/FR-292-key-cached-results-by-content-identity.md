---
id: FR-292
title: "Give analyze requests and records the canonical form the content-keyed cache reads"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-293
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: references
  - target: "ix://agent-ix/quire-specification/FR-306"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-201"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-322"
    type: depends_on
---
# FR-292: Give analyze requests and records the canonical form the content-keyed cache reads

## Description

QSpec FR-306 owns the cache: what is cached (`prove` terminal records,
`analyze` terminal records and `generate` artifacts), the key (a digest in
the QSpec FR-201 domain `quire.cache-key/v1` over the RFC 8785 bytes of the
operation, the subject's `package_id`, the canonical request, the provider's
backend identity and, for `generate`, the target), what stays outside the
key, invalidation, the store and its read check. The driver's `quire-cache`
implements it (ADR-029 CA-1 to CA-4).

QSL's part is what the cache reads from QSL:

- **The canonical `analyze` request.** QSL serializes the key members of an
  `analyze` request to one RFC 8785 form: the items by occurrence key, the
  claims, the domains and bounds, the options that affect a result, and every
  deterministic budget that can move a result to `incomplete` (work units,
  state and transition budgets). It holds no path, timestamp, wall-clock
  deadline, application version string or execution backend.
- **The `analyze` record as a function of the key.** `analyze` returns equal
  records for requests whose key members are equal, on the same
  `package_id` and engine `BackendId` (FR-280, FR-288).
- **The typed reader.** QSL supplies the typed reader that reads an
  `analyze` terminal record from its RFC 8785 bytes, which the cache uses to
  check a stored value as it checks a fresh result.

## Inputs

An `analyze` request, its `CheckedPackage`, and an `analyze` record's bytes.

## Outputs

The request's canonical key-member bytes; the `analyze` records; the typed
reader's record or typed refusal.

## Behavior

- QSL shall serialize an `analyze` request's key members to one RFC 8785
  form holding exactly the members the Description lists.
- The `analyze` operation shall return records with equal RFC 8785 bytes for
  requests whose key members, `package_id` and engine `BackendId` are equal.
- The typed `analyze` record reader shall return the record for the bytes
  QSL's serialization writes, and a typed refusal for any bytes that differ
  from the RFC 8785 re-encoding of the value they hold.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-292-AC-1 | Changing exactly one claim bound, one result-affecting option or the state budget of TC-762's request changes its canonical key-member bytes; changing the request file path, the working directory, the wall-clock deadline or the execution backend leaves them equal. | Test (TC-776) |
| FR-292-AC-2 | Two `analyze` runs of TC-762's request, with different working directories and request file paths, give records with equal RFC 8785 bytes. | Test (TC-776) |
| FR-292-AC-3 | The typed reader reads each AC-2 record back equal to the record serialized; the same record with whitespace inserted between members, or with its members reordered, is refused. | Test (TC-776) |

## Dependencies

- ADR-029 CA-1 to CA-4: what is cached, the key, invalidation and the store.
- ADR-013 O-19: `BackendId`.
- [FR-281](FR-281-analyze-claims-with-in-process-engines.md): `analyze` records.
- [FR-288](FR-288-build-the-registry-from-provider-manifests.md): `BackendId`.
- [FR-293](FR-293-never-cache-failed-cancelled-timed-out-or-plugin-results.md): what `analyze` records hold no wall time for.
- QSpec FR-306: the cache, its key and its read check; QSpec FR-201: the
  digest domain; QSpec FR-322: the `package_id` preimage.

## Overlap

The driver repository specifies and tests `quire-cache` against QSpec
FR-306: the key digest over every member, the store as the set of entries,
the read check, the atomic write, concurrent writers, eviction under the
caller's size limit, and a repeated `prove` read from the store.

## References

- QSL-390 (ARCH-50), QSL-393 (V1-A06): cache identity and invalidation.
- QSpec FR-306 and FR-201 `quire.cache-key/v1` (STD-141): the QSpec half, including the cache-key domain.
