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
    BinaryOperator, CheckCause, CheckMode, CheckRefusal, CheckingLimits, ClauseKind,
    DispatchCandidate, DispatchOperation, DispatchTable, Expression, FunctionDeclaration,
    IllTypedCause, Integer, IntegerInterval, LimitKind, Location, Meter, NodeKey,
    ObjectEnvironment, ObjectIdentity, ObjectReference, ObjectTypeDeclaration, Origin, Outcome,
    PackageDeclarations, PreconditionFailure, ScalarLimits, TypeEnvironment, Undefined,
    UniverseIdentity, Value, ValueType,
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

fn dispatch_expression_on(receiver: &str) -> Expression {
    Expression::Dispatch {
        receiver: Box::new(Expression::Name(receiver.to_owned())),
        member: "size".to_owned(),
        arguments: Vec::new(),
    }
}

fn dispatch_expression() -> Expression {
    dispatch_expression_on("self")
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
    let mut functions = vec![FunctionDeclaration::clause(
        "candidate.body",
        vec![("self".to_owned(), ValueType::Reference(receiver_type))],
        result.clone(),
        None,
        body_value,
        ClauseKind::Body,
    )];
    let precondition_index = precondition.map(|expression| {
        functions.push(FunctionDeclaration::clause(
            "candidate.precondition",
            vec![("self".to_owned(), ValueType::Reference(receiver_type))],
            ValueType::Boolean,
            None,
            expression,
            ClauseKind::Precondition,
        ));
        functions.len() - 1
    });
    // The precondition function (when present) is its own authored clause,
    // reachable from the static FR-146 call-graph regardless of the runtime
    // guard's own decision (`DispatchCandidate::precondition_clauses`); this
    // single-candidate fixture has no redefinition ancestry, so the set is
    // just the precondition function itself (D08's self-loop depends on this).
    let precondition_clauses = precondition_index.into_iter().collect();
    let table = DispatchTable::new(
        vec![(
            receiver_type,
            DispatchCandidate {
                body: 0,
                precondition: precondition_index,
                precondition_clauses,
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
#[trace("TC-196")]
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

/// D07, in TC-196's own shape: `function f using V(r: Reference<M::A>):
/// Integer pure { if r.size() >= 1 then 1 else 0 }` is refused
/// `ill_typed`/`operator-ineligible` at `r.size()` — an ordinary named
/// function admitted alongside the dispatch table, not a standalone
/// clause expression (finding #172-9).
#[trace("TC-196")]
#[test]
fn d07_own_shape_a_dispatch_call_inside_an_ordinary_function_body_is_refused() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let mut package = one_candidate_package(receiver_type, None, ValueType::Integer);
    let body = Expression::If {
        condition: Box::new(Expression::Binary {
            operator: BinaryOperator::GreaterOrEqual,
            left: Box::new(dispatch_expression_on("r")),
            right: Box::new(Expression::Integer(Integer::from(1_i64))),
        }),
        then: Box::new(Expression::Integer(Integer::from(1_i64))),
        otherwise: Box::new(Expression::Integer(Integer::from(0_i64))),
    };
    package.functions.push(FunctionDeclaration::new(
        "f",
        vec![("r".to_owned(), ValueType::Reference(receiver_type))],
        ValueType::Integer,
        None,
        body,
    ));
    let refusals = package
        .check(CheckingLimits::default())
        .expect_err("a dispatch call inside an ordinary function body must be refused");
    assert!(
        refusals.iter().any(|refusal: &CheckRefusal| refusal.cause
            == CheckCause::IllTyped(IllTypedCause::OperatorIneligible)),
        "expected an IllTyped/OperatorIneligible refusal, got {refusals:?}"
    );
}

/// FR-151 (`quire.model.dispatch.single/v1`): a dispatch call argument is
/// type-checked "with reference upcasts only" — never the `Integer`/`Int[..]`
/// widening an ordinary call's argument admits (finding #172-10).
#[trace("TC-196")]
#[test]
fn dispatch_argument_never_admits_integer_to_int_coercion() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let narrow =
        ValueType::Int(IntegerInterval::new(Integer::from(0_i64), Integer::from(10_i64)).unwrap());
    let mut package = one_candidate_package(receiver_type, None, ValueType::Integer);
    package.functions[0]
        .parameters
        .push(("n".to_owned(), narrow.clone()));
    package.dispatch_operations[0].parameters = vec![narrow];
    let checked_package = package.check(CheckingLimits::default()).unwrap();
    let parameters = vec![
        ("self".to_owned(), ValueType::Reference(receiver_type)),
        ("n".to_owned(), ValueType::Integer),
    ];
    let call = Expression::Dispatch {
        receiver: Box::new(Expression::Name("self".to_owned())),
        member: "size".to_owned(),
        arguments: vec![Expression::Name("n".to_owned())],
    };
    let refusal = checked_package
        .check_clause_expression(
            parameters,
            &call,
            None,
            ClauseKind::Invariant,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .expect_err("an unbounded Integer argument must not coerce into a dispatch operation's Int[..] parameter");
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
}

/// D07 (finding #172-6): a synthesized dispatch candidate body is never
/// `callable_by_name` — an ordinary named `Call` targeting it by its
/// internal name (`"candidate.body"`) is refused `missing-name`, the same
/// as any other undeclared name, closing the bypass that would otherwise
/// let an ordinary function body reach a dispatch candidate's own body or
/// precondition clause without ever going through `dispatch_call`'s
/// clause-kind restriction.
#[trace("TC-196")]
#[test]
fn synthesized_dispatch_candidate_is_not_callable_by_name() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let mut package = one_candidate_package(receiver_type, None, ValueType::Integer);
    package.functions.push(FunctionDeclaration::new(
        "f",
        vec![("self".to_owned(), ValueType::Reference(receiver_type))],
        ValueType::Integer,
        None,
        Expression::Call {
            name: "candidate.body".to_owned(),
            arguments: vec![Expression::Name("self".to_owned())],
        },
    ));
    let refusals = package
        .check(CheckingLimits::default())
        .expect_err("a synthesized candidate body must not be callable by name");
    assert!(
        refusals.iter().any(|refusal: &CheckRefusal| refusal.cause
            == CheckCause::MissingName("candidate.body".to_owned())),
        "expected a MissingName(\"candidate.body\") refusal, got {refusals:?}"
    );
}

/// FR-151 (finding #172-4): an out-of-range candidate body function index
/// is refused `invalid_package`/`invalid-value` upfront, before any node is
/// typed — never an `unwrap_or_default`/panic on the malformed table.
#[trace("TC-196")]
#[test]
fn dispatch_candidate_with_an_out_of_range_body_index_is_refused_invalid_dispatch() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let mut package = one_candidate_package(receiver_type, None, ValueType::Integer);
    package.dispatch_tables[0] = DispatchTable::new(
        vec![(
            receiver_type,
            DispatchCandidate {
                body: 99,
                precondition: None,
                precondition_clauses: Vec::new(),
            },
        )],
        1,
    );
    let refusals = package
        .check(CheckingLimits::default())
        .expect_err("an out-of-range candidate body index must be refused");
    assert!(
        refusals.iter().any(|refusal: &CheckRefusal| matches!(
            &refusal.cause,
            CheckCause::InvalidDispatchDeclaration { detail }
                if detail.contains("out of range")
        )),
        "expected an InvalidDispatchDeclaration/out-of-range refusal, got {refusals:?}"
    );
}

/// FR-151 (finding #172-4): a candidate body whose declared parameter count
/// does not match the dispatch operation's own arity (receiver plus
/// arguments) is refused `invalid_package`/`invalid-value`, not admitted
/// with a silently mismatched call frame.
#[trace("TC-196")]
#[test]
fn dispatch_candidate_with_a_mismatched_arity_is_refused_invalid_dispatch() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let mut package = one_candidate_package(receiver_type, None, ValueType::Integer);
    // The operation declares one extra Integer argument that no candidate
    // function accepts (`one_candidate_package`'s body only takes `self`).
    package.dispatch_operations[0].parameters = vec![ValueType::Integer];
    let refusals = package
        .check(CheckingLimits::default())
        .expect_err("a candidate/operation arity mismatch must be refused");
    assert!(
        refusals.iter().any(|refusal: &CheckRefusal| matches!(
            &refusal.cause,
            CheckCause::InvalidDispatchDeclaration { detail } if detail.contains("parameters")
        )),
        "expected an InvalidDispatchDeclaration/arity refusal, got {refusals:?}"
    );
}

/// A two-candidate `model.A`/`model.B` (`B <= A`) bridge fixture, `size`
/// returning `1` for A's own body and `2` for B's, with an optional own
/// precondition on each (`PA`/`PB`). `B.size` redefines `A.size`.
fn ab_bridge_clauses(
    receiver_type: NodeKey,
    pa: Option<Expression>,
    pb: Option<Expression>,
) -> OperationClauses {
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
    if let Some(pa) = pa {
        clauses.own_precondition.insert(a, pa);
    }
    if let Some(pb) = pb {
        clauses.own_precondition.insert(b, pb);
    }
    clauses
}

/// Links the `model.A`/`model.B` bridge fixture (see [`bridge_bundle`]) with
/// the given `PA`/`PB` own-precondition clauses, for a call site whose
/// static receiver type is `receiver_type` and whose root operation is
/// `model.A.size`.
fn ab_bridge_package(
    receiver_type: NodeKey,
    pa: Option<Expression>,
    pb: Option<Expression>,
) -> quire_spec_language::value::CheckedPackage {
    let bundle = bridge_bundle();
    let view = bridge_view(&bundle);
    let a_type = key("model.A");
    let b_type = key("model.B");
    let clauses = ab_bridge_clauses(a_type, pa, pb);
    let mut object_keys = BTreeMap::new();
    object_keys.insert(ProducerKey::fixture("model.A"), a_type);
    object_keys.insert(ProducerKey::fixture("model.B"), b_type);
    let mut meter =
        quire_spec_language::model::accounting::Meter::new(ModelNormalizationLimits::UNLIMITED);
    let root = DispatchRoot {
        key: ProducerKey::fixture("model.A.size"),
        receiver_type,
        closure: GeneralizationClosure::Closed,
    };
    let mut declarations =
        checked_dispatch_operation(&bundle, &view, &root, &object_keys, &clauses, &mut meter)
            .unwrap_or_else(|refusal| {
                panic!("expected a linked, checked dispatch family, got {refusal:?}")
            });
    let types = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(a_type, "A", vec![]),
            ObjectTypeDeclaration::new(b_type, "B", vec![]),
        ],
    )
    .unwrap();
    declarations.types = types;
    declarations.check(CheckingLimits::default()).unwrap()
}

