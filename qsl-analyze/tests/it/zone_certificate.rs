// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-245 (TC-700, TC-1197): the core's check of EN-6's zone certificates,
//! `qsl_replay::check_zone_certificate`, over the timed fixtures of
//! `crate::zone_search`, and FR-235-AC-2's settlement through it.
//!
//! The certificates are the zone search's own; each tampering edits one
//! part of a valid certificate.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{
    certificate_item, settle_zone_check, ZoneCheckLimits, ZoneCheckOutcome, ZoneItem,
};
use qsl_eval::simulation::model::TimedSystem;
use qsl_replay::{
    CertificateCheckLimits, CertificateLocus, CertificateMember, CertificateRejection,
    CertificateRule, DivergentTarget, IncompleteCause, InconclusiveCause, ItemIdentity,
    ModelCheckLimit, ProofBasis, TerminalValue, Zone, ZoneCertificateRefusal,
    ZoneCertificateVerdict,
};
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use quire_exact::Cancel;

use crate::timed_world::{document, world, World, DENSE};
use crate::zone_replay::{fault, lock_resetting, package_id};
use crate::zone_search::{
    check, claim, deadlock, lock, proved, rpc, Certificate, Outcome, INFINITE, SUBJECT,
};

/// `NoLateReply` over `Rpc` with `T = 4 ms`, proved, with its certificate.
fn no_late_reply() -> (crate::timed_world::World, Certificate) {
    let world = rpc(4);
    let certificate = proved(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    (world, certificate)
}

fn verdict(world: &crate::timed_world::World, certificate: &Certificate) -> ZoneCertificateVerdict {
    check(world, certificate, CertificateCheckLimits::default()).expect("the check runs")
}

fn rejected(rule: CertificateRule, at: CertificateLocus) -> ZoneCertificateVerdict {
    ZoneCertificateVerdict::Rejected(CertificateRejection { rule, at })
}

fn node(index: usize) -> CertificateLocus {
    CertificateLocus::ZoneNode {
        node: u64::try_from(index).expect("a small index"),
    }
}

/// The least source of an edge into `target`.
fn first_source(certificate: &Certificate, target: usize) -> usize {
    certificate
        .edges
        .iter()
        .filter(|edge| edge.to == target && edge.from != target)
        .map(|edge| edge.from)
        .min()
        .expect("an edge into the node")
}

/// `certificate` without node `removed` and the edges at it, the other
/// indices renumbered. `removed` is on no part of `admitted`.
fn without_node(certificate: &Certificate, removed: usize) -> Certificate {
    let renumber = |node: usize| if node > removed { node - 1 } else { node };
    let mut kept = Vec::new();
    let mut edges = Vec::new();
    for (index, edge) in certificate.edges.iter().enumerate() {
        if edge.from == removed || edge.to == removed {
            continue;
        }
        kept.push(index);
        let mut edge = edge.clone();
        edge.from = renumber(edge.from);
        edge.to = renumber(edge.to);
        edges.push(edge);
    }
    let edge_of = |old: usize| {
        kept.iter()
            .position(|&index| index == old)
            .expect("an admitted edge is kept")
    };
    let mut tampered = certificate.clone();
    tampered.nodes.remove(removed);
    tampered.edges = edges;
    if let Some(components) = &mut tampered.components {
        components.numbering.remove(removed);
    }
    tampered.initial = certificate.initial.iter().map(|&n| renumber(n)).collect();
    tampered.admitted.start = renumber(certificate.admitted.start);
    tampered.admitted.path = certificate
        .admitted
        .path
        .iter()
        .map(|&e| edge_of(e))
        .collect();
    tampered.admitted.target = match &certificate.admitted.target {
        DivergentTarget::Quiescent(n) => DivergentTarget::Quiescent(renumber(*n)),
        DivergentTarget::Terminal(n) => DivergentTarget::Terminal(renumber(*n)),
        DivergentTarget::Cycle(cycle) => {
            DivergentTarget::Cycle(cycle.iter().map(|&e| edge_of(e)).collect())
        }
    };
    tampered
}

/// FR-245-AC-1 (TC-700): FR-244-AC-1's certificate, `NoLateReply` over
/// `Rpc` with `T = 4 ms`, is accepted, and so is the deadlock-freedom
/// item's over the same subject. FR-244-AC-2's and AC-3's are accepted in
/// `crate::zone_live_certificate`.
#[trace("TC-700")]
#[test]
fn the_search_certificates_are_accepted() {
    let (world, certificate) = no_late_reply();
    assert_eq!(
        verdict(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
    let deadlock_free = proved(deadlock(&world));
    assert_eq!(
        verdict(&world, &deadlock_free),
        ZoneCertificateVerdict::Accepted
    );
}

/// Part of FR-245-AC-2 and FR-245-AC-7 (TC-700, TC-1197): each tampering of
/// `NoLateReply`'s certificate is rejected at its rule and locus. Removing
/// a node leaves the first edge that named it `SuccessorMissing` at that
/// edge's source; shrinking a node's zone to its origin point does the
/// same; lowering an LU bound below the computed one is `Malformed` at
/// `bounds`; cutting `admitted` after `send` to end at its non-quiescent
/// target is `WitnessFails` there; an initial node that does not hold the
/// initial state is `InitialMissing` at that state. The backward edge and
/// the component reason are tampered in `crate::zone_live_certificate`;
/// the bad node is FR-245-AC-5's, over `Lock`.
#[trace("TC-700", "TC-1197")]
#[test]
fn each_tampering_is_rejected_at_its_rule_and_locus() {
    let (world, certificate) = no_late_reply();
    let last = certificate.nodes.len() - 1;
    assert!(!certificate.initial.contains(&last));
    assert_eq!(certificate.admitted.target, DivergentTarget::Quiescent(0));
    let removed = without_node(&certificate, last);
    assert_eq!(
        verdict(&world, &removed),
        rejected(
            CertificateRule::SuccessorMissing,
            node(first_source(&certificate, last))
        )
    );

    let mut shrunk = certificate.clone();
    let send = shrunk
        .edges
        .iter()
        .find(|edge| edge.from == 0 && edge.to != 0)
        .expect("an edge out of the initial node")
        .to;
    shrunk.nodes[send].zone = Zone::zero(shrunk.nodes[send].zone.dim());
    assert_eq!(
        verdict(&world, &shrunk),
        rejected(
            CertificateRule::SuccessorMissing,
            node(first_source(&certificate, send))
        )
    );

    let mut lowered = certificate.clone();
    let upper = lowered.lu_bounds.upper[0]
        .clone()
        .expect("x is compared from above");
    lowered.lu_bounds.upper[0] = Some(upper.sub(&quire_exact::Integer::one()));
    assert_eq!(
        verdict(&world, &lowered),
        rejected(
            CertificateRule::Malformed,
            CertificateLocus::CertificateMember(CertificateMember::Bounds)
        )
    );

    let mut cut = certificate.clone();
    let edge = cut
        .edges
        .iter()
        .position(|edge| edge.from == 0 && edge.to != 0)
        .expect("an edge out of the initial node");
    let target = cut.edges[edge].to;
    cut.admitted.path = vec![edge];
    cut.admitted.target = DivergentTarget::Quiescent(target);
    assert_eq!(
        verdict(&world, &cut),
        rejected(CertificateRule::WitnessFails, node(target))
    );

    let mut misplaced = certificate.clone();
    misplaced.initial = vec![target];
    let CertificateLocus::ModelState(digest) = (match verdict(&world, &misplaced) {
        ZoneCertificateVerdict::Rejected(CertificateRejection {
            rule: CertificateRule::InitialMissing,
            at,
        }) => at,
        other => panic!("expected InitialMissing, got {other:?}"),
    }) else {
        panic!("the initial discrete state's locus");
    };
    assert_eq!(digest, certificate.nodes[0].state_key);
}

/// Part of FR-245-AC-3 (TC-700): `NoLateReply`'s certificate checked for the
/// deadlock-freedom item refuses `stale_dependency`, naming both
/// obligations; with `max_certificate_edges` 1 the check stops at that
/// limit and value, and with `max_certificate_bytes` 1 at that one. Not
/// covered: the source edit, whose FR-098 recompile waits on S4 writing a
/// timed model's members (QSL-580).
#[trace("TC-700")]
#[test]
fn identity_and_budgets_are_checked_first() {
    let (world, certificate) = no_late_reply();
    let other = ItemObligation {
        subject: SUBJECT,
        kind: ObligationKind::DeadlockFreedom,
    };
    let refusal = qsl_replay::check_zone_certificate(
        qsl_replay::ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            max_clocks: qsl_eval::simulation::model::DEFAULT_MAX_CLOCKS,
            item: qsl_replay::CertifiedItem::DeadlockFreedom,
        },
        &other,
        certificate_item,
        &certificate,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    )
    .expect_err("another item's obligation");
    assert_eq!(
        refusal.code(),
        qsl_foundation::diagnostic::Code::StaleDependency
    );
    assert_eq!(
        refusal,
        ZoneCertificateRefusal::ObligationMismatch {
            certificate: Box::new(certificate.obligation.clone()),
            item: Box::new(other),
        }
    );

    for (limits, limit) in [
        (
            CertificateCheckLimits {
                max_certificate_edges: 1,
                ..CertificateCheckLimits::default()
            },
            ModelCheckLimit::CertificateMaxEdges,
        ),
        (
            CertificateCheckLimits {
                max_certificate_bytes: 1,
                ..CertificateCheckLimits::default()
            },
            ModelCheckLimit::CertificateMaxBytes,
        ),
    ] {
        assert_eq!(
            check(&world, &certificate, limits),
            Ok(ZoneCertificateVerdict::Stopped(
                IncompleteCause::LimitReached { limit, value: 1 }
            ))
        );
    }
    assert_eq!(
        ModelCheckLimit::CertificateMaxEdges.setting(),
        "certificate_check.max_certificate_edges"
    );
}

/// FR-245-AC-5 (TC-700): FR-244-AC-4's `Lock` certificate is accepted;
/// without its `locked` mark the node `(B, x <= 1)` is `BadState`; and the
/// same certificate over the variant's state keys, its `L` raised to the
/// variant's 0, checked over the
/// variant where `ping` resets `x` is
/// `BadState` at that `locked` node, from which the `ping` cycle, which
/// now lets time diverge, is reachable.
#[trace("TC-700", "FR-245-AC-5")]
#[test]
fn a_locked_node_is_checked_for_divergence() {
    let world = lock();
    let certificate = proved(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    assert_eq!(
        verdict(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
    let locked = certificate
        .nodes
        .iter()
        .position(|node| node.locked)
        .expect("a locked node");
    let mut unmarked = certificate.clone();
    unmarked.nodes[locked].locked = false;
    assert_eq!(
        verdict(&world, &unmarked),
        rejected(CertificateRule::BadState, node(locked))
    );
    // The variant's certificate: the same graph over the variant's state
    // keys (`A` and `B`) and transition identities (`go` and `ping`), whose reset compares `x` with 0 from below, so its
    // bounds carry `L = 0`.
    let resetting = lock_resetting(true);
    let system = resetting.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let start = timed.initial().remove(0);
    let key = |state: &qsl_eval::simulation::model::ModelSimState| {
        system.state_key(state).expect("a state key").1
    };
    let a = key(start.discrete());
    let (_, gone) = timed
        .steps(&start)
        .expect("decided")
        .into_iter()
        .find(|(step, _)| step.operation().contains("Call/go\""))
        .expect("`go` is enabled");
    let b = key(gone.discrete());
    let old_a = certificate.nodes[certificate.initial[0]].state_key;
    let mut reset_bounds = certificate.clone();
    for listed in &mut reset_bounds.nodes {
        listed.state_key = if listed.state_key == old_a { a } else { b };
    }
    let transition = |state: &qsl_eval::simulation::model::ModelSimState, operation: &str| {
        timed
            .guarded_steps(state)
            .expect("decided")
            .into_iter()
            .find(|step| step.operation().contains(&format!("Call/{operation}\"")))
            .expect("the step is enabled")
            .transition()
            .clone()
    };
    let go = transition(start.discrete(), "go");
    let ping = transition(gone.discrete(), "ping");
    for edge in &mut reset_bounds.edges {
        let old = edge.transition.as_ref().expect("a step");
        edge.transition = Some(if old.operation().contains("Call/go\"") {
            go.clone()
        } else {
            ping.clone()
        });
    }
    reset_bounds.lu_bounds.lower[0] = Some(quire_exact::Integer::zero());
    assert_eq!(
        verdict(&resetting, &reset_bounds),
        rejected(CertificateRule::BadState, node(locked))
    );
}

/// FR-245-AC-7 (TC-1197): the zone loci are written in QSpec FR-331's
/// `{node}`, `{edge}`, `{component}` and `{member}` forms, each index a
/// `U64String`.
#[trace("TC-1197", "FR-245-AC-7")]
#[test]
fn zone_loci_are_written_in_their_fr331_forms() {
    for (locus, written) in [
        (node(3), serde_json::json!({"node": "3"})),
        (
            CertificateLocus::ZoneEdge { edge: 7 },
            serde_json::json!({"edge": "7"}),
        ),
        (
            CertificateLocus::ZoneComponent { component: 0 },
            serde_json::json!({"component": "0"}),
        ),
        (
            CertificateLocus::CertificateMember(CertificateMember::Bounds),
            serde_json::json!({"member": "bounds"}),
        ),
        (
            CertificateLocus::CertificateMember(CertificateMember::Divergence),
            serde_json::json!({"member": "divergence"}),
        ),
    ] {
        assert_eq!(serde_json::to_value(&locus).expect("a locus"), written);
    }
}

/// Part of FR-235-AC-2 (TC-690): `NoLateReply` over `Rpc` with `T = 4 ms`,
/// decided by the zone search, settles `Proved{ZoneCertified, Certified}`
/// once the core accepts its certificate; with one node removed it settles
/// `inconclusive`, `CertificateRejected`. `Settles` is
/// [`a_settles_proof_settles_through_the_core_checker`].
#[trace("TC-690")]
#[test]
fn a_zone_proof_settles_through_the_core_checker() {
    let world = rpc(4);
    let proof = claim(&world, ZoneCheckLimits::default(), &Cancel::new());
    let settle = |outcome: &Outcome| {
        settle_zone_check(
            ZoneItem::Outcome(outcome),
            &package_id(),
            &Cancel::new(),
            |certificate| verdict(&world, certificate),
            |_| Err(fault()),
        )
    };
    let record = settle(&proof);
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::ZoneCertified,
            certification: qsl_replay::Certification::Certified,
        }
    );
    let ZoneCheckOutcome::Holds { certificate, stats } = &proof else {
        panic!("a proof");
    };
    let last = certificate.nodes.len() - 1;
    let tampered: Outcome = ZoneCheckOutcome::Holds {
        certificate: Box::new(without_node(certificate, last)),
        stats: *stats,
    };
    let record = settle(&tampered);
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::SuccessorMissing,
            at: node(first_source(certificate, last)),
        })
    );
}

