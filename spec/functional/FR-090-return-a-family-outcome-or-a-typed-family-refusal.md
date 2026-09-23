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
`Result<Evaluation<T>, InternalFault>`, where
`Evaluation<T> { outcome: FamilyOutcome<T>, location: Option<check::Location>,
losses: Vec<LocatedLoss> }` is a layer-5 `value::expression` type and `T` is
the evaluated family's `ReferenceEvaluation::Observed`. `CheckedPackage::call`
and `CheckedPackage::evaluate` return `Evaluation<Value>`, written
`Evaluation`: `FamilyOutcome<Value>` beside the location and the losses.
`FamilyOutcome` is a QSL layer-3 `check`-core type with exactly two variants:
`Evaluated(quire_exact::Outcome<T>)`, which carries the kernel evaluation
outcome unchanged, and `FamilyEvaluated(FamilyResult)`, which carries a
family-owned evaluation-time result. `FamilyResult` is a layer-3 `check`-core
type with exactly two variants: `Refused(Box<dyn CatalogCoded>)`, category
`refusal`, and `Undefined(Box<dyn UndefinedCoded>)`, category `undefined`.
`CatalogCoded` and `UndefinedCoded` are F `diagnostic` traits. `InternalFault`
is ADR-013 T-4's type in F `diagnostic`
(`qsl_foundation::diagnostic::InternalFault`). A family's `evaluate` hook
returns `Result<EvalOutcome<T>, InternalFault>`, where `EvalOutcome` is a
layer-3 `check`-core type with exactly two variants: `Kernel(Outcome<T>)` and
`Family(FamilyResult)`.

S6a's input type SHALL admit no `Relation` declaration. The S6a family kind,
the closed `check`-core enum the S6a seam dispatches over, has one variant
per family that implements `ReferenceEvaluation` and no `Relation` variant.
S6a's other entry, `CheckedPackage::evaluate`, takes a `CheckedExpression`,
which is not a `Relation` declaration. So no S6a result exists for a
`Relation` declaration and no family-dispatch refusal is representable.

The evaluation-time `wrong_snapshot` cause (`WrongSnapshotCause`) and the
model-query refusal SHALL reach an S6a caller in
`FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))`, carrying their own
catalog codes. `PreconditionFalse` and the absent lookup key SHALL reach an
S6a caller in `FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(_))`,
carrying the undefined reasons `precondition-false` and `absent-key`. None of
the four is a variant of the kernel `Refusal` or `Undefined`.

This requirement carries testable criteria for decisions already taken:

- ADR-013 O-16: the three outcome families, the S6a result type, the
  `FamilyOutcome` and `FamilyResult` shapes, the owner ruling on
  FR-090-OQ-1 (QSL-174) that places the family evaluation causes in
  `FamilyOutcome::FamilyEvaluated`, the owner ruling on FR-090-OQ-2
  (QSL-174) that S6a's input type admits no `Relation`, and the owner ruling
  on FR-090-OQ-3 (QSL-174) that `Evaluation` carries the location and the
  loss records beside the `FamilyOutcome`.
- ADR-013 O-17: each cause type has one exhaustive `catalog_code()` with no
  `_` arm, and a fixed O-16 category.
- ADR-013 T-4: `InternalFault` names the stage and the violated invariant,
  maps to category `internal failure`, and is never a `Refusal`.
- ADR-013 T-6: `WrongSnapshotCause` leaves the kernel `Refusal` and becomes
  an evaluation cause, with its own `catalog_code()`, of `ProtocolClause`,
  the family that owns `Pre`; family causes are never kernel causes;
  `diagnostic::Code` leaves the kernel `Refusal`.
- ADR-011 §2.2, §2.3 and §6.1: the S6a signature, the E6 row that carries
  `Evaluation.location`, the E6 row that refuses bad arguments at admission
  before S6a (`CheckedPackage::call` returns
  `CallFailure::Input(InputRefusal)`), and the layer-3 row that places
  `FamilyOutcome`, `FamilyResult` and `EvalOutcome` in the `check` core.
