// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-327-AC-7 (TC-847): the precedence among a false activation, an
//! undefined evaluation and insufficient data across the activations of a
//! clause, over the closed `Counter` trace `c.level` 0, 1, 2 under
//! event-position false-extension.
//!
//! The letter reads each `holds` atom from its source text and the two
//! aggregate stand-ins by what the AC says of the aggregates: `A` (`count
//! where holds(c.value != 2) over past[0, 0] min_count 1 >= 1`) is true at
//! positions 0 and 1 and short of data (count 0, `min_count` 1) at 2; `B`
//! (`count over past[0, 9] min_count 2 >= 1`) is short of data (count 1,
//! `min_count` 2) at 0 and true at 1 and 2. This covers the evaluator's
//! precedence among those readings; the aggregates' own evaluation is
//! `window_aggregates`'s (FR-194).

use ix_trace_rs::trace;
use qsl_eval::temporal::{
    evaluate, AtomReading, InsufficientData, LetterStop, TemporalLetters, TemporalOutcome,
    TemporalPosition, TemporalTrace, UndefinedCause, UndefinedEvaluation,
};
use qsl_semantics::check::{FormulaNode, FormulaNodeId, SCALAR_LIMITS_UNLIMITED};
use quire_exact::{Meter, Undefined};
use quire_semantic_value::catalog::Category;

use crate::temporal_trace::{fixture, Fixture, EVENT};

/// `A` and `B` are `holds` atoms the letter reads as the aggregates: the
/// evaluator plans an aggregate node only from an emitted package
/// (`aggregate-formula-not-emitted`), and a model-subject `over` refuses one
/// at the check, so the precedence is exercised on the letter's readings.
const A: &str = "holds(c.level = 100)";
const B: &str = "holds(c.level = 200)";

struct Counters<'a> {
    fixture: &'a Fixture,
    levels: Vec<i64>,
}

impl Counters<'_> {
    /// The reading of the `holds` condition `inner` at `level`.
    fn holds(inner: &str, level: i64) -> AtomReading {
        let words: Vec<&str> = inner.split_whitespace().collect();
        let (value, operator, bound) = match words.as_slice() {
            ["c.level", operator, bound] => (level, *operator, *bound),
            [numerator, "mod", divisor, "-", variable, operator, bound] => {
                assert_eq!(*variable, "c.level)", "{inner}");
                let minuend: i64 = divisor.trim_start_matches('(').parse().expect("an integer");
                let divisor = minuend - level;
                if divisor == 0 {
                    return AtomReading::Undefined(UndefinedCause::Kernel(
                        Undefined::DivisionByZero,
                    ));
                }
                let numerator: i64 = numerator.parse().expect("an integer");
                (numerator.rem_euclid(divisor), *operator, *bound)
            }
            other => panic!("unmodelled atom {other:?}"),
        };
        let bound: i64 = bound.parse().expect("an integer bound");
        let truth = match operator {
            "=" => value == bound,
            "!=" => value != bound,
            "<=" => value <= bound,
            ">=" => value >= bound,
            other => panic!("unmodelled operator {other}"),
        };
        if truth {
            AtomReading::True
        } else {
            AtomReading::False
        }
    }
}

impl TemporalLetters for Counters<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let at = usize::try_from(position.index()).expect("a small position");
        let level = self.levels[at];
        match self.fixture.clause.formula().nodes().get(atom.index()) {
            Some(FormulaNode::Holds { span }) => {
                let source = &self.fixture.text[span.start..span.end];
                let (short, count, min_count) = match source {
                    A => (2, 0, 1),
                    B => (0, 1, 2),
                    _ => {
                        let inner = source
                            .strip_prefix("holds(")
                            .and_then(|rest| rest.strip_suffix(')'))
                            .expect("a holds atom");
                        return Ok(Self::holds(inner, level));
                    }
                };
                if at == short {
                    Ok(AtomReading::Undefined(UndefinedCause::InsufficientData(
                        InsufficientData { count, min_count },
                    )))
                } else {
                    Ok(AtomReading::True)
                }
            }
            other => panic!("the letter reads atoms only, not {other:?}"),
        }
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        panic!("these clauses have no `when` condition");
    }
}

fn settle(activation: &str, body: &str, levels: &[i64]) -> (Fixture, TemporalOutcome) {
    let fx = fixture(EVENT, "c: Counter", activation, body);
    let mut letters = Counters {
        fixture: &fx,
        levels: levels.to_vec(),
    };
    let len = u64::try_from(levels.len()).expect("small");
    let outcome = evaluate(
        &fx.clause,
        TemporalTrace::Finite { len },
        &mut letters,
        &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
    )
    .expect("the trace is readable");
    (fx, outcome)
}

