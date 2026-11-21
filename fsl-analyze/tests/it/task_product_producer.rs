// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-418 (S5 of QSL-2240): the producer side. EN-6's symbolic pass
//! explores the task automaton with the engine's own DBM and region walk and
//! writes the certificate; the checker accepts the certificate read back from
//! its JSON; and the engine's quotient equals the checker's, so a mistake the
//! two shared would still show as a disagreement here.

use std::collections::BTreeSet;

use filament_exact::Cancel;
use fsl_analyze::zone_check::{
    explore_task_symbolic, read_back_unchanged, task_product_certificate, SymbolicGraph,
    SymbolicStop,
};
use fsl_eval::simulation::model::{TaskAutomaton, TaskProductSystem, TimedSystem};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_replay::{
    CertificateCheckLimits, CertificateRule, DivergenceWitness, DivergentTarget, QuotientIdentity,
    Zone, ZoneCertificate, ZoneCertificateVerdict,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::task_product_certificate::{
    claim_of, over_utilised, preempting, rejected, solo, tied, verdict, Certificate,
};
use crate::timed_world::World;

type Cells = Vec<Option<(String, bool)>>;

fn zone_cells(zone: &Zone) -> Cells {
    let mut cells = Vec::new();
    for i in 0..zone.dim() {
        for j in 0..zone.dim() {
            cells.push(
                zone.bound(i, j)
                    .expect("a bound")
                    .map(|(value, strict)| (value.to_string(), strict)),
            );
        }
    }
    cells
}

fn dbm_cells(zone: &fsl_analyze::zone_check::Dbm) -> Cells {
    let mut cells = Vec::new();
    for i in 0..zone.dim() {
        for j in 0..zone.dim() {
            cells.push(match zone.bound(i, j).expect("a bound") {
                fsl_analyze::zone_check::Bound::Finite { value, strict } => {
                    Some((value.to_string(), *strict))
                }
                fsl_analyze::zone_check::Bound::Infinite => None,
            });
        }
    }
    cells
}

fn engine_graph(world: &World, claim: &str) -> (SymbolicGraph, Certificate) {
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, claim).expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    let graph = explore_task_symbolic(&automaton, 100_000_000, &Cancel::new())
        .expect("the engine closes the product");
    let occurrence = claim_of(world, claim).occurrence().expect("an occurrence");
    let certificate =
        task_product_certificate(&automaton, &graph, occurrence).expect("a certificate");
    (graph, certificate)
}

/// One vertex: key octets and its zone cells.
type Vertex = (Vec<u8>, Cells);

/// The sets compared: vertices as (key octets, zone), discrete edges and
/// delay edges between such vertices.
#[derive(Debug, Eq, PartialEq)]
struct Shape {
    vertices: BTreeSet<Vertex>,
    discrete: BTreeSet<(Vertex, String, Vertex)>,
    time: BTreeSet<(Vertex, Vertex)>,
    initials: BTreeSet<Vertex>,
    subtractions: BTreeSet<(String, String, String)>,
}

fn engine_shape(graph: &SymbolicGraph) -> Shape {
    let at = |index: usize| {
        let vertex = &graph.vertices[index];
        (vertex.key.as_bytes().to_vec(), dbm_cells(&vertex.zone))
    };
    let identity = |index: usize| match &graph.identities[index] {
        fsl_analyze::zone_check::SymbolicIdentity::Source(transition) => {
            fsl_replay::task_source_transition_wire(transition)
                .expect("a spelling")
                .to_string()
        }
        fsl_analyze::zone_check::SymbolicIdentity::Template(template) => {
            template.to_wire().expect("a spelling").to_string()
        }
    };
    Shape {
        vertices: (0..graph.vertices.len()).map(at).collect(),
        discrete: graph
            .discrete
            .iter()
            .map(|edge| (at(edge.from), identity(edge.identity), at(edge.to)))
            .collect(),
        time: graph.time.iter().map(|&(a, b)| (at(a), at(b))).collect(),
        initials: graph
            .initials
            .iter()
            .flat_map(|initial| initial.vertices.iter().map(|&v| at(v)))
            .collect(),
        subtractions: graph
            .subtractions
            .iter()
            .map(|row| {
                (
                    row.template.to_wire().expect("a spelling").to_string(),
                    row.clock.clone(),
                    row.constant.to_string(),
                )
            })
            .collect(),
    }
}

