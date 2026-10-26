// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-244's component and divergence parts (TC-699) and FR-245's check of
//! them (TC-700, TC-1197): the certificates of a liveness `Holds` on
//! FR-240's product and of the time-lock-freedom item, each accepted by the
//! core's checker and rejected with one part tampered.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::ZoneCheckLimits;
use qsl_replay::{
    CertificateCheckLimits, CertificateLocus, CertificateMember, CertificateRejection,
    CertificateRule, ComponentReason, Components, DivergenceWitness, DivergentTarget,
    FairnessMember, ZoneCertificateVerdict,
};
use qsl_route::deadlock::ObligationKind;
use quire_exact::Cancel;

use crate::timed_world::{document, world, World, DENSE};
use crate::zone_liveness::{decide, response, serve};
use crate::zone_search::{check, claim, proved, rpc, time_lock_item, Certificate, INFINITE};

fn verdict(world: &World, certificate: &Certificate) -> ZoneCertificateVerdict {
    check(world, certificate, CertificateCheckLimits::default()).expect("the check runs")
}

fn rejected(rule: CertificateRule, at: CertificateLocus) -> ZoneCertificateVerdict {
    ZoneCertificateVerdict::Rejected(CertificateRejection { rule, at })
}

fn components(certificate: &Certificate) -> &Components {
    certificate.components.as_ref().expect("a component part")
}

/// Whether every edge of `certificate` goes to an equal or greater
/// component, every node is numbered, and every index has a reason.
fn numbered(certificate: &Certificate) -> bool {
    let components = components(certificate);
    components.numbering.len() == certificate.nodes.len()
        && components
            .numbering
            .iter()
            .all(|index| index.is_some_and(|index| index < components.reasons.len()))
        && certificate
            .edges
            .iter()
            .all(|edge| components.numbering[edge.to] >= components.numbering[edge.from])
}

/// The components that hold an edge of operation `operation` between two
/// of their nodes.
fn holding(certificate: &Certificate, operation: &str) -> Vec<usize> {
    let numbering = &components(certificate).numbering;
    let mut found: Vec<usize> = certificate
        .edges
        .iter()
        .filter(|edge| {
            numbering[edge.from] == numbering[edge.to]
                && edge.transition.as_ref().is_some_and(|transition| {
                    transition
                        .operation()
                        .contains(&format!("Call/{operation}\""))
                })
        })
        .filter_map(|edge| numbering[edge.from])
        .collect();
    found.sort_unstable();
    found.dedup();
    found
}

/// FR-244-AC-2 and FR-245-AC-1: FR-240-AC-1's liveness proof over `Rpc`
/// with `T = 4 ms` carries a component part in which every edge respects
/// the numbering and each component names its reason, and no divergence
/// part; FR-240-AC-2's fair proof over `Serve` names `fair weak serve` as
/// the reason of the component that holds the `idle` loop. The checker
/// accepts both, and two runs give equal certificates.
#[trace("TC-699", "FR-244-AC-2")]
#[trace("TC-700", "FR-245-AC-1")]
#[test]
fn a_liveness_proof_carries_components_with_reasons() {
    let rpc = response(4, "");
    let certificate = proved(decide(&rpc));
    assert!(certificate.divergence.is_none());
    assert!(numbered(&certificate));
    assert!(certificate.nodes.iter().all(|node| node.product.is_some()));
    assert_eq!(certificate, proved(decide(&rpc)));
    assert_eq!(
        verdict(&rpc, &certificate),
        ZoneCertificateVerdict::Accepted
    );

    let fair = serve(">= 1 ms", "fair weak M::Call::serve;");
    let certificate = proved(decide(&fair));
    assert!(numbered(&certificate));
    let weak_serve = ComponentReason::Weak(FairnessMember {
        constraint: 0,
        transition: None,
    });
    let idling = holding(&certificate, "idle");
    assert!(
        idling
            .iter()
            .any(|&index| components(&certificate).reasons[index] == weak_serve),
        "{:?}",
        components(&certificate).reasons
    );
    assert_eq!(
        verdict(&fair, &certificate),
        ZoneCertificateVerdict::Accepted
    );
}

