//! FR-218 (TC-663, TC-1056, TC-1065) over fixture R of the protocol
//! obligation tests, whose object, record, reference and event types these
//! criteria read: a channel's `fifo` ordering, its binder and its key, and
//! the model-state rule of guards, `visible` entries and keys.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::{Setting, SourceIdentity};
use quire_exact::ValueType;
use quire_semantic_value::checking::CheckingLimits;

use super::CheckedOrdering;
use crate::check::refusal::ProtocolAnchorCause;
use crate::check::{CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations};

mod fixture {
    use crate as semantics;
    include!("../../../tests/fixtures/protocol_obligations.rs");
}
use fixture::{obligation_model, HEADER};

/// A protocol whose one channel `Q` carries `carries`, ordered `ordering`.
fn with_channel(carries: &str, ordering: &str) -> String {
    format!(
        "{HEADER}protocol Queue using v over (view: M::Node) on origin {{\n\
         role Service on M::Node;\n\
         channel Q from Service to Service carries {carries} ordering {ordering} \
         delivery unknown capacity 1 overflow reject;\n\
         run sequence Main {{\n\
         attempt Tried by Service on M::Node::step contracts [] as (attempted: M::Flag) {{ true }};\n\
         }}\n\
         finish Closed as (closed: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

fn check(text: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    check_with_limits(text, CheckingLimits::default())
}

fn check_with_limits(
    text: &str,
    limits: CheckingLimits,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        vec![obligation_model()],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: the assembler refuses: {refusal:?}"))
    .check(limits)
}

/// The one channel's checked ordering.
fn ordering(text: &str) -> CheckedOrdering {
    let graph = check(text).unwrap_or_else(|refusals| panic!("{text}: {refusals:?}"));
    let [channel] = graph.protocols()[0].channels.as_slice() else {
        panic!("one channel");
    };
    channel.ordering.clone()
}

/// The one refusal's cause, its catalog pair and the text at its span.
fn only(
    text: &str,
) -> (
    ProtocolAnchorCause,
    (&'static str, Option<&'static str>),
    &str,
) {
    let refusals = check(text).map(|_| ()).expect_err("the protocol refuses");
    let [refusal] = refusals.as_slice() else {
        panic!("{text}: one refusal, got {refusals:?}");
    };
    let CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
        panic!("a protocol cause, got {refusal:?}");
    };
    let span = cause.span();
    (
        (**cause).clone(),
        (refusal.cause.code().as_str(), refusal.cause.cause()),
        &text[span.start..span.end],
    )
}

/// FR-218-AC-12: `ordering fifo by (m: T) {m.ready}` on a channel carrying
/// `T` checks and the checked channel holds `Fifo` with that binder and key;
/// a binder of another type refuses `ill_typed`/`type-mismatch` at the type;
/// a key that reads model state refuses `ill_typed`/`operator-ineligible` at
/// the read; `ordering unordered` holds no key.
#[trace("TC-1056", "FR-218-AC-12")]
#[test]
fn a_fifo_key_is_checked_over_its_binder() {
    let CheckedOrdering::Fifo { binder, key } =
        ordering(&with_channel("M::Flag", "fifo by (m: M::Flag) { m.ready }"))
    else {
        panic!("a Fifo ordering");
    };
    assert_eq!(binder.name, "m");
    assert_eq!(key.root().value_type(), &ValueType::Boolean);
    assert_eq!(
        ordering(&with_channel("M::Flag", "unordered")),
        CheckedOrdering::Unordered
    );

    let text = with_channel("M::Flag", "fifo by (m: M::Tick) { m.ready }");
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("ill_typed", Some("type-mismatch")));
    let ProtocolAnchorCause::CarriesMismatch {
        channel,
        expected,
        actual,
        ..
    } = &cause
    else {
        panic!("a carries mismatch, got {cause:?}");
    };
    assert_eq!(channel, "Q");
    assert_ne!(expected, actual);
    assert_eq!(at, "M::Tick");

    let text = with_channel("M::Hop", "fifo by (m: M::Hop) { m.node.total = 1 }");
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("ill_typed", Some("operator-ineligible")));
    assert!(matches!(&cause, ProtocolAnchorCause::KeyModelState { channel, .. } if channel == "Q"));
    assert_eq!(at, "m.node.total");
}

