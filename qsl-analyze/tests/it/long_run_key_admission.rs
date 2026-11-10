// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual raw/full identity admission and producer expansion use one caller scope.

use crate::long_run_certificate::{availability, limits, produce};
use crate::prob_world::{Model, Operation, World};
use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::{
    build_product_with_key_budget, BudgetedProductError, ProbBuild,
};
use qsl_eval::simulation::model::{ModelKeyBudget, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_foundation::{BackendId, IdentityLimits, RequestIndex, Setting};
use qsl_replay::{
    check_long_run_probability_certificate_with_identity, IncompleteCause, ModelCheckLimit,
    OutcomeItem, ProbabilityDocumentError, ProbabilityKeyAdmission, TerminalRecord,
};
use quire_exact::{
    Cancel, CancelCause, Integer, IntegerDecimalBudget, LimitKind, Meter, Reservation,
    ReservationDenied,
};

fn growth_model() -> Model {
    let package = "test/long-run-growth";
    Model{package,object:"Server",population:"servers",key:"s",integers:vec![("Level",0,99)],fields:vec![("level",format!("ix://{package}/Level"))],operations:vec![Operation{name:"grow",parameters:vec![],modifies:vec!["level"],deletes:false}],body:"pre Live using v on M::Server::grow { self.level = 0 }\npost Grown using v on M::Server::grow { self.level = 99 }\nterminal Done using v on M when { self.level = 99 };\nworkload Steady on M { weight Server::grow = 1; }\nprobabilistic Growth using inf on M over (s: M::Server) under Steady on origin { long_run_fraction holds(s.level = 99) >= rational(1,2) }".to_owned(),initial:serde_json::json!({"level":{"integer":"0"}})}
}

fn growth() -> World {
    World::new(&growth_model())
}
fn undecided_model() -> Model {
    let mut model = growth_model();
    model.body=model.body.replace("post Grown using v on M::Server::grow { self.level = 99 }","post Grown using vc on M::Server::grow { self.level = 99 or (self.level = 10 and 1 mod (self.level - 10) = 0) }");
    model
}
fn undecided_candidate() -> World {
    World::with_value_profile(&undecided_model())
}
fn reward_model(mut model: Model) -> Model {
    model.body = model.body.replace(
        "long_run_fraction holds(s.level = 99) >= rational(1,2)",
        "expected accumulate cost until holds(s.level = 99) >= 3",
    );
    model.body.push_str("\nreward cost on Server::grow = 3;\n");
    model
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn identity_bytes_keep_their_actual_setting_and_wire_limit() {
    let world = availability();
    let bytes = produce(&world, "Lower");
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = qsl_replay::probabilistic_claim_identity(world.claim("Lower")).unwrap();
    let error = check_long_run_probability_certificate_with_identity(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        &bytes,
        limits(),
        IdentityLimits { input_bytes: 0 },
        &Cancel::new(),
    )
    .unwrap_err();
    let ProbabilityDocumentError::KeyEncoding(qsl_eval::simulation::EncodingRefusal::Limit(
        exceeded,
    )) = error
    else {
        panic!("expected identity-input refusal, got {error:?}")
    };
    assert_eq!(exceeded.setting(), Setting::IdentityInputBytes);
    assert_eq!(exceeded.configured_bound(), 0);
    assert!(exceeded.actual() > 0);
    let record = TerminalRecord::incomplete(
        RequestIndex::new(0),
        BackendId::new("en-5"),
        IncompleteCause::LimitReached {
            limit: ModelCheckLimit::IdentityInputBytes,
            value: 0,
        },
    );
    let wire = serde_json::to_value(OutcomeItem::from_terminal(&record)).unwrap();
    assert_eq!(
        wire["cause"],
        serde_json::json!({"kind":"limit-reached","limit":"input_bytes","setting":"identity.input_bytes","value":0})
    );
    assert!(wire.get("truth").is_none());
}

#[trace("TC-636", "FR-201-AC-22", "FR-282-AC-17")]
#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn producer_bounds_larger_candidate_keys_after_a_small_initial_key() {
    for world in [growth(), World::new(&reward_model(growth_model()))] {
        let system = ModelSystem::new(world.subject().simulation).unwrap();
        let initial = system.initial().pop().unwrap();
        let initial_bytes = system.state_key(&initial).unwrap().0.len() as u64;
        let cancel = Cancel::new();
        let mut meter = Meter::new(limits().accounting).with_cancel(cancel.clone());
        let mut storage = 0;
        let mut admission =
            ProbabilityKeyAdmission::new(&mut meter, &mut storage, limits().storage_bytes, &cancel);
        let result = build_product_with_key_budget(
            world.request("Growth", qsl_replay::ModelCheckLimits::default()),
            IdentityLimits {
                input_bytes: initial_bytes,
            },
            &mut admission,
            &cancel,
        );
        let Err(BudgetedProductError::Key(error)) = result else {
            panic!("growing candidate must stop at the owner ceiling: {result:?}")
        };
        let ProbabilityDocumentError::KeyEncoding(qsl_eval::simulation::EncodingRefusal::Limit(
            exceeded,
        )) = admission.refusal(error)
        else {
            panic!("expected typed identity byte exhaustion")
        };
        assert_eq!(exceeded.setting(), Setting::IdentityInputBytes);
        assert_eq!(exceeded.configured_bound(), initial_bytes);
        assert!(exceeded.actual() > u128::from(initial_bytes));
        let mut meter = Meter::new(limits().accounting);
        let mut storage = 0;
        let mut admission =
            ProbabilityKeyAdmission::new(&mut meter, &mut storage, limits().storage_bytes, &cancel);
        let control = build_product_with_key_budget(
            world.request("Growth", qsl_replay::ModelCheckLimits::default()),
            IdentityLimits::default(),
            &mut admission,
            &cancel,
        )
        .unwrap();
        let ProbBuild::Built(product) = control else {
            panic!("positive control must build")
        };
        assert_eq!(product.states.len(), 2);
    }
}

struct CancelLarge<'a> {
    inner: ProbabilityKeyAdmission<'a>,
    cancel: &'a Cancel,
    reached: bool,
    cause: Option<CancelCause>,
}
impl Reservation for CancelLarge<'_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        self.inner.reserve(bytes)
    }
}
impl IntegerDecimalBudget for CancelLarge<'_> {
    fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
        self.inner.work(units)
    }
}
impl ModelKeyBudget for CancelLarge<'_> {
    fn integer(&mut self, value: &Integer) -> Result<(), ReservationDenied> {
        // The admitted post is 99. Values 10 through 98 occur only while
        // the frame enumerates candidates, before the external post-key call.
        if value >= &Integer::from(10_i64) && value < &Integer::from(99_i64) {
            self.reached = true;
            if let Some(cause) = self.cause {
                self.cancel.cancel(cause);
            }
        }
        self.inner.integer(value)
    }
}

