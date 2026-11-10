// SPDX-License-Identifier: AGPL-3.0-or-later
//! Running a case through QSL's library operation (FR-565).

use qsl_analyze::analyze::{
    analyze, AnalyzeLimits, AnalyzeRequest, ClaimItem, Diagnostic, Engines, ItemForm, ItemOptions,
    NegotiatedItem, Payload, ProbabilisticItem, ProbabilisticLimits, SourceProvision,
    SubjectBinding, TemporalItem,
};
use qsl_analyze::exact_probabilistic::ExactProbLimits;
use qsl_analyze::monitor::{monitor, MonitorClause, MonitorLimits, MonitorRequest, MonitorTrace};
use qsl_analyze::provider;
use qsl_eval::execution::{
    execute, ExecuteLimits, ExecuteRequest, ExecuteSelection, PrepareLimits, SuppliedBackends,
};
use qsl_eval::simulation::model::PopulationUniverse;
use qsl_eval::value::QualifiedName;
use qsl_foundation::diagnostic::{StageFailure, Staged};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{BackendId, RequestIndex, SourceIdentity};
use qsl_replay::backends::run_with_backends;
use qsl_replay::spine::{
    check, package, parse, select, AdmittedModels, Call, CallArgument, CheckedUnit,
    DependencyInput, FrontEndFailure, OverObject, PackageLimits, ParseRequest, ParsedUnitSource,
    SpineLimits,
};
use qsl_replay::{
    IncompleteCause, ObjectEnvironment, Operation, OutcomeDocument, OutcomeItem, TerminalRecord,
};
use qsl_route::request::ExtentClassification;
use qsl_route::routing::Disposition;
use qsl_route::{Candidate, EvidenceKind, Registry};
use qsl_semantics::check::Capability;
use qsl_semantics::model::intake::{package_input, PackageInput};
use qsl_semantics::model::observation::{document_reference, DocumentProvision, DocumentRef};
use quire_checked::key::DeclarationKey;
use quire_eval::{CallFailure, Evaluation, ExecutionBackendChoice};
use quire_exact::{Cancel, Integer, Value};
use quire_semantic_value::catalog::CatalogCode;
use quire_semantic_value::checking::CheckingLimits;

use crate::fault_backend::FaultInjectingBackend;
use crate::matrix::{
    AnalyzeInput, ArgumentValue, BackendPlan, Cancellation, CaseInput, EngineItem, ExecuteArgument,
    ExecuteInput, FrontEndInput, FrontEndLimit, Input, MatrixCase, MonitorInput, PluginSettlement,
    ProveInput, RequestParameters, SubjectInput,
};

/// The typed outcome of a case: an FR-286 document, or for a `prove` case
/// the FR-286 item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Report {
    /// The operation's outcome document.
    Document(OutcomeDocument),
    /// The one item a `prove` case yields.
    Item(Box<OutcomeItem>),
}

/// What running a case returns: the typed outcome and its FR-285 exit code.
///
/// The comparison surface between the library and the CLI is the typed
/// document, which the driver encodes with its own canonical encoder
/// (FR-565); this crate writes no bytes of its own.
#[derive(Clone, Debug)]
pub struct CaseOutcome {
    report: Report,
    records: Vec<TerminalRecord>,
    execution: Option<ExecuteResult>,
    exit_code: u8,
}

/// The typed library result of an `execute` case (FR-279, FR-564-AC-3): what
/// `qsl_eval::execution::execute` returns for the case's request, before the
/// FR-286 document is written. It has no equality: `Evaluation` and the
/// completed `Value` have none, and a `Debug` rendering is not an equality
/// contract, so a caller compares the `CallFailure` (which has one) or the
/// document.
#[derive(Clone, Debug)]
pub struct ExecuteResult(std::sync::Arc<Result<Evaluation, CallFailure>>);

impl ExecuteResult {
    /// The `Evaluation` the backend returned, or the `CallFailure`.
    pub fn result(&self) -> &Result<Evaluation, CallFailure> {
        &self.0
    }
}

/// Two outcomes are equal when their documents or items, terminal records,
/// exit codes and typed `execute` failures are equal. A completed `execute`
/// result has no equality of its own, so two completed results are equal
/// through the documents, which hold the same value.
impl PartialEq for CaseOutcome {
    fn eq(&self, other: &Self) -> bool {
        let failure = |outcome: &Self| {
            outcome
                .execution
                .as_ref()
                .map(|execution| execution.0.as_ref().as_ref().err().cloned())
        };
        self.report == other.report
            && self.records == other.records
            && self.exit_code == other.exit_code
            && failure(self) == failure(other)
    }
}

