// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1037 row 15 (FR-116-AC-7): `crate::replay_frame` over the **orders**
//! package's `Orders::Pump::rehome`, invocation **I** (TC-1035): `self`
//! `{Q, p1}`, `created []`, `deleted []`; housing `p1`-`t1`, `p2`-`t1` in the
//! pre snapshot and `p1`-`t2`, `p2`-`t1` in the post snapshot.

use super::frame_replay::{package_digest, packet, request, unit_for, wire, Unit};
use super::*;
use crate::{
    replay_frame, ClaimedChange, DisagreementCause, FrameCounterexample, FrameOperation,
    FrameReplayResult, ReplayResult, Verdict, WitnessEnvelope, WitnessSettlement,
};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::model::intake::meaning;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::observation::{FrameChange, LinkDifference, SelectedObject};
use quire_exact::Identifier;
use quire_semantic_value::catalog::Category;

const PACKAGE: &str = "test/orders";
const DIGEST: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const HOUSING: &str = "relationship/Pump-housing-Tank";

fn identity(node: &str) -> String {
    format!("ix://{PACKAGE}/{node}")
}

fn provenance(node: &str) -> serde_json::Value {
    json!({"generated": {
        "generatorIdentity": node, "generatorVersion": "1.0.0", "inputIdentities": [node],
    }})
}

fn construct(name: &str, meaning: &str) -> serde_json::Value {
    json!({
        "kind": {"module": PACKAGE, "name": name},
        "moduleVersion": "1.0.0",
        "manifestDigest": DIGEST,
        "construct": {"identity": "none", "shape": "record", "members": {}, "meaning": meaning},
    })
}

fn object_type(name: &str, extra: serde_json::Value) -> serde_json::Value {
    let node = identity(name);
    let mut value = json!({
        "identity": node, "displayName": node,
        "kind": {"module": PACKAGE, "name": "object_type"},
        "roles": [], "origin": provenance(&node), "constraints": [], "extensions": [],
        "unknownPolicy": "reject", "supertypes": [], "fields": [], "operations": [],
        "relationships": [],
    });
    for (member, entry) in extra.as_object().expect("an object") {
        value[member] = entry.clone();
    }
    value
}

fn end(object: &str, role: &str) -> serde_json::Value {
    json!({
        "type": identity(object), "role": role,
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
    })
}

fn origin() -> serde_json::Value {
    json!({"source": {
        "sourceIdentity": identity("spec"), "path": "spec.qspec", "startLine": 1, "startColumn": 1,
    }})
}

/// The orders domain package: `Pump-housing-Tank`, and `rehome` whose frame
/// modifies `modifies`.
fn orders_document(modifies: &[String]) -> Vec<u8> {
    let sys = object_type("Sys", json!({}));
    let backup = identity("Pump/backup");
    let pump = object_type(
        "Pump",
        json!({
            "fields": [{
                "identity": backup, "name": "backup", "typeRef": identity("Pump"),
                "presence": "optional", "nullable": false, "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": provenance(&backup),
            }],
            "operations": [{
                "identity": identity("Pump/rehome"), "name": "rehome",
                "params": [], "pre": [], "post": [], "origin": origin(),
                "frame": {"modifies": modifies, "creates": [], "deletes": []},
            }],
            "relationships": [{
                "identity": identity(HOUSING),
                "sourceEnd": end("Pump", "home"), "targetEnd": end("Tank", "pump"),
                "category": "structural", "composite": false,
                "direction": "source-to-target", "origin": origin(),
            }],
        }),
    );
    let tank = object_type("Tank", json!({}));
    let members = ["Sys", "Pump", "Tank"].map(identity);
    let population = json!({
        "identity": identity("Q"), "displayName": identity("Q"),
        "kind": {"module": PACKAGE, "name": "population"},
        "members": members,
        "extent": "closed", "origin": provenance(&identity("Q")),
    });
    serde_json::to_vec(&json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {
            "identity": identity("spec"), "version": "1.0.0",
            "dialect": "spec-bundle", "digest": DIGEST,
        },
        "package": {
            "identity": PACKAGE, "version": "1.0.0", "manifestDigest": DIGEST,
            "mappingVersions": [], "profileVersions": [], "lockDigest": DIGEST,
        },
        "occurrences": [], "extensions": [],
        "constructs": [
            construct("object_type", meaning::OBJECT_TYPE),
            construct("population", meaning::POPULATION),
        ],
        "types": [sys, pump, tank],
        "populations": [population],
    }))
    .expect("valid JSON")
}

