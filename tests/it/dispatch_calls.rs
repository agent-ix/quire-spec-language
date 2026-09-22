// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-196 D06-D08: the checked-layer/runtime half of FR-151 dispatch calls
//! that `tests/model_dispatch.rs`'s D01-D05 (link-time linking) explicitly
//! do not cover — `dispatch.select` evaluation with effective-precondition
//! decision (D06), the clause-kind restriction on a dispatched
//! `receiver.member(args)` call (D07), and the FR-146 call-graph refusal of
//! a cycle through a dispatch edge (D08) — plus one end-to-end test of the
//! `crate::check::checked_dispatch` bridge (ADR-011 §7.3 M-2, QSL-7: moved
//! here from `crate::model::checked_dispatch`) linking a real model family
//! and evaluating a dispatched call through it.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use sha2::{Digest, Sha256};

use qsl_foundation::diagnostic::Code;
use quire_exact::{Integer, IntegerInterval, LimitKind, Meter, ScalarLimits};
use quire_spec_language::check::{
    checked_dispatch_operation, DispatchBridgeRefusal, DispatchRoot, OperationClauses,
};
use quire_spec_language::model::accounting::ModelNormalizationLimits;
use quire_spec_language::model::dispatch::GeneralizationClosure;
use quire_spec_language::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Multiplicity, ObjectTypeRecord,
    OperationEffect, OperationMemberRecord, OperationResult, ValueTypeRef,
};
use quire_spec_language::model::key::DeclarationKey;
use quire_spec_language::model::normalize::{
    normalize, EffectiveView, ModelRefusalCause, NormalizeOutcome,
};
use quire_spec_language::value::{
    BinaryOperator, CallFailure, CheckCause, CheckMode, CheckRefusal, CheckedPackage,
    CheckingLimitKind, CheckingLimits, CheckingStage, ClauseKind, DeclaredClauseKind,
    DispatchCandidate, DispatchFunctionRole, DispatchOperation, DispatchTable, Expression,
    FunctionDeclaration, IllTypedCause, InputRefusal, InvalidDispatchDeclaration, Location,
    NodeKey, ObjectEnvironment, ObjectIdentity, ObjectReference, ObjectTypeDeclaration, Origin,
    Outcome, PackageDeclarations, PreconditionFailure, QualifiedName, TypeEnvironment, Undefined,
    UniverseIdentity, Value, ValueType,
};

// This crate's own `value::Origin` (imported above) is a different type
// from `quire_exact::Origin` -- `CheckedPackage::occurrence` takes the
// latter (ADR-013 O-07's kernel occurrence key), never the former, so both
// are in scope here under distinct names.
use quire_exact::Origin as ExactOrigin;
use quire_exact::Role as ExactRole;

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
    NodeKey::from_digest(Sha256::digest(label.as_bytes()).into())
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
        DeclaredClauseKind::Body,
    )];
    let precondition_index = precondition.map(|expression| {
        functions.push(FunctionDeclaration::clause(
            "candidate.precondition",
            vec![("self".to_owned(), ValueType::Reference(receiver_type))],
            ValueType::Boolean,
            None,
            expression,
            DeclaredClauseKind::Precondition,
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
/// clause expression.
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
/// widening an ordinary call's argument admits.
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

/// H1 (#204 round 1), FR-151 (`quire.model.dispatch.single/v1`): a dispatch
/// call argument admits a reference upcast -- an argument statically typed
/// as a proper subtype of the declared parameter type is admitted, and the
/// checked node's own static type narrows to the declared parameter type.
#[trace("TC-196", "FR-151")]
#[test]
fn dispatch_argument_admits_a_reference_upcast() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let super_type = key("model.dispatch-calls.Super");
    let sub_type = key("model.dispatch-calls.Sub");
    let mut package = one_candidate_package(receiver_type, None, ValueType::Integer);
    package.functions[0]
        .parameters
        .push(("arg".to_owned(), ValueType::Reference(super_type)));
    package.dispatch_operations[0].parameters = vec![ValueType::Reference(super_type)];
    package.types = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(receiver_type, "Receiver", vec![]),
            ObjectTypeDeclaration::new(super_type, "Super", vec![]),
            ObjectTypeDeclaration::new(sub_type, "Sub", vec![]).with_supertypes(vec![super_type]),
        ],
    )
    .unwrap();
    let checked_package = package.check(CheckingLimits::default()).unwrap();
    let parameters = vec![
        ("self".to_owned(), ValueType::Reference(receiver_type)),
        ("arg".to_owned(), ValueType::Reference(sub_type)),
    ];
    let call = Expression::Dispatch {
        receiver: Box::new(Expression::Name("self".to_owned())),
        member: "size".to_owned(),
        arguments: vec![Expression::Name("arg".to_owned())],
    };
    let checked = checked_package
        .check_clause_expression(
            parameters,
            &call,
            None,
            ClauseKind::Invariant,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .expect("a proper subtype argument must be admitted as a static upcast");
    assert_eq!(*checked.value_type(), ValueType::Integer);
}

/// H1 (#204 round 1), FR-151: the upcast admission is one-directional --
/// passing an argument statically typed as a proper *supertype* of the
/// declared parameter (the "downcast" direction) is refused
/// `ill_typed`/`type-mismatch`, exactly as an unrelated type would be.
#[trace("TC-196", "FR-151")]
#[test]
fn dispatch_argument_refuses_a_reference_downcast() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let super_type = key("model.dispatch-calls.Super");
    let sub_type = key("model.dispatch-calls.Sub");
    let mut package = one_candidate_package(receiver_type, None, ValueType::Integer);
    package.functions[0]
        .parameters
        .push(("arg".to_owned(), ValueType::Reference(sub_type)));
    package.dispatch_operations[0].parameters = vec![ValueType::Reference(sub_type)];
    package.types = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(receiver_type, "Receiver", vec![]),
            ObjectTypeDeclaration::new(super_type, "Super", vec![]),
            ObjectTypeDeclaration::new(sub_type, "Sub", vec![]).with_supertypes(vec![super_type]),
        ],
    )
    .unwrap();
    let checked_package = package.check(CheckingLimits::default()).unwrap();
    let parameters = vec![
        ("self".to_owned(), ValueType::Reference(receiver_type)),
        ("arg".to_owned(), ValueType::Reference(super_type)),
    ];
    let call = Expression::Dispatch {
        receiver: Box::new(Expression::Name("self".to_owned())),
        member: "size".to_owned(),
        arguments: vec![Expression::Name("arg".to_owned())],
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
        .expect_err("a supertype argument must never be admitted where a subtype is required");
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
}

