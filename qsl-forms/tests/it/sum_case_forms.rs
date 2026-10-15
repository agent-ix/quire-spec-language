// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-313 source-to-CST-to-forms tests; no semantic admission is asserted.

use ix_trace_rs::trace;
use qsl_cst::{ParsedSource, Production};
use qsl_forms::{
    build_unit, BinaryOperator, BuiltinType, CaseForm, DeclarationForm, ExprNode, Expression,
    FormsCause, FunctionDeclaration, ParsedUnit, TypeFormHead,
};
use qsl_foundation::{SourceIdentity, Span};

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";

fn parse(declarations: &str) -> (String, ParsedSource) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "union.native",
        text.as_bytes(),
        qsl_cst::Limits::default()
            .with_source_bytes(usize::MAX)
            .with_tokens(usize::MAX)
            .with_nodes(usize::MAX),
    )
    .expect("S1 reads source");
    (text, parsed)
}

fn build(declarations: &str) -> (String, ParsedSource, ParsedUnit) {
    let (text, parsed) = parse(declarations);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds admitted source");
    (text, parsed, unit)
}

fn function(unit: &ParsedUnit, index: usize) -> &FunctionDeclaration {
    let DeclarationForm::Function(function) = unit.forms()[index].form() else {
        panic!("function form")
    };
    function
}

