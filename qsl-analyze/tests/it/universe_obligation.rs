// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-336-AC-4 (TC-846 step 4): a temporal item's obligation identity binds
//! the model subject's universes, and a `proved` result joins only the
//! request index of its own item. The `Counter` subject (`crate::counter`)
//! stands in for ADR-018 §6's `ConfigVersion`: both are one object type in
//! one unbounded population, checked under infinite-trace with
//! `fair weak each`.

use ix_trace_rs::trace;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::TerminalRecord;
use qsl_route::deadlock::{ItemObligation, ModelTemporalItem, ObligationKind};
use qsl_route::model_request::ModelRequest;
use qsl_semantics::check::TemporalTraceFamily;
use qsl_semantics::family::{ClaimExtent, FamilyContract};
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Identifier, Origin, Role};

use crate::counter::{clause, document, package, Documents, CONTRACT};

/// The `Counter` subject: its universe of `counters` and its initial
/// state. Which universe the subject holds is what the writer binds.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Subject {
    universe: Vec<&'static str>,
    initial: DocumentRef,
}

fn occurrence() -> OccurrenceKey {
    OccurrenceKey::new(
        WireNodeId::from_digest([1; 32]),
        Origin::new(Role::new("temporal"), 0),
    )
}

/// FR-336-AC-4: `ReachesTwo`-shaped liveness under `fair weak each` over
/// universe `{a, b}` and over `{a, b, c}` gives two obligation identities
/// that differ only in the subject's universe, with the same requirement
/// record and no proof bounds; each `proved` result joins its own request
/// index and no other.
#[trace("TC-846", "FR-336-AC-4")]
#[test]
fn two_universes_are_two_obligations_and_each_result_joins_its_own_index() {
    let document = document(false);
    let checked = package(&document, CONTRACT);
    let clause = clause(
        &document,
        "fair weak each M::Counter::inc;",
        "always eventually holds(c.value = 2)",
    );
    let population = checked
        .graph()
        .population_domain(
            &Identifier::new("M").expect("an identifier"),
            &Identifier::new("counters").expect("an identifier"),
        )
        .expect("keyed")
        .expect("counters resolves");
    let [claim] = TemporalTraceFamily::requirements(&clause)
        .try_into()
        .expect("one claim");
    let requirements = claim
        .requirements(
            WireNodeId::from_digest([1; 32]),
            &[WireNodeId::from_digest([2; 32])],
            Some(population),
        )
        .expect("keyed");
    assert!(matches!(requirements.extent(), ClaimExtent::Unbounded(_)));

    let mut documents = Documents::new(document);
    let initial = documents.start("c0", &[("a", 0), ("b", 0), ("c", 0)]);
    let over = |universe: Vec<&'static str>| Subject {
        universe,
        initial: initial.clone(),
    };
    let (two, three) = (over(vec!["a", "b"]), over(vec!["a", "b", "c"]));
    let occurrence = occurrence();
    let item = |subject| ModelTemporalItem {
        occurrence: &occurrence,
        subject,
        package: checked.graph(),
    };

    let mut request = ModelRequest::new();
    let at_two = request.authored(&item(&two), &requirements);
    let at_three = request.authored(&item(&three), &requirements);
    assert_eq!([at_two.get(), at_three.get()], [0, 1]);
    // Writing the same obligation again is the same item.
    assert_eq!(request.authored(&item(&two), &requirements), at_two);
    assert_eq!(request.items().len(), 2);

    let obligation = |subject: &Subject| ItemObligation {
        subject: subject.clone(),
        kind: ObligationKind::Authored(occurrence.clone()),
    };
    assert_ne!(obligation(&two), obligation(&three));
    // They differ only in the universe.
    assert_eq!(
        Subject {
            universe: two.universe.clone(),
            ..three.clone()
        },
        two
    );
    // A universe is no proof bound and no part of the requirement record.
    let [first, second] = request.items() else {
        panic!("two items");
    };
    assert_eq!(first.extent(), second.extent());
    assert_eq!(first.kind(), second.kind());
    assert!(first.domains().is_empty() && second.domains().is_empty());

    for (index, subject, other) in [(at_two, &two, &three), (at_three, &three, &two)] {
        let proved = TerminalRecord::proved(index, BackendId::new("qsl-en-1"), 1);
        let joined = request
            .obligation(proved.request_index())
            .expect("the result's index is an item of the request");
        assert_eq!(joined, &obligation(subject));
        assert_ne!(joined, &obligation(other));
        assert_eq!(request.index_of(&obligation(subject)), Some(index));
    }
    assert_eq!(request.obligation(RequestIndex::new(2)), None);
}
