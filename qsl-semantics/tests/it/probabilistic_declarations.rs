// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-185 (TC-620): the S3 check of the `random`, `reward` and `workload`
//! declarations over ADR-024 §7.2's `Service` model, and FR-186 (TC-621):
//! the form checks of a `probabilistic` claim.
//!
//! `Service` is a domain package with one object type, `Server { phase:
//! Phase, attempts: Attempts }`, `Phase` and `Outcome` variant types,
//! `Attempts = Int[0, 2]`, and the operations `request`, `attempt(d,
//! outcome)` and `reset`. ADR-024 types `d` as `Quantity<Time>`; neither a
//! domain package nor complete-V1 source can declare a quantity-typed
//! parameter yet (STD-113), so this fixture types `d` as `Millis = Int[1,
//! 12]` and the ADR's `1 ms`, `3 ms` and `12 ms` are written `1`, `3` and
//! `12`. That is why FR-185-AC-1 and FR-185-AC-2, which name the quantity
//! type, are not tagged here; the unit does declare `dimension Time` and
//! `unit ms`, so a quantity literal types.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations, PreconditionSite,
    ProbabilisticCause, WeightOwner,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use quire_exact::{Integer, Rational, ValueType};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value};

use crate::model_operations::{parse_and_build, wire_envelope, wire_field, PACKAGE_IDENTITY};

const P: &str = PACKAGE_IDENTITY;

fn id(node: &str) -> String {
    format!("ix://{P}/{node}")
}

fn generated(node: &str) -> Value {
    json!({"generated": {
        "generatorIdentity": id(node),
        "generatorVersion": "1.0.0",
        "inputIdentities": [id(node)],
    }})
}

fn construct(name: &str, meaning: &str, shape: &str, members: Value) -> Value {
    json!({
        "kind": {"module": P, "name": name},
        "moduleVersion": "1.0.0",
        "manifestDigest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "construct": {"identity": "none", "shape": shape, "members": members, "meaning": meaning},
    })
}

fn definition(name: &str, kind: &str, members: Value) -> Value {
    let mut definition = json!({
        "identity": id(name), "displayName": name,
        "kind": {"module": P, "name": kind},
        "roles": [], "origin": generated(name),
        "constraints": [], "extensions": [], "unknownPolicy": "reject",
    });
    definition
        .as_object_mut()
        .expect("an object")
        .extend(members.as_object().expect("an object").clone());
    definition
}

fn variant_type(name: &str, cases: &[&str]) -> Value {
    let cases: Vec<Value> = cases
        .iter()
        .enumerate()
        .map(|(line, case)| {
            json!({
                "identity": id(&format!("variant/{name}-{case}")),
                "name": case,
                "origin": {"source": {
                    "sourceIdentity": id("spec"), "path": "spec.qspec",
                    "startLine": line + 1, "startColumn": 3,
                }},
            })
        })
        .collect();
    definition(name, "variant_type", json!({"variants": cases}))
}

fn bounded_integer(name: &str, lower: &str, upper: &str) -> Value {
    let constraint = |keyword: &str, value: &str| {
        json!({
            "identity": id(&format!("{name}/constraints/{keyword}")),
            "keyword": keyword,
            "operands": {"value": value},
            "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"),
            "origin": generated(name),
        })
    };
    definition(
        name,
        "value_type",
        json!({
            "scalar": "integer",
            "constraints": [constraint("min", lower), constraint("max", upper)],
        }),
    )
}

fn parameter(operation: &str, name: &str, type_ref: &str) -> Value {
    let identity = id(&format!("Server/{operation}/{name}"));
    json!({
        "identity": identity.clone(), "name": name, "typeRef": type_ref,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"generated": {
            "generatorIdentity": identity.clone(), "generatorVersion": "1.0.0",
            "inputIdentities": [identity],
        }},
    })
}

