// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical requests preserve the external value-provider boundary.
//!
//! The provider is a deterministic I/O-boundary double. Counterexamples still
//! pass through the production native evaluator and replay; these tests do
//! not claim an SMT solver cross-check or genuine CG negotiation coverage.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze, AnalyzeLimits, AnalyzeOutcome, AnalyzeRequest, ClaimItem, Counterexample, Engine,
    Engines, ItemCounterexample, NegotiatedItem, SourceProvision, SubjectBinding,
};
use qsl_analyze::external_provider::{ExternalProvider, ProviderRequest, ProviderVerdict};
use qsl_analyze::provider::{self, ProviderResults};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_package::CheckedPackage;
use qsl_replay::value_check::ValueTarget;
use qsl_replay::{Certification, IncompleteCause, InconclusiveCause, ProofBasis, TerminalValue};
use qsl_route::request::ExtentClassification;
use qsl_route::routing::Disposition;
use qsl_route::{Candidate, Mode, ProviderOrigin};
use qsl_semantics::check::Capability;
use quire_exact::{Cancel, CancelCause, Integer, Value};

const EXTERNAL: &str = "qsl.canonical-test-provider";
const UNBOUNDED: &str = "function q using v(a: Integer, b: Integer): Integer pure { a div b }";
const FINITE_VIOLATED: &str =
    "function q using v(a: Int[0,3], b: Int[0,1]): Integer pure { a div b }";
const FINITE_HOLDING: &str =
    "function q using v(a: Int[0,3], b: Int[1,2]): Integer pure { a div b }";

struct Fixture {
    package: CheckedPackage,
    source: Vec<u8>,
    claim: OccurrenceKey,
}

