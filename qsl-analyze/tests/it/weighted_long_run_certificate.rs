// SPDX-License-Identifier: AGPL-3.0-or-later
//! Weighted long-run proofs checked from actual named draw rewards.

use super::long_run_certificate::{availability_with_claims, limits, produce};
use crate::prob_world::{self, Model, Operation, World};
use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, DispositionKind, Engines, Settlement,
};
use qsl_eval::simulation::model::ModelSystem;
use qsl_foundation::RequestIndex;
use qsl_replay::{
    check_probability_certificate_document, read_weighted_long_run_probability_certificate,
    write_weighted_long_run_probability_certificate, ExactProbabilityCertificateCheck,
    ModelCheckLimits, ProbabilityDocumentError, ProbabilityDocumentLimits,
    WeightedLongRunProbabilityDocument,
};
use quire_exact::{Cancel, CancelCause, Integer, Rational};

fn ratio(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}
fn claim(name: &str, predicate: &str, bound: &str) -> String {
    format!("probabilistic {name} using inf on M over (s: M::Server) under Steady on origin {{ long_run_fraction holds({predicate}) weighted by duration {bound} }}\n")
}
fn availability() -> World {
    availability_with_claims(
        [
            claim("Lower", "s.phase = 0", ">= rational(600,601)"),
            claim("Upper", "s.phase = 0", "<= rational(600,601)"),
            claim("HighControl", "s.phase = 0", "<= rational(999,1000)"),
            claim("TooHigh", "s.phase = 0", ">= rational(999,1000)"),
            claim("TooLow", "s.phase = 0", "<= rational(599,600)"),
        ]
        .concat(),
        "reward duration on Server::tick = 1;\nreward duration on Server::repair = 3;\n",
    )
}
fn check(
    world: &World,
    name: &str,
    bytes: &[u8],
    cancel: &Cancel,
    limits: ProbabilityDocumentLimits,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let identity = qsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
    check_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &identity,
        bytes,
        limits,
        cancel,
    )
}
fn accepted(world: &World, name: &str, document: &WeightedLongRunProbabilityDocument) -> bool {
    let bytes = write_weighted_long_run_probability_certificate(document, limits(), &Cancel::new())
        .unwrap();
    matches!(
        check(world, name, &bytes, &Cancel::new(), limits()).unwrap(),
        ExactProbabilityCertificateCheck::Accepted(_)
    )
}
#[trace("TC-1402", "FR-201-AC-31", "FR-201-AC-33", "FR-199-AC-3")]
#[test]
fn weighted_availability_checks_ratio_gain_bias_and_strict_objective() {
    let world = availability();
    for name in ["Lower", "Upper", "HighControl"] {
        let bytes = produce(&world, name);
        let document =
            read_weighted_long_run_probability_certificate(&bytes, limits(), &Cancel::new())
                .unwrap();
        assert_eq!(document.weight.name.as_str(), "duration");
        assert!(document.weight.unit.is_none());
        assert_eq!(document.certificate.components.len(), 1);
        let component = &document.certificate.components[0];
        assert_eq!(component.value, ratio(600, 601));
        assert_eq!(component.gain, ratio(600, 601));
        assert_eq!(component.bias[0].value, ratio(0, 1));
        assert_eq!(component.bias[1].value, ratio(-2000, 601));
        assert!(document
            .certificate
            .values
            .iter()
            .all(|row| row.value == ratio(600, 601)));
        assert_eq!(
            write_weighted_long_run_probability_certificate(&document, limits(), &Cancel::new())
                .unwrap(),
            bytes
        );
        assert!(accepted(&world, name, &document));
        for mutation in 0..4 {
            let mut bad = document.clone();
            match mutation {
                0 => bad.certificate.components[0].gain = ratio(0, 1),
                1 => bad.certificate.components[0].bias[1].value = ratio(-2001, 601),
                2 => {
                    for row in &mut bad.certificate.components[0].bias {
                        row.value = row.value.add(&ratio(1, 1));
                    }
                }
                _ => bad.certificate.values[0].value = ratio(1800, 1801),
            }
            assert!(!accepted(&world, name, &bad), "mutation {mutation}");
        }
        let mut bad = document.clone();
        bad.weight.name = quire_exact::Identifier::new("other").unwrap();
        let bytes = write_weighted_long_run_probability_certificate(&bad, limits(), &Cancel::new())
            .unwrap();
        assert_eq!(
            check(&world, name, &bytes, &Cancel::new(), limits()),
            Err(ProbabilityDocumentError::ObjectiveMismatch)
        );
    }
}
fn periodic(initial: i64, a_rewards: &str, b_reward: &str, terminal: bool) -> World {
    World::new(&periodic_model(initial, a_rewards, b_reward, terminal))
}
fn periodic_model(initial: i64, a_rewards: &str, b_reward: &str, terminal: bool) -> Model {
    let package = "test/weighted-longrun-periodic";
    let operations = vec![
        Operation {
            name: "a",
            parameters: vec![("d", format!("ix://{package}/Weight"))],
            modifies: vec!["phase"],
            deletes: false,
        },
        Operation {
            name: "b",
            parameters: vec![],
            modifies: vec!["phase"],
            deletes: false,
        },
        Operation {
            name: "enter",
            parameters: vec![("heads", prob_world::BOOLEAN.to_owned())],
            modifies: vec!["phase"],
            deletes: false,
        },
        Operation {
            name: "stay",
            parameters: vec![],
            modifies: vec![],
            deletes: false,
        },
    ];
    World::new(&Model {package,object:"Server",population:"servers",key:"s",
        integers:vec![("Phase",0,3),("Weight",1,5)],fields:vec![("phase",format!("ix://{package}/Phase"))],operations,
        body: format!("pre A using v on M::Server::a {{ self.phase = 0 }}\npost NextA using v on M::Server::a {{ self.phase = 1 }}\npre B using v on M::Server::b {{ self.phase = 1 }}\npost NextB using v on M::Server::b {{ self.phase = 0 }}\npre Enter using v on M::Server::enter {{ self.phase = 2 }}\npost Entered using v on M::Server::enter {{ (heads and self.phase = 0) or (not heads and self.phase = 3) }}\npre Stay using v on M::Server::stay {{ self.phase = 3 }}\nrandom Server::a(d) ~ {{ {a_rewards} }};\nrandom Server::enter(heads) ~ {{ true: 1, false: 3 }};\nreward duration on Server::a = d;\nreward duration on Server::b = {b_reward};\nreward duration on Server::stay = 1;\nworkload Steady on M {{ weight Server::a = 1; weight Server::b = 1; weight Server::enter = 1; weight Server::stay = 1; }}\n{}{}",
            if terminal { "terminal End using v on M when { self.phase = 3 };\n" } else {""},
            claim("Fraction","s.phase = 0 or s.phase = 3",if initial == 2 {">= rational(11,12)"} else {">= rational(2,3)"})),
        initial:serde_json::json!({"phase":{"integer":initial.to_string()}}),
    }
}

#[trace("TC-1402", "FR-201-AC-32", "FR-201-AC-33")]
#[test]
fn weighted_draws_and_transients_use_component_fractions_not_pooled_payoffs() {
    for initial in [0, 2] {
        let world = periodic(initial, "1: 1, 3: 1", "1", false);
        let bytes = produce(&world, "Fraction");
        let doc = read_weighted_long_run_probability_certificate(&bytes, limits(), &Cancel::new())
            .unwrap();
        assert!(doc
            .certificate
            .components
            .iter()
            .any(|c| c.value == ratio(2, 3)
                && c.bias[0].value == ratio(0, 1)
                && c.bias[1].value == ratio(-2, 3)));
        assert!(accepted(&world, "Fraction", &doc));
        if initial == 2 {
            assert!(doc
                .certificate
                .values
                .iter()
                .any(|r| r.value == ratio(11, 12)));
            let mut bad = doc.clone();
            bad.certificate
                .values
                .iter_mut()
                .find(|r| r.value == ratio(11, 12))
                .unwrap()
                .value = ratio(8, 9);
            assert!(!accepted(&world, "Fraction", &bad));
        }
        let swapped = periodic(initial, "3: 1, 1: 1", "1", false);
        assert!(accepted(&swapped, "Fraction", &doc));
        let scaled = periodic(initial, "2: 1, 6: 1", "2", false);
        let scaled_bytes = produce(&scaled, "Fraction");
        let scaled_doc = read_weighted_long_run_probability_certificate(&scaled_bytes, limits(), &Cancel::new()).unwrap();
        assert_eq!(scaled_doc.certificate.components[0].value, ratio(2,3));
        assert_eq!(scaled_doc.certificate.components[0].bias[1].value, ratio(-4,3));
        assert!(!accepted(&scaled, "Fraction", &doc));
        let changed = periodic(initial, "1: 1, 5: 1", "1", false);
        assert!(!accepted(&changed, "Fraction", &doc));
    }
}
#[trace("TC-1402", "FR-201-AC-32", "FR-201-AC-34")]
#[test]
fn reachable_zero_weight_components_remain_typed_unsupported() {
    let world = periodic(2, "1: 1, 3: 1", "1", true);
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let identity = qsl_replay::probabilistic_claim_identity(world.claim("Fraction")).unwrap();
    // A fabricated otherwise plausible document cannot turn zero stationary weight into a ratio.
    let positive = periodic(2, "1: 1, 3: 1", "1", false);
    let bytes = produce(&positive, "Fraction");
    let mut document = read_weighted_long_run_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
    document.certificate.identity = identity.clone();
    let bytes = write_weighted_long_run_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
    assert!(matches!(
        qsl_replay::check_probability_certificate_document(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &identity,
            &bytes,
            limits(),
            &Cancel::new()
        ),
        Err(ProbabilityDocumentError::ZeroWeightComponent { .. })
    ));
    let records = analyze_items(
        &Engines::qsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Fraction", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let Settlement::Disposed(disposed) = &records[0].settlement else {
        panic!("{:?}", records[0]);
    };
    assert_eq!(disposed.kind, DispositionKind::Unsupported);
    assert!(records[0].terminal().is_none());
    assert!(records[0].exact_probabilistic.is_none());
}
#[trace("TC-1402", "FR-201-AC-34")]
#[test]
fn weighted_phase_controls_preserve_actual_cancellation_and_limits() {
    use qsl_replay::{
        check_long_run_probability_certificate_observing, LongRunProbabilityPhase as Phase,
    };
    let world = availability();
    let bytes = produce(&world, "Lower");
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let identity = qsl_replay::probabilistic_claim_identity(world.claim("Lower")).unwrap();
    for phase in [
        Phase::Admission,
        Phase::Key,
        Phase::Weight,
        Phase::Predicate,
        Phase::Workload,
        Phase::Components,
        Phase::Membership,
        Phase::Arithmetic,
    ] {
        let mut reached = false;
        assert!(matches!(
            check_long_run_probability_certificate_observing(
                &system,
                world.package.graph().probabilistic_model().claims(),
                &identity,
                &bytes,
                limits(),
                &Cancel::new(),
                &mut |p| reached |= p == phase
            )
            .unwrap(),
            ExactProbabilityCertificateCheck::Accepted(_)
        ));
        assert!(reached, "{phase:?}");
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            let cancel = Cancel::new();
            let mut reached = false;
            assert_eq!(
                check_long_run_probability_certificate_observing(
                    &system,
                    world.package.graph().probabilistic_model().claims(),
                    &identity,
                    &bytes,
                    limits(),
                    &cancel,
                    &mut |p| if p == phase {
                        reached = true;
                        cancel.cancel(cause);
                    }
                ),
                Err(ProbabilityDocumentError::Cancelled(cause))
            );
            assert!(reached);
        }
    }
    for control in 0..5 {
        let mut cap = limits();
        match control {
            0 => cap.accounting.work_units = 0,
            1 => cap.storage_bytes = 0,
            2 => cap.bytes = 1,
            3 => cap.states = 1,
            _ => cap.scalar_bits = 1,
        };
        assert!(
            check(&world, "Lower", &bytes, &Cancel::new(), cap).is_err(),
            "control {control}"
        );
    }
}

