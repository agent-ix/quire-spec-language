// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-032: newly authored ConfigVersion fixtures through public native APIs.

use quire_contract_ir as ir;
use quire_spec_language::{
    formal_source::FormalSource,
    model_source::{read, ModelSourceLimits},
    native_model::{ModelLimits, NativeModel},
    runtime::{
        ArtifactLimits, FieldBinding, Invocation, InvocationDraft, ModelBinding, ObjectEntry,
        ObjectIdentity, Population, QualifiedName, Snapshot, SnapshotDraft, ValueBinding, ValueId,
        ValueNode,
    },
    ByteDigest, Source, SourceIdentity,
};
use serde_json::{json, Value};
use std::{io, path::Path};

/// Original native model source, independent of historical datatype artifacts.
pub const MODEL: &str = include_str!("model.json");

#[path = "cases.rs"]
mod cases;
pub use cases::Case;
use cases::{CaseSpec, Input, Row};

/// Every declared case, in deterministic generation order.
pub const CASES: &[Case] = cases::CASES;

fn symbol(name: &str) -> ir::SymbolName {
    ir::SymbolName::new(name).expect("static example symbol")
}

fn identity(case: Case, role: &str) -> SourceIdentity {
    SourceIdentity {
        identity: format!("ix://example/config-version/{role}/{}", case.id()),
        revision: "1".into(),
    }
}

/// Compile and admit the checked-in model using the production frontend.
pub fn model() -> io::Result<NativeModel> {
    let source = Source::read(
        SourceIdentity {
            identity: "ix://example/config-version/model".into(),
            revision: "1".into(),
        },
        "model.json",
        MODEL.as_bytes(),
        1_048_576,
    )
    .map_err(io::Error::other)?;
    let source = FormalSource::new(
        source,
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("ConfigVersionModelSource").unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    );
    read(source, "native-rule-model/1", ModelSourceLimits::default())
        .map_err(io::Error::other)?
        .admit(ModelLimits::default())
        .map_err(io::Error::other)
}

fn binding(model: &NativeModel) -> ModelBinding {
    ModelBinding {
        model: model.environment().owner().clone(),
        digest: model.digest(),
    }
}

fn object(model: &NativeModel, key: &str) -> ObjectIdentity {
    ObjectIdentity {
        model: model.environment().owner().clone(),
        record: symbol("ConfigVersion"),
        universe: symbol("config_history"),
        key: key.into(),
    }
}

fn push(arena: &mut Vec<ValueNode>, value: ValueNode) -> ValueId {
    let id = ValueId::new(u32::try_from(arena.len()).expect("bounded static fixture arena"));
    arena.push(value);
    id
}

fn snapshot(
    model: &NativeModel,
    case: Case,
    observation: ir::StateObservation,
    rows: &[Row],
    other: Option<&str>,
) -> io::Result<Snapshot> {
    let mut arena = Vec::new();
    let objects = rows
        .iter()
        .map(|row| {
            let Row {
                key,
                version,
                parent,
            } = *row;
            let version = push(&mut arena, ValueNode::Integer { value: version });
            let parent = match parent {
                Some(key) => {
                    let value = push(
                        &mut arena,
                        ValueNode::Reference {
                            identity: object(model, key),
                        },
                    );
                    push(&mut arena, ValueNode::Present { value })
                }
                None => push(&mut arena, ValueNode::Absent),
            };
            ObjectEntry {
                key: key.into(),
                fields: vec![
                    FieldBinding {
                        name: symbol("versionNumber"),
                        value: version,
                    },
                    FieldBinding {
                        name: symbol("parent"),
                        value: parent,
                    },
                ],
            }
        })
        .collect();
    let values = if let Some(key) = other {
        let value = push(
            &mut arena,
            ValueNode::Object {
                identity: object(model, key),
            },
        );
        vec![ValueBinding {
            declaration: QualifiedName {
                model: model.environment().owner().clone(),
                name: symbol("other"),
            },
            value,
        }]
    } else {
        Vec::new()
    };
    let role = match observation {
        ir::StateObservation::Current => "current",
        ir::StateObservation::Pre => "pre",
        ir::StateObservation::Post => "post",
    };
    Snapshot::new(
        identity(case, role),
        SnapshotDraft {
            observation,
            models: vec![binding(model)],
            populations: vec![Population {
                model: model.environment().owner().clone(),
                record: symbol("ConfigVersion"),
                universe: symbol("config_history"),
                complete: case.spec().complete,
                objects,
            }],
            values,
            arena,
        },
        ArtifactLimits::default(),
    )
    .map_err(io::Error::other)
}

