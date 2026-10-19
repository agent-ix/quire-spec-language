// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-381 (TC-1057, TC-1058): S3 checks a protocol's activation, captures,
//! compensation templates over
//! fixture R. Every assertion is on the refusal's typed code, cause, members
//! and span text, never on a message.
//!
//! A protocol that holds a `compensate` template also holds the `effect` and
//! `commit` nodes it names, and those nodes are not covered by
//! `protocol_clause::content` yet, so such a protocol never checks as a
//! whole. These tests read the obligation refusals of the whole pipeline and
//! assert that a part that checks produces none.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::{SourceIdentity, Span};
use quire_exact::ValueType;
use quire_semantic_value::checking::CheckingLimits;

use super::{
    CaptureOwner, CaptureStage, CheckedActivation, ObligationCause, ObligationOwner,
    ObligationStage, ReadClass, Site, Universe,
};
use crate::check::{
    CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations, ProtocolAnchorCause,
};

mod fixture {
    use crate as semantics;
    include!("../../../tests/fixtures/protocol_obligations.rs");
}
use fixture::{obligation_model, HEADER};

/// S1, S2 and the assembler over `declarations`, against fixture R.
fn assemble(declarations: &str) -> (String, PackageDeclarations) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let assembled = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![obligation_model()],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{declarations}: the assembler refuses: {refusal:?}"));
    (text, assembled)
}

/// Every refusal of the whole pipeline over `declarations`; none when it
/// checks.
fn refusals(declarations: &str) -> (String, Vec<CheckRefusal>) {
    let (text, assembled) = assemble(declarations);
    match assembled.check(CheckingLimits::default()) {
        Err(refusals) => (text, refusals),
        Ok(_) => (text, Vec::new()),
    }
}

/// The checked graph of a protocol that checks as a whole.
fn checked(declarations: &str) -> CheckedGraph {
    let (_, assembled) = assemble(declarations);
    assembled
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"))
}

/// The obligation refusals among `refusals`, in order. Every other refusal
/// must be the not-yet-implemented one for the uncovered `effect`/`commit`
/// nodes, or `content`'s unresolved binder type: a limit, a fault or any other cause would otherwise let a
/// "checks" assertion pass because the stage never ran.
fn obligation_causes(refusals: &[CheckRefusal]) -> Vec<ObligationCause> {
    refusals
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            CheckCause::ProtocolObligation(cause) => Some((**cause).clone()),
            CheckCause::ProtocolAnchor(cause)
                if matches!(
                    **cause,
                    ProtocolAnchorCause::Unimplemented { .. }
                        | ProtocolAnchorCause::BinderType { .. }
                ) =>
            {
                None
            }
            other => panic!("a refusal outside the obligation stage: {other:?}"),
        })
        .collect()
}

/// The obligation refusals of `declarations`, with the unit's text.
fn obligations(declarations: &str) -> (String, Vec<ObligationCause>) {
    let (text, found) = refusals(declarations);
    let causes = obligation_causes(&found);
    (text, causes)
}

fn at<'t>(text: &'t str, cause: &ObligationCause) -> &'t str {
    let span = cause.span();
    &text[span.start..span.end]
}

/// The one obligation refusal of `declarations`.
fn only(declarations: &str) -> (String, ObligationCause) {
    let (text, mut found) = obligations(declarations);
    assert_eq!(found.len(), 1, "{declarations}: {found:?}");
    let cause = found.remove(0);
    (text, cause)
}

/// The refusal as a stage read: its name, stage, owner and read class.
fn stage_read(cause: &ObligationCause) -> (&str, ObligationStage, &ObligationOwner, ReadClass) {
    let ObligationCause::StageRead {
        name,
        stage,
        owner,
        read,
        ..
    } = cause
    else {
        panic!("a stage read, got {cause:?}");
    };
    (name, *stage, owner, *read)
}

/// A protocol with no template: the activation header and the captures are
/// the test's.
fn plain(activation: &str, captures: &str) -> String {
    format!(
        "protocol Refunds using v over (view: M::Node) {activation} {{\n\
         {captures}\n\
         role Service on M::Node;\n\
         run sequence Main {{\n\
         attempt Tried by Service on M::Node::step contracts [] \
         as (attempted: M::Flag) {{ true }};\n\
         }}\n\
         finish Closed as (closed: M::Receipt) {{ true }};\n\
         }}"
    )
}

const TICK: &str = "on each (opened: M::Tick) when (opened.ready)";

