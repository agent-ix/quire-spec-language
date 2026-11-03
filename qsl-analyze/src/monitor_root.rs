// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251 "Plan root" and FR-487: the public handoff from a timed clause of a
//! unit to its tick-monitor plan-root package.
//!
//! [`emit_plan_root`] compiles the unit, derives the plan for the named
//! clause and target, generates the four entries as source (FR-251
//! "Transitions"), appends them to the same unit so that the one checker
//! gives them their types, compiles that unit again, requires it to derive the
//! very same plan, and writes the plan-root node and the declaration-less
//! entry functions onto the emitted package. The caller reads the returned
//! bytes through Contract IR's strict reader; nothing here is read back or
//! admitted by a second scheme.
//!
//! The entries are checked beside the clause, in its unit, so the handoff
//! takes the authored unit rather than a checked package: a checked package
//! carries no source to append to. The unit imports no library: the base
//! package is read back for the writer, and a dependency's admitted package
//! is not part of this request.

use qsl_foundation::SourceIdentity;
use qsl_package::{
    emit_checked, emit_monitor_plan_over, CheckedPackage, Emission, EmitRefusal, MonitorPlanWire,
};
use qsl_replay::spine::{
    check, parse, refusal_or_fault, select, CheckedUnit, DependencyInput, ParseRequest, SpineLimits,
};
use qsl_replay::{
    MonitorPlanRefusal, MonitorTarget, ReplayRefusal, TimedMonitorPlan, TimingObligation,
    TimingObligationSettings,
};
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::intake::PackageInput;
use quire_exact::Cancel;

use crate::monitor_entries::{names, render, spec_from_plan, WIRE_FIELD_NAMES};
use crate::monitor_plan::produce;

/// The authored unit and the selection the plan root is written for.
#[derive(Clone, Copy, Debug)]
pub struct PlanRootRequest<'a> {
    /// The unit's source identity.
    pub source: &'a SourceIdentity,
    /// The path the unit is displayed as.
    pub path: &'a str,
    /// The unit's bytes as authored, with the model time source and
    /// temporal profile its clause needs to emit already declared.
    pub bytes: &'a [u8],
    /// The domain package documents the unit's `model` declarations select.
    pub packages: &'a PackageInput,
    /// The manifest name of the emitted package.
    pub package_name: &'a LibraryName,
    /// The compile limits.
    pub limits: SpineLimits,
    /// The authored name of the temporal clause to monitor.
    pub clause: &'a str,
    /// The target premise (counter, unit, width, uncertainty, rate, gap,
    /// activation capacity).
    pub target: &'a MonitorTarget,
    /// The FR-252 timing-obligation settings.
    pub settings: &'a TimingObligationSettings,
}

/// The plan, its timing obligations and the package that carries it.
#[derive(Debug)]
pub struct PlanRoot {
    /// The derived plan.
    pub plan: TimedMonitorPlan,
    /// The FR-252 timing obligations of the plan.
    pub obligations: [TimingObligation; 2],
    /// The checked-package v2 bytes holding the one `temporal`/
    /// `monitor_plan` node and the four declaration-less entry functions.
    pub emission: Emission,
}

