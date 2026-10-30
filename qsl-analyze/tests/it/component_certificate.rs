// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-339 (TC-526, TC-1141) and FR-126's certificate rows (TC-1085, TC-521):
//! the component certificate EN-1 returns with a liveness proof, checked by
//! the core's `check_components`.
//!
//! The certificates are produced by the engine and read back by the core's
//! checker, which recomputes the product graph itself. Every rejection below
//! alters a certificate the engine produced, so each fails only because the
//! checker evaluates the rule the vector names.
//!
//! Not covered: FR-339-AC-3's synthesized visibility SCC, which has no
//! subject that reaches it and is checked over its product graph by
//! `qsl-replay`'s own unit tests, and FR-339-AC-7's protocol vectors, which
//! QSL-1060 owns over this producer.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_model, ModelCheckRequest};
use qsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_replay::{
    check_closure, check_components, settle_model_check, CertificateLocus, CertificateRejection,
    CertificateRequest, CertificateRule, Certification, ClosureCertificate, ClosureRefusal,
    Component, ComponentCertificate, ComponentWitness, Execution, IncompleteCause,
    InconclusiveCause, ModelCheckItem, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome,
    ProductStateRef, ProofBasis, ProofCertificate, ResultLabel, RunStats, SettlementBasis,
    SmtEncodingLimits, TemporalCertificateCheck, TerminalValue,
};
use qsl_semantics::check::{FormulaNodeId, TemporalClauseNode};
use quire_exact::{Cancel, CancelCause, Meter, ScalarLimits};
use serde_json::json;

use crate::counter::{UNLIMITED, ZERO};
use crate::example_units::{config, config_over, decide, reaches_two, EACH};
use crate::liveness::{counter, is_three};
use crate::por_world::{
    field, integer, level, number, object_type, operation, type_ref, writes, Atom, Fixture, Obj,
    Temporal,
};
use crate::units::World;

const ACCEPTED: TemporalCertificateCheck = TemporalCertificateCheck::Accepted;

fn rejected(rule: CertificateRule, at: &ProductStateRef) -> TemporalCertificateCheck {
    TemporalCertificateCheck::Rejected(CertificateRejection {
        rule,
        at: CertificateLocus::ProductState(at.clone()),
    })
}

fn held(
    outcome: Result<ModelCheckOutcome, impl std::fmt::Debug>,
) -> (ComponentCertificate, RunStats) {
    match outcome {
        Ok(ModelCheckOutcome::Holds {
            certificate: ProofCertificate::Component(certificate),
            stats,
        }) => (certificate, stats),
        other => panic!("expected a component certificate: {other:?}"),
    }
}

/// Check `certificate` for `request` under `limits` and `cancel`.
fn check(
    request: &CertificateRequest<'_>,
    certificate: &ComponentCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> TemporalCertificateCheck {
    check_components(request, certificate, limits, cancel).expect("the request is checked")
}

/// The ConfigVersion example unit's request: `world`'s first clause, its
/// atoms read by `letters`.
fn config_request<'a>(
    world: &'a World,
    letters: &'a dyn ModelLetters,
    meter: ScalarLimits,
) -> CertificateRequest<'a> {
    CertificateRequest {
        subject: world.subject(meter),
        item: ModelCheckItem::Clause {
            clause: &world.clauses[0],
            letters,
        },
        smt_encoding: SmtEncodingLimits::default(),
    }
}

/// The weak `each` proof of the example unit, FR-126-AC-1's claim.
fn weak_each_proof() -> (World, ComponentCertificate, RunStats) {
    let world = config(false, EACH);
    let (certificate, stats) = held(decide(&world));
    (world, certificate, stats)
}

