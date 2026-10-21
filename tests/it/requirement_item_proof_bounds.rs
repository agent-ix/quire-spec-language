// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-097 and FR-075 (ADR-014 §4 "Bounded request"): a requirement item
//! read from a checked package carries no proof bound, and its bounded
//! follow-up is the single item of its own request, at the original
//! occurrence, carrying the substituted proof bounds with QSL's domain
//! kinds (QSpec FR-331 `extent.bounds`); a temporal item's subject proof
//! bounds are separate (`temporal.subject.proof_bounds`).

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBound, ProofBound};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::{CatalogCode, CatalogCoded, RequestIndex};
use qsl_route::request::{
    items_from_requirements, BoundRefusal, ExtentClassification, RequirementItem,
};
use qsl_route::Registry;
use qsl_semantics::check::Capability;
use quire_exact::Integer;

use super::request_builder::package;

/// The requirement items of the unit `declarations`, in record order.
fn items(declarations: &str) -> Vec<RequirementItem> {
    let package = package(declarations);
    items_from_requirements(package.graph().requirements(), &Registry::new(), None)
}

/// The one requirement item of the unit `declarations`.
fn item(declarations: &str) -> RequirementItem {
    let [item] = <[RequirementItem; 1]>::try_from(items(declarations)).unwrap_or_else(|items| {
        panic!("one item: {items:#?}");
    });
    item
}

fn range(lower: i64, upper: i64) -> FiniteBound {
    FiniteBound::integer_range(Integer::from(lower), Integer::from(upper))
        .expect("a non-empty range")
}

/// The bounded follow-up of `item` with `bound` for its one unbounded
/// domain, and that domain with the kind QSL classified for it.
fn bounded(item: &RequirementItem, bound: FiniteBound) -> (RequirementItem, ProofBound) {
    let [(domain, kind)] = item.unbounded() else {
        panic!("one unbounded domain: {:?}", item.unbounded());
    };
    let follow_up = item
        .bounded(&BTreeMap::from([(domain.clone(), bound.clone())]))
        .expect("one bound of the domain's kind");
    let expected = ProofBound::new(domain.clone(), Some(*kind), bound).expect("a paired bound");
    (follow_up, expected)
}

/// A `value-validity` item over an unbounded set has no proof bound and is
/// no temporal item; its bounded follow-up keeps the occurrence, classifies
/// `bounded`, and carries the cardinality bound with the `collection` kind.
#[trace("TC-438", "FR-097-AC-9")]
#[test]
fn a_bounded_follow_up_carries_its_proof_bounds_at_its_occurrence() {
    let item = item("function f using v(s: Set<Int[0, 9999]>): Integer pure { size(s) + 1 }");
    assert_eq!(item.kind(), Capability::ValueValidity);
    assert!(
        item.proof_bounds().is_empty(),
        "a record substitutes nothing"
    );
    assert_eq!(item.temporal_proof_bounds(), None);

    let (follow_up, expected) = bounded(&item, FiniteBound::cardinality(8));
    assert_eq!(expected.kind(), Some(DomainKind::Collection));
    assert_eq!(follow_up.occurrence(), item.occurrence());
    assert_eq!(follow_up.extent(), ExtentClassification::Bounded);
    assert!(follow_up.unbounded().is_empty());
    assert_eq!(follow_up.proof_bounds(), [expected]);
    assert_eq!(follow_up.temporal_proof_bounds(), None);
}

/// Over a package with several records, the follow-up of every unbounded
/// item, one at a non-zero record position among them, is the single item
/// of its own request: index 0, the original occurrence, `bounded`, and one
/// `integer` proof bound per domain in key order (two domains for the outer
/// `+`); the original item keeps its own index and classification.
#[trace("TC-438", "TC-449", "FR-075-AC-8", "FR-097-AC-3")]
#[test]
fn a_follow_up_from_a_later_record_has_its_own_index_and_the_original_occurrence() {
    let items = items("function g using v(n: Integer, m: Integer): Integer pure { (n + 1) + m }");
    assert!(items.len() > 1, "several records: {items:#?}");
    let unbounded: Vec<_> = items
        .iter()
        .filter(|item| !item.unbounded().is_empty())
        .collect();
    assert!(
        unbounded
            .iter()
            .any(|item| item.index() != RequestIndex::new(0)),
        "an unbounded item past position 0: {items:#?}"
    );
    let mut two_domains = false;
    for item in unbounded {
        let bounds: BTreeMap<DomainKey, FiniteBound> = item
            .unbounded()
            .iter()
            .enumerate()
            .map(|(offset, (domain, _))| {
                let upper = i64::try_from(offset).expect("few domains") + 3;
                (domain.clone(), range(0, upper))
            })
            .collect();
        let before = (item.index(), item.extent());
        let follow_up = item.bounded(&bounds).expect("one integer range per domain");
        assert_eq!(follow_up.index(), RequestIndex::new(0));
        assert_eq!(follow_up.occurrence(), item.occurrence());
        assert_eq!(follow_up.extent(), ExtentClassification::Bounded);
        let expected: Vec<_> = item
            .unbounded()
            .iter()
            .map(|(domain, kind)| {
                assert_eq!(*kind, DomainKind::Integer);
                ProofBound::new(domain.clone(), Some(*kind), bounds[domain].clone())
                    .expect("a paired bound")
            })
            .collect();
        assert_eq!(follow_up.proof_bounds(), expected.as_slice());
        let keys: Vec<_> = follow_up
            .proof_bounds()
            .iter()
            .map(ProofBound::domain)
            .collect();
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "key order");
        two_domains |= keys.len() > 1;
        assert_eq!((item.index(), item.extent()), before);
    }
    assert!(two_domains, "the outer `+` reads both parameters");
}

