// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-358: the composite equality-parity claim of a `bounded_shadow` item --
//! its claim, the falsified and verified evidence CG supplies, and the
//! reports the two entries [`crate::replay_composite_parity`] and
//! [`crate::settle_verified_shadow`] return.
//!
//! The claim says CG's generated shadow computes the language's composite
//! equality (QSpec FR-149) at the claimed node: the verdict under the
//! claimed operator and the occurrence-pair count. It is a claim about
//! generated code, never a property of a spec, so no result here is
//! `Refuted`.

use qsl_foundation::bound::ProofBound;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use quire_exact::{Incomplete, Origin, ScalarLimits};
use quire_semantic_value::declaration::EqualityOperator;

use crate::execute::ReplayRefusal;
use crate::identity::ObligationIdentity;
use crate::proof_result::{IncompleteCause, InconclusiveCause, TerminalValue};
use crate::scalar::{OperandRefusal, ScalarAgreement};
use crate::witness::WitnessValue;

/// One equality evaluation's outcome: the verdict under the claimed
/// operator and the occurrence-pair count, counted with no early exit after
/// an unequal pair (QSpec FR-149).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EqualityOutcome {
    /// The verdict under the claimed operator: `NotEqual` negates `Equal`.
    pub equal: bool,
    /// The occurrence-pair count.
    pub pair_count: u64,
}

/// The driver's descriptor of why a native run stopped, carried verbatim and
/// counted by the proof envelope's encoded-size bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeCause(String);

impl NativeCause {
    /// The driver's descriptor `descriptor`.
    pub fn new(descriptor: String) -> Self {
        Self(descriptor)
    }

    /// The descriptor, as the driver gave it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The driver's observation of the proved generated artifact at the falsifying
/// operands, under the original limits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeParityObservation {
    /// The artifact completed this equality outcome.
    Completed(EqualityOutcome),
    /// The artifact refused with this code.
    Refused(Code),
    /// The artifact's run stopped at a limit. A generated-artifact fault.
    Incomplete(NativeCause),
    /// The artifact's run failed. A generated-artifact fault.
    ExecutionFault(NativeCause),
}

/// What the native refinement run (CG FR-028) found about the shadow against
/// the production code. CG's `bounded_shadow` strengths map onto it with
/// none retired: `shadow_proved_refinement_exhaustive` is `Exhausted`,
/// `_sampled` and `_not_run` are `NotExhausted`, `_inconclusive` is
/// `CeilingReached` and `refinement_failed` is `Disagreed`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refinement {
    /// It compared the whole bounded domain, and they agree.
    Exhausted,
    /// They agree where it compared, but it did not compare the whole
    /// domain, or it did not run.
    NotExhausted,
    /// It stopped at a ceiling before it finished.
    CeilingReached,
    /// It found the shadow and the production code disagree.
    Disagreed,
}

/// A composite equality-parity claim (FR-358): what CG FR-033 builds for a
/// `bounded_shadow` item, field for field.
#[derive(Clone, Debug)]
pub struct CompositeParityClaim {
    /// The claimed equality node, by its node identity in the original
    /// package.
    pub node: WireNodeId,
    /// The claimed occurrence of the node in the selected function's body:
    /// a node shared by two occurrences is two obligations.
    pub occurrence: Origin,
    /// The claimed operation.
    pub operator: EqualityOperator,
    /// The CG `ObligationKind` wire string in the obligation identity's
    /// preimage.
    pub obligation_kind: String,
    /// The bounds the harness ran, each keyed by its domain: a parameter
    /// node id and the child-index path into its type.
    pub harness_bounds: Vec<ProofBound>,
    /// The accounting limits the exact evaluation runs under, retained from
    /// the proving run.
    pub limits: ScalarLimits,
    /// The canonical proof-content identity binding the claim to the proved
    /// artifact. Carried into the claim identity; never recomputed.
    pub content_identity: DigestRecord,
}

/// The falsified item's evidence: the operands CG decoded from the
/// falsifying draw and what CG and the driver observed at them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FalsifiedParity {
    /// The left and right operands, in FR-070's forms.
    pub operands: [WitnessValue; 2],
    /// The retained shadow comparison at those operands.
    pub shadow: EqualityOutcome,
    /// The driver's observation of the same proved generated artifact at
    /// those operands under the original limits.
    pub native: NativeParityObservation,
    /// The refinement evidence of the same harness.
    pub refinement: Refinement,
}

/// The verified item's evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedShadow {
    /// The SUCCESS count of the verified run.
    pub success_checks: u32,
    /// The refinement evidence of the verified harness.
    pub refinement: Refinement,
}

/// The observation a claim was made over: a falsified item's evidence or a
/// verified item's.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositeEvidence {
    /// A falsified item: operands, shadow, native observation, refinement.
    Falsified(Box<FalsifiedParity>),
    /// A verified item: SUCCESS count and refinement.
    Verified(VerifiedShadow),
}

