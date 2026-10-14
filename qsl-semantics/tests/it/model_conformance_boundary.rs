// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-151: conformance through actual IR intake, assembly and clause
//! checking. Every test enters through `admit_unit` -> `assemble` ->
//! `check`; no caller states a checked fact, and the work assertions read the
//! meter the model admission carried into the conformance stage.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    CheckCause, CheckRefusal, ModelConformanceFailure, PackageDeclarations, RefinementObligation,
};
use qsl_semantics::model::accounting::{
    ChargePoint, Incomplete, LimitKind, ModelNormalizationLimits,
};
use qsl_semantics::model::intake::{admit_unit, package_input, UnitIntakeCause};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::normalize::ModelRefusalCause;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value};

use crate::model_operations::{
    config_unit_with_body, config_version_document, empty_frame, operation, parse_and_build,
};

const PACKAGE: &str = "example/config-version";
const NOOP: &str = "function noop using v(): Boolean pure { true }";

fn identity(path: &str) -> String {
    format!("ix://{PACKAGE}/{path}")
}

fn key(path: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: identity(path),
    }
}

fn origin(path: &str) -> Value {
    json!({"source": {"sourceIdentity": identity("spec"), "path": path,
        "startLine": 1, "startColumn": 1}})
}

fn object(name: &str, parents: &[&str], fields: Vec<Value>, operations: Vec<Value>) -> Value {
    json!({"identity": identity(name), "displayName": name,
        "kind": {"module": PACKAGE, "name": "object_type"}, "roles": [],
        "origin": origin(name), "constraints": [], "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": parents.iter().map(|parent| identity(parent)).collect::<Vec<_>>(),
        "fields": fields, "operations": operations, "relationships": []})
}

/// A field whose presence is stated apart from its multiplicity.
fn field_with_presence(
    owner: &str,
    name: &str,
    ty: &str,
    lower: u64,
    upper: Option<u64>,
    presence: &str,
    parent: Option<&str>,
) -> Value {
    let path = format!("{owner}/{name}");
    let mut value = json!({"identity": identity(&path), "name": name, "typeRef": identity(ty),
        "presence": presence, "nullable": false, "defaultKind": "none", "origin": origin(&path),
        "multiplicity": {"lower": lower, "ordered": false, "unique": true}});
    if let Some(upper) = upper {
        value["multiplicity"]["upper"] = json!(upper);
    }
    if let Some(parent) = parent {
        value["redefines"] = json!(identity(parent));
    }
    value
}

fn field(
    owner: &str,
    name: &str,
    ty: &str,
    lower: u64,
    upper: Option<u64>,
    parent: Option<&str>,
) -> Value {
    field_with_presence(
        owner,
        name,
        ty,
        lower,
        upper,
        if lower == 0 { "optional" } else { "required" },
        parent,
    )
}

fn scalar(template: &Value, name: &str, maximum: i64) -> Value {
    let mut value = template.clone();
    value["identity"] = json!(identity(name));
    value["displayName"] = json!(name);
    value["origin"] = origin(name);
    for constraint in value["constraints"]
        .as_array_mut()
        .expect("scalar constraints")
    {
        let keyword = constraint["keyword"]
            .as_str()
            .expect("bound keyword")
            .to_owned();
        constraint["identity"] = json!(identity(&format!("{name}/constraints/{keyword}")));
        constraint["origin"] = origin(name);
        if keyword == "max" {
            constraint["operands"]["value"] = json!(maximum.to_string());
        }
    }
    value
}

fn writer(owner: &str, writes: &[&str], parent: Option<&str>) -> Value {
    let mut value = operation(
        "set",
        json!([]),
        None,
        json!({
        "modifies": writes.iter().map(|path| identity(path)).collect::<Vec<_>>(),
        "creates": [], "deletes": []}),
    );
    value["identity"] = json!(identity(&format!("{owner}/set")));
    value["origin"] = origin(&format!("{owner}/set"));
    if let Some(parent) = parent {
        value["redefines"] = json!(identity(parent));
    }
    value
}

/// `owner/set` with typed parameters and result `(type, lower, upper)`.
fn signed_writer(
    owner: &str,
    params: &[(&str, u64, u64)],
    result: Option<(&str, u64, u64)>,
    modifies: &[&str],
    creates: &[&str],
    parent: Option<&str>,
) -> Value {
    let multiplicity = |lower: u64, upper: u64| json!({"lower": lower, "upper": upper, "ordered": false, "unique": true});
    let mut value = writer(owner, modifies, parent);
    value["frame"]["creates"] = json!(creates
        .iter()
        .map(|path| identity(path))
        .collect::<Vec<_>>());
    value["params"] = json!(params
        .iter()
        .enumerate()
        .map(|(position, (ty, lower, upper))| {
            let name = format!("p{}", position + 1);
            json!({"identity": identity(&format!("{owner}/set/{name}")), "name": name,
            "typeRef": identity(ty), "presence": "required", "nullable": false,
            "defaultKind": "none", "multiplicity": multiplicity(*lower, *upper),
            "origin": origin(&format!("{owner}/set/{name}"))})
        })
        .collect::<Vec<_>>());
    if let Some((ty, lower, upper)) = result {
        value["returns"] = json!({"typeRef": identity(ty), "nullable": false,
            "multiplicity": multiplicity(lower, upper)});
    }
    value
}