fn accepts(world: &World, certificate: &ComponentCertificate) -> TemporalCertificateCheck {
    let letters = reaches_two(world);
    check(
        &config_request(world, &letters, UNLIMITED),
        certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
}

fn shape(certificate: &ComponentCertificate) -> Vec<(usize, ComponentWitness)> {
    certificate
        .components
        .iter()
        .map(|component| (component.states.len(), component.witness.clone()))
        .collect()
}

const MISSING: ComponentWitness = ComponentWitness::MissingAcceptance { set: 0 };

fn weak(constraint: u32) -> ComponentWitness {
    ComponentWitness::UnfairWeak { constraint }
}

/// FR-339-AC-1: the weak `each` proof of FR-126-AC-1 returns a certificate
/// whose accepting components carry `UnfairWeak` naming the `each`
/// constraint of the object the binding does not update: for binding `a`
/// the constraint at 0, which edges inside the components never take, and
/// for binding `b` the constraint at 1. The 9 waiting states of each binding
/// carry `MissingAcceptance`. The checker accepts it.
#[trace("TC-526", "FR-339-AC-1")]
#[test]
fn the_weak_each_proof_is_a_certificate_the_checker_accepts() {
    let (world, certificate, stats) = weak_each_proof();
    assert_eq!(stats.product_states, 30);
    assert_eq!(
        shape(&certificate),
        [
            (9, MISSING),
            (3, weak(0)),
            (3, weak(0)),
            (9, MISSING),
            (3, weak(1)),
            (3, weak(1)),
        ]
    );
    assert_eq!(accepts(&world, &certificate), ACCEPTED);
}

/// FR-339-AC-2: the vectors that alter AC-1's certificate. Naming the other
/// object's constraint on the first accepting component is
/// `WitnessFails` at its first state, since edges inside the component take
/// it; the two accepting components listed before the waiting component
/// that reaches them is `BackwardEdge` at a waiting state; a state in two
/// components is `NotPartition` at it; and a `Trivial` witness over the
/// terminal stutter state of the Counter, whose one state has a self-edge,
/// is `WitnessFails`.
#[trace("TC-526", "FR-339-AC-2")]
#[test]
fn an_altered_certificate_is_rejected_at_the_rule_the_alteration_breaks() {
    let (world, certificate, _) = weak_each_proof();

    let mut wrong = certificate.clone();
    wrong.components[1].witness = weak(1);
    assert_eq!(
        accepts(&world, &wrong),
        rejected(
            CertificateRule::WitnessFails,
            &wrong.components[1].states[0]
        )
    );

    let mut early = certificate.clone();
    let waiting = early.components.remove(0);
    early.components.insert(2, waiting.clone());
    let TemporalCertificateCheck::Rejected(CertificateRejection {
        rule: CertificateRule::BackwardEdge,
        at: CertificateLocus::ProductState(at),
    }) = accepts(&world, &early)
    else {
        panic!("the waiting component reaches the accepting ones");
    };
    assert!(waiting.states.contains(&at));

    let mut twice = certificate.clone();
    let shared = twice.components[1].states[0].clone();
    twice.components[2].states.push(shared.clone());
    assert_eq!(
        accepts(&world, &twice),
        rejected(CertificateRule::NotPartition, &shared)
    );

    let fixture = counter(
        crate::counter::document(false),
        "",
        "always eventually holds(c.value = 3)",
    );
    let (mut terminal, _) = fixture.certified(is_three);
    let last = terminal.components.len() - 1;
    terminal.components[last].witness = ComponentWitness::Trivial;
    let letters = fixture.letters(is_three);
    assert_eq!(
        check(
            &fixture.certificate_request(&letters, UNLIMITED),
            &terminal,
            &ModelCheckLimits::default(),
            &Cancel::new(),
        ),
        rejected(
            CertificateRule::WitnessFails,
            &terminal.components[last].states[0]
        )
    );
}

/// FR-126-AC-13 (TC-1085): the weak `each` claim, whose second phase finds no
/// fair accepting cycle, is `Holds` with a component certificate the
/// checker accepts, and the settlement of that outcome is `proved`,
/// exhaustive and certified: a `Holds` has no other certificate.
#[trace("TC-1085", "FR-126-AC-13")]
#[test]
fn a_liveness_proof_settles_certified_through_its_component_certificate() {
    let world = config(false, EACH);
    let outcome = decide(&world).expect("checked");
    assert!(matches!(
        outcome,
        ModelCheckOutcome::Holds {
            certificate: ProofCertificate::Component(_),
            ..
        }
    ));
    let letters = reaches_two(&world);
    let request = config_request(&world, &letters, UNLIMITED);
    let settle = |outcome: &ModelCheckOutcome| {
        settle_model_check(
            outcome,
            &ModelCheckLimits::default(),
            &request,
            &Cancel::new(),
            |_| panic!("a proof replays nothing"),
        )
    };
    let proved = settle(&outcome);
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

    // A rejected component certificate never settles proved.
    let ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Component(mut tampered),
        stats,
    } = outcome
    else {
        unreachable!("matched above");
    };
    tampered.components[1].witness = weak(1);
    let record = settle(&ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Component(tampered),
        stats,
    });
    assert_eq!(record.label, ResultLabel::Inconclusive);
}

