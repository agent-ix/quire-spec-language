// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-165 (TC-590): the `StateGraph` family's S3 check of `possible`,
//! `always possible` and `unique path` claims, against TC-458's
//! `ConfigVersion` fixture package (`crate::model_operations`).
//!
//! Each claim ranges over `Config::ConfigVersion`, whose `versionNumber`
//! is a bound integer, so one fixture gives every form a state predicate.

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKey, DomainKind};
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::WireNodeId;
use qsl_semantics::check::{
    Capability, CheckCause, CheckRefusal, CheckedGraph, CheckedNode, CheckedStateGraphClaim,
    NodeKind, StateGraphCause, StateGraphForm,
};
use qsl_semantics::family::ClaimExtent;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_body, attempt_update_modifies_version_number, config_unit_with_body,
    config_version_document, tc464_integer_argument_source,
};

fn document() -> Vec<u8> {
    config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    )
}

/// The checked graph of `body`'s claims, or every refusal with the source
/// text range each names.
fn check(body: &str) -> Result<CheckedGraph, Vec<(CheckRefusal, Option<String>)>> {
    check_under(body, CheckingLimits::default())
}

fn check_under(
    body: &str,
    limits: CheckingLimits,
) -> Result<CheckedGraph, Vec<(CheckRefusal, Option<String>)>> {
    let document = document();
    let (unit, _) = config_unit_with_body(&document, body);
    let declarations = admit_and_assemble_with_body(&document, body)
        .unwrap_or_else(|refusal| panic!("assembly refused: {refusal:?}"))
        .with_unwritten_state_graph_claims();
    let regions = declarations.regions();
    declarations.check(limits).map_err(|refusals| {
        refusals
            .into_iter()
            .map(|refusal| {
                let named = regions.refusal_region(&refusal).map(|region| {
                    let (start, end) = (region.start() as usize, region.end() as usize);
                    unit[start..end].to_owned()
                });
                (refusal, named)
            })
            .collect()
    })
}

fn claim<'g>(graph: &'g CheckedGraph, name: &str) -> &'g CheckedStateGraphClaim {
    graph
        .state_graph_claim(name)
        .unwrap_or_else(|| panic!("`{name}` checked"))
}

const REACHES_TWO: &str = "possible ReachesTwo using v over (c: Config::ConfigVersion) { \
    c.versionNumber = 2 }\n";
const REACHES_THREE: &str = "possible ReachesThree using v over (c: Config::ConfigVersion) { \
    c.versionNumber = 3 }\n";
const STILL_TWO: &str = "always possible StillTwo using v over (c: Config::ConfigVersion) { \
    c.versionNumber = 2 }\n";
const GUARDED: &str = "always possible Guarded using v over (c: Config::ConfigVersion) \
    from (c.versionNumber != 0) { c.versionNumber = 2 }\n";
const UNIQUE: &str = "unique path InOneWay using v over (c: Config::ConfigVersion) \
    from (c.versionNumber = 0) to (c.versionNumber = 2)\n";

/// The one refusal of a unit that must not check, as its catalog code, cause
/// tag and the source text it names.
fn sole_refusal(body: &str) -> (Code, Option<&'static str>, Option<String>, CheckRefusal) {
    let Err(mut refusals) = check(body) else {
        panic!("`{body}` must not check");
    };
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    let (refusal, named) = refusals.remove(0);
    (refusal.cause.code(), refusal.cause.cause(), named, refusal)
}

/// The integer literals of a checked predicate body in tree order: what
/// tells one comparison from another among predicates over one field.
fn literals(node: CheckedNode<'_>) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if let NodeKind::Literal(literal) = node.kind() {
            found.push(format!("{:?}", literal.0));
        }
        pending.extend(node.children().into_iter().rev());
    }
    found
}

/// The literals of the target of the `possible` claim `P` over `predicate`,
/// checked on its own: the reference a predicate of another claim must equal.
fn reference(predicate: &str) -> Vec<String> {
    let graph = check(&format!(
        "possible Ref using v over (c: Config::ConfigVersion) {{ {predicate} }}\n"
    ))
    .expect("the reference claim checks");
    let StateGraphForm::Possible { target } = claim(&graph, "Ref").form() else {
        panic!("possible");
    };
    literals(target.body().expect("checked"))
}

