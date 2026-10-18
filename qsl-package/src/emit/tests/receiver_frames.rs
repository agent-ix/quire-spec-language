// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1043 (FR-154-AC-6, AC-7, AC-10; FR-105-AC-7) and TC-573 (FR-154-AC-4): the `state`/`frame` node
//! body S4 emits for receiver-scoped field entries and scoped deletes, the
//! node ids and package ids those entries decide, and IR's I2 read of the
//! frame step over edited copies. The expected bodies are the vectors of
//! TC-1043's "Expected Results", written here from the test case.

use qsl_foundation::Code;
use quire_contract_model::{CheckedPackageRefusalCause, CheckedPackageRefusalCode};

use super::*;
use crate::checked_v2::tests::rekey_stale;
use crate::checked_v2::V2ReadRefusal;

#[path = "../../../../qsl-semantics/tests/support/receiver_frames.rs"]
pub(super) mod fixture;

const NODE_DOMAIN: &str = "quire.checked-semantic-node/v1";

struct Emitted {
    emission: Emission,
    wire: Value,
    evidence: CheckedPackageEvidence,
}

/// What [`unit_of`] builds: the unit text, the package document, the package
/// input keyed by digest, and the hex digest the unit pins the package by.
struct UnitParts {
    unit: String,
    document: Vec<u8>,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    digest: String,
}

/// The unit text selecting the package holding `operations` of `Counter`
/// (each named by its own postcondition clause) with the frames of
/// `overrides`, the package document, and the digest the unit pins it by.
fn unit_of(operations: &[&str], overrides: &[(&str, Value)]) -> UnitParts {
    let document = serde_json::to_vec(&fixture::document(operations, overrides))
        .expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         {posts}",
        package = fixture::PACKAGE,
        posts = fixture::posts(operations),
    );
    UnitParts {
        unit,
        document,
        packages,
        digest,
    }
}

/// The package holding `operations` of `Counter` (each named by its own
/// postcondition clause), with the frames of `overrides`.
fn emit(operations: &[&str], overrides: &[(&str, Value)]) -> Emitted {
    let UnitParts {
        unit,
        document,
        packages,
        digest,
    } = unit_of(operations, overrides);
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(digest.as_str(), document.as_slice())
        .expect("a fresh evidence key");
    Emitted {
        emission,
        wire,
        evidence,
    }
}

impl Emitted {
    /// QSL's I2 read of `envelope` as the package pinned at its own
    /// recomputed `package_id`.
    fn read_envelope(&self, envelope: &Value) -> Read {
        let package_id =
            qsl_semantics::library::PackageId::of_preimage(&jcs(&envelope["identity_preimage"]));
        read_v2(
            &jcs(envelope),
            library(),
            V2ReadLimits::default(),
            &self.evidence,
            &qsl_semantics::library::fixtures::single_pin(library(), package_id),
        )
    }

    /// The I2 read of the package as emitted.
    fn read(&self) -> Read {
        self.read_envelope(&self.wire)
    }

    /// The `Counter` object type's node id, `<Counter>` in the vectors.
    fn counter(&self) -> Value {
        let found: Vec<&Value> = super::owners::node_where(&self.wire, "model", "object_type")
            .into_iter()
            .filter(|node| node["owner"]["node"] == fixture::id("Counter"))
            .collect();
        assert_eq!(found.len(), 1, "Counter is one object_type node");
        found[0]["node_id"].clone()
    }

    /// The frame node id the anchor of `operation` references.
    fn frame_id(&self, operation: &str) -> Value {
        let anchors: Vec<&Value> =
            super::owners::node_where(&self.wire, "state", "operation_anchor")
                .into_iter()
                .filter(|anchor| anchor["body"]["members"][1]["value"]["value"] == operation)
                .collect();
        let [anchor] = anchors[..] else {
            panic!("{operation} has one operation_anchor node: {anchors:#?}");
        };
        let target = &anchor["body"]["members"][2]["value"]["target"];
        assert_eq!(anchor["body"]["members"][2]["name"], "frame");
        target.clone()
    }

