// SPDX-License-Identifier: AGPL-3.0-or-later
//! Behavioral tests for the I2 `quire.checked-package/v2` byte reader
//! (ADR-011 §4, QSpec FR-322). Test bytes are built inline, following
//! `tests/library_resolution.rs`'s own approach: no QSpec fixtures are
//! vendored into this repo. Fixtures build a complete, minimal
//! `quire.checked-package/v2` wire (one semantic-graph node, one locked
//! source, an edition and a diagnostics catalog) because IR's v2 reader
//! validates the whole I04 contract unconditionally.

use ix_trace_rs::trace;
use quire_contract_model::{
    CheckedArtifactRef, CheckedExtensionDefinition, CheckedPackageEvidence,
    CheckedPackageRefusalCause, CheckedPackageRefusalCode, CHECKED_PACKAGE_V2,
};
use serde_json::{json, Value};

use super::{read_checked_package_v2, read_v2, Read, V2ReadLimits, V2ReadRefusal};
use std::collections::BTreeMap;

use qsl_foundation::diagnostic::{Code, LimitExceeded, LimitKind, Locus};
use qsl_foundation::node_wire::node_key_from_json;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::Setting;
use qsl_semantics::check::imports::ImportedNames;
use qsl_semantics::check::wire_id::WireGraph;
use qsl_semantics::check::CheckCause;
use qsl_semantics::library::{ImportView, LibraryName};
use quire_exact::{NodeKey, Origin, Role};
use quire_semantic_value::catalog::{CatalogCode, CatalogCoded, Category};

mod protocol_corpus;

fn node_ref(label: &str) -> Value {
    json!({"package": "pkg", "path": label.split("::").skip(1).collect::<Vec<_>>(), "ordinal": 0})
}

/// A QSpec `DefinitionRef`: exactly `{authority, identity}`.
fn artifact_ref(label: &str) -> Value {
    json!({"authority": "pkg", "identity": label})
}

/// A QSpec `RawSourceRef`: authority and identity.
fn source_ref(label: &str) -> Value {
    json!({
        "authority": "pkg",
        "identity": label,
    })
}

/// The `SourceOwner` of a declared node: the lock's one source, `src`
/// (QSpec FR-322 "Owner member schema").
fn declared_owner() -> Value {
    json!({"kind": "source", "authority": "pkg", "identity": "src"})
}

fn selection(role: &str, label: &str) -> Value {
    json!({"role": role, "definition": artifact_ref(label)})
}

/// A FR-322 `DependencySelection` names its direct dependency.
fn dependency_selection(identity: &str, _label: &str) -> Value {
    json!({"identity": identity})
}

/// Fields shared by a `semantic_graph` node and its identity-projection
/// counterpart: `quire_contract_model::CheckedNodeProjectionV2::from`'s own
/// definition of the projection is exactly a `CheckedSemanticNodeV2` with
/// `occurrences` dropped; IR derives the identity projection from
/// `semantic_graph.nodes` that way and the wire carries none. A
/// self-typed scalar with an empty aggregate body needs no other
/// node's identity and no frame/nominal validation. `export` is `R`'s own
/// `declaration`, paired with a `declaration`-role occurrence
/// (`validate_declaration`'s `DeclarationOccurrenceRule`).
fn node_fields(_label: &str, export: &str) -> Value {
    let reference = json!({"package": "pkg", "path": [export], "ordinal": 0});
    json!({
        "node_id": reference,
        "schema_version": "quire.checked-semantic-graph/v2",
        "node_tag": "scalar_type",
        "semantic_form": "text",
        "semantic_type": reference,
        "dependencies": [],
        "declaration": {"qualified_name": [export]},
        "owner": declared_owner(),
        "body": {"term": "aggregate", "members": []},
    })
}

fn node_fields_in_source(label: &str, export: &str, source: &str) -> Value {
    let mut node = node_fields(label, export);
    node["owner"]["identity"] = json!(source);
    node
}

fn graph_node(label: &str, export: &str) -> Value {
    let mut node = node_fields(label, export);
    node["occurrences"] = json!([{"role": "declaration", "ordinal": 0}]);
    node
}

fn graph_node_in_source(label: &str, export: &str, source: &str) -> Value {
    let mut node = node_fields_in_source(label, export, source);
    node["occurrences"] = json!([{"role": "declaration", "ordinal": 0}]);
    node
}

/// A minimal lock with the dependency selections supplied by the caller.
fn base_lock(dependency_selections: Vec<Value>) -> Value {
    json!({
        "sources": [source_ref("src")],
        "definition_selections": [],
        "dependency_selections": dependency_selections,
        "edition": selection("edition", "edition-def"),
        "profile_selections": [],
        "required_features": [],
    })
}

fn diagnostics() -> Value {
    json!({"catalog": artifact_ref("diag-catalog"), "entries": []})
}

fn jcs(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

/// A structurally valid `quire.checked-package/v2` envelope with one graph node.
fn valid_envelope(lock: &Value) -> Value {
    json!({
        "capability_report": [],
        "contract_version": CHECKED_PACKAGE_V2,
        "diagnostics": diagnostics(),
        "lock": lock,
        "name": "pkg",
        "semantic_graph": {
            "graph_version": "quire.checked-semantic-graph/v2",
            "nodes": [graph_node("pkg::R", "R")],
        },
        "source_map": [{
            "node_id": graph_node("pkg::R", "R")["node_id"],
            "role": "declaration",
            "ordinal": 0,
            "regions": [{"source": source_ref("src"), "start": 0, "end": 1}],
        }],
    })
}

fn identity(label: &str) -> LibraryName {
    LibraryName::new(label).unwrap()
}

/// The reader's own artifact byte ceiling at `limit`, reached by `actual`
/// offered bytes: no locus (FR-096).
fn input_bytes(limit: usize, actual: usize) -> LimitExceeded {
    LimitExceeded::new(
        Setting::I2InputBytes,
        u64::try_from(limit).unwrap(),
        u128::try_from(actual).unwrap(),
    )
}

fn read(bytes: &[u8]) -> Read {
    read_v2(
        bytes,
        identity("pkg"),
        V2ReadLimits::default(),
        &CheckedPackageEvidence::new(),
    )
}

/// TC-253 step 1: valid v2 bytes with the requested package name are admitted.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn accepts_valid_bytes() {
    let lock = base_lock(vec![]);
    let bytes = jcs(&valid_envelope(&lock));
    match read(&bytes) {
        Read::Verified {
            admitted, package, ..
        } => {
            assert_eq!(admitted.name(), "pkg");
            assert_eq!(package.exports().count(), 1);
        }
        other => panic!("expected Verified, got {other:?}"),
    }
}

/// A wire whose graph declares `exports` (fixture label, qualified name),
/// each a self-typed scalar with a declaration occurrence.
fn envelope_declaring(exports: &[(&str, &str)]) -> (Value, Value) {
    let mut sorted = exports.to_vec();
    sorted.sort_by_key(|(_, export)| *export);
    envelope_in_graph_order(&sorted)
}

/// A wire whose graph holds `exports` in the offered order.
fn envelope_in_graph_order(exports: &[(&str, &str)]) -> (Value, Value) {
    let lock = base_lock(vec![]);
    let mut envelope = valid_envelope(&lock);
    envelope["semantic_graph"]["nodes"] = exports
        .iter()
        .map(|(label, export)| graph_node(label, export))
        .collect();
    envelope["source_map"] = exports
        .iter()
        .zip(0_u64..)
        .map(|((label, export), start)| {
            json!({
                "node_id": node_fields(label, export)["node_id"],
                "role": "declaration",
                "ordinal": 0,
                "regions": [{"source": source_ref("src"), "start": start, "end": start + 1}],
            })
        })
        .collect();
    (lock, envelope)
}

/// FR-322 structural identities are independent of graph array order.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn reversed_node_order_is_admitted_with_the_same_export_identities() {
    let mut exports = vec![("pkg::Y", "A"), ("pkg::Z", "B")];
    let admitted = |exports: &[(&str, &str)]| {
        let (_, envelope) = envelope_in_graph_order(exports);
        match read(&jcs(&envelope)) {
            Read::Verified { package, .. } => package,
            other => panic!("expected Verified, got {other:?}"),
        }
    };
    let ascending = admitted(&exports);
    exports.reverse();
    let descending = admitted(&exports);
    assert_eq!(ascending, descending);
    assert_eq!(ascending.exports().count(), 2);
}

/// The import view of the verified wire declaring `exports` and the
/// independently decoded node identities from its graph.
fn import_view_of(exports: &[(&str, &str)]) -> (ImportView, BTreeMap<String, NodeKey>) {
    let (_, envelope) = envelope_declaring(exports);
    let verified = match read(&jcs(&envelope)) {
        Read::Verified { package, .. } => package,
        other => panic!("expected Verified, got {other:?}"),
    };
    let expected = envelope["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| {
            let name = node["declaration"]["qualified_name"][0]
                .as_str()
                .unwrap()
                .to_owned();
            let key = node_key_from_json(&node["node_id"]).expect("structural node identity");
            (name, key)
        })
        .collect();
    (verified, expected)
}

/// FR-087-AC-4, TC-254: the view carries each exported declaration at its
/// own structural node identity, independently of its name index.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn import_view_keys_each_export_by_its_structural_node_id() {
    let (view, expected) = import_view_of(&[("pkg::Y", "A"), ("pkg::Z", "B")]);
    let exports: Vec<_> = view.exports().collect();
    let mut expected_exports = vec![("A", &expected["A"]), ("B", &expected["B"])];
    expected_exports.sort_by_key(|(_, key)| *key);
    assert_eq!(exports, expected_exports);
    assert_ne!(expected["A"], expected["B"]);
}

/// FR-087-AC-4, TC-254: the importing checker indexes the view's names.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn the_checker_resolves_an_imported_name_over_the_view_entries() {
    let (view, expected) = import_view_of(&[("pkg::Y", "A"), ("pkg::Z", "B")]);
    let names = ImportedNames::of(&view);
    assert_eq!(names.resolve("A"), Ok(expected["A"].clone()));
    assert_eq!(names.resolve("B"), Ok(expected["B"].clone()));
    let missing = names.resolve("C").unwrap_err();
    assert_eq!(missing, CheckCause::MissingName("C".to_owned()));
    assert_eq!(missing.code(), Code::MissingDeclaration);
    assert_eq!(missing.cause(), Some("missing-name"));
}

/// A valid wire selected under another name refuses with both names typed.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_when_selected_name_disagrees_with_wire() {
    let bytes = jcs(&valid_envelope(&base_lock(vec![])));
    let outcome = read_v2(
        &bytes,
        identity("other"),
        V2ReadLimits::default(),
        &CheckedPackageEvidence::new(),
    );
    assert_eq!(
        outcome,
        Read::Refused(V2ReadRefusal::IdentityMismatch {
            expected: identity("other"),
            actual: "pkg".into(),
        })
    );
}

/// TC-253 step 8 on the wire path: an unsupported contract version
/// refuses with the named version cause and yields no admitted package.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_unsupported_version() {
    let lock = base_lock(vec![]);
    let mut envelope = valid_envelope(&lock);
    envelope["contract_version"] = json!("quire.checked-package/v1");
    let outcome = read(&jcs(&envelope));
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::UnsupportedVersion { wire, .. })
                if &*wire.actual == "quire.checked-package/v1"
        ),
        "expected Refused(UnsupportedVersion), got {outcome:?}"
    );
}

/// The refusal's stable `Code`.
fn envelope_code(outcome: &Read) -> Option<Code> {
    match outcome {
        Read::Refused(
            refusal @ (V2ReadRefusal::Envelope { .. } | V2ReadRefusal::UnsupportedVersion { .. }),
        ) => Some(refusal.code()),
        _ => None,
    }
}

#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_unknown_contract_version() {
    let lock = base_lock(vec![]);
    let mut envelope = valid_envelope(&lock);
    envelope["contract_version"] = json!("quire.checked-package/v1");
    let bytes = jcs(&envelope);
    let outcome = read(&bytes);
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::UnsupportedVersion { .. })
        ),
        "expected Refused(UnsupportedVersion), got {outcome:?}"
    );
    assert_eq!(envelope_code(&outcome), Some(Code::UnknownWire));
}

#[test]
fn refuses_missing_member_as_malformed_wire() {
    let lock = base_lock(vec![]);
    let mut envelope = valid_envelope(&lock).as_object().unwrap().clone();
    envelope.remove("diagnostics");
    let bytes = jcs(&Value::Object(envelope));
    let outcome = read(&bytes);
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::MalformedWire
        ),
        "expected Refused(Envelope(MalformedWire)), got {outcome:?}"
    );
}

#[test]
fn refuses_non_object_wire_as_malformed_wire() {
    let bytes = jcs(&json!(["not", "an", "object"]));
    let outcome = read(&bytes);
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::MalformedWire
        ),
        "expected Refused(Envelope(MalformedWire)), got {outcome:?}"
    );
}

/// Inject a duplicate/extra top-level member by textual mutation: the
/// `json!` macro's own `Map` cannot represent a duplicate key, so the
/// mutation happens on the serialized bytes, exactly as a hostile wire would
/// present it.
fn inject_top_level_member(bytes: &[u8], member: &str) -> Vec<u8> {
    let text = std::str::from_utf8(bytes).unwrap();
    let insert_at = text.find('{').expect("object") + 1;
    let mut mutated = String::with_capacity(text.len() + member.len());
    mutated.push_str(&text[..insert_at]);
    mutated.push_str(member);
    mutated.push_str(&text[insert_at..]);
    mutated.into_bytes()
}

#[test]
fn refuses_duplicate_top_level_member() {
    let lock = base_lock(vec![]);
    let bytes = jcs(&valid_envelope(&lock));
    let mutated = inject_top_level_member(
        &bytes,
        &format!("\"contract_version\":\"{CHECKED_PACKAGE_V2}\","),
    );
    // IR's own duplicate-member path is `serde_json`'s error text (the
    // member name plus its line/column), not a bare member name, so only
    // the code is asserted -- not IR's `path` text (N3).
    let outcome = read(&mutated);
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::DuplicateMember
        ),
        "expected Refused(Envelope(DuplicateMember)), got {outcome:?}"
    );
}

/// A wire whose bytes are not the RFC 8785 bytes of the document they hold
/// (one space after the opening brace) is refused by IR `noncanonical_wire`,
/// which maps to the native `Code::NoncanonicalWire`, not
/// `Code::InvalidPackage`.
#[trace("TC-429", "FR-096-AC-18")]
#[test]
fn maps_noncanonical_bytes_to_the_native_noncanonical_wire_code() {
    let lock = base_lock(vec![]);
    let bytes = inject_top_level_member(&jcs(&valid_envelope(&lock)), " ");
    let outcome = read(&bytes);
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::NoncanonicalWire
        ),
        "expected Refused(Envelope(NoncanonicalWire)), got {outcome:?}"
    );
    assert_eq!(envelope_code(&outcome), Some(Code::NoncanonicalWire));
}

#[test]
fn refuses_unrecognized_top_level_member() {
    let lock = base_lock(vec![]);
    let mut envelope = valid_envelope(&lock);
    envelope["extra_member"] = json!(null);
    let bytes = jcs(&envelope);
    let outcome = read(&bytes);
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::UnknownMember
        ),
        "expected Refused(Envelope(UnknownMember)), got {outcome:?}"
    );
}

/// Definition and source references admit only their declared members.
#[trace("TC-416", "FR-093-AC-17")]
#[test]
fn an_old_reference_shape_refuses_unknown_member() {
    let envelope = valid_envelope(&base_lock(vec![]));
    assert!(matches!(read(&jcs(&envelope)), Read::Verified { .. }));
    for (site, member) in [
        ("/lock/edition/definition", "digest"),
        ("/diagnostics/catalog", "revision"),
        ("/source_map/0/regions/0/source", "revision"),
    ] {
        let mut mutated = envelope.clone();
        mutated.pointer_mut(site).expect("reference site")[member] = json!("retired");
        let outcome = read(&jcs(&mutated));
        assert!(
            matches!(outcome, Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::UnknownMember),
            "{site}/{member} must refuse as an unknown member"
        );
    }
}

/// Equal structural node identities are duplicate nodes even if their source
/// owners or bodies differ (QSpec FR-322 structural identity rule).
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn duplicate_declared_node_id_refuses_at_the_second_node() {
    let (_, mut envelope) = envelope_declaring(&[("pkg::R", "R")]);
    let mut duplicate = graph_node_in_source("pkg::R", "R", "src2");
    duplicate["owner"]["identity"] = json!("src2");
    envelope["semantic_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    envelope["lock"]["sources"]
        .as_array_mut()
        .unwrap()
        .push(source_ref("src2"));
    let outcome = read(&jcs(&envelope));
    assert!(
        matches!(outcome, Read::Refused(V2ReadRefusal::Envelope { refusal, locus })
            if refusal.code == CheckedPackageRefusalCode::InvalidSemanticGraph
                && matches!(locus.as_deref(), Some(Locus::Artifact { pointer, .. })
                    if pointer.as_str() == "/semantic_graph/nodes/1/node_id")),
        "duplicate structural identity must refuse at the second node"
    );
}

/// FR-322 `dependency_selections`: a lock
/// carrying two well-shaped `DependencySelection` entries is read as far as
/// binding them. With no dependency package supplied for them, IR's reader
/// refuses `missing_import` at the first entry (QSpec FR-322-AC-36, IR-289);
/// a package with a supplied closure is admitted in `emit::tests`. An entry
/// in the retired `Selection` shape is a wire-shape refusal located under
/// `/lock/dependency_selections/0`.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn dependency_selections_reach_binding_and_refuse_unsupplied() {
    let dependencies = vec![
        dependency_selection("geometry", "geometry-pkg"),
        dependency_selection("units", "units-pkg"),
    ];
    let lock = base_lock(dependencies);
    let bytes = jcs(&valid_envelope(&lock));
    match read(&bytes) {
        Read::Refused(refusal @ V2ReadRefusal::Envelope { .. }) => {
            let V2ReadRefusal::Envelope { refusal: ir, .. } = &refusal else {
                unreachable!("matched above");
            };
            assert_eq!(ir.code, CheckedPackageRefusalCode::MissingImport);
            let Some(Locus::Artifact { pointer, .. }) = refusal.locus() else {
                panic!("an unsupplied entry is located, got {refusal:?}");
            };
            assert_eq!(pointer.as_str(), "/lock/dependency_selections/0");
        }
        other => panic!("expected missing_import, got {other:?}"),
    }

    let lock = base_lock(vec![selection("profile", "dep-def")]);
    let bytes = jcs(&valid_envelope(&lock));
    match read(&bytes) {
        Read::Refused(refusal @ V2ReadRefusal::Envelope { .. }) => {
            let V2ReadRefusal::Envelope { refusal: ir, .. } = &refusal else {
                unreachable!("matched above");
            };
            assert_eq!(ir.code, CheckedPackageRefusalCode::MalformedWire);
            let Some(Locus::Artifact { pointer, .. }) = refusal.locus() else {
                panic!("a shape refusal is located, got {refusal:?}");
            };
            assert!(
                pointer
                    .as_str()
                    .starts_with("/lock/dependency_selections/0"),
                "{pointer:?}"
            );
        }
        other => panic!("expected a wire-shape refusal, got {other:?}"),
    }
}

