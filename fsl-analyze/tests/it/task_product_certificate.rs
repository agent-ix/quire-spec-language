// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-418 (S4 of QSL-2240): the checker of a fixed-work task-product
//! certificate recomputes the member, the task automaton and the closed
//! refined-region quotient itself, and rejects each tampered certificate at
//! the member or node the vectors ZC-18..ZC-30 name.
//!
//! The certificates here are built from the checker's own quotient
//! (`explore_task_product`), so a mistake the checker's DBM and the
//! producer's would share is invisible to them: the independent producer
//! engine of S5 removes that common mode. What these tests do establish is
//! that the checker rejects every way a certificate can differ from the
//! recomputation.

use std::collections::{BTreeMap, VecDeque};

use filament_exact::{Cancel, Integer};
use fsl_eval::simulation::model::{
    TaskAutomaton, TaskProductSystem, TimedSystem, DEFAULT_MAX_CLOCKS,
};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_replay::{
    check_zone_certificate, explore_task_product, recomputed_member, CertEdge, CertNode,
    CertificateCheckLimits, CertificateItem, CertificateLocus, CertificateMember, CertificateRule,
    CertifiedItem, DivergenceWitness, DivergentTarget, QuotientIdentity, QuotientStop,
    TaskProductQuotient, Zone, ZoneCertificate, ZoneCertificateRefusal, ZoneCertificateRequest,
    ZoneCertificateVerdict,
};
use ix_trace_rs::trace;

use crate::timed_world::{document, world, World, DENSE};
use crate::zone_search::TIMED;

/// A source of one guard-free state: each task is released by its own
/// operation, which changes nothing, so the product's size is the
/// scheduler's.
pub(crate) fn world_for(set: &str, tasks: &[&str], declarations: &str) -> World {
    let names: Vec<String> = tasks.iter().map(|task| format!("fire_{task}")).collect();
    let operations: Vec<(&str, &[&str])> = names
        .iter()
        .map(|name| (name.as_str(), &["phase"][..]))
        .collect();
    let releases: String = tasks
        .iter()
        .map(|task| {
            format!(
                "post Fire_{task} using v on M::Call::fire_{task} {{ self.phase = pre(self.phase) }}\n\
                 release {set}.{task} on M::Call::fire_{task};\n"
            )
        })
        .collect();
    world(
        &document(&[], &operations),
        TIMED,
        &format!("{DENSE}{declarations}{releases}"),
    )
}

pub(crate) type Certificate = ZoneCertificate<OccurrenceKey>;

pub(crate) fn claim_of<'a>(world: &'a World, name: &str) -> &'a fsl_semantics::check::CheckedScheduleClaim {
    world
        .package
        .graph()
        .task_sets()
        .claims()
        .iter()
        .find(|claim| claim.name == name)
        .expect("the claim")
}

/// The entrywise upper bound of two canonical zones: a convex zone holding
/// both.
fn hull(left: &Zone, right: &Zone) -> Zone {
    let dim = left.dim();
    let mut cells = Vec::new();
    for i in 0..dim {
        for j in 0..dim {
            let a = left.bound(i, j).expect("bound");
            let b = right.bound(i, j).expect("bound");
            cells.push(match (a, b) {
                (Some(a), Some(b)) => Some(if (&a.0, !a.1) >= (&b.0, !b.1) { a } else { b }),
                _ => None,
            });
        }
    }
    Zone::from_matrix(dim, cells).expect("matrix")
}

pub(crate) struct Built {
    pub(crate) certificate: Certificate,
    pub(crate) quotient: TaskProductQuotient,
    /// The certificate edge of each distinct (node, identity, node) triple.
    edges: BTreeMap<(usize, usize, usize), usize>,
}

pub(crate) fn build(world: &World, claim_name: &str) -> Built {
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, claim_name).expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    let cancel = Cancel::new();
    let mut digits = 0_u32;
    let (scale, quotient) = loop {
        let scale = Integer::from(10_u64.pow(digits));
        match explore_task_product(&automaton, &scale, 100_000_000, &cancel) {
            Ok(quotient) => break (scale, quotient),
            Err(QuotientStop::Scale) => digits += 1,
            Err(other) => panic!("the quotient does not build: {other:?}"),
        }
    };
    println!(
        "quotient of {claim_name}: {} vertices, {} discrete, {} time",
        quotient.vertices.len(),
        quotient.discrete.len(),
        quotient.time.len()
    );
    // One node per product key: the hull of its regions, in discovery order.
    let mut node_of_key = BTreeMap::new();
    let mut nodes: Vec<CertNode> = Vec::new();
    let mut node_of_vertex = Vec::new();
    for vertex in &quotient.vertices {
        let node = *node_of_key.entry(vertex.key.clone()).or_insert_with(|| {
            nodes.push(CertNode {
                state_key: vertex.key.clone(),
                automaton_state: 0,
                zone: vertex.zone.clone(),
                locked: false,
                product: None,
            });
            nodes.len() - 1
        });
        nodes[node].zone = hull(&nodes[node].zone, &vertex.zone);
        node_of_vertex.push(node);
    }
    let mut edges = BTreeMap::new();
    let mut list = Vec::new();
    for step in &quotient.discrete {
        let triple = (
            node_of_vertex[step.from],
            step.identity,
            node_of_vertex[step.to],
        );
        edges.entry(triple).or_insert_with(|| {
            let mut edge = CertEdge {
                from: triple.0,
                transition: None,
                task_transition: None,
                part: None,
                horizon: None,
                to: triple.2,
                silent: None,
            };
            match &quotient.identities[step.identity] {
                QuotientIdentity::Source(transition) => edge.transition = Some(transition.clone()),
                QuotientIdentity::Template(template) => {
                    edge.task_transition = Some(template.clone());
                }
            }
            list.push(edge);
            list.len() - 1
        });
    }
    // Certificate edges by source node, in discovery order.
    let mut order: Vec<usize> = (0..list.len()).collect();
    order.sort_by_key(|&index| (list[index].from, index));
    let mut renumber = vec![0; list.len()];
    for (new, &old) in order.iter().enumerate() {
        renumber[old] = new;
    }
    let sorted: Vec<CertEdge> = order.iter().map(|&old| list[old].clone()).collect();
    for index in edges.values_mut() {
        *index = renumber[*index];
    }
    let initials = quotient
        .initials
        .iter()
        .map(|initial| node_of_key[&initial.key])
        .collect();
    let member = recomputed_member(&automaton, &scale, &quotient).expect("a finite product");
    let occurrence = claim_of(world, claim_name)
        .occurrence()
        .expect("the claim's occurrence");
    let mut certificate = ZoneCertificate {
        obligation: occurrence,
        scale,
        lu_bounds: fsl_replay::LuBounds {
            lower: Vec::new(),
            upper: Vec::new(),
        },
        nodes,
        edges: sorted,
        initials,
        components: None,
        non_zeno: None,
        divergence: None,
        admitted: None,
        state_graph: None,
        refinement: None,
        task_product: Some(member),
    };
    let built = Built {
        certificate: certificate.clone(),
        quotient,
        edges,
    };
    certificate.admitted = Some(admitted(&built, &node_of_vertex));
    Built {
        certificate,
        ..built
    }
}

