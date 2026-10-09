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
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
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

Conformance is the stage that turns FR-081's effective view into FR-151's
validated effective declarations. Spine intake (FR-056 I1) runs it over each
selected domain package once normalization completes, on the meter
normalization charged, and decides every axis the domain package alone
determines. A refinement obligation that only authored postcondition facts
can discharge is decided at S3 by `check`, from the unit's checked `post`
state clauses (FR-104). A unit yields a checked package only when every
selected domain package passes every axis.

## Inputs

- The effective view [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md)
  produces, and each effective declaration's declared `supertypes`, `subsets`
  and `redefines` edges.
- Field value types and typed multiplicities; operation parameter types,
  multiplicities, result types and declared effect frames (`modifies`,
  `creates`, `deletes`).
- The checked `post` state clauses ([FR-104](FR-104-check-state-clauses.md))
  of the `1-draft` unit that selects the domain package. FR-103 keeps no
  domain-package `pre` or `post` text, so these are the contract clauses a
  refinement obligation reads.
- `ModelNormalizationLimitsV1`, as the one meter normalization and
  conformance charge together, and the expression checker's
  `TypeEnvironmentLimits`.

## Outputs

For each checked redefinition or subsetting relation: an admitted conformance
result, or a `Refused` outcome carrying every failing axis (member kind, axis
name and typed cause), or a typed incomplete result naming the exhausted
charge point. For the expression checker's type-environment admission, a
reached ceiling is a stage failure, `StageFailure::Limit(LimitExceeded)`.

At the spine: an I1 refusal of the `model` declaration carrying every failing
axis I1 decides, an I1 stage limit, or the admitted model with its pending
refinement obligations; then, at S3, a `check` refusal carrying every
unproved obligation, or the checked package.

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
independently. It SHALL charge one `conformance.axis` before each axis runs,
and it SHALL collect every failing axis of the member under check into one
`Refused` outcome rather than returning after the first failing axis. Each
charge adds the work units QSpec `value-accounting.md`'s `conformance.axis`
row gives its axis: `f(x)` for a type-conformance axis, read from the
completed normalization's type derivation facts of `x`; `max(1, t)` for the
effect axis, comparing every frame entry with every redefined entry or grant;
and one for every other axis.

### Conformance is one stage between normalization and assembly

Spine intake SHALL run the conformance stage over a selected domain package
after normalization completes and before the FR-091 assembler runs, charging
the same `ModelNormalizationLimitsV1` meter normalization charged. The stage
checks redefining members, then subsetting fields, each ascending by
declaration key, and each member's axes in FR-151's order, so a field's
refinement axis is charged after its multiplicity axis. Normalization's own
result is the FR-081 effective view: its phase 4 refuses an unresolvable or
conflicting redefinition target, and a package normalization refuses reaches
no `conformance.axis` charge.

I1 SHALL decide every axis except a refinement obligation of kind
`field-presence` or `field-domain`, which it records as pending with its
charge position. If any axis I1 decides fails, then I1 SHALL refuse the
`model` declaration with every such failure, in charge order, and the unit
is not assembled. If a charge is denied, then I1 SHALL stop with the stage
limit naming the exhausted setting and SHALL report no conformance refusal.
I1 checks the unit's `model` declarations in source order and stops at the
first that does not admit, as FR-056 states.

Once the unit's state clauses are checked (FR-104), `check` SHALL decide
every pending obligation of every selected domain package. If any is
unproved, then `check` SHALL refuse the unit with every unproved obligation,
in charge order, each located at its `model` declaration, and SHALL produce
no checked graph.

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

A narrowing field redefinition engages one obligation for each operation
exposed in the owning type's effective view that writes the field: an
operation whose `modifies` names the redefining field or the redefined field,
or reaches the redefined field through `redefines` links. A narrowing that no
exposed operation writes engages no obligation. The model checker SHALL NOT
admit a narrowing redefinition on the strength of its declared shape alone.

