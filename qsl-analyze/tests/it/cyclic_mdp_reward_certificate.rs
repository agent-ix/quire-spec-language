// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual MDP reward subjects, infinity coverage and cumulative caller controls.
use super::long_run_certificate::{limits, produce};
use crate::prob_world::{self, Model, Operation, World};
use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, DispositionKind, Engines, Settlement,
};
use qsl_eval::simulation::model::ModelSystem;
use qsl_replay::{
    check_probability_certificate_document, probabilistic_claim_identity,
    read_probability_proof_document, ExactProbabilityCertificateCheck, ProbabilityDocumentError,
};
use quire_exact::{Cancel, CancelCause};
fn model(initial: i64, target: &str, bound: &str, trap: bool) -> Model {
    let package = "test/mdp-reward";
    let bpost = if trap {
        "self.phase = 2"
    } else {
        "self.phase - (if done then 1 else 0) = 0"
    };
    Model {
        package, object:"Item", population:"items", key:"x",
        integers:vec![("Phase",0,2)], fields:vec![("phase",format!("ix://{package}/Phase"))],
        operations:vec![Operation{name:"a",parameters:vec![("done",prob_world::BOOLEAN.into())],modifies:vec!["phase"],deletes:false},Operation{name:"b",parameters:vec![("done",prob_world::BOOLEAN.into())],modifies:vec!["phase"],deletes:false},Operation{name:"after",parameters:vec![],modifies:vec!["phase"],deletes:false}],
        body:format!("pre A using v on M::Item::a {{ self.phase = 0 }}\npost AD using v on M::Item::a {{ self.phase = 1 }}\npre B using v on M::Item::b {{ self.phase = 0 }}\npost BD using v on M::Item::b {{ {bpost} }}\npre C using v on M::Item::after {{ self.phase > 0 }}\npost CD using v on M::Item::after {{ self.phase - pre(self.phase) = 0 }}\nrandom Item::a(done) ~ {{ true: 1, false: 1 }};\nrandom Item::b(done) ~ {{ true: 1, false: 1 }};\nreward cost on Item::a = 1;\nreward cost on Item::b = 2;\nreward cost on Item::after = 100;\nprobabilistic Cost using inf on M over (x: M::Item) under every scheduler on origin {{ expected accumulate cost until holds({target}) {bound} }}\n"),
        initial:serde_json::json!({"phase":{"integer":initial.to_string()}}),
    }
}
#[trace("TC-1405", "FR-201-AC-44", "FR-201-AC-45")]
#[test]
fn native_mixed_infinity_is_unsupported_even_with_finite_initial_minimum() {
    let world = World::new(&model(0, "x.phase = 1", ">= 1", true));
    let records = analyze_items(
        &Engines::qsl().unwrap(),
        vec![exact_probabilistic_item(
            qsl_foundation::RequestIndex::new(0),
            world.request("Cost", qsl_replay::ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let Settlement::Disposed(disposal) = &records[0].settlement else {
        panic!("must not emit evidence: {records:?}");
    };
    assert_eq!(disposal.kind, DispositionKind::Unsupported);
    assert!(
        format!("{disposal:?}").contains("mixed-infinite-coverage"),
        "{disposal:?}"
    );
    assert!(records[0].terminal().is_none());
}
#[trace("TC-1405", "FR-201-AC-45", "FR-201-AC-46")]
#[test]
fn initial_target_stops_query_and_real_reward_phases_preserve_cancellation() {
    let world = World::new(&model(0, "x.phase = 1", "<= 4", false));
    let bytes = produce(&world, "Cost");
    let document = read_probability_proof_document(&bytes, limits(), &Cancel::new()).unwrap();
    assert!(document.is_cyclic_mdp_reward());
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = probabilistic_claim_identity(world.claim("Cost")).unwrap();
    let claims = world.package.graph().probabilistic_model().claims();
    assert!(matches!(
        check_probability_certificate_document(
            &system,
            claims,
            &item,
            &bytes,
            limits(),
            &Cancel::new()
        ),
        Ok(ExactProbabilityCertificateCheck::Accepted(_))
    ));
    use qsl_replay::RewardProbabilityPhase as P;
    for phase in [
        P::Original,
        P::Stopped,
        P::Key,
        P::Predicate,
        P::Draw,
        P::Reward,
        P::Unit,
        P::Arithmetic,
        P::Progress,
    ] {
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            let cancel = Cancel::new();
            let mut reached = false;
            let stopped = qsl_replay::check_cyclic_reward_probability_certificate_observing(
                &system,
                claims,
                &item,
                &bytes,
                limits(),
                &cancel,
                &mut |actual| {
                    if actual == phase {
                        reached = true;
                        cancel.cancel(cause);
                    }
                },
            );
            assert!(reached, "{phase:?}");
            assert_eq!(
                stopped,
                Err(ProbabilityDocumentError::Cancelled(cause)),
                "{phase:?}"
            );
        }
    }
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    assert_eq!(
        check_probability_certificate_document(&system, claims, &item, &bytes, limits(), &cancel),
        Err(ProbabilityDocumentError::Cancelled(CancelCause::Requested))
    );
    for which in 0..7 {
        let mut bound = limits();
        match which {
            0 => bound.storage_bytes = 1,
            1 => bound.bytes = 1,
            2 => bound.states = 1,
            3 => bound.edges = 1,
            4 => bound.accounting.work_units = 1,
            5 => bound.accounting.integer_bits = 1,
            _ => bound.scalar_bits = 1,
        }
        assert!(
            check_probability_certificate_document(
                &system,
                claims,
                &item,
                &bytes,
                bound,
                &Cancel::new()
            )
            .is_err(),
            "quota {which}"
        );
    }
    let initial = World::new(&model(1, "x.phase = 1", "<= 0", false));
    let bytes = produce(&initial, "Cost");
    let document = read_probability_proof_document(&bytes, limits(), &Cancel::new()).unwrap();
    assert_eq!(document.document().values.len(), 1);
    assert_eq!(
        document.document().values[0].value,
        quire_exact::Rational::from_integer(0.into())
    );
    assert_eq!(document.document().ranking[0].1, 0);
}
