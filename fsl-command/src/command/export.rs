// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1015 (FSpec FR-817): a `native-run/1` request's `export` member asks for
//! a TLA+ or Lean text of the program's checked package. This module reads the
//! member, compiles the program, runs the mapper, writes the
//! `filament.export-outcome/v1` document and, when a file is named and a
//! package was assembled, the target file.

mod lower;
mod map;
mod request;

use super::{
    check_sources, intake_with_bytes, output::NativeResult, output_file, wire, FileCounts,
    RequestOverrides, Result, RunCause, RunLimits, RunResult,
};
use filament_semantic_value::catalog::Category;
use fsl_replay::{OutcomeDocument, OutcomeStage};
use request::Request;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// FSpec FR-298's disposition of one record, as FR-269 names it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Disposition {
    Preserved,
    Conditional,
    Unrepresented,
    Refused,
}

impl Disposition {
    const fn name(self) -> &'static str {
        match self {
            Self::Preserved => "preserved",
            Self::Conditional => "conditional",
            Self::Unrepresented => "unrepresented",
            Self::Refused => "refused",
        }
    }
}

/// One FSpec FR-298 record: the obligation's disposition, its conditions and
/// causes, and the half-open byte regions of the text that represent it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Record {
    pub(super) obligation: String,
    pub(super) disposition: Disposition,
    pub(super) conditions: Vec<Value>,
    pub(super) causes: Vec<Value>,
    pub(super) regions: Vec<(usize, usize)>,
}

impl Record {
    fn value(&self) -> Value {
        json!({
            "obligation": self.obligation,
            "disposition": self.disposition.name(),
            "conditions": self.conditions,
            "causes": self.causes,
            "regions": self.regions.iter().map(|(start, end)| json!([start, end])).collect::<Vec<_>>(),
        })
    }

    /// Whether this record fails `floor` (FSpec FR-811 "The fidelity floor").
    fn fails(&self, floor: request::Floor) -> bool {
        match floor {
            request::Floor::Any => false,
            request::Floor::Conditional => matches!(
                self.disposition,
                Disposition::Unrepresented | Disposition::Refused
            ),
            request::Floor::Preserved => self.disposition != Disposition::Preserved,
        }
    }
}