- ADR-012 §2, §3, §5.1 S1 and §13.5: every family except `Relation`
  implements `ReferenceEvaluation`, whose hook returns
  `Result<EvalOutcome<Observed>, InternalFault>`; a kernel `Outcome`'s
  category follows O-16's evaluation column, and a `FamilyResult` is
  `refusal` or `undefined`; S6a dispatches over the families that implement
  `ReferenceEvaluation`, so `Relation` has no S6a arm.

## Inputs

- An S6a family kind, a checked declaration identity of that family, the
  family's evaluation environment (for `Value`, the checked package, the
  caller's object environment and argument values) and a
  `quire_exact::Meter`.
- For a clause expression (a postcondition, an invariant or another checked
  expression that is not a function declaration): the checked package, the
  `CheckedExpression`, its argument values, the caller's object environment
  and a `quire_exact::Meter`, passed to `CheckedPackage::evaluate`.
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
- For the `absent-key` path: a `lookup<T>(p, r) absent undefined` query
  (FR-153) whose reference `r` names no member of the population bound to
  `p`.

## Outputs

- `Ok(e)`, where `e` is an `Evaluation` whose `outcome` is one of:
  - `FamilyOutcome::Evaluated(outcome)`, where `outcome` is a
    `quire_exact::Outcome<Value>`;
  - `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))`, where
    `cause.catalog_code()` is the family cause's catalog code;
  - `FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))`, where
    `cause.undefined_record()` is the family cause's `UndefinedRecord`: its
    undefined reason and that reason's catalog payload.
- `e.location`, the `check::Location` of the node at which an evaluation that
  did not complete stopped, or `None` when the evaluation completed or
  stopped before any node ran (a meter charge denied at call entry).
- `e.losses`, the loss records of the operations a completed evaluation
  performed, in evaluation order; empty unless `e.outcome` is
  `FamilyOutcome::Evaluated(Outcome::Completed(_))`.
- `Err(InternalFault)` when an S6a invariant breaks.
- From `CheckedPackage::call` and `CheckedPackage::evaluate`, S6a's
  `Ok(e)` unchanged, `Err(CallFailure::Input(InputRefusal))` for an argument
  refused at admission, before S6a, and
  `Err(CallFailure::Fault(InternalFault))` for an S6a invariant break.
- `FamilyResult::Refused` carrying catalog code `wrong_snapshot` with the
  `WrongSnapshotCause` cause tag, or `ModelRefusal::catalog_code()`, for the
  `wrong_snapshot` and model-query paths above; `FamilyResult::Undefined`
  carrying reason `precondition-false` or `absent-key` for the
  `precondition-false` and `absent-key` paths.

## Behavior

### S6a returns one of three results

The S6a seam SHALL dispatch on the S6a family kind, which has one variant
per family implementing `ReferenceEvaluation` and no `Relation` variant, with
one hand-written arm per variant and no `_` arm (ADR-012 §5.1 S1). Each
arm SHALL call its family's `evaluate` hook, whose design-level shape is

```text
fn evaluate(checked: &Self::Checked, env: &mut EvalEnv, meter: &mut Meter)
    -> Result<EvalOutcome<Self::Observed>, InternalFault>;

enum EvalOutcome<T> {
    Kernel(Outcome<T>),     // the kernel evaluation outcome
    Family(FamilyResult),   // a family-owned refusal or undefined result
}
```

The arm SHALL pass each hook result through unchanged:
`Ok(EvalOutcome::Kernel(o))` becomes `Ok(e)` with `e.outcome` equal to
`FamilyOutcome::Evaluated(o)`, `Ok(EvalOutcome::Family(r))` becomes `Ok(e)`
with `e.outcome` equal to `FamilyOutcome::FamilyEvaluated(r)`, and
`Err(fault)` becomes `Err(fault)`. The kernel outcome's category follows
O-16's evaluation column (ADR-012 §13.5, Q210-3); the seam does not rewrite
it. This holds for every arm. The three results are an `Evaluation` whose
`outcome` is `FamilyOutcome::Evaluated`, an `Evaluation` whose `outcome` is
`FamilyOutcome::FamilyEvaluated`, and `Err(InternalFault)`.

The hook has its own `InternalFault` channel because it is where two S6a
invariant breaks are detected (a consumed environment and an unresolved
identity, below), and FR-090-AC-3 requires both to reach the caller as
`Err(InternalFault)`. Refusal and undefined are outcome categories (ADR-013
O-16), so a hook returns them in `Ok`, never in `Err`.

