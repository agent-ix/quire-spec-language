// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1090 (FR-396-AC-2 to FR-396-AC-4): `compile_admitted`'s context holds
//! every record, tuple and union declaration the check admitted, with the keys the
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

use qsl_semantics::check::{mint_union_member_key, CheckedGraph, NodeTag};
use qsl_semantics::value::enumeration::mint_union_variant_id;
use quire_exact::Cancel;
use quire_exact::{ValueType, VariantId};
use quire_semantic_value::declaration::CompositeShape;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
// Two same-shaped records, tuples and unions, a union with a record payload,
// an enum and functions: the enum and the functions are the nodes a composite
// lookup refuses as the wrong kind.
const UNIT: &str = "record Left { a: Boolean; b: Boolean; }\n\
     record Right { a: Boolean; b: Boolean; }\n\
     tuple First(Boolean, Boolean);\n\
     tuple Second(Boolean, Boolean);\n\
     enum Color { BLUE, RED }\n\
     union Shape { Empty, Circle(Integer) }\n\
     union Twin { Empty, Circle(Integer) }\n\
     union Holder { Nothing, Has(Left, Boolean) }\n\
     union Nest { Flat, Deep(Shape, Holder) }\n\
     function is_blue using v(c: Color): Boolean pure { false }\n\
     function uses_left using v(x: Left): Boolean pure { x.a }\n";
const POINTS: &str = "record Point { x: Boolean; }\nunion Pick { A, B(Point) }\n";

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

/// A union member as the producer's own minting gives it: the member name,
/// the `VariantId` of the FR-441 key over the union's node id, and the
/// payload position types from the checked `TypeEnvironment`.
type ExpectedMember = (String, VariantId, Vec<ValueType>);

