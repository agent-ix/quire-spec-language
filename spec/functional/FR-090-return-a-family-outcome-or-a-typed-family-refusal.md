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
`check`-core type with exactly two variants:
`Evaluated(quire_exact::Outcome<T>)`, which carries the kernel evaluation
outcome unchanged, and `Refused(FamilyRefusal)`, which carries a typed
family-dispatch refusal. `InternalFault` is ADR-013 T-4's type in F
`diagnostic` (`qsl_foundation::diagnostic::InternalFault`).

The `check` core SHALL define `FamilyRefusal` as a closed enum whose causes
start with `FamilyNotNativelyEvaluable`, with one exhaustive
`catalog_code()`. F `diagnostic` SHALL map each such catalog code to O-16
category `refusal`.

The evaluation-time `wrong_snapshot` cause (`WrongSnapshotCause`) and the
model-query refusal SHALL reach an S6a caller as QSL refusals carrying their
own catalog codes, never as variants of the kernel `Refusal`.

This requirement carries testable criteria for decisions already taken:

- ADR-013 O-16: the three outcome families, the S6a result type, the
  `FamilyOutcome` shape, `FamilyRefusal`'s first cause, and F `diagnostic`
  mapping `FamilyRefusal::catalog_code()` to category `refusal` while never
  naming `FamilyRefusal`.
- ADR-013 O-17: each cause type has one exhaustive `catalog_code()` with no
  `_` arm, and a fixed O-16 category.
- ADR-013 T-4: `InternalFault` names the stage and the violated invariant,
  maps to category `internal failure`, and is never a `Refusal`.
- ADR-013 T-6: `WrongSnapshotCause` leaves the kernel `Refusal` and becomes a
  `value::expression` family cause with its own `catalog_code()`; family-
  dispatch causes are `FamilyRefusal` causes in the layer-3 `check` core and
  never kernel causes; `diagnostic::Code` leaves the kernel `Refusal`.
- ADR-011 §2.3 and §6.1: the S6a signature, and the layer-3 row that places
  `FamilyOutcome` and `FamilyRefusal` in the `check` core.
- ADR-012 §2, §3, §5.1 S1 and §13.5: the S6a seam's explicit `Relation` arm
  returns `FamilyOutcome::Refused(FamilyRefusal::FamilyNotNativelyEvaluable)`,
  category `refusal`.

## Inputs

- A checked declaration identity of a given `FamilyKind`, the family's
  evaluation environment (for `Value`, the checked package, the caller's
  object environment and argument values) and a `quire_exact::Meter`.
- For the `wrong_snapshot` path: a postcondition whose `pre(..)` reads a
  `Value::Population` argument admitted through `admit_binding`, which
  attaches no pre anchor.
- For the model-query path: an `allInstances<T>(p)` or `lookup<T>(p, r)`
  query that `model::population` refuses with a `ModelRefusal`.

## Outputs

- `Ok(FamilyOutcome::Evaluated(outcome))`, where `outcome` is the
  `quire_exact::Outcome<T>` the family's `evaluate` hook produced.
- `Ok(FamilyOutcome::Refused(refusal))`, where `refusal` is a
  `FamilyRefusal` with a catalog code.
- `Err(InternalFault)` when an S6a invariant breaks.
- A QSL refusal carrying catalog code `wrong_snapshot` with the
  `WrongSnapshotCause` cause tag, or the refusing `ModelRefusal`'s own code
  and cause, for the two evaluation-time paths above.

## Behavior

### S6a returns one of three results

The S6a seam SHALL dispatch on `FamilyKind` with one hand-written arm per
family and no `_` arm (ADR-012 §5.1 S1). For a family that implements
`ReferenceEvaluation`, the arm calls the family's `evaluate` hook and
returns every kernel outcome it produces in `FamilyOutcome::Evaluated`:
`Completed`, `Undefined`, kernel `Refused` and `Incomplete` keep their
variant and payload. A meter-budget `Incomplete` is an `Evaluated` outcome,
never a `FamilyRefusal` and never an `InternalFault` (ADR-013 T-4: `Incomplete`
is an S6a outcome only).

### A family that sits out evaluation refuses with a named cause

