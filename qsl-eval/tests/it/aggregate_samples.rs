// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-194, FR-327-AC-7 (TC-629, TC-847): an aggregate atom the evaluator
//! plans from a checked clause, evaluated over the per-position samples the
//! observation owner supplies (FR-516 "Window aggregate atoms"): its kind,
//! window, `min_count` and comparison are the checked term's, and the
//! values of FR-516-AC-47 and AC-48 give the same results.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::ClauseFacts;
use qsl_eval::temporal::{
    evaluate, evaluate_finite_window, AggregateAtom, AggregateSample, AtomReading, ClockCover,
    InsufficientData, LetterStop, LowerBoundary, ProgressClosure, ProgressCompleteness,
    SampledWindow, TemporalLetters, TemporalOutcome, TemporalPosition, TemporalRefusal,
    TemporalTrace, TimestampedObservation, TimestampedTrace, UndefinedCause, Valuation, Watermark,
    WindowClock, WindowProgress, WindowTrace, DEFAULT_MAX_WINDOW_VALUES,
};
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Integer, Meter, Rational};

use crate::temporal_trace::{fixture, EVENT};

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("a nonzero divisor")
}

/// The reading of `term` compared with `threshold` at `position` over
/// `samples`, the term written as an aggregate atom in a clause body.
fn read(
    term: &str,
    threshold: Rational,
    samples: &[AggregateSample],
    position: u64,
) -> AtomReading {
    let fx = fixture(
        EVENT,
        "c: Counter",
        "on origin",
        &format!("always[0,3] ({term})"),
    );
    let (_, checked) = fx
        .clause
        .aggregates()
        .iter()
        .next()
        .expect("the clause has one aggregate atom");
    let atom = AggregateAtom::of_checked(checked, threshold).expect("a well-formed term");
    assert!(
        samples.iter().all(|sample| sample.is_taken_by(atom.kind())),
        "{term}"
    );
    let len = u64::try_from(samples.len()).expect("small");
    atom.evaluate(
        &WindowTrace::positions(len),
        &mut SampledWindow::new(samples, atom.kind()),
        DEFAULT_MAX_WINDOW_VALUES,
        TemporalPosition::new(position),
        &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
    )
    .expect("the window reads")
}

/// The checked clause `C` of a timestamped-event unit over `c: Quantity<ms>`
/// with `body` as its formula, through the whole package check (the family
/// alone cannot resolve the unit `ms`), handed to `inspect`.
fn timestamped<R>(
    body: &str,
    inspect: impl FnOnce(&qsl_semantics::check::CheckedTemporalClause) -> R,
) -> R {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t0 = \"quire.temporal.timestamped-event.finite-window/v1\";\n\
         dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         temporal C using t0 over (c: Quantity<ms>) clock \"model-steps\" on origin {{ {body} }}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let graph = qsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        built,
        qsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(quire_semantic_value::checking::CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("{body}: {refusals:?}"));
    let [node] = graph.temporal_clauses() else {
        panic!("one clause");
    };
    inspect(node.clause())
}

fn values(values: &[i64]) -> Vec<AggregateSample> {
    values
        .iter()
        .map(|value| AggregateSample::Value(Rational::from_integer(Integer::from(*value))))
        .collect()
}

fn truth(reading: &AtomReading) -> bool {
    match reading {
        AtomReading::True => true,
        AtomReading::False => false,
        AtomReading::Undefined(cause) => panic!("a truth value, not {cause:?}"),
    }
}

/// FR-194-AC-2 over FR-516-AC-47's samples: `sum` over `past[0, 2]` of 3, 4, 5, 6 is 15 at position 3
/// and 7 at position 1; every comparison reads the checked term's operator.
#[trace("TC-629", "FR-194-AC-2")]
#[test]
fn a_sum_is_compared_by_each_comparison_of_the_checked_term() {
    let samples = values(&[3, 4, 5, 6]);
    let sum = |comparison: &str, threshold: i64, position: u64| {
        truth(&read(
            &format!("sum of c.level over past[0,2] {comparison} {threshold}"),
            Rational::from_integer(Integer::from(threshold)),
            &samples,
            position,
        ))
    };
    let at_three = ["<", "<=", "=", ">=", ">"].map(|comparison| sum(comparison, 15, 3));
    assert_eq!(at_three, [false, true, true, true, false]);
    assert!(!sum("<=", 14, 3));
    // Position 1 windows positions 0 and 1 only: 3 + 4.
    assert!(sum("=", 7, 1));
    assert!(!sum("=", 15, 1));
}

