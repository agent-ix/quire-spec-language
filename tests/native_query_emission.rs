// SPDX-License-Identifier: AGPL-3.0-only
//! Actual query source reaches family admission and the independent wire reader.
//! These tests inspect emitted structure, not query execution or runtime conformance.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::linking::composed::scopes::{BinderKind, BinderType};
use quire_spec_language::protocol_artifact::{
    native, wire as w, Error, Limits, ProtocolNumber, Unsupported,
};
use quire_spec_language::syntax::composed::{self as c, ComposedUnit};
use quire_spec_language::syntax::{ExprId, ExprKind};
use setup::{Inputs, Unit};

const FLOW: &str = "protocol Flow using P over (view: M::Node) on origin {
    role Service on M::Node;
    run check Ready using S { Queries(view) };
    finish Closed as (closed: M::Node) { true };
}";

fn discharged(report: &proofs::ProofReport<'_, '_, '_>) {
    assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
    for declaration in report.declarations() {
        assert_eq!(
            report.types().disposition(declaration.declaration()),
            Some(TypeDisposition::Typed),
            "declaration {:?}: type causes {:?}; scope issues {:?}",
            declaration.declaration(),
            report
                .types()
                .declaration(declaration.declaration())
                .map(|typed| typed.causes()),
            report
                .types()
                .binding()
                .scopes()
                .and_then(|scopes| scopes.declaration(declaration.declaration()))
                .map(|scope| &scope.issues)
        );
        assert_eq!(
            declaration.disposition(),
            proofs::ProofDisposition::Discharged,
            "{:?}",
            declaration.causes()
        );
        assert!(declaration.complete());
    }
}

fn value_at<'a>(declaration: &'a w::Declaration, owner: u32, handle: &w::Handle) -> &'a w::Value {
    assert_eq!(handle.declaration, owner);
    &declaration.values[handle.index as usize]
}

fn original_index(unit: &ComposedUnit, expression: ExprId) -> u32 {
    let original = unit.expression(expression).unwrap();
    unit.expressions()
        .iter()
        .position(|node| std::ptr::eq(node, original))
        .unwrap() as u32
}

fn scalar(package: &w::Package, at: u32, name: &str, unit: Option<&str>) {
    let w::Type::Scalar {
        export,
        unit: actual,
        ..
    } = &package.types[at as usize]
    else {
        panic!("expected the original nominal {name} scalar")
    };
    assert_eq!(actual.0.as_deref(), unit);
    assert_eq!(
        package.models[export.model as usize].exports[export.export as usize].path,
        [name]
    );
}

fn sequence(package: &w::Package, at: u32) -> u32 {
    let w::Type::Sequence { element, maximum } = &package.types[at as usize] else {
        panic!("expected retained ordered sequence")
    };
    let ProtocolNumber::Integer(maximum) = maximum.checked().unwrap() else {
        panic!("authored integer capacity")
    };
    assert_eq!(maximum.value(), 5);
    *element
}

