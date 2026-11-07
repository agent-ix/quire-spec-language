// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-283: the `monitor` operation. It checks a supplied trace offline
//! against temporal clauses with the layer-5 trace evaluator replay uses
//! (ADR-029 OP-3, ADR-014 A-4), and reports one [`MonitorRecord`] per
//! selected clause.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::{Code, StageFailure, Staged};
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_replay::spine::{
    run_package_temporal, ClauseDisposition, ClauseRunReport, OverObject, PackageTemporalRequest,
    TemporalTraceRefusal, TemporalUndefinedCause, TemporalVerdict, TraceInput,
};
use qsl_replay::{ClauseRunNotAnItem, OutcomeDocument, OutcomeItem, OutcomeStage};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::PackageInput;
use qsl_semantics::model::observation::{
    admit_trace_snapshot, AdmissionFailure, AdmissionRecord, DocumentProvision, DocumentRef,
    ObservationLimits,
};
use quire_canonical::{Node, NodeRef};
use quire_exact::{Cancel, ScalarLimits};
use quire_semantic_value::catalog::{CatalogCode, Category, InternalFault};

/// The format member of a trace document.
pub const TRACE_FORMAT: &str = "quire.monitor-trace/1";

/// One selected temporal clause: its declared name and the object its `over`
/// parameter binds, resolved in the observation at position 0.
#[derive(Clone, Debug)]
pub struct MonitorClause {
    /// The clause's declared name.
    pub name: String,
    /// The object the clause's `over` parameter names.
    pub over: OverObject,
}

/// A `monitor` request (FR-283 Inputs).
pub struct MonitorRequest<'a> {
    /// The selected clauses, in request order.
    pub clauses: Vec<MonitorClause>,
    /// The bytes of a `quire.monitor-trace/1` trace document.
    pub trace: &'a [u8],
    /// The snapshot documents the trace's positions name.
    pub snapshots: &'a DocumentProvision,
    /// The domain package bytes admission reads.
    pub packages: &'a PackageInput,
}

/// The limits a `monitor` request takes (FR-283 Inputs).
#[derive(Clone, Copy, Debug)]
pub struct MonitorLimits {
    /// The observation limits; `document_bytes` also bounds the trace
    /// document.
    pub observation: ObservationLimits,
    /// The model-normalization limits admission takes.
    pub normalization: ModelNormalizationLimits,
    /// The evaluator's accounting limits.
    pub accounting: ScalarLimits,
}

/// What a clause run reports (FR-283 Outputs table).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MonitorVerdict {
    /// `Completed(false)`: the clause is violated.
    Violated,
    /// An undefined evaluation, settled as a violation.
    Undefined,
    /// A window aggregate short of data: category undefined.
    InsufficientData,
    /// A finite trace that neither shows a bad prefix nor can show more.
    Pending,
    /// A finite-window clause whose closed window leaves it unresolved.
    Indeterminate,
    /// `Completed(true)`: evidence for this trace only, never `proved`.
    Tested,
    /// A clause with a fairness set, over a trace that carries no
    /// enabledness.
    MissingFairnessPremise,
    /// The accounting budget ran out.
    Incomplete,
    /// A window aggregate outgrew `max_window_values`.
    WindowLimitReached,
    /// A `holds` condition was refused.
    ConditionRefused,
    /// The evaluator refused the trace.
    TraceRefused,
}

/// One selected clause's record, in request order.
#[derive(Debug)]
pub struct MonitorRecord {
    /// The clause's declared name.
    pub clause: String,
    /// The verdict.
    pub verdict: MonitorVerdict,
    /// The O-16 category.
    pub category: Category,
    report: ClauseRunReport,
}

impl MonitorRecord {
    /// The clause run behind this record: its `position`, `counterexample`,
    /// `decisive`, cause and usage.
    pub fn report(&self) -> &ClauseRunReport {
        &self.report
    }

    /// The counterexample of a `Violated` or `Undefined` record
    /// (QSpec FR-364), with `trace_position` equal to the record's position.
    pub fn counterexample(&self) -> Option<&qsl_replay::ObservedTemporalCounterexample> {
        self.report.counterexample.as_ref()
    }

