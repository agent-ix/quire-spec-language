// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 O-16 (QSL-174): the S6a evaluation-time result shapes --
//! [`FamilyResult`], [`EvalOutcome`] and [`FamilyOutcome`] -- distinct from
//! [`super::outcome`]'s S1-S4 `CheckOutcome`/`StageFailure`.

use qsl_foundation::diagnostic::{CatalogCoded, UndefinedCoded};

/// ADR-013 O-16: a family's own evaluation-time result. `Refused`, category
/// `refusal`, holds a family-owned refusal cause; `Undefined`, category
/// `undefined`, holds a family-owned undefined cause. Both are trait
/// objects, not a shared enum: O-17 rules that each family owns its own
/// `Cause` enum and that no shared enum lists every family's causes (FR-090
/// Behavior, "A family's evaluation-time result is family-owned"). The
/// `check` core names no family cause type: it holds each cause only
/// through [`CatalogCoded`]/[`UndefinedCoded`].
///
/// `Debug` because `CatalogCoded`/`UndefinedCoded` both carry it as a
/// supertrait (ADR-013 O-16 "Representation"); no `Clone`, `PartialEq` or
/// `Eq`: a consumer matches the arm with `matches!` and compares
/// `catalog_code()`/`undefined_record()`; an assertion on the concrete
/// cause type lives in the producing family's own unit tests.
#[derive(Debug)]
pub enum FamilyResult {
    /// Category `refusal`.
    Refused(Box<dyn CatalogCoded>),
    /// Category `undefined`.
    Undefined(Box<dyn UndefinedCoded>),
}

/// ADR-012 §2 / ADR-013 O-16: what a family's `evaluate` hook returns inside
/// `Ok`. `Err` is reserved for an S6a invariant break
/// ([`qsl_foundation::diagnostic::InternalFault`]): refusal and undefined
/// are outcome categories, so a hook never returns them in `Err` (FR-090
/// Behavior).
#[derive(Debug)]
pub(crate) enum EvalOutcome<T> {
    /// The kernel evaluation outcome, unchanged.
    Kernel(quire_exact::Outcome<T>),
    /// A family-owned evaluation-time refusal or undefined result.
    Family(FamilyResult),
}

/// ADR-013 O-16's S6a seam result.
///
/// **Only two of the three design-level arms exist here, by ruling
/// (FR-090-OQ-2, option C).** `Evaluated`, from a family's own `evaluate`
/// hook (`EvalOutcome::Kernel`); `FamilyEvaluated`, from the same hook's
/// `EvalOutcome::Family`. The third design-level arm, `Refused(FamilyRefusal)`
/// -- "the family does not run here", ADR-013 O-16's rejected-alternatives
/// discussion -- is unrepresentable, not merely unbuilt: a `Relation`
/// declaration never enters S6a's input type at all (`Value` is the only
/// family with a real `FamilyContract`/`ReferenceEvaluation` implementation,
/// #214; `Relation` has none), so nothing ever produces
/// `FamilyNotNativelyEvaluable`, and `FamilyRefusal` has no other variant to
/// give it a reason to exist. Building `FamilyRefusal` and this arm with no
/// real construction site would repeat the forward-declared-shape hazard
/// `crate::family`'s own module doc already removed once
/// (`FamilyContract::package`/`requirements`). The ticket that gives
/// `Relation` a real checker (QSL-152) adds `FamilyRefusal` and this arm
/// together, once `Relation` is a real S6a input and `FamilyNotNativelyEvaluable`
/// has something to refuse.
#[derive(Debug)]
pub enum FamilyOutcome<T> {
    /// The kernel evaluation outcome, unchanged.
    Evaluated(quire_exact::Outcome<T>),
    /// The family ran and produced its own evaluation-time result.
    FamilyEvaluated(FamilyResult),
}
