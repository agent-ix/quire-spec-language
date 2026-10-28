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
    fields.push(field("s", 0, 3, true, false));
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

/// QSpec FR-322-AC-148..150 read-back of the template places: the I2 read
/// admits the emitted package and reconstructs both places.
#[trace("TC-685")]
#[test]
#[ignore = "IR does not yet admit symbolic slot selectors (STD-526 IR delta, luna)"]
fn the_template_places_read_back() {
    let document = clocked_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
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
        .insert_domain_package_document(qsl_semantics::model::key::hex(digest), document.clone())
        .expect("a fresh evidence key");
    let read = read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id()),
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the template places: {failure:?}"));
    let typed = CheckedTimedModel::from_admitted(&read.into_value().admitted)
        .expect("I2 reconstructs the typed timed model");
    let [timed] = typed.as_slice() else {
        panic!("one timed model reads back");
    };
    assert_eq!(timed.clock_places.len(), 2);
    assert!(timed
        .clock_places
        .iter()
        .all(|place| place.guards.is_empty()));
}
