// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-091: the `Value` family's S2 production, run on real complete-V1
//! source through `qsl_cst::parse` and `build_unit`.

use ix_trace_rs::trace;
use qsl_cst::{CstElement, LosslessCst, ParsedSource, Production};
use qsl_forms::{
    build_unit, Accumulation, BinaryOperator, BinderQuery, BuiltinType, DeclarationForm,
    DeclarationKind, ExactNumberKind, ExprNode, ExprRef, Expression, FieldInitializer, FormsCause,
    FunctionDeclaration, ParsedUnit, TermOperator, TypeFormHead,
};
use qsl_foundation::{Code, SourceIdentity, Span, SyntaxLimit};
use quire_exact::{CollectionKind, Integer};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";

fn parse(declarations: &str) -> (String, ParsedSource) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    (text, parsed)
}

fn admissible(declarations: &str) -> (String, ParsedSource) {
    let (text, parsed) = parse(declarations);
    assert!(
        parsed.is_admissible(),
        "{declarations}: {:?}",
        parsed.diagnostics()
    );
    (text, parsed)
}

fn build(declarations: &str) -> (String, ParsedUnit) {
    let (text, parsed) = admissible(declarations);
    let unit = build_unit(&parsed).unwrap_or_else(|failure| panic!("{declarations}: {failure:?}"));
    (text, unit)
}

fn function(form: &DeclarationForm) -> &FunctionDeclaration {
    match form {
        DeclarationForm::Function(function) => function,
        DeclarationForm::Alias(_)
        | DeclarationForm::Record(_)
        | DeclarationForm::Tuple(_)
        | DeclarationForm::Enum(_)
        | DeclarationForm::Dimension(_)
        | DeclarationForm::Unit(_)
        | DeclarationForm::StateClause(_)
        | DeclarationForm::Protocol(_) => panic!("a function form, not {form:?}"),
    }
}

/// The body of `function f using v(): Boolean pure { <body> }`.
fn body(body: &str) -> (String, FunctionDeclaration) {
    let (text, unit) = build(&format!("function f using v(): Boolean pure {{ {body} }}"));
    (text, function(unit.forms()[0].form()).clone())
}

fn refusal(declarations: &str) -> (String, FormsCause, Option<Span>) {
    let (text, parsed) = admissible(declarations);
    match build_unit(&parsed) {
        Err(refusal) => (text, refusal.cause, refusal.span),
        other => panic!("{declarations}: a refusal, not {other:?}"),
    }
}

fn span_of(text: &str, needle: &str) -> Span {
    let start = text.rfind(needle).expect("the needle is in the unit");
    Span {
        start,
        end: start + needle.len(),
    }
}

fn name(spelling: &str) -> String {
    spelling.to_owned()
}

/// A compact rendering of an expression tree, operands in order.
fn show(expression: ExprRef<'_>) -> String {
    let children: Vec<String> = expression.children().into_iter().map(show).collect();
    let head = match expression.node() {
        ExprNode::Boolean(value) => format!("{value}"),
        ExprNode::Integer(value) => format!("{value}"),
        ExprNode::Rational(n, d) => format!("rational({n},{d})"),
        ExprNode::Name(spelling) => spelling.clone(),
        ExprNode::Let { name, .. } => format!("Let[{name}]"),
        ExprNode::If { .. } => "If".into(),
        ExprNode::Binary { operator, .. } => format!("{operator:?}"),
        ExprNode::Negate(_) => "Negate".into(),
        ExprNode::Not(_) => "Not".into(),
        ExprNode::Field { field, .. } => format!("Field[{field}]"),
        ExprNode::Present(_) => "Present".into(),
        ExprNode::Value(_) => "Value".into(),
        ExprNode::Deref(_) => "Deref".into(),
        ExprNode::Call { name, .. } => format!("Call[{name}]"),
        ExprNode::Record { name, fields } => {
            let fields: Vec<String> = fields
                .iter()
                .map(|(field, initializer)| match initializer {
                    FieldInitializer::Value(_) => field.clone(),
                    FieldInitializer::Null => format!("{field}=null"),
                })
                .collect();
            format!("Record[{name};{}]", fields.join(","))
        }
        ExprNode::Collection { kind, .. } => format!("Collection[{kind:?}]"),
        ExprNode::Convert { target, .. } => format!("Convert[{:?}]", target.head),
        ExprNode::Query { query, binder, .. } => format!("Query[{query:?},{binder}]"),
        ExprNode::Flatten(_) => "Flatten".into(),
        ExprNode::Accumulate {
            form,
            accumulator_type,
            accumulator,
            binder,
            ..
        } => format!("Accumulate[{form:?},{accumulator_type},{accumulator},{binder}]"),
        ExprNode::Count {
            result_type,
            binder,
            ..
        } => format!("Count[{result_type},{binder}]"),
        ExprNode::Sum {
            result_type,
            binder,
            ..
        } => format!("Sum[{result_type},{binder}]"),
        ExprNode::Size(_) => "Size".into(),
        ExprNode::Contains { .. } => "Contains".into(),
        ExprNode::AllInstances { target, .. } => format!("AllInstances[{:?}]", target.head),
        ExprNode::Lookup { .. } => "Lookup".into(),
        ExprNode::Dispatch { .. } => "Dispatch".into(),
        ExprNode::Pre(_) => "Pre".into(),
        ExprNode::SelfRef => "SelfRef".into(),
        ExprNode::Result => "Result".into(),
        ExprNode::Reaches { edge, .. } => format!("Reaches[{edge}]"),
    };
    if children.is_empty() {
        head
    } else {
        format!("{head}({})", children.join(", "))
    }
}

