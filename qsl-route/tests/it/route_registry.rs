// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-075/FR-076/FR-080: the `#185` registry's routing behaviour -- which
//! backend is chosen for a requested capability kind, and what is reported
//! when none is.
//!
//! Every test here drives [`qsl_route::Registry`] through its public API only:
//! build a registry, register backends, ask for a capability kind's
//! candidate set, and check which backend (if any) comes back. None of
//! these tests inspects where a type is defined or which module imports
//! which; TC-193 step 6 (FR-075-AC-5's "no local capability-kind enum"
//! check) is a source scan, run separately (see this crate's own
//! `#[cfg(test)]` module in `src/lib.rs`, which already imports only the
//! canonical `Capability` type -- there is no second type to scan for).

use ix_trace_rs::trace;
use qsl_foundation::digest::ByteDigest;
use qsl_route::{
    BackendDescriptor, BackendId, Candidate, CandidateOutcome, ManifestDigest, Mode,
    RegistrationCause, RegistrationRefusal, Registry, ToolIdentity,
};
use qsl_semantics::check::Capability;

fn digest(seed: &[u8]) -> ManifestDigest {
    ManifestDigest::from_digest(ByteDigest::of(seed).as_bytes())
}

fn backend(
    id: &str,
    seed: &[u8],
    advertises: impl IntoIterator<Item = (Capability, Mode)>,
) -> BackendDescriptor {
    BackendDescriptor::new(
        Candidate::new(BackendId::new(id), digest(seed)),
        ToolIdentity::new(format!("tool-for-{id}")),
        advertises,
    )
}

/// A candidate outcome's backend ids, in the order the registry returns
/// them. Panics on [`CandidateOutcome::UnknownBackend`]; callers that expect
/// that variant match on it directly instead.
fn set_ids(outcome: CandidateOutcome) -> Vec<String> {
    let CandidateOutcome::Candidates(set) = outcome else {
        panic!("expected a computed candidate set, got {outcome:?}");
    };
    set.candidates()
        .iter()
        .map(|candidate| candidate.id().as_str().to_owned())
        .collect()
}

/// TC-193 (FR-075-AC-1, FR-075-AC-5): the candidate set for an unnamed
/// request is exactly the registered backends that advertise the item's
/// kind, and mode never filters candidacy.
#[test]
#[trace("TC-193", "FR-075-AC-1")]
fn candidate_set_matches_registered_backends_advertising_the_requested_kind() {
    // Step 1-3: two backends, disjoint advertised kinds.
    let mut registry = Registry::new();
    registry
        .register(backend(
            "backend-a",
            b"a",
            [(Capability::ValueValidity, Mode::Bounded)],
        ))
        .unwrap();
    registry
        .register(backend(
            "backend-b",
            b"b",
            [(Capability::OperationContract, Mode::Bounded)],
        ))
        .unwrap();

    let CandidateOutcome::Candidates(value_validity) =
        registry.candidates(Capability::ValueValidity, None)
    else {
        panic!("expected a computed candidate set");
    };
    assert_eq!(
        value_validity
            .candidates()
            .iter()
            .map(|candidate| candidate.id().as_str())
            .collect::<Vec<_>>(),
        ["backend-a"]
    );

    let CandidateOutcome::Candidates(operation_contract) =
        registry.candidates(Capability::OperationContract, None)
    else {
        panic!("expected a computed candidate set");
    };
    assert_eq!(
        operation_contract
            .candidates()
            .iter()
            .map(|candidate| candidate.id().as_str())
            .collect::<Vec<_>>(),
        ["backend-b"]
    );

    // Step 4: a backend that does not advertise the requested kind at all.
    let mut absent = Registry::new();
    absent
        .register(backend(
            "backend-c",
            b"c",
            [(Capability::TemporalSatisfaction, Mode::Bounded)],
        ))
        .unwrap();
    let CandidateOutcome::Candidates(none) = absent.candidates(Capability::ValueValidity, None)
    else {
        panic!("expected a computed candidate set");
    };
    assert!(none.is_empty());

    // Step 5: mode never filters candidacy -- a bounded-only registrant is
    // still a candidate for the same kind regardless of the item's own
    // extent (extent comparison is `negotiate_*`'s job, not the registry's).
    let mut bounded_only = Registry::new();
    bounded_only
        .register(backend(
            "backend-d",
            b"d",
            [(Capability::FiniteReplay, Mode::Bounded)],
        ))
        .unwrap();
    let CandidateOutcome::Candidates(finite_replay) =
        bounded_only.candidates(Capability::FiniteReplay, None)
    else {
        panic!("expected a computed candidate set");
    };
    assert_eq!(
        finite_replay
            .candidates()
            .iter()
            .map(|candidate| candidate.id().as_str())
            .collect::<Vec<_>>(),
        ["backend-d"]
    );
}

/// TC-195 (FR-075-AC-3): a request naming an unregistered `BackendId`
/// yields a distinct unknown-backend marker, never a plain empty set.
#[test]
#[trace("TC-195", "FR-075-AC-3")]
fn unregistered_named_backend_yields_a_distinct_unknown_backend_marker() {
    let mut registry = Registry::new();
    registry
        .register(backend(
            "backend-a",
            b"a",
            [(Capability::ValueValidity, Mode::Bounded)],
        ))
        .unwrap();

    let unknown = registry.candidates(
        Capability::ValueValidity,
        Some(&BackendId::new("backend-z")),
    );
    assert_eq!(
        unknown,
        CandidateOutcome::UnknownBackend(BackendId::new("backend-z"))
    );

    let registered_but_unadvertised = registry.candidates(
        Capability::OperationContract,
        Some(&BackendId::new("backend-a")),
    );
    let CandidateOutcome::Candidates(set) = registered_but_unadvertised else {
        panic!("a registered backend that just doesn't advertise the kind is a plain empty set");
    };
    assert!(set.is_empty());
}

/// FR-075-AC-7 (quire-specification TC-282 DB-01, FR-290-AC-9): repeating
/// an identical registration -- same id, manifest digest, tool and
/// advertised pairs -- is one registration and is never refused.
#[test]
#[trace("TC-448", "FR-075-AC-7")]
fn identical_repeat_registration_is_not_refused() {
    let a1 = backend(
        "backend-a",
        b"a",
        [(Capability::ValueValidity, Mode::Bounded)],
    );
    let a2 = backend(
        "backend-a",
        b"a",
        [(Capability::ValueValidity, Mode::Bounded)],
    );
    let b = backend(
        "backend-b",
        b"b",
        [(Capability::OperationContract, Mode::Bounded)],
    );

    // The unrelated "backend-b" arrives before both "backend-a" repeats,
    // between them, and after both -- the result must not depend on where.
    for b_position in 0..3 {
        let mut registrations = vec![a1.clone(), a2.clone()];
        registrations.insert(b_position, b.clone());

        let mut registry = Registry::new();
        for (i, r) in registrations.into_iter().enumerate() {
            registry
                .register(r)
                .unwrap_or_else(|_| panic!("registration {i} must not be refused"));
        }

        let CandidateOutcome::Candidates(value_validity) =
            registry.candidates(Capability::ValueValidity, None)
        else {
            panic!("expected a computed candidate set; b_position={b_position}");
        };
        assert_eq!(
            value_validity
                .candidates()
                .iter()
                .map(|candidate| candidate.id().as_str())
                .collect::<Vec<_>>(),
            ["backend-a"],
            "b_position={b_position}"
        );
        assert_eq!(
            registry
                .backends()
                .map(BackendId::as_str)
                .collect::<Vec<_>>(),
            ["backend-a", "backend-b"],
            "b_position={b_position}"
        );
        assert_eq!(registry.refusals().count(), 0, "b_position={b_position}");
    }
}