    /// The FR-286 `monitor` item this record renders to, through
    /// [`OutcomeItem::from_clause_run`].
    ///
    /// # Errors
    ///
    /// [`ClauseRunNotAnItem`] for a run no item can carry.
    pub fn item(&self, index: usize) -> Result<OutcomeItem, ClauseRunNotAnItem> {
        OutcomeItem::from_clause_run(
            qsl_foundation::RequestIndex::new(index),
            &self.clause,
            &self.report,
        )
    }
}

/// A completed `monitor` request.
#[derive(Debug)]
pub struct MonitorOutcome {
    /// One record per selected clause, in request order.
    pub records: Vec<MonitorRecord>,
}

impl MonitorOutcome {
    /// The FR-286 `monitor` document of these records.
    ///
    /// # Errors
    ///
    /// [`ClauseRunNotAnItem`] for the first record no item can carry.
    pub fn document(&self) -> Result<OutcomeDocument, ClauseRunNotAnItem> {
        OutcomeDocument::from_clause_runs(
            Some(OutcomeStage::S6a),
            self.records
                .iter()
                .map(|record| (record.clause.as_str(), &record.report)),
        )
    }
}

/// Why a whole `monitor` request was refused; it holds no clause record.
#[derive(Debug)]
pub enum MonitorRefusal {
    /// The trace document is defective.
    TraceDocument {
        /// The catalog record of the first defect.
        record: Box<AdmissionRecord>,
        /// The JSON pointer of that defect.
        pointer: String,
    },
    /// The first trace position whose FR-106 admission failed.
    Admission {
        /// The position's index.
        position: usize,
        /// FR-106's failure.
        failure: Box<AdmissionFailure>,
    },
    /// FR-330 refused a selected clause at stage `admit` for a reason that is
    /// not one position's FR-106 admission: a bounded-profile clause over a
    /// lasso, an `over` that resolves to no object, or a profile and trace
    /// shape that do not fit.
    ClauseRefused {
        /// The selected clause's name.
        clause: String,
        /// FR-330's refusal.
        failure: Box<AdmissionFailure>,
    },
    /// A selected clause names no temporal clause of the package.
    UnknownClause {
        /// The selected name.
        name: String,
    },
}

/// FR-283: check `request.trace` against each selected clause of `package`.
///
/// # Errors
///
/// [`StageFailure::Refused`] for a trace document defect, a position whose
/// admission fails, a clause FR-330 refuses at stage `admit` or an unknown
/// clause; [`StageFailure::Cancelled`] when `cancel` is cancelled;
/// [`StageFailure::Fault`] when admission or the evaluator faults.
pub fn monitor(
    package: &CheckedPackage,
    request: &MonitorRequest<'_>,
    limits: &MonitorLimits,
    cancel: &Cancel,
) -> Result<Staged<MonitorOutcome>, StageFailure<MonitorRefusal>> {
    let trace = read_trace(request.trace, limits.observation.document_bytes).map_err(|defect| {
        let (record, pointer) = *defect;
        let record = Box::new(record);
        StageFailure::Refused(MonitorRefusal::TraceDocument { record, pointer })
    })?;
    let input = trace.input();

    // FR-283 Behavior: every position is admitted, in position order, before
    // any clause is evaluated, so an unknown clause name or an earlier
    // clause's verdict never hides an admission failure.
    for clause in &request.clauses {
        if let Some(cause) = cancel.cause() {
            return Err(StageFailure::Cancelled(cause));
        }
        admit_positions(package, request, limits, &trace, clause)?;
    }

    let mut records = Vec::with_capacity(request.clauses.len());
    for clause in &request.clauses {
        if let Some(cause) = cancel.cause() {
            return Err(StageFailure::Cancelled(cause));
        }
        let report = run_package_temporal(&PackageTemporalRequest {
            package,
            packages: request.packages,
            model_limits: limits.normalization,
            snapshots: request.snapshots,
            observation_limits: limits.observation,
            accounting: limits.accounting,
            name: &clause.name,
            trace: &input,
            over: &clause.over,
        });
        records.push(record_of(clause, report)?);
    }
    Ok(Staged::new(MonitorOutcome { records }))
}