#[trace("TC-1402", "FR-201-AC-34")]
#[test]
fn actual_weight_representation_and_numeric_arithmetic_limits_are_separate() {
    let mut model=periodic_model(0,"65536: 1","65536",false);
    model.integers.iter_mut().find(|(name,_,_)|*name=="Weight").unwrap().2=65536;
    model.body=model.body.replace(">= rational(2,3)",">= rational(1,2)");
    let world=World::new(&model);
    let bytes=produce(&world,"Fraction");
    let document=read_weighted_long_run_probability_certificate(&bytes,limits(),&Cancel::new()).unwrap();
    assert_eq!(document.certificate.components[0].bias[1].value,ratio(-32768,1));
    let mut control=limits(); control.scalar_bits=16;
    assert!(matches!(check(&world,"Fraction",&bytes,&Cancel::new(),control),Err(ProbabilityDocumentError::Limit{limit:qsl_replay::ProbabilityDocumentLimit::RationalBits,actual:17,..})));
    let mut control=limits();control.accounting.integer_bits=16;
    assert!(matches!(check(&world,"Fraction",&bytes,&Cancel::new(),control),Err(ProbabilityDocumentError::Accounting(ref stop)) if stop.limit_kind==quire_exact::LimitKind::IntegerBits));
}

#[trace("TC-1402", "FR-201-AC-34")]
#[test]
fn global_negative_admission_precedes_proofs_and_zero_binding_invents_nothing() {
    for initial in [0,2] {
        let world=periodic(initial,"1: 1, 3: 1","-1",false);
        let error=ModelSystem::new(world.subject().simulation).err().expect("FR187 rejects an enumerable negative reward");
        let qsl_semantics::model::observation::AdmissionFailure::Refused(ref record)=error else {panic!("{error:?}");};
        assert_eq!(record.catalog.code(),"invalid_probabilistic_model");assert_eq!(record.catalog.cause(),"negative-reward");
        let records=analyze_items(&Engines::qsl().unwrap(),vec![exact_probabilistic_item(RequestIndex::new(0),world.request("Fraction",ModelCheckLimits::default()))],&Cancel::new());
        assert!(matches!(records[0].terminal().unwrap().value(),qsl_replay::TerminalValue::Declined{cause:qsl_replay::ProofRefusalCause::InvalidInput,..}));
        assert!(records[0].exact_probabilistic.is_none());assert!(records[0].counterexample().is_none());
    }
    let model=periodic_model(0,"1: 1, 3: 1","1",false);
    let world=World::with_objects(&model,&[]);
    let records=analyze_items(&Engines::qsl().unwrap(),vec![exact_probabilistic_item(RequestIndex::new(0),world.request("Fraction",ModelCheckLimits::default()))],&Cancel::new());
    let Settlement::Disposed(disposed)=&records[0].settlement else {panic!("{:?}",records[0]);};
    assert_eq!(disposed.kind,DispositionKind::Unsupported);assert!(records[0].terminal().is_none());assert!(records[0].exact_probabilistic.is_none());
}

