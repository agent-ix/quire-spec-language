// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-234 (TC-689): the layer-5 reference evaluator of a clause under the
//! timed profile over a timed trace, whose positions carry exact rational
//! time stamps (QSpec FR-416's pointwise meaning).
//!
//! Each clause is real complete-V1 source over the `test/counters` model
//! `M`, taken through S1, S2, the assembler and the `TemporalTrace` family's
//! check, with `M` made timed (`dense`, unit `ms`) by its supplied time
//! source: no `time` member reaches S3 (FR-230's facet gate). The letter
//! reads `holds(c.level = k)` from the atom's source text over the trace's
//! `c.level` at each position.

use ix_trace_rs::trace;
use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::automaton::{translate_position_negation, Atom, Letter, TimedClaim};
use qsl_eval::simulation::model::ClockValuation;
use qsl_eval::temporal::{
    evaluate, evaluate_timed_at, AtomReading, LetterStop, PositionRefusal, TemporalLetters,
    TemporalOutcome, TemporalPosition, TemporalRefusal, TemporalTrace, TimedTrace, UndefinedCause,
};
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    check_clauses, Activation, CheckedTemporalClause, FormulaNode, FormulaNodeId, ModelTimeSource,
    PackageDeclarations, TemporalProfile, TimeSourceKind, SCALAR_LIMITS_UNLIMITED,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::key::hex;
use quire_exact::{Integer, Meter, Rational, ScalarLimits, Undefined};

use crate::model_simulation::{counters_document, increment, PACKAGE};
use crate::property_automata::accepts;

const VALUE: &str = "quire.value.complete/v1";
const TIMED: &str = "quire.temporal.timed/v1";

pub(crate) struct Fixture {
    text: String,
    pub(crate) clause: CheckedTemporalClause,
}

/// The clause `C` under the timed profile over `c: M::Counter`, `M` timed in
/// `ms`, whose formula is `body`.
pub(crate) fn timed(body: &str) -> Fixture {
    timed_on("on origin", body)
}

/// [`timed`] under `activation`.
pub(crate) fn timed_on(activation: &str, body: &str) -> Fixture {
    let document = counters_document(vec![increment()], false);
    let packages = package_input([document.as_slice()]);
    let digest = *packages.keys().next().expect("one model");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"{VALUE}\";\n\
         profile t0 = \"{TIMED}\";\n\
         model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         dimension Time;\n\
         unit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         temporal C using t0 over (c: M::Counter) clock \"model-time\" {activation} {{ {body} }}\n",
        hex(&digest)
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the model selection admits");
    let mut package = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles");
    package.time_sources = vec![ModelTimeSource {
        model: "M".to_owned(),
        span: qsl_foundation::Span { start: 0, end: 1 },
        model_node: quire_exact::NodeKey::from_digest([0; 32]),
        tick_operation_key: None,
        source: TimeSourceKind::Dense,
        unit: "ms".to_owned(),
        unit_node: quire_exact::NodeKey::from_digest([0; 32]),
        time_dimension: quire_exact::NodeKey::from_digest([0; 32]),
        nominal_nodes: Vec::new(),
        tick_operation_span: None,
        tick_operation_anchor: None,
    }];
    let mut checked = check_clauses(&package);
    let clause = match checked.pop() {
        Some(Ok(clause)) if checked.is_empty() => clause,
        other => panic!("{body}: one checked clause, not {other:?}"),
    };
    assert_eq!(clause.profile(), TemporalProfile::Timed);
    Fixture { text, clause }
}

/// Reads `holds(c.level = k)` over the trace's `c.level` at each position.
struct Levels<'a> {
    fixture: &'a Fixture,
    levels: Vec<i64>,
    /// The position whose atoms are undefined (division by zero), if any.
    undefined_at: Option<usize>,
}

impl TemporalLetters for Levels<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let Some(FormulaNode::Holds { span }) =
            self.fixture.clause.formula().nodes().get(atom.index())
        else {
            panic!("the letter reads only holds atoms");
        };
        let level: i64 = self.fixture.text[span.start..span.end]
            .strip_prefix("holds(c.level = ")
            .and_then(|rest| rest.strip_suffix(')'))
            .expect("a `holds(c.level = k)` atom")
            .parse()
            .expect("an integer level");
        let at = usize::try_from(position.index()).expect("a small position");
        if self.undefined_at == Some(at) {
            return Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            )));
        }
        Ok(if self.levels[at] == level {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        panic!("these clauses have no `when` condition");
    }
}

