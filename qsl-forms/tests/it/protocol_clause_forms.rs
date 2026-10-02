// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-102: the `ProtocolClause` family's S2 production —
//! `invariant`/`pre`/`post` state clauses and the `self`, `result` and
//! `reaches` expressions — run on real complete-V1 source through
//! `qsl_cst::parse` and `build_unit` (TC-456, TC-457).

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, AnchorSite, BinderForm, BinderKind, BuiltinType, DeclarationForm, Expression,
    ExpressionSpans, FormsCause, FormsFailure, FormsLimits, ParsedUnit, ProtocolConstructKind,
    ProtocolDeclarationForm, ProtocolNodeDeclaration, ProtocolNodeKind, ScopedAnchorForm, SpanId,
    StateClauseForm, StateClauseKind, TypeFormHead,
};
use qsl_foundation::diagnostic::LimitKind;
use qsl_foundation::{SourceIdentity, Span};

/// Every unit under test starts with this header (TC-456's own text): one
/// profile and one model selection, neither of which S2 resolves.
const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    model Config = \"example/config-version\" version \"1\" digest \
    \"sha256-jcs:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

fn parse(declarations: &str) -> (String, qsl_cst::ParsedSource) {
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

fn admissible(declarations: &str) -> (String, qsl_cst::ParsedSource) {
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

fn refusal(declarations: &str) -> (String, FormsCause, Option<Span>) {
    let (text, parsed) = admissible(declarations);
    match build_unit(&parsed, FormsLimits::default()) {
        Err(FormsFailure::Refused(refusal)) => (text, refusal.cause, refusal.span),
        other => panic!("{declarations}: a refusal, not {other:?}"),
    }
}

/// SR-722 FND-007 (TC-456 step 1): every node of an expression tree's own
/// spans, not just the root and one child, slices the source to text that
/// parses back as one whole expression on its own -- built the same way
/// `build`'s own caller builds a state clause body, so this needs no
/// separate parse harness and no `show()`-style rendering that would have
/// to recognize every production this fixture's body uses (`present`,
/// `deref`, `value`, `implies`, `<`, none of which `show` itself matches
/// explicitly). `spans.fits(expr)` is a build-time invariant this crate
/// already checks (`DeclarationSpans::fit`), so walking `expr.children()`
/// and `spans.child(id, index)` in lockstep never runs out of either side.
fn assert_every_span_slice_reparses(
    text: &str,
    spans: &ExpressionSpans,
    id: SpanId,
    expr: &Expression,
) {
    let span = spans
        .span(id)
        .unwrap_or_else(|| panic!("{expr:?}: every visited span node has a span"));
    let slice = &text[span.start..span.end];
    let (reparsed_text, reparsed_unit) = build(&format!(
        "invariant Z using v on Config::ConfigVersion at current {{ {slice} }}"
    ));
    let reparsed_form = state_clause(reparsed_unit.forms()[0].form());
    let reparsed_spans = &reparsed_form.spans.body;
    let root_span = reparsed_spans
        .span(reparsed_spans.root())
        .expect("the reparsed body has a root span");
    assert_eq!(
        &reparsed_text[root_span.start..root_span.end],
        slice,
        "{slice:?} does not parse back as one whole expression on its own"
    );
    for (index, child_expr) in expr.children().into_iter().enumerate() {
        let child_id = spans
            .child(id, index)
            .unwrap_or_else(|| panic!("{expr:?}: child {index} has no span (spans.fits failed)"));
        assert_every_span_slice_reparses(text, spans, child_id, child_expr);
    }
}

fn state_clause(form: &DeclarationForm) -> &StateClauseForm {
    match form {
        DeclarationForm::StateClause(state_clause) => state_clause,
        DeclarationForm::Function(_)
        | DeclarationForm::Alias(_)
        | DeclarationForm::Record(_)
        | DeclarationForm::Tuple(_)
        | DeclarationForm::Enum(_)
        | DeclarationForm::Dimension(_)
        | DeclarationForm::Unit(_)
        | DeclarationForm::Protocol(_) => panic!("a state clause form, not {form:?}"),
    }
}

fn protocol_form(form: &DeclarationForm) -> &ProtocolDeclarationForm {
    match form {
        DeclarationForm::Protocol(protocol) => protocol,
        DeclarationForm::Function(_)
        | DeclarationForm::Alias(_)
        | DeclarationForm::Record(_)
        | DeclarationForm::Tuple(_)
        | DeclarationForm::Enum(_)
        | DeclarationForm::Dimension(_)
        | DeclarationForm::Unit(_)
        | DeclarationForm::StateClause(_) => panic!("a protocol form, not {form:?}"),
    }
}

/// `RecoveryFlow` (TC-510's fixture, matching `tests/it/composed_scopes.rs`
/// `COMPENSATION`'s shape): a `compensate Undo` clause (its `for` and
/// `commit` targets, both `compensate-for`/`compensate-commit`) and a
/// `run sequence Main` holding an `attempt` (no anchor: FR-112 lists no
/// site for it), an `effect` (`effect-of`), an `event` (`event-for`) and,
/// as `COMPENSATION` does, a bare `commit` control (no anchor, but it
/// exercises the walk's `Commit` arm). `commit` is the compensation's own
/// `commit` target (`Main::Committed` or `never`); `effect` is included
/// unless `include_effect` is false; `main_extra` is spliced in after the
/// `attempt`, before the `effect`, so a caller can nest further controls
/// inside `Main` (FR-112-AC-2).
fn recovery_flow(commit: &str, include_effect: bool, main_extra: &str) -> String {
    let effect = if include_effect {
        "effect Applied of Tried as (applied: Config::ConfigVersion) { true };\n"
    } else {
        ""
    };
    format!(
        "protocol RecoveryFlow using v over (input: Config::ConfigVersion) on origin {{\n\
         role R on Config::ConfigVersion;\n\
         compensate Undo for Main::Applied as (failure: Config::ConfigVersion) by R \
         on Config::ConfigVersion::attemptUpdate using v clock \"ticks\" {{\n\
         activate first (trigger: Config::ConfigVersion) when {{ true }} {{ }}\n\
         within [0,10]; attempts 2 of Config::ConfigVersion;\n\
         retry (current_attempt: Config::ConfigVersion, prior: Config::ConfigVersion) {{ true }};\n\
         commit {commit}; recover (recovery: Config::ConfigVersion) {{ true }}; }}\n\
         run sequence Main {{\n\
         attempt Tried by R on Config::ConfigVersion::attemptUpdate contracts [] \
         as (tried: Config::ConfigVersion) {{ true }};\n\
         {main_extra}\
         {effect}\
         event Recovered by R for Undo as (notice: Config::ConfigVersion) {{ true }};\n\
         commit Committed by R as (committed: Config::ConfigVersion) {{ true }};\n\
         }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}"
    )
}

fn segment_texts(anchor: &ScopedAnchorForm) -> Vec<&str> {
    anchor
        .anchor
        .segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect()
}

fn scope_names(anchor: &ScopedAnchorForm) -> Vec<&str> {
    anchor.scope.iter().map(|name| name.name.as_str()).collect()
}

#[trace("TC-510", "FR-112-AC-1")]
#[test]
fn s2_builds_four_scoped_anchors_in_source_order() {
    let (text, unit) = build(&recovery_flow("Main::Committed", true, ""));
    let form = protocol_form(unit.forms()[0].form());
    assert_eq!(form.scoped_anchors.len(), 4);

    let compensate_for = &form.scoped_anchors[0];
    assert_eq!(compensate_for.site, AnchorSite::CompensateFor);
    assert_eq!(segment_texts(compensate_for), ["Main", "Applied"]);
    assert!(compensate_for.scope.is_empty());

    let compensate_commit = &form.scoped_anchors[1];
    assert_eq!(compensate_commit.site, AnchorSite::CompensateCommit);
    assert_eq!(segment_texts(compensate_commit), ["Main", "Committed"]);
    assert!(compensate_commit.scope.is_empty());

    let effect_of = &form.scoped_anchors[2];
    assert_eq!(effect_of.site, AnchorSite::EffectOf);
    assert_eq!(segment_texts(effect_of), ["Tried"]);
    assert_eq!(scope_names(effect_of), ["Main"]);

    let event_for = &form.scoped_anchors[3];
    assert_eq!(event_for.site, AnchorSite::EventFor);
    assert_eq!(segment_texts(event_for), ["Undo"]);
    assert_eq!(scope_names(event_for), ["Main"]);

    // Each segment's span covers exactly the segment's text in the source.
    for anchor in &form.scoped_anchors {
        assert_eq!(
            &text[anchor.anchor.span.start..anchor.anchor.span.end],
            anchor
                .anchor
                .segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>()
                .join("::")
        );
        for segment in &anchor.anchor.segments {
            assert_eq!(&text[segment.span.start..segment.span.end], segment.text);
        }
        // Each scope name's span covers exactly that name's text too
        // (FR-112 Outputs: "the names, with their spans").
        for scope_name in &anchor.scope {
            assert_eq!(
                &text[scope_name.span.start..scope_name.span.end],
                scope_name.name
            );
        }
    }
}

fn binder<'a>(
    form: &'a ProtocolDeclarationForm,
    name: &str,
    kind: BinderKind,
    scope: &[&str],
) -> &'a BinderForm {
    form.binders
        .iter()
        .find(|binder| {
            binder.name.name == name
                && binder.kind == kind
                && binder
                    .scope
                    .iter()
                    .map(|s| s.name.as_str())
                    .eq(scope.iter().copied())
        })
        .unwrap_or_else(|| {
            panic!(
                "no {kind:?} named {name} in scope {scope:?}: {:?}",
                form.binders
            )
        })
}

