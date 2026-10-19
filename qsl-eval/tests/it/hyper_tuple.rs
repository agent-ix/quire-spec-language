// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-175 (TC-600): the layer-5 evaluator reads a hyper body over a tuple of
//! behaviours in lockstep.
//!
//! Each clause is real complete-V1 source taken through S1, S2, the assembler
//! and the hyper check (FR-172). The `holds` conditions are not checked at
//! S3, so the letter reads each atom from its source text over the
//! components' observations, the seam the one clause evaluator (FR-107)
//! fills.

use ix_trace_rs::trace;
use qsl_eval::temporal::{
    evaluate_tuple, AtomReading, ComponentPosition, ComponentTrace, LetterStop, TemporalOutcome,
    TemporalPosition, TemporalRefusal, TupleLetters, UndefinedCause,
};
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    check_hyper_clauses, CheckedHyper, CheckedHyperClause, FormulaNode, FormulaNodeId,
    PackageDeclarations, SCALAR_LIMITS_UNLIMITED,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::key::hex;
use quire_exact::{Cancel, CancelCause, LimitKind, Meter, ScalarLimits, Undefined};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use crate::model_simulation::{acquire, counters_document, increment, PACKAGE};

const INFINITE: &str = "quire.temporal.infinite-trace/v1";

/// A unit over the `test/counters` model `M` with one infinite-trace hyper
/// clause `H` over two traces `a` and `b` of `M`, and its checked clause.
struct Fixture {
    text: String,
    clause: CheckedHyperClause,
}

fn fixture(body: &str) -> Fixture {
    fixture_with(vec![increment()], "", body)
}

/// [`fixture`] over a model that declares `operations`, with `alignment`
/// (`align skip {...}` or empty) after the second trace variable.
fn fixture_with(operations: Vec<serde_json::Value>, alignment: &str, body: &str) -> Fixture {
    let document = counters_document(operations, false);
    let packages = package_input([document.as_slice()]);
    let digest = *packages.keys().next().expect("one model");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile inf = \"{INFINITE}\";\n\
         model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         hyper H using inf over behaviours (c: M::Counter) \
         forall trace a of M forall trace b of M {alignment} {{ {body} }}\n",
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
    let package = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles");
    let mut checked = check_hyper_clauses(&package);
    let clause = match checked.pop() {
        Some(Ok(CheckedHyper::Behaviours(clause))) if checked.is_empty() => *clause,
        other => panic!("{body}: one checked hyper clause, not {other:?}"),
    };
    Fixture { text, clause }
}

/// A letter over one observation sequence per component, indexed by the
/// component's own position. It reads `holds(c.value @ x = c.value @ y)` and
/// `holds(c.value @ x = k)`, and records the joint positions and the stutter
/// flags it was given.
struct Letters<'a> {
    fixture: &'a Fixture,
    observations: Vec<Vec<i64>>,
    joints: Vec<u64>,
    reads: Vec<(FormulaNodeId, u64)>,
    stutters: Vec<(u64, Vec<bool>)>,
}

impl<'a> Letters<'a> {
    fn new(fixture: &'a Fixture, observations: Vec<Vec<i64>>) -> Self {
        Self {
            fixture,
            observations,
            joints: Vec::new(),
            reads: Vec::new(),
            stutters: Vec::new(),
        }
    }

    fn component(name: &str) -> usize {
        match name {
            "a" => 0,
            "b" => 1,
            other => panic!("unmodelled trace variable {other}"),
        }
    }
}