/// FR-218-AC-20: a FIFO key of type `Boolean`, and a key of a record type,
/// check because the clause checker admits `=` over them; a key of an event
/// type, over which it admits no `=`, refuses `ill_typed`/
/// `operator-ineligible` at the key, retaining the type.
#[trace("TC-1056", "FR-218-AC-20")]
#[test]
fn a_fifo_key_admits_equality() {
    for (carries, key) in [("M::Flag", "m.ready"), ("M::Flag", "m")] {
        let ordering = ordering(&with_channel(
            carries,
            &format!("fifo by (m: {carries}) {{ {key} }}"),
        ));
        assert!(matches!(ordering, CheckedOrdering::Fifo { .. }), "{key}");
    }
    let text = with_channel("M::Pulse", "fifo by (m: M::Pulse) { m }");
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("ill_typed", Some("operator-ineligible")));
    let ProtocolAnchorCause::KeyEquality {
        channel, key_type, ..
    } = &cause
    else {
        panic!("a key-equality refusal, got {cause:?}");
    };
    assert_eq!(channel, "Q");
    assert!(matches!(**key_type, ValueType::Composite(_)));
    assert_eq!(at, "m");
}

/// A protocol `Decide` whose run tree is `run`, over `view: M::Node`.
fn decide(members: &str, run: &str) -> String {
    format!(
        "{HEADER}protocol Decide using v over (view: M::Node) on origin {{\n\
         role Service on M::Node;\n\
         {members}\n\
         run {run}\n\
         finish Closed as (closed: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

/// An attempt `Tried` by `Service` with binder `attempted: binder_type`,
/// then a two-case `choice Pick` by `Service`.
fn attempted(binder_type: &str, visible: &str, yes: &str, no: &str) -> String {
    format!(
        "sequence Main {{ attempt Tried by Service on M::Node::step contracts [] \
         as (attempted: {binder_type}) {{ true }}; \
         choice Pick by Service visible ({visible}) {{ \
         case yes when {{ {yes} }} sequence Yes {{ }} \
         case no when {{ {no} }} sequence No {{ }} }} }}"
    )
}

fn choice_with_guards(count: usize, yes: &str, no: &str) -> String {
    let events = (0..count)
        .map(|index| format!("event E{index} by Service as (a{index}: Boolean) {{ true }}; "))
        .collect::<String>();
    let visible = (0..count)
        .map(|index| format!("a{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    decide(
        "",
        &format!(
            "sequence Main {{ {events} \
             choice Pick by Service visible ({visible}) {{ \
             case yes when {{ {yes} }} sequence Yes {{ }} \
             case no when {{ {no} }} sequence No {{ }} }} }}"
        ),
    )
}

fn nested_repeat_with_guard(guard: &str) -> String {
    nested_repeat_with_guard_and_outer_progress(guard, false)
}

fn nested_repeat_with_guard_and_outer_progress(guard: &str, outer_progress: bool) -> String {
    let outer_progress = if outer_progress {
        "event OuterProgress by Service as (outer_progress: Boolean) { true };"
    } else {
        ""
    };
    decide(
        "",
        &format!(
            "sequence Main {{ \
             event A by Service as (a: Boolean) {{ true }}; \
             event B by Service as (b: Boolean) {{ true }}; \
             repeat Outer by Service visible () while {{ true }} \
             sequence OuterBody {{ \
             {outer_progress} \
             repeat Inner by Service visible (a, b) max 1 while {{ {guard} }} \
             sequence InnerBody {{ \
             event Progress by Service as (progress: Boolean) {{ true }}; }} \
             exhausted check Exhausted using v {{ true }}; \
             }} }}"
        ),
    )
}

#[trace("TC-1172", "FR-218-AC-30")]
#[test]
fn choice_partition_charges_once_per_valuation() {
    let three_atoms = choice_with_guards(3, "a0 and a1 and a2", "not (a0 and a1 and a2)");
    let one_atom = choice_with_guards(1, "a0 and a0 and a0", "not (a0 and a0 and a0)");
    for text in [&three_atoms, &one_atom] {
        let graph = check(text).expect("the choice guards partition");
        let [protocol] = graph.protocols() else {
            panic!("one checked protocol");
        };
        assert_eq!(protocol.slots.choices.len(), 1);
    }
    for (text, budget, denied) in [(&one_atom, 1_u64, 2_u128), (&three_atoms, 7, 8)] {
        let refusals = check_with_limits(text, CheckingLimits::default().with_work_budget(budget))
            .expect_err("the next choice valuation exceeds the configured budget");
        let [refusal] = refusals.as_slice() else {
            panic!("one limit refusal, got {refusals:?}");
        };
        let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
            panic!("a work-budget refusal, got {refusal:?}");
        };
        assert_eq!(limit.kind.setting(), Setting::S3WorkUnits);
        assert_eq!(limit.limit, budget);
        assert_eq!(limit.actual, denied);
        let region = limit.region.as_ref().expect("the choice's region");
        let start = usize::try_from(region.start()).expect("region start fits usize");
        let end = usize::try_from(region.end()).expect("region end fits usize");
        assert_eq!(&text[start..end], "Pick");
    }
}

#[trace("TC-1172", "FR-218-AC-30")]
#[test]
fn raising_the_caller_work_budget_admits_the_partition() {
    const RAISED_WORK_UNITS: u64 = 33_554_432;
    let text = choice_with_guards(8, "a0", "not a0");
    check_with_limits(
        &text,
        CheckingLimits::default().with_work_budget(RAISED_WORK_UNITS),
    )
    .expect("the caller can raise s3.work_units to admit the partition proof");
}

#[trace("TC-1172", "FR-218-AC-32", "FR-048-AC-5")]
#[test]
fn nested_repeat_feasibility_is_independent_and_budgeted() {
    let feasible_exit = nested_repeat_with_guard("a");
    let refusals = check(&feasible_exit).expect_err("the inner repeat may skip its body");
    assert!(refusals.iter().any(|refusal| matches!(
        &refusal.cause,
        CheckCause::ProtocolAnchor(cause)
            if matches!(cause.as_ref(), ProtocolAnchorCause::NoProgress { repeat, .. } if repeat == "Outer")
    )));
    check(&nested_repeat_with_guard("a or not a or b or not b"))
        .expect("the nested repeat's infeasible false guard removes its skip edge");

    let guard_false_infeasible =
        nested_repeat_with_guard_and_outer_progress("a or not a or b or not b", true);
    check(&guard_false_infeasible).expect("the nested repeat always enters its body");

    // FR-048-AC-5: a two-atom proof needs four work units. Zero units refuses
    // before the first valuation; three refuses on the fourth. At exactly
    // four, the proof completes and any later work has a different locus.
    let zero = check_with_limits(
        &guard_false_infeasible,
        CheckingLimits::default().with_work_budget(0),
    )
    .expect_err("zero proof budget refuses before evaluating its first valuation");
    assert_resource_work_limit(&zero, 0);
    assert_limit_region(&guard_false_infeasible, &zero, "Inner");

    let one_short = check_with_limits(
        &guard_false_infeasible,
        CheckingLimits::default().with_work_budget(3),
    )
    .expect_err("the fourth independent valuation exceeds the one-short limit");
    assert_resource_work_limit(&one_short, 3);
    assert_limit_region(&guard_false_infeasible, &one_short, "Inner");

    let exact = check_with_limits(
        &guard_false_infeasible,
        CheckingLimits::default().with_work_budget(4),
    )
    .expect_err("the proof completes at four units, before later S3 work");
    assert!(!exact.is_empty(), "later S3 work still needs its own units");
    for refusal in &exact {
        let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
            panic!("a typed later work limit, got {refusal:?}");
        };
        assert_eq!(limit.kind.setting(), Setting::S3WorkUnits);
        assert_eq!(limit.limit, 4);
        assert_eq!(limit.actual, 5);
        assert!(limit.region.is_none(), "the repeat proof already completed");
    }
    assert!(!exact.iter().any(|refusal| {
        matches!(&refusal.cause, CheckCause::ResourceExhausted(limit) if limit.region.as_ref().is_some_and(|region| {
            let start = usize::try_from(region.start()).expect("region start fits usize");
            let end = usize::try_from(region.end()).expect("region end fits usize");
            &guard_false_infeasible[start..end] == "Inner"
        }))
    }));
    assert!(!exact.iter().any(|refusal| matches!(
        &refusal.cause,
        CheckCause::ProtocolAnchor(cause)
            if matches!(cause.as_ref(), ProtocolAnchorCause::NoProgress { repeat, .. } if repeat == "Outer")
    )));
}

fn assert_resource_work_limit(refusals: &[CheckRefusal], bound: u64) {
    let [refusal] = refusals else {
        panic!("one resource refusal, got {refusals:?}");
    };
    let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
        panic!("a work-budget refusal, got {refusal:?}");
    };
    assert_eq!(limit.kind.setting(), Setting::S3WorkUnits);
    assert_eq!(limit.limit, bound);
    assert_eq!(limit.actual, u128::from(bound) + 1);
}