/// Part of FR-235-AC-2 (TC-690): `Settles` over `Rpc` with `T = 4 ms`, a
/// TT-3 claim decided on the liveness product with FR-242's timed claim
/// automaton, settles `Proved{ZoneCertified, Certified}` once the core,
/// rebuilding the automaton itself, accepts its certificate; with one node
/// removed it settles `inconclusive`, `CertificateRejected`.
#[trace("TC-690")]
#[test]
fn a_settles_proof_settles_through_the_core_checker() {
    let world = crate::zone_mitl::settles(4, "[0 ms, 3 ms]");
    let proof = claim(&world, ZoneCheckLimits::default(), &Cancel::new());
    let settle = |outcome: &Outcome| {
        settle_zone_check(
            ZoneItem::Outcome(outcome),
            &package_id(),
            &Cancel::new(),
            |certificate| verdict(&world, certificate),
            |_| Err(fault()),
        )
    };
    assert_eq!(
        settle(&proof).value,
        TerminalValue::Proved {
            basis: ProofBasis::ZoneCertified,
            certification: qsl_replay::Certification::Certified,
        }
    );
    let ZoneCheckOutcome::Holds { certificate, stats } = &proof else {
        panic!("a proof");
    };
    let last = certificate.nodes.len() - 1;
    let tampered: Outcome = ZoneCheckOutcome::Holds {
        certificate: Box::new(without_node(certificate, last)),
        stats: *stats,
    };
    assert!(matches!(
        settle(&tampered).value,
        TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected { .. })
    ));
}

