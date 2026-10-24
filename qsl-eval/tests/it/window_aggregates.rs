// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-194 (TC-629): window aggregates evaluate exactly over past windows
//! under both trace profiles, and hand the temporal evaluator a Boolean per
//! position.

use ix_trace_rs::trace;
use qsl_eval::temporal::{
    evaluate, AggregateAtom, AggregateKind, AggregateTerm, AtomReading, InsufficientData,
    LetterStop, ObservedWindowExpressions, ObservedWindowSource, PastWindow, TemporalLetters,
    TemporalOutcome, TemporalPosition, TemporalTrace, TermRefusal, UndefinedCause,
    UndefinedEvaluation, WindowLimitReached, WindowRead, WindowRefusal, WindowSource, WindowTrace,
    DEFAULT_MAX_WINDOW_VALUES, MAX_WINDOW_VALUES_SETTING,
};
use qsl_forms::{BinaryOperator, Expression};
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    check_clauses, CheckedExpression, CheckedTemporalClause, FormulaNodeId, PackageDeclarations,
    SCALAR_LIMITS_UNLIMITED,
};
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{
    ComparisonOperator, EffectiveId, FieldValue, Integer, IntegerInterval, LimitKind, Meter,
    ObjectId, ObjectReference, Presence, Rational, RationalDomain, ScalarLimits, Undefined,
    UniverseId, Value, ValueType,
};
use quire_semantic_value::catalog::Category;
use quire_semantic_value::checking::{CheckMode, CheckingLimits};
use quire_semantic_value::declaration::{FieldDeclaration, ObjectTypeDeclaration, TypeEnvironment};
use quire_semantic_value::object_closure::ObjectClosure;
use sha2::{Digest, Sha256};

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
    /// The positions whose value is undefined by division by zero.
    divides_by_zero: Vec<u64>,
}

impl Rows {
    fn new(admit: Vec<bool>, value: Vec<Rational>, predicate: Vec<bool>) -> Self {
        Self {
            admit,
            value,
            predicate,
            visited: Vec::new(),
            divides_by_zero: Vec::new(),
        }
    }

    /// These rows, but with the value at each of `positions` undefined.
    fn undefined_at(mut self, positions: &[u64]) -> Self {
        self.divides_by_zero = positions.to_vec();
        self
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
    fn admits(
        &mut self,
        position: TemporalPosition,
        _: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        self.visited.push(position.index());
        Ok(WindowRead::Defined(row(&self.admit, position)))
    }

    fn value(
        &mut self,
        position: TemporalPosition,
        _: &mut Meter,
    ) -> Result<WindowRead<Rational>, LetterStop> {
        if self.divides_by_zero.contains(&position.index()) {
            return Ok(WindowRead::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            )));
        }
        Ok(WindowRead::Defined(row(&self.value, position)))
    }

    fn predicate(
        &mut self,
        position: TemporalPosition,
        _: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        Ok(WindowRead::Defined(row(&self.predicate, position)))
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
) -> Result<AtomReading, LetterStop> {
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    atom.evaluate(
        trace,
        rows,
        limit,
        TemporalPosition::new(position),
        &mut meter,
    )
}