#[trace("TC-636", "FR-201-AC-22", "FR-282-AC-17")]
#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn producer_cancellation_observes_real_larger_candidate_representation() {
    for world in [
        undecided_candidate(),
        World::with_value_profile(&reward_model(undecided_model())),
    ] {
        for cause in [
            None,
            Some(CancelCause::Requested),
            Some(CancelCause::Deadline),
        ] {
            let cancel = Cancel::new();
            let mut meter = Meter::new(limits().accounting).with_cancel(cancel.clone());
            let mut storage = 0;
            let mut budget = CancelLarge {
                inner: ProbabilityKeyAdmission::new(
                    &mut meter,
                    &mut storage,
                    limits().storage_bytes,
                    &cancel,
                ),
                cancel: &cancel,
                reached: false,
                cause,
            };
            let result = build_product_with_key_budget(
                world.request("Growth", qsl_replay::ModelCheckLimits::default()),
                IdentityLimits::default(),
                &mut budget,
                &cancel,
            );
            assert!(
                budget.reached,
                "the owner must consult the caller for a larger frame candidate"
            );
            match (cause,result) {
            (None,Err(BudgetedProductError::Product(qsl_analyze::exact_probabilistic::ProductRefusal::UndecidedSuccessor(_))))=>{},
            (Some(cause),Err(BudgetedProductError::Key(error)))=>assert_eq!(budget.inner.refusal(error),ProbabilityDocumentError::Cancelled(cause)),
            (Some(cause),Err(BudgetedProductError::Product(qsl_analyze::exact_probabilistic::ProductRefusal::Stopped(qsl_analyze::exact_probabilistic::ProductStop::Cancelled(actual)))))=>assert_eq!(actual,cause),
            (cause,other)=>panic!("candidate control {cause:?} must retain actual undecided/cancellation semantics, got {other:?}"),
        }
        }
    }
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn caller_storage_and_work_remain_cumulative_across_raw_keys() {
    let world = availability();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let initial = system.initial().pop().unwrap();
    let cancel = Cancel::new();
    let mut meter = Meter::new(limits().accounting);
    let mut first_storage = 0;
    {
        let mut admission = ProbabilityKeyAdmission::new(
            &mut meter,
            &mut first_storage,
            limits().storage_bytes,
            &cancel,
        );
        system
            .state_key_bytes_with_budget(
                &initial,
                IdentityLimits::default(),
                &mut admission,
                &cancel,
            )
            .unwrap();
    }
    let first_work = meter.consumed(LimitKind::WorkUnits);
    assert!(first_storage > 0);
    assert!(first_work > 0);
    for work in [false, true] {
        let mut scalar = limits().accounting;
        if work {
            scalar.work_units = first_work * 2 - 1;
        }
        let mut meter = Meter::new(scalar);
        let mut storage = 0;
        let bound = if work {
            limits().storage_bytes
        } else {
            first_storage * 2 - 1
        };
        {
            let mut admission =
                ProbabilityKeyAdmission::new(&mut meter, &mut storage, bound, &cancel);
            system
                .state_key_bytes_with_budget(
                    &initial,
                    IdentityLimits::default(),
                    &mut admission,
                    &cancel,
                )
                .unwrap();
        }
        let mut admission = ProbabilityKeyAdmission::new(&mut meter, &mut storage, bound, &cancel);
        let error = system
            .state_key_bytes_with_budget(
                &initial,
                IdentityLimits::default(),
                &mut admission,
                &cancel,
            )
            .unwrap_err();
        let error = admission.refusal(error);
        if work {
            assert!(
                matches!(error,ProbabilityDocumentError::Accounting(ref x) if x.limit_kind==LimitKind::WorkUnits)
            );
        } else {
            assert!(
                matches!(error,ProbabilityDocumentError::Limit{limit:qsl_replay::ProbabilityDocumentLimit::StorageBytes,bound:actual_bound,..}if actual_bound==bound)
            );
        }
    }
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn model_representation_bits_are_distinct_from_probability_value_bits() {
    let world = growth();
    let bytes = produce(&world, "Growth");
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = qsl_replay::probabilistic_claim_identity(world.claim("Growth")).unwrap();
    let mut ceiling = limits();
    ceiling.scalar_bits = 2;
    let control = qsl_replay::check_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        &bytes,
        ceiling,
        &Cancel::new(),
    )
    .unwrap();
    assert!(matches!(control,qsl_replay::ExactProbabilityCertificateCheck::Accepted(_)),"model integer 99 needs seven representation bits while all probabilities and the threshold fit two");
    ceiling.accounting.integer_bits = 6;
    let result = qsl_replay::check_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        &bytes,
        ceiling,
        &Cancel::new(),
    );
    assert!(
        matches!(result,Err(ProbabilityDocumentError::Accounting(ref x)) if x.limit_kind==LimitKind::IntegerBits)
    );
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn full_product_parse_staging_and_output_share_the_raw_key_storage_scope() {
    let world = availability();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let initial = system.initial().pop().unwrap();
    let binding = qsl_replay::claim_binding(&system, world.claim("Lower")).unwrap();
    let reference = binding.as_ref().map(|b| &b.reference);
    let cancel = Cancel::new();
    let encode = |bound: u64,
                  storage: &mut u64|
     -> Result<qsl_replay::ProbabilityProductKey, ProbabilityDocumentError> {
        let mut meter = Meter::new(limits().accounting).with_cancel(cancel.clone());
        let mut admission = ProbabilityKeyAdmission::new(&mut meter, storage, bound, &cancel);
        let raw = system
            .state_key_bytes_with_budget(
                &initial,
                IdentityLimits::default(),
                &mut admission,
                &cancel,
            )
            .map_err(|error| admission.refusal(error))?;
        qsl_replay::ProbabilityProductKey::for_model_state_with_budget(
            reference,
            &raw,
            IdentityLimits::default(),
            &mut admission,
            &cancel,
        )
        .map_err(|error| admission.refusal(error))
    };
    let mut measured = 0;
    let key = encode(limits().storage_bytes, &mut measured).unwrap();
    assert!(measured > 0);
    let raw = system.state_key(&initial).unwrap().0;
    let expected = qsl_replay::ProbabilityProductKey::for_model_state(reference, &raw).unwrap();
    assert_eq!(key, expected);
    let mut exact = 0;
    assert_eq!(encode(measured, &mut exact).unwrap(), expected);
    assert_eq!(exact, measured);
    let mut short = 0;
    assert!(
        matches!(encode(measured-1,&mut short),Err(ProbabilityDocumentError::Limit{limit:qsl_replay::ProbabilityDocumentLimit::StorageBytes,bound,..})if bound==measured-1)
    );
    assert!(
        short < measured,
        "the denied reservation must not be performed"
    );
}

