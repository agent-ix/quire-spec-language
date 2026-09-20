// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-062, ADR-012 §2): the shared checked-family contract.
//!
//! Every QSL semantic family (`Value`, `StateModel`, `SumCase`,
//! `TemporalTrace`, `ProtocolClause`, `Relation`; ADR-012 §1) implements
//! [`FamilyContract`] (and, except `Relation`, [`ReferenceEvaluation`])
//! exactly once. This module is the "check core" ADR-012 §13.1 places the
//! contract in: it owns the six shared parts (identity, provenance, checked
//! input, requirements, structured outcome, stage hooks) and no
//! family-specific logic.
//!
//! **What this ticket (#214/QSL-25) migrates onto the contract**: `Value`'s
//! function-declaration and function-application forms only
//! ([`crate::value::expression::family`]). `FamilyKind`'s other five
//! variants exist here because ADR-012 §1 closes the catalogue over all six
//! names now, and [`stage_hooks`] must answer, exhaustively, what every
//! family does at every stage -- but only `Value`'s two migrated forms have
//! a real [`FamilyContract`] implementation today. The other five report
//! [`HookStatus::NotYetMigrated`]: their forms still check through the
//! pre-existing `value::expression`/`model`/`state`/`temporal`/
//! `protocol_artifact` code, unchanged by this ticket, until their own
//! tickets migrate them (ADR-012 §14.1).
//!
//! **Provisional types.** [`outcome`] (`Staged`/`StageFailure`/
//! `LimitExceeded`/`InternalFault`) implements a shape ADR-013 T-4 already
//! decides, but whose canonical Rust home (`#213` S-5's `diagnostic` crate)
//! has not landed as of this ticket. It is built here, to the ADR's own
//! decided shape, because #214 needs it now and nothing else defines it yet.
//! This is not a compatibility layer for a retiring path -- it is the one
//! implementation of an already-decided design that a sibling ticket owns
//! but has not yet delivered. When #213 S-5 lands, QSL's copy is expected to
//! move or be replaced by theirs; that migration is out of this ticket's
//! scope.
//!
//! **The sixth contract part, `Requirements` (ADR-012 §2), is deferred.**
//! No family this repository has migrated -- `Value`'s function-declaration
//! and function-application forms, the only forms #214 migrates -- carries
//! an FR-057 capability kind (owner ruling, #214 review). A `Requirements`
//! type built now would have had exactly the fields `CapabilityKind`,
//! `Extent` and `Bound` supply, and deleting those three (because nothing
//! constructs one yet) leaves `Requirements` with no fields at all: a
//! zero-content type that cannot be told apart from "not implemented" by
//! anything that reads it. Shipping that shell would make a future family
//! with a real capability kind satisfy `FamilyContract` identically whether
//! it wires `requirements()` correctly or not -- the exact failure this
//! contract exists to prevent. So `#214` does not add a `requirements`
//! method to `FamilyContract`, and the FR-062 rows for this part are left
//! unbacked rather than backed by an unexercised shell. The first family
//! ticket with a real FR-057 capability kind adds `requirements()` (and
//! `Requirements`/`CapabilityKind`/`Extent`/`Bound`, or whatever shape that
//! kind actually needs) to the trait then, validated against a real
//! instance instead of guessed in advance.
//!
//! Whether function declaration/application itself carries an FR-057 kind
//! is an open question against `#229` (capability vocabulary), not resolved
//! here: FR-062 (spec/functional/FR-062-implement-checked-family-contract.md
//! :46-48) and ADR-012 §2 (line 213) both say a claim form with no FR-057
//! kind yields no `Requirements` value, but neither names function
//! declaration/application specifically, and no FR-057 claim-form table
//! entry for it exists anywhere in this repository's specs today. The
//! deferral above holds regardless of how that question resolves, because
//! it rests on "no migrated family has a capability kind," not on function
//! forms specifically.

mod contract;
mod outcome;

pub(crate) use contract::{
    CheckContext, DiagnosticSink, EvaluateRefusal, FamilyContract, ReferenceEvaluation, ScopeStack,
    StageLimits,
};
pub(crate) use outcome::{CheckOutcome, InternalFault, StageFailure, Staged};