impl Eq for CaseOutcome {}

impl CaseOutcome {
    fn of(report: Report, records: Vec<TerminalRecord>, execution: Option<ExecuteResult>) -> Self {
        let category = match &report {
            Report::Document(document) => document.category(),
            Report::Item(item) => item.category(),
        };
        Self {
            report,
            records,
            execution,
            exit_code: category.exit_code(),
        }
    }

    /// The typed document, or for a `prove` case the item.
    #[must_use]
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// The terminal records an `analyze` or `prove` case's engines settled,
    /// in request order; empty for every other verb and for a stage failure.
    #[must_use]
    pub fn records(&self) -> &[TerminalRecord] {
        &self.records
    }

    /// The typed `execute` result of an `execute` case; `None` for every
    /// other verb.
    #[must_use]
    pub fn execution(&self) -> Option<&ExecuteResult> {
        self.execution.as_ref()
    }

    /// The O-16 category of the document or item.
    #[must_use]
    pub fn category(&self) -> quire_semantic_value::catalog::Category {
        match &self.report {
            Report::Document(document) => document.category(),
            Report::Item(item) => item.category(),
        }
    }

    /// The FR-285 exit code of the outcome's category.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

/// The stage of running a case that failed (FR-565).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaseStage {
    /// Reading the case's inputs: its unit, packages and snapshot documents.
    Read,
    /// Checking the case's unit, or preparing the claim or subject checked
    /// from it.
    Check,
    /// The `package` operation.
    Package,
    /// The `execute` operation.
    Execute,
    /// The `analyze` operation.
    Analyze,
    /// The `monitor` operation.
    Monitor,
    /// The `prove` operation.
    Prove,
    /// Building the typed document or item from the operation's records.
    Document,
}

/// Why a case could not be run to an outcome: the failure of running it as
/// such, not an outcome of its operation, which is never an error whatever
/// its category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("running the case failed at stage {stage:?}")]
pub struct CaseError {
    /// The stage that failed.
    pub stage: CaseStage,
}

impl CaseError {
    const fn at(stage: CaseStage) -> Self {
        Self { stage }
    }
}

/// Run `case` through QSL's library operation with an empty registry and
/// `Engines::qsl()`: the first-slice configuration FR-566 compares the
/// others to. The driver passes its own registry to [`run_case_with`].
///
/// # Errors
///
/// [`CaseError`] when the case's data does not reach the operation under
/// test.
pub fn run_case(case: &MatrixCase) -> Result<CaseOutcome, CaseError> {
    let engines = Engines::qsl().map_err(|_| CaseError::at(CaseStage::Analyze))?;
    run_case_with(case, &Registry::new(), &engines)
}

/// Run `case` with the caller's registry and engines. A case whose
/// operation takes neither ignores them.
///
/// # Errors
///
/// As [`run_case`].
pub fn run_case_with(
    case: &MatrixCase,
    registry: &Registry,
    engines: &Engines,
) -> Result<CaseOutcome, CaseError> {
    // The cases here route their items by hand, so layer R's registry has no
    // part in them; FR-566 holds the outcome unchanged whatever it holds.
    let _ = registry;
    let cancel = operation_handle(case.cancellation);
    let parameters = &case.parameters;
    let mut records = Vec::new();
    let mut execution = None;
    let report = match &case.input {
        CaseInput::Check(input) => front_end(input, false, parameters, &cancel)?,
        CaseInput::Package(input) => front_end(input, true, parameters, &cancel)?,
        CaseInput::Execute(input) => run_execute(input, parameters, &cancel, &mut execution)?,
        CaseInput::Analyze(input) => {
            run_analyze(input, parameters, engines, &cancel, &mut records)?
        }
        CaseInput::Monitor(input) => run_monitor(input, parameters, &cancel)?,
        CaseInput::Prove(input) => run_prove(input, parameters, engines, &cancel, &mut records)?,
    };
    Ok(CaseOutcome::of(report, records, execution))
}

