// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-029 PL-7 (FR-289-AC-4 to FR-289-AC-6): the terminal arms of a process
//! provider. The driver reads a plugin's FR-331 result with the typed reader
//! and settles it here: a counterexample through S6a replay, a `proved` as a
//! `trusted` proof, and a result that claims a QSL-only cause as a tool
//! failure. The pre-replay counterexample is the driver's wire record, not a
//! QSL type; this module sees only its replay.

use super::{
    Certification, InconclusiveCause, ProofBasis, TerminalRecord, TerminalValue,
    REPLAY_PARITY_KIND, REPLAY_REFUSED_KIND,
};
use crate::execute::ReplayRefusal;
use crate::outcome::OutcomeDiagnostic;
use crate::result::{InputSettlement, ReplayResult};
use qsl_foundation::diagnostic::Category;
use qsl_foundation::{BackendId, RequestIndex};

/// A cause settled by QSL's own replay or Kani mapping, never by a provider
/// (ADR-029 PL-7). The wire spells only the kind; the typed payload exists
/// on the QSL side alone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QslOnlyCause {
    /// `replay-parity`.
    ReplayParity,
    /// `replay-refused`.
    ReplayRefused,
    /// `kani_vacuous_proof`, spelled as the plugin wire spells it.
    KaniVacuousProof,
}

/// The plugin wire's spelling of the vacuity cause. QSpec FR-331 keeps the
/// AD-016 underscore spelling for it (`kani_vacuous_proof` is outside the
/// hyphenated cause table), whereas QSL's outcome document writes
/// `kani-vacuous-proof` ([`super::KANI_VACUOUS_PROOF_KIND`]). The hyphenated
/// spelling is no cause FR-331 lists for a plugin, so it is an ordinary
/// unknown kind here.
const PLUGIN_WIRE_KANI_VACUOUS_PROOF: &str = "kani_vacuous_proof";

impl QslOnlyCause {
    /// The cause kind's spelling on the plugin wire (QSpec FR-331).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReplayParity => REPLAY_PARITY_KIND,
            Self::ReplayRefused => REPLAY_REFUSED_KIND,
            Self::KaniVacuousProof => PLUGIN_WIRE_KANI_VACUOUS_PROOF,
        }
    }
}

/// A plugin result QSL refuses; the item settles `failed` (ADR-029 PL-7).
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PluginResultRefusal {
    /// The result claims a cause only QSL settles.
    #[error("a plugin result claims the QSL-only cause {}", .0.as_str())]
    QslOnlyCause(QslOnlyCause),
    /// The `proved` result reports a basis only the core establishes: the
    /// Kani check count (`bounded-proof`), a core-recomputed closed form or
    /// a replayed witness. `exact-value`, `value-bounds` and `zone-certified`
    /// are core-checked too but have no `ProofBasis` variant yet; one added
    /// later is refused here.
    #[error("a plugin proof reports a basis only the core establishes")]
    CoreOnlyBasis,
    /// A `refuted` result with nothing to replay as canonical assignments.
    #[error("a plugin `refuted` result has no counterexample to replay")]
    MissingCounterexample,
}

impl PluginResultRefusal {
    /// The outcome-document diagnostic for this refusal (ADR-029 PL-7,
    /// "Refused plugin results"): `invalid_result`/`unknown-cause` for a
    /// QSL-only cause (member `cause`) or a core-only basis (member `basis`),
    /// `invalid_result`/`missing-member` for a `refuted` with nothing to
    /// replay (member `counterexample`). `item_index` is the result item's
    /// index and `received` the spelling the plugin sent, which the driver
    /// holds as wire text.
    pub fn diagnostic(&self, item_index: usize, received: &str) -> OutcomeDiagnostic {
        let (cause, member) = match self {
            Self::QslOnlyCause(_) => ("unknown-cause", "cause"),
            Self::CoreOnlyBasis => ("unknown-cause", "basis"),
            Self::MissingCounterexample => ("missing-member", "counterexample"),
        };
        OutcomeDiagnostic::new(
            Some(cause),
            "invalid_result",
            None,
            format!("result item {item_index}: member `{member}`, received `{received}`"),
        )
    }

    /// The item's terminal record: `failed`, naming the plugin. The record
    /// carries no diagnostic; the driver carries the refusal in the outcome
    /// document (`OutcomeDiagnostic::new`).
    pub fn record(self, request_index: RequestIndex, plugin: BackendId) -> TerminalRecord {
        TerminalRecord::failed(request_index, plugin)
    }
}