// ADR-013 O-11's `QualifiedName` (the replay executor's typed
// function-selection key, FR-065-AC-6) lives at
// `crate::value::expression::QualifiedName` today: it is `pub`, because
// `CheckedPackage::call` -- the one entry point that needs it in this
// ticket -- is part of `value::expression`'s own public API, and only
// `Value`'s function family uses it yet. When the layer-6 `replay` facade
// (#243) widens it to other families, its canonical home may move here;
// that migration is out of this ticket's scope.

/// ADR-012 §1's closed family catalogue. `#[cfg(seam_probe)]`
/// adds one probe-only variant (FR-063) that no non-probe code constructs or
/// matches.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum FamilyKind {
    Value,
    StateModel,
    SumCase,
    TemporalTrace,
    ProtocolClause,
    Relation,
    /// FR-063: exists only so `--cfg seam_probe` makes every match
    /// below non-exhaustive. Never constructed outside the probe build.
    #[cfg(seam_probe)]
    __SeamProbe,
}

impl FamilyKind {
    /// The `catalog_code()` family prefix (ADR-012 §5.1 S1: "the `FamilyKind`
    /// prefix arm of `catalog_code()`"). No two families share a prefix.
    ///
    /// FR-063 seam: adding a `FamilyKind` variant with no arm here fails
    /// `--cfg seam_probe` with `E0004`.
    pub(crate) fn catalog_code_prefix(self) -> &'static str {
        match self {
            Self::Value => "value",
            Self::StateModel => "state-model",
            Self::SumCase => "sum-case",
            Self::TemporalTrace => "temporal-trace",
            Self::ProtocolClause => "protocol-clause",
            Self::Relation => "relation",
            // FR-063: no arm for `Self::__SeamProbe` -- under
            // `--cfg seam_probe` this match is deliberately
            // non-exhaustive (`E0004`), the seam probe's evidence for the
            // "`FamilyKind` prefix arm of `catalog_code()`" seam
            // (ADR-012 §5.1 S1). Do not add a catch-all to make it compile.
        }
    }

    /// Every non-probe member of ADR-012 §1's closed catalogue, in
    /// declaration order. [`crate::value::expression::PackageDeclarations::
    /// check`]'s real (non-test) call to
    /// `assert_distinct_catalog_code_prefixes` is this array's one
    /// production consumer: five of the six members have no `FamilyContract`
    /// implementation yet (this ticket migrates only `Value`), so nothing
    /// else constructs them outside a test -- but the catalogue itself is
    /// ADR-012 §1's own closed, forward-declared design (all six members
    /// exist now so each sibling ticket's `FamilyContract` implementation is
    /// added against an already-complete enum, not one it also has to grow),
    /// not speculative shape #214 invented.
    pub(crate) const fn all() -> [FamilyKind; 6] {
        [
            Self::Value,
            Self::StateModel,
            Self::SumCase,
            Self::TemporalTrace,
            Self::ProtocolClause,
            Self::Relation,
        ]
    }
}

/// A real (non-test) invariant: no two [`FamilyKind`] members share a
/// `catalog_code_prefix()` (ADR-012 §5.1 S1's own rule, "no two families
/// share a prefix"). Constructs every non-`Value` member for real, which is
/// what keeps [`FamilyKind`]'s five not-yet-migrated members from being
/// merely forward-declared shape with no production reader.
pub(crate) fn assert_distinct_catalog_code_prefixes() {
    let mut prefixes: Vec<&'static str> = FamilyKind::all()
        .iter()
        .map(|kind| kind.catalog_code_prefix())
        .collect();
    prefixes.sort_unstable();
    prefixes.dedup();
    assert_eq!(
        prefixes.len(),
        FamilyKind::all().len(),
        "FamilyKind's catalog-code prefixes must be pairwise distinct"
    );
}

/// One stage of the contract (ADR-012 §8) that [`stage_hooks`] reports
/// [`HookStatus`] for.
///
/// **No `Requirements` stage.** ADR-012 §2 names a `requirements` stage
/// too, but `#214` defers the sixth contract part entirely (no family's
/// `FamilyContract` has a `requirements` method here yet -- see
/// `crate::family`'s module doc), so there is no hook for any family to
/// report a status for at this stage; adding it back is part of the same
/// deferred work `requirements()` itself is.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Stage {
    Check,
    Package,
    Evaluate,
}