#[test]
fn incomplete_when_bytes_exceed_the_ceiling() {
    let lock = base_lock(vec![]);
    let bytes = jcs(&valid_envelope(&lock));
    let limits = V2ReadLimits {
        artifact_bytes: bytes.len() - 1,
        ..V2ReadLimits::default()
    };
    let outcome = read_v2(
        &bytes,
        identity("pkg"),
        limits,
        &CheckedPackageEvidence::new(),
    );
    assert_eq!(
        outcome,
        Read::Limit(input_bytes(bytes.len() - 1, bytes.len()))
    );
}

/// A valid wire of at least `min_bytes`, made large by one export whose
/// qualified name is long (it appears in the semantic graph).
fn wire_of_at_least(min_bytes: usize) -> Vec<u8> {
    let export = format!("R{}", "r".repeat(min_bytes));
    let (_, envelope) = envelope_declaring(&[("pkg::R", &export)]);
    let bytes = jcs(&envelope);
    assert!(bytes.len() >= min_bytes);
    bytes
}

/// A caller-raised `artifact_bytes` is enforced as given by this
/// reader *and* by IR, which receives it as its own `bytes` ceiling. A valid
/// wire past the 16 MiB default is refused at the byte-length gate by
/// default, and verified under a ceiling raised to its length -- not
/// stopped by IR at a fixed byte ceiling of its own.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn a_caller_raised_artifact_bytes_ceiling_admits_a_valid_wire_past_the_default() {
    let default_bytes = V2ReadLimits::default().artifact_bytes;
    let bytes = wire_of_at_least(default_bytes + 1);
    let read_under = |limits: V2ReadLimits| {
        read_v2(
            &bytes,
            identity("pkg"),
            limits,
            &CheckedPackageEvidence::new(),
        )
    };

    assert_eq!(
        read_under(V2ReadLimits::default()),
        Read::Limit(input_bytes(default_bytes, bytes.len())),
        "the default ceiling must refuse an oversized read at the byte-length gate"
    );

    let raised = V2ReadLimits {
        artifact_bytes: bytes.len(),
        ..V2ReadLimits::default()
    };
    match read_under(raised) {
        Read::Verified {
            effective_limits, ..
        } => assert_eq!(effective_limits, raised),
        other => panic!("expected Verified under the raised ceiling, got {other:?}"),
    }
}

/// IR's own ceilings are the caller's too. A wire with more
/// semantic-graph nodes than IR's 10,000-node default is `Incomplete` at
/// IR's `nodes` meter under the default `nodes`, naming that bound, and
/// verified once the caller raises `nodes`.
#[trace("TC-253", "FR-087-AC-3")]
#[trace("TC-739", "FR-264-AC-4")]
#[test]
fn a_caller_raised_ir_node_ceiling_admits_past_the_ir_default() {
    let default_nodes = V2ReadLimits::default().nodes;
    let count = usize::try_from(default_nodes).unwrap() + 1;
    let labels: Vec<(String, String)> = (0..count)
        .map(|index| (format!("pkg::N{index}"), format!("N{index}")))
        .collect();
    let exports: Vec<(&str, &str)> = labels
        .iter()
        .map(|(label, export)| (label.as_str(), export.as_str()))
        .collect();
    let (_, envelope) = envelope_declaring(&exports);
    let bytes = jcs(&envelope);
    let read_under = |limits: V2ReadLimits| {
        read_v2(
            &bytes,
            identity("pkg"),
            limits,
            &CheckedPackageEvidence::new(),
        )
    };

    // So many nodes exceed the 16 MiB byte default too; admit the bytes so
    // only IR's node ceiling is in play.
    let base = V2ReadLimits {
        artifact_bytes: bytes.len(),
        ..V2ReadLimits::default()
    };
    match read_under(base) {
        Read::Limit(exceeded) => {
            assert_eq!(exceeded.kind(), LimitKind::NodeCount);
            assert_eq!(exceeded.configured_bound(), default_nodes);
            assert_eq!(exceeded.actual(), u128::from(default_nodes) + 1);
            assert!(matches!(exceeded.locus(), Some(Locus::Artifact { .. })));
        }
        other => panic!("expected Incomplete(Limit(Nodes)) at IR's default, got {other:?}"),
    }

    let raised = V2ReadLimits {
        nodes: default_nodes + 1,
        ..base
    };
    match read_under(raised) {
        Read::Verified {
            effective_limits, ..
        } => assert_eq!(effective_limits, raised),
        other => panic!("expected Verified under the raised node ceiling, got {other:?}"),
    }
}

/// FR-264-AC-1's read-back half: a flat package of 100,000 nodes, read with
/// the limits raised to fit it, is a `V2Read` that clones, compares equal to
/// its clone, formats for debug and drops on a 512 KiB stack, since its
/// decoded nodes have a fixed depth.
#[trace("TC-738", "FR-264-AC-1")]
#[test]
fn a_100000_node_v2_read_clones_compares_formats_and_drops_on_a_small_stack() {
    const NODES: usize = 100_000;
    let labels: Vec<(String, String)> = (0..NODES)
        .map(|index| (format!("pkg::N{index}"), format!("N{index}")))
        .collect();
    let exports: Vec<(&str, &str)> = labels
        .iter()
        .map(|(label, export)| (label.as_str(), export.as_str()))
        .collect();
    let (_, envelope) = envelope_declaring(&exports);
    let bytes = jcs(&envelope);
    let limits = V2ReadLimits {
        artifact_bytes: bytes.len(),
        nodes: 2 * NODES as u64,
        edges: 2 * NODES as u64,
        occurrences: 4 * NODES as u64,
        work: 1 << 40,
        ..V2ReadLimits::default()
    };
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || {
            let read = read_checked_package_v2(
                &bytes,
                identity("pkg"),
                limits,
                &CheckedPackageEvidence::new(),
            )
            .unwrap_or_else(|failure| panic!("expected a verified read, got {failure:?}"))
            .into_value();
            let clone = read.clone();
            assert!(read == clone, "the read equals its clone");
            assert!(!format!("{read:?}").is_empty());
            drop(clone);
            drop(read);
        })
        .expect("the thread spawns")
        .join()
        .expect("clone, compare, format and drop complete on a 512 KiB stack");
}

/// Reaching a *caller-raised* ceiling (not just the default)
/// still refuses, naming the limit kind ([`LimitKind::InputBytes`]) and
/// the caller's own configured bound.
#[test]
fn reaching_a_caller_raised_artifact_bytes_ceiling_refuses_naming_the_kind_and_bound() {
    let raised = V2ReadLimits {
        artifact_bytes: V2ReadLimits::default().artifact_bytes + 1_000,
        ..V2ReadLimits::default()
    };
    let oversized = vec![b'x'; raised.artifact_bytes + 1];
    let outcome = read_v2(
        &oversized,
        identity("pkg"),
        raised,
        &CheckedPackageEvidence::new(),
    );
    assert_eq!(
        outcome,
        Read::Limit(input_bytes(raised.artifact_bytes, oversized.len())),
        "must name the raised ceiling actually in force, not the original default"
    );
}

#[test]
fn exact_selected_limits_admit_the_boundary() {
    let lock = base_lock(vec![]);
    let bytes = jcs(&valid_envelope(&lock));
    let limits = V2ReadLimits {
        artifact_bytes: bytes.len(),
        ..V2ReadLimits::default()
    };
    let outcome = read_v2(
        &bytes,
        identity("pkg"),
        limits,
        &CheckedPackageEvidence::new(),
    );
    assert!(matches!(outcome, Read::Verified { .. }));
}

/// A wire nested 100,000 deep, read on a
/// 512 KiB stack, is refused as a malformed wire. The outcome is no limit
/// and names no depth.
#[trace("TC-739", "FR-264-AC-5")]
#[test]
fn a_wire_nested_100000_deep_reads_without_a_depth_outcome_on_a_small_stack() {
    const WRAPS: usize = 100_000;
    let bytes = format!("{}1{}", "[".repeat(WRAPS), "]".repeat(WRAPS)).into_bytes();
    let outcome = std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || {
            let outcome = read_v2(
                &bytes,
                identity("pkg"),
                V2ReadLimits {
                    artifact_bytes: bytes.len(),
                    ..V2ReadLimits::default()
                },
                &CheckedPackageEvidence::new(),
            );
            matches!(
                &outcome,
                Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                    if refusal.code == CheckedPackageRefusalCode::MalformedWire
            )
        })
        .expect("the thread spawns")
        .join()
        .expect("the read completes on a 512 KiB stack");
    assert!(outcome, "expected Refused(Envelope(MalformedWire))");
}

/// The occurrence key of `label`'s `declaration` occurrence.
fn declaration_key(label: &str) -> OccurrenceKey {
    OccurrenceKey::new(
        node_key_from_json(&node_ref(label)).expect("fixture declaration identity"),
        Origin::new(Role::new("declaration"), 0),
    )
}

