// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-232-AC-1: the request writer's time-lock-freedom item, over the
//! `Counter` subject (`crate::counter`) made timed by a `time` member.

use ix_trace_rs::trace;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_route::deadlock::{ItemObligation, ModelTemporalItem, ObligationKind};
use qsl_route::request::{ExtentClassification, ItemKind, RequestItem};
use qsl_semantics::check::Capability;
use quire_exact::{Origin, Role};

use crate::counter::{document, package, timed_package, Documents, CONTRACT, PACKAGE};
use crate::derived_request::derived_items;

const TIMED: &str = "dimension Time;\nunit s : Time = rational(1, 1);\n\
     unit ms : Time = rational(1, 1000) * s;\ntime M dense unit ms;\n";

/// Every subject here reads the one model `M`'s selection.
const OVER_M: &[&str] = &[PACKAGE];

/// The derived items of `kind` among `derived`.
fn of_kind<S: Clone>(
    derived: &[(RequestItem, ItemObligation<S>)],
    kind: ItemKind,
) -> Vec<(RequestItem, ItemObligation<S>)> {
    derived
        .iter()
        .filter(|(item, _)| item.item_kind() == Some(kind))
        .cloned()
        .collect()
}

fn occurrence(byte: u8) -> OccurrenceKey {
    OccurrenceKey::new(
        crate::fixture_ids::node_key([byte]),
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
    let derived = derived_items(&both, &[], &[], OVER_M);
    let deadlock = of_kind(&derived, ItemKind::DeadlockFreedom);
    let time_lock = of_kind(&derived, ItemKind::TimeLockFreedom);
    assert_eq!(deadlock.len(), 1);
    assert_eq!(time_lock.len(), 1);
    let (item, obligation) = &time_lock[0];
    assert_eq!(obligation.subject, rpc);
    assert_eq!(item.kind(), Capability::TemporalSatisfaction);
    assert_eq!(
        item.extent(),
        ExtentClassification::Unbounded {
            finite_bound_available: false
        }
    );
    assert_eq!(obligation.kind, ObligationKind::TimeLockFreedom);
    assert_ne!(*obligation, deadlock[0].1);
    assert!(both.iter().all(|item| item.obligation() != *obligation));

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
    let time_lock = of_kind(
        &derived_items(&mixed, &[], &[], OVER_M),
        ItemKind::TimeLockFreedom,
    );
    assert_eq!(time_lock.len(), 1);
    assert_eq!(time_lock[0].1.subject, rpc);

    let terminal_any = [ModelTemporalItem {
        occurrence: &first,
        subject: &rpc,
        package: opted_out.graph(),
    }];
    let opted_out = derived_items(&terminal_any, &[], &[], OVER_M);
    assert!(of_kind(&opted_out, ItemKind::DeadlockFreedom).is_empty());
    assert_eq!(of_kind(&opted_out, ItemKind::TimeLockFreedom).len(), 1);

    // A subject reading another selection than the package's timed `M` is
    // not timed.
    let reading = |selection: &str| {
        qsl_route::deadlock::SubjectReading::read(timed.graph(), |identity| identity == selection)
    };
    assert!(reading(PACKAGE).timed());
    assert!(!reading("test/other").timed());
}
