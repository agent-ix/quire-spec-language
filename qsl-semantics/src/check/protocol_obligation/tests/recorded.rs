//! FR-381's recorded outputs over a protocol that checks as a whole, and the
//! commit-dominance rule over the compiled causal edge relation (TC-1057,
//! TC-1058).

use ix_trace_rs::trace;
use qsl_forms::ProtocolNodeKind;
use qsl_foundation::diagnostic::Code;

use super::{
    ambiguous_population_model, assemble, assemble_against, changed, checked, edit,
    obligation_causes, only, other, refusals, BASE,
};
use crate::check::protocol_clause::ProtocolNodeId;
use crate::check::{
    CaptureOwner, CaptureStage, CheckCause, CheckedActivation, CheckedCommit, CheckedGraph,
    ObligationCause, ProtocolAnchorCause,
};

/// The protocol profile FR-381-AC-11's refusal names.
const PROTOCOL_PROFILE: &str = "quire.protocol.finite-global/v1";

/// The id of the node `name` of `kind` in the one protocol `declarations`
/// holds.
fn node(declarations: &str, name: &str, kind: ProtocolNodeKind) -> ProtocolNodeId {
    let (_, assembled) = assemble(declarations);
    let protocol = &assembled.protocols[0];
    let index = protocol
        .declarations
        .iter()
        .position(|declaration| declaration.name.name == name && declaration.kind == kind)
        .unwrap_or_else(|| panic!("a {kind:?} node {name}"));
    ProtocolNodeId(index)
}

/// FR-381-AC-1, the recorded half: TC-1057's refund template checks as a
/// whole, and the checked clause holds the protocol capture, the template's
/// registration and activation captures with their stages and owner, the
/// `OnEach` activation and the one compensation naming its two captures.
#[trace("TC-1057", "FR-381-AC-1")]
#[test]
fn the_refund_template_checks_and_records_every_stage() {
    let graph = checked(BASE);
    let protocol = &graph.protocols()[0];
    let obligations = &protocol.obligations;
    let CheckedActivation::OnEach { binder, guard, .. } = &obligations.activation else {
        panic!("an OnEach activation: {:?}", obligations.activation);
    };
    assert_eq!(binder, "opened");
    assert!(guard.is_some());
    let stages: Vec<(&str, CaptureStage, CaptureOwner)> = obligations
        .captures
        .iter()
        .map(|capture| (capture.name.as_str(), capture.stage, capture.owner))
        .collect();
    assert_eq!(
        stages,
        [
            ("order", CaptureStage::Protocol, CaptureOwner::Protocol),
            (
                "target",
                CaptureStage::Registration,
                CaptureOwner::Compensation(0)
            ),
            (
                "activated",
                CaptureStage::Activation,
                CaptureOwner::Compensation(0)
            ),
        ]
    );
    let [compensation] = obligations.compensations.as_slice() else {
        panic!("one compensation: {:?}", obligations.compensations);
    };
    assert_eq!(compensation.name, "Refund");
    assert_eq!(compensation.registration_captures, [1]);
    assert_eq!(compensation.activation_captures, [2]);
    assert_eq!(compensation.trigger.name, "trigger");
    assert_eq!(compensation.retry.earlier.name, "earlier");
    assert_eq!(compensation.retry.later.name, "later");
    assert_eq!(compensation.recover.binder.name, "recovered");
}

/// The frame records keyed at `frame` in `graph`.
fn frame_records(graph: &CheckedGraph, frame: quire_exact::NodeKey) -> usize {
    let frame = qsl_foundation::digest::WireNodeId::from_digest(*frame.as_bytes());
    graph
        .requirements()
        .keys()
        .filter(|key| key.node() == frame)
        .count()
}