/// ADR-013 O-12, C-14: the verified read carries the wire's `source_map` as
/// the package source map. Each occurrence key maps to exactly its wire
/// regions, in wire order, under the wire's `RawSourceRef`; each node's
/// occurrences are exactly its own entries.
#[trace("TC-421", "FR-095-AC-3")]
#[test]
fn a_verified_read_carries_the_wire_source_map() {
    let (_, mut envelope) = envelope_declaring(&[("pkg::A", "A"), ("pkg::B", "B")]);
    let second = envelope["source_map"][1]["regions"][0].clone();
    envelope["source_map"][1]["regions"] = json!([
        {"source": source_ref("src"), "start": 7, "end": 9},
        second,
    ]);
    let source_map = match read(&jcs(&envelope)) {
        Read::Verified { source_map, .. } => source_map,
        other => panic!("expected Verified, got {other:?}"),
    };
    let entries = envelope["source_map"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    for entry in entries {
        let node = node_key_from_json(&entry["node_id"]).expect("wire node identity");
        let key = OccurrenceKey::new(node.clone(), Origin::new(Role::new("declaration"), 0));
        let regions = source_map
            .regions(&key)
            .unwrap_or_else(|| panic!("no regions for {key}"));
        let spans: Vec<(u64, u64)> = regions.iter().map(|r| (r.start(), r.end())).collect();
        let expected: Vec<(u64, u64)> = entry["regions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r["start"].as_u64().unwrap(), r["end"].as_u64().unwrap()))
            .collect();
        assert_eq!(spans, expected);
        for region in regions {
            let source = region.source();
            assert_eq!((source.authority(), source.identity()), ("pkg", "src"));
        }
        assert_eq!(
            source_map
                .occurrences(&node)
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>(),
            [key]
        );
    }
    assert_eq!(source_map.regions(&declaration_key("pkg::C")), None);
}

/// ADR-013 O-07: a source-map entry naming a node the graph does not hold
/// refuses at the read, as `invalid_source_map`.
#[trace("TC-421", "FR-095-AC-4")]
#[test]
fn a_source_map_entry_naming_an_unknown_node_refuses() {
    let (_, mut envelope) = envelope_declaring(&[("pkg::A", "A")]);
    envelope["source_map"][0]["node_id"] = node_ref("pkg::unknown");
    match read(&jcs(&envelope)) {
        Read::Refused(refusal) => {
            assert!(
                matches!(
                    &refusal,
                    V2ReadRefusal::Envelope { refusal: ir, .. }
                        if ir.code == CheckedPackageRefusalCode::InvalidSourceMap
                ),
                "{refusal:?}"
            );
            assert_eq!(refusal.code(), Code::InvalidSourceMap);
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

/// A source-map entry that IR admitted but this crate's provenance types
/// refuse -- here a reversed region and an empty structural node path, which IR's
/// reader never admits -- is `invalid_source_map`, carrying the typed
/// defect, never a panic or a silently dropped entry.
#[trace("TC-421", "FR-095-AC-3")]
#[test]
fn a_source_map_the_provenance_types_refuse_is_invalid_source_map() {
    let entry = |node_id: Value, start: u64, end: u64| {
        serde_json::from_value::<quire_contract_model::CheckedSourceMapEntry>(json!({
            "node_id": node_id,
            "role": "expression",
            "ordinal": 0,
            "regions": [{"source": source_ref("src"), "start": start, "end": end}],
        }))
        .unwrap()
    };
    let reversed = super::package_source_map(
        &qsl_semantics::check::wire_id::WireGraph::default(),
        &[entry(node_ref("pkg::A"), 9, 3)],
    )
    .unwrap_err();
    assert_eq!(
        reversed,
        super::SourceMapDefect::Provenance(
            qsl_foundation::source::provenance::InvalidProvenance::ReversedRegion {
                start: 9,
                end: 3
            }
        )
    );
    let node = super::package_source_map(
        &qsl_semantics::check::wire_id::WireGraph::default(),
        &[entry(
            json!({"package": "pkg", "path": [], "ordinal": 0}),
            0,
            1,
        )],
    )
    .unwrap_err();
    assert_eq!(node, super::SourceMapDefect::NodeId);
    for defect in [reversed, node] {
        assert_eq!(V2ReadRefusal::from(defect).code(), Code::InvalidSourceMap);
    }
}

/// ADR-013 O-07: every IR occurrence role maps to its FR-322 spelling, the
/// same string IR itself serializes the role as. The list names all six
/// FR-322 roles, so a swapped pair fails.
#[trace("TC-421", "FR-095-AC-3")]
#[test]
fn every_occurrence_role_is_spelled_as_ir_serializes_it() {
    use quire_contract_model::CheckedOccurrenceRole as R;
    for (role, spelling) in [
        (R::Declaration, "declaration"),
        (R::Type, "type"),
        (R::Expression, "expression"),
        (R::Anchor, "anchor"),
        (R::Claim, "claim"),
        (R::Generated, "generated"),
    ] {
        assert_eq!(super::role_spelling(&role), spelling);
        assert_eq!(super::occurrence_role(spelling), Some(role.clone()));
        assert_eq!(serde_json::to_value(&role).unwrap(), json!(spelling));
    }
    assert_eq!(super::occurrence_role("reference"), None);
}

/// The published QSpec checked-package-v2 fixture directory under
/// `the QSpec checkout`, and its `positive-*.json` fixtures in name order.
fn qspec_v2_fixtures() -> Option<(std::path::PathBuf, Vec<std::path::PathBuf>)> {
    let qspec = qspec_directory()?;
    let directory = std::path::Path::new(&qspec).join("proposals/checked-package-v2/fixtures");
    let mut paths: Vec<_> = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("reading {}: {error}", directory.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("positive-") && name.ends_with(".json"))
        })
        .collect();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "{} holds no positive fixture",
        directory.display()
    );
    Some((directory, paths))
}

/// Published fixtures read with their selected domain documents, as
/// paths relative to QSpec's `proposals/checked-package-v2`.
const ALL_FAMILIES: &str = "fixtures/positive-all-families.json";
const TIMED_MODEL_NODES: &str = "fixtures/positive-timed-model-nodes.json";
const TICK_TIMED_MODEL_NODES: &str = "fixtures/positive-tick-timed-model-nodes.json";

/// The path of the published fixture at `path` relative to QSpec's
/// `proposals/checked-package-v2`, as `selected-documents.json` keys it.
fn published_fixture(path: &std::path::Path) -> String {
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a fixture file name");
    format!("fixtures/{file}")
}

fn read_fixture(path: &std::path::Path) -> Value {
    let bytes =
        std::fs::read(path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parsing {}: {error}", path.display()))
}

/// Supplies, as extension definition evidence, every definition QSpec
/// publishes in `extension-definitions.json` under `the QSpec checkout` for a
/// `definition_selections` entry of `envelope`, except those in `withheld`
/// (QSpec FR-469 "Published definitions"). A selection with no published
/// definition (an `agent-ix` value or rule selection) is not held; an
/// absent file holds nothing, so the reader refuses
/// `unknown_required_capability` at the extension node.
fn supply_qspec_extension_definitions(
    evidence: &mut CheckedPackageEvidence,
    envelope: &Value,
    withheld: &[CheckedArtifactRef],
) {
    #[derive(serde::Deserialize)]
    struct Published {
        reference: CheckedArtifactRef,
        definition: CheckedExtensionDefinition,
    }
    #[derive(serde::Deserialize)]
    struct File {
        definitions: Vec<Published>,
    }
    let Some(qspec) = qspec_directory() else {
        return;
    };
    let path = std::path::Path::new(&qspec)
        .join("proposals/checked-package-v2/extension-definitions.json");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => panic!("reading {}: {error}", path.display()),
    };
    let file: File = serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parsing {}: {error}", path.display()));
    let Some(selections) = envelope["lock"]["definition_selections"].as_array() else {
        return;
    };
    for published in file.definitions {
        let selected = selections.iter().any(|row| {
            row["authority"].as_str() == Some(&*published.reference.authority)
                && row["identity"].as_str() == Some(&*published.reference.identity)
        });
        if selected && !withheld.contains(&published.reference) {
            evidence
                .insert_extension_definition(published.reference, published.definition)
                .expect("a fresh extension definition key");
        }
    }
}

/// Supplies, as dependency package evidence, the admitted package of every
/// `dependency_selections` entry of `envelope` (QSpec FR-322-AC-37: a
/// `dependency_reference` callee is admitted when the dependency's admitted
/// package is supplied under its identity). The package is the published
/// `positive-*.json` fixture whose name the selection names, itself read
/// against its own [`fixture_evidence`].
fn supply_qspec_dependency_packages(evidence: &mut CheckedPackageEvidence, envelope: &Value) {
    let Some((_, fixtures)) = qspec_v2_fixtures() else {
        return;
    };
    let Some(selections) = envelope["lock"]["dependency_selections"].as_array() else {
        return;
    };
    for selection in selections {
        // A malformed entry (the selection vectors' shape defects) supplies
        // nothing; the reader refuses it by its shape.
        let Some(identity_text) = selection["identity"].as_str() else {
            continue;
        };
        // A selection naming no published fixture supplies nothing, so the
        // reader's missing-import refusal remains observable.
        let Some((path, library)) = fixtures
            .iter()
            .map(|path| (path, read_fixture(path)))
            .find(|(_, candidate)| candidate["name"].as_str() == Some(identity_text))
        else {
            continue;
        };
        // The dependency is read against its own whole evidence, the
        // domain packages of its selected models included.
        let library_fixture = published_fixture(path);
        let inner = fixture_evidence(Some(&library_fixture), &library, &library, &[]);
        let admitted = read_checked_package_v2(
            &fixture_wire_bytes(&library),
            identity(identity_text),
            V2ReadLimits::default(),
            &inner,
        )
        .unwrap_or_else(|failure| panic!("dependency {identity_text} is admitted: {failure:?}"))
        .into_value()
        .admitted;
        evidence
            .insert_dependency_package(identity_text, admitted)
            .expect("a fresh evidence key");
    }
}

/// The canonical bytes IR's reader receives for `envelope`.
fn fixture_wire_bytes(envelope: &Value) -> Vec<u8> {
    jcs(envelope)
}

/// QSL's whole I2 read of `envelope`, the published fixture `fixture` (a
/// path relative to QSpec's `proposals/checked-package-v2`) or a mutation
/// of it, under its declared name. Evidence treats the fixture's required
/// features as supported. The published fixture is pretty-printed; the
/// wire is its canonical form.
fn read_fixture_wire(fixture: &str, envelope: &Value) -> (String, Read) {
    read_fixture_wire_withholding(fixture, envelope, &[])
}

/// [`read_fixture_wire`] with the extension definitions named by
/// `withheld` left out of the held evidence (QSpec FR-469 "Published
/// definitions": the withheld-definition control).
fn read_fixture_wire_withholding(
    fixture: &str,
    envelope: &Value,
    withheld: &[CheckedArtifactRef],
) -> (String, Read) {
    read_fixture_wire_holding(Some(fixture), envelope, envelope, withheld)
}

/// [`read_fixture_wire_withholding`] holding the published definitions
/// that `selecting`'s lock selects, rather than `envelope`'s own. A locally
/// built envelope derives from no published `fixture` and selects no domain
/// document.
fn read_fixture_wire_holding(
    fixture: Option<&str>,
    envelope: &Value,
    selecting: &Value,
    withheld: &[CheckedArtifactRef],
) -> (String, Read) {
    let evidence = fixture_evidence(fixture, envelope, selecting, withheld);
    let name = envelope["name"].as_str().expect("fixture name").to_owned();
    let outcome = read_v2(
        &fixture_wire_bytes(envelope),
        identity(&name),
        V2ReadLimits::default(),
        &evidence,
    );
    (name, outcome)
}

/// The evidence `envelope` is read against: the published definitions that
/// `selecting`'s lock selects, minus `withheld`, the required features, the
/// dependency packages `envelope` selects, and the domain document that
/// QSpec pairs with `fixture` when `envelope` has a model root of its
/// identity.
fn fixture_evidence(
    fixture: Option<&str>,
    envelope: &Value,
    selecting: &Value,
    withheld: &[CheckedArtifactRef],
) -> CheckedPackageEvidence {
    let mut evidence = CheckedPackageEvidence::new();
    supply_qspec_extension_definitions(&mut evidence, selecting, withheld);
    for feature in envelope["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    supply_qspec_dependency_packages(&mut evidence, envelope);
    let Some((identity, document)) = fixture.and_then(selected_domain_document) else {
        return evidence;
    };
    let selects_document = envelope["semantic_graph"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|node| {
            node["semantic_form"] == "model_root"
                && node["owner"]["identity"].as_str() == Some(&*identity)
        });
    if selects_document {
        evidence
            .insert_domain_package_document(identity, document)
            .expect("unique domain document");
    }
    evidence
}

/// The published domain package document QSpec's
/// `proposals/checked-package-v2/selected-documents.json` pairs with
/// `fixture` (TC-233: several documents carry `acme/orders`, so a fixture,
/// and a mutation of it, is read against its own document only), with the
/// identity that document carries. `None` for a fixture with no entry.
fn selected_domain_document(fixture: &str) -> Option<(String, Vec<u8>)> {
    let proposals = qspec_directory()?.join("proposals/checked-package-v2");
    let pairing = read_fixture(&proposals.join("selected-documents.json"));
    let document = pairing["selected_documents"]
        .get(fixture)?
        .as_str()
        .unwrap_or_else(|| panic!("{fixture}: the selected document is a path"));
    let path = proposals.join(document);
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let decoded: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parsing {}: {error}", path.display()));
    let identity = decoded["package"]["identity"]
        .as_str()
        .unwrap_or_else(|| panic!("{document} carries a package identity"))
        .to_owned();
    Some((identity, bytes))
}

/// IR's admitted package for `envelope`, the admitted published fixture `fixture`.
fn admitted_fixture(
    fixture: &str,
    envelope: &Value,
) -> std::sync::Arc<quire_contract_model::CheckedPackageV2> {
    let evidence = fixture_evidence(Some(fixture), envelope, envelope, &[]);
    let read = read_checked_package_v2(
        &fixture_wire_bytes(envelope),
        identity(envelope["name"].as_str().unwrap_or("pkg")),
        V2ReadLimits::default(),
        &evidence,
    )
    .unwrap_or_else(|failure| panic!("the fixture is admitted: {failure:?}"));
    read.into_value().admitted
}

/// The typed graph nodes and source map of a read package.
pub(crate) type FixtureGraph = (
    Vec<quire_contract_model::CheckedSemanticNodeV2>,
    Vec<quire_contract_model::CheckedSourceMapEntry>,
);

/// IR's whole read of the published fixture `fixture` (a path relative to
/// QSpec's `proposals/checked-package-v2`) after the optional JSON `patch`:
/// the typed graph nodes and source map it admits, or the refusal code and
/// the primary locus pointer. Tests read typed nodes this way:
/// `CheckedTerm` has no serde decoder.
pub(crate) fn read_fixture_graph(
    fixture: &str,
    patch: Option<&Value>,
) -> Result<FixtureGraph, (CheckedPackageRefusalCode, Option<String>)> {
    let directory = qspec_directory().expect("QSPEC_DIR supplies owner fixtures");
    let mut envelope = read_fixture(&directory.join("proposals/checked-package-v2").join(fixture));
    if let Some(patch) = patch {
        apply_json_patch(&mut envelope, patch);
    }
    match read_fixture_wire(fixture, &envelope).1 {
        Read::Verified { .. } => {
            let package = admitted_fixture(fixture, &envelope);
            Ok((package.graph().nodes.clone(), package.source_map().to_vec()))
        }
        Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => {
            Err((refusal.code, locus_pointer(&refusal)))
        }
        other => panic!("{fixture}: unexpected outcome {other:?}"),
    }
}

/// [`read_fixture_graph`] of a fixture the reader admits.
pub(crate) fn admitted_fixture_graph(fixture: &str, patch: Option<&Value>) -> FixtureGraph {
    read_fixture_graph(fixture, patch)
        .unwrap_or_else(|refusal| panic!("{fixture} is admitted: {refusal:?}"))
}

/// ADR-013 C-14 over QSpec's published positive `quire.checked-package/v2`
/// fixtures, read at run time from
/// `the QSpec checkout/proposals/checked-package-v2/fixtures/positive-*.json`: QSL's
/// whole I2 read (`read_checked_package_v2`) admits each fixture, and every
/// one of its `source_map` entries looks up in the verified read's package
/// source map, by its occurrence key, to exactly its wire regions, while
/// each node's occurrences number exactly its entries.
///
/// Read directly from the QSpec checkout; nothing of QSpec is
/// copied into this repository.
#[trace("TC-421", "FR-095-AC-3", "TC-1111", "NFR-015-AC-6")]
#[test]
fn conformance_c14_source_map_lookup_over_qspec_positive_fixtures() {
    let Some((_, paths)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let mut looked_up = 0_usize;
    for path in &paths {
        let envelope = read_fixture(path);
        let source_map = match read_fixture_wire(&published_fixture(path), &envelope).1 {
            Read::Verified { source_map, .. } => source_map,
            other => panic!("{}: expected Verified, got {other:?}", path.display()),
        };
        let entries = envelope["source_map"].as_array().unwrap();
        let graph = WireGraph::of_package_value(&envelope).expect("the fixture graph");
        let mut per_node = BTreeMap::<NodeKey, usize>::new();
        for entry in entries {
            let node = graph
                .key_of_value(&entry["node_id"])
                .expect("fixture node identity");
            let key = OccurrenceKey::new(
                node.clone(),
                Origin::new(
                    Role::new(entry["role"].as_str().unwrap()),
                    entry["ordinal"].as_u64().unwrap(),
                ),
            );
            let regions = source_map
                .regions(&key)
                .unwrap_or_else(|| panic!("{}: no regions for {key}", path.display()));
            let wire = entry["regions"].as_array().unwrap();
            assert_eq!(regions.len(), wire.len(), "{}: {key}", path.display());
            for (region, wire) in regions.iter().zip(wire) {
                assert_eq!(region.start(), wire["start"].as_u64().unwrap());
                assert_eq!(region.end(), wire["end"].as_u64().unwrap());
                let source = region.source();
                assert_eq!(source.authority(), wire["source"]["authority"]);
                assert_eq!(source.identity(), wire["source"]["identity"]);
            }
            *per_node.entry(node).or_default() += 1;
            looked_up += 1;
        }
        for (node, count) in per_node {
            assert_eq!(
                source_map.occurrences(&node).count(),
                count,
                "{}",
                path.display()
            );
        }
    }
    println!(
        "conformance: {looked_up} source-map entries over {} positive fixtures",
        paths.len()
    );
}

/// FR-320-AC-4: the published nullary and reference-payload union
/// values carry distinct structural application identities and read through I2.
#[trace("FR-320-AC-4")]
#[test]
fn qspec_union_value_application_keys_are_admitted_by_i2() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let envelope = read_fixture(&directory.join("positive-union-nodes.json"));
    let nodes = envelope["semantic_graph"]["nodes"]
        .as_array()
        .expect("positive union fixture has semantic graph nodes");
    let union_values: Vec<_> = nodes
        .iter()
        .filter(|node| node["node_tag"] == "value" && node["semantic_form"] == "union_value")
        .collect();
    let mut members = BTreeMap::new();
    let mut identities = std::collections::BTreeSet::new();
    let graph = WireGraph::of_package_value(&envelope).expect("the fixture graph");
    for node in union_values {
        let member = node["body"]["members"][0]["name"]
            .as_str()
            .expect("union value has a named member");
        let id = &node["node_id"];
        assert_eq!(id["package"], envelope["name"]);
        assert_eq!(id["path"][0], "value");
        assert_eq!(id["path"][1], "union_value");
        assert!(identities.insert(graph.key_of_value(id).expect("structural union identity")));
        members.insert(member, node);
    }
    assert_eq!(members.len(), 2, "fixture has both union vectors");
    assert_eq!(
        members["Empty"]["body"]["members"][0]["value"],
        json!({"term": "aggregate", "members": []}),
        "nullary vector has an empty aggregate payload"
    );
    assert_eq!(
        members["Rect"]["body"]["members"][0]["value"]["term"], "aggregate",
        "reference-payload vector keeps its aggregate payload"
    );
    match read_fixture_wire("fixtures/positive-union-nodes.json", &envelope) {
        (name, Read::Verified { admitted, .. }) => assert_eq!(admitted.name(), name),
        (_, other) => panic!("expected I2 to admit union vectors, got {other:?}"),
    }
}

/// QSL's whole I2 read (`read_checked_package_v2`) admits every
/// published positive `quire.checked-package/v2` fixture under its name.
/// Graph order is independent of structural node identity. Each published
/// structural mutation of `positive-all-families.json` still refuses with
/// its expected cause.
///
/// Read directly from the QSpec checkout; nothing of QSpec is
/// copied into this repository.
#[trace("TC-253", "FR-087-AC-3", "TC-1111", "NFR-015-AC-6")]
#[test]
fn conformance_i2_read_over_qspec_checked_package_v2_fixtures() {
    let Some((directory, paths)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let mut unsorted = 0_usize;
    for path in &paths {
        let envelope = read_fixture(path);
        let keys: Vec<NodeKey> = WireGraph::of_package_value(&envelope)
            .expect("fixture node identities")
            .keys()
            .to_vec();
        if !keys.is_sorted() {
            unsorted += 1;
        }
        match read_fixture_wire(&published_fixture(path), &envelope) {
            (name, Read::Verified { admitted, .. }) => {
                assert_eq!(admitted.name(), name, "{}", path.display());
            }
            (_, other) => panic!("{}: expected Verified, got {other:?}", path.display()),
        }
    }
    assert!(
        unsorted > 0,
        "no positive fixture lists its nodes out of ascending order"
    );
    println!(
        "conformance: {} positive fixtures through QSL's full I2 read ({unsorted} in non-ascending node order)",
        paths.len()
    );

    let adverse = read_fixture(&directory.join("adverse.json"));
    let base = read_fixture(&directory.join("positive-all-families.json"));
    let mutations = adverse["structural_mutations"].as_array().unwrap();
    for mutation in mutations {
        let id = mutation["id"].as_str().unwrap();
        let expected = match mutation["outcome"].as_str().unwrap() {
            "refused:unknown_contract_version" => CheckedPackageRefusalCode::UnknownContractVersion,
            "refused:unsupported_node_tag" => CheckedPackageRefusalCode::UnsupportedNodeTag,
            "refused:invalid_semantic_graph" => CheckedPackageRefusalCode::InvalidSemanticGraph,
            "refused:invalid_package" => CheckedPackageRefusalCode::InvalidPackage,
            other => panic!("{id}: unmapped adverse outcome {other}"),
        };
        let mut candidate = base.clone();
        *candidate
            .pointer_mut(mutation["pointer"].as_str().unwrap())
            .unwrap_or_else(|| panic!("{id}: pointer exists")) = mutation["replacement"].clone();
        let (_, outcome) = read_fixture_wire(ALL_FAMILIES, &candidate);
        // FR-096: an unknown contract version is `unknown_wire`, located at
        // `/contract_version`; every other IR refusal keeps IR's code.
        let refused_as_expected = match &outcome {
            Read::Refused(refusal @ V2ReadRefusal::UnsupportedVersion { .. }) => {
                expected == CheckedPackageRefusalCode::UnknownContractVersion
                    && refusal
                        .locus()
                        .is_some_and(|locus| matches!(locus, Locus::Artifact { pointer, .. } if pointer.as_str() == "/contract_version"))
            }
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => refusal.code == expected,
            _ => false,
        };
        assert!(
            refused_as_expected,
            "{id}: expected a refusal as {expected:?}, got {outcome:?}"
        );
    }
    println!(
        "conformance: {} adverse mutations refused with their expected causes",
        mutations.len()
    );
    unknown_temporal_profile_is_unknown_profile(&base);
}

/// FR-447-AC-13 (QSpec FR-481): QSL's I2 read admits `positive-all-families.json`
/// with its three `hybrid` nodes (the `Tank` model, mode and transition), and
/// refuses a `hybrid_model` whose body binds `modes` before `variables`
/// `malformed_wire`. Read from the QSpec checkout by `make conformance`.
#[trace(
    "TC-685",
    "FR-230-AC-36",
    "FR-230-AC-37",
    "QSpec-TC-367",
    "QSpec-FR-322-AC-126",
    "QSpec-FR-322-AC-127",
    "FR-447-AC-13"
)]
#[test]
fn conformance_hybrid_nodes_are_admitted_and_a_swapped_model_body_is_refused() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let base = read_fixture(&directory.join("positive-all-families.json"));
    let nodes = base["semantic_graph"]["nodes"].as_array().unwrap();
    let hybrid = nodes
        .iter()
        .filter(|node| node["node_tag"] == "hybrid")
        .count();
    assert_eq!(
        hybrid, 3,
        "the fixture holds Tank's model, mode and transition"
    );
    match read_fixture_wire(ALL_FAMILIES, &base) {
        (_, Read::Verified { .. }) => {}
        (_, other) => panic!("hybrid fixture: expected Verified, got {other:?}"),
    }

    let model = nodes
        .iter()
        .position(|node| node["semantic_form"] == "hybrid_model")
        .expect("the Tank model node");
    let mut swapped = base.clone();
    swapped["semantic_graph"]["nodes"][model]["body"]["members"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    match read_fixture_wire(ALL_FAMILIES, &swapped) {
        (_, Read::Refused(V2ReadRefusal::Envelope { refusal, .. })) => {
            assert_eq!(refusal.code, CheckedPackageRefusalCode::MalformedWire);
        }
        (_, other) => panic!("swapped model body: expected malformed_wire, got {other:?}"),
    }
}

/// FR-447-AC-13: QSL's I2 read-back of `positive-all-families.json` returns
/// `Tank`'s `CheckedHybridModel`: `x` and `y` classed `hybrid`, one mode
/// `Filling` whose rates are the bare variable `y` and the literal `1`, and
/// one transition resetting `x` to `y`. Read from the QSpec checkout.
#[trace("TC-1187", "FR-447-AC-13")]
#[test]
fn conformance_tank_reads_back_with_a_bare_variable_rate_and_reset() {
    use qsl_semantics::check::{
        CheckedHybridModel, CheckedMode, CheckedTransition, CompareOp, ContinuousVariable,
        HybridTerm, InitialEntry, ModeIndex, ResetConstraint, VariableClass, VariableIndex,
    };
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let base = read_fixture(&directory.join("positive-all-families.json"));
    let package = admitted_fixture(ALL_FAMILIES, &base);
    let read = super::read_hybrids(&package, 10_000).expect("the fixture reads back");
    let name = |text: &str| quire_exact::Identifier::new(text).expect("an identifier");
    let [tank] = read.models.as_slice() else {
        panic!("one model: {:?}", read.models);
    };
    assert_eq!(
        tank.model,
        CheckedHybridModel {
            name: name("Tank"),
            variables: vec![
                ContinuousVariable {
                    name: name("x"),
                    class: VariableClass::Hybrid
                },
                ContinuousVariable {
                    name: name("y"),
                    class: VariableClass::Hybrid
                },
            ],
            initial: vec![InitialEntry {
                mode: ModeIndex::new(0),
                condition: HybridTerm::Boolean(true),
            }],
            modes: vec![CheckedMode {
                name: name("Filling"),
                invariant: HybridTerm::Boolean(true),
                rates: vec![
                    HybridTerm::Variable(VariableIndex::new(1)),
                    HybridTerm::Constant(quire_exact::Rational::from_integer(1_i64.into())),
                ],
                transitions: vec![CheckedTransition {
                    target: ModeIndex::new(0),
                    guard: HybridTerm::Boolean(true),
                    resets: vec![ResetConstraint {
                        variable: VariableIndex::new(0),
                        relation: CompareOp::Eq,
                        value: HybridTerm::Variable(VariableIndex::new(1)),
                    }],
                }],
            }],
        }
    );
    assert!(read.claims.is_empty());
}

/// FR-447-AC-13 (QSpec FR-481 check 5): with `Tank`'s two variables both
/// classed `clock`, the model is re-keyed and QSL's I2 read refuses it
/// `invalid_package`/`invalid-value` at the first variable's class literal,
/// because `x`'s rate is the variable `y`, not the literal `1` a `clock`
/// takes. Read from the QSpec checkout.
#[trace("TC-1187", "FR-447-AC-13")]
#[test]
fn conformance_a_model_with_every_variable_clocked_is_refused_at_a_class_literal() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let mut candidate = read_fixture(&directory.join("positive-all-families.json"));
    let model = candidate["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node["semantic_form"] == "hybrid_model")
        .expect("the Tank model node");
    let variables = candidate["semantic_graph"]["nodes"][model]["body"]["members"][0]["value"]
        ["members"]
        .as_array_mut()
        .expect("variables");
    for variable in variables {
        variable["members"][1]["value"]["value"] = json!("clock");
    }
    match read_fixture_wire(ALL_FAMILIES, &candidate) {
        (_, Read::Refused(V2ReadRefusal::Envelope { refusal, .. })) => {
            assert_eq!(refusal.code, CheckedPackageRefusalCode::InvalidPackage);
            assert_eq!(
                refusal.cause,
                Some(CheckedPackageRefusalCause::InvalidValue)
            );
            let path = locus_pointer(&refusal).expect("a locus");
            assert!(
                path.ends_with("/body/members/0/value/members/0/members/1/value/value"),
                "{path}"
            );
        }
        (_, other) => panic!("every variable clocked: expected invalid_package, got {other:?}"),
    }
}

/// FR-447-AC-13 / QSpec FR-322-AC-120: the I2 reader retains every dense
/// timed node body exactly as IR admitted it, with each structural identity.
#[trace("TC-1187", "FR-447-AC-13")]
#[test]
fn conformance_timed_model_nodes_are_retained_by_i2() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let envelope = read_fixture(&directory.join("positive-timed-model-nodes.json"));
    let (name, outcome) = read_fixture_wire(TIMED_MODEL_NODES, &envelope);
    let Read::Verified {
        admitted,
        timed_models,
        ..
    } = outcome
    else {
        panic!("timed fixture must pass the real IR reader: {outcome:?}");
    };
    assert_eq!(admitted.name(), name);

    let wire_nodes = envelope["semantic_graph"]["nodes"].as_array().unwrap();
    let retained = &admitted.graph().nodes;
    let expected_timed = wire_nodes
        .iter()
        .filter(|node| node["node_tag"] == "timed_model")
        .collect::<Vec<_>>();
    let actual_timed = retained
        .iter()
        .filter(|node| node.node_tag.as_ref() == "timed_model")
        .collect::<Vec<_>>();
    assert_eq!(actual_timed.len(), 9);
    assert_eq!(
        actual_timed
            .iter()
            .map(|node| node.semantic_form.as_ref())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        8
    );
    for expected in expected_timed {
        let actual = actual_timed
            .iter()
            .find(|node| serde_json::to_value(&node.node_id).unwrap() == expected["node_id"])
            .expect("every wire timed node survives the read");
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            *expected,
            "complete timed node {:?} round trips",
            expected["node_id"]
        );
    }
    assert_eq!(
        admitted.source_map().len(),
        envelope["source_map"].as_array().unwrap().len()
    );

    let [timed] = timed_models.as_slice() else {
        panic!("one timed model is reconstructed");
    };
    assert_eq!(timed.source.kind, super::CheckedTimeSourceKind::Dense);
    assert_eq!(timed.source.period, None);
    assert_eq!(timed.clock_places.len(), 1);
    assert_eq!(timed.clock_places[0].path.len(), 2);
    assert!(matches!(
        &timed.clock_places[0].path[0],
        super::timed_model::ClockPathStep::Field(name) if name == "clock"
    ));
    assert!(matches!(
        &timed.clock_places[0].path[1],
        super::timed_model::ClockPathStep::Slot { collection, minimum: 0, maximum: None, .. }
            if collection == "map"
    ));
    assert_eq!(timed.resets.len(), 1);
    assert_eq!(timed.guards.len(), 1);
    assert_eq!(timed.invariants.len(), 1);
    assert_eq!(timed.urgencies.len(), 2);
    assert_eq!(timed.constraints.len(), 1);
    assert_eq!(timed.constraints[0].bound, "3/1");
    assert_eq!(
        timed.constraints[0].comparison,
        super::timed_model::ClockComparison::Le
    );
    assert_eq!(timed.constraints[0].bound_type, timed.source.unit);
}

