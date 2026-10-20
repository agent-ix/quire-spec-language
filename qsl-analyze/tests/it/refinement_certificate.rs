// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-149 and FR-144-AC-7 (TC-556): EN-1 hands over the simulation relation
//! of a holding refinement safety half, the core's checker accepts a true one
//! and rejects each tampered or false one, and settlement labels the proof
//! `Certified` only after the checker accepts.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::ClauseOutcome;
use qsl_eval::simulation::model::{operation_identity_text, ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_replay::{
    check_simulation_certificate, check_step, map_state, open_systems, settle_refinement_safety,
    CertificateLocus, CertificateRule, Certification, CertifiedEdge, CertifiedPosition,
    ConcreteStep, DigestRecord, FailedStep, HistoryValues, IncompleteCause, InconclusiveCause,
    ModelCheckLimit, ModelCheckLimits, ProofBasis, RefinementFailure, RefinementPosition,
    SafetyOutcome, SettlementBasis, SimulationCertificate, SimulationCertificateRequest,
    SimulationRefusal, StepRejection, StepVerdict, Taken, TemporalCertificateCheck, TerminalValue,
};
use qsl_replay::{EvalFailure, MappingRowRef, MappingUndetermined, Undetermined, UndeterminedStep};
use qsl_semantics::model::state::ModelState;
use quire_exact::{Cancel, CancelCause, ChargePoint, Incomplete, Integer, LimitKind, ScalarLimits};

use crate::refinement_fixture::{
    cas_refines_counter, cas_state, key, lost_update_refinement, mutated, Variant, World, CAS,
    SPEC, UNLIMITED,
};

fn request(world: &World) -> SimulationCertificateRequest<'_> {
    SimulationCertificateRequest {
        refinement: world.refinement(),
        concrete: world.concrete_request(UNLIMITED),
        abstract_package: world.package(),
        abstract_initial: vec![world.abstract_initial()],
    }
}

/// `CasRefinesCounter` without its `ensure` row: the item FR-149-AC-1 names.
fn safety_only() -> World {
    World::new(&mutated(
        &cas_refines_counter(),
        "ensure fair weak each Spec::Counter::inc;",
        "",
    ))
}

/// The outcome of the safety run of `world`.
fn run(world: &World) -> SafetyOutcome {
    check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked")
    .safety
}

/// The certificate of a holding run, and the product states it stored.
fn certificate(world: &World) -> (SimulationCertificate, u64) {
    let SafetyOutcome::Holds { certificate, stats } = run(world) else {
        panic!("the safety half holds");
    };
    (*certificate, stats.product_states)
}

fn check(world: &World, certificate: &SimulationCertificate) -> TemporalCertificateCheck {
    check_with(world, certificate, &ModelCheckLimits::default())
}

fn check_with(
    world: &World,
    certificate: &SimulationCertificate,
    limits: &ModelCheckLimits,
) -> TemporalCertificateCheck {
    check_simulation_certificate(&request(world), certificate, limits, &Cancel::new())
        .expect("the certificate is checked")
}

fn rejected(check: TemporalCertificateCheck) -> (CertificateRule, CertificateLocus) {
    let TemporalCertificateCheck::Rejected(rejection) = check else {
        panic!("the certificate is rejected: {check:?}");
    };
    (rejection.rule, rejection.at)
}

/// The digest of the concrete state `state` in `system`.
fn digest(system: &ModelSystem<'_>, state: &ModelState) -> DigestRecord {
    system.state_key_of(state).expect("a key").1
}

/// The index of the certificate position of the concrete state
/// `(value, tmpA, busyA, tmpB, busyB)`.
fn position_of(world: &World, certificate: &SimulationCertificate, at: [i64; 5]) -> u64 {
    let concrete = world.concrete_system(UNLIMITED);
    let wanted = digest(
        &concrete,
        &cas_state(at[0], at[1], at[2] == 1, at[3], at[4] == 1),
    );
    let found = certificate
        .positions
        .iter()
        .position(|position| position.concrete == wanted)
        .expect("the state is a position");
    u64::try_from(found).expect("an index")
}