/// TC-1057's base protocol `Refunds`, with the bodies of its run tree the
/// bare literal `true`.
const BASE: &str = "protocol Refunds using v over (view: M::Node)\n\
    on each (opened: M::Tick) when (opened.ready) {\n\
    capture order: M::Total = view.total;\n\
    role Service on M::Node;\n\
    compensate Refund for Main::Applied as (forward: M::Receipt)\n\
        by Service on M::Node::step using v clock \"refund-clock\" {\n\
      capture target: M::Total = forward.total;\n\
      activate first (trigger: M::Flag) when { not trigger.ready and target >= 0 } {\n\
        capture activated: Boolean = trigger.ready;\n\
      }\n\
      within [0,30]; attempts 3 of M::Attempt;\n\
      retry (earlier: M::Attempt, later: M::Attempt) {\n\
        earlier.tally < 3 and later.tally = earlier.tally + 1\n\
        and target >= 0 and not activated\n\
      };\n\
      commit Main::Committed;\n\
      recover (recovered: M::Receipt) { recovered.n = 1 and target >= 0 and not activated };\n\
    }\n\
    run sequence Main {\n\
      attempt Tried by Service on M::Node::step contracts [] as (attempted: M::Flag) { true };\n\
      effect Applied of Main::Tried as (applied: M::Receipt) { true };\n\
      commit Committed by Service as (committed: M::Flag) { true };\n\
    }\n\
    finish Closed as (closed: M::Receipt) { true };\n\
    }";

/// `text` with its first `old` replaced by `new`.
fn edit(text: &str, old: &str, new: &str) -> String {
    assert!(text.contains(old), "the protocol holds {old}");
    text.replacen(old, new, 1)
}

/// `BASE` with its first `old` replaced by `new`.
fn changed(old: &str, new: &str) -> String {
    edit(BASE, old, new)
}

const TRIGGER: &str =
    "activate first (trigger: M::Flag) when { not trigger.ready and target >= 0 } {\n\
        capture activated: Boolean = trigger.ready;\n\
      }";

const RETRY: &str = "retry (earlier: M::Attempt, later: M::Attempt) {\n\
        earlier.tally < 3 and later.tally = earlier.tally + 1\n\
        and target >= 0 and not activated\n\
      };";

const RECOVER: &str =
    "recover (recovered: M::Receipt) { recovered.n = 1 and target >= 0 and not activated };";

