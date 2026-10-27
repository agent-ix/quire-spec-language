// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-430's dense-time interactions on EN-6 (TC-1170): strong fairness
//! read over time.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::ZoneCheckOutcome;
use qsl_replay::{CertificateCheckLimits, ComponentReason, ZoneCertificateVerdict};
use quire_exact::{Integer, Rational};

use crate::timed_world::{document, world, World, DENSE};
use crate::zone_liveness::decide;
use crate::zone_replay::{replay, reproduced};
use crate::zone_search::{check, delays, operations, proved, violated, INFINITE};

/// FR-430-AC-4's `Blink`: `tick`, guarded `x >= 1 ms`, resets `x` under
/// `x <= 1 ms`, so it fires exactly at `x = 1`; `fire`, guarded `x > 0 ms
/// and x < 1 ms`, sets `fired` (here `retried`). The claim `eventually
/// holds(b.fired)` under infinite-trace and `fairness`.
fn blink(fairness: &str) -> World {
    world(
        &document(&["x"], &[("tick", &["x"]), ("fire", &["retried"])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Tick using v on M::Call::tick {{ self.x >= 1 ms }}\n\
             post Ticked using v on M::Call::tick {{ self.x = 0 ms }}\n\
             pre Fire using v on M::Call::fire {{ self.x > 0 ms and self.x < 1 ms }}\n\
             post Fired using v on M::Call::fire {{ self.retried }}\n\
             time invariant Blinking when {{ true }} {{ self.x <= 1 ms }}\n\
             temporal Fires using inf over (b: M::Call) clock \"model-steps\" on origin \
             {{ {fairness} eventually holds(b.retried) }}\n"
        ),
    )
}

/// FR-430-AC-4 (TC-1170): over `Blink`, `eventually holds(b.fired)` under
/// `fair weak fire` is violated by a lasso of `tick` steps whose loop delay
/// is 1, since `fire` is never continuously enabled; under `fair strong
/// fire` it holds, with a certificate the core accepts, since `fire` is
/// enabled at instants inside every delay of that lasso though at no
/// position of it; the certificate states the strong reason with its
/// nested numbering (ADR-026 CF-2).
#[trace("TC-1170", "FR-430-AC-4")]
#[test]
fn strong_fairness_reads_enabledness_between_positions() {
    let weak = blink("fair weak M::Call::fire;");
    let counterexample = violated(decide(&weak));
    let lasso = counterexample.lasso.clone().expect("a lasso");
    let looped = &operations(&counterexample)[lasso.entry..];
    assert!(
        !looped.is_empty() && looped.iter().all(|step| step == "tick"),
        "{looped:?}"
    );
    let total = delays(&counterexample)[lasso.entry..]
        .iter()
        .fold(Rational::from_integer(Integer::zero()), |sum, delay| {
            sum.add(delay)
        });
    assert_eq!(total, Rational::from_integer(Integer::one()));
    let settlement = replay(&weak, &counterexample).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");

    let strong = blink("fair strong M::Call::fire;");
    let outcome = decide(&strong);
    assert!(
        matches!(outcome, ZoneCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );
    let certificate = proved(outcome);
    // ADR-026 CF-2: the strong reason with its nested numbering.
    let components = certificate.components.as_ref().expect("a component part");
    assert!(
        components
            .reasons
            .iter()
            .any(|reason| matches!(reason, ComponentReason::Strong { .. })),
        "{components:?}"
    );
    assert_eq!(
        check(&strong, &certificate, CertificateCheckLimits::default()),
        Ok(ZoneCertificateVerdict::Accepted)
    );
}
