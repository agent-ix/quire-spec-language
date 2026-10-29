// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-483 and FR-484 (TC-972, TC-973): the `value-validity` check arm.
//!
//! The QSpec vectors `proposals/backend-check-arm-v1/vectors.json` are read
//! in place from `$QSPEC_DIR`, never copied. A vector's `model` is a small
//! expression over bounded integers; each is written out as a QSL function,
//! compiled through S1 to S4 and checked by the arm, and the arm's result is
//! compared with the vector's result and with an independent evaluation of
//! the model. Counts are derived from the vectors.

use std::collections::HashMap;
use std::num::NonZeroU64;

use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze, value_item, AnalyzeItem, DispositionKind, Engines, Settlement,
};
use qsl_analyze::value_check::{manifest, ValueRequest};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::RequestIndex;
use qsl_package::CheckedPackage;
use qsl_replay::value_check::{ParameterDomain, Selection, ValueTarget, Verdict};
use qsl_replay::{
    Certification, IncompleteCause, InconclusiveCause, ModelCheckLimit, OccurrenceKey, ProofBasis,
    TerminalValue,
};
use qsl_route::Mode;
use qsl_semantics::check::{Capability, PackageDeclarations};
use qsl_semantics::value::{CatalogRole, DefinitionLock};
use quire_exact::{Cancel, Integer, IntegerInterval, Origin, Role, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::Value as Json;

fn vectors() -> Vec<Json> {
    let root = std::env::var_os("QSPEC_DIR")
        .expect("QSPEC_DIR names the QSpec checkout whose check-arm vectors this reads");
    let path = std::path::Path::new(&root).join("proposals/backend-check-arm-v1/vectors.json");
    let text = std::fs::read_to_string(&path).expect("the vectors file is readable");
    let document: Json = serde_json::from_str(&text).expect("the vectors are JSON");
    document["cases"].as_array().expect("cases").clone()
}

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

fn declaration(package: &CheckedPackage, function: &str) -> OccurrenceKey {
    let callable = package
        .graph()
        .callable(function)
        .expect("the function exists");
    OccurrenceKey::new(
        WireNodeId::from_digest(*callable.identity.as_bytes()),
        Origin::new(Role::new("declaration"), 0),
    )
}

// ---- the vector models, written as QSL and evaluated independently ----

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
                "/" => format!("({} div {})", arg(1), arg(2)),
                _ => format!("({} {op} {})", arg(1), arg(2)),
            }
        }
        other => panic!("bad expression {other}"),
    }
}

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

/// Every assignment of the model's variables that violates its goal.
fn violations(model: &Json) -> Vec<HashMap<String, i128>> {
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
    let functions = &model["functions"];
    all.into_iter()
        .filter(|vars| eval(&model["goal"], vars, functions) != Some(1))
        .collect()
}