fn case(expression: &Expression) -> &CaseForm {
    let ExprNode::Case(case) = expression.root_node() else {
        panic!("case form")
    };
    case
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

fn body(source: &str) -> String {
    format!("function f using v(s: Shape): Integer pure {{ {source} }}")
}

#[trace("TC-815", "FR-313-AC-1")]
#[test]
fn authored_union_keeps_nullary_payload_arity_order_and_spans() {
    let source = "union Shape { Empty, Circle(Integer), Rect(Integer, Integer), }";
    let (text, parsed, unit) = build(source);
    assert_eq!(unit.forms().len(), 1);
    let DeclarationForm::Union(union) = unit.forms()[0].form() else {
        panic!("union form")
    };
    assert_eq!(union.name.name, "Shape");
    assert_eq!(slice(&text, union.name.span), "Shape");
    assert_eq!(slice(&text, union.span), source);
    assert_eq!(unit.forms()[0].span(), union.span);
    assert_eq!(
        union
            .members
            .iter()
            .map(|member| member.name.name.as_str())
            .collect::<Vec<_>>(),
        ["Empty", "Circle", "Rect"]
    );
    for (member, (expected, arity)) in union.members.iter().zip([
        ("Empty", 0),
        ("Circle(Integer)", 1),
        ("Rect(Integer, Integer)", 2),
    ]) {
        assert_eq!(slice(&text, member.span), expected);
        assert_eq!(slice(&text, member.name.span), member.name.name);
        assert_eq!(member.payload.len(), arity);
        for payload in &member.payload {
            assert_eq!(payload.head, TypeFormHead::Builtin(BuiltinType::Integer));
            assert_eq!(slice(&text, payload.span), "Integer");
        }
    }
    assert!(parsed
        .cst()
        .nodes()
        .iter()
        .any(|node| node.production() == Production::UnionDeclaration));
}

#[trace("TC-815", "FR-313-AC-1")]
#[test]
fn independent_payload_positions_and_nested_type_references_are_preserved() {
    let (text, _, unit) = build(
        "union Message { Pair(Text[0, 20; unicode-scalars], Boolean, Option<Sequence<Integer>[0, 8]>), Last, First(Integer) }",
    );
    let DeclarationForm::Union(union) = unit.forms()[0].form() else {
        panic!("union")
    };
    assert_eq!(
        union
            .members
            .iter()
            .map(|member| member.name.name.as_str())
            .collect::<Vec<_>>(),
        ["Pair", "Last", "First"]
    );
    assert_eq!(
        union.members[0]
            .payload
            .iter()
            .map(|payload| slice(&text, payload.span))
            .collect::<Vec<_>>(),
        [
            "Text[0, 20; unicode-scalars]",
            "Boolean",
            "Option<Sequence<Integer>[0, 8]>"
        ]
    );
    assert_eq!(
        union.members[0].payload[2].head,
        TypeFormHead::Builtin(BuiltinType::Option)
    );
    assert_eq!(
        union.members[0].payload[2].arguments[0].arguments[0].head,
        TypeFormHead::Builtin(BuiltinType::Integer)
    );
}

#[trace("TC-815", "FR-313-AC-2")]
#[test]
fn authored_case_keeps_member_binder_body_order_and_source_spans() {
    let source = "case s { Circle(r): r; Shape::Rect(w, h): w * h; Empty: 0; }";
    let (text, _, unit) = build(&body(source));
    let function = function(&unit, 0);
    let expression = &function.body;
    let case = case(expression);
    assert_eq!(slice(&text, case.span), source);
    assert_eq!(
        expression.at(case.scrutinee).node(),
        &ExprNode::Name("s".into())
    );
    assert_eq!(
        case.arms
            .iter()
            .map(|arm| arm.member.name.as_str())
            .collect::<Vec<_>>(),
        ["Circle", "Shape::Rect", "Empty"]
    );
    assert_eq!(
        case.arms
            .iter()
            .map(|arm| arm
                .binders
                .iter()
                .map(|binder| binder.name.as_str())
                .collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        [vec!["r"], vec!["w", "h"], vec![]]
    );
    for (arm, expected) in
        case.arms
            .iter()
            .zip(["Circle(r): r;", "Shape::Rect(w, h): w * h;", "Empty: 0;"])
    {
        assert_eq!(slice(&text, arm.span), expected);
        assert_eq!(slice(&text, arm.member.span), arm.member.name);
        for binder in &arm.binders {
            assert_eq!(slice(&text, binder.span), binder.name);
        }
    }
    assert_eq!(
        expression.at(case.arms[0].body).node(),
        &ExprNode::Name("r".into())
    );
    let ExprNode::Binary {
        operator,
        left,
        right,
    } = expression.at(case.arms[1].body).node()
    else {
        panic!("multiplication")
    };
    assert_eq!(*operator, BinaryOperator::Multiply);
    assert_eq!(expression.at(*left).node(), &ExprNode::Name("w".into()));
    assert_eq!(expression.at(*right).node(), &ExprNode::Name("h".into()));
    assert_eq!(
        expression.at(case.arms[2].body).node(),
        &ExprNode::Integer(0_i64.into())
    );
    let spans = &function.spans().unwrap().body;
    assert!(spans.fits(expression));
    assert_eq!(slice(&text, spans.span(spans.root()).unwrap()), source);
    for (index, expected) in ["s", "r", "w * h", "0"].iter().enumerate() {
        assert_eq!(
            slice(
                &text,
                spans
                    .span(spans.child(spans.root(), index).unwrap())
                    .unwrap()
            ),
            *expected
        );
    }
}

#[trace("TC-815", "FR-313-AC-2")]
#[test]
fn construction_stays_name_or_call_and_duplicate_arms_stay_authored() {
    let (_, _, unit) = build(&format!(
        "{} {} {}",
        body("Shape::Empty"),
        body("Shape::Circle(3)"),
        body("case s { Rect(h, w): h; Empty: 1; Rect(w, h): w; }")
    ));
    assert_eq!(
        function(&unit, 0).body.root_node(),
        &ExprNode::Name("Shape::Empty".into())
    );
    let expression = &function(&unit, 1).body;
    let ExprNode::Call { name, arguments } = expression.root_node() else {
        panic!("call")
    };
    assert_eq!(name, "Shape::Circle");
    assert_eq!(arguments.len(), 1);
    assert_eq!(
        expression.at(arguments[0]).node(),
        &ExprNode::Integer(3_i64.into())
    );
    let case = case(&function(&unit, 2).body);
    assert_eq!(
        case.arms
            .iter()
            .map(|arm| arm.member.name.as_str())
            .collect::<Vec<_>>(),
        ["Rect", "Empty", "Rect"]
    );
    assert_eq!(
        case.arms[0]
            .binders
            .iter()
            .map(|binder| binder.name.as_str())
            .collect::<Vec<_>>(),
        ["h", "w"]
    );
    assert_eq!(
        case.arms[2]
            .binders
            .iter()
            .map(|binder| binder.name.as_str())
            .collect::<Vec<_>>(),
        ["w", "h"]
    );
}

#[trace("TC-816", "FR-313-AC-3")]
#[test]
fn expression_and_real_protocol_choice_cases_coexist_in_one_admitted_unit() {
    let expression_source = "case (R { f: 1 }) { A: 0; }";
    let protocol = "protocol P using v over (input: Shape) on origin { role R on Shape; run choice Decide by R visible (true) { case Yes when { true } sequence Y { } case No when { false } sequence N { } } finish End as (outcome: Boolean) { true }; }";
    let (text, parsed, unit) = build(&format!("{} {protocol}", body(expression_source)));
    let expression = &function(&unit, 0).body;
    assert!(
        matches!(expression.at(case(expression).scrutinee).node(), ExprNode::Record { name, .. } if name == "R")
    );
    let expressions = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::CaseExpression)
        .collect::<Vec<_>>();
    assert_eq!(expressions.len(), 1);
    assert_eq!(slice(&text, expressions[0].span()), expression_source);
    let protocol_cases = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::Case)
        .map(|node| slice(&text, node.span()))
        .collect::<Vec<_>>();
    assert_eq!(
        protocol_cases,
        [
            "case Yes when { true } sequence Y { }",
            "case No when { false } sequence N { }"
        ]
    );
    let DeclarationForm::Protocol(protocol) = unit.forms()[1].form() else {
        panic!("protocol form")
    };
    assert_eq!(protocol.name.name, "P");
    let cases = protocol
        .declarations
        .iter()
        .filter(|node| node.kind == qsl_forms::ProtocolNodeKind::Case)
        .collect::<Vec<_>>();
    assert_eq!(
        cases
            .iter()
            .map(|node| node.name.name.as_str())
            .collect::<Vec<_>>(),
        ["Yes", "No"]
    );
    for node in cases {
        assert_eq!(slice(&text, node.name.span), node.name.name);
        assert_eq!(
            protocol
                .scope_names(node.scope)
                .iter()
                .map(|name| name.name.as_str())
                .collect::<Vec<_>>(),
            ["Decide"]
        );
    }
}

