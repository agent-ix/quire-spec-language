---
id: FR-090
title: "Return a family outcome or a typed family refusal from S6a evaluation"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-013
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: traces_to
---
# FR-090: Return a family outcome or a typed family refusal from S6a evaluation

## Description

ADR-011 S6a reference evaluation SHALL return
`Result<FamilyOutcome<T>, InternalFault>`. `FamilyOutcome` is a QSL layer-3
`check`-core type with exactly three variants:
`Evaluated(quire_exact::Outcome<T>)`, which carries the kernel evaluation
outcome unchanged; `Refused(FamilyRefusal)`, which carries a typed
family-dispatch refusal; and `FamilyEvaluated(FamilyResult)`, which carries a
family-owned evaluation-time result. `FamilyResult` is a layer-3 `check`-core
type with exactly two variants: `Refused(Box<dyn CatalogCoded>)`, category
`refusal`, and `Undefined(Box<dyn UndefinedCoded>)`, category `undefined`.
`CatalogCoded` and `UndefinedCoded` are F `diagnostic` traits. `InternalFault`
is ADR-013 T-4's type in F `diagnostic`
(`qsl_foundation::diagnostic::InternalFault`).

The `check` core SHALL define `FamilyRefusal` as a closed enum of
family-dispatch causes only, starting with `FamilyNotNativelyEvaluable`, with
one exhaustive `catalog_code()`. F `diagnostic` SHALL map each such catalog
code to O-16 category `refusal`.

The evaluation-time `wrong_snapshot` cause (`WrongSnapshotCause`) and the
model-query refusal SHALL reach an S6a caller in
`FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))`, carrying their own
catalog codes. `PreconditionFalse` SHALL reach an S6a caller in
`FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(_))`, carrying the
undefined reason `precondition-false`. None of the three is a variant of the
kernel `Refusal` or `Undefined`, or of `FamilyRefusal`.

This requirement carries testable criteria for decisions already taken:

- ADR-013 O-16: the three outcome families, the S6a result type, the
  `FamilyOutcome` and `FamilyResult` shapes, `FamilyRefusal`'s first cause,
  F `diagnostic` mapping `FamilyRefusal::catalog_code()` to category
  `refusal` while never naming `FamilyRefusal`, and the owner ruling on
  FR-090-OQ-1 (QSL-174) that places the three evaluation causes in
  `FamilyOutcome::FamilyEvaluated`.
- ADR-013 O-17: each cause type has one exhaustive `catalog_code()` with no
  `_` arm, and a fixed O-16 category.
- ADR-013 T-4: `InternalFault` names the stage and the violated invariant,
  maps to category `internal failure`, and is never a `Refusal`.
- ADR-013 T-6: `WrongSnapshotCause` leaves the kernel `Refusal` and becomes a
  `Value`-family evaluation cause in `value::expression` with its own
  `catalog_code()`; family-
  dispatch causes are `FamilyRefusal` causes in the layer-3 `check` core and
  never kernel causes; `diagnostic::Code` leaves the kernel `Refusal`.
- ADR-011 §2.3 and §6.1: the S6a signature, the E6 row that refuses bad
  arguments at admission before S6a (`CheckedPackage::call` returns
  `CallFailure::Input(InputRefusal)`), and the layer-3 row that places
  `FamilyOutcome` and `FamilyRefusal` in the `check` core.
- ADR-012 §2, §3, §5.1 S1 and §13.5: every family except `Relation`
  implements `ReferenceEvaluation`, whose kernel `Outcome` maps by O-16's
  evaluation column and whose `FamilyResult` maps to `refusal` or
  `undefined`; the S6a seam's explicit `Relation` arm returns
  `FamilyOutcome::Refused(FamilyRefusal::FamilyNotNativelyEvaluable)`,
  category `refusal`.

## Inputs

- A checked declaration identity of a given `FamilyKind`, the family's
  evaluation environment (for `Value`, the checked package, the caller's
  object environment and argument values) and a `quire_exact::Meter`.
- For argument admission: the caller's argument values, including
  `Value::Population(PopulationId)` arguments.