#[trace("TC-1187", "FR-415-AC-17", "FR-447-AC-13")]
#[test]
fn conformance_tick_timed_model_has_operation_anchor_and_no_clock_places() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let envelope = read_fixture(&directory.join("positive-tick-timed-model-nodes.json"));
    let (name, outcome) = read_fixture_wire(TICK_TIMED_MODEL_NODES, &envelope);
    let Read::Verified {
        admitted,
        timed_models,
        ..
    } = outcome
    else {
        panic!("tick timed fixture must pass the real IR reader: {outcome:?}");
    };
    assert_eq!(admitted.name(), name);
    let [timed] = timed_models.as_slice() else {
        panic!("one tick timed model is reconstructed");
    };
    assert_eq!(timed.source.kind, super::CheckedTimeSourceKind::Tick);
    assert_eq!(timed.source.period.as_deref(), Some("1/2"));
    assert!(timed.clock_places.is_empty());
    assert!(timed.resets.is_empty());
    assert!(timed.guards.is_empty());
    assert!(timed.invariants.is_empty());
    assert!(timed.urgencies.is_empty());
    assert!(timed.constraints.is_empty());
    let source = &envelope["semantic_graph"]["nodes"][44];
    let operation_id = &source["body"]["members"][3]["value"]["target"];
    let operation = envelope["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["node_id"] == *operation_id)
        .unwrap();
    assert_eq!(operation["node_tag"], "state");
    assert_eq!(operation["semantic_form"], "operation_anchor");
    assert!(admitted
        .graph()
        .nodes
        .iter()
        .any(|node| serde_json::to_value(&node.node_id).unwrap() == *operation_id));
}

/// FR-415-AC-17 / FR-322-AC-122: the typed timed-model writer re-emits all
/// eight structural forms and the resulting package is admitted by I2.
#[trace("TC-367", "FR-415-AC-17", "FR-322-AC-122")]
#[test]
fn conformance_typed_timed_model_writer_emits_an_admitted_package() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let original = read_fixture(&directory.join("positive-timed-model-nodes.json"));
    let (_, read_outcome) = read_fixture_wire(TIMED_MODEL_NODES, &original);
    let Read::Verified {
        admitted,
        timed_models,
        ..
    } = read_outcome
    else {
        panic!("timed fixture must pass the real IR reader: {read_outcome:?}");
    };
    let [timed] = timed_models.as_slice() else {
        panic!("one typed timed model is reconstructed");
    };

    let emitted_nodes = timed
        .write_nodes()
        .expect("fixture has integer literal type");
    assert_eq!(emitted_nodes.len(), 9);
    assert_eq!(
        emitted_nodes
            .iter()
            .map(|node| node.semantic_form.as_ref())
            .collect::<Vec<_>>(),
        [
            "time_source",
            "clock_place",
            "clock_reset",
            "clock_guard",
            "time_invariant",
            "urgency",
            "urgency",
            "clock_constraint",
            "timed_model",
        ]
    );
    let round_tripped = super::timed_model::CheckedTimedModel::from_nodes(&emitted_nodes)
        .expect("writer output reconstructs as the same typed model");
    assert_eq!(round_tripped, timed_models);

    let emission = crate::emit::emit_timed_models(&admitted, &timed_models)
        .expect("QSL writes the checked timed model into a complete package");
    let emitted_package: Value =
        serde_json::from_slice(emission.package().bytes()).expect("writer emits a v2 package");
    assert_eq!(emission.package().name(), emitted_package["name"]);
    let (_, writer_outcome) = read_fixture_wire(TIMED_MODEL_NODES, &emitted_package);
    assert!(
        matches!(writer_outcome, Read::Verified { .. }),
        "QSL typed timed-model writer output is admitted as a complete package: {writer_outcome:?}"
    );
}

/// FR-322-AC-120: a dense timed model with no clock places reconstructs and
/// re-emits even though its timed nodes contain no integer literals.
#[trace("TC-367", "FR-322-AC-120", "FR-415-AC-17")]
#[test]
fn a_no_clock_dense_timed_model_needs_no_integer_literal_type() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let mut envelope = read_fixture(&directory.join("positive-timed-model-nodes.json"));
    let graph_before = WireGraph::of_package_value(&envelope).expect("the fixture graph");
    let nodes = envelope["semantic_graph"]["nodes"].as_array_mut().unwrap();
    let mut timed_indices = Vec::new();
    let mut source_index = None;
    let mut root_index = None;
    for (index, node) in nodes.iter_mut().enumerate() {
        if node["node_tag"] != "timed_model" {
            continue;
        }
        timed_indices.push(index);
        match node["semantic_form"].as_str().unwrap() {
            "time_source" => source_index = Some(index),
            "timed_model" => root_index = Some(index),
            _ => {}
        }
    }
    let source_index = source_index.unwrap();
    let root_index = root_index.unwrap();
    nodes[source_index]["body"]["members"][0]["value"]["value"] = json!("dense");
    nodes[source_index]["body"]["members"]
        .as_array_mut()
        .unwrap()
        .truncate(2);
    for binding in nodes[root_index]["body"]["members"].as_array_mut().unwrap() {
        if matches!(
            binding["name"].as_str(),
            Some("clock_places" | "resets" | "guards" | "invariants" | "urgencies" | "constraints")
        ) {
            binding["value"]["members"] = json!([]);
        }
    }
    let keep = [source_index, root_index]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let removed_ids = timed_indices
        .into_iter()
        .filter(|index| !keep.contains(index))
        .map(|index| nodes[index]["node_id"].clone())
        .collect::<Vec<_>>();

    fn targets(graph: &WireGraph, body: &Value) -> Vec<Value> {
        fn walk(value: &Value, output: &mut Vec<Value>) {
            match value {
                Value::Object(fields) => {
                    if fields.get("term") == Some(&json!("reference")) {
                        output.push(fields["target"].clone());
                    }
                    fields.values().for_each(|child| walk(child, output));
                }
                Value::Array(items) => items.iter().for_each(|child| walk(child, output)),
                _ => {}
            }
        }
        let mut output = Vec::new();
        walk(body, &mut output);
        output.sort_by_key(|value| graph.key_of_value(value).expect("reference identity"));
        output.dedup();
        output
    }
    // FR-322: drop every naming of the removed nodes while their indices
    // still name them (the root's emptied lists, its dependencies and their
    // source-map entries), then remove them and renumber the rest.
    for node in nodes.iter_mut() {
        if node["node_tag"] == "timed_model" {
            node["dependencies"] = json!(targets(&graph_before, &node["body"]));
        }
    }
    envelope["source_map"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| !removed_ids.contains(&entry["node_id"]));
    retain_nodes(&mut envelope, |node| {
        node["node_tag"] != "timed_model"
            || matches!(
                node["semantic_form"].as_str(),
                Some("time_source" | "timed_model")
            )
    });

    let (_, outcome) = read_fixture_wire(TIMED_MODEL_NODES, &envelope);
    let Read::Verified {
        admitted,
        timed_models,
        ..
    } = outcome
    else {
        panic!("the dense no-clock timed model is admitted: {outcome:?}");
    };
    let [timed] = timed_models.as_slice() else {
        panic!("one no-clock model reconstructs");
    };
    assert_eq!(timed.source.kind, super::CheckedTimeSourceKind::Dense);
    assert!(timed.clock_places.is_empty());
    assert!(timed.literal_types.integer.is_none());

    let emission = crate::emit::emit_timed_models(&admitted, &timed_models)
        .expect("a no-clock model writes without an integer literal type");
    let emitted: Value = serde_json::from_slice(emission.package().bytes()).unwrap();
    let (_, round_trip) = read_fixture_wire(TIMED_MODEL_NODES, &emitted);
    let Read::Verified {
        timed_models: reread,
        ..
    } = round_trip
    else {
        panic!("the emitted dense no-clock model is read back: {round_trip:?}");
    };
    assert_eq!(reread.len(), 1);
    assert_eq!(reread[0].source.kind, super::CheckedTimeSourceKind::Dense);
    assert!(reread[0].clock_places.is_empty());
}

