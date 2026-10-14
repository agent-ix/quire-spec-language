// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-277-AC-4, FR-277-AC-5, FR-321-AC-5 (TC-830, TC-906): the supplied-value
//! admission phase of the public checked call. A caller-owned cumulative
//! budget A (`supplied.admission_work_units`) is spent one QSV
//! supplied-membership event at a time, before the call, and a stop is the
//! phase-marked pre-call record with zero evaluation consumption.
//!
//! The expected event sequences below are enumerated here from the
//! membership event order of QSV FR-109 (visit the root, compare the
//! collection's kind and bound, compare its element type chain, schedule
//! each child, then visit each child), never read back from the code under
//! test.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use ix_trace_rs::trace;
use qsl_eval::value::{CallFailure, CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::supplied::{
    AttemptedCount, SuppliedCause, SuppliedEvent, SuppliedFailure, SuppliedLimitKind,
    SuppliedLimits, SuppliedLocus, SuppliedPhase, SuppliedRun, SuppliedStop, ADMISSION_WORK_UNITS,
    CONVERSION_WORK_UNITS, DEFAULT_SUPPLIED_WORK_UNITS,
};
use quire_exact::{
    form, Cancel, CancelCause, Decimal, Integer, Meter, Outcome, ScalarLimits, Value, ValueType,
};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::supplied_membership::{
    InvalidMembershipCause, MembershipPart, PathStep, UnsupportedHelperFamily,
};

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

const UNIT: &str = "function seq using v(xs: Sequence<Boolean>[0, 64]): Boolean pure { false }\n\
     function pair using v(a: Sequence<Boolean>[0, 64], b: Sequence<Boolean>[0, 64]): Boolean pure { false }\n\
     function digit using v(x: Int[0, 9]): Boolean pure { false }\n\
     function money using v(d: Decimal[0, 100; 0, 2; exact]): Boolean pure { false }\n\
     function digits using v(xs: Sequence<Int[0, 9]>[0, 8]): Boolean pure { false }\n";

fn package() -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{UNIT}"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler builds the package declarations");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the package checks"),
    )
}

fn name(function: &str) -> QualifiedName {
    QualifiedName::unqualified(function).expect("an identifier")
}

/// A `Sequence<Boolean>` of `count` elements, typed as `function`'s
/// parameter `index`.
fn booleans(package: &CheckedPackage, function: &str, index: usize, count: usize) -> Value {
    let callable = package.graph().callable(function).expect("declared");
    let ValueType::Collection(declared) = &callable.parameters[index].1 else {
        panic!("a collection parameter");
    };
    let mut meter = Meter::new(UNLIMITED);
    match form(declared, vec![Value::Boolean(true); count], &mut meter) {
        Outcome::Completed(value) => value,
        other => panic!("the sequence forms: {other:?}"),
    }
}

/// The QSV membership events admitting a `Sequence<Boolean>` of `count`
/// elements spends, in order: the root visit, the collection's kind-and-bound
/// pair, the element type chain's one pair, one scheduling per child, then
/// one visit per child.
fn admission_events(count: usize) -> Vec<SuppliedEvent> {
    let mut events = vec![
        SuppliedEvent::ValueVisit,
        SuppliedEvent::TypeComparison,
        SuppliedEvent::TypeComparison,
    ];
    events.extend(std::iter::repeat_n(SuppliedEvent::ChildScheduling, count));
    events.extend(std::iter::repeat_n(SuppliedEvent::ValueVisit, count));
    events
}

fn limits(admission: u64) -> SuppliedLimits {
    SuppliedLimits::default().with_admission_work_units(admission)
}

/// One `call_supplied` of `function` under `limits` and `cancel`.
fn call(
    package: &CheckedPackage,
    function: &str,
    arguments: Vec<Value>,
    limits: SuppliedLimits,
    cancel: Cancel,
) -> (
    Result<qsl_eval::value::Evaluation, CallFailure>,
    Meter,
    SuppliedRun,
) {
    let mut meter = Meter::new(UNLIMITED);
    let mut run = SuppliedRun::new(limits, cancel).expect("the run");
    let result = package.call_supplied(
        &name(function),
        arguments,
        &ObjectEnvironment::default(),
        &mut meter,
        &mut run,
    );
    (result, meter, run)
}