fn declarations_of(model: &Json) -> String {
    let mut text = String::new();
    if let Some(functions) = model["functions"].as_object() {
        for (name, function) in functions {
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
    }
    let params: Vec<String> = model["variables"]
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
        .collect();
    text += &format!(
        "function subject using v({}): Boolean pure {{ {} }}\n",
        params.join(", "),
        source(&model["goal"])
    );
    text
}

fn index() -> RequestIndex {
    RequestIndex::new(0)
}

/// Run the arm over `function` of `package` through `analyze`.
fn run(
    package: &CheckedPackage,
    key: &OccurrenceKey,
    function: &str,
    max_cases: Option<u64>,
) -> qsl_analyze::analyze::AnalyzeRecord {
    let target = ValueTarget::select(package, function, key);
    let request = target.map(|target| ValueRequest {
        target,
        max_cases: max_cases.and_then(NonZeroU64::new),
    });
    analyze(
        &Engines::qsl().expect("the linked engines register"),
        vec![value_item(index(), request)],
        &Cancel::new(),
    )
    .remove(0)
}

fn terminal(record: &qsl_analyze::analyze::AnalyzeRecord) -> &TerminalValue {
    record
        .terminal()
        .expect("the item settles a terminal record")
        .value()
}

/// The vectors that carry a clause model the arm decides.
fn clause_vectors() -> Vec<Json> {
    vectors()
        .into_iter()
        .filter(|c| {
            c["item"]["kind"] == "value-validity"
                && c["model"]["goal"].is_array()
                && c["negotiation"]["disposition"] == "supported"
                && c.get("candidate").is_none()
                && !c["model"]["functions"]
                    .as_object()
                    .is_some_and(|f| f.contains_key("loop"))
                && !c["name"].as_str().unwrap().contains("callee body absent")
        })
        .collect()
}

#[trace("QSpec-TC-973", "QSpec-FR-484-AC-1")]
#[trace("QSpec-TC-973", "QSpec-FR-484-AC-2")]
#[trace("QSpec-TC-973", "QSpec-FR-484-AC-7")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-1")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-2")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-3")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-4")]
#[test]
fn the_clause_vectors_settle_as_the_vectors_state() {
    let cases = clause_vectors();
    assert!(!cases.is_empty());
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let model = &case["model"];
        let package = package_of(&declarations_of(model));
        let key = declaration(&package, "subject");
        // The descriptor default the vector's registry publishes, or the item's own.
        let effective = case["item"]["check_limit"]
            .as_u64()
            .or_else(|| case["registry"][0]["bounds"]["max_cases"].as_u64());
        let record = run(&package, &key, "subject", effective);
        let expected = case["arm"]["result"].as_str().unwrap();
        let found = violations(model);
        match (expected, terminal(&record)) {
            (
                "proved",
                TerminalValue::Proved {
                    basis,
                    certification,
                },
            ) => {
                assert!(found.is_empty(), "{id}: independent evaluation agrees");
                assert_eq!(*basis, ProofBasis::Exhaustive, "{id}");
                assert_eq!(*certification, Certification::Uncertified, "{id}");
            }
            ("refuted", TerminalValue::Refuted) => {
                let counterexample = record.value_check.as_ref().expect("a counterexample");
                let named: HashMap<String, i128> = model["variables"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .zip(counterexample.reproduced.assignment())
                    .map(|(v, value)| {
                        let Value::Integer(n) = value else {
                            panic!("{id}: an integer")
                        };
                        (
                            v["name"].as_str().unwrap().to_owned(),
                            n.to_string().parse().unwrap(),
                        )
                    })
                    .collect();
                assert!(found.contains(&named), "{id}: {named:?} violates the goal");
                // An undefined goal names its occurrence; a false one does not.
                let undefined = eval(&model["goal"], &named, &model["functions"]).is_none();
                assert_eq!(
                    counterexample.reproduced.violation().undefined.is_some(),
                    undefined,
                    "{id}"
                );
                if undefined {
                    assert!(record.outcome_item().is_some());
                }
            }
            (
                "incomplete",
                TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, value }),
            ) => {
                assert_eq!(*limit, ModelCheckLimit::ValueCheckMaxCases, "{id}");
                assert_eq!(
                    (limit.as_str(), limit.setting()),
                    ("max_cases", "value_check.max_cases")
                );
                assert_eq!(Some(*value), effective, "{id}: the effective limit");
            }
            ("inconclusive", TerminalValue::Inconclusive(_)) => {}
            (other, found) => panic!("{id}: vector says {other}, the arm settled {found:?}"),
        }
    }
}

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-4")]
#[test]
fn the_item_limit_replaces_the_descriptor_default() {
    let package = package_of("function subject using v(x: Int[0, 255]): Boolean pure { x >= 0 }");
    let key = declaration(&package, "subject");
    let small = run(&package, &key, "subject", Some(50));
    assert!(matches!(
        terminal(&small),
        TerminalValue::Incomplete(IncompleteCause::LimitReached { value: 50, .. })
    ));
    let exact = run(&package, &key, "subject", Some(256));
    assert!(
        matches!(terminal(&exact), TerminalValue::Proved { .. }),
        "a budget equal to the domain finishes"
    );
    // No item limit: the descriptor's published default applies.
    let default = run(&package, &key, "subject", None);
    assert!(matches!(terminal(&default), TerminalValue::Proved { .. }));
    let engines = Engines::qsl().unwrap();
    let descriptor = engines
        .registrations()
        .find(|d| d.id().as_str() == manifest::BACKEND_ID)
        .expect("the arm is registered");
    assert_eq!(descriptor.max_cases(), Some(manifest::DEFAULT_MAX_CASES));
}

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-3")]
#[test]
fn a_candidate_the_replay_does_not_reproduce_is_not_a_refutation() {
    let package =
        package_of("function subject using v(x: Int[0, 255]): Boolean pure { x + 1 > x }");
    let key = declaration(&package, "subject");
    let target = ValueTarget::select(&package, "subject", &key).unwrap();
    let candidate = vec![Value::Integer(Integer::from(3_i64))];
    // The goal holds at the candidate, so no violation exists to replay.
    assert_eq!(
        target.evaluate(&candidate, &Cancel::new()).unwrap(),
        Verdict::Holds
    );
    let claimed = qsl_replay::value_check::Violation { undefined: None };
    assert!(target
        .replay(&candidate, &claimed, &Cancel::new())
        .unwrap()
        .is_none());
}

