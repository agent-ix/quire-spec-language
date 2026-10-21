// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-213 (TC-658): interval operators over a protocol subject measure
//! distance in counted steps (ADR-027 PB-2, PB-3).
//!
//! ADR-027 §7's repaired `Fill` (`TwoPre: self.v <= 1`) and §7.1's `Pay`
//! (`bal` is `Cell.v`, see `crate::protocol_compensation`) are read as
//! behaviours of `ProtocolSystem<Sc>`: the evaluator reads each position's
//! atom through the state and each step's `counts` flag, and EN-1 checks the
//! same claim forms on the product with the claim's counted monitor.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{
    check_model, check_protocol, ModelCheckRequest, ProtocolCheckRequest,
};
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ProtocolSubject};
use qsl_eval::simulation::model::{ExpansionLimits, ModelSimState};
use qsl_eval::simulation::protocol::{
    PositionAnchor, ProtocolSystem, ProtocolTransition, Sc, StepKind,
};
use qsl_eval::simulation::{SubjectSystem, TransitionSystem};
use qsl_eval::temporal::{
    evaluate_at, AtomReading, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TemporalTrace,
};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::CheckedPackage;
use qsl_replay::{
    replay_protocol_trace, settle_protocol_check, ModelCheckItem, ModelCheckLimits,
    ModelCheckOutcome, ModelTraceSubject, ProtocolCertificateRequest, ProtocolTraceSubject,
    ReplayLimits, ResultLabel, TemporalCounterexample, UndecidedCause,
};
use qsl_semantics::check::{CheckedTemporalClause, FormulaNodeId};
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Cancel, Incomplete, Meter};

use crate::counter::UNLIMITED;
use crate::fill::{
    attempt, contracts, document, fill_unit, instance, model_subject, package, package_id,
    protocol, reachable, sc_system, subject, unit_text, universe, v, Documents, Reached,
};
use crate::model_settle::{envelope_at, request};
use crate::protocol_compensation::pay;

/// A claim over `k: M::Cell`, `on origin`, under event-position
/// false-extension.
fn claim(formula: &str) -> String {
    format!(
        "temporal Claim using tb over (k: M::Cell) clock \"model-steps\" on origin \
         {{ {formula} }}\n"
    )
}

/// `body` from one initial snapshot, `c.v = 0`, with its checked package.
struct Case {
    body: String,
    documents: Documents,
    initial: Vec<DocumentRef>,
    checked: CheckedPackage,
}

impl Case {
    fn new(body: &str) -> Self {
        let mut documents = Documents::new();
        let initial = vec![documents.start("s0", &[("c", 0)])];
        let checked = package(body);
        Self {
            body: body.to_owned(),
            documents,
            initial,
            checked,
        }
    }

    fn subject(&self, protocol: &str) -> ProtocolSubject<'_> {
        subject(
            &self.checked,
            &self.documents,
            self.initial.clone(),
            protocol,
            "c",
        )
    }

    fn clause(&self) -> &CheckedTemporalClause {
        self.checked.graph().temporal_clauses()[0].clause()
    }

    /// EN-1 over `protocol`'s claim, read through `letters`.
    fn check(
        &self,
        protocol: &str,
        letters: &dyn ModelLetters,
    ) -> ModelCheckOutcome<ProtocolTransition> {
        check_protocol(
            ProtocolCheckRequest {
                subject: self.subject(protocol),
                item: ModelCheckItem::Clause {
                    clause: self.clause(),
                    letters,
                },
                max_depth: None,
                max_store_buffer: None,
                max_live_instances: None,
                limits: ModelCheckLimits::default(),
            },
            &Cancel::new(),
        )
        .expect("the protocol subject checks")
    }

    /// FR-127's label of `outcome`, its counterexample replayed by FR-217.
    fn label(
        &self,
        protocol: &str,
        letters: &dyn ModelLetters,
        outcome: &ModelCheckOutcome<ProtocolTransition>,
    ) -> ResultLabel {
        let id = package_id(&self.body);
        let unit = unit_text(&self.body).into_bytes();
        let node = &self.checked.graph().temporal_clauses()[0];
        let clause_node = WireNodeId::from_digest(*node.identity().as_bytes());
        let occurrence = OccurrenceKey::new(clause_node, node.claim().clone());
        let replay = |payload: &TemporalCounterexample<ProtocolTransition>| {
            let subject = self.subject(protocol);
            replay_protocol_trace(
                request(&document(), self.documents.snapshots.iter(), &unit, id),
                &ProtocolTraceSubject {
                    model: ModelTraceSubject {
                        initial: self.initial.clone(),
                        universes: universe(&["c"]),
                        expansion: ExpansionLimits {
                            max_candidates: 1_000_000,
                        },
                    },
                    protocol: protocol.to_owned(),
                    over: instance("c"),
                    roles: subject.roles,
                    memory: subject.memory,
                    max_store_buffer: None,
                    max_live_instances: None,
                },
                &envelope_at(&unit, id, payload.clone(), clause_node, occurrence.clone()),
                letters,
                ReplayLimits::default(),
            )
        };
        let record = settle_protocol_check(
            outcome,
            &ModelCheckLimits::default(),
            &id,
            &ProtocolCertificateRequest {
                subject: self.subject(protocol),
                item: ModelCheckItem::Clause {
                    clause: self.clause(),
                    letters,
                },
                max_store_buffer: None,
                max_live_instances: None,
            },
            &Cancel::new(),
            replay,
        );
        record.label
    }
}

