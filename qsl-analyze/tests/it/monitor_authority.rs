// SPDX-License-Identifier: AGPL-3.0-or-later

// The tested boundary retains original owner refusals inline after budget denial.
#![allow(clippy::result_large_err)]
//! FR-251 same-emission authority through genuine observation-owner admission.

mod observation;

use ix_trace_rs::trace;
use qsl_analyze::monitor_root::{
    AdmittedPlanAuthority, PlanAuthorityRefusal, PlanAuthorityRefusalCode, PlanAuthoritySelection,
    PlanAuthorityValidationCaps, PlanRoot, ProgressSelectionRefusalCode,
};
use qsl_semantics::check::TimeSourceKind;
use quire_exact::{Origin, Role};
use quire_observation::authority::{self, progress, SubjectSelection};

use crate::monitor_root::{authored, root_of, target, LIMITS};

fn native_budget() -> quire_contract_model::NativeJoinBudget {
    quire_contract_model::NativeJoinBudget::new(100_000_000, 100_000_000)
        .with_read_storage_limit(100_000_000)
}

fn native_locus() -> quire_contract_model::NativeJoinLocus {
    quire_contract_model::NativeJoinLocus {
        source: quire_contract_model::NativeJoinLocusSource::ReachedRow(37),
        form: "qsl_authority_selection",
    }
}

fn tick_root() -> PlanRoot {
    let text = authored("always (holds(c.versionNumber = 2) implies eventually[0 ms, 3 ms] holds(c.versionNumber = 3))")
        .replace("time M dense unit ms;", "time M tick M::ConfigVersion::attemptUpdate period 2 ms unit ms;");
    root_of(&text, "C", &target()).unwrap()
}

fn limits() -> PlanAuthorityValidationCaps {
    PlanAuthorityValidationCaps {
        read: LIMITS,
        order: quire_canonical::Limits::new(1 << 20),
    }
}

fn node(key: quire_exact::NodeKey) -> progress::NodeIdentity {
    progress::NodeIdentity::from_key(key, authority::Limits::owner_max()).unwrap()
}

fn row(root: &PlanRoot) -> progress::AssociationRow {
    let source = &root.checked_unit().package().graph().time_sources()[0];
    let claim = root.plan().subject();
    progress::AssociationRow {
        model: source.model.clone(),
        model_node: node(source.model_node.clone()),
        claim: progress::ClaimOccurrence::new(
            node(claim.clause_node().clone()),
            claim.occurrence().origin(),
        ),
        source: match &source.source {
            TimeSourceKind::Dense => progress::TimeSource::Dense,
            TimeSourceKind::Tick { operation, period } => progress::TimeSource::Tick {
                operation: operation.clone(),
                operation_node: node(source.tick_operation_anchor.as_ref().unwrap().clone()),
                period_numerator: period.numerator().to_string(),
                period_denominator: period.denominator().to_string(),
            },
        },
        unit: source.unit.clone(),
        unit_node: node(source.unit_node.clone()),
        time_dimension: node(source.time_dimension.clone()),
        observation_source: observation::source().as_str().to_owned(),
        observation_clock: "clock:monitor-events".to_owned(),
        observation_revision: "1".to_owned(),
    }
}

fn admit(
    root: PlanRoot,
    progress: Option<progress::View>,
    subject: &SubjectSelection,
    limits: PlanAuthorityValidationCaps,
) -> Result<AdmittedPlanAuthority, PlanAuthorityRefusal> {
    let source = root.checked_unit().source().identity().clone();
    let claim = root.plan().subject().clone();
    root.admit_authority(
        PlanAuthoritySelection {
            source: &source,
            claim: &claim,
            observation_subject: subject,
            observation_clock: &observation::clock(),
            observation_source: &observation::source(),
        },
        progress,
        limits,
        &mut native_budget(),
        native_locus(),
    )
}