/// The index of the edge of `operation` out of position `from`.
fn edge_of(certificate: &SimulationCertificate, from: u64, operation: &str) -> usize {
    let text = operation_identity_text(&key(CAS, &format!("Counter/{operation}"))).expect("a text");
    certificate.successors[usize::try_from(from).expect("an index")]
        .iter()
        .position(|edge| {
            edge.transition
                .as_ref()
                .is_some_and(|transition| transition.operation() == text)
        })
        .expect("the position has the edge")
}

fn edge_mut(certificate: &mut SimulationCertificate, from: u64, edge: usize) -> &mut CertifiedEdge {
    &mut certificate.successors[usize::try_from(from).expect("an index")][edge]
}

fn step(position: u64, edge: Option<u64>) -> CertificateLocus {
    CertificateLocus::SimulationStep {
        position,
        edge,
        verdict: None,
    }
}

/// FR-149-AC-1 (TC-556 step 1): the certificate holds one position per
/// stored product state, and the checker returns `Accepted`.
#[trace("TC-556", "FR-149-AC-1")]
#[test]
fn the_engines_certificate_is_accepted() {
    let world = safety_only();
    let (certificate, stored) = certificate(&world);
    assert_eq!(
        u64::try_from(certificate.positions.len()).expect("a count"),
        stored
    );
    assert_eq!(certificate.successors.len(), certificate.positions.len());
    assert_eq!(
        check(&world, &certificate),
        TemporalCertificateCheck::Accepted
    );
}

/// FR-149-AC-2 (TC-556 step 2): the `Taken` of the `commitA` edge from
/// `(0, 0, t, 0, f)` changed to `Stutter` is rejected `PostDiffers` at that
/// position and edge.
#[trace("TC-556", "FR-149-AC-2")]
#[test]
fn a_changed_taken_is_rejected() {
    let world = safety_only();
    let (mut certificate, _) = certificate(&world);
    let from = position_of(&world, &certificate, [0, 0, 1, 0, 0]);
    let edge = edge_of(&certificate, from, "commitA");
    edge_mut(&mut certificate, from, edge).taken = Taken::Stutter;
    assert_eq!(
        rejected(check(&world, &certificate)),
        (
            CertificateRule::PostDiffers,
            step(from, Some(u64::try_from(edge).expect("an index")))
        )
    );
}

/// FR-149-AC-2 (TC-556 step 2): that edge's post index pointing at the
/// initial position is rejected `PostDiffers`.
#[trace("TC-556", "FR-149-AC-2")]
#[test]
fn a_post_index_at_the_initial_position_is_rejected() {
    let world = safety_only();
    let (mut certificate, _) = certificate(&world);
    let from = position_of(&world, &certificate, [0, 0, 1, 0, 0]);
    let edge = edge_of(&certificate, from, "commitA");
    let initial = certificate.initial[0];
    edge_mut(&mut certificate, from, edge).post = initial;
    assert_eq!(
        rejected(check(&world, &certificate)),
        (
            CertificateRule::PostDiffers,
            step(from, Some(u64::try_from(edge).expect("an index")))
        )
    );
}

/// FR-149-AC-2 (TC-556 step 2): one listed edge removed is rejected
/// `SuccessorsDiffer` at that position.
#[trace("TC-556", "FR-149-AC-2")]
#[test]
fn a_removed_edge_is_rejected() {
    let world = safety_only();
    let (mut certificate, _) = certificate(&world);
    let from = position_of(&world, &certificate, [0, 0, 1, 0, 0]);
    let edge = edge_of(&certificate, from, "commitA");
    certificate.successors[usize::try_from(from).expect("an index")].remove(edge);
    assert_eq!(
        rejected(check(&world, &certificate)),
        (CertificateRule::SuccessorsDiffer, step(from, None))
    );
}

/// FR-149-AC-2 (TC-556 step 2): an edge's transition changed, an edge
/// appended, and the edges of a position reordered are each rejected
/// `SuccessorsDiffer`.
#[trace("TC-556", "FR-149-AC-2")]
#[test]
fn a_changed_added_or_reordered_edge_is_rejected() {
    let world = safety_only();
    let (certificate, _) = certificate(&world);
    let from = position_of(&world, &certificate, [0, 0, 1, 0, 0]);
    let at = usize::try_from(from).expect("an index");
    assert!(certificate.successors[at].len() > 1);

    let mut changed = certificate.clone();
    changed.successors[at][0].transition = None;
    let mut added = certificate.clone();
    let extra = added.successors[at][0].clone();
    added.successors[at].push(extra);
    let mut reordered = certificate.clone();
    reordered.successors[at].swap(0, 1);
    for tampered in [changed, added, reordered] {
        assert_eq!(
            rejected(check(&world, &tampered)),
            (CertificateRule::SuccessorsDiffer, step(from, None))
        );
    }
}

