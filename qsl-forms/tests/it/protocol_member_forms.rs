// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-308-AC-7 to AC-12 (TC-1055): S2 builds the capture, relationship,
//! `requires temporal`, `related by` and compensation forms of a protocol,
//! in the field of their position and in source order, on real complete-V1
//! source through `qsl_cst::parse` and `build_unit`.

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, ActivationForm, CaptureForm, CompensationCommit, CompensationForm, DeclarationForm,
    ExprNode, ExpressionForm, FormsCause, IntervalUpper, ParsedUnit, ProtocolDeclarationForm,
    ProtocolNodeDeclaration, TypeFormHead,
};
use qsl_foundation::{Code, SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    model Config = \"example/config-version\" version \"1\" digest \
    \"sha256-jcs:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

const OBJECT: &str = "Config::ConfigVersion";

/// The run control every protocol here holds unless a test writes its own.
const RUN: &str = "attempt Tried by R on Config::ConfigVersion::attemptUpdate contracts [] \
    as (tried: Config::ConfigVersion) { true };";

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

/// One protocol: `captures` before its role, `members` after it, then
/// `run`.
fn source(captures: &str, members: &str, run: &str) -> String {
    format!(
        "{HEADER}protocol Flow using v over (input: {OBJECT}) on origin {{\n\
         {captures}\n\
         role R on {OBJECT};\n\
         {members}\n\
         run {run}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    )
}

/// One `compensate` template whose variable parts the test writes.
fn compensation(
    registration: &str,
    activation: &str,
    within: &str,
    attempts: &str,
    commit: &str,
) -> String {
    format!(
        "compensate Undo for Main::Applied as (forward: {OBJECT}) by R \
         on {OBJECT}::attemptUpdate using v clock \"ticks\" {{\n\
         {registration}\n\
         activate first (trigger: {OBJECT}) when {{ true }} {{ {activation} }}\n\
         within {within}; attempts {attempts} of {OBJECT};\n\
         retry (earlier: {OBJECT}, later: {OBJECT}) {{ true }};\n\
         commit {commit}; recover (recovered: {OBJECT}) {{ true }}; }}"
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

fn capture_names(captures: &[CaptureForm]) -> Vec<&str> {
    captures
        .iter()
        .map(|capture| capture.parameter.name.name.as_str())
        .collect()
}

fn declaration<'a>(form: &'a ProtocolDeclarationForm, name: &str) -> &'a ProtocolNodeDeclaration {
    form.declarations
        .iter()
        .find(|declaration| declaration.name.name == name)
        .unwrap_or_else(|| panic!("a declaration named {name}"))
}

fn only_compensation(form: &ProtocolDeclarationForm) -> &CompensationForm {
    let [compensation] = &form.compensations[..] else {
        panic!("one compensation: {:?}", form.compensations);
    };
    compensation
}

/// FR-308-AC-7 (TC-1055 steps 1 and 2): a capture builds into the field of
/// its position, with name, type, initializer and span; the registration
/// captures keep their source order.
#[trace("TC-1055", "FR-308-AC-7")]
#[test]
fn a_capture_builds_into_the_field_of_its_position() {
    let captures = format!("capture order: {OBJECT} = input.order;");
    let undo = compensation(
        &format!("capture target: {OBJECT} = forward.total;\ncapture floor: {OBJECT} = 0;"),
        "capture activated: Boolean = trigger.ready;",
        "[0,30]",
        "3",
        "Main::Committed",
    );
    let text = source(&captures, &undo, RUN);
    let unit = build(&text);
    let form = protocol(&unit);

    assert_eq!(capture_names(&form.captures), ["order"]);
    let order = &form.captures[0];
    assert_eq!(
        slice(&text, order.span),
        format!("capture order: {OBJECT} = input.order;")
    );
    assert_eq!(
        order.parameter.value_type.head,
        TypeFormHead::Name(OBJECT.to_owned())
    );
    assert_eq!(expression_text(&text, &order.value), "input.order");
    assert!(matches!(
        order.value.expression.root_node(),
        ExprNode::Field { field, .. } if field == "order"
    ));

    let undo = only_compensation(form);
    assert_eq!(
        capture_names(&undo.registration_captures),
        ["target", "floor"]
    );
    assert_eq!(
        slice(&text, undo.registration_captures[0].span),
        format!("capture target: {OBJECT} = forward.total;")
    );
    assert_eq!(
        expression_text(&text, &undo.registration_captures[1].value),
        "0"
    );
    assert_eq!(capture_names(&undo.activation_captures), ["activated"]);
    let activated = &undo.activation_captures[0];
    assert_eq!(
        slice(&text, activated.span),
        "capture activated: Boolean = trigger.ready;"
    );
    assert_eq!(expression_text(&text, &activated.value), "trigger.ready");
}

/// FR-308-AC-8 (TC-1055 step 3): two relationship members build two forms
/// in source order, each span slicing to its text.
#[trace("TC-1055", "FR-308-AC-8")]
#[test]
fn a_relationship_builds_its_name_and_target() {
    let members = "relationship Pipe = Config::pipe;\nrelationship Drain = Config::drain;";
    let text = source("", members, RUN);
    let unit = build(&text);
    let form = protocol(&unit);

    let [pipe, drain] = &form.relationships[..] else {
        panic!("two relationships: {:?}", form.relationships);
    };
    assert_eq!(pipe.name.name, "Pipe");
    assert_eq!(slice(&text, pipe.name.span), "Pipe");
    assert_eq!(pipe.target.name, "Config::pipe");
    assert_eq!(slice(&text, pipe.target.span), "Config::pipe");
    assert_eq!(slice(&text, pipe.span), "relationship Pipe = Config::pipe;");
    assert_eq!(drain.name.name, "Drain");
    assert_eq!(drain.target.name, "Config::drain");
    assert_eq!(
        slice(&text, drain.span),
        "relationship Drain = Config::drain;"
    );
}

/// FR-308-AC-9 (TC-1055 step 4): a requirement written twice builds twice,
/// in source order, merged and resolved to nothing.
#[trace("TC-1055", "FR-308-AC-9")]
#[test]
fn a_repeated_requirement_builds_once_per_member() {
    let members = "requires temporal Due;\nrequires temporal Late;\nrequires temporal Due;";
    let text = source("", members, RUN);
    let unit = build(&text);
    let form = protocol(&unit);

    let names: Vec<&str> = form
        .requirements
        .iter()
        .map(|requirement| requirement.temporal.name.as_str())
        .collect();
    assert_eq!(names, ["Due", "Late", "Due"]);
    let spans: Vec<&str> = form
        .requirements
        .iter()
        .map(|requirement| slice(&text, requirement.span))
        .collect();
    assert_eq!(
        spans,
        [
            "requires temporal Due;",
            "requires temporal Late;",
            "requires temporal Due;"
        ]
    );
    assert!(form.requirements[0].span.start < form.requirements[2].span.start);
}

/// FR-308-AC-10 (TC-1055 steps 5 and 7): an `effect` node's `related by`
/// builds one form with both ends in written order; a call end is the
/// call's whole expression form.
#[trace("TC-1055", "FR-308-AC-10")]
#[test]
fn a_related_clause_builds_both_ends_as_expression_forms() {
    let run = format!(
        "effect Applied of Main::Tried as (applied: {OBJECT}) \
         related by Pays (applied.order, input.order) {{ true }};"
    );
    let text = source("", "", &run);
    let unit = build(&text);
    let [related] = &declaration(protocol(&unit), "Applied").related[..] else {
        panic!("one related form");
    };
    assert_eq!(related.relationship.name, "Pays");
    assert_eq!(
        slice(&text, related.span),
        "related by Pays (applied.order, input.order)"
    );
    assert_eq!(expression_text(&text, &related.ends[0]), "applied.order");
    assert_eq!(expression_text(&text, &related.ends[1]), "input.order");

    let run = format!(
        "attempt Tried by R on {OBJECT}::attemptUpdate contracts [] as (attempted: {OBJECT}) \
         related by Pays (a, b) related by Pays (c, d) related by Owns (e, f) {{ true }};"
    );
    let text = source("", "", &run);
    let unit = build(&text);
    let clauses = &declaration(protocol(&unit), "Tried").related;
    let written: Vec<(&str, &str, &str)> = clauses
        .iter()
        .map(|clause| {
            (
                clause.relationship.name.as_str(),
                expression_text(&text, &clause.ends[0]),
                expression_text(&text, &clause.ends[1]),
            )
        })
        .collect();
    assert_eq!(
        written,
        [("Pays", "a", "b"), ("Pays", "c", "d"), ("Owns", "e", "f")]
    );

    let run = format!(
        "effect Applied of Main::Tried as (applied: {OBJECT}) \
         related by Pays (f(applied), input.order) {{ true }};"
    );
    let text = source("", "", &run);
    let unit = build(&text);
    let [related] = &declaration(protocol(&unit), "Applied").related[..] else {
        panic!("one related form");
    };
    assert_eq!(expression_text(&text, &related.ends[0]), "f(applied)");
    assert!(matches!(
        related.ends[0].expression.root_node(),
        ExprNode::Call { name, arguments } if name == "f" && arguments.len() == 1
    ));
}

/// FR-308-AC-11 (TC-1055 steps 8 to 11): every compensation member is
/// built; `commit never` is `Never`; a zero count and an inverted interval
/// build as written; a count past `u64` fails at S1.
#[trace("TC-1055", "FR-308-AC-11")]
#[test]
fn a_compensation_builds_every_member_as_written() {
    let undo = compensation("", "", "[0,30]", "3", "Main::Committed");
    let text = source("", &undo, RUN);
    let unit = build(&text);
    let undo = only_compensation(protocol(&unit));
    assert_eq!(undo.name.name, "Undo");
    assert_eq!(undo.target.name, "Main::Applied");
    assert_eq!(undo.forward.name.name, "forward");
    assert_eq!(undo.role.name, "R");
    assert_eq!(undo.operation.name, format!("{OBJECT}::attemptUpdate"));
    assert_eq!(undo.using.alias, "v");
    assert_eq!(undo.clock.text, "ticks");
    assert_eq!(slice(&text, undo.clock.span), "\"ticks\"");
    assert_eq!(undo.trigger.name.name, "trigger");
    assert_eq!(expression_text(&text, &undo.guard), "true");
    assert_eq!(
        (undo.within.lower, undo.within.upper),
        (0, IntervalUpper::Finite(30))
    );
    assert_eq!(slice(&text, undo.within.span), "[0,30]");
    assert_eq!(undo.attempts.count, 3);
    assert_eq!(slice(&text, undo.attempts.count_span), "3");
    assert_eq!(
        undo.attempts.record.head,
        TypeFormHead::Name(OBJECT.to_owned())
    );
    assert_eq!(undo.retry.earlier.name.name, "earlier");
    assert_eq!(undo.retry.later.name.name, "later");
    assert_eq!(expression_text(&text, &undo.retry.relation), "true");
    let CompensationCommit::Node(node) = &undo.commit else {
        panic!("a node commit: {:?}", undo.commit);
    };
    assert_eq!(node.name, "Main::Committed");
    assert_eq!(slice(&text, node.span), "Main::Committed");
    assert_eq!(undo.recover.binder.name.name, "recovered");
    assert_eq!(expression_text(&text, &undo.recover.predicate), "true");
    assert!(slice(&text, undo.span).starts_with("compensate Undo"));

    let undo = compensation("", "", "[0,30]", "3", "never");
    let text = source("", &undo, RUN);
    let unit = build(&text);
    let CompensationCommit::Never(span) = &only_compensation(protocol(&unit)).commit else {
        panic!("a never commit");
    };
    assert_eq!(slice(&text, *span), "never");

    let undo = compensation("", "", "[31,30]", "0", "never");
    let text = source("", &undo, RUN);
    let unit = build(&text);
    let undo = only_compensation(protocol(&unit));
    assert_eq!(undo.attempts.count, 0);
    assert_eq!(
        (undo.within.lower, undo.within.upper),
        (31, IntervalUpper::Finite(30))
    );

    let count = "18446744073709551616";
    let undo = compensation("", "", "[0,30]", count, "never");
    let text = source("", &undo, RUN);
    let parsed = parse(&text);
    assert!(!parsed.is_admissible(), "{text}");
    assert_eq!(parsed.diagnostics()[0].code, Code::InvalidSyntax);
    let at = parsed.diagnostics()[0]
        .byte_span()
        .expect("the diagnostic is located")
        .start;
    assert_eq!(at, text.find(count).expect("the count is in the source"));
    assert!(build_unit(&parsed).is_err());
}

/// A protocol writing one of every member FR-308 AC-7 to AC-11 names.
fn full_source() -> String {
    let undo = compensation(
        &format!("capture target: {OBJECT} = forward.total;"),
        "capture activated: Boolean = trigger.ready;",
        "[0,30]",
        "3",
        "Main::Committed",
    );
    let members = format!("relationship Pipe = Config::pipe;\nrequires temporal Due;\n{undo}");
    let run = format!(
        "effect Applied of Main::Tried as (applied: {OBJECT}) \
         related by Pays (applied.order, input.order) {{ true }};"
    );
    source(
        &format!("capture order: {OBJECT} = input.order;"),
        &members,
        &run,
    )
}

/// FR-308-AC-12 (TC-1055 steps 12 and 13): a recovery node inside a
/// capture, a relationship, a requirement, a `related by` or a compensation
/// member refuses with `RecoveringCst` and no form; a protocol with all five
/// builds equal forms twice.
#[trace("TC-1055", "FR-308-AC-12")]
#[test]
fn a_recovering_member_refuses_and_a_whole_protocol_builds_equal_twice() {
    let full = full_source();
    for (written, broken) in [
        (
            format!("capture order: {OBJECT} = input.order;"),
            format!("capture order: {OBJECT} = input.order"),
        ),
        (
            "relationship Pipe = Config::pipe;".to_owned(),
            "relationship Pipe = Config::pipe".to_owned(),
        ),
        (
            "requires temporal Due;".to_owned(),
            "requires temporal Due".to_owned(),
        ),
        (
            "related by Pays (applied.order, input.order)".to_owned(),
            "related by Pays (applied.order, input.order".to_owned(),
        ),
        (
            format!("attempts 3 of {OBJECT};"),
            format!("attempts 3 of {OBJECT}"),
        ),
    ] {
        assert!(full.contains(&written), "{written}");
        let text = full.replacen(&written, &broken, 1);
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

/// A protocol's `activation` builds into its own field: `on origin` as
/// `Origin`, and `on each (x: T) when (e)` as `Each` with the binder, the
/// declared type as written and the guard with its span; a missing `when`
/// builds no guard.
#[test]
fn a_protocol_activation_builds_into_its_field() {
    let written = |activation: &str| {
        format!(
            "{HEADER}protocol Flow using v over (input: {OBJECT}) {activation} {{\n\
             role R on {OBJECT};\n\
             run {RUN}\n\
             finish End as (outcome: Boolean) {{ true }};\n\
             }}\n"
        )
    };

    let origin_text = written("on origin");
    let origin = build(&origin_text);
    assert_eq!(protocol(&origin).activation, ActivationForm::Origin);

    let each_text = written(&format!("on each (opened: {OBJECT}) when (opened.ready)"));
    let each = build(&each_text);
    let ActivationForm::Each { parameter, when } = &protocol(&each).activation else {
        panic!("an `on each` activation: {:?}", protocol(&each).activation);
    };
    assert_eq!(parameter.name.name, "opened");
    assert_eq!(slice(&each_text, parameter.name.span), "opened");
    let TypeFormHead::Name(type_name) = &parameter.value_type.head else {
        panic!("a named type: {:?}", parameter.value_type.head);
    };
    assert_eq!(type_name, OBJECT);
    let guard = when.as_ref().expect("the guard is written");
    assert_eq!(expression_text(&each_text, guard), "opened.ready");

    let bare_text = written(&format!("on each (opened: {OBJECT})"));
    let bare = build(&bare_text);
    let ActivationForm::Each { when, .. } = &protocol(&bare).activation else {
        panic!("an `on each` activation: {:?}", protocol(&bare).activation);
    };
    assert!(when.is_none());
}