`Completed`, `Undefined` and kernel `Refused` keep their variant and payload.
A meter-budget `Incomplete` is an `Evaluated` outcome, never a
`FamilyResult` and never an `InternalFault` (ADR-013 T-4: `Incomplete` is an
S6a outcome only). When the `evaluate` hook's own meter charge is denied, the
hook returns `Ok(EvalOutcome::Kernel(Outcome::Incomplete(i)))` and the seam
SHALL return `Ok(e)` with `e.outcome` equal to
`FamilyOutcome::Evaluated(Outcome::Incomplete(i))`.

### `Evaluation` carries the location and the loss records

The S6a seam SHALL return the `FamilyOutcome` inside an `Evaluation`, beside
the evaluation's `location` and `losses`. A family's `evaluate` hook records
both in the evaluation environment it receives (`EvalEnv`, a layer-5 type;
for `Value`, `EvaluationEnv`) as it evaluates, and the seam SHALL build the
`Evaluation` from the hook's result and that environment, carrying both
unchanged (ADR-011 §2.2 E6: carried, never re-derived). The caller receives
them only inside the `Evaluation`, so it cannot drop them apart from the
outcome. `FamilyOutcome`, `FamilyResult` and `EvalOutcome` hold no
location.
`location` is `check::Location` (`src/check/refusal.rs`), the declaration
origin and child-index path that every checked expression node carries. It
is the one locus of the evaluation: the node at which an evaluation that did
not complete stopped, or `None` when the evaluation completed or stopped
before any node ran (a meter charge denied at call entry). A consumer
reports `None` as "unavailable" (`quire.native.diagnostics/v1` common
context). `losses` holds the loss records of the operations a completed
evaluation performed, in evaluation order, as QSpec FR-140-AC-3 requires
("the rounded value plus a canonical-rational loss record"); it is empty
unless the outcome is `FamilyOutcome::Evaluated(Outcome::Completed(_))`.
`Evaluation` is a layer-5 `value::expression` type because `LocatedLoss`
holds value-layer types.

Each catalog locus of a family cause is `Evaluation.location`, the node
whose evaluation raised the cause. For `precondition-false`, the catalog's
call locus is the location of the dispatched call node
`receiver.member(args)` whose selected method's precondition is `false`, not
the root of the evaluated expression. For `absent-key`, the lookup locus (and
for `invalid_runtime_input`/`absent-key`, the query locus) is the location of
the `lookup` node.

### Clause expressions evaluate through `CheckedPackage::evaluate`

`CheckedPackage::evaluate` is the S6a entry point for a checked clause
expression. It is a method of the `CheckedPackageEvaluation` extension trait,
which `value::expression` defines and implements for `CheckedPackage`
(ADR-011 §4), so a caller brings that trait into scope. Its target signature
is

```text
fn evaluate(
    &self,
    expression: &CheckedExpression,
    arguments: Vec<Value>,
    objects: &ObjectEnvironment,
    meter: &mut Meter,
) -> Result<Evaluation, CallFailure>;
```

It SHALL admit `arguments` against the expression's parameters before S6a
and return `Err(CallFailure::Input(InputRefusal))` for an argument refused
there. It SHALL return S6a's `Ok(Evaluation)` unchanged, and an S6a
`Err(InternalFault)` as `Err(CallFailure::Fault(fault))`. It returns the same
`FamilyOutcome` arms as the function-declaration seam: the evaluation-time
family results of the sections below reach a clause-expression caller
through it. `call` is a method of the same trait.

### `Relation` never enters S6a

S6a's input type SHALL admit no `Relation` declaration: the S6a family kind
has no `Relation` variant (ADR-012 §2, §3). `Relation` implements no
`ReferenceEvaluation` hook. The abstraction relation has no FR-057
capability kind (ADR-012 §7.2), so no backend proves it and no counterexample
of it exists. The refinement gates' claims have kind `operation-contract`,
one per clause implication (FR-057); their clauses reach S6a as clause
expressions through `CheckedPackage::evaluate`, not as a `Relation`
declaration. No path produces an S6a evaluation of a `Relation` declaration,
so `FamilyOutcome` has no arm for "the family does not run here". This is the
precise statement of FR-062-AC-6's non-native evaluability.

