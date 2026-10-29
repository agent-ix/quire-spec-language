// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-483 and FR-484 (TC-972, TC-973): the `value-validity` check arm.
//!
//! The QSpec vectors `proposals/backend-check-arm-v1/vectors.json` are read
//! in place from `$QSPEC_DIR`, never copied. A vector's `model` is a small
//! expression over bounded integers; each is written out as a QSL function,
//! compiled through S1 to S4 and checked by the arm through `analyze`, and
//! the arm's result is compared with the vector's result, cause and locus and
//! with an independent evaluation of the model. Every value-validity vector
//! that negotiates `supported` is accounted for below: decided here, decided
//! with a named deviation, or named as outside this arm. Counts are derived
//! from the vectors.
//!
//! The vectors place a clause at occurrence role `claim`; the tests build
//! that key on the function's node, as the vector states it.

use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU64;

use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze, value_item, AnalyzeItem, AnalyzeRecord, DispositionKind, Engine, Engines, Settlement,
};
use qsl_analyze::value_check::{manifest, ValueRequest};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::RequestIndex;
use qsl_package::CheckedPackage;
use qsl_replay::value_check::{ParameterDomain, Selection, ValueTarget, Verdict, Violation};
use qsl_replay::{
    Certification, DisagreementCause, IncompleteCause, InconclusiveCause, ModelCheckLimit,
    OccurrenceKey, ProofBasis, TerminalValue,
};
use qsl_route::Mode;
use qsl_semantics::check::{Capability, NodeTag, PackageDeclarations};
use qsl_semantics::value::{CatalogRole, DefinitionLock};
use quire_exact::{Cancel, Integer, IntegerInterval, Origin, Role, Value};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::location::{Location, Origin as LocationOrigin};
use serde_json::Value as Json;

// ---- reading the vectors ----

fn vectors() -> Vec<Json> {
    let root = std::env::var_os("QSPEC_DIR")
        .expect("QSPEC_DIR names the QSpec checkout whose check-arm vectors this reads");
    let path = std::path::Path::new(&root).join("proposals/backend-check-arm-v1/vectors.json");
    let text = std::fs::read_to_string(&path).expect("the vectors file is readable");
    let document: Json = serde_json::from_str(&text).expect("the vectors are JSON");
    document["cases"].as_array().expect("cases").clone()
}

fn descriptor_cases() -> Vec<Json> {
    let root = std::env::var_os("QSPEC_DIR").expect("QSPEC_DIR");
    let path = std::path::Path::new(&root).join("proposals/backend-check-arm-v1/vectors.json");
    let document: Json = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    document["descriptor_cases"].as_array().unwrap().clone()
}

/// The value-validity vectors the negotiation settles `supported`: the ones
/// that reach an arm.
fn supported() -> Vec<Json> {
    vectors()
        .into_iter()
        .filter(|c| {
            c["item"]["kind"] == "value-validity" && c["negotiation"]["disposition"] == "supported"
        })
        .collect()
}

fn id_of(case: &Json) -> &str {
    case["id"].as_str().unwrap()
}

