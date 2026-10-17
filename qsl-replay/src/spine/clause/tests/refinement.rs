// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-861 and TC-862 (FR-341, FR-342): the refinement class of a spine
//! compile result and of a clause-run report. Real compiles and runs use
//! FR-108's ConfigVersion fixtures; the variants and multi-cause vectors a
//! fixture cannot reach are built here. Every expected class is a literal.

use super::frame::{frame_input, object, run};
use super::*;
use crate::spine::ImportRefusal;
use crate::spine::{
    classify_clause_run, classify_compile, classify_refusal, ClauseRunReport, CompileRefusal,
    ReachedLimit, RefinementClass, RefusalCause, RunRefusal, SuppliedLibrary,
};
use qsl_foundation::diagnostic::{LimitExceeded, StageFailure};
use qsl_foundation::{Code, Setting, SettingLimits};
use qsl_package::{EmitRefusal, ImportViewRefusal};
use qsl_semantics::check::{AssemblyCause, AssemblyError, AssemblyRefusal};
use qsl_semantics::check::{
    CheckCause, CheckRefusal, DispatchLinkCause, KeyFault, ProtocolAnchorCause,
    ProtocolReferenceCause,
};
use qsl_semantics::library::LibraryName;
use quire_semantic_value::location::{Location, Origin};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";

fn compile(unit: &str, limits: SpineLimits) -> Result<ComposedUnit, Box<CompileRefusal>> {
    compose(
        source(),
        "refinement.native",
        unit.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        limits,
    )
}

fn cause(code: Code, cause: Option<&'static str>) -> RefusalCause {
    RefusalCause {
        code,
        cause,
        limit: None,
    }
}

fn location() -> Location {
    Location::root(Origin::Expression)
}

fn check(causes: Vec<CheckCause>) -> CompileRefusal {
    CompileRefusal::Check {
        refusals: causes
            .into_iter()
            .map(|cause| CheckRefusal {
                location: location(),
                cause,
            })
            .collect(),
        region: None,
    }
}

fn ill_typed() -> CheckCause {
    CheckCause::NonBooleanRoot
}

fn invariant() -> CheckCause {
    CheckCause::InternalFault(Box::new(KeyFault::StageFault(
        qsl_foundation::InternalFault::new("S3", "refinement-test"),
    )))
}

fn not_yet_implemented() -> CheckCause {
    CheckCause::ProtocolAnchor(Box::new(ProtocolAnchorCause::Unimplemented {
        name: "Flow".to_owned(),
        construct: "compensate",
        span: qsl_foundation::Span { start: 0, end: 1 },
    }))
}

fn class_of(refusal: &CompileRefusal) -> RefinementClass {
    classify_refusal(refusal).class
}

