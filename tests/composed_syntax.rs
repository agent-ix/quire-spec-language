// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-113: independent grammar scenarios through the public edition-selected API.

use ix_trace_rs::trace;
use quire_spec_language::syntax::composed::{
    Activation, ComposedUnit, ControlKind, DeclarationKind, EventKind, NativeUnit, ParameterType,
    ProductOp, ProtocolRequirement, QueryOp, TemporalKind, TemporalOp, ValueKind,
};
use quire_spec_language::syntax::{BinaryOp, ExprId, ExprKind};
use quire_spec_language::{
    format::format, parse, parse_native, parse_native_source, Code, Diagnostic, Limits, Phase,
    Source, SourceIdentity, Span,
};

const HEADER: &str = r#"language "ix:native" edition "1-draft";
profile S = "quire.state.graph/v1" version "test:state" digest "unresolved-state";
profile T = "quire.temporal.timestamped-event.finite-window/v1" version "test:temporal" digest "unresolved-temporal";
profile P = "quire.protocol.finite-global/v1" version "test:protocol" digest "unresolved-protocol";
model M = "test:orders-and-refunds" version "test:model" digest "unresolved-model";
"#;
const PREDICATE: &str =
    "predicate RefundMatches using S (v: M::OrderView): Boolean { v.refund = v.paid }";
const STATE: &str =
    "invariant OrderValid using S on M::OrderView at current { RefundMatches(self) }";
const TEMPORAL: &str = r#"temporal RefundDue using T over (view: M::OrderView) clock "order-clock" on origin {
  capture paid: M::Money = view.paid;
  eventually [0,5] holds(RefundMatches(view))
}"#;
const PROTOCOL: &str = r#"protocol Orders using P over (view: M::WorkflowView) on origin {
  role OrdersService on M::OrderService;
  role PaymentsService on M::PaymentService;
  channel Requests from OrdersService to PaymentsService carries M::ChargeRequest ordering unordered delivery [0,3];
  requires temporal RefundDue;
  run sequence Main {
    send Sent via Requests as (sent: M::SendView) { sent.amount = view.amount };
    receive Received via Requests of Sent as (received: M::ReceiptView) { received.amount = view.amount };
    check Paid using S { RefundMatches(view.order) };
  }
  finish Closed as (closed: M::ClosureView) { closed.complete };
}"#;

fn identity() -> SourceIdentity {
    SourceIdentity {
        identity: "test:composed-syntax".into(),
        revision: "test:authored".into(),
    }
}

fn read(text: &str, limits: Limits) -> Result<NativeUnit, Box<Diagnostic>> {
    parse_native(identity(), "composed.native", text.as_bytes(), limits)
}

fn unit(declarations: &str) -> ComposedUnit {
    let NativeUnit::Composed(unit) =
        read(&format!("{HEADER}{declarations}"), Limits::default()).unwrap()
    else {
        panic!("the composed selector must produce a composed unit");
    };
    unit
}

fn state(expression: &str) -> ComposedUnit {
    unit(&format!(
        "invariant Test using S on M::OrderView at current {{ {expression} }}"
    ))
}

fn body(unit: &ComposedUnit) -> ExprId {
    let DeclarationKind::State { body, .. } = unit.declarations()[0].kind else {
        panic!("expected state declaration");
    };
    body
}

fn binary(unit: &ComposedUnit, id: ExprId, expected: BinaryOp) -> (ExprId, ExprId) {
    let ValueKind::Shared(ExprKind::Binary { op, left, right }) = unit.expression(id).unwrap().kind
    else {
        panic!("expected binary value expression");
    };
    assert_eq!(op, expected);
    (left, right)
}