fn operation(name: &str, params: Vec<Value>, modifies: &[&str]) -> Value {
    let modifies: Vec<String> = modifies
        .iter()
        .map(|field| id(&format!("Server/{field}")))
        .collect();
    json!({
        "identity": id(&format!("Server/{name}")),
        "name": name,
        "params": params,
        "pre": [],
        "post": [],
        "origin": {"source": {
            "sourceIdentity": id("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1,
        }},
        "frame": {"modifies": modifies, "creates": [], "deletes": []},
    })
}

/// ADR-024 §7.2's `Service` domain package (see the module doc for `d`).
fn service_document() -> Vec<u8> {
    service_document_with(&[])
}

/// [`service_document`], with `requires` as `attempt`'s domain-package
/// `requires` clauses.
fn service_document_with(requires: &[&str]) -> Vec<u8> {
    let mut attempt = operation(
        "attempt",
        vec![
            parameter("attempt", "d", &id("Millis")),
            parameter("attempt", "outcome", &id("Outcome")),
        ],
        &["phase", "attempts"],
    );
    let clauses: Vec<Value> = requires
        .iter()
        .map(|text| json!({"language": "quire", "text": text, "origin": attempt["origin"].clone()}))
        .collect();
    attempt["pre"] = json!(clauses);
    let server = definition(
        "Server",
        "object_type",
        json!({
            "supertypes": [],
            "fields": [
                wire_field(&id("Server/phase"), "phase", &id("Phase"), "required", 1),
                wire_field(&id("Server/attempts"), "attempts", &id("Attempts"), "required", 1),
            ],
            "operations": [
                operation("request", vec![], &["phase", "attempts"]),
                attempt,
                operation("reset", vec![], &["phase"]),
            ],
            "relationships": [],
        }),
    );
    let population = json!({
        "identity": id("servers"), "displayName": id("servers"),
        "kind": {"module": P, "name": "population"},
        "members": [id("Server")], "extent": "closed",
        "origin": generated("servers"),
    });
    let constructs = json!([
        construct("object_type", meaning::OBJECT_TYPE, "record", json!({})),
        construct("population", meaning::POPULATION, "record", json!({})),
        construct("value_type", meaning::VALUE_TYPE, "record", json!({})),
        construct(
            "variant_type",
            meaning::VARIANT_TYPE,
            "enumeration",
            json!({"variants": "required"})
        ),
    ]);
    let types = json!([
        variant_type("Phase", &["Idle", "Busy", "Done"]),
        variant_type("Outcome", &["Ok", "Fail"]),
        bounded_integer("Attempts", "0", "2"),
        bounded_integer("Millis", "1", "12"),
        server,
    ]);
    wire_envelope(constructs, types, json!([population]))
        .to_string()
        .into_bytes()
}

/// The unit selecting `Service`, with `dimension Time` and `unit ms`, and
/// `body` after them.
fn service_unit(body: &str) -> (String, std::collections::BTreeMap<[u8; 32], Vec<u8>>) {
    service_unit_over(&service_document(), body)
}

/// [`service_unit`] over `document`.
fn service_unit_over(
    document: &[u8],
    body: &str,
) -> (String, std::collections::BTreeMap<[u8; 32], Vec<u8>>) {
    let packages = package_input([document]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model Service = {P:?} version \"1.0.0\" digest \"sha256-jcs:{digest}\";\n\
         dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         {body}\n"
    );
    (unit, packages)
}

fn assemble(body: &str) -> PackageDeclarations {
    assemble_over(&service_document(), body)
}

/// [`assemble`] over `document`.
fn assemble_over(document: &[u8], body: &str) -> PackageDeclarations {
    let (unit, packages) = service_unit_over(document, body);
    let built = parse_and_build(&unit);
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("I1 refused: {refusal:?}"))
    .with_parsed_operation_clauses(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    })
    .unwrap_or_else(|failure| panic!("a kept operation clause does not parse: {failure:?}"));
    let raw = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-620", "fixture", "fixture:1"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit")
    .source()
    .reference()
    .clone();
    PackageDeclarations::assemble(raw, built, models, Vec::new())
        .unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"))
}

fn checked(body: &str) -> CheckedGraph {
    assemble(body)
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{body}: the unit checks: {refusals:?}"))
}

fn refusals_of(body: &str) -> Vec<CheckRefusal> {
    assemble(body)
        .check(CheckingLimits::default())
        .expect_err("the unit refuses")
}