fn ms(value: u64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

/// The value of `body` at `position` of the trace with `stamps` (in `ms`)
/// and `c.level` `levels`.
fn at(body: &str, stamps: &[u64], levels: &[i64], position: u64) -> TemporalOutcome {
    let fixture = timed(body);
    let trace = TimedTrace::new(stamps.iter().copied().map(ms).collect()).expect("a timed trace");
    let mut letters = Levels {
        fixture: &fixture,
        levels: levels.to_vec(),
        undefined_at: None,
    };
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    evaluate_timed_at(
        &fixture.clause,
        &trace,
        TemporalPosition::new(position),
        &mut letters,
        &mut meter,
    )
    .expect("the trace is readable")
}

fn violated(position: u64) -> TemporalOutcome {
    TemporalOutcome::Violated {
        at: TemporalPosition::new(position),
    }
}

/// FR-234-AC-4: on stamps `0, 1, 3` with `q` (`c.level = 2`) only at
/// position 2, `eventually[0 ms, 3 ms] holds(q)` at 0 is true and
/// `eventually[0 ms, 3 ms) holds(q)` at 0 false; with `p` (`c.level = 1`)
/// only at position 1, `once(1 ms, 2 ms] holds(p)` at 2 is true. Inserting a
/// step that repeats position 1's discrete state at stamp 1 leaves each
/// value unchanged.
#[trace("TC-689", "FR-234-AC-4")]
#[test]
fn timed_operators_read_stamp_differences_with_each_ends_openness() {
    let closed = "eventually[0 ms, 3 ms] holds(c.level = 2)";
    let open = "eventually[0 ms, 3 ms) holds(c.level = 2)";
    let once = "once(1 ms, 2 ms] holds(c.level = 1)";
    let (stamps, levels) = ([0, 1, 3], [0, 1, 2]);
    assert_eq!(at(closed, &stamps, &levels, 0), TemporalOutcome::Holds);
    assert_eq!(at(open, &stamps, &levels, 0), violated(0));
    assert_eq!(at(once, &stamps, &levels, 2), TemporalOutcome::Holds);

    // Position 1 repeated at stamp 1: the old position 2 is now 3.
    let (stamps, levels) = ([0, 1, 1, 3], [0, 1, 1, 2]);
    assert_eq!(at(closed, &stamps, &levels, 0), TemporalOutcome::Holds);
    assert_eq!(at(open, &stamps, &levels, 0), violated(0));
    assert_eq!(at(once, &stamps, &levels, 3), TemporalOutcome::Holds);
}

/// FR-234-AC-3: the unbounded `always (holds(p) implies eventually[2 ms,
/// 2 ms] holds(q))`, which a timed subject does not admit, evaluates on a
/// supplied timed trace, punctual interval included: a `p` with no `q`
/// exactly 2 ms later on a trace that has passed that instant is a
/// violation, and with the `q` there the prefix decides nothing.
#[trace("TC-689", "FR-234-AC-3")]
#[test]
fn a_punctual_unbounded_claim_evaluates_on_a_timed_trace() {
    let body = "always (holds(c.level = 1) implies eventually[2 ms, 2 ms] holds(c.level = 2))";
    assert_eq!(at(body, &[0, 1, 3], &[1, 0, 2], 0), violated(0));
    assert_eq!(
        at(body, &[0, 2, 3], &[1, 2, 0], 0),
        TemporalOutcome::Pending
    );
}

/// FR-234 "Pointwise meaning": `until`, `since` and the duals read their
/// left operand over the positions between; a future window still reaching
/// past the last stamp leaves the prefix undecided, and one that closed
/// before it decides.
#[trace("TC-689", "FR-234")]
#[test]
fn timed_until_since_and_the_duals_read_the_positions_between() {
    let (stamps, levels) = ([0, 1, 2, 4], [1, 1, 2, 0]);
    let until = "holds(c.level = 1) until[1 ms, 2 ms] holds(c.level = 2)";
    assert_eq!(at(until, &stamps, &levels, 0), TemporalOutcome::Holds);
    // The witness at 2 ms is outside `[0 ms, 1 ms]` and the window closed.
    let early = "holds(c.level = 1) until[0 ms, 1 ms] holds(c.level = 2)";
    assert_eq!(at(early, &stamps, &levels, 0), violated(0));
    let since = "holds(c.level = 2) since[2 ms, 3 ms] holds(c.level = 1)";
    // At 4 ms: `c.level = 1` at 0 ms (4 ms back) is outside, at 1 ms (3 ms
    // back) inside, but position 3 breaks the `c.level = 2` run.
    assert_eq!(at(since, &stamps, &levels, 3), violated(3));
    assert_eq!(
        at(
            "historically[0 ms, 1 ms] holds(c.level = 1)",
            &stamps,
            &levels,
            1
        ),
        TemporalOutcome::Holds
    );
    // `always[0 ms, 5 ms]` reaches past the last stamp: the prefix holds it
    // so far and decides nothing.
    assert_eq!(
        at("always[0 ms, 5 ms] holds(c.level = 1)", &[0, 1], &[1, 1], 0),
        TemporalOutcome::Pending
    );
    assert_eq!(
        at(
            "always[0 ms, 1 ms] holds(c.level = 1)",
            &[0, 1, 2],
            &[1, 1, 0],
            0
        ),
        TemporalOutcome::Holds
    );
    assert_eq!(
        at("eventually holds(c.level = 3)", &[0, 1], &[1, 1], 0),
        TemporalOutcome::Pending
    );
}

/// A timed trace refuses an empty, negative or decreasing stamp sequence;
/// the timed evaluator refuses a position past the trace, and the step
/// evaluator refuses a timed clause.
#[trace("TC-689", "FR-234")]
#[test]
fn timed_traces_and_timed_clauses_are_read_by_the_timed_evaluator_only() {
    assert_eq!(
        TimedTrace::new(Vec::new()),
        Err(TemporalRefusal::EmptyTrace)
    );
    assert_eq!(
        TimedTrace::new(vec![ms(0), ms(2), ms(1)]),
        Err(TemporalRefusal::DecreasingStamp {
            position: TemporalPosition::new(2)
        })
    );
    assert_eq!(
        TimedTrace::new(vec![Rational::from_integer(Integer::from(-1_i64))]),
        Err(TemporalRefusal::NegativeStamp {
            position: TemporalPosition::new(0)
        })
    );
    let fixture = timed("eventually[0 ms, 3 ms] holds(c.level = 2)");
    let trace = TimedTrace::new(vec![ms(0)]).expect("a timed trace");
    let mut letters = Levels {
        fixture: &fixture,
        levels: vec![0],
        undefined_at: None,
    };
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    assert_eq!(
        evaluate_timed_at(
            &fixture.clause,
            &trace,
            TemporalPosition::new(1),
            &mut letters,
            &mut meter
        ),
        Err(TemporalRefusal::Position(PositionRefusal::OutsideTrace {
            position: TemporalPosition::new(1)
        }))
    );
    assert_eq!(
        evaluate(
            &fixture.clause,
            TemporalTrace::Finite { len: 1 },
            &mut letters,
            &mut meter
        ),
        Err(TemporalRefusal::ProfileNotEvaluated {
            profile: TemporalProfile::Timed
        })
    );
}

/// The value of `body` at `position` of the timed lasso with `stamps` (in
/// `ms`), `c.level` `levels`, re-entering position `loop_start` after
/// `delay` ms.
fn lasso_at(
    body: &str,
    stamps: &[u64],
    levels: &[i64],
    loop_start: u64,
    delay: u64,
    position: u64,
) -> TemporalOutcome {
    let fixture = timed(body);
    let trace = TimedTrace::lasso(
        stamps.iter().copied().map(ms).collect(),
        loop_start,
        ms(delay),
    )
    .expect("a timed lasso");
    let mut letters = Levels {
        fixture: &fixture,
        levels: levels.to_vec(),
        undefined_at: None,
    };
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    evaluate_timed_at(
        &fixture.clause,
        &trace,
        TemporalPosition::new(position),
        &mut letters,
        &mut meter,
    )
    .expect("the lasso is readable")
}

/// FR-231 "Evaluation": a position past the represented trace is the loop
/// position it wraps to, with stamps growing by the loop's total delay. On
/// stamps `0, 1` (levels `0, 2`) looping to 0 with delay 2, `c.level = 2`
/// holds at 1, 3, 5, ... ms, so `eventually[5 ms, 5 ms]` of it holds at
/// position 0 and not at 1, where the finite prefix decides neither.
#[trace("TC-686", "FR-231")]
#[test]
fn a_timed_lasso_grows_its_stamps_by_the_loops_delay() {
    let body = "eventually[5 ms, 5 ms] holds(c.level = 2)";
    assert_eq!(
        lasso_at(body, &[0, 1], &[0, 2], 0, 2, 0),
        TemporalOutcome::Holds
    );
    assert_eq!(lasso_at(body, &[0, 1], &[0, 2], 0, 2, 1), violated(1));
    assert_eq!(at(body, &[0, 1], &[0, 2], 0), TemporalOutcome::Pending);
    // Position 5 wraps to position 1, and position 4 to position 0.
    assert_eq!(lasso_at(body, &[0, 1], &[0, 2], 0, 2, 5), violated(5));
    assert_eq!(
        lasso_at(body, &[0, 1], &[0, 2], 0, 2, 4),
        TemporalOutcome::Holds
    );
}

/// FR-231 "Evaluation": an unbounded operator over a lasso is decided, not
/// left to the prefix. `always eventually holds(c.level = 2)` holds when
/// the loop contains the level and is violated when it does not, and a
/// level seen only before the loop is no help.
#[trace("TC-686", "FR-231")]
#[test]
fn an_unbounded_claim_is_decided_on_a_timed_lasso() {
    let body = "always eventually holds(c.level = 2)";
    assert_eq!(
        lasso_at(body, &[0, 1, 2], &[0, 2, 0], 1, 3, 0),
        TemporalOutcome::Holds
    );
    assert_eq!(lasso_at(body, &[0, 1, 2], &[2, 0, 0], 1, 3, 0), violated(0));
    // `until` needs its guard on every position before the witness.
    let until = "holds(c.level = 0) until holds(c.level = 2)";
    assert_eq!(
        lasso_at(until, &[0, 1, 2], &[0, 0, 2], 0, 4, 0),
        TemporalOutcome::Holds
    );
    assert_eq!(
        lasso_at(until, &[0, 1, 2], &[0, 1, 2], 0, 4, 0),
        violated(0)
    );
}

/// FR-231 "Evaluation": a past operator over a lasso reads the unrolled
/// history. On stamps `0, 1` (levels `0, 2`) looping to 0 with delay 2,
/// `always once holds(c.level = 2)` fails at position 0, which has not seen
/// the level yet, and holds from position 1 on, repetitions included.
#[trace("TC-686", "FR-231")]
#[test]
fn a_past_operator_reads_the_unrolled_loop() {
    let body = "always once holds(c.level = 2)";
    assert_eq!(lasso_at(body, &[0, 1], &[0, 2], 0, 2, 0), violated(0));
    assert_eq!(
        lasso_at(body, &[0, 1], &[0, 2], 0, 2, 1),
        TemporalOutcome::Holds
    );
    assert_eq!(
        lasso_at(body, &[0, 1], &[0, 2], 0, 2, 7),
        TemporalOutcome::Holds
    );
}

/// A timed lasso refuses a loop start that is not a represented position,
/// a loop of no positive delay, and a delay that re-enters before the last
/// represented stamp.
#[trace("TC-686", "FR-231")]
#[test]
fn a_timed_lasso_refuses_a_loop_that_is_not_time_divergent() {
    let stamps = || vec![ms(0), ms(3)];
    assert_eq!(
        TimedTrace::lasso(stamps(), 2, ms(1)),
        Err(TemporalRefusal::EmptyLoop)
    );
    assert_eq!(
        TimedTrace::lasso(stamps(), 0, ms(0)),
        Err(TemporalRefusal::NonPositiveLoopDelay)
    );
    assert_eq!(
        TimedTrace::lasso(stamps(), 0, ms(2)),
        Err(TemporalRefusal::LoopReentersBeforeLastStamp)
    );
    assert!(TimedTrace::lasso(stamps(), 0, ms(3))
        .expect("a loop that re-enters at the last stamp")
        .is_lasso());
}

/// One run of `body` on a timed lasso, with the stamps in `ms`, a loop
/// delay of `delay`, the work budget `limits` and an optional undefined atom.
fn lasso_run(
    body: &str,
    (stamps, levels): (&[u64], &[i64]),
    (loop_start, delay): (u64, Rational),
    position: u64,
    limits: ScalarLimits,
    undefined_at: Option<usize>,
) -> Result<TemporalOutcome, TemporalRefusal> {
    let fixture = timed(body);
    let trace = TimedTrace::lasso(stamps.iter().copied().map(ms).collect(), loop_start, delay)
        .expect("a timed lasso");
    let mut letters = Levels {
        fixture: &fixture,
        levels: levels.to_vec(),
        undefined_at,
    };
    let mut meter = Meter::new(limits);
    evaluate_timed_at(
        &fixture.clause,
        &trace,
        TemporalPosition::new(position),
        &mut letters,
        &mut meter,
    )
}

/// SR-12265 FND-001: positions of one loop that share a stamp are separate
/// classes of the tail scan. On stamps `0, 0, 0` (levels `0, 2, 0`) looping
/// to 0 with delay 1 ms, the level occurs at every repetition, so
/// `always eventually` of it holds at 0, `eventually` of it holds at 5, and
/// `eventually (0 ms, 1 ms]` of it holds at 0 (the open lower end drops
/// offset 0 and keeps the next repetition).
#[trace("TC-686", "FR-231")]
#[test]
fn a_tail_scan_reads_every_position_that_shares_a_stamp() {
    let run = |body, position| lasso_at(body, &[0, 0, 0], &[0, 2, 0], 0, 1, position);
    assert_eq!(
        run("always eventually holds(c.level = 2)", 0),
        TemporalOutcome::Holds
    );
    assert_eq!(
        run("eventually holds(c.level = 2)", 5),
        TemporalOutcome::Holds
    );
    assert_eq!(
        run("eventually(0 ms, 1 ms] holds(c.level = 2)", 0),
        TemporalOutcome::Holds
    );
}

/// A bounded `until` whose witness lies past the represented trace reads
/// its witness and its guard through the tail: on stamps `0, 1, 2` (levels
/// `2, 0, 0`) looping with delay 3, the level is at 0, 3, 6, ... ms, so from
/// position 1 its first occurrence is 2 ms ahead, past the last position.
#[trace("TC-686", "FR-231")]
#[test]
fn a_bounded_until_reads_its_window_in_the_tail() {
    let until = |body| lasso_at(body, &[0, 1, 2], &[2, 0, 0], 0, 3, 1);
    assert_eq!(
        until("holds(c.level = 0) until[2 ms, 2 ms] holds(c.level = 2)"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        until("holds(c.level = 0) until[3 ms, 3 ms] holds(c.level = 2)"),
        violated(1)
    );
}

/// A work budget that runs out while a lasso is unrolled or scanned is
/// `Incomplete`, not a truth value.
#[trace("TC-686", "FR-231")]
#[test]
fn a_timed_lasso_stops_when_the_meter_runs_out() {
    let limits = ScalarLimits {
        work_units: 3,
        ..SCALAR_LIMITS_UNLIMITED
    };
    let outcome = lasso_run(
        "always eventually holds(c.level = 2)",
        (&[0, 1], &[0, 2]),
        (0, ms(2)),
        0,
        limits,
        None,
    )
    .expect("the lasso is readable");
    assert!(
        matches!(outcome, TemporalOutcome::Incomplete(_)),
        "{outcome:?}"
    );
}

/// A loop delay so small that the unrolling count overflows `u64` refuses
/// `TraceTooLong` instead of allocating.
#[trace("TC-686", "FR-231")]
#[test]
fn a_timed_lasso_refuses_an_unrolling_that_overflows() {
    let tiny = Rational::new(
        Integer::one(),
        Integer::from(10_u64).pow(&Integer::from(30_u64)),
    )
    .expect("a non-zero denominator");
    assert_eq!(
        lasso_run(
            "eventually[0 ms, 3 ms] holds(c.level = 2)",
            (&[0], &[0]),
            (0, tiny),
            0,
            SCALAR_LIMITS_UNLIMITED,
            None,
        ),
        Err(TemporalRefusal::TraceTooLong)
    );
}

/// An atom undefined at a represented position leaves that position and
/// every later one, repetitions included, without a value: the claim is
/// `Undefined` there, not decided by the repeated earlier positions.
#[trace("TC-686", "FR-231")]
#[test]
fn an_undefined_atom_on_a_timed_lasso_is_reported_at_its_position() {
    let outcome = lasso_run(
        "always eventually holds(c.level = 2)",
        (&[0, 1, 2], &[0, 2, 0]),
        (0, ms(3)),
        0,
        SCALAR_LIMITS_UNLIMITED,
        Some(1),
    )
    .expect("the lasso is readable");
    assert!(
        matches!(outcome, TemporalOutcome::Undefined(ref undefined) if undefined.at == TemporalPosition::new(1)),
        "{outcome:?}"
    );
}

/// FR-240's claim automaton, `translate_position_negation`, answers to the
/// timed evaluator: for each timed-profile body with no interval, over
/// every lasso of 1 to 4 positions with `c.level` in `0..=2` and stamps
/// one `ms` apart, its loop re-entered 1 `ms` after its last position, the
/// automaton of the negation
/// accepts the lasso exactly when the evaluator finds the clause violated
/// at position 0.
#[trace("TC-695", "FR-240")]
#[test]
fn the_position_negation_answers_to_the_timed_evaluator() {
    let bodies = [
        "always holds(c.level = 1)",
        "eventually holds(c.level = 1)",
        "always eventually holds(c.level = 1)",
        "eventually always holds(c.level = 1)",
        "always (holds(c.level = 1) implies eventually holds(c.level = 2))",
        "holds(c.level = 1) until holds(c.level = 2)",
        "holds(c.level = 1) release holds(c.level = 2)",
        "always (holds(c.level = 2) implies once holds(c.level = 1))",
        "eventually (holds(c.level = 1) and always not holds(c.level = 2))",
    ];
    for body in bodies {
        let fixture = timed(body);
        let mut property = translate_position_negation(&fixture.clause, 1 << 16)
            .unwrap_or_else(|why| panic!("{body}: {why}"));
        let gba = property
            .explore()
            .unwrap_or_else(|why| panic!("{body}: {why}"));
        // The level each atom of the automaton's letter reads.
        let atom_levels: Vec<i64> = property
            .atoms()
            .iter()
            .map(|node| {
                let Some(FormulaNode::Holds { span }) =
                    fixture.clause.formula().nodes().get(node.index())
                else {
                    panic!("{body}: an atom is a holds atom");
                };
                fixture.text[span.start..span.end]
                    .strip_prefix("holds(c.level = ")
                    .and_then(|rest| rest.strip_suffix(')'))
                    .and_then(|level| level.parse().ok())
                    .expect("a `holds(c.level = k)` atom")
            })
            .collect();
        let (mut compared, mut violated) = (0, 0);
        for length in 1..=4_u32 {
            for code in 0..3_u32.pow(length) {
                let levels: Vec<i64> = (0..length)
                    .map(|position| i64::from(code / 3_u32.pow(position) % 3))
                    .collect();
                let word: Vec<Letter> = levels
                    .iter()
                    .map(|level| {
                        atom_levels
                            .iter()
                            .enumerate()
                            .filter(|(_, atom)| *atom == level)
                            .fold(Letter::default(), |letter, (index, _)| {
                                letter.with(Atom::new(index).expect("a small atom"))
                            })
                    })
                    .collect();
                let stamps: Vec<u64> = (0..u64::from(length)).collect();
                for loop_start in 0..length {
                    let trace = TimedTrace::lasso(
                        stamps.iter().copied().map(ms).collect(),
                        u64::from(loop_start),
                        ms(u64::from(length - loop_start)),
                    )
                    .expect("a timed lasso");
                    let mut letters = Levels {
                        fixture: &fixture,
                        levels: levels.clone(),
                        undefined_at: None,
                    };
                    let outcome = evaluate_timed_at(
                        &fixture.clause,
                        &trace,
                        TemporalPosition::new(0),
                        &mut letters,
                        &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
                    )
                    .expect("the lasso is readable");
                    let refuted = match outcome {
                        TemporalOutcome::Holds => false,
                        TemporalOutcome::Violated { .. } => true,
                        other => panic!("{body} over {levels:?}: {other:?}"),
                    };
                    let loop_start = usize::try_from(loop_start).expect("a small position");
                    assert_eq!(
                        accepts(&gba, &word, loop_start),
                        refuted,
                        "{body} over {levels:?} looping to {loop_start}"
                    );
                    compared += 1;
                    violated += usize::from(refuted);
                }
            }
        }
        assert_eq!(compared, 426);
        assert!(
            violated > 0 && violated < compared,
            "{body}: {violated} violated"
        );
    }
}

/// A deterministic stream of pseudo-random numbers (xorshift64).
struct Stream(u64);

impl Stream {
    fn below(&mut self, bound: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % bound
    }

    fn pick<'a>(&mut self, choices: &[&'a str]) -> &'a str {
        choices[usize::try_from(self.below(choices.len() as u64)).expect("small")]
    }
}