/// `certificate` with its obligation renamed to `world`'s first clause, so
/// it is checked as a proof of that clause.
fn retarget(mut certificate: Certificate, world: &World) -> Certificate {
    certificate.obligation = ItemObligation {
        subject: SUBJECT,
        kind: ObligationKind::Authored(qsl_replay::node_occurrence(
            &world.package.graph().temporal_clauses()[0],
        )),
    };
    certificate
}

fn summary(outcome: &Outcome) -> &'static str {
    match outcome {
        ZoneCheckOutcome::Holds { .. } => "Holds",
        ZoneCheckOutcome::HoldsReduced { .. } => "HoldsReduced",
        ZoneCheckOutcome::Reduction { .. } => "Reduction",
        ZoneCheckOutcome::Violated { .. } => "Violated",
        ZoneCheckOutcome::Undecided { .. } => "Undecided",
        ZoneCheckOutcome::Stopped { .. } => "Stopped",
    }
}

/// `flip` moves phase 0 to 1, or with `nondet` to 1 or 2, under one
/// transition identity; `back` returns from 1. The claim is `always
/// holds(not (c.phase = 2))`.
fn flip(nondet: bool) -> World {
    let post = if nondet {
        "self.phase = 1 or self.phase = 2"
    } else {
        "self.phase = 1"
    };
    world(
        &document(&["x"], &[("flip", &["phase"]), ("back", &["phase"])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Flip using v on M::Call::flip {{ self.phase = 0 }}\n\
             post Flipped using v on M::Call::flip {{ {post} }}\n\
             pre Back using v on M::Call::back {{ self.phase = 1 }}\n\
             post Backed using v on M::Call::back {{ self.phase = 0 }}\n\
             temporal NoTwo using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(not (c.phase = 2)) }}\n"
        ),
    )
}

