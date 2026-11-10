// SPDX-License-Identifier: AGPL-3.0-or-later
//! End-to-end exact long-run candidates checked against an independent graph.

use crate::prob_world::{self, Model, Operation, World};
use ix_trace_rs::trace;
use qsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use qsl_eval::simulation::model::ModelSystem;
use qsl_foundation::RequestIndex;
use qsl_replay::{
    check_probability_certificate_document, read_long_run_probability_certificate,
    write_long_run_probability_certificate, ExactProbabilityCertificateCheck,
    LongRunProbabilityDocument, ModelCheckLimits, ProbabilityDocumentError,
    ProbabilityDocumentLimits,
};
use quire_exact::{Cancel, CancelCause, Integer, Rational};

fn ratio(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}
pub(super) fn limits() -> ProbabilityDocumentLimits {
    ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 16_000_000,
        states: 128,
        edges: 1024,
        scalar_bits: 128,
        accounting: qsl_replay::AccountingLimits::default().0,
    }
}
fn claim(name: &str, predicate: &str, bound: &str) -> String {
    format!("probabilistic {name} using inf on M over (s: M::Server) under Steady on origin {{ long_run_fraction holds({predicate}) {bound} }}\n")
}
pub(super) fn availability() -> World {
    let package = "test/longrun-availability";
    let claims = [
        claim("Lower", "s.phase = 0", ">= rational(1800,1801)"),
        claim("Upper", "s.phase = 0", "<= rational(1800,1801)"),
        claim("TooHigh", "s.phase = 0", ">= rational(1801,1802)"),
        claim("TooLow", "s.phase = 0", "<= rational(999,1000)"),
    ]
    .concat();
    World::new(&Model {
        package, object: "Server", population: "servers", key: "s", integers: vec![("Phase", 0, 1)], fields: vec![("phase", format!("ix://{package}/Phase"))],
        operations: vec![Operation { name: "tick", parameters: vec![("to_down", prob_world::BOOLEAN.to_owned())], modifies: vec!["phase"], deletes: false }, Operation { name: "repair", parameters: vec![("ok", prob_world::BOOLEAN.to_owned())], modifies: vec!["phase"], deletes: false }],
        body: "pre Up using v on M::Server::tick { self.phase = 0 }\npost Ticked using v on M::Server::tick { (to_down and self.phase = 1) or (not to_down and self.phase = 0) }\npre Down using v on M::Server::repair { self.phase = 1 }\npost Repaired using v on M::Server::repair { (ok and self.phase = 0) or (not ok and self.phase = 1) }\nrandom Server::tick(to_down) ~ { true: 1, false: 1999 };\nrandom Server::repair(ok) ~ { true: 9, false: 1 };\nworkload Steady on M { weight Server::tick = 1; weight Server::repair = 1; }\n".to_owned() + &claims,
        initial: serde_json::json!({"phase": {"integer": "0"}}),
    })
}