/// FR-381-AC-5, the recorded half: the checked compensation holds its
/// effect's id and the operation's anchor and frame node, and an attempt and
/// a template naming one operation share one anchor node, one frame node
/// and one frame record. A template whose operation no attempt names is
/// bound to that operation's own nodes.
#[trace("TC-1058", "FR-381-AC-5")]
#[test]
fn a_template_shares_the_operations_anchor_and_frame() {
    let graph = checked(BASE);
    let protocol = &graph.protocols()[0];
    let [compensation] = protocol.obligations.compensations.as_slice() else {
        panic!("one compensation");
    };
    assert_eq!(
        compensation.effect,
        node(BASE, "Applied", ProtocolNodeKind::Effect)
    );
    assert_eq!(compensation.operation.declaration.name(), "step");
    let [attempt] = protocol.attempts.as_slice() else {
        panic!("one attempt");
    };
    let [bound] = protocol.compensation_operations.as_slice() else {
        panic!(
            "one template binding: {:?}",
            protocol.compensation_operations
        );
    };
    assert_eq!(bound.anchor, attempt.anchor);
    assert_eq!(bound.frame, attempt.frame);
    assert_ne!(bound.anchor, bound.frame);
    assert_eq!(frame_records(&graph, bound.frame), 1);

    // The same template on another operation than the attempt's: its own
    // anchor and frame, each with its one frame record.
    let source = changed(
        "by Service on M::Node::step using v",
        "by Service on M::Node::undo using v",
    );
    let graph = checked(&source);
    let protocol = &graph.protocols()[0];
    let [attempt] = protocol.attempts.as_slice() else {
        panic!("one attempt");
    };
    let [bound] = protocol.compensation_operations.as_slice() else {
        panic!("one template binding");
    };
    assert_ne!(bound.anchor, attempt.anchor);
    assert_ne!(bound.frame, attempt.frame);
    assert_eq!(frame_records(&graph, bound.frame), 1);
    assert_eq!(frame_records(&graph, attempt.frame), 1);
}