The `Relation` arm SHALL return
`Ok(FamilyOutcome::Refused(FamilyRefusal::FamilyNotNativelyEvaluable))`. It
calls no `evaluate` hook and charges nothing to the meter.

### A broken invariant is an internal fault

When an S6a invariant breaks, the seam SHALL return `Err(InternalFault)`
naming stage S6a and the violated invariant by a stable identifier. The
case the `Value` family has today is an evaluation environment whose
arguments an earlier call already consumed
(`EvaluateRefusal::EnvironmentAlreadyConsumed`, `src/family/contract.rs`).
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

The `value::expression` family SHALL own a cause type carrying
`WrongSnapshotCause`, with its own exhaustive `catalog_code()` returning
code `wrong_snapshot` and the cause tag of the variant it holds:
`wrong-anchor` for `WrongAnchor`, `forbidden-pre-read` for
`ForbiddenPreRead`. F `diagnostic`'s map SHALL yield `Category::Refusal` for
each of these codes. When evaluation of a `pre(..)` read meets a
`Value::Population` with no attached pre anchor, the caller SHALL receive a
refusal whose catalog code is `wrong_snapshot`/`wrong-anchor`, produced
through that family cause's `catalog_code()`.

### The model-query refusal is carried with its own code

When `allInstances<T>(p)` or `lookup<T>(p, r)` is refused by
`model::population` with a `ModelRefusal`, the caller SHALL receive a
refusal whose catalog code is that `ModelRefusal`'s own code and cause
pair. The carried refusal holds no native-v1 `qsl_foundation::diagnostic::
Code` value (ADR-013 §6 lists that `Code` as lane-private, and T-6 removes
it from the kernel `Refusal`).

### The kernel refusal holds kernel causes only

`quire_exact::Refusal` has no variant that names `WrongSnapshotCause`,
`ModelRefusal`, `FamilyRefusal`, `FamilyNotNativelyEvaluable` or any
`qsl_foundation` type. A caller that matches `FamilyOutcome::Evaluated(
Outcome::Refused(r))` reads a kernel cause.

### Layering

