// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-091: the `Value` family's S2 production, run on real complete-V1
//! source through `qsl_cst::parse` and `build_unit`.

use ix_trace_rs::trace;
use qsl_cst::{CstElement, LosslessCst, ParsedSource, Production};
use qsl_forms::{
    build_unit, Accumulation, BinaryOperator, BinderQuery, BuiltinType, DeclarationForm,
    Expression, FieldInitializer, FormsCause, FormsFailure, FormsLimits, FunctionDeclaration,
    ParsedUnit, TypeFormHead,
};
use qsl_foundation::diagnostic::{LimitKind, Locus};
use qsl_foundation::{Code, SourceIdentity, Span};
use quire_exact::{CollectionKind, Integer};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\" version \"1\" digest \
    \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

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
    let unit = build_unit(&parsed, FormsLimits::default())
        .unwrap_or_else(|failure| panic!("{declarations}: {failure:?}"));
    (text, unit)
}

fn function(form: &DeclarationForm) -> &FunctionDeclaration {
    match form {
        DeclarationForm::Function(function) => function,
        DeclarationForm::Alias(_) | DeclarationForm::Record(_) | DeclarationForm::Tuple(_) => {
            panic!("a function form, not {form:?}")
        }
    }
}

/// The body of `function f using v(): Boolean pure { <body> }`.
fn body(body: &str) -> (String, FunctionDeclaration) {
    let (text, unit) = build(&format!("function f using v(): Boolean pure {{ {body} }}"));
    (text, function(unit.forms()[0].form()).clone())
}