/// A second template `Other` for the base protocol's effect, with the
/// registration capture the test writes.
fn other(registration: &str) -> String {
    format!(
        "compensate Other for Main::Applied as (otherfwd: M::Receipt)\n\
         by Service on M::Node::step using v clock \"other-clock\" {{\n\
         {registration}\n\
         activate first (otrigger: M::Flag) when {{ true }} {{ }}\n\
         within [0,30]; attempts 3 of M::Attempt;\n\
         retry (oearlier: M::Attempt, olater: M::Attempt) {{ true }};\n\
         commit never;\n\
         recover (orecovered: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

/// The base protocol checks nothing the obligations refuse.
#[test]
fn the_base_protocol_gives_no_obligation_refusal() {
    let (_, found) = obligations(BASE);
    assert!(found.is_empty(), "{found:?}");
}

/// FR-381-AC-4: `activation on each (x: T) when (g)` checks and is recorded
/// with its binder and guard; `on origin` is recorded as `Origin`; a guard
/// that reads the `over` input, a protocol capture or nothing Boolean
/// refuses as the criterion states; an unresolved binder type refuses as a
/// declared type does.
#[trace("TC-1057", "FR-381-AC-4")]
#[test]
fn the_activation_is_checked_and_recorded() {
    let graph = checked(&plain(TICK, "capture order: M::Total = view.total;"));
    let CheckedActivation::OnEach { binder, guard, .. } =
        &graph.protocols()[0].obligations.activation
    else {
        panic!("an `on each` activation");
    };
    assert_eq!(binder, "opened");
    assert!(guard.is_some());

    let graph = checked(&plain("on origin", "capture order: M::Total = view.total;"));
    assert_eq!(
        graph.protocols()[0].obligations.activation,
        CheckedActivation::Origin
    );

    let graph = checked(&plain("on each (opened: M::Tick)", ""));
    let CheckedActivation::OnEach { guard, .. } = &graph.protocols()[0].obligations.activation
    else {
        panic!("an `on each` activation");
    };
    assert!(guard.is_none(), "an absent guard is recorded as absent");

    let (text, cause) = only(&plain(
        "on each (opened: M::Tick) when (opened.ready and view.total > 0)",
        "",
    ));
    assert!(matches!(
        &cause,
        ObligationCause::ModelStateRead {
            stage: ObligationStage::ProtocolActivation,
            owner: ObligationOwner::Protocol,
            ..
        }
    ));
    assert_eq!(cause.code(), Code::IllTyped);
    assert_eq!(cause.tag(), Some("operator-ineligible"));
    assert_eq!(at(&text, &cause), "view.total");

    let (text, cause) = only(&plain(
        "on each (opened: M::Tick) when (order >= 0)",
        "capture order: M::Total = view.total;",
    ));
    assert_eq!(
        stage_read(&cause),
        (
            "order",
            ObligationStage::ProtocolActivation,
            &ObligationOwner::Protocol,
            ReadClass::Stage
        )
    );
    assert_eq!(cause.code(), Code::MissingDeclaration);
    assert_eq!(cause.tag(), Some("missing-name"));
    assert_eq!(at(&text, &cause), "order");

    let (text, cause) = only(&plain("on each (opened: M::Tick) when (opened.count)", ""));
    let ObligationCause::Clause { cause: inner, .. } = &cause else {
        panic!("a clause refusal, got {cause:?}");
    };
    assert_eq!(**inner, CheckCause::NonBooleanRoot);
    assert_eq!(cause.code(), Code::IllTyped);
    assert_eq!(cause.tag(), Some("non-boolean-root"));
    assert_eq!(at(&text, &cause), "opened.count");

    let (text, found) = refusals(&plain("on each (opened: M::Absent)", ""));
    let [CheckRefusal {
        cause: CheckCause::ProtocolAnchor(cause),
        ..
    }] = &found[..]
    else {
        panic!("one binder-type refusal, got {found:?}");
    };
    assert_eq!(cause.code(), Code::MissingDeclaration);
    assert_eq!(cause.tag(), "missing-name");
    assert!(text[cause.span().start..cause.span().end].ends_with("Absent"));
}

/// A protocol capture is recorded with its stage and owner, in source order.
#[test]
fn protocol_captures_are_recorded_in_source_order() {
    let graph = checked(&plain(
        TICK,
        "capture order: M::Total = view.total;\ncapture again: M::Total = order;",
    ));
    let captures = &graph.protocols()[0].obligations.captures;
    let names: Vec<&str> = captures
        .iter()
        .map(|capture| capture.name.as_str())
        .collect();
    assert_eq!(names, ["order", "again"]);
    for capture in captures {
        assert_eq!(capture.stage, CaptureStage::Protocol);
        assert_eq!(capture.owner, CaptureOwner::Protocol);
        assert!(matches!(capture.declared_type, ValueType::Int(_)));
    }
}

/// FR-381-AC-2: a capture that reads itself or a capture written later
/// refuses with `read` `forward`, one that reads another template's capture
/// with `foreign`, and two captures that read each other give exactly one
/// refusal, at the read inside the earlier capture, with the later capture
/// checking.
#[trace("TC-1057", "FR-381-AC-2")]
#[test]
fn a_capture_cannot_read_itself_a_later_capture_or_another_templates() {
    let (text, cause) = only(&plain(TICK, "capture a: Boolean = a;"));
    assert_eq!(
        stage_read(&cause),
        (
            "a",
            ObligationStage::ProtocolCapture,
            &ObligationOwner::Protocol,
            ReadClass::Forward
        )
    );
    assert_eq!(at(&text, &cause), "a");
    assert!(
        text[cause.span().start..].starts_with("a;"),
        "the read inside the initializer, not the declared name"
    );

    let (text, cause) = only(&plain(
        TICK,
        "capture a: Boolean = b;\ncapture b: Boolean = true;",
    ));
    assert_eq!(stage_read(&cause).3, ReadClass::Forward);
    assert!(text[cause.span().start..].starts_with("b;"));

    let (_, found) = obligations(&plain(
        TICK,
        "capture a: Boolean = true;\ncapture b: Boolean = a;",
    ));
    assert!(found.is_empty(), "{found:?}");

    let (text, cause) = only(&plain(
        TICK,
        "capture x: Boolean = y;\ncapture y: Boolean = x;",
    ));
    assert_eq!(stage_read(&cause).3, ReadClass::Forward);
    assert_eq!(at(&text, &cause), "y");
    assert!(text[cause.span().start..].starts_with("y;"));

    let with_other = changed(
        "run sequence Main",
        &format!(
            "{}run sequence Main",
            other("capture seen: M::Total = target;")
        ),
    );
    let (text, cause) = only(&with_other);
    assert_eq!(
        stage_read(&cause),
        (
            "target",
            ObligationStage::RegistrationCapture,
            &ObligationOwner::Compensation("Other".to_owned()),
            ReadClass::Foreign
        )
    );
    assert_eq!(at(&text, &cause), "target");

    let (_, cause) = only(&changed(
        "capture order: M::Total = view.total;",
        "capture order: M::Total = target;",
    ));
    assert_eq!(
        stage_read(&cause),
        (
            "target",
            ObligationStage::ProtocolCapture,
            &ObligationOwner::Protocol,
            ReadClass::Foreign
        )
    );
}

/// FR-381-AC-2 for a template: a registration capture that reads itself
/// refuses `forward`.
#[test]
fn a_registration_capture_cannot_read_itself() {
    let (text, cause) = only(&changed(
        "capture target: M::Total = forward.total;",
        "capture target: M::Total = target;",
    ));
    assert_eq!(
        stage_read(&cause),
        (
            "target",
            ObligationStage::RegistrationCapture,
            &ObligationOwner::Compensation("Refund".to_owned()),
            ReadClass::Forward
        )
    );
    assert!(text[cause.span().start..].starts_with("target;"));
}

/// FR-381-AC-1: a read of a name another stage's row owns refuses
/// `missing_declaration`/`missing-name` at the read with `read` `stage`,
/// and a name nothing declares with `unbound`.
#[test]
fn a_name_outside_its_stage_refuses_with_the_stage_read() {
    let cases = [
        (
            changed(
                "capture target: M::Total = forward.total;",
                "capture target: M::Total = recovered.total;",
            ),
            "recovered",
            ObligationStage::RegistrationCapture,
            ReadClass::Stage,
        ),
        (
            changed(
                "not trigger.ready and target >= 0",
                "target >= 0 and forward.total = target",
            ),
            "forward",
            ObligationStage::TriggerGuard,
            ReadClass::Stage,
        ),
        (
            changed(
                "and target >= 0 and not activated\n};",
                "and trigger.ready\n};",
            ),
            "trigger",
            ObligationStage::Retry,
            ReadClass::Stage,
        ),
        (
            changed("recovered.n = 1 and target", "earlier.tally = 1 and target"),
            "earlier",
            ObligationStage::Recovery,
            ReadClass::Stage,
        ),
        (
            changed("not trigger.ready and target >= 0", "ghost.ready"),
            "ghost",
            ObligationStage::TriggerGuard,
            ReadClass::Unbound,
        ),
    ];
    for (declarations, name, stage, read) in cases {
        let (text, cause) = only(&declarations);
        assert_eq!(
            stage_read(&cause),
            (
                name,
                stage,
                &ObligationOwner::Compensation("Refund".to_owned()),
                read
            ),
            "{declarations}"
        );
        assert_eq!(cause.code(), Code::MissingDeclaration);
        assert_eq!(cause.tag(), Some("missing-name"));
        assert_eq!(at(&text, &cause), name);
    }
}

/// FR-381-AC-3: a capture of another type than it declares refuses
/// `type-mismatch` retaining capture, expected and actual; an initializer
/// with an undischarged range obligation refuses `unproved-range` at the
/// operation; an unresolved declared type refuses as a declared type does; a
/// capture named like a visible binder refuses as FR-113 states.
#[trace("TC-1057", "FR-381-AC-3")]
#[test]
fn a_capture_is_checked_against_its_declared_type() {
    let (text, found) = refusals(&plain(TICK, "capture order: M::Absent = view.total;"));
    let anchors: Vec<_> = found
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            CheckCause::ProtocolAnchor(cause) => Some(cause),
            _ => None,
        })
        .collect();
    let [cause] = &anchors[..] else {
        panic!("one declared-type refusal, got {found:?}");
    };
    assert_eq!(cause.code(), Code::MissingDeclaration);
    assert_eq!(cause.tag(), "missing-name");
    assert!(text[cause.span().start..cause.span().end].ends_with("Absent"));
    assert!(obligation_causes(&found).is_empty());

    let (text, cause) = only(&plain(TICK, "capture order: Boolean = view.total;"));
    let ObligationCause::CaptureType {
        capture,
        expected,
        actual,
        ..
    } = &cause
    else {
        panic!("a capture type refusal, got {cause:?}");
    };
    assert_eq!(capture, "order");
    assert_eq!(*expected, ValueType::Boolean);
    assert!(matches!(actual, ValueType::Int(_)), "{actual:?}");
    assert_eq!(cause.code(), Code::IllTyped);
    assert_eq!(cause.tag(), Some("type-mismatch"));
    assert_eq!(at(&text, &cause), "view.total");

    let (text, cause) = only(&plain(TICK, "capture bumped: M::Tally = view.tally + 1;"));
    let ObligationCause::Clause { cause: inner, .. } = &cause else {
        panic!("a clause refusal, got {cause:?}");
    };
    assert!(matches!(**inner, CheckCause::Unproved(_)), "{inner:?}");
    assert_eq!(cause.code(), Code::UndefinedExpression);
    assert_eq!(cause.tag(), Some("unproved-range"));
    assert_eq!(at(&text, &cause), "view.tally + 1");

    let (_, found) = refusals(&changed(
        "capture target: M::Total = forward.total;",
        "capture target: M::Total = forward.total;\ncapture forward: Boolean = true;",
    ));
    let shadows: Vec<_> = found
        .iter()
        .filter(|refusal| {
            matches!(&refusal.cause, CheckCause::ProtocolAnchor(cause)
                if cause.code() == Code::AmbiguousDeclaration && cause.tag() == "ambiguous-name")
        })
        .collect();
    assert_eq!(shadows.len(), 1, "{found:?}");
}

