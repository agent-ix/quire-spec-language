// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-185 (TC-620): the S3 check of the `random`, `reward` and `workload`
//! declarations over ADR-024 §7.2's `Service` model, and FR-186 (TC-621):
//! the S3 typing of a `probabilistic` claim over it.
//!
//! `Service` is a domain package with one object type, `Server { phase:
//! Phase, attempts: Attempts }` (and an operation-free `Client { phase }`), `Phase` and `Outcome` variant types,
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
use qsl_semantics::check::CheckedLiteral;
use qsl_semantics::check::{
    Bound, BoundedMeasure, CheckCause, CheckRefusal, CheckedGraph, CheckingLimitKind, Confidence,
    Event, PackageDeclarations, PreconditionSite, ProbForm, ProbScheduler, ProbabilisticCause,
    ProbabilisticFeature, ReachTarget, WeightOwner,
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
    service_document_full(requires, Vec::new())
}

/// [`service_document_with`], with `extra` operations on `Server` after
/// `reset`.
fn service_document_full(requires: &[&str], extra: Vec<Value>) -> Vec<u8> {
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
    let operations: Vec<Value> = [
        operation("request", vec![], &["phase", "attempts"]),
        attempt,
        operation("reset", vec![], &["phase"]),
    ]
    .into_iter()
    .chain(extra)
    .collect();
    let server = definition(
        "Server",
        "object_type",
        json!({
            "supertypes": [],
            "fields": [
                wire_field(&id("Server/phase"), "phase", &id("Phase"), "required", 1),
                wire_field(&id("Server/attempts"), "attempts", &id("Attempts"), "required", 1),
            ],
            "operations": operations,
            "relationships": [],
        }),
    );
    let client = definition(
        "Client",
        "object_type",
        json!({
            "supertypes": [],
            "fields": [wire_field(&id("Client/phase"), "phase", &id("Phase"), "required", 1)],
            "operations": [],
            "relationships": [],
        }),
    );
    let clients = json!({
        "identity": id("clients"), "displayName": id("clients"),
        "kind": {"module": P, "name": "population"},
        "members": [id("Client")], "extent": "closed",
        "origin": generated("clients"),
    });
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
        client,
    ]);
    wire_envelope(constructs, types, json!([population, clients]))
        .to_string()
        .into_bytes()
}

/// The unit selecting `Service`, with `dimension Time` and `unit ms`, and
/// `body` after them; a `model` line of `body` is a second selection and
/// goes in the header.
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
    let (selections, body): (Vec<&str>, Vec<&str>) =
        body.lines().partition(|line| line.starts_with("model "));
    let packages = if selections.is_empty() {
        packages
    } else {
        package_input([document, other_document().as_slice()])
    };
    let (selections, body) = (selections.join("\n"), body.join("\n"));
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile ev = \"quire.temporal.event-position.false-extension/v1\";\n\
         profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         profile tm = \"quire.temporal.timed/v1\";\n\
         model Service = {P:?} version \"1.0.0\" digest \"sha256-jcs:{digest}\";\n\
         {selections}\n\
         dimension Time;\n\
         dimension Length;\n\
         unit ms : Time = rational(1, 1);\n\
         unit s : Time = rational(1000, 1) * ms;\n\
         unit m : Length = rational(1, 1);\n\
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

