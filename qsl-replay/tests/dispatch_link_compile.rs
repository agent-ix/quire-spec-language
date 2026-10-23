// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-793, TC-795, TC-1011 (FR-302-AC-1, FR-302-AC-5, FR-302-AC-8): the spine
//! compile links the dispatch table of each operation a unit's dispatched
//! calls name, from the unit's own `operation-body` declarations and `pre`
//! clauses and the admitted model view. As an integration test it reaches
//! only the crate's public API.
//!
//! The model is the `example/config-version` package with `ConfigVersion`
//! (`A`) declaring the operations under test and subtypes `Sub` (`B`), `C`
//! and `D` each redefining `size`.

use std::collections::BTreeMap;

use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{
    AdmissionUsage, AdmittedObservations, DocumentRef, Observation,
};
use quire_eval::FamilyOutcome;
use quire_exact::{
    EffectiveId, FieldValue, Integer, LimitKind, Meter, ObjectId, ObjectReference, Outcome,
    UniverseId, Value as Quire,
};
use quire_semantic_value::object_closure::ObjectClosure;

use ix_trace_rs::trace;
use qsl_replay::spine::{self, CheckedUnit, CompileRefusal, ParseRequest, SpineLimits};
use qsl_replay::{Code, DependencyInput, SourceIdentity};
use qsl_semantics::check::{CheckCause, CheckedNode, NodeKind};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::package_input;
use quire_exact::Cancel;
use quire_semantic_value::location::{Location, Origin};
use serde_json::{json, Value};

const PACKAGE: &str = "example/config-version";
const EXAMPLE: &str = include_str!("../../examples/config-version/model.semantic-ir.json");

/// `owner.name(params): Integer`, redefining `redefines.name` when given.
fn query(owner: &str, name: &str, params: &[&str], redefines: Option<&str>) -> Value {
    let identity = format!("ix://{PACKAGE}/{owner}/{name}");
    let parameters: Vec<Value> = params
        .iter()
        .map(|parameter| {
            let identity = format!("{identity}/{parameter}");
            json!({
                "identity": identity,
                "name": parameter,
                "typeRef": "ix://quire/native/Integer",
                "presence": "required",
                "nullable": false,
                "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": {"generated": {
                    "generatorIdentity": identity,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [identity],
                }},
            })
        })
        .collect();
    let mut operation = json!({
        "identity": identity,
        "name": name,
        "params": parameters,
        "returns": {
            "typeRef": "ix://quire/native/Integer",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        },
        "pre": [],
        "post": [],
        "origin": {"source": {
            "sourceIdentity": format!("ix://{PACKAGE}/spec"),
            "path": "spec.qspec",
            "startLine": 1,
            "startColumn": 1,
        }},
        "frame": {"modifies": [], "creates": [], "deletes": []},
    });
    if let Some(parent) = redefines {
        operation["redefines"] = json!(format!("ix://{PACKAGE}/{parent}/{name}"));
    }
    operation
}

/// The package: `ConfigVersion` declaring `operations`, and each of
/// `subtypes` as `(name, supertype, operations)`.
fn package(operations: Vec<Value>, subtypes: &[(&str, &str, Vec<Value>)]) -> Vec<u8> {
    let mut envelope: Value = serde_json::from_str(EXAMPLE).expect("the example parses");
    let types = envelope["types"].as_array_mut().expect("types");
    let config_version = types
        .iter_mut()
        .find(|node| node["identity"] == format!("ix://{PACKAGE}/ConfigVersion"))
        .expect("ConfigVersion");
    config_version["operations"] = json!(operations);
    for (name, supertype, operations) in subtypes {
        let identity = format!("ix://{PACKAGE}/{name}");
        types.push(json!({
            "identity": identity,
            "displayName": identity,
            "kind": {"module": PACKAGE, "name": "object_type"},
            "roles": [],
            "origin": {"generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }},
            "constraints": [],
            "extensions": [],
            "unknownPolicy": "reject",
            "supertypes": if supertype.is_empty() {
                json!([])
            } else {
                json!([format!("ix://{PACKAGE}/{supertype}")])
            },
            "fields": [],
            "operations": operations,
        }));
    }
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// The unit selecting `document` as model `Config`, then `declarations`, and
/// its package input.
fn unit(document: &[u8], declarations: &str) -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    let packages = package_input([document]);
    let digest = packages.keys().next().expect("one package");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model Config = {PACKAGE:?} version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         {declarations}\n",
        qsl_semantics::model::key::hex(digest)
    );
    (text, packages)
}

/// The spine's S1, I1 and S3/S4 over `declarations`.
fn compile(
    document: &[u8],
    declarations: &str,
    limits: SpineLimits,
) -> Result<CheckedUnit, Box<CompileRefusal>> {
    let (text, packages) = unit(document, declarations);
    let source = SourceIdentity::new("a", "u", "git", "1");
    let cancel = Cancel::new();
    let refusal = |failure| {
        spine::refusal_or_fault(failure).expect("the front end does not fault on this source")
    };
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(refusal)?
        .into_value();
    Ok(spine::check(
        &parsed,
        &models,
        &DependencyInput::default(),
        &spine::LockEvidence::default(),
        limits,
        &cancel,
    )
    .map_err(refusal)?
    .into_value())
}

fn compiled(document: &[u8], declarations: &str, limits: SpineLimits) -> CheckedUnit {
    compile(document, declarations, limits)
        .unwrap_or_else(|refusal| panic!("the unit compiles, got {refusal:?}"))
}

fn operation_name(owner: &str, name: &str) -> String {
    format!("ix://{PACKAGE}/{owner}/{name}")
}

