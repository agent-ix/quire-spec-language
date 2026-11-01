// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked protocol temporal evaluation and immutable correction lineage.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::protocol_artifact::{v3, wire::Handle};
use qsl_package::CheckedPackage;

use super::{evaluate_checked_with_progress_v2, Ledger, Limits, Obligation, Report, Trace, Usage};

/// An opaque semantic-trigger identity supplied by the observation owner.
///
/// QSL retains the exact UTF-8 bytes and does not parse, normalize, or derive
/// this value from any other trigger field. This checked API has no separate
/// identity-version envelope; malformed, stale, and cross-version encodings
/// therefore are not inferred from opaque identity bytes.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SemanticTriggerIdentity(String);

impl SemanticTriggerIdentity {
    /// Retain a nonempty owner-supplied identity byte-for-byte.
    pub fn new(value: impl Into<String>) -> Result<Self, ProtocolRefusal> {
        let value = value.into();
        if value.is_empty() {
            return Err(ProtocolRefusal::EmptyTriggerIdentity);
        }
        Ok(Self(value))
    }

    /// The exact UTF-8 spelling selected by the observation owner.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A correction names the exact immediately preceding evaluation result.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProtocolEvaluationId(u64);

impl ProtocolEvaluationId {
    /// Stable identifier assigned by one [`ProtocolLedger`].
    pub fn value(self) -> u64 {
        self.0
    }
}

/// The checked protocol event-control and temporal declaration selected by
/// the admitted `/3` control-to-temporal map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolSelection {
    /// Exact event control handle in the admitted package.
    pub control: Handle,
    /// Exact selected temporal declaration index in the admitted package.
    pub temporal_declaration: u32,
}

/// One checked protocol temporal evaluation input.
#[derive(Clone, Debug)]
pub struct ProtocolEvaluationInput {
    /// The admitted control and temporal-declaration selection.
    pub selection: ProtocolSelection,
    /// Checked temporal evidence, including positions, progress, closure,
    /// receipts, captures, and other independent facts.
    pub trace: Trace,
    /// Owner-selected identities in trigger order. These values replace the
    /// identities in `trace.triggers`; all other trace fields remain evidence.
    pub trigger_identities: Vec<SemanticTriggerIdentity>,
    /// Present only when this evaluation corrects an earlier result.
    pub correction: Option<ProtocolEvaluationId>,
}

/// Why a checked protocol temporal input was refused before evaluation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProtocolRefusal {
    /// The owner supplied an empty semantic-trigger identity.
    #[error("semantic-trigger identity is empty")]
    EmptyTriggerIdentity,
    /// The owner identity count did not match the trigger evidence count.
    #[error("owner identity count does not match admitted trigger count")]
    TriggerIdentityCount,
    /// The selected control has no admitted temporal mapping.
    #[error("event control has no admitted temporal mapping")]
    MissingActivationMapping,
    /// The temporal declaration differs from the admitted control mapping.
    #[error("temporal declaration differs from the admitted control mapping")]
    ActivationMappingMismatch,
    /// The selected temporal declaration is not present in the checked graph.
    #[error("selected temporal declaration is not in the checked package")]
    MissingCheckedDeclaration,
    /// A correction does not name the latest result for this exact selection.
    #[error("correction must name the exact direct predecessor for this obligation")]
    CorrectionPredecessor,
    /// A correction did not change the evaluation input.
    #[error("correction input is unchanged")]
    UnchangedCorrection,
    /// A correction changed the selected owner identity set.
    #[error("correction changed the selected semantic-trigger identity")]
    CorrectionIdentity,
    /// The ledger's result identifier space is exhausted.
    #[error("protocol evaluation identifier space is exhausted")]
    IdentifierExhausted,
}

/// A retained protocol evaluation and its immutable predecessor relation.
#[derive(Clone, Debug)]
pub struct ProtocolEvaluation {
    id: ProtocolEvaluationId,
    predecessor: Option<ProtocolEvaluationId>,
    selection: ProtocolSelection,
    anchor: String,
    report: Report,
    settled_siblings: Vec<Obligation>,
    completed_siblings: Vec<Obligation>,
}

impl ProtocolEvaluation {
    /// This result's immutable ledger identity.
    pub fn id(&self) -> ProtocolEvaluationId {
        self.id
    }

    /// Exact direct predecessor, if this result is a correction.
    pub fn predecessor(&self) -> Option<ProtocolEvaluationId> {
        self.predecessor
    }