/// The form part of the first criterion (per-object instances and node identity are deferred):
/// `possible` checks to `Possible` with its target checked as a Boolean state
/// body over the `over` parameter.
#[trace("TC-590")]
#[test]
fn possible_checks_to_the_possible_form() {
    let graph = check(&format!("{REACHES_TWO}{REACHES_THREE}")).expect("both claims check");
    for name in ["ReachesTwo", "ReachesThree"] {
        let checked = claim(&graph, name);
        assert_eq!(checked.over().expect("over recorded").name, "c");
        let StateGraphForm::Possible { target } = checked.form() else {
            panic!("{name} is Possible: {:?}", checked.form());
        };
        assert!(!target.is_constant_true());
        let body = target.body().expect("a written target is checked");
        assert_eq!(body.value_type(), &quire_exact::ValueType::Boolean);
        let expected = if name == "ReachesTwo" {
            "c.versionNumber = 2"
        } else {
            "c.versionNumber = 3"
        };
        assert_eq!(literals(body), reference(expected), "{name}'s target");
    }
    assert_ne!(
        reference("c.versionNumber = 2"),
        reference("c.versionNumber = 3")
    );
    assert_eq!(graph.state_graph_claims().len(), 2);
}

/// TC-590, the form part of FR-165 (instances and node identity are deferred): an `always possible` claim without `from` checks with `from`
/// the constant `true`; with `from` it checks that predicate.
#[trace("TC-590")]
#[test]
fn always_possible_checks_with_the_constant_true_or_the_written_from() {
    let graph = check(&format!("{STILL_TWO}{GUARDED}")).expect("both claims check");
    let StateGraphForm::AlwaysPossible { from, target } = claim(&graph, "StillTwo").form() else {
        panic!("always possible");
    };
    assert!(from.is_constant_true());
    assert!(from.body().is_none());
    assert_eq!(
        literals(target.body().expect("checked")),
        reference("c.versionNumber = 2")
    );

    let StateGraphForm::AlwaysPossible { from, target } = claim(&graph, "Guarded").form() else {
        panic!("always possible");
    };
    assert!(!from.is_constant_true());
    let (from, target) = (
        literals(from.body().expect("checked")),
        literals(target.body().expect("checked")),
    );
    assert_eq!(from, reference("c.versionNumber != 0"));
    assert_eq!(target, reference("c.versionNumber = 2"));
    assert_ne!(from, target);
}

/// TC-590, the form part of FR-165 (instances and node identity are deferred): `unique path` checks to `UniquePath` with `from` and `to`.
#[trace("TC-590")]
#[test]
fn unique_path_checks_to_the_unique_path_form() {
    let graph = check(UNIQUE).expect("the claim checks");
    let StateGraphForm::UniquePath { from, to } = claim(&graph, "InOneWay").form() else {
        panic!("unique path");
    };
    let (from, to) = (
        literals(from.body().expect("checked")),
        literals(to.body().expect("checked")),
    );
    assert_eq!(from, reference("c.versionNumber = 0"));
    assert_eq!(to, reference("c.versionNumber = 2"));
    assert_ne!(from, to);
}

/// TC-590, the form part of FR-165 (instances and node identity are deferred): `possible P` and `always possible P` over one `P` record
/// different forms.
#[trace("TC-590")]
#[test]
fn possible_and_always_possible_over_one_predicate_differ_in_form() {
    let graph = check(&format!("{REACHES_TWO}{STILL_TWO}")).expect("both claims check");
    assert!(matches!(
        claim(&graph, "ReachesTwo").form(),
        StateGraphForm::Possible { .. }
    ));
    assert!(matches!(
        claim(&graph, "StillTwo").form(),
        StateGraphForm::AlwaysPossible { .. }
    ));
}

/// TC-590, the form part of FR-165 (instances and node identity are deferred): every claim records (`temporal-satisfaction`, `Unbounded`)
/// over a domain that names the claim's node.
#[trace("TC-590")]
#[test]
fn a_claim_records_temporal_satisfaction_over_an_unbounded_extent() {
    let graph = check(&format!("{REACHES_TWO}{GUARDED}{UNIQUE}")).expect("the claims check");
    for name in ["ReachesTwo", "Guarded", "InOneWay"] {
        let record = claim(&graph, name).claim();
        assert_eq!(record.kind(), Capability::TemporalSatisfaction);
        let node = WireNodeId::from_digest([7; 32]);
        let requirements = record.requirements(node);
        assert_eq!(requirements.kind(), Capability::TemporalSatisfaction);
        let ClaimExtent::Unbounded(domains) = requirements.extent() else {
            panic!("{name}: an unbounded extent");
        };
        let key = DomainKey::Node {
            node,
            path: Vec::new(),
        };
        assert_eq!(domains.len(), 1);
        assert_eq!(domains.kind(&key), Some(DomainKind::InfiniteTrace));
        assert_eq!(requirements.extent().to_wire(), "unbounded");
        // The record names the node it is keyed by and no other.
        let other = WireNodeId::from_digest([8; 32]);
        let ClaimExtent::Unbounded(other_domains) = record.requirements(other).extent().clone()
        else {
            panic!("{name}: an unbounded extent");
        };
        assert_eq!(other_domains.kind(&key), None);
    }
}

