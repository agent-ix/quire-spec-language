// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-620: the `native-analyze/1` request reader and the analyze run over
//! it. The reader applies FR-026's envelope and intake rules, compiles the
//! program as FR-027 compiles it, provisions the supplied documents under
//! their own identities, and hands the items to `fsl-analyze`'s resolver
//! and document writer. The caller negotiates (FR-281), so the run takes the
//! caller's [`Negotiate`].

use std::path::Path;

use filament_exact::{Cancel, CancelCause};
use fsl_analyze::analyze::document::{
    resolve_items, settle_package_run, settle_run, AnalyzeDocumentError, AnalyzeRun, ItemSelection,
    Negotiate, PackageRun, PackageRunError, SettledRun, SubjectSelection,
};
use fsl_analyze::analyze::{AnalyzeLimits, DeadlineSupply, Engine, Engines, SourceProvision};
use fsl_analyze::provider::ProviderOptions;
use fsl_eval::simulation::protocol::{MemorySelection, MemorySelectionRefusal, ResolvedMemory};
use fsl_semantics::model::observation::DocumentProvision;

use super::settle::Settle;
use super::{
    check_sources, intake_with_bytes, request, request_integer_division, require_native_header,
    wire, FileCounts, FormalSource, NativeResult, Result, RunCause, RunError, RunLimits, RunResult,
};

fn invalid(member: &str, reason: impl Into<String>) -> RunCause {
    RunCause::AnalyzeRequest {
        member: member.to_owned(),
        reason: reason.into(),
    }
}

fn validate_activated_population(
    position: usize,
    population: &serde_json::Value,
) -> std::result::Result<(), (String, String)> {
    let base = format!("/items/{position}/population");
    let invalid_at = |pointer: String, reason: String| Err((pointer, reason));
    let Some(groups) = population.as_array() else {
        return invalid_at(base, "population must be an array".to_owned());
    };
    for (group_index, group) in groups.iter().enumerate() {
        let group_pointer = format!("{base}/{group_index}");
        let Some(group) = group.as_object() else {
            return invalid_at(
                group_pointer,
                "population entry must be an object".to_owned(),
            );
        };
        for key in group.keys() {
            if !matches!(key.as_str(), "role" | "instance" | "members") {
                return invalid_at(
                    format!("{group_pointer}/{}", pointer_token(key)),
                    format!("unknown population entry member `{key}`"),
                );
            }
        }
        let role_pointer = format!("{group_pointer}/role");
        if group
            .get("role")
            .and_then(serde_json::Value::as_str)
            .is_none()
        {
            return invalid_at(role_pointer, "role must be a string".to_owned());
        }
        if let Some(instance) = group.get("instance") {
            if !instance.is_null() && !instance.is_string() {
                return invalid_at(
                    format!("{group_pointer}/instance"),
                    "instance must be a string or null".to_owned(),
                );
            }
        }
        let members_pointer = format!("{group_pointer}/members");
        let Some(members) = group.get("members").and_then(serde_json::Value::as_array) else {
            return invalid_at(members_pointer, "members must be an array".to_owned());
        };
        for (member_index, member) in members.iter().enumerate() {
            let member_pointer = format!("{members_pointer}/{member_index}");
            let Some(member) = member.as_object() else {
                return invalid_at(
                    member_pointer,
                    "population member must be an object".to_owned(),
                );
            };
            for key in member.keys() {
                if !matches!(key.as_str(), "authority" | "identity") {
                    return invalid_at(
                        format!("{member_pointer}/{}", pointer_token(key)),
                        format!("unknown population member field `{key}`"),
                    );
                }
            }
            for field in ["authority", "identity"] {
                let pointer = format!("{member_pointer}/{field}");
                let Some(value) = member.get(field).and_then(serde_json::Value::as_str) else {
                    return invalid_at(pointer, format!("{field} must be a string"));
                };
                if value.is_empty() {
                    return invalid_at(pointer, format!("{field} must be nonempty"));
                }
            }
        }
    }
    Ok(())
}

/// Whether `role` is one of FSpec FR-331's five `premises` roles a
/// composition item names.
fn is_premise_role(role: fsl_replay::planner::SlotRole) -> bool {
    use fsl_replay::planner::SlotRole;
    matches!(
        role,
        SlotRole::ComponentGuarantee
            | SlotRole::Rely
            | SlotRole::LivenessDependency
            | SlotRole::Fairness
            | SlotRole::Environment
    )
}

/// FR-1044: a composition item's `premises` is an array of closed
/// `{role, key}` objects whose `role` is one of FSpec FR-331's five, each
/// `key` an object. Whether the list is the derived one is decided against
/// the checked declaration, per item.
fn validate_composition_premises(
    position: usize,
    premises: &serde_json::Value,
) -> std::result::Result<(), (String, String)> {
    let base = format!("/items/{position}/premises");
    let Some(entries) = premises.as_array() else {
        return Err((base, "premises must be an array".to_owned()));
    };
    for (entry_index, entry) in entries.iter().enumerate() {
        let pointer = format!("{base}/{entry_index}");
        let Some(entry) = entry.as_object() else {
            return Err((pointer, "a premise must be an object".to_owned()));
        };
        if let Some(member) = entry
            .keys()
            .find(|member| !matches!(member.as_str(), "role" | "key"))
        {
            return Err((
                format!("{pointer}/{}", pointer_token(member)),
                format!("unknown premise member `{member}`"),
            ));
        }
        match entry.get("role").and_then(serde_json::Value::as_str) {
            Some(role)
                if fsl_replay::planner::SlotRole::parse(role).is_some_and(is_premise_role) => {}
            Some(role) => {
                return Err((
                    format!("{pointer}/role"),
                    format!("`{role}` is not a premise role"),
                ));
            }
            None => {
                return Err((
                    format!("{pointer}/role"),
                    "role must be a string".to_owned(),
                ))
            }
        }
        if !entry.get("key").is_some_and(serde_json::Value::is_object) {
            return Err((format!("{pointer}/key"), "key must be an object".to_owned()));
        }
    }
    Ok(())
}

