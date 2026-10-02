---
id: FR-335
title: "Settle a claim over an unbounded declaration end to end"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-076
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-333
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-144
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-346
    type: depends_on
---
# FR-335: Settle a claim over an unbounded declaration end to end

## Description

A claim whose roots include an unbounded declaration (FR-333) SHALL reach a
settlement through the spine, the request writer and negotiation exactly as
ADR-014 §4, §6 and §10 scenarios 2 and 3 state, and never by narrowing the
declaration. With no capable backend registered, the claim SHALL settle
`unsupported` with a warning naming the capability. With only a
bounded-mode backend, it SHALL settle `requires-bound` when every unbounded
domain is boundable, and `unsupported`, warned, `unbounded-extent` when one
is not. A request that supplies a `ProofBound` per domain SHALL become its
own bounded item. S6a evaluation of the claim on one concrete value is a
test of that value.

## Use case

An author declares `s: Set<Int[0, 9999]>` with no bound and writes a claim
over it. Before any proof backend exists they learn the claim is
`unsupported` and why. When Kani is registered they learn that a bound
would help, ask again with `Cardinality{maximum: 8}`, and get a result that
says it holds for sets of at most 8 members, while the unbounded claim
stays unproved.

## Inputs

- A unit with a Value function `f using v(s: Set<Int[0, 9999]>): Integer pure
  { size(s) + 1 }`, whose `+` application is a `value-validity` claim with
  root `s` (FR-062-AC-13, ADR-014 §4).
- The registry (FR-075), the request writer (FR-097) and a negotiation
  disposition for each item (CG `negotiate_*` produces it).
- Optionally a map from `DomainKey` to `FiniteBound`.
- For S6a, a concrete value of `s`.

## Outputs

- The claim's requested items, their dispositions, and their O-16
  categories.

## Behavior

- The claim's requirement record SHALL be (`value-validity`,
  `Unbounded{[s's DomainKey]}`) with domain kind collection, computed from
  the checked types and never from the caller (ADR-014 §6 step 5).
- When no registrant advertises `value-validity`, the item SHALL settle
  `unsupported`, warned, naming `value-validity` (FR-076, QSpec FR-290).
- The request writer SHALL write the item with `finite_bound_available`
  true when every unbounded domain is boundable, and a `requires-bound`
  disposition for it SHALL settle it `requires-bound`, with no harness
  emitted.
- When a root's domain is not boundable (a quantity, or an FR-228-AC-5
  loop), the request writer SHALL write `finite_bound_available` false, and
  an `unbounded-extent` disposition SHALL settle the item `unsupported`,
  warned, naming the kind, the candidate and its modes.
- A bounded request with one `FiniteBound::Cardinality` for `s` SHALL be
  written as a new item with its own request index, classified `bounded`,
  whose obligation identity includes the substituted bound (FR-097-AC-3).
  The unbounded item SHALL keep its `requires-bound` disposition, and a
  result on the bounded item SHALL join only the bounded item's request
  index.
- S6a SHALL evaluate the function on a concrete set of any size with no
  cardinality refusal, stopping only on the caller's meter (FR-097-AC-7),
  and the result SHALL be evidence for that value only (ADR-014 §8).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-335-AC-1 | Compiled from source and requested with an empty registry, `f`'s `+` claim settles `unsupported` with a warning naming `value-validity`; its record is (`value-validity`, `Unbounded`) with one collection domain at `s`. | Test (TC-845) |
| FR-335-AC-2 | The claim is written with `finite_bound_available` true; with only a test descriptor advertising (`value-validity`, `bounded`) and a fixture `requires-bound` disposition it settles `requires-bound`, and no harness is emitted. | Test (TC-845) |
| FR-335-AC-3 | A bounded request with `ProofBound{s, Cardinality{maximum: 8}}` writes a second item with its own request index, classified `bounded`, whose obligation identity differs from the unbounded item's and from that of a request with `maximum: 9`; the unbounded item still settles `requires-bound`. | Test (TC-845) |
| FR-335-AC-4 | The same function with an added root of a quantity type, read by the `+` application, is written with `finite_bound_available` false and, with a fixture `unbounded-extent` disposition for the bounded-only descriptor, settles `unsupported`, warned. | Test (TC-845) |
| FR-335-AC-5 | At S6a, `f` on the concrete set `{0, 1, …, 999}` of 1,000 distinct members of `Int[0, 9999]` returns 1,001 with no cardinality refusal; with a meter too small for the construction it returns `Incomplete` at the `collection.bound` or element charge point; neither result changes the AC-1 item's disposition. | Test (TC-845) |

## Dependencies

- ADR-014 §4, §6 step 5, §8, §10 scenarios 2 and 3; ADR-012 §7.1 and §7.3.
- [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md)
  (extent, request writer, bounded requests),
  [FR-062](FR-062-implement-checked-family-contract.md) AC-13 (operation
  application records), [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-076](FR-076-settle-backend-absence-as-unsupported.md),
  [FR-333](FR-333-read-an-optional-collection-bound-and-population-maximum.md).
- QSL's tests supply each negotiation disposition and backend result as a
  fixture and run no CG or driver code; the end-to-end run through CG
  `negotiate_*` and the driver lives in quire-integration.
- QSpec FR-144-AC-12, FR-290, FR-346.

## References

- Linear QSL-385 (spec ticket); QSL-42 (implementation).
- QSpec half: QSpec FR-144 (merged).