/// The TD-6 witness: a path to a vertex where time idles forever, or to a
/// closed walk with a positive delay, over the quotient.
fn admitted(built: &Built, node_of_vertex: &[usize]) -> DivergenceWitness {
    let quotient = &built.quotient;
    let count = quotient.vertices.len();
    // (to, certificate edge or None for a delay)
    let mut out: Vec<Vec<(usize, Option<usize>)>> = vec![Vec::new(); count];
    for step in &quotient.discrete {
        let triple = (
            node_of_vertex[step.from],
            step.identity,
            node_of_vertex[step.to],
        );
        out[step.from].push((step.to, Some(built.edges[&triple])));
    }
    for &(from, to) in &quotient.time {
        out[from].push((to, None));
    }
    let initials: Vec<usize> = quotient
        .initials
        .iter()
        .flat_map(|initial| initial.vertices.iter().copied())
        .collect();
    let reach = |from: &[usize]| {
        let mut parent: Vec<Option<(usize, Option<usize>)>> = vec![None; count];
        let mut seen = vec![false; count];
        let mut queue = VecDeque::new();
        for &vertex in from {
            seen[vertex] = true;
            queue.push_back(vertex);
        }
        while let Some(vertex) = queue.pop_front() {
            for &(to, edge) in &out[vertex] {
                if !seen[to] {
                    seen[to] = true;
                    parent[to] = Some((vertex, edge));
                    queue.push_back(to);
                }
            }
        }
        (seen, parent)
    };
    let path_to = |parent: &[Option<(usize, Option<usize>)>], mut vertex: usize| {
        let mut path = Vec::new();
        while let Some((before, edge)) = parent[vertex] {
            path.extend(edge);
            vertex = before;
        }
        path.reverse();
        (vertex, path)
    };
    let (seen, parent) = reach(&initials);
    let start_node = |vertex: usize| {
        let (root, _) = path_to(&parent, vertex);
        node_of_vertex[root]
    };
    // A vertex where time idles forever.
    if let Some(idle) =
        (0..count).find(|&vertex| seen[vertex] && quotient.time.contains(&(vertex, vertex)))
    {
        let (_, path) = path_to(&parent, idle);
        return DivergenceWitness {
            start: start_node(idle),
            path,
            target: DivergentTarget::Quiescent(node_of_vertex[idle]),
        };
    }
    // A closed walk through a delay: `to` back to `from` over any edges.
    for &(from, to) in &quotient.time {
        if !seen[from] {
            continue;
        }
        let (back, back_parent) = reach(&[to]);
        if !back[from] {
            continue;
        }
        let (_, cycle) = path_to(&back_parent, from);
        let (_, path) = path_to(&parent, to);
        if cycle.is_empty() {
            continue;
        }
        return DivergenceWitness {
            start: start_node(to),
            path,
            target: DivergentTarget::Cycle(cycle),
        };
    }
    panic!(
        "the product admits no time-divergent behaviour: {} vertices, {} discrete, {} time, {} initials",
        count,
        quotient.discrete.len(),
        quotient.time.len(),
        quotient.initials.len()
    );
}

pub(crate) fn verdict(
    world: &World,
    claim_name: &str,
    certificate: &Certificate,
    limits: CertificateCheckLimits,
) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<OccurrenceKey>> {
    verdict_with(world, claim_name, certificate, limits, &Cancel::new())
}

fn verdict_with(
    world: &World,
    claim_name: &str,
    certificate: &Certificate,
    limits: CertificateCheckLimits,
    cancel: &Cancel,
) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<OccurrenceKey>> {
    check_zone_certificate(
        ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            max_clocks: DEFAULT_MAX_CLOCKS,
            item: CertifiedItem::Schedulability {
                claim: claim_of(world, claim_name),
            },
        },
        &certificate.obligation,
        |obligation: &OccurrenceKey| CertificateItem::Schedulability(obligation),
        certificate,
        limits,
        cancel,
    )
}

fn node_at(index: usize) -> CertificateLocus {
    CertificateLocus::ZoneNode { node: index as u64 }
}

const SOLO: &str = "taskset S {\n\
     task a periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
     }\n\
     schedulable SFp on S under fixed_priority;\n\
     schedulable SEdf on S under edf;\n";

pub(crate) fn solo() -> World {
    world_for("S", &["a"], SOLO)
}

/// Two tasks declared `b` then `a` with equal deadlines and no priorities.
pub(crate) fn tied() -> World {
    world_for(
        "E",
        &["b", "a"],
        "taskset E {\n\
         task b periodic 6 ms deadline 6 ms bcet 1 ms wcet 1 ms lo;\n\
         task a periodic 6 ms deadline 6 ms bcet 2 ms wcet 2 ms lo;\n\
         }\n\
         schedulable EEdf on E under edf;\n",
    )
}

/// FR-418-AC-13: the checker accepts the periodic task certificate it
/// recomputes, closed modulo the rank/slot of completed jobs, at the default
/// work limit; each node is a union of refined regions the checker
/// established itself.
#[trace("TC-371", "FR-418-AC-13")]
#[test]
fn the_recomputed_periodic_certificate_is_accepted() {
    for claim in ["SFp", "SEdf"] {
        let world = solo();
        let built = build(&world, claim);
        assert_eq!(
            verdict(
                &world,
                claim,
                &built.certificate,
                CertificateCheckLimits::default()
            ),
            Ok(ZoneCertificateVerdict::Accepted),
            "{claim}"
        );
    }
}

fn edit(
    world: &World,
    claim: &str,
    change: impl FnOnce(&mut Certificate),
) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<OccurrenceKey>> {
    let mut certificate = build(world, claim).certificate;
    change(&mut certificate);
    verdict(
        world,
        claim,
        &certificate,
        CertificateCheckLimits::default(),
    )
}