/// Every dispatched call under `node`: its member name and the table it names.
fn calls(
    node: CheckedNode<'_>,
    operations: &[qsl_semantics::check::DispatchOperation],
    found: &mut Vec<(String, usize)>,
) {
    if let NodeKind::Dispatch {
        table, operation, ..
    } = node.kind()
    {
        found.push((operations[*operation].member.clone(), *table));
    }
    for child in node.children() {
        calls(child, operations, found);
    }
}

/// FR-302-AC-1 (TC-793): `size` is declared at `A` and redefined at `B`
/// (`Sub`, a subtype of `A`), `area` at `A` only; each has a body. Two
/// preconditions call `self.size()` and `self.area()` on `A`, and one clause
/// on `B` calls both. The package carries exactly two tables, one per called
/// operation: `size` maps `A` to `A`'s candidate and `B` to `B`'s, `area`
/// maps `A` and `B` to `A`'s. Every call names the table of its operation.
#[trace("TC-793", "FR-302-AC-1")]
#[test]
fn unit_compile_carries_one_linked_table_per_called_operation() {
    let document = package(
        vec![
            query("ConfigVersion", "size", &[], None),
            query("ConfigVersion", "area", &[], None),
            query("ConfigVersion", "probe1", &[], None),
            query("ConfigVersion", "probe2", &[], None),
        ],
        &[(
            "Sub",
            "ConfigVersion",
            vec![query("Sub", "size", &[], Some("ConfigVersion"))],
        )],
    );
    let checked = compiled(
        &document,
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         body areaA using v on Config::ConfigVersion::area() { 3 }\n\
         pre c1 using v on Config::ConfigVersion::probe1 { self.size() + self.area() > 0 }\n\
         pre c2 using v on Config::ConfigVersion::probe2 { self.area() + self.size() > 1 }\n\
         pre c3 using v on Config::Sub::probe1 { self.size() + self.area() > 2 }\n",
        SpineLimits::default(),
    );
    let graph = checked.package().graph();
    let a = graph.state_clause("c1").expect("c1").context();
    let b = graph.state_clause("c3").expect("c3").context();
    assert_ne!(a, b);

    // The expected tables are literals: for each of A and B, the body the
    // table links.
    let named = |index: usize| {
        graph
            .function_state(index)
            .expect("a function")
            .name
            .to_owned()
    };
    let mut tables: Vec<(String, String, usize)> = graph
        .dispatch_tables()
        .iter()
        .map(|table| {
            let linked = |subtype| named(table.linked_for(&subtype).expect("linked").body);
            (linked(a), linked(b), table.candidate_count() as usize)
        })
        .collect();
    tables.sort();
    assert_eq!(
        tables,
        [
            (
                operation_name("ConfigVersion", "area"),
                operation_name("ConfigVersion", "area"),
                1
            ),
            (
                operation_name("ConfigVersion", "size"),
                operation_name("Sub", "size"),
                2
            ),
        ]
    );

    // Every call names the table of its own operation: the table a `size`
    // call names links the `size` candidates, the table an `area` call names
    // the `area` candidate, whichever table index each has.
    let linked_pair = |table: usize| {
        let table = &graph.dispatch_tables()[table];
        let linked = |subtype| named(table.linked_for(&subtype).expect("linked").body);
        (linked(a), linked(b))
    };
    let mut found = Vec::new();
    for clause in ["c1", "c2", "c3"] {
        calls(
            graph.state_clause(clause).expect("clause").body(),
            graph.scope().dispatch_operations(),
            &mut found,
        );
    }
    let sizes = (
        operation_name("ConfigVersion", "size"),
        operation_name("Sub", "size"),
    );
    let areas = (
        operation_name("ConfigVersion", "area"),
        operation_name("ConfigVersion", "area"),
    );
    let mut counts = BTreeMap::new();
    for (member, table) in &found {
        let expected = if member == "size" { &sizes } else { &areas };
        assert_eq!(&linked_pair(*table), expected, "a `{member}` call");
        *counts.entry(member.clone()).or_insert(0_usize) += 1;
    }
    assert_eq!(
        counts,
        BTreeMap::from([("area".to_owned(), 3), ("size".to_owned(), 3)])
    );
}

/// The chain `ConfigVersion <- Sub <- C <- D`, each declaring `size`, with a
/// body at each: three `redefines` edges.
fn chain() -> (Vec<u8>, &'static str) {
    (
        package(
            vec![
                query("ConfigVersion", "size", &[], None),
                query("ConfigVersion", "probe", &[], None),
            ],
            &[
                (
                    "Sub",
                    "ConfigVersion",
                    vec![query("Sub", "size", &[], Some("ConfigVersion"))],
                ),
                ("C", "Sub", vec![query("C", "size", &[], Some("Sub"))]),
                ("D", "C", vec![query("D", "size", &[], Some("C"))]),
            ],
        ),
        "body b0 using v on Config::ConfigVersion::size() { 0 }\n\
         body b1 using v on Config::Sub::size() { 1 }\n\
         body b2 using v on Config::C::size() { 2 }\n\
         body b3 using v on Config::D::size() { 3 }\n\
         pre c using v on Config::ConfigVersion::probe { self.size() > 0 }\n",
    )
}

fn limits_with_family_steps(family_steps: u64) -> SpineLimits {
    SpineLimits {
        model: ModelNormalizationLimits::default().with_family_steps(family_steps),
        ..SpineLimits::default()
    }
}

