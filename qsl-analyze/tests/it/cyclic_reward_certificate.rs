// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite first-target reward proofs with cyclic continuations.

use super::long_run_certificate::{limits, produce};
use crate::prob_world::{self, Model, Operation, World};
use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, DispositionKind, Engines, Settlement,
};
use qsl_eval::simulation::{model::ModelSystem, TransitionSystem};
use qsl_foundation::RequestIndex;
use qsl_replay::{
    check_probability_certificate_document, probability_certificate_document,
    read_probability_certificate, write_probability_certificate, ExactProbabilityCertificate,
    ExactProbabilityCertificateCheck, ExactProbabilityValue, ModelCheckLimits,
    ProbabilityCertificateDocument, ProbabilityCertificateLocus, ProbabilityCertificateRejection,
    ProbabilityCertificateRule, ProbabilityDocumentError, ProbabilityDocumentLimits,
    ProbabilityProductKey,
};
use quire_exact::{Cancel, CancelCause, Integer, Rational};

fn ratio(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}
fn model(cost: &str, initial: i64, target: &str, bound: &str, exit: bool) -> Model {
    let package = "test/cyclic-reward";
    Model {
        package, object: "Item", population: "items", key: "x",
        integers: vec![("Phase", 0, 2)],
        fields: vec![("phase", format!("ix://{package}/Phase"))],
        operations: vec![
            Operation {name:"step", parameters:vec![("done", prob_world::BOOLEAN.to_owned())], modifies:vec!["phase"], deletes:false},
            Operation {name:"after", parameters:vec![], modifies:vec!["phase"], deletes:false},
        ],
        body: format!("pre Live using v on M::Item::step {{ self.phase = 0 }}\npost Stepped using v on M::Item::step {{ (done and self.phase = {}) or (not done and self.phase = 0) }}\npre After using v on M::Item::after {{ self.phase > 0 }}\npost Aftered using v on M::Item::after {{ (pre(self.phase) = 1 and self.phase = 2) or (pre(self.phase) = 2 and self.phase = 1) }}\nrandom Item::step(done) ~ {{ true: 1, false: 1 }};\nreward cost on Item::step = {cost};\nreward cost on Item::after = 100;\nworkload Even on M {{ weight Item::step = 1; weight Item::after = 1; }}\nprobabilistic Cost using inf on M over (x: M::Item) under Even on origin {{ expected accumulate cost until holds({target}) {bound} }}\n",i32::from(exit)),
        initial: serde_json::json!({"phase":{"integer":initial.to_string()}}),
    }
}
fn check(
    world: &World,
    bytes: &[u8],
    control: &Cancel,
    limits: ProbabilityDocumentLimits,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = qsl_replay::probabilistic_claim_identity(world.claim("Cost")).unwrap();
    check_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        bytes,
        limits,
        control,
    )
}
fn doc(world: &World) -> (Vec<u8>, ProbabilityCertificateDocument) {
    let bytes = produce(world, "Cost");
    let value = read_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
    assert!(value.cyclic_dtmc && value.reward.is_some());
    (bytes, value)
}
fn accepted(world: &World, document: &ProbabilityCertificateDocument) -> bool {
    let bytes = write_probability_certificate(document, limits(), &Cancel::new()).unwrap();
    matches!(
        check(world, &bytes, &Cancel::new(), limits()).unwrap(),
        ExactProbabilityCertificateCheck::Accepted(_)
    )
}
#[trace("TC-1403", "FR-201-AC-35", "FR-201-AC-37", "FR-198-AC-3")]
#[test]
fn geometric_rewards_include_entry_once_and_ignore_post_target_cycles() {
    for (cost, value) in [("1", 2), ("if done then 3 else 1", 4)] {
        for op in [">=", "<="] {
            let world = World::new(&model(
                cost,
                0,
                "x.phase = 1",
                &format!("{op} {value}"),
                true,
            ));
            let (bytes, document) = doc(&world);
            assert_eq!(
                document.values.len(),
                2,
                "post-target state is not evidence"
            );
            assert_eq!(document.targets.len(), 1);
            assert_eq!(document.values[0].value, ratio(value, 1));
            assert_eq!(document.values[1].value, ratio(0, 1));
            assert_eq!(
                write_probability_certificate(&document, limits(), &Cancel::new()).unwrap(),
                bytes
            );
            assert!(accepted(&world, &document));
            let mut bad = document.clone();
            bad.values[0].value = ratio(value + 1, 1);
            assert!(!accepted(&world, &bad));
            let mut loose = document.clone();
            loose.ranking[0].1 = 17;
            assert!(
                accepted(&world, &loose),
                "properness is not exact BFS distance"
            );
            let mut bad = document.clone();
            bad.ranking[0].1 = 0;
            let bytes = write_probability_certificate(&bad, limits(), &Cancel::new()).unwrap();
            assert!(matches!(
                check(&world, &bytes, &Cancel::new(), limits()).unwrap(),
                ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::RankBroken,
                    ..
                })
            ));
        }
    }
}
#[trace("TC-1403", "FR-201-AC-36", "FR-201-AC-38")]
#[test]
fn zero_reward_still_requires_properness_and_initial_target_is_zero() {
    let world = World::new(&model("0", 0, "x.phase = 1", "<= 0", true));
    let (_, document) = doc(&world);
    assert!(document.values.iter().all(|r| r.value == ratio(0, 1)));
    let mut bad = document.clone();
    bad.ranking[0].1 = 0;
    assert!(!accepted(&world, &bad));
    let initial = World::new(&model("1", 1, "x.phase = 1", "<= 0", true));
    let (_, document) = doc(&initial);
    assert_eq!(document.values.len(), 1);
    assert_eq!(document.values[0].value, ratio(0, 1));
    assert_eq!(document.ranking[0].1, 0);
    let closed = World::new(&model("0", 0, "x.phase = 1", "<= 0", false));
    let system = ModelSystem::new(closed.subject().simulation).unwrap();
    let binding = qsl_replay::claim_binding(&system, closed.claim("Cost"))
        .unwrap()
        .unwrap();
    let raw = system.state_key(&system.initial()[0]).unwrap().0;
    let key = ProbabilityProductKey::for_model_state(Some(&binding.reference), &raw).unwrap();
    let item = qsl_replay::probabilistic_claim_identity(closed.claim("Cost")).unwrap();
    let certificate = ExactProbabilityCertificate {
        values: vec![ExactProbabilityValue {
            state: key.clone(),
            value: ratio(0, 1),
        }],
    };
    let document = probability_certificate_document(
        &system,
        closed.package.graph().probabilistic_model().claims(),
        &item,
        &certificate,
        limits(),
        &Cancel::new(),
    )
    .unwrap();
    assert!(document.cyclic_dtmc);
    assert_eq!(document.ranking, vec![(key.clone(), 0)]);
    let bytes = write_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
    assert_eq!(
        check(&closed, &bytes, &Cancel::new(), limits()).unwrap(),
        ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
            rule: ProbabilityCertificateRule::RankBroken,
            at: ProbabilityCertificateLocus::ProductState(key)
        })
    );
    let records = analyze_items(
        &Engines::qsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            closed.request("Cost", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    assert!(
        matches!(&records[0].settlement,Settlement::Disposed(d) if d.kind==DispositionKind::Unsupported)
    );
    assert!(records[0].terminal().is_none());
}
#[trace("TC-1403", "FR-201-AC-36")]
#[test]
fn predicate_undefined_only_after_target_is_not_evaluated() {
    let world = World::new(&model(
        "1",
        0,
        "if x.phase = 2 then (1 mod 0 = 0) else (x.phase = 1)",
        "<= 2",
        true,
    ));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = qsl_replay::probabilistic_claim_identity(world.claim("Cost")).unwrap();
    assert_eq!(
        qsl_replay::admit_expected_reward_certificate_subject(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &item,
            limits(),
            &Cancel::new()
        ),
        Ok(())
    );
    let (_, document) = doc(&world);
    assert_eq!(document.values.len(), 2);
    assert!(accepted(&world, &document));
    let relevant = World::new(&model(
        "1",
        0,
        "if x.phase = 0 then (1 mod 0 = 0) else (x.phase = 1)",
        "<= 2",
        true,
    ));
    let bytes = write_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
    let rejection = check(&relevant, &bytes, &Cancel::new(), limits());
    assert!(matches!(
        &rejection,
        Err(ProbabilityDocumentError::UndefinedPredicate(_))
    ));
    let relevant_system = ModelSystem::new(relevant.subject().simulation).unwrap();
    assert_eq!(
        qsl_replay::admit_expected_reward_certificate_subject(
            &relevant_system,
            relevant.package.graph().probabilistic_model().claims(),
            &item,
            limits(),
            &Cancel::new()
        ),
        rejection.map(|_| ())
    );
}
#[trace("TC-1403", "FR-201-AC-35", "FR-201-AC-37")]
#[test]
fn cyclic_reward_units_normalize_to_selected_comparison_unit() {
    let mut source = model(
        "if done then 6 ms else 2 ms",
        0,
        "x.phase = 1",
        "<= 0.008 s",
        true,
    );
    source.body=format!("dimension Time;\nunit ms : Time = rational(1,1);\nunit s : Time = rational(1000,1) * ms;\n{}",source.body);
    source.body = source.body.replace(
        "reward cost on Item::after = 100;",
        "reward cost on Item::after = 100 ms;",
    );
    let world = World::new(&source);
    let (_, document) = doc(&world);
    assert_eq!(document.values[0].value, ratio(8, 1));
    assert!(document.reward.as_ref().unwrap().unit.is_some());
    assert!(accepted(&world, &document));
}
#[trace("TC-1403", "FR-201-AC-38")]
#[test]
fn actual_cyclic_reward_budget_and_cancellation_causes_remain_typed() {
    let world = World::new(&model("1", 0, "x.phase = 1", "<= 2", true));
    let (bytes, _) = doc(&world);
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let control = Cancel::new();
        control.cancel(cause);
        assert_eq!(
            check(&world, &bytes, &control, limits()),
            Err(ProbabilityDocumentError::Cancelled(cause))
        );
    }
    let mut narrow = limits();
    narrow.accounting.work_units = 0;
    assert!(matches!(
        check(&world, &bytes, &Cancel::new(), narrow),
        Err(ProbabilityDocumentError::Accounting(_))
    ));
}

