// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EN-1 item over an authored temporal clause, built from bytes
//! (FR-281, QSL-1819): the checked package, the request's subject binding,
//! the clause's identity and the caller's budgets are everything it needs.
//! No closure, letters or verdict is supplied by the caller: the letters are
//! the production `ClauseLetters` of the admitted `ModelSystem`, and a
//! counterexample replays through `replay_model_trace` over the same source.
//!
//! Owner design: [`ClauseSubject`] holds only borrows of the caller's inputs
//! (`'a`) and the validated clause identity, so the [`AnalyzeItem<'a>`] it
//! makes outlives it. The `ModelSystem` and the `ClauseLetters` borrowing it
//! are built inside the item's work, where they live exactly as long as the
//! run, its settlement and its replay.
//!
//! The replay request names the clause as FR-128 records: a `state_clause`
//! target of the clause's declared name, the clause node's obligation
//! identity, this engine as backend and the canonical [`CLAUSE_TRANSCRIPT`].

use qsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, ModelSystem};
use qsl_eval::simulation::Limits;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::RequestIndex;
use qsl_package::CheckedPackage;
use qsl_replay::{
    replay_model_trace, CertificateRequest, EnvelopeSource, ModelCheckItem, ModelCheckLimits,
    ModelTraceReplayResult, ModelTraceSubject, ObligationIdentity, QualifiedName, ReplayLimits,
    ReplayPackage, ReplayRefusal, ReplayRequestWire, ReplaySource, ReplayTargetMembers,
    RequestTargetWire, StateEnvironment, TemporalCounterexample, Witness, WitnessEnvelope,
    WitnessPacket,
};
use qsl_route::Mode;
use qsl_semantics::check::{Capability, TemporalClauseNode};
use quire_exact::Identifier;
use quire_semantic_value::catalog::InternalFault;

use super::items::{settle_model_item, ModelItem, BROKEN, CLAUSE_FAMILY};
use super::{AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SubjectBinding};
use crate::model_check::{manifest as en1, ModelCheckRequest};

/// The replay source an EN-1 authored-clause counterexample carries: a
/// witness transcript with no values, since the clause's lasso is the
/// envelope's payload (FR-128).
pub const CLAUSE_TRANSCRIPT: &str = "<<<assertion|model_check|clause|>>>";

/// The budgets of one authored-clause run.
#[derive(Clone, Copy, Debug)]
pub struct ClauseLimits {
    /// FR-101's `max_states` and `max_transitions`.
    pub limits: Limits,
    /// The search horizon `k`; `None` sets none.
    pub max_depth: Option<usize>,
    /// The most successor candidates one expansion enumerates.
    pub max_candidates: u64,
}

impl Default for ClauseLimits {
    fn default() -> Self {
        Self {
            limits: Limits::default(),
            max_depth: None,
            max_candidates: u64::MAX,
        }
    }
}

/// Why no clause item was made.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClauseItemRefusal {
    /// The package holds no temporal clause at the key (node and claim
    /// occurrence).
    #[error("the package holds no temporal clause at the given occurrence")]
    UnknownClause,
    /// The clause's declared name is no identifier, so no `state_clause`
    /// target can name it.
    #[error("the clause's declared name is not an identifier")]
    UnnamedClause,
}

/// The inputs of one authored-clause item.
#[derive(Clone)]
pub struct ClauseSubject<'a> {
    package: &'a CheckedPackage,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    unit: &'a [u8],
    node: &'a TemporalClauseNode,
    run: ClauseLimits,
}

