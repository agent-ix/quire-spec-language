// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-194 (TC-629): window aggregates evaluate exactly over past windows
//! under both trace profiles, and hand the temporal evaluator a Boolean per
//! position.

use ix_trace_rs::trace;
use qsl_eval::temporal::{
    evaluate, AggregateAtom, AggregateKind, AggregateStop, AggregateTerm, AtomReading,
    InsufficientData, PastWindow, TemporalLetters, TemporalOutcome, TemporalPosition,
    TemporalTrace, TermRefusal, UndefinedCause, WindowLimitReached, WindowRefusal, WindowSource,
    WindowTrace, DEFAULT_MAX_WINDOW_VALUES, MAX_WINDOW_VALUES_SETTING,
};
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    check_clauses, CheckedTemporalClause, FormulaNodeId, PackageDeclarations,
    SCALAR_LIMITS_UNLIMITED,
};
use qsl_semantics::model::intake::SelectedModels;
use quire_exact::{ComparisonOperator, Incomplete, Integer, LimitKind, Meter, Rational};

const UNLIMITED: u64 = u64::MAX;

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn whole(value: i64) -> Rational {
    ratio(value, 1)
}

/// The rows of a trace: whether the filter admits it, its value and its
/// predicate. `visited` lists the positions the window filter was asked
/// about, in order.
struct Rows {
    admit: Vec<bool>,
    value: Vec<Rational>,
    predicate: Vec<bool>,
    visited: Vec<u64>,
}

impl Rows {
    fn new(admit: Vec<bool>, value: Vec<Rational>, predicate: Vec<bool>) -> Self {
        Self {
            admit,
            value,
            predicate,
            visited: Vec::new(),
        }
    }

    fn values(values: &[i64]) -> Self {
        Self::new(
            vec![true; values.len()],
            values.iter().copied().map(whole).collect(),
            vec![false; values.len()],
        )
    }
}

fn row<T: Clone>(rows: &[T], position: TemporalPosition) -> T {
    rows[usize::try_from(position.index()).expect("small index")].clone()
}

impl WindowSource for Rows {
    fn admits(&mut self, position: TemporalPosition, _: &mut Meter) -> Result<bool, Incomplete> {
        self.visited.push(position.index());
        Ok(row(&self.admit, position))
    }

    fn value(&mut self, position: TemporalPosition, _: &mut Meter) -> Result<Rational, Incomplete> {
        Ok(row(&self.value, position))
    }

    fn predicate(&mut self, position: TemporalPosition, _: &mut Meter) -> Result<bool, Incomplete> {
        Ok(row(&self.predicate, position))
    }
}

fn term(kind: AggregateKind, window: PastWindow, min_count: u64) -> AggregateTerm {
    AggregateTerm::new(kind, window, min_count).expect("a well-formed term")
}

fn seconds(a: i64, b: i64) -> PastWindow {
    PastWindow::Duration {
        a: whole(a),
        b: whole(b),
    }
}

fn read(
    atom: &AggregateAtom,
    trace: &WindowTrace,
    rows: &mut Rows,
    limit: u64,
    position: u64,
) -> Result<AtomReading, AggregateStop> {
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    atom.evaluate(
        trace,
        rows,
        limit,
        TemporalPosition::new(position),
        &mut meter,
    )
}

fn truth(reading: Result<AtomReading, AggregateStop>) -> bool {
    match reading {
        Ok(AtomReading::True) => true,
        Ok(AtomReading::False) => false,
        other => panic!("a Boolean reading, not {other:?}"),
    }
}

/// Twenty responses, one second apart, with latencies of 1 ms but for the
/// last two, 6 and 7 ms. Values are in milliseconds, or in seconds when
/// `unit` is 1000.
fn responses(unit: i64) -> (WindowTrace, Rows) {
    let mut latencies = vec![1_i64; 18];
    latencies.extend([6, 7]);
    let stamps = (1..=20).map(whole).collect();
    let rows = Rows::new(
        vec![true; 20],
        latencies.into_iter().map(|ms| ratio(ms, unit)).collect(),
        vec![true; 20],
    );
    (WindowTrace::timed(stamps).expect("ordered"), rows)
}

