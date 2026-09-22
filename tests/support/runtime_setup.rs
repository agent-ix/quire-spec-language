// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared public-API setup for native validation and reference execution tests.

// File-relative #[path], not `crate::support::native_rule_model` -- this file
// is also compiled directly into the library crate itself (`src/lib.rs`'s
// `#[cfg(test)] #[path = "../tests/support/runtime_setup.rs"] mod
// runtime_test_setup;`, QSL-175/#306 pre-dates and is out of this ticket's
// scope), where no `crate::support` exists. A self-relative `mod` resolves
// identically in both hosts.
#[path = "native_rule_model.rs"]
pub(crate) mod native_rule_model;

use quire_contract_ir as ir;
use quire_spec_language::checking::{
    check, CheckBindings, CheckLimits, CheckedPackage, ClauseBinding,
};
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::runtime::{
    ArtifactLimits, ExecutionSelection, FieldBinding, Invocation, InvocationDraft, ModelBinding,
    ObjectEntry, ObjectIdentity, ObservationSelection, Population, QualifiedName, RuntimeInput,
    Snapshot, SnapshotDraft, SnapshotRef, ValueId, ValueNode,
};
use quire_spec_language::syntax::ClauseKind;
use quire_spec_language::{link_native, parse, Limits, LinkLimits, SourceIdentity};

pub(crate) fn symbol(name: &str) -> ir::SymbolName {
    native_rule_model::symbol(name)
}

pub(crate) fn authored_owner() -> ir::RequirementRef {
    ir::RequirementRef::parse("example/runtime-rules", "PopulationRule", 7).unwrap()
}

pub(crate) fn checked<'a>(models: &'a [NativeModel], expression: &str) -> CheckedPackage<'a> {
    checked_kind(models, expression, ClauseKind::Invariant)
}

pub(crate) fn checked_kind<'a>(
    models: &'a [NativeModel],
    expression: &str,
    kind: ClauseKind,
) -> CheckedPackage<'a> {
    request(models, expression, kind).expect("static setup must succeed before runtime judgment")
}

pub(crate) fn request<'a>(
    models: &'a [NativeModel],
    expression: &str,
    kind: ClauseKind,
) -> Result<CheckedPackage<'a>, Box<quire_spec_language::Diagnostic>> {
    request_source(models, expression, kind, |text| text)
}

pub(crate) fn request_source<'a>(
    models: &'a [NativeModel],
    expression: &str,
    kind: ClauseKind,
    rewrite: impl FnOnce(String) -> String,
) -> Result<CheckedPackage<'a>, Box<quire_spec_language::Diagnostic>> {
    let (keyword, context, execution_point) = match kind {
        ClauseKind::Invariant => (
            "invariant",
            "M::Node at current",
            ir::ExecutionPoint::Handler {
                name: ir::AnchorName::new("validate").unwrap(),
            },
        ),
        ClauseKind::Precondition => (
            "pre",
            "M::Node::step",
            ir::ExecutionPoint::Pre {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        ),
        ClauseKind::Postcondition => (
            "post",
            "M::Node::step",
            ir::ExecutionPoint::Post {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        ),
    };
    let additional_imports: String = models
        .iter()
        .skip(1)
        .enumerate()
        .map(|(index, model)| {
            format!(
                "model N{index} = {} version \"{}\" digest \"{}\";\n",
                serde_json::to_string(model.environment().owner().package().as_str()).unwrap(),
                model.environment().owner().revision().get(),
                model.digest(),
            )
        })
        .collect();
    let text = format!(
        "language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"example/rule-tests\" version \"1\" digest \"{}\";\n{additional_imports}{keyword} Rule on {context} {{ {expression} }}\n{keyword} Other on {context} {{ true }}\n",
        models[0].digest()
    );
    let text = rewrite(text);
    let unit = parse(
        SourceIdentity {
            identity: "test:runtime-rule".into(),
            revision: "7".into(),
        },
        "runtime-rule.native",
        text.as_bytes(),
        Limits::default(),
    )?;
    let source = FormalSource::new(
        unit.source().clone(),
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("RuntimeRuleSource").unwrap(),
            ir::SourceRevision::new(7).unwrap(),
        ),
    );
    let linked =
        link_native(unit, models, LinkLimits::default()).map_err(|error| error.diagnostic)?;
    let clauses = [("Rule", "population_rule"), ("Other", "other_rule")]
        .into_iter()
        .map(|(name, clause)| ClauseBinding {
            name: name.into(),
            requirement: authored_owner(),
            clause: ir::ClauseId::new(clause).unwrap(),
            execution_point: execution_point.clone(),
        })
        .collect();
    let checked = check(
        linked,
        CheckBindings { source, clauses },
        CheckLimits::default(),
    )
    .map_err(|error| error.diagnostic)?;
    assert_eq!(checked.clauses().len(), 2);
    assert_eq!(checked.clauses()[0].binding().requirement, authored_owner());
    Ok(checked)
}

pub(crate) fn object(model: &NativeModel, key: &str) -> ObjectIdentity {
    ObjectIdentity {
        model: model.environment().owner().clone(),
        record: symbol("Node"),
        universe: symbol("nodes"),
        key: key.into(),
    }
}

