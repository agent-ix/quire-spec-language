// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-100, FR-945, FR-1039: a `native-run/1` request that selects an existing
//! `filament.checked-package/v2` artifact through `package` in place of a
//! `program`.
//!
//! The strict checked-package reader admits the selected bytes against the
//! request's domain packages; the package's `lock` supplies the edition,
//! profiles and laws (the integer-division law included, FR-767) and its name
//! rows the function the `call` selects. The call runs over the
//! [`fsl_package::BarePackage`] of the admitted package. No source is read or
//! compiled, and no refusal falls back to source.

use std::path::Path;

use super::{output, wire, FileCounts, Result, RunCause, RunResult};

/// The FR-517 files a `{directory}` selection reads.
const PACKAGE_FILE: &str = "checked-package.json";
const IDENTITIES_FILE: &str = "identities.json";
const EQUALITY_FILE: &str = "equality.json";

/// The contract version the package route runs (FR-1039).
const CONTRACT_VERSION: &str = "filament.checked-package/v2";

/// A prepared call over a selected checked package: the admitted package,
/// its handoff witness rows when the selection is a directory, and the call.
pub struct PreparedPackageRun {
    dependencies: Vec<fsl_package::DependencyArtifact>,
    package: filament_contract_model::CheckedPackageV2,
    identities: Option<fsl_package::ReadIdentityRows>,
    equality: Option<fsl_package::ReadEqualityRows>,
    budget: filament_contract_model::NativeJoinBudget,
    packages: fsl_semantics::model::intake::PackageInput,
    model_limits: fsl_semantics::model::accounting::ModelNormalizationLimits,
    work: Work,
    /// The request's `package` selection, echoed by the result.
    selection: serde_json::Value,
    /// The domain package sources the request supplied.
    models: Vec<fsl_foundation::Source>,
}

/// What a package run does once the package is admitted.
enum Work {
    /// FR-945: call a function the package names.
    Call(Box<fsl_replay::spine::Call>),
    /// FR-1039: lower a temporal clause the package names (FR-931). The
    /// bytes are the `cross_evaluate.initial` snapshot.
    Lowering(super::tl::TlLowering, Option<Vec<u8>>),
    /// FR-1039: run a temporal clause the package names over record samples.
    Clause(Box<ClauseWork>),
}

/// A `clause` run over a selected package: the clause, the trace it reads and
/// the selection the result echoes.
struct ClauseWork {
    name: String,
    trace: fsl_replay::spine::TraceInput,
    /// The object the clause's `over` parameter names, for a trace of
    /// snapshots; `None` for a trace of record samples.
    over: Option<fsl_replay::spine::OverObject>,
    /// The progress ledger the request carries.
    ledger: Option<fsl_replay::spine::ProgressLedger>,
    /// The snapshot documents the trace names.
    snapshots: fsl_semantics::model::observation::DocumentProvision,
    observation_limits: fsl_semantics::model::observation::ObservationLimits,
    selection: fsl_replay::spine::ClauseRunSelection,
    accounting: filament_exact::ScalarLimits,
}

/// The package bytes and handoff rows admitted for a selected package. The
/// export route shares this reader so package execution and mapping apply the
/// same strict V2 admission and file/directory handoff rules.
pub(super) struct AdmittedPackage {
    pub(super) dependencies: Vec<fsl_package::DependencyArtifact>,
    pub(super) package: filament_contract_model::CheckedPackageV2,
    pub(super) identities: Option<fsl_package::ReadIdentityRows>,
    pub(super) equality: Option<fsl_package::ReadEqualityRows>,
    pub(super) budget: filament_contract_model::NativeJoinBudget,
    pub(super) packages: fsl_semantics::model::intake::PackageInput,
    pub(super) model_limits: fsl_semantics::model::accounting::ModelNormalizationLimits,
    pub(super) snapshot: Option<Vec<u8>>,
    /// The observation files [`admit_files`] read, in the order it was given.
    pub(super) observations: Vec<Vec<u8>>,
    pub(super) selection: serde_json::Value,
    pub(super) models: Vec<fsl_foundation::Source>,
}

