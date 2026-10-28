// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230-AC-49/AC-50 (TC-685 step 33), QSpec FR-322-AC-148..150: a dense
//! model's clock places reach S4 as template `clock_place` nodes, one for
//! each path, whose slot steps carry the symbolic element selector.

use super::admission_corpus::config_version_unit;
use super::*;
use crate::checked_v2::{read_checked_package_v2, CheckedTimedModel};
use quire_contract_model::CheckedPackageEvidence;

/// ConfigVersion with the clock `x: Clock` and the clock collection `s:
/// Sequence<Clock>[0, 3]`.
fn clocked_document() -> Vec<u8> {
    clocked_document_with(true)
}

/// ConfigVersion with the clock `x: Clock`, and `s: Sequence<Clock>[0, 3]`
/// when `collection` holds.
fn clocked_document_with(collection: bool) -> Vec<u8> {
    let mut document: Value =
        serde_json::from_slice(config_version_unit::DOMAIN_PACKAGE.as_bytes())
            .expect("the fixture is JSON");
    let config = &mut document["types"][1];
    let template = config["fields"][0].clone();
    let field = |name: &str, lower: u64, upper: u64, ordered: bool, unique: bool| {
        let identity = format!("ix://example/config-version/ConfigVersion/{name}");
        let mut field = template.clone();
        field["identity"] = json!(identity);
        field["name"] = json!(name);
        field["typeRef"] = json!("ix://quire/native/Clock");
        field["multiplicity"] =
            json!({"lower": lower, "upper": upper, "ordered": ordered, "unique": unique});
        field["origin"]["generated"]["generatorIdentity"] = json!(identity);
        field["origin"]["generated"]["inputIdentities"] = json!([identity]);
        field
    };
    let fields = config["fields"].as_array_mut().expect("fields");
    fields.push(field("x", 1, 1, false, true));
    if collection {
        fields.push(field("s", 0, 3, true, false));
    }
    serde_json::to_vec(&document).expect("the fixture stays JSON")
}

/// The unit selecting [`clocked_document`] under a dense time source, with
/// `body` after it.
fn clocked_unit(packages: &BTreeMap<[u8; 32], Vec<u8>>, body: &str) -> String {
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let unit = config_version_unit::unit_text(&qsl_semantics::model::key::hex(digest)).replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n",
    );
    format!(
        "{unit}dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config dense unit ms;\n{body}"
    )
}

/// The member `name` of the aggregate body `node`.
fn member<'v>(node: &'v Value, name: &str) -> &'v Value {
    &node["members"]
        .as_array()
        .expect("an aggregate")
        .iter()
        .find(|member| member["name"] == name)
        .unwrap_or_else(|| panic!("member {name}"))["value"]
}

/// Each step of an aggregate path, its literal members by name and a
/// reference member as `ref:<digest>`.
fn steps(path: &Value) -> Vec<Vec<(String, String)>> {
    path["members"]
        .as_array()
        .expect("a path")
        .iter()
        .map(|step| {
            step["members"]
                .as_array()
                .expect("a step")
                .iter()
                .map(|member| {
                    let value = &member["value"];
                    let shown = match value["term"].as_str() {
                        Some("reference") => format!(
                            "ref:{}",
                            value["target"]["digest"].as_str().unwrap_or_default()
                        ),
                        _ => match &value["value"] {
                            Value::String(text) => text.clone(),
                            other => other.to_string(),
                        },
                    };
                    (
                        member["name"].as_str().unwrap_or_default().to_owned(),
                        shown,
                    )
                })
                .collect()
        })
        .collect()
}

