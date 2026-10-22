// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-773 / FR-289-AC-3: a client outside the crate builds a terminal record
//! only from a replay result and a producing backend identity, then reads it
//! through `qsl_replay`'s root re-exports alone.

use ix_trace_rs::trace;
use qsl_foundation::BackendId;
use qsl_replay::{
    read_backend_provider_envelope, BackendProviderSource, CallSiteRefusal, Category, DeclineCode,
    EvaluatedValue, InconclusiveCause, InputArmResult, InternalFault, ProofRefusalCause,
    ReplayRefusal, ReplayResult, ReportedInconclusiveCause, RequestIndex, TerminalRecord,
    TerminalValue, Verdict, WitnessArmResult, WitnessCheck,
};

fn violating_replay() -> ReplayResult {
    ReplayResult::Witness(WitnessArmResult::settle_range_violation(
        None,
        Vec::new(),
        qsl_replay::spine::default_accounting(0),
    ))
}

/// A violation replay is the only public source of a refuted terminal record;
/// the backend identity is carried through the envelope and its round trip.
#[trace("TC-773", "FR-289-AC-3")]
#[test]
fn refuted_record_requires_replay_and_backend_evidence() {
    let replay = violating_replay();
    let record = TerminalRecord::new(
        RequestIndex::new(7),
        BackendId::new("replay-backend"),
        &replay,
    )
    .unwrap();
    let source = BackendProviderSource {
        backend_identity: "replay-backend".to_owned(),
        items: vec![record],
    };
    let first =
        read_backend_provider_envelope(&source, qsl_replay::ReplayLimits::default()).unwrap();
    assert_eq!(first[0].record().value(), &TerminalValue::Refuted);
    assert_eq!(first[0].backend().as_str(), "replay-backend");
    assert_eq!(
        first[0].record().backend().unwrap().as_str(),
        "replay-backend"
    );
    assert_eq!(first[0].record().replay_result(), Some(&replay));

    let serialized = qsl_replay::ProofResultEnvelope::to_source(&first).unwrap();
    let second =
        read_backend_provider_envelope(&serialized, qsl_replay::ReplayLimits::default()).unwrap();
    assert_eq!(second[0].record().value(), &TerminalValue::Refuted);
    assert_eq!(second[0].backend().as_str(), "replay-backend");
    assert_eq!(second[0].record().replay_result(), Some(&replay));
}

/// FR-289-AC-3 / TC-773 step 3: a replay that did not reproduce a violation
/// cannot be relabelled as `refuted`.
#[trace("TC-773", "FR-289-AC-3")]
#[test]
fn non_violating_replay_is_refused() {
    let replay = ReplayResult::Witness(WitnessArmResult::settle(
        qsl_replay::Verdict::from_category(Category::Success),
        qsl_replay::Verdict::from_category(Category::Success),
        Category::Success,
        Some(EvaluatedValue::Boolean(true)),
        qsl_replay::WitnessCheck::Agrees(None),
        Vec::new(),
        qsl_replay::spine::default_accounting(0),
    ));

    assert_eq!(
        TerminalRecord::new(
            RequestIndex::new(9),
            BackendId::new("replay-backend"),
            &replay,
        ),
        Err(qsl_replay::TerminalRecordRefusal::ReplayDidNotRefute(
            Category::Success
        ))
    );
}

/// A violation category alone cannot turn an unsettled witness replay into
/// refutation, whether parity failed or the replay completed no value.
#[trace("TC-773", "FR-289-AC-3")]
#[test]
fn inconclusive_witness_replay_cannot_refute() {
    let violation = Verdict::from_category(Category::Violation);
    let success = Verdict::from_category(Category::Success);
    for (replayed, value) in [
        (success, Some(EvaluatedValue::Boolean(false))),
        (violation, None),
    ] {
        let replay = ReplayResult::Witness(WitnessArmResult::settle(
            violation,
            replayed,
            Category::Violation,
            value,
            WitnessCheck::Agrees(None),
            Vec::new(),
            qsl_replay::spine::default_accounting(0),
        ));
        assert_eq!(
            TerminalRecord::new(RequestIndex::new(0), BackendId::new("backend"), &replay),
            Err(qsl_replay::TerminalRecordRefusal::ReplayDidNotRefute(
                Category::Inconclusive
            ))
        );
    }
}