/// FR-381-AC-14: a trigger guard, a recovery predicate, an activation guard
/// and a retry relation that read a field of an object binder refuse
/// `operator-ineligible` at the read naming their stage; the same reads
/// over a record binder, and a registration capture over an object binder,
/// check.
#[trace("TC-1057", "FR-381-AC-14")]
#[test]
fn a_barred_stage_cannot_read_a_field_of_an_object_binder() {
    let stage_of = |cause: &ObligationCause| match cause {
        ObligationCause::ModelStateRead { stage, .. } => *stage,
        other => panic!("a model-state read, got {other:?}"),
    };

    let (text, cause) = only(&changed(
        TRIGGER,
        "activate first (trigger: M::Node) when { trigger.n = 1 } {\n\
         capture activated: Boolean = trigger.n = 1;\n}",
    ));
    assert_eq!(stage_of(&cause), ObligationStage::TriggerGuard);
    assert_eq!(cause.code(), Code::IllTyped);
    assert_eq!(cause.tag(), Some("operator-ineligible"));
    assert_eq!(at(&text, &cause), "trigger.n");

    let (text, cause) = only(&changed(
        RECOVER,
        "recover (recovered: M::Node) { recovered.n = 1 };",
    ));
    assert_eq!(stage_of(&cause), ObligationStage::Recovery);
    assert_eq!(at(&text, &cause), "recovered.n");

    let (text, cause) = only(&changed(
        TICK,
        "on each (opened: M::Node) when (opened.n = 1)",
    ));
    assert_eq!(stage_of(&cause), ObligationStage::ProtocolActivation);
    assert_eq!(at(&text, &cause), "opened.n");

    // An object attempt type is refused at the type, and each barred read
    // of the object retry binders after it, in source order.
    let (text, found) = obligations(&edit(
        &changed("attempts 3 of M::Attempt", "attempts 3 of M::Node"),
        RETRY,
        "retry (earlier: M::Node, later: M::Node) { earlier.n = later.n };",
    ));
    let [ObligationCause::AttemptType { .. }, first @ ObligationCause::ModelStateRead { .. }, second @ ObligationCause::ModelStateRead { .. }] =
        &found[..]
    else {
        panic!("an attempt type refusal and two barred reads, got {found:?}");
    };
    assert_eq!(stage_of(first), ObligationStage::Retry);
    assert_eq!(at(&text, first), "earlier.n");
    assert_eq!(stage_of(second), ObligationStage::Retry);
    assert_eq!(at(&text, second), "later.n");

    let (_, found) = obligations(&plain(
        "on each (opened: M::Node)",
        "capture seen: M::Total = opened.total;",
    ));
    assert!(
        found.is_empty(),
        "a capture may read model state: {found:?}"
    );
}

