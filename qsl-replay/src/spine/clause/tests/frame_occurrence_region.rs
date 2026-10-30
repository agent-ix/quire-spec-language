// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-462 step 6 (FR-105-AC-9): the source region of a frame node's
//! `generated` occurrences, read from the emitted `source_map`.

use super::receiver_frames::unit_and_packages;
use super::*;

const REFERENCE: &str = "Config::ConfigVersion::attemptUpdate";

fn compile(unit: &str, packages: &qsl_semantics::model::intake::PackageInput) -> ComposedUnit {
    compose(
        source(),
        "frame-occurrence-region.native",
        unit.as_bytes(),
        packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("compile refused: {refusal:?}"))
}

/// The ConfigVersion unit with `earlier` inserted before `VersionUnchanged`.
fn config_version(earlier: &str) -> (String, ComposedUnit) {
    let (unit, packages) = config_version_unit_and_packages();
    let unit = unit.replacen(
        "post VersionUnchanged",
        &format!("{earlier}post VersionUnchanged"),
        1,
    );
    let compiled = compile(&unit, &packages);
    (unit, compiled)
}

/// Every `generated` source-map entry of the emitted `state`/`frame` node
/// that `operation`'s `operation_anchor` references, or of the one frame
/// node when `operation` is `None`.
fn generated_entries(compiled: &ComposedUnit, operation: Option<&str>) -> Vec<serde_json::Value> {
    let wire: serde_json::Value =
        serde_json::from_slice(compiled.emitted.bytes()).expect("emitted bytes are JSON");
    let nodes = wire["semantic_graph"]["nodes"].as_array().expect("nodes");
    let frame_id = match operation {
        None => {
            let frames: Vec<_> = nodes
                .iter()
                .filter(|node| node["body"]["term"] == "frame")
                .collect();
            let [frame] = frames[..] else {
                panic!("one frame node: {frames:#?}");
            };
            frame["node_id"].clone()
        }
        Some(operation) => {
            let anchors: Vec<_> = nodes
                .iter()
                .filter(|node| node["semantic_form"] == "operation_anchor")
                .filter(|node| node["body"]["members"][1]["value"]["value"] == operation)
                .collect();
            let [anchor] = anchors[..] else {
                panic!("{operation} has one anchor: {anchors:#?}");
            };
            anchor["body"]["members"][2]["value"]["target"].clone()
        }
    };
    wire["source_map"]
        .as_array()
        .expect("source_map")
        .iter()
        .filter(|entry| entry["node_id"] == frame_id && entry["role"] == "generated")
        .cloned()
        .collect()
}

/// The offset and text in `unit` of `entry`'s one region.
fn region_text(unit: &str, entry: &serde_json::Value) -> (usize, String) {
    let [region] = &entry["regions"].as_array().expect("regions")[..] else {
        panic!("exactly one region: {entry:#?}");
    };
    let start = usize::try_from(region["start"].as_u64().unwrap()).unwrap();
    let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
    (start, unit[start..end].to_owned())
}

/// Where `reference` begins in the clause `post {name} using v on `.
fn reference_in(unit: &str, name: &str, reference: &str) -> (usize, String) {
    let prefix = format!("post {name} using v on ");
    let start = unit.find(&prefix).expect("the clause") + prefix.len();
    (start, reference.to_owned())
}

fn attempt_reference_in(unit: &str) -> (usize, String) {
    let prefix = "on ";
    let attempt = unit
        .find("attempt Update by R on Config::ConfigVersion::attemptUpdate contracts []")
        .expect("the attempt");
    let start = unit[attempt..]
        .find(prefix)
        .map(|offset| attempt + offset + prefix.len())
        .expect("the operation reference");
    (start, REFERENCE.to_owned())
}

fn protocol_attempt() -> &'static str {
    "protocol AttemptUpdate using v over (input: Boolean) on origin {\n\
     role R on Config::ConfigVersion;\n\
     run sequence Main {\n\
     attempt Update by R on Config::ConfigVersion::attemptUpdate contracts [] as (tried: Boolean) { true };\n\
     }\n\
     finish End as (outcome: Boolean) { true };\n\
     }\n"
}