/// FR-230-AC-49 (TC-685 step 33), QSpec FR-322-AC-148/149: `x: Clock` and
/// `s: Sequence<Clock>[0, 3]` emit one `clock_place` each. The place of `s`
/// is the template of every slot: its path is `field(s)`, `slot(sequence,
/// <Integer type node>, 0, 3)`, with the symbolic selector a reference to
/// the emitted Integer scalar node and no index. Each `resolved_path` row
/// names its declaring type, member and type, the collection row its kind
/// and bounds, the slot row `Sequence<ix://quire/native/Clock>[slot]`. With
/// no guard, each place's `guards` is the empty aggregate (FR-230-AC-50),
/// and the root lists both places.
#[trace("TC-685")]
#[test]
fn a_clock_collection_emits_one_template_place() {
    let document = clocked_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let graph = assemble_with_models(clocked_unit(&packages, "").as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("a model of clocks alone checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let wire = wire(&emission);
    let integer = qsl_semantics::check::builtin_integer_type_key().to_string();
    assert!(
        nodes(&wire)
            .iter()
            .any(|node| node["node_id"]["digest"] == integer
                && node["node_tag"] == "scalar_type"
                && node["semantic_form"] == "integer"),
        "the selector's Integer type node is in the graph"
    );
    let places: Vec<&Value> = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "clock_place")
        .collect();
    let path_of = |place: &Value| steps(member(&place["body"], "path"));
    let field = |name: &str| {
        vec![
            ("kind".to_owned(), "field".to_owned()),
            ("name".to_owned(), name.to_owned()),
        ]
    };
    let collection = places
        .iter()
        .copied()
        .find(|place| path_of(place).len() == 2)
        .expect("the place of s");
    assert_eq!(
        path_of(collection),
        [
            field("s"),
            vec![
                ("kind".to_owned(), "slot".to_owned()),
                ("collection".to_owned(), "sequence".to_owned()),
                ("selector".to_owned(), format!("ref:{integer}")),
                ("minimum".to_owned(), "0".to_owned()),
                ("maximum".to_owned(), "3".to_owned()),
            ],
        ]
    );
    let rows = steps(member(&collection["body"], "resolved_path"));
    let row = |values: [&str; 7]| {
        [
            "path_step",
            "declaring_type",
            "member_identity",
            "member_type",
            "collection_kind",
            "minimum",
            "maximum",
        ]
        .into_iter()
        .zip(values)
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect::<Vec<_>>()
    };
    assert_eq!(
        rows,
        [
            row([
                "0",
                "ix://example/config-version/ConfigVersion",
                "ix://example/config-version/ConfigVersion/s",
                "Sequence<ix://quire/native/Clock>",
                "sequence",
                "0",
                "3",
            ]),
            row([
                "1",
                "ix://quire/native/Clock",
                "Sequence<ix://quire/native/Clock>[slot]",
                "ix://quire/native/Clock",
                "none",
                "",
                "",
            ]),
        ]
    );
    let single = places
        .iter()
        .copied()
        .find(|place| path_of(place).len() == 1)
        .expect("the place of x");
    assert_eq!(path_of(single), [field("x")]);
    for place in &places {
        assert_eq!(member(&place["body"], "guards")["members"], json!([]));
    }
    let [root] = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "timed_model")
        .collect::<Vec<_>>()[..]
    else {
        panic!("one emitted timed root");
    };
    let mut listed: Vec<&Value> = member(&root["body"], "clock_places")["members"]
        .as_array()
        .expect("clock_places")
        .iter()
        .map(|reference| &reference["target"]["digest"])
        .collect();
    let mut emitted: Vec<&Value> = places
        .iter()
        .map(|place| &place["node_id"]["digest"])
        .collect();
    listed.sort_by_key(|digest| digest.to_string());
    emitted.sort_by_key(|digest| digest.to_string());
    assert_eq!(listed, emitted);
    assert_eq!(listed.len(), 2);
}

/// FR-230-AC-50 (TC-685 step 33): a model with a guard is refused
/// `TimedModelNotEmitted` until its guard is written, never emitted with an
/// empty `guards` aggregate.
#[trace("TC-685")]
#[test]
fn a_clock_model_with_a_guard_is_not_emitted() {
    let document = clocked_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let unit = clocked_unit(
        &packages,
        "pre Late using v on Config::ConfigVersion::attemptUpdate { self.x >= 1 ms }\n",
    );
    let refusals = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect_err("a guard is not written yet");
    assert!(
        refusals.iter().any(|refusal| matches!(
            &refusal.cause,
            qsl_semantics::check::CheckCause::Temporal(cause)
                if matches!(**cause, qsl_semantics::check::TemporalCause::TimedModelNotEmitted { .. })
        )),
        "{refusals:?}"
    );
}