/// An interval for a random operator: under a future operator one starting
/// at a closed 0, or none; otherwise any.
fn random_interval(stream: &mut Stream, nested: bool) -> &'static str {
    if nested {
        stream.pick(&["", "[0 ms, 1 ms]", "[0 ms, 2 ms)", "[0 ms, 0 ms]"])
    } else {
        stream.pick(&[
            "",
            "[0 ms, 1 ms]",
            "[1 ms, 2 ms]",
            "(0 ms, 1 ms]",
            "(1 ms, 2 ms)",
            "[1 ms, 1 ms]",
        ])
    }
}

/// A random past-only formula over `p` and `q`.
fn random_past(stream: &mut Stream, depth: u32, nested: bool) -> String {
    if depth == 0 || stream.below(3) == 0 {
        return stream
            .pick(&[
                "holds(c.level = 1)",
                "holds(c.level = 2)",
                "not holds(c.level = 1)",
            ])
            .to_owned();
    }
    let interval = random_interval(stream, nested);
    match stream.below(4) {
        0 => format!(
            "once{interval} ({})",
            random_past(stream, depth - 1, nested)
        ),
        1 => format!(
            "historically{interval} ({})",
            random_past(stream, depth - 1, nested)
        ),
        2 => format!(
            "({}) since{interval} ({})",
            random_past(stream, depth - 1, nested),
            random_past(stream, depth - 1, nested)
        ),
        _ => format!(
            "({}) or ({})",
            random_past(stream, depth - 1, nested),
            random_past(stream, depth - 1, nested)
        ),
    }
}