#[trace(
    "TC-1081",
    "FR-251-AC-45",
    "FR-251-AC-47",
    "FR-251-AC-48",
    "FR-251-AC-50"
)]
#[test]
fn genuine_tick_authority_retains_complete_original_values_and_same_emission() {
    let root = tick_root();
    let original_row = row(&root);
    let (progress, subject) = observation::admitted(Some(vec![original_row.clone()]));
    let owner_progress_pointer = progress.association_canonical_bytes().unwrap().as_ptr();
    let expected_progress = progress.association_canonical_bytes().unwrap().to_vec();
    let expected_order = root.model_step_order(limits().order).unwrap();
    let original_package_bytes = root.emission().package().bytes().to_vec();
    let original_plan = root.plan().clone();
    let original_obligations = root.obligations().clone();
    let original_source = root.checked_unit().source().identity().clone();
    let complete_association = serde_json::to_value(progress.association().unwrap()).unwrap();
    let context = admit(root, Some(progress), &subject, limits()).unwrap();
    drop(subject);
    assert!(context
        .root()
        .checked_unit()
        .package()
        .graph()
        .time_sources()
        .iter()
        .any(|source| std::ptr::eq(source, context.checked_time_source())));
    assert!(matches!(
        context.checked_time_source().source,
        TimeSourceKind::Tick { .. }
    ));
    let exact: &quire_contract_model::CheckedRational = context.checked_tick_period().unwrap();
    assert_eq!(exact.numerator.as_ref(), "2");
    assert_eq!(exact.denominator.as_ref(), "1");
    assert_eq!(context.admitted_order(), expected_order.bytes());
    assert_eq!(context.progress_context(), expected_progress);
    assert_eq!(context.progress_context().as_ptr(), owner_progress_pointer);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(context.progress_context()).unwrap(),
        complete_association
    );
    assert_ne!(context.progress_context(), context.progress().bytes());
    assert_eq!(
        context.progress().association().unwrap().rows(),
        &[original_row]
    );
    assert_eq!(context.root().plan(), &original_plan);
    assert_eq!(context.root().obligations(), &original_obligations);
    assert_eq!(
        context.root().checked_unit().source().identity(),
        &original_source
    );
    assert_eq!(
        context.root().emission().package().bytes(),
        original_package_bytes
    );
    let selection = context.root().plan().subject();
    let declaration = context
        .root()
        .emission()
        .temporal_binding_rows()
        .declaration(
            selection.clause_node(),
            selection.occurrence().origin().ordinal(),
        )
        .unwrap();
    assert_eq!(
        context
            .admitted_package()
            .monitor_plans()
            .next()
            .unwrap()
            .clause(),
        declaration
    );
    let binding = context.admission_binding();
    assert!(context.admitted_package().clone().retains(&binding));
    let independent = match quire_contract_model::read_checked_package(
        context.root().emission().package().bytes(),
        LIMITS,
        context.root().emission().evidence(),
    ) {
        quire_contract_model::CheckedPackageDispatchResult::AdmittedV2(package) => package,
        other => panic!("retained evidence strict read: {other:?}"),
    };
    assert!(
        !independent.retains(&binding),
        "equal-byte independent reads cannot replace the retained admission"
    );
}