/// SR-766 FND-003: every binder position FR-113's no-shadowing rule reaches
/// -- the protocol's own `over (p)` input parameter, its `activation on
/// each (p) when (...)` parameter, a top-level `capture`, a `compensate`
/// declaration's own record binder, trigger, capture and retry/recovery
/// parameters, an event node's record binder and `finish`'s own record
/// binder -- is collected, in source order, with the scope FR-113 gives it.
/// Dropping any one binder position from the S2 walk (SR-766's mutations
/// M3/M4: the `finish` binder, the protocol's own top-level captures, or
/// `Scope::named_types`'s own type check downstream) leaves every other
/// qsl-forms and qsl-semantics test green; this test would not be, since it
/// names every position directly rather than only asserting "no refusal".
#[trace("TC-510", "FR-113")]
#[test]
fn s2_builds_every_binder_position_in_source_order() {
    let source = "protocol Recording using v over (input: Config::ConfigVersion) \
         on each (activated: Config::ConfigVersion) when (true) {\n\
         capture logged: Config::ConfigVersion = input;\n\
         role R on Config::ConfigVersion;\n\
         compensate Undo for Main::Applied as (failure: Config::ConfigVersion) by R \
         on Config::ConfigVersion::attemptUpdate using v clock \"ticks\" {\n\
         activate first (trigger: Config::ConfigVersion) when { true } \
         { capture noted: Config::ConfigVersion = trigger; }\n\
         within [0,10]; attempts 2 of Config::ConfigVersion;\n\
         retry (current_attempt: Config::ConfigVersion, prior: Config::ConfigVersion) { true };\n\
         commit Main::Committed; recover (recovery: Config::ConfigVersion) { true }; }\n\
         run sequence Main {\n\
         attempt Tried by R on Config::ConfigVersion::attemptUpdate contracts [] \
         as (tried: Config::ConfigVersion) { true };\n\
         effect Applied of Tried as (applied: Config::ConfigVersion) { true };\n\
         event Recovered by R for Undo as (notice: Config::ConfigVersion) { true };\n\
         commit Committed by R as (committed: Config::ConfigVersion) { true };\n\
         }\n\
         finish End as (outcome: Boolean) { true };\n\
         }";
    let (_, unit) = build(source);
    let form = protocol_form(unit.forms()[0].form());

    binder(form, "input", BinderKind::Input, &[]);
    binder(form, "activated", BinderKind::ActivationParameter, &[]);
    binder(form, "logged", BinderKind::Capture, &[]);
    binder(form, "failure", BinderKind::RecordBinder, &[]);
    binder(form, "trigger", BinderKind::Trigger, &["Undo"]);
    binder(form, "noted", BinderKind::Capture, &["Undo"]);
    binder(
        form,
        "current_attempt",
        BinderKind::RetryParameter,
        &["Undo"],
    );
    binder(form, "prior", BinderKind::RetryParameter, &["Undo"]);
    binder(form, "recovery", BinderKind::RecoveryParameter, &["Undo"]);
    binder(form, "tried", BinderKind::RecordBinder, &["Main"]);
    binder(form, "applied", BinderKind::RecordBinder, &["Main"]);
    binder(form, "notice", BinderKind::RecordBinder, &["Main"]);
    binder(form, "committed", BinderKind::RecordBinder, &["Main"]);
    binder(form, "outcome", BinderKind::RecordBinder, &[]);

    // Every position is collected exactly once, and `finish`'s own record
    // binder is last: dropping it (SR-766 M4) would leave `outcome`
    // missing entirely rather than merely reordered.
    assert_eq!(form.binders.len(), 14, "{:?}", form.binders);
    assert_eq!(form.binders.last().unwrap().name.name, "outcome");

    // Source order: the protocol's own input and activation parameters
    // precede its top-level capture, which precedes the whole `compensate`
    // declaration, which precedes `run`'s own control-tree binders, which
    // precede `finish`'s.
    let starts: Vec<usize> = form
        .binders
        .iter()
        .map(|binder| binder.name.span.start)
        .collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(
        starts, sorted,
        "binders are not in source order: {starts:?}"
    );
}

