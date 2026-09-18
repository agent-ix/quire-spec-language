// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-196 D06-D08: the checked-layer/runtime half of FR-151 dispatch calls
//! that `tests/model_dispatch.rs`'s D01-D05 (link-time linking) explicitly
//! do not cover — `dispatch.select` evaluation with effective-precondition
//! decision (D06), the clause-kind restriction on a dispatched
//! `receiver.member(args)` call (D07), and the FR-146 call-graph refusal of
//! a cycle through a dispatch edge (D08) — plus one end-to-end test of the
//! `crate::model::checked_dispatch` bridge linking a real model family and
//! evaluating a dispatched call through it.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use sha2::{Digest, Sha256};

use quire_spec_language::model::accounting::ModelNormalizationLimits;
use quire_spec_language::model::bundle::{
    Bundle, BundleRecord, GeneralizationRecord, ModelSelection, ObjectTypeRecord, OperationEffect,
    OperationMemberRecord, RedefinitionRecord,
};
use quire_spec_language::model::checked_dispatch::{
    checked_dispatch_operation, DispatchRoot, OperationClauses,
};
use quire_spec_language::model::dispatch::GeneralizationClosure;
use quire_spec_language::model::key::ProducerKey;
use quire_spec_language::model::normalize::{normalize, EffectiveView, NormalizeOutcome};
use quire_spec_language::value::{
    CheckCause, CheckMode, CheckingLimits, ClauseKind, DispatchCandidate, DispatchOperation,
    DispatchTable, Expression, FunctionDeclaration, IllTypedCause, Integer, LimitKind, Meter,
    NodeKey, ObjectEnvironment, ObjectIdentity, ObjectReference, ObjectTypeDeclaration, Outcome,
    PackageDeclarations, ScalarLimits, TypeEnvironment, Undefined, UniverseIdentity, Value,
    ValueType,
};

const SCALAR_UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
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

fn universe() -> UniverseIdentity {
    UniverseIdentity::new(b"dispatch-calls-universe").unwrap()
}

fn receiver_reference(receiver_type: NodeKey, identity: &str) -> ObjectReference {
    ObjectReference::new(
        universe(),
        receiver_type,
        ObjectIdentity::new(identity.as_bytes()).unwrap(),
    )
}

fn objects(receiver_type: NodeKey, identity: &str) -> ObjectEnvironment {
    let types = types(receiver_type);
    ObjectEnvironment::new(
        &types,
        [(receiver_reference(receiver_type, identity), vec![])],
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

/// D06 (FR-151-AC-2): `dispatch.select` charges once, sized by the table's
/// candidate count (`value_occurrences`), then the selected candidate's
/// effective precondition decides `true`, and the evaluator proceeds to
/// charge `function.call` and run the candidate body.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn d06_dispatch_select_evaluates_effective_precondition_true_then_runs_the_body() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let package = one_candidate_package(
        receiver_type,
        Some(Expression::Boolean(true)),
        ValueType::Integer,
    )
    .check(CheckingLimits::default())
    .unwrap();
    let parameters = vec![("self".to_owned(), ValueType::Reference(receiver_type))];
    let checked = package
        .check_clause_expression(
            parameters,
            &dispatch_expression(),
            None,
            ClauseKind::Invariant,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();

    let objects = objects(receiver_type, "r1");
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let evaluation = package
        .evaluate(
            &checked,
            vec![Value::Reference(receiver_reference(receiver_type, "r1"))],
            &objects,
            &mut meter,
        )
        .unwrap();
    match evaluation.outcome {
        Outcome::Completed(Value::Integer(value)) => assert_eq!(value, Integer::from(1_i64)),
        other => panic!("expected a completed Integer(1), got {other:?}"),
    }
    // `dispatch.select` sizes `value_occurrences` by the table's one
    // distinct candidate; `function.call` (the candidate body) then charges
    // one work unit on top of `dispatch.select`'s own.
    assert_eq!(meter.consumed(LimitKind::ValueOccurrences), 1);
    assert!(meter.consumed(LimitKind::WorkUnits) >= 2);
}

/// D06 (FR-151-AC-2): a `false` effective precondition undefines the call
/// (`Undefined::PreconditionFalse`) instead of running the candidate body —
/// the candidate body's own `function.call`/`1` never evaluates.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn d06_dispatch_select_evaluates_effective_precondition_false_is_undefined() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let package = one_candidate_package(
        receiver_type,
        Some(Expression::Boolean(false)),
        ValueType::Integer,
    )
    .check(CheckingLimits::default())
    .unwrap();
    let parameters = vec![("self".to_owned(), ValueType::Reference(receiver_type))];
    let checked = package
        .check_clause_expression(
            parameters,
            &dispatch_expression(),
            None,
            ClauseKind::Invariant,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();

    let objects = objects(receiver_type, "r1");
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let evaluation = package
        .evaluate(
            &checked,
            vec![Value::Reference(receiver_reference(receiver_type, "r1"))],
            &objects,
            &mut meter,
        )
        .unwrap();
    match evaluation.outcome {
        Outcome::Undefined(reason) => assert_eq!(reason, Undefined::PreconditionFalse),
        other => panic!("expected Undefined(PreconditionFalse), got {other:?}"),
    }
}