pub(crate) fn rejected(
    result: Result<ZoneCertificateVerdict, ZoneCertificateRefusal<OccurrenceKey>>,
) -> (CertificateRule, CertificateLocus) {
    match result {
        Ok(ZoneCertificateVerdict::Rejected(rejection)) => (rejection.rule, rejection.at),
        other => panic!("expected a rejection, got {other:?}"),
    }
}

/// A fixed-priority pair where `q`, released later with the earlier
/// priority, preempts `p`: its service clock is bounded-subtracted.
pub(crate) fn preempting() -> World {
    world_for(
        "Q",
        &["p", "q"],
        "taskset Q {\n\
         task p periodic 10 ms deadline 10 ms bcet 4 ms wcet 4 ms priority 2 lo;\n\
         task q sporadic 5 ms deadline 3 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
         }\n\
         schedulable QFp on Q under fixed_priority;\n\
         schedulable QEdf on Q under edf;\n",
    )
}

fn malformed_member() -> (CertificateRule, CertificateLocus) {
    (
        CertificateRule::Malformed,
        CertificateLocus::CertificateMember(CertificateMember::TaskProduct),
    )
}

fn big(value: u64) -> Integer {
    Integer::from(value)
}

/// FR-418-AC-12 and AC-13 (member recompute): every member the checker
/// recomputes must match, so each way of changing one is malformed at
/// `task_product`.
#[test]
fn a_member_that_differs_from_its_recomputation_is_malformed() {
    type Tamper = (&'static str, &'static str, Box<dyn Fn(&mut Certificate)>);
    let tampers: Vec<Tamper> = vec![
        (
            "max_jobs + 1",
            "SFp",
            Box::new(|c| {
                c.task_product.as_mut().unwrap().queue_bounds[0].max_jobs += 1;
            }),
        ),
        (
            "max_jobs - 1",
            "SFp",
            Box::new(|c| {
                c.task_product.as_mut().unwrap().queue_bounds[0].max_jobs -= 1;
            }),
        ),
        (
            "cap != sum",
            "SFp",
            Box::new(|c| {
                c.task_product.as_mut().unwrap().time_progress.live_job_cap += 1;
            }),
        ),
        (
            "work lower != upper",
            "SFp",
            Box::new(|c| {
                let row = &mut c.task_product.as_mut().unwrap().queue_bounds[0];
                row.work_lower = crate::zone_search::exact(1, 4000);
            }),
        ),
        (
            "a ceiling changed",
            "SFp",
            Box::new(|c| {
                c.task_product.as_mut().unwrap().clock_ceilings[0].maximum = big(9_999_999);
            }),
        ),
    ];
    for (name, claim, change) in tampers {
        assert_eq!(
            rejected(edit(&solo(), claim, change)),
            malformed_member(),
            "{name}"
        );
    }
    let diagonals = |c: &Certificate| c.task_product.as_ref().unwrap().diagonal_constraints.len();
    assert!(
        diagonals(&build(&preempting(), "QEdf").certificate) > 0,
        "EDF over two tasks has diagonals"
    );
    assert_eq!(
        rejected(edit(&preempting(), "QEdf", |c| {
            c.task_product.as_mut().unwrap().diagonal_constraints.pop();
        })),
        malformed_member(),
        "a diagonal dropped"
    );
    assert_eq!(
        rejected(edit(&preempting(), "QEdf", |c| {
            c.task_product.as_mut().unwrap().diagonal_constraints[0].constant = big(9_999_999);
        })),
        malformed_member(),
        "a diagonal constant changed"
    );
    // The subtraction table: a preempting fixed-priority pair has a row.
    let world = preempting();
    let rows = |c: &Certificate| c.task_product.as_ref().unwrap().subtractions.len();
    assert_eq!(rows(&build(&world, "QFp").certificate), 1);
    assert_eq!(
        rejected(edit(&world, "QFp", |c| {
            c.task_product.as_mut().unwrap().subtractions[0].constant = big(9_999_999);
        })),
        malformed_member(),
        "a subtraction changed"
    );
    assert_eq!(
        rejected(edit(&world, "QFp", |c| {
            c.task_product.as_mut().unwrap().subtractions.clear();
        })),
        malformed_member(),
        "a subtraction dropped"
    );
}

fn template_edge(certificate: &Certificate, action: fsl_replay::TaskTransitionAction) -> usize {
    certificate
        .edges
        .iter()
        .position(|edge| {
            edge.task_transition
                .as_ref()
                .is_some_and(|template| template.action == action)
        })
        .expect("an edge of that action")
}

/// FR-418-AC-12 (matcher): an edge whose label the recomputation does not
/// produce leaves its source node without the recomputed successor.
#[trace("TC-371", "FR-418-AC-12")]
#[test]
fn a_forged_template_leaves_the_successor_missing() {
    let world = solo();
    let built = build(&world, "SFp");
    let dispatch = template_edge(
        &built.certificate,
        fsl_replay::TaskTransitionAction::Dispatch,
    );
    let from = built.certificate.edges[dispatch].from;
    // A forged priority.
    let forged = edit(&world, "SFp", |c| {
        c.edges[dispatch].task_transition.as_mut().unwrap().priority = Some(99);
    });
    assert_eq!(
        rejected(forged),
        (CertificateRule::SuccessorMissing, node_at(from))
    );
    // A slot not bound to the live job (ZC-25's shape).
    let slot = edit(&world, "SFp", |c| {
        c.edges[dispatch].task_transition.as_mut().unwrap().slot = 1;
    });
    assert_eq!(
        rejected(slot),
        (CertificateRule::SuccessorMissing, node_at(from))
    );
}

/// FR-418-AC-13 (matcher): an activation whose rank is beyond the
/// recomputed live-job range is not produced by the recomputation (ZC-24).
#[trace("TC-371", "FR-418-AC-13")]
#[test]
fn an_out_of_range_activation_rank_leaves_the_successor_missing() {
    let world = solo();
    let built = build(&world, "SFp");
    let activate = template_edge(
        &built.certificate,
        fsl_replay::TaskTransitionAction::Activate,
    );
    let from = built.certificate.edges[activate].from;
    let result = edit(&world, "SFp", |c| {
        c.edges[activate].task_transition.as_mut().unwrap().job_rank = 7;
    });
    assert_eq!(
        rejected(result),
        (CertificateRule::SuccessorMissing, node_at(from))
    );
}