/// The emitted package over `document` with no further unit text, read back
/// through I2 into its typed timed models.
fn read_back(document: &[u8]) -> Result<Vec<CheckedTimedModel>, String> {
    let packages = qsl_semantics::model::intake::package_input([document]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let graph = assemble_with_models(clocked_unit(&packages, "").as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("a model of clocks alone checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(qsl_semantics::model::key::hex(digest), document.to_vec())
        .expect("a fresh evidence key");
    let read = read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id()),
    )
    .map_err(|failure| format!("{failure:?}"))?;
    CheckedTimedModel::from_admitted(&read.into_value().admitted).map_err(str::to_owned)
}

/// QSpec FR-322 "Timed-model nodes" read-back: the template place of `x:
/// Clock`, with its `resolved_path` row and empty `guards`, is admitted by
/// I2 and reconstructed.
#[trace("TC-685")]
#[test]
fn a_clock_place_reads_back() {
    let typed = read_back(&clocked_document_with(false))
        .unwrap_or_else(|failure| panic!("the I2 read admits the place: {failure}"));
    let [timed] = typed.as_slice() else {
        panic!("one timed model reads back");
    };
    let [place] = timed.clock_places.as_slice() else {
        panic!("one place reads back");
    };
    assert_eq!(
        place.path,
        [crate::checked_v2::ClockPathStep::Field("x".to_owned())]
    );
    assert!(place.guards.is_empty());
}

/// QSpec FR-322-AC-148 read-back of a template slot step: the place of `s:
/// Sequence<Clock>[0, 3]`, whose first row's member type is the rendered
/// collection `Sequence<ix://quire/native/Clock>`.
#[trace("TC-685")]
#[test]
#[ignore = "IR-1083 (validate symbolic timed slots, SL-01..05): the pinned IR renders a collection field row without its collection and refuses malformed-clock-place"]
fn a_template_slot_place_reads_back() {
    let typed = read_back(&clocked_document())
        .unwrap_or_else(|failure| panic!("the I2 read admits the template places: {failure}"));
    let [timed] = typed.as_slice() else {
        panic!("one timed model reads back");
    };
    assert_eq!(timed.clock_places.len(), 2);
    assert!(timed
        .clock_places
        .iter()
        .all(|place| place.guards.is_empty()));
}

/// ConfigVersion with the record `Timer { x: Clock; n: VersionNumber }`, the
/// package sequence type `Ticks` of `Clock`, and the fields of QSpec's
/// timed-slot-selector vectors: `ts: Sequence<Timer>[0, 3]` (SL-01), `g:
/// Sequence<Ticks>` unbounded, the `Sequence<Sequence<Clock>>` of SL-05,
/// and `s: Sequence<Clock>` unbounded (SL-08).
fn vector_document() -> Vec<u8> {
    const PREFIX: &str = "ix://example/config-version";
    let mut document: Value =
        serde_json::from_slice(config_version_unit::DOMAIN_PACKAGE.as_bytes())
            .expect("the fixture is JSON");
    let generated = |identity: &str| {
        json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity]}})
    };
    let field = |owner: &str, name: &str, type_ref: &str, multiplicity: Value| {
        let identity = format!("{PREFIX}/{owner}/{name}");
        json!({"identity": identity, "name": name, "typeRef": type_ref,
            "presence": "required", "nullable": false, "defaultKind": "none",
            "multiplicity": multiplicity, "origin": generated(&identity)})
    };
    let single = json!({"lower": 1, "upper": 1, "ordered": false, "unique": false});
    let declared_type = |name: &str, kind: Value, extra: Value| {
        let identity = format!("{PREFIX}/{name}");
        let mut node = json!({"identity": identity, "displayName": identity, "kind": kind,
            "roles": [], "origin": generated(&identity), "constraints": [], "extensions": [],
            "unknownPolicy": "reject"});
        for (key, value) in extra.as_object().expect("members") {
            node[key] = value.clone();
        }
        node
    };
    let constructs = document["constructs"].as_array_mut().expect("constructs");
    let mut record = constructs[0].clone();
    record["kind"]["name"] = json!("value_record");
    record["construct"]["meaning"] =
        json!(qsl_semantics::model::intake::meaning::RECORD_VALUE_TYPE);
    constructs.push(record);
    let types = document["types"].as_array_mut().expect("types");
    types.push(declared_type(
        "Timer",
        json!({"module": "example/config-version", "name": "value_record"}),
        json!({"fields": [
            field("Timer", "x", "ix://quire/native/Clock", single.clone()),
            field("Timer", "n", &format!("{PREFIX}/VersionNumber"),
                json!({"lower": 1, "upper": 1, "ordered": false, "unique": true})),
        ]}),
    ));
    types.push(declared_type(
        "Ticks",
        json!("sequence"),
        json!({"items": "ix://quire/native/Clock"}),
    ));
    let fields = types[1]["fields"].as_array_mut().expect("fields");
    let unbounded = json!({"lower": 0, "ordered": true, "unique": false});
    fields.push(field(
        "ConfigVersion",
        "ts",
        &format!("{PREFIX}/Timer"),
        json!({"lower": 0, "upper": 3, "ordered": true, "unique": false}),
    ));
    fields.push(field(
        "ConfigVersion",
        "g",
        &format!("{PREFIX}/Ticks"),
        unbounded.clone(),
    ));
    fields.push(field(
        "ConfigVersion",
        "s",
        "ix://quire/native/Clock",
        unbounded,
    ));
    serde_json::to_vec(&document).expect("the fixture stays JSON")
}