/// An input arm can report a violation, but it cannot supply the witness
/// settlement needed for a refuted backend record.
#[trace("TC-773", "FR-289-AC-3")]
#[test]
fn violating_input_arm_cannot_refute() {
    let violation = Verdict::from_category(Category::Violation);
    let replay = ReplayResult::Input(InputArmResult::settle(
        violation,
        violation,
        Category::Violation,
        Some(EvaluatedValue::Boolean(false)),
        &WitnessCheck::Agrees(None),
        Vec::new(),
        qsl_replay::spine::default_accounting(0),
    ));
    assert_eq!(
        TerminalRecord::new(RequestIndex::new(0), BackendId::new("backend"), &replay),
        Err(qsl_replay::TerminalRecordRefusal::ReplayLacksWitness)
    );
}

/// FR-289-AC-3 / TC-773: a provider envelope cannot claim a record
/// produced by a different backend.
#[trace("TC-773", "FR-289-AC-3")]
#[test]
fn record_backend_must_match_provider_identity() {
    let record = TerminalRecord::new(
        RequestIndex::new(4),
        BackendId::new("record-backend"),
        &violating_replay(),
    )
    .unwrap();
    let result = read_backend_provider_envelope(
        &BackendProviderSource {
            backend_identity: "manifest-backend".to_owned(),
            items: vec![record],
        },
        qsl_replay::ReplayLimits::default(),
    );
    assert!(matches!(
        result,
        Err(qsl_replay::ProofResultRefusal::BackendIdentityMismatch { .. })
    ));
}

/// TC-177 / FR-069-AC-1: public constructors for non-refuted values remain
/// available to an external facade client.
#[trace("TC-177", "FR-069-AC-1")]
#[test]
fn non_refuted_records_are_constructible_through_the_facade() {
    let backend = BackendId::new("provider-backend");
    let records = vec![
        TerminalRecord::declined(
            RequestIndex::new(0),
            backend.clone(),
            ProofRefusalCause::InvalidInput,
            DeclineCode::Qsl(qsl_replay::Code::MissingDeclaration),
        ),
        TerminalRecord::inconclusive(
            RequestIndex::new(1),
            backend.clone(),
            InconclusiveCause::ReplayRefused(qsl_replay::Code::StaleDependency),
        ),
        TerminalRecord::proved(RequestIndex::new(2), backend.clone(), 1),
        TerminalRecord::failed(RequestIndex::new(3), backend.clone()),
    ];
    let envelopes = read_backend_provider_envelope(
        &BackendProviderSource {
            backend_identity: backend.as_str().to_owned(),
            items: records,
        },
        qsl_replay::ReplayLimits::default(),
    )
    .unwrap();
    assert_eq!(envelopes[0].record().value().category(), Category::Refusal);
    assert_eq!(
        envelopes[1].record().value().category(),
        Category::Inconclusive
    );
    assert_eq!(
        envelopes[2].record().value(),
        &TerminalValue::Proved {
            basis: qsl_replay::ProofBasis::Checks { success_checks: 1 },
            certification: qsl_replay::Certification::Certified,
        }
    );
    assert_eq!(envelopes[3].record().value(), &TerminalValue::Failed);
}

