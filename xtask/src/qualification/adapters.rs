// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public-operation adapters for the corpus-owned inputs of FR-350.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use qsl_foundation::diagnostic::StageFailure;
use qsl_foundation::{AccountingSetting, SettingLimits};
use qsl_replay::spine;
use qsl_replay::{Cancel, OutcomeDocument};
use qsl_semantics::model::{intake, observation};
use serde::Serialize;
use serde_json::{json, Value};

use super::{ConsumerType, InputFile, InputKind, Operation, Vector};

mod replay;

pub(super) struct Schemas {
    call: jsonschema::JSONSchema,
    replay: jsonschema::JSONSchema,
}

impl Schemas {
    pub(super) fn new(qspec: &Path, corpus: &Value) -> crate::error::Result<Self> {
        let path = qspec.join("proposals/checked-package-v2/schema.json");
        let bytes = fs::read(&path).map_err(|error| {
            crate::error::Error::QualificationRefusal(format!("{}: {error}", path.display()))
        })?;
        let package: Value = serde_json::from_slice(&bytes).map_err(|error| {
            crate::error::Error::QualificationRefusal(format!("{}: {error}", path.display()))
        })?;
        let compile = |name| {
            jsonschema::JSONSchema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .with_document("urn:quire:checked-package:v2".to_owned(), package.clone())
                .with_document("urn:quire:conformance-corpus:v1".to_owned(), corpus.clone())
                .compile(&json!({"$ref": format!("urn:quire:conformance-corpus:v1#/$defs/{name}")}))
                .map_err(|error| crate::error::Error::QualificationRefusal(error.to_string()))
        };
        Ok(Self {
            call: compile("CallInput")?,
            replay: compile("ReplayRequest")?,
        })
    }
}

#[derive(Serialize)]
pub(super) struct Failure {
    code: &'static str,
    cause: &'static str,
    pointer: String,
    message: String,
}

impl Failure {
    fn decode(pointer: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: "qualification_input",
            cause: "corpus-decode",
            pointer: pointer.into(),
            message: message.into(),
        }
    }

    fn io(path: &str, message: impl Into<String>) -> Self {
        Self {
            code: "input-io",
            cause: "input-io",
            pointer: path.to_owned(),
            message: message.into(),
        }
    }
}

pub(super) enum Partition {
    Result,
    Unsupported,
    Incomplete,
    ToolFailure,
}

pub(super) struct Execution {
    pub(super) actual: Value,
    pub(super) bytes: Option<Vec<u8>>,
    pub(super) partition: Partition,
}