/// FR-302-AC-5 (TC-795): a called family with `n = 3` `redefines` edges
/// compiles at `family_steps = 3`, and at `family_steps = 2` refuses at S3
/// with `resource_exhausted`/`insufficient-next-charge` and the retained
/// charge, naming the setting that raises the limit and its value 2, with no
/// checked package.
#[trace("TC-795", "FR-302-AC-5")]
#[test]
fn unit_compile_refuses_a_family_past_the_callers_family_steps() {
    let (document, declarations) = chain();
    let checked = compiled(&document, declarations, limits_with_family_steps(3));
    assert_eq!(checked.package().graph().dispatch_tables().len(), 1);

    let refusal = compile(&document, declarations, limits_with_family_steps(2))
        .expect_err("the family walk is past the limit");
    let CompileRefusal::Check { refusals, .. } = refusal.as_ref() else {
        panic!("an S3 refusal, got {refusal:?}");
    };
    let [failed] = refusals.as_slice() else {
        panic!("one failed family, got {refusals:?}");
    };
    let CheckCause::DispatchLink(cause) = &failed.cause else {
        panic!("a dispatch link refusal, got {:?}", failed.cause);
    };
    assert_eq!(cause.code, Code::ResourceExhausted);
    assert_eq!(cause.catalog.code(), "resource_exhausted");
    assert_eq!(cause.catalog.cause(), "insufficient-next-charge");
    assert_eq!(
        cause.operation.node,
        operation_name("ConfigVersion", "size")
    );
    // The charge record reaches the report through the refusal's fields.
    assert_eq!(
        refusal.fields().expect("the retained charge"),
        BTreeMap::from([
            ("setting", "model.family_steps".to_owned()),
            ("limit", "2".to_owned()),
            ("actual", "3".to_owned()),
            ("stage", "model-normalization".to_owned()),
            (
                "accounting_contract",
                "ModelNormalizationLimitsV1".to_owned()
            ),
            ("limit_kind", "family_steps".to_owned()),
            ("consumed", "2".to_owned()),
            ("next_charge", "1".to_owned()),
            ("charge_point", "model.family-edge".to_owned()),
        ])
    );
}

/// A model with `A.add(amount: Integer): Integer` and the probe the call is
/// written in.
fn add_package() -> Vec<u8> {
    package(
        vec![
            query("ConfigVersion", "add", &["amount"], None),
            query("ConfigVersion", "probe", &[], None),
        ],
        &[],
    )
}

fn add_unit(body: &str) -> String {
    format!("{body}\npre c using v on Config::ConfigVersion::probe {{ self.add(1) > 0 }}\n")
}

/// FR-302-AC-8 (TC-1011): a body with more parameters than its operation,
/// and a body whose type is not the operation's result, each refuse at S3 as
/// `ill_typed`/`type-mismatch` located at the body, with no package; the
/// well-formed body compiles.
#[trace("TC-1011", "FR-302-AC-8")]
#[test]
fn unit_compile_checks_a_bodys_arity_and_type_against_its_operation() {
    let document = add_package();
    let ok = compiled(
        &document,
        &add_unit("body ok using v on Config::ConfigVersion::add(amount) { amount }"),
        SpineLimits::default(),
    );
    assert_eq!(ok.package().graph().dispatch_tables().len(), 1);

    for body in [
        "body tooMany using v on Config::ConfigVersion::add(amount, extra) { amount }",
        "body wrongType using v on Config::ConfigVersion::add(amount) { true }",
    ] {
        let refusal = compile(&document, &add_unit(body), SpineLimits::default())
            .expect_err("the body does not match its operation");
        assert_eq!(refusal.code(), Code::IllTyped, "{body}");
        let CompileRefusal::Check { refusals, region } = refusal.as_ref() else {
            panic!("an S3 refusal for {body}, got {refusal:?}");
        };
        let [refusal] = refusals.as_slice() else {
            panic!("one refusal for {body}, got {refusals:?}");
        };
        assert_eq!(refusal.cause.cause(), Some("type-mismatch"), "{body}");
        assert!(region.is_some(), "{body}: located at the body");
        assert!(
            matches!(
                &refusal.location.origin,
                Origin::Body { function, .. } if function == &operation_name("ConfigVersion", "add")
            ),
            "{body}: {:?}",
            refusal.location
        );
    }
}

/// FR-302-AC-7, FR-103: a candidate's precondition comes from the unit `pre`
/// clauses that resolved to its operation, not from the domain package's
/// `requires` text. `A.size` has two unit `pre` clauses, conjoined; `B.size`
/// has none, so its effective precondition is `true` (FR-151: an absent own
/// precondition absorbs). Every domain `requires` is `false` and reaches
/// neither.
#[trace("TC-986", "FR-302-AC-7", "FR-103-AC-9")]
#[test]
fn a_candidates_precondition_is_the_units_pre_clauses_not_the_domain_requires() {
    let mut size = query("ConfigVersion", "size", &[], None);
    let mut sub_size = query("Sub", "size", &[], Some("ConfigVersion"));
    for operation in [&mut size, &mut sub_size] {
        let clause = json!({
            "language": "quire",
            "text": "false",
            "origin": operation["origin"].clone(),
        });
        operation["pre"] = json!([clause]);
    }
    let document = package(
        vec![size, query("ConfigVersion", "probe", &[], None)],
        &[("Sub", "ConfigVersion", vec![sub_size])],
    );
    let declarations = "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         pre preA1 using v on Config::ConfigVersion::size { true }\n\
         pre preA2 using v on Config::ConfigVersion::size { 1 > 0 }\n\
         pre c1 using v on Config::ConfigVersion::probe { self.size() > 0 }\n\
         pre c2 using v on Config::Sub::probe { self.size() > 0 }\n";
    let checked = compiled(&document, declarations, SpineLimits::default());
    let graph = checked.package().graph();
    let a = graph.state_clause("c1").expect("c1").context();
    let b = graph.state_clause("c2").expect("c2").context();
    let [table] = graph.dispatch_tables() else {
        panic!("one table for the one called operation");
    };
    assert!(table
        .linked_for(&a)
        .expect("A links")
        .precondition
        .is_some());
    assert!(table
        .linked_for(&b)
        .expect("B links")
        .precondition
        .is_none());

    // Without the unit `pre` clauses no candidate has a precondition,
    // whatever the domain `requires` says.
    let without = "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         pre c1 using v on Config::ConfigVersion::probe { self.size() > 0 }\n\
         pre c2 using v on Config::Sub::probe { self.size() > 0 }\n";
    let checked = compiled(&document, without, SpineLimits::default());
    let [table] = checked.package().graph().dispatch_tables() else {
        panic!("one table");
    };
    for subtype in [a, b] {
        assert!(table
            .linked_for(&subtype)
            .expect("links")
            .precondition
            .is_none());
    }
}