/// FR-127-AC-2 / AC-9: the certificate rejection labels and typed query/proof-step loci
/// remain constructible and observable through the public facade.
#[trace("TC-522", "FR-127-AC-2", "FR-127-AC-9")]
#[test]
fn certificate_rejection_vocabulary_is_constructible_through_the_public_facade() {
    use qsl_replay::{CertificateLocus, CertificateRule, QueryPart};

    let backend = BackendId::new("certificate-provider");
    let values = [
        InconclusiveCause::CertificateRejected {
            rule: CertificateRule::QueryMismatch,
            at: CertificateLocus::Query {
                part: QueryPart::Unrolling,
            },
        },
        InconclusiveCause::CertificateRejected {
            rule: CertificateRule::ProofStepInvalid,
            at: CertificateLocus::ProofStep {
                part: QueryPart::Step,
                index: u64::MAX,
            },
        },
    ];
    let source = BackendProviderSource {
        backend_identity: backend.as_str().to_owned(),
        items: values
            .iter()
            .enumerate()
            .map(|(index, cause)| {
                TerminalRecord::inconclusive(
                    RequestIndex::new(index),
                    backend.clone(),
                    cause.clone(),
                )
            })
            .collect(),
    };
    let read =
        read_backend_provider_envelope(&source, qsl_replay::ReplayLimits::default()).unwrap();
    assert_eq!(read.len(), values.len());
    for (envelope, cause) in read.iter().zip(&values) {
        assert_eq!(envelope.category(), Category::Inconclusive);
        assert_eq!(
            envelope.record().value(),
            &TerminalValue::Inconclusive(cause.clone())
        );
        assert_eq!(
            envelope.inconclusive_cause(),
            Some(&ReportedInconclusiveCause::Cause(cause.clone()))
        );
    }
}

/// FR-121-AC-16: an `InternalFault` built through `qsl_replay::InternalFault`
/// is a `ReplayRefusal::Fault`, and `TerminalValue::from_replay_refusal`
/// settles it `Failed` (an internal failure), never `Inconclusive`.
#[trace("TC-516", "FR-121-AC-16")]
#[test]
fn a_replay_fault_built_through_the_facade_settles_failed() {
    let fault = InternalFault::new("terminal-map", "kind-total");
    assert_eq!(fault.category(), Category::InternalFailure);
    let settled = TerminalValue::from_replay_refusal(&ReplayRefusal::Fault(fault));
    assert_eq!(settled, TerminalValue::Failed);
    assert_eq!(settled.category(), Category::InternalFailure);
}

/// FR-121-AC-17: a `CallSiteRefusal::Fault` built through the facade settles
/// `Failed` by `TerminalValue::from_call_site_refusal`, never `Declined`.
#[trace("TC-516", "FR-121-AC-17")]
#[test]
fn a_call_site_fault_built_through_the_facade_settles_failed() {
    let fault = CallSiteRefusal::Fault(InternalFault::new("call-site", "function-node"));
    let settled = TerminalValue::from_call_site_refusal(&fault);
    assert_eq!(settled, TerminalValue::Failed);
    assert_eq!(settled.category(), Category::InternalFailure);
}

fn input_replay(replayed: Category, value: Option<EvaluatedValue>) -> ReplayResult {
    ReplayResult::Input(InputArmResult::settle(
        Verdict::from_category(Category::Violation),
        Verdict::from_category(replayed),
        replayed,
        value,
        &WitnessCheck::Agrees(None),
        Vec::new(),
        qsl_replay::spine::default_accounting(0),
    ))
}

/// FR-289-AC-4 to AC-7 / TC-773: a reproduced plugin counterexample settles
/// `refuted`, names the plugin and keeps the replay result; a
/// non-reproduction settles `replay-parity`; a refusal goes through
/// `from_replay_refusal`; a fault is `failed`.
#[trace("TC-773", "FR-289-AC-4", "FR-289-AC-5", "FR-289-AC-6", "FR-289-AC-7")]
#[test]
fn plugin_counterexample_settles_by_its_replay() {
    let plugin = || BackendId::new("acme-prover");
    let index = RequestIndex::new(3);

    let reproduced = Ok(input_replay(
        Category::Violation,
        Some(EvaluatedValue::Boolean(false)),
    ));
    let record = TerminalRecord::from_plugin_counterexample(index, plugin(), &reproduced);
    assert_eq!(record.value(), &TerminalValue::Refuted);
    assert_eq!(record.backend(), Some(&plugin()));
    assert_eq!(record.replay_result(), reproduced.as_ref().ok());

    for replayed in [
        input_replay(Category::Success, Some(EvaluatedValue::Boolean(true))),
        input_replay(Category::Violation, None),
    ] {
        let record = TerminalRecord::from_plugin_counterexample(index, plugin(), &Ok(replayed));
        assert!(matches!(
            record.value(),
            TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
        ));
        assert_eq!(record.value().category(), Category::Inconclusive);
        assert_eq!(record.replay_result(), None);
    }

    let refusal = ReplayRefusal::Fault(InternalFault::new("plugin-arm", "replay"));
    let record = TerminalRecord::from_plugin_counterexample(index, plugin(), &Err(refusal));
    assert_eq!(record.value(), &TerminalValue::Failed);

    let witness = Ok(violating_replay());
    let record = TerminalRecord::from_plugin_counterexample(index, plugin(), &witness);
    assert_eq!(record.value(), &TerminalValue::Failed);
}

