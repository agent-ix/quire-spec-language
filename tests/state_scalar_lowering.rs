// SPDX-License-Identifier: AGPL-3.0-only
//! FR-034: concrete native state reads, strict IR binding and validated input materialization.

// Existing shared fixtures use only the public compiler/runtime APIs.
#[allow(dead_code)]
#[path = "../examples/config-version/fixtures.rs"]
mod config;
#[allow(dead_code)]
#[path = "support/runtime_setup.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::{
    checking::{check, CheckBindings, CheckLimits, ClauseBinding},
    formal_source::FormalSource,
    link_native,
    lowering::{
        lower_for, InputProjectionCode, InputProjectionLimits, LoweringCode, LoweringLimits,
        PrimitiveValue, ProjectedReadOrigin, ProjectionTarget,
    },
    native_model::NativeModel,
    package::{NativePackage, PackageLimits},
    parse,
    runtime::{
        self, ArtifactLimits, ExecutionSelection, Invocation, ObservationSelection, RuntimeInput,
        RuntimeReference, Snapshot, ValidationLimits, ValueBinding, ValueId, ValueNode,
    },
    syntax::ClauseKind,
    Limits, LinkLimits, SourceIdentity,
};
use serde_json::{json, Value};
use std::{path::Path, process::Command};

const TARGET: ProjectionTarget = ProjectionTarget::StateScalarIrV1;

fn config_case<'m>(
    directory: &Path,
    models: &'m [NativeModel],
    case: config::Case,
) -> (NativePackage<'m>, RuntimeInput, ExecutionSelection) {
    config::write(directory, &models[0], case).unwrap();
    let job: Value =
        serde_json::from_slice(&std::fs::read(directory.join("request.json")).unwrap()).unwrap();
    let request = &job["request"];
    let source = &request["program"]["source"];
    let unit = parse(
        SourceIdentity {
            identity: source["identity"].as_str().unwrap().into(),
            revision: source["revision"].as_str().unwrap().into(),
        },
        "program.native",
        &std::fs::read(directory.join("program.native")).unwrap(),
        Limits::default(),
    )
    .unwrap();
    let formal = FormalSource::new(
        unit.source().clone(),
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new(source["document"].as_str().unwrap()).unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    );
    let requirement =
        ir::RequirementRef::parse("example/config-version", "VersionUnchanged", 1).unwrap();
    let clause = ir::ClauseId::new("version_unchanged").unwrap();
    let checked = check(
        link_native(unit, models, LinkLimits::default()).unwrap(),
        CheckBindings {
            source: formal,
            clauses: vec![ClauseBinding {
                name: "VersionUnchanged".into(),
                requirement: requirement.clone(),
                clause: clause.clone(),
                execution_point: ir::ExecutionPoint::Post {
                    operation: ir::AnchorName::new("attemptUpdate").unwrap(),
                },
            }],
        },
        CheckLimits::default(),
    )
    .unwrap();
    let snapshots = request["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            Snapshot::read_verified(
                &serde_json::from_value(entry["reference"].clone()).unwrap(),
                &std::fs::read(directory.join(entry["file"].as_str().unwrap())).unwrap(),
                ArtifactLimits::default(),
            )
            .unwrap()
        })
        .collect();
    let entry = &request["invocations"][0];
    let invocation = Invocation::read_verified(
        &serde_json::from_value(entry["reference"].clone()).unwrap(),
        &std::fs::read(directory.join("invocation.json")).unwrap(),
        ArtifactLimits::default(),
    )
    .unwrap();
    let selection = ExecutionSelection {
        requirement,
        clause,
        observation: ObservationSelection::Invocation {
            invocation: invocation.reference(),
        },
    };
    (
        NativePackage::new(checked, PackageLimits::default()).unwrap(),
        RuntimeInput {
            snapshots,
            invocations: vec![invocation],
        },
        selection,
    )
}

fn package<'m>(models: &'m [NativeModel], text: &str, kind: ClauseKind) -> NativePackage<'m> {
    NativePackage::new(
        setup::checked_kind(models, text, kind),
        PackageLimits::default(),
    )
    .unwrap()
}

