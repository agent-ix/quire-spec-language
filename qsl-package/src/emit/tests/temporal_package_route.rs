// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337 (TC-1326 step 5), FR-515 (TC-1310 rows 19 to 23): what the emitter
//! returns for each temporal clause occurrence, and what the checked package
//! views read from the emitted bytes.

use std::sync::Arc;

use quire_contract_model::{
    CheckedNodeId, CheckedNodeKind, CheckedPackageV2, ClockConfiguration, TemporalForm,
};

use super::temporal_clause::{compile, compile_with, declared, declared_on, evidence, unit, EVENT};
use super::*;
use crate::checked_views::{instance_bound_atoms, temporal_subject};

/// QSL's I2 read of `bytes`, and IR's admitted package.
fn admit_bytes(bytes: &[u8], wire: &Value) -> Arc<CheckedPackageV2> {
    let read = crate::checked_v2::read_checked_package_v2(
        bytes,
        library(),
        V2ReadLimits::fixture(),
        &evidence(wire),
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the package: {failure:?}"));
    read.into_value().admitted
}

fn clause_ids(package: &CheckedPackageV2) -> Vec<CheckedNodeId> {
    package
        .resolved_graph()
        .nodes
        .iter()
        .zip(package.node_kinds())
        .filter(|(_, kind)| **kind == CheckedNodeKind::Temporal(TemporalForm::TemporalClause))
        .map(|(node, _)| node.node_id.clone())
        .collect()
}

/// A clock of its own for each clause name.
fn authority(clause: &qsl_semantics::check::CheckedTemporalClause) -> Option<ClockConfiguration> {
    Some(ClockConfiguration::EventPosition {
        sequence_authority: format!("authority-{}", clause.name()).into(),
    })
}

fn sequence_authority(row: &quire_contract_model::TemporalBinding) -> &str {
    match &row.clock {
        ClockConfiguration::EventPosition { sequence_authority } => sequence_authority,
        other => panic!("an event-position clock: {other:?}"),
    }
}

const BODY: &str = "eventually[0,1] holds(true)";

/// The two structurally equal clauses `First` and `Second` in `order`.
fn twins(order: [&str; 2]) -> String {
    unit(
        EVENT,
        &format!("{}\n{}", declared(order[0], BODY), declared(order[1], BODY)),
    )
}

/// FR-337-AC-23 (TC-1326 step 5): for two structurally equal clauses with
/// ordinals 0 and 1 under a clocked profile, the emission's
/// `TemporalClauseRef` values equal the `temporal_clause_names` rows read
/// back, name the two declarations of the checked graph by `clause`, and its
/// `profile_selections` and `temporal_bindings` rows equal the lock's and the
/// package's byte for byte; reordering the declarations changes `clause` and
/// no `declaration`.
#[trace("FR-337-AC-23", "TC-1326")]
#[test]
fn the_emission_returns_the_clause_refs_and_rows_it_wrote() {
    let mut refs = Vec::new();
    for order in [["First", "Second"], ["Second", "First"]] {
        let compiled = compile_with(&twins(order), &authority);
        let rows = compiled.emission.temporal_rows();
        let package = admit_bytes(compiled.emission.package.bytes(), &compiled.wire);

        let names = package.temporal_clause_names().expect("name rows");
        assert_eq!(rows.clauses().len(), names.len());
        for (found, row) in rows.clauses().iter().zip(names) {
            assert_eq!(found.declaration, row.declaration);
            assert_eq!(found.occurrence, row.occurrence);
            assert_eq!(
                compiled.clauses[found.clause].clause().name(),
                row.name.as_ref(),
                "`clause` names the declaration the row names"
            );
        }
        assert_eq!(
            crate::emit::tests::wire_bytes(rows.profile_selections()).unwrap(),
            crate::emit::tests::wire_bytes(&package.lock().profile_selections).unwrap()
        );
        assert_eq!(
            rows.bindings(),
            package.temporal_bindings().expect("binding rows")
        );
        assert_eq!(
            serde_json::to_value(rows.bindings()).unwrap(),
            compiled.wire["temporal_bindings"]
        );
        // Each declaration by its clause's name: the id, the occurrence and
        // the position in the checked graph.
        let mut named: Vec<_> = rows
            .clauses()
            .iter()
            .map(|found| {
                (
                    compiled.clauses[found.clause].clause().name().to_owned(),
                    found.declaration.clone(),
                    found.clause,
                )
            })
            .collect();
        named.sort();
        refs.push(named);
    }
    let [original, reversed] = &refs[..] else {
        panic!("two compiles");
    };
    for (before, after) in original.iter().zip(reversed) {
        assert_eq!(before.0, after.0);
        assert_eq!(before.1, after.1, "no declaration changes");
        assert_ne!(before.2, after.2, "`clause` changes");
    }
}

/// FR-515-AC-19 (TC-1310 row 19): two clauses of different nodes with
/// different clock rows each read their own row as `clock()`.
#[trace("FR-515-AC-19", "TC-1310")]
#[test]
fn each_node_reads_its_own_clock_row() {
    let compiled = compile_with(
        &unit(
            EVENT,
            &format!(
                "{}\n{}",
                declared("First", "eventually[0,1] holds(true)"),
                declared("Second", "eventually[0,2] holds(true)")
            ),
        ),
        &authority,
    );
    let package = admit_bytes(compiled.emission.package.bytes(), &compiled.wire);
    let ids = clause_ids(&package);
    assert_eq!(ids.len(), 2, "two nodes");
    let mut seen = Vec::new();
    for id in &ids {
        let view = temporal_subject(&package, id).expect("the clause reads");
        let row = view.clock().expect("a clocked clause has its row");
        assert_eq!(row.declaration, *id);
        seen.push(sequence_authority(row).to_owned());
    }
    seen.sort();
    assert_eq!(seen, ["authority-First", "authority-Second"]);
}

/// The wire of the twins of one node with their occurrences' ordinals,
/// clock rows and name rows renumbered as `[(old, new)]`, the node's
/// `occurrences` array in the order given and the lock rows ascending.
fn renumbered(
    compiled: &super::temporal_clause::Compiled,
    map: [(u64, u64); 2],
) -> (Vec<u8>, Value) {
    let renumber = |ordinal: &Value| {
        let old = ordinal.as_u64().expect("an ordinal");
        let (_, new) = map.iter().find(|(from, _)| *from == old).expect("mapped");
        Value::from(*new)
    };
    let mut wire = compiled.wire.clone();
    let id = wire["temporal_bindings"][0]["declaration"].clone();
    for node in wire["semantic_graph"]["nodes"].as_array_mut().unwrap() {
        if node["node_id"] == id {
            for occurrence in node["occurrences"].as_array_mut().unwrap() {
                occurrence["ordinal"] = renumber(&occurrence["ordinal"]);
            }
        }
    }
    for entry in wire["source_map"].as_array_mut().unwrap() {
        if entry["node_id"] == id && entry["role"] == "claim" {
            entry["ordinal"] = renumber(&entry["ordinal"]);
        }
    }
    for member in ["temporal_bindings", "temporal_clause_names"] {
        let rows = wire[member].as_array_mut().unwrap();
        for row in rows.iter_mut() {
            row["occurrence"] = renumber(&row["occurrence"]);
        }
        rows.sort_by_key(|row| row["occurrence"].as_u64());
    }
    let bytes = crate::emit::tests::wire_bytes(&wire).expect("the wire encodes");
    (bytes, wire)
}

/// FR-515-AC-20 (TC-1310 row 20): twins of one node whose `occurrences`
/// array runs ordinal 7 then 3 while the lock's rows ascend 3 then 7, with
/// different clock configurations, read through the reader: `clock_of`
/// returns the row keyed by each ordinal and `clock()` is `None`.
#[trace("FR-515-AC-20", "TC-1310")]
#[test]
fn twin_clock_rows_pair_by_key_and_not_by_position() {
    let compiled = compile_with(&twins(["First", "Second"]), &authority);
    // First (ordinal 0) becomes ordinal 7, Second (ordinal 1) ordinal 3.
    let (bytes, wire) = renumbered(&compiled, [(0, 7), (1, 3)]);
    let package = admit_bytes(&bytes, &wire);
    let [node] = clause_ids(&package).try_into().expect("one shared node");
    let view = temporal_subject(&package, &node).expect("the twins read");

    let ordinals: Vec<_> = view.occurrences().iter().map(|o| o.ordinal).collect();
    assert_eq!(ordinals, [7, 3], "the node's own order");
    let rows = package.temporal_bindings().expect("rows");
    assert_eq!(
        rows.iter().map(|row| row.occurrence).collect::<Vec<_>>(),
        [3, 7],
        "the lock's order"
    );
    assert_eq!(view.clock(), None, "several occurrences");
    let [seven, three] = view.occurrences() else {
        panic!("two occurrences");
    };
    assert_eq!(
        sequence_authority(view.clock_of(three).expect("ordinal 3")),
        "authority-Second"
    );
    assert_eq!(
        sequence_authority(view.clock_of(seven).expect("ordinal 7")),
        "authority-First"
    );
    // A pairing by position would give the first occurrence the first row.
    assert_ne!(
        view.clock_of(seven).map(sequence_authority),
        Some(sequence_authority(&rows[0]))
    );
    let missing = crate::checked_views::ClauseOccurrence {
        ordinal: 5,
        ..seven.clone()
    };
    assert_eq!(view.clock_of(&missing), None);
}

/// FR-515-AC-23 (TC-1310 row 23, the view's part): `formula_graph()` lists
/// every formula node with the positions of its operands, while `formula()`
/// lists the same nodes without the edges, as AC-3 states.
#[trace("FR-515-AC-23", "FR-515-AC-3", "TC-1310")]
#[test]
fn the_formula_graph_lists_each_node_with_its_operand_positions() {
    let compiled = compile(&unit(
        EVENT,
        &declared("Due", "always[0,30] holds(c.versionNumber <= 1)"),
    ));
    let package = admit_bytes(compiled.emission.package.bytes(), &compiled.wire);
    let [node] = clause_ids(&package).try_into().expect("one clause");
    let view = temporal_subject(&package, &node).unwrap();
    let graph: Vec<_> = view
        .formula_graph()
        .iter()
        .map(|used| (used.operation, used.operands.clone()))
        .collect();
    assert_eq!(
        graph,
        [
            ("quire.op.temporal.always", vec![1]),
            ("quire.op.temporal.holds", vec![]),
        ]
    );
    assert_eq!(
        view.formula()
            .iter()
            .map(|used| used.operation)
            .collect::<Vec<_>>(),
        ["quire.op.temporal.always", "quire.op.temporal.holds"]
    );

    let compiled = compile(&unit(
        EVENT,
        &declared(
            "Three",
            "always[0,5] (holds(c.versionNumber >= 11) and holds(c.versionNumber >= 12) \
             and holds(c.versionNumber >= 13))",
        ),
    ));
    let package = admit_bytes(compiled.emission.package.bytes(), &compiled.wire);
    let [node] = clause_ids(&package).try_into().expect("one clause");
    let view = temporal_subject(&package, &node).unwrap();
    let graph: Vec<_> = view
        .formula_graph()
        .iter()
        .map(|used| (used.operation, used.operands.clone()))
        .collect();
    assert_eq!(
        graph,
        [
            ("quire.op.temporal.always", vec![1]),
            ("quire.op.temporal.and", vec![2, 5]),
            ("quire.op.temporal.and", vec![3, 4]),
            ("quire.op.temporal.holds", vec![]),
            ("quire.op.temporal.holds", vec![]),
            ("quire.op.temporal.holds", vec![]),
        ]
    );
}

/// The `holds` atoms of a clause with a guard that reads the trigger binder:
/// one reads a capture, one reads only `over`, one reads neither, one reads
/// `over` and the capture.
fn atoms(order: [usize; 4]) -> String {
    let atoms = [
        "holds(n >= 1)",
        "holds(c.versionNumber <= 5)",
        "holds(true)",
        "holds(c.versionNumber >= n)",
    ];
    let [a, b, c, d] = order.map(|index| atoms[index]);
    unit(
        EVENT,
        &declared_on(
            "Due",
            "on each (q: Config::ConfigVersion) when (q.versionNumber >= 2)",
            &format!("capture n: Int[0, 1000] = c.versionNumber; always[0,5] ({a} and {b} and {c} and {d})"),
        ),
    )
}

fn bound(unit: &str) -> Vec<bool> {
    let compiled = compile(unit);
    let package = admit_bytes(compiled.emission.package.bytes(), &compiled.wire);
    let [node] = clause_ids(&package).try_into().expect("one clause");
    let view = temporal_subject(&package, &node).unwrap();
    assert_eq!(view.predicate_leaves().len(), 4, "the guard is no atom");
    instance_bound_atoms(&view).expect("the clause reads")
}

/// FR-515-AC-22 (TC-1310 row 22): the instance-bound atoms are derived from
/// the checked predicate's binders, by formula order: the atom reading a
/// capture and the one reading `over` and a capture are bound, the one
/// reading only `over` and the one reading neither are not, the guard has no
/// index, and swapping the formula order of the first two atoms moves the
/// indices with them.
#[trace("FR-515-AC-22", "TC-1310")]
#[test]
fn instance_bound_atoms_are_derived_by_formula_order() {
    assert_eq!(bound(&atoms([0, 1, 2, 3])), [true, false, false, true]);
    assert_eq!(bound(&atoms([1, 0, 2, 3])), [false, true, false, true]);
}

/// FR-532-AC-11: aggregate capture dependence survives package reading.
#[trace("FR-532-AC-11", "FR-516-AC-47")]
#[test]
fn aggregate_formula_graph_retains_members_and_capture_binding() {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t = \"{EVENT}\";\n\
         temporal Due using t over (e: Int[0, 9]) clock \"model-steps\" on origin {{\
           capture n: Int[0, 9] = e; always[0,5] (sum of n over past[0,2] <= 15)\
         }}\n"
    );
    let compiled = compile(&source);
    let package = admit_bytes(compiled.emission.package.bytes(), &compiled.wire);
    let node = clause_ids(&package).pop().expect("one temporal clause");
    let view = temporal_subject(&package, &node).expect("aggregate is a readable formula atom");
    let aggregate = view
        .formula_graph()
        .iter()
        .find(|entry| entry.kind() == Some(crate::checked_views::FormulaOperation::Aggregate))
        .expect("one aggregate atom");
    let quire_contract_model::CheckedTerm::Aggregate { members } = aggregate.aggregate.unwrap()
    else {
        panic!("aggregate bindings");
    };
    let names = members
        .iter()
        .filter_map(|member| match member {
            quire_contract_model::CheckedTerm::Binding { name, .. } => Some(name.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        names.join(","),
        "kind,value,window_kind,lower,upper,min_count,comparison,threshold"
    );
    assert_eq!(
        instance_bound_atoms(&view).expect("aggregate binder scan"),
        [true]
    );
}