impl PreparedPackageRun {
    pub(super) fn sources(&self) -> crate::Sources {
        crate::Sources::from_sources(self.models.iter().cloned())
    }
}

fn invalid(member: &str, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: member.to_owned(),
        reason: reason.into(),
    }
}

/// The request-stage refusal of a package the strict reader refused, with
/// the reader's pointer under `/request/package` (FR-1039).
fn package_refusal(refusal: fsl_package::PackageAdmissionRefusal) -> RunCause {
    let fsl_package::PackageAdmissionRefusal {
        code,
        cause,
        pointer,
        identity,
        ..
    } = refusal;
    let label = match &cause {
        Some(cause) => format!("{}/{cause}", code.as_str()),
        None => code.as_str().to_owned(),
    };
    if let Some(identity) = identity {
        // FR-1039: a recorded model selection with no `models` entry.
        return RunCause::AnalyzePackage {
            code,
            cause,
            pointer: Some("/request/models".to_owned()),
            message: format!("{label} at /request/models: no models entry supplies {identity}"),
        };
    }
    let pointer = format!("/request/package{}", pointer.unwrap_or_default());
    RunCause::AnalyzePackage {
        code,
        cause,
        message: format!("{label} at {pointer}"),
        pointer: Some(pointer),
    }
}

/// A selected path beneath the request's directory.
fn beneath(member: &str, path: &str) -> Result<String> {
    use std::path::Component;
    if path.is_empty()
        || Path::new(path).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(invalid(
            member,
            "must name a path beneath the request directory",
        ));
    }
    Ok(path.trim_end_matches('/').to_owned())
}

/// Read a `package` request (FR-1039 "Behavior"): every member that would
/// restate the package's records refuses before any selected file is opened;
/// then the package and the request's domain packages are read and the
/// strict reader admits the package.
pub(super) fn prepare(
    directory: &Path,
    request: &wire::Request,
    package: &wire::RunPackageSelection,
    dependent_bytes: usize,
) -> Result<PreparedPackageRun> {
    validate_request(request)?;
    if package.source_package.is_some() {
        return Err(invalid(
            "package.source_package",
            "source_package is only selected with package export",
        ));
    }
    if request.clause.is_some() && package.file.is_some() {
        return Err(RunCause::RequiresDirectory {
            member: "clause".to_owned(),
        });
    }
    for (present, member) in [
        (request.items.is_some(), "items"),
        (request.verification_plan.is_some(), "verification_plan"),
        (request.model_based_test.is_some(), "model_based_test"),
    ] {
        if present {
            return Err(invalid(
                member,
                "running this member over a selected checked package is not implemented yet",
            ));
        }
    }
    if let Some(clause) = &request.clause {
        return prepare_clause(directory, request, package, clause, dependent_bytes);
    }
    if let Some(member) = &request.tl_lowering {
        return prepare_lowering(directory, request, package, member, dependent_bytes);
    }
    let Some(call) = &request.call else {
        return Err(invalid("call", "a package run names its call"));
    };
    if call.requirements.is_some() {
        return Err(invalid(
            "call.requirements",
            "checked-package requirement items need the V2 requirement record from QSL-2543",
        ));
    }
    let mut spine_call = super::read_call(call, request)?;
    let admitted = admit(
        directory,
        request,
        package,
        dependent_bytes,
        call.snapshot.as_deref(),
    )?;
    if let Some(bytes) = &admitted.snapshot {
        let limits = fsl_replay::spine::ObservationLimitsWire::resolve(None)
            .map_err(|error| invalid(&error.member, error.reason))?;
        let observation =
            fsl_replay::spine::SuppliedObservation::from_document("snapshot", bytes.clone())
                .map_err(|error| invalid(&error.member, error.reason))?;
        spine_call.snapshot = Some(fsl_replay::spine::CallSnapshot {
            observation,
            limits,
        });
    }
    Ok(PreparedPackageRun {
        package: admitted.package,
        dependencies: admitted.dependencies,
        identities: admitted.identities,
        equality: admitted.equality,
        budget: admitted.budget,
        packages: admitted.packages,
        model_limits: admitted.model_limits,
        work: Work::Call(Box::new(spine_call)),
        selection: admitted.selection,
        models: admitted.models,
    })
}