#[trace("TC-1402", "FR-201-AC-34")]
#[test]
fn admitted_object_parameter_draws_do_not_coerce_negative_weight_to_zero() {
    let mut model=periodic_model(0,"1: 1, 3: 1","1",false);
    model.operations[0].parameters.push(("who",format!("ix://{}/Server",model.package)));
    model.body=model.body.replace("reward duration on Server::a = d;","reward duration on Server::a = if who = who then -1 else 1;");
    let world=World::new(&model);
    let system=ModelSystem::new(world.subject().simulation).expect("object-domain reward is deferred by FR187 admission");
    let identity=qsl_replay::probabilistic_claim_identity(world.claim("Fraction")).unwrap();
    assert_eq!(qsl_replay::weighted_long_run_probability_certificate_document(&system,world.package.graph().probabilistic_model().claims(),&identity,vec![],vec![],limits(),&Cancel::new()),Err(ProbabilityDocumentError::Unsupported));
    let records=analyze_items(&Engines::qsl().unwrap(),vec![exact_probabilistic_item(RequestIndex::new(0),world.request("Fraction",ModelCheckLimits::default()))],&Cancel::new());
    let Settlement::Disposed(disposed)=&records[0].settlement else {panic!("{:?}",records[0]);};
    assert_eq!(disposed.kind,DispositionKind::Unsupported);assert!(records[0].terminal().is_none());assert!(records[0].exact_probabilistic.is_none());
}
