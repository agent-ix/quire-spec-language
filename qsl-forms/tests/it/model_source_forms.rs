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
    (source, function.as_ref().clone())
}

fn span(source: &str, needle: &str) -> Span {
    let start = source.rfind(needle).expect("literal source region");
    Span {
        start,
        end: start + needle.len(),
    }
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
        let ExprNode::Lookup {
            target, absence, ..
        } = function.body.root_node()
        else {
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
        assert_eq!(
            spans.span(spans.child(spans.root(), 0).unwrap()),
            Some(Span {
                start: span(&source, "(p, r)").start + 1,
                end: span(&source, "(p, r)").start + 2,
            })
        );
        assert_eq!(
            spans.span(spans.child(spans.root(), 1).unwrap()),
            Some(Span {
                start: span(&source, "(p, r)").start + 4,
                end: span(&source, "(p, r)").start + 5,
            })
        );
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
    let ExprNode::Dispatch {
        member, arguments, ..
    } = clause.body.root_node()
    else {
        panic!("expected existing Dispatch node");
    };
    assert_eq!(member, "query");
    assert_eq!(arguments.len(), 2);
    let children = clause.body.root().children();
    assert_eq!(children.len(), 3);
    assert!(matches!(children[0].node(), ExprNode::Deref(_)));
    assert_eq!(
        children[0].children()[0].node(),
        &ExprNode::Name("r".into())
    );
    assert_eq!(children[1].node(), &ExprNode::Integer(11_i64.into()));
    assert_eq!(children[2].node(), &ExprNode::Integer(22_i64.into()));
    let spans = &clause.spans.body;
    assert_eq!(spans.span(spans.root()), Some(span(&source, body)));
    for (index, needle) in ["deref(r)", "11", "22"].into_iter().enumerate() {
        assert_eq!(
            spans.span(spans.child(spans.root(), index).unwrap()),
            Some(span(&source, needle))
        );
    }
}

#[trace("TC-790", "FR-300-AC-1")]
#[test]
fn chained_calls_and_fields_keep_each_receiver_before_its_arguments() {
    let body = "deref(r).first(11).field.second(22, lookup<M::T>(p, r) absent empty)";
    let (source, function) = function(body);
    let root = function.body.root();
    let ExprNode::Dispatch {
        member, arguments, ..
    } = root.node()
    else {
        panic!("second dispatch");
    };
    assert_eq!(member, "second");
    assert_eq!(arguments.len(), 2);
    let children = root.children();
    assert_eq!(children[1].node(), &ExprNode::Integer(22_i64.into()));
    assert!(matches!(
        children[2].node(),
        ExprNode::Lookup {
            absence: AbsenceMode::Empty,
            ..
        }
    ));
    let ExprNode::Field { field, .. } = children[0].node() else {
        panic!("field navigation");
    };
    assert_eq!(field, "field");
    let first = children[0].children()[0];
    let ExprNode::Dispatch {
        member, arguments, ..
    } = first.node()
    else {
        panic!("first dispatch");
    };
    assert_eq!(member, "first");
    assert_eq!(arguments.len(), 1);
    assert_eq!(
        first.children()[1].node(),
        &ExprNode::Integer(11_i64.into())
    );
    assert!(matches!(first.children()[0].node(), ExprNode::Deref(_)));
    let spans = &function.spans().unwrap().body;
    assert_eq!(spans.span(spans.root()), Some(span(&source, body)));
    let field = spans.child(spans.root(), 0).unwrap();
    assert_eq!(
        spans.span(field),
        Some(span(&source, "deref(r).first(11).field"))
    );
    let first = spans.child(field, 0).unwrap();
    assert_eq!(spans.span(first), Some(span(&source, "deref(r).first(11)")));
}

#[trace("TC-790", "FR-300-AC-1", "FR-300-AC-2")]
#[test]
fn model_source_admission_defers_missing_names_and_wrong_keys_to_checking() {
    for (body, member) in [("r.missing()", "missing"), ("r.exhausted()", "exhausted")] {
        let (_, function) = function(body);
        let ExprNode::Dispatch {
            member: actual,
            arguments,
            ..
        } = function.body.root_node()
        else {
            panic!("dispatch");
        };
        assert_eq!(actual, member);
        assert!(arguments.is_empty());
    }
    let (_, lookup) = function("lookup<M::Missing>(p, 17) absent refused");
    let ExprNode::Lookup { target, .. } = lookup.body.root_node() else {
        panic!("lookup");
    };
    assert_eq!(target.head, TypeFormHead::Name("M::Missing".into()));
    assert_eq!(
        lookup.body.root().children()[1].node(),
        &ExprNode::Integer(17_i64.into())
    );
    let (_, field) = function("deref(r).missing");
    assert!(matches!(field.body.root_node(), ExprNode::Field { field, .. } if field == "missing"));
    let (_, instances) = function("allInstances<M::T>(p)");
    let ExprNode::AllInstances { target, .. } = instances.body.root_node() else {
        panic!("allInstances");
    };
    assert_eq!(target.head, TypeFormHead::Name("M::T".into()));
    assert_eq!(
        instances.body.root().children()[0].node(),
        &ExprNode::Name("p".into())
    );
    let (_, reaches) = function("reaches(r, p, parent)");
    assert!(matches!(reaches.body.root_node(), ExprNode::Reaches { edge, .. } if edge == "parent"));
    assert_eq!(
        reaches.body.root().children()[0].node(),
        &ExprNode::Name("r".into())
    );
    assert_eq!(
        reaches.body.root().children()[1].node(),
        &ExprNode::Name("p".into())
    );
}

#[trace("TC-790", "FR-300-AC-1")]
#[test]
fn named_calls_stay_distinct_from_member_calls_and_type_arguments_survive() {
    let (_, function) = function("query(r).member(convert<Int[-2,3]>(1))");
    let children = function.body.root().children();
    assert!(
        matches!(children[0].node(), ExprNode::Call { name, arguments } if name == "query" && arguments.len() == 1)
    );
    let ExprNode::Convert { target, .. } = children[1].node() else {
        panic!("convert");
    };
    assert_eq!(target.bounds, ["-2", "3"]);
    let (_, nested) = self::function("lookup<Reference<M::T>>(query(p), r.query()) absent refused");
    let ExprNode::Lookup { target, .. } = nested.body.root_node() else {
        panic!("lookup");
    };
    assert_eq!(target.arguments.len(), 1);
    assert_eq!(target.arguments[0].head, TypeFormHead::Name("M::T".into()));
    assert!(
        matches!(nested.body.root().children()[0].node(), ExprNode::Call { name, .. } if name == "query")
    );
    assert!(
        matches!(nested.body.root().children()[1].node(), ExprNode::Dispatch { member, arguments, .. } if member == "query" && arguments.is_empty())
    );
}
