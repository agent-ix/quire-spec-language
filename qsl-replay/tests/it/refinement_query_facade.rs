// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1275 (FR-492-AC-1 to FR-492-AC-3): a consumer encodes a refinement's
//! `Initial` and `Step { row }` queries and reads the ordered step-row
//! mapping through the `qsl_replay` root. The queries and mapping are called
//! as `qsl_replay::{encode_refinement_query, refinement_step_sources}`
//! over `RefinementCertificateRequest` and `RefinementQueryKind` as the root
//! exports them; the fixture world (the checked graph, the oracle) is built
//! with the shared refinement support.

use ix_trace_rs::trace;
use qsl_replay::{
    encode_refinement_query, refinement_step_sources, RefinementCertificateRequest,
    RefinementQueryKind,
};

use crate::refinement_fixture::{cas_refines_counter, mutated, World};
use crate::refinement_proofs::{row_of, with_request};

fn is_permutation(sources: &[u64]) -> bool {
    let mut sorted = sources.to_vec();
    sorted.sort_unstable();
    sorted == (0..sources.len() as u64).collect::<Vec<_>>()
}

fn script(request: &RefinementCertificateRequest<'_>, kind: RefinementQueryKind) -> String {
    String::from_utf8(encode_refinement_query(request, kind).expect("encodes")).expect("UTF-8")
}

/// FR-492-AC-1: the root-exported entry encodes the `Initial` query and the
/// `Step` query of a named row, and the script of a row is not the script of
/// another row.
#[trace("TC-1275", "FR-492-AC-1")]
#[test]
fn the_root_encodes_initial_and_step_queries() {
    let world = World::new(&cas_refines_counter());
    let peek = row_of(&world, "peek");
    let commit = row_of(&world, "commitA");
    with_request(&world, |request| {
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

/// FR-492-AC-2: the step-row mapping is a permutation of the source rows,
/// equals the order the checker emitted, and sends each query row to the
/// source row of the same operation.
#[trace("TC-1275", "FR-492-AC-2")]
#[test]
fn the_step_row_mapping_is_the_source_row_permutation() {
    let world = World::new(&cas_refines_counter());
    with_request(&world, |request| {
        let sources = refinement_step_sources(request).expect("maps");
        assert_eq!(sources.len(), 7);
        assert!(is_permutation(&sources), "{sources:?}");
        let oracle = world
            .refinement()
            .v2_emission()
            .expect("emits")
            .step_sources;
        assert_eq!(
            sources,
            oracle.iter().map(|s| *s as u64).collect::<Vec<_>>()
        );
        for name in ["peek", "commitA", "commitB", "beginA", "retryB"] {
            let row = row_of(&world, name) as usize;
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
    let first = World::new(&base);
    let second = World::new(&swapped);
    let sources =
        |world: &World| with_request(world, |r| refinement_step_sources(r).expect("maps"));
    let (a, b) = (sources(&first), sources(&second));
    assert!(is_permutation(&a) && is_permutation(&b));
    assert_ne!(a, b, "the written order differs, so the mapping differs");
    let query = |world: &World, name: &str| {
        let row = row_of(world, name);
        with_request(world, |r| script(r, RefinementQueryKind::Step { row }))
    };
    assert_eq!(query(&first, "commitA"), query(&second, "commitA"));
}
