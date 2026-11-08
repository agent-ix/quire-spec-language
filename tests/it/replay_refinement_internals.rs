// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual refinement lists cross the complete-package replay boundary.

use crate::support::refinement_cas::{self, Variant};
use ix_trace_rs::trace;
use qsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits, SuppliedLibrary};
use qsl_replay::{
    AccountingLimits, EvaluatedValue, Identifier, InputSettlement, QualifiedName, ReplayLimits,
    ReplayPackage, ReplayRefusal, ReplayRequestWire, ReplayResult, ReplaySource, RequestTargetWire,
    StateEnvironment,
};
use quire_exact::Cancel;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const ROOT: &str = "test/refinement-root";
const LIBRARY: &str = "test/geometry";
const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.model.complete/v1\";\n";

fn documents() -> Vec<Vec<u8>> {
    let mut documents = refinement_cas::documents(Variant::CAS);
    // Use the existing row-empty refinement fixture, independent of counter
    // transitions: no operation contract is needed by these two protocols.
    for document in &mut documents {
        let mut value: Value = serde_json::from_slice(document).unwrap();
        value["populations"] = json!([]);
        for ty in value["types"].as_array_mut().unwrap() {
            if ty["kind"]["name"] == "object_type" {
                ty["operations"] = json!([]);
            }
        }
        *document = serde_json::to_vec(&value).unwrap();
    }
    documents
}

fn unit(protocol: &str, refinement: &str, imports: &str, function: &str) -> String {
    format!(
        "{HEADER}profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n\
         {imports}model Spec = \"{spec}\";\n\
         model Impl = \"{concrete}\";\n\
         protocol {protocol} using p over (c: Spec::Counter) on origin {{\n\
         role w on Spec::Counter; scheduling adversarial;\n\
         run sequence Main {{ }}\n\
         finish Done as (d: Boolean) {{ true }};\n\
         }}\n\
         refinement {refinement} using inf abstract Spec::{protocol} concrete Impl {{}}\n\
         {function}\n",
        spec = refinement_cas::SPEC,
        concrete = refinement_cas::CAS,
    )
}

fn internals(refinement: &str, protocol: &str) -> Value {
    json!([{"refinement":refinement,"internal":[format!("Spec::{protocol}::Done")]}])
}

fn request(with_dependency: bool) -> ReplayRequestWire {
    let documents = documents();
    let libraries = if with_dependency {
        vec![
            SuppliedLibrary {
                identity: "test/aaa".into(),
                source: qsl_foundation::SourceIdentity::new("test", "earlier"),
                path: "earlier.native".into(),
                bytes: format!("{HEADER}function unused using v(): Boolean pure {{ true }}\n")
                    .into_bytes(),
            },
            SuppliedLibrary {
                identity: LIBRARY.into(),
                source: qsl_foundation::SourceIdentity::new("test", "geometry"),
                path: "geometry.native".into(),
                bytes: unit(
                    "Other",
                    "CasOther",
                    "",
                    "function libraryValue using v(x: Boolean): Boolean pure { x }",
                )
                // Each refinement's abstract protocol belongs to its own package.
                .replace(refinement_cas::SPEC, refinement_cas::THIRD)
                .into_bytes(),
            },
        ]
    } else {
        Vec::new()
    };
    let (imports, function) = if with_dependency {
        (
            "import \"test/aaa\" as earlier;\nimport \"test/geometry\" as g;\n",
            "function entry using v(): Boolean pure { g::libraryValue(false) }",
        )
    } else {
        ("", "function entry using v(): Boolean pure { false }")
    };
    let program = unit("Twice", "CasTwice", imports, function);
    let root_internals = internals("CasTwice", "Twice");
    let library_internals = internals("CasOther", "Other");
    let dependencies = DependencyInput::new(libraries.clone()).unwrap();
    let dependencies = if with_dependency {
        dependencies
            .with_refinement_internals([(LIBRARY.to_owned(), library_internals.clone())])
            .unwrap()
    } else {
        dependencies
    };
    let source = qsl_foundation::SourceIdentity::new("test", "root");
    let selection = QualifiedName::new(vec![Identifier::new("entry").unwrap()]).unwrap();
    let located = qsl_replay::call_site(
        source.clone(),
        ROOT.into(),
        "root.native",
        program.as_bytes(),
        documents.iter().map(Vec::as_slice),
        &dependencies,
        &selection,
    )
    .expect("actual source compilation issues the function site");
    let cancel = Cancel::new();
    let limits = SpineLimits::default();
    let parse_request = ParseRequest {
        source: &source,
        path: "root.native",
        bytes: program.as_bytes(),
    };
    let parsed = spine::parse(&parse_request, limits.source, &cancel)
        .unwrap()
        .into_value();
    let admitted = spine::admit_parsed_unit_list(vec![parsed], &dependencies).unwrap();
    let packages =
        qsl_semantics::model::intake::package_input(documents.iter().map(Vec::as_slice)).unwrap();
    let models = spine::select_units(&admitted, &packages, limits.model, limits.source, &cancel)
        .unwrap()
        .into_value();
    let checked = spine::check_units_with_refinement_internals(
        &admitted,
        ROOT.into(),
        Some(root_internals.clone()),
        &models,
        &dependencies,
        limits,
        &cancel,
    )
    .expect("the actual root and dependency lists check in their own owners")
    .into_value();
    let emitted = spine::package(&checked[0], spine::PackageLimits::default(), &cancel)
        .unwrap()
        .into_value();
    let mut offered: Value = serde_json::from_slice(emitted.package().bytes()).unwrap();
    let refinement = checked[0]
        .package()
        .graph()
        .refinements()
        .iter()
        .find(|refinement| refinement.name() == "CasTwice")
        .expect("the genuine checked refinement");
    assert_eq!(
        refinement
            .abstract_internal()
            .iter()
            .map(|class| class.spelling())
            .collect::<Vec<_>>(),
        ["Twice::Done"]
    );
    // The raw replay carrier supplies the same explicit list that genuine
    // producer checking used to create the retained refinement class.
    offered["lock"]["refinement_internals"] = root_internals;
    let offered = serde_json::to_vec(&offered).unwrap();
    let offered = quire_canonical::read(&offered, u64::MAX).unwrap();
    let offered =
        quire_canonical::to_vec(&offered.root(), quire_canonical::Limits::new(u64::MAX)).unwrap();
    let mut byte_provision = vec![("test".into(), "root".into(), program.into_bytes())];
    byte_provision.extend(libraries.iter().map(|library| {
        (
            "test".into(),
            library.source.identity.to_owned(),
            library.bytes.clone(),
        )
    }));
    ReplayRequestWire {
        package: ReplayPackage::read(&offered, ReplayLimits::default().input_bytes).unwrap(),
        profile_selections: Vec::new(),
        dependencies: libraries
            .iter()
            .map(|library| qsl_replay::DependencyEntryWire {
                identity: library.identity.clone(),
                sources: vec![("test".into(), library.source.identity.to_owned())],
                refinement_internals: (library.identity == LIBRARY)
                    .then(|| serde_json::from_value(library_internals.clone()).unwrap()),
            })
            .collect(),
        replay_target: RequestTargetWire::function(selection, located.site.declaration.clone()),
        source: ReplaySource::Input(Vec::new()),
        obligation_identity: located.site.obligation_identity().clone(),
        backend: "actual-refinement-replay".into(),
        state_environment: StateEnvironment::default(),
        accounting_limits: AccountingLimits::default().0,
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision,
        documents,
    }
}

