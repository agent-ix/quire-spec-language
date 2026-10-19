// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-438 (FR-097, ADR-014 §4 "Bounded request"): a requirement item read
//! from a checked package carries no proof bound, and its bounded follow-up
//! carries the substituted proof bounds, with QSL's domain kinds, at the
//! same occurrence; a temporal item carries them as its subject's proof
//! bounds too (QSpec FR-331 `extent.bounds`, `temporal.subject.proof_bounds`).

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKind, FiniteBound, ProofBound};
use qsl_route::request::{items_from_requirements, ExtentClassification, RequirementItem};
use qsl_route::Registry;
use qsl_semantics::check::Capability;
use quire_exact::Integer;

use super::request_builder::package;

/// The one requirement item of the unit `declarations`.
fn item(declarations: &str) -> RequirementItem {
    let package = package(declarations);
    let items = items_from_requirements(package.graph().requirements(), &Registry::new(), None);
    let [item] = <[RequirementItem; 1]>::try_from(items).unwrap_or_else(|items| {
        panic!("one item: {items:#?}");
    });
    item
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
/// no temporal item; its bounded follow-up keeps the occurrence and index,
/// classifies `bounded`, and carries the cardinality bound with the
/// `collection` kind.
#[trace("TC-438", "FR-097-AC-3", "FR-097-AC-9")]
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
    assert_eq!(follow_up.index(), item.index());
    assert_eq!(follow_up.extent(), ExtentClassification::Bounded);
    assert!(follow_up.unbounded().is_empty());
    assert_eq!(follow_up.proof_bounds(), [expected]);
    assert_eq!(follow_up.temporal_proof_bounds(), None);
}

/// A `temporal-satisfaction` item over a record with an unranged `Integer`
/// field is temporal with no subject proof bound; its bounded follow-up
/// carries the integer-range bound with the `integer` kind both as its
/// extent's bounds and as its subject's, at the same occurrence.
#[trace("TC-438", "FR-097-AC-3", "FR-097-AC-9")]
#[test]
fn a_temporal_follow_up_carries_its_subject_proof_bounds_at_its_occurrence() {
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

    let range = FiniteBound::integer_range(Integer::from(0_i64), Integer::from(3_i64))
        .expect("a non-empty range");
    let (follow_up, expected) = bounded(&item, range);
    assert_eq!(expected.kind(), Some(DomainKind::Integer));
    assert_eq!(follow_up.occurrence(), item.occurrence());
    assert_eq!(follow_up.extent(), ExtentClassification::Bounded);
    assert_eq!(follow_up.proof_bounds(), std::slice::from_ref(&expected));
    assert_eq!(follow_up.temporal_proof_bounds(), Some(&[expected][..]));
}