/// FR-381-AC-15: a trigger guard that reads through a reference-typed field
/// and a recovery predicate that dereferences one refuse
/// `operator-ineligible` naming their stage; an activation capture that
/// reads the same field checks.
#[trace("TC-1057", "FR-381-AC-15")]
#[test]
fn a_barred_stage_cannot_read_through_a_reference() {
    let (text, cause) = only(&changed(
        TRIGGER,
        "activate first (trigger: M::Hop) when { trigger.node.n = 1 } {\n\
         capture activated: Boolean = trigger.ready;\n}",
    ));
    assert!(matches!(
        cause,
        ObligationCause::ModelStateRead {
            stage: ObligationStage::TriggerGuard,
            ..
        }
    ));
    assert_eq!(at(&text, &cause), "trigger.node.n");

    let (text, cause) = only(&changed(
        RECOVER,
        "recover (recovered: M::Hop) { deref(recovered.node).n = 1 };",
    ));
    assert!(matches!(
        cause,
        ObligationCause::ModelStateRead {
            stage: ObligationStage::Recovery,
            ..
        }
    ));
    assert_eq!(at(&text, &cause), "deref(recovered.node)");

    let (_, found) = obligations(&changed(
        TRIGGER,
        "activate first (trigger: M::Hop) when { not trigger.ready } {\n\
         capture activated: Boolean = trigger.node.n = 1;\n}",
    ));
    assert!(found.is_empty(), "{found:?}");
}

/// FR-381: the `over` input is model state; a barred stage that reads it
/// refuses even when the read is not a field read.
#[test]
fn a_barred_stage_cannot_read_the_over_input() {
    let (text, cause) = only(&changed(
        "target >= 0 and not activated\n};",
        "view.total >= 0\n};",
    ));
    assert!(matches!(
        cause,
        ObligationCause::ModelStateRead {
            stage: ObligationStage::Retry,
            ..
        }
    ));
    assert_eq!(at(&text, &cause), "view.total");
}

/// FR-381-AC-5, the forward binder: a template whose forward binder is not
/// the record type of its effect refuses `type-mismatch` retaining
/// compensation, expected and actual; an absent `by` role and an absent
/// operation refuse `missing-name`.
#[test]
fn a_template_is_checked_against_its_effect_role_and_operation() {
    let declarations = edit(
        &changed("(forward: M::Receipt)", "(forward: M::Flag)"),
        "capture target: M::Total = forward.total;",
        "capture target: M::Total = 0;",
    );
    let (text, cause) = only(&declarations);
    let ObligationCause::ForwardType {
        compensation,
        expected,
        actual,
        ..
    } = &cause
    else {
        panic!("a forward type refusal, got {cause:?}");
    };
    assert_eq!(compensation, "Refund");
    assert_ne!(expected, actual);
    assert_eq!(cause.code(), Code::IllTyped);
    assert_eq!(cause.tag(), Some("type-mismatch"));
    assert_eq!(at(&text, &cause), "M::Flag");

    let (text, cause) = only(&changed(
        "by Service on M::Node::step using",
        "by Absent on M::Node::step using",
    ));
    let ObligationCause::MissingRole {
        compensation, name, ..
    } = &cause
    else {
        panic!("a missing role, got {cause:?}");
    };
    assert_eq!((compensation.as_str(), name.as_str()), ("Refund", "Absent"));
    assert_eq!(cause.code(), Code::MissingDeclaration);
    assert_eq!(cause.tag(), Some("missing-name"));
    assert_eq!(at(&text, &cause), "Absent");

    let (text, cause) = only(&changed(
        "on M::Node::step using",
        "on M::Node::missing using",
    ));
    let ObligationCause::MissingOperation { name, .. } = &cause else {
        panic!("a missing operation, got {cause:?}");
    };
    assert_eq!(name, "missing");
    assert_eq!(cause.tag(), Some("missing-name"));
    assert_eq!(at(&text, &cause), "missing");
}

