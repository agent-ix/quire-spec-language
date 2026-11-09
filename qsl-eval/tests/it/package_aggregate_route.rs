// SPDX-License-Identifier: AGPL-3.0-or-later
use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    EvidenceCounter, EvidenceIncomplete, EvidencePackage, EvidenceRequest, NonValueKind,
    Settlement, Truth,
};
use qsl_eval::temporal::{AggregateSample, UndefinedCause};
use qsl_foundation::SourceIdentity;
use qsl_package::{
    emit_checked_with_temporal_selections, read_evaluation_emission, CheckedPackage,
    ClockConfiguration, TemporalClockSelection, V2ReadLimits,
};
use qsl_semantics::check::{PackageDeclarations, SCALAR_LIMITS_UNLIMITED};
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::intake::{package_input, SelectedModels};
use quire_exact::{Integer, Meter, Rational, ScalarLimits, Undefined};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::json;

use crate::temporal_activation::{id, origin, position, request, rows_for, unlimited, Env};
use crate::temporal_trace::{fixture, EVENT};

struct EvidenceRoutes<'p> {
    from_emission: EvidencePackage<'p>,
    from_read: EvidencePackage<'p>,
    from_package: EvidencePackage<'p>,
}

fn with_routes<R>(
    body: &str,
    run: impl FnOnce(EvidenceRoutes<'_>, &qsl_package::CheckedNodeId, &Env) -> R,
) -> R {
    let fixture = fixture(EVENT, "e: Int[0, 9]", "on origin", body);
    let env = rows_for(
        fixture,
        EVENT,
        json!({"kind": "event_position", "sequence_authority": "order-events"}),
    )
    .with_clock_name("model-steps");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "aggregate-route"),
        "aggregate-route.native",
        env.fixture.text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the aggregate clause");
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the clause");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        built,
        SelectedModels::default(),
        Vec::new(),
    )
    .expect("the unit assembles");
    let graph = declarations
        .check(
            quire_semantic_value::checking::CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect("the aggregate clause checks");
    let selection = graph
        .temporal_clauses()
        .iter()
        .map(|clause| {
            TemporalClockSelection::new(
                graph.source().clone(),
                clause.clause().span(),
                ClockConfiguration::EventPosition {
                    sequence_authority: "order-events".into(),
                },
            )
        })
        .collect::<Vec<_>>();
    let package = CheckedPackage::link(graph);
    let emission = emit_checked_with_temporal_selections(
        &package,
        &selection,
        qsl_package::EmitLimits::fixture(),
    )
    .expect("the checked package emits");
    let no_domains = package_input(std::iter::empty::<&[u8]>()).expect("empty model closure");
    let read = read_evaluation_emission(
        &package,
        &emission,
        LibraryName::new("fixture").expect("a library name"),
        &no_domains,
        V2ReadLimits::fixture(),
        qsl_package::EmitLimits::fixture(),
        qsl_package::SourceWitnessSupplier::new(&emission),
    )
    .expect("the checked package reader admits the aggregate");
    let clause = read
        .admitted()
        .temporal_clause_names()
        .and_then(|rows| rows.first())
        .expect("one named aggregate clause")
        .declaration
        .clone();
    let from_emission = EvidencePackage::from_emission(
        &emission,
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("emission facts read");
    let from_read = EvidencePackage::from_read(
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("package facts read");
    let bare = qsl_package::BarePackage::new(read.admitted());
    let from_package = EvidencePackage::from_package(
        &bare,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("bare package facts read");
    run(
        EvidenceRoutes {
            from_emission,
            from_read,
            from_package,
        },
        &clause,
        &env,
    )
}

fn with_package<R>(
    body: &str,
    run: impl FnOnce(EvidencePackage<'_>, &qsl_package::CheckedNodeId, &Env) -> R,
) -> R {
    with_routes(body, |routes, clause, env| {
        run(routes.from_read, clause, env)
    })
}

fn asked(env: &Env, samples: &[Option<AggregateSample>]) -> EvidenceRequest {
    let positions = samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let mut position = position(index as u64, &[]);
            if let Some(sample) = sample {
                position.aggregate_samples.insert(0, sample.clone());
            }
            position
        })
        .collect::<Vec<_>>();
    let mut request = request(env, positions);
    request.origins = vec![origin("origin-1", "payload")];
    request.trigger = Some(id("origin-1"));
    request.decision_progress = Some(samples.len().saturating_sub(1) as u64);
    request.surrounding_progress = request.decision_progress;
    request.completeness.as_mut().unwrap().state =
        qsl_eval::temporal::ProgressCompleteness::Complete {
            through: samples.len().saturating_sub(1) as u64,
        };
    request
}

fn values(values: &[i64]) -> Vec<Option<AggregateSample>> {
    values
        .iter()
        .map(|value| {
            Some(AggregateSample::Value(Rational::from_integer(
                Integer::from(*value),
            )))
        })
        .collect()
}

fn assert_truth(body: &str, samples: &[Option<AggregateSample>], expected: Truth) {
    with_package(body, |package, clause, env| {
        let result = package
            .evaluate_temporal_evidence(clause, &asked(env, samples), &mut unlimited())
            .map(|issued| issued.result().clone())
            .expect("the exact aggregate comparison evaluates");
        assert_eq!(result.truth, Some(expected), "{body}");
    });
}

#[trace("FR-516-AC-47", "FR-516-AC-49", "FR-516-AC-50", "FR-532-AC-11")]
#[test]
fn checked_package_aggregate_uses_samples_and_charges_window_reads() {
    with_routes(
        "always[0,3] (sum of e over past[0,2] <= 15)",
        |routes, clause, env| {
            let samples = values(&[3, 4, 5, 6]);
            let request = asked(env, &samples);
            let mut meter = unlimited();
            let result = routes
                .from_emission
                .evaluate_temporal_evidence(clause, &request, &mut meter)
                .map(|issued| issued.result().clone())
                .expect("the aggregate evidence evaluates");
            let charges = meter.admitted_charges().to_vec();
            for package in [&routes.from_read, &routes.from_package] {
                let mut route_meter = unlimited();
                let route_result = package
                    .evaluate_temporal_evidence(clause, &request, &mut route_meter)
                    .map(|issued| issued.result().clone())
                    .expect("the package read route evaluates");
                assert_eq!(route_result.truth, result.truth);
                assert_eq!(route_result.non_value, result.non_value);
                assert_eq!(route_result.incomplete, result.incomplete);
                assert_eq!(route_result.usage, result.usage);
                assert_eq!(route_meter.admitted_charges(), charges);
            }
            assert_eq!(result.truth, Some(Truth::True));
            for (counter, expected) in [
                (EvidenceCounter::FormulaNodes, 2),
                (EvidenceCounter::Instances, 1),
                (EvidenceCounter::Positions, 4),
                (EvidenceCounter::Valuations, 13),
                (EvidenceCounter::EvaluationSteps, 19),
                (EvidenceCounter::Visited, 19),
            ] {
                assert_eq!(result.usage.get(counter), expected);
            }

            let mut missing = samples.clone();
            missing[1] = None;
            let missing = routes
                .from_read
                .evaluate_temporal_evidence(clause, &asked(env, &missing), &mut unlimited())
                .map(|issued| issued.result().clone())
                .expect("an omitted sample is missing evidence");
            assert_eq!(missing.truth, None);
            assert_eq!(
                missing.non_value.as_ref().map(|value| value.kind),
                Some(NonValueKind::Missing)
            );

            let mut undefined = samples.clone();
            undefined[1] = Some(AggregateSample::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            )));
            let undefined = routes
                .from_read
                .evaluate_temporal_evidence(clause, &asked(env, &undefined), &mut unlimited())
                .map(|issued| issued.result().clone())
                .expect("an undefined sample is reported as a non-value");
            assert_eq!(undefined.truth, None);
            assert_eq!(
                undefined.non_value.as_ref().map(|value| value.kind),
                Some(NonValueKind::Refused)
            );
            assert_eq!(
                undefined.non_value.as_ref().map(|value| value.code),
                Some("undefined-atom")
            );

            let mut wrong_kind = asked(env, &samples);
            wrong_kind.positions[0]
                .aggregate_samples
                .insert(0, AggregateSample::Holds(true));
            wrong_kind.limits.valuations = 0;
            let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
            let refused = routes
                .from_read
                .evaluate_temporal_evidence(clause, &wrong_kind, &mut meter)
                .map(|issued| issued.result().clone());
            assert!(matches!(
                refused,
                Err(quire_eval::CallFailure::Admission(_))
            ));
            assert!(meter.admitted_charges().is_empty());

            let stopped = routes
                .from_read
                .with_max_window_values(2)
                .evaluate_temporal_evidence(clause, &asked(env, &samples), &mut unlimited())
                .map(|issued| issued.result().clone())
                .expect("the window ceiling is an incomplete result");
            assert_eq!(stopped.truth, None);
            let Some(EvidenceIncomplete::WindowLimit(limit)) = stopped.incomplete else {
                panic!("the aggregate ceiling has a typed limit result");
            };
            assert_eq!(
                [limit.limit, limit.value, limit.position.index()],
                [2, 3, 2]
            );
            assert_eq!(limit.setting, "monitor.max_window_values");
        },
    );
}