/// FR-075-AC-4 (quire-specification TC-282 DB-03, FR-290-AC-10): two
/// unequal descriptors under one `BackendId` conflict. Every registration
/// of that identity is refused, the held registration is withdrawn, and
/// the identity becomes unregistered -- under either arrival order.
#[test]
#[trace("TC-196", "FR-075-AC-4")]
fn conflicting_backend_identity_registration_refuses_both_under_either_order() {
    for reversed in [false, true] {
        let mut registry = Registry::new();
        let a1 = backend(
            "backend-a",
            b"a",
            [(Capability::ValueValidity, Mode::Bounded)],
        );
        let a2 = backend(
            "backend-a",
            b"a2",
            [(Capability::OperationContract, Mode::Bounded)],
        );
        let (first, second) = if reversed { (a2, a1) } else { (a1, a2) };

        registry.register(first).unwrap();
        let refusals = registry
            .register(second)
            .expect_err("a conflicting descriptor under one identity refuses both");

        assert_eq!(refusals.len(), 2, "reversed={reversed}");
        for refusal in &refusals {
            assert_eq!(
                refusal.identity(),
                &BackendId::new("backend-a"),
                "reversed={reversed}"
            );
            assert_eq!(
                refusal.catalog_code().to_string(),
                "invalid_capability/duplicate-backend",
                "reversed={reversed}"
            );
        }

        // Neither advertised kind is reachable any more: the identity is
        // withdrawn entirely, not merely refused for the second arrival.
        let CandidateOutcome::Candidates(operation_contract) =
            registry.candidates(Capability::OperationContract, None)
        else {
            panic!("expected a computed candidate set");
        };
        assert!(operation_contract.is_empty(), "reversed={reversed}");
        let CandidateOutcome::Candidates(value_validity) =
            registry.candidates(Capability::ValueValidity, None)
        else {
            panic!("expected a computed candidate set");
        };
        assert!(value_validity.is_empty(), "reversed={reversed}");
        assert_eq!(
            registry.candidates(
                Capability::ValueValidity,
                Some(&BackendId::new("backend-a"))
            ),
            CandidateOutcome::UnknownBackend(BackendId::new("backend-a")),
            "reversed={reversed}"
        );
    }
}

/// TC-197 (FR-076-AC-1, FR-076-AC-2): when no registrant advertises the
/// requested kind, the registry's own returned output -- not data the
/// caller already held -- names the kind, and the named backend when the
/// request named one, with no panic and no refusal-shaped result.
#[test]
#[trace("TC-197", "FR-076-AC-1", "FR-076-AC-2")]
fn empty_candidate_set_carries_the_data_an_unsupported_warning_needs() {
    let mut registry = Registry::new();
    registry
        .register(backend(
            "backend-a",
            b"a",
            [(Capability::ValueValidity, Mode::Bounded)],
        ))
        .unwrap();

    let CandidateOutcome::Candidates(unnamed) =
        registry.candidates(Capability::TemporalSatisfaction, None)
    else {
        panic!("expected a computed candidate set, not an unknown-backend marker");
    };
    assert!(unnamed.is_empty());
    assert_eq!(unnamed.kind(), Capability::TemporalSatisfaction);
    assert_eq!(unnamed.requested_backend(), None);

    let CandidateOutcome::Candidates(named) = registry.candidates(
        Capability::TemporalSatisfaction,
        Some(&BackendId::new("backend-a")),
    ) else {
        panic!("expected a computed candidate set, not an unknown-backend marker");
    };
    assert!(named.is_empty());
    assert_eq!(named.kind(), Capability::TemporalSatisfaction);
    assert_eq!(
        named.requested_backend(),
        Some(&BackendId::new("backend-a"))
    );
}

/// TC-198 (FR-076-AC-3): the registry's output shapes for "no registrant
/// advertises this kind" and "this backend identity was already
/// registered" are structurally distinct, and no candidate-computation
/// return type carries a deferred/pending/hold variant.
#[test]
#[trace("TC-198", "FR-076-AC-3")]
fn backend_absence_never_settles_as_a_refusal_or_a_hold_at_the_registry() {
    let mut registry = Registry::new();
    registry
        .register(backend(
            "backend-a",
            b"a",
            [(Capability::ValueValidity, Mode::Bounded)],
        ))
        .unwrap();

    // Absence: an ordinary `CandidateOutcome::Candidates`, empty.
    let absence = registry.candidates(Capability::TemporalSatisfaction, None);
    let CandidateOutcome::Candidates(set) = &absence else {
        panic!("absence must be a computed candidate set, never a refusal-shaped variant");
    };
    assert!(set.is_empty());

    // Registration refusal: `register` returns
    // `Result<(), Vec<RegistrationRefusal>>`, a wholly different type from
    // `candidates`'s `CandidateOutcome` -- there is no shared enum whose
    // variants a caller could confuse, and no conversion exists between the
    // two. Each `RegistrationRefusal` itself names only the conflicted
    // identity; it carries no candidate-set data.
    let refusals = registry
        .register(backend(
            "backend-a",
            b"a2",
            [(Capability::OperationContract, Mode::Bounded)],
        ))
        .expect_err("conflicting registration is refused");
    for refusal in &refusals {
        assert_eq!(refusal.identity().as_str(), "backend-a");
    }

    // Neither `CandidateOutcome` variant is a deferred/pending/hold: both
    // are ordinary data, produced synchronously and carrying nothing that
    // waits on a future registration.
    assert!(matches!(absence, CandidateOutcome::Candidates(_)));
    let unknown = registry.candidates(Capability::ValueValidity, Some(&BackendId::new("ghost")));
    assert!(matches!(unknown, CandidateOutcome::UnknownBackend(_)));
    assert_ne!(
        absence, unknown,
        "absence and an unknown-backend marker are distinct outcomes"
    );
}

/// TC-207 (FR-080-AC-4): one independently-asserting unit test per ADR-012
/// §5.2 row, each checking that row's specific documented outcome (not
/// merely "did not panic").
mod adr_012_section_5_2_rows {
    use super::*;

    /// Row (a): a conflicting `BackendId` registration.
    #[test]
    #[trace("TC-207", "FR-080-AC-4")]
    fn row_a_duplicate_backend_id_is_refused() {
        let mut registry = Registry::new();
        registry
            .register(backend(
                "x",
                b"1",
                [(Capability::ValueValidity, Mode::Bounded)],
            ))
            .unwrap();
        let refusals = registry
            .register(backend(
                "x",
                b"2",
                [(Capability::OperationContract, Mode::Bounded)],
            ))
            .expect_err("a conflicting registration under the same identity must be refused");
        for refusal in &refusals {
            assert_eq!(refusal.identity().as_str(), "x");
        }
    }

