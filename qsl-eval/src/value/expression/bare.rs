// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531, FR-533: the protocol-slot evaluation of a package read from its
//! bytes alone.
//!
//! The call selects the slot from the package's `protocol` node, projects its
//! expression and the declarations it runs against from the package's nodes
//! and the identity rows, admits the supply, and runs the one S6a machine
//! over the projection. No value of the source side (a `CheckedPackage`, a
//! `CheckedBody`, a source graph) reaches any step.

use std::collections::BTreeMap;

use qsl_package::{BarePackage, BareSelectionFailure, BareSlot, BareWorld, ProtocolSlotDescriptor};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::{CallAdmissionFailure, CallAdmissionLocus, CallFailure, Evaluation};
use quire_exact::{Meter, NodeKey, Value};
use quire_semantic_value::catalog::InternalFault;

use super::evaluate::reader_source::ReaderNode;
use super::evaluate::{Machine, World};
use super::protocol_slot::{resolve, ProtocolSlot, ProtocolSlotEvaluation, ProtocolSupply};
use super::slot_supply::{admit_slot_supply, AdmittedSupply, SlotHost, SourceView};
use super::{
    unsupported_projection, ApplicationObservation, CheckedPackageEvaluation, ClauseEvaluation,
    ClauseSetEvaluation, FrameEvaluation, QualifiedName, Separation, SuppliedRun,
    ValueInvariantEvaluation,
};

/// A selection or projection refusal as the call reports it (FR-533
/// "Refusals and loci of the bare slot call"): a witness refusal at the
/// `Package` locus, an unsupported projection at its `Node` locus, a missing
/// node or member at theirs.
fn refusal(package: &BarePackage<'_>, failure: BareSelectionFailure) -> CallFailure {
    match failure.into_view(package.package().name()) {
        Ok(view) => unsupported_projection(&view),
        Err(failure) => resolve::selection_failure(failure),
    }
}

/// What supply admission reads from a bytes-only package. It holds no
/// population type and no model selection: an observation is admitted
/// through a bare view, which is not yet available (FR-533 "Observation
/// admission"), so a value that names an object reference is refused as a
/// reference to a model the package does not select.
struct BareHost<'w> {
    world: &'w BareWorld,
    levels: BTreeMap<NodeKey, usize>,
}

impl SlotHost for BareHost<'_> {
    fn types(&self) -> &quire_semantic_value::declaration::TypeEnvironment {
        self.world.types()
    }

    fn binder_level(&self, key: &NodeKey) -> usize {
        self.levels.get(key).copied().unwrap_or(usize::MAX)
    }

    fn population_types(
        &self,
        _node: &NodeKey,
    ) -> Option<(
        &quire_checked::key::DeclarationKey,
        &[quire_exact::EffectiveId],
    )> {
        None
    }

    fn holds_model(&self, _model: &qsl_semantics::model::domain_package::DomainPackageRef) -> bool {
        false
    }

    fn observation_is_for_view(
        &self,
        _observation: &qsl_package::AdmittedProtocolObservation<'_>,
    ) -> bool {
        false
    }
}