/// The spans of the unit's `Declaration` nodes and its `Profile` node.
fn declaration_spans(cst: &LosslessCst, production: Production) -> Vec<Span> {
    cst.root()
        .children()
        .iter()
        .filter_map(|child| match child {
            CstElement::Node(index) => cst.nodes().get(*index),
            CstElement::Token(_) => None,
        })
        .filter(|node| node.production() == production)
        .map(|node| node.span())
        .collect()
}

#[trace("FR-091-AC-1", "TC-392")]
#[test]
fn a_unit_builds_one_form_per_declaration_in_source_order() {
    let (_, parsed) = admissible(
        "type Digit = Int[0, 9];\n\
         function inc using v(x: Digit): Int[0, 10] pure { x + 1 }\n\
         record Point { x: Int[0, 9]; }\n\
         tuple Pair(Int[0, 9], Int[0, 9]);",
    );
    let unit = build_unit(&parsed).expect("the unit builds");
    let kinds: Vec<&str> = unit
        .forms()
        .iter()
        .map(|form| match form.form() {
            DeclarationForm::Alias(_) => "alias",
            DeclarationForm::Function(_) => "function",
            DeclarationForm::Record(_) => "record",
            DeclarationForm::Tuple(_) => "tuple",
            DeclarationForm::Enum(_) => "enum",
            DeclarationForm::Dimension(_) => "dimension",
            DeclarationForm::Unit(_) => "unit",
            DeclarationForm::StateClause(_) => "state_clause",
            DeclarationForm::Protocol(_) => "protocol",
        })
        .collect();
    assert_eq!(kinds, ["alias", "function", "record", "tuple"]);
    let spans: Vec<Span> = unit.forms().iter().map(|form| form.span()).collect();
    assert_eq!(
        spans,
        declaration_spans(parsed.cst(), Production::Declaration)
    );
    assert_eq!(unit.edition(), Some("1-draft"));
    for form in unit.forms() {
        assert_eq!(form.edition(), Some("1-draft"));
    }
    let selections = unit.selections();
    assert_eq!(selections.profiles.len(), 1);
    assert!(selections.imports.is_empty());
    assert!(selections.models.is_empty());
    let profile = &selections.profiles[0];
    assert_eq!(profile.alias, "v");
    assert_eq!(profile.identity, "quire.value.complete/v1");
    assert_eq!(
        vec![profile.span],
        declaration_spans(parsed.cst(), Production::Profile)
    );
}

#[trace("FR-091-AC-2", "TC-393")]
#[test]
fn a_function_form_carries_its_signature_as_syntax() {
    let (text, unit) = build(
        "function inc using v(x: Int[0, 9], y: Digit): Int[0, 10] pure decreases(x) { x + 1 }",
    );
    let inc = function(unit.forms()[0].form());
    assert_eq!(inc.name, "inc");
    let using = inc.using().expect("the using alias is carried");
    assert_eq!(using.alias, "v");
    assert_eq!(&text[using.span.start..using.span.end], "v");
    assert_eq!(using.span, span_of(&text, "v(x").start_and(1));
    let [(x, x_type), (y, y_type)] = inc.parameters.as_slice() else {
        panic!("two parameters");
    };
    assert_eq!((x.as_str(), y.as_str()), ("x", "y"));
    assert!(matches!(
        x_type.head,
        TypeFormHead::Builtin(BuiltinType::Int)
    ));
    assert_eq!(x_type.bounds, ["0", "9"]);
    assert!(matches!(&y_type.head, TypeFormHead::Name(name) if name == "Digit"));
    assert!(matches!(
        inc.result.head,
        TypeFormHead::Builtin(BuiltinType::Int)
    ));
    assert_eq!(inc.result.bounds, ["0", "10"]);
    assert!(
        matches!(inc.measure.as_ref().map(Expression::root_node), Some(ExprNode::Name(name)) if name == "x")
    );
    assert_eq!(show(inc.body.root()), "Add(x, 1)");
}

/// A span's first `length` bytes.
trait StartAnd {
    fn start_and(self, length: usize) -> Span;
}

impl StartAnd for Span {
    fn start_and(self, length: usize) -> Span {
        Span {
            start: self.start,
            end: self.start + length,
        }
    }
}

