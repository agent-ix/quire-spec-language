// SPDX-License-Identifier: AGPL-3.0-or-later
//! S3 checker and S6a evaluator inputs: `PackageDeclarations` built from
//! S2 forms, the same way the checker's own integration tests build them
//! (no S1 source reaches `PackageDeclarations` yet: the value-function
//! family has no CST-to-form production).

use qsl_eval::value::{CheckedPackageEvaluation, Evaluation, QualifiedName};
use qsl_forms::{BinaryOperator, BuiltinType, Expression, FunctionDeclaration, TypeForm};
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    CheckRefusal, CheckedGraph, CheckingLimits, EnumBinding, PackageDeclarations,
};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::declaration::{
    FieldDeclaration, InvalidDeclaration, ObjectTypeDeclaration, TypeEnvironment,
    TypeEnvironmentLimits,
};
use qsl_semantics::value::enumeration::{
    EnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage,
};
use qsl_semantics::value::{NodeIdentityPreimage, NodeOwner, OwnerSelection, OwnerSubject};
use quire_exact::{EffectiveId, Presence, ValueType};
use quire_exact::{Integer, Meter, Outcome, ScalarLimits, Value};
use quire_exact::{NodeKey, NODE_KEY_DOMAIN};
use sha2::{Digest, Sha256};

/// A kernel limit set no evaluation in this crate runs out of.
pub const SCALAR_UNLIMITED: ScalarLimits = ScalarLimits {
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

/// The source reference every benchmark package is declared under: the
/// empty unit admitted as (`agent-ix`, `qsl-bench`, `bench`, `1`). Its
/// authority and identity are the declared nodes' owner (FR-001, FR-092).
pub(crate) fn source() -> RawSourceRef {
    qsl_foundation::Source::read(
        qsl_foundation::SourceIdentity::new("agent-ix", "qsl-bench", "bench", "1"),
        "qsl-bench",
        b"",
        0,
    )
    .expect("a named empty unit is admitted")
    .reference()
    .clone()
}

fn integer() -> TypeForm {
    TypeForm::builtin(
        BuiltinType::Integer,
        qsl_foundation::Span { start: 0, end: 0 },
    )
}

fn function(name: String, body: Expression) -> FunctionDeclaration {
    FunctionDeclaration::new(
        name,
        vec![("x".to_owned(), integer())],
        integer(),
        None,
        body,
    )
}

/// The name of the chain's `index`-th function.
pub fn chain_name(index: usize) -> String {
    format!("f{index}")
}

/// An N-function call chain: `f0(x) = f1(x)`, ..., `f{N-2}(x) = f{N-1}(x)`,
/// `f{N-1}(x) = x`. One call per function, no recursion, so the
/// termination check finds N singleton components. Calling `f0` pushes N
/// evaluator call frames.
pub fn call_chain(functions: usize) -> PackageDeclarations {
    let declarations = (0..functions)
        .map(|index| {
            let body = if index + 1 < functions {
                Expression::Call {
                    name: chain_name(index + 1),
                    arguments: vec![Expression::Name("x".to_owned())],
                }
            } else {
                Expression::Name("x".to_owned())
            };
            function(chain_name(index), body)
        })
        .collect();
    PackageDeclarations {
        functions: declarations,
        ..PackageDeclarations::new(source())
    }
}

/// N independent functions, `fI(x) = x + 1`, with no call between them:
/// the same declaration count as [`call_chain`], with no call edge for the
/// termination check or for a call-site signature lookup to walk.
pub fn independent(functions: usize) -> PackageDeclarations {
    let declarations = (0..functions)
        .map(|index| {
            function(
                chain_name(index),
                Expression::Binary {
                    operator: BinaryOperator::Add,
                    left: Box::new(Expression::Name("x".to_owned())),
                    right: Box::new(Expression::Integer(Integer::from(1_i64))),
                },
            )
        })
        .collect();
    PackageDeclarations {
        functions: declarations,
        ..PackageDeclarations::new(source())
    }
}

/// The name of the `index`-th case of [`enum_members`]'s enum.
pub fn case_name(index: usize) -> String {
    format!("c{index}")
}

/// One admitted ordered enum `E` with `members` cases `c0`, `c1`, ...
///
/// # Panics
///
/// Panics when the generated declaration or a member does not admit: a
/// harness defect, since both are built from their own preimages' keys.
pub fn enum_binding(members: usize) -> EnumBinding {
    let owners = OwnerSelection::new([NodeOwner::Definition(OwnerSubject {
        authority: "agent-ix".into(),
        identity: "qsl-bench".into(),
    })]);
    let cases: Vec<String> = (0..members).map(case_name).collect();
    let preimage = EnumDeclarationPreimage::from_json(serde_json::json!({
        "version": "quire.enum-declaration-node/v1",
        "owner": {"kind": "definition", "authority": "agent-ix", "identity": "qsl-bench"},
        "qualified_declaration": ["Bench", "E"],
        "ordered": true,
        "members": cases,
    }))
    .expect("a well-formed enum declaration preimage");
    let key = NodeKey::from_digest(preimage.digest().expect("the preimage encodes"));
    let declaration =
        EnumDeclaration::admit(preimage, key, &owners).expect("the declaration admits");
    let members = cases
        .iter()
        .map(|case| {
            let member = EnumMemberPreimage::from_json(serde_json::json!({
                "version": "quire.enum-member-node/v1",
                "declaration_node_id": {
                    "domain": NODE_KEY_DOMAIN,
                    "digest": declaration.key().to_string(),
                },
                "case": case,
            }))
            .expect("a well-formed enum member preimage");
            let key = NodeKey::from_digest(member.digest().expect("the preimage encodes"));
            declaration
                .admit_member(&member, key)
                .expect("the member admits")
        })
        .collect();
    EnumBinding {
        name: "E".to_owned(),
        declaration,
        members,
    }
}

/// N independent functions in a package declaring `binding`, each comparing
/// two of its members: `fI(x) = E::c{I mod M} == E::c0`. Every function
/// resolves an enum member by name and checks an enum equality, so a
/// per-function cost that grows with the enum's member count shows here.
pub fn enum_members(functions: usize, binding: &EnumBinding) -> PackageDeclarations {
    let members = binding.members.len().max(1);
    let boolean = TypeForm::builtin(
        BuiltinType::Boolean,
        qsl_foundation::Span { start: 0, end: 0 },
    );
    let declarations = (0..functions)
        .map(|index| {
            FunctionDeclaration::new(
                chain_name(index),
                vec![("x".to_owned(), integer())],
                boolean.clone(),
                None,
                Expression::Binary {
                    operator: BinaryOperator::Equal,
                    left: Box::new(Expression::Name(format!(
                        "E::{}",
                        case_name(index % members)
                    ))),
                    right: Box::new(Expression::Name(format!("E::{}", case_name(0)))),
                },
            )
        })
        .collect();
    PackageDeclarations {
        functions: declarations,
        enums: vec![binding.clone()],
        ..PackageDeclarations::new(source())
    }
}

/// Check `declarations` at the default checking limits (NFR-011).
pub fn check(declarations: PackageDeclarations) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    declarations.check(CheckingLimits::default())
}