/// FR-533 "The call": evaluate `slot`, already selected from `package`'s
/// nodes, over `supply`.
///
/// # Errors
///
/// A [`CallFailure`]: a view-creation refusal as `Admission` with the
/// catalog pair `unknown_required_feature`/`unsupported-feature` before any
/// charge, a supply refusal as FR-390 states it, or a fault.
pub fn evaluate_selected_protocol_slot(
    package: &BarePackage<'_>,
    slot: &BareSlot,
    supply: &ProtocolSupply<'_>,
    meter: &mut Meter,
) -> Result<ProtocolSlotEvaluation, CallFailure> {
    let world = package
        .world()
        .map_err(|failure| refusal(package, failure))?;
    let projection = package
        .project_protocol_slot(slot)
        .map_err(|failure| refusal(package, failure))?;
    if slot
        .sources()
        .iter()
        .any(|source| projection.local_slot(source.binder()) != Some(source.index()))
    {
        return Err(CallFailure::Fault(InternalFault::new(
            "FR-533",
            "bare-slot-binder-is-the-selected-frame-slot",
        )));
    }
    let sources = slot
        .sources()
        .iter()
        .map(|source| SourceView {
            index: source.index(),
            value_type: source.value_type(),
            binder: source.binder(),
        })
        .collect::<Vec<_>>();
    let host = BareHost {
        world: &world,
        levels: slot
            .sources()
            .iter()
            .map(|source| (source.binder().clone(), source.level()))
            .collect(),
    };
    let AdmittedSupply {
        mut frame,
        unavailable,
        binder_slots,
    } = admit_slot_supply(&host, &sources, slot.frame_len(), supply)?;
    let objects = ObjectEnvironment::default();
    let no_reads = BTreeMap::new();
    // The call owns the meter's interval capture while it runs (FR-520-AC-11).
    let caller_capture = meter.take_charge_accounting().is_some();
    meter.enable_charge_accounting();
    frame.resize(projection.frame_slots().max(frame.len()), None);
    let run = Machine::with_world(
        World::Bytes(&world),
        &objects,
        None,
        Some(&no_reads),
        meter,
        &[],
    )
    .run_protocol_frame(
        ReaderNode::root(&projection),
        frame,
        unavailable,
        binder_slots,
    )
    .map_err(CallFailure::Fault);
    if !caller_capture {
        meter.disable_charge_accounting();
    }
    let (result, charges) = run?;
    Ok(match result {
        Ok(evaluation) => ProtocolSlotEvaluation::Evaluated(evaluation, charges),
        Err((binder, cause, location)) => ProtocolSlotEvaluation::Unavailable {
            binder,
            cause,
            location,
            charges,
        },
    })
}

/// A capability of the source route the bytes do not carry (FR-531
/// "Capability inventory"): refused at the `Package` locus, with no charge.
fn unmet(package: &BarePackage<'_>, reason: &'static str) -> CallFailure {
    let mut record = super::occurrence_record(
        "unknown_required_feature",
        "unsupported-feature",
        Vec::new(),
    );
    record
        .fields
        .insert("package", package.package().name().to_owned());
    record.fields.insert("reason", reason.to_owned());
    CallFailure::Admission(Box::new(CallAdmissionFailure::Refused(
        quire_eval::CallAdmissionRecord {
            record,
            locus: CallAdmissionLocus::Package,
        },
    )))
}

const SOURCE_DECLARATION: &str = "needs-source-declaration";
const SOURCE_CHECKED_FORM: &str = "needs-source-checked-form";