/// FR-341-AC-1: a ConfigVersion unit that compiles classifies `admitted`.
#[trace("TC-861", "FR-341-AC-1")]
#[test]
fn a_unit_that_compiles_is_admitted() {
    let (unit, packages) = config_version_unit_and_packages();
    let compiled = compose(
        source(),
        "clause-run-config-version.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    );
    let classified = classify_compile(&compiled);
    assert_eq!(classified.class, RefinementClass::Admitted);
    assert!(classified.causes.is_empty());
}

/// FR-341-AC-2: setup defects classify `tool failure`, whichever cause is
/// first.
#[trace("TC-861", "FR-341-AC-2")]
#[test]
fn a_broken_setup_is_a_tool_failure() {
    // `Profile`: a header profile naming no catalog row.
    let unknown_profile = compile(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"test:unknown-profile\";\n\
         function f using v(): Boolean pure { true }\n",
        SpineLimits::default(),
    );
    let classified = classify_compile(&unknown_profile);
    assert_eq!(classified.class, RefinementClass::ToolFailure);
    assert_eq!(
        classified.causes,
        [cause(Code::UnknownProfile, Some("unsupported-selection"))]
    );

    // `Intake`: the unit's `model` package is not supplied.
    let (unit, _) = config_version_unit_and_packages();
    let unsupplied = compose(
        source(),
        "clause-run-config-version.native",
        unit.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    );
    let refusal = unsupplied.as_ref().expect_err("no package is supplied");
    assert!(matches!(**refusal, CompileRefusal::Intake { .. }));
    assert_eq!(
        classify_compile(&unsupplied).class,
        RefinementClass::ToolFailure
    );

    // `DependencyInput`: a library repeating the unit's source owner.
    let library = SuppliedLibrary {
        identity: "test/lib".to_owned(),
        source: source(),
        path: "lib.native".to_owned(),
        bytes: format!("{HEADER}function g using v(): Boolean pure {{ true }}\n").into_bytes(),
    };
    let dependencies = DependencyInput::new([library]).expect("an admissible input");
    let owner_clash = compose(
        source(),
        "refinement.native",
        format!("{HEADER}function f using v(): Boolean pure {{ true }}\n").as_bytes(),
        &BTreeMap::new(),
        &dependencies,
        SpineLimits::default(),
    );
    let refusal = owner_clash.as_ref().expect_err("the owner is shared");
    assert!(matches!(**refusal, CompileRefusal::DependencyInput(_)));
    assert_eq!(
        classify_compile(&owner_clash).class,
        RefinementClass::ToolFailure
    );

    // `Import`: a library the input does not hold.
    let missing_import = compile(
        &format!(
            "{HEADER}import \"test/units\" as u;\nfunction f using v(): Boolean pure {{ true }}\n"
        ),
        SpineLimits::default(),
    );
    let refusal = missing_import.as_ref().expect_err("nothing is supplied");
    assert!(matches!(**refusal, CompileRefusal::Import { .. }));
    assert_eq!(
        classify_compile(&missing_import).class,
        RefinementClass::ToolFailure
    );

    // A typed refusal cause and a `runtime_invariant` cause, in either order.
    assert_eq!(
        class_of(&check(vec![ill_typed(), invariant()])),
        RefinementClass::ToolFailure
    );
    assert_eq!(
        class_of(&check(vec![invariant(), ill_typed()])),
        RefinementClass::ToolFailure
    );
}

/// FR-341-AC-3: an ill-typed body is `refused` carrying `ill_typed`, and a
/// refusal's later typed cause decides over an earlier unsupported one.
#[trace("TC-861", "FR-341-AC-3")]
#[test]
fn a_typed_refusal_cause_anywhere_is_refused() {
    let ill_typed_body = compile(
        &format!("{HEADER}function f using v(): Boolean pure {{ 1 + true }}\n"),
        SpineLimits::default(),
    );
    let classified = classify_compile(&ill_typed_body);
    assert_eq!(classified.class, RefinementClass::Refused);
    assert_eq!(
        classified.causes.first().map(|c| c.code),
        Some(Code::IllTyped)
    );

    let both = classify_refusal(&check(vec![not_yet_implemented(), ill_typed()]));
    assert_eq!(both.class, RefinementClass::Refused);
    assert_eq!(
        both.causes,
        [
            cause(Code::UnsupportedConstruct, Some("not-yet-implemented")),
            cause(Code::IllTyped, Some("non-boolean-root")),
        ]
    );
}

/// FR-341, FR-091-AC-43: a malformed `UUID("...")` literal is a typed
/// `ill_typed`/`type-mismatch` refusal of the unit's content, so it classifies
/// `refused`, carries no limit and is no `incomplete` or `unsupported` class.
#[trace("TC-861", "FR-341-AC-3")]
#[test]
fn a_malformed_uuid_literal_is_a_refusal_of_the_content() {
    let refusal = check(vec![CheckCause::MalformedUuidLiteral {
        argument: "00112233-4455-6677-8899-AABBCCDDEEFF".to_owned(),
    }]);
    let classified = classify_refusal(&refusal);
    assert_eq!(classified.class, RefinementClass::Refused);
    assert_eq!(
        classified.causes,
        [cause(Code::IllTyped, Some("type-mismatch"))]
    );
    assert!(classified.limits().is_empty());
}

/// FR-341-AC-4: a reached stage limit is `incomplete`, naming the limit and
/// its value.
#[trace("TC-861", "FR-341-AC-4")]
#[test]
fn a_reached_limit_is_incomplete_naming_the_limit() {
    let unit = format!("{HEADER}function f using v(): Boolean pure {{ true }}\n");

    let mut limits = SpineLimits::default();
    assert!(limits.set_bound(Setting::S1Nodes, 1));
    let source_stage = classify_compile(&compile(&unit, limits));
    assert_eq!(source_stage.class, RefinementClass::Incomplete);
    assert_eq!(
        source_stage.limits(),
        [ReachedLimit {
            name: "s1.nodes",
            bound: 1
        }]
    );

    let mut limits = SpineLimits::default();
    assert!(limits.set_bound(Setting::S3Nodes, 1));
    let check_stage = classify_compile(&compile(
        &format!(
            "{HEADER}function g using v(): Boolean pure {{ true }}\n\
             function f using v(): Boolean pure {{ not not true }}\n"
        ),
        limits,
    ));
    assert_eq!(check_stage.class, RefinementClass::Incomplete);
    assert_eq!(
        check_stage.limits(),
        [ReachedLimit {
            name: "s3.nodes",
            bound: 1
        }]
    );
}

/// FR-341-AC-5: a construct the build does not implement, an omitting
/// emission and an unsupported projection each classify `unsupported`.
#[trace("TC-861", "FR-341-AC-5")]
#[test]
fn an_implementation_gap_is_unsupported() {
    assert_eq!(
        class_of(&check(vec![not_yet_implemented()])),
        RefinementClass::Unsupported
    );
    assert_eq!(
        class_of(&CompileRefusal::Omitted(Vec::new())),
        RefinementClass::Unsupported
    );
    let projection = CompileRefusal::Emit(EmitRefusal::NothingToEmit {
        omitted: Vec::new(),
    });
    assert_eq!(
        projection.causes(),
        [cause(Code::UnsupportedProjection, None)]
    );
    assert_eq!(class_of(&projection), RefinementClass::Unsupported);
}

/// FR-341-AC-6: a refusal whose every cause is a profile-prohibited form
/// classifies `prohibited`.
#[trace("TC-861", "FR-341-AC-6")]
#[test]
fn a_prohibited_form_is_prohibited() {
    let refusal = check(vec![
        CheckCause::ProtocolReference(Box::new(ProtocolReferenceCause::UnsupportedRoot {
            root: "attempt",
        })),
        CheckCause::ProtocolReference(Box::new(ProtocolReferenceCause::Repeated {
            first: location(),
        })),
    ]);
    let classified = classify_refusal(&refusal);
    assert_eq!(classified.class, RefinementClass::Prohibited);
    assert_eq!(
        classified.causes,
        [
            cause(Code::UnsupportedConstruct, Some("declaration-form")),
            cause(Code::UnsupportedConstruct, Some("expression-form")),
        ]
    );
}

/// FR-341: the all-causes accessor yields every entry of a vector refusal in
/// order, and one cause for every other variant.
#[trace("TC-861", "FR-341-AC-3")]
#[test]
fn the_accessor_yields_every_cause_in_order() {
    let refusal = check(vec![invariant(), ill_typed(), not_yet_implemented()]);
    let codes: Vec<Code> = refusal.causes().iter().map(|cause| cause.code).collect();
    assert_eq!(
        codes,
        [
            Code::RuntimeInvariant,
            Code::IllTyped,
            Code::UnsupportedConstruct
        ]
    );
    assert_eq!(
        CompileRefusal::Omitted(Vec::new()).causes().len(),
        1,
        "a non-vector variant has one cause"
    );
}

fn current_report(clause: &str, root: i64, child: i64) -> ClauseRunReport {
    let model_digest_hex = config_version_model_digest_hex();
    let label = config_version_document_ref("current-snap");
    let bytes = config_version_snapshot(&label, &model_digest_hex, root, Some(child));
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = config_version_current_selection(clause, label, "child");
    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    run_clause(request).expect("a well-formed request always reports")
}

fn assert_run(
    report: &ClauseRunReport,
    class: RefinementClass,
    stage: ClauseRunStage,
) -> crate::spine::RunClassification {
    let classified = classify_clause_run(report);
    assert_eq!(
        (classified.classification.class, classified.stage),
        (class, stage),
        "{:?}",
        report.disposition
    );
    classified
}

/// FR-342-AC-1: the evaluated verdicts classify at stage `evaluate`.
#[trace("TC-862", "FR-342-AC-1")]
#[test]
fn evaluated_verdicts_classify_at_evaluate() {
    use RefinementClass::{Admitted, Refused};
    assert_run(
        &current_report("ParentOrder", 1, 2),
        Admitted,
        ClauseRunStage::Evaluate,
    );
    assert_run(
        &current_report("ParentOrder", 3, 2),
        Refused,
        ClauseRunStage::Evaluate,
    );
    let forbidden_parent_change = run(frame_input(
        config_version_request,
        "attemptUpdate",
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 2, None)],
        |_| {},
    ));
    let classified = assert_run(&forbidden_parent_change, Refused, ClauseRunStage::Evaluate);
    assert_eq!(
        classified.classification.causes,
        [cause(Code::FrameViolation, Some("unauthorized-change"))]
    );
}