`FamilyOutcome` and `FamilyRefusal` are defined once, in the layer-3 `check`
core. Under ADR-011 §6.1's "Depends on" column, the modules that may name
them are: the layer-3 `check` core and the family checker modules above it
in layer 3; layer 4 `package`; layer 5 `value::expression` and its family
evaluators and `simulation`; layer R `route`; and layer 6 `replay`,
`command`, `cli` and `main`. K (`quire-exact`), F (`qsl-foundation`),
layer 1, layer 2 `forms`, and the layer-3 modules ordered below the `check`
core (`semantic_value`, `model`, `library`) name neither type.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-090-AC-1 | Given a checked `Value` function and an evaluation that the kernel completes, leaves undefined (a zero divisor), refuses with a kernel cause (an integer result outside its declared domain) or leaves incomplete (a meter too tight for the call's first charge), S6a returns `Ok(FamilyOutcome::Evaluated(o))` in each case, where `o` is a `quire_exact::Outcome` equal in variant and payload to the outcome the family's `evaluate` hook produced; none of the four is returned as `FamilyOutcome::Refused` or as `Err`. | Test (TC-382) |
| FR-090-AC-2 | Given a `Relation` declaration and a meter, S6a returns `Ok(FamilyOutcome::Refused(FamilyRefusal::FamilyNotNativelyEvaluable))`, and the meter's recorded usage after the call equals its usage before it. | Test (TC-383) |
| FR-090-AC-3 | Given a `Value` evaluation environment whose arguments an earlier S6a call already consumed, a second S6a call on it returns `Err(fault)` without panicking, where `fault.stage()` names S6a, `fault.invariant()` is a stable identifier for the consumed-environment invariant, `fault.category()` is `Category::InternalFailure` and `fault.catalog_code()` is `runtime_invariant`/`established-invariant-broken`; the result is neither `Ok(FamilyOutcome::Refused(_))` nor `Ok(FamilyOutcome::Evaluated(Outcome::Refused(_)))`. | Test (TC-384) |
| FR-090-AC-4 | For every `FamilyRefusal` variant, `catalog_code()` returns a code and cause defined by the `quire.native.diagnostics/v1` revision FR-322 selects; the method matches every variant with no `_` arm, so adding a variant without an arm fails to compile. | Test (TC-385) |
| FR-090-AC-5 | F `diagnostic`'s catalog-code-to-category map returns `Category::Refusal` for the code of every `FamilyRefusal` variant and for every code the `value::expression` family cause's `catalog_code()` returns; `qsl-foundation` declares no dependency through which it could name `FamilyRefusal`. | Test (TC-386) |
| FR-090-AC-6 | The `value::expression` family cause carrying `WrongSnapshotCause` has an exhaustive `catalog_code()` with no `_` arm that returns `wrong_snapshot`/`wrong-anchor` for `WrongAnchor` and `wrong_snapshot`/`forbidden-pre-read` for `ForbiddenPreRead`. | Test (TC-387) |
| FR-090-AC-7 | Given a postcondition `pre(allInstances<T>(p))` evaluated with a population argument admitted through `admit_binding` (no pre anchor), the caller receives a refusal whose catalog code is `wrong_snapshot`/`wrong-anchor`; the result is not a panic and not `FamilyOutcome::Evaluated(Outcome::Refused(_))`, and `quire_exact::Refusal` has no variant naming `WrongSnapshotCause`. | Test (TC-388) |
| FR-090-AC-8 | Given an `allInstances<T>(p)` query whose selected member count is above the population's declared maximum, the caller receives a refusal whose catalog code is `cardinality_out_of_bound`/`above-maximum`, equal to the refusing `ModelRefusal`'s code and cause; the result is not a panic and not `FamilyOutcome::Evaluated(Outcome::Refused(_))`, and the carried refusal holds no `qsl_foundation::diagnostic::Code` value. | Test (TC-389) |
| FR-090-AC-9 | `FamilyOutcome` and `FamilyRefusal` are each defined once, in the layer-3 `check` core. No `use` edge or inline path under `src/forms/`, `src/model/` or `src/library/` resolves to either type, and neither `quire-exact/Cargo.toml` nor `qsl-foundation/Cargo.toml` declares a dependency on the crate that defines them. | Test (TC-390) |

## Dependencies

- **Upstream:** [FR-062](FR-062-implement-checked-family-contract.md) owns
  `FamilyContract`, `ReferenceEvaluation` and `FamilyKind`, which the S6a
  seam dispatches over. [FR-068](FR-068-split-expression-checking-into-check-stage.md)
  placed `WrongSnapshotCause` in `crate::check` (FR-068-AC-4, AC-8).
  F `diagnostic`'s `CatalogCode`, `Category` and `InternalFault` landed with
  ADR-013 §7 S-5a.
- **Downstream:** QSL-131's removal of `src/value/outcome.rs`'s kernel copy
  in favour of `quire_exact::{Outcome, Refusal}` needs FR-090-AC-7 and
  FR-090-AC-8, because that copy's `WrongSnapshot` and `Model` variants are
  the two `quire_exact::Refusal` does not have. In the other direction,
  FR-090-AC-1's `Evaluated` payload is `quire_exact::Outcome<T>`, which the
  `Value` evaluator produces once that copy is gone. The two changes are
  coupled in both directions.
- ADR-012 §14.1 lists "`Relation`'s non-native-evaluability arm" under
  QSL-152. FR-090-AC-2 specifies that arm's behaviour; which ticket builds it
  is a ticketing question, not a design one.
- ADR-013 T-4 also decides `CheckedPackage::call`'s
  `Result<FamilyOutcome, CallFailure>` with `CallFailure { Input(InputRefusal),
  Fault(InternalFault) }`. This requirement specifies the S6a result that
  `call` carries; it does not specify `call`'s admission step.

## Status

Specified under QSL-174 (ADR-013 O-16, O-17, T-4, T-6). Not yet implemented.
Measured at `origin/main` `8b28023c`: `FamilyOutcome`, `FamilyRefusal` and
`FamilyNotNativelyEvaluable` appear under `src/` only in doc comments; the
provisional stand-in is `EvaluateRefusal` (`src/family/contract.rs:411`);
`CheckedPackage::call` (`src/value/expression/mod.rs:254`) returns
`Result<Evaluation, InputRefusal>` and ends its consumed-environment arm in
`unreachable!`; and F `diagnostic` has no catalog-code-to-category map yet.
`quire_exact::Meter::charge` is public (QSL-153 and QSL-166 are Done), so
FR-090-AC-1's `Incomplete` case is constructible.

FR-090-AC-4 and FR-090-AC-5 for `FamilyNotNativelyEvaluable` wait on
FR-090-OQ-2. The carrier of FR-090-AC-7 and FR-090-AC-8 waits on
FR-090-OQ-1.

## Open Questions

- **FR-090-OQ-1: Which S6a carrier holds an evaluation-time family cause?**
  ADR-013 T-6 makes `WrongSnapshotCause` "a `value::expression` family cause
  mapped through its own `catalog_code()`", and the kernel `Refusal` loses it.
  O-16 fixes `FamilyOutcome` at two arms, and its `Evaluated` arm holds only
  the kernel outcome. The remaining arm, `Refused(FamilyRefusal)`, is
  described in O-16, T-6 and ADR-012 §2 as carrying *family-dispatch* causes
  for a family that sits out a stage. O-17 adds that "no shared enum lists
  every family's causes". ADR-011 §6.1 places `FamilyRefusal` in the layer-3
  `check` core, which cannot name a type defined in layer-5
  `value::expression` or in a family module ("inside each layer the order is
  core, then families"). The ADRs therefore do not say whether the
  evaluation-time snapshot cause reaches the caller:
  (a) as a `FamilyRefusal` variant, which needs the cause type defined in the
  `check` core rather than in `value::expression`;
  (b) as an argument-admission refusal in T-4's `CallFailure::Input`, before
  S6a; or
  (c) through another carrier.
  The same question applies to the model-query refusal. It also applies to
  three more non-kernel results measured in `src/value/outcome.rs` that the
  ticket does not name: `Refusal::UnresolvedPopulation`,
  `Refusal::PopulationMaximumMismatch` and
  `Undefined::PreconditionFalse`. None of the three is a `quire_exact`
  variant, and `FamilyOutcome` has no non-kernel `Undefined` arm.
  FR-090-AC-7 and FR-090-AC-8 state the behaviour a caller observes under
  any of these carriers. The owner's ruling decides where the cause type is
  defined and how it travels.
- **FR-090-OQ-2: No catalog code exists for `FamilyNotNativelyEvaluable`.**
  ADR-013 O-16 requires `FamilyRefusal::catalog_code()`, and O-17 requires
  the code to come from the catalog revision FR-322 selects
  (`quire.native.diagnostics/v1` `1-draft.6`). O-17 also forbids a
  conversion that invents a code. That revision's code table has no code for
  a family that does not evaluate natively. A catalog entry is needed from
  agent-ix/quire-specification, in the way ADR-013 QC-11 obtained
  `stage_limit_exceeded`.
- **FR-090-OQ-3: Where do an evaluation's location and loss records travel?**
  O-16 and ADR-011 §2.3 put the kernel `Outcome<T>` unchanged in
  `FamilyOutcome::Evaluated`. Today S6a returns `Evaluation { outcome,
  location, losses }` (`src/value/expression/evaluate.rs:50`), and ADR-011
  §2.1's E6 row names the output "`Evaluation` / `FamilyOutcome`". The ADRs
  do not say whether `Evaluated` wraps `Outcome<T>` alone, with location and
  losses carried beside the `FamilyOutcome`, or wraps an `Evaluation`.
- **FR-090-OQ-4: What do the S6a arms of the four unmigrated families
  return?** ADR-012 evaluates `StateModel`, `SumCase`, `TemporalTrace` and
  `ProtocolClause` natively, but none of them implements
  `ReferenceEvaluation` yet (`src/family/mod.rs`). FR-090 requires an S6a
  seam with one arm per `FamilyKind` and no `_` arm.
  `FamilyNotNativelyEvaluable` is wrong for these four, because they are not
  families that sit out evaluation. The ADRs name no result for a natively
  evaluable family whose `evaluate` hook has not landed.