/// The functions a model declares, by name.
fn functions(model: &Json) -> Vec<(String, Json)> {
    model["functions"]
        .as_object()
        .map(|f| f.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}

fn calls(e: &Json, out: &mut Vec<String>) {
    if let Json::Array(items) = e {
        if items[0] == "call" {
            out.push(items[1].as_str().unwrap().to_owned());
        }
        items.iter().skip(1).for_each(|i| calls(i, out));
    }
}

/// Which vectors the arm cannot be given as a QSL package, and why.
#[derive(Debug, PartialEq)]
enum Shape {
    /// A clause model the arm decides.
    Clause,
    /// A scalar-operation model.
    Scalar,
    /// A call to a function that has no declared body (FR-484-AC-9).
    CalleeAbsent,
    /// A function that calls itself without a decreasing measure: QSL's
    /// termination check admits none, so no package holds it.
    NonReturning,
    /// No model: the vector exercises another provider or form.
    NoModel,
}

fn shape(case: &Json) -> Shape {
    let model = &case["model"];
    let declared: Vec<String> = functions(model).into_iter().map(|(n, _)| n).collect();
    let mut called = Vec::new();
    calls(&model["goal"], &mut called);
    for (_, f) in functions(model) {
        calls(&f["body"], &mut called);
    }
    if model.get("goal").is_none() && model.get("body").is_none() {
        return Shape::NoModel;
    }
    if model["body"].is_array() {
        return Shape::Scalar;
    }
    for (name, f) in functions(model) {
        let mut inner = Vec::new();
        calls(&f["body"], &mut inner);
        if inner.contains(&name) {
            return Shape::NonReturning;
        }
    }
    if called.iter().any(|c| !declared.contains(c)) {
        return Shape::CalleeAbsent;
    }
    Shape::Clause
}

// ---- the model as QSL source, and as an independent evaluation ----

fn source(e: &Json) -> String {
    match e {
        Json::Number(n) => {
            let n = n.as_i64().unwrap();
            if n < 0 {
                format!("(0 - {})", -n)
            } else {
                n.to_string()
            }
        }
        Json::String(name) => name.clone(),
        Json::Array(items) => {
            let op = items[0].as_str().unwrap();
            let arg = |i: usize| source(&items[i]);
            match op {
                "call" => {
                    let args: Vec<String> = (2..items.len()).map(arg).collect();
                    format!("{}({})", items[1].as_str().unwrap(), args.join(", "))
                }
                "if" => format!("(if {} then {} else {})", arg(1), arg(2), arg(3)),
                "@" => arg(2),
                "/" => format!("({} div {})", arg(1), arg(2)),
                _ => format!("({} {op} {})", arg(1), arg(2)),
            }
        }
        other => panic!("bad expression {other}"),
    }
}

/// `None` when the expression is undefined at `vars`.
fn eval(e: &Json, vars: &HashMap<String, i128>, functions: &Json) -> Option<i128> {
    match e {
        Json::Number(n) => Some(i128::from(n.as_i64().unwrap())),
        Json::String(name) => Some(vars[name]),
        Json::Array(items) => {
            let op = items[0].as_str().unwrap();
            let arg = |i: usize| eval(&items[i], vars, functions);
            let flag = |b: bool| Some(i128::from(b));
            match op {
                "+" => Some(arg(1)? + arg(2)?),
                "-" => Some(arg(1)? - arg(2)?),
                "*" => Some(arg(1)? * arg(2)?),
                "/" => {
                    let (a, d) = (arg(1)?, arg(2)?);
                    (d != 0).then(|| a.div_euclid(d))
                }
                ">" => flag(arg(1)? > arg(2)?),
                "<" => flag(arg(1)? < arg(2)?),
                "<=" => flag(arg(1)? <= arg(2)?),
                ">=" => flag(arg(1)? >= arg(2)?),
                "=" => flag(arg(1)? == arg(2)?),
                "!=" => flag(arg(1)? != arg(2)?),
                "and" => {
                    if arg(1)? == 0 {
                        Some(0)
                    } else {
                        flag(arg(2)? != 0)
                    }
                }
                "@" => arg(2),
                "if" => {
                    if arg(1)? != 0 {
                        arg(2)
                    } else {
                        arg(3)
                    }
                }
                "call" => {
                    let function = &functions[items[1].as_str().unwrap()];
                    let mut scope = HashMap::new();
                    for (i, p) in function["params"].as_array().unwrap().iter().enumerate() {
                        scope.insert(p.as_str().unwrap().to_owned(), arg(2 + i)?);
                    }
                    eval(&function["body"], &scope, functions)
                }
                other => panic!("operation {other} has no QSL spelling here"),
            }
        }
        other => panic!("bad expression {other}"),
    }
}

type Vars = HashMap<String, i128>;

fn assignments(model: &Json) -> Vec<Vars> {
    let mut all = vec![HashMap::new()];
    for v in model["variables"].as_array().unwrap() {
        let (lo, hi) = (v["lo"].as_i64().unwrap(), v["hi"].as_i64().unwrap());
        all = all
            .into_iter()
            .flat_map(|m| {
                (lo..=hi).map(move |x| {
                    let mut m = m.clone();
                    m.insert(v["name"].as_str().unwrap().to_owned(), i128::from(x));
                    m
                })
            })
            .collect();
    }
    all
}

/// Every assignment at which the clause goal is false or undefined.
fn violations(model: &Json) -> Vec<Vars> {
    assignments(model)
        .into_iter()
        .filter(|vars| eval(&model["goal"], vars, &model["functions"]) != Some(1))
        .collect()
}

fn param_list(model: &Json) -> String {
    model["variables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            format!(
                "{}: Int[{}, {}]",
                v["name"].as_str().unwrap(),
                v["lo"],
                v["hi"]
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn declarations_of(model: &Json) -> String {
    let mut text = String::new();
    for (name, function) in functions(model) {
        let params: Vec<String> = function["params"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| format!("{}: Integer", p.as_str().unwrap()))
            .collect();
        text += &format!(
            "function {name} using v({}): Integer pure {{ {} }}\n",
            params.join(", "),
            source(&function["body"])
        );
    }
    if model["goal"].is_array() {
        text += &format!(
            "function subject using v({}): Boolean pure {{ {} }}\n",
            param_list(model),
            source(&model["goal"])
        );
    } else {
        let boolean = matches!(model["body"][0].as_str(), Some("and" | ">" | "<" | "="));
        text += &format!(
            "function subject using v({}): {} pure {{ {} }}\n",
            param_list(model),
            if boolean { "Boolean" } else { "Integer" },
            source(&model["body"])
        );
    }
    text
}

// ---- compiling and running ----

fn package_of(declarations: &str) -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{declarations}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}\n{text}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler builds the package declarations");
    let lock = DefinitionLock::pinned();
    declarations.integer_division = Some(
        lock.admit_integer_division(
            &[lock
                .entry(CatalogRole::IntegerDivisionEuclidean)
                .reference()],
            None,
        )
        .expect("Euclidean division is admitted"),
    );
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|e| panic!("the package checks: {e:?}\n{text}")),
    )
}

fn wire_of(package: &CheckedPackage, function: &str) -> WireNodeId {
    let callable = package
        .graph()
        .callable(function)
        .expect("the function exists");
    WireNodeId::from_digest(*callable.identity.as_bytes())
}

/// The key the vector names: the function's node at the vector's role and
/// ordinal.
fn key_of(package: &CheckedPackage, function: &str, occurrence: &Json) -> OccurrenceKey {
    OccurrenceKey::new(
        wire_of(package, function),
        Origin::new(
            Role::new(occurrence["role"].as_str().unwrap()),
            occurrence["ordinal"].as_u64().unwrap(),
        ),
    )
}

fn index() -> RequestIndex {
    RequestIndex::new(0)
}

fn integer(n: i128) -> Value {
    Value::Integer(Integer::from(n))
}

fn engines_with(max_cases: u64) -> Engines {
    let mut engines = Engines::default();
    engines
        .with(Engine::value_check_with_max_cases(NonZeroU64::new(max_cases).unwrap()).unwrap())
        .unwrap();
    engines
}