/// H1 (#204 round 1), FR-151: an argument statically typed as a reference to
/// an object type unrelated to the declared parameter (neither an ancestor
/// nor a descendant) is refused `ill_typed`/`type-mismatch`.
#[trace("TC-196", "FR-151")]
#[test]
fn dispatch_argument_refuses_an_unrelated_reference_type() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let super_type = key("model.dispatch-calls.Super");
    let unrelated_type = key("model.dispatch-calls.Unrelated");
    let mut package = one_candidate_package(receiver_type, None, ValueType::Integer);
    package.functions[0]
        .parameters
        .push(("arg".to_owned(), ValueType::Reference(super_type)));
    package.dispatch_operations[0].parameters = vec![ValueType::Reference(super_type)];
    package.types = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(receiver_type, "Receiver", vec![]),
            ObjectTypeDeclaration::new(super_type, "Super", vec![]),
            ObjectTypeDeclaration::new(unrelated_type, "Unrelated", vec![]),
        ],
    )
    .unwrap();
    let checked_package = package.check(CheckingLimits::default()).unwrap();
    let parameters = vec![
        ("self".to_owned(), ValueType::Reference(receiver_type)),
        ("arg".to_owned(), ValueType::Reference(unrelated_type)),
    ];
    let call = Expression::Dispatch {
        receiver: Box::new(Expression::Name("self".to_owned())),
        member: "size".to_owned(),
        arguments: vec![Expression::Name("arg".to_owned())],
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
        .expect_err("an unrelated reference type must never be admitted");
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
}

/// D07: a synthesized dispatch candidate body is never
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