// ---------------------------------------------------------------------------
// The Ring and Fork units
// ---------------------------------------------------------------------------

/// A unit of one `v`-valued (or `phase`-valued) object whose operations step
/// the value `from -> to`, one precondition and one postcondition each.
fn stepper(
    owner: &'static str,
    population: &'static str,
    key: &'static str,
    field_name: &'static str,
    steps: &[(&str, i64, i64)],
    claim: &str,
) -> Fixture {
    let operations = steps
        .iter()
        .map(|(name, _, _)| operation(owner, name, None, &writes(owner, &[field_name])))
        .collect();
    let contracts: Vec<String> = steps
        .iter()
        .flat_map(|(name, from, to)| {
            [
                format!(
                    "pre Pre{name} using v on M::{owner}::{name} {{ self.{field_name} = {from} }}"
                ),
                format!(
                    "post Post{name} using v on M::{owner}::{name} {{ self.{field_name} = {to} }}"
                ),
            ]
        })
        .collect();
    let contracts: Vec<&str> = contracts.iter().map(String::as_str).collect();
    Fixture::new(
        vec![
            level("Level", 2),
            object_type(
                owner,
                vec![field(owner, field_name, &type_ref("Level"))],
                operations,
            ),
        ],
        &[(population, owner)],
        &contracts,
        &[Temporal {
            over: &format!("{key}: M::{owner}"),
            body: claim,
        }],
        &[Obj {
            population,
            key,
            object_type: owner,
            fields: json!({ field_name: integer(0) }),
        }],
        &[(population, &[key])],
    )
}

/// FR-339-AC-6's `Ring`: `a` from 0 to 1, `x` from 0 to 0, `b` from 1 to 2
/// and `c` from 2 to 0, under a weak constraint on `a`.
fn ring() -> Fixture {
    stepper(
        "Ring",
        "rings",
        "r",
        "v",
        &[("a", 0, 1), ("x", 0, 0), ("b", 1, 2), ("c", 2, 0)],
        "fair weak M::Ring::a; always eventually holds(r.v = 1)",
    )
}

/// FR-126-AC-18's `Fork`: `left` from phase 0 to 1, `right` from 0 to 2 and
/// `back` from 2 to 1, with no fairness constraint.
fn fork() -> Fixture {
    stepper(
        "Fork",
        "forks",
        "f",
        "phase",
        &[("left", 0, 1), ("right", 0, 2), ("back", 2, 1)],
        "always eventually holds(f.phase = 1)",
    )
}

fn phase_is_one(
    owner: &'static str,
    population: &'static str,
    field_name: &'static str,
) -> Vec<Atom> {
    let at = format!("{owner}/{field_name}");
    vec![Box::new(move |state, key| {
        number(state, population, key, &at).map(|value| value == 1)
    })]
}

fn ring_atoms() -> Vec<Atom> {
    phase_is_one("Ring", "rings", "v")
}

fn fork_atoms() -> Vec<Atom> {
    phase_is_one("Fork", "forks", "phase")
}

fn proof_of(fixture: &Fixture, atoms: Vec<Atom>) -> (ComponentCertificate, RunStats) {
    held(fixture.run(Some(0), atoms, Default::default()))
}

/// The checker's request for `fixture`'s only clause under `meter`.
fn unit_check(
    fixture: &Fixture,
    atoms: Vec<Atom>,
    certificate: &ComponentCertificate,
    limits: &ModelCheckLimits,
    meter: ScalarLimits,
    cancel: &Cancel,
) -> TemporalCertificateCheck {
    let (letters, _) = fixture.letters(0, atoms);
    let mut subject = fixture.subject();
    subject.simulation.meter = meter;
    check(
        &CertificateRequest {
            subject,
            item: ModelCheckItem::Clause {
                clause: fixture.clause(0),
                letters: &letters,
            },
            smt_encoding: SmtEncodingLimits::default(),
        },
        certificate,
        limits,
        cancel,
    )
}

