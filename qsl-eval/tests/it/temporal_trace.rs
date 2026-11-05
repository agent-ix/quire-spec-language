// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-327, FR-328, FR-329, FR-125 (SM-8): the layer-5 TemporalTrace
//! evaluator over a closed finite trace, a finite prefix and an observed
//! lasso.
//!
//! Each clause is real complete-V1 source taken through S1, S2, the
//! assembler and the `TemporalTrace` family's check. The `holds` conditions
//! are not checked at S3, so the letter is a [`TemporalLetters`]
//! that reads each atom from its source text, the seam the one clause
//! evaluator (FR-107) fills.

use ix_trace_rs::trace;
use qsl_eval::temporal::{
    evaluate, evaluate_at, unrolling, AtomReading, CaptureReading, InsufficientData, LetterStop,
    PositionRefusal, TemporalLetters, TemporalOutcome, TemporalPosition, TemporalRefusal,
    TemporalTrace, UndefinedCause, Unrolling,
};
use qsl_foundation::diagnostic::StageFailure;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    check_clauses, CheckedTemporalClause, FormulaNode, FormulaNodeId, PackageDeclarations,
    SCALAR_LIMITS_UNLIMITED,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input, SelectedModels};
use quire_exact::{ChargePoint, LimitKind, Meter, ScalarLimits, Undefined};
use quire_semantic_value::catalog::CatalogCode;
use quire_semantic_value::catalog::Category;
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

use crate::model_simulation::{acquire, counters_document, increment, release, PACKAGE};

pub(crate) const EVENT: &str = "quire.temporal.event-position.false-extension/v1";
pub(crate) const INFINITE: &str = "quire.temporal.infinite-trace/v1";
const VALUE: &str = "quire.value.complete/v1";

/// A unit with one `temporal` clause `C` whose formula is `body`.
pub(crate) struct Fixture {
    pub(crate) text: String,
    pub(crate) clause: CheckedTemporalClause,
}

pub(crate) fn fixture(profile: &str, over: &str, activation: &str, body: &str) -> Fixture {
    fixture_within(profile, over, activation, body, qsl_cst::Limits::default())
}

/// [`fixture`] with the S1 `limits` the unit is read under.
pub(crate) fn fixture_within(
    profile: &str,
    over: &str,
    activation: &str,
    body: &str,
    limits: qsl_cst::Limits,
) -> Fixture {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"{VALUE}\";\n\
         profile t0 = \"{profile}\";\n\
         record Counter {{ level: Int[0, 9]; }}\n\
         record Config {{ versionNumber: Int[0, 9]; }}\n\
         temporal C using t0 over ({over}) clock \"model-steps\" {activation} {{ {body} }}\n"
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        limits,
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let package = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        built,
        SelectedModels::default(),
        Vec::new(),
    )
    .expect("the unit assembles");
    let mut checked = check_clauses(&package);
    let clause = match checked.pop() {
        Some(Ok(clause)) if checked.is_empty() => clause,
        Some(Err(StageFailure::Refused(refusal))) => panic!("{body}: refused: {refusal:?}"),
        other => panic!("{body}: one checked clause, not {other:?}"),
    };
    Fixture { text, clause }
}

/// A unit over the `test/counters` model `M` with one infinite-trace `temporal`
/// clause `C` over `c: M::Counter`, `on origin`, so a fairness constraint can
/// name the model operation `M::Counter::increment`.
pub(crate) fn modelled(body: &str) -> Fixture {
    modelled_with(vec![increment()], body)
}

/// [`modelled`] over a model that declares `operations`.
fn modelled_with(operations: Vec<serde_json::Value>, body: &str) -> Fixture {
    let document = counters_document(operations, false);
    let packages = package_input([document.as_slice()]).unwrap();
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"{VALUE}\";\n\
         profile t0 = \"{INFINITE}\";\n\
         model M = \"{PACKAGE}\";\n\
         temporal C using t0 over (c: M::Counter) clock \"model-steps\" on origin {{ {body} }}\n",
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u"),
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
    let package = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles");
    let mut checked = check_clauses(&package);
    let clause = match checked.pop() {
        Some(Ok(clause)) if checked.is_empty() => clause,
        other => panic!("{body}: one checked clause, not {other:?}"),
    };
    Fixture { text, clause }
}

/// `c.level` over `Counter`, `on origin`.
fn counter(profile: &str, body: &str) -> Fixture {
    fixture(profile, "c: Counter", "on origin", body)
}

/// `a.versionNumber` over `Config`, `on origin`, under infinite-trace.
fn config(body: &str) -> Fixture {
    fixture(INFINITE, "a: Config", "on origin", body)
}

/// One observation: the values the atoms of these tests read.
#[derive(Clone, Copy)]
struct Observation {
    c: i64,
    a: i64,
    b: i64,
}

fn counters(values: &[i64]) -> Vec<Observation> {
    values
        .iter()
        .map(|c| Observation { c: *c, a: 0, b: 0 })
        .collect()
}

fn versions(values: &[i64]) -> Vec<Observation> {
    values
        .iter()
        .map(|a| Observation { c: 0, a: *a, b: 0 })
        .collect()
}

/// A letter that reads each `holds(...)` atom from its source text over an
/// observation: `holds(<var> <op> <k>)` and
/// `holds(<n> / (2 - c.level) <op> <k>)` (integer division; a zero divisor is
/// the kernel's `DivisionByZero`).
struct Letters<'a> {
    fixture: &'a Fixture,
    observations: Vec<Observation>,
    reads: Vec<u64>,
    /// The positions whose step does not count for interval distance.
    uncounted: Vec<u64>,
    /// Whether the step that closes a lasso's loop does not count.
    uncounted_loop_edge: bool,
    /// The positions whose `holds` condition the S6a evaluation refuses.
    refused: Vec<u64>,
    /// The readings forced on the `holds` atom of the given source text at
    /// the given position, for a clause of several atoms.
    forced: Vec<(u64, &'static str, Forced)>,
    /// The positions whose `on each` `when` guard is refused.
    refused_guards: Vec<u64>,
    /// The positions whose capture initializer is refused.
    refused_captures: Vec<u64>,
}

/// What a test forces an atom to read.
#[derive(Clone, Copy)]
enum Forced {
    Refused,
    Undefined,
    ShortOfData,
}

impl<'a> Letters<'a> {
    fn new(fixture: &'a Fixture, observations: Vec<Observation>) -> Self {
        Self {
            fixture,
            observations,
            reads: Vec::new(),
            uncounted: Vec::new(),
            uncounted_loop_edge: false,
            refused: Vec::new(),
            forced: Vec::new(),
            refused_guards: Vec::new(),
            refused_captures: Vec::new(),
        }
    }

    fn reading(text: &str, observation: Observation) -> AtomReading {
        let inner = text
            .strip_prefix("holds(")
            .and_then(|rest| rest.strip_suffix(')'))
            .expect("a holds atom");
        let words: Vec<&str> = inner.split_whitespace().collect();
        let variable = |name: &str| match name {
            "c.level" => observation.c,
            "a.versionNumber" => observation.a,
            "b.versionNumber" => observation.b,
            other => panic!("unmodelled variable {other}"),
        };
        let (value, operator, bound) = match words.as_slice() {
            [name, operator, bound] => (variable(name), *operator, *bound),
            [numerator, "/", "(2", "-", denominator, operator, bound] => {
                let name = denominator.strip_suffix(')').expect("a closed divisor");
                let divisor = 2 - variable(name);
                if divisor == 0 {
                    return AtomReading::Undefined(UndefinedCause::Kernel(
                        Undefined::DivisionByZero,
                    ));
                }
                let numerator: i64 = numerator.parse().expect("an integer numerator");
                (numerator / divisor, *operator, *bound)
            }
            other => panic!("unmodelled atom {other:?}"),
        };
        let bound: i64 = bound.parse().expect("an integer bound");
        let holds = match operator {
            "=" => value == bound,
            "<=" => value <= bound,
            ">=" => value >= bound,
            ">" => value > bound,
            other => panic!("unmodelled operator {other}"),
        };
        if holds {
            AtomReading::True
        } else {
            AtomReading::False
        }
    }
}