/// A random timed formula over `p` (`c.level = 1`) and `q` (`c.level =
/// 2`); `nested` under a future operator.
fn random_formula(stream: &mut Stream, depth: u32, nested: bool) -> String {
    if depth == 0 || stream.below(4) == 0 {
        return match stream.below(4) {
            0 => random_past(stream, 1, nested),
            1 => "not holds(c.level = 2)".to_owned(),
            2 => "holds(c.level = 1)".to_owned(),
            _ => "holds(c.level = 2)".to_owned(),
        };
    }
    let interval = random_interval(stream, nested);
    let sub = |stream: &mut Stream| random_formula(stream, depth - 1, true);
    match stream.below(8) {
        0 => format!("eventually{interval} ({})", sub(stream)),
        1 => format!("always{interval} ({})", sub(stream)),
        2 => format!("({}) until{interval} ({})", sub(stream), sub(stream)),
        3 => format!("({}) release{interval} ({})", sub(stream), sub(stream)),
        4 => format!(
            "({}) and ({})",
            random_formula(stream, depth - 1, nested),
            random_formula(stream, depth - 1, nested)
        ),
        5 => format!(
            "({}) or ({})",
            random_formula(stream, depth - 1, nested),
            random_formula(stream, depth - 1, nested)
        ),
        6 => format!(
            "({}) implies ({})",
            random_formula(stream, depth - 1, nested),
            random_formula(stream, depth - 1, nested)
        ),
        _ => format!("not ({})", random_formula(stream, depth - 1, nested)),
    }
}

