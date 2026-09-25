---
id: FR-082
title: "Resolve conformance, subsetting and redefinition over the closed declaration set"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-006
    type: depends_on
---
# FR-082: Resolve conformance, subsetting and redefinition over the closed declaration set

## Description

When checking a specialization, a subsetting feature or a redefining member
against the declaration it narrows, the model checker SHALL decide conformance
over the closed ancestor graph the admitted domain package declares. The model
checker SHALL check every independent variance axis the redefined member kind
defines. The model checker SHALL report every failing axis rather than
stopping at the first.

This requirement is QSL's own binding of quire-specification
[FR-151](ix://agent-ix/quire-specification/FR-151)'s conformance and
redefinition rules into the compiler's behavior. It does not restate FR-151's
axis table; it states the checker's exhaustiveness, boundedness and refusal
contract for applying it.

## Inputs

- The effective view [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md)
  produces, and each effective declaration's declared `supertypes`, `subsets`
  and `redefines` edges.
- Field value types and typed multiplicities; operation parameter types,
  multiplicities, result types and declared effect frames (`modifies`,
  `creates`, `deletes`).
- Established postcondition facts available for a narrowing field
  redefinition's refinement obligation.
- `ModelNormalizationLimitsV1`.

## Outputs

For each checked redefinition or subsetting relation: an admitted conformance
result, or a `Refused` outcome carrying every failing axis (member kind, axis
name and typed cause), or a typed incomplete result naming the exhausted
charge point.

## Behavior

### Conformance is closed-graph reachability, not a display match

The model checker SHALL decide that a type `S` conforms to a type `T` exactly
when `S` and `T` are the same effective type or a chain of declared
`supertypes` edges reaches `T` from `S`, walked over the admitted domain
package's own declared edges only. It SHALL NOT treat two declarations that
share a display name, and are not the same effective declaration, as
conforming.

### Every independent axis is checked, and every failure is reported

For a redefining field, the model checker SHALL check the value-type axis,
the multiplicity axis and the refinement-obligation axis independently. For a
redefining operation, it SHALL check arity, each parameter's type and
multiplicity, the result type and multiplicity, and effect-frame inclusion
independently. It SHALL charge one unit of conformance work before each axis
runs, and it SHALL collect every failing axis of the member under check into
one `Refused` outcome rather than returning after the first failing axis.

### Unequal arity carves out the parameter axes

Arity is checked first. When a redefining operation's parameter count does
not equal the redefined operation's, the model checker SHALL refuse with a
type-mismatch cause naming the arity difference and SHALL NOT check any
per-parameter type or multiplicity axis for that pair — there is no
parameter-by-position correspondence to check across an unequal parameter
list. The result-type, result-multiplicity and effect axes remain
independent of arity and are still checked and reported on their own. This
is quire-specification [FR-151](ix://agent-ix/quire-specification/FR-151)'s
arity row ("equal parameter count; when unequal, no parameter axis is
checked"), which this exhaustiveness requirement's own "every independent
axis is checked" rule above does not override.

### Subsetting and redefinition targets must exist and must not cycle

If a subsetting or redefining member names a feature that is not declared on
a supertype the owner conforms to, the model checker SHALL refuse it with a
redefinition-target cause naming the missing target. If the declared
`supertypes` edges of the admitted package contain a cycle, the model checker
SHALL refuse specialization for every declaration on the cycle with a
specialization-cycle cause, and SHALL derive no conformance edge across the
cycle.

### The refinement obligation admits no partial pass

A narrowing field redefinition SHALL be admitted only when an established
postcondition fact proves the narrowing; absent that fact, the model checker
SHALL refuse it with an unproved-refinement cause naming the missing
obligation kind. The model checker SHALL NOT admit a narrowing redefinition
on the strength of its declared shape alone.

### Ancestor and conformance walks are bounded

The model checker SHALL bound every ancestor-chain and conformance walk it
performs by the caller-supplied `ancestor_steps` ceiling carried in
`ModelNormalizationLimitsV1` (and, for population admission, in
`PopulationAdmissionLimitsV1`). The ceiling counts the types one walk
expands, the starting type included, so a target `n` generalization steps
above the starting type is reached at a ceiling of `n`. The checker SHALL use
the caller's ceiling as given and SHALL NOT substitute a fixed implementation
ceiling for it. The ceiling is read, never charged. If a walk would expand
one type more than the ceiling, the model checker SHALL refuse with a
resource-exhaustion cause (`ancestor-steps`) naming the ceiling and SHALL NOT
report a conformance or non-conformance verdict for that walk. Reaching this
ceiling is a `Refused` outcome, never the `Incomplete` outcome
`ModelNormalizationLimitsV1`'s own axis-charge and record-charge points
produce when a charge is denied; the two are distinct result variants over
distinct counters, consistent with ADR-013 O-21 ("an exhausted meter yields
`Incomplete` with its charge point and limit; a stage limit refuses with
`LimitExceeded`").

### The expression checker decides the same relation over the same edges

The expression checker's type environment SHALL admit a package's object
types with the same declared `supertypes` edges the model walks, and SHALL
give the same verdict the model gives at evaluation:

- A supertype naming no admitted object type, and a `supertypes` cycle, SHALL
  refuse the environment.
- Admission SHALL use the same `ancestor_steps` ceiling the population
  binding walks under, counted the same way. An object type whose walk
  (the type itself plus every ancestor) would expand more types than the
  ceiling SHALL refuse the environment with the resource-exhaustion cause
  (`ancestor-steps`) naming the ceiling. Every conformance question the
  checker then answers, evaluation answers the same way; none is one
  evaluation would refuse.
- Each object type's attributes SHALL be flattened once, at admission: its
  own fields and every ancestor's. A field another field of the set
  redefines SHALL be hidden, and its redefiner SHALL take its one storage
  slot. When several redefinitions of one field reach a type, only the one
  whose owner is more derived than every other SHALL stay exposed; if none
  is, the environment SHALL refuse with a redefinition-conflict cause. A
  field with an inherited field's name that does not redefine it SHALL
  refuse as a duplicate member, and a `redefines` naming no field of a
  proper ancestor SHALL refuse with a redefinition-target cause. The
  exposed field SHALL narrow every field it stands for: required wherever
  one of them is, and admitting only values that field's type admits.
  Otherwise the environment SHALL refuse as ill-typed.
- `deref(r).f` SHALL resolve `f` in `r`'s static type's flattened set, and
  SHALL read the one slot of the referenced object's own type that stands
  for that field.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-082-AC-1 | Given a redefining operation whose parameter type, result multiplicity and effect frame each independently violate their axis, the checker's `Refused` outcome names all three failing axes with their own typed cause, not only the first one checked. | Test (TC-218) |
| FR-082-AC-2 | Given a member redefinition naming a target absent from any supertype the owner conforms to, and separately a declared `supertypes` cycle, each is refused with its own named cause (redefinition-target, specialization-cycle) and no conformance edge is derived across the cycle. | Test (TC-219) |
| FR-082-AC-3 | Given a conformance ancestor chain longer than the bound, the checker refuses with a resource-exhaustion cause naming the bound and reports neither conformance nor non-conformance for that walk; a chain at exactly the bound is admitted. | Test (TC-220) |
| FR-082-AC-4 | Given a narrowing field redefinition with no established postcondition fact proving the narrowing, the checker refuses unproved-refinement; given the same redefinition with the obligation established, the checker admits it. | Test (TC-221) |
| FR-082-AC-5 | Given a redefining operation whose parameter count differs from the redefined operation's, the checker's `Refused` outcome names exactly the arity failure with a type-mismatch cause and checks no per-parameter type or multiplicity axis for that pair; the result-type, result-multiplicity and effect axes are still independently checked and reported when they also fail. | Test (TC-239) |
| FR-082-AC-6 | Given one set of object types and `supertypes` edges, the expression checker's type environment and the model's evaluation-time walk give the same verdict for a supertype naming no admitted type (both refuse), a `supertypes` cycle (both refuse), a chain whose walk fits the shared `ancestor_steps` ceiling (both admit and agree on every conformance answer) and a chain one type past it (both refuse `ancestor-steps` naming the ceiling). | Test (TC-219, TC-220) |

## Dependencies

- **Upstream:** [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md)
  supplies the effective declarations this requirement checks;
  quire-specification FR-151 owns the normative variance-axis table and
  AD-006 owns the model-view decision to keep conformance edges, subsetting
  and redefinition in the checked model view.
- **Downstream:** [FR-083](FR-083-resolve-unique-most-specific-dispatch.md)
  uses this requirement's conformance relation to decide dispatch
  applicability and dominance.
- This requirement's behavior touches the same compiler surface as
  [FR-068](FR-068-split-expression-checking-into-check-stage.md), which
  declares a bounded, temporary `model` → `check` import edge confined to
  exactly the names FR-068-AC-9 and FR-068-CON-5 list. This requirement does
  not alter, widen or contradict that edge; it specifies conformance and
  redefinition behavior only.