/// The handle the operation under test is called with: cancelled before the
/// call when the plan says so (FR-564: no timing is involved).
fn operation_handle(plan: Cancellation) -> Cancel {
    let cancel = Cancel::new();
    if let Cancellation::BeforeCall(cause) = plan {
        cancel.cancel(cause);
    }
    cancel
}

fn prerequisite<E>(stage: CaseStage) -> impl Fn(E) -> CaseError {
    move |_| CaseError::at(stage)
}

/// A unit parsed and its models selected, at default limits and with an
/// uncancelled handle: the stages before the operation under test.
struct Prepared {
    package_name: &'static str,
    parsed: ParsedUnitSource,
    models: AdmittedModels,
    packages: PackageInput,
}

impl Prepared {
    fn new(
        unit: &Input,
        packages: &[Input],
        parameters: &RequestParameters,
    ) -> Result<Self, CaseError> {
        let uncancelled = Cancel::new();
        let defaults = SpineLimits::default();
        let identity = SourceIdentity::new(parameters.source_authority, parameters.source_identity);
        let parsed = parse(
            &ParseRequest {
                source: &identity,
                path: unit.file_name,
                bytes: unit.bytes,
            },
            defaults.source,
            &uncancelled,
        )
        .map_err(prerequisite(CaseStage::Read))?
        .into_value();
        let packages = package_input(packages.iter().map(|package| package.bytes))
            .map_err(prerequisite(CaseStage::Read))?;
        let models = select(&parsed, &packages, defaults.model, &uncancelled)
            .map_err(prerequisite(CaseStage::Check))?
            .into_value();
        Ok(Self {
            package_name: parameters.package_name,
            parsed,
            models,
            packages,
        })
    }

    fn check(
        &self,
        limits: SpineLimits,
        cancel: &Cancel,
    ) -> Result<Staged<CheckedUnit>, FrontEndFailure> {
        check(
            &self.parsed,
            self.package_name.to_owned(),
            &self.models,
            &DependencyInput::default(),
            limits,
            cancel,
        )
    }

    /// The unit checked at default limits with an uncancelled handle.
    fn checked(self) -> Result<(CheckedUnit, PackageInput), CaseError> {
        let checked = self
            .check(SpineLimits::default(), &Cancel::new())
            .map_err(prerequisite(CaseStage::Check))?
            .into_value();
        Ok((checked, self.packages))
    }
}

/// `check` or `package` over the case's unit.
fn front_end(
    input: &FrontEndInput,
    packaged: bool,
    parameters: &RequestParameters,
    cancel: &Cancel,
) -> Result<Report, CaseError> {
    let defaults = SpineLimits::default();
    let prepared = Prepared::new(&input.unit, input.packages, parameters)?;
    if !packaged {
        let limits = match input.limit {
            FrontEndLimit::S3Nodes(nodes) => SpineLimits {
                checking: CheckingLimits::new(nodes)
                    .with_input_bytes(defaults.checking.input_bytes())
                    .with_work_budget(defaults.checking.work_budget()),
                ..defaults
            },
            FrontEndLimit::Default | FrontEndLimit::OutputBytes(_) => defaults,
        };
        let checked = prepared.check(limits, cancel);
        return Ok(Report::Document(OutcomeDocument::from_check(&checked)));
    }
    let (checked, _) = prepared.checked()?;
    let limits = match input.limit {
        FrontEndLimit::OutputBytes(bytes) => PackageLimits::default().with_output_bytes(bytes),
        FrontEndLimit::Default | FrontEndLimit::S3Nodes(_) => PackageLimits::default(),
    };
    let emitted = package(&checked, limits, cancel);
    Ok(Report::Document(OutcomeDocument::from_package(&emitted)))
}

