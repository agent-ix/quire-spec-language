---
id: TC-1076
title: "A state clause or a holds atom names a population as an operand or a domain"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: verifies
---
# TC-1076: A state clause or a holds atom names a population as an operand or a domain

## Description

Verify that `M::P` resolves to a domain-package population inside a state clause or a `holds` atom, types as a population operand or domain only, takes the clause's observation, becomes a domain of the requirement record and evaluates over the snapshot's population. Scope: FR-391-AC-1 to FR-391-AC-9.

Catches a name resolved as an enum value or left unresolved, a population accepted as a value (compared, bound by `let`, or returned), a population read at `current` inside a precondition, a named population missing from the record's domains, and an incomplete population decided as an empty set.

## Test Procedure

Fixture: the `Counter` unit of FR-155-AC-1 (model alias `M`, population `counters`, operations `bump()` on `Counter` and `grow()` on `Widget`), plus a second population `archive` of `Counter`, a type `Widget` covered only by a population `widgets`, and a `bound B on M::counters [3]` where a row says so. Compile one package per row and check it; evaluate at S6a where a row says so.

| # | Input | Expected |
| --- | --- | --- |
| 1 | `pre Room using v on M::Counter::bump { forall(o in allInstances<M::Counter>(M::counters) : o.v < 2) }` | checks as a `Precondition`; operand `Population<Counter>`; result `Set<Reference<Counter>>`; every read `pre` |
| 2 | `invariant Cap using v on M::Counter at current { forall(o in allInstances<M::Counter>(M::counters) : o.v < 4) }` | checks; reads `current` |
| 3 | operand `M::nothing`; alias `M` also an enum name | `missing_declaration`/`missing-name` at `nothing`; `ambiguous_declaration`/`ambiguous-name` at the name |
| 4 | `M::counters = M::counters`; `let q = M::counters in allInstances<M::Counter>(q)`; `size(M::counters)`; body `M::counters`; `M::Counter = M::Counter` | the first four each `ill_typed`/`operator-ineligible` at the name; the last `missing_declaration`/`missing-name` at `Counter` |
| 5 | row 1 with the bound | operand `Population<Counter>[3]`; result `Set<Reference<Counter>>[0,3]`; the record's `counters` domain `Population(Some(3))` |
| 6 | row 1 with the second population `archive`; then `pre W using v on M::Widget::grow { forall(o in allInstances<M::Counter>(M::counters) : true) }` | `ambiguous_declaration`/`ambiguous-name` at the `on` of row 1; the second records `widgets` and `counters` as domains |
| 7 | evaluate row 1 over `counters` holding `a`, `b`, `c` (complete); over an incomplete `counters`; `allInstances<M::Other>(M::counters)` with `Other` covered by no member type of `counters` | exactly `a`, `b`, `c` in canonical key order with one `population.visit` charge per member; `Incomplete` with `incomplete_population`/`incomplete-scope`; FR-084's typed incomplete result naming `Other`, `FamilyResult::Undefined` with reason `incomplete-closure`, and no collection |
| 8 | with `archive` present, `pre A using v on M::Widget::grow { forall(o in allInstances<M::Counter>(M::counters) : true) }` and `pre B` naming `M::archive` instead, and `pre C` naming `M::counters` as `A` does; their read footprints; then emit the package | two different checked terms, each carrying its own population's declaration key; of the populations `A` reads `AnyMembership{counters}` and not `AnyMembership{archive}`, and `B` the reverse (each footprint also holds the receiver's own `Membership` per FR-155); emission yields two population reference nodes, one per population, `C` references the same node as `A`, and the package is admitted by the full reader |
| 9 | on the `Counter` unit with `bound B on M::counters [3]` and `Watch`/`watchers`: `post P using v on M::Counter::bump { forall(o in M::counters : o.v < 2) }`; `pre Q using v on M::Counter::bump { exists(o in M::counters : o.v = 2) }`; `invariant Cap using v on M::Counter at current { count(o in M::counters where o.v = 0) < 4 }` with and without the bound; `temporal C using t0 over (w: M::Watch) clock "model-steps" on origin { eventually holds(forall(x in M::counters : x.v < 2)) }`; then evaluate each over `counters` complete and not complete | each checks with the population as a domain of its clause or temporal clause (a postcondition reads `post`, a precondition `pre`, the invariant and the atom `current`); the `count` is typed `Int[0,3]` with the bound and unbounded above without it; evaluation equals the `allInstances` spelling, and `Incomplete` with `incomplete_population`/`incomplete-scope` when `counters` is not complete |
| 10 | `M::counters` in a function body and in the type-level invariant of an object type; `seq(i in 0 .. 2: i)` in an invariant | `missing_declaration`/`missing-name` at `counters` (both); `unsupported_construct`/`expression-form` at the `seq` |
| 11 | `terminal Done using v on M when { forall(c in M::counters : c.v = 2) };` on the `Counter` unit; evaluate its predicate over `counters` holding `a`, `b` with `v` 2 and 2, and with `v` 2 and 1; read the requirement of the deadlock-freedom item of a request with a temporal item over the unit's subject (FR-124-AC-2) | checks to `When` with `counters` read at `current` and a domain of the item's requirement; the predicate is true, then false |

Tag the tests `#[trace("TC-1076", "FR-391-AC-n")]`.

## Expected Results

The rows give the outcomes in the table. A mutant that accepts a population as a value passes rows 1 to 3 and fails row 4; a mutant that refuses a population domain in a postcondition, precondition, invariant or `holds` atom fails row 9, and one that resolves `M::P` in a function body fails row 10.
