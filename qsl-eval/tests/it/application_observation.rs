// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-090: public owned observations over real checked source functions.

use ix_trace_rs::trace;
use qsl_eval::value::{
    ApplicationObservation, ApplicationObservationOptions, CheckedPackageEvaluation, QualifiedName,
};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::{admit_ieee_profile, DefinitionRole};
use quire_eval::{CallFailure, FamilyOutcome};
use quire_exact::{
    Charge, ChargeAdmission, ChargeEvent, ChargePoint, IeeeValue, Integer, LimitKind, Meter,
    Outcome, ScalarLimits, Value,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::location::{ExpressionLocation as Location, ExpressionOwner as Origin};

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

fn options() -> ApplicationObservationOptions {
    ApplicationObservationOptions {
        function_case_limit: u64::MAX,
        max_events: 20_000,
        max_size_requests: 40_000,
    }
}

fn package(source: &str) -> CheckedPackage {
    package_with(source, |_| Default::default())
}

fn package_with(
    source: &str,
    models: impl FnOnce(&qsl_forms::ParsedUnit) -> qsl_semantics::model::intake::SelectedModels,
) -> CheckedPackage {
    let source = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n{source}");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u"),
        "observation.native",
        source.as_bytes(),
        qsl_cst::Limits::default().with_work_units(1 << 40),
    )
    .expect("source parses");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("source forms");
    let selected = models(&unit);
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("source assembles");
    declarations.ieee_profile = Some(
        admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).expect("IEEE profile"),
    );
    CheckedPackage::link(
        declarations
            .check(
                CheckingLimits::default()
                    .with_nodes(1 << 30)
                    .with_input_bytes(1 << 30)
                    .with_work_budget(1 << 40),
            )
            .expect("source checks"),
    )
}

fn observe(
    package: &CheckedPackage,
    name: &str,
    args: Vec<Value>,
    meter: &mut Meter,
    options: ApplicationObservationOptions,
) -> ApplicationObservation {
    package.call_observing_applications(
        &QualifiedName::unqualified(name).expect("function name"),
        args,
        &ObjectEnvironment::default(),
        meter,
        options,
    )
}

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn sum_package() -> CheckedPackage {
    package("function f using v(a: Integer, b: Integer): Integer pure { a + b }")
}

fn request(point: ChargePoint, sizes: &[(LimitKind, u64)], results: u64) -> ChargeEvent {
    ChargeEvent {
        point,
        work_units: Integer::from(1_u64),
        result_units: Integer::from(results),
        work_reservation: None,
        result_reservation: None,
        sizes: sizes
            .iter()
            .map(|(kind, amount)| (*kind, Integer::from(*amount)))
            .collect(),
        admission: ChargeAdmission::Admitted,
    }
}

fn expected_sum() -> Vec<ChargeEvent> {
    vec![
        request(ChargePoint::FunctionCall, &[], 0),
        request(
            ChargePoint::IntegerArithmeticOperands,
            &[
                (LimitKind::IntegerBits, 3),
                (LimitKind::ValueOccurrences, 2),
            ],
            0,
        ),
        request(
            ChargePoint::IntegerArithmeticArithmetic,
            &[(LimitKind::IntegerBits, 4)],
            0,
        ),
        request(ChargePoint::IntegerArithmeticResultRetain, &[], 1),
    ]
}

fn assert_totals(report: &ApplicationObservation, meter: &Meter) {
    for kind in LimitKind::ALL {
        assert_eq!(
            report.meter_totals.amount(kind),
            meter.consumed(kind),
            "{kind:?}"
        );
    }
}