#[trace("TC-816", "FR-313-AC-3")]
#[test]
fn unparenthesized_record_opens_arm_list_and_fails_there() {
    let (text, parsed) = parse(&body("case R { f: 1 } { A: 0; }"));
    assert!(!parsed.is_admissible());
    let diagnostic = &parsed.diagnostics()[0];
    assert_eq!(diagnostic.code, qsl_cst::CompleteCode::InvalidSyntax);
    assert_eq!(slice(&text, diagnostic.byte_span().unwrap()), "}");
    assert!(parsed.cst().recoveries()[0].expected.contains(';'));
    assert_eq!(
        build_unit(&parsed).unwrap_err().cause,
        FormsCause::RecoveringCst
    );
}

#[trace("TC-815", "FR-313-AC-1", "FR-313-AC-2")]
#[test]
fn malformed_union_and_case_fail_at_their_grammar_not_setup() {
    for (source, offending) in [
        ("union U { A() }".into(), ")"),
        ("union U { A(Integer,) }".into(), ")"),
        ("union U { A B }".into(), "B"),
        (body("case s { A(): 0; }"), ")"),
        (body("case s { A(x,): x; }"), ")"),
        (body("case s { A: 0 }"), "}"),
        (body("case s { A when true: 0; }"), "when"),
    ] {
        let (text, parsed) = parse(&source);
        assert!(!parsed.is_admissible(), "{source}");
        assert_eq!(
            parsed.diagnostics()[0].code,
            qsl_cst::CompleteCode::InvalidSyntax,
            "{source}"
        );
        assert_eq!(
            slice(&text, parsed.diagnostics()[0].byte_span().unwrap()),
            offending,
            "{source}"
        );
        assert_eq!(
            build_unit(&parsed).unwrap_err().cause,
            FormsCause::RecoveringCst
        );
    }
}

#[trace("TC-815", "FR-313-AC-2")]
#[test]
fn arena_walks_and_rewrites_reach_case_children_without_renaming_arm_binders() {
    let (_, _, unit) = build(&body("case s { A(x): case x { B(y): y; }; C: s; }"));
    let expression = &function(&unit, 0).body;
    assert_eq!(expression.root_node().children().len(), 3);
    let rewritten = expression.respell_names(|_, spellings| {
        if let Some(name) = spellings.reference {
            *name = format!("renamed_{name}");
        }
        for binder in spellings.binders {
            *binder = format!("renamed_{binder}");
        }
    });
    let outer = case(&rewritten);
    assert_eq!(
        rewritten.at(outer.scrutinee).node(),
        &ExprNode::Name("renamed_s".into())
    );
    assert_eq!(outer.arms[0].binders[0].name, "x");
    assert_eq!(
        rewritten.at(outer.arms[1].body).node(),
        &ExprNode::Name("renamed_s".into())
    );
    let ExprNode::Case(inner) = rewritten.at(outer.arms[0].body).node() else {
        panic!("nested case")
    };
    assert_eq!(inner.arms[0].binders[0].name, "y");
    assert_eq!(
        rewritten.at(inner.scrutinee).node(),
        &ExprNode::Name("renamed_x".into())
    );
    assert_eq!(
        rewritten.at(inner.arms[0].body).node(),
        &ExprNode::Name("renamed_y".into())
    );
    assert_eq!(rewritten.root_node().binders(), ["x"]);
    let composed = Expression::call("f", vec![Expression::name("prefix"), rewritten.clone()]);
    let ExprNode::Call { arguments, .. } = composed.root_node() else {
        panic!("call")
    };
    let ExprNode::Case(shifted) = composed.at(arguments[1]).node() else {
        panic!("shifted case")
    };
    assert_eq!(
        composed.at(shifted.scrutinee).node(),
        &ExprNode::Name("renamed_s".into())
    );
    assert_eq!(
        composed.at(shifted.arms[1].body).node(),
        &ExprNode::Name("renamed_s".into())
    );
    assert!(expression
        .respelled(|_, node| {
            if let ExprNode::Name(name) = node {
                *name = format!("{name}_copy");
            }
        })
        .is_ok());
}

