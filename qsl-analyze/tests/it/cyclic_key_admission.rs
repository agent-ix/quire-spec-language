// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cyclic and native admission exercise the actual bounded owner route.

use crate::prob_world::{Model, Operation, World};
use ix_trace_rs::trace;
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::TransitionSystem;
use qsl_foundation::{IdentityLimits, Setting};
use qsl_replay::{ProbabilityDocumentError, ProbabilityDocumentLimits};
use quire_exact::{Cancel, CancelCause};

fn world() -> World {
    let package = "test/cyclic-key-growth";
    World::new(&Model {
        package, object: "Server", population: "servers", key: "s",
        integers: vec![("Level", 0, 99)],
        fields: vec![("level", format!("ix://{package}/Level"))],
        operations: vec![
            Operation { name: "grow", parameters: vec![], modifies: vec!["level"], deletes: false },
            Operation { name: "back", parameters: vec![], modifies: vec!["level"], deletes: false },
        ],
        body: "pre Live using v on M::Server::grow { self.level = 0 }\npost Grown using v on M::Server::grow { self.level = 99 }\npre Back using v on M::Server::back { self.level = 99 }\npost Backed using v on M::Server::back { self.level = 0 }\nprobabilistic Growth using inf on M over (s: M::Server) under every scheduler on origin { probability >= rational(1,2) [ eventually holds(s.level = 99) ] }".to_owned(),
        initial: serde_json::json!({"level":{"integer":"0"}}),
    })
}
fn limits() -> ProbabilityDocumentLimits {
    ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 16_000_000,
        states: 100,
        edges: 100,
        scalar_bits: 128,
        accounting: qsl_replay::AccountingLimits::default().0,
    }
}
fn admit(
    world: &World,
    system: &ModelSystem<'_>,
    limits: ProbabilityDocumentLimits,
    identity: IdentityLimits,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    qsl_replay::admit_cyclic_eventual_certificate_subject_with_identity(
        system,
        world.package.graph().probabilistic_model().claims(),
        &qsl_replay::probabilistic_claim_identity(world.claim("Growth")).unwrap(),
        limits,
        cancel,
        identity,
    )
}
fn identity_error(error: ProbabilityDocumentError, bound: u64) {
    let ProbabilityDocumentError::KeyEncoding(qsl_eval::simulation::EncodingRefusal::Limit(
        exceeded,
    )) = error
    else {
        panic!("must preserve the owner identity limit: {error:?}");
    };
    assert_eq!(exceeded.setting(), Setting::IdentityInputBytes);
    assert_eq!(exceeded.configured_bound(), bound);
    assert!(exceeded.actual() > u128::from(bound));
}

#[trace("TC-636", "FR-201-AC-26", "FR-282-AC-19")]
#[test]
fn cyclic_and_native_preserve_raw_post_and_full_key_identity_causes() {
    let world = world();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let cancel = Cancel::new();
    let mut no_transport = limits();
    no_transport.bytes = 0;
    admit(
        &world,
        &system,
        no_transport,
        IdentityLimits::default(),
        &cancel,
    )
    .unwrap();
    // State level 99 needs seven representation bits; semantic edge masses
    // are one, so the probability arithmetic ceiling may still be one bit.
    let mut probability_bits = limits();
    probability_bits.scalar_bits = 1;
    admit(
        &world,
        &system,
        probability_bits,
        IdentityLimits::default(),
        &cancel,
    )
    .unwrap();
    let mut representation_bits = limits();
    representation_bits.accounting.integer_bits = 1;
    assert!(
        matches!(admit(&world, &system, representation_bits, IdentityLimits::default(), &cancel),
        Err(ProbabilityDocumentError::Accounting(incomplete))
            if incomplete.limit_kind == quire_exact::LimitKind::IntegerBits)
    );
    identity_error(
        admit(
            &world,
            &system,
            limits(),
            IdentityLimits { input_bytes: 0 },
            &cancel,
        )
        .unwrap_err(),
        0,
    );
    let raw = system.state_key(&system.initial()[0]).unwrap().0;
    let bound = u64::try_from(raw.len()).unwrap();
    let native = qsl_replay::ExactProbLimits::default();
    identity_error(
        qsl_replay::admit_acyclic_probabilistic_refutation_subject_with_identity(
            &system,
            &native,
            &cancel,
            IdentityLimits { input_bytes: bound },
        )
        .unwrap_err(),
        bound,
    );
    let binding = qsl_replay::claim_binding(&system, world.claim("Growth"))
        .unwrap()
        .unwrap();
    let full =
        qsl_replay::ProbabilityProductKey::for_model_state(Some(&binding.reference), &raw).unwrap();
    let bound = u64::try_from(full.as_bytes().len()).unwrap();
    identity_error(
        admit(
            &world,
            &system,
            limits(),
            IdentityLimits { input_bytes: bound },
            &cancel,
        )
        .unwrap_err(),
        bound,
    );
}