#[trace("TC-1081", "FR-251-AC-49")]
#[test]
fn complete_handoffs_measure_work_logical_storage_and_read_storage_on_one_budget() {
    use quire_contract_model::{NativeJoinBudget, NativeJoinChargeKind, PackageReadResource};
    for tick in [false, true] {
    let handoff = |budget: &mut NativeJoinBudget| {
        let root = if tick {
            tick_root()
        } else {
            root_of(
                &authored("eventually[0 ms, 3 ms] holds(c.versionNumber = 3)"),
                "C",
                &target(),
            ).unwrap()
        };
        let (progress, subject) = observation::admitted(Some(vec![row(&root)]));
        let source = root.checked_unit().source().identity().clone();
        let claim = root.plan().subject().clone();
        root.admit_authority(
            PlanAuthoritySelection {
                source: &source,
                claim: &claim,
                observation_subject: &subject,
                observation_clock: &observation::clock(),
                observation_source: &observation::source(),
            },
            Some(progress),
            limits(),
            budget,
            native_locus(),
        )
    };
    let mut measured = native_budget();
    let original = handoff(&mut measured).unwrap();
    let need = measured.consumed();
    let read_need = measured.read_storage_consumed();
    assert!(need.work > 0 && need.storage > 0 && read_need > 0);
    let mut exact =
        NativeJoinBudget::new(need.work, need.storage).with_read_storage_limit(read_need);
    let admitted = handoff(&mut exact).unwrap();
    assert_eq!(admitted.admitted_order(), original.admitted_order());
    assert_eq!(admitted.progress_context(), original.progress_context());
    assert_eq!(exact.consumed(), need);
    assert_eq!(exact.read_storage_consumed(), read_need);
    for kind in [NativeJoinChargeKind::Work, NativeJoinChargeKind::Storage] {
        let mut short = NativeJoinBudget::new(
            need.work - u64::from(kind == NativeJoinChargeKind::Work),
            need.storage - u64::from(kind == NativeJoinChargeKind::Storage),
        )
        .with_read_storage_limit(read_need);
        let error = handoff(&mut short).unwrap_err();
        let PlanAuthorityRefusal::Budget {
            denied,
            locus,
            configured_bound,
            consumed,
        } = error
        else {
            panic!("the final materialization retains its native denial: {error:?}");
        };
        assert_eq!(denied.kind, kind);
        assert_eq!(short.first_unpaid(), Some(denied));
        assert_eq!(locus, short.first_unpaid_locus());
        assert_eq!(consumed, short.consumed());
        assert_eq!(
            configured_bound,
            match kind {
                NativeJoinChargeKind::Work => need.work - 1,
                NativeJoinChargeKind::Storage => need.storage - 1,
            }
        );
    }
    let read_denial = |error: PlanAuthorityRefusal, budget: &NativeJoinBudget, bound| {
        let (denied, configured) = match error {
            PlanAuthorityRefusal::ReadExhausted(error) => {
                assert_eq!(Some(error.locus), budget.first_unpaid_locus());
                (error.denied, error.configured_bound)
            }
            PlanAuthorityRefusal::ReadBudgetIncomplete(error) => (error.denied, error.limit),
            other => panic!("the strict reader preserves its original resource: {other:?}"),
        };
        assert_eq!(denied.resource, PackageReadResource::ReadStorage);
        assert_eq!(Some(denied.amount), budget.first_unpaid_read_storage());
        assert_eq!(configured, bound);
        assert!(matches!(
            budget.first_unpaid_locus().unwrap().source,
            quire_contract_model::NativeJoinLocusSource::PackageRead(_, _)
        ));
        assert!(budget.read_storage_consumed() <= bound);
    };
    for read_bound in [0, read_need - 1] {
        let mut short =
            NativeJoinBudget::new(need.work, need.storage).with_read_storage_limit(read_bound);
        read_denial(handoff(&mut short).unwrap_err(), &short, read_bound);
        assert!(short.consumed().work > 0);
        assert!(short.consumed().storage > 0);
    }
    let read_bound = read_need * 2 - 1;
    let mut reused =
        NativeJoinBudget::new(need.work * 2, need.storage * 2).with_read_storage_limit(read_bound);
    handoff(&mut reused).unwrap();
    assert_eq!(reused.consumed(), need);
    assert_eq!(reused.read_storage_consumed(), read_need);
    read_denial(handoff(&mut reused).unwrap_err(), &reused, read_bound);
    assert!(reused.consumed().work > need.work);
    assert!(reused.consumed().storage > need.storage);
    assert!(reused.read_storage_consumed() >= read_need);
    for kind in [NativeJoinChargeKind::Work, NativeJoinChargeKind::Storage] {
        let work_bound = need.work * 2 - u64::from(kind == NativeJoinChargeKind::Work);
        let storage_bound = need.storage * 2 - u64::from(kind == NativeJoinChargeKind::Storage);
        let mut reused = NativeJoinBudget::new(work_bound, storage_bound)
            .with_read_storage_limit(read_need * 2);
        handoff(&mut reused).unwrap();
        assert_eq!(reused.consumed(), need);
        let error = handoff(&mut reused).unwrap_err();
        let PlanAuthorityRefusal::Budget { denied, locus, configured_bound, consumed } = error else {
            panic!("the second complete handoff retains its original native denial: {error:?}");
        };
        assert_eq!(denied.kind, kind);
        assert_eq!(Some(denied), reused.first_unpaid());
        assert_eq!(locus, reused.first_unpaid_locus());
        assert_eq!(consumed, reused.consumed());
        assert!(consumed.work >= need.work && consumed.storage >= need.storage);
        assert_eq!(configured_bound, if kind == NativeJoinChargeKind::Work { work_bound } else { storage_bound });
    }
    let mut prefixed = NativeJoinBudget::new(need.work + 3, need.storage + 5)
        .with_read_storage_limit(read_need);
    prefixed.accounted_charge(NativeJoinChargeKind::Work, 3, native_locus()).unwrap();
    prefixed.accounted_charge(NativeJoinChargeKind::Storage, 5, native_locus()).unwrap();
    handoff(&mut prefixed).unwrap();
    assert_eq!(prefixed.consumed().work, need.work + 3);
    assert_eq!(prefixed.consumed().storage, need.storage + 5);
    assert_eq!(prefixed.read_storage_consumed(), read_need);
    }
}

