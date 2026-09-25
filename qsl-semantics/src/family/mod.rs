// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-062, ADR-012 §2): the shared checked-family contract.
//!
//! Every QSL semantic family (`Value`, `StateModel`, `SumCase`,
//! `TemporalTrace`, `ProtocolClause`, `Relation`; ADR-012 §1) implements
//! `FamilyContract` (and, except `Relation`, `ReferenceEvaluation`)
//! exactly once. This module is the "check core" ADR-012 §13.1 places the
//! contract in.
//!
//! **What this ticket (#214/QSL-25) migrates onto the contract**: `Value`'s
//! function-declaration form only (`value::expression::family`).
//! `FamilyKind`'s other five variants exist here because ADR-012 §1 closes
//! the catalogue over all six names now, so each sibling ticket's
//! `FamilyContract` implementation is added against an already-complete
//! enum, not one it also has to grow; none of the other five has any
//! `FamilyContract` implementation yet, and their forms are unchanged by
//! this ticket.
//!
//! **`stage_hooks`/`Stage`/`HookStatus` are deleted (PR #262 review,
//! finding F7).** An earlier version of this module carried a
//! stage-participation table -- `stage_hooks(FamilyKind, Stage) ->
//! HookStatus`, matching ADR-012 §5.1 S1's "stage-participation table"
//! seam -- plus three `assert_eq!(stage_hooks(...), Implemented)` call
//! sites so the table would have a non-test reader. Every one of those
//! three call sites passed a *literal, compile-time-known* `FamilyKind`
//! and `Stage` into a hand-written `match` and then asserted the result
//! equalled the exact value that same `match` arm already returns for
//! those literals -- an assertion that cannot fail, manufactured only to
//! give otherwise-dead code a caller. Once those three call sites are gone
//! (correctly, per F7), `stage_hooks` itself has no real (non-test) reader
//! left: nothing in this ticket's actual runtime asks "what hook status
//! does family X have at stage Y" to make a real decision (forms are
//! routed by the parser's own separate, S2 closed-enum dispatch, not by a
//! FamilyKind/Stage query). A table nothing reads is the same
//! forward-declared-shape hazard `Requirements` and `Cause` already are in
//! this module, so it is deleted rather than kept `#[allow(dead_code)]` or
//! kept alive by more fabricated callers. This narrows FR-063-AC-6's own
//! checked-in seam-probe list from two S1 locations to one (only
//! `catalog_code_prefix`'s match survives; see `xtask/src/seam_probe.rs`
//! and FR-063's own amended spec text) -- a real, further narrowing this
//! review round produced, not something #214's first pass got right.
//! `Stage`'s own "no `Requirements` stage" doc note is deleted along with
//! `Stage`.
//!
//! **Stage outcome types.** `Staged`/`StageFailure`/`LimitExceeded` live in
//! the foundation `diagnostic` module, ADR-013 T-4's home for them
//! (QSL-160); this module keeps only the `CheckOutcome` alias.
//!
//! **The sixth contract part, `requirements` (ADR-012 §2, FR-062-AC-4).**
//! ADR-014 §11 moved it to QSL-140: [`Requirements`], [`ClaimExtent`] and
//! the `FamilyContract::requirements` method live in [`requirements`].
//! `Value`'s function-declaration form carries no FR-057 capability kind
//! (FR-057: "no kind for an expression nested in a clause, such as a
//! function application"), so its `requirements` returns `None`. The
//! claim families QSL-42 and QSL-43 migrate return one value each.

mod contract;
mod evaluation;
mod outcome;
pub mod requirements;

pub use contract::{CheckContext, FamilyContract};
// Public only under `test-support`: the layer-5 evaluator's tests build a
// `CheckContext` through `check::check_context`; no shipped caller outside
// layer 3 names them (QSL-181).
#[cfg(any(test, feature = "test-support"))]
pub use contract::{Diagnostic, DiagnosticSink, ScopeStack, StageLimits};
#[cfg(not(any(test, feature = "test-support")))]
pub(crate) use contract::{DiagnosticSink, ScopeStack, StageLimits};
pub use evaluation::{EvalOutcome, FamilyOutcome, FamilyResult};
pub use outcome::CheckOutcome;
pub use requirements::{
    classify_extent, ClaimExtent, ClassifyFailure, DomainKind, Requirements, UnboundedDomains,
};

// ADR-013 O-11's `QualifiedName` (the replay executor's typed
// function-selection key, FR-065-AC-6) lives at
// `qsl_eval::value::QualifiedName` today: it is `pub`, because
// `CheckedPackage::call` -- the one entry point that needs it in this
// ticket -- is part of `value::expression`'s own public API, and only
// `Value`'s function family uses it yet. When the layer-6 `replay` facade
// (#243) widens it to other families, its canonical home may move here;
// that migration is out of this ticket's scope.