/// FR-114 "Inputs": the attempt's own `on M::T::op` operation
/// name and `contracts [...]` list are captured into `ProtocolDeclarationForm:
/// :attempts`, alongside its `ProtocolNodeKind::Attempt` declaration, with
/// no name resolved yet (the assembler's job, FR-114 "Behavior"). Naming
/// `declaration` by index rather than only asserting "one attempt form
/// exists" catches a mutation that pushes the wrong declaration's index.
#[trace("TC-513")]
#[test]
fn s2_captures_an_attempts_operation_name_and_contracts_list() {
    let source = recovery_flow("Main::Committed", true, "").replacen(
        "attempt Tried by R on Config::ConfigVersion::attemptUpdate contracts []",
        "attempt Tried by R on Config::ConfigVersion::attemptUpdate \
         contracts [VersionUnchanged, AnotherClause]",
        1,
    );
    let (_, unit) = build(&source);
    let form = protocol_form(unit.forms()[0].form());
    assert_eq!(form.attempts.len(), 1, "{:?}", form.attempts);
    let attempt = &form.attempts[0];
    assert_eq!(attempt.context.name, "Config::ConfigVersion");
    assert_eq!(attempt.operation.name, "attemptUpdate");
    assert_eq!(attempt.role.name, "R");
    let contract_names: Vec<&str> = attempt
        .contracts
        .iter()
        .map(|name| name.name.as_str())
        .collect();
    assert_eq!(contract_names, ["VersionUnchanged", "AnotherClause"]);

    let declaration = &form.declarations[attempt.declaration];
    assert_eq!(declaration.kind, ProtocolNodeKind::Attempt);
    assert_eq!(declaration.name.name, "Tried");
}