/// FR-381-AC-7, the count: `attempts 0` refuses `type-mismatch` at the
/// count; `attempts 1` and the greatest `u64` check.
#[test]
fn an_attempt_count_of_zero_refuses() {
    let (text, cause) = only(&changed("attempts 3 of", "attempts 0 of"));
    let ObligationCause::ZeroAttempts { compensation, .. } = &cause else {
        panic!("a zero count, got {cause:?}");
    };
    assert_eq!(compensation, "Refund");
    assert_eq!(cause.code(), Code::IllTyped);
    assert_eq!(cause.tag(), Some("type-mismatch"));
    assert_eq!(at(&text, &cause), "0");

    for count in ["1", "18446744073709551615"] {
        let (_, found) = obligations(&changed("attempts 3 of", &format!("attempts {count} of")));
        assert!(found.is_empty(), "attempts {count}: {found:?}");
    }
}

/// FR-381-AC-8: `within [31,30]` and `within [0,*]` refuse `type-mismatch`
/// at the interval; `[0,0]` and `[0,30]` check.
#[trace("TC-1058", "FR-381-AC-8")]
#[test]
fn a_deadline_is_a_closed_interval_with_a_at_most_b() {
    for (written, lower, upper) in [("[31,30]", 31, Some(30)), ("[0,*]", 0, None)] {
        let (text, cause) = only(&changed("within [0,30]", &format!("within {written}")));
        assert_eq!(
            cause,
            ObligationCause::Deadline {
                compensation: "Refund".to_owned(),
                lower,
                upper,
                span: cause.span(),
            }
        );
        assert_eq!(cause.code(), Code::IllTyped);
        assert_eq!(cause.tag(), Some("type-mismatch"));
        assert_eq!(at(&text, &cause), written);
    }
    for written in ["[0,0]", "[0,30]"] {
        let (_, found) = obligations(&changed("within [0,30]", &format!("within {written}")));
        assert!(found.is_empty(), "within {written}: {found:?}");
    }
}

/// FR-381-AC-9: a `retry` binder whose type is not the attempt type refuses
/// `type-mismatch`, `earlier` first; binders of the attempt type check, for
/// a record type and for an event type.
#[trace("TC-1058", "FR-381-AC-9")]
#[test]
fn the_retry_binders_have_the_attempt_type() {
    let (text, found) = obligations(&changed(
        "attempts 3 of M::Attempt",
        "attempts 3 of M::Flag",
    ));
    let binders: Vec<&str> = found
        .iter()
        .map(|cause| match cause {
            ObligationCause::RetryBinder { binder, .. } => binder.as_str(),
            other => panic!("a retry binder refusal, got {other:?}"),
        })
        .collect();
    assert_eq!(binders, ["earlier", "later"]);
    for cause in &found {
        let ObligationCause::RetryBinder {
            compensation,
            expected,
            actual,
            ..
        } = cause
        else {
            unreachable!("matched above");
        };
        assert_eq!(compensation, "Refund");
        assert_ne!(expected, actual);
        assert_eq!(cause.code(), Code::IllTyped);
        assert_eq!(cause.tag(), Some("type-mismatch"));
        assert_eq!(at(&text, cause), "M::Attempt");
    }

    for attempt in ["M::Flag", "M::Pulse"] {
        let declarations = edit(
            &changed(
                "attempts 3 of M::Attempt",
                &format!("attempts 3 of {attempt}"),
            ),
            RETRY,
            &format!(
                "retry (earlier: {attempt}, later: {attempt}) {{ earlier.ready = later.ready }};"
            ),
        );
        let (_, found) = obligations(&declarations);
        assert!(found.is_empty(), "{attempt}: {found:?}");
    }
}

