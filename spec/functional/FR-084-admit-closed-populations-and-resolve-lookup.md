---
id: FR-084
title: "Admit closed populations and resolve typed lookup and allInstances"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-047
    type: references
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-006
    type: depends_on
---
# FR-084: Admit closed populations and resolve typed lookup and allInstances

## Description

When admitting a population binding and evaluating a `lookup` or
`allInstances` query against it, the model checker SHALL distinguish a
genuine closure failure from a genuine refusal and from resource
incompleteness, and SHALL return a complete, deduplicated result only when
the query's required closure is established; it SHALL NOT substitute an
empty, first-match or otherwise partial result for a closure or refusal
outcome.

This requirement is QSL's own binder contract for quire-specification
[FR-153](ix://agent-ix/quire-specification/FR-153) ("Query explicit closed
model environments"), and it supplies the "explicit membership and closure
authority" and "concrete object membership and closure" that
[FR-047](FR-047-evaluate-finite-object-reference-graphs.md)'s own graph
evaluator names as independently supplied runtime inputs rather than
something FR-047 itself admits.

## Inputs

- A population declaration from the admitted domain package: its member
  types and its declared extent (`closed` or `open`).
- Runtime member records: each names an object identity and the declaration
  key of its most-specific effective type.
- The effective view and conformance relation
  [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md)
  produces.
- `PopulationAdmissionLimitsV1`.
- For lookup: a key reference and a declared absence mode.

## Outputs

- Admission: an admitted `PopulationBinding`, a distinct unknown-closure
  result, a typed refusal, or a typed incomplete result.
- `allInstances<T>`: a complete `Set<Reference<T>>`, a typed incomplete
  result, or a typed refusal.
- `lookup<T>`: a resolved reference (present or absent per the declared
  absence mode), a typed refusal, or a typed incomplete result.

## Behavior

### Admission is total over four distinct outcomes

The model checker SHALL admit a population binding only after establishing,
for every runtime member record, that its most-specific type is covered by
the population declaration's member types and that no two records share
universe and object identity with differing types. When the population
declaration's extent is not `closed`, or object closure otherwise cannot be
established, the model checker SHALL report a distinct unknown-closure
outcome, carrying no binding. This outcome SHALL be a different result type
from a genuine admission refusal (a real defect such as a foreign type, an
abstract-instance member, or a conflicting identity) and from a resource
incomplete outcome (an admission-limit charge denied). None of the three
SHALL be reported as, or converted into, either of the others.

### The model selection's own generalization-graph closure is decided at admission

Distinct from allInstances's own per-`T` subtype-closure check below: whether
the admitted domain package's generalization graph is closed at all — so
that the population's declared member types could even be checked for
coverage — is decided once, as part of admission, from the
`GeneralizationClosure` the caller supplies. When that graph-level closure
does not hold, admission SHALL report the unknown-closure outcome above
(naming the type whose subtypes are unresolved) and SHALL admit no binding
at all. Consequently, no admitted `PopulationBinding` can itself carry an
unresolved generalization-graph closure: by the time a binding exists for
`lookup` or `allInstances` to query, that graph-level closure already
holds. `lookup` and `allInstances` SHALL NOT re-check it, and SHALL NOT
report it as a distinct outcome of a query against an already-admitted
binding — only admission observes and reports it.

### allInstances requires both object and subtype closure

`allInstances<T>` SHALL return a complete result only when the population's
object closure is established and every effective type conforming to `T` is
covered by the population's declared member types (subtype closure for `T`).
If either closure does not hold, the model checker SHALL return a typed
incomplete result naming which closure is missing and SHALL return no
collection, complete or partial. When both closures hold, the returned set
SHALL contain each qualifying member exactly once, keyed by its reference
identity, in canonical reference-key order, independent of the input member
order.

### lookup distinguishes a genuine absence from a genuine refusal or incompleteness

`lookup<T>(p, r)` SHALL resolve `r` against the population binding's members
only. It SHALL report the query's declared absence mode (for example,
`Undefined`) when no member matches the key. Because the "The model
selection's own generalization-graph closure is decided at admission"
clause above excludes an unresolved generalization-graph closure from ever
reaching a `lookup` call — no admitted `PopulationBinding` can carry one —
`lookup` itself SHALL NOT report an unknown-closure outcome at all; it
reports only a genuine absence, a genuine refusal (a foreign-universe key,
a type mismatch or an unresolvable identity) or a resource-exhaustion
incompleteness, and SHALL NOT default any of these to the declared absence
mode.

### Typed results and their refusals

