// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-383-AC-25: a caller-funded large recursion group survives S4 and the
//! complete I2 read without a native-stack or group-size ceiling.

use super::*;

#[trace("FR-383-AC-25")]
#[test]
fn a_100000_node_cycle_emits_and_reads_back_under_supplied_limits() {
    const RECORDS: usize = 50_000;
    let names: Vec<String> = (0..RECORDS).map(|index| format!("Node{index}")).collect();
    let keys: Vec<NodeKey> = names
        .iter()
        .map(|name| {
            NodeKey::new("pkg", vec![quire_exact::PathSegment::Text(name.clone())], 0)
                .expect("a fixture node identity")
        })
        .collect();
    let records = (0..RECORDS).map(|index| {
        CompositeDeclaration::new(
            keys[index],
            names[index].clone(),
            CompositeShape::Record(vec![FieldDeclaration::new(
                "next",
                ValueType::collection(CollectionType::new(
                    CollectionKind::Sequence,
                    ValueType::Composite(keys[(index + 1) % RECORDS]),
                    None,
                )),
                Presence::Required,
            )]),
        )
    });
    let types = TypeEnvironment::new(records, []).expect("the cycle types admit");
    let limits = CheckingLimits::new(500_000).with_work_budget(u64::MAX);
    let checked = PackageDeclarations {
        types,
        ..PackageDeclarations::new(source(), "pkg", qsl_foundation::IdentityLimits::default())
    }
    .check(limits)
    .unwrap_or_else(|refusals| {
        panic!(
            "caller budget admits the cycle; checker returned {} refusals",
            refusals.len()
        )
    });
    let group_size = checked
        .semantic_graph()
        .nodes()
        .filter(|node| node.recursion().is_some())
        .count();
    assert_eq!(group_size, 100_000, "the graph holds the full cyclic group");

    let package = CheckedPackage::link(checked);
    let emission = emit_under(&package, package.graph().source());
    assert!(emission.omitted.is_empty());
    let wire = wire(&emission);
    let limits = V2ReadLimits::default()
        .with_artifact_bytes(usize::MAX)
        .with_nodes(500_000)
        .with_edges(1_000_000)
        .with_occurrences(500_000)
        .with_work(u64::MAX);
    let read = read_v2(
        emission.package.bytes(),
        library(),
        limits,
        &read_evidence(&emission),
    );
    assert!(matches!(read, Read::Verified { .. }), "I2 read: {read:?}");
    assert!(
        wire["semantic_graph"]["nodes"]
            .as_array()
            .is_some_and(|nodes| nodes.len() >= 100_000),
        "the wire holds at least 100,000 nodes"
    );
}