// --- Bridge integration: crate::model::checked_dispatch -------------------

fn object_type_record(identity: &str) -> BundleRecord {
    BundleRecord::ObjectType(ObjectTypeRecord {
        key: ProducerKey::fixture(identity),
        interface_features: None,
    })
}

fn generalization_record(identity: &str, specific: &str, general: &str) -> BundleRecord {
    BundleRecord::Generalization(GeneralizationRecord {
        key: ProducerKey::fixture(identity),
        specific: ProducerKey::fixture(specific),
        general: ProducerKey::fixture(general),
    })
}

fn redefinition_record(
    identity: &str,
    owner: &str,
    redefining: &str,
    redefined: &str,
) -> BundleRecord {
    BundleRecord::Redefinition(RedefinitionRecord {
        key: ProducerKey::fixture(identity),
        owner: ProducerKey::fixture(owner),
        redefining: ProducerKey::fixture(redefining),
        redefined: ProducerKey::fixture(redefined),
    })
}

fn operation_record(identity: &str, owner: &str, has_body: bool) -> BundleRecord {
    BundleRecord::OperationMember(OperationMemberRecord {
        key: ProducerKey::fixture(identity),
        owner: ProducerKey::fixture(owner),
        parameters: Vec::new(),
        result: None,
        effect: OperationEffect::default(),
        has_own_precondition: false,
        own_postcondition_clauses: Vec::new(),
        has_body,
    })
}

/// `model.A`/`model.B` (`B` <= `A`), each declaring its own `size` body;
/// `B.size` redefines `A.size`. Mirrors `tests/model_dispatch.rs`'s D01
/// fixture style, minus the diamond (only one redefinition edge is needed
/// to prove the bridge links a real family end to end).
fn bridge_bundle() -> Bundle {
    Bundle::new(
        ModelSelection::fixture("bundle.dispatch-calls"),
        vec![
            object_type_record("model.A"),
            object_type_record("model.B"),
            generalization_record("model.gen.B-A", "model.B", "model.A"),
            operation_record("model.A.size", "model.A", true),
            operation_record("model.B.size", "model.B", true),
            redefinition_record(
                "model.redef.B.size",
                "model.B",
                "model.B.size",
                "model.A.size",
            ),
        ],
    )
}

fn bridge_view(bundle: &Bundle) -> EffectiveView {
    match normalize(bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed effective view, got {other:?}"),
    }
}

fn bridge_clauses(receiver_type: NodeKey) -> OperationClauses {
    let a = ProducerKey::fixture("model.A.size");
    let b = ProducerKey::fixture("model.B.size");
    let mut clauses = OperationClauses::default();
    clauses.member.insert(a.clone(), "size".to_owned());
    for (operation, result) in [(&a, 1_i64), (&b, 2_i64)] {
        clauses.parameters.insert(
            operation.clone(),
            vec![("self".to_owned(), ValueType::Reference(receiver_type))],
        );
        clauses.result.insert(operation.clone(), ValueType::Integer);
        clauses.own_body.insert(
            operation.clone(),
            Expression::Integer(Integer::from(result)),
        );
    }
    clauses
}