- For the `wrong_snapshot` path: a postcondition whose `pre(..)` reads a
  `Value::Population` argument admitted through `admit_binding`, which
  attaches no pre anchor.
- For the model-query path: an `allInstances<T>(p)` or `lookup<T>(p, r)`
  query that `model::population` refuses with a `ModelRefusal`.
- For the `precondition-false` path: an FR-151 dispatched call
  `receiver.member(args)` whose selected method's effective precondition
  evaluates to `false`.

## Outputs

- `Ok(FamilyOutcome::Evaluated(outcome))`, where `outcome` is a
  `quire_exact::Outcome<T>`.
- `Ok(FamilyOutcome::Refused(refusal))`, where `refusal` is a
  `FamilyRefusal` with a catalog code.
- `Ok(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)))`, where
  `cause.catalog_code()` is the family cause's catalog code.
- `Ok(FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)))`, where
  `cause.undefined_reason()` is the family cause's undefined reason.
- `Err(InternalFault)` when an S6a invariant breaks.
- From `CheckedPackage::call`, `Err(CallFailure::Input(InputRefusal))` for an
  argument refused at admission, before S6a.
- `FamilyResult::Refused` carrying catalog code `wrong_snapshot` with the
  `WrongSnapshotCause` cause tag, or `ModelRefusal::catalog_code()`, for the
  `wrong_snapshot` and model-query paths above; `FamilyResult::Undefined`
  carrying reason `precondition-false` for the `precondition-false` path.

## Behavior

### S6a returns one of four results

The S6a seam SHALL dispatch on `FamilyKind` with one hand-written arm per
family and no `_` arm (ADR-012 §5.1 S1). Every family except `Relation`
implements `ReferenceEvaluation` (ADR-012 §2); its arm SHALL call that
family's `evaluate` hook. When the hook returns a kernel outcome, the arm
SHALL return it in `FamilyOutcome::Evaluated`, mapped by O-16's evaluation
column (ADR-012 §13.5, Q210-3). When the hook returns a `FamilyResult`, the
arm SHALL return it unchanged in `FamilyOutcome::FamilyEvaluated`. This holds
for the `Value`, `StateModel`, `SumCase`, `TemporalTrace` and
`ProtocolClause` arms. The four results are `FamilyOutcome::Evaluated`,
`FamilyOutcome::Refused`, `FamilyOutcome::FamilyEvaluated` and
`Err(InternalFault)`.

`Completed`, `Undefined` and kernel `Refused` keep their variant and payload.
A meter-budget `Incomplete` is an `Evaluated` outcome, never a
`FamilyRefusal` and never an `InternalFault` (ADR-013 T-4: `Incomplete` is an
S6a outcome only). When the `evaluate` hook reports its own meter charge
denied as `EvaluateFailure::Incomplete(i)`, the seam SHALL return
`Ok(FamilyOutcome::Evaluated(Outcome::Incomplete(i)))`.

### A family that sits out evaluation refuses with a named cause

The `Relation` arm SHALL return
`Ok(FamilyOutcome::Refused(FamilyRefusal::FamilyNotNativelyEvaluable))`
without calling any `evaluate` hook (ADR-012 §2, §3). This is the precise
statement of FR-062-AC-6's "named refusal stating non-native evaluability".

`FamilyRefusal` holds family-dispatch causes only: "the family does not run
here", which routing (#185) may answer by trying another backend.
`FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))` means "the family
ran and refused the answer", which is never retried. No `FamilyRefusal`
variant carries an evaluation cause.

### A family's evaluation-time result is family-owned

`FamilyResult::Refused` holds a family-owned refusal cause behind F
`diagnostic`'s `CatalogCoded` trait, whose one method is O-17's
`fn catalog_code(&self) -> CatalogCode`; its category is `refusal`.
`FamilyResult::Undefined` holds a family-owned undefined cause behind F
`diagnostic`'s `UndefinedCoded` trait, whose one method is
`fn undefined_reason(&self) -> UndefinedReason`; its category is `undefined`.
`UndefinedReason` is the closed reason set of the `quire.native.diagnostics/v1`
"Undefined reasons" table, which the catalog states is not a refusal code or
cause. The `check` core names no family cause type: it holds each cause only
through these traits. A consumer outside the family reads the O-17
`RefusalRecord` built from a `FamilyResult::Refused` cause's
`catalog_code()`, or the undefined reason of a `FamilyResult::Undefined`
cause, never the cause itself (ADR-013 O-16, O-17).