/// FR-149-AC-2 (TC-556 step 2): every field of a certificate, corrupted, is
/// rejected: the initial list, a position's concrete digest and abstract
/// digests, a post index, and a `Taken`.
#[trace("TC-556", "FR-149-AC-2")]
#[test]
fn each_corrupted_field_is_rejected() {
    let world = safety_only();
    let (certificate, _) = certificate(&world);
    let other = certificate.positions[1].clone();

    let mut initial = certificate.clone();
    initial.initial[0] = 1;
    assert_eq!(
        rejected(check(&world, &initial)),
        (CertificateRule::Malformed, step(0, None))
    );
    let mut beyond = certificate.clone();
    beyond.initial[0] = u64::MAX;
    assert_eq!(
        rejected(check(&world, &beyond)),
        (CertificateRule::Malformed, step(0, None))
    );

    // The initial position's concrete digest, its abstract digests.
    let mut concrete = certificate.clone();
    concrete.positions[0].concrete = other.concrete;
    assert_eq!(
        rejected(check(&world, &concrete)).0,
        CertificateRule::InitialMismatch
    );
    let mut mapped = certificate.clone();
    mapped.positions[0].abstract_states.clear();
    assert_eq!(
        rejected(check(&world, &mapped)).0,
        CertificateRule::InitialMismatch
    );

    // A later position's digests are met as the post of an edge.
    let mut later = certificate.clone();
    later.positions[1].concrete = certificate.positions[0].concrete;
    assert_eq!(
        rejected(check(&world, &later)).0,
        CertificateRule::PostDiffers
    );
    let mut abstract_later = certificate.clone();
    abstract_later.positions[1].abstract_states = vec![];
    assert_eq!(
        rejected(check(&world, &abstract_later)).0,
        CertificateRule::PostDiffers
    );

    // A post index out of range, and one at another position, on the last
    // position's first edge so that no position loses its only way in.
    let last = certificate.positions.len() - 1;
    let at_last = step(u64::try_from(last).unwrap(), Some(0));
    let mut out_of_range = certificate.clone();
    out_of_range.successors[last][0].post = u64::MAX;
    assert_eq!(
        rejected(check(&world, &out_of_range)),
        (CertificateRule::PostDiffers, at_last.clone())
    );
    let mut elsewhere = certificate.clone();
    let post = elsewhere.successors[last][0].post;
    elsewhere.successors[last][0].post = u64::from(post == 0);
    assert_eq!(
        rejected(check(&world, &elsewhere)),
        (CertificateRule::PostDiffers, at_last)
    );

    // Every edge's `Taken`, changed in turn, is rejected at that edge.
    for (at, edges) in certificate.successors.iter().enumerate() {
        for (number, edge) in edges.iter().enumerate() {
            let mut tampered = certificate.clone();
            tampered.successors[at][number].taken = match edge.taken {
                Taken::Stutter => Taken::Abstract(Vec::new()),
                Taken::Abstract(_) => Taken::Stutter,
            };
            let (rule, _) = rejected(check(&world, &tampered));
            assert_eq!(rule, CertificateRule::PostDiffers);
        }
    }
}

/// FR-149 (TC-556): a certificate whose lists do not match, or that lists a
/// position nothing reaches, is rejected `Malformed`.
#[trace("TC-556")]
#[test]
fn a_malformed_certificate_is_rejected() {
    let world = safety_only();
    let (certificate, _) = certificate(&world);
    let count = u64::try_from(certificate.positions.len()).expect("a count");

    let mut short = certificate.clone();
    short.successors.pop();
    assert_eq!(
        rejected(check(&world, &short)),
        (CertificateRule::Malformed, step(count - 1, None))
    );
    let mut doubled = certificate.clone();
    doubled.initial.push(0);
    assert_eq!(
        rejected(check(&world, &doubled)).0,
        CertificateRule::Malformed
    );
    let mut stray = certificate.clone();
    stray.positions.push(CertifiedPosition {
        concrete: certificate.positions[0].concrete,
        abstract_states: Vec::new(),
    });
    stray.successors.push(Vec::new());
    assert_eq!(
        rejected(check(&world, &stray)),
        (CertificateRule::Malformed, step(count, None))
    );
}