struct Ask {
    max_cases: Option<u64>,
    reported: Option<Vec<Value>>,
}

impl Ask {
    fn search() -> Self {
        Self {
            max_cases: None,
            reported: None,
        }
    }
}

fn run_in(
    engines: &Engines,
    package: &CheckedPackage,
    key: &OccurrenceKey,
    function: &str,
    ask: Ask,
) -> AnalyzeRecord {
    let request = ValueTarget::select(package, function, key).map(|target| ValueRequest {
        target,
        max_cases: ask.max_cases.and_then(NonZeroU64::new),
        reported: ask.reported,
    });
    analyze(engines, vec![value_item(index(), request)], &Cancel::new()).remove(0)
}

fn run(
    package: &CheckedPackage,
    key: &OccurrenceKey,
    function: &str,
    max_cases: Option<u64>,
) -> AnalyzeRecord {
    run_in(
        &Engines::qsl().unwrap(),
        package,
        key,
        function,
        Ask {
            max_cases,
            reported: None,
        },
    )
}

fn terminal(record: &AnalyzeRecord) -> &TerminalValue {
    record
        .terminal()
        .unwrap_or_else(|| panic!("a terminal record, not {:?}", record.settlement))
        .value()
}

/// The item's FR-331 result label, as the outcome writer renders it.
fn label(record: &AnalyzeRecord) -> String {
    let item = record.outcome_item().expect("a terminal item");
    serde_json::to_value(&item).unwrap()["result"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn named(model: &Json, assignment: &[Value]) -> Vars {
    model["variables"]
        .as_array()
        .unwrap()
        .iter()
        .zip(assignment)
        .map(|(v, value)| {
            let Value::Integer(n) = value else {
                panic!("an integer")
            };
            (
                v["name"].as_str().unwrap().to_owned(),
                n.to_string().parse().unwrap(),
            )
        })
        .collect()
}

fn candidate_of(model: &Json, candidate: &Json) -> Vec<Value> {
    model["variables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            integer(i128::from(
                candidate[v["name"].as_str().unwrap()].as_i64().unwrap(),
            ))
        })
        .collect()
}

/// The vector's registry default for `max_cases`.
fn registry_default(case: &Json) -> u64 {
    case["registry"][0]["bounds"]["max_cases"].as_u64().unwrap()
}

/// Where the vectors and FR-484 differ on the cause name of a zero divisor.
/// FR-484 says `cause` is "the catalogued reason the native evaluator
/// reports", which for a zero divisor is `division-by-zero` (TC-366 WA-08,
/// TC-348 UE-12); `quotient-outside-domain` is the cause of the distinct
/// `division_out_of_domain` refusal of a bounded consumer domain (FR-272,
/// TC-192 DIV-05). The arm carries the native reason; the vector's name is
/// recorded here, not matched.
const VECTOR_ZERO_DIVISOR_CAUSE: &str = "quotient-outside-domain";
const NATIVE_ZERO_DIVISOR_CAUSE: &str = "division-by-zero";

/// Compare one run against the vector's stated outcome: result, cause,
/// locus and counterexample.
fn assert_matches_vector(case: &Json, model: &Json, record: &AnalyzeRecord, claim_node: &str) {
    let id = id_of(case);
    let expected = &case["outcome"]["results"][0];
    let result = expected["result"].as_str().unwrap();
    assert_eq!(label(record), result, "{id}: the result");
    let found = violations(model);
    match terminal(record) {
        TerminalValue::Proved {
            basis,
            certification,
        } => {
            assert!(found.is_empty(), "{id}: the model has no violation");
            assert_eq!(case["method"], "enumeration", "{id}");
            assert_eq!(*basis, ProofBasis::Exhaustive, "{id}");
            assert_eq!(*certification, Certification::Uncertified, "{id}");
            assert!(expected.get("cause").is_none(), "{id}");
        }
        TerminalValue::Refuted => {
            let counterexample = record.value_check.as_ref().expect("a counterexample");
            let vars = named(model, counterexample.reproduced.assignment());
            assert!(found.contains(&vars), "{id}: {vars:?} violates the goal");
            let undefined = counterexample.reproduced.violation().undefined.as_ref();
            let model_undefined = eval(&model["goal"], &vars, &model["functions"]).is_none();
            assert_eq!(undefined.is_some(), model_undefined, "{id}");
            match expected.get("cause") {
                Some(cause) => {
                    assert_eq!(cause["type"], "undefined-evaluation", "{id}");
                    let at = undefined.expect("an undefined evaluation names its locus");
                    assert_eq!(cause["cause"], VECTOR_ZERO_DIVISOR_CAUSE, "{id}");
                    assert_eq!(
                        at.reason, NATIVE_ZERO_DIVISOR_CAUSE,
                        "{id}: the native cause"
                    );
                    // The locus is the innermost undefined subexpression, an
                    // `expression` occurrence of its own, never the claim.
                    assert_eq!(at.locus.origin().role().as_str(), "expression", "{id}");
                    assert_ne!(
                        at.locus.node().to_string(),
                        claim_node,
                        "{id}: not the claim"
                    );
                    let item = serde_json::to_value(record.outcome_item().unwrap()).unwrap();
                    assert_eq!(
                        item["cause"]["kind"], "undefined-evaluation",
                        "{id}: {item}"
                    );
                    assert_eq!(item["cause"]["cause"], at.reason, "{id}");
                }
                None => assert!(undefined.is_none(), "{id}: a false goal names no locus"),
            }
        }
        TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, value }) => {
            let cause = &expected["cause"];
            assert_eq!(cause["type"], "limit-reached", "{id}");
            assert_eq!(cause["limit"], limit.as_str(), "{id}");
            assert_eq!(cause["setting"], limit.setting(), "{id}");
            assert_eq!(cause["value"], *value, "{id}");
            assert_eq!(*limit, ModelCheckLimit::ValueCheckMaxCases, "{id}");
        }
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_)) => {
            assert_eq!(expected["result"], "inconclusive", "{id}");
        }
        other => panic!("{id}: the vector says {result}, the arm settled {other:?}"),
    }
}

