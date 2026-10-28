// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-242's timed claim automaton on EN-6's liveness product: ADR-026
//! §11's `Settles` (TT-3) over `Rpc`, decided, certified, refuted and
//! replayed (TC-692, TC-694, TC-697).

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{ZoneCheckLimit, ZoneCheckLimits, ZoneCheckOutcome, ZoneStop};
use qsl_replay::{CertificateCheckLimits, ZoneCertificateVerdict};
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use quire_exact::{Cancel, CancelCause, Integer, Rational};

use crate::timed_world::{document_unbounded, World};
use crate::zone_replay::{replay, reproduced};
use crate::zone_search::{
    check, claim, delays, operations, proved, rpc_claim, rpc_world, violated, Outcome, SUBJECT,
    TIMED,
};

/// ADR-026 §11's `Settles` over `Rpc` with deadline `t`, its `eventually`
/// over `interval`: every `Waiting` position is followed within the
/// interval by a position that is not `Waiting`.
pub(crate) fn settles(t: u32, interval: &str) -> World {
    rpc_claim(t, TIMED, &settles_clause(interval))
}

/// `Settles`' clause text, its `eventually` over `interval`.
fn settles_clause(interval: &str) -> String {
    format!(
        "temporal Settles using tt over (c: M::Call) clock \"model-time\" on origin \
         {{ always (holds(c.phase = 1) implies eventually{interval} holds(not (c.phase = 1))) }}\n"
    )
}

fn decide(world: &World) -> Outcome {
    claim(world, ZoneCheckLimits::default(), &Cancel::new())
}

fn whole(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

/// FR-242's TT-3 behaviour: `Settles` is proved over `Rpc` with `T = 3 ms`
/// and with `T = 4 ms`, each certificate accepted by the core, which
/// rebuilds the claim automaton itself; under `eventually[0 ms, 3 ms)` it
/// is refuted (FR-237-AC-1's `Settles` half) by `send` at 0 then `reply`
/// after delay 3, the exact boundary, and that counterexample reproduces.
#[trace("TC-697", "FR-242")]
#[trace("TC-692", "FR-237-AC-1")]
#[test]
fn settles_is_proved_and_its_strict_variant_refuted() {
    for t in [3, 4] {
        let world = settles(t, "[0 ms, 3 ms]");
        let certificate = proved(decide(&world));
        assert!(certificate.components.is_some());
        assert_eq!(
            check(&world, &certificate, CertificateCheckLimits::default()),
            Ok(ZoneCertificateVerdict::Accepted),
            "T = {t}"
        );
    }
    let strict = settles(4, "[0 ms, 3 ms)");
    let counterexample = violated(decide(&strict));
    assert_eq!(operations(&counterexample)[..2], ["send", "reply"]);
    assert_eq!(delays(&counterexample)[..2], [whole(0), whole(3)]);
    let settlement = replay(&strict, &counterexample).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
}

/// Part of FR-239-AC-4 (TC-694): `Settles` over `Rpc` with
/// `max_symbolic_states` 2 stops naming the value 2; with a cancelled
/// handle it stops `Cancelled`; two runs give equal outcomes, certificates
/// and counterexamples. The `RequiresBound` half is
/// [`an_unbounded_root_requires_a_bound`].
#[trace("TC-694")]
#[test]
fn settles_stops_at_its_budgets_and_repeats() {
    let world = settles(4, "[0 ms, 3 ms]");
    let limits = ZoneCheckLimits {
        max_symbolic_states: 2,
        ..ZoneCheckLimits::default()
    };
    let ZoneCheckOutcome::Stopped { cause, .. } = claim(&world, limits, &Cancel::new()) else {
        panic!("a budget of 2 stops");
    };
    assert_eq!(
        cause,
        ZoneStop::LimitReached(ZoneCheckLimit::MaxSymbolicStates(2))
    );
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let ZoneCheckOutcome::Stopped { cause, .. } =
        claim(&world, ZoneCheckLimits::default(), &cancel)
    else {
        panic!("a cancelled run stops");
    };
    assert_eq!(cause, ZoneStop::Cancelled);
    assert_eq!(proved(decide(&world)), proved(decide(&world)));
    let strict = settles(4, "[0 ms, 3 ms)");
    assert_eq!(violated(decide(&strict)), violated(decide(&strict)));
}

/// FR-245 over a timed claim automaton: the checker rebuilds the automaton
/// from the clause it is asked about and trusts nothing of the
/// certificate's. `Settles`' proof at `T = 4 ms`, renamed to the strict
/// variant `eventually[0 ms, 3 ms)`, which `reply` at exactly 3 ms refutes,
/// is rejected.
#[trace("TC-700")]
#[test]
fn a_settles_certificate_is_checked_against_the_rebuilt_automaton() {
    let world = settles(4, "[0 ms, 3 ms]");
    let mut certificate = proved(decide(&world));
    let strict = settles(4, "[0 ms, 3 ms)");
    certificate.obligation = ItemObligation {
        subject: SUBJECT,
        kind: ObligationKind::Authored(qsl_replay::node_occurrence(
            &strict.package.graph().temporal_clauses()[0],
        )),
        reductions: qsl_route::reductions::ObligationReductions::default(),
    };
    let verdict =
        check(&strict, &certificate, CertificateCheckLimits::default()).expect("the check runs");
    assert!(
        matches!(verdict, ZoneCertificateVerdict::Rejected(_)),
        "{verdict:?}"
    );
}

/// Part of FR-239-AC-4 (TC-694): `Settles` over `Rpc` whose `send` also
/// takes `n: Integer`, an unbounded population root, refuses
/// `RequiresBound` (FR-126), naming the unbounded domain; the refusal is
/// read before any state is explored, when the timed subject is built. Two
/// runs refuse alike.
#[trace("TC-694")]
#[test]
fn an_unbounded_root_requires_a_bound() {
    let document = document_unbounded(
        &["x"],
        &[
            ("send", &["phase", "inflight", "late", "x"]),
            ("reply", &["phase", "inflight", "late"]),
            ("timeout", &["phase"]),
            ("reset", &["phase"]),
        ],
    );
    let world = rpc_world(&document, 4, TIMED, &settles_clause("[0 ms, 3 ms]"));
    // A timed subject is built only to be expanded, so the bound check
    // refuses it before EN-6 is asked.
    let refused = || {
        let system = world.system();
        qsl_eval::simulation::model::TimedSystem::new(&system, "M").err()
    };
    let Some(qsl_eval::simulation::model::TimedSubjectRefusal::NotSimulated(
        qsl_eval::simulation::NotSimulated::RequiresBound(roots),
    )) = refused()
    else {
        panic!("expected RequiresBound, got {:?}", refused());
    };
    assert!(!roots.domains.is_empty(), "{roots:?}");
    assert_eq!(refused(), refused());
}
