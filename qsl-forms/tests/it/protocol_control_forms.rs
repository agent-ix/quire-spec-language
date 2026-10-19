// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-308-AC-2 to AC-5 and AC-13 (TC-887, TC-1065): S2 builds the channel,
//! `parallel`, `repeat`, `terminal` and `scheduling` forms of a protocol
//! with every member as written and its span, on real complete-V1 source
//! through `qsl_cst::parse` and `build_unit`.

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, ChannelCapacity, ChannelDelivery, ChannelForm, ChannelOrdering, ChannelOverflow,
    DeclarationForm, ExpressionForm, FormsCause, JoinPolicy, LossSelector, Outstanding, ParsedUnit,
    ProtocolConstructKind, ProtocolDeclarationForm, ProtocolMemberForm, ProtocolNodeKind,
    SchedulingForm, TerminalForm,
};
use qsl_foundation::{Code, SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    model Config = \"example/config-version\" version \"1\" digest \
    \"sha256-jcs:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

const OBJECT: &str = "Config::ConfigVersion";

/// The run control every protocol here holds unless a test writes its own.
const RUN: &str = "sequence Main { attempt Tried by R on Config::ConfigVersion::attemptUpdate \
    contracts [] as (tried: Config::ConfigVersion) { true }; }";

fn parse(text: &str) -> qsl_cst::ParsedSource {
    qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn build(text: &str) -> ParsedUnit {
    let parsed = parse(text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    build_unit(&parsed).unwrap_or_else(|failure| panic!("{text}: {failure:?}"))
}

fn protocol(unit: &ParsedUnit) -> &ProtocolDeclarationForm {
    let [form] = unit.forms() else {
        panic!("one declaration: {:?}", unit.forms());
    };
    let DeclarationForm::Protocol(protocol) = form.form() else {
        panic!("a protocol form: {:?}", form.form());
    };
    protocol
}

/// One protocol with roles `R` and `S`, `members` between the roles and
/// `run`, and the given run control.
fn source(members: &str, run: &str) -> String {
    format!(
        "{HEADER}protocol Flow using v over (input: {OBJECT}) on origin {{\n\
         role R on {OBJECT};\n\
         role S on {OBJECT};\n\
         {members}\n\
         run {run}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    )
}

fn channel_member(overflow: &str) -> String {
    format!(
        "channel Pay from R to S carries {OBJECT} ordering unordered delivery at-least-once \
         capacity 1 overflow {overflow};"
    )
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

fn expression_text<'t>(text: &'t str, form: &ExpressionForm) -> &'t str {
    let span = form
        .spans
        .span(form.spans.root())
        .expect("the root has a span");
    slice(text, span)
}

fn only_channel(form: &ProtocolDeclarationForm) -> &ChannelForm {
    let [channel] = &form.channels[..] else {
        panic!("one channel: {:?}", form.channels);
    };
    channel
}

/// Channel form only (no criterion is claimed): a bounded channel builds its roles, its
/// payload type and each premise as written, with the span of its text.
#[trace("TC-887")]
#[test]
fn a_channel_builds_every_premise_as_written() {
    let text = source(&channel_member("block"), RUN);
    let unit = build(&text);
    let channel = only_channel(protocol(&unit));

    assert_eq!(channel.name.name, "Pay");
    assert_eq!(slice(&text, channel.from.span), "R");
    assert_eq!(channel.from.name, "R");
    assert_eq!(channel.to.name, "S");
    assert_eq!(slice(&text, channel.carries.span), OBJECT);
    assert_eq!(channel.ordering.value, ChannelOrdering::Unordered);
    assert_eq!(slice(&text, channel.ordering.span), "unordered");
    assert_eq!(channel.delivery.value, ChannelDelivery::AtLeastOnce);
    assert_eq!(slice(&text, channel.delivery.span), "at-least-once");
    assert_eq!(channel.capacity.value, ChannelCapacity::Literal(1));
    assert_eq!(slice(&text, channel.capacity.span), "1");
    assert_eq!(channel.overflow.value, ChannelOverflow::Block);
    assert_eq!(slice(&text, channel.overflow.span), "block");
    assert!(slice(&text, channel.span).starts_with("channel Pay"));
    assert!(slice(&text, channel.span).ends_with(';'));
}

/// The other delivery, capacity and ordering spellings build their own
/// values, so no premise is read from another.
#[trace("TC-887")]
#[test]
fn each_delivery_capacity_and_ordering_spelling_builds_its_own_value() {
    for (spelling, expected) in [
        ("unknown", ChannelDelivery::Unknown),
        ("at-most-once", ChannelDelivery::AtMostOnce),
        ("at-least-once", ChannelDelivery::AtLeastOnce),
        ("exactly-once-premise", ChannelDelivery::ExactlyOncePremise),
    ] {
        let member = format!(
            "channel Pay from R to S carries {OBJECT} ordering unordered delivery {spelling} \
             capacity symbolic(Bound) overflow reject;"
        );
        let text = source(&member, RUN);
        let unit = build(&text);
        let channel = only_channel(protocol(&unit));
        assert_eq!(channel.delivery.value, expected, "{spelling}");
        assert_eq!(slice(&text, channel.delivery.span), spelling);
        assert_eq!(
            channel.capacity.value,
            ChannelCapacity::Symbolic("Bound".to_owned())
        );
        assert_eq!(slice(&text, channel.capacity.span), "symbolic(Bound)");
    }

    let member = format!(
        "channel Pay from R to S carries {OBJECT} ordering fifo by (m: {OBJECT}) {{ m.ready }} \
         delivery unknown capacity 2 overflow reject;"
    );
    let text = source(&member, RUN);
    let unit = build(&text);
    let form = protocol(&unit);
    let channel = only_channel(form);
    let ChannelOrdering::Fifo { binder, key } = &channel.ordering.value else {
        panic!("a fifo ordering: {:?}", channel.ordering);
    };
    assert_eq!(binder.name.name, "m");
    assert_eq!(expression_text(&text, key), "m.ready");
    assert!(slice(&text, channel.ordering.span).starts_with("fifo by"));
    assert!(form
        .constructs
        .iter()
        .any(|construct| construct.kind == ProtocolConstructKind::FifoOrdering));
}

/// FR-308-AC-13 (TC-1065 rows 1 and 2): the five overflow spellings build
/// `Reject`, `Block` and `Loss` with their selector, each span slicing to its
/// text; `overflow loss` with no selector and `overflow block head` fail at
/// the parser with a syntax diagnostic and build no form.
#[trace("TC-1065", "FR-308-AC-13")]
#[test]
fn the_loss_selector_follows_loss_and_no_other_overflow_policy() {
    for (spelling, expected) in [
        ("reject", ChannelOverflow::Reject),
        ("block", ChannelOverflow::Block),
        ("loss head", ChannelOverflow::Loss(LossSelector::Head)),
        ("loss tail", ChannelOverflow::Loss(LossSelector::Tail)),
        ("loss new", ChannelOverflow::Loss(LossSelector::New)),
    ] {
        let text = source(&channel_member(spelling), RUN);
        let unit = build(&text);
        let channel = only_channel(protocol(&unit));
        assert_eq!(channel.overflow.value, expected, "{spelling}");
        assert_eq!(slice(&text, channel.overflow.span), spelling);
    }

    // The parser stops at the first token the grammar does not admit: the
    // `;` where a selector was due, and the selector after `block`.
    for (written, before_failure) in [("loss", "overflow loss"), ("block head", "overflow block ")]
    {
        let text = source(&channel_member(written), RUN);
        let parsed = parse(&text);
        assert!(!parsed.is_admissible(), "{written}");
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(diagnostic.code, Code::InvalidSyntax, "{written}");
        let at = diagnostic.byte_span().expect("the diagnostic is located");
        assert_eq!(
            at.start,
            text.find(before_failure)
                .map(|start| start + before_failure.len())
                .expect("the policy is in the source"),
            "{written}"
        );
        assert!(build_unit(&parsed).is_err(), "{written}");
    }
}

/// A selector word is a loss selector only after `loss`: elsewhere it is an
/// identifier.
#[trace("TC-1065")]
#[test]
fn head_tail_and_new_stay_identifiers_outside_a_loss_overflow() {
    let run = "sequence head { attempt tail by R on Config::ConfigVersion::attemptUpdate \
        contracts [] as (new: Config::ConfigVersion) { true }; }";
    let text = source("", run);
    let unit = build(&text);
    let form = protocol(&unit);
    let names: Vec<&str> = form
        .declarations
        .iter()
        .map(|declaration| declaration.name.name.as_str())
        .collect();
    assert!(names.contains(&"head"), "{names:?}");
    assert!(names.contains(&"tail"), "{names:?}");
}

const PARALLEL_BRANCHES: &str = "{ branch a sequence A { } branch b sequence B { } }";

/// FR-308-AC-2: a `parallel` joined by quorum with an `outstanding` rule, and
/// the same joined by `all` with none.
#[trace("TC-887", "FR-308-AC-2")]
#[test]
fn a_parallel_builds_its_join_policy_and_outstanding_rule() {
    let run = format!("parallel P {PARALLEL_BRANCHES} join quorum(2) [a, b] outstanding cancel;");
    let text = source("", &run);
    let unit = build(&text);
    let form = protocol(&unit);
    let [parallel] = &form.parallels[..] else {
        panic!("one parallel: {:?}", form.parallels);
    };
    assert_eq!(parallel.name.name, "P");
    assert_eq!(
        form.declarations[parallel.declaration].kind,
        ProtocolNodeKind::Parallel
    );
    let branches: Vec<&str> = parallel
        .branches
        .iter()
        .map(|branch| branch.name.as_str())
        .collect();
    assert_eq!(branches, ["a", "b"]);
    assert_eq!(parallel.join.value, JoinPolicy::Quorum(2));
    assert_eq!(slice(&text, parallel.join.span), "quorum(2)");
    assert_eq!(
        slice(&text, parallel.quorum_count.expect("a quorum count")),
        "2"
    );
    let labels: Vec<&str> = parallel
        .labels
        .iter()
        .map(|label| label.name.as_str())
        .collect();
    assert_eq!(labels, ["a", "b"]);
    assert_eq!(slice(&text, parallel.labels_span), "[a, b]");
    let outstanding = parallel.outstanding.as_ref().expect("an outstanding rule");
    assert_eq!(outstanding.value, Outstanding::Cancel);
    assert_eq!(slice(&text, outstanding.span), "outstanding cancel");

    let run = format!("parallel P {PARALLEL_BRANCHES} join all [a, b];");
    let text = source("", &run);
    let unit = build(&text);
    let [parallel] = &protocol(&unit).parallels[..] else {
        panic!("one parallel");
    };
    assert_eq!(parallel.join.value, JoinPolicy::All);
    assert_eq!(parallel.quorum_count, None);
    assert_eq!(parallel.outstanding, None);

    let run = format!("parallel P {PARALLEL_BRANCHES} join any [] outstanding continue;");
    let text = source("", &run);
    let unit = build(&text);
    let [parallel] = &protocol(&unit).parallels[..] else {
        panic!("one parallel");
    };
    assert_eq!(parallel.join.value, JoinPolicy::Any);
    assert!(parallel.labels.is_empty());
    assert_eq!(slice(&text, parallel.labels_span), "[]");
    assert_eq!(
        parallel.outstanding.as_ref().map(|rule| rule.value),
        Some(Outstanding::Continue)
    );

    let run = format!("parallel P {PARALLEL_BRANCHES} join predicate(done) [a, b];");
    let text = source("", &run);
    let unit = build(&text);
    let form = protocol(&unit);
    assert_eq!(
        form.parallels[0].join.value,
        JoinPolicy::Predicate("done".to_owned())
    );
    assert!(form
        .constructs
        .iter()
        .any(|construct| construct.kind == ProtocolConstructKind::JoinPredicate));
}

/// FR-308-AC-3: a `repeat` with a loop proof and no maximum, and one with a
/// maximum, no proof and an `exhausted` control.
#[trace("TC-887", "FR-308-AC-3")]
#[test]
fn a_repeat_builds_its_maximum_loop_proof_and_exhausted_control() {
    let run = "repeat Again by S visible () invariant { true } variant { 1 } while { false } \
        sequence Body { }";
    let text = source("", run);
    let unit = build(&text);
    let form = protocol(&unit);
    let [repeat] = &form.repeats[..] else {
        panic!("one repeat: {:?}", form.repeats);
    };
    assert_eq!(repeat.name.name, "Again");
    assert_eq!(repeat.owner.name, "S");
    assert_eq!(
        form.declarations[repeat.declaration].kind,
        ProtocolNodeKind::Repeat
    );
    assert!(repeat.visible.is_empty());
    assert_eq!(slice(&text, repeat.visible_span), "visible ()");
    assert_eq!(repeat.max, None);
    let proof = repeat.loop_proof.as_ref().expect("a loop proof");
    assert_eq!(expression_text(&text, &proof.invariant), "true");
    assert_eq!(expression_text(&text, &proof.variant), "1");
    assert!(slice(&text, proof.span).starts_with("invariant"));
    assert!(slice(&text, proof.span).ends_with("{ 1 }"));
    assert_eq!(expression_text(&text, &repeat.guard), "false");
    assert_eq!(slice(&text, repeat.body), "sequence Body { }");
    assert_eq!(repeat.exhausted, None);

    let run = "repeat Again by S visible (input.ready) max 3 while { false } sequence Body { } \
        exhausted sequence Out { }";
    let text = source("", run);
    let unit = build(&text);
    let [repeat] = &protocol(&unit).repeats[..] else {
        panic!("one repeat");
    };
    let max = repeat.max.as_ref().expect("a maximum");
    assert_eq!(max.value, 3);
    assert_eq!(slice(&text, max.span), "3");
    assert_eq!(repeat.loop_proof, None);
    assert_eq!(repeat.visible.len(), 1);
    assert_eq!(expression_text(&text, &repeat.visible[0]), "input.ready");
    assert_eq!(slice(&text, repeat.body), "sequence Body { }");
    assert_eq!(
        slice(&text, repeat.exhausted.expect("an exhausted control")),
        "sequence Out { }"
    );

    let run = "repeat Again by S visible () max 3 invariant { true } variant { 1 } \
        while { false } sequence Body { } exhausted sequence Out { }";
    let text = source("", run);
    let unit = build(&text);
    let [repeat] = &protocol(&unit).repeats[..] else {
        panic!("one repeat");
    };
    assert_eq!(repeat.max.as_ref().map(|max| max.value), Some(3));
    assert!(repeat.loop_proof.is_some());
    assert!(repeat.exhausted.is_some());

    // `exhausted` belongs to a maximum alone, and a maximum needs it.
    for written in [
        "repeat Again by S visible () while { false } sequence Body { } exhausted sequence Out { }",
        "repeat Again by S visible () max 3 while { false } sequence Body { }",
    ] {
        let parsed = parse(&source("", written));
        assert!(!parsed.is_admissible(), "{written}");
    }
}

/// An `await` builds the reference, the clock, the interval, the matched
/// event and its two controls as written.
#[trace("TC-887")]
#[test]
fn an_await_builds_its_parts_as_written() {
    let run = "sequence Main { \
        send Sent via Pay as (sent: Config::ConfigVersion) { true }; \
        await Wait after Main::Sent using v clock \"ticks\" within [0,5] \
        match receive Reply via Pay of Main::Sent as (reply: Config::ConfigVersion) { true }; \
        then sequence Then { } timeout sequence Timeout { } }";
    let text = source(&channel_member("block"), run);
    let unit = build(&text);
    let form = protocol(&unit);
    let [wait] = &form.awaits[..] else {
        panic!("one await: {:?}", form.awaits);
    };
    assert_eq!(wait.name.name, "Wait");
    assert_eq!(
        form.declarations[wait.declaration].kind,
        ProtocolNodeKind::Await
    );
    assert_eq!(wait.after.name, "Main::Sent");
    assert_eq!(slice(&text, wait.after.span), "Main::Sent");
    assert_eq!(wait.using.alias, "v");
    assert_eq!(wait.clock.text, "ticks");
    assert_eq!(slice(&text, wait.clock.span), "\"ticks\"");
    assert_eq!(slice(&text, wait.within.span), "[0,5]");
    assert_eq!(wait.matched.name, "Reply");
    assert_eq!(slice(&text, wait.then), "sequence Then { }");
    assert_eq!(slice(&text, wait.timeout), "sequence Timeout { }");
}

fn terminal_text<'t>(text: &'t str, member: &ProtocolMemberForm) -> &'t str {
    slice(text, member.span())
}

/// FR-308-AC-4: `terminal when`, `terminal any` and `scheduling adversarial`
/// build three members in source order with their spans; the words are
/// identifiers elsewhere.
#[trace("TC-887", "FR-308-AC-4")]
#[test]
fn terminal_and_scheduling_members_keep_their_source_order() {
    let members = "terminal when { input.ready };\nterminal any;\nscheduling adversarial;";
    let text = source(members, RUN);
    let unit = build(&text);
    let form = protocol(&unit);
    let [first, second, third] = &form.members[..] else {
        panic!("three members: {:?}", form.members);
    };
    let ProtocolMemberForm::Terminal(TerminalForm::When { predicate, .. }) = first else {
        panic!("terminal when: {first:?}");
    };
    assert_eq!(expression_text(&text, predicate), "input.ready");
    assert_eq!(
        terminal_text(&text, first),
        "terminal when { input.ready };"
    );
    assert!(matches!(
        second,
        ProtocolMemberForm::Terminal(TerminalForm::Any { .. })
    ));
    assert_eq!(terminal_text(&text, second), "terminal any;");
    assert!(matches!(
        third,
        ProtocolMemberForm::Scheduling(SchedulingForm::Adversarial { .. })
    ));
    assert_eq!(terminal_text(&text, third), "scheduling adversarial;");
    assert!(form
        .constructs
        .iter()
        .any(|construct| construct.kind == ProtocolConstructKind::TerminalWhen));

    // `fork`, `join`, `terminal` and `scheduling` are identifiers wherever
    // the grammar does not reserve them.
    let run = "sequence fork { attempt terminal by R on Config::ConfigVersion::attemptUpdate \
        contracts [] as (scheduling: Config::ConfigVersion) { true }; }";
    let text = source("", run);
    let unit = build(&text);
    let form = protocol(&unit);
    let fork = form
        .declarations
        .iter()
        .find(|declaration| declaration.name.name == "fork")
        .expect("a declaration named fork");
    assert_eq!(fork.kind, ProtocolNodeKind::Sequence);
    assert!(form
        .declarations
        .iter()
        .any(|declaration| declaration.name.name == "terminal"));
    assert!(form.members.is_empty());
}

/// FR-308-AC-5: a recovery node inside a channel, a `parallel` or a
/// `terminal` or `scheduling` member refuses `RecoveringCst` with no form,
/// and building a declaration twice gives equal forms.
#[trace("TC-887", "FR-308-AC-5")]
#[test]
fn a_recovering_cst_refuses_and_a_rebuilt_declaration_is_equal() {
    let full = source(
        &format!(
            "{}\nterminal when {{ true }};\nscheduling adversarial;",
            channel_member("loss head")
        ),
        "sequence Main { \
         parallel P { branch a sequence A { } branch b sequence B { } } \
         join quorum(2) [a, b] outstanding cancel; \
         repeat Again by S visible () max 2 while { false } sequence Body { } \
         exhausted sequence Out { } \
         await Wait after Main::Body using v clock \"ticks\" within [0,5] \
         match event Reply by S as (reply: Config::ConfigVersion) { true }; \
         then sequence Then { } timeout sequence Timeout { } }",
    );
    for (written, broken) in [
        ("overflow loss head;", "overflow loss head"),
        ("outstanding cancel;", "outstanding cancel"),
        ("terminal when { true };", "terminal when { true }"),
        ("scheduling adversarial;", "scheduling adversarial"),
    ] {
        assert!(full.contains(written), "{written}");
        let text = full.replacen(written, broken, 1);
        let parsed = parse(&text);
        assert!(
            !parsed.cst().recoveries().is_empty(),
            "{broken}: S1 recovers"
        );
        match build_unit(&parsed) {
            Err(refusal) => assert_eq!(refusal.cause, FormsCause::RecoveringCst, "{broken}"),
            Ok(unit) => panic!("{broken}: a refusal, not {unit:?}"),
        }
    }

    let first = build(&full);
    let second = build(&full);
    assert_eq!(protocol(&first), protocol(&second));
}

/// FR-308-AC-1: a replicated role, a channel and an `on each` activation
/// build a `Replicated` role with population `workers`, max 2 and an `Until`
/// lifetime, a `ChannelForm` with `AtLeastOnce`, `Literal(1)` and `Block`,
/// and an `Each` activation with binder `x`, each member's span covering its
/// text. A static role builds no replicated part, and `workflow` and
/// `scope` lifetimes build as written.
#[trace("TC-887", "FR-308-AC-1")]
#[test]
fn a_replicated_role_a_channel_and_an_activation_build_as_written() {
    let role = format!("role w each {OBJECT} from workers max 2 lifetime until(w.done);");
    let channel = format!(
        "channel ch from R to w carries {OBJECT} ordering unordered delivery at-least-once \
         capacity 1 overflow block;"
    );
    let text = source(&format!("{role}\n{channel}"), RUN).replacen(
        "on origin",
        &format!("on each (x: {OBJECT}) when (true)"),
        1,
    );
    let unit = build(&text);
    let form = protocol(&unit);

    let [static_r, static_s, replicated] = &form.roles[..] else {
        panic!("three roles: {:?}", form.roles);
    };
    assert_eq!(static_r.replicated, None);
    assert_eq!(slice(&text, static_s.span), format!("role S on {OBJECT};"));
    assert_eq!(replicated.name.name, "w");
    assert_eq!(slice(&text, replicated.span), role);
    assert_eq!(slice(&text, replicated.context.span), OBJECT);
    let Some(qsl_forms::ReplicatedRoleForm {
        population,
        max,
        max_span,
        lifetime,
    }) = &replicated.replicated
    else {
        panic!("a replicated role: {replicated:?}");
    };
    assert_eq!(population.name, "workers");
    assert_eq!(slice(&text, population.span), "workers");
    assert_eq!(*max, 2);
    assert_eq!(slice(&text, *max_span), "2");
    let qsl_forms::RoleLifetime::Until(condition) = lifetime else {
        panic!("an Until lifetime: {lifetime:?}");
    };
    assert_eq!(expression_text(&text, condition), "w.done");

    let channel_form = only_channel(form);
    assert_eq!(channel_form.delivery.value, ChannelDelivery::AtLeastOnce);
    assert_eq!(channel_form.capacity.value, ChannelCapacity::Literal(1));
    assert_eq!(channel_form.overflow.value, ChannelOverflow::Block);
    assert_eq!(slice(&text, channel_form.span), channel);

    let qsl_forms::ActivationForm::Each { parameter, when } = &form.activation else {
        panic!("an Each activation: {:?}", form.activation);
    };
    assert_eq!(parameter.name.name, "x");
    assert_eq!(
        when.as_ref().map(|when| expression_text(&text, when)),
        Some("true")
    );

    for (written, scope) in [("workflow", false), ("scope", true)] {
        let text = source(
            &format!("role w each {OBJECT} from workers max 1 lifetime {written};"),
            RUN,
        );
        let unit = build(&text);
        let lifetime = protocol(&unit).roles[2]
            .replicated
            .as_ref()
            .map(|replicated| replicated.lifetime.clone());
        match (lifetime, scope) {
            (Some(qsl_forms::RoleLifetime::Workflow(span)), false)
            | (Some(qsl_forms::RoleLifetime::Scope(span)), true) => {
                assert_eq!(slice(&text, span), written);
            }
            (other, _) => panic!("{written}: {other:?}"),
        }
    }
}

/// Verifies the parse only and claims no criterion.
/// A `fence` control and a `parallel` with a `memory` clause
/// build as protocol nodes beside the controls around them; the `memory`
/// words do not become the parallel's name or join labels.
#[test]
fn a_fence_and_a_memory_clause_build_beside_their_controls() {
    let run = format!(
        "sequence Main {{ parallel P memory tso {PARALLEL_BRANCHES} join all [a, b]; \
         fence Barrier ordering seq_cst; }}"
    );
    let text = source("", &run);
    let unit = build(&text);
    let form = protocol(&unit);
    let [parallel] = &form.parallels[..] else {
        panic!("one parallel: {:?}", form.parallels);
    };
    assert_eq!(parallel.name.name, "P");
    let labels: Vec<&str> = parallel
        .labels
        .iter()
        .map(|label| label.name.as_str())
        .collect();
    assert_eq!(labels, ["a", "b"]);
    let fences: Vec<&str> = form
        .declarations
        .iter()
        .filter(|declaration| declaration.kind == ProtocolNodeKind::Fence)
        .map(|declaration| declaration.name.name.as_str())
        .collect();
    assert_eq!(fences, ["Barrier"]);
}