/// FR-342-AC-2: select and admit refusals classify at their own stage.
#[trace("TC-862", "FR-342-AC-2")]
#[test]
fn select_and_admit_refusals_classify_at_their_stage() {
    use RefinementClass::{Absent, Refused};
    assert_run(
        &current_report("Absent", 1, 2),
        Absent,
        ClauseRunStage::Select,
    );

    let integer_function = run_clause(config_version_function_request(
        "n",
        vec![ClauseArgument {
            parameter: "a".to_owned(),
            value: config_version_reference("child"),
        }],
    ))
    .unwrap();
    assert_run(&integer_function, Refused, ClauseRunStage::Select);

    let unknown_parameter = run_clause(config_version_function_request(
        "sameIdentity",
        vec![ClauseArgument {
            parameter: "c".to_owned(),
            value: config_version_reference("child"),
        }],
    ))
    .unwrap();
    assert_run(&unknown_parameter, Refused, ClauseRunStage::Admit);
}

/// FR-342-AC-3: an exhausted work budget is `incomplete` at `evaluate`,
/// naming `work_units` and the value zero.
#[trace("TC-862", "FR-342-AC-3")]
#[test]
fn an_exhausted_budget_is_incomplete_naming_work_units() {
    let model_digest_hex = config_version_model_digest_hex();
    let label = config_version_document_ref("current-snap");
    let bytes = config_version_snapshot(&label, &model_digest_hex, 1, Some(2));
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = config_version_current_selection("ParentOrder", label, "child");
    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    request.accounting = default_accounting(0);
    let report = run_clause(request).unwrap();
    let classified = assert_run(
        &report,
        RefinementClass::Incomplete,
        ClauseRunStage::Evaluate,
    );
    assert_eq!(
        classified.classification.limits(),
        [ReachedLimit {
            name: "work_units",
            bound: 0
        }]
    );
}