#[trace("TC-462", "FR-105-AC-9")]
#[test]
fn frame_occurrence_region_is_the_first_naming_clauses_operation_reference() {
    // The ConfigVersion package: the one occurrence is at `VersionUnchanged`.
    let (unit, base) = config_version("");
    let [entry] = &generated_entries(&base, None)[..] else {
        panic!("one generated entry");
    };
    assert_eq!(
        region_text(&unit, entry),
        reference_in(&unit, "VersionUnchanged", REFERENCE)
    );

    // A clause naming the operation earlier takes the region, and the
    // entry's node id, role and ordinal stay the same.
    let (earlier_unit, earlier) =
        config_version("post Earlier using v on Config::ConfigVersion::attemptUpdate { true }\n");
    let [moved] = &generated_entries(&earlier, None)[..] else {
        panic!("one generated entry after adding the earlier clause");
    };
    assert_eq!(
        region_text(&earlier_unit, moved),
        reference_in(&earlier_unit, "Earlier", REFERENCE)
    );
    for key in ["node_id", "role", "ordinal"] {
        assert_eq!(moved[key], entry[key], "{key} is unchanged");
    }

    // Fixture R: `all` and `same` share a frame node, ordinals 0 and 1, and
    // each is placed at its own operation's reference.
    let (unit, packages) = unit_and_packages(&["noop", "all", "same"]);
    let compiled = compile(&unit, &packages);
    let all = generated_entries(&compiled, Some("all"));
    assert_eq!(all, generated_entries(&compiled, Some("same")));
    let [first, second] = &all[..] else {
        panic!("one generated entry per operation: {all:#?}");
    };
    assert_eq!(first["ordinal"], serde_json::json!(0));
    assert_eq!(second["ordinal"], serde_json::json!(1));
    assert_eq!(
        region_text(&unit, first),
        reference_in(&unit, "DoneAll", "Counters::Counter::all")
    );
    assert_eq!(
        region_text(&unit, second),
        reference_in(&unit, "DoneSame", "Counters::Counter::same")
    );

    // A frame occurrence with no region refuses; placing every occurrence
    // emits.
    let graph = base.package.graph();
    let frames: Vec<_> = graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.semantic_form() == "frame")
        .map(|node| node.key())
        .collect();
    let [frame] = frames[..] else {
        panic!("one frame node: {frames:?}");
    };
    let located: Vec<_> = graph
        .occurrences()
        .filter(|(key, origin, _)| *key == frame && origin.role().as_str() == "generated")
        .collect();
    let [(_, _, frame_location)] = located[..] else {
        panic!("one generated frame occurrence: {located:?}");
    };
    qsl_package::emit_package(&base.package, |location| graph.region(location))
        .expect("every occurrence placed emits");
    let refusal = qsl_package::emit_package(&base.package, |location| {
        (location != frame_location)
            .then(|| graph.region(location))
            .flatten()
    })
    .expect_err("the unplaced frame occurrence refuses");
    assert!(
        matches!(
            &refusal,
            qsl_package::EmitRefusal::UnlocatedOccurrence {
                node,
                role: quire_contract_model::CheckedOccurrenceRole::Generated,
                ordinal: 0,
            } if *node == frame
        ),
        "{refusal:?}"
    );
}

/// A protocol attempt names the operation at its qualified reference, and
/// the first clause or attempt in source order determines the frame region.
#[trace("TC-462", "FR-105-AC-11")]
#[test]
fn frame_occurrence_region_is_the_first_naming_clause_or_attempt_in_source_order() {
    let (base, packages) = config_version_unit_and_packages();
    let attempt = protocol_attempt();

    let attempt_first = base.replacen(
        "post VersionUnchanged",
        &format!("{attempt}post VersionUnchanged"),
        1,
    );
    let attempt_first_graph = compile(&attempt_first, &packages);
    let [attempt_entry] = &generated_entries(&attempt_first_graph, Some("attemptUpdate"))[..]
    else {
        panic!("one generated frame occurrence for the clause and attempt");
    };
    assert_eq!(
        region_text(&attempt_first, attempt_entry),
        attempt_reference_in(&attempt_first)
    );

    let clause_first = base.replacen(
        "function sameIdentity",
        &format!("{attempt}function sameIdentity"),
        1,
    );
    let clause_first_graph = compile(&clause_first, &packages);
    let [clause_entry] = &generated_entries(&clause_first_graph, Some("attemptUpdate"))[..] else {
        panic!("one generated frame occurrence for the clause and attempt");
    };
    assert_eq!(
        region_text(&clause_first, clause_entry),
        reference_in(&clause_first, "VersionUnchanged", REFERENCE)
    );
    for key in ["node_id", "role", "ordinal"] {
        assert_eq!(attempt_entry[key], clause_entry[key], "{key} is stable");
    }

    let clause_start = base.find("post VersionUnchanged").expect("the clause");
    let function_start = base.find("function sameIdentity").expect("the function");
    let attempt_only = format!(
        "{}{attempt}{}",
        &base[..clause_start],
        &base[function_start..]
    );
    let attempt_only_graph = compile(&attempt_only, &packages);
    let [only_attempt_entry] = &generated_entries(&attempt_only_graph, Some("attemptUpdate"))[..]
    else {
        panic!("one generated frame occurrence for the attempt");
    };
    assert_eq!(
        region_text(&attempt_only, only_attempt_entry),
        attempt_reference_in(&attempt_only)
    );
}