#[trace("QSpec-TC-973", "QSpec-FR-484-AC-4")]
#[trace("QSpec-TC-973", "QSpec-FR-484-AC-6")]
#[test]
fn a_scalar_operation_is_refuted_at_its_own_occurrence_and_never_proved() {
    let package = package_of(
        "function f using v(a: Int[-2, 2], b: Int[-2, 2]): Integer pure \
         { (if b != 0 then a div b else a) + a div b }",
    );
    let keys: Vec<OccurrenceKey> = package
        .graph()
        .requirements()
        .iter()
        .filter(|(_, r)| {
            r.requirements().kind() == Capability::ValueValidity && r.owner() == Some("f")
        })
        .map(|(k, _)| k.clone())
        .collect();
    let mut refuted = 0;
    for key in &keys {
        let record = run(&package, key, "f", None);
        match terminal(&record) {
            TerminalValue::Refuted => {
                refuted += 1;
                let counterexample = record.value_check.as_ref().unwrap();
                let undefined = counterexample
                    .reproduced
                    .violation()
                    .undefined
                    .as_ref()
                    .unwrap();
                assert_eq!(
                    &undefined.locus, key,
                    "the locus is the claim's own occurrence"
                );
                // b = 0 is the one violating value of the divisor.
                let Value::Integer(b) = &counterexample.reproduced.assignment()[1] else {
                    panic!()
                };
                assert_eq!(b.to_string(), "0");
            }
            TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive { .. }) => {}
            other => panic!("a scalar operation is refuted or unknown, not {other:?}"),
        }
    }
    // The unguarded division is refuted; the guarded one is never proved.
    assert!(refuted >= 1, "{keys:?}");
}

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-5")]
#[test]
fn a_claim_that_selects_no_form_is_declined_without_a_check() {
    let package = package_of("function f using v(x: Int[0, 3]): Int[0, 4] pure { x + 1 }");
    let none = OccurrenceKey::new(
        WireNodeId::from_digest([7; 32]),
        Origin::new(Role::new("subject"), 0),
    );
    assert!(matches!(
        ValueTarget::select(&package, "f", &none),
        Err(Selection::NoForm)
    ));
    // A non-Boolean function's own declaration is not a clause.
    let non_boolean = declaration(&package, "f");
    assert!(matches!(
        ValueTarget::select(&package, "f", &non_boolean),
        Err(Selection::NoForm)
    ));
    let record = run(&package, &none, "f", None);
    let TerminalValue::Declined { cause, .. } = terminal(&record) else {
        panic!("declined, not {:?}", record.settlement)
    };
    assert_eq!(cause.as_str(), "invalid-input");
}

#[trace("QSpec-TC-973", "QSpec-FR-484-AC-8")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-6")]
#[test]
fn an_unbounded_integer_parameter_needs_a_bound() {
    let package = package_of("function subject using v(n: Integer): Boolean pure { n + 1 > n }");
    let key = declaration(&package, "subject");
    let record = run(&package, &key, "subject", None);
    let Settlement::Disposed(disposed) = &record.settlement else {
        panic!("a disposition, not {:?}", record.settlement)
    };
    assert_eq!(disposed.kind, DispositionKind::Unsupported);
    // The same claim with an integer-range bound is bounded and proved.
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
            }),
        )],
        &Cancel::new(),
    )
    .remove(0);
    assert!(matches!(terminal(&record), TerminalValue::Proved { .. }));
}

#[trace("QSpec-TC-973", "QSpec-FR-484-AC-3")]
#[test]
fn a_parameter_the_arm_does_not_enumerate_is_unsupported_not_proved() {
    let package = package_of("function subject using v(t: String): Boolean pure { t = t }");
    let key = declaration(&package, "subject");
    let record = run(&package, &key, "subject", None);
    assert!(
        matches!(terminal(&record), TerminalValue::Unsupported(_)),
        "{:?}",
        record.settlement
    );
}

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-7")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-8")]
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
    let key = declaration(&package, "subject");
    let request = ValueTarget::select(&package, "subject", &key).map(|target| ValueRequest {
        target,
        max_cases: None,
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

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-6")]
#[test]
fn negotiated_refusals_run_no_arm() {
    use qsl_route::routing::Disposition;
    let package = package_of("function subject using v(x: Int[0, 3]): Boolean pure { x >= 0 }");
    let key = declaration(&package, "subject");
    for (disposition, kind) in [
        (Disposition::RequiresBound, DispositionKind::RequiresBound),
        (Disposition::Unsupported, DispositionKind::Unsupported),
        (Disposition::InvalidRequest, DispositionKind::InvalidRequest),
    ] {
        let request = ValueTarget::select(&package, "subject", &key).map(|target| ValueRequest {
            target,
            max_cases: None,
        });
        let item: AnalyzeItem<'_> = value_item(index(), request).disposed(disposition, None);
        let records = analyze(&Engines::qsl().unwrap(), vec![item], &Cancel::new());
        let Settlement::Disposed(disposed) = &records[0].settlement else {
            panic!()
        };
        assert_eq!(disposed.kind, kind);
    }
}