/// Each refusal's code, catalog cause and the source text its region names.
fn located(body: &str) -> Vec<(Code, Option<&'static str>, String)> {
    let declarations = assemble(body);
    let regions = declarations.regions();
    let (text, _) = service_unit(body);
    declarations
        .check(CheckingLimits::default())
        .expect_err("the unit refuses")
        .iter()
        .map(|refusal| {
            let region = regions
                .refusal_region(refusal)
                .unwrap_or_else(|| panic!("{refusal:?} names a region of the unit"));
            (
                refusal.cause.code(),
                refusal.cause.cause(),
                text[region.start() as usize..region.end() as usize].to_owned(),
            )
        })
        .collect()
}

fn invalid(cause: &'static str, at: &str) -> (Code, Option<&'static str>, String) {
    (Code::InvalidProbabilisticModel, Some(cause), at.to_owned())
}

fn not_yet(at: &str) -> (Code, Option<&'static str>, String) {
    (
        Code::UnsupportedConstruct,
        Some("not-yet-implemented"),
        at.to_owned(),
    )
}

const D: &str = "random Server::attempt(d) ~ { 1: 90, 3: 7, 12: 3 };";
const OUTCOME: &str =
    "random Server::attempt(outcome) ~ { Service::Outcome::Ok: 98, Service::Outcome::Fail: 2 };";
const DURATION: &str = "reward duration on Server::attempt = d;";
const STEADY: &str = "workload Steady on Service { weight Server::request = 1; \
     weight Server::attempt = 1; weight Server::reset = 1; }";

fn service(replacing: Option<(&str, &str)>, extra: &str) -> String {
    let mut declarations = vec![D, OUTCOME, DURATION, STEADY];
    if let Some((old, new)) = replacing {
        for declaration in &mut declarations {
            if *declaration == old {
                *declaration = new;
            }
        }
    }
    format!("{}\n{extra}", declarations.join("\n"))
}

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

/// TC-620 step 1: `Service` checks, and the checked graph records each
/// random parameter's support and probabilities in declared order, the
/// `duration` reward's value type and `Steady`'s weight per operation.
#[trace("TC-620")]
#[test]
fn the_service_model_checks_and_records_its_declarations() {
    let graph = checked(&service(None, ""));
    let model = graph.probabilistic_model();
    let [d, outcome] = model.random_parameters() else {
        panic!("two random parameters: {:?}", model.random_parameters());
    };
    assert_eq!((d.operation().name(), d.parameter()), ("attempt", "d"));
    assert_eq!(d.operation().model(), "Service");
    assert_eq!(d.position(), 0);
    let probabilities: Vec<Rational> = d.support().iter().map(|v| v.probability.clone()).collect();
    assert_eq!(probabilities, [ratio(9, 10), ratio(7, 100), ratio(3, 100)]);
    let values: Vec<String> = d
        .support()
        .iter()
        .map(|value| format!("{:?}", value.value.0))
        .collect();
    assert_eq!(values.len(), 3);
    assert!(
        values[0].contains('1') && values[2].contains("12"),
        "{values:?}"
    );
    assert_eq!(
        outcome
            .support()
            .iter()
            .map(|value| value.probability.clone())
            .collect::<Vec<_>>(),
        [ratio(49, 50), ratio(1, 50)]
    );
    assert_eq!(outcome.position(), 1);
    assert_eq!(
        model.random_parameter(d.operation().key(), "outcome"),
        Some(outcome)
    );

    let [duration] = model.rewards() else {
        panic!("one reward");
    };
    assert_eq!(duration.name(), "duration");
    assert_eq!(duration.operation(), d.operation());
    assert!(matches!(duration.value_type(), ValueType::Int(_)));
    assert_eq!(duration.parameters().len(), 2);

    let steady = model.workload("Steady").expect("Steady is recorded");
    assert_eq!(steady.model(), "Service");
    let weights: Vec<(&str, Rational)> = steady
        .weights()
        .iter()
        .map(|(operation, weight)| (operation.name(), weight.clone()))
        .collect();
    assert_eq!(
        weights,
        [
            ("request", ratio(1, 1)),
            ("attempt", ratio(1, 1)),
            ("reset", ratio(1, 1))
        ]
    );
}