fn quarters(value: u64) -> Rational {
    Rational::new(Integer::from(value), Integer::from(4_u64)).expect("a rational")
}

/// Whether `claim` accepts the timed lasso of `stamps` (quarters of a ms),
/// letters `word`, re-entering `loop_start` `period` quarters after it:
/// some reachable cycle of its run graph visits every acceptance set. Clock
/// values past `ceiling` are held at `ceiling + 1`, which every guard reads
/// alike.
fn claim_accepts(
    claim: &mut TimedClaim,
    stamps: &[u64],
    word: &[Letter],
    loop_start: usize,
    period: u64,
) -> bool {
    let ceiling = claim
        .all_constants()
        .into_iter()
        .max()
        .unwrap_or_else(|| Rational::from_integer(Integer::zero()));
    let clocks = claim.clock_count();
    // Clock values in quarters of a ms.
    let after = |position: usize| -> (usize, u64) {
        if position + 1 < stamps.len() {
            (position + 1, stamps[position + 1] - stamps[position])
        } else {
            (loop_start, stamps[loop_start] + period - stamps[position])
        }
    };
    type Key = (usize, usize, Vec<u64>);
    let start = claim.initial().expect("the initial state");
    let mut numbers: BTreeMap<Key, usize> = BTreeMap::new();
    let mut nodes: Vec<Key> = Vec::new();
    let mut edges: Vec<Vec<usize>> = Vec::new();
    let first = (0, start.index(), vec![0_u64; clocks]);
    numbers.insert(first.clone(), 0);
    nodes.push(first);
    edges.push(Vec::new());
    let mut at = 0;
    while at < nodes.len() {
        let (position, q, values) = nodes[at].clone();
        let state = claim.reached_state(q).expect("a reached state");
        let valuation = ClockValuation::new(values.iter().copied().map(quarters).collect());
        for edge in claim
            .successors(state, word[position])
            .expect("the automaton reads the letter")
        {
            if !edge
                .guard
                .iter()
                .all(|constraint| constraint.holds(&valuation))
            {
                continue;
            }
            let (next, delay) = after(position);
            let moved: Vec<u64> = values
                .iter()
                .enumerate()
                .map(|(clock, value)| {
                    let base = if edge.resets.contains(&clock) {
                        0
                    } else {
                        *value
                    };
                    let advanced = base + delay;
                    // Past the ceiling every guard reads a value alike.
                    if quarters(advanced) > ceiling {
                        (0..).find(|q| quarters(*q) > ceiling).unwrap_or(advanced)
                    } else {
                        advanced
                    }
                })
                .collect();
            let key = (next, edge.target.index(), moved);
            let to = *numbers.entry(key.clone()).or_insert_with(|| {
                nodes.push(key);
                edges.push(Vec::new());
                nodes.len() - 1
            });
            edges[at].push(to);
        }
        at += 1;
    }
    // A cycle visiting every acceptance set: within one strongly connected
    // component of the reachable graph, by mutual reachability.
    let reach: Vec<BTreeSet<usize>> = (0..nodes.len())
        .map(|from| {
            let mut seen = BTreeSet::new();
            let mut stack = edges[from].clone();
            while let Some(node) = stack.pop() {
                if seen.insert(node) {
                    stack.extend(edges[node].iter().copied());
                }
            }
            seen
        })
        .collect();
    (0..nodes.len()).any(|root| {
        if !reach[root].contains(&root) {
            return false;
        }
        let component: Vec<usize> = reach[root]
            .iter()
            .copied()
            .filter(|node| reach[*node].contains(&root))
            .collect();
        (0..claim.acceptance_set_count()).all(|set| {
            component.iter().any(|&node| {
                let q = claim.reached_state(nodes[node].1).expect("a reached state");
                claim.is_accepting_in(set, q)
            })
        })
    })
}

