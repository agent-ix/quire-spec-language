---
id: FR-097
title: "S-6: classify claim extent and write bounded requests"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-011
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-010
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
---
# FR-097: S-6: classify claim extent and write bounded requests

## Description

ADR-013 §7 slice **S-6** (Linear QSL-140) builds the interfaces ADR-014 §11
names for it. This requirement states their observable behaviour. ADR-014 is
the design authority; this requirement adds no rule of its own.

- F `bound`: `DomainKey`, `FiniteBound`, `ProofBound` and `IntervalKey`
  (ADR-014 B-4, TR-3).
- The layer-3 extent of a requested item, `ClaimExtent`, computed by the
  ADR-014 §4 extent rule (`qsl_semantics::family`).
- The O-20 request writer (`qsl_route::request`): the FR-331 extent
  classification with its available finite bound, and bounded requests.
- `qsl_eval::simulation::explore::Outcome::category()` (ADR-014 §7).
- The kernel field shapes `CollectionType.bound: Option<CardinalityBound>`
  and `ValueType::Population(Option<u64>)` (ADR-014 N-3).

## Inputs

- A claim's argument and bound-variable types, each with the wire id of the
  checked node that carries it.
- The package's type environment.
- For a bounded request, a map from `DomainKey` to `FiniteBound`.

## Outputs

- `ClaimExtent::Bounded`, or `ClaimExtent::Unbounded` with each unbounded
  domain's key and kind.
- One requested item per call, with its own request index, its FR-331 extent
  classification and, for a bounded request, its proof bounds.
- A refusal `invalid_runtime_input`/`invalid-value` for bad bounds.

## Behavior

The extent rule, the available finite bound and the bounded request are
ADR-014 §4's. The O-16 map for exploration is ADR-014 §7's. An absent bound
means unbounded (ADR-014 §2). No accounting limit, stage limit, profile
ceiling or backend budget converts into a proof bound (ADR-014 §1).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-097-AC-1 | `FiniteBound::integer_range` refuses `lower > upper`, `FiniteBound::depth` refuses zero, and each admitted bound reports its kind. `IntervalKey::new` refuses `lower > upper`, and two keys that differ in any of `lower`, `upper`, profile or clock binding are unequal. `DomainKey`s order by node, then path. | Test (TC-436) |
| FR-097-AC-2 | Over a claim's argument types, each collection with no bound, population with no maximum, `Integer` with no range, recursive record or tuple, and quantity is exactly one unbounded domain, keyed by its node and child-index path, with its kind; a quantity's domain takes no finite bound. Every other type adds none, and a claim with none is `Bounded`. Classifying the same roots twice gives equal extents. The walk stops with a node-count stage limit at its ceiling, and a composite missing from the environment is an internal fault. | Test (TC-437) |
| FR-097-AC-3 | The request writer classifies a bounded item `bounded`, and an unbounded item `unbounded` with `finite_bound_available` true exactly when every domain is boundable; a loop or infinite-trace domain makes it false. A bounded request that supplies one bound of the right kind per domain is written as its own item, with its own request index, classified `bounded`, carrying its proof bounds in key order; the unbounded item keeps its own index and classification. Two bounded items with different bounds are different items. | Test (TC-438) |
| FR-097-AC-4 | The writer refuses, in this order: a bounded request for an item with no unbounded domain; a bound whose key names no unbounded domain of the item; an item with any domain no finite bound can stand for (loop, infinite trace, quantity), whether or not a bound was supplied for it; then, domain by domain in key order, a missing bound or a bound of the wrong kind. Each refusal is `invalid_runtime_input`/`invalid-value`, and no item is written. | Test (TC-438) |
| FR-097-AC-5 | `explore::Outcome::category()` maps `Exhaustive` to success and `Bounded` and `Cancelled` to incomplete; the stopped outcomes keep their frontier and, for `Bounded`, the limit reached. | Test (TC-439) |
| FR-097-AC-6 | For records QSL checks and emits, IR's v2 lowering at the pinned revision with `require_bounds` returns `RequiresBound` exactly when QSL's extent is `Unbounded`, and IR's first unbounded node is a form QSL names as a domain. | Test (TC-440) |
| FR-097-AC-7 | An unbounded collection type admits a collection value of any size with no cardinality refusal; it still charges `collection.bound`, and stops with `Incomplete` at that charge point only when the caller's meter runs out. `K<T>` and `K<T>[0, u64::MAX]` are different types and different v2 nodes. The checker types `map`, `flatMap`, `filter` and `flatten` over an unbounded source as unbounded, and proves no size maximum for it. | Test (TC-441) |
| FR-097-AC-8 | A `Population<T>` with no declared maximum is unbounded: `allInstances` over it checks to an unbounded `Set<Reference<T>>` and selects every member with no cardinality refusal; a population parameter admits a binding only when their declared maxima are equal, absence included; and it lowers to its own v2 node, distinct from an unbounded `Set<Reference<T>>`. | Test (TC-441) |

## Dependencies

- [ADR-014](../decisions/ADR-014-temporal-trace-and-boundedness-architecture.md)
  §1, §2, §4, §7, §9 N-3 and §11.
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-19, O-20, O-21 and the §7 S-6 row.
- [FR-062](FR-062-implement-checked-family-contract.md) AC-4:
  `FamilyContract::requirements()`, which returns the `Requirements` value
  carrying this requirement's `ClaimExtent`.
- [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md) AC-3:
  the witness envelope's `run_limits` and `FiniteBound` declared domains.
- IR at the revision `qsl-package/Cargo.toml` pins, for AC-6.

## Status

Specified and implemented under QSL-140. TC-436 to TC-439 and TC-441 pass
locally. FR-097-AC-8's lowering clause is not implemented: until QSL-42 gives
an unbounded population its own node, lowering one refuses with
`UnrepresentableBound` rather than writing the bare set node an unbounded
`Set<Reference<T>>` also has. TC-441 step 5 checks that interim refusal. TC-440 is partly passed: its agreeing fixtures pass, and an ignored
test asserts agreement for three fixtures IR's predicate at the pinned revision gets
wrong. IR-283: IR's `requires-bound` does not distinguish positions, so a
`bounded_domain` over the shared `integer` scalar bounds every integer
position, and the `collection_bounds` literals typed at that node read as an
unbounded integer. IR-284: IR has no recursion rule. A quantity fixture is not
compared yet: the emitter omits a record whose field names a declared unit
node, because lowering does not build that node. The ignored test is
un-ignored when both land.
