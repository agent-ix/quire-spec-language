//! FR-218 "Relationships" (TC-1056) over fixture R of the protocol
//! obligation tests, extended with TC-1056 fixture P's object type `Peer`,
//! its record `Plain` and its relationships `pipe` and `owns` from `Peer` to
//! `Node`.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use quire_exact::ValueType;
use quire_semantic_value::checking::CheckingLimits;

use crate::check::node_key::NodeRef;
use crate::check::refusal::{
    ProtocolAnchorCause, ProtocolConstruct, ProtocolReferenceCause, RelatedEnd,
};
use crate::check::{CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations};
use crate::model::domain_package::{
    DomainPackageRecord, FieldMemberRecord, Multiplicity, NativeValueType, ObjectTypeRecord,
    RecordValueTypeRecord, RelationshipDirection, RelationshipEnd, RelationshipRecord,
    ValueTypeRef,
};
use quire_checked::key::DeclarationKey;

// The fixture's plain constructor serves the other protocol tests.
#[allow(dead_code)]
mod fixture {
    use crate as semantics;
    include!("../../../tests/fixtures/protocol_obligations.rs");
}
use fixture::{obligation_model_with, HEADER};

fn key(name: &str) -> DeclarationKey {
    DeclarationKey {
        package: "example/protocol-obligations".to_owned(),
        node: format!("ix://example/protocol-obligations/{name}"),
    }
}

fn field(owner: &str, name: &str, value_type: ValueTypeRef) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: key(&format!("{owner}/{name}")),
        owner: key(owner),
        value_type,
        multiplicity: Multiplicity {
            lower: 1,
            upper: Some(1),
            ordered: false,
            unique: true,
        },
        presence: quire_exact::Presence::Required,
        subsets: Vec::new(),
        redefines: None,
    })
}

fn relationship(name: &str) -> DomainPackageRecord {
    let end = |type_name: &str| RelationshipEnd {
        type_identity: key(type_name),
        role: None,
        multiplicity: None,
    };
    DomainPackageRecord::Relationship(RelationshipRecord {
        key: key(name),
        declared_at: None,
        source: end("Peer"),
        target: end("Node"),
        direction: RelationshipDirection::SourceToTarget,
    })
}

/// Fixture P's additions to fixture R.
fn extra() -> Vec<DomainPackageRecord> {
    vec![
        DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: key("Peer"),
            interface_features: None,
            abstract_type: false,
            supertypes: Vec::new(),
        }),
        field("Peer", "id", ValueTypeRef::Package(key("Digit"))),
        DomainPackageRecord::RecordValueType(RecordValueTypeRecord {
            key: key("Plain"),
            identity_fields: Vec::new(),
            supertypes: Vec::new(),
        }),
        field(
            "Plain",
            "ready",
            ValueTypeRef::Native(NativeValueType::Boolean),
        ),
        field("Plain", "id", ValueTypeRef::Package(key("Digit"))),
        field("Plain", "peer", ValueTypeRef::Package(key("Peer"))),
        relationship("pipe"),
        relationship("owns"),
    ]
}

/// TC-1056's base protocol `Pipeline` with the `members` written before
/// `run` and the `related` clauses on its `receive Got`.
fn pipeline(members: &str, related: &str) -> String {
    format!(
        "{HEADER}protocol Pipeline using v over (view: M::Node) on origin {{\n\
         role Seller on M::Node;\n\
         role Buyer on M::Node;\n\
         {members}\n\
         channel Pay from Seller to Buyer carries M::Plain ordering unordered \
         delivery at-least-once capacity 2 overflow block;\n\
         run sequence Main {{\n\
         send Sent via Pay as (sent: M::Plain) {{ true }};\n\
         receive Got via Pay of Main::Sent as (got: M::Plain) {related} {{ true }};\n\
         }}\n\
         finish Closed as (closed: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

fn check(text: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![obligation_model_with(extra())],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: the assembler refuses: {refusal:?}"))
    .check(CheckingLimits::default())
}

fn checks(text: &str) -> CheckedGraph {
    check(text).unwrap_or_else(|refusals| panic!("{text}: {refusals:?}"))
}

/// One refusal's cause, its catalog pair and the text at its span.
type Refused<'t> = (
    ProtocolAnchorCause,
    (&'static str, Option<&'static str>),
    &'t str,
);

/// Every refusal's cause, catalog pair and the text at its span.
fn refused(text: &str) -> Vec<Refused<'_>> {
    let refusals = check(text).map(|_| ()).expect_err("the protocol refuses");
    refusals
        .iter()
        .map(|refusal| {
            let CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
                panic!("a protocol cause, got {refusal:?}");
            };
            let span = cause.span();
            (
                (**cause).clone(),
                (refusal.cause.code().as_str(), refusal.cause.cause()),
                &text[span.start..span.end],
            )
        })
        .collect()
}