fn truth(reading: Result<AtomReading, LetterStop>) -> bool {
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
/// so it is true. The second pass states the latencies pre-scaled to seconds
/// against `5/1000`: it shows the arithmetic is exact in either scale, not
/// that a threshold in another unit is converted (the checker's step, which
/// is not implemented here).
#[trace("TC-629", "FR-194-AC-1")]
#[test]
fn a_timestamped_quantile_takes_the_nearest_rank_value_in_either_prescaled_unit() {
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

/// FR-194-AC-2: `past[2, 3]` at position 5 holds positions 2 and 3, and at
/// position 1, before its nearest offset, holds nothing.
#[trace("TC-629", "FR-194-AC-2")]
#[test]
fn an_event_position_window_with_a_nearest_offset_skips_the_newest_positions() {
    let atom = AggregateAtom::new(
        term(
            AggregateKind::Count,
            PastWindow::Positions { a: 2, b: 3 },
            1,
        ),
        ComparisonOperator::Equal,
        whole(2),
    );
    let trace = WindowTrace::positions(6);
    let mut rows = Rows::values(&[1; 6]);
    assert!(truth(read(&atom, &trace, &mut rows, UNLIMITED, 5)));
    assert_eq!(rows.visited, [2, 3]);

    let mut rows = Rows::values(&[1; 6]);
    assert_eq!(
        read(&atom, &trace, &mut rows, UNLIMITED, 1),
        Ok(AtomReading::Undefined(UndefinedCause::InsufficientData(
            InsufficientData {
                count: 0,
                min_count: 1
            }
        )))
    );
    assert!(rows.visited.is_empty());
}

/// FR-194-AC-2: `fraction holds(e.ok)` over `true, false, true, true` is
/// exactly 3/4, and `sum`, `min` and `max` of 1, 4, 2 are 7, 1 and 4; a
/// quantile whose rank is not whole rounds up.
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
        // ceil(1/2 * 3) = 2: the second smallest of 1, 2, 4.
        (AggregateKind::Quantile(ratio(1, 2)), 2),
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

/// FR-194-AC-3: a window of exactly `max_window_values` values is read, and
/// one more stops the evaluation naming `max_window_values` and the limit.
#[trace("TC-629", "FR-194-AC-3")]
#[test]
fn a_window_one_value_over_the_limit_stops() {
    let atom = AggregateAtom::new(
        term(AggregateKind::Sum, PastWindow::Positions { a: 0, b: 19 }, 1),
        ComparisonOperator::GreaterOrEqual,
        whole(0),
    );
    let trace = WindowTrace::positions(20);
    // Position 9 holds 10 values, position 10 holds 11.
    assert!(truth(read(
        &atom,
        &trace,
        &mut Rows::values(&[1; 20]),
        10,
        9
    )));
    assert_eq!(
        read(&atom, &trace, &mut Rows::values(&[1; 20]), 10, 10),
        Err(LetterStop::WindowLimit(WindowLimitReached {
            limit: "max_window_values",
            value: 10,
            setting: "monitor.max_window_values",
        }))
    );
    assert_eq!(MAX_WINDOW_VALUES_SETTING, "monitor.max_window_values");
    assert_eq!(DEFAULT_MAX_WINDOW_VALUES, 1_048_576);
}

/// FR-194-AC-3: one evaluation over 20 values charges 20 work units. A
/// filter that admits 10 of 20 positions still charges 20: the unit is the
/// window position visited, not the value gathered.
#[trace("TC-629", "FR-194-AC-3")]
#[test]
fn each_window_position_visited_charges_one_work_unit() {
    let atom = AggregateAtom::new(
        term(AggregateKind::Sum, PastWindow::Positions { a: 0, b: 19 }, 1),
        ComparisonOperator::GreaterOrEqual,
        whole(0),
    );
    let trace = WindowTrace::positions(20);
    let work = |mut rows: Rows| {
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        atom.evaluate(&trace, &mut rows, 20, TemporalPosition::new(19), &mut meter)
            .expect("the window is within the limit");
        meter.consumed(LimitKind::WorkUnits)
    };
    assert_eq!(work(Rows::values(&[1; 20])), 20);
    let half: Vec<bool> = (0..20).map(|position| position % 2 == 0).collect();
    let filtered = Rows::new(half, vec![whole(1); 20], vec![false; 20]);
    assert_eq!(work(filtered), 20);
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
    let mut meter = Meter::new(ScalarLimits {
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
        matches!(&stop, LetterStop::Incomplete(record) if record.limit_kind == LimitKind::WorkUnits),
        "{stop:?}"
    );
}

/// FR-194-AC-4: a quantile of 1 and a `min_count` of 0 are refused when the
/// term is formed.
#[trace("TC-629", "FR-194-AC-4")]
#[test]
fn a_quantile_of_one_and_a_zero_min_count_are_refused() {
    let positions = || PastWindow::Positions { a: 0, b: 3 };
    assert_eq!(
        AggregateTerm::new(AggregateKind::Quantile(whole(1)), positions(), 1),
        Err(TermRefusal::QuantileOutOfRange)
    );
    assert_eq!(
        AggregateTerm::new(AggregateKind::Count, positions(), 0),
        Err(TermRefusal::ZeroMinCount)
    );
}

/// A quantile of 0, a reversed or negative window, a window of the other
/// profile's kind, decreasing time stamps and a position off the trace are
/// refused.
#[test]
fn other_ill_formed_terms_and_unreadable_windows_are_refused() {
    let positions = || PastWindow::Positions { a: 0, b: 3 };
    assert_eq!(
        AggregateTerm::new(AggregateKind::Quantile(whole(0)), positions(), 1),
        Err(TermRefusal::QuantileOutOfRange)
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
        Err(LetterStop::Window(WindowRefusal::WindowKindMismatch))
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
        Err(LetterStop::Window(WindowRefusal::OutsideTrace {
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
struct Derived<'a, S> {
    atom: &'a AggregateAtom,
    trace: &'a WindowTrace,
    rows: S,
    limit: u64,
    readings: Vec<AtomReading>,
    /// The work units the aggregate itself charged, apart from the
    /// evaluator's node-visit and letter-read units.
    window_units: u64,
}

impl<'a, S: WindowSource> Derived<'a, S> {
    fn new(atom: &'a AggregateAtom, trace: &'a WindowTrace, rows: S, limit: u64) -> Self {
        Self {
            atom,
            trace,
            rows,
            limit,
            readings: Vec::new(),
            window_units: 0,
        }
    }
}

impl<S: WindowSource> TemporalLetters for Derived<'_, S> {
    fn atom(
        &mut self,
        _: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let before = meter.consumed(LimitKind::WorkUnits);
        let reading =
            self.atom
                .evaluate(self.trace, &mut self.rows, self.limit, position, meter)?;
        self.window_units += meter.consumed(LimitKind::WorkUnits) - before;
        self.readings.push(reading.clone());
        Ok(reading)
    }

    fn activation(
        &mut self,
        _: TemporalPosition,
        _: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        Ok(AtomReading::True)
    }
}

fn run(
    atom: &AggregateAtom,
    errors: Vec<bool>,
    limit: u64,
    budget: ScalarLimits,
) -> (TemporalOutcome, Vec<AtomReading>) {
    let len = errors.len();
    let trace = WindowTrace::positions(u64::try_from(len).expect("small"));
    let mut letters = Derived::new(
        atom,
        &trace,
        Rows::new(errors, vec![whole(0); len], vec![false; len]),
        limit,
    );
    let mut meter = Meter::new(budget);
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

fn settle(atom: &AggregateAtom, errors: Vec<bool>) -> (TemporalOutcome, Vec<AtomReading>) {
    run(atom, errors, UNLIMITED, SCALAR_LIMITS_UNLIMITED)
}

/// FR-194-AC-4: the temporal evaluator receives the aggregate as a Boolean
/// per position, and a holding clause settles `Holds`.
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

    // Two errors inside the window falsify the atom at position 1, and the
    // clause is violated at its activation, position 0.
    let (outcome, _) = settle(&errors_at_most_one, vec![true, true, false, false]);
    assert_eq!(
        outcome,
        TemporalOutcome::Violated {
            at: TemporalPosition::new(0)
        }
    );
}

/// FR-194-AC-3: an atom undefined for too few values makes the clause
/// undefined at that position, with the insufficient-data cause.
#[trace("TC-629", "FR-194-AC-3")]
#[test]
fn an_atom_with_too_few_values_makes_the_clause_undefined() {
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
        TemporalOutcome::Undefined(ref undefined) => assert_eq!(
            undefined.cause,
            UndefinedCause::InsufficientData(InsufficientData {
                count: 1,
                min_count: 5
            })
        ),
        other => panic!("an undefined clause, not {other:?}"),
    }
    // A window short of data is O-16 undefined, not a violation (FR-327).
    assert_eq!(outcome.category(), Category::Undefined);
}

/// FR-327: any other undefined atom stays category violation.
#[test]
fn a_kernel_undefined_atom_is_a_violation() {
    let outcome = TemporalOutcome::Undefined(UndefinedEvaluation {
        at: TemporalPosition::new(0),
        cause: UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero),
    });
    assert_eq!(outcome.category(), Category::Violation);
}

/// FR-194-AC-3: the stop for a window over `max_window_values` reaches the
/// run's outcome as the limit record, not as a work-budget `Incomplete`; an
/// exhausted work budget stays the ordinary `Incomplete`.
#[trace("TC-629", "FR-194-AC-3")]
#[test]
fn a_window_over_the_limit_settles_the_run_with_the_limit_record() {
    let count = AggregateAtom::new(
        term(
            AggregateKind::Count,
            PastWindow::Positions { a: 0, b: 19 },
            1,
        ),
        ComparisonOperator::GreaterOrEqual,
        whole(0),
    );
    // The window at position 2 holds 3 values, one more than the limit 2.
    let (outcome, _) = run(&count, vec![true; 20], 2, SCALAR_LIMITS_UNLIMITED);
    assert_eq!(
        outcome,
        TemporalOutcome::WindowLimitReached(WindowLimitReached {
            limit: "max_window_values",
            value: 2,
            setting: "monitor.max_window_values",
        })
    );
    assert_eq!(outcome.category(), Category::Incomplete);

    // A limit that admits every window the clause reads does not stop it.
    let (outcome, _) = run(&count, vec![true; 20], 20, SCALAR_LIMITS_UNLIMITED);
    assert_eq!(outcome, TemporalOutcome::Holds);

    let (outcome, _) = run(
        &count,
        vec![true; 20],
        UNLIMITED,
        ScalarLimits {
            work_units: 3,
            ..SCALAR_LIMITS_UNLIMITED
        },
    );
    assert!(
        matches!(&outcome, TemporalOutcome::Incomplete(record) if record.limit_kind == LimitKind::WorkUnits),
        "{outcome:?}"
    );
}

/// The package of one object type `M::E` with an integer field `x`, and the
/// object environment of each position of a trace of such an object `e1`
/// whose `x` is `xs[i]` at position `i`.
struct Observed {
    package: CheckedPackage,
    environments: Vec<ObjectEnvironment>,
    over: ObjectReference,
    object_type: EffectiveId,
}

impl Observed {
    fn new(xs: &[i64]) -> Self {
        let effective = EffectiveId::from_digest(Sha256::digest(b"M::E").into());
        let declaration = ObjectTypeDeclaration::new(
            effective,
            "M::E",
            vec![
                FieldDeclaration::new("x", ValueType::Integer, Presence::Required),
                FieldDeclaration::new("one", quotient(), Presence::Required),
            ],
        );
        let types = TypeEnvironment::new([], vec![declaration]).expect("one object type");
        // The complete value profile, which `/` needs, is selected by a
        // function declaration.
        let text = "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\";\n\
             function zero using v(): Integer pure { 0 }\n";
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "window", "git", "1"),
            "window.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        let mut declarations = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            unit,
            Vec::new(),
            Vec::new(),
        )
        .expect("the unit assembles");
        declarations.types = types.clone();
        let graph = declarations
            .check(CheckingLimits::default())
            .expect("the package checks");
        let over = ObjectReference::new(
            UniverseId::from_digest([0x42; 32]),
            effective,
            ObjectId::new("e1").expect("an object id"),
        );
        let environments = xs
            .iter()
            .map(|x| {
                let present = FieldValue::Present(Value::Integer(Integer::from(*x)));
                let one = FieldValue::Present(Value::Rational(whole(1)));
                ObjectEnvironment::new(
                    ObjectClosure::new(
                        &types,
                        [(over.clone(), vec![("x", present), ("one", one)])],
                        &[],
                    )
                    .expect("the object closes"),
                )
            })
            .collect();
        Self {
            package: CheckedPackage::link(graph),
            environments,
            over,
            object_type: effective,
        }
    }

    /// The checked expression over the parameter `e`.
    fn check(&self, expression: &Expression, expected: Option<&ValueType>) -> CheckedExpression {
        let parameter = ValueType::Reference(self.object_type);
        self.package
            .graph()
            .check_expression(
                vec![("e".to_owned(), parameter)],
                expression,
                expected,
                CheckMode::Kernel,
                CheckingLimits::default(),
            )
            .expect("the expression checks")
    }

    /// `atom` at `position` of `trace`, its expressions read from the objects.
    fn read(
        &self,
        atom: &AggregateAtom,
        expressions: ObservedWindowExpressions<'_>,
        trace: &WindowTrace,
        position: u64,
    ) -> AtomReading {
        let mut source = self.source(expressions);
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        atom.evaluate(
            trace,
            &mut source,
            UNLIMITED,
            TemporalPosition::new(position),
            &mut meter,
        )
        .expect("the window is readable")
    }

    fn source<'a>(
        &'a self,
        expressions: ObservedWindowExpressions<'a>,
    ) -> ObservedWindowSource<'a> {
        ObservedWindowSource::new(&self.package, &self.environments, &self.over, expressions)
    }
}