/// D07's bypass closed at the other entry point: `CheckedPackage::call`, the
/// public runtime path that invokes a function by name directly (never
/// through a checked expression tree, so `check.rs`'s own
/// `callable_by_name` gate on `Expression::Call` never runs), must not be
/// able to reach a synthesized dispatch candidate body either.
///
/// **Rebuilt (PR #262 review, finding F5).** An earlier version of this
/// test named the target with a two-segment `QualifiedName` (`["candidate",
/// "body"]`), reasoning that a real FR-151 synthesized name always contains
/// a literal `.` and so can never be a single identifier segment
/// `QualifiedName::new` accepts (`is_identifier` admits no `.`). That is
/// true, but it means the test never reached the `callable_by_name` filter
/// at all: `function.as_unqualified()` already returns `None` for two
/// segments, so `package.call` refuses at that guard, before
/// `self.function(name).filter(|(_, checked)|
/// checked.signature.callable_by_name)` -- the actual mechanism this test
/// exists to probe -- ever runs. Deleting the `.filter(...)` call still made
/// the old assertion pass, which is exactly F5's "test that cannot fail the
/// way it is written" shape. This version instead builds a minimal package
/// (no dispatch table needed) whose one function is declared through
/// [`FunctionDeclaration::clause`] -- the same crate-internal, never
/// name-callable constructor FR-151 dispatch candidates use -- under a
/// name that *is* a single plain identifier (`FunctionDeclaration::clause`
/// itself places no dot-free restriction on `name`; only `QualifiedName`
/// does), so `as_unqualified()` succeeds, `self.function` finds a real
/// declaration by that identifier, and refusal can only come from the
/// `callable_by_name` filter itself.
#[trace("TC-196")]
#[test]
fn checked_package_call_refuses_a_non_callable_by_name_function_found_by_lookup() {
    let graph = PackageDeclarations {
        functions: vec![FunctionDeclaration::clause(
            "internal_guard",
            vec![],
            ValueType::Boolean,
            None,
            Expression::Boolean(true),
            DeclaredClauseKind::Body,
        )],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .expect("a single clause-kind function with no dispatch table checks cleanly");
    let package = CheckedPackage::link(graph);
    let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let refusal = package
        .call(
            &QualifiedName::unqualified("internal_guard").unwrap(),
            Vec::new(),
            &objects,
            &mut meter,
        )
        .expect_err(
            "a function declared through FunctionDeclaration::clause must not be callable by \
             name even when found by plain-identifier lookup",
        );
    assert_eq!(
        refusal,
        CallFailure::Input(InputRefusal::UnknownFunction("internal_guard".to_owned()))
    );
}

/// FR-065-AC-2's reordering clause: a declaration's checked identity does
/// not depend on any other declaration's existence or position in the
/// package.
///
/// **Rebuilt (PR #262 review, coordinator round 3).** The previous version
/// (`identity_ignores_unrelated_declarations`, `src/check/family.rs`
/// post-QSL-139, formerly `src/value/expression/family.rs`, `TC-163`) minted
/// the same identity twice from the same
/// `FunctionDeclaration` and compared it to itself; no second declaration
/// was ever constructed, so there was nothing for the property to be
/// independent *of*. This version checks two packages whose two functions
/// are declared in opposite order and compares `target`'s checked identity
/// across both -- a real reordering, at the one level position could
/// actually leak (`PackageDeclarations::check`; see
/// `mint_declaration_identity`'s own doc on why a typed body's
/// `NodeKind::Call { function: usize, .. }` index makes checked position
/// matter even though the *parsed* preimage this identity hashes does not).
///
/// Also gives a real, non-fabricated test caller to four `pub`
/// `CheckedPackage` methods PR #262 review (coordinator round 3, finding 3)
/// found with zero callers and zero tests anywhere in the crate:
/// `function_identity` (read back here across both orderings), `occurrence`
/// (the target's own declaration occurrence), and the
/// `emit_function_package_v2`/`decode_function_package_v2` round trip
/// (which also exercises `family::link_function_identity`, `emit_function_
/// package_v2`'s own one caller, previously itself uncalled).
#[trace("TC-163", "FR-065-AC-2")]
#[test]
fn function_identity_survives_reordering_check_linking_and_a_v2_round_trip() {
    fn declaration(name: &str, body: Expression) -> FunctionDeclaration {
        FunctionDeclaration::new(name, Vec::new(), ValueType::Boolean, None, body)
    }

    let target = declaration("target", Expression::Boolean(true));
    let unrelated = declaration("unrelated", Expression::Boolean(false));

    let target_first = PackageDeclarations {
        functions: vec![target.clone(), unrelated.clone()],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .expect("two unrelated boolean-literal functions check cleanly");
    let unrelated_first = PackageDeclarations {
        functions: vec![unrelated, target],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .expect("reordering the same two declarations checks cleanly too");

    let identity_target_first = target_first
        .function_identity("target")
        .expect("target is declared in this package");
    let identity_unrelated_first = unrelated_first
        .function_identity("target")
        .expect("target is declared in this package, just declared second here");
    assert_eq!(
        identity_target_first, identity_unrelated_first,
        "target's checked identity must not depend on unrelated's existence or position"
    );

    let origin = ExactOrigin::new(ExactRole::new("declaration"), 0);
    assert!(
        target_first
            .occurrence(identity_target_first, &origin)
            .is_some(),
        "check must record target's own declaration occurrence, resolvable by (identity, origin)"
    );

    let target_name = QualifiedName::unqualified("target").expect("\"target\" is an identifier");
    let target_first = CheckedPackage::link(target_first);
    let bytes = target_first
        .emit_function_package_v2()
        .expect("every declared name here is identifier-shaped");
    let decoded = CheckedPackage::decode_function_package_v2(&bytes)
        .expect("this crate's own emit_function_package_v2 output decodes cleanly");
    let decoded_identity = decoded
        .into_iter()
        .find(|(name, _)| *name == target_name)
        .map(|(_, identity)| identity)
        .expect("target survives the v2 round trip");
    assert_eq!(
        decoded_identity, identity_target_first,
        "identity must survive check, S4 linking and a v2 emit/decode round trip unchanged"
    );

    // FR-065-AC-2 says identity is unchanged at all three checkpoints
    // (check, linking, v2) *under reordering* -- the assertions above only
    // exercise `target_first`'s v2 round trip; without this, `unrelated_
    // first` (the reordered package) is checked and its `function_identity`
    // compared, but never itself emitted to v2, so the reordering claim was
    // only half-covered at the v2 checkpoint (PR #262 review round 4, item
    // 6). Round-trip `unrelated_first` too and compare against the same
    // target identity.
    let unrelated_first = CheckedPackage::link(unrelated_first);
    let unrelated_first_bytes = unrelated_first
        .emit_function_package_v2()
        .expect("every declared name here is identifier-shaped");
    let unrelated_first_decoded =
        CheckedPackage::decode_function_package_v2(&unrelated_first_bytes)
            .expect("unrelated_first's own emit_function_package_v2 output decodes cleanly");
    let unrelated_first_decoded_identity = unrelated_first_decoded
        .into_iter()
        .find(|(name, _)| *name == target_name)
        .map(|(_, identity)| identity)
        .expect("target survives the v2 round trip from the reordered package too");
    assert_eq!(
        unrelated_first_decoded_identity, identity_target_first,
        "target's identity must survive check, linking and a v2 round trip identically \
         regardless of unrelated's position"
    );
}

/// PR #262 review, finding F4: `PackageDeclarations::check` used to hardcode
/// the checked-family contract's own nesting-depth `StageLimits` at
/// `MAX_CHECKING_DEPTH`, ignoring the caller's own `CheckingLimits` entirely
/// -- so the contract's `StageFailure::Limit` -> `CheckCause::
/// ResourceExhausted` arm was dead through this, the only production entry
/// point that reaches it. This test calls `check` with `CheckingLimits::new(
/// _, 0)` (this method's own `depth()` bound wired through, per this
/// finding's fix) and asserts a real `ResourceExhausted` refusal comes back
/// -- if the wiring reverts to the hardcoded constant, `enter_nesting`'s
/// `0 >= 128` never holds and this package checks cleanly instead, failing
/// this test.
///
/// **Untagged.** This test's zero-depth `CheckingLimits` refuses before
/// `ValueFunctionFamily::check` ever reaches `check_declaration_body`,
/// because `check`'s contract-level `StageLimits.nesting_depth` is derived
/// from the same caller-supplied `CheckingLimits.depth()`
/// (`PackageDeclarations::check`, `src/check/mod.rs`) and is charged once
/// per top-level declaration -- it proves that wiring is live through the
/// public API, not real recursive descent. QSL-148 has since moved real
/// recursive checking into `check_declaration_body`, reached from
/// `ValueFunctionFamily::check`, and `Typer`'s own separate
/// `CheckingLimits.depth` bound on that real descent is demonstrated by
/// `real_checker_depth_limit_is_the_proximate_cause`
/// (`value::expression::family`, also untagged) -- but neither test backs
/// FR-062-AC-7 itself: that criterion's own `Limit`-outcome-on-real-descent
/// requirement would need `CheckContext` threaded through `Typer`'s
/// recursive engine, which is out of scope here. See FR-062's own Status
/// section, AC-7 row, for the full reasoning.
#[test]
fn contract_nesting_limit_reflects_the_callers_own_checking_limits() {
    let declaration = |name: &str| {
        FunctionDeclaration::new(
            name,
            Vec::new(),
            ValueType::Boolean,
            None,
            Expression::Boolean(true),
        )
    };
    let tight_limits = CheckingLimits::new(u64::MAX, 0).expect("0 is within MAX_CHECKING_DEPTH");
    let refused = PackageDeclarations {
        functions: vec![declaration("f")],
        ..PackageDeclarations::default()
    }
    .check(tight_limits)
    .expect_err("a zero-depth limit must refuse every declaration's contract-level check");
    assert!(
        refused.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::ResourceExhausted {
                stage: CheckingStage::Typing,
                kind: CheckingLimitKind::Depth,
                limit: 0,
            }
        )),
        "expected a contract-level ResourceExhausted(Depth, limit=0) refusal, got {refused:?}"
    );

    let admitting_limits = CheckingLimits::default();
    PackageDeclarations {
        functions: vec![declaration("f")],
        ..PackageDeclarations::default()
    }
    .check(admitting_limits)
    .expect("the default depth limit admits an ordinary boolean-literal function");
}

/// [`FunctionDeclaration::clause`] takes [`DeclaredClauseKind`], which has
/// no `Postcondition` variant: the only path that can actually stand behind
/// a real postcondition, `CheckedPackage::check_postcondition_expression`,
/// remains fully reachable and unaffected by that narrowing — it is a
/// standalone expression check outside `PackageDeclarations::functions`
/// entirely, never routed through `FunctionDeclaration::clause` at all.
#[trace("TC-196")]
#[test]
fn check_postcondition_expression_still_admits_a_real_postcondition() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let package = one_candidate_package(receiver_type, None, ValueType::Integer)
        .check(CheckingLimits::default())
        .unwrap();
    let parameters = vec![("self".to_owned(), ValueType::Reference(receiver_type))];
    let checked = package
        .check_postcondition_expression(
            parameters,
            &Expression::Boolean(true),
            Some(&ValueType::Boolean),
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .expect("check_postcondition_expression must still admit an ordinary postcondition");
    assert_eq!(checked.value_type(), &ValueType::Boolean);
}

/// FR-151: an out-of-range candidate body function index is refused
/// `invalid_package`/`invalid-value` upfront, before any node is typed —
/// never an `unwrap_or_default`/panic on the malformed table — and the
/// refusal names the out-of-range role and index rather than a free-form
/// string.
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
            CheckCause::InvalidDispatchDeclaration(
                InvalidDispatchDeclaration::FunctionOutOfRange {
                    role: DispatchFunctionRole::Body,
                    index: 99,
                }
            )
        )),
        "expected an InvalidDispatchDeclaration/FunctionOutOfRange refusal, got {refusals:?}"
    );
}

/// FR-151: a candidate body whose declared parameter count does not match
/// the dispatch operation's own arity (receiver plus arguments) is refused
/// `invalid_package`/`invalid-value`, not admitted with a silently
/// mismatched call frame — and the refusal is located at the candidate
/// function's own declaration, not the expression root.
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
            CheckCause::InvalidDispatchDeclaration(InvalidDispatchDeclaration::Arity {
                role: DispatchFunctionRole::Body,
                index: 0,
                declared: 1,
                expected: 2,
            })
        )),
        "expected an InvalidDispatchDeclaration/Arity refusal, got {refusals:?}"
    );
    let location = refusals
        .iter()
        .find(|refusal| {
            matches!(
                &refusal.cause,
                CheckCause::InvalidDispatchDeclaration(InvalidDispatchDeclaration::Arity { .. })
            )
        })
        .map(|refusal| &refusal.location)
        .expect("an Arity refusal must be present");
    assert_eq!(
        location,
        &Location {
            path: vec![],
            origin: Origin::Body {
                function: "candidate.body".to_owned(),
                index: 0,
            },
        },
        "an Arity refusal must point at the candidate function's own declaration, not the expression root"
    );
}