#[trace("FR-091-AC-3", "TC-394")]
#[trace("TC-456", "FR-102-AC-3")]
#[test]
fn each_expression_construct_maps_to_its_variant() {
    let cases = [
        ("true", "true"),
        ("false", "false"),
        ("42", "42"),
        ("rational(1, -2)", "rational(1,-2)"),
        ("a", "a"),
        ("M::a", "M::a"),
        ("E::m", "E::m"),
        ("let x = a in b", "Let[x](a, b)"),
        ("if a then b else c", "If(a, b, c)"),
        ("a implies b", "Implies(a, b)"),
        ("a or b", "Or(a, b)"),
        ("a and b", "And(a, b)"),
        ("a = b", "Equal(a, b)"),
        ("a != b", "NotEqual(a, b)"),
        ("a < b", "Less(a, b)"),
        ("a <= b", "LessOrEqual(a, b)"),
        ("a > b", "Greater(a, b)"),
        ("a >= b", "GreaterOrEqual(a, b)"),
        ("a + b", "Add(a, b)"),
        ("a - b", "Subtract(a, b)"),
        ("a * b", "Multiply(a, b)"),
        ("a / b", "Divide(a, b)"),
        ("not a", "Not(a)"),
        ("-a", "Negate(a)"),
        ("a.f", "Field[f](a)"),
        ("a.f.g", "Field[g](Field[f](a))"),
        ("present(a)", "Present(a)"),
        ("value(a)", "Value(a)"),
        ("f(a, b)", "Call[f](a, b)"),
        ("f()", "Call[f]"),
        ("M::Q(a, b)", "Call[M::Q](a, b)"),
        ("R { f: a, g: null }", "Record[R;f,g=null](a)"),
        ("sequence[a, b]", "Collection[Sequence](a, b)"),
        ("set[a]", "Collection[Set](a)"),
        ("bag[a]", "Collection[Bag](a)"),
        ("orderedSet[a]", "Collection[OrderedSet](a)"),
        ("map(x in c: x)", "Query[Map,x](c, x)"),
        ("collect(x in c: x)", "Query[Map,x](c, x)"),
        ("filter(x in c: a)", "Query[Filter,x](c, a)"),
        ("flatMap(x in c: a)", "Query[FlatMap,x](c, a)"),
        ("forall(x in c: a)", "Query[Forall,x](c, a)"),
        ("exists(x in c: a)", "Query[Exists,x](c, a)"),
        ("flatten(c)", "Flatten(c)"),
        (
            "fold<A>(acc, x in c: a, identity: b)",
            "Accumulate[Fold,A,acc,x](c, a, b)",
        ),
        (
            "reduce<A>(acc, x in c: a)",
            "Accumulate[Reduce,A,acc,x](c, a)",
        ),
        ("count<N>(x in c: a)", "Count[N,x](c, a)"),
        ("sum<N>(x in c: a)", "Sum[N,x](c, a)"),
        ("size(c)", "Size(c)"),
        ("contains(c, a)", "Contains(c, a)"),
        ("convert<Int[0, 9]>(a)", "Convert[Builtin(Int)](a)"),
        ("deref(a)", "Deref(a)"),
        ("pre(a)", "Pre(a)"),
        ("allInstances<M::T>(p)", "AllInstances[Name(\"M::T\")](p)"),
        ("(a)", "a"),
        // FR-102 (TC-456): `self`, `result` and a single-segment
        // `reaches` edge build as expressions instead of refusing.
        ("self", "SelfRef"),
        ("result", "Result"),
        ("reaches(a, b, e)", "Reaches[e](a, b)"),
        ("not reaches(a, a, e)", "Not(Reaches[e](a, a))"),
    ];
    for (source, expected) in cases {
        let (_, declaration) = body(source);
        assert_eq!(show(declaration.body.root()), expected, "{source}");
    }
    let (_, declaration) = body("R { f: a, g: null }");
    let ExprNode::Record { fields, .. } = declaration.body.root_node() else {
        panic!("a record value");
    };
    assert!(matches!(fields[1], (ref g, FieldInitializer::Null) if g == "g"));
    let (_, declaration) = body("rational(1, -2)");
    assert!(matches!(
        declaration.body.root_node(),
        ExprNode::Rational(n, d) if *n == Integer::from(1_i64) && *d == Integer::from(-2_i64)
    ));
    let (_, declaration) = body("sequence[a]");
    assert!(matches!(
        declaration.body.root_node(),
        ExprNode::Collection {
            kind: CollectionKind::Sequence,
            ..
        }
    ));
    let (_, declaration) = body("fold<A>(acc, x in c: a)");
    assert!(matches!(
        declaration.body.root_node(),
        ExprNode::Accumulate {
            form: Accumulation::Fold,
            identity: None,
            ..
        }
    ));
    let (_, declaration) = body("map(x in c: x)");
    assert!(matches!(
        declaration.body.root_node(),
        ExprNode::Query {
            query: BinderQuery::Map,
            ..
        }
    ));
    let (_, declaration) = body("a + b");
    assert!(matches!(
        declaration.body.root_node(),
        ExprNode::Binary {
            operator: BinaryOperator::Add,
            ..
        }
    ));
}

