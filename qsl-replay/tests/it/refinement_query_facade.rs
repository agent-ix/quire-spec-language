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
            source: SourceIdentity::new("agent-ix", "refinement"),
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

// These are authored adapter framing controls, not CG solver-run evidence.
type HandoffPart = (Option<u64>, Vec<u8>, Vec<u8>);

fn handoff(parts: &[HandoffPart]) -> Vec<u8> {
    let mut bytes = u64::try_from(parts.len())
        .expect("fixture count")
        .to_le_bytes()
        .to_vec();
    for (row, query, proof) in parts {
        match row {
            None => bytes.push(0),
            Some(row) => {
                bytes.push(1);
                bytes.extend_from_slice(&row.to_le_bytes());
            }
        }
        for field in [query, proof] {
            bytes.extend_from_slice(
                &u64::try_from(field.len())
                    .expect("fixture bytes")
                    .to_le_bytes(),
            );
            bytes.extend_from_slice(field);
        }
    }
    bytes
}

fn public_parts(request: &RefinementQueryRequest<'_>) -> Vec<HandoffPart> {
    use crate::refinement_proofs::{initial_proof, step_proof};
    let query = script(request, RefinementQueryKind::Initial);
    let mut parts = vec![(
        None,
        query.as_bytes().to_vec(),
        initial_proof(&query).into_bytes(),
    )];
    for row in 0..request.step_count().expect("selected steps") {
        let row = u64::try_from(row).expect("fixture row");
        let query = script(request, RefinementQueryKind::Step { row });
        parts.push((
            Some(row),
            query.as_bytes().to_vec(),
            step_proof(&query).into_bytes(),
        ));
    }
    parts
}

#[trace(
    "TC-1275",
    "FR-492-AC-8",
    "FR-492-AC-10",
    "FR-492-AC-12",
    "FR-492-AC-13"
)]
#[test]
fn public_handoff_preserves_selected_rows_and_delegates_the_native_checker() {
    use qsl_replay::{CertificateCheckLimits, SmtProofCheck};
    use quire_exact::Cancel;
    with_request(&cas_refines_counter(), |request, world| {
        assert_eq!(
            request.refinement_key(),
            &world.refinement().v2_emission().expect("emits").identity
        );
        assert_eq!(request.step_count().expect("row count"), 7);
        let parts = public_parts(request);
        let limits = CertificateCheckLimits::default();
        let certificate = request
            .assemble_smt_certificate(&handoff(&parts), &limits, &Cancel::new())
            .expect("complete adapter input");
        let mut reversed = parts.clone();
        reversed.reverse();
        assert_eq!(
            request.assemble_smt_certificate(&handoff(&reversed), &limits, &Cancel::new()),
            Ok(certificate.clone())
        );
        assert_eq!(certificate.base.query, parts[0].1);
        assert_eq!(certificate.base.proof, parts[0].2);
        for (part, expected) in certificate.steps.iter().zip(parts.iter().skip(1)) {
            assert_eq!(part.query, expected.1);
            assert_eq!(part.proof, expected.2);
        }
        assert_eq!(
            request.check_smt_proof(&certificate, &limits, &Cancel::new()),
            Ok(SmtProofCheck::Verified)
        );
    });
    let zero = cas_refines_counter()
        .lines()
        .filter(|line| !line.trim_start().starts_with("step "))
        .collect::<Vec<_>>()
        .join("\n");
    with_request(&zero, |request, _| {
        assert_eq!(request.step_count().expect("zero selected rows"), 0);
        let parts = public_parts(request);
        let certificate = request
            .assemble_smt_certificate(
                &handoff(&parts),
                &CertificateCheckLimits::default(),
                &Cancel::new(),
            )
            .expect("base only");
        assert!(certificate.steps.is_empty());
        assert_eq!(
            request.check_smt_proof(
                &certificate,
                &CertificateCheckLimits::default(),
                &Cancel::new()
            ),
            Ok(SmtProofCheck::Verified)
        );
    });
}

#[trace("TC-1275", "FR-492-AC-9")]
#[test]
fn public_handoff_refuses_invalid_framing_and_selected_record_shapes() {
    use qsl_replay::{CertificateCheckLimits, RefinementHandoffRefusal as Error};
    use quire_exact::Cancel;
    with_request(&cas_refines_counter(), |request, _| {
        let original = public_parts(request);
        let limits = CertificateCheckLimits::default();
        let assert_error = |bytes: Vec<u8>, error| {
            assert_eq!(
                request.assemble_smt_certificate(&bytes, &limits, &Cancel::new()),
                Err(error)
            )
        };
        for removed in [0, 4] {
            let mut parts = original.clone();
            parts.remove(removed);
            assert_error(handoff(&parts), Error::Shape);
        }
        for slot in [0, 4] {
            let mut parts = original.clone();
            parts[slot] = parts[1].clone();
            assert_error(handoff(&parts), Error::Shape);
        }
        let mut parts = original.clone();
        parts[4].0 = Some(7);
        assert_error(handoff(&parts), Error::Shape);
        assert_error(handoff(&original[..1]), Error::Shape);
        let honest = handoff(&original);
        for end in 0..honest.len() {
            let result = request.assemble_smt_certificate(&honest[..end], &limits, &Cancel::new());
            assert!(
                matches!(result, Err(Error::Malformed)),
                "truncated at {end}: {result:?}"
            );
        }
        let mut unknown = honest.clone();
        unknown[8] = 2;
        assert_error(unknown, Error::Malformed);
        let mut impossible = honest.clone();
        impossible[9..17].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_error(impossible, Error::Malformed);
        let mut trailing = honest;
        trailing.push(0);
        assert_error(trailing, Error::Malformed);
    });
}