/// FR-342-AC-4: dispositions the gate never produces classify `tool failure`.
#[trace("TC-862", "FR-342-AC-4")]
#[test]
fn dispositions_the_gate_never_produces_are_tool_failures() {
    let fault = || qsl_foundation::InternalFault::new("test", "refinement-test");
    let dispositions = [
        ClauseDisposition::UnknownLanguage {
            language: "ix:other".to_owned(),
        },
        ClauseDisposition::Admit(qsl_semantics::model::observation::AdmissionFailure::Fault(
            fault(),
        )),
        ClauseDisposition::EvaluateFault(fault()),
    ];
    for disposition in dispositions {
        let report = ClauseRunReport {
            package_id: None,
            usage: Default::default(),
            basis: crate::proof_result::SettlementBasis::Unavailable,
            witness: None,
            disposition,
        };
        assert_eq!(
            classify_clause_run(&report).classification.class,
            RefinementClass::ToolFailure,
            "{:?}",
            report.disposition
        );
    }
}

/// FR-342-AC-5: a run whose compile refuses classifies at stage `compile`
/// with exactly the class FR-341 gives its refusal.
#[trace("TC-862", "FR-342-AC-5")]
#[test]
fn a_compile_refusal_classifies_as_fr_341_does() {
    let mut request = request(ClauseRunSelection::Function {
        name: "seven".to_owned(),
        arguments: Vec::new(),
        snapshot: config_version_document_ref("snap"),
    });
    request.packages = BTreeMap::new();
    let report = run_clause(request).unwrap();
    let classified = assert_run(
        &report,
        RefinementClass::ToolFailure,
        ClauseRunStage::Compile,
    );
    let ClauseDisposition::Compile(refusal) = &report.disposition else {
        panic!("expected a compile refusal, got {:?}", report.disposition);
    };
    assert_eq!(classified.classification, classify_refusal(refusal));
}

