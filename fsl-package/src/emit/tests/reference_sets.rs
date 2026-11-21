// SPDX-License-Identifier: AGPL-3.0-or-later
//! Protocol sets through source checking and the actual package emitter.

use super::*;

#[trace("FSpec-FR-464-AC-3")]
#[test]
fn attempt_contract_sets_are_structural_across_source_permutations() {
    let declarations = "\
        post Zed using v on Config::ConfigVersion::attemptUpdate { true }\n\
        post Alpha using v on Config::ConfigVersion::attemptUpdate { false }\n\
        post Middle using v on Config::ConfigVersion::attemptUpdate { true }";
    let mut sets = Vec::new();
    for contracts in [
        "Zed, Alpha, Middle",
        "Middle, Zed, Alpha",
        "Alpha, Middle, Zed",
    ] {
        let source = attempt_frame_unit(
            declarations,
            "Config::ConfigVersion::attemptUpdate",
            contracts,
        );
        let emitted = emit_attempt_unit(&source, false);
        let graph = WireGraph::of_package_value(&emitted.wire).unwrap();
        let attempt = nodes(&emitted.wire)
            .iter()
            .find(|node| node["body"]["operation"]["member"]["operator"] == "attempt")
            .expect("the emitted attempt");
        let contracts = attempt["body"]["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|binding| binding["name"] == "contracts")
            .expect("the contracts set");
        let structural: Vec<_> = contracts["value"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|reference| {
                fsl_semantics::check::wire_id::structural_id(
                    &graph.key_of_value(&reference["target"]).unwrap(),
                )
            })
            .collect();
        assert_eq!(structural.len(), 3, "all contracts survive emission");
        assert!(
            structural.windows(2).all(|pair| pair[0] < pair[1]),
            "contract targets are strictly ascending structural identities: {structural:?}"
        );
        let mut expected: Vec<_> = emitted.clauses.iter().map(node_id).collect();
        expected.sort_unstable();
        assert_eq!(
            structural, expected,
            "the set names exactly the checked clauses"
        );
        sets.push(structural);
    }
    assert!(
        sets.windows(2).all(|pair| pair[0] == pair[1]),
        "source permutations have one reference-set encoding"
    );
}