/// The certificate of every concrete state the model reaches, built by
/// listing them in breadth-first order with the mapped state of each and the
/// `Taken` that `check_step` gives, as the lost-update model has no engine
/// certificate because its safety half does not hold.
fn listed(world: &World) -> SimulationCertificate {
    let systems = open_systems(
        world.refinement(),
        &world.concrete_request(UNLIMITED),
        world.package(),
        vec![world.abstract_initial()],
    )
    .expect("the subjects open");
    let concrete = &systems.concrete;
    let abstract_system = &systems.abstract_system;
    let position = |state: &ModelState| RefinementPosition {
        concrete: state.clone(),
        history: HistoryValues::default(),
        abstract_states: vec![
            map_state(
                world.refinement(),
                concrete,
                abstract_system,
                state,
                &HistoryValues::default(),
            )
            .expect("the state maps")
            .visible,
        ],
    };
    let certified = |position: &RefinementPosition| CertifiedPosition {
        concrete: digest(concrete, &position.concrete),
        abstract_states: position
            .abstract_states
            .iter()
            .map(|state| abstract_system.state_key_of(state).expect("a key").1)
            .collect(),
    };
    let mut index: BTreeMap<DigestRecord, u64> = BTreeMap::new();
    let mut states = Vec::new();
    let mut positions = Vec::new();
    let mut add =
        |state: &ModelSimState, states: &mut Vec<_>, positions: &mut Vec<CertifiedPosition>| {
            let at = position(state.state());
            let certified = certified(&at);
            *index.entry(certified.concrete).or_insert_with(|| {
                states.push((state.clone(), at));
                positions.push(certified);
                u64::try_from(positions.len() - 1).expect("an index")
            })
        };
    let initial: Vec<u64> = concrete
        .initial_states()
        .iter()
        .map(|state| add(state, &mut states, &mut positions))
        .collect();
    let mut successors = Vec::new();
    let mut at = 0;
    while at < states.len() {
        let (state, pre) = states[at].clone();
        let mut edges = concrete
            .successors(&state)
            .expect("the state expands")
            .successors
            .into_iter()
            .map(|(transition, post)| {
                let order = concrete.transition_key(&transition).expect("a key");
                let key = concrete.state_key(&post).expect("a key").0;
                (order, key, Some(transition), post)
            })
            .collect::<Vec<_>>();
        edges.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        if edges.is_empty() {
            edges.push((Vec::new(), Vec::new(), None, state.clone()));
        }
        let mut row = Vec::new();
        for (_, _, transition, post) in edges {
            let taken = match check_step(
                world.refinement(),
                concrete,
                abstract_system,
                &pre,
                &ConcreteStep {
                    position: 0,
                    transition: transition.as_ref(),
                    post: &post,
                },
            )
            .verdict
            {
                StepVerdict::Passes { taken, .. } => taken,
                _ => Taken::Stutter,
            };
            row.push(CertifiedEdge {
                transition,
                post: add(&post, &mut states, &mut positions),
                taken,
            });
        }
        successors.push(row);
        at += 1;
    }
    SimulationCertificate {
        refinement: world.refinement().name().to_owned(),
        positions,
        initial,
        successors,
    }
}

/// FR-149-AC-3 (TC-556 step 3): the lost-update model's listed states are
/// rejected `StepFails` at the `commitB` edge from `(1, 0, f, 0, t)`, with
/// the verdict `AbstractStepRejected{inc(c), Postcondition}` in the locus.
#[trace("TC-556", "FR-149-AC-3")]
#[test]
fn a_false_relation_is_rejected_with_its_step_verdict() {
    let world = World::with(Variant::LOST_UPDATE, &lost_update_refinement());
    let certificate = listed(&world);
    let (rule, at) = rejected(check(&world, &certificate));
    assert_eq!(rule, CertificateRule::StepFails);
    let from = position_of(&world, &certificate, [1, 0, 0, 0, 1]);
    let edge = edge_of(&certificate, from, "commitB");
    let CertificateLocus::SimulationStep {
        position,
        edge: at_edge,
        verdict: Some(verdict),
    } = at
    else {
        panic!("a step locus with a verdict: {at:?}");
    };
    let FailedStep::Fails(RefinementFailure::AbstractStepRejected {
        transition, cause, ..
    }) = *verdict
    else {
        panic!("a rejected abstract step: {verdict:?}");
    };
    assert_eq!((position, at_edge), (from, u64::try_from(edge).ok()));
    assert_eq!(
        transition.operation(),
        operation_identity_text(&key(SPEC, "Counter/inc")).expect("a text")
    );
    assert_eq!(cause, StepRejection::Postcondition);
}

