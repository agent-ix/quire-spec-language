// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-846 step 4: the EN-1 request path's `ModelRequest` seam, driven from a
//! real `ModelSubject` over the `Counter` package (`crate::liveness`'s
//! fixture) and its checked `fair weak each` clause.
//!
//! The request is built by `qsl_analyze::model_check::request::author_clause`,
//! which reads the clause's requirement record from the subject's own checked
//! package and identifies the subject by its `SubjectIdentity`. Two requests
//! for one clause over subjects that differ only in a universe get two
//! request indices, and a `proved` result joins only the obligation at its own
//! index.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{author_clause, RequestRefusal, SubjectIdentity};
use qsl_eval::model_subject::ModelSubject;
use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBound, ProofBound};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::{node_occurrence, TerminalRecord};
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use qsl_route::model_request::ModelRequest;
use qsl_route::request::ExtentClassification;

use crate::counter::CONTRACT;
use crate::counter::{document, key, subject, UNLIMITED};
use crate::liveness::{counter, fixture, Fixture};

fn reaches_two() -> Fixture {
    counter(
        document(false),
        "fair weak each M::Counter::inc;",
        "always eventually holds(c.value = 2)",
    )
}

fn fixture_with(body: &str) -> Fixture {
    fixture(
        document(false),
        body,
        "fair weak each M::Counter::inc;",
        "always eventually holds(c.value = 2)",
        &[("c", 0)],
    )
}

fn over<'a>(fixture: &'a Fixture, keys: &[&str]) -> ModelSubject<'a> {
    subject(
        &fixture.package,
        &fixture.documents,
        fixture.initial.clone(),
        keys,
        UNLIMITED,
    )
}

fn identity(subject: &ModelSubject<'_>) -> SubjectIdentity {
    SubjectIdentity::of(subject).expect("the package emits")
}

/// `ReachesTwo` under `fair weak each` over universe `{a, b}` and over `{a,
/// b, c}` gives two obligations that differ only in the subject's universe,
/// with the same requirement record (the clause's own, unbounded) and no
/// proof bounds; each `proved` result joins its own request index and no
/// other.
#[trace("TC-846", "FR-336-AC-4")]
#[test]
fn two_universes_are_two_obligations_and_each_result_joins_its_own_index() {
    let fixture = reaches_two();
    let [clause] = fixture.package.graph().temporal_clauses() else {
        panic!("one declared clause");
    };
    let (two, three) = (
        over(&fixture, &["a", "b"]),
        over(&fixture, &["a", "b", "c"]),
    );

    let mut request = ModelRequest::new();
    let at_two = author_clause(&mut request, &two, clause).expect("the clause has a record");
    let at_three = author_clause(&mut request, &three, clause).expect("the clause has a record");
    assert_eq!([at_two.get(), at_three.get()], [0, 1]);
    // Writing the same obligation again is the same item, whatever order the
    // universe lists its keys in.
    let again = over(&fixture, &["b", "a"]);
    assert_eq!(
        author_clause(&mut request, &again, clause).expect("the clause has a record"),
        at_two
    );
    assert_eq!(request.items().len(), 2);

    let occurrence = node_occurrence(clause);
    let obligation = |subject: &ModelSubject<'_>| ItemObligation {
        subject: SubjectIdentity::of(subject).expect("the package emits"),
        kind: ObligationKind::Authored(occurrence.clone()),
    };
    assert_ne!(obligation(&two), obligation(&three));
    // They differ only in the universe: with the same keys the identities
    // are equal, and the identity names the universe it holds.
    assert_eq!(obligation(&two), obligation(&again));
    let population = key("counters");
    let keys = |identity: &SubjectIdentity| {
        identity
            .universe(&population)
            .map(|keys| keys.iter().cloned().collect::<Vec<_>>())
    };
    assert_eq!(
        keys(&identity(&three)),
        Some(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()])
    );

    // A universe is no proof bound and no part of the requirement record.
    let [first, second] = request.items() else {
        panic!("two items");
    };
    assert_eq!(first.extent(), second.extent());
    assert_eq!(first.kind(), second.kind());
    assert!(matches!(
        first.extent(),
        ExtentClassification::Unbounded { .. }
    ));
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

/// FR-336's identity projection: a proof bound or another initial state is a
/// different subject, and a clause the subject's package never declared is
/// refused rather than written.
#[trace("TC-846", "FR-336-AC-5")]
#[test]
fn a_proof_bound_or_an_initial_state_changes_the_identity_and_a_foreign_clause_is_refused() {
    let mut fixture = reaches_two();
    let plain = identity(&over(&fixture, &["a", "b"]));

    let mut bounded = over(&fixture, &["a", "b"]);
    bounded.proof_bounds.push(
        ProofBound::new(
            DomainKey::Node {
                node: WireNodeId::from_digest([9; 32]),
                path: Vec::new(),
            },
            Some(DomainKind::Population),
            FiniteBound::Cardinality { maximum: 3 },
        )
        .expect("a population takes a cardinality bound"),
    );
    assert_ne!(identity(&bounded), plain);

    // The same clause, domain package and initial document over a checked
    // package whose `post Inc` contract differs is another subject: its
    // successor relation differs, though the clause's own nodes do not.
    let rival = fixture_with(&CONTRACT.replace("= 1 }", "= 2 }"));
    let rival_subject = over(&rival, &["a", "b"]);
    let [rival_clause] = rival.package.graph().temporal_clauses() else {
        panic!("one declared clause");
    };
    let [clause] = fixture.package.graph().temporal_clauses() else {
        panic!("one declared clause");
    };
    assert_eq!(node_occurrence(rival_clause), node_occurrence(clause));
    assert_ne!(identity(&rival_subject), plain);
    let mut shared = ModelRequest::new();
    let first = author_clause(&mut shared, &over(&fixture, &["a", "b"]), clause).expect("record");
    let second = author_clause(&mut shared, &rival_subject, rival_clause).expect("record");
    assert_ne!(first, second);

    // A provision carrying a package the model does not select is the same
    // subject.
    let mut provision = fixture.documents.packages.clone();
    provision.insert([7; 32], b"unselected".to_vec());
    let mut padded = over(&fixture, &["a", "b"]);
    padded.simulation.packages = &provision;
    assert_eq!(identity(&padded), plain);

    let second = fixture.documents.start("s1", &[("c", 1)]);
    let mut restarted = over(&fixture, &["a", "b"]);
    restarted.simulation.initial.push(second);
    assert_ne!(identity(&restarted), plain);

    // A clause of another package has no record in this subject's package.
    let other = counter(
        document(false),
        "fair weak each M::Counter::inc;",
        "always eventually holds(c.value = 3)",
    );
    let [foreign] = other.package.graph().temporal_clauses() else {
        panic!("one declared clause");
    };
    let mut request = ModelRequest::new();
    let refusal = author_clause(&mut request, &over(&fixture, &["a"]), foreign)
        .expect_err("the clause is not a claim site of the subject's package");
    assert_eq!(
        refusal,
        RequestRefusal::NoRequirementRecord {
            occurrence: node_occurrence(foreign)
        }
    );
    assert!(request.items().is_empty());
}