fn document(types: impl FnOnce(&Value) -> Vec<Value>) -> Vec<u8> {
    let base = config_version_document(
        operation("unused", json!([]), None, empty_frame()),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let mut value: Value = serde_json::from_slice(&base).expect("fixture JSON");
    let scalar = value["types"]
        .as_array()
        .expect("types")
        .iter()
        .find(|ty| ty["scalar"] == "integer")
        .expect("bound scalar")
        .clone();
    value["types"] = json!(types(&scalar));
    value["populations"] = json!([]);
    // The envelope declares a construct only for a kind some type uses.
    let used: std::collections::BTreeSet<String> = value["types"]
        .as_array()
        .expect("types")
        .iter()
        .filter_map(|ty| ty["kind"]["name"].as_str().map(str::to_owned))
        .collect();
    value["constructs"]
        .as_array_mut()
        .expect("constructs")
        .retain(|construct| {
            construct["kind"]["name"]
                .as_str()
                .is_some_and(|name| used.contains(name))
        });
    serde_json::to_vec(&value).expect("fixture encoding")
}

/// One run of a unit through admission, assembly and checking. `n` is the
/// model work the admission consumed before the conformance stage;
/// `budget` is the `work_units` limit the run was admitted under.
struct Run {
    n: u64,
    outcome: Result<(), Vec<CheckRefusal>>,
}

fn admit_n(unit: &str, packages: &BTreeMap<[u8; 32], Vec<u8>>) -> u64 {
    let built = parse_and_build(unit);
    admit_unit(
        &built.selections().models,
        packages,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the package admits")
    .consumed(LimitKind::WorkUnits)
}

/// The unit over `document` and `body`, with the stage allowed `extra`
/// units of model work beyond what admission consumed (`None`: unlimited).
fn run(document: &[u8], body: &str, extra: Option<u64>) -> Run {
    let (unit, packages) = config_unit_with_body(document, body);
    run_unit(&unit, &packages, extra)
}

fn run_unit(unit: &str, packages: &BTreeMap<[u8; 32], Vec<u8>>, extra: Option<u64>) -> Run {
    let n = admit_n(unit, packages);
    let limits = ModelNormalizationLimits {
        work_units: extra.map_or(u64::MAX, |extra| n + extra),
        ..ModelNormalizationLimits::UNLIMITED
    };
    let built = parse_and_build(unit);
    let selected = admit_unit(&built.selections().models, packages, limits)
        .expect("normalization fits before the conformance stage")
        .with_parsed_operation_clauses(|text| {
            qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
        })
        .expect("kept clauses parse");
    assert_eq!(
        selected.consumed(LimitKind::WorkUnits),
        n,
        "the admission operation's own work"
    );
    let raw = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-196", "fixture", "1"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("source");
    let outcome = PackageDeclarations::assemble(
        raw.source().reference().clone(),
        built,
        selected,
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {:?}", refusal.errors))
    .check(CheckingLimits::default())
    .map(|_| ());
    Run { n, outcome }
}

fn failures(refusals: &[CheckRefusal]) -> Vec<&ModelConformanceFailure> {
    refusals
        .iter()
        .map(|refusal| match &refusal.cause {
            CheckCause::ModelConformance(failure) => &**failure,
            other => panic!("a typed model conformance failure is required: {other:?}"),
        })
        .collect()
}

fn tuples(
    outcome: &Result<(), Vec<CheckRefusal>>,
) -> Result<(), Vec<(Code, Option<&'static str>)>> {
    outcome.as_ref().map(|_| ()).map_err(|refusals| {
        refusals
            .iter()
            .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
            .collect()
    })
}

fn check(document: &[u8], body: &str) -> Result<(), Vec<(Code, Option<&'static str>)>> {
    tuples(&run(document, body, None).outcome)
}

fn denial(refusals: &[CheckRefusal]) -> Incomplete {
    assert_eq!(
        refusals.len(),
        1,
        "an unfinished stage exposes only its denial: {refusals:?}"
    );
    match &refusals[0].cause {
        CheckCause::ModelIncomplete(incomplete) => *incomplete,
        other => panic!("an incomplete record is required: {other:?}"),
    }
}

/// The exact record the stage reports when `total - 1` of its `total` units
/// of work are available: `work_units`, the prefix spent, and the next
/// `conformance.axis` charge of 1.
fn assert_complete_at_total_and_exact_denial_one_less(
    document: &[u8],
    body: &str,
    total: u64,
    expect: impl FnOnce(&Result<(), Vec<CheckRefusal>>),
) {
    let complete = run(document, body, Some(total));
    expect(&complete.outcome);
    let n = complete.n;
    let short = run(document, body, Some(total - 1));
    let refusals = short.outcome.expect_err("the final charge does not fit");
    assert_eq!(
        (refusals[0].cause.code(), refusals[0].cause.cause()),
        (Code::ResourceExhausted, Some("insufficient-next-charge"))
    );
    assert_eq!(
        denial(&refusals),
        Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: n + total - 1,
            consumed: n + total - 1,
            next_charge: 1,
            charge_point: ChargePoint::ConformanceAxis,
        }
    );
}

// ------------------------------------------------------------------ R08

/// The six R08 fixtures always enter through actual document admission and
/// authored postcondition checking; callers supply no established facts.
fn refinement_fixture(row: usize) -> (Vec<u8>, &'static str) {
    let numeric = row >= 3;
    let narrowed_name = if numeric {
        "cs"
    } else if row == 2 {
        "xr"
    } else {
        "xb"
    };
    let bytes = document(|template| {
        vec![
            scalar(template, "Count", 9),
            scalar(template, "Small", 5),
            object(
                "A",
                &[],
                vec![
                    field_with_presence(
                        "A",
                        "x",
                        "A",
                        if row == 2 { 0 } else { 1 },
                        Some(1),
                        "optional",
                        None,
                    ),
                    field("A", "c", "Count", 1, Some(1), None),
                ],
                vec![writer("A", &["A/x", "A/c"], None)],
            ),
            object(
                "B",
                &["A"],
                vec![field_with_presence(
                    "B",
                    narrowed_name,
                    if numeric {
                        "Small"
                    } else if row == 2 {
                        "B"
                    } else {
                        "A"
                    },
                    if row == 2 { 0 } else { 1 },
                    Some(1),
                    if row == 2 { "optional" } else { "required" },
                    Some(if numeric { "A/c" } else { "A/x" }),
                )],
                if [1, 4, 5].contains(&row) {
                    vec![writer("B", &["A/x", "A/c"], Some("A/set"))]
                } else {
                    vec![]
                },
            ),
        ]
    });
    let body = match row {
        1 => "post QB using v on Config::B::set { present(self.xb) }",
        4 => "post QC using v on Config::B::set { self.cs <= 5 }",
        5 => "post QD using v on Config::B::set { self.cs <= 6 }",
        _ => NOOP,
    };
    (bytes, body)
}

pub(super) fn refinement_vector(row: usize) -> Result<(), Vec<(Code, Option<&'static str>)>> {
    let (bytes, body) = refinement_fixture(row);
    check(&bytes, body)
}

#[trace("FR-082-AC-4", "FR-082-AC-8", "QSpec-TC-196")]
#[test]
fn actual_postconditions_decide_all_six_refinement_vectors() {
    for (row, expected) in [
        (0, false),
        (1, true),
        (2, false),
        (3, false),
        (4, true),
        (5, false),
    ] {
        let result = refinement_vector(row);
        if expected {
            assert_eq!(result, Ok(()), "R08 row {row}");
        } else {
            assert_eq!(
                result,
                Err(vec![(
                    Code::UndefinedExpression,
                    Some("unproved-refinement")
                )]),
                "R08 row {row} must reach its conformance obligation"
            );
        }
    }
}

/// T9: the exact work of each R08 vector on the carried meter, its typed
/// payload, and the denial one unit short.
#[trace(
    "QSpec-FR-151-AC-11",
    "QSpec-FR-150-AC-10",
    "QSpec-TC-196",
    "FR-082-AC-4",
    "FR-082-AC-8"
)]
#[test]
fn r08_totals_on_the_carried_meter_with_typed_payloads() {
    for (row, total, obligation) in [
        (0, 3, Some(RefinementObligation::Presence)),
        (1, 12, None),
        (2, 4, Some(RefinementObligation::NoProofForm)),
        (3, 3, Some(RefinementObligation::Domain)),
        (4, 12, None),
        (5, 12, Some(RefinementObligation::Domain)),
    ] {
        let (bytes, body) = refinement_fixture(row);
        assert_complete_at_total_and_exact_denial_one_less(&bytes, body, total, |outcome| {
            let Some(obligation) = obligation else {
                assert!(outcome.is_ok(), "R08 row {row}: {outcome:?}");
                return;
            };
            let refusals = outcome
                .as_ref()
                .expect_err("the complete stage preserves the failed obligation");
            let failures = failures(refusals);
            assert_eq!(failures.len(), 1);
            let failure = failures[0];
            let member = if row >= 3 {
                "B/cs"
            } else if row == 2 {
                "B/xr"
            } else {
                "B/xb"
            };
            let parent = if row >= 3 { "A/c" } else { "A/x" };
            let writer = if row == 5 { "B/set" } else { "A/set" };
            assert_eq!(failure.member, key(member));
            assert_eq!(failure.parent, key(parent));
            assert_eq!(failure.writer, Some(key(writer)));
            assert_eq!(failure.obligation, Some(obligation));
            assert_eq!(failure.axis, "refinement");
            assert_eq!(failure.code, Code::UndefinedExpression);
            assert_eq!(failure.cause, ModelRefusalCause::UnprovedRefinement);
            assert_eq!(failure.member_origin, Some(origin(member)));
            assert_eq!(failure.parent_origin, Some(origin(parent)));
            assert_eq!(failure.writer_origin, Some(origin(writer)));
        });
    }
}

