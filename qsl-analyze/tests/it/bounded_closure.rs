// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338 and FR-127 for a bounded (TP-2) clause over the Counter subject
//! (`crate::counter`) from `c = 0`, where `inc` runs until the deadlock at 3:
//! EN-1's proof of `eventually[0,u] holds(A)` and `always[0,u] holds(A)`
//! carries a closure certificate of the model states under the canonical
//! bounded state each was read to, the core checker accepts or rejects it,
//! and the settlement map settles `proved` on its acceptance.
//!
//! Every expected certificate is written from the Counter arithmetic and the
//! counted-distance rule, not from the engine or the checker. Not covered:
//! an `on each` bounded clause, a lower bound, an interval in an operand and
//! past operators, which have no canonical bounded state yet (Remaining
//! work: QSL-878).

use ix_trace_rs::trace;
use qsl_eval::automaton::{BoundedState, BoundedTranslation, StateContext};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_replay::{
    check_closure, settle_model_check, CertificateLocus, CertificateRejection, CertificateRequest,
    CertificateRule, ClauseIdentity, ClosureCertificate, DigestRecord, IncompleteCause,
    ModelCheckItem, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome, ProductStateRef,
    ResultLabel, TemporalCertificateCheck,
};
use quire_exact::{Cancel, Value};

use crate::counter::{reference, subject, value, Letters, UNLIMITED};
use crate::lasso_replay::{bounded_fixture, package_id};
use crate::liveness::Fixture;

/// A bounded clause over the one counter `c`, with its letters.
struct Bounded {
    fixture: Fixture,
    atom: fn(i64) -> Option<bool>,
    letters: Letters,
}

fn bounded(formula: &str, atom: fn(i64) -> Option<bool>) -> Bounded {
    Bounded {
        fixture: bounded_fixture(formula),
        atom,
        letters: Letters { atom },
    }
}

impl Bounded {
    fn item(&self) -> ModelCheckItem<'_> {
        ModelCheckItem::Clause {
            clause: &self.fixture.clauses[0],
            letters: &self.letters,
        }
    }

    fn request(&self) -> CertificateRequest<'_> {
        let fixture = &self.fixture;
        CertificateRequest {
            subject: subject(
                &fixture.package,
                &fixture.documents,
                fixture.initial.clone(),
                &fixture.keys,
                UNLIMITED,
            ),
            item: self.item(),
        }
    }

    fn run(&self) -> ModelCheckOutcome {
        self.run_under(ModelCheckLimits::default(), None)
    }

    fn run_under(&self, limits: ModelCheckLimits, max_depth: Option<usize>) -> ModelCheckOutcome {
        self.fixture
            .run(self.atom, max_depth, limits, UNLIMITED, &Cancel::new())
            .expect("checked")
    }

    fn translation(&self) -> BoundedTranslation {
        let fixture = &self.fixture;
        let identity = ClauseIdentity::resolve(
            &fixture.package,
            &fixture.documents.packages,
            &fixture.clauses[0],
            &Cancel::new(),
        )
        .expect("the package declares the clause");
        let binding = vec![qsl_eval::automaton::ContextBinding {
            parameter: identity.over().expect("an over node"),
            value: Value::Reference(reference(&fixture.package, &fixture.documents, "c")),
        }];
        let context = StateContext::clause(&fixture.clauses[0], identity.occurrence().clone(), binding);
        BoundedTranslation::clause(&fixture.clauses[0], &context).expect("translates")
    }

    /// The digest of the model state `c = v`.
    fn digest(&self, v: i64) -> DigestRecord {
        let fixture = &self.fixture;
        let system = ModelSystem::new(
            subject(
                &fixture.package,
                &fixture.documents,
                fixture.initial.clone(),
                &fixture.keys,
                UNLIMITED,
            )
            .simulation,
        )
        .expect("the subject admits");
        let mut state: ModelSimState = system.initial().remove(0);
        while value(&state, "c") != v {
            state = system
                .successors(&state)
                .expect("expands")
                .successors
                .remove(0)
                .1;
        }
        system.state_key(&state).expect("encodes").1
    }

    /// The product state of `c = v` read to `state`.
    fn at(&self, v: i64, state: BoundedState) -> ProductStateRef {
        ProductStateRef {
            model_state: self.digest(v),
            automaton_state: self.translation().key(state).expect("encodes"),
        }
    }

    fn certificate_of(&self, states: &[(i64, BoundedState)]) -> ClosureCertificate {
        let mut states: Vec<ProductStateRef> =
            states.iter().map(|(v, state)| self.at(*v, *state)).collect();
        states.sort();
        ClosureCertificate { states }
    }
}