struct CancelProductReservation<'a> {
    cancel: &'a Cancel,
    cause: Option<CancelCause>,
    admission: Option<ProbabilityKeyAdmission<'a>>,
    reserved: usize,
}
impl Reservation for CancelProductReservation<'_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        self.reserved += bytes;
        if let Some(cause) = self.cause {
            self.cancel.cancel(cause);
        }
        match &mut self.admission {
            Some(admission) => admission.reserve(bytes),
            None => Ok(()),
        }
    }
}
impl IntegerDecimalBudget for CancelProductReservation<'_> {
    fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
        match &mut self.admission {
            Some(admission) => admission.work(units),
            None => Ok(()),
        }
    }
}
impl ModelKeyBudget for CancelProductReservation<'_> {
    fn integer(&mut self, value: &Integer) -> Result<(), ReservationDenied> {
        match &mut self.admission {
            Some(admission) => admission.integer(value),
            None => Ok(()),
        }
    }
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn full_product_reservation_phase_polls_actual_requested_and_deadline_causes() {
    let world = availability();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let initial = system.initial().pop().unwrap();
    let raw = system.state_key(&initial).unwrap().0;
    let binding = qsl_replay::claim_binding(&system, world.claim("Lower")).unwrap();
    let reference = binding.as_ref().map(|b| &b.reference);
    let expected = qsl_replay::ProbabilityProductKey::for_model_state(reference, &raw).unwrap();
    // Exercise both the real caller admission and a reservation provider that
    // leaves cancellation polling to the product wrapper. The trigger is an
    // actual parse/staging/output reservation, after the initial work charge.
    for caller_admission in [true, false] {
        for cause in [
            None,
            Some(CancelCause::Requested),
            Some(CancelCause::Deadline),
        ] {
            let cancel = Cancel::new();
            let mut meter = Meter::new(limits().accounting).with_cancel(cancel.clone());
            let mut storage = 0;
            let admission = caller_admission.then(|| {
                ProbabilityKeyAdmission::new(
                    &mut meter,
                    &mut storage,
                    limits().storage_bytes,
                    &cancel,
                )
            });
            let mut budget = CancelProductReservation {
                cancel: &cancel,
                cause,
                admission,
                reserved: 0,
            };
            let result = qsl_replay::ProbabilityProductKey::for_model_state_with_budget(
                reference,
                &raw,
                IdentityLimits::default(),
                &mut budget,
                &cancel,
            );
            assert!(
                budget.reserved > 0,
                "actual full-product reservation must be reached"
            );
            match cause {
                None => assert_eq!(result.unwrap(), expected),
                Some(cause) => {
                    let error =
                        result.expect_err("reservation-phase cancellation must stop the key");
                    if let Some(admission) = &mut budget.admission {
                        assert!(matches!(
                            error,
                            qsl_eval::simulation::model::ModelKeyError::Denied
                        ));
                        assert!(
                            matches!(admission.refusal(error), ProbabilityDocumentError::Cancelled(actual) if actual == cause)
                        );
                    } else {
                        assert!(
                            matches!(error, qsl_eval::simulation::model::ModelKeyError::Cancelled(actual) if actual == cause)
                        );
                    }
                    assert_eq!(cancel.tripped(), Some(cause));
                }
            }
        }
    }
}
