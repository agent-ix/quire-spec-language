// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S6a family contract (ADR-012 §2, FR-090-AC-4): the
//! `ReferenceEvaluation` hook every evaluating family implements and the
//! closed [`S6aFamilyKind`] the S6a seam (`evaluate_declaration`, in this
//! module's parent) dispatches over.
//!
//! Both are layer 5 (ADR-011 §6.2 `family` row, amended by QSL-181): the hook
//! is an S6a hook, and its implementation needs the evaluator's own
//! environment. `ReferenceEvaluation: FamilyContract` still makes one marker
//! type per family implement both halves (ADR-012 §2). With the trait here,
//! the `impl ReferenceEvaluation for` a layer-3 marker type
//! (`crate::check::ValueFunctionFamily`) sits in the trait's own crate once
//! `check` is `qsl-semantics`, which the orphan rule (E0117) requires.

use crate::family::{EvalOutcome, FamilyContract, FamilyKind};
use qsl_foundation::diagnostic::InternalFault;
use quire_exact::Meter;

/// ADR-012 §2's `ReferenceEvaluation`: the `evaluate` hook every family
/// implements except `Relation` (which has no native evaluation: S6a's input
/// type admits no `Relation`, ADR-012 §2 and FR-090-AC-4, so no S6a arm or
/// refusal exists for it).
///
/// `Env` is a GAT (`type Env<'a>`), not a plain associated type: a family's
/// real evaluation environment (for `Value`, the checked package it
/// resolves `checked`'s identity against, the caller's object environment
/// and its own accounting meter) is borrowed for the one call, not owned by
/// the family marker type.
pub(crate) trait ReferenceEvaluation: FamilyContract {
    /// The observed evaluation result (a kernel value, a state observation
    /// or a trace verdict, depending on the family).
    type Observed;
    /// The evaluation environment every family's `evaluate` reads.
    type Env<'a>;
    /// What a runtime caller actually has in hand to look `evaluate` up by
    /// (PR #303 review, finding N3): always a bare identity at call sites
    /// like `CheckedPackage::call`, which only ever stores the minted
    /// identity a checked declaration resolved to, not the full
    /// `Self::Checked` payload `check` produced it alongside. Distinct from
    /// `Self::Checked` on purpose -- QSL-148 makes `Checked` a richer struct
    /// (the minted identity together with the real checked body, so `check`
    /// can return both through its ordinary `Ok` rather than a side
    /// channel); `evaluate` still only ever needs the identity half, so it
    /// keeps its own narrower type instead of forcing every caller to carry
    /// a full checked payload just to look up an evaluation.
    type Key;

    /// Evaluate `checked` under `env` and `meter`. Returns
    /// `Ok(EvalOutcome::Kernel(o))` for the kernel evaluation outcome
    /// unchanged -- ADR-012 §2 reserves this hook alone for returning a
    /// meter-budget `Incomplete` outcome (`check` never does, FR-062-AC-5),
    /// which travels here as `EvalOutcome::Kernel(Outcome::Incomplete(_))` --
    /// `Ok(EvalOutcome::Family(r))` for a family-owned evaluation-time
    /// refusal or undefined result (ADR-013 O-16), or `Err(InternalFault)`
    /// for a broken S6a invariant (FR-090-AC-3), never a `FamilyResult` or
    /// a kernel-shaped refusal.
    fn evaluate<'a>(
        checked: &Self::Key,
        env: &mut Self::Env<'a>,
        meter: &mut Meter,
    ) -> Result<EvalOutcome<Self::Observed>, InternalFault>;
}

/// Declares [`S6aFamilyKind`] and its [`S6aFamilyKind::ALL`] from one
/// variant list, so `ALL` cannot miss a variant.
macro_rules! s6a_family_kinds {
    ($($(#[$doc:meta])* $variant:ident),+ $(,)?) => {
        /// The S6a family kind (FR-090-AC-4, ADR-012 §5.1 S1): the closed
        /// family set the S6a seam dispatches over. It has one variant per
        /// family that implements `ReferenceEvaluation`, and never a
        /// `Relation` variant, so no S6a call can name a `Relation`
        /// declaration (ADR-013 O-16). A family gains its variant, its
        /// `S6aFamilyKind::family` arm and its seam arm in the change that
        /// implements its `ReferenceEvaluation` hook. `#[cfg(seam_probe)]`
        /// adds one probe-only variant (FR-063) that no non-probe code
        /// constructs or matches.
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub(crate) enum S6aFamilyKind {
            $($(#[$doc])* $variant,)+
            /// FR-063: exists only so `--cfg seam_probe` makes every
            /// `match` over `S6aFamilyKind` non-exhaustive. Never
            /// constructed outside the probe build.
            #[cfg(seam_probe)]
            __SeamProbe,
        }

        impl S6aFamilyKind {
            /// Every S6a family kind, in declaration order.
            pub(crate) const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];
        }
    };
}

s6a_family_kinds! {
    /// `Value`, through `ValueFunctionFamily`'s `evaluate` hook.
    Value,
}

impl S6aFamilyKind {
    /// The [`FamilyKind`] this S6a family kind evaluates. The compile-time
    /// check below rejects a mapping to `FamilyKind::Relation` and two S6a
    /// kinds mapping to one family.
    ///
    /// FR-063 seam: adding an `S6aFamilyKind` variant with no arm here fails
    /// `--cfg seam_probe` with `E0004`.
    pub(crate) const fn family(self) -> FamilyKind {
        match self {
            Self::Value => FamilyKind::Value,
            // FR-063: no arm for `Self::__SeamProbe` -- under
            // `--cfg seam_probe` this match is deliberately non-exhaustive
            // (`E0004`). Do not add a catch-all to make it compile.
        }
    }
}

/// FR-090-AC-4 checked at compile time, in every build: no S6a family kind
/// maps to `FamilyKind::Relation`, and no two map to one family, so a
/// `Relation` variant cannot be added by routing it through another
/// family's arm.
const _: () = {
    let kinds = S6aFamilyKind::ALL;
    let mut i = 0;
    while i < kinds.len() {
        assert!(
            !matches!(kinds[i].family(), FamilyKind::Relation),
            "an S6a family kind maps to FamilyKind::Relation"
        );
        let mut j = i + 1;
        while j < kinds.len() {
            assert!(
                kinds[i].family() as u8 != kinds[j].family() as u8,
                "two S6a family kinds map to one FamilyKind"
            );
            j += 1;
        }
        i += 1;
    }
};