/// FR-415-AC-17: changes to each typed time-source, clock-place and
/// constraint input change the relevant node key and survive I2 read-back.
/// Part of FR-230-AC-30 (TC-685 step 26) at the writer: changing only the
/// source unit, a tick period, a clock place's path, a constraint's bound or
/// comparator, or a reset's value changes the emitted typed nodes and
/// survives read-back. Not covered: the S3-to-S4 emission of a checked
/// model's clocks, guards, invariants and urgency, which S4 does not write
/// yet (`TimedModelNotEmitted`), and occurrence reordering.
#[trace("TC-367", "FR-415-AC-17", "FR-322-AC-122")]
#[trace("TC-685")]
#[test]
fn changed_timed_inputs_rekey_and_read_back_the_mutated_model() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let mut base_wire = read_fixture(&directory.join("positive-timed-model-nodes.json"));

    // Add a second unit with the same time dimension so the unit mutation is
    // a valid selected-time-unit change, not a deliberately ill-typed edge.
    let alternate_unit = {
        let nodes = base_wire["semantic_graph"]["nodes"].as_array_mut().unwrap();
        let unit_index = nodes
            .iter()
            .position(|node| node["semantic_form"] == "unit")
            .expect("time unit");
        let original = nodes[unit_index].clone();
        let mut alternate = original.clone();
        let original_id = original["node_id"].clone();
        alternate["declaration"]["qualified_name"][1] = json!("ms");
        alternate["node_id"]["path"][1] = json!("ms");
        alternate["body"]["target_unit_node_id"] = original_id.clone();
        alternate["body"]["scale"] = json!({
            "numerator": "1",
            "denominator": "1000"
        });
        alternate["dependencies"] =
            json!([original["dependencies"][0].clone(), original_id.clone()]);
        alternate["dependencies"]
            .as_array_mut()
            .unwrap()
            .sort_by_key(|value| node_key_from_json(value).expect("dependency identity"));
        alternate["occurrences"][0]["ordinal"] = json!(1);
        let alternate_id = alternate["node_id"].clone();
        nodes.push(alternate);
        let map = base_wire["source_map"].as_array_mut().unwrap();
        let mut alternate_map = map
            .iter()
            .find(|entry| entry["node_id"] == original_id)
            .expect("unit source-map entry")
            .clone();
        alternate_map["node_id"] = alternate_id.clone();
        alternate_map["ordinal"] = json!(1);
        map.push(alternate_map);
        alternate_id
    };
    let (_, base_outcome) = read_fixture_wire(TIMED_MODEL_NODES, &base_wire);
    let Read::Verified {
        admitted,
        timed_models,
        ..
    } = base_outcome
    else {
        panic!("fixture with another same-dimension time unit admits: {base_outcome:?}");
    };
    let [original] = timed_models.as_slice() else {
        panic!("one timed model reconstructs");
    };

    let mut cases = Vec::new();
    let mut changed_unit = original.clone();
    changed_unit.source.unit = serde_json::from_value(alternate_unit.clone()).unwrap();
    for reset in &mut changed_unit.resets {
        reset.value_type = changed_unit.source.unit.clone();
    }
    for constraint in &mut changed_unit.constraints {
        constraint.bound_type = changed_unit.source.unit.clone();
    }
    cases.push(("unit", changed_unit));

    let mut changed_path = original.clone();
    let audit_key = admitted
        .graph()
        .nodes
        .iter()
        .find(|node| {
            node.node_tag.as_ref() == "value"
                && node.semantic_form.as_ref() == "literal"
                && serde_json::to_string(&node.body)
                    .expect("the checked body serializes")
                    .contains("audit")
        })
        .expect("fixture text key value")
        .node_id
        .clone();
    let super::timed_model::ClockPathStep::Slot { selector, .. } =
        &mut changed_path.clock_places[0].path[1]
    else {
        panic!("fixture clock path has a map key selector");
    };
    *selector = audit_key;
    cases.push(("path", changed_path));

    let mut changed_bound = original.clone();
    changed_bound.constraints[0].bound = "7/2".to_owned();
    cases.push(("bound", changed_bound));

    let mut changed_comparison = original.clone();
    changed_comparison.constraints[0].comparison = super::timed_model::ClockComparison::Lt;
    cases.push(("comparison", changed_comparison));

    let mut changed_reset = original.clone();
    changed_reset.resets[0].value = "5/2".to_owned();
    cases.push(("reset", changed_reset));

    let id_for = |package: &quire_contract_model::CheckedPackageV2, form: &str| {
        package
            .graph()
            .nodes
            .iter()
            .find(|node| {
                node.node_tag.as_ref() == "timed_model" && node.semantic_form.as_ref() == form
            })
            .expect("timed node form")
            .node_id
            .clone()
    };
    for (case, mut expected) in cases {
        let emission = crate::emit::emit_timed_models(&admitted, &[expected.clone()])
            .unwrap_or_else(|error| panic!("{case} mutation emits: {error}"));
        let wire: Value = serde_json::from_slice(emission.package().bytes()).unwrap();
        assert_eq!(wire["name"], base_wire["name"], "{case} keeps package name");
        let (_, outcome) = read_fixture_wire(TIMED_MODEL_NODES, &wire);
        if case == "path" {
            // FR-322-AC-148: a slot selector is the symbolic type node of its
            // domain. The fixture's map admits only the one `Text` type node,
            // so a literal-key path has no valid read-back: it rekeys the
            // place and I2 refuses it as a malformed clock place.
            assert!(
                matches!(
                    &outcome,
                    Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                        if refusal.cause == Some(CheckedPackageRefusalCause::MalformedClockPlace)
                ),
                "a literal slot selector is refused as a malformed clock place: {outcome:?}"
            );
            continue;
        }
        let Read::Verified {
            timed_models: reread,
            ..
        } = outcome
        else {
            panic!("{case} mutation is read back by I2: {outcome:?}");
        };
        let [actual] = reread.as_slice() else {
            panic!("{case} mutation leaves one timed model");
        };
        expected.metadata = actual.metadata.clone();
        expected.source.metadata = actual.source.metadata.clone();
        for (expected, actual) in expected.clock_places.iter_mut().zip(&actual.clock_places) {
            expected.metadata = actual.metadata.clone();
        }
        // The referenced timed nodes are rekeyed; each reference is
        // taken from the read-back model.
        for (expected, actual) in expected.resets.iter_mut().zip(&actual.resets) {
            expected.metadata = actual.metadata.clone();
            expected.target = actual.target.clone();
        }
        for (expected, actual) in expected.guards.iter_mut().zip(&actual.guards) {
            expected.metadata = actual.metadata.clone();
            expected.parts = actual.parts.clone();
        }
        for (expected, actual) in expected.invariants.iter_mut().zip(&actual.invariants) {
            expected.metadata = actual.metadata.clone();
            expected.bounds = actual.bounds.clone();
        }
        for (expected, actual) in expected.urgencies.iter_mut().zip(&actual.urgencies) {
            match (expected, actual) {
                (
                    super::timed_model::CheckedUrgency::When { metadata, .. },
                    super::timed_model::CheckedUrgency::When {
                        metadata: actual, ..
                    },
                )
                | (
                    super::timed_model::CheckedUrgency::Operation { metadata, .. },
                    super::timed_model::CheckedUrgency::Operation {
                        metadata: actual, ..
                    },
                ) => *metadata = actual.clone(),
                _ => panic!("urgency form is unchanged"),
            }
        }
        for (expected, actual) in expected.constraints.iter_mut().zip(&actual.constraints) {
            expected.metadata = actual.metadata.clone();
            expected.target = actual.target.clone();
        }
        assert_eq!(actual, &expected, "{case} typed content reads back exactly");
    }

    let tick_wire = read_fixture(&directory.join("positive-tick-timed-model-nodes.json"));
    let (_, tick_outcome) = read_fixture_wire(TICK_TIMED_MODEL_NODES, &tick_wire);
    let Read::Verified {
        admitted: tick_admitted,
        timed_models: tick_models,
        ..
    } = tick_outcome
    else {
        panic!("tick fixture admits before its period mutation: {tick_outcome:?}");
    };
    let mut tick_models = tick_models;
    assert_eq!(tick_models.len(), 1, "tick fixture reconstructs one model");
    let mut changed_tick = tick_models.remove(0);
    let original_tick_source = id_for(&tick_admitted, "time_source");
    changed_tick.source.period = Some("3/2".to_owned());
    let tick_emission =
        crate::emit::emit_timed_models(&tick_admitted, std::slice::from_ref(&changed_tick))
            .expect("changed tick period emits");
    let changed_tick_wire: Value = serde_json::from_slice(tick_emission.package().bytes()).unwrap();
    assert_eq!(changed_tick_wire["name"], tick_wire["name"]);
    let (_, changed_tick_outcome) = read_fixture_wire(
        "fixtures/positive-tick-timed-model-nodes.json",
        &changed_tick_wire,
    );
    let Read::Verified {
        admitted: reread_tick,
        timed_models: reread_tick_models,
        ..
    } = changed_tick_outcome
    else {
        panic!("changed tick period reads back: {changed_tick_outcome:?}");
    };
    assert_ne!(id_for(&reread_tick, "time_source"), original_tick_source);
    assert_eq!(reread_tick_models[0].source.period.as_deref(), Some("3/2"));
}

/// FR-322-AC-120 and AC-124: positive hybrid and timed graph references all
/// close over their own documents, and a unit node's nominal dimension is
/// exactly the timed root's explicit `time_dimension` authority.
#[trace("TC-1187", "FR-447-AC-13")]
#[test]
fn conformance_hybrid_and_timed_positive_references_close_and_units_match_dimensions() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    for name in [
        "positive-all-families.json",
        "positive-timed-model-nodes.json",
    ] {
        let envelope = read_fixture(&directory.join(name));
        let nodes = envelope["semantic_graph"]["nodes"].as_array().unwrap();
        let graph = WireGraph::of_package_value(&envelope).expect("fixture node ids");
        let ids = graph
            .keys()
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), nodes.len(), "{name}: node ids are unique");
        fn visit(
            value: &Value,
            graph: &WireGraph,
            ids: &std::collections::BTreeSet<NodeKey>,
            name: &str,
            package: &str,
        ) {
            match value {
                Value::Object(object) => {
                    if object.contains_key("package")
                        && object.contains_key("path")
                        && object.contains_key("ordinal")
                    {
                        let key = graph.key_of_value(value).expect("structural reference");
                        if object.get("package").and_then(Value::as_str) == Some(package) {
                            assert!(ids.contains(&key), "{name}: dangling NodeRef {key:?}");
                        }
                    }
                    for child in object.values() {
                        visit(child, graph, ids, name, package);
                    }
                }
                Value::Array(values) => {
                    for child in values {
                        visit(child, graph, ids, name, package);
                    }
                }
                _ => {}
            }
        }
        visit(
            &envelope["semantic_graph"],
            &graph,
            &ids,
            name,
            envelope["name"].as_str().unwrap(),
        );
        let (_, outcome) = read_fixture_wire(&format!("fixtures/{name}"), &envelope);
        let Read::Verified { admitted, .. } = outcome else {
            panic!("{name}: complete positive read expected, got {outcome:?}");
        };
        for node in &admitted.graph().nodes {
            if node.node_tag.as_ref() == "scalar_type" && node.semantic_form.as_ref() == "unit" {
                let value = serde_json::to_value(node).expect("admitted node serializes");
                assert_eq!(
                    value["semantic_type"], value["body"]["dimension_node_id"],
                    "{name}: unit is dimension-typed"
                );
            }
        }
    }
}

/// The published TM schema and semantic refusal vectors reach the real I2
/// reader. The body order is supplied by IR's FR-322 reader: hybrid,
/// timed-model, then protocol; QSpec pins that order in TC-233.
/// FR-230-AC-31 and FR-230-AC-34 (TC-685 steps 26 and 27): I2 refuses a
/// missing root (`missing-timed-model`), a dangling or wrong-kind edge
/// (`timed-node-reference`), a second place of one path whose forged row
/// refuses `malformed-clock-place`, a repeated place reference (`invalid_semantic_graph`, no cause, QSpec
/// FR-322-AC-260's order refusal, TM-30), a
/// noncanonical path (`malformed-clock-place`) and a non-time unit or
/// malformed quantity (`invalid-time-quantity`), each at the first failing
/// timed node's locus; a `resolved_path` row missing, extra or mismatched in
/// its declaring type, member identity, member type or collection metadata
/// refuses `malformed-clock-place`; FR-230-AC-35 to AC-37's vectors refuse as
/// those criteria state.
#[trace("TC-1187", "FR-447-AC-13")]
#[trace("TC-685", "FR-230-AC-31", "FR-230-AC-34")]
#[trace("QSpec-FR-322-AC-260")]
#[test]
fn conformance_timed_model_vectors_keep_their_published_refusal_loci() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let base = read_fixture(&directory.join("positive-timed-model-nodes.json"));
    let tick_base = read_fixture(&directory.join("positive-tick-timed-model-nodes.json"));
    let vectors = read_fixture(&directory.join("../timed-model-node-vectors.json"));
    let indexes = &vectors["node_indexes"];
    for row in vectors["schema_refusals"].as_array().unwrap() {
        let mut candidate = base.clone();
        let node_index = indexes[row["node"].as_str().unwrap()].as_u64().unwrap() as usize;
        let body = &mut candidate["semantic_graph"]["nodes"][node_index]["body"]["members"];
        match row["mutate"].as_str().unwrap() {
            "remove-unit" => {
                body.as_array_mut().unwrap().remove(1);
            }
            "extra-binding" => body.as_array_mut().unwrap().push(
                json!({"term":"binding","name":"extra","value":{"term":"aggregate","members":[]}}),
            ),
            "invalid-path-step-kind" => {
                body[1]["value"]["members"][0]["members"][0]["value"]["value"] = json!("slot")
            }
            "swap-model-source" => body.as_array_mut().unwrap().swap(0, 1),
            "empty-path" => body[1]["value"]["members"] = json!([]),
            other => panic!("{}: unknown schema vector {other}", row["id"]),
        }
        let expected_locus = row["locus"].as_str().unwrap().replace(
            &format!("/nodes/{}", row["node"].as_str().unwrap()),
            &format!("/nodes/{node_index}"),
        );
        let (_, outcome) = read_fixture_wire(TIMED_MODEL_NODES, &candidate);
        assert_timed_refusal(
            &outcome,
            "malformed_wire",
            None,
            &expected_locus,
            &row["id"],
        );
    }

    let semantic_rows = vectors["semantic_refusals"].as_array().unwrap();
    for row in semantic_rows
        .iter()
        .filter(|row| row["id"] != "TM-23")
        .chain(semantic_rows.iter().filter(|row| row["id"] == "TM-23"))
    {
        let is_tick = row["fixture"] == "tick";
        let mut candidate = if is_tick {
            tick_base.clone()
        } else {
            base.clone()
        };
        match row["mutation"].as_str().unwrap() {
            "remove-timed-root" => {
                candidate["semantic_graph"]["nodes"]
                    .as_array_mut()
                    .unwrap()
                    .remove(if is_tick { 45 } else { 52 });
            }
            "dangling-tick-operation" => {
                let source = &mut candidate["semantic_graph"]["nodes"][44];
                let target = &mut source["body"]["members"][3]["value"]["target"];
                let old = target.clone();
                target["path"] = json!(["missing-operation"]);
                let replacement = target.clone();
                replace_dependency(&mut candidate, 44, &old, &replacement);
            }
            "function-as-tick-operation" => {
                let target = candidate["semantic_graph"]["nodes"][5]["node_id"].clone();
                let source = &mut candidate["semantic_graph"]["nodes"][44];
                let old = source["body"]["members"][3]["value"]["target"].clone();
                source["body"]["members"][3]["value"]["target"] = target.clone();
                replace_dependency(&mut candidate, 44, &old, &target);
            }
            "second-place-forged-row" => {
                // QSpec TM-12 (STD-559): a distinct second place of the same
                // path, listed after the first and located after every
                // published place, so the root is in declared order. Only its
                // derived `resolved_path` can differ, so its last row names a
                // member the selected declaration lacks, and the row join,
                // which precedes the root's aggregates, refuses it there. A
                // repeated reference would repeat its order key and refuse as
                // an order defect first (TM-30). The second place receives
                // a distinct producer-selected ordinal on the same path.
                let mut second = candidate["semantic_graph"]["nodes"][45].clone();
                let original = second["node_id"].clone();
                let rows = second["body"]["members"][2]["value"]["members"]
                    .as_array_mut()
                    .unwrap();
                let identity = &mut rows.last_mut().unwrap()["members"][2]["value"]["value"];
                *identity = json!(format!("{}#again", identity.as_str().unwrap()));
                let mut placeholder = original.clone();
                placeholder["ordinal"] = json!(1);
                second["node_id"] = placeholder.clone();
                second["semantic_type"] = placeholder.clone();
                let mut entry = candidate["source_map"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|entry| entry["node_id"] == original)
                    .unwrap()
                    .clone();
                entry["node_id"] = placeholder.clone();
                entry["regions"][0]["start"] = json!(9000);
                entry["regions"][0]["end"] = json!(9001);
                candidate["source_map"].as_array_mut().unwrap().push(entry);
                let at = candidate["semantic_graph"]["nodes"]
                    .as_array()
                    .unwrap()
                    .len()
                    - 1;
                insert_node(&mut candidate, at, second);
                let root = &mut candidate["semantic_graph"]["nodes"][52];
                root["body"]["members"][3]["value"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"term": "reference", "target": placeholder.clone()}));
                // The root's dependencies stay in node-id order, so the
                // forged row, not an order defect, is the first refusal.
                let dependencies = root["dependencies"].as_array_mut().unwrap();
                dependencies.push(placeholder);
                sort_dependencies(&mut candidate, 52);
            }
            "repeated-clock-place-reference" => {
                // QSpec FR-322-AC-260, TM-30: the first place's reference
                // listed again repeats its predecessor's order key, the
                // order refusal with no cause at the repeated member.
                let places = candidate["semantic_graph"]["nodes"][52]["body"]["members"][3]
                    ["value"]["members"]
                    .as_array_mut()
                    .unwrap();
                let first = places[0].clone();
                places.push(first);
            }
            "noncanonical-place-path" => {
                candidate["semantic_graph"]["nodes"][45]["body"]["members"][1]["value"]["members"]
                    [0]["members"][1]["value"]["value"] = json!("not-clock")
            }
            "zero-tick-period" => {
                candidate["semantic_graph"]["nodes"][44]["body"]["members"][4]["value"]["value"] =
                    json!("0")
            }
            "missing-clock-resolution-row" => {
                let steps = candidate["semantic_graph"]["nodes"][45]["body"]["members"][1]["value"]
                    ["members"]
                    .as_array_mut()
                    .unwrap();
                let duplicate = steps[0].clone();
                steps.push(duplicate);
            }
            "mismatched-clock-path-step" => {
                candidate["semantic_graph"]["nodes"][45]["body"]["members"][2]["value"]["members"]
                    [0]["members"][0]["value"]["value"] = json!(1)
            }
            "wrong-clock-collection-metadata" => {
                candidate["semantic_graph"]["nodes"][45]["body"]["members"][2]["value"]["members"]
                    [0]["members"][4]["value"]["value"] = json!("sequence")
            }
            "undeclared-clock-member" => {
                candidate["semantic_graph"]["nodes"][45]["body"]["members"][2]["value"]["members"]
                    [0]["members"][2]["value"]["value"] = json!("ix://acme/orders/Order/unknown")
            }
            "wrong-terminal-clock-type" => {
                candidate["semantic_graph"]["nodes"][45]["body"]["members"][2]["value"]["members"]
                    [0]["members"][3]["value"]["value"] = json!("ix://quire/native/Integer")
            }
            "coherent-forged-selected-chain" => {
                let place = &mut candidate["semantic_graph"]["nodes"][45]["body"]["members"];
                place[1]["value"]["members"][1]["members"][1]["value"]["value"] = json!("sequence");
                let rows = &mut place[2]["value"]["members"];
                rows[0]["members"][3]["value"]["value"] = json!("Sequence<Evil>");
                rows[0]["members"][4]["value"]["value"] = json!("sequence");
                rows[1]["members"][1]["value"]["value"] = json!("Evil");
                rows[1]["members"][2]["value"]["value"] = json!("Sequence<Evil>[slot]");
            }
            "tick-model-context-mismatch" => {
                let target = candidate["semantic_graph"]["nodes"][14]["node_id"].clone();
                let source = &mut candidate["semantic_graph"]["nodes"][44];
                let old = source["body"]["members"][2]["value"]["target"].clone();
                source["body"]["members"][2]["value"]["target"] = target.clone();
                replace_dependency(&mut candidate, 44, &old, &target);
            }
            "unresolved-tick-operation" => {
                candidate["semantic_graph"]["nodes"][46]["body"]["members"][1]["value"]["value"] =
                    json!("missing")
            }
            "tick-source-with-clock-place" => {
                let mut place = base["semantic_graph"]["nodes"][45].clone();
                rebase_index_segments(&mut place, &base, &candidate);
                candidate["semantic_graph"]["nodes"]
                    .as_array_mut()
                    .unwrap()
                    .push(place.clone());
                let root = &mut candidate["semantic_graph"]["nodes"][45];
                root["body"]["members"][3]["value"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"term":"reference", "target":place["node_id"].clone()}));
                root["dependencies"]
                    .as_array_mut()
                    .unwrap()
                    .push(place["node_id"].clone());
                sort_dependencies(&mut candidate, 45);
            }
            "source-unit-length" | "source-unit-derived-non-time" | "source-unit-derived-time" => {
                let mutation = row["mutation"].as_str().unwrap();
                let dimension_name = match mutation {
                    "source-unit-length" => "Length",
                    "source-unit-derived-non-time" => "Area",
                    _ => "Duration",
                };
                let time_id = candidate["semantic_graph"]["nodes"][42]["node_id"].clone();
                let mut dimension = candidate["semantic_graph"]["nodes"][42].clone();
                dimension["declaration"]["qualified_name"] = json!(["Example", dimension_name]);
                dimension["node_id"]["path"] = json!(["Example", dimension_name]);
                if dimension_name == "Area" || dimension_name == "Duration" {
                    let base_id = if dimension_name == "Area" {
                        let mut length = dimension.clone();
                        length["declaration"]["qualified_name"] = json!(["Example", "Length"]);
                        length["node_id"]["path"] = json!(["Example", "Length"]);
                        let length_id = length["node_id"].clone();
                        length["semantic_type"] = length_id.clone();
                        candidate["semantic_graph"]["nodes"]
                            .as_array_mut()
                            .unwrap()
                            .push(length);
                        length_id
                    } else {
                        time_id.clone()
                    };
                    dimension["body"]["terms"] = json!([{
                        "dimension_node_id": base_id.clone(),
                        "exponent": if dimension_name == "Area" { "2" } else { "1" }
                    }]);
                    dimension["dependencies"] = json!([base_id]);
                }
                let dimension_id = dimension["node_id"].clone();
                dimension["semantic_type"] = dimension_id.clone();
                candidate["semantic_graph"]["nodes"]
                    .as_array_mut()
                    .unwrap()
                    .push(dimension);
                let mut unit = candidate["semantic_graph"]["nodes"][43].clone();
                unit["declaration"]["qualified_name"] = json!(["Example", "u"]);
                unit["node_id"]["path"] = json!(["Example", "u"]);
                unit["body"]["dimension_node_id"] = dimension_id.clone();
                let unit_id = unit["node_id"].clone();
                unit["semantic_type"] = dimension_id.clone();
                unit["dependencies"] = json!([dimension_id]);
                candidate["semantic_graph"]["nodes"]
                    .as_array_mut()
                    .unwrap()
                    .push(unit);
                let source = &mut candidate["semantic_graph"]["nodes"][44];
                let old = source["body"]["members"][1]["value"]["target"].clone();
                source["body"]["members"][1]["value"]["target"] = unit_id.clone();
                replace_dependency(&mut candidate, 44, &old, &unit_id);
            }
            other => panic!("{}: unknown semantic vector {other}", row["id"]),
        }
        let expected_cause = row["expected_cause"].as_str().map(|cause| {
            serde_json::from_value::<CheckedPackageRefusalCause>(json!(cause))
                .unwrap_or_else(|_| panic!("{}: unrecognized cause {cause}", row["id"]))
        });
        assert_timed_refusal(
            &read_fixture_wire(
                if is_tick {
                    "fixtures/positive-tick-timed-model-nodes.json"
                } else {
                    "fixtures/positive-timed-model-nodes.json"
                },
                &candidate,
            )
            .1,
            row["expected_code"].as_str().unwrap(),
            expected_cause,
            row["locus"].as_str().unwrap(),
            &row["id"],
        );
    }
}

