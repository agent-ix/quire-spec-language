// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-009: real frontend-to-binder correspondence and refusal controls.

// This shared fixture module also serves runtime execution tests in other binaries.
#[allow(dead_code)]
#[path = "support/runtime_setup.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::lowering::{lower, LoweringCode, LoweringLimits};
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::package::{NativePackage, PackageLimits};
use quire_spec_language::syntax::ClauseKind;
use serde_json::json;

fn boolean_model(input: bool) -> NativeModel {
    setup::authored_model(|model| {
        model["values"].as_array_mut().unwrap().push(json!({
            "name":"enabled", "kind":if input { "input" } else { "state" },
            "type":{"kind":"boolean"}
        }));
        if input {
            model["operations"][0]["parameters"] = json!(["enabled"]);
        }
    })
}

fn package<'a>(models: &'a [NativeModel], expression: &str, kind: ClauseKind) -> NativePackage<'a> {
    NativePackage::new(
        setup::checked_kind(models, expression, kind),
        PackageLimits::default(),
    )
    .expect("valid native package before lowering")
}

#[test]
#[trace("TC-092", "FR-009-AC-1", "FR-009-AC-3")]
fn preserves_complete_population_sources_and_observations() {
    for (input, kind, native, projected) in [
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
        let models = [boolean_model(input)];
        let package = package(&models, "enabled implies (not enabled or true)", kind);
        let projection = lower(&package, LoweringLimits::default()).unwrap();
        assert!(std::ptr::eq(projection.native(), &package));
        assert_eq!(projection.profile(), "boolean-oracle/v1");
        assert_eq!(projection.bound().clauses().len(), 2);
        assert_eq!(
            projection
                .bound()
                .clauses()
                .iter()
                .map(|c| c.identity().clause().as_str())
                .collect::<Vec<_>>(),
            ["other_rule", "population_rule"]
        );
        let bound = &projection.bound().clauses()[1];
        let checked = package.checked();
        assert_eq!(bound.identity().requirement(), &setup::authored_owner());
        assert_eq!(
            bound.anchor(),
            &checked.clauses()[0].binding().execution_point
        );
        assert_eq!(
            checked.bindings().source.to_native(bound.source()).unwrap(),
            checked.linked().unit().clauses()[0].span
        );
        let ir::ExpressionKind::Boolean {
            operator,
            left,
            right,
        } = bound.expression().expression().kind()
        else {
            panic!("actual implication tree")
        };
        assert_eq!(*operator, ir::BooleanOperator::Implication);
        let source = checked.linked().unit().source().text();
        for (expression, expected) in [(left, "enabled"), (right, "not enabled or true")] {
            let span = checked
                .bindings()
                .source
                .to_native(expression.source())
                .unwrap();
            assert_eq!(&source[span.start..span.end], expected);
        }
        assert_eq!(
            projection.reads().len(),
            1,
            "repeated occurrences share one projected declaration"
        );
        let read = &projection.reads()[0];
        assert_eq!(read.clause, *bound.identity());
        assert_eq!(
            read.declaration.identity.owner,
            *models[0].environment().owner()
        );
        assert_eq!(read.model_digest, models[0].digest());
        assert_eq!(read.native_observation, native);
        assert_eq!(read.ir_observation, projected);
        assert_eq!(read.name.as_str(), "enabled");
        assert_eq!(
            bound.environment().values()[0].source(),
            &read.declaration.source
        );
        assert_eq!(bound.expression().dependencies().len(), 1);
        assert_eq!(
            ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap(),
            *projection.bound()
        );

        let mut wire: serde_json::Value = serde_json::from_slice(projection.bytes()).unwrap();
        wire["bindings"].as_array_mut().unwrap().pop();
        assert!(ir::BoundPackage::from_json_bytes(&serde_json::to_vec(&wire).unwrap()).is_err());
    }
}

#[test]
#[trace("TC-092", "FR-009-AC-4")]
fn bound_and_native_identities_cover_their_respective_declarations() {
    let state = [boolean_model(false)];
    let input = [boolean_model(true)];
    let state_package = package(&state, "enabled", ClauseKind::Precondition);
    let input_package = package(&input, "enabled", ClauseKind::Precondition);
    let state_projection = lower(&state_package, LoweringLimits::default()).unwrap();
    let input_projection = lower(&input_package, LoweringLimits::default()).unwrap();
    assert_ne!(
        state_projection.bound().digest(),
        input_projection.bound().digest()
    );
    assert_eq!(
        state_projection.bound().clauses()[1].environment().values()[0].kind(),
        ir::ValueDeclarationKind::State
    );
    assert_eq!(
        input_projection.bound().clauses()[1].environment().values()[0].kind(),
        ir::ValueDeclarationKind::Input
    );

    let changed = [setup::authored_model(|model| {
        model["scalars"][0]["maximum"] = json!(1001)
    })];
    let original = [setup::native_rule_model::parts().model()];
    let original = package(&original, "true", ClauseKind::Invariant);
    let changed = package(&changed, "true", ClauseKind::Invariant);
    assert_ne!(original.canonical_identity(), changed.canonical_identity());
    assert_eq!(
        lower(&original, LoweringLimits::default())
            .unwrap()
            .bound()
            .digest(),
        lower(&changed, LoweringLimits::default())
            .unwrap()
            .bound()
            .digest()
    );
}

