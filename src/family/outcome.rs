// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 T-4's structured stage outcome: `Staged<T>`/`StageFailure<C>`,
//! distinct from the kernel `Outcome<T>`/`Incomplete` `evaluate` alone
//! returns (FR-062-AC-5).
//!
//! **Provisional.** ADR-013 T-4 assigns these types' canonical home to
//! `#213` S-5's `diagnostic` crate, which has not landed. This module
//! implements T-4's already-decided shape locally so #214 has something to
//! return; see `crate::family`'s module doc.

/// A stage's successful output plus any warnings it raised along the way.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Staged<T> {
    pub(crate) value: T,
    pub(crate) warnings: Vec<String>,
}

impl<T> Staged<T> {
    pub(crate) fn new(value: T) -> Self {
        Self {
            value,
            warnings: Vec::new(),
        }
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
/// unconstructed shape `Requirements`'s substructure was. The first stage
/// entry that genuinely checks one of the other three adds it back.
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

/// A stage's own internal-invariant violation (ADR-013 T-4): never a
/// `Refusal`, mapped to the O-16 internal-failure category.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InternalFault {
    pub(crate) stage: &'static str,
    pub(crate) invariant: &'static str,
}

impl InternalFault {
    pub(crate) fn new(stage: &'static str, invariant: &'static str) -> Self {
        Self { stage, invariant }
    }
}

/// ADR-013 T-4's `StageFailure`: a reached limit or an internal fault -- the
/// ways a stage can fail without producing output, for a family with no
/// typed refusal cause exercised yet.
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
/// reachable via that hazard, `#214` narrows `StageFailure` to the two ways
/// this ticket's family can genuinely fail; the first family with a real
/// typed refusal cause adds `Refused` back, parameterised over its own
/// (inhabited) `Cause` type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum StageFailure {
    Limit(LimitExceeded),
    Fault(InternalFault),
}

/// ADR-013 T-4's `Result<Staged<T>, StageFailure>`: every S1-S4 stage hook's
/// return shape (FR-062 "structured outcome"), narrowed per
/// [`StageFailure`]'s own doc.
pub(crate) type CheckOutcome<T> = Result<Staged<T>, StageFailure>;