/// `execute` over the case's unit: the typed result of
/// `qsl_eval::execution::execute` over the checked package (FR-279), and the
/// FR-286 document of the same call through [`run_with_backends`], the
/// function-call path the `quire` CLI takes (FR-296). The typed call runs over
/// a package checked with an uncancelled handle, so a pre-cancelled handle
/// reaches the seam as `CallFailure::Cancelled`.
fn run_execute(
    input: &ExecuteInput,
    parameters: &RequestParameters,
    cancel: &Cancel,
    typed: &mut Option<ExecuteResult>,
) -> Result<Report, CaseError> {
    let accounting = parameters.accounting;
    let choice = match input.backend {
        BackendPlan::Interpreter => ExecutionBackendChoice::Interpreter,
        BackendPlan::JitNotSupplied => ExecutionBackendChoice::Jit,
        BackendPlan::FaultInjecting => ExecutionBackendChoice::Aot,
    };
    let fault_injecting = FaultInjectingBackend;
    let backends = match input.backend {
        BackendPlan::FaultInjecting => SuppliedBackends {
            aot: Some(&fault_injecting),
            jit: None,
        },
        BackendPlan::Interpreter | BackendPlan::JitNotSupplied => SuppliedBackends::default(),
    };
    let value = |argument: &ExecuteArgument| match argument.value {
        ArgumentValue::Integer(value) => Value::Integer(Integer::from(value)),
        ArgumentValue::Boolean(value) => Value::Boolean(value),
    };

    let (checked, _) = Prepared::new(&input.unit, &[], parameters)?.checked()?;
    let function =
        QualifiedName::unqualified(input.function).map_err(prerequisite(CaseStage::Execute))?;
    let request = ExecuteRequest {
        selection: ExecuteSelection::Function {
            name: function,
            arguments: input.arguments.iter().map(value).collect(),
            environment: ObjectEnvironment::default(),
        },
        backend: choice,
    };
    let limits = ExecuteLimits {
        accounting,
        supplied: Default::default(),
        prepare: PrepareLimits::default(),
    };
    *typed = Some(ExecuteResult(std::sync::Arc::new(execute(
        checked.package(),
        &request,
        &backends,
        &limits,
        cancel,
    ))));

    let call = Call {
        function: input.function.to_owned(),
        arguments: input
            .arguments
            .iter()
            .map(|argument| CallArgument {
                parameter: argument.parameter.to_owned(),
                value: value(argument),
            })
            .collect(),
        accounting,
        supplied: Default::default(),
        backend: choice,
    };
    let result = run_with_backends(
        SourceIdentity::new(parameters.source_authority, parameters.source_identity),
        parameters.package_name.to_owned(),
        input.unit.file_name,
        input.unit.bytes,
        &PackageInput::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        &call,
        &ObjectEnvironment::default(),
        &backends,
        &PrepareLimits::default(),
        cancel,
    );
    Ok(Report::Document(OutcomeDocument::from_run(&result)))
}

/// The snapshot documents by the reference each names itself with.
fn snapshot_provision(documents: &[Input]) -> Result<DocumentProvision, CaseError> {
    documents
        .iter()
        .map(|document| {
            let reference =
                document_reference(document.bytes).ok_or(CaseError::at(CaseStage::Read))?;
            Ok((reference, document.bytes.to_vec()))
        })
        .collect()
}

/// A checked model subject and the members of its binding.
struct Subject {
    parameters: RequestParameters,
    unit: &'static [u8],
    checked: CheckedUnit,
    packages: PackageInput,
    snapshots: DocumentProvision,
    initial: Vec<DocumentRef>,
    universes: Vec<PopulationUniverse>,
}

impl Subject {
    fn new(input: &SubjectInput, parameters: &RequestParameters) -> Result<Self, CaseError> {
        let (checked, packages) =
            Prepared::new(&input.unit, input.packages, parameters)?.checked()?;
        Ok(Self {
            parameters: *parameters,
            unit: input.unit.bytes,
            checked,
            packages,
            snapshots: snapshot_provision(input.snapshots)?,
            initial: input
                .initial
                .iter()
                .map(|(authority, identity)| DocumentRef {
                    authority: (*authority).to_owned(),
                    identity: (*identity).to_owned(),
                })
                .collect(),
            universes: vec![PopulationUniverse {
                population: DeclarationKey {
                    package: input.population.package.to_owned(),
                    node: input.population.node.to_owned(),
                },
                keys: input
                    .population
                    .keys
                    .iter()
                    .map(|key| (*key).to_owned())
                    .collect(),
            }],
        })
    }

