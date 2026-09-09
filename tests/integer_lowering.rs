// SPDX-License-Identifier: AGPL-3.0-only
//! FR-033: actual native integer derivation, strict binding and command export.

// Deliberate shared public-API fixture module, also used by the existing runtime tests.
#[allow(dead_code)]
#[path = "support/runtime_setup.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_contract_ir_backend as backend_ir;
use quire_spec_language::{
    lowering::{lower, lower_for, LoweringCode, LoweringLimits, ProjectionTarget},
    native_model::NativeModel,
    package::{NativePackage, PackageLimits},
    syntax::ClauseKind,
};
use serde_json::{json, Value};
use std::{path::Path, process::Command};

const TARGET: ProjectionTarget = ProjectionTarget::IntegerIrV1;

fn model(input: bool, maximum: i64) -> NativeModel {
    setup::authored_model(|model| {
        model["scalars"][0]["maximum"] = json!(maximum);
        for (name, ty) in [
            ("amount", json!({"kind":"scalar","name":"Version"})),
            ("delta", json!({"kind":"scalar","name":"Signed"})),
            ("distance", json!({"kind":"scalar","name":"Distance"})),
            ("flag", json!({"kind":"boolean"})),
        ] {
            model["values"].as_array_mut().unwrap().push(json!({
                "name":name,"kind":if input {"input"} else {"state"},"type":ty
            }));
        }
        if input {
            model["operations"][0]["parameters"] = json!(["amount", "delta", "distance", "flag"]);
        }
    })
}

fn package<'a>(models: &'a [NativeModel], text: &str, kind: ClauseKind) -> NativePackage<'a> {
    NativePackage::new(
        setup::checked_kind(models, text, kind),
        PackageLimits::default(),
    )
    .unwrap()
}

fn selected(bound: &ir::BoundPackage) -> &ir::BoundClause {
    bound
        .clauses()
        .iter()
        .find(|clause| clause.identity().clause().as_str() == "population_rule")
        .unwrap()
}

fn nodes<'a>(expression: &'a Value, result: &mut Vec<&'a Value>) {
    result.push(expression);
    for child in ["left", "right", "operand"] {
        if expression[child].is_object() {
            nodes(&expression[child], result);
        }
    }
}