/// FR-165-AC-2: a predicate that reads `pre(c.versionNumber)` refuses
/// `unsupported_construct`/`expression-form` at the read, with no checked
/// claim.
#[trace("TC-590", "FR-165-AC-2")]
#[test]
fn a_pre_state_read_refuses_at_the_read() {
    let (code, cause, named, refusal) = sole_refusal(
        "possible P using v over (c: Config::ConfigVersion) { pre(c.versionNumber) = 2 }\n",
    );
    assert_eq!(code, Code::UnsupportedConstruct);
    assert_eq!(cause, Some("expression-form"));
    assert_eq!(named.as_deref(), Some("pre(c.versionNumber)"));
    assert!(matches!(
        &refusal.cause,
        CheckCause::StateGraph(cause) if matches!(**cause, StateGraphCause::ReadsBeyondState { .. })
    ));
}

/// FR-165-AC-2: `self` and `result` are anchors a state predicate may not
/// read.
#[trace("TC-590", "FR-165-AC-2")]
#[test]
fn an_anchor_read_refuses_at_the_read() {
    for read in ["self", "result"] {
        let (code, cause, named, _) = sole_refusal(&format!(
            "possible P using v over (c: Config::ConfigVersion) {{ {read}.versionNumber = 2 }}\n"
        ));
        assert_eq!(code, Code::UnsupportedConstruct, "{read}");
        assert_eq!(cause, Some("expression-form"), "{read}");
        assert_eq!(named.as_deref(), Some(read));
    }
}

/// FR-165-AC-2: a temporal operator in a predicate refuses
/// `unsupported_construct`/`expression-form` at the operator.
#[trace("TC-590", "FR-165-AC-2")]
#[test]
fn a_temporal_operator_refuses_at_the_operator() {
    let (code, cause, named, refusal) = sole_refusal(
        "possible P using v over (c: Config::ConfigVersion) { eventually c.versionNumber = 2 }\n",
    );
    assert_eq!(code, Code::UnsupportedConstruct);
    assert_eq!(cause, Some("expression-form"));
    assert_eq!(named.as_deref(), Some("eventually"));
    assert!(matches!(
        &refusal.cause,
        CheckCause::StateGraph(cause) if matches!(**cause, StateGraphCause::TemporalOperator { .. })
    ));
}

/// FR-165-AC-2: a fairness constraint refuses
/// `unsupported_construct`/`expression-form` at the constraint.
#[trace("TC-590", "FR-165-AC-2")]
#[test]
fn a_fairness_constraint_refuses_at_the_constraint() {
    let (code, cause, named, refusal) = sole_refusal(
        "always possible P using v over (c: Config::ConfigVersion) { \
         fair weak attemptUpdate; c.versionNumber = 2 }\n",
    );
    assert_eq!(code, Code::UnsupportedConstruct);
    assert_eq!(cause, Some("expression-form"));
    assert_eq!(named.as_deref(), Some("fair weak attemptUpdate;"));
    assert!(matches!(
        &refusal.cause,
        CheckCause::StateGraph(cause) if matches!(**cause, StateGraphCause::Fairness { .. })
    ));
}

/// FR-165-AC-2: a claim with one refusal checks nothing else of the unit
/// into the graph: the refused claim is not in the result, and the unit as a
/// whole does not check.
#[trace("TC-590", "FR-165-AC-2")]
#[test]
fn a_refused_claim_leaves_the_unit_unchecked() {
    let body = format!(
        "{REACHES_TWO}possible Bad using v over (c: Config::ConfigVersion) {{ \
         eventually c.versionNumber = 2 }}\n"
    );
    assert!(check(&body).is_err());
}