struct Inputs<'a> {
    files: Vec<(&'a InputFile, Vec<u8>)>,
    unreadable_bytes: Vec<(&'a str, String)>,
}

impl<'a> Inputs<'a> {
    fn read(vector: &'a Vector, root: &Path) -> Result<Self, Failure> {
        let mut inputs = Vec::new();
        let mut unreadable_bytes = Vec::new();
        for file in &vector.input.files {
            let read = || -> Result<Vec<u8>, Failure> {
                let path = fs::canonicalize(root.join(&file.path))
                    .map_err(|error| Failure::io(&file.path, error.to_string()))?;
                if !path.starts_with(root) {
                    return Err(Failure::io(
                        &file.path,
                        "input resolves outside the corpus root",
                    ));
                }
                fs::read(path).map_err(|error| Failure::io(&file.path, error.to_string()))
            };
            match read() {
                Ok(bytes) => inputs.push((file, bytes)),
                Err(error)
                    if file.kind == InputKind::Bytes
                        && vector.subject.operation == Operation::Replay =>
                {
                    unreadable_bytes.push((file.path.as_str(), error.message))
                }
                Err(error) => return Err(error),
            }
        }
        Ok(Self {
            files: inputs,
            unreadable_bytes,
        })
    }

    fn one(&self, kind: InputKind) -> Result<(&InputFile, &[u8]), Failure> {
        let mut files = self.files.iter().filter(|(file, _)| file.kind == kind);
        let Some((file, bytes)) = files.next() else {
            return Err(Failure::decode(
                "/input/files",
                format!("missing {kind:?} input"),
            ));
        };
        if files.next().is_some() {
            return Err(Failure::decode(
                "/input/files",
                format!("more than one {kind:?} input"),
            ));
        }
        Ok((file, bytes))
    }

    fn models(&self) -> impl Iterator<Item = &[u8]> {
        self.files
            .iter()
            .filter(|(file, _)| file.kind == InputKind::Model)
            .map(|(_, bytes)| bytes.as_slice())
    }
}

fn document(bytes: &[u8], schema: &jsonschema::JSONSchema) -> Result<Value, Failure> {
    let value = super::parse_unique_json(bytes).map_err(|message| Failure::decode("", message))?;
    if let Err(mut errors) = schema.validate(&value) {
        if let Some(error) = errors.next() {
            let mut pointer = error.instance_path.to_string();
            if let jsonschema::error::ValidationErrorKind::Required { property } = &error.kind {
                if let Some(property) = property.as_str() {
                    pointer.push('/');
                    pointer.push_str(property);
                }
            }
            return Err(Failure::decode(pointer, error.to_string()));
        }
    }
    Ok(value)
}

fn unsigned(value: &Value, pointer: &str) -> Result<u64, Failure> {
    let text = value
        .as_str()
        .ok_or_else(|| Failure::decode(pointer, "expected an unsigned decimal string"))?;
    let number = text
        .parse::<u64>()
        .map_err(|_| Failure::decode(pointer, "unsigned decimal is outside u64"))?;
    if number.to_string() != text {
        return Err(Failure::decode(pointer, "noncanonical unsigned decimal"));
    }
    Ok(number)
}

fn text<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, Failure> {
    value
        .as_str()
        .ok_or_else(|| Failure::decode(pointer, "expected a string"))
}

fn array<'a>(value: &'a Value, pointer: &str) -> Result<&'a [Value], Failure> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| Failure::decode(pointer, "expected an array"))
}

fn object<'a>(
    value: &'a Value,
    pointer: &str,
) -> Result<&'a serde_json::Map<String, Value>, Failure> {
    value
        .as_object()
        .ok_or_else(|| Failure::decode(pointer, "expected an object"))
}

fn encode(value: impl Serialize) -> Result<Vec<u8>, Failure> {
    serde_json::to_vec(&value).map_err(|error| Failure::decode("", error.to_string()))
}

fn from_document(document: OutcomeDocument, actual: Option<Value>) -> Result<Execution, Failure> {
    let bytes = document
        .to_bytes()
        .map_err(|error| Failure::decode("", error.to_string()))?;
    let encoded: Value =
        serde_json::from_slice(&bytes).map_err(|error| Failure::decode("", error.to_string()))?;
    let partition = match encoded["category"].as_str() {
        Some("unsupported") => Partition::Unsupported,
        Some("incomplete" | "inconclusive" | "cancelled") => Partition::Incomplete,
        Some("internal-failure" | "internal_failure" | "fault") => Partition::ToolFailure,
        _ => Partition::Result,
    };
    let actual = actual.unwrap_or_else(|| {
        match encoded["diagnostics"]
            .as_array()
            .and_then(|diagnostics| diagnostics.first())
        {
            Some(diagnostic) => diagnostic.clone(),
            None => encoded["result"].clone(),
        }
    });
    Ok(Execution {
        actual,
        bytes: Some(bytes),
        partition,
    })
}

fn compile_refusal(refusal: &qsl_replay::ReplayRefusal) -> Result<Execution, Failure> {
    let diagnostic = match refusal {
        qsl_replay::ReplayRefusal::Recompile(refusal) => {
            qsl_replay::OutcomeDiagnostic::from_compile_refusal(refusal)
        }
        qsl_replay::ReplayRefusal::RecompileWithProtocolResults { refusal, .. } => {
            qsl_replay::OutcomeDiagnostic::from_compile_refusal(refusal)
        }
        _ => qsl_replay::OutcomeDiagnostic::new(
            refusal.cause(),
            refusal.code().as_str(),
            None,
            refusal.to_string(),
        ),
    };
    let actual =
        serde_json::to_value(diagnostic).map_err(|error| Failure::decode("", error.to_string()))?;
    Ok(Execution {
        actual,
        bytes: None,
        partition: Partition::Result,
    })
}