#[test]
#[trace("TC-112", "FR-034-AC-1", "FR-034-AC-2", "FR-034-AC-5")]
fn concrete_config_update_binds_and_materializes_actual_pre_post_values() {
    let models = [config::model().unwrap()];
    let root = tempfile::tempdir().unwrap();
    for (case, post, truth) in [
        (config::Case::Unchanged, 2, true),
        (config::Case::Changed, 3, false),
    ] {
        let directory = root.path().join(case.id());
        let (native, input, selection) = config_case(&directory, &models, case);
        let projection = lower_for(&native, TARGET, LoweringLimits::default()).unwrap();
        let bound = projection.bound();
        assert_eq!(
            ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap(),
            *bound
        );
        let backend = ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap();
        assert_eq!(backend.digest().to_string(), bound.digest().to_string());
        let clause = &bound.clauses()[0];
        assert_eq!(clause.environment().values().len(), 1);
        assert_eq!(
            clause.environment().values()[0].value_type(),
            &ir::ValueType::integer(
                ir::IntegerType::new(
                    ir::IntegerDomain::Signed,
                    0,
                    1000,
                    ir::OverflowPolicy::Reject
                )
                .unwrap()
            )
        );
        let ir::ExpressionKind::Compare {
            operator: ir::ComparisonOperator::Equal,
            left,
            right,
        } = clause.expression().expression().kind()
        else {
            panic!("actual equality");
        };
        for (expression, observation) in [
            (left, ir::StateObservation::Post),
            (right, ir::StateObservation::Pre),
        ] {
            assert!(
                matches!(expression.kind(), ir::ExpressionKind::ValueReference { observation: actual, .. } if *actual == observation)
            );
            let source = &native.checked().bindings().source;
            assert_eq!(
                source
                    .source()
                    .slice(source.to_native(expression.source()).unwrap())
                    .unwrap(),
                "self.versionNumber"
            );
        }
        assert_eq!(projection.reads().len(), 2);
        for read in projection.reads() {
            assert_eq!(read.origin, ProjectedReadOrigin::SelfField);
            assert_eq!(read.model_digest, models[0].digest());
            assert_eq!(
                &read.declaration.identity.owner,
                models[0].environment().owner()
            );
            assert_eq!(read.native_observation, read.ir_observation);
            let source = models[0].source();
            let field: Value = serde_json::from_str(
                source
                    .source()
                    .slice(source.to_native(&read.declaration.source).unwrap())
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(field["name"], "versionNumber");
        }
        let context = runtime::validate(
            native.checked(),
            input,
            selection,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        let inputs = projection
            .inputs(&context, InputProjectionLimits::default(), || false)
            .unwrap();
        assert!(std::ptr::eq(inputs.context(), &context));
        assert!(std::ptr::eq(inputs.bound(), bound));
        assert_eq!(inputs.inputs().len(), 2);
        for input in inputs.inputs() {
            let observation = input.read().native_observation;
            assert_eq!(
                input.value(),
                PrimitiveValue::Integer(if observation == ir::StateObservation::Pre {
                    2
                } else {
                    post
                })
            );
            let snapshot = context.snapshot(observation).unwrap();
            assert_eq!(
                input.artifact(),
                RuntimeReference::Snapshot(snapshot.reference())
            );
            assert_eq!(input.object().unwrap().key, "child");
            assert_eq!(
                &input.object().unwrap().model,
                models[0].environment().owner()
            );
            assert!(matches!(
                snapshot.draft().arena[usize::try_from(input.arena_value().index()).unwrap()],
                ValueNode::Integer { .. }
            ));
        }
        assert!(
            matches!(runtime::evaluate(&context, runtime::EvaluationLimits::default(), || false).outcome(), runtime::EvaluationOutcome::Completed(actual) if *actual == truth)
        );
        let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
            .args(["lower"])
            .arg(directory.join("compile.json"))
            .args(["--target", "state-scalar-ir/v1"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, projection.bytes());
        for target in [
            ProjectionTarget::BooleanOracleV1,
            ProjectionTarget::IntegerIrV1,
        ] {
            assert_eq!(
                lower_for(&native, target, LoweringLimits::default())
                    .unwrap_err()
                    .code,
                LoweringCode::Unsupported
            );
        }
    }
    let (native, input, selection) = config_case(
        &root.path().join("forbidden"),
        &models,
        config::Case::ForbiddenParent,
    );
    assert!(lower_for(&native, TARGET, LoweringLimits::default()).is_ok());
    let failure = runtime::validate(
        native.checked(),
        input,
        selection,
        ValidationLimits::default(),
        || false,
    )
    .unwrap_err();
    assert!(failure
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == quire_spec_language::Code::FrameViolation));
}

#[test]
#[trace("TC-112", "FR-034-AC-1", "FR-034-AC-3")]
fn aliases_do_not_collide_and_direct_reads_keep_state_and_parameter_origins() {
    for input_kind in [false, true] {
        let models = [setup::authored_model(|model| {
            model["values"].as_array_mut().unwrap().extend([
                json!({"name":"nativeField0","kind":if input_kind {"input"} else {"state"},"type":{"kind":"scalar","name":"Version"}}),
                json!({"name":"flag","kind":if input_kind {"input"} else {"state"},"type":{"kind":"boolean"}}),
            ]);
            if input_kind {
                model["operations"][0]["parameters"] = json!(["nativeField0", "flag"]);
            }
        })];
        let native = package(
            &models,
            "flag and nativeField0 = 1 and self.n = pre(self.n)",
            ClauseKind::Postcondition,
        );
        let projection = lower_for(&native, TARGET, LoweringLimits::default()).unwrap();
        let fields: Vec<_> = projection
            .reads()
            .iter()
            .filter(|read| read.origin == ProjectedReadOrigin::SelfField)
            .collect();
        assert_eq!(fields.len(), 2);
        assert!(fields
            .iter()
            .all(|read| read.name.as_str() == "nativeField1"));
        let mut draft = setup::draft(&models[0]);
        if !input_kind {
            draft.arena.push(ValueNode::Boolean { value: true });
            draft.values = vec![
                ValueBinding {
                    declaration: setup::qualified(&models[0], "nativeField0"),
                    value: ValueId::new(0),
                },
                ValueBinding {
                    declaration: setup::qualified(&models[0], "flag"),
                    value: ValueId::new(5),
                },
            ];
        }
        let (input, selection) = setup::recorded(&models[0], draft.clone(), draft, |invocation| {
            if input_kind {
                invocation.arena.push(ValueNode::Integer { value: 1 });
                invocation.parameters = vec![
                    ValueBinding {
                        declaration: setup::qualified(&models[0], "nativeField0"),
                        value: ValueId::new(1),
                    },
                    ValueBinding {
                        declaration: setup::qualified(&models[0], "flag"),
                        value: ValueId::new(0),
                    },
                ];
            }
        });
        let context = runtime::validate(
            native.checked(),
            input.clone(),
            selection.clone(),
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        let inputs = projection
            .inputs(&context, InputProjectionLimits::default(), || false)
            .unwrap();
        assert_eq!(inputs.inputs().len(), 4);
        let direct: Vec<_> = inputs
            .inputs()
            .iter()
            .filter(|input| input.object().is_none())
            .collect();
        assert_eq!(direct.len(), 2);
        assert_eq!(
            direct
                .iter()
                .map(|input| input.read().name.as_str())
                .collect::<Vec<_>>(),
            ["flag", "nativeField0"]
        );
        for input in direct {
            let flag = input.read().name.as_str() == "flag";
            assert_eq!(
                input.value(),
                if flag {
                    PrimitiveValue::Boolean(true)
                } else {
                    PrimitiveValue::Integer(1)
                }
            );
            assert_eq!(
                input.read().origin,
                ProjectedReadOrigin::Value(if input_kind {
                    ir::ValueDeclarationKind::Input
                } else {
                    ir::ValueDeclarationKind::State
                })
            );
            assert_eq!(
                input.read().native_observation,
                if input_kind {
                    ir::StateObservation::Pre
                } else {
                    ir::StateObservation::Post
                }
            );
            assert_eq!(
                input.read().ir_observation,
                if input_kind {
                    ir::StateObservation::Current
                } else {
                    ir::StateObservation::Post
                }
            );
            assert_eq!(
                input.artifact(),
                if input_kind {
                    RuntimeReference::Invocation(context.invocation().unwrap().reference())
                } else {
                    RuntimeReference::Snapshot(
                        context
                            .snapshot(ir::StateObservation::Post)
                            .unwrap()
                            .reference(),
                    )
                }
            );
        }
        let mut other = selection;
        other.clause = ir::ClauseId::new("other_rule").unwrap();
        let other = runtime::validate(
            native.checked(),
            input,
            other,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        assert!(projection
            .inputs(&other, InputProjectionLimits::default(), || false)
            .unwrap()
            .inputs()
            .is_empty());
    }
}

#[test]
#[trace("TC-112", "FR-034-AC-1", "FR-034-AC-3", "FR-034-AC-4")]
fn observation_reads_limits_cancellation_and_context_mismatch_are_atomic() {
    let models = [setup::authored_model(|model| {
        model["values"].as_array_mut().unwrap().push(
            json!({"name":"amount","kind":"state","type":{"kind":"scalar","name":"Version"}}),
        );
    })];
    let text = "amount = pre(amount) and (pre((self))).n = pre(self.n)";
    let native = package(&models, text, ClauseKind::Postcondition);
    let projection = lower_for(&native, TARGET, LoweringLimits::default()).unwrap();
    assert_eq!(projection.reads().len(), 3); // amount pre/post plus one deduplicated field pre.
    let mut draft = setup::draft(&models[0]);
    draft.values.push(ValueBinding {
        declaration: setup::qualified(&models[0], "amount"),
        value: ValueId::new(0),
    });
    let (input, selection) = setup::recorded(&models[0], draft.clone(), draft, |_| {});
    let context = runtime::validate(
        native.checked(),
        input.clone(),
        selection.clone(),
        ValidationLimits::default(),
        || false,
    )
    .unwrap();
    let good = projection
        .inputs(&context, InputProjectionLimits::default(), || false)
        .unwrap();
    assert_eq!(
        good.inputs()
            .iter()
            .map(|input| input.value())
            .collect::<Vec<_>>(),
        vec![PrimitiveValue::Integer(1); 3]
    );
    for work in [0, good.work() - 1] {
        assert_eq!(
            projection
                .inputs(&context, InputProjectionLimits { work }, || false)
                .unwrap_err()
                .code,
            InputProjectionCode::ResourceExhausted
        );
    }
    let mut polls = 0;
    let stopped = projection
        .inputs(&context, InputProjectionLimits::default(), || {
            polls += 1;
            polls >= 4
        })
        .unwrap_err();
    assert_eq!(stopped.code, InputProjectionCode::Cancelled);
    assert!(stopped.work > 0);
    assert_eq!(
        projection
            .inputs(
                &context,
                InputProjectionLimits { work: good.work() },
                || false
            )
            .unwrap()
            .work(),
        good.work()
    );
    let copy = package(&models, text, ClauseKind::Postcondition);
    let other = runtime::validate(
        copy.checked(),
        input,
        selection,
        ValidationLimits::default(),
        || false,
    )
    .unwrap();
    let mismatch = projection
        .inputs(&other, InputProjectionLimits::default(), || false)
        .unwrap_err();
    assert_eq!(mismatch.code, InputProjectionCode::ContextMismatch);
    assert_eq!(mismatch.work, 0);
    assert!(mismatch.read.is_none());
    let usage = projection.usage();
    assert_eq!(
        usage.nodes,
        native.checked().linked().unit().expressions().len() + 1,
        "one fresh self.n alias candidate; repeated pre/group uses reuse it"
    );
    let exact = LoweringLimits {
        nodes: usage.nodes,
        depth: usage.max_depth,
        bytes: usage.bytes,
    };
    for limits in [
        LoweringLimits {
            nodes: exact.nodes - 1,
            ..exact
        },
        LoweringLimits {
            depth: exact.depth - 1,
            ..exact
        },
        LoweringLimits {
            bytes: exact.bytes - 1,
            ..exact
        },
    ] {
        assert_eq!(
            lower_for(&native, TARGET, limits).unwrap_err().code,
            LoweringCode::ResourceExhausted
        );
    }
    assert_eq!(
        lower_for(&native, TARGET, exact).unwrap().bytes(),
        projection.bytes()
    );
}

#[test]
#[trace("TC-112", "FR-034-AC-4", "FR-034-AC-5")]
fn nonprimitive_and_nonself_receivers_refuse_whole_packages() {
    let models = [setup::authored_model(|_| {})];
    for text in [
        "other.n < 7",
        "present(self.parent)",
        "self = other",
        "self.peer = self.peer",
        "let a = self in a.n < 7",
        "deref(self.peer).n < 7",
    ] {
        let native = package(&models, text, ClauseKind::Invariant);
        assert_eq!(
            lower_for(&native, TARGET, LoweringLimits::default())
                .unwrap_err()
                .code,
            LoweringCode::Unsupported,
            "{text}"
        );
    }
    let checked = setup::request_source(&models, "self.n < 7", ClauseKind::Invariant, |text| {
        text.replace("{ true }", "{ other.n < 7 }")
    })
    .unwrap();
    let native = NativePackage::new(checked, PackageLimits::default()).unwrap();
    let error = lower_for(&native, TARGET, LoweringLimits::default()).unwrap_err();
    assert_eq!(error.code, LoweringCode::Unsupported);
    assert_eq!(error.clause.unwrap().clause().as_str(), "other_rule");
}

#[test]
#[trace("TC-112", "FR-034-AC-1", "FR-034-AC-2", "FR-034-AC-3", "FR-034-AC-4")]
fn current_and_pre_context_fields_keep_boolean_values_and_distinct_aliases() {
    let models = [setup::authored_model(|model| {
        model["records"][0]["fields"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":"active","type":{"kind":"boolean"}}));
    })];
    for (kind, observation) in [
        (ClauseKind::Invariant, ir::StateObservation::Current),
        (ClauseKind::Precondition, ir::StateObservation::Pre),
    ] {
        let native = package(&models, "self.active and self.n = 1", kind);
        let mut draft = setup::draft(&models[0]);
        draft.arena.push(ValueNode::Boolean { value: false });
        draft.populations[0].objects[0]
            .fields
            .push(setup::field("active", 5));
        let (input, selection) = if kind == ClauseKind::Invariant {
            let snapshot = setup::snapshot(draft);
            let selection = setup::selection(&models[0], snapshot.reference());
            (setup::input(snapshot), selection)
        } else {
            setup::recorded(&models[0], draft.clone(), draft, |_| {})
        };
        let projection = lower_for(&native, TARGET, LoweringLimits::default()).unwrap();
        let context = runtime::validate(
            native.checked(),
            input,
            selection,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        let inputs = projection
            .inputs(&context, InputProjectionLimits::default(), || false)
            .unwrap();
        assert_eq!(
            inputs
                .inputs()
                .iter()
                .map(|input| input.value())
                .collect::<Vec<_>>(),
            vec![PrimitiveValue::Boolean(false), PrimitiveValue::Integer(1)]
        );
        assert_ne!(
            inputs.inputs()[0].read().name,
            inputs.inputs()[1].read().name
        );
        assert!(inputs
            .inputs()
            .iter()
            .all(|input| input.read().native_observation == observation));
        assert!(matches!(
            runtime::evaluate(&context, runtime::EvaluationLimits::default(), || false).outcome(),
            runtime::EvaluationOutcome::Completed(false)
        ));
        assert_eq!(
            projection
                .inputs(&context, InputProjectionLimits::default(), || true)
                .unwrap_err()
                .code,
            InputProjectionCode::Cancelled
        );
    }
}

#[test]
#[trace("TC-112", "FR-034-AC-3", "FR-034-AC-4")]
fn captured_input_refusals_happen_before_a_validated_context_can_be_projected() {
    let models = [setup::authored_model(|model| {
        model["values"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":"flag","kind":"input","type":{"kind":"boolean"}}));
        model["operations"][0]["parameters"] = json!(["flag"]);
    })];
    // A current invariant cannot statically acquire invocation parameters.
    let rejected = setup::request(&models, "flag", ClauseKind::Invariant).unwrap_err();
    assert_eq!(rejected.code, quire_spec_language::Code::MissingDeclaration);
    assert_eq!(rejected.source.identity, "test:runtime-rule");

    let native = package(&models, "flag", ClauseKind::Postcondition);
    let projection = lower_for(&native, TARGET, LoweringLimits::default()).unwrap();
    let recorded = |include_parameter| {
        setup::recorded(
            &models[0],
            setup::draft(&models[0]),
            setup::draft(&models[0]),
            |invocation| {
                if include_parameter {
                    invocation.parameters.push(ValueBinding {
                        declaration: setup::qualified(&models[0], "flag"),
                        value: ValueId::new(0),
                    });
                }
            },
        )
    };
    let (input, selection) = recorded(true);
    let context = runtime::validate(
        native.checked(),
        input.clone(),
        selection.clone(),
        ValidationLimits::default(),
        || false,
    )
    .unwrap();
    let materialized = projection
        .inputs(&context, InputProjectionLimits::default(), || false)
        .unwrap();
    assert_eq!(
        materialized.work(),
        3,
        "one read, one actual parameter, one arena value"
    );
    assert_eq!(
        projection
            .inputs(&context, InputProjectionLimits { work: 3 }, || false)
            .unwrap()
            .work(),
        3
    );
    let exhausted = projection
        .inputs(&context, InputProjectionLimits { work: 2 }, || false)
        .unwrap_err();
    assert_eq!(exhausted.code, InputProjectionCode::ResourceExhausted);
    assert_eq!(exhausted.work, 2);
    assert!(exhausted.read.is_none());
    let [value] = materialized.inputs() else {
        panic!("one actual captured Boolean")
    };
    assert_eq!(value.value(), PrimitiveValue::Boolean(true));
    assert_eq!(
        value.artifact(),
        RuntimeReference::Invocation(context.invocation().unwrap().reference())
    );

    let mut without_invocation = input;
    without_invocation.invocations.clear();
    let missing = runtime::validate(
        native.checked(),
        without_invocation,
        selection.clone(),
        ValidationLimits::default(),
        || false,
    )
    .unwrap_err();
    assert_eq!(missing.status, runtime::ValidationStatus::Incomplete);
    let diagnostic = missing
        .diagnostics
        .iter()
        .find(|d| d.code == quire_spec_language::Code::UnavailableObservation)
        .expect("missing selected invocation is a validation failure");
    let location = diagnostic.runtime.as_ref().unwrap();
    assert_eq!(
        location.artifact,
        RuntimeReference::Invocation(context.invocation().unwrap().reference())
    );
    assert_eq!(location.requirement, selection.requirement);
    assert_eq!(location.clause, selection.clause);

    let snapshot = setup::snapshot(setup::draft(&models[0]));
    let current_selection = setup::selection(&models[0], snapshot.reference());
    let current = runtime::validate(
        native.checked(),
        setup::input(snapshot),
        current_selection,
        ValidationLimits::default(),
        || false,
    )
    .unwrap_err();
    assert_eq!(current.status, runtime::ValidationStatus::Refused);
    assert!(current
        .diagnostics
        .iter()
        .any(|d| d.code == quire_spec_language::Code::WrongSnapshot));

    let (input, selection) = recorded(false);
    let invocation = input.invocations[0].reference();
    let parameter = runtime::validate(
        native.checked(),
        input,
        selection.clone(),
        ValidationLimits::default(),
        || false,
    )
    .unwrap_err();
    assert_eq!(parameter.status, runtime::ValidationStatus::Refused);
    let diagnostic = parameter
        .diagnostics
        .iter()
        .find(|d| {
            d.code == quire_spec_language::Code::InvalidRuntimeInput
                && d.runtime.as_ref().is_some_and(|location| {
                    location.path
                        == [runtime::RuntimePathSegment::Parameter(setup::qualified(
                            &models[0], "flag",
                        ))]
                })
        })
        .expect("missing declared parameter remains a typed, located validation failure");
    let location = diagnostic.runtime.as_ref().unwrap();
    assert_eq!(location.artifact, RuntimeReference::Invocation(invocation));
    assert_eq!(location.requirement, selection.requirement);
    assert_eq!(location.clause, selection.clause);
    // None of these failed public validations yields a context to pass to inputs().
}

#[test]
#[trace("TC-112", "FR-034-AC-3", "FR-034-AC-5")]
fn every_projection_target_keeps_its_boolean_and_signed_integer_capability() {
    let models = [setup::authored_model(|model| {
        model["values"].as_array_mut().unwrap().extend([
            json!({"name":"flag","kind":"state","type":{"kind":"boolean"}}),
            json!({"name":"signedValue","kind":"state","type":{"kind":"scalar","name":"Signed"}}),
        ]);
    })];
    for (text, integer) in [
        ("flag and not false", false),
        ("flag and signedValue = -1", true),
    ] {
        let native = package(&models, text, ClauseKind::Invariant);
        let mut draft = setup::draft(&models[0]);
        draft.arena.extend([
            ValueNode::Boolean { value: true },
            ValueNode::Integer { value: -1 },
        ]);
        draft.values = vec![
            ValueBinding {
                declaration: setup::qualified(&models[0], "flag"),
                value: ValueId::new(5),
            },
            ValueBinding {
                declaration: setup::qualified(&models[0], "signedValue"),
                value: ValueId::new(6),
            },
        ];
        let snapshot = setup::snapshot(draft);
        let selection = setup::selection(&models[0], snapshot.reference());
        let context = runtime::validate(
            native.checked(),
            setup::input(snapshot),
            selection,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        for (target, supports_integer) in [
            (ProjectionTarget::BooleanOracleV1, false),
            (ProjectionTarget::IntegerIrV1, true),
            (ProjectionTarget::StateScalarIrV1, true),
        ] {
            let result = lower_for(&native, target, LoweringLimits::default());
            if integer && !supports_integer {
                let error = result.unwrap_err();
                assert_eq!(error.code, LoweringCode::Unsupported);
                assert_eq!(
                    error.code.code(),
                    quire_spec_language::Code::UnsupportedProjection
                );
                assert_eq!(error.clause.unwrap().clause().as_str(), "population_rule");
                continue;
            }
            let projection = result.unwrap();
            assert_eq!(
                ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap(),
                *projection.bound()
            );
            let values = projection
                .inputs(&context, InputProjectionLimits::default(), || false)
                .unwrap();
            let expected = if integer {
                vec![PrimitiveValue::Boolean(true), PrimitiveValue::Integer(-1)]
            } else {
                vec![PrimitiveValue::Boolean(true)]
            };
            assert_eq!(
                values
                    .inputs()
                    .iter()
                    .map(|v| v.value())
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(values
                .inputs()
                .iter()
                .all(|v| v.read().origin
                    == ProjectedReadOrigin::Value(ir::ValueDeclarationKind::State)));
            assert!(matches!(
                runtime::evaluate(&context, runtime::EvaluationLimits::default(), || false)
                    .outcome(),
                runtime::EvaluationOutcome::Completed(true)
            ));
        }
    }
}

#[test]
#[trace("TC-112", "FR-034-AC-1", "FR-034-AC-4")]
fn real_model_alias_collisions_consume_bounded_work_before_candidate_creation() {
    let models = [setup::authored_model(|model| {
        for index in 0..16 {
            model["values"].as_array_mut().unwrap().push(json!({
                "name":format!("nativeField{index}"), "kind":"state",
                "type":{"kind":"scalar", "name":"Version"}
            }));
        }
    })];
    let native = package(&models, "self.n = 1", ClauseKind::Invariant);
    let unit = native.checked().linked().unit();
    // Rule has equality, field, self and literal nodes; Other has one true node.
    // Sixteen real declaration collisions precede the successful seventeenth name.
    assert_eq!(unit.expressions().len(), 5);
    let limits = LoweringLimits {
        nodes: 5 + 17,
        ..LoweringLimits::default()
    };
    let exact = lower_for(&native, TARGET, limits).unwrap();
    assert_eq!(exact.usage().nodes, 22);
    assert_eq!(exact.usage().max_depth, 3);
    let [read] = exact.reads() else {
        panic!("one exact self field read")
    };
    assert_eq!(read.name.as_str(), "nativeField16");
    assert_eq!(read.origin, ProjectedReadOrigin::SelfField);
    assert_eq!(read.model_digest, models[0].digest());
    assert_eq!(
        &read.declaration.identity.owner,
        models[0].environment().owner()
    );

    // Three AST visits precede alias search. After sixteen charged collisions,
    // the next candidate is refused at the original field without partial IR.
    let blocked = lower_for(
        &native,
        TARGET,
        LoweringLimits {
            nodes: 3 + 16,
            ..limits
        },
    )
    .unwrap_err();
    assert_eq!(blocked.code, LoweringCode::ResourceExhausted);
    assert_eq!(
        blocked.code.code(),
        quire_spec_language::Code::ResourceExhausted
    );
    assert_eq!(
        blocked.clause.as_ref().unwrap().clause().as_str(),
        "population_rule"
    );
    assert_eq!(
        unit.source().slice(blocked.source.unwrap()).unwrap(),
        "self.n"
    );
    let one_short = lower_for(
        &native,
        TARGET,
        LoweringLimits {
            nodes: 21,
            ..limits
        },
    )
    .unwrap_err();
    assert_eq!(one_short.code, LoweringCode::ResourceExhausted);
    assert_eq!(
        one_short.clause.as_ref().unwrap().clause().as_str(),
        "other_rule"
    );
    assert_eq!(
        unit.source().slice(one_short.source.unwrap()).unwrap(),
        "true"
    );
    assert_eq!(
        lower_for(&native, TARGET, limits).unwrap().bytes(),
        exact.bytes(),
        "fresh retry restarts alias work and preserves bytes"
    );
}