fn pointer_token(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

/// A read `native-analyze/1` request: its compiled-from sources, supplied
/// documents, subject, items and limits, with no compiler, evaluator or
/// engine call made.
pub struct PreparedAnalyze {
    input: Box<AnalyzeInput>,
}

/// The owned inputs of one analyze run (FR-620 "Library surface").
pub struct AnalyzeInput {
    /// The program source.
    pub source: FormalSource,
    /// The caller's domain-package selections, compile request members and
    /// libraries, compiled as FR-027 compiles them.
    compile: wire::CompileRequest,
    /// The supplied documents, each under its own identity.
    pub documents: DocumentProvision,
    /// The request's default model subject.
    pub subject: SubjectSelection,
    /// The request's `memory` member (FSL FR-1033, FR-219), also carried by
    /// [`Self::subject`].
    pub memory: Vec<fsl_eval::simulation::protocol::MemorySelection>,
    /// The caller's items; `None` for FR-620's default items.
    pub items: Option<Vec<ItemSelection>>,
    /// The limits every item of the run shares.
    pub limits: AnalyzeLimits,
    /// The run's deadline in milliseconds.
    pub deadline_ms: Option<u64>,
    /// The optional request-supplied integer-division law.
    integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
}

impl PreparedAnalyze {
    /// Transfer the read inputs without running a stage.
    pub fn into_input(self) -> AnalyzeInput {
        *self.input
    }
}

/// Read a `native-analyze/1` request and every file it selects (FR-620,
/// FR-026), performing no compiler, evaluator, engine or provider call.
///
/// # Errors
///
/// `invalid-request` for a malformed, wrong-format or unknown-member
/// request; FR-026's resource causes for a byte or file limit; the source
/// refusal of an unreadable program.
pub fn prepare_analyze(
    path: &Path,
    limits: RunLimits,
) -> std::result::Result<PreparedAnalyze, Box<RunError>> {
    super::with_request_limits(path, limits.request_bytes, |directory, bytes| {
        prepare_bytes(directory, bytes, limits)
    })
}

fn prepare_bytes(directory: &Path, bytes: &[u8], limits: RunLimits) -> Result<PreparedAnalyze> {
    match read(directory, bytes, limits)? {
        Readout::Source(input, _) => Ok(PreparedAnalyze {
            input: Box::new(input),
        }),
        // A prepared run holds the source it compiles; a package request
        // has none to prepare (FR-850).
        Readout::Package(_) => Err(RunCause::AnalyzeNotYetImplemented {
            member: "package",
            detail: ": a prepared run holds a program source".to_owned(),
            locus: None,
        }),
    }
}

/// What [`read`] read: a `program` request's inputs with the intake that
/// still charges compilation, or a `package` request's admitted package.
enum Readout<'d> {
    Source(AnalyzeInput, super::Intake<'d>),
    Package(PackageSelection),
}

/// A `package` request (FR-850): the package the strict reader admitted
/// from the request's bytes, with the request's other members.
struct PackageSelection {
    admitted: AdmittedHandoff,
    members: Box<RunMembers>,
    memory: Vec<fsl_eval::simulation::protocol::MemorySelection>,
    items: Option<Vec<ItemSelection>>,
    deadline_ms: Option<u64>,
}

/// FR-850, FR-940: analyze a `package` request over the `BarePackage` of
/// its admitted package, read through `AnalysisView`, never compiling
/// source, with the caller's `negotiate` under `cancel` and its deadline.
/// Each item reads the admitted package form its engine family supports.
fn run_package(
    selection: PackageSelection,
    negotiate: &dyn Negotiate,
    providers: Vec<Engine>,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> Result<RunResult> {
    let engines = engines_with(providers, options)?;
    let PackageSelection {
        admitted,
        members,
        memory,
        items,
        deadline_ms,
    } = selection;
    let AdmittedHandoff {
        package: admitted,
        identities,
        equality,
        mut budget,
        supplied,
        package_bytes,
        identity_bytes,
        equality_bytes,
    } = admitted;
    let package = fsl_package::BarePackage::new(&admitted)
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: equality.as_ref(),
            },
            &mut budget,
        )
        .map_err(|failure| {
            // FR-531 "Denial is its own typed refusal": a denied identity
            // join names the `i2.*` setting, never a witness defect.
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
    let RunMembers {
        documents,
        mut subject,
        limits,
    } = *members;
    check_memory_using(&memory, |protocol, entries| {
        use fsl_package::AnalysisView;
        package
            .protocol_execution_named(protocol)
            .ok()
            .flatten()
            .map(|form| ResolvedMemory::resolve_package(&form, entries))
    })?;
    subject.memory = memory;
    let binding = subject.binding(&supplied, &documents);
    let run = PackageRun {
        package: &package,
        package_bytes: &package_bytes,
        supply: fsl_replay::HandoffSupply {
            identities: &identity_bytes,
            equality: equality_bytes.as_deref(),
        },
        subject,
        binding,
        items: items.as_deref(),
        engines: &engines,
        limits,
    };
    let settled = with_deadline(cancel, deadline_ms, || {
        settle_package_run(run, negotiate, cancel)
    })
    .map_err(|failure| match failure {
        PackageRunError::Pending(pending) => RunCause::AnalyzeNotYetImplemented {
            member: "package",
            detail: pending.detail(),
            locus: pending.locus(),
        },
        PackageRunError::Document(AnalyzeDocumentError::Refused(refusal)) => {
            RunCause::AnalyzeRefused(Box::new(refusal))
        }
        PackageRunError::Document(AnalyzeDocumentError::Fault) => RunCause::AnalyzeFault,
    })?;
    Ok(RunResult {
        exit_code: settled.exit_code,
        value: NativeResult::analyze_document(settled.document),
        outcome: None,
        requirements: None,
        sources: crate::Sources::default(),
    })
}

/// The request's inputs and the intake that read them, which still charges
/// the libraries and domain packages compilation reads.
fn read<'d>(directory: &'d Path, bytes: &[u8], limits: RunLimits) -> Result<Readout<'d>> {
    let dependent_bytes = limits.dependent_bytes;
    let replay = fsl_replay::ReplayLimits::default().with_input_bytes(limits.replay_input_bytes);
    let mut request: wire::AnalyzeRequest = request(bytes).map_err(|cause| match cause {
        RunCause::Json(error) => invalid("request", error.to_string()),
        cause => cause,
    })?;
    validate_items(request.items.as_deref())?;
    if let Some(package) = &request.package {
        let (admitted, mut intake) = admit_package(directory, dependent_bytes, &request, package)?;
        let memory = read_memory(&request.memory)?;
        let members = Box::new(read_run_members(&request, &mut intake, replay)?);
        return Ok(Readout::Package(PackageSelection {
            admitted,
            members,
            memory,
            items: request.items,
            deadline_ms: request.deadline_ms,
        }));
    }
    let integer_division = request_integer_division(request.integer_division.take())?;
    let Some(program) = &request.program else {
        return Err(invalid("program", "a program or package is required"));
    };
    let memory = read_memory(&request.memory)?;
    let mut intake = intake_with_bytes(
        directory,
        FileCounts {
            programs: request.libraries.len().saturating_add(1),
            models: request.models.len(),
            observations: request.documents.len(),
            extensions: 0,
        },
        dependent_bytes,
    )?;
    let source = intake.source(&program.source, "program.source")?;
    require_native_header(source.source())?;
    let RunMembers {
        documents,
        mut subject,
        limits,
    } = read_run_members(&request, &mut intake, replay)?;
    subject.memory.clone_from(&memory);
    let compile = wire::CompileRequest {
        models: Some(request.models),
        program: wire::CompileProgram {
            sources: Vec::new(),
            extraction: false,
        },
        refinement_internals: request.refinement_internals,
        libraries: Some(request.libraries),
        extensions: Vec::new(),
        clocks: (!request.clocks.is_empty()).then_some(request.clocks),
        output: None,
        handoff: None,
        integer_division: None,
        check_mode: filament_semantic_value::checking::CheckMode::Linked,
    };
    Ok(Readout::Source(
        AnalyzeInput {
            source,
            compile,
            documents,
            subject,
            memory,
            items: request.items,
            limits,
            deadline_ms: request.deadline_ms,
            integer_division,
        },
        intake,
    ))
}