Every `FamilyOutcome::FamilyEvaluated` result means "the family ran": a
`FamilyResult::Refused(_)` is "the family ran and refused the answer", which
routing (#185) never retries.

### A family's evaluation-time result is family-owned

`FamilyResult::Refused` holds a family-owned refusal cause behind F
`diagnostic`'s `CatalogCoded` trait, whose one method is O-17's
`fn catalog_code(&self) -> CatalogCode`; its category is `refusal`.
`FamilyResult::Undefined` holds a family-owned undefined cause behind F
`diagnostic`'s `UndefinedCoded` trait, whose one method is
`fn undefined_record(&self) -> UndefinedRecord`; its category is `undefined`.
`UndefinedRecord { reason, fields }` is an F `diagnostic` type, the
undefined-side counterpart of O-17's `RefusalRecord`. `reason` is an
`UndefinedReason`, the closed reason set of the
`quire.native.diagnostics/v1` "Undefined reasons" table, which the catalog
states is not a refusal code or cause. `fields` is the structured payload that
table requires for that reason. The trait returns the record, not the bare
reason, because the catalog requires the payload and a consumer outside the
family holds only the trait object, so without the record it could reach
the payload only by downcasting to the family's type. The locus is
`Evaluation.location`, not a field of `UndefinedRecord`.

The `check` core names no family cause type: it holds each cause only
through these traits. A consumer outside the family reads the O-17
`RefusalRecord` built from a `FamilyResult::Refused` cause's
`catalog_code()`, or the `UndefinedRecord` of a `FamilyResult::Undefined`
cause, never the cause itself (ADR-013 O-16, O-17).

### A family result is an in-process trait object

`CatalogCoded` and `UndefinedCoded` each have the supertraits
`fmt::Debug + Send + Sync + 'static`, so a `FamilyOutcome` is `Debug`, can
cross a thread, and holds no borrow. `FamilyResult` and `FamilyOutcome`
derive no `Clone`, `PartialEq` or `Eq`. A test matches the arm with
`matches!` and compares `catalog_code()` or `undefined_record()`; the kernel
`Outcome` inside `Evaluated` keeps its own `Eq` and is compared with
`assert_eq!`. An assertion on a concrete cause type lives in the producing
family's unit tests (the TC-387 pattern), not at the S6a seam. `FamilyOutcome`
and `FamilyResult` are in-process only and have no serde implementation: a
result crosses a process boundary as the `RefusalRecord` or `UndefinedRecord`
built from it (ADR-013 O-16 records the rejected representations).

### Bad arguments are refused at admission; a broken invariant is an internal fault

`CheckedPackage::call` SHALL admit its arguments before S6a and return
`Result<Evaluation, CallFailure>`, with `CallFailure { Input(InputRefusal),
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
`FamilyResult` or as a kernel `Refused` outcome.

### Every family refusal has a catalog code and category `refusal`

F `diagnostic` SHALL provide the map from a `CatalogCode` to its O-16
`Category`. That map SHALL yield `Category::Refusal` for every code the
`catalog_code()` of a `FamilyResult::Refused` cause type returns: the
`ProtocolClause` snapshot cause and `ModelRefusal` (sections below). F
`diagnostic` names no `check`-core type and no family cause type: the map
reads the `CatalogCode` only.

### The evaluation-time snapshot cause is a family cause

`ProtocolClause`, the family that owns `Pre` (ADR-012 §4.3; FR-091-AC-8),
SHALL own an evaluation cause type carrying `WrongSnapshotCause` and
implementing `CatalogCoded`, defined in `value::expression` beside the
`ProtocolClause` evaluator, with its own exhaustive
`catalog_code()` returning
code `wrong_snapshot` and the cause tag of the variant it holds:
`wrong-anchor` for `WrongAnchor`, `forbidden-pre-read` for
`ForbiddenPreRead`. F `diagnostic`'s map SHALL yield `Category::Refusal` for
each of these codes. When evaluation of a `pre(..)` read meets a
`Value::Population` with no attached pre anchor, S6a SHALL return `Ok(e)` with `e.outcome` equal to
`FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))`, where
`cause.catalog_code()` is `wrong_snapshot`/`wrong-anchor`. This is an S6a
result, not an admission refusal: `admit_binding` admits the argument, and
the refusal arises when evaluation reads `pre(..)`. Under O-17, the caller
reads this as a `RefusalRecord`, never as the family cause itself. The cause
belongs to `ProtocolClause` because a cause belongs to the family whose
construct produces it, and `wrong_snapshot` arises only from a `pre(..)`
read.

### The model-query refusal is carried with its own code

`ModelRefusal` is the `StateModel` family's evaluation cause, defined in
`model` (ADR-012 §1 and §3 assign population evaluation to `StateModel`). It
SHALL have one
exhaustive `catalog_code()` (ADR-013 O-17 lists `ModelRefusal` among the
cause types that each have one) and implement `CatalogCoded`. When
`allInstances<T>(p)` or `lookup<T>(p, r)` is refused by `model::population`
with a `ModelRefusal`, S6a SHALL return `Ok(e)` with `e.outcome` equal to
`FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))`, where
`cause.catalog_code()` is that refusal's `ModelRefusal::catalog_code()`. The
carried refusal holds no native-v1 `qsl_foundation::diagnostic::Code` value:
ADR-013 R-09 gives a lane-private representation no canonical authority and
no new consumer.

### A false dispatched precondition is a family-owned undefined result

The `StateModel` family SHALL own an undefined cause type implementing
`UndefinedCoded`, defined in `value::expression` beside the `StateModel`
evaluator, with a `PreconditionFalse` variant and an `AbsentKey` variant.
For `PreconditionFalse`, `undefined_record()` returns reason
`precondition-false` with the reason's
catalog payload as `fields`: the called effective operation, the selected
method's effective identity and the receiver reference. The call locus is
`e.location`, the location of the dispatched call node.
When an FR-151 dispatched call's selected method's effective precondition
evaluates to `false`, S6a SHALL return `Ok(e)` with `e.outcome` equal to
`FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))`, where
`cause.undefined_record().reason` is `precondition-false`. The result is
category `undefined`, never `refusal`. `quire_exact::Undefined` has no
`PreconditionFalse` variant: the operation, selected method and receiver come
from FR-151 dispatch, which is QSL model vocabulary (ADR-013 O-13).

The owner is `StateModel` because ADR-012 assigns dispatch and dispatch
preconditions to it (§1 family table and checked-type table, §3 per-family
table, §4.3). The module path does not decide the owner: layer 5
`value::expression` holds every family's evaluator (ADR-011 §6.1).

### An absent lookup key is a family-owned undefined result

When a `lookup<T>(p, r) absent undefined` query (FR-153) finds no member of
`r`'s key in the population bound to `p`, S6a SHALL return `Ok(e)` with `e.outcome` equal to
`FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))`, where
`cause` is the `StateModel` undefined cause's `AbsentKey` variant and
`cause.undefined_record()` has reason `absent-key` and, as `fields`, the
reason's catalog payload: the population binding and the requested reference
key. The `lookup` locus is `e.location`, the location of the `lookup`
node. The result is category `undefined`. `quire_exact::Undefined`
has no `AbsentKey` variant, and this `UndefinedRecord` is the only carrier
of undefined reason `absent-key`.

`absent-key` is a `StateModel` undefined cause for three reasons. The
catalog requires a payload for it, which a payload-free kernel variant
cannot carry. `StateModel` lookup is the only code that detects an absent
key. A key-lookup cause is model vocabulary, which ADR-013 O-13 keeps out of
the kernel.

`lookup<T>(p, r) absent refused` is a different query: FR-153 has it refuse
an absent key. That refusal is `ModelRefusalCause::AbsentKey`, catalog code
`invalid_runtime_input` with cause tag `absent-key`, category `refusal`
(native-diagnostics `invalid_runtime_input` row; QSpec TC-198 L03), and
reaches the caller in `FamilyResult::Refused` as the model-query refusal
above. The catalog defines the `absent-key` cause tag of
`invalid_runtime_input` and the `absent-key` undefined reason as separate
entries, and each has exactly one carrier: the absence mode the query names
selects which one S6a returns.

### The kernel refusal holds kernel causes only

`quire_exact::Refusal` has no variant that names `WrongSnapshotCause`,
`ModelRefusal` or any
`qsl_foundation` type, and `quire_exact::Undefined` has no
`PreconditionFalse` or `AbsentKey` variant. A caller that matches `FamilyOutcome::Evaluated(
Outcome::Refused(r))` reads a kernel cause.

### Layering

`FamilyOutcome`, `FamilyResult` and `EvalOutcome` are
defined once, in the layer-3 `check` core; `EvalOutcome` is there because it
is the family hook's return type and the `check` core defines the hook.
`CatalogCoded`, `UndefinedCoded`
and `UndefinedRecord` are defined once, in F `diagnostic`, so that `model`
(below the `check` core) and `value::expression` (above it) can each
implement the traits for their own cause types. Under ADR-011 §6.1's
"Depends on" column, the modules that may name the three `check`-core types
are: the layer-3 `check` core and the family checker modules above it
in layer 3; layer 4 `package`; layer 5 `value::expression` and its family
evaluators and `simulation`; layer R `route`; and layer 6 `replay`,
`command`, `cli` and `main`. K (`quire-exact`), F (`qsl-foundation`),
layer 1 (`qsl-cst`), layer 2 `forms`, and the layer-3 modules ordered below
the `check` core (`semantic_value`, `model`, `library`) name none of the
three `check`-core types.
Today `semantic_value`'s modules are `value::{definition, enumeration, unit,
quantity, key, reference}` (ADR-011 §6.2).

The `check` core names no family cause type: no item under it names the
`ProtocolClause` snapshot cause type, `ModelRefusal` or the `StateModel`
undefined cause type.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-090-AC-1 | Given a checked `Value` function `f(x: Float[binary64]): Rational[-9..9 / 1..9] = convert(x)`, which the checker admits (an IEEE-to-rational conversion carries no definedness obligation), S6a returns `Ok(e)` with exactly these `e.outcome` values. For `x = 0.5` with an unlimited meter: `FamilyOutcome::Evaluated(Outcome::Completed(Value::Rational(1/2)))`, with `e.location` `None` and `e.losses` empty. For `x = NaN`: `FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::IeeeNotFinite))`, with `e.location` `Some` and `e.losses` empty. For `x = 20.0`: `FamilyOutcome::Evaluated(Outcome::Refused(Refusal::IeeeRationalOutOfDomain))`, with `e.location` `Some` and `e.losses` empty. For `x = 0.5` with a meter whose work limit is zero, the hook's `Ok(EvalOutcome::Kernel(Outcome::Incomplete(i)))` becomes `FamilyOutcome::Evaluated(Outcome::Incomplete(i))`, and `i` names `ChargePoint::FunctionCall`. None of the four is returned as `FamilyOutcome::FamilyEvaluated` or as `Err`. | Test (TC-382) |
| FR-090-AC-3 | Each S6a invariant break returns `Err(fault)` without panicking. The cases are: a second S6a call on a `Value` evaluation environment whose arguments an earlier S6a call already consumed; and an S6a call with a checked identity the package does not resolve. For each, `fault.stage()` names S6a, `fault.invariant()` is the stable identifier of that invariant (distinct for the two cases), and `fault.category()` is `Category::InternalFailure`. The result is not an `Ok(e)` whose `e.outcome` is `FamilyOutcome::Evaluated(Outcome::Refused(_))` or a `FamilyOutcome::FamilyEvaluated`. | Test (TC-384) |
| FR-090-AC-4 | S6a's input type admits no `Relation`. The S6a family kind has no `Relation` variant, and the S6a seam's family parameter has that type; `FamilyOutcome` has exactly the two variants `Evaluated` and `FamilyEvaluated`. A test holds an exhaustive `match` with no `_` arm over each type whose arms name neither `Relation` nor a third `FamilyOutcome` variant, so adding either variant fails to compile, and the test passes each S6a family kind variant to the S6a seam's dispatch, which compiles only if the seam takes that type. This is the precise form of FR-062-AC-6. | Test (TC-385) |
| FR-090-AC-5 | F `diagnostic`'s catalog-code-to-category map returns `Category::Refusal` for every code the `ProtocolClause` snapshot cause's `catalog_code()` returns and for every code `ModelRefusal::catalog_code()` returns; `qsl-foundation` has no dependency on the crate that defines `FamilyOutcome` or on any crate at layer 3 or above. | Test (TC-386) |
| FR-090-AC-6 | The `ProtocolClause` family cause carrying `WrongSnapshotCause` has an exhaustive `catalog_code()` with no `_` arm that returns `wrong_snapshot`/`wrong-anchor` for `WrongAnchor` and `wrong_snapshot`/`forbidden-pre-read` for `ForbiddenPreRead`. | Test (TC-387) |
| FR-090-AC-7 | Given a postcondition `pre(allInstances<T>(p))` evaluated through `CheckedPackage::evaluate` with a population argument admitted through `admit_binding` (no pre anchor), the result is `Ok(e)` with `e.outcome` equal to `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))` and `cause.catalog_code()` equal to `wrong_snapshot`/`wrong-anchor`. The result is not a panic, not `FamilyOutcome::Evaluated(Outcome::Refused(_))` and not `Err(CallFailure::Input(_))`. `quire_exact::Refusal` has no variant naming `WrongSnapshotCause`. | Test (TC-388) |
| FR-090-AC-8 | Given an `allInstances<T>(p)` query whose selected member count is above the population's declared maximum, evaluated through `CheckedPackage::evaluate`, the result is `Ok(e)` with `e.outcome` equal to `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))` and `cause.catalog_code()` equal to the refusing `ModelRefusal`'s `catalog_code()`, `cardinality_out_of_bound`/`above-maximum`. The result is not a panic and not `FamilyOutcome::Evaluated(Outcome::Refused(_))`. The carried refusal holds no `qsl_foundation::diagnostic::Code` value. | Test (TC-389) |
| FR-090-AC-9 | `FamilyOutcome`, `FamilyResult` and `EvalOutcome` are each defined once, in the layer-3 `check` core. No `use` edge or inline path under `qsl-forms/src/`, `src/model/`, `src/library/`, the `semantic_value` modules (`src/value/{definition, enumeration, unit, quantity, key, reference}`) or `qsl-cst/src/` resolves to any of the three. No `use` edge or inline path under the `check` core resolves to the `ProtocolClause` snapshot cause type, `ModelRefusal` or the `StateModel` undefined cause type. Neither `quire-exact`, `qsl-foundation` nor `qsl-cst` depends on the crate that defines the three. | Test (TC-390) |
| FR-090-AC-10 | Given a checked `Value` function with a `Population<T>[N]` parameter, `CheckedPackage::call` with an argument whose `PopulationId` names no recorded binding, or whose resolved binding's declared maximum differs from `N`, returns `Err(CallFailure::Input(_))` before S6a runs. Given the same argument passed directly to the S6a seam, bypassing admission, S6a returns `Err(InternalFault)`, not a kernel or family refusal. | Test (TC-391) |
| FR-090-AC-11 | Given an FR-151 dispatched call `receiver.member(args)` whose selected method's effective precondition evaluates to `false` (QSpec TC-196 D06), evaluated through `CheckedPackage::evaluate`, the result is `Ok(e)` with `e.outcome` equal to `FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))`, where `cause.undefined_record()` has `reason` `precondition-false` and `fields` naming the called effective operation, the selected method's effective identity and the receiver reference, and `e.location` is the location of the dispatched call node, not of the evaluated expression's root. The result is not `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))`, not `FamilyOutcome::Evaluated(Outcome::Undefined(_))` and not a panic, and `quire_exact::Undefined` has no `PreconditionFalse` variant. | Test (TC-407) |
| FR-090-AC-12 | Given a `lookup<T>(p, r) absent undefined` query whose reference `r` names no member of the population bound to `p`, evaluated through `CheckedPackage::evaluate`, the result is `Ok(e)` with `e.outcome` equal to `FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))`, where `cause.undefined_record()` has `reason` `absent-key` and `fields` naming the population binding and the requested reference key. The result is not `FamilyOutcome::Evaluated(Outcome::Undefined(_))`, not `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))` and not a panic, and `quire_exact::Undefined` has no `AbsentKey` variant. The same query with `absent refused` returns `Ok(e)` with `e.outcome` equal to `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))` and `cause.catalog_code()` equal to `invalid_runtime_input`/`absent-key`. | Test (TC-408) |

## Dependencies

- **Upstream:** [FR-062](FR-062-implement-checked-family-contract.md) owns
  `FamilyContract`, `ReferenceEvaluation` and `FamilyKind`. The S6a family
  kind has one variant for each `FamilyKind` that implements
  `ReferenceEvaluation`. FR-090-AC-4 is the precise form of FR-062-AC-6.
  [FR-068](FR-068-split-expression-checking-into-check-stage.md) placed
  `WrongSnapshotCause` in `crate::check` (FR-068-AC-4, AC-8).
  [FR-089](FR-089-carry-population-identity-across-the-kernel-boundary.md)
  owns the population-identity admission checks FR-090-AC-10 routes through
  `CallFailure::Input`. F `diagnostic`'s `CatalogCode`, `Category` and
  `InternalFault` landed with ADR-013 §7 S-5a.
- **Downstream:** none. QSL-131's removal of `src/value/outcome.rs`'s kernel
  copy in favour of `quire_exact::{Outcome, Refusal, Undefined}` needs no
  FR-090-AC change: QSL-174 already stripped that copy down to the kernel's
  own variant set before this removal, so it carried no `WrongSnapshot`,
  `Model`, `UnresolvedPopulation` or `PopulationMaximumMismatch` refusal
  variant, and no `Undefined::PreconditionFalse` or `Undefined::AbsentKey`
  variant, for the removal to affect (see Status, "`quire_exact::Undefined`
  and `Refusal`, and QSL's kernel copy ... have no `PreconditionFalse`,
  `AbsentKey`, `WrongSnapshot` or `Model` variant"). FR-090-AC-1's `Evaluated`
  payload was already `quire_exact::Outcome<T>`; QSL-131 O2 is exactly the
  removal that lets the `Value` evaluator produce it directly, with no
  coupled change on either side.
- ADR-012 §14.1 lists `Relation`'s non-native evaluability under QSL-152
  (FR-062-AC-6). FR-090-AC-4 specifies it as a property of S6a's input type;
  which ticket builds that type is a ticketing question, not a design one.

## Status

Implemented under QSL-174 (ADR-013 O-16, O-17, T-4, T-6), all open
questions ruled.

`FamilyOutcome<T>` has two arms, `Evaluated(quire_exact::Outcome<T>)` and
`FamilyEvaluated(FamilyResult)`: S6a admits no `Relation`
(`src/family/evaluation.rs`). `EvalOutcome<T> { Kernel(Outcome<T>),
Family(FamilyResult) }` is the `evaluate` hook's return shape.
`Evaluation { outcome: FamilyOutcome<Value>, location, losses }` is
`CheckedPackage::call`'s and `CheckedPackage::evaluate`'s result
(`src/value/expression/{evaluate,mod}.rs`). `ValueFunctionFamily::evaluate`
records `location` and `losses` as owned fields of its `EvaluationEnv` on
every `Ok` return, and the S6a seam builds the `Evaluation` from them.

The family cause types are defined in `value::expression`
(`src/value/expression/causes.rs`): `ProtocolClauseSnapshot`,
`StateModelUndefined { PreconditionFalse, AbsentKey { binding, key } }` and
`ModelQueryRefusal { cause, detail }`, the model-query refusal S6a carries,
which holds no native-v1 `Code`. `value::model_query` returns a
model-layer `ModelQueryHalt`, and the evaluator turns it into a
`FamilyResult`. `ModelRefusalCause::catalog_code()` is one exhaustive match
with no `_` arm (`src/model/refusal.rs`), and `ModelRefusal::catalog_code()`
delegates to it. F `diagnostic`'s `category_of` maps a `CatalogCode` to its
O-16 category. `quire_exact::Undefined` and `Refusal`, and QSL's kernel copy
in `src/value/outcome.rs`, have no `PreconditionFalse`, `AbsentKey`,
`WrongSnapshot` or `Model` variant.

The S6a family kind is `S6aFamilyKind { Value }` (`src/family/mod.rs`):
`Value` is the family that implements `ReferenceEvaluation`. The S6a seam
`evaluate_declaration` (`src/value/expression/mod.rs`) matches it with one
arm per variant and no `_` arm, and `CheckedPackage::call` evaluates
through it (QSL-191).

FR-090-AC-1 and AC-3 to AC-12 (TC-382, TC-384 to TC-391, TC-407, TC-408)
are `✅ Passed locally`.
