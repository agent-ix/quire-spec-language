// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-155 steps 3 to 6 (FR-057-AC-6, FR-057-AC-8): registration admits
//! advertised labels under the requested-pair rules, candidate sets follow
//! the FR-290 rule under every registration order, and routing consumes
//! settled dispositions as data. Negotiation is quire-contract-codegen's
//! and is not run here: its settlements are fixture records shaped like
//! `negotiate_*` output (TC-155 Description).
//!
//! Steps 1 and 2 (FR-057-AC-5) exercise the composed linker, not `route`,
//! and are planned under #213.

use ix_trace_rs::trace;
use qsl_route::routing::{route, Disposition};
use qsl_route::{
    BackendDescriptor, BackendId, Candidate, CandidateOutcome, ManifestDigest, RegistrationCause,
    Registry, ToolIdentity,
};
use qsl_semantics::check::Capability;

fn candidate(id: &str, digest_byte: u8) -> Candidate {
    Candidate::new(
        BackendId::new(id),
        ManifestDigest::from_digest([digest_byte; 32]),
    )
}

fn admit(
    id: &str,
    digest_byte: u8,
    advertised: &[(Option<&str>, &str)],
) -> Result<BackendDescriptor, qsl_route::RegistrationRefusal> {
    BackendDescriptor::admit(
        candidate(id, digest_byte),
        ToolIdentity::new(format!("tool-for-{id}")),
        advertised.iter().map(|&(kind, mode)| (kind, Some(mode))),
    )
}

fn set_ids(outcome: CandidateOutcome) -> Vec<String> {
    let CandidateOutcome::Candidates(set) = outcome else {
        panic!("expected a computed candidate set, got {outcome:?}");
    };
    set.candidates()
        .iter()
        .map(|candidate| candidate.id().as_str().to_owned())
        .collect()
}

/// TC-155 step 3: an unknown kind, an absent kind and an unknown mode each
/// refuse with their `invalid_capability` cause, keyed by backend identity,
/// and the refused registration contributes nothing. A second, unequal
/// descriptor under an already-held identity conflicts (FR-290 "Candidate
/// set and negotiation"): both registrations are refused and the identity
/// is withdrawn (FR-290-AC-10).
#[test]
#[trace("TC-155", "FR-057-AC-8")]
fn malformed_and_repeated_registrations_refuse_keyed_by_identity() {
    let mut registry = Registry::new();
    let held = admit("held", 1, &[(Some("global-conformance"), "bounded")])
        .expect("an exact FR-290 label and mode admit");
    registry.register(held.clone()).unwrap();

    // `GlobalConformance` is the Rust variant name, not the FR-290 label.
    let unknown_kind = admit("unknown-kind", 2, &[(Some("GlobalConformance"), "bounded")])
        .expect_err("a Rust variant name is not an FR-290 label");
    assert_eq!(unknown_kind.identity().as_str(), "unknown-kind");
    assert_eq!(
        unknown_kind.cause(),
        &RegistrationCause::UnknownKind("GlobalConformance".to_owned())
    );
    assert_eq!(unknown_kind.catalog_code().code(), "invalid_capability");
    assert_eq!(unknown_kind.catalog_code().cause(), "unknown-kind");

    let absent_kind = admit(
        "absent-kind",
        3,
        &[(Some("value-validity"), "bounded"), (None, "bounded")],
    )
    .expect_err("an absent kind is never defaulted");
    assert_eq!(absent_kind.identity().as_str(), "absent-kind");
    assert_eq!(absent_kind.cause(), &RegistrationCause::AbsentKind);
    assert_eq!(absent_kind.catalog_code().cause(), "absent-kind");

    let unknown_mode = admit("unknown-mode", 4, &[(Some("value-validity"), "finite")])
        .expect_err("`finite` is not an FR-290 mode");
    assert_eq!(unknown_mode.identity().as_str(), "unknown-mode");
    assert_eq!(
        unknown_mode.cause(),
        &RegistrationCause::UnknownMode(Some("finite".to_owned()))
    );
    assert_eq!(unknown_mode.catalog_code().cause(), "unknown-mode");

    // A second registration under "held" with a different manifest digest
    // is a conflict (FR-290 "Candidate set and negotiation"): both
    // registrations are refused and the identity is withdrawn from the
    // registry entirely.
    let conflicting = admit("held", 5, &[(Some("value-validity"), "unbounded")])
        .expect("the labels themselves are admitted");
    let duplicates = registry
        .register(conflicting)
        .expect_err("a conflicting descriptor under a held identity refuses both");
    assert_eq!(duplicates.len(), 2);
    for duplicate in &duplicates {
        assert_eq!(duplicate.identity().as_str(), "held");
        assert_eq!(duplicate.cause(), &RegistrationCause::DuplicateBackend);
        assert_eq!(duplicate.catalog_code().cause(), "duplicate-backend");
    }

    // "held" is now unregistered: its earlier registration is withdrawn,
    // and it contributes no candidate for any kind.
    assert!(!registry
        .backends()
        .map(BackendId::as_str)
        .any(|id| id == "held"));
    assert!(registry.descriptors().all(|d| d.id().as_str() != "held"));
    assert!(set_ids(registry.candidates(Capability::ValueValidity, None)).is_empty());
    assert!(set_ids(registry.candidates(Capability::GlobalConformance, None)).is_empty());
    assert_eq!(
        registry.candidates(Capability::GlobalConformance, Some(&BackendId::new("held"))),
        CandidateOutcome::UnknownBackend(BackendId::new("held"))
    );
}

