// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1275 (FR-492-AC-1 to FR-492-AC-3): a consumer encodes a refinement's
//! `Initial` and `Step { row }` queries and reads the ordered step-row
//! mapping through the `qsl_replay` root. The queries and mapping are called
//! as `qsl_replay::{encode_refinement_query, refinement_step_sources}`
//! over a `RefinementQueryRequest` built from `qsl_replay` root items alone
//! (the retained checked closure from `acquire_checked_closure`); the fixture world (the checked graph, the oracle) is built
//! with the shared refinement support.

use ix_trace_rs::trace;
use qsl_replay::{
    acquire_checked_closure, encode_refinement_query, refinement_step_sources, DependencyInput,
    ModelNormalizationLimits, ObservationLimits, RefinementQueryKind, RefinementQueryRequest,
    RefinementQuerySubject, RefinementUniverse, ReplayLimits, SourceIdentity, StageLimits,
};

use crate::refinement_fixture::{
    cas_refines_counter, key, mutated, packages, unit_text, Variant, World, CAS, SPEC, UNLIMITED,
};
use crate::refinement_proofs::row_of;

/// Run `body` with the facade request of the refinement text `refinement`,
/// built from `qsl_replay` root items alone: the retained checked closure of
/// the unit, the refinement's node key and the two subjects. The `World`
/// supplies only the fixture's values (documents, snapshots, the node key).
fn with_request<T>(
    refinement: &str,
    body: impl FnOnce(&RefinementQueryRequest<'_>, &World) -> T,
) -> T {
    let world = World::new(refinement);
    let documents = packages(Variant::CAS);
    let text = unit_text(&documents, refinement, Variant::CAS);
    let compiled = acquire_checked_closure(
        SourceIdentity::new("agent-ix", "refinement", "git", "1"),
        "refinement.native",
        text.as_bytes(),
        documents.values().map(Vec::as_slice),
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("the unit compiles");
    let subject = |model: &str, initial| RefinementQuerySubject {
        packages: world.package_provision(),
        snapshots: world.snapshot_provision(),
        initial: vec![initial],
        universes: vec![RefinementUniverse {
            population: key(model, "counters"),
            keys: world.object_keys().to_vec(),
        }],
        max_candidates: 1_000_000,
        normalization: ModelNormalizationLimits::default(),
        observation: ObservationLimits::default(),
        meter: UNLIMITED,
        proof_bounds: Vec::new(),
    };
    let request = RefinementQueryRequest::new(
        compiled.handle(),
        world.refinement().v2_emission().expect("emits").identity,
        subject(CAS, world.concrete_initial()),
        subject(SPEC, world.abstract_initial()),
    );
    body(&request, &world)
}

fn is_permutation(sources: &[u64]) -> bool {
    let mut sorted = sources.to_vec();
    sorted.sort_unstable();
    sorted == (0..sources.len() as u64).collect::<Vec<_>>()
}

fn script(request: &RefinementQueryRequest<'_>, kind: RefinementQueryKind) -> String {
    String::from_utf8(encode_refinement_query(request, kind).expect("encodes")).expect("UTF-8")
}

/// FR-492-AC-1: the facade entry encodes the `Initial` query and the
/// `Step` query of a named row, and the script of a row is not the script of
/// another row.
#[trace("TC-1275", "FR-492-AC-1")]
#[test]
fn the_facade_encodes_initial_and_step_queries() {
    with_request(&cas_refines_counter(), |request, world| {
        let peek = row_of(world, "peek");
        let commit = row_of(world, "commitA");
        let initial = script(request, RefinementQueryKind::Initial);
        assert!(initial.starts_with("(set-logic QF_LIA)\n"));
        assert!(initial.ends_with("(check-sat)\n"));
        assert!(!initial.contains("s1."));
        let stutter = script(request, RefinementQueryKind::Step { row: peek });
        let operation = script(request, RefinementQueryKind::Step { row: commit });
        assert!(stutter.contains("s1.c.value"));
        assert_ne!(stutter, operation);
        assert_eq!(
            stutter,
            script(request, RefinementQueryKind::Step { row: peek })
        );
    });
}

/// FR-492-AC-2: the step-row mapping is a permutation of the source rows and
/// sends each query row to the source row of the same operation.
#[trace("TC-1275", "FR-492-AC-2")]
#[test]
fn the_step_row_mapping_is_the_source_row_permutation() {
    with_request(&cas_refines_counter(), |request, world| {
        let sources = refinement_step_sources(request).expect("maps");
        assert_eq!(sources.len(), 7);
        assert!(is_permutation(&sources), "{sources:?}");
        for name in ["peek", "commitA", "commitB", "beginA", "retryB"] {
            let row = row_of(world, name) as usize;
            let source = sources[row] as usize;
            assert!(world.refinement().steps()[source]
                .spelling()
                .ends_with(name));
        }
    });
}

/// FR-492-AC-3: rewriting the rows in another written order changes the
/// mapping, not the query a given operation's row encodes.
#[trace("TC-1275", "FR-492-AC-3")]
#[test]
fn reordering_the_written_rows_changes_the_mapping_not_the_query() {
    let base = cas_refines_counter();
    let swapped = mutated(
        &base,
        "step Impl::Counter::commitA -> Spec::Counter::inc(self);\nstep Impl::Counter::commitB -> Spec::Counter::inc(self);\n",
        "step Impl::Counter::commitB -> Spec::Counter::inc(self);\nstep Impl::Counter::commitA -> Spec::Counter::inc(self);\n",
    );
    let probe = |text: &str| {
        with_request(text, |request, world| {
            let row = row_of(world, "commitA");
            (
                refinement_step_sources(request).expect("maps"),
                script(request, RefinementQueryKind::Step { row }),
            )
        })
    };
    let ((a, a_query), (b, b_query)) = (probe(&base), probe(&swapped));
    assert!(is_permutation(&a) && is_permutation(&b));
    assert_ne!(a, b, "the written order differs, so the mapping differs");
    assert_eq!(a_query, b_query);
}