/// FR-289-AC-6 / TC-773: a non-fault replay refusal settles
/// `inconclusive` `replay-refused`, like any counterexample's.
#[trace("TC-773", "FR-289-AC-6")]
#[test]
fn plugin_counterexample_refusal_matches_from_replay_refusal() {
    let refusal = || ReplayRefusal::Cancelled(qsl_replay::CancelCause::Requested);
    let record = TerminalRecord::from_plugin_counterexample(
        RequestIndex::new(0),
        BackendId::new("acme-prover"),
        &Err(refusal()),
    );
    assert_eq!(
        record.value(),
        &TerminalValue::from_replay_refusal(&refusal())
    );
}

/// FR-289-AC-9, AC-10 / TC-773: a plugin proof is `proved`, `trusted`, names the
/// plugin and reads back through the envelope; the Kani check-count basis is
/// refused.
#[trace("TC-773", "FR-289-AC-9", "FR-289-AC-10")]
#[test]
fn plugin_proof_is_trusted_and_names_the_plugin() {
    let record = TerminalRecord::from_plugin_proof(
        RequestIndex::new(1),
        BackendId::new("acme-prover"),
        qsl_replay::ProofBasis::Exhaustive,
    )
    .unwrap();
    assert_eq!(
        record.value(),
        &TerminalValue::Proved {
            basis: qsl_replay::ProofBasis::Exhaustive,
            certification: qsl_replay::Certification::Trusted,
        }
    );
    let envelopes = read_backend_provider_envelope(
        &BackendProviderSource {
            backend_identity: "acme-prover".to_owned(),
            items: vec![record],
        },
        qsl_replay::ReplayLimits::default(),
    )
    .unwrap();
    assert_eq!(envelopes[0].category(), Category::Success);
    assert_eq!(envelopes[0].backend().as_str(), "acme-prover");

    assert_eq!(
        TerminalRecord::from_plugin_proof(
            RequestIndex::new(1),
            BackendId::new("acme-prover"),
            qsl_replay::ProofBasis::Checks { success_checks: 0 },
        ),
        Err(qsl_replay::PluginResultRefusal::CoreOnlyBasis)
    );
    assert_eq!(
        TerminalRecord::from_plugin_proof(
            RequestIndex::new(1),
            BackendId::new("acme-prover"),
            qsl_replay::ProofBasis::ClosedForm {
                analysis: qsl_replay::ClosedFormAnalysis::FixedPriorityRta
            },
        ),
        Err(qsl_replay::PluginResultRefusal::CoreOnlyBasis)
    );
    assert_eq!(
        TerminalRecord::from_plugin_proof(
            RequestIndex::new(1),
            BackendId::new("acme-prover"),
            qsl_replay::ProofBasis::Witness {
                sources: Vec::new()
            },
        ),
        Err(qsl_replay::PluginResultRefusal::CoreOnlyBasis)
    );
}

