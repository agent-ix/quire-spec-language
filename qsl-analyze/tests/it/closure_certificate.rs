// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338 and FR-127 over the Counter subject (`crate::counter`): EN-1's
//! safety proof carries a closure certificate, `qsl-replay`'s core checker
//! accepts or rejects it, and the settlement map settles `Certified` only on
//! the checker's acceptance.
//!
//! Every certificate offered here is built from the engine's own output or
//! from the subject's states read through `ModelSystem`; the expected
//! rejection loci are computed from the Counter arithmetic, not from the
//! checker.
//!
//! Not covered: ADR-018 §6's `ConfigVersion` subject and FR-120-AC-9's
//! `test/tallies` subject, which have no model-check fixture; the Counter
//! subject stands in for both, so the FR-338-AC-1 to AC-3 vectors are not
//! verified as written.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_model, ModelCheckRequest};
use qsl_eval::automaton::{SafetyState, SafetyTranslation, StateContext};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_replay::{
    check_closure, compile_package, settle_model_check, CertificateLocus, CertificateRejection,
    CertificateRequest, CertificateRule, Certification, ClauseIdentity, ClosureCertificate,
    ClosureRefusal, DependencyInput, DigestDomain, DigestRecord, Execution, IncompleteCause,
    InconclusiveCause, ModelCheckItem, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome,
    ProductStateRef, ProofBasis, ReachedLimit, ReplayLimits, ResultLabel, SettlementBasis,
    StageLimits, TemporalCertificateCheck, TerminalValue, UndecidedCause,
};
use qsl_semantics::check::CheckedTemporalClause;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Cancel, CancelCause, ObjectReference, ScalarLimits, Value};

use crate::counter::{
    clause, document, identity, package, reference, subject, unit_text, value, Documents, Letters,
    CONTRACT, UNLIMITED, ZERO,
};

const GUARDED: &str = "pre CanInc using v on M::Counter::inc { 6 mod (2 - self.value) >= 0 }\n\
                       post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n";

/// A Counter subject over the counters `keys`, from one initial snapshot.
struct World {
    body: String,
    package: CheckedPackage,
    documents: Documents,
    initial: Vec<DocumentRef>,
    keys: Vec<&'static str>,
}

fn world(body: &str, start: &[(&'static str, i64)]) -> World {
    let document = document(false);
    let package = package(&document, body);
    let mut documents = Documents::new(document);
    let initial = vec![documents.start("s0", start)];
    World {
        body: body.to_owned(),
        package,
        documents,
        initial,
        keys: start.iter().map(|(key, _)| *key).collect(),
    }
}

/// A TP-1 clause with its letters and identity.
struct Invariant {
    clause: CheckedTemporalClause,
    letters: Letters,
    identity: ClauseIdentity,
}

impl World {
    fn invariant(&self, formula: &str, atom: fn(i64) -> Option<bool>) -> Invariant {
        Invariant {
            clause: clause(&self.documents.document, "", formula),
            letters: Letters {
                bindings: self.objects(),
                atom,
            },
            identity: identity(),
        }
    }

    fn objects(&self) -> Vec<ObjectReference> {
        self.keys
            .iter()
            .map(|key| reference(&self.package, &self.documents, key))
            .collect()
    }

    fn request<'w>(
        &'w self,
        item: ModelCheckItem<'w>,
        meter: ScalarLimits,
    ) -> CertificateRequest<'w> {
        CertificateRequest {
            subject: subject(
                &self.package,
                &self.documents,
                self.initial.clone(),
                &self.keys,
                meter,
            ),
            item,
        }
    }

    /// The `package_id` FR-098's recompile of the unit gives.
    fn package_id(&self) -> DigestRecord {
        let unit = unit_text(&self.documents.document, &self.body);
        compile_package(
            SourceIdentity::new("agent-ix", "counter", "git", "1"),
            "counter.native",
            unit.as_bytes(),
            [self.documents.document.as_slice()],
            &DependencyInput::default(),
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("the unit compiles")
        .package_id()
    }

    fn system(&self) -> ModelSystem<'_> {
        ModelSystem::new(
            self.request(ModelCheckItem::DeadlockFreedom, UNLIMITED)
                .subject
                .simulation,
        )
        .expect("the subject admits")
    }

    /// Every reachable model state, by breadth-first enumeration through
    /// `ModelSystem`, with each counter's value.
    fn reachable(&self) -> Vec<(DigestRecord, Vec<i64>)> {
        let system = self.system();
        let mut seen: Vec<(DigestRecord, Vec<i64>)> = Vec::new();
        let mut queue: Vec<ModelSimState> = system.initial();
        while let Some(state) = queue.pop() {
            let digest = system.state_key(&state).expect("encodes").1;
            if seen.iter().any(|(known, _)| *known == digest) {
                continue;
            }
            seen.push((
                digest,
                self.keys.iter().map(|key| value(&state, key)).collect(),
            ));
            let expansion = system.successors(&state).expect("expands");
            queue.extend(expansion.successors.into_iter().map(|(_, next)| next));
        }
        seen
    }

    /// The model state with these counter values.
    fn digest(&self, values: &[i64]) -> DigestRecord {
        self.reachable()
            .into_iter()
            .find(|(_, at)| at == values)
            .map(|(digest, _)| digest)
            .unwrap_or_else(|| panic!("{values:?} is reachable"))
    }

    /// The deadlock-freedom item's or the clause's run.
    fn run(&self, item: ModelCheckItem<'_>) -> ModelCheckOutcome {
        check_model(
            ModelCheckRequest {
                subject: subject(
                    &self.package,
                    &self.documents,
                    self.initial.clone(),
                    &self.keys,
                    UNLIMITED,
                ),
                item,
                max_depth: None,
                limits: ModelCheckLimits::default(),
            },
            &Cancel::new(),
        )
        .expect("checked")
    }
}

