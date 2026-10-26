// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-245 (TC-700, TC-1197): the core's check of EN-6's zone certificates,
//! `qsl_replay::check_zone_certificate`, over the timed fixtures of
//! `crate::zone_search`, and FR-235-AC-2's settlement through it.
//!
//! The certificates are the zone search's own; each tampering edits one
//! part of a valid certificate.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{settle_zone_check, ZoneCheckLimits, ZoneCheckOutcome, ZoneItem};
use qsl_eval::simulation::model::TimedSystem;
use qsl_replay::{
    CertificateCheckLimits, CertificateLocus, CertificateMember, CertificateRejection,
    CertificateRule, DivergentTarget, IncompleteCause, InconclusiveCause, ModelCheckLimit,
    ProofBasis, TerminalValue, Zone, ZoneCertificateRefusal, ZoneCertificateVerdict,
};
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use quire_exact::Cancel;

use crate::zone_replay::{fault, lock_resetting, package_id};
use crate::zone_search::{
    check, claim, deadlock, lock, proved, rpc, Certificate, Outcome, SUBJECT,
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
/// item's over the same subject. Not covered: FR-244-AC-2's liveness and
/// AC-3's time-lock-freedom certificates, which QSL-583 builds.
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
/// initial state is `InitialMissing` at that state. Not covered: the
/// backward edge and the component reason, which need FR-244's component
/// part (QSL-583); the bad node, which FR-245-AC-5 covers over `Lock`.
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
            item: qsl_replay::ModelCheckItem::DeadlockFreedom,
        },
        &other,
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
/// `inconclusive`, `CertificateRejected`. Not covered: `Settles`, a TT-3
/// claim the search decides with MightyL (QSL-583).
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