/// The rational type of `6 / e.x`.
fn quotient() -> ValueType {
    let interval = |lower: i64, upper: i64| {
        IntegerInterval::new(Integer::from(lower), Integer::from(upper)).expect("an interval")
    };
    ValueType::Rational(
        RationalDomain::new(interval(-100, 100), interval(1, 100)).expect("a domain"),
    )
}

/// `e.x`.
fn x() -> Expression {
    Expression::field(
        Expression::deref(Expression::name("e".to_owned())),
        "x".to_owned(),
    )
}

/// `6 / e.x`.
fn six_over_x() -> Expression {
    Expression::binary(BinaryOperator::Divide, Expression::integer(6_i64), x())
}

/// `6 / e.x > e.one`, where `one` is the field that holds 1: a checked
/// ordering of rationals takes its type from an operand that is not a
/// literal.
fn six_over_x_above_one() -> Expression {
    Expression::binary(
        BinaryOperator::Greater,
        six_over_x(),
        Expression::field(
            Expression::deref(Expression::name("e".to_owned())),
            "one".to_owned(),
        ),
    )
}

/// `e.x != 0`.
fn x_is_nonzero() -> Expression {
    Expression::binary(BinaryOperator::NotEqual, x(), Expression::integer(0_i64))
}

fn division_by_zero() -> AtomReading {
    AtomReading::Undefined(UndefinedCause::Kernel(Undefined::DivisionByZero))
}