impl Invariant {
    fn item(&self) -> ModelCheckItem<'_> {
        ModelCheckItem::Clause {
            clause: &self.clause,
            letters: &self.letters,
            identity: &self.identity,
        }
    }

    /// The translation of the instance bound to `object`.
    fn translation(&self, object: &ObjectReference) -> SafetyTranslation {
        let binding = vec![qsl_eval::automaton::ContextBinding {
            parameter: self.identity.over.expect("an over node"),
            value: Value::Reference(object.clone()),
        }];
        let context = StateContext::clause(&self.clause, self.identity.occurrence.clone(), binding);
        SafetyTranslation::reachable_invariant(&self.clause, &context).expect("translates")
    }
}

fn certificate(outcome: &ModelCheckOutcome) -> ClosureCertificate {
    match outcome {
        ModelCheckOutcome::Holds { certificate, .. } => certificate.clone(),
        other => panic!("expected Holds, got {other:?}"),
    }
}

fn check(
    request: &CertificateRequest<'_>,
    certificate: &ClosureCertificate,
) -> TemporalCertificateCheck {
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

/// Every state of `states` under `translation`'s `state`.
fn closure(
    states: &[(DigestRecord, Vec<i64>)],
    translation: &SafetyTranslation,
) -> ClosureCertificate {
    let key = translation.key(SafetyState::Carried).expect("encodes");
    ClosureCertificate {
        states: states
            .iter()
            .map(|(digest, _)| ProductStateRef {
                model_state: *digest,
                automaton_state: key.clone(),
            })
            .collect(),
    }
}

/// FR-338's accepted vector on the two-counter Counter subject from (0, 0):
/// `always holds(c.value <= 3)` over `{a, b}` holds and its certificate
/// holds every one of the 16 reachable model states under each of the two
/// instances' carried root, and the core checker accepts it.
#[trace("TC-525")]
#[test]
fn a_proved_invariant_carries_a_certificate_the_core_accepts() {
    let world = world(CONTRACT, &[("a", 0), ("b", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let certificate = certificate(&world.run(invariant.item()));
    let reachable = world.reachable();
    assert_eq!(reachable.len(), 16);
    let mut expected = Vec::new();
    for object in world.objects() {
        expected.extend(closure(&reachable, &invariant.translation(&object)).states);
    }
    expected.sort();
    assert_eq!(certificate.states, expected);
    assert_eq!(
        check(&world.request(invariant.item(), UNLIMITED), &certificate),
        TemporalCertificateCheck::Accepted
    );
}

/// FR-338's rejected vectors on the two-counter subject: the certificate
/// with model state (1, 0) removed for instance `a` is rejected
/// `SuccessorMissing` at that product state, and with the initial (0, 0)
/// removed, `InitialMissing` at it.
#[trace("TC-525")]
#[test]
fn a_certificate_missing_a_reached_state_is_rejected_at_it() {
    let world = world(CONTRACT, &[("a", 0), ("b", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let full = certificate(&world.run(invariant.item()));
    let a = invariant.translation(&world.objects()[0]);
    let without = |values: &[i64]| {
        let at = ProductStateRef {
            model_state: world.digest(values),
            automaton_state: a.key(SafetyState::Carried).expect("encodes"),
        };
        let mut certificate = full.clone();
        certificate.states.retain(|state| *state != at);
        assert_eq!(certificate.states.len(), full.states.len() - 1);
        (certificate, at)
    };
    let request = world.request(invariant.item(), UNLIMITED);
    let (successor, at) = without(&[1, 0]);
    assert_eq!(
        check(&request, &successor),
        rejected(CertificateRule::SuccessorMissing, at)
    );
    let (initial, at) = without(&[0, 0]);
    assert_eq!(
        check(&request, &initial),
        rejected(CertificateRule::InitialMissing, at)
    );
}

/// FR-338's deadlock vector over the Counter subject with no `terminal`
/// declaration: a certificate holding every reachable state (values 0 to 3)
/// under the deadlock-freedom item's carried root is rejected `BadState` at
/// value 3, the deadlock, under the rejecting state; the same states under
/// `always holds(c.value <= 3)`'s carried root are accepted, since the
/// deadlock changes no authored claim (ADR-018 DL-6).
#[trace("TC-525")]
#[test]
fn a_deadlock_is_bad_for_the_deadlock_item_alone() {
    let world = world(CONTRACT, &[("c", 0)]);
    let reachable = world.reachable();
    assert_eq!(reachable.len(), 4);
    let deadlock = SafetyTranslation::deadlock_freedom().expect("translates");
    let request = world.request(ModelCheckItem::DeadlockFreedom, UNLIMITED);
    assert_eq!(
        check(&request, &closure(&reachable, &deadlock)),
        rejected(
            CertificateRule::BadState,
            ProductStateRef {
                model_state: world.digest(&[3]),
                automaton_state: deadlock.key(SafetyState::Rejected).expect("encodes"),
            }
        )
    );

    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let translation = invariant.translation(&world.objects()[0]);
    assert_eq!(
        check(
            &world.request(invariant.item(), UNLIMITED),
            &closure(&reachable, &translation)
        ),
        TemporalCertificateCheck::Accepted
    );
}

/// FR-338's `ContractUndetermined` vector, over the Counter subject with
/// `pre CanInc { 6 mod (2 - self.value) >= 0 }` standing in for
/// `test/tallies`: a certificate for `always holds(true)` holding values 0
/// to 2 is rejected `BadState` at value 2, whose expansion evaluates the
/// precondition undefined. An atom that is undefined or false at a reached
/// state is `BadState` there under the rejecting state.
#[trace("TC-525")]
#[test]
fn an_undetermined_contract_or_a_failing_atom_is_a_bad_state() {
    let world = world(GUARDED, &[("c", 0)]);
    let reachable = world.reachable();
    assert_eq!(reachable.iter().map(|(_, values)| values[0]).max(), Some(2));
    let truth = world.invariant("always holds(true)", |_| Some(true));
    let translation = truth.translation(&world.objects()[0]);
    assert_eq!(
        check(
            &world.request(truth.item(), UNLIMITED),
            &closure(&reachable, &translation)
        ),
        rejected(
            CertificateRule::BadState,
            ProductStateRef {
                model_state: world.digest(&[2]),
                automaton_state: translation.key(SafetyState::Carried).expect("encodes"),
            }
        )
    );

    let world = self::world(CONTRACT, &[("c", 0)]);
    let reachable = world.reachable();
    for (formula, atom) in [
        (
            "always holds(c.value != 2)",
            (|v| Some(v != 2)) as fn(i64) -> Option<bool>,
        ),
        ("always holds(6 / (2 - c.value) >= 0)", |v| {
            (v != 2).then(|| 6 / (2 - v) >= 0)
        }),
    ] {
        let invariant = world.invariant(formula, atom);
        let translation = invariant.translation(&world.objects()[0]);
        assert_eq!(
            check(
                &world.request(invariant.item(), UNLIMITED),
                &closure(&reachable, &translation)
            ),
            rejected(
                CertificateRule::BadState,
                ProductStateRef {
                    model_state: world.digest(&[2]),
                    automaton_state: translation.key(SafetyState::Rejected).expect("encodes"),
                }
            ),
            "{formula}"
        );
    }
}

/// The certificate is checked against the request's own item: the
/// deadlock-freedom item's certificate offered for a clause, or a clause's
/// certificate offered for the clause at another occurrence, is rejected
/// `ShapeMismatch` at its first state, before any key is used for
/// membership; a model state digest of another domain is rejected the same
/// way.
#[trace("TC-525")]
#[test]
fn a_certificate_of_another_item_is_not_admitted() {
    let world = world(CONTRACT, &[("c", 0)]);
    let reachable = world.reachable();
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let deadlock = closure(
        &reachable,
        &SafetyTranslation::deadlock_freedom().expect("translates"),
    );
    assert_eq!(
        check(&world.request(invariant.item(), UNLIMITED), &deadlock),
        rejected(CertificateRule::ShapeMismatch, deadlock.states[0].clone())
    );

    let own = closure(&reachable, &invariant.translation(&world.objects()[0]));
    let moved = Invariant {
        identity: ClauseIdentity {
            occurrence: qsl_foundation::source::provenance::OccurrenceKey::new(
                identity().occurrence.node(),
                quire_exact::Origin::new(quire_exact::Role::new("temporal"), 1),
            ),
            ..identity()
        },
        ..world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3))
    };
    assert_eq!(
        check(&world.request(moved.item(), UNLIMITED), &own),
        rejected(CertificateRule::ShapeMismatch, own.states[0].clone())
    );

    let mut foreign = own.clone();
    foreign.states[0].model_state = DigestRecord::mint(
        DigestDomain::Sha256Jcs,
        *own.states[0].model_state.as_bytes(),
    );
    assert_eq!(
        check(&world.request(invariant.item(), UNLIMITED), &foreign),
        rejected(CertificateRule::ShapeMismatch, foreign.states[0].clone())
    );
}

/// FR-127's certificate rows: the settlement map runs the core checker on
/// a `Holds` outcome's certificate itself, over the request's own package. The engine's certificate settles
/// `proved`, `closed-scope`, `Proved{Exhaustive, Certified}`; the same
/// outcome with a state removed settles `inconclusive`, `unsettled`,
/// `CertificateRejected{SuccessorMissing, at}`, never `proved`; settled
/// against another `package_id` than the request's package it is
/// `ReplayRefused(stale_dependency)`, never `proved`.
#[trace("TC-525")]
#[test]
fn a_proof_settles_certified_only_when_its_certificate_is_accepted() {
    let world = world(CONTRACT, &[("c", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let outcome = world.run(invariant.item());
    let request = world.request(invariant.item(), UNLIMITED);
    let package = world.package_id();
    let settle_at = |outcome: &ModelCheckOutcome, package: &DigestRecord, cancel: &Cancel| {
        settle_model_check(
            outcome,
            &ModelCheckLimits::default(),
            package,
            &request,
            cancel,
            |_| panic!("a proof replays nothing"),
        )
    };
    let settle =
        |outcome: &ModelCheckOutcome, cancel: &Cancel| settle_at(outcome, &package, cancel);
    let proved = settle(&outcome, &Cancel::new());
    assert_eq!(
        proved.value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );
    assert_eq!(
        (proved.label, proved.basis, proved.execution),
        (
            ResultLabel::Proved,
            SettlementBasis::ClosedScope,
            Execution::Completed
        )
    );

    let elsewhere = DigestRecord::mint(DigestDomain::PackageSemanticV2, [9; 32]);
    assert_eq!(
        settle_at(&outcome, &elsewhere, &Cancel::new()).value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency))
    );

    let ModelCheckOutcome::Holds { certificate, stats } = outcome else {
        panic!("a proof");
    };
    let mut tampered = certificate.clone();
    let one = world.digest(&[1]);
    let index = tampered
        .states
        .iter()
        .position(|state| state.model_state == one)
        .expect("value 1 is certified");
    let at = tampered.states.remove(index);
    let record = settle(
        &ModelCheckOutcome::Holds {
            certificate: tampered,
            stats,
        },
        &Cancel::new(),
    );
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::SuccessorMissing,
            at: CertificateLocus::ProductState(at),
        })
    );
    assert_eq!(
        (record.label, record.basis),
        (ResultLabel::Inconclusive, SettlementBasis::Unsettled)
    );

    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Deadline);
    let stopped = settle(&ModelCheckOutcome::Holds { certificate, stats }, &cancel);
    assert_eq!(
        stopped.value,
        TerminalValue::Incomplete(IncompleteCause::Cancelled {
            source: CancelCause::Deadline
        })
    );
    assert_eq!(stopped.execution, Execution::ResourceIncomplete);
}

/// A budget or a cancellation stops the checker with the typed cause and
/// never accepts: `max_states` 2, `max_transitions` 1 and
/// `max_automaton_states` 0 name their limit and value, a zero evaluation
/// meter names the meter, and a cancelled handle names its source. The
/// map settles a stopped check `Incomplete` with the same cause and limit.
#[trace("TC-525")]
#[test]
fn a_stopped_check_names_what_stopped_it() {
    let world = world(CONTRACT, &[("c", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let outcome = world.run(invariant.item());
    let certificate = certificate(&outcome);
    let request = world.request(invariant.item(), UNLIMITED);
    let defaults = ModelCheckLimits::default();
    let stop = |limit, value| {
        TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached { limit, value })
    };
    let mut states = defaults;
    states.limits.max_states = 2;
    let mut transitions = defaults;
    transitions.limits.max_transitions = 1;
    let automaton = ModelCheckLimits {
        max_automaton_states: 0,
        ..defaults
    };
    for (limits, expected) in [
        (states, stop(ModelCheckLimit::MaxStates, 2)),
        (transitions, stop(ModelCheckLimit::MaxTransitions, 1)),
        (automaton, stop(ModelCheckLimit::MaxAutomatonStates, 0)),
    ] {
        assert_eq!(
            check_closure(&request, &certificate, &limits, &Cancel::new()).expect("checked"),
            expected
        );
    }
    assert_eq!(
        check_closure(
            &world.request(invariant.item(), ZERO),
            &certificate,
            &defaults,
            &Cancel::new()
        )
        .expect("checked"),
        stop(ModelCheckLimit::EvaluationMeter, 0)
    );
    for source in [CancelCause::Requested, CancelCause::Deadline] {
        let cancel = Cancel::new();
        cancel.cancel(source);
        assert_eq!(
            check_closure(&request, &certificate, &defaults, &cancel).expect("checked"),
            TemporalCertificateCheck::Stopped(IncompleteCause::Cancelled { source })
        );
    }

    let record = settle_model_check(
        &outcome,
        &states,
        &world.package_id(),
        &request,
        &Cancel::new(),
        |_| panic!("a proof replays nothing"),
    );
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 2
        })
    );
    assert_eq!(
        record.limit_reached,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxStates,
            value: 2
        })
    );
    assert_eq!(record.execution, Execution::ResourceIncomplete);
}