#[trace("FR-091-AC-3", "TC-394")]
#[test]
fn grouping_and_associativity_come_from_the_cst() {
    let cases = [
        ("a or b and c", "Or(a, And(b, c))"),
        ("a - b - c", "Subtract(Subtract(a, b), c)"),
        ("a implies b implies c", "Implies(a, Implies(b, c))"),
        ("(a + b) * c", "Multiply(Add(a, b), c)"),
        ("a + b * c - d", "Subtract(Add(a, Multiply(b, c)), d)"),
        ("a + b + c + d", "Add(Add(Add(a, b), c), d)"),
        ("not not a", "Not(Not(a))"),
    ];
    for (source, expected) in cases {
        let (_, declaration) = body(source);
        assert_eq!(show(declaration.body.root()), expected, "{source}");
    }
}

#[trace("FR-091-AC-4", "FR-091-AC-5", "FR-091-AC-6", "TC-395")]
#[test]
fn s2_refuses_inadmissible_input_and_undispatched_declarations() {
    let (_, recovering) = parse("function f using v(): Boolean pure { true ");
    assert!(!recovering.cst().recoveries().is_empty());
    match build_unit(&recovering) {
        Err(refusal) => {
            assert_eq!(refusal.cause, FormsCause::RecoveringCst);
            assert_eq!(refusal.cause.catalog_code(), Code::InvalidSyntax);
        }
        other => panic!("RecoveringCst, not {other:?}"),
    }

    let (_, mut diagnosed) = admissible("function f using v(): Boolean pure { true }");
    let (_, recovering_diagnostic) = parse("function f using v(): Boolean pure { true ");
    let mut prepended = recovering_diagnostic.diagnostics()[0].clone();
    prepended.code = Code::UnknownProfile;
    diagnosed.prepend_diagnostic(prepended);
    assert!(diagnosed.cst().recoveries().is_empty());
    match build_unit(&diagnosed) {
        Err(refusal) => {
            assert_eq!(
                refusal.cause,
                FormsCause::DiagnosedSource(Code::UnknownProfile)
            );
            assert_eq!(refusal.cause.catalog_code(), Code::UnknownProfile);
        }
        other => panic!("the diagnosed-source cause, not {other:?}"),
    }

    // FR-102 gave `invariant`/`pre`/`post` their own dispatch
    // entry, so FR-091-AC-6's undispatched case moves to a spelling no
    // family claims yet: `synthesis` (`SynthesisDeclaration`).
    let synthesis = "synthesis Syn using v grammar M::G domain M::D satisfies { true };";
    let (text, cause, span) = refusal(&format!(
        "function t using v(): Boolean pure {{ true }}\n{synthesis}"
    ));
    assert_eq!(
        cause,
        FormsCause::NoDispatchEntry {
            spelling: name("synthesis")
        }
    );
    assert_eq!(span, Some(span_of(&text, synthesis)));
}

#[trace("FR-091-AC-7", "TC-396")]
#[test]
fn a_construct_no_variant_represents_refuses_the_unit() {
    let cases = [
        ("\"text\"", Production::Primary, "\"text\""),
        ("decimal(1, 2)", Production::ExactNumber, "decimal(1, 2)"),
        ("a mod b", Production::Product, "a mod b"),
        ("xs[0]", Production::Postfix, "xs[0]"),
        ("none", Production::Primary, "none"),
        // FR-102: a single-segment `reaches` edge now builds
        // (`each_expression_construct_maps_to_its_variant`); a
        // multi-segment edge still refuses, but at the qualified edge
        // itself, not the whole construct (FR-102-AC-3).
        ("reaches(a, b, M::R)", Production::QualifiedName, "M::R"),
        ("if a then b else a mod b", Production::Product, "a mod b"),
        (
            "size<Int[0, 1]>(c)",
            Production::Primary,
            "size<Int[0, 1]>(c)",
        ),
        ("a div b", Production::Product, "a div b"),
        ("null", Production::Primary, "null"),
        ("a rem b", Production::Product, "a rem b"),
        (
            "float32(bits: 0x00000000)",
            Production::FloatValue,
            "float32(bits: 0x00000000)",
        ),
        (
            "float64(bits: 0x0000000000000000)",
            Production::FloatValue,
            "float64(bits: 0x0000000000000000)",
        ),
    ];
    for (source, production, construct) in cases {
        let (text, cause, span) = refusal(&format!(
            "function f using v(): Boolean pure {{ {source} }}"
        ));
        assert_eq!(
            cause,
            FormsCause::UnrepresentedConstruct { production },
            "{source}"
        );
        assert_eq!(cause.catalog_code(), Code::UnsupportedConstruct);
        assert_eq!(span, Some(span_of(&text, construct)), "{source}");
    }
}