/// FR-149-AC-4 (TC-556 step 4): the certificate checked with
/// `max_transitions` 1 stops at that limit.
#[trace("TC-556", "FR-149-AC-4")]
#[test]
fn a_transition_limit_stops_the_check() {
    let world = safety_only();
    let (certificate, _) = certificate(&world);
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_transitions = 1;
    assert_eq!(
        check_with(&world, &certificate, &limits),
        TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxTransitions,
            value: 1
        })
    );
    let mut states = ModelCheckLimits::default();
    states.limits.max_states = 1;
    assert_eq!(
        check_with(&world, &certificate, &states),
        TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 1
        })
    );
}

/// FR-149-AC-4 (TC-556 step 4): a certificate naming another refinement is
/// refused `stale_dependency`/`content-mismatch`, and checking the same
/// certificate twice gives equal results.
#[trace("TC-556", "FR-149-AC-4")]
#[test]
fn another_refinement_is_refused_and_a_check_repeats() {
    let world = safety_only();
    let (mut certificate, _) = certificate(&world);
    assert_eq!(check(&world, &certificate), check(&world, &certificate));
    certificate.refinement = "OtherRefinement".to_owned();
    let refusal = check_simulation_certificate(
        &request(&world),
        &certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
    .expect_err("the certificate names another refinement");
    assert!(matches!(refusal, SimulationRefusal::Stale { .. }));
    assert!(refusal
        .to_string()
        .starts_with("stale_dependency/content-mismatch"));
}

/// FR-144-AC-7 (TC-556 step 5): `CasRefinesCounter` without its `ensure` row
/// settles its safety half `proved`, `closed-scope`, `Certified`, after the
/// checker accepts the certificate.
#[trace("TC-556", "FR-144-AC-7")]
#[test]
fn an_accepted_certificate_settles_certified() {
    let world = safety_only();
    let outcome = run(&world);
    let record = settle_refinement_safety(
        &outcome,
        &ModelCheckLimits::default(),
        &request(&world),
        &Cancel::new(),
        |_| panic!("a holding half has no counterexample"),
    );
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified
        }
    );
    assert_eq!(record.basis, SettlementBasis::ClosedScope);
    assert_eq!(record.method, "explicit-state");
}

/// FR-144-AC-7 (TC-556 step 5): with the `Taken` for the `commitA` edge from
/// `(0, 0, t, 0, f)` changed to `Stutter`, the half settles `inconclusive`,
/// `CertificateRejected{PostDiffers, ..}`, never `proved`.
#[trace("TC-556", "FR-144-AC-7")]
#[test]
fn a_rejected_certificate_settles_inconclusive() {
    let world = safety_only();
    let SafetyOutcome::Holds {
        mut certificate,
        stats,
    } = run(&world)
    else {
        panic!("the safety half holds");
    };
    let from = position_of(&world, &certificate, [0, 0, 1, 0, 0]);
    let edge = edge_of(&certificate, from, "commitA");
    edge_mut(&mut certificate, from, edge).taken = Taken::Stutter;
    let record = settle_refinement_safety(
        &SafetyOutcome::Holds { certificate, stats },
        &ModelCheckLimits::default(),
        &request(&world),
        &Cancel::new(),
        |_| panic!("a holding half has no counterexample"),
    );
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::PostDiffers,
            at: step(from, Some(u64::try_from(edge).expect("an index")))
        })
    );
    assert_eq!(record.basis, SettlementBasis::Unsettled);
}