/// The record of `clause`'s run, or the whole-request failure the run's
/// disposition stands for.
fn record_of(
    clause: &MonitorClause,
    report: ClauseRunReport,
) -> Result<MonitorRecord, StageFailure<MonitorRefusal>> {
    let verdict = verdict_of(&clause.name, &report.disposition)?;
    Ok(MonitorRecord {
        clause: clause.name.clone(),
        verdict,
        category: report.category(),
        report,
    })
}

/// The verdict `disposition` stands for, or the whole-request failure.
fn verdict_of(
    clause: &str,
    disposition: &ClauseDisposition,
) -> Result<MonitorVerdict, StageFailure<MonitorRefusal>> {
    use ClauseDisposition as D;
    use TemporalVerdict as V;
    let verdict = match disposition {
        D::EvaluateFault(fault) => return Err(StageFailure::Fault(fault.clone())),
        D::MissingName { name } => {
            return Err(StageFailure::Refused(MonitorRefusal::UnknownClause {
                name: name.clone(),
            }))
        }
        // Every position admitted before the run, so this is FR-330's own
        // admit-stage refusal of the clause, or a fault of admission itself.
        D::Admit(AdmissionFailure::Fault(fault)) => return Err(StageFailure::Fault(fault.clone())),
        D::Admit(failure) => {
            return Err(StageFailure::Refused(MonitorRefusal::ClauseRefused {
                clause: clause.to_owned(),
                failure: Box::new(failure.clone()),
            }))
        }
        D::Temporal(verdict) => match verdict {
            V::Holds => MonitorVerdict::Tested,
            V::Violated { .. } => MonitorVerdict::Violated,
            V::Undefined {
                cause: TemporalUndefinedCause::InsufficientData { .. },
                ..
            } => MonitorVerdict::InsufficientData,
            V::Undefined { .. } => MonitorVerdict::Undefined,
            V::Pending | V::PendingWindow { .. } | V::PendingNextEvent { .. } => {
                MonitorVerdict::Pending
            }
            V::Indeterminate { .. } => MonitorVerdict::Indeterminate,
            V::MissingFairnessPremise { .. } => MonitorVerdict::MissingFairnessPremise,
            V::Incomplete(_) | V::ActivationIncomplete { .. } | V::ProgressLimitReached(_) => {
                MonitorVerdict::Incomplete
            }
            V::WindowLimitReached { .. } => MonitorVerdict::WindowLimitReached,
        },
        D::TemporalRefusal(TemporalTraceRefusal::Fault(fault)) => {
            return Err(StageFailure::Fault(fault.clone()))
        }
        D::TemporalRefusal(_) => MonitorVerdict::TraceRefused,
        D::Evaluate(_) | D::ArgumentRefusal(_) => MonitorVerdict::ConditionRefused,
        other => {
            return Err(StageFailure::Fault(InternalFault::new(
                "monitor",
                match other {
                    D::Compile(_) | D::UnknownLanguage { .. } => "clause-run-compiled-source",
                    _ => "clause-run-disposition-is-a-temporal-result",
                },
            )))
        }
    };
    Ok(verdict)
}

/// Admit every trace position as `clause` reads it, in position order. A
/// clause the package does not declare admits nothing here: its run reports
/// the missing name.
fn admit_positions(
    package: &CheckedPackage,
    request: &MonitorRequest<'_>,
    limits: &MonitorLimits,
    trace: &TraceDocument,
    clause: &MonitorClause,
) -> Result<(), StageFailure<MonitorRefusal>> {
    let graph = package.graph();
    let Some(node) = graph
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == clause.name)
    else {
        return Ok(());
    };
    let named = node.named_populations();
    for (position, document) in trace.positions().enumerate() {
        let admitted = admit_trace_snapshot(
            graph.model_selections(),
            graph.scope().types(),
            request.packages,
            limits.normalization,
            request.snapshots,
            document,
            limits.observation,
            &named,
        );
        match admitted {
            Ok(_) => {}
            Err(AdmissionFailure::Fault(fault)) => return Err(StageFailure::Fault(fault)),
            Err(failure) => {
                return Err(StageFailure::Refused(MonitorRefusal::Admission {
                    position,
                    failure: Box::new(failure),
                }))
            }
        }
    }
    Ok(())
}