#[test]
#[trace("TC-113", "FR-035-AC-1")]
fn four_families_retain_selectors_trees_and_authored_spans() {
    let declarations = [PREDICATE, STATE, TEMPORAL, PROTOCOL];
    let parsed = unit(&declarations.join("\n"));
    assert_eq!(parsed.edition().value, "1-draft");
    assert_eq!(parsed.profiles().len(), 3);
    assert_eq!(parsed.models().len(), 1);
    assert_eq!(parsed.declarations().len(), 4);
    for (i, (name, profile)) in [
        ("RefundMatches", "S"),
        ("OrderValid", "S"),
        ("RefundDue", "T"),
        ("Orders", "P"),
    ]
    .into_iter()
    .enumerate()
    {
        let declaration = &parsed.declarations()[i];
        assert_eq!(
            (
                declaration.name.value.as_str(),
                declaration.profile.value.as_str()
            ),
            (name, profile)
        );
        assert_eq!(parsed.source().slice(declaration.name.span), Some(name));
        assert_eq!(
            parsed.source().slice(declaration.profile.span),
            Some(profile)
        );
        assert_eq!(
            parsed.source().slice(declaration.span),
            Some(declarations[i])
        );
    }
    let DeclarationKind::Predicate {
        parameters, body, ..
    } = &parsed.declarations()[0].kind
    else {
        panic!("predicate");
    };
    assert_eq!(parameters[0].name.value, "v");
    assert!(matches!(parameters[0].ty, ParameterType::Model(_)));
    binary(&parsed, *body, BinaryOp::Equal);
    let DeclarationKind::State { body, .. } = &parsed.declarations()[1].kind else {
        panic!("state");
    };
    let ValueKind::Invoke { name, arguments } = &parsed.expression(*body).unwrap().kind else {
        panic!("shared predicate reference");
    };
    assert_eq!(name.value, "RefundMatches");
    assert_eq!(arguments.len(), 1);
    assert!(matches!(
        parsed.expression(arguments[0]).unwrap().kind,
        ValueKind::Shared(ExprKind::SelfValue)
    ));
    let DeclarationKind::Temporal {
        formula,
        captures,
        clock,
        activation,
        ..
    } = &parsed.declarations()[2].kind
    else {
        panic!("temporal");
    };
    assert!(matches!(activation.as_ref(), Activation::Origin { .. }));
    assert_eq!(clock.value, "order-clock");
    assert_eq!(captures[0].parameter.name.value, "paid");
    let TemporalKind::Unary {
        op,
        interval,
        argument,
    } = &parsed.temporal(*formula).unwrap().kind
    else {
        panic!("eventually");
    };
    assert_eq!(op.value, TemporalOp::Eventually);
    let interval = interval.as_ref().unwrap();
    assert_eq!(
        (interval.lower.value.as_str(), interval.upper.value.as_str()),
        ("0", "5")
    );
    let TemporalKind::Holds(value) = parsed.temporal(*argument).unwrap().kind else {
        panic!("holds");
    };
    assert!(matches!(
        parsed.expression(value).unwrap().kind,
        ValueKind::Invoke { .. }
    ));
    let DeclarationKind::Protocol(protocol) = &parsed.declarations()[3].kind else {
        panic!("protocol");
    };
    assert_eq!(protocol.roles.len(), 2);
    assert_eq!(protocol.channels[0].name.value, "Requests");
    assert_eq!(protocol.finish.name.value, "Closed");
    let ControlKind::Sequence(children) = &parsed.control(protocol.run).unwrap().kind else {
        panic!("sequence");
    };
    assert_eq!(children.len(), 3);
    let ControlKind::Event(sent) = &parsed.control(children[0]).unwrap().kind else {
        panic!("send");
    };
    assert!(matches!(sent.kind, EventKind::Send { .. }));
    let ControlKind::Event(received) = &parsed.control(children[1]).unwrap().kind else {
        panic!("receive");
    };
    let EventKind::Receive { send, .. } = &received.kind else {
        panic!("receive correspondence");
    };
    assert_eq!(send.path[0].value, "Sent");
    let ControlKind::Check { expression, .. } = &parsed.control(children[2]).unwrap().kind else {
        panic!("check");
    };
    assert!(matches!(
        parsed.expression(*expression).unwrap().kind,
        ValueKind::Invoke { .. }
    ));
}

#[test]
#[trace("TC-113", "FR-035-AC-2")]
fn value_precedence_and_implication_associativity_match_the_grammar() {
    let parsed = state("a implies b or c and d = e + f * g");
    let (_, right) = binary(&parsed, body(&parsed), BinaryOp::Implies);
    let (_, right) = binary(&parsed, right, BinaryOp::Or);
    let (_, right) = binary(&parsed, right, BinaryOp::And);
    let (_, right) = binary(&parsed, right, BinaryOp::Equal);
    let (_, right) = binary(&parsed, right, BinaryOp::Add);
    binary(&parsed, right, BinaryOp::Multiply);
    let parsed = state("a implies b implies c");
    let (_, right) = binary(&parsed, body(&parsed), BinaryOp::Implies);
    binary(&parsed, right, BinaryOp::Implies);
    let parsed = state("a - b - c");
    let (left, _) = binary(&parsed, body(&parsed), BinaryOp::Subtract);
    binary(&parsed, left, BinaryOp::Subtract);
}