/// FR-245 (SR-1637001 FND-001): a successor is matched to an edge by its
/// post-state too. When one transition identity has two posts, the
/// deterministic model's certificate, which lists one, is rejected
/// `SuccessorMissing` over the nondeterministic model, which EN-6 refutes.
#[trace("TC-700")]
#[test]
fn a_successor_is_matched_by_its_post_state() {
    let nondet = flip(true);
    let honest = claim(&nondet, ZoneCheckLimits::default(), &Cancel::new());
    assert_eq!(summary(&honest), "Violated");
    let det = flip(false);
    let certificate = retarget(
        proved(claim(&det, ZoneCheckLimits::default(), &Cancel::new())),
        &nondet,
    );
    assert_eq!(
        verdict(&nondet, &certificate),
        rejected(CertificateRule::SuccessorMissing, node(0))
    );
}

/// FR-245 (SR-1637001 FND-002): the deadlock-freedom item's coverage is
/// read under the aM bisimulation, which preserves deadlocks. `go` is
/// guarded by `x <= 1 ms`, so the model deadlocks past 1; node zones capped
/// at `x <= 1`, which cover every valuation under the aLU simulation, do
/// not cover the initial zone and are rejected.
#[trace("TC-700")]
#[test]
fn a_deadlock_is_not_hidden_by_coverage() {
    let guarded = world(
        &document(&["x"], &[("go", &[])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Go using v on M::Call::go {{ self.x <= 1 ms }}\n\
             temporal StaysA using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(c.phase = 0) }}\n"
        ),
    );
    assert_eq!(summary(&deadlock(&guarded)), "Undecided");
    let mut certificate = proved(claim(&guarded, ZoneCheckLimits::default(), &Cancel::new()));
    certificate.obligation.kind = ObligationKind::DeadlockFreedom;
    let scale = certificate.scale.clone();
    for listed in &mut certificate.nodes {
        listed.zone.constrain(1, 0, &scale, false).expect("a clock");
    }
    assert!(matches!(
        verdict(&guarded, &certificate),
        ZoneCertificateVerdict::Rejected(CertificateRejection {
            rule: CertificateRule::InitialMissing,
            ..
        })
    ));
}