impl TupleLetters for Letters<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        joint: TemporalPosition,
        components: &[ComponentPosition],
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        self.joints.push(joint.index());
        self.reads.push((atom, joint.index()));
        self.stutters.push((
            joint.index(),
            components.iter().map(|c| c.stutter).collect(),
        ));
        let Some(FormulaNode::Holds { span }) =
            self.fixture.clause.body().nodes().get(atom.index())
        else {
            panic!("the letter reads only holds atoms");
        };
        let text = &self.fixture.text[span.start..span.end];
        let inner = text
            .strip_prefix("holds(")
            .and_then(|rest| rest.strip_suffix(')'))
            .expect("a holds atom");
        let words: Vec<&str> = inner.split_whitespace().collect();
        let read = |name: &str| {
            let index = Self::component(name);
            let position =
                usize::try_from(components[index].position.index()).expect("a small position");
            self.observations[index][position]
        };
        let holds = match words.as_slice() {
            ["c.value", "@", x, "=", "c.value", "@", y] => read(x) == read(y),
            ["1", "/", "c.value", "@", x, "=", k] => {
                let divisor = read(x);
                if divisor == 0 {
                    return Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                        Undefined::DivisionByZero,
                    )));
                }
                1 / divisor == k.parse::<i64>().expect("an integer")
            }
            ["c.value", "@", x, "=", k] => read(x) == k.parse::<i64>().expect("an integer"),
            other => panic!("unmodelled atom {other:?}"),
        };
        Ok(if holds {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

const ALWAYS_EQUAL: &str = "always holds(c.value @ a = c.value @ b)";

fn run<'a>(
    fixture: &'a Fixture,
    components: &[ComponentTrace],
    observations: Vec<Vec<i64>>,
) -> (TemporalOutcome, Letters<'a>) {
    let mut letters = Letters::new(fixture, observations);
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let outcome = evaluate_tuple(&fixture.clause, components, &mut letters, &mut meter, None)
        .expect("the tuple is readable");
    (outcome, letters)
}

fn violated(index: u64) -> TemporalOutcome {
    TemporalOutcome::Violated {
        at: TemporalPosition::new(index),
    }
}

fn lasso(prefix: u64, loop_len: u64) -> ComponentTrace {
    ComponentTrace::Lasso { prefix, loop_len }
}

/// ADR-023 section 8: the vault `V`, an object with a secret `h` and a public
/// output `l`, and the `step(i)` operation under its two post clauses. The
/// tuple evaluator takes the traces' observations, so the test drives the
/// vault's transition relation to produce them; the counters model's `value`
/// stands for `l`, the observation the clause reads.
#[derive(Clone, Copy)]
enum Vault {
    /// `self.l = (i + self.h) mod 2`.
    Leaky,
    /// `self.l = i`.
    Secure,
}

impl Vault {
    /// The state `(h, l)` after `step(input)` from `(h, _)`.
    fn step(self, (h, _): (i64, i64), input: i64) -> (i64, i64) {
        match self {
            Self::Leaky => (h, (input + h) % 2),
            Self::Secure => (h, input),
        }
    }

    /// The `l` observations of the behaviour from `initial` taking `step(0)`
    /// at every joint step, up to and including its loop entry position.
    fn observations(self, initial: (i64, i64), positions: usize) -> Vec<i64> {
        let mut state = initial;
        let mut seen = vec![state.1];
        for _ in 1..positions {
            state = self.step(state, 0);
            seen.push(state.1);
        }
        seen
    }
}

/// FR-175-AC-1: ADR-023 section 8.1's leaky counterexample, `a` from `(0, 0)`
/// and `b` from `(1, 0)` each taking `step(0)` with loop entry 1, is `false`
/// at joint position 1; the same pair over the secure model is `true`.
#[trace("TC-600", "FR-175-AC-1")]
#[test]
fn the_vault_pair_is_false_at_joint_position_one_when_leaky_and_true_when_secure() {
    let fixture = fixture(ALWAYS_EQUAL);
    // Two positions, the second the loop entry: loop entry 1.
    let shape = [lasso(1, 1), lasso(1, 1)];
    let pair = |vault: Vault| vec![vault.observations((0, 0), 2), vault.observations((1, 0), 2)];
    assert_eq!(pair(Vault::Leaky), [vec![0, 0], vec![0, 1]]);
    let (leaky, _) = run(&fixture, &shape, pair(Vault::Leaky));
    assert_eq!(leaky, violated(1));
    let (secure, _) = run(&fixture, &shape, pair(Vault::Secure));
    assert_eq!(secure, TemporalOutcome::Holds);
}

/// FR-175-AC-2: a lasso with loop length 2 and one with loop length 3 are
/// read over a joint period of 6. Joint offsets 0..=2 pair equal outputs and
/// offset 3 pairs `a`'s second loop position with `b`'s first, so the body
/// is `false` at joint position 3 and `trace_position` names it. A reading
/// over the longer loop alone (period 3) would never reach offset 3. (The
/// spec's "fifth position" cannot occur for this pair: the offsets 0..=3
/// already tie every loop observation together.)
#[trace("TC-600", "FR-175-AC-2")]
#[test]
fn a_tuple_of_two_and_three_position_loops_is_read_over_a_joint_period_of_six() {
    let fixture = fixture(ALWAYS_EQUAL);
    let shape = [lasso(0, 2), lasso(0, 3)];
    let (differing, _) = run(&fixture, &shape, vec![vec![0, 1], vec![0, 1, 0]]);
    assert_eq!(differing, violated(3));
    // A difference at offset 2 is found at offset 2.
    let (earlier, _) = run(&fixture, &shape, vec![vec![0, 1], vec![0, 1, 2]]);
    assert_eq!(earlier, violated(2));
    // Equal outputs hold and read exactly one period: joint positions 0..=5.
    let (equal, letters) = run(&fixture, &shape, vec![vec![5, 5], vec![5, 5, 5]]);
    assert_eq!(equal, TemporalOutcome::Holds);
    assert_eq!(letters.joints, [0, 1, 2, 3, 4, 5]);
}

/// FR-175-AC-2: the joint stem is the longest component stem. `a` has a stem
/// of 3 and a loop of 2, `b` a loop of 3, so the joint stem is 3; joint
/// positions 0..=3 pair equal outputs and joint position 4 pairs `a`'s
/// second loop position (7) with `b`'s position 1 (1).
#[trace("TC-600", "FR-175-AC-2")]
#[test]
fn the_joint_stem_is_the_longest_component_stem() {
    let fixture = fixture(ALWAYS_EQUAL);
    let shape = [lasso(3, 2), lasso(0, 3)];
    let (differing, letters) = run(&fixture, &shape, vec![vec![0, 1, 2, 0, 7], vec![0, 1, 2]]);
    assert_eq!(differing, violated(4));
    assert_eq!(letters.joints.iter().max(), Some(&8));
    let (equal, letters) = run(&fixture, &shape, vec![vec![5, 5, 5, 5, 5], vec![5, 5, 5]]);
    assert_eq!(equal, TemporalOutcome::Holds);
    assert_eq!(letters.joints.iter().max(), Some(&8));
}

/// FR-175-AC-2: a past operator under `always` gives the same value at a loop
/// position and at its unrolled copy, so a tuple and the same behaviours
/// written with unrolled loops evaluate alike, `trace_position` included.
#[trace("TC-600", "FR-175-AC-2")]
#[test]
fn a_past_operator_gives_the_same_outcome_at_a_loop_position_and_its_unrolled_copy() {
    let past =
        fixture("always (holds(c.value @ a = 1) implies not once[2,2] holds(c.value @ b = 2))");
    let unrolled = |values: &[i64], times: usize| values.repeat(times);
    // The violation needs loop history: `a` reads 1 at joint position 2, two
    // joint steps after `b` read 2 at position 0, which is only so in the
    // second copy of a loop of length 2. Both encodings fix it at position 2.
    let compact = run(
        &past,
        &[lasso(0, 2), lasso(0, 2)],
        vec![vec![1, 0], vec![2, 0]],
    )
    .0;
    let twice = run(
        &past,
        &[lasso(0, 4), lasso(0, 4)],
        vec![unrolled(&[1, 0], 2), unrolled(&[2, 0], 2)],
    )
    .0;
    assert_eq!(compact, violated(2));
    assert_eq!(compact, twice);

    // The 2-and-3 pair against its common unrolling (4 and 6).
    for (a, b) in [
        (vec![1, 0], vec![2, 0, 0]),
        (vec![1, 0], vec![0, 0, 0]),
        (vec![0, 1], vec![2, 0, 3]),
    ] {
        let compact = run(
            &past,
            &[lasso(0, 2), lasso(0, 3)],
            vec![a.clone(), b.clone()],
        )
        .0;
        let twice = run(
            &past,
            &[lasso(0, 4), lasso(0, 6)],
            vec![unrolled(&a, 2), unrolled(&b, 2)],
        )
        .0;
        assert_eq!(compact, twice, "{a:?} {b:?}");
    }
}

/// FR-175 "Work": one unit per (node, joint position) visit, whether the
/// tuple holds or is violated, so locating `trace_position` costs the caller
/// nothing more; and each atom is read once per represented joint position.
#[trace("TC-600")]
#[test]
fn locating_the_false_position_costs_no_more_work_than_the_evaluation() {
    let fixture = fixture(ALWAYS_EQUAL);
    let shape = [lasso(0, 8), lasso(0, 8)];
    let holding = vec![vec![5; 8], vec![5; 8]];
    let mut differing = holding.clone();
    differing[1][6] = 6;
    let consumed = |limits: ScalarLimits, values: Vec<Vec<i64>>| {
        let mut letters = Letters::new(&fixture, values);
        let mut meter = Meter::new(limits);
        let outcome = evaluate_tuple(&fixture.clause, &shape, &mut letters, &mut meter, None)
            .expect("readable");
        (outcome, meter.consumed(LimitKind::WorkUnits), letters.reads)
    };
    let (held, held_work, _) = consumed(SCALAR_LIMITS_UNLIMITED, holding);
    assert_eq!(held, TemporalOutcome::Holds);
    let (violated_outcome, work, reads) = consumed(SCALAR_LIMITS_UNLIMITED, differing.clone());
    assert_eq!(violated_outcome, violated(6));
    assert_eq!(work, held_work);
    let distinct: std::collections::BTreeSet<_> = reads.iter().collect();
    assert_eq!(distinct.len(), reads.len());
    assert!(reads.iter().all(|(_, joint)| *joint < 8));
    let within = |work_units| ScalarLimits {
        work_units,
        ..SCALAR_LIMITS_UNLIMITED
    };
    // Headroom above what the evaluation needs still gives the verdict.
    assert_eq!(consumed(within(work + 2), differing.clone()).0, violated(6));
    let (short, _, _) = consumed(within(work - 1), differing);
    assert!(matches!(short, TemporalOutcome::Incomplete(_)), "{short:?}");
}

/// FR-276: the position search polls the caller's cancel handle, so a
/// cancellation after the evaluation's own charges stops the search with no
/// position.
#[trace("TC-600")]
#[test]
fn a_cancellation_during_the_position_search_stops_the_evaluation() {
    let fixture = fixture(ALWAYS_EQUAL);
    let shape = [lasso(0, 8), lasso(0, 8)];
    let mut values = vec![vec![5; 8], vec![5; 8]];
    values[1][6] = 6;
    // The charges the evaluation itself polls, with a handle that never trips.
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED).with_cancel(Cancel::new());
    let mut letters = Letters::new(&fixture, values.clone());
    evaluate_tuple(&fixture.clause, &shape, &mut letters, &mut meter, None).expect("readable");
    let own = meter.admission_count();

    // A handle that cancels at the first poll after those.
    let polls = Arc::new(AtomicU64::new(0));
    let handle: Arc<OnceLock<Cancel>> = Arc::new(OnceLock::new());
    let cancel = {
        let (polls, handle) = (polls.clone(), handle.clone());
        Cancel::observing(move || {
            if polls.fetch_add(1, Ordering::Relaxed) == own {
                if let Some(cancel) = handle.get() {
                    cancel.cancel(CancelCause::Deadline);
                }
            }
        })
    };
    handle.set(cancel.clone()).expect("set once");
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED).with_cancel(cancel.clone());
    let mut letters = Letters::new(&fixture, values);
    let outcome = evaluate_tuple(
        &fixture.clause,
        &shape,
        &mut letters,
        &mut meter,
        Some(&cancel),
    )
    .expect("readable");
    assert!(
        matches!(outcome, TemporalOutcome::Incomplete(_)),
        "{outcome:?}"
    );
    assert_eq!(cancel.tripped(), Some(CancelCause::Deadline));
}