    /// The `state`/`frame` node `operation`'s anchor references.
    fn frame(&self, operation: &str) -> &Value {
        let id = self.frame_id(operation);
        super::owners::node_where(&self.wire, "state", "frame")
            .into_iter()
            .find(|node| node["node_id"] == id)
            .unwrap_or_else(|| panic!("{operation}'s frame node is in the package"))
    }
}

/// The NodeRef object `<Counter>` stands for.
fn node_ref(id: &Value) -> Value {
    json!({"digest": id["digest"], "domain": NODE_DOMAIN})
}

/// The frame body vectors W1 to W6 over `counter` (`<Counter>`).
fn vectors(counter: &Value) -> [Value; 6] {
    let field = |name: &str, scope: &str| json!({"declaration": node_ref(counter), "kind": "field", "name": name, "scope": scope});
    let scoped = |entries: Vec<Value>| Value::Array(entries);
    [
        // W1 (bump)
        json!({"creates": [], "deletes": [], "deletes_scoped": scoped(vec![json!({"kind": "receiver"})]),
               "modifies": [field("step", "receiver"), field("value", "every-object")], "term": "frame"}),
        // W2 (merge)
        json!({"creates": [], "deletes": [node_ref(counter)],
               "deletes_scoped": scoped(vec![
                   json!({"kind": "receiver"}),
                   json!({"kind": "parameter", "name": "a"}),
                   json!({"kind": "parameter", "name": "b"}),
               ]), "modifies": [], "term": "frame"}),
        // W3 (noop)
        json!({"creates": [], "deletes": [], "deletes_scoped": [], "modifies": [], "term": "frame"}),
        // W4 (own)
        json!({"creates": [], "deletes": [], "deletes_scoped": [],
               "modifies": [field("value", "receiver")], "term": "frame"}),
        // W5 (all)
        json!({"creates": [], "deletes": [], "deletes_scoped": [],
               "modifies": [field("value", "every-object")], "term": "frame"}),
        // W6 (drop)
        json!({"creates": [], "deletes": [], "deletes_scoped": [json!({"kind": "receiver"})],
               "modifies": [], "term": "frame"}),
    ]
}

const ALL: [&str; 7] = ["bump", "merge", "noop", "own", "all", "drop", "dispose"];

/// FR-154-AC-6 (TC-1043 step 1): the frame bodies of `bump`, `merge` and
/// `noop` are vectors W1, W2 and W3, the frames of `bump` and `merge` depend
/// on `<Counter>` and the scoped entries add no other dependency, and the
/// package reads back `Verified` (the body of W1 is admitted).
#[trace("TC-1043", "FR-154-AC-6", "FR-154-AC-10")]
#[test]
fn the_frame_bodies_of_scoped_entries_are_the_wire_vectors() {
    let emitted = emit(&ALL, &[]);
    let counter = emitted.counter();
    let [w1, w2, w3, ..] = vectors(&counter);
    assert_eq!(emitted.frame("bump")["body"], w1);
    assert_eq!(emitted.frame("merge")["body"], w2);
    assert_eq!(emitted.frame("noop")["body"], w3);
    for operation in ["bump", "merge"] {
        assert_eq!(
            emitted.frame(operation)["dependencies"],
            json!([counter]),
            "{operation}"
        );
    }
    assert!(matches!(emitted.read(), Read::Verified { .. }));
}