#[trace("FR-516-AC-47", "FR-516-AC-50", "FR-532-AC-11")]
#[test]
fn false_aggregate_result_matches_all_package_routes() {
    with_routes(
        "always[3,3] (sum of e over past[0,2] <= 14)",
        |routes, clause, env| {
            let mut request = asked(env, &values(&[3, 4, 5, 6]));
            request.decision_closure = Some(qsl_eval::temporal::ProgressClosure::Open);
            let mut meter = unlimited();
            let result = routes
                .from_emission
                .evaluate_temporal_evidence(clause, &request, &mut meter)
                .map(|issued| issued.result().clone())
                .expect("the false aggregate evaluates");
            let charges = meter.admitted_charges().to_vec();
            assert_eq!(result.truth, Some(Truth::False));
            assert_eq!(result.settlement, Some(Settlement::DecisiveCounterexample));
            for package in [&routes.from_read, &routes.from_package] {
                let mut route_meter = unlimited();
                let route_result = package
                    .evaluate_temporal_evidence(clause, &request, &mut route_meter)
                    .map(|issued| issued.result().clone())
                    .expect("the package read route evaluates the false result");
                assert_eq!(route_result.truth, result.truth);
                assert_eq!(route_result.settlement, result.settlement);
                assert_eq!(route_result.usage, result.usage);
                assert_eq!(route_result.non_value, result.non_value);
                assert_eq!(route_result.incomplete, result.incomplete);
                assert_eq!(route_meter.admitted_charges(), charges);
            }
        },
    );
}

