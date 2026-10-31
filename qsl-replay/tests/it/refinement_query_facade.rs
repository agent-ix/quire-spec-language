// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1275 (FR-492-AC-1, FR-492-AC-5 to FR-492-AC-7): a consumer encodes a
//! refinement's `Initial` and `Step { row }` queries through the `qsl_replay`
//! root. The queries are called as `qsl_replay::encode_refinement_query`
//! over a `RefinementQueryRequest` built from `qsl_replay` root items alone
//! (the retained checked closure from `acquire_checked_closure`, the node key
//! from `refinement_node_key`, the population key from
//! `RefinementUniverse::for_population`). The fixture supplies only document
//! values and the assertion oracle.

use ix_trace_rs::trace;
use qsl_replay::{
    acquire_checked_closure, encode_refinement_query, CheckedClosureHandle, DependencyInput,
    RefinementKeyRefusal, RefinementQueryKind, RefinementQueryRequest, RefinementQuerySubject,
    RefinementUniverse, ReplayLimits, SourceIdentity, StageLimits,
};

use crate::refinement_fixture::{
    cas_refines_counter, key, packages, unit_text, Variant, World, CAS, SPEC, UNLIMITED,
};
use crate::refinement_proofs::row_of;

/// The declared name of the refinement in the fixture text.
const REFINEMENT: &str = "CasRefinesCounter";

/// Run `body` with the retained checked closure of the unit that declares
/// the refinement text `refinement`, obtained through `acquire_checked_closure`.
fn with_closure<T>(refinement: &str, body: impl FnOnce(&CheckedClosureHandle, &World) -> T) -> T {
    let world = World::new(refinement);
    let documents = packages(Variant::CAS);
    let text = unit_text(&documents, refinement, Variant::CAS);
    let compiled = acquire_checked_closure(
        qsl_replay::CompileUnit {
            source: SourceIdentity::new("agent-ix", "refinement", "git", "1"),
            package_name: "fixture".to_owned(),
            path: "refinement.native",
            bytes: text.as_bytes(),
        },
        documents.values().map(Vec::as_slice),
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("the unit compiles");
    body(compiled.handle(), &world)
}

/// Run `body` with the facade request of the refinement text `refinement`.
///
/// The closure, the refinement's `NodeKey` (by declared name) and the
/// population's `DeclarationKey` (by declared package and name) come from
/// `qsl_replay` root items alone. What still comes from the fixture `World`
/// is the document values: the provision maps, the initial documents and the
/// object keys (QSL-1282 scope), and the oracle used only by the assertions.
fn with_request<T>(
    refinement: &str,
    body: impl FnOnce(&RefinementQueryRequest<'_>, &World) -> T,
) -> T {
    with_closure(refinement, |handle, world| {
        let subject = |model: &str, initial| {
            RefinementQuerySubject::new(
                world.package_provision(),
                world.snapshot_provision(),
                vec![initial],
                vec![RefinementUniverse::for_population(
                    model,
                    "counters",
                    world.object_keys().to_vec(),
                )],
                1_000_000,
                UNLIMITED,
            )
        };
        let node = handle
            .refinement_node_key(REFINEMENT)
            .expect("the refinement is declared");
        let request = RefinementQueryRequest::new(
            handle,
            node,
            subject(CAS, world.concrete_initial()),
            subject(SPEC, world.abstract_initial()),
        );
        body(&request, world)
    })
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

/// FR-492-AC-5: the declared name of a refinement resolves to its node key,
/// and the key names the same refinement the checked graph holds.
#[trace("TC-1275", "FR-492-AC-5")]
#[test]
fn a_declared_refinement_name_resolves_to_its_node_key() {
    with_closure(&cas_refines_counter(), |handle, world| {
        let node = handle.refinement_node_key(REFINEMENT).expect("resolves");
        assert_eq!(
            node,
            world.refinement().v2_emission().expect("emits").identity
        );
        assert_eq!(
            node,
            handle.refinement_node_key(REFINEMENT).expect("stable")
        );
    });
}

/// FR-492-AC-6: an unknown name and a name declared as another kind each
/// refuse with their own typed refusal.
#[trace("TC-1275", "FR-492-AC-6")]
#[test]
fn an_unknown_or_non_refinement_name_is_refused() {
    with_closure(&cas_refines_counter(), |handle, _| {
        assert_eq!(
            handle.refinement_node_key("Missing"),
            Err(RefinementKeyRefusal::UnknownName {
                name: "Missing".to_owned()
            })
        );
        assert_eq!(
            handle.refinement_node_key("CanInc"),
            Err(RefinementKeyRefusal::NotARefinement {
                name: "CanInc".to_owned()
            })
        );
    });
}

/// FR-492-AC-7: the by-name population constructor builds the declaration key
/// of the population declared in the package.
#[trace("TC-1275", "FR-492-AC-7")]
#[test]
fn a_population_universe_is_keyed_by_declared_name() {
    let universe = RefinementUniverse::for_population(CAS, "counters", vec!["a".to_owned()]);
    assert_eq!(
        universe,
        RefinementUniverse::new(key(CAS, "counters"), vec!["a".to_owned()])
    );
}
