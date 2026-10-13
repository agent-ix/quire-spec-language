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

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_foundation::Setting;
use qsl_replay::spine::{self, CheckedUnit, CompileRefusal, ParseRequest, SpineLimits};
use qsl_replay::{Code, DependencyInput, SourceIdentity};
use qsl_semantics::check::{CheckCause, CheckedNode, NodeKind};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::package_input;
use quire_exact::Cancel;
use quire_semantic_value::location::Origin;
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
            "supertypes": [format!("ix://{PACKAGE}/{supertype}")],
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
         profile v = \"quire.value.complete/v1\";\n\
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

    // Every call names its operation's table: both `size` calls through A
    // and the one through B name one table; the `area` calls name the other.
    let mut found = Vec::new();
    for clause in ["c1", "c2", "c3"] {
        calls(
            graph.state_clause(clause).expect("clause").body(),
            graph.scope().dispatch_operations(),
            &mut found,
        );
    }
    let by_member = |member: &str| -> Vec<usize> {
        found
            .iter()
            .filter(|(name, _)| name == member)
            .map(|(_, table)| *table)
            .collect()
    };
    let (sizes, areas) = (by_member("size"), by_member("area"));
    assert_eq!((sizes.len(), areas.len()), (3, 3));
    assert_eq!(sizes.iter().collect::<BTreeSet<_>>().len(), 1);
    assert_eq!(areas.iter().collect::<BTreeSet<_>>().len(), 1);
    assert_ne!(sizes[0], areas[0]);
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
/// with `resource_exhausted`/`family-steps`, naming the setting that raises
/// the limit, its value 2 and the count the refused step would reach, with no
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
    let [refusal] = refusals.as_slice() else {
        panic!("one failed family, got {refusals:?}");
    };
    let CheckCause::DispatchLink(cause) = &refusal.cause else {
        panic!("a dispatch link refusal, got {:?}", refusal.cause);
    };
    assert_eq!(cause.code, Code::ResourceExhausted);
    assert_eq!(cause.catalog.code(), "resource_exhausted");
    assert_eq!(cause.catalog.cause(), "family-steps");
    assert_eq!(
        cause.operation.node,
        operation_name("ConfigVersion", "size")
    );
    let limit = cause.limit.expect("the limit reached");
    assert_eq!(limit.setting, Setting::ModelFamilySteps);
    assert_eq!((limit.bound, limit.reached), (2, 3));
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
#[trace("TC-793", "FR-302-AC-1")]
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
    let [failure] = cause.subtypes.as_slice() else {
        panic!("one failing subtype, got {:?}", cause.subtypes);
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
#[trace("TC-1011", "FR-302-AC-8")]
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
#[trace("TC-793", "FR-302-AC-1")]
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