/// The limit refusal of an exact value past the 65,536-bit default
/// `s3.rational_bits` ceiling, at `at`.
fn limit(at: &str) -> (Code, Option<&'static str>, String) {
    (
        Code::StageLimitExceeded,
        Some("rational-bits-exceeded"),
        at.to_owned(),
    )
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
    let values: Vec<CheckedLiteral> = d.support().iter().map(|v| v.value.clone()).collect();
    assert_eq!(
        values,
        [1_i64, 3, 12]
            .map(|value| CheckedLiteral(quire_exact::Value::Integer(Integer::from(value))))
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

/// TC-620 step 9, the ill-typed clause only: an integer support value for a
/// random parameter typed as a reference to `Server` is refused
/// `value-outside-type`, naming the parameter, and the `Int[1, 12]` variant
/// checks. FR-185-AC-9's refused case is a reference *literal* naming a
/// `Server` key; S3 has no reference-literal form yet, so that clause is
/// untested (QSL-971).
#[trace("TC-620")]
#[test]
fn a_reference_valued_random_parameter_refuses_value_outside_type() {
    let document = service_document_full(
        &[],
        vec![operation(
            "assign",
            vec![parameter("assign", "target", &id("Server"))],
            &["phase"],
        )],
    );
    let steady = "workload Steady on Service { weight Server::request = 1; \
         weight Server::attempt = 1; weight Server::reset = 1; weight Server::assign = 1; }";
    let body = |random: &str| format!("{D}\n{OUTCOME}\n{DURATION}\n{steady}\n{random}\n");
    let refused = body("random Server::assign(target) ~ { 3: 1 };");
    let refusals = assemble_over(&document, &refused)
        .check(CheckingLimits::default())
        .expect_err("a reference-typed support is refused");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    assert_eq!(refusal.cause.cause(), Some("value-outside-type"));
    let CheckCause::Probabilistic(cause) = &refusal.cause else {
        panic!("a probabilistic refusal: {refusal:?}");
    };
    assert!(names_parameter(cause, "target"), "{cause:?}");

    // The `Int[1, 12]` parameter with the integer value `3` checks.
    assemble_over(&document, &body(""))
        .check(CheckingLimits::default())
        .expect("the integer-typed parameter checks");
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

/// FR-185 "Exact values": a configured `s3.rational_bits` replaces the
/// default ceiling, inclusive, and the limit refusal names that setting and
/// the configured limit.
#[trace("TC-620")]
#[test]
fn a_configured_rational_bits_ceiling_is_named_by_the_refusal() {
    let within = service(Some((STEADY, &steady_with_request("255"))), "");
    assemble(&within)
        .check(CheckingLimits::default().with_rational_bits(8))
        .expect("255 is eight bits");
    let past = service(Some((STEADY, &steady_with_request("256"))), "");
    let refusals = assemble(&past)
        .check(CheckingLimits::default().with_rational_bits(8))
        .expect_err("256 is nine bits");
    let CheckCause::ResourceExhausted(limit) = &refusals[0].cause else {
        panic!("a limit refusal: {:?}", refusals[0].cause);
    };
    assert_eq!(
        (limit.kind.setting().name(), limit.limit, limit.actual),
        ("s3.rational_bits", 8, 9)
    );
}

/// TC-620 step 2 (FR-185 "Exact values"), each as the weight of
/// `Server::request`: a value past 65,536 bits as written refuses before it
/// is reduced or expanded, with the limit refusal, even when it reduces to
/// 1; one within the ceiling is admitted and recorded reduced.
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
        assert_eq!(located(&body), [limit(&value)], "{value:.40}");
        let refusals = refusals_of(&body);
        let CheckCause::ResourceExhausted(limit) = &refusals[0].cause else {
            panic!("a limit refusal: {:?}", refusals[0].cause);
        };
        assert_eq!(limit.limit, 65_536);
        assert!(limit.actual > 65_536, "{value:.40}");
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
/// unknown unit; in a function body it types and checks (FR-093-AC-33).
#[trace("TC-620")]
#[test]
fn quantity_literals_type_against_the_declared_units() {
    assemble("function lag using v(): Boolean pure { 5 ms <= 3 ms }")
        .check(CheckingLimits::default())
        .expect("a quantity literal in a function body checks");
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

/// FR-185 "Exact values": a quantity literal's magnitude is bounded by the
/// configured `s3.rational_bits`, inclusive, in a function body.
#[trace("TC-620")]
#[test]
fn a_quantity_literal_is_bounded_by_the_configured_rational_bits() {
    let limits = CheckingLimits::default().with_rational_bits(8);
    assemble("function lag using v(): Boolean pure { 255 ms <= 3 ms }")
        .check(limits)
        .expect("eight bits is within the ceiling");
    let past = assemble("function lag using v(): Boolean pure { 256 ms <= 3 ms }")
        .check(limits)
        .expect_err("256 is nine bits");
    let CheckCause::ResourceExhausted(limit) = &past[0].cause else {
        panic!("a limit refusal: {:?}", past[0].cause);
    };
    assert_eq!(
        (limit.kind.setting().name(), limit.limit, limit.actual),
        ("s3.rational_bits", 8, 9)
    );
}

/// FR-185-AC-6 under a configured ceiling: weights `1/3` and `1/5` are each
/// within 3 bits, their sum `8/15` has 4, so the distribution is refused at
/// `d` naming `s3.rational_bits` and the configured limit.
#[trace("TC-620", "FR-185-AC-6")]
#[test]
fn a_distribution_total_is_bounded_by_the_configured_rational_bits() {
    let random = "random Server::attempt(d) ~ { 1: rational(1, 3), 3: rational(1, 5) };";
    let body = service(Some((D, random)), "");
    let limits = CheckingLimits::default().with_rational_bits(3);
    let refusals = assemble(&body)
        .check(limits)
        .expect_err("the sum is four bits");
    let CheckCause::ResourceExhausted(limit) = &refusals[0].cause else {
        panic!("a limit refusal: {:?}", refusals[0].cause);
    };
    assert_eq!(
        (limit.kind.setting().name(), limit.limit, limit.actual),
        ("s3.rational_bits", 3, 4)
    );
    assemble(&body)
        .check(CheckingLimits::default().with_rational_bits(7))
        .expect("the sum is within seven bits (the other declarations' 98)");
}
const OVER: &str = "over (s: Service::Server)";

fn claim(body: &str) -> String {
    format!("probabilistic C using ev on Service {OVER} under Steady on origin {body}")
}

/// `Service::Phase::Idle`, `Busy` and `Done` as the predicates `holds(..)`
/// reads.
fn phase(case: &str) -> String {
    format!("s.phase = Service::Phase::{case}")
}

/// A reward of a quantity type: `Service`'s `attempt` takes 1 ms. ADR-024
/// §7.2 types `d` as `Quantity<Time>`; this fixture cannot (see the module
/// doc), so the quantity reward is declared beside the integer `duration`.
const LATENCY: &str = "reward latency on Server::attempt = 1 ms;";

/// ADR-024 §7's four claims, over this fixture's `Service` model: its
/// `Health` and `Avail` subjects (`n.healthy`, `v.up`) are `Server` objects
/// whose `phase` is read, so the claims keep their forms, windows,
/// thresholds and confidences.
fn no_fault() -> String {
    format!(
        "probabilistic NoFault using ev on Service {OVER} under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.0005 \
         {{ probability >= 0.999 [ always[0,1000] holds({}) ] }}",
        phase("Idle")
    )
}

fn p95(threshold: &str) -> String {
    format!(
        "probabilistic P95 using ev on Service {OVER} under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         {{ quantile 0.95 of (accumulate latency from holds({}) until holds({}) within 10) \
         <= {threshold} }}",
        phase("Busy"),
        phase("Done")
    )
}

fn long_run() -> String {
    format!(
        "probabilistic LongRun using ev on Service {OVER} under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.0002 \
         {{ long_run_fraction holds({}) >= 0.999 }}",
        phase("Idle")
    )
}

fn monthly() -> String {
    format!(
        "probabilistic Monthly using ev on Service {OVER} under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.005 \
         {{ probability >= 0.99 [ fraction holds({}) over [0, 10000] >= rational(999, 1000) ] }}",
        phase("Idle")
    )
}

/// The four claims and the quantity reward `P95` accumulates.
fn adr_claims() -> String {
    service(
        None,
        &format!(
            "{LATENCY}\n{}\n{}\n{}\n{}",
            no_fault(),
            p95("5 ms"),
            long_run(),
            monthly()
        ),
    )
}

const IDLE: &str = "s.phase = Service::Phase::Idle";

fn confidence(alpha: &str, beta: &str, indifference: &str) -> Confidence {
    let value = |text: &str| -> Rational {
        let (numerator, denominator) = text.split_once('/').expect("a ratio");
        ratio(
            numerator.parse().expect("a numerator"),
            denominator.parse().expect("a denominator"),
        )
    };
    Confidence {
        alpha: value(alpha),
        beta: value(beta),
        indifference: value(indifference),
    }
}

/// FR-186-AC-1 (TC-621 step 1): ADR-024 §7's four claims check, each with
/// `Workload(Steady)`, the model subject `Service`, the event-position
/// profile and its stated confidence; the forms, bounds, windows and the
/// reward are read back.
#[trace("TC-621", "FR-186-AC-1")]
#[test]
fn the_adr_024_claims_check_and_record_their_forms() {
    let graph = checked(&adr_claims());
    let model = graph.probabilistic_model();
    let [no_fault, p95, long_run, monthly] = model.claims() else {
        panic!("four claims: {:?}", model.claims());
    };
    for (claim, name) in [
        (no_fault, "NoFault"),
        (p95, "P95"),
        (long_run, "LongRun"),
        (monthly, "Monthly"),
    ] {
        assert_eq!(claim.name, name);
        assert_eq!(
            model.claim(name).map(|found| &found.name),
            Some(&claim.name)
        );
        assert_eq!(claim.subject.alias, "Service");
        assert_eq!(claim.subject.package.identity, P);
        assert_eq!(claim.profile.authority(), "agent-ix");
        assert_eq!(
            claim.profile.identity(),
            "quire.temporal.event-position.false-extension/v1"
        );
        assert_eq!(
            claim.scheduler,
            ProbScheduler::Workload("Steady".to_owned())
        );
        assert!(
            claim.fairness.is_empty(),
            "{name}: no fairness under a workload"
        );
        let over = claim.over.as_ref().expect("the claim's over binding");
        assert_eq!(over.name, "s");
        assert!(matches!(over.value_type, ValueType::Reference(_)));
    }

    let ProbForm::Probability {
        bound,
        event: Event::Formula { formula, horizon },
    } = &no_fault.form
    else {
        panic!(
            "NoFault is a probability over a formula: {:?}",
            no_fault.form
        );
    };
    assert_eq!(*bound, Bound::AtLeast(ratio(999, 1000)));
    assert_eq!(*horizon, 1000);
    assert!(formula
        .formula()
        .nodes()
        .iter()
        .any(|node| matches!(node, qsl_semantics::check::FormulaNode::Unary { .. })));
    assert_eq!(
        no_fault.confidence,
        Some(confidence("1/100", "1/100", "1/2000"))
    );

    let ProbForm::Quantile {
        q,
        measure: BoundedMeasure::Accumulate { reward, within, .. },
        threshold,
    } = &p95.form
    else {
        panic!("P95 is a quantile of an accumulate: {:?}", p95.form);
    };
    assert_eq!(*q, ratio(19, 20));
    assert_eq!((reward.as_str(), *within), ("latency", 10));
    assert_eq!(*threshold, Bound::AtMost(ratio(5, 1)));
    assert_eq!(p95.confidence, Some(confidence("1/100", "1/100", "1/100")));

    let ProbForm::LongRunFraction {
        bound,
        weight,
        predicate,
    } = &long_run.form
    else {
        panic!("LongRun is a long-run fraction: {:?}", long_run.form);
    };
    assert_eq!(*bound, Bound::AtLeast(ratio(999, 1000)));
    assert!(weight.is_none());
    assert!(predicate.slots() >= 1, "the over parameter is bound");
    assert_eq!(
        long_run.confidence,
        Some(confidence("1/100", "1/100", "1/5000"))
    );

    let ProbForm::Probability {
        bound,
        event:
            Event::Compare {
                measure: BoundedMeasure::Fraction(fraction),
                bound: compared,
            },
    } = &monthly.form
    else {
        panic!(
            "Monthly is a probability over a compared fraction: {:?}",
            monthly.form
        );
    };
    assert_eq!(*bound, Bound::AtLeast(ratio(99, 100)));
    assert_eq!(fraction.window, 10_000);
    assert!(fraction.weight.is_none());
    assert_eq!(*compared, Bound::AtLeast(ratio(999, 1000)));
    assert_eq!(
        monthly.confidence,
        Some(confidence("1/100", "1/100", "1/200"))
    );
}

/// FR-186 "Outputs": a claim declared first in the source is the first
/// declaration S4 emission would refuse the package for
/// (`probabilistic-claim`), since FR-322's v2 vocabulary has no node for it.
#[trace("TC-621")]
#[test]
fn a_checked_claim_is_a_declaration_s4_would_refuse() {
    let first = format!("{LATENCY}\n{}\n{}", no_fault(), service(None, ""));
    let graph = checked(&first);
    let model = graph.probabilistic_model();
    assert!(!model.is_empty());
    assert_eq!(
        model.first_declaration(),
        Some((ProbabilisticFeature::Reward, "latency".to_owned()))
    );
    let claim_first = format!("{}\n{LATENCY}\n{}", no_fault(), service(None, ""));
    let graph = checked(&claim_first);
    assert_eq!(
        graph.probabilistic_model().first_declaration(),
        Some((ProbabilisticFeature::Claim, "NoFault".to_owned()))
    );
    assert_eq!(ProbabilisticFeature::Claim.name(), "probabilistic-claim");
}

/// The checked claim `C` of `body`.
fn claim_of(graph: &CheckedGraph) -> &qsl_semantics::check::CheckedProbabilisticClaim {
    graph
        .probabilistic_model()
        .claim("C")
        .expect("the claim C checks")
}

/// FR-186-AC-2 (TC-621 step 2): `P95` with the threshold `0.005 s` checks and
/// records `5 ms` (the reward's unit, converted exactly); with `5 m`, a length,
/// it is refused `ill_typed`/`unit-mismatch` naming both dimensions; an
/// integer reward takes a natural-number threshold and refuses a quantity.
#[trace("TC-621", "FR-186-AC-2")]
#[test]
fn a_threshold_converts_within_the_rewards_dimension_and_refuses_another() {
    let p95 = |threshold: &str| service(None, &format!("{LATENCY}\n{}", p95(threshold)));
    for (threshold, recorded) in [("0.005 s", 5), ("5 ms", 5), ("2 s", 2000)] {
        let graph = checked(&p95(threshold));
        let ProbForm::Quantile {
            threshold: bound, ..
        } = &graph.probabilistic_model().claim("P95").expect("P95").form
        else {
            panic!("a quantile");
        };
        assert_eq!(*bound, Bound::AtMost(ratio(recorded, 1)), "{threshold}");
    }
    let graph = checked(&p95("0.0005 s"));
    let ProbForm::Quantile { threshold, .. } =
        &graph.probabilistic_model().claim("P95").expect("P95").form
    else {
        panic!("a quantile");
    };
    assert_eq!(*threshold, Bound::AtMost(ratio(1, 2)), "a half millisecond");

    let mismatched = p95("5 m");
    assert_eq!(
        located(&mismatched),
        [(Code::IllTyped, Some("unit-mismatch"), "5 m".to_owned())]
    );
    let [ProbabilisticCause::UnitMismatch {
        claim: named,
        threshold,
        reward,
        ..
    }] = &causes(&mismatched)[..]
    else {
        panic!("a unit mismatch: {:?}", causes(&mismatched));
    };
    assert_eq!(
        (named.as_str(), threshold.as_str(), reward.as_str()),
        ("P95", "Length", "Time")
    );

    // An integer reward: a natural number, not a quantity.
    let measure = "accumulate duration from holds(s.phase = Service::Phase::Busy) \
                   until holds(s.phase = Service::Phase::Done) within 10";
    let graph = checked(&service(
        None,
        &claim(&format!("{{ quantile 0.95 of ({measure}) <= 5 }}")),
    ));
    let ProbForm::Quantile { threshold, .. } = &claim_of(&graph).form else {
        panic!("a quantile");
    };
    assert_eq!(*threshold, Bound::AtMost(ratio(5, 1)));
    let quantity = service(
        None,
        &claim(&format!("{{ quantile 0.95 of ({measure}) <= 5 ms }}")),
    );
    let [ProbabilisticCause::UnitMismatch {
        threshold, reward, ..
    }] = &causes(&quantity)[..]
    else {
        panic!("a unit mismatch: {:?}", causes(&quantity));
    };
    assert_eq!(
        (threshold.as_str(), reward.as_str()),
        ("Time", "dimensionless")
    );
    // A quantity reward with a bare number is ill typed, at the threshold.
    let bare = p95("5");
    assert_eq!(
        located(&bare),
        [(Code::IllTyped, Some("type-mismatch"), "5".to_owned())]
    );
}

/// FR-186 "Events and measures": a comparison's threshold has the type of its
/// measure: a natural number for `steps`, an exact rational in `(0, 1)` for
/// `fraction`, the reward's type for `accumulate`.
#[trace("TC-621", "FR-186-AC-5")]
#[test]
fn a_comparison_threshold_has_the_type_of_its_measure() {
    let steps = "steps from holds(s.phase = Service::Phase::Busy) \
                 until holds(s.phase = Service::Phase::Done) within 5";
    let graph = checked(&service(
        None,
        &claim(&format!("{{ probability >= 0.5 [ {steps} >= 3 ] }}")),
    ));
    let ProbForm::Probability {
        bound,
        event:
            Event::Compare {
                measure: BoundedMeasure::Steps { within, .. },
                bound: compared,
            },
    } = &claim_of(&graph).form
    else {
        panic!(
            "a probability over compared steps: {:?}",
            claim_of(&graph).form
        );
    };
    assert_eq!(*bound, Bound::AtLeast(ratio(1, 2)));
    assert_eq!((*within, compared), (5, &Bound::AtLeast(ratio(3, 1))));

    let fraction = format!("fraction holds({IDLE}) over [0, 10]");
    for threshold in [
        "rational(1, 1)",
        "rational(0, 1)",
        "rational(3, 2)",
        "rational(-1, 2)",
    ] {
        let body = service(
            None,
            &claim(&format!(
                "{{ probability >= 0.5 [ {fraction} >= {threshold} ] }}"
            )),
        );
        assert_eq!(
            located(&body),
            [invalid("fraction-bound-out-of-range", threshold)],
            "{threshold}"
        );
    }
    let graph = checked(&service(
        None,
        &claim(&format!(
            "{{ probability >= 0.5 [ {fraction} <= rational(1, 4) ] }}"
        )),
    ));
    let ProbForm::Probability {
        event: Event::Compare { bound, .. },
        ..
    } = &claim_of(&graph).form
    else {
        panic!("a compared fraction");
    };
    assert_eq!(*bound, Bound::AtMost(ratio(1, 4)));

    // A step count is no rational, and a duration is no step count.
    for threshold in ["rational(1, 2)", "5 ms"] {
        let body = service(
            None,
            &claim(&format!(
                "{{ probability >= 0.5 [ {steps} >= {threshold} ] }}"
            )),
        );
        let found = located(&body);
        assert_eq!(found.len(), 1, "{threshold}: {found:?}");
        assert_eq!(
            (found[0].0, found[0].1, found[0].2.as_str()),
            (Code::IllTyped, Some("type-mismatch"), threshold),
            "{threshold}"
        );
    }
}

/// FR-186 "Events and measures": `accumulate R` names a reward some operation
/// of the subject's model declares, and `weighted by R` names one; a name no
/// operation declares is `missing-name` at the name.
#[trace("TC-621", "FR-186-AC-6")]
#[test]
fn a_reward_a_claim_names_is_one_the_model_declares() {
    let accumulate = |reward: &str| {
        service(
            None,
            &claim(&format!(
                "{{ quantile 0.5 of (accumulate {reward} from holds({IDLE}) \
                 until holds({IDLE}) within 3) <= 4 }}"
            )),
        )
    };
    assert_eq!(
        located(&accumulate("nothing")),
        [(
            Code::MissingDeclaration,
            Some("missing-name"),
            "nothing".to_owned()
        )]
    );
    let graph = checked(&accumulate("duration"));
    let ProbForm::Quantile {
        measure: BoundedMeasure::Accumulate { reward, .. },
        ..
    } = &claim_of(&graph).form
    else {
        panic!("an accumulate");
    };
    assert_eq!(reward.as_str(), "duration");

    let weighted = |weight: &str| {
        service(
            None,
            &format!(
                "{LATENCY}\n{}",
                claim(&format!(
                    "{{ mean of (fraction holds({IDLE}) over [0, 10] weighted by {weight}) >= 0.5 }}"
                ))
            ),
        )
    };
    let graph = checked(&weighted("latency"));
    let ProbForm::MeanOfFraction { bound, fraction } = &claim_of(&graph).form else {
        panic!("a mean of a fraction");
    };
    assert_eq!(*bound, Bound::AtLeast(ratio(1, 2)));
    assert_eq!(fraction.window, 10);
    assert_eq!(
        fraction.weight.as_ref().map(|weight| weight.as_str()),
        Some("latency")
    );
    for unknown in ["nothing", "delay"] {
        assert_eq!(
            located(&weighted(unknown)),
            [(
                Code::MissingDeclaration,
                Some("missing-name"),
                unknown.to_owned()
            )],
            "{unknown}"
        );
    }
    let long_run = |weight: &str| {
        service(
            None,
            &format!(
                "{LATENCY}\n{}",
                claim(&format!(
                    "{{ long_run_fraction holds({IDLE}) weighted by {weight} >= 0.5 }}"
                ))
            ),
        )
    };
    let graph = checked(&long_run("latency"));
    let ProbForm::LongRunFraction { weight, .. } = &claim_of(&graph).form else {
        panic!("a long-run fraction");
    };
    assert_eq!(
        weight.as_ref().map(|weight| weight.as_str()),
        Some("latency")
    );
}

/// A reward name declared with two value types on two operations names no
/// one reward: `ambiguous_declaration`/`ambiguous-name` at the name.
#[trace("TC-621", "FR-186-AC-6")]
#[test]
fn a_reward_of_two_value_types_is_ambiguous() {
    let body = service(
        None,
        &format!(
            "{LATENCY}\nreward latency on Server::request = 2;\n{}",
            claim(&format!(
                "{{ quantile 0.5 of (accumulate latency from holds({IDLE}) \
                 until holds({IDLE}) within 3) <= 4 }}"
            ))
        ),
    );
    assert_eq!(
        located(&body),
        [(
            Code::AmbiguousDeclaration,
            Some("ambiguous-name"),
            "latency".to_owned()
        )]
    );
    // The same value type on both operations is one reward.
    let body = service(
        None,
        &format!(
            "{LATENCY}\nreward latency on Server::request = 2 ms;\n{}",
            claim(&format!(
                "{{ quantile 0.5 of (accumulate latency from holds({IDLE}) \
                 until holds({IDLE}) within 3) <= 4 ms }}"
            ))
        ),
    );
    checked(&body);
}

/// A claim's `over` type that resolves to no type refuses at the type, and
/// an accumulated measure compared in an event carries its reward, window and
/// bound.
#[trace("TC-621")]
#[test]
fn an_over_type_resolves_and_an_accumulate_event_compares() {
    let nowhere = service(
        None,
        "probabilistic C using ev on Service over (s: Service::Nowhere) under Steady on origin \
         { probability >= 0.5 [ holds(true) ] }",
    );
    let found = located(&nowhere);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, Some("missing-name"));
    assert!(found[0].2.contains("Nowhere"), "{found:?}");

    let accumulate =
        format!("accumulate duration from holds({IDLE}) until holds({IDLE}) within 4 <= 7");
    let graph = checked(&service(
        None,
        &claim(&format!("{{ probability >= 0.5 [ {accumulate} ] }}")),
    ));
    let ProbForm::Probability {
        event:
            Event::Compare {
                measure: BoundedMeasure::Accumulate { reward, within, .. },
                bound,
            },
        ..
    } = &claim_of(&graph).form
    else {
        panic!("a compared accumulate: {:?}", claim_of(&graph).form);
    };
    assert_eq!((reward.as_str(), *within), ("duration", 4));
    assert_eq!(*bound, Bound::AtMost(ratio(7, 1)));
}

/// FR-186-AC-3 (TC-621 step 3): an event with an operator that has no closed
/// interval is refused `unbounded-event`, located at the operator or the
/// interval; a nested window checks with the horizon the intervals sum to.
#[trace("TC-621", "FR-186-AC-3")]
#[test]
fn an_unbounded_event_is_refused_and_a_windowed_one_has_its_horizon() {
    let event = |formula: &str| {
        service(
            None,
            &claim(&format!("{{ probability >= 0.99 [ {formula} ] }}")),
        )
    };
    let holds = format!("holds({IDLE})");
    for (formula, at) in [
        (format!("always {holds}"), "always"),
        (format!("eventually[0,100] always {holds}"), "always"),
        (format!("eventually[0,*] {holds}"), "[0,*]"),
    ] {
        assert_eq!(
            located(&event(&formula)),
            [invalid("unbounded-event", at)],
            "{formula}"
        );
    }
    let graph = checked(&event(&format!("eventually[0,100] always[0,10] {holds}")));
    let ProbForm::Probability {
        event: Event::Formula { formula, horizon },
        ..
    } = &claim_of(&graph).form
    else {
        panic!("a formula event");
    };
    assert_eq!(*horizon, 110);
    let atom = formula
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
        .expect("one holds atom");
    assert!(
        formula.condition(atom).is_some(),
        "the atom's predicate is typed"
    );
    // A connective of bounded operators sums along the longest path.
    let graph = checked(&event(&format!(
        "eventually[0,7] {holds} and always[0,20] {holds}"
    )));
    let ProbForm::Probability {
        event: Event::Formula { horizon, .. },
        ..
    } = &claim_of(&graph).form
    else {
        panic!("a formula event");
    };
    assert_eq!(*horizon, 20);
    // `a > b` refuses under every profile.
    let inverted = event(&format!("eventually[5,2] {holds}"));
    assert_eq!(
        located(&inverted),
        [(Code::IllTyped, Some("type-mismatch"), "[5,2]".to_owned())]
    );
    // An unbounded profile admits no event window.
    let inf = service(
        None,
        &format!(
            "probabilistic C using inf on Service {OVER} under Steady on origin \
             {{ probability >= 0.99 [ eventually[0,5] {holds} ] }}"
        ),
    );
    let [ProbabilisticCause::UnboundedEvent { claim: named, .. }] = &causes(&inf)[..] else {
        panic!("an unbounded event: {:?}", causes(&inf));
    };
    assert_eq!(named, "C");
    // The timed profile has no event reading: it is refused for that alone.
    let timed = service(
        None,
        &format!(
            "probabilistic C using tm on Service {OVER} under Steady on origin \
             {{ probability >= 0.99 [ eventually[0,5] {holds} ] }}"
        ),
    );
    let [ProbabilisticCause::TimedEventProfile { claim: named, .. }] = &causes(&timed)[..] else {
        panic!("a timed event profile: {:?}", causes(&timed));
    };
    assert_eq!(named, "C");
}

/// FR-186 "Kind and subject": a claim over a name that is no `model` alias
/// but declares a type is a trace subject, refused `trace-subject`; a name
/// that declares nothing, an unknown profile alias, a profile that is no
/// temporal profile and an unknown workload are `missing-name` or
/// `not-temporal-profile`, each at the name.
#[trace("TC-621", "FR-186-AC-3", "FR-186-AC-6")]
#[test]
fn a_claim_names_a_model_a_temporal_profile_and_a_workload() {
    let trace = service(
        None,
        "record Reading { level: Integer; }\n\
         probabilistic C using ev on Reading under Steady on origin \
         { probability >= 0.9 [ holds(true) ] }",
    );
    assert_eq!(located(&trace), [invalid("trace-subject", "Reading")]);
    assert!(matches!(
        &causes(&trace)[..],
        [ProbabilisticCause::TraceSubject { claim: named, subject, .. }]
            if named == "C" && subject == "Reading"
    ));
    for (written, replacement, at) in [
        ("on Service", "on Nowhere", "Nowhere"),
        ("using ev", "using zz", "zz"),
        ("under Steady", "under Nope", "Nope"),
    ] {
        let source = claim("{ probability >= 0.9 [ holds(true) ] }").replace(written, replacement);
        assert_eq!(
            located(&service(None, &source)),
            [(
                Code::MissingDeclaration,
                Some("missing-name"),
                at.to_owned()
            )],
            "{replacement}"
        );
    }
    let not_temporal =
        claim("{ probability >= 0.9 [ holds(true) ] }").replace("using ev", "using v");
    assert_eq!(
        located(&service(None, &not_temporal)),
        [invalid("not-temporal-profile", "v")]
    );
}

/// FR-186-AC-3 (TC-621 step 3): a workload declared on another model than the
/// claim's subject is refused `foreign-workload` naming the workload and its
/// model; a workload of the subject's own model checks, and the checked claim
/// reads back its model.
#[trace("TC-621", "FR-186-AC-3")]
#[test]
fn a_workload_of_another_model_is_foreign() {
    let selection = other_model();
    let weights =
        "{ weight Server::request = 1; weight Server::attempt = 1; weight Server::reset = 1; }";
    let foreign_claim = |workload: &str| {
        format!(
            "probabilistic NoFault using ev on Other over (s: Other::Server) \
             under {workload} on origin \
             {{ probability >= 0.9 [ holds(s.phase = Other::Phase::Idle) ] }}"
        )
    };
    let body = |workload: &str| {
        let declarations = service(
            None,
            &format!(
                "workload Throttle on Service {weights}\nworkload Calm on Other {weights}\n{}",
                foreign_claim(workload)
            ),
        )
        .replace("random Server::", "random Service::Server::")
        .replace("on Server::attempt =", "on Service::Server::attempt =");
        format!("{selection}\n{declarations}")
    };
    let foreign = body("Throttle");
    assert_eq!(located(&foreign), [invalid("foreign-workload", "Throttle")]);
    let [ProbabilisticCause::ForeignWorkload {
        claim: named,
        workload,
        model,
        ..
    }] = &causes(&foreign)[..]
    else {
        panic!("a foreign workload: {:?}", causes(&foreign));
    };
    assert_eq!(
        (named.as_str(), workload.as_str(), model.as_str()),
        ("NoFault", "Throttle", "Service")
    );
    let graph = checked(&body("Calm"));
    let checked_claim = graph
        .probabilistic_model()
        .claim("NoFault")
        .expect("NoFault checks");
    assert_eq!(
        checked_claim.scheduler,
        ProbScheduler::Workload("Calm".to_owned())
    );
    assert_eq!(checked_claim.subject.alias, "Other");
    assert_eq!(checked_claim.subject.package.identity, OTHER);
}

/// FR-186-AC-3 (TC-621 step 3): `under every scheduler fair weak ...` checks
/// and carries its fairness set, each constraint with its kind, operation and
/// granularity; a repeated constraint joins the set once; an operation of no
/// model refuses `missing-name`.
#[trace("TC-621", "FR-186-AC-3", "FR-186-AC-6")]
#[test]
fn every_scheduler_carries_its_fairness_set() {
    let every = |fairness: &str| {
        service(
            None,
            &format!(
                "probabilistic C using ev on Service {OVER} \
                 under every scheduler {fairness} on origin \
                 {{ probability >= 0.9 [ holds({IDLE}) ] }}"
            ),
        )
    };
    let graph = checked(&every(
        "fair weak Server::attempt fair strong each Server::reset fair weak Server::attempt",
    ));
    let found = claim_of(&graph);
    assert_eq!(found.scheduler, ProbScheduler::Every);
    let set: Vec<(String, String, String)> = found
        .fairness
        .iter()
        .map(|constraint| {
            (
                format!("{:?}", constraint.kind()),
                constraint.operation().name().to_owned(),
                format!("{:?}", constraint.granularity()),
            )
        })
        .collect();
    assert_eq!(
        set,
        [
            ("Weak".to_owned(), "attempt".to_owned(), "Whole".to_owned()),
            ("Strong".to_owned(), "reset".to_owned(), "Each".to_owned()),
        ]
    );
    let bare = checked(&every(""));
    assert!(claim_of(&bare).fairness.is_empty());
    assert_eq!(claim_of(&bare).scheduler, ProbScheduler::Every);
    assert_eq!(
        located(&every("fair weak Server::nothing")),
        [(
            Code::MissingDeclaration,
            Some("missing-name"),
            "Server::nothing".to_owned()
        )]
    );
}

/// A claim's state predicate is a Boolean clause over its `over` parameter,
/// located at its own text: a name the claim does not bind, a predicate that
/// is no Boolean and a claim with no `over` all refuse there.
#[trace("TC-621")]
#[test]
fn a_predicate_is_a_boolean_clause_over_the_over_parameter() {
    let predicate = |text: &str| {
        service(
            None,
            &claim(&format!("{{ probability >= 0.9 [ holds({text}) ] }}")),
        )
    };
    let graph = checked(&predicate(IDLE));
    let ProbForm::Probability {
        event: Event::Formula { formula, .. },
        ..
    } = &claim_of(&graph).form
    else {
        panic!("a formula event");
    };
    let condition = formula
        .condition(0)
        .or_else(|| formula.condition(1))
        .expect("the atom");
    assert!(condition.slots() >= 1);
    let found = located(&predicate("t.phase = Service::Phase::Idle"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        (found[0].0, found[0].1),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    let found = located(&predicate("s.attempts"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        (found[0].0, found[0].2.as_str()),
        (Code::IllTyped, "s.attempts")
    );
    let no_over = service(
        None,
        &format!(
            "probabilistic C using ev on Service under Steady on origin \
             {{ probability >= 0.9 [ holds({IDLE}) ] }}"
        ),
    );
    let found = located(&no_over);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, Some("missing-name"));
}

/// The forms FR-195 owns, a timed window and an `on each` activation refuse
/// `unsupported_construct`/`not-yet-implemented` at the construct: no claim
/// compiles unchecked.
#[trace("TC-621")]
#[test]
fn what_s3_does_not_check_yet_refuses_not_yet_implemented() {
    let phase_a = format!("holds({IDLE})");
    for (body, construct, at) in [
        (
            format!("{{ expected elapsed until {phase_a} <= 3 }}"),
            "an `expected elapsed` claim",
            IDLE,
        ),
        (
            format!(
                "{{ probability >= 0.5 [ elapsed from {phase_a} until {phase_a} within 5 >= 3 ] }}"
            ),
            "an `elapsed` measure",
            "elapsed from holds(s.phase = Service::Phase::Idle) until holds(s.phase = Service::Phase::Idle) within 5",
        ),
        (
            format!(
                "{{ probability >= 0.5 [ steps from {phase_a} until {phase_a} within 5 ms >= 3 ] }}"
            ),
            "a timed window",
            "5 ms",
        ),
    ] {
        let source = claim(&body);
        let found = located(&service(None, &source));
        assert_eq!(found, [not_yet(at)], "{body}");
        let [ProbabilisticCause::NotYetImplemented { construct: named, .. }] =
            &causes(&service(None, &source))[..]
        else {
            panic!("not yet implemented: {body}");
        };
        assert_eq!(*named, construct);
    }
    // The activation, not the claim, is what is not checked.
    let each = format!(
        "probabilistic C using ev on Service {OVER} under Steady \
         on each (m: Service::Server) {{ probability >= 0.9 [ {phase_a} ] }}"
    );
    assert_eq!(located(&service(None, &each)), [not_yet("m")]);
}

/// A claim `C` `using` the profile `profile`, `on origin`, over `Service`'s
/// `Server`, under `scheduler` and with `confidence`.
fn exact_claim(profile: &str, scheduler: &str, confidence: &str, body: &str) -> String {
    service(
        None,
        &format!(
            "{LATENCY}\nprobabilistic C using {profile} on Service {OVER} under {scheduler} \
             on origin {confidence} {{ {body} }}"
        ),
    )
}

const CONFIDENT: &str = "with alpha 0.01, beta 0.01, indifference 0.002";

fn done() -> String {
    phase("Done")
}

/// Checks FR-195-AC-1 (TC-630 step 1): `probability >= 0.99 [eventually holds(..)]`
/// under the infinite-trace profile and every scheduler checks as
/// `Reach{AtLeast(99/100), Eventually}`, with `fair strong Server::attempt` it
/// records the fairness set, and the two have distinct normal forms; a
/// `holds(A) until holds(B)` event and an `always holds(P)` event check as
/// `Until` and `Always` under a workload.
#[trace("TC-630", "FR-195-AC-1")]
#[test]
fn an_unbounded_event_under_the_infinite_profile_is_a_reach_form() {
    let terminates = checked(&exact_claim(
        "inf",
        "every scheduler",
        "",
        &format!("probability >= 0.99 [ eventually holds({IDLE}) ]"),
    ));
    let terminates = claim_of(&terminates);
    let ProbForm::Reach { bound, target } = &terminates.form else {
        panic!("a reach form: {:?}", terminates.form);
    };
    assert_eq!(*bound, Bound::AtLeast(ratio(99, 100)));
    assert!(matches!(target, ReachTarget::Eventually(_)));
    assert_eq!(terminates.scheduler, ProbScheduler::Every);
    assert!(terminates.fairness.is_empty());

    let fair = checked(&exact_claim(
        "inf",
        "every scheduler fair strong Server::attempt",
        "",
        &format!("probability >= 0.99 [ eventually holds({IDLE}) ]"),
    ));
    let fair = claim_of(&fair);
    assert!(matches!(
        fair.form,
        ProbForm::Reach {
            target: ReachTarget::Eventually(_),
            ..
        }
    ));
    let [constraint] = &fair.fairness[..] else {
        panic!("one fairness constraint: {:?}", fair.fairness);
    };
    assert_eq!(
        (
            format!("{:?}", constraint.kind()),
            constraint.operation().name()
        ),
        ("Strong".to_owned(), "attempt")
    );
    assert_ne!(terminates.normal_form, fair.normal_form);

    let until = checked(&exact_claim(
        "inf",
        "Steady",
        "",
        &format!(
            "probability >= 0.95 [ holds(not ({})) until holds({}) ]",
            done(),
            done()
        ),
    ));
    let until = claim_of(&until);
    let ProbForm::Reach { bound, target } = &until.form else {
        panic!("a reach form: {:?}", until.form);
    };
    assert_eq!(*bound, Bound::AtLeast(ratio(19, 20)));
    assert!(matches!(target, ReachTarget::Until(_, _)));

    let always = checked(&exact_claim(
        "inf",
        "Steady",
        "",
        &format!("probability <= 0.5 [ always holds({IDLE}) ]"),
    ));
    let always = claim_of(&always);
    let ProbForm::Reach { bound, target } = &always.form else {
        panic!("a reach form: {:?}", always.form);
    };
    assert_eq!(*bound, Bound::AtMost(ratio(1, 2)));
    assert!(matches!(target, ReachTarget::Always(_)));
}

/// Checks FR-195-AC-1 (TC-630 step 1): a temporal operator, or a constant, where a
/// reachability event needs a `holds(P)` state predicate is refused
/// `not-state-predicate` at the operand; other events under the
/// infinite-trace profile stay unbounded, and the same `eventually` under a
/// bounded profile is still an unbounded event.
#[trace("TC-630", "FR-195-AC-1")]
#[test]
fn a_reach_operand_must_be_a_state_predicate() {
    let holds = format!("holds({IDLE})");
    let refused = |profile: &str, formula: &str| {
        located(&exact_claim(
            profile,
            "Steady",
            "",
            &format!("probability >= 0.9 [ {formula} ]"),
        ))
    };
    for (formula, at) in [
        (
            format!("eventually eventually {holds}"),
            format!("eventually {holds}"),
        ),
        (
            format!("{holds} until eventually {holds}"),
            format!("eventually {holds}"),
        ),
        (format!("always not {holds}"), format!("not {holds}")),
        ("eventually true".to_owned(), "true".to_owned()),
    ] {
        assert_eq!(
            refused("inf", &formula),
            [invalid("not-state-predicate", &at)],
            "{formula}"
        );
    }
    assert_eq!(
        refused("inf", &format!("eventually[0,5] {holds}")),
        [invalid(
            "unbounded-event",
            &format!("eventually[0,5] {holds}")
        )]
    );
    assert_eq!(
        refused("inf", &format!("{holds} and eventually {holds}")),
        [
            invalid(
                "unbounded-event",
                &format!("{holds} and eventually {holds}")
            ),
            invalid("unbounded-event", "eventually"),
        ]
    );
    assert_eq!(
        refused("ev", &format!("eventually {holds}")),
        [invalid("unbounded-event", "eventually")]
    );
}

/// Checks FR-195-AC-2 (TC-630 step 2): `expected accumulate R until holds(B) <= c`
/// checks as `ExpectedReward` with a threshold that is a rational for an
/// integer reward and a quantity in the reward's unit for a quantity reward;
/// a quantity of another dimension, a quantity for an integer reward and an
/// unknown reward are refused, and `expected elapsed` is not checked (no
/// subject is timed).
#[trace("TC-630", "FR-195-AC-2")]
#[test]
fn an_expected_reward_claim_is_typed_by_its_rewards_unit() {
    let expected = |body: &str| exact_claim("ev", "Steady", "", body);
    let target = format!("holds({IDLE} or {})", done());
    let graph = checked(&expected(&format!(
        "expected accumulate duration until {target} <= rational(6, 5)"
    )));
    let claim = claim_of(&graph);
    let ProbForm::ExpectedReward {
        bound,
        reward,
        until,
    } = &claim.form
    else {
        panic!("an expected reward: {:?}", claim.form);
    };
    assert_eq!(*bound, Bound::AtMost(ratio(6, 5)));
    assert_eq!(reward.as_str(), "duration");
    assert!(until.slots() >= 1, "the over parameter is bound");
    assert!(claim.confidence.is_none());

    let whole = checked(&expected(&format!(
        "expected accumulate duration until {target} >= 3"
    )));
    assert!(matches!(
        claim_of(&whole).form,
        ProbForm::ExpectedReward {
            bound: Bound::AtLeast(ref found),
            ..
        } if *found == ratio(3, 1)
    ));

    let quantity = checked(&expected(&format!(
        "expected accumulate latency until {target} <= 0.005 s"
    )));
    assert!(matches!(
        claim_of(&quantity).form,
        ProbForm::ExpectedReward {
            bound: Bound::AtMost(ref found),
            ..
        } if *found == ratio(5, 1)
    ));

    let at = IDLE.to_owned() + " or " + &done();
    assert_eq!(
        located(&expected(&format!(
            "expected accumulate latency until {target} <= 5 m"
        ))),
        [(Code::IllTyped, Some("unit-mismatch"), "5 m".to_owned())]
    );
    assert_eq!(
        located(&expected(&format!(
            "expected accumulate duration until {target} <= 5 ms"
        ))),
        [(Code::IllTyped, Some("unit-mismatch"), "5 ms".to_owned())]
    );
    assert_eq!(
        located(&expected(&format!(
            "expected accumulate nothing until {target} <= 3"
        ))),
        [(
            Code::MissingDeclaration,
            Some("missing-name"),
            "nothing".to_owned()
        )]
    );
    assert_eq!(
        located(&expected(&format!(
            "expected elapsed until {target} <= 3 ms"
        ))),
        [not_yet(&at)]
    );
}

/// Checks FR-195-AC-3 (TC-630 step 3): `exact_only` is true for a reachability form,
/// an expected reward, a claim under every scheduler and a claim with no
/// confidence parameters, each alone, and false for a bounded event under a
/// workload with its confidence parameters.
#[trace("TC-630", "FR-195-AC-3")]
#[test]
fn exact_only_marks_the_claims_only_exact_evidence_decides() {
    let bounded = format!("probability >= 0.99 [ always[0,10] holds({IDLE}) ]");
    let reach = format!("probability >= 0.99 [ eventually holds({IDLE}) ]");
    let expected = format!("expected accumulate duration until holds({IDLE}) <= 3");
    for (case, source, only) in [
        (
            "P95 with its confidence under a workload",
            service(None, &format!("{LATENCY}\n{}", p95("5 ms"))),
            false,
        ),
        (
            "P95 with no confidence",
            service(
                None,
                &format!(
                    "{LATENCY}\n{}",
                    p95("5 ms")
                        .replace(", indifference 0.01", "")
                        .replace("with alpha 0.01, beta 0.01", "")
                ),
            ),
            true,
        ),
        (
            "a bounded event under every scheduler",
            exact_claim("ev", "every scheduler", CONFIDENT, &bounded),
            true,
        ),
        (
            "a bounded event with no confidence",
            exact_claim("ev", "Steady", "", &bounded),
            true,
        ),
        (
            "a reach form alone",
            exact_claim("inf", "Steady", CONFIDENT, &reach),
            true,
        ),
        (
            "an expected reward alone",
            exact_claim("ev", "Steady", CONFIDENT, &expected),
            true,
        ),
    ] {
        let graph = checked(&source);
        let found = graph
            .probabilistic_model()
            .claims()
            .iter()
            .find(|claim| claim.name == if case.starts_with("P95") { "P95" } else { "C" })
            .unwrap_or_else(|| panic!("{case}: the claim checks"));
        assert_eq!(found.exact_only(), only, "{case}");
    }
}

/// Checks FR-195-AC-1, AC-2 and FR-186 "Normal form": the obligation identity of an
/// exact-only claim binds its form: `eventually`, `until` and `always` of
/// one predicate, a different predicate, and two expected rewards that differ
/// only in the reward are six different claims.
#[trace("TC-630", "FR-195-AC-1")]
#[test]
fn exact_only_forms_have_distinct_normal_forms() {
    let idle = format!("holds({IDLE})");
    let done = format!("holds({})", done());
    let claim = |name: &str, body: &str| {
        format!(
            "probabilistic {name} using inf on Service {OVER} under Steady on origin \
             {{ {body} }}"
        )
    };
    let source = service(
        None,
        &[
            LATENCY.to_owned(),
            claim(
                "Eventually",
                &format!("probability >= 0.9 [ eventually {idle} ]"),
            ),
            claim("Always", &format!("probability >= 0.9 [ always {idle} ]")),
            claim(
                "Until",
                &format!("probability >= 0.9 [ {idle} until {done} ]"),
            ),
            claim(
                "Other",
                &format!("probability >= 0.9 [ eventually {done} ]"),
            ),
            claim(
                "Cost",
                &format!("expected accumulate duration until {idle} <= 3"),
            ),
            claim(
                "Latency",
                &format!("expected accumulate latency until {idle} <= 3 ms"),
            ),
        ]
        .join("\n"),
    );
    let graph = checked(&source);
    let forms: Vec<_> = ["Eventually", "Always", "Until", "Other", "Cost", "Latency"]
        .iter()
        .map(|name| {
            graph
                .probabilistic_model()
                .claim(name)
                .unwrap_or_else(|| panic!("{name} checks"))
                .normal_form
                .clone()
        })
        .collect();
    for (at, form) in forms.iter().enumerate() {
        for other in &forms[at + 1..] {
            assert_ne!(form, other);
        }
    }
}

/// FR-186-AC-4 (TC-621 step 4): the claim's normal form binds the subject,
/// the profile, the form, the scheduler, its fairness set, the `over` type and
/// the stated confidence, and nothing else: claims that differ in any of those
/// have distinct normal forms, and claims that differ only in their name, the
/// spacing, the commutation of an equality, the `over` binder's name or the
/// order a workload's weights are written in have equal ones.
#[trace("TC-621", "FR-186-AC-4")]
#[test]
fn the_normal_form_binds_what_the_claim_asserts() {
    const BURST: &str = "workload Burst on Service { weight Server::request = 3; \
                         weight Server::attempt = 1; weight Server::reset = 1; }";
    const BURST_REORDERED: &str = "workload Burst on Service { weight Server::reset = 1; \
                         weight Server::attempt = 1; weight Server::request = 3; }";
    let source = |name: &str, header: &str, scheduler: &str, confidence: &str, body: &str| {
        service(
            None,
            &format!(
                "{BURST}\nprobabilistic {name} using {header} on Service {OVER} \
                 under {scheduler} on origin {confidence} {{ {body} }}"
            ),
        )
    };
    const WITH: &str = "with alpha 0.01, beta 0.01, indifference 0.0005";
    let event = format!("probability >= 0.999 [ always[0,1000] holds({IDLE}) ]");
    let form = |header: &str, scheduler: &str, confidence: &str, body: &str| {
        let graph = checked(&source("C", header, scheduler, confidence, body));
        claim_of(&graph).normal_form.as_bytes().to_vec()
    };
    let base = form("ev", "Steady", WITH, &event);
    let variants = [
        (
            "alpha 0.05",
            form(
                "ev",
                "Steady",
                "with alpha 0.05, beta 0.01, indifference 0.0005",
                &event,
            ),
        ),
        (
            "beta",
            form(
                "ev",
                "Steady",
                "with alpha 0.01, beta 0.02, indifference 0.0005",
                &event,
            ),
        ),
        (
            "indifference",
            form(
                "ev",
                "Steady",
                "with alpha 0.01, beta 0.01, indifference 0.0004",
                &event,
            ),
        ),
        ("no confidence", form("ev", "Steady", "", &event)),
        ("workload Burst", form("ev", "Burst", WITH, &event)),
        (
            "every scheduler",
            form("ev", "every scheduler", WITH, &event),
        ),
        (
            "weak fairness",
            form(
                "ev",
                "every scheduler fair weak Server::attempt",
                WITH,
                &event,
            ),
        ),
        (
            "strong fairness",
            form(
                "ev",
                "every scheduler fair strong Server::attempt",
                WITH,
                &event,
            ),
        ),
        (
            "each-granular fairness",
            form(
                "ev",
                "every scheduler fair weak each Server::attempt",
                WITH,
                &event,
            ),
        ),
        (
            "weaker threshold",
            form("ev", "Steady", WITH, &event.replace("0.999", "0.99")),
        ),
        (
            "at most",
            form("ev", "Steady", WITH, &event.replace(">=", "<=")),
        ),
        (
            "window",
            form("ev", "Steady", WITH, &event.replace("1000", "999")),
        ),
        (
            "predicate",
            form("ev", "Steady", WITH, &event.replace("Idle", "Busy")),
        ),
        (
            "another form",
            form(
                "ev",
                "Steady",
                WITH,
                &format!("long_run_fraction holds({IDLE}) >= 0.999"),
            ),
        ),
    ];
    for (name, found) in &variants {
        assert_ne!(*found, base, "{name} is another claim");
    }
    for (index, (first, one)) in variants.iter().enumerate() {
        for (second, other) in &variants[index + 1..] {
            assert_ne!(one, other, "{first} and {second} are distinct claims");
        }
    }
    // The profile is part of the normal form: the same long-run claim under
    // the event-position and the infinite-trace profile.
    let long_run = format!("long_run_fraction holds({IDLE}) >= 0.999");
    assert_ne!(
        form("ev", "Steady", WITH, &long_run),
        form("inf", "Steady", WITH, &long_run)
    );
    // The name, the spacing and a commuted equality bind nothing.
    let renamed = checked(&source(
        "Renamed",
        "ev",
        "Steady",
        WITH,
        "probability   >=  0.999 [ always[0,1000]   holds(Service::Phase::Idle = s.phase) ]",
    ));
    assert_eq!(
        renamed
            .probabilistic_model()
            .claim("Renamed")
            .expect("Renamed")
            .normal_form
            .as_bytes(),
        &base[..]
    );
    // A commuted equality in a predicate that is no formula atom: the
    // predicate's own normal form is what is bound.
    let commuted = |predicate: &str| {
        let graph = checked(&service(
            None,
            &claim(&format!(
                "{{ long_run_fraction holds({predicate}) >= 0.999 }}"
            )),
        ));
        claim_of(&graph).normal_form.as_bytes().to_vec()
    };
    assert_eq!(commuted(IDLE), commuted("Service::Phase::Idle = s.phase"));
    assert_ne!(commuted(IDLE), commuted("s.phase = Service::Phase::Busy"));
    // The `over` binder's name is not content.
    let renamed_binder = checked(&service(
        None,
        "probabilistic C using ev on Service over (t: Service::Server) \
         under Steady on origin { long_run_fraction holds(t.phase = Service::Phase::Idle) \
         >= 0.999 }",
    ));
    assert_eq!(
        claim_of(&renamed_binder).normal_form.as_bytes(),
        commuted(IDLE).as_slice()
    );
    // The order a workload's weights are written in is not content.
    let burst = |declaration: &str| {
        let graph = checked(&service(
            None,
            &format!(
                "{declaration}\nprobabilistic C using ev on Service {OVER} under Burst \
                 on origin {{ long_run_fraction holds({IDLE}) >= 0.999 }}"
            ),
        ));
        claim_of(&graph).normal_form.as_bytes().to_vec()
    };
    assert_eq!(burst(BURST), burst(BURST_REORDERED));
    assert_ne!(
        burst(BURST),
        burst(&BURST.replace("request = 3", "request = 4"))
    );
    // Only the subject differs, or only the `over` binding.
    let subject = |model: &str| {
        let selection = other_model();
        let declarations = service(
            None,
            &format!(
                "probabilistic C using ev on {model} under every scheduler on origin \
                 {{ probability >= 0.9 [ holds(true) ] }}"
            ),
        )
        .replace("random Server::", "random Service::Server::")
        .replace("on Server::attempt =", "on Service::Server::attempt =");
        let graph = checked(&format!("{selection}\n{declarations}"));
        claim_of(&graph).normal_form.as_bytes().to_vec()
    };
    assert_ne!(subject("Service"), subject("Other"));
    let over = |binding: &str| {
        let graph = checked(&service(
            None,
            &format!(
                "probabilistic C using ev on Service {binding} under Steady on origin \
                 {{ probability >= 0.9 [ holds(true) ] }}"
            ),
        ));
        claim_of(&graph).normal_form.as_bytes().to_vec()
    };
    assert_ne!(over(OVER), over(""));
}

/// FR-186-AC-4 (TC-621 step 4): a fairness set written in another order, a
/// reward declared on its operations in another order and two declarations of
/// equal content each keep one normal form; a claim that differs only in its
/// `over` type has another.
#[trace("TC-621", "FR-186-AC-4")]
#[test]
fn the_normal_form_ignores_declaration_order_and_binds_the_over_type() {
    let normal_form = |extra: &str, body: &str| {
        let graph = checked(&service(None, &format!("{extra}\n{body}")));
        claim_of(&graph).normal_form.as_bytes().to_vec()
    };
    let fair = |fairness: &str| {
        normal_form(
            "",
            &format!(
                "probabilistic C using ev on Service {OVER} under every scheduler {fairness} \
                 on origin {{ probability >= 0.9 [ holds({IDLE}) ] }}"
            ),
        )
    };
    let forward = fair("fair weak Server::attempt fair strong each Server::reset");
    assert_eq!(
        forward,
        fair("fair strong each Server::reset fair weak Server::attempt")
    );
    assert_ne!(forward, fair("fair weak Server::attempt"));

    let accumulate = claim(&format!(
        "{{ quantile 0.5 of (accumulate latency from holds({IDLE}) until holds({IDLE}) \
         within 3) <= 4 ms }}"
    ));
    let rewards =
        |first: &str, second: &str| normal_form(&format!("{first}\n{second}"), &accumulate);
    let on_attempt = "reward latency on Server::attempt = 1 ms;";
    let on_request = "reward latency on Server::request = 2 ms;";
    assert_eq!(
        rewards(on_attempt, on_request),
        rewards(on_request, on_attempt)
    );
    assert_ne!(
        rewards(on_attempt, on_request),
        rewards(on_attempt, "reward other on Server::reset = 1;")
    );

    // Two declarations of equal content, in one unit.
    let graph = checked(&service(
        None,
        &format!(
            "{}\n{}",
            claim("{ probability >= 0.9 [ holds(true) ] }"),
            claim("{ probability >= 0.9 [ holds(true) ] }")
                .replace("probabilistic C", "probabilistic D"),
        ),
    ));
    let model = graph.probabilistic_model();
    assert_eq!(
        model.claim("C").expect("C").normal_form,
        model.claim("D").expect("D").normal_form
    );

    // Only the `over` type differs: `Server` and `Client` are two object
    // types of the one model.
    let over = |object: &str| {
        normal_form(
            "",
            &format!(
                "probabilistic C using ev on Service over (s: Service::{object}) \
                 under Steady on origin {{ probability >= 0.9 [ holds(true) ] }}"
            ),
        )
    };
    assert_ne!(over("Server"), over("Client"));
}

/// FR-186 "Events and measures": `accumulate R` names a reward of the
/// subject's own model: a reward only another model declares is
/// `missing-name`.
#[trace("TC-621")]
#[test]
fn a_reward_declared_only_by_another_model_is_not_the_subjects() {
    let selection = other_model();
    let weights =
        "{ weight Server::request = 1; weight Server::attempt = 1; weight Server::reset = 1; }";
    let declarations = service(
        None,
        &format!(
            "workload Calm on Other {weights}\n\
             probabilistic C using ev on Other over (s: Other::Server) under Calm on origin \
             {{ quantile 0.5 of (accumulate duration from holds(s.phase = Other::Phase::Idle) \
             until holds(s.phase = Other::Phase::Idle) within 3) <= 4 }}"
        ),
    )
    .replace("random Server::", "random Service::Server::")
    .replace("on Server::attempt =", "on Service::Server::attempt =");
    assert_eq!(
        located(&format!("{selection}\n{declarations}")),
        [(
            Code::MissingDeclaration,
            Some("missing-name"),
            "duration".to_owned()
        )]
    );
}

/// An event whose interval bounds sum past `u64` is refused with the limit
/// refusal, located at the operator whose interval overflowed the sum.
#[trace("TC-621", "FR-186-AC-5")]
#[test]
fn an_event_horizon_past_u64_is_refused_at_its_operator() {
    let huge = "9223372036854775808";
    let body = service(
        None,
        &claim(&format!(
            "{{ probability >= 0.9 [ eventually[0,{huge}] eventually[0,{huge}] holds({IDLE}) ] }}"
        )),
    );
    let found = located(&body);
    assert_eq!(
        found,
        [(
            Code::StageLimitExceeded,
            Some("work-budget-exceeded"),
            "eventually".to_owned()
        )]
    );
}

/// An `over` parameter must be of an object type of the model; a threshold
/// that is not a literal is refused as such.
#[trace("TC-621", "FR-186-AC-5")]
#[test]
fn an_over_type_and_a_threshold_have_their_required_shape() {
    let scalar = service(
        None,
        "probabilistic C using ev on Service over (n: Integer) under Steady on origin \
         { probability >= 0.5 [ holds(true) ] }",
    );
    assert_eq!(located(&scalar), [invalid("over-not-object-type", "n")]);
    let steps = "steps from holds(s.phase = Service::Phase::Busy) \
                 until holds(s.phase = Service::Phase::Done) within 5";
    let body = service(
        None,
        &claim(&format!("{{ probability >= 0.5 [ {steps} >= 1 + 2 ] }}")),
    );
    assert_eq!(
        located(&body),
        [(Code::IllTyped, Some("type-mismatch"), "1 + 2".to_owned())]
    );
    assert!(matches!(
        &causes(&body)[..],
        [ProbabilisticCause::ThresholdNotLiteral { .. }]
    ));
}

/// FR-186 "Thresholds": a probability threshold, quantile level, mean
/// threshold or long-run threshold of 0 or 1 refuses `qualitative-threshold`
/// at the threshold, in every notation an exact value is written in. That is
/// FR-186-AC-2's `probability >= 1 [E]` and `probability >= 0 [E]`.
#[trace("TC-621", "FR-186-AC-2")]
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
/// or all three once, is not refused for it (FR-186-AC-4).
#[trace("TC-621", "FR-186-AC-4")]
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
    let none = checked(&service(
        None,
        &claim("{ probability >= 0.999 [ holds(true) ] }"),
    ));
    assert_eq!(none.probabilistic_model().claims()[0].confidence, None);
    let reordered = checked(&service(
        None,
        &claim(
            "with indifference 0.0005, beta 0.01, alpha 0.01 { probability >= 0.999 [ holds(true) ] }",
        ),
    ));
    assert_eq!(
        reordered.probabilistic_model().claims()[0].confidence,
        Some(confidence("1/100", "1/100", "1/2000"))
    );
}

/// FR-186 "Confidence parameters": a parameter that breaks its range, or an
/// indifference region outside `(0, 1)`, refuses `invalid-confidence`
/// (FR-186-AC-4: `indifference 0.002` makes `theta + iota` 1.001).
#[trace("TC-621", "FR-186-AC-4")]
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
/// refuses `quantile-over-fraction` at the measure (FR-186-AC-2).
#[trace("TC-621", "FR-186-AC-2")]
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

/// A second distribution or reward is a duplicate even when the first was
/// refused.
#[trace("TC-620")]
#[test]
fn a_duplicate_of_a_refused_declaration_is_still_a_duplicate() {
    let first = "random Server::attempt(d) ~ { 1: 0 };";
    let second = "random Server::attempt(d) ~ { 3: 1 };";
    let body = service(Some((D, first)), second);
    assert_eq!(
        located(&body),
        [
            invalid("invalid-weight", "0"),
            invalid("duplicate-distribution", second)
        ]
    );
    let second = "reward duration on Server::attempt = true;";
    let body = service(
        Some((DURATION, "reward duration on Server::attempt = outcome;")),
        second,
    );
    assert_eq!(
        located(&body),
        [
            invalid("reward-type", "outcome"),
            invalid("duplicate-reward", second)
        ]
    );
}

/// FR-185 "Exact values" applied to S3's own arithmetic: weights each within
/// the ceiling whose denominators are large and coprime would grow the
/// weight sum without bound, so the sum is bounded by the same ceiling and
/// the declaration refuses promptly with the limit refusal.
#[trace("TC-620")]
#[test]
fn a_weight_sum_past_the_ceiling_refuses_promptly() {
    let weights: Vec<String> = (0..12_u64)
        .map(|k| {
            let denominator = Integer::one()
                .shifted_left(65_534)
                .add(&Integer::from(2 * k + 1));
            format!("rational(1, {denominator})")
        })
        .collect();
    let pairs: Vec<String> = weights
        .iter()
        .enumerate()
        .map(|(value, weight)| format!("{}: {weight}", value + 1))
        .collect();
    let random = format!("random Server::attempt(d) ~ {{ {} }};", pairs.join(", "));
    let started = std::time::Instant::now();
    let refusals = refusals_of(&service(Some((D, &random)), ""));
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
        panic!("a limit refusal: {:?}", refusal.cause);
    };
    assert_eq!(limit.limit, 65_536);
    // A wide margin over the one or two bounded additions it takes: the
    // unbounded sum took minutes.
    assert!(started.elapsed() < std::time::Duration::from_secs(60));
}

/// FR-185 "Workloads": an operation weighted twice refuses `duplicate-weight`
/// even when its first weight is invalid.
#[trace("TC-620")]
#[test]
fn a_second_weight_after_an_invalid_one_is_still_a_duplicate() {
    let workload = "workload Steady on Service { weight Server::request = 0; \
         weight Server::attempt = 1; weight Server::reset = 1; weight Server::request = 2; }";
    assert_eq!(
        located(&service(Some((STEADY, workload)), "")),
        [
            invalid("invalid-weight", "0"),
            invalid("duplicate-weight", "weight Server::request = 2;")
        ]
    );
}

/// FR-185 "Rewards": a reward that reads state still declares its name, so a
/// later reward of that name on that operation is a duplicate.
#[trace("TC-620")]
#[test]
fn a_reward_after_one_that_reads_state_is_still_a_duplicate() {
    let second = "reward duration on Server::attempt = d;";
    let body = service(
        Some((
            DURATION,
            "reward duration on Server::attempt = self.attempts;",
        )),
        second,
    );
    assert_eq!(
        located(&body),
        [
            invalid("state-read-in-reward", "self"),
            invalid("duplicate-reward", second)
        ]
    );
}

/// FR-185 "Random parameters": a precondition read of a random parameter
/// refuses even when the parameter's distribution was refused.
#[trace("TC-620")]
#[test]
fn a_precondition_read_of_a_refused_distribution_still_refuses() {
    let pre = "pre Busy using v on Service::Server::attempt \
               { self.phase = Service::Phase::Busy and d < 5 }";
    let body = service(Some((D, "random Server::attempt(d) ~ { 1: 0 };")), pre);
    assert_eq!(
        located(&body),
        [
            invalid("invalid-weight", "0"),
            invalid("random-read-in-precondition", "d")
        ]
    );
}

/// FR-186-AC-4 (TC-621 step 4): `alpha decimal(1, 4294967295)` is read
/// without expansion, measured as written and refused with the limit refusal
/// at the value (QSL-883 pending, as FR-185 states), whatever its range.
#[trace("TC-621", "FR-186-AC-4")]
#[test]
fn a_huge_written_confidence_value_is_refused_with_the_limit_refusal() {
    let source = claim(
        "with alpha decimal(1, 4294967295), beta 0.01, indifference 0.0005 \
         { probability >= 0.999 [ holds(true) ] }",
    );
    assert_eq!(
        located(&service(None, &source)),
        [limit("decimal(1, 4294967295)")]
    );
    let refusals = refusals_of(&service(None, &source));
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
        panic!("a limit refusal: {:?}", refusal.cause);
    };
    assert_eq!(limit.kind.setting().name(), "s3.rational_bits");
    assert_eq!(limit.limit, 65_536);
    assert!(limit.actual > 65_536);
}

/// The identity of [`other_document`]'s package.
const OTHER: &str = "example/other-service";

/// The `Service` document as a second domain package, `example/other-service`.
fn other_document() -> Vec<u8> {
    String::from_utf8(service_document())
        .expect("UTF-8")
        .replace(P, OTHER)
        .into_bytes()
}

/// The `model Other` selection of [`other_document`]: a second model of
/// the package.
fn other_model() -> String {
    let packages = package_input([other_document().as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    format!(
        "model Other = {OTHER:?} version \"1.0.0\" digest \"sha256-jcs:{}\";",
        qsl_semantics::model::key::hex(digest)
    )
}

/// FR-185-AC-5 (TC-620 step 5): a second `workload Steady on Service`, and a
/// `workload Steady on Other` on a second model of the package, are each
/// refused `conflicting_declaration`/`duplicate-declaration` with `subject`
/// `Steady` and `related` both declarations' names in source order; the
/// first remains the package's `Steady`: `related[0]` is the first, earlier
/// in the source. A refused
/// unit has no graph to read `Steady` from, so the related regions are where
/// the clause is observable.
#[trace("TC-620", "FR-185-AC-5")]
#[test]
fn a_workload_name_is_package_wide() {
    let other = "workload Steady on Other { weight Server::request = 1; \
         weight Server::attempt = 1; weight Server::reset = 1; }";
    let selection = other_model();
    for (prefix, second) in [("", STEADY), (selection.as_str(), other)] {
        // With two models, a `random` or `reward` names its model.
        let declarations = service(None, second)
            .replace("random Server::", "random Service::Server::")
            .replace("on Server::attempt =", "on Service::Server::attempt =");
        let body = format!("{prefix}\n{declarations}");
        let declarations = assemble(&body);
        let regions = declarations.regions();
        let (text, _) = service_unit(&body);
        let refusals = declarations
            .check(CheckingLimits::default())
            .expect_err("the unit refuses");
        let [refusal] = &refusals[..] else {
            panic!("{second}: one refusal: {refusals:?}");
        };
        let CheckCause::Probabilistic(cause) = &refusal.cause else {
            panic!("a probabilistic cause: {refusal:?}");
        };
        assert!(matches!(
            cause.as_ref(),
            ProbabilisticCause::DuplicateWorkload { workload, .. } if workload == "Steady"
        ));
        assert_eq!(
            (refusal.cause.code(), refusal.cause.cause()),
            (Code::ConflictingDeclaration, Some("duplicate-declaration"))
        );
        let related: Vec<(u64, &str)> = regions
            .related_regions(refusal)
            .iter()
            .map(|region| {
                (
                    region.start(),
                    &text[region.start() as usize..region.end() as usize],
                )
            })
            .collect();
        assert_eq!(related.len(), 2);
        assert!(related[0].0 < related[1].0);
        assert_eq!(
            related.iter().map(|(_, text)| *text).collect::<Vec<_>>(),
            ["Steady", "Steady"]
        );
    }
}

/// FR-185-AC-6 (TC-620 step 6): the running total of a distribution's
/// weights is bounded by the 65,536-bit ceiling after each addition:
/// `1/3 + 1/(2^65534 + 1)` has a 65,536-bit denominator and checks;
/// `1/3 + 1/(2^65535 + 5)` has a 65,537-bit one and is refused with the
/// limit refusal at the parameter `d`.
#[trace("TC-620", "FR-185-AC-6")]
#[test]
fn a_distribution_total_is_bounded_after_each_addition() {
    let random = |offset: u64, shift: u64| {
        let denominator = Integer::one()
            .shifted_left(shift)
            .add(&Integer::from(offset));
        format!(
            "random Server::attempt(d) ~ {{ 1: rational(1, 3), 3: rational(1, {denominator}) }};"
        )
    };
    checked(&service(Some((D, &random(1, 65_534))), ""));
    let refused = service(Some((D, &random(5, 65_535))), "");
    assert_eq!(located(&refused), [limit("d")]);
    let refusals = refusals_of(&refused);
    let CheckCause::ResourceExhausted(limit) = &refusals[0].cause else {
        panic!("a limit refusal: {refusals:?}");
    };
    assert_eq!((limit.limit, limit.actual), (65_536, 65_537));
}

/// FR-185-AC-6 (TC-620 step 6): a check of the 65,537-bit distribution,
/// which unhindered refuses with the 65,536-bit ceiling, ends cancelled when
/// its handle is tripped: before it starts, or at its first meter charge
/// once it runs. The check itself observes the cancellation (the handle's
/// `tripped` cause is `Requested`), and its refusals are the cancelled
/// work-budget refusal, never the ceiling.
#[trace("TC-620", "FR-185-AC-6")]
#[test]
fn a_cancelled_check_ends_cancelled() {
    let denominator = Integer::one()
        .shifted_left(65_535)
        .add(&Integer::from(5u64));
    let refused = service(
        Some((
            D,
            &format!(
                "random Server::attempt(d) ~ {{ 1: rational(1, 3), 3: rational(1, {denominator}) }};"
            ),
        )),
        "",
    );
    let before = quire_exact::Cancel::new();
    before.cancel(quire_exact::CancelCause::Requested);
    let handle = std::sync::Arc::new(std::sync::OnceLock::<quire_exact::Cancel>::new());
    let tripping = {
        let handle = std::sync::Arc::clone(&handle);
        quire_exact::Cancel::observing(move || {
            if let Some(cancel) = handle.get() {
                cancel.cancel(quire_exact::CancelCause::Requested);
            }
        })
    };
    assert!(handle.set(tripping.clone()).is_ok());
    for (when, cancel) in [("before", before), ("at the first charge", tripping)] {
        let refusals = assemble(&refused)
            .check_with_cancel(CheckingLimits::default(), &cancel)
            .expect_err("a cancelled check refuses");
        assert_eq!(
            cancel.tripped(),
            Some(quire_exact::CancelCause::Requested),
            "{when}: the check observed the cancellation"
        );
        assert!(!refusals.is_empty(), "{when}");
        for refusal in &refusals {
            let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
                panic!("{when}: a cancelled refusal: {refusal:?}");
            };
            assert_eq!(
                (limit.kind, limit.limit, limit.actual),
                (CheckingLimitKind::WorkBudget, 0, 0),
                "{when}: {refusal:?}"
            );
        }
    }
}
