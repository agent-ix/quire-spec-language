// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-362-AC-7 (TC-1026 step 4): one source holding a model `relation` and a
//! `hyper` clause yields a checked unit with both clauses in its graph and a
//! type report, the hyper clause emits as one `claim`/`hyperproperty` node,
//! and a refusal is the clause's own typed cause, `NotEmitted` only in the
//! FR-488 cases. Every case compiles real source through the spine over the
//! `test/config-version` domain package.

use super::*;
use crate::spine::{check, parse, select, ParseRequest};
use qsl_foundation::diagnostic::StageFailure;
use qsl_semantics::check::{CheckCause, TemporalCause};
use quire_exact::Cancel;

const INFINITE: &str = "quire.temporal.infinite-trace/v1";
const EVENT: &str = "quire.temporal.event-position.false-extension/v1";

const PREDICATE: &str = "predicate Positive using v(n: Integer): Boolean { n > 0 }";
const RELATION: &str = "relation Det using t over (x: Config::ConfigVersion::attemptUpdate, \
    y: Config::ConfigVersion::attemptUpdate) \
    { x.result = y.result and pre(x.self.versionNumber) = pre(y.self.versionNumber) \
    implies x.self.versionNumber = y.self.versionNumber }";
const READ_A_B: &str = "always holds(c.versionNumber @ a = c.versionNumber @ b)";

fn hyper(fairness: &str, body: &str) -> String {
    format!(
        "hyper NonInterference using t over behaviours (c: Config::ConfigVersion) \
         forall trace a of Config {fairness} forall trace b of Config \
         match {{ a.step.op = b.step.op }} {{ {body} }}"
    )
}

/// `declarations` checked and linked through the spine under the temporal
/// profile `profile`.
fn produce(
    profile: &str,
    declarations: &str,
) -> (
    String,
    Result<
        qsl_foundation::diagnostic::Staged<crate::spine::CheckedUnit>,
        crate::spine::FrontEndFailure,
    >,
) {
    let document = config_version_domain_document_with_operations(Vec::new());
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"{profile}\";\n\
         model Config = {CONFIG_VERSION_PACKAGE_IDENTITY:?};\n\
         {declarations}\n"
    );
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let identity = qsl_foundation::SourceIdentity::new("test", "tc-1026");
    let parsed = parse(
        &ParseRequest {
            source: &identity,
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("S1 and S2 read the unit")
    .into_value();
    let models = select(&parsed, &packages, limits.model, &cancel)
        .expect("I1 admits the model")
        .into_value();
    let checked = check(
        &parsed,
        "fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    );
    (text, checked)
}

/// The checked clause refusals of a unit the producer refused.
fn causes(refused: crate::spine::FrontEndFailure) -> Vec<CheckCause> {
    let StageFailure::Refused(refusal) = refused else {
        panic!("a refusal, not {refused:?}");
    };
    match *refusal {
        CompileRefusal::Check { refusals, .. } => {
            refusals.into_iter().map(|refusal| refusal.cause).collect()
        }
        other => panic!("a check refusal, not {other:?}"),
    }
}

/// FR-362-AC-7, first sentence: a unit holding a `predicate`, a `relation`
/// clause over model operations and a `hyper` clause over `behaviours`
/// yields one checked unit whose graph holds the relation and the hyper
/// clause and whose emitted package holds the relation's
/// `claim`/`step_relation` node and the hyper clause's
/// `claim`/`hyperproperty` node, and which has a type report.
#[trace("FR-362-AC-7", "TC-1026")]
#[test]
fn a_relation_and_a_hyper_clause_yield_one_checked_unit_with_both_nodes() {
    let declarations = format!("{PREDICATE}\n{RELATION}\n{}", hyper("", READ_A_B));
    let (_, checked) = produce(INFINITE, &declarations);
    let staged = checked.expect("the unit checks and links");
    let unit = staged.value();

    let graph = unit.package().graph();
    let [relation] = graph.step_relations() else {
        panic!("one relation in the graph");
    };
    assert_eq!(relation.name(), "Det");
    let [clause] = graph.hyper_clauses() else {
        panic!("one hyper clause in the graph");
    };
    assert_eq!(clause.name(), "NonInterference");
    let (_, alone) = produce(INFINITE, PREDICATE);
    let predicate_only = alone.expect("the predicate alone checks");
    let obligations = unit.type_report().obligations().len();
    assert_ne!(obligations, 0, "the predicate leaves value obligations");
    assert!(
        obligations > predicate_only.value().type_report().obligations().len(),
        "the report reads the whole source, not the predicate alone"
    );

    let emission = qsl_package::emit_checked(unit.package()).expect("the unit emits");
    let wire: serde_json::Value =
        serde_json::from_slice(emission.package().bytes()).expect("the wire is JSON");
    let nodes = wire["semantic_graph"]["nodes"].as_array().expect("nodes");
    let forms_of = |form: &str| -> Vec<&str> {
        nodes
            .iter()
            .filter(|node| node["semantic_form"] == form)
            .map(|node| node["node_tag"].as_str().expect("a node tag"))
            .collect()
    };
    assert_eq!(forms_of("step_relation"), ["claim"]);
    assert_eq!(forms_of("hyperproperty"), ["claim"]);
}

/// FR-362-AC-7, second sentence: the same unit with the `hyper` clause
/// carrying a fairness constraint under a bounded temporal profile is
/// refused only as `NotEmitted` for that clause, at the constraint's span,
/// and produces no checked unit (no graph and no type report).
#[trace("FR-362-AC-7", "TC-1026")]
#[test]
fn a_fairness_constraint_under_a_bounded_profile_is_refused_only_as_not_emitted() {
    let fairness = "fair weak Config::ConfigVersion::attemptUpdate;";
    let body = "always [0, 3] holds(c.versionNumber @ a = c.versionNumber @ b)";
    let declarations = format!("{PREDICATE}\n{RELATION}\n{}", hyper(fairness, body));
    let (text, checked) = produce(EVENT, &declarations);
    let found = causes(checked.expect_err("the unit is refused"));
    let [CheckCause::Temporal(cause)] = found.as_slice() else {
        panic!("only a temporal refusal: {found:?}");
    };
    let TemporalCause::NotEmitted { span, .. } = &**cause else {
        panic!("not emitted, got {cause:?}");
    };
    assert_eq!(
        &text[span.start..span.end],
        "fair weak Config::ConfigVersion::attemptUpdate;"
    );
}

/// FR-362-AC-7, last sentence: a clause with a fault is refused with its
/// typed hyper cause, never with `NotEmitted`.
#[trace("FR-362-AC-7", "TC-1026")]
#[test]
fn a_faulty_hyper_clause_is_refused_with_its_own_cause_and_not_as_not_emitted() {
    let declarations = hyper("", "always holds(c.nope @ a = c.nope @ b)");
    let (_, checked) = produce(INFINITE, &declarations);
    let found = causes(checked.expect_err("the unit is refused"));
    let [CheckCause::Hyper(cause)] = found.as_slice() else {
        panic!("only a hyper refusal: {found:?}");
    };
    assert!(
        matches!(
            **cause,
            qsl_semantics::check::HyperCause::BodyMissing { .. }
        ),
        "{cause:?}"
    );
}