#[trace("TC-815", "FR-313-AC-1", "FR-313-AC-2", "FR-257-AC-1")]
#[test]
fn deep_case_arenas_and_union_payloads_use_existing_charged_growth() {
    let depth = 10_000;
    let source = format!(
        "union U {{ A({}Integer{}) }} {}",
        "Option<".repeat(depth),
        ">".repeat(depth),
        body(&format!(
            "{}s{}",
            "case s { A(x): ".repeat(depth),
            "; }".repeat(depth)
        ))
    );
    let (_, parsed, unit) = build(&source);
    let expression = &function(&unit, 1).body;
    assert_eq!(expression.len(), depth * 2 + 1);
    assert!(expression.len() <= parsed.cst().nodes().len());
    assert_eq!(
        expression
            .iter()
            .filter(|node| matches!(node.node(), ExprNode::Case(_)))
            .count(),
        depth
    );
    assert_eq!(&expression.clone(), expression);
    let DeclarationForm::Union(union) = unit.forms()[0].form() else {
        panic!("union")
    };
    let payload = &union.members[0].payload[0];
    assert_eq!(&payload.clone(), payload);
    let mut nested = payload;
    for _ in 0..depth {
        assert_eq!(nested.head, TypeFormHead::Builtin(BuiltinType::Option));
        nested = &nested.arguments[0];
    }
    assert_eq!(nested.head, TypeFormHead::Builtin(BuiltinType::Integer));
}

#[trace("TC-815", "FR-313-AC-2", "FR-257-AC-1")]
#[test]
fn default_frontend_limits_admit_a_thousand_nested_cases_and_charge_binder_nodes() {
    let depth = 1_000;
    let text = format!(
        "{HEADER}{}",
        body(&format!(
            "{}s{}",
            "case s { A(x): ".repeat(depth),
            "; }".repeat(depth)
        ))
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "case.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("default S1 limits");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("default S2 path");
    assert_eq!(function(&unit, 0).body.len(), depth * 2 + 1);
    assert_eq!(
        parsed
            .cst()
            .nodes()
            .iter()
            .filter(|node| node.production() == Production::CaseBinder)
            .count(),
        depth
    );
}

#[trace("TC-815", "FR-313-AC-2", "FR-257-AC-1")]
#[test]
fn wide_arm_binders_are_admitted_or_refused_by_the_existing_s1_node_budget() {
    let binders = (0..64)
        .map(|index| format!("x{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let (text, parsed, _) = build(&body(&format!("case s {{ A({binders}): x0; }}")));
    let nodes = parsed.cst().nodes().len();
    let identity = || SourceIdentity::new("a", "u", "git", "1");
    let exact = qsl_cst::parse(
        identity(),
        "union.native",
        text.as_bytes(),
        qsl_cst::Limits::default().with_nodes(nodes),
    )
    .expect("exact node budget admits");
    assert!(exact.is_admissible(), "{:?}", exact.diagnostics());
    let forms = build_unit(&exact).expect("no extra S2 limit");
    assert_eq!(case(&function(&forms, 0).body).arms[0].binders.len(), 64);
    let refused = qsl_cst::parse(
        identity(),
        "union.native",
        text.as_bytes(),
        qsl_cst::Limits::default().with_nodes(nodes - 1),
    )
    .expect_err("one fewer node refuses");
    assert_eq!(refused.code, qsl_cst::CompleteCode::StageLimitExceeded);
    assert_eq!(
        refused.limit(),
        Some(qsl_foundation::SyntaxLimit::Nodes { bound: nodes - 1 })
    );
}

#[trace("TC-816", "FR-313-AC-3")]
#[test]
fn scrutinee_restriction_follows_unbracketed_edges_and_resets_inside_delimiters() {
    for scrutinee in [
        "f(R { f: 1 })",
        "sequence[R { f: 1 }]",
        "if true then (R { f: 1 }) else s",
        "let x = (R { f: 1 }) in x",
    ] {
        let (_, _, unit) = build(&body(&format!("case {scrutinee} {{ A: 0; }}")));
        assert_eq!(case(&function(&unit, 0).body).arms[0].member.name, "A");
    }
    for scrutinee in [
        "if true then R { f: 1 } else s",
        "let x = R { f: 1 } in x",
        "s + R { f: 1 }",
    ] {
        let (_, parsed) = parse(&body(&format!("case {scrutinee} {{ A: 0; }}")));
        assert!(!parsed.is_admissible(), "{scrutinee}");
        assert_eq!(
            parsed.diagnostics()[0].code,
            qsl_cst::CompleteCode::InvalidSyntax
        );
    }
}