/// A violation at the last unrolled position whose bounded-future part reads
/// beyond it: the position search runs past the unrolled length by the
/// body's future reach, so the compact and the unrolled encodings agree.
#[trace("TC-600", "FR-175-AC-2")]
#[test]
fn a_violation_at_the_last_unrolled_position_is_located_through_the_future_reach() {
    let fixture = fixture(
        "always ((holds(c.value @ a = 1) and once[2,2] holds(c.value @ b = 2)) \
         implies eventually[1,1] holds(c.value @ a = 1))",
    );
    let compact = run(
        &fixture,
        &[lasso(0, 2), lasso(0, 2)],
        vec![vec![0, 1], vec![0, 2]],
    )
    .0;
    let unrolled = run(
        &fixture,
        &[lasso(0, 4), lasso(0, 4)],
        vec![vec![0, 1, 0, 1], vec![0, 2, 0, 2]],
    )
    .0;
    assert_eq!(compact, unrolled);
    assert_eq!(compact, violated(4));
}

/// FR-175 "Outputs": a body that is undefined on the tuple is undefined at
/// the first joint position an atom is, with its cause.
#[trace("TC-600")]
#[test]
fn an_atom_with_no_value_makes_the_body_undefined_at_the_first_such_joint_position() {
    let fixture = fixture("always holds(1 / c.value @ a = 1)");
    let shape = [lasso(0, 2), lasso(0, 3)];
    // `a`'s second loop position reads 0, so `1 / value` has no value there:
    // joint position 1.
    let mut letters = Letters::new(&fixture, vec![vec![1, 0], vec![0, 0, 0]]);
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let outcome =
        evaluate_tuple(&fixture.clause, &shape, &mut letters, &mut meter, None).expect("readable");
    let TemporalOutcome::Undefined(undefined) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(undefined.at, TemporalPosition::new(1));
    assert_eq!(
        undefined.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
}

/// FR-175 covers lockstep only: a clause with `align skip` (FR-178) is
/// refused as not yet implemented.
#[trace("TC-600")]
#[test]
fn a_clause_with_align_skip_is_refused_as_not_yet_implemented() {
    let fixture = fixture_with(
        vec![increment(), acquire()],
        "align skip { M::Counter::acquire }",
        ALWAYS_EQUAL,
    );
    let mut letters = Letters::new(&fixture, vec![vec![0], vec![0]]);
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let refusal = evaluate_tuple(
        &fixture.clause,
        &[lasso(0, 1), lasso(0, 1)],
        &mut letters,
        &mut meter,
        None,
    )
    .expect_err("align skip is not lockstep");
    assert_eq!(refusal, TemporalRefusal::AlignSkipNotImplemented);
    assert_eq!(
        refusal.catalog_code(),
        qsl_foundation::CatalogCode::new("unsupported_construct", "not-yet-implemented")
    );
}

/// FR-175-AC-3: a component that ends at a terminal state reads its terminal
/// observation at every later joint position and takes the `stutter` step
/// there; an interval counts joint positions.
#[trace("TC-600", "FR-175-AC-3")]
#[test]
fn a_terminal_component_stutters_at_its_terminal_state_and_intervals_count_joint_positions() {
    let fixture = fixture(ALWAYS_EQUAL);
    // `a` ends at its terminal state (position 1, value 1); `b` loops over
    // three positions, so the joint behaviour runs to joint position 4. `a`
    // reads its terminal value 1 at every later joint position, so the pair
    // is equal throughout, and `a`'s step into joint positions 2 and 3 is the
    // stutter step.
    let shape = [ComponentTrace::Terminal { len: 2 }, lasso(0, 3)];
    let (outcome, letters) = run(&fixture, &shape, vec![vec![1, 1], vec![1, 1, 1]]);
    assert_eq!(outcome, TemporalOutcome::Holds);
    assert_eq!(
        letters.stutters,
        [
            (0, vec![false, false]),
            (1, vec![false, false]),
            (2, vec![true, false]),
            (3, vec![true, false]),
            (4, vec![true, false]),
        ]
    );
    // The terminal observation is what `a` reads: a `b` that differs from it
    // at joint position 2 is false there.
    let (outcome, _) = run(&fixture, &shape, vec![vec![1, 1], vec![1, 1, 2]]);
    assert_eq!(outcome, violated(2));

    // The other component has period 1: the joint stem is `a`'s stutter
    // position, so the flag is read there and at every later joint step.
    let (outcome, letters) = run(
        &fixture,
        &[ComponentTrace::Terminal { len: 2 }, lasso(0, 1)],
        vec![vec![1, 1], vec![1]],
    );
    assert_eq!(outcome, TemporalOutcome::Holds);
    assert_eq!(
        letters.stutters,
        [
            (0, vec![false, false]),
            (1, vec![false, false]),
            (2, vec![true, false]),
        ]
    );

    let interval = self::fixture("eventually[0,2] holds(c.value @ a = 1)");
    let a_terminal = |len, values: Vec<i64>| {
        run(
            &interval,
            &[ComponentTrace::Terminal { len }, lasso(0, 1)],
            vec![values, vec![0]],
        )
        .0
    };
    // The 1 at joint position 2 is inside [0, 2]; at joint position 3 it is
    // not, and the `false` is fixed once position 2 is read.
    assert_eq!(a_terminal(3, vec![0, 0, 1]), TemporalOutcome::Holds);
    assert_eq!(a_terminal(4, vec![0, 0, 0, 1]), violated(2));
}

/// FR-175 "Inputs": one component per variable, none empty.
#[trace("TC-600")]
#[test]
fn a_tuple_of_the_wrong_size_or_with_an_empty_component_is_refused() {
    let fixture = fixture(ALWAYS_EQUAL);
    let mut letters = Letters::new(&fixture, vec![vec![0]]);
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    assert_eq!(
        evaluate_tuple(
            &fixture.clause,
            &[lasso(0, 1)],
            &mut letters,
            &mut meter,
            None
        ),
        Err(TemporalRefusal::TupleArity {
            expected: 2,
            found: 1
        })
    );
    assert_eq!(
        evaluate_tuple(
            &fixture.clause,
            &[lasso(0, 1), lasso(0, 0)],
            &mut letters,
            &mut meter,
            None
        ),
        Err(TemporalRefusal::EmptyLoop)
    );
}