/// FR-341-AC-4: an assembly type-environment ceiling is `incomplete`, naming
/// the limit and its value.
#[trace("TC-861", "FR-341-AC-4")]
#[test]
fn an_assembly_type_limit_is_incomplete_naming_the_limit() {
    let refusal = CompileRefusal::Assembly {
        refusal: AssemblyRefusal {
            errors: vec![AssemblyError {
                cause: AssemblyCause::TypeLimit(LimitExceeded::new(
                    Setting::EnvironmentWorkUnits,
                    1,
                    2,
                )),
                span: qsl_foundation::Span { start: 0, end: 1 },
            }],
        },
        region: None,
    };
    let classified = classify_refusal(&refusal);
    assert_eq!(classified.class, RefinementClass::Incomplete);
    assert_eq!(
        classified.limits(),
        [ReachedLimit {
            name: "environment.work_units",
            bound: 1
        }]
    );
}

/// FR-341 rule 4: an import whose I2 read reached an `i2.*` ceiling is
/// `incomplete`, naming the limit, not a broken setup.
#[trace("TC-861", "FR-341-AC-4")]
#[test]
fn an_import_read_ceiling_is_incomplete_naming_the_limit() {
    let identity = LibraryName::new("test/units").expect("a library identity");
    let refusal = CompileRefusal::Import {
        refusal: ImportRefusal::View {
            identity: identity.clone(),
            refusal: Box::new(ImportViewRefusal::Read {
                package: identity,
                refusal: StageFailure::Limit(LimitExceeded::new(Setting::I2Nodes, 1, 2)),
            }),
        },
        region: None,
    };
    let classified = classify_refusal(&refusal);
    assert_eq!(classified.class, RefinementClass::Incomplete);
    assert_eq!(
        classified.limits(),
        [ReachedLimit {
            name: "i2.nodes",
            bound: 1
        }]
    );
}

/// A one-object `current` snapshot for `ParentOrder`: `child` at version 2
/// naming `parent_key`, in a population that is `complete` or not.
fn child_snapshot_report(identity: &str, complete: bool, parent_key: &str) -> ClauseRunReport {
    let model_digest_hex = config_version_model_digest_hex();
    let label = config_version_document_ref(identity);
    let population = config_version_population_identity();
    let value = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {
            "authority": label.authority, "identity": label.identity,
            "revision_namespace": label.revision_namespace, "revision": label.revision,
        },
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {
            "identity": CONFIG_VERSION_PACKAGE_IDENTITY, "version": "1.0.0",
            "digest": format!("sha256-jcs:{model_digest_hex}"),
        },
        "populations": [{
            "population": population,
            "complete": complete,
            "objects": [{
                "key": "child", "type": config_version_type(),
                "fields": {
                    "versionNumber": {"integer": "2"},
                    "parent": {"present": {"reference": {"population": population, "key": parent_key}}},
                },
            }],
        }],
    });
    let bytes = value.to_string().into_bytes();
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = config_version_current_selection("ParentOrder", label, "child");
    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    run_clause(request).unwrap()
}