#[trace("TC-1403", "FR-201-AC-38")]
#[test]
fn phase_observed_stops_use_original_token_in_every_cyclic_reward_pass() {
    use qsl_replay::{
        check_cyclic_reward_probability_certificate_observing, RewardProbabilityPhase as P,
    };
    let world = World::new(&model(
        "if done then 3 else 1",
        0,
        "x.phase = 1",
        "<= 4",
        true,
    ));
    let (bytes, _) = doc(&world);
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = qsl_replay::probabilistic_claim_identity(world.claim("Cost")).unwrap();
    for phase in [
        P::Unit,
        P::Original,
        P::Key,
        P::Stopped,
        P::Predicate,
        P::Draw,
        P::Reward,
        P::Ranks,
        P::Progress,
        P::Arithmetic,
    ] {
        let mut reached = false;
        let baseline = Cancel::new();
        assert!(matches!(
            check_cyclic_reward_probability_certificate_observing(
                &system,
                world.package.graph().probabilistic_model().claims(),
                &item,
                &bytes,
                limits(),
                &baseline,
                &mut |actual| {
                    if actual == phase {
                        reached = true;
                    }
                }
            )
            .unwrap(),
            ExactProbabilityCertificateCheck::Accepted(_)
        ));
        assert!(reached, "baseline must reach {phase:?}");
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            let cancel = Cancel::new();
            let mut reached = false;
            let result = check_cyclic_reward_probability_certificate_observing(
                &system,
                world.package.graph().probabilistic_model().claims(),
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
            assert!(reached, "{phase:?} must be actual before {cause:?}");
            assert_eq!(
                result,
                Err(ProbabilityDocumentError::Cancelled(cause)),
                "{phase:?}"
            );
        }
    }
}