/// ADR-012 §1's closed family catalogue. `#[cfg(seam_probe)]`
/// adds one probe-only variant (FR-063) that no non-probe code constructs or
/// matches.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FamilyKind {
    /// `Value`: values, expressions and function declarations.
    Value,
    /// `StateModel`: state models.
    StateModel,
    /// `SumCase`: sum-type cases.
    SumCase,
    /// `TemporalTrace`: temporal traces.
    TemporalTrace,
    /// `ProtocolClause`: protocol clauses.
    ProtocolClause,
    /// `Relation`: relations, which have no native evaluation.
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
    ///
    /// `const fn` so the compile-time distinctness check below can call
    /// it at compile time.
    ///
    /// FR-063-AC-7: `#[deny(...)]` closes the escape hatch a `_ =>
    /// unsupported(...)` fallback arm would otherwise open (invisible to the
    /// seam probe alone, since it would compile even under `--cfg
    /// seam_probe`).
    #[deny(clippy::wildcard_enum_match_arm)]
    pub(crate) const fn catalog_code_prefix(self) -> &'static str {
        match self {
            Self::Value => "value",
            Self::StateModel => "state-model",
            Self::SumCase => "sum-case",
            Self::TemporalTrace => "temporal-trace",
            Self::ProtocolClause => "protocol-clause",
            Self::Relation => "relation",
            // FR-063: no arm for `Self::__SeamProbe` under `--cfg
            // seam_probe` alone -- this match is deliberately
            // non-exhaustive (`E0004`) in `xtask seam-probe`'s build of this
            // crate, the seam probe's evidence for the "`FamilyKind` prefix
            // arm of `catalog_code()`" seam (ADR-012 §5.1 S1). Do not add a
            // catch-all to make it compile.
            //
            // The arm below exists only in the probe's build of the crates
            // that depend on this one (`--cfg seam_probe --cfg
            // seam_probe_downstream`): they match over this crate's probe
            // variants (`Capability`, `WrongSnapshotCause`, `FamilyOutcome`),
            // so this crate must compile there for their `E0004`s to be
            // reported at all (QSL-181: one crate's failed build stops every
            // crate above it).
            #[cfg(seam_probe_downstream)]
            Self::__SeamProbe => "__seam_probe__",
        }
    }
}

/// Byte-for-byte equality, `const fn` because `str`/`[u8]` equality is not
/// yet a stable `const` trait impl; used only by
/// the compile-time distinctness check below (`const _`, near the end of this file).
const fn prefixes_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// ADR-012 §5.1 S1's "no two families share a prefix" checked at compile
/// time, in every build, not only under `#[cfg(test)]`.
///
/// **Why this exists, and why it is not `#[allow(dead_code)]`.** Closing
/// `FamilyKind` over all six ADR-012 §1 members now -- before five of them
/// have any `FamilyContract` implementation -- means `catalog_code_prefix`'s
/// five non-`Value` arms have no other real (non-test) reader: nothing yet
/// asks a `StateModel`/`SumCase`/`TemporalTrace`/`ProtocolClause`/`Relation`
/// value what its prefix is, because no sibling family ticket has landed.
/// Silencing that with `#[allow(dead_code)]`, or fabricating a call site
/// that passes each variant in only to assert the literal result its own
/// `match` arm already returns, is exactly the PR #262 F7 pattern this
/// module's own doc rejects elsewhere. This constant is not that: it is a
/// real, always-enforced structural invariant of a *closed* catalogue --
/// distinct prefixes are meaningless to check while the catalogue could
/// still grow, and become a genuine property worth guarding the moment it
/// closes, which is exactly what ADR-012 §1 does now. Evaluating it forces
/// the compiler to construct and compare all six variants' prefixes on
/// every build, catching a collision (for example a sibling ticket copying
/// an existing prefix) at compile time instead of only in
/// `family_prefixes_are_distinct` below, which only runs under `cargo test`.
///
/// Named `_` (the standard `const _: () = { assert!(...) };` idiom, not
/// this module's invention) rather than given a real name: a named `const`
/// nothing references would itself be flagged dead code, and the anonymous
/// form is the documented, ecosystem-standard way (used by, for example,
/// the `static_assertions` crate) to force a `const` body to evaluate for
/// its assertions alone, with no reader needed.
const _: () = {
    let kinds = [
        FamilyKind::Value,
        FamilyKind::StateModel,
        FamilyKind::SumCase,
        FamilyKind::TemporalTrace,
        FamilyKind::ProtocolClause,
        FamilyKind::Relation,
    ];
    let mut i = 0;
    while i < kinds.len() {
        let mut j = i + 1;
        while j < kinds.len() {
            assert!(
                !prefixes_eq(
                    kinds[i].catalog_code_prefix(),
                    kinds[j].catalog_code_prefix()
                ),
                "two FamilyKind variants share a catalog_code_prefix"
            );
            j += 1;
        }
        i += 1;
    }
};

#[cfg(test)]
mod tests {
    use super::*;

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
