// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-430 "Certificate", FR-244 "Refinement form" and FR-245
//! "Refinement form" (TC-1170, TC-699, TC-700): a timed refinement's
//! holding safety half carries a zone certificate over the refinement
//! product, which the core's checker accepts before the item settles
//! `proved`.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{settle_timed_safety, TimedSafetyOutcome};
use qsl_eval::simulation::model::DEFAULT_MAX_CLOCKS;
use qsl_replay::{
    check_zone_certificate, CertificateCheckLimits, CertificateItem, CertificateLocus,
    CertificateMember, CertificateRule, CertifiedItem, ProofBasis, Taken, TerminalValue,
    ZoneCertificate, ZoneCertificateRefusal, ZoneCertificateRequest, ZoneCertificateVerdict,
};
use qsl_replay::{CertificateRejection, Certification};
use quire_exact::Cancel;

use crate::timed_refinement::{dead_undo, safety, unit, CONTRACTS};
use crate::timed_world::{refining_from, Refining, DENSE};

/// The certificate of `world`'s holding safety half.
fn certificate(world: &Refining) -> ZoneCertificate<String> {
    match safety(world) {
        TimedSafetyOutcome::Holds { certificate, .. } => *certificate,
        other => panic!("the safety half holds: {other:?}"),
    }
}

/// FR-245 over `world`'s refinement item: `certificate` checked against
/// the refinement, its abstract package and `world`'s abstract initial
/// state.
fn check(
    world: &Refining,
    certificate: &ZoneCertificate<String>,
) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<String>> {
    let [refinement] = world.package.graph().refinements() else {
        panic!("one refinement");
    };
    let initial = [world.abstract_initial.clone()];
    check_zone_certificate(
        ZoneCertificateRequest {
            simulation: world.concrete_request(),
            model: "M",
            max_clocks: DEFAULT_MAX_CLOCKS,
            item: CertifiedItem::Refinement {
                refinement,
                abstract_package: &world.package,
                abstract_initial: &initial,
            },
        },
        &refinement.name().to_owned(),
        |name| CertificateItem::Refinement(name),
        certificate,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    )
}

fn rejected(verdict: ZoneCertificateVerdict) -> (CertificateRule, CertificateLocus) {
    match verdict {
        ZoneCertificateVerdict::Rejected(CertificateRejection { rule, at }) => (rule, at),
        other => panic!("expected a rejection, got {other:?}"),
    }
}

fn edge(index: usize) -> CertificateLocus {
    CertificateLocus::ZoneEdge {
        edge: u64::try_from(index).expect("an index"),
    }
}

/// The `TReq` refinement of FR-430-AC-1.
fn treq() -> Refining {
    unit("self.inflight").unwrap_or_else(|refusals| panic!("{refusals:?}"))
}

/// FR-244-AC-5 and FR-430-AC-8: `TReq`'s holding half carries the
/// refinement form: the part present, each node's second component an
/// index into its positions, every edge a discrete `send` step with its
/// `Taken`, and an `admitted` path. FR-245 accepts it, the item settles
/// `Proved{ZoneCertified, Certified}`, and two runs give equal
/// certificates.
#[trace("TC-1170", "FR-430-AC-8")]
#[trace("TC-699", "FR-244-AC-5")]
#[test]
fn a_holding_timed_refinement_carries_an_accepted_certificate() {
    let world = treq();
    let certificate = certificate(&world);
    let part = certificate
        .refinement
        .as_ref()
        .expect("the refinement form");
    assert!(certificate
        .nodes
        .iter()
        .all(|node| node.automaton_state < part.positions.len() && !node.locked));
    assert!(!certificate.edges.is_empty());
    assert!(certificate.edges.iter().all(|edge| edge
        .transition
        .as_ref()
        .is_some_and(|transition| transition.operation().contains("/Call/send\""))));
    assert_eq!(part.taken.len(), certificate.edges.len());
    assert!(part
        .taken
        .iter()
        .all(|taken| matches!(taken, Some(Taken::Abstract(_)))));
    assert_eq!(certificate, self::certificate(&world));
    assert_eq!(
        check(&world, &certificate),
        Ok(ZoneCertificateVerdict::Accepted)
    );
    let outcome = safety(&world);
    let record = settle_timed_safety(
        &outcome,
        &Cancel::new(),
        |certificate| check(&world, certificate).expect("the check runs"),
        |_| panic!("a holding half replays nothing"),
    );
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::ZoneCertified,
            certification: Certification::Certified,
        }
    );
}

/// FR-430-AC-8 and FR-245-AC-8: AC-6's holding certificate lists the
/// post-state of `undo` as a `locked` node at the reserved index, entered
/// by the failing `undo` edge, and FR-245 accepts it; with the `locked`
/// mark removed, it is `Rejected` `StepFails` at that edge.
#[trace("TC-1170", "FR-430-AC-8")]
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn a_failing_step_into_a_time_lock_is_a_locked_node_of_the_certificate() {
    let world = dead_undo(true);
    let mut certificate = certificate(&world);
    let reserved = certificate
        .refinement
        .as_ref()
        .expect("the refinement form")
        .positions
        .len();
    let entry = certificate
        .nodes
        .iter()
        .position(|node| node.locked)
        .expect("a locked node");
    assert_eq!(certificate.nodes[entry].automaton_state, reserved);
    let (into, undo) = certificate
        .edges
        .iter()
        .enumerate()
        .find(|(_, edge)| edge.to == entry)
        .expect("an edge enters the locked node");
    assert!(undo
        .transition
        .as_ref()
        .is_some_and(|transition| transition.operation().contains("/Call/undo\"")));
    let part = certificate.refinement.as_ref().expect("the part");
    assert_eq!(part.taken[into], None);
    assert_eq!(
        check(&world, &certificate),
        Ok(ZoneCertificateVerdict::Accepted)
    );
    certificate.nodes[entry].locked = false;
    assert_eq!(
        rejected(check(&world, &certificate).expect("the check runs")),
        (CertificateRule::StepFails, edge(into))
    );
}