/// SR-770 FND-001/FND-008: S2 records what S3 needs to decide
/// whether it checks a protocol in full -- the `using` alias, each role,
/// each binder's declared type, each body block (whether it is a bare
/// Boolean literal) and every other construct by kind, in source order --
/// and reads an attempt's role after `by` and its `contracts` only between
/// the brackets, so the `related by` identifier and the role never become
/// contract entries.
#[trace("TC-513")]
#[test]
fn s2_records_the_parts_s3_reads_to_decide_coverage() {
    let source = "protocol Covered using v over (input: Boolean) \
         on each (activated: Boolean) when (true) {\n\
         capture logged: Boolean = input;\n\
         role R on Config::ConfigVersion;\n\
         role W each Config::ConfigVersion from R max 2 lifetime workflow;\n\
         channel C from R to W carries Boolean ordering unordered delivery at-most-once \
         capacity 1 overflow reject;\n\
         requires temporal T;\n\
         run sequence Main {\n\
         attempt Tried by W on Config::ConfigVersion::attemptUpdate contracts [Kept] \
         as (tried: Config::ConfigVersion) related by rel (true, true) { tried };\n\
         }\n\
         finish End as (outcome: Boolean) { false };\n\
         }";
    let (text, unit) = build(source);
    let form = protocol_form(unit.forms()[0].form());
    let at = |span: Span| &text[span.start..span.end];

    assert_eq!(form.using.alias, "v");
    assert_eq!(at(form.using.span), "v");
    let roles: Vec<(&str, &str)> = form
        .roles
        .iter()
        .map(|role| (role.name.name.as_str(), role.context.name.as_str()))
        .collect();
    assert_eq!(
        roles,
        [
            ("R", "Config::ConfigVersion"),
            ("W", "Config::ConfigVersion")
        ]
    );

    let constructs: Vec<ProtocolConstructKind> = form
        .constructs
        .iter()
        .map(|construct| construct.kind)
        .collect();
    assert_eq!(
        constructs,
        [
            ProtocolConstructKind::ActivationEach,
            ProtocolConstructKind::Capture,
            ProtocolConstructKind::ReplicatedRole,
            ProtocolConstructKind::Channel,
            ProtocolConstructKind::Requirement,
            ProtocolConstructKind::Related,
        ]
    );
    assert!(at(form.constructs[3].span).starts_with("channel C"));
    assert!(at(form.constructs[5].span).starts_with("related by rel"));

    let bodies: Vec<(&str, Option<bool>)> = form
        .bodies
        .iter()
        .map(|body| (at(body.span), body.constant))
        .collect();
    assert_eq!(bodies, [("tried", None), ("false", Some(false))]);
    assert_eq!(
        form.declarations[form.bodies[0].owner].kind,
        ProtocolNodeKind::Attempt
    );
    assert_eq!(
        form.declarations[form.bodies[1].owner].kind,
        ProtocolNodeKind::Finish
    );

    let [attempt] = form.attempts.as_slice() else {
        panic!("one attempt: {:?}", form.attempts);
    };
    assert_eq!(attempt.role.name, "W");
    assert_eq!(at(attempt.role.span), "W");
    let contracts: Vec<&str> = attempt.contracts.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(contracts, ["Kept"]);

    let tried = binder(form, "tried", BinderKind::RecordBinder, &["Main"]);
    assert_eq!(
        tried.value_type.head,
        TypeFormHead::Name("Config::ConfigVersion".to_owned())
    );
    let input = binder(form, "input", BinderKind::Input, &[]);
    assert_eq!(
        input.value_type.head,
        TypeFormHead::Builtin(BuiltinType::Boolean)
    );
}

/// FR-114-AC-2: `contracts []` still builds an `AttemptForm`, with
/// an empty `contracts` list rather than none at all.
#[trace("TC-513")]
#[test]
fn s2_admits_an_empty_contracts_list() {
    let (_, unit) = build(&recovery_flow("Main::Committed", true, ""));
    let form = protocol_form(unit.forms()[0].form());
    assert_eq!(form.attempts.len(), 1, "{:?}", form.attempts);
    assert!(form.attempts[0].contracts.is_empty());
}

#[trace("TC-510", "FR-112-AC-2")]
#[test]
fn a_reference_in_a_nested_control_records_every_enclosing_named_control() {
    let parallel = "parallel Both {\n\
         branch left await Wait after Sent using v clock \"ticks\" within [0,1] \
         match send Ping via Messages as (ping: Config::ConfigVersion) { true }; \
         then sequence Then { } timeout sequence Timeout { }\n\
         branch right sequence Right { }\n\
         } join all [left,right];\n";

    let (_, unit) = build(&recovery_flow("Main::Committed", true, parallel));
    let form = protocol_form(unit.forms()[0].form());
    let await_after = form
        .scoped_anchors
        .iter()
        .find(|anchor| anchor.site == AnchorSite::AwaitAfter)
        .expect("an await-after anchor");
    assert_eq!(segment_texts(await_after), ["Sent"]);
    assert_eq!(scope_names(await_after), ["Main", "Both", "left"]);

    // `commit never;` builds no `compensate-commit` anchor.
    let (_, unit) = build(&recovery_flow("never", true, parallel));
    let form = protocol_form(unit.forms()[0].form());
    assert!(form
        .scoped_anchors
        .iter()
        .all(|anchor| anchor.site != AnchorSite::CompensateCommit));
}

