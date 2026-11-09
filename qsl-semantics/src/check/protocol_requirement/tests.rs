//! FR-218 "Requirements" (TC-1056, TC-1065) over fixture R of the protocol
//! obligation tests, whose header selects the temporal profile `t` a
//! temporal clause is written `using`.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use quire_semantic_value::checking::CheckingLimits;

use super::{check, CheckedRequirement, UnitDeclarations};
use crate::check::refusal::ProtocolAnchorCause;
use crate::check::{CheckCause, CheckRefusal, PackageDeclarations};

mod fixture {
    use crate as semantics;
    include!("../../../tests/fixtures/protocol_obligations.rs");
}
use fixture::{obligation_model, HEADER};

#[trace("TC-1065", "FR-218-AC-31")]
#[test]
fn join_kind_retains_first_declaration_and_diagnostic_keyword() {
    use super::UnitDeclarationKind::{Callable, Other};
    use qsl_forms::DeclarationKind::{Function, Predicate};

    for (first, expected_kind, expected_keyword) in [
        (Callable(Function), Some(Function), "function"),
        (Callable(Predicate), Some(Predicate), "predicate"),
        (Other("terminal"), None, "terminal"),
    ] {
        let declarations =
            UnitDeclarations::new(&[], &[], [("done", first), ("done", Callable(Function))]);
        assert_eq!(declarations.callable_kind("done"), expected_kind);
        assert_eq!(declarations.kind("done"), Some(expected_keyword));
        assert_eq!(declarations.callable_kind("missing"), None);
        assert_eq!(declarations.kind("missing"), None);
    }
}

/// A temporal clause `name` whose formula holds `condition`.
fn temporal(name: &str, condition: &str) -> String {
    format!(
        "temporal {name} using t over (c: Boolean) clock \"steps\" on origin \
         {{ always holds({condition}) }}\n"
    )
}

