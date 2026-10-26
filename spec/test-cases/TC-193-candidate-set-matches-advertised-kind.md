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
kind for ordinary items and, for `probabilistic-satisfaction`, only those that
advertise the item's exact evidence kind; that a state-graph item is matched
by exact advertised form; that an advertised mode does not filter candidacy;
and that the registry module defines no second
capability-kind enum. Scope: FR-075-AC-1, FR-075-AC-5, FR-075-AC-19 to
FR-075-AC-22.

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
6. Build a registry with backend E advertising `probabilistic-satisfaction`
   with `{statistical}` and backend F advertising it with `{exact}`. Compute
   unnamed candidates for probabilistic items naming `statistical` and
   `exact`.
7. Run a source scan over the registry module (FR-075's implementation) for
   any enum or struct definition whose variants or fields name an FR-290
   capability-kind label, distinct from an import of the canonical
   `Capability` type.
8. Build a registry with EN-6's `qsl-model-check` (no `dynamics`) and
   `hybrid-fixture` (`dynamics` `{hybrid}`, both solver budgets in `bounds`),
   in both orders; compute candidates for `SlowEnough`'s item, carrying
   `hybrid-dynamics`, for a temporal item without it, for the hybrid item
   naming `qsl-model-check`, and for a `value-validity` item; repeat with
   `hybrid-fixture`'s `dynamics` `{discrete, hybrid}` and with
   `qsl-model-check` alone.
9. Admit each `dynamics` vector of FR-075-AC-20 through
   `descriptor_from_manifest`.
10. Register an EN-1 descriptor advertising `temporal-satisfaction` and all
    three state-graph forms. For each checked state-graph form, build a request
    item carrying that form and compute its candidates. Repeat with descriptors
    that omit the member and that advertise only `possible`; also compute a
    temporal-formula item's candidates against those descriptors.
11. Name the form-less descriptor for a state-graph item, and compute
    candidates for each state-graph form with a registry containing the
    descriptor that advertises only `possible`.
12. Admit `state_graph_forms` values `[]`, [`possible`, `possible`],
    [`eventually-possible`], a non-array value, and a list on a descriptor
    without `temporal-satisfaction`. Register two otherwise equal descriptors
    of one identity whose state-graph form sets differ.

## Expected Results

Step 2 returns a candidate set containing exactly Backend A. Step 3 returns
a candidate set containing exactly Backend B. Step 4 returns an empty
candidate set (Backend C does not advertise `value-validity`). Step 5
returns a candidate set containing exactly Backend D: the item's unbounded
extent does not exclude a bounded-only registrant from candidacy, because
mode is not compared at this stage. Step 6 returns E only for the
`statistical` item and F only for the `exact` item; neither evidence kind
substitutes for the other. Step 7 finds no locally defined capability-kind
type; every capability-kind match in the registry module resolves to the one
canonical `Capability` type. Step 8 returns the sets FR-075-AC-19 states, equal
for both orders. Step 9 refuses or admits each vector as FR-075-AC-20
states.

Step 10 returns the EN-1 descriptor as the candidate for each exact
state-graph form. A descriptor with no `state_graph_forms` is admitted but is
never a candidate for a state-graph item; the descriptor listing only
`possible` is a candidate only for that form. The temporal-formula candidate
set is unchanged by the presence or absence of the member. Step 11 returns an
empty candidate set for every form not advertised by the named or registered
backend. Step 12 refuses each malformed descriptor before insertion with
`invalid_capability`/`invalid-state-graph-forms`, keyed by backend identity,
leaving an earlier registration standing; the two otherwise equal descriptors
with different form sets conflict under FR-075-AC-4.