/// The orders unit selecting `document` as model `Orders`, with the
/// postcondition that gives `rehome` its frame node.
fn orders_unit(modifies: &[String]) -> (Unit, String) {
    let document = orders_document(modifies);
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let digest = hex(packages.keys().next().expect("one model"));
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model Orders = {PACKAGE:?} version \"1.0.0\" digest \"sha256-jcs:{digest}\";\n\
         post RehomePost using v on Orders::Pump::rehome {{ true }}\n"
    );
    (unit_for(unit, document), digest)
}

fn labelled(name: &str, digest: &str, mut document: serde_json::Value) -> serde_json::Value {
    document["model"] = json!({
        "identity": PACKAGE, "version": "1.0.0", "digest": format!("sha256-jcs:{digest}"),
    });
    document["identity"] = document_identity_json(&config_version_document_ref(name));
    document
}

fn housing(instances: &[(&str, &str)]) -> serde_json::Value {
    let end = |key: &str| json!({"population": identity("Q"), "key": key});
    json!({
        "relationship": identity(HOUSING), "complete": true,
        "instances": instances
            .iter()
            .map(|(source, target)| json!({"source": end(source), "target": end(target)}))
            .collect::<Vec<_>>(),
    })
}

fn side(observation: &str, instances: &[(&str, &str)]) -> serde_json::Value {
    let objects: Vec<_> = [
        ("s1", "Sys"),
        ("p1", "Pump"),
        ("p2", "Pump"),
        ("t1", "Tank"),
        ("t2", "Tank"),
    ]
    .iter()
    .map(|(key, type_name)| {
        let fields = if *type_name == "Pump" {
            json!({"backup": {"absent": {}}})
        } else {
            json!({})
        };
        json!({"key": key, "type": identity(type_name), "fields": fields})
    })
    .collect();
    json!({
        "format": "quire.state.snapshot/v1",
        "observation": observation,
        "populations": [{"population": identity("Q"), "complete": true, "objects": objects}],
        "relationships": [housing(instances)],
    })
}

/// Invocation **I**: its three documents, by reference and bytes.
fn invocation_i(digest: &str) -> (DocumentRef, Vec<(DocumentRef, Vec<u8>)>) {
    let provision = |name: &str, document: serde_json::Value| {
        let label = config_version_document_ref(name);
        let bytes = document.to_string().into_bytes();
        (
            DocumentRef {
                digest: document_digest(&bytes),
                ..label
            },
            bytes,
        )
    };
    let call = "replay-call";
    let mut pre = labelled(
        "replay-pre",
        digest,
        side("pre", &[("p1", "t1"), ("p2", "t1")]),
    );
    let mut post = labelled(
        "replay-post",
        digest,
        side("post", &[("p1", "t2"), ("p2", "t1")]),
    );
    for document in [&mut pre, &mut post] {
        document["invocation"] = document_identity_json(&config_version_document_ref(call));
    }
    let pre = provision("replay-pre", pre);
    let post = provision("replay-post", post);
    let reference = |held: &DocumentRef| {
        json!({
            "identity": document_identity_json(held),
            "digest": format!("sha256-jcs:{}", hex(&held.digest)),
        })
    };
    let invocation = labelled(
        call,
        digest,
        json!({
            "format": "quire.state.invocation/v1",
            "context": identity("Pump"),
            "operation": "rehome",
            "self": {"population": identity("Q"), "key": "p1"},
            "pre": reference(&pre.0),
            "post": reference(&post.0),
            "parameters": {}, "result": null, "created": [], "deleted": [],
        }),
    );
    let invocation = provision(call, invocation);
    (invocation.0.clone(), vec![invocation, pre, post])
}

fn link(difference: LinkDifference, source: &str, target: &str) -> ClaimedChange {
    let end = |key: &str| SelectedObject {
        population: identity("Q"),
        key: key.to_owned(),
    };
    ClaimedChange::Link {
        relationship: DeclarationKey {
            package: PACKAGE.to_owned(),
            node: identity(HOUSING),
        },
        difference,
        source: end(source),
        target: end(target),
    }
}