/// FR-1039: prepare a `clause` request over a selected package. The package
/// form reads a `temporal` selection of a clause: over a finite trace or a
/// lasso of record samples, or over snapshots of its model world (a finite
/// trace, a lasso or a model-arm behaviour). Any other selection refuses here,
/// before a file is opened. A run builds states over the package's model
/// world, so a `{file}` selection refuses it.
fn prepare_clause(
    directory: &Path,
    request: &wire::Request,
    package: &wire::RunPackageSelection,
    clause: &wire::ClauseRun,
    dependent_bytes: usize,
) -> Result<PreparedPackageRun> {
    use fsl_replay::spine::{
        read_clause_selection, ClauseRunSelection, ClauseSelectionPartsKind, SuppliedObservation,
        TraceInput,
    };

    if request.call.is_some() || request.tl_lowering.is_some() {
        return Err(invalid(
            "clause",
            "tl_lowering, call and clause are mutually exclusive",
        ));
    }
    if package.file.is_some() {
        return Err(RunCause::RequiresDirectory {
            member: "clause".to_owned(),
        });
    }
    let not_read = |member: &str| {
        invalid(
            member,
            "this clause run is not implemented over a selected checked package yet",
        )
    };
    let parts = clause
        .selection
        .into_parts()
        .map_err(|error| invalid(&error.member, error.reason))?;
    if !matches!(parts, ClauseSelectionPartsKind::Temporal { .. }) {
        return Err(not_read("clause.selection"));
    }
    let observation_limits =
        fsl_replay::spine::ObservationLimitsWire::resolve(clause.observation_limits.as_ref())
            .map_err(|error| invalid(&error.member, error.reason))?;
    // Each observation file is read once, as the source route reads it, and the
    // progress ledger file beside them.
    let mut files: Vec<&str> = Vec::new();
    for file in clause.observations.values() {
        if !files.contains(&file.as_str()) {
            files.push(file);
        }
    }
    let observed = files.len();
    if let Some(ledger) = clause.selection.ledger_path() {
        files.push(ledger);
    }
    let admitted = admit_files(directory, request, package, dependent_bytes, &files)?;
    let mut observations = std::collections::BTreeMap::new();
    for (key, file) in &clause.observations {
        let index = files
            .iter()
            .position(|candidate| *candidate == file.as_str())
            .filter(|index| *index < observed)
            .ok_or_else(|| invalid("clause.observations", "an observation names no file"))?;
        let bytes = admitted
            .observations
            .get(index)
            .ok_or_else(|| invalid("clause.observations", "an observation file was not read"))?;
        let mut observation = SuppliedObservation::from_document(key, bytes.clone())
            .map_err(|error| invalid(&error.member, error.reason))?;
        observation.path = Some(file.clone());
        observations.insert(key.clone(), observation);
    }
    let ledger = match clause.selection.ledger_path() {
        Some(_) => admitted
            .observations
            .get(observed)
            .map(|bytes| {
                fsl_replay::spine::read_ledger(bytes)
                    .map_err(|error| invalid(&error.member, error.reason))
            })
            .transpose()?,
        None => None,
    };
    let selection = read_clause_selection(parts, &observations, ledger)
        .map_err(|error| invalid(&error.member, error.reason))?;
    let (name, trace, over, ledger) = match &selection {
        ClauseRunSelection::SampledTemporal {
            name,
            trace,
            ledger,
        } => {
            let runs = match trace {
                TraceInput::Finite(positions) => !positions.is_empty(),
                TraceInput::Lasso { repeat, .. } => !repeat.is_empty(),
                TraceInput::Progressed { .. }
                | TraceInput::Timestamped { .. }
                | TraceInput::Model { .. } => false,
                TraceInput::Timed { .. } => true,
            };
            if !runs || ledger.is_some() {
                return Err(not_read("clause.selection.trace"));
            }
            (name, trace, None, None)
        }
        ClauseRunSelection::Temporal {
            name,
            trace,
            over,
            ledger,
        } => (name, trace, Some(over.clone()), ledger.clone()),
        ClauseRunSelection::Clause(_)
        | ClauseRunSelection::ClauseWithPath { .. }
        | ClauseRunSelection::Function { .. }
        | ClauseRunSelection::Frame { .. } => return Err(not_read("clause.selection")),
    };
    // FR-106: the snapshot documents the trace names, as `read_clause_run_request`
    // collects them.
    let mut snapshots = std::collections::BTreeMap::new();
    for observation in observations.into_values() {
        match observation.format.as_str() {
            "filament.state.snapshot/v1" => {
                snapshots.insert(observation.reference, observation.bytes);
            }
            "filament.state.invocation/v1" => {}
            _ => return Err(invalid("observation", "unsupported document format")),
        }
    }
    let work = ClauseWork {
        name: name.clone(),
        trace: trace.clone(),
        over,
        ledger,
        snapshots,
        observation_limits,
        selection: selection.clone(),
        accounting: super::call_accounting(&clause.accounting),
    };
    Ok(PreparedPackageRun {
        package: admitted.package,
        dependencies: admitted.dependencies,
        identities: admitted.identities,
        equality: admitted.equality,
        budget: admitted.budget,
        packages: admitted.packages,
        model_limits: admitted.model_limits,
        work: Work::Clause(Box::new(work)),
        selection: admitted.selection,
        models: admitted.models,
    })
}

