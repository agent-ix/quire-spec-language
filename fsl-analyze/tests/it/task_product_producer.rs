// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-418 (S5 of QSL-2240): the producer side. EN-6's symbolic pass
//! explores the task automaton with the engine's own DBM and region walk and
//! writes the certificate; the checker accepts the certificate read back from
//! its JSON; and the engine's quotient equals the checker's, so a mistake the
//! two shared would still show as a disagreement here.

use std::collections::BTreeSet;

use filament_exact::Cancel;
use fsl_analyze::zone_check::{
    explore_task_symbolic, task_product_certificate, SymbolicGraph, SymbolicStop,
};
use fsl_eval::simulation::model::{TaskAutomaton, TaskProductSystem, TimedSystem};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_replay::{
    CertificateCheckLimits, CertificateRule, QuotientIdentity, Zone, ZoneCertificate,
    ZoneCertificateVerdict,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::task_product_certificate::{
    build, claim_of, over_utilised, preempting, rejected, solo, tied, verdict, Certificate,
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

/// The sets compared: vertices as (key octets, zone), discrete edges and
/// delay edges between such vertices.
#[derive(Debug, Eq, PartialEq)]
struct Shape {
    vertices: BTreeSet<(Vec<u8>, Cells)>,
    discrete: BTreeSet<((Vec<u8>, Cells), String, (Vec<u8>, Cells))>,
    time: BTreeSet<((Vec<u8>, Cells), (Vec<u8>, Cells))>,
    initials: BTreeSet<(Vec<u8>, Cells)>,
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
    let quotient =
        fsl_replay::explore_task_product(&automaton, scale, 100_000_000, &Cancel::new())
            .expect("the checker's quotient");
    let at = |index: usize| {
        let vertex = &quotient.vertices[index];
        (vertex.key.as_bytes().to_vec(), zone_cells(&vertex.zone))
    };
    let identity = |index: usize| match &quotient.identities[index] {
        QuotientIdentity::Source(transition) => {
            fsl_replay::task_source_transition_wire(transition)
                .expect("a spelling")
                .to_string()
        }
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

/// FR-418: the engine's closed quotient (its own DBM and region walk) equals
/// the quotient the checker recomputes, vertex for vertex, edge for edge, so
/// a bug in one that the certificates of one side alone would hide shows.
#[trace("TC-371", "FR-418-AC-19")]
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
            verdict(&world, claim, &certificate, CertificateCheckLimits::default())
                .expect("the check runs"),
            ZoneCertificateVerdict::Accepted,
            "{claim}"
        );
        let wire = certificate.to_wire(json!({})).expect("a wire");
        let (read, _) = ZoneCertificate::<OccurrenceKey>::from_wire(
            &wire,
            certificate.obligation.clone(),
        )
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

/// FR-418-AC-11/AC-19: an emitted certificate edited on the wire is
/// rejected: a member, a node, an edge or a zone.
#[trace("TC-371", "FR-418-AC-19")]
#[test]
fn a_tampered_emitted_certificate_is_rejected() {
    let world = preempting();
    // `max_jobs` of the first task one too many.
    let result = tampered(&world, "QFp", |wire| {
        let jobs = wire["task_product"]["queue_bounds"][0]["max_jobs"]
            .as_u64()
            .expect("a count");
        wire["task_product"]["queue_bounds"][0]["max_jobs"] = json!(jobs + 1);
    });
    assert_eq!(
        rejected(result).0,
        CertificateRule::Malformed,
        "a changed member is malformed"
    );
    // The last edge dropped: the node's successor is missing.
    let result = tampered(&world, "QFp", |wire| {
        wire["edges"].as_array_mut().expect("edges").pop();
    });
    assert!(matches!(
        rejected(result).0,
        CertificateRule::SuccessorMissing | CertificateRule::Malformed
    ));
    // An edge retargeted to the first node.
    let result = tampered(&world, "QFp", |wire| {
        let edges = wire["edges"].as_array_mut().expect("edges");
        let last = edges.len() - 1;
        let target = edges[last]["target"].as_u64().expect("a node");
        edges[last]["target"] = json!(if target == 0 { 1 } else { 0 });
    });
    assert_ne!(
        result.expect("the check runs"),
        ZoneCertificateVerdict::Accepted
    );
    // Every node squeezed to a point no longer covers the closure.
    let result = tampered(&world, "QFp", |wire| {
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
    });
    assert_ne!(
        result.expect("the check runs"),
        ZoneCertificateVerdict::Accepted
    );
}

/// FR-418: the engine does not state a proof for an over-utilised set: a
/// deadline miss is reachable in its quotient, so no certificate is built.
#[trace("TC-371", "FR-418-AC-2")]
#[test]
fn no_certificate_is_built_for_a_reachable_miss() {
    let world = over_utilised();
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "OFp").expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    let graph = explore_task_symbolic(&automaton, 100_000_000, &Cancel::new())
        .expect("the product closes");
    let occurrence = claim_of(&world, "OFp").occurrence().expect("an occurrence");
    assert!(task_product_certificate(&automaton, &graph, occurrence).is_err());
}

/// FR-418-AC-20: the engine's pass obeys the work limit and the cancel.
#[trace("TC-371", "FR-418-AC-20")]
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
    let _ = build;
}
