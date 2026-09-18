---
id: FR-151
title: "Resolve conformance, redefinition and closed dispatch"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/AD-006
    type: references
  - target: ix://agent-ix/quire-specification/FR-150
    type: references
---

# FR-151: Resolve conformance, redefinition and closed dispatch

## Description

When checking specialization or a dispatched call, the model checker SHALL
apply the selected conformance, subsetting, redefinition and method-precedence
rules over a closed declaration set.

## Inputs

The effective view of FR-150, its generalization, subsetting and redefinition
records, producer interface `1.3.0` operation signatures and typed
multiplicities, authored contract clauses and operation bodies, and the selected
`quire.model.complete/v1` definition.

## Outputs

Validated effective declarations and a linked dispatch table, or a located
cycle, incompatibility, unproved refinement, ambiguity or incomplete-closure
refusal.

## Behavior

Type `S` conforms to type `T` exactly when `S` and `T` are the same effective
type or a chain of supplied generalization records leads from `S` to `T`. `S`
is strictly narrower than `T` exactly when it is a proper descendant of `T` in
that closed graph; equal shapes of distinct declarations never make one
narrower. Redefinition retains correspondence and cannot weaken inherited
contracts. Dispatch selects, for each closed subtype, the unique undominated
method; registration, source or record order never resolves ambiguity, and no
linearization is applied.

## Conformance rules

Rule `quire.model.conformance.variance/v1` checks a redefining member against the
member it redefines on these independent axes, in this order, charging one
`conformance.axis` before each axis runs and reporting every failing axis in
this order; checking is exhaustive under the limits (FR-150), while a runtime
check stops at its first failure:

| Member kind | Axis | Rule | Refusal |
| --- | --- | --- | --- |
| field | value type | the redefining type conforms to the redefined type; for value types, the FR-149 equality-conversion table admits the redefining type to the redefined type | `ill_typed`/`variance-result` |
| field | multiplicity | rule below | `ill_typed`/`multiplicity-narrowing` |
| field | refinement | obligation below | `undefined_expression`/`unproved-refinement` |
| operation | arity | equal parameter count; when unequal, no parameter axis is checked | `ill_typed`/`type-mismatch` |
| operation | parameter type `i` | the redefined parameter type conforms to the redefining one (contravariant) | `ill_typed`/`variance-parameter` |
| operation | parameter multiplicity `i` | the redefined multiplicity conforms to the redefining one | `ill_typed`/`multiplicity-narrowing` |
| operation | result type | the redefining result type conforms to the redefined one; both absent passes; one absent fails | `ill_typed`/`variance-result` |
| operation | result multiplicity | the redefining multiplicity conforms to the redefined one | `ill_typed`/`multiplicity-narrowing` |
| operation | effect | frame inclusion below | `ill_typed`/`effect-escape` |
| operation | precondition | by construction below | none |
| operation | postcondition | by construction below | none |

The receiver is parameter 0 and is covariant by construction: the redefining
receiver type is the redefining owner. Rule
`quire.model.conformance.multiplicity/v1`: typed multiplicity `[l2, u2]`
conforms to `[l1, u1]` exactly when `l1 <= l2` and `u2 <= u1`, where
`unbounded` is greater than every finite value; `ordered` and `unique` must be
equal, otherwise `ill_typed`/`multiplicity-narrowing`. Subsetting is checked by
the same rule: the subsetting feature's type conforms to the subsetted
feature's type (`ill_typed`/`subsetting-type`) and its multiplicity conforms
(`ill_typed`/`multiplicity-narrowing`).

Rule `quire.model.conformance.effect/v1`: an effect set is the operation's
producer-supplied frame `{fieldWrites, creates, deletes}`. The redefining effect
is included in the redefined effect exactly when each written field is a
written field of the redefined effect or reaches one through redefinition
records, and each created or deleted type conforms to some created or deleted
type, respectively, of the redefined effect. A create or delete of a subtype is
covered by a grant for its supertype. Any other entry is `effect-escape`, one
refusal per entry. The same inclusion governs FR-013 frames under inheritance.

Rule `quire.model.conformance.refinement/v1` makes contracts Liskov-correct by
construction, so no implication between authored expressions is proved. The
effective precondition of an operation member is the disjunction of its own
precondition clauses with the effective preconditions of the members it
redefines; the effective postcondition is the conjunction of its own
postcondition clauses with theirs; and the effective invariant of a type is the
conjunction of its own invariant clauses with those of every ancestor. An
absent precondition is `true`, and an absent postcondition or invariant is
`true`. A subtype therefore cannot weaken an invariant or postcondition or
strengthen a precondition, and no refusal exists for those axes.

The one obligation not discharged by construction arises when a field
redefinition narrows the value type or multiplicity and an exposed operation of
the owning effective type (inherited or own) writes that field. That operation's
effective postcondition must establish the narrowed domain through FR-146
closed guard facts: an interval fact for a narrowed numeric domain, a presence
fact for a raised lower bound. Inside the postcondition, `self` is a stable-path
root (FR-146), and for this obligation a projection `self.f` onto the narrowed
field or the field it redefines has the declared facts of the redefined parent
member being refined (its declared type interval and multiplicity), never the
narrowed type; a narrowed declaration therefore never discharges its own
obligation. A missing fact is refused
`undefined_expression`/`unproved-refinement` with obligation `field-domain` or
`field-presence`; a narrowing no FR-146 fact form can express (a narrowed
object type or an upper bound on a collection) is refused with obligation
`no-proof-form`. No SMT or external prover is selected.