#[trace("FR-091-AC-8", "TC-396")]
#[test]
fn nested_constructs_of_other_families_are_built_where_written() {
    for (source, expected) in [
        ("deref(x)", "Deref(x)"),
        ("allInstances<M::T>(x)", "AllInstances[Name(\"M::T\")](x)"),
        ("pre(x)", "Pre(x)"),
    ] {
        let (_, unit) = build(&format!(
            "function f using v(x: Int[0, 9]): Boolean pure {{ {source} }}"
        ));
        assert_eq!(show(function(unit.forms()[0].form()).body.root()), expected);
    }
    let (_, unit) =
        build("function g using v(x: Int[0, 9]): Boolean pure decreases(pre(x)) { true }");
    let measure = function(unit.forms()[0].form())
        .measure
        .as_ref()
        .expect("the measure is carried");
    assert_eq!(show(measure.root()), "Pre(x)");
}

fn nots(count: usize) -> String {
    format!("{}a", "not ".repeat(count))
}

/// The depth of an expression tree: one forward loop over its arena, each
/// node one deeper than its deepest child.
fn depth(tree: &Expression) -> usize {
    let mut depths: Vec<usize> = Vec::with_capacity(tree.len());
    for node in tree.iter() {
        let deepest = node
            .node()
            .children()
            .into_iter()
            .map(|child| depths[child.index()])
            .max()
            .unwrap_or(0);
        depths.push(deepest + 1);
    }
    depths.last().copied().unwrap_or(0)
}

#[trace("FR-091-AC-9", "TC-397")]
#[test]
fn deep_not_chains_build_with_no_s2_limit() {
    for count in [8, 20, 129] {
        let (_, unit) = build(&format!(
            "function f using v(a: Boolean): Boolean pure {{ {} }}",
            nots(count)
        ));
        let body = &function(unit.forms()[0].form()).body;
        assert_eq!(depth(body), count + 1, "{count}");
    }
}

/// FR-096-AC-3: S1 over a body of eight `not`s with its node ceiling one
/// below the unit's node count refuses naming the node ceiling at the
/// region its diagnostic names; S2 over the same body, parsed at the
/// default S1 limits, builds its form.
#[trace("FR-096-AC-3", "TC-427")]
#[test]
fn s1_bounds_the_body_and_s2_builds_it() {
    let declaration = format!(
        "function f using v(a: Boolean): Boolean pure {{ {} }}",
        nots(8)
    );
    let (_, parsed) = admissible(&declaration);
    let nodes = parsed.cst().nodes().len();
    let text = format!("{HEADER}{declaration}\n");
    let refusal = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default().with_nodes(nodes - 1),
    )
    .expect_err("one node below the unit's count refuses");
    assert_eq!(
        refusal.limit(),
        Some(SyntaxLimit::Nodes { bound: nodes - 1 })
    );
    assert!(refusal.byte_span().is_some(), "{refusal:?}");
    let unit = build_unit(&parsed).expect("S2 builds the body");
    assert_eq!(depth(&function(unit.forms()[0].form()).body), 9);
}

#[trace("FR-091-AC-10", "TC-403")]
#[test]
fn every_expression_node_carries_its_span() {
    let (text, declaration) = body("if a then b else c + d");
    let spans = declaration.spans().expect("the form carries its spans");
    let at = |path: &[usize]| {
        let span = spans.body.at(path).expect("the path names a node");
        &text[span.start..span.end]
    };
    assert_eq!(at(&[]), "if a then b else c + d");
    assert_eq!(at(&[2]), "c + d");
    assert_eq!(at(&[2, 0]), "c");
    assert_eq!(at(&[2, 1]), "d");
    let (text, declaration) = body("(a + b) * c");
    let spans = declaration.spans().expect("the form carries its spans");
    let at = |path: &[usize]| {
        let span = spans.body.at(path).expect("the path names a node");
        &text[span.start..span.end]
    };
    assert_eq!(at(&[]), "(a + b) * c");
    assert_eq!(at(&[0]), "a + b");
    assert_eq!(at(&[1]), "c");
    let declaration_span = spans.declaration;
    assert!(text[declaration_span.start..declaration_span.end].starts_with("function f"));
}

/// TC-403 step 3: every `Expression` node's span lies inside its parent's.
#[trace("FR-091-AC-10", "TC-403")]
#[test]
fn every_expression_span_lies_inside_its_parents() {
    fn walk(
        spans: &qsl_forms::ExpressionSpans,
        path: &mut Vec<usize>,
        parent: (usize, usize),
        seen: &mut usize,
    ) {
        let mut index = 0;
        while let Some(span) = spans.at(&{
            let mut child = path.clone();
            child.push(index);
            child
        }) {
            assert!(
                span.start >= parent.0 && span.end <= parent.1,
                "child {path:?}/{index} {span:?} escapes its parent {parent:?}"
            );
            *seen += 1;
            path.push(index);
            walk(spans, path, (span.start, span.end), seen);
            path.pop();
            index += 1;
        }
    }
    let mut seen = 0;
    for source in ["if a then b else c + d", "(a + b) * c"] {
        let (_, declaration) = body(source);
        let spans = declaration.spans().expect("the form carries its spans");
        let root = spans.body.at(&[]).expect("the body has a span");
        walk(
            &spans.body,
            &mut Vec::new(),
            (root.start, root.end),
            &mut seen,
        );
    }
    assert_eq!(seen, 9, "every non-root node of both bodies carries a span");
}

