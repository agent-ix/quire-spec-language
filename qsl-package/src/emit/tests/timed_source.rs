// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230-AC-32/AC-38: a checked time source reaches S4 as typed timed nodes,
//! preserves its resolved model/unit identities, and passes I2 read-back.

use super::admission_corpus::config_version_unit;
use super::*;
use crate::checked_v2::{read_checked_package_v2, CheckedTimedModel};
use quire_contract_model::CheckedPackageEvidence;

fn package_units_document() -> Vec<u8> {
    let mut document: Value =
        serde_json::from_slice(config_version_unit::DOMAIN_PACKAGE.as_bytes())
            .expect("the fixture is JSON");
    let origin = document["types"][0]["origin"].clone();
    let time = "ix://example/config-version/dimension/Time";
    let second = "ix://example/config-version/unit/s";
    document["dimensions"] = json!([{
        "identity": time,
        "name": "Time",
        "base": true,
        "terms": [],
        "origin": origin,
    }]);
    document["units"] = json!([
        {
            "identity": second,
            "name": "s",
            "dimension": time,
            "scale": "1",
            "offset": "0",
            "origin": origin,
        },
        {
            "identity": "ix://example/config-version/unit/ms",
            "name": "ms",
            "dimension": time,
            "target": second,
            "scale": "1/1000",
            "offset": "0",
            "origin": origin,
        },
    ]);
    serde_json::to_vec(&document).expect("the fixture stays JSON")
}

#[trace("FR-230-AC-32", "FR-230-AC-38")]
#[test]
fn a_checked_time_source_emits_and_reads_back_its_resolved_nodes() {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]).unwrap();
    let unit = config_version_unit::unit_text().replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n",
    );
    let unit = format!(
        "{unit}dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config dense unit ms;\n"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the time source checks: {refusals:?}"));
    let [checked_source] = graph.time_sources() else {
        panic!("one checked time source");
    };
    let model_node = checked_source.model_node.clone();
    let unit_node = checked_source.unit_node.clone();
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);

    let wire = wire(&emission);
    assert!(
        nodes(&wire)
            .iter()
            .any(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "text"),
        "the timed source metadata's Text type is in the graph"
    );
    let [root] = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "timed_model")
        .collect::<Vec<_>>()[..]
    else {
        panic!("one emitted timed root");
    };
    let time_dimension = root["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "time_dimension")
        .unwrap()["value"]["target"]
        .clone();
    assert!(
        nodes(&wire)
            .iter()
            .any(|node| node["node_id"] == time_dimension),
        "the time dimension node {time_dimension} is in the emitted graph"
    );
    let model = root["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "model")
        .unwrap();
    assert_eq!(
        qsl_foundation::node_wire::node_key_from_json(&model["value"]["target"]).unwrap(),
        model_node
    );
    let source_id = root["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "source")
        .unwrap()["value"]["target"]
        .clone();
    let source_node = nodes(&wire)
        .iter()
        .find(|node| node["node_id"] == source_id)
        .expect("the root references its typed source");
    assert_eq!(source_node["semantic_form"], "time_source");
    assert_eq!(
        source_node["body"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["name"] == "unit")
            .unwrap()["value"]["target"],
        qsl_foundation::node_wire::node_key_to_json(&unit_node).unwrap()
    );

    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(document), document.to_vec())
        .expect("a fresh evidence key");
    let read = read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the emitted timed model: {failure:?}"));
    let typed = CheckedTimedModel::from_admitted(&read.into_value().admitted)
        .expect("I2 reconstructs the typed timed model");
    let [timed] = typed.as_slice() else {
        panic!("one timed model reads back");
    };
    assert_eq!(
        serde_json::to_value(&timed.model).unwrap(),
        qsl_foundation::node_wire::node_key_to_json(&model_node).unwrap()
    );
    assert_eq!(
        timed.source.kind,
        crate::checked_v2::CheckedTimeSourceKind::Dense
    );
    assert!(timed.clock_places.is_empty());
    assert!(timed.resets.is_empty());
    assert!(timed.guards.is_empty());
    assert!(timed.invariants.is_empty());
    assert!(timed.urgencies.is_empty());
    assert!(timed.constraints.is_empty());
}

/// QSpec FR-322 "Nominal nodes": a source dimension or unit a timed source
/// names is located at its own declaration, not at the `time` declaration.
#[trace("QSpec-FR-322-AC-1")]
#[test]
fn a_source_time_nominal_is_located_at_its_own_declaration() {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]).unwrap();
    let unit = config_version_unit::unit_text().replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n",
    );
    let unit = format!(
        "{unit}dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config dense unit ms;\n"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the time source checks: {refusals:?}"));
    let [checked_source] = graph.time_sources() else {
        panic!("one checked time source");
    };
    let nominals = [
        (checked_source.time_dimension.clone(), "Time"),
        (checked_source.unit_node.clone(), "ms"),
    ];
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let wire = wire(&emission);
    for (key, declaration) in nominals {
        let id = qsl_foundation::node_wire::node_key_to_json(&key).unwrap();
        let entry = wire["source_map"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["node_id"] == id && entry["role"] == "declaration")
            .unwrap_or_else(|| panic!("a declaration occurrence of {declaration}"));
        let region = &entry["regions"][0];
        let (start, end) = (
            region["start"].as_u64().unwrap() as usize,
            region["end"].as_u64().unwrap() as usize,
        );
        assert_eq!(&unit[start..end], declaration);
    }
}