/// FR-151: a candidate body whose non-receiver parameter type does not
/// match the dispatch operation's own declared argument type is refused
/// `invalid_package`/`invalid-value`, distinct from an arity mismatch, and
/// still located at the candidate function's own declaration.
#[trace("TC-196")]
#[test]
fn dispatch_candidate_with_a_mismatched_parameter_type_is_refused_invalid_dispatch() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let mut package = one_candidate_package(receiver_type, None, ValueType::Integer);
    package.dispatch_operations[0].parameters = vec![ValueType::Integer];
    package.functions[0] = FunctionDeclaration::clause(
        "candidate.body",
        vec![
            ("self".to_owned(), ValueType::Reference(receiver_type)),
            ("n".to_owned(), ValueType::Boolean),
        ],
        ValueType::Integer,
        None,
        Expression::Integer(Integer::from(1_i64)),
        DeclaredClauseKind::Body,
    );
    let refusals = package
        .check(CheckingLimits::default())
        .expect_err("a candidate/operation parameter type mismatch must be refused");
    let location = refusals
        .iter()
        .find_map(|refusal| match &refusal.cause {
            CheckCause::InvalidDispatchDeclaration(InvalidDispatchDeclaration::ParameterType {
                role: DispatchFunctionRole::Body,
                index: 0,
            }) => Some(&refusal.location),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!("expected an InvalidDispatchDeclaration/ParameterType refusal, got {refusals:?}")
        });
    assert_eq!(
        location,
        &Location {
            path: vec![],
            origin: Origin::Body {
                function: "candidate.body".to_owned(),
                index: 0,
            },
        },
        "a ParameterType refusal must point at the candidate function's own declaration, not the expression root"
    );
}

/// FR-151: a candidate body whose declared result type does not match the
/// dispatch operation's own declared result type is refused
/// `invalid_package`/`invalid-value`, distinct from an arity or parameter
/// mismatch, and still located at the candidate function's own declaration.
#[trace("TC-196")]
#[test]
fn dispatch_candidate_with_a_mismatched_result_type_is_refused_invalid_dispatch() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let mut package = one_candidate_package(receiver_type, None, ValueType::Integer);
    package.dispatch_operations[0].result = ValueType::Boolean;
    let refusals = package
        .check(CheckingLimits::default())
        .expect_err("a candidate/operation result type mismatch must be refused");
    let location = refusals
        .iter()
        .find_map(|refusal| match &refusal.cause {
            CheckCause::InvalidDispatchDeclaration(InvalidDispatchDeclaration::ResultType {
                role: DispatchFunctionRole::Body,
                index: 0,
            }) => Some(&refusal.location),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!("expected an InvalidDispatchDeclaration/ResultType refusal, got {refusals:?}")
        });
    assert_eq!(
        location,
        &Location {
            path: vec![],
            origin: Origin::Body {
                function: "candidate.body".to_owned(),
                index: 0,
            },
        },
        "a ResultType refusal must point at the candidate function's own declaration, not the expression root"
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
    let a = DeclarationKey::fixture("model.A.size");
    let b = DeclarationKey::fixture("model.B.size");
    let mut clauses = OperationClauses::default();
    // Both members get a `member` entry (not only the family's original
    // `a`): M5 (#204 round 1) roots a call through `B` at `b` itself, since
    // `B`'s own effective member for "size" is `B.size`, not `A.size`.
    clauses.member.insert(a.clone(), "size".to_owned());
    clauses.member.insert(b.clone(), "size".to_owned());
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
/// static receiver type is `receiver_type`. The root operation is
/// `receiver_type`'s own effective member for "size" -- `model.A.size` for
/// `model.A` itself, `model.B.size` for `model.B`, since `B` redefines it
/// (M5, #204 round 1: a redefiner is exposed only by its own root, never by
/// its ancestor's) — unchecked, so a caller expecting the check itself to be
/// refused (e.g. a D08 cycle) can inspect the refusals directly.
fn ab_bridge_declarations(
    receiver_type: NodeKey,
    pa: Option<Expression>,
    pb: Option<Expression>,
) -> PackageDeclarations {
    let domain_package = bridge_bundle();
    let view = bridge_view(&domain_package);
    let a_type = key("model.A");
    let b_type = key("model.B");
    let clauses = ab_bridge_clauses(a_type, pa, pb);
    let mut object_keys = BTreeMap::new();
    object_keys.insert(DeclarationKey::fixture("model.A"), a_type);
    object_keys.insert(DeclarationKey::fixture("model.B"), b_type);
    let mut meter =
        quire_spec_language::model::accounting::Meter::new(ModelNormalizationLimits::UNLIMITED);
    let root_key = if receiver_type == b_type {
        DeclarationKey::fixture("model.B.size")
    } else {
        DeclarationKey::fixture("model.A.size")
    };
    let root = DispatchRoot {
        key: root_key,
        closure: GeneralizationClosure::Closed,
    };
    let mut declarations = checked_dispatch_operation(
        &domain_package,
        &view,
        &root,
        &object_keys,
        &clauses,
        &mut meter,
    )
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
    declarations
}

/// [`ab_bridge_declarations`], checked and unwrapped, for callers that
/// expect the check to succeed.
fn ab_bridge_package(
    receiver_type: NodeKey,
    pa: Option<Expression>,
    pb: Option<Expression>,
) -> quire_spec_language::value::CheckedPackage {
    let graph = ab_bridge_declarations(receiver_type, pa, pb)
        .check(CheckingLimits::default())
        .unwrap();
    CheckedPackage::link(graph)
}

/// D06 (FR-151-AC-7/AC-8): a `B`-typed receiver, statically declared
/// `Reference<B>` at the call site, always selects `B.size` — the
/// most-specific redefining candidate — never `A.size`, the less-specific
/// method `B.size` redefines. `B` declares no own precondition at all, and
/// `A`'s own precondition is `false`: since the absent-precondition-is-true
/// rule makes `B`'s effective precondition `true`
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
        .graph()
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
    // Rooted at `model.B.size` itself (M5, #204 round 1: `B` redefines
    // `A.size`, so it is exposed only by its own root, never `A`'s), this
    // table has `B.size` as its one and only candidate: `dispatch.select`
    // sizes `value_occurrences` and its own work-unit charge by that one
    // candidate; `function.call` then charges one further work unit for
    // B's own body — A's body is never even built into this table's own
    // `functions`, let alone charged.
    assert_eq!(meter.consumed(LimitKind::ValueOccurrences), 1);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 2);
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
        .graph()
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