pub(crate) fn field(name: &str, value: u32) -> FieldBinding {
    FieldBinding {
        name: symbol(name),
        value: ValueId::new(value),
    }
}

pub(crate) fn draft(model: &NativeModel) -> SnapshotDraft {
    SnapshotDraft {
        observation: ir::StateObservation::Current,
        models: vec![ModelBinding {
            model: model.environment().owner().clone(),
            digest: model.digest(),
        }],
        populations: vec![Population {
            model: model.environment().owner().clone(),
            record: symbol("Node"),
            universe: symbol("nodes"),
            complete: true,
            objects: vec![ObjectEntry {
                key: "self".into(),
                fields: vec![
                    field("n", 0),
                    field("signed", 1),
                    field("den", 0),
                    field("wide", 1),
                    field("count", 1),
                    field("distance", 0),
                    field("duration", 0),
                    field("parent", 2),
                    field("peer", 3),
                    field("items", 4),
                ],
            }],
        }],
        values: Vec::new(),
        arena: vec![
            ValueNode::Integer { value: 1 },
            ValueNode::Integer { value: 0 },
            ValueNode::Absent,
            ValueNode::Reference {
                identity: object(model, "self"),
            },
            ValueNode::Sequence { values: Vec::new() },
        ],
    }
}

pub(crate) fn snapshot(draft: SnapshotDraft) -> Snapshot {
    Snapshot::new(
        SourceIdentity {
            identity: "test:runtime-current".into(),
            revision: "1".into(),
        },
        draft,
        ArtifactLimits::default(),
    )
    .expect("flat input construction must succeed before model-aware validation")
}

pub(crate) fn selection(model: &NativeModel, snapshot: SnapshotRef) -> ExecutionSelection {
    ExecutionSelection {
        requirement: authored_owner(),
        clause: ir::ClauseId::new("population_rule").unwrap(),
        observation: ObservationSelection::Current {
            snapshot,
            self_object: object(model, "self"),
        },
    }
}

pub(crate) fn input(snapshot: Snapshot) -> RuntimeInput {
    RuntimeInput {
        snapshots: vec![snapshot],
        invocations: Vec::new(),
    }
}

pub(crate) fn authored_model(change: impl FnOnce(&mut serde_json::Value)) -> NativeModel {
    let mut data = serde_json::from_str(native_rule_model::FIXTURE).unwrap();
    change(&mut data);
    native_rule_model::from_text(
        &serde_json::to_string_pretty(&data).unwrap(),
        "runtime-operation-model.json",
        "1",
    )
    .expect("source-derived model roles are admitted before runtime setup")
    .model()
}

pub(crate) fn qualified(model: &NativeModel, name: &str) -> QualifiedName {
    QualifiedName {
        model: model.environment().owner().clone(),
        name: symbol(name),
    }
}

pub(crate) fn recorded(
    model: &NativeModel,
    mut before: SnapshotDraft,
    mut after: SnapshotDraft,
    change: impl FnOnce(&mut InvocationDraft),
) -> (RuntimeInput, ExecutionSelection) {
    before.observation = ir::StateObservation::Pre;
    after.observation = ir::StateObservation::Post;
    let mut snapshots = Vec::new();
    for (label, draft) in [("pre", before), ("post", after)] {
        snapshots.push(
            Snapshot::new(
                SourceIdentity {
                    identity: format!("test:runtime-{label}"),
                    revision: "1".into(),
                },
                draft,
                ArtifactLimits::default(),
            )
            .expect("operation snapshot structure must be valid"),
        );
    }
    let mut invocation = InvocationDraft {
        models: vec![ModelBinding {
            model: model.environment().owner().clone(),
            digest: model.digest(),
        }],
        context: qualified(model, "Node"),
        operation: symbol("step"),
        anchor: ir::AnchorName::new("step").unwrap(),
        self_object: object(model, "self"),
        pre: snapshots[0].reference(),
        post: snapshots[1].reference(),
        parameters: Vec::new(),
        result: Some(ValueId::new(0)),
        created: Vec::new(),
        deleted: Vec::new(),
        arena: vec![ValueNode::Boolean { value: true }],
    };
    change(&mut invocation);
    let invocation = Invocation::new(
        SourceIdentity {
            identity: "test:runtime-step".into(),
            revision: "1".into(),
        },
        invocation,
        ArtifactLimits::default(),
    )
    .expect("recorded invocation structure must be valid");
    let selected = ExecutionSelection {
        requirement: authored_owner(),
        clause: ir::ClauseId::new("population_rule").unwrap(),
        observation: ObservationSelection::Invocation {
            invocation: invocation.reference(),
        },
    };
    (
        RuntimeInput {
            snapshots,
            invocations: vec![invocation],
        },
        selected,
    )
}

pub(crate) fn change_field(draft: &mut SnapshotDraft, name: &str, value: ValueNode) {
    let index = u32::try_from(draft.arena.len()).unwrap();
    draft.arena.push(value);
    draft.populations[0].objects[0]
        .fields
        .iter_mut()
        .find(|field| field.name.as_str() == name)
        .unwrap()
        .value = ValueId::new(index);
}
