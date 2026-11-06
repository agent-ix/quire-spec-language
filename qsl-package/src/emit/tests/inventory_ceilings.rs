// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-521 row 11, QSL-838 ruling (TC-1316): the legacy `definitions` and
//! `models` inventory dimensions are bounded on the retained path by
//! `i2.nodes`, because each selected definition or model is a node of the
//! emitted graph. The fixtures are real emissions whose node count is driven
//! by the inventory: N authored function definitions, and the model selection.

use super::temporal_clause::{compile, evidence, unit, INFINITE};
use super::*;

/// QSL's I2 read of the compiled package with `i2.nodes` at `bound`.
fn read_nodes(compiled: &temporal_clause::Compiled, bound: u64) -> Read {
    read_v2(
        compiled.emission.package.bytes(),
        library(),
        V2ReadLimits::fixture().with_nodes(bound),
        &evidence(&compiled.wire),
    )
}

fn node_count(compiled: &temporal_clause::Compiled) -> u64 {
    u64::try_from(
        compiled.wire["semantic_graph"]["nodes"]
            .as_array()
            .expect("nodes")
            .len(),
    )
    .unwrap()
}

fn functions(count: usize) -> String {
    (0..count)
        .map(|index| format!("function f{index} using v(): Boolean pure {{ true }}\n"))
        .collect()
}

/// Zero, exact and one-short `i2.nodes` over `compiled`, whose node count is
/// `total`.
fn assert_node_ceiling(compiled: &temporal_clause::Compiled, total: u64, label: &str) {
    for bound in [0, total - 1] {
        let Read::Limit(limit) = read_nodes(compiled, bound) else {
            panic!("{label}: i2.nodes at {bound} must stop the read");
        };
        assert_eq!(limit.setting(), qsl_foundation::Setting::I2Nodes, "{label}");
        assert_eq!(limit.configured_bound(), bound, "{label}");
        assert_eq!(limit.actual(), u128::from(total), "{label}");
    }
    assert!(
        matches!(read_nodes(compiled, total), Read::Verified { .. }),
        "{label}: i2.nodes at the exact count admits"
    );
}

/// `definitions`: each authored definition adds exactly one graph node, so
/// `i2.nodes` refuses an inventory of N definitions one node short.
#[trace("TC-1316", "FR-521-AC-5", "FR-096-AC-10", "FR-255-AC-1")]
#[test]
fn a_definition_inventory_is_bounded_by_the_node_ceiling() {
    const N: usize = 4;
    let none = compile(&unit(INFINITE, &functions(1)));
    let many = compile(&unit(INFINITE, &functions(1 + N)));
    let base = node_count(&none);
    let total = node_count(&many);
    assert_eq!(total - base, N as u64, "each definition is one node");
    assert_node_ceiling(&many, total, "definitions");
    // The inventory alone moves the exact bound: the smaller package is
    // refused by the bound that is exact for the larger one minus N.
    assert!(matches!(read_nodes(&none, base), Read::Verified { .. }));
    assert!(matches!(read_nodes(&many, base), Read::Limit(_)));
}

/// `models`: the selected model contributes its model-root node to the graph,
/// so `i2.nodes` bounds a package that selects a model.
#[trace("TC-1316", "FR-521-AC-5", "FR-096-AC-10", "FR-255-AC-1")]
#[test]
fn a_model_selection_is_bounded_by_the_node_ceiling() {
    let with_model = compile(&unit(INFINITE, &functions(1)));
    let model_roots = with_model.wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|node| node["semantic_form"] == "model_root")
        .count();
    assert!(model_roots >= 1, "the selected model is a graph node");
    assert_node_ceiling(&with_model, node_count(&with_model), "models");
}
