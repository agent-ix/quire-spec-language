// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-196 D06-D08: the checked-layer/runtime half of FR-151 dispatch calls
//! that `tests/model_dispatch.rs`'s D01-D05 (link-time linking) explicitly
//! do not cover — `dispatch.select` evaluation with effective-precondition
//! decision (D06), the clause-kind restriction on a dispatched
//! `receiver.member(args)` call (D07), and the FR-146 call-graph refusal of
//! a cycle through a dispatch edge (D08) — plus one end-to-end test of the
//! `crate::model::checked_dispatch` bridge linking a real model family and
//! evaluating a dispatched call through it.

use ix_trace_rs::trace;
use sha2::{Digest, Sha256};

use quire_spec_language::value::{
    CheckCause, CheckMode, CheckingLimits, ClauseKind, DispatchCandidate, DispatchOperation,
    DispatchTable, Expression, FunctionDeclaration, IllTypedCause, Integer, NodeKey,
    ObjectTypeDeclaration, PackageDeclarations, TypeEnvironment, ValueType,
};

fn key(label: &str) -> NodeKey {
    let digest = format!("{:x}", Sha256::digest(label.as_bytes()));
    NodeKey::from_hex(&digest).unwrap()
}

fn types(receiver_type: NodeKey) -> TypeEnvironment {
    TypeEnvironment::new(
        [],
        [ObjectTypeDeclaration::new(
            receiver_type,
            "Receiver",
            vec![],
        )],
    )
    .unwrap()
}

fn dispatch_expression() -> Expression {
    Expression::Dispatch {
        receiver: Box::new(Expression::Name("self".to_owned())),
        member: "size".to_owned(),
        arguments: Vec::new(),
    }
}

/// A one-candidate dispatch package: `functions[0]` is the linked
/// candidate's body (always `1`); `functions[1]`, when `precondition` is
/// given, is its effective precondition. `dispatch_tables[0]` links
/// `receiver_type` to it; `dispatch_operations[0]` is the `self.size()`
/// call site's target, declared `result`.
fn one_candidate_package(
    receiver_type: NodeKey,
    precondition: Option<Expression>,
    result: ValueType,
) -> PackageDeclarations {
    let body_value = match &result {
        ValueType::Integer => Expression::Integer(Integer::from(1_i64)),
        ValueType::Boolean => Expression::Boolean(true),
        other => panic!("this fixture only supports Integer/Boolean results, not {other:?}"),
    };
    let mut functions = vec![FunctionDeclaration {
        name: "candidate.body".to_owned(),
        parameters: vec![("self".to_owned(), ValueType::Reference(receiver_type))],
        result: result.clone(),
        measure: None,
        body: body_value,
        clause_kind: ClauseKind::Body,
    }];
    let precondition_index = precondition.map(|expression| {
        functions.push(FunctionDeclaration {
            name: "candidate.precondition".to_owned(),
            parameters: vec![("self".to_owned(), ValueType::Reference(receiver_type))],
            result: ValueType::Boolean,
            measure: None,
            body: expression,
            clause_kind: ClauseKind::Precondition,
        });
        functions.len() - 1
    });
    let table = DispatchTable::new(
        vec![(
            receiver_type,
            DispatchCandidate {
                body: 0,
                precondition: precondition_index,
            },
        )],
        1,
    );
    PackageDeclarations {
        types: types(receiver_type),
        functions,
        dispatch_operations: vec![DispatchOperation {
            receiver_type,
            member: "size".to_owned(),
            parameters: Vec::new(),
            result,
            table: 0,
        }],
        dispatch_tables: vec![table],
        ..PackageDeclarations::default()
    }
}

/// D07 (FR-151-AC-2): a dispatched `receiver.member(args)` call is admitted
/// while checking an invariant, precondition or postcondition, and refused
/// `ill_typed`/`operator-ineligible` while checking a function or operation
/// body — the same `ClauseKind` context every ordinary named-function body
/// checks under.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn d07_dispatch_call_admitted_only_inside_invariant_precondition_postcondition() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let package = one_candidate_package(receiver_type, None, ValueType::Integer)
        .check(CheckingLimits::default())
        .unwrap();
    let parameters = vec![("self".to_owned(), ValueType::Reference(receiver_type))];
    let expression = dispatch_expression();

    for allowed in [
        ClauseKind::Invariant,
        ClauseKind::Precondition,
        ClauseKind::Postcondition,
    ] {
        let checked = package.check_clause_expression(
            parameters.clone(),
            &expression,
            None,
            allowed,
            CheckMode::Kernel,
            CheckingLimits::default(),
        );
        assert!(
            checked.is_ok(),
            "{allowed:?} should admit a dispatched call, got {checked:?}"
        );
    }

    let refusal = package
        .check_clause_expression(
            parameters,
            &expression,
            None,
            ClauseKind::Body,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .expect_err("a function/operation body must refuse a dispatched call");
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::OperatorIneligible)
    );
}