pub(super) fn unrelated_numeric_clause() {
    let (bytes, _) = refinement_fixture(4);
    assert_eq!(
        check(&bytes, "post Q using v on Config::B::set { self.cs >= 0 }"),
        Err(vec![(
            Code::UndefinedExpression,
            Some("unproved-refinement")
        )])
    );
}

pub(super) fn conjoined_numeric_clauses() {
    let bytes = document(|template| {
        let mut count = scalar(template, "Count", 9);
        for constraint in count["constraints"].as_array_mut().unwrap() {
            if constraint["keyword"] == "min" {
                constraint["operands"]["value"] = json!("-5");
            }
        }
        vec![
            count,
            scalar(template, "Small", 5),
            object(
                "A",
                &[],
                vec![field("A", "c", "Count", 1, Some(1), None)],
                vec![writer("A", &["A/c"], None)],
            ),
            object(
                "B",
                &["A"],
                vec![field("B", "cs", "Small", 1, Some(1), Some("A/c"))],
                vec![writer("B", &["A/c"], Some("A/set"))],
            ),
        ]
    });
    for clauses in [
        "post Lower using v on Config::B::set { self.cs >= 0 }\npost Upper using v on Config::B::set { self.cs <= 5 }",
        "post Upper using v on Config::B::set { self.cs <= 5 }\npost Lower using v on Config::B::set { self.cs >= 0 }",
    ] {
        assert_eq!(check(&bytes, clauses), Ok(()), "both real clauses establish [0,5]");
    }
    for clause in [
        "post Lower using v on Config::B::set { self.cs >= 0 }",
        "post Upper using v on Config::B::set { self.cs <= 5 }",
    ] {
        assert_eq!(
            check(&bytes, clause),
            Err(vec![(
                Code::UndefinedExpression,
                Some("unproved-refinement")
            )]),
            "neither checked clause alone proves the complete narrowed interval"
        );
    }
}

pub(super) fn malformed_numeric_domain() {
    let bytes = document(|template| {
        let mut count = scalar(template, "Count", 0);
        for constraint in count["constraints"].as_array_mut().unwrap() {
            if constraint["keyword"] == "min" {
                constraint["operands"]["value"] = json!("9");
            }
        }
        vec![
            count,
            object(
                "A",
                &[],
                vec![field("A", "c", "Count", 1, Some(1), None)],
                vec![],
            ),
        ]
    });
    let (unit, packages) = config_unit_with_body(&bytes, NOOP);
    let built = parse_and_build(&unit);
    let refusal = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect_err("a malformed domain refuses before any clause or conformance stage");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("a malformed domain cause is required: {refusal:?}");
    };
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert!(
        matches!(
            refusals[0].cause,
            ModelRefusalCause::IntakeMalformedDeclaration { .. }
        ),
        "{:?}",
        refusals[0].cause
    );
}

/// Same displayed member/operation names in another genuine selection
/// cannot donate checked facts to this selection's exposed writer.
pub(super) fn cross_selection_refinement_decoy(row: usize) {
    let numeric = row == 0;
    let bytes = document(|template| {
        vec![
            scalar(template, "Count", 9),
            scalar(template, "Small", 5),
            object(
                "A",
                &[],
                vec![field_with_presence(
                    "A",
                    "x",
                    if numeric { "Count" } else { "A" },
                    1,
                    Some(1),
                    if numeric { "required" } else { "optional" },
                    None,
                )],
                vec![writer("A", &["A/x"], None)],
            ),
            object(
                "B",
                &["A"],
                vec![field(
                    "B",
                    "xb",
                    if numeric { "Small" } else { "A" },
                    1,
                    Some(1),
                    Some("A/x"),
                )],
                vec![],
            ),
        ]
    });
    let mut decoy: Value = serde_json::from_slice(&bytes).unwrap();
    decoy["types"][3]["operations"] = json!([writer(
        "B",
        &[if row.is_multiple_of(2) { "A/x" } else { "B/xb" }],
        Some("A/set")
    )]);
    let mut proof_context = "B";
    let mut proof_operation = "set";
    let mut proof_field = "xb";
    match row {
        3 => {
            decoy["types"][3]["operations"][0]["identity"] = json!(identity("B/write"));
            decoy["types"][3]["operations"][0]["name"] = json!("write");
            proof_operation = "write";
        }
        4 => {
            decoy["types"].as_array_mut().unwrap().push(object(
                "C",
                &[],
                vec![field("C", "x", "A", 1, Some(1), None)],
                vec![writer("C", &["C/x"], None)],
            ));
        }
        5 => {
            proof_context = "A";
            proof_field = "x";
        }
        6 => {
            decoy["types"][3]["fields"][0]["identity"] = json!(identity("B/yn"));
            decoy["types"][3]["fields"][0]["name"] = json!("yn");
            proof_field = "yn";
        }
        _ => {}
    }
    let decoy = serde_json::to_vec(&decoy).unwrap();
    let decoy = String::from_utf8(decoy)
        .unwrap()
        .replace(PACKAGE, "example/other")
        .into_bytes();
    let first_digest = package_input([bytes.as_slice()])
        .into_keys()
        .next()
        .unwrap();
    let other_digest = package_input([decoy.as_slice()])
        .into_keys()
        .next()
        .unwrap();
    let packages = package_input([bytes.as_slice(), decoy.as_slice()]);
    let proof = if numeric {
        format!("self.{proof_field} <= 5")
    } else {
        format!("present(self.{proof_field})")
    };
    for reverse in [false, true] {
        let mut selections = [
            format!(
                "model Original = {PACKAGE:?} version \"1.0.0\" digest \"sha256-jcs:{}\";",
                hex(&first_digest)
            ),
            format!(
                "model Decoy = \"example/other\" version \"1.0.0\" digest \"sha256-jcs:{}\";",
                hex(&other_digest)
            ),
        ];
        if reverse {
            selections.reverse();
        }
        let unit = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n{}\npost Proof using v on Decoy::{proof_context}::{proof_operation} {{ {proof} }}\n", selections.join("\n"));
        let refused = run_unit(&unit, &packages, None)
            .outcome
            .expect_err("another selection's postcondition cannot prove the original's writer");
        let failures = failures(&refused);
        assert_eq!(failures.len(), 1, "the decoy's own proof succeeds");
        let failure = failures[0];
        assert_eq!(failure.member, key("B/xb"));
        assert_eq!(failure.parent, key("A/x"));
        assert_eq!(failure.writer, Some(key("A/set")));
        assert_eq!(
            failure.obligation,
            Some(if numeric {
                RefinementObligation::Domain
            } else {
                RefinementObligation::Presence
            })
        );
        assert_eq!(failure.cause, ModelRefusalCause::UnprovedRefinement);
        assert_eq!(failure.member_origin, Some(origin("B/xb")));
        assert_eq!(failure.writer_origin, Some(origin("A/set")));
    }
}