/// Two distinct dispatch operations (`size`, `weight`) that happen to share
/// one [`DispatchTable`] index: dispatching through `weight` must report
/// `PreconditionFailure.operation == "weight"`, not `"size"`, so the
/// reported operation always comes from this node's own `operation` index
/// (set once, at `check.rs`'s `dispatch_call`) rather than being re-derived
/// at evaluation time by searching `dispatch_operations` for whichever
/// operation happens to name the same `table` first.
#[trace("TC-196", "FR-151-AC-7")]
#[test]
fn d06_two_operations_sharing_one_table_report_the_operation_actually_dispatched() {
    let receiver_type = key("model.dispatch-calls.Receiver");
    let functions = vec![
        FunctionDeclaration::clause(
            "shared.body",
            vec![("self".to_owned(), ValueType::Reference(receiver_type))],
            ValueType::Integer,
            None,
            Expression::Integer(Integer::from(1_i64)),
            DeclaredClauseKind::Body,
        ),
        FunctionDeclaration::clause(
            "shared.precondition",
            vec![("self".to_owned(), ValueType::Reference(receiver_type))],
            ValueType::Boolean,
            None,
            Expression::Boolean(false),
            DeclaredClauseKind::Precondition,
        ),
    ];
    let table = DispatchTable::new(
        vec![(
            receiver_type,
            DispatchCandidate {
                body: 0,
                precondition: Some(1),
                precondition_clauses: vec![1],
            },
        )],
        1,
    );
    let package = PackageDeclarations {
        types: types(receiver_type),
        functions,
        dispatch_operations: vec![
            DispatchOperation {
                receiver_type,
                member: "size".to_owned(),
                parameters: Vec::new(),
                result: ValueType::Integer,
                table: 0,
            },
            DispatchOperation {
                receiver_type,
                member: "weight".to_owned(),
                parameters: Vec::new(),
                result: ValueType::Integer,
                table: 0,
            },
        ],
        dispatch_tables: vec![table],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .unwrap();

    let objects = objects(receiver_type, "r1");
    let parameters = vec![("self".to_owned(), ValueType::Reference(receiver_type))];
    let weight_call = Expression::Dispatch {
        receiver: Box::new(Expression::Name("self".to_owned())),
        member: "weight".to_owned(),
        arguments: Vec::new(),
    };
    let checked = package
        .check_clause_expression(
            parameters,
            &weight_call,
            None,
            ClauseKind::Invariant,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();
    let package = CheckedPackage::link(package);

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
        Outcome::Undefined(Undefined::PreconditionFalse(failure)) => {
            assert_eq!(
                *failure,
                PreconditionFailure {
                    operation: "weight".to_owned(),
                    selected: "shared.body".to_owned(),
                    receiver: receiver_reference(receiver_type, "r1"),
                }
            );
        }
        other => panic!("expected Undefined(PreconditionFalse), got {other:?}"),
    }
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

/// FR-151-AC-7: a `B`-typed receiver whose own precondition is `false`,
/// combined through the "own OR ancestor" combinator with `A`'s own
/// (ancestor) precondition also `false`, is refused
/// `Undefined::PreconditionFalse` naming `model.B.size` — never falling back
/// to the less-specific `model.A.size` the ancestor term belongs to.
#[trace("TC-196", "FR-151-AC-7")]
#[test]
fn d06_bridge_own_and_ancestor_precondition_both_false_selects_b_never_a() {
    let b_type = key("model.B");
    let package = ab_bridge_package(
        b_type,
        Some(Expression::Boolean(false)),
        Some(Expression::Boolean(false)),
    );

    let objects = objects(b_type, "b1");
    let parameters = vec![("self".to_owned(), ValueType::Reference(b_type))];
    let checked = package
        .graph()
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
        Outcome::Undefined(Undefined::PreconditionFalse(failure)) => {
            assert_eq!(
                *failure,
                PreconditionFailure {
                    operation: "size".to_owned(),
                    selected: "model.B.size".to_owned(),
                    receiver: receiver_reference(b_type, "b1"),
                }
            );
        }
        other => panic!("expected Undefined(PreconditionFalse), got {other:?}"),
    }
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 2);
}

/// FR-151-AC-7: the same `B`-typed receiver with `A`'s own precondition
/// `true` completes through the combinator — `B`'s own `false` clause does
/// not block the call once the ancestor term is satisfied. Both `B`'s own
/// clause and the combinator are present and genuinely evaluated (unlike
/// the absent-precondition case, which never builds a combinator at all),
/// so this charges one work unit more than that case: `dispatch.select`,
/// the combinator's own `function.call`, and the candidate body's
/// `function.call`.
#[trace("TC-196", "FR-151-AC-7")]
#[test]
fn d06_bridge_own_false_ancestor_true_completes_through_combinator() {
    let b_type = key("model.B");
    let package = ab_bridge_package(
        b_type,
        Some(Expression::Boolean(true)),
        Some(Expression::Boolean(false)),
    );

    let objects = objects(b_type, "b1");
    let parameters = vec![("self".to_owned(), ValueType::Reference(b_type))];
    let checked = package
        .graph()
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
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 3);
}

/// `self.size() >= 0`: a precondition that dispatches back into the same
/// operation it guards, for the D08 bridge tests below.
fn self_recursive_precondition() -> Expression {
    Expression::Binary {
        operator: BinaryOperator::GreaterOrEqual,
        left: Box::new(dispatch_expression()),
        right: Box::new(Expression::Integer(Integer::from(0_i64))),
    }
}

/// D08 (FR-151-AC-3), through the real [`checked_dispatch_operation`] bridge
/// rather than the hand-built single-candidate fixture
/// [`d08_a_cycle_through_a_dispatch_edge_is_refused_definition_cycle`] above:
/// `A`'s own precondition dispatches back into `A.size` itself
/// (`self.size() >= 0`); `B` declares no own precondition. The static
/// FR-146 ancestry the bridge builds gives exactly one cycle edge,
/// `model.A.size.precondition -> model.A.size.precondition`.
#[trace("TC-196", "FR-151-AC-3")]
#[test]
fn d08_bridge_self_recursive_precondition_is_refused_definition_cycle() {
    let a_type = key("model.A");
    let declarations = ab_bridge_declarations(a_type, Some(self_recursive_precondition()), None);
    let refusals = declarations
        .check(CheckingLimits::default())
        .expect_err("a dispatch-edge cycle through the bridge must be refused");
    let refusal = refusals
        .iter()
        .find(|refusal: &&CheckRefusal| matches!(refusal.cause, CheckCause::DefinitionCycle { .. }))
        .unwrap_or_else(|| panic!("expected a DefinitionCycle refusal, got {refusals:?}"));
    assert_eq!(
        refusal.cause,
        CheckCause::DefinitionCycle {
            edges: vec![(
                "model.A.size.precondition".to_owned(),
                "model.A.size.precondition".to_owned(),
            )],
        }
    );
}