/// A `temporal-satisfaction` item over a record with an unranged `Integer`
/// field is temporal with no subject proof bound; its bounded follow-up
/// carries the integer-range bound with the `integer` kind as its extent's
/// bounds, at the same occurrence; its subject's bounds stay empty.
#[trace("TC-438", "FR-097-AC-9")]
#[test]
fn a_temporal_follow_up_carries_its_extent_bounds_at_its_occurrence() {
    let item = item(
        "profile t = \"quire.temporal.event-position.false-extension/v1\";\n\
         record Loose { n: Integer; }\n\
         temporal C using t over (c: Loose) clock \"model-steps\" on origin \
         { eventually[0, 3] holds(c.n = 2) }",
    );
    assert_eq!(item.kind(), Capability::TemporalSatisfaction);
    assert_eq!(
        item.extent(),
        ExtentClassification::Unbounded {
            finite_bound_available: true
        }
    );
    assert_eq!(item.temporal_proof_bounds(), Some(&[][..]));

    let (follow_up, expected) = bounded(&item, range(0, 3));
    assert_eq!(expected.kind(), Some(DomainKind::Integer));
    assert_eq!(follow_up.occurrence(), item.occurrence());
    assert_eq!(follow_up.extent(), ExtentClassification::Bounded);
    assert_eq!(follow_up.proof_bounds(), std::slice::from_ref(&expected));
    assert_eq!(
        follow_up.temporal_proof_bounds(),
        Some(&[][..]),
        "the extent's bounds are not the subject's"
    );
}

/// A temporal item's subject proof bounds are independent of its extent:
/// supplied on an item whose extent stays `unbounded` (an infinite-trace
/// domain), they are returned in domain-key order at the same occurrence,
/// move nothing into the extent, and are refused on a non-temporal item or
/// for one domain twice (QSpec FR-331 `temporal.subject.proof_bounds`).
#[trace("TC-438", "FR-097-AC-9")]
#[test]
fn subject_proof_bounds_are_independent_of_the_extent() {
    let temporal = item(
        "profile t = \"quire.temporal.infinite-trace/v1\";\n\
         record Counter { level: Int[0, 9]; }\n\
         temporal C using t over (c: Counter) clock \"model-steps\" on origin \
         { eventually holds(c.level = 2) }",
    );
    let bound = |ordinal: u32, upper: u64| {
        ProofBound::new(
            DomainKey::Population {
                member_type: WireNodeId::from_digest([1; 32]),
                ordinal,
            },
            Some(DomainKind::Population),
            FiniteBound::cardinality(upper),
        )
        .expect("a paired bound")
    };
    let (b, a) = (bound(1, 3), bound(0, 2));
    let with = temporal
        .with_subject_proof_bounds([b.clone(), a.clone()])
        .expect("distinct domains");
    assert_eq!(with.temporal_proof_bounds(), Some(&[a.clone(), b][..]));
    assert_eq!(with.occurrence(), temporal.occurrence());
    assert_eq!(with.index(), temporal.index());
    assert_eq!(with.extent(), temporal.extent());
    assert_eq!(with.unbounded(), temporal.unbounded());
    assert!(with.proof_bounds().is_empty());

    let repeated = temporal
        .with_subject_proof_bounds([a.clone(), a.clone()])
        .expect_err("one domain twice");
    assert!(matches!(
        repeated,
        BoundRefusal::DuplicateSubjectDomain { .. }
    ));
    let set = item("function f using v(s: Set<Int[0, 9999]>): Integer pure { size(s) + 1 }");
    assert_eq!(
        set.with_subject_proof_bounds([a])
            .expect_err("not temporal"),
        BoundRefusal::NotTemporal
    );
}

/// `bounded` refuses `invalid_runtime_input`/`invalid-value` for an item
/// that is already bounded (a follow-up) and for an infinite-trace temporal
/// item, whose formula domain no finite bound stands for, so that item's
/// subject proof bounds stay empty.
#[trace("TC-438", "FR-097-AC-4")]
#[test]
fn bounded_refuses_a_bounded_item_and_an_infinite_trace_item() {
    let set = item("function f using v(s: Set<Int[0, 9999]>): Integer pure { size(s) + 1 }");
    let (follow_up, _) = bounded(&set, FiniteBound::cardinality(8));
    let again = follow_up
        .bounded(&BTreeMap::new())
        .expect_err("a bounded item has no follow-up");
    assert_eq!(again, BoundRefusal::NoUnboundedDomain);

    let temporal = item(
        "profile t = \"quire.temporal.infinite-trace/v1\";\n\
         record Counter { level: Int[0, 9]; }\n\
         temporal C using t over (c: Counter) clock \"model-steps\" on origin \
         { eventually holds(c.level = 2) }",
    );
    assert_eq!(
        temporal.extent(),
        ExtentClassification::Unbounded {
            finite_bound_available: false
        }
    );
    let [(formula, DomainKind::InfiniteTrace)] = temporal.unbounded() else {
        panic!("one infinite-trace domain: {:?}", temporal.unbounded());
    };
    let refusal = temporal
        .bounded(&BTreeMap::from([(
            formula.clone(),
            FiniteBound::cardinality(8),
        )]))
        .expect_err("no finite bound stands for an infinite-trace formula");
    assert_eq!(
        refusal,
        BoundRefusal::UnboundableDomain {
            domain: formula.clone(),
            kind: DomainKind::InfiniteTrace,
        }
    );
    for refusal in [again, refusal] {
        assert_eq!(
            refusal.catalog_code(),
            CatalogCode::new("invalid_runtime_input", "invalid-value")
        );
    }
    assert_eq!(temporal.temporal_proof_bounds(), Some(&[][..]));
}