/// A `Cancel` that cancels itself, `Requested`, at its `polls + 1`th poll.
fn cancelled_after(polls: u32) -> Cancel {
    let seen = Arc::new(AtomicU32::new(0));
    let slot: Arc<OnceLock<Cancel>> = Arc::new(OnceLock::new());
    let (counted, target) = (Arc::clone(&seen), Arc::clone(&slot));
    let cancel = Cancel::observing(move || {
        if counted.fetch_add(1, Ordering::Relaxed) == polls {
            if let Some(cancel) = target.get() {
                cancel.cancel(CancelCause::Requested);
            }
        }
    });
    assert!(slot.set(cancel.clone()).is_ok());
    cancel
}

fn limit(limit: ModelCheckLimit, value: u64) -> TemporalCertificateCheck {
    TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached { limit, value })
}

/// FR-339-AC-6, the `Ring` unit: five product states and nine edges over two
/// automaton states, and components listed `{(0,q0), (1,q0), (2,q0)}`
/// (`MissingAcceptance`), `{(2,q1)}` (`Trivial`), `{(0,q1)}` (`UnfairWeak`
/// naming `a`). Checked with `max_states` 5, `max_transitions` 9 and
/// `max_automaton_states` 2 it is accepted, so the three components charge
/// none of them; one below the closure's count in any of the three stops the
/// closure check naming that limit. A meter budget with no work stops it
/// where FR-338's check of the closure alone stops, and a cancel at the
/// first poll after the closure check stops it `Cancelled`.
#[trace("TC-526", "FR-339-AC-6")]
#[test]
fn component_work_charges_no_budget_and_the_closure_check_stops_first() {
    let fixture = ring();
    let (certificate, stats) = proof_of(&fixture, ring_atoms());
    assert_eq!(
        (
            stats.product_states,
            stats.product_edges,
            stats.automaton_states
        ),
        (5, 9, 2)
    );
    assert_eq!(
        shape(&certificate),
        [(3, MISSING), (1, ComponentWitness::Trivial), (1, weak(0))]
    );
    let unit = |limits: &ModelCheckLimits, meter, cancel: &Cancel| {
        unit_check(&fixture, ring_atoms(), &certificate, limits, meter, cancel)
    };
    let mut exact = ModelCheckLimits::default();
    exact.limits.max_states = 5;
    exact.limits.max_transitions = 9;
    exact.max_automaton_states = 2;
    assert_eq!(unit(&exact, UNLIMITED, &Cancel::new()), ACCEPTED);
    assert_eq!(
        unit(&ModelCheckLimits::default(), UNLIMITED, &Cancel::new()),
        ACCEPTED
    );
    let mut states = exact;
    states.limits.max_states = 4;
    assert_eq!(
        unit(&states, UNLIMITED, &Cancel::new()),
        limit(ModelCheckLimit::MaxStates, 4)
    );
    let mut edges = exact;
    edges.limits.max_transitions = 8;
    assert_eq!(
        unit(&edges, UNLIMITED, &Cancel::new()),
        limit(ModelCheckLimit::MaxTransitions, 8)
    );
    let mut automaton = exact;
    automaton.max_automaton_states = 1;
    assert_eq!(
        unit(&automaton, UNLIMITED, &Cancel::new()),
        limit(ModelCheckLimit::MaxAutomatonStates, 1)
    );

    // The meter stop is the closure check's own: a certificate with no
    // components stops at the same place, before any component rule runs.
    let defaults = ModelCheckLimits::default();
    let closure_only = ComponentCertificate {
        closure: certificate.closure.clone(),
        components: Vec::new(),
    };
    let alone = |meter| {
        unit_check(
            &fixture,
            ring_atoms(),
            &closure_only,
            &defaults,
            meter,
            &Cancel::new(),
        )
    };
    let stopped = alone(ZERO);
    assert!(
        matches!(
            stopped,
            TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
                limit: ModelCheckLimit::EvaluationMeter { .. },
                ..
            })
        ),
        "{stopped:?}"
    );
    assert_eq!(unit(&defaults, ZERO, &Cancel::new()), stopped);
    let TemporalCertificateCheck::Rejected(CertificateRejection {
        rule: CertificateRule::NotPartition,
        ..
    }) = alone(UNLIMITED)
    else {
        panic!("a closure with no components is not a partition");
    };

    // A cancel at the first poll after the closure check's last: the
    // closure check of an unpartitioned closure polls no more, and the
    // checks of the whole certificate poll beyond it.
    let count = |certificate: &ComponentCertificate| {
        let polls = Arc::new(AtomicU32::new(0));
        let counted = Arc::clone(&polls);
        let counting = Cancel::observing(move || {
            counted.fetch_add(1, Ordering::Relaxed);
        });
        unit_check(
            &fixture,
            ring_atoms(),
            certificate,
            &defaults,
            UNLIMITED,
            &counting,
        );
        polls.load(Ordering::Relaxed)
    };
    let components_start = count(&closure_only);
    assert!(count(&certificate) > components_start);
    assert_eq!(
        unit(&defaults, UNLIMITED, &cancelled_after(components_start)),
        TemporalCertificateCheck::Stopped(IncompleteCause::Cancelled {
            source: CancelCause::Requested
        })
    );
}