/// FR-381-AC-6: two templates for one effect check and give two
/// registrations; two templates of one name refuse `ambiguous-name` at both
/// declarations; a template `for` an `attempt`, `send` or `event` node, and
/// a `commit` that names an `effect`, refuse as FR-113-AC-7 states.
#[trace("TC-1058", "FR-381-AC-6")]
#[test]
fn two_templates_register_twice_and_a_wrong_target_refuses() {
    let two = changed(
        "run sequence Main",
        &format!("{}run sequence Main", other("")),
    );
    let graph = checked(&two);
    let compensations = &graph.protocols()[0].obligations.compensations;
    let effect = node(&two, "Applied", ProtocolNodeKind::Effect);
    let names: Vec<(&str, ProtocolNodeId)> = compensations
        .iter()
        .map(|compensation| (compensation.name.as_str(), compensation.effect))
        .collect();
    assert_eq!(names, [("Refund", effect), ("Other", effect)]);

    let same_name = edit(&two, "compensate Other for", "compensate Refund for");
    let (text, found) = refusals(&same_name);
    let ambiguous: Vec<&str> = found
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            CheckCause::ProtocolAnchor(cause) => match cause.as_ref() {
                ProtocolAnchorCause::Ambiguous { name, span, .. } if name == "Refund" => {
                    Some(&text[span.start..span.end])
                }
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(ambiguous, ["Refund", "Refund"], "{found:?}");
    for refusal in &found {
        if let CheckCause::ProtocolAnchor(cause) = &refusal.cause {
            if let ProtocolAnchorCause::Ambiguous { .. } = cause.as_ref() {
                assert_eq!(refusal.cause.code(), Code::AmbiguousDeclaration);
                assert_eq!(refusal.cause.cause(), Some("ambiguous-name"));
            }
        }
    }

    let with_nodes = changed(
        "commit Committed by Service as (committed: M::Flag) { true };",
        "commit Committed by Service as (committed: M::Flag) { true };\n\
         event Noted by Service as (noted: M::Flag) { true };",
    );
    for (target, actual) in [("Main::Tried", "attempt"), ("Main::Noted", "event")] {
        let source = edit(
            &with_nodes,
            "compensate Refund for Main::Applied",
            &format!("compensate Refund for {target}"),
        );
        let (text, found) = refusals(&source);
        let wrong: Vec<(&str, &str)> = found
            .iter()
            .filter_map(|refusal| match &refusal.cause {
                CheckCause::ProtocolAnchor(cause) => match cause.as_ref() {
                    ProtocolAnchorCause::WrongKind {
                        site,
                        actual: found,
                        span,
                        ..
                    } => {
                        assert_eq!(*site, "compensate-for");
                        assert_eq!(*found, actual);
                        Some((*found, &text[span.start..span.end]))
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(wrong, [(actual, target)], "{found:?}");
    }
    let commit_effect = changed("commit Main::Committed;", "commit Main::Applied;");
    let (_, found) = refusals(&commit_effect);
    assert!(
        found.iter().any(|refusal| matches!(
            &refusal.cause,
            CheckCause::ProtocolAnchor(cause) if matches!(
                cause.as_ref(),
                ProtocolAnchorCause::WrongKind { site: "compensate-commit", actual: "effect", .. }
            )
        )),
        "{found:?}"
    );
}

/// FR-381-AC-6, a `send` target: a template `for` a `send` refuses as
/// FR-113-AC-7 states.
#[trace("TC-1058", "FR-381-AC-6")]
#[test]
fn a_template_for_a_send_refuses_at_its_target() {
    let source = edit(
        &changed(
            "role Service on M::Node;",
            "role Service on M::Node;\n\
             channel Wire from Service to Service carries M::Flag ordering unordered \
             delivery unknown capacity 1 overflow reject;",
        ),
        "commit Committed by Service as (committed: M::Flag) { true };",
        "commit Committed by Service as (committed: M::Flag) { true };\n\
         send Pinged via Wire as (pinged: M::Flag) { true };",
    );
    let source = edit(
        &source,
        "compensate Refund for Main::Applied",
        "compensate Refund for Main::Pinged",
    );
    let (text, found) = refusals(&source);
    let wrong: Vec<&str> = found
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            CheckCause::ProtocolAnchor(cause) => match cause.as_ref() {
                ProtocolAnchorCause::WrongKind {
                    site: "compensate-for",
                    actual: "send",
                    span,
                    ..
                } => Some(&text[span.start..span.end]),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(wrong, ["Main::Pinged"], "{found:?}");
}

/// FR-381-AC-7, the recorded half: `attempts 1` and `attempts
/// 18446744073709551615` check and the checked compensation holds exactly
/// that count.
#[trace("TC-1058", "FR-381-AC-7")]
#[test]
fn the_attempt_count_is_recorded_exactly() {
    for count in [1, u64::MAX] {
        let graph = checked(&changed("attempts 3 of", &format!("attempts {count} of")));
        let [compensation] = graph.protocols()[0].obligations.compensations.as_slice() else {
            panic!("one compensation");
        };
        assert_eq!(compensation.maximum_attempts, count);
    }
}

/// `BASE` with its `run` tree replaced by `run`, under a header that selects
/// the protocol profile.
fn with_run(run: &str) -> String {
    let start = BASE.find("run sequence Main").expect("the run tree");
    let end = BASE.find("finish Closed").expect("the finish");
    format!(
        "profile p = \"{PROTOCOL_PROFILE}\";\n{}{run}\n{}",
        &BASE[..start],
        &BASE[end..]
    )
}

const TRIED: &str =
    "attempt Tried by Service on M::Node::step contracts [] as (attempted: M::Flag) { true };";
const APPLIED: &str = "effect Applied of Main::Tried as (applied: M::Receipt) { true };";
const COMMITTED: &str = "commit Committed by Service as (committed: M::Flag) { true };";

/// `run` with a `parallel P` between the attempt and the effect: branch
/// `a` holds the commit and branch `b` a `check`, joined by `join`; the
/// template's commit names `Main::P::a::Committed`.
fn joined(join: &str) -> String {
    edit(
        &with_run(&format!(
            "run sequence Main {{\n{TRIED}\n\
             parallel P {{ branch a {COMMITTED} branch b check Idle using v {{ true }}; }} \
             join {join};\n{APPLIED}\n}}"
        )),
        "commit Main::Committed;",
        "commit Main::P::a::Committed;",
    )
}

/// FR-381-AC-11: `commit never` and `commit Main::Committed` check and the
/// checked compensation holds `Never` and `Node` of the commit's id; a
/// commit that dominates the forward effect on the control tree refuses `unsupported_construct`/`declaration-form` at the commit anchor,
/// retaining the selected protocol profile, compensation, member `commit`,
/// commit, effect and reason `commit-precedes-effect` -- a commit sequenced
/// between the attempt and its effect, and one inside a `join all` branch,
/// a branch of `quorum(2) [a, b]` and the one branch of `join any [a]`, each
/// with the effect after the parallel; a commit that follows the effect, one
/// in a parallel branch whose sibling holds the effect, one in a branch of
/// `join any [a, b]` or of `quorum(1) [a, b]` with the effect after the
/// parallel, one in the body of a `repeat` with the effect after the
/// `repeat` and one in one `case` of a `choice` with the effect after the
/// `choice` check (the cross-branch `send`/`receive` case is
/// [`a_commit_ordered_across_branches_is_admitted`]).
#[trace("TC-1058", "FR-381-AC-11")]
#[test]
fn a_commit_that_dominates_its_effect_refuses() {
    let graph = checked(BASE);
    let [compensation] = graph.protocols()[0].obligations.compensations.as_slice() else {
        panic!("one compensation");
    };
    assert_eq!(
        compensation.commit,
        CheckedCommit::Node(node(BASE, "Committed", ProtocolNodeKind::Commit))
    );
    let never = changed("commit Main::Committed;", "commit never;");
    let graph = checked(&never);
    assert_eq!(
        graph.protocols()[0].obligations.compensations[0].commit,
        CheckedCommit::Never
    );

    // TC-1058 row 19: the commit between the attempt and the effect. The
    // attempt-to-effect edge lets no execution skip it.
    let between = with_run(&format!(
        "run sequence Main {{\n{TRIED}\n{COMMITTED}\n{APPLIED}\n}}"
    ));
    let (text, cause) = only(&between);
    let ObligationCause::CommitPrecedesEffect {
        profile,
        compensation,
        commit,
        effect,
        span,
    } = &cause
    else {
        panic!("a commit-precedes-effect refusal, got {cause:?}");
    };
    assert_eq!(profile.as_deref(), Some(PROTOCOL_PROFILE));
    assert_eq!(compensation, "Refund");
    assert_eq!((commit.as_str(), effect.as_str()), ("Committed", "Applied"));
    assert_eq!(cause.code(), Code::UnsupportedConstruct);
    assert_eq!(cause.tag(), Some("declaration-form"));
    assert_eq!(cause.member(), Some("commit"));
    assert_eq!(cause.reason(), Some("commit-precedes-effect"));
    assert_eq!(&text[span.start..span.end], "Main::Committed");

    // A commit nested before the effect inside a sequence still dominates,
    // and so does a commit in a branch the join requires (rows 19a, 19c).
    let nested = edit(
        &with_run(&format!(
            "run sequence Main {{\nsequence Early {{ {COMMITTED} }}\n{TRIED}\n{APPLIED}\n}}"
        )),
        "commit Main::Committed;",
        "commit Main::Early::Committed;",
    );
    let required = ["all [a, b]", "quorum(2) [a, b]", "any [a]"].map(joined);
    for source in std::iter::once(nested).chain(required) {
        let (text, cause) = only(&source);
        let ObligationCause::CommitPrecedesEffect { commit, span, .. } = &cause else {
            panic!("{source}: a commit-precedes-effect refusal, got {cause:?}");
        };
        assert_eq!(commit, "Committed");
        assert!(text[span.start..span.end].ends_with("::Committed"));
    }

    // Rows 19b and 19d: a join that requires no branch admits the commit.
    for join in ["any [a, b]", "quorum(1) [a, b]"] {
        checked(&joined(join));
    }
    let after = with_run(&format!(
        "run sequence Main {{\n{TRIED}\n{APPLIED}\n{COMMITTED}\n}}"
    ));
    let sibling = edit(
        &edit(
            &with_run(&format!(
                "run parallel Main {{\n\
                 branch work sequence Work {{ {TRIED}\n\
                 effect Applied of Tried as (applied: M::Receipt) {{ true }}; }}\n\
                 branch close {COMMITTED}\n\
                 }} join all [work, close];"
            )),
            "compensate Refund for Main::Applied",
            "compensate Refund for Main::work::Work::Applied",
        ),
        "commit Main::Committed;",
        "commit Main::close::Committed;",
    );
    // Row 19e: a `repeat` is skippable whatever its guard.
    let repeated = edit(
        &with_run(&format!(
            "run sequence Main {{\n{TRIED}\n\
             repeat Again by Service visible (attempted.ready) max 2 while {{ attempted.ready }} \
             sequence Body {{ commit Committed by Service as (committed: M::Flag) \
             {{ committed.ready }}; event Beat by Service as (beat: M::Flag) {{ true }}; }} \
             exhausted check Out using v {{ true }};\n{APPLIED}\n}}"
        )),
        "commit Main::Committed;",
        "commit Main::Again::Body::Committed;",
    );
    let case = edit(
        &with_run(&format!(
            "run sequence Main {{\n{TRIED}\n\
             choice Decide by Service visible () {{\n\
             case early when {{ true }} {COMMITTED}\n\
             case late when {{ false }} sequence Skip {{ }}\n\
             }}\n{APPLIED}\n}}"
        )),
        "commit Main::Committed;",
        "commit Main::Decide::early::Committed;",
    );
    for source in [after, sibling, repeated, case] {
        checked(&source);
    }
}

/// FR-381-AC-11, TC-1058 row 20b: under `join all [s, r]`, branch `s`
/// holds `commit C` and then `send Tx`, and branch `r` `attempt T`, the
/// `receive` of `Tx` and then `effect E`. Every execution reaching `E` has
/// passed `C` through the message edge, but dominance is decided on the
/// control tree only, where no `sequence` orders the two branches, so the
/// template checks (STD-189: deciding it through cross-branch edges is a
/// later extension).
#[trace("TC-1058", "FR-381-AC-11")]
#[test]
fn a_commit_ordered_across_branches_is_admitted() {
    let run = "run sequence Main {\n\
         parallel P {\n\
         branch s sequence S { commit C by Service as (c: M::Flag) { true }; \
         send Tx via Ch as (tx: M::Flag) { true }; }\n\
         branch r sequence R { attempt T by Service on M::Node::step contracts [] \
         as (t: M::Flag) { true }; \
         receive Rx via Ch of Main::P::s::S::Tx as (rx: M::Flag) { true }; \
         effect E of Main::P::r::R::T as (e: M::Receipt) { true }; }\n\
         } join all [s, r];\n}";
    let source = edit(
        &with_run(run),
        "role Service on M::Node;",
        "role Service on M::Node;\nrole Peer on M::Node;\n\
         channel Ch from Service to Peer carries M::Flag ordering unordered \
         delivery at-least-once capacity 1 overflow block;",
    );
    let source = edit(
        &edit(
            &source,
            "compensate Refund for Main::Applied",
            "compensate Refund for Main::P::r::R::E",
        ),
        "commit Main::Committed;",
        "commit Main::P::s::S::C;",
    );
    let graph = checked(&source);
    assert_eq!(
        graph.protocols()[0].obligations.compensations[0].commit,
        CheckedCommit::Node(node(&source, "C", ProtocolNodeKind::Commit))
    );
}

/// FR-381-AC-12, the recorded half: a `Full` template with commit
/// `Main::Committed` and a `Partial` template with commit `never`, of one
/// effect, give two checked compensations with their own captures, maxima
/// and commits, in source order; a refusal in each is reported, in source
/// order.
#[trace("TC-1058", "FR-381-AC-12")]
#[test]
fn a_full_and_a_partial_recovery_are_two_compensations() {
    let full = BASE
        .replacen("compensate Refund for", "compensate Full for", 1)
        .replacen("attempts 3 of", "attempts 4 of", 1);
    let partial = other("capture part: M::Total = otherfwd.total;")
        .replacen("compensate Other for", "compensate Partial for", 1)
        .replacen("attempts 3 of", "attempts 2 of", 1);
    let source = edit(
        &full,
        "run sequence Main",
        &format!("{partial}run sequence Main"),
    );
    let graph = checked(&source);
    let obligations = &graph.protocols()[0].obligations;
    let committed = node(&source, "Committed", ProtocolNodeKind::Commit);
    let summary: Vec<(&str, u64, CheckedCommit, usize)> = obligations
        .compensations
        .iter()
        .map(|compensation| {
            (
                compensation.name.as_str(),
                compensation.maximum_attempts,
                compensation.commit,
                compensation.registration_captures.len(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("Full", 4, CheckedCommit::Node(committed), 1),
            ("Partial", 2, CheckedCommit::Never, 1),
        ]
    );
    let owners: Vec<(&str, CaptureOwner)> = obligations
        .captures
        .iter()
        .map(|capture| (capture.name.as_str(), capture.owner))
        .collect();
    assert_eq!(
        owners,
        [
            ("order", CaptureOwner::Protocol),
            ("target", CaptureOwner::Compensation(0)),
            ("activated", CaptureOwner::Compensation(0)),
            ("part", CaptureOwner::Compensation(1)),
        ]
    );

    let refused = edit(
        &edit(&source, "attempts 4 of", "attempts 0 of"),
        "attempts 2 of",
        "attempts 0 of",
    );
    let (_, found) = refusals(&refused);
    let names: Vec<&str> = found
        .iter()
        .map(|refusal| match &refusal.cause {
            CheckCause::ProtocolObligation(cause) => match cause.as_ref() {
                ObligationCause::ZeroAttempts { compensation, .. } => compensation.as_str(),
                other => panic!("a zero count, got {other:?}"),
            },
            other => panic!("an obligation refusal, got {other:?}"),
        })
        .collect();
    assert_eq!(names, ["Full", "Partial"]);
}

/// A template whose operation's type is a member type of two populations
/// refuses `ambiguous_declaration`/`ambiguous-name` at the operation, naming
/// both populations: its frame's extent is undecided, as FR-114 refuses an
/// attempt's.
#[test]
fn a_template_over_two_populations_refuses_its_operation() {
    let source = changed(
        "by Service on M::Node::step using v",
        "by Service on M::Pool::drain using v",
    );
    let (text, assembled) = assemble_against(ambiguous_population_model(), &source);
    let found = assembled
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("an operation over two populations refuses");
    let causes = obligation_causes(&found);
    let [cause] = causes.as_slice() else {
        panic!("one obligation refusal: {found:?}");
    };
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::AmbiguousDeclaration, Some("ambiguous-name"))
    );
    let ObligationCause::AmbiguousPopulation {
        compensation,
        populations,
        span,
    } = cause
    else {
        panic!("an ambiguous population, got {cause:?}");
    };
    assert_eq!(compensation, "Refund");
    let names: Vec<&str> = populations
        .iter()
        .map(|population| population.node.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "ix://example/protocol-obligations/Pools",
            "ix://example/protocol-obligations/Reserves"
        ]
    );
    assert_eq!(&text[span.start..span.end], "M::Pool::drain");
}
