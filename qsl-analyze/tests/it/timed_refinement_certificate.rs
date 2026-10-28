// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-430 "Certificate", FR-244 "Refinement form" and FR-245
//! "Refinement form" (TC-1170, TC-699, TC-700): a timed refinement's
//! holding safety half carries a zone certificate over the refinement
//! product, which the core's checker accepts before the item settles
//! `proved`.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{
    check_timed_refinement, settle_timed_safety, TimedRefinementRequest, TimedSafetyOutcome,
    ZoneCheckLimits,
};
use qsl_eval::simulation::model::DEFAULT_MAX_CLOCKS;
use qsl_replay::{
    check_zone_certificate, CertificateCheckLimits, CertificateItem, CertificateLocus,
    CertificateMember, CertificateRule, CertifiedItem, ProofBasis, Taken, TerminalValue,
    ZoneCertificate, ZoneCertificateRefusal, ZoneCertificateRequest, ZoneCertificateVerdict,
};
use qsl_replay::{CertificateRejection, Certification, FailedStep, RefinementFailure, Zone};
use quire_exact::Cancel;

use crate::timed_refinement::{dead_undo, dead_undo_with, safety, unit, CONTRACTS};
use crate::timed_world::{document, refining, refining_from, Refining, DENSE};

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
        verdict: None,
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
    let (rule, at) = rejected(check(&world, &certificate).expect("the check runs"));
    assert_eq!(rule, CertificateRule::StepFails);
    let CertificateLocus::ZoneEdge {
        edge: at_edge,
        verdict: Some(verdict),
    } = at
    else {
        panic!("StepFails names the edge and the failing verdict: {at:?}");
    };
    assert_eq!(at_edge, u64::try_from(into).expect("an index"));
    assert!(
        matches!(
            *verdict,
            FailedStep::Fails(RefinementFailure::StutterChanged { .. })
        ),
        "{verdict:?}"
    );
}

/// FR-245 "Refinement form": a `locked` node with a time-divergent
/// continuation is `BadState`, the rule that makes the locked region's
/// unjudged steps sound. AC-6's certificate checked against the same model
/// without the `Dead` invariant, whose `undo` post-state then diverges, is
/// `Rejected` `SuccessorMissing` with its zones as built, and with every
/// zone widened to the universe `BadState` at the locked node.
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn a_locked_node_with_a_divergent_continuation_is_a_bad_state() {
    let certificate = certificate(&dead_undo(true));
    let open = dead_undo_with(true, false);
    assert!(matches!(safety(&open), TimedSafetyOutcome::Violated { .. }));
    let (rule, _) = rejected(check(&open, &certificate).expect("the check runs"));
    assert_eq!(rule, CertificateRule::SuccessorMissing);
    let mut wide = certificate.clone();
    for node in &mut wide.nodes {
        node.zone = Zone::universe(node.zone.dim());
    }
    let entry = wide
        .nodes
        .iter()
        .position(|node| node.locked)
        .expect("a locked node");
    assert_eq!(
        rejected(check(&open, &wide).expect("the check runs")),
        (
            CertificateRule::BadState,
            CertificateLocus::ZoneNode {
                node: u64::try_from(entry).expect("an index")
            }
        )
    );
}