/// The model-state phase, as the digest of the state it is: `Fork`'s
/// product states are told apart by their model state and automaton key.
fn digest_of(fixture: &Fixture, phase: u8) -> qsl_replay::DigestRecord {
    let system = fixture.system();
    let mut state = fixture.start();
    if phase != 0 {
        let successors = system.successors(&state).expect("expands").successors;
        // `left` is canonically before `right`.
        let pick = usize::from(phase == 2);
        state = successors[pick].1.clone();
    }
    system.state_key(&state).expect("encodes").1
}

/// FR-126-AC-18, the `Fork` unit: the five product states are `(0,q0)`,
/// `(0,q1)`, `(1,q0)`, `(2,q0)`, `(2,q1)`; the five singleton components are
/// listed `(0,q0)`, `(0,q1)`, `(2,q0)`, `(1,q0)`, `(2,q1)`: the edge
/// `(2,q0) -> (1,q0)` lists `(2,q0)` first although `(1,q0)` has the smaller
/// vertex index, and `(2,q1)`, which no edge leaves, is last. Only
/// `(1,q0)`, the terminal stutter, carries `MissingAcceptance`; the others
/// are `Trivial`; the checker accepts it.
#[trace("TC-521")]
#[test]
fn components_are_listed_by_the_least_vertex_whose_predecessors_are_listed() {
    let fixture = fork();
    let (certificate, stats) = proof_of(&fixture, fork_atoms());
    assert_eq!(stats.product_states, 5);
    assert_eq!(
        shape(&certificate),
        [
            (1, ComponentWitness::Trivial),
            (1, ComponentWitness::Trivial),
            (1, ComponentWitness::Trivial),
            (1, MISSING),
            (1, ComponentWitness::Trivial),
        ]
    );
    let listed: Vec<&ProductStateRef> = certificate
        .components
        .iter()
        .map(|component| &component.states[0])
        .collect();
    let phases = [
        digest_of(&fixture, 0),
        digest_of(&fixture, 0),
        digest_of(&fixture, 2),
        digest_of(&fixture, 1),
        digest_of(&fixture, 2),
    ];
    assert_eq!(
        listed.iter().map(|at| at.model_state).collect::<Vec<_>>(),
        phases
    );
    // `q0` reads any letter; `q1` is the accepting state.
    let q0 = &listed[0].automaton_state;
    let q1 = &listed[1].automaton_state;
    assert_ne!(q0, q1);
    assert_eq!(
        listed
            .iter()
            .map(|at| &at.automaton_state)
            .collect::<Vec<_>>(),
        [q0, q1, q0, q0, q1]
    );
    assert_eq!(
        unit_check(
            &fixture,
            fork_atoms(),
            &certificate,
            &ModelCheckLimits::default(),
            UNLIMITED,
            &Cancel::new()
        ),
        ACCEPTED
    );
}