/// What the mapper made of the request.
pub(super) enum Mapped {
    /// Text, its name map and one record per obligation.
    Assembled {
        text: String,
        name_map: Vec<Value>,
        records: Vec<Record>,
        /// The first obligation the target can state and this build does
        /// not lower.
        unlowered: Option<Unlowered>,
        /// The `export.work_units` the node charges spent.
        work: u64,
        /// The output regions the text writes, one `export.work_units` each.
        regions_written: u64,
    },
    /// A denied export charge: the setting that stopped it and its bound.
    Incomplete { setting: &'static str, bound: u64 },
}

/// An obligation the mapper met and does not lower (FSpec FR-811: a mapper
/// failure, never a record).
pub(super) struct Unlowered {
    pub(super) obligation: String,
    pub(super) construct: String,
    /// What the build does not lower; empty when the whole construct is not.
    pub(super) detail: String,
    /// The node the construct is at, and its FR-203 region.
    pub(super) node: String,
    pub(super) locus: Value,
    /// Whether the mapper broke its own invariant, a defect and not a
    /// construct it does not lower yet.
    pub(super) fault: bool,
}

/// The facts of the compiled program the outcome's `package` member names.
struct Program {
    identity: String,
    version: String,
}

/// Run the request at `directory`/`bytes` when it carries an `export` member.
///
/// # Errors
///
/// A refusal of the request, of its program, or a mapper failure.
pub(super) fn run_bytes(
    directory: &Path,
    bytes: &[u8],
    limits: RunLimits,
    overrides: &RequestOverrides<'_>,
) -> Result<Option<RunResult>> {
    let mut document: Value = serde_json::from_slice(bytes).map_err(RunCause::Json)?;
    if document.get("format").and_then(Value::as_str)
        != Some(fsl_foundation::wire_format::WireFormat::RunRequest.as_str())
    {
        return Ok(None);
    }
    let Some(body) = document.get_mut("request").and_then(Value::as_object_mut) else {
        return Ok(None);
    };
    let Some(member) = body.remove("export") else {
        return Ok(None);
    };
    // FR-1031: an export beside a transfer is a selection conflict.
    if body.contains_key("refinement_transfer") {
        let present: Vec<&str> = std::iter::once("export")
            .chain(
                super::TRANSFER_EXCLUSIVE
                    .into_iter()
                    .map(|(name, _)| name)
                    .filter(|name| body.contains_key(*name)),
            )
            .collect();
        return Err(super::transfer_conflict(&present));
    }
    let package_selection = body.get("package").cloned();
    let given = overrides
        .limits
        .map(|limits| limits.export)
        .unwrap_or_default();
    let request = request::read(&member, &given)?;
    for name in ["output", "handoff"] {
        if body.contains_key(name) {
            return Err(RunCause::ClauseRequest {
                member: name.to_owned(),
                reason: "a compile member is not an export member; name the file in export.output"
                    .to_owned(),
            });
        }
    }
    let output = target_path(directory, &request, overrides)?;
    let package_program = package_selection.as_ref().map(source_package).transpose()?;
    if let Some(observer) = &request.observer {
        return Ok(Some(unsupported_observer(
            &request,
            observer,
            output.as_deref(),
        )));
    }
    if let (Some(_), Some(program)) = (&package_selection, package_program) {
        document["format"] = json!(fsl_foundation::wire_format::WireFormat::RunRequest.as_str());
        let package_request_bytes = serde_json::to_vec(&document).map_err(RunCause::Output)?;
        let package_request: wire::Request = super::request(&package_request_bytes)?;
        super::package_run::validate_request(&package_request)?;
        let Some(package) = package_request.package.as_ref() else {
            return Err(RunCause::ExportRequest {
                path: "/request/package".to_owned(),
                cause: "wrong-value-kind",
                reason: "the package selection is not an object".to_owned(),
            });
        };
        let admitted = super::package_run::admit(
            directory,
            &package_request,
            package,
            limits.dependent_bytes,
            request.initial.as_deref(),
        )?;
        let clauses = map::package_clauses(&admitted.package)?;
        let models = admitted
            .models
            .iter()
            .map(|model| model.text().as_bytes().to_vec())
            .collect::<Vec<_>>();
        let initial = admitted
            .snapshot
            .as_deref()
            .map(|bytes| {
                let document: Value =
                    serde_json::from_slice(bytes).map_err(|error| RunCause::ClauseRequest {
                        member: "/request/export/initial".to_owned(),
                        reason: format!("the snapshot is not JSON: {error}"),
                    })?;
                if document.get("format").and_then(Value::as_str)
                    != Some("filament.state.snapshot/v1")
                {
                    return Err(RunCause::ClauseRequest {
                        member: "/request/export/initial".to_owned(),
                        reason: "the file is not a filament.state.snapshot/v1 document".to_owned(),
                    });
                }
                Ok(document)
            })
            .transpose()?;
        let mapped = map::run(
            &admitted.package,
            &program.identity,
            &clauses,
            &models,
            initial.as_ref(),
            &request,
            &request.limits.effective(),
        )?;
        return finish(&request, &mapped, &program, output.as_deref());
    }
    document["format"] = json!(fsl_foundation::wire_format::WireFormat::CompileRequest.as_str());
    let compile = serde_json::to_vec(&document).map_err(RunCause::Output)?;
    let compile: wire::CompileRequest = super::request(&compile)?;
    let mapped = compile_and_map(directory, &compile, &request, limits, overrides)?;
    finish(&request, &mapped.0, &mapped.1, output.as_deref())
}

/// Read the exact source identity/version pair an export request supplies
/// beside a checked package. Checked-package/v2 does not retain the source's
/// formal revision, so this route never derives it from package bytes.
fn source_package(value: &Value) -> Result<Program> {
    let refuse = |path: &str, cause, reason: &str| RunCause::ExportRequest {
        path: path.to_owned(),
        cause,
        reason: reason.to_owned(),
    };
    let Some(package) = value.as_object() else {
        return Err(refuse(
            "/request/package",
            "wrong-value-kind",
            "expected an object",
        ));
    };
    let Some(source) = package.get("source_package") else {
        return Err(refuse(
            "/request/package/source_package",
            "missing-member",
            "export over a checked package requires source_package",
        ));
    };
    let Some(source) = source.as_object() else {
        return Err(refuse(
            "/request/package/source_package",
            "wrong-value-kind",
            "expected an object",
        ));
    };
    if let Some(unknown) = source
        .keys()
        .find(|key| !matches!(key.as_str(), "identity" | "version"))
    {
        return Err(refuse(
            &format!("/request/package/source_package/{unknown}"),
            "unknown-member",
            "unknown source_package member",
        ));
    }
    let identity_path = "/request/package/source_package/identity";
    let identity = source
        .get("identity")
        .ok_or_else(|| refuse(identity_path, "missing-member", "required member is absent"))?;
    let Some(identity) = identity.as_str() else {
        return Err(refuse(
            identity_path,
            "wrong-value-kind",
            "expected a string",
        ));
    };
    if identity.is_empty() {
        return Err(refuse(identity_path, "invalid-value", "must not be empty"));
    }
    let version_path = "/request/package/source_package/version";
    let version = source
        .get("version")
        .ok_or_else(|| refuse(version_path, "missing-member", "required member is absent"))?;
    let Some(version) = version.as_str() else {
        return Err(refuse(
            version_path,
            "wrong-value-kind",
            "expected a string",
        ));
    };
    if version.is_empty() {
        return Err(refuse(version_path, "invalid-value", "must not be empty"));
    }
    Ok(Program {
        identity: identity.to_owned(),
        version: version.to_owned(),
    })
}

/// A valid `observer` this build cannot run (FSpec FR-817): `unsupported`,
/// raised after the request is read and its output path admitted, and before
/// any stage, so `last_stage` is null and no limit, floor or file applies.
fn unsupported_observer(request: &Request, observer: &str, output: Option<&Path>) -> RunResult {
    let path = "/request/export/observer";
    let reason = format!("observer `{observer}` is not run by this build");
    let mut export = json!({
        "format": "filament.export-outcome/v1",
        "mapping_profile_id": request.target.profile(),
        "target_family": request.target.family(),
        "status": "refused",
        "records": [],
        "refusal": {
            "code": "unsupported_projection",
            "cause": "unsupported-requested-capability",
            "capability": "observer",
            "observer": observer,
            "reason": reason,
            "details": {"path": path},
        },
    });
    if let Some(named) = output_name(request, output) {
        export["output"] = json!({"path": named, "bytes_written": 0});
    }
    let document = OutcomeDocument::new(fsl_replay::Operation::Export, None, Category::Unsupported)
        .with_export(export.clone())
        .with_diagnostics(vec![fsl_replay::OutcomeDiagnostic::new(
            Some("unsupported-requested-capability"),
            "unsupported_projection",
            None,
            reason,
        )
        .with_details(Some(serde_json::Map::from_iter([(
            "path".to_owned(),
            Value::String(path.to_owned()),
        )])))]);
    RunResult {
        exit_code: Category::Unsupported.exit_code(),
        value: NativeResult::export_document(json!({"export": export})),
        outcome: Some(document),
        requirements: None,
        sources: crate::Sources::default(),
    }
}

/// The path the outcome's `output` member names: the member's spelling, else
/// the caller's path.
fn output_name(request: &Request, output: Option<&Path>) -> Option<String> {
    request
        .output
        .clone()
        .or_else(|| output.map(|path| path.display().to_string()))
}

/// The file the export writes: the member's, the caller's, or none. A path
/// that exists refuses before any stage runs (FR-1015).
fn target_path(
    directory: &Path,
    request: &Request,
    overrides: &RequestOverrides<'_>,
) -> Result<Option<PathBuf>> {
    let path = match (&request.output, overrides.output) {
        (Some(_), Some(_)) => return Err(RunCause::OutputGivenTwice),
        (Some(member), None) => Some(directory.join(member)),
        (None, supplied) => supplied.map(Path::to_path_buf),
    };
    if let Some(path) = &path {
        if path.symlink_metadata().is_ok() {
            return Err(RunCause::OutputExists { path: path.clone() });
        }
    }
    Ok(path)
}

fn compile_and_map(
    directory: &Path,
    compile: &wire::CompileRequest,
    request: &Request,
    limits: RunLimits,
    overrides: &RequestOverrides<'_>,
) -> Result<(Mapped, Program)> {
    if compile.program.extraction {
        return Err(RunCause::ExtractionRequiresRun);
    }
    if compile.program.sources.len() != 1 {
        return Err(RunCause::ClauseRequest {
            member: "program".to_owned(),
            reason: "an export names one program source".to_owned(),
        });
    }
    let counts = FileCounts {
        programs: compile
            .libraries
            .as_ref()
            .map_or(0, Vec::len)
            .saturating_add(1),
        models: compile.models.as_ref().map_or(0, Vec::len),
        observations: usize::from(request.initial.is_some()),
        extensions: compile.extensions.len(),
    };
    let mut intake = intake_with_bytes(directory, counts, limits.dependent_bytes)?;
    let selection = &compile.program.sources[0];
    let source = intake.source(selection, "program.sources[0].source")?;
    super::require_native_header(source.source())?;
    let program = Program {
        identity: super::package_name(source.source().identity()),
        version: selection.identity.formal_revision.to_string(),
    };
    let initial = request
        .initial
        .as_deref()
        .map(|file| {
            let bytes = intake.file(file, intake.remaining)?;
            let refuse = |reason: String| RunCause::ClauseRequest {
                member: "/request/export/initial".to_owned(),
                reason,
            };
            let document = serde_json::from_slice::<Value>(&bytes)
                .map_err(|error| refuse(format!("`{file}` is not a JSON document: {error}")))?;
            if document.get("format").and_then(Value::as_str) != Some("filament.state.snapshot/v1")
            {
                return Err(refuse(format!(
                    "`{file}` is not a filament.state.snapshot/v1 document"
                )));
            }
            Ok(document)
        })
        .transpose()?;
    let spine_limits = overrides.limits.copied().unwrap_or_default();
    let effective = request.limits.effective();
    let mapped = check_sources(
        compile,
        &mut intake,
        std::slice::from_ref(&source),
        false,
        spine_limits,
        None,
        None,
        |_intake,
         checked,
         selections,
         _points,
         packages,
         _,
         _libraries,
         models,
         _inventory,
         failure| {
            let cancel = filament_exact::Cancel::new();
            let result = fsl_replay::spine::package_with_temporal_selections(
                &checked,
                selections,
                spine_limits.package,
                &cancel,
            );
            let emitted = result.map_err(failure)?.into_value();
            let admitted = fsl_package::admit_checked_package(
                emitted.package().bytes(),
                &packages,
                fsl_package::V2ReadLimits::bounded(
                    fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT,
                ),
            )
            .map_err(admission_cause)?;
            let graph = checked.package().graph();
            let clauses: Vec<map::ClauseFact> = graph
                .state_clauses()
                .iter()
                .map(|clause| map::ClauseFact {
                    name: clause.name().to_owned(),
                    obligation: (clause.kind() == fsl_forms::StateClauseKind::Invariant)
                        .then(|| clause.identity()),
                    construct: "state/state_clause",
                    bounded: !clause.population_bounds().is_empty(),
                })
                .chain(graph.temporal_clauses().iter().map(|node| map::ClauseFact {
                    name: node.clause().name().to_owned(),
                    obligation: Some(node.identity()),
                    construct: "temporal/temporal_clause",
                    bounded: false,
                }))
                .chain(
                    graph
                        .hybrid_claim_sites()
                        .iter()
                        .map(|site| map::ClauseFact {
                            name: site.name.clone(),
                            obligation: Some(site.identity.clone()),
                            construct: "temporal/temporal_clause",
                            bounded: false,
                        }),
                )
                .collect();
            map::run(
                &admitted,
                &program.identity,
                &clauses,
                &models,
                initial.as_ref(),
                request,
                &effective,
            )
        },
    )?;
    Ok((mapped, program))
}

fn finish(
    request: &Request,
    mapped: &Mapped,
    program: &Program,
    output: Option<&Path>,
) -> Result<Option<RunResult>> {
    let mut export = json!({
        "format": "filament.export-outcome/v1",
        "mapping_profile_id": request.target.profile(),
        "target_family": request.target.family(),
    });
    let category = match mapped {
        Mapped::Assembled {
            text,
            name_map,
            records,
            unlowered,
            work,
            regions_written,
        } => assembled(
            &mut export,
            request,
            program,
            records,
            (text, name_map),
            unlowered.as_ref(),
            work.saturating_add(*regions_written),
        )?,
        Mapped::Incomplete { setting, bound } => {
            incomplete(&mut export, setting, *bound);
            Category::Incomplete
        }
    };
    let written = match (mapped, category) {
        (Mapped::Assembled { text, .. }, Category::Success) => {
            if let Some(path) = output {
                output_file::write_new_file(path, text.as_bytes()).map_err(
                    |cause| match cause {
                        RunCause::Io { path, error } => RunCause::ExportOutput { path, error },
                        other => other,
                    },
                )?;
            }
            text.len()
        }
        _ => 0,
    };
    if let Some(named) = output_name(request, output) {
        export["output"] = json!({"path": named, "bytes_written": written});
    }
    let mut document = OutcomeDocument::new(
        fsl_replay::Operation::Export,
        Some(OutcomeStage::S4),
        category,
    )
    .with_export(export.clone());
    if export["refusal"]["code"] == "unimplemented_capability" {
        let refusal = &export["refusal"];
        let text = |member: &str| refusal[member].as_str().unwrap_or_default().to_owned();
        document = document.with_diagnostics(vec![fsl_replay::OutcomeDiagnostic::new(
            Some("unimplemented-meaning"),
            "unimplemented_capability",
            None,
            format!("the export does not lower {}", text("construct")),
        )
        .with_details(Some(serde_json::Map::from_iter([(
            "capability".to_owned(),
            Value::String(text("capability")),
        )])))]);
    }
    Ok(Some(RunResult {
        exit_code: category.exit_code(),
        value: NativeResult::export_document(json!({"export": export})),
        outcome: Some(document),
        requirements: None,
        sources: crate::Sources::default(),
    }))
}

/// A denied charge: the limit reached (FSpec FR-817 `limit-reached`), and no
/// records, text or package.
fn incomplete(export: &mut Value, setting: &str, bound: u64) {
    export["status"] = json!("incomplete");
    export["cause"] = json!({
        "type": "limit-reached",
        "limit": setting.strip_prefix("export.").unwrap_or(setting),
        "value": bound,
        "setting": setting,
    });
    export["records"] = json!([]);
}

/// Fill `export` for an assembled mapping. The stop order is admission,
/// classification, the floor, then the text (FR-1015): below the floor the
/// request is `refused` with no package and no text; a representable
/// obligation this build does not lower is `unimplemented_capability` (the
/// classification stops at its node, before the floor); a text over
/// `export.output_bytes` is `incomplete`.
fn assembled(
    export: &mut Value,
    request: &Request,
    program: &Program,
    records: &[Record],
    (text, name_map): (&str, &[Value]),
    unlowered: Option<&Unlowered>,
    work: u64,
) -> Result<Category> {
    let effective = request.limits.effective();
    if let Some(unlowered) = unlowered {
        if unlowered.fault {
            return Err(RunCause::ExportMapper {
                construct: unlowered.construct.clone(),
                reason: format!(
                    "obligation `{}` is representable in {} and the mapper broke its own invariant{}",
                    unlowered.obligation,
                    request.target.profile(),
                    if unlowered.detail.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", unlowered.detail)
                    }
                ),
            });
        }
        let family = request.target.family();
        export["status"] = json!("refused");
        export["refusal"] = json!({
            "code": "unimplemented_capability",
            "cause": "unimplemented-meaning",
            "capability": format!("export:{family}:{}", unlowered.construct),
            "target": family,
            "construct": unlowered.construct,
            "node": unlowered.node,
            "locus": unlowered.locus,
        });
        return Ok(Category::Unsupported);
    }
    export["records"] = json!(records.iter().map(Record::value).collect::<Vec<_>>());
    let failing: Vec<Value> = records
        .iter()
        .filter(|record| record.fails(request.floor))
        .map(Record::value)
        .collect();
    if !failing.is_empty() {
        export["status"] = json!("refused");
        export["refusal"] = json!({
            "code": "unsupported_projection",
            "cause": "below-fidelity-floor",
            "failing": failing,
        });
        return Ok(Category::Unsupported);
    }
    if work > effective.work_units {
        incomplete(export, "export.work_units", effective.work_units);
        return Ok(Category::Incomplete);
    }
    if u64::try_from(text.len()).map_or(true, |length| length > effective.output_bytes) {
        incomplete(export, "export.output_bytes", effective.output_bytes);
        return Ok(Category::Incomplete);
    }
    let application = fsl_replay::build_inventory(&fsl_route::Registry::new()).application;
    export["status"] = json!("assembled");
    export["package"] = json!({
        "source_package": {"identity": program.identity, "version": program.version},
        "target_profile": request.target.profile(),
        "generator": {"identity": application.name, "version": application.version},
        "limits": {
            "export.output_bytes": effective.output_bytes,
            "export.nodes": effective.nodes,
            "export.work_units": effective.work_units,
        },
    });
    export["module"] = json!(map::module_name(&program.identity));
    export["target_text"] = json!(text);
    export["name_map"] = json!(name_map);
    export["observer"] = Value::Null;
    Ok(Category::Success)
}