fn refusal(declarations: &str) -> (String, FormsCause, Option<Span>) {
    let (text, parsed) = admissible(declarations);
    match build_unit(&parsed, FormsLimits::default()) {
        Err(FormsFailure::Refused(refusal)) => (text, refusal.cause, refusal.span),
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
fn show(expression: &Expression) -> String {
    let children: Vec<String> = expression.children().into_iter().map(show).collect();
    let head = match expression {
        Expression::Boolean(value) => format!("{value}"),
        Expression::Integer(value) => format!("{value}"),
        Expression::Rational(n, d) => format!("rational({n},{d})"),
        Expression::Name(spelling) => spelling.clone(),
        Expression::Let { name, .. } => format!("Let[{name}]"),
        Expression::If { .. } => "If".into(),
        Expression::Binary { operator, .. } => format!("{operator:?}"),
        Expression::Negate(_) => "Negate".into(),
        Expression::Not(_) => "Not".into(),
        Expression::Field { field, .. } => format!("Field[{field}]"),
        Expression::Present(_) => "Present".into(),
        Expression::Value(_) => "Value".into(),
        Expression::Deref(_) => "Deref".into(),
        Expression::Call { name, .. } => format!("Call[{name}]"),
        Expression::Record { name, fields } => {
            let fields: Vec<String> = fields
                .iter()
                .map(|(field, initializer)| match initializer {
                    FieldInitializer::Value(_) => field.clone(),
                    FieldInitializer::Null => format!("{field}=null"),
                })
                .collect();
            format!("Record[{name};{}]", fields.join(","))
        }
        Expression::Collection { kind, .. } => format!("Collection[{kind:?}]"),
        Expression::Convert { target, .. } => format!("Convert[{:?}]", target.head),
        Expression::Query { query, binder, .. } => format!("Query[{query:?},{binder}]"),
        Expression::Flatten(_) => "Flatten".into(),
        Expression::Accumulate {
            form,
            accumulator_type,
            accumulator,
            binder,
            ..
        } => format!("Accumulate[{form:?},{accumulator_type},{accumulator},{binder}]"),
        Expression::Count {
            result_type,
            binder,
            ..
        } => format!("Count[{result_type},{binder}]"),
        Expression::Sum {
            result_type,
            binder,
            ..
        } => format!("Sum[{result_type},{binder}]"),
        Expression::Size(_) => "Size".into(),
        Expression::Contains { .. } => "Contains".into(),
        Expression::AllInstances { target, .. } => format!("AllInstances[{:?}]", target.head),
        Expression::Lookup { .. } => "Lookup".into(),
        Expression::Dispatch { .. } => "Dispatch".into(),
        Expression::Pre(_) => "Pre".into(),
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
    let unit = build_unit(&parsed, FormsLimits::default()).expect("the unit builds");
    let kinds: Vec<&str> = unit
        .forms()
        .iter()
        .map(|form| match form.form() {
            DeclarationForm::Alias(_) => "alias",
            DeclarationForm::Function(_) => "function",
            DeclarationForm::Record(_) => "record",
            DeclarationForm::Tuple(_) => "tuple",
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
    assert_eq!(profile.definition.identity(), "quire.value.complete/v1");
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
    assert!(matches!(&inc.measure, Some(Expression::Name(name)) if name == "x"));
    assert_eq!(show(&inc.body), "Add(x, 1)");
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
    ];
    for (source, expected) in cases {
        let (_, declaration) = body(source);
        assert_eq!(show(&declaration.body), expected, "{source}");
    }
    let (_, declaration) = body("R { f: a, g: null }");
    let Expression::Record { fields, .. } = &declaration.body else {
        panic!("a record value");
    };
    assert!(matches!(fields[1], (ref g, FieldInitializer::Null) if g == "g"));
    let (_, declaration) = body("rational(1, -2)");
    assert!(matches!(
        &declaration.body,
        Expression::Rational(n, d) if *n == Integer::from(1_i64) && *d == Integer::from(-2_i64)
    ));
    let (_, declaration) = body("sequence[a]");
    assert!(matches!(
        &declaration.body,
        Expression::Collection {
            kind: CollectionKind::Sequence,
            ..
        }
    ));
    let (_, declaration) = body("fold<A>(acc, x in c: a)");
    assert!(matches!(
        &declaration.body,
        Expression::Accumulate {
            form: Accumulation::Fold,
            identity: None,
            ..
        }
    ));
    let (_, declaration) = body("map(x in c: x)");
    assert!(matches!(
        &declaration.body,
        Expression::Query {
            query: BinderQuery::Map,
            ..
        }
    ));
    let (_, declaration) = body("a + b");
    assert!(matches!(
        &declaration.body,
        Expression::Binary {
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
        assert_eq!(show(&declaration.body), expected, "{source}");
    }
}

#[trace("FR-091-AC-4", "FR-091-AC-5", "FR-091-AC-6", "TC-395")]
#[test]
fn s2_refuses_inadmissible_input_and_undispatched_declarations() {
    let (_, recovering) = parse("function f using v(): Boolean pure { true ");
    assert!(!recovering.cst().recoveries().is_empty());
    match build_unit(&recovering, FormsLimits::default()) {
        Err(FormsFailure::Refused(refusal)) => {
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
    match build_unit(&diagnosed, FormsLimits::default()) {
        Err(FormsFailure::Refused(refusal)) => {
            assert_eq!(
                refusal.cause,
                FormsCause::DiagnosedSource(Code::UnknownProfile)
            );
            assert_eq!(refusal.cause.catalog_code(), Code::UnknownProfile);
        }
        other => panic!("the diagnosed-source cause, not {other:?}"),
    }

    let invariant = "invariant Positive using v on M::T at current { true }";
    let (text, cause, span) = refusal(&format!(
        "function t using v(): Boolean pure {{ true }}\n{invariant}"
    ));
    assert_eq!(
        cause,
        FormsCause::NoDispatchEntry {
            spelling: name("invariant")
        }
    );
    assert_eq!(span, Some(span_of(&text, invariant)));

    for (declaration, leading) in [
        ("enum Color { red, green }", "enum"),
        ("ordered enum Size { small, large }", "ordered"),
        ("predicate p using v(): Boolean { true }", "predicate"),
        ("dimension Length;", "dimension"),
        ("unit metre: Length = rational(1, 1);", "unit"),
    ] {
        let (text, cause, span) = refusal(declaration);
        assert_eq!(
            cause,
            FormsCause::NoDispatchEntry {
                spelling: name(leading)
            },
            "{declaration}"
        );
        assert_eq!(cause.catalog_code(), Code::UnsupportedConstruct);
        assert_eq!(span, Some(span_of(&text, declaration)), "{declaration}");
    }
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
        (
            "reaches(a, b, M::R)",
            Production::Primary,
            "reaches(a, b, M::R)",
        ),
        ("if a then b else a mod b", Production::Product, "a mod b"),
        (
            "size<Int[0, 1]>(c)",
            Production::Primary,
            "size<Int[0, 1]>(c)",
        ),
        ("a div b", Production::Product, "a div b"),
        ("null", Production::Primary, "null"),
        ("self", Production::Primary, "self"),
        ("result", Production::Primary, "result"),
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
        assert_eq!(show(&function(unit.forms()[0].form()).body), expected);
    }
    let (_, unit) =
        build("function g using v(x: Int[0, 9]): Boolean pure decreases(pre(x)) { true }");
    let measure = function(unit.forms()[0].form())
        .measure
        .as_ref()
        .expect("the measure is carried");
    assert_eq!(show(measure), "Pre(x)");
}

fn nots(count: usize) -> String {
    format!("{}a", "not ".repeat(count))
}

#[trace("FR-091-AC-9", "TC-397")]
#[test]
fn the_nesting_depth_bound_refuses_past_its_limit() {
    let limits = FormsLimits { nesting_depth: 8 };
    let (_, seven) = admissible(&format!(
        "function f using v(a: Boolean): Boolean pure {{ {} }}",
        nots(7)
    ));
    build_unit(&seven, limits).expect("depth 8 builds");
    for count in [8, 20] {
        let (text, parsed) = admissible(&format!(
            "function f using v(a: Boolean): Boolean pure {{ {} }}",
            nots(count)
        ));
        match build_unit(&parsed, limits) {
            Err(FormsFailure::Limit { limit, span }) => {
                assert_eq!(limit.kind(), LimitKind::NestingDepth);
                assert_eq!(limit.configured_bound(), 8);
                // The node at depth 9: the ninth `not` from the left, or
                // the `a` under eight of them.
                let expected = if count == 8 {
                    span_of(&text, "a }").start_and(1)
                } else {
                    let start = text.find("not").expect("a not") + 4 * 8;
                    Span {
                        start,
                        end: span_of(&text, "a }").start + 1,
                    }
                };
                assert_eq!(span, expected, "{count}");
            }
            other => panic!("{count}: a depth limit, not {other:?}"),
        }
    }
}

/// FR-096-AC-3: with S2 nesting-depth bound 8, a body of eight `not`s over
/// `a` stops at depth 9 with the configured bound, the actual depth and
/// `Locus::Region` over the span of `a` (the node at depth 9) under the
/// unit's `RawSourceRef`.
#[trace("FR-096-AC-3", "TC-427")]
#[test]
fn the_s2_depth_limit_is_located_at_the_first_node_past_the_bound() {
    let (text, parsed) = admissible(&format!(
        "function f using v(a: Boolean): Boolean pure {{ {} }}",
        nots(8)
    ));
    let Err(FormsFailure::Limit { limit, .. }) =
        build_unit(&parsed, FormsLimits { nesting_depth: 8 })
    else {
        panic!("depth 9 is past the bound");
    };
    assert_eq!(limit.kind(), LimitKind::NestingDepth);
    assert_eq!(limit.configured_bound(), 8);
    assert_eq!(limit.actual(), 9);
    let a = u64::try_from(span_of(&text, "a }").start).unwrap();
    let Some(Locus::Region(region)) = limit.locus() else {
        panic!("an S2 limit carries its region, got {:?}", limit.locus());
    };
    assert_eq!(region.source(), parsed.source().reference());
    assert_eq!((region.start(), region.end()), (a, a + 1));
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

#[trace("FR-091-AC-21", "TC-406")]
#[test]
fn the_nesting_depth_limit_reports_stage_limit_exceeded() {
    let (_, parsed) = admissible("function f using v(a: Boolean): Boolean pure { not a }");
    let failure = build_unit(&parsed, FormsLimits { nesting_depth: 1 })
        .expect_err("depth 2 is past the bound");
    assert_eq!(failure.catalog_code(), Code::StageLimitExceeded);
    let FormsFailure::Limit { limit, .. } = failure else {
        panic!("a limit");
    };
    use qsl_foundation::diagnostic::CatalogCoded as _;
    assert_eq!(
        limit.catalog_code().to_string(),
        "stage_limit_exceeded/nesting-depth-exceeded"
    );
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
    let (_, unit) =
        build("function h using v(x: Float64, y: Float32[exact]): Boolean pure { true }");
    let h = function(unit.forms()[0].form());
    let [(_, x), (_, y)] = h.parameters.as_slice() else {
        panic!("two parameters");
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
}