#[trace("TC-510", "FR-112-AC-3")]
#[test]
fn building_twice_gives_equal_forms_independent_of_a_named_targets_existence() {
    let source = recovery_flow("Main::Committed", true, "");
    let (_, unit_a) = build(&source);
    let (_, unit_b) = build(&source);
    assert_eq!(
        protocol_form(unit_a.forms()[0].form()),
        protocol_form(unit_b.forms()[0].form())
    );

    // Removing the `effect Applied` node -- the declaration named
    // `Applied`, which `compensate ... for Main::Applied` merely spells --
    // leaves every other scoped anchor unchanged: S2 resolves nothing, so
    // a reference's own form never depends on whether its named target
    // exists.
    let without_effect = recovery_flow("Main::Committed", false, "");
    let (_, unit_c) = build(&without_effect);
    let form_a = protocol_form(unit_a.forms()[0].form());
    let form_c = protocol_form(unit_c.forms()[0].form());
    assert_eq!(form_c.scoped_anchors.len(), 3);
    // `compensate-for` and `compensate-commit` are written before the
    // removed `effect Applied`, so their own spans are unaffected: they
    // build byte-for-byte the same ScopedAnchorForm either way.
    assert_eq!(form_c.scoped_anchors[0], form_a.scoped_anchors[0]);
    assert_eq!(form_c.scoped_anchors[1], form_a.scoped_anchors[1]);
    // `event-for` is written after the removed `effect Applied`, so its
    // span shifts left with the source; its site, segments and scope names
    // -- the content S2 actually resolves nothing about -- stay the same.
    let event_for_c = &form_c.scoped_anchors[2];
    let event_for_a = &form_a.scoped_anchors[3];
    assert_eq!(event_for_c.site, event_for_a.site);
    assert_eq!(segment_texts(event_for_c), segment_texts(event_for_a));
    assert_eq!(scope_names(event_for_c), scope_names(event_for_a));
}

/// SR-753/SR-754 FND-001: FR-113 Inputs says a control declares the names
/// of its direct child controls and event nodes, and lists `await` among
/// the structural controls, so an await's matched event template and its
/// `then`/`timeout` branches are its own children -- unlike the await's
/// own `after` reference, which sits in the scope enclosing the await
/// (asserted by FR-112-AC-2 already; re-asserted here for contrast).
#[trace("TC-510", "FR-112")]
#[test]
fn an_awaits_matched_event_and_branches_are_scoped_inside_the_await() {
    let awaiting = "await Wait after Sent using v clock \"ticks\" within [0,1] \
         match receive Got via Messages of Sent as (got: Config::ConfigVersion) { true }; \
         then sequence Then {\n\
         effect Applied of Tried as (applied: Config::ConfigVersion) { true };\n\
         } timeout sequence Timeout { }\n";
    let (_, unit) = build(&recovery_flow("Main::Committed", false, awaiting));
    let form = protocol_form(unit.forms()[0].form());

    let await_after = form
        .scoped_anchors
        .iter()
        .find(|anchor| anchor.site == AnchorSite::AwaitAfter)
        .expect("the await-after anchor");
    assert_eq!(segment_texts(await_after), ["Sent"]);
    assert_eq!(scope_names(await_after), ["Main"]);

    let receive_of = form
        .scoped_anchors
        .iter()
        .find(|anchor| anchor.site == AnchorSite::ReceiveOf)
        .expect("the await's matched event's receive-of anchor");
    assert_eq!(segment_texts(receive_of), ["Sent"]);
    assert_eq!(scope_names(receive_of), ["Main", "Wait"]);

    let effect_of = form
        .scoped_anchors
        .iter()
        .find(|anchor| anchor.site == AnchorSite::EffectOf)
        .expect("the await's then-branch effect-of anchor");
    assert_eq!(scope_names(effect_of), ["Main", "Wait", "Then"]);
}

/// SR-753 FND-004: no test built a `choice`/`case` or a `repeat`, so a
/// missed scope push in either was untested. `choice`'s and `repeat`'s own
/// names, and each `case`'s, enclose their bodies the same way `parallel`'s
/// and `branch`'s do (FR-112-AC-2).
#[trace("TC-510", "FR-112")]
#[test]
fn choice_and_repeat_named_controls_enclose_their_bodies() {
    let choice = "choice Decision by R visible () {\n\
         case yes when { true } sequence Yes {\n\
         effect Applied of Tried as (applied: Config::ConfigVersion) { true };\n\
         }\n\
         case no when { false } sequence No { }\n\
         }\n";
    let (_, unit) = build(&recovery_flow("Main::Committed", false, choice));
    let form = protocol_form(unit.forms()[0].form());
    let effect_of = form
        .scoped_anchors
        .iter()
        .find(|anchor| anchor.site == AnchorSite::EffectOf)
        .expect("the choice case's effect-of anchor");
    assert_eq!(scope_names(effect_of), ["Main", "Decision", "yes", "Yes"]);

    let repeat = "repeat Loop by R visible (true) max 2 invariant { true } variant { 1 } \
         while { false } sequence Body {\n\
         effect Applied of Tried as (applied: Config::ConfigVersion) { true };\n\
         } exhausted sequence Exhausted {\n\
         event Recovered by R for Undo as (notice: Config::ConfigVersion) { true };\n\
         }\n";
    let (_, unit) = build(&recovery_flow("Main::Committed", false, repeat));
    let form = protocol_form(unit.forms()[0].form());
    let effect_of = form
        .scoped_anchors
        .iter()
        .find(|anchor| anchor.site == AnchorSite::EffectOf)
        .expect("the repeat body's effect-of anchor");
    assert_eq!(scope_names(effect_of), ["Main", "Loop", "Body"]);
    // `event-for` appears twice here (the repeat's own `exhausted` branch,
    // and `recovery_flow`'s trailing `event Recovered ... for Undo`, at
    // `["Main"]`); the repeat's own is the one nested three levels deep.
    let event_for = form
        .scoped_anchors
        .iter()
        .find(|anchor| anchor.site == AnchorSite::EventFor && anchor.scope.len() == 3)
        .expect("the repeat's exhausted-branch event-for anchor");
    assert_eq!(scope_names(event_for), ["Main", "Loop", "Exhausted"]);
}