    /// Row (b): a capability kind outside the vocabulary cannot reach the
    /// registry at all -- `Capability` is a closed, already-typed enum, so
    /// every one of its ten members is admitted uniformly by construction
    /// (ADR-012 §5.2: "this record adds nothing" for this row).
    #[test]
    #[trace("TC-207", "FR-080-AC-4")]
    fn row_b_every_canonical_capability_kind_is_admitted_uniformly() {
        let mut registry = Registry::new();
        let advertises: Vec<(Capability, Mode)> = Capability::ALL
            .iter()
            .map(|&kind| (kind, Mode::Bounded))
            .collect();
        registry
            .register(backend("x", b"1", advertises))
            .expect("every Capability::ALL member registers without a registry-level refusal");
        for kind in Capability::ALL {
            let CandidateOutcome::Candidates(set) = registry.candidates(*kind, None) else {
                panic!("expected a computed candidate set for {kind:?}");
            };
            assert_eq!(
                set.candidates().len(),
                1,
                "{kind:?} did not match its own registrant"
            );
        }
    }

    /// Row (c): an unregistered named `BackendId`.
    #[test]
    #[trace("TC-207", "FR-080-AC-4")]
    fn row_c_unregistered_named_backend_id_yields_the_unknown_marker() {
        let registry = Registry::new();
        let outcome =
            registry.candidates(Capability::ValueValidity, Some(&BackendId::new("ghost")));
        assert_eq!(
            outcome,
            CandidateOutcome::UnknownBackend(BackendId::new("ghost"))
        );
    }

    /// Row (d): a capability kind no registrant advertises.
    #[test]
    #[trace("TC-207", "FR-080-AC-4")]
    fn row_d_unadvertised_kind_yields_an_empty_candidate_set() {
        let mut registry = Registry::new();
        registry
            .register(backend(
                "x",
                b"1",
                [(Capability::ValueValidity, Mode::Bounded)],
            ))
            .unwrap();
        let CandidateOutcome::Candidates(set) = registry.candidates(Capability::Composition, None)
        else {
            panic!("expected a computed candidate set");
        };
        assert!(set.is_empty());
    }

    /// Row (e): more than one registrant matches and the request names
    /// none -- the registry reports every match (ambiguity resolution is
    /// `negotiate_*`'s job, not the registry's).
    #[test]
    #[trace("TC-207", "FR-080-AC-4")]
    fn row_e_more_than_one_match_with_no_named_backend_returns_every_candidate() {
        let mut registry = Registry::new();
        registry
            .register(backend(
                "x",
                b"1",
                [(Capability::ValueValidity, Mode::Bounded)],
            ))
            .unwrap();
        registry
            .register(backend(
                "y",
                b"2",
                [(Capability::ValueValidity, Mode::Unbounded)],
            ))
            .unwrap();
        let CandidateOutcome::Candidates(set) =
            registry.candidates(Capability::ValueValidity, None)
        else {
            panic!("expected a computed candidate set");
        };
        assert_eq!(set.candidates().len(), 2);
    }
}

/// Mirrors quire-specification TC-282's DB-01..DB-08 vectors (FR-290-AC-9,
/// FR-290-AC-10), each run under every ordering of its own registrations
/// (or, for DB-07 and DB-08, every relevant arrival order), checking that
/// the registry snapshot, its refusals and every named candidate set agree
/// with TC-282's expected result regardless of order.
mod tc_282_duplicate_backend_identity {
    use super::*;

    /// `A1`: identity `a`, one digest, advertising (`value-validity`,
    /// `bounded`).
    fn a1() -> BackendDescriptor {
        backend(
            "a",
            b"tc282-a-1",
            [(Capability::ValueValidity, Mode::Bounded)],
        )
    }

    /// `A2`: identity `a`, a second, distinct digest, advertising the same
    /// (`value-validity`, `bounded`) pair as `A1`.
    fn a2() -> BackendDescriptor {
        backend(
            "a",
            b"tc282-a-2",
            [(Capability::ValueValidity, Mode::Bounded)],
        )
    }

    /// `A1'`: identity `a`, `A1`'s own digest, advertising
    /// (`value-validity`, `unbounded`) instead.
    fn a1_prime() -> BackendDescriptor {
        backend(
            "a",
            b"tc282-a-1",
            [(Capability::ValueValidity, Mode::Unbounded)],
        )
    }

    /// `B`: identity `b`, advertising (`value-validity`, `bounded`).
    fn b_backend() -> BackendDescriptor {
        backend(
            "b",
            b"tc282-b",
            [(Capability::ValueValidity, Mode::Bounded)],
        )
    }

    /// `A1`'s and `A2`'s manifest digests, ordered so `.0 < .1` bytewise.
    /// TC-282 labels the smaller `d1` and the larger `d2`; which of `a1()`
    /// and `a2()`'s own hash-derived digests happens to come out smaller is
    /// not asserted anywhere else, so this orders them at run time rather
    /// than assuming a fixed seed produces the smaller hash.
    fn ordered_digests() -> (ManifestDigest, ManifestDigest) {
        let x = a1().candidate().manifest_digest();
        let y = a2().candidate().manifest_digest();
        if x < y {
            (x, y)
        } else {
            (y, x)
        }
    }

    /// Every ordering of `0..n` (n <= 5 here, so a plain recursive
    /// enumeration is clearer than pulling in a permutation crate).
    fn permutations(n: usize) -> Vec<Vec<usize>> {
        fn permute(remaining: Vec<usize>, acc: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
            if remaining.is_empty() {
                out.push(acc.clone());
                return;
            }
            for i in 0..remaining.len() {
                let mut rest = remaining.clone();
                let chosen = rest.remove(i);
                acc.push(chosen);
                permute(rest, acc, out);
                acc.pop();
            }
        }
        let mut out = Vec::new();
        permute((0..n).collect(), &mut Vec::new(), &mut out);
        out
    }

    /// Register `pool[order[i]]` for each `i` in turn, collecting each
    /// call's own `Err` (if any). TC-282's own vectors are registry-level;
    /// none of DB-01..DB-06 or DB-08's descriptors are refused at admission.
    fn build(
        order: &[usize],
        pool: &[BackendDescriptor],
    ) -> (Registry, Vec<Vec<RegistrationRefusal>>) {
        let mut registry = Registry::new();
        let mut errs = Vec::new();
        for &index in order {
            if let Err(refusals) = registry.register(pool[index].clone()) {
                errs.push(refusals);
            }
        }
        (registry, errs)
    }

    fn refusal_digests(registry: &Registry) -> Vec<ManifestDigest> {
        registry
            .refusals()
            .map(|refusal| refusal.backend().manifest_digest())
            .collect()
    }

