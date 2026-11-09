// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-281-AC-1 and AC-6 (TC-762, TC-763) for EN-5: an unbounded reachability
//! claim compiled from source runs through `analyze_items` on the EN-5
//! adapter, which builds the product, solves it exactly, has the core's
//! checker accept the certificate and only then settles `proved`.
//!
//! Not reachable: `refuted`. FR-203 settles it only after FR-202's replay of a
//! counterexample, which is not implemented, so a missed bound settles
//! `unsupported` (see the last test) and no test constructs a refutation.

use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, DispositionKind, Engines, Settlement,
};
use qsl_eval::simulation::Limits;
use qsl_foundation::RequestIndex;
use qsl_replay::{
    Certification, ExactOrBounds, Extremum, IncompleteCause, ModelCheckLimit, ModelCheckLimits,
    ProbabilityBoundSide, ProofBasis, TerminalValue,
};
use quire_exact::{Cancel, Integer, Rational};

use crate::prob_world::{self, World};

/// `Link` delivers within two attempts: the least delivery probability over
/// every scheduler is 24/25 (send_b twice), the greatest 99/100.
const CLAIMS: &str =
    "probabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin \
     { probability >= 0.95 [ eventually holds(m.delivered) ] }\n\
     probabilistic Unlikely using inf on M over (m: M::Msg) under every scheduler on origin \
     { probability >= 0.97 [ eventually holds(m.delivered) ] }\n";

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

fn settle(
    world: &World,
    claim: &str,
    limits: ModelCheckLimits,
) -> qsl_analyze::analyze::AnalyzeRecord {
    let engines = Engines::qsl().expect("the linked engines are admitted");
    let item = exact_probabilistic_item(RequestIndex::new(0), world.request(claim, limits));
    let mut records = analyze_items(&engines, vec![item], &Cancel::new());
    assert_eq!(records.len(), 1);
    records.remove(0)
}

#[trace("TC-762", "FR-281-AC-1")]
#[test]
fn a_reachability_bound_that_holds_settles_proved_with_the_checked_certificate() {
    let world = World::new(&prob_world::link(CLAIMS));
    let record = settle(&world, "Likely", ModelCheckLimits::default());
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(terminal.backend().map(|b| b.as_str()), Some("en-5"));
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, entries, .. },
        certification,
    } = terminal.value()
    else {
        panic!("expected proved, got {:?}", terminal.value());
    };
    assert_eq!(*certification, Certification::Certified);
    assert_eq!(value.value(), &ratio(24, 25));
    let [entry] = entries.as_slice() else {
        panic!("one entry");
    };
    assert_eq!(entry.extremum, Some(Extremum::Min));
    assert!(
        matches!(&entry.value, ExactOrBounds::Exact { value } if value.value() == &ratio(24, 25))
    );

    let evidence = record
        .exact_probabilistic
        .as_ref()
        .expect("the record carries the accepted certificate");
    assert_eq!(evidence.value, ratio(24, 25));
    assert_eq!(evidence.side, ProbabilityBoundSide::AtLeast);
    // Link has five reachable states: the start, delivered by either send,
    // one loss, two losses.
    assert!(evidence.certificate.values.len() >= 4);
    assert!(evidence
        .certificate
        .values
        .iter()
        .any(|entry| entry.value == ratio(24, 25)));
}

#[trace("TC-763", "FR-281-AC-6")]
#[test]
fn a_state_budget_of_one_settles_incomplete() {
    let world = World::new(&prob_world::link(CLAIMS));
    let limits = ModelCheckLimits {
        limits: Limits::default().with_max_states(1),
        ..ModelCheckLimits::default()
    };
    let record = settle(&world, "Likely", limits);
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(terminal.backend().map(|b| b.as_str()), Some("en-5"));
    assert_eq!(
        terminal.value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ExactProbMaxStates,
            value: 1,
        })
    );
    assert!(record.exact_probabilistic.is_none());
}

/// The exact value 24/25 misses `>= 0.97`. `refuted` needs FR-202's replay,
/// which does not exist, so the item is `unsupported` and never `proved`.
#[trace("TC-762", "FR-281-AC-1")]
#[test]
fn a_missed_bound_is_not_proved_and_not_yet_refuted() {
    let world = World::new(&prob_world::link(CLAIMS));
    let record = settle(&world, "Unlikely", ModelCheckLimits::default());
    assert!(record.exact_probabilistic.is_none());
    let Settlement::Disposed(disposed) = &record.settlement else {
        panic!("expected a disposition, got {:?}", record.settlement);
    };
    assert_eq!(disposed.kind, DispositionKind::Unsupported);
}