/// Read the `native-analyze/1` request at `path`, compile its program,
/// resolve its items, have `negotiate` settle them, run them under `cancel`
/// and its deadline, and return FR-621's document with its FR-285 exit code.
///
/// # Errors
///
/// A whole-request refusal (FR-624): the request stage's, a compile stage's,
/// or the subject binding's and source provision's; the caller writes its
/// `analyze` refusal document.
pub fn analyze(
    path: &Path,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    analyze_with_limits(path, RunLimits::default(), negotiate, cancel)
}

/// [`analyze`] under the caller's request and dependent-file byte ceilings.
///
/// # Errors
///
/// As [`analyze`].
pub fn analyze_with_limits(
    path: &Path,
    limits: RunLimits,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    analyze_with_providers_and_limits(
        path,
        limits,
        negotiate,
        Vec::new(),
        &ProviderOptions::default(),
        cancel,
    )
}

/// [`analyze`] with the caller's started process providers (FR-635 "One rule
/// for every provider-routing entry"): each an [`Engine::external`] engine
/// over its `Process` descriptor and the handle its items are sent over.
/// They register beside `Engines::fsl()` before any item is negotiated, so
/// an item reaches one only by negotiation. An empty `providers` is
/// [`analyze`]. `options` carries the run's FSpec FR-462 limits, which every
/// provider request carries (FR-964).
///
/// # Errors
///
/// As [`analyze`]; and `invalid-request` naming `providers` when a provider
/// repeats a registered `BackendId`, before any item runs.
pub fn analyze_with_providers(
    path: &Path,
    negotiate: &dyn Negotiate,
    providers: Vec<Engine>,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    analyze_with_providers_and_limits(
        path,
        RunLimits::default(),
        negotiate,
        providers,
        options,
        cancel,
    )
}

