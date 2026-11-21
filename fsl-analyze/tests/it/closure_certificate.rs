// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338 and FR-127 over the Counter subject (`crate::counter`): EN-1's
//! safety proof carries a closure certificate, `fsl-replay`'s core checker
//! accepts or rejects it, and the settlement map settles `Certified` only on
//! the checker's acceptance.
//!
//! Every certificate offered here is built from the engine's own output or
//! from the subject's states read through `ModelSystem`; the expected
//! rejection loci are computed from the Counter arithmetic, not from the
//! checker.
//!
//! Not covered: ADR-018 §6's `ConfigVersion` subject, which has no
//! model-check fixture; the Counter subject stands in for it, so the
//! FR-338-AC-1 vectors, and the AC-2 vector over `ConfigVersion`, are not
//! verified as written. The deadlock test below tags AC-2 for its deadlock
//! vector alone; the missing-state rejections are checked over the Counter
//! and carry no AC-2 tag.

use filament_exact::{Cancel, CancelCause, ObjectReference, ScalarLimits};
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest};
use fsl_eval::automaton::{SafetyState, SafetyTranslation, StateContext};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::TransitionSystem;
use fsl_package::CheckedPackage;
use fsl_package::EmitLimits;
use fsl_replay::{
    check_closure, settle_model_check, CertificateLocus, CertificateRejection, CertificateRequest,
    CertificateRule, Certification, ClauseIdentity, ClauseRootRefusal, ClosureCertificate,
    ClosureRefusal, Execution, IncompleteCause, InconclusiveCause, ItemRefusal, ModelCheckItem,
    ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome, ProductStateRef, ProofBasis,
    ProofCertificate, ReachedLimit, ResultLabel, SettlementBasis, TemporalCertificateCheck,
    TerminalValue, UndecidedCause,
};
use fsl_semantics::check::CheckedTemporalClause;
use fsl_semantics::model::observation::DocumentRef;
use ix_trace_rs::trace;

use crate::counter::{
    document, package_with_clauses, reference, subject, value, Documents, Letters, CONTRACT,
    UNLIMITED, ZERO,
};
use crate::fill::tcb_01_event_position_selection;

mod certificate_boundary_cases;

const GUARDED: &str = "pre CanInc using v on M::Counter::inc { 6 mod (2 - self.value) >= 0 }\n\
                       post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n";

/// A Counter subject over the counters `keys`, from one initial snapshot.
pub(crate) struct World {
    body: String,
    package: CheckedPackage,
    documents: Documents,
    initial: Vec<DocumentRef>,
    keys: Vec<&'static str>,
    check_mode: CheckMode,
}