/// FR-194-AC-1: the nearest-rank 95th percentile of the 20 latencies is the
/// 19th smallest, 6 ms, so `<= 5 ms` is false; the 90th is the 18th, 1 ms,
/// so it is true. The threshold `0.005 s` over values in seconds agrees.
#[trace("TC-629", "FR-194-AC-1")]
#[test]
fn a_timestamped_quantile_takes_the_nearest_rank_value() {
    let at_last = 19;
    for (unit, threshold) in [(1, whole(5)), (1000, ratio(5, 1000))] {
        for (q, expected) in [(ratio(95, 100), false), (ratio(9, 10), true)] {
            let atom = AggregateAtom::new(
                term(AggregateKind::Quantile(q.clone()), seconds(0, 300), 1),
                ComparisonOperator::LessOrEqual,
                threshold.clone(),
            );
            let (trace, mut rows) = responses(unit);
            assert_eq!(
                truth(read(&atom, &trace, &mut rows, UNLIMITED, at_last)),
                expected,
                "quantile {q} against {threshold}"
            );
        }
    }
}

/// FR-194-AC-1: under timestamped-event, `past[2 s, 4 s]` at t = 10 holds
/// the events stamped 6 to 8 and no others.
#[trace("TC-629", "FR-194-AC-1")]
#[test]
fn a_timestamped_window_keeps_only_events_in_its_time_range() {
    let atom = AggregateAtom::new(
        term(AggregateKind::Count, seconds(2, 4), 1),
        ComparisonOperator::Equal,
        whole(3),
    );
    let (trace, mut rows) = responses(1);
    // At t = 10 (position 9) the window holds t in [6, 8]: positions 5 to 7.
    assert!(truth(read(&atom, &trace, &mut rows, UNLIMITED, 9)));
    assert_eq!(rows.visited, [5, 6, 7]);
}

/// FR-194-AC-2: under event-position `count ... past[0, 9]` reads positions
/// 0 to 4 at position 4 and 6 to 15 at position 15, never a later one.
#[trace("TC-629", "FR-194-AC-2")]
#[test]
fn an_event_position_window_reads_back_from_the_current_position_only() {
    let trace = WindowTrace::positions(16);
    let atom = AggregateAtom::new(
        term(
            AggregateKind::Count,
            PastWindow::Positions { a: 0, b: 9 },
            1,
        ),
        ComparisonOperator::LessOrEqual,
        whole(2),
    );
    let errors: Vec<bool> = (0..16).map(|position| position % 5 == 0).collect();
    let mut rows = Rows::new(errors, vec![whole(0); 16], vec![false; 16]);
    // Errors at 0 and 5 within 0..=4 and 0..=5: count 1 at position 4.
    assert!(truth(read(&atom, &trace, &mut rows, UNLIMITED, 4)));
    assert_eq!(rows.visited, [0, 1, 2, 3, 4]);

    let mut rows = Rows::new(
        (0..16).map(|position| position % 5 == 0).collect(),
        vec![whole(0); 16],
        vec![false; 16],
    );
    // Errors at 10 and 15 within 6..=15: count 2.
    assert!(truth(read(&atom, &trace, &mut rows, UNLIMITED, 15)));
    assert_eq!(rows.visited, (6..=15).collect::<Vec<u64>>());
}

/// FR-194-AC-2: `fraction holds(e.ok)` over `true, false, true, true` is
/// exactly 3/4, and `sum`, `min` and `max` of 1, 4, 2 are 7, 1 and 4.
#[trace("TC-629", "FR-194-AC-2")]
#[test]
fn fraction_sum_min_and_max_are_exact() {
    let trace = WindowTrace::positions(4);
    let window = || PastWindow::Positions { a: 0, b: 3 };
    let mut ok = Rows::new(
        vec![true; 4],
        vec![whole(0); 4],
        vec![true, false, true, true],
    );
    let fraction = |operator| {
        AggregateAtom::new(
            term(AggregateKind::Fraction, window(), 1),
            operator,
            ratio(3, 4),
        )
    };
    assert!(truth(read(
        &fraction(ComparisonOperator::Equal),
        &trace,
        &mut ok,
        UNLIMITED,
        3
    )));
    assert!(!truth(read(
        &fraction(ComparisonOperator::Greater),
        &trace,
        &mut ok,
        UNLIMITED,
        3
    )));

    for (kind, expected) in [
        (AggregateKind::Sum, 7),
        (AggregateKind::Min, 1),
        (AggregateKind::Max, 4),
    ] {
        let atom = AggregateAtom::new(
            term(kind.clone(), PastWindow::Positions { a: 0, b: 2 }, 1),
            ComparisonOperator::Equal,
            whole(expected),
        );
        let mut rows = Rows::values(&[1, 4, 2]);
        assert!(
            truth(read(
                &atom,
                &WindowTrace::positions(3),
                &mut rows,
                UNLIMITED,
                2
            )),
            "{kind:?} of 1, 4, 2 is {expected}"
        );
    }
}