/// [`analyze_with_providers`] under the caller's request and dependent-file
/// byte ceilings.
///
/// # Errors
///
/// As [`analyze_with_providers`].
pub fn analyze_with_providers_and_limits(
    path: &Path,
    limits: RunLimits,
    negotiate: &dyn Negotiate,
    providers: Vec<Engine>,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    super::with_request_limits(path, limits.request_bytes, |directory, bytes| {
        let (input, mut intake) = match read(directory, bytes, limits)? {
            Readout::Source(input, intake) => (input, intake),
            Readout::Package(selection) => {
                return run_package(selection, negotiate, providers, options, cancel);
            }
        };
        let mut result = run_input(&mut intake, input, negotiate, providers, options, cancel)?;
        result.sources = crate::Sources::from_sources(intake.sources.iter().cloned());
        Ok(result)
    })
}

/// FSL's engines with the caller's process providers registered beside them,
/// under the run's provider `options`.
pub(super) fn engines_with(providers: Vec<Engine>, options: &ProviderOptions) -> Result<Engines> {
    let mut engines = Engines::fsl()
        .map_err(|_| RunCause::AnalyzeFault)?
        .under(*options);
    for provider in providers {
        engines.with(provider).map_err(|duplicate| {
            invalid(
                "providers",
                format!(
                    "a configured provider repeats the registered backend {}",
                    duplicate.backend.as_str()
                ),
            )
        })?;
    }
    Ok(engines)
}

fn run_input(
    intake: &mut super::Intake<'_>,
    input: AnalyzeInput,
    negotiate: &dyn Negotiate,
    providers: Vec<Engine>,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> Result<RunResult> {
    // The registry is fixed before the program is read, so a provider that
    // cannot register refuses the request before any stage runs.
    let engines = engines_with(providers, options)?;
    let AnalyzeInput {
        source,
        compile,
        documents,
        subject,
        memory,
        items,
        limits,
        deadline_ms,
        integer_division,
    } = input;
    let routed = RoutedItems {
        documents,
        subject,
        memory,
        items,
        limits,
        deadline_ms,
    };
    let sources = std::slice::from_ref(&source);
    check_sources(
        &compile,
        intake,
        sources,
        false,
        fsl_replay::spine::SpineLimits::default(),
        integer_division,
        None,
        |_intake, checked, _, _, packages, libraries, _, _, _, _| {
            let units = std::iter::once(&source).chain(libraries);
            let settled = settle_items(
                checked.package(),
                &packages,
                units,
                &routed,
                &Settle {
                    negotiate,
                    engines: &engines,
                    cancel,
                },
            )?;
            Ok(RunResult {
                exit_code: settled.exit_code,
                value: NativeResult::analyze_document(settled.document),
                outcome: None,
                requirements: None,
                sources: crate::Sources::default(),
            })
        },
    )
}

/// The FR-620 items of one request and what they read besides the compiled
/// program: an analyze request's, or a monitor request's provider-routed
/// `items` (FR-644).
pub(super) struct RoutedItems {
    /// The supplied documents, each under its own identity.
    pub documents: DocumentProvision,
    /// The request's default model subject.
    pub subject: SubjectSelection,
    /// The request's `memory` member (FSL FR-1033, FR-219).
    pub memory: Vec<fsl_eval::simulation::protocol::MemorySelection>,
    /// The caller's items; `None` for FR-620's default items.
    pub items: Option<Vec<ItemSelection>>,
    /// The limits every item of the run shares.
    pub limits: AnalyzeLimits,
    /// The run's deadline in milliseconds.
    pub deadline_ms: Option<u64>,
}

/// The FR-620 settlement of `routed` over the compiled `package`: each item
/// resolved against `settle`'s registry, negotiated by its negotiator, the
/// in-process items run on the linked engines and the provider items sent
/// through one FR-950 request per backend (FR-635), all under its cancel
/// handle and the run's deadline. `units` are the request's units in
/// request order, the program then each library as listed.
///
/// # Errors
///
/// A whole-request refusal (FR-624) of the subject binding or source
/// provision, or an internal fault.
pub(super) fn settle_items<'u>(
    package: &fsl_package::CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
    units: impl IntoIterator<Item = &'u FormalSource>,
    routed: &RoutedItems,
    settle: &Settle<'_>,
) -> Result<SettledRun> {
    let mut provision = SourceProvision::default();
    let mut labels = Vec::new();
    for read in units {
        labels.push(read.source().reference().clone());
        provision.insert(
            read.source().reference(),
            read.source().text().as_bytes().to_vec(),
        );
    }
    let mut registry = fsl_route::Registry::new();
    for descriptor in settle.engines.registrations() {
        registry
            .register(descriptor.clone())
            .map_err(|_| RunCause::AnalyzeFault)?;
    }
    check_memory(package, &routed.memory)?;
    let binding = routed.subject.binding(packages, &routed.documents);
    let resolved = resolve_items(
        package,
        &routed.subject,
        routed.items.as_deref(),
        &registry,
        &labels,
        &binding,
        &routed.limits,
    );
    let run = AnalyzeRun {
        package,
        binding,
        provision,
        items: resolved,
        engines: settle.engines,
        limits: routed.limits,
    };
    with_deadline(settle.cancel, routed.deadline_ms, || {
        settle_run(run, settle.negotiate, settle.cancel)
    })
    .map_err(|error| match error {
        AnalyzeDocumentError::Refused(refusal) => RunCause::AnalyzeRefused(Box::new(refusal)),
        AnalyzeDocumentError::Fault => RunCause::AnalyzeFault,
    })
}