// ------------------------------------------------------------------ C05

fn chain(child_maximum: i64) -> Vec<u8> {
    document(|template| {
        vec![
            scalar(template, "DA", 9),
            scalar(template, "DB", 5),
            scalar(template, "DC", child_maximum),
            object(
                "A",
                &[],
                vec![field("A", "x", "DA", 1, Some(1), None)],
                vec![writer("A", &["A/x"], None)],
            ),
            object(
                "B",
                &["A"],
                vec![field("B", "xb", "DB", 1, Some(1), Some("A/x"))],
                vec![writer("B", &["B/xb"], Some("A/set"))],
            ),
            object(
                "C",
                &["B"],
                vec![field("C", "xc", "DC", 1, Some(1), Some("B/xb"))],
                vec![writer("C", &["C/xc"], Some("B/set"))],
            ),
        ]
    })
}

fn chain_body(guard: u32) -> String {
    format!(
        "post QB using v on Config::B::set {{ self.xb <= {guard} }}\n\
        post QC using v on Config::C::set {{ self.xc <= 3 }}"
    )
}

/// T2: C05(a), a child narrowing its immediate parent which narrows its own.
#[trace("FR-082-AC-4", "QSpec-TC-196")]
#[test]
fn c05a_immediate_parent_chain_admits() {
    assert_eq!(check(&chain(3), &chain_body(5)), Ok(()));
}

/// T1: C05(b), a child that widens its immediate parent reports one
/// `variance-result` at the conformance stage, never a generic assembly
/// `RedefinitionWidens`.
#[trace("FR-082-AC-4", "QSpec-TC-196")]
#[test]
fn c05b_widened_child_reports_variance_result_not_redefinition_widens() {
    let refusals = run(&chain(7), &chain_body(5), None)
        .outcome
        .expect_err("a child that widens its immediate parent refuses");
    let failures = failures(&refusals);
    assert_eq!(failures.len(), 1);
    let failure = failures[0];
    assert_eq!(
        (failure.code, failure.cause.clone()),
        (Code::IllTyped, ModelRefusalCause::VarianceResult)
    );
    assert_eq!(refusals[0].cause.cause(), Some("variance-result"));
    assert_eq!(failure.axis, "value-type");
    assert_eq!(failure.member, key("C/xc"));
    assert_eq!(failure.parent, key("B/xb"));
    assert_eq!(failure.writer, None);
    assert!(
        failure.detail.contains(&identity("DC")) && failure.detail.contains(&identity("DB")),
        "the declared domains are named: {}",
        failure.detail
    );
}

/// T3: C05(c), the owning writer's guard decides the field domain: `B/set`
/// proving only `<= 6` leaves `B/xb` unproved against `A/x`, while `C` passes.
#[trace("FR-082-AC-4", "QSpec-TC-196")]
#[test]
fn c05c_owning_writer_b_set_refuses_field_domain() {
    let refusals = run(&chain(3), &chain_body(6), None)
        .outcome
        .expect_err("QB <= 6 does not prove [0,5]");
    let failures = failures(&refusals);
    assert_eq!(failures.len(), 1);
    let failure = failures[0];
    assert_eq!(
        (failure.code, failure.cause.clone()),
        (
            Code::UndefinedExpression,
            ModelRefusalCause::UnprovedRefinement
        )
    );
    assert_eq!(failure.member, key("B/xb"));
    assert_eq!(failure.parent, key("A/x"));
    assert_eq!(failure.writer, Some(key("B/set")));
    assert_eq!(failure.obligation, Some(RefinementObligation::Domain));
}

// ------------------------------------------------------------------ C04

fn presence_fixture(extra: Option<&str>) -> Vec<u8> {
    document(|_| {
        let mut types = vec![
            object(
                "A",
                &[],
                vec![field_with_presence(
                    "A",
                    "x",
                    "A",
                    1,
                    Some(1),
                    "optional",
                    None,
                )],
                vec![writer("A", &["A/x"], None)],
            ),
            object(
                "B",
                &["A"],
                vec![field("B", "xb", "A", 1, Some(1), Some("A/x"))],
                vec![writer("B", &["A/x"], Some("A/set"))],
            ),
        ];
        match extra {
            Some("sibling") => types.push(object(
                "S",
                &["A"],
                vec![],
                vec![writer("S", &["A/x"], Some("A/set"))],
            )),
            Some("descendant") => types.push(object(
                "D",
                &["B"],
                vec![],
                vec![writer("D", &["A/x"], Some("B/set"))],
            )),
            _ => {}
        }
        types
    })
}

/// T5: `present(self.xb)` types against the parent's optional presence, only
/// the owning writer's guard discharges the obligation, and a guard on a
/// sibling or descendant `set` does not.
#[trace("QSpec-FR-151-AC-13", "FR-082-AC-4", "QSpec-TC-196")]
#[test]
fn c04_present_types_from_optional_parent_and_owner_only() {
    let guard = "post QB using v on Config::B::set { present(self.xb) }";
    assert_eq!(check(&presence_fixture(None), guard), Ok(()));
    for (extra, body) in [
        (None, NOOP.to_owned()),
        (
            Some("sibling"),
            "post QS using v on Config::S::set { present(self.x) }".to_owned(),
        ),
        (
            Some("descendant"),
            "post QD using v on Config::D::set { true }".to_owned(),
        ),
    ] {
        let refusals = run(&presence_fixture(extra), &body, None)
            .outcome
            .expect_err("only the owning writer's guard discharges field presence");
        let failures = failures(&refusals);
        assert_eq!(failures.len(), 1, "{extra:?}");
        assert_eq!(
            failures[0].obligation,
            Some(RefinementObligation::Presence),
            "{extra:?}"
        );
        assert_eq!(failures[0].member, key("B/xb"));
        assert_eq!(failures[0].parent, key("A/x"));
    }
}

// ------------------------------------------------------------------ C01

fn six_failures(reverse: bool) -> Vec<u8> {
    document(|_| {
        let mut types = vec![
            object(
                "A",
                &[],
                vec![field_with_presence(
                    "A",
                    "x",
                    "A",
                    1,
                    Some(1),
                    "optional",
                    None,
                )],
                vec![signed_writer(
                    "A",
                    &[("A", 0, 1)],
                    Some(("B", 1, 1)),
                    &["A/x"],
                    &[],
                    None,
                )],
            ),
            object(
                "B",
                &["A"],
                vec![
                    field("B", "xb", "A", 1, Some(1), Some("A/x")),
                    field("B", "y", "A", 0, Some(1), None),
                ],
                vec![signed_writer(
                    "B",
                    &[("B", 1, 1)],
                    Some(("A", 1, 1)),
                    &["A/x", "B/y"],
                    &[],
                    Some("A/set"),
                )],
            ),
        ];
        if reverse {
            types.reverse();
        }
        types
    })
}

