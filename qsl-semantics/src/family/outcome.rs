// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 T-4's structured stage outcome: `Staged<T>`/`StageFailure<C>`,
//! distinct from the kernel `Outcome<T>`/`Incomplete` `evaluate` alone
//! returns (FR-062-AC-5).
//!
//! **Provisional.** ADR-013 T-4 assigns these types' canonical home to
//! `#213` S-5's `diagnostic` crate. `src/diagnostic.rs` has since landed
//! (merged into this branch from `origin/main`); migrating `Staged`/
//! `StageFailure`/`LimitExceeded` onto it is real work QSL-162 does
//! deliberately, not a byproduct of this one. This module implements T-4's
//! already-decided shape locally so #214 has something to return; see
//! `crate::family`'s module doc.

/// A stage's successful output.
///
/// **No `warnings` field (PR #262 review, coordinator round 3, finding
/// 4).** ADR-013 T-4's design names "output plus any warnings it raised
/// along the way", and an earlier version of this struct carried a
/// `warnings: Vec<String>` for that -- but nothing in #214's one migrated
/// stage entry ever pushes into it (`Staged::new` always constructs it
/// empty) and nothing anywhere reads it back; a field that is always
/// `Vec::new()` on the way in and never inspected on the way out is
/// write-only in both directions, not a real warnings channel. `Staged<T>`
/// itself stays -- ADR-013 T-4's `Result<Staged<T>, StageFailure>` return
/// shape is real and every S1-S4 hook returns it, distinguishing a stage's
/// structured output from a bare `T` -- so QSL-162, which owns adding the
/// field back with a real producer and a real consumer in the same change,
/// does that when a stage entry genuinely raises a warning, not as a hollow
/// shell restored speculatively.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Staged<T> {
    pub(crate) value: T,
}

impl<T> Staged<T> {
    pub(crate) fn new(value: T) -> Self {
        Self { value }
    }
}

/// Which explicit stage-entry limit was reached. Distinct from
/// `quire_exact::accounting::LimitKind`, which bounds value-kernel
/// materialization sizes, not a checking stage's own entry limits.
///
/// **All four ADR-013 T-4 kinds, restored (QSL-153).** `InputBytes`,
/// `NodeCount` and `WorkBudget` were deleted by PR #262 review for having
/// no real producer. `InputBytes`/`NodeCount`'s real producer and consumer
/// are named in `crate::family::contract::StageLimits`'s own doc;
/// `WorkBudget`'s is named there too, but through a denied
/// `crate::family::CheckContext::meter` charge rather than a `StageLimits`
/// field (PR #302 review finding 3). `crate::check::mod::
/// PackageDeclarations::check`'s exhaustive match on this enum (mapping
/// each variant to a `CheckingLimitKind`) is the one production consumer
/// that forces every variant here to be handled, not silently ignored.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum StageLimitKind {
    NestingDepth,
    /// A declaration's own preimage byte length exceeded its configured
    /// limit.
    InputBytes,
    /// A declaration's own visited `Expression` node count exceeded its
    /// configured limit.
    NodeCount,
    /// A declaration's own preimage field-write count exceeded its
    /// configured limit.
    WorkBudget,
}

/// A reached stage limit (ADR-013 T-4), distinct in type from a refusal, a
/// checked node and `Incomplete` (FR-062-AC-5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LimitExceeded {
    pub(crate) kind: StageLimitKind,
    pub(crate) configured_bound: u64,
}

impl LimitExceeded {
    pub(crate) fn new(kind: StageLimitKind, configured_bound: u64) -> Self {
        Self {
            kind,
            configured_bound,
        }
    }
}

/// ADR-013 T-4's `StageFailure`: the ways a stage can fail without
/// producing output, narrowed to the one way this ticket's family can
/// genuinely fail.
///
/// **No `Fault` variant either.** An earlier draft of this type also
/// carried `Fault(InternalFault)` (ADR-013 T-4's internal-invariant
/// category) with one real-looking construction site: `check` recomputed
/// the declaration's identity a second time and compared it against the
/// first, returning `Fault` on a mismatch. That comparison can never fail
/// -- the identity is a pure function of its own arguments,
/// called twice with the same arguments, so the two results are equal by
/// construction, not by anything the runtime checks. That is a fabricated
/// reader, not a real one, so #214 review (PR #262) ruled it out along
/// with `Fault`/`InternalFault` themselves: nothing else in this ticket's
/// one migrated family constructs either. `src/diagnostic.rs`'s own
/// `InternalFault` (landed after this module was written, from `#213` S-5)
/// is the type's real eventual home; a future stage entry that needs a
/// genuine internal-fault outcome reaches for that one, not a revived copy
/// here.
///
/// **`Refused` is generic over the family's own `Cause` (QSL-148).** ADR-013
/// T-4's own design names a family-typed-cause refusal (`Refused { causes:
/// Vec<C> }`), and ADR-012 §5.1 S4 separately requires each family's own
/// `Cause` enum and its `catalog_code()` mapping to exist as a closed seam.
/// #214 shipped `StageFailure` with no `Refused` variant at all, because
/// `Value`'s function-declaration/application family had, at the time, no
/// real typed refusal cause: `check` minted identity unconditionally past
/// its one limit check, so nothing constructed one, and a zero-variant
/// `DeclarationCause` invented just to fill the shape would have been the
/// same fabricated-surface hazard as the deleted `Requirements`.
///
/// QSL-148 gives `Value`'s function family a genuine one:
/// `ValueFunctionFamily::check` now runs the declaration's real typing and
/// definedness pass (`check::family::check_declaration_body`) inside the
/// contract's own `check` entry point, and an ill-typed or undefined body is
/// refused with the crate's real, already-catalogued `check::CheckRefusal`
/// (`FamilyContract::Cause = CheckRefusal` for that family) -- not a shape
/// invented to satisfy this enum. `Refused(C)` carries exactly one cause
/// (not `Vec<C>`): a single `check` call examines one form and produces at
/// most one refusal, so the `Vec` (and the causeless-`Refused{causes: vec![]}`
/// hazard it invited) is not reproduced here. `catalog_code()` for
/// `CheckRefusal` already exists (`CheckCause::code`/`CheckCause::cause`,
/// `src/check/refusal.rs`), so this is not a second, competing mapping --
/// QSL-152's remaining scope is a per-family `Cause` enum for the *other*
/// five families, once they migrate, not this one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StageFailure<C> {
    /// A configured stage limit was reached first.
    Limit(LimitExceeded),
    /// The family refused the form with its own typed cause.
    Refused(C),
}

/// ADR-013 T-4's `Result<Staged<T>, StageFailure<C>>`: every S1-S4 stage
/// hook's return shape (FR-062 "structured outcome"), generic over the
/// family's own refusal cause `C` (see [`StageFailure`]'s own doc).
pub type CheckOutcome<T, C> = Result<Staged<T>, StageFailure<C>>;