pub(super) fn produce(world: &World, name: &str) -> Vec<u8> {
    let records = analyze_items(
        &Engines::qsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request(name, ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let terminal = records[0].terminal().unwrap();
    terminal
        .exact_certificate_document()
        .unwrap_or_else(|| panic!("expected proof: {:?}", terminal.value()))
        .to_vec()
}
fn check(
    world: &World,
    name: &str,
    bytes: &[u8],
    control: &Cancel,
    limits: ProbabilityDocumentLimits,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = qsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
    check_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        bytes,
        limits,
        control,
    )
}
fn accepted(world: &World, name: &str, doc: &LongRunProbabilityDocument) -> bool {
    let bytes = write_long_run_probability_certificate(doc, limits(), &Cancel::new()).unwrap();
    matches!(
        check(world, name, &bytes, &Cancel::new(), limits()).unwrap(),
        ExactProbabilityCertificateCheck::Accepted(_)
    )
}
#[trace("TC-1401", "FR-201-AC-27", "FR-199-AC-1")]
#[test]
fn availability_proofs_preserve_original_transitions_and_gain_bias() {
    let world = availability();
    for name in ["Lower", "Upper"] {
        let bytes = produce(&world, name);
        let doc = read_long_run_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
        assert_eq!(doc.components.len(), 1);
        assert_eq!(doc.components[0].value, ratio(1800, 1801));
        assert_eq!(doc.components[0].gain, ratio(1800, 1801));
        assert_eq!(doc.components[0].bias[0].value, ratio(0, 1));
        assert_eq!(doc.components[0].bias[1].value, ratio(-2000, 1801));
        assert!(doc.values.iter().all(|row| row.value == ratio(1800, 1801)));
        assert_eq!(
            write_long_run_probability_certificate(&doc, limits(), &Cancel::new()).unwrap(),
            bytes
        );
        assert!(accepted(&world, name, &doc));
        for mutation in 0..4 {
            let mut bad = doc.clone();
            match mutation {
                0 => bad.components[0].bias[1].value = ratio(2001, 1801),
                1 => bad.components[0].gain = ratio(0, 1),
                2 => {
                    for row in &mut bad.components[0].bias {
                        row.value = row.value.add(&ratio(1, 1));
                    }
                }
                _ => {
                    for row in &mut bad.values {
                        row.value = ratio(1, 1);
                    }
                    bad.components[0].value = ratio(1, 1);
                    bad.components[0].gain = ratio(1, 1);
                }
            }
            assert!(!accepted(&world, name, &bad), "mutation {mutation}");
        }
    }
}

fn finite(periodic: bool) -> World {
    let package = "test/longrun-finite";
    let body = if periodic {
        "post Next using v on M::Server::step { self.phase = 1-pre(self.phase) }\nworkload Steady on M { weight Server::step = 1; }\n".to_owned()+&claim("Fraction","s.phase = 1", ">= rational(1,2)")
    } else {
        "pre Begin using v on M::Server::step { self.phase = 0 }\npost Next using v on M::Server::step { (heads and self.phase = 1) or (not heads and self.phase = 2) }\nrandom Server::step(heads) ~ { true: 1, false: 2 };\nterminal Done using v on M when { self.phase > 0 };\nworkload Steady on M { weight Server::step = 1; }\n".to_owned()+&claim("Fraction","s.phase = 1", ">= rational(1,3)")
    };
    World::new(&Model {
        package,
        object: "Server",
        population: "servers",
        key: "s",
        integers: vec![("Phase", 0, if periodic { 1 } else { 2 })],
        fields: vec![("phase", format!("ix://{package}/Phase"))],
        operations: vec![Operation {
            name: "step",
            parameters: if periodic {
                vec![]
            } else {
                vec![("heads", prob_world::BOOLEAN.to_owned())]
            },
            modifies: vec!["phase"],
            deletes: false,
        }],
        body,
        initial: serde_json::json!({"phase":{"integer":"0"}}),
    })
}
#[trace("TC-1401", "FR-201-AC-28", "FR-201-AC-29")]
#[test]
fn independent_components_fix_terminal_values_transients_and_periodic_classes() {
    for periodic in [false, true] {
        let world = finite(periodic);
        let bytes = produce(&world, "Fraction");
        let doc = read_long_run_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
        assert_eq!(doc.components.len(), if periodic { 1 } else { 2 });
        assert!(accepted(&world, "Fraction", &doc));
        if periodic {
            assert!(doc.values.iter().all(|r| r.value == ratio(1, 2)));
        } else {
            assert_eq!(doc.values.len(), 3);
            for v in [ratio(0, 1), ratio(1, 3), ratio(1, 1)] {
                assert!(doc.values.iter().any(|r| r.value == v));
            }
        }
        let mut missing = doc.clone();
        missing.components.pop();
        assert!(!accepted(&world, "Fraction", &missing));
        if !periodic {
            let mut merged = doc.clone();
            let extra = merged.components.pop().unwrap();
            merged.components[0].bias.extend(extra.bias);
            merged.components[0]
                .bias
                .sort_by(|a, b| a.state.cmp(&b.state));
            assert!(!accepted(&world, "Fraction", &merged));
            let mut bad = doc.clone();
            bad.values
                .iter_mut()
                .find(|r| r.value == ratio(1, 3))
                .unwrap()
                .value = ratio(1, 2);
            assert!(!accepted(&world, "Fraction", &bad));
        }
    }
}
#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn long_run_control_stops_preserve_actual_causes_without_acceptance() {
    let world = availability();
    let bytes = produce(&world, "Lower");
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let cancel = Cancel::new();
        cancel.cancel(cause);
        assert_eq!(
            check(&world, "Lower", &bytes, &cancel, limits()),
            Err(ProbabilityDocumentError::Cancelled(cause))
        );
    }
    let mut capped = limits();
    capped.accounting.work_units = 0;
    assert!(matches!(
        check(&world, "Lower", &bytes, &Cancel::new(), capped),
        Err(ProbabilityDocumentError::Accounting(_))
    ));
    let mut capped = limits();
    capped.states = 1;
    assert!(matches!(
        check(&world, "Lower", &bytes, &Cancel::new(), capped),
        Err(ProbabilityDocumentError::Limit {
            limit: qsl_replay::ProbabilityDocumentLimit::States,
            ..
        })
    ));
    let mut capped = limits();
    capped.storage_bytes = 0;
    assert!(matches!(
        check(&world, "Lower", &bytes, &Cancel::new(), capped),
        Err(ProbabilityDocumentError::Limit {
            limit: qsl_replay::ProbabilityDocumentLimit::StorageBytes,
            ..
        })
    ));
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn phase_observed_cancellation_uses_the_actual_checker_control() {
    use qsl_replay::{
        check_long_run_probability_certificate_observing, LongRunProbabilityPhase as Phase,
    };
    let world = availability();
    let bytes = produce(&world, "Lower");
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = qsl_replay::probabilistic_claim_identity(world.claim("Lower")).unwrap();
    let claims = world.package.graph().probabilistic_model().claims();
    for phase in [
        Phase::Admission,
        Phase::Key,
        Phase::Predicate,
        Phase::Workload,
        Phase::Components,
        Phase::Membership,
        Phase::Arithmetic,
    ] {
        let mut reached = false;
        let control = check_long_run_probability_certificate_observing(
            &system,
            claims,
            &item,
            &bytes,
            limits(),
            &Cancel::new(),
            &mut |actual| {
                if actual == phase {
                    reached = true;
                }
            },
        )
        .unwrap();
        assert!(reached, "phase {phase:?}");
        assert!(matches!(
            control,
            ExactProbabilityCertificateCheck::Accepted(_)
        ));
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            let cancel = Cancel::new();
            let mut reached = false;
            let stopped = check_long_run_probability_certificate_observing(
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
            assert!(reached, "phase {phase:?}");
            assert_eq!(stopped, Err(ProbabilityDocumentError::Cancelled(cause)));
        }
    }
}

