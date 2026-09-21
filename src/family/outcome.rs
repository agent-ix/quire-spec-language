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
pub(crate) struct Staged<T> {
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
/// **`NestingDepth` only.** ADR-013 T-4 names four limit kinds ("input
/// bytes, nesting depth, node count, work budget"), but `Value`'s
/// function-declaration `check` -- #214's one migrated stage entry --
/// charges and checks only nesting depth (FR-062-AC-7); nothing in this
/// ticket's real scope ever reaches an input-bytes, node-count or
/// work-budget limit, so those three variants would be exactly the same
/// unconstructed shape `Requirements`'s substructure was. FR-062-AC-5 names
/// all four limit kinds and is unbacked for exactly this narrowing; QSL-153
/// owns adding the other three variants back, together with a real
/// producer for each (`FR-062`'s own Status section).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum StageLimitKind {
    NestingDepth,
}

/// A reached stage limit (ADR-013 T-4), distinct in type from a refusal, a
/// checked node and `Incomplete` (FR-062-AC-5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LimitExceeded {
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
/// `mint_declaration_identity` a second time and compared it against the
/// first, returning `Fault` on a mismatch. That comparison can never fail
/// -- `mint_declaration_identity` is a pure function of its own arguments,
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
/// **No `Refused` variant.** ADR-013 T-4's own design also names a
/// family-typed-cause refusal (`Refused { causes: Vec<C> }`), and ADR-012
/// §5.1 S4 separately requires each family's own `Cause` enum and its
/// `catalog_code()` mapping to exist as a closed seam -- but `Value`'s
/// function-declaration/application family has no real typed refusal cause
/// distinct from its existing checking refusals (`CheckCause`), so #214
/// does not add a `Cause` enum for it (a `DeclarationCause` with zero real
/// variants was tried and deleted: a probe over it would test only its own
/// `catalog_code()` mapping, not a seam). Nothing in this ticket's one
/// migrated family ever produces a typed refusal cause through this
/// outcome type: `Value`'s function-declaration/application
/// `check` mints identity unconditionally past its one limit check, so nothing
/// here constructs a `Refused`. Worse, `Vec<C>` over an uninhabited `C` is
/// still constructible *empty* -- `Refused { causes: Vec::new() }` compiles
/// and asserts nothing, a causeless refusal. Rather than ship a variant only
/// reachable via that hazard, `#214` narrows `StageFailure` to the one way
/// this ticket's family can genuinely fail; QSL-152 owns adding `Refused`
/// back, parameterised over a real, inhabited `Cause` type, alongside the
/// rest of the contract's deferred parts (FR-062-AC-1/AC-4/AC-6/AC-8/AC-9).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum StageFailure {
    Limit(LimitExceeded),
}

/// ADR-013 T-4's `Result<Staged<T>, StageFailure>`: every S1-S4 stage hook's
/// return shape (FR-062 "structured outcome"), narrowed per
/// [`StageFailure`]'s own doc.
pub(crate) type CheckOutcome<T> = Result<Staged<T>, StageFailure>;
