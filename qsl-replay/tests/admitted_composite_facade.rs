// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1090 (FR-396-AC-2 to FR-396-AC-4): `compile_admitted`'s context holds
//! every record and tuple declaration the check admitted, with the keys the
//! spine's own checked graph minted, and refuses as typed. Only the oracle
//! that produces the expected keys runs the spine directly. FR-396-AC-1 (the
//! crate root alone) is `admitted_composite_root_only.rs`.

use ix_trace_rs::trace;
use qsl_foundation::Setting;
use qsl_replay::spine::{self, ParseRequest, SpineLimits};
use qsl_replay::{
    compile_admitted, AdmittedCompilation, AdmittedCompositeForm, AdmittedLookupRefusal,
    DependencyInput, NodeKey, ReplayLimits, SourceIdentity, StageLimits, SuppliedLibrary,
    WireNodeId,
};
use std::collections::BTreeSet;

use qsl_semantics::check::{CheckedGraph, CheckedTypeNode, CompositeForm, NodeTag};
use quire_exact::Cancel;
use quire_exact::ValueType;
use quire_semantic_value::declaration::CompositeShape;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
// Two same-shaped records and two same-shaped tuples, plus an enum, a union
// and a function: the nodes a composite lookup refuses as the wrong kind.
const UNIT: &str = "record Left { a: Boolean; b: Boolean; }\n\
     record Right { a: Boolean; b: Boolean; }\n\
     tuple First(Boolean, Boolean);\n\
     tuple Second(Boolean, Boolean);\n\
     enum Color { BLUE, RED }\n\
     union Shape { Empty, Circle(Integer) }\n\
     function is_blue using v(c: Color): Boolean pure { false }\n\
     function uses_left using v(x: Left): Boolean pure { x.a }\n";
const POINTS: &str = "record Point { x: Boolean; }\n";

fn unit_identity() -> SourceIdentity {
    SourceIdentity::new("a", "u", "git", "1")
}

fn limits() -> StageLimits {
    [(Setting::S1InputBytes, 1 << 20)].into_iter().collect()
}

fn wire(key: NodeKey) -> WireNodeId {
    WireNodeId::from_digest(*key.as_bytes())
}

fn admit(text: &str, dependencies: &DependencyInput) -> AdmittedCompilation {
    compile_admitted(
        unit_identity(),
        "unit.native",
        text.as_bytes(),
        [],
        dependencies,
        limits(),
        ReplayLimits::default(),
    )
    .expect("the unit compiles")
}

fn library(identity: &str, name: &str, body: &str) -> SuppliedLibrary {
    SuppliedLibrary {
        identity: identity.to_owned(),
        source: SourceIdentity::new("a", name, "git", "1"),
        path: format!("{name}.native"),
        bytes: format!("{HEADER}{body}").into_bytes(),
    }
}

/// Run `read` over the spine's own checked graph of `text`.
fn with_spine_graph<T>(text: &str, read: impl FnOnce(&CheckedGraph) -> T) -> T {
    let source = unit_identity();
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("parses")
    .into_value();
    let models = spine::select(&parsed, &Default::default(), limits.model, &cancel)
        .expect("selects")
        .into_value();
    let checked = spine::check(
        &parsed,
        &models,
        &DependencyInput::default(),
        &spine::LockEvidence::default(),
        limits,
        &cancel,
    )
    .expect("checks")
    .into_value();
    read(checked.package().graph())
}

/// One record or tuple declaration as the spine's own checked package holds
/// it: the id of its `composite_type` semantic node, the key its checked
/// `TypeEnvironment` gives it (the key replay converts arguments against),
/// its name and form. Read from the type environment and the semantic graph,
/// not from the checked type nodes the facade reads.
type Expected = (NodeKey, NodeKey, String, AdmittedCompositeForm);

/// Every record and tuple declaration of `graph`, ascending by node id.
fn spine_composites(graph: &CheckedGraph) -> Vec<Expected> {
    let mut composites = Vec::new();
    for declaration in graph.scope().types().composites() {
        let form = match declaration.shape() {
            CompositeShape::Record(_) => AdmittedCompositeForm::Record,
            CompositeShape::Tuple(_) => AdmittedCompositeForm::Tuple,
            CompositeShape::Union(_) => continue,
        };
        let name = declaration.name().to_owned();
        let node = graph
            .semantic_graph()
            .nodes()
            .find(|node| {
                node.node_tag() == NodeTag::CompositeType
                    && node
                        .declaration()
                        .and_then(|declared| declared.last())
                        .map(|last| last.as_str())
                        == Some(name.as_str())
            })
            .expect("the declaration's composite_type node")
            .key();
        composites.push((node, declaration.key(), name, form));
    }
    composites.sort_by_key(|(node, ..)| *node);
    composites
}