/// Replays a hand-built envelope for `Orders::Pump::rehome` over the orders
/// package whose frame modifies `modifies`, carrying invocation **I** and
/// `change`.
fn replay_rehome(modifies: &[String], change: ClaimedChange) -> FrameReplayResult {
    let (unit, digest) = orders_unit(modifies);
    let (invocation, documents) = invocation_i(&digest);
    let graph = unit.compiled.package.graph();
    let identifier = |text: &str| Identifier::new(text).expect("an identifier");
    let selection = graph
        .resolve_operation(
            &identifier("Orders"),
            &identifier("Pump"),
            &identifier("rehome"),
        )
        .expect("Orders::Pump::rehome resolves");
    let (_, frame) = graph
        .operation_frame(&selection)
        .expect("RehomePost names the operation");
    let (anchor, frame_node) = (wire(frame.anchor()), wire(frame.frame()));
    let occurrence = OccurrenceKey::new(frame_node, frame.frame_origin().clone());
    let target = crate::ReplayTarget::FrameOperation(FrameOperation {
        object: crate::QualifiedName::new(vec![identifier("Orders"), identifier("Pump")]).unwrap(),
        operation: identifier("rehome"),
    });
    let payload = FrameCounterexample {
        anchor,
        frame: frame_node,
        occurrence: occurrence.clone(),
        invocation,
        change,
    };
    let package_id = package_digest(unit.compiled.emitted.package_id());
    let wire_request = request(
        &unit.bytes,
        &unit.domain_document,
        package_id,
        &documents,
        target.clone(),
    );
    let packet = packet(
        &unit.bytes,
        package_id,
        super::frame_replay::witness_source(),
        frame_node,
        occurrence,
        payload,
        target,
    );
    let envelope = WitnessEnvelope::reconstruct(packet, crate::ReplayLimits::default())
        .expect("a complete packet reconstructs");
    replay_frame(wire_request, &envelope, crate::ReplayLimits::default())
        .expect("the replay settles")
}

fn found_link(result: &FrameReplayResult) -> &FrameChange {
    &result.found().expect("the replay found a violation").change
}

fn assert_found_removed_p1_t1(result: &FrameReplayResult) {
    let FrameChange::Link {
        relationship,
        difference,
        source,
        target,
        modifies,
    } = found_link(result)
    else {
        panic!("expected a link change, got {:?}", found_link(result));
    };
    assert_eq!(relationship.node, identity(HOUSING));
    assert_eq!(*difference, LinkDifference::Removed);
    assert_eq!((source.key.as_str(), target.key.as_str()), ("p1", "t1"));
    assert_eq!(source.population, identity("Q"));
    assert!(modifies.is_empty());
}

fn assert_reproduced(result: &FrameReplayResult) {
    let ReplayResult::Witness(arm) = result.result() else {
        panic!("expected the witness arm, got {:?}", result.result());
    };
    assert_eq!(
        arm.settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    assert_eq!(arm.category(), Category::Violation);
}

/// FR-116-AC-7 (TC-1037 row 15): under a frame that modifies no
/// relationship, the claim `p1`-`t1` `removed` reproduces and the result
/// holds the payload's change and the replay's `Link`.
#[trace("TC-1037", "FR-116-AC-7")]
#[test]
fn a_claimed_removed_link_reproduces_with_the_replays_link() {
    let claim = link(LinkDifference::Removed, "p1", "t1");
    let result = replay_rehome(&[], claim.clone());
    assert_reproduced(&result);
    assert_eq!(result.claimed(), &claim);
    assert_found_removed_p1_t1(&result);
    assert_eq!(
        ClaimedChange::of_witness(result.found().expect("a witness")),
        claim
    );
}

/// FR-116-AC-7 (TC-1037 row 15): the claim `p1`-`t2` `added` reproduces the
/// same, and both changes stay as found.
#[trace("TC-1037", "FR-116-AC-7")]
#[test]
fn a_claimed_added_link_reproduces_keeping_both_changes() {
    let claim = link(LinkDifference::Added, "p1", "t2");
    let result = replay_rehome(&[], claim.clone());
    assert_reproduced(&result);
    assert_eq!(result.claimed(), &claim);
    assert_found_removed_p1_t1(&result);
}

/// FR-116-AC-7 (TC-1037 row 15): under a frame that modifies
/// `Pump-housing-Tank` the replay finds nothing: `inconclusive`, `Verdicts`.
#[trace("TC-1037", "FR-116-AC-7")]
#[test]
fn a_frame_modifying_the_relationship_is_inconclusive_by_verdicts() {
    let claim = link(LinkDifference::Removed, "p1", "t1");
    let result = replay_rehome(&[identity(HOUSING)], claim.clone());
    let ReplayResult::Witness(arm) = result.result() else {
        panic!("expected the witness arm, got {:?}", result.result());
    };
    assert_eq!(arm.settlement(), WitnessSettlement::Inconclusive);
    assert_eq!(
        arm.disagreement(),
        Some(&DisagreementCause::Verdicts {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Success),
        })
    );
    assert!(result.found().is_none());
    assert_eq!(result.claimed(), &claim);
}