pub(super) fn execute(
    vector: &Vector,
    root: &Path,
    limits: &qsl_replay::CallerLimits,
    overrides: &[String],
    schemas: &Schemas,
) -> Result<Execution, Failure> {
    let inputs = Inputs::read(vector, root)?;
    match (vector.subject.consumer_type, vector.subject.operation) {
        (ConsumerType::Runtime, Operation::Replay) => {
            replay::execute(&inputs, root, limits, overrides, &schemas.replay)
        }
        (ConsumerType::Runtime, Operation::Run) => run(&inputs, limits, overrides, &schemas.call),
        (ConsumerType::Language, Operation::Compile) => {
            let (source, bytes) = inputs.one(InputKind::Source)?;
            match qsl_replay::compile_package(
                qsl_replay::CompileUnit {
                    source: qsl_foundation::SourceIdentity::new("agent-ix", &source.path),
                    package_name: "qualification".to_owned(),
                    path: &source.path,
                    bytes,
                },
                inputs.models(),
                &spine::DependencyInput::default(),
                qsl_replay::StageLimits::from_iter(limits.spine.bounds()),
                limits.replay,
            ) {
                Ok(compiled) => Ok(Execution {
                    actual: serde_json::from_slice(compiled.bytes())
                        .map_err(|error| Failure::decode("", error.to_string()))?,
                    bytes: Some(compiled.bytes().to_vec()),
                    partition: Partition::Result,
                }),
                Err(refusal) => compile_refusal(&refusal),
            }
        }
        (ConsumerType::Tool, Operation::Format) => {
            let (source, bytes) = inputs.one(InputKind::Source)?;
            let result = qsl_cst::parse(
                qsl_foundation::SourceIdentity::new("agent-ix", &source.path),
                &source.path,
                bytes,
                limits.spine.source,
            );
            match result {
                Ok(parsed) => match qsl_cst::format::format_with_limits(&parsed, limits.format) {
                    Ok(formatted) => Ok(Execution {
                        actual: json!(formatted),
                        bytes: Some(formatted.into_bytes()),
                        partition: Partition::Result,
                    }),
                    Err(refusal) => diagnostic(refusal.diagnostic()),
                },
                Err(refusal) => diagnostic(&refusal),
            }
        }
        _ => Err(Failure::decode(
            "/subject",
            "subject has no QSL public operation",
        )),
    }
}

fn diagnostic(diagnostic: &qsl_cst::diagnostic::CompleteDiagnostic) -> Result<Execution, Failure> {
    Ok(Execution {
        actual: json!({"code": diagnostic.code.as_str(), "cause": diagnostic.cause.as_str(), "locus": diagnostic.region.as_ref().map(|region| qsl_replay::OutcomeLocus::from(&qsl_foundation::diagnostic::Locus::Region(region.clone()))), "message": diagnostic.message}),
        bytes: None,
        partition: Partition::Result,
    })
}

fn front_end_failure(failure: spine::FrontEndFailure) -> Result<Execution, Failure> {
    match failure {
        StageFailure::RefusedWithPayload(refusal, _) => {
            front_end_failure(StageFailure::Refused(*refusal))
        }
        StageFailure::Refused(refusal) => {
            let result = Err(Box::new(spine::RunRefusal::Compile(refusal)));
            from_document(OutcomeDocument::from_run(&result), None)
        }
        StageFailure::Limit(limit) | StageFailure::LimitWithPayload(limit, _) => {
            let result = Err(Box::new(spine::RunRefusal::Compile(Box::new(
                spine::CompileRefusal::Limit(limit),
            ))));
            from_document(OutcomeDocument::from_run(&result), None)
        }
        StageFailure::Cancelled(cause) => from_document(
            OutcomeDocument::from_run(&Err(Box::new(spine::RunRefusal::Cancelled(cause)))),
            None,
        ),
        StageFailure::Fault(fault) => from_document(
            OutcomeDocument::from_run(&Err(Box::new(spine::RunRefusal::Fault(fault)))),
            None,
        ),
    }
}