/// The certificate is canonical (ruling U5): an edge or node the
/// recomputation does not produce is rejected, and a removed edge leaves
/// its node's successor missing.
#[trace("TC-371", "FR-418-AC-19")]
#[test]
fn the_certificate_is_the_reachable_closure() {
    let world = solo();
    let built = build(&world, "SFp");
    let count = built.certificate.edges.len();
    let extra = edit(&world, "SFp", |c| {
        let copy = c.edges[0].clone();
        c.edges.push(copy);
    });
    assert_eq!(
        rejected(extra),
        (
            CertificateRule::ExtraEdge,
            CertificateLocus::ZoneEdge { edge: count as u64 }
        )
    );
    let nodes = built.certificate.nodes.len();
    let unreached = edit(&world, "SFp", |c| {
        let copy = c.nodes[0].clone();
        c.nodes.push(copy);
    });
    assert_eq!(
        rejected(unreached),
        (CertificateRule::UnreachedNode, node_at(nodes))
    );
    let removed_from = built.certificate.edges[0].from;
    let removed = edit(&world, "SFp", |c| {
        let admitted = c.admitted.as_mut().unwrap();
        assert!(
            !admitted.path.contains(&0),
            "the witness does not use edge 0"
        );
        for edge in &mut admitted.path {
            *edge -= usize::from(*edge > 0);
        }
        if let DivergentTarget::Cycle(cycle) = &mut admitted.target {
            for edge in cycle {
                *edge -= usize::from(*edge > 0);
            }
        }
        c.edges.remove(0);
    });
    assert_eq!(
        rejected(removed),
        (CertificateRule::SuccessorMissing, node_at(removed_from))
    );
}

/// Two releases at one instant are two source edges, fired in either order;
/// removing one interleaving leaves the node's successor missing.
#[trace("TC-371", "FR-418-AC-13")]
#[test]
fn one_interleaving_of_simultaneous_releases_cannot_be_removed() {
    let world = tied();
    let built = build(&world, "EEdf");
    let releases: Vec<usize> = built
        .certificate
        .edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| {
            edge.task_transition
                .as_ref()
                .is_some_and(|t| t.action == fsl_replay::TaskTransitionAction::Activate)
        })
        .map(|(index, _)| index)
        .collect();
    let node = built.certificate.edges[releases[0]].from;
    let both: Vec<usize> = releases
        .iter()
        .copied()
        .filter(|&index| built.certificate.edges[index].from == node)
        .collect();
    assert_eq!(
        both.len(),
        2,
        "both tasks can release first at the initial node"
    );
    let last = *both.last().expect("two");
    let result = edit(&world, "EEdf", |c| {
        let admitted = c.admitted.as_mut().unwrap();
        for edge in &mut admitted.path {
            *edge -= usize::from(*edge > last);
        }
        if let DivergentTarget::Cycle(cycle) = &mut admitted.target {
            for edge in cycle {
                *edge -= usize::from(*edge > last);
            }
        }
        c.edges.remove(last);
    });
    assert_eq!(
        rejected(result),
        (CertificateRule::SuccessorMissing, node_at(node))
    );
}

/// A node that does not cover the initial zero valuation is
/// `initial-missing`; a successor outside its target's zone is
/// `successor-missing` (two DBM columns swapped).
#[trace("TC-371", "FR-418-AC-2")]
#[test]
fn a_node_that_does_not_cover_is_rejected() {
    let world = solo();
    let built = build(&world, "SFp");
    let initial = built.certificate.initials[0];
    let result = edit(&world, "SFp", |c| {
        let zone = &mut c.nodes[initial].zone;
        zone.constrain(0, 1, &Integer::from(-1_i64), false)
            .expect("constrain");
    });
    assert_eq!(
        rejected(result),
        (
            CertificateRule::InitialMissing,
            CertificateLocus::ModelState(built.quotient.initials[0].key.clone())
        )
    );
    // Two clock columns swapped in the last node's DBM.
    let last = built.certificate.nodes.len() - 1;
    let swapped = edit(&world, "SFp", |c| {
        let zone = &c.nodes[last].zone;
        let dim = zone.dim();
        let mut cells = Vec::new();
        for i in 0..dim {
            for j in 0..dim {
                let (i, j) = (
                    if i == 1 {
                        2
                    } else if i == 2 {
                        1
                    } else {
                        i
                    },
                    if j == 1 {
                        2
                    } else if j == 2 {
                        1
                    } else {
                        j
                    },
                );
                cells.push(zone.bound(i, j).expect("bound"));
            }
        }
        c.nodes[last].zone = Zone::from_matrix(dim, cells).expect("matrix");
    });
    assert_eq!(
        rejected(swapped),
        (CertificateRule::SuccessorMissing, node_at(2))
    );
}

/// ZC-18's ready-order tamper: the queue order is part of the node key, so
/// a node whose ready order is reversed is not the recomputed successor.
#[trace("TC-371", "FR-418-AC-12")]
#[test]
fn a_reversed_ready_order_is_not_the_recomputed_key() {
    let world = tied();
    let built = build(&world, "EEdf");
    let target = built
        .certificate
        .nodes
        .iter()
        .position(|node| {
            String::from_utf8_lossy(node.state_key.as_bytes()).contains("|q:0.0:1.0l.")
        })
        .expect("a node with two ready jobs");
    let result = edit(&world, "EEdf", |c| {
        let text = String::from_utf8_lossy(c.nodes[target].state_key.as_bytes())
            .replace("|q:0.0:1.0l.", "|q:1.0:0.0l.");
        c.nodes[target].state_key = text.into_bytes().into();
    });
    let (rule, _) = rejected(result);
    assert_eq!(rule, CertificateRule::SuccessorMissing);
}

