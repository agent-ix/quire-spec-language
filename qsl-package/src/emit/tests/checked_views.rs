// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515 (TC-1310): the temporal subject and the checked predicate read from
//! a checked package. Every package is emitted by the S4 emitter and read back
//! through IR's reader; each expectation is read directly from the package's
//! nodes, lock and source map, or from the emitted wire, by the test.

use quire_contract_model::{
    CheckedNodeId, CheckedNodeKind, CheckedOccurrenceRole, CheckedOperationInterval,
    CheckedPackageV2, ClockConfiguration, ClockExactNumber, TemporalForm,
};

use super::protocol_graph::{flow_over, model};
use super::reachability::lower_with;
use super::temporal_clause::{admit, compile, unit, EVENT, INFINITE};
use super::*;
use crate::checked_views::{
    checked_predicate, temporal_subject, Activation, CheckedPredicateView, ClauseOccurrence,
    PredicateKind, PredicateSelection, ViewRefusal,
};

const FIXED: &str = "quire.temporal.fixed-sample.false-extension/v1";
const TIMESTAMPED: &str = "quire.temporal.timestamped-event.finite-window/v1";

const HOLDS: &str = "quire.op.temporal.holds";

/// A unit selecting the ConfigVersion model and one profile alias per
/// profile, then `declarations`.
fn unit_of(profiles: &[(&str, &str)], declarations: &str) -> String {
    let selected: String = profiles
        .iter()
        .map(|(alias, identity)| format!("profile {alias} = \"{identity}\";\n"))
        .collect();
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         {selected}\
         model Config = \"example/config-version\";\n\
         {declarations}\n"
    )
}

/// `temporal name using profile over (c: ConfigVersion) clock "model-steps"
/// activation { body }`.
fn clause(name: &str, profile: &str, activation: &str, body: &str) -> String {
    format!(
        "temporal {name} using {profile} over (c: Config::ConfigVersion) clock \"model-steps\" \
         {activation} {{ {body} }}"
    )
}

/// The `temporal`/`temporal_clause` node ids of `package`, in graph order.
fn clause_ids(package: &CheckedPackageV2) -> Vec<CheckedNodeId> {
    package
        .graph()
        .nodes
        .iter()
        .zip(package.node_kinds())
        .filter(|(_, kind)| **kind == CheckedNodeKind::Temporal(TemporalForm::TemporalClause))
        .map(|(node, _)| node.node_id.clone())
        .collect()
}

/// The only clause of `package`.
fn only_clause(package: &CheckedPackageV2) -> CheckedNodeId {
    let [id] = clause_ids(package).try_into().expect("one clause");
    id
}

/// The wire node whose id serializes as `id`.
fn wire_node<'w>(wire: &'w Value, id: &CheckedNodeId) -> &'w Value {
    let sought = serde_json::to_value(id).expect("an id serializes");
    nodes(wire)
        .iter()
        .find(|node| node["node_id"] == sought)
        .unwrap_or_else(|| panic!("a wire node {sought}"))
}

/// The id a wire value spells.
fn id_of(value: &Value) -> CheckedNodeId {
    serde_json::from_value(value.clone()).expect("a node id")
}

/// The id the reference term `term` names.
fn referenced(term: &Value) -> CheckedNodeId {
    assert_eq!(term["term"], "reference", "{term}");
    id_of(&term["target"])
}

/// The `value`/`parameter` node named `name`.
fn parameter(wire: &Value, name: &str) -> CheckedNodeId {
    let found: Vec<_> = nodes(wire)
        .iter()
        .filter(|node| {
            node["semantic_form"] == "parameter"
                && node["body"]["members"][0]["value"]["value"] == name
        })
        .collect();
    let [node] = found[..] else {
        panic!("one parameter {name}: {found:?}");
    };
    id_of(&node["node_id"])
}

/// The clause-body argument `index` of the wire clause node of `id`.
fn argument<'w>(wire: &'w Value, id: &CheckedNodeId, index: usize) -> &'w Value {
    &wire_node(wire, id)["body"]["arguments"][index]
}

fn text(value: &Value) -> &str {
    value.as_str().expect("text")
}

/// An integer interval's rendered lower and optional upper bound.
type Bounds = Option<(String, Option<String>)>;

/// The integer bounds of `interval`, rendered.
fn bounds(interval: &Option<CheckedOperationInterval>) -> Bounds {
    match interval.as_ref()? {
        CheckedOperationInterval::Integer { lower, upper } => {
            Some((lower.to_string(), upper.as_ref().map(ToString::to_string)))
        }
        timed => panic!("an integer interval: {timed:?}"),
    }
}

/// The ops of `view`'s formula, each with its rendered bounds.
fn ops(view: &crate::checked_views::TemporalSubjectView<'_>) -> Vec<(String, Bounds)> {
    view.formula()
        .iter()
        .map(|used| (used.operation.to_owned(), bounds(&used.interval)))
        .collect()
}