    /// Every refusal in every `Err` this identity's registrations produced
    /// is well-formed: identity `"a"`, cause `DuplicateBackend`, a digest
    /// drawn from `valid_digests`, and -- within one `Err` -- refusals in
    /// ascending digest order with no repeated digest. Used where the
    /// number and grouping of failing calls legitimately varies with
    /// registration order (unlike DB-03/DB-04/DB-06/DB-08, which always
    /// produce exactly one `Err` whose content an equality check can pin
    /// down directly).
    fn assert_refusals_well_formed(
        errs: &[Vec<RegistrationRefusal>],
        valid_digests: &[ManifestDigest],
    ) {
        for err in errs {
            assert!(!err.is_empty(), "an Err must never be empty");
            let mut previous: Option<ManifestDigest> = None;
            for refusal in err {
                assert_eq!(refusal.identity().as_str(), "a");
                assert_eq!(refusal.cause(), &RegistrationCause::DuplicateBackend);
                let digest = refusal.backend().manifest_digest();
                assert!(
                    valid_digests.contains(&digest),
                    "refusal digest must be one of this identity's own digests"
                );
                if let Some(prev) = previous {
                    assert!(
                        prev < digest,
                        "refusals within one Err are ascending with no repeat"
                    );
                }
                previous = Some(digest);
            }
        }
    }

    /// DB-01: `A1`, `A1` -- `a` held once with `d1`; no refusal; the
    /// unnamed item's candidates are exactly (`a`, `d1`); the snapshot
    /// equals one registration of `A1`.
    #[test]
    #[trace("TC-447", "FR-075-AC-7")]
    fn db_01_identical_repeat_holds_once_with_no_refusal() {
        let pool = [a1(), a1()];
        let orderings = permutations(pool.len());
        let baseline_registry = build(&orderings[0], &pool).0;

        for order in &orderings {
            let (registry, errs) = build(order, &pool);
            assert!(errs.is_empty(), "order {order:?}");
            assert_eq!(registry, baseline_registry, "order {order:?}");
            assert_eq!(refusal_digests(&registry).len(), 0, "order {order:?}");
            assert_eq!(
                set_ids(registry.candidates(Capability::ValueValidity, None)),
                ["a"],
                "order {order:?}"
            );
            assert_eq!(
                set_ids(registry.candidates(Capability::ValueValidity, Some(&BackendId::new("a")))),
                ["a"],
                "order {order:?}"
            );
            assert_eq!(
                registry.candidates(Capability::ValueValidity, Some(&BackendId::new("b"))),
                CandidateOutcome::UnknownBackend(BackendId::new("b")),
                "order {order:?}: b was never registered in this vector"
            );
        }
    }

    /// DB-02: `A1`, `A1`, `A1`, `B` -- as DB-01, with `b` also held; the
    /// unnamed item's candidates are `a` then `b`, in candidate order.
    #[test]
    #[trace("TC-447", "FR-075-AC-7")]
    fn db_02_identical_repeats_plus_another_identity_hold_both() {
        let pool = [a1(), a1(), a1(), b_backend()];
        let orderings = permutations(pool.len());
        let baseline_registry = build(&orderings[0], &pool).0;

        for order in &orderings {
            let (registry, errs) = build(order, &pool);
            assert!(errs.is_empty(), "order {order:?}");
            assert_eq!(registry, baseline_registry, "order {order:?}");
            assert_eq!(
                registry
                    .backends()
                    .map(BackendId::as_str)
                    .collect::<Vec<_>>(),
                ["a", "b"],
                "order {order:?}"
            );
            assert_eq!(
                set_ids(registry.candidates(Capability::ValueValidity, None)),
                ["a", "b"],
                "order {order:?}"
            );
            assert_eq!(
                set_ids(registry.candidates(Capability::ValueValidity, Some(&BackendId::new("a")))),
                ["a"],
                "order {order:?}"
            );
            assert_eq!(
                set_ids(registry.candidates(Capability::ValueValidity, Some(&BackendId::new("b")))),
                ["b"],
                "order {order:?}"
            );
        }
    }

    /// DB-03: `A1`, `A2` -- two refusals, `invalid_capability`/
    /// `duplicate-backend` keyed (`a`, `d1`) then (`a`, `d2`); `a` is not
    /// held; the unnamed item's candidates are empty; the item naming `a`
    /// receives the unknown-backend mark carrying `a`.
    #[test]
    #[trace("TC-447", "FR-075-AC-4")]
    fn db_03_conflicting_digests_refuse_both_and_unregister_the_identity() {
        let (d1, d2) = ordered_digests();
        let pool = [a1(), a2()];
        let orderings = permutations(pool.len());
        let baseline_registry = build(&orderings[0], &pool).0;
        // Regardless of which of A1/A2 arrives first, the conflicting call's
        // own `Err` carries both digests, ascending -- order affects only
        // which call it is, never its content.
        let baseline_err = {
            let mut registry = Registry::new();
            registry.register(a1()).unwrap();
            registry.register(a2()).unwrap_err()
        };

        for order in &orderings {
            let (registry, errs) = build(order, &pool);
            assert_eq!(errs, vec![baseline_err.clone()], "order {order:?}");
            assert_eq!(registry, baseline_registry, "order {order:?}");
            assert_eq!(refusal_digests(&registry), vec![d1, d2], "order {order:?}");
            assert!(registry.backends().next().is_none(), "order {order:?}");
            assert!(
                set_ids(registry.candidates(Capability::ValueValidity, None)).is_empty(),
                "order {order:?}"
            );
            assert_eq!(
                registry.candidates(Capability::ValueValidity, Some(&BackendId::new("a"))),
                CandidateOutcome::UnknownBackend(BackendId::new("a")),
                "order {order:?}"
            );
        }
    }

    /// DB-04: `A1`, `A2`, `B` -- as DB-03 for `a`; `b` is held; the unnamed
    /// item's candidates are exactly `b`; the item naming `b` holds `b`.
    #[test]
    #[trace("TC-447", "FR-075-AC-4")]
    fn db_04_conflicting_identity_alongside_an_unrelated_identity() {
        let (d1, d2) = ordered_digests();
        let pool = [a1(), a2(), b_backend()];
        let orderings = permutations(pool.len());
        let baseline_registry = build(&orderings[0], &pool).0;
        let baseline_err = {
            let mut registry = Registry::new();
            registry.register(a1()).unwrap();
            registry.register(a2()).unwrap_err()
        };

        for order in &orderings {
            let (registry, errs) = build(order, &pool);
            assert_eq!(errs, vec![baseline_err.clone()], "order {order:?}");
            assert_eq!(registry, baseline_registry, "order {order:?}");
            assert_eq!(refusal_digests(&registry), vec![d1, d2], "order {order:?}");
            assert!(
                !registry
                    .backends()
                    .map(BackendId::as_str)
                    .any(|id| id == "a"),
                "order {order:?}"
            );
            assert_eq!(
                set_ids(registry.candidates(Capability::ValueValidity, None)),
                ["b"],
                "order {order:?}"
            );
            assert_eq!(
                set_ids(registry.candidates(Capability::ValueValidity, Some(&BackendId::new("b")))),
                ["b"],
                "order {order:?}: naming the still-held identity holds it"
            );
        }
    }

