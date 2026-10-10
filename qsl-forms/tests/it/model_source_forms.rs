// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-300/301 source prerequisites through the real S1/S2 boundaries.

use ix_trace_rs::trace;
use qsl_cst::{Limits, ParsedSource};
use qsl_forms::{build_unit, DeclarationForm, ExprNode, FunctionDeclaration, TypeFormHead};
use qsl_foundation::{absence::AbsenceMode, SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    profile m = \"quire.model.complete/v1\";\n";

fn parse(declaration: &str) -> (String, ParsedSource) {
    let source = format!("{HEADER}{declaration}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "model-source", "test", "1"),
        "model-source.native",
        source.as_bytes(),
        Limits::default(),
    )
    .expect("valid source metadata");
    assert_eq!(parsed.source().identity().identity, "model-source");
    assert_eq!(parsed.cst().render(), source.as_bytes());
    (source, parsed)
}

fn function(body: &str) -> (String, FunctionDeclaration) {
    let (source, parsed) = parse(&format!(
        "function f using v(p: M::P, r: Reference<M::T>): Reference<M::T> pure {{ {body} }}"
    ));
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 maps the admitted source");
    let DeclarationForm::Function(function) = unit.forms()[0].form() else {
        panic!("expected function");
    };
    (source, function.clone())
}

fn span(source: &str, needle: &str) -> Span {
    let start = source.rfind(needle).expect("literal source region");
    Span { start, end: start + needle.len() }
}

#[trace("TC-790", "TC-792", "FR-300-AC-1", "FR-301-AC-1")]
#[test]
fn lookup_retains_target_operands_explicit_absence_and_spans() {
    for (word, expected) in [
        ("undefined", AbsenceMode::Undefined),
        ("empty", AbsenceMode::Empty),
        ("refused", AbsenceMode::Refused),
    ] {
        let body = format!("lookup<M::T>(p, r) absent {word}");
        let (source, function) = function(&body);
        let ExprNode::Lookup { target, absence, .. } = function.body.root_node() else {
            panic!("expected existing Lookup node");
        };
        assert_eq!(target.head, TypeFormHead::Name("M::T".into()));
        assert!(target.arguments.is_empty());
        assert_eq!(target.span, span(&source, "M::T"));
        assert_eq!(*absence, expected);
        let children = function.body.root().children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].node(), &ExprNode::Name("p".into()));
        assert_eq!(children[1].node(), &ExprNode::Name("r".into()));
        let spans = &function.spans().expect("source-derived spans").body;
        assert_eq!(spans.span(spans.root()), Some(span(&source, &body)));
        assert_eq!(spans.span(spans.child(spans.root(), 0).unwrap()), Some(span(&source, "p")));
        assert_eq!(spans.span(spans.child(spans.root(), 1).unwrap()), Some(span(&source, "r")));
    }
}

#[trace("TC-790", "TC-792", "FR-300-AC-1", "FR-301-AC-2")]
#[test]
fn dispatch_preserves_receiver_and_argument_order_in_a_precondition() {
    let body = "deref(r).query(11, 22)";
    let (source, parsed) = parse(&format!("pre Ready using m on M::T::op {{ {body} }}"));
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 clause mapping");
    let DeclarationForm::StateClause(clause) = unit.forms()[0].form() else {
        panic!("expected precondition");
    };
    let ExprNode::Dispatch { member, arguments, .. } = clause.body.root_node() else {
        panic!("expected existing Dispatch node");
    };
    assert_eq!(member, "query");
    assert_eq!(arguments.len(), 2);
    let children = clause.body.root().children();
    assert_eq!(children.len(), 3);
    assert!(matches!(children[0].node(), ExprNode::Deref(_)));
    assert_eq!(children[0].children()[0].node(), &ExprNode::Name("r".into()));
    assert_eq!(children[1].node(), &ExprNode::Integer(11.into()));
    assert_eq!(children[2].node(), &ExprNode::Integer(22.into()));
    let spans = &clause.spans.body;
    assert_eq!(spans.span(spans.root()), Some(span(&source, body)));
    for (index, needle) in ["deref(r)", "11", "22"].into_iter().enumerate() {
        assert_eq!(spans.span(spans.child(spans.root(), index).unwrap()), Some(span(&source, needle)));
    }
}