/// T6: all static and refinement failures, exhaustively, in member then axis
/// order, identical after reversing the document's node order. A result
/// multiplicity wider than the parent's never reaches the stage: the
/// Semantic IR reader refuses that redefinition first.
#[trace("QSpec-FR-151-AC-12", "FR-082-AC-1", "FR-082-AC-8", "QSpec-TC-196")]
#[test]
fn c01_r03_and_r08a_report_every_failure_in_member_axis_order() {
    let mut baseline: Option<(Vec<String>, u64)> = None;
    for reverse in [false, true] {
        let bytes = six_failures(reverse);
        let complete = run(&bytes, NOOP, None);
        let refusals = complete.outcome.expect_err("six failures");
        let failures = failures(&refusals);
        let causes: Vec<_> = failures
            .iter()
            .map(|failure| failure.cause.clone())
            .collect();
        assert_eq!(failures.len(), 5, "{causes:?}");
        assert!(
            matches!(
                causes[0],
                ModelRefusalCause::VarianceParameter { index: 1, .. }
            ),
            "{causes:?}"
        );
        assert!(matches!(
            causes[1],
            ModelRefusalCause::MultiplicityNarrowing { .. }
        ));
        assert_eq!(causes[2], ModelRefusalCause::VarianceResult);
        assert!(
            matches!(&causes[3], ModelRefusalCause::EffectEscape { field } if *field == key("B/y"))
        );
        assert_eq!(causes[4], ModelRefusalCause::UnprovedRefinement);
        assert!(failures[..4]
            .iter()
            .all(|failure| failure.member == key("B/set") && failure.parent == key("A/set")));
        assert_eq!(failures[4].member, key("B/xb"));
        assert_eq!(failures[4].obligation, Some(RefinementObligation::Presence));
        // The work of the whole stage: one unit short denies, the total admits.
        let mut total = 1;
        while run(&bytes, NOOP, Some(total))
            .outcome
            .as_ref()
            .err()
            .is_some_and(|refusals| matches!(refusals[0].cause, CheckCause::ModelIncomplete(_)))
        {
            total += 1;
        }
        let signature: Vec<String> = failures
            .iter()
            .map(|failure| format!("{:?}/{}", failure.cause, failure.axis))
            .collect();
        match &baseline {
            Some(expected) => assert_eq!(&(signature, total), expected, "reversed node order"),
            None => baseline = Some((signature, total)),
        }
    }
}

/// T7: a source refusal wins over the conformance stage, which then charges
/// nothing: the run admitted with exactly the model work admission used
/// still reports the source refusal.
#[trace("QSpec-FR-151-AC-14", "FR-082-AC-17", "QSpec-TC-196")]
#[test]
fn c02_unresolved_clause_name_precedes_conformance() {
    let bytes = six_failures(false);
    for (body, code, cause) in [
        (
            "post Q using v on Config::B::set { missing }",
            Code::MissingDeclaration,
            "missing-name",
        ),
        (
            "post Q using v on Config::B::set { 1 }",
            Code::IllTyped,
            "non-boolean-root",
        ),
    ] {
        let refused = run(&bytes, body, Some(0))
            .outcome
            .expect_err("the source refusal is reported");
        assert_eq!(refused.len(), 1, "{body}: {refused:?}");
        assert_eq!(
            (refused[0].cause.code(), refused[0].cause.cause()),
            (code, Some(cause)),
            "{body}"
        );
        assert!(!matches!(
            refused[0].cause,
            CheckCause::ModelIncomplete(_) | CheckCause::ModelConformance(_)
        ));
    }
}

// ------------------------------------------------------------------ R02-R09

/// Fixture H of the operation vectors: types `A`, `B` (`B` <= `A`); `A/x` and
/// `B/y` typed `A` `{0,1}`; `A/op(p1: A {0,1}): B {1,1}`.
fn h_vector(row: usize) -> Vec<u8> {
    document(|_| {
        let mut b_fields = vec![field("B", "y", "A", 0, Some(1), None)];
        if row == 9 {
            b_fields.push(field("B", "w", "A", 0, Some(1), Some("A/x")));
        }
        let child = match row {
            2 => signed_writer(
                "B",
                &[("A", 0, 2)],
                Some(("B", 1, 1)),
                &[],
                &["B"],
                Some("A/set"),
            ),
            3 => signed_writer(
                "B",
                &[("B", 1, 1)],
                Some(("A", 1, 1)),
                &["B/y"],
                &[],
                Some("A/set"),
            ),
            4 => signed_writer("B", &[], Some(("B", 1, 1)), &[], &["B"], Some("A/set")),
            _ => signed_writer(
                "B",
                &[("A", 0, 2)],
                Some(("B", 1, 1)),
                &["B/w"],
                &[],
                Some("A/set"),
            ),
        };
        vec![
            object(
                "A",
                &[],
                vec![field("A", "x", "A", 0, Some(1), None)],
                vec![signed_writer(
                    "A",
                    &[("A", 0, 1)],
                    Some(("B", 1, 1)),
                    &["A/x"],
                    &["A"],
                    None,
                )],
            ),
            object("B", &["A"], b_fields, vec![child]),
        ]
    })
}

/// The least model work the stage needs to complete: one unit short of it
/// denies.
fn stage_total(document: &[u8], body: &str) -> u64 {
    let mut total = 0;
    while run(document, body, Some(total))
        .outcome
        .as_ref()
        .err()
        .is_some_and(|refusals| matches!(refusals[0].cause, CheckCause::ModelIncomplete(_)))
    {
        total += 1;
    }
    total
}

/// T8: the operation vectors' totals on the carried meter, and the exact
/// denial one unit short.
#[trace(
    "QSpec-FR-150-AC-10",
    "QSpec-FR-151-AC-11",
    "FR-082-AC-17",
    "QSpec-TC-196"
)]
#[test]
fn r02_r03_r04_r09_totals_on_the_carried_meter() {
    for (row, total) in [(2, 10), (3, 8), (4, 8), (9, 10)] {
        let bytes = h_vector(row);
        // R09 also redefines a field (`B/w` over `A/x`), a pair the stage
        // prices on its own beside the operation's 10.
        let total = if row == 9 {
            let field_pair = stage_total(
                &document(|_| {
                    vec![
                        object(
                            "A",
                            &[],
                            vec![field("A", "x", "A", 0, Some(1), None)],
                            vec![],
                        ),
                        object(
                            "B",
                            &["A"],
                            vec![field("B", "w", "A", 0, Some(1), Some("A/x"))],
                            vec![],
                        ),
                    ]
                }),
                NOOP,
            );
            total + field_pair
        } else {
            total
        };
        assert_complete_at_total_and_exact_denial_one_less(
            &bytes,
            NOOP,
            total,
            |outcome| match row {
                3 => {
                    let refusals = outcome.as_ref().expect_err("R03 reports its failing axes");
                    assert_eq!(failures(refusals).len(), 4);
                }
                4 => assert_eq!(
                    tuples(outcome),
                    Err(vec![(Code::IllTyped, Some("type-mismatch"))])
                ),
                _ => assert_eq!(tuples(outcome), Ok(()), "R0{row}"),
            },
        );
    }
}