#[trace("TC-1338", "FR-090-AC-23")]
#[test]
fn completed_call_preserves_actual_outcome_full_requests_and_totals() {
    let package = sum_package();
    let mut meter = Meter::new(UNLIMITED);
    let report = observe(&package, "f", vec![int(3), int(5)], &mut meter, options());
    assert!(
        matches!(report.outcome(), Some(FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value)))) if value == &Integer::from(8_i64))
    );
    let actual = report.evaluation.as_ref().expect("evaluation");
    assert!(std::ptr::eq(
        report.outcome().expect("outcome"),
        &actual.outcome
    ));
    assert_eq!(report.losses(), Some(actual.losses.as_slice()));
    let ledger = report.ledger.as_ref().expect("complete ledger");
    assert_eq!(
        ledger
            .entries()
            .iter()
            .map(|entry| entry.request.clone())
            .collect::<Vec<_>>(),
        expected_sum()
    );
    assert!(ledger
        .entries()
        .iter()
        .all(|entry| entry.location.is_some()));
    assert_eq!(report.observed.len(), 1);
    for kind in LimitKind::ALL {
        assert_eq!(ledger.interval_amounts().amount(kind), meter.consumed(kind));
    }
    assert_totals(&report, &meter);
    assert!(meter.take_charge_capture().is_none());
    assert!(
        matches!(report.into_evaluation().expect("owned result").outcome, FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) if value == Integer::from(8_i64))
    );
}

#[trace("TC-1338", "FR-090-AC-23")]
#[test]
fn incomplete_call_has_only_its_admitted_prefix_and_failed_operation_context() {
    let package = sum_package();
    for (work_units, retained) in [(0, 0), (2, 2)] {
        let mut meter = Meter::new(ScalarLimits {
            work_units,
            ..UNLIMITED
        });
        let report = observe(&package, "f", vec![int(3), int(5)], &mut meter, options());
        assert!(matches!(
            report.outcome(),
            Some(FamilyOutcome::Evaluated(Outcome::Incomplete(_)))
        ));
        let entries = report
            .ledger
            .as_ref()
            .expect("complete admitted prefix")
            .entries();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.request.clone())
                .collect::<Vec<_>>(),
            expected_sum()[..retained]
        );
        assert!(entries.iter().all(|entry| entry.location.is_some()));
        assert!(
            report.observed.is_empty(),
            "failed operation is not a completed application"
        );
        assert_totals(&report, &meter);
        assert!(meter.take_charge_capture().is_none());
    }
}

#[trace("TC-1338", "FR-090-AC-24")]
#[test]
fn separate_whole_call_caps_return_typed_prefix_without_affecting_evaluation() {
    let package = sum_package();
    for (max_events, max_size_requests, retained) in [(2, 20, 2), (20, 1, 1), (20, 2, 2)] {
        let mut meter = Meter::new(UNLIMITED);
        let report = observe(
            &package,
            "f",
            vec![int(3), int(5)],
            &mut meter,
            ApplicationObservationOptions {
                max_events,
                max_size_requests,
                ..options()
            },
        );
        assert!(
            matches!(report.outcome(), Some(FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value)))) if value == &Integer::from(8_i64))
        );
        let partial = report
            .ledger
            .as_ref()
            .expect_err("whole-call capacity exhausted");
        assert_eq!(
            partial
                .admitted_prefix()
                .iter()
                .map(|entry| entry.request.clone())
                .collect::<Vec<_>>(),
            expected_sum()[..retained]
        );
        assert_eq!(
            partial
                .admitted_prefix_amounts()
                .amount(LimitKind::WorkUnits),
            retained as u64
        );
        assert_eq!(report.meter_totals.amount(LimitKind::WorkUnits), 4);
        assert_eq!(report.meter_totals.amount(LimitKind::ResultUnits), 1);
        assert_totals(&report, &meter);
        assert!(meter.take_charge_capture().is_none());
    }
    let mut meter = Meter::new(ScalarLimits {
        work_units: 2,
        ..UNLIMITED
    });
    let report = observe(
        &package,
        "f",
        vec![int(3), int(5)],
        &mut meter,
        ApplicationObservationOptions {
            max_events: 2,
            ..options()
        },
    );
    assert!(
        report.ledger.is_err(),
        "denied third boundary also needs capacity"
    );
    assert!(matches!(
        report.outcome(),
        Some(FamilyOutcome::Evaluated(Outcome::Incomplete(_)))
    ));
}