fn open(remaining: u64) -> BoundedState {
    BoundedState::Open { remaining }
}

fn is_at_least_one(v: i64) -> Option<bool> {
    Some(v >= 1)
}

fn certificate(outcome: &ModelCheckOutcome) -> ClosureCertificate {
    match outcome {
        ModelCheckOutcome::Holds { certificate, .. } => certificate.clone(),
        other => panic!("expected Holds, got {other:?}"),
    }
}

fn check(request: &CertificateRequest<'_>, certificate: &ClosureCertificate) -> TemporalCertificateCheck {
    check_closure(
        request,
        certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the request is checked")
}

fn rejected(rule: CertificateRule, at: ProductStateRef) -> TemporalCertificateCheck {
    TemporalCertificateCheck::Rejected(CertificateRejection {
        rule,
        at: CertificateLocus::ProductState(at),
    })
}

/// FR-338-AC-6: `eventually[0,1] holds(c.value >= 1)` from `c = 0` reads p
/// false then true, so the monitor advances `Open(1)` to `Accept` and stays
/// there past the horizon. The proof's certificate is exactly those four
/// product states, the core checker accepts it, and the map settles
/// `proved`.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn a_bounded_eventually_that_is_met_carries_a_certificate_the_core_accepts() {
    let clause = bounded("eventually[0,1] holds(c.value >= 1)", is_at_least_one);
    let outcome = clause.run();
    let expected = clause.certificate_of(&[
        (0, open(1)),
        (1, BoundedState::Accept),
        (2, BoundedState::Accept),
        (3, BoundedState::Accept),
    ]);
    assert_eq!(certificate(&outcome), expected);
    let request = clause.request();
    assert_eq!(check(&request, &expected), TemporalCertificateCheck::Accepted);
    let record = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &package_id(&clause.fixture),
        &request,
        &Cancel::new(),
        |_| panic!("a proof replays nothing"),
    );
    assert_eq!(record.label, ResultLabel::Proved);
}

/// FR-338-AC-6: p false then false rejects. `eventually[0,1] holds(c.value
/// >= 2)` reads false at 0 and 1, so EN-1 refutes it and carries no
/// certificate.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn a_bounded_eventually_that_is_missed_is_refuted() {
    let clause = bounded("eventually[0,1] holds(c.value >= 2)", |v| Some(v >= 2));
    assert!(matches!(clause.run(), ModelCheckOutcome::Violated { .. }));
}

/// FR-338-AC-6: a certificate missing a reached product state is rejected at
/// it, `SuccessorMissing` for a successor and `InitialMissing` for the
/// initial state; a certificate that holds a state under another bounded
/// state than it was read to is missing the one it was read to.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn a_bounded_certificate_missing_a_reached_state_is_rejected_at_it() {
    let clause = bounded("eventually[0,1] holds(c.value >= 1)", is_at_least_one);
    let full = certificate(&clause.run());
    let request = clause.request();
    let without = |v: i64, state: BoundedState| {
        let at = clause.at(v, state);
        let mut certificate = full.clone();
        certificate.states.retain(|held| *held != at);
        assert_eq!(certificate.states.len(), full.states.len() - 1);
        (certificate, at)
    };
    let (successor, at) = without(2, BoundedState::Accept);
    assert_eq!(
        check(&request, &successor),
        rejected(CertificateRule::SuccessorMissing, at)
    );
    let (initial, at) = without(0, open(1));
    assert_eq!(
        check(&request, &initial),
        rejected(CertificateRule::InitialMissing, at)
    );
    let mut renamed = full.clone();
    renamed.states.retain(|held| *held != clause.at(1, BoundedState::Accept));
    renamed.states.push(clause.at(1, open(0)));
    assert_eq!(
        check(&request, &renamed),
        rejected(CertificateRule::SuccessorMissing, clause.at(1, BoundedState::Accept))
    );
}