/// FR-245-AC-8's rejections of a forged `TReq` certificate: with its
/// positions removed `Malformed` at `positions`, before any successor is
/// recomputed; with an edge's target position replaced, or its `taken`
/// changed to `Stutter`, `PostDiffers` at that edge; checked against the
/// abstract initial `sent = true` of FR-430-AC-7, `InitialMismatch` at the
/// initial node; with the refinement part removed, `Malformed` at
/// `refinement`.
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn a_forged_refinement_certificate_is_rejected() {
    let world = treq();
    let certificate = certificate(&world);
    let mut forged = certificate.clone();
    forged
        .refinement
        .as_mut()
        .expect("the part")
        .positions
        .clear();
    assert_eq!(
        rejected(check(&world, &forged).expect("the check runs")),
        (
            CertificateRule::Malformed,
            CertificateLocus::CertificateMember(CertificateMember::Positions)
        )
    );
    let mut forged = certificate.clone();
    forged.refinement = None;
    assert_eq!(
        rejected(check(&world, &forged).expect("the check runs")),
        (
            CertificateRule::Malformed,
            CertificateLocus::CertificateMember(CertificateMember::Refinement)
        )
    );
    // The first edge goes from the initial node's position to another.
    let first = &certificate.edges[0];
    let (from, to) = (
        certificate.nodes[first.from].automaton_state,
        certificate.nodes[first.to].automaton_state,
    );
    assert_ne!(from, to, "the first send changes the position");
    let mut forged = certificate.clone();
    forged.nodes[first.to].automaton_state = from;
    assert_eq!(
        rejected(check(&world, &forged).expect("the check runs")),
        (CertificateRule::PostDiffers, edge(0))
    );
    let mut forged = certificate.clone();
    forged.refinement.as_mut().expect("the part").taken[0] = Some(Taken::Stutter);
    assert_eq!(
        rejected(check(&world, &forged).expect("the check runs")),
        (CertificateRule::PostDiffers, edge(0))
    );
    let sent = refining_from(
        &crate::timed_refinement::req(),
        &crate::timed_refinement::treq(),
        &format!(
            "{DENSE}{CONTRACTS}{}",
            crate::timed_refinement::refinement("self.inflight")
        ),
        true,
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert_eq!(
        rejected(check(&sent, &certificate).expect("the check runs")),
        (
            CertificateRule::InitialMismatch,
            CertificateLocus::ZoneNode {
                node: u64::try_from(certificate.initial[0]).expect("an index")
            }
        )
    );
}

/// FR-245-AC-8: checked against a refinement item over another abstract
/// package, the certificate is refused `stale_dependency`/
/// `content-mismatch`.
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn a_refinement_certificate_for_another_abstract_package_is_refused() {
    let world = treq();
    let certificate = certificate(&world);
    let other_abstract = crate::timed_world::renamed_package(
        &crate::timed_world::document(&[], &[("send", &["inflight"])]),
        "test/other-req",
    );
    let other = crate::timed_world::refining(
        &other_abstract,
        &crate::timed_refinement::treq(),
        &format!(
            "{DENSE}{CONTRACTS}{}",
            crate::timed_refinement::refinement("self.inflight")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let refusal = check(&other, &certificate).expect_err("another abstract package");
    assert_eq!(refusal, ZoneCertificateRefusal::RefinementMismatch);
    assert_eq!(refusal.code().as_str(), "stale_dependency");
}

/// FR-245-AC-8: the `TReq` certificate checked under a request whose
/// evaluation meter allows no work stops at the first `check_step`,
/// `Stopped(LimitReached)` naming the evaluation meter, never `Rejected`.
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn a_refinement_certificate_check_out_of_meter_stops() {
    let world = treq();
    let certificate = certificate(&world);
    let [refinement] = world.package.graph().refinements() else {
        panic!("one refinement");
    };
    let initial = [world.abstract_initial.clone()];
    let mut simulation = world.concrete_request();
    simulation.meter.work_units = 0;
    let verdict = check_zone_certificate(
        ZoneCertificateRequest {
            simulation,
            model: "M",
            max_clocks: DEFAULT_MAX_CLOCKS,
            item: CertifiedItem::Refinement {
                refinement,
                abstract_package: &world.package,
                abstract_initial: &initial,
            },
        },
        &refinement.name().to_owned(),
        |name| CertificateItem::Refinement(name),
        &certificate,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    );
    assert!(
        matches!(
            verdict,
            Ok(ZoneCertificateVerdict::Stopped(
                qsl_replay::IncompleteCause::LimitReached {
                    limit: qsl_replay::ModelCheckLimit::EvaluationMeter { .. },
                    ..
                }
            ))
        ),
        "{verdict:?}"
    );
}