#[test]
#[trace("TC-113", "FR-035-AC-2")]
fn temporal_constants_holds_and_grouped_relations_have_distinct_trees() {
    let parsed = unit(
        r#"temporal Test using T over (view: M::OrderView) clock "c" on origin {
      true implies holds(true) implies (false until [0,2] holds(view.ok))
    }"#,
    );
    let DeclarationKind::Temporal { formula, .. } = parsed.declarations()[0].kind else {
        panic!("temporal");
    };
    let TemporalKind::Binary {
        op, left, right, ..
    } = &parsed.temporal(formula).unwrap().kind
    else {
        panic!("implication");
    };
    assert_eq!(op.value, TemporalOp::Implies);
    assert_eq!(
        parsed.temporal(*left).unwrap().kind,
        TemporalKind::Constant(true)
    );
    let TemporalKind::Binary {
        op, left, right, ..
    } = &parsed.temporal(*right).unwrap().kind
    else {
        panic!("right-associated implication");
    };
    assert_eq!(op.value, TemporalOp::Implies);
    let TemporalKind::Holds(value) = parsed.temporal(*left).unwrap().kind else {
        panic!("holds true");
    };
    assert_eq!(
        parsed.expression(value).unwrap().kind,
        ValueKind::Shared(ExprKind::Boolean(true))
    );
    let TemporalKind::Group(inner) = parsed.temporal(*right).unwrap().kind else {
        panic!("explicit group");
    };
    let TemporalKind::Binary { op, interval, .. } = &parsed.temporal(inner).unwrap().kind else {
        panic!("bounded relation");
    };
    assert_eq!(op.value, TemporalOp::Until);
    assert_eq!(interval.as_ref().unwrap().upper.value, "2");
}

#[test]
#[trace("TC-113", "FR-035-AC-2")]
fn malformed_family_bodies_and_trailing_content_never_return_a_unit() {
    let temporal = |formula: &str| {
        format!(
            r#"temporal Test using T over (view: M::OrderView) clock "c" on origin {{ {formula} }}"#
        )
    };
    let bad = [
        STATE.replace("RefundMatches(self)", "a < b < c"),
        STATE.replace("RefundMatches(self)", "RefundMatches(eventually [0,1] holds(true))"),
        temporal("true until [0,1] false since [0,1] true"),
        temporal("holds(always [0,1] true)"),
        PROTOCOL.replace("via Requests as", "via as"),
        PROTOCOL.replace("run sequence Main", "run sequence"),
        format!("{PROTOCOL} garbage"),
        "protocol P using P over (v: M::WorkflowView) on origin { role R on M::Service; compensate C for Main::E as (e: M::Effect) by R on M::Thing::refund using T clock \"c\" { activate first".into(),
    ];
    for declaration in bad {
        let text = format!("{HEADER}{declaration}");
        let error = read(&text, Limits::default()).unwrap_err();
        assert_eq!(error.code, Code::InvalidSyntax, "{declaration}: {error}");
        assert_eq!(error.source, identity());
        assert_eq!(error.path, "composed.native");
    }
    for declaration in [STATE, TEMPORAL, PROTOCOL] {
        assert_eq!(unit(declaration).declarations().len(), 1);
    }
}