/// Run the clause of a prepared `clause` request over the package's own
/// nodes and evaluator.
fn execute_clause(
    bare: &fsl_package::BarePackage<'_>,
    packages: &fsl_semantics::model::intake::PackageInput,
    model_limits: fsl_semantics::model::accounting::ModelNormalizationLimits,
    work: ClauseWork,
) -> Result<RunResult> {
    let not_read = |reason: String| invalid("clause.selection", reason);
    let model = if matches!(work.trace, fsl_replay::spine::TraceInput::Timed { .. }) {
        fsl_eval::simulation::model::PackageModelInput::of_timed_trace(bare, packages, model_limits)
    } else {
        fsl_eval::simulation::model::PackageModelInput::of(bare, packages, model_limits)
    }
    .map_err(|refusal| not_read(refusal.to_string()))?;
    let package = bare.package();
    let named = package
        .temporal_clause_names()
        .unwrap_or_default()
        .iter()
        .find(|row| *row.name == *work.name);
    let found = named
        .map(|row| {
            let key = fsl_package::checked_views::temporal_subject(package, &row.declaration)
                .ok()
                .and_then(|subject| subject.declaration_key())
                .ok_or_else(|| RunCause::TlFault("an admitted clause node has a key".to_owned()))?;
            let claim = fsl_foundation::source::provenance::OccurrenceKey::new(
                key.clone(),
                filament_exact::Origin::new(filament_exact::Role::new("claim"), row.occurrence),
            );
            let clause = model
                .temporal_run_clause(
                    &claim,
                    filament_semantic_value::checking::CheckingLimits::default().nodes(),
                )
                .map_err(|refusal| not_read(refusal.to_string()))?;
            if work.over.is_none()
                && !matches!(
                    clause.profile(),
                    fsl_semantics::check::TemporalProfile::InfiniteTrace
                        | fsl_semantics::check::TemporalProfile::Timed
                )
            {
                return Err(not_read(
                    "only an infinite-trace or timed clause runs over these record samples"
                        .to_owned(),
                ));
            }
            Ok((clause, key))
        })
        .transpose()?;
    let report = match &work.over {
        None => *fsl_replay::spine::run_package_sampled_authenticated(
            &fsl_replay::spine::PackageSampledRequest {
                evaluator: bare,
                name: &work.name,
                clause: found.as_ref().map(|(clause, key)| (clause, key)),
                types: model.input().types,
                trace: &work.trace,
                accounting: work.accounting,
            },
            bare.package(),
        ),
        Some(over) => {
            let input = model.input();
            fsl_replay::spine::run_package_snapshots(&fsl_replay::spine::PackageSnapshotRequest {
                package: bare,
                input: &input,
                name: &work.name,
                clause: found.as_ref().map(|(clause, key)| (clause, key)),
                packages,
                model_limits,
                snapshots: &work.snapshots,
                observation_limits: work.observation_limits,
                accounting: work.accounting,
                trace: &work.trace,
                over,
                ledger: work.ledger.as_ref(),
            })
            .map_err(|refusal| invalid(refusal.member, refusal.reason))?
        }
    };
    output::clause_report(report, &work.selection)
}