/// FR-523-AC-9: a denied aggregate-window visit leaves its counters at the
/// successful prefix, including the evaluator's valuation and visit totals.
#[trace("FR-523-AC-9", "FR-516-AC-50", "NFR-008-AC-5")]
#[test]
fn denied_aggregate_window_visit_keeps_counters_at_the_successful_prefix() {
    with_routes(
        "always[0,4] (sum of e over past[0,4] <= 100)",
        |routes, clause, env| {
            let mut meter = Meter::new(ScalarLimits {
                work_units: 12,
                ..SCALAR_LIMITS_UNLIMITED
            });
            let result = routes
                .from_read
                .evaluate_temporal_evidence(
                    clause,
                    &asked(env, &values(&[3, 4, 5, 6, 7])),
                    &mut meter,
                )
                .map(|issued| issued.result().clone())
                .expect("a denied window visit is an incomplete result");

            assert_eq!(result.truth, None);
            assert!(matches!(
                result.incomplete,
                Some(EvidenceIncomplete::Meter { .. })
            ));
            assert_eq!(result.usage.get(EvidenceCounter::Valuations), 17);
            assert_eq!(result.usage.get(EvidenceCounter::EvaluationSteps), 24);
            assert_eq!(result.usage.get(EvidenceCounter::Visited), 24);
        },
    );
}

#[trace("FR-516-AC-49")]
#[test]
fn insufficient_aggregate_data_is_a_missing_non_value() {
    with_package(
        "always[0,3] (sum of e over past[0,2] min_count 2 <= 15)",
        |package, clause, env| {
            let result = package
                .evaluate_temporal_evidence(
                    clause,
                    &asked(env, &values(&[3, 4, 5, 6])),
                    &mut unlimited(),
                )
                .map(|issued| issued.result().clone())
                .expect("insufficient aggregate data is a non-value");
            assert_eq!(result.truth, None);
            let non_value = result.non_value.expect("missing aggregate value");
            assert_eq!(non_value.kind, NonValueKind::Missing);
            assert_eq!(non_value.code, "insufficient-data");
            assert_eq!(non_value.dimension.as_deref(), Some("valuation:0@0"));
            assert_eq!(
                non_value.detail,
                [
                    ("count".to_owned(), "1".to_owned()),
                    ("min_count".to_owned(), "2".to_owned())
                ]
            );
        },
    );
}

#[trace("FR-516-AC-47", "FR-516-AC-48")]
#[test]
fn checked_package_aggregate_kinds_compare_exactly() {
    let number_samples = values(&[3, 4, 5, 6]);
    for body in [
        "always[3,3] (min of e over past[0,2] = 4)",
        "always[3,3] (max of e over past[0,2] = 6)",
        "always[3,3] (quantile 0.5 of e over past[0,2] = 5)",
        "always[3,3] (quantile 0.75 of e over past[0,2] = 6)",
    ] {
        assert_truth(body, &number_samples, Truth::True);
    }

    let counted = [
        AggregateSample::Counted,
        AggregateSample::Counted,
        AggregateSample::Excluded,
        AggregateSample::Counted,
    ]
    .map(Some);
    assert_truth(
        "always[3,3] (count over past[0,2] = 2)",
        &counted,
        Truth::True,
    );

    let fraction = [
        AggregateSample::Holds(true),
        AggregateSample::Holds(false),
        AggregateSample::Holds(true),
        AggregateSample::Excluded,
    ]
    .map(Some);
    assert_truth(
        "always[3,3] (fraction holds(e >= 0) over past[0,3] = 2/3)",
        &fraction,
        Truth::True,
    );
    assert_truth(
        "always[3,3] (sum of e over past[0,2] <= 14)",
        &number_samples,
        Truth::False,
    );
}