    /// DB-05: `A1`, `A2`, `A1` -- as DB-03; the second `A1` is refused with
    /// the first, and no third refusal key is reported.
    #[test]
    #[trace("TC-447", "FR-075-AC-4")]
    fn db_05_a_further_repeat_of_a_conflicted_identity_adds_no_new_key() {
        let (d1, d2) = ordered_digests();
        let pool = [a1(), a2(), a1()];
        let orderings = permutations(pool.len());
        let baseline_registry = build(&orderings[0], &pool).0;

        for order in &orderings {
            let (registry, errs) = build(order, &pool);
            // Which call discovers the conflict (and so how many calls fail
            // at all) depends on where the repeated `A1` falls relative to
            // `A2` -- that is legitimate order-dependent *shape*, not a
            // violation of order-independence, so each `Err`'s own content
            // is checked structurally rather than against one fixed
            // sequence.
            assert_refusals_well_formed(&errs, &[d1, d2]);
            assert_eq!(
                errs.iter().filter(|err| err.len() == 2).count(),
                1,
                "exactly one call discovers the two-digest conflict; order {order:?}"
            );
            assert_eq!(registry, baseline_registry, "order {order:?}");
            assert_eq!(refusal_digests(&registry), vec![d1, d2], "order {order:?}");
            assert!(registry.backends().next().is_none(), "order {order:?}");
            assert!(
                set_ids(registry.candidates(Capability::ValueValidity, None)).is_empty(),
                "order {order:?}"
            );
            assert_eq!(
                registry.candidates(Capability::ValueValidity, Some(&BackendId::new("a"))),
                CandidateOutcome::UnknownBackend(BackendId::new("a")),
                "order {order:?}"
            );
            assert_eq!(
                registry.candidates(Capability::ValueValidity, Some(&BackendId::new("b"))),
                CandidateOutcome::UnknownBackend(BackendId::new("b")),
                "order {order:?}: b was never registered in this vector"
            );
        }
    }

    /// DB-06: `A1`, `A1'` -- one refusal, `invalid_capability`/
    /// `duplicate-backend` keyed (`a`, `d1`); `a` is not held; the item
    /// naming `a` receives the unknown-backend mark.
    #[test]
    #[trace("TC-447", "FR-075-AC-4")]
    fn db_06_same_digest_different_contents_gives_one_refusal() {
        let d1 = a1().candidate().manifest_digest();
        let pool = [a1(), a1_prime()];
        let orderings = permutations(pool.len());
        let baseline_registry = build(&orderings[0], &pool).0;
        let baseline_err = {
            let mut registry = Registry::new();
            registry.register(a1()).unwrap();
            registry.register(a1_prime()).unwrap_err()
        };

        for order in &orderings {
            let (registry, errs) = build(order, &pool);
            assert_eq!(errs, vec![baseline_err.clone()], "order {order:?}");
            assert_eq!(registry, baseline_registry, "order {order:?}");
            assert_eq!(refusal_digests(&registry), vec![d1], "order {order:?}");
            assert!(registry.backends().next().is_none(), "order {order:?}");
            assert_eq!(
                registry.candidates(Capability::ValueValidity, Some(&BackendId::new("a"))),
                CandidateOutcome::UnknownBackend(BackendId::new("a")),
                "order {order:?}"
            );
        }
    }

    /// DB-07: `A1`, `M` -- `M` refuses as `invalid_capability`/
    /// `unknown-mode` keyed (`a`, `d2`); it is not admitted, so `a` is held
    /// once with `d1` and no `duplicate-backend` is reported. `M` never
    /// becomes a [`BackendDescriptor`] (it fails
    /// [`BackendDescriptor::admit`]), so it cannot appear in a `Registry`
    /// permutation pool at all; both relative arrival orders are exercised
    /// directly instead.
    #[test]
    #[trace("TC-447", "FR-057-AC-8")]
    fn db_07_a_malformed_registration_never_reaches_the_registry() {
        let (_d1, d2) = ordered_digests();
        for m_first in [false, true] {
            let mut registry = Registry::new();
            let attempt_m = || {
                BackendDescriptor::admit(
                    Candidate::new(BackendId::new("a"), d2),
                    ToolIdentity::new("tool-for-a"),
                    [(Some("value-validity"), Some("finite"))],
                )
                .expect_err("`finite` is not an FR-290 mode")
            };

            if m_first {
                let refusal = attempt_m();
                assert_eq!(refusal.identity().as_str(), "a", "m_first={m_first}");
                assert_eq!(
                    refusal.backend(),
                    &Candidate::new(BackendId::new("a"), d2),
                    "M's refusal is keyed (a, d2); m_first={m_first}"
                );
                assert_eq!(
                    refusal.cause(),
                    &RegistrationCause::UnknownMode(Some("finite".to_owned()))
                );
                registry.register(a1()).unwrap();
            } else {
                registry.register(a1()).unwrap();
                let refusal = attempt_m();
                assert_eq!(refusal.identity().as_str(), "a", "m_first={m_first}");
                assert_eq!(
                    refusal.backend(),
                    &Candidate::new(BackendId::new("a"), d2),
                    "M's refusal is keyed (a, d2); m_first={m_first}"
                );
                assert_eq!(
                    refusal.cause(),
                    &RegistrationCause::UnknownMode(Some("finite".to_owned()))
                );
            }

            assert_eq!(
                registry
                    .backends()
                    .map(BackendId::as_str)
                    .collect::<Vec<_>>(),
                ["a"],
                "m_first={m_first}"
            );
            assert_eq!(refusal_digests(&registry).len(), 0, "m_first={m_first}");
            assert_eq!(
                set_ids(registry.candidates(Capability::ValueValidity, None)),
                ["a"],
                "m_first={m_first}"
            );
        }
    }

    /// DB-08: register `A1` and read the snapshot; register `A2` and read
    /// the snapshot again; repeat with `A2` first -- the first snapshot
    /// holds `a` with the first registration's digest; the second equals
    /// DB-03's snapshot, with the first registration withdrawn and its
    /// refusal reported.
    #[test]
    #[trace("TC-447", "FR-075-AC-4")]
    fn db_08_the_conflict_appears_only_once_the_second_registration_arrives() {
        let (d1, d2) = ordered_digests();
        let baseline_err = {
            let mut registry = Registry::new();
            registry.register(a1()).unwrap();
            registry.register(a2()).unwrap_err()
        };
        let mut second_snapshots = Vec::new();

        for a2_first in [false, true] {
            let mut registry = Registry::new();
            let (first, second) = if a2_first { (a2(), a1()) } else { (a1(), a2()) };
            let first_digest = first.candidate().manifest_digest();

            registry.register(first).unwrap();
            assert_eq!(
                registry
                    .backends()
                    .map(BackendId::as_str)
                    .collect::<Vec<_>>(),
                ["a"],
                "a2_first={a2_first}"
            );
            let CandidateOutcome::Candidates(set) =
                registry.candidates(Capability::ValueValidity, None)
            else {
                panic!("expected a computed candidate set; a2_first={a2_first}");
            };
            assert_eq!(
                set.candidates()
                    .iter()
                    .map(Candidate::manifest_digest)
                    .collect::<Vec<_>>(),
                [first_digest],
                "a2_first={a2_first}"
            );
            assert_eq!(refusal_digests(&registry).len(), 0, "a2_first={a2_first}");

            let err = registry
                .register(second)
                .expect_err("the conflicting arrival refuses");
            assert_eq!(err, baseline_err, "a2_first={a2_first}");
            assert!(registry.backends().next().is_none(), "a2_first={a2_first}");
            assert_eq!(
                refusal_digests(&registry),
                vec![d1, d2],
                "a2_first={a2_first}"
            );
            assert!(
                set_ids(registry.candidates(Capability::ValueValidity, None)).is_empty(),
                "a2_first={a2_first}"
            );
            second_snapshots.push(registry);
        }

        // The second snapshot -- DB-03's snapshot -- is the same whichever
        // of A1/A2 arrived first.
        assert_eq!(second_snapshots[0], second_snapshots[1]);
    }

