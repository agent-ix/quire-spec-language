---
id: FR-289
title: "Keep results unchanged by installing, removing or reordering providers"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-080
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: references
  - target: "ix://agent-ix/quire-specification/FR-339"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-331"
    type: depends_on
---
# FR-289: Keep results unchanged by installing, removing or reordering providers

## Description

Installing, removing or reordering providers SHALL change results only
through each item's candidate set, and only for items whose capability kind
the change touches (ADR-029 PV-2):

1. **Front end.** `parse`, `select`, `check` and `package` (FR-278) take no
   registry, so their output is independent of it. One source compiled with
   an empty registry and with a populated one gives byte-equal v2 bytes and
   the same check outcome (QSpec FR-339-AC-3).
2. **Order.** Two registries built from the same manifests in any order are
   equal and give identical candidate sets (FR-080).
3. **Locality.** An item's terminal record is a function of the item, its
   candidate set and the routed provider's run. Adding a provider that does
   not advertise an item's kind leaves that item's candidate set and record
   unchanged.
4. **Meaning.** A `refuted` record always comes from a counterexample that
   S6a replay reproduced, whichever provider produced it. A `proved` record
   names the `BackendId` of the provider that produced it.

## Inputs

Registries built from sets of FR-331 manifests (FR-288); claim items.

## Outputs

Candidate sets (FR-075) and FR-331 terminal records.

## Behavior

- No front-end operation shall take a registry or read one.
- Layer R shall give equal candidate sets for an item from registries built
  from the same manifests in any order.
- When a provider that does not advertise an item's capability kind is added
  to or removed from a registry, layer R shall give that item the same
  candidate set.
- QSL's terminal record shall hold the value `refuted` only together with the
  S6a replay result that reproduced its counterexample.
- QSL's terminal record shall hold the value `proved` only together with the
  producing provider's `BackendId`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-289-AC-1 | Compiling `tests/fixtures/spine-compile.native` in a test that also builds an empty registry, and in one that builds a registry from three manifests, gives byte-equal `package` output and equal `check` outcomes. | Test (TC-773) |
| FR-289-AC-2 | For a property-generated set of manifests and items, adding a manifest that advertises none of an item's kinds leaves that item's candidate set equal, and adding one that advertises the item's kind adds exactly that backend to it. | Test (TC-773) |
| FR-289-AC-3 | A terminal record built with the value `refuted` and no reproducing replay result, or with the value `proved` and no `BackendId`, cannot be constructed through QSL's public API (`compile_fail` doctests with compiling controls); a record built from a reproducing replay result reads back `refuted` with that replay result. | Test (TC-773) |

## Dependencies

- ADR-029 PV-2: the four invariants.
- ADR-011 FB-07: replay through S6a.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md): candidate sets.
- [FR-080](FR-080-registry-evidence-and-gates.md): order invariance, backed by TC-194.
- [FR-278](FR-278-parse-select-check-and-package-as-library-operations.md): the front end.
- [FR-288](FR-288-build-the-registry-from-provider-manifests.md): the registry.
- [FR-098](FR-098-execute-a-replay-request.md): replay.
- QSpec FR-339-AC-3: installing a backend never changes syntax; QSpec
  FR-331: terminal records.

## References

- QSL-393 (V1-A06): provider negotiation that leaves results unchanged.
- QSpec FR-305 (STD-141): the QSpec half.