Each obligation SHALL be discharged only by the facts its operation's
effective postcondition establishes. That postcondition is the conjunction of
the checked `post` clauses whose resolved operation (FR-103) is that operation
or a member it redefines, directly or through a chain. Its facts are the ones
FR-146's guard-fact derivation proves on the conjunction's true outcome for
the stable path `self.f`, where `f` is the redefining field or the field it
redefines, and `self.f` carries the redefined parent member's declared
interval and presence. An undischarged obligation SHALL be refused
`undefined_expression`/`unproved-refinement` naming the redefining field, the
writing operation and the obligation, `field-presence` or `field-domain`. A
narrowing no FR-146 fact form expresses, an object-typed narrowing or a
narrowed collection upper bound, SHALL be refused with obligation
`no-proof-form` at I1.

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
| FR-082-AC-10 | Given TC-196's fixture H as a Semantic IR domain package selected by a `1-draft` unit, with `B/op` declaring `redefines: A/op` and R03's signature, the spine's `select` refuses the `model` declaration with exactly R03's five refusals in R03's order, after eight `conformance.axis` charges; with R04's signature it refuses with exactly the arity `type-mismatch` refusal, after six charges; with R02's and with R09's package, `select` admits and `check` checks the unit. `normalize` over the R03 package completes with an effective view. | Test |
| FR-082-AC-11 | The `select` runs of FR-082-AC-10 charge TC-196's per-axis work units on the meter normalization charged: ten for R02, eight for R03, eight for R04 and ten for R09. With `model.work_units` set to R02's normalization spend plus ten, `select` admits R02; one lower, it stops at `conformance.axis` with a stage limit naming `model.work_units` and reports no conformance refusal. | Test |
| FR-082-AC-12 | Through `select`, TC-196 R05's five shapes give `multiplicity-narrowing`, admitted, admitted, `multiplicity-narrowing` and `multiplicity-narrowing`, and R06's first three shapes give `multiplicity-narrowing`, `subsetting-type` and admitted, each refusal naming both declarations. R07's first shape refuses at `select` with `invalid_model_binding`/`redefinition-target` for `B/z` and no `conformance.axis` charge. | Test |
| FR-082-AC-13 | Given TC-196 R08's package selected by a unit that authors R08's `post` clauses as state clauses on `M::B::set`: shape (c) refuses at `select` with obligation `no-proof-form`; (a) refuses at `check` with `undefined_expression`/`unproved-refinement` naming `B/xb`, operation `A/set` and `field-presence`; (d) refuses at `check` naming `B/cs`, operation `A/set` and `field-domain`; (f) refuses at `check` naming `B/cs`, operation `B/set` and `field-domain`, because `self.cs <= 6` read at `A/c`'s `Count` proves `[0, 6]`; for (b) and (e), `check` checks the unit. | Test |
| FR-082-AC-14 | Given R08 shape (e) plus operation `A/reset` with no params, no returns and frame `{modifies: [A/c], creates: [], deletes: []}`, `check` refuses with exactly one unproved obligation, naming `B/cs`, operation `A/reset` and `field-domain`; with `post QR using V on M::B::reset { self.cs <= 5 }` added to the unit, `check` checks it. | Test |

## Dependencies

- **Upstream:** [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md)
  supplies the effective declarations this requirement checks;
  quire-specification FR-151 owns the normative variance-axis table and
  AD-006 owns the model-view decision to keep conformance edges, subsetting
  and redefinition in the checked model view.
- **Downstream:** [FR-083](FR-083-resolve-unique-most-specific-dispatch.md)
  uses this requirement's conformance relation to decide dispatch
  applicability and dominance. Dispatch linking charges after this stage on
  the same meter (QSpec `value-accounting.md` stage order). Remaining work:
  QSL-495 links dispatch at S3.
- The I1 axes run in `model` and the refinement obligation's discharge runs
  in `check`, the direction [FR-074](FR-074-move-model-below-check.md) and
  ADR-011 §6.1 establish: `check` reads model state and the obligations I1
  records.
- [FR-104](FR-104-check-state-clauses.md) checks the `post`
  clauses whose facts discharge an obligation; FR-103 resolves each clause's
  operation and reads each operation's `redefines`.

## References

- Linear QSL-254 (AC-8: presence and multiplicity are separate narrowing axes).
- Linear QSL-642 (AC-9: the field-domain obligation over exact integers up to i128).
- Linear QSL-56 (AC-10 to AC-14: the conformance stage composed at the spine).