`allInstances<T>(p)`'s admitted result is exactly
`Set<Reference<T>>[0,N]` when `p` declares a maximum `N`, and the unbounded
`Set<Reference<T>>` when `p` declares none — never an untyped or bare
collection. A selected count above a declared maximum SHALL refuse with a
cardinality-out-of-bound cause naming the maximum and the selected count;
an unbounded population SHALL admit every selected count and SHALL NOT
refuse cardinality-out-of-bound. `lookup<T>(p, r)`'s present result is
`Reference<T>` in the `undefined` and `refused` absence modes and
`Option<Reference<T>>` in the `empty` mode, in every case typed to the
queried `T`; a key whose universe differs from `T`'s universe SHALL refuse
with a foreign-universe cause; a receiver that is not a population binding,
or a queried `T` that is not a model object type, SHALL refuse with an
operator-ineligible or type-mismatch cause respectively. This is
quire-specification [FR-153](ix://agent-ix/quire-specification/FR-153)'s
typed-result and refusal contract (`Set<Reference<T>>[0,N]`,
`cardinality_out_of_bound`/`above-maximum`, `foreign_reference`/
`foreign-universe`, `ill_typed`/`operator-ineligible`), bound to this
compiler's own types.

### A reference key's type component is stable regardless of which conforming type queries it

When the same underlying object is selected under two different queried
types `T1` and `T2` that it conforms to (for example, `allInstances<A>` and
`allInstances<C>` where the object's own most-specific effective type is
`C`, a subtype of `A`), the returned `ReferenceKey`'s type component SHALL
be the object's own most-specific effective type in both cases — never `T1`
or `T2` — and the two returned keys for that object SHALL be
byte-identical. This is quire-specification
[FR-153](ix://agent-ix/quire-specification/FR-153)-AC-6's "a subtype object
seen through supertype and subtype queries has one reference key whose type
component is its most-specific type," bound to this compiler's own
`ReferenceKey` type.

### A universe is one connected supertype component

The model checker SHALL compute one object universe per connected component
of the model's object-type supertype graph, with the identity that
quire-specification `model-complete.md` ("Object universe") defines, and
SHALL NOT compute one universe for a whole model or one per root type. A
reference key SHALL carry the universe of its object type's component, and
its object component SHALL be the member's authored object identity, never
a digest of it (ADR-013 O-05, OQ-C and OQ-E rulings).

### Conflicting identity refuses admission outright

If two runtime member records name the same universe and object identity but
different most-specific types, the model checker SHALL refuse admission with
a conflicting-identity cause and SHALL admit no binding from that input.
Duplicate records SHALL collapse into one member only when both their key and
their content digest are equal.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-084-CON-1 | An unknown-closure outcome, a refusal outcome and an incomplete outcome of population admission SHALL be represented as distinct result variants at the type level, not as one variant distinguished only by an inspectable field. | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-084-AC-1 | Given a population declared with an `open` extent, admission reports the distinct unknown-closure outcome, carrying no binding; given the same closed-extent population whose generalization graph is not itself closed (`GeneralizationClosure::Open`), admission likewise reports the distinct unknown-closure outcome, naming the unresolved type, and carries no binding; given the same population declared `closed`, with a closed generalization graph and every member covered, admission succeeds; given a member of a type the population does not declare, admission is refused (not unknown-closure). | Test (TC-226) |
| FR-084-AC-2 | Given a closed population whose object closure holds but whose declared member types do not cover every effective subtype of the queried type, `allInstances<T>` returns a typed incomplete result naming the missing subtype closure and no collection; given both closures established, it returns the complete deduplicated set in canonical reference-key order regardless of input member order. | Test (TC-227) |
| FR-084-AC-3 | Given a `lookup<T>` query whose key matches no member, the result is the declared absence mode, a genuine completed result. | Test (TC-228) |
| FR-084-AC-4 | Given two runtime member records sharing universe and object identity but differing most-specific type, admission refuses conflicting-identity and admits no binding; given two records with equal key and equal content digest, admission collapses them into one member. | Test (TC-229) |
| FR-084-AC-5 | `allInstances<T>(p)`'s result is `Set<Reference<T>>[0,N]` when `p` declares maximum `N`, and the unbounded `Set<Reference<T>>` when `p` declares none, admitting every selected count; a selected count above a declared `N` refuses cardinality-out-of-bound; `lookup<T>(p, r)`'s present result is typed to `T` in every absence mode; a foreign-universe key, a non-binding receiver or a non-object-type `T` each refuse with their own named cause. | Test (TC-240) |
| FR-084-AC-6 | Given an object whose most-specific effective type `C` is a proper subtype of `A`, selecting it through `allInstances<A>` and separately through `allInstances<C>` against the same binding yields the same `ReferenceKey` in both results, whose type component names `C` in both cases, never `A`. | Test (TC-242) |
| FR-084-AC-7 | Given a model whose object types form three disconnected supertype components, `A` with subtype `B`, `C` and `D` with common subtype `E`, and `X` alone, the reference keys of a `B`, an `E` and an `X` member, each admitted in its own population, carry three different universes: the `quire.model.object-universe/v1` digests over the model selection with root types `[A]`, `[C, D]` and `[X]`. Each key's object component equals the member's authored object identity bytes. | Test (TC-410) |

## Dependencies

- **Upstream:** [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md)
  supplies the effective declarations this requirement's population binding
  is keyed against; quire-specification FR-153 owns the normative closed-
  environment query rule and AD-006 owns the decision to keep populations
  and closed dispatch sets in the model view. FR-153-AC-9 (the unbounded
  `p` case of FR-084-AC-5) landed in quire-specification PR #75. Since
  QSL-140, `qsl-semantics/src/model/population.rs`'s `all_instances` returns
  the unbounded `Set<Reference<T>>` (a result with no bound) when `p`
  declares no maximum, admitting every selected count (TC-240 step 3,
  `an_unbounded_binding_selects_every_member_with_no_bound`). The other
  FR-084-AC-5 clauses keep their own status under TC-240.
- ADR-013 §8 OQ-C and OQ-E, and quire-specification `model-complete.md`
  and FR-204, fix the universe and object identity (FR-084-AC-7). Remaining
  work: QSL-131.
- **Downstream:** [FR-047](FR-047-evaluate-finite-object-reference-graphs.md)'s
  graph evaluator receives concrete object membership and closure as an
  independently supplied runtime input; this requirement is that input's own
  admission and query contract, not a restatement of FR-047's reachability
  or dereference behavior, which stays out of this requirement's scope.