#[trace("FR-091-AC-1")]
#[test]
fn record_and_tuple_forms_carry_their_names_and_members() {
    let (text, unit) = build(
        "record Point { x: Int[0, 9]; y: Option<Point>?; }\n\
         tuple Pair(Int[0, 9], Reference<M::T>);",
    );
    let DeclarationForm::Record(record) = unit.forms()[0].form() else {
        panic!("a record");
    };
    assert_eq!(record.name.name, "Point");
    assert_eq!(&text[record.name.span.start..record.name.span.end], "Point");
    assert_eq!(record.fields.len(), 2);
    assert!(!record.fields[0].optional);
    assert!(record.fields[1].optional);
    assert!(matches!(
        record.fields[1].type_form.head,
        TypeFormHead::Builtin(BuiltinType::Option)
    ));
    assert!(matches!(
        &record.fields[1].type_form.arguments[0].head,
        TypeFormHead::Name(name) if name == "Point"
    ));
    let DeclarationForm::Tuple(tuple) = unit.forms()[1].form() else {
        panic!("a tuple");
    };
    assert_eq!(tuple.name.name, "Pair");
    assert_eq!(tuple.elements.len(), 2);
    assert!(matches!(
        &tuple.elements[1].arguments[0].head,
        TypeFormHead::Name(name) if name == "M::T"
    ));
}

#[trace("FR-091-AC-23", "TC-405")]
#[test]
fn a_bare_float_type_is_admitted_and_builds_with_no_rounding_mode() {
    let (_, unit) = build(
        "function h using v(x: Float64, y: Float32[exact], z: Float32): Boolean pure { true }",
    );
    let h = function(unit.forms()[0].form());
    let [(_, x), (_, y), (_, z)] = h.parameters.as_slice() else {
        panic!("three parameters");
    };
    assert!(matches!(
        x.head,
        TypeFormHead::Builtin(BuiltinType::Float64)
    ));
    assert!(x.bounds.is_empty());
    assert!(matches!(
        y.head,
        TypeFormHead::Builtin(BuiltinType::Float32)
    ));
    assert_eq!(y.bounds, ["exact"]);
    assert!(matches!(
        z.head,
        TypeFormHead::Builtin(BuiltinType::Float32)
    ));
    assert!(z.bounds.is_empty());
}

/// TC-395 step 4: each declaration builds one form and is not refused with
/// `NoDispatchEntry`.
#[trace("FR-091-AC-6", "TC-395")]
#[test]
fn enum_predicate_dimension_and_unit_declarations_build_forms() {
    for declaration in [
        "enum Color { RED }",
        "ordered enum Level { LOW }",
        "predicate P using v(x: Boolean): Boolean { x }",
        "dimension Length;",
        "dimension Length;\nunit m : Length = rational(1, 1);",
    ] {
        let (_, unit) = build(declaration);
        assert_eq!(
            unit.forms().len(),
            declaration.lines().count(),
            "{declaration}"
        );
    }
}

/// TC-395 step 5: only the `Value` family form builder names the
/// declaration productions it owns.
#[trace("FR-091-AC-6", "TC-395")]
#[test]
fn only_the_value_builder_names_the_enum_dimension_unit_and_predicate_productions() {
    use syn::visit::Visit;

    struct Names(Vec<String>);
    impl<'ast> Visit<'ast> for Names {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            if let [head, production] = segments.as_slice() {
                if head == "Production" {
                    self.0.push(production.clone());
                }
            }
            syn::visit::visit_path(self, path);
        }
    }

    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let owned = [
        "EnumDeclaration",
        "EnumMember",
        "Predicate",
        "DimensionDeclaration",
        "UnitDeclaration",
    ];
    let mut owners = Vec::new();
    for entry in std::fs::read_dir(&src).expect("the crate's src directory") {
        let path = entry.expect("a directory entry").path();
        let source = std::fs::read_to_string(&path).expect("a source file");
        let file = syn::parse_file(&source).expect("the file parses");
        let mut names = Names(Vec::new());
        names.visit_file(&file);
        if names.0.iter().any(|name| owned.contains(&name.as_str())) {
            owners.push(path.file_name().expect("a file name").to_owned());
        }
    }
    assert_eq!(owners, [std::ffi::OsString::from("value.rs")]);
}