fn assert_timed_refusal(
    outcome: &Read,
    expected_code: &str,
    expected_cause: Option<CheckedPackageRefusalCause>,
    expected_pointer: &str,
    id: &Value,
) {
    let Read::Refused(V2ReadRefusal::Envelope { refusal, locus }) = outcome else {
        panic!("{id}: expected IR refusal at {expected_pointer}, got {outcome:?}");
    };
    let code = serde_json::to_value(refusal.code).unwrap();
    assert_eq!(code, expected_code, "{id}: refusal code");
    assert_eq!(refusal.cause, expected_cause, "{id}: refusal cause");
    assert!(
        matches!(
            locus.as_deref(),
            Some(Locus::Artifact { pointer, .. }) if pointer.as_str() == expected_pointer
        ),
        "{id}: refusal locus, got {locus:?}"
    );
}

/// QSpec FR-322 "Clause nodes" (TC-367 CK-36): each clause vector of
/// `timed-clause-node-vectors.json`, its RFC 6902 patch applied to the
/// published clause package, reaches QSL's I2 read with its published
/// outcome: an admitted case is verified, a schema case refuses
/// `malformed_wire` within its node, and each other case refuses with its
/// code and cause at its locus. `CL-25` (STD-558) is a reset naming the
/// anchor of another model whose frame is that model's, refused by the
/// timed-model join; `CL-54` is the same reset whose frame stays on the
/// first model, refused `invalid_model_binding` in the state step first.
/// `CL-57` and `CL-59` admit typed guard/invariant binders; `CL-58`, `CL-60`
/// and `CL-62` refuse typed binder violations.
#[trace("TC-685", "FR-230-AC-34")]
#[trace("QSpec-FR-322-AC-261", "QSpec-FR-322-AC-262", "QSpec-FR-322-AC-263")]
#[trace("QSpec-FR-322-AC-295", "QSpec-TC-367")]
#[test]
fn conformance_timed_clause_vectors_keep_their_published_outcomes() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let proposals = directory.parent().expect("the proposal directory");
    let vectors = read_fixture(&proposals.join("timed-clause-node-vectors.json"));
    let mut replayed = 0_usize;
    for case in vectors["cases"].as_array().expect("CL cases") {
        let id = &case["id"];
        let fixture = case["fixture"]
            .as_str()
            .unwrap_or("fixtures/positive-timed-clause-nodes.json");
        let mut package = read_fixture(&proposals.join(fixture));
        if let Some(patch) = case.get("patch") {
            apply_json_patch(&mut package, patch);
        }
        let (_, outcome) = read_fixture_wire(fixture, &package);
        let locus = case["locus"].as_str().unwrap_or_default();
        if case["schema"] == "invalid" && case.get("expected").is_none() {
            let Read::Refused(V2ReadRefusal::Envelope { refusal, locus }) = &outcome else {
                panic!("{id}: expected a typed refusal, got {outcome:?}");
            };
            assert_eq!(
                refusal.code,
                CheckedPackageRefusalCode::InvalidSemanticGraph,
                "{id}"
            );
            assert!(
                matches!(
                    locus.as_deref(),
                    Some(Locus::Artifact { pointer, .. })
                        if pointer.as_str().starts_with("/semantic_graph/nodes/70")
                ),
                "{id}: schema-invalid execution point locus, got {locus:?}"
            );
            replayed += 1;
            continue;
        }
        match case["expected"].as_str().expect("an outcome") {
            "admitted" => assert!(
                matches!(outcome, Read::Verified { .. }),
                "{id}: expected admitted, got {outcome:?}"
            ),
            "refused-by-schema" => {
                let Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) = &outcome else {
                    panic!("{id}: expected malformed_wire, got {outcome:?}");
                };
                assert_eq!(
                    refusal.code,
                    CheckedPackageRefusalCode::MalformedWire,
                    "{id}"
                );
                let pointer = locus_pointer(refusal).unwrap_or_default();
                assert!(pointer.starts_with(locus), "{id}: {pointer} within {locus}");
            }
            "refused" => {
                let cause = case["expected_cause"].as_str().map(|cause| {
                    serde_json::from_value::<CheckedPackageRefusalCause>(json!(cause))
                        .unwrap_or_else(|_| panic!("{id}: unrecognized cause {cause}"))
                });
                assert_timed_refusal(
                    &outcome,
                    case["expected_code"].as_str().expect("a code"),
                    cause,
                    locus,
                    id,
                );
            }
            other => panic!("{id}: unknown outcome {other}"),
        }
        replayed += 1;
    }
    assert!(replayed > 0, "the clause vectors replay");
}

/// The token of a JSON pointer step.
fn unescape(step: &str) -> String {
    step.replace("~1", "/").replace("~0", "~")
}

/// Apply the RFC 6902 `add`, `remove` and `replace` operations of
/// `patch` to `document`.
pub(crate) fn apply_json_patch(document: &mut Value, patch: &Value) {
    for operation in patch.as_array().expect("a patch") {
        let path = operation["path"].as_str().expect("a path");
        let (parent, last) = path.rsplit_once('/').expect("a nonroot path");
        let last = unescape(last);
        let container = document
            .pointer_mut(parent)
            .unwrap_or_else(|| panic!("{parent} resolves"));
        let value = operation["value"].clone();
        match (operation["op"].as_str().expect("an op"), container) {
            ("add", Value::Array(items)) if last == "-" => items.push(value),
            ("add", Value::Array(items)) => items.insert(last.parse().unwrap(), value),
            ("add" | "replace", Value::Object(fields)) => {
                fields.insert(last, value);
            }
            ("replace", Value::Array(items)) => items[last.parse::<usize>().unwrap()] = value,
            ("remove", Value::Array(items)) => {
                items.remove(last.parse().unwrap());
            }
            ("remove", Value::Object(fields)) => {
                fields.remove(&last);
            }
            (op, _) => panic!("unsupported patch operation {op} at {path}"),
        }
    }
}

/// Keep the graph nodes `keep` admits, renumbering every index segment the
/// envelope holds to its node's new graph index (FR-322 "Operand index
/// segments"). A retained id that names a removed node is a fixture defect.
fn retain_nodes(envelope: &mut Value, keep: impl Fn(&Value) -> bool) {
    let nodes = envelope["semantic_graph"]["nodes"].as_array_mut().unwrap();
    let mut renumbered = Vec::with_capacity(nodes.len());
    let mut next = 0_u64;
    for node in nodes.iter() {
        renumbered.push(keep(node).then(|| {
            next += 1;
            next - 1
        }));
    }
    let mut position = 0;
    nodes.retain(|_| {
        position += 1;
        renumbered[position - 1].is_some()
    });
    let mut pending = vec![&mut *envelope];
    while let Some(value) = pending.pop() {
        match value {
            Value::Object(object) => {
                let is_id = object.len() == 3
                    && object.contains_key("package")
                    && object.contains_key("ordinal");
                for (name, member) in object.iter_mut() {
                    if is_id && name == "path" {
                        for segment in member.as_array_mut().into_iter().flatten() {
                            if let Some(index) = segment.get("node").and_then(Value::as_u64) {
                                let moved = renumbered[usize::try_from(index).unwrap()]
                                    .expect("a retained id names a retained node");
                                *segment = json!({"node": moved});
                            }
                        }
                    } else {
                        pending.push(member);
                    }
                }
            }
            Value::Array(items) => pending.extend(items.iter_mut()),
            _ => {}
        }
    }
}

/// Replace `old` by `replacement` in node `position`'s dependencies, kept in
/// ascending structural-key order.
fn replace_dependency(envelope: &mut Value, position: usize, old: &Value, replacement: &Value) {
    let entry = envelope["semantic_graph"]["nodes"][position]["dependencies"]
        .as_array_mut()
        .expect("node dependencies")
        .iter_mut()
        .find(|entry| *entry == old)
        .expect("body reference has a matching dependency");
    *entry = replacement.clone();
    sort_dependencies(envelope, position);
}

/// Insert `node` into the envelope's graph at index `at`, moving every operand
/// index segment (FR-322) that names a node at or past `at` up by one so each
/// still names the node it named before the insert.
fn insert_node(envelope: &mut Value, at: usize, mut node: Value) {
    shift_index_segments(envelope, at);
    shift_index_segments(&mut node, at);
    envelope["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("graph nodes")
        .insert(at, node);
}

/// Rewrite every operand index segment (FR-322) in `value`, which names a node
/// of `from`'s graph, to the index of the node with the same key in `to`'s.
fn rebase_index_segments(value: &mut Value, from: &Value, to: &Value) {
    let from = WireGraph::of_package_value(from).expect("the source graph");
    let to = WireGraph::of_package_value(to).expect("the target graph");
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            Value::Object(members) => {
                if let Some(Value::Array(path)) = members.get_mut("path") {
                    for segment in path.iter_mut() {
                        if let Some(index) = segment.get("node").and_then(Value::as_u64) {
                            let key = &from.keys()[index as usize];
                            let rebased = to.index_of(key).expect("the operand is in both graphs");
                            segment["node"] = json!(rebased);
                        }
                    }
                }
                pending.extend(members.values_mut());
            }
            Value::Array(items) => pending.extend(items.iter_mut()),
            _ => {}
        }
    }
}

/// Move every operand index segment in `value` that names a node at or past
/// `at` up by one.
fn shift_index_segments(value: &mut Value, at: usize) {
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            Value::Object(members) => {
                if let Some(Value::Array(path)) = members.get_mut("path") {
                    for segment in path.iter_mut() {
                        if let Some(index) = segment.get("node").and_then(Value::as_u64) {
                            if index as usize >= at {
                                segment["node"] = json!(index + 1);
                            }
                        }
                    }
                }
                pending.extend(members.values_mut());
            }
            Value::Array(items) => pending.extend(items.iter_mut()),
            _ => {}
        }
    }
}

/// Sort node `position`'s dependencies by structural key, each id resolved
/// against the envelope's graph.
fn sort_dependencies(envelope: &mut Value, position: usize) {
    let graph = WireGraph::of_package_value(envelope).expect("the graph");
    envelope["semantic_graph"]["nodes"][position]["dependencies"]
        .as_array_mut()
        .expect("node dependencies")
        .sort_by_key(|value| graph.key_of_value(value).expect("dependency identity"));
}

/// FR-250's five temporal profiles are the only identities a temporal
/// application's `temporal_profile` law may name. Changing the law in the
/// checked package is refused as `unknown_profile` at that definition.
fn unknown_temporal_profile_is_unknown_profile(base: &Value) {
    let clause = base["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .position(|node| {
            node["node_tag"] == "temporal" && node["semantic_form"] == "temporal_clause"
        })
        .expect("the all-families fixture holds a temporal clause");
    let mut package = base.clone();
    package["semantic_graph"]["nodes"][clause]["body"]["operation"]["laws"][0]["definition"]
        ["identity"] = json!("quire.fixture.temporal-profile/v1");
    let (_, outcome) = read_fixture_wire(ALL_FAMILIES, &package);
    let Read::Refused(refusal @ V2ReadRefusal::Envelope { refusal: ir, .. }) = &outcome else {
        panic!("expected an unknown_profile refusal, got {outcome:?}");
    };
    assert_eq!(ir.code, CheckedPackageRefusalCode::UnknownProfile, "{ir:?}");
    assert_eq!(refusal.code(), Code::UnknownProfile);
    assert!(
        matches!(refusal.locus(), Some(Locus::Artifact { pointer, .. })
            if pointer.as_str()
                == format!("/semantic_graph/nodes/{clause}/body/operation/laws/0/definition")),
        "{refusal:?}"
    );
}