/// Why no plan root was produced.
#[derive(Debug, thiserror::Error)]
pub enum PlanRootRefusal {
    /// A front-end stage refused the unit, or the unit with its entries.
    #[error("the unit does not compile: {0}")]
    FrontEnd(Box<ReplayRefusal>),
    /// No temporal clause of the unit has this name.
    #[error("the unit has no temporal clause named {clause}")]
    UnknownClause {
        /// The requested authored clause name.
        clause: String,
    },
    /// Several temporal clauses of the unit have this name.
    #[error("the unit has several temporal clauses named {clause}")]
    AmbiguousClause {
        /// The requested authored clause name.
        clause: String,
    },
    /// The clause and target derive no complete program (FR-251 "Fault
    /// boundary").
    #[error("the clause derives no plan: {0}")]
    Plan(#[from] MonitorPlanRefusal),
    /// The unit with the entries appended derives a different plan than the
    /// unit alone, so the entries do not belong to the plan.
    #[error("the unit with its entries derives a different plan")]
    EntriesChangedThePlan,
    /// The plan-root writer refused.
    #[error("the plan root cannot be written: {0}")]
    Write(#[from] EmitRefusal),
}

fn front_end(failure: qsl_replay::spine::FrontEndFailure) -> PlanRootRefusal {
    PlanRootRefusal::FrontEnd(Box::new(match refusal_or_fault(failure) {
        Ok(refusal) => ReplayRefusal::Recompile(refusal),
        Err(fault) => ReplayRefusal::Fault(fault),
    }))
}

fn compile(
    request: &PlanRootRequest<'_>,
    bytes: &[u8],
    cancel: &Cancel,
) -> Result<CheckedUnit, PlanRootRefusal> {
    let parsed = parse(
        &ParseRequest {
            source: request.source,
            path: request.path,
            bytes,
        },
        request.limits.source,
        cancel,
    )
    .map_err(front_end)?
    .into_value();
    let admitted = select(&parsed, request.packages, request.limits.model, cancel)
        .map_err(front_end)?
        .into_value();
    let checked = check(
        &parsed,
        request.package_name.as_str().to_owned(),
        &admitted,
        &DependencyInput::default(),
        request.limits,
        cancel,
    )
    .map_err(front_end)?
    .into_value();
    Ok(checked)
}

/// The one clause named `name` of `package`, by authored name.
fn clause_of<'p>(
    package: &'p CheckedPackage,
    name: &str,
) -> Result<&'p qsl_semantics::check::TemporalClauseNode, PlanRootRefusal> {
    let mut named = package
        .graph()
        .temporal_clauses()
        .iter()
        .filter(|node| node.clause().name() == name);
    let first = named.next().ok_or_else(|| PlanRootRefusal::UnknownClause {
        clause: name.to_owned(),
    })?;
    if named.next().is_some() {
        return Err(PlanRootRefusal::AmbiguousClause {
            clause: name.to_owned(),
        });
    }
    Ok(first)
}

/// Produce the plan-root package of `request`'s clause and target.
pub fn emit_plan_root(
    request: &PlanRootRequest<'_>,
    cancel: &Cancel,
) -> Result<PlanRoot, PlanRootRefusal> {
    let authored = compile(request, request.bytes, cancel)?;
    let node = clause_of(authored.package(), request.clause)?;
    let (plan, obligations) = produce(
        authored.package(),
        request.package_name.clone(),
        node.clause(),
        request.target,
        request.settings,
    )?;
    let entries = render(&spec_from_plan(&plan).map_err(MonitorPlanRefusal::from)?);

    let mut with_entries = request.bytes.to_vec();
    with_entries.push(b'\n');
    with_entries.extend_from_slice(entries.as_bytes());
    let combined = compile(request, &with_entries, cancel)?;
    let node = clause_of(combined.package(), request.clause)?;
    let (again, _) = produce(
        combined.package(),
        request.package_name.clone(),
        node.clause(),
        request.target,
        request.settings,
    )?;
    if again != plan {
        return Err(PlanRootRefusal::EntriesChangedThePlan);
    }

    let emitted = emit_checked(combined.package())?;
    let mut evidence = emitted.evidence().clone();
    for (identity, document) in request.packages {
        // A document the evidence already holds is the same document: the
        // reader refuses a conflicting one when the base is read back.
        let _ = evidence.insert_domain_package_document(identity.as_str(), document.as_slice());
    }
    let premises = plan.root_premises();
    let buffers: Vec<(usize, u64)> = plan
        .buffers()
        .iter()
        .map(|(id, capacity)| (id.index(), *capacity))
        .collect();
    let key = node.identity();
    let wire = MonitorPlanWire {
        clause: &key,
        formula: plan.program().formula(),
        counter: &premises.counter,
        counter_width_bits: premises.counter_width_bits,
        max_reading_gap: premises.max_reading_gap,
        max_event_rate: &premises.max_event_rate,
        uncertainty: &premises.uncertainty,
        buffers: &buffers,
        tick_unit: plan.binding().unit(),
        activation_capacity: premises.activation_capacity,
        state: names::STATE,
        inputs: names::INPUTS,
        scope: names::SCOPE,
        result: names::RESULT,
        entries: [
            names::INIT,
            names::ON_EVENT,
            names::ON_WATERMARK,
            names::ON_GAP,
        ],
        renamed_records: &["Instance", "Activation", "LimitReached"],
        renames: &WIRE_FIELD_NAMES,
    };
    let emission = emit_monitor_plan_over(
        emitted.package(),
        request.package_name.clone(),
        &evidence,
        &wire,
    )?;
    Ok(PlanRoot {
        plan,
        obligations,
        emission,
    })
}