/// D08, with `B` also declaring its own precondition (`false`): this forces
/// the "own OR ancestor" combinator (`model.B.size.precondition.effective`)
/// the bridge only builds for a genuine multi-term disjunction. The cycle
/// is still exactly `model.A.size.precondition -> model.A.size.precondition`
/// — `B`'s combinator dispatches into the same family but never back into
/// itself, so it never joins the cycle. This case is the one that
/// regresses if `DispatchTable::callees` ever goes back to reading
/// `DispatchCandidate::precondition` (the single runtime-absorbed index)
/// instead of `precondition_clauses` (the full static ancestry): `B`'s
/// runtime-absorbed precondition is the combinator's own index, not the
/// authored clauses it disjoins, so the case-1 test above (a single-term
/// candidate with no ancestor) cannot tell the two apart on its own.
#[trace("TC-196", "FR-151-AC-3")]
#[test]
fn d08_bridge_ancestor_cycle_survives_a_sibling_combinator() {
    let a_type = key("model.A");
    let declarations = ab_bridge_declarations(
        a_type,
        Some(self_recursive_precondition()),
        Some(Expression::Boolean(false)),
    );
    let refusals = declarations
        .check(CheckingLimits::default())
        .expect_err("a dispatch-edge cycle through the bridge must be refused");
    let refusal = refusals
        .iter()
        .find(|refusal: &&CheckRefusal| matches!(refusal.cause, CheckCause::DefinitionCycle { .. }))
        .unwrap_or_else(|| panic!("expected a DefinitionCycle refusal, got {refusals:?}"));
    assert_eq!(
        refusal.cause,
        CheckCause::DefinitionCycle {
            edges: vec![(
                "model.A.size.precondition".to_owned(),
                "model.A.size.precondition".to_owned(),
            )],
        }
    );
}

/// FR-151-AC-7: importing an ancestor's own precondition into a
/// descendant's own parameter names is capture-avoiding. `A`'s own
/// precondition, `let b = 5 in a = a`, uses a `let`-bound name (`b`) that
/// happens to collide with `B`'s own parameter name (`b`); renaming `A`'s
/// free `a` into `B`'s `b` while splicing this term into `B`'s combinator
/// must not let that renamed reference fall under the unrelated `let`
/// (which would either wrongly refuse `AmbiguousName { name: "b" }` or
/// silently change what the precondition means, by comparing the `let`'s
/// own `5` to itself instead of `B`'s own receiver parameter to itself).
#[trace("TC-196", "FR-151-AC-7")]
#[test]
fn d06_bridge_ancestor_let_binder_colliding_with_descendant_parameter_does_not_capture() {
    let a_type = key("model.A");
    let b_type = key("model.B");
    let a = DeclarationKey::fixture("model.A.size");
    let b = DeclarationKey::fixture("model.B.size");
    let mut clauses = OperationClauses::default();
    clauses.member.insert(a.clone(), "size".to_owned());
    clauses.member.insert(b.clone(), "size".to_owned());
    clauses.parameters.insert(
        a.clone(),
        vec![("a".to_owned(), ValueType::Reference(a_type))],
    );
    clauses.parameters.insert(
        b.clone(),
        vec![("b".to_owned(), ValueType::Reference(b_type))],
    );
    clauses.result.insert(a.clone(), ValueType::Integer);
    clauses.result.insert(b.clone(), ValueType::Integer);
    clauses
        .own_body
        .insert(a.clone(), Expression::Integer(Integer::from(1_i64)));
    clauses
        .own_body
        .insert(b.clone(), Expression::Integer(Integer::from(2_i64)));
    clauses.own_precondition.insert(
        a.clone(),
        Expression::Let {
            name: "b".to_owned(),
            value: Box::new(Expression::Integer(Integer::from(5_i64))),
            body: Box::new(Expression::Binary {
                operator: BinaryOperator::Equal,
                left: Box::new(Expression::Name("a".to_owned())),
                right: Box::new(Expression::Name("a".to_owned())),
            }),
        },
    );
    clauses
        .own_precondition
        .insert(b.clone(), Expression::Boolean(false));

    let domain_package = bridge_bundle();
    let view = bridge_view(&domain_package);
    let mut object_keys = BTreeMap::new();
    object_keys.insert(DeclarationKey::fixture("model.A"), a_type);
    object_keys.insert(DeclarationKey::fixture("model.B"), b_type);
    let mut meter =
        quire_spec_language::model::accounting::Meter::new(ModelNormalizationLimits::UNLIMITED);
    let root = DispatchRoot {
        // #204 round 1, M5: rooted at `b`, not the family's original `a` --
        // `B`'s own effective member for "size" is `B.size`, and this test
        // calls through a `B`-typed receiver, so only `B`'s own root exposes
        // `DispatchOperation { receiver_type: b_type, .. }`.
        key: b.clone(),
        closure: GeneralizationClosure::Closed,
    };
    let mut declarations = checked_dispatch_operation(
        &domain_package,
        &view,
        &root,
        &object_keys,
        &clauses,
        &mut meter,
    )
    .unwrap_or_else(|refusal| {
        panic!("expected a linked, checked dispatch family, got {refusal:?}")
    });
    declarations.types = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(a_type, "A", vec![]),
            ObjectTypeDeclaration::new(b_type, "B", vec![]),
        ],
    )
    .unwrap();

    let package = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| {
            panic!("expected the capture-avoiding rename to check cleanly, got {refusals:?}")
        });

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
    let package = CheckedPackage::link(package);

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
        other => panic!(
            "expected a completed Integer(2) (B's own body) — A's ancestor term reduces to \
             the receiver compared to itself, always true once capture-avoided — got {other:?}"
        ),
    }
}

// --- Bridge integration: crate::check::checked_dispatch -------------------

fn object_type_record(identity: &str, supertypes: Vec<&str>) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: DeclarationKey::fixture(identity),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes
            .into_iter()
            .map(DeclarationKey::fixture)
            .collect(),
    })
}

/// #174: every fixture operation in this file is dispatched (checked
/// through `checked_dispatch_operation`), so its model-layer record must
/// itself be a query -- FR-151's own restriction, not merely a checker-layer
/// convention -- a declared result and the default (empty) effect. The
/// result's own value type is never consulted by `link_dispatch`/
/// `checked_dispatch_operation` (only `OperationClauses`'s pre-translated
/// checker `ValueType` is); `normalize`'s own phase-1 reference validation
/// does require it to name a real declared type, so this reuses the
/// operation's own `owner` key, always declared by every fixture bundle in
/// this file.
fn operation_record(
    identity: &str,
    owner: &str,
    has_body: bool,
    redefines: Option<&str>,
) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        parameters: Vec::new(),
        result: Some(OperationResult {
            value_type: ValueTypeRef::Package(DeclarationKey::fixture(owner)),
            multiplicity: Multiplicity {
                lower: 1,
                upper: Some(1),
                ordered: false,
                unique: true,
            },
        }),
        effect: OperationEffect::default(),
        has_own_precondition: false,
        own_postcondition_clauses: Vec::new(),
        has_body,
        redefines: redefines.map(DeclarationKey::fixture),
    })
}