/// `go` needs `x >= 2 ms` to leave phase 0, where with `invariant` time is
/// held to `x <= 1 ms`, so every behaviour is time-locked.
fn stuck(invariant: bool) -> World {
    let held = if invariant {
        "time invariant Stuck when { self.phase = 0 } { self.x <= 1 ms }\n"
    } else {
        ""
    };
    world(
        &document(&["x"], &[("go", &["phase"])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Go using v on M::Call::go {{ self.phase = 0 and self.x >= 2 ms }}\n\
             post Gone using v on M::Call::go {{ self.phase = 1 }}\n\
             {held}\
             temporal Fine using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(not (c.phase = 2)) }}\n"
        ),
    )
}

/// FR-245 (SR-1637001 FND-003): `admitted` is recomputed in exact zones
/// held to each state's own invariants. Over the time-locked model, which
/// EN-6 finds has no admitted behaviour, a free model's certificate whose
/// `admitted` path takes `go` to the quiescent phase 1 is rejected
/// `WitnessFails` at its start: no valuation within `x <= 1` takes `go`,
/// so the edge holds no successor.
#[trace("TC-700")]
#[test]
fn an_admitted_path_is_one_some_behaviour_takes() {
    let locked = stuck(true);
    assert_eq!(
        summary(&claim(&locked, ZoneCheckLimits::default(), &Cancel::new())),
        "Undecided"
    );
    let free = stuck(false);
    let mut certificate = retarget(
        proved(claim(&free, ZoneCheckLimits::default(), &Cancel::new())),
        &locked,
    );
    certificate.lu_bounds.upper[0] = Some(certificate.scale.clone());
    let go = certificate
        .edges
        .iter()
        .position(|edge| edge.from == 0 && edge.to != 0)
        .expect("go");
    let target = certificate.edges[go].to;
    certificate.admitted.start = 0;
    certificate.admitted.path = vec![go];
    certificate.admitted.target = DivergentTarget::Quiescent(target);
    assert_eq!(
        verdict(&locked, &certificate),
        rejected(CertificateRule::WitnessFails, node(0))
    );
}

