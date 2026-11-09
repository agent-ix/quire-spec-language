// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-5 routing over operation-owned subjects and source provisions.
//! Caller-supplied dispositions exercise the QSL provider boundary; actual
//! codegen negotiation belongs to its external integration tests. Other
//! TC-762 engine members remain separately covered.

use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze, Aggregate, AnalyzeLimits, AnalyzeOutcome, AnalyzeRequest, ClaimItem, Engines,
    ItemForm, ItemOptions, NegotiatedItem, ProbabilisticItem, ProbabilisticLimits, TemporalItem,
};
use qsl_analyze::exact_probabilistic::ExactProbLimits;
use qsl_analyze::provider;
use qsl_eval::simulation::Limits;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::{
    Certification, IncompleteCause, ModelCheckLimit, ModelCheckLimits, ProofBasis,
    ProofRefusalCause, TerminalRecord, TerminalValue,
};
use qsl_route::{request::ExtentClassification, routing::Disposition, Candidate, EvidenceKind};
use qsl_semantics::check::Capability;
use quire_exact::{Cancel, CancelCause, Integer, Rational};
use quire_semantic_value::catalog::CatalogCode;

use crate::prob_world::{self, World};

const CLAIMS: &str =
    "probabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin \
     { probability >= 0.95 [ eventually holds(m.delivered) ] }\n";

fn item(world: &World, index: usize, limits: ExactProbLimits) -> NegotiatedItem {
    let candidate = Candidate::new(BackendId::new("en-5"));
    NegotiatedItem {
        item: ClaimItem {
            request_index: RequestIndex::new(index),
            claim: world
                .claim("Likely")
                .occurrence()
                .expect("authored claim")
                .clone(),
            kind: Capability::ProbabilisticSatisfaction,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: true,
            },
            backend: Some(BackendId::new("en-5")),
            candidates: vec![candidate.clone()],
            options: ItemOptions {
                required: true,
                ..ItemOptions::default()
            },
            form: Some(ItemForm::Probabilistic(ProbabilisticItem {
                evidence: EvidenceKind::Exact,
                limits: ProbabilisticLimits::Exact(limits),
                method: None,
                seed: None,
                sampler: None,
                max_witnesses: None,
                symmetry: Vec::new(),
                timed_sampling: None,
            })),
        },
        disposition: Disposition::Supported(candidate),
        diagnostic: None,
    }
}

fn direct(world: &World, items: Vec<NegotiatedItem>, cancel: &Cancel) -> AnalyzeOutcome {
    analyze(
        &world.package,
        &AnalyzeRequest {
            binding: world.binding(),
            provision: world.provision.clone(),
            items,
        },
        &Engines::qsl().expect("the engines register"),
        &AnalyzeLimits::unbounded(),
        cancel,
    )
    .expect("the binding admits")
    .into_value()
}

fn served(world: &World, items: &[NegotiatedItem], cancel: &Cancel) -> Vec<TerminalRecord> {
    provider::run(
        &world.package,
        &world.binding(),
        &world.provision,
        items,
        &Engines::qsl().expect("the engines register"),
        &AnalyzeLimits::unbounded(),
        cancel,
    )
    .records
}

fn one_state() -> ExactProbLimits {
    ExactProbLimits {
        model: ModelCheckLimits {
            limits: Limits::default().with_max_states(1),
            ..ModelCheckLimits::default()
        },
        ..ExactProbLimits::default()
    }
}

/// The canonical exact member and source provision return equal certified
/// records. The caller supplies the negotiation disposition; real codegen
/// negotiation remains external integration evidence.
#[trace("TC-761", "TC-762", "FR-280-AC-2", "FR-281-AC-1")]
#[test]
fn exact_claim_is_certified_and_equal_through_analyze_and_provider() {
    let world = World::new(&prob_world::link(CLAIMS));
    let routed = item(&world, 7, ExactProbLimits::default());
    let analyzed = direct(&world, vec![routed.clone()], &Cancel::new());
    let records = served(&world, &[routed], &Cancel::new());
    assert_eq!(analyzed.records, records);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].request_index(), RequestIndex::new(7));
    assert_eq!(records[0].backend().map(BackendId::as_str), Some("en-5"));
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        certification,
    } = records[0].value()
    else {
        panic!("expected an exact proof, got {:?}", records[0].value());
    };
    assert_eq!(*certification, Certification::Certified);
    assert_eq!(
        value.value(),
        &Rational::new(Integer::from(24_i64), Integer::from(25_i64)).unwrap()
    );
    assert_eq!(analyzed.aggregate, Ok(Aggregate::Attainable));
}

/// The canonical provider and analyze entries preserve EN-5's typed budget
/// stop over the same source provision and caller disposition.
#[trace("TC-761", "TC-763", "FR-280-AC-2", "FR-281-AC-6")]
#[test]
fn exact_state_budget_settles_incomplete_through_both_entries() {
    let world = World::new(&prob_world::link(CLAIMS));
    let routed = item(&world, 3, one_state());
    let analyzed = direct(&world, vec![routed.clone()], &Cancel::new());
    let records = served(&world, &[routed], &Cancel::new());
    assert_eq!(analyzed.records, records);
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ExactProbMaxStates,
            value: 1,
        })
    );
}