fn stop_of(failure: Result<qsl_eval::value::Evaluation, CallFailure>) -> SuppliedStop {
    match failure {
        Err(CallFailure::Supplied(stop)) => *stop,
        other => panic!("a supplied-phase stop, got {other:?}"),
    }
}

/// The run's full record of its first stop: the phase context that
/// accompanies the invalid-input and cancellation failures, which keep
/// their own arms of the checked call's error.
fn recorded(run: &SuppliedRun) -> &SuppliedStop {
    match run.failure() {
        Some(SuppliedFailure::Stop(stop)) => stop,
        other => panic!("a recorded supplied-phase stop, got {other:?}"),
    }
}

/// No evaluation ran: the meter is untouched, so there is no `FunctionCall`
/// charge and no semantic consumption of any counter.
fn assert_no_evaluation(meter: &Meter) {
    assert_eq!(meter.admission_count(), 0, "no charge was admitted");
    for kind in quire_exact::LimitKind::ALL {
        assert_eq!(meter.consumed(kind), 0, "{kind:?} consumed nothing");
    }
}

/// FR-277: the published defaults are 16,777,216 each, named as the setting
/// names FR-277 gives, independent of each other.
#[trace("TC-830", "FR-277-AC-4")]
#[test]
fn the_helper_ceilings_default_to_16777216_and_are_independent() {
    let defaults = SuppliedLimits::default();
    assert_eq!(DEFAULT_SUPPLIED_WORK_UNITS, 16_777_216);
    assert_eq!(defaults.admission_work_units(), 16_777_216);
    assert_eq!(defaults.conversion_work_units(), 16_777_216);
    assert_eq!(ADMISSION_WORK_UNITS, "supplied.admission_work_units");
    assert_eq!(CONVERSION_WORK_UNITS, "supplied.conversion_work_units");
    let raised = defaults.with_admission_work_units(5);
    assert_eq!(raised.admission_work_units(), 5);
    assert_eq!(raised.conversion_work_units(), 16_777_216);
    let lowered = defaults.with_conversion_work_units(7);
    assert_eq!(lowered.admission_work_units(), 16_777_216);
    assert_eq!(lowered.conversion_work_units(), 7);
}

/// FR-277-AC-4, FR-321-AC-5: A-1 denies the actual next event unspent with
/// the phase, parameter, event, setting, ceiling, successful spend and
/// denied amount; A admits. Zero denies the first event. Neither touches the
/// evaluator's meter.
#[trace("TC-830", "FR-277-AC-4", "FR-321-AC-5")]
#[test]
fn one_less_than_the_event_sequence_denies_its_next_event_and_exact_admits() {
    let package = package();
    for count in [0_usize, 1, 5] {
        let events = admission_events(count);
        let exact = u64::try_from(events.len()).unwrap();
        let argument = booleans(&package, "seq", 0, count);

        let (admitted, meter, run) = call(
            &package,
            "seq",
            vec![argument.clone()],
            limits(exact),
            Cancel::new(),
        );
        let evaluation = admitted.expect("the exact ceiling admits");
        assert!(matches!(
            evaluation.outcome,
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
        ));
        assert_eq!(run.admission_spent(), exact, "{count} elements spend A");
        assert_eq!(run.conversion_spent(), 0, "admission spends no H");
        assert!(meter.admission_count() > 0, "the admitted call evaluated");

        for denied in [0, exact - 1] {
            let (result, meter, run) = call(
                &package,
                "seq",
                vec![argument.clone()],
                limits(denied),
                Cancel::new(),
            );
            let stop = stop_of(result);
            let at = usize::try_from(denied).unwrap();
            assert_eq!(stop.context.phase, SuppliedPhase::Admission);
            assert_eq!(stop.context.parameter, Some(0));
            assert_eq!(stop.context.event, Some(events[at]), "{count}/{denied}");
            assert_eq!(stop.context.consumed, denied, "the denied step is unspent");
            let limit = stop.limit().expect("a limit stop");
            assert_eq!(limit.limit_kind, SuppliedLimitKind::AdmissionWorkUnits);
            assert_eq!(limit.field(), "supplied.admission_work_units");
            assert_eq!(limit.limit, denied);
            assert_eq!(limit.next_charge, 1);
            assert_eq!(limit.counter, AttemptedCount::Count(denied + 1));
            assert_eq!(run.admission_spent(), denied);
            assert_no_evaluation(&meter);
        }
    }
}