/// `Lock` with a second clock `w`, bounded only by an invariant of a phase
/// never reached; `ping` resets `x` when `resets`.
fn lock_with_w(resets: bool) -> World {
    let ping: &[&str] = if resets { &["x"] } else { &[] };
    let reset = if resets {
        "post Pinged using v on M::Call::ping { self.x = 0 ms }\n"
    } else {
        ""
    };
    world(
        &document(&["x", "w"], &[("go", &["phase"]), ("ping", ping)]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Go using v on M::Call::go {{ self.phase = 0 }}\n\
             post Gone using v on M::Call::go {{ self.phase = 1 }}\n\
             pre Ping using v on M::Call::ping {{ self.phase = 1 }}\n\
             {reset}\
             time invariant Locked when {{ self.phase = 1 }} {{ self.x <= 1 ms }}\n\
             time invariant Far when {{ self.phase = 3 }} {{ self.w <= 5 ms }}\n\
             temporal StaysA using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(c.phase = 0) }}\n"
        ),
    )
}

/// FR-245 (SR-1637001 FND-004): a set of nodes is Zeno only by a clock its
/// states' own invariants bound, never by bounds read from node zones.
/// The plain model's certificate, its nodes renamed to the variant in
/// which `ping` resets `x` and its zones bounding `w` as `x`, is rejected
/// `BadState` at the locked node: `ping` then lets time diverge, and EN-6
/// refutes the variant.
#[trace("TC-700")]
#[test]
fn a_zeno_set_is_bounded_by_its_own_invariants() {
    let plain = lock_with_w(false);
    let resetting = lock_with_w(true);
    assert_eq!(
        summary(&claim(
            &resetting,
            ZoneCheckLimits::default(),
            &Cancel::new()
        )),
        "Violated"
    );
    let certificate = proved(claim(&plain, ZoneCheckLimits::default(), &Cancel::new()));
    let system = resetting.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let start = timed.initial().remove(0);
    let key = |state: &qsl_eval::simulation::model::ModelSimState| {
        system.state_key(state).expect("a key").1
    };
    let (_, gone) = timed
        .steps(&start)
        .expect("decided")
        .into_iter()
        .find(|(step, _)| step.operation().contains("Call/go\""))
        .expect("go");
    let transition = |state: &qsl_eval::simulation::model::ModelSimState, operation: &str| {
        timed
            .guarded_steps(state)
            .expect("decided")
            .into_iter()
            .find(|step| step.operation().contains(&format!("Call/{operation}\"")))
            .expect("enabled")
            .transition()
            .clone()
    };
    let (a, b) = (key(start.discrete()), key(gone.discrete()));
    let (go, ping) = (
        transition(start.discrete(), "go"),
        transition(gone.discrete(), "ping"),
    );
    let initial_key = certificate.nodes[certificate.initial[0]].state_key;
    let mut moved = retarget(certificate, &resetting);
    for listed in &mut moved.nodes {
        listed.state_key = if listed.state_key == initial_key {
            a
        } else {
            b
        };
    }
    for edge in &mut moved.edges {
        let old = edge.transition.as_ref().expect("a step");
        edge.transition = Some(if old.operation().contains("Call/go\"") {
            go.clone()
        } else {
            ping.clone()
        });
    }
    let x = moved
        .lu_bounds
        .upper
        .iter()
        .position(|upper| upper.as_ref() == Some(&moved.scale))
        .expect("x's place");
    moved.lu_bounds.lower[x] = Some(quire_exact::Integer::zero());
    let locked = moved
        .nodes
        .iter()
        .position(|listed| listed.locked)
        .expect("the locked node");
    assert_eq!(
        verdict(&resetting, &moved),
        rejected(CertificateRule::BadState, node(locked))
    );
}