#[trace("TC-1403", "FR-201-AC-38")]
#[test]
fn fallback_retains_the_same_cumulative_storage_quota() {
    let world = World::new(&model("1", 0, "x.phase = 1", "<= 2", true));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = qsl_replay::probabilistic_claim_identity(world.claim("Cost")).unwrap();
    let claims = world.package.graph().probabilistic_model().claims();
    let direct = |bytes| {
        let mut cap = limits();
        cap.storage_bytes = bytes;
        qsl_replay::admit_cyclic_eventual_certificate_subject_with_identity(
            &system,
            claims,
            &item,
            cap,
            &Cancel::new(),
            qsl_foundation::IdentityLimits::default(),
        )
    };
    let (mut low, mut high) = (0, limits().storage_bytes);
    assert!(direct(high).is_ok());
    while low < high {
        let mid = low + (high - low) / 2;
        if direct(mid).is_ok() {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    assert!(direct(low).is_ok());
    let mut cap = limits();
    cap.storage_bytes = low;
    assert!(
        matches!(
            qsl_replay::admit_expected_reward_certificate_subject(
                &system,
                claims,
                &item,
                cap,
                &Cancel::new(),
            ),
            Err(ProbabilityDocumentError::Limit {
                limit: qsl_replay::ProbabilityDocumentLimit::StorageBytes,
                ..
            })
        ),
        "failed legacy attempt must consume the same quota as the stopped fallback"
    );
    assert!(qsl_replay::admit_expected_reward_certificate_subject(
        &system,
        claims,
        &item,
        limits(),
        &Cancel::new(),
    )
    .is_ok());
}
