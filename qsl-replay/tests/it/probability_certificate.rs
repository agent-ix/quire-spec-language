// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's bounded checker adapter starts from an admitted checked model.

use base64::Engine;
use ix_trace_rs::trace;
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::TransitionSystem;
use qsl_replay::{
    check_model_exact_reachability, ExactProbabilityCertificate, ExactProbabilityCertificateCheck,
    ProbabilityCertificateLocus, ProbabilityCertificateRule, ProbabilityObjective,
};
use qsl_semantics::check::{ProbForm, ReachTarget};
use serde_json::json;

use crate::vault_world::{with_body, Build, World, SECURE};

const CLAIM: &str = "probabilistic Reach using inf on V under every scheduler on origin \
                     { probability >= 0 [ eventually holds(false) ] }";

#[trace("TC-636", "FR-201-AC-1")]
#[test]
fn exact_checker_reconstructs_bounded_graph_from_checked_model_system() {
    let build = Build {
        extra: CLAIM,
        ..SECURE
    };
    let world = World::build(
        &["v"],
        false,
        &with_body("", "", "always holds(v.l = v.l)"),
        &build,
    );
    let mut subject = world.subject();
    subject.simulation.initial.truncate(1);
    let system = ModelSystem::new(subject.simulation.clone()).expect("model admits");
    let claim = world
        .package()
        .graph()
        .probabilistic_model()
        .claim("Reach")
        .expect("checked reach claim");
    let ProbForm::Reach {
        bound,
        target: ReachTarget::Eventually(predicate),
    } = &claim.form
    else {
        panic!("unbounded reach form: {:?}", claim.form);
    };
    let initial = system
        .initial()
        .into_iter()
        .next()
        .expect("one selected initial state");
    let model_key = system.state_key(&initial).expect("canonical model key").0;

    let result = check_model_exact_reachability(
        &system,
        predicate,
        &ExactProbabilityCertificate { values: Vec::new() },
        ProbabilityObjective::Minimum,
        bound,
        128,
        1024,
    )
    .expect("the finite graph is reconstructed within limits");
    let ExactProbabilityCertificateCheck::Rejected(rejection) = result else {
        panic!("the empty certificate is rejected: {result:?}");
    };
    assert_eq!(rejection.rule, ProbabilityCertificateRule::Malformed);
    let ProbabilityCertificateLocus::ProductState(key) = rejection.at else {
        panic!("missing values are reported at their product state");
    };
    let encoded: serde_json::Value =
        serde_json::from_slice(key.as_bytes()).expect("canonical product key JSON");
    assert_eq!(
        encoded,
        json!([
            "quire.probability-product-key/v1",
            null,
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(model_key),
            null,
            null,
            null
        ])
    );
}