#[trace("TC-761", "FR-280-AC-6")]
#[test]
fn reversed_exact_items_keep_their_records_and_indices() {
    let world = World::new(&prob_world::link(CLAIMS));
    let first = item(&world, 9, ExactProbLimits::default());
    let second = item(&world, 2, one_state());
    let forward = served(&world, &[first.clone(), second.clone()], &Cancel::new());
    let reverse = served(&world, &[second, first], &Cancel::new());
    assert_eq!(forward.len(), 2);
    assert_eq!(reverse, forward.into_iter().rev().collect::<Vec<_>>());
    assert_eq!(reverse[0].request_index(), RequestIndex::new(2));
    assert_eq!(reverse[1].request_index(), RequestIndex::new(9));
}

/// An exact member selected on EN-4 must not silently become measured or
/// invoke EN-5 under a different producing backend.
#[trace("TC-762")]
#[test]
fn exact_member_on_a_sampling_backend_is_unsupported_and_affects_required_aggregation() {
    let world = World::new(&prob_world::link(CLAIMS));
    let mut wrong = item(&world, 4, ExactProbLimits::default());
    wrong.item.backend = Some(BackendId::new("en-4"));
    wrong.item.candidates = vec![Candidate::new(BackendId::new("en-4"))];
    wrong.disposition = Disposition::Supported(Candidate::new(BackendId::new("en-4")));
    let required = direct(&world, vec![wrong.clone()], &Cancel::new());
    assert_eq!(required.records.len(), 1);
    assert!(matches!(
        required.records[0].value(),
        TerminalValue::Unsupported(_)
    ));
    assert_eq!(required.records[0].backend(), None);
    assert_eq!(
        required.records[0].diagnostic(),
        Some(CatalogCode::new(
            "unsupported_projection",
            "unsupported-requested-capability",
        ))
    );
    assert_eq!(
        required.aggregate,
        Ok(Aggregate::Unavailable {
            request_index: RequestIndex::new(4)
        })
    );
    wrong.item.options.required = false;
    let optional = direct(&world, vec![wrong.clone()], &Cancel::new());
    assert_eq!(optional.aggregate, Ok(Aggregate::Attainable));
    assert_eq!(optional.records, served(&world, &[wrong], &Cancel::new()));
}

#[trace("TC-761")]
#[test]
fn mixed_clause_and_exact_members_keep_input_order() {
    let world = World::new(&prob_world::link(CLAIMS));
    let candidate = Candidate::new(BackendId::new("en-1"));
    let absent_clause = NegotiatedItem {
        item: ClaimItem {
            request_index: RequestIndex::new(8),
            claim: world
                .claim("Likely")
                .occurrence()
                .expect("authored claim")
                .clone(),
            kind: Capability::TemporalSatisfaction,
            extent: ExtentClassification::Bounded,
            backend: Some(BackendId::new("en-1")),
            candidates: vec![candidate.clone()],
            options: ItemOptions {
                required: false,
                ..ItemOptions::default()
            },
            form: Some(ItemForm::Temporal(TemporalItem::default())),
        },
        disposition: Disposition::Supported(candidate),
        diagnostic: None,
    };
    let items = vec![
        item(&world, 5, ExactProbLimits::default()),
        absent_clause,
        item(&world, 1, one_state()),
    ];
    let analyzed = direct(&world, items.clone(), &Cancel::new());
    let records = provider::run(
        &world.package,
        &world.binding(),
        &world.provision,
        &items,
        &Engines::qsl().expect("the engines register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .records;
    assert_eq!(records, analyzed.records);
    assert_eq!(
        records
            .iter()
            .map(TerminalRecord::request_index)
            .collect::<Vec<_>>(),
        [
            RequestIndex::new(5),
            RequestIndex::new(8),
            RequestIndex::new(1)
        ]
    );
    assert!(matches!(records[0].value(), TerminalValue::Proved { .. }));
    assert!(matches!(records[1].value(), TerminalValue::Declined { .. }));
    assert!(matches!(records[2].value(), TerminalValue::Incomplete(_)));
    assert_eq!(analyzed.aggregate, Ok(Aggregate::Attainable));
}

#[trace("TC-761", "FR-280-AC-6")]
#[test]
fn missing_snapshot_declines_each_exact_item_without_a_backend() {
    let world = World::new(&prob_world::link(CLAIMS));
    let mut binding = world.binding();
    binding.initial[0].identity.push_str("-missing");
    let records = provider::run(
        &world.package,
        &binding,
        &world.provision,
        &[
            item(&world, 6, ExactProbLimits::default()),
            item(&world, 0, one_state()),
        ],
        &Engines::qsl().expect("the engines register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .records;
    assert_eq!(records.len(), 2);
    for (record, index) in records.iter().zip([6, 0]) {
        assert_eq!(record.request_index(), RequestIndex::new(index));
        assert_eq!(record.backend(), None);
        assert!(matches!(
            record.value(),
            TerminalValue::Declined {
                cause: ProofRefusalCause::IncompleteInput,
                ..
            }
        ));
        assert_eq!(
            record.diagnostic(),
            Some(CatalogCode::new(
                "unavailable_observation",
                "missing-required-artifact"
            ))
        );
    }
}

#[trace("TC-762")]
#[test]
fn cancellation_preserves_the_cause_for_exact_members() {
    let world = World::new(&prob_world::link(CLAIMS));
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let records = served(
        &world,
        &[item(&world, 0, ExactProbLimits::default())],
        &cancel,
    );
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].value(),
        &TerminalValue::Incomplete(IncompleteCause::Cancelled {
            source: CancelCause::Requested
        })
    );
    assert_eq!(records[0].backend().map(BackendId::as_str), Some("en-5"));
}