/// FR-144 Settlement (TC-556): a certificate for another refinement settles
/// `ReplayRefused(stale_dependency)`, and a check that a limit stops settles
/// V-7 naming the limit.
#[trace("TC-556")]
#[test]
fn a_stale_or_stopped_check_settles_without_a_proof() {
    let world = safety_only();
    let SafetyOutcome::Holds {
        mut certificate,
        stats,
    } = run(&world)
    else {
        panic!("the safety half holds");
    };
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_transitions = 1;
    let stopped = settle_refinement_safety(
        &SafetyOutcome::Holds {
            certificate: certificate.clone(),
            stats,
        },
        &limits,
        &request(&world),
        &Cancel::new(),
        |_| panic!("a holding half has no counterexample"),
    );
    assert_eq!(
        stopped.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxTransitions,
            value: 1
        })
    );
    certificate.refinement = "OtherRefinement".to_owned();
    let stale = settle_refinement_safety(
        &SafetyOutcome::Holds { certificate, stats },
        &ModelCheckLimits::default(),
        &request(&world),
        &Cancel::new(),
        |_| panic!("a holding half has no counterexample"),
    );
    assert_eq!(
        stale.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
            qsl_replay::Code::StaleDependency
        ))
    );
}

/// `CasRefinesCounter` without its `ensure` row, with `peek` and `beginB`
/// never enabled: a chain of six states ending in a terminal state at depth
/// 5, whose stutter step ADR-018 SM-4 adds.
fn terminal_world() -> World {
    let text = mutated(
        &cas_refines_counter(),
        "ensure fair weak each Spec::Counter::inc;",
        "",
    );
    World::new(&format!(
        "pre PeekNever using v on Impl::Counter::peek {{ false }}\n\
         pre BeginBNever using v on Impl::Counter::beginB {{ false }}\n{text}"
    ))
}

fn run_to(world: &World, max_depth: Option<usize>) -> SafetyOutcome {
    check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            max_depth,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked")
    .safety
}

/// FR-149 Behavior "Production" and "Successors" (TC-556): the certificate
/// of a model with a terminal state lists the stutter step of ADR-018 SM-4
/// and is accepted; with that edge removed it is rejected `SuccessorsDiffer`.
#[trace("TC-556", "FR-149-AC-6")]
#[test]
fn a_terminal_state_lists_its_stutter_step() {
    let world = terminal_world();
    let SafetyOutcome::Holds { certificate, .. } = run_to(&world, None) else {
        panic!("the safety half holds");
    };
    assert_eq!(
        check(&world, &certificate),
        TemporalCertificateCheck::Accepted
    );
    let terminal = certificate
        .successors
        .iter()
        .position(|edges| edges.iter().all(|edge| edge.transition.is_none()) && !edges.is_empty())
        .expect("a terminal state lists a stutter step");
    let mut removed = certificate.clone();
    removed.successors[terminal].clear();
    assert_eq!(
        rejected(check(&world, &removed)),
        (
            CertificateRule::SuccessorsDiffer,
            step(u64::try_from(terminal).expect("an index"), None)
        )
    );
}

/// FR-142 horizon and FR-149 Production (TC-556): a horizon at the depth of
/// a terminal state still gives a certificate that its checker accepts, at
/// every horizon at which the run holds.
#[trace("TC-556")]
#[test]
fn a_horizon_at_a_terminal_state_gives_an_accepted_certificate() {
    let world = terminal_world();
    let mut held = Vec::new();
    for depth in 0..=8 {
        if let SafetyOutcome::Holds { certificate, .. } = run_to(&world, Some(depth)) {
            assert_eq!(
                check(&world, &certificate),
                TemporalCertificateCheck::Accepted,
                "max_depth {depth}"
            );
            held.push(depth);
        }
    }
    assert!(
        held.contains(&5),
        "the terminal state is at depth 5: {held:?}"
    );
}

/// FR-149 Behavior "Successors" (TC-556): a position whose expansion leaves
/// a contract undetermined has unknown successors, so it is rejected
/// `BadState`. Here `peek`'s precondition is undefined at `value` 2, and
/// `successors()` drops the transition, so only `BadState` stops a
/// certificate that leaves it out.
#[trace("TC-556")]
#[test]
fn an_undetermined_contract_is_a_bad_state() {
    let (certificate, _) = certificate(&safety_only());
    let text = mutated(
        &cas_refines_counter(),
        "ensure fair weak each Spec::Counter::inc;",
        "",
    );
    let guarded = World::new(&format!(
        "pre PeekDefined using v on Impl::Counter::peek {{ 6 mod (2 - self.value) >= 0 }}\n{text}"
    ));
    assert_eq!(
        rejected(check(&guarded, &certificate)).0,
        CertificateRule::BadState
    );
}