impl TemporalLetters for Letters<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        self.reads.push(position.index());
        if self.refused.contains(&position.index()) {
            return Err(LetterStop::Refused);
        }
        let Some(FormulaNode::Holds { span }) =
            self.fixture.clause.formula().nodes().get(atom.index())
        else {
            panic!("the letter reads only holds atoms");
        };
        let text = &self.fixture.text[span.start..span.end];
        if let Some((_, _, forced)) = self
            .forced
            .iter()
            .find(|(at, atom, _)| *at == position.index() && *atom == text)
        {
            return match forced {
                Forced::Refused => Err(LetterStop::Refused),
                Forced::Undefined => Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                    Undefined::DivisionByZero,
                ))),
                Forced::ShortOfData => Ok(AtomReading::Undefined(
                    UndefinedCause::InsufficientData(InsufficientData {
                        count: 0,
                        min_count: 1,
                    }),
                )),
            };
        }
        let observation = *self
            .observations
            .get(usize::try_from(position.index()).expect("a small position"))
            .expect("the position is represented");
        Ok(Self::reading(text, observation))
    }

    fn activation(
        &mut self,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        if self.refused_guards.contains(&position.index()) {
            return Err(LetterStop::Refused);
        }
        Ok(AtomReading::True)
    }

    fn captures(
        &mut self,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<CaptureReading, LetterStop> {
        if self.refused_captures.contains(&position.index()) {
            return Err(LetterStop::Refused);
        }
        Ok(CaptureReading::Fixed)
    }

    fn counts_step(&self, position: TemporalPosition) -> bool {
        !self.uncounted.contains(&position.index())
    }

    fn counts_loop_edge(&self) -> bool {
        !self.uncounted_loop_edge
    }
}

fn limits(work_units: u64) -> ScalarLimits {
    ScalarLimits {
        work_units,
        ..SCALAR_LIMITS_UNLIMITED
    }
}

fn work(meter: &Meter) -> u64 {
    meter.consumed(LimitKind::WorkUnits)
}

/// The outcome of `fixture` on `trace` over `observations`, and the work it
/// consumed.
fn run(
    fixture: &Fixture,
    trace: TemporalTrace,
    observations: Vec<Observation>,
    budget: ScalarLimits,
) -> (TemporalOutcome, u64, Vec<u64>) {
    let mut letters = Letters::new(fixture, observations);
    let mut meter = Meter::new(budget);
    let outcome =
        evaluate(&fixture.clause, trace, &mut letters, &mut meter).expect("the trace is readable");
    (outcome, work(&meter), letters.reads)
}

fn outcome(
    fixture: &Fixture,
    trace: TemporalTrace,
    observations: Vec<Observation>,
) -> TemporalOutcome {
    run(fixture, trace, observations, SCALAR_LIMITS_UNLIMITED).0
}

fn finite(len: u64) -> TemporalTrace {
    TemporalTrace::Finite { len }
}

fn violated(index: u64) -> TemporalOutcome {
    TemporalOutcome::Violated {
        at: TemporalPosition::new(index),
    }
}

/// The closed `Counter` trace `c.level` 0, 1, 2 under event-position.
fn closed(body: &str) -> TemporalOutcome {
    outcome(&counter(EVENT, body), finite(3), counters(&[0, 1, 2]))
}

/// The `Counter` prefix `c.level` 0, 1, 2 under infinite-trace.
fn prefix(body: &str) -> TemporalOutcome {
    outcome(&counter(INFINITE, body), finite(3), counters(&[0, 1, 2]))
}

/// FR-327-AC-1: over the closed `Counter` trace with `c.level` 0, 1, 2 under
/// event-position false-extension, `on origin`, the four bounded clauses
/// settle as the AC states, position 3 being past closure.
#[trace("TC-837", "FR-327-AC-1")]
#[test]
fn a_closed_trace_settles_bounded_clauses_with_false_extension_past_its_end() {
    assert_eq!(
        closed("eventually[0,2] holds(c.level = 2)"),
        TemporalOutcome::Holds
    );
    assert_eq!(closed("eventually[0,1] holds(c.level = 2)"), violated(0));
    assert_eq!(closed("always[0,3] holds(c.level <= 2)"), violated(0));
    assert_eq!(
        closed("always[0,3] not holds(c.level > 2)"),
        TemporalOutcome::Holds
    );
}

/// The outcome of `fixture` on the closed trace `c.level` 0 to 4, the steps
/// into positions 1 and 3 uncounted: the counted distance from position 0 is
/// 0, 0, 1, 1, 2 (ADR-027 PB-3).
fn counted(fixture: &Fixture) -> TemporalOutcome {
    let mut letters = Letters::new(fixture, counters(&[0, 1, 2, 3, 4]));
    letters.uncounted = vec![1, 3];
    evaluate(
        &fixture.clause,
        finite(5),
        &mut letters,
        &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
    )
    .expect("the trace is readable")
}

/// ADR-027 PB-3: an interval measures distance in
/// counted steps, several positions share a distance and an uncounted step
/// does not move it. With the steps into positions 1 and 3 uncounted,
/// `eventually[0,1]` reaches `c.level = 3` at position 3, `always[0,1]`
/// reads positions 0 to 3, `eventually[1,1]` reads positions 2 and 3, and the
/// past operator `once[0,0]` at position 3 still reads position 2.
#[trace("TC-658")]
#[test]
fn an_uncounted_step_moves_the_position_and_not_the_distance() {
    let holds = |body: &str| counted(&counter(EVENT, body));
    assert_eq!(
        holds("eventually[0,1] holds(c.level = 3)"),
        TemporalOutcome::Holds
    );
    assert_eq!(holds("eventually[0,1] holds(c.level = 4)"), violated(0));
    assert_eq!(
        holds("eventually[1,1] holds(c.level = 3)"),
        TemporalOutcome::Holds
    );
    assert_eq!(holds("eventually[1,1] holds(c.level = 4)"), violated(0));
    assert_eq!(
        holds("always[0,1] holds(c.level <= 3)"),
        TemporalOutcome::Holds
    );
    assert_eq!(holds("always[0,1] holds(c.level <= 2)"), violated(0));
    let each = fixture(
        EVENT,
        "c: Counter",
        "on each (q: Counter)",
        "holds(c.level = 3) implies once[0,0] holds(c.level = 2)",
    );
    assert_eq!(counted(&each), TemporalOutcome::Holds);
    let farther = fixture(
        EVENT,
        "c: Counter",
        "on each (q: Counter)",
        "holds(c.level = 4) implies once[0,0] holds(c.level = 3)",
    );
    assert_eq!(counted(&farther), violated(4));
}

/// ADR-027 PB-3 (FR-338): a lasso with an uncounted step is read in counted
/// steps, its loop-closing step included. On the lasso `0 | 1 2` of
/// `c.level` with a counted loop, `eventually[0,1] holds(c.level = 2)` holds
/// at position 0, because position 2 is one counted step from position 1's
/// predecessor only when position 1 does not count: with the step into
/// position 1 uncounted, positions 0 and 1 share distance 0 and position 2
/// is at distance 1, so it holds; counting every step, position 2 is at
/// distance 2 and it fails.
#[trace("TC-658", "FR-338-AC-6")]
#[test]
fn a_lasso_with_an_uncounted_step_is_read_in_counted_steps() {
    let fixture = counter(INFINITE, "eventually[0,1] holds(c.level = 2)");
    let lasso = TemporalTrace::Lasso {
        prefix: 1,
        loop_len: 2,
    };
    let at = |uncounted: Vec<u64>, edge: bool| {
        let mut letters = Letters::new(&fixture, counters(&[0, 1, 2]));
        letters.uncounted = uncounted;
        letters.uncounted_loop_edge = edge;
        evaluate(
            &fixture.clause,
            lasso,
            &mut letters,
            &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
        )
        .expect("the lasso is readable")
    };
    assert_eq!(at(vec![1], false), TemporalOutcome::Holds);
    assert_eq!(at(vec![], false), violated(0));
    // The loop-closing step counts or not as its letter says.
    assert_eq!(at(vec![], true), violated(0));
    assert_eq!(at(vec![1], true), TemporalOutcome::Holds);
}

/// Reading a lasso in counted steps pays one unit per trace position, named
/// `temporal.visit`; a meter that runs out during that read stops `Incomplete`
/// at that point.
#[trace("TC-658", "FR-338-AC-6")]
#[test]
fn the_counted_step_read_is_charged_at_temporal_visit() {
    let fixture = counter(INFINITE, "eventually[0,1] holds(c.level = 2)");
    let lasso = TemporalTrace::Lasso {
        prefix: 1,
        loop_len: 2,
    };
    let mut letters = Letters::new(&fixture, counters(&[0, 1, 2]));
    letters.uncounted = vec![1];
    let outcome = evaluate(
        &fixture.clause,
        lasso,
        &mut letters,
        &mut Meter::new(limits(1)),
    )
    .expect("the lasso is readable");
    let TemporalOutcome::Incomplete(incomplete) = outcome else {
        panic!("one unit cannot read three positions: {outcome:?}");
    };
    assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
    assert_eq!(incomplete.charge_point, ChargePoint::TemporalVisit);
}