#[trace("TC-1081", "FR-251-AC-46")]
#[test]
fn absent_progress_and_owner_absent_association_keep_original_typed_refusals() {
    let root = tick_root();
    let (_, subject) = observation::admitted(None);
    assert_eq!(
        admit(root, None, &subject, limits()).unwrap_err().code(),
        PlanAuthorityRefusalCode::MissingProgress
    );
    let root = tick_root();
    let (view, subject) = observation::admitted(None);
    match admit(root, Some(view), &subject, limits()).unwrap_err() {
        PlanAuthorityRefusal::Progress(error) => {
            assert_eq!(error.code(), authority::ErrorCode::MissingPremise)
        }
        other => panic!("wrong refusal {other:?}"),
    }
}

#[trace("TC-1081", "FR-251-AC-46")]
#[test]
fn independently_admitted_wrong_claims_and_time_facts_refuse_without_context() {
    for control in 0..13 {
        let root = tick_root();
        let mut selected = row(&root);
        let expected = match control {
            0 => {
                selected.model.push_str("_other");
                ProgressSelectionRefusalCode::Model
            }
            1 => {
                selected.unit = "s".to_owned();
                ProgressSelectionRefusalCode::Unit
            }
            2 => {
                selected.model_node = selected.unit_node.clone();
                ProgressSelectionRefusalCode::Node
            }
            3 => {
                selected.unit_node = selected.model_node.clone();
                ProgressSelectionRefusalCode::Node
            }
            4 => {
                selected.time_dimension = selected.unit_node.clone();
                ProgressSelectionRefusalCode::Node
            }
            5 => {
                selected.claim = progress::ClaimOccurrence::new(
                    selected.model_node.clone(),
                    selected.claim.occurrence(),
                );
                ProgressSelectionRefusalCode::Claim
            }
            6 => {
                selected.claim = progress::ClaimOccurrence::new(
                    selected.claim.clause_node().clone(),
                    &Origin::new(Role::new("other"), selected.claim.ordinal()),
                );
                ProgressSelectionRefusalCode::Claim
            }
            7 => {
                selected.claim = progress::ClaimOccurrence::new(
                    selected.claim.clause_node().clone(),
                    &Origin::new(
                        Role::new(selected.claim.role()),
                        selected.claim.ordinal() + 1,
                    ),
                );
                ProgressSelectionRefusalCode::Claim
            }
            8 => {
                selected.source = progress::TimeSource::Dense;
                ProgressSelectionRefusalCode::TimeSource
            }
            _ => {
                match &mut selected.source {
                    progress::TimeSource::Tick {
                        operation,
                        operation_node,
                        period_numerator,
                        period_denominator,
                    } => match control {
                        9 => {
                            operation.push_str("_other");
                        }
                        10 => {
                            *operation_node = selected.model_node.clone();
                        }
                        11 => *period_numerator = "3".to_owned(),
                        _ => *period_denominator = "2".to_owned(),
                    },
                    progress::TimeSource::Dense => panic!("genuine tick source"),
                }
                if control == 10 {
                    ProgressSelectionRefusalCode::Node
                } else {
                    ProgressSelectionRefusalCode::TimeSource
                }
            }
        };
        let (view, subject) = observation::admitted(Some(vec![selected]));
        match admit(root, Some(view), &subject, limits()).unwrap_err() {
            PlanAuthorityRefusal::Selection(code) => {
                assert_eq!(code, expected, "control {control}")
            }
            other => panic!("control {control}: {other:?}"),
        }
    }
}