/// FR-126-AC-18, the example unit: the closure's `states` ascend with no
/// duplicate, every component of binding `a` is listed before every
/// component of binding `b`, each component's states ascend, and a second
/// run gives an equal certificate.
#[trace("TC-521")]
#[test]
fn a_certificate_lists_its_states_and_bindings_in_canonical_order() {
    let (world, certificate, _) = weak_each_proof();
    let states = &certificate.closure.states;
    assert!(states.windows(2).all(|pair| pair[0] < pair[1]));
    for component in &certificate.components {
        assert!(component.states.windows(2).all(|pair| pair[0] < pair[1]));
    }
    let binding_of = |component: &Component| match component.witness {
        ComponentWitness::UnfairWeak { constraint } => Some(constraint),
        _ => None,
    };
    let named: Vec<u32> = certificate
        .components
        .iter()
        .filter_map(binding_of)
        .collect();
    assert_eq!(named, [0, 0, 1, 1]);
    let again = decide(&world);
    let (again, _) = held(again);
    assert_eq!(format!("{again:?}"), format!("{certificate:?}"));
    let _: &ClosureCertificate = &certificate.closure;
}

// ---------------------------------------------------------------------------
// FR-339-AC-5: a guarded `on each` activation
// ---------------------------------------------------------------------------

/// The example unit with the claim `formula` under infinite-trace `on each`
/// guarded by `c.versionNumber = guard`, read through the production letters.
fn guarded_world(fairness: &str, formula: &str, guard: &str) -> World {
    use crate::example_units::{CYCLES, OVER};
    let domain = crate::units::Domain::config_version();
    let clause = format!(
        "temporal C using t0 over ({OVER}) clock \"model-steps\" \
         on each (q: Config::ConfigVersion) when ({guard}) {{ {fairness} {formula} }}\n"
    );
    let text = crate::units::unit(&domain, "Config", &format!("{CYCLES}{clause}"));
    World::new(
        domain,
        &text,
        &[
            ("a", crate::units::config_object(0)),
            ("b", crate::units::config_object(0)),
        ],
    )
}

/// `world`'s first clause and its production letters.
struct Production<'p> {
    world: &'p World,
    node: &'p TemporalClauseNode,
    letters: &'p dyn ModelLetters,
}

impl<'p> Production<'p> {
    /// The checker's request for the clause read through `letters`.
    fn request(&self, letters: &'p dyn ModelLetters) -> CertificateRequest<'p> {
        CertificateRequest {
            subject: self.world.unlimited(),
            item: ModelCheckItem::Clause {
                clause: self.node.clause(),
                letters,
            },
            smt_encoding: SmtEncodingLimits::default(),
        }
    }

    fn check(&self, certificate: &ComponentCertificate) -> TemporalCertificateCheck {
        check(
            &self.request(self.letters),
            certificate,
            &ModelCheckLimits::default(),
            &Cancel::new(),
        )
    }
}

fn with_production_letters<T>(world: &World, run: impl for<'p> FnOnce(&Production<'p>) -> T) -> T {
    let system = ModelSystem::new(world.unlimited().simulation).expect("the subject admits");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("the clause has letters");
    run(&Production {
        world,
        node,
        letters: &letters,
    })
}

fn guarded_proof(world: &World) -> (ComponentCertificate, RunStats) {
    with_production_letters(world, |production| {
        let request = production.request(production.letters);
        held(check_model(
            ModelCheckRequest {
                subject: request.subject,
                item: request.item,
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        ))
    })
}

/// Letters that read the activation condition as undefined where the
/// instance's version is 1, and as `inner` otherwise.
struct UndefinedAtOne<'a> {
    world: &'a World,
    inner: &'a dyn ModelLetters,
}

impl ModelLetters for UndefinedAtOne<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        self.inner.atom(atom, instance, state, meter)
    }

    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        if self.world.integer(state, &instance.key, "versionNumber") == 0 {
            return Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                quire_exact::Undefined::DivisionByZero,
            )));
        }
        self.inner.activation(instance, state, meter)
    }

    fn reads(&self, atom: FormulaNodeId) -> crate::units::Reads {
        self.inner.reads(atom)
    }

    fn activation_reads(&self) -> crate::units::Reads {
        self.inner.activation_reads()
    }
}

