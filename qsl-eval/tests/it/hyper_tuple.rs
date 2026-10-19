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
    TemporalPosition, TemporalRefusal, TupleLetters,
};
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    check_hyper_clauses, CheckedHyper, CheckedHyperClause, FormulaNode, FormulaNodeId,
    PackageDeclarations, SCALAR_LIMITS_UNLIMITED,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::key::hex;
use quire_exact::Meter;

use crate::model_simulation::{counters_document, increment, PACKAGE};

const INFINITE: &str = "quire.temporal.infinite-trace/v1";

/// A unit over the `test/counters` model `M` with one infinite-trace hyper
/// clause `H` over two traces `a` and `b` of `M`, and its checked clause.
struct Fixture {
    text: String,
    clause: CheckedHyperClause,
}

fn fixture(body: &str) -> Fixture {
    let document = counters_document(vec![increment()], false);
    let packages = package_input([document.as_slice()]);
    let digest = *packages.keys().next().expect("one model");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile inf = \"{INFINITE}\";\n\
         model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         hyper H using inf over behaviours (c: M::Counter) \
         forall trace a of M forall trace b of M {{ {body} }}\n",
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
    stutters: Vec<(u64, Vec<bool>)>,
}

impl<'a> Letters<'a> {
    fn new(fixture: &'a Fixture, observations: Vec<Vec<i64>>) -> Self {
        Self {
            fixture,
            observations,
            joints: Vec::new(),
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
    let outcome = evaluate_tuple(&fixture.clause, components, &mut letters, &mut meter)
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

/// FR-175-AC-1: the leaky pair, outputs differing at joint position 1 and
/// loop entry 1, is `false` at position 1; the secure pair is `true`.
#[trace("TC-600", "FR-175-AC-1")]
#[test]
fn a_leaky_pair_is_false_at_the_first_differing_joint_position_and_a_secure_pair_holds() {
    let fixture = fixture(ALWAYS_EQUAL);
    let shape = [lasso(1, 1), lasso(1, 1)];
    let (leaky, _) = run(&fixture, &shape, vec![vec![0, 5], vec![0, 6]]);
    assert_eq!(leaky, violated(1));
    let (secure, _) = run(&fixture, &shape, vec![vec![0, 5], vec![0, 5]]);
    assert_eq!(secure, TemporalOutcome::Holds);
}

/// FR-175-AC-2: a lasso of loop length 2 and one of length 3 are read over a
/// joint period of 6 after the longest stem, each component by the
/// prefix-then-loop rule, and `trace_position` names the joint position.
#[trace("TC-600", "FR-175-AC-2")]
#[test]
fn a_tuple_of_two_and_three_position_loops_is_read_over_a_joint_period_of_six() {
    let fixture = fixture(ALWAYS_EQUAL);
    // `a` has a stem of 3 and a loop of 2, `b` a loop of 3: the joint stem is
    // 3 and the joint period 6. Joint positions 0..=3 pair equal outputs;
    // joint position 4 pairs `a`'s second loop position (7) with `b`'s
    // position 1 (1).
    let shape = [lasso(3, 2), lasso(0, 3)];
    let (differing, letters) = run(&fixture, &shape, vec![vec![0, 1, 2, 0, 7], vec![0, 1, 2]]);
    assert_eq!(differing, violated(4));
    assert_eq!(letters.joints.iter().max(), Some(&8));

    // Equal outputs throughout hold, and every joint position
    // of the stem and of one period is read: 3 + 6 positions.
    let (equal, letters) = run(&fixture, &shape, vec![vec![5, 5, 5, 5, 5], vec![5, 5, 5]]);
    assert_eq!(equal, TemporalOutcome::Holds);
    assert_eq!(letters.joints.iter().max(), Some(&8));
    assert!(letters.joints.iter().all(|joint| *joint < 9));
}

/// FR-175-AC-2: a past operator under `always` is the same at a loop
/// position and at its unrolled copy, so the same behaviours written with
/// their loops unrolled once evaluate alike.
#[trace("TC-600", "FR-175-AC-2")]
#[test]
fn a_past_operator_gives_the_same_value_at_a_loop_position_and_its_unrolled_copy() {
    let fixture =
        fixture("always (holds(c.value @ a = 1) implies once[1,1] holds(c.value @ b = 2))");
    for (a, b, expected) in [
        // `a` reads 1 at its loop position 1; `b` read 2 one joint step earlier.
        (vec![0, 1], vec![0, 2], false),
        (vec![0, 1], vec![2, 0], true),
    ] {
        let compact = [lasso(0, 2), lasso(0, 2)];
        let (value, _) = run(&fixture, &compact, vec![a.clone(), b.clone()]);
        let unrolled = [lasso(0, 4), lasso(0, 4)];
        let twice = |values: &Vec<i64>| values.repeat(2);
        let (unrolled_value, _) = run(&fixture, &unrolled, vec![twice(&a), twice(&b)]);
        assert_eq!(value, unrolled_value);
        assert_eq!(value == TemporalOutcome::Holds, expected, "{a:?} {b:?}");
    }
}

/// FR-175-AC-3: a component that ends at a terminal state reads its terminal
/// observation at every later joint position and takes the `stutter` step
/// there; an interval counts joint positions.
#[trace("TC-600", "FR-175-AC-3")]
#[test]
fn a_terminal_component_stutters_at_its_terminal_state_and_intervals_count_joint_positions() {
    let fixture = fixture(ALWAYS_EQUAL);
    // `a` ends at its terminal state (position 1, value 1); `b` loops over
    // three positions, so the joint behaviour runs to joint position 3. `a`
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
        ]
    );
    // The terminal observation is what `a` reads: a `b` that differs from it
    // at joint position 2 is false there.
    let (outcome, _) = run(&fixture, &shape, vec![vec![1, 1], vec![1, 1, 2]]);
    assert_eq!(outcome, violated(2));

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
        evaluate_tuple(&fixture.clause, &[lasso(0, 1)], &mut letters, &mut meter),
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
            &mut meter
        ),
        Err(TemporalRefusal::EmptyLoop)
    );
}