impl Fixture {
    fn new(declaration: &str) -> Self {
        let (package, source) = crate::value_validity::package_with_source(declaration);
        let claims = package
            .graph()
            .requirements()
            .iter()
            .filter(|(_, record)| {
                record.requirements().kind() == Capability::ValueValidity
                    && record.owner() == Some("q")
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        let [claim] = claims.as_slice() else {
            panic!("one producer-recorded division claim: {claims:?}")
        };
        ValueTarget::select(&package, "q", claim)
            .expect("the producer's division occurrence selects a real target");
        Self {
            package,
            source,
            claim: claim.clone(),
        }
    }

    fn item(&self, engines: &Engines, extent: ExtentClassification) -> NegotiatedItem {
        let mut registry = qsl_route::Registry::new();
        for descriptor in engines.registrations() {
            registry
                .register(descriptor.clone())
                .expect("unique registered backends");
        }
        let backend = BackendId::new(EXTERNAL);
        let qsl_route::CandidateOutcome::Candidates(candidates) = registry.candidates_for_evidence(
            Capability::ValueValidity,
            &qsl_route::ItemEvidence::Absent,
            Some(&backend),
        ) else {
            panic!("the external provider is a registered candidate")
        };
        let [candidate] = candidates.candidates() else {
            panic!("one named registered external candidate")
        };
        let mut item = ClaimItem::new(
            &self.package,
            RequestIndex::new(7),
            self.claim.clone(),
            Capability::ValueValidity,
            extent,
        )
        .expect("the checked constructor admits the producer's claim");
        item.backend = Some(backend);
        item.candidates = vec![candidate.clone()];
        item.options.required = true;
        NegotiatedItem {
            item,
            disposition: Disposition::Supported(candidate.clone()),
            diagnostic: None,
        }
    }
}

#[derive(Debug)]
struct RecordingProvider {
    verdict: ProviderVerdict,
    calls: Arc<AtomicUsize>,
    cancel_during_check: bool,
}

impl ExternalProvider for RecordingProvider {
    fn check(&self, _request: &ProviderRequest<'_, '_>, cancel: &Cancel) -> ProviderVerdict {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.cancel_during_check {
            cancel.cancel(CancelCause::Deadline);
        }
        self.verdict.clone()
    }
}

fn engines(
    verdict: ProviderVerdict,
    bounded: bool,
    origin: ProviderOrigin,
    cancel_during_check: bool,
) -> (Engines, Arc<AtomicUsize>) {
    let mut advertised = vec![(
        Some(Capability::ValueValidity.to_wire()),
        Some(Mode::Unbounded.to_wire()),
    )];
    if bounded {
        advertised.push((
            Some(Capability::ValueValidity.to_wire()),
            Some(Mode::Bounded.to_wire()),
        ));
    }
    let descriptor = qsl_route::descriptor_from_manifest(
        Candidate::new(BackendId::new(EXTERNAL)),
        origin,
        advertised,
        qsl_route::ManifestMembers {
            domains: Some(&[qsl_foundation::bound::DomainKind::Integer.to_wire()]),
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
    .expect("the descriptor advertises the real external value arm");
    let calls = Arc::new(AtomicUsize::new(0));
    let mut engines = Engines::qsl().expect("QSL manifests register");
    engines
        .with(
            Engine::external(
                descriptor,
                Box::new(RecordingProvider {
                    verdict,
                    calls: calls.clone(),
                    cancel_during_check,
                }),
            )
            .expect("the provider advertises unbounded value-validity"),
        )
        .expect("the external backend is distinct");
    (engines, calls)
}

fn both_entries(
    fixture: &Fixture,
    engines: &Engines,
    extent: ExtentClassification,
    pre_cancelled: bool,
) -> (AnalyzeOutcome, ProviderResults) {
    let packages = qsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("no domain packages are required");
    let snapshots = qsl_semantics::model::observation::DocumentProvision::default();
    let request = AnalyzeRequest {
        binding: SubjectBinding {
            packages: &packages,
            snapshots: &snapshots,
            initial: Vec::new(),
            universes: Vec::new(),
        },
        provision: SourceProvision::single(fixture.package.graph().source(), &fixture.source),
        items: vec![fixture.item(engines, extent)],
    };
    let limits = AnalyzeLimits::unbounded();
    let analyze_cancel = Cancel::new();
    let provider_cancel = Cancel::new();
    if pre_cancelled {
        analyze_cancel.cancel(CancelCause::Requested);
        provider_cancel.cancel(CancelCause::Requested);
    }
    let analyzed = analyze(
        &fixture.package,
        &request,
        engines,
        &limits,
        &analyze_cancel,
    )
    .expect("the canonical request admits")
    .into_value();
    let served = provider::run(
        &fixture.package,
        &request.binding,
        &request.provision,
        &request.items,
        engines,
        &limits,
        &provider_cancel,
    );
    assert_eq!(analyzed.records.len(), 1);
    assert_eq!(analyzed.records, served.records);
    assert_eq!(analyzed.records[0].request_index(), RequestIndex::new(7));
    (analyzed, served)
}

fn large_division_assignment() -> Vec<Value> {
    vec![
        Value::Integer(Integer::from(1_i128 << 100)),
        Value::Integer(Integer::from(0_i64)),
    ]
}

fn assert_division_assignment(assignment: &[Value]) {
    let [Value::Integer(dividend), Value::Integer(divisor)] = assignment else {
        panic!("the two reported mathematical integers: {assignment:?}")
    };
    assert_eq!(dividend, &Integer::from(1_i128 << 100));
    assert_eq!(divisor, &Integer::from(0_i64));
}

fn assignment_counterexample(
    counterexamples: &[ItemCounterexample],
) -> &qsl_analyze::value_check::ValueCounterexample {
    let [item] = counterexamples else {
        panic!("one reproduced value counterexample: {counterexamples:?}")
    };
    assert_eq!(item.request_index, RequestIndex::new(7));
    assert_eq!(item.backend.as_str(), EXTERNAL);
    let Counterexample::Assignments(counterexample) = &item.counterexample else {
        panic!("the native-replayed assignment: {:?}", item.counterexample)
    };
    counterexample
}

#[trace("TC-1337", "FR-281-AC-14", "TC-761", "FR-280-AC-2")]
#[test]
fn external_unbounded_counterexamples_are_called_and_replayed_by_both_entries() {
    let fixture = Fixture::new(UNBOUNDED);
    let (engines, calls) = engines(
        ProviderVerdict::Refuted {
            counterexample: large_division_assignment(),
        },
        false,
        ProviderOrigin::Linked,
        false,
    );
    let (analyzed, served) = both_entries(
        &fixture,
        &engines,
        ExtentClassification::Unbounded {
            finite_bound_available: false,
        },
        false,
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "each entry calls the provider"
    );
    assert_eq!(analyzed.records[0].value(), &TerminalValue::Refuted);
    assert_eq!(
        analyzed.records[0].backend().map(BackendId::as_str),
        Some(EXTERNAL)
    );
    let target = ValueTarget::select(&fixture.package, "q", &fixture.claim)
        .expect("the replay target is the producer's division claim");
    for result in [&analyzed.counterexamples, &served.counterexamples] {
        let kept = assignment_counterexample(result);
        assert_eq!(kept.claim, fixture.claim);
        assert_division_assignment(kept.reproduced.assignment());
        let replayed = target
            .replay(
                kept.reproduced.assignment(),
                kept.reproduced.violation(),
                &Cancel::new(),
            )
            .expect("the native evaluator admits the assignment")
            .expect("the violation reproduces independently");
        assert_division_assignment(replayed.assignment());
        assert_eq!(replayed.violation(), kept.reproduced.violation());
    }
}

#[trace("TC-1337", "FR-281-AC-15", "TC-761", "FR-280-AC-2")]
#[test]
fn a_bounded_external_candidate_keeps_its_unknown_verdict_instead_of_a_native_result() {
    let fixture = Fixture::new(FINITE_VIOLATED);
    let native = qsl_analyze::value_check::check_value(
        &qsl_analyze::value_check::ValueRequest {
            target: ValueTarget::select(&fixture.package, "q", &fixture.claim)
                .expect("the finite native target exists"),
            max_cases: None,
            reported: None,
        },
        qsl_analyze::value_check::manifest::DEFAULT_MAX_CASES,
        &Cancel::new(),
    )
    .expect("the native control evaluates");
    assert!(matches!(
        native,
        qsl_analyze::value_check::ValueOutcome::Refuted(_)
    ));
    let (engines, calls) = engines(
        ProviderVerdict::Unknown,
        true,
        ProviderOrigin::Linked,
        false,
    );
    let (analyzed, served) = both_entries(&fixture, &engines, ExtentClassification::Bounded, false);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(matches!(
        analyzed.records[0].value(),
        TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive { .. })
    ));
    assert_eq!(
        analyzed.records[0].backend().map(BackendId::as_str),
        Some(EXTERNAL)
    );
    assert!(analyzed.counterexamples.is_empty());
    assert!(served.counterexamples.is_empty());
}

#[trace("TC-1337", "FR-281-AC-15", "TC-761", "FR-280-AC-2")]
#[test]
fn a_nonviolating_external_assignment_is_never_promoted_to_refuted() {
    let fixture = Fixture::new(UNBOUNDED);
    let (engines, calls) = engines(
        ProviderVerdict::Refuted {
            counterexample: vec![
                Value::Integer(Integer::from(7_i64)),
                Value::Integer(Integer::from(2_i64)),
            ],
        },
        false,
        ProviderOrigin::Linked,
        false,
    );
    let (analyzed, served) = both_entries(
        &fixture,
        &engines,
        ExtentClassification::Unbounded {
            finite_bound_available: false,
        },
        false,
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(matches!(
        analyzed.records[0].value(),
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
    ));
    assert!(analyzed.counterexamples.is_empty());
    assert!(served.counterexamples.is_empty());
}

#[trace("TC-1337", "FR-281-AC-18", "TC-761", "FR-280-AC-2")]
#[test]
fn external_proofs_preserve_the_selected_provider_origin_through_both_entries() {
    let fixture = Fixture::new(FINITE_HOLDING);
    for (origin, certification) in [
        (ProviderOrigin::Linked, Certification::Uncertified),
        (ProviderOrigin::Process, Certification::Trusted),
    ] {
        let (engines, calls) = engines(ProviderVerdict::Proved, true, origin, false);
        let (analyzed, served) =
            both_entries(&fixture, &engines, ExtentClassification::Bounded, false);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            analyzed.records[0].value(),
            &TerminalValue::Proved {
                basis: ProofBasis::Inductive { depth: 0 },
                certification,
            }
        );
        assert_eq!(
            analyzed.records[0].backend().map(BackendId::as_str),
            Some(EXTERNAL)
        );
        assert!(analyzed.counterexamples.is_empty());
        assert!(served.counterexamples.is_empty());
    }
}

#[trace("TC-1337", "FR-281-AC-19", "TC-763", "FR-281-AC-12")]
#[test]
fn pre_cancelled_external_requests_do_not_invoke_the_provider() {
    let fixture = Fixture::new(UNBOUNDED);
    let (engines, calls) = engines(
        ProviderVerdict::Refuted {
            counterexample: large_division_assignment(),
        },
        false,
        ProviderOrigin::Linked,
        false,
    );
    let (analyzed, served) = both_entries(
        &fixture,
        &engines,
        ExtentClassification::Unbounded {
            finite_bound_available: false,
        },
        true,
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        analyzed.records[0].value(),
        &TerminalValue::Incomplete(IncompleteCause::Cancelled {
            source: CancelCause::Requested
        })
    );
    assert!(analyzed.counterexamples.is_empty());
    assert!(served.counterexamples.is_empty());
}

#[trace("TC-1337", "FR-281-AC-19", "TC-763", "FR-281-AC-12")]
#[test]
fn cancellation_from_each_external_callback_keeps_no_candidate_counterexample() {
    let fixture = Fixture::new(UNBOUNDED);
    let (engines, calls) = engines(
        ProviderVerdict::Refuted {
            counterexample: large_division_assignment(),
        },
        false,
        ProviderOrigin::Linked,
        true,
    );
    let (analyzed, served) = both_entries(
        &fixture,
        &engines,
        ExtentClassification::Unbounded {
            finite_bound_available: false,
        },
        false,
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        analyzed.records[0].value(),
        &TerminalValue::Incomplete(IncompleteCause::Cancelled {
            source: CancelCause::Deadline
        })
    );
    assert_eq!(
        analyzed.records[0].backend().map(BackendId::as_str),
        Some(EXTERNAL)
    );
    assert!(analyzed.counterexamples.is_empty());
    assert!(served.counterexamples.is_empty());
}