/// `bad` needs `x < 1/2 ms`; with `reachable` false it is never enabled.
/// An invariant of an unreached phase compares `x` with 3.
fn half_guard(reachable: bool) -> World {
    let pre = if reachable {
        "self.phase = 0"
    } else {
        "self.phase = 3"
    };
    world(
        &document(&["x"], &[("bad", &["phase"])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Bad using v on M::Call::bad {{ {pre} and self.x < rational(1, 2) ms }}\n\
             post Badded using v on M::Call::bad {{ self.phase = 2 }}\n\
             time invariant Far when {{ self.phase = 3 }} {{ self.x <= 3 ms }}\n\
             temporal NoTwo using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(not (c.phase = 2)) }}\n"
        ),
    )
}

/// FR-245 (SR-1637001 FND-005): every constant, not only each clock's
/// largest, must be exact at the scale. The unreachable-guard model's
/// certificate at scale 1 is rejected `Malformed` at `scale` over the model
/// whose `bad` is enabled while `x < 1/2`, which EN-6 refutes.
#[trace("TC-700")]
#[test]
fn every_constant_is_exact_at_the_scale() {
    let real = half_guard(true);
    assert_eq!(
        summary(&claim(&real, ZoneCheckLimits::default(), &Cancel::new())),
        "Violated"
    );
    let sibling = half_guard(false);
    let mut certificate = retarget(
        proved(claim(&sibling, ZoneCheckLimits::default(), &Cancel::new())),
        &real,
    );
    certificate.scale = quire_exact::Integer::one();
    certificate.lu_bounds.upper[0] = Some(quire_exact::Integer::from(3_i64));
    assert_eq!(
        verdict(&real, &certificate),
        rejected(
            CertificateRule::Malformed,
            CertificateLocus::CertificateMember(CertificateMember::Scale)
        )
    );
}