/// FR-125: a clause with no instance is `Undecided(NoInstance)` before any
/// state is explored, and the checker refuses to certify it: an empty
/// certificate is never an acceptance.
#[trace("TC-525")]
#[test]
fn a_clause_with_no_instance_is_never_certified() {
    let world = world(CONTRACT, &[("c", 0)]);
    let mut invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    invariant.letters.bindings.clear();
    assert!(matches!(
        world.run(invariant.item()),
        ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInstance,
            stats,
        } if stats.product_states == 0
    ));
    assert!(matches!(
        check_closure(
            &world.request(invariant.item(), UNLIMITED),
            &ClosureCertificate::default(),
            &ModelCheckLimits::default(),
            &Cancel::new()
        ),
        Err(ClosureRefusal::NoInstance)
    ));
}

/// ADR-016 EX-10: a subject with no initial state examines no behaviour, so
/// the checker refuses it and an empty certificate offered with a `Holds`
/// settles `inconclusive`, `NoInitialState`, never `Certified`.
#[trace("TC-525")]
#[test]
fn a_subject_with_no_initial_state_is_never_certified() {
    let mut world = world(CONTRACT, &[("c", 0)]);
    world.initial.clear();
    let request = world.request(ModelCheckItem::DeadlockFreedom, UNLIMITED);
    let empty = ClosureCertificate::default();
    assert!(matches!(
        check_closure(
            &request,
            &empty,
            &ModelCheckLimits::default(),
            &Cancel::new()
        ),
        Err(ClosureRefusal::NoInitialState)
    ));
    let record = settle_model_check(
        &ModelCheckOutcome::Holds {
            certificate: empty,
            stats: qsl_replay::RunStats::default(),
        },
        &ModelCheckLimits::default(),
        &world.package_id(),
        &request,
        &Cancel::new(),
        |_| panic!("a proof replays nothing"),
    );
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::NoInitialState)
    );
    assert_eq!(record.label, ResultLabel::Inconclusive);
}

