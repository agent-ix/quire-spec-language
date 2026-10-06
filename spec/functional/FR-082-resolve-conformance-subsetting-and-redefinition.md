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
  - target: ix://agent-ix/quire-spec-language/ADR-014
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
- `ModelNormalizationLimitsV1`, and the expression checker's
  `TypeEnvironmentLimits`.

## Outputs

For each checked redefinition or subsetting relation: an admitted conformance
result, or a `Refused` outcome carrying every failing axis (member kind, axis
name and typed cause), or a typed incomplete result naming the exhausted
charge point. For the expression checker's type-environment admission, a
reached ceiling is a stage failure, `StageFailure::Limit(LimitExceeded)`.

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

The field-domain obligation is decided over exact integers. The redefined
field's declared scalar bounds and a postcondition's comparison literal are
exact integers in `i128::MIN..=i128::MAX`, the ceiling FR-091 fixes for
every integer bound and literal, and the checker SHALL NOT narrow either to
a smaller width. A scalar domain whose bounds exceed `i64`, such as
`[0, 18446744073709551615]`, and a literal above `i64::MAX` take part in
the obligation like any other.

### Presence and multiplicity are separate narrowing axes

The model checker SHALL read a field's optionality from its declared
`presence` ([FR-056](FR-056-admit-domain-package-model-declarations.md);
QSpec `model-complete.md` Presence row) and never from its multiplicity.
A field redefinition whose redefined field is `optional` and whose
redefining field is `required` narrows presence, and SHALL engage QSpec
FR-151's `field-presence` obligation whatever the two multiplicities are.
A redefinition whose two fields declare the same presence engages no
obligation through presence. Its multiplicity axis engages the obligation
FR-151 gives a narrowed multiplicity, judged from the two declared bounds
alone. An upper bound of `1` does not make a field optional for this
check.

### Ancestor and conformance walks are bounded

The model checker SHALL charge every `supertypes` edge an ancestor-chain or
conformance walk follows against the caller's `ancestor_steps` work limit,
carried in `ModelNormalizationLimitsV1` (and, for population admission, in
`PopulationAdmissionLimitsV1`), before following the edge. `ancestor_steps`
counts the edges one walk follows, so under multiple supertypes it counts
the walk's closure, never a chain depth; each walk starts a fresh count.
Each walk runs over an explicit stack, not native recursion. The limit has a
published default of 16777216 edges (NFR-012) and no ceiling: the caller
raises or lowers it, and the value is used as given. If a charge is denied,
the model checker SHALL stop with the `Incomplete` outcome naming
`ancestor_steps`, its configured value and the count the denied charge would
have reached (ADR-013 O-21, ADR-030 D-1), and SHALL NOT report a conformance
or non-conformance verdict for that walk.

The walks above charge B-2 limits, carried by the accounting-contract limits
types `ModelNormalizationLimitsV1` and `PopulationAdmissionLimitsV1`, so a
denied charge settles `Incomplete` with `resource_exhausted` wherever they run
([ADR-014](../decisions/ADR-014-temporal-trace-and-boundedness-architecture.md)
§1, NFR-012). The expression checker's type-environment admission below reads
its own check-stage limits type, `TypeEnvironmentLimits`, whose limits are
stage limits (ADR-014 B-3, FR-096): `ancestor_steps`, setting
`environment.ancestor_steps`, and `work_units`, setting `environment.work_units`, each
with a published default of 16777216 and no ceiling (FR-255). When a type's ancestor closure would
follow more `supertypes` edges than its `ancestor_steps` limit, the
expression checker SHALL return `StageFailure::Limit(LimitExceeded)` with
limit kind edge count, catalog `stage_limit_exceeded`/`edge-count-exceeded`. When type-environment admission
would spend more than its `work_units` budget, the expression checker SHALL
return `StageFailure::Limit(LimitExceeded)` with limit kind work budget,
catalog `stage_limit_exceeded`/`work-budget-exceeded`. Each `LimitExceeded` carries the
configured limit, the actual counter (the limit plus one for
`ancestor_steps`; for `work_units`, the cumulative total the refused charge
would have reached), and no `Locus`: the
object types come from an admitted domain package, not a source unit, which
is FR-096's third no-region case.

### The expression checker decides the same relation over the same edges

The expression checker's type environment SHALL admit a package's object
types with the same declared `supertypes` edges the model walks, and SHALL
give the same verdict the model gives at evaluation:

- A supertype naming no admitted object type, and a `supertypes` cycle, SHALL
  refuse the environment.