/// The context holds exactly the spine's record and tuple keys; same-shaped
/// declarations have distinct keys, and a repeated lookup returns the same.
#[trace("TC-1090", "FR-396-AC-2")]
#[test]
fn composite_keys_are_the_spines_distinct_and_stable() {
    let text = format!("{HEADER}{UNIT}");
    let (expected, parameter) = with_spine_graph(&text, |graph| {
        let parameter = graph
            .callable("uses_left")
            .expect("the function")
            .parameters[0]
            .1
            .clone();
        (spine_composites(graph), parameter)
    });
    let mut names: Vec<_> = expected.iter().map(|(.., name, _)| name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["First", "Left", "Right", "Second"]);

    let admitted = admit(&text, &DependencyInput::default());
    let context = admitted.context();
    let table: Vec<Expected> = context
        .composites()
        .map(|composite| {
            (
                composite.node(),
                composite.key(),
                composite
                    .qualified_name()
                    .last()
                    .expect("a name")
                    .as_str()
                    .to_owned(),
                composite.form(),
            )
        })
        .collect();
    assert_eq!(table, expected);

    // The kernel key is the one the checked function signature carries for
    // a parameter of the record's type, and not the node id.
    let left = context
        .composites()
        .find(|composite| composite.qualified_name().last().map(|n| n.as_str()) == Some("Left"))
        .expect("Left");
    assert_eq!(parameter, ValueType::Composite(left.key()));
    assert_ne!(left.key(), left.node());

    let keys: BTreeSet<_> = expected.iter().map(|(_, key, ..)| *key).collect();
    assert_eq!(keys.len(), 4, "each declaration has its own key");

    for (node, key, ..) in &expected {
        let first = context.composite_declaration(wire(*node)).expect("found");
        let again = context.composite_declaration(wire(*node)).expect("found");
        assert_eq!(first.key(), *key);
        assert_eq!(first, again);
    }
    let recompiled = admit(&text, &DependencyInput::default());
    let again: Vec<_> = recompiled.context().composites().cloned().collect();
    let first: Vec<_> = context.composites().cloned().collect();
    assert_eq!(again, first);
}

/// An id nobody admitted is `Unknown`; an enum, a union and a function of
/// this package are `WrongKind`; no lookup returns a partial value.
#[trace("TC-1090", "FR-396-AC-3")]
#[test]
fn composite_lookups_refuse_unknown_and_wrong_kind_ids() {
    let admitted = admit(&format!("{HEADER}{UNIT}"), &DependencyInput::default());
    let context = admitted.context();

    let nobody = WireNodeId::from_digest([7; 32]);
    assert_eq!(
        context.composite_declaration(nobody),
        Err(AdmittedLookupRefusal::Unknown {
            declaration: nobody,
            case: None
        })
    );

    let color = wire(context.enums().next().expect("the enum").key());
    assert_eq!(
        context.composite_declaration(color),
        Err(AdmittedLookupRefusal::WrongKind { declaration: color })
    );

    // The union's checked type node and the function's node, from the
    // spine's own graph.
    let (union, function) = with_spine_graph(&format!("{HEADER}{UNIT}"), |graph| {
        let union = graph
            .checked_type_nodes()
            .find_map(|type_node| match type_node {
                CheckedTypeNode::Composite {
                    node,
                    form: CompositeForm::Union,
                    ..
                } => Some(*node),
                _ => None,
            })
            .expect("the union's node");
        let function = graph.function_identity("is_blue").expect("the function");
        (union, function)
    });
    for id in [wire(union), wire(function)] {
        assert_eq!(
            context.composite_declaration(id),
            Err(AdmittedLookupRefusal::WrongKind { declaration: id })
        );
    }
}

/// A record a dependency admitted is `Foreign`, naming that dependency,
/// directly or through an intermediate library; the unit's own record is
/// found.
#[trace("TC-1090", "FR-396-AC-3")]
#[test]
fn a_dependencys_composite_is_foreign() {
    let points = library("test/points", "points", POINTS);
    let alone = compile_admitted(
        points.source.clone(),
        &points.path,
        &points.bytes,
        [],
        &DependencyInput::default(),
        limits(),
        ReplayLimits::default(),
    )
    .expect("the library compiles");
    let point = alone.context().composites().next().expect("the record");
    let point_id = wire(point.node());
    let package = alone.context().package_id();
    let mid = library(
        "test/mid",
        "mid",
        "import \"test/points\" as p;\nfunction h using v(): Boolean pure { true }\n",
    );

    for (import, dependencies) in [
        ("test/points", vec![points.clone()]),
        ("test/mid", vec![points.clone(), mid.clone()]),
    ] {
        let text = format!("{HEADER}import \"{import}\" as d;\n{UNIT}");
        let admitted = admit(
            &text,
            &DependencyInput::new(dependencies).expect("a dependency input"),
        );
        let context = admitted.context();
        assert_eq!(
            context.composite_declaration(point_id),
            Err(AdmittedLookupRefusal::Foreign {
                declaration: point_id,
                package,
            })
        );
        assert_eq!(context.composites().count(), 4);
        for own in context.composites() {
            assert_eq!(context.composite_declaration(wire(own.node())), Ok(own));
        }
    }
}