/// FR-149 Behavior "Limits" (TC-556): a cancelled check is `Stopped` with
/// the cancellation, never `Accepted` or `Rejected`.
#[trace("TC-556", "FR-149-AC-10")]
#[test]
fn a_cancelled_check_stops() {
    let world = safety_only();
    let (certificate, _) = certificate(&world);
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let check = check_simulation_certificate(
        &request(&world),
        &certificate,
        &ModelCheckLimits::default(),
        &cancel,
    )
    .expect("the certificate is checked");
    assert_eq!(
        check,
        TemporalCertificateCheck::Stopped(IncompleteCause::Cancelled {
            source: CancelCause::Requested
        })
    );
}

fn request_under(world: &World, work: u64) -> SimulationCertificateRequest<'_> {
    SimulationCertificateRequest {
        concrete: world.concrete_request(ScalarLimits {
            work_units: work,
            ..UNLIMITED
        }),
        ..request(world)
    }
}

fn check_under(
    world: &World,
    certificate: &SimulationCertificate,
    work: u64,
) -> TemporalCertificateCheck {
    check_simulation_certificate(
        &request_under(world, work),
        certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the certificate is checked")
}

/// `CasRefinesCounter` without its `ensure` row, with `extra` contract
/// clauses before its refinement.
fn with_contracts(extra: &str) -> World {
    let text = mutated(
        &cas_refines_counter(),
        "ensure fair weak each Spec::Counter::inc;",
        "",
    );
    World::new(&format!("{extra}\n{text}"))
}

/// A `beginA` precondition that is true and costs about 150 conjuncts.
fn heavy_contract() -> String {
    let conjuncts = vec!["self.value >= 0"; 150].join(" and ");
    format!("pre HeavyBegin using v on Impl::Counter::beginA {{ {conjuncts} }}")
}

/// The least work-unit meter under which the plain certificate is accepted:
/// `B`, big enough for every mapping row and ordinary contract.
fn least_meter(world: &World, certificate: &SimulationCertificate) -> u64 {
    let (mut low, mut high) = (1, 1 << 20);
    while low < high {
        let middle = low + (high - low) / 2;
        if check_under(world, certificate, middle) == TemporalCertificateCheck::Accepted {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    low
}

/// FR-149-AC-5 (c) (TC-556): a certificate of a model whose concrete
/// contract costs more than `B`, checked under `B` where every mapping row
/// fits, stops naming `EvaluationMeter`, never `Rejected`.
#[trace("TC-556", "FR-149-AC-5")]
#[test]
fn a_concrete_contract_over_the_budget_stops_the_check() {
    let plain = safety_only();
    let (certificate, _) = certificate(&plain);
    let b = least_meter(&plain, &certificate);
    let heavy = with_contracts(&heavy_contract());
    assert_eq!(
        check_under(&heavy, &certificate, UNLIMITED.work_units),
        TemporalCertificateCheck::Accepted
    );
    assert_eq!(
        check_under(&heavy, &certificate, b),
        TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::EvaluationMeter,
            value: b
        })
    );
}

/// FR-144 Settlement (TC-556): a subject with no initial state settles
/// `Inconclusive(NoInitialState)`, not `failed`.
#[trace("TC-556")]
#[test]
fn a_subject_with_no_initial_state_settles_no_initial_state() {
    let world = safety_only();
    let SafetyOutcome::Holds { certificate, stats } = run(&world) else {
        panic!("the safety half holds");
    };
    let empty = SimulationCertificateRequest {
        concrete: world.request(CAS, Vec::new(), UNLIMITED),
        ..request(&world)
    };
    let record = settle_refinement_safety(
        &SafetyOutcome::Holds { certificate, stats },
        &ModelCheckLimits::default(),
        &empty,
        &Cancel::new(),
        |_| panic!("a holding half has no counterexample"),
    );
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::NoInitialState)
    );
}