/// `model.A`/`model.B` (`B` <= `A`), each declaring its own `size` body;
/// `B.size` redefines `A.size`. Mirrors `tests/model_dispatch.rs`'s D01
/// fixture style, minus the diamond (only one redefinition edge is needed
/// to prove the bridge links a real family end to end).
fn bridge_bundle() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.dispatch-calls"),
        vec![
            object_type_record("model.A", vec![]),
            object_type_record("model.B", vec!["model.A"]),
            operation_record("model.A.size", "model.A", true, None),
            operation_record("model.B.size", "model.B", true, Some("model.A.size")),
        ],
    )
}

fn bridge_view(domain_package: &DomainPackage) -> EffectiveView {
    match normalize(domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed effective view, got {other:?}"),
    }
}

fn bridge_clauses(receiver_type: NodeKey) -> OperationClauses {
    let a = DeclarationKey::fixture("model.A.size");
    let b = DeclarationKey::fixture("model.B.size");
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

/// End-to-end: `crate::check::checked_dispatch::checked_dispatch_operation`
/// links `model.A.size`'s real two-candidate dispatch family (`model.A.size`
/// and `model.B.size`, which redefines it) from a `DomainPackage`, types both
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
    let domain_package = bridge_bundle();
    let view = bridge_view(&domain_package);
    let a_type = key("model.A");
    let b_type = key("model.B");
    let clauses = bridge_clauses(a_type);
    let mut object_keys = BTreeMap::new();
    object_keys.insert(DeclarationKey::fixture("model.A"), a_type);
    object_keys.insert(DeclarationKey::fixture("model.B"), b_type);
    let mut meter =
        quire_spec_language::model::accounting::Meter::new(ModelNormalizationLimits::UNLIMITED);

    let root = DispatchRoot {
        key: DeclarationKey::fixture("model.A.size"),
        closure: GeneralizationClosure::Closed,
    };
    let mut declarations = checked_dispatch_operation(
        &domain_package,
        &view,
        &root,
        &object_keys,
        &clauses,
        &mut meter,
    )
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
    let package = CheckedPackage::link(package);

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

/// `model.A` (declares `size`), `model.C` (`<- model.A`, no operation record
/// of its own at all -- inherits `size` unredefined).
fn inherited_only_bridge_bundle() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.dispatch-calls-inherited"),
        vec![
            object_type_record("model.A", vec![]),
            object_type_record("model.C", vec!["model.A"]),
            operation_record("model.A.size", "model.A", true, None),
        ],
    )
}

fn inherited_only_clauses(receiver_type: NodeKey) -> OperationClauses {
    let a = DeclarationKey::fixture("model.A.size");
    let mut clauses = OperationClauses::default();
    clauses.member.insert(a.clone(), "size".to_owned());
    clauses.parameters.insert(
        a.clone(),
        vec![("self".to_owned(), ValueType::Reference(receiver_type))],
    );
    clauses.result.insert(a.clone(), ValueType::Integer);
    clauses
        .own_body
        .insert(a, Expression::Integer(Integer::from(1_i64)));
    clauses
}

/// #176: one `checked_dispatch_operation` call for `A.size`, rooted at `A`
/// (`root.receiver_type = a_type`), exposes the dispatch through both `A`
/// (the operation's own declared owner) and `C` (a conforming subtype that
/// only inherits `size`, never redefines it) -- `link_dispatch`'s own table
/// already links `C` to the identical candidate `A.size` (its own
/// conformance-only linking never depended on redefinition), but before
/// #176 only `root.receiver_type` itself ever reached
/// `PackageDeclarations::dispatch_operations`, so a call whose receiver's
/// declared static type was `C` refused `ineligible` even though `C`
/// exposes `size` unambiguously. FR-151 (`quire.model.dispatch.single/v1`):
/// "member-name resolves statically to exactly one exposed effective
/// operation... of the receiver's static type `T`" -- `T` is not required to
/// be the operation's own declared owner.
#[trace("TC-196", "FR-151")]
#[test]
fn bridge_exposes_dispatch_through_an_inherited_static_type_that_never_redefines() {
    let domain_package = inherited_only_bridge_bundle();
    let view = bridge_view(&domain_package);
    let a_type = key("model.A");
    let c_type = key("model.C");
    let clauses = inherited_only_clauses(a_type);
    let mut object_keys = BTreeMap::new();
    object_keys.insert(DeclarationKey::fixture("model.A"), a_type);
    object_keys.insert(DeclarationKey::fixture("model.C"), c_type);
    let mut meter =
        quire_spec_language::model::accounting::Meter::new(ModelNormalizationLimits::UNLIMITED);

    let root = DispatchRoot {
        key: DeclarationKey::fixture("model.A.size"),
        closure: GeneralizationClosure::Closed,
    };
    let mut declarations = checked_dispatch_operation(
        &domain_package,
        &view,
        &root,
        &object_keys,
        &clauses,
        &mut meter,
    )
    .unwrap_or_else(|refusal| {
        panic!("expected a linked, checked dispatch family, got {refusal:?}")
    });

    let mut receiver_types: Vec<NodeKey> = declarations
        .dispatch_operations
        .iter()
        .map(|operation| operation.receiver_type)
        .collect();
    receiver_types.sort();
    let mut expected = vec![a_type, c_type];
    expected.sort();
    assert_eq!(
        receiver_types, expected,
        "the bridge must expose size through both A (its own declared \
         owner) and C (a conforming subtype that only inherits it)"
    );
    assert!(
        declarations
            .dispatch_operations
            .iter()
            .all(|operation| operation.table == 0),
        "every exposed static type shares the identical linked table"
    );

    let types = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(a_type, "A", vec![]),
            ObjectTypeDeclaration::new(c_type, "C", vec![]),
        ],
    )
    .unwrap();
    declarations.types = types;
    let package = declarations.check(CheckingLimits::default()).unwrap();

    // The real proof: a clause whose own `self` parameter is declared
    // `Reference<C>` -- not `A`, the operation's own declared owner --
    // still type-checks `self.size()` as a dispatch call.
    let parameters = vec![("self".to_owned(), ValueType::Reference(c_type))];
    package
        .check_clause_expression(
            parameters,
            &dispatch_expression(),
            None,
            ClauseKind::Invariant,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap_or_else(|refusal| {
            panic!("a C-typed receiver must dispatch through the inherited A.size: {refusal:?}")
        });
}

/// One declared object type `A` and one operation `A.size`, varied per test
/// below by `result`/`effect` -- the minimal shape needed to reach #174's
/// query-only check without any redefinition family to also link.
fn not_a_query_bundle(result: Option<OperationResult>, effect: OperationEffect) -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.dispatch-calls-not-a-query"),
        vec![
            object_type_record("model.A", vec![]),
            DomainPackageRecord::OperationMember(OperationMemberRecord {
                key: DeclarationKey::fixture("model.A.size"),
                owner: DeclarationKey::fixture("model.A"),
                parameters: Vec::new(),
                result,
                effect,
                has_own_precondition: false,
                own_postcondition_clauses: Vec::new(),
                has_body: true,
                redefines: None,
            }),
        ],
    )
}