/// FR-154-AC-7, FR-105 (TC-1043 steps 1 and 2): `own`, `all` and `drop` are
/// vectors W4, W5 and W6; frames differing only by one entry's scope, or only
/// by `deletes self`, have different node ids; two packages differing only by
/// that one scope have different `package_id`s; a frame of type-wide entries
/// carries `deletes_scoped: []` and every field entry `every-object`.
#[trace("TC-1043", "FR-154-AC-7")]
#[test]
fn a_scope_or_a_scoped_delete_changes_the_node_id_and_the_package_id() {
    let emitted = emit(&ALL, &[]);
    let counter = emitted.counter();
    let [_, _, w3, w4, w5, w6] = vectors(&counter);
    assert_eq!(emitted.frame("own")["body"], w4);
    assert_eq!(emitted.frame("all")["body"], w5);
    assert_eq!(emitted.frame("drop")["body"], w6);
    assert_ne!(emitted.frame_id("own"), emitted.frame_id("all"));
    assert_ne!(emitted.frame_id("drop"), emitted.frame_id("noop"));
    assert_eq!(emitted.frame("noop")["body"], w3);
    assert_eq!(emitted.frame("all")["body"]["deletes_scoped"], json!([]));

    let without_all: Vec<&str> = ALL.iter().copied().filter(|name| *name != "all").collect();
    let b1 = emit(&without_all, &[]);
    let b2 = emit(
        &without_all,
        &[(
            "own",
            json!({"modifies": [fixture::id("Counter/value")], "creates": [], "deletes": []}),
        )],
    );
    assert_ne!(
        b1.emission.package().package_id(),
        b2.emission.package().package_id()
    );
}

/// FR-154-AC-4 (TC-573): the frame node of an operation under `modifies self.value`
/// carries the scope `receiver` and under `modifies value` the scope
/// `every-object`, and the two nodes' contents differ; a frame naming `value`
/// in both `modifies_receiver` and `modifies` is refused at intake with
/// `invalid_model_binding`/`conflicting-binding` naming both entries and the
/// operation, so no package and no frame node is built.
#[trace("TC-573", "FR-154-AC-4")]
#[test]
fn the_two_scopes_of_one_field_differ_in_the_node_and_both_at_once_refuse_at_intake() {
    use qsl_semantics::model::intake::{admit_unit, UnitIntakeCause};
    use qsl_semantics::model::key::DeclarationKey;
    use qsl_semantics::model::refusal::ModelRefusalCause;

    let emitted = emit(&["own", "all"], &[]);
    let own = emitted.frame("own");
    let all = emitted.frame("all");
    assert_eq!(own["body"]["modifies"][0]["scope"], "receiver");
    assert_eq!(all["body"]["modifies"][0]["scope"], "every-object");
    assert_ne!(own["body"], all["body"]);

    let value = fixture::id("Counter/value");
    let both = json!({
        "modifies": [value], "modifies_receiver": [value], "creates": [], "deletes": [],
    });
    let UnitParts { unit, packages, .. } = unit_of(&["own"], &[("own", both)]);
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:tc-573", "fixture", "fixture:1"),
        "program.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let refusal = admit_unit(
        &built.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect_err("both scopes of one field refuse at intake");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("{refusal:?}: expected a Refused cause");
    };
    assert_eq!(refusals.len(), 1, "one refusal: {refusals:?}");
    let refusal = &refusals[0];
    assert_eq!(refusal.code.as_str(), "invalid_model_binding");
    let ModelRefusalCause::ConflictingBinding { key, .. } = &refusal.cause else {
        panic!("{:?}: expected ConflictingBinding", refusal.cause);
    };
    assert_eq!(
        key,
        &DeclarationKey {
            package: fixture::PACKAGE.to_owned(),
            node: fixture::id("Counter/own"),
        }
    );
    assert!(
        refusal.detail.contains("modifies[0]") && refusal.detail.contains("modifies_receiver[0]"),
        "names both entries: {}",
        refusal.detail
    );
}