    /// FR-075-AC-4: a conflict is not special-cased to exactly two distinct
    /// manifest digests. Three admitted registrations of one identity, all
    /// with distinct digests, still refuse one distinct digest per
    /// registration, keyed `(identity, digest)` bytewise, under every order.
    #[test]
    #[trace("TC-447", "FR-075-AC-4")]
    fn three_way_conflict_reports_one_refusal_per_distinct_digest() {
        let a1 = a1();
        let a2 = a2();
        let a3 = backend(
            "a",
            b"tc282-a-3",
            [(Capability::ValueValidity, Mode::Bounded)],
        );
        let mut digests: Vec<ManifestDigest> = [&a1, &a2, &a3]
            .iter()
            .map(|d| d.candidate().manifest_digest())
            .collect();
        digests.sort();
        assert_eq!(
            digests.len(),
            3,
            "seeds must hash to three distinct digests"
        );

        let pool = [a1, a2, a3];
        let orderings = permutations(pool.len());
        assert_eq!(orderings.len(), 6, "3! = 6 orderings");
        let baseline_registry = build(&orderings[0], &pool).0;

        for order in &orderings {
            let (registry, errs) = build(order, &pool);
            assert_refusals_well_formed(&errs, &digests);
            assert_eq!(
                errs.iter()
                    .flat_map(|err| err.iter().map(|r| r.backend().manifest_digest()))
                    .collect::<std::collections::BTreeSet<_>>(),
                digests
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>(),
                "every digest is refused exactly once across the call sequence; order {order:?}"
            );
            assert_eq!(registry, baseline_registry, "order {order:?}");
            assert_eq!(refusal_digests(&registry), digests, "order {order:?}");
            assert!(registry.backends().next().is_none(), "order {order:?}");
            assert_eq!(
                registry.candidates(Capability::ValueValidity, Some(&BackendId::new("a"))),
                CandidateOutcome::UnknownBackend(BackendId::new("a")),
                "order {order:?}"
            );
        }
    }
}

/// TC-194 (FR-075-AC-2, FR-080-AC-1): registries built from any permutation
/// of the same `BackendDescriptor` set are equal and compute identical
/// candidate sets, in identical order, for every item -- independent of
/// registration order.
mod permutation_equality {
    use super::*;

    fn descriptors() -> Vec<BackendDescriptor> {
        vec![
            backend(
                "backend-a",
                b"a",
                [
                    (Capability::ValueValidity, Mode::Bounded),
                    (Capability::OperationContract, Mode::Bounded),
                ],
            ),
            backend(
                "backend-b",
                b"b",
                [(Capability::ValueValidity, Mode::Unbounded)],
            ),
            backend(
                "backend-c",
                b"c",
                [(Capability::FiniteReplay, Mode::Bounded)],
            ),
        ]
    }

    fn items() -> Vec<Capability> {
        vec![
            Capability::ValueValidity,        // more than one registrant
            Capability::OperationContract,    // exactly one registrant
            Capability::FiniteReplay,         // exactly one registrant
            Capability::TemporalSatisfaction, // no registrant
            Capability::Composition,          // no registrant
        ]
    }

    fn build(order: &[usize], pool: &[BackendDescriptor]) -> Registry {
        let mut registry = Registry::new();
        for &index in order {
            registry.register(pool[index].clone()).unwrap();
        }
        registry
    }

    /// Every permutation of `[0, 1, 2]`, generated in place (Heap's
    /// algorithm) -- 6 orderings for 3 descriptors. TC-194 asks for at
    /// least 20 sampled orderings; §`five_descriptor_full_enumeration`
    /// below supplies the rest (5! = 120, of which this test samples the
    /// first 24 by prefix-fixing two extra descriptors), giving 30 total
    /// exercised orderings across the two tests -- an explicit enumeration
    /// rather than a property-test-framework generator (TC-194 Test
    /// Procedure step 3 permits either).
    fn heap_permutations(n: usize) -> Vec<Vec<usize>> {
        let mut items: Vec<usize> = (0..n).collect();
        let mut c = vec![0; n];
        let mut result = vec![items.clone()];
        let mut i = 0;
        while i < n {
            if c[i] < i {
                if i % 2 == 0 {
                    items.swap(0, i);
                } else {
                    items.swap(c[i], i);
                }
                result.push(items.clone());
                c[i] += 1;
                i = 0;
            } else {
                c[i] = 0;
                i += 1;
            }
        }
        result
    }

    /// TC-194's permutation-invariance property, factored out of the two
    /// tests below so the same assertion can also run, deliberately, against
    /// the order-dependent `MutantRegistry` (`the_shared_permutation_
    /// assertion_actually_fails_against_the_mutant_registry` below) --
    /// proving this helper is a real, order-sensitive gate rather than two
    /// independently hand-written checks that merely resemble one (PR #305
    /// review, finding 9).
    ///
    /// Generic over "the registry type" via `build` (construct one from an
    /// ordering and the descriptor pool) and `candidates` (read one
    /// capability kind's candidate backend ids back out of it, in a form
    /// comparable across registry types): the real `Registry` and the
    /// mutant both plug in here unchanged.
    fn assert_permutation_invariant<Built>(
        orderings: &[Vec<usize>],
        pool: &[BackendDescriptor],
        items: &[Capability],
        build: impl Fn(&[usize], &[BackendDescriptor]) -> Built,
        candidates: impl Fn(&Built, Capability) -> Vec<String>,
    ) {
        let baseline = build(&orderings[0], pool);
        let baseline_candidates: Vec<Vec<String>> = items
            .iter()
            .map(|&kind| candidates(&baseline, kind))
            .collect();
        for order in orderings {
            let built = build(order, pool);
            let observed: Vec<Vec<String>> =
                items.iter().map(|&kind| candidates(&built, kind)).collect();
            assert_eq!(
                observed, baseline_candidates,
                "order {order:?} produced a different candidate-set sequence"
            );
        }
    }