/// ADR-027 PB-3 (FR-338): a loop with no counted step keeps every distance
/// from a position before it constant. `eventually[1,1] holds(c.level = 2)`
/// is not met on the lasso `0 | 1 2` when the loop's steps and its closing
/// step are all uncounted: no position is one counted step from position 0.
/// With one counted step in the loop it is met, and the same claim over
/// `always[1,1]` is vacuously true on the first lasso.
#[trace("TC-658", "FR-338-AC-6")]
#[test]
fn a_loop_without_a_counted_step_meets_no_interval_that_needs_one() {
    let lasso = TemporalTrace::Lasso {
        prefix: 1,
        loop_len: 2,
    };
    let verdict = |formula: &str, uncounted: Vec<u64>, edge: bool| {
        let fixture = counter(INFINITE, formula);
        let mut letters = Letters::new(&fixture, counters(&[0, 1, 2]));
        letters.uncounted = uncounted;
        letters.uncounted_loop_edge = edge;
        evaluate(
            &fixture.clause,
            lasso,
            &mut letters,
            &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
        )
        .expect("the lasso is readable")
    };
    // Steps 1 and 2 are the prefix's step into the loop and the loop's own;
    // the closing step is the third.
    assert_eq!(
        verdict("eventually[1,1] holds(c.level = 2)", vec![1, 2], true),
        violated(0)
    );
    assert_eq!(
        verdict("eventually[1,1] holds(c.level = 2)", vec![1], true),
        TemporalOutcome::Holds,
        "position 2 is one counted step from position 0 once step 2 counts"
    );
    assert_eq!(
        verdict("always[1,1] holds(c.level = 9)", vec![1, 2], true),
        TemporalOutcome::Holds
    );
}

/// The outcome of `fixture` on the lasso `0 | 1 2` of `c.level`, with every
/// step uncounted, the closing step included: a loop with no counted step
/// (ADR-027 PB-3), so positions 1 and 2 are at distance 0 from position 0 and
/// from each other for ever.
fn zero_loop(fixture: &Fixture) -> TemporalOutcome {
    let mut letters = Letters::new(fixture, counters(&[0, 1, 2]));
    letters.uncounted = vec![1, 2];
    letters.uncounted_loop_edge = true;
    evaluate(
        &fixture.clause,
        lasso(1, 2),
        &mut letters,
        &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
    )
    .expect("the lasso is readable")
}