/// FR-302-AC-7: a called operation whose family has no body for a concrete
/// subtype is not applicable there. `A` is concrete and only `B.size` has a
/// body, so the compile refuses at S3 as `ambiguous_dispatch`/`no-applicable`
/// naming `A`, and produces no package.
#[trace("TC-794", "FR-302-AC-3")]
#[test]
fn a_family_with_no_body_for_a_concrete_subtype_refuses_at_s3() {
    let document = package(
        vec![
            query("ConfigVersion", "size", &[], None),
            query("ConfigVersion", "probe", &[], None),
        ],
        &[(
            "Sub",
            "ConfigVersion",
            vec![query("Sub", "size", &[], Some("ConfigVersion"))],
        )],
    );
    let refusal = compile(
        &document,
        "body sizeB using v on Config::Sub::size() { 2 }\n\
         pre c1 using v on Config::ConfigVersion::probe { self.size() > 0 }\n",
        SpineLimits::default(),
    )
    .expect_err("A has no applicable candidate");
    assert_eq!(refusal.code(), Code::AmbiguousDispatch);
    let CompileRefusal::Check { refusals, .. } = refusal.as_ref() else {
        panic!("an S3 refusal, got {refusal:?}");
    };
    let [refusal] = refusals.as_slice() else {
        panic!("one failed family, got {refusals:?}");
    };
    let CheckCause::DispatchLink(cause) = &refusal.cause else {
        panic!("a dispatch link refusal, got {:?}", refusal.cause);
    };
    assert_eq!(cause.catalog.cause(), "no-applicable");
    let Some(failure) = &cause.subtype else {
        panic!("a failing subtype, got none");
    };
    assert_eq!(
        failure.subtype.node,
        operation_name_of_type("ConfigVersion")
    );
    assert!(failure.candidates.is_empty());
}

fn operation_name_of_type(name: &str) -> String {
    format!("ix://{PACKAGE}/{name}")
}

/// FR-302: a body reads its receiver as `self` and its parameters under the
/// names it wrote, which need not be the operation's.
#[trace("TC-986", "FR-302-AC-7")]
#[test]
fn a_body_reads_its_receiver_and_its_renamed_parameter() {
    let checked = compiled(
        &add_package(),
        &add_unit(
            "body renamed using v on Config::ConfigVersion::add(n) { n + deref(self).versionNumber }",
        ),
        SpineLimits::default(),
    );
    assert_eq!(checked.package().graph().dispatch_tables().len(), 1);
}

/// FR-302, FR-060: the facade's `compile_package` emits the package the
/// linked unit checks to, with a body function and a precondition function
/// for each candidate and the disjunction `B.size` takes with the `A.size`
/// it redefines. E4 places every occurrence of those functions at the unit
/// declaration they came from, so the emission is whole and deterministic.
#[trace("TC-1018", "FR-096-AC-20")]
#[test]
fn a_unit_with_linked_dispatch_emits_a_package() {
    let document = package(
        vec![
            query("ConfigVersion", "size", &[], None),
            query("ConfigVersion", "probe", &[], None),
        ],
        &[(
            "Sub",
            "ConfigVersion",
            vec![query("Sub", "size", &[], Some("ConfigVersion"))],
        )],
    );
    let (text, _) = unit(
        &document,
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         pre preA using v on Config::ConfigVersion::size { true }\n\
         pre preB using v on Config::Sub::size { false }\n\
         pre c using v on Config::ConfigVersion::probe { self.size() > 0 }\n",
    );
    let emit = || {
        qsl_replay::compile_package(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            text.as_bytes(),
            [document.as_slice()],
            &DependencyInput::default(),
            qsl_replay::StageLimits::default(),
            qsl_replay::ReplayLimits::default(),
        )
        .unwrap_or_else(|refusal| panic!("the linked unit emits, got {refusal:?}"))
    };
    let (first, second) = (emit(), emit());
    assert!(!first.bytes().is_empty());
    assert_eq!(first.bytes(), second.bytes());
    assert_eq!(first.package_id(), second.package_id());
}