/// Compare source handles directly, including each original BinderId and the
/// boundary where its lexical body begins. No wire table supplies the oracle.
fn original_queries(report: &proofs::ProofReport<'_, '_, '_>, package: &w::Package) -> [usize; 8] {
    let namespace = report.types().binding().namespace();
    let mut counts = [0; 8];
    for (owner, declaration) in package.declarations.iter().enumerate() {
        let owner = owner as u32;
        let [id] = namespace.lookup(&declaration.name) else {
            panic!("one original declaration")
        };
        let typed = report.types().declaration(*id).unwrap();
        let unit = namespace.unit(typed.unit()).unwrap();
        let scope = report
            .types()
            .binding()
            .scopes()
            .unwrap()
            .declaration(*id)
            .unwrap();
        assert_eq!(
            package.sources[declaration.locus.source as usize].text,
            unit.source().text()
        );
        for value in &declaration.values {
            let original = &unit.expressions()[value.original_expression as usize];
            assert_eq!(
                (
                    value.locus.span.start as usize,
                    value.locus.span.end as usize
                ),
                (original.span.start, original.span.end)
            );
            assert_eq!(value.locus.source, declaration.locus.source);
            assert_eq!(
                value
                    .operator_locus
                    .0
                    .as_ref()
                    .map(|locus| (locus.span.start as usize, locus.span.end as usize)),
                original.operator_span.map(|span| (span.start, span.end))
            );
            let (expected_operator, name, domain, body) = match &original.kind {
                c::ValueKind::Size { argument, .. } => {
                    let w::ValueOperation::Size { collection, result } = &value.operation else {
                        panic!("size remains size")
                    };
                    assert_eq!(
                        value_at(declaration, owner, collection).original_expression,
                        original_index(unit, *argument)
                    );
                    assert_eq!(*result, value.value_type);
                    scalar(package, *result, "Tally", None);
                    counts[0] += 1;
                    continue;
                }
                c::ValueKind::Contains {
                    collection: original_collection,
                    member: original_member,
                } => {
                    let w::ValueOperation::Contains { collection, member } = &value.operation
                    else {
                        panic!("contains preserves ordered operands")
                    };
                    assert_eq!(
                        value_at(declaration, owner, collection).original_expression,
                        original_index(unit, *original_collection)
                    );
                    assert_eq!(
                        value_at(declaration, owner, member).original_expression,
                        original_index(unit, *original_member)
                    );
                    assert!(matches!(
                        package.types[value.value_type as usize],
                        w::Type::Boolean {}
                    ));
                    counts[1] += 1;
                    continue;
                }
                c::ValueKind::Shared(ExprKind::Quantifier {
                    universal,
                    name,
                    domain,
                    predicate,
                }) => (
                    if *universal {
                        w::Query::ForAll
                    } else {
                        w::Query::Exists
                    },
                    name,
                    *domain,
                    *predicate,
                ),
                c::ValueKind::Query {
                    op,
                    binder,
                    domain,
                    body,
                    ..
                } => (
                    match op.value {
                        c::QueryOp::Filter => w::Query::Filter,
                        c::QueryOp::Map => w::Query::Map,
                        c::QueryOp::Count => w::Query::Count,
                        c::QueryOp::Sum => w::Query::Sum,
                    },
                    binder,
                    *domain,
                    *body,
                ),
                _ => continue,
            };
            let w::ValueOperation::Query {
                operator,
                binder,
                collection,
                body: lowered_body,
                result,
            } = &value.operation
            else {
                panic!("query remains an executable query node")
            };
            assert_eq!(*operator, expected_operator);
            assert_eq!(*result, value.value_type);
            assert_eq!(binder.declaration, owner);
            let original_binder = scope
                .binders
                .iter()
                .position(|candidate| {
                    candidate.kind == BinderKind::Query && candidate.span == name.span
                })
                .unwrap();
            assert!(
                matches!(scope.binders[original_binder].ty, BinderType::ElementOf(original_domain) if original_domain == domain)
            );
            for read in typed
                .nodes()
                .iter()
                .filter(|node| node.binder.is_some_and(|id| id.index() == original_binder))
            {
                let retained_read = declaration
                    .values
                    .iter()
                    .find(|value| {
                        value.original_expression == original_index(unit, read.expression)
                    })
                    .unwrap();
                assert!(
                    matches!(&retained_read.operation, w::ValueOperation::Read { binder: target } if target == binder)
                );
            }
            let retained = &declaration.binders[binder.index as usize];
            assert_eq!(retained.kind, w::BinderKind::Query);
            assert_eq!(retained.name, name.value);
            assert_eq!(
                (
                    retained.locus.span.start as usize,
                    retained.locus.span.end as usize
                ),
                (name.span.start, name.span.end)
            );
            let domain_value = value_at(declaration, owner, collection);
            let body_value = value_at(declaration, owner, lowered_body);
            assert_eq!(
                domain_value.original_expression,
                original_index(unit, domain)
            );
            assert_eq!(body_value.original_expression, original_index(unit, body));
            assert_eq!(
                domain_value.scope, value.scope,
                "a query binder does not scope its collection"
            );
            assert_eq!(body_value.scope, retained.scope);
            assert_eq!(
                declaration.scopes[retained.scope.index as usize]
                    .parent
                    .0
                    .as_ref(),
                Some(&value.scope)
            );
            counts[match operator {
                w::Query::ForAll => 2,
                w::Query::Exists => 3,
                w::Query::Filter => 4,
                w::Query::Map => 5,
                w::Query::Count => 6,
                w::Query::Sum => 7,
            }] += 1;
            match operator {
                w::Query::ForAll | w::Query::Exists => assert!(matches!(
                    package.types[*result as usize],
                    w::Type::Boolean {}
                )),
                w::Query::Filter => {
                    scalar(package, sequence(package, *result), "Amount", Some("U"))
                }
                w::Query::Map => assert!(matches!(
                    package.types[sequence(package, *result) as usize],
                    w::Type::Boolean {}
                )),
                w::Query::Count => scalar(package, *result, "Tally", None),
                w::Query::Sum => scalar(package, *result, "Total", Some("U")),
            }
        }
    }
    counts
}

