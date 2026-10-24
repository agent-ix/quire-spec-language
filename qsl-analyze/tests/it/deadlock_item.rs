// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124-AC-2: the request writer's deadlock-freedom item over the
//! `Counter` subject (`crate::counter`), under each terminal declaration.
//!
//! The subject is named here by its universe and its initial document, the
//! identity a request writer reads; the writer is generic over it.

use ix_trace_rs::trace;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_route::deadlock::{
    deadlock_freedom_items, ItemObligation, ModelTemporalItem, ObligationKind,
};
use qsl_route::request::ExtentClassification;
use qsl_semantics::check::Capability;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Origin, Role};
use quire_semantic_value::fairness::PropertyForm;

use crate::counter::{document, package, Documents, CONTRACT};

/// The `Counter` subject: universe `{c}`, initial value 0.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Subject {
    universe: Vec<&'static str>,
    initial: DocumentRef,
}

fn occurrence(byte: u8) -> OccurrenceKey {
    OccurrenceKey::new(
        WireNodeId::from_digest([byte; 32]),
        Origin::new(Role::new("temporal"), 0),
    )
}

/// The deadlock-freedom items of a request with two temporal items over
/// the one `Counter` subject whose package declares `terminal`.
fn derived(terminal: &str) -> (Vec<ItemObligation<Subject>>, Vec<ItemObligation<Subject>>) {
    let document = document(false);
    let checked = package(&document, &format!("{CONTRACT}{terminal}"));
    let mut documents = Documents::new(document);
    let subject = Subject {
        universe: vec!["c"],
        initial: documents.start("c0", &[("c", 0)]),
    };
    let (first, second) = (occurrence(1), occurrence(2));
    let items = [
        ModelTemporalItem {
            occurrence: &first,
            subject: &subject,
            package: checked.graph(),
        },
        ModelTemporalItem {
            occurrence: &second,
            subject: &subject,
            package: checked.graph(),
        },
    ];
    let authored = items.iter().map(ModelTemporalItem::obligation).collect();
    let derived = deadlock_freedom_items(&items);
    for item in &derived {
        assert_eq!(item.subject(), &subject);
        assert_eq!(item.property_form(), PropertyForm::ReachableInvariant);
        assert_eq!(item.kind(), Capability::TemporalSatisfaction);
        assert_eq!(
            item.extent(),
            ExtentClassification::Unbounded {
                finite_bound_available: false
            }
        );
        assert_eq!(item.obligation().kind, ObligationKind::DeadlockFreedom);
    }
    (
        authored,
        derived.iter().map(|item| item.obligation()).collect(),
    )
}

/// FR-124-AC-2: with the `None` or the `When` declaration two temporal
/// items over one subject give exactly one deadlock-freedom item, whose
/// obligation identity is neither authored item's; with `Any` they give
/// none.
#[test]
#[trace("TC-519", "FR-124-AC-2")]
fn one_deadlock_freedom_item_per_subject_unless_any_terminal_is_intended() {
    for terminal in ["", "terminal Done using v on M when { self.value = 3 };\n"] {
        let (authored, derived) = derived(terminal);
        assert_eq!(authored.len(), 2, "{terminal:?}");
        assert_ne!(authored[0], authored[1], "{terminal:?}");
        assert_eq!(derived.len(), 1, "{terminal:?}");
        assert!(
            authored.iter().all(|obligation| *obligation != derived[0]),
            "{terminal:?}: {derived:?} is an authored identity"
        );
    }
    let (authored, derived) = derived("terminal Halt using v on M any;\n");
    assert_eq!(authored.len(), 2);
    assert!(derived.is_empty(), "{derived:?}");
}