    /// [`Registry::candidates`], reduced to its backend ids so it compares
    /// against the mutant's own reduced form in
    /// [`assert_permutation_invariant`]. An unnamed request (`None`) never
    /// yields `CandidateOutcome::UnknownBackend`.
    fn registry_candidate_ids(registry: &Registry, kind: Capability) -> Vec<String> {
        let CandidateOutcome::Candidates(set) = registry.candidates(kind, None) else {
            panic!("an unnamed request never yields UnknownBackend");
        };
        set.candidates()
            .iter()
            .map(|candidate| candidate.id().as_str().to_owned())
            .collect()
    }

    #[test]
    #[trace("TC-194", "FR-075-AC-2", "FR-080-AC-1")]
    fn every_permutation_of_three_descriptors_gives_an_equal_registry_and_identical_candidates() {
        let pool = descriptors();
        let items = items();
        let orderings = heap_permutations(pool.len());
        assert!(orderings.len() >= 6, "3! = 6 orderings");

        // Registry equality (not just candidate-sequence equality) also
        // holds regardless of order.
        let baseline_registry = build(&orderings[0], &pool);
        for order in &orderings {
            assert_eq!(
                build(order, &pool),
                baseline_registry,
                "registry built from order {order:?} is not equal to the baseline"
            );
        }

        assert_permutation_invariant(&orderings, &pool, &items, build, registry_candidate_ids);
    }

    /// Five descriptors give 5! = 120 orderings; this test runs every one of
    /// them, exceeding TC-194's "at least 20" bar with a larger descriptor
    /// pool than the three-descriptor test above.
    #[test]
    #[trace("TC-194", "FR-075-AC-2", "FR-080-AC-1")]
    fn all_120_orderings_of_five_descriptors_agree() {
        let pool = vec![
            backend("a", b"1", [(Capability::ValueValidity, Mode::Bounded)]),
            backend("b", b"2", [(Capability::ValueValidity, Mode::Bounded)]),
            backend("c", b"3", [(Capability::OperationContract, Mode::Bounded)]),
            backend("d", b"4", [(Capability::FiniteReplay, Mode::Unbounded)]),
            backend("e", b"5", [(Capability::Composition, Mode::Bounded)]),
        ];
        let items = items();
        let orderings = heap_permutations(pool.len());
        assert_eq!(orderings.len(), 120, "5! = 120 orderings");

        let baseline_registry = build(&orderings[0], &pool);
        for order in &orderings {
            assert_eq!(build(order, &pool), baseline_registry);
        }

        assert_permutation_invariant(&orderings, &pool, &items, build, registry_candidate_ids);
    }

    /// Like [`build`], but tolerates a conflicting registration's `Err`
    /// instead of unwrapping it -- the multiset the next test builds
    /// deliberately includes one, so a plain `.unwrap()` would panic on
    /// whichever ordering registers the conflicting pair.
    fn build_allowing_conflicts(order: &[usize], pool: &[BackendDescriptor]) -> Registry {
        let mut registry = Registry::new();
        for &index in order {
            let _ = registry.register(pool[index].clone());
        }
        registry
    }

    /// FR-075-AC-2, extended to FR-075-AC-4/AC-7 (quire-specification
    /// FR-290-AC-9/AC-10): a descriptor multiset that mixes an identical
    /// repeat of one identity with a conflicting pair under another
    /// identity still gives the same registry, the same refusal list (in
    /// the same order) and the same candidate sets under every registration
    /// order.
    #[test]
    #[trace("TC-194", "FR-075-AC-2", "FR-075-AC-4", "FR-075-AC-7")]
    fn permutation_invariance_holds_with_repeats_and_conflicts_in_the_multiset() {
        let repeat = backend(
            "backend-a",
            b"a",
            [(Capability::ValueValidity, Mode::Bounded)],
        );
        let conflict_first = backend(
            "backend-c",
            b"c1",
            [(Capability::FiniteReplay, Mode::Bounded)],
        );
        let conflict_second = backend(
            "backend-c",
            b"c2",
            [(Capability::FiniteReplay, Mode::Unbounded)],
        );
        let pool = vec![
            repeat.clone(),
            repeat,
            conflict_first,
            conflict_second,
            backend(
                "backend-b",
                b"b",
                [(Capability::ValueValidity, Mode::Unbounded)],
            ),
        ];
        let items = items();
        let orderings = heap_permutations(pool.len());
        assert_eq!(orderings.len(), 120, "5! = 120 orderings");

        let baseline_registry = build_allowing_conflicts(&orderings[0], &pool);
        let baseline_refusals: Vec<_> = baseline_registry.refusals().collect();
        assert_eq!(
            baseline_refusals.len(),
            2,
            "backend-c's two descriptors carry distinct digests (DB-03 shape), so the \
             conflict is one refusal per digest"
        );
        for order in &orderings {
            let registry = build_allowing_conflicts(order, &pool);
            assert_eq!(registry, baseline_registry, "order {order:?}");
            let refusals: Vec<_> = registry.refusals().collect();
            assert_eq!(refusals, baseline_refusals, "order {order:?}");
        }

        assert_permutation_invariant(
            &orderings,
            &pool,
            &items,
            build_allowing_conflicts,
            registry_candidate_ids,
        );
    }

    /// A mutant registry that stores registrations in a `Vec` and returns
    /// candidates in that `Vec`'s own insertion order, never sorting by
    /// `(identity, manifest digest)`. This demonstrates the property-test
    /// methodology above is actually sensitive to order-dependent candidate
    /// computation, not vacuously passing (TC-194 Test Procedure step 6).
    struct MutantRegistry(Vec<BackendDescriptor>);

    impl MutantRegistry {
        fn build(order: &[usize], pool: &[BackendDescriptor]) -> Self {
            Self(order.iter().map(|&index| pool[index].clone()).collect())
        }

        /// Unsorted, unlike `Registry::candidates` -- the defect under test.
        /// `advertises` is private to `route`, so this determines "does
        /// `descriptor` advertise `kind`" the same way any external caller
        /// would have to: build a one-backend real `Registry` and ask it.
        fn candidates_unsorted(&self, kind: Capability) -> Vec<String> {
            self.0
                .iter()
                .filter(|descriptor| {
                    let mut solo = Registry::new();
                    solo.register((*descriptor).clone()).unwrap();
                    matches!(
                        solo.candidates(kind, None),
                        CandidateOutcome::Candidates(set) if !set.is_empty()
                    )
                })
                .map(|descriptor| descriptor.id().as_str().to_owned())
                .collect()
        }
    }

    #[test]
    #[trace("TC-194")]
    fn a_mutant_insertion_order_registry_fails_the_permutation_property() {
        // Two backends both advertising the same kind: insertion order is
        // observable in the mutant's unsorted output, but not in the real
        // `Registry`'s sorted output.
        let pool = vec![
            backend(
                "z-backend",
                b"z",
                [(Capability::ValueValidity, Mode::Bounded)],
            ),
            backend(
                "a-backend",
                b"a",
                [(Capability::ValueValidity, Mode::Bounded)],
            ),
        ];
        let forward =
            MutantRegistry::build(&[0, 1], &pool).candidates_unsorted(Capability::ValueValidity);
        let reversed =
            MutantRegistry::build(&[1, 0], &pool).candidates_unsorted(Capability::ValueValidity);
        assert_ne!(
            forward, reversed,
            "the mutant's own insertion-order output must actually differ between orderings, \
             or this test cannot demonstrate the property test's sensitivity"
        );

        // The real registry, by contrast, agrees regardless of order.
        let real_forward = build(&[0, 1], &pool);
        let real_reversed = build(&[1, 0], &pool);
        assert_eq!(
            real_forward.candidates(Capability::ValueValidity, None),
            real_reversed.candidates(Capability::ValueValidity, None),
            "the real Registry must not reproduce the mutant's order-dependence"
        );
    }