#[test]
#[trace("TC-119", "TC-121", "FR-040-AC-5", "FR-042-AC-4", "FR-042-AC-7")]
fn all_eight_queries_emit_original_ordered_graphs_after_actual_discharge() {
    let inputs = Inputs::new(&[
        Unit {
            name: "queries",
            body: "predicate Queries using S (input: M::Node): Boolean {
            size<M::Tally>(input.amounts) >= 0 and contains(input.amounts, 2)
            and forall(allItem in input.amounts: allItem >= 1)
            and exists(someItem in input.amounts: someItem <= 20)
            and contains(filter(kept in input.amounts: kept = 2), 2)
            and contains(map(projected in input.amounts: projected >= 1), true)
            and count<M::Tally>(counted in input.amounts: counted = 2) >= 0
            and sum<M::Total>(summand in input.amounts: summand) >= 0
        }",
            declarations: &["Queries"],
        },
        Unit {
            name: "query-flow",
            body: FLOW,
            declarations: &["Flow"],
        },
    ]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selections| {
            discharged(proofs);
            let admitted = native::admit(proofs, selections, Limits::default())
                .into_result()
                .expect("supported native query package");
            let package = admitted.package();
            assert_eq!(original_queries(proofs, package), [1, 3, 1, 1, 1, 1, 1, 1]);
            let query = package
                .declarations
                .iter()
                .position(|declaration| declaration.name == "Queries")
                .unwrap() as u32;
            let flow = package
                .declarations
                .iter()
                .find(|declaration| declaration.name == "Flow")
                .unwrap();
            assert_eq!(flow.requires, [query]);
            assert!(matches!(flow.body, w::Body::Protocol { .. }));
            let call = flow
                .values
                .iter()
                .find_map(|value| match &value.operation {
                    w::ValueOperation::Call {
                        predicate,
                        arguments,
                    } => Some((predicate, arguments)),
                    _ => None,
                })
                .unwrap();
            assert_eq!(*call.0, query);
            assert_eq!(call.1.len(), 1);
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("independent original source/model/definition selections");
            assert_eq!(read.package(), package);
            assert_eq!(read.digest(), emitted.digest());
        },
    );
}