fn assert_limit_region(text: &str, refusals: &[CheckRefusal], expected: &str) {
    let [refusal] = refusals else {
        panic!("one limit refusal, got {refusals:?}");
    };
    let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
        panic!("a work-budget refusal, got {refusal:?}");
    };
    let region = limit
        .region
        .as_ref()
        .unwrap_or_else(|| panic!("resource refusal is not at the nested repeat: {limit:?}"));
    let start = usize::try_from(region.start()).expect("region start fits usize");
    let end = usize::try_from(region.end()).expect("region end fits usize");
    assert_eq!(&text[start..end], expected);
}

/// FR-218-AC-2: a `choice` guard that reads the `over` input's field `k.v`
/// refuses `ill_typed`/`operator-ineligible` at the read; a two-case
/// `choice` whose `visible` entry `attempted.n` is an individual non-Boolean
/// fact, admitted as a decision fact, and whose guards are
/// `attempted.n > 0` and `attempted.n > 1`, refuses
/// `undefined_expression`/`unproved-exhaustiveness` with both arm faults,
/// while `attempted.n > 0` and `not (attempted.n > 0)` check; a join that names a
/// label that is no branch refuses `missing_declaration`/`missing-name` at
/// the label.
#[trace("TC-663", "FR-218-AC-2")]
#[test]
fn a_guard_reads_no_model_state_and_partitions_its_facts() {
    let text = decide(
        "",
        &attempted("M::Receipt", "", "view.n > 0", "not (view.n > 0)"),
    );
    let refusals = check(&text)
        .map(|_| ())
        .expect_err("the guard reads model state");
    let reads: Vec<&str> = refusals
        .iter()
        .map(|refusal| {
            let CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
                panic!("a protocol cause, got {refusal:?}");
            };
            assert!(
                matches!(cause.as_ref(), ProtocolAnchorCause::SlotModelState { node, .. } if node == "Pick"),
                "{cause:?}"
            );
            assert_eq!(refusal.cause.code().as_str(), "ill_typed");
            assert_eq!(refusal.cause.cause(), Some("operator-ineligible"));
            let span = cause.span();
            &text[span.start..span.end]
        })
        .collect();
    assert_eq!(reads, ["view.n", "view.n"]);

    let text = decide(
        "",
        &attempted(
            "M::Receipt",
            "attempted.n",
            "attempted.n > 0",
            "attempted.n > 1",
        ),
    );
    let (cause, pair, at) = only(&text);
    assert_eq!(
        pair,
        ("undefined_expression", Some("unproved-exhaustiveness"))
    );
    let ProtocolAnchorCause::Exhaustiveness { failed, .. } = &cause else {
        panic!("an exhaustiveness refusal, got {cause:?}");
    };
    // `attempted.n > 0` and `attempted.n > 1` are two independent atoms.
    assert_eq!(
        failed,
        &[
            crate::check::ArmFault::MissingArm,
            crate::check::ArmFault::DuplicateArm
        ]
    );
    assert_eq!(at, "Pick");
    // TC-1056 row 45: one atom and its negation, `not (got.count > 0)`,
    // partition over the individual non-Boolean entry.
    check(&decide(
        "",
        &attempted(
            "M::Receipt",
            "attempted.n",
            "attempted.n > 0",
            "not (attempted.n > 0)",
        ),
    ))
    .expect("an atom and its negation partition");

    let text = decide(
        "",
        "parallel Fill { branch left sequence L { } branch right sequence R { } } \
         join all [left, middle];",
    );
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("missing_declaration", Some("missing-name")));
    assert!(matches!(cause, ProtocolAnchorCause::Missing { .. }));
    assert_eq!(at, "middle");
}