#[trace("TC-1401", "FR-201-AC-27")]
#[test]
fn valid_long_run_equations_still_require_the_checked_initial_threshold() {
    let world = availability();
    let bytes = produce(&world, "Lower");
    let original = read_long_run_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
    for name in ["TooHigh", "TooLow"] {
        let mut document = original.clone();
        document.identity = qsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
        let bytes =
            write_long_run_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
        let result = check(&world, name, &bytes, &Cancel::new(), limits()).unwrap();
        let ExactProbabilityCertificateCheck::Rejected(rejection) = result else {
            panic!("threshold must reject")
        };
        assert_eq!(
            rejection.rule,
            qsl_replay::ProbabilityCertificateRule::ThresholdMissed
        );
    }
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn reachable_undefined_predicate_retains_its_family_cause() {
    use qsl_replay::long_run_probability_certificate_document;
    let source="probabilistic Dropped using inf on M over (c: M::Coin) under Even on origin { long_run_fraction holds(c.v = 0) >= rational(1,2) }\n";
    let world = World::new(&prob_world::coin(source));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = qsl_replay::probabilistic_claim_identity(world.claim("Dropped")).unwrap();
    let result = long_run_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        vec![],
        vec![],
        limits(),
        &Cancel::new(),
    );
    assert!(
        matches!(result,Err(ProbabilityDocumentError::UndefinedPredicate(qsl_eval::temporal::UndefinedCause::Family(ref cause))) if cause.reason==quire_semantic_value::catalog::UndefinedReason::AbsentKey)
    );
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn weighted_every_scheduler_and_finite_profile_claims_issue_no_long_run_document() {
    for source in [
        "probabilistic Unsupported using inf on M over (s: M::Server) under every scheduler on origin { long_run_fraction holds(s.phase = 0) >= rational(1,2) }",
        "probabilistic Unsupported using inf on M over (s: M::Server) under Steady on origin { long_run_fraction holds(s.phase = 0) weighted by duration >= rational(1,2) }",
        "probabilistic Unsupported using ev on M over (s: M::Server) under Steady on origin { long_run_fraction holds(s.phase = 0) >= rational(1,2) }",
    ] {
        let world=World::new(&prob_world::service(source));
        let records=analyze_items(&Engines::qsl().unwrap(),vec![exact_probabilistic_item(RequestIndex::new(0),world.request("Unsupported",ModelCheckLimits::default()))],&Cancel::new());
        let qsl_analyze::analyze::Settlement::Disposed(disposed)=&records[0].settlement else {panic!("expected unsupported disposition: {:?}",records[0]);};
        assert_eq!(disposed.kind,qsl_analyze::analyze::DispositionKind::Unsupported);
        assert!(records[0].terminal().is_none());assert!(records[0].exact_probabilistic.is_none());
    }
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn no_bound_instance_produces_no_document_or_fabricated_probability() {
    let source="probabilistic Empty using inf on M over (s: M::Server) under Steady on origin { long_run_fraction holds(s.phase = 0) >= rational(1,2) }";
    let world = World::with_objects(&prob_world::service(source), &[]);
    let records = analyze_items(
        &Engines::qsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Empty", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let qsl_analyze::analyze::Settlement::Disposed(disposed) = &records[0].settlement else {
        panic!("expected unsupported disposition: {:?}", records[0]);
    };
    assert_eq!(
        disposed.kind,
        qsl_analyze::analyze::DispositionKind::Unsupported
    );
    assert!(records[0].terminal().is_none());
    assert!(records[0].exact_probabilistic.is_none());
    assert!(records[0].counterexample().is_none());
}

#[trace("TC-1401", "FR-201-AC-29")]
#[test]
fn changed_optimization_refuses_the_checked_objective() {
    let world = availability();
    let bytes = produce(&world, "Lower");
    let mut document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    document["objective"]["optimization"] = serde_json::json!("minimum");
    let bytes = serde_json::to_vec(&document).unwrap();
    assert_eq!(
        check(&world, "Lower", &bytes, &Cancel::new(), limits()),
        Err(ProbabilityDocumentError::ObjectiveMismatch)
    );
}

#[trace("TC-1401", "FR-201-AC-29")]
#[test]
fn missing_long_run_evidence_members_reject_at_their_public_member_locus() {
    use qsl_replay::{
        ProbabilityCertificateLocus, ProbabilityCertificateRejection, ProbabilityCertificateRule,
    };
    let world = availability();
    let bytes = produce(&world, "Lower");
    let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for member in ["values", "bias", "gain", "value"] {
        let mut document = original.clone();
        let locus = if member == "values" {
            document.as_object_mut().unwrap().remove(member);
            "values"
        } else {
            document["components"][0]
                .as_object_mut()
                .unwrap()
                .remove(member);
            "components"
        };
        let bytes = serde_json::to_vec(&document).unwrap();
        assert_eq!(
            check(&world, "Lower", &bytes, &Cancel::new(), limits()),
            Ok(ExactProbabilityCertificateCheck::Rejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::Malformed,
                    at: ProbabilityCertificateLocus::Member(locus)
                }
            )),
            "missing {member}"
        );
    }
    assert!(matches!(
        check(&world, "Lower", &bytes, &Cancel::new(), limits()).unwrap(),
        ExactProbabilityCertificateCheck::Accepted(_)
    ));
}