fn checker_shape(world: &World, claim: &str, scale: &filament_exact::Integer) -> Shape {
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, claim).expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    let quotient = fsl_replay::explore_task_product(&automaton, scale, 100_000_000, &Cancel::new())
        .expect("the checker's quotient");
    let at = |index: usize| {
        let vertex = &quotient.vertices[index];
        (vertex.key.as_bytes().to_vec(), zone_cells(&vertex.zone))
    };
    let identity = |index: usize| match &quotient.identities[index] {
        QuotientIdentity::Source(transition) => fsl_replay::task_source_transition_wire(transition)
            .expect("a spelling")
            .to_string(),
        QuotientIdentity::Template(template) => template.to_wire().expect("a spelling").to_string(),
    };
    Shape {
        vertices: (0..quotient.vertices.len()).map(at).collect(),
        discrete: quotient
            .discrete
            .iter()
            .map(|edge| (at(edge.from), identity(edge.identity), at(edge.to)))
            .collect(),
        time: quotient.time.iter().map(|&(a, b)| (at(a), at(b))).collect(),
        initials: quotient
            .initials
            .iter()
            .flat_map(|initial| initial.vertices.iter().map(|&v| at(v)))
            .collect(),
        subtractions: quotient
            .subtractions
            .iter()
            .map(|row| {
                (
                    row.transition.to_wire().expect("a spelling").to_string(),
                    row.clock.clone(),
                    row.constant.to_string(),
                )
            })
            .collect(),
    }
}

const SETS: &[(&str, &str)] = &[
    ("solo", "SFp"),
    ("solo", "SEdf"),
    ("tied", "EEdf"),
    ("preempting", "QFp"),
    ("preempting", "QEdf"),
];

fn set(name: &str) -> World {
    match name {
        "solo" => solo(),
        "tied" => tied(),
        "preempting" => preempting(),
        other => panic!("no set {other}"),
    }
}

/// The emitted certificate is read back unchanged, and a witness the wire
/// would decode differently (a cycle that is two laps of itself reads back as
/// one lap behind a one-lap path) is refused, so `proved` never rests on a
/// certificate other than the one built.
#[trace("TC-371")]
#[test]
fn a_witness_the_wire_decodes_differently_is_not_read_back_unchanged() {
    let mut refused = 0;
    for (name, claim) in SETS {
        let world = set(name);
        let (_, certificate) = engine_graph(&world, claim);
        let occurrence = certificate.obligation.clone();
        assert!(
            read_back_unchanged(&certificate, json!({}), &occurrence).is_some(),
            "{claim}"
        );
        let Some(DivergenceWitness {
            start,
            path,
            target: DivergentTarget::Cycle(cycle),
        }) = certificate.admitted.clone()
        else {
            continue;
        };
        let mut flat = path;
        flat.extend(&cycle);
        let mut doubled = flat.clone();
        doubled.extend(&flat);
        let mut changed = certificate.clone();
        changed.admitted = Some(DivergenceWitness {
            start,
            path: Vec::new(),
            target: DivergentTarget::Cycle(doubled),
        });
        assert!(
            read_back_unchanged(&changed, json!({}), &occurrence).is_none(),
            "{claim}"
        );
        refused += 1;
    }
    assert!(refused > 0, "no set carried a cycle witness to change");
}

/// FR-418: the engine's closed quotient (its own DBM and region walk) equals
/// the quotient the checker recomputes, vertex for vertex, edge for edge, so
/// a bug in one that the certificates of one side alone would hide shows.
#[trace("TC-371")]
#[test]
fn the_engine_quotient_equals_the_checker_quotient() {
    for (name, claim) in SETS {
        let world = set(name);
        let (graph, certificate) = engine_graph(&world, claim);
        let scale = &certificate.scale;
        assert_eq!(
            engine_shape(&graph),
            checker_shape(&world, claim, scale),
            "{claim}"
        );
    }
}

/// FR-418-AC-11, AC-13: the certificate the engine writes is accepted by the
/// checker after a round trip through its JSON, so the proof rests on the
/// emitted bytes.
#[trace("TC-371", "FR-418-AC-11")]
#[test]
fn the_engine_certificate_is_accepted_by_the_checker_from_its_json() {
    for (name, claim) in SETS {
        let world = set(name);
        let (_, certificate) = engine_graph(&world, claim);
        assert_eq!(
            verdict(
                &world,
                claim,
                &certificate,
                CertificateCheckLimits::default()
            )
            .expect("the check runs"),
            ZoneCertificateVerdict::Accepted,
            "{claim}"
        );
        let wire = certificate.to_wire(json!({})).expect("a wire");
        let (read, _) =
            ZoneCertificate::<OccurrenceKey>::from_wire(&wire, certificate.obligation.clone())
                .expect("the wire reads");
        assert_eq!(read.to_wire(json!({})).expect("a wire"), wire, "{claim}");
        assert_eq!(
            verdict(&world, claim, &read, CertificateCheckLimits::default())
                .expect("the check runs"),
            ZoneCertificateVerdict::Accepted,
            "{claim}"
        );
    }
}

fn tampered(
    world: &World,
    claim: &str,
    change: impl FnOnce(&mut Value),
) -> Result<ZoneCertificateVerdict, fsl_replay::ZoneCertificateRefusal<OccurrenceKey>> {
    let (_, certificate) = engine_graph(world, claim);
    let mut wire = certificate.to_wire(json!({})).expect("a wire");
    change(&mut wire);
    let (read, _) = ZoneCertificate::<OccurrenceKey>::from_wire(&wire, certificate.obligation)
        .expect("the tampered wire still reads");
    verdict(world, claim, &read, CertificateCheckLimits::default())
}