    fn binding(&self) -> SubjectBinding<'_> {
        SubjectBinding {
            packages: &self.packages,
            snapshots: &self.snapshots,
            initial: self.initial.clone(),
            universes: self.universes.clone(),
        }
    }

    /// The source bytes of the package under the labels it names.
    fn provision(&self) -> SourceProvision {
        SourceProvision::single(self.checked.package().graph().source(), self.unit)
    }

    /// The item, negotiated by the case: an item for the engine that runs it,
    /// or (FR-281-AC-3) one whose disposition is `unsupported`.
    fn work(&self, item: &EngineItem) -> Result<NegotiatedItem, CaseError> {
        let graph = self.checked.package().graph();
        let claim = |name: &str| {
            graph
                .probabilistic_model()
                .claim(name)
                .and_then(|claim| claim.occurrence().cloned())
                .ok_or(CaseError::at(CaseStage::Check))
        };
        let request_index = RequestIndex::new(0);
        let parameters = &self.parameters;
        let supported = |backend: &str, claim, kind, extent, form| {
            let candidate = Candidate::new(BackendId::new(backend));
            NegotiatedItem {
                item: ClaimItem {
                    request_index,
                    claim,
                    kind,
                    extent,
                    backend: Some(BackendId::new(backend)),
                    candidates: vec![candidate.clone()],
                    options: ItemOptions::default(),
                    form: Some(form),
                },
                disposition: Disposition::Supported(candidate),
                diagnostic: None,
            }
        };
        let unbounded = ExtentClassification::Unbounded {
            finite_bound_available: true,
        };
        let temporal = |max_states: Option<usize>| -> Result<_, CaseError> {
            let [node] = graph.temporal_clauses() else {
                return Err(CaseError::at(CaseStage::Check));
            };
            let mut limits = qsl_replay::ModelCheckLimits::default();
            if let Some(max_states) = max_states {
                limits.limits.max_states = max_states;
            }
            Ok((
                OccurrenceKey::new(node.identity(), node.claim().clone()),
                ItemForm::Temporal(TemporalItem {
                    limits,
                    ..TemporalItem::default()
                }),
            ))
        };
        Ok(match item {
            EngineItem::Statistical { claim: name } => supported(
                "en-4",
                claim(name)?,
                Capability::ProbabilisticSatisfaction,
                unbounded,
                ItemForm::Probabilistic(ProbabilisticItem {
                    evidence: EvidenceKind::Statistical,
                    limits: ProbabilisticLimits::Statistical(
                        qsl_replay::statistical::StatisticalLimits::default(),
                    ),
                    method: Some(qsl_replay::statistical::StatisticalMethod::Okamoto),
                    seed: Some(parameters.statistical_seed),
                    sampler: Some(
                        qsl_foundation::selection::DefinitionRef::new(
                            parameters.sampler.0,
                            parameters.sampler.1,
                        )
                        .map_err(prerequisite(CaseStage::Check))?,
                    ),
                    max_witnesses: Some(parameters.max_witnesses),
                    symmetry: Vec::new(),
                    timed_sampling: None,
                }),
            ),
            EngineItem::Clause => {
                let (key, form) = temporal(parameters.max_states)?;
                supported(
                    "en-1",
                    key,
                    Capability::TemporalSatisfaction,
                    ExtentClassification::Bounded,
                    form,
                )
            }
            EngineItem::UnsupportedClause => {
                let (key, form) = temporal(parameters.max_states)?;
                let mut negotiated = supported(
                    "en-1",
                    key,
                    Capability::TemporalSatisfaction,
                    ExtentClassification::Bounded,
                    form,
                );
                negotiated.item.candidates.clear();
                negotiated.item.backend = None;
                negotiated.disposition = Disposition::Unsupported;
                negotiated.diagnostic = Some(Diagnostic {
                    code: CatalogCode::new(
                        "unsupported_projection",
                        "unsupported-requested-capability",
                    ),
                    payload: Payload::Kind(Capability::TemporalSatisfaction),
                });
                negotiated
            }
            EngineItem::Exact { claim: name } => {
                let mut limits = ExactProbLimits::default();
                if let Some(max_states) = parameters.max_states {
                    limits.model.limits.max_states = max_states;
                }
                supported(
                    "en-5",
                    claim(name)?,
                    Capability::ProbabilisticSatisfaction,
                    unbounded,
                    ItemForm::Probabilistic(ProbabilisticItem {
                        evidence: EvidenceKind::Exact,
                        limits: ProbabilisticLimits::Exact(limits),
                        method: None,
                        seed: None,
                        sampler: None,
                        max_witnesses: None,
                        symmetry: Vec::new(),
                        timed_sampling: None,
                    }),
                )
            }
        })
    }
}

/// The document of a stage that stopped without a result.
fn stopped_document<C>(
    operation: Operation,
    failure: StageFailure<C>,
    refused: impl FnOnce(&C) -> OutcomeDocument,
) -> OutcomeDocument {
    match failure.into_refused() {
        Ok(cause) => refused(&cause),
        Err(stopped) => OutcomeDocument::from_stopped(operation, stopped),
    }
}

