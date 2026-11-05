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
    std::fs::write(
        &model,
        serde_json::to_vec(&tc1070::document()).expect("model"),
    )
    .expect("model");
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
fn handoff(
    source: &str,
) -> (
    tempfile::TempDir,
    CheckedHandoffRequest,
    BTreeMap<String, Vec<u8>>,
) {
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
///
/// Selection level only: FR-531-AC-14's evaluation parity (value and charge
/// log) belongs to the evaluation call (B2b), so only FR-517-AC-21 is tagged.
#[trace("FR-517-AC-21", "TC-1331", "TC-1312")]
#[test]
fn the_directory_for_fixture_e_holds_the_equality_file_a_slot_call_reads() {
    let (holder, request, files) = handoff(tc1070::SOURCE);
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
    // The emitter produces no selected source-unit row for fixture E (it
    // declares no unit), so FR-517-AC-21's non-empty `source_units` is not met.
    assert!(text["source_units"].as_array().is_some_and(Vec::is_empty));
    assert_eq!(rows.to_bytes(&package).expect("encodes"), *file);
    let again = holder.path().join("again");
    write(&request, &again).expect("a second write");
    assert_eq!(
        tree(&again)["equality.json"],
        *file,
        "two writes of equal requests are byte-identical"
    );
    let identities = ReadIdentityRows::read(&files["identities.json"], &package).expect("rows");
    let refund = node_key(&package, "Refunds", "Refund");
    let relation = select(
        &package,
        &identities,
        Some(&rows),
        refund.clone(),
        "retry.relation",
        None,
    )
    .expect("the retry relation selects over the equality rows");
    assert!(relation.origin().is_some());
    // FR-531-AC-16: no equality application in the walk, no equality read.
    select(&package, &identities, None, refund, "trigger.guard", None)
        .expect("the trigger guard needs no equality rows");
}

/// FR-531-AC-16, FR-533-AC-15 (equality half): a missing, foreign and
/// unknown equality witness each refuse the retry relation.
///
/// Selection level only: the `Admission` failure, exit 21, `Package` locus and
/// meter halves of FR-531-AC-16 and FR-533-AC-15 are B2b's, so those ACs are
/// not tagged here.
#[trace("TC-1331", "TC-1333")]
#[test]
fn the_equality_witness_refusals_are_missing_foreign_and_unknown() {
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let package = read(&request, &files["checked-package.json"]);
    let identities = ReadIdentityRows::read(&files["identities.json"], &package).expect("rows");
    let refund = node_key(&package, "Refunds", "Refund");
    let refusal = |equality: Option<&ReadEqualityRows>| match select(
        &package,
        &identities,
        equality,
        refund.clone(),
        "retry.relation",
        None,
    ) {
        Err(BareSelectionFailure::Equality(refusal)) => refusal,
        other => panic!("not an equality refusal: {:?}", other.map(|_| ())),
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

/// FR-517-AC-21 (second half): a unit with no `==` or `!=` application
/// writes the file with the three lists empty.
#[trace("FR-517-AC-21", "TC-1312")]
#[test]
fn a_unit_without_an_equality_application_writes_the_file_empty() {
    let holder = tempfile::tempdir().expect("directory");
    let mut request = fixture_request(
        holder.path(),
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n\
         function f using v(x: Int[0, 9], p: Boolean): Integer pure { if p then x + 1 else x + 1 }\n",
    );
    request.emit.models.clear();
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
    assert!(matches!(
        read(b"not json"),
        Err(EqualityFileRefusal::Malformed { .. })
    ));
    let mut unknown = good.clone();
    unknown["extra"] = Value::Null;
    assert!(matches!(
        read(&json(&unknown)),
        Err(EqualityFileRefusal::Malformed { .. })
    ));
    let repeated = String::from_utf8(empty_file())
        .expect("text")
        .replace("\"facts\": []", "\"facts\": [],\n  \"facts\": []");
    assert!(matches!(
        read(repeated.as_bytes()),
        Err(EqualityFileRefusal::Malformed { .. })
    ));
    let mut tag = good.clone();
    tag["facts"][0]["fact"]["kind"] = "nonsense".into();
    assert!(matches!(
        read(&json(&tag)),
        Err(EqualityFileRefusal::Malformed { .. })
    ));
    let mut format = good.clone();
    format["format"] = "quire.checked-handoff.equality/2".into();
    assert_eq!(
        read(&json(&format)).err(),
        Some(EqualityFileRefusal::Format(
            "quire.checked-handoff.equality/2".into()
        ))
    );
    let mut foreign = good.clone();
    foreign["applications"][0]["application"]["package"] = "elsewhere".into();
    assert_eq!(
        read(&json(&foreign)).err(),
        Some(EqualityFileRefusal::Unjoined {
            member: "applications",
            index: 0
        })
    );
    // An application of the package that is no equality application.
    let mut plain = good.clone();
    let other = good["applications"][0]["application"].clone();
    let not_equality = package
        .graph()
        .nodes
        .iter()
        .map(|node| serde_json::to_value(&node.node_id).expect("id"))
        .find(|id| *id != other)
        .expect("another node");
    plain["applications"][0]["application"] = not_equality;
    assert_eq!(
        read(&json(&plain)).err(),
        Some(EqualityFileRefusal::Unjoined {
            member: "applications",
            index: 0
        })
    );
}

const DECISION: &str = "protocol Pipeline run Main Decide decision ";

/// The package of fixture L's handoff with `edit` applied to its
/// `protocol_locations` rows, re-admitted by the reader.
fn edited(
    request: &CheckedHandoffRequest,
    files: &BTreeMap<String, Vec<u8>>,
    edit: impl FnOnce(&mut Vec<Value>),
) -> Box<CheckedPackageV2> {
    let mut wire: Value = serde_json::from_slice(&files["checked-package.json"]).expect("package");
    let rows = wire["checked_term_data"]["protocol_locations"]
        .as_array_mut()
        .expect("rows");
    edit(rows);
    read(request, &serde_json::to_vec(&wire).expect("bytes"))
}

fn origin_name(row: &Value) -> &str {
    row["location"]["origin"]["name"]
        .as_str()
        .unwrap_or_default()
}

/// The origins of `Decide`'s decision family, in the order of their names.
fn decision_origins(package: &CheckedPackageV2) -> Vec<String> {
    let mut names: Vec<String> = package
        .checked_term_data()
        .expect("term data")
        .protocol_locations
        .iter()
        .filter_map(|row| match &row.location.origin {
            quire_contract_model::CheckedProtocolLocationOrigin::TypeDeclaration { name }
                if name.starts_with(DECISION) =>
            {
                Some(name.to_string())
            }
            _ => None,
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

fn decide_origin(
    package: &CheckedPackageV2,
    files: &BTreeMap<String, Vec<u8>>,
    member: &str,
    index: usize,
) -> Result<Option<String>, BareSelectionFailure> {
    let identities = ReadIdentityRows::read(&files["identities.json"], package).expect("rows");
    let decide = node_key(package, "Pipeline", "Decide");
    select(package, &identities, None, decide, member, Some(index))
        .map(|slot| slot.origin().map(str::to_owned))
}

/// FR-533-AC-12 (numeric half): origins renamed to `999..1005`,
/// `1000..1004` and `1001..1002` are taken in numeric `(start, end)` order,
/// so the visible entry takes `999..1005`; the bytes order would give it
/// `1000..1004`.
#[trace("FR-533-AC-12", "TC-1333")]
#[test]
fn renamed_origins_are_taken_in_numeric_order_not_byte_order() {
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let original = read(&request, &files["checked-package.json"]);
    let [first, second, third] = decision_origins(&original)
        .try_into()
        .expect("three origins");
    let renamed = [
        (first, "999..1005"),
        (second, "1000..1004"),
        (third, "1001..1002"),
    ];
    let package = edited(&request, &files, |rows| {
        for row in rows {
            let name = origin_name(row).to_owned();
            if let Some((_, span)) = renamed.iter().find(|(old, _)| *old == name) {
                row["location"]["origin"]["name"] = format!("{DECISION}{span}").into();
            }
        }
    });
    let want = |span: &str| Some(format!("{DECISION}{span}"));
    let origin = |member, index| decide_origin(&package, &files, member, index).expect("selects");
    assert_eq!(origin("visible", 0), want("999..1005"));
    assert_eq!(origin("cases.guard", 1), want("1000..1004"));
    assert_eq!(origin("cases.guard", 0), want("1001..1002"));
}

/// FR-533-AC-14 (missing candidate half): with the first origin's rows
/// removed, `visible[0]` (rank 0, one candidate) takes the second origin and
/// `cases.guard[1]` (rank 1, none) refuses `MissingLocationProjection` at the
/// `Node` locus of the `got.ready` application.
#[trace("FR-533-AC-14", "TC-1333")]
#[test]
fn a_slot_whose_rank_exceeds_its_candidates_refuses_missing_location_projection() {
    use qsl_package::UnsupportedReason;
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let original = read(&request, &files["checked-package.json"]);
    let [first, second, _] = decision_origins(&original)
        .try_into()
        .expect("three origins");
    let package = edited(&request, &files, |rows| {
        rows.retain(|row| origin_name(row) != first);
    });
    assert_eq!(
        decide_origin(&package, &files, "visible", 0).expect("selects"),
        Some(second)
    );
    let Err(BareSelectionFailure::Unsupported(view)) =
        decide_origin(&package, &files, "cases.guard", 1)
    else {
        panic!("the slot has no origin");
    };
    // Selection takes no meter and charges nothing; the refusal is a Node
    // locus at the application, and no slot (no partial view) is returned.
    let UnsupportedReason::MissingLocationProjection(missing) = view.reason() else {
        panic!("not a missing location projection");
    };
    assert_eq!(view.locus().key, missing.application);
    assert_eq!((missing.rows, missing.index), (0, 0));
}

/// FR-390-AC-65 (selection level): a slot is named by (control node, member,
/// index) alone: equal triples select the same root, distinct indexes of one
/// member select distinct roots, and an index the control does not hold
/// refuses.
#[trace("FR-390-AC-65", "TC-1331")]
#[test]
fn a_slot_is_named_by_control_node_member_and_index() {
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let package = read(&request, &files["checked-package.json"]);
    let identities = ReadIdentityRows::read(&files["identities.json"], &package).expect("rows");
    let decide = node_key(&package, "Pipeline", "Decide");
    let root = |member: &str, index| {
        select(
            &package,
            &identities,
            None,
            decide.clone(),
            member,
            Some(index),
        )
        .map(|slot| slot.root().clone())
    };
    let guard0 = root("cases.guard", 0).expect("selects");
    let guard1 = root("cases.guard", 1).expect("selects");
    assert_eq!(root("cases.guard", 0).expect("selects"), guard0);
    assert_ne!(guard0, guard1);
    assert_ne!(root("visible", 0).expect("selects"), guard0);
    assert!(matches!(
        root("cases.guard", 2),
        Err(BareSelectionFailure::Slot(_))
    ));
}

/// FR-390-AC-67 (selection level): a slot with both a `Node` refusal (a
/// binder type the identity rows cannot project) and an equality-witness
/// refusal reports the equality one, `Package` before `Node`; the witness
/// refusals are Missing for no rows, Foreign for another package's rows and
/// Unknown for rows that join nothing. The `Package` locus half of AC-66 and
/// AC-67 waits for `CallAdmissionLocus::Package` (quire-eval).
#[trace("FR-390-AC-67", "TC-1331")]
#[test]
fn an_equality_witness_refusal_is_reported_before_a_node_refusal() {
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let package = read(&request, &files["checked-package.json"]);
    let rows = ReadEqualityRows::read(&files["equality.json"], &package).expect("rows");
    let mut stripped: Value = serde_json::from_slice(&files["identities.json"]).expect("JSON");
    stripped["fields"] = Value::Array(Vec::new());
    let identities =
        ReadIdentityRows::read(&serde_json::to_vec(&stripped).expect("bytes"), &package)
            .expect("fewer rows still read");
    let refund = node_key(&package, "Refunds", "Refund");
    let relation = |equality: Option<&ReadEqualityRows>| {
        select(
            &package,
            &identities,
            equality,
            refund.clone(),
            "retry.relation",
            None,
        )
        .map(|_| ())
        .map_err(|failure| match failure {
            BareSelectionFailure::Equality(refusal) => format!("equality {refusal:?}"),
            BareSelectionFailure::Unsupported(view) => format!("node {}", view.reason().name()),
            other => format!("{other:?}"),
        })
    };
    assert_eq!(
        relation(Some(&rows)),
        Err("node missing-type-representation".to_owned()),
        "with the witness the node refusal is what remains"
    );
    assert_eq!(relation(None), Err("equality Missing".to_owned()));
    let other = request_other_package();
    assert_eq!(relation(Some(&other)), Err("equality Foreign".to_owned()));
    let empty = ReadEqualityRows::read(&empty_file(), &package).expect("empty rows read");
    assert_eq!(relation(Some(&empty)), Err("equality Unknown".to_owned()));
}
