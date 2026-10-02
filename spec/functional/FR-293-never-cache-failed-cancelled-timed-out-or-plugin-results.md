---
id: FR-293
title: "Mark which QSL results the cache never stores, and keep wall time out of stored records"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-292
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: references
  - target: "ix://agent-ix/quire-specification/FR-306"
    type: depends_on
---
# FR-293: Mark which QSL results the cache never stores, and keep wall time out of stored records

## Description

QSpec FR-306 states what the cache never stores: a result whose category is
internal failure, a cancelled or deadline result, a time-limited
`incomplete`, any member recording wall time, and every result a plugin
produced (ADR-029 CA-5, ruling RU-5). The driver's `quire-cache` applies
it.

QSL's part is that its terminal records let the cache tell these apart
without reading text:

- the record's O-16 category;
- a typed cause that distinguishes cancellation (FR-276), a deadline or other
  time limit, and a deterministic budget (work units, state or transition
  budgets);
- the producing provider's `BackendId` (FR-288), which marks a plugin's
  result (FR-290).

A QSL `analyze` record holds no member that records wall time, so an
`incomplete` record whose cause is a deterministic budget is a function of
the key and can be stored.

## Inputs

A QSL terminal record.

## Outputs

The record's category, its typed cause and its producer's `BackendId`.

## Behavior

- Each QSL terminal record shall carry its O-16 category and its producer's
  `BackendId`.
- An `incomplete` record's typed cause shall distinguish cancellation, a
  time limit and a deterministic budget, and name the budget for the last.
- A QSL `analyze` record shall hold no member that records wall time.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-293-AC-1 | An `analyze` record settled `incomplete` by a cancelled `Cancel`, one settled `incomplete` by a `Deadline` cancel, and FR-281-AC-6's record settled by a state budget of 1 carry typed causes that are pairwise distinct, the last naming the state budget; a plugin's `proved` record carries the plugin's `BackendId`. | Test (TC-777) |
| FR-293-AC-2 | FR-281-AC-1's and FR-281-AC-6's records, serialized and read back with the typed reader, hold no member that records a time, and two runs of each give equal bytes. | Test (TC-777) |

## Dependencies

- ADR-029 CA-5: what is never stored; ruling RU-5.
- [FR-276](FR-276-cancel-a-lifecycle-operation.md): cancellation causes.
- [FR-288](FR-288-build-the-registry-from-provider-manifests.md): `BackendId`.
- [FR-290](FR-290-settle-plugin-results-as-typed-terminal-records.md): plugin results.
- [FR-292](FR-292-key-cached-results-by-content-identity.md): the canonical `analyze` record.
- QSpec FR-306: what the cache never stores.

## Overlap

The driver repository specifies and tests that `quire-cache` stores none of
the results QSpec FR-306 excludes, and stores a deterministic-budget
`incomplete` result.

## References

- QSL-390 (ARCH-50): ruling RU-5, recorded on the ticket.
- QSpec FR-306 (STD-141): the QSpec half.