fn source_selection(file: &str, identity: &SourceIdentity, text: &str, document: &str) -> Value {
    json!({"file":file,"identity":identity.identity,"revision":identity.revision,
        "digest":ByteDigest::of(text.as_bytes()).to_string(),"document":document,"formal_revision":1})
}

struct Program {
    native: String,
    document: String,
    owner: Value,
    selection: Value,
}

struct Syntax {
    keyword: &'static str,
    context: &'static str,
    point: Value,
}

fn write_program(directory: &Path, model: &NativeModel, case: Case) -> io::Result<Program> {
    let spec = case.spec();
    let clause = spec.clause;
    let syntax = match spec.input {
        Input::Update { .. } => Syntax {
            keyword: "post",
            context: "Config::ConfigVersion::attemptUpdate",
            point: json!({"kind":"post","operation":"attemptUpdate"}),
        },
        Input::Current { .. } => Syntax {
            keyword: "invariant",
            context: "Config::ConfigVersion at current",
            point: json!({"kind":"handler","name":"validate"}),
        },
    };
    let native = format!(
        "// SPDX-License-Identifier: AGPL-3.0-only\nlanguage \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel Config = \"example/config-version\" version \"1\" digest \"{}\";\n{} {} on {} {{ {} }}\n",
        model.digest(), syntax.keyword, clause.name, syntax.context, clause.expression,
    );
    std::fs::write(directory.join("program.native"), &native)?;
    let owner = json!({"package":"example/config-version","requirement":clause.name,"revision":1});
    let document = format!("ConfigVersion{}", case.id().replace('-', "_"));
    let selection = json!({
        "source":source_selection("program.native", &identity(case, "native"), &native, &document),
        "clauses":[{"name":clause.name,"owner":owner,"clause":clause.id,"point":syntax.point}]
    });
    Ok(Program {
        native,
        document,
        owner,
        selection,
    })
}

struct Inputs {
    snapshots: Vec<Value>,
    invocations: Vec<Value>,
    observation: Value,
}

fn write_snapshot(directory: &Path, index: usize, snapshot: &Snapshot) -> io::Result<Value> {
    let file = format!("snapshot-{index}.json");
    std::fs::write(directory.join(&file), snapshot.bytes())?;
    Ok(json!({"file":file,"reference":snapshot.reference()}))
}

fn write_inputs(directory: &Path, model: &NativeModel, case: Case) -> io::Result<Inputs> {
    match case.spec().input {
        Input::Current {
            rows,
            self_key,
            other,
        } => {
            let snapshot = snapshot(model, case, ir::StateObservation::Current, rows, other)?;
            Ok(Inputs {
                snapshots: vec![write_snapshot(directory, 0, &snapshot)?],
                invocations: Vec::new(),
                observation: json!({"kind":"current","snapshot":snapshot.reference(),
                    "self_object":object(model, self_key)}),
            })
        }
        Input::Update { pre, post } => {
            let pre = snapshot(model, case, ir::StateObservation::Pre, pre, None)?;
            let post = snapshot(model, case, ir::StateObservation::Post, post, None)?;
            write_update(directory, model, case, &pre, &post)
        }
    }
}