#[trace("TC-636", "FR-201-AC-26", "FR-282-AC-19")]
#[test]
fn real_owner_route_preserves_cumulative_stops_and_mid_operation_cancellation() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, OnceLock};
    let world = world();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let live = Cancel::new();
    let _count = live.count_charges();
    admit(&world, &system, limits(), IdentityLimits::default(), &live).unwrap();
    let total = live.charges();
    assert!(total > 2);
    for at in [1, total / 2, total] {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let seen = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&seen);
        let cancel = Cancel::observing(move || {
            if count.fetch_add(1, Ordering::Relaxed) + 1 == at {
                weak.upgrade()
                    .unwrap()
                    .get()
                    .unwrap()
                    .cancel(CancelCause::Deadline);
            }
        });
        slot.set(cancel.clone()).unwrap();
        assert_eq!(
            admit(
                &world,
                &system,
                limits(),
                IdentityLimits::default(),
                &cancel
            ),
            Err(ProbabilityDocumentError::Cancelled(CancelCause::Deadline))
        );
        assert!(
            seen.load(Ordering::Relaxed) >= at,
            "the owner may poll again while unwinding its typed stop"
        );
    }
    let mut storage = limits();
    storage.storage_bytes = 0;
    assert!(matches!(
        admit(
            &world,
            &system,
            storage,
            IdentityLimits::default(),
            &Cancel::new()
        ),
        Err(ProbabilityDocumentError::Limit {
            limit: qsl_replay::ProbabilityDocumentLimit::StorageBytes,
            bound: 0,
            ..
        })
    ));
    let mut work = limits();
    work.accounting.work_units = 0;
    assert!(
        matches!(admit(&world, &system, work, IdentityLimits::default(), &Cancel::new()),
        Err(ProbabilityDocumentError::Accounting(incomplete)) if incomplete.limit_kind == quire_exact::LimitKind::WorkUnits)
    );
    let mut retained = 0;
    let mut meter = quire_exact::Meter::new(limits().accounting);
    let cancel = Cancel::new();
    {
        let mut budget = qsl_replay::ProbabilityKeyAdmission::new(
            &mut meter,
            &mut retained,
            limits().storage_bytes,
            &cancel,
        );
        let raw = system
            .state_key_bytes_with_budget(
                &system.initial()[0],
                IdentityLimits::default(),
                &mut budget,
                &cancel,
            )
            .unwrap();
        let binding = qsl_replay::claim_binding(&system, world.claim("Growth"))
            .unwrap()
            .unwrap();
        qsl_replay::ProbabilityProductKey::for_model_state_with_budget(
            Some(&binding.reference),
            &raw,
            IdentityLimits::default(),
            &mut budget,
            &cancel,
        )
        .unwrap();
    }
    let mut cumulative = limits();
    cumulative.storage_bytes = retained;
    assert!(retained > 0);
    assert!(
        matches!(admit(&world, &system, cumulative, IdentityLimits::default(), &Cancel::new()),
        Err(ProbabilityDocumentError::Limit { limit: qsl_replay::ProbabilityDocumentLimit::StorageBytes, bound, actual })
            if bound == retained && actual > retained),
        "a ceiling sufficient for one real raw/full key cannot reset for the rest of the graph"
    );
}