impl<'a> ClauseSubject<'a> {
    /// The subject of the clause at `key` (its node and `claim` occurrence)
    /// of `package`. `unit` is the source bytes `package` was checked from,
    /// which the replay recompiles.
    ///
    /// # Errors
    ///
    /// [`ClauseItemRefusal`] when the package holds no such clause.
    pub fn new(
        package: &'a CheckedPackage,
        binding: SubjectBinding<'a>,
        limits: AnalyzeLimits,
        unit: &'a [u8],
        key: &OccurrenceKey,
        run: ClauseLimits,
    ) -> Result<Self, ClauseItemRefusal> {
        let node = package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.identity() == *key.node() && node.claim() == key.origin())
            .ok_or(ClauseItemRefusal::UnknownClause)?;
        clause_name(node)?;
        Ok(Self {
            package,
            binding,
            limits,
            unit,
            node,
            run,
        })
    }

    /// The `temporal-satisfaction` item that runs the clause on EN-1 and
    /// settles it by `settle_model_check` (FR-127), replaying a
    /// counterexample through the production `replay_model_trace`.
    pub fn item(&self, index: RequestIndex) -> AnalyzeItem<'a> {
        let subject = self.clone();
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            CLAUSE_FAMILY,
            Mode::Bounded,
            en1::BACKEND_ID,
            move |cancel| {
                let Ok(system) = ModelSystem::new(subject.simulation()) else {
                    return EngineReport::Refused(Refusal::Failed(BROKEN));
                };
                let Ok(letters) = system.clause_letters(subject.node) else {
                    return EngineReport::Refused(Refusal::Failed(BROKEN));
                };
                let item = || ModelCheckItem::Clause {
                    clause: subject.node.clause(),
                    letters: &letters,
                };
                settle_model_item(
                    ModelItem {
                        request: ModelCheckRequest {
                            temporal_clock_selections: &[],
                            subject: subject.model_subject(),
                            item: item(),
                            max_depth: subject.run.max_depth,
                            limits: ModelCheckLimits {
                                limits: subject.run.limits,
                                ..ModelCheckLimits::default()
                            },
                            reductions: Default::default(),
                        },
                        certificate: CertificateRequest {
                            temporal_clock_selections: &[],
                            subject: subject.model_subject(),
                            item: item(),
                            smt_encoding: qsl_replay::SmtEncodingLimits::default(),
                        },
                        replay: Box::new(|payload| subject.replay(payload, &letters)),
                    },
                    cancel,
                )
            },
        )
    }

    /// Replays `payload`, a counterexample of this clause, through
    /// `replay_model_trace` over the same source and the production letters:
    /// the replay `item` settles `refuted` by, run again for a caller that
    /// holds the record's counterexample.
    ///
    /// # Errors
    ///
    /// [`ReplayRefusal`] as `replay_model_trace` refuses it.
    pub fn replay_counterexample(
        &self,
        payload: &TemporalCounterexample,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        let broken = || ReplayRefusal::Fault(InternalFault::new("analyze", "clause-letters"));
        let system = ModelSystem::new(self.simulation()).map_err(ReplayRefusal::Admission)?;
        let letters = system.clause_letters(self.node).map_err(|_| broken())?;
        self.replay(payload, &letters)
    }

    fn simulation(&self) -> ModelSimulationRequest<'a> {
        ModelSimulationRequest {
            package: self.package,
            packages: self.binding.packages,
            normalization: self.limits.normalization,
            snapshots: self.binding.snapshots,
            initial: self.binding.initial.clone(),
            universes: self.binding.universes.clone(),
            observation: self.limits.observation,
            expansion: ExpansionLimits {
                max_candidates: self.run.max_candidates,
            },
            meter: self.limits.accounting,
            emit: self.limits.emit,
        }
    }

    fn model_subject(&self) -> qsl_eval::model_subject::ModelSubject<'a> {
        qsl_eval::model_subject::ModelSubject {
            simulation: self.simulation(),
            proof_bounds: Vec::new(),
        }
    }

    /// FR-128's replay of `payload` over the same source and subject.
    fn replay(
        &self,
        payload: &TemporalCounterexample,
        letters: &dyn qsl_eval::model_subject::ModelLetters,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        let fault =
            |cause: &'static str| ReplayRefusal::Fault(InternalFault::new("analyze", cause));
        let name = clause_name(self.node).map_err(|_| fault("clause-name"))?;
        let identity = self.node.identity();
        let source = self.package.graph().source();
        let emission =
            qsl_package::emit_checked_with_temporal_selections(self.package, &[], self.limits.emit)
                .map_err(|_| fault("package-emission"))?;
        let package = ReplayPackage::read(
            emission.package().bytes(),
            ReplayLimits::default().input_bytes,
        )
        .map_err(|_| fault("package-emission"))?;
        let transcript = || Witness::parse(CLAUSE_TRANSCRIPT).map_err(|_| fault("transcript"));
        let wire = ReplayRequestWire {
            package,
            profile_selections: Vec::new(),
            dependencies: Vec::new(),
            replay_target: RequestTargetWire::state_clause(name.clone()),
            source: ReplaySource::Witness(transcript()?),
            obligation_identity: ObligationIdentity::new(identity.clone(), None, 0),
            backend: en1::BACKEND_ID.to_owned(),
            state_environment: StateEnvironment::new(Vec::new()),
            accounting_limits: self.limits.accounting,
            stage_limits: std::collections::BTreeMap::new(),
            supplied_limits: None,
            object_environment: None,
            declared_domains: Vec::new(),
            byte_provision: vec![(
                source.authority().to_owned(),
                source.identity().to_owned(),
                self.unit.to_vec(),
            )],
            documents: self
                .binding
                .packages
                .values()
                .chain(self.binding.snapshots.values())
                .cloned()
                .collect(),
        };
        let envelope = WitnessEnvelope::reconstruct(
            WitnessPacket {
                obligation_identity: Some(ObligationIdentity::new(identity.clone(), None, 0)),
                occurrence_key: Some(OccurrenceKey::new(
                    identity.clone(),
                    self.node.claim().clone(),
                )),
                clause_node: Some(identity),
                replay_target: ReplayTargetMembers::state_clause(name),
                profile_selections: Some(Vec::new()),
                run_limits: Some(self.limits.accounting),
                declared_domains: Some(Vec::new()),
                backend: Some(en1::BACKEND_ID.to_owned()),
                trace_position: Some(None),
                source: Some(EnvelopeSource::Witness(transcript()?)),
                family_payload: Some(payload.clone()),
            },
            ReplayLimits::default(),
        )
        .map_err(|_| fault("envelope"))?;
        replay_model_trace(
            wire,
            &ModelTraceSubject {
                timed: None,
                initial: self.binding.initial.clone(),
                universes: self.binding.universes.clone(),
                expansion: ExpansionLimits {
                    max_candidates: self.run.max_candidates,
                },
            },
            &envelope,
            letters,
            ReplayLimits::default(),
        )
    }
}

/// The `state_clause` target of a temporal clause: its declared name.
fn clause_name(node: &TemporalClauseNode) -> Result<QualifiedName, ClauseItemRefusal> {
    let segment =
        Identifier::new(node.clause().name()).map_err(|_| ClauseItemRefusal::UnnamedClause)?;
    QualifiedName::new(vec![segment]).map_err(|_| ClauseItemRefusal::UnnamedClause)
}