#[trace("FR-091-AC-25", "TC-480")]
#[test]
fn s2_builds_enum_forms_with_members_in_source_order_and_display_strings() {
    let (text, unit) = build("ordered enum Level { LOW, HIGH = \"High\", }");
    let [form] = unit.forms() else {
        panic!("one form")
    };
    let DeclarationForm::Enum(level) = form.form() else {
        panic!("an enum form");
    };
    assert_eq!(level.name.name, "Level");
    assert_eq!(level.name.span, span_of(&text, "Level"));
    assert!(level.ordered);
    let [low, high] = level.members.as_slice() else {
        panic!("two members")
    };
    assert_eq!(low.case.name, "LOW");
    assert_eq!(low.case.span, span_of(&text, "LOW"));
    assert_eq!(low.display, None);
    assert_eq!(high.case.name, "HIGH");
    assert_eq!(high.case.span, span_of(&text, "HIGH"));
    assert_eq!(
        high.display,
        Some(("\"High\"".to_owned(), span_of(&text, "\"High\"")))
    );

    let (_, unit) = build("enum Color { RED, BLUE }");
    let DeclarationForm::Enum(color) = unit.forms()[0].form() else {
        panic!("an enum form");
    };
    assert!(!color.ordered);
    let cases: Vec<&str> = color.members.iter().map(|m| m.case.name.as_str()).collect();
    assert_eq!(cases, ["RED", "BLUE"]);
}

#[trace("FR-091-AC-26", "TC-480")]
#[test]
fn s2_builds_a_predicate_as_a_function_declaration_of_kind_predicate() {
    let (text, unit) = build("predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }");
    let predicate = function(unit.forms()[0].form());
    assert_eq!(predicate.kind(), DeclarationKind::Predicate);
    assert_eq!(predicate.name, "Positive");
    let using = predicate.using().expect("a using alias");
    assert_eq!(using.alias, "v");
    let at = span_of(&text, "v(x").start;
    assert_eq!(
        using.span,
        Span {
            start: at,
            end: at + 1
        }
    );
    let [(parameter, form)] = predicate.parameters.as_slice() else {
        panic!("one parameter")
    };
    assert_eq!(parameter, "x");
    assert!(matches!(form.head, TypeFormHead::Builtin(BuiltinType::Int)));
    assert_eq!(form.bounds, ["0", "9"]);
    assert!(matches!(
        predicate.result.head,
        TypeFormHead::Builtin(BuiltinType::Boolean)
    ));
    assert_eq!(predicate.result.span, span_of(&text, "Boolean"));
    assert!(predicate.measure.is_none());
    assert_eq!(show(predicate.body.root()), "Greater(x, 0)");

    let (_, unit) = build("function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }");
    assert_eq!(
        function(unit.forms()[0].form()).kind(),
        DeclarationKind::Function
    );
}

#[trace("FR-091-AC-31", "TC-482")]
#[test]
fn s2_builds_dimension_and_unit_forms_as_written() {
    let (_, unit) = build("dimension Length;");
    let DeclarationForm::Dimension(length) = unit.forms()[0].form() else {
        panic!("a dimension form");
    };
    assert_eq!(length.name.name, "Length");
    assert!(length.terms.is_empty());

    let (text, unit) = build("dimension Accel = Length * Time^-2 / Mass;");
    let DeclarationForm::Dimension(accel) = unit.forms()[0].form() else {
        panic!("a dimension form");
    };
    let shown: Vec<_> = accel
        .terms
        .iter()
        .map(|term| (term.operator, term.name.name.as_str(), term.name.span))
        .collect();
    assert_eq!(
        shown,
        [
            (None, "Length", span_of(&text, "Length *")),
            (
                Some(TermOperator::Multiply),
                "Time",
                span_of(&text, "Time^")
            ),
            (Some(TermOperator::Divide), "Mass", span_of(&text, "Mass")),
        ]
        .map(|(operator, name, span)| (
            operator,
            name,
            Span {
                start: span.start,
                end: span.start + name.len()
            }
        ))
    );
    assert_eq!(accel.terms[0].exponent, None);
    assert_eq!(
        accel.terms[1].exponent,
        Some((Integer::from(-2_i64), span_of(&text, "-2")))
    );
    assert_eq!(accel.terms[2].exponent, None);

    let (text, unit) = build("unit C : Temperature = rational(1, 1) * K + decimal(27315, 2);");
    let DeclarationForm::Unit(celsius) = unit.forms()[0].form() else {
        panic!("a unit form");
    };
    assert_eq!(celsius.name.name, "C");
    let at = |needle: &str, skip: usize| {
        let start = span_of(&text, needle).start + skip;
        Span {
            start,
            end: start + 1,
        }
    };
    assert_eq!(celsius.name.span, at("unit C", 5));
    assert_eq!(celsius.dimension.name, "Temperature");
    assert_eq!(celsius.dimension.span, span_of(&text, "Temperature"));
    assert_eq!(celsius.scale.kind, ExactNumberKind::Rational);
    assert_eq!(celsius.scale.first, Integer::from(1_i64));
    assert_eq!(celsius.scale.second, Integer::from(1_i64));
    assert_eq!(celsius.scale.span, span_of(&text, "rational(1, 1)"));
    let target = celsius.target.as_ref().expect("a target");
    assert_eq!(target.name, "K");
    assert_eq!(target.span, at("* K", 2));
    let offset = celsius.offset.as_ref().expect("an offset");
    assert_eq!(offset.kind, ExactNumberKind::Decimal);
    assert_eq!(offset.first, Integer::from(27315_i64));
    assert_eq!(offset.second, Integer::from(2_i64));
    assert_eq!(offset.span, span_of(&text, "decimal(27315, 2)"));

    let (_, unit) = build("unit m : Length = rational(1, 1);");
    let DeclarationForm::Unit(metre) = unit.forms()[0].form() else {
        panic!("a unit form");
    };
    assert!(metre.target.is_none() && metre.offset.is_none());
}

