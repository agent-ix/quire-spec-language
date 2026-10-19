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
use qsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest};
use qsl_eval::automaton::{SafetyState, SafetyTranslation, StateContext};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_replay::{
    check_closure, compile_package, settle_model_check, CertificateLocus, CertificateRejection,
    CertificateRequest, CertificateRule, Certification, ClauseIdentity, ClauseRootRefusal,
    ClosureCertificate, ClosureRefusal, DependencyInput, DigestDomain, DigestRecord, Execution,
    IncompleteCause, InconclusiveCause, ItemRefusal, ModelCheckItem, ModelCheckLimit,
    ModelCheckLimits, ModelCheckOutcome, ProductStateRef, ProofBasis, ReachedLimit, ReplayLimits,
    ResultLabel, SettlementBasis, StageLimits, TemporalCertificateCheck, TerminalValue,
    UndecidedCause,
};
use qsl_semantics::check::CheckedTemporalClause;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Cancel, CancelCause, ObjectReference, ScalarLimits, Value};

use crate::counter::{
    clause_text, document, package, package_with_clauses, reference, subject, unit_text, value,
    Documents, Letters, CONTRACT, UNLIMITED, ZERO,
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

/// A TP-1 clause with its letters, declared in a package of its own: the
/// clause is a declaration of that package, whose `body` the unit text ends
/// with.
struct Invariant {
    body: String,
    package: CheckedPackage,
    clause: CheckedTemporalClause,
    letters: Letters,
    identity: ClauseIdentity,
}

impl World {
    fn invariant(&self, formula: &str, atom: fn(i64) -> Option<bool>) -> Invariant {
        let declared = [("C", "", formula)];
        let (package, mut clauses) =
            package_with_clauses(&self.documents.document, &self.body, &declared);
        let body = format!("{}{}", self.body, clause_text("C", "", formula));
        self.declared(body, package, clauses.remove(0), atom)
    }

    /// `clause`, declared by `package` whose unit body is `body`, with
    /// letters over this world's objects.
    fn declared(
        &self,
        body: String,
        package: CheckedPackage,
        clause: CheckedTemporalClause,
        atom: fn(i64) -> Option<bool>,
    ) -> Invariant {
        let identity =
            ClauseIdentity::resolve(&package, &self.documents.packages, &clause, &Cancel::new())
                .expect("the package declares the clause");
        Invariant {
            body,
            package,
            clause,
            letters: Letters {
                bindings: self.objects(),
                atom,
            },
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
            subject: subject(
                package,
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
        self.package_id_of(&self.body)
    }

    /// The `package_id` FR-098's recompile of the unit with `body` gives.
    fn package_id_of(&self, body: &str) -> DigestRecord {
        let unit = unit_text(&self.documents.document, body);
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

    /// The run of `item` over `package`.
    fn run_in(&self, package: &CheckedPackage, item: ModelCheckItem<'_>) -> ModelCheckOutcome {
        check_model(
            ModelCheckRequest {
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
        }
    }

    fn request<'w>(&'w self, world: &'w World, meter: ScalarLimits) -> CertificateRequest<'w> {
        self.request_with(world, self.item(), meter)
    }

    /// A request for `item` over this clause's package.
    fn request_with<'w>(
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

    /// The `package_id` of the recompile of the unit that declares the
    /// clause.
    fn package_id(&self, world: &World) -> DigestRecord {
        world.package_id_of(&self.body)
    }

    /// The translation of the instance bound to `object`.
    fn translation(&self, object: &ObjectReference) -> SafetyTranslation {
        let binding = vec![qsl_eval::automaton::ContextBinding {
            parameter: self.identity.over().expect("an over node"),
            value: Value::Reference(object.clone()),
        }];
        let context =
            StateContext::clause(&self.clause, self.identity.occurrence().clone(), binding);
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

/// The least of `states` by model-state digest octets then key octets.
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
            model_state: world.digest(values),
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
            &invariant.request(&world, UNLIMITED),
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
            &truth.request(&world, UNLIMITED),
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
        ("always holds(6 mod (2 - c.value) >= 0)", |v| {
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
                    model_state: world.digest(&[2]),
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
/// digest of another domain is rejected the same way. Every key refused, the
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

    let own = closure(&reachable, &invariant.translation(&world.objects()[0]));
    let mut foreign = own.clone();
    foreign.states[0].model_state = DigestRecord::mint(
        DigestDomain::Sha256Jcs,
        *own.states[0].model_state.as_bytes(),
    );
    assert_eq!(
        check(&invariant.request(&world, UNLIMITED), &foreign),
        rejected(CertificateRule::ShapeMismatch, foreign.states[0].clone())
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
    let body = format!(
        "{}{}{}",
        world.body,
        clause_text("C", "", formula),
        clause_text("D", "", formula)
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
        ClauseIdentity::resolve(&package, &world.documents.packages, clause, &Cancel::new())
            .expect("the package declares the clause")
    };
    for (identity, node) in [(at(c), node_c), (at(d), node_d)] {
        assert_eq!(
            identity.occurrence().node(),
            WireNodeId::from_digest(*node.identity().as_bytes())
        );
        assert_eq!(identity.occurrence().origin(), node.claim());
        assert_eq!(identity.occurrence().origin().role().as_str(), "claim");
        assert_eq!(
            identity.over(),
            Some(WireNodeId::from_digest(*node.over().as_bytes()))
        );
    }
    assert_ne!(
        at(c).occurrence().origin().ordinal(),
        at(d).occurrence().origin().ordinal()
    );

    let c = world.declared(body.clone(), package, c.clone(), |v| Some(v <= 3));
    let (package, _) = package_with_clauses(
        &world.documents.document,
        &world.body,
        &[("C", "", formula), ("D", "", formula)],
    );
    let d = world.declared(body, package, d.clone(), |v| Some(v <= 3));
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
            subject: request.subject,
            item: foreign,
            max_depth: None,
            limits: ModelCheckLimits::default(),
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
        &invariant.package_id(&world),
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
        &claim.package_id(&world),
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
/// `CertificateRejected{SuccessorMissing, at}`, never `proved`; settled
/// against another `package_id` than the request's package it is
/// `ReplayRefused(stale_dependency)`, never `proved`.
#[trace("TC-525", "FR-127-AC-1")]
#[test]
fn a_proof_settles_certified_only_when_its_certificate_is_accepted() {
    let world = world(CONTRACT, &[("c", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let outcome = invariant.run(&world);
    let request = invariant.request(&world, UNLIMITED);
    let package = invariant.package_id(&world);
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
        &invariant.package_id(&world),
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
        invariant.run(&world),
        ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInstance,
            stats,
        } if stats.product_states == 0
    ));
    assert!(matches!(
        check_closure(
            &invariant.request(&world, UNLIMITED),
            &ClosureCertificate::default(),
            &ModelCheckLimits::default(),
            &Cancel::new()
        ),
        Err(ClosureRefusal::NoInstance)
    ));
}

/// QSpec FR-361-AC-8 (QSpec TC-294) and FR-127: a clause
/// whose empty universe gives it zero instances settles `inconclusive` with
/// the wire cause `no-instance` and basis `unsettled`, never `proved` or
/// `refuted`. The zero instances are made by clearing `Letters.bindings`;
/// deriving them from the universes is QSL-886's.
#[trace("TC-520", "QSpec-FR-361-AC-8")]
#[test]
fn a_clause_with_no_instance_settles_inconclusive_never_proved_or_refuted() {
    let world = world(CONTRACT, &[("c", 0)]);
    let mut invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    invariant.letters.bindings.clear();
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
        &world.package_id(),
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
    let mut certificate = certificate(&invariant.run(&world));
    let padding = certificate.states[0].clone();
    certificate.states.extend(std::iter::repeat_n(
        ProductStateRef {
            automaton_state: qsl_eval::automaton::AutomatonStateKey::imported(b"[".to_vec()),
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
    let package = invariant.package_id(&world);
    let imported =
        |octets: String| qsl_eval::automaton::AutomatonStateKey::imported(octets.into_bytes());
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
                certificate,
                stats: outcome.stats(),
            },
            &ModelCheckLimits::default(),
            &package,
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