/// Check and S4-link an N-function call chain for the evaluator benchmark.
///
/// # Panics
///
/// Panics when the chain does not check: the evaluator benchmark has
/// nothing to time without a checked package.
pub fn linked_chain(functions: usize) -> CheckedPackage {
    CheckedPackage::link(
        check(call_chain(functions)).expect("an acyclic Integer call chain checks"),
    )
}

/// Call `f0(5)` on a linked [`call_chain`] with an unlimited kernel meter.
///
/// # Panics
///
/// Panics when `f0` is not a callable name -- `QualifiedName` refusing
/// `"f0"` or the package not resolving it is a harness defect.
pub fn call_head(package: &CheckedPackage, objects: &ObjectEnvironment) -> Evaluation {
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    package
        .call(
            &QualifiedName::unqualified(chain_name(0)).expect("f0 is identifier-shaped"),
            vec![Value::Integer(Integer::from(5_i64))],
            objects,
            &mut meter,
        )
        .expect("f0 resolves and its Integer argument validates")
}

/// Whether `evaluation` completed with the chain's expected result, `5`.
pub fn completed_with_five(evaluation: &Evaluation) -> bool {
    matches!(
        &evaluation.outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value)))
            if *value == Integer::from(5_i64)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smallest_packages_check_and_evaluate() {
        assert!(check(call_chain(1)).is_ok());
        assert!(check(independent(1)).is_ok());
        assert!(check(enum_members(2, &enum_binding(2))).is_ok());
        let package = linked_chain(2);
        assert!(completed_with_five(&call_head(
            &package,
            &ObjectEnvironment::default()
        )));
    }
}

/// A linear chain `M::T0 <- M::T1 <- ... <- M::T{depth-1}` of model object
/// types, each declaring one integer field of its own (QSL-57's admission
/// cost shape: type `k` flattens to `k + 1` slots).
pub fn object_chain(depth: usize) -> Vec<ObjectTypeDeclaration> {
    let key = |level: usize| {
        EffectiveId::from_digest(Sha256::digest(format!("M::T{level}").as_bytes()).into())
    };
    (0..depth)
        .map(|level| {
            let supertypes = level.checked_sub(1).map(key).into_iter().collect();
            ObjectTypeDeclaration::new(
                key(level),
                format!("M::T{level}"),
                vec![FieldDeclaration::new(
                    format!("f{level}"),
                    ValueType::Integer,
                    Presence::Required,
                )],
            )
            .with_supertypes(supertypes)
        })
        .collect()
}

/// Admit `object_types` as one check-time `TypeEnvironment` under the
/// default `ancestor_steps` and `work_units` budget.
pub fn admit_object_types(
    object_types: Vec<ObjectTypeDeclaration>,
    work_units: u64,
) -> Result<TypeEnvironment, InvalidDeclaration> {
    TypeEnvironment::bounded(
        [],
        object_types,
        TypeEnvironmentLimits {
            work_units,
            ..TypeEnvironmentLimits::default()
        },
    )
}
