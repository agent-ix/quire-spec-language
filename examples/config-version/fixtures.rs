// SPDX-License-Identifier: AGPL-3.0-only
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

/// Explicit authored scenarios; expected results live in the integration test.
#[derive(Clone, Copy, Debug)]
pub enum Case {
    Healthy,
    Violating,
    Absent,
    Cycle,
    SelfLoop,
    Distinct,
    Dangling,
    Incomplete,
    MissingModel,
    Exhausted,
    Unchanged,
    Changed,
    ForbiddenParent,
}

/// The complete runnable example population.
pub const CASES: [Case; 13] = [
    Case::Healthy,
    Case::Violating,
    Case::Absent,
    Case::Cycle,
    Case::SelfLoop,
    Case::Distinct,
    Case::Dangling,
    Case::Incomplete,
    Case::MissingModel,
    Case::Exhausted,
    Case::Unchanged,
    Case::Changed,
    Case::ForbiddenParent,
];

impl Case {
    /// Stable case-specific identity and output directory.
    pub fn id(self) -> &'static str {
        match self {
            Self::Healthy => "healthy-parent",
            Self::Violating => "violating-parent",
            Self::Absent => "absent-parent",
            Self::Cycle => "cycle",
            Self::SelfLoop => "self-loop",
            Self::Distinct => "distinct-identities",
            Self::Dangling => "dangling-parent",
            Self::Incomplete => "incomplete-population",
            Self::MissingModel => "missing-model",
            Self::Exhausted => "exhausted-work",
            Self::Unchanged => "unchanged-version",
            Self::Changed => "changed-version",
            Self::ForbiddenParent => "forbidden-parent-change",
        }
    }

    fn operation(self) -> bool {
        matches!(
            self,
            Self::Unchanged | Self::Changed | Self::ForbiddenParent
        )
    }

    fn clause(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Cycle | Self::SelfLoop => ("NoCycle", "no_cycle", "not reaches(self, self, parent)"),
            Self::Distinct => ("SameIdentity", "same_identity", "self = other"),
            Self::Unchanged | Self::Changed | Self::ForbiddenParent => ("VersionUnchanged", "version_unchanged", "self.versionNumber = pre(self.versionNumber)"),
            Self::Healthy | Self::Violating | Self::Absent | Self::Dangling | Self::Incomplete |
            Self::MissingModel | Self::Exhausted => ("ParentOrder", "parent_order", "present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber"),
        }
    }
}

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
pub fn model() -> NativeModel {
    let source = Source::read(
        SourceIdentity {
            identity: "ix://example/config-version/model".into(),
            revision: "1".into(),
        },
        "model.json",
        MODEL.as_bytes(),
        1_048_576,
    )
    .expect("checked-in model source");
    let source = FormalSource::new(
        source,
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("ConfigVersionModelSource").unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    );
    read(source, "native-rule-model/1", ModelSourceLimits::default())
        .expect("checked-in ConfigVersion model compiles")
        .admit(ModelLimits::default())
        .expect("checked-in ConfigVersion roles are valid")
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

fn snapshot(model: &NativeModel, case: Case, observation: ir::StateObservation) -> Snapshot {
    let mut rows = vec![("root", 1, None), ("child", 2, Some("root"))];
    match case {
        Case::Violating => rows[0].1 = 3,
        Case::Absent => rows.truncate(1),
        Case::Cycle => rows[0].2 = Some("child"),
        Case::SelfLoop => rows = vec![("child", 2, Some("child"))],
        Case::Distinct => rows = vec![("root", 2, None), ("child", 2, None)],
        Case::Dangling | Case::Incomplete => rows[1].2 = Some("missing"),
        Case::Changed if observation == ir::StateObservation::Post => rows[1].1 = 3,
        Case::ForbiddenParent if observation == ir::StateObservation::Post => rows[1].2 = None,
        Case::Healthy
        | Case::MissingModel
        | Case::Exhausted
        | Case::Unchanged
        | Case::Changed
        | Case::ForbiddenParent => {}
    }
    let mut arena = Vec::new();
    let objects = rows
        .into_iter()
        .map(|(key, version, parent)| {
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
    let values = if matches!(case, Case::Distinct) {
        let value = push(
            &mut arena,
            ValueNode::Object {
                identity: object(model, "root"),
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
                complete: !matches!(case, Case::Incomplete),
                objects,
            }],
            values,
            arena,
        },
        ArtifactLimits::default(),
    )
    .expect("example snapshots are structurally valid, including adverse semantic inputs")
}

fn source_selection(file: &str, identity: &SourceIdentity, text: &str, document: &str) -> Value {
    json!({"file":file,"identity":identity.identity,"revision":identity.revision,
        "digest":ByteDigest::of(text.as_bytes()).to_string(),"document":document,"formal_revision":1})
}

/// Emit one selected case. I/O errors propagate; static fixture errors are defects.
pub fn write(directory: &Path, model: &NativeModel, case: Case) -> io::Result<()> {
    std::fs::create_dir_all(directory)?;
    std::fs::write(directory.join("model.json"), MODEL)?;
    let (name, clause, expression) = case.clause();
    let (keyword, context, point) = if case.operation() {
        (
            "post",
            "Config::ConfigVersion::attemptUpdate",
            json!({"kind":"post","operation":"attemptUpdate"}),
        )
    } else {
        (
            "invariant",
            "Config::ConfigVersion at current",
            json!({"kind":"handler","name":"validate"}),
        )
    };
    let native = format!("// SPDX-License-Identifier: AGPL-3.0-only\nlanguage \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel Config = \"example/config-version\" version \"1\" digest \"{}\";\n{keyword} {name} on {context} {{ {expression} }}\n", model.digest());
    std::fs::write(directory.join("program.native"), &native)?;
    let owner = json!({"package":"example/config-version","requirement":name,"revision":1});
    let source_id = identity(case, "native");
    // SourceDocumentId is opaque; use a valid, explicit case-specific identifier.
    let document = format!("ConfigVersion{}", case.id().replace('-', "_"));
    let program = json!({"source":source_selection("program.native", &source_id, &native, &document),
        "clauses":[{"name":name,"owner":owner,"clause":clause,"point":point}]});
    let snapshots = if case.operation() {
        vec![
            snapshot(model, case, ir::StateObservation::Pre),
            snapshot(model, case, ir::StateObservation::Post),
        ]
    } else {
        vec![snapshot(model, case, ir::StateObservation::Current)]
    };
    let mut snapshot_files = Vec::new();
    for (index, snapshot) in snapshots.iter().enumerate() {
        let file = format!("snapshot-{index}.json");
        std::fs::write(directory.join(&file), snapshot.bytes())?;
        snapshot_files.push(json!({"file":file,"reference":snapshot.reference()}));
    }
    let (invocations, observation) = if case.operation() {
        let invocation = Invocation::new(
            identity(case, "invocation"),
            InvocationDraft {
                models: vec![binding(model)],
                context: QualifiedName {
                    model: model.environment().owner().clone(),
                    name: symbol("ConfigVersion"),
                },
                operation: symbol("attemptUpdate"),
                anchor: ir::AnchorName::new("attemptUpdate").unwrap(),
                self_object: object(model, "child"),
                pre: snapshots[0].reference(),
                post: snapshots[1].reference(),
                parameters: Vec::new(),
                result: Some(ValueId::new(0)),
                created: Vec::new(),
                deleted: Vec::new(),
                arena: vec![ValueNode::Boolean { value: true }],
            },
            ArtifactLimits::default(),
        )
        .expect("static recorded update fixture");
        std::fs::write(directory.join("invocation.json"), invocation.bytes())?;
        (
            vec![json!({"file":"invocation.json","reference":invocation.reference()})],
            json!({"kind":"invocation","invocation":invocation.reference()}),
        )
    } else {
        (
            Vec::new(),
            json!({"kind":"current","snapshot":snapshots[0].reference(),
        "self_object":object(model, if matches!(case, Case::Absent) {"root"} else {"child"})}),
        )
    };
    let models = if matches!(case, Case::MissingModel) {
        Vec::new()
    } else {
        vec![
            json!({"format":"native-rule-model/1","source":source_selection("model.json", model.source().source().identity(), MODEL, "ConfigVersionModelSource")}),
        ]
    };
    let mut job = json!({"format":"native-run/1","request":{"models":models,"program":program,
        "snapshots":snapshot_files,"invocations":invocations,"selection":{"owner":owner,"clause":clause,"observation":observation}}});
    if matches!(case, Case::Exhausted) {
        job["request"]["limits"] = json!({"expression_steps":0});
    }
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec_pretty(&job)?,
    )?;
    let compilation =
        json!({"format":"native-compile/1","request":{"models":models,"program":program}});
    std::fs::write(
        directory.join("compile.json"),
        serde_json::to_vec_pretty(&compilation)?,
    )?;
    let body: String = native.lines().map(|line| format!("  {line}\n")).collect();
    let markdown = format!("<!-- SPDX-License-Identifier: AGPL-3.0-only -->\n# ConfigVersion — café\n\n## Invariants\n\n### {clause}\n  ```ix:native\n{body}  ```\n").replace('\n', "\r\n");
    std::fs::write(directory.join("rules.md"), &markdown)?;
    let selected = &mut job["request"]["program"];
    selected["source"] = source_selection(
        "rules.md",
        &identity(case, "spec"),
        &markdown,
        &format!("Markdown{document}"),
    );
    let extracted = identity(case, "extracted");
    selected["extraction"] = json!({"body":{"identity":extracted.identity,"revision":extracted.revision,"document":format!("Extracted{document}"),"formal_revision":1}});
    std::fs::write(
        directory.join("markdown-run.json"),
        serde_json::to_vec_pretty(&job)?,
    )?;
    Ok(())
}