#[test]
#[trace("TC-113", "FR-035-AC-3", "FR-035-AC-6")]
fn keyword_classification_is_edition_and_position_specific() {
    let parsed = unit("pre CanRetry using S on M::result::retry { self.retry = M::result::retry }");
    let DeclarationKind::State {
        context,
        operation,
        body,
        ..
    } = &parsed.declarations()[0].kind
    else {
        panic!("state");
    };
    assert_eq!(context.name.value, "result");
    assert_eq!(operation.as_ref().unwrap().value, "retry");
    let (left, right) = binary(&parsed, *body, BinaryOp::Equal);
    let ValueKind::Shared(ExprKind::Field { name, .. }) = &parsed.expression(left).unwrap().kind
    else {
        panic!("qualified field");
    };
    assert_eq!(name.value, "retry");
    let ValueKind::Shared(ExprKind::EnumValue { name, variant, .. }) =
        &parsed.expression(right).unwrap().kind
    else {
        panic!("qualified model members");
    };
    assert_eq!(
        (name.value.as_str(), variant.value.as_str()),
        ("result", "retry")
    );
    for reserved in ["result", "retry", "temporal", "compensate", "Boolean"] {
        let declaration = format!("predicate {reserved} using S (): Boolean {{ true }}");
        assert_eq!(
            read(&format!("{HEADER}{declaration}"), Limits::default())
                .unwrap_err()
                .code,
            Code::InvalidSyntax
        );
        let declaration =
            format!("predicate Valid using S ({reserved}: Boolean): Boolean {{ true }}");
        assert_eq!(
            read(&format!("{HEADER}{declaration}"), Limits::default())
                .unwrap_err()
                .code,
            Code::InvalidSyntax
        );
    }
    assert_eq!(
        unit("predicate retrying using S (): Boolean { true }").declarations()[0]
            .name
            .value,
        "retrying"
    );
    let historical = "language \"ix:native\" edition \"0-draft\"; profile \"state-finite/0-draft\"; model M = \"test:model\" version \"1\" digest \"unresolved\"; invariant Test on M::Thing at current { let retry = true in retry }";
    let NativeUnit::Historical(selected) = read(historical, Limits::default()).unwrap() else {
        panic!("historical edition");
    };
    let original = parse(
        identity(),
        "composed.native",
        historical.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(format(&selected).unwrap(), format(&original).unwrap());
    assert_eq!(selected.source().digest(), original.source().digest());
}

#[test]
#[trace("TC-113", "FR-035-AC-4")]
fn crlf_comments_escapes_and_multibyte_literals_preserve_original_coordinates() {
    let expression = r#""caf\u00e9😀\"" = "café😀\"""#;
    let declaration =
        format!("invariant Located using S on M::OrderView at current {{ {expression} }}");
    let text =
        format!("// café: // stays a comment\n{HEADER}{declaration}\n").replace('\n', "\r\n");
    let source = Source::read(
        identity(),
        "composed.native",
        text.as_bytes(),
        Limits::default().source_bytes,
    )
    .unwrap();
    let NativeUnit::Composed(parsed) = parse_native_source(source, Limits::default()).unwrap()
    else {
        panic!("composed");
    };
    assert_eq!(parsed.source().text().as_bytes(), text.as_bytes());
    let literal_nodes: Vec<_> = parsed
        .expressions()
        .iter()
        .filter_map(|expression| match &expression.kind {
            ValueKind::Shared(ExprKind::Text(value)) => Some((expression.span, value)),
            _ => None,
        })
        .collect();
    assert_eq!(literal_nodes.len(), 2);
    assert_eq!(literal_nodes[0].1, "café😀\"");
    assert_eq!(literal_nodes[1].1, "café😀\"");
    let raw = r#""caf\u00e9😀\"""#;
    let start = text.find(raw).unwrap();
    assert_eq!(
        literal_nodes[0].0,
        Span {
            start,
            end: start + raw.len()
        }
    );
    assert_eq!(parsed.source().slice(literal_nodes[0].0), Some(raw));
    let coordinates = parsed.source().locate(literal_nodes[0].0).unwrap();
    assert_eq!(coordinates.start.line, 7);
    assert_eq!(coordinates.start.byte, start);
    assert!(literal_nodes[0].0.end - literal_nodes[0].0.start > literal_nodes[0].1.len());
}

#[test]
#[trace("TC-113", "FR-035-AC-2", "FR-035-AC-6")]
fn unavailable_edition_is_located_without_falling_back_to_historical_syntax() {
    let text = format!("{HEADER}{STATE}").replacen("1-draft", "future-edition", 1);
    let error = read(&text, Limits::default()).unwrap_err();
    assert_eq!(error.code, Code::UnknownEdition);
    assert_eq!(error.phase, Phase::Profile);
    assert_eq!(
        &text[error.span.start.byte..error.span.end.byte],
        "\"future-edition\""
    );
    assert_eq!(
        parse(
            identity(),
            "composed.native",
            format!("{HEADER}{STATE}").as_bytes(),
            Limits::default()
        )
        .unwrap_err()
        .code,
        Code::UnknownEdition
    );
}

#[trace(
    "TC-113",
    "TC-132",
    "TC-133",
    "FR-035-AC-4",
    "FR-048-AC-2",
    "FR-048-AC-3"
)]
#[test]
fn negative_and_unbounded_choreography_counts_refuse_as_unsigned_syntax() {
    for text in [
        format!(
            "{HEADER}{}",
            PROTOCOL.replace("delivery [0,3]", "delivery [-1,3]")
        ),
        format!(
            "{HEADER}protocol InvalidRepeat using P over (view: M::OrderView) on origin {{
                role Service on M::OrderView;
                run repeat Loop by Service visible (true) max -1 while {{ true }}
                    check Body using S {{ true }};
                    exhausted check Exhausted using S {{ true }};
                finish Closed as (closed: M::OrderView) {{ true }};
            }}"
        ),
        format!(
            "{HEADER}protocol UnboundedRepeat using P over (view: M::OrderView) on origin {{
                role Service on M::OrderView;
                run repeat Loop by Service visible (true) max * while {{ true }}
                    check Body using S {{ true }};
                    exhausted check Exhausted using S {{ true }};
                finish Closed as (closed: M::OrderView) {{ true }};
            }}"
        ),
    ] {
        let error = read(&text, Limits::default()).expect_err("count must be an unsigned integer");
        assert_eq!(error.code, Code::InvalidSyntax);
        assert_eq!(error.phase, Phase::Parse);
    }
}