/// A trace document read: its prefix and, for a lasso, its loop.
struct TraceDocument {
    prefix: Vec<DocumentRef>,
    repeat: Option<Vec<DocumentRef>>,
}

impl TraceDocument {
    fn input(&self) -> TraceInput {
        match &self.repeat {
            None => TraceInput::Finite(self.prefix.clone()),
            Some(repeat) => TraceInput::Lasso {
                prefix: self.prefix.clone(),
                repeat: repeat.clone(),
            },
        }
    }

    fn positions(&self) -> impl Iterator<Item = &DocumentRef> {
        self.prefix.iter().chain(self.repeat.iter().flatten())
    }
}

type Defect = Box<(AdmissionRecord, String)>;

fn defect(code: Code, cause: &'static str, field: Option<&str>, pointer: &str) -> Defect {
    let mut fields = BTreeMap::new();
    if let Some(field) = field {
        fields.insert("field", field.to_owned());
    }
    Box::new((
        AdmissionRecord {
            catalog: CatalogCode::new(code.as_str(), cause),
            fields,
            path: Vec::new(),
            kernel: None,
        },
        pointer.to_owned(),
    ))
}

/// Read a trace document under FR-283 "Trace document", at its first defect.
fn read_trace(bytes: &[u8], input_bytes: u64) -> Result<TraceDocument, Defect> {
    let actual = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if actual > input_bytes {
        let mut record = defect(Code::StageLimitExceeded, "input-bytes-exceeded", None, "").0;
        record.fields.extend([
            ("kind", "input-bytes-exceeded".to_owned()),
            ("bound", input_bytes.to_string()),
            ("actual", actual.to_string()),
            ("setting", "observation.input_bytes".to_owned()),
        ]);
        return Err(Box::new((record, String::new())));
    }
    let document = quire_canonical::read(bytes, input_bytes)
        .map_err(|_| defect(Code::InvalidRuntimeInput, "invalid-value", None, ""))?;
    let root = document.root();
    let Node::Object(members) = root.node() else {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
            None,
            "",
        ));
    };
    let names: Vec<&str> = members.map(|(name, _)| name).collect();
    if let Some(missing) = ["format", "prefix"]
        .into_iter()
        .find(|name| !names.contains(name))
    {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "missing-member",
            Some(missing),
            "",
        ));
    }
    if let Some(unknown) = names
        .iter()
        .find(|name| !["format", "prefix", "loop"].contains(name))
    {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "unknown-member",
            Some(unknown),
            "",
        ));
    }
    let member = |name: &str| root.get(name);
    let format_ok = member("format")
        .is_some_and(|format| matches!(format.node(), Node::String(text) if text == TRACE_FORMAT));
    if !format_ok {
        return Err(defect(
            Code::UnknownWire,
            "unsupported-wire",
            None,
            "/format",
        ));
    }
    let prefix = read_refs(member("prefix"), "/prefix")?;
    let repeat = member("loop")
        .map(|node| read_refs(Some(node), "/loop"))
        .transpose()?;
    if repeat.is_none() && prefix.is_empty() {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "invalid-value",
            None,
            "/prefix",
        ));
    }
    if repeat.as_ref().is_some_and(Vec::is_empty) {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "invalid-value",
            None,
            "/loop",
        ));
    }
    Ok(TraceDocument { prefix, repeat })
}