/// FR-194-AC-2 over FR-516-AC-47 and AC-48's samples: `min` 4 and `max` 6 over `past[0, 2]` at
/// position 3, `quantile 0.5` 5 and `quantile 0.75` 6, `fraction` 2/3 of
/// `Holds` samples `true, false, true`, and `count` of the `Counted`
/// samples, an `Excluded` one not counted.
#[trace("TC-629", "FR-194-AC-2")]
#[test]
fn each_kind_reads_the_window_of_its_checked_term() {
    let samples = values(&[3, 4, 5, 6]);
    let at_three = |term: &str, threshold: i64| {
        truth(&read(
            term,
            Rational::from_integer(Integer::from(threshold)),
            &samples,
            3,
        ))
    };
    assert!(at_three("min of c.level over past[0,2] = 4", 4));
    assert!(!at_three("min of c.level over past[0,2] = 5", 5));
    assert!(at_three("max of c.level over past[0,2] = 6", 6));
    assert!(at_three("quantile 0.5 of c.level over past[0,2] = 5", 5));
    assert!(at_three("quantile 0.75 of c.level over past[0,2] = 6", 6));
    assert!(!at_three("quantile 0.75 of c.level over past[0,2] = 5", 5));

    let holds = [true, false, true].map(AggregateSample::Holds);
    let fraction = |comparison: &str| {
        truth(&read(
            &format!("fraction holds(c.level != 0) over past[0,2] {comparison} 2/3"),
            ratio(2, 3),
            &holds,
            2,
        ))
    };
    assert!(fraction("="));
    assert!(!fraction("<"));

    let counted = [
        AggregateSample::Counted,
        AggregateSample::Counted,
        AggregateSample::Excluded,
        AggregateSample::Counted,
    ];
    let count = |threshold: i64| {
        truth(&read(
            "count over past[0,2] = 2",
            Rational::from_integer(Integer::from(threshold)),
            &counted,
            3,
        ))
    };
    assert!(count(2));
    assert!(!count(3));
}

/// FR-194-AC-3: fewer samples than `min_count` is insufficient data with the
/// window's count and the term's `min_count`, never a truth value, and the
/// first undefined sample, oldest first, wins over a short window.
#[trace("TC-629", "FR-194-AC-3")]
#[test]
fn a_short_window_is_insufficient_data_and_an_undefined_sample_wins() {
    let one = [AggregateSample::Counted, AggregateSample::Excluded];
    assert_eq!(
        read(
            "count over past[0,1] min_count 2 >= 1",
            ratio(1, 1),
            &one,
            1
        ),
        AtomReading::Undefined(UndefinedCause::InsufficientData(InsufficientData {
            count: 1,
            min_count: 2
        }))
    );
    let undefined = UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero);
    let both = [
        AggregateSample::Counted,
        AggregateSample::Undefined(undefined.clone()),
    ];
    assert_eq!(
        read(
            "count over past[0,1] min_count 3 >= 1",
            ratio(1, 1),
            &both,
            1
        ),
        AtomReading::Undefined(undefined)
    );
}

/// Which sample kinds a term takes (FR-516 "Window aggregate atoms").
#[trace("TC-629")]
#[test]
fn a_sample_of_another_kind_is_not_taken_by_the_term() {
    let kinds = |term: &str| {
        let fx = fixture(
            EVENT,
            "c: Counter",
            "on origin",
            &format!("always[0,3] ({term})"),
        );
        let (_, checked) = fx.clause.aggregates().iter().next().expect("an aggregate");
        let atom = AggregateAtom::of_checked(checked, ratio(1, 1)).expect("well formed");
        [
            AggregateSample::Counted,
            AggregateSample::Value(ratio(1, 1)),
            AggregateSample::Holds(true),
            AggregateSample::Excluded,
        ]
        .map(|sample| sample.is_taken_by(atom.kind()))
    };
    assert_eq!(
        kinds("count over past[0,2] >= 1"),
        [true, false, false, true]
    );
    assert_eq!(
        kinds("sum of c.level over past[0,2] >= 1"),
        [false, true, false, true]
    );
    assert_eq!(
        kinds("quantile 0.5 of c.level over past[0,2] >= 1"),
        [false, true, false, true]
    );
    assert_eq!(
        kinds("fraction holds(c.level != 0) over past[0,2] >= 1"),
        [false, false, true, true]
    );
}