/// FR-165 "Predicates": a predicate checks as an invariant body does, so a
/// non-Boolean predicate refuses as a non-Boolean state body, at the
/// predicate.
#[trace("TC-590", "FR-165-AC-2")]
#[test]
fn a_non_boolean_predicate_refuses_as_an_invariant_body_does() {
    let (code, cause, named, _) = sole_refusal(
        "possible P using v over (c: Config::ConfigVersion) { c.versionNumber + 1 }\n",
    );
    assert_eq!(code, Code::IllTyped);
    assert_eq!(cause, Some("non-boolean-root"));
    assert_eq!(named.as_deref(), Some("c.versionNumber + 1"));
}

/// FR-165-AC-2: a temporal operator inside a connective, a negation or
/// parentheses is refused at the operator, as one leading an operand is.
#[trace("TC-590", "FR-165-AC-2")]
#[test]
fn a_temporal_operator_inside_a_connective_negation_or_parentheses_refuses_at_the_operator() {
    for predicate in [
        "c.versionNumber = 1 and eventually c.versionNumber = 2",
        "(eventually c.versionNumber = 2)",
        "not eventually c.versionNumber = 2",
    ] {
        let (code, cause, named, _) = sole_refusal(&format!(
            "possible P using v over (c: Config::ConfigVersion) {{ {predicate} }}\n"
        ));
        assert_eq!(code, Code::UnsupportedConstruct, "{predicate}");
        assert_eq!(cause, Some("expression-form"), "{predicate}");
        assert_eq!(named.as_deref(), Some("eventually"), "{predicate}");
    }
}

/// FR-165 "Requirement record": a claim that checks is refused as not
/// emitted unless the caller asks to see the checked claim, so a compiled
/// package never silently lacks it.
#[trace("TC-590")]
#[test]
fn a_checked_claim_is_refused_as_not_emitted_by_default() {
    let document = document();
    let (unit, _) = config_unit_with_body(&document, REACHES_TWO);
    let declarations =
        admit_and_assemble_with_body(&document, REACHES_TWO).expect("the unit assembles");
    let regions = declarations.regions();
    let Err(refusals) = declarations.check(CheckingLimits::default()) else {
        panic!("a state-graph claim is not emitted yet");
    };
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    assert_eq!(refusal.cause.code(), Code::UnsupportedConstruct);
    assert_eq!(refusal.cause.cause(), Some("not-yet-implemented"));
    let region = regions
        .refusal_region(refusal)
        .expect("located at the claim");
    assert_eq!(
        unit[region.start() as usize..region.end() as usize].trim_end(),
        REACHES_TWO.trim_end()
    );
}

/// Two claims named alike refuse at the second, so a name keys one claim.
#[trace("TC-590")]
#[test]
fn a_second_claim_with_a_name_refuses_at_the_second() {
    let body = format!("{REACHES_TWO}{REACHES_TWO}");
    let document = document();
    let (unit, _) = config_unit_with_body(&document, &body);
    let refusal = admit_and_assemble_with_body(&document, &body)
        .expect_err("a duplicate name refuses at assembly");
    let [error] = refusal.errors.as_slice() else {
        panic!("one error: {refusal:?}");
    };
    assert_eq!(error.cause.code(), Code::ConflictingDeclaration);
    assert_eq!(&unit[error.span.start..error.span.end], "ReachesTwo");
    assert_eq!(error.span.start, unit.rfind("ReachesTwo").expect("second"));
}

/// The node limit applies to the claim's cumulative total: a limit one
/// predicate fits under refuses a claim of two such predicates.
#[trace("TC-590")]
#[test]
fn the_node_limit_applies_to_the_claims_cumulative_total() {
    let one = "possible P using v over (c: Config::ConfigVersion) { c.versionNumber = 2 }\n";
    let two = "unique path P using v over (c: Config::ConfigVersion) \
        from (c.versionNumber = 2) to (c.versionNumber = 2)\n";
    let fits = (1..400)
        .find(|nodes| check_under(one, CheckingLimits::new(*nodes)).is_ok())
        .expect("a limit one predicate fits under");
    assert!(check_under(two, CheckingLimits::new(fits)).is_err());
    assert!(check_under(two, CheckingLimits::new(fits * 2 + 8)).is_ok());
}

/// An `over` type that exists but is no object type refuses `ill_typed`/
/// `type-mismatch` at the type, not as a missing name.
#[trace("TC-590")]
#[test]
fn an_over_type_that_is_not_an_object_type_refuses_at_the_type() {
    let body = "possible P using v over (c: Integer) { c = 2 }\n";
    let document = document();
    let (unit, _) = config_unit_with_body(&document, body);
    let refusal = admit_and_assemble_with_body(&document, body)
        .expect_err("a non-object over type refuses at assembly");
    let [error] = refusal.errors.as_slice() else {
        panic!("one error: {refusal:?}");
    };
    assert_eq!(error.cause.code(), Code::IllTyped);
    assert_eq!(error.cause.catalog_code().cause(), "type-mismatch");
    assert_eq!(&unit[error.span.start..error.span.end], "Integer");
}

