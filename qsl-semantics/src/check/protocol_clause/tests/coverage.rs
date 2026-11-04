//! The unchecked-content refusal and the grammar-required content checks.

use ix_trace_rs::trace;

use super::attempt_fixture::attempt_flow;
use super::{assemble, checks, only_cause, recovery_flow, refusals};
use crate::check::refusal::ProtocolAnchorCause;
use quire_semantic_value::checking::CheckingLimits;

fn checks_with_header(header: &str, source: &str) {
    let (_, assembled) = super::assemble_with(header, source);
    assembled
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{source}: {refusals:?}"));
}

fn refusals_with_header(header: &str, source: &str) -> (String, Vec<crate::check::CheckRefusal>) {
    let (text, assembled) = super::assemble_with(header, source);
    let refusals = assembled
        .check(CheckingLimits::default())
        .expect_err("the wrong profile kind refuses");
    (text, refusals)
}

fn recovery_flow_with_await(await_form: &str) -> String {
    recovery_flow("Main::Committed", "").replacen(
        "effect Applied of Tried as (applied: Boolean) { true };",
        &format!("effect Applied of Tried as (applied: Boolean) {{ true }};\n{await_form}"),
        1,
    )
}

/// FR-508 / TC-1303: each protocol use resolves its own alias to the required
/// profile kind, and a wrong-kind selection refuses at that alias with the
/// selected identity and required kind retained.
#[trace(
    "FR-508-AC-1",
    "FR-508-AC-2",
    "FR-508-AC-3",
    "FR-508-AC-4",
    "FR-508-AC-5",
    "TC-1303"
)]
#[test]
fn protocol_use_sites_refuse_a_profile_of_the_wrong_kind() {
    // Profile declarations precede the `model` declaration of the header.
    let (profiles, model) = super::HEADER
        .split_once("model ")
        .expect("the header declares its model");
    let header = format!(
        "{profiles}profile i = \"quire.temporal.infinite-trace/v1\";\n\
         profile tm = \"quire.temporal.timed/v1\";\n\
         profile s = \"quire.state.core/v1\";\n\
         model {model}"
    );
    let protocol = attempt_flow("Flow", "").replace("using pp", "using v");
    let await_form = "await Wait after Applied using v clock \"ticks\" within [0,1] \
        match event Seen by R for Undo as (seen: Boolean) { true }; \
        then sequence Then { } timeout sequence Timeout { }";
    let check_form = "check Fine using pp { true };";
    let cases = [
        (protocol, "v", "quire.value.complete/v1", "protocol profile"),
        (
            attempt_flow("Flow", "").replace("using pp", "using t"),
            "t",
            "quire.temporal.infinite-trace/v1",
            "protocol profile",
        ),
        (
            attempt_flow("Flow", "").replace("using pp", "using e"),
            "e",
            "quire.temporal.event-position.false-extension/v1",
            "protocol profile",
        ),
        (
            attempt_flow("Flow", "").replace("using pp", "using tm"),
            "tm",
            "quire.temporal.timed/v1",
            "protocol profile",
        ),
        (
            attempt_flow("Flow", "").replace("using pp", "using s"),
            "s",
            "quire.state.core/v1",
            "protocol profile",
        ),
        (
            recovery_flow_with_await(await_form),
            "v",
            "quire.value.complete/v1",
            "temporal profile",
        ),
        (
            recovery_flow_with_await(&await_form.replace("using v", "using pp")),
            "pp",
            "quire.protocol.finite-global/v1",
            "temporal profile",
        ),
        (
            recovery_flow_with_await(&await_form.replace("using v", "using i")),
            "i",
            "quire.temporal.infinite-trace/v1",
            "temporal profile",
        ),
        (
            recovery_flow("Main::Committed", check_form),
            "pp",
            "quire.protocol.finite-global/v1",
            "state-bearing layer",
        ),
        (
            recovery_flow("Main::Committed", "check Fine using e { true };"),
            "e",
            "quire.temporal.event-position.false-extension/v1",
            "state-bearing layer",
        ),
        (
            recovery_flow("Main::Committed", "check Fine using tm { true };"),
            "tm",
            "quire.temporal.timed/v1",
            "state-bearing layer",
        ),
        (
            recovery_flow("Main::Committed", "check Fine using t { true };"),
            "t",
            "quire.temporal.infinite-trace/v1",
            "state-bearing layer",
        ),
        (
            recovery_flow("Main::Committed", "").replace(
                "on M::Actor::op using e clock",
                "on M::Actor::op using pp clock",
            ),
            "pp",
            "quire.protocol.finite-global/v1",
            "temporal profile",
        ),
        (
            recovery_flow("Main::Committed", "").replace(
                "on M::Actor::op using e clock",
                "on M::Actor::op using v clock",
            ),
            "v",
            "quire.value.complete/v1",
            "temporal profile",
        ),
        (
            recovery_flow("Main::Committed", "").replace(
                "on M::Actor::op using e clock",
                "on M::Actor::op using t clock",
            ),
            "t",
            "quire.temporal.infinite-trace/v1",
            "temporal profile",
        ),
        (
            recovery_flow("Main::Committed", "").replace(
                "on M::Actor::op using e clock",
                "on M::Actor::op using i clock",
            ),
            "i",
            "quire.temporal.infinite-trace/v1",
            "temporal profile",
        ),
    ];

    for (source, alias, selected, required) in cases {
        let (text, found) = refusals_with_header(&header, &source);
        let matches: Vec<_> = found
            .iter()
            .filter(|refusal| refusal.cause.code().as_str() == "unknown_profile")
            .collect();
        assert_eq!(matches.len(), 1, "{source}\n{found:?}");
        assert_eq!(
            matches[0].cause.cause(),
            Some("wrong-selection-role"),
            "{source}"
        );
        let ProtocolAnchorCause::WrongProfileKind {
            alias: actual_alias,
            selected: actual_selected,
            required: actual_required,
            span,
        } = (match &matches[0].cause {
            crate::check::CheckCause::ProtocolAnchor(cause) => cause.as_ref(),
            other => panic!("a protocol anchor refusal, got {other:?}"),
        })
        else {
            panic!("a wrong profile kind, got {:?}", matches[0].cause);
        };
        assert_eq!(actual_alias, alias, "{source}");
        assert_eq!(actual_selected, selected, "{source}");
        assert_eq!(*actual_required, required, "{source}");
        assert_eq!(&text[span.start..span.end], alias, "{source}");
    }
}