/// FR-057-AC-8: an absent mode refuses `unknown-mode` with no bytes, and a
/// pair whose kind and mode are both bad reports its kind (the documented
/// first-failure rule: kind before mode within one pair).
#[test]
#[trace("TC-155", "FR-057-AC-8")]
fn absent_mode_and_a_doubly_bad_pair_refuse_with_the_pinned_cause() {
    let absent_mode = BackendDescriptor::admit(
        candidate("absent-mode", 1),
        ToolIdentity::new("tool"),
        [(Some("value-validity"), None)],
    )
    .expect_err("an absent mode is never defaulted");
    assert_eq!(absent_mode.cause(), &RegistrationCause::UnknownMode(None));
    assert_eq!(absent_mode.catalog_code().cause(), "unknown-mode");

    let both_bad = admit("both-bad", 2, &[(Some("Refinement"), "finite")])
        .expect_err("neither label is admitted");
    assert_eq!(
        both_bad.cause(),
        &RegistrationCause::UnknownKind("Refinement".to_owned())
    );
}

/// FR-290 "Candidate set and negotiation": registration causes are reported
/// ordered bytewise by backend identity, then manifest digest. Two refusals
/// under one identity with different digests stay distinct and order by
/// digest; a smaller identity orders first whatever its digest.
#[test]
#[trace("TC-155", "FR-057-AC-8")]
fn refusals_order_by_identity_then_manifest_digest() {
    let later_digest = admit("same", 9, &[(None, "bounded")]).unwrap_err();
    let earlier_digest = admit("same", 1, &[(None, "bounded")]).unwrap_err();
    let other_identity = admit("earlier", 200, &[(None, "bounded")]).unwrap_err();
    assert_ne!(later_digest, earlier_digest);
    assert_eq!(later_digest.identity(), earlier_digest.identity());
    assert_eq!(earlier_digest.backend(), &candidate("same", 1));

    let mut refusals = vec![
        later_digest.clone(),
        earlier_digest.clone(),
        other_identity.clone(),
    ];
    refusals.sort();
    assert_eq!(refusals, [other_identity, earlier_digest, later_digest]);
}

/// The TC-155 step 4 registry: two backends advertising
/// (`operation-contract`, `bounded`) and one advertising only
/// `global-conformance`, registered in `order`.
fn step_4_registry(order: &[usize]) -> Registry {
    let pool = [
        admit("backend-b", 2, &[(Some("operation-contract"), "bounded")]).unwrap(),
        admit("backend-a", 1, &[(Some("operation-contract"), "bounded")]).unwrap(),
        admit("protocol", 3, &[(Some("global-conformance"), "bounded")]).unwrap(),
    ];
    let mut registry = Registry::new();
    for &index in order {
        registry.register(pool[index].clone()).unwrap();
    }
    registry
}