/// QSpec FR-322-AC-35: read the authored dependency selection mutations
/// and ordering vectors through I2. A well-shaped but unsupplied entry
/// reaches binding and refuses `missing_import` at its first selection.
#[trace("TC-253", "TC-416", "FR-087-AC-3", "FR-093-AC-16")]
#[test]
fn conformance_dependency_selection_vectors() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let vectors = read_fixture(&directory.join("../dependency-selection-vectors.json"));
    let fixture = vectors["base"].as_str().unwrap();
    let base = read_fixture(&directory.join("..").join(fixture));
    let with = |entries: &Value| {
        let mut package = base.clone();
        package["lock"]["dependency_selections"] = entries.clone();
        package
    };
    let refused_at = |outcome: &Read| -> (
        CheckedPackageRefusalCode,
        Option<CheckedPackageRefusalCause>,
        String,
    ) {
        let Read::Refused(refusal @ V2ReadRefusal::Envelope { refusal: ir, .. }) = outcome else {
            panic!("expected an envelope refusal, got {outcome:?}");
        };
        let Some(Locus::Artifact { pointer, .. }) = refusal.locus() else {
            panic!("entry refusal must be located: {refusal:?}");
        };
        (ir.code, ir.cause, pointer.as_str().to_owned())
    };
    let selections = &vectors["dependency_selections"];
    let normal = with(selections);
    let (code, _, pointer) = refused_at(&read_fixture_wire(fixture, &normal).1);
    assert_eq!(code, CheckedPackageRefusalCode::MissingImport);
    assert_eq!(pointer, "/lock/dependency_selections/0");

    let entry_mutations = vectors["entry_mutations"].as_array().unwrap();
    for mutation in entry_mutations {
        let id = mutation["id"].as_str().unwrap();
        let mut entries = selections.clone();
        entries[0] = mutation["entry"].clone();
        let (code, _, pointer) = refused_at(&read_fixture_wire(fixture, &with(&entries)).1);
        let expected = match mutation["outcome"].as_str().unwrap() {
            "refused:malformed_wire" => CheckedPackageRefusalCode::MalformedWire,
            "refused:unknown_member" => CheckedPackageRefusalCode::UnknownMember,
            other => panic!("{id}: unknown outcome {other}"),
        };
        assert_eq!(code, expected, "{id}");
        assert!(
            pointer.starts_with("/lock/dependency_selections/0"),
            "{id}: {pointer}"
        );
    }

    let order_vectors = vectors["order_vectors"].as_array().unwrap();
    for vector in order_vectors {
        let id = vector["id"].as_str().unwrap();
        let outcome = read_fixture_wire(fixture, &with(&vector["dependency_selections"])).1;
        let expected = vector["outcome"].as_str().unwrap();
        if expected == "admitted" {
            let (code, _, pointer) = refused_at(&outcome);
            assert_eq!(code, CheckedPackageRefusalCode::MissingImport, "{id}");
            assert_eq!(pointer, "/lock/dependency_selections/0", "{id}");
            continue;
        }
        let (code, cause, pointer) = refused_at(&outcome);
        assert_eq!(code, CheckedPackageRefusalCode::InvalidPackage, "{id}");
        let expected_cause = match expected {
            "refused:invalid_package/conflicting-definition" => {
                CheckedPackageRefusalCause::ConflictingDefinition
            }
            "refused:invalid_package/invalid-value" => CheckedPackageRefusalCause::InvalidValue,
            other => panic!("{id}: unknown outcome {other}"),
        };
        assert_eq!(cause, Some(expected_cause), "{id}");
        let Read::Refused(V2ReadRefusal::Envelope { refusal: ir, .. }) = &outcome else {
            unreachable!("{id}: refused_at matched an envelope refusal");
        };
        let loci: Vec<&str> = ir
            .loci
            .iter()
            .filter_map(|location| location.path.as_ref())
            .map(|path| path.as_str())
            .collect();
        let expected_loci: Vec<&str> = vector["loci"]
            .as_array()
            .unwrap()
            .iter()
            .map(|locus| locus.as_str().unwrap())
            .collect();
        assert_eq!(loci, expected_loci, "{id}");
        assert_eq!(Some(&pointer.as_str()), expected_loci.first(), "{id}");
    }
    println!(
        "conformance: {} dependency-selection entry mutations and {} order vectors",
        entry_mutations.len(),
        order_vectors.len()
    );
}

/// `Locus::Artifact` at a JSON pointer into the offered bytes.
fn artifact_locus(pointer: &str) -> Locus {
    Locus::Artifact {
        pointer: pointer.parse().unwrap(),
    }
}

/// FR-096-AC-9: IR's refusal of the contract version is
/// `unknown_wire`/`unsupported-wire`, naming the version the bytes carry and
/// the one this reader admits, located at `/contract_version`. Bytes that are not JSON refuse with no locus.
#[trace("TC-429", "FR-096-AC-9")]
#[test]
fn an_unknown_contract_version_is_unsupported_wire_at_contract_version() {
    let lock = base_lock(vec![]);
    let mut envelope = valid_envelope(&lock);
    envelope["contract_version"] = json!("quire.checked-package/v3");
    let bytes = jcs(&envelope);
    let Read::Refused(refusal) = read(&bytes) else {
        panic!("an unknown contract version must refuse");
    };
    let expected_locus = artifact_locus("/contract_version");
    assert_eq!(refusal.locus(), Some(&expected_locus));
    let record = refusal
        .unsupported_wire_record()
        .expect("an unknown contract version is unsupported-wire");
    assert_eq!(
        record.code(),
        CatalogCode::new("unknown_wire", "unsupported-wire")
    );
    assert_eq!(record.category(), Category::Refusal);
    assert_eq!(record.locus(), Some(&expected_locus));
    assert_eq!(
        record.fields(),
        &BTreeMap::from([
            ("actual", "quire.checked-package/v3".to_owned()),
            ("expected", "quire.checked-package/v2".to_owned()),
        ])
    );

    let not_json = b"not json";
    match read(not_json) {
        Read::Refused(refusal @ V2ReadRefusal::Envelope { .. }) => {
            assert_eq!(refusal.locus(), None);
            assert_eq!(refusal.unsupported_wire_record(), None);
        }
        other => panic!("bytes that are not JSON must refuse, got {other:?}"),
    }
}

/// FR-096 "The I2 reader locates its refusals in the artifact": an IR
/// refusal at a value is located at the pointer of that value -- here an
/// out-of-range structural node ordinal and a
/// graph node's tag. A repeated dependency identity is located at the
/// repeating entry, `/lock/dependency_selections/1` (FR-322).
#[trace("TC-429", "FR-096-AC-9")]
#[test]
fn a_refusal_at_a_value_is_located_at_its_pointer() {
    let lock = base_lock(vec![]);
    let mut ordinal = valid_envelope(&lock);
    ordinal["semantic_graph"]["nodes"][0]["node_id"]["ordinal"] = json!(9_007_199_254_740_992_u64);
    let mut tag = valid_envelope(&lock);
    tag["semantic_graph"]["nodes"][0]["node_tag"] = json!("nonsense");
    for (envelope, code, pointer) in [
        (
            ordinal,
            CheckedPackageRefusalCode::MalformedWire,
            "/semantic_graph/nodes/0/node_id/ordinal",
        ),
        (
            tag,
            CheckedPackageRefusalCode::UnsupportedNodeTag,
            "/semantic_graph/nodes/0/node_tag",
        ),
    ] {
        let bytes = jcs(&envelope);
        match read(&bytes) {
            Read::Refused(refused @ V2ReadRefusal::Envelope { .. }) => {
                let V2ReadRefusal::Envelope { refusal, .. } = &refused else {
                    unreachable!("matched above");
                };
                assert_eq!(refusal.code, code);
                assert_eq!(refused.locus(), Some(&artifact_locus(pointer)));
            }
            other => panic!("expected an envelope refusal at {pointer}, got {other:?}"),
        }
    }

    let lock = base_lock(vec![
        dependency_selection("geometry", "geometry-pkg"),
        dependency_selection("geometry", "geometry-pkg-2"),
    ]);
    let bytes = jcs(&valid_envelope(&lock));
    match read(&bytes) {
        Read::Refused(refusal @ V2ReadRefusal::Envelope { .. }) => {
            let V2ReadRefusal::Envelope { refusal: ir, .. } = &refusal else {
                unreachable!("matched above");
            };
            assert_eq!(ir.code, CheckedPackageRefusalCode::InvalidPackage);
            assert_eq!(
                refusal.locus(),
                Some(&artifact_locus("/lock/dependency_selections/0"))
            );
        }
        other => panic!("expected a repeated identity to refuse, got {other:?}"),
    }
}

/// FR-096-AC-10: each of IR's reader limits
/// the reader can reach, lowered below a real fixture, stops the read with
/// its own kind, the configured bound, IR's counter as actual, and
/// `Locus::Artifact` at the pointer IR reports. The reader's own artifact byte ceiling reports input bytes with
/// no locus.
#[trace("TC-429", "FR-096-AC-10")]
#[trace("TC-720", "FR-255-AC-1")]
#[trace("TC-721", "FR-255-AC-4")]
#[trace("TC-739", "FR-264-AC-4")]
#[test]
fn each_reader_limit_names_its_kind_bound_actual_and_locus() {
    let defaults = V2ReadLimits::default();
    let lock = base_lock(vec![]);
    let base = valid_envelope(&lock);
    let (_, two_nodes) = envelope_declaring(&[("pkg::A", "A"), ("pkg::B", "B")]);
    let mut diagnosed = base.clone();
    diagnosed["diagnostics"]["entries"] = json!([{
        "stage": "type_checking",
        "code": "invalid_syntax",
        "cause_tag": "malformed-json",
        "details": [],
        "loci": [],
    }]);
    let mut edged = valid_envelope(&lock);
    let node_id = edged["semantic_graph"]["nodes"][0]["node_id"].clone();
    edged["semantic_graph"]["nodes"][0]["dependencies"] = json!([node_id]);

    let cases = [
        (
            &two_nodes,
            V2ReadLimits {
                nodes: 1,
                ..defaults
            },
            Setting::I2Nodes,
            1,
            2,
            "/semantic_graph/nodes/1",
        ),
        (
            &edged,
            V2ReadLimits {
                edges: 0,
                ..defaults
            },
            Setting::I2Edges,
            0,
            1,
            "/semantic_graph/nodes/0/dependencies/0",
        ),
        (
            &base,
            V2ReadLimits {
                occurrences: 0,
                ..defaults
            },
            Setting::I2Occurrences,
            0,
            1,
            "/source_map/0",
        ),
        (
            &diagnosed,
            V2ReadLimits {
                diagnostics: 0,
                ..defaults
            },
            Setting::I2Diagnostics,
            0,
            1,
            "/diagnostics/entries/0",
        ),
        (
            &base,
            V2ReadLimits {
                work: 0,
                ..defaults
            },
            Setting::I2WorkUnits,
            0,
            1,
            "/semantic_graph/nodes/0/node_id",
        ),
    ];
    for (envelope, limits, setting, bound, actual, pointer) in cases {
        let bytes = fixture_wire_bytes(envelope);
        let outcome = read_v2(
            &bytes,
            identity("pkg"),
            limits,
            &CheckedPackageEvidence::new(),
        );
        let expected = LimitExceeded::new(setting, bound, actual).at(Some(artifact_locus(pointer)));
        assert_eq!(outcome, Read::Limit(expected.clone()), "{setting}");
        assert_eq!(expected.setting(), setting);
        assert_eq!(
            expected.catalog_code(),
            CatalogCode::new("stage_limit_exceeded", setting.kind().catalog_cause())
        );
        // The setting named in the outcome raises that limit, by name.
        let mut raised = limits;
        assert!(qsl_foundation::SettingLimits::set_bound(
            &mut raised,
            setting,
            bound + 1_000_000
        ));
        let rerun = read_v2(
            &bytes,
            identity("pkg"),
            raised,
            &CheckedPackageEvidence::new(),
        );
        assert!(!matches!(rerun, Read::Limit(_)), "{setting}: {rerun:?}");
    }

    let bytes = jcs(&base);
    let outcome = read_v2(
        &bytes,
        identity("pkg"),
        V2ReadLimits {
            artifact_bytes: bytes.len() - 1,
            ..defaults
        },
        &CheckedPackageEvidence::new(),
    );
    let Read::Limit(exceeded) = outcome else {
        panic!("the artifact byte ceiling must stop the read, got {outcome:?}");
    };
    assert_eq!(exceeded.kind(), LimitKind::InputBytes);
    assert_eq!(exceeded.locus(), None);
}

/// The single `state`/`frame` node of a QSpec `checked-package/v2` fixture.
/// Asserts there is exactly one, rather than silently taking the first of
/// several.
fn frame_node_index(package: &Value) -> usize {
    let mut frames = package["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .enumerate()
        .filter(|(_, node)| node["node_tag"] == "state" && node["semantic_form"] == "frame");
    let (index, _) = frames
        .next()
        .expect("fixture carries exactly one frame node");
    assert!(
        frames.next().is_none(),
        "fixture carries exactly one frame node"
    );
    index
}

/// FR-340: QSL's whole I2 read admits or refuses a `state`/`frame`
/// node's `modifies`/`creates`/`deletes` membership exactly as QSpec's
/// `node-identity-vectors.json` `frame_mutations` record: the closed
/// per-member node-kind eligibility table, the
/// `missing_declaration`/`missing-name` vs
/// `invalid_model_binding`/`malformed-declaration` refusal split, and their
/// precedence over each other and over a canonical-order defect (a
/// meaning-join defect always wins; the lower-keyed of two defective frames
/// is reported over a higher-keyed frame's stronger defect). Each vector
/// replaces `positive-all-families.json`'s own frame node's `dependencies`
/// and body members with its own, adds its `second_frame` node where one is
/// given, and recomputes the envelope's identity the way FR-322's own
/// emitter would. The check runs entirely through the existing I2 dispatch
/// (`read_checked_package_v2` delegates frame-body admission to IR's
/// `frame_defect`, already mapped by `map_refusal_code`); no new admission
/// path is added for it.
///
/// Read at run time from the QSpec checkout. Nothing of QSpec is
/// copied into this repository. Like this file's other QSpec conformance checks,
/// the vectors this covers (including FR-340-AC-1, AC-4 and AC-9: an
/// all-empty frame body, a misordered member array, and the lower-id
/// frame of two defective ones winning) run only when `make conformance` is
/// part of the gate, not under a plain `cargo test`/`make ci`.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn conformance_fr340_frame_mutations_match_qspec_vectors() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let vectors = read_fixture(&directory.join("../node-identity-vectors.json"));
    let mutations = vectors["frame_mutations"]
        .as_array()
        .expect("frame_mutations");
    assert!(
        !mutations.is_empty(),
        "node-identity-vectors.json holds no frame_mutations vector"
    );
    let base = read_fixture(&directory.join("positive-all-families.json"));
    let index = frame_node_index(&base);

    for mutation in mutations {
        let name = mutation["name"].as_str().expect("name");
        let mut candidate = base.clone();
        {
            let node = &mut candidate["semantic_graph"]["nodes"][index];
            node["dependencies"] = mutation["dependencies"].clone();
            node["body"]["modifies"] = mutation["modifies"].clone();
            node["body"]["creates"] = mutation["creates"].clone();
            node["body"]["deletes"] = mutation["deletes"].clone();
            // Vectors that do not exercise the scoped deletes carry none.
            node["body"]["deletes_scoped"] = mutation
                .get("deletes_scoped")
                .cloned()
                .unwrap_or_else(|| json!([]));
        }
        if let Some(second_frame) = mutation.get("second_frame") {
            candidate["semantic_graph"]["nodes"]
                .as_array_mut()
                .expect("nodes")
                .push(second_frame.clone());
        }
        let expected_code = match mutation["expected_code"].as_str().expect("expected_code") {
            "missing_declaration" => CheckedPackageRefusalCode::MissingDeclaration,
            "invalid_model_binding" => CheckedPackageRefusalCode::InvalidModelBinding,
            "invalid_semantic_graph" => CheckedPackageRefusalCode::InvalidSemanticGraph,
            "malformed_wire" => CheckedPackageRefusalCode::MalformedWire,
            other => panic!("{name}: unmapped expected_code {other}"),
        };
        let expected_cause = match mutation["expected_cause"].as_str() {
            Some("missing-name") => Some(CheckedPackageRefusalCause::MissingName),
            Some("malformed-declaration") => Some(CheckedPackageRefusalCause::MalformedDeclaration),
            Some("conflicting-binding") => Some(CheckedPackageRefusalCause::ConflictingBinding),
            None => None,
            Some(other) => panic!("{name}: unmapped expected_cause {other}"),
        };
        let expected_locus = mutation
            .get("expected_locus_node_id")
            .or_else(|| mutation.get("expected_locus"))
            .expect("published structural locus");

        let (_, outcome) = read_fixture_wire(ALL_FAMILIES, &candidate);
        match outcome {
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => {
                assert_eq!(refusal.code, expected_code, "{name}: code ({refusal:?})");
                assert_eq!(refusal.cause, expected_cause, "{name}: cause");
                assert_eq!(
                    refusal
                        .locus
                        .as_ref()
                        .map(|node| serde_json::to_value(node).unwrap()),
                    Some(expected_locus.clone()),
                    "{name}: locus"
                );
            }
            other => panic!("{name}: expected a refusal, got {other:?}"),
        }
    }
    println!(
        "conformance: {} frame-body mutation vectors matched",
        mutations.len()
    );
}

/// QSpec FR-322 "Operand index segments" (TC-233 NI-01..NI-12): each
/// `node-index-vectors.json` case, its RFC 6902 patch applied to the base
/// package, is refused `malformed_wire` by QSL's whole I2 read. A schema-invalid
/// operand (a nested `node_id` object, a negative, string or fractional
/// index, an extra or missing member, an index segment in a
/// `dependency_reference` node) is refused within the patched member; an
/// index past the graph, a self-naming node and two nodes naming each other
/// are refused with no cause at the published `node` member.
#[trace(
    "QSpec-TC-233",
    "QSpec-FR-322-AC-278",
    "QSpec-FR-322-AC-279",
    "QSpec-FR-322-AC-280"
)]
#[test]
fn conformance_node_index_vectors_refuse_malformed_wire() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let proposals = directory.parent().expect("the proposal directory");
    let vectors = read_fixture(&proposals.join("node-index-vectors.json"));
    let fixture = vectors["base"].as_str().expect("a base fixture");
    let base = read_fixture(&proposals.join(fixture));
    let cases = vectors["cases"].as_array().expect("NI cases");
    assert!(!cases.is_empty(), "node-index-vectors.json holds no case");
    for case in cases {
        let id = &case["id"];
        assert_eq!(case["expected"], "malformed_wire", "{id}: expected outcome");
        let locus = case["locus"].as_str().expect("a locus");
        let mut package = base.clone();
        apply_json_patch(&mut package, &case["patch"]);
        let (_, outcome) = read_fixture_wire(fixture, &package);
        match case["schema"].as_str().expect("a schema outcome") {
            "invalid" => {
                let Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) = &outcome else {
                    panic!("{id}: expected malformed_wire, got {outcome:?}");
                };
                assert_eq!(
                    refusal.code,
                    CheckedPackageRefusalCode::MalformedWire,
                    "{id}"
                );
                let pointer = locus_pointer(refusal).unwrap_or_default();
                assert!(pointer.starts_with(locus), "{id}: {pointer} within {locus}");
            }
            "valid" => assert_timed_refusal(&outcome, "malformed_wire", None, locus, id),
            other => panic!("{id}: unknown schema outcome {other}"),
        }
    }
    println!("conformance: {} node-index vectors refused", cases.len());
}

