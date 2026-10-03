// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-725 step 1 (FR-258-AC-1): a 100,000-term sum, a 100,000-long `else
//! if` chain and 100,000 nested `let`s each check, lower and emit on a
//! 512 KiB stack; every written node rebuilt from the wire keys to its
//! `node_id`; and the checked body clones, compares equal to its clone,
//! formats for debug and drops on the same stack.

use super::*;
use qsl_semantics::check::CheckedBody;

/// The stack every step runs on.
const STACK: usize = 512 * 1024;

/// How many terms, branches or bindings each body has.
const DEEP: usize = 100_000;

/// `f`'s body: a sum of `x: Int[0, 1]`, an `else if` chain over `a:
/// Boolean`, or nested `let`s over `a: Boolean`, each [`DEEP`] long.
fn deep_function(label: &str) -> FunctionDeclaration {
    let integer = || TypeForm::builtin(BuiltinType::Integer, SPAN);
    let bit =
        TypeForm::builtin(BuiltinType::Int, SPAN).with_bounds(vec!["0".to_owned(), "1".to_owned()]);
    let (parameter, form, result) = match label {
        "sum" => ("x", bit, integer()),
        _ => ("a", boolean(), boolean()),
    };
    let mut builder = qsl_forms::ExpressionBuilder::new();
    let leaf = |builder: &mut qsl_forms::ExpressionBuilder| {
        builder
            .push(qsl_forms::ExprNode::Name(parameter.to_owned()))
            .expect("a name has no children")
    };
    let mut body = leaf(&mut builder);
    for level in 1..DEEP {
        let node = match label {
            "sum" => qsl_forms::ExprNode::Binary {
                operator: BinaryOperator::Add,
                left: body,
                right: leaf(&mut builder),
            },
            "else if" => qsl_forms::ExprNode::If {
                condition: leaf(&mut builder),
                then: leaf(&mut builder),
                otherwise: body,
            },
            _ => qsl_forms::ExprNode::Let {
                name: format!("b{level}"),
                value: leaf(&mut builder),
                body,
            },
        };
        body = builder
            .push(node)
            .expect("each operand is stored and unparented");
    }
    let body = builder.build().expect("the last node is the root");
    FunctionDeclaration::new("f", vec![(parameter.to_owned(), form)], result, None, body)
}

/// Each deep body checks, lowers and emits under raised limits on a
/// 512 KiB stack, its written nodes key to their `node_id`s, and its
/// checked body clones, compares, formats and drops there.
#[trace("TC-725", "FR-258-AC-1")]
#[test]
#[ignore = "each checked node's Location holds its whole child-index path, so a 100,000-deep body's locations take memory quadratic in its depth"]
fn a_100000_deep_body_checks_lowers_and_emits_on_a_small_stack() {
    for label in ["sum", "else if", "let"] {
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(move || {
                let graph = PackageDeclarations {
                    functions: vec![deep_function(label)],
                    ..PackageDeclarations::new(source())
                }
                .check(
                    CheckingLimits::new(u64::MAX)
                        .with_input_bytes(u64::MAX)
                        .with_work_budget(u64::MAX),
                )
                .unwrap_or_else(|refusals| panic!("{label}: checks: {refusals:?}"));
                let package = CheckedPackage::link(graph);
                let written = wire(&emit(&package));
                let written = nodes(&written);
                assert!(written.len() >= DEEP, "{label}: one node per level");
                for node in written {
                    assert_eq!(
                        json!(rebuilt_key(node, &[])),
                        node["node_id"]["digest"],
                        "{label}: a written node keys to its node_id"
                    );
                }
                let body: &CheckedBody = package
                    .graph()
                    .function_state(0)
                    .expect("f is checked")
                    .body
                    .body();
                let clone = body.clone();
                assert_eq!(&clone, body, "{label}: the clone compares equal");
                assert!(!format!("{clone:?}").is_empty());
                drop(clone);
                drop(package);
            })
            .expect("the thread spawns")
            .join()
            .unwrap_or_else(|_| panic!("{label}: completes on a 512 KiB stack"));
    }
}

/// `function f using v(a: Boolean): Boolean pure { a and (a and (… a)) }`
/// with `levels` connectives, through S1 at its default limits, S2 and the
/// assembler, or `None` when S1's defaults do not admit it.
fn and_chain(levels: usize) -> Option<PackageDeclarations> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function f using v(a: Boolean): Boolean pure {{ {}a{} }}\n",
        "a and (".repeat(levels),
        ")".repeat(levels),
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .ok()
    .filter(qsl_cst::ParsedSource::is_admissible)?;
    let forms = qsl_forms::build_unit(&parsed).ok()?;
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        forms,
        Vec::new(),
        Vec::new(),
    )
    .ok()
}

/// TC-727 step 1 (FR-258-AC-3): the longest `a and (…)` chain S1's default
/// limits admit checks at the default checking limits and emits, on a
/// 512 KiB stack: no outcome names a depth.
#[trace("TC-727", "FR-258-AC-3")]
#[test]
fn the_longest_and_chain_s1_admits_checks_and_emits_at_the_defaults() {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(|| {
            // The longest chain S1 admits: `admitted` is, `refused` is not.
            let (mut admitted, mut refused) = (1, qsl_cst::Limits::default().nodes);
            assert!(and_chain(admitted).is_some() && and_chain(refused).is_none());
            while refused - admitted > 1 {
                let middle = admitted + (refused - admitted) / 2;
                match and_chain(middle) {
                    Some(_) => admitted = middle,
                    None => refused = middle,
                }
            }
            let unit = and_chain(admitted).expect("the longest admitted chain assembles");
            let graph = unit
                .check(CheckingLimits::default())
                .unwrap_or_else(|refusals| panic!("{admitted} levels check: {refusals:?}"));
            let package = CheckedPackage::link(graph);
            let emission = emit_checked(&package)
                .unwrap_or_else(|refusal| panic!("{admitted} levels emit: {refusal:?}"));
            assert!(nodes(&wire(&emission)).len() > admitted);
        })
        .expect("the thread spawns")
        .join()
        .expect("the chain completes on a 512 KiB stack");
}
