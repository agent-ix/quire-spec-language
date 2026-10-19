//! FR-218 "Relationships" (TC-1056) over fixture R of the protocol
//! obligation tests, extended with TC-1056 fixture P's object type `Peer`,
//! its record `Plain` and its relationships `pipe` and `owns` from `Peer` to
//! `Node`.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use quire_exact::ValueType;
use quire_semantic_value::checking::CheckingLimits;

use crate::check::refusal::{ProtocolAnchorCause, RelatedEnd};
use crate::check::{CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations};
use crate::model::domain_package::{
    DomainPackageRecord, FieldMemberRecord, Multiplicity, NativeValueType, ObjectTypeRecord,
    RecordValueTypeRecord, RelationshipDirection, RelationshipEnd, RelationshipRecord,
    ValueTypeRef,
};
use crate::model::key::DeclarationKey;

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