/// A model whose divergence cycle leaves a clock unreset: `loop`, guarded
/// `y >= 1`, resets `y`, held to `y <= 2`, while `x` grows forever.
fn drifting() -> World {
    world(
        &document(&["x", "y"], &[("loop", &["y"])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Loop using v on M::Call::loop {{ self.y >= 1 ms }}\n\
             post Looped using v on M::Call::loop {{ self.y = 0 ms }}\n\
             time invariant Busy when {{ self.phase = 0 }} {{ self.y <= 2 ms }}\n\
             temporal Fine using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(not (c.phase = 2)) }}\n"
        ),
    )
}

/// SR-1637001 FND-012, SR-1637002 FND-005: an honest certificate whose
/// `admitted` is a cycle that leaves `x` unreset, so that `x` drifts at
/// each repetition, is accepted for the clause and for the deadlock item.
/// The repetition check reads the cycle's edges again at each round,
/// against `max_certificate_edges`: a budget of the listed edges alone
/// stops it, naming the limit (SR-1637001 FND-013).
#[trace("TC-700")]
#[test]
fn a_cycle_that_leaves_a_clock_unreset_is_accepted() {
    let world = drifting();
    let certificates = [
        proved(claim(&world, ZoneCheckLimits::default(), &Cancel::new())),
        proved(deadlock(&world)),
    ];
    for certificate in &certificates {
        let DivergentTarget::Cycle(cycle) = &certificate.admitted.target else {
            panic!("a cycle witness, got {:?}", certificate.admitted);
        };
        assert!(!cycle.is_empty());
        assert_eq!(
            verdict(&world, certificate),
            ZoneCertificateVerdict::Accepted
        );
        let listed = u64::try_from(certificate.edges.len()).expect("a small count");
        assert_eq!(
            check(
                &world,
                certificate,
                CertificateCheckLimits {
                    max_certificate_edges: listed,
                    ..CertificateCheckLimits::default()
                },
            ),
            Ok(ZoneCertificateVerdict::Stopped(
                IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::CertificateMaxEdges,
                    value: listed,
                }
            ))
        );
    }
}

/// FR-245: a certificate whose obligation is the request's own but names
/// another item than the recompiled one refuses `stale_dependency`, naming
/// both identities (SR-1637001 FND-014): a clause's proof read as the
/// deadlock-freedom item.
#[trace("TC-700")]
#[test]
fn an_identity_mismatch_names_both_identities() {
    let (world, certificate) = no_late_reply();
    let refusal = qsl_replay::check_zone_certificate(
        qsl_replay::ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            max_clocks: qsl_eval::simulation::model::DEFAULT_MAX_CLOCKS,
            item: qsl_replay::CertifiedItem::DeadlockFreedom,
        },
        &certificate.obligation,
        certificate_item,
        &certificate,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    )
    .expect_err("a clause's certificate");
    assert_eq!(
        refusal.code(),
        qsl_foundation::diagnostic::Code::StaleDependency
    );
    assert_eq!(
        refusal,
        ZoneCertificateRefusal::IdentityMismatch {
            certificate: Box::new(ItemIdentity::Clause(qsl_replay::node_occurrence(
                &world.package.graph().temporal_clauses()[0],
            ))),
            recompiled: Box::new(ItemIdentity::DeadlockFreedom),
        }
    );
}
