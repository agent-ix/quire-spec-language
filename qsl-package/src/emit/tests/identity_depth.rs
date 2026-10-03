// SPDX-License-Identifier: AGPL-3.0-or-later
//! The identity precondition for arbitrary nesting depth (ADR-030 D-4.4):
//! the emitted v2 package's identity preimage, and every node it projects,
//! has a depth fixed by its schema, never one that follows the source's
//! expression depth. A deep body emits as many nodes, each naming its
//! operands by reference, so its identity preimage is no deeper than a
//! shallow body's.

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

/// The deepest JSON nesting of the `identity_preimage` member of the v2 wire
/// emitted for `function`, which projects every node of the package.
fn identity_preimage_depth(function: FunctionDeclaration) -> usize {
    let preimage = &wire(&emit(&package(vec![function])))["identity_preimage"];
    assert!(
        preimage.is_object(),
        "the wire carries an identity preimage"
    );
    json_depth(&serde_json::to_vec(preimage).expect("a JSON value encodes"))
}

/// A 100-level `and` chain, `else if` chain and `let` chain each emit an
/// identity preimage no deeper than the same form at 4 levels.
#[trace("TC-728", "FR-259-AC-4")]
#[test]
fn a_deep_bodys_package_identity_preimage_has_a_fixed_depth() {
    let and = |_, body| Expression::binary(BinaryOperator::And, name("a"), body);
    let else_if = |_, body| Expression::if_then_else(name("a"), name("a"), body);
    let nested_let = |level, body| Expression::let_in(format!("b{level}"), name("a"), body);
    let steps: [(&str, Step); 3] = [("and", and), ("else if", else_if), ("let", nested_let)];
    for (label, step) in steps {
        assert_eq!(
            identity_preimage_depth(deep(4, step)),
            identity_preimage_depth(deep(100, step)),
            "{label}: the identity preimage's depth follows the source depth"
        );
    }
}