fn run_analyze(
    input: &AnalyzeInput,
    parameters: &RequestParameters,
    engines: &Engines,
    cancel: &Cancel,
    settled: &mut Vec<TerminalRecord>,
) -> Result<Report, CaseError> {
    let document = match input {
        AnalyzeInput::Engine {
            subject,
            item,
            deadline,
        } => {
            let subject = Subject::new(subject, parameters)?;
            let request = AnalyzeRequest {
                binding: subject.binding(),
                items: vec![subject.work(item)?],
                provision: subject.provision(),
            };
            let limits = AnalyzeLimits {
                accounting: parameters.accounting,
                deadline: *deadline,
                ..AnalyzeLimits::unbounded()
            };
            match analyze(
                subject.checked.package(),
                &request,
                engines,
                &limits,
                cancel,
            ) {
                Ok(staged) => {
                    settled.extend(staged.value().records.iter().cloned());
                    staged.value().document()
                }
                Err(failure) => {
                    stopped_document(Operation::Analyze, failure, |refusal| refusal.document())
                }
            }
        }
    };
    Ok(Report::Document(document))
}

fn run_monitor(
    input: &MonitorInput,
    parameters: &RequestParameters,
    cancel: &Cancel,
) -> Result<Report, CaseError> {
    let (checked, packages) = Prepared::new(&input.unit, input.packages, parameters)?.checked()?;
    let snapshots = snapshot_provision(input.snapshots)?;
    let request = MonitorRequest {
        clauses: input
            .clauses
            .iter()
            .map(|name| MonitorClause {
                name: (*name).to_owned(),
                over: Some(OverObject {
                    population: input.population.to_owned(),
                    key: input.key.to_owned(),
                }),
            })
            .collect(),
        trace: MonitorTrace::Document(input.trace.bytes),
        snapshots: &snapshots,
        packages: &packages,
        sources: &[],
        temporal_selections: &[],
    };
    let defaults = SpineLimits::default();
    let limits = MonitorLimits {
        observation: Default::default(),
        normalization: defaults.model,
        accounting: parameters.accounting,
        max_window_values: parameters.max_window_values,
    };
    let document = match monitor(checked.package(), &request, &limits, cancel) {
        Ok(staged) => staged
            .value()
            .document()
            .map_err(|_| CaseError::at(CaseStage::Document))?,
        Err(failure) => stopped_document(Operation::Monitor, failure, |refusal| refusal.document()),
    };
    Ok(Report::Document(document))
}

fn run_prove(
    input: &ProveInput,
    parameters: &RequestParameters,
    engines: &Engines,
    cancel: &Cancel,
    settled: &mut Vec<TerminalRecord>,
) -> Result<Report, CaseError> {
    let record = match input {
        ProveInput::Provider { subject, item } => {
            let subject = Subject::new(subject, parameters)?;
            let limits = AnalyzeLimits {
                accounting: parameters.accounting,
                ..AnalyzeLimits::unbounded()
            };
            let records = provider::run(
                subject.checked.package(),
                &subject.binding(),
                &subject.provision(),
                &[subject.work(item)?],
                engines,
                &limits,
                cancel,
            )
            .records;
            let [record] = <[TerminalRecord; 1]>::try_from(records)
                .map_err(|_| CaseError::at(CaseStage::Document))?;
            record
        }
        ProveInput::Plugin(settlement) => plugin_record(*settlement),
    };
    let item = OutcomeItem::from_terminal(&record);
    settled.push(record);
    Ok(Report::Item(Box::new(item)))
}

/// FR-291: the record QSL builds from a plugin run's settlement.
fn plugin_record(settlement: PluginSettlement) -> TerminalRecord {
    let index = RequestIndex::new(0);
    let backend = BackendId::new("qsl.test-plugin");
    match settlement {
        PluginSettlement::TimedOut => {
            TerminalRecord::incomplete(index, backend, IncompleteCause::TimedOut)
        }
        PluginSettlement::Cancelled(source) => {
            TerminalRecord::incomplete(index, backend, IncompleteCause::Cancelled { source })
        }
        PluginSettlement::Failed(failure) => TerminalRecord::tool_failed(index, backend, failure),
    }
}
