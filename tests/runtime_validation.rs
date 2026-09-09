// SPDX-License-Identifier: AGPL-3.0-only
//! FR-007: real checked-model population validation, before predicate execution.

#[path = "runtime_validation_cases/bindings.rs"]
mod binding_cases;
#[path = "runtime_validation_cases/invocation.rs"]
mod invocation_cases;
#[path = "runtime_validation_cases/limits.rs"]
mod limit_cases;
#[path = "support/native_rule_model.rs"]
mod native_rule_model;
#[path = "runtime_validation_cases/values.rs"]
mod value_cases;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::{
    check, CheckBindings, CheckLimits, CheckedPackage, ClauseBinding,
};
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::DeclarationKey;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::runtime::{
    validate, ArtifactLimits, ExecutionSelection, FieldBinding, Invocation, InvocationDraft,
    ModelBinding, ObjectEntry, ObjectIdentity, ObservationSelection, Population, QualifiedName,
    RuntimeInput, Snapshot, SnapshotDraft, SnapshotRef, ValidationLimits, ValidationStatus,
    ValueBinding, ValueId, ValueNode,
};
use quire_spec_language::syntax::ClauseKind;
use quire_spec_language::{
    link_native, parse, ByteDigest, Code, Limits, LinkLimits, Phase, SourceIdentity,
};

fn symbol(name: &str) -> ir::SymbolName {
    native_rule_model::symbol(name)
}

fn authored_owner() -> ir::RequirementRef {
    ir::RequirementRef::parse("example/runtime-rules", "PopulationRule", 7).unwrap()
}

fn checked<'a>(models: &'a [NativeModel], expression: &str) -> CheckedPackage<'a> {
    checked_kind(models, expression, ClauseKind::Invariant)
}

fn checked_kind<'a>(
    models: &'a [NativeModel],
    expression: &str,
    kind: ClauseKind,
) -> CheckedPackage<'a> {
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
    let text = format!(
        "language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"example/rule-tests\" version \"1\" digest \"{}\";\n{keyword} Rule on {context} {{ {expression} }}\n{keyword} Other on {context} {{ true }}\n",
        models[0].digest()
    );
    let unit = parse(
        SourceIdentity {
            identity: "test:runtime-rule".into(),
            revision: "7".into(),
        },
        "runtime-rule.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("authored runtime rule parses before validation setup");
    let source = FormalSource::new(
        unit.source().clone(),
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("RuntimeRuleSource").unwrap(),
            ir::SourceRevision::new(7).unwrap(),
        ),
    );
    let linked = link_native(unit, models, LinkLimits::default())
        .expect("exact native model links before population validation");
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
    .expect("static type/definedness setup must succeed before runtime judgment");
    assert_eq!(checked.clauses().len(), 2);
    assert_eq!(checked.clauses()[0].binding().requirement, authored_owner());
    checked
}

fn object(model: &NativeModel, key: &str) -> ObjectIdentity {
    ObjectIdentity {
        model: model.environment().owner().clone(),
        record: symbol("Node"),
        universe: symbol("nodes"),
        key: key.into(),
    }
}

fn field(name: &str, value: u32) -> FieldBinding {
    FieldBinding {
        name: symbol(name),
        value: ValueId::new(value),
    }
}

fn draft(model: &NativeModel) -> SnapshotDraft {
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

fn snapshot(draft: SnapshotDraft) -> Snapshot {
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

fn selection(model: &NativeModel, snapshot: SnapshotRef) -> ExecutionSelection {
    ExecutionSelection {
        requirement: authored_owner(),
        clause: ir::ClauseId::new("population_rule").unwrap(),
        observation: ObservationSelection::Current {
            snapshot,
            self_object: object(model, "self"),
        },
    }
}

fn input(snapshot: Snapshot) -> RuntimeInput {
    RuntimeInput {
        snapshots: vec![snapshot],
        invocations: Vec::new(),
    }
}

#[test]
#[trace("TC-058", "TC-059", "FR-007-AC-6", "FR-007-AC-8")]
fn complete_current_context_retains_exact_checked_source_and_input() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "false");
    let artifact = snapshot(draft(&models[0]));
    let retained = artifact.bytes().to_vec();
    let selected = selection(&models[0], artifact.reference());
    let context = validate(
        &checked,
        input(artifact),
        selected.clone(),
        ValidationLimits::default(),
        || false,
    )
    .expect("valid self-cycle and all typed fields admit a context even for a false predicate");
    assert!(std::ptr::eq(context.checked(), &checked));
    assert_eq!(
        context.clause().binding().clause.as_str(),
        "population_rule"
    );
    assert_eq!(context.selection(), &selected);
    assert_eq!(context.input().snapshots[0].bytes(), retained);
    assert!(context.usage().work > 0);
}

#[test]
#[trace("TC-060", "FR-007-AC-7", "FR-007-AC-5")]
fn skipped_invalid_field_refuses_with_actual_native_and_model_loci() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "false");
    let mut malformed = draft(&models[0]);
    malformed.arena.push(ValueNode::Integer { value: 1001 });
    malformed.populations[0].objects[0].fields[0].value = ValueId::new(5);
    let artifact = snapshot(malformed);
    let selected = selection(&models[0], artifact.reference());
    let report = validate(
        &checked,
        input(artifact),
        selected,
        ValidationLimits::default(),
        || false,
    )
    .unwrap_err();
    assert_eq!(report.status, ValidationStatus::Refused);
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == Code::InvalidRuntimeInput)
        .expect("the skipped Version field is still checked against 0..1000");
    assert_eq!(diagnostic.phase, Phase::Validate);
    assert_eq!(
        diagnostic.source,
        *checked.linked().unit().source().identity()
    );
    assert!(diagnostic.runtime.is_some());
    assert!(diagnostic.related.iter().any(|location| {
        location.identity.owner == *models[0].environment().owner()
            && location.identity.key
                == DeclarationKey::Field {
                    record: symbol("Node"),
                    field: symbol("n"),
                }
    }));
    assert!(!diagnostic.is_incomplete());
}

