// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-322 "Nominal nodes": a dimension equivalence declaration's
//! `scalar_type`/`dimension_equivalence` node, and the `SourceOwner` of the
//! enum, dimension and unit nodes a unit declares, over real source.

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use quire_exact::{Integer, PathSegment, Rational};
use quire_semantic_value::checking::CheckingLimits;

use crate::check::node_key::{BodyTerm, LeafTerm, MemberTerm, NodeTag, Owner};
use crate::check::{CheckedGraph, PackageDeclarations};
use crate::value::nominal_payload::NominalPayload;

/// Two base dimensions with a root unit each, then `rest`.
fn source(rest: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         dimension A;\nunit ua : A = rational(1, 1);\n\
         dimension B;\nunit ub : B = rational(1, 1);\n\
         {rest}\n"
    )
}

fn check(text: &str) -> CheckedGraph {
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        crate::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(CheckingLimits::default())
    .expect("the unit checks")
}

/// QSpec FR-322 (XP-01's shape): one equivalence declaration lowers to
/// one self-typed `scalar_type`/`dimension_equivalence` node under the
/// unit's `SourceOwner`, with no `declaration`, its body referencing the
/// two dimension nodes, keyed
/// `[scalar_type, dimension_equivalence, first, second]` in the package,
/// its payload the two dimension nodes ascending and the root scale of
/// the first, and one `anchor` occurrence at the declaration. The same
/// pair written in the other order stores the reciprocal and is the same
/// node.
#[trace("FR-322-AC-51", "FR-322-AC-268")]
#[test]
fn dimension_equivalence_lowers_to_its_nominal_node() {
    let declaration = "dimension A ~ B by 1 ua = 1000 ub;";
    let text = source(declaration);
    let checked = check(&text);
    let graph = checked.semantic_graph();
    let node = graph
        .nodes()
        .find(|node| node.semantic_form() == "dimension_equivalence")
        .expect("the equivalence node");
    assert_eq!(node.node_tag(), NodeTag::ScalarType);
    assert_eq!(
        node.semantic_type().unwrap_or(node.key()),
        node.key(),
        "self-typed"
    );
    assert_eq!(node.declaration(), None);
    assert!(matches!(node.owner(), Some(Owner::Source(_))));

    let Some(NominalPayload::DimensionEquivalence(payload)) = node.nominal_payload() else {
        panic!(
            "a dimension equivalence payload: {:?}",
            node.nominal_payload()
        );
    };
    let [first, second] = payload.dimensions();
    assert!(first < second, "ascending node keys");
    // The body references exactly the two dimension nodes, ascending.
    let BodyTerm::Aggregate(body) = node.body() else {
        panic!("an aggregate body: {:?}", node.body());
    };
    let referenced: Vec<_> = body
        .members
        .iter()
        .map(|member| match member {
            MemberTerm::Leaf(LeafTerm::Reference { target }) => target.admitted().cloned(),
            _ => None,
        })
        .collect();
    assert_eq!(referenced, vec![Some(first.clone()), Some(second.clone())]);
    for dimension in [first, second] {
        let related = graph
            .node(dimension.clone())
            .expect("a related dimension node");
        assert_eq!(related.semantic_form(), "dimension");
    }
    let a_first = graph
        .node(first.clone())
        .and_then(|node| node.declaration())
        .is_some_and(|name| name.iter().map(|segment| segment.as_str()).eq(["A"]));
    // One canonical root of A is 1000 of B's.
    let thousand = Rational::from_integer(Integer::from(1000_u64));
    let expected = if a_first {
        thousand
    } else {
        Rational::from_integer(Integer::from(1_u64))
            .div(&thousand)
            .expect("a nonzero divisor")
    };
    assert_eq!(payload.scale(), &expected);

    let key = node.key();
    assert_eq!(key.package(), "fixture");
    assert_eq!(key.ordinal(), 0);
    assert_eq!(
        key.path(),
        vec![
            PathSegment::Text("scalar_type".to_owned()),
            PathSegment::Text("dimension_equivalence".to_owned()),
            PathSegment::Node(first.clone()),
            PathSegment::Node(second.clone()),
        ]
    );

    let occurrences: Vec<_> = checked
        .occurrences()
        .filter(|(identity, _, _)| *identity == key)
        .map(|(identity, origin, _)| (identity, origin))
        .collect();
    assert_eq!(occurrences.len(), 1, "one occurrence");
    let (identity, origin) = &occurrences[0];
    assert_eq!(origin.role().as_str(), "anchor");
    let region = checked
        .occurrence_region(identity.clone(), origin)
        .expect("the anchor is located");
    let (start, end) = (
        usize::try_from(region.start()).expect("a start"),
        usize::try_from(region.end()).expect("an end"),
    );
    assert_eq!(&text[start..end], declaration);

    // The other written order stores the reciprocal: the same node.
    let reversed = check(&source("dimension B ~ A by 1000 ub = 1 ua;"));
    let other = reversed
        .semantic_graph()
        .node(key)
        .expect("the same equivalence node");
    assert_eq!(other.nominal_payload(), node.nominal_payload());
}