/// FR-105-AC-7 (TC-1043 step 2b): `all` and `same` have equal frames, so the
/// package holds one `state`/`frame` node for both, whose body is W5 and
/// which has a `generated` occurrence for `all` and then one for `same`, and
/// two `operation_anchor` nodes both referencing it; the I2 read admits it.
#[trace("TC-1043", "FR-105-AC-7")]
#[test]
fn equal_frames_share_one_node_with_an_occurrence_per_operation() {
    let emitted = emit(&["noop", "all", "same"], &[]);
    let counter = emitted.counter();
    let [_, _, _, _, w5, _] = vectors(&counter);
    let frames: Vec<&Value> = super::owners::node_where(&emitted.wire, "state", "frame")
        .into_iter()
        .filter(|node| node["body"] == w5)
        .collect();
    let [frame] = frames[..] else {
        panic!("one frame node with body W5: {frames:#?}");
    };
    assert_eq!(emitted.frame_id("all"), frame["node_id"]);
    assert_eq!(emitted.frame_id("same"), frame["node_id"]);
    let occurrences = frame["occurrences"].as_array().expect("occurrences");
    assert_eq!(occurrences.len(), 2, "{occurrences:#?}");
    assert!(occurrences.iter().all(|o| o["role"] == "generated"));
    assert_eq!(
        occurrences
            .iter()
            .map(|o| o["ordinal"].clone())
            .collect::<Vec<_>>(),
        [json!(0), json!(1)]
    );
    let anchors = super::owners::node_where(&emitted.wire, "state", "operation_anchor");
    assert_eq!(anchors.len(), 3, "noop, all and same each have an anchor");
    assert!(matches!(emitted.read(), Read::Verified { .. }));
}

/// Edits the frame node of `operation` in a copy of the emitted package by
/// `edit`, re-keys the node, its anchor and their dependents, and recomputes
/// the `package_id`, then reads the copy (TC-1043 step 4).
fn read_edited(emitted: &Emitted, operation: &str, edit: impl FnOnce(&mut Value)) -> Read {
    let id = emitted.frame_id(operation);
    let mut envelope = emitted.wire.clone();
    let node = envelope["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find(|node| node["node_id"] == id)
        .expect("the frame node");
    edit(&mut node["body"]);
    rekey_stale(&mut envelope);
    emitted.read_envelope(&envelope)
}

fn refusal(
    read: &Read,
) -> (
    CheckedPackageRefusalCode,
    Option<CheckedPackageRefusalCause>,
    Code,
) {
    match read {
        Read::Refused(outer @ V2ReadRefusal::Envelope { refusal, .. }) => {
            (refusal.code, refusal.cause, outer.code())
        }
        other => panic!("expected an envelope refusal, got {other:?}"),
    }
}

/// FR-154-AC-10 (TC-1043 step 4, X1 to X3): a field entry with no `scope`, a
/// `scope` other than `receiver` or `every-object`, and a frame body with no
/// `deletes_scoped` each refuse `invalid_semantic_graph` with no cause.
#[trace("TC-1043", "FR-154-AC-10")]
#[test]
fn a_missing_or_unknown_scope_and_a_missing_deletes_scoped_refuse_as_schema_failures() {
    let emitted = emit(&ALL, &[]);
    let x1 = read_edited(&emitted, "bump", |body| {
        body["modifies"][0]
            .as_object_mut()
            .expect("entry")
            .remove("scope");
    });
    let x2 = read_edited(&emitted, "bump", |body| {
        body["modifies"][0]["scope"] = json!("all");
    });
    let x3 = read_edited(&emitted, "bump", |body| {
        body.as_object_mut().expect("body").remove("deletes_scoped");
    });
    for (row, read) in [("X1", x1), ("X2", x2), ("X3", x3)] {
        assert_eq!(
            refusal(&read),
            (
                CheckedPackageRefusalCode::InvalidSemanticGraph,
                None,
                Code::InvalidPackage
            ),
            "{row}"
        );
    }
}

/// FR-154-AC-10 (X4 to X6): `deletes_scoped` with a parameter before the
/// receiver, or `b` before `a`, refuses `invalid_semantic_graph`; a
/// `parameter` entry `c` that `merge` has no parameter for refuses
/// `missing_declaration`/`missing-name`.
#[trace("TC-1043", "FR-154-AC-10")]
#[test]
fn a_misordered_deletes_scoped_and_an_unknown_parameter_refuse() {
    let emitted = emit(&ALL, &[]);
    let receiver = || json!({"kind": "receiver"});
    let parameter = |name: &str| json!({"kind": "parameter", "name": name});
    let x4 = read_edited(&emitted, "merge", |body| {
        body["deletes_scoped"] = json!([parameter("a"), receiver(), parameter("b")]);
    });
    let x5 = read_edited(&emitted, "merge", |body| {
        body["deletes_scoped"] = json!([receiver(), parameter("b"), parameter("a")]);
    });
    for (row, read) in [("X4", x4), ("X5", x5)] {
        assert_eq!(
            refusal(&read),
            (
                CheckedPackageRefusalCode::InvalidSemanticGraph,
                None,
                Code::InvalidPackage
            ),
            "{row}"
        );
    }
    let x6 = read_edited(&emitted, "merge", |body| {
        body["deletes_scoped"] =
            json!([receiver(), parameter("a"), parameter("b"), parameter("c")]);
    });
    assert_eq!(
        refusal(&x6),
        (
            CheckedPackageRefusalCode::MissingDeclaration,
            Some(CheckedPackageRefusalCause::MissingName),
            Code::MissingDeclaration
        )
    );
    // Located at the frame node, in the state step (FR-342 check 5).
    let Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) = &x6 else {
        panic!("{x6:?}");
    };
    let rekeyed = frame_with_parameter_c(&emitted);
    assert_eq!(
        refusal.locus.as_ref().map(|node| node.digest.as_ref()),
        Some(rekeyed.as_str())
    );
}