/// Run `work` with `cancel` cancelled for `deadline` once `deadline_ms`
/// elapses (FR-287): a deadline of 0 has passed before any item runs.
fn with_deadline<T>(cancel: &Cancel, deadline_ms: Option<u64>, work: impl FnOnce() -> T) -> T {
    match deadline_ms {
        None => work(),
        Some(0) => {
            cancel.cancel(CancelCause::Deadline);
            work()
        }
        Some(milliseconds) => std::thread::scope(|scope| {
            let (done, finished) = std::sync::mpsc::channel::<()>();
            let timer = cancel.clone();
            scope.spawn(move || {
                let wait = std::time::Duration::from_millis(milliseconds);
                if let Err(std::sync::mpsc::RecvTimeoutError::Timeout) = finished.recv_timeout(wait)
                {
                    timer.cancel(CancelCause::Deadline);
                }
            });
            let value = work();
            // The timer thread ends on this message or on its timeout; a
            // closed channel also ends it.
            let _ = done.send(());
            value
        }),
    }
}

/// FR-850 "Admission": a request that selects `package` is read by the strict
/// checked-package reader against the supplied domain packages, and refuses
/// before any item resolves when a compile member sits beside it, the reader
/// refuses the bytes, a recorded domain package is not supplied, or a
/// supplied one is not recorded.
fn admit_package<'d>(
    directory: &'d Path,
    dependent_bytes: usize,
    request: &wire::AnalyzeRequest,
    package: &wire::HandoffSelection,
) -> Result<(AdmittedHandoff, super::Intake<'d>)> {
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
    if request.refinement_internals.is_some() {
        return Err(invalid(
            "refinement_internals",
            "a package is already compiled",
        ));
    }
    if request.integer_division.is_some() {
        return Err(invalid("integer_division", "a package is already compiled"));
    }
    let mut intake = intake_with_bytes(
        directory,
        FileCounts {
            // The handoff's package, identity and equality files.
            programs: 3,
            models: request.models.len(),
            observations: request.documents.len(),
            extensions: 0,
        },
        dependent_bytes,
    )?;
    let handoff = handoff_directory(&package.directory)?;
    // FR-850 "Admission": an absent handoff directory or package file is a
    // package refusal, not an I/O failure.
    if !directory.join(&handoff).is_dir() {
        return Err(witness_refusal(
            &handoff,
            "missing-file",
            "the request names no handoff directory there",
        ));
    }
    if !directory.join(&handoff).join(PACKAGE_FILE).is_file() {
        return Err(witness_refusal(
            PACKAGE_FILE,
            "missing-file",
            "the handoff directory holds no checked package",
        ));
    }
    let bytes = intake.file(&format!("{handoff}/{PACKAGE_FILE}"), intake.remaining)?;
    if request
        .models
        .iter()
        .any(|model| !super::is_model_input_format(model))
    {
        return Err(RunCause::CompleteSelection(
            super::CompleteSelection::NativeModels,
        ));
    }
    let documents = request
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| intake.source(&model.source, &format!("models[{index}].source")))
        .collect::<Result<Vec<_>>>()?;
    let supplied = super::package_input_from_selected_models(&request.models, &documents).map_err(
        |cause| match cause {
            RunCause::ClauseRequest { member, reason } if member == "models" => {
                RunCause::AnalyzeRequest { member, reason }
            }
            cause => cause,
        },
    )?;
    let limits =
        fsl_package::V2ReadLimits::bounded(fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT);
    let admitted =
        fsl_package::admit_checked_package(&bytes, &supplied, limits).map_err(package_refusal)?;
    for identity in supplied.keys() {
        if !admitted
            .selected_model_packages()
            .any(|recorded| recorded == identity)
        {
            return Err(conflicting_model(identity));
        }
    }
    // FR-850 "Admission": the handoff's identity rows are required, its
    // equality rows read when the producer wrote them (FR-531).
    if !directory.join(&handoff).join(IDENTITIES_FILE).is_file() {
        return Err(witness_refusal(
            IDENTITIES_FILE,
            "missing-file",
            "the handoff directory holds no identity file",
        ));
    }
    let identity_bytes = intake.file(&format!("{handoff}/{IDENTITIES_FILE}"), intake.remaining)?;
    // One native budget for the pass: the identity read, then the supply's
    // join (FR-531 "One join, then borrowing").
    let mut budget = limits.native_join_limits().budget();
    let identities = fsl_package::ReadIdentityRows::read(&identity_bytes, &admitted, &mut budget)
        .map_err(|refusal| {
        let cause = match &refusal {
            fsl_package::IdentityFileRefusal::Malformed { .. } => "malformed-file",
            fsl_package::IdentityFileRefusal::Format(_) => "unknown-format",
            fsl_package::IdentityFileRefusal::Unjoined { .. } => "unjoined-row",
            fsl_package::IdentityFileRefusal::Native(_)
            | fsl_package::IdentityFileRefusal::Rows(_)
            | fsl_package::IdentityFileRefusal::SourceObject(_)
            | fsl_package::IdentityFileRefusal::SourceOperation(_) => "native-witness",
        };
        witness_refusal(IDENTITIES_FILE, cause, &refusal.to_string())
    })?;
    let (equality, equality_bytes) = if directory.join(&handoff).join(EQUALITY_FILE).is_file() {
        let equality_bytes =
            intake.file(&format!("{handoff}/{EQUALITY_FILE}"), intake.remaining)?;
        let rows =
            fsl_package::ReadEqualityRows::read(&equality_bytes, &admitted).map_err(|refusal| {
                witness_refusal(EQUALITY_FILE, "refused-file", &refusal.to_string())
            })?;
        (Some(rows), Some(equality_bytes))
    } else if admitted
        .checked_term_data()
        .is_some_and(|data| !data.equality_decisions.is_empty())
    {
        // FR-850 "Admission": a package holding equality applications is
        // read with the producer's equality rows, so their file is required.
        return Err(witness_refusal(
            EQUALITY_FILE,
            "missing-file",
            "the handoff directory holds no equality file for the package's equality applications",
        ));
    } else {
        (None, None)
    };
    Ok((
        AdmittedHandoff {
            package: admitted,
            identities,
            equality,
            budget,
            supplied,
            package_bytes: bytes,
            identity_bytes,
            equality_bytes,
        },
        intake,
    ))
}