pub(crate) fn world(body: &str, start: &[(&'static str, i64)]) -> World {
    world_with_mode(body, start, CheckMode::Linked)
}

fn world_with_mode(body: &str, start: &[(&'static str, i64)], check_mode: CheckMode) -> World {
    let document = document(false);
    let package = crate::counter::package_with_mode(&document, body, check_mode);
    let mut documents = Documents::new(document);
    let initial = vec![documents.start("s0", start)];
    World {
        body: body.to_owned(),
        package,
        documents,
        initial,
        keys: start.iter().map(|(key, _)| *key).collect(),
        check_mode,
    }
}

/// A TP-1 clause with its letters, declared in a package of its own: the
/// clause is a declaration of that package.
pub(crate) struct Invariant {
    package: CheckedPackage,
    clause: CheckedTemporalClause,
    letters: Letters,
    identity: ClauseIdentity,
}

impl World {
    pub(crate) fn invariant(&self, formula: &str, atom: fn(i64) -> Option<bool>) -> Invariant {
        let declared = [("C", "", formula)];
        let (package, mut clauses) = crate::counter::package_with_clauses_mode(
            &self.documents.document,
            &self.body,
            &declared,
            self.check_mode,
        );
        self.declared(package, clauses.remove(0), atom)
    }

    /// `clause`, declared by `package`, with letters over this world's
    /// objects.
    fn declared(
        &self,
        package: CheckedPackage,
        clause: CheckedTemporalClause,
        atom: fn(i64) -> Option<bool>,
    ) -> Invariant {
        let identity = ClauseIdentity::resolve(
            &package,
            &self.documents.packages,
            &clause,
            &[],
            &Cancel::new(),
            EmitLimits::fixture(),
        )
        .expect("the package declares the clause");
        Invariant {
            package,
            clause,
            letters: Letters { atom },
            identity,
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
        self.request_in(&self.package, item, meter)
    }

    /// [`World::request`] over `package`, the world's with a clause declared.
    fn request_in<'w>(
        &'w self,
        package: &'w CheckedPackage,
        item: ModelCheckItem<'w>,
        meter: ScalarLimits,
    ) -> CertificateRequest<'w> {
        CertificateRequest {
            temporal_clock_selections: &[],
            subject: subject(
                package,
                &self.documents,
                self.initial.clone(),
                &self.keys,
                meter,
            ),
            item,
            smt_encoding: fsl_replay::SmtEncodingLimits::default(),
        }
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
    fn reachable(&self) -> Vec<(fsl_eval::simulation::StateKey, Vec<i64>)> {
        let system = self.system();
        let mut seen: Vec<(fsl_eval::simulation::StateKey, Vec<i64>)> = Vec::new();
        let mut queue: Vec<ModelSimState> = system.initial();
        while let Some(state) = queue.pop() {
            let (bytes, _) = system.state_key(&state).expect("encodes");
            let state_key = fsl_eval::simulation::StateKey::from_canonical_bytes(bytes);
            if seen.iter().any(|(known, _)| known == &state_key) {
                continue;
            }
            seen.push((
                state_key,
                self.keys.iter().map(|key| value(&state, key)).collect(),
            ));
            let expansion = system.successors(&state).expect("expands");
            queue.extend(expansion.successors.into_iter().map(|(_, next)| next));
        }
        seen
    }

    /// The model state with these counter values.
    fn state_key(&self, values: &[i64]) -> fsl_eval::simulation::StateKey {
        self.reachable()
            .into_iter()
            .find(|(_, at)| at == values)
            .map(|(state_key, _)| state_key)
            .unwrap_or_else(|| panic!("{values:?} is reachable"))
    }

    /// The run of `item` over `package`.
    fn run_in(&self, package: &CheckedPackage, item: ModelCheckItem<'_>) -> ModelCheckOutcome {
        check_model(
            ModelCheckRequest {
                temporal_clock_selections: &[],
                subject: subject(
                    package,
                    &self.documents,
                    self.initial.clone(),
                    &self.keys,
                    UNLIMITED,
                ),
                item,
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
        .expect("checked")
    }
}

impl Invariant {
    /// The EN-1 item `analyze` runs for this clause under `limits`
    /// (FR-281): the run, its closure checker's request, and `replay` for a
    /// counterexample.
    pub(crate) fn model_item<'w>(
        &'w self,
        world: &'w World,
        limits: ModelCheckLimits,
        replay: impl FnOnce(
                &fsl_replay::TemporalCounterexample,
            )
                -> Result<fsl_replay::ModelTraceReplayResult, fsl_replay::ReplayRefusal>
            + 'w,
    ) -> fsl_analyze::analyze::ModelItem<'w> {
        fsl_analyze::analyze::ModelItem {
            request: ModelCheckRequest {
                temporal_clock_selections: &[],
                subject: subject(
                    &self.package,
                    &world.documents,
                    world.initial.clone(),
                    &world.keys,
                    UNLIMITED,
                ),
                item: self.item(),
                max_depth: None,
                limits,
                reductions: Default::default(),
            },
            stutter_closure: false,
            certificate: self.request(world, UNLIMITED),
            replay: Box::new(replay),
        }
    }

    fn item(&self) -> ModelCheckItem<'_> {
        ModelCheckItem::Clause {
            clause: &self.clause,
            letters: &self.letters,
        }
    }

    fn request<'w>(&'w self, world: &'w World, meter: ScalarLimits) -> CertificateRequest<'w> {
        self.request_with(world, self.item(), meter)
    }

    /// A request for `item` over this clause's package.
    pub(crate) fn request_with<'w>(
        &'w self,
        world: &'w World,
        item: ModelCheckItem<'w>,
        meter: ScalarLimits,
    ) -> CertificateRequest<'w> {
        world.request_in(&self.package, item, meter)
    }

    fn run(&self, world: &World) -> ModelCheckOutcome {
        world.run_in(&self.package, self.item())
    }

    /// The translation of the instance bound to `object`.
    fn translation(&self, object: &ObjectReference) -> SafetyTranslation {
        let binding = vec![fsl_eval::automaton::ContextBinding::instance(
            self.identity.over().expect("an over node"),
            object.object().as_str(),
        )];
        let context =
            StateContext::clause(&self.clause, self.identity.occurrence().clone(), binding);
        SafetyTranslation::reachable_invariant(&self.clause, &context).expect("translates")
    }
}

fn certificate(outcome: &ModelCheckOutcome) -> ClosureCertificate {
    match outcome {
        ModelCheckOutcome::Holds {
            certificate: ProofCertificate::Closure(certificate),
            ..
        } => certificate.clone(),
        other => panic!("expected Holds, got {other:?}"),
    }
}

fn closure_proof(certificate: &ProofCertificate) -> &ClosureCertificate {
    match certificate {
        ProofCertificate::Closure(closure) => closure,
        ProofCertificate::Component(_) => panic!("expected a closure certificate"),
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

/// The least of `states` by model-state key octets then key octets.
fn least(states: &[ProductStateRef]) -> ProductStateRef {
    states
        .iter()
        .min_by(|left, right| {
            (left.model_state.as_bytes(), left.automaton_state.as_bytes()).cmp(&(
                right.model_state.as_bytes(),
                right.automaton_state.as_bytes(),
            ))
        })
        .cloned()
        .expect("a state")
}

/// Every state of `states` under `translation`'s `state`.
fn closure(
    states: &[(fsl_eval::simulation::StateKey, Vec<i64>)],
    translation: &SafetyTranslation,
) -> ClosureCertificate {
    let key = translation.key(SafetyState::Carried).expect("encodes");
    ClosureCertificate {
        states: states
            .iter()
            .map(|(state_key, _)| ProductStateRef {
                model_state: state_key.clone(),
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
    let certificate = certificate(&invariant.run(&world));
    let reachable = world.reachable();
    assert_eq!(reachable.len(), 16);
    let mut expected = Vec::new();
    for object in world.objects() {
        expected.extend(closure(&reachable, &invariant.translation(&object)).states);
    }
    expected.sort();
    assert_eq!(certificate.states, expected);
    assert_eq!(
        check(&invariant.request(&world, UNLIMITED), &certificate),
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
    let full = certificate(&invariant.run(&world));
    let a = invariant.translation(&world.objects()[0]);
    let without = |values: &[i64]| {
        let at = ProductStateRef {
            model_state: world.state_key(values),
            automaton_state: a.key(SafetyState::Carried).expect("encodes"),
        };
        let mut certificate = full.clone();
        certificate.states.retain(|state| *state != at);
        assert_eq!(certificate.states.len(), full.states.len() - 1);
        (certificate, at)
    };
    let request = invariant.request(&world, UNLIMITED);
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
#[trace("TC-525", "FR-338-AC-2")]
#[test]
fn a_deadlock_is_bad_for_the_deadlock_item_alone() {
    let world = world(CONTRACT, &[("c", 0)]);
    let reachable = world.reachable();
    assert_eq!(reachable.len(), 4);
    let deadlock = SafetyTranslation::deadlock_freedom().expect("translates");
    let request = world.request(ModelCheckItem::DeadlockFreedom, UNLIMITED);
    assert_eq!(
        check(&request, &closure(&reachable, &deadlock),),
        rejected(
            CertificateRule::BadState,
            ProductStateRef {
                model_state: world.state_key(&[3]),
                automaton_state: deadlock.key(SafetyState::Rejected).expect("encodes"),
            }
        )
    );

    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let translation = invariant.translation(&world.objects()[0]);
    assert_eq!(
        check(
            &invariant.request(&world, UNLIMITED),
            &closure(&reachable, &translation)
        ),
        TemporalCertificateCheck::Accepted
    );
}

/// FR-338-AC-3's `ContractUndetermined` vector, over the Counter subject with
/// `pre CanInc { 6 mod (2 - self.value) >= 0 }`: a certificate for `always holds(true)` holding values 0
/// to 2 is rejected `BadState` at value 2, whose expansion evaluates the
/// precondition undefined. An atom that is undefined or false at a reached
/// state is `BadState` there under the rejecting state.
#[trace("TC-525", "FR-338-AC-3")]
#[test]
fn an_undetermined_contract_or_a_failing_atom_is_a_bad_state() {
    let function =
        "function odd_value using v(n: Int[0, 3]): Boolean pure { 6 mod (2 - n) >= 0 }\n";
    let guarded = format!(
        "{function}{}",
        GUARDED.replace("6 mod (2 - self.value) >= 0", "odd_value(self.value)")
    );
    let world = world_with_mode(&guarded, &[("c", 0)], CheckMode::Kernel);
    let reachable = world.reachable();
    assert_eq!(reachable.iter().map(|(_, values)| values[0]).max(), Some(2));
    let truth = world.invariant("always holds(true)", |_| Some(true));
    let translation = truth.translation(&world.objects()[0]);
    assert_eq!(
        check(
            &truth.request(&world, UNLIMITED),
            &closure(&reachable, &translation)
        ),
        rejected(
            CertificateRule::BadState,
            ProductStateRef {
                model_state: world.state_key(&[2]),
                automaton_state: translation.key(SafetyState::Carried).expect("encodes"),
            }
        )
    );

    let body = format!("{function}{CONTRACT}");
    let world = world_with_mode(&body, &[("c", 0)], CheckMode::Kernel);
    let reachable = world.reachable();
    for (formula, atom) in [
        (
            "always holds(c.value != 2)",
            (|v| Some(v != 2)) as fn(i64) -> Option<bool>,
        ),
        ("always holds(odd_value(c.value))", |v| {
            (v != 2).then(|| 6 % (2 - v) >= 0)
        }),
    ] {
        let invariant = world.invariant(formula, atom);
        let translation = invariant.translation(&world.objects()[0]);
        assert_eq!(
            check(
                &invariant.request(&world, UNLIMITED),
                &closure(&reachable, &translation)
            ),
            rejected(
                CertificateRule::BadState,
                ProductStateRef {
                    model_state: world.state_key(&[2]),
                    automaton_state: translation.key(SafetyState::Rejected).expect("encodes"),
                }
            ),
            "{formula}"
        );
    }
}

/// The certificate is checked against the request's own item: the
/// deadlock-freedom item's certificate offered for a clause is rejected
/// `ShapeMismatch`, before any key is used for membership; a model state
/// state key of another domain is rejected the same way. Every key refused, the
/// locus is the least refused state.
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
        check(&invariant.request(&world, UNLIMITED), &deadlock),
        rejected(CertificateRule::ShapeMismatch, least(&deadlock.states))
    );
}

/// TC-1079: the checker takes a clause's identity from the package alone.
/// Two structurally identical clauses `C` and `D` share one node and are two
/// `claim` occurrences of it; the identity each is checked under is the
/// node, ordinal and `over` node of its own declaration in the package, and
/// `C`'s certificate offered for `D` is rejected `ShapeMismatch`, since the
/// two key contexts differ in the occurrence.
#[trace("TC-1079")]
#[test]
fn the_checker_resolves_each_clause_s_identity_from_the_package() {
    let world = world(CONTRACT, &[("c", 0)]);
    let formula = "always holds(c.value <= 3)";
    let (package, clauses) = package_with_clauses(
        &world.documents.document,
        &world.body,
        &[("C", "", formula), ("D", "", formula)],
    );
    let [c, d] = clauses.as_slice() else {
        panic!("two declared clauses");
    };
    let nodes = package.graph().temporal_clauses();
    let [node_c, node_d] = nodes else {
        panic!("two clause declarations");
    };
    assert_eq!(node_c.identity(), node_d.identity(), "one shared node");
    let at = |clause: &CheckedTemporalClause| {
        ClauseIdentity::resolve(
            &package,
            &world.documents.packages,
            clause,
            &[],
            &Cancel::new(),
            EmitLimits::fixture(),
        )
        .expect("the package declares the clause")
    };
    for (identity, node) in [(at(c), node_c), (at(d), node_d)] {
        assert_eq!(*identity.occurrence().node(), node.identity());
        assert_eq!(identity.occurrence().origin(), node.claim());
        assert_eq!(identity.occurrence().origin().role().as_str(), "claim");
        assert_eq!(identity.over(), Some(node.over().clone()));
    }
    assert_ne!(
        at(c).occurrence().origin().ordinal(),
        at(d).occurrence().origin().ordinal()
    );

    let c = world.declared(package, c.clone(), |v| Some(v <= 3));
    let (package, _) = package_with_clauses(
        &world.documents.document,
        &world.body,
        &[("C", "", formula), ("D", "", formula)],
    );
    let d = world.declared(package, d.clone(), |v| Some(v <= 3));
    let own = certificate(&c.run(&world));
    let reachable = world.reachable();
    let mut expected = closure(&reachable, &c.translation(&world.objects()[0])).states;
    expected.sort();
    assert_eq!(own.states, expected);
    assert_eq!(
        check(&c.request(&world, UNLIMITED), &own),
        TemporalCertificateCheck::Accepted
    );
    assert_eq!(
        check(&d.request(&world, UNLIMITED), &own),
        rejected(CertificateRule::ShapeMismatch, least(&own.states))
    );
}

/// TC-1080 (the unresolved-root half of FR-338-AC-8): a request whose clause
/// the request's package does not declare has no root. The core refuses it,
/// it does not reject a certificate, and the settlement is `Failed`: not
/// `Certified`, not `Inconclusive`. The engine refuses it too, before any
/// state is explored.
#[trace("TC-1080", "FR-338-AC-8")]
#[test]
fn a_request_whose_clause_root_does_not_resolve_settles_failed() {
    let world = world(CONTRACT, &[("c", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let outcome = invariant.run(&world);
    let certificate = certificate(&outcome);
    assert_eq!(
        check(&invariant.request(&world, UNLIMITED), &certificate),
        TemporalCertificateCheck::Accepted
    );

    // A clause the request's package does not declare.
    let (_, mut others) = package_with_clauses(
        &world.documents.document,
        &world.body,
        &[("Other", "", "always holds(c.value <= 3)")],
    );
    let other = others.remove(0);
    let foreign = ModelCheckItem::Clause {
        clause: &other,
        letters: &invariant.letters,
    };
    let request = invariant.request_with(&world, foreign, UNLIMITED);
    let unresolved = ClauseRootRefusal::Undeclared {
        name: "Other".to_owned(),
    };
    match check_closure(
        &request,
        &certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    ) {
        Err(ClosureRefusal::Item(ItemRefusal::ClauseRoot(refusal))) => {
            assert_eq!(refusal, unresolved);
        }
        other => panic!("expected the clause-root-unresolved refusal, got {other:?}"),
    }
    let refused = check_model(
        ModelCheckRequest {
            temporal_clock_selections: &[],
            subject: request.subject,
            item: foreign,
            max_depth: None,
            limits: ModelCheckLimits::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    );
    assert!(
        matches!(&refused, Err(ModelCheckRefusal::Item(ItemRefusal::ClauseRoot(refusal))) if *refusal == unresolved),
        "{refused:?}"
    );

    let request = invariant.request_with(&world, foreign, UNLIMITED);
    let record = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &request,
        &Cancel::new(),
        |_| panic!("a proof replays nothing"),
    );
    assert_eq!(record.value, TerminalValue::Failed);
    assert_eq!(record.label, ResultLabel::Failed);
    assert_eq!(record.execution, Execution::Failed);
}

/// A TP-3 proof has no canonical closure certificate until its translation
/// states have FR-338 keys (QSL-878): over the Counter subject,
/// `always eventually[0,1] holds(c.value <= 3)` under infinite-trace holds
/// with an empty certificate, which settles `Failed`, never `Proved`.
#[test]
fn a_safety_proof_with_an_empty_certificate_settles_failed_never_proved() {
    let world = world(CONTRACT, &[("c", 0)]);
    let claim = world.invariant("always eventually[0,1] holds(c.value <= 3)", |v| {
        Some(v <= 3)
    });
    let outcome = claim.run(&world);
    assert!(certificate(&outcome).states.is_empty());
    let record = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &claim.request(&world, UNLIMITED),
        &Cancel::new(),
        |_| panic!("a proof replays nothing"),
    );
    assert_eq!(record.value, TerminalValue::Failed);
    assert_eq!(
        (record.label, record.execution),
        (ResultLabel::Failed, Execution::Failed)
    );
}

/// FR-127's certificate rows: the settlement map runs the core checker on
/// a `Holds` outcome's certificate itself, over the request's own package. The engine's certificate settles
/// `proved`, `closed-scope`, `Proved{Exhaustive, Certified}`; the same
/// outcome with a state removed settles `inconclusive`, `unsettled`,
/// `CertificateRejected{SuccessorMissing, at}`, never `proved`.
#[trace("TC-525", "FR-127-AC-1")]
#[test]
fn a_proof_settles_certified_only_when_its_certificate_is_accepted() {
    let world = world(CONTRACT, &[("c", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let outcome = invariant.run(&world);
    let request = invariant.request(&world, UNLIMITED);
    let settle = |outcome: &ModelCheckOutcome, cancel: &Cancel| {
        settle_model_check(
            outcome,
            &ModelCheckLimits::default(),
            &request,
            cancel,
            |_| panic!("a proof replays nothing"),
        )
    };
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

    let ModelCheckOutcome::Holds { certificate, stats } = outcome else {
        panic!("a proof");
    };
    let mut tampered = closure_proof(&certificate).clone();
    let one = world.state_key(&[1]);
    let index = tampered
        .states
        .iter()
        .position(|state| state.model_state == one)
        .expect("value 1 is certified");
    let at = tampered.states.remove(index);
    let record = settle(
        &ModelCheckOutcome::Holds {
            certificate: ProofCertificate::Closure(tampered),
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

/// FR-127-AC-7 end to end through the settlement map, on FR-338's two-counter
/// `{a, b}` vector. The accepted TP-1 certificate settles `proved`,
/// `closed-scope`, `Proved{Exhaustive, Certified}`; the certificate with
/// model state (1, 0) removed for instance `a` settles `inconclusive`,
/// `unsettled`, `CertificateRejected{SuccessorMissing, (1, 0)}`. A graph
/// budget of 2 states settles `Incomplete(LimitReached{max_states, 2})` whose
/// record names `model_check.max_states`; a zero `work_units` meter settles
/// `LimitReached{work_units, 0}` naming `accounting.work_units`; a requested
/// and a deadline cancellation keep their source. None of the stops settles
/// `CertificateRejected` (of any rule, `WitnessFails` included).
#[trace("TC-522", "FR-127-AC-7")]
#[test]
fn a_certificate_check_settles_accepted_rejected_or_stopped_with_the_named_setting() {
    let world = world(CONTRACT, &[("a", 0), ("b", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let outcome = invariant.run(&world);
    let settle =
        |outcome: &ModelCheckOutcome, limits: &ModelCheckLimits, meter, cancel: &Cancel| {
            settle_model_check(
                outcome,
                limits,
                &invariant.request(&world, meter),
                cancel,
                |_| panic!("a proof replays nothing"),
            )
        };
    let defaults = ModelCheckLimits::default();

    let accepted = settle(&outcome, &defaults, UNLIMITED, &Cancel::new());
    assert_eq!(
        accepted.value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );
    assert_eq!(
        (accepted.label, accepted.basis),
        (ResultLabel::Proved, SettlementBasis::ClosedScope)
    );

    let ModelCheckOutcome::Holds { certificate, stats } = &outcome else {
        panic!("a proof");
    };
    let a = invariant.translation(&world.objects()[0]);
    let at = ProductStateRef {
        model_state: world.state_key(&[1, 0]),
        automaton_state: a.key(SafetyState::Carried).expect("encodes"),
    };
    let mut tampered = closure_proof(certificate).clone();
    tampered.states.retain(|state| *state != at);
    assert_eq!(
        tampered.states.len(),
        closure_proof(certificate).states.len() - 1
    );
    let rejected = settle(
        &ModelCheckOutcome::Holds {
            certificate: ProofCertificate::Closure(tampered),
            stats: *stats,
        },
        &defaults,
        UNLIMITED,
        &Cancel::new(),
    );
    assert_eq!(
        rejected.value,
        TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::SuccessorMissing,
            at: CertificateLocus::ProductState(at),
        })
    );
    assert_eq!(
        (rejected.label, rejected.basis),
        (ResultLabel::Inconclusive, SettlementBasis::Unsettled)
    );

    let mut small = defaults;
    small.limits.max_states = 2;
    let max_states = settle(&outcome, &small, UNLIMITED, &Cancel::new());
    let work_units = settle(&outcome, &defaults, ZERO, &Cancel::new());
    let requested = Cancel::new();
    requested.cancel(CancelCause::Requested);
    let deadline = Cancel::new();
    deadline.cancel(CancelCause::Deadline);
    let cancelled_requested = settle(&outcome, &defaults, UNLIMITED, &requested);
    let cancelled_deadline = settle(&outcome, &defaults, UNLIMITED, &deadline);

    let meter = ModelCheckLimit::EvaluationMeter {
        counter: filament_exact::LimitKind::ValueOccurrences,
    };
    let limit_stops = [
        (
            &max_states,
            ModelCheckLimit::MaxStates,
            2,
            "model_check.max_states",
        ),
        (&work_units, meter, 0, "accounting.value_occurrences"),
    ];
    for (record, limit, value, setting) in limit_stops {
        assert_eq!(
            record.value,
            TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, value })
        );
        assert_eq!(record.limit_reached, Some(ReachedLimit { limit, value }));
        assert_eq!(limit.setting(), setting);
    }
    for (record, source) in [
        (&cancelled_requested, CancelCause::Requested),
        (&cancelled_deadline, CancelCause::Deadline),
    ] {
        assert_eq!(
            record.value,
            TerminalValue::Incomplete(IncompleteCause::Cancelled { source })
        );
        assert_eq!(record.limit_reached, None);
    }
    for stopped in [
        &max_states,
        &work_units,
        &cancelled_requested,
        &cancelled_deadline,
    ] {
        assert_eq!(stopped.execution, Execution::ResourceIncomplete);
        assert!(!matches!(
            stopped.value,
            TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected { .. })
        ));
    }
}

/// A cancellation that arrives after the package is bound, during the
/// certificate check itself, settles `Incomplete(Cancelled{source})` with no
/// limit and never `CertificateRejected`. The handle is cancelled from its
/// observer at the last meter charge of the settlement, which the package
/// emission alone does not reach.
#[trace("TC-522")]
#[test]
fn a_cancellation_during_the_certificate_check_settles_cancelled() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, OnceLock};

    let world = world(CONTRACT, &[("a", 0), ("b", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let outcome = invariant.run(&world);
    let request = invariant.request(&world, UNLIMITED);
    let clock_selections = tcb_01_event_position_selection(
        request
            .subject
            .simulation
            .source_package()
            .expect("a source-route subject"),
    );
    let settle = |cancel: &Cancel| {
        settle_model_check(
            &outcome,
            &ModelCheckLimits::default(),
            &request,
            cancel,
            |_| panic!("a proof replays nothing"),
        )
    };
    let counted = |run: &dyn Fn(&Cancel)| {
        let polls = Arc::new(AtomicU32::new(0));
        let seen = Arc::clone(&polls);
        run(&Cancel::observing(move || {
            seen.fetch_add(1, Ordering::Relaxed);
        }));
        polls.load(Ordering::Relaxed)
    };
    let emission = counted(&|cancel| {
        fsl_package::emit_checked_with_temporal_selections_and_cancel(
            request
                .subject
                .simulation
                .source_package()
                .expect("a source-route subject"),
            &clock_selections,
            cancel,
            EmitLimits::fixture(),
        )
        .expect("emits with the fixture's selected temporal clock");
    });
    let total = counted(&|cancel| {
        settle(cancel);
    });
    assert!(
        total > emission,
        "the check charges the meter after binding"
    );

    for source in [CancelCause::Requested, CancelCause::Deadline] {
        let polls = Arc::new(AtomicU32::new(0));
        let handle: Arc<OnceLock<Cancel>> = Arc::new(OnceLock::new());
        let (seen, target) = (Arc::clone(&polls), Arc::clone(&handle));
        let cancel = Cancel::observing(move || {
            if seen.fetch_add(1, Ordering::Relaxed) + 1 == total {
                if let Some(cancel) = target.get() {
                    cancel.cancel(source);
                }
            }
        });
        assert!(handle.set(cancel.clone()).is_ok());
        let record = settle(&cancel);
        assert_eq!(cancel.cause(), Some(source));
        assert_eq!(
            record.value,
            TerminalValue::Incomplete(IncompleteCause::Cancelled { source })
        );
        assert_eq!(record.limit_reached, None);
    }
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
    let outcome = invariant.run(&world);
    let certificate = certificate(&outcome);
    let request = invariant.request(&world, UNLIMITED);
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
            &invariant.request(&world, ZERO),
            &certificate,
            &defaults,
            &Cancel::new()
        )
        .expect("checked"),
        stop(
            ModelCheckLimit::EvaluationMeter {
                counter: filament_exact::LimitKind::ValueOccurrences
            },
            0
        )
    );
    for source in [CancelCause::Requested, CancelCause::Deadline] {
        let cancel = Cancel::new();
        cancel.cancel(source);
        assert_eq!(
            check_closure(&request, &certificate, &defaults, &cancel).expect("checked"),
            TemporalCertificateCheck::Stopped(IncompleteCause::Cancelled { source })
        );
    }

    let record = settle_model_check(&outcome, &states, &request, &Cancel::new(), |_| {
        panic!("a proof replays nothing")
    });
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

/// FR-125: a clause whose subject's universe is empty has no instance and
/// is `Undecided(NoInstance)` before any state is explored, and the checker refuses to certify it: an empty
/// certificate is never an acceptance.
#[trace("TC-525")]
#[test]
fn a_clause_with_no_instance_is_never_certified() {
    let world = world(CONTRACT, &[]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    assert!(matches!(
        invariant.run(&world),
        ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInstance,
            stats,
        } if stats.product_states == 0
    ));
    assert!(matches!(
        check_closure(
            &invariant.request(&world, UNLIMITED),
            &ClosureCertificate { states: Vec::new() },
            &ModelCheckLimits::default(),
            &Cancel::new()
        ),
        Err(ClosureRefusal::NoInstance)
    ));
}

/// FSpec FR-361-AC-8 (FSpec TC-294) and FR-127: a clause
/// whose empty universe gives it zero instances settles `inconclusive` with
/// the wire cause `no-instance` and basis `unsettled`, never `proved` or
/// `refuted`. The zero instances come from the subject's empty universe of
/// `counters`.
#[trace("TC-520", "FSpec-FR-361-AC-8")]
#[test]
fn a_clause_with_no_instance_settles_inconclusive_never_proved_or_refuted() {
    let world = world(CONTRACT, &[]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let outcome = invariant.run(&world);
    assert!(matches!(
        outcome,
        ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInstance,
            stats,
        } if stats.product_states == 0
    ));
    let request = world.request(invariant.item(), UNLIMITED);
    let record = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &request,
        &Cancel::new(),
        |_| panic!("no instance replays nothing"),
    );
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::NoInstance)
    );
    assert_eq!(InconclusiveCause::NoInstance.as_str(), "no-instance");
    assert_eq!(record.label, ResultLabel::Inconclusive);
    assert_eq!(record.basis, SettlementBasis::Unsettled);
    assert_eq!(record.execution, Execution::Completed);
    assert!(record.counterexample.is_none());
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
    let empty = ClosureCertificate { states: Vec::new() };
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
            certificate: ProofCertificate::Closure(empty),
            stats: fsl_replay::RunStats::default(),
        },
        &ModelCheckLimits::default(),
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
    let mut certificate = certificate(&invariant.run(&world));
    let padding = certificate.states[0].clone();
    certificate.states.extend(std::iter::repeat_n(
        ProductStateRef {
            automaton_state: fsl_eval::automaton::AutomatonStateKey::imported(b"[".to_vec()),
            ..padding
        },
        10,
    ));
    let request = invariant.request(&world, UNLIMITED);
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
            automaton_state: fsl_eval::automaton::AutomatonStateKey::imported(
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

/// FR-338's refused-key outcome on the two-counter Counter subject standing
/// in for ADR-018 §6's: the engine's certificate with one state's key
/// replaced in turn by a foreign-context key, a non-canonical encoding and
/// an unsorted set is rejected `ShapeMismatch` at that state, carrying the
/// refused key and its model state, and settles `CertificateRejected`, not
/// `proved`; restored, it is accepted. With two states' keys refused, both
/// orderings of `states` report the least refused `ProductStateRef`.
#[trace("TC-1080", "FR-338-AC-8")]
#[test]
fn a_refused_key_rejects_the_certificate_at_the_least_refused_state() {
    let world = world(CONTRACT, &[("a", 0), ("b", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let outcome = invariant.run(&world);
    let full = certificate(&outcome);
    let request = invariant.request(&world, UNLIMITED);
    let imported =
        |octets: String| fsl_eval::automaton::AutomatonStateKey::imported(octets.into_bytes());
    let valid =
        String::from_utf8(full.states[5].automaton_state.as_bytes().to_vec()).expect("UTF-8");
    let carried_body = r#"["safety",[[[[[],"positive"]],[],[],[]]]]"#;
    assert!(valid.ends_with(&format!("{carried_body}]")), "{valid}");
    let deadlock = SafetyTranslation::deadlock_freedom()
        .expect("translates")
        .key(SafetyState::Carried)
        .expect("encodes");
    for refused_key in [
        deadlock,
        imported(valid.replacen(",", ", ", 1)),
        imported(valid.replace(
            carried_body,
            r#"["safety",[[[],[],[],[]],[[[[],"positive"]],[],[],[]]]]"#,
        )),
    ] {
        let mut certificate = full.clone();
        certificate.states[5].automaton_state = refused_key;
        let at = certificate.states[5].clone();
        assert_eq!(
            check(&request, &certificate),
            rejected(CertificateRule::ShapeMismatch, at.clone())
        );
        let record = settle_model_check(
            &ModelCheckOutcome::Holds {
                certificate: ProofCertificate::Closure(certificate),
                stats: outcome.stats(),
            },
            &ModelCheckLimits::default(),
            &request,
            &Cancel::new(),
            |_| panic!("a proof replays nothing"),
        );
        assert_eq!(
            record.value,
            TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
                rule: CertificateRule::ShapeMismatch,
                at: CertificateLocus::ProductState(at),
            })
        );
    }
    assert_eq!(check(&request, &full), TemporalCertificateCheck::Accepted);

    let mut two = full.clone();
    for index in [3, 11] {
        two.states[index].automaton_state = imported(valid.replacen(",", ", ", 1));
    }
    let least = [two.states[3].clone(), two.states[11].clone()]
        .into_iter()
        .min_by(|left, right| {
            (left.model_state.as_bytes(), left.automaton_state.as_bytes()).cmp(&(
                right.model_state.as_bytes(),
                right.automaton_state.as_bytes(),
            ))
        })
        .expect("two refused states");
    assert_eq!(
        check(&request, &two),
        rejected(CertificateRule::ShapeMismatch, least.clone())
    );
    two.states.reverse();
    assert_eq!(
        check(&request, &two),
        rejected(CertificateRule::ShapeMismatch, least)
    );
}

const AT_MOST: &str =
    "invariant AtMostThree using v on M::Counter at current { self.value <= 3 }\n\
                       invariant AtMostTwo using v on M::Counter at current { self.value <= 2 }\n\
                       constraint Low using v on M::Counter at current { self.value <= 3 }\n";

/// The checked state clause `name` of `world`'s package.
fn state_clause<'w>(world: &'w World, name: &str) -> &'w fsl_semantics::check::CheckedStateClause {
    world
        .package
        .graph()
        .state_clauses()
        .iter()
        .find(|clause| clause.name() == name)
        .unwrap_or_else(|| panic!("{name} is checked"))
}

/// The invariant root's translation of `clause` (FR-338).
fn invariant_translation(clause: &fsl_semantics::check::CheckedStateClause) -> SafetyTranslation {
    SafetyTranslation::state_invariant(fsl_foundation::source::provenance::OccurrenceKey::new(
        clause.identity(),
        clause.claim().clone(),
    ))
    .expect("translates")
}

/// FR-951-AC-1 and AC-3 over the one-counter subject from 0 (values 0 to
/// 3): `invariant AtMostThree`'s EN-1 proof carries a closure certificate of
/// every reachable model state under the carried root of the invariant
/// key-context root `["invariant", N, "claim", O]` with binding `[]`, and
/// FR-338's checker accepts it. The same states under `AtMostTwo`'s root
/// are rejected `BadState` at `value = 3`, where its letter is false, and a
/// certificate request whose invariant names the `constraint` clause `Low`
/// is refused as a foreign clause root.
#[trace("TC-2661", "FR-951-AC-1", "FR-951-AC-3")]
#[test]
fn an_invariant_proof_is_certified_under_the_invariant_root() {
    let body = format!("{CONTRACT}{AT_MOST}");
    let world = world(&body, &[("c", 0)]);
    let reachable = world.reachable();
    assert_eq!(reachable.len(), 4);

    let three = state_clause(&world, "AtMostThree");
    let item = ModelCheckItem::Invariant {
        clause: fsl_replay::InvariantClause::of(three),
    };
    let certificate = certificate(&world.run_in(&world.package, item));
    let translation = invariant_translation(three);
    let mut expected = closure(&reachable, &translation).states;
    expected.sort();
    let mut states = certificate.states.clone();
    states.sort();
    assert_eq!(states, expected);
    let key: serde_json::Value =
        serde_json::from_slice(certificate.states[0].automaton_state.as_bytes()).expect("JSON");
    let context = &key[1];
    assert_eq!(context[0][0], "invariant", "{key}");
    assert!(
        context[0][1].is_object(),
        "the root's node is a NodeId: {key}"
    );
    assert_eq!(context[0][2], "claim", "{key}");
    assert_eq!(context[0][3], three.claim().ordinal().to_string(), "{key}");
    assert_eq!(context[1], "filament.temporal.infinite-trace/v1", "{key}");
    assert_eq!(context[2], "infinite", "{key}");
    assert_eq!(context[3], serde_json::json!([]), "{key}");
    assert_eq!(
        check(&world.request(item, UNLIMITED), &certificate),
        TemporalCertificateCheck::Accepted
    );

    let two = state_clause(&world, "AtMostTwo");
    let bad = invariant_translation(two);
    assert_eq!(
        check(
            &world.request(
                ModelCheckItem::Invariant {
                    clause: fsl_replay::InvariantClause::of(two),
                },
                UNLIMITED
            ),
            &closure(&reachable, &bad)
        ),
        rejected(
            CertificateRule::BadState,
            ProductStateRef {
                model_state: world.state_key(&[3]),
                automaton_state: bad.key(SafetyState::Rejected).expect("encodes"),
            }
        )
    );

    let low = state_clause(&world, "Low");
    let refused = check_closure(
        &world.request(
            ModelCheckItem::Invariant {
                clause: fsl_replay::InvariantClause::of(low),
            },
            UNLIMITED,
        ),
        &certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    );
    assert!(
        matches!(
            refused,
            Err(ClosureRefusal::Item(ItemRefusal::ClauseRoot(
                ClauseRootRefusal::Undeclared { .. }
            )))
        ),
        "{refused:?}"
    );
}

#[trace(
    "TC-645",
    "TC-646",
    "FR-163-AC-1",
    "FR-163-AC-2",
    "FR-163-AC-4",
    "FR-163-AC-5"
)]
#[test]
fn hyper_product_certificate_is_recomputed_and_rejects_tampering() {
    use crate::hyper_world::{non_interference, Vault, World, EQUAL_INPUTS, STARTS};
    use filament_exact::Cancel;
    use fsl_replay::{
        check_product_closure, CertificateLocus, CertificateMember, CertificateRule, HyperOutcome,
        ModelCheckLimits, ProductClosureRequest, TemporalCertificateCheck,
    };
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let cancel = Cancel::new();
    let outcome = world
        .run(ModelCheckLimits::default(), &cancel)
        .expect("secure product search");
    let HyperOutcome::HoldsReduced { certificates, .. } = outcome else {
        panic!("secure copy-swap proof");
    };
    let subjects = vec![world.subject()];
    let request = ProductClosureRequest {
        package: &world.package,
        clause: &world.clause,
        subjects: &subjects,
        meter: crate::counter::UNLIMITED,
    };
    let checked = |proof: &fsl_replay::ProductClosureCertificate, limits: &ModelCheckLimits| {
        check_product_closure(&request, proof, limits, &cancel).expect("core check")
    };
    let proof = certificates.first().expect("one instance certificate");
    assert_eq!(proof.closure.len(), 6);
    let repeated = world
        .run(ModelCheckLimits::default(), &cancel)
        .expect("repeat search");
    assert_eq!(
        certificates,
        match repeated {
            HyperOutcome::HoldsReduced { certificates, .. } => certificates,
            other => panic!("repeat proof: {other:?}"),
        }
    );
    let limits = ModelCheckLimits::default();
    assert_eq!(checked(proof, &limits), TemporalCertificateCheck::Accepted);
    assert_eq!(checked(proof, &limits), TemporalCertificateCheck::Accepted);
    let mut malformed = proof.clone();
    malformed.components = None;
    assert!(
        matches!(checked(&malformed, &limits), TemporalCertificateCheck::Rejected(rejection)
        if rejection.rule == CertificateRule::Malformed
            && rejection.at == CertificateLocus::CertificateMember(CertificateMember::Components))
    );
    let mut duplicate = proof.clone();
    duplicate.closure.insert(0, duplicate.closure[0].clone());
    assert!(
        matches!(checked(&duplicate, &limits), TemporalCertificateCheck::Rejected(rejection)
        if rejection.rule == CertificateRule::Malformed)
    );
    let mut unsorted = proof.clone();
    unsorted.closure.swap(0, 1);
    assert_eq!(
        checked(&unsorted, &limits),
        TemporalCertificateCheck::Rejected(fsl_replay::CertificateRejection {
            rule: CertificateRule::Malformed,
            at: CertificateLocus::HyperProductState(unsorted.closure[1].0.clone()),
        })
    );
    let components = proof.components.as_ref().expect("universal partition");
    assert!(proof.closure.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(components
        .iter()
        .all(|component| component.states.windows(2).all(|pair| pair[0] < pair[1])));
    let mut overlapping = proof.clone();
    overlapping.components.as_mut().expect("partition")[0]
        .states
        .push(components[0].states[0].clone());
    assert!(
        matches!(checked(&overlapping, &limits), TemporalCertificateCheck::Rejected(rejection)
        if rejection.rule == CertificateRule::NotPartition)
    );
    let mut bad_witness = proof.clone();
    for component in bad_witness.components.as_mut().expect("partition") {
        component.witness = fsl_replay::ComponentWitness::MissingAcceptance { set: u32::MAX };
    }
    assert!(
        matches!(checked(&bad_witness, &limits), TemporalCertificateCheck::Rejected(rejection)
        if rejection.rule == CertificateRule::WitnessFails)
    );
    let mut backward = proof.clone();
    // Each secure SCC contains the two public outputs. Splitting its cycle
    // into singleton components makes a backward edge unavoidable.
    backward.components = Some(
        proof
            .closure
            .iter()
            .map(|state| fsl_replay::Component {
                states: vec![state.clone()],
                witness: fsl_replay::ComponentWitness::Trivial,
            })
            .collect(),
    );
    assert!(
        matches!(checked(&backward, &limits), TemporalCertificateCheck::Rejected(rejection)
        if rejection.rule == CertificateRule::BackwardEdge)
    );
    let mut alternative_order = proof.clone();
    alternative_order
        .components
        .as_mut()
        .expect("partition")
        .reverse();
    assert_eq!(
        checked(&alternative_order, &limits),
        TemporalCertificateCheck::Accepted
    );
    // Removing a reached non-initial state must fail closure before the
    // component partition is examined.
    let mut successor_missing = false;
    for index in 0..proof.closure.len() {
        let mut missing = proof.clone();
        missing.closure.remove(index);
        if matches!(checked(&missing, &limits), TemporalCertificateCheck::Rejected(rejection)
            if rejection.rule == CertificateRule::SuccessorMissing)
        {
            successor_missing = true;
        }
    }
    assert!(
        successor_missing,
        "a non-initial member must be required by closure"
    );
    let mut missing = proof.clone();
    missing.closure.clear();
    assert!(
        matches!(checked(&missing, &limits), TemporalCertificateCheck::Rejected(rejection)
        if rejection.rule == CertificateRule::InitialMissing)
    );
    let tight = ModelCheckLimits {
        limits: fsl_eval::simulation::Limits {
            max_states: 1,
            ..limits.limits
        },
        ..limits
    };
    assert_eq!(
        checked(proof, &tight),
        TemporalCertificateCheck::Stopped(fsl_replay::IncompleteCause::LimitReached {
            limit: fsl_replay::ModelCheckLimit::MaxStates,
            value: 1,
        })
    );
}

/// Missing component structure has its own locus even when no state key is
/// supplied. A zero recomputation budget must not hide that malformed shape.
#[trace("TC-646", "FR-163-AC-5")]
#[test]
fn hyper_product_missing_components_precedes_recomputation_for_both_forms() {
    use crate::hyper_world::{non_interference, opaque, Vault, World, EQUAL_INPUTS, STARTS};
    use fsl_replay::{
        check_product_closure, CertificateMember, HyperOutcome, ProductClosureRequest,
    };

    for declaration in [non_interference(EQUAL_INPUTS), opaque("")] {
        let world = World::new(Vault::Secure, false, &STARTS, &declaration);
        let cancel = Cancel::new();
        let outcome = world
            .run(ModelCheckLimits::default(), &cancel)
            .expect("proof");
        let certificates = match outcome {
            HyperOutcome::Holds { certificates, .. }
            | HyperOutcome::HoldsReduced { certificates, .. } => certificates,
            other => panic!("holding world: {other:?}"),
        };
        let subjects = [world.subject()];
        let request = ProductClosureRequest {
            package: &world.package,
            clause: &world.clause,
            subjects: &subjects,
            meter: UNLIMITED,
        };
        let mut proof = certificates.into_iter().next().expect("one binding");
        proof.components = None;
        let limits = ModelCheckLimits {
            limits: fsl_eval::simulation::Limits {
                max_states: 0,
                max_transitions: 0,
            },
            ..ModelCheckLimits::default()
        };
        for empty in [false, true] {
            if empty {
                proof.closure.clear();
            }
            assert_eq!(
                check_product_closure(&request, &proof, &limits, &cancel).expect("core check"),
                TemporalCertificateCheck::Rejected(CertificateRejection {
                    rule: CertificateRule::Malformed,
                    at: CertificateLocus::CertificateMember(CertificateMember::Components),
                }),
                "form {:?}, empty closure {empty}",
                proof.form
            );
        }
    }
}

/// A forged engine proof and a checker budget stop must reach settlement as
/// inconclusive and incomplete respectively; neither may retain certification.
#[trace("TC-646", "FR-163-AC-2", "FR-163-AC-4", "FR-182-AC-1")]
#[test]
fn hyper_product_rejection_and_checker_stop_never_settle_certified() {
    use crate::hyper_world::{non_interference, Vault, World, EQUAL_INPUTS, STARTS};
    use fsl_replay::{settle_hyper_check, HyperOutcome, ProductClosureRequest};

    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let cancel = Cancel::new();
    let outcome = world
        .run(ModelCheckLimits::default(), &cancel)
        .expect("proof");
    let subjects = [world.subject()];
    let request = ProductClosureRequest {
        package: &world.package,
        clause: &world.clause,
        subjects: &subjects,
        meter: UNLIMITED,
    };
    let settle = |outcome: &HyperOutcome, limits: &ModelCheckLimits| {
        settle_hyper_check(outcome, limits, &[], Some(&request), &cancel, |_| {
            panic!("a holding outcome has no counterexample to replay")
        })
        .expect("settled core check")
    };
    let limits = ModelCheckLimits::default();
    let mut forged = outcome.clone();
    let certificates = match &mut forged {
        HyperOutcome::HoldsReduced { certificates, .. } => certificates,
        other => panic!("copy-swap proof: {other:?}"),
    };
    certificates[0].closure.clear();
    let record = settle(&forged, &limits);
    assert!(matches!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::InitialMissing,
            ..
        })
    ));
    assert_eq!(record.label, ResultLabel::Inconclusive);
    assert!(record.certificates.is_empty());
    let tight = ModelCheckLimits {
        limits: fsl_eval::simulation::Limits {
            max_states: 1,
            ..limits.limits
        },
        ..limits
    };
    let record = settle(&outcome, &tight);
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 1,
        })
    );
    assert_eq!(record.label, ResultLabel::Failed);
    assert_eq!(record.execution, Execution::ResourceIncomplete);
    assert!(record.certificates.is_empty());
}

/// Closure failure precedes a rejecting monitor member, even when that
/// rejecting member is initial and is visited before the missing successor.
#[trace("TC-646", "FR-163-AC-2")]
#[test]
fn hyper_product_closure_is_checked_before_a_projected_violation() {
    use crate::hyper_world::{instance, Vault, World};
    use fsl_analyze::model_check::retain;
    use fsl_eval::hyper::{
        CheckedHyperLetters, CheckedStepMatch, ProjectedInputs, ProjectedProduct,
    };
    use fsl_replay::{
        check_product_closure, product_state_key, ProductClosureCertificate, ProductClosureRequest,
    };

    let declaration = crate::hyper_projected::aligned(&[])
        .replace("v.l", "v.h")
        .replace("v.h @ a = v.h @ b", "v.h @ a != v.h @ b");
    let world = World::busy(Vault::Busy, &crate::hyper_projected::IDLE, &declaration);
    let subject = world.subject();
    let system = ModelSystem::new(subject.simulation.clone()).expect("subject");
    let systems = vec![&system; world.clause.quantifiers().len()];
    let letters =
        CheckedHyperLetters::new(&world.package, &world.clause, systems.clone()).expect("letters");
    let matcher = CheckedStepMatch::new(&world.package, &world.clause);
    let cancel = Cancel::new();
    let instance = instance();
    let limits = ModelCheckLimits::default();
    let mut product = ProjectedProduct::new(ProjectedInputs {
        systems,
        instance: &instance,
        body: world.clause.body(),
        profile: world.clause.profile(),
        align_skip: world.clause.align_skip(),
        letters: &letters,
        matcher: Some(&matcher),
        max_automaton_states: limits.max_automaton_states,
        meter: UNLIMITED,
        cancel: &cancel,
    })
    .expect("projected product");
    product.start().expect("initial letters");
    let explored = retain(
        &product,
        limits.limits,
        fsl_foundation::IdentityLimits::default(),
        None,
        || false,
    )
    .expect("retained product");
    let mut closure = explored
        .graph
        .states()
        .iter()
        .map(|member| product_state_key(&product.key(&member.state)).expect("key"))
        .collect::<Vec<_>>();
    closure.sort();
    let mut proof = ProductClosureCertificate {
        form: fsl_semantics::check::HyperForm::ProjectionAligned,
        instance: instance
            .iter()
            .map(|binding| (binding.parameter.clone(), binding.instance.key.clone()))
            .collect(),
        reduction: None,
        closure,
        components: None,
    };
    let subjects = [subject];
    let request = ProductClosureRequest {
        package: &world.package,
        clause: &world.clause,
        subjects: &subjects,
        meter: UNLIMITED,
    };
    assert!(
        matches!(check_product_closure(&request, &proof, &limits, &cancel).expect("check"),
        TemporalCertificateCheck::Rejected(rejection) if rejection.rule == CertificateRule::ViolatingMember)
    );
    let initial = product
        .initial()
        .iter()
        .map(|state| product_state_key(&product.key(state)).expect("initial key"))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        proof.closure.len() > initial.len(),
        "the product has non-initial successors"
    );
    proof.closure.retain(|key| initial.contains(key));
    assert!(
        matches!(check_product_closure(&request, &proof, &limits, &cancel).expect("check"),
        TemporalCertificateCheck::Rejected(rejection) if rejection.rule == CertificateRule::SuccessorMissing)
    );
}

/// HP-3's empty universal tuple is real data, independent of whether its
/// existential witness set is populated or empty.
#[trace("TC-646", "FR-163-AC-7")]
#[test]
fn hyper_product_zero_universal_keys_preserve_their_witness_sets() {
    use crate::hyper_world::{Vault, World, STARTS};
    use fsl_eval::hyper::Property;
    use fsl_eval::simulation::StateKey;
    use fsl_replay::ProductStateKey;

    let declaration = "hyper Hidden using inf over behaviours (v: V::Vault) \
        exists trace b of V exists trace c of V \
        match { b.step.Vault::step.i = c.step.Vault::step.i } \
        { holds(v.h @ b != v.h @ c) and always holds(v.l @ b = v.l @ c) }";
    for (vault, empty) in [(Vault::Secure, false), (Vault::Leaky, true)] {
        let world = World::new(vault, false, &STARTS, declaration);
        let explored = world.explore_witness();
        let member = explored
            .graph
            .states()
            .iter()
            .find(|member| {
                matches!(&member.state.property, Property::Witnesses(set)
                if set.members().is_empty() == empty)
            })
            .expect("the product reaches the expected witness set");
        assert!(member.state.components.is_empty());
        let key = ProductStateKey(StateKey::from_canonical_bytes(member.key.clone()));
        let bytes = serde_json::to_vec(&key).expect("serialize actual product key");
        let read: serde_json::Value = serde_json::from_slice(&bytes).expect("read key document");
        assert_eq!(read["states"], serde_json::json!([]));
        let witnesses = read["witness_set"].as_array().expect("present witness set");
        assert_eq!(witnesses.is_empty(), empty);
        assert!(read.get("automaton_state").is_none());
        for witness in witnesses {
            assert_eq!(witness["states"].as_array().map(Vec::len), Some(2));
            assert!(witness["automaton_state"]
                .as_str()
                .is_some_and(|key| !key.is_empty()));
        }
    }
}

/// A secure search's component witnesses cannot certify the leaking
/// product, even if every leaking state and successor is supplied.
#[trace("TC-646", "FR-163-AC-3")]
#[test]
fn hyper_product_leaky_closure_rejects_the_secure_partition() {
    use crate::hyper_world::{non_interference, Vault, World, EQUAL_INPUTS, STARTS};
    use fsl_eval::simulation::StateKey;
    use fsl_replay::{check_product_closure, HyperOutcome, ProductClosureRequest, ProductStateKey};

    let declaration = non_interference(EQUAL_INPUTS);
    let secure = World::new(Vault::Secure, false, &STARTS, &declaration);
    let limits = ModelCheckLimits::default();
    let cancel = Cancel::new();
    let HyperOutcome::HoldsReduced {
        mut certificates, ..
    } = secure.run(limits, &cancel).expect("secure proof")
    else {
        panic!("secure copy-swap proof");
    };
    let mut proof = certificates.pop().expect("one binding");
    let leaky = World::new(Vault::Leaky, false, &STARTS, &declaration);
    let explored = leaky.explore();
    proof.reduction = None;
    proof.closure = explored
        .graph
        .states()
        .iter()
        .map(|member| ProductStateKey(StateKey::from_canonical_bytes(member.key.clone())))
        .collect();
    proof.closure.sort();
    assert!(proof.closure.len() > secure.explore().graph.states().len());
    let subjects = [leaky.subject()];
    let request = ProductClosureRequest {
        package: &leaky.package,
        clause: &leaky.clause,
        subjects: &subjects,
        meter: UNLIMITED,
    };
    assert!(
        matches!(check_product_closure(&request, &proof, &limits, &cancel).expect("core check"),
        TemporalCertificateCheck::Rejected(rejection) if rejection.rule == CertificateRule::NotPartition)
    );
}