/// FR-381-AC-13: an attempt type that is not a record type or an event type
/// refuses `invalid_model_binding`/`malformed-declaration` at the type,
/// once, and gives no refusal at the retry binders; an unresolved type
/// refuses `missing-name`.
#[test]
fn an_attempt_type_must_be_a_record_or_an_event() {
    let relation_true = |attempt: &str| {
        edit(
            &changed(
                "attempts 3 of M::Attempt",
                &format!("attempts 3 of {attempt}"),
            ),
            RETRY,
            &format!("retry (earlier: {attempt}, later: {attempt}) {{ true }};"),
        )
    };
    // The variant type `M::Mode` is a named attempt type too (FR-381-AC-13).
    // TC-1058 row 15c's inline `Set<M::Total>[0, 3]` attempt type is not
    // reachable: the `attempts N of` member's grammar takes a model name
    // (QSpec FR-239's EBNF), so S1 refuses an inline type.
    for (attempt, name) in [
        ("M::Node", Some("M::Node")),
        ("M::Total", Some("M::Total")),
        ("M::Mode", Some("M::Mode")),
    ] {
        let (text, cause) = only(&relation_true(attempt));
        let ObligationCause::AttemptType {
            compensation,
            name: written,
            ..
        } = &cause
        else {
            panic!("an attempt type refusal, got {cause:?}");
        };
        assert_eq!(compensation, "Refund");
        assert_eq!(written.as_deref(), name, "{attempt}");
        assert_eq!(cause.code(), Code::InvalidModelBinding);
        assert_eq!(cause.tag(), Some("malformed-declaration"));
        assert_eq!(at(&text, &cause), attempt);
    }

    let (text, cause) = only(&relation_true("M::Absent"));
    assert!(matches!(cause, ObligationCause::AttemptTypeName { .. }));
    assert_eq!(cause.code(), Code::MissingDeclaration);
    assert_eq!(cause.tag(), Some("missing-name"));
    assert!(at(&text, &cause).ends_with("Absent"));
}

/// FR-381-AC-10: the retry relation is a total Boolean; without the bound
/// that makes `earlier.tally + 1` land in `Tally` it refuses
/// `unproved-range`, and a non-Boolean relation refuses `non-boolean-root`.
#[trace("TC-1058", "FR-381-AC-10")]
#[test]
fn the_retry_relation_is_a_total_boolean() {
    let (text, cause) = only(&changed(
        "earlier.tally < 3 and later.tally = earlier.tally + 1\nand target",
        "later.tally = earlier.tally + 1\nand target",
    ));
    let ObligationCause::Clause {
        stage,
        cause: inner,
        ..
    } = &cause
    else {
        panic!("a clause refusal, got {cause:?}");
    };
    assert_eq!(*stage, ObligationStage::Retry);
    assert!(matches!(**inner, CheckCause::Unproved(_)), "{inner:?}");
    assert_eq!(cause.code(), Code::UndefinedExpression);
    assert_eq!(cause.tag(), Some("unproved-range"));
    assert_eq!(at(&text, &cause), "earlier.tally + 1");

    let (text, cause) = only(&changed(
        RETRY,
        "retry (earlier: M::Attempt, later: M::Attempt) { earlier.tally };",
    ));
    assert_eq!(cause.code(), Code::IllTyped);
    assert_eq!(cause.tag(), Some("non-boolean-root"));
    assert_eq!(at(&text, &cause), "earlier.tally");

    // The base relation reads a registration capture and an activation
    // capture, and checks.
    let (_, found) = obligations(BASE);
    assert!(found.is_empty(), "{found:?}");
}

/// FR-381, recovery: a non-Boolean predicate refuses `non-boolean-root`, an
/// undischarged range obligation `unproved-range`, and an unresolved binder
/// type refuses as a declared type does.
#[test]
fn the_recovery_predicate_is_a_total_boolean() {
    let (text, cause) = only(&changed(
        RECOVER,
        "recover (recovered: M::Receipt) { recovered.n };",
    ));
    assert_eq!(cause.tag(), Some("non-boolean-root"));
    assert_eq!(at(&text, &cause), "recovered.n");

    let (text, cause) = only(&changed(
        RECOVER,
        "recover (recovered: M::Receipt) { recovered.tally = recovered.tally + 1 };",
    ));
    assert_eq!(cause.code(), Code::UndefinedExpression);
    assert_eq!(cause.tag(), Some("unproved-range"));
    assert_eq!(at(&text, &cause), "recovered.tally + 1");

    // The unresolved binder type is `protocol_clause::content`'s refusal,
    // `missing_declaration`/`missing-name` at the type, and the only one.
    let (text, found) = refusals(&changed(
        RECOVER,
        "recover (recovered: M::Absent) { true };",
    ));
    assert!(obligation_causes(&found).is_empty(), "{found:?}");
    let [refusal] = found.as_slice() else {
        panic!("one refusal: {found:?}");
    };
    let CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
        panic!("a protocol cause: {refusal:?}");
    };
    let ProtocolAnchorCause::BinderType { binder, span, .. } = cause.as_ref() else {
        panic!("a binder-type cause: {cause:?}");
    };
    assert_eq!(binder, "recovered");
    assert_eq!(refusal.cause.code().as_str(), "missing_declaration");
    assert_eq!(refusal.cause.cause(), Some("missing-name"));
    assert_eq!(&text[span.start..span.end], "M::Absent");
}