fn atom_of(
    kind: AggregateKind,
    window: PastWindow,
    min_count: u64,
    operator: ComparisonOperator,
    threshold: Rational,
) -> AggregateAtom {
    AggregateAtom::new(term(kind, window, min_count), operator, threshold)
}

/// FR-194-AC-5: over `x` values `2, 1, 0, 4`, read from the observed
/// objects by the production source, an undefined value, filter or
/// predicate makes the atom undefined with its cause at the first undefined
/// position, ahead of any sum and of `min_count`; a guarded filter keeps it
/// defined; the clause is undefined at the first position whose atom is, in
/// the violation category; and `InsufficientData` stays O-16 undefined.
#[trace("TC-629", "FR-194-AC-5")]
#[test]
fn an_undefined_value_filter_or_predicate_stops_the_window_at_its_first_position() {
    let observed = Observed::new(&[2, 1, 0, 4]);
    let value = observed.check(&six_over_x(), Some(&quotient()));
    let unguarded = ObservedWindowExpressions {
        value: Some(&value),
        ..ObservedWindowExpressions::default()
    };
    let trace = WindowTrace::positions(4);
    let positions = |a, b| PastWindow::Positions { a, b };
    let read_at = |atom: &AggregateAtom, expressions: ObservedWindowExpressions<'_>, position| {
        observed.read(atom, expressions, &trace, position)
    };

    // The clause is undefined where the atom first is: position 2, whose
    // window is the first to hold `x` 0.
    let sum = atom_of(
        AggregateKind::Sum,
        positions(0, 3),
        1,
        ComparisonOperator::LessOrEqual,
        whole(20),
    );
    let mut letters = Derived::new(&sum, &trace, observed.source(unguarded), UNLIMITED);
    let outcome = evaluate(
        &clause(),
        TemporalTrace::Finite { len: 4 },
        &mut letters,
        &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
    )
    .expect("the trace is readable");
    assert_eq!(
        outcome,
        TemporalOutcome::Undefined(UndefinedEvaluation {
            at: TemporalPosition::new(2),
            cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
        })
    );
    assert_eq!(outcome.category(), Category::Violation);

    // At position 3 the atom is undefined, not the sum 10.5 of the other
    // three and not a holding result.
    assert_eq!(read_at(&sum, unguarded, 3), division_by_zero());

    // The guarded sum stays defined: 3 + 6 + 3/2 = 21/2.
    let guard = observed.check(&x_is_nonzero(), None);
    let guarded = ObservedWindowExpressions {
        filter: Some(&guard),
        ..unguarded
    };
    assert_eq!(read_at(&sum, guarded, 3), AtomReading::True);
    let exactly = |threshold: Rational| {
        let atom = atom_of(
            AggregateKind::Sum,
            positions(0, 3),
            1,
            ComparisonOperator::GreaterOrEqual,
            threshold,
        );
        read_at(&atom, guarded, 3)
    };
    assert_eq!(exactly(ratio(21, 2)), AtomReading::True);
    assert_eq!(exactly(ratio(22, 2)), AtomReading::False);

    // An undefined filter, and an undefined predicate.
    let filter = observed.check(&six_over_x_above_one(), None);
    let count = atom_of(
        AggregateKind::Count,
        positions(0, 3),
        1,
        ComparisonOperator::GreaterOrEqual,
        whole(1),
    );
    let filtered = ObservedWindowExpressions {
        filter: Some(&filter),
        ..ObservedWindowExpressions::default()
    };
    assert_eq!(read_at(&count, filtered, 3), division_by_zero());
    let fraction = atom_of(
        AggregateKind::Fraction,
        positions(0, 3),
        1,
        ComparisonOperator::GreaterOrEqual,
        ratio(1, 2),
    );
    let predicated = ObservedWindowExpressions {
        predicate: Some(&filter),
        ..ObservedWindowExpressions::default()
    };
    assert_eq!(read_at(&fraction, predicated, 3), division_by_zero());

    // Undefined beats `min_count`; a guarded sum short of `min_count` is
    // `InsufficientData`, which is O-16 undefined and never a violation.
    let needs_five = |guard: ObservedWindowExpressions<'_>| {
        let atom = atom_of(
            AggregateKind::Sum,
            positions(0, 3),
            5,
            ComparisonOperator::LessOrEqual,
            whole(20),
        );
        read_at(&atom, guard, 3)
    };
    assert_eq!(needs_five(unguarded), division_by_zero());
    let short = AtomReading::Undefined(UndefinedCause::InsufficientData(InsufficientData {
        count: 3,
        min_count: 5,
    }));
    assert_eq!(needs_five(guarded), short);
    let atom = atom_of(
        AggregateKind::Sum,
        positions(0, 3),
        5,
        ComparisonOperator::LessOrEqual,
        whole(20),
    );
    let mut letters = Derived::new(&atom, &trace, observed.source(guarded), UNLIMITED);
    let outcome = evaluate(
        &clause(),
        TemporalTrace::Finite { len: 4 },
        &mut letters,
        &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
    )
    .expect("the trace is readable");
    assert!(matches!(
        &outcome,
        TemporalOutcome::Undefined(UndefinedEvaluation {
            cause: UndefinedCause::InsufficientData(_),
            ..
        })
    ));
    assert_eq!(outcome.category(), Category::Undefined);

    // `past[2, 3]` at position 3 holds positions 0 and 1 only, with no zero.
    let behind = positions(2, 3);
    let sum_behind = atom_of(
        AggregateKind::Sum,
        behind.clone(),
        1,
        ComparisonOperator::Equal,
        whole(9),
    );
    assert_eq!(read_at(&sum_behind, unguarded, 3), AtomReading::True);
    let count_behind = atom_of(
        AggregateKind::Count,
        behind.clone(),
        1,
        ComparisonOperator::Equal,
        whole(2),
    );
    assert_eq!(read_at(&count_behind, unguarded, 3), AtomReading::True);
    let fraction_behind = atom_of(
        AggregateKind::Fraction,
        behind,
        1,
        ComparisonOperator::Equal,
        whole(1),
    );
    assert_eq!(read_at(&fraction_behind, predicated, 3), AtomReading::True);
}