/// The one declaration named `name` of kind `kind` in `form.declarations`.
fn decl<'a>(
    form: &'a ProtocolDeclarationForm,
    name: &str,
    kind: ProtocolNodeKind,
) -> &'a ProtocolNodeDeclaration {
    let matches: Vec<&ProtocolNodeDeclaration> = form
        .declarations
        .iter()
        .filter(|declaration| declaration.name.name == name && declaration.kind == kind)
        .collect();
    match matches[..] {
        [declaration] => declaration,
        _ => panic!(
            "expected exactly one {kind:?} named {name}, found {}: {:?}",
            matches.len(),
            form.declarations
        ),
    }
}

fn decl_scope(declaration: &ProtocolNodeDeclaration) -> Vec<&str> {
    declaration
        .scope
        .iter()
        .map(|name| name.name.as_str())
        .collect()
}

/// SR-761 FND-003: the S2 declaration collection (FR-113 Inputs) had no
/// direct test -- only the anchors built alongside it were asserted, so
/// removing a `declare(...)` call for `choice`, `case`, `parallel`,
/// `branch`, `repeat`, `check` or `finish` passed every existing test. This
/// exercises every control kind and asserts each declaration's own name,
/// kind and scope.
#[trace("TC-510", "FR-113")]
#[test]
fn s2_builds_a_declaration_for_every_control_kind() {
    let controls = "choice Decision by R visible () {\n\
         case yes when { true } sequence Yes { }\n\
         case no when { false } sequence No { }\n\
         }\n\
         parallel Both {\n\
         branch left sequence Left { }\n\
         branch right sequence Right { }\n\
         } join all [left,right];\n\
         repeat Loop by R visible (true) max 2 invariant { true } variant { 1 } \
         while { false } sequence Body { } exhausted sequence Exhausted { }\n\
         check Guard using v { true };\n";
    let (_, unit) = build(&recovery_flow("Main::Committed", true, controls));
    let form = protocol_form(unit.forms()[0].form());

    // Top level: the `run` control itself, the `finish` node and the
    // `compensate` template -- FR-113 Inputs: "the top level declares the
    // protocol's `run` control, its `finish` node and its `compensate`
    // templates".
    assert_eq!(
        decl_scope(decl(form, "Main", ProtocolNodeKind::Sequence)),
        Vec::<&str>::new()
    );
    assert_eq!(
        decl_scope(decl(form, "End", ProtocolNodeKind::Finish)),
        Vec::<&str>::new()
    );
    assert_eq!(
        decl_scope(decl(form, "Undo", ProtocolNodeKind::CompensateTemplate)),
        Vec::<&str>::new()
    );

    // Main declares its direct children: every event node and control
    // written directly inside it.
    for (name, kind) in [
        ("Tried", ProtocolNodeKind::Attempt),
        ("Decision", ProtocolNodeKind::Choice),
        ("Both", ProtocolNodeKind::Parallel),
        ("Loop", ProtocolNodeKind::Repeat),
        ("Guard", ProtocolNodeKind::Check),
        ("Applied", ProtocolNodeKind::Effect),
        ("Recovered", ProtocolNodeKind::Event),
        ("Committed", ProtocolNodeKind::Commit),
    ] {
        assert_eq!(decl_scope(decl(form, name, kind)), ["Main"], "{name}");
    }

    // `choice` declares its `case`s directly; each `case` declares its own
    // inner control.
    assert_eq!(
        decl_scope(decl(form, "yes", ProtocolNodeKind::Case)),
        ["Main", "Decision"]
    );
    assert_eq!(
        decl_scope(decl(form, "no", ProtocolNodeKind::Case)),
        ["Main", "Decision"]
    );
    assert_eq!(
        decl_scope(decl(form, "Yes", ProtocolNodeKind::Sequence)),
        ["Main", "Decision", "yes"]
    );
    assert_eq!(
        decl_scope(decl(form, "No", ProtocolNodeKind::Sequence)),
        ["Main", "Decision", "no"]
    );

    // `parallel` declares its `branch`es directly; each `branch` declares
    // its own inner control.
    assert_eq!(
        decl_scope(decl(form, "left", ProtocolNodeKind::Branch)),
        ["Main", "Both"]
    );
    assert_eq!(
        decl_scope(decl(form, "right", ProtocolNodeKind::Branch)),
        ["Main", "Both"]
    );
    assert_eq!(
        decl_scope(decl(form, "Left", ProtocolNodeKind::Sequence)),
        ["Main", "Both", "left"]
    );
    assert_eq!(
        decl_scope(decl(form, "Right", ProtocolNodeKind::Sequence)),
        ["Main", "Both", "right"]
    );

    // `repeat` declares both its `while`-body control and its `exhausted`
    // control directly.
    assert_eq!(
        decl_scope(decl(form, "Body", ProtocolNodeKind::Sequence)),
        ["Main", "Loop"]
    );
    assert_eq!(
        decl_scope(decl(form, "Exhausted", ProtocolNodeKind::Sequence)),
        ["Main", "Loop"]
    );
}