fn assert_false_replay(wire: ReplayRequestWire) {
    let result = qsl_replay::replay(wire, ReplayLimits::default())
        .expect("the complete genuine package admits and replays");
    let ReplayResult::Input(input) = result else {
        panic!("an input replay");
    };
    assert_eq!(input.value(), Some(EvaluatedValue::Boolean(false)));
    assert_eq!(
        input.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );
}

#[trace("TC-444", "FR-098-AC-20")]
#[test]
fn supplied_actual_refinement_internal_list_replays_a_value() {
    let wire = request(false);
    let raw: Value = serde_json::from_slice(wire.package.bytes()).unwrap();
    assert_eq!(
        raw["lock"]["refinement_internals"],
        internals("CasTwice", "Twice")
    );
    assert_false_replay(wire);
}

#[trace("TC-444", "FR-098-AC-20")]
#[test]
fn unresolved_internal_class_keeps_its_native_cause_and_request_locus() {
    let mut wire = request(false);
    let mut raw: Value = serde_json::from_slice(wire.package.bytes()).unwrap();
    raw["lock"]["refinement_internals"][0]["internal"][0] = json!("Spec::Twice::zz");
    wire.package = ReplayPackage::read(&serde_json::to_vec(&raw).unwrap(), u64::MAX).unwrap();
    let error = qsl_replay::replay(wire, ReplayLimits::default()).unwrap_err();
    assert_eq!(error.code(), qsl_foundation::Code::MissingDeclaration);
    assert_eq!(error.cause(), Some("missing-name"));
    assert_eq!(
        error.pointer().as_deref(),
        Some("/package/lock/refinement_internals/0/internal/0")
    );
    let ReplayRefusal::RefinementInternals { refusal, .. } = error else {
        panic!("actual native list refusal");
    };
    let qsl_replay::CompileRefusal::Check { refusals, .. } = *refusal else {
        panic!("native root S3 check");
    };
    let qsl_semantics::check::CheckCause::Refinement(refusal) = &refusals[0].cause else {
        panic!("native refinement cause");
    };
    let qsl_semantics::check::RefinementCause::InternalList(defect) = &refusal.cause else {
        panic!("native list defect");
    };
    assert_eq!(defect.kind, qsl_semantics::check::ListDefectKind::Missing);
    assert_eq!(defect.pointer, "/0/internal/0");
}

#[trace("TC-444", "FR-098-AC-20")]
#[test]
fn each_supplied_list_reaches_only_its_root_or_dependency_refinement() {
    let wire = request(true);
    assert_eq!(wire.dependencies[0].identity, "test/aaa");
    assert!(wire.dependencies[0].refinement_internals.is_none());
    assert_eq!(wire.dependencies[1].identity, LIBRARY);
    assert_eq!(
        serde_json::to_value(wire.dependencies[1].refinement_internals.as_ref().unwrap()).unwrap(),
        internals("CasOther", "Other")
    );
    let raw: Value = serde_json::from_slice(wire.package.bytes()).unwrap();
    assert_eq!(
        raw["lock"]["refinement_internals"],
        internals("CasTwice", "Twice")
    );
    // The dependency must not receive the root's otherwise valid list.
    let mut misassigned = wire.clone();
    misassigned.dependencies[1].refinement_internals =
        Some(serde_json::from_value(internals("CasTwice", "Twice")).unwrap());
    let error = qsl_replay::replay(misassigned, ReplayLimits::default()).unwrap_err();
    assert_eq!(error.code(), qsl_foundation::Code::MissingDeclaration);
    assert_eq!(error.cause(), Some("missing-name"));
    assert_eq!(
        error.pointer().as_deref(),
        Some("/dependencies/1/refinement_internals/0/refinement")
    );
    assert_false_replay(wire);
}
