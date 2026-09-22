// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-114/115: lexical and structural stages through their public Rust boundary.
//! Unresolved authored model types remain syntax in model-independent scope cases.
use crate::support::native_rule_model;

use ix_trace_rs::trace;
use quire_spec_language::linking::composed::binding_work::{
    Dimension, Limits as BindingLimits, Work,
};
use quire_spec_language::linking::composed::models::{bind_models, ModelInput};
use quire_spec_language::linking::composed::scopes::{
    self, Anchor, BinderKind, BinderType, DeclarationScope, ScopeDisposition, ScopeIssue,
    ScopeReport, StructuralKind, SymbolKind,
};
use quire_spec_language::linking::composed::{
    admit_namespace, DeclarationId, ExpectedSource, SourceInventory, SyntaxNamespace, WorkLimits,
};
use quire_spec_language::{Limits, Source, SourceIdentity};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile S = \"test:unresolved-definition\" version \"1\" digest \"unresolved\";\n";
fn program(body: &str) -> String {
    format!("{HEADER}{body}")
}
fn with_namespace(texts: &[String], test: impl FnOnce(&SyntaxNamespace)) {
    let sources: Vec<_> = texts
        .iter()
        .enumerate()
        .map(|(index, text)| {
            Source::read(
                SourceIdentity {
                    identity: format!("scope:{index}"),
                    revision: "authored".into(),
                },
                format!("scope-{index}.native"),
                text.as_bytes(),
                Limits::default().source_bytes,
            )
            .unwrap()
        })
        .collect();
    let inventory = SourceInventory {
        language: "ix:native".into(),
        edition: "1-draft".into(),
        units: sources
            .iter()
            .map(|source| ExpectedSource {
                authority: source.identity().identity.clone(),
                identity: source.identity().clone(),
                digest: source.digest(),
            })
            .collect(),
    };
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(report.issues().is_empty(), "{:?}", report.issues());
    assert!(report.exhaustion().is_none());
    let namespace = report.namespace().unwrap();
    assert!(namespace.dependencies_complete());
    test(namespace);
}
fn id(namespace: &SyntaxNamespace, name: &str) -> DeclarationId {
    let [id] = namespace.lookup(name) else {
        panic!("one {name}")
    };
    *id
}
fn scope<'a>(
    namespace: &SyntaxNamespace,
    report: &'a ScopeReport,
    name: &str,
) -> &'a DeclarationScope {
    report.declaration(id(namespace, name)).unwrap()
}
fn resolved(namespace: &SyntaxNamespace, report: &ScopeReport, name: &str) {
    let declaration = scope(namespace, report, name);
    assert_eq!(
        declaration.disposition(),
        ScopeDisposition::Resolved,
        "{name}: {:?}",
        declaration.issues
    );
}
fn inspect(body: &str, test: impl FnOnce(&SyntaxNamespace, &ScopeReport)) {
    with_namespace(&[program(body)], |namespace| {
        let models = bind_models(namespace, &[], &mut Work::new(BindingLimits::default()));
        let report = scopes::resolve(namespace, &models, &mut Work::new(BindingLimits::default()));
        assert!(report.exhaustion.is_none(), "{:?}", report.exhaustion);
        test(namespace, &report);
    });
}
fn unavailable(scope: &DeclarationScope, name: &str) {
    assert_eq!(scope.disposition(), ScopeDisposition::Refused);
    assert!(
        scope.issues.iter().any(
            |issue| matches!(issue, ScopeIssue::OutOfScope { name: found, .. } if found == name)
        ),
        "{:?}",
        scope.issues
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-2", "FR-036-AC-4")]
fn lexical_bodies_keep_parameters_let_and_query_domain_scopes() {
    inspect("predicate Good using S (input: Boolean): Boolean { let saved = input in forall(item in input: saved and item) }\n\
        predicate Initializer using S (): Boolean { let later = later in later }\n\
        predicate QueryDomain using S (): Boolean { forall(element in element: element) }\n\
        predicate Shadow using S (valueIn: Boolean): Boolean { let valueIn = true in valueIn }\n\
        predicate Alias using S (S: Boolean): Boolean { S }\n\
        predicate Ambient using S (): Boolean { pre(self) = result }", |namespace, report| {
        resolved(namespace, report, "Good");
        let good = scope(namespace, report, "Good");
        assert_eq!(good.binders.iter().map(|binder| binder.kind).collect::<Vec<_>>(), [BinderKind::Parameter, BinderKind::Let, BinderKind::Query]);
        for occurrence in &good.values {
            assert_eq!(occurrence.unit, good.unit);
            let binder = &good.binders[occurrence.target.index()];
            assert_eq!(namespace.unit(good.unit).unwrap().source().slice(occurrence.span), binder.name.as_deref());
        }
        unavailable(scope(namespace, report, "Initializer"), "later");
        unavailable(scope(namespace, report, "QueryDomain"), "element");
        assert!(scope(namespace, report, "Shadow").issues.iter().any(|issue| matches!(issue, ScopeIssue::DuplicateBinder { .. })));
        assert!(scope(namespace, report, "Alias").issues.iter().any(|issue| matches!(issue, ScopeIssue::ReservedBinder { .. })));
        assert!(scope(namespace, report, "Ambient").issues.iter().any(|issue| matches!(issue, ScopeIssue::AmbientUnavailable { .. })));
    });
}

const TEMPORAL: &str = "temporal Timing using S over (input: Boolean) clock \"clock\" on each (trigger: Boolean) when (trigger and input) {\n\
    capture firstValue: Boolean = trigger;\n\
    capture secondValue: Boolean = firstValue and input;\n\
    eventually [0,2] holds(secondValue and input)\n}";

#[test]
#[trace("TC-114", "FR-036-AC-2", "FR-036-AC-4")]
fn activation_guard_sequential_captures_and_formula_have_distinct_environments() {
    inspect(TEMPORAL, |namespace, report| {
        resolved(namespace, report, "Timing");
        let timing = scope(namespace, report, "Timing");
        let capture = timing
            .binders
            .iter()
            .position(|binder| binder.name.as_deref() == Some("secondValue"))
            .unwrap();
        assert_eq!(timing.binders[capture].anchor, Anchor::Activation);
        let occurrences: Vec<_> = timing
            .values
            .iter()
            .filter(|occurrence| occurrence.target.index() == capture)
            .collect();
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0].evaluation_anchor, Anchor::TemporalInstant);
    });
    for (from, to, name) in [
        (
            "holds(secondValue and input)",
            "holds(trigger and input)",
            "trigger",
        ),
        (
            "when (trigger and input)",
            "when (firstValue and input)",
            "firstValue",
        ),
        (
            "firstValue: Boolean = trigger",
            "firstValue: Boolean = secondValue",
            "secondValue",
        ),
    ] {
        assert!(TEMPORAL.contains(from));
        inspect(&TEMPORAL.replacen(from, to, 1), |namespace, report| {
            unavailable(scope(namespace, report, "Timing"), name)
        });
    }
}