/// FR-339-AC-5 (TC-1215): the example unit under infinite-trace `on each`
/// with the condition `c.versionNumber = 1` and a weak `each` constraint on
/// `attemptUpdate`. For binding `a` the component of the active states at
/// `(1,0)`, `(1,1)`, `(1,2)` carries `UnfairWeak` naming `upd(a)` and the
/// component of the waiting states `MissingAcceptance{set: 0}`, the
/// activation's acceptance set (symmetrically for `b`); the checker accepts
/// it and FR-127 settles `proved`. With the condition `c.versionNumber = 7`
/// and no fairness, every component with a cycle carries
/// `MissingAcceptance{set: 0}`. The rejected vectors: the waiting
/// component naming `upd(a)` and the active component naming
/// `MissingAcceptance{set: 0}` are `WitnessFails` at their first states; the
/// closure without binding `a`'s active state at `(1,0)` is
/// `SuccessorMissing` at it; and the condition undefined at version 0 is
/// `BadState` at binding `a`'s waiting state at `(0,0)`.
#[trace("TC-1215", "FR-339-AC-5")]
#[test]
fn a_guarded_each_activation_has_its_own_acceptance_set_in_the_certificate() {
    let world = guarded_world(
        "fair weak each Config::ConfigVersion::attemptUpdate;",
        "eventually holds(c.versionNumber = 2)",
        "c.versionNumber = 1",
    );
    let (certificate, stats) = guarded_proof(&world);
    assert_eq!(
        shape(&certificate),
        [(9, MISSING), (3, weak(0)), (9, MISSING), (3, weak(1))]
    );
    let accepted = |certificate: &ComponentCertificate| {
        with_production_letters(&world, |production| production.check(certificate))
    };
    assert_eq!(accepted(&certificate), ACCEPTED);
    let record = with_production_letters(&world, |production| {
        settle_model_check(
            &ModelCheckOutcome::Holds {
                certificate: ProofCertificate::Component(certificate.clone()),
                stats,
            },
            &ModelCheckLimits::default(),
            &production.request(production.letters),
            &Cancel::new(),
            |_| panic!("a proof replays nothing"),
        )
    });
    assert_eq!(record.label, ResultLabel::Proved);

    let mut waiting_unfair = certificate.clone();
    waiting_unfair.components[0].witness = weak(0);
    assert_eq!(
        accepted(&waiting_unfair),
        rejected(
            CertificateRule::WitnessFails,
            &waiting_unfair.components[0].states[0]
        )
    );
    let mut active_accepting = certificate.clone();
    active_accepting.components[1].witness = MISSING;
    assert_eq!(
        accepted(&active_accepting),
        rejected(
            CertificateRule::WitnessFails,
            &active_accepting.components[1].states[0]
        )
    );

    // Binding `a`'s active state at `(1,0)`: its model state is the
    // successor of the start that updates `a`.
    let system = ModelSystem::new(world.unlimited().simulation).expect("admits");
    let start = system.initial()[0].clone();
    let updated = system
        .successors(&start)
        .expect("expands")
        .successors
        .into_iter()
        .find(|(_, state)| {
            world.integer(state, "a", "versionNumber") == 1
                && world.integer(state, "b", "versionNumber") == 0
        })
        .expect("a's update is a step")
        .1;
    let one_zero = system.state_key(&updated).expect("encodes").1;
    let removed = certificate.components[1]
        .states
        .iter()
        .find(|at| at.model_state == one_zero)
        .expect("binding a's active component holds (1,0)")
        .clone();
    let mut gap = certificate.clone();
    gap.closure.states.retain(|at| *at != removed);
    assert_eq!(
        accepted(&gap),
        rejected(CertificateRule::SuccessorMissing, &removed)
    );

    let initial = system.state_key(&start).expect("encodes").1;
    let waiting = certificate.components[0]
        .states
        .iter()
        .find(|at| at.model_state == initial)
        .expect("binding a's waiting component holds (0,0)")
        .clone();
    let undefined = with_production_letters(&world, |production| {
        let wrapped = UndefinedAtOne {
            world: &world,
            inner: production.letters,
        };
        check(
            &production.request(&wrapped),
            &certificate,
            &ModelCheckLimits::default(),
            &Cancel::new(),
        )
    });
    assert_eq!(undefined, rejected(CertificateRule::BadState, &waiting));

    let unmet = guarded_world(
        "",
        "eventually holds(c.versionNumber = 2)",
        "c.versionNumber = 7",
    );
    let (never, _) = guarded_proof(&unmet);
    assert!(never
        .components
        .iter()
        .all(|component| component.witness == ComponentWitness::Trivial
            || component.witness == MISSING));
    assert!(never
        .components
        .iter()
        .any(|component| component.witness == MISSING));
    let accepts_never = with_production_letters(&unmet, |production| production.check(&never));
    assert_eq!(accepts_never, ACCEPTED);
}