#[trace("TC-1081", "FR-251-AC-46")]
#[test]
fn independent_source_subject_clock_and_foreign_emission_selections_refuse() {
    for control in 0..7 {
        let root = tick_root();
        let (view, subject) = observation::admitted(Some(vec![row(&root)]));
        let mut source = root.checked_unit().source().identity().clone();
        let mut expected_subject = subject.clone();
        let mut clock = observation::clock();
        let mut observation_source = observation::source();
        let claim = root.plan().subject().clone();
        let expected = match control {
            0 => {
                source.authority.push_str("_other");
                PlanAuthorityRefusalCode::Source
            }
            1 => {
                source.identity.push_str("_other");
                PlanAuthorityRefusalCode::Source
            }
            2 => {
                expected_subject.scope_identity = observation::id("window:other");
                PlanAuthorityRefusalCode::Selection
            }
            3 => {
                expected_subject.population_identity = observation::id("population:other");
                PlanAuthorityRefusalCode::Selection
            }
            4 => {
                clock = authority::clock::Selection::new(
                    observation::id("clock:other"),
                    observation::id("1"),
                );
                PlanAuthorityRefusalCode::Selection
            }
            5 => {
                clock = authority::clock::Selection::new(
                    observation::id("clock:monitor-events"),
                    observation::id("2"),
                );
                PlanAuthorityRefusalCode::Selection
            }
            _ => {
                observation_source = observation::id("source:other");
                PlanAuthorityRefusalCode::Selection
            }
        };
        let result = root.admit_authority(
            PlanAuthoritySelection {
                source: &source,
                claim: &claim,
                observation_subject: &expected_subject,
                observation_clock: &clock,
                observation_source: &observation_source,
            },
            Some(view),
            limits(),
            &mut native_budget(),
            native_locus(),
        );
        let error = result.unwrap_err();
        assert_eq!(error.code(), expected, "control {control}");
        if control == 6 {
            assert!(matches!(
                error,
                PlanAuthorityRefusal::Selection(ProgressSelectionRefusalCode::Source)
            ));
        }
    }
    let original = tick_root();
    let (view, subject) = observation::admitted(Some(vec![row(&original)]));
    let foreign = root_of(
        &authored("eventually[0 ms, 5 ms] holds(c.versionNumber = 3)"),
        "C",
        &target(),
    )
    .unwrap();
    let source = original.checked_unit().source().identity().clone();
    let claim = original.plan().subject().clone();
    let error = foreign
        .admit_authority(
            PlanAuthoritySelection {
                source: &source,
                claim: &claim,
                observation_subject: &subject,
                observation_clock: &observation::clock(),
                observation_source: &observation::source(),
            },
            Some(view),
            limits(),
            &mut native_budget(),
            native_locus(),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        PlanAuthorityRefusal::Claim | PlanAuthorityRefusal::Selection(_)
    ));
}