/// The FR-517 files a `package` selection reads from its handoff directory.
const PACKAGE_FILE: &str = "checked-package.json";
const IDENTITIES_FILE: &str = "identities.json";
const EQUALITY_FILE: &str = "equality.json";

/// The admitted handoff of a `package` request: the package the strict
/// reader admitted, its identity rows and its equality rows when present.
struct AdmittedHandoff {
    package: filament_contract_model::CheckedPackageV2,
    identities: fsl_package::ReadIdentityRows,
    equality: Option<fsl_package::ReadEqualityRows>,
    budget: filament_contract_model::NativeJoinBudget,
    /// The request's domain packages, bound to the package's model
    /// selections.
    supplied: fsl_semantics::model::intake::PackageInput,
    /// The admitted package's bytes, which replay reads (FR-850).
    package_bytes: Vec<u8>,
    /// The handoff's identity file bytes, which replay reads.
    identity_bytes: Vec<u8>,
    /// The handoff's equality file bytes, when present.
    equality_bytes: Option<Vec<u8>>,
}

/// The handoff directory a `package` selection names, beneath the request's
/// directory.
fn handoff_directory(directory: &str) -> Result<String> {
    use std::path::Component;
    let path = Path::new(directory);
    if directory.is_empty()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(invalid(
            "package.directory",
            "must name a directory beneath the request directory",
        ));
    }
    Ok(directory.trim_end_matches('/').to_owned())
}

/// FR-850 "Admission": a handoff witness file that is absent or refused.
fn witness_refusal(file: &str, cause: &str, detail: &str) -> RunCause {
    RunCause::AnalyzePackage {
        code: fsl_foundation::Code::InvalidPackage,
        cause: Some(cause.to_owned()),
        pointer: None,
        message: format!("invalid_package/{cause}: {file}: {detail}"),
    }
}

fn conflicting_model(identity: &str) -> RunCause {
    RunCause::AnalyzePackage {
        code: fsl_foundation::Code::InvalidPackage,
        cause: Some("conflicting-definition".to_owned()),
        pointer: None,
        message: format!(
            "invalid_package/conflicting-definition: the models entry {identity} is not a domain \
             package the package records, or repeats one"
        ),
    }
}

/// The request-stage refusal of a package the strict reader refused.
fn package_refusal(refusal: fsl_package::PackageAdmissionRefusal) -> RunCause {
    let fsl_package::PackageAdmissionRefusal {
        code,
        cause,
        pointer,
        identity,
    } = refusal;
    let named = match (&identity, &pointer) {
        (Some(identity), _) => format!(": no models entry supplies {identity}"),
        (None, Some(pointer)) => format!(" at {pointer}"),
        (None, None) => String::new(),
    };
    let label = match &cause {
        Some(cause) => format!("{}/{cause}", code.as_str()),
        None => code.as_str().to_owned(),
    };
    RunCause::AnalyzePackage {
        code,
        cause,
        pointer,
        message: format!("{label}{named}"),
    }
}

/// FSL FR-1033 "Reading `memory`": each entry is a closed object with exactly
/// `parallel` and `model`, the model `sc`, `tso` or `ra`; a malformed one
/// refuses the whole request naming the member by its pointer.
fn read_memory(
    entries: &[serde_json::Value],
) -> Result<Vec<fsl_eval::simulation::protocol::MemorySelection>> {
    use fsl_foundation::MemoryModelName;

    entries
        .iter()
        .enumerate()
        .map(|(position, entry)| {
            let Some(object) = entry.as_object() else {
                return Err(invalid(
                    &format!("/memory/{position}"),
                    "a memory entry is an object",
                ));
            };
            if let Some(member) = object
                .keys()
                .find(|member| !matches!(member.as_str(), "parallel" | "model"))
            {
                return Err(invalid(
                    &format!("/memory/{position}/{member}"),
                    "a memory entry holds only parallel and model",
                ));
            }
            let parallel = object
                .get("parallel")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    invalid(
                        &format!("/memory/{position}/parallel"),
                        "a memory entry names its parallel as a string",
                    )
                })?;
            let model = match object.get("model").and_then(serde_json::Value::as_str) {
                Some("sc") => MemoryModelName::Sc,
                Some("tso") => MemoryModelName::Tso,
                Some("ra") => MemoryModelName::Ra,
                _ => {
                    return Err(invalid(
                        &format!("/memory/{position}/model"),
                        "a memory model is sc, tso or ra",
                    ));
                }
            };
            Ok(fsl_eval::simulation::protocol::MemorySelection {
                parallel: parallel.to_owned(),
                model,
            })
        })
        .collect()
}

/// FSL FR-1033 "Reading `memory`": an entry naming no outermost `parallel`
/// of a protocol of the package (a nested `parallel`, an attempt, a protocol
/// the package does not declare), and two entries naming one `parallel`,
/// refuse the request before any item runs.
fn check_memory(
    package: &fsl_package::CheckedPackage,
    memory: &[fsl_eval::simulation::protocol::MemorySelection],
) -> Result<()> {
    check_memory_using(memory, |protocol, entries| {
        package
            .graph()
            .protocol(protocol)
            .map(|checked| ResolvedMemory::resolve(checked, entries))
    })
}