#[trace("TC-1338", "FR-090-AC-25")]
#[trace("TC-1338", "FR-090-AC-26")]
#[test]
fn displaced_capture_is_intact_and_all_return_paths_leave_capture_disabled() {
    let package = sum_package();
    for prior_truncated in [false, true] {
        for name in ["f", "missing"] {
            let mut meter = Meter::new(UNLIMITED);
            meter.enable_charge_accounting();
            meter.enable_charge_capture(1, 3);
            meter
                .charge(
                    Charge::new(ChargePoint::CollectionVisit)
                        .size(LimitKind::IntegerBits, 7)
                        .size(LimitKind::IntegerBits, 2),
                )
                .expect("precharge");
            if prior_truncated {
                meter
                    .charge(Charge::new(ChargePoint::CollectionVisit))
                    .expect("overflow capture");
            }
            let mut expected_meter = meter.clone();
            let expected = expected_meter.disable_charge_capture();
            let report = observe(&package, name, vec![int(3), int(5)], &mut meter, options());
            assert_eq!(report.displaced_capture, expected);
            if name == "missing" {
                assert!(matches!(report.evaluation, Err(CallFailure::Input(_))));
            }
            assert!(meter.take_charge_capture().is_none());
            // Observation must not drain the caller's existing accounting interval.
            assert_eq!(
                meter
                    .take_charge_accounting()
                    .expect("caller accounting remains active")
                    .amount(LimitKind::WorkUnits),
                meter.consumed(LimitKind::WorkUnits)
            );
            meter.enable_charge_capture(50, 50);
            let next = observe(&package, "f", vec![int(3), int(5)], &mut meter, options());
            assert!(next
                .displaced_capture
                .expect("explicitly enabled")
                .expect("empty capture")
                .events()
                .is_empty());
            assert_eq!(
                next.ledger.expect("fresh complete ledger").entries().len(),
                4
            );
            assert!(meter.take_charge_capture().is_none());
        }
    }
}

#[trace("TC-1338", "FR-090-AC-25")]
#[test]
fn global_highwaters_and_interval_amounts_remain_distinct_on_a_shared_meter() {
    let package = sum_package();
    let mut meter = Meter::new(UNLIMITED);
    meter
        .charge(
            Charge::new(ChargePoint::CollectionVisit)
                .size(LimitKind::IntegerBits, 100)
                .work(Integer::from(17_u64))
                .results(9),
        )
        .expect("precharge");
    for run in 1..=2 {
        let report = observe(&package, "f", vec![int(3), int(5)], &mut meter, options());
        let interval = report.ledger.as_ref().expect("complete").interval_amounts();
        assert_eq!(interval.amount(LimitKind::IntegerBits), 4);
        assert_eq!(interval.amount(LimitKind::WorkUnits), 4);
        assert_eq!(interval.amount(LimitKind::ResultUnits), 1);
        assert_eq!(report.meter_totals.amount(LimitKind::IntegerBits), 100);
        assert_eq!(
            report.meter_totals.amount(LimitKind::WorkUnits),
            17 + run * 4
        );
        assert_eq!(report.meter_totals.amount(LimitKind::ResultUnits), 9 + run);
        assert_totals(&report, &meter);
    }
}

#[trace("TC-1338", "FR-090-AC-24")]
#[test]
fn equality_plan_keeps_actual_increment_and_distinct_reservations() {
    use quire_exact::{
        form_collection, CardinalityBound, CollectionKind, CollectionType, ValueType,
    };

    let package = package("function f using v(a: Sequence<Integer>[0, 2], b: Sequence<Integer>[0, 2]): Boolean pure { a = b }");
    let collection_type = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Integer,
        Some(CardinalityBound::new(0, 2).expect("bound")),
    );
    let argument = form_collection(
        &collection_type,
        vec![int(1), int(2)],
        &mut Meter::new(UNLIMITED),
    )
    .expect("typed sequence")
    .completed()
    .expect("sequence forms");
    let mut meter = Meter::new(UNLIMITED);
    let report = observe(
        &package,
        "f",
        vec![argument.clone(), argument],
        &mut meter,
        options(),
    );
    assert!(matches!(
        report.outcome(),
        Some(FamilyOutcome::Evaluated(Outcome::Completed(
            Value::Boolean(true)
        )))
    ));
    let ledger = report.ledger.as_ref().expect("complete");
    let plan = &ledger
        .entries()
        .iter()
        .find(|row| row.request.point == ChargePoint::EqualityPlan)
        .expect("plan boundary")
        .request;
    assert_eq!(plan.work_units, Integer::from(1_u64));
    assert_eq!(plan.result_units, Integer::from(0_u64));
    assert_eq!(plan.work_reservation, Some(Integer::from(5_u64)));
    assert_eq!(plan.result_reservation, Some(Integer::from(1_u64)));
    assert_eq!(
        plan.sizes,
        [(LimitKind::ValueOccurrences, Integer::from(3_u64))]
    );
    for kind in LimitKind::ALL {
        assert_eq!(ledger.interval_amounts().amount(kind), meter.consumed(kind));
    }
}