/// FR-218-AC-26: a `choice` guard and a `visible` entry that read a field of
/// an object-typed attempt binder, and a FIFO key that reads a field through
/// a reference-typed field of its record binder, each refuse
/// `ill_typed`/`operator-ineligible` at the read, naming the guard, the
/// `visible` entry or the key; the same reads over a record binder's own
/// fields check. A channel that carries an object type refuses once, at the
/// type, and gives no refusal at its `send`, `receive` or FIFO binders.
#[trace("TC-1056", "FR-218-AC-26")]
#[test]
fn an_object_or_reference_read_is_model_state() {
    let slots = |text: &str| -> Vec<(crate::check::SlotKind, String)> {
        let refusals = check(text)
            .map(|_| ())
            .expect_err("the reads are model state");
        refusals
            .iter()
            .map(|refusal| {
                let CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
                    panic!("a protocol cause, got {refusal:?}");
                };
                let ProtocolAnchorCause::SlotModelState { node, slot, span } = cause.as_ref()
                else {
                    panic!("a model-state refusal, got {cause:?}");
                };
                assert_eq!(node, "Pick");
                assert_eq!(refusal.cause.cause(), Some("operator-ineligible"));
                (*slot, text[span.start..span.end].to_owned())
            })
            .collect()
    };
    let guard = decide(
        "",
        &attempted("M::Node", "", "attempted.n > 0", "not (attempted.n > 0)"),
    );
    assert_eq!(
        slots(&guard),
        [
            (crate::check::SlotKind::Guard, "attempted.n".to_owned()),
            (crate::check::SlotKind::Guard, "attempted.n".to_owned()),
        ]
    );
    let visible = decide(
        "",
        &attempted("M::Node", "attempted.n > 0", "true", "false"),
    );
    assert_eq!(
        slots(&visible),
        [(crate::check::SlotKind::Visible, "attempted.n".to_owned())]
    );
    let text = with_channel("M::Hop", "fifo by (m: M::Hop) { m.node.n }");
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("ill_typed", Some("operator-ineligible")));
    assert!(matches!(cause, ProtocolAnchorCause::KeyModelState { .. }));
    assert_eq!(at, "m.node.n");

    // The same reads over a record binder's own fields check.
    check(&decide(
        "",
        &attempted(
            "M::Receipt",
            "attempted.n > 0",
            "attempted.n > 0",
            "not (attempted.n > 0)",
        ),
    ))
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    check(&with_channel("M::Hop", "fifo by (m: M::Hop) { m.ready }"))
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));

    // A channel of an object type refuses once, at the type.
    let text = decide(
        "channel Q from Service to Service carries M::Node ordering fifo by (m: M::Node) \
         { true } delivery unknown capacity 1 overflow reject;",
        "sequence Main { send Tx via Q as (tx: M::Node) { true }; \
         receive Rx via Q of Tx as (rx: M::Node) { true }; }",
    );
    let (cause, pair, at) = only(&text);
    assert_eq!(
        pair,
        ("invalid_model_binding", Some("malformed-declaration"))
    );
    assert!(matches!(
        cause,
        ProtocolAnchorCause::ChannelCarriesObject { .. }
    ));
    assert_eq!(at, "M::Node");
}