/// The `Probabilistic` cause of each refusal.
fn causes(body: &str) -> Vec<ProbabilisticCause> {
    refusals_of(body)
        .into_iter()
        .map(|refusal| match refusal.cause {
            CheckCause::Probabilistic(cause) => *cause,
            other => panic!("{body}: not a probabilistic refusal: {other:?}"),
        })
        .collect()
}

fn names_parameter(cause: &ProbabilisticCause, expected: &str) -> bool {
    match cause {
        ProbabilisticCause::InvalidWeight {
            owner: WeightOwner::Parameter { parameter, .. },
            ..
        }
        | ProbabilisticCause::RepeatedSupportValue { parameter, .. }
        | ProbabilisticCause::DuplicateDistribution { parameter, .. }
        | ProbabilisticCause::ValueOutsideType { parameter, .. }
        | ProbabilisticCause::RandomReadInPrecondition { parameter, .. } => parameter == expected,
        _ => false,
    }
}

/// TC-620 step 2 (FR-185 "Random parameters"): each random-parameter variant
/// of `Service` refuses with a diagnostic naming the parameter and carrying
/// its cause, at the value, weight or declaration it names.
#[trace("TC-620")]
#[test]
fn random_parameter_variants_refuse_naming_the_parameter() {
    for (replacement, cause, at) in [
        (
            "random Server::attempt(d) ~ { 1: 0, 3: 7 };",
            "invalid-weight",
            "0",
        ),
        (
            "random Server::attempt(d) ~ { 1: rational(-1, 1), 3: 7 };",
            "invalid-weight",
            "rational(-1, 1)",
        ),
        (
            "random Server::attempt(d) ~ { 1: 9, 1: 1 };",
            "repeated-support-value",
            "1",
        ),
        (
            "random Server::attempt(d) ~ { true: 9, 3: 1 };",
            "value-outside-type",
            "true",
        ),
        (
            "random Server::attempt(d) ~ { 1 ms: 9, 3: 1 };",
            "value-outside-type",
            "1 ms",
        ),
        (
            "random Server::attempt(d) ~ { 13: 9, 3: 1 };",
            "value-outside-type",
            "13",
        ),
        (
            "random Server::attempt(d) ~ { 1 + 2: 9 };",
            "value-outside-type",
            "1 + 2",
        ),
    ] {
        let body = service(Some((D, replacement)), "");
        assert_eq!(located(&body), [invalid(cause, at)], "{replacement}");
        let [found] = &causes(&body)[..] else {
            panic!("{replacement}: one refusal");
        };
        assert!(names_parameter(found, "d"), "{found:?}");
    }
    // The same value in another notation is the same value.
    let body = service(
        Some((
            OUTCOME,
            "random Server::attempt(outcome) ~ { Service::Outcome::Ok: 1, Service::Outcome::Ok: 2 };",
        )),
        "",
    );
    assert_eq!(
        located(&body),
        [invalid("repeated-support-value", "Service::Outcome::Ok")]
    );
    // A second distribution of `d` refuses at the second declaration.
    let second = "random Server::attempt(d) ~ { 3: 1 };";
    let body = service(None, second);
    assert_eq!(located(&body), [invalid("duplicate-distribution", second)]);
    assert!(names_parameter(&causes(&body)[0], "d"));
}