/// ZC-18: for two equal-deadline jobs whose tasks declare no priorities, EDF
/// is derived from the bound claim and ties break by task name (`a` before
/// `b`) whatever the declaration order (`b`, `a`); the reversed first
/// dispatch edge is rejected, and a `policy` member is not a
/// `TaskProductCertificate`.
#[trace("TC-371", "FR-418-AC-12")]
#[test]
fn an_edf_tie_dispatches_the_canonically_first_task() {
    let world = tied();
    let built = build(&world, "EEdf");
    assert_eq!(
        verdict(
            &world,
            "EEdf",
            &built.certificate,
            CertificateCheckLimits::default()
        ),
        Ok(ZoneCertificateVerdict::Accepted)
    );
    let dispatches: Vec<&CertEdge> = built
        .certificate
        .edges
        .iter()
        .filter(|edge| {
            edge.task_transition
                .as_ref()
                .is_some_and(|t| t.action == fsl_replay::TaskTransitionAction::Dispatch)
        })
        .collect();
    assert!(dispatches.iter().all(|edge| edge
        .task_transition
        .as_ref()
        .unwrap()
        .priority
        .is_none()));
    // At the node holding both ready jobs the only dispatch is `a`.
    let both = built
        .certificate
        .nodes
        .iter()
        .position(|node| {
            String::from_utf8_lossy(node.state_key.as_bytes()).contains("|q:0.0:1.0l.")
        })
        .expect("a node with two ready jobs");
    let at_tie: Vec<&str> = built
        .certificate
        .edges
        .iter()
        .filter(|edge| edge.from == both)
        .filter_map(|edge| edge.task_transition.as_ref())
        .filter(|t| t.action == fsl_replay::TaskTransitionAction::Dispatch)
        .map(|t| t.task.as_str())
        .collect();
    assert_eq!(at_tie, ["a"]);
    let index = built
        .certificate
        .edges
        .iter()
        .position(|edge| {
            edge.from == both
                && edge
                    .task_transition
                    .as_ref()
                    .is_some_and(|t| t.action == fsl_replay::TaskTransitionAction::Dispatch)
        })
        .expect("the tie dispatch");
    let reversed = edit(&world, "EEdf", |c| {
        c.edges[index].task_transition.as_mut().unwrap().task = "b".to_owned();
    });
    assert_eq!(
        rejected(reversed),
        (CertificateRule::SuccessorMissing, node_at(both))
    );
    // A `policy` member is not a task-product certificate.
    let mut wire = built
        .certificate
        .task_product
        .as_ref()
        .unwrap()
        .to_wire()
        .expect("wire");
    wire.as_object_mut()
        .unwrap()
        .insert("policy".to_owned(), serde_json::json!("fixed-priority"));
    assert!(fsl_replay::TaskProductCertificate::from_wire(&wire).is_err());
}

/// ADR-026 TD-6 on the quotient: a `locked` node that reaches time
/// divergence is `bad-state`; an `admitted` witness that idles where time
/// cannot pass forever fails.
#[trace("TC-371", "FR-418-AC-2")]
#[test]
fn divergence_and_the_admitted_witness_are_replayed_on_the_quotient() {
    let world = solo();
    let built = build(&world, "SFp");
    let locked = edit(&world, "SFp", |c| c.nodes[0].locked = true);
    assert_eq!(rejected(locked), (CertificateRule::BadState, node_at(0)));
    let release = built
        .certificate
        .edges
        .iter()
        .position(|edge| edge.from == built.certificate.initials[0] && edge.to != edge.from)
        .expect("the first step");
    let to = built.certificate.edges[release].to;
    let idles_nowhere = edit(&world, "SFp", |c| {
        c.admitted = Some(DivergenceWitness {
            start: c.initials[0],
            path: vec![release],
            target: DivergentTarget::Quiescent(to),
        });
    });
    assert_eq!(
        rejected(idles_nowhere),
        (CertificateRule::WitnessFails, node_at(to))
    );
}

/// A work limit below the graph's size settles
/// `incomplete` naming `max_check_work`, never an acceptance.
#[trace("TC-371", "FR-418-AC-20")]
#[test]
fn a_work_limit_below_the_graph_is_incomplete() {
    let world = solo();
    let built = build(&world, "SFp");
    let limits = CertificateCheckLimits {
        max_check_work: 1,
        ..CertificateCheckLimits::default()
    };
    assert_eq!(
        verdict(&world, "SFp", &built.certificate, limits),
        Ok(ZoneCertificateVerdict::Stopped(
            fsl_replay::IncompleteCause::LimitReached {
                limit: fsl_replay::ModelCheckLimit::ZoneMaxCheckWork,
                value: 1,
            }
        ))
    );
}

/// Ruling U6: a dual-WCET set with a mode switch is not covered by the
/// closed finite product, so its check settles `certificate-unchecked`
/// whatever the certificate says.
#[trace("TC-371", "FR-418-AC-21")]
#[test]
fn a_mixed_criticality_set_is_certificate_unchecked() {
    let world = world_for(
        "D",
        &["command", "worker"],
        "taskset D {\n\
         task command periodic 4 ms deadline 4 ms bcet 2 ms wcet 2 ms priority 1 hi wcet_hi 3 ms;\n\
         task worker periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 2 lo;\n\
         }\n\
         mode D hi on M::Call::fire_command;\n\
         schedulable DFp on D under fixed_priority;\n",
    );
    let mut certificate = build(&solo(), "SFp").certificate;
    certificate.obligation = claim_of(&world, "DFp").occurrence().expect("occurrence");
    assert_eq!(
        verdict(
            &world,
            "DFp",
            &certificate,
            CertificateCheckLimits::default()
        ),
        Ok(ZoneCertificateVerdict::Stopped(
            fsl_replay::IncompleteCause::CertificateUnchecked
        ))
    );
}

/// A source with no operations and one task set that declares no release.
fn no_release() -> World {
    world(
        &document(&[], &[]),
        TIMED,
        &format!(
            "{DENSE}taskset N {{\n\
             task a periodic 4 ms deadline 4 ms bcet 1 ms wcet 2 ms priority 1 lo;\n\
             }}\n\
             schedulable NFp on N under fixed_priority;\n"
        ),
    )
}

/// The one-node, no-edge `closed-no-release-product/v1` certificate of
/// `claim_name`, its node the claim's initial source state. With
/// `releasable` the source has the release operation, so the closure is
/// false (ZC-28's shape).
fn no_release_certificate(world: &World, claim_name: &str) -> Certificate {
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, claim_name).expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    let (location, _) = automaton.initial().expect("initial").remove(0);
    let rows = fsl_replay::TaskQueueBound::from_checked_task_set(product.plan().task_set)
        .expect("queue bounds");
    let source_clocks = automaton.source_clocks();
    let dim = source_clocks + 1;
    ZoneCertificate {
        obligation: claim_of(world, claim_name)
            .occurrence()
            .expect("occurrence"),
        scale: Integer::from(1_u64),
        lu_bounds: fsl_replay::LuBounds {
            lower: vec![None; source_clocks],
            upper: vec![None; source_clocks],
        },
        nodes: vec![CertNode {
            state_key: location.source_key().clone(),
            automaton_state: 0,
            zone: Zone::zero(dim),
            locked: false,
            product: None,
        }],
        edges: Vec::new(),
        initials: vec![0],
        components: None,
        non_zeno: None,
        divergence: None,
        admitted: Some(DivergenceWitness {
            start: 0,
            path: Vec::new(),
            target: DivergentTarget::Quiescent(0),
        }),
        state_graph: None,
        refinement: None,
        task_product: Some(
            fsl_replay::TaskProductCertificate::closed_no_release_product(rows)
                .expect("a no-release member"),
        ),
    }
}

