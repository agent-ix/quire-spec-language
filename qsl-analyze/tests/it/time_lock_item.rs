// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-232-AC-1: the request writer's time-lock-freedom item, over the
//! `Counter` subject (`crate::counter`) made timed by a `time` member.

use ix_trace_rs::trace;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_route::deadlock::{
    deadlock_freedom_items, time_lock_freedom_items, ModelTemporalItem, ObligationKind,
};
use qsl_route::request::ExtentClassification;
use qsl_semantics::check::Capability;
use quire_exact::{Origin, Role};

use crate::counter::{document, package, timed_package, Documents, CONTRACT};

const TIMED: &str = "dimension Time;\nunit s : Time = rational(1, 1);\n\
     unit ms : Time = rational(1, 1000) * s;\ntime M dense unit ms;\n";

/// Every subject here ranges over the one model `M`.
fn over_m<S>(_: &S) -> Vec<String> {
    vec!["M".to_owned()]
}

fn occurrence(byte: u8) -> OccurrenceKey {
    OccurrenceKey::new(
        WireNodeId::from_digest([byte; 32]),
        Origin::new(Role::new("temporal"), 0),
    )
}

/// FR-232-AC-1: two claims over one timed subject carry one deadlock-freedom
/// and one time-lock-freedom item, with distinct obligation identities; one
/// claim over a timed and one over an untimed subject carry one
/// time-lock-freedom item, for the timed one; `terminal any` removes the
/// deadlock-freedom item and not the time-lock-freedom item.
#[test]
#[trace("TC-687", "FR-232-AC-1")]
fn one_time_lock_freedom_item_per_timed_subject() {
    let bytes = document(false);
    let timed = timed_package(&bytes, &format!("{CONTRACT}{TIMED}"));
    let untimed = package(&bytes, CONTRACT);
    let opted_out = timed_package(
        &bytes,
        &format!("{CONTRACT}{TIMED}terminal Halt using v on M any;\n"),
    );
    let mut documents = Documents::new(bytes);
    let rpc = documents.start("c0", &[("c", 0)]);
    let counter = documents.start("c1", &[("d", 0)]);
    let (first, second) = (occurrence(1), occurrence(2));

    let both = [
        ModelTemporalItem {
            occurrence: &first,
            subject: &rpc,
            package: timed.graph(),
        },
        ModelTemporalItem {
            occurrence: &second,
            subject: &rpc,
            package: timed.graph(),
        },
    ];
    let deadlock = deadlock_freedom_items(&both, &[]);
    let time_lock = time_lock_freedom_items(&both, &[], over_m);
    assert_eq!(deadlock.len(), 1);
    assert_eq!(time_lock.len(), 1);
    assert_eq!(time_lock[0].subject(), &rpc);
    assert_eq!(time_lock[0].kind(), Capability::TemporalSatisfaction);
    assert_eq!(
        time_lock[0].extent(),
        ExtentClassification::Unbounded {
            finite_bound_available: false
        }
    );
    assert_eq!(
        time_lock[0].obligation().kind,
        ObligationKind::TimeLockFreedom
    );
    assert_ne!(time_lock[0].obligation(), deadlock[0].obligation());
    assert!(both
        .iter()
        .all(|item| item.obligation() != time_lock[0].obligation()));

    let mixed = [
        ModelTemporalItem {
            occurrence: &first,
            subject: &rpc,
            package: timed.graph(),
        },
        ModelTemporalItem {
            occurrence: &second,
            subject: &counter,
            package: untimed.graph(),
        },
    ];
    let time_lock = time_lock_freedom_items(&mixed, &[], over_m);
    assert_eq!(time_lock.len(), 1);
    assert_eq!(time_lock[0].subject(), &rpc);

    let terminal_any = [ModelTemporalItem {
        occurrence: &first,
        subject: &rpc,
        package: opted_out.graph(),
    }];
    assert!(deadlock_freedom_items(&terminal_any, &[]).is_empty());
    assert_eq!(time_lock_freedom_items(&terminal_any, &[], over_m).len(), 1);

    // A subject over another model than the package's timed `M` is not
    // timed.
    assert!(time_lock_freedom_items(&both, &[], |_| vec!["N".to_owned()]).is_empty());
}