/// Key admission is bounded and cancellable before any state is explored:
/// a certificate padded past `max_states` entries per instance stops at
/// that limit before its padding is read, and a cancelled handle stops it
/// before a malformed first entry is examined.
#[trace("TC-525")]
#[test]
fn key_admission_is_bounded_and_cancellable() {
    let world = world(CONTRACT, &[("c", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let mut certificate = certificate(&world.run(invariant.item()));
    let padding = certificate.states[0].clone();
    certificate.states.extend(std::iter::repeat_n(
        ProductStateRef {
            automaton_state: qsl_eval::automaton::AutomatonStateKey::imported(b"[".to_vec()),
            ..padding
        },
        10,
    ));
    let request = world.request(invariant.item(), UNLIMITED);
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 4;
    assert_eq!(
        check_closure(&request, &certificate, &limits, &Cancel::new()).expect("checked"),
        TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 4
        })
    );
    certificate.states.reverse();
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    assert_eq!(
        check_closure(
            &request,
            &certificate,
            &ModelCheckLimits::default(),
            &cancel
        )
        .expect("checked"),
        TemporalCertificateCheck::Stopped(IncompleteCause::Cancelled {
            source: CancelCause::Requested
        })
    );
}

/// FR-338's admission negatives reach the checker as imported octets: a
/// malformed key, a non-canonical decimal path and a reordered set are each
/// refused before membership, rejecting the certificate `ShapeMismatch` at
/// that entry rather than reading it as a missing or member state.
#[trace("TC-525")]
#[test]
fn a_malformed_key_in_a_certificate_is_refused_before_membership() {
    let world = world(CONTRACT, &[("c", 0)]);
    let reachable = world.reachable();
    let deadlock = SafetyTranslation::deadlock_freedom().expect("translates");
    let request = world.request(ModelCheckItem::DeadlockFreedom, UNLIMITED);
    let carried = String::from_utf8(
        deadlock
            .key(SafetyState::Carried)
            .expect("encodes")
            .as_bytes()
            .to_vec(),
    )
    .expect("UTF-8");
    for octets in [
        "[".to_owned(),
        carried.replace(r#"[[],"positive"]"#, r#"[["01"],"positive"]"#),
        carried.replace(",", ", "),
    ] {
        let mut certificate = closure(&reachable, &deadlock);
        let at = ProductStateRef {
            automaton_state: qsl_eval::automaton::AutomatonStateKey::imported(
                octets.clone().into_bytes(),
            ),
            ..certificate.states[0].clone()
        };
        certificate.states.insert(0, at.clone());
        assert_eq!(
            check(&request, &certificate),
            rejected(CertificateRule::ShapeMismatch, at),
            "{octets}"
        );
    }
}