/// The rule `QFp`'s certificate is rejected with after `change` edits its
/// wire.
fn rule_after(change: impl FnOnce(&mut Value)) -> CertificateRule {
    rejected(tampered(&preempting(), "QFp", change)).0
}

/// An emitted certificate edited on the wire is rejected with the checker's
/// own rule: a changed member (FR-418-AC-11), a dropped edge, a retargeted
/// edge, squeezed zones, and (FR-418-AC-19) an extra node and an extra edge.
#[trace("TC-371", "FR-418-AC-19")]
#[test]
fn a_tampered_emitted_certificate_is_rejected() {
    // `max_jobs` of the first task one too many.
    assert_eq!(
        rule_after(|wire| {
            let jobs = wire["task_product"]["queue_bounds"][0]["max_jobs"]
                .as_u64()
                .expect("a count");
            wire["task_product"]["queue_bounds"][0]["max_jobs"] = json!(jobs + 1);
        }),
        CertificateRule::Malformed,
    );
    // The last edge dropped.
    assert_eq!(
        rule_after(|wire| {
            wire["edges"].as_array_mut().expect("edges").pop();
        }),
        CertificateRule::SuccessorMissing,
    );
    // Every node squeezed to a point.
    assert_eq!(
        rule_after(|wire| {
            for node in wire["nodes"].as_array_mut().expect("nodes") {
                for row in node["zone"].as_array_mut().expect("a zone") {
                    for entry in row.as_array_mut().expect("a row") {
                        if entry != "infinity" {
                            entry["bound"] = json!("0");
                            entry["strict"] = json!(false);
                        }
                    }
                }
            }
        }),
        CertificateRule::SuccessorMissing,
    );
    // A node nothing reaches.
    assert_eq!(
        rule_after(|wire| {
            let nodes = wire["nodes"].as_array_mut().expect("nodes");
            let copy = nodes[0].clone();
            nodes.push(copy);
        }),
        CertificateRule::UnreachedNode,
    );
    // An edge the recomputation does not produce.
    assert_eq!(
        rule_after(|wire| {
            let edges = wire["edges"].as_array_mut().expect("edges");
            let copy = edges[0].clone();
            edges.push(copy);
        }),
        CertificateRule::ExtraEdge,
    );
}

/// FR-418: the engine does not state a proof for an over-utilised set: a
/// deadline miss is reachable in its quotient, so no certificate is built.
#[trace("TC-371")]
#[test]
fn no_certificate_is_built_for_a_reachable_miss() {
    let world = over_utilised();
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "OFp").expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    // The pass stops at the first reachable miss, so the whole quotient is
    // never explored and the concrete search refutes promptly.
    assert!(matches!(
        explore_task_symbolic(&automaton, 100_000_000, &Cancel::new()),
        Err(SymbolicStop::Missed)
    ));
}

/// The engine's pass obeys its work limit and the cancel (the analyze-level
/// FR-418-AC-20 test is `a_small_max_check_work_settles_limit_reached`).
#[trace("TC-371")]
#[test]
fn the_symbolic_pass_stops_at_its_work_limit_and_on_cancel() {
    let world = preempting();
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "QFp").expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    assert!(matches!(
        explore_task_symbolic(&automaton, 10, &Cancel::new()),
        Err(SymbolicStop::WorkLimit(10))
    ));
    let cancel = Cancel::new();
    cancel.cancel(filament_exact::CancelCause::Requested);
    assert!(matches!(
        explore_task_symbolic(&automaton, 100_000_000, &cancel),
        Err(SymbolicStop::Cancelled(_))
    ));
}

/// A receiver reference whose `object_type` is `null` is not an effective-id
/// object (the spine's `read_effective` refuses a null root), so the wire is
/// not read.
#[trace("TC-371")]
#[test]
fn a_null_receiver_object_type_is_not_read() {
    let world = preempting();
    let (_, certificate) = engine_graph(&world, "QFp");
    let mut wire = certificate.to_wire(json!({})).expect("a wire");
    let edge = wire["edges"]
        .as_array_mut()
        .expect("edges")
        .iter_mut()
        .find(|edge| edge["transition"]["source_transition"]["receiver"].is_object())
        .expect("an edge with a source receiver");
    edge["transition"]["source_transition"]["receiver"]["object_type"] = Value::Null;
    let refusal = ZoneCertificate::<OccurrenceKey>::from_wire(&wire, certificate.obligation)
        .expect_err("a null object type is refused");
    assert!(
        format!("{refusal:?}").contains("object_type"),
        "the refusal names the member: {refusal:?}"
    );
}