fn read_refs(node: Option<NodeRef<'_>>, pointer: &str) -> Result<Vec<DocumentRef>, Defect> {
    let wrong = |at: &str| defect(Code::InvalidRuntimeInput, "wrong-value-kind", None, at);
    let Some(Node::Array(items)) = node.map(|node| node.node()) else {
        return Err(wrong(pointer));
    };
    items
        .enumerate()
        .map(|(index, item)| read_ref(item, &format!("{pointer}/{index}")))
        .collect()
}

/// One `DocumentRef` element: `{"identity": {"authority", "identity"}}`.
/// Reads one document reference object; member names are wire words compared at
/// this JSON boundary.
#[ix_trace_rs::string_edge]
fn read_ref(item: NodeRef<'_>, pointer: &str) -> Result<DocumentRef, Defect> {
    let Node::Object(members) = item.node() else {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
            None,
            pointer,
        ));
    };
    let names: Vec<&str> = members.map(|(name, _)| name).collect();
    let Some(identity) = item.get("identity") else {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "missing-member",
            Some("identity"),
            pointer,
        ));
    };
    if let Some(unknown) = names.iter().find(|name| **name != "identity") {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "unknown-member",
            Some(unknown),
            pointer,
        ));
    }
    let identity_pointer = format!("{pointer}/identity");
    if !matches!(identity.node(), Node::Object(_)) {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
            None,
            &identity_pointer,
        ));
    }
    let label = |name: &'static str| -> Result<String, Defect> {
        match identity.get(name).map(|node| node.node()) {
            Some(Node::String(text)) => Ok(text.to_owned()),
            Some(_) => Err(defect(
                Code::InvalidRuntimeInput,
                "wrong-value-kind",
                None,
                &format!("{identity_pointer}/{name}"),
            )),
            None => Err(defect(
                Code::InvalidRuntimeInput,
                "missing-member",
                Some(name),
                &identity_pointer,
            )),
        }
    };
    let labels = SourceIdentity::new(label("authority")?, label("identity")?);
    if let Some(blank) = labels.first_blank_label() {
        let mut found = defect(
            Code::InvalidSourceIdentity,
            "blank-label",
            None,
            &identity_pointer,
        );
        found.0.fields.insert("label", blank.as_str().to_owned());
        return Err(found);
    }
    let reference = DocumentRef {
        authority: labels.authority,
        identity: labels.identity,
    };
    Ok(reference)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fault() -> InternalFault {
        InternalFault::new("test", "a-broken-invariant")
    }

    fn refused() -> AdmissionFailure {
        AdmissionFailure::Refused(Box::new(AdmissionRecord {
            catalog: CatalogCode::new("invalid_runtime_input", "invalid-value"),
            fields: BTreeMap::new(),
            path: Vec::new(),
            kernel: None,
        }))
    }

    /// FR-283-AC-13, Outputs: a `TemporalRefusal::Fault` fails the whole
    /// request with `StageFailure::Fault`, never a record.
    #[test]
    fn a_trace_evaluator_fault_fails_the_request() {
        let disposition = ClauseDisposition::TemporalRefusal(TemporalTraceRefusal::Fault(fault()));
        assert!(matches!(
            verdict_of("P", &disposition),
            Err(StageFailure::Fault(_))
        ));
        let disposition = ClauseDisposition::TemporalRefusal(TemporalTraceRefusal::EmptyLoop);
        assert!(matches!(
            verdict_of("P", &disposition),
            Ok(MonitorVerdict::TraceRefused)
        ));
    }

    /// FR-330: an admit-stage refusal that is no position's admission names
    /// its clause, and an admission fault fails the request.
    #[test]
    fn a_clause_admit_refusal_names_its_clause_and_a_fault_is_a_fault() {
        let Err(StageFailure::Refused(MonitorRefusal::ClauseRefused { clause, .. })) =
            verdict_of("P", &ClauseDisposition::Admit(refused()))
        else {
            panic!("a clause refusal");
        };
        assert_eq!(clause, "P");
        assert!(matches!(
            verdict_of(
                "P",
                &ClauseDisposition::Admit(AdmissionFailure::Fault(fault()))
            ),
            Err(StageFailure::Fault(_))
        ));
    }
}