/// FR-165-AC-4 (S3 half): each of the three forms written without `over`
/// checks to its form and records no `over` binding; with `over` the binding
/// is recorded.
#[trace("TC-590", "FR-165-AC-4")]
#[test]
fn a_claim_without_over_checks_and_records_no_binding() {
    let graph = check(concat!(
        "possible Anything using v { true }\n",
        "always possible Still using v from (true) { true }\n",
        "unique path One using v from (true) to (false)\n",
    ))
    .expect("the claims check");
    assert!(matches!(
        claim(&graph, "Anything").form(),
        StateGraphForm::Possible { .. }
    ));
    assert!(matches!(
        claim(&graph, "Still").form(),
        StateGraphForm::AlwaysPossible { .. }
    ));
    assert!(matches!(
        claim(&graph, "One").form(),
        StateGraphForm::UniquePath { .. }
    ));
    for name in ["Anything", "Still", "One"] {
        assert!(claim(&graph, name).over().is_none(), "{name}");
    }

    let graph = check(REACHES_TWO).expect("the claim checks");
    let over = claim(&graph, "ReachesTwo").over().expect("over recorded");
    assert_eq!(over.name, "c");
}

/// FR-165-AC-4: a claim with no `over` has no object to name, so a predicate
/// that names one refuses as a missing name.
#[trace("TC-590", "FR-165-AC-4")]
#[test]
fn a_claim_without_over_binds_no_object_name() {
    let (code, cause, named, _) = sole_refusal("possible P using v { c.versionNumber = 2 }\n");
    assert_eq!(code, Code::MissingDeclaration);
    assert_eq!(cause, Some("missing-name"));
    assert_eq!(named.as_deref(), Some("c"));
}

/// FR-165-AC-4: the three formula shapes S1 reads are each refused at
/// `eventually`, and the claim without `over` checks.
#[trace("TC-590", "FR-165-AC-4")]
#[test]
fn ac4_claims_check_or_refuse_at_eventually() {
    check("possible Anything using v { true }\n").expect("the first claim checks");
    for predicate in [
        "c.versionNumber = 2 and eventually c.versionNumber = 3",
        "not eventually c.versionNumber = 2",
        "(eventually c.versionNumber = 2)",
    ] {
        let (code, cause, named, _) = sole_refusal(&format!(
            "possible R using v over (c: Config::ConfigVersion) {{ {predicate} }}\n"
        ));
        assert_eq!(code, Code::UnsupportedConstruct, "{predicate}");
        assert_eq!(cause, Some("expression-form"), "{predicate}");
        assert_eq!(named.as_deref(), Some("eventually"), "{predicate}");
    }
}

/// FR-165-AC-5: a predicate naming an operation parameter refuses
/// `missing-name` at the name; the same claim reading `pre(...)` still
/// refuses `expression-form`.
#[trace("TC-590", "FR-165-AC-5")]
#[test]
fn an_operation_parameter_read_refuses_as_a_missing_name() {
    let (document, _) = tc464_integer_argument_source();
    let body = "possible P using v over (c: Config::ConfigVersion) { c.versionNumber = amount }\n";
    let (unit, _) = config_unit_with_body(&document, body);
    let declarations = admit_and_assemble_with_body(&document, body)
        .expect("the unit assembles")
        .with_unwritten_state_graph_claims();
    let regions = declarations.regions();
    let refusals = declarations
        .check(CheckingLimits::default())
        .expect_err("the parameter is not in scope");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    assert_eq!(refusal.cause.code(), Code::MissingDeclaration);
    assert_eq!(refusal.cause.cause(), Some("missing-name"));
    let region = regions.refusal_region(refusal).expect("located");
    assert_eq!(
        &unit[region.start() as usize..region.end() as usize],
        "amount"
    );

    let (code, cause, _, _) = sole_refusal(
        "possible P using v over (c: Config::ConfigVersion) { pre(c.versionNumber) = 2 }\n",
    );
    assert_eq!(code, Code::UnsupportedConstruct);
    assert_eq!(cause, Some("expression-form"));
}