#[trace("TC-1338", "FR-090-AC-23")]
#[test]
fn opaque_ieee_conversion_retains_losses_and_typed_refusal() {
    let arguments = || vec![Value::Float(IeeeValue::binary64(0x47ef_ffff_f000_0000))];
    for mode in ["toward-zero", "exact"] {
        let package = package(&format!("function f using v(a: Float64[nearest-even]): Float32[{mode}] pure {{ convert<Float32[{mode}]>(a) }}"));
        let mut meter = Meter::new(UNLIMITED);
        let report = observe(&package, "f", arguments(), &mut meter, options());
        if mode == "exact" {
            assert!(matches!(
                report.outcome(),
                Some(FamilyOutcome::Evaluated(Outcome::Refused(_)))
            ));
        } else {
            assert!(matches!(
                report.outcome(),
                Some(FamilyOutcome::Evaluated(Outcome::Completed(_)))
            ));
            assert!(!report.losses().expect("actual losses").is_empty());
        }
        let ordinary = package
            .call(
                &QualifiedName::unqualified("f").expect("name"),
                arguments(),
                &ObjectEnvironment::default(),
                &mut Meter::new(UNLIMITED),
            )
            .expect("ordinary evaluation");
        assert_eq!(
            format!("{:?}", report.outcome().expect("actual outcome")),
            format!("{:?}", ordinary.outcome)
        );
        assert_eq!(report.losses(), Some(ordinary.losses.as_slice()));
        let requests: Vec<_> = report
            .ledger
            .as_ref()
            .expect("full opaque ledger")
            .entries()
            .iter()
            .map(|row| row.request.clone())
            .collect();
        let mut expected = vec![
            request(ChargePoint::FunctionCall, &[], 0),
            request(
                ChargePoint::IeeeOperands,
                &[
                    (LimitKind::IntegerBits, 64),
                    (LimitKind::ValueOccurrences, 1),
                ],
                0,
            ),
            request(
                ChargePoint::IeeeExactIntermediate,
                &[(LimitKind::IntegerBits, 32)],
                0,
            ),
            request(ChargePoint::IeeeRound, &[(LimitKind::IntegerBits, 32)], 0),
        ];
        if mode != "exact" {
            expected.push(request(ChargePoint::IeeeResultRetain, &[], 1));
        }
        assert_eq!(
            requests, expected,
            "opaque kernel boundaries retain their full payload"
        );
        assert_totals(&report, &meter);
    }
}

#[trace("TC-1338", "FR-090-AC-24")]
#[test]
fn capture_has_no_diagnostic_4096_event_ceiling() {
    let elements = vec!["1"; 4200].join(", ");
    let package = package(&format!(
        "function f using v(): Sequence<Integer>[0, 5000] pure {{ sequence[{elements}] }}"
    ));
    let mut meter = Meter::new(UNLIMITED);
    let report = observe(&package, "f", Vec::new(), &mut meter, options());
    assert!(matches!(
        report.outcome(),
        Some(FamilyOutcome::Evaluated(Outcome::Completed(_)))
    ));
    let ledger = report.ledger.as_ref().expect("explicit capacities suffice");
    assert!(ledger.entries().len() > 4096);
    assert_eq!(
        ledger
            .entries()
            .iter()
            .filter(|row| row.request.point == ChargePoint::CollectionElement)
            .count(),
        4200
    );
    for kind in LimitKind::ALL {
        assert_eq!(ledger.interval_amounts().amount(kind), meter.consumed(kind));
    }
}