    /// Exact admitted control and temporal declaration selected for this result.
    pub fn selection(&self) -> &ProtocolSelection {
        &self.selection
    }

    /// The input trace's exact evaluation anchor, retained independently of
    /// trigger captures and assessment premises.
    pub fn anchor(&self) -> &str {
        &self.anchor
    }

    /// The temporal report produced by QSL from the checked declaration and
    /// supplied evidence.
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// Obligations that settled before a sibling hit a typed evaluation stop.
    /// The stopped obligation has no partial truth and remains in `report` as
    /// its typed error.
    pub fn settled_siblings(&self) -> &[Obligation] {
        &self.settled_siblings
    }

    /// All completed sibling dispositions retained before another sibling
    /// reached a typed evaluation stop.
    pub fn completed_siblings(&self) -> &[Obligation] {
        &self.completed_siblings
    }
}

type ObligationKey = (u32, u32, u32);

/// Retains immutable results for checked protocol temporal obligations.
#[derive(Clone, Debug, Default)]
pub struct ProtocolLedger {
    next_id: u64,
    next_package_id: u64,
    evaluations: BTreeMap<ProtocolEvaluationId, ProtocolEvaluation>,
    packages: BTreeMap<u64, v3::wire::Package>,
    by_obligation: BTreeMap<(u64, ObligationKey), Vec<ProtocolEvaluationId>>,
    inputs: BTreeMap<ProtocolEvaluationId, Trace>,
    temporal_ledgers: BTreeMap<(u64, ObligationKey), Ledger>,
}

impl ProtocolLedger {
    /// An empty ledger with no asserted evaluation history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Find one immutable result by its ledger identity.
    pub fn get(&self, id: ProtocolEvaluationId) -> Option<&ProtocolEvaluation> {
        self.evaluations.get(&id)
    }

    /// Evaluate one checked protocol temporal obligation through the admitted
    /// control mapping. Invalid selections and correction edges refuse before
    /// the temporal evaluator runs or the ledger changes.
    pub fn evaluate(
        &mut self,
        checked: &CheckedPackage,
        admitted: &v3::AdmittedPackage,
        input: ProtocolEvaluationInput,
        limits: Limits,
    ) -> Result<&ProtocolEvaluation, ProtocolRefusal> {
        let mapping = admitted
            .activation_mappings()
            .iter()
            .find(|mapping| mapping.control == input.selection.control)
            .ok_or(ProtocolRefusal::MissingActivationMapping)?;
        if mapping.temporal_declaration != input.selection.temporal_declaration {
            return Err(ProtocolRefusal::ActivationMappingMismatch);
        }
        let declaration = admitted
            .temporal_selection()
            .package()
            .temporal_bindings
            .iter()
            .position(|binding| binding.declaration == input.selection.temporal_declaration)
            .ok_or(ProtocolRefusal::MissingCheckedDeclaration)?;
        if checked
            .graph()
            .temporal_clauses()
            .get(declaration)
            .is_none()
        {
            return Err(ProtocolRefusal::MissingCheckedDeclaration);
        }
        if input.trigger_identities.len() != input.trace.triggers.len() {
            return Err(ProtocolRefusal::TriggerIdentityCount);
        }
        let effective = limits.bounded();
        let clause = checked
            .graph()
            .temporal_clauses()
            .get(declaration)
            .ok_or(ProtocolRefusal::MissingCheckedDeclaration)?
            .clause();
        let guarded = matches!(
            clause.activation(),
            qsl_semantics::check::Activation::Each { guarded: true }
        );
        if let Some(exhaustion) = preflight_input(
            &input.trace,
            &input.trigger_identities,
            effective,
            declaration,
            guarded,
            clause.captures().len(),
        ) {
            return self.record_bounded_input(
                input.selection,
                input.trace.anchor,
                effective,
                exhaustion,
            );
        }
        let mut trace = input.trace;
        for (trigger, identity) in trace.triggers.iter_mut().zip(input.trigger_identities) {
            trigger.identity = identity.0;
        }

        let package_id = self.package_id(admitted.package())?;
        let key = (
            package_id,
            (
                input.selection.control.declaration,
                input.selection.control.index,
                input.selection.temporal_declaration,
            ),
        );
        let history = self.by_obligation.get(&key);
        let predecessor = match (input.correction, history.and_then(|ids| ids.last())) {
            (Some(requested), Some(latest)) if requested == *latest => {
                let prior = self
                    .inputs
                    .get(&requested)
                    .ok_or(ProtocolRefusal::CorrectionPredecessor)?;
                if prior == &trace {
                    return Err(ProtocolRefusal::UnchangedCorrection);
                }
                if !same_trigger_identities(prior, &trace) {
                    return Err(ProtocolRefusal::CorrectionIdentity);
                }
                Some(requested)
            }
            (Some(_), _) => return Err(ProtocolRefusal::CorrectionPredecessor),
            (None, _) => None,
        };
        let id = ProtocolEvaluationId(self.next_id);
        let next_id = self
            .next_id
            .checked_add(1)
            .ok_or(ProtocolRefusal::IdentifierExhausted)?;
        let temporal_ledger = self.temporal_ledgers.entry(key).or_default();
        let (report, settled_siblings, completed_siblings) = evaluate_siblings(
            checked,
            admitted,
            declaration,
            temporal_ledger,
            trace.clone(),
            limits,
        );
        self.next_id = next_id;
        let anchor = trace.anchor.clone();
        self.inputs.insert(id, trace);
        self.by_obligation.entry(key).or_default().push(id);
        self.evaluations.insert(
            id,
            ProtocolEvaluation {
                id,
                predecessor,
                selection: input.selection,
                anchor,
                report,
                settled_siblings,
                completed_siblings,
            },
        );
        self.evaluations
            .get(&id)
            .ok_or(ProtocolRefusal::IdentifierExhausted)
    }