/// FR-381-AC-10, the typing it rests on: `=` between an `Int[..]` operand and
/// an `Integer`-typed sum admits the sum into the interval with a range
/// obligation, so the guard decides the relation; a bare `Integer` operand
/// and an explicit `convert<Integer>` keep the equality matrix's rules.
#[trace("TC-1058", "FR-381-AC-10")]
#[test]
fn an_int_equals_a_sum_is_range_obligated_and_other_mixes_keep_the_matrix() {
    let (_, found) = obligations(BASE);
    assert!(found.is_empty(), "the guarded sum checks: {found:?}");

    let (text, cause) = only(&changed(
        "earlier.tally < 3 and later.tally = earlier.tally + 1\nand target",
        "later.tally = earlier.tally + 1\nand target",
    ));
    assert_eq!(cause.tag(), Some("unproved-range"));
    assert_eq!(at(&text, &cause), "earlier.tally + 1");

    let wide = changed(
        "capture order: M::Total = view.total;",
        "capture order: M::Total = view.total;\n capture wide: Integer = view.total + 1;",
    );
    let (_, cause) = only(&edit(
        &wide,
        "later.tally = earlier.tally + 1\nand target",
        "later.tally = wide\nand target",
    ));
    assert_eq!(cause.tag(), Some("type-mismatch"), "{cause:?}");

    let converted = changed(
        "earlier.tally < 3 and later.tally = earlier.tally + 1",
        "convert<Integer>(later.tally) = earlier.tally + 1",
    );
    let (_, found) = obligations(&converted);
    assert!(found.is_empty(), "{found:?}");
}

/// FR-381: a refusal inside one template does not hide another's, and the
/// refusals come in source order.
#[test]
fn every_template_is_checked_and_refusals_come_in_source_order() {
    let second = other("capture seen: M::Total = 0;").replacen("attempts 3 of", "attempts 0 of", 1);
    let declarations = edit(
        &changed("run sequence Main", &format!("{second}run sequence Main")),
        "attempts 3 of",
        "attempts 0 of",
    );
    let (_, found) = obligations(&declarations);
    let names: Vec<&str> = found
        .iter()
        .map(|cause| match cause {
            ObligationCause::ZeroAttempts { compensation, .. } => compensation.as_str(),
            other => panic!("a zero count, got {other:?}"),
        })
        .collect();
    assert_eq!(names, ["Refund", "Other"]);
}

/// The catalog pair of every cause is the one FR-381 and FR-218 name.
#[test]
fn every_cause_maps_to_its_catalog_pair() {
    let span = Span { start: 0, end: 1 };
    let compensation = || "Refund".to_owned();
    let cases = [
        (
            ObligationCause::StageRead {
                name: "x".to_owned(),
                stage: ObligationStage::Retry,
                owner: ObligationOwner::Protocol,
                read: ReadClass::Unbound,
                span,
            },
            Code::MissingDeclaration,
            "missing-name",
        ),
        (
            ObligationCause::ModelStateRead {
                stage: ObligationStage::Retry,
                owner: ObligationOwner::Protocol,
                span,
            },
            Code::IllTyped,
            "operator-ineligible",
        ),
        (
            ObligationCause::ZeroAttempts {
                compensation: compensation(),
                span,
            },
            Code::IllTyped,
            "type-mismatch",
        ),
        (
            ObligationCause::AmbiguousOperation {
                compensation: compensation(),
                name: "step".to_owned(),
                span,
            },
            Code::AmbiguousDeclaration,
            "ambiguous-name",
        ),
        (
            ObligationCause::AttemptType {
                compensation: compensation(),
                name: None,
                span,
            },
            Code::InvalidModelBinding,
            "malformed-declaration",
        ),
    ];
    for (cause, code, tag) in cases {
        assert_eq!(cause.code(), code, "{cause:?}");
        assert_eq!(cause.tag(), Some(tag), "{cause:?}");
    }
}

fn site(later: &[&str], template: Option<usize>) -> Site {
    Site {
        stage: ObligationStage::RegistrationCapture,
        owner: ObligationOwner::Protocol,
        template,
        bound: Vec::new(),
        input: None,
        later: later.iter().map(|name| (*name).to_owned()).collect(),
    }
}

/// The class of a refused read follows FR-381's definitions: a later
/// capture of the stage or the capture itself is `forward` before anything
/// else, another template's name is `foreign`, a name that exists is
/// `stage`, any other is `unbound`.
#[test]
fn a_refused_read_is_classified_in_the_order_the_criteria_state() {
    let universe = Universe {
        protocol: ["order".to_owned()].into_iter().collect(),
        templates: vec![
            ["target".to_owned(), "later".to_owned()]
                .into_iter()
                .collect(),
            ["seen".to_owned()].into_iter().collect(),
        ],
        nodes: ["applied".to_owned()].into_iter().collect(),
    };
    let in_first = site(&["later"], Some(0));
    assert_eq!(universe.classify("later", &in_first), ReadClass::Forward);
    assert_eq!(universe.classify("seen", &in_first), ReadClass::Foreign);
    assert_eq!(universe.classify("target", &in_first), ReadClass::Stage);
    assert_eq!(universe.classify("order", &in_first), ReadClass::Stage);
    assert_eq!(universe.classify("applied", &in_first), ReadClass::Stage);
    assert_eq!(universe.classify("ghost", &in_first), ReadClass::Unbound);

    let at_protocol = site(&[], None);
    assert_eq!(
        universe.classify("target", &at_protocol),
        ReadClass::Foreign
    );
    assert_eq!(universe.classify("order", &at_protocol), ReadClass::Stage);
}