impl CheckedPackageEvaluation for BarePackage<'_> {
    fn call(
        &self,
        _function: &QualifiedName,
        _arguments: Vec<Value>,
        _objects: &ObjectEnvironment,
        _meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }

    fn call_observing_applications(
        &self,
        _function: &QualifiedName,
        _arguments: Vec<Value>,
        _objects: &ObjectEnvironment,
        _meter: &mut Meter,
        _observation: ApplicationObservation<'_>,
    ) -> Result<Evaluation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }

    fn call_supplied(
        &self,
        _function: &QualifiedName,
        _arguments: Vec<Value>,
        _objects: &ObjectEnvironment,
        _meter: &mut Meter,
        _run: &mut SuppliedRun,
    ) -> Result<Evaluation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }

    fn evaluate(
        &self,
        _expression: &qsl_semantics::check::CheckedExpression,
        _arguments: Vec<Value>,
        _objects: &ObjectEnvironment,
        _meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        Err(unmet(self, SOURCE_CHECKED_FORM))
    }

    fn evaluate_protocol_slot(
        &self,
        slot: &ProtocolSlot,
        supply: &ProtocolSupply<'_>,
        meter: &mut Meter,
    ) -> Result<ProtocolSlotEvaluation, CallFailure> {
        let descriptor =
            ProtocolSlotDescriptor::new(slot.node.clone(), slot.member.clone(), slot.index);
        let selected = self
            .select_protocol_slot(&descriptor)
            .map_err(|failure| refusal(self, failure))?;
        evaluate_selected_protocol_slot(self, &selected, supply, meter)
    }

    fn evaluate_selected_protocol_slot(
        &self,
        _view: &qsl_package::AdmittedEvaluationView<'_>,
        selected: &qsl_package::SelectedProtocolTerm<'_, '_>,
        _supply: &ProtocolSupply<'_>,
        _meter: &mut Meter,
    ) -> Result<ProtocolSlotEvaluation, CallFailure> {
        // A view and a selection of the source side belong to a source
        // package, never to this one (use `evaluate_selected_protocol_slot`
        // of this module for a slot selected from the bytes).
        Err(resolve::refused(
            "invalid_model_binding",
            "wrong-model-selection",
            CallAdmissionLocus::Node(selected.root().node().clone()),
        ))
    }

    fn evaluate_holds(
        &self,
        _clause: &qsl_semantics::check::TemporalClauseNode,
        _atom: qsl_semantics::check::FormulaNodeId,
        _over: Value,
        _objects: &ObjectEnvironment,
        _meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        Err(unmet(self, SOURCE_CHECKED_FORM))
    }

    fn evaluate_holds_traced(
        &self,
        _clause: &qsl_semantics::check::TemporalClauseNode,
        _atom: qsl_semantics::check::FormulaNodeId,
        _over: &quire_exact::ObjectReference,
        _objects: &ObjectEnvironment,
        _observation: super::ObservationIdentity,
        _meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure> {
        Err(unmet(self, SOURCE_CHECKED_FORM))
    }

    fn evaluate_guard(
        &self,
        _clause: &qsl_semantics::check::TemporalClauseNode,
        _over: Value,
        _objects: &ObjectEnvironment,
        _meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        Err(unmet(self, SOURCE_CHECKED_FORM))
    }

    fn evaluate_hyper_holds(
        &self,
        _clause: &qsl_semantics::check::CheckedHyperClause,
        _atom: qsl_semantics::check::FormulaNodeId,
        _slots: Vec<crate::hyper::HyperSlot>,
        _components: &[&ObjectEnvironment],
        _meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        Err(unmet(self, SOURCE_CHECKED_FORM))
    }

    fn evaluate_step_predicate(
        &self,
        _clause: &qsl_semantics::check::CheckedHyperClause,
        _part: crate::hyper::MatchPart,
        _conjunct: usize,
        _labels: &[crate::hyper::StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        Err(unmet(self, SOURCE_CHECKED_FORM))
    }

    fn evaluate_clause(
        &self,
        _clause: &QualifiedName,
        _observations: &qsl_semantics::model::observation::AdmittedObservations,
        _meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }

    fn evaluate_clause_set(
        &self,
        _clauses: &[(
            QualifiedName,
            &qsl_semantics::model::observation::AdmittedObservations,
        )],
        _limits: quire_exact::ScalarLimits,
    ) -> Result<ClauseSetEvaluation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }

    fn evaluate_value_invariants(
        &self,
        _observations: &qsl_semantics::model::observation::AdmittedObservations,
        _meter: &mut Meter,
    ) -> Result<ValueInvariantEvaluation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }

    fn check_separation(
        &self,
        _clause: &QualifiedName,
        _observations: &qsl_semantics::model::observation::AdmittedObservations,
        _claim: super::WitnessClaim<'_>,
        _meter: &mut Meter,
    ) -> Result<Separation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }

    fn evaluate_frame(
        &self,
        _frame: NodeKey,
        _invocation: &qsl_semantics::model::observation::AdmittedInvocation<'_>,
        _meter: &mut Meter,
    ) -> Result<FrameEvaluation, CallFailure> {
        Err(unmet(self, SOURCE_CHECKED_FORM))
    }

    fn evaluate_collection_write(
        &self,
        _clause: &QualifiedName,
        _observations: &qsl_semantics::model::observation::AdmittedObservations,
        _field: &str,
        _written: &qsl_semantics::check::Written,
        _meter: &mut Meter,
    ) -> Result<Option<super::collection_write::WrittenCollection>, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }
}