/// The digest of the re-keyed frame node holding X6's `parameter c` entry.
fn frame_with_parameter_c(emitted: &Emitted) -> String {
    let mut envelope = emitted.wire.clone();
    let id = emitted.frame_id("merge");
    let node = envelope["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find(|node| node["node_id"] == id)
        .expect("the frame node");
    node["body"]["deletes_scoped"] = json!([
        {"kind": "receiver"},
        {"kind": "parameter", "name": "a"},
        {"kind": "parameter", "name": "b"},
        {"kind": "parameter", "name": "c"},
    ]);
    rekey_stale(&mut envelope);
    let nodes = envelope["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes");
    let frames: Vec<&Value> = nodes
        .iter()
        .filter(|node| {
            node["semantic_form"] == "frame"
                && node["body"]["deletes_scoped"]
                    .as_array()
                    .is_some_and(|entries| entries.len() == 4)
        })
        .collect();
    let [frame] = frames[..] else {
        panic!("one re-keyed frame: {frames:#?}");
    };
    frame["node_id"]["digest"]
        .as_str()
        .expect("digest")
        .to_owned()
}

/// FR-154-AC-10 (X7 to X9): the body of W1 re-keyed is admitted; two entries
/// for `step` of both scopes, in either listed order, refuse
/// `invalid_model_binding`/`conflicting-binding` and not
/// `invalid_semantic_graph`.
#[trace("TC-1043", "FR-154-AC-10")]
#[test]
fn both_scopes_of_one_field_refuse_conflicting_binding_in_either_order() {
    let emitted = emit(&ALL, &[]);
    let counter = emitted.counter();
    let step = |scope: &str| json!({"declaration": node_ref(&counter), "kind": "field", "name": "step", "scope": scope});
    let value = json!({"declaration": node_ref(&counter), "kind": "field", "name": "value", "scope": "every-object"});
    let x7 = read_edited(&emitted, "bump", |_| {});
    assert!(matches!(x7, Read::Verified { .. }), "X7: {x7:?}");
    let x8 = read_edited(&emitted, "bump", |body| {
        body["modifies"] = json!([step("receiver"), step("every-object"), value.clone()]);
    });
    let x9 = read_edited(&emitted, "bump", |body| {
        body["modifies"] = json!([step("every-object"), step("receiver"), value.clone()]);
    });
    for (row, read) in [("X8", x8), ("X9", x9)] {
        assert_eq!(
            refusal(&read),
            (
                CheckedPackageRefusalCode::InvalidModelBinding,
                Some(CheckedPackageRefusalCause::ConflictingBinding),
                Code::InvalidModelBinding
            ),
            "{row}"
        );
    }
}
