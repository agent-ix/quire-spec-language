// SPDX-License-Identifier: AGPL-3.0-or-later
//! The identity precondition for arbitrary nesting depth (ADR-030 D-4.4):
//! the emitted v2 package's identity preimage, and every node it projects,
//! has a depth fixed by its schema, never one that follows the source's
//! expression depth. A deep body emits as many nodes, each naming its
//! operands by reference, so its wire is no deeper than a shallow body's.

use super::*;
use qsl_semantics::check::json_depth;

/// One level of a nested body: the body at `level` around `body`.
type Step = fn(usize, Expression) -> Expression;

/// `d(a)` whose body nests `step` around `a`, `levels` times.
fn deep(levels: usize, step: Step) -> FunctionDeclaration {
    function(
        "d",
        &["a"],
        (0..levels).fold(name("a"), |body, level| step(level, body)),
    )
}

/// The deepest JSON nesting of the v2 wire emitted for `function`, which
/// holds the package's identity preimage and every node it projects.
fn wire_depth(function: FunctionDeclaration) -> usize {
    json_depth(emit(&package(vec![function])).package.bytes())
}

/// A 100-level `and` chain, `else if` chain and `let` chain each emit a
/// wire no deeper than the same form at 4 levels.
#[trace("TC-725", "FR-258-AC-5")]
#[test]
fn a_deep_bodys_package_identity_preimage_has_a_fixed_depth() {
    let and = |_, body| Expression::Binary {
        operator: BinaryOperator::And,
        left: Box::new(name("a")),
        right: Box::new(body),
    };
    let else_if = |_, body| Expression::If {
        condition: Box::new(name("a")),
        then: Box::new(name("a")),
        otherwise: Box::new(body),
    };
    let nested_let = |level, body| Expression::Let {
        name: format!("b{level}"),
        value: Box::new(name("a")),
        body: Box::new(body),
    };
    let steps: [(&str, Step); 3] = [("and", and), ("else if", else_if), ("let", nested_let)];
    for (label, step) in steps {
        assert_eq!(
            wire_depth(deep(4, step)),
            wire_depth(deep(100, step)),
            "{label}: the emitted wire's depth follows the source depth"
        );
    }
}