/// A protocol `name` with the `members` written before `run`.
fn protocol(name: &str, members: &str) -> String {
    format!(
        "protocol {name} using p over (k: M::Node) on origin {{\n\
         role w on M::Node;\n\
         {members}\n\
         run sequence Main {{\n\
         attempt A by w on M::Node::step contracts [] as (a: M::Flag) {{ true }};\n\
         }}\n\
         finish Done as (d: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

fn assemble(declarations: &str) -> (String, PackageDeclarations) {
    assemble_under(HEADER, declarations)
}

/// [`assemble`] under `header` in place of [`HEADER`].
fn assemble_under(header: &str, declarations: &str) -> (String, PackageDeclarations) {
    // A leading `profile` line is a header selection, written before the
    // model.
    let (profile, declarations) = match declarations.split_once('\n') {
        Some((first, rest)) if first.starts_with("profile ") => (format!("{first}\n"), rest),
        _ => (String::new(), declarations),
    };
    let header = header.replacen("model M", &format!("{profile}model M"), 1);
    let text = format!("{header}{declarations}");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let assembled = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        vec![obligation_model()],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: the assembler refuses: {refusal:?}"));
    (text, assembled)
}

/// The requirements of the protocol `name` of `declarations`, checked
/// alone.
fn requirements(
    declarations: &str,
    name: &str,
) -> Result<Vec<CheckedRequirement>, Vec<CheckRefusal>> {
    let (_, assembled) = assemble(declarations);
    let index = assembled
        .protocols
        .iter()
        .position(|protocol| protocol.name.name == name)
        .expect("the protocol");
    let unit = UnitDeclarations::new(&assembled.temporal_clauses, &assembled.protocols, []);
    check(
        index,
        &assembled.protocols,
        &assembled.temporal_clauses,
        &unit,
    )
}

/// The protocol refusals the whole check of `declarations` gives, with the
/// unit's text.
fn protocol_refusals(declarations: &str) -> (String, Vec<(ProtocolAnchorCause, CheckRefusal)>) {
    let (text, assembled) = assemble(declarations);
    let refusals = assembled
        .check(
            quire_semantic_value::checking::CheckMode::Linked,
            CheckingLimits::default(),
        )
        .err()
        .unwrap_or_default();
    let found = refusals
        .into_iter()
        .filter_map(|refusal| match &refusal.cause {
            CheckCause::ProtocolAnchor(cause) => Some(((**cause).clone(), refusal.clone())),
            _ => None,
        })
        .collect();
    (text, found)
}

/// The one protocol refusal of `declarations`, its catalog pair and the
/// text at its span.
fn only(
    declarations: &str,
) -> (
    ProtocolAnchorCause,
    (&'static str, Option<&'static str>),
    String,
) {
    let (text, found) = protocol_refusals(declarations);
    let [(cause, refusal)] = found.as_slice() else {
        panic!("{declarations}: one protocol refusal, got {found:?}");
    };
    let span = cause.span();
    (
        cause.clone(),
        (refusal.cause.code().as_str(), refusal.cause.cause()),
        text[span.start..span.end].to_owned(),
    )
}

/// FR-218-AC-16 but its import case: `requires temporal Due;` binds the
/// requirement to `Due`'s position, and `Due` required twice is recorded
/// once; `Nothing` and a name the unit binds only as a model alias refuse
/// `missing_declaration`/`missing-name` at the name, and a predicate refuses
/// `ill_typed`/`operator-ineligible` there, naming the kind.
#[trace("TC-1056")]
#[test]
fn a_requirement_binds_its_temporal_clause_by_position() {
    let unit = format!(
        "{}{}{}",
        temporal("Other", "true"),
        temporal("Due", "true"),
        protocol(
            "Fill",
            "requires temporal Due;\nrequires temporal Other;\nrequires temporal Due;"
        ),
    );
    let checked = requirements(&unit, "Fill").expect("the requirements check");
    let [due, other] = checked.as_slice() else {
        panic!("two requirements, got {checked:?}");
    };
    assert_eq!((due.temporal, other.temporal), (1, 0));
    assert!(due.span.start < other.span.start);
    let (_, found) = protocol_refusals(&unit);
    assert!(found.is_empty(), "{found:?}");

    for name in ["Nothing", "M"] {
        let (cause, pair, at) = only(&format!(
            "{}{}",
            temporal("Due", "true"),
            protocol("Fill", &format!("requires temporal {name};"))
        ));
        assert_eq!(pair, ("missing_declaration", Some("missing-name")));
        assert_eq!(
            cause,
            ProtocolAnchorCause::MissingRequirement {
                name: name.to_owned(),
                span: cause.span(),
            }
        );
        assert_eq!(at, name);
    }

    let (cause, pair, at) = only(&format!(
        "predicate Accept using v(): Boolean {{ true }}\n{}",
        protocol("Fill", "requires temporal Accept;")
    ));
    assert_eq!(pair, ("ill_typed", Some("operator-ineligible")));
    let ProtocolAnchorCause::RequirementKind { name, kind, .. } = &cause else {
        panic!("a requirement kind refusal, got {cause:?}");
    };
    assert_eq!((name.as_str(), *kind), ("Accept", "predicate"));
    assert_eq!(at, "Accept");
}

/// FR-218-AC-16, TC-1056 row 23a (FR-337-AC-10): `requires temporal Due;`
/// over a clause with an `on each` activation and a capture binds the
/// requirement to `Due`'s position, and the unit checks, `Due` being
/// emitted.
#[trace("TC-1056", "FR-218-AC-16")]
#[test]
fn a_requirement_binds_a_clause_with_an_activation_and_a_capture() {
    let due = "temporal Due using t over (c: Boolean) clock \"steps\" on each (x: Boolean) \
               { capture n: Boolean = c; always holds(n) }\n";
    let unit = format!(
        "{}{}{}",
        temporal("Other", "true"),
        due,
        protocol("Fill", "requires temporal Due;"),
    );
    let checked = requirements(&unit, "Fill").expect("the requirements check");
    let [due] = checked.as_slice() else {
        panic!("one requirement, got {checked:?}");
    };
    assert_eq!(due.temporal, 1);
    // A temporal clause reads the unit's one selected temporal profile, so
    // the unit selects `t` alone: the fixture header's timed profile `e` is
    // left out.
    let one_temporal = HEADER.replace("profile e = \"quire.temporal.timed/v1\";\n", "");
    assert_ne!(one_temporal, HEADER, "the header selects the timed profile");
    let (_, assembled) = assemble_under(&one_temporal, &unit);
    let refusals = assembled
        .check(
            quire_semantic_value::checking::CheckMode::Linked,
            CheckingLimits::default(),
        )
        .err()
        .unwrap_or_default();
    assert!(
        refusals
            .iter()
            .all(|refusal| !matches!(refusal.cause, CheckCause::Temporal(_))),
        "{refusals:?}"
    );
}

/// FR-383, FR-218-AC-25: the one-list check of a unit whose protocol is
/// refused leads with the protocol's own refusals and adds nothing a later
/// stage derives from the refusal (the clause naming the refused protocol is
/// not lowered into an `unsupported_feature`).
#[trace("TC-1065", "FR-218-AC-25")]
#[test]
fn a_refused_protocol_is_followed_by_no_later_stage_consequence() {
    let unit = format!(
        "{}{}",
        temporal("Due", "protocol Fill"),
        protocol("Fill", "requires temporal Due;")
    );
    let (_, assembled) = assemble(&unit);
    let refusals = assembled
        .check(
            quire_semantic_value::checking::CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect_err("the cycle refuses the unit");
    assert!(
        refusals
            .iter()
            .all(|refusal| matches!(refusal.cause, CheckCause::ProtocolAnchor(_))),
        "only the protocol's own refusals: {refusals:?}"
    );
}

/// FR-218-AC-25: `requires temporal Due;` where `Due` holds `protocol
/// Fill` and the requiring protocol is `Fill`, and the same through a
/// second clause that a protocol `Fill` names requires, each refuse
/// `invalid_package`/`definition-cycle` at the requirement, naming `Fill`
/// and the clause; the same requirement from a protocol `Drain` that no
/// clause names checks. No refused unit compiles.
#[trace("TC-1065", "FR-218-AC-25")]
#[test]
fn a_requirement_whose_clause_reaches_its_protocol_is_a_cycle() {
    let direct = format!(
        "{}{}",
        temporal("Due", "protocol Fill"),
        protocol("Fill", "requires temporal Due;")
    );
    // `Fill` requires `Due`, which names `Pour`, which requires `Back`, which
    // names `Fill`.
    let through = format!(
        "{}{}{}{}",
        temporal("Due", "protocol Pour"),
        temporal("Back", "protocol Fill"),
        protocol("Fill", "requires temporal Due;"),
        protocol("Pour", "requires temporal Back;"),
    );
    for (unit, temporal) in [(&direct, "Due"), (&through, "Due")] {
        let (_, found) = protocol_refusals(unit);
        let (cause, refusal) = found
            .iter()
            .find(|(cause, _)| {
                matches!(cause, ProtocolAnchorCause::RequirementCycle { protocol, .. }
                    if protocol == "Fill")
            })
            .unwrap_or_else(|| panic!("{unit}: a cycle at Fill, got {found:?}"));
        assert_eq!(
            (refusal.cause.code().as_str(), refusal.cause.cause()),
            ("invalid_package", Some("definition-cycle"))
        );
        let ProtocolAnchorCause::RequirementCycle {
            temporal: found, ..
        } = cause
        else {
            unreachable!("matched above");
        };
        assert_eq!(found, temporal);
    }

    let drain = format!(
        "{}{}{}",
        temporal("Due", "protocol Fill"),
        protocol("Fill", ""),
        protocol("Drain", "requires temporal Due;"),
    );
    assert_eq!(
        requirements(&drain, "Drain").map(|found| found.len()),
        Ok(1)
    );
    let (_, found) = protocol_refusals(&drain);
    assert!(found.is_empty(), "{found:?}");
}

/// FR-218-AC-16: a temporal requirement binds to the clause's declaration
/// position and no longer refuses when that clause is emitted.
#[trace("FR-218-AC-16")]
#[test]
fn an_emitted_requirement_keeps_its_temporal_clause_position() {
    let unit = format!(
        "profile pp = \"quire.protocol.finite-global/v1\";\n{}{}",
        temporal("Due", "true"),
        protocol("Fill", "requires temporal Due;")
    );
    let checked = requirements(&unit, "Fill").expect("the emitted requirement resolves");
    assert_eq!(checked.len(), 1);
    assert_eq!(checked[0].temporal, 0);
    let (_, refusals) = protocol_refusals(&unit);
    assert!(refusals.is_empty(), "{refusals:?}");
}

/// QSL-1679 (the spine successor of the composed binding report's three
/// family path): one package holds a predicate, a temporal clause that calls
/// it and captures a value, and a protocol that requires the clause and
/// calls the predicate from its events and `check`. The package checks in one
/// pass with no refusal but the protocol's pending emission, and the
/// result holds the clause with its capture and the protocol bound to that
/// clause's position, the clause and the protocol each binding a parameter
/// named `view`.
#[trace("TC-1056", "FR-218-AC-16")]
#[test]
fn one_package_checks_three_families_and_binds_the_requirement() {
    let unit = "predicate Positive using v (amount: M::Total): Boolean { amount >= 0 }\n\
        temporal Earlier using t over (view: M::Node) clock \"orders\" on origin \
        { always holds(Positive(view.total)) }\n\
        temporal Due using t over (view: M::Node) clock \"orders\" \
        on each (started: M::Node) when (Positive(started.total)) \
        { capture saved: M::Total = started.total; \
        always holds(Positive(view.total) and saved >= 0) }\n\
        protocol Flow using p over (view: M::Node) on origin {\n\
        role Service on M::Node;\n\
        requires temporal Due;\n\
        run sequence Main {\n\
        event Happened by Service as (happened: M::Receipt) { Positive(happened.total) };\n\
        check HealthyNow using v { Positive(1) };\n\
        }\n\
        finish Closed as (closed: M::Receipt) { Positive(closed.total) };\n\
        }\n";
    let one_temporal = HEADER.replace("profile e = \"quire.temporal.timed/v1\";\n", "");
    let (_, assembled) = assemble_under(&one_temporal, unit);
    let refusals = assembled
        .check(
            quire_semantic_value::checking::CheckMode::Linked,
            CheckingLimits::default(),
        )
        .err()
        .unwrap_or_default();
    assert!(
        refusals
            .iter()
            .all(|refusal| matches!(&refusal.cause, CheckCause::ProtocolReference(_))),
        "{refusals:?}"
    );
    let (_, assembled) = assemble_under(&one_temporal, unit);
    let graph = assembled
        .check_with_protocol_results(
            quire_semantic_value::checking::CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect("every declaration of the three families checks");
    let [earlier, clause] = graph.temporal_clauses() else {
        panic!("two temporal clauses");
    };
    assert_eq!(earlier.clause().name(), "Earlier");
    assert_eq!(clause.clause().name(), "Due");
    assert_eq!(clause.captures.len(), 1);
    let flow = graph.protocol("Flow").expect("the protocol checks");
    let [due] = flow.requirements.as_slice() else {
        panic!("one requirement, got {:?}", flow.requirements);
    };
    assert_eq!(due.temporal, 1, "Due follows Earlier");
}

/// QSL-1679 (the spine successor of the composed report's `exports()`
/// occurrences with model-type targets): a model type written in a clause
/// binder, trigger and capture resolves to that model's own type. The
/// scalar `M::Total` is the integer interval `0..1000` and `M::Node` is a
/// reference to the model object, and a binder written `M::Tally` resolves
/// to the scalar `0..3`, so a wrong target changes a value asserted here.
#[trace("TC-1056", "FR-218-AC-37")]
#[test]
fn a_model_type_in_a_clause_binder_resolves_to_the_models_own_type() {
    use quire_exact::{Integer, IntegerInterval, ValueType};

    let interval = |lower: i64, upper: i64| {
        ValueType::Int(
            IntegerInterval::new(Integer::from(lower), Integer::from(upper))
                .expect("a non-empty interval"),
        )
    };
    let unit = "temporal Due using t over (view: M::Node) clock \"orders\" \
        on each (started: M::Node) \
        { capture saved: M::Total = started.total; \
        capture level: M::Tally = started.tally; \
        always holds(saved >= 0) }\n";
    let one_temporal = HEADER.replace("profile e = \"quire.temporal.timed/v1\";\n", "");
    let (_, assembled) = assemble_under(&one_temporal, unit);
    let graph = assembled
        .check(
            quire_semantic_value::checking::CheckMode::Linked,
            CheckingLimits::default(),
        )
        .unwrap_or_else(|refusals| panic!("the clause checks: {refusals:?}"));
    let [node] = graph.temporal_clauses() else {
        panic!("one temporal clause");
    };
    let clause = node.clause();
    let captures: Vec<_> = clause.captures().collect();
    assert_eq!(
        captures,
        vec![("saved", &interval(0, 1000)), ("level", &interval(0, 3))]
    );
    let (_, over) = clause.over();
    let (_, trigger) = clause.trigger().expect("an `on each` clause has a trigger");
    let node = ValueType::Reference(
        graph
            .scope()
            .object_type_named(None, "M::Node")
            .expect("the model's object type Node"),
    );
    assert_eq!(over, &node);
    assert_eq!(trigger, &node);
    assert_ne!(over, &interval(0, 1000));
}

/// QSL-1679 (the cross-unit successor of the composed report's three family
/// binding): the predicate, the temporal clause that calls it and the
/// protocol that requires that clause and calls the predicate are written in
/// three units of one aggregate package. The package checks whatever the
/// request order, the protocol's requirement binds the clause declared in
/// the other unit, and without the clause's unit the same requirement is
/// refused.
#[trace("TC-1056", "FR-218-AC-38")]
#[test]
fn three_units_bind_a_predicate_a_clause_and_a_protocol_across_units() {
    use crate::check::assemble::AssemblyLimits;
    use quire_exact::Cancel;

    let header = |profiles: &str| {
        format!(
            "language \"ix:native\" edition \"1-draft\";\n{profiles}model M = \"example/protocol-obligations\";\n"
        )
    };
    let predicate = format!(
        "{}predicate Positive using v (amount: M::Total): Boolean {{ amount >= 0 }}\n",
        header("profile v = \"quire.value.complete/v1\";\n")
    );
    let earlier = format!(
        "{}temporal Earlier using t over (view: M::Node) clock \"orders\" on origin \
         {{ always holds(Positive(view.total)) }}\n",
        header("profile t = \"quire.temporal.infinite-trace/v1\";\n")
    );
    let clause = format!(
        "{}temporal Due using t over (view: M::Node) clock \"orders\" \
         on each (started: M::Node) when (Positive(started.total)) \
         {{ capture saved: M::Total = started.total; \
         always holds(Positive(view.total) and saved >= 0) }}\n",
        header("profile t = \"quire.temporal.infinite-trace/v1\";\n")
    );
    let protocol = format!(
        "{}protocol Flow using p over (view: M::Node) on origin {{\n\
         role Service on M::Node;\n\
         requires temporal Due;\n\
         run sequence Main {{\n\
         event Happened by Service as (happened: M::Receipt) {{ Positive(happened.total) }};\n\
         }}\n\
         finish Closed as (closed: M::Receipt) {{ Positive(closed.total) }};\n\
         }}\n",
        header("profile p = \"quire.protocol.finite-global/v1\";\n")
    );
    let parsed = |identity: &str, text: &str| {
        let parsed = qsl_cst::parse(
            SourceIdentity::new("repo", identity),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
        (
            parsed.source().reference().clone(),
            build_unit(&parsed).expect("S2 builds the unit"),
        )
    };
    let assemble = |texts: &[(&str, &str)]| {
        PackageDeclarations::assemble_units(
            "example/protocol-obligations".to_owned(),
            texts
                .iter()
                .map(|(identity, text)| parsed(identity, text))
                .collect(),
            vec![obligation_model()],
            Vec::new(),
            AssemblyLimits::default(),
            &Cancel::new(),
        )
        .unwrap_or_else(|refusal| panic!("the units assemble: {refusal:?}"))
    };

    // Units sort by identity: `Earlier`, in `a1`, precedes `Due`, in `b`.
    for order in [[0, 1, 2, 3], [3, 2, 1, 0], [2, 3, 0, 1]] {
        let all = [
            ("a", &predicate),
            ("a1", &earlier),
            ("b", &clause),
            ("c", &protocol),
        ];
        let texts: Vec<_> = order
            .iter()
            .map(|&i| (all[i].0, all[i].1.as_str()))
            .collect();
        let graph = assemble(&texts)
            .check_with_protocol_results(
                quire_semantic_value::checking::CheckMode::Linked,
                CheckingLimits::default(),
            )
            .unwrap_or_else(|refusals| panic!("{order:?}: the package checks: {refusals:?}"));
        let [first, node] = graph.temporal_clauses() else {
            panic!("{order:?}: two temporal clauses");
        };
        assert_eq!(first.clause().name(), "Earlier");
        assert_eq!(node.clause().name(), "Due");
        assert_eq!(node.clause().captures().len(), 1);
        let flow = graph.protocol("Flow").expect("the protocol checks");
        let [due] = flow.requirements.as_slice() else {
            panic!("{order:?}: one requirement, got {:?}", flow.requirements);
        };
        assert_eq!(due.temporal, 1, "{order:?}: Due follows Earlier");
    }

    let refusals = assemble(&[("a", &predicate), ("a1", &earlier), ("c", &protocol)])
        .check(
            quire_semantic_value::checking::CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect_err("the requirement names a clause no unit declares");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, got {refusals:?}");
    };
    assert_eq!(
        (refusal.cause.code().as_str(), refusal.cause.cause()),
        ("missing_declaration", Some("missing-name"))
    );
    let CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
        panic!("a protocol anchor refusal, got {refusal:?}");
    };
    assert!(
        matches!(cause.as_ref(), ProtocolAnchorCause::MissingRequirement { name, .. }
            if name == "Due"),
        "{cause:?}"
    );
}