/// FR-1039: prepare a `tl_lowering` request over a selected package. Its
/// member decodes as the source route decodes it; a `cross_evaluate` builds
/// states over the package's model world, so it needs the handoff's rows
/// and a `{file}` selection refuses it before any file is opened.
fn prepare_lowering(
    directory: &Path,
    request: &wire::Request,
    package: &wire::RunPackageSelection,
    member: &serde_json::Value,
    dependent_bytes: usize,
) -> Result<PreparedPackageRun> {
    if request.call.is_some() || request.clause.is_some() {
        return Err(invalid(
            "tl_lowering",
            "tl_lowering, call and clause are mutually exclusive",
        ));
    }
    let lowering = super::tl::decode(member)?;
    if lowering.cross.is_some() && package.file.is_some() {
        return Err(RunCause::RequiresDirectory {
            member: "tl_lowering.cross_evaluate".to_owned(),
        });
    }
    let initial = match &lowering.cross {
        Some(super::tl::CrossEvaluate::Initial { initial, .. }) => Some(initial.as_str()),
        Some(super::tl::CrossEvaluate::Trace { .. }) | None => None,
    };
    let admitted = admit(directory, request, package, dependent_bytes, initial)?;
    Ok(PreparedPackageRun {
        package: admitted.package,
        dependencies: admitted.dependencies,
        identities: admitted.identities,
        equality: admitted.equality,
        budget: admitted.budget,
        packages: admitted.packages,
        model_limits: admitted.model_limits,
        work: Work::Lowering(lowering, admitted.snapshot),
        selection: admitted.selection,
        models: admitted.models,
    })
}

/// Package routes cannot restate choices already held by the package.
pub(super) fn validate_request(request: &wire::Request) -> Result<()> {
    // FR-767: a package carries its recorded law, so a request-selected law
    // beside it refuses first, naming `integer_division`.
    if request.integer_division.is_some() {
        return Err(invalid(
            "integer_division",
            "a package already records its integer-division law",
        ));
    }
    if request.program.is_some() {
        return Err(invalid(
            "program",
            "a program cannot be selected beside a package",
        ));
    }
    if !request.libraries.is_empty() {
        return Err(invalid(
            "libraries",
            "a package already holds its dependency selections",
        ));
    }
    // FR-640: a package was compiled under its clock selections.
    if !request.clocks.is_empty() {
        return Err(invalid("clocks", "a package already records its clocks"));
    }
    Ok(())
}

/// Admit the selected V2 package and the source documents for its selected
/// domain packages, preserving file/directory handoff behavior.
pub(super) fn admit(
    directory: &Path,
    request: &wire::Request,
    package: &wire::RunPackageSelection,
    dependent_bytes: usize,
    snapshot: Option<&str>,
) -> Result<AdmittedPackage> {
    let mut admitted = admit_files(
        directory,
        request,
        package,
        dependent_bytes,
        snapshot.as_slice(),
    )?;
    admitted.snapshot = admitted.observations.pop();
    Ok(admitted)
}