fn claim_node(package: &CheckedPackage, function: &str) -> String {
    wire_of(package, function).to_string()
}

// ---- the vectors ----

/// Every supported value-validity vector is classified; the classes are
/// derived from the vectors, so a vector the QSpec adds is a new line here.
#[test]
fn every_supported_vector_is_decided_deviating_or_named_as_outside_this_arm() {
    let mut by_shape: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for case in supported() {
        by_shape
            .entry(format!("{:?}", shape(&case)))
            .or_default()
            .push(id_of(&case).to_owned());
    }
    let total: usize = by_shape.values().map(Vec::len).sum();
    assert_eq!(total, supported().len());
    // The shapes the other tests below decide.
    assert!(by_shape["Clause"].len() >= 9, "{by_shape:?}");
    assert!(by_shape["Scalar"].len() >= 5, "{by_shape:?}");
    assert_eq!(by_shape["CalleeAbsent"], ["VV-11"]);
    assert_eq!(by_shape["NonReturning"], ["VV-17"]);
    // The vectors of other providers carry no model: SMT, Kani, plugin,
    // timed-out and cancelled engines, the union case-clause and the
    // independence vector.
    assert!(
        by_shape["NoModel"].iter().all(|id| id != "VV-01"),
        "{by_shape:?}"
    );
}

#[trace("QSpec-TC-973", "QSpec-FR-484-AC-1")]
#[trace("QSpec-TC-973", "QSpec-FR-484-AC-2")]
#[trace("QSpec-TC-973", "QSpec-FR-484-AC-7")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-3")]
#[test]
fn the_clause_vectors_settle_as_the_vectors_state() {
    let cases: Vec<Json> = supported()
        .into_iter()
        .filter(|c| shape(c) == Shape::Clause)
        .collect();
    assert!(cases.len() >= 9);
    for case in cases {
        let id = id_of(&case);
        let model = &case["model"];
        let package = package_of(&declarations_of(model));
        let key = key_of(&package, "subject", &case["item"]["occurrence"]);
        assert_eq!(
            key.origin().role().as_str(),
            "claim",
            "{id}: the vector's own role"
        );
        let engines = engines_with(registry_default(&case));
        let item_limit = case["item"]["check_limit"].as_u64();
        let node = claim_node(&package, "subject");
        // Searched by the arm, unless the vector's point is a reported
        // candidate that is not itself a violation.
        let candidate = case.get("candidate").map(|c| candidate_of(model, c));
        let candidate_violates = candidate
            .as_ref()
            .is_none_or(|c| violations(model).contains(&named(model, c)));
        if candidate_violates {
            let record = run_in(
                &engines,
                &package,
                &key,
                "subject",
                Ask {
                    max_cases: item_limit,
                    reported: None,
                },
            );
            assert_matches_vector(&case, model, &record, &node);
        }
        // The vector's candidate reaches the arm as an engine's report, and
        // the native replay settles it.
        if let Some(candidate) = candidate {
            let record = run_in(
                &engines,
                &package,
                &key,
                "subject",
                Ask {
                    max_cases: item_limit,
                    reported: Some(candidate.clone()),
                },
            );
            assert_matches_vector(&case, model, &record, &node);
            if candidate_violates {
                let found = record.value_check.as_ref().unwrap();
                assert_eq!(found.reproduced.assignment().len(), candidate.len(), "{id}");
                assert_eq!(
                    named(model, found.reproduced.assignment()),
                    named(model, &candidate),
                    "{id}: the replayed candidate"
                );
            } else {
                // FR-483-AC-3: the replay does not reproduce it.
                assert!(
                    matches!(
                        terminal(&record),
                        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
                            DisagreementCause::Verdicts { .. }
                        ))
                    ),
                    "{id}: {:?}",
                    record.settlement
                );
                assert!(record.value_check.is_none(), "{id}");
            }
        }
    }
}