/// FR-242 (TC-697): the timed claim automaton of the negation answers to
/// the evaluator. For random timed-profile formulas over `p` and `q` that
/// nest `eventually`, `always`, `until`, `release` and past operators (a
/// nested interval starting at a closed 0, any interval at the origin), on
/// random timed lassos of length at most 6 with stamps in steps of 1/4 ms,
/// the automaton accepts a lasso exactly when the evaluator returns
/// `false` on it.
#[trace("TC-697", "FR-242")]
#[test]
fn the_timed_claim_automaton_agrees_with_the_evaluator_on_random_lassos() {
    let mut stream = Stream(0x9e37_79b9_7f4a_7c15);
    let (mut formulas, mut held, mut violated) = (0, 0, 0);
    while formulas < 200 {
        let body = random_formula(&mut stream, 3, false);
        if ![
            "eventually",
            "always",
            "until",
            "release",
            "once",
            "historically",
            "since",
        ]
        .iter()
        .any(|operator| body.contains(operator))
        {
            continue;
        }
        let fixture = timed(&body);
        let mut claim = match TimedClaim::new(&fixture.clause, 1 << 20) {
            Ok(claim) => claim,
            Err(qsl_eval::automaton::AutomatonError::ProfileNotTranslated { .. }) => continue,
            Err(why) => panic!("{body}: {why}"),
        };
        formulas += 1;
        let atom_levels = levels_of(&fixture, claim.atoms());
        for _ in 0..80 {
            let length = usize::try_from(stream.below(6) + 1).expect("small");
            let loop_start = usize::try_from(stream.below(length as u64)).expect("small");
            let mut stamps = vec![0_u64];
            for _ in 1..length {
                let last = *stamps.last().expect("a stamp");
                stamps.push(last + stream.below(5));
            }
            let period = stamps[length - 1] - stamps[loop_start] + 1 + stream.below(4);
            let levels: Vec<i64> = (0..length)
                .map(|_| i64::try_from(stream.below(3)).expect("small"))
                .collect();
            let word: Vec<Letter> = levels
                .iter()
                .map(|level| letter_of(&atom_levels, *level))
                .collect();
            let trace = TimedTrace::lasso(
                stamps.iter().copied().map(quarters).collect(),
                u64::try_from(loop_start).expect("small"),
                quarters(period),
            )
            .expect("a timed lasso");
            let mut letters = Levels {
                fixture: &fixture,
                levels: levels.clone(),
                undefined_at: None,
            };
            let outcome = evaluate_timed_at(
                &fixture.clause,
                &trace,
                TemporalPosition::new(0),
                &mut letters,
                &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
            )
            .expect("the lasso is readable");
            let refuted = match outcome {
                TemporalOutcome::Holds => false,
                TemporalOutcome::Violated { .. } => true,
                other => panic!("{body}: {other:?}"),
            };
            assert_eq!(
                claim_accepts(&mut claim, &stamps, &word, loop_start, period),
                refuted,
                "{body} over stamps {stamps:?} (quarters), levels {levels:?}, loop to \
                 {loop_start} after {period}"
            );
            held += usize::from(!refuted);
            violated += usize::from(refuted);
        }
    }
    assert!(
        held > 100 && violated > 100,
        "{held} held, {violated} violated"
    );
}