    fn package_id(&mut self, package: &v3::wire::Package) -> Result<u64, ProtocolRefusal> {
        if let Some((id, _)) = self.packages.iter().find(|(_, known)| *known == package) {
            return Ok(*id);
        }
        let id = self.next_package_id;
        self.next_package_id = id
            .checked_add(1)
            .ok_or(ProtocolRefusal::IdentifierExhausted)?;
        self.packages.insert(id, package.clone());
        Ok(id)
    }

    fn record_bounded_input(
        &mut self,
        selection: ProtocolSelection,
        anchor: String,
        limits: Limits,
        exhaustion: super::Exhaustion,
    ) -> Result<&ProtocolEvaluation, ProtocolRefusal> {
        let id = ProtocolEvaluationId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(ProtocolRefusal::IdentifierExhausted)?;
        self.evaluations.insert(
            id,
            ProtocolEvaluation {
                id,
                predecessor: None,
                selection,
                anchor,
                report: Report {
                    result: Err(super::Error::Exhausted(exhaustion)),
                    limits,
                    usage: Usage::default(),
                    authenticated: None,
                },
                settled_siblings: Vec::new(),
                completed_siblings: Vec::new(),
            },
        );
        self.evaluations
            .get(&id)
            .ok_or(ProtocolRefusal::IdentifierExhausted)
    }
}

fn same_trigger_identities(left: &Trace, right: &Trace) -> bool {
    left.triggers.len() == right.triggers.len()
        && left
            .triggers
            .iter()
            .zip(&right.triggers)
            .all(|(left, right)| left.identity == right.identity)
}