impl CompositeEvidence {
    /// The refinement evidence of the harness.
    pub fn refinement(&self) -> Refinement {
        match self {
            Self::Falsified(falsified) => falsified.refinement,
            Self::Verified(verified) => verified.refinement,
        }
    }
}

/// The identity of the claim a replay settles: everything the request, the
/// claim and the observation fixed. It is built before anything is decoded,
/// so every outcome of the replay carries it in full, a refusal included. A
/// consumer checks `report.claim()` equals [`CompositeIdentity::new`] over
/// what it sent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositeIdentity {
    /// The obligation the claim replays, as the request carried it.
    pub obligation: ObligationIdentity,
    /// The claimed equality node.
    pub node: WireNodeId,
    /// The claimed occurrence of the node.
    pub occurrence: Origin,
    /// The claimed operation.
    pub operator: EqualityOperator,
    /// The CG `ObligationKind` wire string.
    pub obligation_kind: String,
    /// The bounds the harness ran.
    pub harness_bounds: Vec<ProofBound>,
    /// The observation, as supplied: the operands, shadow, native
    /// observation and refinement of a falsified item, or the SUCCESS count
    /// and refinement of a verified one.
    pub evidence: CompositeEvidence,
    /// The proof-content identity of the observation, as supplied.
    pub content_identity: DigestRecord,
    /// The limits the exact evaluation runs under.
    pub limits: ScalarLimits,
}

impl CompositeIdentity {
    /// The identity of `claim` over `evidence`, for the request carrying
    /// `obligation`.
    pub fn new(
        obligation: ObligationIdentity,
        claim: &CompositeParityClaim,
        evidence: CompositeEvidence,
    ) -> Self {
        Self {
            obligation,
            node: claim.node,
            occurrence: claim.occurrence.clone(),
            operator: claim.operator,
            obligation_kind: claim.obligation_kind.clone(),
            harness_bounds: claim.harness_bounds.clone(),
            evidence,
            content_identity: claim.content_identity,
            limits: claim.limits,
        }
    }

    /// Bytes this identity adds to an encoded record beyond its fixed size:
    /// the obligation, node and content digests, the kind, the bounds and
    /// the operands.
    pub(crate) fn measured_bytes(&self) -> usize {
        3 * 32
            + self.occurrence.role().as_str().len()
            + self.obligation_kind.len()
            + self
                .harness_bounds
                .iter()
                .map(|bound| crate::witness::finite_bound_bytes(bound.bound()))
                .sum::<usize>()
            + match &self.evidence {
                CompositeEvidence::Falsified(falsified) => falsified
                    .operands
                    .iter()
                    .map(WitnessValue::value_text_len)
                    .fold(0, usize::saturating_add)
                    .saturating_add(match &falsified.native {
                        NativeParityObservation::Incomplete(cause)
                        | NativeParityObservation::ExecutionFault(cause) => cause.as_str().len(),
                        NativeParityObservation::Completed(_)
                        | NativeParityObservation::Refused(_) => 0,
                    }),
                CompositeEvidence::Verified(_) => 0,
            }
    }
}

/// Which limit stage left a replay incomplete. The report keeps it apart
/// from the terminal value, which is `Incomplete(ResourceExhausted)` for
/// both.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IncompleteStage {
    /// Admitting an operand reached one of the request's accounting limits:
    /// the counter, its configured value and the count reached. No exact
    /// evaluation ran.
    Admission(Box<Incomplete>),
    /// QSL's exact evaluation reached a limit: QSL's counter, its configured
    /// value and the count reached.
    ExactEvaluation(Box<Incomplete>),
    /// The refinement run reached a ceiling before it finished (CG
    /// FR-028-AC-24).
    RefinementCeiling,
}

/// What a falsified-item replay found, one per row F-1 to F-7. None of these
/// is a `Refuted` result.
#[derive(Debug)]
pub enum CompositeParityResult {
    /// Row F-1: the refinement run found the shadow and the production code
    /// disagree. `Failed`, whatever the operands' admission, the native
    /// observation or the comparison would give.
    Disagreed,
    /// Row F-3: an operand failed admission; nothing was evaluated.
    RefusedInput(OperandRefusal),
    /// Row F-2: the native run stopped before it produced an outcome to
    /// compare. `Failed`, a generated-artifact fault, settled before any
    /// admission or exact evaluation.
    GeneratedFault {
        /// The native observation, with the driver's cause.
        native: NativeParityObservation,
    },
    /// Rows F-4, F-5 and F-6: a limit stopped the settlement, at this stage.
    Incomplete {
        /// The stage that stopped.
        stage: IncompleteStage,
    },
    /// Row F-7: the exact outcome and the retained shadow outcome differ in
    /// the verdict or the pair count: a CG defect, `Failed`.
    Diverged {
        /// QSL's exact outcome.
        exact: EqualityOutcome,
        /// The retained shadow outcome.
        shadow: EqualityOutcome,
        /// The accounting charges the evaluation incurred.
        charges: ScalarLimits,
    },
    /// Row F-7: the exact outcome equals the shadow's: the counterexample
    /// does not reproduce, `Inconclusive(ScalarAgrees)`.
    Agrees {
        /// The claim and the outcome both sides reached.
        agreement: ScalarAgreement,
        /// The accounting charges the evaluation incurred.
        charges: ScalarLimits,
    },
    /// A common step refused: the request is not this claim.
    Refused(Box<ReplayRefusal>),
}