/// An outer `repeat` whose only body is the nested `repeat Inner` with
/// `max` (written when given), the guard `true`, `body` and, with a
/// maximum, the `exhausted` control `exhausted`.
fn nested_repeat_of(max: Option<u64>, body: &str, exhausted: &str) -> String {
    let (max, exhausted) = match max {
        Some(max) => (format!("max {max} "), format!(" exhausted {exhausted};")),
        None => (String::new(), String::new()),
    };
    decide(
        "",
        &format!(
            "sequence Main {{ \
             repeat Outer by Service visible () while {{ true }} \
             sequence OuterBody {{ \
             repeat Inner by Service visible () {max}while {{ true }} \
             sequence InnerBody {{ {body} }}{exhausted} \
             }} }}"
        ),
    )
}

fn refuses_outer(text: &str) {
    let refusals = check(text).expect_err("the outer repeat has no progress");
    assert!(
        refusals.iter().any(|refusal| matches!(
            &refusal.cause,
            CheckCause::ProtocolAnchor(cause)
                if matches!(cause.as_ref(), ProtocolAnchorCause::NoProgress { repeat, .. } if repeat == "Outer")
        )),
        "{refusals:?}"
    );
}

const PROGRESS: &str = "event Progress by Service as (progress: Boolean) { true };";
const EXHAUSTED_EVENT: &str = "event Spent by Service as (spent: Boolean) { true }";