### Bad arguments are refused at admission; a broken invariant is an internal fault

`CheckedPackage::call` SHALL admit its arguments before S6a and return
`Result<FamilyOutcome, CallFailure>`, with `CallFailure { Input(InputRefusal),
Fault(InternalFault) }` (ADR-013 T-4; ADR-011 §2.3 E6 row). A
`Value::Population(population_id)` argument whose identity names no recorded
binding, or whose resolved binding's declared maximum differs from the
parameter's, SHALL be refused there as `CallFailure::Input(InputRefusal)`.
FR-089-AC-4 and FR-089-AC-5 already refuse both at admission (`validate`).

When an S6a invariant breaks, the seam SHALL return `Err(InternalFault)`
naming stage S6a and the violated invariant by a stable identifier (ADR-013
T-4). Because the E6 row places bad input at admission, the following
S6a-internal conditions are invariant breaks:

- an evaluation environment whose arguments an earlier call already consumed
  (`EvaluateRefusal::EnvironmentAlreadyConsumed`);
- a checked identity that the package does not resolve
  (`EvaluateRefusal::UnknownIdentity`), since `call` resolved the function
  before S6a;
- a `Value::Population` identity the evaluator cannot resolve, or whose
  declared maximum does not match (today's evaluator-level
  `Refusal::UnresolvedPopulation` and `Refusal::PopulationMaximumMismatch`).

No S6a path panics, and no S6a path reports an internal fault as a
`FamilyRefusal` or as a kernel `Refused` outcome.

### Every family refusal has a catalog code and category `refusal`

`FamilyRefusal::catalog_code(&self) -> CatalogCode` SHALL match every
variant with no `_` arm and return a code and cause the `quire.native.
diagnostics/v1` catalog revision selected by FR-322 defines (ADR-013 O-17
Serialized authority). F `diagnostic` SHALL provide the map from a
`CatalogCode` to its O-16 `Category`, and that map SHALL yield
`Category::Refusal` for every code `FamilyRefusal::catalog_code()` returns.
F `diagnostic` names no `FamilyRefusal`: the map reads the `CatalogCode`
only.

### The evaluation-time snapshot cause is a family cause

The `value::expression` family SHALL own a `Value`-family evaluation cause
type carrying `WrongSnapshotCause` and implementing `CatalogCoded`, with its
own exhaustive `catalog_code()` returning
code `wrong_snapshot` and the cause tag of the variant it holds:
`wrong-anchor` for `WrongAnchor`, `forbidden-pre-read` for
`ForbiddenPreRead`. F `diagnostic`'s map SHALL yield `Category::Refusal` for
each of these codes. When evaluation of a `pre(..)` read meets a
`Value::Population` with no attached pre anchor, S6a SHALL return
`Ok(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)))`, where
`cause.catalog_code()` is `wrong_snapshot`/`wrong-anchor`. This is an S6a
result, not an admission refusal: `admit_binding` admits the argument, and
the refusal arises when evaluation reads `pre(..)`. Under O-17, the caller
reads this as a `RefusalRecord`, never as the family cause itself.

### The model-query refusal is carried with its own code

`ModelRefusal` is the model family's evaluation cause. It SHALL have one
exhaustive `catalog_code()` (ADR-013 O-17 lists `ModelRefusal` among the
cause types that each have one) and implement `CatalogCoded`. When
`allInstances<T>(p)` or `lookup<T>(p, r)` is refused by `model::population`
with a `ModelRefusal`, S6a SHALL return
`Ok(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)))`, where
`cause.catalog_code()` is that refusal's `ModelRefusal::catalog_code()`. The
carried refusal holds no native-v1 `qsl_foundation::diagnostic::Code` value:
ADR-013 R-09 gives a lane-private representation no canonical authority and
no new consumer.

