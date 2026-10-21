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
use qsl_eval::temporal::{
    evaluate, evaluate_timed_at, AtomReading, LetterStop, PositionRefusal, TemporalLetters,
    TemporalOutcome, TemporalPosition, TemporalRefusal, TemporalTrace, TimedTrace,
};
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    check_clauses, CheckedTemporalClause, FormulaNode, FormulaNodeId, ModelTimeSource,
    PackageDeclarations, TemporalProfile, TimeSourceKind, SCALAR_LIMITS_UNLIMITED,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::key::hex;
use quire_exact::{Integer, Meter, Rational};

use crate::model_simulation::{counters_document, increment, PACKAGE};

const VALUE: &str = "quire.value.complete/v1";
const TIMED: &str = "quire.temporal.timed/v1";

struct Fixture {
    text: String,
    clause: CheckedTemporalClause,
}

/// The clause `C` under the timed profile over `c: M::Counter`, `M` timed in
/// `ms`, whose formula is `body`.
fn timed(body: &str) -> Fixture {
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
         temporal C using t0 over (c: M::Counter) clock \"model-time\" on origin {{ {body} }}\n",
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
        source: TimeSourceKind::Dense,
        unit: "ms".to_owned(),
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
