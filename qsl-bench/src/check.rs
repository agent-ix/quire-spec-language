// SPDX-License-Identifier: AGPL-3.0-or-later
//! S3 checker and S6a evaluator inputs: `PackageDeclarations` built from
//! S2 forms, the same way the checker's own integration tests build them
//! (no S1 source reaches `PackageDeclarations` yet: the value-function
//! family has no CST-to-form production).

use qsl_eval::value::{CheckedPackageEvaluation, Evaluation, QualifiedName};
use qsl_forms::{BinaryOperator, BuiltinType, Expression, FunctionDeclaration, TypeForm};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckRefusal, CheckedGraph, CheckingLimits, PackageDeclarations};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Integer, Meter, Outcome, ScalarLimits, Value};

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
        ..PackageDeclarations::default()
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
        ..PackageDeclarations::default()
    }
}

/// Check `declarations` at the default (unlimited-node, maximum-depth)
/// checking limits.
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
        let package = linked_chain(2);
        assert!(completed_with_five(&call_head(
            &package,
            &ObjectEnvironment::default()
        )));
    }
}