/// Whether, and how, a family participates in one [`Stage`] (ADR-012 §5.1
/// S1's stage-participation table; ADR-012 §13.1's answer to "a missing hook
/// is a compile error... a family that sits out a stage has an explicit,
/// hand-written arm").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum HookStatus {
    /// A real [`FamilyContract`]/[`ReferenceEvaluation`] hook exists for at
    /// least one of the family's forms.
    Implemented,
    /// The family's forms still check through their pre-existing,
    /// pre-contract code path; no [`FamilyContract`] implementation exists
    /// for this family yet. Not the same as [`Self::ExplicitlyUnsupported`]:
    /// this family fully participates in the stage, just not through this
    /// contract yet, and its own ticket (ADR-012 §14.1) migrates it.
    NotYetMigrated,
    /// The family declares, permanently, that it does not take part in this
    /// stage (ADR-012 §2: `Relation` at `evaluate`). The hook still has an
    /// explicit arm, returning a named refusal rather than a value.
    ExplicitlyUnsupported,
}

/// The stage-participation table (ADR-012 §5.1 S1, FR-063-AC-6): which hook
/// status each family has at each stage, including the explicit `Relation`
/// evaluation arm (ADR-012 §2).
///
/// FR-063 seam: adding a `FamilyKind` or `Stage` variant with no arm here
/// fails `--cfg seam_probe` with `E0004`.
pub(crate) fn stage_hooks(family: FamilyKind, stage: Stage) -> HookStatus {
    match (family, stage) {
        (FamilyKind::Value, Stage::Check | Stage::Package | Stage::Evaluate) => {
            HookStatus::Implemented
        }
        (
            FamilyKind::StateModel
            | FamilyKind::SumCase
            | FamilyKind::TemporalTrace
            | FamilyKind::ProtocolClause,
            Stage::Check | Stage::Package | Stage::Evaluate,
        ) => HookStatus::NotYetMigrated,
        (FamilyKind::Relation, Stage::Check | Stage::Package) => HookStatus::NotYetMigrated,
        // ADR-012 §2: `Relation` has no native `evaluate` -- its S6a arm
        // returns `FamilyOutcome::Refused(FamilyRefusal::
        // FamilyNotNativelyEvaluable)`. This is the one hand-written
        // "sits out this stage" arm the contract requires, distinct from
        // "not yet migrated": `Relation` never migrates at `evaluate`.
        (FamilyKind::Relation, Stage::Evaluate) => HookStatus::ExplicitlyUnsupported,
        // FR-063: no arm for `FamilyKind::__SeamProbe` here, and no `_`
        // arm -- under `--cfg seam_probe` this match is deliberately
        // non-exhaustive (`E0004`), which is the seam probe's evidence for
        // this seam. Do not add a catch-all to make it compile.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// TC-160 step 9 (FR-062-AC-6): `Relation`'s evaluation-stage status is
    /// the named "sits out" arm, not "not yet migrated" -- the contract
    /// distinguishes a family that never evaluates natively from one that
    /// simply has not migrated yet.
    #[trace("TC-160", "FR-062-AC-6")]
    #[test]
    fn relation_evaluate_is_explicitly_unsupported() {
        assert_eq!(
            stage_hooks(FamilyKind::Relation, Stage::Evaluate),
            HookStatus::ExplicitlyUnsupported
        );
    }

    /// FR-062-AC-1/FR-063-AC-6: `Value` is the one family with a real
    /// contract implementation at every stage in this ticket.
    #[trace("TC-160", "FR-062-AC-1")]
    #[test]
    fn value_is_implemented_at_every_stage() {
        for stage in [Stage::Check, Stage::Package, Stage::Evaluate] {
            assert_eq!(
                stage_hooks(FamilyKind::Value, stage),
                HookStatus::Implemented
            );
        }
    }

    /// Every family has a distinct `catalog_code()` prefix (ADR-012 §5.1
    /// S1).
    #[test]
    fn family_prefixes_are_distinct() {
        let families = [
            FamilyKind::Value,
            FamilyKind::StateModel,
            FamilyKind::SumCase,
            FamilyKind::TemporalTrace,
            FamilyKind::ProtocolClause,
            FamilyKind::Relation,
        ];
        let mut prefixes: Vec<&str> = families.iter().map(|f| f.catalog_code_prefix()).collect();
        prefixes.sort_unstable();
        prefixes.dedup();
        assert_eq!(prefixes.len(), families.len());
    }
}