/// Read the cause kind a plugin's `inconclusive` result names: a QSL-only
/// kind is refused, any other kind is the caller's to read.
#[qsl_attrs::string_edge]
pub fn read_plugin_cause_kind(kind: &str) -> Result<(), PluginResultRefusal> {
    let claimed = match kind {
        REPLAY_PARITY_KIND => QslOnlyCause::ReplayParity,
        REPLAY_REFUSED_KIND => QslOnlyCause::ReplayRefused,
        PLUGIN_WIRE_KANI_VACUOUS_PROOF => QslOnlyCause::KaniVacuousProof,
        _ => return Ok(()),
    };
    Err(PluginResultRefusal::QslOnlyCause(claimed))
}

impl TerminalRecord {
    /// A plugin counterexample settled by its S6a replay as canonical
    /// assignments (`ReplaySource::Input`). A reproduced violation settles
    /// `refuted`, naming `plugin` and keeping the replay result; a replay
    /// that did not reproduce it settles `inconclusive` with `replay-parity`;
    /// a refusal settles through [`TerminalValue::from_replay_refusal`]. A
    /// witness-arm result is not a plugin counterexample's replay and
    /// settles `failed`.
    pub fn from_plugin_counterexample(
        request_index: RequestIndex,
        plugin: BackendId,
        replay: &Result<ReplayResult, ReplayRefusal>,
    ) -> Self {
        let (value, evidence) = match replay {
            Ok(ReplayResult::Input(arm))
                if arm.settlement() == InputSettlement::ReproducedWithoutWitness
                    && arm.category() == Category::Violation =>
            {
                (
                    TerminalValue::Refuted,
                    Some(ReplayResult::Input(arm.clone())),
                )
            }
            Ok(ReplayResult::Input(arm)) => (
                match arm.disagreement() {
                    Some(cause) => {
                        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(cause.clone()))
                    }
                    // Reproduced, yet not a violation: nothing was refuted.
                    None => TerminalValue::Failed,
                },
                None,
            ),
            Ok(ReplayResult::Witness(_)) => (TerminalValue::Failed, None),
            Err(refusal) => (TerminalValue::from_replay_refusal(refusal), None),
        };
        let mut record = Self::with_backend(request_index, plugin, value);
        record.replay = evidence;
        record
    }

    /// A plugin's `proved` result: `proved` with the basis the plugin
    /// reports, the certification `trusted` and `plugin`'s identity
    /// (ADR-029 PL-7, RU-4). The Kani check-count basis is refused.
    pub fn from_plugin_proof(
        request_index: RequestIndex,
        plugin: BackendId,
        basis: ProofBasis,
    ) -> Result<Self, PluginResultRefusal> {
        // Exhaustive on purpose: a new `ProofBasis` variant must be decided
        // here. A basis whose certificate or evidence the core checks
        // (ADR-018 §1, ADR-026, ADR-028) is never a plugin's to claim.
        match basis {
            ProofBasis::Checks { .. }
            | ProofBasis::ClosedForm { .. }
            | ProofBasis::Witness { .. } => {
                return Err(PluginResultRefusal::CoreOnlyBasis);
            }
            ProofBasis::Exhaustive
            | ProofBasis::BoundedComplete { .. }
            | ProofBasis::Inductive { .. }
            | ProofBasis::BoundedSolver { .. }
            | ProofBasis::Reduced { .. } => {}
        }
        Ok(Self::with_backend(
            request_index,
            plugin,
            TerminalValue::Proved {
                basis,
                certification: Certification::Trusted,
            },
        ))
    }

    /// A plugin's `inconclusive` result with the typed cause the driver read
    /// from the wire. `replay-parity` and `replay-refused` are QSL's own and
    /// are refused (the cause `kani_vacuous_proof` has no
    /// [`InconclusiveCause`] value; [`read_plugin_cause_kind`] refuses its
    /// spelling).
    pub fn from_plugin_inconclusive(
        request_index: RequestIndex,
        plugin: BackendId,
        cause: InconclusiveCause,
    ) -> Result<Self, PluginResultRefusal> {
        match cause {
            InconclusiveCause::ReplayParity(_) => Err(PluginResultRefusal::QslOnlyCause(
                QslOnlyCause::ReplayParity,
            )),
            InconclusiveCause::ReplayRefused(_) => Err(PluginResultRefusal::QslOnlyCause(
                QslOnlyCause::ReplayRefused,
            )),
            other => Ok(Self::with_backend(
                request_index,
                plugin,
                TerminalValue::Inconclusive(other),
            )),
        }
    }
}