/// FR-242-AC-1 (TC-697): for each of `eventually[0 ms, 3 ms] holds(q)`,
/// `eventually[0 ms, 3 ms) holds(q)`, `holds(p) until(1 ms, 2 ms] holds(q)`
/// and `always (holds(p) implies eventually[0 ms, 2 ms] holds(q))`, on
/// 1,000 random timed lassos with stamps in steps of 1/4 ms and length at
/// most 8, the timed claim automaton of the negation accepts exactly the
/// lassos on which the evaluator returns `false`.
#[trace("TC-697", "FR-242-AC-1")]
#[test]
fn the_mitl_vectors_agree_with_the_evaluator() {
    let mut stream = Stream(0x5eed_1234_abcd_ef01);
    for body in [
        "eventually[0 ms, 3 ms] holds(c.level = 2)",
        "eventually[0 ms, 3 ms) holds(c.level = 2)",
        "holds(c.level = 1) until(1 ms, 2 ms] holds(c.level = 2)",
        "always (holds(c.level = 1) implies eventually[0 ms, 2 ms] holds(c.level = 2))",
    ] {
        let fixture = timed(body);
        let mut claim = TimedClaim::new(&fixture.clause, 1 << 20).expect("the clause translates");
        let atom_levels = levels_of(&fixture, claim.atoms());
        let (mut held, mut violated) = (0, 0);
        for _ in 0..1_000 {
            let length = usize::try_from(stream.below(8) + 1).expect("small");
            let loop_start = usize::try_from(stream.below(length as u64)).expect("small");
            let mut stamps = vec![0_u64];
            for _ in 1..length {
                let last = *stamps.last().expect("a stamp");
                stamps.push(last + stream.below(5));
            }
            let period = stamps[length - 1] - stamps[loop_start] + 1 + stream.below(4);
            let levels: Vec<i64> = (0..length)
                .map(|_| i64::try_from(stream.below(3)).expect("small"))
                .collect();
            let word: Vec<Letter> = levels
                .iter()
                .map(|level| letter_of(&atom_levels, *level))
                .collect();
            let trace = TimedTrace::lasso(
                stamps.iter().copied().map(quarters).collect(),
                u64::try_from(loop_start).expect("small"),
                quarters(period),
            )
            .expect("a timed lasso");
            let mut letters = Levels {
                fixture: &fixture,
                levels: levels.clone(),
                undefined_at: None,
            };
            let refuted = match evaluate_timed_at(
                &fixture.clause,
                &trace,
                TemporalPosition::new(0),
                &mut letters,
                &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
            )
            .expect("the lasso is readable")
            {
                TemporalOutcome::Holds => false,
                TemporalOutcome::Violated { .. } => true,
                other => panic!("{body}: {other:?}"),
            };
            assert_eq!(
                claim_accepts(&mut claim, &stamps, &word, loop_start, period),
                refuted,
                "{body} over stamps {stamps:?} (quarters), levels {levels:?}, loop to \
                 {loop_start} after {period}"
            );
            held += usize::from(!refuted);
            violated += usize::from(refuted);
        }
        assert!(
            held > 0 && violated > 0,
            "{body}: {held} held, {violated} violated"
        );
    }
}

/// The level each atom of the letter reads, by atom.
fn levels_of(fixture: &Fixture, atoms: &[FormulaNodeId]) -> Vec<i64> {
    atoms
        .iter()
        .map(|node| {
            let Some(FormulaNode::Holds { span }) =
                fixture.clause.formula().nodes().get(node.index())
            else {
                panic!("an atom is a holds atom");
            };
            fixture.text[span.start..span.end]
                .strip_prefix("holds(c.level = ")
                .and_then(|rest| rest.strip_suffix(')'))
                .and_then(|level| level.parse().ok())
                .expect("a `holds(c.level = k)` atom")
        })
        .collect()
}

/// The letter of a position at `level`.
fn letter_of(atom_levels: &[i64], level: i64) -> Letter {
    atom_levels
        .iter()
        .enumerate()
        .filter(|(_, atom)| **atom == level)
        .fold(Letter::default(), |letter, (index, _)| {
            letter.with(Atom::new(index).expect("a small atom"))
        })
}