fn reference(graph: &CheckedGraph, name: &str) -> ValueType {
    ValueType::Reference(
        graph
            .scope()
            .object_type_named(name)
            .unwrap_or_else(|| panic!("the object type {name}")),
    )
}

/// FR-218-AC-14: `relationship Pipe = M::pipe;` binds the checked
/// relationship to the export's key and its end types `Peer`, `Node` in
/// that order; `M::nothing` refuses `missing_declaration`/`missing-name` at
/// the name; `M::Node`, an object type, and `M::Flag`, a record, refuse
/// `invalid_model_binding`/`wrong-export` there, retaining the required
/// kind, the actual kind and the export's key; two relationships `Pipe`
/// refuse `ambiguous_declaration`/`ambiguous-name` at both, and a channel
/// and a relationship of one name check (FR-218-AC-10's last case).
#[trace("TC-1056")]
#[test]
fn a_relationship_binds_its_model_relationship() {
    let graph = checks(&pipeline("relationship Pipe = M::pipe;", ""));
    let [pipe] = graph
        .protocol("Pipeline")
        .expect("checked")
        .relationships
        .as_slice()
    else {
        panic!("one relationship");
    };
    assert_eq!(pipe.name, "Pipe");
    assert_eq!(pipe.key, key("pipe"));
    assert_eq!(
        pipe.ends,
        [reference(&graph, "M::Peer"), reference(&graph, "M::Node")]
    );
    checks(&pipeline("relationship Pay = M::pipe;", ""));

    let text = pipeline("relationship Pipe = M::nothing;", "");
    let [(cause, pair, at)] = refused(&text).try_into().expect("one refusal");
    assert_eq!(pair, ("missing_declaration", Some("missing-name")));
    assert!(matches!(
        cause,
        ProtocolAnchorCause::MissingRelationship { .. }
    ));
    assert_eq!(at, "M::nothing");

    for (target, kind) in [("Node", "object type"), ("Flag", "record value type")] {
        let text = pipeline(&format!("relationship Pipe = M::{target};"), "");
        let [(cause, pair, at)] = refused(&text).try_into().expect("one refusal");
        assert_eq!(pair, ("invalid_model_binding", Some("wrong-export")));
        let ProtocolAnchorCause::RelationshipExport {
            required,
            actual,
            export,
            ..
        } = &cause
        else {
            panic!("a wrong-export refusal, got {cause:?}");
        };
        assert_eq!((*required, *actual), ("relationship", kind));
        assert_eq!(**export, key(target));
        assert_eq!(at, format!("M::{target}"));
    }

    let text = pipeline(
        "relationship Pipe = M::pipe;\nrelationship Pipe = M::owns;",
        "",
    );
    let found = refused(&text);
    assert_eq!(found.len(), 2, "{found:?}");
    for (cause, pair, at) in &found {
        assert_eq!(*pair, ("ambiguous_declaration", Some("ambiguous-name")));
        assert!(matches!(cause, ProtocolAnchorCause::Ambiguous { name, .. } if name == "Pipe"));
        assert_eq!(*at, "Pipe");
    }
    assert!(found[0].0.span().start < found[1].0.span().start);
}

