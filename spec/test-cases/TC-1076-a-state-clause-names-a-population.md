---
id: TC-1076
title: "A state clause names a population as the operand of allInstances and lookup"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: verifies
---
# TC-1076: A state clause names a population as the operand of allInstances and lookup

## Description

Verify that `M::P` resolves to a domain-package population inside a state clause, types as a population operand only, takes the clause's observation, becomes a domain of the requirement record and evaluates over the snapshot's population. Scope: FR-391-AC-1 to FR-391-AC-6.

Catches a name resolved as an enum value or left unresolved, a population accepted as a value (compared, bound by `let`, or returned), a population read at `current` inside a precondition, a named population missing from the record's domains, and an incomplete population decided as an empty set.

## Test Procedure

Fixture: the `Counter` unit of FR-155-AC-1 (model alias `M`, population `counters`, operations `bump()` on `Counter` and `grow()` on `Widget`), plus a second population `archive` of `Counter`, a type `Widget` covered only by a population `widgets`, and a `bound B on M::counters [3]` where a row says so. Compile one package per row and check it; evaluate at S6a where a row says so.

| # | Input | Expected |
| --- | --- | --- |
| 1 | `pre Room using v on M::Counter::bump { forall(o in allInstances<M::Counter>(M::counters) : o.v < 2) }` | checks as a `Precondition`; operand `Population<Counter>`; result `Set<Reference<Counter>>`; every read `pre` |
| 2 | `invariant Cap using v on M::Counter at current { count<M::Counter>(o in allInstances<M::Counter>(M::counters) : true) < 4 }` | checks; reads `current` |
| 3 | operand `M::nothing`; alias `M` also an enum name | `missing_declaration`/`missing-name` at `nothing`; `ambiguous_declaration`/`ambiguous-name` at the name |
| 4 | `M::counters = M::counters`; `let q = M::counters in allInstances<M::Counter>(q)`; `size(M::counters)`; body `M::counters`; the row 1 expression in a function body; `M::Counter` as a value | the first four each `ill_typed`/`operator-ineligible` at the name; the last two `missing_declaration`/`missing-name` |
| 5 | row 1 with the bound | operand `Population<Counter>[3]`; result `Set<Reference<Counter>>[0,3]`; the record's `counters` domain `Population(Some(3))` |
| 6 | row 1 with the second population `archive`; then `pre W using v on M::Widget::grow { forall(o in allInstances<M::Counter>(M::counters) : true) }` | `ambiguous_declaration`/`ambiguous-name` at the `on` of row 1; the second records `widgets` and `counters` as domains |
| 7 | evaluate row 1 over `counters` holding `a`, `b`, `c` (complete); over an incomplete `counters`; `allInstances<M::Other>(M::counters)` with `Other` covered by no member type of `counters` | exactly `a`, `b`, `c` in canonical key order with one `population.visit` charge per member; `Incomplete` with `incomplete_population`/`incomplete-scope`; FR-084's typed incomplete result naming `Other`, `FamilyResult::Undefined` with reason `incomplete-closure`, and no collection |
| 8 | with `archive` present, `pre A using v on M::Widget::grow { forall(o in allInstances<M::Counter>(M::counters) : true) }` and `pre B` naming `M::archive` instead; their read footprints; then emit the package | two different checked terms, each carrying its own population's declaration key; of the populations `A` reads `AnyMembership{counters}` and not `AnyMembership{archive}`, and `B` the reverse (each footprint also holds the receiver's own `Membership` per FR-155); emission refuses `unsupported_construct`/`not-yet-implemented` |

Tag the tests `#[trace("TC-1076", "FR-391-AC-n")]`.

## Expected Results

The rows give the outcomes in the table. A mutant that accepts a population as a value passes rows 1 to 3 and fails row 4; a mutant that resolves `M::P` outside a state clause fails row 4's last case.