/// The evaluator plans a clause with an aggregate atom and reads it through
/// the letter as it reads a `holds` atom: the atom the letter is asked about
/// is the aggregate node.
#[trace("TC-847", "FR-327-AC-7")]
#[test]
fn the_evaluator_asks_the_letter_for_the_aggregate_node() {
    struct Asked(Vec<FormulaNodeId>);
    impl TemporalLetters for Asked {
        fn atom(
            &mut self,
            atom: FormulaNodeId,
            _: TemporalPosition,
            _: &mut Meter,
        ) -> Result<AtomReading, LetterStop> {
            self.0.push(atom);
            Ok(AtomReading::True)
        }
        fn activation(
            &mut self,
            _: TemporalPosition,
            _: &mut Meter,
        ) -> Result<AtomReading, LetterStop> {
            Ok(AtomReading::True)
        }
    }
    let fx = fixture(
        EVENT,
        "c: Counter",
        "on origin",
        "always[0,1] (count over past[0,1] >= 1)",
    );
    let mut asked = Asked(Vec::new());
    let outcome = evaluate(
        &fx.clause,
        TemporalTrace::Finite { len: 2 },
        &mut asked,
        &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
    )
    .expect("the trace is readable");
    assert_eq!(outcome, TemporalOutcome::Holds);
    let aggregate = *fx.clause.aggregates().keys().next().expect("an aggregate");
    assert_eq!(asked.0.len(), 2);
    assert!(asked.0.iter().all(|id| id.index() == aggregate));
}

/// FR-194-AC-10: a `count` or `sum` with `min_count 0` over a window of no
/// counted values is 0, not insufficient data; `of_checked` keeps the written
/// `min_count`.
#[trace("TC-629", "FR-194-AC-10")]
#[test]
fn a_zero_min_count_count_over_an_empty_window_is_zero() {
    let excluded = [AggregateSample::Excluded, AggregateSample::Excluded];
    assert!(truth(&read(
        "count over past[0,1] min_count 0 <= 2",
        ratio(2, 1),
        &excluded,
        1
    )));
    assert!(!truth(&read(
        "count over past[0,1] min_count 0 >= 1",
        ratio(1, 1),
        &excluded,
        1
    )));
    assert!(truth(&read(
        "sum of c.level over past[0,1] min_count 0 = 0",
        ratio(0, 1),
        &excluded,
        1
    )));
}

/// FR-194, FR-337-AC-15: the duration window of a timestamped clause is read
/// from the checked term's bounds, the nearest age `a` first: with stamps 0,
/// 100, 200, 400 ms, `past[0 ms, 300 ms]` at the last position holds the
/// stamps 100, 200 and 400, and `past[100 ms, 300 ms]` holds 100 and 200.
#[trace("TC-629", "FR-194-AC-1")]
#[test]
fn a_duration_window_reads_the_checked_terms_bounds_in_order() {
    let equal = |window: &str, expected: i64| {
        timestamped(
            &format!("always[0,3] (count over {window} = {expected})"),
            |clause| {
                let (_, checked) = clause.aggregates().iter().next().expect("an aggregate");
                let samples = vec![AggregateSample::Counted; 4];
                let stamps = [0_i64, 100, 200, 400]
                    .map(|stamp| Rational::from_integer(Integer::from(stamp)))
                    .to_vec();
                let atom =
                    AggregateAtom::of_checked(checked, ratio(expected, 1)).expect("well formed");
                truth(
                    &atom
                        .evaluate(
                            &WindowTrace::timed(stamps).expect("increasing stamps"),
                            &mut SampledWindow::new(&samples, atom.kind()),
                            DEFAULT_MAX_WINDOW_VALUES,
                            TemporalPosition::new(3),
                            &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
                        )
                        .expect("the window reads"),
                )
            },
        )
    };
    assert!(equal("past[0 ms, 300 ms]", 3));
    assert!(!equal("past[0 ms, 300 ms]", 2));
    assert!(equal("past[100 ms, 300 ms]", 2));
    assert!(!equal("past[100 ms, 300 ms]", 3));
}

