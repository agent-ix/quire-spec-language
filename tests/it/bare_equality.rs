// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531 "The equality file" and FR-533 "Slot naming on the bytes route"
//! (TC-1331, TC-1333, TC-1312): the equality file the handoff writer writes
//! for TC-1070's protocols, its reader and refusals, and the origin
//! resolution of span-named slots over the package alone.

use std::collections::BTreeMap;
use std::path::Path;

use ix_trace_rs::trace;
use qsl_package::{
    BarePackage, BareSelectionFailure, BareSupply, EqualityFileRefusal, EqualityWitnessRefusal,
    ProtocolSlotDescriptor, ReadEqualityRows, ReadIdentityRows,
};
use quire_contract_model::CheckedPackageV2;
use quire_exact::NodeKey;
use quire_spec_language::checked_handoff::{evidence, write, CheckedHandoffRequest, Corpus};
use quire_spec_language::emit_checked_v2::Request;
use serde_json::Value;

use super::checked_handoff::{admitted, request, tree};

#[path = "../../qsl-semantics/tests/support/tc1070.rs"]
mod tc1070;

/// The request that compiles TC-1070's source `source` (fixtures E and L).
fn fixture_request(directory: &Path, source: &str) -> CheckedHandoffRequest {
    let unit = directory.join("unit.native");
    let model = directory.join("model.json");
    std::fs::write(&unit, source).expect("source");
    std::fs::write(&model, serde_json::to_vec(&tc1070::document()).expect("model")).expect("model");
    CheckedHandoffRequest {
        emit: Request {
            source: unit,
            package_name: "acceptance".into(),
            models: vec![model],
            libraries: Vec::new(),
            execution_points: None,
            temporal_selections: Vec::new(),
            out: None,
        },
        corpus: Corpus::None,
    }
}

/// The handoff directory of `source`: its request, files and admitted package.
fn handoff(source: &str) -> (tempfile::TempDir, CheckedHandoffRequest, BTreeMap<String, Vec<u8>>) {
    let holder = tempfile::tempdir().expect("directory");
    let request = fixture_request(holder.path(), source);
    let out = holder.path().join("handoff");
    write(&request, &out).expect("the handoff is written");
    let files = tree(&out);
    (holder, request, files)
}

fn read(request: &CheckedHandoffRequest, bytes: &[u8]) -> Box<CheckedPackageV2> {
    admitted(bytes, &evidence(request).expect("evidence"))
}

/// The key of the protocol node `first ... last` declares.
fn node_key(package: &CheckedPackageV2, first: &str, last: &str) -> NodeKey {
    let ids = qsl_semantics::check::wire_id::WireGraph::new(
        package.graph().nodes.iter().map(|node| &node.node_id),
    )
    .expect("wire ids");
    let node = package
        .graph()
        .nodes
        .iter()
        .find(|node| {
            &*node.node_tag == "protocol"
                && node.declaration.as_ref().is_some_and(|d| {
                    d.qualified_name.first().map(|s| &**s) == Some(first)
                        && d.qualified_name.last().map(|s| &**s) == Some(last)
                })
        })
        .unwrap_or_else(|| panic!("{first} {last} is declared"));
    ids.key(&node.node_id).expect("key")
}

fn select(
    package: &CheckedPackageV2,
    identities: &ReadIdentityRows,
    equality: Option<&ReadEqualityRows>,
    node: NodeKey,
    member: &str,
    index: Option<usize>,
) -> Result<qsl_package::BareSlot, BareSelectionFailure> {
    BarePackage::new(package)
        .with_supply(BareSupply {
            identities,
            equality,
        })
        .select_protocol_slot(&ProtocolSlotDescriptor::new(node, member, index))
}

/// FR-531-AC-14, FR-517-AC-21: the directory written for fixture E holds
/// `equality.json`; reading it with the package gives rows that encode back
/// to the file, the file is stable across writes, and the retry relation of
/// `Refund` (an integer `=`) selects over them.
#[trace("FR-531-AC-14", "FR-517-AC-21", "TC-1331", "TC-1312")]
#[test]
fn the_directory_for_fixture_e_holds_the_equality_file_a_slot_call_reads() {
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let file = &files["equality.json"];
    let package = read(&request, &files["checked-package.json"]);
    let text: Value = serde_json::from_slice(file).expect("JSON");
    assert_eq!(text["format"], "quire.checked-handoff.equality/1");
    assert!(file.ends_with(b"\n"));
    for member in ["applications", "facts"] {
        assert!(
            text[member].as_array().is_some_and(|rows| !rows.is_empty()),
            "{member} holds the integer equality rows"
        );
    }
    let rows = ReadEqualityRows::read(file, &package).expect("the file reads");
    assert_eq!(rows.to_bytes(&package).expect("encodes"), *file);
    let (_second, _, again) = handoff(tc1070::SOURCE);
    assert_eq!(again["equality.json"], *file, "two writes are byte-identical");
    let identities = ReadIdentityRows::read(&files["identities.json"], &package).expect("rows");
    let refund = node_key(&package, "Refunds", "Refund");
    let relation = select(&package, &identities, Some(&rows), refund.clone(), "retry.relation", None)
        .expect("the retry relation selects over the equality rows");
    assert!(relation.origin().is_some());
    // FR-531-AC-16: no equality application in the walk, no equality read.
    select(&package, &identities, None, refund, "trigger.guard", None)
        .expect("the trigger guard needs no equality rows");
}