/// TC-620 step 2: a precondition of `Server::attempt` that reads `d`
/// refuses `random-read-in-precondition` at the read, naming the parameter
/// and the precondition; a postcondition may read it.
#[trace("TC-620")]
#[test]
fn a_precondition_that_reads_a_random_parameter_refuses_at_the_read() {
    let pre = "pre Busy using v on Service::Server::attempt \
               { self.phase = Service::Phase::Busy and d < 5 }";
    let body = service(None, pre);
    assert_eq!(
        located(&body),
        [invalid("random-read-in-precondition", "d")]
    );
    assert_eq!(
        causes(&body),
        [ProbabilisticCause::RandomReadInPrecondition {
            operation: "attempt".to_owned(),
            parameter: "d".to_owned(),
            precondition: PreconditionSite::Clause("Busy".to_owned()),
        }]
    );
    // The same read of a parameter that is not random checks.
    let unread = "pre Busy using v on Service::Server::attempt \
                  { self.phase = Service::Phase::Busy and d < 5 }";
    checked(&format!("{OUTCOME}\n{DURATION}\n{STEADY}\n{unread}"));
    // A postcondition reads it.
    checked(&service(
        None,
        "post Counted using v on Service::Server::attempt { d >= 1 }",
    ));
    // A domain-package `requires` clause that reads it refuses at its
    // position.
    let document = service_document_with(&["self.phase = Service::Phase::Busy and d < 5"]);
    let refusals = assemble_over(&document, &service(None, ""))
        .check(CheckingLimits::default())
        .expect_err("the requires reads d");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    assert_eq!(refusal.cause.cause(), Some("random-read-in-precondition"));
    assert_eq!(
        refusal.operation_member().map(|member| member.to_string()),
        Some("Operation.pre[0]".to_owned())
    );
}

/// TC-620 step 2: an operation or parameter the declaration names that does
/// not resolve refuses `missing_declaration`/`missing-name` at the name.
#[trace("TC-620")]
#[test]
fn an_unresolved_operation_or_parameter_refuses_missing_name() {
    let missing = |at: &str| {
        (
            Code::MissingDeclaration,
            Some("missing-name"),
            at.to_owned(),
        )
    };
    assert_eq!(
        located(&service(None, "random Server::launch(d) ~ { 1: 1 };")),
        [missing("Server::launch")]
    );
    assert_eq!(
        located(&service(
            None,
            "random Other::Server::attempt(d) ~ { 1: 1 };"
        )),
        [missing("Other::Server::attempt")]
    );
    assert_eq!(
        located(&service(None, "random Server::attempt(z) ~ { 1: 1 };")),
        [missing("z")]
    );
    assert_eq!(
        located(&service(None, "reward cost on Server::launch = 1;")),
        [missing("Server::launch")]
    );
    assert_eq!(
        located(&service(
            Some((
                STEADY,
                "workload Steady on Other { weight Server::request = 1; }"
            )),
            ""
        )),
        [missing("Other")]
    );
    // The model may be named.
    checked(&service(
        Some((
            D,
            "random Service::Server::attempt(d) ~ { 1: 90, 3: 7, 12: 3 };",
        )),
        "",
    ));
}

/// `2^(bits - 1)`, an integer of exactly `bits` bits, in decimal.
fn integer_of_bits(bits: u64) -> String {
    Integer::one().shifted_left(bits - 1).to_string()
}

fn steady_with_request(weight: &str) -> String {
    format!(
        "workload Steady on Service {{ weight Server::request = {weight}; \
         weight Server::attempt = 1; weight Server::reset = 1; }}"
    )
}

/// TC-620 step 2 (FR-185 "Exact values"), each as the weight of
/// `Server::request`: a value past 65,536 bits as written refuses before it
/// is reduced or expanded (`not-yet-implemented` until QSL-883 names its
/// limit refusal), even when it reduces to 1; one within the ceiling is
/// admitted and recorded reduced.
#[trace("TC-620")]
#[test]
fn exact_values_are_bounded_as_written() {
    let n = integer_of_bits(70_000);
    for value in [
        "decimal(1, 4294967295)".to_owned(),
        format!("rational({n}, {n})"),
        format!("rational({}, 1)", integer_of_bits(65_537)),
        "decimal(1, 19729)".to_owned(),
    ] {
        let body = service(Some((STEADY, &steady_with_request(&value))), "");
        assert_eq!(located(&body), [not_yet(&value)], "{value:.40}");
    }
    let m = integer_of_bits(65_536);
    for (value, expected) in [
        (
            format!("rational({m}, 1)"),
            Rational::from_integer(m.parse().expect("an integer")),
        ),
        (
            "decimal(1, 19728)".to_owned(),
            Rational::from_integer(Integer::one()).divided_by_power_of_ten(19_728),
        ),
        ("decimal(25, 3)".to_owned(), ratio(1, 40)),
        ("decimal(5, 0)".to_owned(), ratio(5, 1)),
        ("0.5".to_owned(), ratio(1, 2)),
    ] {
        let graph = checked(&service(Some((STEADY, &steady_with_request(&value))), ""));
        let steady = graph
            .probabilistic_model()
            .workload("Steady")
            .expect("Steady");
        assert_eq!(steady.weights()[0].1, expected, "{value:.40}");
    }
    // A zero denominator is undefined.
    let body = service(Some((STEADY, &steady_with_request("rational(1, 0)"))), "");
    assert_eq!(
        located(&body),
        [(
            Code::UndefinedExpression,
            Some("unproved-nonzero"),
            "rational(1, 0)".to_owned()
        )]
    );
}

