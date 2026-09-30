// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-034: ConfigVersion object-presence and graph-reachability clauses refuse
//! the state-scalar projection at their exact authored loci, before IR binding.

use crate::support::config_version as config;

use std::{fs, path::Path};

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use quire_contract_ir as ir;
use quire_spec_language::{
    checking::{check, CheckBindings, CheckLimits, ClauseBinding},
    formal_source::FormalSource,
    link_native,
    lowering::{lower_for, LoweringCode, LoweringLimits, ProjectionTarget},
    native_model::NativeModel,
    package::{NativePackage, PackageLimits},
    parse, Limits, LinkLimits,
};
use serde_json::Value;

fn compile_config<'m>(
    directory: &Path,
    models: &'m [NativeModel],
    case: config::Case,
) -> NativePackage<'m> {
    config::write(directory, &models[0], case).unwrap();
    let job: Value =
        serde_json::from_slice(&fs::read(directory.join("request.json")).unwrap()).unwrap();
    let request = &job["request"];
    let program = &request["program"];
    let source = &program["source"];
    let unit = parse(
        SourceIdentity {
            authority: source["authority"].as_str().unwrap().into(),
            identity: source["identity"].as_str().unwrap().into(),
            revision_namespace: source["revision_namespace"].as_str().unwrap().into(),
            revision: source["revision"].as_str().unwrap().into(),
        },
        "program.native",
        &fs::read(directory.join("program.native")).unwrap(),
        Limits::default(),
    )
    .unwrap();
    let formal = FormalSource::new(
        unit.source().clone(),
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new(source["document"].as_str().unwrap()).unwrap(),
            ir::SourceRevision::new(source["formal_revision"].as_u64().unwrap()).unwrap(),
        ),
    );
    let selected = &program["clauses"][0];
    let owner = &selected["owner"];
    let requirement = ir::RequirementRef::parse(
        owner["package"].as_str().unwrap(),
        owner["requirement"].as_str().unwrap(),
        owner["revision"].as_u64().unwrap(),
    )
    .unwrap();
    let clause = ir::ClauseId::new(selected["clause"].as_str().unwrap()).unwrap();
    let execution_point: ir::ExecutionPoint =
        serde_json::from_value(selected["point"].clone()).unwrap();
    let checked = check(
        link_native(unit, models, LinkLimits::default()).unwrap(),
        CheckBindings {
            source: formal,
            clauses: vec![ClauseBinding {
                name: selected["name"].as_str().unwrap().into(),
                requirement,
                clause,
                execution_point,
            }],
        },
        CheckLimits::default(),
    )
    .unwrap();
    NativePackage::new(checked, PackageLimits::default()).unwrap()
}

#[test]
#[trace("TC-112", "FR-034-AC-6")]
fn object_and_graph_clauses_refuse_at_exact_authored_loci_without_artifacts() {
    let models = [config::model().unwrap()];
    let root = tempfile::tempdir().unwrap();
    for (case, clause, spelling) in [
        (config::Case::Healthy, "parent_order", "present"),
        (config::Case::Cycle, "no_cycle", "reaches"),
    ] {
        let native = compile_config(&root.path().join(case.id()), &models, case);
        let error = lower_for(
            &native,
            ProjectionTarget::StateScalarIrV1,
            LoweringLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, LoweringCode::Unsupported);
        assert_eq!(error.clause.as_ref().unwrap().clause().as_str(), clause);
        let span = error.source.expect("unsupported native expression locus");
        let source = &native.checked().bindings().source;
        assert!(source.source().slice(span).unwrap().contains(spelling));
        assert!(
            error.upstream.is_empty(),
            "refusal occurs before IR binding"
        );
    }
}