/// [`admit`] reading the observation files `files` beside the package, as the
/// source route reads the observations of a `clause` request: each file once,
/// counted against the same selected-file and byte ceilings.
pub(super) fn admit_files(
    directory: &Path,
    request: &wire::Request,
    package: &wire::RunPackageSelection,
    dependent_bytes: usize,
    files: &[&str],
) -> Result<AdmittedPackage> {
    // FR-1039: `equality` is a string admitted on the file form only, and
    // refuses before any selected file is opened.
    let equality_file = match &package.equality {
        None => None,
        Some(serde_json::Value::String(path)) => Some((path, beneath("package.equality", path)?)),
        Some(_) => return Err(invalid("package.equality", "must be a string")),
    };
    if equality_file.is_some() && package.directory.is_some() {
        return Err(invalid(
            "package.equality",
            "an equality file is selected beside a file, not beside a directory",
        ));
    }
    let (selection, handoff) = match (&package.file, &package.directory) {
        (Some(file), None) => (
            match &equality_file {
                Some((equality, _)) => serde_json::json!({ "file": file, "equality": equality }),
                None => serde_json::json!({ "file": file }),
            },
            Selected::File(beneath("package.file", file)?),
        ),
        (None, Some(handoff)) => (
            serde_json::json!({ "directory": handoff }),
            Selected::Directory(beneath("package.directory", handoff)?),
        ),
        _ => {
            return Err(invalid(
                "package",
                "expected exactly one of file and directory",
            ));
        }
    };
    let equality_file = equality_file.map(|(_, path)| path);
    if request
        .models
        .iter()
        .any(|model| !super::is_model_input_format(model))
    {
        return Err(RunCause::CompleteRunSelection(
            super::CompleteRunSelection::NativeModel,
        ));
    }
    let mut intake = super::intake_with_bytes(
        directory,
        FileCounts {
            programs: match handoff {
                Selected::File(_) => 1 + usize::from(equality_file.is_some()),
                Selected::Directory(_) => 2,
            },
            models: request.models.len(),
            observations: files.len(),
            extensions: 0,
        },
        dependent_bytes,
    )?;
    let package_path = match &handoff {
        Selected::File(file) => file.clone(),
        Selected::Directory(handoff) => format!("{handoff}/{PACKAGE_FILE}"),
    };
    if !directory.join(&package_path).is_file() {
        return Err(invalid(
            "package",
            format!("no checked package is at {package_path}"),
        ));
    }
    let bytes = match &handoff {
        Selected::Directory(handoff) => {
            super::handoff::read_file(&mut intake, handoff, PACKAGE_FILE, false)?
        }
        Selected::File(_) => intake.file(&package_path, intake.remaining)?,
    };
    require_contract_version(&bytes)?;
    let documents = request
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| intake.source(&model.source, &format!("models[{index}].source")))
        .collect::<Result<Vec<_>>>()?;
    let supplied = super::package_input_from_selected_models(&request.models, &documents).map_err(
        |cause| match cause {
            RunCause::ClauseRequest { member, reason } => invalid(&member, reason),
            cause => cause,
        },
    )?;
    let limits =
        fsl_package::V2ReadLimits::bounded(fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT);
    let (admitted, identities, equality, budget, dependencies) = match &handoff {
        Selected::Directory(handoff) => {
            let loaded = super::handoff::admit(&mut intake, handoff, bytes, &supplied, limits)?;
            (
                loaded.package,
                Some(loaded.identities),
                loaded.equality,
                loaded.budget,
                loaded.dependencies,
            )
        }
        Selected::File(_) => {
            let admitted = fsl_package::admit_checked_package(&bytes, &supplied, limits)
                .map_err(package_refusal)?;
            for (index, identity) in supplied.keys().enumerate() {
                if !admitted
                    .selected_model_packages()
                    .any(|recorded| recorded == identity)
                {
                    return Err(invalid(
                        &format!("/request/models/{index}"),
                        format!("the package records no model selection {identity}"),
                    ));
                }
            }
            let equality =
                read_equality_file(directory, &mut intake, equality_file.as_deref(), &admitted)?;
            (
                admitted,
                None,
                equality,
                limits.native_join_limits().budget(),
                Vec::new(),
            )
        }
    };
    let observations = files
        .iter()
        .map(|file| intake.file(file, intake.remaining))
        .collect::<Result<Vec<_>>>()?;
    Ok(AdmittedPackage {
        dependencies,
        package: admitted,
        identities,
        equality,
        budget,
        packages: supplied,
        model_limits: fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        snapshot: None,
        observations,
        selection,
        models: documents
            .iter()
            .map(|document| document.source().clone())
            .collect(),
    })
}

/// The form of a `package` selection.
enum Selected {
    File(String),
    Directory(String),
}