/// TC-620 step 2: a bare `-1` is no `exact-value`, so S1 refuses it and S3
/// never sees it.
#[trace("TC-620")]
#[test]
fn a_bare_negative_weight_is_refused_at_s1() {
    let (unit, _) = service_unit("random Server::attempt(d) ~ { 1: -1 };");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-620", "fixture", "fixture:1"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    );
    let codes: Vec<qsl_cst::CompleteCode> = match &parsed {
        Ok(parsed) => parsed.diagnostics().iter().map(|d| d.code).collect(),
        Err(failure) => vec![failure.code],
    };
    assert!(
        codes.contains(&qsl_cst::CompleteCode::InvalidSyntax),
        "{codes:?}"
    );
}

fn names_workload_operation(cause: &ProbabilisticCause, expected: (&str, &str)) -> bool {
    match cause {
        ProbabilisticCause::IncompleteWorkload {
            workload,
            operation,
            ..
        }
        | ProbabilisticCause::UnknownOperation {
            workload,
            operation,
            ..
        }
        | ProbabilisticCause::DuplicateWeight {
            workload,
            operation,
            ..
        }
        | ProbabilisticCause::InvalidWeight {
            owner:
                WeightOwner::Workload {
                    workload,
                    operation,
                },
            ..
        } => (workload.as_str(), operation.as_str()) == expected,
        _ => false,
    }
}

/// TC-620 step 3 (FR-185 "Workloads"): each workload variant refuses naming
/// the workload and the operation and carrying its cause.
#[trace("TC-620", "FR-185-AC-3")]
#[test]
fn workload_variants_refuse_naming_the_workload_and_the_operation() {
    let without_reset =
        "workload Steady on Service { weight Server::request = 1; weight Server::attempt = 1; }";
    let body = service(Some((STEADY, without_reset)), "");
    assert_eq!(
        located(&body),
        [invalid("incomplete-workload", without_reset)]
    );
    assert!(names_workload_operation(
        &causes(&body)[0],
        ("Steady", "Server::reset")
    ));

    let unknown = "workload Steady on Service { weight Server::request = 1; \
         weight Server::attempt = 1; weight Server::reset = 1; weight Server::launch = 1; }";
    let body = service(Some((STEADY, unknown)), "");
    assert_eq!(
        located(&body),
        [invalid("unknown-operation", "Server::launch")]
    );
    assert!(names_workload_operation(
        &causes(&body)[0],
        ("Steady", "Server::launch")
    ));

    let twice = "workload Steady on Service { weight Server::request = 1; \
         weight Server::attempt = 1; weight Server::reset = 1; weight Server::request = 2; }";
    let body = service(Some((STEADY, twice)), "");
    assert_eq!(
        located(&body),
        [invalid("duplicate-weight", "weight Server::request = 2;")]
    );
    assert!(names_workload_operation(
        &causes(&body)[0],
        ("Steady", "Server::request")
    ));

    let body = service(Some((STEADY, &steady_with_request("0"))), "");
    assert_eq!(located(&body), [invalid("invalid-weight", "0")]);
    assert!(names_workload_operation(
        &causes(&body)[0],
        ("Steady", "Server::request")
    ));
}