- Admission SHALL use the same `ancestor_steps` limit the population
  binding walks under, counted the same way. An object type whose ancestor
  closure would follow more `supertypes` edges than the limit SHALL refuse
  the environment with a stage limit (`LimitExceeded`, limit kind edge
  count) naming the limit. Check time is the stricter side: it charges each
  type's full closure, while an evaluation walk from that type follows at
  most those edges and may stop early, so every conformance question the
  checker answers, evaluation completes with the same answer.
- Admission SHALL charge the ancestor closure and the attribute flattening
  below to a `work_units` budget: one unit for each ancestor or attribute
  copied into a type's set and each field a lineage names. When the budget
  runs out, the environment SHALL refuse with a stage limit (`LimitExceeded`,
  limit kind work budget) naming the budget. Admission work SHALL grow with the
  flattened slot count, not faster.
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
  one of them is, and admitting only values that field's type admits. A
  reference field SHALL be redefined only with the identical reference
  type, since evaluation matches a reference's object type exactly.
  Otherwise the environment SHALL refuse as ill-typed.
- `deref(r).f` SHALL resolve `f` in `r`'s static type's flattened set, and
  SHALL read the one slot of the referenced object's own type that stands
  for that field.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-082-AC-1 | Given a redefining operation whose parameter type, result multiplicity and effect frame each independently violate their axis, the checker's `Refused` outcome names all three failing axes with their own typed cause, not only the first one checked. | Test (TC-218) |
| FR-082-AC-2 | Given a member redefinition naming a target absent from any supertype the owner conforms to, and separately a declared `supertypes` cycle, each is refused with its own named cause (redefinition-target, specialization-cycle) and no conformance edge is derived across the cycle. | Test (TC-219) |
| FR-082-AC-3 | Given a conformance walk that follows `n` `supertypes` edges, the checker with `ancestor_steps` at `n − 1` stops `Incomplete` naming `ancestor_steps`, bound `n − 1` and count `n`, and reports neither conformance nor non-conformance for that walk; at `n` it gives the conformance verdict. A 10,000-long chain's conformance walk completes on a thread with a 512 KiB stack at the default `ancestor_steps`, and no outcome names a depth. | Test (TC-220) |
| FR-082-AC-4 | Given a narrowing field redefinition with no established postcondition fact proving the narrowing, the checker refuses unproved-refinement; given the same redefinition with the obligation established, the checker admits it. | Test (TC-221) |
| FR-082-AC-5 | Given a redefining operation whose parameter count differs from the redefined operation's, the checker's `Refused` outcome names exactly the arity failure with a type-mismatch cause and checks no per-parameter type or multiplicity axis for that pair; the result-type, result-multiplicity and effect axes are still independently checked and reported when they also fail. | Test (TC-239) |
| FR-082-AC-6 | Given one set of object types and `supertypes` edges, the expression checker's type environment is never less strict than the model's evaluation-time walk: a supertype naming no admitted type and a `supertypes` cycle refuse at both; a chain whose walk fits the shared `ancestor_steps` limit admits at both with the same answer to every conformance question; and a chain one edge past it refuses at check time with a stage limit (`LimitExceeded`, edge count) naming the limit, as the model's walk over it stops `Incomplete` at evaluation naming `ancestor_steps`. Anything check time admits, evaluation completes. | Test (TC-219, TC-220) |
| FR-082-AC-7 | Given object types whose ancestor closure and flattened attribute sets would cost more than the admission `work_units` budget, the expression checker's type environment refuses with a stage limit (`LimitExceeded`, work budget) naming the budget; a linear chain admits within four work units per flattened slot. | Test (TC-220) |
| FR-082-AC-8 | Given a field `f` declared `optional` with multiplicity `[1, 1]`, redefined by a field declared `required` with multiplicity `[1, 1]` and written by an exposed operation whose postcondition establishes no presence fact, the checker refuses `unproved-refinement` with obligation `field-presence`; with `present(self.f)` established it admits the redefinition. Given `f` and its redefinition both declared `required` with multiplicity `[0, 1]`, or both `optional` with multiplicity `[1, 1]`, written by the same operation with no established fact, the checker admits the redefinition and engages no obligation. | Test (TC-896) |
| FR-082-AC-9 | Given a domain package whose scalar type `Wide` is bound to `[0, 18446744073709551615]` (its bounds written as decimal strings, FR-056), a field `f: Wide` redefined by a field of scalar type `Narrow` bound to `[0, 9223372036854775808]`, and an exposed operation writing `f` whose stated postcondition is `self.f <= 9223372036854775808`, the checker admits the redefinition; with the postcondition `self.f <= 9223372036854775809` it refuses `unproved-refinement` with obligation `field-domain`. | Test (TC-911) |

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

## References

- Linear QSL-254 (AC-8: presence and multiplicity are separate narrowing axes).
- Linear QSL-642 (AC-9: the field-domain obligation over exact integers up to i128).