#[test]
#[trace("TC-113", "FR-035-AC-5")]
fn exact_small_limits_admit_and_each_next_charge_refuses() {
    // 25 authored tokens; one profile, one declaration and one Boolean node.
    // The existing value parser enters expression and precedence contexts.
    let text = "language \"ix:native\" edition \"1-draft\"; profile S = \"test:profile\" version \"1\" digest \"unresolved\"; predicate P using S(): Boolean { true }";
    let exact = Limits {
        source_bytes: text.len(),
        tokens: 25,
        nodes: 3,
        nesting: 2,
    };
    let NativeUnit::Composed(parsed) = read(text, exact).unwrap() else {
        panic!("composed");
    };
    assert_eq!(parsed.declarations().len(), 1);
    assert_eq!(parsed.expressions().len(), 1);
    for (limits, phase) in [
        (
            Limits {
                source_bytes: text.len() - 1,
                ..exact
            },
            Phase::Source,
        ),
        (
            Limits {
                tokens: 24,
                ..exact
            },
            Phase::Lex,
        ),
        (Limits { nodes: 2, ..exact }, Phase::Parse),
        (
            Limits {
                nesting: 1,
                ..exact
            },
            Phase::Parse,
        ),
    ] {
        let error = read(text, limits).unwrap_err();
        assert_eq!(error.code, Code::ResourceExhausted);
        assert_eq!(error.phase, phase);
    }
    let unary = text.replace("{ true }", "{ not true }");
    assert_eq!(
        read(
            &unary,
            Limits {
                nodes: 3,
                ..Limits::default()
            }
        )
        .unwrap_err()
        .code,
        Code::ResourceExhausted
    );
    let NativeUnit::Composed(parsed) = read(
        &unary,
        Limits {
            nodes: 4,
            ..Limits::default()
        },
    )
    .unwrap() else {
        panic!("composed");
    };
    assert_eq!(parsed.expressions().len(), 2);
}

