// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual checked fairness, cancellation phases and caller admission ceilings.
use super::long_run_certificate::{limits, produce};
use crate::prob_world::{self, World};
use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, DispositionKind, Engines, Settlement,
};
use qsl_eval::simulation::model::ModelSystem;
use qsl_foundation::RequestIndex;
use qsl_replay::{
    check_bounded_fair_probability_certificate_observing, check_probability_certificate_document,
    probabilistic_claim_identity, read_probability_proof_document, BoundedFairProbabilityPhase,
    ExactProbabilityCertificateCheck, ModelCheckLimits, ProbabilityDocumentError,
};
use quire_exact::{Cancel, CancelCause};

const CLAIM: &str = "probabilistic Likely using ev on M over (m: M::Msg) under every scheduler fair weak Msg::send_a fair strong each Msg::send_b on origin { probability >= 0.95 [ eventually[0,2] holds(m.delivered) ] }\n";

#[trace("TC-1406", "FR-201-AC-47", "FR-201-AC-48", "FR-201-AC-50")]
#[test]
fn actual_fairness_stages_preserve_requested_deadline_and_caller_ceilings() {
    let world = World::new(&prob_world::link(CLAIM));
    let bytes = produce(&world, "Likely");
    assert!(
        read_probability_proof_document(&bytes, limits(), &Cancel::new())
            .unwrap()
            .is_bounded_fair()
    );
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = probabilistic_claim_identity(world.claim("Likely")).unwrap();
    let claims = world.package.graph().probabilistic_model().claims();
    for phase in [
        BoundedFairProbabilityPhase::Intake,
        BoundedFairProbabilityPhase::Declaration,
        BoundedFairProbabilityPhase::Metadata,
        BoundedFairProbabilityPhase::Graph,
        BoundedFairProbabilityPhase::Equality,
    ] {
        let mut reached = false;
        let control = check_bounded_fair_probability_certificate_observing(
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
        assert!(reached);
        assert!(matches!(
            control,
            ExactProbabilityCertificateCheck::Accepted(_)
        ));
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            let cancel = Cancel::new();
            let mut reached = false;
            let stopped = check_bounded_fair_probability_certificate_observing(
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
    for cap in 0..7 {
        let mut narrow = limits();
        match cap {
            0 => narrow.bytes = 0,
            1 => narrow.storage_bytes = 0,
            2 => narrow.states = 0,
            3 => narrow.edges = 0,
            4 => narrow.accounting.work_units = 0,
            5 => narrow.scalar_bits = 0,
            6 => narrow.accounting.integer_bits = 0,
            _ => unreachable!(),
        }
        let stopped = check_probability_certificate_document(
            &system,
            claims,
            &item,
            &bytes,
            narrow,
            &Cancel::new(),
        );
        assert!(
            matches!(
                stopped,
                Err(ProbabilityDocumentError::Limit { .. }
                    | ProbabilityDocumentError::Accounting(_)
                    | ProbabilityDocumentError::IntegerText { .. })
            ),
            "cap {cap}: {stopped:?}"
        );
    }
}

#[trace("TC-1406", "FR-201-AC-47", "FR-201-AC-49")]
#[test]
fn empty_multiple_bindings_and_refuted_fair_bound_never_emit_new_profile_proofs() {
    let model = prob_world::link(CLAIM);
    for world in [
        World::with_objects(&model, &[]),
        World::with_objects(&model, &["a", "b"]),
        World::new(&prob_world::link(&CLAIM.replace("0.95", "0.97"))),
    ] {
        let records = analyze_items(
            &Engines::qsl().unwrap(),
            vec![exact_probabilistic_item(
                RequestIndex::new(0),
                world.request("Likely", ModelCheckLimits::default()),
            )],
            &Cancel::new(),
        );
        assert!(records[0].terminal().is_none());
        assert!(
            matches!(&records[0].settlement, Settlement::Disposed(disposal) if disposal.kind == DispositionKind::Unsupported)
        );
    }
    for threshold in ["0", "1"] {
        assert!(World::try_new(&prob_world::link(&CLAIM.replace("0.95", threshold))).is_err());
    }
    let global = CLAIM
        .replace("over (m: M::Msg) ", "")
        .replace("holds(m.delivered)", "holds(true)");
    let mut model = prob_world::link(&global);
    for operation in &mut model.operations { operation.modifies.clear(); }
    model.body = model.body.replace("self.attempts - pre(self.attempts) = 1 and self.delivered = (not lost)", "self.attempts = pre(self.attempts) and self.delivered = pre(self.delivered)");
    let world = World::with_objects(&model, &["a", "b"]);
    let records = analyze_items(&Engines::qsl().unwrap(), vec![exact_probabilistic_item(RequestIndex::new(0), world.request("Likely", ModelCheckLimits::default()))], &Cancel::new());
    let terminal = records[0].terminal().unwrap_or_else(|| panic!("actual multi-receiver admission: {:?}", records[0].settlement));
    let bytes = terminal.exact_certificate_document().unwrap().to_vec();
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
        ),
        Ok(ExactProbabilityCertificateCheck::Accepted(_))
    ));
}