/// TC-155 steps 4 and 6: the unnamed request carries both capable backends,
/// ordered by identity then digest, and chooses neither; a named request
/// carries exactly that backend; a named backend lacking the kind gives an
/// empty set; an unregistered name gives the unknown-backend mark. The same
/// holds with the backends registered in reverse order.
#[test]
#[trace("TC-155", "FR-057-AC-8")]
fn candidate_sets_are_the_fr_290_rule_under_every_registration_order() {
    let kind = Capability::OperationContract;
    let forward = step_4_registry(&[0, 1, 2]);
    let reverse = step_4_registry(&[2, 1, 0]);
    assert_eq!(forward, reverse);

    for registry in [&forward, &reverse] {
        assert_eq!(
            set_ids(registry.candidates(kind, None)),
            ["backend-a", "backend-b"],
            "two capable backends are two candidates, never a chosen one"
        );
        assert_eq!(
            set_ids(registry.candidates(kind, Some(&BackendId::new("backend-b")))),
            ["backend-b"]
        );
        assert!(set_ids(registry.candidates(kind, Some(&BackendId::new("protocol")))).is_empty());
        assert_eq!(
            registry.candidates(kind, Some(&BackendId::new("absent"))),
            CandidateOutcome::UnknownBackend(BackendId::new("absent"))
        );
    }
}

/// TC-155 steps 5 and 6: given `global-conformance` settled `supported` and
/// `operation-contract` settled `unsupported` for its empty candidate set,
/// only the `supported` item is routed, to its one candidate. The
/// `unsupported` item gets no target and is not a refusal; `route` returns
/// every item's route at once, so nothing is pending after it returns. The
/// route is the same with the backends registered in reverse order.
#[test]
#[trace("TC-155", "FR-057-AC-6", "FR-057-AC-8")]
fn only_the_supported_item_is_routed() {
    let routes_for = |order: &[usize]| {
        let registry = step_4_registry(order);
        // Fixture settlement shaped like `negotiate_*` output: the protocol
        // item has exactly one candidate, so it can settle `supported`; the
        // operation-contract item named a backend that lacks the kind, so
        // its candidate set is empty and it settles `unsupported`.
        let CandidateOutcome::Candidates(protocol) =
            registry.candidates(Capability::GlobalConformance, None)
        else {
            panic!("expected a computed candidate set");
        };
        let [one] = protocol.candidates() else {
            panic!("expected exactly one candidate, got {protocol:?}");
        };
        let CandidateOutcome::Candidates(contract) = registry.candidates(
            Capability::OperationContract,
            Some(&BackendId::new("protocol")),
        ) else {
            panic!("expected a computed candidate set");
        };
        assert!(contract.is_empty());

        let dispositions = [
            Disposition::Supported(one.clone()),
            Disposition::Unsupported,
        ];
        route(&dispositions)
            .into_iter()
            .map(|target| target.cloned())
            .collect::<Vec<_>>()
    };

    let forward = routes_for(&[0, 1, 2]);
    assert_eq!(forward, [Some(candidate("protocol", 3)), None]);
    assert_eq!(routes_for(&[2, 1, 0]), forward);
}

/// FR-057 "Stage ownership": an item settled `requires-bound` or
/// `invalid-request` gets no target either, and a non-supported item
/// between two supported ones does not shift or delay their routes.
#[test]
#[trace("TC-155", "FR-057-AC-6")]
fn every_non_supported_disposition_gets_no_target_and_blocks_no_other_item() {
    let first = candidate("first", 1);
    let second = candidate("second", 2);
    let dispositions = [
        Disposition::Supported(first.clone()),
        Disposition::RequiresBound,
        Disposition::InvalidRequest,
        Disposition::Unsupported,
        Disposition::Supported(second.clone()),
    ];
    assert_eq!(
        route(&dispositions),
        [Some(&first), None, None, None, Some(&second)]
    );
}