#[trace("TC-1081", "FR-251-AC-46", "FR-251-AC-48")]
#[test]
fn exact_canonical_bounds_succeed_and_one_short_preserves_owner_failures() {
    let root = tick_root();
    let (view, subject) = observation::admitted(Some(vec![row(&root)]));
    let order_size = root.model_step_order(limits().order).unwrap().bytes().len() as u64;
    let mut exact = limits();
    exact.order = quire_canonical::Limits::new(order_size);
    let context = admit(root, Some(view), &subject, exact).unwrap();
    assert_eq!(context.admitted_order().len() as u64, order_size);
    assert_eq!(
        context.progress_context().as_ptr(),
        context
            .progress()
            .association_canonical_bytes()
            .unwrap()
            .as_ptr()
    );
    let root = tick_root();
    let (view, subject) = observation::admitted(Some(vec![row(&root)]));
    let mut short = exact;
    short.order = quire_canonical::Limits::new(order_size - 1);
    assert!(matches!(
        admit(root, Some(view), &subject, short),
        Err(PlanAuthorityRefusal::Order(_))
    ));
}

#[trace("TC-1081", "FR-251-AC-46")]
#[test]
fn progress_owner_admission_keeps_its_exact_and_one_short_resource_limits() {
    let root = tick_root();
    let (view, _) = observation::admitted(Some(vec![row(&root)]));
    let owner_document_bytes = view.bytes().len();
    let mut limits = authority::Limits::owner_max();
    limits.max_output_bytes = owner_document_bytes;
    let (exact, _) = observation::admitted_with_progress_limits(Some(vec![row(&root)]), limits)
        .expect("exact owner admission output ceiling");
    assert_eq!(exact.bytes().len(), owner_document_bytes);
    limits.max_output_bytes -= 1;
    let error =
        observation::admitted_with_progress_limits(Some(vec![row(&root)]), limits).unwrap_err();
    assert_eq!(error.code(), authority::ErrorCode::ResourceIncomplete);
}

#[trace("TC-1081", "FR-251-AC-45", "FR-251-AC-47")]
#[test]
fn final_emission_retains_actual_read_evidence_without_fixture_reinjection() {
    let roots = [
        tick_root(),
        root_of(
            &authored("eventually[0 ms, 3 ms] holds(c.versionNumber = 3)"),
            "C",
            &target(),
        )
        .unwrap(),
    ];
    for root in roots {
        match quire_contract_model::read_checked_package(
            root.emission().package().bytes(),
            LIMITS,
            root.emission().evidence(),
        ) {
            quire_contract_model::CheckedPackageDispatchResult::AdmittedV2(package) => {
                assert_eq!(package.monitor_plans().len(), 1)
            }
            other => {
                panic!("actual retained final evidence must admit without reinjection: {other:?}")
            }
        }
    }
}

#[trace("TC-1081", "FR-251-AC-46")]
#[test]
fn changing_only_the_independent_checked_claim_node_role_or_ordinal_refuses_claim() {
    for control in 0..3 {
        let root = tick_root();
        let (view, subject) = observation::admitted(Some(vec![row(&root)]));
        let source = root.checked_unit().source().identity().clone();
        let original = root.plan().subject();
        let selected_node = if control == 0 {
            root.checked_unit().package().graph().time_sources()[0]
                .model_node
                .clone()
        } else {
            original.clause_node().clone()
        };
        let original_occurrence = original.occurrence().origin();
        let occurrence = Origin::new(
            if control == 1 {
                Role::new("other")
            } else {
                original_occurrence.role().clone()
            },
            original_occurrence.ordinal() + u64::from(control == 2),
        );
        let claim = qsl_replay::MonitorSubject::new(
            original.package().clone(),
            selected_node.clone(),
            qsl_foundation::source::provenance::OccurrenceKey::new(selected_node, occurrence),
            original.clause_name(),
        );
        let error = root
            .admit_authority(
                PlanAuthoritySelection {
                    source: &source,
                    claim: &claim,
                    observation_subject: &subject,
                    observation_clock: &observation::clock(),
                    observation_source: &observation::source(),
                },
                Some(view),
                limits(),
                &mut native_budget(),
                native_locus(),
            )
            .unwrap_err();
        assert!(
            matches!(error, PlanAuthorityRefusal::Claim),
            "control {control}: {error:?}"
        );
    }
}