### A false dispatched precondition is a family-owned undefined result

The `value::expression` family SHALL own a `Value`-family undefined cause
type with a `PreconditionFalse` variant implementing `UndefinedCoded`. Its
payload is the `precondition-false` reason's catalog payload: the called
effective operation, the selected method's effective identity and the
receiver reference. The call locus is the evaluation's location, whose
carrier is FR-090-OQ-3.
When an FR-151 dispatched call's selected method's effective precondition
evaluates to `false`, S6a SHALL return
`Ok(FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)))`, where
`cause.undefined_reason()` is `precondition-false`. The result is category
`undefined`, never `refusal`. `quire_exact::Undefined` has no
`PreconditionFalse` variant: the operation, selected method and receiver come
from FR-151 dispatch, which is QSL model vocabulary (ADR-013 O-13).

### The kernel refusal holds kernel causes only

`quire_exact::Refusal` has no variant that names `WrongSnapshotCause`,
`ModelRefusal`, `FamilyRefusal`, `FamilyNotNativelyEvaluable` or any
`qsl_foundation` type, and `quire_exact::Undefined` has no
`PreconditionFalse` variant. A caller that matches `FamilyOutcome::Evaluated(
Outcome::Refused(r))` reads a kernel cause.

### Layering

`FamilyOutcome`, `FamilyRefusal` and `FamilyResult` are defined once, in the
layer-3 `check` core. `CatalogCoded` and `UndefinedCoded` are defined once,
in F `diagnostic`, so that `model` (below the `check` core) and
`value::expression` (above it) can each implement them for their own cause
types. Under ADR-011 §6.1's "Depends on" column, the modules that may name
them are: the layer-3 `check` core and the family checker modules above it
in layer 3; layer 4 `package`; layer 5 `value::expression` and its family
evaluators and `simulation`; layer R `route`; and layer 6 `replay`,
`command`, `cli` and `main`. K (`quire-exact`), F (`qsl-foundation`),
layer 1 (`qsl-cst`), layer 2 `forms`, and the layer-3 modules ordered below
the `check` core (`semantic_value`, `model`, `library`) name neither type.
Today `semantic_value`'s modules are `value::{definition, enumeration, unit,
quantity, key, reference}` (ADR-011 §6.2).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-090-AC-1 | Given a checked `Value` function `f(x: Float[binary64]): Rational[-9..9 / 1..9] = convert(x)`, which the checker admits (an IEEE-to-rational conversion carries no definedness obligation), S6a returns exactly these results. For `x = 0.5` with an unlimited meter: `Ok(FamilyOutcome::Evaluated(Outcome::Completed(Value::Rational(1/2))))`. For `x = NaN`: `Ok(FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::IeeeNotFinite)))`. For `x = 20.0`: `Ok(FamilyOutcome::Evaluated(Outcome::Refused(Refusal::IeeeRationalOutOfDomain)))`. For `x = 0.5` with a meter whose work limit is zero, the hook's `EvaluateFailure::Incomplete(i)` becomes `Ok(FamilyOutcome::Evaluated(Outcome::Incomplete(i)))`, and `i` names `ChargePoint::FunctionCall`. None of the four is returned as `FamilyOutcome::Refused` or as `Err`. | Test (TC-382) |
| FR-090-AC-2 | Given a `Relation` declaration, S6a returns `Ok(FamilyOutcome::Refused(FamilyRefusal::FamilyNotNativelyEvaluable))` and calls no `evaluate` hook. This is the precise form of FR-062-AC-6. | Test (TC-383) |
| FR-090-AC-3 | Each S6a invariant break returns `Err(fault)` without panicking. The cases are: a second S6a call on a `Value` evaluation environment whose arguments an earlier S6a call already consumed; and an S6a call with a checked identity the package does not resolve. For each, `fault.stage()` names S6a, `fault.invariant()` is the stable identifier of that invariant (distinct for the two cases), and `fault.category()` is `Category::InternalFailure`. The result is neither `Ok(FamilyOutcome::Refused(_))` nor `Ok(FamilyOutcome::Evaluated(Outcome::Refused(_)))`. | Test (TC-384) |
| FR-090-AC-4 | For every `FamilyRefusal` variant, `catalog_code()` returns a code and cause defined by the `quire.native.diagnostics/v1` revision FR-322 selects; the method matches every variant with no `_` arm, so adding a variant without an arm fails to compile. | Test (TC-385) |
| FR-090-AC-5 | F `diagnostic`'s catalog-code-to-category map returns `Category::Refusal` for the code of every `FamilyRefusal` variant and for every code the `value::expression` family cause's `catalog_code()` returns; `qsl-foundation` has no dependency on the crate that defines `FamilyRefusal` or on any crate at layer 3 or above. | Test (TC-386) |
| FR-090-AC-6 | The `value::expression` family cause carrying `WrongSnapshotCause` has an exhaustive `catalog_code()` with no `_` arm that returns `wrong_snapshot`/`wrong-anchor` for `WrongAnchor` and `wrong_snapshot`/`forbidden-pre-read` for `ForbiddenPreRead`. | Test (TC-387) |
| FR-090-AC-7 | Given a postcondition `pre(allInstances<T>(p))` evaluated with a population argument admitted through `admit_binding` (no pre anchor), S6a returns `Ok(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)))` with `cause.catalog_code()` equal to `wrong_snapshot`/`wrong-anchor`. The result is not a panic, not `FamilyOutcome::Evaluated(Outcome::Refused(_))`, not `FamilyOutcome::Refused(_)` and not `Err(CallFailure::Input(_))` from `CheckedPackage::call`. Neither `quire_exact::Refusal` nor `FamilyRefusal` has a variant naming `WrongSnapshotCause`. | Test (TC-388) |
| FR-090-AC-8 | Given an `allInstances<T>(p)` query whose selected member count is above the population's declared maximum, S6a returns `Ok(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)))` with `cause.catalog_code()` equal to the refusing `ModelRefusal`'s `catalog_code()`, `cardinality_out_of_bound`/`above-maximum`. The result is not a panic, not `FamilyOutcome::Evaluated(Outcome::Refused(_))` and not `FamilyOutcome::Refused(_)`. The carried refusal holds no `qsl_foundation::diagnostic::Code` value, and `FamilyRefusal` has no variant naming `ModelRefusal`. | Test (TC-389) |
| FR-090-AC-9 | `FamilyOutcome` and `FamilyRefusal` are each defined once, in the layer-3 `check` core. No `use` edge or inline path under `src/forms/`, `src/model/`, `src/library/`, the `semantic_value` modules (`src/value/{definition, enumeration, unit, quantity, key, reference}`) or `qsl-cst/src/` resolves to either type. Neither `quire-exact`, `qsl-foundation` nor `qsl-cst` depends on the crate that defines them. | Test (TC-390) |
| FR-090-AC-10 | Given a checked `Value` function with a `Population<T>[N]` parameter, `CheckedPackage::call` with an argument whose `PopulationId` names no recorded binding, or whose resolved binding's declared maximum differs from `N`, returns `Err(CallFailure::Input(_))` before S6a runs. Given the same argument passed directly to the S6a seam, bypassing admission, S6a returns `Err(InternalFault)`, not a kernel or family refusal. | Test (TC-391) |
| FR-090-AC-11 | Given an FR-151 dispatched call `receiver.member(args)` whose selected method's effective precondition evaluates to `false` (QSpec TC-196 D06), S6a returns `Ok(FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)))` with `cause.undefined_reason()` equal to `precondition-false`, and the cause's payload names the called effective operation, the selected method's effective identity and the receiver reference. The result is not `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))`, not `FamilyOutcome::Evaluated(Outcome::Undefined(_))`, not `FamilyOutcome::Refused(_)` and not a panic, and `quire_exact::Undefined` has no `PreconditionFalse` variant. | Test (TC-407) |

## Dependencies

- **Upstream:** [FR-062](FR-062-implement-checked-family-contract.md) owns
  `FamilyContract`, `ReferenceEvaluation` and `FamilyKind`, which the S6a
  seam dispatches over. FR-090-AC-2 is the precise form of FR-062-AC-6.
  [FR-068](FR-068-split-expression-checking-into-check-stage.md) placed
  `WrongSnapshotCause` in `crate::check` (FR-068-AC-4, AC-8).
  [FR-089](FR-089-carry-population-identity-across-the-kernel-boundary.md)
  owns the population-identity admission checks FR-090-AC-10 routes through
  `CallFailure::Input`. F `diagnostic`'s `CatalogCode`, `Category` and
  `InternalFault` landed with ADR-013 §7 S-5a.
- **Downstream:** QSL-131's removal of `src/value/outcome.rs`'s kernel copy
  in favour of `quire_exact::{Outcome, Refusal}` needs FR-090-AC-7,
  FR-090-AC-8, FR-090-AC-10 and FR-090-AC-11, because that copy's
  `WrongSnapshot`, `Model`, `UnresolvedPopulation` and
  `PopulationMaximumMismatch` refusal variants and its
  `Undefined::PreconditionFalse` variant are ones `quire_exact` does not
  have. In the other direction,
  FR-090-AC-1's `Evaluated` payload is `quire_exact::Outcome<T>`, which the
  `Value` evaluator produces once that copy is gone. The two changes are
  coupled in both directions.
- ADR-012 §14.1 lists "`Relation`'s non-native-evaluability arm" under
  QSL-152 (FR-062-AC-6). FR-090-AC-2 specifies that arm's behaviour; which
  ticket builds it is a ticketing question, not a design one.

## Status

Specified under QSL-174 (ADR-013 O-16, O-17, T-4, T-6). Not yet implemented.
Today `FamilyOutcome`, `FamilyRefusal` and `FamilyNotNativelyEvaluable`
appear under `src/` only in doc comments, and the provisional stand-in is
`family::EvaluateRefusal`. `CheckedPackage::call` returns
`Result<Evaluation, InputRefusal>` and ends its consumed-environment arm in
`unreachable!`. F `diagnostic` has no catalog-code-to-category map yet.
`model::normalize::ModelRefusal` has no `catalog_code()` yet, though O-17
requires one; it carries the native-v1 `Code` instead.
`quire_exact::Meter::charge` is public (QSL-153 and QSL-166 are Done), so
FR-090-AC-1's `Incomplete` case is constructible.

FR-090-AC-4 and FR-090-AC-5 for `FamilyNotNativelyEvaluable` wait on
FR-090-OQ-2. `FamilyResult`, `CatalogCoded`, `UndefinedCoded` and
`UndefinedReason` do not exist yet; `Undefined::PreconditionFalse` is still a
variant of QSL's kernel copy in `src/value/outcome.rs`. Remaining work:
QSL-174 implementation.

## Open Questions

- **FR-090-OQ-2: No catalog code exists for `FamilyNotNativelyEvaluable`.**
  ADR-013 O-16 requires `FamilyRefusal::catalog_code()`, and O-17 requires
  the code to come from the catalog revision FR-322 selects
  (`quire.native.diagnostics/v1` `1-draft.6`, owned by
  `ix://agent-ix/quire-specification`). O-17 also forbids a conversion that
  invents a code. That revision's code table has no code for a family that
  does not evaluate natively, and its `unsupported_*` codes are category
  `unsupported`, not `refusal`. A catalog entry is needed from
  `ix://agent-ix/quire-specification`, in the way ADR-013 QC-11 obtained
  `stage_limit_exceeded`.
- **FR-090-OQ-3: Where do an evaluation's location and loss records travel?**
  ADR-011 §2.2's E6 row requires `Evaluation.location` to be carried, and
  ADR-011 §2.3 and O-16 fix `FamilyOutcome::Evaluated`'s payload as the
  kernel `Outcome<T>`, unchanged. So location cannot be dropped, and
  `Evaluated` does not wrap an `Evaluation`. The ADRs do not say where
  location and losses travel beside the `FamilyOutcome` that S6a returns.
  They never mention losses.