fn names_reward(cause: &ProbabilisticCause, expected: &str) -> bool {
    match cause {
        ProbabilisticCause::StateReadInReward { reward, .. }
        | ProbabilisticCause::RewardType { reward, .. }
        | ProbabilisticCause::DuplicateReward { reward, .. } => reward == expected,
        _ => false,
    }
}

/// TC-620 step 4 (FR-185 "Rewards"): each reward variant refuses naming the
/// reward and carrying its cause, and `reward cost on Server::request = 2`
/// checks.
#[trace("TC-620", "FR-185-AC-4")]
#[test]
fn reward_variants_refuse_naming_the_reward() {
    let body = service(
        Some((
            DURATION,
            "reward duration on Server::attempt = self.attempts;",
        )),
        "",
    );
    assert_eq!(located(&body), [invalid("state-read-in-reward", "self")]);
    assert!(names_reward(&causes(&body)[0], "duration"));

    let body = service(
        None,
        "reward ok on Server::attempt = (outcome = Service::Outcome::Ok);",
    );
    assert_eq!(
        located(&body),
        [invalid("reward-type", "outcome = Service::Outcome::Ok")]
    );
    assert!(names_reward(&causes(&body)[0], "ok"));

    let second = "reward duration on Server::attempt = 2 * d;";
    let body = service(None, second);
    assert_eq!(located(&body), [invalid("duplicate-reward", second)]);
    assert!(names_reward(&causes(&body)[0], "duration"));

    let graph = checked(&service(None, "reward cost on Server::request = 2;"));
    let rewards = graph.probabilistic_model().rewards();
    let cost = rewards
        .iter()
        .find(|reward| reward.name() == "cost")
        .expect("cost is recorded");
    assert_eq!(cost.operation().name(), "request");
    assert!(matches!(
        cost.value_type(),
        ValueType::Integer | ValueType::Int(_)
    ));
}

/// FR-185 "Rewards": a reward of a quantity type checks, with the quantity
/// type of its unit; the same reward name on another operation, and another
/// name on the same operation, are other rewards.
#[trace("TC-620")]
#[test]
fn a_quantity_reward_checks_and_reward_names_are_per_operation() {
    let graph = checked(&service(
        None,
        "reward latency on Server::request = 5 ms;\n\
         reward duration on Server::reset = 1;\n\
         reward latency on Server::attempt = rational(1, 2) ms;",
    ));
    let rewards = graph.probabilistic_model().rewards();
    assert_eq!(rewards.len(), 4);
    let latency = &rewards[1];
    assert_eq!(latency.name(), "latency");
    let ValueType::Quantity(unit) = latency.value_type() else {
        panic!("a quantity: {:?}", latency.value_type());
    };
    assert_eq!(rewards[3].value_type(), &ValueType::Quantity(*unit));
}