/// Admit every request memory entry before resolving any item, using the
/// selected route's protocol facts and the same refusal pointers.
fn check_memory_using(
    memory: &[MemorySelection],
    resolve: impl Fn(
        &str,
        &[MemorySelection],
    ) -> Option<std::result::Result<ResolvedMemory, MemorySelectionRefusal>>,
) -> Result<()> {
    let refused = |member: String, reason: String| RunCause::AnalyzeMemory { member, reason };
    let mut protocols: Vec<&str> = Vec::new();
    for (position, entry) in memory.iter().enumerate() {
        let protocol = entry.parallel.split("::").next().unwrap_or_default();
        if resolve(protocol, &[]).is_none() {
            return Err(refused(
                format!("/memory/{position}/parallel"),
                format!("{} names no protocol of the package", entry.parallel),
            ));
        }
        if !protocols.contains(&protocol) {
            protocols.push(protocol);
        }
    }
    for protocol in protocols {
        let entries: Vec<_> = memory
            .iter()
            .filter(|entry| entry.parallel.split("::").next() == Some(protocol))
            .cloned()
            .collect();
        if let Some(Err(refusal)) = resolve(protocol, &entries) {
            let member = memory
                .iter()
                .position(|entry| entries.first().is_some_and(|first| first == entry))
                .map_or_else(
                    || "/memory".to_owned(),
                    |position| format!("/memory/{position}"),
                );
            return Err(refused(member, refusal.to_string()));
        }
    }
    Ok(())
}

/// The members FR-620 reads beside the program or package selection: the
/// supplied documents under their own identities, the request's default
/// model subject and the limits every item of the run shares.
struct RunMembers {
    documents: DocumentProvision,
    subject: SubjectSelection,
    limits: AnalyzeLimits,
}

/// Read `request`'s [`RunMembers`] through `intake`, refusing a malformed
/// or unprovisioned document, trace or recording and an incomplete
/// universe as FR-620 states.
fn read_run_members(
    request: &wire::AnalyzeRequest,
    intake: &mut super::Intake<'_>,
    replay: fsl_replay::ReplayLimits,
) -> Result<RunMembers> {
    let documents = read_documents(intake, &request.documents, request.items.as_deref())?;
    let mut limits = analyze_limits(&request.limits, request.deadline_ms);
    limits.replay = replay;
    let subject = read_subject(request.subject.clone(), &documents)?;
    Ok(RunMembers {
        documents,
        subject,
        limits,
    })
}

/// FR-620's item checks that need no compiled program: `recording` and
/// `population` belong to a global-conformance item, and FR-634's
/// `simulation` member to a simulation item alone.
pub(super) fn validate_items(items: Option<&[ItemSelection]>) -> Result<()> {
    for (position, item) in items.unwrap_or_default().iter().enumerate() {
        if item.kind.as_deref() == Some(fsl_semantics::check::Capability::Composition.to_wire()) {
            if let Some(premises) = &item.premises {
                validate_composition_premises(position, premises)
                    .map_err(|(pointer, reason)| invalid(&pointer, reason))?;
            }
        }
        if item.recording.is_some()
            && item.kind.as_deref()
                != Some(fsl_semantics::check::Capability::GlobalConformance.to_wire())
        {
            return Err(invalid(
                &format!("/items/{position}/recording"),
                "recording is only valid on a global-conformance item",
            ));
        }
        if let Some(population) = &item.population {
            if item.kind.as_deref()
                != Some(fsl_semantics::check::Capability::GlobalConformance.to_wire())
            {
                return Err(invalid(
                    &format!("/items/{position}/population"),
                    "population is only valid on a global-conformance item",
                ));
            }
            if item.recording.is_none() {
                return Err(invalid(
                    &format!("/items/{position}/population"),
                    "population requires a recording",
                ));
            }
            validate_activated_population(position, population)
                .map_err(|(pointer, reason)| invalid(&pointer, reason))?;
            serde_json::from_value::<Vec<fsl_analyze::analyze::document::ActivatedRolePopulation>>(
                population.clone(),
            )
            .map_err(|error| {
                invalid(&format!("/items/{position}/population"), error.to_string())
            })?;
        }
    }
    // FR-634: a `simulation` item names no claim and carries its
    // `simulation` member, and only it does; `seed` and `samples` belong to
    // `sample` only.
    for (position, item) in items.unwrap_or_default().iter().enumerate() {
        let simulation =
            item.kind.as_deref() == Some(fsl_analyze::analyze::document::SIMULATION_KIND);
        match (&item.simulation, simulation) {
            (Some(_), false) => {
                return Err(invalid(
                    &format!("/items/{position}/simulation"),
                    "simulation is only valid on a simulation item",
                ));
            }
            (None, true) => {
                return Err(invalid(
                    &format!("/items/{position}/simulation"),
                    "a simulation item carries its simulation member",
                ));
            }
            (Some(settings), true) => {
                if item.claim.is_some() {
                    return Err(invalid(
                        &format!("/items/{position}/claim"),
                        "a simulation item settles no claim",
                    ));
                }
                if let Some(member) = settings.misplaced_member() {
                    return Err(invalid(
                        &format!("/items/{position}/simulation/{member}"),
                        format!("{member} belongs to a sample simulation only"),
                    ));
                }
            }
            (None, false) => {}
        }
    }
    Ok(())
}