#[trace("TC-1338", "FR-090-AC-24")]
#[test]
fn nested_calls_preserve_completed_values_counts_and_whole_call_capacity() {
    let package = package("function double using v(x: Integer): Integer pure { x + x }\nfunction f using v(x: Integer): Integer pure { 100 + double(x) }");
    for function_case_limit in [0, 1, 2] {
        let mut meter = Meter::new(UNLIMITED);
        let report = observe(
            &package,
            "f",
            vec![int(3)],
            &mut meter,
            ApplicationObservationOptions {
                function_case_limit,
                ..options()
            },
        );
        assert!(!report.unavailable_imported_call);
        assert_eq!(report.function_case_limit_reached, function_case_limit < 2);
        assert_eq!(report.function_evaluations, function_case_limit);
        if function_case_limit == 2 {
            assert!(
                matches!(report.outcome(), Some(FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value)))) if value == &Integer::from(106_i64))
            );
            assert_eq!(
                report
                    .observed
                    .iter()
                    .map(|(_, value)| match value {
                        Value::Integer(value) => value.to_string(),
                        other => panic!("integer result: {other:?}"),
                    })
                    .collect::<Vec<_>>(),
                ["6", "6", "106"]
            );
        } else {
            assert!(matches!(
                report.outcome(),
                Some(FamilyOutcome::Evaluated(Outcome::Incomplete(_)))
            ));
            assert!(report.observed.is_empty());
        }
        let ledger = report
            .ledger
            .as_ref()
            .expect("complete ledger despite case limit");
        for kind in LimitKind::ALL {
            assert_eq!(ledger.interval_amounts().amount(kind), meter.consumed(kind));
        }
    }
    let mut meter = Meter::new(UNLIMITED);
    let report = observe(
        &package,
        "f",
        vec![int(3)],
        &mut meter,
        ApplicationObservationOptions {
            max_events: 3,
            ..options()
        },
    );
    let partial = report
        .ledger
        .as_ref()
        .expect_err("nested steps share one capacity");
    assert_eq!(partial.admitted_prefix().len(), 3);
    assert!(
        matches!(report.outcome(), Some(FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value)))) if value == &Integer::from(106_i64))
    );
}

#[trace("TC-1338", "FR-090-AC-23")]
#[test]
fn nested_and_failed_charge_locations_match_the_authored_operation_paths() {
    // The declaration indices and expression paths are fixed by this source:
    // double's addition is its root; f's call to double is addition child 1.
    // These expected locations do not read the ledger or machine context.
    let package = package("function double using v(x: Integer): Integer pure { x + x }\nfunction f using v(x: Integer): Integer pure { 100 + double(x) }");
    let double_add = Location::root(Origin::Body {
        function: "double".to_owned(),
        index: 0,
    });
    let outer_add = Location::root(Origin::Body {
        function: "f".to_owned(),
        index: 1,
    });
    let nested_call = Location::at(
        Origin::Body {
            function: "f".to_owned(),
            index: 1,
        },
        &[1],
    );
    assert_ne!(double_add, outer_add);
    assert_ne!(nested_call, outer_add);
    let expected = [
        (ChargePoint::FunctionCall, Some(outer_add.clone())),
        (ChargePoint::FunctionCall, Some(nested_call)),
        (
            ChargePoint::IntegerArithmeticOperands,
            Some(double_add.clone()),
        ),
        (
            ChargePoint::IntegerArithmeticArithmetic,
            Some(double_add.clone()),
        ),
        (
            ChargePoint::IntegerArithmeticResultRetain,
            Some(double_add.clone()),
        ),
        (
            ChargePoint::IntegerArithmeticOperands,
            Some(outer_add.clone()),
        ),
        (
            ChargePoint::IntegerArithmeticArithmetic,
            Some(outer_add.clone()),
        ),
        (ChargePoint::IntegerArithmeticResultRetain, Some(outer_add)),
    ];
    for (work_units, retained) in [(u64::MAX, 8), (3, 3)] {
        let mut meter = Meter::new(ScalarLimits {
            work_units,
            ..UNLIMITED
        });
        let report = observe(&package, "f", vec![int(3)], &mut meter, options());
        let ledger = report.ledger.as_ref().expect("complete admitted ledger");
        let actual: Vec<_> = ledger
            .entries()
            .iter()
            .map(|entry| (entry.request.point, entry.location.clone()))
            .collect();
        assert_eq!(
            actual,
            expected[..retained],
            "each charge names its authored operation, including the halted nested addition"
        );
        if retained == 3 {
            let evaluation = report.evaluation.as_ref().expect("incomplete evaluation");
            assert!(
                matches!(&evaluation.outcome, FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) if incomplete.charge_point == ChargePoint::IntegerArithmeticArithmetic)
            );
            assert_eq!(evaluation.location.as_ref(), Some(&double_add));
            assert!(
                report.observed.is_empty(),
                "the halted addition produces no completed-application evidence"
            );
        } else {
            assert!(
                matches!(report.outcome(), Some(FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value)))) if value == &Integer::from(106_i64))
            );
        }
    }
}