#[test]
#[trace("TC-113", "FR-035-AC-1", "FR-035-AC-2")]
fn shared_queries_and_exact_numeric_syntax_have_typed_nodes() {
    for (expression, expected) in [
        ("filter(x in self.items: x.ok)", QueryOp::Filter),
        ("map(x in self.items: x.amount)", QueryOp::Map),
        ("count<M::Count>(x in self.items: x.ok)", QueryOp::Count),
        ("sum<M::Money>(x in self.items: x.amount)", QueryOp::Sum),
    ] {
        let parsed = state(expression);
        let ValueKind::Query {
            op,
            result,
            binder,
            domain,
            body: query_body,
        } = &parsed.expression(body(&parsed)).unwrap().kind
        else {
            panic!("query");
        };
        assert_eq!(op.value, expected);
        assert_eq!(binder.value, "x");
        assert_eq!(
            result.is_some(),
            matches!(expected, QueryOp::Count | QueryOp::Sum)
        );
        assert_eq!(
            parsed
                .source()
                .slice(parsed.expression(*domain).unwrap().span),
            Some("self.items")
        );
        assert!(matches!(
            parsed.expression(*query_body).unwrap().kind,
            ValueKind::Shared(ExprKind::Field { .. })
        ));
    }
    let parsed = state("size<M::Count>(self.items)");
    let ValueKind::Size { domain, argument } = &parsed.expression(body(&parsed)).unwrap().kind
    else {
        panic!("typed size");
    };
    assert_eq!(domain.name.value, "Count");
    assert_eq!(
        parsed
            .source()
            .slice(parsed.expression(*argument).unwrap().span),
        Some("self.items")
    );
    let parsed = state("contains(self.items, candidate)");
    let ValueKind::Contains { collection, member } = parsed.expression(body(&parsed)).unwrap().kind
    else {
        panic!("contains");
    };
    assert_eq!(
        parsed
            .source()
            .slice(parsed.expression(collection).unwrap().span),
        Some("self.items")
    );
    assert_eq!(
        parsed
            .source()
            .slice(parsed.expression(member).unwrap().span),
        Some("candidate")
    );
    let parsed = state("rational(-2,4)");
    let ValueKind::Rational {
        numerator,
        denominator,
    } = &parsed.expression(body(&parsed)).unwrap().kind
    else {
        panic!("rational");
    };
    assert_eq!(
        (numerator.value.as_str(), denominator.value.as_str()),
        ("-2", "4")
    );
    assert_eq!(parsed.source().slice(numerator.span), Some("-2"));
    let parsed = state("a / b mod c * d");
    let (left, _) = binary(&parsed, body(&parsed), BinaryOp::Multiply);
    let ValueKind::Product { op, left, .. } = &parsed.expression(left).unwrap().kind else {
        panic!("mod");
    };
    assert_eq!(op.value, ProductOp::Mod);
    assert_eq!(parsed.source().slice(op.span), Some("mod"));
    let ValueKind::Product { op, .. } = &parsed.expression(*left).unwrap().kind else {
        panic!("slash");
    };
    assert_eq!(op.value, ProductOp::Slash);
    assert_eq!(parsed.source().slice(op.span), Some("/"));
}