/// FR-508-AC-3/4 (TC-1303): every identity in the closed admission sets is
/// accepted at the use sites that need it.
#[trace("FR-508-AC-3", "FR-508-AC-4", "TC-1303")]
#[test]
fn every_admitted_temporal_and_state_profile_is_accepted_at_its_use_site() {
    let temporal = [
        "quire.temporal.event-position.false-extension/v1",
        "quire.temporal.fixed-sample.false-extension/v1",
        "quire.temporal.timestamped-event.finite-window/v1",
        "quire.temporal.timed/v1",
    ];
    for identity in temporal {
        let header = super::attempt_fixture::HEADER.replace(
            "profile e = \"quire.temporal.event-position.false-extension/v1\";",
            &format!("profile e = \"{identity}\";"),
        );
        let await_form = "await Wait after Applied using e clock \"ticks\" within [0,1] \
            match event Seen by R for Undo as (seen: Boolean) { true }; \
            then sequence Then { } timeout sequence Timeout { }";
        checks_with_header(&header, &recovery_flow_with_await(await_form));
        // FR-508-AC-3 requires every admitted temporal identity at both
        // protocol temporal use sites, including the compensation template.
        checks_with_header(&header, &recovery_flow("Main::Committed", ""));
    }

    let state_layers = [
        "quire.state.core/v1",
        "quire.state.queries/v1",
        "quire.state.graph/v1",
        "quire.value.complete/v1",
        "quire.model.complete/v1",
        "quire.temporal.complete/v1",
    ];
    for identity in state_layers {
        let header = super::attempt_fixture::HEADER.replace(
            "profile v = \"quire.value.complete/v1\";",
            &format!("profile v = \"{identity}\";"),
        );
        checks_with_header(
            &header,
            &recovery_flow("Main::Committed", "check Fine using v { true };"),
        );
    }
}