/// FR-277-AC-4: the stop's locus is the producer's rooted path: the
/// argument, the child edges from the denied occurrence back to it, and the
/// local part of the event.
#[trace("TC-830", "FR-277-AC-4")]
#[test]
fn a_denied_event_names_its_rooted_locus() {
    let package = package();
    let argument = booleans(&package, "seq", 0, 3);
    let exact = u64::try_from(admission_events(3).len()).unwrap();
    // The last event visits the third element.
    let (result, _, _) = call(
        &package,
        "seq",
        vec![argument],
        limits(exact - 1),
        Cancel::new(),
    );
    let stop = stop_of(result);
    assert_eq!(
        stop.context.locus,
        Some(SuppliedLocus::Membership {
            argument: 0,
            steps_from_leaf: vec![PathStep::CollectionElement(2)],
            part: MembershipPart::Value,
        })
    );
}

/// FR-277-AC-5: one cumulative A across arguments: the second argument
/// spends on top of the first and is the one denied, naming parameter 1.
#[trace("TC-830", "FR-277-AC-5")]
#[test]
fn the_admission_budget_is_cumulative_across_arguments() {
    let package = package();
    let (first, second) = (2_usize, 4_usize);
    let spent_first = u64::try_from(admission_events(first).len()).unwrap();
    let events_second = admission_events(second);
    let total = spent_first + u64::try_from(events_second.len()).unwrap();
    let arguments = vec![
        booleans(&package, "pair", 0, first),
        booleans(&package, "pair", 1, second),
    ];

    let (admitted, _, run) = call(
        &package,
        "pair",
        arguments.clone(),
        limits(total),
        Cancel::new(),
    );
    admitted.expect("the summed ceiling admits both");
    assert_eq!(run.admission_spent(), total);

    let (result, meter, run) = call(
        &package,
        "pair",
        arguments,
        limits(total - 1),
        Cancel::new(),
    );
    let stop = stop_of(result);
    assert_eq!(
        stop.context.parameter,
        Some(1),
        "the later argument is denied"
    );
    assert_eq!(
        stop.context.consumed,
        total - 1,
        "no reset between arguments"
    );
    assert_eq!(
        stop.context.event,
        events_second.last().copied(),
        "the last event of the second argument"
    );
    assert_eq!(run.admission_spent(), total - 1);
    assert_no_evaluation(&meter);
}

/// FR-277: H and A are two budgets: a generous H does not rescue an
/// admission ceiling, and the admission phase spends only A.
#[trace("TC-830", "FR-277-AC-4")]
#[test]
fn the_conversion_ceiling_does_not_pay_for_admission() {
    let package = package();
    let argument = booleans(&package, "seq", 0, 2);
    let exact = u64::try_from(admission_events(2).len()).unwrap();
    let generous = SuppliedLimits::default()
        .with_admission_work_units(exact - 1)
        .with_conversion_work_units(u64::MAX);
    let (result, _, run) = call(&package, "seq", vec![argument], generous, Cancel::new());
    let stop = stop_of(result);
    assert_eq!(stop.context.phase, SuppliedPhase::Admission);
    assert_eq!(run.conversion_spent(), 0);
}

/// FR-277-AC-5: the original handle reaches the first helper: a handle
/// cancelled before admission stops it with its original cause, as a
/// cancellation and not as a limit, in each cause.
#[trace("TC-830", "FR-277-AC-5")]
#[test]
fn a_handle_cancelled_before_admission_stops_it_with_its_original_cause() {
    let package = package();
    let argument = booleans(&package, "seq", 0, 2);
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let cancel = Cancel::new();
        cancel.cancel(cause);
        let (result, meter, run) = call(
            &package,
            "seq",
            vec![argument.clone()],
            SuppliedLimits::default(),
            cancel,
        );
        assert!(
            matches!(result, Err(CallFailure::Cancelled(original)) if original == cause),
            "the existing cancelled failure with its original cause: {result:?}"
        );
        let stop = recorded(&run);
        assert_eq!(stop.cause, SuppliedCause::Cancelled(cause));
        assert!(stop.limit().is_none(), "a cancellation is not a limit");
        assert_eq!(stop.context.phase, SuppliedPhase::Admission);
        assert_eq!(stop.context.parameter, Some(0));
        assert_eq!(stop.context.event, Some(SuppliedEvent::ValueVisit));
        assert_eq!(stop.context.consumed, 0);
        assert_eq!(run.admission_spent(), 0);
        assert_no_evaluation(&meter);
    }
}