/// Runs `checked_dispatch_operation` against `not_a_query_bundle`'s own
/// `model.A.size` and returns the refusal it must produce.
fn not_a_query_refusal(domain_package: &DomainPackage) -> DispatchBridgeRefusal {
    let view = bridge_view(domain_package);
    let a_type = key("model.A");
    let clauses = inherited_only_clauses(a_type);
    let mut object_keys = BTreeMap::new();
    object_keys.insert(DeclarationKey::fixture("model.A"), a_type);
    let mut meter =
        quire_spec_language::model::accounting::Meter::new(ModelNormalizationLimits::UNLIMITED);
    let root = DispatchRoot {
        key: DeclarationKey::fixture("model.A.size"),
        closure: GeneralizationClosure::Closed,
    };
    checked_dispatch_operation(
        domain_package,
        &view,
        &root,
        &object_keys,
        &clauses,
        &mut meter,
    )
    .expect_err("a non-query dispatch target must refuse, not link")
}

/// #174 (FR-151, `quire.model.dispatch.single/v1`): "Only query operations,
/// whose result is present and whose effect set is empty, may be called."
/// `model.A.size` declares no result at all -- refused `NotAQuery`, not
/// silently admitted as a dispatch target.
#[trace("TC-196", "FR-151")]
#[test]
fn checked_dispatch_operation_refuses_a_dispatch_target_with_no_result() {
    let domain_package = not_a_query_bundle(None, OperationEffect::default());
    match not_a_query_refusal(&domain_package) {
        DispatchBridgeRefusal::NotAQuery(refusal) => {
            assert_eq!(refusal.code, Code::IllTyped);
            assert_eq!(refusal.cause, ModelRefusalCause::OperatorIneligible);
            assert!(
                refusal.detail.contains("model.A.size"),
                "detail must name the offending operation: {}",
                refusal.detail
            );
        }
        other => panic!("expected NotAQuery, got {other:?}"),
    }
}

/// #174: `model.A.size` declares a result but a non-empty effect (it
/// `creates` `model.A`) -- refused `NotAQuery` for the identical reason: FR-151
/// admits only an operation whose effect set is empty as a dispatch target.
#[trace("TC-196", "FR-151")]
#[test]
fn checked_dispatch_operation_refuses_a_dispatch_target_with_a_non_empty_effect() {
    let domain_package = not_a_query_bundle(
        Some(OperationResult {
            value_type: ValueTypeRef::Package(DeclarationKey::fixture("model.A")),
            multiplicity: Multiplicity {
                lower: 1,
                upper: Some(1),
                ordered: false,
                unique: true,
            },
        }),
        OperationEffect {
            modifies: Vec::new(),
            creates: vec![DeclarationKey::fixture("model.A")],
            deletes: Vec::new(),
        },
    );
    match not_a_query_refusal(&domain_package) {
        DispatchBridgeRefusal::NotAQuery(refusal) => {
            assert_eq!(refusal.code, Code::IllTyped);
            assert_eq!(refusal.cause, ModelRefusalCause::OperatorIneligible);
            assert!(
                refusal.detail.contains("model.A.size"),
                "detail must name the offending operation: {}",
                refusal.detail
            );
        }
        other => panic!("expected NotAQuery, got {other:?}"),
    }
}

/// #174 (FR-151, `quire.model.dispatch.single/v1`), H3 (#204 round 1): the
/// query-only check runs against `root.key` first, then every other
/// distinct candidate in plain `DeclarationKey` order -- never
/// `domain_package.records`' own insertion order. `model.B.size` (which
/// redefines `model.A.size`) is declared *before* `model.A.size` in this
/// bundle's own `records`, and is itself also not a query; if the check
/// still walked `ordered_candidates`' record-order sort (the pre-#204-
/// round-1 shape), it would report `model.B.size` first. It must report
/// `model.A.size` -- `root.key`, the statically resolved operation this
/// whole call is about -- regardless.
#[trace("TC-196", "FR-151")]
#[test]
fn checked_dispatch_operation_checks_root_key_first_not_record_order() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.dispatch-calls-h3-root-first"),
        vec![
            object_type_record("model.A", vec![]),
            object_type_record("model.B", vec!["model.A"]),
            DomainPackageRecord::OperationMember(OperationMemberRecord {
                key: DeclarationKey::fixture("model.B.size"),
                owner: DeclarationKey::fixture("model.B"),
                parameters: Vec::new(),
                result: None,
                effect: OperationEffect::default(),
                has_own_precondition: false,
                own_postcondition_clauses: Vec::new(),
                has_body: true,
                redefines: Some(DeclarationKey::fixture("model.A.size")),
            }),
            DomainPackageRecord::OperationMember(OperationMemberRecord {
                key: DeclarationKey::fixture("model.A.size"),
                owner: DeclarationKey::fixture("model.A"),
                parameters: Vec::new(),
                result: None,
                effect: OperationEffect::default(),
                has_own_precondition: false,
                own_postcondition_clauses: Vec::new(),
                has_body: true,
                redefines: None,
            }),
        ],
    );
    let view = bridge_view(&domain_package);
    let a_type = key("model.A");
    let b_type = key("model.B");
    let mut clauses = OperationClauses::default();
    let a = DeclarationKey::fixture("model.A.size");
    let b = DeclarationKey::fixture("model.B.size");
    for (operation, receiver) in [(&a, a_type), (&b, b_type)] {
        clauses.member.insert(operation.clone(), "size".to_owned());
        clauses.parameters.insert(
            operation.clone(),
            vec![("self".to_owned(), ValueType::Reference(receiver))],
        );
        clauses
            .own_body
            .insert(operation.clone(), Expression::Integer(Integer::from(1_i64)));
    }
    let mut object_keys = BTreeMap::new();
    object_keys.insert(DeclarationKey::fixture("model.A"), a_type);
    object_keys.insert(DeclarationKey::fixture("model.B"), b_type);
    let mut meter =
        quire_spec_language::model::accounting::Meter::new(ModelNormalizationLimits::UNLIMITED);
    let root = DispatchRoot {
        key: a.clone(),
        closure: GeneralizationClosure::Closed,
    };
    let refusal = checked_dispatch_operation(
        &domain_package,
        &view,
        &root,
        &object_keys,
        &clauses,
        &mut meter,
    )
    .expect_err("neither candidate declares a result: the family must refuse, not link");
    match refusal {
        DispatchBridgeRefusal::NotAQuery(refusal) => {
            assert!(
                refusal.detail.contains("model.A.size"),
                "root.key (model.A.size) must be the one reported, not model.B.size \
                 (declared first in domain_package.records): {}",
                refusal.detail
            );
        }
        other => panic!("expected NotAQuery, got {other:?}"),
    }
}