/// T4 (R05): a parameter multiplicity narrower than its parent's reaches the
/// multiplicity-narrowing cause at the conformance stage. The field direction
/// ({1,3} redefined by {0,5}) is refused by the Semantic IR reader before it
/// reaches the stage.
#[trace("QSpec-FR-151-AC-14", "QSpec-TC-196")]
#[test]
fn c03_r05_multiplicity_narrowing_reaches_its_fr151_cause() {
    let bytes = document(|_| {
        vec![
            object(
                "A",
                &[],
                vec![],
                vec![signed_writer("A", &[("A", 0, 5)], None, &[], &[], None)],
            ),
            object(
                "B",
                &["A"],
                vec![],
                vec![signed_writer(
                    "B",
                    &[("A", 1, 3)],
                    None,
                    &[],
                    &[],
                    Some("A/set"),
                )],
            ),
        ]
    });
    let refusals = run(&bytes, NOOP, None)
        .outcome
        .expect_err("a narrower parameter multiplicity refuses");
    let failures = failures(&refusals);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].code, Code::IllTyped);
    let ModelRefusalCause::MultiplicityNarrowing { from, to } = &failures[0].cause else {
        panic!("{:?}", failures[0].cause);
    };
    assert_eq!((from.lower, from.upper), (0, Some(5)));
    assert_eq!((to.lower, to.upper), (1, Some(3)));
    assert_eq!(failures[0].member, key("B/set"));
    assert_eq!(failures[0].parent, key("A/set"));
}

/// T4 (R08(c)): an object-typed narrowing has no proof form, and it is
/// reached as that cause rather than as a generic reference widening.
#[trace("QSpec-FR-151-AC-14", "QSpec-TC-196")]
#[test]
fn c03_r08c_object_typed_narrowing_reaches_no_proof_form() {
    let (bytes, body) = refinement_fixture(2);
    let refusals = run(&bytes, body, None).outcome.expect_err("no proof form");
    let failures = failures(&refusals);
    assert_eq!(failures.len(), 1);
    assert_eq!(
        (failures[0].code, failures[0].cause.clone()),
        (
            Code::UndefinedExpression,
            ModelRefusalCause::UnprovedRefinement
        )
    );
    assert_eq!(
        failures[0].obligation,
        Some(RefinementObligation::NoProofForm)
    );
    assert_eq!(failures[0].member, key("B/xr"));
    assert_eq!(failures[0].writer, Some(key("A/set")));
}

#[trace("FR-082-AC-16", "QSpec-TC-196", "QSpec-TC-198")]
#[test]
fn reference_covariance_uses_the_complete_model_without_a_writer_obligation() {
    for (parent_type, child_type, compatible) in
        [("A", "B", true), ("B", "A", false), ("A", "C", false)]
    {
        let bytes = document(|_| {
            vec![
                object(
                    "A",
                    &[],
                    vec![field("A", "x", parent_type, 1, Some(1), None)],
                    vec![],
                ),
                object(
                    "B",
                    &["A"],
                    vec![field("B", "xb", child_type, 1, Some(1), Some("A/x"))],
                    vec![],
                ),
                object("C", &[], vec![], vec![]),
            ]
        });
        let expected = if compatible {
            Ok(())
        } else {
            Err(vec![(Code::IllTyped, Some("variance-result"))])
        };
        assert_eq!(
            check(&bytes, NOOP),
            expected,
            "Reference<{child_type}> redefines Reference<{parent_type}>"
        );
    }
}

// ------------------------------------------------------------------ C06 / C07

fn two_model_unit(reverse: bool) -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    let bytes = document(|_| {
        vec![
            object(
                "A",
                &[],
                vec![field("A", "x", "A", 1, Some(1), None)],
                vec![],
            ),
            object(
                "B",
                &["A"],
                vec![field("B", "x", "C", 1, Some(1), Some("A/x"))],
                vec![],
            ),
            object("C", &[], vec![], vec![]),
        ]
    });
    let second = String::from_utf8(bytes.clone())
        .expect("fixture JSON")
        .replace(PACKAGE, "example/other")
        .into_bytes();
    let input = package_input([bytes.as_slice(), second.as_slice()]);
    let first_digest = package_input([bytes.as_slice()])
        .into_keys()
        .next()
        .expect("first digest");
    let second_digest = package_input([second.as_slice()])
        .into_keys()
        .next()
        .expect("second digest");
    let mut declarations = [
        format!(
            "model Z = {PACKAGE:?} version \"1.0.0\" digest \"sha256-jcs:{}\";",
            hex(&first_digest)
        ),
        format!(
            "model A = \"example/other\" version \"1.0.0\" digest \"sha256-jcs:{}\";",
            hex(&second_digest)
        ),
    ];
    if reverse {
        declarations.reverse();
    }
    (format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n{}\n{NOOP}\n",
        declarations.join("\n")), input)
}

/// T10: two selections report in selection byte order whatever the alias or
/// source order, with identical totals.
#[trace("QSpec-FR-151-AC-12", "FR-082-AC-17", "QSpec-TC-196")]
#[test]
fn c06_two_selections_canonical_order() {
    let mut totals = None;
    for reverse in [false, true] {
        let (unit, packages) = two_model_unit(reverse);
        let refusals = run_unit(&unit, &packages, None)
            .outcome
            .expect_err("both selections fail");
        let order: Vec<_> = failures(&refusals)
            .iter()
            .map(|failure| {
                (
                    failure.member.package.clone(),
                    failure.member.node.clone(),
                    failure.axis,
                )
            })
            .collect();
        assert_eq!(
            order,
            vec![
                (PACKAGE.to_owned(), identity("B/x"), "value-type"),
                (
                    "example/other".to_owned(),
                    "ix://example/other/B/x".to_owned(),
                    "value-type"
                ),
            ]
        );
        let mut total = 1;
        while run_unit(&unit, &packages, Some(total))
            .outcome
            .as_ref()
            .err()
            .is_some_and(|refusals| matches!(refusals[0].cause, CheckCause::ModelIncomplete(_)))
        {
            total += 1;
        }
        match totals {
            Some(expected) => assert_eq!(
                total, expected,
                "identical totals whatever the source order"
            ),
            None => totals = Some(total),
        }
    }
}

/// T11: a denial in the second selection withholds the first selection's
/// completed failures and does not reset the carried meter.
#[trace("QSpec-FR-151-AC-15", "FR-082-AC-17", "QSpec-TC-196")]
#[test]
fn c07_second_selection_denial_withholds_first_failures() {
    let (unit, packages) = two_model_unit(true);
    let full = run_unit(&unit, &packages, None);
    let mut total = 1;
    while run_unit(&unit, &packages, Some(total))
        .outcome
        .as_ref()
        .err()
        .is_some_and(|refusals| matches!(refusals[0].cause, CheckCause::ModelIncomplete(_)))
    {
        total += 1;
    }
    assert!(failures(&full.outcome.expect_err("both fail")).len() == 2);
    let refused = run_unit(&unit, &packages, Some(total - 1))
        .outcome
        .expect_err("the last charge is denied");
    let incomplete = denial(&refused);
    assert_eq!(
        incomplete,
        Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: full.n + total - 1,
            consumed: full.n + total - 1,
            next_charge: 1,
            charge_point: ChargePoint::ConformanceAxis,
        }
    );
}

// ------------------------------------------------------------------ misc

