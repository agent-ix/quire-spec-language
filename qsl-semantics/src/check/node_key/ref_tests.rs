// SPDX-License-Identifier: AGPL-3.0-or-later
//! `BodyTerm::for_each_ref` and `map_refs` reach every reference, pending
//! ones included, and refuse a pending image for an operation member's node.

use super::*;
use quire_exact::PathSegment;

fn key(name: &str) -> NodeKey {
    NodeKey::new("pkg", vec![PathSegment::Text(name.to_owned())], 0).expect("a key")
}

fn pending() -> NodeRef {
    NodeRef::Pending(PlaceholderId(7))
}

/// A dependency reference holding a pending handle is visited and mapped.
#[test]
fn a_pending_dependency_reference_is_visited_and_mapped() {
    let body = BodyTerm::Leaf(LeafTerm::DependencyReference { node: pending() });
    let mut seen = Vec::new();
    body.for_each_ref(&mut |reference| seen.push(reference.clone()));
    assert_eq!(seen, vec![pending()]);
    let mapped = body
        .map_refs(&mut |_| NodeRef::Admitted(key("resolved")))
        .expect("a leaf maps");
    assert_eq!(
        mapped,
        BodyTerm::Leaf(LeafTerm::DependencyReference {
            node: NodeRef::Admitted(key("resolved"))
        })
    );
}

/// An operation member's declaration is always an admitted key: a pending
/// image for it is refused, not ignored.
#[test]
fn a_pending_image_for_an_operation_member_is_refused() {
    let mut operation = Operation::plain("quire.op.test");
    operation.member = Some(Member::Element {
        declaration: key("declaration"),
    });
    let body = BodyTerm::Application(ApplicationTerm {
        operator: Operator::Call,
        operation,
        result_type: NodeRef::Admitted(key("result")),
        arguments: Vec::new(),
    });
    let mut seen = Vec::new();
    body.for_each_ref(&mut |reference| seen.push(reference.clone()));
    assert_eq!(seen.len(), 2, "the member declaration and the result type");
    assert!(matches!(
        body.map_refs(&mut |_| pending()),
        Err(NodeKeyRefusal::UnresolvedPlaceholder)
    ));
    assert!(body.map_refs(&mut |reference| reference.clone()).is_ok());
}
