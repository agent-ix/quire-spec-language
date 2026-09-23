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

/// ADR-013 O-16's S6a seam result. Two arms: S6a's input admits no
/// `Relation` (FR-090, O-16). `#[cfg(seam_probe)]` adds one probe-only
/// variant (FR-063) that no non-probe code constructs or matches.
#[derive(Debug)]
pub enum FamilyOutcome<T> {
    /// The kernel evaluation outcome, unchanged.
    Evaluated(quire_exact::Outcome<T>),
    /// The family ran and produced its own evaluation-time result.
    FamilyEvaluated(FamilyResult),
    /// FR-063: exists only so `--cfg seam_probe` makes every `match` over
    /// `FamilyOutcome` non-exhaustive. Never constructed outside the probe
    /// build.
    #[cfg(seam_probe)]
    __SeamProbe,
}

/// The S6a seam's pass-through (FR-090, ADR-012 §2): `Kernel(o)` becomes
/// `Evaluated(o)` and `Family(r)` becomes `FamilyEvaluated(r)`, each
/// unchanged.
impl<T> From<EvalOutcome<T>> for FamilyOutcome<T> {
    fn from(outcome: EvalOutcome<T>) -> Self {
        match outcome {
            EvalOutcome::Kernel(outcome) => Self::Evaluated(outcome),
            EvalOutcome::Family(result) => Self::FamilyEvaluated(result),
        }
    }
}