/// FR-149-AC-7 (TC-556): a position that only an edge listed at a
/// higher-indexed position reaches is `Malformed` at that position, before
/// any recomputation.
#[trace("TC-556", "FR-149-AC-7")]
#[test]
fn a_position_reached_only_from_a_later_position_is_malformed() {
    let world = safety_only();
    let (mut certificate, _) = certificate(&world);
    let last = certificate.positions.len() - 1;
    let last_index = u64::try_from(last).expect("an index");
    for edges in &mut certificate.successors[..last] {
        for edge in edges.iter_mut().filter(|edge| edge.post == last_index) {
            edge.post = 0;
        }
    }
    assert_eq!(
        rejected(check(&world, &certificate)),
        (CertificateRule::Malformed, step(last_index, None))
    );
}

/// FR-149-AC-8 (TC-556): the locus of AC-3's rejection and AC-2's, and one
/// verdict of each other kind, serialize as the wire-form table lists.
#[trace("TC-556", "FR-149-AC-8")]
#[test]
fn a_rejection_serializes_its_locus_and_verdict() {
    let world = World::with(Variant::LOST_UPDATE, &lost_update_refinement());
    let (_, at) = rejected(check(&world, &listed(&world)));
    let wire = serde_json::to_value(&at).expect("a locus serializes");
    let CertificateLocus::SimulationStep {
        verdict: Some(verdict),
        ..
    } = &at
    else {
        panic!("a verdict");
    };
    let FailedStep::Fails(RefinementFailure::AbstractStepRejected {
        position,
        transition,
        ..
    }) = verdict.as_ref()
    else {
        panic!("a rejected abstract step");
    };
    let identity: serde_json::Value = serde_json::from_slice(
        &transition
            .canonical_bytes(quire_canonical::Limits::new(u64::MAX))
            .expect("an identity"),
    )
    .expect("a document");
    assert_eq!(
        wire["verdict"],
        serde_json::json!({
            "kind": "AbstractStepRejected",
            "position": position.to_string(),
            "transition": identity,
            "cause": {"kind": "Postcondition"},
        })
    );
    assert!(wire["position"].is_string() && wire["edge"].is_string());

    let plain = safety_only();
    let (mut certificate, _) = certificate(&plain);
    let from = position_of(&plain, &certificate, [0, 0, 1, 0, 0]);
    let edge = edge_of(&certificate, from, "commitA");
    edge_mut(&mut certificate, from, edge).taken = Taken::Stutter;
    let (_, at) = rejected(check(&plain, &certificate));
    let wire = serde_json::to_value(&at).expect("a locus serializes");
    assert!(wire["verdict"].is_null());
    assert_eq!(wire["position"], from.to_string());
    assert_eq!(wire["edge"], edge.to_string());

    let kinds = [
        (
            FailedStep::Fails(RefinementFailure::InitialNotAbstract { initial: 1 }),
            serde_json::json!({"kind": "InitialNotAbstract", "initial": "1"}),
        ),
        (
            FailedStep::Fails(RefinementFailure::StutterChanged { position: 2 }),
            serde_json::json!({"kind": "StutterChanged", "position": "2"}),
        ),
        (
            FailedStep::Fails(RefinementFailure::NoAbstractMatch { position: 3 }),
            serde_json::json!({"kind": "NoAbstractMatch", "position": "3"}),
        ),
        (
            FailedStep::Undetermined(UndeterminedStep::new(Undetermined::Mapping(Box::new(
                MappingUndetermined {
                    row: MappingRowRef::Object {
                        concrete_type: key(CAS, "Counter"),
                    },
                    object: None,
                    cause: EvalFailure::Evaluation(Box::new(ClauseOutcome::Incomplete(
                        Incomplete {
                            limit_kind: LimitKind::IntegerBits,
                            limit: 8,
                            consumed: 8,
                            next_charge: Integer::from(1_i64),
                            charge_point: ChargePoint::GraphExpand,
                        },
                    ))),
                },
            )))),
            serde_json::json!({
                "kind": "Undetermined",
                "cause": "MappingUndetermined",
                "evaluation": {"outcome": "incomplete", "limit": "integer_bits"},
            }),
        ),
        (
            FailedStep::Undetermined(UndeterminedStep::new(Undetermined::UndecidedSuccessor)),
            serde_json::json!({
                "kind": "Undetermined",
                "cause": "UndecidedSuccessor",
                "evaluation": {"outcome": "refused"},
            }),
        ),
    ];
    for (verdict, expected) in kinds {
        assert_eq!(
            serde_json::to_value(&verdict).expect("serializes"),
            expected
        );
    }
}