/// FR-338-AC-6: the final counted-distance region and the terminal false
/// extension give the evaluator's result. `always[0,3] holds(c.value >= 0)`
/// ends its window at the terminal state `c = 3` (`Open(0)`), which the
/// false extension discharges, so it holds and its certificate is accepted;
/// `always[0,4]` leaves a counted step in the window at `c = 3` (`Open(1)`),
/// where the false extension's false letter rejects, so a certificate that
/// holds the states is rejected `BadState` at the terminal state.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn the_terminal_false_extension_closes_the_last_region() {
    let closes = bounded("always[0,3] holds(c.value >= 0)", |v| Some(v >= 0));
    let outcome = closes.run();
    let expected = closes.certificate_of(&[
        (0, open(3)),
        (1, open(2)),
        (2, open(1)),
        (3, open(0)),
    ]);
    assert_eq!(certificate(&outcome), expected);
    assert_eq!(
        check(&closes.request(), &expected),
        TemporalCertificateCheck::Accepted
    );

    let open_at_end = bounded("always[0,4] holds(c.value >= 0)", |v| Some(v >= 0));
    assert!(matches!(
        open_at_end.run(),
        ModelCheckOutcome::Violated { .. }
    ));
    let held = open_at_end.certificate_of(&[
        (0, open(4)),
        (1, open(3)),
        (2, open(2)),
        (3, open(1)),
    ]);
    assert_eq!(
        check(&open_at_end.request(), &held),
        rejected(CertificateRule::BadState, open_at_end.at(3, open(1)))
    );
}

/// FR-338-AC-6 and ADR-018 UE-1: the claim reads positions 0 to its horizon,
/// the positions after the monitor accepts included, and no later one. With
/// the atom undefined at `c = 2`, `eventually[0,1]` never reads position 2
/// and holds, while `eventually[0,2]` reads it after accepting at 1: EN-1
/// finds the undefined evaluation and the checker rejects a certificate of
/// the states `BadState` at the state read to undefined.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn the_positions_of_the_horizon_are_read_after_the_monitor_accepts() {
    fn undefined_at_two(v: i64) -> Option<bool> {
        (v != 2).then_some(v >= 1)
    }
    let unread = bounded("eventually[0,1] holds(c.value >= 1)", undefined_at_two);
    let proof = certificate(&unread.run());
    assert_eq!(
        check(&unread.request(), &proof),
        TemporalCertificateCheck::Accepted
    );

    let read = bounded("eventually[0,2] holds(c.value >= 1)", undefined_at_two);
    assert!(matches!(
        read.run(),
        ModelCheckOutcome::Violated { .. }
    ));
    let held = read.certificate_of(&[
        (0, open(2)),
        (1, BoundedState::Accept),
        (2, BoundedState::Accept),
        (3, BoundedState::Accept),
    ]);
    assert_eq!(
        check(&read.request(), &held),
        rejected(CertificateRule::BadState, read.at(2, BoundedState::Reject))
    );
}

/// A budget stops the checker with the typed cause and never accepts: a
/// certificate padded past `max_states` entries stops at `MaxStates`, and
/// `max_transitions` 1 stops the first expansion at `MaxTransitions` naming
/// the limit and value.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn a_stopped_bounded_check_names_what_stopped_it() {
    let clause = bounded("eventually[0,1] holds(c.value >= 1)", is_at_least_one);
    let proof = certificate(&clause.run());
    let request = clause.request();
    let stop = |limit, value| {
        TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached { limit, value })
    };
    let mut states = ModelCheckLimits::default();
    states.limits.max_states = 2;
    let mut transitions = ModelCheckLimits::default();
    transitions.limits.max_transitions = 1;
    for (limits, expected) in [
        (states, stop(ModelCheckLimit::MaxStates, 2)),
        (transitions, stop(ModelCheckLimit::MaxTransitions, 1)),
    ] {
        assert_eq!(
            check_closure(&request, &proof, &limits, &Cancel::new()).expect("checked"),
            expected
        );
    }
}