/// The cause of an admission refusal of the compiled package: the refusal's
/// own catalogued code, typed cause and pointer, and the identity at fault.
fn admission_cause(refusal: fsl_package::PackageAdmissionRefusal) -> RunCause {
    let fsl_package::PackageAdmissionRefusal {
        code,
        cause,
        pointer,
        identity,
        ..
    } = refusal;
    let label = cause.as_ref().map_or_else(
        || code.as_str().to_owned(),
        |cause| format!("{}/{cause}", code.as_str()),
    );
    let mut message = format!("the package the run compiled is not admitted: {label}");
    if let Some(pointer) = &pointer {
        message.push_str(&format!(" at {pointer}"));
    }
    if let Some(identity) = &identity {
        message.push_str(&format!(" for {identity}"));
    }
    RunCause::AnalyzePackage {
        code,
        cause,
        pointer,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::RunError;
    use fsl_foundation::Code;
    use ix_trace_rs::trace;

    /// FSpec FR-817-AC-17: a package the run compiled but admission refuses
    /// stops the export with the refusal's own code, cause and pointer and
    /// that code's exit, never as an `ExportMapper` failure (exit 30).
    #[test]
    #[trace("FSpec-TC-2040", "FSpec-FR-817-AC-17")]
    fn an_admission_refusal_keeps_its_own_code_cause_and_exit() {
        for (code, exit) in [
            (Code::InvalidPackage, 20),
            (Code::ResourceExhausted, 22),
            (Code::MissingImport, 20),
        ] {
            let cause = admission_cause(fsl_package::PackageAdmissionRefusal {
                limit: None,
                code,
                cause: Some("invalid-value".to_owned()),
                pointer: Some("/claim_names/2".to_owned()),
                identity: None,
            });
            assert_eq!(cause.code(), code);
            let message = cause.to_string();
            assert!(message.contains("invalid-value"), "{message}");
            assert!(message.contains("/claim_names/2"), "{message}");
            let error = RunError {
                cause,
                sources: crate::Sources::default(),
            };
            assert_eq!(error.category().exit_code(), exit, "{code:?}");
            assert_ne!(error.category().exit_code(), 30);
        }
    }
}
