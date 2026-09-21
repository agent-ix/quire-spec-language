---
id: TC-193
title: "Candidate set matches registered backends advertising the requested kind"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-193: Candidate set matches registered backends advertising the requested kind

## Description

Verify that the registry's candidate-set computation for an unnamed request
returns exactly the registered backends that advertise the item's capability
kind, and none that do not; that an advertised mode does not filter
candidacy; and that the registry module defines no second capability-kind
enum. Scope: FR-075-AC-1, FR-075-AC-5.

Catches an implementation that matches candidates by backend registration
presence alone (any registered backend is a candidate for any item,
ignoring `advertises`); an implementation that matches by mode instead of,
or in addition to, kind (a bounded-only registrant is silently excluded
from an unbounded item's candidate set because the matcher compares extent
before it should, when ADR-012 §7.2 reserves that comparison for
`negotiate_*`); and an implementation that reintroduces a local
capability-kind enum (for example a QSL-only `Capability`-shaped type used
only inside the registry module) instead of importing the canonical type,
which would compile and pass every other case in this test case while
still violating FR-075-AC-5.

## Test Procedure

1. Build a registry with two backends: Backend A advertising
   `value-validity`, Backend B advertising `operation-contract`.
2. Compute the candidate set for an item requiring `value-validity` with no
   named backend.
3. Compute the candidate set for an item requiring `operation-contract` with
   no named backend.
4. Build a second registry with one backend, Backend C, advertising
   `temporal-satisfaction`, and compute the candidate set for an item
   requiring `value-validity`.
5. Build a third registry with one backend, Backend D, advertising
   `finite-replay` in `bounded` mode only. Compute the candidate set for an
   item requiring `finite-replay` whose declared extent is `unbounded`.
6. Run a source scan over the registry module (FR-075's implementation) for
   any enum or struct definition whose variants or fields name an FR-290
   capability-kind label, distinct from an import of the canonical
   `Capability` type.

## Expected Results

Step 2 returns a candidate set containing exactly Backend A. Step 3 returns
a candidate set containing exactly Backend B. Step 4 returns an empty
candidate set (Backend C does not advertise `value-validity`). Step 5
returns a candidate set containing exactly Backend D: the item's unbounded
extent does not exclude a bounded-only registrant from candidacy, because
mode is not compared at this stage. Step 6 finds no locally defined
capability-kind type; every capability-kind match in the registry module
resolves to the one canonical `Capability` type.