/// SR-761 FND-003: `ScopedAnchorForm::channel` and
/// `ProtocolNodeDeclaration::channel` had no test. Only `send` and
/// `receive` carry a channel; every other kind carries `None`, and a
/// `receive-of` anchor carries its own owning `receive`'s channel while
/// every other site carries none.
#[trace("TC-510", "FR-113")]
#[test]
fn s2_builds_the_channel_of_a_send_and_receive_only() {
    let source = "protocol Handoff using v over (input: Config::ConfigVersion) on origin {\n\
         role R on Config::ConfigVersion;\n\
         run sequence Main {\n\
         send Ping via C as (ping: Config::ConfigVersion) { true };\n\
         receive Got via D of Ping as (got: Config::ConfigVersion) { true };\n\
         }\n\
         finish End as (outcome: Boolean) { true };\n\
         }";
    let (_, unit) = build(source);
    let form = protocol_form(unit.forms()[0].form());

    let send = decl(form, "Ping", ProtocolNodeKind::Send);
    assert_eq!(send.channel.as_deref(), Some("C"));
    let receive = decl(form, "Got", ProtocolNodeKind::Receive);
    assert_eq!(receive.channel.as_deref(), Some("D"));
    // Every other declared kind carries no channel.
    assert_eq!(decl(form, "Main", ProtocolNodeKind::Sequence).channel, None);
    assert_eq!(decl(form, "End", ProtocolNodeKind::Finish).channel, None);

    let receive_of = form
        .scoped_anchors
        .iter()
        .find(|anchor| anchor.site == AnchorSite::ReceiveOf)
        .expect("the receive's own receive-of anchor");
    assert_eq!(receive_of.channel.as_deref(), Some("D"));
}

/// SR-753 FND-003: `await ... then <Control>` and `repeat ... exhausted
/// <Control>` nest with no bracket, so only an explicit depth charge --
/// not the CST's own bracket-based nesting ceiling -- bounds a chain of
/// them. A chain past the limit refuses instead of recursing without
/// bound (a 500-await chain aborts the process on a small stack without
/// this charge).
#[trace("TC-510", "FR-112")]
#[test]
fn the_control_walk_refuses_past_its_nesting_depth_limit_instead_of_recursing_unbounded() {
    let limits = FormsLimits { nesting_depth: 6 };
    let awaits = |count: u64| {
        let mut control = "sequence End { }".to_owned();
        for i in 0..count {
            control = format!(
                "await W{i} after Sent using v clock \"ticks\" within [0,1] \
                 match send Ping via Messages as (ping: Config::ConfigVersion) {{ true }}; \
                 then {control} timeout sequence T{i} {{ }}"
            );
        }
        control
    };
    let source = |count: u64| {
        format!(
            "protocol Chain using v over (input: Config::ConfigVersion) on origin {{\n\
             role R on Config::ConfigVersion;\n\
             run sequence Main {{ {} }}\n\
             finish End as (outcome: Boolean) {{ true }};\n\
             }}",
            awaits(count)
        )
    };

    let (_, parsed) = admissible(&source(2));
    build_unit(&parsed, limits).expect("a short chain of awaits builds");

    let (text, parsed) = admissible(&source(20));
    match build_unit(&parsed, limits) {
        Err(FormsFailure::Limit { limit, .. }) => {
            assert_eq!(limit.kind(), LimitKind::NestingDepth);
        }
        other => panic!("{text}: a limit refusal, not {other:?}"),
    }
}

/// A compact rendering of an expression tree, operands in order (mirrors
/// `value_forms::show`, extended with the three forms this ticket adds).
fn show(expression: &Expression) -> String {
    let children: Vec<String> = expression.children().into_iter().map(show).collect();
    let head = match expression {
        Expression::Boolean(value) => format!("{value}"),
        Expression::Integer(value) => format!("{value}"),
        Expression::Name(spelling) => spelling.clone(),
        Expression::Binary { operator, .. } => format!("{operator:?}"),
        Expression::Not(_) => "Not".into(),
        Expression::Field { field, .. } => format!("Field[{field}]"),
        Expression::Pre(_) => "Pre".into(),
        Expression::SelfRef => "SelfRef".into(),
        Expression::Result => "Result".into(),
        Expression::Reaches { edge, .. } => format!("Reaches[{edge}]"),
        other => panic!("this fixture's bodies use no other form: {other:?}"),
    };
    if children.is_empty() {
        head
    } else {
        format!("{head}({})", children.join(", "))
    }
}

