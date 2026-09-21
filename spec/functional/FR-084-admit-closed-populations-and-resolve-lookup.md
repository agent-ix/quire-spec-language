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

### lookup distinguishes absence, refusal and incompleteness

`lookup<T>(p, r)` SHALL resolve `r` against the population binding's members
only. It SHALL report the query's declared absence mode (for example,
`Undefined`) when no member matches the key, and SHALL NOT default an
unresolved closure to that same absence outcome: an unestablished closure is
always the distinct incomplete or unknown-closure outcome, never a resolved
absence.

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
| FR-084-AC-1 | Given a population declared with an `open` extent, admission reports the distinct unknown-closure outcome, carrying no binding; given the same population declared `closed` with every member covered, admission succeeds; given a member of a type the population does not declare, admission is refused (not unknown-closure). | Test (TC-226) |
| FR-084-AC-2 | Given a closed population whose object closure holds but whose declared member types do not cover every effective subtype of the queried type, `allInstances<T>` returns a typed incomplete result naming the missing subtype closure and no collection; given both closures established, it returns the complete deduplicated set in canonical reference-key order regardless of input member order. | Test (TC-227) |
| FR-084-AC-3 | Given a `lookup<T>` query whose key matches no member, the result is the declared absence mode; given the same binding with subtype closure not established for `T`, the result is a typed incomplete or unknown-closure outcome, never the absence mode standing in for it. | Test (TC-228) |
| FR-084-AC-4 | Given two runtime member records sharing universe and object identity but differing most-specific type, admission refuses conflicting-identity and admits no binding; given two records with equal key and equal content digest, admission collapses them into one member. | Test (TC-229) |

## Dependencies

- **Upstream:** [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md)
  supplies the effective declarations this requirement's population binding
  is keyed against; quire-specification FR-153 owns the normative closed-
  environment query rule and AD-006 owns the decision to keep populations
  and closed dispatch sets in the model view.
- **Downstream:** [FR-047](FR-047-evaluate-finite-object-reference-graphs.md)'s
  graph evaluator receives concrete object membership and closure as an
  independently supplied runtime input; this requirement is that input's own
  admission and query contract, not a restatement of FR-047's reachability
  or dereference behavior, which stays out of this requirement's scope.