/// A handle that cancels itself with `cause` at its `at`-th poll, and the
/// count of polls the handle has seen.
fn cancelling_at(at: u64, cause: CancelCause) -> (Cancel, Arc<AtomicU64>) {
    let slot: Arc<OnceLock<Cancel>> = Arc::new(OnceLock::new());
    let polls = Arc::new(AtomicU64::new(0));
    let (watch_slot, watch_polls) = (Arc::clone(&slot), Arc::clone(&polls));
    let cancel = Cancel::observing(move || {
        let seen = watch_polls.fetch_add(1, Ordering::Relaxed) + 1;
        if seen == at {
            if let Some(handle) = watch_slot.get() {
                handle.cancel(cause);
            }
        }
    });
    slot.set(cancel.clone()).expect("set once");
    (cancel, polls)
}

/// FR-277-AC-5: cancellation at a later argument remains cancellation with
/// its original cause and the cumulative spend, never a limit and never a
/// fresh handle: the handle that stops argument 1 is the one that served
/// argument 0.
#[trace("TC-830", "FR-277-AC-5")]
#[test]
fn a_cancellation_during_a_later_argument_keeps_its_cause_and_the_spend() {
    let package = package();
    let arguments = vec![
        booleans(&package, "pair", 0, 2),
        booleans(&package, "pair", 1, 2),
    ];
    // Polls the handle sees admitting the first argument alone, measured
    // through the handle itself.
    let (counting, polls) = cancelling_at(u64::MAX, CancelCause::Requested);
    let (_, _, _) = call(
        &package,
        "seq",
        vec![booleans(&package, "seq", 0, 2)],
        SuppliedLimits::default(),
        counting,
    );
    let first_argument_polls = polls.load(Ordering::Relaxed);
    assert!(first_argument_polls > 0);

    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let (cancel, _) = cancelling_at(first_argument_polls + 2, cause);
        let (result, meter, run) = call(
            &package,
            "pair",
            arguments.clone(),
            SuppliedLimits::default(),
            cancel,
        );
        assert!(
            matches!(result, Err(CallFailure::Cancelled(original)) if original == cause),
            "the existing cancelled failure with its original cause: {result:?}"
        );
        let stop = recorded(&run);
        assert_eq!(stop.cause, SuppliedCause::Cancelled(cause));
        assert_eq!(stop.context.parameter, Some(1));
        assert!(
            stop.context.consumed >= u64::try_from(admission_events(2).len()).unwrap(),
            "the first argument's spend is kept: {}",
            stop.context.consumed
        );
        assert_eq!(stop.context.consumed, run.admission_spent());
        assert_no_evaluation(&meter);
    }
}

/// FR-277-AC-4: a value that is not a member of its parameter's type
/// retains the original `WrongValueKind` refusal and parameter, with the
/// phase context; it is neither a limit nor a failed predicate.
#[trace("TC-830", "FR-277-AC-4")]
#[test]
fn invalid_input_keeps_its_refusal_and_gains_the_phase_context() {
    let package = package();
    let (result, meter, run) = call(
        &package,
        "seq",
        vec![Value::Boolean(true)],
        SuppliedLimits::default(),
        Cancel::new(),
    );
    assert!(
        matches!(
            result,
            Err(CallFailure::Input(InputRefusal::WrongValueKind {
                parameter: 0
            }))
        ),
        "the existing input refusal: {result:?}"
    );
    let stop = recorded(&run);
    assert_eq!(
        stop.cause,
        SuppliedCause::Input {
            refusal: InputRefusal::WrongValueKind { parameter: 0 },
            membership: Some(InvalidMembershipCause::Kind),
        }
    );
    assert_eq!(stop.context.phase, SuppliedPhase::Admission);
    assert_eq!(stop.context.parameter, Some(0));
    assert_eq!(stop.context.event, Some(SuppliedEvent::ValueVisit));
    assert_eq!(
        stop.context.consumed, 1,
        "the visit that found the fault was a successful event"
    );
    assert_eq!(run.admission_spent(), 1);
    assert_no_evaluation(&meter);
}

