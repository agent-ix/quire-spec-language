// SPDX-License-Identifier: AGPL-3.0-or-later
//! The exact checker and EN-5 item refuse a claim whose `over` parameter
//! has no instance, rather than supplying a vacuous probability proof.

use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, DispositionKind, Engines, Settlement,
};
use qsl_eval::simulation::model::ModelSystem;
use qsl_foundation::RequestIndex;
use qsl_replay::{
    check_claim_exact_reachability, claim_binding, ExactProbabilityCertificate,
    ExactProbabilityGraphError, ModelCheckLimits, ObligationIdentity,
};
use quire_exact::{Cancel, ValueType};

use crate::prob_world::{self, World};

const CLAIM: &str =
    "probabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin \
     { probability >= 0.95 [ eventually holds(m.delivered) ] }\n";

#[trace("TC-636", "FR-201")]
#[test]
fn the_checker_reports_an_empty_over_population_as_unsupported_binding() {
    let world = World::with_objects(&prob_world::link(CLAIM), &[]);
    let system = ModelSystem::new(world.subject().simulation)
        .expect("the empty bounded population must admit before checking the binding");
    let claim = world.claim("Likely");
    let over = claim
        .over
        .as_ref()
        .expect("the claim has an over parameter");
    assert!(matches!(&over.value_type, ValueType::Reference(_)));
    assert!(system
        .instances(&over.value_type)
        .expect("the bounded population can enumerate its instances")
        .is_empty());
    assert_eq!(
        claim_binding(&system, claim).expect_err("zero instances cannot bind the claim"),
        ExactProbabilityGraphError::UnsupportedBinding,
    );

    let occurrence = claim
        .occurrence()
        .expect("the checked claim has a package identity");
    let item = ObligationIdentity::new(occurrence.node().clone(), None, 0);
    let limits = ModelCheckLimits::default();
    assert_eq!(
        check_claim_exact_reachability(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &item,
            &ExactProbabilityCertificate { values: Vec::new() },
            limits.limits.max_states,
            limits.limits.max_transitions,
            &Cancel::new(),
        ),
        Err(ExactProbabilityGraphError::UnsupportedBinding),
        "the public checker must refuse the empty binding before inspecting certificate values",
    );
}

#[trace("TC-762", "FR-281", "FR-203")]
#[test]
fn an_en5_item_over_no_instances_is_unsupported_without_proof_or_counterexample() {
    let world = World::with_objects(&prob_world::link(CLAIM), &[]);
    let engines = Engines::qsl().expect("the linked engines are admitted");
    let item = exact_probabilistic_item(
        RequestIndex::new(0),
        world.request("Likely", ModelCheckLimits::default()),
    );
    let records = analyze_items(&engines, vec![item], &Cancel::new());
    let [record] = records.as_slice() else {
        panic!("the empty binding must still settle exactly one item: {records:?}");
    };
    let Settlement::Disposed(disposed) = &record.settlement else {
        panic!("the empty binding must be unsupported, not a terminal verdict: {record:?}");
    };
    assert_eq!(disposed.kind, DispositionKind::Unsupported);
    assert_eq!(
        disposed.backend.as_ref().map(|id| id.as_str()),
        Some("en-5")
    );
    assert!(record.terminal().is_none());
    assert!(record.exact_probabilistic.is_none());
    assert!(record.counterexample().is_none());
}
