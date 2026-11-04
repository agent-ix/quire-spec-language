// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's bounded checker adapter starts from an admitted checked model and
//! reaches it only through the claim binder.

use base64::Engine;
use ix_trace_rs::trace;
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::TransitionSystem;
use qsl_replay::{
    check_claim_exact_reachability, ExactProbabilityCertificate, ExactProbabilityCertificateCheck,
    ExactProbabilityGraphError, ExactProbabilityValue, ObligationIdentity, ProbabilityBoundSide,
    ProbabilityCertificateLocus, ProbabilityCertificateRejection, ProbabilityCertificateRule,
    ProbabilityProductKey,
};
use qsl_semantics::check::{Bound, CheckedProbabilisticClaim, ProbForm, ReachTarget};
use quire_exact::{Integer, NodeKey, Rational};
use serde_json::json;

use crate::vault_world::{with_body, Build, World, SECURE};

/// Reaches `holds(true)` at the initial state, so its graph is one state.
const LIKELY: &str = "probabilistic Same using inf on V under every scheduler on origin \
                      { probability >= 0.5 [ eventually holds(true) ] }\n";
/// The same name, never reaching its target from the initial state.
const UNLIKELY: &str = "probabilistic Same using inf on V under every scheduler on origin \
                        { probability >= 0.25 [ eventually holds(false) ] }\n";
/// A false claim: the target is reached with probability 1, not at most 1/2.
const FALSE_AT_MOST: &str = "probabilistic AtMost using inf on V under every scheduler on origin \
                             { probability <= 0.5 [ eventually holds(true) ] }\n";
/// A reach form this slice does not check.
const ALWAYS: &str = "probabilistic Always using inf on V under every scheduler on origin \
                      { probability >= 0.5 [ always holds(true) ] }\n";
/// A claim of another name, whose node no `Same` claim carries.
const OTHER: &str = "probabilistic Other using inf on V under every scheduler on origin \
                     { probability >= 0.5 [ eventually holds(true) ] }\n";

fn world_with(extra: &str) -> World {
    let build = Build { extra, ..SECURE };
    World::build(
        &["v"],
        false,
        &with_body("", "", "always holds(true)"),
        &build,
    )
}

fn claims(world: &World) -> &[CheckedProbabilisticClaim] {
    world.package().graph().probabilistic_model().claims()
}

fn by_form(world: &World, wanted: impl Fn(&ProbForm) -> bool) -> &CheckedProbabilisticClaim {
    claims(world)
        .iter()
        .find(|claim| wanted(&claim.form))
        .expect("the claim with that form")
}

fn threshold(
    world: &World,
    target: fn(&Bound) -> Option<&Rational>,
    numerator: i64,
    denominator: i64,
) -> &CheckedProbabilisticClaim {
    let wanted = Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap();
    by_form(world, |form| match form {
        ProbForm::Reach {
            bound,
            target: ReachTarget::Eventually(_),
        } => target(bound) == Some(&wanted),
        _ => false,
    })
}

fn at_least(bound: &Bound) -> Option<&Rational> {
    match bound {
        Bound::AtLeast(value) => Some(value),
        Bound::AtMost(_) => None,
    }
}

fn at_most(bound: &Bound) -> Option<&Rational> {
    match bound {
        Bound::AtMost(value) => Some(value),
        Bound::AtLeast(_) => None,
    }
}

fn node_of(claim: &CheckedProbabilisticClaim) -> NodeKey {
    claim
        .occurrence()
        .expect("a checked claim has a package identity")
        .node()
        .clone()
}

/// The model system, and its single initial state's FR-196 v1 product key.
fn system_and_root(world: &World) -> (ModelSystem<'_>, ProbabilityProductKey) {
    let mut subject = world.subject();
    subject.simulation.initial.truncate(1);
    let system = ModelSystem::new(subject.simulation.clone()).expect("model admits");
    let initial = system.initial().into_iter().next().expect("one initial");
    let model_key = system.state_key(&initial).expect("model key").0;
    let bytes = serde_json::to_vec(&json!([
        "quire.probability-product-key/v1",
        null,
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(model_key),
        null,
        null,
        null
    ]))
    .unwrap();
    (system, ProbabilityProductKey::from_canonical_bytes(bytes))
}

fn one() -> Rational {
    Rational::from_integer(Integer::from(1_i64))
}

fn rooted(root: &ProbabilityProductKey, value: Rational) -> ExactProbabilityCertificate {
    ExactProbabilityCertificate {
        values: vec![ExactProbabilityValue {
            state: root.clone(),
            value,
        }],
    }
}

fn check(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    node: &NodeKey,
    certificate: &ExactProbabilityCertificate,
) -> Result<ExactProbabilityCertificateCheck, ExactProbabilityGraphError> {
    check_claim_exact_reachability(
        system,
        claims,
        &ObligationIdentity::new(node.clone(), None, 0),
        certificate,
        128,
        1024,
    )
}