#[test]
#[trace("TC-113", "FR-035-AC-1", "FR-035-AC-2")]
fn protocol_controls_and_compensation_preserve_their_distinct_operands() {
    let text = include_str!("fixtures/composed-choreography.native");
    let NativeUnit::Composed(parsed) = read(text, Limits::default()).unwrap() else {
        panic!("composed");
    };
    let DeclarationKind::Protocol(protocol) = &parsed.declarations()[0].kind else {
        panic!("protocol");
    };
    let Activation::Each { trigger, guard, .. } = &protocol.activation else {
        panic!("each activation");
    };
    assert_eq!(trigger.name.value, "started");
    assert_eq!(
        parsed
            .source()
            .slice(parsed.expression(guard.unwrap()).unwrap().span),
        Some("started.ready")
    );
    assert_eq!(protocol.captures[0].parameter.name.value, "requested");
    assert_eq!(protocol.relationships[0].model.name.value, "OrderPayment");
    assert_eq!(protocol.requirements.len(), 2);
    let ProtocolRequirement::Compensation(compensation) = &protocol.requirements[1] else {
        panic!("compensation");
    };
    assert_eq!(
        compensation
            .effect
            .path
            .iter()
            .map(|name| name.value.as_str())
            .collect::<Vec<_>>(),
        ["Main", "Applied"]
    );
    assert_eq!(compensation.operation.name.value, "refund");
    assert_eq!(
        compensation.registration_captures[0].parameter.name.value,
        "saved"
    );
    assert_eq!(
        compensation.activation_captures[0].parameter.name.value,
        "expected"
    );
    assert_eq!(compensation.trigger.name.value, "trigger");
    assert_eq!(compensation.attempts.value, "2");
    assert_eq!(
        (
            compensation.earlier.name.value.as_str(),
            compensation.later.name.value.as_str()
        ),
        ("older", "newer")
    );
    assert_eq!(compensation.recovery.name.value, "recovery");
    assert_eq!(
        parsed
            .source()
            .slice(parsed.expression(compensation.retry).unwrap().span),
        Some("newer.previous = older.id")
    );
    assert_eq!(
        parsed
            .source()
            .slice(parsed.expression(compensation.recover).unwrap().span),
        Some("recovery.remaining = expected")
    );
    let ControlKind::Sequence(children) = &parsed.control(protocol.run).unwrap().kind else {
        panic!("root sequence");
    };
    let names: Vec<_> = children
        .iter()
        .map(|id| parsed.control(*id).unwrap().name.value.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "Sent",
            "Received",
            "Charged",
            "Applied",
            "Failed",
            "Choose",
            "Both",
            "Retry",
            "Refund",
            "Committed"
        ]
    );
    let ControlKind::Event(sent) = &parsed.control(children[0]).unwrap().kind else {
        panic!("send");
    };
    assert_eq!(sent.related[0].relationship.value, "Owner");
    assert_eq!(
        parsed
            .source()
            .slice(parsed.expression(sent.related[0].from).unwrap().span),
        Some("view.order")
    );
    assert_eq!(
        parsed
            .source()
            .slice(parsed.expression(sent.related[0].to).unwrap().span),
        Some("sent.payment")
    );
    let ControlKind::Event(attempt) = &parsed.control(children[2]).unwrap().kind else {
        panic!("attempt");
    };
    let EventKind::Attempt {
        operation,
        contracts,
        ..
    } = &attempt.kind
    else {
        panic!("attempt operands");
    };
    assert_eq!(operation.name.value, "charge");
    assert_eq!(
        contracts
            .iter()
            .map(|name| name.value.as_str())
            .collect::<Vec<_>>(),
        ["CanCharge", "ChargedCorrectly"]
    );
    let ControlKind::Event(effect) = &parsed.control(children[3]).unwrap().kind else {
        panic!("effect");
    };
    let EventKind::Effect { attempt } = &effect.kind else {
        panic!("effect correspondence");
    };
    assert_eq!(attempt.path[0].value, "Charged");
    let ControlKind::Choice {
        role,
        visible,
        cases,
    } = &parsed.control(children[5]).unwrap().kind
    else {
        panic!("choice");
    };
    assert_eq!(role.value, "OrdersService");
    assert_eq!(visible.len(), 1);
    assert_eq!(
        cases
            .iter()
            .map(|case| case.name.value.as_str())
            .collect::<Vec<_>>(),
        ["yes", "no"]
    );
    let ControlKind::Parallel { branches, join } = &parsed.control(children[6]).unwrap().kind
    else {
        panic!("parallel");
    };
    assert_eq!(
        branches
            .iter()
            .map(|branch| branch.name.value.as_str())
            .collect::<Vec<_>>(),
        ["left", "right"]
    );
    assert_eq!(
        join.iter()
            .map(|name| name.value.as_str())
            .collect::<Vec<_>>(),
        ["left", "right"]
    );
    let ControlKind::Repeat {
        maximum,
        body,
        exhausted,
        ..
    } = &parsed.control(children[7]).unwrap().kind
    else {
        panic!("repeat");
    };
    assert_eq!(maximum.value, "2");
    assert_eq!(parsed.control(*body).unwrap().name.value, "Again");
    assert_eq!(
        parsed.control(*exhausted).unwrap().name.value,
        "LimitReached"
    );
    let ControlKind::Await {
        after,
        clock,
        within,
        event,
        then,
        timeout,
        ..
    } = &parsed.control(children[8]).unwrap().kind
    else {
        panic!("await");
    };
    assert_eq!(after.path[0].value, "Applied");
    assert_eq!(clock.value, "payment-clock");
    assert_eq!(within.upper.value, "5");
    assert_eq!(parsed.control(*event).unwrap().name.value, "Refunded");
    assert_eq!(parsed.control(*then).unwrap().name.value, "GotRefund");
    assert_eq!(parsed.control(*timeout).unwrap().name.value, "Expired");
    assert!(matches!(
        parsed.control(children[9]).unwrap().kind,
        ControlKind::Commit { .. }
    ));
    assert_eq!(protocol.finish.parameter.name.value, "closed");
}