fn protocol(run: &str, final_value: &str) -> String {
    format!(
        "protocol Flow using S over (view: Boolean) on origin {{\n\
        role R on M::Service;\n\
        channel C from R to R carries M::Payload ordering unordered delivery [0,1];\n\
        run sequence Main {{ {run} }}\n\
        finish Closed as (done: Boolean) {{ {final_value} }};\n}}"
    )
}

#[test]
#[trace("TC-114", "FR-036-AC-2", "FR-036-AC-4")]
fn sequence_and_all_branch_join_export_only_definite_event_records() {
    let run = "parallel Both { branch left event Left by R as (leftValue: Boolean) { leftValue };\n\
        branch right event Right by R as (rightValue: Boolean) { rightValue }; } join all [left,right];\n\
        check Joined using S { leftValue and rightValue };";
    inspect(
        &protocol(run, "done and leftValue and rightValue"),
        |namespace, report| {
            resolved(namespace, report, "Flow");
            let flow = scope(namespace, report, "Flow");
            assert!(flow
                .symbols
                .iter()
                .any(|symbol| symbol.kind == SymbolKind::Branch && symbol.name.value == "left"));
            assert_eq!(
                flow.references
                    .iter()
                    .filter(|reference| reference.required == StructuralKind::Branch)
                    .count(),
                2
            );
            let left = flow
                .binders
                .iter()
                .find(|binder| binder.name.as_deref() == Some("leftValue"))
                .unwrap();
            assert!(matches!(left.anchor, Anchor::Control(_)));
        },
    );
    let sibling = run.replace("{ rightValue };", "{ leftValue };");
    inspect(&protocol(&sibling, "done"), |namespace, report| {
        unavailable(scope(namespace, report, "Flow"), "leftValue")
    });
    let invalid_join = run.replace("join all [left,right]", "join all [left,left]");
    inspect(&protocol(&invalid_join, "done"), |namespace, report| {
        assert!(scope(namespace, report, "Flow")
            .issues
            .iter()
            .any(|issue| matches!(issue, ScopeIssue::InvalidJoin { .. })))
    });
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn choice_and_iteration_records_do_not_escape_and_await_match_is_then_only() {
    let choice = "choice Decision by R visible (view) { case yes when { view } event Yes by R as (yesValue: Boolean) { yesValue };\n\
        case no when { not view } event No by R as (noValue: Boolean) { noValue }; }\n\
        check Escape using S { yesValue };";
    inspect(&protocol(choice, "done"), |namespace, report| {
        unavailable(scope(namespace, report, "Flow"), "yesValue")
    });
    let repeat = "repeat Loop by R visible (view) max 2 while { view } event Again by R as (loopValue: Boolean) { loopValue };\n\
        exhausted sequence Limit {} check Escape using S { loopValue };";
    inspect(&protocol(repeat, "done"), |namespace, report| {
        unavailable(scope(namespace, report, "Flow"), "loopValue")
    });
    let wait = "event Start by R as (started: Boolean) { started };\n\
        await Wait after Start using S clock \"clock\" within [0,2]\n\
        match event Reply by R as (reply: Boolean) { reply };\n\
        then check Received using S { reply } ; timeout check TimedOut using S { view };";
    inspect(&protocol(wait, "done"), |namespace, report| {
        resolved(namespace, report, "Flow")
    });
    inspect(
        &protocol(
            &wait.replace("TimedOut using S { view }", "TimedOut using S { reply }"),
            "done",
        ),
        |namespace, report| unavailable(scope(namespace, report, "Flow"), "reply"),
    );
    inspect(&protocol(wait, "done and reply"), |namespace, report| {
        unavailable(scope(namespace, report, "Flow"), "reply")
    });
}

const COMPENSATION: &str = "protocol RecoveryFlow using S over (view: Boolean) on origin {\n\
    role R on M::Service;\n\
    compensate Undo for Main::Applied as (forward: Boolean) by R on M::Object::change using S clock \"clock\" {\n\
      capture saved: Boolean = forward;\n\
      activate first (trigger: Boolean) when { trigger and saved } { capture frozen: Boolean = trigger; }\n\
      within [0,2]; attempts 2 of M::Attempt;\n\
      retry (earlier: Boolean, later: Boolean) { earlier and later and saved and frozen };\n\
      commit Main::Committed; recover (recovered: Boolean) { recovered and saved and frozen };\n\
    }\n\
    run sequence Main {\n\
      attempt Tried by R on M::Object::change contracts [] as (attempted: Boolean) { attempted };\n\
      effect Applied of Tried as (applied: Boolean) { applied };\n\
      event Recovered by R for Undo as (recoveryEvent: Boolean) { recoveryEvent };\n\
      commit Committed by R as (committed: Boolean) { committed };\n\
    }\n\
    finish Closed as (done: Boolean) { done };\n}";

#[test]
#[trace("TC-114", "FR-036-AC-2", "FR-036-AC-4")]
fn compensation_registration_activation_retry_and_recovery_keep_their_anchors() {
    inspect(COMPENSATION, |namespace, report| {
        resolved(namespace, report, "RecoveryFlow");
        let scope = scope(namespace, report, "RecoveryFlow");
        for (name, anchor) in [
            ("saved", Anchor::Registration(0)),
            ("frozen", Anchor::CompensationActivation(0)),
            ("earlier", Anchor::Retry(0)),
            ("recovered", Anchor::Recovery(0)),
        ] {
            assert_eq!(
                scope
                    .binders
                    .iter()
                    .find(|binder| binder.name.as_deref() == Some(name))
                    .unwrap()
                    .anchor,
                anchor
            );
        }
        for kind in [
            StructuralKind::Effect,
            StructuralKind::Attempt,
            StructuralKind::Commit,
            StructuralKind::Compensation,
        ] {
            assert!(scope
                .references
                .iter()
                .any(|reference| reference.required == kind && reference.target.is_some()));
        }
    });
    for (from, to, missing) in [
        ("trigger and saved", "forward and saved", "forward"),
        (
            "earlier and later and saved and frozen",
            "trigger and later",
            "trigger",
        ),
        (
            "recovered and saved and frozen",
            "recovered and earlier",
            "earlier",
        ),
        (
            "saved: Boolean = forward",
            "saved: Boolean = frozen",
            "frozen",
        ),
    ] {
        assert!(COMPENSATION.contains(from));
        inspect(&COMPENSATION.replacen(from, to, 1), |namespace, report| {
            unavailable(scope(namespace, report, "RecoveryFlow"), missing)
        });
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn fifo_is_isolated_and_structural_targets_keep_kind_path_and_precedence() {
    let run = "send Sent via C as (sent: Boolean) { sent }; receive Got via C of Main::Sent as (received: Boolean) { received };";
    let text = protocol(run, "done").replace(
        "ordering unordered",
        "ordering fifo by (message: Boolean) { message }",
    );
    inspect(&text, |namespace, report| {
        resolved(namespace, report, "Flow");
        let scope = scope(namespace, report, "Flow");
        let reference = scope
            .references
            .iter()
            .find(|reference| reference.required == StructuralKind::Send)
            .unwrap();
        assert_eq!(
            reference
                .path
                .iter()
                .map(|part| part.value.as_str())
                .collect::<Vec<_>>(),
            ["Main", "Sent"]
        );
        assert_eq!(
            scope.symbols[reference.target.unwrap().index()].kind,
            SymbolKind::Send
        );
        assert_eq!(
            namespace
                .unit(scope.unit)
                .unwrap()
                .source()
                .slice(reference.span),
            Some("Main::Sent")
        );
    });
    inspect(
        &text.replace("{ message }", "{ view }"),
        |namespace, report| unavailable(scope(namespace, report, "Flow"), "view"),
    );
    for (from, to) in [("of Main::Sent", "of Main"), ("via C as", "via R as")] {
        assert!(text.contains(from));
        inspect(&text.replacen(from, to, 1), |namespace, report| {
            assert!(scope(namespace, report, "Flow")
                .issues
                .iter()
                .any(|issue| matches!(issue, ScopeIssue::WrongTargetKind { .. })))
        });
    }
    let future = "receive Got via C of Sent as (received: Boolean) { received }; send Sent via C as (sent: Boolean) { sent };";
    inspect(&protocol(future, "done"), |namespace, report| {
        assert!(scope(namespace, report, "Flow")
            .issues
            .iter()
            .any(|issue| matches!(issue, ScopeIssue::UnavailableTarget { .. })))
    });
}

#[test]
#[trace("TC-114", "FR-036-AC-4", "FR-036-AC-7")]
fn equal_binder_names_in_distinct_units_keep_owners_and_exact_scope_limits_retry() {
    with_namespace(
        &[
            program("predicate First using S (input: Boolean): Boolean { input }"),
            program("predicate Second using S (input: Boolean): Boolean { input }"),
        ],
        |namespace| {
            let models = bind_models(namespace, &[], &mut Work::new(BindingLimits::default()));
            let report =
                scopes::resolve(namespace, &models, &mut Work::new(BindingLimits::default()));
            let first = scope(namespace, &report, "First");
            let second = scope(namespace, &report, "Second");
            assert_ne!(first.unit, second.unit);
            assert_ne!(first.declaration, second.declaration);
            assert_eq!(first.values[0].unit, first.unit);
            assert_eq!(second.values[0].unit, second.unit);
            assert_eq!(first.binders[0].name, second.binders[0].name);
        },
    );
    with_namespace(
        &[program(
            "predicate Only using S (input: Boolean): Boolean { input }",
        )],
        |namespace| {
            let models = bind_models(namespace, &[], &mut Work::new(BindingLimits::default()));
            // One declaration + binder + frame + occurrence; declaration, binder-name,
            // profile-alias, value-node and name inspections; one lexical frame edge.
            let exact = BindingLimits {
                bindings: 4,
                references: 5,
                edges: 1,
                ..BindingLimits::default()
            };
            for (limits, dimension) in [
                (
                    BindingLimits {
                        bindings: 0,
                        ..exact
                    },
                    Dimension::Bindings,
                ),
                (
                    BindingLimits {
                        bindings: 3,
                        ..exact
                    },
                    Dimension::Bindings,
                ),
                (
                    BindingLimits {
                        references: 4,
                        ..exact
                    },
                    Dimension::References,
                ),
                (BindingLimits { edges: 0, ..exact }, Dimension::Edges),
            ] {
                let mut work = Work::new(limits);
                let failed = scopes::resolve(namespace, &models, &mut work);
                assert_eq!(failed.exhaustion.unwrap().dimension, dimension);
                assert_eq!(
                    failed.disposition(id(namespace, "Only")),
                    ScopeDisposition::Unfinished
                );
                let prior_usage = work.usage();
                let retried = scopes::resolve(namespace, &models, &mut Work::new(exact));
                resolved(namespace, &retried, "Only");
                assert_eq!(work.usage(), prior_usage);
                assert_eq!(
                    failed.disposition(id(namespace, "Only")),
                    ScopeDisposition::Unfinished
                );
            }
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-4")]
fn actual_model_operation_inputs_result_and_pre_self_seed_the_state_scope() {
    let baseline = native_rule_model::parts().model();
    assert!(baseline.roles().operations[0].parameters.is_empty());
    assert_eq!(
        baseline.roles().operations[0].name,
        native_rule_model::symbol("step")
    );
    let mut document: serde_json::Value = serde_json::from_str(native_rule_model::FIXTURE).unwrap();
    document["values"].as_array_mut().unwrap().push(serde_json::json!({"name":"delta", "kind":"input", "type":{"kind":"scalar", "name":"Version"}}));
    document["operations"][0]["parameters"] = serde_json::json!(["delta"]);
    let model = native_rule_model::from_text(
        &serde_json::to_string(&document).unwrap(),
        "scope-model.json",
        "draft:1",
    )
    .unwrap()
    .model();
    let source = format!(
        "{HEADER}model M = \"{}\" version \"{}\" digest \"{}\";\n\
        post Changed using S on M::Node::step {{ self.n = pre(self.n) + delta and result }}",
        model.environment().owner().package().as_str(),
        model.environment().owner().revision().get(),
        model.digest()
    );
    with_namespace(&[source], |namespace| {
        let inputs = [ModelInput::Native(&model)];
        let models = bind_models(namespace, &inputs, &mut Work::new(BindingLimits::default()));
        let report = scopes::resolve(namespace, &models, &mut Work::new(BindingLimits::default()));
        resolved(namespace, &report, "Changed");
        let scope = scope(namespace, &report, "Changed");
        let delta = scope
            .binders
            .iter()
            .find(|binder| binder.name.as_deref() == Some("delta"))
            .unwrap();
        assert_eq!(delta.kind, BinderKind::InvocationParameter);
        assert_eq!(delta.anchor, Anchor::InvocationInput);
        assert!(matches!(delta.ty, BinderType::ModelValue(_)));
        assert!(scope
            .values
            .iter()
            .any(
                |occurrence| occurrence.evaluation_anchor == Anchor::InvocationPre
                    && scope.binders[occurrence.target.index()].kind == BinderKind::SelfValue
            ));
        assert!(scope
            .values
            .iter()
            .any(|occurrence| scope.binders[occurrence.target.index()].kind
                == BinderKind::ResultValue));
    });
}

#[test]
#[trace("TC-114", "FR-036-AC-2", "FR-036-AC-4")]
fn an_undeclared_value_retains_its_missing_name_and_original_expression() {
    inspect(
        "predicate Missing using S (): Boolean { neverDeclared }\n\
         predicate Independent using S (input: Boolean): Boolean { input }",
        |namespace, report| {
            let missing = scope(namespace, report, "Missing");
            assert_eq!(missing.disposition(), ScopeDisposition::Refused);
            let [ScopeIssue::MissingValue {
                expression,
                span,
                name,
            }] = missing.issues.as_slice()
            else {
                panic!("one missing value: {:?}", missing.issues)
            };
            assert_eq!(name, "neverDeclared");
            let unit = namespace.unit(missing.unit).unwrap();
            assert_eq!(unit.expression(*expression).unwrap().span, *span);
            assert_eq!(unit.source().slice(*span), Some("neverDeclared"));
            assert!(missing.values.is_empty());
            resolved(namespace, report, "Independent");
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-2")]
fn an_unavailable_model_operation_does_not_invent_missing_input_names() {
    let model = native_rule_model::parts().model();
    let source = format!(
        "{HEADER}model M = \"{}\" version \"{}\" digest \"{}\";\n\
         pre Missing using S on M::Node::absent {{ invocationInput }}\n\
         post Available using S on M::Node::step {{ result }}",
        model.environment().owner().package().as_str(),
        model.environment().owner().revision().get(),
        model.digest()
    );
    with_namespace(&[source], |namespace| {
        let inputs = [ModelInput::Native(&model)];
        let models = bind_models(namespace, &inputs, &mut Work::new(BindingLimits::default()));
        let report = scopes::resolve(namespace, &models, &mut Work::new(BindingLimits::default()));
        assert!(report.exhaustion.is_none());
        let missing = scope(namespace, &report, "Missing");
        assert_eq!(missing.disposition(), ScopeDisposition::Refused);
        let [ScopeIssue::ModelOperationUnavailable { span }] = missing.issues.as_slice() else {
            panic!("one unavailable operation: {:?}", missing.issues)
        };
        assert_eq!(
            namespace.unit(missing.unit).unwrap().source().slice(*span),
            Some("absent")
        );
        assert!(missing.values.is_empty());
        resolved(namespace, &report, "Available");
    });
}

#[test]
#[trace("TC-114", "FR-036-AC-2", "FR-036-AC-4")]
fn structural_duplicates_are_local_and_missing_paths_preserve_the_authored_target() {
    let nested = "sequence Left { send Shared via C as (leftValue: Boolean) { leftValue }; }\n\
        sequence Right { send Shared via C as (rightValue: Boolean) { rightValue };\n\
        receive Got via C of Main::Left::Shared as (received: Boolean) { received }; }";
    inspect(
        &protocol(nested, "done and leftValue and rightValue and received"),
        |namespace, report| {
            resolved(namespace, report, "Flow");
            let flow = scope(namespace, report, "Flow");
            let shared: Vec<_> = flow
                .symbols
                .iter()
                .filter(|symbol| symbol.name.value == "Shared")
                .collect();
            assert_eq!(shared.len(), 2);
            assert_ne!(shared[0].parent, shared[1].parent);
            let receive = flow
                .references
                .iter()
                .find(|reference| reference.required == StructuralKind::Send)
                .unwrap();
            let selected = &flow.symbols[receive.target.unwrap().index()];
            assert_eq!(
                flow.symbols[selected.parent.unwrap().index()].name.value,
                "Left"
            );
        },
    );
    inspect(
        &protocol(
            &nested.replace("Main::Left::Shared", "Main::Absent::Shared"),
            "done",
        ),
        |namespace, report| {
            let flow = scope(namespace, report, "Flow");
            assert_eq!(flow.disposition(), ScopeDisposition::Refused);
            let [ScopeIssue::MissingTarget { reference }] = flow.issues.as_slice() else {
                panic!("one missing structural target: {:?}", flow.issues)
            };
            let reference = &flow.references[*reference];
            assert!(reference.target.is_none());
            assert_eq!(
                namespace
                    .unit(flow.unit)
                    .unwrap()
                    .source()
                    .slice(reference.span),
                Some("Main::Absent::Shared")
            );
        },
    );
    let duplicate = "send Shared via C as (firstValue: Boolean) { firstValue };\n\
        send Shared via C as (secondValue: Boolean) { secondValue };\n\
        receive Got via C of Shared as (received: Boolean) { received };";
    inspect(&protocol(duplicate, "done"), |namespace, report| {
        let flow = scope(namespace, report, "Flow");
        assert_eq!(flow.disposition(), ScopeDisposition::Refused);
        assert_eq!(flow.issues.len(), 2);
        let (symbol, previous) = flow
            .issues
            .iter()
            .find_map(|issue| match issue {
                ScopeIssue::DuplicateSymbol { symbol, previous } => Some((*symbol, *previous)),
                _ => None,
            })
            .unwrap();
        let (symbol, previous) = (
            &flow.symbols[symbol.index()],
            &flow.symbols[previous.index()],
        );
        assert_eq!(symbol.name.value, "Shared");
        assert_eq!(symbol.name.value, previous.name.value);
        assert_eq!(symbol.parent, previous.parent);
        assert_ne!(symbol.name.span, previous.name.span);
        let reference = flow
            .issues
            .iter()
            .find_map(|issue| match issue {
                ScopeIssue::AmbiguousTarget { reference } => Some(&flow.references[*reference]),
                _ => None,
            })
            .unwrap();
        assert!(reference.target.is_none());
        assert_eq!(
            namespace
                .unit(flow.unit)
                .unwrap()
                .source()
                .slice(reference.span),
            Some("Shared")
        );
    });
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn await_refuses_send_and_attempt_matches_at_the_original_control() {
    for matched in [
        "send Matched via C as (matchedValue: Boolean) { matchedValue };",
        "attempt Matched by R on M::Service::act contracts [] as (matchedValue: Boolean) { matchedValue };",
    ] {
        let run = format!(
            "event Start by R as (started: Boolean) {{ started }};\n\
             await Wait after Start using S clock \"clock\" within [0,2]\n\
             match {matched}\n\
             then check Received using S {{ matchedValue }};\n\
             timeout check TimedOut using S {{ view }};"
        );
        inspect(&protocol(&run, "done"), |namespace, report| {
            let flow = scope(namespace, report, "Flow");
            assert_eq!(flow.disposition(), ScopeDisposition::Refused);
            let [ScopeIssue::InvalidAwaitEvent { control, span }] = flow.issues.as_slice() else {
                panic!("one invalid await event: {:?}", flow.issues)
            };
            let unit = namespace.unit(flow.unit).unwrap();
            let original = unit.control(*control).unwrap();
            assert_eq!(original.name.value, "Matched");
            assert_eq!(original.span, *span);
            assert_eq!(unit.source().slice(*span), Some(matched));
        });
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn a_receive_cannot_substitute_another_declared_channel_for_its_send() {
    let source = protocol(
        "send Sent via C as (sent: Boolean) { sent };\n\
         receive Got via C of Main::Sent as (received: Boolean) { received };",
        "done",
    ).replace("run sequence Main", "channel Other from R to R carries M::Payload ordering unordered delivery [0,1];\nrun sequence Main");
    inspect(&source, |namespace, report| {
        resolved(namespace, report, "Flow")
    });
    inspect(
        &source.replace("receive Got via C", "receive Got via Other"),
        |namespace, report| {
            let flow = scope(namespace, report, "Flow");
            assert_eq!(flow.disposition(), ScopeDisposition::Refused);
            let [ScopeIssue::IncompatibleReference { reference }] = flow.issues.as_slice() else {
                panic!("one incompatible send channel: {:?}", flow.issues)
            };
            let reference = &flow.references[*reference];
            assert_eq!(reference.required, StructuralKind::Send);
            assert_eq!(
                flow.symbols[reference.target.unwrap().index()].name.value,
                "Sent"
            );
            assert_eq!(
                namespace
                    .unit(flow.unit)
                    .unwrap()
                    .source()
                    .slice(reference.span),
                Some("Main::Sent")
            );
        },
    );
}