fn rejected(
    result: Result<ExactProbabilityCertificateCheck, ExactProbabilityGraphError>,
) -> ProbabilityCertificateRejection {
    match result {
        Ok(ExactProbabilityCertificateCheck::Rejected(rejection)) => rejection,
        other => panic!("expected a certificate rejection: {other:?}"),
    }
}

#[trace("TC-636")]
#[test]
fn bound_claim_reconstructs_its_graph_and_reports_the_omitted_state() {
    let world = world_with(UNLIKELY);
    let claim = threshold(&world, at_least, 1, 4);
    let (system, root) = system_and_root(&world);
    let empty = ExactProbabilityCertificate { values: Vec::new() };

    let rejection = rejected(check(&system, claims(&world), &node_of(claim), &empty));
    assert_eq!(rejection.rule, ProbabilityCertificateRule::Malformed);
    assert_eq!(
        rejection.at,
        ProbabilityCertificateLocus::ProductState(root)
    );
}

#[trace("TC-636")]
#[test]
fn claim_binding_uses_the_checked_node_not_name_or_source_order() {
    let forward = world_with(&format!("{LIKELY}{UNLIKELY}"));
    let reversed = world_with(&format!("{UNLIKELY}{LIKELY}"));

    // The same-named claims keep their NodeKeys when their order is swapped.
    let likely = node_of(threshold(&forward, at_least, 1, 2));
    let unlikely = node_of(threshold(&forward, at_least, 1, 4));
    assert_ne!(likely, unlikely);
    assert_eq!(likely, node_of(threshold(&reversed, at_least, 1, 2)));
    assert_eq!(unlikely, node_of(threshold(&reversed, at_least, 1, 4)));

    for world in [&forward, &reversed] {
        let (system, root) = system_and_root(world);
        let certificate = rooted(&root, one());

        assert_eq!(
            check(&system, claims(world), &likely, &certificate),
            Ok(ExactProbabilityCertificateCheck::Accepted {
                value: one(),
                side: ProbabilityBoundSide::AtLeast,
            })
        );
        // The other claim's target is never reached, so the same certificate
        // is rejected for it: its graph has states the certificate omits.
        let rejection = rejected(check(&system, claims(world), &unlikely, &certificate));
        assert_eq!(rejection.rule, ProbabilityCertificateRule::Malformed);
        let ProbabilityCertificateLocus::ProductState(missing) = rejection.at else {
            panic!("a missing value is reported at its product state");
        };
        assert_ne!(missing, root, "the root's value is present");
    }
}

/// The claim fixes the scheduler objective: a false `under every scheduler`
/// `<=` claim is rejected, and the `>=` claim over the same graph is not.
#[trace("TC-636")]
#[test]
fn claim_fixes_the_objective_and_a_false_claim_is_rejected() {
    let world = world_with(&format!("{LIKELY}{FALSE_AT_MOST}"));
    let (system, root) = system_and_root(&world);
    let certificate = rooted(&root, one());
    let false_claim = node_of(threshold(&world, at_most, 1, 2));

    let rejection = rejected(check(&system, claims(&world), &false_claim, &certificate));
    assert_eq!(rejection.rule, ProbabilityCertificateRule::ThresholdMissed);
    assert_eq!(
        rejection.at,
        ProbabilityCertificateLocus::ProductState(root)
    );
    assert_eq!(
        check(
            &system,
            claims(&world),
            &node_of(threshold(&world, at_least, 1, 2)),
            &certificate
        ),
        Ok(ExactProbabilityCertificateCheck::Accepted {
            value: one(),
            side: ProbabilityBoundSide::AtLeast,
        })
    );
}

#[trace("TC-636")]
#[test]
fn binder_refuses_unbound_nodes_and_unsupported_forms() {
    let world = world_with(&format!("{LIKELY}{ALWAYS}"));
    let other = world_with(OTHER);
    let (system, root) = system_and_root(&world);
    let certificate = rooted(&root, one());

    // A node no claim of the full list carries binds nothing.
    let foreign = node_of(&claims(&other)[0]);
    assert!(claims(&world).iter().all(|claim| node_of(claim) != foreign));
    assert_eq!(
        check(&system, claims(&world), &foreign, &certificate),
        Err(ExactProbabilityGraphError::ClaimNotBound)
    );

    // A bound claim of a form this slice does not check is refused.
    let always = node_of(by_form(&world, |form| {
        matches!(
            form,
            ProbForm::Reach {
                target: ReachTarget::Always(_),
                ..
            }
        )
    }));
    assert_eq!(
        check(&system, claims(&world), &always, &certificate),
        Err(ExactProbabilityGraphError::UnsupportedClaimForm)
    );
}