    /// PR #305 review, finding 9: the *shared* [`assert_permutation_invariant`]
    /// helper the two passing tests above call -- not a separately
    /// hand-written comparison that merely resembles it -- actually fails
    /// when applied to [`MutantRegistry`]. This is the helper's own mutation
    /// evidence: it demonstrates the factored assertion is a real,
    /// order-sensitive gate, not just descriptive of what those two tests
    /// already checked by hand.
    #[test]
    #[trace("TC-194")]
    fn the_shared_permutation_assertion_actually_fails_against_the_mutant_registry() {
        let pool = vec![
            backend(
                "z-backend",
                b"z",
                [(Capability::ValueValidity, Mode::Bounded)],
            ),
            backend(
                "a-backend",
                b"a",
                [(Capability::ValueValidity, Mode::Bounded)],
            ),
        ];
        let orderings = vec![vec![0, 1], vec![1, 0]];
        let items = vec![Capability::ValueValidity];

        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_permutation_invariant(
                &orderings,
                &pool,
                &items,
                MutantRegistry::build,
                MutantRegistry::candidates_unsorted,
            );
        }));
        assert!(
            outcome.is_err(),
            "assert_permutation_invariant must fail (panic) against the order-dependent \
             MutantRegistry, or it is not actually sensitive to the defect it exists to catch"
        );
    }
}

/// TC-205 step 4 (FR-080-AC-2): `deny.toml` actually refuses a crate graph
/// that depends on one of the three registry-discovery crates it names
/// (`inventory`, `linkme`, `ctor`), not merely that the file's text mentions
/// them.
///
/// This module previously inspected `deny.toml`'s own text instead of
/// running it: `bans_table_text` cut the `[bans]` table's body at the first
/// `[` character, which is the opening bracket of `deny = [`'s own array,
/// not a later `[section]` header -- so the extracted "table" never
/// contained the crate names at all, and both of its tests failed
/// (PR #305 review finding 2). A text scan cannot demonstrate FR-080-AC-2's
/// actual claim (`cargo deny check bans` denies these crates) even when
/// correctly written: it does not exercise `cargo-deny` at all. This module
/// runs `cargo-deny` itself instead.
///
/// A first version of this trigger test (PR #305 review round 2, finding 1)
/// asserted only a non-zero exit plus the banned crate's name appearing
/// somewhere in the output. Both hold for a *resolution* failure too --
/// an unresolvable version requirement, an offline index, or a yanked
/// release all exit non-zero and name the crate in cargo's own "failed to
/// select a version" message, without cargo-deny's ban check ever running.
/// This version instead asserts on cargo-deny's own `error[banned]` / "is
/// explicitly banned" diagnostic, which only the ban check emits, and covers
/// all three denied crates in one throwaway graph so removing any one of
/// them from `deny.toml` is caught.
mod cargo_deny_bans_the_three_registry_crates {
    use std::path::Path;
    use std::process::Command;

    use ix_trace_rs::trace;

    /// The three registry-discovery crates `deny.toml` denies (FR-080-AC-2).
    const DENIED_CRATES: [&str; 3] = ["ctor", "inventory", "linkme"];

    /// FR-080-AC-2's actual behaviour: `cargo deny check bans` refuses a
    /// throwaway crate graph that depends on all three registry-discovery
    /// crates `deny.toml` denies, emitting cargo-deny's own `error[banned]`
    /// diagnostic for each one -- not merely exiting non-zero with the
    /// crate's name somewhere in the output, which a dependency-resolution
    /// failure would also produce without the ban check ever running.
    ///
    /// Skips, rather than fails, when the `cargo-deny` binary is not on
    /// `PATH`: this crate does not depend on `cargo-deny` being installed to
    /// build or test locally (`make cargo-deny-bans`'s own header documents
    /// installing it), while `.github/workflows/ci.yml` installs a pinned
    /// version unconditionally, so the hosted gate always exercises this
    /// test for real. `make cargo-deny-bans` itself fails outright when the
    /// binary is absent, so `make ci` cannot go green by relying on this
    /// skip.
    #[test]
    #[trace("TC-205", "FR-080-AC-2")]
    fn cargo_deny_refuses_a_graph_that_depends_on_a_denied_crate() {
        if Command::new("cargo-deny")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!(
                "skipping cargo_deny_refuses_a_graph_that_depends_on_a_denied_crate: \
                 `cargo-deny` binary not found on PATH; see `make cargo-deny-bans`'s own \
                 comment in the Makefile for how to install it"
            );
            return;
        }

        let scratch =
            tempfile::tempdir().expect("create a scratch directory for the throwaway manifest");
        let deps: String = DENIED_CRATES
            .iter()
            .map(|name| format!("{name} = \"*\"\n"))
            .collect();
        std::fs::write(
            scratch.path().join("Cargo.toml"),
            format!(
                "[package]\n\
                 name = \"cargo-deny-trigger-check\"\n\
                 version = \"0.0.0\"\n\
                 edition = \"2021\"\n\
                 publish = false\n\
                 \n\
                 [dependencies]\n\
                 {deps}"
            ),
        )
        .expect("write the throwaway manifest");
        std::fs::create_dir_all(scratch.path().join("src")).expect("create src/");
        std::fs::write(scratch.path().join("src/lib.rs"), "").expect("write an empty lib.rs");

        // The workspace's own `deny.toml`, one level above this crate.
        let deny_toml = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("qsl-route lives one level below the workspace root")
            .join("deny.toml");
        let output = Command::new("cargo-deny")
            .arg("--manifest-path")
            .arg(scratch.path().join("Cargo.toml"))
            .arg("check")
            .arg("--config")
            .arg(&deny_toml)
            .arg("bans")
            .output()
            .expect("run cargo-deny against the throwaway manifest");

        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.status.success(),
            "cargo-deny must refuse a graph depending on the three denied registry-discovery \
             crates; output:\n{combined}"
        );
        assert!(
            !combined.contains("failed to select a version"),
            "this run hit dependency resolution, not the ban check -- it proves nothing about \
             deny.toml; output:\n{combined}"
        );
        for name in DENIED_CRATES {
            let ban_line_present = combined.lines().any(|line| {
                line.contains("error[banned]")
                    && line.contains(&format!("'{name} ="))
                    && line.contains("is explicitly banned")
            });
            assert!(
                ban_line_present,
                "cargo-deny must emit its own `error[banned]: crate '{name} = ...' is \
                 explicitly banned` diagnostic for `{name}`, not merely mention its name; \
                 got:\n{combined}"
            );
        }
    }
}
