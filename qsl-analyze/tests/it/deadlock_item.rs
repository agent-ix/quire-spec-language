// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124-AC-2: the request writer's deadlock-freedom item over the
//! `Counter` subject (`crate::counter`), under each terminal declaration.
//!
//! The subject is named here by its universe and its initial document, the
//! identity a request writer reads; the writer is generic over it.

use ix_trace_rs::trace;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_route::deadlock::{ItemObligation, ModelStateGraphItem, ModelTemporalItem, ObligationKind};
use qsl_route::request::{ExtentClassification, ItemKind};
use qsl_semantics::check::Capability;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Origin, Role};
use quire_semantic_value::fairness::PropertyForm;

use crate::counter::{document, package, Documents, CONTRACT, PACKAGE};
use crate::derived_request::derived_items;

/// The `Counter` subject: universe `{c}`, initial value 0.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Subject {
    universe: Vec<&'static str>,
    initial: DocumentRef,
}

/// Every subject here reads the one model `M`'s selection.
const OVER_M: &[&str] = &[PACKAGE];

fn occurrence(byte: u8) -> OccurrenceKey {
    OccurrenceKey::new(
        crate::fixture_ids::node_key([byte]),
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
    let derived = derived_items(&items, &[], &[], OVER_M);
    for (item, obligation) in &derived {
        assert_eq!(item.item_kind(), Some(ItemKind::DeadlockFreedom));
        assert_eq!(obligation.subject, subject);
        assert_eq!(item.property_form(), Some(PropertyForm::ReachableInvariant));
        assert_eq!(item.kind(), Capability::TemporalSatisfaction);
        assert_eq!(
            item.extent(),
            ExtentClassification::Unbounded {
                finite_bound_available: false
            }
        );
        assert_eq!(obligation.kind, ObligationKind::DeadlockFreedom);
    }
    (
        authored,
        derived
            .into_iter()
            .map(|(_, obligation)| obligation)
            .collect(),
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

/// FR-165-AC-3: a state-graph item contributes its model subject to the same
/// derived deadlock-freedom set as temporal items; a shared subject is
/// deduplicated, and `terminal any` still opts the subject out.
#[test]
#[trace("TC-590", "FR-165-AC-3")]
fn state_graph_items_share_the_subject_deadlock_item() {
    let document = document(false);
    let checked = package(&document, CONTRACT);
    let mut documents = Documents::new(document);
    let subject = Subject {
        universe: vec!["c"],
        initial: documents.start("c0", &[("c", 0)]),
    };
    let occurrence = occurrence(3);
    let temporal = [ModelTemporalItem {
        occurrence: &occurrence,
        subject: &subject,
        package: checked.graph(),
    }];
    let state_graph = [ModelStateGraphItem {
        subject: &subject,
        package: checked.graph(),
    }];

    let derived = derived_items(&temporal, &[], &state_graph, OVER_M);
    assert_eq!(derived.len(), 1, "equal subjects share one derived item");
    assert_eq!(derived[0].1.subject, subject);
    assert_eq!(derived[0].1.kind, ObligationKind::DeadlockFreedom);

    let graph_only_subject = Subject {
        universe: vec!["other"],
        initial: documents.start("c1", &[("other", 0)]),
    };
    let graph_only = [ModelStateGraphItem {
        subject: &graph_only_subject,
        package: checked.graph(),
    }];
    let graph_only_derived = derived_items(&[], &[], &graph_only, OVER_M);
    assert_eq!(
        graph_only_derived.len(),
        1,
        "graph-only subjects contribute"
    );
    assert_eq!(graph_only_derived[0].1.subject, graph_only_subject);

    let opted_out = package(
        &documents.document,
        &format!("{CONTRACT}terminal Halt using v on M any;\n"),
    );
    let graph_only = [ModelStateGraphItem {
        subject: &subject,
        package: opted_out.graph(),
    }];
    assert!(derived_items(&[], &[], &graph_only, OVER_M).is_empty());
}