/// QSpec FR-322: an enum, dimension and unit node the unit declares
/// carries the unit's `SourceOwner`, the owner its other declared nodes
/// carry.
#[trace("FR-322-AC-51")]
#[test]
fn declared_nominal_nodes_carry_the_source_owner() {
    let checked = check(&source(
        "enum Colour { RED, BLUE }\n\
         function f using v(d: Quantity<ua>): Boolean pure { true }",
    ));
    let graph = checked.semantic_graph();
    let declared = |form: &str, name: &str| {
        graph
            .nodes()
            .find(|node| {
                node.semantic_form() == form
                    && node
                        .declaration()
                        .is_some_and(|qualified| qualified.iter().map(|s| s.as_str()).eq([name]))
            })
            .unwrap_or_else(|| panic!("the {form} {name} node"))
    };
    let function_owner = graph
        .nodes()
        .find(|node| node.node_tag() == NodeTag::Function)
        .and_then(|node| node.owner())
        .cloned()
        .expect("the function node's owner");
    assert!(matches!(function_owner, Owner::Source(_)));
    for (form, name) in [("enum", "Colour"), ("dimension", "A"), ("unit", "ua")] {
        assert_eq!(
            declared(form, name).owner(),
            Some(&function_owner),
            "{form} {name}"
        );
    }
}

/// QSpec FR-322 "Nominal nodes" and AC-287: each equivalence node's one
/// `anchor` occurrence has the ordinal of its declaration among the
/// equivalence declarations of its source in source order, with that
/// declaration's own region. The source order here is the reverse of the
/// node-id order (`A ~ B` sorts before `B ~ C`), with a third declaration
/// between the two.
#[trace("FR-322-AC-287")]
#[test]
fn equivalence_anchor_ordinal_is_the_declaration_index() {
    let first = "dimension B ~ C by 1 ub = 1000 uc;";
    let middle = "dimension C ~ D by 1 uc = 1000 ud;";
    let last = "dimension A ~ B by 1 ua = 1000 ub;";
    let text = source(&format!(
        "dimension C;\nunit uc : C = rational(1, 1);\n\
         dimension D;\nunit ud : D = rational(1, 1);\n\
         {first}\n{middle}\n{last}"
    ));
    let checked = check(&text);
    let graph = checked.semantic_graph();
    let mut anchors: Vec<(u64, String)> = graph
        .nodes()
        .filter(|node| node.semantic_form() == "dimension_equivalence")
        .map(|node| {
            let key = node.key();
            let occurrences: Vec<_> = checked
                .occurrences()
                .filter(|(identity, _, _)| *identity == key)
                .collect();
            let [(identity, origin, _)] = occurrences.as_slice() else {
                panic!("one anchor occurrence: {occurrences:?}");
            };
            assert_eq!(origin.role().as_str(), "anchor");
            let region = checked
                .occurrence_region(identity.clone(), origin)
                .expect("the anchor is located");
            let (start, end) = (
                usize::try_from(region.start()).expect("a start"),
                usize::try_from(region.end()).expect("an end"),
            );
            (origin.ordinal(), text[start..end].to_owned())
        })
        .collect();
    anchors.sort();
    assert_eq!(
        anchors,
        vec![
            (0, first.to_owned()),
            (1, middle.to_owned()),
            (2, last.to_owned())
        ],
        "ordinal = declaration index"
    );
}