#[test]
#[trace("TC-093", "FR-009-AC-2", "FR-009-AC-7")]
fn refuses_checked_features_instead_of_dropping_obligations() {
    let models = [setup::native_rule_model::parts().model()];
    for expression in [
        "deref(self.peer) = self",
        "self.n > 0",
        "let x = true in x",
        "if true then true else false",
        "forall(x in self.items: x >= 0)",
    ] {
        let package = package(&models, expression, ClauseKind::Invariant);
        let bytes = package.bytes().to_vec();
        let failure = lower(&package, LoweringLimits::default()).unwrap_err();
        assert_eq!(failure.code, LoweringCode::Unsupported, "{expression}");
        assert_eq!(failure.clause.unwrap().clause().as_str(), "population_rule");
        assert!(failure.source.is_some());
        assert_eq!(package.bytes(), bytes);
        assert_eq!(package.checked().clauses().len(), 2);
    }
    let checked = setup::request_source(&models, "true", ClauseKind::Invariant, |source| {
        source.replace(
            "Other on M::Node at current { true }",
            "Other on M::Node at current { self.n > 0 }",
        )
    })
    .unwrap();
    let package = NativePackage::new(checked, PackageLimits::default()).unwrap();
    let failure = lower(&package, LoweringLimits::default()).unwrap_err();
    assert_eq!(failure.code, LoweringCode::Unsupported);
    assert_eq!(failure.clause.unwrap().clause().as_str(), "other_rule");
}

#[test]
#[trace("TC-093", "FR-009-AC-6")]
fn exact_limits_and_fresh_retries() {
    let hard = LoweringLimits::default();
    assert_eq!(
        (hard.nodes, hard.depth, hard.bytes),
        (10_000, 64, 16_777_216)
    );
    let models = [setup::native_rule_model::parts().model()];
    let package = package(&models, "(true and not false)", ClauseKind::Invariant);
    let projection = lower(&package, LoweringLimits::default()).unwrap();
    // Five nodes in Rule (including its group), one in Other; deepest leaf at four.
    assert_eq!(projection.usage().nodes, 6);
    assert_eq!(projection.usage().max_depth, 4);
    let exact = LoweringLimits {
        nodes: 6,
        depth: 4,
        bytes: projection.bytes().len(),
    };
    assert_eq!(lower(&package, exact).unwrap().bytes(), projection.bytes());
    for limits in [
        LoweringLimits { nodes: 0, ..exact },
        LoweringLimits { nodes: 5, ..exact },
        LoweringLimits { depth: 0, ..exact },
        LoweringLimits { depth: 3, ..exact },
        LoweringLimits { bytes: 0, ..exact },
        LoweringLimits {
            bytes: exact.bytes - 1,
            ..exact
        },
    ] {
        assert_eq!(
            lower(&package, limits).unwrap_err().code,
            LoweringCode::ResourceExhausted
        );
        assert_eq!(lower(&package, exact).unwrap().bytes(), projection.bytes());
    }
    let above = LoweringLimits {
        nodes: usize::MAX,
        depth: usize::MAX,
        bytes: usize::MAX,
    };
    assert_eq!(lower(&package, above).unwrap().usage(), projection.usage());
}

#[test]
#[trace("TC-093", "FR-009-AC-7")]
fn refuses_owner_populations_that_one_ir_package_cannot_represent() {
    use quire_spec_language::checking::{check, CheckLimits};
    use quire_spec_language::{link_native, parse, Limits, LinkLimits};

    let models = [setup::native_rule_model::parts().model()];
    for owner in [
        ir::RequirementRef::parse("example/foreign", "PopulationRule", 7).unwrap(),
        ir::RequirementRef::parse("example/runtime-rules", "PopulationRule", 8).unwrap(),
    ] {
        let original = setup::checked(&models, "true");
        let mut bindings = original.bindings().clone();
        bindings.clauses[1].requirement = owner;
        let source = original.linked().unit().source();
        let unit = parse(
            source.identity().clone(),
            source.path(),
            source.text().as_bytes(),
            Limits::default(),
        )
        .unwrap();
        let checked = check(
            link_native(unit, &models, LinkLimits::default()).unwrap(),
            bindings,
            CheckLimits::default(),
        )
        .unwrap();
        let package = NativePackage::new(checked, PackageLimits::default()).unwrap();
        let failure = lower(&package, LoweringLimits::default()).unwrap_err();
        assert_eq!(failure.code, LoweringCode::Unsupported);
        assert_eq!(failure.clause.unwrap().clause().as_str(), "other_rule");
        assert_eq!(
            failure.source,
            Some(package.checked().linked().unit().clauses()[1].span)
        );
        assert_eq!(package.checked().clauses().len(), 2);
    }
}