/// QSpec FR-322 "Nominal nodes", AC-51 and AC-81: a dimension an admitted
/// domain package declares and a source unit's function names is written
/// with its `ModelOwner`, no `declaration`, a `generated` occurrence and the
/// path `["scalar_type", "dimension", <FCD identity>]`; the unit over it and
/// the function using the unit are written too, and the I2 read admits them.
#[trace("QSpec-FR-322-AC-51", "QSpec-FR-322-AC-81")]
#[test]
fn a_package_dimension_a_function_names_is_written_model_owned() {
    let document = package_units_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = config_version_unit::unit_text();
    let unit = format!(
        "{unit}dimension Dur = Config::Time;\nunit dsec : Dur = rational(1, 1);\n\
         function f using v(d: Quantity<dsec>): Boolean pure {{ true }}\n"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let wire_nodes = nodes(&wire);
    let of_form = |form: &str| -> Vec<&Value> {
        wire_nodes
            .iter()
            .filter(|node| node["semantic_form"] == form)
            .collect()
    };
    let time = "ix://example/config-version/dimension/Time";
    let [dimension] = of_form("dimension")
        .into_iter()
        .filter(|node| node["owner"]["kind"] == "model")
        .collect::<Vec<_>>()[..]
    else {
        panic!("one model-owned dimension node");
    };
    assert_eq!(dimension["owner"]["node"], time);
    assert!(dimension.get("declaration").is_none_or(Value::is_null));
    assert_eq!(
        dimension["node_id"]["path"],
        json!(["scalar_type", "dimension", time])
    );
    let occurrences: Vec<_> = wire["source_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["node_id"] == dimension["node_id"])
        .collect();
    let [occurrence] = occurrences[..] else {
        panic!("one occurrence: {occurrences:?}");
    };
    assert_eq!(occurrence["role"], "generated");
    assert!(
        of_form("unit")
            .iter()
            .any(|node| node["declaration"]["qualified_name"] == json!(["dsec"])),
        "the source unit over it is written"
    );
    assert!(
        of_form("pure_function")
            .iter()
            .any(|node| node["declaration"]["qualified_name"] == json!(["f"])),
        "the function using the unit is written"
    );

    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(&document), document)
        .expect("a fresh evidence key");
    read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the model-owned dimension: {failure:?}"));
}

#[trace("FR-230-AC-5", "FR-230-AC-32", "FR-230-AC-38")]
#[test]
fn a_package_owned_time_unit_emits_its_model_owned_nominal_nodes() {
    let document = package_units_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = config_version_unit::unit_text().replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n",
    );
    let unit = format!("{unit}time Config dense unit Config::ms;\n");
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package time source checks: {refusals:?}"));
    let [checked_source] = graph.time_sources() else {
        panic!("one checked time source");
    };
    let dimension_node = checked_source.time_dimension.clone();
    let unit_node = checked_source.unit_node.clone();
    let emission =
        emit_checked(&CheckedPackage::link(graph)).expect("the package-owned timed model emits");
    let wire = wire(&emission);
    for key in [&dimension_node, &unit_node] {
        assert!(
            nodes(&wire)
                .iter()
                .any(
                    |node| qsl_foundation::node_wire::node_key_from_json(&node["node_id"])
                        .ok()
                        .as_ref()
                        == Some(key)
                ),
            "the model-owned nominal node {key} is emitted"
        );
    }
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(&document), document)
        .expect("a fresh evidence key");
    read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits package-owned time nodes: {failure:?}"));
}