/// Whether the evaluator refutes `fixture`'s clause on the timed lasso: at
/// position 0 under `on origin`; under `on each`, at some position, read
/// through enough unrolled loop iterations that every past window of at
/// most 3 ms has settled.
fn evaluator_refutes(
    fixture: &Fixture,
    body: &str,
    trace: &TimedTrace,
    letters: &mut Levels<'_>,
    length: usize,
    loop_start: usize,
) -> bool {
    let each = matches!(fixture.clause.activation(), Activation::Each { .. });
    let last = if each {
        length + 16 * (length - loop_start)
    } else {
        1
    };
    (0..last).any(|position| {
        match evaluate_timed_at(
            &fixture.clause,
            trace,
            TemporalPosition::new(u64::try_from(position).expect("small")),
            letters,
            &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
        )
        .expect("the lasso is readable")
        {
            TemporalOutcome::Holds => false,
            TemporalOutcome::Violated { .. } => true,
            other => panic!("{body} at {position}: {other:?}"),
        }
    })
}

/// The automaton of `fixture` against the evaluator on `lassos` random
/// lassos of length at most `max_length`, as [`evaluator_refutes`] reads
/// them: the numbers held and violated.
fn agreement(
    fixture: &Fixture,
    body: &str,
    stream: &mut Stream,
    lassos: usize,
    max_length: u64,
) -> (usize, usize) {
    let mut claim = TimedClaim::new(&fixture.clause, 1 << 20).expect("the clause translates");
    let atom_levels = levels_of(fixture, claim.atoms());
    let (mut held, mut violated) = (0, 0);
    for _ in 0..lassos {
        let length = usize::try_from(stream.below(max_length) + 1).expect("small");
        let loop_start = usize::try_from(stream.below(length as u64)).expect("small");
        let mut stamps = vec![0_u64];
        for _ in 1..length {
            let last = *stamps.last().expect("a stamp");
            stamps.push(last + stream.below(5));
        }
        let period = stamps[length - 1] - stamps[loop_start] + 1 + stream.below(4);
        let levels: Vec<i64> = (0..length)
            .map(|_| i64::try_from(stream.below(3)).expect("small"))
            .collect();
        let word: Vec<Letter> = levels
            .iter()
            .map(|level| letter_of(&atom_levels, *level))
            .collect();
        let trace = TimedTrace::lasso(
            stamps.iter().copied().map(quarters).collect(),
            u64::try_from(loop_start).expect("small"),
            quarters(period),
        )
        .expect("a timed lasso");
        let mut letters = Levels {
            fixture,
            levels: levels.clone(),
            undefined_at: None,
        };
        let refuted = evaluator_refutes(fixture, body, &trace, &mut letters, length, loop_start);
        assert_eq!(
            claim_accepts(&mut claim, &stamps, &word, loop_start, period),
            refuted,
            "{body} over stamps {stamps:?} (quarters), levels {levels:?}, loop to \
             {loop_start} after {period}"
        );
        held += usize::from(!refuted);
        violated += usize::from(refuted);
    }
    (held, violated)
}

/// FR-242 (TC-697, SR-1650002 FND-001): the one-clock subsumption rules,
/// read where they decide. The negation of `eventually (p and
/// eventually[0 ms, 1 ms] q)` restarts `always[0, 1]` at every `p`, kept
/// once by the newer start; that of `eventually (p and (p until[0 ms,
/// 1 ms] q))` restarts `release[0, 1]` alike; that of `eventually (p and
/// always[0 ms, 1 ms] not q)` restarts `eventually[0, 1]`, kept by the older.
/// Under `on origin` and `on each`, on 1,000 random timed lassos each, the
/// automaton accepts exactly where the evaluator refutes the clause.
/// Keeping the older `release` instead fails the first two at once.
#[trace("TC-697", "FR-242")]
#[test]
fn the_subsumption_rules_answer_to_the_evaluator() {
    let mut stream = Stream(0x1650_5ab5_0000_0001);
    for activation in ["on origin", "on each (q: M::Counter)"] {
        for body in [
            "eventually (holds(c.level = 1) and eventually[0 ms, 1 ms] holds(c.level = 2))",
            "eventually (holds(c.level = 1) and \
             (holds(c.level = 1) until[0 ms, 1 ms] holds(c.level = 2)))",
            "eventually (holds(c.level = 1) and always[0 ms, 1 ms] not holds(c.level = 2))",
        ] {
            let fixture = timed_on(activation, body);
            let (held, violated) = agreement(&fixture, body, &mut stream, 1_000, 8);
            assert!(
                held > 0 && violated > 0,
                "{activation} {body}: {held} held, {violated} violated"
            );
        }
    }
}

/// FR-242 (TC-697, SR-1650002 FND-004): the agreement test under `on
/// each`, where every operator is read below the activating `until`. For
/// 100 random formulas, on 40 random timed lassos each, the automaton
/// accepts exactly where the evaluator refutes the clause at some position.
#[trace("TC-697", "FR-242")]
#[test]
fn the_timed_claim_automaton_agrees_under_each_activation() {
    let mut stream = Stream(0x1650_eac4_0000_0002);
    let (mut formulas, mut held, mut violated) = (0, 0, 0);
    while formulas < 100 {
        let body = random_formula(&mut stream, 3, true);
        let fixture = timed_on("on each (q: M::Counter)", &body);
        match TimedClaim::new(&fixture.clause, 1 << 20) {
            Ok(_) => {}
            Err(qsl_eval::automaton::AutomatonError::ProfileNotTranslated { .. }) => continue,
            Err(why) => panic!("{body}: {why}"),
        }
        formulas += 1;
        let (more_held, more_violated) = agreement(&fixture, &body, &mut stream, 40, 6);
        held += more_held;
        violated += more_violated;
    }
    assert!(
        held > 50 && violated > 50,
        "{held} held, {violated} violated"
    );
}