#[trace("FR-082-AC-4", "FR-104", "QSpec-TC-196")]
#[test]
fn admitted_clause_failures_precede_the_conformance_stage() {
    let bytes = chain(3);
    for (expression, code, cause) in [
        ("missing", Code::MissingDeclaration, "missing-name"),
        ("1", Code::IllTyped, "non-boolean-root"),
    ] {
        let body = format!("post QB using v on Config::B::set {{ {expression} }}");
        assert_eq!(
            check(&bytes, &body),
            Err(vec![(code, Some(cause))]),
            "{expression}"
        );
    }
}

#[trace("FR-082-AC-4", "FR-104", "QSpec-TC-196")]
#[test]
fn an_unrelated_or_removed_guard_cannot_prove_an_owning_writer() {
    for (body, extra_owner) in [
        ("post QB using v on Config::B::set { true }", None),
        (
            "post QS using v on Config::S::set { self.xs <= 5 }",
            Some("S"),
        ),
        (
            "post QC using v on Config::C::set { self.xc <= 3 }",
            Some("C"),
        ),
    ] {
        let bytes = document(|template| {
            let mut types = vec![
                scalar(template, "Count", 9),
                scalar(template, "Small", 5),
                scalar(template, "Tiny", 3),
                object(
                    "A",
                    &[],
                    vec![field("A", "x", "Count", 1, Some(1), None)],
                    vec![writer("A", &["A/x"], None)],
                ),
                object(
                    "B",
                    &["A"],
                    vec![field("B", "xb", "Small", 1, Some(1), Some("A/x"))],
                    vec![writer("B", &["B/xb"], Some("A/set"))],
                ),
            ];
            if let Some(owner) = extra_owner {
                let sibling = owner == "S";
                types.push(object(
                    owner,
                    &[if sibling { "A" } else { "B" }],
                    vec![field(
                        owner,
                        if sibling { "xs" } else { "xc" },
                        if sibling { "Small" } else { "Tiny" },
                        1,
                        Some(1),
                        Some(if sibling { "A/x" } else { "B/xb" }),
                    )],
                    vec![writer(
                        owner,
                        &[if sibling { "S/xs" } else { "C/xc" }],
                        Some(if sibling { "A/set" } else { "B/set" }),
                    )],
                ));
            }
            types
        });
        assert_eq!(
            check(&bytes, body),
            Err(vec![(
                Code::UndefinedExpression,
                Some("unproved-refinement")
            )]),
            "guard={body}"
        );
    }
}

#[trace("FR-082-AC-2", "QSpec-TC-196")]
#[test]
fn contested_field_and_operation_targets_refuse_at_intake_with_one_structural_authority() {
    for operation_contest in [false, true] {
        let bytes = document(|_| {
            if operation_contest {
                let first = writer("B", &[], Some("A/set"));
                let mut second = first.clone();
                second["identity"] = json!(identity("B/set2"));
                second["name"] = json!("set2");
                second["origin"] = origin("B/set2");
                vec![
                    object("A", &[], vec![], vec![writer("A", &[], None)]),
                    object("B", &["A"], vec![], vec![first, second]),
                ]
            } else {
                vec![
                    object(
                        "A",
                        &[],
                        vec![field("A", "x", "A", 1, Some(1), None)],
                        vec![],
                    ),
                    object(
                        "B",
                        &["A"],
                        vec![
                            field("B", "xb", "A", 1, Some(1), Some("A/x")),
                            field("B", "xb2", "A", 1, Some(1), Some("A/x")),
                        ],
                        vec![],
                    ),
                ]
            }
        });
        let (unit, packages) = config_unit_with_body(&bytes, NOOP);
        let built = parse_and_build(&unit);
        let refused = admit_unit(
            &built.selections().models,
            &packages,
            ModelNormalizationLimits::UNLIMITED,
        )
        .expect_err("two redefiners of one target are contested");
        let UnitIntakeCause::Refused(refusals) = refused.cause else {
            panic!("{refused:?}")
        };
        assert_eq!(refusals[0].code, Code::InvalidModelBinding);
        assert!(
            matches!(
                refusals[0].cause,
                ModelRefusalCause::RedefinitionTarget { .. }
            ),
            "operations={operation_contest}: {:?}",
            refusals[0].cause
        );
    }
}

/// FR150 permits subsetting and redefinition together: only redefinition
/// replaces the target. Equal input keys from different rules are distinct
/// facts.
#[trace("FR-082-AC-2", "FR-082-AC-3", "QSpec-TC-196", "QSpec-TC-213")]
#[test]
fn one_actual_member_can_subset_and_redefine_the_same_parent() {
    use qsl_semantics::model::key::{RULE_QUALIFY, RULE_REDEFINE, RULE_SUBSET};
    for reverse in [false, true] {
        let bytes = document(|_| {
            let mut child = field("B", "xb", "A", 1, Some(1), Some("A/x"));
            child["subsets"] = json!([identity("A/x")]);
            let mut types = vec![
                object(
                    "A",
                    &[],
                    vec![field("A", "x", "A", 1, Some(1), None)],
                    vec![],
                ),
                object("B", &["A"], vec![child], vec![]),
            ];
            if reverse {
                types.reverse();
            }
            types
        });
        let (unit, packages) = config_unit_with_body(&bytes, NOOP);
        let built = parse_and_build(&unit);
        let selected = admit_unit(
            &built.selections().models,
            &packages,
            ModelNormalizationLimits::UNLIMITED,
        )
        .expect("different rules retain equal inputs without a duplicate-path refusal");
        let view = &selected[0].view;
        let owner = view.type_identities()[&key("B")];
        let entry = view
            .declarations()
            .iter()
            .find(|entry| {
                entry.preimage.owner_effective_type == Some(owner)
                    && entry.preimage.original == key("B/xb")
            })
            .expect("the member's effective entry");
        let rules: Vec<_> = entry
            .preimage
            .derivation
            .iter()
            .map(|fact| fact.rule.clone())
            .collect();
        assert_eq!(
            rules,
            vec![RULE_QUALIFY, RULE_SUBSET, RULE_REDEFINE],
            "reverse={reverse}"
        );
    }
}

// ------------------------------------------------------------------ lineages

/// `A/x` redefined by `B1/y` (<= 5) and `B2/z` (<= 7); `C` inherits both and
/// `C/w` redefines `A/x` itself.
fn diamond(c_maximum: i64, reverse: bool) -> Vec<u8> {
    document(|template| {
        let mut types = vec![
            scalar(template, "DA", 9),
            scalar(template, "D1", 5),
            scalar(template, "D2", 7),
            scalar(template, "DC", c_maximum),
            object(
                "A",
                &[],
                vec![field("A", "x", "DA", 1, Some(1), None)],
                vec![],
            ),
            object(
                "B1",
                &["A"],
                vec![field("B1", "y", "D1", 1, Some(1), Some("A/x"))],
                vec![],
            ),
            object(
                "B2",
                &["A"],
                vec![field("B2", "z", "D2", 1, Some(1), Some("A/x"))],
                vec![],
            ),
            object(
                "C",
                &["B1", "B2"],
                vec![field("C", "w", "DC", 1, Some(1), Some("A/x"))],
                vec![],
            ),
        ];
        if reverse {
            types.reverse();
        }
        types
    })
}