#[test]
#[trace("TC-119", "TC-121", "FR-040-AC-5", "FR-042-AC-4", "FR-042-AC-7")]
fn nested_queries_preserve_body_scopes_and_captured_pre_origin_in_emitted_contracts() {
    let mut inputs = Inputs::new(&[
        Unit { name: "query-contracts", body: "pre Before using S on M::Node::step { forall(prior in self.amounts: prior >= 1) }
        post After using S on M::Node::step {
            let previous = pre(self.amounts) in
            forall(outerItem in filter(filteredItem in previous: filteredItem >= 1):
                exists(innerItem in self.amounts: innerItem >= 1))
            and forall(currentItem in self.amounts: currentItem >= 1)
        }", declarations: &["Before", "After"] },
        Unit { name: "query-operation", body: "protocol Flow using P over (view: M::Node) on origin {
            role Service on M::Node;
            run attempt Tried by Service on M::Node::step contracts [Before,After] as (attempted: M::Plain) { attempted.ready };
            finish Closed as (closed: M::Node) { true };
        }", declarations: &["Flow"] },
    ]);
    let operation = inputs.step_contracts("Before", "After");
    inputs.with_proofs(TypeLimits::default(), proofs::ProofLimits::default(), |proofs, selections| {
        discharged(proofs);
        let admitted = native::admit(proofs, selections, Limits::default()).into_result().expect("nested and captured query contracts");
        let package = admitted.package();
        assert_eq!(original_queries(proofs, package), [0, 0, 3, 1, 1, 0, 0, 0]);
        let after = package.declarations.iter().find(|declaration| declaration.name == "After").unwrap();
        assert_eq!(after.execution, w::Execution::Post { operation: operation.clone() });
        let previous = after.binders.iter().find(|binder| binder.name == "previous").unwrap();
        assert_eq!(after.anchors[previous.anchor.index as usize].kind, w::AnchorKind::InvocationPost);
        let initializer = previous.initializer.0.as_ref().unwrap();
        let initial = &after.values[initializer.index as usize];
        assert!(matches!(initial.operation, w::ValueOperation::Pre { .. }));
        let w::Origin::Anchor { anchor } = &initial.origin else { panic!("original pre observation") };
        assert_eq!(after.anchors[anchor.index as usize].kind, w::AnchorKind::InvocationPre);
        let nested: Vec<_> = after.binders.iter().enumerate().filter(|(_, binder)| binder.kind == w::BinderKind::Query && ["outerItem", "filteredItem", "innerItem"].contains(&binder.name.as_str())).collect();
        assert_eq!(nested.len(), 3);
        for (position, (index, binder)) in nested.iter().enumerate() {
            for (other_index, other) in &nested[..position] {
                assert_ne!(index, other_index);
                assert_ne!(binder.scope, other.scope);
                assert_ne!(binder.locus, other.locus);
            }
        }
        let filter = after.values.iter().find(|value| matches!(value.operation, w::ValueOperation::Query { operator: w::Query::Filter, .. })).unwrap();
        let outer = after.values.iter().find(|value| matches!(&value.operation, w::ValueOperation::Query { operator: w::Query::ForAll, collection, .. } if after.values[collection.index as usize].original_expression == filter.original_expression)).unwrap();
        let inner = after.values.iter().find(|value| matches!(value.operation, w::ValueOperation::Query { operator: w::Query::Exists, .. })).unwrap();
        let w::ValueOperation::Query { binder, .. } = &outer.operation else { panic!("outer forall") };
        let outer_scope = &after.binders[binder.index as usize].scope;
        assert_eq!(filter.scope, outer.scope, "nested collection is outside the outer binder scope");
        assert_eq!(&inner.scope, outer_scope, "nested body inherits its actual outer binder scope");
        let w::ValueOperation::Query { collection, .. } = &filter.operation else { panic!("filter") };
        assert_eq!(after.values[collection.index as usize].origin, initial.origin, "reading the captured collection does not retag it as post");
        let emitted = native::emit(&admitted, Limits::default()).into_result().unwrap();
        assert_eq!(inputs.read(proofs, &emitted).into_result().unwrap().package(), package);
    });
}

#[test]
#[trace("TC-119", "TC-121", "FR-040-AC-5", "FR-042-AC-3")]
fn unsafe_element_body_and_unfinished_proof_cannot_create_native_emission_authority() {
    let inputs = Inputs::new(&[
        Unit { name: "unsafe-query", body: "predicate Queries using S (input: M::Node): Boolean { exists(item in input.items: item + 1 <= 1000) }", declarations: &["Queries"] },
        Unit { name: "unsafe-flow", body: FLOW, declarations: &["Flow"] },
    ]);
    inputs.with_proofs(TypeLimits::default(), proofs::ProofLimits::default(), |proofs, selected| {
        let namespace = proofs.types().binding().namespace();
        let [query] = namespace.lookup("Queries") else { panic!("query source") };
        let [flow] = namespace.lookup("Flow") else { panic!("flow source") };
        assert_eq!(proofs.types().disposition(*query), Some(TypeDisposition::Typed));
        let refused = proofs.declaration(*query).unwrap();
        assert_eq!(refused.disposition(), proofs::ProofDisposition::Refused);
            let cause = refused.causes().iter().find(|cause| matches!(&cause.kind, proofs::CauseKind::Unproved { diagnostics } if diagnostics.iter().any(|diagnostic| diagnostic.obligation_kind == Some(ir::DefinednessObligationKind::CheckedRange)))).unwrap();
        let unit = namespace.unit(cause.site.unit).unwrap();
        assert_eq!(&unit.source().text()[cause.site.span.start..cause.site.span.end], "item + 1");
        assert_eq!(cause.site.declaration, *query);
        let original = unit.expression(cause.site.expression.unwrap()).unwrap();
        assert_eq!(original.span, cause.site.span);
        assert!(proofs.declaration(*flow).unwrap().causes().iter().any(|cause| matches!(cause.kind, proofs::CauseKind::Dependency { target } if target == *query)));
        let report = native::admit(proofs, selected, Limits::default());
        assert_eq!(report.result().err(), Some(&Error::Unsupported(Unsupported::FamilyProof)));
    });
    let total = Inputs::new(&[
        Unit { name: "bounded-query", body: "predicate Queries using S (input: M::Node): Boolean { forall(item in input.items: item < 1000 implies item + 1 <= 1000) }", declarations: &["Queries"] },
        Unit { name: "bounded-flow", body: FLOW, declarations: &["Flow"] },
    ]);
    total.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits {
            goals: 0,
            ..proofs::ProofLimits::default()
        },
        |proofs, selected| {
            assert_eq!(
                proofs.exhaustion().unwrap().dimension,
                proofs::work::Dimension::Goals
            );
            let [query] = proofs.types().binding().namespace().lookup("Queries") else {
                panic!("query")
            };
            assert_eq!(
                proofs.types().disposition(*query),
                Some(TypeDisposition::Typed)
            );
            assert_eq!(
                proofs.declaration(*query).unwrap().disposition(),
                proofs::ProofDisposition::Unfinished
            );
            assert_eq!(
                native::admit(proofs, selected, Limits::default())
                    .result()
                    .err(),
                Some(&Error::Unsupported(Unsupported::FamilyProof))
            );
        },
    );
    total.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .unwrap();
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            assert_eq!(
                total
                    .read(proofs, &emitted)
                    .into_result()
                    .unwrap()
                    .package(),
                admitted.package()
            );
        },
    );
}