/// FR-289-AC-11 / TC-773: a plugin result naming a QSL-only cause kind is
/// refused and its item settles `failed`; any other kind is not refused.
#[trace("TC-773", "FR-289-AC-11", "FR-289-AC-13")]
#[test]
fn plugin_result_claiming_a_qsl_only_cause_fails() {
    for (kind, cause) in [
        ("replay-parity", qsl_replay::QslOnlyCause::ReplayParity),
        ("replay-refused", qsl_replay::QslOnlyCause::ReplayRefused),
        (
            "kani_vacuous_proof",
            qsl_replay::QslOnlyCause::KaniVacuousProof,
        ),
    ] {
        let refusal = qsl_replay::read_plugin_cause_kind(kind).unwrap_err();
        assert_eq!(
            refusal,
            qsl_replay::PluginResultRefusal::QslOnlyCause(cause)
        );
        assert_eq!(cause.as_str(), kind);
        let record = refusal.record(RequestIndex::new(2), BackendId::new("acme-prover"));
        assert_eq!(record.value(), &TerminalValue::Failed);
        assert_eq!(record.value().category(), Category::InternalFailure);
    }
    for cause in [
        InconclusiveCause::BoundReached { depth: 1 },
        InconclusiveCause::InductionNotClosed { depth: 1 },
        InconclusiveCause::UndecidedSuccessor,
        InconclusiveCause::NoInitialState,
        InconclusiveCause::NoInstance,
        InconclusiveCause::MatchUndetermined,
        InconclusiveCause::VacuousMatch,
        InconclusiveCause::MappingUndetermined,
        InconclusiveCause::SufficientTestFailed,
        InconclusiveCause::SolverInconclusive,
    ] {
        assert_eq!(qsl_replay::read_plugin_cause_kind(cause.as_str()), Ok(()));
    }
    // The remaining `InconclusiveCause` kinds carry payloads built elsewhere;
    // their spellings are listed so every kind but the three is covered.
    // FR-331 spells the vacuity cause with an underscore on the plugin wire;
    // QSL's outcome-document spelling is no plugin cause and is not refused.
    assert_eq!(
        qsl_replay::read_plugin_cause_kind("kani-vacuous-proof"),
        Ok(())
    );
    for kind in [
        "certificate-rejected",
        "scalar-agrees",
        "well-definedness-unchecked",
        "reduction-not-preserving",
        "symmetry-broken",
        "constraint-reached",
        "reduction-horizon",
        "memory-bound-reached",
        "instance-bound-reached",
    ] {
        assert_eq!(qsl_replay::read_plugin_cause_kind(kind), Ok(()));
    }
}

/// FR-289-AC-12 / TC-773: the typed `inconclusive` constructor refuses the
/// QSL-only causes and keeps any other.
#[trace("TC-773", "FR-289-AC-12")]
#[test]
fn plugin_inconclusive_refuses_qsl_only_causes() {
    let plugin = || BackendId::new("acme-prover");
    let refused = TerminalRecord::from_plugin_inconclusive(
        RequestIndex::new(0),
        plugin(),
        InconclusiveCause::ReplayRefused(qsl_replay::Code::StaleDependency),
    );
    assert_eq!(
        refused,
        Err(qsl_replay::PluginResultRefusal::QslOnlyCause(
            qsl_replay::QslOnlyCause::ReplayRefused
        ))
    );
    let parity = TerminalRecord::from_plugin_inconclusive(
        RequestIndex::new(0),
        plugin(),
        InconclusiveCause::ReplayParity(qsl_replay::DisagreementCause::NoValue {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Violation),
        }),
    );
    assert_eq!(
        parity,
        Err(qsl_replay::PluginResultRefusal::QslOnlyCause(
            qsl_replay::QslOnlyCause::ReplayParity
        ))
    );
    let kept = TerminalRecord::from_plugin_inconclusive(
        RequestIndex::new(0),
        plugin(),
        InconclusiveCause::NoInitialState,
    )
    .unwrap();
    assert_eq!(
        kept.value(),
        &TerminalValue::Inconclusive(InconclusiveCause::NoInitialState)
    );
}