#[trace("TC-1401", "FR-201-AC-29")]
#[test]
fn conflicting_long_run_identity_refuses_before_key_enumeration() {
    use qsl_replay::{
        check_long_run_probability_certificate_observing, LongRunProbabilityPhase as Phase,
    };
    let world = availability();
    let bytes = produce(&world, "Lower");
    let mut document =
        read_long_run_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
    document.identity = qsl_replay::probabilistic_claim_identity(world.claim("Upper")).unwrap();
    let foreign =
        write_long_run_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = qsl_replay::probabilistic_claim_identity(world.claim("Lower")).unwrap();
    let mut entered = false;
    assert_eq!(
        check_long_run_probability_certificate_observing(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &item,
            &foreign,
            limits(),
            &Cancel::new(),
            &mut |phase| {
                if phase == Phase::Key {
                    entered = true;
                }
            }
        ),
        Err(ProbabilityDocumentError::IdentityMismatch)
    );
    assert!(
        !entered,
        "foreign identity must stop before the first key is enumerated"
    );
    let mut entered = false;
    let control = check_long_run_probability_certificate_observing(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        &bytes,
        limits(),
        &Cancel::new(),
        &mut |phase| {
            if phase == Phase::Key {
                entered = true;
            }
        },
    )
    .unwrap();
    assert!(entered);
    assert!(matches!(
        control,
        ExactProbabilityCertificateCheck::Accepted(_)
    ));
}