/// The scalar-operation vectors. VV-05 is decided as stated. VV-04 and VV-06
/// state `proved`, which the arm cannot establish: the evaluator reports no
/// occurrence it passed through, so the arm answers `inconclusive` and never
/// `proved`. VV-10 states the vacuous path condition, which needs the same
/// reach information, and VV-08 a range-narrowing operand, whose bound the
/// arm does not check.
#[test]
fn the_scalar_operation_vectors_are_refuted_when_undefined_and_never_proved() {
    let cases: Vec<Json> = supported()
        .into_iter()
        .filter(|c| shape(c) == Shape::Scalar)
        .collect();
    let ids: Vec<&str> = cases.iter().map(id_of).collect();
    assert_eq!(ids, ["VV-04", "VV-05", "VV-06", "VV-08", "VV-10"]);
    for case in cases {
        let id = id_of(&case);
        let model = &case["model"];
        let body = model["body"].to_string();
        if body.contains("\"narrow\"") {
            // Named deviation: no QSL spelling of the narrowing, and the
            // operand's bound is not an obligation this arm discharges.
            assert_eq!(id, "VV-08");
            continue;
        }
        let package = package_of(&declarations_of(model));
        let mut records: Vec<OccurrenceKey> = package
            .graph()
            .requirements()
            .iter()
            .filter(|(_, r)| {
                r.requirements().kind() == Capability::ValueValidity && r.owner() == Some("subject")
            })
            .map(|(k, _)| k.clone())
            .collect();
        records.sort_by_key(|k| k.origin().ordinal());
        let ordinal = case["item"]["occurrence"]["ordinal"].as_u64().unwrap();
        let key = records
            .iter()
            .find(|k| k.origin().ordinal() == ordinal)
            .unwrap_or_else(|| {
                panic!("{id}: no requirement record at ordinal {ordinal}: {records:?}")
            })
            .clone();
        let record = run(&package, &key, "subject", None);
        let outcome = label(&record);
        match id {
            "VV-05" => {
                assert_eq!(outcome, "refuted");
                let found = record.value_check.as_ref().unwrap();
                let at = found.reproduced.violation().undefined.as_ref().unwrap();
                assert_eq!(at.locus, key, "the locus is the occurrence itself");
                assert_eq!(at.reason, NATIVE_ZERO_DIVISOR_CAUSE);
                // The vector's candidate (b = 0) replays to the same locus.
                let candidate = candidate_of(model, &case["candidate"]);
                let replayed = run_in(
                    &Engines::qsl().unwrap(),
                    &package,
                    &key,
                    "subject",
                    Ask {
                        max_cases: None,
                        reported: Some(candidate),
                    },
                );
                assert_eq!(label(&replayed), "refuted");
                let again = replayed.value_check.as_ref().unwrap();
                assert_eq!(again.reproduced.violation(), found.reproduced.violation());
            }
            "VV-04" | "VV-06" => {
                assert_eq!(case["outcome"]["results"][0]["result"], "proved");
                assert_eq!(outcome, "inconclusive", "{id}: never proved");
                assert!(matches!(
                    terminal(&record),
                    TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive { .. })
                ));
            }
            "VV-10" => {
                assert_eq!(outcome, "inconclusive");
                assert!(
                    matches!(
                        terminal(&record),
                        TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive { .. })
                    ),
                    "the vector's kani_vacuous_proof needs reach information the evaluator lacks"
                );
            }
            other => panic!("unexpected scalar vector {other}"),
        }
    }
}

// ---- the clause semantics ----