/// ADR-027 PB-3 (FR-338): past windows over a loop with no counted step.
/// Every position is at distance 0 from every earlier one, across passes,
/// so `once[0,2]` at position 2 reaches position 0 and `once[1,2]` reaches
/// nothing; `historically[0,1]` includes position 0's level 0 and
/// `historically[1,2]` reads only the virtual positions before the origin,
/// where every atom is false.
#[trace("TC-658", "FR-338-AC-6")]
#[test]
fn a_past_window_over_a_loop_without_a_counted_step_reads_distance_zero() {
    let holds = |body: &str| zero_loop(&counter(INFINITE, body));
    assert_eq!(
        holds("always (holds(c.level = 2) implies once[0,2] holds(c.level = 0))"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        holds("always (holds(c.level = 2) implies once[1,2] holds(c.level = 0))"),
        violated(0)
    );
    assert_eq!(
        holds("always (holds(c.level = 2) implies historically[1,2] not holds(c.level = 9))"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        holds("always (holds(c.level = 2) implies historically[0,1] holds(c.level >= 1))"),
        violated(0)
    );
}

/// ADR-027 PB-3 (FR-338): `until` and `release` over a loop with no counted
/// step. The witness at position 2 is at distance 0, so `until[0,2]` holds
/// and `until[1,2]` has no witness; `release[1,2]` is vacuous and
/// `release[0,2]` needs its right operand at position 0.
#[trace("TC-658", "FR-338-AC-6")]
#[test]
fn until_and_release_over_a_loop_without_a_counted_step_read_distance_zero() {
    let holds = |body: &str| zero_loop(&counter(INFINITE, body));
    assert_eq!(
        holds("holds(c.level <= 1) until[0,2] holds(c.level = 2)"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        holds("holds(c.level <= 1) until[1,2] holds(c.level = 2)"),
        violated(0)
    );
    assert_eq!(
        holds("holds(c.level = 9) release[1,2] holds(c.level = 8)"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        holds("holds(c.level = 9) release[0,2] holds(c.level = 8)"),
        violated(0)
    );
}

/// ADR-027 PB-3 (FR-338): an `on each` activation reads its window over a
/// loop with no counted step. From position 0 the window `[0,1]` holds
/// positions 1 and 2 at distance 0, so level 2 falls inside it.
#[trace("TC-658", "FR-338-AC-6")]
#[test]
fn an_each_activation_over_a_loop_without_a_counted_step_reads_distance_zero() {
    let each = |body: &str| {
        zero_loop(&fixture(
            INFINITE,
            "c: Counter",
            "on each (q: Counter)",
            body,
        ))
    };
    assert_eq!(each("always[0,1] holds(c.level <= 1)"), violated(0));
    assert_eq!(
        each("always[1,1] holds(c.level <= 1)"),
        TemporalOutcome::Holds
    );
}

/// ADR-027 PB-3 (FR-338), STD-254: a bounded-profile `eventually` and
/// `always` over a loop with no counted step are read as the infinite trace
/// they are; an interval that needs a counted step is not met.
#[trace("TC-658", "FR-338-AC-6")]
#[test]
fn a_bounded_profile_lasso_without_a_counted_step_is_read_in_distance_zero() {
    let holds = |body: &str| zero_loop(&counter(EVENT, body));
    assert_eq!(
        holds("eventually[0,5] holds(c.level = 2)"),
        TemporalOutcome::Holds
    );
    assert_eq!(holds("eventually[1,5] holds(c.level = 2)"), violated(0));
    assert_eq!(
        holds("always[1,5] holds(c.level = 9)"),
        TemporalOutcome::Holds
    );
    assert_eq!(holds("always[0,5] holds(c.level <= 1)"), violated(0));
}

/// FR-327-AC-2: `always[0,0] holds(c.level <= 1)` with activation `on each`
/// is false at position 2.
#[trace("TC-837", "FR-327-AC-2")]
#[test]
fn an_each_activation_names_the_first_activating_position_that_fails() {
    let each = fixture(
        EVENT,
        "c: Counter",
        "on each (q: Counter)",
        "always[0,0] holds(c.level <= 1)",
    );
    assert_eq!(outcome(&each, finite(3), counters(&[0, 1, 2])), violated(2));
}

/// FR-327-AC-3: `TracePosition` descriptors `0` and `2` decode; `02`, `-1`,
/// the empty string and `18446744073709551616` each refuse
/// `invalid_runtime_input`/`invalid-value` with their own cause; position 3
/// on the three-position trace refuses at reconstruction; position 12
/// encodes as `12`.
#[trace("TC-837", "FR-327-AC-3")]
#[test]
fn positions_decode_from_decimal_ascii_and_refuse_everything_else() {
    let trace = finite(3);
    assert_eq!(trace.locate("0"), Ok(TemporalPosition::new(0)));
    assert_eq!(trace.locate("2"), Ok(TemporalPosition::new(2)));
    let code = CatalogCode::new("invalid_runtime_input", "invalid-value");
    for (descriptor, refusal) in [
        ("02", PositionRefusal::LeadingZero),
        ("-1", PositionRefusal::NotDecimal),
        ("", PositionRefusal::NotDecimal),
        ("18446744073709551616", PositionRefusal::Overflow),
        (
            "3",
            PositionRefusal::OutsideTrace {
                position: TemporalPosition::new(3),
            },
        ),
    ] {
        assert_eq!(trace.locate(descriptor), Err(refusal), "{descriptor:?}");
        assert_eq!(refusal.catalog_code(), code, "{descriptor:?}");
    }
    assert_eq!(TemporalPosition::new(12).descriptor(), "12");
    assert_eq!(TemporalPosition::new(0).descriptor(), "0");
}

/// FR-327-AC-4: `eventually[0,2] holds(c.level = 2)` `on origin` makes 4
/// visits (the `eventually` node at position 0 and the atom at positions 0,
/// 1 and 2); a meter of 3 work units is `Incomplete` at the `WorkUnits`
/// charge point with no truth value, a meter of 4 completes and charges 4.
#[trace("TC-837", "FR-327-AC-4")]
#[test]
fn work_is_charged_per_visit_and_exhaustion_returns_no_truth_value() {
    let fx = counter(EVENT, "eventually[0,2] holds(c.level = 2)");
    let (settled, spent, reads) = run(&fx, finite(3), counters(&[0, 1, 2]), limits(4));
    assert_eq!(settled, TemporalOutcome::Holds);
    assert_eq!(spent, 4);
    assert_eq!(reads, [0, 1, 2]);

    let (short, spent, _) = run(&fx, finite(3), counters(&[0, 1, 2]), limits(3));
    let TemporalOutcome::Incomplete(incomplete) = short else {
        panic!("three units stop the fourth visit: {short:?}");
    };
    assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
    assert_eq!(incomplete.limit, 3);
    assert_eq!(incomplete.consumed, 3);
    assert_eq!(incomplete.charge_point, ChargePoint::TemporalVisit);
    assert_eq!(spent, 3);
    assert_eq!(
        TemporalOutcome::Incomplete(incomplete).category(),
        Category::Incomplete
    );
}

fn undefined_at(index: u64, cause: Undefined) -> TemporalOutcome {
    TemporalOutcome::Undefined(qsl_eval::temporal::UndefinedEvaluation {
        at: TemporalPosition::new(index),
        cause: UndefinedCause::Kernel(cause),
    })
}

/// FR-327-AC-5: `always[0,2] holds(2 / (2 - c.level) >= 1)` `on origin`
/// returns `Undefined` at position 2 with division by zero, category
/// violation; `eventually[0,1] holds(2 / (2 - c.level) = 2)` reads positions
/// 0 and 1 only and completes true.
#[trace("TC-847", "FR-327-AC-5")]
#[test]
fn an_undefined_atom_at_a_position_the_clause_reads_is_returned_with_its_cause() {
    let fx = counter(EVENT, "always[0,2] holds(2 / (2 - c.level) >= 1)");
    let (settled, _, _) = run(
        &fx,
        finite(3),
        counters(&[0, 1, 2]),
        SCALAR_LIMITS_UNLIMITED,
    );
    assert_eq!(settled, undefined_at(2, Undefined::DivisionByZero));
    assert_eq!(settled.category(), Category::Violation);

    let fx = counter(EVENT, "eventually[0,1] holds(2 / (2 - c.level) = 2)");
    let (settled, _, reads) = run(
        &fx,
        finite(3),
        counters(&[0, 1, 2]),
        SCALAR_LIMITS_UNLIMITED,
    );
    assert_eq!(settled, TemporalOutcome::Holds);
    assert_eq!(reads, [0, 1]);
}

/// FR-328-AC-1: over the `Counter` prefix 0, 1, 2 under infinite-trace,
/// `always holds(c.level <= 1)` is false at position 0, `always
/// holds(c.level <= 5)` is pending, and `eventually` is pending whether its
/// atom is reached or not.
#[trace("TC-838", "FR-328-AC-1")]
#[test]
fn a_prefix_reports_a_bad_prefix_of_a_safety_formula_and_pends_the_rest() {
    assert_eq!(prefix("always holds(c.level <= 1)"), violated(0));
    assert_eq!(
        prefix("always holds(c.level <= 5)"),
        TemporalOutcome::Pending
    );
    assert_eq!(
        prefix("eventually holds(c.level = 2)"),
        TemporalOutcome::Pending
    );
    assert_eq!(
        prefix("eventually holds(c.level = 9)"),
        TemporalOutcome::Pending
    );
}

/// FR-328-AC-2: a response property with an interval under `always` is false
/// at position 0 when the present positions refute it, and pending when
/// position 3 decides it.
#[trace("TC-838", "FR-328-AC-2")]
#[test]
fn an_interval_response_is_refuted_by_present_positions_and_pends_on_unknown_ones() {
    assert_eq!(
        prefix("always (holds(c.level = 1) implies eventually[0,1] holds(c.level = 3))"),
        violated(0)
    );
    assert_eq!(
        prefix("always (holds(c.level = 2) implies eventually[0,1] holds(c.level = 3))"),
        TemporalOutcome::Pending
    );
}

/// FR-328-AC-3: `once[1,1]` at position 0 reads before position 0, where
/// atoms are false.
#[trace("TC-838", "FR-328-AC-3")]
#[test]
fn a_past_operator_reads_atoms_as_false_before_position_zero() {
    assert_eq!(
        prefix("always (holds(c.level = 0) implies once[1,1] holds(c.level = 9))"),
        violated(0)
    );
}

/// FR-328-AC-4: none of the AC-1 to AC-3 clauses is `tested` or `proved`;
/// each pending one is O-16 inconclusive and each violation is a violation.
#[trace("TC-838", "FR-328-AC-4")]
#[test]
fn a_prefix_never_settles_tested_and_pending_is_inconclusive() {
    let clauses = [
        "always holds(c.level <= 1)",
        "always holds(c.level <= 5)",
        "eventually holds(c.level = 2)",
        "eventually holds(c.level = 9)",
        "always (holds(c.level = 1) implies eventually[0,1] holds(c.level = 3))",
        "always (holds(c.level = 2) implies eventually[0,1] holds(c.level = 3))",
        "always (holds(c.level = 0) implies once[1,1] holds(c.level = 9))",
    ];
    let categories: Vec<Category> = clauses
        .iter()
        .map(|body| {
            let settled = prefix(body);
            assert_ne!(settled, TemporalOutcome::Holds, "{body}");
            settled.category()
        })
        .collect();
    assert_eq!(
        categories,
        [
            Category::Violation,
            Category::Inconclusive,
            Category::Inconclusive,
            Category::Inconclusive,
            Category::Violation,
            Category::Inconclusive,
            Category::Violation,
        ]
    );
}

/// FR-328-AC-5: `eventually holds(2 / (2 - c.level) = 3)`, outside the
/// safety fragment, is `Undefined` at position 2, not pending.
#[trace("TC-847", "FR-328-AC-5")]
#[test]
fn an_undefined_letter_in_a_prefix_is_undefined_whatever_the_fragment() {
    assert_eq!(
        prefix("eventually holds(2 / (2 - c.level) = 3)"),
        undefined_at(2, Undefined::DivisionByZero)
    );
}

/// The one constraint of `fixture`'s fairness set, which must be
/// `fair weak whole increment`.
fn the_weak_whole_increment(
    fixture: &Fixture,
) -> quire_semantic_value::fairness::FairnessConstraint {
    let [constraint] = fixture.clause.fairness() else {
        panic!("one constraint: {:?}", fixture.clause.fairness());
    };
    assert_eq!(constraint.kind(), FairnessKind::Weak);
    assert_eq!(constraint.operation().name(), "increment");
    assert_eq!(constraint.granularity(), FairnessGranularity::Whole);
    constraint.clone()
}

/// FR-328-AC-6: with the fairness set `fair weak increment` over a supplied
/// prefix, `always holds(c.level <= 1)` is `MissingFairnessPremise` naming
/// `fair weak whole increment`, O-16 unsupported, with no position evaluated
/// and no work charged; with an empty fairness set it is false at position 0.
#[trace("TC-848", "FR-328-AC-6")]
#[test]
fn a_fairness_set_over_a_supplied_prefix_is_a_missing_premise_and_evaluates_nothing() {
    let fair = modelled("fair weak M::Counter::increment; always holds(c.level <= 1)");
    let (settled, spent, reads) = run(&fair, finite(3), counters(&[0, 1, 2]), limits(0));
    assert_eq!(
        settled,
        TemporalOutcome::MissingFairnessPremise {
            constraint: the_weak_whole_increment(&fair)
        }
    );
    assert_eq!(settled.category(), Category::Unsupported);
    assert_eq!((spent, reads), (0, Vec::new()));

    assert_eq!(prefix("always holds(c.level <= 1)"), violated(0));
}

fn lasso(prefix: u64, loop_len: u64) -> TemporalTrace {
    TemporalTrace::Lasso { prefix, loop_len }
}

const RECURRENT_ZERO_AFTER_TWO: &str =
    "always eventually (holds(a.versionNumber = 0) and once[1,1] holds(a.versionNumber = 2))";

/// FR-329-AC-1: on the observed lasso with an empty prefix and loop
/// `a.versionNumber` 0, 1, 2, three infinite-trace clauses settle exactly.
#[trace("TC-839", "FR-329-AC-1")]
#[test]
fn a_lasso_is_evaluated_exactly_including_a_past_condition_across_the_loop_edge() {
    let observations = || versions(&[0, 1, 2]);
    assert_eq!(
        outcome(
            &config(RECURRENT_ZERO_AFTER_TWO),
            lasso(0, 3),
            observations()
        ),
        TemporalOutcome::Holds
    );
    assert_eq!(
        outcome(
            &config("always eventually holds(a.versionNumber = 2)"),
            lasso(0, 3),
            observations()
        ),
        TemporalOutcome::Holds
    );
    assert_eq!(
        outcome(
            &config("eventually always holds(a.versionNumber = 0)"),
            lasso(0, 3),
            observations()
        ),
        violated(0)
    );
}

/// FR-329-AC-2: on the observed lasso with prefix `a.versionNumber` 5 and
/// loop 0, 1, 2, a past condition reaches the prefix although no loop
/// position holds 5, and `once[1,1]` at loop position 1 reads position 0.
#[trace("TC-839", "FR-329-AC-2")]
#[test]
fn a_past_operator_on_a_lasso_reads_back_into_the_prefix() {
    let observations = || versions(&[5, 0, 1, 2]);
    assert_eq!(
        outcome(
            &config("always (holds(a.versionNumber = 0) implies once holds(a.versionNumber = 5))"),
            lasso(1, 3),
            observations()
        ),
        TemporalOutcome::Holds
    );
    assert_eq!(
        outcome(
            &config(
                "always (holds(a.versionNumber = 0) implies once[1,1] holds(a.versionNumber = 2))"
            ),
            lasso(1, 3),
            observations()
        ),
        violated(0)
    );
}

/// FR-329 "Lasso shape": a lasso with an empty loop refuses
/// `invalid_runtime_input`/`invalid-value`, and so does a finite trace with
/// no observation. (FR-329-AC-3's `Lasso::Model` half needs the FR-120
/// model subject and is not exercised here.)
#[test]
fn a_lasso_with_an_empty_loop_and_an_empty_finite_trace_are_refused() {
    let fx = config("always holds(a.versionNumber = 0)");
    let mut letters = Letters::new(&fx, Vec::new());
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let refused = evaluate(&fx.clause, lasso(2, 0), &mut letters, &mut meter);
    assert_eq!(refused, Err(TemporalRefusal::EmptyLoop));
    assert_eq!(
        TemporalRefusal::EmptyLoop.catalog_code(),
        CatalogCode::new("invalid_runtime_input", "invalid-value")
    );
    let refused = evaluate(&fx.clause, finite(0), &mut letters, &mut meter);
    assert_eq!(refused, Err(TemporalRefusal::EmptyTrace));
    assert!(letters.reads.is_empty());
    assert_eq!(work(&meter), 0);
}

/// FR-329-AC-5: the first clause of AC-1 with a meter one unit short of its
/// visit count returns `Incomplete` and no truth value.
#[trace("TC-839", "FR-329-AC-5")]
#[test]
fn a_lasso_one_work_unit_short_returns_incomplete() {
    let fx = config(RECURRENT_ZERO_AFTER_TWO);
    let (settled, total, _) = run(
        &fx,
        lasso(0, 3),
        versions(&[0, 1, 2]),
        SCALAR_LIMITS_UNLIMITED,
    );
    assert_eq!(settled, TemporalOutcome::Holds);
    assert!(total > 0);
    let (short, spent, _) = run(&fx, lasso(0, 3), versions(&[0, 1, 2]), limits(total - 1));
    let TemporalOutcome::Incomplete(incomplete) = short else {
        panic!("one unit short stops the run: {short:?}");
    };
    assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
    assert_eq!(incomplete.consumed, total - 1);
    assert_eq!(spent, total - 1);
    let (whole, _, _) = run(&fx, lasso(0, 3), versions(&[0, 1, 2]), limits(total));
    assert_eq!(whole, TemporalOutcome::Holds);
}

/// FR-329-AC-6: on the observed `Counter` lasso with an empty prefix and loop
/// `c.level` 0, 1, 2, `always eventually holds(2 / (2 - c.level) = 2)` is
/// `Undefined` at position 2, category violation.
#[trace("TC-847", "FR-329-AC-6")]
#[test]
fn an_undefined_atom_in_a_lasso_loop_is_returned_at_its_first_position() {
    let fx = counter(INFINITE, "always eventually holds(2 / (2 - c.level) = 2)");
    let settled = outcome(&fx, lasso(0, 3), counters(&[0, 1, 2]));
    assert_eq!(settled, undefined_at(2, Undefined::DivisionByZero));
    assert_eq!(settled.category(), Category::Violation);
}

/// FR-329-AC-7: with `fair weak increment` over an observed lasso the
/// clause is `MissingFairnessPremise` naming `fair weak whole increment`,
/// O-16 unsupported, with no position read; with an empty fairness set it is
/// false at position 0.
#[trace("TC-848", "FR-329-AC-7")]
#[test]
fn a_fairness_set_over_an_observed_lasso_is_a_missing_premise_and_evaluates_nothing() {
    let observations = || {
        [0, 1, 2]
            .iter()
            .map(|a| Observation { c: 0, a: *a, b: 0 })
            .collect::<Vec<_>>()
    };
    let clause = "always eventually holds(b.versionNumber = 2)";
    let fair = modelled(&format!("fair weak M::Counter::increment; {clause}"));
    let (settled, spent, reads) = run(&fair, lasso(0, 3), observations(), limits(0));
    assert_eq!(
        settled,
        TemporalOutcome::MissingFairnessPremise {
            constraint: the_weak_whole_increment(&fair)
        }
    );
    assert_eq!(settled.category(), Category::Unsupported);
    assert_eq!((spent, reads), (0, Vec::new()));

    assert_eq!(
        outcome(&config(clause), lasso(0, 3), observations()),
        violated(0)
    );
}

/// FR-125-AC-3 (the lasso reading of SM-8): on the lasso with an empty prefix
/// and loop `(0,0) (1,0) (2,0)`, `eventually[0,4] holds(a.versionNumber = 0)`
/// at position 2 reads positions 2 to 6 by wrapping and is true; `once[0,3]
/// holds(a.versionNumber = 2)` at position 1 reads positions 1 and 0, the
/// rest before position 0 as false, and is false; each visit charges one
/// work unit.
#[trace("TC-520", "FR-125-AC-3")]
#[test]
fn a_formula_is_read_at_a_position_by_wrapping_and_by_false_before_the_origin() {
    let at = |fx: &Fixture, position: u64| {
        let mut letters = Letters::new(fx, versions(&[0, 1, 2]));
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let settled = evaluate_at(
            &fx.clause,
            lasso(0, 3),
            TemporalPosition::new(position),
            &mut letters,
            &mut meter,
        )
        .expect("the lasso is readable");
        (settled, work(&meter))
    };
    let future = config("eventually[0,4] holds(a.versionNumber = 0)");
    // Three atom reads (the loop's positions), the one operator visit, and
    // one unit for the window position the trace does not represent (position
    // 3 by wrapping, position -1 before the origin).
    assert_eq!(at(&future, 2), (TemporalOutcome::Holds, 5));
    let past = config("once[0,3] holds(a.versionNumber = 2)");
    assert_eq!(at(&past, 1), (violated(1), 5));
}

/// FR-125-AC-5: `always eventually (holds(a.versionNumber = 0) and
/// once[1,1] holds(a.versionNumber = 2))` has past reach 1 and is evaluated
/// over `m = 2` loop copies, 6 positions; its conjunction is false at
/// position 0, true at position 3 and, read by congruence, true at position
/// 9; and the formula is true.
#[trace("TC-520", "FR-125-AC-5")]
#[test]
fn the_unrolling_follows_the_past_reach_and_reads_past_it_by_congruence() {
    let whole = config(RECURRENT_ZERO_AFTER_TWO);
    assert_eq!(
        unrolling(&whole.clause, lasso(0, 3)),
        Ok(Unrolling {
            past_reach: 1,
            copies: 2,
            positions: 6,
        })
    );
    assert_eq!(
        outcome(&whole, lasso(0, 3), versions(&[0, 1, 2])),
        TemporalOutcome::Holds
    );

    let conjunction = config("holds(a.versionNumber = 0) and once[1,1] holds(a.versionNumber = 2)");
    let at = |position: u64| {
        let mut letters = Letters::new(&conjunction, versions(&[0, 1, 2]));
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        evaluate_at(
            &conjunction.clause,
            lasso(0, 3),
            TemporalPosition::new(position),
            &mut letters,
            &mut meter,
        )
        .expect("the lasso is readable")
    };
    assert_eq!(at(0), violated(0));
    assert_eq!(at(3), TemporalOutcome::Holds);
    assert_eq!(at(9), TemporalOutcome::Holds);
}

/// FR-125-AC-6: the letter of `always holds(6 / (2 - c.level) >= 0)` under
/// infinite-trace is defined at positions 0 and 1 and undefined at position
/// 2, cause division by zero; `eventually[0,1] holds(6 / (2 - c.level) =
/// 6)` under event-position, `on origin`, reads positions 0 and 1 only and
/// is true.
#[trace("TC-537", "FR-125-AC-6")]
#[test]
fn the_letter_is_read_up_to_its_first_undefined_position_and_a_bounded_origin_to_its_horizon() {
    let fx = counter(INFINITE, "always holds(6 / (2 - c.level) >= 0)");
    let (settled, _, reads) = run(
        &fx,
        finite(4),
        counters(&[0, 1, 2, 3]),
        SCALAR_LIMITS_UNLIMITED,
    );
    assert_eq!(settled, undefined_at(2, Undefined::DivisionByZero));
    assert_eq!(reads, [0, 1, 2]);

    let fx = counter(EVENT, "eventually[0,1] holds(6 / (2 - c.level) = 6)");
    let (settled, _, reads) = run(
        &fx,
        finite(4),
        counters(&[0, 1, 2, 3]),
        SCALAR_LIMITS_UNLIMITED,
    );
    assert_eq!(settled, TemporalOutcome::Holds);
    assert_eq!(reads, [0, 1]);
}

/// FR-091-AC-3 as FR-327 reads it: `p until[1,2] q` is true with `p` false at
/// the anchor when `q` is true at offset one, because the left operand is
/// required only from the lower bound to the witness.
#[test]
fn until_with_a_lower_bound_does_not_require_its_left_operand_before_the_bound() {
    assert_eq!(
        closed("holds(c.level = 9) until[1,2] holds(c.level = 1)"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        closed("holds(c.level = 9) until[1,2] holds(c.level = 2)"),
        violated(0)
    );
}

/// Open-ended windows under infinite-trace: `eventually[2,*]` reads
/// positions from the offset on and `once[2,*]` reads back from it.
#[test]
fn an_open_window_reads_from_its_lower_bound_on_or_back_from_it() {
    let wanted = "holds(a.versionNumber = 1)";
    let later = config(&format!("eventually[2,*] {wanted}"));
    assert_eq!(
        outcome(&later, lasso(4, 1), versions(&[0, 0, 0, 1, 0])),
        TemporalOutcome::Holds
    );
    assert_eq!(
        outcome(&later, lasso(4, 1), versions(&[0, 1, 0, 0, 0])),
        violated(0)
    );
    let earlier = config(&format!("once[2,*] {wanted}"));
    let at = |position: u64, values: &[i64]| {
        let mut letters = Letters::new(&earlier, versions(values));
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        evaluate_at(
            &earlier.clause,
            lasso(4, 1),
            TemporalPosition::new(position),
            &mut letters,
            &mut meter,
        )
        .expect("the lasso is readable")
    };
    assert_eq!(at(1, &[0, 1, 0, 0, 0]), violated(1));
    assert_eq!(at(3, &[0, 1, 0, 0, 0]), TemporalOutcome::Holds);
}

/// A lasso is read under the infinite-trace profile only.
#[test]
fn a_lasso_under_a_bounded_profile_is_refused() {
    let fx = counter(EVENT, "eventually[0,2] holds(c.level = 2)");
    let mut letters = Letters::new(&fx, counters(&[0, 1, 2]));
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let refused = evaluate(&fx.clause, lasso(0, 3), &mut letters, &mut meter);
    assert!(matches!(
        refused,
        Err(TemporalRefusal::LassoUnderBoundedProfile { .. })
    ));
    assert!(letters.reads.is_empty());
}

/// A node visit is charged once per position however many windows read it:
/// in three nested `eventually[0,1]` over an atom that is never true, the
/// innermost visit at position 1 is read by two visits of the middle node
/// and charged once. Six operator visits and three atom reads.
#[test]
fn a_node_visit_is_charged_once_per_position() {
    let fx = counter(
        EVENT,
        "eventually[0,1] eventually[0,1] eventually[0,1] holds(c.level > 5)",
    );
    let (settled, spent, reads) = run(
        &fx,
        finite(3),
        counters(&[0, 1, 2]),
        SCALAR_LIMITS_UNLIMITED,
    );
    assert_eq!(settled, violated(0));
    assert_eq!(reads, [0, 1, 2]);
    // The innermost visit at position 2 scans position 3, past the trace,
    // which costs one more unit.
    assert_eq!(spent, 3 + 6 + 1);
}

/// A window of 2^60 positions beside a past reach of 2^60 stops on the meter
/// instead of scanning the unrolled loop copies unpaid.
#[test]
fn a_window_over_unrepresented_positions_is_paid_per_step_and_stops_on_the_meter() {
    let fx = config(
        "eventually[0,1152921504606846976] holds(a.versionNumber = 9) \
         and once[0,1152921504606846976] holds(a.versionNumber = 9)",
    );
    let (settled, spent, _) = run(&fx, lasso(0, 3), versions(&[0, 1, 2]), limits(10_000));
    let TemporalOutcome::Incomplete(incomplete) = settled else {
        panic!("the meter stops the scan: {settled:?}");
    };
    assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
    assert_eq!(incomplete.charge_point, ChargePoint::TemporalVisit);
    assert_eq!(spent, 10_000);
}

/// An open `[a,*]` is the limit of the closed `[a,b]`: the left operand of
/// `until` is required over `[a,d)`, not from the anchor (QSpec FR-091). On
/// the lasso `a.versionNumber` 0, 1, 2, `p until[1,*] q` with `p` = 1 and `q` =
/// 2 holds at position 0 although `p` is false there, before the lower bound;
/// with `p` = 0 it fails, `p` being false at position 1, which is at or after
/// the lower bound and before the witness at position 2.
#[test]
fn an_open_until_requires_its_left_operand_from_the_lower_bound_to_the_witness() {
    let holds = config("holds(a.versionNumber = 1) until[1,*] holds(a.versionNumber = 2)");
    assert_eq!(
        outcome(&holds, lasso(0, 3), versions(&[0, 1, 2])),
        TemporalOutcome::Holds
    );
    let fails = config("holds(a.versionNumber = 0) until[1,*] holds(a.versionNumber = 2)");
    assert_eq!(
        outcome(&fails, lasso(0, 3), versions(&[0, 1, 2])),
        violated(0)
    );
    let unbounded = config("holds(a.versionNumber = 0) until[0,*] holds(a.versionNumber = 1)");
    assert_eq!(
        outcome(&unbounded, lasso(0, 3), versions(&[0, 1, 2])),
        TemporalOutcome::Holds
    );
}

/// The past limit: `p since[1,*] q` at position 2 requires `p` over
/// `(j, i-1]`, not `(j, i]`. With `q` = 0 at position 0 and `p` = 1 at
/// position 1 it holds although `p` is false at position 2, the most recent
/// position; with `p` = 0 it fails, `p` being false at position 1.
#[test]
fn an_open_since_requires_its_left_operand_up_to_the_lower_bound_back_from_the_anchor() {
    let at = |body: &str| {
        let fx = config(body);
        let mut letters = Letters::new(&fx, versions(&[0, 1, 2]));
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        evaluate_at(
            &fx.clause,
            lasso(0, 3),
            TemporalPosition::new(2),
            &mut letters,
            &mut meter,
        )
        .expect("the lasso is readable")
    };
    assert_eq!(
        at("holds(a.versionNumber = 1) since[1,*] holds(a.versionNumber = 0)"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        at("holds(a.versionNumber = 0) since[1,*] holds(a.versionNumber = 0)"),
        violated(2)
    );
}

/// The shape of one checked fairness constraint.
fn shape(
    constraint: &quire_semantic_value::fairness::FairnessConstraint,
) -> (FairnessKind, &str, FairnessGranularity) {
    (
        constraint.kind(),
        constraint.operation().name(),
        constraint.granularity(),
    )
}

/// The lasso `0 2 0` over a counter that two operations step, the supplied
/// trace of FR-132, with the liveness clause none of its states meets.
fn mutex_outcome(constraints: &str) -> (Fixture, TemporalOutcome) {
    let clause = "always eventually holds(c.level = 1)";
    let fixture = modelled_with(
        vec![acquire(), release()],
        &format!("{constraints}{clause}"),
    );
    let settled = outcome(&fixture, lasso(0, 3), counters(&[0, 2, 0]));
    (fixture, settled)
}

/// FR-132-AC-1: a supplied lasso with a fairness constraint on `acquire`
/// settles `MissingFairnessPremise` naming the first constraint of the
/// fairness set, whatever its kind and granularity, O-16 unsupported.
#[trace("TC-533", "FR-132-AC-1")]
#[test]
fn a_fairness_constraint_over_a_supplied_lasso_names_the_first_constraint() {
    let cases = [
        (
            "fair weak each M::Counter::acquire; ",
            (FairnessKind::Weak, FairnessGranularity::Each),
        ),
        (
            "fair strong M::Counter::acquire; ",
            (FairnessKind::Strong, FairnessGranularity::Whole),
        ),
        (
            "fair strong each M::Counter::acquire; fair weak whole M::Counter::release; ",
            (FairnessKind::Strong, FairnessGranularity::Each),
        ),
    ];
    for (constraints, (kind, granularity)) in cases {
        let (fixture, settled) = mutex_outcome(constraints);
        let TemporalOutcome::MissingFairnessPremise { constraint } = &settled else {
            panic!("{constraints}: not a missing premise: {settled:?}");
        };
        assert_eq!(
            shape(constraint),
            (kind, "acquire", granularity),
            "{constraints}"
        );
        // Built from the kind and granularity the case writes, on the
        // operation the clause's first constraint names.
        let expected = quire_semantic_value::fairness::FairnessConstraint::new(
            kind,
            fixture.clause.fairness()[0].operation().clone(),
            granularity,
        );
        assert_eq!(constraint, &expected, "{constraints}");
        assert_eq!(settled.category(), Category::Unsupported);
    }
}

/// FR-132-AC-2: the same lasso and clause with an empty fairness set is
/// false, a violation at position 0.
#[trace("TC-533", "FR-132-AC-2")]
#[test]
fn an_empty_fairness_set_over_a_supplied_lasso_is_evaluated() {
    let (_, settled) = mutex_outcome("");
    assert_eq!(settled, violated(0));
    assert_eq!(settled.category(), Category::Violation);
}

/// The outcome of `body` `on each` over the closed `Counter` trace `c.level`
/// 0, 1, 2 under event-position.
fn closed_each(body: &str) -> TemporalOutcome {
    outcome(
        &fixture(EVENT, "c: Counter", "on each (q: Counter)", body),
        finite(3),
        counters(&[0, 1, 2]),
    )
}

/// FR-327-AC-8: a temporal constant stays its authored value past closure
/// and through a grouping node, while the `holds` atom beside it is read at
/// each position and is false past the end: the constant and the atom are
/// not folded.
#[trace("TC-837", "FR-327-AC-8")]
#[test]
fn a_temporal_constant_keeps_its_value_past_closure_and_through_grouping() {
    assert_eq!(closed("always[0,3] true"), TemporalOutcome::Holds);
    assert_eq!(closed("always[0,3] (true)"), TemporalOutcome::Holds);
    assert_eq!(closed("always[0,3] holds(c.level <= 2)"), violated(0));
}

/// FR-327-AC-9: `until[1,2]` requires its left operand only from the lower
/// bound to the witness, and `release[1,2]` is its dual: each of the four
/// clauses of the AC settles on its own side.
#[trace("TC-837", "FR-327-AC-9")]
#[test]
fn until_and_release_with_a_lower_bound_settle_on_the_witness() {
    assert_eq!(
        closed("holds(c.level = 1) until[1,2] holds(c.level = 2)"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        closed("holds(c.level = 1) until[1,2] holds(c.level = 3)"),
        violated(0)
    );
    assert_eq!(
        closed("holds(c.level = 9) release[1,2] holds(c.level <= 2)"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        closed("holds(c.level = 9) release[1,2] holds(c.level = 1)"),
        violated(0)
    );
}

/// FR-327-AC-10: strong previous (`once[1,1]`) holds where the antecedent
/// holds and the previous position satisfies it, and names the activation
/// whose previous position does not.
#[trace("TC-837", "FR-327-AC-10")]
#[test]
fn a_strong_previous_is_read_at_the_activations_where_its_antecedent_holds() {
    assert_eq!(
        closed_each("holds(c.level = 1) implies once[1,1] holds(c.level = 0)"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        closed_each("holds(c.level = 1) implies once[1,1] holds(c.level = 5)"),
        violated(1)
    );
}

/// FR-327-AC-11: `once[0,1]` over an always-true atom holds at every
/// activation, and `historically[0,1] holds(c.level >= 1)` fails at position
/// 0, where the only position in the window is false.
#[trace("TC-837", "FR-327-AC-11")]
#[test]
fn once_and_historically_read_back_from_each_activation() {
    assert_eq!(
        closed_each("once[0,1] holds(c.level >= 0)"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        closed_each("historically[0,1] holds(c.level >= 1)"),
        violated(0)
    );
    // `historically` is not `once`: at position 2 the window holds levels 1
    // and 2, and `once[0,1] holds(c.level <= 1)` holds there while
    // `historically[0,1] holds(c.level <= 1)` does not.
    assert_eq!(
        closed_each("holds(c.level = 0) or once[0,1] holds(c.level <= 1)"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        closed_each("holds(c.level = 0) or historically[0,1] holds(c.level <= 1)"),
        violated(2)
    );
}

/// FR-327-AC-12: `since` and `triggered`, `until` and `release` are Boolean
/// duals: each form agrees with its dual written through `not`.
#[trace("TC-837", "FR-327-AC-12")]
#[test]
fn since_triggered_until_and_release_agree_with_their_duals() {
    let since = closed_each("holds(c.level >= 1) since[0,2] holds(c.level = 1)");
    let dual = closed_each("not (not holds(c.level >= 1) triggered[0,2] not holds(c.level = 1))");
    assert_eq!(since, dual);
    assert_eq!(since, violated(0));

    // `triggered` is not `since`: at position 2 `q` (level <= 1) is false and
    // `p` (level >= 1) has held only at the end, so `since` holds through
    // position 1 and `triggered` is false.
    assert_eq!(
        closed_each("holds(c.level = 0) or (holds(c.level >= 1) since[0,1] holds(c.level <= 1))"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        closed_each(
            "holds(c.level = 0) or (holds(c.level >= 1) triggered[0,1] holds(c.level <= 1))"
        ),
        violated(2)
    );

    for (operands, expected) in [
        (
            ("holds(c.level = 9)", "holds(c.level <= 2)"),
            TemporalOutcome::Holds,
        ),
        (("holds(c.level = 9)", "holds(c.level = 1)"), violated(0)),
    ] {
        let (left, right) = operands;
        let release = closed(&format!("{left} release[1,2] {right}"));
        let dual = closed(&format!("not (not {left} until[1,2] not {right})"));
        assert_eq!(release, dual, "{left} release {right}");
        assert_eq!(release, expected, "{left} release {right}");
    }
}

/// FR-327-AC-13: negation is pointwise: `eventually[0,1] not holds(c.level =
/// 9)` holds, and `eventually[0,1] holds(c.level = 9)` is false at position
/// 0, not an untimed reading of the whole formula.
#[trace("TC-837", "FR-327-AC-13")]
#[test]
fn negation_is_pointwise_inside_a_window() {
    assert_eq!(
        closed("eventually[0,1] not holds(c.level = 9)"),
        TemporalOutcome::Holds
    );
    assert_eq!(closed("eventually[0,1] holds(c.level = 9)"), violated(0));
}

/// [`run`] over a trace whose `holds` condition is refused at the positions
/// `refused`, returning the evaluator's whole result and the positions it
/// read.
fn run_refusing(
    fixture: &Fixture,
    trace: TemporalTrace,
    observations: Vec<Observation>,
    refused: &[u64],
) -> (Result<TemporalOutcome, TemporalRefusal>, Vec<u64>) {
    let mut letters = Letters::new(fixture, observations);
    letters.refused = refused.to_vec();
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let result = evaluate(&fixture.clause, trace, &mut letters, &mut meter);
    (result, letters.reads)
}

fn refused_at(index: u64) -> Result<TemporalOutcome, TemporalRefusal> {
    Err(TemporalRefusal::ConditionRefused {
        at: TemporalPosition::new(index),
    })
}

/// FR-327-AC-6, the bounded profile: `always[0,2] holds(c.level <= 1)` `on
/// origin` over `c.level` 0, 0, 0 with the condition refused at position 1
/// returns the refused condition at position 1 (not a fault) and reads
/// positions 0 and 1 only; with `c.level` 2 at position 0 the activation
/// evaluates false there and the result is `Completed(false)` at position 0,
/// the later refusal never deciding it.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_refused_condition_stops_reading_at_its_position_unless_an_earlier_activation_is_false() {
    let fx = counter(EVENT, "always[0,2] holds(c.level <= 1)");
    let (result, reads) = run_refusing(&fx, finite(3), counters(&[0, 0, 0]), &[1]);
    assert_eq!(result, refused_at(1));
    assert_eq!(reads, [0, 1]);

    let (result, reads) = run_refusing(&fx, finite(3), counters(&[2, 0, 0]), &[1]);
    assert_eq!(result, Ok(violated(0)));
    assert_eq!(
        result.as_ref().map(TemporalOutcome::category),
        Ok(Category::Violation)
    );
    assert_eq!(reads, [0, 1]);

    // A refusal at the first position is the result with nothing before it.
    let (result, reads) = run_refusing(&fx, finite(3), counters(&[0, 0, 0]), &[0]);
    assert_eq!(result, refused_at(0));
    assert_eq!(reads, [0]);

    // Without a refusal the same clause holds, so the refusal was the
    // condition's.
    let (result, _) = run_refusing(&fx, finite(3), counters(&[0, 0, 0]), &[]);
    assert_eq!(result, Ok(TemporalOutcome::Holds));
}

/// FR-327-AC-6, evaluation order: an atom undefined at an earlier position
/// is returned before a later refusal, and a refusal at an earlier position
/// before a later undefined atom; for an `on each` clause the first false
/// activation before the refusal is returned.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_refused_condition_and_an_undefined_atom_are_returned_in_evaluation_order() {
    let fx = counter(EVENT, "always[0,2] holds(2 / (2 - c.level) >= 1)");
    let (result, _) = run_refusing(&fx, finite(3), counters(&[2, 0, 0]), &[1]);
    assert_eq!(
        result,
        Ok(undefined_at(0, Undefined::DivisionByZero)),
        "undefined at 0 precedes the refusal at 1"
    );
    let (result, reads) = run_refusing(&fx, finite(3), counters(&[0, 2, 0]), &[0]);
    assert_eq!(
        result,
        refused_at(0),
        "the refusal at 0 precedes position 1"
    );
    assert_eq!(reads, [0]);

    let each = fixture(
        EVENT,
        "c: Counter",
        "on each (q: Counter)",
        "always[0,0] holds(c.level <= 1)",
    );
    let (result, _) = run_refusing(&each, finite(3), counters(&[0, 2, 0]), &[2]);
    assert_eq!(
        result,
        Ok(violated(1)),
        "the false activation at 1 precedes the refusal at 2"
    );
    let (result, _) = run_refusing(&each, finite(3), counters(&[0, 1, 0]), &[2]);
    assert_eq!(result, refused_at(2));
}

/// FR-327-AC-6, the infinite-trace profile: the same refusal at position 1
/// of the prefix 0, 1, 2 (FR-328) and of the lasso with an empty prefix and
/// loop 0, 1 (FR-329) is the refused condition at position 1.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_refused_condition_stops_an_infinite_trace_prefix_and_lasso() {
    let fx = counter(INFINITE, "always holds(c.level <= 5)");
    let (result, _) = run_refusing(&fx, finite(3), counters(&[0, 1, 2]), &[1]);
    assert_eq!(result, refused_at(1));
    let lasso = TemporalTrace::Lasso {
        prefix: 0,
        loop_len: 2,
    };
    let (result, reads) = run_refusing(&fx, lasso, counters(&[0, 1]), &[1]);
    assert_eq!(result, refused_at(1));
    assert_eq!(reads, [0, 1]);
    // A false atom at position 0 decides the prefix before the refusal.
    let (result, _) = run_refusing(&fx, finite(3), counters(&[6, 1, 2]), &[1]);
    assert_eq!(result, Ok(violated(0)));
}

/// [`run_refusing`] with `forced` readings on atoms.
fn run_forced(
    fixture: &Fixture,
    observations: Vec<Observation>,
    forced: &[(u64, &'static str, Forced)],
) -> Result<TemporalOutcome, TemporalRefusal> {
    let mut letters = Letters::new(fixture, observations);
    letters.forced = forced.to_vec();
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    evaluate(&fixture.clause, finite(3), &mut letters, &mut meter)
}

const LOW: &str = "holds(c.level <= 5)";
const HIGH: &str = "holds(c.level >= 0)";

/// FR-327 "Undefined evaluation", "Refused condition": an atom short of data
/// beside a refused condition at one position never decides the result, in
/// either clause-node order, and an earlier position short of data does not
/// hide a later refusal: "an earlier atom short of data SHALL NOT be
/// returned".
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_refused_condition_is_returned_over_an_atom_short_of_data_in_either_node_order() {
    let fx = counter(EVENT, &format!("always[0,2] ({LOW} and {HIGH})"));
    let cases = [
        [(1, LOW, Forced::ShortOfData), (1, HIGH, Forced::Refused)],
        [(1, LOW, Forced::Refused), (1, HIGH, Forced::ShortOfData)],
        [(0, LOW, Forced::ShortOfData), (1, HIGH, Forced::Refused)],
    ];
    for forced in cases {
        assert_eq!(
            run_forced(&fx, counters(&[0, 0, 0]), &forced),
            refused_at(1)
        );
    }
    // Without the refusal the short atom is the result, so the refusal above
    // is what it lost to.
    let result = run_forced(&fx, counters(&[0, 0, 0]), &[(1, LOW, Forced::ShortOfData)]);
    assert!(
        matches!(&result, Ok(TemporalOutcome::InsufficientData(found))
            if found.at == TemporalPosition::new(1)),
        "{result:?}"
    );
}

/// FR-327 "Undefined evaluation": at one position an atom undefined earlier
/// in clause-node order decides over a refused atom after it, and a refused
/// atom earlier in clause-node order stops the position before an undefined
/// atom after it is read.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn an_undefined_atom_before_a_refused_atom_at_one_position_is_returned() {
    let fx = counter(EVENT, &format!("always[0,2] ({LOW} and {HIGH})"));
    let result = run_forced(
        &fx,
        counters(&[0, 0, 0]),
        &[(1, LOW, Forced::Undefined), (1, HIGH, Forced::Refused)],
    );
    assert_eq!(result, Ok(undefined_at(1, Undefined::DivisionByZero)));
    let result = run_forced(
        &fx,
        counters(&[0, 0, 0]),
        &[(1, LOW, Forced::Refused), (1, HIGH, Forced::Undefined)],
    );
    assert_eq!(result, refused_at(1));
}

/// FR-327 "Refused condition": the refusal of an `on each` `when` guard
/// stops the run at its position as a refused condition; an activation
/// before it that evaluates false is returned instead.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_refused_when_guard_is_a_refused_condition_at_its_position() {
    let fx = fixture(
        EVENT,
        "c: Counter",
        "on each (q: Counter) when (true)",
        "always[0,0] holds(c.level <= 1)",
    );
    let run = |values: &[i64]| {
        let mut letters = Letters::new(&fx, counters(values));
        letters.refused_guards = vec![1];
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        evaluate(&fx.clause, finite(3), &mut letters, &mut meter)
    };
    assert_eq!(run(&[0, 0, 0]), refused_at(1));
    assert_eq!(run(&[2, 0, 0]), Ok(violated(0)));
}

/// FR-327 "Refused condition": the refusal of a capture initializer at an
/// activation is a refused condition at that position.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_refused_capture_is_a_refused_condition_at_its_position() {
    let fx = fixture(
        EVENT,
        "c: Counter",
        "on each (q: Counter)",
        "capture amount: Int[0, 9] = q.level; always[0,0] holds(c.level <= 1)",
    );
    let run = |values: &[i64]| {
        let mut letters = Letters::new(&fx, counters(values));
        letters.refused_captures = vec![1];
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        evaluate(&fx.clause, finite(3), &mut letters, &mut meter)
    };
    assert_eq!(run(&[0, 0, 0]), refused_at(1));
    assert_eq!(run(&[2, 0, 0]), Ok(violated(0)));
}