fn write_update(
    directory: &Path,
    model: &NativeModel,
    case: Case,
    pre: &Snapshot,
    post: &Snapshot,
) -> io::Result<Inputs> {
    let invocation = Invocation::new(
        identity(case, "invocation"),
        InvocationDraft {
            models: vec![binding(model)],
            context: QualifiedName {
                model: model.environment().owner().clone(),
                name: symbol("ConfigVersion"),
            },
            operation: symbol("attemptUpdate"),
            anchor: ir::AnchorName::new("attemptUpdate").expect("static anchor name"),
            self_object: object(model, "child"),
            pre: pre.reference(),
            post: post.reference(),
            parameters: Vec::new(),
            result: Some(ValueId::new(0)),
            created: Vec::new(),
            deleted: Vec::new(),
            arena: vec![ValueNode::Boolean { value: true }],
        },
        ArtifactLimits::default(),
    )
    .map_err(io::Error::other)?;
    std::fs::write(directory.join("invocation.json"), invocation.bytes())?;
    Ok(Inputs {
        snapshots: vec![
            write_snapshot(directory, 0, pre)?,
            write_snapshot(directory, 1, post)?,
        ],
        invocations: vec![json!({"file":"invocation.json","reference":invocation.reference()})],
        observation: json!({"kind":"invocation","invocation":invocation.reference()}),
    })
}

fn write_model(directory: &Path, model: &NativeModel, spec: CaseSpec) -> io::Result<Vec<Value>> {
    std::fs::write(directory.join("model.json"), MODEL)?;
    Ok(if spec.include_model {
        vec![
            json!({"format":"native-rule-model/1","source":source_selection(
            "model.json", model.source().source().identity(), MODEL, "ConfigVersionModelSource")}),
        ]
    } else {
        Vec::new()
    })
}

fn write_json(directory: &Path, file: &str, value: &Value) -> io::Result<()> {
    std::fs::write(directory.join(file), serde_json::to_vec_pretty(value)?)
}

fn write_markdown(
    directory: &Path,
    case: Case,
    program: &Program,
    mut job: Value,
) -> io::Result<()> {
    let mut lines = vec![
        "<!-- SPDX-License-Identifier: AGPL-3.0-only -->".to_owned(),
        "# ConfigVersion — café".to_owned(),
        String::new(),
        "## Invariants".to_owned(),
        String::new(),
        format!("### {}", case.spec().clause.id),
        "  ```ix:native".to_owned(),
    ];
    lines.extend(program.native.lines().map(|line| format!("  {line}")));
    lines.push("  ```".to_owned());
    lines.push(String::new());
    let markdown = lines.join("\r\n");
    std::fs::write(directory.join("rules.md"), &markdown)?;
    let selected = &mut job["request"]["program"];
    selected["source"] = source_selection(
        "rules.md",
        &identity(case, "spec"),
        &markdown,
        &format!("Markdown{}", program.document),
    );
    let extracted = identity(case, "extracted");
    selected["extraction"] = json!({"body":{"identity":extracted.identity,
        "revision":extracted.revision,"document":format!("Extracted{}", program.document),"formal_revision":1}});
    write_json(directory, "markdown-run.json", &job)
}

fn write_requests(
    directory: &Path,
    case: Case,
    models: Vec<Value>,
    program: Program,
    inputs: Inputs,
) -> io::Result<()> {
    let mut job = json!({"format":"native-run/1","request":{
        "models":models,"program":program.selection,
        "snapshots":inputs.snapshots,"invocations":inputs.invocations,
        "selection":{"owner":program.owner,"clause":case.spec().clause.id,"observation":inputs.observation}
    }});
    if let Some(steps) = case.spec().expression_steps {
        job["request"]["limits"] = json!({"expression_steps":steps});
    }
    write_json(directory, "request.json", &job)?;
    write_json(
        directory,
        "compile.json",
        &json!({"format":"native-compile/1","request":{"models":models,"program":program.selection}}),
    )?;
    write_markdown(directory, case, &program, job)
}

/// Emit one case through public constructors, propagating construction and I/O failures.
pub fn write(directory: &Path, model: &NativeModel, case: Case) -> io::Result<()> {
    std::fs::create_dir_all(directory)?;
    let models = write_model(directory, model, case.spec())?;
    let program = write_program(directory, model, case)?;
    let inputs = write_inputs(directory, model, case)?;
    write_requests(directory, case, models, program, inputs)
}