/// FR-194-AC-3: with `min_count 5` and three responses in the window, the
/// atom is `Undefined(InsufficientData{3, 5})`.
#[trace("TC-629", "FR-194-AC-3")]
#[test]
fn a_sparse_window_is_undefined_with_insufficient_data() {
    let atom = AggregateAtom::new(
        term(
            AggregateKind::Count,
            PastWindow::Positions { a: 0, b: 9 },
            5,
        ),
        ComparisonOperator::LessOrEqual,
        whole(100),
    );
    let mut rows = Rows::new(
        vec![true, false, true, false, true, false],
        vec![whole(0); 6],
        vec![false; 6],
    );
    assert_eq!(
        read(&atom, &WindowTrace::positions(6), &mut rows, UNLIMITED, 5),
        Ok(AtomReading::Undefined(UndefinedCause::InsufficientData(
            InsufficientData {
                count: 3,
                min_count: 5
            }
        )))
    );
}

/// FR-194-AC-3: with `max_window_values` 10 and 20 values in the window the
/// evaluation stops naming `max_window_values` and 10; with no ceiling it
/// charges exactly 20 work units.
#[trace("TC-629", "FR-194-AC-3")]
#[test]
fn a_window_over_the_limit_stops_and_each_value_charges_one_unit() {
    let atom = AggregateAtom::new(
        term(AggregateKind::Sum, PastWindow::Positions { a: 0, b: 19 }, 1),
        ComparisonOperator::GreaterOrEqual,
        whole(0),
    );
    let trace = WindowTrace::positions(20);
    let mut rows = Rows::values(&[1; 20]);
    assert_eq!(
        read(&atom, &trace, &mut rows, 10, 19),
        Err(AggregateStop::LimitReached(WindowLimitReached {
            limit: "max_window_values",
            value: 10,
            setting: "monitor.max_window_values",
        }))
    );
    assert_eq!(MAX_WINDOW_VALUES_SETTING, "monitor.max_window_values");
    assert_eq!(DEFAULT_MAX_WINDOW_VALUES, 1_048_576);

    let mut rows = Rows::values(&[1; 20]);
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    atom.evaluate(&trace, &mut rows, 20, TemporalPosition::new(19), &mut meter)
        .expect("exactly the limit is allowed");
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 20);
}

/// FR-194-AC-3: an exhausted work budget stops the aggregate `Incomplete`.
#[trace("TC-629", "FR-194-AC-3")]
#[test]
fn an_exhausted_budget_stops_the_aggregate_without_a_value() {
    let atom = AggregateAtom::new(
        term(
            AggregateKind::Count,
            PastWindow::Positions { a: 0, b: 19 },
            1,
        ),
        ComparisonOperator::GreaterOrEqual,
        whole(0),
    );
    let mut rows = Rows::values(&[1; 20]);
    let mut meter = Meter::new(quire_exact::ScalarLimits {
        work_units: 5,
        ..SCALAR_LIMITS_UNLIMITED
    });
    let stop = atom
        .evaluate(
            &WindowTrace::positions(20),
            &mut rows,
            UNLIMITED,
            TemporalPosition::new(19),
            &mut meter,
        )
        .expect_err("five units cannot visit twenty values");
    assert!(
        matches!(&stop, AggregateStop::Incomplete(record) if record.limit_kind == LimitKind::WorkUnits),
        "{stop:?}"
    );
}

/// FR-194-AC-4: a quantile of 1 or 0, `min_count` 0 and a reversed window
/// are refused as terms, and a window of the other profile's kind is
/// refused on a trace.
#[trace("TC-629", "FR-194-AC-4")]
#[test]
fn ill_formed_terms_and_mismatched_windows_are_refused() {
    let positions = || PastWindow::Positions { a: 0, b: 3 };
    assert_eq!(
        AggregateTerm::new(AggregateKind::Quantile(whole(1)), positions(), 1),
        Err(TermRefusal::QuantileOutOfRange)
    );
    assert_eq!(
        AggregateTerm::new(AggregateKind::Quantile(whole(0)), positions(), 1),
        Err(TermRefusal::QuantileOutOfRange)
    );
    assert_eq!(
        AggregateTerm::new(AggregateKind::Count, positions(), 0),
        Err(TermRefusal::ZeroMinCount)
    );
    assert_eq!(
        AggregateTerm::new(
            AggregateKind::Count,
            PastWindow::Positions { a: 4, b: 3 },
            1
        ),
        Err(TermRefusal::EmptyWindow)
    );
    assert_eq!(
        AggregateTerm::new(AggregateKind::Count, seconds(-1, 3), 1),
        Err(TermRefusal::NegativeDuration)
    );

    let atom = AggregateAtom::new(
        term(AggregateKind::Count, seconds(0, 300), 1),
        ComparisonOperator::LessOrEqual,
        whole(1),
    );
    assert_eq!(
        read(
            &atom,
            &WindowTrace::positions(2),
            &mut Rows::values(&[1, 1]),
            UNLIMITED,
            1
        ),
        Err(AggregateStop::Refused(WindowRefusal::WindowKindMismatch))
    );
    assert_eq!(
        WindowTrace::timed(vec![whole(2), whole(1)]),
        Err(WindowRefusal::TimestampsDecrease)
    );
    assert_eq!(
        read(
            &atom,
            &WindowTrace::positions(2),
            &mut Rows::values(&[1, 1]),
            UNLIMITED,
            2
        ),
        Err(AggregateStop::Refused(WindowRefusal::OutsideTrace {
            position: 2
        }))
    );
}