#[test]
#[trace("TC-059", "FR-007-AC-1", "FR-007-AC-2", "FR-007-AC-8")]
fn missing_target_is_dangling_only_when_its_population_is_complete() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "true");
    for complete in [false, true] {
        let mut offered = draft(&models[0]);
        offered.populations[0].complete = complete;
        offered.arena[3] = ValueNode::Reference {
            identity: object(&models[0], "missing"),
        };
        let artifact = snapshot(offered);
        let selected = selection(&models[0], artifact.reference());
        let report = validate(
            &checked,
            input(artifact),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap_err();
        let (status, code) = if complete {
            (ValidationStatus::Refused, Code::DanglingReference)
        } else {
            (ValidationStatus::Incomplete, Code::IncompletePopulation)
        };
        assert_eq!(report.status, status);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code));
        if !complete {
            assert!(report
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != Code::DanglingReference));
        }
    }
}

#[test]
#[trace("TC-058", "FR-007-AC-6", "FR-007-AC-9")]
fn selection_refuses_foreign_clauses_stale_bytes_and_duplicate_inventory() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "true");
    let artifact = snapshot(draft(&models[0]));
    for variant in 0..5 {
        let mut offered = input(artifact.clone());
        let mut selected = selection(&models[0], artifact.reference());
        let (status, code) = match variant {
            0 => {
                selected.requirement =
                    ir::RequirementRef::parse("example/runtime-rules", "DifferentRule", 7).unwrap();
                (ValidationStatus::Refused, Code::InvalidModelBinding)
            }
            1 => {
                selected.clause = ir::ClauseId::new("absent_clause").unwrap();
                (ValidationStatus::Refused, Code::InvalidModelBinding)
            }
            2 => {
                selected = selection(
                    &models[0],
                    SnapshotRef::new(
                        artifact.identity().clone(),
                        ByteDigest::of(b"different input bytes"),
                    )
                    .unwrap(),
                );
                (ValidationStatus::Refused, Code::StaleDependency)
            }
            3 => {
                offered.snapshots.push(artifact.clone());
                (ValidationStatus::Refused, Code::InvalidRuntimeInput)
            }
            4 => {
                offered.snapshots.clear();
                (ValidationStatus::Incomplete, Code::UnavailableObservation)
            }
            _ => unreachable!(),
        };
        let report = validate(
            &checked,
            offered,
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap_err();
        assert_eq!(report.status, status, "selection variant {variant}");
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code));
    }
}

#[test]
#[trace("TC-064", "FR-007-AC-13")]
fn conflicting_population_order_preserves_defects_and_actual_byte_provenance() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "true");
    let mut offered = draft(&models[0]);
    let mut duplicate = offered.populations[0].clone();
    duplicate.complete = false;
    offered.populations.push(duplicate);
    let baseline = snapshot(offered.clone()).reference();
    let mut reports = Vec::new();
    for reverse in [false, true] {
        let mut current = offered.clone();
        if reverse {
            current.populations.reverse();
        }
        let artifact = snapshot(current);
        let expected = artifact.reference();
        let selected = selection(&models[0], expected.clone());
        let mut report = validate(
            &checked,
            input(artifact),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap_err();
        assert_eq!(report.status, ValidationStatus::Refused);
        for diagnostic in &mut report.diagnostics {
            let runtime = diagnostic.runtime.as_mut().unwrap();
            assert_eq!(
                runtime.artifact,
                quire_spec_language::runtime::RuntimeReference::Snapshot(expected.clone())
            );
            // Compare logical defects under each permutation's exact artifact
            // correspondence; the byte-significant reordered payload has a new digest.
            runtime.artifact =
                quire_spec_language::runtime::RuntimeReference::Snapshot(baseline.clone());
        }
        reports.push(report.diagnostics);
    }
    assert_eq!(reports[0], reports[1]);
}

#[test]
#[trace("TC-064", "FR-007-AC-6", "FR-007-AC-13")]
fn foreign_clause_diagnostic_keeps_requested_artifact_after_unrelated_inventory_defects() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "true");
    let artifact = snapshot(draft(&models[0]));
    let mut selected = selection(&models[0], artifact.reference());
    selected.clause = ir::ClauseId::new("foreign").unwrap();
    let unrelated = Snapshot::new(
        SourceIdentity {
            identity: "test:unrelated-duplicate".into(),
            revision: "2".into(),
        },
        draft(&models[0]),
        ArtifactLimits::default(),
    )
    .unwrap();
    let offered = RuntimeInput {
        snapshots: vec![artifact.clone(), unrelated.clone(), unrelated],
        invocations: Vec::new(),
    };
    let report = validate(
        &checked,
        offered,
        selected.clone(),
        ValidationLimits::default(),
        || false,
    )
    .unwrap_err();
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == Code::InvalidModelBinding)
        .unwrap();
    let runtime = diagnostic.runtime.as_ref().unwrap();
    assert_eq!(
        runtime.artifact,
        quire_spec_language::runtime::RuntimeReference::Snapshot(artifact.reference())
    );
    assert_eq!(runtime.clause, selected.clause);
    assert_eq!(runtime.requirement, selected.requirement);
    assert!(runtime.path.is_empty());
    assert!(runtime.observation.is_none());
    assert_eq!(diagnostic.span.start.byte, 0);
    assert_eq!(diagnostic.span.end.byte, 0);
}