#[trace("TC-456", "FR-102-AC-1")]
#[test]
fn an_invariant_builds_one_state_clause_form_with_no_operation() {
    let (text, unit) = build(
        "invariant ParentOrder using v on Config::ConfigVersion at current \
         { present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }",
    );
    let form = state_clause(unit.forms()[0].form());
    assert_eq!(form.kind, StateClauseKind::Invariant);
    assert_eq!(form.name.name, "ParentOrder");
    assert_eq!(form.profile.alias, "v");
    assert_eq!(form.context.name, "Config::ConfigVersion");
    assert!(form.operation.is_none());

    // Every sub-expression's span slices the source to its own text.
    let spans = &form.spans.body;
    let whole_body =
        "present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber";
    let root = spans.span(spans.root()).expect("the root span");
    assert_eq!(&text[root.start..root.end], whole_body);
    let present_call_span = {
        let start = text
            .find("present(self.parent)")
            .expect("present() is in the unit");
        Span {
            start,
            end: start + "present(self.parent)".len(),
        }
    };
    let left = spans
        .span(spans.child(spans.root(), 0).unwrap())
        .expect("the implies' left operand span");
    assert_eq!(left, present_call_span);
    assert_eq!(&text[left.start..left.end], "present(self.parent)");

    // Every node, not just the root and its left child (SR-722 FND-007).
    assert_every_span_slice_reparses(&text, spans, spans.root(), &form.body);
}

#[trace("TC-456", "FR-102-AC-2")]
#[test]
fn pre_and_post_build_their_own_kind_with_the_operation_member() {
    let (_, unit) = build("pre P using v on Config::ConfigVersion::attemptUpdate { true }");
    let form = state_clause(unit.forms()[0].form());
    assert_eq!(form.kind, StateClauseKind::Precondition);
    assert_eq!(form.context.name, "Config::ConfigVersion");
    assert_eq!(
        form.operation.as_ref().map(|o| o.name.as_str()),
        Some("attemptUpdate")
    );

    let (_, unit) = build("post P using v on Config::ConfigVersion::attemptUpdate { true }");
    let form = state_clause(unit.forms()[0].form());
    assert_eq!(form.kind, StateClauseKind::Postcondition);
    assert_eq!(
        form.operation.as_ref().map(|o| o.name.as_str()),
        Some("attemptUpdate")
    );
}

#[trace("TC-456", "FR-102-AC-3")]
#[test]
fn self_result_and_reaches_build_and_a_qualified_edge_refuses() {
    let (_, unit) = build(
        "invariant NoCycle using v on Config::ConfigVersion at current \
         { not reaches(self, self, parent) }",
    );
    let form = state_clause(unit.forms()[0].form());
    assert_eq!(show(&form.body), "Not(Reaches[parent](SelfRef, SelfRef))");

    let (_, unit) = build(
        "post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate \
         { self.versionNumber = pre(self.versionNumber) }",
    );
    let form = state_clause(unit.forms()[0].form());
    assert_eq!(
        show(&form.body),
        "Equal(Field[versionNumber](SelfRef), Pre(Field[versionNumber](SelfRef)))"
    );

    let (_, unit) = build("post R using v on Config::ConfigVersion::attemptUpdate { result }");
    let form = state_clause(unit.forms()[0].form());
    assert!(matches!(form.body, Expression::Result));

    let (text, cause, span) = refusal(
        "invariant Q using v on Config::ConfigVersion at current \
         { reaches(self, self, Config::ConfigVersion::parent) }",
    );
    assert_eq!(
        cause,
        FormsCause::UnrepresentedConstruct {
            production: qsl_cst::Production::QualifiedName
        }
    );
    let needle = "Config::ConfigVersion::parent";
    let start = text.rfind(needle).expect("the edge is in the unit");
    assert_eq!(
        span,
        Some(Span {
            start,
            end: start + needle.len()
        })
    );
}

#[trace("TC-457", "FR-102-AC-5")]
#[test]
fn a_state_clause_body_obeys_the_same_nesting_depth_limit_as_a_function_body() {
    // A bare `(e)` is a transparent pass-through (`value`'s own doc: "a
    // single-operand precedence level maps to its operand"), so it adds no
    // arena node and no depth; `not` is the nesting device `value_forms`'s
    // own FR-091 depth tests use for exactly this reason.
    let limits = FormsLimits { nesting_depth: 4 };
    let nots = |count: u64| "not ".repeat(count as usize) + "true";
    let source = |count: u64| {
        format!(
            "invariant N using v on Config::ConfigVersion at current {{ {} }}",
            nots(count)
        )
    };

    // `nesting_depth` counts the root at depth 1, so `limit - 1` `not`s
    // reach exactly the bound (the leaf `true` at depth `limit`).
    let (_, parsed) = admissible(&source(limits.nesting_depth - 1));
    build_unit(&parsed, limits).expect("exactly at the limit builds");

    let (text, parsed) = admissible(&source(limits.nesting_depth));
    match build_unit(&parsed, limits) {
        Err(FormsFailure::Limit { limit, span }) => {
            assert_eq!(limit.kind(), LimitKind::NestingDepth);
            let expected_start = text.find("true").expect("the leaf is in the unit");
            assert_eq!(span.start, expected_start);
        }
        other => panic!("a limit refusal, not {other:?}"),
    }
}