const EVENT: &str = "quire.temporal.event-position.false-extension/v1";

/// A temporal clause `always[0,3] holds(c.level <= 9)` over an event
/// position trace: its one atom is read from an aggregate.
fn clause() -> CheckedTemporalClause {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t0 = \"{EVENT}\";\n\
         record Counter {{ level: Int[0, 9]; }}\n\
         temporal C using t0 over (c: Counter) clock \"model-steps\" on origin {{ \
         always[0,3] holds(c.level <= 9) }}\n"
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let package = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        SelectedModels::default(),
        Vec::new(),
    )
    .expect("the unit assembles");
    let mut checked = check_clauses(&package);
    match checked.pop() {
        Some(Ok(clause)) if checked.is_empty() => clause,
        other => panic!("one checked clause, not {other:?}"),
    }
}

/// The clause's atom, read as the aggregate at each position.
struct Derived<'a> {
    atom: &'a AggregateAtom,
    trace: &'a WindowTrace,
    rows: Rows,
    readings: Vec<AtomReading>,
}

impl TemporalLetters for Derived<'_> {
    fn atom(
        &mut self,
        _: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        let reading = self
            .atom
            .evaluate(self.trace, &mut self.rows, UNLIMITED, position, meter)
            .expect("the window is readable");
        self.readings.push(reading.clone());
        Ok(reading)
    }

    fn activation(
        &mut self,
        _: TemporalPosition,
        _: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        Ok(AtomReading::True)
    }
}

fn settle(atom: &AggregateAtom, errors: Vec<bool>) -> (TemporalOutcome, Vec<AtomReading>) {
    let len = errors.len();
    let trace = WindowTrace::positions(u64::try_from(len).expect("small"));
    let mut letters = Derived {
        atom,
        trace: &trace,
        rows: Rows::new(errors, vec![whole(0); len], vec![false; len]),
        readings: Vec::new(),
    };
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let outcome = evaluate(
        &clause(),
        TemporalTrace::Finite {
            len: u64::try_from(len).expect("small"),
        },
        &mut letters,
        &mut meter,
    )
    .expect("the trace is readable");
    (outcome, letters.readings)
}

/// FR-194-AC-4 and AC-3: the temporal evaluator receives the aggregate as a
/// Boolean per position, a holding clause settles `Holds`, and an atom
/// undefined for too few values makes the clause undefined at that position.
#[trace("TC-629", "FR-194-AC-4")]
#[test]
fn the_temporal_layer_reads_the_aggregate_as_a_boolean_signal() {
    let errors_at_most_one = AggregateAtom::new(
        term(
            AggregateKind::Count,
            PastWindow::Positions { a: 0, b: 9 },
            1,
        ),
        ComparisonOperator::LessOrEqual,
        whole(1),
    );
    let (outcome, readings) = settle(&errors_at_most_one, vec![true, false, false, false]);
    assert_eq!(outcome, TemporalOutcome::Holds);
    assert!(readings
        .iter()
        .all(|reading| matches!(reading, AtomReading::True | AtomReading::False)));

    // Two errors inside the window falsify the atom at position 1.
    let (outcome, _) = settle(&errors_at_most_one, vec![true, true, false, false]);
    assert_eq!(
        outcome,
        TemporalOutcome::Violated {
            at: TemporalPosition::new(0)
        }
    );

    let needs_five = AggregateAtom::new(
        term(
            AggregateKind::Count,
            PastWindow::Positions { a: 0, b: 9 },
            5,
        ),
        ComparisonOperator::LessOrEqual,
        whole(1),
    );
    let (outcome, _) = settle(&needs_five, vec![true; 4]);
    match outcome {
        TemporalOutcome::Undefined(undefined) => assert_eq!(
            undefined.cause,
            UndefinedCause::InsufficientData(InsufficientData {
                count: 1,
                min_count: 5
            })
        ),
        other => panic!("an undefined clause, not {other:?}"),
    }
}