/// FR-218-AC-15: `related by Pipe (sent.peer, view)` checks and `Got` holds
/// the clause; `related by Absent (..)` refuses `missing_declaration`/
/// `missing-name` at `Absent`; swapped ends refuse `ill_typed`/
/// `type-mismatch` at each end, retaining the relationship, `end`, expected
/// and actual, and so does an end of a scalar id; two clauses on one node
/// are held in source order.
#[trace("TC-1056", "FR-218-AC-15")]
#[test]
fn a_related_clause_types_each_end_by_its_relationship() {
    let members = "relationship Pipe = M::pipe;\nrelationship Owns = M::owns;";
    let graph = checks(&pipeline(members, "related by Pipe (sent.peer, view)"));
    let checked = graph.protocol("Pipeline").expect("checked");
    let [clause] = checked.slots.related.as_slice() else {
        panic!("one related clause");
    };
    assert_eq!(checked.relationships[clause.relationship].name, "Pipe");
    let peer = reference(&graph, "M::Peer");
    let node = reference(&graph, "M::Node");
    assert_eq!(clause.ends[0].root().value_type(), &peer);
    assert_eq!(clause.ends[1].root().value_type(), &node);

    let graph = checks(&pipeline(
        members,
        "related by Pipe (sent.peer, view) related by Owns (sent.peer, view) \
         related by Pipe (sent.peer, view)",
    ));
    let checked = graph.protocol("Pipeline").expect("checked");
    let names: Vec<&str> = checked
        .slots
        .related
        .iter()
        .map(|clause| checked.relationships[clause.relationship].name.as_str())
        .collect();
    assert_eq!(names, ["Pipe", "Owns", "Pipe"]);

    let text = pipeline(members, "related by Absent (sent.peer, view)");
    let [(cause, pair, at)] = refused(&text).try_into().expect("one refusal");
    assert_eq!(pair, ("missing_declaration", Some("missing-name")));
    assert_eq!(
        cause,
        ProtocolAnchorCause::MissingRelated {
            node: "Got".to_owned(),
            name: "Absent".to_owned(),
            span: cause.span(),
        }
    );
    assert_eq!(at, "Absent");

    let text = pipeline(members, "related by Pipe (view, sent.peer)");
    let found = refused(&text);
    let expected = [
        (RelatedEnd::First, &peer, &node, "view"),
        (RelatedEnd::Second, &node, &peer, "sent.peer"),
    ];
    assert_eq!(found.len(), 2, "{found:?}");
    for ((cause, pair, at), (end, wanted, got, written)) in found.iter().zip(expected) {
        assert_eq!(*pair, ("ill_typed", Some("type-mismatch")));
        assert_eq!(
            *cause,
            ProtocolAnchorCause::RelatedEnd {
                relationship: "Pipe".to_owned(),
                end,
                expected: Box::new(wanted.clone()),
                actual: Box::new(got.clone()),
                span: cause.span(),
            }
        );
        assert_eq!(*at, written);
    }

    // Two equal scalar ids never satisfy a relationship end.
    let text = pipeline(members, "related by Pipe (sent.id, view)");
    let [(cause, pair, at)] = refused(&text).try_into().expect("one refusal");
    assert_eq!(pair, ("ill_typed", Some("type-mismatch")));
    assert!(matches!(
        cause,
        ProtocolAnchorCause::RelatedEnd {
            end: RelatedEnd::First,
            ..
        }
    ));
    assert_eq!(at, "sent.id");
}

/// The packaged `Bare` protocol with `members` over fixture P, checked under
/// the production gate, and the `relationships` argument of its clause node:
/// each target's key text and its tag and form.
fn clause_relationships(members: &str) -> (Vec<(String, String, String)>, String) {
    let text = bare("on origin", members, "true").replacen(
        "model M",
        "profile pp = \"quire.protocol.finite-global/v1\";\nmodel M",
        1,
    );
    let graph = checks(&text);
    let nodes: Vec<_> = graph.semantic_graph().nodes().collect();
    let clause = nodes
        .iter()
        .find(|node| node.semantic_form() == "protocol_clause")
        .expect("the protocol writes its clause");
    let body = serde_json::to_value(clause.body()).expect("a body serializes");
    let argument = body["arguments"]
        .as_array()
        .and_then(|arguments| arguments.iter().find(|a| a["name"] == "relationships"))
        .expect("the relationships argument");
    let targets = argument["value"]["members"]
        .as_array()
        .expect("a set")
        .iter()
        .map(|term| {
            let target = serde_json::to_string(&term["target"]["digest"]).unwrap();
            let node = nodes
                .iter()
                .find(|node| {
                    serde_json::to_value(NodeRef(node.key())).unwrap()["digest"]
                        == term["target"]["digest"]
                })
                .expect("a listed relationship is a node of the graph");
            (
                target,
                format!("{:?}", node.node_tag()),
                node.semantic_form().to_owned(),
            )
        })
        .collect();
    (targets, serde_json::to_string(&body).unwrap())
}