/// ZC-26..ZC-29 and the ZC-28 shape: a closed source product with no
/// reachable checked task release is accepted; a positive cap or empty
/// `queue_bounds` is malformed at `task_product`; a reachable release is
/// `successor-missing` at the node.
#[trace("TC-371", "FR-418-AC-14")]
#[test]
fn a_no_release_closure_is_accepted_only_when_no_release_is_reachable() {
    let world = no_release();
    let certificate = no_release_certificate(&world, "NFp");
    assert_eq!(
        verdict(
            &world,
            "NFp",
            &certificate,
            CertificateCheckLimits::default()
        ),
        Ok(ZoneCertificateVerdict::Accepted),
        "ZC-26"
    );
    let mut capped = certificate.clone();
    capped
        .task_product
        .as_mut()
        .unwrap()
        .time_progress
        .live_job_cap = 1;
    assert_eq!(
        rejected(verdict(
            &world,
            "NFp",
            &capped,
            CertificateCheckLimits::default()
        )),
        malformed_member(),
        "ZC-27"
    );
    let mut empty = certificate.clone();
    empty.task_product.as_mut().unwrap().queue_bounds.clear();
    assert_eq!(
        rejected(verdict(
            &world,
            "NFp",
            &empty,
            CertificateCheckLimits::default()
        )),
        malformed_member(),
        "ZC-29"
    );
    // The same closure over a source that binds a release (ZC-28).
    let releasing = solo();
    let wrong = no_release_certificate(&releasing, "SFp");
    assert_eq!(
        rejected(verdict(
            &releasing,
            "SFp",
            &wrong,
            CertificateCheckLimits::default()
        )),
        (CertificateRule::SuccessorMissing, node_at(0)),
        "ZC-28"
    );
}

/// The nodes of `built` whose state key a missed vertex of the quotient
/// carries.
fn missed_nodes(built: &Built) -> Vec<usize> {
    let keys: Vec<_> = built
        .quotient
        .vertices
        .iter()
        .filter(|vertex| vertex.missed)
        .map(|vertex| &vertex.key)
        .collect();
    built
        .certificate
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| keys.contains(&&node.state_key))
        .map(|(index, _)| index)
        .collect()
}

/// Two fixed-priority tasks at utilisation 2/3 + 2/3: the lower priority
/// task misses its deadline.
pub(crate) fn over_utilised() -> World {
    world_for(
        "O",
        &["p", "q"],
        "taskset O {\n\
         task p periodic 3 ms deadline 3 ms bcet 2 ms wcet 2 ms priority 1 lo;\n\
         task q periodic 3 ms deadline 3 ms bcet 2 ms wcet 2 ms priority 2 lo;\n\
         }\n\
         schedulable OFp on O under fixed_priority;\n",
    )
}

/// A reachable deadline miss is `bad-state` when its node is not locked
/// (the `here.missed` rule), and `certificate-unchecked` when the node is
/// locked on a real miss node.
#[trace("TC-371", "FR-418-AC-2")]
#[test]
fn a_reachable_deadline_miss_is_bad_state_or_unchecked_when_locked() {
    let world = over_utilised();
    let built = build(&world, "OFp");
    let missed = missed_nodes(&built);
    assert!(!missed.is_empty(), "the over-utilised pair misses");
    let (rule, at) = rejected(verdict(
        &world,
        "OFp",
        &built.certificate,
        CertificateCheckLimits::default(),
    ));
    assert_eq!(rule, CertificateRule::BadState);
    assert!(
        missed.iter().any(|&node| at == node_at(node)),
        "{at:?} is not a miss node"
    );
    let mut locked = built.certificate.clone();
    for &node in &missed {
        locked.nodes[node].locked = true;
    }
    assert_eq!(
        verdict(&world, "OFp", &locked, CertificateCheckLimits::default()),
        Ok(ZoneCertificateVerdict::Stopped(
            fsl_replay::IncompleteCause::CertificateUnchecked
        ))
    );
}

/// A source with a clock `x` that is never reset (held to 1 ms) and a clock
/// `y` that a `tick` operation resets whenever it is positive: time can
/// pass only by ever smaller steps summing to less than 1 ms (Zeno).
fn zeno() -> World {
    zeno_with(false)
}

/// [`zeno`] with a `park` operation after which the cap no longer holds, so
/// time can pass forever and the product has a real witness.
fn parked() -> World {
    zeno_with(true)
}

fn zeno_with(park: bool) -> World {
    let all: [(&str, &[&str]); 3] = [
        ("fire_a", &["phase"]),
        ("tick", &["y"]),
        ("park", &["phase"]),
    ];
    let operations = &all[..if park { 3 } else { 2 }];
    let parking = if park {
        "pre Park using v on M::Call::park { self.phase = 0 }\n\
         post Parked using v on M::Call::park { self.phase = 1 }\n"
    } else {
        ""
    };
    world(
        &document(&["x", "y"], operations),
        TIMED,
        &format!(
            "{DENSE}taskset Z {{\n\
             task a periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
             }}\n\
             schedulable ZFp on Z under fixed_priority;\n\
             post Fire_a using v on M::Call::fire_a {{ self.phase = pre(self.phase) }}\n\
             release Z.a on M::Call::fire_a;\n\
             pre Tick using v on M::Call::tick {{ self.y > 0 ms }}\n\
             post Ticked using v on M::Call::tick {{ self.y = 0 ms }}\n\
             time invariant Cap when {{ self.phase = 0 }} {{ self.x <= 1 ms }}\n\
             {parking}"
        ),
    )
}