/// FR-1039 "A package that cannot run": a document whose `contract_version`
/// is not the v2 contract is refused before the strict reader runs.
fn require_contract_version(bytes: &[u8]) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct Contract {
        contract_version: Option<String>,
    }
    // A document that is not a JSON object is the strict reader's refusal.
    let Ok(Contract {
        contract_version: Some(version),
    }) = serde_json::from_slice::<Contract>(bytes)
    else {
        return Ok(());
    };
    if version == CONTRACT_VERSION {
        return Ok(());
    }
    Err(RunCause::AnalyzePackage {
        code: fsl_foundation::Code::InvalidPackage,
        cause: Some("unknown-contract-version".to_owned()),
        pointer: Some("/request/package/contract_version".to_owned()),
        message: format!(
            "invalid_package/unknown-contract-version at /request/package/contract_version: \
             {version} is not {CONTRACT_VERSION}"
        ),
    })
}

/// FR-1039: the equality rows of a `{file, equality}` selection, read through
/// `ReadEqualityRows::read` against the admitted package as a handoff's
/// `equality.json` is. A `{file}` selection with no `equality` over a package
/// that holds an equality application refuses `invalid_package/missing-file`,
/// and one over a package with none carries no rows.
fn read_equality_file(
    directory: &Path,
    intake: &mut super::Intake<'_>,
    equality: Option<&str>,
    package: &filament_contract_model::CheckedPackageV2,
) -> Result<Option<fsl_package::ReadEqualityRows>> {
    let Some(path) = equality else {
        if package
            .checked_term_data()
            .is_some_and(|data| !data.equality_decisions.is_empty())
        {
            return Err(RunCause::AnalyzePackage {
                code: fsl_foundation::Code::InvalidPackage,
                cause: Some("missing-file".to_owned()),
                pointer: Some("/request/package/equality".to_owned()),
                message: "invalid_package/missing-file at /request/package/equality: the package \
                          holds an equality application and the selection names no equality file"
                    .to_owned(),
            });
        }
        return Ok(None);
    };
    if !directory.join(path).is_file() {
        return Err(invalid(
            "package.equality",
            format!("no equality file is at {path}"),
        ));
    }
    let bytes = intake
        .file(path, intake.remaining)
        .map_err(|cause| match cause {
            limit @ RunCause::Limit(_) => limit,
            _ => invalid(
                "package.equality",
                format!("the equality file at {path} cannot be read"),
            ),
        })?;
    fsl_package::ReadEqualityRows::read(&bytes, package)
        .map(Some)
        .map_err(|refusal| {
            let variant = match &refusal {
                fsl_package::EqualityFileRefusal::Malformed { .. } => "Malformed",
                fsl_package::EqualityFileRefusal::Format(_) => "Format",
                fsl_package::EqualityFileRefusal::Unjoined { .. } => "Unjoined",
            };
            RunCause::AnalyzePackage {
                code: fsl_foundation::Code::InvalidPackage,
                cause: Some("refused-file".to_owned()),
                pointer: Some("/request/package/equality".to_owned()),
                message: format!(
                    "invalid_package/refused-file at /request/package/equality: {path}: \
                     {variant}: {refusal}"
                ),
            }
        })
}

/// The handoff directory's identity rows, and its equality rows when the
/// producer wrote them (FR-850 "Admission", FR-531).
fn read_witness(
    directory: &Path,
    intake: &mut super::Intake<'_>,
    handoff: &str,
    package: &filament_contract_model::CheckedPackageV2,
    budget: &mut filament_contract_model::NativeJoinBudget,
) -> Result<(
    Option<fsl_package::ReadIdentityRows>,
    Option<fsl_package::ReadEqualityRows>,
)> {
    let witness = |file: &str, detail: String| RunCause::AnalyzePackage {
        code: fsl_foundation::Code::InvalidPackage,
        cause: Some("native-witness".to_owned()),
        pointer: Some("/request/package/directory".to_owned()),
        message: format!("invalid_package/native-witness: {file}: {detail}"),
    };
    let identities_path = format!("{handoff}/{IDENTITIES_FILE}");
    if !directory.join(&identities_path).is_file() {
        return Err(invalid(
            "package",
            format!("the handoff directory {handoff} holds no {IDENTITIES_FILE}"),
        ));
    }
    let identity_bytes = intake.file(&identities_path, intake.remaining)?;
    let identities = fsl_package::ReadIdentityRows::read(&identity_bytes, package, budget)
        .map_err(|refusal| witness(IDENTITIES_FILE, refusal.to_string()))?;
    let equality_path = format!("{handoff}/{EQUALITY_FILE}");
    let equality = if directory.join(&equality_path).is_file() {
        let equality_bytes = intake.file(&equality_path, intake.remaining)?;
        Some(
            fsl_package::ReadEqualityRows::read(&equality_bytes, package)
                .map_err(|refusal| witness(EQUALITY_FILE, refusal.to_string()))?,
        )
    } else {
        None
    };
    Ok((Some(identities), equality))
}