/// FR-383 "Order, sets and collapse": a `relationship` member writes no node
/// of its own. The clause's `relationships` set holds each model
/// relationship's `relation`/`relationship` node, in ascending key order,
/// once: two members that bind one relationship give one entry, and the
/// member's alias is carried nowhere.
#[trace("TC-1056")]
#[test]
fn relationships_lower_to_the_model_relationship_nodes_once_each() {
    let (none, _) = clause_relationships("");
    assert!(none.is_empty());
    let (pipe, _) = clause_relationships("relationship Pipe = M::pipe;");
    assert_eq!(pipe.len(), 1);
    assert_eq!(
        (pipe[0].1.as_str(), pipe[0].2.as_str()),
        ("Relation", "relationship")
    );
    let (both, body) =
        clause_relationships("relationship Owns = M::owns;\nrelationship Pipe = M::pipe;");
    assert_eq!(both.len(), 2);
    assert!(both[0].0 < both[1].0, "ascending: {both:?}");
    assert!(!body.contains("Owns") && !body.contains("Pipe"), "no alias");
    let (reversed, _) =
        clause_relationships("relationship Pipe = M::pipe;\nrelationship Owns = M::owns;");
    assert_eq!(both, reversed, "source order changes nothing");
    let (repeated, _) = clause_relationships(
        "relationship Pipe = M::pipe;\nrelationship Again = M::pipe;\nrelationship Owns = M::owns;",
    );
    assert_eq!(both, repeated, "a repeat is one entry");
}

/// The constructs the production check (no `with_unwritten_protocols`)
/// refuses `text` for, in order, with a protocol profile selected. Every
/// refusal must be a `NotEmitted` one.
fn unwritten(text: &str) -> Vec<ProtocolConstruct> {
    let text = text.replacen(
        "model M",
        "profile pp = \"quire.protocol.finite-global/v1\";\nmodel M",
        1,
    );
    let refusals = check(&text).map(|_| ()).expect_err("the protocol refuses");
    refusals
        .iter()
        .map(|refusal| match &refusal.cause {
            CheckCause::ProtocolReference(cause) => match cause.as_ref() {
                ProtocolReferenceCause::NotEmitted { construct } => *construct,
                ProtocolReferenceCause::Repeated { .. } => panic!("{refusal:?}"),
            },
            other => panic!("{text}: only NotEmitted refusals, got {other:?}"),
        })
        .collect()
}

/// A protocol over the checked R fixture with `members` before `run`, an
/// empty `run` sequence and a `finish`, so the only construct it names is
/// what `members` and `activation` write.
fn bare(activation: &str, members: &str, finish: &str) -> String {
    let (captures, members) = members
        .strip_prefix("capture ")
        .map_or(("", members), |_| (members, ""));
    format!(
        "{HEADER}protocol Bare using v over (view: M::Node) {activation} {{\n\
         {captures}\n\
         role Seller on M::Node;\n\
         {members}\n\
         run sequence Main {{ }}\n\
         finish Closed as (closed: M::Receipt) {{ {finish} }};\n\
         }}\n"
    )
}

/// FR-092-AC-18, FR-383: a protocol with a protocol profile selected that
/// names a construct whose node is not written yet never checks to a package
/// that lacks that part of its graph: it refuses `NotEmitted`, naming the
/// construct and the ticket that writes it, for every such construct.
#[trace("FR-092-AC-18", "FR-383-AC-1")]
#[test]
fn every_unwritten_construct_refuses_the_protocol_naming_its_ticket() {
    use ProtocolConstruct as C;
    let cases: Vec<(String, Vec<C>, &str)> = vec![
        (bare("on origin", "", "true"), vec![], ""),
        (
            bare("on origin", "capture start: M::Total = view.total;", "true"),
            vec![C::Captures],
            "QSL-1121e",
        ),
        (
            bare("on each (opened: M::Tick) when (opened.ready)", "", "true"),
            vec![C::ActivationGuard],
            "QSL-1121e",
        ),
        (
            bare("on origin", "terminal any;", "true"),
            vec![C::Terminal],
            "QSL-1121f",
        ),
        (
            bare("on origin", "", "ok()"),
            vec![C::FinishReadsDeclaration],
            "QSL-1121g",
        ),
    ];
    for (text, expected, ticket) in cases {
        let text = if expected == [C::FinishReadsDeclaration] {
            text.replacen(
                "protocol Bare",
                "function ok using v(): Boolean pure { true }\nprotocol Bare",
                1,
            )
        } else {
            text
        };
        if expected.is_empty() {
            let packaged = text.replacen(
                "model M",
                "profile pp = \"quire.protocol.finite-global/v1\";\nmodel M",
                1,
            );
            checks(&packaged);
            continue;
        }
        assert_eq!(unwritten(&text), expected, "{text}");
        for construct in expected {
            assert_eq!(construct.tracking(), ticket, "{construct:?}");
        }
    }
}