/// FR-342-AC-2: a dangling parent refuses at `admit`.
#[trace("TC-862", "FR-342-AC-2")]
#[test]
fn a_dangling_parent_is_refused_at_admit() {
    let report = child_snapshot_report("dangling-parent", true, "ghost");
    let classified = assert_run(&report, RefinementClass::Refused, ClauseRunStage::Admit);
    assert_eq!(
        classified.classification.causes.first().map(|c| c.code),
        Some(Code::DanglingReference)
    );
}

/// FR-342-AC-3: an incomplete population is `incomplete` at `admit`, naming
/// its cause; the cause carries no numeric limit.
#[trace("TC-862", "FR-342-AC-3")]
#[test]
fn an_incomplete_population_is_incomplete_at_admit() {
    let report = child_snapshot_report("incomplete-population", false, "root");
    let classified = assert_run(&report, RefinementClass::Incomplete, ClauseRunStage::Admit);
    assert_eq!(
        classified.classification.causes,
        [cause(Code::IncompletePopulation, Some("incomplete-scope"))]
    );
    assert!(classified.classification.limits().is_empty());
}

/// FR-342-AC-4: a stale package is a `tool failure` at `compile`.
#[trace("TC-862", "FR-342-AC-4")]
#[test]
fn a_stale_package_is_a_tool_failure_at_compile() {
    let mut request = config_version_request(ClauseRunSelection::Function {
        name: "n".to_owned(),
        arguments: Vec::new(),
        snapshot: config_version_document_ref("current-snap"),
    });
    request.expected_package_id = Some(compiled().emitted.package_id());
    let report = run_clause(request).unwrap();
    assert_run(
        &report,
        RefinementClass::ToolFailure,
        ClauseRunStage::Compile,
    );
}

/// FR-342: a cancelled argument refusal reads as the table's `refused`,
/// carrying the `cancelled` code.
#[trace("TC-862", "FR-342-AC-2")]
#[test]
fn a_cancelled_argument_refusal_is_refused_at_admit() {
    let report = ClauseRunReport {
        package_id: None,
        usage: Default::default(),
        basis: crate::proof_result::SettlementBasis::Unavailable,
        witness: None,
        disposition: ClauseDisposition::ArgumentRefusal(Box::new(RunRefusal::Cancelled(
            quire_exact::CancelCause::Requested,
        ))),
    };
    let classified = assert_run(&report, RefinementClass::Refused, ClauseRunStage::Admit);
    assert_eq!(
        classified.classification.causes,
        [cause(Code::Cancelled, None)]
    );
}

/// FR-341 rules 3 and 4: the caller's model budget denied at S3 is
/// `incomplete` naming its limit, as at I1, never `refused`.
#[trace("TC-861", "FR-341-AC-4")]
#[test]
fn a_model_budget_denial_at_s3_is_incomplete_naming_the_limit() {
    let incomplete = qsl_semantics::model::accounting::Incomplete {
        limit_kind: qsl_semantics::model::accounting::LimitKind::WorkUnits,
        limit: 7,
        consumed: 7,
        next_charge: 1,
        charge_point: qsl_semantics::model::accounting::ChargePoint::NormalizeRecord,
    };
    let denial = incomplete.denial();
    let expected = ReachedLimit {
        name: denial.setting().name(),
        bound: 7,
    };

    let model = classify_refusal(&check(vec![CheckCause::ModelIncomplete(incomplete)]));
    assert_eq!(model.class, RefinementClass::Incomplete);
    assert_eq!(model.limits(), [expected]);

    let dispatch = classify_refusal(&check(vec![CheckCause::DispatchLink(Box::new(
        DispatchLinkCause {
            operation: qsl_semantics::model::key::DeclarationKey {
                package: "test/orders".to_owned(),
                node: "op".to_owned(),
            },
            code: Code::ResourceExhausted,
            catalog: qsl_foundation::CatalogCode::new(
                "resource_exhausted",
                "insufficient-next-charge",
            ),
            denial: Some(denial),
            subtype: None,
        },
    ))]));
    assert_eq!(dispatch.class, RefinementClass::Incomplete);
    assert_eq!(dispatch.limits(), [expected]);
}