#[trace("TC-1275", "FR-492-AC-10")]
#[test]
fn public_assembly_keeps_bad_proofs_present_for_the_native_checker() {
    use qsl_replay::{CertificateCheckLimits, CertificateLocus, CertificateRule, QueryPart};
    use quire_exact::Cancel;
    with_request(&cas_refines_counter(), |request, _| {
        let limits = CertificateCheckLimits::default();
        for (proof, rule) in [
            (vec![], CertificateRule::NotRefutation),
            (vec![0xff], CertificateRule::Malformed),
        ] {
            let mut parts = public_parts(request);
            parts[0].2 = proof.clone();
            let certificate = request
                .assemble_smt_certificate(&handoff(&parts), &limits, &Cancel::new())
                .expect("present proof assembles");
            assert_eq!(certificate.base.proof, proof);
            let rejected = request
                .check_smt_proof(&certificate, &limits, &Cancel::new())
                .expect_err("native proof rejection");
            assert_eq!(rejected.rule, rule);
            assert_eq!(
                rejected.at,
                CertificateLocus::Query {
                    part: QueryPart::Base
                }
            );
        }
        let mut parts = public_parts(request);
        parts[0].1.push(b' ');
        let certificate = request
            .assemble_smt_certificate(&handoff(&parts), &limits, &Cancel::new())
            .expect("query copied without reinterpretation");
        let rejected = request
            .check_smt_proof(&certificate, &limits, &Cancel::new())
            .expect_err("wrong query refuses");
        assert_eq!(rejected.rule, CertificateRule::QueryMismatch);
    });
}

#[trace("TC-1275", "FR-492-AC-11")]
#[test]
fn public_handoff_stops_at_its_own_exact_stage_limits_and_cancellation() {
    use qsl_replay::{
        CertificateCheckLimits, IncompleteCause, ModelCheckLimit,
        RefinementHandoffRefusal as Error, SmtProofCheck,
    };
    use quire_exact::{Cancel, CancelCause};
    with_request(&cas_refines_counter(), |request, _| {
        let bytes = handoff(&public_parts(request));
        let limits = CertificateCheckLimits {
            max_certificate_bytes: u64::try_from(bytes.len()).expect("size"),
            max_certificate_edges: 8,
            ..CertificateCheckLimits::default()
        };
        let certificate = request
            .assemble_smt_certificate(&bytes, &limits, &Cancel::new())
            .expect("exact assembly budgets");
        for bound in [0, limits.max_certificate_bytes - 1] {
            let short = CertificateCheckLimits {
                max_certificate_bytes: bound,
                ..limits
            };
            assert_eq!(
                request.assemble_smt_certificate(&bytes, &short, &Cancel::new()),
                Err(Error::Stopped(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::CertificateMaxBytes,
                    value: bound
                }))
            );
        }
        for bound in [0, 7] {
            let short = CertificateCheckLimits {
                max_certificate_edges: bound,
                ..limits
            };
            assert_eq!(
                request.assemble_smt_certificate(&bytes, &short, &Cancel::new()),
                Err(Error::Stopped(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::CertificateMaxEdges,
                    value: bound
                }))
            );
        }
        let cancel = Cancel::new();
        cancel.cancel(CancelCause::Requested);
        assert_eq!(
            request.assemble_smt_certificate(&bytes, &limits, &cancel),
            Err(Error::Stopped(IncompleteCause::Cancelled {
                source: CancelCause::Requested
            }))
        );
        assert!(matches!(
            request.check_smt_proof(&certificate, &limits, &Cancel::new()),
            Ok(SmtProofCheck::Stopped(IncompleteCause::LimitReached { .. }))
        ));
        assert_eq!(
            request.check_smt_proof(
                &certificate,
                &CertificateCheckLimits::default(),
                &Cancel::new()
            ),
            Ok(SmtProofCheck::Verified)
        );
    });
}

#[trace("TC-1275", "FR-492-AC-9", "FR-492-AC-13")]
#[test]
fn a_request_key_from_another_refinement_cannot_define_the_selected_rows() {
    use qsl_replay::{CertificateCheckLimits, RefinementHandoffRefusal, SmtEncodingRefusal};
    use quire_exact::Cancel;
    let foreign = World::new(&cas_refines_counter().replace(
        "refinement CasRefinesCounter ",
        "refinement OtherRefinement ",
    ));
    let foreign_key = foreign
        .refinement()
        .v2_emission()
        .expect("foreign refinement emits")
        .identity;
    with_closure(&cas_refines_counter(), |handle, world| {
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
        let request = RefinementQueryRequest::new(
            handle,
            foreign_key.clone(),
            subject(CAS, world.concrete_initial()),
            subject(SPEC, world.abstract_initial()),
        );
        assert_eq!(request.refinement_key(), &foreign_key);
        assert!(matches!(
            request.step_count(),
            Err(SmtEncodingRefusal::Fault(_))
        ));
        assert!(matches!(
            request.assemble_smt_certificate(
                &handoff(&[(None, vec![], vec![])]),
                &CertificateCheckLimits::default(),
                &Cancel::new()
            ),
            Err(RefinementHandoffRefusal::Fault(_))
        ));
    });
}
