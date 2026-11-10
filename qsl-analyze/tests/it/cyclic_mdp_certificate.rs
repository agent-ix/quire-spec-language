// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual checked MDP subjects, caller controls and first-hit scope.
use super::long_run_certificate::limits;
use crate::prob_world::{Model, Operation, World};
use ix_trace_rs::trace;
use qsl_eval::simulation::model::ModelSystem;
use qsl_replay::{
    check_cyclic_mdp_probability_certificate_observing, check_probability_certificate_document,
    probabilistic_claim_identity, read_probability_proof_document, CyclicMdpProbabilityPhase as P,
    ExactProbabilityCertificateCheck, ProbabilityDocumentError,
};
use quire_exact::{Cancel, CancelCause};
fn produce(world: &World, name: &str) -> Vec<u8> {
    use qsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
    let records = analyze_items(
        &Engines::qsl().unwrap(),
        vec![exact_probabilistic_item(
            qsl_foundation::RequestIndex::new(0),
            world.request(name, qsl_replay::ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    records[0]
        .terminal()
        .and_then(|t| t.exact_certificate_document())
        .unwrap_or_else(|| panic!("expected proof: {records:#?}"))
        .to_vec()
}

fn model(op: &str, target: &str) -> Model {
    let package = "test/cyclic-mdp";
    let threshold = if op == ">=" {
        "rational(1,3)"
    } else {
        "rational(9,10)"
    };
    let post = "self.phase - outcome = 0";
    Model {
        package, object:"Item", population:"items", key:"x",
        integers:vec![("Phase",0,3),("Outcome",0,2)],
        fields:vec![("phase",format!("ix://{package}/Phase"))],
        operations:vec![
            Operation {name:"a",parameters:vec![("outcome",format!("ix://{package}/Outcome"))],modifies:vec!["phase"],deletes:false},
            Operation {name:"b",parameters:vec![("outcome",format!("ix://{package}/Outcome"))],modifies:vec!["phase"],deletes:false},
            Operation {name:"after",parameters:vec![],modifies:vec!["phase"],deletes:false}],
        body:format!("pre A using v on M::Item::a {{ self.phase = 0 }}\npost AD using v on M::Item::a {{ {post} }}\npre B using v on M::Item::b {{ self.phase = 0 }}\npost BD using v on M::Item::b {{ {post} }}\npre After using v on M::Item::after {{ self.phase = 1 or self.phase = 3 }}\npost AfterD using v on M::Item::after {{ self.phase - (if pre(self.phase) = 1 then 3 else 1) = 0 }}\nrandom Item::a(outcome) ~ {{ 0: 5, 1: 2, 2: 3 }};\nrandom Item::b(outcome) ~ {{ 0: 5, 1: 4, 2: 1 }};\nterminal Stop using v on M when {{ self.phase = 2 }};\nprobabilistic Likely using inf on M over (x: M::Item) under every scheduler on origin {{ probability {op} {threshold} [ eventually holds({target}) ] }}\n"),
        initial:serde_json::json!({"phase":{"integer":"0"}}),
    }
}
fn world(op: &str, target: &str) -> World {
    World::new(&model(op, target))
}

#[trace("TC-1404", "FR-201-AC-39", "FR-201-AC-42", "FR-198-AC-5")]
#[test]
fn first_hit_candidate_does_not_evaluate_query_after_target_and_original_model_still_admits() {
    for op in [">=", "<="] {
        let world = world(op, "if x.phase = 3 then (1 mod 0 = 0) else x.phase = 1");
        let bytes = produce(&world, "Likely");
        let document = read_probability_proof_document(&bytes, limits(), &Cancel::new()).unwrap();
        assert!(document.is_cyclic_mdp());
        assert_eq!(
            document.document().values.len(),
            3,
            "post-target state is absent"
        );
        let system = ModelSystem::new(world.subject().simulation).unwrap();
        let item = probabilistic_claim_identity(world.claim("Likely")).unwrap();
        assert!(matches!(
            check_probability_certificate_document(
                &system,
                world.package.graph().probabilistic_model().claims(),
                &item,
                &bytes,
                limits(),
                &Cancel::new()
            )
            .unwrap(),
            ExactProbabilityCertificateCheck::Accepted(_)
        ));
    }
}
#[trace("TC-1404", "FR-201-AC-42")]
#[test]
fn caller_stops_are_observed_in_actual_mdp_passes_and_limits_do_not_accept() {
    let fixture = world(">=", "x.phase = 1");
    let bytes = produce(&fixture, "Likely");
    let system = ModelSystem::new(fixture.subject().simulation).unwrap();
    let item = probabilistic_claim_identity(fixture.claim("Likely")).unwrap();
    let claims = fixture.package.graph().probabilistic_model().claims();
    for phase in [
        P::OriginalAdmission,
        P::StoppedGraph,
        P::Rows,
        P::ActionSums,
        P::Progress,
    ] {
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            let cancel = Cancel::new();
            let mut reached = false;
            let result = check_cyclic_mdp_probability_certificate_observing(
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
            assert_eq!(result, Err(ProbabilityDocumentError::Cancelled(cause)));
        }
    }
    for tiny in [0, 1, 128] {
        let mut bound = limits();
        bound.storage_bytes = tiny;
        assert!(check_probability_certificate_document(
            &system,
            claims,
            &item,
            &bytes,
            bound,
            &Cancel::new()
        )
        .is_err());
    }
    let empty = world("<=", "false");
    let empty_bytes = produce(&empty, "Likely");
    let empty_system = ModelSystem::new(empty.subject().simulation).unwrap();
    let empty_item = probabilistic_claim_identity(empty.claim("Likely")).unwrap();
    let empty_doc =
        read_probability_proof_document(&empty_bytes, limits(), &Cancel::new()).unwrap();
    assert!(empty_doc.document().targets.is_empty());
    assert!(empty_doc
        .document()
        .ranking
        .iter()
        .all(|(_, rank)| *rank == 0));
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let cancel = Cancel::new();
        let mut reached = false;
        let result = check_cyclic_mdp_probability_certificate_observing(
            &empty_system,
            empty.package.graph().probabilistic_model().claims(),
            &empty_item,
            &empty_bytes,
            limits(),
            &cancel,
            &mut |phase| {
                if phase == P::ZeroSupport {
                    reached = true;
                    cancel.cancel(cause);
                }
            },
        );
        assert!(reached);
        assert_eq!(result, Err(ProbabilityDocumentError::Cancelled(cause)));
    }
    let result = qsl_replay::check_cyclic_mdp_probability_certificate_with_identity(
        &system,
        claims,
        &item,
        &bytes,
        limits(),
        &Cancel::new(),
        qsl_foundation::IdentityLimits { input_bytes: 0 },
    );
    let Err(ProbabilityDocumentError::KeyEncoding(qsl_eval::simulation::EncodingRefusal::Limit(
        error,
    ))) = result
    else {
        panic!("actual identity-owner refusal: {result:?}");
    };
    assert_eq!(error.setting(), qsl_foundation::Setting::IdentityInputBytes);
    assert_eq!(error.configured_bound(), 0);
    assert!(error.actual() > 0);
    let mut representation = limits();
    representation.accounting.integer_bits = 1;
    assert!(matches!(
        check_probability_certificate_document(
            &system,
            claims,
            &item,
            &bytes,
            representation,
            &Cancel::new()
        ),
        Err(ProbabilityDocumentError::Accounting(_))
    ));
    let mut bound = limits();
    bound.accounting.work_units = 1;
    assert!(check_probability_certificate_document(
        &system,
        claims,
        &item,
        &bytes,
        bound,
        &Cancel::new()
    )
    .is_err());
}

#[trace("TC-1404", "FR-201-AC-42")]
#[test]
fn fallback_attempts_keep_the_caller_storage_ledger() {
    let fixture = world(">=", "x.phase = 1");
    let bytes = produce(&fixture, "Likely");
    let document = read_probability_proof_document(&bytes, limits(), &Cancel::new()).unwrap();
    let certificate = qsl_replay::ExactProbabilityCertificate {
        values: document.document().values.clone(),
    };
    let system = ModelSystem::new(fixture.subject().simulation).unwrap();
    let item = probabilistic_claim_identity(fixture.claim("Likely")).unwrap();
    let claims = fixture.package.graph().probabilistic_model().claims();
    let minimum = |fallback: bool| {
        let mut low = 0;
        let mut high = limits().storage_bytes;
        while low < high {
            let middle = low + (high - low) / 2;
            let mut bound = limits();
            bound.storage_bytes = middle;
            let result = if fallback {
                qsl_replay::probability_proof_document(
                    &system,
                    claims,
                    &item,
                    &certificate,
                    bound,
                    &Cancel::new(),
                )
            } else {
                qsl_replay::cyclic_mdp_probability_certificate_document(
                    &system,
                    claims,
                    &item,
                    &certificate,
                    bound,
                    &Cancel::new(),
                )
            };
            match result {
                Ok(_) => high = middle,
                Err(ProbabilityDocumentError::Limit {
                    limit: qsl_replay::ProbabilityDocumentLimit::StorageBytes,
                    ..
                }) => low = middle + 1,
                Err(error) => panic!("unexpected construction stop: {error:?}"),
            }
        }
        low
    };
    let direct = minimum(false);
    let fallback = minimum(true);
    assert!(
        fallback > direct,
        "the failed legacy attempt remains charged: {fallback} <= {direct}"
    );
}

#[trace("TC-1404", "FR-201-AC-39", "FR-201-AC-42")]
#[test]
fn undefined_query_after_target_does_not_skip_original_markov_admission() {
    let target = "if x.phase = 3 then (1 mod 0 = 0) else x.phase = 1";
    let valid = world(">=", target);
    let bytes = produce(&valid, "Likely");
    let document = read_probability_proof_document(&bytes, limits(), &Cancel::new()).unwrap();
    let certificate = qsl_replay::ExactProbabilityCertificate {
        values: document.document().values.clone(),
    };
    let mut invalid = model(">=", target);
    invalid.body = invalid.body.replace(
        "self.phase - (if pre(self.phase) = 1 then 3 else 1) = 0",
        "self.phase = 1 or self.phase = 3",
    );
    let invalid = World::new(&invalid);
    let system = ModelSystem::new(invalid.subject().simulation).unwrap();
    let item = probabilistic_claim_identity(invalid.claim("Likely")).unwrap();
    assert!(
        qsl_replay::cyclic_mdp_probability_certificate_document(
            &system,
            invalid.package.graph().probabilistic_model().claims(),
            &item,
            &certificate,
            limits(),
            &Cancel::new()
        )
        .is_err(),
        "multiple post-state choices after P must still refuse"
    );
}

#[trace("TC-1404", "TC-636", "FR-201-AC-42", "FR-201-AC-24")]
#[test]
fn pre_target_undefined_retains_its_actual_cause_in_new_and_old_adapters() {
    let valid = world(">=", "x.phase = 1");
    let bytes = produce(&valid, "Likely");
    let document = read_probability_proof_document(&bytes, limits(), &Cancel::new()).unwrap();
    let certificate = qsl_replay::ExactProbabilityCertificate {
        values: document.document().values.clone(),
    };
    for dtmc in [false, true] {
        let mut specification = model(">=", "if x.phase = 0 then (1 mod 0 = 0) else x.phase = 1");
        if dtmc {
            specification.body = specification.body.replace(
                "pre B using v on M::Item::b { self.phase = 0 }",
                "pre B using v on M::Item::b { false }",
            );
        }
        let fixture = World::new(&specification);
        let system = ModelSystem::new(fixture.subject().simulation).unwrap();
        let item = probabilistic_claim_identity(fixture.claim("Likely")).unwrap();
        let claims = fixture.package.graph().probabilistic_model().claims();
        let result = qsl_replay::probability_proof_document(
            &system,
            claims,
            &item,
            &certificate,
            limits(),
            &Cancel::new(),
        );
        assert!(
            matches!(result, Err(ProbabilityDocumentError::UndefinedPredicate(_))),
            "{result:?}"
        );
        let result = qsl_replay::admit_cyclic_eventual_certificate_subject_with_identity(
            &system,
            claims,
            &item,
            limits(),
            &Cancel::new(),
            qsl_foundation::IdentityLimits::default(),
        );
        assert!(
            matches!(result, Err(ProbabilityDocumentError::UndefinedPredicate(_))),
            "legacy: {result:?}"
        );
    }
}