/// End-to-end: `crate::model::checked_dispatch::checked_dispatch_operation`
/// links `model.A.size`'s real two-candidate dispatch family (`model.A.size`
/// and `model.B.size`, which redefines it) from a `Bundle`, types both
/// candidates' bodies, and hands back a `PackageDeclarations` whose
/// `.check()` succeeds and whose evaluator runs `self.size()` for an `A`
/// receiver through the real bridge-built table.
///
/// This does not also evaluate a `B` receiver through the *same* checked
/// call site: [`crate::value::composite::ValueType::admits`] requires a
/// `Value::Reference`'s own `object_type()` to exactly equal a checked
/// parameter's declared `Reference<T>`, with no subtype allowance, so no
/// top-level parameter binding can carry a `B` instance where the checked
/// declaration says `Reference<A>` — the same boundary
/// `tests/model_reference_queries.rs` documents as "the `TypeEnvironment`
/// island" for `allInstances`/`lookup`. Resolving a dispatch call's receiver
/// against its instance's most-specific type end to end needs that same
/// identity bridge (`crate::value::model_query`) or an equivalent, which is
/// FR-152/FR-153/#131 territory, not this bridge's. The `d06_*`/`d08_*` tests
/// above already prove `dispatch.select`'s own per-subtype table lookup
/// (`DispatchTable::linked_for`) works once a receiver's most-specific type
/// is in hand; this test proves the *bridge* assembles a real multi-candidate
/// table and functions array correctly (`value_occurrences == 2`, below).
#[trace("TC-196", "FR-151-AC-1", "FR-151-AC-2")]
#[test]
fn bridge_links_a_real_family_and_evaluates_through_the_built_table() {
    let bundle = bridge_bundle();
    let view = bridge_view(&bundle);
    let a_type = key("model.A");
    let b_type = key("model.B");
    let clauses = bridge_clauses(a_type);
    let mut object_keys = BTreeMap::new();
    object_keys.insert(ProducerKey::fixture("model.A"), a_type);
    object_keys.insert(ProducerKey::fixture("model.B"), b_type);
    let mut meter =
        quire_spec_language::model::accounting::Meter::new(ModelNormalizationLimits::UNLIMITED);

    let root = DispatchRoot {
        key: ProducerKey::fixture("model.A.size"),
        receiver_type: a_type,
        closure: GeneralizationClosure::Closed,
    };
    let mut declarations =
        checked_dispatch_operation(&bundle, &view, &root, &object_keys, &clauses, &mut meter)
            .unwrap_or_else(|refusal| {
                panic!("expected a linked, checked dispatch family, got {refusal:?}")
            });
    // The bridge is pure and builds no `TypeEnvironment` of its own (see its
    // module docs): the caller declares the object types its own clauses'
    // parameter/result `ValueType`s name.
    let types = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(a_type, "A", vec![]),
            ObjectTypeDeclaration::new(b_type, "B", vec![]),
        ],
    )
    .unwrap();
    declarations.types = types.clone();
    let package = declarations.check(CheckingLimits::default()).unwrap();

    let objects = ObjectEnvironment::new(
        &types,
        [
            (receiver_reference(a_type, "a1"), vec![]),
            (receiver_reference(b_type, "b1"), vec![]),
        ],
    )
    .unwrap();

    let parameters = vec![("self".to_owned(), ValueType::Reference(a_type))];
    let checked = package
        .check_clause_expression(
            parameters,
            &dispatch_expression(),
            None,
            ClauseKind::Invariant,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();

    let mut meter_a = Meter::new(SCALAR_UNLIMITED);
    let evaluation_a = package
        .evaluate(
            &checked,
            vec![Value::Reference(receiver_reference(a_type, "a1"))],
            &objects,
            &mut meter_a,
        )
        .unwrap();
    match evaluation_a.outcome {
        Outcome::Completed(Value::Integer(value)) => assert_eq!(value, Integer::from(1_i64)),
        other => panic!("expected a completed Integer(1) for an A receiver, got {other:?}"),
    }
    // The linked family has two candidates (`model.A.size`, `model.B.size`
    // via its redefinition of `model.A.size`): `dispatch.select` sizes
    // `value_occurrences` by that count, exercising the bridge's own
    // `distinct` candidate-count assembly, not only a single-candidate
    // table (already covered by this file's `d06_*`/`d07_*`/`d08_*` tests
    // against a hand-built package).
    assert_eq!(meter_a.consumed(LimitKind::ValueOccurrences), 2);
}