/// QSpec FR-322-AC-148/149 conformance (TC-685 step 33): the emitted
/// template places of `ts`, `g` and `s` equal the `node` of the published
/// timed-slot-selector vectors SL-01, SL-05 and SL-08 (body and
/// dependencies), once the vectors' `acme/orders` model node digest and
/// type identities are read as this package's. Reads the vectors from
/// `$QSPEC_DIR` (skipped, and passing, when it is unset; `make
/// conformance` requires it); nothing of QSpec is copied here.
#[trace("TC-685")]
#[test]
fn conformance_the_template_places_match_the_slot_selector_vectors() {
    let Some(qspec) = std::env::var_os("QSPEC_DIR") else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let path = std::path::Path::new(&qspec)
        .join("proposals/checked-package-v2/timed-slot-selector-vectors.json");
    let vectors: Value = serde_json::from_slice(
        &std::fs::read(&path).unwrap_or_else(|failure| panic!("{}: {failure}", path.display())),
    )
    .expect("the vectors are JSON");
    let document = vector_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let graph = assemble_with_models(clocked_unit(&packages, "").as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("a model of clocks alone checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let wire = wire(&emission);
    let place = |name: &str| {
        nodes(&wire)
            .iter()
            .find(|node| {
                node["semantic_form"] == "clock_place"
                    && steps(member(&node["body"], "path"))[0][1].1 == name
            })
            .unwrap_or_else(|| panic!("the place of {name}"))
            .clone()
    };
    let vector_model = vectors["type_nodes"]["model"]
        .as_str()
        .expect("the model digest");
    for (case, name) in [("SL-01", "ts"), ("SL-05", "g"), ("SL-08", "s")] {
        let emitted = place(name);
        let model = member(&emitted["body"], "model")["target"]["digest"]
            .as_str()
            .expect("a model reference")
            .to_owned();
        let published = vectors["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|vector| vector["id"] == case)
            .unwrap_or_else(|| panic!("{case} is published"));
        let read = serde_json::to_string(&published["node"])
            .expect("JSON")
            .replace(vector_model, &model)
            .replace(
                "ix://acme/orders/Order",
                "ix://example/config-version/ConfigVersion",
            )
            .replace("ix://acme/orders/", "ix://example/config-version/");
        let published: Value = serde_json::from_str(&read).expect("JSON");
        assert_eq!(emitted["body"], published["body"], "{case}");
        let digests = |node: &Value| {
            let mut digests: Vec<String> = node["dependencies"]
                .as_array()
                .expect("dependencies")
                .iter()
                .map(|dependency| dependency["digest"].as_str().unwrap_or_default().to_owned())
                .collect();
            digests.sort();
            digests
        };
        assert_eq!(digests(&emitted), digests(&published), "{case}");
    }
}
