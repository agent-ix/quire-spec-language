// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-243 (TC-698): every non-probabilistic timed item over a `time dense`
//! subject routes to EN-6 alone, with EN-1 and EN-6 both registered: the
//! timed clauses TT-1 to TT-4, an infinite-trace clause over the dense
//! subject, and the deadlock-freedom and time-lock-freedom items derived
//! from them. An item over an untimed subject keeps its candidates.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_analyze::model_check::manifest as en1;
use qsl_analyze::zone_check::manifest as en6;
use qsl_replay::node_occurrence;
use qsl_route::deadlock::{deadlock_freedom_items, time_lock_freedom_items, ModelTemporalItem};
use qsl_route::request::items_from_requirements;
use qsl_route::{CandidateOutcome, ManifestDigest, ProviderOrigin, Registry};
use qsl_semantics::check::{Capability, TimedPropertyForm};

use crate::timed_world::{document, world, World};
use crate::zone_search::{rpc_claim, INFINITE, TIMED};

/// EN-1 and EN-6, each registered from its own manifest.
pub(crate) fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register(
            en1::descriptor(ProviderOrigin::Linked, ManifestDigest::from_digest([1; 32]))
                .expect("EN-1 admits"),
        )
        .expect("registered");
    registry
        .register(
            en6::descriptor(ProviderOrigin::Linked, ManifestDigest::from_digest([6; 32]))
                .expect("EN-6 admits"),
        )
        .expect("registered");
    registry
}

/// The backend identities of a computed candidate set.
pub(crate) fn ids(outcome: &CandidateOutcome) -> Vec<String> {
    let CandidateOutcome::Candidates(set) = outcome else {
        panic!("a computed candidate set: {outcome:?}");
    };
    set.candidates()
        .iter()
        .map(|candidate| candidate.id().as_str().to_owned())
        .collect()
}

/// The candidates of `world`'s one temporal clause, whether its record is
/// marked dense, and the dense marks and candidates of the deadlock-freedom
/// and time-lock-freedom items derived from it.
fn routes(world: &World) -> (Vec<String>, bool, Vec<(bool, Vec<String>)>) {
    let registry = registry();
    let graph = world.package.graph();
    let [clause] = graph.temporal_clauses() else {
        panic!("one temporal clause");
    };
    let occurrence = node_occurrence(clause);
    let records = graph.requirements();
    let dense = records
        .get(&occurrence)
        .expect("the clause's record")
        .requirements()
        .dense_time();
    let items = items_from_requirements(records, &registry, None);
    let item = items
        .iter()
        .find(|item| *item.occurrence() == occurrence)
        .expect("the clause's item");
    let subject = "rpc";
    let temporal = [ModelTemporalItem {
        occurrence: &occurrence,
        subject: &subject,
        package: graph,
    }];
    let derived_route = |dense: bool| {
        ids(&registry.restrict_to_dense_time(
            registry.candidates_for_temporal_dynamics(
                Capability::TemporalSatisfaction,
                None,
                &BTreeSet::new(),
                &BTreeSet::new(),
                false,
            ),
            dense,
        ))
    };
    let derived = deadlock_freedom_items(&temporal, &[], &[])
        .iter()
        .map(|item| (item.dense_time(), derived_route(item.dense_time())))
        .chain(
            time_lock_freedom_items(&temporal, &[], |_| vec!["M".to_owned()])
                .iter()
                .map(|item| (item.dense_time(), derived_route(item.dense_time()))),
        )
        .collect();
    (ids(item.candidates()), dense, derived)
}

/// FR-243-AC-1 and AC-3 (TC-698): over the dense `Rpc`, `NoLateReply` under
/// the infinite-trace profile and each timed form TT-1 to TT-4 route to
/// EN-6 alone, and so do the deadlock-freedom and time-lock-freedom items
/// over the same subject.
#[trace("TC-698", "FR-243-AC-1")]
#[trace("TC-698", "FR-243-AC-3")]
#[test]
fn timed_items_over_a_dense_subject_route_to_en6_alone() {
    let clause = |profile: &str, body: &str| {
        format!(
            "temporal P using {profile} over (c: M::Call) clock \"{clock}\" on origin {{ {body} }}\n",
            clock = if profile == "tt" { "model-time" } else { "model-steps" },
        )
    };
    let infinite = rpc_claim(4, INFINITE, &clause("inf", "always holds(not c.late)"));
    let timed = [
        (
            "always holds(not c.late)",
            TimedPropertyForm::TimedInvariant,
        ),
        (
            "eventually[0 ms, 3 ms] holds(c.inflight)",
            TimedPropertyForm::BoundedWindow {
                horizon: quire_exact::Rational::from_integer(quire_exact::Integer::from(3u64)),
            },
        ),
        (
            "always (holds(c.inflight) implies eventually[0 ms, 3 ms] holds(not c.inflight))",
            TimedPropertyForm::TimedSafety,
        ),
        (
            "always eventually holds(c.inflight)",
            TimedPropertyForm::TimedLiveness,
        ),
    ];
    let mut worlds = vec![(None, infinite)];
    for (body, form) in timed {
        worlds.push((Some(form), rpc_claim(4, TIMED, &clause("tt", body))));
    }
    for (form, world) in &worlds {
        let [node] = world.package.graph().temporal_clauses() else {
            panic!("one temporal clause");
        };
        assert_eq!(node.clause().timed_form(), form.as_ref());
        let (candidates, dense, derived) = routes(world);
        assert!(dense, "{form:?}");
        assert_eq!(candidates, [en6::BACKEND_ID], "{form:?}");
        assert_eq!(
            derived,
            [
                (true, vec![en6::BACKEND_ID.to_owned()]),
                (true, vec![en6::BACKEND_ID.to_owned()])
            ],
            "{form:?}: the deadlock-freedom and time-lock-freedom items"
        );
    }
}

/// FR-243 / FR-430: an item over an untimed subject is not marked dense and
/// keeps both candidates; it derives a deadlock-freedom item that is not
/// dense, and no time-lock-freedom item.
#[trace("TC-698")]
#[test]
fn an_item_over_an_untimed_subject_keeps_its_candidates() {
    let untimed = world(
        &document(&[], &[("send", &["inflight"])]),
        INFINITE,
        "temporal P using inf over (c: M::Call) clock \"model-steps\" on origin \
         { always holds(not c.late) }\n",
    );
    let (candidates, dense, derived) = routes(&untimed);
    assert!(!dense);
    assert_eq!(candidates, [en1::BACKEND_ID, en6::BACKEND_ID]);
    assert_eq!(
        derived,
        [(
            false,
            vec![en1::BACKEND_ID.to_owned(), en6::BACKEND_ID.to_owned()]
        )]
    );
}