/// FR-531-AC-16, FR-533-AC-15 (equality half): a missing, foreign and
/// unknown equality witness each refuse the retry relation.
#[trace("FR-531-AC-16", "FR-533-AC-15", "TC-1331", "TC-1333")]
#[test]
fn the_equality_witness_refusals_are_missing_foreign_and_unknown() {
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let package = read(&request, &files["checked-package.json"]);
    let identities = ReadIdentityRows::read(&files["identities.json"], &package).expect("rows");
    let refund = node_key(&package, "Refunds", "Refund");
    let refusal = |equality: Option<&ReadEqualityRows>| {
        match select(&package, &identities, equality, refund.clone(), "retry.relation", None) {
            Err(BareSelectionFailure::Equality(refusal)) => refusal,
            other => panic!("not an equality refusal: {:?}", other.map(|_| ())),
        }
    };
    assert_eq!(refusal(None), EqualityWitnessRefusal::Missing);
    // Rows of another package.
    let other = request_other_package();
    assert_eq!(refusal(Some(&other)), EqualityWitnessRefusal::Foreign);
    // Rows that join no equality application of this package.
    let empty = ReadEqualityRows::read(&empty_file(), &package).expect("empty rows read");
    assert_eq!(refusal(Some(&empty)), EqualityWitnessRefusal::Unknown);
}

fn empty_file() -> Vec<u8> {
    b"{\n  \"format\": \"quire.checked-handoff.equality/1\",\n  \"applications\": [],\n  \"facts\": [],\n  \"source_units\": []\n}\n".to_vec()
}

/// The equality rows of recipe R's package: a package other than fixture E's.
fn request_other_package() -> ReadEqualityRows {
    let request = request("event_position", Corpus::None);
    let holder = tempfile::tempdir().expect("directory");
    let out = holder.path().join("handoff");
    write(&request, &out).expect("written");
    let files = tree(&out);
    let package = read(&request, &files["checked-package.json"]);
    ReadEqualityRows::read(&files["equality.json"], &package).expect("reads")
}

/// FR-517-AC-21 (second half): a unit with no equality application writes
/// the file with the three lists empty.
#[trace("FR-517-AC-21", "TC-1312")]
#[test]
fn a_unit_without_an_equality_application_writes_the_file_empty() {
    let request = request("event_position", Corpus::None);
    let holder = tempfile::tempdir().expect("directory");
    let out = holder.path().join("handoff");
    write(&request, &out).expect("written");
    assert_eq!(tree(&out)["equality.json"], empty_file());
}

/// FR-531-AC-15: the closed refusals of `ReadEqualityRows::read`.
#[trace("FR-531-AC-15", "TC-1331")]
#[test]
fn the_equality_file_reader_refuses_as_the_spec_closes_it() {
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let package = read(&request, &files["checked-package.json"]);
    let good: Value = serde_json::from_slice(&files["equality.json"]).expect("JSON");
    let read = |bytes: &[u8]| ReadEqualityRows::read(bytes, &package);
    let json = |value: &Value| serde_json::to_vec(value).expect("bytes");
    assert!(matches!(read(b"not json"), Err(EqualityFileRefusal::Malformed { .. })));
    let mut unknown = good.clone();
    unknown["extra"] = Value::Null;
    assert!(matches!(read(&json(&unknown)), Err(EqualityFileRefusal::Malformed { .. })));
    let repeated = String::from_utf8(empty_file())
        .expect("text")
        .replace("\"facts\": []", "\"facts\": [],\n  \"facts\": []");
    assert!(matches!(read(repeated.as_bytes()), Err(EqualityFileRefusal::Malformed { .. })));
    let mut tag = good.clone();
    tag["facts"][0]["fact"]["kind"] = "nonsense".into();
    assert!(matches!(read(&json(&tag)), Err(EqualityFileRefusal::Malformed { .. })));
    let mut format = good.clone();
    format["format"] = "quire.checked-handoff.equality/2".into();
    assert_eq!(
        read(&json(&format)).err(),
        Some(EqualityFileRefusal::Format("quire.checked-handoff.equality/2".into()))
    );
    let mut foreign = good.clone();
    foreign["applications"][0]["application"]["package"] = "elsewhere".into();
    assert_eq!(
        read(&json(&foreign)).err(),
        Some(EqualityFileRefusal::Unjoined { member: "applications", index: 0 })
    );
    // An application of the package that is no equality application.
    let mut plain = good.clone();
    let other = good["applications"][0]["application"].clone();
    let identity = &files["checked-package.json"];
    let wire: Value = serde_json::from_slice(identity).expect("package");
    let not_equality = wire["graph"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|n| n["node_id"] != other && n["node_tag"] == "value"))
        .map(|node| node["node_id"].clone())
        .expect("a value node");
    plain["applications"][0]["application"] = not_equality;
    assert_eq!(
        read(&json(&plain)).err(),
        Some(EqualityFileRefusal::Unjoined { member: "applications", index: 0 })
    );
}