/// Selection-prefix evidence only; complete materialization accounting also
/// requires the owner canonical preflight and retained association interfaces.
#[trace("TC-1081", "FR-251-AC-49")]
#[test]
fn selection_prefix_pays_source_comparison_before_inspection_and_keeps_prior_charges() {
    use quire_contract_model::{NativeJoinBudget, NativeJoinChargeKind};
    for one_short in [false, true] {
        let root = tick_root();
        let (view, subject) = observation::admitted(Some(vec![row(&root)]));
        let mut source = root.checked_unit().source().identity().clone();
        source.authority = "other".to_owned();
        let claim = root.plan().subject().clone();
        // Eleven units belong to an earlier preparation phase; three cover
        // this call's entry check and the first unequal byte pair.
        let mut budget = NativeJoinBudget::new(if one_short { 13 } else { 14 }, 100);
        let locus = native_locus();
        budget
            .accounted_charge(NativeJoinChargeKind::Work, 11, locus)
            .unwrap();
        let error = root
            .admit_authority(
                PlanAuthoritySelection {
                    source: &source,
                    claim: &claim,
                    observation_subject: &subject,
                    observation_clock: &observation::clock(),
                    observation_source: &observation::source(),
                },
                Some(view),
                limits(),
                &mut budget,
                locus,
            )
            .unwrap_err();
        if one_short {
            match error {
                PlanAuthorityRefusal::Budget {
                    denied,
                    locus: first,
                    configured_bound,
                    consumed,
                } => {
                    assert_eq!(denied.kind, NativeJoinChargeKind::Work);
                    assert_eq!(denied.amount, 2);
                    assert_eq!(configured_bound, 13);
                    assert_eq!(first, Some(locus));
                    assert_eq!(consumed.work, 12);
                    assert_eq!(budget.consumed(), consumed);
                    assert_eq!(budget.first_unpaid(), Some(denied));
                }
                other => panic!("wrong shared-budget refusal: {other:?}"),
            }
        } else {
            assert!(matches!(error, PlanAuthorityRefusal::Source));
            assert_eq!(budget.consumed().work, 14);
            assert_eq!(budget.first_unpaid(), None);
        }
        assert_eq!(budget.consumed().storage, 0);
    }
}

#[trace("TC-1081", "FR-251-AC-49")]
#[test]
fn selection_prefix_denies_zero_remaining_storage_before_owning_a_walk_stack() {
    use quire_contract_model::{NativeJoinBudget, NativeJoinChargeKind};
    for prior_storage in [0, 3] {
        let root = tick_root();
        let (view, subject) = observation::admitted(Some(vec![row(&root)]));
        let source = root.checked_unit().source().identity().clone();
        let claim = root.plan().subject().clone();
        let mut budget = NativeJoinBudget::new(10_000, prior_storage);
        let locus = native_locus();
        budget
            .accounted_charge(NativeJoinChargeKind::Storage, prior_storage.into(), locus)
            .unwrap();
        let error = root
            .admit_authority(
                PlanAuthoritySelection {
                    source: &source,
                    claim: &claim,
                    observation_subject: &subject,
                    observation_clock: &observation::clock(),
                    observation_source: &observation::source(),
                },
                Some(view),
                limits(),
                &mut budget,
                locus,
            )
            .unwrap_err();
        match error {
            PlanAuthorityRefusal::Budget {
                denied,
                locus: first,
                configured_bound,
                consumed,
            } => {
                assert_eq!(denied.kind, NativeJoinChargeKind::Storage);
                assert_eq!(denied.amount, 1);
                assert_eq!(first, Some(locus));
                assert_eq!(configured_bound, prior_storage);
                assert_eq!(consumed.storage, prior_storage);
                assert_eq!(budget.consumed(), consumed);
                assert_eq!(budget.first_unpaid(), Some(denied));
            }
            other => panic!("walk storage was not precharged: {other:?}"),
        }
    }
}