/// A cycle of region steps that never advances time by a fixed amount
/// (the Zeno `tick` cycle) is not an `admitted` witness: the rule is the
/// ordinary checker's `advancing_cycle`, not any quotient cycle through a
/// positive delay.
#[trace("TC-371", "FR-418-AC-2")]
#[test]
fn a_zeno_region_cycle_is_not_an_admitted_witness() {
    let world = zeno();
    let built = build(&world, "ZFp");
    let admitted = built.certificate.admitted.as_ref().expect("a witness");
    assert!(
        matches!(admitted.target, DivergentTarget::Cycle(_)),
        "the only divergence the product has is the Zeno cycle: {:?}",
        admitted.target
    );
    let (rule, _) = rejected(verdict(
        &world,
        "ZFp",
        &built.certificate,
        CertificateCheckLimits::default(),
    ));
    assert_eq!(rule, CertificateRule::WitnessFails);
}

/// [`no_release`]'s task set, with a source that binds `a`'s release to an
/// operation: the one difference between the two sources.
fn releasable() -> World {
    world_for(
        "N",
        &["a"],
        "taskset N {\n\
         task a periodic 4 ms deadline 4 ms bcet 1 ms wcet 2 ms priority 1 lo;\n\
         }\n\
         schedulable NFp on N under fixed_priority;\n",
    )
}

/// ZC-28 made discriminating: the certificate over the release-free source
/// is accepted, and the same certificate (equal but for the claim it names)
/// over the source that binds the release is `successor-missing`, so the
/// reachable release is the only defect.
#[trace("TC-371", "FR-418-AC-14")]
#[test]
fn a_reachable_release_is_the_only_defect_of_a_no_release_closure() {
    let free = no_release();
    let bound = releasable();
    let accepted = no_release_certificate(&free, "NFp");
    let mut refused = no_release_certificate(&bound, "NFp");
    // The release operation's own source edge, covering its successor (the
    // same node), so that no ordinary rule can reject the certificate.
    let donor = build(&solo(), "SFp").certificate;
    let mut template =
        donor.edges[template_edge(&donor, fsl_replay::TaskTransitionAction::Activate)].clone();
    let release = template
        .task_transition
        .as_ref()
        .expect("a template")
        .release
        .clone();
    refused.edges.push(CertEdge {
        from: 0,
        transition: Some(release),
        task_transition: None,
        part: Some(0),
        horizon: None,
        to: 0,
        silent: None,
    });
    assert_eq!(
        verdict(&free, "NFp", &accepted, CertificateCheckLimits::default()),
        Ok(ZoneCertificateVerdict::Accepted)
    );
    assert_eq!(accepted.nodes[0].state_key, refused.nodes[0].state_key);
    assert_eq!(accepted.nodes[0].zone, refused.nodes[0].zone);
    assert_eq!(accepted.task_product, refused.task_product);
    assert_eq!(
        rejected(verdict(
            &bound,
            "NFp",
            &refused,
            CertificateCheckLimits::default()
        )),
        (CertificateRule::SuccessorMissing, node_at(0))
    );
    // A task-transition edge is not part of this method.
    template.from = 0;
    template.to = 0;
    let mut with_template = accepted.clone();
    with_template.edges.push(template);
    assert_eq!(
        rejected(verdict(
            &free,
            "NFp",
            &with_template,
            CertificateCheckLimits::default()
        )),
        (
            CertificateRule::Malformed,
            CertificateLocus::CertificateMember(CertificateMember::Edges)
        )
    );
}

/// FR-418-AC-14: scheduler clock metadata under the no-release method is
/// refused as a malformed `task_product`.
#[trace("TC-371", "FR-418-AC-14")]
#[test]
fn a_no_release_closure_refuses_scheduler_clock_metadata() {
    let world = no_release();
    let mut certificate = no_release_certificate(&world, "NFp");
    certificate
        .task_product
        .as_mut()
        .unwrap()
        .clock_ceilings
        .push(fsl_replay::TaskClockCeiling {
            clock: "scheduler".to_owned(),
            maximum: big(4),
        });
    assert_eq!(
        rejected(verdict(
            &world,
            "NFp",
            &certificate,
            CertificateCheckLimits::default()
        )),
        malformed_member()
    );
}

/// The certificate edge and byte budgets bound the finite-product route as
/// they bound the ordinary one, and a cancelled check settles cancelled.
#[test]
fn the_certificate_budgets_and_cancellation_stop_the_task_product_check() {
    let world = solo();
    let built = build(&world, "SFp");
    let edges = CertificateCheckLimits {
        max_certificate_edges: 1,
        ..CertificateCheckLimits::default()
    };
    assert_eq!(
        verdict(&world, "SFp", &built.certificate, edges),
        Ok(ZoneCertificateVerdict::Stopped(
            fsl_replay::IncompleteCause::LimitReached {
                limit: fsl_replay::ModelCheckLimit::CertificateMaxEdges,
                value: 1,
            }
        ))
    );
    let bytes = CertificateCheckLimits {
        max_certificate_bytes: 1,
        ..CertificateCheckLimits::default()
    };
    assert_eq!(
        verdict(&world, "SFp", &built.certificate, bytes),
        Ok(ZoneCertificateVerdict::Stopped(
            fsl_replay::IncompleteCause::LimitReached {
                limit: fsl_replay::ModelCheckLimit::CertificateMaxBytes,
                value: 1,
            }
        ))
    );
    let cancel = Cancel::new();
    cancel.cancel(filament_exact::CancelCause::Requested);
    assert_eq!(
        verdict_with(
            &world,
            "SFp",
            &built.certificate,
            CertificateCheckLimits::default(),
            &cancel
        ),
        Ok(ZoneCertificateVerdict::Stopped(
            fsl_replay::IncompleteCause::Cancelled {
                source: filament_exact::CancelCause::Requested
            }
        ))
    );
}

/// The nodes of the quotient that share a state key with exactly one other
/// region, and are neither initial nor the only way into their key.
fn two_region_node(built: &Built) -> Option<(usize, Vec<usize>)> {
    let mut by_key: BTreeMap<_, Vec<usize>> = BTreeMap::new();
    for (index, vertex) in built.quotient.vertices.iter().enumerate() {
        by_key.entry(&vertex.key).or_default().push(index);
    }
    for (key, regions) in by_key {
        if regions.len() != 2 {
            continue;
        }
        let node = built
            .certificate
            .nodes
            .iter()
            .position(|node| &node.state_key == key)
            .expect("the key's node");
        if !built.certificate.initials.contains(&node) {
            return Some((node, regions));
        }
    }
    None
}