/// TC-722's function bodies, each 100,000 levels deep, with the depth of
/// the form tree S2 builds for each: brackets group without a node of
/// their own, so the bracket nest's tree is its one literal.
fn deep_bodies() -> [(String, usize); 5] {
    const DEPTH: usize = 100_000;
    [
        (format!("{}1{}", "(".repeat(DEPTH), ")".repeat(DEPTH)), 1),
        (format!("{}a", "not ".repeat(DEPTH)), DEPTH + 1),
        (vec!["x"; DEPTH].join(" + "), DEPTH),
        (format!("{}x", "if a then x else ".repeat(DEPTH)), DEPTH + 1),
        (format!("{}x", "let v = x in ".repeat(DEPTH)), DEPTH + 1),
    ]
}

/// S2 builds each of TC-722's 100,000-deep function bodies on a 512 KiB
/// stack, parsed under S1 limits raised to fit it; the built unit clones,
/// compares equal to its clone, formats for debug and drops there too.
#[trace("TC-724", "FR-257-AC-1")]
#[test]
fn deep_bodies_build_clone_compare_format_and_drop_on_a_small_stack() {
    for (body, expected_depth) in deep_bodies() {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(move || {
                let text = format!(
                    "{HEADER}function f using v(a: Boolean, x: Integer): Integer pure {{ {body} }}\n"
                );
                let parsed = qsl_cst::parse(
                    SourceIdentity::new("a", "u", "git", "1"),
                    "unit.native",
                    text.as_bytes(),
                    qsl_cst::Limits::default()
                        .with_source_bytes(usize::MAX)
                        .with_tokens(usize::MAX)
                        .with_nodes(usize::MAX),
                )
                .expect("S1 reads the body under raised limits");
                assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
                let unit = build_unit(&parsed).expect("S2 builds the body");
                let built = function(unit.forms()[0].form());
                assert_eq!(depth(&built.body), expected_depth);
                let copy = unit.clone();
                assert_eq!(function(copy.forms()[0].form()).body, built.body);
                assert!(!format!("{unit:?}").is_empty());
                drop(copy);
                drop(unit);
            })
            .expect("spawn a 512 KiB thread")
            .join()
            .expect("S2 and the built unit's traits do not overflow a 512 KiB stack");
    }
}

/// A parameter type nested 100,000 `Option`s deep builds on a 512 KiB
/// stack under raised S1 limits, and its type form clones, compares equal
/// to its clone, formats for debug and drops there too.
#[trace("TC-724", "FR-257-AC-1")]
#[test]
fn a_deep_parameter_type_builds_clones_compares_formats_and_drops_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            const DEPTH: usize = 100_000;
            let text = format!(
                "{HEADER}function f using v(p: {}Boolean{}): Boolean pure {{ true }}\n",
                "Option<".repeat(DEPTH),
                ">".repeat(DEPTH)
            );
            let parsed = qsl_cst::parse(
                SourceIdentity::new("a", "u", "git", "1"),
                "unit.native",
                text.as_bytes(),
                qsl_cst::Limits::default()
                    .with_source_bytes(usize::MAX)
                    .with_tokens(usize::MAX)
                    .with_nodes(usize::MAX),
            )
            .expect("S1 reads the type under raised limits");
            assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
            let unit = build_unit(&parsed).expect("S2 builds the type");
            let parameter = &function(unit.forms()[0].form()).parameters[0].1;
            let mut levels = 0;
            let mut form = parameter;
            while let [argument] = form.arguments.as_slice() {
                levels += 1;
                form = argument;
            }
            assert_eq!(levels, DEPTH);
            assert_eq!(form.head, TypeFormHead::Builtin(BuiltinType::Boolean));
            let copy = parameter.clone();
            assert_eq!(&copy, parameter);
            let rendered = format!("{copy:?}");
            assert_eq!(rendered.matches("TypeForm {").count(), DEPTH + 1);
            drop(copy);
            drop(unit);
        })
        .expect("spawn a 512 KiB thread")
        .join()
        .expect("S2 and the type form's traits do not overflow a 512 KiB stack");
}