At runtime, subsetting is checked at binding admission, charged as
`binding.subset-value` under `PopulationAdmissionLimitsV1` and without an
evaluation charge, for every object of a closed population: the values of the subsetting
feature are a subset of the values of the subsetted feature. A violation is
`invalid_runtime_input`/`subsetting-violation`; a population whose object
closure is not established is `incomplete_population`/`incomplete-scope`.

## Dispatch rules

Rule `quire.model.dispatch.single/v1`. A dispatched call is
`receiver.member-name(args)` whose receiver is `self`, a `deref(...)` result or a
`Reference<T>` value. `member-name` resolves statically to exactly one exposed
effective operation `o` of the receiver's static type `T`; its arguments are
type-checked statically against `o`'s signature with reference upcasts only.
Only query operations, whose result is present and whose effect set is empty,
may be called, and only inside invariant, precondition and postcondition
blocks; a call in a function or operation body is `ill_typed`/`operator-ineligible`.
A body is supplied by an `operation-body` declaration.

At link time, the family of `o` is `o`'s original declaration together with
every redefining operation reaching a subtype of `T`; its members that have a
body are the candidates. For every effective type `S` conforming to `T` in the
closed generalization graph, the applicable candidates are those whose
effective owner `S` conforms to. Candidate `p` dominates candidate `q` exactly
when `p`'s owner is strictly narrower than `q`'s owner. The unique undominated
applicable candidate is linked for `S`. No applicable candidate is
`ambiguous_dispatch`/`no-applicable`; several undominated candidates are
`ambiguous_dispatch`/`multiple-undominated` with the candidate identities and
dominance pairs. An inherited, unredefined operation reached along several
paths is one candidate by original identity. Each subtype is charged as
`dispatch.subtype`, each (subtype, candidate) test as `dispatch.candidate` and
each dominance-pair enumeration as `dispatch.dominance`. Linking is exhaustive
under the limits: it reports every no-applicable and every ambiguous subtype of
every called operation, in charge order, and then produces no dispatch table.
A ModelSelection whose `generalizationClosure` is not `closed` has no dispatch
table: linking returns the incomplete outcome
`incomplete_population`/`unclosed-method-set`, not a refusal, before any
dispatch charge. Every ambiguity is therefore a link-time refusal.

Dispatch edges are call-graph edges. For each dispatched call, the FR-146
call graph has an edge from the declaration containing the call (a function,
an invariant, precondition or postcondition clause, or an operation body) to
every candidate's `operation-body` and to every precondition clause of every
candidate's effective precondition, which the call evaluates. Operation bodies
and contract clauses have no `decreases` form, so a cycle containing a dispatch
edge is refused at link time as `refused { code: invalid_package, cause:
definition-cycle }`, one refusal per strongly connected component, listing its
edges ascending by caller and then callee source declaration order; for
example, a precondition of `A.size` that calls `self.size()` is a self-loop.

Preconditions are not part of applicability. At runtime the receiver and the
arguments are evaluated, `dispatch.select` is charged, the method linked for
the receiver's most-specific type is selected, the selected member's effective
precondition is evaluated as an ordinary obligation whose false, undefined or
incomplete result propagates (false is `undefined` with the catalogued reason
`precondition-false`), and `function.call` precedes the body. Inside a
postcondition, the receiver's most-specific type is read from the receiver
reference's own observation; an object deleted in the post state keeps its pre
type.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-151-AC-1 | A compatible specialization/redefinition yields one effective member with complete provenance. | Test (TC-196) |
| FR-151-AC-2 | An inheritance cycle or incompatible variance refuses with its named cause and the contributing records; a weakened invariant is not expressible because effective invariants are conjunctions. | Test (TC-196) |
| FR-151-AC-3 | Zero or multiple undominated methods for any closed subtype refuse at link time with `ambiguous_dispatch`, every such subtype is reported, an open generalization closure is incomplete with `unclosed-method-set`, and a call-graph cycle through a dispatch edge refuses `definition-cycle`. | Test (TC-196) |
| FR-151-AC-4 | Parameter, result, multiplicity and effect variance are checked and reported independently, and pre/postconditions are combined by construction. | Test (TC-196) |
| FR-151-AC-5 | Registration and source order cannot change the selected method or resolve an ambiguity. | Test (TC-196) |
| FR-151-AC-6 | A narrowing field redefinition written by an exposed operation without an FR-146 establishing fact refuses `unproved-refinement` with its obligation. | Test (TC-196) |
| FR-151-AC-7 | A false selected precondition yields `undefined` for the call and never selects a less specific method. | Test (TC-196) |
| FR-151-AC-8 | Dispatch charges at their exact bound complete and the one-less run is incomplete at the named charge point. | Test (TC-196) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