/// FR-418-AC-18: a target whose `Z` is the union of two whole refined
/// regions covers a successor in either of them; the same node reduced to
/// one region alone leaves the other region's successor missing.
#[trace("TC-371", "FR-418-AC-18")]
#[test]
fn a_union_of_two_regions_covers_a_successor_in_either_and_one_region_does_not() {
    let (world, claim, built, node, regions) = [
        (solo(), "SFp"),
        (tied(), "EEdf"),
        (preempting(), "QFp"),
        (parked(), "ZFp"),
    ]
    .into_iter()
    .find_map(|(world, claim)| {
        let built = build(&world, claim);
        let (node, regions) = two_region_node(&built)?;
        Some((world, claim, built, node, regions))
    })
    .expect("a fixture with a non-initial key of exactly two regions");
    let (first, second) = (
        &built.quotient.vertices[regions[0]].zone,
        &built.quotient.vertices[regions[1]].zone,
    );
    assert_eq!(
        built.certificate.nodes[node].zone,
        hull(first, second),
        "the node is the union of its two regions"
    );
    assert_eq!(
        verdict(
            &world,
            claim,
            &built.certificate,
            CertificateCheckLimits::default()
        ),
        Ok(ZoneCertificateVerdict::Accepted)
    );
    let reduced = edit(&world, claim, |c| {
        c.nodes[node].zone = first.clone();
    });
    let (rule, at) = rejected(reduced);
    assert_eq!(rule, CertificateRule::SuccessorMissing);
    let sources: Vec<CertificateLocus> = std::iter::once(node)
        .chain(
            built
                .certificate
                .edges
                .iter()
                .filter(|edge| edge.to == node)
                .map(|edge| edge.from),
        )
        .map(node_at)
        .collect();
    assert!(sources.contains(&at), "{at:?}");
}

/// A canonical zone from hand-written rows of non-strict bounds.
fn rows(rows: &[[i64; 6]]) -> Zone {
    let cells: Vec<_> = rows
        .iter()
        .flatten()
        .map(|&value| Some((Integer::from(value), false)))
        .collect();
    Zone::from_matrix(6, cells).expect("a matrix")
}

/// FR-418-AC-13 (ZC-19/ZC-23 shape): the periodic task `a` (period 4 ms,
/// work 1 ms) has four nodes, written out by hand here rather than read
/// from the checker's own quotient: the idle initial state, the released
/// job, the running job and the completed job, one release edge each way
/// round the period. The zones are over the clocks `a.arrival`,
/// `a.slot0.service`, `a.slot0.deadline`, `a.slot1.service` and
/// `a.slot1.deadline`; only the state keys are taken from the recomputation.
#[trace("TC-371", "FR-418-AC-13")]
#[test]
fn a_hand_written_certificate_for_one_periodic_task_is_accepted() {
    let world = solo();
    let built = build(&world, "SFp");
    let mut certificate = built.certificate.clone();
    let zero = [[0; 6]; 6];
    let running = [
        [0, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 1, 1],
        [1, 0, 0, 0, 1, 1],
        [1, 0, 0, 0, 1, 1],
        [0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0],
    ];
    let completed = [
        [0, -1, 0, 0, 0, 0],
        [4, 0, 4, 4, 4, 4],
        [0, -1, 0, 0, 0, 0],
        [0, -1, 0, 0, 0, 0],
        [0, -1, 0, 0, 0, 0],
        [0, -1, 0, 0, 0, 0],
    ];
    let zones = [zero, zero, running, completed];
    assert_eq!(certificate.nodes.len(), zones.len());
    for (node, zone) in certificate.nodes.iter_mut().zip(&zones) {
        node.zone = rows(zone);
    }
    let shape: Vec<_> = certificate
        .edges
        .iter()
        .map(|edge| {
            (
                edge.from,
                edge.to,
                edge.task_transition.as_ref().map(|t| t.action),
            )
        })
        .collect();
    use fsl_replay::TaskTransitionAction::{Activate, Complete, Dispatch};
    assert_eq!(
        shape,
        [
            (0, 1, Some(Activate)),
            (1, 2, Some(Dispatch)),
            (2, 3, Some(Complete)),
            (3, 1, Some(Activate)),
        ]
    );
    certificate.initials = vec![0];
    certificate.admitted = Some(DivergenceWitness {
        start: 0,
        path: Vec::new(),
        target: DivergentTarget::Quiescent(0),
    });
    assert_eq!(
        verdict(
            &world,
            "SFp",
            &certificate,
            CertificateCheckLimits::default()
        ),
        Ok(ZoneCertificateVerdict::Accepted)
    );
    // The same certificate with the completed node's zone narrowed to the
    // origin does not cover what the running node completes into.
    let mut narrowed = certificate.clone();
    narrowed.nodes[3].zone = rows(&zero);
    assert_eq!(
        rejected(verdict(
            &world,
            "SFp",
            &narrowed,
            CertificateCheckLimits::default(),
        )),
        (CertificateRule::SuccessorMissing, node_at(2))
    );
}

/// A node must be closed under delay within its key: narrowed to the one
/// region the discrete edges enter, only its delay successors escape, and
/// the node itself (not a predecessor) is `successor-missing`.
#[trace("TC-371", "FR-418-AC-2")]
#[test]
fn a_node_not_closed_under_delay_is_successor_missing_at_itself() {
    let world = solo();
    let built = build(&world, "SFp");
    let quotient = &built.quotient;
    let (node, entry) = built
        .certificate
        .nodes
        .iter()
        .enumerate()
        .find_map(|(node, certified)| {
            let mut entries: Vec<usize> = quotient
                .discrete
                .iter()
                .map(|edge| edge.to)
                .chain(
                    quotient
                        .initials
                        .iter()
                        .flat_map(|initial| initial.vertices.iter().copied()),
                )
                .filter(|&vertex| quotient.vertices[vertex].key == certified.state_key)
                .collect();
            entries.sort_unstable();
            entries.dedup();
            let regions = quotient
                .vertices
                .iter()
                .filter(|vertex| vertex.key == certified.state_key)
                .count();
            (entries.len() == 1 && regions > 1).then(|| (node, entries[0]))
        })
        .expect("a node with one entry region and delay successors");
    let narrowed = edit(&world, "SFp", |c| {
        c.nodes[node].zone = quotient.vertices[entry].zone.clone();
    });
    assert_eq!(
        rejected(narrowed),
        (CertificateRule::SuccessorMissing, node_at(node))
    );
}
