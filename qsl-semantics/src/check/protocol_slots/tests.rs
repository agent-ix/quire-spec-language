//! The C1b3 protocol members over fixture R of the protocol obligation
//! tests (FR-211, FR-218): the `terminal when` predicate.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use quire_exact::ValueType;
use quire_semantic_value::checking::CheckingLimits;

use super::CheckedTerminalMember;
use crate::check::refusal::{ProtocolAnchorCause, SlotKind};
use crate::check::{CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations};

mod fixture {
    use crate as semantics;
    include!("../../../tests/fixtures/protocol_obligations.rs");
}
use fixture::{obligation_model, HEADER};

/// ADR-027 §7's `Fill` over `k: M::Node`, activated `activation`, with
/// `members` written before `run`.
fn fill(activation: &str, members: &str) -> String {
    format!(
        "{HEADER}protocol Fill using v over (k: M::Node) {activation} {{\n\
         capture start: M::Total = k.total;\n\
         role w on M::Node;\n\
         {members}\n\
         run sequence Main {{\n\
         parallel Both {{\n\
         branch left attempt A by w on M::Node::step contracts [] as (a: M::Flag) {{ true }};\n\
         branch right attempt B by w on M::Node::step contracts [] as (b: M::Flag) {{ true }};\n\
         }} join all [left, right];\n\
         }}\n\
         finish Done as (d: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

const ORIGIN: &str = "on origin";
const EACH: &str = "on each (opened: M::Tick) when (opened.ready)";

fn check(text: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
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
        unit,
        vec![obligation_model()],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: the assembler refuses: {refusal:?}"))
    .check(CheckingLimits::default())
}

fn checks(text: &str) -> CheckedGraph {
    check(text).unwrap_or_else(|refusals| panic!("{text}: {refusals:?}"))
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

fn terminal(graph: &CheckedGraph) -> &CheckedTerminalMember {
    &graph
        .protocol("Fill")
        .expect("the checked protocol")
        .slots
        .terminal
}

/// FR-211 "The `terminal` member" (FR-211-AC-1's checking half; its
/// package-identity half waits on FR-383's emission of the member): `Fill`
/// checks with no `terminal` member, with `terminal when` a Boolean
/// predicate over `k`'s field and with `terminal any`, and records each.
/// A predicate that reads binder `a` refuses `missing_declaration`/
/// `missing-name` at the read, as FR-104 refuses a non-state read in an
/// invariant, and a non-Boolean predicate refuses `non-boolean-root`.
#[trace("TC-656")]
#[test]
fn a_terminal_predicate_reads_the_model_observation_and_the_captures() {
    assert_eq!(
        terminal(&checks(&fill(ORIGIN, ""))),
        &CheckedTerminalMember::None
    );
    assert_eq!(
        terminal(&checks(&fill(ORIGIN, "terminal any;"))),
        &CheckedTerminalMember::Any
    );
    let graph = checks(&fill(
        ORIGIN,
        "terminal when { k.total = 1 and start >= 0 };",
    ));
    let CheckedTerminalMember::When(predicate) = terminal(&graph) else {
        panic!("a `when` member");
    };
    assert_eq!(predicate.root().value_type(), &ValueType::Boolean);

    let text = fill(ORIGIN, "terminal when { a.ready };");
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("missing_declaration", Some("missing-name")));
    let ProtocolAnchorCause::SlotClause {
        node, slot, cause, ..
    } = &cause
    else {
        panic!("a slot refusal, got {cause:?}");
    };
    assert_eq!((node.as_str(), *slot), ("Fill", SlotKind::Terminal));
    assert_eq!(**cause, CheckCause::MissingName("a".to_owned()));
    assert_eq!(at, "a");

    let (cause, pair, _) = only(&fill(ORIGIN, "terminal when { k.total };"));
    assert_eq!(pair, ("ill_typed", Some("non-boolean-root")));
    assert!(matches!(
        cause,
        ProtocolAnchorCause::SlotClause {
            slot: SlotKind::Terminal,
            ..
        }
    ));
}

/// QSpec FR-429 "The `terminal` member": under `activation on each` the
/// predicate has no instance binder, so a read of a protocol capture or of
/// the activation binder refuses `missing_declaration`/`missing-name` at the
/// read; the `over` input is still read.
#[trace("TC-656")]
#[test]
fn an_on_each_terminal_predicate_reads_no_capture_and_no_activation_binder() {
    checks(&fill(EACH, "terminal when { k.total = 1 };"));
    for (read, name) in [("start >= 0", "start"), ("opened.ready", "opened")] {
        let text = fill(EACH, &format!("terminal when {{ {read} }};"));
        let (cause, pair, at) = only(&text);
        assert_eq!(
            pair,
            ("missing_declaration", Some("missing-name")),
            "{read}"
        );
        let ProtocolAnchorCause::SlotClause { cause, .. } = &cause else {
            panic!("a slot refusal, got {cause:?}");
        };
        assert_eq!(**cause, CheckCause::MissingName(name.to_owned()));
        assert_eq!(at, name);
    }
}