const EACH: &str = "on each (q: Counter)";
const ORIGIN: &str = "on origin";

fn violated(at: u64) -> TemporalOutcome {
    TemporalOutcome::Violated {
        at: TemporalPosition::new(at),
    }
}

fn division_by_zero(at: u64) -> TemporalOutcome {
    TemporalOutcome::Undefined(UndefinedEvaluation {
        at: TemporalPosition::new(at),
        cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
    })
}

/// `outcome` is insufficient data at `at` for the aggregate `source`, with
/// `count` values against `min_count`, category undefined.
fn assert_insufficient(
    fx: &Fixture,
    outcome: &TemporalOutcome,
    at: u64,
    source: &str,
    count: u64,
    min_count: u64,
) {
    let TemporalOutcome::InsufficientData(short) = outcome else {
        panic!("insufficient data, not {outcome:?}");
    };
    assert_eq!(short.at, TemporalPosition::new(at));
    assert_eq!(short.data, InsufficientData { count, min_count });
    let Some(FormulaNode::Holds { span }) = fx.clause.formula().nodes().get(short.atom.index())
    else {
        panic!("the short atom is an atom node");
    };
    assert_eq!(&fx.text[span.start..span.end], source);
    assert_eq!(outcome.category(), Category::Undefined);
}

/// FR-327-AC-7: with `A` and `B` as above, `always[0,2] A` `on origin` is
/// `InsufficientData{where: 2, atom: A, count: 0, min_count: 1}`, category
/// undefined, neither a truth value nor `UndefinedEvaluation`; a false
/// activation outranks one that lacked data (the later false activation at 2
/// wins over the activation at 0 short of `B`); an `UndefinedEvaluation`
/// outranks `InsufficientData` whichever aggregate sits beside it; with no
/// activation false the unreadable one is `InsufficientData`, also when a
/// false `holds` atom sits beside the short `B` in the one letter; and on
/// the one-position trace with level 2 the undefined `holds` is reported in
/// either operand order.
#[trace("TC-847", "FR-327-AC-7")]
#[test]
fn a_false_activation_outranks_an_undefined_one_which_outranks_missing_data() {
    let trace = [0, 1, 2];
    let (fx, outcome) = settle(ORIGIN, &format!("always[0,2] ({A})"), &trace);
    assert_insufficient(&fx, &outcome, 2, A, 0, 1);

    let (_, outcome) = settle(
        EACH,
        &format!("always[0,0] (holds(c.level != 1) and ({A}))"),
        &trace,
    );
    assert_eq!(outcome, violated(1));
    let (_, outcome) = settle(
        EACH,
        &format!("always[0,0] (holds(c.level != 2) and ({B}))"),
        &trace,
    );
    assert_eq!(outcome, violated(2));

    let (_, outcome) = settle(
        EACH,
        &format!("always[0,0] (holds(3 mod (1 - c.level) >= 0) and ({A}))"),
        &trace,
    );
    assert_eq!(outcome, division_by_zero(1));
    let (_, outcome) = settle(
        EACH,
        &format!("always[0,0] (holds(3 mod (2 - c.level) >= 0) and ({B}))"),
        &trace,
    );
    assert_eq!(outcome, division_by_zero(2));

    let (fx, outcome) = settle(
        EACH,
        &format!("always[0,0] (holds(c.level <= 2) and ({B}))"),
        &trace,
    );
    assert_insufficient(&fx, &outcome, 0, B, 1, 2);
    let (fx, outcome) = settle(
        ORIGIN,
        &format!("always[0,2] (holds(c.level != 2) and ({B}))"),
        &trace,
    );
    assert_insufficient(&fx, &outcome, 0, B, 1, 2);
    let (fx, outcome) = settle(
        EACH,
        &format!("always[0,0] (holds(c.level != 0) and ({B}))"),
        &trace,
    );
    assert_insufficient(&fx, &outcome, 0, B, 1, 2);

    let (_, outcome) = settle(
        EACH,
        &format!("always[0,0] (holds(3 mod (2 - c.level) >= 0) and ({A}))"),
        &[2],
    );
    assert_eq!(outcome, division_by_zero(0));
    let (_, outcome) = settle(
        EACH,
        &format!("always[0,0] (({A}) and holds(3 mod (2 - c.level) >= 0))"),
        &[2],
    );
    assert_eq!(outcome, division_by_zero(0));
}