#[trace("QSpec-TC-973", "QSpec-FR-484-AC-7")]
#[test]
fn an_undefined_subexpression_is_located_at_its_own_occurrence() {
    let package = package_of(
        "function subject using v(a: Int[-1, 1], b: Int[-1, 1]): Boolean pure \
         { (a div b) = (a div b) }",
    );
    let key = key_of(
        &package,
        "subject",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    let record = run(&package, &key, "subject", None);
    assert!(matches!(terminal(&record), TerminalValue::Refuted));
    let found = record.value_check.as_ref().unwrap();
    let at = found.reproduced.violation().undefined.as_ref().unwrap();
    // The first operand of the comparison, the first `a div b` evaluated.
    let expected = Location::at(
        LocationOrigin::Body {
            function: "subject".to_owned(),
            index: 0,
        },
        &[0],
    );
    let package_location = package
        .graph()
        .occurrence(
            package
                .graph()
                .semantic_graph()
                .resolve_wire(at.locus.node())
                .unwrap(),
            at.locus.origin(),
        )
        .expect("the locus is an occurrence of the package");
    assert_eq!(package_location, &expected);
    assert_ne!(at.locus, key);
    assert_eq!(at.reason, NATIVE_ZERO_DIVISOR_CAUSE);
    // The assignment is the first violating one in enumeration order.
    let first = [integer(-1), integer(0)];
    assert_eq!(found.reproduced.assignment().len(), first.len());
    assert!(matches!(&found.reproduced.assignment()[1], Value::Integer(b) if b.to_string() == "0"));
}

#[trace("QSpec-TC-973", "QSpec-FR-484-AC-7")]
#[test]
fn an_undefinedness_inside_a_callee_is_located_in_the_callees_body() {
    let package = package_of(
        "function inverse using v(n: Integer): Integer pure { 10 div n }\n\
         function subject using v(x: Int[-1, 1]): Boolean pure { inverse(x) >= 0 - 10 }",
    );
    let key = key_of(
        &package,
        "subject",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    let record = run(&package, &key, "subject", None);
    assert!(
        matches!(terminal(&record), TerminalValue::Refuted),
        "{:?}",
        record.settlement
    );
    let found = record.value_check.as_ref().unwrap();
    let at = found.reproduced.violation().undefined.as_ref().unwrap();
    let location = package
        .graph()
        .occurrence(
            package
                .graph()
                .semantic_graph()
                .resolve_wire(at.locus.node())
                .unwrap(),
            at.locus.origin(),
        )
        .unwrap();
    // The call site is not a second locus: the locus lies in the callee.
    assert!(
        matches!(&location.origin, LocationOrigin::Body { function, .. } if function == "inverse"),
        "{location:?}"
    );
}

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-2")]
#[test]
fn an_unsatisfiable_domain_proves_no_case_and_is_vacuous_not_proved() {
    let package = package_of("function subject using v(x: Int[0, 3]): Boolean pure { x >= 0 }");
    let key = key_of(
        &package,
        "subject",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    let target = ValueTarget::select(&package, "subject", &key).unwrap();
    let node = target.parameters()[0].node();
    let outside = IntegerInterval::new(Integer::from(10_i64), Integer::from(20_i64)).unwrap();
    let target = target.bounded(node, &outside);
    assert!(matches!(
        target.parameters()[0].domain(),
        ParameterDomain::Integers(None)
    ));
    let record = analyze(
        &Engines::qsl().unwrap(),
        vec![value_item(
            index(),
            Ok(ValueRequest {
                target,
                max_cases: None,
                reported: None,
            }),
        )],
        &Cancel::new(),
    )
    .remove(0);
    // Zero cases examined: not an exhaustive proof, the vacuous one.
    assert!(
        matches!(
            terminal(&record),
            TerminalValue::Proved {
                basis: ProofBasis::Checks { success_checks: 0 },
                ..
            }
        ),
        "{:?}",
        record.settlement
    );
    let item = serde_json::to_value(record.outcome_item().unwrap()).unwrap();
    assert_eq!(item["result"], "inconclusive", "{item}");
    assert_eq!(item["category"], "inconclusive", "{item}");
    assert_eq!(item["cause"]["kind"], "kani-vacuous-proof", "{item}");
}

/// The mutation kills at the replay: a replay compares the violation it
/// reproduces with the one it was asked to confirm.
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-3")]
#[test]
fn a_replay_confirms_the_violation_it_was_asked_about_and_no_other() {
    let package = package_of(
        "function subject using v(a: Int[-1, 1], b: Int[-1, 1]): Boolean pure \
         { (a div b) = (a div b) }",
    );
    let key = key_of(
        &package,
        "subject",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    let target = ValueTarget::select(&package, "subject", &key).unwrap();
    let at = [integer(1), integer(0)];
    let Verdict::Violated(actual) = target.evaluate(&at, &Cancel::new()).unwrap() else {
        panic!("undefined at b = 0")
    };
    assert!(target
        .replay(&at, &actual, &Cancel::new())
        .unwrap()
        .is_some());
    // A false goal is not the undefined one the candidate claimed.
    let claimed_false = Violation { undefined: None };
    assert!(target
        .replay(&at, &claimed_false, &Cancel::new())
        .unwrap()
        .is_none());
    // A different locus or reason is not the same violation.
    let mut elsewhere = actual.clone();
    elsewhere.undefined.as_mut().unwrap().reason = "none-value";
    assert!(target
        .replay(&at, &elsewhere, &Cancel::new())
        .unwrap()
        .is_none());
    let mut other_locus = actual.clone();
    other_locus.undefined.as_mut().unwrap().locus = key.clone();
    assert!(target
        .replay(&at, &other_locus, &Cancel::new())
        .unwrap()
        .is_none());
    // Where the goal holds, no violation is reproduced.
    let fine = [integer(1), integer(1)];
    assert_eq!(
        target.evaluate(&fine, &Cancel::new()).unwrap(),
        Verdict::Holds
    );
    assert!(target
        .replay(&fine, &actual, &Cancel::new())
        .unwrap()
        .is_none());
    // A false goal replays only as a false goal.
    let false_goal = package_of("function subject using v(x: Int[0, 3]): Boolean pure { x < 3 }");
    let key = key_of(
        &false_goal,
        "subject",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    let target = ValueTarget::select(&false_goal, "subject", &key).unwrap();
    let three = [integer(3)];
    assert!(target
        .replay(&three, &claimed_false, &Cancel::new())
        .unwrap()
        .is_some());
    assert!(target
        .replay(&three, &actual, &Cancel::new())
        .unwrap()
        .is_none());
}

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-5")]
#[test]
fn a_claim_that_selects_no_form_is_declined_without_a_check() {
    let package = package_of("function f using v(x: Int[0, 3]): Int[0, 4] pure { x + 1 }");
    let none = OccurrenceKey::new(
        WireNodeId::from_digest([7; 32]),
        Origin::new(Role::new("claim"), 0),
    );
    assert!(matches!(
        ValueTarget::select(&package, "f", &none),
        Err(Selection::NoForm)
    ));
    // A non-Boolean function is not a clause, by either role.
    for role in ["declaration", "claim"] {
        let key = OccurrenceKey::new(wire_of(&package, "f"), Origin::new(Role::new(role), 0));
        assert!(matches!(
            ValueTarget::select(&package, "f", &key),
            Err(Selection::NoForm)
        ));
    }
    let record = run(&package, &none, "f", None);
    let TerminalValue::Declined { cause, .. } = terminal(&record) else {
        panic!("declined, not {:?}", record.settlement)
    };
    assert_eq!(cause.as_str(), "invalid-input");
    assert_eq!(label(&record), "declined");
}

// ---- the budget ----

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-4")]
#[test]
fn the_item_limit_replaces_the_descriptor_default_and_the_descriptor_default_applies() {
    let package = package_of("function subject using v(x: Int[0, 255]): Boolean pure { x >= 0 }");
    let key = key_of(
        &package,
        "subject",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    let limit = |record: &AnalyzeRecord| match terminal(record) {
        TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, value }) => {
            assert_eq!(*limit, ModelCheckLimit::ValueCheckMaxCases);
            Some(*value)
        }
        TerminalValue::Proved { .. } => None,
        other => panic!("{other:?}"),
    };
    // The descriptor publishes 50: no item limit stops at 50.
    let engines = engines_with(50);
    let by_default = run_in(&engines, &package, &key, "subject", Ask::search());
    assert_eq!(limit(&by_default), Some(50));
    // The item's 100 replaces the default of 50, and replaces a default of
    // 10,000 the other way: distinct values on both sides.
    let item = run_in(
        &engines,
        &package,
        &key,
        "subject",
        Ask {
            max_cases: Some(100),
            reported: None,
        },
    );
    assert_eq!(limit(&item), Some(100));
    let large = engines_with(10_000);
    let small_item = run_in(
        &large,
        &package,
        &key,
        "subject",
        Ask {
            max_cases: Some(30),
            reported: None,
        },
    );
    assert_eq!(limit(&small_item), Some(30));
    // A budget equal to the domain finishes; one short does not.
    let exact = run(&package, &key, "subject", Some(256));
    assert_eq!(limit(&exact), None);
    let short = run(&package, &key, "subject", Some(255));
    assert_eq!(limit(&short), Some(255));
    // The production default is the descriptor's own.
    let production = Engines::qsl().unwrap();
    let descriptor = production
        .registrations()
        .find(|d| d.id().as_str() == manifest::BACKEND_ID)
        .expect("the arm is registered");
    assert_eq!(descriptor.max_cases(), Some(manifest::DEFAULT_MAX_CASES));
}

/// The domain is enumerated one value at a time: an item with a budget no
/// machine could hold in memory over a domain no machine could enumerate
/// starts, and stops at its first violation or its small real limit.
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-4")]
#[test]
fn a_huge_domain_and_a_huge_budget_are_enumerated_lazily() {
    let package = package_of(
        "function wide using v(x: Int[0, 1000000000000000000000000000000], \
         y: Int[0, 1000000000000000000000000000000]): Boolean pure { x + y >= 0 }\n\
         function breaks using v(x: Int[0, 1000000000000000000000000000000]): Boolean pure \
         { x < 5 }",
    );
    let wide = key_of(
        &package,
        "wide",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    let stopped = run(&package, &wide, "wide", Some(1_000));
    assert!(matches!(
        terminal(&stopped),
        TerminalValue::Incomplete(IncompleteCause::LimitReached { value: 1_000, .. })
    ));
    let breaks = key_of(
        &package,
        "breaks",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    // A watchdog ends a run that never finds the violation, so a broken
    // search fails this test after a minute instead of never returning.
    let cancel = Cancel::new();
    let watchdog = cancel.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(60));
        watchdog.cancel(quire_exact::CancelCause::Deadline);
    });
    let request = ValueTarget::select(&package, "breaks", &breaks).map(|target| ValueRequest {
        target,
        max_cases: NonZeroU64::new(u64::MAX),
        reported: None,
    });
    let huge = analyze(
        &Engines::qsl().unwrap(),
        vec![value_item(index(), request)],
        &cancel,
    )
    .remove(0);
    assert!(
        matches!(terminal(&huge), TerminalValue::Refuted),
        "{:?}",
        huge.settlement
    );
    let found = huge.value_check.as_ref().unwrap();
    assert!(matches!(&found.reproduced.assignment()[0], Value::Integer(x) if x.to_string() == "5"));
}

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-4")]
#[test]
fn a_cancelled_run_is_incomplete_cancelled_and_evaluates_nothing() {
    let package = package_of("function subject using v(x: Int[0, 255]): Boolean pure { x >= 0 }");
    let key = key_of(
        &package,
        "subject",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    let cancel = Cancel::new();
    cancel.cancel(quire_exact::CancelCause::Deadline);
    let request = ValueTarget::select(&package, "subject", &key).map(|target| ValueRequest {
        target,
        max_cases: None,
        reported: None,
    });
    let records = analyze(
        &Engines::qsl().unwrap(),
        vec![value_item(index(), request)],
        &cancel,
    );
    assert!(matches!(
        terminal(&records[0]),
        TerminalValue::Incomplete(IncompleteCause::Cancelled { .. })
    ));
}

// ---- negotiation and registration ----

#[trace("QSpec-TC-973", "QSpec-FR-484-AC-8")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-6")]
#[test]
fn an_unbounded_integer_parameter_needs_a_bound_and_a_bound_makes_it_bounded() {
    let package = package_of("function subject using v(n: Integer): Boolean pure { n + 1 > n }");
    let key = key_of(
        &package,
        "subject",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    let record = run(&package, &key, "subject", None);
    let Settlement::Disposed(disposed) = &record.settlement else {
        panic!("a disposition, not {:?}", record.settlement)
    };
    assert_eq!(disposed.kind, DispositionKind::Unsupported);
    let target = ValueTarget::select(&package, "subject", &key).unwrap();
    let node = target.parameters()[0].node();
    let bound = IntegerInterval::new(Integer::from(-3_i64), Integer::from(3_i64)).unwrap();
    let target = target.bounded(node, &bound);
    assert!(matches!(
        target.parameters()[0].domain(),
        ParameterDomain::Integers(Some(_))
    ));
    let record = analyze(
        &Engines::qsl().unwrap(),
        vec![value_item(
            index(),
            Ok(ValueRequest {
                target,
                max_cases: None,
                reported: None,
            }),
        )],
        &Cancel::new(),
    )
    .remove(0);
    assert!(matches!(
        terminal(&record),
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            ..
        }
    ));
}

#[test]
fn a_parameter_the_arm_does_not_enumerate_is_unsupported_not_proved() {
    let package = package_of("function subject using v(t: String): Boolean pure { t = t }");
    let key = key_of(
        &package,
        "subject",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    let record = run(&package, &key, "subject", None);
    assert!(
        matches!(terminal(&record), TerminalValue::Unsupported(_)),
        "{:?}",
        record.settlement
    );
    assert_eq!(label(&record), "unsupported");
}

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-7")]
#[test]
fn the_production_registry_holds_the_arm_and_an_absent_provider_is_refused() {
    let engines = Engines::qsl().unwrap();
    let descriptor = engines
        .registrations()
        .find(|d| d.id().as_str() == manifest::BACKEND_ID)
        .expect("a ValueValidity candidate is registered");
    let pairs: Vec<_> = descriptor.advertises().collect();
    assert_eq!(pairs, [(Capability::ValueValidity, Mode::Bounded)]);
    // No provider linked: the item names a candidate the set does not hold.
    let package = package_of("function subject using v(x: Int[0, 3]): Boolean pure { x >= 0 }");
    let key = key_of(
        &package,
        "subject",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    let request = ValueTarget::select(&package, "subject", &key).map(|target| ValueRequest {
        target,
        max_cases: None,
        reported: None,
    });
    let records = analyze(
        &Engines::default(),
        vec![value_item(index(), request)],
        &Cancel::new(),
    );
    let Settlement::Disposed(disposed) = &records[0].settlement else {
        panic!("a disposition")
    };
    assert_eq!(disposed.kind, DispositionKind::InvalidRequest);
}

/// The vectors' `descriptor_cases`: a descriptor that advertises
/// `value-validity` or `operation-contract` is admitted only with a positive
/// `max_cases` count.
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-4")]
#[test]
fn a_check_arm_descriptor_needs_a_positive_max_cases_default() {
    use qsl_route::{
        descriptor_from_manifest_with_max_cases, Candidate, ManifestDigest, ManifestMembers,
        MaxCasesMember, ProviderOrigin,
    };
    let member = |bounds: &Json| match bounds {
        Json::Null => MaxCasesMember::Absent,
        Json::Object(o) => match o.get("max_cases") {
            None => MaxCasesMember::Absent,
            Some(n) => n
                .as_u64()
                .map_or(MaxCasesMember::NotACount, MaxCasesMember::Count),
        },
        _ => MaxCasesMember::NotACount,
    };
    let cases = descriptor_cases();
    assert!(cases.len() >= 5);
    for kind in ["value-validity", "operation-contract"] {
        for entry in &cases {
            let admitted = descriptor_from_manifest_with_max_cases(
                Candidate::new(qsl_foundation::BackendId::new("arm")),
                ProviderOrigin::Linked,
                ManifestDigest::from_digest([1; 32]),
                [(Some(kind), Some("bounded"))],
                ManifestMembers {
                    domains: Some(&["integer"]),
                    ..ManifestMembers::default()
                },
                member(&entry["bounds"]),
            );
            assert_eq!(
                admitted.is_ok(),
                entry["valid"] == true,
                "{kind}: {}",
                entry["name"]
            );
            if let Ok(descriptor) = admitted {
                assert_eq!(
                    descriptor.max_cases().map(NonZeroU64::get),
                    entry["bounds"]["max_cases"].as_u64()
                );
            }
        }
    }
    // A backend of another kind is not asked for one.
    assert!(descriptor_from_manifest_with_max_cases(
        Candidate::new(qsl_foundation::BackendId::new("other")),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest([2; 32]),
        [(Some("refinement"), Some("bounded"))],
        ManifestMembers {
            domains: Some(&["integer"]),
            ..ManifestMembers::default()
        },
        MaxCasesMember::Absent,
    )
    .map_or_else(|e| format!("{e:?}"), |_| String::new())
    .is_empty());
}

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-6")]
#[test]
fn negotiated_refusals_run_no_arm() {
    use qsl_route::routing::Disposition;
    let package = package_of("function subject using v(x: Int[0, 3]): Boolean pure { x >= 0 }");
    let key = key_of(
        &package,
        "subject",
        &serde_json::json!({"role": "claim", "ordinal": 0}),
    );
    for (disposition, kind) in [
        (Disposition::RequiresBound, DispositionKind::RequiresBound),
        (Disposition::Unsupported, DispositionKind::Unsupported),
        (Disposition::InvalidRequest, DispositionKind::InvalidRequest),
    ] {
        let request = ValueTarget::select(&package, "subject", &key).map(|target| ValueRequest {
            target,
            max_cases: None,
            reported: None,
        });
        let item: AnalyzeItem<'_> = value_item(index(), request).disposed(disposition, None);
        let records = analyze(&Engines::qsl().unwrap(), vec![item], &Cancel::new());
        let Settlement::Disposed(disposed) = &records[0].settlement else {
            panic!()
        };
        assert_eq!(disposed.kind, kind);
    }
}

/// The arm names an undefined subexpression by the `expression` occurrence
/// the package locates at the evaluator's location. Two occurrences of one
/// location would make that a guess, so the arm refuses to guess
/// (`locus_of`); here, over packages that nest calls, guards and repeated
/// applications, no location holds two non-parameter `expression`
/// occurrences, so every undefined subexpression has exactly one locus.
#[test]
fn a_location_holds_one_candidate_expression_occurrence() {
    let packages = [
        package_of(
            "function inverse using v(n: Integer): Integer pure { 10 div n }\n\
             function subject using v(x: Int[-1, 1]): Boolean pure { inverse(x) >= 0 - 10 }",
        ),
        package_of(
            "function subject using v(a: Int[-2, 2], b: Int[-2, 2]): Integer pure \
             { (if b != 0 then a div b else a) + a div b }",
        ),
        package_of(
            "function subject using v(a: Int[-2, 2], b: Int[-2, 2]): Boolean pure \
             { b != 0 and a div b > 0 }",
        ),
    ];
    for package in &packages {
        let graph = package.graph();
        let mut seen: HashMap<String, (String, u64)> = HashMap::new();
        for (node, origin, location) in graph.occurrences() {
            if origin.role().as_str() != "expression" {
                continue;
            }
            if graph.semantic_graph().node(node).unwrap().node_tag() == NodeTag::Value {
                continue;
            }
            let at = format!("{:?}{:?}", location.origin, location.path());
            let here = (format!("{node:?}"), origin.ordinal());
            if let Some(other) = seen.insert(at.clone(), here.clone()) {
                panic!("{at} holds {other:?} and {here:?}");
            }
        }
        assert!(!seen.is_empty());
    }
}