/// FR-185: a quantity literal types against the unit it names and refuses an
/// unknown unit; in a function body it types, and S4 has no literal form to
/// emit it as, so it refuses `not-yet-implemented`.
#[trace("TC-620")]
#[test]
fn quantity_literals_type_against_the_declared_units() {
    let refusals = refusals_of("function lag using v(): Boolean pure { 5 ms <= 3 ms }");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    assert_eq!(
        (refusal.cause.code(), refusal.cause.cause()),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
    let refusals = refusals_of("function lag using v(): Boolean pure { 5 parsec <= 3 ms }");
    assert_eq!(
        refusals
            .iter()
            .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
            .collect::<Vec<_>>(),
        [(Code::MissingDeclaration, Some("missing-name"))]
    );
    let refusals = refusals_of("function lag using v(): Boolean pure { 5 ms }");
    assert_eq!(refusals[0].cause.code(), Code::IllTyped, "{refusals:?}");
}

fn claim(body: &str) -> String {
    format!("probabilistic C using v on Service under Steady on origin {body}")
}

/// A claim that passes its form checks refuses `not-yet-implemented` at the
/// claim: nothing probabilistic compiles unchecked.
#[trace("TC-621")]
#[test]
fn a_claim_that_passes_the_form_checks_refuses_not_yet_implemented() {
    let source = claim(
        "with alpha 0.01, beta 0.01, indifference 0.0005 \
         { probability >= 0.999 [ always[0,1000] holds(n.healthy) ] }",
    );
    assert_eq!(located(&service(None, &source)), [not_yet(&source)]);
}

/// FR-186 "Thresholds": a probability threshold, quantile level, mean
/// threshold or long-run threshold of 0 or 1 refuses `qualitative-threshold`
/// at the threshold, in every notation an exact value is written in.
#[trace("TC-621")]
#[test]
fn a_threshold_of_zero_or_one_refuses_qualitative() {
    for (body, at) in [
        ("{ probability >= 1 [ holds(true) ] }", "1"),
        ("{ probability >= 0 [ holds(true) ] }", "0"),
        ("{ probability <= 1.0 [ holds(true) ] }", "1.0"),
        (
            "{ probability >= rational(3, 3) [ holds(true) ] }",
            "rational(3, 3)",
        ),
        (
            "{ probability >= decimal(10, 1) [ holds(true) ] }",
            "decimal(10, 1)",
        ),
        (
            "{ quantile 1 of (steps from holds(a) until holds(b) within 5) <= 3 }",
            "1",
        ),
        ("{ mean of (fraction holds(a) over [0, 5]) >= 0 }", "0"),
        ("{ long_run_fraction holds(a) >= 1 }", "1"),
    ] {
        let source = claim(body);
        assert_eq!(
            located(&service(None, &source)),
            [invalid("qualitative-threshold", at)],
            "{body}"
        );
    }
}

/// FR-186 "Confidence parameters": a clause that states one or two of the
/// three, or one twice, refuses `partial-confidence`; one that states none,
/// or all three once, is not refused for it.
#[trace("TC-621")]
#[test]
fn a_confidence_clause_states_all_three_parameters_once_or_none() {
    let lone = claim("with alpha 0.01 { probability >= 0.999 [ holds(true) ] }");
    assert_eq!(
        located(&service(None, &lone)),
        [
            invalid("partial-confidence", "alpha 0.01"),
            invalid("partial-confidence", "alpha 0.01"),
        ]
    );
    let repeated = claim(
        "with alpha 0.01, alpha 0.05, beta 0.01, indifference 0.0005 \
         { probability >= 0.999 [ holds(true) ] }",
    );
    assert_eq!(
        located(&service(None, &repeated)),
        [invalid("partial-confidence", "alpha")]
    );
    for ok in [
        "{ probability >= 0.999 [ holds(true) ] }",
        "with indifference 0.0005, beta 0.01, alpha 0.01 { probability >= 0.999 [ holds(true) ] }",
    ] {
        let source = claim(ok);
        assert_eq!(located(&service(None, &source)), [not_yet(&source)], "{ok}");
    }
}

/// FR-186 "Confidence parameters": a parameter that breaks its range, or an
/// indifference region outside `(0, 1)`, refuses `invalid-confidence`.
#[trace("TC-621")]
#[test]
fn confidence_parameters_keep_their_ranges() {
    for (clause, at) in [
        ("alpha 0.01, beta 0.01, indifference 0.002", "indifference"),
        ("alpha 0.01, beta 0.01, indifference 0", "indifference"),
        ("alpha 1, beta 0.01, indifference 0.0005", "alpha"),
        ("alpha 0.01, beta 0, indifference 0.0005", "beta"),
    ] {
        let source = claim(&format!(
            "with {clause} {{ probability >= 0.999 [ holds(true) ] }}"
        ));
        assert_eq!(
            located(&service(None, &source)),
            [invalid("invalid-confidence", at)],
            "{clause}"
        );
    }
}

/// FR-186 "Events and measures": a `quantile` over a `fraction` measure
/// refuses `quantile-over-fraction` at the measure.
#[trace("TC-621")]
#[test]
fn a_quantile_over_a_fraction_refuses() {
    let measure = "fraction holds(v.up) over [0, 10]";
    let source = claim(&format!(
        "{{ quantile 0.95 of ({measure}) <= rational(1, 2) }}"
    ));
    assert_eq!(
        located(&service(None, &source)),
        [invalid("quantile-over-fraction", measure)]
    );
}
