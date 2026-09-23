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
//! **Provisional types.** `outcome` (`Staged`/`StageFailure`/
//! `LimitExceeded`) implements a shape ADR-013 T-4 already decides, but
//! whose canonical Rust home (`#213` S-5's `diagnostic` crate) had not
//! landed when this module was first written. `src/diagnostic.rs` has
//! since landed (merged into this branch from `origin/main`) with its own,
//! real `InternalFault`; this module's own copy of `InternalFault` was
//! deleted in the same review round that removed its one (fabricated)
//! construction site, rather than migrated onto `src/diagnostic.rs`'s --
//! that migration, for `Staged`/`StageFailure`/`LimitExceeded` too, is real
//! work QSL-162 does deliberately, not a byproduct of this one.
//!
//! **The sixth contract part, `Requirements` (ADR-012 §2), is deferred.**
//! No family this repository has migrated -- `Value`'s function-declaration
//! form, the only form #214 migrates -- carries an FR-057 capability kind.
//! FR-057:159-162 states plainly that "no kind for an expression nested in
//! a clause, such as a function application" exists, and FR-057:182-186
//! states "family-body admission is language admission, not a capability
//! kind" -- explicit text, not an inference from absence, and not an open
//! question against #229: function declaration/application has no FR-057
//! kind. A `Requirements` type built now would have had exactly the fields
//! `CapabilityKind`, `Extent` and `Bound` supply, and deleting those three
//! (because nothing constructs one yet) leaves `Requirements` with no
//! fields at all: a zero-content type that cannot be told apart from "not
//! implemented" by anything that reads it. Shipping that shell would make
//! a future family with a real capability kind satisfy `FamilyContract`
//! identically whether it wires `requirements()` correctly or not -- the
//! exact failure this contract exists to prevent. So `#214` does not add a
//! `requirements` method to `FamilyContract`, and the FR-062 rows for this
//! part are left unbacked rather than backed by an unexercised shell.
//! QSL-152 (FR-062-AC-1/AC-4/AC-6/AC-8/AC-9) owns adding `requirements()`
//! (and `Requirements`/`CapabilityKind`/`Extent`/`Bound`, or whatever shape
//! a real FR-057 capability kind actually needs) to the trait, validated
//! against a real instance instead of guessed in advance.

mod contract;
mod evaluation;
mod outcome;

pub use contract::{CheckContext, FamilyContract};
pub(crate) use contract::{DiagnosticSink, ScopeStack, StageLimits};
pub use evaluation::{EvalOutcome, FamilyOutcome, FamilyResult};
pub(crate) use outcome::StageLimitKind;
pub use outcome::{CheckOutcome, LimitExceeded, StageFailure, Staged};

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
    pub(crate) const fn catalog_code_prefix(self) -> &'static str {
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