fn run(
    inputs: &Inputs<'_>,
    limits: &qsl_replay::CallerLimits,
    overrides: &[String],
    schema: &jsonschema::JSONSchema,
) -> Result<Execution, Failure> {
    let (source, bytes) = inputs.one(InputKind::Source)?;
    let (_, call_bytes) = inputs.one(InputKind::Call)?;
    let call_input = document(call_bytes, schema)?;
    let function = text(&call_input["function"], "/function")?;
    let source_identity = qsl_foundation::SourceIdentity::new("agent-ix", &source.path);
    let packages = match intake::package_input(inputs.models()) {
        Ok(packages) => packages,
        Err(_) => {
            return match qsl_replay::compile_package(
                qsl_replay::CompileUnit {
                    source: source_identity,
                    package_name: "qualification".to_owned(),
                    path: &source.path,
                    bytes,
                },
                inputs.models(),
                &spine::DependencyInput::default(),
                qsl_replay::StageLimits::from_iter(limits.spine.bounds()),
                limits.replay,
            ) {
                Err(refusal) => compile_refusal(&refusal),
                Ok(_) => Err(Failure::decode(
                    "/input/files",
                    "model input changed during admission",
                )),
            };
        }
    };
    let cancel = Cancel::new();
    let parsed = match spine::parse(
        &spine::ParseRequest {
            source: &source_identity,
            path: &source.path,
            bytes,
        },
        limits.spine.source,
        &cancel,
    ) {
        Ok(parsed) => parsed.into_value(),
        Err(failure) => return front_end_failure(failure),
    };
    let selected = match spine::select(&parsed, &packages, limits.spine.model, &cancel) {
        Ok(selected) => selected.into_value(),
        Err(failure) => return front_end_failure(failure),
    };
    let checked = match spine::check(
        &parsed,
        "qualification".to_owned(),
        &selected,
        &spine::DependencyInput::default(),
        limits.spine,
        &cancel,
    ) {
        Ok(checked) => checked.into_value(),
        Err(failure) => return front_end_failure(failure),
    };
    let mut objects = qsl_replay::ObjectEnvironment::new(Default::default());
    let snapshots = inputs
        .0
        .iter()
        .filter(|(file, _)| matches!(file.kind, InputKind::ObjectContext | InputKind::Population))
        .collect::<Vec<_>>();
    if let Some((_, bytes)) = snapshots.first() {
        let selected_document = observation::document_reference(bytes).ok_or_else(|| {
            Failure::decode(
                "/input/files",
                "object context is not an identified snapshot",
            )
        })?;
        let provision = snapshots
            .iter()
            .map(|(_, bytes)| {
                observation::document_reference(bytes)
                    .map(|reference| (reference, bytes.to_vec()))
                    .ok_or_else(|| {
                        Failure::decode(
                            "/input/files",
                            "object context is not an identified snapshot",
                        )
                    })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let models = selected
            .models()
            .iter()
            .map(|model| model.view.model_selection().clone())
            .collect::<Vec<_>>();
        objects = match observation::admit_current_snapshot(
            &models,
            checked.package().graph().scope().types(),
            &packages,
            limits.spine.model,
            &provision,
            &selected_document,
            limits.observation,
        ) {
            Ok((objects, _)) => objects,
            Err(failure) => return admission_failure(failure),
        };
    }
    let signature = checked.package().graph().callable(function);
    let values = array(&call_input["arguments"], "/arguments")?;
    let mut arguments = Vec::new();
    if let Some(signature) = signature {
        for (index, value) in values.iter().enumerate() {
            let (parameter, _) = signature.parameters.get(index).ok_or_else(|| {
                Failure::decode(
                    format!("/arguments/{index}"),
                    "argument has no declared parameter",
                )
            })?;
            arguments.push(spine::CallArgument {
                parameter: parameter.clone(),
                value: call_value(value, &objects, &checked, index)?,
            });
        }
    }
    let mut accounting = qsl_replay::AccountingLimits::default();
    for (name, value) in object(&call_input["accounting"], "/accounting")? {
        let setting = AccountingSetting::ALL
            .iter()
            .copied()
            .find(|setting| setting.name() == format!("accounting.{name}"))
            .ok_or_else(|| {
                Failure::decode(format!("/accounting/{name}"), "unknown accounting counter")
            })?;
        accounting.set_bound(setting, unsigned(value, &format!("/accounting/{name}"))?);
    }
    for operand in overrides {
        if let Some((name, _)) = operand.split_once('=') {
            if let Some(qsl_foundation::SettingName::Accounting(setting)) =
                qsl_foundation::SettingName::from_name(name)
            {
                if let Some((_, bound)) = limits
                    .accounting
                    .bounds()
                    .into_iter()
                    .find(|(candidate, _)| *candidate == setting)
                {
                    accounting.set_bound(setting, bound);
                }
            }
        }
    }
    let call = spine::Call {
        function: function.to_owned(),
        arguments,
        accounting: accounting.0,
        supplied: Default::default(),
    };
    let result = spine::run(
        source_identity,
        "qualification".to_owned(),
        &source.path,
        bytes,
        &packages,
        &spine::DependencyInput::default(),
        limits.spine,
        &call,
        &objects,
        &cancel,
    );
    from_document(OutcomeDocument::from_run(&result), None)
}

fn admission_failure(failure: observation::AdmissionFailure) -> Result<Execution, Failure> {
    match failure {
        observation::AdmissionFailure::Refused(record)
        | observation::AdmissionFailure::Incomplete(record) => Ok(Execution {
            actual: json!({"code": record.catalog.code(), "cause": record.catalog.cause(), "fields": record.fields, "path": record.path.iter().map(|step| match step { observation::PathStep::Field(name) => json!({"kind":"field", "name":name}), observation::PathStep::OptionValue => json!({"kind":"option-value"}), observation::PathStep::Index(index) => json!({"kind":"index", "index":index}) }).collect::<Vec<_>>()}),
            bytes: None,
            partition: Partition::Result,
        }),
        observation::AdmissionFailure::Fault(fault) => Ok(Execution {
            actual: json!({"code": fault.catalog_code().code(), "cause": fault.catalog_code().cause()}),
            bytes: None,
            partition: Partition::ToolFailure,
        }),
    }
}

fn call_value(
    value: &Value,
    objects: &qsl_replay::ObjectEnvironment,
    _checked: &spine::CheckedUnit,
    index: usize,
) -> Result<qsl_replay::Value, Failure> {
    let pointer = format!("/arguments/{index}");
    match text(&value["kind"], &pointer)? {
        "boolean" => value["value"]
            .as_bool()
            .map(qsl_replay::Value::Boolean)
            .ok_or_else(|| Failure::decode(pointer, "expected a Boolean")),
        "integer" => {
            let spelling = text(&value["decimal"], &pointer)?;
            let integer = spelling
                .parse::<qsl_replay::Integer>()
                .map_err(|error| Failure::decode(&pointer, format!("{error:?}")))?;
            Ok(qsl_replay::Value::Integer(integer))
        }
        "population" => {
            let name = text(&value["name"], &pointer)?;
            let binding = objects
                .population_binding(name)
                .ok_or_else(|| Failure::decode(&pointer, "population was not admitted"))?;
            Ok(qsl_replay::Value::Population(binding.population_id()))
        }
        "reference" => {
            let graph = qsl_replay::WireGraph::default();
            let universe = graph
                .key_of_value(&value["universe"])
                .map_err(|message| Failure::decode(&pointer, message))?;
            let object_type = &value["type"];
            let identity = text(&value["object_identity"], &pointer)?;
            let reference = objects.references().iter().find(|reference| {
                let original = reference.object_type().original();
                reference.universe().root_type() == &universe
                    && reference.object().as_str() == identity
                    && object_type == &json!({"package": original.package(), "path": ["model", "object_type", original.node()], "ordinal": 0})
            }).ok_or_else(|| Failure::decode(&pointer, "reference is absent from the admitted object context"))?;
            Ok(qsl_replay::Value::Reference(reference.clone()))
        }
        _ => Err(Failure::decode(pointer, "unknown corpus input value")),
    }
}
