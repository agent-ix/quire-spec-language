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
pub(super) mod tc1070;

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
        native_join: qsl_package::V2ReadLimits::fixture().native_join_limits(),
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
        .with_supply(
            BareSupply {
                identities,
                equality,
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply")
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
    let identities = ReadIdentityRows::read(
        &files["identities.json"],
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("rows");
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
    let identities = ReadIdentityRows::read(
        &files["identities.json"],
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("rows");
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
    let identities = ReadIdentityRows::read(
        &files["identities.json"],
        package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("rows");
    let decide = node_key(package, "Pipeline", "Decide");
    select(package, &identities, None, decide, member, Some(index))
        .map(|slot| slot.origin().map(str::to_owned))
}

const CASE: &str = "protocol Pipeline run Main Decide case ";

/// The refusal of the projection of the slot `member[index]` of `Pipeline`'s
/// `control`, which is where a missing `protocol_locations` row under the
/// slot's origin is found (FR-533-AC-14); `None` when it projects.
fn projection_refusal(
    package: &CheckedPackageV2,
    files: &BTreeMap<String, Vec<u8>>,
    control: &str,
    member: &str,
    index: usize,
) -> Option<BareSelectionFailure> {
    let identities = ReadIdentityRows::read(
        &files["identities.json"],
        package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("rows");
    let bare = BarePackage::new(package)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: None,
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let slot = bare
        .select_protocol_slot(&ProtocolSlotDescriptor::new(
            node_key(package, "Pipeline", control),
            member.to_owned(),
            Some(index),
        ))
        .expect("the slot selects");
    bare.project_protocol_slot(&slot).err()
}

/// FR-533-AC-11, FR-383-AC-63: a slot is selected by (control node, member,
/// index) alone, and only a `visible` entry takes a span-named origin; each
/// case guard takes the origin its label names, and no row is under any
/// other `decision` origin.
#[trace("FR-533-AC-11", "FR-383-AC-63", "TC-1333", "TC-1063")]
#[test]
fn fixture_l_slots_take_their_own_origins() {
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let package = read(&request, &files["checked-package.json"]);
    let [visible] = decision_origins(&package).try_into().expect("one origin");
    assert_eq!(visible, format!("{DECISION}2641..2650"));
    assert_eq!(
        decide_origin(&package, &files, "visible", 0).expect("selects"),
        Some(visible)
    );
    assert_eq!(
        decide_origin(&package, &files, "cases.guard", 0).expect("selects"),
        Some(format!("{CASE}no guard"))
    );
    assert_eq!(
        decide_origin(&package, &files, "cases.guard", 1).expect("selects"),
        Some(format!("{CASE}yes guard"))
    );
}

/// FR-533-AC-12 (numeric half): with the origins `999..1005` and
/// `1000..1004` added, each holding a copy of the `visible` entry's rows
/// numbered on after the group's existing `term_ordinal` values, the visible
/// entry takes `999..1005` (numeric `(start, end)` order; the bytes order
/// would give it `1000..1004`) and the case guards keep their `case` origins.
#[trace("FR-533-AC-12", "TC-1333")]
#[test]
fn added_origins_are_taken_in_numeric_order_not_byte_order() {
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let original = read(&request, &files["checked-package.json"]);
    let [visible] = decision_origins(&original).try_into().expect("one origin");
    let package = edited(&request, &files, |rows| {
        // The rows of one (application, occurrence) group are consecutive and
        // ascending by `term_ordinal`: each copy goes after its group's last
        // row, numbered on after the existing values.
        let group = |row: &Value| format!("{}{}", row["application"], row["occurrence"]);
        let copies: Vec<Value> = rows
            .iter()
            .filter(|row| origin_name(row) == visible)
            .cloned()
            .collect();
        let mut ordered = Vec::with_capacity(rows.len());
        let mut held = std::mem::take(rows).into_iter().peekable();
        while let Some(row) = held.next() {
            let key = group(&row);
            let mut ordinal = row["term_ordinal"].as_u64().expect("ordinal") + 1;
            ordered.push(row);
            while held.peek().is_some_and(|next| group(next) == key) {
                let next = held.next().expect("peeked");
                ordinal = next["term_ordinal"].as_u64().expect("ordinal") + 1;
                ordered.push(next);
            }
            for span in ["999..1005", "1000..1004"] {
                for copy in copies.iter().filter(|copy| group(copy) == key) {
                    let mut row = copy.clone();
                    row["location"]["origin"]["name"] = format!("{DECISION}{span}").into();
                    row["term_ordinal"] = ordinal.into();
                    ordinal += 1;
                    ordered.push(row);
                }
            }
        }
        *rows = ordered;
    });
    let origin = |member, index| decide_origin(&package, &files, member, index).expect("selects");
    assert_eq!(origin("visible", 0), Some(format!("{DECISION}999..1005")));
    assert_eq!(origin("cases.guard", 0), Some(format!("{CASE}no guard")));
    assert_eq!(origin("cases.guard", 1), Some(format!("{CASE}yes guard")));
}

fn binding_mut<'a>(group: &'a mut Value, name: &str) -> &'a mut Value {
    &mut group["members"]
        .as_array_mut()
        .expect("group members")
        .iter_mut()
        .find(|binding| binding["name"] == name)
        .expect("named binding")["value"]
}

/// FR-533-AC-14, FR-383-AC-63: a genuine reference-valued call makes the two
/// related ends non-leaf walks; they are named by their record's position, not
/// by a span, and the bytes route builds the same names.
#[trace("TC-1333", "FR-533-AC-14", "FR-383-AC-63")]
#[test]
fn related_calls_are_named_by_record_position() {
    use qsl_eval::value::{
        CheckedPackageEvaluation, ProtocolSlot, ProtocolSlotEvaluation, ProtocolSupply,
        SuppliedValue,
    };
    let source = tc1070::SOURCE
        .replace("profile S = \"quire.state.graph/v1\";", "profile S = \"quire.state.graph/v1\";\nprofile v = \"quire.value.complete/v1\";")
        .replace("model M = \"test/orders\";", "model M = \"test/orders\";\nfunction identityRef using v(x: M::Node): M::Node pure { x }")
        .replace("related by Owns (view, view)", "related by Owns (identityRef(view), identityRef(view))");
    assert_ne!(source, tc1070::SOURCE);
    let (_holder, request, files) = handoff(&source);
    let original = read(&request, &files["checked-package.json"]);
    const RELATED: &str = "protocol Pipeline run Main Got related ";
    let mut origins: Vec<_> = original
        .checked_term_data()
        .expect("term data")
        .protocol_locations
        .iter()
        .filter_map(|row| match &row.location.origin {
            quire_contract_model::CheckedProtocolLocationOrigin::TypeDeclaration { name }
                if name.starts_with(RELATED) =>
            {
                Some(name.to_string())
            }
            _ => None,
        })
        .collect();
    origins.sort();
    origins.dedup();
    assert_eq!(
        origins,
        [format!("{RELATED}0 from"), format!("{RELATED}0 to")],
        "both authentic related ends have an application walk, under their positions"
    );
    let package = original;
    let identities = ReadIdentityRows::read(
        &files["identities.json"],
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("identities");
    let bare = BarePackage::new(&package)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: None,
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let mut roots = Vec::new();
    let mut charges = Vec::new();
    for (member, side) in [("related.from", "from"), ("related.to", "to")] {
        let slot = ProtocolSlot {
            node: node_key(&package, "Pipeline", "Got"),
            member: member.into(),
            index: Some(0),
        };
        let selected = bare
            .select_protocol_slot(&ProtocolSlotDescriptor::new(
                slot.node.clone(),
                slot.member.clone(),
                slot.index,
            ))
            .expect("the related call selects");
        assert_eq!(
            selected.origin(),
            Some(format!("{RELATED}0 {side}").as_str())
        );
        roots.push(selected.root().clone());
        let supply = ProtocolSupply {
            binders: selected
                .sources()
                .iter()
                .map(|source| {
                    (
                        source.binder().clone(),
                        SuppliedValue::Unavailable(
                            quire_semantic_value::catalog::CatalogCode::new(
                                "unavailable_observation",
                                "missing-required-artifact",
                            ),
                        )
                        .into(),
                    )
                })
                .collect(),
            observation: None,
        };
        let mut meter = quire_exact::Meter::new(UNLIMITED);
        let result = bare.evaluate_protocol_slot(&slot, &supply, &mut meter);
        assert!(
            matches!(result, Ok(ProtocolSlotEvaluation::Unavailable { .. })),
            "the real call reads the unavailable reference: {result:?}"
        );
        charges.push(meter.admitted_charges().to_vec());
    }
    assert_eq!(
        roots[0], roots[1],
        "the producer emits one shared call root"
    );
    assert_eq!(charges[0], charges[1]);

    // A copy whose rows of `related 0 to` are removed refuses at the call
    // for `related.to[0]` and leaves `related.from[0]` selectable.
    let without_to = edited(&request, &files, |rows| {
        rows.retain(|row| origin_name(row) != format!("{RELATED}0 to"));
    });
    assert!(projection_refusal(&without_to, &files, "Got", "related.from", 0).is_none());
    let Some(BareSelectionFailure::Unsupported(view)) =
        projection_refusal(&without_to, &files, "Got", "related.to", 0)
    else {
        panic!("the end has no rows");
    };
    assert!(matches!(
        view.reason(),
        qsl_package::UnsupportedReason::MissingLocationProjection(_)
    ));
}

/// FR-533-AC-13 (related half): two records of one relationship whose `from`
/// ends are one call application and whose `to` ends differ each pair with
/// their own end, `related.from[0]` under `related 0 from` and
/// `related.from[1]` under `related 1 from`, which locate the walk at
/// distinct origins.
#[trace("TC-1333", "FR-533-AC-13", "FR-383-AC-63")]
#[test]
fn equal_related_ends_of_distinct_records_take_their_own_origin() {
    let source = tc1070::SOURCE
        .replace("profile S = \"quire.state.graph/v1\";", "profile S = \"quire.state.graph/v1\";\nprofile v = \"quire.value.complete/v1\";")
        .replace("model M = \"test/orders\";", "model M = \"test/orders\";\nfunction identityRef using v(x: M::Node): M::Node pure { x }")
        .replace("protocol Pipeline using P over (view: M::Node) on origin {", "protocol Pipeline using P over (view: M::Node) on origin {\n capture peer: M::Node = view;")
        .replace("related by Owns (view, view)", "related by Owns (identityRef(view), peer) related by Owns (identityRef(view), view)");
    assert_ne!(source, tc1070::SOURCE);
    let (_holder, request, files) = handoff(&source);
    let package = read(&request, &files["checked-package.json"]);
    let identities = ReadIdentityRows::read(
        &files["identities.json"],
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("identities");
    let bare = BarePackage::new(&package)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: None,
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let mut locations = Vec::new();
    for index in 0..2 {
        let slot = bare
            .select_protocol_slot(&ProtocolSlotDescriptor::new(
                node_key(&package, "Pipeline", "Got"),
                "related.from".to_owned(),
                Some(index),
            ))
            .expect("selects");
        let name = format!("protocol Pipeline run Main Got related {index} from");
        assert_eq!(slot.origin(), Some(name.as_str()));
        let projection = bare.project_protocol_slot(&slot).expect("projects");
        let location = projection
            .location(projection.root())
            .expect("the call is located")
            .clone();
        assert_eq!(
            location.origin,
            quire_semantic_value::location::Origin::TypeDeclaration { name }
        );
        locations.push(location);
    }
    assert_ne!(locations[0], locations[1]);
}

/// Two genuine case slots share a walk only in this admitted package copy:
/// the source checker correctly refuses identical exhaustive guards. Each
/// takes the origin its own case's label names, in either label order, and
/// reads the `Location` of its own origin's rows.
#[trace("TC-1333", "FR-533-AC-13")]
#[test]
fn tied_case_guards_take_their_own_origin_in_either_label_order() {
    use qsl_eval::value::{CheckedPackageEvaluation, ProtocolSlot, ProtocolSlotEvaluation};
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let original = read(&request, &files["checked-package.json"]);
    assert_eq!(decision_origins(&original).len(), 1);
    for labels_follow_source in [false, true] {
        let mut wire: Value =
            serde_json::from_slice(&files["checked-package.json"]).expect("package");
        let nodes = wire["semantic_graph"]["nodes"]
            .as_array_mut()
            .expect("nodes");
        let decide = nodes
            .iter_mut()
            .find(|node| {
                node["declaration"]["qualified_name"]
                    == serde_json::json!(["Pipeline", "run", "Main", "Decide"])
            })
            .expect("choice");
        let cases = &mut decide["body"]["arguments"]
            .as_array_mut()
            .expect("arguments")
            .iter_mut()
            .find(|binding| binding["name"] == "cases")
            .expect("cases")["value"]["members"];
        let cases = cases.as_array_mut().expect("case groups");
        let root = binding_mut(&mut cases[1], "guard")["target"].clone();
        let removed = binding_mut(&mut cases[0], "guard")["target"].clone();
        binding_mut(&mut cases[0], "guard")["target"] = root;
        if labels_follow_source {
            binding_mut(&mut cases[0], "label")["value"] = "b".into();
            binding_mut(&mut cases[1], "label")["value"] = "a".into();
            cases.swap(0, 1);
        }
        let body_order: Vec<_> = cases
            .iter_mut()
            .map(|case| {
                binding_mut(case, "body")["target"]["path"]
                    .as_array()
                    .expect("body path")
                    .last()
                    .expect("body name")
                    .clone()
            })
            .collect();
        assert_eq!(
            body_order,
            if labels_follow_source {
                serde_json::json!(["A", "B"])
            } else {
                serde_json::json!(["B", "A"])
            }
            .as_array()
            .expect("body names")
            .clone()
        );
        decide["dependencies"]
            .as_array_mut()
            .expect("dependencies")
            .retain(|id| *id != removed);
        // Keep the authentic emitted negation node: the control no longer
        // selects it, and the selected walk's rows name only its new root.
        let rows = wire["checked_term_data"]["protocol_locations"]
            .as_array_mut()
            .expect("locations");
        rows.retain(|row| row["application"] != removed);
        let labels = if labels_follow_source {
            ["b", "a"]
        } else {
            ["no", "yes"]
        };
        for row in rows
            .iter_mut()
            .filter(|row| origin_name(row).starts_with(CASE))
        {
            row["location"]["path"] = serde_json::json!([]);
            let renamed = if origin_name(row) == format!("{CASE}no guard") {
                labels[0]
            } else {
                labels[1]
            };
            row["location"]["origin"]["name"] = format!("{CASE}{renamed} guard").into();
        }
        let package = read(&request, &serde_json::to_vec(&wire).expect("bytes"));
        let identities = ReadIdentityRows::read(
            &files["identities.json"],
            &package,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("identities");
        let bare = BarePackage::new(&package)
            .with_supply(
                BareSupply {
                    identities: &identities,
                    equality: None,
                },
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("the fixture budget admits the supply");
        let world = bare.world().expect("world");
        let got = record(
            world.types(),
            "Flag",
            vec![("ready", quire_exact::Value::Boolean(true))],
        );
        let mut charges = Vec::new();
        let expected = if labels_follow_source {
            ["a", "b"]
        } else {
            ["no", "yes"]
        };
        let mut locations = Vec::new();
        for (index, label) in expected.into_iter().enumerate() {
            let slot = ProtocolSlot {
                node: node_key(&package, "Pipeline", "Decide"),
                member: "cases.guard".into(),
                index: Some(index),
            };
            let selected = bare
                .select_protocol_slot(&ProtocolSlotDescriptor::new(
                    slot.node.clone(),
                    slot.member.clone(),
                    slot.index,
                ))
                .expect("tied slot selects");
            assert_eq!(
                selected.origin(),
                Some(format!("{CASE}{label} guard").as_str())
            );
            let supply = supply_of(&selected, vec![("got", got.clone())]);
            let mut meter = quire_exact::Meter::new(UNLIMITED);
            let result = bare.evaluate_protocol_slot(&slot, &supply, &mut meter);
            let Ok(ProtocolSlotEvaluation::Evaluated(evaluation, binder_charges)) = &result else {
                panic!("evaluates: {result:?}");
            };
            assert_eq!(
                evaluation.location, None,
                "a completed evaluation has no stop location"
            );
            let projection = bare.project_protocol_slot(&selected).expect("projects");
            let expected_location = quire_semantic_value::location::Location::at(
                quire_semantic_value::location::Origin::TypeDeclaration {
                    name: format!("{CASE}{label} guard"),
                },
                &[],
            );
            assert_eq!(
                projection.location(projection.root()),
                Some(&expected_location),
                "the visit retains its own origin's paired location"
            );
            locations.push(expected_location);
            charges.push((
                format!("{binder_charges:?}"),
                meter.admitted_charges().to_vec(),
            ));
            assert!(is_boolean(&completed(result), true));
        }
        assert_ne!(locations[0], locations[1], "the two slots read two origins");
        assert_eq!(
            charges[0], charges[1],
            "origin pairing does not change ordered charges"
        );
    }
}

/// FR-533-AC-14 (missing rows half): with the rows of `case yes guard`
/// removed, `cases.guard[1]` refuses `MissingLocationProjection` at the
/// `Node` locus of the `got.ready` application and `cases.guard[0]` and
/// `visible[0]` are unaffected; with the rows of the `decision` origin
/// removed, `visible[0]` refuses the same.
#[trace("TC-1333", "FR-533-AC-14")]
#[test]
fn a_slot_without_rows_under_its_origin_refuses_missing_location_projection() {
    use qsl_package::UnsupportedReason;
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let original = read(&request, &files["checked-package.json"]);
    let [visible] = decision_origins(&original).try_into().expect("one origin");
    let package = edited(&request, &files, |rows| {
        rows.retain(|row| origin_name(row) != format!("{CASE}yes guard"));
    });
    assert_eq!(
        decide_origin(&package, &files, "visible", 0).expect("selects"),
        Some(visible)
    );
    assert_eq!(
        decide_origin(&package, &files, "cases.guard", 0).expect("selects"),
        Some(format!("{CASE}no guard"))
    );
    assert!(projection_refusal(&package, &files, "Decide", "cases.guard", 0).is_none());
    let Some(BareSelectionFailure::Unsupported(view)) =
        projection_refusal(&package, &files, "Decide", "cases.guard", 1)
    else {
        panic!("the slot has no rows under its origin");
    };
    // Selection takes no meter and charges nothing; the refusal is a Node
    // locus at the application, and no slot (no partial view) is returned.
    let UnsupportedReason::MissingLocationProjection(missing) = view.reason() else {
        panic!("not a missing location projection");
    };
    assert_eq!(view.locus().key(), Some(&missing.application));
    assert_eq!((missing.rows, missing.index), (0, 0));

    // The public call must propagate the same view refusal before supply
    // admission or the first scalar charge.
    use qsl_eval::value::{CheckedPackageEvaluation, ProtocolSlot, ProtocolSupply};
    let identities = ReadIdentityRows::read(
        &files["identities.json"],
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("rows");
    let bare = BarePackage::new(&package)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: None,
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let slot = ProtocolSlot {
        node: node_key(&package, "Pipeline", "Decide"),
        member: "cases.guard".into(),
        index: Some(1),
    };
    let mut meter = quire_exact::Meter::new(UNLIMITED);
    let result = bare.evaluate_protocol_slot(
        &slot,
        &ProtocolSupply {
            binders: Vec::new(),
            observation: None,
        },
        &mut meter,
    );
    let Err(quire_eval::CallFailure::Admission(refusal)) = result else {
        panic!("missing candidate refuses at the call: {result:?}");
    };
    let quire_eval::CallAdmissionFailure::Refused(refused) = *refusal else {
        panic!("a projection refusal");
    };
    assert_eq!(
        refused.locus,
        quire_eval::CallAdmissionLocus::Node(missing.application.clone())
    );
    assert_eq!(
        refused.record.fields.get("reason").map(String::as_str),
        Some("missing-location-projection")
    );
    for counter in quire_exact::LimitKind::ALL {
        assert_eq!(meter.consumed(counter), 0, "{counter:?}");
    }
    assert!(meter.admitted_charges().is_empty());

    let all_missing = edited(&request, &files, |rows| {
        rows.retain(|row| !origin_name(row).starts_with(DECISION));
    });
    let identities = ReadIdentityRows::read(
        &files["identities.json"],
        &all_missing,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("identities");
    let bare = BarePackage::new(&all_missing)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: None,
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    for (member, index) in [("visible", 0)] {
        let slot = ProtocolSlot {
            node: node_key(&all_missing, "Pipeline", "Decide"),
            member: member.into(),
            index: Some(index),
        };
        let mut untouched = quire_exact::Meter::new(UNLIMITED);
        let result = bare.evaluate_protocol_slot(
            &slot,
            &ProtocolSupply {
                binders: Vec::new(),
                observation: None,
            },
            &mut untouched,
        );
        let Err(quire_eval::CallFailure::Admission(failure)) = result else {
            panic!("all candidate rows missing refuses: {result:?}");
        };
        let quire_eval::CallAdmissionFailure::Refused(refused) = *failure else {
            panic!("a typed refusal");
        };
        assert_eq!(
            refused.locus,
            quire_eval::CallAdmissionLocus::Node(missing.application.clone())
        );
        assert_eq!(
            refused.record.fields.get("reason").map(String::as_str),
            Some("missing-location-projection")
        );
        for counter in quire_exact::LimitKind::ALL {
            assert_eq!(untouched.consumed(counter), 0, "{counter:?}");
        }
        assert!(untouched.admitted_charges().is_empty());
    }
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
    let identities = ReadIdentityRows::read(
        &files["identities.json"],
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("rows");
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
/// Unknown for rows that join nothing. The `Package` locus of the call
/// (FR-390-AC-66) is asserted by the evaluation tests of qsl-eval
/// (`bare_slots`).
#[trace("FR-390-AC-67", "TC-1331")]
#[test]
fn an_equality_witness_refusal_is_reported_before_a_node_refusal() {
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let package = read(&request, &files["checked-package.json"]);
    let rows = ReadEqualityRows::read(&files["equality.json"], &package).expect("rows");
    let mut stripped: Value = serde_json::from_slice(&files["identities.json"]).expect("JSON");
    stripped["fields"] = Value::Array(Vec::new());
    let identities = ReadIdentityRows::read(
        &serde_json::to_vec(&stripped).expect("bytes"),
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
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

/// Fixture E with a function whose body holds `==` called from `Refund`'s
/// trigger guard.
fn source_with_called_equality() -> String {
    tc1070::SOURCE
        .replace(
            "profile S = \"quire.state.graph/v1\";",
            "profile S = \"quire.state.graph/v1\";\nprofile v = \"quire.value.complete/v1\";",
        )
        .replace(
            "model M = \"test/orders\";",
            "model M = \"test/orders\";\nfunction isOne using v(x: Int[0, 9]): Boolean pure { x = 1 }",
        )
        .replace(
            "when { not trigger.ready and target >= 0 } {\n      capture activated",
            "when { not trigger.ready and target >= 0 and isOne(3) } {\n      capture activated",
        )
}

/// FR-531 "Refusals" (selection level): an equality application inside a
/// function the slot calls is part of the slot's walk. `trigger.guard` then
/// refuses without the rows (`Missing`) and selects with them, while the
/// `Partial` guard, which calls no such function, needs none. Closing the
/// witness check over the dependency closure (which stops at functions)
/// would not refuse the first case.
#[trace("TC-1331")]
#[test]
fn an_equality_inside_a_called_function_is_part_of_the_slots_walk() {
    let source = source_with_called_equality();
    assert_ne!(source, tc1070::SOURCE);
    let (_holder, request, files) = handoff(&source);
    let package = read(&request, &files["checked-package.json"]);
    let identities = ReadIdentityRows::read(
        &files["identities.json"],
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("rows");
    let rows = ReadEqualityRows::read(&files["equality.json"], &package).expect("rows");
    let guard = |name: &str, equality: Option<&ReadEqualityRows>| {
        select(
            &package,
            &identities,
            equality,
            node_key(&package, "Refunds", name),
            "trigger.guard",
            None,
        )
        .map(|_| ())
    };
    assert!(matches!(
        guard("Refund", None),
        Err(BareSelectionFailure::Equality(
            EqualityWitnessRefusal::Missing
        ))
    ));
    guard("Refund", Some(&rows)).expect("selects with the rows");
    guard("Partial", None).expect("a guard that calls no such function needs no rows");
}

const UNLIMITED: quire_exact::ScalarLimits = quire_exact::ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

/// A record of the package's own declarations, built field by field.
fn record(
    types: &quire_semantic_value::declaration::TypeEnvironment,
    name: &str,
    fields: Vec<(&str, quire_exact::Value)>,
) -> quire_exact::Value {
    let key = types
        .composites()
        .find(|record| record.name().ends_with(name))
        .expect("the package declares the record")
        .key();
    types
        .record(
            key,
            fields
                .into_iter()
                .map(|(field, value)| (field, quire_exact::FieldValue::Present(value)))
                .collect(),
        )
        .expect("the record is well formed")
}

fn is_boolean(value: &quire_exact::Value, expected: bool) -> bool {
    matches!(value, quire_exact::Value::Boolean(held) if *held == expected)
}

fn supply_of(
    slot: &qsl_package::BareSlot,
    values: Vec<(&str, quire_exact::Value)>,
) -> qsl_eval::value::ProtocolSupply<'static> {
    qsl_eval::value::ProtocolSupply {
        binders: values
            .into_iter()
            .map(|(name, value)| {
                let binder = slot
                    .sources()
                    .iter()
                    .find(|source| source.name() == name)
                    .unwrap_or_else(|| panic!("the slot reads {name}"))
                    .binder()
                    .clone();
                (binder, qsl_eval::value::SuppliedValue::Value(value).into())
            })
            .collect(),
        observation: None,
    }
}

fn completed(
    result: Result<qsl_eval::value::ProtocolSlotEvaluation, quire_eval::CallFailure>,
) -> quire_exact::Value {
    use quire_eval::FamilyOutcome;
    let Ok(qsl_eval::value::ProtocolSlotEvaluation::Evaluated(evaluation, charges)) = result else {
        panic!("the slot evaluates: {result:?}");
    };
    assert!(
        charges.binders().count() >= 1,
        "the per-binder charge log is carried"
    );
    let FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(value)) = evaluation.outcome
    else {
        panic!("a completed outcome: {:?}", evaluation.outcome);
    };
    value
}

/// FR-531-AC-14, FR-531-AC-16, FR-533-AC-16, FR-533-AC-3 (evaluate halves):
/// from the handoff directory's files alone (no `CheckedPackage`, no
/// `Emission` in this test), the integer equality of `Refund`'s retry
/// relation evaluates over the equality rows to the value its text states,
/// and `trigger.guard`, which holds no `==`, evaluates without them.
#[trace(
    "FR-531-AC-14",
    "FR-531-AC-16",
    "FR-533-AC-16",
    "FR-533-AC-3",
    "TC-1331",
    "TC-1333"
)]
#[test]
fn the_retry_relation_evaluates_from_the_handoff_files_alone() {
    use qsl_eval::value::{CheckedPackageEvaluation, ProtocolSlot};
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let package = read(&request, &files["checked-package.json"]);
    let identities = ReadIdentityRows::read(
        &files["identities.json"],
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("rows");
    let rows = ReadEqualityRows::read(&files["equality.json"], &package).expect("rows");
    let with_rows = BarePackage::new(&package)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: Some(&rows),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let without_rows = BarePackage::new(&package)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: None,
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let world = with_rows.world().expect("the package declares its records");
    let types = world.types();
    let refund = node_key(&package, "Refunds", "Refund");
    let relation = |earlier: i64, later: i64| {
        let slot = ProtocolSlot {
            node: refund.clone(),
            member: "retry.relation".to_owned(),
            index: None,
        };
        let selected = with_rows
            .select_protocol_slot(&ProtocolSlotDescriptor::new(
                slot.node.clone(),
                slot.member.clone(),
                slot.index,
            ))
            .expect("selects");
        let attempt = |tally: i64| {
            record(
                types,
                "Attempt",
                vec![("tally", quire_exact::Value::Integer(tally.into()))],
            )
        };
        let given = supply_of(
            &selected,
            vec![
                ("earlier", attempt(earlier)),
                ("later", attempt(later)),
                ("target", quire_exact::Value::Integer(5_i64.into())),
                ("activated", quire_exact::Value::Boolean(false)),
            ],
        );
        let mut meter = quire_exact::Meter::new(UNLIMITED);
        let result = with_rows.evaluate_protocol_slot(&slot, &given, &mut meter);
        (completed(result), meter, slot, given)
    };
    // earlier.tally < 3 and later.tally = earlier.tally + 1 and target >= 0
    // and not activated.
    let (held, meter, slot, given) = relation(1, 2);
    assert!(is_boolean(&held, true), "{held:?}");
    assert!(meter.consumed(quire_exact::LimitKind::WorkUnits) > 0);
    assert!(is_boolean(&relation(1, 3).0, false));
    assert!(is_boolean(&relation(3, 3).0, false));
    // The same call over a supply without the equality rows refuses before
    // any charge, at the package locus.
    let mut untouched = quire_exact::Meter::new(UNLIMITED);
    let Err(quire_eval::CallFailure::Admission(refusal)) =
        without_rows.evaluate_protocol_slot(&slot, &given, &mut untouched)
    else {
        panic!("the relation is refused without the equality rows");
    };
    let quire_eval::CallAdmissionFailure::Refused(refused) = *refusal else {
        panic!("a refusal");
    };
    assert_eq!(refused.locus, quire_eval::CallAdmissionLocus::Package);
    assert_eq!(refused.record.catalog.code(), "unknown_required_feature");
    assert_eq!(refused.record.catalog.cause(), "unsupported-feature");
    assert_eq!(
        refused.record.fields.get("reason").map(String::as_str),
        Some("missing-equality-witness")
    );
    for counter in quire_exact::LimitKind::ALL {
        assert_eq!(untouched.consumed(counter), 0, "{counter:?}");
    }
    assert!(untouched.admitted_charges().is_empty());
    // `trigger.guard` holds no `==`: it needs no rows.
    let guard = ProtocolSlot {
        node: refund,
        member: "trigger.guard".to_owned(),
        index: None,
    };
    let selected = without_rows
        .select_protocol_slot(&ProtocolSlotDescriptor::new(
            guard.node.clone(),
            guard.member.clone(),
            guard.index,
        ))
        .expect("selects");
    let flag = record(
        types,
        "Flag",
        vec![("ready", quire_exact::Value::Boolean(false))],
    );
    let given = supply_of(
        &selected,
        vec![
            ("trigger", flag),
            ("target", quire_exact::Value::Integer(5_i64.into())),
        ],
    );
    let mut meter = quire_exact::Meter::new(UNLIMITED);
    assert!(is_boolean(
        &completed(without_rows.evaluate_protocol_slot(&guard, &given, &mut meter)),
        true
    ));
}

/// FR-390-AC-67 (through the real walk): the slot walk of `Refund`'s
/// `recovery.predicate` reaches a `Node` refusal (the `recovered.total`
/// operand of its `=`, whose record type the identity rows cannot project)
/// before the `=` application that finds no equality decision. The walk
/// reports the equality-witness refusal at the `Package` locus, whatever
/// the walk reached first: Missing for no rows, Foreign for another
/// package's, Unknown for rows that join nothing; with the rows the `Node`
/// refusal is what remains.
#[trace("FR-390-AC-67", "FR-533-AC-15", "TC-1331", "TC-1333")]
#[test]
fn a_package_refusal_found_after_a_node_refusal_in_the_walk_is_reported_first() {
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let package = read(&request, &files["checked-package.json"]);
    let rows = ReadEqualityRows::read(&files["equality.json"], &package).expect("rows");
    let full = ReadIdentityRows::read(
        &files["identities.json"],
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("identities");
    let refund = node_key(&package, "Refunds", "Refund");
    let slot = select(
        &package,
        &full,
        Some(&rows),
        refund,
        "recovery.predicate",
        None,
    )
    .expect("selects with the whole supply");
    let mut stripped: Value = serde_json::from_slice(&files["identities.json"]).expect("JSON");
    stripped["fields"] = Value::Array(Vec::new());
    let identities = ReadIdentityRows::read(
        &serde_json::to_vec(&stripped).expect("bytes"),
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("fewer rows still read");
    let project = |equality: Option<&ReadEqualityRows>| {
        BarePackage::new(&package)
            .with_supply(
                BareSupply {
                    identities: &identities,
                    equality,
                },
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("the fixture budget admits the supply")
            .project_protocol_slot(&slot)
            .map(|_| ())
            .map_err(|failure| match failure {
                BareSelectionFailure::Equality(refusal) => format!("equality {refusal:?}"),
                BareSelectionFailure::Unsupported(view) => {
                    format!("node {}", view.reason().name())
                }
                other => format!("{other:?}"),
            })
    };
    assert_eq!(
        project(Some(&rows)),
        Err("node missing-type-representation".to_owned())
    );
    assert_eq!(project(None), Err("equality Missing".to_owned()));
    let other = request_other_package();
    assert_eq!(project(Some(&other)), Err("equality Foreign".to_owned()));
    let empty = ReadEqualityRows::read(&empty_file(), &package).expect("empty rows read");
    assert_eq!(project(Some(&empty)), Err("equality Unknown".to_owned()));
}

/// Repeated real equality-slot selection and projection borrow the decisions
/// retained by exact-budget bytes admission without charging another native read.
#[trace("TC-1331", "FR-531-AC-28")]
#[test]
fn equality_slots_reuse_the_decisions_retained_by_exact_budget_admission() {
    use quire_contract_model::NativeJoinBudget;
    let (_holder, request, files) = handoff(tc1070::SOURCE);
    let package = read(&request, &files["checked-package.json"]);
    let identities = ReadIdentityRows::read(
        &files["identities.json"],
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("original identities");
    let equality =
        ReadEqualityRows::read(&files["equality.json"], &package).expect("original equality");
    let supply = BareSupply {
        identities: &identities,
        equality: Some(&equality),
    };
    let mut measure = NativeJoinBudget::new(1 << 30, 1 << 30);
    BarePackage::new(&package)
        .with_supply(supply, &mut measure)
        .expect("measure complete entry");
    let need = measure.consumed();
    let mut exact = NativeJoinBudget::new(need.work, need.storage);
    let bare = BarePackage::new(&package)
        .with_supply(supply, &mut exact)
        .expect("exact budget admits");
    let descriptor = ProtocolSlotDescriptor::new(
        node_key(&package, "Refunds", "Refund"),
        "retry.relation",
        None,
    );
    let slot = bare
        .select_protocol_slot(&descriptor)
        .expect("the equality-bearing retry relation");
    let projection = bare
        .project_protocol_slot(&slot)
        .expect("real slot projection");
    fn equality_positions(projection: &qsl_package::ReaderProtocolProjection) -> Vec<usize> {
        let mut pending = vec![projection.root()];
        let mut positions = Vec::new();
        while let Some(index) = pending.pop() {
            if matches!(
                projection.kind(index),
                Some(quire_checked::node::NodeKind::Equality(..))
            ) {
                positions.push(index);
            }
            pending.extend(projection.children(index).expect("projected node children"));
        }
        positions
    }
    assert_eq!(
        projection.value_type(projection.root()),
        Some(&quire_exact::ValueType::Boolean)
    );
    let positions = equality_positions(&projection);
    let [position] = positions.as_slice() else {
        panic!("the authored retry conjunction has exactly one equality")
    };
    let original_location = projection
        .location(*position)
        .expect("the equality has its original protocol location")
        .clone();
    let quire_semantic_value::location::Origin::TypeDeclaration {
        name: original_origin,
    } = &original_location.origin
    else {
        panic!("authored protocol type-declaration origin")
    };
    let semantic_row = package.checked_term_data().expect("original native term data").protocol_locations.iter().find(|row| {
        matches!(&row.location.origin, quire_contract_model::CheckedProtocolLocationOrigin::TypeDeclaration { name } if name.as_ref() == original_origin)
            && row.location.path.iter().map(|ordinal| match ordinal {
                quire_contract_model::CheckedKeyOrdinal::Safe(value) => usize::try_from(*value).expect("small authored child index"),
                quire_contract_model::CheckedKeyOrdinal::BeyondSafe(_) => panic!("fixture child indices are small"),
            }).collect::<Vec<_>>() == original_location.path()
    }).expect("original protocol occurrence at the equality's authored origin and path");
    let row = equality
        .applications()
        .iter()
        .find(|row| row.application == semantic_row.application)
        .expect("that original protocol application has an authentic equality owner row");
    let decision = package
        .admitted_equality(
            &row.application,
            &row.occurrence,
            equality.applications(),
            equality.facts(),
            equality.source_units(),
            &mut NativeJoinBudget::new(1 << 30, 1 << 30),
        )
        .expect("independent original owner decision");
    assert_eq!(decision.location(), &row.location);
    let expected = Box::new(
        quire_semantic_value::declaration::CheckedEquality::from_admitted_decision(&decision),
    );
    let Some(quire_checked::node::NodeKind::Equality(operator, actual, _, _)) =
        projection.kind(*position)
    else {
        unreachable!("the equality position was found")
    };
    assert_eq!(
        *operator,
        quire_semantic_value::declaration::EqualityOperator::Equal
    );
    assert_eq!(actual, &expected);
    let world = bare.world().expect("original admitted record declarations");
    let protocol_slot = qsl_eval::value::ProtocolSlot {
        node: descriptor.node().clone(),
        member: "retry.relation".to_owned(),
        index: None,
    };
    for (later, expected_value) in [(2_i64, true), (3_i64, false)] {
        let selected = bare
            .select_protocol_slot(&descriptor)
            .expect("selection reuses admission");
        assert_eq!(selected.root(), slot.root());
        assert_eq!(selected.origin(), slot.origin());
        let repeated = bare
            .project_protocol_slot(&selected)
            .expect("projection reuses admission");
        assert_eq!(repeated.root(), projection.root());
        assert_eq!(
            repeated.value_type(repeated.root()),
            Some(&quire_exact::ValueType::Boolean)
        );
        let positions = equality_positions(&repeated);
        let [position] = positions.as_slice() else {
            panic!("still exactly the authored equality")
        };
        assert_eq!(repeated.location(*position), Some(&original_location));
        let Some(quire_checked::node::NodeKind::Equality(operator, actual, _, _)) =
            repeated.kind(*position)
        else {
            unreachable!("located equality")
        };
        assert_eq!(
            *operator,
            quire_semantic_value::declaration::EqualityOperator::Equal
        );
        assert_eq!(actual, &expected);
        let given = supply_of(
            &selected,
            vec![
                (
                    "earlier",
                    record(
                        world.types(),
                        "Attempt",
                        vec![("tally", quire_exact::Value::Integer(1_i64.into()))],
                    ),
                ),
                (
                    "later",
                    record(
                        world.types(),
                        "Attempt",
                        vec![("tally", quire_exact::Value::Integer(later.into()))],
                    ),
                ),
                ("target", quire_exact::Value::Integer(5_i64.into())),
                ("activated", quire_exact::Value::Boolean(false)),
            ],
        );
        let mut meter = quire_exact::Meter::new(UNLIMITED);
        let value = completed(
            qsl_eval::value::CheckedPackageEvaluation::evaluate_protocol_slot(
                &bare,
                &protocol_slot,
                &given,
                &mut meter,
            ),
        );
        assert!(
            is_boolean(&value, expected_value),
            "the authored equality changes the conjunction: {value:?}"
        );
        assert_eq!(exact.consumed(), need);
        assert_eq!(exact.first_unpaid(), None);
    }
}