/// D06 (FR-151-AC-7/AC-8): a `B`-typed receiver, statically declared
/// `Reference<B>` at the call site, always selects `B.size` — the
/// most-specific redefining candidate — never `A.size`, the less-specific
/// method `B.size` redefines. `B` declares no own precondition at all, and
/// `A`'s own precondition is `false`: since the absent-precondition-is-true
/// rule (finding #172-1) makes `B`'s effective precondition `true`
/// unconditionally (its own clause is absent, so no ancestor's precondition
/// is even consulted), the call still completes with `B`'s own body value,
/// proving both properties at once.
#[trace("TC-196", "FR-151-AC-7", "FR-151-AC-8")]
#[test]
fn d06_bridge_absent_precondition_selects_most_specific_never_the_less_specific() {
    let b_type = key("model.B");
    let package = ab_bridge_package(b_type, Some(Expression::Boolean(false)), None);

    let objects = objects(b_type, "b1");
    let parameters = vec![("self".to_owned(), ValueType::Reference(b_type))];
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

    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let evaluation = package
        .evaluate(
            &checked,
            vec![Value::Reference(receiver_reference(b_type, "b1"))],
            &objects,
            &mut meter,
        )
        .unwrap();
    match evaluation.outcome {
        Outcome::Completed(Value::Integer(value)) => assert_eq!(value, Integer::from(2_i64)),
        other => panic!("expected a completed Integer(2) (B's own body), got {other:?}"),
    }
    // `dispatch.select` sizes `value_occurrences` by the family's two
    // distinct candidates; `function.call` then charges exactly one work
    // unit for B's own body (never A's — A's body is never even
    // considered, matching zero additional `function.call` charges).
    assert_eq!(meter.consumed(LimitKind::ValueOccurrences), 2);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 3);
}