impl CompositeParityResult {
    /// The terminal value this result settles for the proof envelope.
    pub fn terminal_value(&self) -> TerminalValue {
        match self {
            Self::Disagreed | Self::GeneratedFault { .. } | Self::Diverged { .. } => {
                TerminalValue::Failed
            }
            Self::RefusedInput(refusal) => {
                TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(refusal.code()))
            }
            Self::Incomplete { .. } => {
                TerminalValue::Incomplete(IncompleteCause::ResourceExhausted)
            }
            Self::Agrees { agreement, .. } => {
                TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(agreement.clone()))
            }
            Self::Refused(refusal) => TerminalValue::from_replay_refusal(refusal),
        }
    }
}

/// What a falsified-item replay settled, with the claim identity carried
/// unchanged on every outcome, a refusal included.
#[derive(Debug)]
pub struct CompositeParityReport {
    claim: CompositeIdentity,
    result: CompositeParityResult,
}

impl CompositeParityReport {
    pub(crate) fn new(claim: CompositeIdentity, result: CompositeParityResult) -> Self {
        Self { claim, result }
    }

    /// The full claim: the identity members and the observation, on every
    /// outcome. A consumer checks `claim()` equals the claim it sent.
    pub fn claim(&self) -> &CompositeIdentity {
        &self.claim
    }

    /// The obligation identity the request carried.
    pub fn obligation(&self) -> ObligationIdentity {
        self.claim.obligation
    }

    /// What the replay found.
    pub fn result(&self) -> &CompositeParityResult {
        &self.result
    }

    /// The terminal value this report settles.
    pub fn terminal_value(&self) -> TerminalValue {
        self.result.terminal_value()
    }
}

/// What a verified-item settlement found, one per row V-1 to V-5, or a
/// refusal. None of these is a `Refuted` result.
#[derive(Debug)]
pub enum VerifiedShadowResult {
    /// Row V-1: the refinement run found the shadow and the production code
    /// disagree: `Failed`.
    Disagreed,
    /// Row V-2: the refinement run reached a ceiling:
    /// `Incomplete(ResourceExhausted)`, with the stage recorded.
    Incomplete {
        /// Always [`IncompleteStage::RefinementCeiling`].
        stage: IncompleteStage,
    },
    /// Row V-3: a proof with no SUCCESS check: vacuous.
    Vacuous,
    /// Row V-4: the refinement run was exhaustive and every derived position
    /// is covered: a proof.
    Proved {
        /// The SUCCESS count of the verified run.
        success_checks: u32,
    },
    /// Row V-5: evidence for the bounds the harness ran, never promoted to a
    /// proof.
    Tested,
    /// A common step refused: the request is not this claim.
    Refused(Box<ReplayRefusal>),
}

impl VerifiedShadowResult {
    /// The terminal value this result settles for the proof envelope.
    pub fn terminal_value(&self) -> TerminalValue {
        match self {
            Self::Disagreed => TerminalValue::Failed,
            Self::Incomplete { .. } => {
                TerminalValue::Incomplete(IncompleteCause::ResourceExhausted)
            }
            Self::Vacuous => TerminalValue::Proved { success_checks: 0 },
            Self::Proved { success_checks } => TerminalValue::Proved {
                success_checks: *success_checks,
            },
            Self::Tested => TerminalValue::Tested,
            Self::Refused(refusal) => TerminalValue::from_replay_refusal(refusal),
        }
    }
}

/// What a verified-item settlement settled, with the claim identity carried
/// unchanged on every outcome, a refusal included.
#[derive(Debug)]
pub struct VerifiedShadowReport {
    claim: CompositeIdentity,
    result: VerifiedShadowResult,
}

impl VerifiedShadowReport {
    pub(crate) fn new(claim: CompositeIdentity, result: VerifiedShadowResult) -> Self {
        Self { claim, result }
    }

    /// The full claim: the identity members and the observation, on every
    /// outcome. A consumer checks `claim()` equals the claim it sent.
    pub fn claim(&self) -> &CompositeIdentity {
        &self.claim
    }

    /// The obligation identity the request carried.
    pub fn obligation(&self) -> ObligationIdentity {
        self.claim.obligation
    }

    /// What the settlement found.
    pub fn result(&self) -> &VerifiedShadowResult {
        &self.result
    }

    /// The terminal value this report settles.
    pub fn terminal_value(&self) -> TerminalValue {
        self.result.terminal_value()
    }
}