#[trace("TC-1338", "FR-090-AC-23")]
#[test]
fn family_refusal_is_owned_without_projecting_its_cause() {
    use crate::tc235_k2 as k2;
    use qsl_semantics::model::accounting::ModelNormalizationLimits;
    use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
    use quire_checked::key::DeclarationKey;
    use quire_eval::FamilyResult;
    use quire_exact::{ObjectId, ObjectReference};
    use serde_json::json;

    let document = k2::domain_document(
        vec![k2::declaration(
            "Obj",
            "thing",
            json!({
                "identityFields": [k2::id("Obj/n")],
                "fields": [k2::field("Obj", "n", "ix://quire/native/Integer")],
            }),
        )],
        vec![k2::construct("thing", meaning::OBJECT_TYPE)],
    );
    let bytes = serde_json::to_vec(&document).expect("model document");
    let packages = package_input([bytes.as_slice()]).expect("model input");
    let mut reference = None;
    let package = package_with(
        "profile m = \"quire.model.complete/v1\";\nmodel M = \"test/orders\";\nfunction f using m(a: M::Obj): Integer pure { deref(a).n }",
        |unit| {
            let selected = admit_unit(
                &unit.selections().models,
                &packages,
                ModelNormalizationLimits::default(),
            )
            .expect("model selection");
            let view = &selected[0].view;
            let object_type = view
                .type_identities()
                .get(&DeclarationKey {
                    package: k2::PACKAGE.to_owned(),
                    node: k2::id("Obj"),
                })
                .expect("selected object type")
                .clone();
            reference = Some(ObjectReference::new(
                view.object_universe()
                    .identity()
                    .expect("universe identity"),
                object_type,
                ObjectId::new("missing").expect("object id"),
            ));
            selected
        },
    );
    let objects = ObjectEnvironment::default().with_unresolved_references();
    let mut meter = Meter::new(UNLIMITED);
    let report = package.call_observing_applications(
        &QualifiedName::unqualified("f").expect("name"),
        vec![Value::Reference(
            reference.expect("reference to the selected model"),
        )],
        &objects,
        &mut meter,
        options(),
    );
    let Some(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))) = report.outcome()
    else {
        panic!("family refusal: {:?}", report.evaluation);
    };
    assert_eq!(
        cause.catalog_code(),
        quire_semantic_value::catalog::CatalogCode::new(
            "unknown_required_feature",
            "unsupported-feature"
        )
    );
    assert_eq!(
        cause
            .catalog_fields()
            .expect("typed refusal fields")
            .get("construct")
            .map(String::as_str),
        Some("dereference")
    );
    let ledger = report.ledger.as_ref().expect("full family-refusal ledger");
    assert_eq!(
        ledger
            .entries()
            .iter()
            .map(|entry| entry.request.clone())
            .collect::<Vec<_>>(),
        [request(ChargePoint::FunctionCall, &[], 0)]
    );
    assert!(report.observed.is_empty());
    assert_totals(&report, &meter);
    assert!(matches!(
        report.into_evaluation().expect("actual result").outcome,
        FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))
    ));
}