/// Run a prepared package call (FR-945): build the bare package over the
/// admitted bytes and its witness rows, call the function and render the
/// result as the source route renders it.
pub(super) fn execute(prepared: PreparedPackageRun) -> Result<RunResult> {
    let PreparedPackageRun {
        package,
        identities,
        equality,
        mut budget,
        dependencies,
        packages,
        model_limits,
        work,
        selection,
        models,
    } = prepared;
    // A `{file}` selection carries no handoff rows: its call reads only the
    // declarations the package's own nodes give (FR-1039 "a `call` runs from
    // either form").
    let identities = identities.unwrap_or_else(|| {
        fsl_package::ReadIdentityRows::of_rows(
            &package,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    });
    let bare = fsl_package::BarePackage::new(&package)
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: equality.as_ref(),
            },
            &mut budget,
        )
        .map_err(|failure| {
            let setting = failure
                .setting()
                .map_or_else(|| "i2.work_units".to_owned(), |setting| setting.to_string());
            RunCause::AnalyzePackage {
                code: fsl_foundation::Code::StageLimitExceeded,
                cause: None,
                pointer: None,
                message: format!("stage_limit_exceeded: {setting}: {failure:?}"),
            }
        })?;
    let call = match work {
        Work::Call(call) => *call,
        Work::Clause(work) => {
            let mut result = execute_clause(&bare, &packages, model_limits, *work)?;
            result.sources = crate::Sources::from_sources(models);
            return Ok(result);
        }
        Work::Lowering(lowering, snapshot) => {
            let lowered = super::tl::lower_over_package(
                &bare,
                &lowering,
                &super::tl::PackageRun {
                    packages: &packages,
                    model_limits,
                    snapshot: snapshot.as_deref(),
                },
            )?;
            let mut result = match lowered {
                super::tl::PackageLowered::Record(record) => RunResult {
                    exit_code: record.exit_code,
                    value: super::NativeResult::lowering_document(super::tl::document(
                        record.record,
                    )),
                    outcome: None,
                    requirements: None,
                    sources: crate::Sources::default(),
                },
                // The source route's clause report for the same stop.
                super::tl::PackageLowered::Reported(reported) => {
                    output::clause_report(*reported.report, &reported.selection)?
                }
            };
            result.sources = crate::Sources::from_sources(models);
            return Ok(result);
        }
    };
    let objects = if let Some(snapshot) = &call.snapshot {
        let provision = std::collections::BTreeMap::from([(
            snapshot.observation.reference.clone(),
            snapshot.observation.bytes.clone(),
        )]);
        bare.admit_call_snapshot(
            &packages,
            model_limits,
            &provision,
            &snapshot.observation.reference,
            snapshot.limits,
        )
        .map(|(objects, _)| objects)
        .map_err(|failure| {
            use fsl_semantics::model::observation::AdmissionFailure;
            match failure {
                AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record) => {
                    RunCause::SpineRun(Box::new(fsl_replay::spine::RunRefusal::ObjectEnvironment(
                        record,
                    )))
                }
                AdmissionFailure::Fault(fault) => {
                    RunCause::SpineRun(Box::new(fsl_replay::spine::RunRefusal::Fault(fault)))
                }
            }
        })?
    } else {
        fsl_semantics::model::object_environment::ObjectEnvironment::default()
    };
    let outcome =
        fsl_replay::spine::run_package(&bare, &call, &objects, &filament_exact::Cancel::new())
            .map_err(|refusal| RunCause::SpineRun(Box::new(*refusal)))?;
    let mut result = output::spine_run_result_for_package(&selection, &call.function, outcome)?;
    result.sources = crate::Sources::from_sources(models);
    Ok(result)
}