/// D06 (FR-151-AC-7/AC-8): an `A`-typed receiver with `A`'s own precondition
/// `false` (no ancestor to inherit from — `A` is the family's root) is
/// refused `Undefined::PreconditionFalse`, and the candidate body's own
/// `function.call` never charges: only `dispatch.select`'s own charge is
/// consumed.
#[trace("TC-196", "FR-151-AC-7", "FR-151-AC-8")]
#[test]
fn d06_bridge_false_precondition_is_undefined_and_never_charges_function_call() {
    let a_type = key("model.A");
    let package = ab_bridge_package(a_type, Some(Expression::Boolean(false)), None);

    let objects = objects(a_type, "a1");
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

    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let evaluation = package
        .evaluate(
            &checked,
            vec![Value::Reference(receiver_reference(a_type, "a1"))],
            &objects,
            &mut meter,
        )
        .unwrap();
    match evaluation.outcome {
        Outcome::Undefined(Undefined::PreconditionFalse(failure)) => {
            assert_eq!(
                *failure,
                PreconditionFailure {
                    operation: "size".to_owned(),
                    selected: "model.A.size".to_owned(),
                    receiver: receiver_reference(a_type, "a1"),
                }
            );
        }
        other => panic!("expected Undefined(PreconditionFalse), got {other:?}"),
    }
    // Only `dispatch.select` charges (sized by the family's two distinct
    // candidates); the candidate body's own `function.call` never runs.
    assert_eq!(meter.consumed(LimitKind::ValueOccurrences), 2);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 2);
}