fn preflight_input(
    trace: &Trace,
    identities: &[SemanticTriggerIdentity],
    limits: Limits,
    declaration: usize,
    guarded: bool,
    capture_count: usize,
) -> Option<super::Exhaustion> {
    use super::LimitDimension as Dimension;

    let subject = super::Subject {
        declaration,
        ..super::Subject::default()
    };
    if trace.positions.len() > limits.positions {
        return Some(super::Exhaustion {
            dimension: Dimension::Positions,
            used: 0,
            requested: trace.positions.len(),
            limit: limits.positions,
            subject,
        });
    }
    let mut valuations = 0usize;
    let mut preflight_work = 0usize;
    for position in &trace.positions {
        valuations = valuations.saturating_add(position.valuations.len());
        if valuations > limits.valuations {
            return Some(preflight_exhaustion(
                Dimension::Valuations,
                limits.valuations,
                subject,
            ));
        }
        for instance in position.instance_valuations.values() {
            preflight_work = preflight_work.saturating_add(1);
            if preflight_work > limits.visits {
                return Some(preflight_exhaustion(
                    Dimension::Visits,
                    limits.visits,
                    subject,
                ));
            }
            valuations = valuations.saturating_add(instance.len());
            if valuations > limits.valuations {
                return Some(preflight_exhaustion(
                    Dimension::Valuations,
                    limits.valuations,
                    subject,
                ));
            }
        }
    }
    if trace.evicted.len() > limits.retention {
        return Some(super::Exhaustion {
            dimension: Dimension::Retention,
            used: 0,
            requested: trace.evicted.len(),
            limit: limits.retention,
            subject,
        });
    }
    // Bound scans over per-instance valuation tables, deliveries, and supplied
    // captures before constructing temporary identity state. Empty instance
    // maps still consume scan work; retained-instance and capture charges below
    // are computed once per successful semantic identity.
    for trigger in &trace.triggers {
        preflight_work = preflight_work.saturating_add(1);
        if preflight_work > limits.visits {
            return Some(preflight_exhaustion(
                Dimension::Visits,
                limits.visits,
                subject,
            ));
        }
        preflight_work = preflight_work.saturating_add(trigger.captures.len());
        if preflight_work > limits.visits {
            return Some(preflight_exhaustion(
                Dimension::Visits,
                limits.visits,
                subject,
            ));
        }
    }

    let evicted_captures: BTreeSet<(&str, usize)> = trace
        .evicted
        .iter()
        .filter_map(|evicted| match evicted {
            super::Eviction::Capture { instance, capture } => Some((instance.as_str(), *capture)),
            super::Eviction::Valuation { .. } => None,
        })
        .collect();

    let mut instances = BTreeSet::new();
    let mut captured = BTreeSet::new();
    let mut retained_captures = 0usize;
    for (trigger, identity) in trace.triggers.iter().zip(identities) {
        let active = trace.trigger_evidence == super::Evidence::Admitted
            && (!guarded || trigger.guard == Some(true));
        if !active {
            continue;
        }

        if instances.insert(identity.as_str()) && instances.len() > limits.instances {
            return Some(super::Exhaustion {
                dimension: Dimension::Instances,
                used: 0,
                requested: instances.len(),
                limit: limits.instances,
                subject,
            });
        }

        let captures_established = trigger.captures.len() == capture_count
            && trigger.anchor == trace.anchor
            && trigger.captures.iter().enumerate().all(|(index, capture)| {
                !evicted_captures.contains(&(identity.as_str(), index))
                    && matches!(
                        capture,
                        super::CaptureInput::Value { anchor, .. } if anchor == &trace.anchor
                    )
            });
        if captures_established && captured.insert(identity.as_str()) {
            retained_captures = retained_captures.saturating_add(capture_count);
            if retained_captures > limits.captures {
                return Some(super::Exhaustion {
                    dimension: Dimension::Captures,
                    used: 0,
                    requested: retained_captures,
                    limit: limits.captures,
                    subject,
                });
            }
        }
    }
    None
}

fn preflight_exhaustion(
    dimension: super::LimitDimension,
    limit: usize,
    subject: super::Subject,
) -> super::Exhaustion {
    super::Exhaustion {
        dimension,
        used: 0,
        requested: limit.saturating_add(1),
        limit,
        subject,
    }
}