#[trace("TC-1401", "FR-201-AC-29")]
#[test]
fn conflicting_identity_precedes_source_provision_and_compilation() {
    use qsl_replay::{
        check_probability_certificate_source, ByteProvision, DefinitionSourceRef,
        ProbabilitySourceError, ProbabilitySourceLimits, ProbabilitySourceRequest,
    };
    let world = availability();
    let bytes = produce(&world, "Lower");
    let mut document =
        read_long_run_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
    document.identity = qsl_replay::probabilistic_claim_identity(world.claim("Upper")).unwrap();
    let foreign =
        write_long_run_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
    let item = qsl_replay::probabilistic_claim_identity(world.claim("Lower")).unwrap();
    let source = DefinitionSourceRef::from_wire(("test".to_owned(), "unprovided".to_owned()));
    let provision = ByteProvision::admit(
        vec![],
        vec![],
        std::iter::empty::<(String, &DefinitionSourceRef)>(),
    )
    .unwrap();
    let expected = qsl_replay::spine::SourceComponent {
        authority: "test".to_owned(),
        identity: "unprovided".to_owned(),
        content: "invalid source".to_owned(),
    };
    let check = |certificate: &[u8]| {
        check_probability_certificate_source(
            ProbabilitySourceRequest {
                source: &source,
                expected_source: &expected,
                package_name: "never-compiled",
                provision: &provision,
                item: &item,
                initial: vec![qsl_semantics::model::observation::DocumentRef {
                    authority: "test".to_owned(),
                    identity: "absent-state".to_owned(),
                }],
                universes: vec![],
                certificate,
            },
            ProbabilitySourceLimits {
                spine: qsl_replay::spine::SpineLimits::default(),
                observation: qsl_semantics::model::observation::ObservationLimits::default(),
                expansion: qsl_eval::simulation::model::ExpansionLimits {
                    max_candidates: 100_000,
                },
                scalar: qsl_replay::AccountingLimits::default().0,
                document: limits(),
            },
            &Cancel::new(),
        )
    };
    assert!(matches!(
        check(&foreign),
        Err(ProbabilitySourceError::Document(
            ProbabilityDocumentError::IdentityMismatch
        ))
    ));
    assert!(
        matches!(check(&bytes), Err(ProbabilitySourceError::SourceMismatch)),
        "matching identity must reach the missing provision control"
    );
}