/// D08 (FR-151-AC-3): a strongly connected component reached only through a
/// dispatch edge is refused `invalid_package`/`definition-cycle` — here the
/// candidate's own effective precondition dispatches back to itself, the
/// smallest possible dispatch cycle (call graph nodes `{"candidate.precondition"}`,
/// a single self-loop edge `"candidate.precondition" -> "candidate.precondition"`)
/// — before the measure-decrease obligation (which this pseudo-function,
/// having no `decreases` clause, would otherwise fail on `missing-measure`)
/// is even attempted.
#[trace("TC-196", "FR-151-AC-3")]
#[test]
fn d08_a_cycle_through_a_dispatch_edge_is_refused_definition_cycle() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let package = one_candidate_package(
        receiver_type,
        Some(dispatch_expression()),
        ValueType::Boolean,
    );
    let refusals = package
        .check(CheckingLimits::default())
        .expect_err("a dispatch-edge cycle must be refused");
    let refusal = refusals
        .iter()
        .find(|refusal: &&CheckRefusal| matches!(refusal.cause, CheckCause::DefinitionCycle { .. }))
        .unwrap_or_else(|| panic!("expected a DefinitionCycle refusal, got {refusals:?}"));
    assert_eq!(
        refusal.cause,
        CheckCause::DefinitionCycle {
            edges: vec![(
                "candidate.precondition".to_owned(),
                "candidate.precondition".to_owned(),
            )],
        }
    );
    assert_eq!(
        refusal.location,
        Location {
            origin: Origin::Body {
                function: "candidate.precondition".to_owned(),
                index: 1,
            },
            path: Vec::new(),
        }
    );
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
#[trace("TC-196")]
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
