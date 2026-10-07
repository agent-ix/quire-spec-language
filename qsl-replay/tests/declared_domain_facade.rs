// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-516 (FR-121-AC-13): a client outside the crate builds a
//! `DeclaredDomain` through `qsl_replay`'s root re-exports alone. As an
//! integration test it reaches only the crate's public API, so a
//! re-export that is not `pub` fails to compile here.

use ix_trace_rs::trace;
use qsl_replay::{
    call_site, DeclaredDomain, DependencyInput, DomainKey, DomainKind, EmptyFiniteBound,
    EmptyInterval, FiniteBound, FiniteBoundKind, Identifier, Integer, IntegerInterval, ProofBound,
    QualifiedName, SourceIdentity,
};

/// A unit of one Boolean predicate `p` of one parameter `x`.
const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function p using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";

/// FR-121-AC-13: a `DeclaredDomain` over `[0, 9]` on the parameter node
/// `call_site` names, built from `ProofBound`, `DomainKey`, `FiniteBound`
/// and `Integer`; its kind is `FiniteBoundKind::IntegerRange` and its
/// interval the `IntegerInterval` built directly. The inverted range
/// refuses `EmptyFiniteBound::InvertedIntegerRange` through
/// `FiniteBound::integer_range` and `EmptyInterval` through
/// `IntegerInterval::new`.
#[trace("TC-516", "FR-121-AC-13")]
#[test]
fn a_declared_domain_is_built_through_the_facade_alone() {
    let name = QualifiedName::new(vec![Identifier::new("p").unwrap()]).unwrap();
    let site = call_site(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        UNIT.as_bytes(),
        [],
        &DependencyInput::default(),
        &name,
    )
    .expect("the unit compiles and declares p");
    let [(_, parameter)] = site.site.parameters[..] else {
        panic!("p declares one parameter: {site:?}");
    };

    let bound = FiniteBound::integer_range(Integer::from(0_i64), Integer::from(9_i64))
        .expect("[0, 9] is not empty");
    let declared = DeclaredDomain::new(
        ProofBound::new(
            DomainKey::Node {
                node: parameter,
                path: Vec::new(),
            },
            Some(DomainKind::Integer),
            bound.clone(),
        )
        .unwrap(),
    );
    assert_eq!(
        declared.domain(),
        &DomainKey::Node {
            node: parameter,
            path: Vec::new()
        }
    );
    assert_eq!(declared.bound(), &bound);
    assert_eq!(declared.bound().kind(), FiniteBoundKind::IntegerRange);
    let FiniteBound::IntegerRange(interval) = declared.bound() else {
        panic!("an integer range: {bound:?}");
    };
    assert_eq!(
        interval,
        &IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64))
            .expect("[0, 9] is not empty")
    );

    assert_eq!(
        FiniteBound::integer_range(Integer::from(9_i64), Integer::from(0_i64)),
        Err(EmptyFiniteBound::InvertedIntegerRange)
    );
    assert_eq!(
        IntegerInterval::new(Integer::from(9_i64), Integer::from(0_i64)),
        Err(EmptyInterval)
    );
}
