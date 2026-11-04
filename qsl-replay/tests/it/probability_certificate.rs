// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's bounded checker adapter starts from an admitted checked model.

use base64::Engine;
use ix_trace_rs::trace;
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::TransitionSystem;
use qsl_replay::{
    check_claim_exact_reachability, check_model_exact_reachability, ExactProbabilityCertificate,
    ExactProbabilityCertificateCheck, ExactProbabilityGraphError, ExactProbabilityValue,
    ObligationIdentity, ProbabilityBoundSide, ProbabilityCertificateLocus,
    ProbabilityCertificateRule, ProbabilityObjective, ProbabilityProductKey,
};
use qsl_semantics::check::{Bound, CheckedProbabilisticClaim, ProbForm, ReachTarget};
use quire_exact::{Integer, NodeKey, Rational};
use serde_json::json;

use crate::vault_world::{with_body, Build, World, SECURE};

const CLAIM: &str = "probabilistic Reach using inf on V under every scheduler on origin \
                     { probability >= 0.5 [ eventually holds(false) ] }";

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
        &with_body("", "", "always holds(true)"),
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

const LIKELY: &str = "probabilistic Same using inf on V under every scheduler on origin \
                      { probability >= 0.5 [ eventually holds(true) ] }\n";
const UNLIKELY: &str = "probabilistic Same using inf on V under every scheduler on origin \
                        { probability >= 0.25 [ eventually holds(false) ] }\n";

fn world_with(extra: &str) -> World {
    let build = Build { extra, ..SECURE };
    World::build(
        &["v"],
        false,
        &with_body("", "", "always holds(true)"),
        &build,
    )
}

/// The claim declared with `eventually holds(true)` (threshold 1/2) and the
/// one declared with `eventually holds(false)` (threshold 1/4).
fn claim_by_threshold(
    world: &World,
    numerator: i64,
    denominator: i64,
) -> &CheckedProbabilisticClaim {
    let wanted = Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap();
    world
        .package()
        .graph()
        .probabilistic_model()
        .claims()
        .iter()
        .find(|claim| {
            matches!(&claim.form, ProbForm::Reach { bound: Bound::AtLeast(b), .. } if *b == wanted)
        })
        .expect("the claim with that threshold")
}

fn node_of(claim: &CheckedProbabilisticClaim) -> NodeKey {
    claim
        .occurrence()
        .expect("an assembled claim has a checked identity")
        .node()
        .clone()
}

#[trace("TC-1288", "FR-498-AC-2", "FR-201-AC-1")]
#[test]
fn claim_binding_uses_the_checked_node_not_name_or_source_order() {
    let forward = world_with(&format!("{LIKELY}{UNLIKELY}"));
    let reversed = world_with(&format!("{UNLIKELY}{LIKELY}"));

    // The same-named claims keep their NodeKeys when their order is swapped.
    let likely = node_of(claim_by_threshold(&forward, 1, 2));
    let unlikely = node_of(claim_by_threshold(&forward, 1, 4));
    assert_ne!(likely, unlikely);
    assert_eq!(likely, node_of(claim_by_threshold(&reversed, 1, 2)));
    assert_eq!(unlikely, node_of(claim_by_threshold(&reversed, 1, 4)));

    for world in [&forward, &reversed] {
        let mut subject = world.subject();
        subject.simulation.initial.truncate(1);
        let system = ModelSystem::new(subject.simulation.clone()).expect("model admits");
        let initial = system.initial().into_iter().next().expect("one initial");
        let model_key = system.state_key(&initial).expect("model key").0;
        // `holds(true)` ends the walk at the initial state with value 1.
        let one = Rational::from_integer(Integer::from(1_i64));
        let root = {
            let bytes = serde_json::to_vec(&json!([
                "quire.probability-product-key/v1",
                null,
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(model_key),
                null,
                null,
                null
            ]))
            .unwrap();
            ProbabilityProductKey::from_canonical_bytes(bytes)
        };
        let certificate = ExactProbabilityCertificate {
            values: vec![ExactProbabilityValue {
                state: root,
                value: one.clone(),
            }],
        };
        let claims = world.package().graph().probabilistic_model().claims();
        let check = |node: &NodeKey| {
            check_claim_exact_reachability(
                &system,
                claims,
                &ObligationIdentity::new(node.clone(), None, 0),
                &certificate,
                ProbabilityObjective::Minimum,
                128,
                1024,
            )
        };

        assert_eq!(
            check(&likely),
            Ok(ExactProbabilityCertificateCheck::Accepted {
                value: one,
                side: ProbabilityBoundSide::AtLeast,
            })
        );
        // The other claim's target is never reached, so the same certificate
        // is not accepted for it.
        assert!(!matches!(
            check(&unlikely),
            Ok(ExactProbabilityCertificateCheck::Accepted { .. })
        ));
        // With no checked claim carrying the node, nothing binds.
        let foreign = ObligationIdentity::new(unlikely.clone(), None, 0);
        assert_eq!(
            check_claim_exact_reachability(
                &system,
                &claims[..0],
                &foreign,
                &certificate,
                ProbabilityObjective::Minimum,
                128,
                1024,
            ),
            Err(ExactProbabilityGraphError::ClaimNotBound)
        );
    }
}