/// A redefinition whose target is reached through two direct supertypes is
/// judged against the member of each: the direct supertype the lineage came
/// down is the first edge the staged admission selects the parent by, and both
/// verdicts appear, identically whatever the document's node order.
#[trace(
    "QSpec-FR-151-AC-12",
    "QSpec-FR-110-AC-5",
    "FR-082-AC-17",
    "QSpec-TC-196"
)]
#[test]
fn diamond_redefinition_walks_every_lineage_and_reports_each_verdict() {
    for reverse in [false, true] {
        assert_eq!(
            check(&diamond(3, reverse), NOOP),
            Ok(()),
            "within both parents, reverse={reverse}"
        );
        for (maximum, failing) in [(6, vec!["B1/y"]), (8, vec!["B1/y", "B2/z"])] {
            let refusals = run(&diamond(maximum, reverse), NOOP, None)
                .outcome
                .expect_err("a failing lineage refuses the axis");
            let failures = failures(&refusals);
            assert_eq!(
                failures
                    .iter()
                    .map(|failure| failure.parent.clone())
                    .collect::<Vec<_>>(),
                failing.iter().map(|parent| key(parent)).collect::<Vec<_>>(),
                "max={maximum}, reverse={reverse}"
            );
            assert!(failures.iter().all(|failure| failure.member == key("C/w")
                && failure.cause == ModelRefusalCause::VarianceResult
                && failure.axis == "value-type"));
        }
    }
}

/// FR-110-AC-3: the staged admission spends the work strict admission spends.
#[trace("QSpec-FR-110-AC-3", "QSpec-TC-196")]
#[test]
fn staged_admission_spends_the_work_of_strict_admission() {
    for bytes in [chain(3), diamond(3, false)] {
        let (unit, packages) = config_unit_with_body(&bytes, NOOP);
        let built = parse_and_build(&unit);
        let selected = admit_unit(
            &built.selections().models,
            &packages,
            ModelNormalizationLimits::UNLIMITED,
        )
        .expect("admits");
        let raw = qsl_cst::parse(
            qsl_foundation::SourceIdentity::new("test", "tc-196", "fixture", "1"),
            "unit.native",
            unit.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("source");
        let declarations = PackageDeclarations::assemble(
            raw.source().reference().clone(),
            built,
            selected,
            Vec::new(),
        )
        .expect("assembles");
        assert_eq!(
            declarations.staged_work_units(),
            Some(declarations.types.spent())
        );
        assert!(declarations.types.spent() > 0);
    }
}

// ------------------------------------------------------------------ R05 fields

fn multiplicity_chain(rows: &[(u64, Option<u64>)]) -> Vec<u8> {
    document(|_| {
        let names = ["A", "B", "C"];
        let mut types = Vec::new();
        for (index, (lower, upper)) in rows.iter().enumerate() {
            let owner = names[index];
            let parent = (index > 0).then(|| format!("{}/n", names[index - 1]));
            let parents: Vec<&str> = if index > 0 {
                vec![names[index - 1]]
            } else {
                vec![]
            };
            types.push(object(
                owner,
                &parents,
                vec![field_with_presence(
                    owner,
                    "n",
                    "A",
                    *lower,
                    *upper,
                    "optional",
                    parent.as_deref(),
                )],
                vec![],
            ));
        }
        types
    })
}

/// R05: a field redefinition whose multiplicity stays within its target's
/// bounds admits, for bounded, unbounded and chained targets, whatever the
/// order of the document's nodes.
#[trace("QSpec-FR-151-AC-2", "FR-082-AC-4", "QSpec-TC-196")]
#[test]
fn r05_field_multiplicity_within_the_target_admits() {
    for rows in [
        vec![(0, Some(5)), (1, Some(3))],
        vec![(0, None), (0, Some(5))],
        vec![(0, None), (0, Some(9)), (1, Some(3))],
        vec![(0, Some(5)), (0, Some(5))],
    ] {
        assert_eq!(check(&multiplicity_chain(&rows), NOOP), Ok(()), "{rows:?}");
    }
}

/// Only collection cardinality is narrowed away: a redefinition that also
/// widens the element, the presence or the referenced type still refuses.
#[trace("QSpec-FR-151-AC-2", "FR-082-AC-4", "QSpec-TC-196")]
#[test]
fn r05_field_multiplicity_narrowing_does_not_hide_other_widenings() {
    let widened_element = document(|template| {
        vec![
            scalar(template, "DA", 5),
            scalar(template, "DB", 9),
            object(
                "A",
                &[],
                vec![field_with_presence(
                    "A",
                    "n",
                    "DA",
                    0,
                    Some(5),
                    "optional",
                    None,
                )],
                vec![],
            ),
            object(
                "B",
                &["A"],
                vec![field_with_presence(
                    "B",
                    "n",
                    "DB",
                    1,
                    Some(3),
                    "optional",
                    Some("A/n"),
                )],
                vec![],
            ),
        ]
    });
    let widened_presence = document(|_| {
        vec![
            object(
                "A",
                &[],
                vec![field_with_presence(
                    "A",
                    "n",
                    "A",
                    0,
                    Some(5),
                    "required",
                    None,
                )],
                vec![],
            ),
            object(
                "B",
                &["A"],
                vec![field_with_presence(
                    "B",
                    "n",
                    "A",
                    1,
                    Some(3),
                    "optional",
                    Some("A/n"),
                )],
                vec![],
            ),
        ]
    });
    let widened_reference = document(|_| {
        vec![
            object(
                "A",
                &[],
                vec![field_with_presence(
                    "A",
                    "n",
                    "B",
                    0,
                    Some(5),
                    "optional",
                    None,
                )],
                vec![],
            ),
            object(
                "B",
                &["A"],
                vec![field_with_presence(
                    "B",
                    "n",
                    "A",
                    1,
                    Some(3),
                    "optional",
                    Some("A/n"),
                )],
                vec![],
            ),
        ]
    });
    for (name, bytes, cause) in [
        ("element", widened_element, "variance-result"),
        ("presence", widened_presence, "type-mismatch"),
        ("reference", widened_reference, "variance-result"),
    ] {
        let outcome = check(&bytes, NOOP);
        assert_eq!(outcome, Err(vec![(Code::IllTyped, Some(cause))]), "{name}");
    }
}

/// AC-8, second sentence: both fields required [0,1], or both optional [1,1],
/// written with no fact, admit and engage no obligation.
#[trace("FR-082-AC-8", "QSpec-TC-196")]
#[test]
fn equal_presence_and_equal_multiplicity_engage_no_refinement_obligation() {
    for (lower, upper, presence) in [(0, 1, "required"), (1, 1, "optional")] {
        let bytes = document(|_| {
            vec![
                object(
                    "A",
                    &[],
                    vec![field_with_presence(
                        "A",
                        "f",
                        "A",
                        lower,
                        Some(upper),
                        presence,
                        None,
                    )],
                    vec![writer("A", &["A/f"], None)],
                ),
                object(
                    "B",
                    &["A"],
                    vec![field_with_presence(
                        "B",
                        "g",
                        "A",
                        lower,
                        Some(upper),
                        presence,
                        Some("A/f"),
                    )],
                    vec![writer("B", &["A/f"], Some("A/set"))],
                ),
            ]
        });
        assert_eq!(check(&bytes, NOOP), Ok(()), "{presence} [{lower},{upper}]");
    }
}