/// FR-508-AC-5 (TC-1303): all wrong-kind sites are returned together in
/// source order, leaving the package rejected.
#[trace("FR-508-AC-5", "TC-1303")]
#[test]
fn wrong_kind_protocol_and_check_aliases_refuse_in_source_order() {
    let source = recovery_flow("Main::Committed", "check Fine using e { true };").replace(
        "protocol RecoveryFlow using pp",
        "protocol RecoveryFlow using v",
    );
    let (text, refusals) = refusals(&source);
    let wrong: Vec<_> = refusals
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            crate::check::CheckCause::ProtocolAnchor(cause) => match cause.as_ref() {
                ProtocolAnchorCause::WrongProfileKind { span, .. } => Some(span),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(wrong.len(), 2, "{refusals:?}");
    assert!(wrong[0].start < wrong[1].start, "{wrong:?}");
    assert_eq!(&text[wrong[0].start..wrong[0].end], "v");
    assert_eq!(&text[wrong[1].start..wrong[1].end], "e");
}

/// FR-508: an absent alias keeps the existing `MissingProfile` refusal.
#[trace("FR-508-AC-5", "TC-1303")]
#[test]
fn a_missing_protocol_profile_alias_keeps_missing_profile() {
    let source = format!(
        "function unusedFn using v(): Boolean pure {{ true }}\n{}",
        attempt_flow("Flow", "").replace("using pp", "using unusedFn")
    );
    let (_, refusals) = refusals(&source);
    assert!(refusals.iter().any(|refusal| matches!(
        &refusal.cause,
        crate::check::CheckCause::ProtocolAnchor(cause)
            if matches!(cause.as_ref(), ProtocolAnchorCause::MissingProfile { alias, .. } if alias == "unusedFn")
    )), "{refusals:?}");
}

/// SR-770 FND-001, now that every node kind and body is checked: a
/// protocol whose anchors (FR-113) and attempt bindings (FR-114) all resolve
/// and that holds an `effect`, an `event` and a `commit` checks, and a body
/// that reads its node's own binder checks too, while the same protocol
/// with a `related by` clause naming no relationship refuses
/// `missing_declaration`/`missing-name` at the name (FR-218
/// "Relationships") rather than compiling with the clause unchecked.
#[trace("TC-511", "FR-113")]
#[test]
fn a_resolving_protocol_with_unchecked_content_still_refuses() {
    let source = recovery_flow("Main::Committed", "");
    checks(&source);
    checks(&source.replacen(
        "as (applied: Boolean) { true }",
        "as (applied: Boolean) { applied }",
        1,
    ));
    let source = source.replacen(
        "as (applied: Boolean) { true }",
        "as (applied: Boolean) related by Pays (applied, applied) { true }",
        1,
    );
    let (refusal, cause) = only_cause(&source);
    assert_eq!(refusal.cause.code().as_str(), "missing_declaration");
    assert_eq!(refusal.cause.cause(), Some("missing-name"));
    let ProtocolAnchorCause::MissingRelated { name, span, .. } = cause else {
        panic!("a missing relationship, got {cause:?}");
    };
    assert_eq!(name, "Pays");
    let (text, _) = assemble(&source);
    assert_eq!(&text[span.start..span.end], "Pays");
}

/// A protocol made only of parts the checker covers in full
/// (a role, a `sequence`, an `attempt` and a `finish`, each body a bare
/// Boolean literal) checks through the whole pipeline, and its checked
/// attempt names the resolved operation's own anchor and frame nodes.
#[trace("TC-513", "FR-114")]
#[test]
fn a_fully_covered_attempt_protocol_checks() {
    let graph = checks(&attempt_flow("Flow", ""));
    let [protocol] = graph.protocols() else {
        panic!("one protocol: {:?}", graph.protocols());
    };
    assert_eq!(protocol.name, "Flow");
    let [attempt] = protocol.attempts.as_slice() else {
        panic!("one attempt: {:?}", protocol.attempts);
    };
    assert_ne!(attempt.anchor, attempt.frame);
}

/// One `content` refusal per unchecked or ill-formed part of an
/// otherwise fully covered protocol (SR-770 FND-001), each checked
/// against its own catalog code, tag and cause.
#[trace("TC-513", "FR-114")]
#[test]
fn each_unchecked_part_of_an_attempt_protocol_refuses() {
    let base = attempt_flow("Flow", "");
    let cases: [(&str, &str, &str, &str); 7] = [
        // An ill-typed body is refused by the clause checker.
        (
            "as (tried: Boolean) { true }",
            "as (tried: Boolean) { 1 + true }",
            "ill_typed",
            "type-mismatch",
        ),
        // A `send` via a name that is no channel (FR-218 "Channels").
        (
            "contracts [] as (tried: Boolean) { true };\n",
            "contracts [] as (tried: Boolean) { true };\n\
             send Ping via R as (ping: Boolean) { true };\n",
            "missing_declaration",
            "missing-name",
        ),
        (
            "role R on M::Actor;",
            "role R on Nope::Actor;",
            "missing_declaration",
            "missing-name",
        ),
        (
            "attempt Tried by R",
            "attempt Tried by Q",
            "missing_declaration",
            "missing-name",
        ),
        (
            "(tried: Boolean)",
            "(tried: Undeclared)",
            "missing_declaration",
            "missing-name",
        ),
        (
            "using pp over",
            "using M over",
            "missing_declaration",
            "missing-name",
        ),
        (
            "role R on M::Actor;",
            "role R on M::Actor;\nrole R on M::Actor;",
            "ambiguous_declaration",
            "ambiguous-name",
        ),
    ];
    for (from, to, code, tag) in cases {
        let source = base.replacen(from, to, 1);
        assert_ne!(source, base, "{from}");
        let (_, found) = refusals(&source);
        assert!(!found.is_empty(), "{source}");
        for refusal in &found {
            assert_eq!(refusal.cause.code().as_str(), code, "{source}: {found:?}");
            assert_eq!(refusal.cause.cause(), Some(tag), "{source}: {found:?}");
        }
    }
}

/// FR-508-AC-5 / TC-1303: an absent compensation profile alias retains the
/// existing `MissingProfile` refusal at the alias.
#[trace("FR-508-AC-5", "TC-1303", "QSpec-FR-464")]
#[test]
fn a_compensation_without_its_temporal_alias_keeps_missing_profile() {
    let source = recovery_flow("Main::Committed", "");
    checks(&source);
    let header = super::HEADER
        .replace(
            "profile e = \"quire.temporal.event-position.false-extension/v1\";\n",
            "",
        )
        .replace("profile t = \"quire.temporal.infinite-trace/v1\";\n", "");
    assert_ne!(
        header,
        super::HEADER,
        "the fixture header has no temporal profile"
    );
    let (text, assembled) = super::assemble_with(&header, &source);
    let found = assembled
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("a template whose using alias is not selected refuses");
    let missing_profile = found.iter().find_map(|refusal| {
        let crate::check::CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
            return None;
        };
        match cause.as_ref() {
            ProtocolAnchorCause::MissingProfile { alias, span } if alias == "e" => {
                Some((refusal, span))
            }
            _ => None,
        }
    });
    let Some((refusal, span)) = missing_profile else {
        panic!("a missing profile alias refusal: {found:?}");
    };
    assert_eq!(refusal.cause.code().as_str(), "missing_declaration");
    assert_eq!(refusal.cause.cause(), Some("missing-name"));
    assert!(
        !found.iter().any(|refusal| matches!(
            &refusal.cause,
            crate::check::CheckCause::ProtocolAnchor(cause)
                if matches!(cause.as_ref(), ProtocolAnchorCause::WrongProfileKind { .. })
        )),
        "an absent profile alias is not a wrong-kind selection: {found:?}"
    );
    assert_eq!(&text[span.start..span.end], "e");
}