/// A position the owner supplied no sample for, and a sample of a kind the
/// term does not take, are faults: a `count` over a `Value` sample and a
/// `sum` over a `Counted` one alike, and the last sample is not repeated.
#[trace("TC-629")]
#[test]
fn an_unsupplied_position_and_a_sample_of_another_kind_are_faults() {
    let fault = |code: &'static str| {
        LetterStop::Fault(quire_semantic_value::catalog::InternalFault::new(
            "eval.temporal",
            code,
        ))
    };
    let attempt = |term: &str, samples: &[AggregateSample], len: u64, position: u64| {
        let fx = fixture(
            EVENT,
            "c: Counter",
            "on origin",
            &format!("always[0,3] ({term})"),
        );
        let (_, checked) = fx.clause.aggregates().iter().next().expect("an aggregate");
        let atom = AggregateAtom::of_checked(checked, ratio(0, 1)).expect("well formed");
        atom.evaluate(
            &WindowTrace::positions(len),
            &mut SampledWindow::new(samples, atom.kind()),
            DEFAULT_MAX_WINDOW_VALUES,
            TemporalPosition::new(position),
            &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
        )
    };
    let counted = [AggregateSample::Counted, AggregateSample::Counted];
    // The window at position 3 reaches positions 2 and 3, which the owner
    // did not supply.
    assert!(matches!(
        attempt("count over past[0,1] >= 0", &counted, 4, 3),
        Err(LetterStop::Fault(_))
    ));
    assert_eq!(
        attempt(
            "count over past[0,0] >= 0",
            &[AggregateSample::Value(ratio(1, 1))],
            1,
            0
        ),
        Err(fault("aggregate-sample-kind"))
    );
    assert_eq!(
        attempt(
            "sum of c.level over past[0,0] >= 0",
            &[AggregateSample::Counted],
            1,
            0
        ),
        Err(fault("aggregate-sample-kind"))
    );
}

/// FR-516 "Window aggregate atoms": a finite-window or package run reads an
/// atom as a `holds` valuation, and an aggregate atom has none until the
/// request carries its samples, so the facts of a clause with an aggregate
/// atom are refused and the letter is never asked for the atom (a valuation of
/// `True` would answer a truth FR-194 does not give for `min_count 5`).
#[trace("TC-629")]
#[test]
fn a_finite_window_run_never_reads_an_aggregate_atom_as_a_valuation() {
    struct Never;
    impl TemporalLetters for Never {
        fn atom(
            &mut self,
            _: FormulaNodeId,
            _: TemporalPosition,
            _: &mut Meter,
        ) -> Result<AtomReading, LetterStop> {
            panic!("the aggregate atom is read from the letter");
        }
        fn valuation(
            &mut self,
            _: FormulaNodeId,
            _: TemporalPosition,
            _: &mut Meter,
        ) -> Result<Valuation, LetterStop> {
            panic!("the aggregate atom is read from the valuation map");
        }
        fn activation(
            &mut self,
            _: TemporalPosition,
            _: &mut Meter,
        ) -> Result<AtomReading, LetterStop> {
            Ok(AtomReading::True)
        }
    }
    let trace = TimestampedTrace::admit(
        "C",
        WindowClock {
            unit: "ms".to_owned(),
            cover: ClockCover::Exact,
        },
        (0..3)
            .map(|stamp| TimestampedObservation {
                stamp,
                order_key: None,
                observation: format!("obs{stamp}"),
            })
            .collect(),
        WindowProgress {
            watermark: Watermark::Inclusive(2),
            completeness: ProgressCompleteness::Complete { through: 2 },
            scope: ProgressClosure::Closed,
            execution: ProgressClosure::Closed,
            lower_boundary: LowerBoundary::AuthoritativeOrigin,
        },
    )
    .expect("the trace admits");
    let refused = evaluate_finite_window(
        &timestamped(
            "always[0,2] (count over past[0 ms, 1 ms] min_count 5 >= 1)",
            ClauseFacts::of_checked,
        ),
        &trace,
        &mut Never,
        &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
    );
    assert!(
        matches!(refused, Err(TemporalRefusal::Fault(_))),
        "{refused:?}"
    );
}
