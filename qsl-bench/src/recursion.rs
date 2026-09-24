// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-203: `PackageDeclarations` with N recursive components, for the
//! termination check's per-component cost. Kept apart from [`crate::check`],
//! whose generators are MP-002's protected apparatus.

use qsl_forms::{BuiltinType, Expression, FunctionDeclaration, TypeForm};
use qsl_semantics::check::PackageDeclarations;

use crate::check::chain_name;

/// N self-recursive functions, `fI(x) = fI(x)`, with no `decreases`
/// measure: N singleton recursive components, each refused
/// `unproved-decrease` / `missing-measure`, so the check reports N
/// refusals and builds N refusal cycles.
pub fn self_recursive(functions: usize) -> PackageDeclarations {
    let integer = || {
        TypeForm::builtin(
            BuiltinType::Integer,
            qsl_foundation::Span { start: 0, end: 0 },
        )
    };
    let declarations = (0..functions)
        .map(|index| {
            FunctionDeclaration::new(
                chain_name(index),
                vec![("x".to_owned(), integer())],
                integer(),
                None,
                Expression::Call {
                    name: chain_name(index),
                    arguments: vec![Expression::Name("x".to_owned())],
                },
            )
        })
        .collect();
    PackageDeclarations {
        functions: declarations,
        ..PackageDeclarations::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::check;

    #[test]
    fn each_self_recursive_function_is_refused_once() {
        let refusals = check(self_recursive(3)).expect_err("no function has a measure");
        assert_eq!(refusals.len(), 3);
        assert!(refusals
            .iter()
            .all(|refusal| refusal.cause.cause() == Some("unproved-decrease")));
    }
}