/// A `model`/`object_type` node (FR-340's `creates`/`deletes`-eligible, and
/// -- as a `modifies` field entry's `declaration` -- `modifies`-eligible,
/// model member kind), typed by `type_ref`, with the minimal valid
/// empty-`aggregate` body real fixtures use.
fn model_node(label: &str, form: &str, qualified_name: &str, type_ref: &Value) -> Value {
    json!({
        "node_id": node_ref(label),
        "schema_version": "quire.checked-semantic-graph/v2",
        "node_tag": "model",
        "semantic_form": form,
        "semantic_type": type_ref,
        "declaration": {"qualified_name": [qualified_name]},
        "owner": declared_owner(),
        "dependencies": [],
        "body": {"term": "aggregate", "members": []},
    })
}

/// [`graph_node`] with the empty `aggregate` body QSpec's own self-typed
/// scalar types carry: the usual literal body names the node's own id, so the
/// node's derived key would depend on itself and never settle.
fn keyable_type_node(label: &str, export: &str) -> Value {
    let mut node = graph_node(label, export);
    node["semantic_form"] = json!("text");
    node["body"] = json!({"term": "aggregate", "members": []});
    node
}

fn model_graph_node(label: &str, form: &str, qualified_name: &str, type_ref: &Value) -> Value {
    let mut node = model_node(label, form, qualified_name, type_ref);
    node["occurrences"] = json!([{"role": "declaration", "ordinal": 0}]);
    node
}

/// A `state`/`frame` `modifies` entry naming a `relation`/`relationship`
/// node (STD-111's `{kind: "relationship", declaration}` shape).
fn modifies_relationship(declaration: &Value) -> Value {
    json!({"kind": "relationship", "declaration": declaration})
}

/// A `state`/`frame` `modifies` entry naming a field of `declaration`, a
/// `model`/`object_type` or `model`/`record_value_type` node (STD-111's
/// `{kind: "field", declaration, name}` shape).
fn modifies_field(declaration: &Value, name: &str) -> Value {
    json!({"kind": "field", "declaration": declaration, "name": name, "scope": "every-object"})
}

/// A `state`/`frame` node whose `dependencies` and body members are given
/// verbatim.
fn frame_node(
    label: &str,
    type_ref: &Value,
    dependencies: Vec<Value>,
    modifies: Vec<Value>,
    creates: Vec<Value>,
    deletes: Vec<Value>,
) -> Value {
    json!({
        "node_id": node_ref(label),
        "schema_version": "quire.checked-semantic-graph/v2",
        "node_tag": "state",
        "semantic_form": "frame",
        "semantic_type": type_ref,
        "dependencies": dependencies,
        // A `state`/`frame` node's own closed occurrence vocabulary is
        // `generated` (`structural.rs`'s `Frame => CheckedOccurrenceRole::
        // Generated`), not `declaration`.
        "occurrences": [{"role": "generated", "ordinal": 0}],
        "body": {
            "term": "frame",
            "modifies": modifies,
            "creates": creates,
            "deletes": deletes,
            "deletes_scoped": [],
        },
    })
}

/// A source-map entry for a node's own occurrence.
fn source_map_entry_for_node(node_id: &Value, role: &str) -> Value {
    json!({
        "node_id": node_id,
        "role": role,
        "ordinal": 0,
        "regions": [{"source": source_ref("src"), "start": 0, "end": 1}],
    })
}

/// A complete minimal envelope over the supplied nodes.
fn valid_envelope_over(nodes: Vec<Value>) -> Value {
    let mut envelope = valid_envelope(&base_lock(vec![]));
    envelope["source_map"] = nodes_source_map(&nodes);
    envelope["semantic_graph"]["nodes"] = Value::Array(nodes);
    envelope
}

fn nodes_source_map(nodes: &[Value]) -> Value {
    Value::Array(
        nodes
            .iter()
            .map(|node| {
                let role = node["occurrences"][0]["role"].as_str().unwrap();
                source_map_entry_for_node(&node["node_id"], role)
            })
            .collect(),
    )
}

/// A minimal always-run FR-340 frame package with one type, object and frame.
/// Locally constructed; this stands in for the missing FR-340 fixture.
fn frame_fixture(frame_body: impl FnOnce(&Value) -> (Vec<Value>, Vec<Value>, Vec<Value>)) -> Value {
    let type_node = keyable_type_node("pkg::T", "T");
    let type_ref = type_node["node_id"].clone();
    let object_ref = node_ref("pkg::O");
    let (modifies, creates, deletes) = frame_body(&object_ref);
    let object_node = model_graph_node("pkg::O", "object_type", "O", &type_ref);
    let frame = frame_node(
        "pkg::Frame",
        &object_ref,
        vec![object_ref.clone()],
        modifies,
        creates,
        deletes,
    );
    valid_envelope_over(vec![type_node, object_node, frame])
}

fn read_frame_fixture(envelope: &Value) -> Read {
    let (_, outcome) = read_fixture_wire_holding(None, envelope, envelope, &[]);
    outcome
}

/// FR-340: a `state`/`frame` node whose `modifies`/`creates`/`deletes`
/// entries are eligible members of their own declared `dependencies` (a
/// `field` entry naming an `object_type` in `modifies`, and the same
/// `object_type` in `creates`) is admitted through QSL's whole I2 read.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn frame_body_membership_against_dependencies_is_admitted() {
    let envelope = frame_fixture(|object| {
        (
            vec![modifies_field(object, "value")],
            vec![object.clone()],
            vec![],
        )
    });
    let outcome = read_frame_fixture(&envelope);
    assert!(
        matches!(outcome, Read::Verified { .. }),
        "a frame whose modifies/creates entries are both declared dependencies of \
         their own eligible kind must be admitted, got {outcome:?}"
    );
}

/// FR-340: an entry naming a node that no dependency of the frame declares
/// -- here, a node absent from the whole graph -- refuses
/// `missing_declaration`/`missing-name`, located at the offending entry, and
/// this crate's own `V2ReadRefusal::code()` maps that to `Code::
/// MissingDeclaration`. The other half of FR-340-AC-13 (a real node elsewhere
/// in the graph that is simply never declared to this frame) is covered by
/// the `undeclared-entry` vector in
/// `conformance_fr340_frame_mutations_match_qspec_vectors`, not repeated here.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn frame_entry_outside_dependencies_refuses_as_missing_declaration() {
    let field_id = node_ref("pkg::F");
    let type_node = keyable_type_node("pkg::T", "T");
    let type_ref = type_node["node_id"].clone();
    let object_ref = node_ref("pkg::O");
    let object_node = model_graph_node("pkg::O", "object_type", "O", &type_ref);
    // `F` is never declared as a dependency of this frame, only named in
    // `modifies`.
    let frame = frame_node(
        "pkg::Frame",
        &object_ref,
        vec![object_ref.clone()],
        vec![modifies_field(&node_ref("pkg::F"), "value")],
        vec![],
        vec![],
    );
    let envelope = valid_envelope_over(vec![type_node, object_node, frame]);
    let outcome = read_frame_fixture(&envelope);
    match &outcome {
        Read::Refused(outer @ V2ReadRefusal::Envelope { refusal, .. }) => {
            assert_eq!(refusal.code, CheckedPackageRefusalCode::MissingDeclaration);
            assert_eq!(refusal.cause, Some(CheckedPackageRefusalCause::MissingName));
            assert_eq!(
                refusal
                    .locus
                    .as_ref()
                    .map(|node| serde_json::to_value(node).unwrap()),
                Some(field_id.clone())
            );
            assert_eq!(outer.code(), Code::MissingDeclaration);
        }
        other => panic!("expected a missing_declaration refusal, got {other:?}"),
    }
}

/// FR-340: a `modifies` entry claiming the `relationship` kind for a
/// declared dependency whose node kind that kind does not admit (a
/// `model`/`object_type` node, which `relationship` never admits -- only a
/// `relation`/`relationship` node) refuses
/// `invalid_model_binding`/`malformed-declaration`, located at the offending
/// entry, and this crate's own `V2ReadRefusal::code()` maps that to
/// `Code::InvalidModelBinding`.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn frame_entry_of_an_ineligible_kind_refuses_as_invalid_model_binding() {
    let envelope = frame_fixture(|object| {
        // `object` (`model`/`object_type`) is declared, but a `relationship`
        // kind entry admits only a `relation`/`relationship` node, never
        // `model`/`object_type`.
        (vec![modifies_relationship(object)], vec![], vec![])
    });
    let object_id = node_ref("pkg::O");
    let outcome = read_frame_fixture(&envelope);
    match &outcome {
        Read::Refused(outer @ V2ReadRefusal::Envelope { refusal, .. }) => {
            assert_eq!(refusal.code, CheckedPackageRefusalCode::InvalidModelBinding);
            assert_eq!(
                refusal.cause,
                Some(CheckedPackageRefusalCause::MalformedDeclaration)
            );
            assert_eq!(
                refusal
                    .locus
                    .as_ref()
                    .map(|node| serde_json::to_value(node).unwrap()),
                Some(object_id.clone())
            );
            assert_eq!(outer.code(), Code::InvalidModelBinding);
        }
        other => panic!("expected an invalid_model_binding refusal, got {other:?}"),
    }
}

/// ADR-030: gathering a 100,000-long dependency closure runs on a 128 KiB
/// stack and holds every package of it once. A recursive walk would use one
/// frame per link.
#[trace("FR-099-AC-7", "TC-446")]
#[test]
fn a_dependency_closure_of_any_length_is_held_on_a_small_stack() {
    const LINKS: usize = 100_000;
    let chain = crate::checked::dependency_chain(LINKS);
    let held = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn_scoped(scope, || {
                let mut held = BTreeMap::new();
                super::hold_closure(&chain, &mut held);
                held.len()
            })
            .expect("spawn a 128 KiB thread")
            .join()
            .expect("holding the closure must not overflow a 128 KiB stack")
    });
    assert_eq!(held, LINKS);
}

/// Runs one vector of `extension-definition-vectors.json`, asserting its
/// `expected` result: admitted, or `unknown_required_capability` with the
/// vector's code, no cause, `locus` and `document_pointer`.
fn assert_extension_vector(directory: &std::path::Path, vector: &Value) {
    let id = vector["id"].as_str().unwrap();
    let fixture = vector["fixture"].as_str().unwrap();
    let envelope = read_fixture(&directory.join(fixture));
    let withheld: Vec<CheckedArtifactRef> =
        serde_json::from_value(vector["withheld"].clone()).expect("withheld refs");
    let outcome = read_fixture_wire_withholding(fixture, &envelope, &withheld).1;
    let expected = &vector["expected"];
    match (expected["result"].as_str().unwrap(), outcome) {
        ("admitted", Read::Verified { .. }) => {}
        ("refused", Read::Refused(V2ReadRefusal::Envelope { refusal, locus })) => {
            assert_eq!(
                serde_json::to_value(&refusal.code).unwrap(),
                expected["code"],
                "{id}"
            );
            assert_eq!(refusal.cause, None, "{id}");
            let node = refusal
                .locus
                .as_ref()
                .unwrap_or_else(|| panic!("{id}: no locus"));
            assert_eq!(
                serde_json::to_value(node).unwrap(),
                expected["locus"],
                "{id}"
            );
            let pointer = expected["document_pointer"].as_str().unwrap();
            assert_eq!(
                refusal.document_pointer.as_ref().map(|p| p.as_str()),
                Some(pointer),
                "{id}"
            );
            assert!(
                matches!(locus.as_deref(), Some(Locus::Artifact { pointer: at, .. })
                    if at.as_str() == pointer),
                "{id}: {locus:?}"
            );
        }
        (_, other) => panic!("{id}: expected {expected}, got {other:?}"),
    }
}

/// QSpec's `extension-definition-vectors.json` (XD-01..04, TC-951 X19):
/// the published extension fixtures read with every published definition
/// held are admitted, and read with the `retention` definition withheld
/// refuse `unknown_required_capability` with no cause, at the vector's node
/// `locus` and `document_pointer`. A refusal returns no package, so the
/// lock, source map and node are unchanged by construction (X05).
///
/// Read from the QSpec checkout.
#[trace("FR-469-AC-10", "FR-469-AC-3", "TC-951")]
#[test]
fn extension_definition_vectors_xd_01_to_04() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let package_dir = directory.join("..");
    let file = read_fixture(&package_dir.join("extension-definition-vectors.json"));
    assert_eq!(
        file["definitions"], "extension-definitions.json",
        "the vectors name the definitions file the loader reads"
    );
    let vectors = file["vectors"].as_array().expect("vectors");
    assert!(!vectors.is_empty());
    for vector in vectors {
        assert_extension_vector(&package_dir, vector);
    }
}

/// TC-951 X06: with the `retention` entry removed from
/// `lock.definition_selections`, a reader that holds the definition and one
/// that does not both refuse `missing_declaration`/`missing-selection` at
/// `/semantic_graph/nodes/{i}/body/definition`, before the held check.
///
/// Read from the QSpec checkout.
#[trace("FR-469-AC-3", "TC-951")]
#[test]
fn extension_definition_absent_from_selections_refuses_missing_selection() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let fixture = "fixtures/positive-extension-nodes.json";
    let original = read_fixture(&directory.join("positive-extension-nodes.json"));
    let mut mutated = original.clone();
    {
        let pointer = "/lock/definition_selections";
        let rows = mutated
            .pointer_mut(pointer)
            .and_then(Value::as_array_mut)
            .expect("definition_selections");
        let before = rows.len();
        rows.retain(|row| row["identity"] != "retention");
        assert_eq!(rows.len(), before - 1, "the fixture selects retention once");
    }
    let index = mutated["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node["node_tag"] == "extension")
        .expect("the fixture carries an extension node");
    let held = CheckedArtifactRef {
        authority: "example".into(),
        identity: "retention".into(),
    };
    // The first reader holds `retention` (its selection source is the
    // original lock); the second holds nothing of it.
    for (holds, outcome) in [
        (
            true,
            read_fixture_wire_holding(Some(fixture), &mutated, &original, &[]).1,
        ),
        (
            false,
            read_fixture_wire_withholding(fixture, &mutated, &[held]).1,
        ),
    ] {
        let Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) = outcome else {
            panic!("holds={holds}: expected a refusal, got {outcome:?}");
        };
        assert_eq!(refusal.code, CheckedPackageRefusalCode::MissingDeclaration);
        assert_eq!(
            refusal.cause,
            Some(CheckedPackageRefusalCause::MissingSelection),
            "holds={holds}"
        );
        assert_eq!(
            locus_pointer(&refusal),
            Some(format!("/semantic_graph/nodes/{index}/body/definition")),
            "holds={holds}"
        );
    }
}

/// TC-951 X18: `extension-definitions.json` meets its schema, holds one
/// entry per reference in reference (UTF-8 byte) order, and pairs each
/// grammar production with a typed-node entry of the same keyword whose
/// clause names are the production's clauses.
///
/// Read from the QSpec checkout.
#[trace("FR-469-AC-9", "TC-951")]
#[test]
fn published_extension_definitions_conform_to_their_schema_and_pairing() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let package_dir = directory.join("..");
    let file = read_fixture(&package_dir.join("extension-definitions.json"));
    let schema = read_fixture(&package_dir.join("extension-definitions.schema.json"));
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .expect("the schema compiles");
    if let Err(errors) = compiled.validate(&file) {
        let errors: Vec<_> = errors.map(|error| error.to_string()).collect();
        panic!("extension-definitions.json violates its schema: {errors:?}");
    }
    let entries = file["definitions"].as_array().unwrap();
    assert!(!entries.is_empty());
    let references: Vec<(&str, &str)> = entries
        .iter()
        .map(|entry| {
            (
                entry["reference"]["authority"].as_str().unwrap(),
                entry["reference"]["identity"].as_str().unwrap(),
            )
        })
        .collect();
    assert!(
        references.windows(2).all(|pair| pair[0] < pair[1]),
        "one entry per reference, in reference order: {references:?}"
    );
    for entry in entries {
        let productions = entry["definition"]["grammar_schema"]["productions"]
            .as_array()
            .unwrap();
        let nodes = entry["definition"]["typed_node_schema"]["nodes"]
            .as_array()
            .unwrap();
        for production in productions {
            let node = nodes
                .iter()
                .find(|node| node["keyword"] == production["keyword"])
                .unwrap_or_else(|| panic!("no node entry for {}", production["keyword"]));
            let names: Vec<_> = node["clauses"]
                .as_array()
                .unwrap()
                .iter()
                .map(|clause| clause["name"].clone())
                .collect();
            assert_eq!(
                &names,
                production["clauses"].as_array().unwrap(),
                "{}",
                production["keyword"]
            );
        }
    }
}

/// IR FR-038-AC-292: a `wrong_snapshot` envelope (a `choice`/`repeat` decision
/// reading a fact outside its role's visible set) reaches QSL as the typed
/// `wrong_snapshot` code, not as a generic invalid package.
#[test]
fn an_ir_wrong_snapshot_refusal_maps_to_the_wrong_snapshot_code() {
    use quire_contract_model::CheckedPackageRefusalCode as Ir;
    for (ir, qsl) in [
        (Ir::WrongSnapshot, Code::WrongSnapshot),
        (Ir::IllTyped, Code::IllTyped),
        (Ir::InvalidPackage, Code::InvalidPackage),
    ] {
        assert_eq!(super::map_refusal_code(ir), qsl);
    }
}

fn qspec_directory() -> Option<std::path::PathBuf> {
    std::env::var_os("QSPEC_DIR").map(std::path::PathBuf::from)
}

/// The primary location's pointer of an IR refusal, if it has one.
fn locus_pointer(refusal: &quire_contract_model::CheckedPackageRefusal) -> Option<String> {
    refusal
        .loci
        .first()
        .and_then(|location| location.path.as_ref())
        .map(|path| path.as_str().to_owned())
}