#[trace("FR-230-AC-32", "FR-230-AC-35")]
#[test]
fn a_tick_time_source_emits_its_exact_period_and_operation_anchor() {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]).unwrap();
    let unit = config_version_unit::unit_text().replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n",
    );
    let unit = format!(
        "{unit}dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick ConfigVersion::attemptUpdate period 1 ms unit ms;\n"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the tick source checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the tick model emits");
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(document), document.to_vec())
        .expect("a fresh evidence key");
    let read = read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the tick model: {failure:?}"));
    let typed = CheckedTimedModel::from_admitted(&read.into_value().admitted)
        .expect("I2 reconstructs the tick model");
    let [timed] = typed.as_slice() else {
        panic!("one timed model reads back");
    };
    assert_eq!(
        timed.source.kind,
        crate::checked_v2::CheckedTimeSourceKind::Tick
    );
    assert_eq!(timed.source.period.as_deref(), Some("1/1"));
    assert!(timed.source.operation.is_some());
}

/// FR-230-AC-18's identity half (TC-685 step 7): with `example/config-version`
/// (`R`: `Time`, `s`, `ms`) and the same package as `example/flow` (`F`:
/// `Time`, `sec`) both selected and a source `Time` with `s`, the time
/// members `time R dense unit R::ms`, `time R dense unit F::sec` and `time
/// R dense unit s` each emit a timed root whose `time_dimension` is their
/// own `Time` node, three different structural nodes. The S3 refusals of mixed
/// dimensions are `qsl-semantics`' `time_dimensions_of_different_owners_do_not_mix`.
#[trace("TC-685", "FR-230-AC-18")]
#[test]
fn three_time_dimensions_have_distinct_structural_keys() {
    let rpc = package_units_document();
    let mut flow: Value = serde_json::from_str(
        &config_version_unit::DOMAIN_PACKAGE.replace("example/config-version", "example/flow"),
    )
    .expect("the renamed fixture is JSON");
    let origin = flow["types"][0]["origin"].clone();
    let time = "ix://example/flow/dimension/Time";
    flow["dimensions"] = json!([{
        "identity": time, "name": "Time", "base": true, "terms": [], "origin": origin,
    }]);
    flow["units"] = json!([{
        "identity": "ix://example/flow/unit/sec", "name": "sec", "dimension": time,
        "scale": "1", "offset": "0", "origin": origin,
    }]);
    let flow = serde_json::to_vec(&flow).expect("JSON");
    let packages =
        qsl_semantics::model::intake::package_input([rpc.as_slice(), flow.as_slice()]).unwrap();
    let identity = |unit: &str| {
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.model.complete/v1\";\n\
             profile t = \"quire.temporal.complete/v1\";\n\
             model R = \"example/config-version\";\n\
             model F = \"example/flow\";\n\
             dimension Time;\nunit s : Time = rational(1, 1);\n\
             function noop using v(): Boolean pure {{ true }}\n\
             time R dense unit {unit};\n"
        );
        let graph = assemble_with_models(text.as_bytes(), &packages)
            .expect("the unit assembles")
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{unit}: {refusals:?}"));
        let dimension = graph.time_sources()[0].time_dimension.clone();
        let emission = emit_checked(&CheckedPackage::link(graph))
            .unwrap_or_else(|refusal| panic!("{unit} emits: {refusal:?}"));
        // The emitted timed root references the checked time dimension.
        let wire = wire(&emission);
        let [root] = nodes(&wire)
            .iter()
            .filter(|node| node["semantic_form"] == "timed_model")
            .collect::<Vec<_>>()[..]
        else {
            panic!("{unit}: one emitted timed root");
        };
        let emitted = root["body"]["members"]
            .as_array()
            .and_then(|members| {
                members
                    .iter()
                    .find(|member| member["name"] == "time_dimension")
            })
            .map(|member| member["value"]["target"].clone())
            .expect("the root's time dimension");
        assert_eq!(
            qsl_foundation::node_wire::node_key_from_json(&emitted).unwrap(),
            dimension,
            "{unit}"
        );
        dimension
    };
    let rpc = identity("R::ms");
    let flow = identity("F::sec");
    let own = identity("s");
    // The three selected time dimensions remain distinct.
    for (left, right) in [(&rpc, &flow), (&rpc, &own), (&flow, &own)] {
        assert_ne!(left, right);
    }
}