/// The claim's one atom, a predicate of the bound cell's `v`.
struct CellV(fn(i64) -> bool);

impl ModelLetters for CellV {
    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        Ok(if (self.0)(v(state, &instance.key)) {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

/// One behaviour of a protocol system: its states and the steps between
/// them, read by the evaluator as a finite trace.
struct Behaviour {
    states: Vec<ModelSimState>,
    steps: Vec<ProtocolTransition>,
    holds: fn(i64) -> bool,
    /// Whether the letter reports the steps' own counts, or counts every
    /// step as a model subject does.
    by_kind: bool,
}

impl Behaviour {
    /// The behaviour of `system` from its initial state taking, at each
    /// state, the step `pick` chooses among those enabled there.
    fn follow(
        system: &ProtocolSystem<'_, Sc>,
        holds: fn(i64) -> bool,
        pick: impl Fn(&[(ProtocolTransition, usize)]) -> usize,
    ) -> Self {
        let reached = reachable(system);
        let atoms = |reached: &Reached| system.atom_state(&reached.state).into_owned();
        let mut at = 0;
        let mut states = vec![atoms(&reached[0])];
        let mut steps = Vec::new();
        while !reached[at].steps.is_empty() {
            let (transition, next) = &reached[at].steps[pick(&reached[at].steps)];
            steps.push(transition.clone());
            states.push(atoms(&reached[*next]));
            at = *next;
        }
        Self {
            states,
            steps,
            holds,
            by_kind: true,
        }
    }

    fn at(&self, clause: &CheckedTemporalClause, position: u64) -> TemporalOutcome {
        let len = u64::try_from(self.states.len()).expect("a small trace");
        evaluate_at(
            clause,
            TemporalTrace::Finite { len },
            TemporalPosition::new(position),
            &mut self.letters(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the trace is readable")
    }

    fn letters(&self) -> Letters<'_> {
        Letters(self)
    }

    /// The positions after the first at which a counted step was taken.
    fn counted(&self) -> Vec<bool> {
        self.steps.iter().map(ProtocolTransition::counts).collect()
    }
}

struct Letters<'a>(&'a Behaviour);

impl TemporalLetters for Letters<'_> {
    fn atom(
        &mut self,
        _atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let index = usize::try_from(position.index()).expect("a small position");
        Ok(if (self.0.holds)(v(&self.0.states[index], "c")) {
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
        panic!("these claims have no `when` condition");
    }

    fn counts_step(&self, position: TemporalPosition) -> bool {
        let step = usize::try_from(position.index()).expect("a small position") - 1;
        !self.0.by_kind || self.0.steps[step].counts()
    }
}

fn violated(position: u64) -> TemporalOutcome {
    TemporalOutcome::Violated {
        at: TemporalPosition::new(position),
    }
}

/// The step whose node ends with `suffix`, else the first.
fn preferring(suffix: &'static str) -> impl Fn(&[(ProtocolTransition, usize)]) -> usize {
    move |steps| {
        steps
            .iter()
            .position(|(step, _)| step.node().is_some_and(|node| node.ends_with(suffix)))
            .unwrap_or(0)
    }
}

fn is_three(value: i64) -> bool {
    value == 3
}

fn at_least_two(value: i64) -> bool {
    value >= 2
}

fn is_zero(value: i64) -> bool {
    value == 0
}

fn repaired_fill(formula: &str) -> Case {
    Case::new(&(fill_unit("self.v <= 1", "") + &claim(formula)))
}

/// FR-213-AC-1 (TC-658): on the behaviour of the repaired `Fill` that takes
/// `B` before `A`, positions 1 (`fork`), 4 (`join`) and 5 (`finish`) carry
/// `protocol` anchors and positions 2 and 3 the anchors of `setTwo` and
/// `inc`; the counted steps are `attempt(B)` and `attempt(A)`, so
/// `eventually[0,2] holds(k.v = 3)` holds at position 0 and
/// `eventually[0,1] holds(k.v = 3)` does not. Read as a model subject reads
/// it, with every step counted, `k.v = 3` is three steps away and
/// `eventually[0,2]` fails.
#[trace("TC-658", "FR-213-AC-1")]
#[test]
fn counted_steps_decide_the_interval_over_the_repaired_fill() {
    let two = repaired_fill("eventually[0,2] holds(k.v = 3)");
    let subject = two.subject("Fill");
    let system = sc_system(&subject);
    let mut behaviour = Behaviour::follow(&system, is_three, preferring("/B"));
    let anchors: Vec<PositionAnchor<'_>> = behaviour.steps.iter().map(|s| s.anchor()).collect();
    assert_eq!(anchors.len(), 5, "{anchors:?}");
    for position in [0, 3, 4] {
        assert!(
            matches!(anchors[position], PositionAnchor::Protocol { .. }),
            "step {position}: {:?}",
            anchors[position]
        );
    }
    assert_eq!(
        [
            behaviour.steps[0].kind(),
            behaviour.steps[3].kind(),
            behaviour.steps[4].kind()
        ],
        [StepKind::Fork, StepKind::Join, StepKind::Finish]
    );
    for (position, operation) in [(1, "setTwo"), (2, "inc")] {
        assert!(
            matches!(anchors[position], PositionAnchor::Operation(name) if name.contains(operation)),
            "step {position}: {:?}",
            anchors[position]
        );
    }
    assert_eq!(behaviour.counted(), [false, true, true, false, false]);
    assert_eq!(behaviour.at(two.clause(), 0), TemporalOutcome::Holds);
    let one = repaired_fill("eventually[0,1] holds(k.v = 3)");
    assert_eq!(behaviour.at(one.clause(), 0), violated(0));
    behaviour.by_kind = false;
    assert_eq!(behaviour.at(two.clause(), 0), violated(0));
}

/// FR-213-AC-2 (TC-658): over §7.1's `Pay`, on the failure path
/// `eventually[0,2] holds(k.bal = 0)` holds at position 2 and fails at
/// position 1, whose distance to the refund is 3 with `cend` and `finish`
/// uncounted; on the success path it fails at every position from 1.
#[trace("TC-658", "FR-213-AC-2")]
#[test]
fn counted_steps_decide_the_interval_over_the_compensation_paths() {
    let case = Case::new(&(pay("true") + &claim("eventually[0,2] holds(k.v = 0)")));
    let subject = case.subject("Pay");
    let system = sc_system(&subject);
    let protocol = &system;
    let failing = |value: bool| {
        move |steps: &[(ProtocolTransition, usize)]| {
            let event = |step: &ProtocolTransition| {
                let key = protocol.transition_key(step).expect("an identity");
                let identity: serde_json::Value = serde_json::from_slice(&key).expect("JSON");
                step.kind() == StepKind::Event && identity["record"]["value"] == value
            };
            steps
                .iter()
                .position(|(step, _)| {
                    event(step) && step.node().is_some_and(|node| node.ends_with("Fail"))
                })
                .unwrap_or(0)
        }
    };
    let failure = Behaviour::follow(&system, is_zero, failing(true));
    assert_eq!(
        failure.steps.iter().map(|s| s.kind()).collect::<Vec<_>>(),
        [
            StepKind::Attempt,
            StepKind::Event,
            StepKind::Event,
            StepKind::CAttempt,
            StepKind::CEnd,
            StepKind::Finish
        ]
    );
    assert_eq!(
        failure.counted(),
        [true, true, true, true, false, false],
        "the refund is the fourth counted step"
    );
    assert_eq!(failure.at(case.clause(), 2), TemporalOutcome::Holds);
    assert_eq!(failure.at(case.clause(), 1), violated(1));
    let success = Behaviour::follow(&system, is_zero, failing(false));
    for position in 1..u64::try_from(success.states.len()).expect("a small trace") {
        assert_eq!(success.at(case.clause(), position), violated(position));
    }
}

/// FR-213-AC-3 (TC-658): EN-1 reproduces AC-1. Over the repaired `Fill`,
/// `eventually[0,2] holds(k.v >= 2)` under event-position false-extension
/// holds on every behaviour; `eventually[0,1] holds(k.v = 3)` settles
/// `refuted` with a prefix on which no position at counted distance 0 or 1
/// from position 0 has `k.v = 3`, and that prefix replays. Not met: the
/// first claim does not settle `proved` yet. FR-127 gives a bounded clause
/// that label only with a closure certificate, FR-338's translation covers
/// TP-1 items alone, and `settle_protocol_check` returns `Failed` for it.
/// The test asserts that label, so it flips when the certificate lands.
/// Remaining work: QSL-878.
#[trace("TC-658", "FR-213-AC-3")]
#[test]
fn the_model_checker_counts_steps_over_the_repaired_fill() {
    let proved = repaired_fill("eventually[0,2] holds(k.v >= 2)");
    let letters = CellV(at_least_two);
    let outcome = proved.check("Fill", &letters);
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );
    assert_eq!(
        proved.label("Fill", &letters, &outcome),
        ResultLabel::Failed
    );

    let refuted = repaired_fill("eventually[0,1] holds(k.v = 3)");
    let letters = CellV(|value| value == 3);
    let outcome = refuted.check("Fill", &letters);
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    let subject = refuted.subject("Fill");
    let system = sc_system(&subject);
    let mut state = system.initial().remove(0);
    let (mut distance, mut values) = (0, vec![(0, v(&system.atom_state(&state), "c"))]);
    for step in &counterexample.prefix {
        let next = system
            .successors(&state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| *transition == step.transition)
            .expect("the step is enabled")
            .1;
        distance += i64::from(step.transition.counts());
        values.push((distance, v(&system.atom_state(&next), "c")));
        state = next;
    }
    assert!(
        values
            .iter()
            .all(|(distance, value)| *distance > 1 || *value != 3),
        "{values:?}"
    );
    assert_eq!(values.last().map(|(distance, _)| *distance), Some(2));
    assert_eq!(
        refuted.label("Fill", &letters, &outcome),
        ResultLabel::Refuted
    );
}

/// FR-213-AC-3 (TC-658), the model-subject half: over the `Fill` model
/// subject every step counts (ADR-018's reading), so
/// `eventually[0,1] holds(k.v = 3)` is refuted by the one step that leaves
/// the last position of its window, and `eventually[0,2] holds(k.v >= 2)` by
/// two steps (`touch` leaves `k.v` at 0), not by the protocol's counted steps.
#[trace("TC-658", "FR-213-AC-3")]
#[test]
fn a_model_subject_counts_every_step() {
    for (formula, holds, steps) in [
        (
            "eventually[0,1] holds(k.v = 3)",
            is_three as fn(i64) -> bool,
            1,
        ),
        ("eventually[0,2] holds(k.v >= 2)", at_least_two, 2),
    ] {
        let case = repaired_fill(formula);
        let letters = CellV(holds);
        let outcome = check_model(
            ModelCheckRequest {
                subject: model_subject(
                    &case.checked,
                    &case.documents,
                    case.initial.clone(),
                    &["c"],
                ),
                item: ModelCheckItem::Clause {
                    clause: case.clause(),
                    letters: &letters,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
        .expect("the model subject checks");
        let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
            panic!("{formula}: {outcome:?}");
        };
        assert_eq!(counterexample.prefix.len(), steps, "{formula}");
    }
}

/// FR-213-AC-2 (TC-658), a vector that tells counted steps from positions:
/// on the failure path of `Pay`, `cend` and `finish` are uncounted and the
/// false extension's stutter is the first counted step past them, so
/// `always[0,1] holds(k.bal = 0)` is violated at position 4 (the refund's
/// post-state), though it holds there if every step counts. The spec's own
/// AC-2 vector gives the same verdicts either way (QSL-396).
#[trace("TC-658", "FR-213-AC-2")]
#[test]
fn an_uncounted_cend_and_finish_end_the_window_after_the_refund() {
    let case = Case::new(&(pay("true") + &claim("always[0,1] holds(k.v = 0)")));
    let subject = case.subject("Pay");
    let system = sc_system(&subject);
    let protocol = &system;
    let failed = |steps: &[(ProtocolTransition, usize)]| {
        steps
            .iter()
            .position(|(step, _)| {
                let key = protocol.transition_key(step).expect("an identity");
                let identity: serde_json::Value = serde_json::from_slice(&key).expect("JSON");
                step.kind() == StepKind::Event
                    && step.node().is_some_and(|node| node.ends_with("Fail"))
                    && identity["record"]["value"] == true
            })
            .unwrap_or(0)
    };
    let mut behaviour = Behaviour::follow(&system, is_zero, failed);
    assert_eq!(behaviour.at(case.clause(), 4), violated(4));
    behaviour.by_kind = false;
    assert_eq!(behaviour.at(case.clause(), 4), TemporalOutcome::Holds);
}

/// ADR-027 PB-3: a cycle of uncounted steps keeps an interval that needs a
/// counted step open, so it is not met on that lasso. On a channel whose
/// delivery is `unknown`, `duplicate` then `lose` returns to the state it
/// left, the product state coalesces and the monitor never rejects; EN-1
/// must not answer `Holds` for `eventually[0,2] holds(k.v = 1)`, which every
/// behaviour that takes its counted steps satisfies at the attempt. It
/// settles `Undecided` with the uncounted-loop cause.
#[test]
fn an_uncounted_cycle_that_keeps_an_interval_open_is_not_holds() {
    let members = "channel ch from w to w carries Boolean ordering fifo by (m: Boolean) \
                   { true } delivery unknown capacity 2 overflow block;";
    let run = format!(
        "sequence Main {{ send S1 via ch as (s1: Boolean) {{ s1 }}; \
         {} }}",
        attempt("A", "inc", "IncPre, IncPost", "a", "true")
    );
    let unit = contracts("self.v <= 1") + &protocol("P", members, &run, "d");
    let case = Case::new(&(unit + &claim("eventually[0,2] holds(k.v = 1)")));
    let letters = CellV(|value| value == 1);
    let outcome = case.check("P", &letters);
    assert!(
        matches!(
            outcome,
            ModelCheckOutcome::Undecided {
                cause: UndecidedCause::UncountedLoop,
                ..
            }
        ),
        "{outcome:?}"
    );
}

/// ADR-027 PB-3: an operation, an event (an `activate` included), a `send`
/// and a `receive` count; every other step kind, `spawn`, `retire`, `fence`
/// and a memory step included, moves the position and not the distance.
#[trace("TC-658")]
#[test]
fn only_operations_events_sends_and_receives_count() {
    use StepKind::*;
    for kind in [Attempt, CAttempt, Event, Activate, Send, Receive] {
        assert!(kind.counts(), "{kind:?}");
    }
    for kind in [
        Duplicate, Lose, Fork, Join, Timeout, Finish, Memory, Fence, CEnd, Spawn, Retire,
    ] {
        assert!(!kind.counts(), "{kind:?}");
    }
}