#[trace("TC-1081", "FR-251-AC-49")]
#[test]
fn selection_prefix_composes_two_attempts_without_restoring_spent_work() {
    use quire_contract_model::{NativeJoinBudget, NativeJoinChargeKind};
    let locus = native_locus();
    let mut budget = NativeJoinBudget::new(5, 100);
    for attempt in 0..2 {
        let root = tick_root();
        let (view, subject) = observation::admitted(Some(vec![row(&root)]));
        let mut source = root.checked_unit().source().identity().clone();
        source.authority = "other".to_owned();
        let claim = root.plan().subject().clone();
        let error = root
            .admit_authority(
                PlanAuthoritySelection {
                    source: &source,
                    claim: &claim,
                    observation_subject: &subject,
                    observation_clock: &observation::clock(),
                    observation_source: &observation::source(),
                },
                Some(view),
                limits(),
                &mut budget,
                locus,
            )
            .unwrap_err();
        if attempt == 0 {
            assert!(matches!(error, PlanAuthorityRefusal::Source));
            assert_eq!(budget.consumed().work, 3);
        } else {
            match error {
                PlanAuthorityRefusal::Budget {
                    denied,
                    configured_bound,
                    consumed,
                    locus: first,
                } => {
                    assert_eq!(denied.kind, NativeJoinChargeKind::Work);
                    assert_eq!(denied.amount, 2);
                    assert_eq!(configured_bound, 5);
                    assert_eq!(consumed.work, 4);
                    assert_eq!(first, Some(locus));
                }
                other => panic!("second attempt restored an allowance: {other:?}"),
            }
        }
    }
}

#[trace("TC-1081", "FR-251-AC-50")]
#[test]
fn dense_authority_borrows_original_time_source_without_tick_period() {
    let root = root_of(
        &authored("eventually[0 ms, 3 ms] holds(c.versionNumber = 3)"),
        "C",
        &target(),
    )
    .unwrap();
    let (view, subject) = observation::admitted(Some(vec![row(&root)]));
    let context = admit(root, Some(view), &subject, limits()).unwrap();
    assert!(context
        .root()
        .checked_unit()
        .package()
        .graph()
        .time_sources()
        .iter()
        .any(|source| std::ptr::eq(source, context.checked_time_source())));
    assert!(matches!(
        context.checked_time_source().source,
        TimeSourceKind::Dense
    ));
    assert!(context.checked_tick_period().is_none());
}

#[trace("TC-1081", "FR-251-AC-46", "FR-251-AC-50")]
#[test]
fn changed_period_components_are_checked_at_the_same_emission_claim_clock() {
    for numerator_changed in [true, false] {
        let root = tick_root();
        let mut changed = row(&root);
        let progress::TimeSource::Tick {
            period_numerator,
            period_denominator,
            ..
        } = &mut changed.source
        else {
            panic!("genuine checked tick association");
        };
        if numerator_changed {
            *period_numerator = "3".to_owned();
        } else {
            *period_denominator = "2".to_owned();
        }
        let (view, subject) = observation::admitted(Some(vec![changed.clone()]));
        let mut read_denied = limits();
        read_denied.read.nodes = 0;
        assert!(
            matches!(
                admit(root, Some(view), &subject, read_denied),
                Err(PlanAuthorityRefusal::ReadIncomplete(_))
            ),
            "the changed period is not rejected by an uncharged pre-read formatter"
        );
        let root = tick_root();
        let (view, subject) = observation::admitted(Some(vec![changed]));
        assert!(
            matches!(
                admit(root, Some(view), &subject, limits()),
                Err(PlanAuthorityRefusal::Selection(
                    ProgressSelectionRefusalCode::TimeSource
                ))
            ),
            "the genuine bound clock must reject the changed period component"
        );
    }
}