#[test]
#[trace("TC-111", "FR-033-AC-1")]
fn strict_readers_reconstruct_integer_operators_bounds_and_guarded_obligations() {
    let models = [model(false, 1000)];
    for (expression, operator) in [
        ("amount < 1000 implies amount + 1 <= 1000", "add"),
        ("amount > 0 implies amount - 1 >= 0", "subtract"),
        ("amount <= 500 implies amount * 2 <= 1000", "multiply"),
        ("amount > 0 implies amount div amount = 1", "divide"),
        ("amount rem 1 = 0", "remainder"),
    ] {
        let native = package(&models, expression, ClauseKind::Invariant);
        let projection = lower_for(&native, TARGET, LoweringLimits::default())
            .unwrap_or_else(|error| panic!("{expression}: {error:?}"));
        assert_eq!(projection.profile(), "integer-ir/v1");
        assert!(std::ptr::eq(projection.native(), &native));
        assert_eq!(
            ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap(),
            *projection.bound()
        );
        let consumer = backend_ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap();
        assert_eq!(
            consumer.digest().to_string(),
            projection.bound().digest().to_string()
        );
        let clause = selected(projection.bound());
        assert!(
            !clause.expression().obligations().is_empty(),
            "{expression}"
        );
        let ir::ValueType::Integer { value } = clause.environment().values()[0].value_type() else {
            panic!("actual bounded integer declaration");
        };
        assert_eq!(
            (
                value.domain(),
                value.minimum(),
                value.maximum(),
                value.overflow()
            ),
            (
                ir::IntegerDomain::Signed,
                0,
                1000,
                ir::OverflowPolicy::Reject
            )
        );
        let wire: Value = serde_json::from_slice(projection.bytes()).unwrap();
        let binding = wire["bindings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|binding| binding["clause"]["clause"] == "population_rule")
            .unwrap();
        let mut expressions = Vec::new();
        nodes(&binding["expression"]["expression"], &mut expressions);
        let numeric = expressions
            .iter()
            .find(|node| node["node"] == "numeric")
            .unwrap();
        assert_eq!(numeric["operator"], operator, "{expression}");
        assert_eq!(numeric["left"]["name"], "amount");
        for literal in expressions
            .iter()
            .filter(|node| node["node"] == "integer_literal")
        {
            assert_eq!(
                literal["value_type"],
                json!({"kind":"integer","domain":"signed","minimum":0,"maximum":1000,"overflow":"reject"})
            );
        }
        assert_eq!(
            binding["expression"]["values"][0]["value_type"],
            json!({"kind":"integer","domain":"signed","minimum":0,"maximum":1000,"overflow":"reject"})
        );
    }
}

#[test]
#[trace("TC-111", "FR-033-AC-1")]
fn comparisons_negation_and_source_spans_preserve_authored_operand_order() {
    let models = [model(false, 1000)];
    for (spelling, operator) in [
        ("=", ir::ComparisonOperator::Equal),
        ("!=", ir::ComparisonOperator::NotEqual),
        ("<", ir::ComparisonOperator::Less),
        ("<=", ir::ComparisonOperator::LessEqual),
        (">", ir::ComparisonOperator::Greater),
        (">=", ir::ComparisonOperator::GreaterEqual),
    ] {
        let text = format!("amount {spelling} 7");
        let native = package(&models, &text, ClauseKind::Invariant);
        let projection = lower_for(&native, TARGET, LoweringLimits::default()).unwrap();
        let expression = selected(projection.bound()).expression().expression();
        let ir::ExpressionKind::Compare {
            operator: actual,
            left,
            right,
        } = expression.kind()
        else {
            panic!("actual comparison");
        };
        assert_eq!(*actual, operator);
        assert!(
            matches!(left.kind(), ir::ExpressionKind::ValueReference { name, .. } if name.as_str() == "amount")
        );
        assert!(matches!(
            right.kind(),
            ir::ExpressionKind::IntegerLiteral { value: 7, .. }
        ));
        let source = &native.checked().bindings().source;
        for (node, expected) in [
            (expression, text.as_str()),
            (left.as_ref(), "amount"),
            (right.as_ref(), "7"),
        ] {
            let span = source.to_native(node.source()).unwrap();
            assert_eq!(source.source().slice(span).unwrap(), expected);
        }
    }
    let native = package(&models, "-delta <= 10", ClauseKind::Invariant);
    let projection = lower_for(&native, TARGET, LoweringLimits::default()).unwrap();
    let ir::ExpressionKind::Compare { left, .. } = selected(projection.bound())
        .expression()
        .expression()
        .kind()
    else {
        panic!("comparison of negation");
    };
    assert!(matches!(
        left.kind(),
        ir::ExpressionKind::NumericNegate { .. }
    ));
    assert_eq!(
        ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap(),
        *projection.bound()
    );
}

#[test]
#[trace("TC-111", "FR-033-AC-2")]
fn direct_reads_keep_model_scalar_and_observation_correspondence() {
    for (input, kind, native_observation, ir_observation) in [
        (
            false,
            ClauseKind::Invariant,
            ir::StateObservation::Current,
            ir::StateObservation::Current,
        ),
        (
            false,
            ClauseKind::Precondition,
            ir::StateObservation::Pre,
            ir::StateObservation::Pre,
        ),
        (
            false,
            ClauseKind::Postcondition,
            ir::StateObservation::Post,
            ir::StateObservation::Post,
        ),
        (
            true,
            ClauseKind::Precondition,
            ir::StateObservation::Pre,
            ir::StateObservation::Current,
        ),
        (
            true,
            ClauseKind::Postcondition,
            ir::StateObservation::Pre,
            ir::StateObservation::Current,
        ),
    ] {
        let models = [model(input, 1000)];
        let native = package(&models, "amount < 1000 and distance >= 0", kind);
        let projection = lower_for(&native, TARGET, LoweringLimits::default()).unwrap();
        assert_eq!(projection.reads().len(), 2);
        for read in projection.reads() {
            assert_eq!(read.native_observation, native_observation);
            assert_eq!(read.ir_observation, ir_observation);
            assert_eq!(read.model_digest, models[0].digest());
            assert_eq!(
                read.declaration.identity.owner,
                *models[0].environment().owner()
            );
            let value = selected(projection.bound())
                .environment()
                .values()
                .iter()
                .find(|value| value.name() == &read.name)
                .unwrap();
            assert_eq!(value.source(), &read.declaration.source);
        }
        let distance = models[0]
            .roles()
            .scalars
            .iter()
            .find(|role| role.name.as_str() == "Distance")
            .unwrap();
        assert!(matches!(&distance.kind,
            quire_spec_language::native_model::ScalarKind::Integer {
                unit: quire_spec_language::native_model::Unit::Named(unit)
            } if unit.as_str() == "metre"));
    }
    let original = [model(false, 1000)];
    let changed = [model(false, 1001)];
    let a = package(&original, "amount < 7", ClauseKind::Invariant);
    let b = package(&changed, "amount < 7", ClauseKind::Invariant);
    assert_ne!(a.canonical_identity(), b.canonical_identity());
    assert_ne!(
        lower_for(&a, TARGET, LoweringLimits::default())
            .unwrap()
            .bound()
            .digest(),
        lower_for(&b, TARGET, LoweringLimits::default())
            .unwrap()
            .bound()
            .digest()
    );
}

#[test]
#[trace("TC-111", "FR-033-AC-3")]
fn target_refusals_and_exact_limits_remain_atomic_and_fresh() {
    let models = [model(false, 1000)];
    let native = package(&models, "amount < 7", ClauseKind::Invariant);
    assert_eq!(
        lower(&native, LoweringLimits::default()).unwrap_err().code,
        LoweringCode::Unsupported
    );
    let good = lower_for(&native, TARGET, LoweringLimits::default()).unwrap();
    let usage = good.usage();
    let exact = LoweringLimits {
        nodes: usage.nodes,
        depth: usage.max_depth,
        bytes: usage.bytes,
    };
    assert_eq!(
        lower_for(&native, TARGET, exact).unwrap().bytes(),
        good.bytes()
    );
    for limits in [
        LoweringLimits { nodes: 0, ..exact },
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
    for expression in [
        "self.n < 7",
        "self = other",
        "let x = amount in x < 7",
        "size(self.items) > self.count",
    ] {
        let native = package(&models, expression, ClauseKind::Invariant);
        assert_eq!(
            lower_for(&native, TARGET, LoweringLimits::default())
                .unwrap_err()
                .code,
            LoweringCode::Unsupported,
            "{expression}"
        );
    }
    let checked = setup::request_source(&models, "amount < 7", ClauseKind::Invariant, |text| {
        text.replace(
            "invariant Other on M::Node at current { true }",
            "invariant Other on M::Node at current { self.n < 7 }",
        )
    })
    .unwrap();
    let later = NativePackage::new(checked, PackageLimits::default()).unwrap();
    let error = lower_for(&later, TARGET, LoweringLimits::default()).unwrap_err();
    assert_eq!(error.code, LoweringCode::Unsupported);
    assert_eq!(error.clause.unwrap().clause().as_str(), "other_rule");
    assert_eq!(
        lower_for(&native, TARGET, LoweringLimits::default())
            .unwrap()
            .bytes(),
        good.bytes()
    );
}

fn source(source: &quire_spec_language::formal_source::FormalSource, file: &str) -> Value {
    json!({"file":file,"identity":source.source().identity().identity,
        "revision":source.source().identity().revision,"digest":source.source().digest().to_string(),
        "document":source.identity().document().as_str(),"formal_revision":source.identity().revision().get()})
}

fn write_job(directory: &Path, native: &NativePackage<'_>, model: &NativeModel) {
    let program = &native.checked().bindings().source;
    std::fs::write(directory.join("program.native"), program.source().text()).unwrap();
    std::fs::write(directory.join("model.json"), model.source().source().text()).unwrap();
    let clauses: Vec<_> = native.checked().bindings().clauses.iter().map(|binding| json!({
        "name":binding.name,"owner":{"package":binding.requirement.package().as_str(),
            "requirement":binding.requirement.requirement().as_str(),"revision":binding.requirement.revision().get()},
        "clause":binding.clause.as_str(),"point":binding.execution_point
    })).collect();
    let request = json!({"format":"native-compile/1","request":{
        "models":[{"format":"native-rule-model/1","source":source(model.source(),"model.json")}],
        "program":{"source":source(program,"program.native"),"clauses":clauses}
    }});
    std::fs::write(
        directory.join("compile.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
}

#[test]
#[trace("TC-111", "FR-033-AC-4")]
fn actual_command_exports_selected_bytes_and_current_backend_refuses_numeric_inputs() {
    let directory = tempfile::tempdir().unwrap();
    let models = [model(false, 1000)];
    let native = package(&models, "amount < 7", ClauseKind::Invariant);
    write_job(directory.path(), &native, &models[0]);
    let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("lower")
        .arg(directory.path().join("compile.json"))
        .args(["--target", "integer-ir/v1"])
        .current_dir(directory.path().parent().unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let projection = lower_for(&native, TARGET, LoweringLimits::default()).unwrap();
    assert_eq!(output.stdout, projection.bytes());
    let consumer = backend_ir::BoundPackage::from_json_bytes(&output.stdout).unwrap();
    let error = quire_contract_codegen::generate_bound_oracles(
        &consumer,
        quire_contract_codegen::AttestationContext {
            // Synthetic generator context only; no attestation claim.
            record_digest: &"0".repeat(64),
            candidate_revision: &"0".repeat(40),
        },
    )
    .unwrap_err();
    let quire_contract_codegen::BoundGenerationError::Clause {
        identity,
        diagnostics,
    } = error
    else {
        panic!("expected numeric clause refusal");
    };
    assert_eq!(identity.clause().as_str(), "population_rule");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
        == quire_contract_codegen::GenerationErrorCode::UnsupportedExpression));
    let invalid = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args(["lower", "/missing/input.json", "--target", "future/v9"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("unknown lowering target"));
    let unsupported = package(&models, "self.n < 7", ClauseKind::Invariant);
    write_job(directory.path(), &unsupported, &models[0]);
    let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("lower")
        .arg(directory.path().join("compile.json"))
        .args(["--target", "integer-ir/v1"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["code"], "unsupported_projection");
    assert_eq!(error["details"]["profile"], "integer-ir/v1");
}