/// Every record, tuple and union declaration of `graph`, ascending by node
/// id.
fn spine_composites(graph: &CheckedGraph) -> Vec<Expected> {
    let mut composites = Vec::new();
    for declaration in graph.scope().types().composites() {
        let form = match declaration.shape() {
            CompositeShape::Record(_) => AdmittedCompositeForm::Record,
            CompositeShape::Tuple(_) => AdmittedCompositeForm::Tuple,
            CompositeShape::Union(_) => AdmittedCompositeForm::Union,
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
    assert_eq!(
        names,
        ["First", "Holder", "Left", "Nest", "Right", "Second", "Shape", "Twin"]
    );

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
    assert_eq!(keys.len(), 8, "each declaration has its own key");

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

/// An id nobody admitted is `Unknown`; an enum and a function of this
/// package are `WrongKind`; no lookup returns a partial value.
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

    let function = with_spine_graph(&format!("{HEADER}{UNIT}"), |graph| {
        graph.function_identity("is_blue").expect("the function")
    });
    let id = wire(function);
    assert_eq!(
        context.composite_declaration(id),
        Err(AdmittedLookupRefusal::WrongKind { declaration: id })
    );
}

/// The oracle for a union's members: the member keys the producer mints over
/// the union's node id, retyped as `VariantId`s, with the payload types the
/// checked `TypeEnvironment` declares, read independently of the facade.
fn spine_members(graph: &CheckedGraph, name: &str) -> (NodeKey, Vec<ExpectedMember>) {
    let types = graph.scope().types();
    let declaration = types
        .composites()
        .find(|declaration| declaration.name() == name)
        .expect("the union");
    let node = types.union_key(declaration.key()).expect("sealed");
    let CompositeShape::Union(members) = declaration.shape() else {
        panic!("{name} is a union");
    };
    let members = members
        .iter()
        .map(|member| {
            let minted =
                mint_union_variant_id(mint_union_member_key(node, member.identifier().as_str()));
            // The sealed environment's own binding agrees with the minting.
            let sealed = types
                .union_member_named(declaration.key(), member.identifier().as_str())
                .and_then(|(_, declared)| declared.member())
                .expect("a sealed member");
            assert_eq!(sealed.variant(), minted);
            (
                member.identifier().as_str().to_owned(),
                minted,
                member.positions().to_vec(),
            )
        })
        .collect();
    (node, members)
}

/// A union is found by its `composite_type` node id with its kernel key and
/// every member's `VariantId` and payload in declared order; two same-shaped
/// unions have distinct keys and member ids; a record payload position is
/// `ValueType::Composite` of the record's admitted key.
#[trace("TC-1090", "FR-396-AC-6", "FR-396-AC-8")]
#[test]
fn union_members_are_the_producers_minted_variants_in_declared_order() {
    let text = format!("{HEADER}{UNIT}");
    let admitted = admit(&text, &DependencyInput::default());
    let context = admitted.context();
    let by_name = |name: &str| {
        context
            .composites()
            .find(|composite| composite.qualified_name().last().map(|n| n.as_str()) == Some(name))
            .expect("admitted")
    };

    let shape_of = |name: &str| by_name(name).union_members().expect("members");
    let mut seen = BTreeSet::new();
    for name in ["Shape", "Twin", "Holder"] {
        let (node, expected) = with_spine_graph(&text, |graph| spine_members(graph, name));
        let found = by_name(name);
        assert_eq!(found.form(), AdmittedCompositeForm::Union);
        // The witness value of a union names the settled node.
        assert_eq!(found.node(), node);
        assert_eq!(context.composite_declaration(wire(node)), Ok(found));
        let members: Vec<ExpectedMember> = found
            .union_members()
            .expect("a union has members")
            .iter()
            .map(|member| {
                (
                    member.identifier().as_str().to_owned(),
                    member.variant(),
                    member.payload().to_vec(),
                )
            })
            .collect();
        assert_eq!(members, expected);
        for (_, variant, _) in &members {
            assert!(
                seen.insert(*variant),
                "member ids are distinct across unions"
            );
        }
    }
    assert_eq!(seen.len(), 2 + 2 + 2, "Shape, Twin and Holder: six members");

    let (shape, twin) = (by_name("Shape"), by_name("Twin"));
    assert_ne!(shape.key(), twin.key());
    assert_eq!(
        shape.union_members().map(<[_]>::len),
        twin.union_members().map(<[_]>::len)
    );

    // Literal payloads, independent of the environment the oracle reads.
    for union in [shape_of("Shape"), shape_of("Twin")] {
        let payloads: Vec<_> = union.iter().map(|m| m.payload().to_vec()).collect();
        assert_eq!(payloads, [vec![], vec![ValueType::Integer]]);
    }

    // A record payload position carries the record's kernel key.
    let left = by_name("Left");
    let has = &by_name("Holder").union_members().expect("members")[1];
    assert_eq!(
        has.payload(),
        [ValueType::Composite(left.key()), ValueType::Boolean]
    );
    assert!(by_name("Left").union_members().is_none());

    // A union position is `ValueType::Composite` of that union's kernel key
    // (its type handle), not of its node id; the checked signature of a
    // parameter of the union's type says the same.
    let (shape, holder) = (by_name("Shape"), by_name("Holder"));
    let deep = &by_name("Nest").union_members().expect("members")[1];
    assert_eq!(
        deep.payload(),
        [
            ValueType::Composite(shape.key()),
            ValueType::Composite(holder.key())
        ]
    );
    assert_ne!(shape.key(), shape.node());
    let parameter = with_spine_graph(
        &format!("{HEADER}{UNIT}function takes using v(s: Shape): Boolean pure {{ false }}\n"),
        |graph| {
            graph.callable("takes").expect("function").parameters[0]
                .1
                .clone()
        },
    );
    assert_eq!(parameter, ValueType::Composite(shape.key()));

    // Recompiling returns the same union declarations.
    let again = admit(&text, &DependencyInput::default());
    assert_eq!(
        again.context().composites().cloned().collect::<Vec<_>>(),
        context.composites().cloned().collect::<Vec<_>>()
    );
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
    let found = |name: &str| {
        wire(
            alone
                .context()
                .composites()
                .find(|composite| {
                    composite.qualified_name().last().map(|n| n.as_str()) == Some(name)
                })
                .expect("the declaration")
                .node(),
        )
    };
    let (point_id, pick_id) = (found("Point"), found("Pick"));
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
        for declaration in [point_id, pick_id] {
            assert_eq!(
                context.composite_declaration(declaration),
                Err(AdmittedLookupRefusal::Foreign {
                    declaration,
                    package,
                })
            );
        }
        assert_eq!(context.composites().count(), 8);
        for own in context.composites() {
            assert_eq!(context.composite_declaration(wire(own.node())), Ok(own));
        }
    }
}