/// FR-515-AC-1 (TC-1310 row 1): for a clause of each of the event-position,
/// fixed-sample and timestamped-event profiles, each accessor equals the value
/// the test reads from the node, lock, source map or `temporal_bindings()`
/// row. A unit selects one temporal profile, so each profile is its own
/// package.
#[trace("FR-515-AC-1", "TC-1310")]
#[test]
fn each_clause_of_the_package_reads_its_node_facts() {
    let cases = [
        (
            EVENT,
            "always[0,30] holds(c.versionNumber <= 1)",
            ClockConfiguration::EventPosition {
                sequence_authority: "test-order-events".into(),
            },
            ("quire.op.temporal.always", ("0", "30")),
        ),
        (
            FIXED,
            "eventually[0,5] holds(c.versionNumber = 2)",
            ClockConfiguration::FixedSample {
                epoch: ClockExactNumber::Integer {
                    decimal: "0".into(),
                },
                period: ClockExactNumber::Rational {
                    numerator: "1".into(),
                    denominator: "10".into(),
                },
                unit: "test-milliseconds".into(),
            },
            ("quire.op.temporal.eventually", ("0", "5")),
        ),
        (
            TIMESTAMPED,
            "always[1,9] holds(c.versionNumber >= 1)",
            ClockConfiguration::TimestampedEvent {
                timestamp_unit: "test-rfc3339".into(),
            },
            ("quire.op.temporal.always", ("1", "9")),
        ),
    ];
    for (profile, body, clock, (operation, (lower, upper))) in cases {
        let compiled = compile(&unit(profile, &clause("Due", "t", "on origin", body)));
        let package = admit(&compiled);
        let wire = &compiled.wire;
        let id = only_clause(&package);
        let view = temporal_subject(&package, &id).expect("the clause reads");
        let node = package
            .graph()
            .nodes
            .iter()
            .find(|node| node.node_id == id)
            .expect("the clause node");
        assert_eq!(*view.declaration(), id, "{profile}");
        let [claim] = &node.occurrences[..] else {
            panic!("one declaration");
        };
        assert_eq!(claim.role, CheckedOccurrenceRole::Claim);
        assert_eq!(
            view.occurrences(),
            [ClauseOccurrence {
                node: id.clone(),
                role: CheckedOccurrenceRole::Claim,
                ordinal: claim.ordinal,
            }],
            "{profile}"
        );
        assert_eq!(*view.over(), referenced(argument(wire, &id, 0)));
        assert_eq!(*view.over(), parameter(wire, "c"));
        assert_eq!(view.profile().definition.identity.as_ref(), profile);
        assert!(package
            .lock()
            .profile_selections
            .iter()
            .any(|row| row == view.profile()));
        let row = package
            .temporal_bindings()
            .expect("a configured clause carries a binding")
            .iter()
            .find(|row| row.declaration == id)
            .expect("this declaration's row");
        assert_eq!(view.clock(), Some(row));
        assert_eq!(row.clock, clock, "{profile}");
        assert_eq!(&*row.clock_name, text(&argument(wire, &id, 1)["value"]));
        assert_eq!(*view.activation(), Activation::Origin);
        assert_eq!(view.captures(), []);
        let source = package
            .source_map()
            .iter()
            .find(|entry| entry.node_id == id && entry.role == CheckedOccurrenceRole::Claim)
            .expect("the claim entry");
        assert_eq!(view.source_span(), source);
        assert_eq!(
            ops(&view),
            [
                (
                    operation.to_owned(),
                    Some((lower.to_owned(), Some(upper.to_owned())))
                ),
                (HOLDS.to_owned(), None)
            ],
            "{profile}"
        );
        assert_eq!(view.predicate_leaves().len(), 1);
    }
    // A clause whose profile selects no clock has no `temporal_bindings()` row.
    let unconfigured = compile(&unit(
        INFINITE,
        &clause(
            "Plain",
            "t",
            "on origin",
            "always holds(c.versionNumber <= 1)",
        ),
    ));
    let package = admit(&unconfigured);
    let view = temporal_subject(&package, &only_clause(&package)).expect("reads");
    assert_eq!(view.clock(), None);
    assert_eq!(package.temporal_bindings(), None);
}