/// AC-6's model with `poke` (precondition `dead`), a stutter the locked
/// region takes: an edge from the locked entry back into it.
fn poked() -> Refining {
    let model = document(
        &["x"],
        &[
            ("send", &["inflight", "x"]),
            ("enter", &["late", "x"]),
            ("undo", &["inflight", "retried"]),
            ("poke", &["retried"]),
        ],
    );
    let contracts = "post ASend using v on A::Call::send { self.inflight }\n\
         pre MSend using v on M::Call::send { not self.retried and self.x >= 1 ms }\n\
         post MSent using v on M::Call::send { self.inflight and self.x = 0 ms }\n\
         pre MEnter using v on M::Call::enter { self.inflight and not self.retried and self.x >= 1 ms }\n\
         post MEntered using v on M::Call::enter { self.late and self.x = 0 ms }\n\
         pre MUndo using v on M::Call::undo { self.late and self.inflight and not self.retried and self.x <= 0 ms }\n\
         post MUndone using v on M::Call::undo { not self.inflight and self.retried }\n\
         pre MPoke using v on M::Call::poke { self.retried }\n\
         post MPoked using v on M::Call::poke { self.retried }\n\
         time invariant Dead when { self.retried } { self.x <= 0 ms }\n";
    let body = crate::timed_refinement::refinement("self.inflight")
        .replace("late = self.late", "late = false")
        .replace("retried = self.retried", "retried = false")
        .replace(
            "step M::Call::send -> A::Call::send(self);",
            "step M::Call::send -> A::Call::send(self);\n\
             step M::Call::enter -> stutter;\nstep M::Call::undo -> stutter;\n\
             step M::Call::poke -> stutter;",
        );
    refining(
        &crate::timed_refinement::req(),
        &model,
        &format!("{DENSE}{contracts}{body}"),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
}

/// FR-245 "Refinement form": an edge of the locked region takes concrete
/// steps only. `poke` from the locked entry back into it is accepted with
/// no `taken`; with a `taken` it is `Rejected` `PostDiffers` at that edge.
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn an_edge_of_the_locked_region_takes_no_abstract_step() {
    let world = poked();
    let certificate = certificate(&world);
    let entry = certificate
        .nodes
        .iter()
        .position(|node| node.locked)
        .expect("a locked node");
    let (inside, _) = certificate
        .edges
        .iter()
        .enumerate()
        .find(|(_, edge)| edge.from == entry)
        .expect("poke leaves the locked entry");
    assert_eq!(
        check(&world, &certificate),
        Ok(ZoneCertificateVerdict::Accepted)
    );
    let mut forged = certificate.clone();
    forged.refinement.as_mut().expect("the part").taken[inside] = Some(Taken::Stutter);
    assert_eq!(
        rejected(check(&world, &forged).expect("the check runs")),
        (CertificateRule::PostDiffers, edge(inside))
    );
}

/// FR-245-AC-8's second meter fixture, FR-141-AC-9's `HeavyEnq` shape:
/// `TReq` whose abstract `send` postcondition is its conjunction repeated
/// 150 times. Its certificate, built under an unlimited meter, checked
/// under `B`, the least per-evaluation budget under which the light `TReq`
/// holds, stops naming the evaluation meter and `B`, never `Rejected`.
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn a_heavy_refinement_certificate_checked_over_the_budget_stops() {
    let light = treq();
    let holds = |work: u64| {
        let [refinement] = light.package.graph().refinements() else {
            panic!("one refinement");
        };
        let mut concrete = light.concrete_request();
        concrete.meter.work_units = work;
        matches!(
            check_timed_refinement(
                TimedRefinementRequest {
                    refinement,
                    obligation: refinement.name().to_owned(),
                    concrete,
                    abstract_package: &light.package,
                    abstract_initial: vec![light.abstract_initial.clone()],
                    limits: ZoneCheckLimits::default(),
                },
                &Cancel::new(),
            )
            .expect("EN-6 takes the refinement")
            .safety,
            TimedSafetyOutcome::Holds { .. }
        )
    };
    let (mut low, mut high) = (1_u64, 1 << 20);
    while low < high {
        let middle = low + (high - low) / 2;
        if holds(middle) {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    let budget = low;
    let heavy_post = vec!["self.inflight"; 151].join(" and ");
    let world = crate::timed_world::refining(
        &crate::timed_refinement::req(),
        &crate::timed_refinement::treq(),
        &format!(
            "{DENSE}{}{}",
            CONTRACTS.replace(
                "post ASend using v on A::Call::send { self.inflight }",
                &format!("post ASend using v on A::Call::send {{ {heavy_post} }}"),
            ),
            crate::timed_refinement::refinement("self.inflight")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let certificate = certificate(&world);
    let [refinement] = world.package.graph().refinements() else {
        panic!("one refinement");
    };
    let initial = [world.abstract_initial.clone()];
    let mut simulation = world.concrete_request();
    simulation.meter.work_units = budget;
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
    assert_eq!(
        verdict,
        Ok(ZoneCertificateVerdict::Stopped(
            qsl_replay::IncompleteCause::LimitReached {
                limit: qsl_replay::ModelCheckLimit::EvaluationMeter {
                    counter: quire_exact::LimitKind::WorkUnits,
                },
                value: budget,
            }
        ))
    );
}

/// FR-245 `max_certificate_bytes` counts the refinement part: a budget the
/// `TReq` certificate fits stops the same certificate with its positions
/// repeated, before any work.
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn the_certificate_size_budget_counts_the_refinement_part() {
    let world = treq();
    let certificate = certificate(&world);
    let checked = |certificate: &ZoneCertificate<String>, bytes: u64| {
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
            CertificateCheckLimits {
                max_certificate_bytes: bytes,
                ..CertificateCheckLimits::default()
            },
            &Cancel::new(),
        )
    };
    let over = |verdict: &Result<ZoneCertificateVerdict, _>| {
        matches!(
            verdict,
            Ok(ZoneCertificateVerdict::Stopped(
                qsl_replay::IncompleteCause::LimitReached {
                    limit: qsl_replay::ModelCheckLimit::CertificateMaxBytes,
                    ..
                }
            ))
        )
    };
    let mut bytes = 64_u64;
    while over(&checked(&certificate, bytes)) {
        bytes *= 2;
    }
    let mut inflated = certificate.clone();
    let part = inflated.refinement.as_mut().expect("the part");
    let first = part.positions[0].clone();
    part.positions.extend(std::iter::repeat_n(
        first,
        usize::try_from(bytes).unwrap_or(usize::MAX),
    ));
    assert!(over(&checked(&inflated, bytes)));
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

/// `TReq` with `send` also requiring `not self.retried` and, when `dead`,
/// the time invariant `when self.retried { x <= 0 }`, over the initial
/// states `m0` (every Boolean false) and `m1` (`inflight` and `retried`
/// true), which fails `check_initial`.
fn two_initial(dead: bool) -> Refining {
    let contracts = CONTRACTS.replace(
        "pre MSend using v on M::Call::send { self.x >= 1 ms }",
        "pre MSend using v on M::Call::send { not self.retried and self.x >= 1 ms }",
    );
    let invariant = if dead {
        "time invariant Dead when { self.retried } { self.x <= 0 ms }\n"
    } else {
        ""
    };
    refining(
        &crate::timed_refinement::req(),
        &crate::timed_refinement::treq(),
        &format!(
            "{DENSE}{contracts}{invariant}{}",
            crate::timed_refinement::refinement("self.inflight")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
}

/// `world`'s concrete request over both its initial states.
fn both_initial(world: &Refining) -> qsl_eval::simulation::model::ModelSimulationRequest<'_> {
    let mut request = world.concrete_request();
    request.initial = vec![
        world.concrete_initial.clone(),
        world.retried_initial.clone(),
    ];
    request
}

/// FR-245 over both initial states of `world`.
fn check_both(
    world: &Refining,
    certificate: &ZoneCertificate<String>,
) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<String>> {
    let [refinement] = world.package.graph().refinements() else {
        panic!("one refinement");
    };
    let initial = [world.abstract_initial.clone()];
    check_zone_certificate(
        ZoneCertificateRequest {
            simulation: both_initial(world),
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

/// FR-430 TV-1 per initial state, FR-245 "Refinement form": of two initial
/// states, one fails `check_initial` and is time-locked, the other passes
/// and diverges. EN-6 holds, its certificate lists the failing one as a
/// `locked` initial node at the reserved index, and FR-245 accepts it. The
/// same certificate, its locked initial node's zone widened, checked where
/// that initial state's time can diverge, is `Rejected` `InitialMismatch`
/// at that node.
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn a_time_locked_failing_initial_state_is_a_locked_initial_node() {
    let world = two_initial(true);
    let [refinement] = world.package.graph().refinements() else {
        panic!("one refinement");
    };
    let outcome = check_timed_refinement(
        TimedRefinementRequest {
            refinement,
            obligation: refinement.name().to_owned(),
            concrete: both_initial(&world),
            abstract_package: &world.package,
            abstract_initial: vec![world.abstract_initial.clone()],
            limits: ZoneCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("EN-6 takes the refinement")
    .safety;
    let TimedSafetyOutcome::Holds { certificate, .. } = outcome else {
        panic!("the safety half holds: {outcome:?}");
    };
    let reserved = certificate
        .refinement
        .as_ref()
        .expect("the part")
        .positions
        .len();
    let locked = certificate
        .initial
        .iter()
        .copied()
        .find(|&node| certificate.nodes[node].locked)
        .expect("a locked initial node");
    assert_eq!(certificate.nodes[locked].automaton_state, reserved);
    assert_eq!(certificate.initial.len(), 2);
    assert_eq!(
        check_both(&world, &certificate),
        Ok(ZoneCertificateVerdict::Accepted)
    );
    let open = two_initial(false);
    let mut forged = (*certificate).clone();
    forged.nodes[locked].zone = Zone::universe(forged.nodes[locked].zone.dim());
    assert_eq!(
        rejected(check_both(&open, &forged).expect("the check runs")),
        (
            CertificateRule::InitialMismatch,
            CertificateLocus::ZoneNode {
                node: u64::try_from(locked).expect("an index")
            }
        )
    );
}

/// FR-245 "Refinement form", SR-1700002 (disposition 2c92949f): an edge of
/// the locked region into a positioned node is `PostDiffers` even with no
/// `taken`. The `poke` edge from the locked entry redirected to a copy of
/// the entry at position 0, unmarked, is rejected at that edge.
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn a_locked_region_edge_into_a_positioned_node_is_post_differs() {
    let world = poked();
    let certificate = certificate(&world);
    let entry = certificate
        .nodes
        .iter()
        .position(|node| node.locked)
        .expect("a locked node");
    let (inside, _) = certificate
        .edges
        .iter()
        .enumerate()
        .find(|(_, edge)| edge.from == entry)
        .expect("poke leaves the locked entry");
    let mut forged = certificate.clone();
    let mut copy = forged.nodes[entry].clone();
    copy.locked = false;
    copy.automaton_state = 0;
    forged.nodes.push(copy);
    forged.edges[inside].to = forged.nodes.len() - 1;
    assert_eq!(
        forged.refinement.as_ref().expect("the part").taken[inside],
        None
    );
    assert_eq!(
        rejected(check(&world, &forged).expect("the check runs")),
        (CertificateRule::PostDiffers, edge(inside))
    );
}
