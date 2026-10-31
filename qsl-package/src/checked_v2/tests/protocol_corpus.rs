// SPDX-License-Identifier: AGPL-3.0-or-later
//! The positive packages named by QSpec's protocol corpus, read in place.

use super::*;

/// The protocol corpus's positive rows select packages, rather than copies of
/// packages held by QSL. Check the advertised form and operator actually occur.
#[trace("FR-383-AC-19", "FR-383-AC-20", "TC-1063")]
#[test]
fn conformance_protocol_vector_positive_packages_are_admitted() {
    let Some(directory) = qspec_directory() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let fixtures = directory.join("proposals/checked-package-v2/fixtures");
    let vectors = read_fixture(&fixtures.join("protocol-node-vectors.json"));
    let positives = vectors["positive"]
        .as_array()
        .expect("positive protocol vectors");
    assert!(
        !positives.is_empty(),
        "protocol corpus contains positive packages"
    );
    let mut failures = Vec::new();
    for row in positives {
        let id = row["id"].as_str().expect("vector id");
        assert_eq!(row["expected"], "admitted", "{id}");
        let fixture = row["fixture"].as_str().expect("positive package path");
        let envelope = read_fixture(&fixtures.join(fixture));
        let outcome = read_fixture_wire(&format!("fixtures/{fixture}"), &envelope).1;
        let Read::Verified { admitted, .. } = outcome else {
            failures.push(format!(
                "{id}: {fixture} must be admitted by QSL I2: {outcome:?}"
            ));
            continue;
        };
        let matching = envelope["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|node| {
                node["node_tag"] == "protocol" && node["semantic_form"] == row["node_form"]
            })
            .filter(|node| {
                row.get("operator").is_none_or(|operator| {
                    node["body"]["operation"]["member"]["operator"] == *operator
                })
            })
            .collect::<Vec<_>>();
        assert!(
            !matching.is_empty(),
            "{id}: advertised protocol node exists"
        );
        for node in matching {
            let retained = admitted
                .graph()
                .nodes
                .iter()
                .find(|candidate| {
                    serde_json::to_value(&candidate.node_id).unwrap() == node["node_id"]
                })
                .expect("I2 retains the advertised node");
            assert_eq!(
                serde_json::to_value(retained).unwrap(),
                *node,
                "{id}: exact protocol node read-back"
            );
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!(
        "conformance: {} protocol vector positive packages admitted",
        positives.len()
    );
}