/// FR-515-AC-2 (TC-1310 row 2): `on origin`, `on each (q) when (g)` and
/// `on each (q)` read as `Origin`, `Each` with trigger and guard and `Each`
/// with no guard; the guard reads as `ActivationGuard` and is no predicate
/// leaf.
#[trace("FR-515-AC-2", "FR-515-AC-10", "TC-1310")]
#[test]
fn the_activation_and_its_guard_are_read_from_the_clause() {
    let compiled = compile(&unit_of(
        &[("t", INFINITE)],
        &[
            clause(
                "Origin",
                "t",
                "on origin",
                "always holds(c.versionNumber <= 1)",
            ),
            clause(
                "Guarded",
                "t",
                "on each (q: Config::ConfigVersion) when (c.versionNumber >= 3)",
                "always holds(c.versionNumber <= 2)",
            ),
            clause(
                "Unguarded",
                "t",
                "on each (q: Config::ConfigVersion)",
                "always holds(c.versionNumber <= 4)",
            ),
        ]
        .join("\n"),
    ));
    let package = admit(&compiled);
    let wire = &compiled.wire;
    let activation =
        |id: &CheckedNodeId| temporal_subject(&package, id).unwrap().activation().clone();
    let ids = clause_ids(&package);
    let pick = |wanted: &dyn Fn(&Activation) -> bool| {
        let found: Vec<_> = ids.iter().filter(|id| wanted(&activation(id))).collect();
        let [id] = found[..] else {
            panic!("one clause: {found:?}");
        };
        id.clone()
    };
    let origin = pick(&|a| *a == Activation::Origin);
    let guarded = pick(&|a| matches!(a, Activation::Each { guard: Some(_), .. }));
    let unguarded = pick(&|a| matches!(a, Activation::Each { guard: None, .. }));

    assert_eq!(activation(&origin), Activation::Origin);
    let member = |id: &CheckedNodeId, name: &str| {
        argument(wire, id, 2)["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["name"] == name)
            .map(|member| referenced(&member["value"]))
    };
    let trigger = parameter(wire, "q");
    assert_eq!(
        activation(&guarded),
        Activation::Each {
            trigger: trigger.clone(),
            guard: member(&guarded, "guard"),
        }
    );
    assert!(member(&guarded, "guard").is_some());
    assert_eq!(
        activation(&unguarded),
        Activation::Each {
            trigger,
            guard: None,
        }
    );

    let subject = temporal_subject(&package, &guarded).unwrap();
    let guard = member(&guarded, "guard").unwrap();
    assert!(!subject.predicate_leaves().contains(&guard));
    let view = checked_predicate(
        &package,
        &PredicateSelection {
            clause: guarded.clone(),
            leaf: guard.clone(),
        },
    )
    .expect("the guard reads");
    assert_eq!(view.kind(), PredicateKind::ActivationGuard);
    assert_eq!(*view.leaf(), guard);
    assert_eq!(view.parent_operation(), Some("quire.op.temporal.clause"));
    // The unguarded and origin clauses have no guard to select: the same leaf
    // is read as the clause's own `holds` operand or is not in the clause.
    let holds = temporal_subject(&package, &unguarded)
        .unwrap()
        .predicate_leaves()[0]
        .clone();
    let view = checked_predicate(
        &package,
        &PredicateSelection {
            clause: unguarded.clone(),
            leaf: holds,
        },
    )
    .unwrap();
    assert_eq!(view.kind(), PredicateKind::HoldsLeaf);
    assert_eq!(
        checked_predicate(
            &package,
            &PredicateSelection {
                clause: origin,
                leaf: guard,
            },
        )
        .err(),
        Some(ViewRefusal::LeafNotInClause)
    );
}

/// The right-hand literal of the comparison `leaf`.
fn compared_literal(package: &CheckedPackageV2, leaf: &CheckedNodeId) -> String {
    let quire_contract_model::CheckedTerm::Application { arguments, .. } =
        package.term(leaf).expect("the leaf node")
    else {
        panic!("a comparison");
    };
    let quire_contract_model::CheckedTerm::Reference { target } = &arguments[1] else {
        panic!("a reference operand");
    };
    let literal = package.term(target).expect("the operand node");
    match literal.literal_read().expect("a literal").value {
        quire_contract_model::CheckedLiteralValue::Integer(value) => value.to_string(),
        other => panic!("an integer literal: {other:?}"),
    }
}

/// FR-515-AC-3 (TC-1310 row 3): `always[0,30] holds(...)` reads as `always`
/// with `[0, 30]` then `holds` and one leaf; three `holds` atoms read three
/// leaves in formula pre-order.
#[trace("FR-515-AC-3", "TC-1310")]
#[test]
fn the_formula_lists_its_operators_and_holds_operands_in_pre_order() {
    let compiled = compile(&unit(
        EVENT,
        &clause(
            "Due",
            "t",
            "on origin",
            "always[0,30] holds(c.versionNumber <= 1)",
        ),
    ));
    let package = admit(&compiled);
    let view = temporal_subject(&package, &only_clause(&package)).unwrap();
    assert_eq!(
        ops(&view),
        [
            (
                "quire.op.temporal.always".to_owned(),
                Some(("0".to_owned(), Some("30".to_owned())))
            ),
            (HOLDS.to_owned(), None)
        ]
    );
    assert_eq!(view.predicate_leaves().len(), 1);
    assert_eq!(compared_literal(&package, &view.predicate_leaves()[0]), "1");

    let three = compile(&unit(
        INFINITE,
        &clause(
            "Three",
            "t",
            "on origin",
            "always (holds(c.versionNumber >= 11) and holds(c.versionNumber >= 12) \
             and holds(c.versionNumber >= 13))",
        ),
    ));
    let package = admit(&three);
    let view = temporal_subject(&package, &only_clause(&package)).unwrap();
    assert_eq!(
        ops(&view),
        [
            ("quire.op.temporal.always".to_owned(), None),
            ("quire.op.temporal.and".to_owned(), None),
            ("quire.op.temporal.and".to_owned(), None),
            (HOLDS.to_owned(), None),
            (HOLDS.to_owned(), None),
            (HOLDS.to_owned(), None),
        ]
    );
    let literals: Vec<_> = view
        .predicate_leaves()
        .iter()
        .map(|leaf| compared_literal(&package, leaf))
        .collect();
    assert_eq!(literals, ["11", "12", "13"]);
}

/// FR-515-AC-4 (TC-1310 row 4): the `HoldsLeaf` of `Due` reads with its
/// leaf, clause, the binders it reads and the leaf's own source span.
#[trace("FR-515-AC-4", "TC-1310")]
#[test]
fn a_holds_leaf_reads_its_kind_binders_and_span() {
    let compiled = compile(&unit(
        INFINITE,
        &clause(
            "Due",
            "t",
            "on each (q: Config::ConfigVersion) when (c.versionNumber >= 2)",
            "capture n: Int[0, 1000] = c.versionNumber; \
             always (holds(c.versionNumber >= n) and holds(true))",
        ),
    ));
    let package = admit(&compiled);
    let wire = &compiled.wire;
    let id = only_clause(&package);
    let subject = temporal_subject(&package, &id).unwrap();
    let [reads_n, literal] = subject.predicate_leaves() else {
        panic!("two leaves");
    };
    let view = |leaf: &CheckedNodeId| {
        checked_predicate(
            &package,
            &PredicateSelection {
                clause: id.clone(),
                leaf: leaf.clone(),
            },
        )
        .expect("the leaf reads")
    };
    let first = view(reads_n);
    assert_eq!(first.kind(), PredicateKind::HoldsLeaf);
    assert_eq!(first.leaf(), reads_n);
    assert_eq!(*first.declaration(), id);
    assert_eq!(first.parent_operation(), Some(HOLDS));
    assert_eq!(first.profile(), subject.profile());
    // `c` first, then the capture `n`; the trigger `q` is not read.
    assert_eq!(
        first.binders(),
        [parameter(wire, "c"), parameter(wire, "n")]
    );
    // The clause's claim entry covers its name; the leaf lies inside the
    // source of the formula root that reads it.
    let formula_regions: Vec<_> = package
        .source_map()
        .iter()
        .filter(|entry| entry.node_id == referenced(argument(wire, &id, 5)))
        .flat_map(|entry| entry.regions.iter())
        .collect();
    assert!(!first.source_span().regions.is_empty());
    for region in &first.source_span().regions {
        assert!(
            formula_regions
                .iter()
                .any(|outer| outer.source == region.source
                    && outer.start <= region.start
                    && region.end <= outer.end),
            "{region:?} outside {formula_regions:?}"
        );
    }
    let leaf_entry = package
        .source_map()
        .iter()
        .find(|entry| entry.node_id == *reads_n && entry.regions == first.source_span().regions)
        .expect("the leaf's own source map entry");
    assert_eq!(first.source_span(), leaf_entry);
    let second = view(literal);
    assert_eq!(second.kind(), PredicateKind::HoldsLeaf);
    assert_eq!(second.binders(), []);
    assert!(first.belongs_to(&package));
}

/// The leaf `view` reads.
fn leaf_of(view: &CheckedPredicateView<'_>) -> CheckedNodeId {
    view.leaf().clone()
}

/// FR-515-AC-4: a Boolean sub-term of a `holds` operand that is neither the
/// operand nor the guard reads as a `ClauseLeaf` with the operation that reads
/// it as its parent.
#[trace("FR-515-AC-4", "FR-515-AC-17", "TC-1310")]
#[test]
fn a_boolean_term_below_a_holds_operand_is_a_clause_leaf() {
    let compiled = compile(&unit(
        INFINITE,
        &clause(
            "Nested",
            "t",
            "on origin",
            "always holds((c.versionNumber >= 1) and (c.versionNumber <= 9))",
        ),
    ));
    let package = admit(&compiled);
    let id = only_clause(&package);
    let subject = temporal_subject(&package, &id).unwrap();
    let [operand] = subject.predicate_leaves() else {
        panic!("one leaf");
    };
    let quire_contract_model::CheckedTerm::Application { arguments, .. } =
        package.term(operand).unwrap()
    else {
        panic!("a conjunction");
    };
    let parts: Vec<_> = arguments
        .iter()
        .map(|term| match term {
            quire_contract_model::CheckedTerm::Reference { target } => target.clone(),
            other => panic!("a reference: {other:?}"),
        })
        .collect();
    let operation = |view: &CheckedPredicateView<'_>| view.parent_operation().map(str::to_owned);
    let mut seen = Vec::new();
    for part in parts {
        let view = checked_predicate(
            &package,
            &PredicateSelection {
                clause: id.clone(),
                leaf: part.clone(),
            },
        )
        .expect("a Boolean part reads");
        assert_eq!(view.kind(), PredicateKind::ClauseLeaf);
        assert_eq!(leaf_of(&view), part);
        assert_eq!(operation(&view), Some("quire.op.boolean.and".to_owned()));
        // The entry is the leaf node's own, inside the formula root's source.
        let formula_root = match package.term(view.declaration()).unwrap() {
            quire_contract_model::CheckedTerm::Application { arguments, .. } => {
                match arguments.last().unwrap() {
                    quire_contract_model::CheckedTerm::Reference { target } => target.clone(),
                    other => panic!("a formula reference: {other:?}"),
                }
            }
            other => panic!("a clause application: {other:?}"),
        };
        let outer: Vec<_> = package
            .source_map()
            .iter()
            .filter(|entry| entry.node_id == formula_root)
            .flat_map(|entry| entry.regions.iter())
            .collect();
        assert!(package
            .source_map()
            .iter()
            .any(|entry| entry.node_id == part && view.source_span() == entry));
        assert!(!view.source_span().regions.is_empty());
        for region in &view.source_span().regions {
            assert!(
                outer.iter().any(|root| root.source == region.source
                    && root.start <= region.start
                    && region.end <= root.end),
                "{region:?} outside {outer:?}"
            );
        }
        seen.push(part);
    }
    assert_eq!(seen.len(), 2);
    assert_ne!(seen[0], seen[1]);
}

/// FR-515-AC-5 (TC-1310 row 5): an id that is absent, names a protocol
/// clause or names a temporal node of another form refuses `UnresolvedClause`;
/// a leaf of another clause refuses `LeafNotInClause`, an integer-typed term
/// `LeafNotBoolean`.
#[trace("FR-515-AC-5", "TC-1310")]
#[test]
fn a_selection_the_package_does_not_hold_is_refused() {
    let temporal = clause(
        "Due",
        "t",
        "on origin",
        "always holds(c.versionNumber <= 1)",
    );
    let other = clause(
        "Other",
        "t",
        "on origin",
        "always holds(c.versionNumber >= 7)",
    );
    let compiled = compile(&unit_of(
        &[("t", INFINITE)],
        &format!("{temporal}\n{other}"),
    ));
    let package = admit(&compiled);
    let [due, other] = clause_ids(&package).try_into().expect("two clauses");
    let due_leaf = temporal_subject(&package, &due).unwrap().predicate_leaves()[0].clone();
    let other_leaf = temporal_subject(&package, &other)
        .unwrap()
        .predicate_leaves()[0]
        .clone();

    // A temporal node of another form: the clause's formula root.
    let formula_root = referenced(argument(&compiled.wire, &due, 5));
    for not_a_clause in [formula_root, due_leaf.clone()] {
        assert_eq!(
            temporal_subject(&package, &not_a_clause).err(),
            Some(ViewRefusal::UnresolvedClause)
        );
    }
    // An id the package does not hold: a clause of a different package.
    let foreign = compile(&unit(
        INFINITE,
        &clause(
            "Foreign",
            "t",
            "on origin",
            "always holds(c.versionNumber >= 99)",
        ),
    ));
    let foreign_package = admit(&foreign);
    let foreign_id = only_clause(&foreign_package);
    assert!(!clause_ids(&package).contains(&foreign_id));
    assert_eq!(
        temporal_subject(&package, &foreign_id).err(),
        Some(ViewRefusal::UnresolvedClause)
    );
    assert_eq!(
        checked_predicate(
            &package,
            &PredicateSelection {
                clause: foreign_id,
                leaf: due_leaf.clone(),
            },
        )
        .err(),
        Some(ViewRefusal::UnresolvedClause)
    );
    // A leaf of another clause.
    assert_eq!(
        checked_predicate(
            &package,
            &PredicateSelection {
                clause: due.clone(),
                leaf: other_leaf,
            },
        )
        .err(),
        Some(ViewRefusal::LeafNotInClause)
    );
    // An integer-typed term of the clause: the field read `c.versionNumber`.
    let quire_contract_model::CheckedTerm::Application { arguments, .. } =
        package.term(&due_leaf).unwrap()
    else {
        panic!("a comparison");
    };
    let integer = match &arguments[0] {
        quire_contract_model::CheckedTerm::Reference { target } => target.clone(),
        other => panic!("a reference: {other:?}"),
    };
    assert_eq!(
        checked_predicate(
            &package,
            &PredicateSelection {
                clause: due,
                leaf: integer,
            },
        )
        .err(),
        Some(ViewRefusal::LeafNotBoolean)
    );
}

/// FR-515-AC-5: the id of a `protocol_clause` is no temporal clause, and the
/// clause a protocol requires names that protocol in `requirements()`.
#[trace("FR-515-AC-5", "FR-515-AC-15", "TC-1310")]
#[test]
fn a_protocol_clause_is_no_temporal_clause_and_requirements_name_it() {
    const PROFILES: &str = "profile p = \"quire.protocol.finite-global/v1\";\n\
                            profile t = \"quire.temporal.infinite-trace/v1\";\n";
    let temporal = "temporal Due using t over (c: M::Order) clock \"model-steps\" \
                    on origin { always true }\n\
                    temporal Free using t over (c: M::Order) clock \"model-steps\" \
                    on origin { always holds(true) }";
    let protocol = flow_over(
        "M::Order",
        "on origin",
        "role Buyer on M::Order;\nrequires temporal Due;",
    );
    let lowered = lower_with(model(), PROFILES, &format!("{temporal}\n{protocol}"));
    let package: &CheckedPackageV2 = lowered.ir.as_ref().expect("the package is admitted");
    let protocol_ids: Vec<CheckedNodeId> = package
        .graph()
        .nodes
        .iter()
        .zip(package.node_kinds())
        .filter(|(_, kind)| matches!(kind, CheckedNodeKind::Protocol(_)))
        .filter(|(node, _)| &*node.semantic_form == "protocol_clause")
        .map(|(node, _)| node.node_id.clone())
        .collect();
    let [protocol_id] = protocol_ids.try_into().expect("one protocol clause");
    assert_eq!(
        temporal_subject(package, &protocol_id).err(),
        Some(ViewRefusal::UnresolvedClause)
    );
    let requiring = |clause_name_leaf_count: usize| {
        clause_ids(package)
            .into_iter()
            .filter(|id| {
                temporal_subject(package, id)
                    .unwrap()
                    .predicate_leaves()
                    .len()
                    == clause_name_leaf_count
            })
            .collect::<Vec<_>>()
    };
    // `Due` (`always true`) carries no `holds`; `Free` carries one.
    let [due] = requiring(0).try_into().expect("Due");
    let [free] = requiring(1).try_into().expect("Free");
    assert_eq!(
        temporal_subject(package, &due).unwrap().requirements(),
        [&protocol_id]
    );
    assert_eq!(
        temporal_subject(package, &free).unwrap().requirements(),
        [] as [&CheckedNodeId; 0]
    );
}

/// FR-515-AC-6 (TC-1310 row 6): a view answers `belongs_to` for the package
/// it was read from and not for a second package emitted from the same source.
#[trace("FR-515-AC-6", "TC-1310")]
#[test]
fn a_view_belongs_to_the_package_it_was_read_from() {
    let source = unit(
        INFINITE,
        &clause(
            "Due",
            "t",
            "on origin",
            "always holds(c.versionNumber <= 1)",
        ),
    );
    let first = admit(&compile(&source));
    let second = admit(&compile(&source));
    assert_eq!(
        first.as_ref(),
        second.as_ref(),
        "the same source, the same package"
    );
    let view = temporal_subject(&first, &only_clause(&first)).unwrap();
    assert!(view.belongs_to(&first));
    assert!(!view.belongs_to(&second));
    let other = temporal_subject(&second, &only_clause(&second)).unwrap();
    assert!(other.belongs_to(&second));
    assert!(!other.belongs_to(&first));
}

/// FR-515-AC-7 (TC-1310 row 7): two structurally equal clauses are one node
/// with two occurrences of different ordinals; a changed interval changes the
/// formula and the node's structural key; a source comment changes no id.
#[trace("FR-515-AC-7", "TC-1310")]
#[test]
fn equal_clauses_share_a_node_and_an_interval_or_comment_decides_identity() {
    let body = "eventually[3,*] holds(c.versionNumber <= 1)";
    let twins = |first: &str, second: &str| {
        format!(
            "{}\n{}",
            clause("First", "t", "on origin", first),
            clause("Second", "t", "on origin", second)
        )
    };
    let shared = admit(&compile(&unit(INFINITE, &twins(body, body))));
    let [node] = clause_ids(&shared).try_into().expect("one shared node");
    let view = temporal_subject(&shared, &node).unwrap();
    let ordinals: Vec<_> = view.occurrences().iter().map(|o| o.ordinal).collect();
    assert_eq!(view.occurrences().len(), 2);
    assert_ne!(ordinals[0], ordinals[1]);
    assert!(view.occurrences().iter().all(|o| o.node == node));

    let changed_body = "eventually[4,*] holds(c.versionNumber <= 1)";
    let changed = admit(&compile(&unit(INFINITE, &twins(body, changed_body))));
    assert_eq!(clause_ids(&changed).len(), 2);
    let mut by_lower = Vec::new();
    for id in clause_ids(&changed) {
        let view = temporal_subject(&changed, &id).unwrap();
        assert_eq!(view.occurrences().len(), 1);
        by_lower.push((bounds(&view.formula()[0].interval), id));
    }
    by_lower.sort_by(|left, right| left.0.cmp(&right.0));
    let [(three, three_id), (four, four_id)] = &by_lower[..] else {
        panic!("two clauses: {by_lower:?}");
    };
    assert_eq!(*three, Some(("3".to_owned(), None)));
    assert_eq!(*four, Some(("4".to_owned(), None)));
    let original = temporal_subject(&shared, &node).unwrap();
    assert_eq!(bounds(&original.formula()[0].interval), *three);
    // The interval is part of the node's key: the unchanged clause keeps the
    // shared node's key and the changed one has another.
    assert_eq!(
        changed.structural_key(three_id),
        shared.structural_key(&node)
    );
    assert_ne!(
        changed.structural_key(four_id),
        shared.structural_key(&node)
    );

    let source = unit(INFINITE, &clause("Due", "t", "on origin", body));
    let commented = source.replace("eventually[3,*]", "// a comment\n eventually[3,*]");
    assert_ne!(source, commented);
    let plain = admit(&compile(&source));
    let with_comment = admit(&compile(&commented));
    let (plain_id, commented_id) = (only_clause(&plain), only_clause(&with_comment));
    let (a, b) = (
        temporal_subject(&plain, &plain_id).unwrap(),
        temporal_subject(&with_comment, &commented_id).unwrap(),
    );
    assert_eq!(a.declaration(), b.declaration());
    assert_eq!(a.over(), b.over());
    assert_eq!(a.predicate_leaves(), b.predicate_leaves());
    assert_eq!(a.activation(), b.activation());
    assert_eq!(ops(&a), ops(&b));
    assert_eq!(
        plain.structural_key(&plain_id),
        with_comment.structural_key(&commented_id)
    );
    // The comment moved the source region the leaf is read at, not its id.
    let leaf_span = |package: &CheckedPackageV2, id: &CheckedNodeId, leaf: &CheckedNodeId| {
        checked_predicate(
            package,
            &PredicateSelection {
                clause: id.clone(),
                leaf: leaf.clone(),
            },
        )
        .unwrap()
        .source_span()
        .regions
        .clone()
    };
    assert_ne!(
        leaf_span(&plain, &plain_id, &a.predicate_leaves()[0]),
        leaf_span(&with_comment, &commented_id, &b.predicate_leaves()[0])
    );
}

/// FR-515-AC-14 (TC-1310): a shared declaration resolves its name only by
/// occurrence identity; a missing ordinal does not borrow either sibling's
/// name, while a unique declaration and its predicate view retain theirs.
#[trace("FR-515-AC-14", "TC-1310")]
#[test]
fn temporal_clause_names_are_selected_by_declaration_and_occurrence() {
    let body = "eventually[0,1] holds(c.versionNumber = 2)";
    let twins = format!(
        "{}\n{}",
        clause("Alpha", "e", "on origin", body),
        clause("Beta", "e", "on origin", body)
    );
    let shared = admit(&compile(&unit_of(&[("e", EVENT)], &twins)));
    let [shared_id] = clause_ids(&shared).try_into().expect("one shared node");
    let shared_view = temporal_subject(&shared, &shared_id).expect("shared clause view");
    let [alpha, beta] = shared_view.occurrences() else {
        panic!("two claim occurrences: {:?}", shared_view.occurrences());
    };
    let names = shared
        .temporal_clause_names()
        .expect("the emitted name rows");
    assert_eq!(shared_view.name(), None);
    for (occurrence, expected) in [(alpha, "Alpha"), (beta, "Beta")] {
        let row_name = names
            .iter()
            .find(|row| row.declaration == shared_id && row.occurrence == occurrence.ordinal)
            .map(|row| row.name.as_ref());
        assert_eq!(row_name, Some(expected));
        assert_eq!(shared_view.name_of(occurrence), row_name);
    }
    let absent = ClauseOccurrence {
        node: shared_id.clone(),
        role: CheckedOccurrenceRole::Claim,
        ordinal: 7,
    };
    assert_eq!(shared_view.name_of(&absent), None);

    let gamma = admit(&compile(&unit_of(
        &[("e", EVENT)],
        &clause(
            "Gamma",
            "e",
            "on each (q: Config::ConfigVersion) when (c.versionNumber >= 3)",
            body,
        ),
    )));
    let [gamma_id] = clause_ids(&gamma).try_into().expect("one Gamma clause");
    let gamma_view = temporal_subject(&gamma, &gamma_id).expect("Gamma view");
    assert_eq!(gamma_view.name(), Some("Gamma"));
    assert_eq!(
        gamma_view.name_of(&gamma_view.occurrences()[0]),
        Some("Gamma")
    );
    let Activation::Each {
        guard: Some(guard), ..
    } = gamma_view.activation().clone()
    else {
        panic!("Gamma has a guarded activation");
    };
    let activation_guard = checked_predicate(
        &gamma,
        &PredicateSelection {
            clause: gamma_id.clone(),
            leaf: guard,
        },
    )
    .expect("Gamma activation guard view");
    assert_eq!(activation_guard.kind(), PredicateKind::ActivationGuard);
    assert_eq!(activation_guard.name(), Some("Gamma"));

    let leaf = gamma_view.predicate_leaves()[0].clone();
    let predicate = checked_predicate(
        &gamma,
        &PredicateSelection {
            clause: gamma_id,
            leaf,
        },
    )
    .expect("Gamma predicate view");
    assert_eq!(predicate.name(), Some("Gamma"));
    assert_eq!(
        predicate.name_of(&gamma_view.occurrences()[0]),
        Some("Gamma")
    );
}

/// FR-515 "Determinism": the view read from the emitted bytes is the same on
/// a second read of the same bytes.
#[trace("FR-515-AC-1", "FR-515-AC-7", "TC-1310")]
#[test]
fn a_second_read_of_the_same_bytes_gives_the_same_view() {
    let compiled = compile(&unit(
        EVENT,
        &clause(
            "Due",
            "t",
            "on origin",
            "always[0,30] holds(c.versionNumber <= 1)",
        ),
    ));
    let (first, second) = (admit(&compiled), admit(&compiled));
    for (a, b) in clause_ids(&first).iter().zip(clause_ids(&second).iter()) {
        let (left, right) = (
            temporal_subject(&first, a).unwrap(),
            temporal_subject(&second, b).unwrap(),
        );
        assert_eq!(left.declaration(), right.declaration());
        assert_eq!(left.occurrences(), right.occurrences());
        assert_eq!(left.over(), right.over());
        assert_eq!(left.profile(), right.profile());
        assert_eq!(left.clock(), right.clock());
        assert_eq!(left.activation(), right.activation());
        assert_eq!(left.predicate_leaves(), right.predicate_leaves());
        assert_eq!(ops(&left), ops(&right));
        assert!(left.belongs_to(&first) && !left.belongs_to(&second));
    }
}

/// FR-515-AC-13 (TC-1310 row 13): two structurally equal clauses under a
/// clock-configured profile are one node with two occurrences of different
/// ordinals and one `temporal_bindings` row per occurrence, admitted by the
/// reader. `clock()` is `None` for a node with several occurrences
/// (FR-515-AC-19's rule) and `clock_of` returns each occurrence's row.
#[trace("FR-515-AC-13", "TC-1310")]
#[test]
fn equal_clauses_under_a_clock_profile_share_a_node_with_two_occurrences() {
    let body = "always[0,30] holds(c.versionNumber <= 1)";
    let declarations = format!(
        "{}\n{}",
        clause("First", "t", "on origin", body),
        clause("Second", "t", "on origin", body)
    );
    let package = admit(&compile(&unit(EVENT, &declarations)));
    let [node] = clause_ids(&package).try_into().expect("one shared node");
    let view = temporal_subject(&package, &node).unwrap();
    let ordinals: Vec<_> = view.occurrences().iter().map(|o| o.ordinal).collect();
    assert_eq!(ordinals.len(), 2);
    assert_ne!(ordinals[0], ordinals[1]);
    assert!(view.occurrences().iter().all(|o| o.node == node));
    assert_eq!(view.clock(), None);
    for occurrence in view.occurrences() {
        let row = view.clock_of(occurrence).expect("one row per occurrence");
        assert_eq!(row.declaration, node);
        assert_eq!(row.occurrence, occurrence.ordinal);
    }
}

/// FR-515-AC-11 (TC-1310 row 11): a guard equal to a `holds` operand is one
/// node; it reads as `ActivationGuard` and is among `predicate_leaves()` once
/// per `holds` atom that reads it.
#[trace("FR-515-AC-11", "TC-1310")]
#[test]
fn a_guard_equal_to_a_holds_operand_is_one_node_in_both_places() {
    let compiled = compile(&unit(
        INFINITE,
        &clause(
            "Shared",
            "t",
            "on each (q: Config::ConfigVersion) when (c.versionNumber >= 3)",
            "always (holds(c.versionNumber >= 3) and holds(c.versionNumber >= 3))",
        ),
    ));
    let package = admit(&compiled);
    let id = only_clause(&package);
    let subject = temporal_subject(&package, &id).unwrap();
    let Activation::Each {
        guard: Some(guard), ..
    } = subject.activation().clone()
    else {
        panic!("a guarded activation");
    };
    assert_eq!(subject.predicate_leaves(), [guard.clone(), guard.clone()]);
    let view = checked_predicate(
        &package,
        &PredicateSelection {
            clause: id,
            leaf: guard,
        },
    )
    .unwrap();
    assert_eq!(view.kind(), PredicateKind::ActivationGuard);
}

/// FR-515-AC-12 (TC-1310 row 12): a leaf the clause reads whose source map
/// entry is absent refuses `LeafHasNoSource`.
#[trace("FR-515-AC-12", "TC-1310")]
#[test]
fn a_clause_leaf_without_a_source_entry_is_refused() {
    let compiled = compile(&unit(
        INFINITE,
        &clause(
            "Due",
            "t",
            "on origin",
            "always holds(c.versionNumber <= 1)",
        ),
    ));
    let package = admit(&compiled);
    let id = only_clause(&package);
    let leaf = temporal_subject(&package, &id).unwrap().predicate_leaves()[0].clone();
    let selection = PredicateSelection {
        clause: id,
        leaf: leaf.clone(),
    };
    assert!(checked_predicate(&package, &selection).is_ok());

    // An admitted package holds an entry for every occurrence, so the refusal
    // is read over the package's source map without the leaf's entries.
    let without: Vec<_> = package
        .source_map()
        .iter()
        .filter(|entry| entry.node_id != leaf)
        .cloned()
        .collect();
    assert!(without.len() < package.source_map().len());
    assert_eq!(
        crate::checked_views::predicate::read_predicate(&package, &without, &selection).err(),
        Some(ViewRefusal::LeafHasNoSource)
    );
}

/// FR-515-AC-16 (TC-1310 row 16): the `source_span` of an activation guard is
/// one of the source map entries of the guard node.
#[trace("FR-515-AC-16", "TC-1310")]
#[test]
fn an_activation_guard_reads_a_source_entry_of_its_node() {
    let compiled = compile(&unit(
        INFINITE,
        &clause(
            "Guarded",
            "t",
            "on each (q: Config::ConfigVersion) when (c.versionNumber >= 3)",
            "always holds(c.versionNumber <= 2)",
        ),
    ));
    let package = admit(&compiled);
    let id = only_clause(&package);
    let Activation::Each {
        guard: Some(guard), ..
    } = temporal_subject(&package, &id)
        .unwrap()
        .activation()
        .clone()
    else {
        panic!("a guarded activation");
    };
    let view = checked_predicate(
        &package,
        &PredicateSelection {
            clause: id,
            leaf: guard.clone(),
        },
    )
    .unwrap();
    assert_eq!(view.kind(), PredicateKind::ActivationGuard);
    let entries: Vec<_> = package
        .source_map()
        .iter()
        .filter(|entry| entry.node_id == guard)
        .collect();
    assert!(!entries.is_empty());
    assert!(entries.contains(&view.source_span()));
}

/// The Boolean `holds` operand every clause of the shared-operand tests reads.
const SHARED_OPERAND: &str = "c.versionNumber <= 1";

/// The source regions of the formula root node of `clause`.
fn formula_regions(
    package: &CheckedPackageV2,
    clause: &CheckedNodeId,
) -> Vec<quire_contract_model::CheckedSourceRegion> {
    let quire_contract_model::CheckedTerm::Application { arguments, .. } =
        package.term(clause).unwrap()
    else {
        panic!("a clause application");
    };
    let quire_contract_model::CheckedTerm::Reference { target } = arguments.last().unwrap() else {
        panic!("a formula reference");
    };
    package
        .source_map()
        .iter()
        .filter(|entry| entry.node_id == *target)
        .flat_map(|entry| entry.regions.iter().cloned())
        .collect()
}

fn inside(
    inner: &[quire_contract_model::CheckedSourceRegion],
    outer: &[quire_contract_model::CheckedSourceRegion],
) -> bool {
    !inner.is_empty()
        && inner.iter().all(|region| {
            outer.iter().any(|root| {
                root.source == region.source && root.start <= region.start && region.end <= root.end
            })
        })
}

/// FR-515-AC-4 and AC-17: a `holds` operand two clauses share is one node with
/// an entry per reading, and each clause's view picks the entry inside its own
/// formula root's source.
#[trace("FR-515-AC-4", "FR-515-AC-17", "TC-1310")]
#[test]
fn a_shared_operand_reads_the_span_inside_its_own_clause() {
    let compiled = compile(&unit_of(
        &[("t", INFINITE)],
        &[
            clause(
                "First",
                "t",
                "on origin",
                &format!("always holds({SHARED_OPERAND})"),
            ),
            clause(
                "Second",
                "t",
                "on origin",
                &format!("eventually holds({SHARED_OPERAND})"),
            ),
        ]
        .join("\n"),
    ));
    let package = admit(&compiled);
    let [first, second] = clause_ids(&package).try_into().expect("two clauses");
    let leaf_of =
        |id: &CheckedNodeId| temporal_subject(&package, id).unwrap().predicate_leaves()[0].clone();
    let leaf = leaf_of(&first);
    assert_eq!(leaf, leaf_of(&second), "the operand is one shared node");
    assert!(
        package
            .source_map()
            .iter()
            .filter(|entry| entry.node_id == leaf)
            .count()
            >= 2,
        "one entry per reading"
    );
    let mut spans = Vec::new();
    for id in [&first, &second] {
        let view = checked_predicate(
            &package,
            &PredicateSelection {
                clause: id.clone(),
                leaf: leaf.clone(),
            },
        )
        .unwrap();
        assert_eq!(view.kind(), PredicateKind::HoldsLeaf);
        assert!(
            inside(&view.source_span().regions, &formula_regions(&package, id)),
            "the span lies inside this clause's own formula root"
        );
        spans.push(view.source_span().regions.clone());
    }
    assert_ne!(spans[0], spans[1]);
}

/// FR-515-AC-3: a formula node shared by several parents is read once per
/// parent, so a package of `n` formula nodes lists more than `n` applications,
/// and an expansion past the budget refuses `FormulaTooLarge` rather than
/// growing without bound.
#[trace("FR-515-AC-3", "FR-515-AC-31", "TC-1310")]
#[test]
fn a_formula_whose_expansion_exceeds_the_budget_is_refused() {
    let atom = format!("holds({SHARED_OPERAND})");
    let compiled = compile(&unit(
        INFINITE,
        &clause(
            "Doubled",
            "t",
            "on origin",
            &format!("always (({atom} and {atom}) and ({atom} and {atom}))"),
        ),
    ));
    let package = admit(&compiled);
    let id = only_clause(&package);
    let formula_nodes = package
        .node_kinds()
        .iter()
        .filter(|kind| **kind == CheckedNodeKind::Temporal(TemporalForm::Formula))
        .count();
    assert_eq!(formula_nodes, 4, "always, two levels of and, one holds");
    let view = temporal_subject(&package, &id).unwrap();
    assert_eq!(view.formula().len(), 8, "each shared use is listed");
    assert_eq!(view.predicate_leaves().len(), 4);
    assert!(crate::checked_views::temporal_subject_within(&package, &id, 8).is_ok());
    assert_eq!(
        crate::checked_views::temporal_subject_within(&package, &id, 7).err(),
        Some(ViewRefusal::FormulaTooLarge)
    );
}

/// FR-515-AC-2 and AC-5: a Boolean sub-term of the activation guard is a term
/// the clause reads, so it selects as a `ClauseLeaf`; a sub-term of another
/// clause's guard is not.
#[trace("FR-515-AC-5", "FR-515-AC-2", "TC-1310")]
#[test]
fn a_sub_term_of_the_activation_guard_is_a_clause_leaf() {
    let compiled = compile(&unit_of(
        &[("t", INFINITE)],
        &[
            clause(
                "Guarded",
                "t",
                "on each (q: Config::ConfigVersion) when ((c.versionNumber >= 3) and (c.versionNumber <= 9))",
                "always holds(c.versionNumber <= 2)",
            ),
            clause(
                "Plain",
                "t",
                "on origin",
                "always holds(c.versionNumber <= 4)",
            ),
        ]
        .join("\n"),
    ));
    let package = admit(&compiled);
    let ids = clause_ids(&package);
    let guarded = ids
        .iter()
        .find(|id| {
            matches!(
                temporal_subject(&package, id).unwrap().activation(),
                Activation::Each { .. }
            )
        })
        .unwrap()
        .clone();
    let plain = ids.iter().find(|id| **id != guarded).unwrap().clone();
    let Activation::Each {
        guard: Some(guard), ..
    } = temporal_subject(&package, &guarded)
        .unwrap()
        .activation()
        .clone()
    else {
        panic!("a guard");
    };
    let quire_contract_model::CheckedTerm::Application { arguments, .. } =
        package.term(&guard).unwrap()
    else {
        panic!("a conjunction");
    };
    for argument in arguments {
        let quire_contract_model::CheckedTerm::Reference { target } = argument else {
            panic!("a reference operand");
        };
        let view = checked_predicate(
            &package,
            &PredicateSelection {
                clause: guarded.clone(),
                leaf: target.clone(),
            },
        )
        .expect("a guard part reads");
        assert_eq!(view.kind(), PredicateKind::ClauseLeaf);
        assert_eq!(view.parent_operation(), Some("quire.op.boolean.and"));
        assert_eq!(
            checked_predicate(
                &package,
                &PredicateSelection {
                    clause: plain.clone(),
                    leaf: target.clone(),
                },
            )
            .err(),
            Some(ViewRefusal::LeafNotInClause)
        );
    }
}

/// Two structurally equal clauses `Alpha` and `Beta` (one shared node, two
/// occurrences) and the same source built twice.
fn twin_source() -> String {
    let body = "eventually[3,*] holds(c.versionNumber <= 1)";
    unit(
        INFINITE,
        &format!(
            "{}\n{}",
            clause("Alpha", "t", "on origin", body),
            clause("Beta", "t", "on origin", body)
        ),
    )
}

/// FR-515-AC-29 (TC-1310): every authored occurrence has its own handle,
/// `occurrence_of` returns the key `arena_of` was given, two equal clauses
/// that share a node keep both keys and both handles, and a second build of
/// the same source gives the same keys and handles.
#[trace("FR-515-AC-29")]
#[test]
fn every_occurrence_has_a_distinct_handle_that_reads_back_its_key() {
    let package = admit(&compile(&twin_source()));
    let id = only_clause(&package);
    let view = temporal_subject(&package, &id).unwrap();
    let [first, second] = view.occurrences() else {
        panic!("two occurrences: {:?}", view.occurrences());
    };
    let (first_handle, second_handle) = (
        view.arena_of(first).unwrap(),
        view.arena_of(second).unwrap(),
    );
    assert_ne!(first_handle, second_handle);
    assert_ne!(first.ordinal, second.ordinal);
    assert_eq!(view.occurrence_of(first_handle.clone()).unwrap(), *first);
    assert_eq!(view.occurrence_of(second_handle.clone()).unwrap(), *second);
    assert_eq!(first.node, second.node, "one shared node keeps both keys");

    let again = admit(&compile(&twin_source()));
    let again_view = temporal_subject(&again, &only_clause(&again)).unwrap();
    assert_eq!(again_view.occurrences(), view.occurrences());
    assert_eq!(again_view.arena_of(first).unwrap(), first_handle);
    assert_eq!(again_view.arena_of(second).unwrap(), second_handle);
    assert_eq!(again_view.occurrence_of(second_handle).unwrap(), *second);

    let single = admit(&compile(&unit(
        INFINITE,
        &clause(
            "Due",
            "t",
            "on origin",
            "eventually[3,*] holds(c.versionNumber <= 1)",
        ),
    )));
    let single_view = temporal_subject(&single, &only_clause(&single)).unwrap();
    let [only] = single_view.occurrences() else {
        panic!("one occurrence");
    };
    let handle = single_view.arena_of(only).unwrap();
    assert_eq!(single_view.occurrence_of(handle).unwrap(), *only);
}

/// FR-515-AC-30 (TC-1310): a key the declaration does not hold, and a handle
/// no row declares, are refused with the named variant and return no row.
#[trace("FR-515-AC-30")]
#[test]
fn an_undeclared_key_or_handle_is_refused() {
    let package = admit(&compile(&twin_source()));
    let id = only_clause(&package);
    let view = temporal_subject(&package, &id).unwrap();
    let last = view.occurrences().iter().map(|o| o.ordinal).max().unwrap();
    let past = ClauseOccurrence {
        node: id.clone(),
        role: CheckedOccurrenceRole::Claim,
        ordinal: last + 1,
    };
    assert_eq!(
        view.arena_of(&past),
        Err(ViewRefusal::OccurrenceNotDeclared)
    );
    let wrong_role = ClauseOccurrence {
        role: CheckedOccurrenceRole::Declaration,
        ..view.occurrences()[0].clone()
    };
    assert_eq!(
        view.arena_of(&wrong_role),
        Err(ViewRefusal::OccurrenceNotDeclared)
    );

    // Two distinct clauses: the key of the other declaration is refused, and
    // so is the handle the other declaration's view made.
    let body = |bound: u32| format!("eventually[{bound},*] holds(c.versionNumber <= 1)");
    let distinct = admit(&compile(&unit(
        INFINITE,
        &format!(
            "{}\n{}",
            clause("Alpha", "t", "on origin", &body(3)),
            clause("Beta", "t", "on origin", &body(4))
        ),
    )));
    let [left_id, right_id] = clause_ids(&distinct).try_into().expect("two clauses");
    let left = temporal_subject(&distinct, &left_id).unwrap();
    let right = temporal_subject(&distinct, &right_id).unwrap();
    assert_eq!(
        left.arena_of(&right.occurrences()[0]),
        Err(ViewRefusal::OccurrenceNotDeclared)
    );
    let foreign = right.arena_of(&right.occurrences()[0]).unwrap();
    assert_eq!(
        left.occurrence_of(foreign),
        Err(ViewRefusal::ArenaHandleNotDeclared)
    );

    // A handle past the last row of the same declaration: the shared node of
    // `twin_source` built with one occurrence has one row, so the second
    // occurrence's handle is one past it.
    let one = admit(&compile(&unit(
        INFINITE,
        &clause("Alpha", "t", "on origin", &body(3)),
    )));
    let one_view = temporal_subject(&one, &only_clause(&one)).unwrap();
    assert_eq!(one_view.occurrences().len(), 1);
    let second = view.arena_of(&view.occurrences()[1]).unwrap();
    assert_eq!(
        one_view.occurrence_of(second),
        Err(ViewRefusal::ArenaHandleNotDeclared),
        "slot one is past the single row of the same declaration"
    );
}

/// FR-515-AC-31 (TC-1310, inspection): the relation and the other owner reads
/// take no budget, limit or receipt parameter and return none. The explicit
/// budget of `a_formula_whose_expansion_exceeds_the_budget_is_refused` is the
/// test-only entry to the same walk the default bounds.
#[trace("FR-515-AC-31")]
#[test]
fn the_owner_reads_take_no_budget_and_return_no_receipt() {
    use crate::checked_views::{
        mapping_support, ArenaHandle, Classification, Closure, MappingRefusal,
    };
    let package = admit(&compile(&twin_source()));
    let id = only_clause(&package);
    // Each call below passes exactly the parameters the requirement lists and
    // binds a bare `Result` with no receipt in it.
    let view = temporal_subject(&package, &id).expect("the view");
    let key = view.occurrences()[0].clone();
    let handle: Result<ArenaHandle, ViewRefusal> = view.arena_of(&key);
    let back: Result<ClauseOccurrence, ViewRefusal> = view.occurrence_of(handle.expect("declared"));
    assert_eq!(back, Ok(key));
    let support: Result<Classification, MappingRefusal> = mapping_support(&view, Closure::Complete);
    assert!(support.is_ok());
}