#[test]
#[trace("TC-113", "FR-035-AC-2")]
fn balanced_but_incomplete_protocol_forms_are_refused_by_the_grammar() {
    let valid = include_str!("fixtures/composed-choreography.native");
    for omitted in [
        "attempts 2 of M::AttemptView;",
        "recover (recovery: M::RecoveryView) { recovery.remaining = expected };",
        "case no when { not view.confirmed } sequence NoBody {}",
        "branch right sequence Right { check Recorded using S { view.recorded }; }",
        "join all [left,right];",
        "exhausted sequence LimitReached {}",
        "timeout sequence Expired {}",
    ] {
        assert!(
            valid.contains(omitted),
            "the negative control must alter its input"
        );
        let malformed = valid.replacen(omitted, "", 1);
        let error = read(&malformed, Limits::default()).unwrap_err();
        assert_eq!(
            error.code,
            Code::InvalidSyntax,
            "omitting {omitted}: {error}"
        );
        assert_eq!(error.phase, Phase::Parse);
    }
    let NativeUnit::Composed(parsed) = read(valid, Limits::default()).unwrap() else {
        panic!("positive counterpart");
    };
    assert_eq!(parsed.declarations().len(), 1);
}

#[test]
#[trace("TC-113", "FR-035-AC-4")]
fn operator_locations_come_from_the_original_tokens_across_comments() {
    let parsed = state("not // not and café\n self.ok and false");
    let root = parsed.expression(body(&parsed)).unwrap();
    assert_eq!(
        parsed.source().slice(root.operator_span.unwrap()),
        Some("and")
    );
    let (left, _) = binary(&parsed, body(&parsed), BinaryOp::And);
    let operand = parsed.expression(left).unwrap();
    assert_eq!(
        parsed.source().slice(operand.operator_span.unwrap()),
        Some("not")
    );
    assert_eq!(
        parsed.source().slice(operand.span),
        Some("not // not and café\n self.ok")
    );
    let parsed = state("present(self.optional)");
    assert_eq!(
        parsed.source().slice(
            parsed
                .expression(body(&parsed))
                .unwrap()
                .operator_span
                .unwrap()
        ),
        Some("present")
    );
}

#[test]
#[trace("TC-113", "FR-035-AC-1", "FR-035-AC-2")]
fn future_past_unaries_and_relations_keep_their_selected_operator_and_interval() {
    for (word, expected) in [
        ("eventually", TemporalOp::Eventually),
        ("always", TemporalOp::Always),
        ("once", TemporalOp::Once),
        ("historically", TemporalOp::Historically),
    ] {
        let parsed = unit(&format!(
            r#"temporal Test using T over (v: M::View) clock "c" on origin {{ {word} [2,7] holds(v.ok) }}"#
        ));
        let DeclarationKind::Temporal { formula, .. } = parsed.declarations()[0].kind else {
            panic!("temporal");
        };
        let TemporalKind::Unary {
            op,
            interval,
            argument,
        } = &parsed.temporal(formula).unwrap().kind
        else {
            panic!("temporal unary");
        };
        assert_eq!(op.value, expected);
        assert_eq!(parsed.source().slice(op.span), Some(word));
        let interval = interval.as_ref().unwrap();
        assert_eq!(
            (interval.lower.value.as_str(), interval.upper.value.as_str()),
            ("2", "7")
        );
        assert!(matches!(
            parsed.temporal(*argument).unwrap().kind,
            TemporalKind::Holds(_)
        ));
    }
    for (word, expected) in [
        ("until", TemporalOp::Until),
        ("release", TemporalOp::Release),
        ("since", TemporalOp::Since),
        ("triggered", TemporalOp::Triggered),
    ] {
        let parsed = unit(&format!(
            r#"temporal Test using T over (v: M::View) clock "c" on origin {{ holds(v.left) {word} [2,7] holds(v.right) }}"#
        ));
        let DeclarationKind::Temporal { formula, .. } = parsed.declarations()[0].kind else {
            panic!("temporal");
        };
        let TemporalKind::Binary {
            op,
            interval,
            left,
            right,
        } = &parsed.temporal(formula).unwrap().kind
        else {
            panic!("temporal relation");
        };
        assert_eq!(op.value, expected);
        assert_eq!(parsed.source().slice(op.span), Some(word));
        assert_eq!(interval.as_ref().unwrap().upper.value, "7");
        for (id, expected) in [(*left, "v.left"), (*right, "v.right")] {
            let TemporalKind::Holds(value) = parsed.temporal(id).unwrap().kind else {
                panic!("holds operand");
            };
            assert_eq!(
                parsed
                    .source()
                    .slice(parsed.expression(value).unwrap().span),
                Some(expected)
            );
        }
    }
}