/// Read the supplied `documents` through `intake`, each under its declared
/// or stated identity (FR-821), and refuse an item `trace` or `recording`
/// naming no provisioned document (FR-636).
pub(super) fn read_documents(
    intake: &mut super::Intake<'_>,
    selections: &[wire::DocumentFileSelection],
    items: Option<&[ItemSelection]>,
) -> Result<DocumentProvision> {
    let mut documents = DocumentProvision::new();
    let mut recording_documents = std::collections::BTreeSet::new();
    for (position, document) in selections.iter().enumerate() {
        let bytes = intake.file(&document.file, intake.remaining)?;
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
            invalid(
                &format!("/documents/{position}"),
                format!("{} is not a JSON document: {error}", document.file),
            )
        })?;
        let format = parsed.get("format").and_then(serde_json::Value::as_str);
        let declared_identity = match parsed.get("identity") {
            Some(value) => Some(
                serde_json::from_value::<fsl_analyze::analyze::document::DocumentSelection>(
                    value.clone(),
                )
                .map_err(|error| {
                    invalid(
                        &format!("/documents/{position}/identity"),
                        format!("document identity is malformed: {error}"),
                    )
                })?,
            ),
            None => None,
        };
        let stated_identity = match &document.identity {
            Some(value) => Some(
                serde_json::from_value::<fsl_analyze::analyze::document::DocumentSelection>(
                    value.clone(),
                )
                .map_err(|error| {
                    invalid(
                        &format!("/documents/{position}/identity"),
                        format!("document identity is malformed: {error}"),
                    )
                })?,
            ),
            None => None,
        };
        if format == Some("filament.state.snapshot/v1") && document.identity.is_some() {
            return Err(invalid(
                &format!("/documents/{position}/identity"),
                "a supplied snapshot uses its declared identity and cannot state a wrapper identity",
            ));
        }
        if let (Some(declared), Some(stated)) = (&declared_identity, &stated_identity) {
            if declared != stated {
                return Err(invalid(
                    &format!("/documents/{position}/identity"),
                    format!(
                        "stated identity {{{}, {}}} contradicts declared identity {{{}, {}}}",
                        stated.authority, stated.identity, declared.authority, declared.identity
                    ),
                ));
            }
        }
        let Some(identity) = declared_identity.as_ref().or(stated_identity.as_ref()) else {
            return Err(invalid(
                &format!("/documents/{position}"),
                format!("{} declares no identity and none was stated", document.file),
            ));
        };
        let reference = fsl_semantics::model::observation::DocumentRef {
            authority: identity.authority.clone(),
            identity: identity.identity.clone(),
        };
        if format == Some("filament.protocol-recording/1") {
            recording_documents.insert((reference.authority.clone(), reference.identity.clone()));
        }
        if documents.insert(reference, bytes).is_some() {
            return Err(invalid(
                &format!("/documents/{position}"),
                format!("{} repeats a provisioned document identity", document.file),
            ));
        }
    }
    // FR-636: a `trace` naming no provisioned document refuses the request.
    for (position, trace) in items
        .unwrap_or_default()
        .iter()
        .enumerate()
        .filter_map(|(position, item)| item.trace.as_ref().map(|trace| (position, trace)))
    {
        if !documents.contains_key(&trace.reference()) {
            return Err(invalid(
                "items",
                format!(
                    "items[{position}].trace {{{}, {}}} names no provisioned document",
                    trace.authority, trace.identity
                ),
            ));
        }
    }
    for (position, recording) in
        items
            .unwrap_or_default()
            .iter()
            .enumerate()
            .filter_map(|(position, item)| {
                item.recording
                    .as_ref()
                    .map(|recording| (position, recording))
            })
    {
        let identity = recording.reference();
        if !recording_documents.contains(&(identity.authority.clone(), identity.identity.clone())) {
            return Err(invalid(
                &format!("/items/{position}/recording"),
                format!(
                    "recording {{{}, {}}} does not name a provisioned protocol recording",
                    recording.authority, recording.identity,
                ),
            ));
        }
    }
    Ok(documents)
}

/// The limits every item of the run shares: FR-620's `limits` over the
/// unbounded defaults, and the run's deadline.
pub(super) fn analyze_limits(
    request: &wire::AnalyzeLimits,
    deadline_ms: Option<u64>,
) -> AnalyzeLimits {
    let mut limits = AnalyzeLimits::unbounded();
    let mut accounting = fsl_replay::AccountingLimits(limits.accounting);
    for (setting, bound) in request.accounting.entries() {
        accounting.set_bound(setting, bound);
    }
    limits.accounting = accounting.0;
    limits.observation = request.observation.resolve();
    limits.normalization = request.normalization.resolve();
    limits.deadline = deadline_ms.map_or(DeadlineSupply::Unbounded, DeadlineSupply::Milliseconds);
    limits
}

/// The request's model subject: `subject` completed by the default subject
/// over `documents`, or that default.
pub(super) fn read_subject(
    subject: Option<SubjectSelection>,
    documents: &DocumentProvision,
) -> Result<SubjectSelection> {
    // FR-620: an explicit `subject.universes` refuses an unprovisioned or
    // incomplete reference; the default subject's universes are the complete
    // populations its snapshots declare, so it refuses neither.
    let default_subject = SubjectSelection::default_over(documents);
    Ok(match subject {
        // A subject that states only its `protocol` keeps the default model
        // parts (FSL FR-1033).
        Some(subject) => {
            let subject = subject.completed_by(&default_subject);
            subject
                .check_universes(documents)
                .map_err(RunCause::AnalyzeUniverse)?;
            subject
        }
        None => default_subject,
    })
}