// ---------------------------------------------------------------------------
// A closure is not a liveness proof
// ---------------------------------------------------------------------------

/// FR-339-AC-8: a TP-4 clause is proved only by a component certificate. The
/// `Counter`'s `always eventually holds(c.value = 3)` with letters that never
/// hold is `Violated`; a closure grown from the checker's own `InitialMissing`
/// and `SuccessorMissing` rejections of that claim covers every product
/// state, and offered as the certificate of a `Holds` it is refused by
/// `check_closure`, so it settles `failed` and never `proved`. The component
/// certificate of the claim under its own letters settles `proved`,
/// `certified`.
#[trace("TC-526", "FR-339-AC-8")]
#[test]
fn a_closure_certificate_never_proves_a_liveness_claim() {
    fn never(_: i64) -> Option<bool> {
        Some(false)
    }
    let fixture = counter(
        crate::counter::document(false),
        "",
        "always eventually holds(c.value = 3)",
    );
    let (honest, stats) = fixture.certified(is_three);
    let violated = fixture.decide(never).expect("the clause is decided");
    assert!(matches!(violated, ModelCheckOutcome::Violated { .. }));

    let letters = fixture.letters(never);
    let request = fixture.certificate_request(&letters, UNLIMITED);
    let mut forged = ComponentCertificate {
        closure: ClosureCertificate { states: Vec::new() },
        components: Vec::new(),
    };
    loop {
        let verdict = check(
            &request,
            &forged,
            &ModelCheckLimits::default(),
            &Cancel::new(),
        );
        let TemporalCertificateCheck::Rejected(CertificateRejection {
            rule: CertificateRule::InitialMissing | CertificateRule::SuccessorMissing,
            at: CertificateLocus::ProductState(missing),
        }) = verdict
        else {
            break;
        };
        forged.closure.states.push(missing);
        forged.closure.states.sort();
    }
    assert!(forged.closure.states.len() >= 4);

    let settle = |certificate: ProofCertificate, request: &CertificateRequest<'_>| {
        settle_model_check(
            &ModelCheckOutcome::Holds { certificate, stats },
            &ModelCheckLimits::default(),
            request,
            &Cancel::new(),
            |_| panic!("a proof replays nothing"),
        )
    };
    let record = settle(ProofCertificate::Closure(forged.closure.clone()), &request);
    assert_eq!(record.label, ResultLabel::Failed);
    assert!(matches!(
        check_closure(
            &request,
            &forged.closure,
            &ModelCheckLimits::default(),
            &Cancel::new()
        ),
        Err(ClosureRefusal::LivenessNeedsComponents)
    ));

    let honest_letters = fixture.letters(is_three);
    let record = settle(
        ProofCertificate::Component(honest),
        &fixture.certificate_request(&honest_letters, UNLIMITED),
    );
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );
}

/// FR-339: components offered for an item that is not a TP-4 clause are
/// refused `NotLiveness`, never accepted and never rejected.
#[trace("TC-526", "FR-339-AC-8")]
#[test]
fn components_offered_for_a_safety_clause_are_refused() {
    let fixture = counter(
        crate::counter::document(false),
        "",
        "always holds(c.value <= 3)",
    );
    let letters = fixture.letters(is_three);
    let (liveness, _) = counter(
        crate::counter::document(false),
        "",
        "always eventually holds(c.value = 3)",
    )
    .certified(is_three);
    assert!(matches!(
        check_components(
            &fixture.certificate_request(&letters, UNLIMITED),
            &liveness,
            &ModelCheckLimits::default(),
            &Cancel::new(),
        ),
        Err(ClosureRefusal::NotLiveness)
    ));
}