/// `document` with the object type `name` declared abstract.
fn abstract_type(document: &[u8], name: &str) -> Vec<u8> {
    let mut envelope: Value = serde_json::from_slice(document).expect("valid JSON");
    envelope["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|node| node["identity"] == format!("ix://{PACKAGE}/{name}"))
        .expect("the type")["abstract"] = json!(true);
    serde_json::to_vec(&envelope).expect("valid JSON")
}

fn refused_family(refusal: &CompileRefusal) -> &qsl_semantics::check::DispatchLinkCause {
    let CompileRefusal::Check { refusals, .. } = refusal else {
        panic!("an S3 refusal, got {refusal:?}");
    };
    let [failed] = refusals.as_slice() else {
        panic!("one failed family, got {refusals:?}");
    };
    let CheckCause::DispatchLink(cause) = &failed.cause else {
        panic!("a dispatch link refusal, got {:?}", failed.cause);
    };
    cause
}

/// FR-302-AC-9 (TC-1019): a family is called by the call that reaches it,
/// not by its member name. `Other` is unrelated to `ConfigVersion` and
/// declares a `size` of its own with no body. A unit whose only `size` call
/// is on a `ConfigVersion` receiver compiles; a unit with a `size` call on an
/// `Other` receiver refuses `ambiguous_dispatch`/`no-applicable`, naming
/// `Other.size`.
#[trace("TC-1019", "FR-302-AC-9")]
#[test]
fn an_unrelated_family_of_the_same_name_refuses_only_when_a_call_reaches_it() {
    let document = package(
        vec![
            query("ConfigVersion", "size", &[], None),
            query("ConfigVersion", "probe", &[], None),
        ],
        &[(
            "Other",
            "",
            vec![
                query("Other", "size", &[], None),
                query("Other", "probe", &[], None),
            ],
        )],
    );
    let declarations = "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         pre c1 using v on Config::ConfigVersion::probe { self.size() > 0 }\n";
    let checked = compiled(&document, declarations, SpineLimits::default());
    assert!(!checked.package().graph().dispatch_tables().is_empty());

    let refusal = compile(
        &document,
        &format!("{declarations}pre c2 using v on Config::Other::probe {{ self.size() > 0 }}\n"),
        SpineLimits::default(),
    )
    .expect_err("the call on an Other receiver finds a family with no body");
    assert_eq!(refusal.code(), Code::AmbiguousDispatch);
    let cause = refused_family(&refusal);
    assert_eq!(cause.catalog.cause(), "no-applicable");
    assert_eq!(cause.operation.node, operation_name("Other", "size"));
    // The family's refusal is located at the call that needed it.
    assert_at_clause(&refusal, "c2");

    // A call on a receiver type that has no `size` anywhere is not that
    // family's: it refuses `missing_declaration`/`missing-name` at the call.
    let document = package(
        vec![
            query("ConfigVersion", "size", &[], None),
            query("ConfigVersion", "probe", &[], None),
        ],
        &[
            (
                "Other",
                "",
                vec![
                    query("Other", "size", &[], None),
                    query("Other", "probe", &[], None),
                ],
            ),
            ("Lone", "", vec![query("Lone", "probe", &[], None)]),
        ],
    );
    let refusal = compile(
        &document,
        &format!("{declarations}pre c3 using v on Config::Lone::probe {{ self.size() > 0 }}\n"),
        SpineLimits::default(),
    )
    .expect_err("Lone has no `size`");
    assert_eq!(refusal.code(), Code::MissingDeclaration);
    let CompileRefusal::Check { refusals, .. } = refusal.as_ref() else {
        panic!("an S3 refusal, got {refusal:?}");
    };
    assert_eq!(refusals[0].cause.cause(), Some("missing-name"));
    assert_at_clause(&refusal, "c3");
}

/// The first refusal of the S3 refusal `refusal` is located at the clause
/// `clause`, and the refusal carries a source region.
fn assert_at_clause(refusal: &CompileRefusal, clause: &str) {
    let CompileRefusal::Check { refusals, region } = refusal else {
        panic!("an S3 refusal, got {refusal:?}");
    };
    assert!(
        matches!(&refusals[0].location.origin, Origin::StateClause { clause: found, .. } if found == clause),
        "{:?}",
        refusals[0].location
    );
    assert!(region.is_some(), "a region at the call");
}

/// FR-302-AC-1 (TC-793): a call whose receiver's static type is an abstract
/// type conforming to the root's declaring type names the root's table.
/// `Mid` is abstract, between `ConfigVersion` and `Sub`, and a clause on
/// `Mid` calls `size`.
#[trace("TC-793", "FR-302-AC-1")]
#[test]
fn a_call_on_an_abstract_conforming_type_names_the_roots_table() {
    let document = abstract_type(
        &package(
            vec![
                query("ConfigVersion", "size", &[], None),
                query("ConfigVersion", "probe", &[], None),
            ],
            &[
                ("Mid", "ConfigVersion", vec![]),
                (
                    "Sub",
                    "Mid",
                    vec![query("Sub", "size", &[], Some("ConfigVersion"))],
                ),
            ],
        ),
        "Mid",
    );
    let checked = compiled(
        &document,
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         pre cm using v on Config::Mid::probe { self.size() > 0 }\n",
        SpineLimits::default(),
    );
    let graph = checked.package().graph();
    let mut found = Vec::new();
    calls(
        graph.state_clause("cm").expect("cm").body(),
        graph.scope().dispatch_operations(),
        &mut found,
    );
    assert_eq!(found.len(), 1);
    assert_eq!(graph.dispatch_tables().len(), 1);
}

/// FR-302 Inputs, TC-196 D07: a dispatched call in an operation body is no
/// input to linking. It refuses `ill_typed`/`operator-ineligible`, where the
/// body is checked.
#[trace("TC-1019", "FR-302-AC-9")]
#[test]
fn a_dispatched_call_in_an_operation_body_refuses_ill_typed() {
    let document = package(
        vec![
            query("ConfigVersion", "size", &[], None),
            query("ConfigVersion", "other", &[], None),
            query("ConfigVersion", "probe", &[], None),
        ],
        &[],
    );
    let refusal = compile(
        &document,
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body otherA using v on Config::ConfigVersion::other() { self.size() }\n\
         pre c using v on Config::ConfigVersion::probe { self.other() > 0 }\n",
        SpineLimits::default(),
    )
    .expect_err("a body cannot dispatch");
    assert_eq!(refusal.code(), Code::IllTyped);
    let CompileRefusal::Check { refusals, .. } = refusal.as_ref() else {
        panic!("an S3 refusal, got {refusal:?}");
    };
    assert_eq!(refusals[0].cause.cause(), Some("operator-ineligible"));
}

/// FR-096-AC-20 (TC-1018): a function synthesized for dispatch from the
/// unit's declarations is located at them. The body function of `A.size`
/// resolves to the body expression of its `operation-body`, the precondition
/// function to the first unit `pre` clause of its operation (the
/// disjunction `B.size` takes with `A.size` too), and each clause function's
/// one `generated` occurrence to its body.
#[trace("TC-1018", "FR-096-AC-20")]
#[test]
fn a_function_synthesized_for_dispatch_is_located_at_the_declarations_it_came_from() {
    let document = package(
        vec![
            query("ConfigVersion", "size", &[], None),
            query("ConfigVersion", "probe", &[], None),
        ],
        &[(
            "Sub",
            "ConfigVersion",
            vec![query("Sub", "size", &[], Some("ConfigVersion"))],
        )],
    );
    let pre_a = "pre preA using v on Config::ConfigVersion::size { true }";
    let pre_b = "pre preB using v on Config::Sub::size { false }";
    let declarations = format!(
        "body sizeA using v on Config::ConfigVersion::size() {{ 11 }}\n\
         body sizeB using v on Config::Sub::size() {{ 22 }}\n\
         {pre_a}\n{pre_b}\n\
         pre c using v on Config::ConfigVersion::probe {{ self.size() > 0 }}\n"
    );
    let (text, _) = unit(&document, &declarations);
    let checked = compiled(&document, &declarations, SpineLimits::default());
    let graph = checked.package().graph();
    let region_text = |name: &str| -> (String, String) {
        let index = (0..)
            .map_while(|index| graph.function_state(index).map(|state| (index, state.name)))
            .find_map(|(index, found)| (found == name).then_some(index))
            .unwrap_or_else(|| panic!("a function named {name}"));
        let location = Location::root(Origin::Body {
            function: name.to_owned(),
            index,
        });
        let region = graph
            .region(&location)
            .unwrap_or_else(|| panic!("{name} resolves to a region"));
        let text_of = |region: &qsl_foundation::source::provenance::SourceRegion| {
            let (start, end) = (region.start() as usize, region.end() as usize);
            text[start..end].to_owned()
        };
        // Its one occurrence is `generated`, at the same location.
        let key = graph.function_identity(name).expect("an identity");
        let occurrences: Vec<_> = graph
            .occurrences()
            .filter(|(node, origin, _)| *node == key && origin.role().as_str() == "generated")
            .collect();
        let [(_, _, found)] = occurrences.as_slice() else {
            panic!("one generated occurrence of {name}, got {occurrences:?}");
        };
        assert_eq!(graph.region(found).as_ref(), Some(&region), "{name}");
        (text_of(&region), name.to_owned())
    };
    let size_a = operation_name("ConfigVersion", "size");
    let size_b = operation_name("Sub", "size");
    assert_eq!(region_text(&size_a).0, "11");
    assert_eq!(region_text(&size_b).0, "22");
    assert_eq!(region_text(&format!("{size_a}.precondition")).0, pre_a);
    assert_eq!(region_text(&format!("{size_b}.precondition")).0, pre_b);
    assert_eq!(
        region_text(&format!("{size_b}.precondition.effective")).0,
        pre_b
    );
}

/// `document` with the object type `name` given `supertypes` (its names).
fn with_supertypes(document: &[u8], name: &str, supertypes: &[&str]) -> Vec<u8> {
    let mut envelope: Value = serde_json::from_slice(document).expect("valid JSON");
    envelope["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|node| node["identity"] == format!("ix://{PACKAGE}/{name}"))
        .expect("the type")["supertypes"] = json!(supertypes
        .iter()
        .map(|supertype| format!("ix://{PACKAGE}/{supertype}"))
        .collect::<Vec<_>>());
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// `size` redefined at `Sub` (`B`) and at `C`, both subtypes of
/// `ConfigVersion` (`A`), with the concrete `D` inheriting from both and redefining it without a
/// body; `area`
/// redefined at `Sub` only. The probes the calls are written in.
fn diamond() -> Vec<u8> {
    with_supertypes(
        &package(
            vec![
                query("ConfigVersion", "size", &[], None),
                query("ConfigVersion", "area", &[], None),
                query("ConfigVersion", "probe1", &[], None),
                query("ConfigVersion", "probe2", &[], None),
            ],
            &[
                (
                    "Sub",
                    "ConfigVersion",
                    vec![
                        query("Sub", "size", &[], Some("ConfigVersion")),
                        query("Sub", "area", &[], Some("ConfigVersion")),
                    ],
                ),
                (
                    "C",
                    "ConfigVersion",
                    vec![query("C", "size", &[], Some("ConfigVersion"))],
                ),
                (
                    "D",
                    "ConfigVersion",
                    vec![query("D", "size", &[], Some("ConfigVersion"))],
                ),
            ],
        ),
        "D",
        &["Sub", "C"],
    )
}

/// The failing subtypes of `refusal`, as `(operation, cause, subtype)` in
/// report order.
fn failures(refusal: &CompileRefusal) -> Vec<(String, &'static str, String)> {
    let CompileRefusal::Check { refusals, .. } = refusal else {
        panic!("an S3 refusal, got {refusal:?}");
    };
    refusals
        .iter()
        .map(|refusal| {
            let CheckCause::DispatchLink(cause) = &refusal.cause else {
                panic!("a dispatch link refusal, got {:?}", refusal.cause);
            };
            assert_eq!(cause.code, Code::AmbiguousDispatch);
            assert_eq!(cause.catalog.code(), "ambiguous_dispatch");
            let failure = cause.subtype.as_ref().expect("a failing subtype");
            assert_eq!(cause.catalog.cause(), failure.cause);
            (
                cause.operation.node.clone(),
                failure.cause,
                failure.subtype.node.clone(),
            )
        })
        .collect()
}

/// FR-302-AC-2 and FR-302-AC-4 (TC-794 steps 1 and 3): `D` inherits `size`
/// from `Sub` and `C`, which both redefine `ConfigVersion.size` and neither
/// dominates the other, so the unit refuses `ambiguous_dispatch`/
/// `multiple-undominated` naming `D`, both candidates and the dominance
/// pairs. A unit whose second call is `area`, with bodies only at `Sub`, adds
/// `no-applicable` for `C` and `ConfigVersion`: one refusal names all three
/// failures in charge order (`area` before `size`, though the `size` call is
/// written first; subtypes by ascending effective identity), the same on a
/// second compile, with no checked package.
#[trace("TC-794", "FR-302-AC-2", "FR-302-AC-4")]
#[test]
fn a_unit_with_several_ambiguous_families_is_refused_once_naming_each() {
    let document = diamond();
    let size = "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
                body sizeB using v on Config::Sub::size() { 2 }\n\
                body sizeC using v on Config::C::size() { 3 }\n\
                pre c1 using v on Config::ConfigVersion::probe1 { self.size() > 0 }\n";
    let refusal = compile(&document, size, SpineLimits::default())
        .expect_err("D has two undominated candidates");
    assert_eq!(refusal.code(), Code::AmbiguousDispatch);
    assert_eq!(
        failures(&refusal),
        [(
            operation_name("ConfigVersion", "size"),
            "multiple-undominated",
            operation_name_of_type("D"),
        )]
    );
    let cause = refused_family(&refusal);
    let failure = cause.subtype.as_ref().expect("a failing subtype");
    assert_eq!(
        failure
            .candidates
            .iter()
            .map(|key| key.node.clone())
            .collect::<Vec<_>>(),
        [
            operation_name("C", "size"),
            operation_name("ConfigVersion", "size"),
            operation_name("Sub", "size"),
        ]
    );
    assert_eq!(
        failure
            .dominance_pairs
            .iter()
            .map(|(over, under)| (over.node.clone(), under.node.clone()))
            .collect::<Vec<_>>(),
        [
            (
                operation_name("C", "size"),
                operation_name("ConfigVersion", "size")
            ),
            (
                operation_name("Sub", "size"),
                operation_name("ConfigVersion", "size")
            ),
        ]
    );

    let both = format!(
        "{size}body areaB using v on Config::Sub::area() {{ 4 }}\n\
         pre c2 using v on Config::ConfigVersion::probe2 {{ self.area() > 0 }}\n"
    );
    let refusal = compile(&document, &both, SpineLimits::default()).expect_err("two families fail");
    let expected = [
        (
            operation_name("ConfigVersion", "area"),
            "no-applicable",
            operation_name_of_type("C"),
        ),
        (
            operation_name("ConfigVersion", "area"),
            "no-applicable",
            operation_name_of_type("ConfigVersion"),
        ),
        (
            operation_name("ConfigVersion", "size"),
            "multiple-undominated",
            operation_name_of_type("D"),
        ),
    ];
    assert_eq!(failures(&refusal), expected);
    let again = compile(&document, &both, SpineLimits::default()).expect_err("refused again");
    assert_eq!(failures(&again), expected);
}

fn object_reference(effective: EffectiveId, key: &str) -> ObjectReference {
    ObjectReference::new(
        UniverseId::from_digest([7; 32]),
        effective,
        ObjectId::new(key.to_owned()).expect("a key"),
    )
}

/// Evaluates clause `name` of `checked` over one object of type `receiver`,
/// a `ConfigVersion` with `versionNumber` 1 and no parent, and gives the
/// outcome with the work units it consumed.
fn run_pre(
    checked: &CheckedUnit,
    name: &str,
    receiver: EffectiveId,
) -> (FamilyOutcome<Quire>, u64) {
    let package = checked.package();
    let graph = package.graph();
    let clause = graph.state_clause(name).expect("the clause");
    let reference = object_reference(receiver, "self");
    let environment = ObjectEnvironment::new(
        ObjectClosure::new(
            graph.scope().types(),
            vec![(
                reference.clone(),
                vec![
                    (
                        "versionNumber",
                        FieldValue::Present(Quire::Integer(Integer::from(1_i64))),
                    ),
                    ("parent", FieldValue::Absent),
                ],
            )],
            &[],
        )
        .expect("a closed environment"),
    );
    let observations = AdmittedObservations {
        usage: AdmissionUsage::default(),
        clause: clause.identity(),
        current: None,
        pre: Some(Observation {
            identity: DocumentRef {
                authority: "test".to_owned(),
                identity: "pre".to_owned(),
                revision_namespace: "ns".to_owned(),
                revision: "1".to_owned(),
                digest: [0; 32],
            },
            environment,
            populations: BTreeMap::new(),
            out_of_range: Vec::new(),
        }),
        post: None,
        self_object: reference,
        invocation: None,
        self_kind: qsl_semantics::model::observation::SelfKind::Object,
        receiver_pre: None,
        receiver_post: None,
        parameters: Vec::new(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    };
    let mut meter = Meter::new(qsl_replay::spine::default_accounting(1_000_000));
    let evaluation = package
        .evaluate_clause(
            &QualifiedName::unqualified(name).unwrap(),
            &observations,
            &mut meter,
        )
        .unwrap_or_else(|failure| panic!("{name} evaluates, got {failure:?}"))
        .evaluation;
    (evaluation.outcome, meter.consumed(LimitKind::WorkUnits))
}

/// [`run_pre`]'s completed Boolean.
fn evaluate_pre(checked: &CheckedUnit, name: &str, receiver: EffectiveId) -> bool {
    match run_pre(checked, name, receiver).0 {
        FamilyOutcome::Evaluated(Outcome::Completed(Quire::Boolean(value))) => value,
        other => panic!("a completed Boolean, got {other:?}"),
    }
}

/// FR-302-AC-6 (TC-809): `size` has a body at `ConfigVersion` (`A`) returning
/// 1 and a redefinition at `Sub` (`B`) returning 2; the precondition is
/// `self.size() = 2`. The unit compiles; the precondition is `true` for a
/// receiver whose most-specific type is `B` and `false` for one of type `A`.
#[trace("TC-809", "FR-302-AC-6")]
#[test]
fn evaluation_runs_the_body_the_linked_table_selects_for_the_receivers_type() {
    let document = package(
        vec![
            query("ConfigVersion", "size", &[], None),
            query("ConfigVersion", "probe", &[], None),
        ],
        &[(
            "Sub",
            "ConfigVersion",
            vec![query("Sub", "size", &[], Some("ConfigVersion"))],
        )],
    );
    let checked = compiled(
        &document,
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         pre onA using v on Config::ConfigVersion::probe { self.size() = 2 }\n\
         pre onB using v on Config::Sub::probe { self.size() = 2 }\n",
        SpineLimits::default(),
    );
    let graph = checked.package().graph();
    let a = graph.state_clause("onA").expect("onA").context();
    let b = graph.state_clause("onB").expect("onB").context();
    assert_ne!(a, b);
    assert!(evaluate_pre(&checked, "onA", b));
    assert!(!evaluate_pre(&checked, "onA", a));
}

/// FR-395-AC-12 (TC-1089 step 8): a function the checker synthesizes from an
/// operation `body` owns its arithmetic record though no call can name it,
/// and that record is no fault for a call to an unrelated function.
#[trace("TC-1089", "FR-395-AC-12")]
#[test]
fn a_clause_function_is_a_known_owner_and_no_fault() {
    let declarations = format!(
        "{}\nfunction inc using v(x: Int[0, 9]): Int[0, 10] pure {{ x + 1 }}",
        add_unit(
            "body renamed using v on Config::ConfigVersion::add(n) { n + deref(self).versionNumber }"
        )
    );
    let document = add_package();
    let checked = compiled(&document, &declarations, SpineLimits::default());
    let graph = checked.package().graph();
    let clause_owner = graph
        .requirements()
        .values()
        .filter_map(|record| record.owner())
        .find(|owner| *owner != "inc")
        .expect("the body's record has a clause function as owner");
    assert!(graph.declares_function(clause_owner));
    assert!(graph.callable(clause_owner).is_none());

    let (text, packages) = unit(&document, &declarations);
    let call = qsl_replay::Call {
        function: "inc".to_owned(),
        arguments: vec![qsl_replay::CallArgument {
            parameter: "x".to_owned(),
            value: quire_exact::Value::Integer(Integer::from(3_i64)),
        }],
        accounting: spine::default_accounting(1_000_000),
        supplied: qsl_replay::SuppliedLimits::default(),
    };
    let result = qsl_replay::run_with_requirements(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
        &call,
        &qsl_replay::ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .expect("a clause function's record is no fault");
    assert_eq!(result.requirements().members().len(), 1);
}

/// FR-151-AC-19, QSpec TC-196 D10 on the dispatch path (QSL-935): a member's
/// own `pre` clauses are conjoined in ascending clause name under FR-006's
/// short-circuit, whatever their source order. `ConfigVersion::size` has the
/// clauses `Aaa` and `Zzz`, one `false` and the other undefined (it calls
/// `other`, whose own precondition is `false`). With the false clause named
/// first the undefined clause is neither evaluated nor reported (the call is
/// undefined for `size`'s own false precondition); with the undefined clause
/// named first the false one is not reached (the call is undefined for
/// `other`'s). Swapping only the source order changes neither the outcome nor
/// the charges.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-19", "FR-120-AC-15")]
#[test]
fn own_precondition_clauses_conjoin_in_ascending_name_whatever_the_source_order() {
    let document = package(
        vec![
            query("ConfigVersion", "size", &[], None),
            query("ConfigVersion", "other", &[], None),
            query("ConfigVersion", "probe", &[], None),
        ],
        &[],
    );
    let run = |first: &str, second: &str| {
        let declarations = format!(
            "body sizeA using v on Config::ConfigVersion::size() {{ 1 }}\n\
             body otherA using v on Config::ConfigVersion::other() {{ 1 }}\n\
             pre otherPre using v on Config::ConfigVersion::other {{ false }}\n\
             {first}\n{second}\n\
             pre c1 using v on Config::ConfigVersion::probe {{ self.size() > 0 }}\n"
        );
        let checked = compiled(&document, &declarations, SpineLimits::default());
        let receiver = checked
            .package()
            .graph()
            .state_clause("c1")
            .expect("c1")
            .context();
        let (outcome, charged) = run_pre(&checked, "c1", receiver);
        (format!("{outcome:?}"), charged)
    };
    let falsy =
        |name: &str| format!("pre {name} using v on Config::ConfigVersion::size {{ false }}");
    let undefined = |name: &str| {
        format!("pre {name} using v on Config::ConfigVersion::size {{ self.other() > 0 }}")
    };

    // The false clause sorts first: the undefined one is never evaluated.
    let false_first = run(&falsy("Aaa"), &undefined("Zzz"));
    assert_eq!(false_first, run(&undefined("Zzz"), &falsy("Aaa")));
    // The undefined clause sorts first: the false one is never reached.
    let undefined_first = run(&undefined("Aaa"), &falsy("Zzz"));
    assert_eq!(undefined_first, run(&falsy("Zzz"), &undefined("Aaa")));
    // The two orders differ in what they evaluate: the false clause makes
    // `size`'s own precondition false, the undefined one carries `other`'s.
    assert!(
        false_first.0.contains("\"operation\": \"size\""),
        "{false_first:?}"
    );
    assert!(
        undefined_first.0.contains("\"operation\": \"other\""),
        "{undefined_first:?}"
    );
}