/// FR-289-AC-8 / TC-773: a replay that reproduced something other than a
/// violation refutes nothing and settles `failed` with no replay result.
#[trace("TC-773", "FR-289-AC-8")]
#[test]
fn reproduced_non_violation_settles_failed() {
    let success = Verdict::from_category(Category::Success);
    let violation = Verdict::from_category(Category::Violation);
    // Both verdicts agree on success; and verdicts agree on violation but the
    // arm's category is success (the category guard alone stops `refuted`).
    for (proved, replayed) in [(success, success), (violation, violation)] {
        let replay = ReplayResult::Input(InputArmResult::settle(
            proved,
            replayed,
            Category::Success,
            Some(EvaluatedValue::Boolean(true)),
            &WitnessCheck::Agrees(None),
            Vec::new(),
            qsl_replay::spine::default_accounting(0),
        ));
        let record = TerminalRecord::from_plugin_counterexample(
            RequestIndex::new(0),
            BackendId::new("acme-prover"),
            &Ok(replay),
        );
        assert_eq!(record.value(), &TerminalValue::Failed);
        assert_eq!(record.replay_result(), None);
    }
}

/// FR-289-AC-14 / TC-773: a refused plugin result carries the
/// `invalid_result` code with the cause PL-7 names, and the member it
/// offended.
#[trace("TC-773", "FR-289-AC-14")]
#[test]
fn refused_plugin_results_carry_pl7_diagnostics() {
    let unknown_cause = qsl_replay::read_plugin_cause_kind("replay-parity")
        .unwrap_err()
        .diagnostic(2, "replay-parity");
    assert_eq!(unknown_cause.code(), "invalid_result");
    assert_eq!(unknown_cause.cause(), Some("unknown-cause"));
    assert!(unknown_cause.message().contains("`cause`"));
    assert!(unknown_cause.message().contains("replay-parity"));
    assert!(unknown_cause.message().contains('2'));

    let unknown_basis =
        qsl_replay::PluginResultRefusal::CoreOnlyBasis.diagnostic(0, "zone-certified");
    assert_eq!(unknown_basis.code(), "invalid_result");
    assert_eq!(unknown_basis.cause(), Some("unknown-cause"));
    assert!(unknown_basis.message().contains("`basis`"));
    assert!(unknown_basis.message().contains("zone-certified"));

    let missing = qsl_replay::PluginResultRefusal::MissingCounterexample.diagnostic(1, "");
    assert_eq!(missing.code(), "invalid_result");
    assert_eq!(missing.cause(), Some("missing-member"));
    assert!(missing.message().contains("`counterexample`"));
    let record = qsl_replay::PluginResultRefusal::MissingCounterexample
        .record(RequestIndex::new(1), BackendId::new("acme-prover"));
    assert_eq!(record.value(), &TerminalValue::Failed);
}

/// FR-289-AC-9 / TC-773: every basis PL-7 lets a plugin carry, a
/// bounded-solver one included, settles `proved` with `trusted` and the
/// plugin's identity.
#[trace("TC-773", "FR-289-AC-9")]
#[test]
fn every_plugin_basis_is_trusted() {
    use qsl_replay::ProofBasis;
    let horizon = qsl_replay::SolverHorizon::new(quire_exact::Rational::from_integer(
        qsl_replay::Integer::from(5_i64),
    ))
    .unwrap();
    for basis in [
        ProofBasis::Exhaustive,
        ProofBasis::BoundedComplete { depth: 3 },
        ProofBasis::Inductive { depth: 2 },
        ProofBasis::Reduced {
            reductions: Vec::new(),
        },
        ProofBasis::BoundedSolver {
            method: qsl_replay::SolverMethod::DeltaComplete,
            horizon,
            jumps: 4,
        },
    ] {
        let record = TerminalRecord::from_plugin_proof(
            RequestIndex::new(0),
            BackendId::new("acme-prover"),
            basis.clone(),
        )
        .unwrap();
        assert_eq!(
            record.value(),
            &TerminalValue::Proved {
                basis,
                certification: qsl_replay::Certification::Trusted,
            }
        );
        assert_eq!(record.backend().map(BackendId::as_str), Some("acme-prover"));
    }
}