/// FR-277-AC-4: the public `call` is `call_supplied` at the default
/// ceilings: it admits what the exact ceiling admits.
#[trace("TC-830", "FR-277-AC-4")]
#[test]
fn the_plain_call_runs_at_the_default_ceilings() {
    let package = package();
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &name("seq"),
            vec![booleans(&package, "seq", 0, 3)],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the default ceilings admit it");
    assert!(matches!(
        evaluation.outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
}

/// IR-714: a helper family QSV has not qualified for supplied admission is
/// a distinct typed stop before that helper runs, never invalid input and
/// never a limit. One vector per family reachable from source.
#[trace("TC-830", "FR-277-AC-6")]
#[test]
fn a_run_refusing_unqualified_families_stops_before_the_helper() {
    let package = package();
    let numeric = Value::Integer(Integer::from(3_i64));
    let decimal = Value::Decimal(Decimal::new(Integer::from(150_i64), 2));
    for (function, argument, family) in [
        ("digit", numeric, UnsupportedHelperFamily::NumericDomain),
        ("money", decimal, UnsupportedHelperFamily::DecimalDomain),
    ] {
        let mut meter = Meter::new(UNLIMITED);
        let mut run = SuppliedRun::new(SuppliedLimits::default(), Cancel::new())
            .expect("the run")
            .refusing_unqualified_families();
        let result = package.call_supplied(
            &name(function),
            vec![argument],
            &ObjectEnvironment::default(),
            &mut meter,
            &mut run,
        );
        let Err(CallFailure::UnsupportedHelper(stop)) = result else {
            panic!("{function}: an unsupported-helper stop, got {result:?}");
        };
        assert_eq!(stop.family, family);
        assert_eq!(stop.context.phase, SuppliedPhase::Admission);
        assert_eq!(stop.context.parameter, Some(0));
        assert_no_evaluation(&meter);
        assert!(run.unbudgeted_families().is_empty());
    }
}

/// The default run keeps a family QSV cannot yet bound working: the
/// argument is admitted by the unchanged unbudgeted membership, and the
/// family is listed so the gap is visible, not hidden.
#[trace("TC-830", "FR-277-AC-6")]
#[test]
fn the_default_run_admits_an_unqualified_family_unbudgeted_and_lists_it() {
    let package = package();
    let mut meter = Meter::new(UNLIMITED);
    let mut run = SuppliedRun::new(SuppliedLimits::default(), Cancel::new()).expect("the run");
    let evaluation = package
        .call_supplied(
            &name("digit"),
            vec![Value::Integer(Integer::from(3_i64))],
            &ObjectEnvironment::default(),
            &mut meter,
            &mut run,
        )
        .expect("a digit is admitted");
    assert!(matches!(
        evaluation.outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
    assert_eq!(
        run.unbudgeted_families(),
        [UnsupportedHelperFamily::NumericDomain]
    );
    // A wrong value is still the original refusal under that path.
    let mut meter = Meter::new(UNLIMITED);
    let mut run = SuppliedRun::new(SuppliedLimits::default(), Cancel::new()).expect("the run");
    let refused = package.call_supplied(
        &name("digit"),
        vec![Value::Integer(Integer::from(30_i64))],
        &ObjectEnvironment::default(),
        &mut meter,
        &mut run,
    );
    assert!(matches!(
        refused,
        Err(CallFailure::Input(InputRefusal::WrongValueKind {
            parameter: 0
        }))
    ));
    assert!(matches!(
        recorded(&run).cause,
        SuppliedCause::Input {
            refusal: InputRefusal::WrongValueKind { parameter: 0 },
            ..
        }
    ));
}