/// QSpec FR-464 "Nested `repeat` rule" (STD-539, QSpec #481): a nested
/// `repeat` is counted by its body only. `repeat max 1 while true` whose
/// body is only a `check` and whose `exhausted` control is an event leaves
/// the outer `repeat` refused (FR-464-AC-23); with the event in its body it
/// is admitted.
#[trace("TC-663", "FR-218-AC-32")]
#[test]
fn a_nested_repeat_is_never_counted_through_its_exhausted_control() {
    refuses_outer(&nested_repeat_of(
        Some(1),
        "check Idle using v { true };",
        EXHAUSTED_EVENT,
    ));
    check(&nested_repeat_of(
        Some(1),
        PROGRESS,
        "check Done using v { true }",
    ))
    .expect("a nested repeat that steps in its body is a step of the outer one");
}

/// QSpec FR-464 "Nested `repeat` rule" item 2 (STD-539, FR-464-AC-21): a
/// nested `repeat max 0 while true` never enters its body, so it gives the
/// outer `repeat` no progress whatever its guard; with no maximum, or a
/// maximum of 1, the same nested `repeat` is a step.
#[trace("TC-663", "FR-218-AC-32")]
#[test]
fn a_nested_repeat_with_maximum_zero_gives_no_progress() {
    refuses_outer(&nested_repeat_of(
        Some(0),
        PROGRESS,
        "check Done using v { true }",
    ));
    check(&nested_repeat_of(None, PROGRESS, ""))
        .expect("an unbounded nested repeat with an infeasible exit steps");
    check(&nested_repeat_of(
        Some(1),
        PROGRESS,
        "check Done using v { true }",
    ))
    .expect("a maximum of 1 enters the body once per entry");
}

/// The `NoProgress` refusals of `text`, by repeat name.
fn no_progress(text: &str) -> Vec<String> {
    check(text)
        .err()
        .unwrap_or_default()
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            CheckCause::ProtocolAnchor(cause) => match cause.as_ref() {
                ProtocolAnchorCause::NoProgress { repeat, .. } => Some(repeat.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// QSpec FR-464-AC-21's `choice` clause (STD-539, SR-1758002 FND-001): a
/// nested `repeat` that is the only stepping part of a `choice` case
/// follows the nested `repeat` rule. With `max 0` the outer `repeat` is
/// refused; with `max 1` it is admitted.
#[trace("TC-663", "FR-218-AC-32")]
#[test]
fn a_nested_repeat_in_a_choice_case_follows_the_rule() {
    let text = |max: u64| {
        decide(
            "",
            &format!(
                "sequence Main {{ \
                 repeat Outer by Service visible () while {{ true }} \
                 choice Pick by Service visible () {{ \
                 case work when {{ true }} sequence Work {{ \
                 event Worked by Service as (worked: Boolean) {{ true }}; }} \
                 case more when {{ false }} repeat Inner by Service visible () max {max} \
                 while {{ true }} sequence InnerBody {{ {PROGRESS} }} \
                 exhausted check Spent using v {{ true }}; }} }}"
            ),
        )
    };
    assert_eq!(no_progress(&text(0)), ["Outer"]);
    check(&text(1)).expect("a maximum of 1 makes the case step");
}

/// QSpec FR-464 (STD-539, SR-1758001 FND-001, SR-1758002 FND-002): through
/// an intermediate `repeat Mid`, a `max 0` `Inner` gives neither `Mid` nor
/// `Outer` progress, and only the first non-stepping `repeat` in tree
/// order, `Outer`, is refused. A `repeat` reached through `Mid`'s in-body
/// `exhausted` control is no step of `Mid`: with `Mid`'s body only a
/// `check`, `Outer` is refused again.
#[trace("TC-663", "FR-218-AC-32")]
#[test]
fn deeper_nesting_refuses_the_first_repeat_once() {
    let deeper = decide(
        "",
        &format!(
            "sequence Main {{ \
             repeat Outer by Service visible () while {{ true }} \
             repeat Mid by Service visible () while {{ true }} \
             repeat Inner by Service visible () max 0 while {{ true }} \
             sequence InnerBody {{ {PROGRESS} }} \
             exhausted check Spent using v {{ true }}; }}"
        ),
    );
    assert_eq!(no_progress(&deeper), ["Outer"]);

    let through_exhausted = decide(
        "",
        &format!(
            "sequence Main {{ \
             repeat Outer by Service visible () while {{ true }} \
             repeat Mid by Service visible () max 1 while {{ true }} \
             sequence MidBody {{ check Idle using v {{ true }}; }} \
             exhausted repeat Inner by Service visible () while {{ true }} \
             sequence InnerBody {{ {PROGRESS} }} }}"
        ),
    );
    assert_eq!(no_progress(&through_exhausted), ["Outer"]);
}