fn evaluate_siblings(
    checked: &CheckedPackage,
    admitted: &v3::AdmittedPackage,
    declaration: usize,
    ledger: &mut Ledger,
    mut trace: Trace,
    limits: Limits,
) -> (Report, Vec<Obligation>, Vec<Obligation>) {
    let effective = limits.bounded();
    if trace.triggers.len() <= 1 || trace.trigger_evidence != super::Evidence::Admitted {
        return (
            evaluate_checked_with_progress_v2(
                checked,
                admitted.temporal_selection(),
                declaration,
                &trace,
                effective,
                ledger,
            ),
            Vec::new(),
            Vec::new(),
        );
    }

    // Keep every delivery of one identity together so activation can apply its
    // duplicate-receipt and contradiction rules to the complete identity.
    let mut groups: Vec<(String, usize, Vec<super::Trigger>)> = Vec::new();
    let mut group_indices: BTreeMap<String, usize> = BTreeMap::new();
    for (ordinal, trigger) in std::mem::take(&mut trace.triggers).into_iter().enumerate() {
        if let Some(index) = group_indices.get(&trigger.identity).copied() {
            groups[index].2.push(trigger);
        } else {
            group_indices.insert(trigger.identity.clone(), groups.len());
            groups.push((trigger.identity.clone(), ordinal, vec![trigger]));
        }
    }

    let mut accumulated = Usage::default();
    let mut obligations = Vec::new();
    let mut settled_siblings = Vec::new();
    let mut authenticated = None;
    let mut stopped = None;
    let mut has_uninstanced_disposition = false;
    for (_, ordinal, triggers) in groups {
        trace.triggers = triggers;
        let mut report = evaluate_checked_with_progress_v2(
            checked,
            admitted.temporal_selection(),
            declaration,
            &trace,
            remaining_limits(effective, accumulated),
            ledger,
        );
        trace.triggers = Vec::new();
        remap_report_instance(&mut report, ordinal);
        merge_usage(&mut accumulated, report.usage());
        authenticated = report.authenticated().cloned().or(authenticated);
        match report.result() {
            Ok(values) => {
                for value in values {
                    if let Obligation::Assessed(assessment) = value {
                        if assessment.instance.is_none() {
                            if has_uninstanced_disposition {
                                continue;
                            }
                            has_uninstanced_disposition = true;
                        }
                    }
                    obligations.push(value.clone());
                }
            }
            Err(error) => {
                stopped = Some(error.clone());
                break;
            }
        }
    }

    let has_instance = obligations.iter().any(|obligation| match obligation {
        Obligation::Assessed(assessment) => assessment.instance.is_some(),
        Obligation::Unactivated { .. } => true,
    });
    if has_instance || stopped.is_some() {
        obligations.retain(|obligation| match obligation {
            Obligation::Assessed(assessment) => assessment.instance.is_some(),
            Obligation::Unactivated { .. } => true,
        });
    }

    if let Some(error) = stopped {
        let completed_siblings = obligations.clone();
        settled_siblings.extend(
            obligations
                .into_iter()
                .filter(|obligation| match obligation {
                    Obligation::Assessed(assessment) => assessment.truth.is_some(),
                    Obligation::Unactivated { .. } => false,
                }),
        );
        (
            Report {
                result: Err(error),
                limits: effective,
                usage: accumulated,
                authenticated,
            },
            settled_siblings,
            completed_siblings,
        )
    } else {
        (
            Report {
                result: Ok(obligations),
                limits: effective,
                usage: accumulated,
                authenticated,
            },
            settled_siblings,
            Vec::new(),
        )
    }
}

fn remap_report_instance(report: &mut Report, instance: usize) {
    match &mut report.result {
        Ok(obligations) => {
            for obligation in obligations {
                match obligation {
                    Obligation::Assessed(assessment) => {
                        assessment.subject.instance = instance;
                        if let Some(incomplete) = &mut assessment.incomplete {
                            incomplete.subject.instance = instance;
                        }
                    }
                    Obligation::Unactivated { subject, error, .. } => {
                        subject.instance = instance;
                        remap_error_instance(error, instance);
                    }
                }
            }
        }
        Err(error) => remap_error_instance(error, instance),
    }
}

fn remap_error_instance(error: &mut super::Error, instance: usize) {
    match error {
        super::Error::Refused(refusal) => match refusal {
            super::Refusal::Binding { subject, .. }
            | super::Refusal::Order { subject }
            | super::Refusal::Capture { subject, .. }
            | super::Refusal::Contradiction { subject, .. }
            | super::Refusal::Progress { subject, .. }
            | super::Refusal::Reference { subject } => subject.instance = instance,
        },
        super::Error::Incomplete(incomplete) => incomplete.subject.instance = instance,
        super::Error::Exhausted(exhaustion) => exhaustion.subject.instance = instance,
        super::Error::Unsupported(_) => {}
    }
}

fn remaining_limits(limits: Limits, usage: Usage) -> Limits {
    Limits {
        positions: limits.positions.saturating_sub(usage.positions),
        valuations: limits.valuations.saturating_sub(usage.valuations),
        instances: limits.instances.saturating_sub(usage.instances),
        captures: limits.captures.saturating_sub(usage.captures),
        retention: limits.retention,
        visits: limits.visits.saturating_sub(usage.visits),
        depth: limits.depth,
        horizon: limits.horizon,
    }
}

fn merge_usage(total: &mut Usage, usage: Usage) {
    total.positions = total.positions.saturating_add(usage.positions);
    total.valuations = total.valuations.saturating_add(usage.valuations);
    // Every trigger group remains an independently retained obligation for
    // this invocation, matching the evaluator's cumulative active-instance
    // charge across a full trace.
    total.instances = total.instances.saturating_add(usage.instances);
    total.captures = total.captures.saturating_add(usage.captures);
    total.retention = total.retention.max(usage.retention);
    total.visits = total.visits.saturating_add(usage.visits);
    total.depth = total.depth.max(usage.depth);
    total.horizon = total.horizon.max(usage.horizon);
}