/// FR-244-AC-3 and FR-245-AC-1: the time-lock-freedom item over the
/// unmodified `Rpc` (`T = 4 ms`) is proved with a divergence part that
/// names, for every node, a path from it to a quiescent node, and no
/// component part. Two runs give equal certificates, and the checker
/// accepts it.
#[trace("TC-699", "FR-244-AC-3")]
#[trace("TC-700", "FR-245-AC-1")]
#[test]
fn a_time_lock_proof_carries_a_divergence_part() {
    let world = rpc(4);
    let certificate = proved(time_lock_item(&world));
    assert_eq!(certificate.obligation.kind, ObligationKind::TimeLockFreedom);
    assert!(certificate.components.is_none());
    let divergence = certificate.divergence.as_ref().expect("a divergence part");
    assert_eq!(divergence.len(), certificate.nodes.len());
    for (node, witness) in divergence.iter().enumerate() {
        assert_eq!(witness.start, node);
        assert!(
            matches!(witness.target, DivergentTarget::Quiescent(_)),
            "{witness:?}"
        );
    }
    assert_eq!(certificate, proved(time_lock_item(&world)));
    assert_eq!(
        verdict(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
}

/// FR-245-AC-2 and AC-7 over a liveness certificate: an edge pointing to a
/// node of smaller component index is rejected `BackwardEdge` at that
/// `ZoneEdge`; a component reason naming a fairness constraint that an
/// edge inside the component takes, `WitnessFails` at that
/// `ZoneComponent`; and a certificate without `components`, `Malformed` at
/// `CertificateMember("components")` before any successor is recomputed.
#[trace("TC-700", "FR-245-AC-2")]
#[trace("TC-1197", "FR-245-AC-7")]
#[test]
fn a_tampered_component_part_is_rejected() {
    let rpc = response(4, "");
    let certificate = proved(decide(&rpc));
    // One edge between two components, its target renumbered below its
    // source: the first edge the check then finds backward.
    let mut backward = certificate.clone();
    let numbering = &components(&certificate).numbering;
    let edge = certificate
        .edges
        .iter()
        .find(|edge| numbering[edge.from] < numbering[edge.to] && numbering[edge.from] > Some(0))
        .expect("an edge between two later components");
    backward.components.as_mut().expect("components").numbering[edge.to] = Some(0);
    let renumbered = &components(&backward).numbering;
    let first = backward
        .edges
        .iter()
        .position(|edge| renumbered[edge.to] < renumbered[edge.from])
        .expect("a backward edge");
    assert_eq!(
        verdict(&rpc, &backward),
        rejected(
            CertificateRule::BackwardEdge,
            CertificateLocus::ZoneEdge {
                edge: u64::try_from(first).expect("a small index"),
            }
        )
    );

    // Under `fair weak serve; fair weak idle`, the `idle` loop's component
    // is unfair by `serve`; naming `idle`, which its own edges take, fails.
    let fair = serve(
        ">= 1 ms",
        "fair weak M::Call::serve; fair weak M::Call::idle;",
    );
    let certificate = proved(decide(&fair));
    assert_eq!(
        verdict(&fair, &certificate),
        ZoneCertificateVerdict::Accepted
    );
    let weak_serve = ComponentReason::Weak(FairnessMember {
        constraint: 0,
        transition: None,
    });
    let index = holding(&certificate, "idle")
        .into_iter()
        .find(|&index| components(&certificate).reasons[index] == weak_serve)
        .expect("the idle loop's component");
    let mut wrong = certificate.clone();
    wrong.components.as_mut().expect("components").reasons[index] =
        ComponentReason::Weak(FairnessMember {
            constraint: 1,
            transition: None,
        });
    assert_eq!(
        verdict(&fair, &wrong),
        rejected(
            CertificateRule::WitnessFails,
            CertificateLocus::ZoneComponent {
                component: u64::try_from(index).expect("a small index"),
            }
        )
    );

    // Without its component part, before any successor is recomputed: the
    // same verdict with every edge's target removed too.
    let mut missing = certificate.clone();
    missing.components = None;
    let expected = rejected(
        CertificateRule::Malformed,
        CertificateLocus::CertificateMember(CertificateMember::Components),
    );
    assert_eq!(verdict(&fair, &missing), expected);
    missing.edges.clear();
    assert_eq!(verdict(&fair, &missing), expected);
}

/// `Lock` with a `back` step from `B` to `A` guarded `x < 1 ms`: at `B`
/// every valuation but `x = 1` may step back and let time diverge at `A`,
/// while at `x = 1` only `ping`, which resets nothing, stays enabled.
fn lock_with_back() -> World {
    world(
        &document(
            &["x"],
            &[("go", &["phase"]), ("ping", &[]), ("back", &["phase"])],
        ),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Go using v on M::Call::go {{ self.phase = 0 }}\n\
             post Gone using v on M::Call::go {{ self.phase = 1 }}\n\
             pre Ping using v on M::Call::ping {{ self.phase = 1 }}\n\
             pre Back using v on M::Call::back {{ self.phase = 1 and self.x < 1 ms }}\n\
             post Backed using v on M::Call::back {{ self.phase = 0 }}\n\
             time invariant Locked when {{ self.phase = 1 }} {{ self.x <= 1 ms }}\n\
             temporal Phases using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(c.phase = 0 or c.phase = 1) }}\n"
        ),
    )
}