/// FR-194-AC-6: over `x` values `1, 0, 2, 3` the window is read oldest
/// position first and stops at position 1: the atom at position 3 is
/// undefined and charges 2 visit units (a newest-first visit would charge
/// 3), the clause is undefined at position 1 having charged 3 visit units
/// over positions 0 and 1, and `max_window_values` 2 does not turn the
/// atom `Incomplete`, because the third admitted value is not reached. A
/// filter that rejects 10 of 20 positions charges 20 visit units.
#[trace("TC-629", "FR-194-AC-6")]
#[test]
fn the_window_is_read_oldest_first_and_charges_only_the_positions_visited() {
    let sum = atom_of(
        AggregateKind::Sum,
        PastWindow::Positions { a: 0, b: 3 },
        1,
        ComparisonOperator::LessOrEqual,
        whole(20),
    );
    let rows = || {
        Rows::new(
            vec![true; 4],
            vec![whole(6), whole(0), whole(3), whole(2)],
            vec![false; 4],
        )
        .undefined_at(&[1])
    };
    let trace = WindowTrace::positions(4);

    for limit in [UNLIMITED, 2] {
        let mut source = rows();
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let reading = sum.evaluate(
            &trace,
            &mut source,
            limit,
            TemporalPosition::new(3),
            &mut meter,
        );
        assert_eq!(reading, Ok(division_by_zero()), "limit {limit}");
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 2, "limit {limit}");
        assert_eq!(source.visited, vec![0, 1], "limit {limit}");
    }

    let mut letters = Derived::new(&sum, &trace, rows(), UNLIMITED);
    let outcome = evaluate(
        &clause(),
        TemporalTrace::Finite { len: 4 },
        &mut letters,
        &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
    )
    .expect("the trace is readable");
    assert_eq!(
        outcome,
        TemporalOutcome::Undefined(UndefinedEvaluation {
            at: TemporalPosition::new(1),
            cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
        })
    );
    assert_eq!(
        letters.readings,
        vec![AtomReading::True, division_by_zero()]
    );
    assert_eq!(letters.window_units, 3);

    let wide = atom_of(
        AggregateKind::Sum,
        PastWindow::Positions { a: 0, b: 19 },
        1,
        ComparisonOperator::GreaterOrEqual,
        whole(0),
    );
    let half: Vec<bool> = (0..20).map(|position| position % 2 == 0).collect();
    let mut filtered = Rows::new(half, vec![whole(1); 20], vec![false; 20]);
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    wide.evaluate(
        &WindowTrace::positions(20),
        &mut filtered,
        20,
        TemporalPosition::new(19),
        &mut meter,
    )
    .expect("the window is within the limit");
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 20);
}