/// FR-245-AC-2 over a divergence part: each node's witness is checked from
/// the node, and the node's every valuation must diverge. Over
/// [`lock_with_back`], whose time-lock-freedom item is violated at `B`
/// with `x = 1`, its proof of `always holds(c.phase = 0 or c.phase = 1)`
/// read as that item with every node's path
/// to the quiescent `A` (through `back` from `B`) is rejected
/// `WitnessFails` at the `B` node: its path holds from `x < 1` only. A
/// witness that starts at another node fails at its node too, and a
/// time-lock certificate without its divergence part is `Malformed` at
/// `CertificateMember("divergence")`.
#[trace("TC-700", "FR-245-AC-2")]
#[trace("TC-1197", "FR-245-AC-7")]
#[test]
fn a_divergence_part_must_cover_every_valuation() {
    let world = lock_with_back();
    assert!(matches!(
        time_lock_item(&world),
        qsl_analyze::zone_check::ZoneCheckOutcome::Violated { .. }
    ));
    let mut forged = proved(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    forged.obligation.kind = ObligationKind::TimeLockFreedom;
    let state_of = |node: usize| forged.nodes[node].state_key;
    let a = forged.initial[0];
    let b = (0..forged.nodes.len())
        .find(|&node| state_of(node) != state_of(a))
        .expect("a `B` node");
    let back = forged
        .edges
        .iter()
        .position(|edge| edge.from == b && state_of(edge.to) == state_of(a))
        .expect("a `back` edge");
    let to_a = forged.edges[back].to;
    let witness = |node: usize| {
        if state_of(node) == state_of(a) {
            DivergenceWitness {
                start: node,
                path: Vec::new(),
                target: DivergentTarget::Quiescent(node),
            }
        } else {
            DivergenceWitness {
                start: node,
                path: vec![back],
                target: DivergentTarget::Quiescent(to_a),
            }
        }
    };
    forged.divergence = Some((0..forged.nodes.len()).map(witness).collect());
    let at = |node: usize| CertificateLocus::ZoneNode {
        node: u64::try_from(node).expect("a small index"),
    };
    assert_eq!(
        verdict(&world, &forged),
        rejected(CertificateRule::WitnessFails, at(b))
    );

    let honest = rpc(4);
    let certificate = proved(time_lock_item(&honest));
    let mut moved = certificate.clone();
    let witnesses = moved.divergence.as_mut().expect("a divergence part");
    witnesses[1] = witnesses[0].clone();
    assert_eq!(
        verdict(&honest, &moved),
        rejected(CertificateRule::WitnessFails, at(1))
    );
    let mut missing = certificate;
    missing.divergence = None;
    assert_eq!(
        verdict(&honest, &missing),
        rejected(
            CertificateRule::Malformed,
            CertificateLocus::CertificateMember(CertificateMember::Divergence),
        )
    );
}
