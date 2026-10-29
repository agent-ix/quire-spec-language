// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-430 "Timed refinement" (TC-1170): S3 admits a refinement whose
//! concrete model is timed and whose abstract model is untimed, and refuses
//! a timed abstract model and a clock read in a row; EN-6 decides its
//! safety half on the symbolic product of zones with FR-142's discrete
//! product.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{
    check_timed_model, check_timed_refinement, TimedRefinementRequest, TimedRefinementUndecided,
    TimedSafetyOutcome, ZoneCheckItem, ZoneCheckLimits, ZoneCheckOutcome, ZoneCheckRequest,
    ZoneUndecided,
};
use qsl_eval::simulation::model::{ModelSystem, TimedSystem};
use qsl_eval::simulation::Limits;
use qsl_replay::{
    open_systems, replay_timed_refinement_trace, RefinementFailure, RefinementTraceSettlement,
    TemporalCounterexample,
};
use qsl_replay::{CertifiedItem, ZoneCertificateRequest};
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use qsl_route::reductions::{ObligationReductions, Reductions};
use qsl_semantics::check::CheckedRefinement;
use quire_exact::{Cancel, Rational};

use qsl_semantics::model::intake::meaning;
use serde_json::{json, Value as Json};

use crate::timed_world::{
    document, ident, refining, refining_from, renamed_package, wire_field, wire_type, Refining,
    DENSE, PACKAGE, PLACEHOLDER,
};

/// FR-430-AC-1's untimed `Req`: the object `c` (here `Call`) with `sent`
/// (here `inflight`), and `send` setting it.
pub(crate) fn req() -> Vec<u8> {
    renamed_package(&document(&[], &[("send", &["inflight"])]), "test/req")
}

/// FR-430-AC-1's timed `TReq`: `Req` with the clock `x`, `send` guarded
/// `x >= 1`.
pub(crate) fn treq() -> Vec<u8> {
    document(&["x"], &[("send", &["inflight"])])
}

/// The concrete model's steps of `counterexample`, each with its delay.
fn steps(counterexample: &TemporalCounterexample) -> Vec<(String, Rational)> {
    counterexample
        .prefix
        .iter()
        .map(|step| {
            let operation = step.transition.operation();
            let name = ["send", "enter", "undo"]
                .into_iter()
                .find(|name| operation.contains(&format!("/Call/{name}\"")))
                .unwrap_or_else(|| panic!("an operation of the fixture: {operation}"));
            (
                name.to_owned(),
                step.delay.clone().expect("a timed step has a delay"),
            )
        })
        .collect()
}

/// The subject every refinement obligation of these tests names.
pub(crate) const SUBJECT: &str = "timed-refinement";

/// `world`'s one refinement.
pub(crate) fn the_refinement(world: &Refining) -> &CheckedRefinement {
    let [refinement] = world.package.graph().refinements() else {
        panic!("one refinement");
    };
    refinement
}

/// The obligation of `world`'s one refinement item (FR-144). A refinement
/// with no record (one the v2 writer does not emit) names a fixed
/// occurrence: only a proof's certificate reads it.
pub(crate) fn obligation(world: &Refining) -> ItemObligation<&'static str> {
    let unrecorded = || {
        qsl_foundation::source::provenance::OccurrenceKey::new(
            qsl_foundation::digest::WireNodeId::from_digest([0; 32]),
            quire_exact::Origin::new(quire_exact::Role::new("declaration"), 0),
        )
    };
    ItemObligation {
        subject: SUBJECT,
        kind: ObligationKind::Refinement {
            occurrence: the_refinement(world)
                .requirement_occurrence()
                .cloned()
                .unwrap_or_else(unrecorded),
            abstract_initial: std::iter::once(world.abstract_initial.clone()).collect(),
        },
        reductions: ObligationReductions::default(),
    }
}

/// EN-6's request for the safety half of `world`'s one refinement.
pub(crate) fn safety_request(world: &Refining) -> TimedRefinementRequest<'_, &'static str> {
    TimedRefinementRequest {
        obligation: obligation(world),
        refinement: the_refinement(world),
        concrete: world.concrete_request(),
        abstract_package: &world.package,
        abstract_initial: vec![world.abstract_initial.clone()],
        limits: ZoneCheckLimits::default(),
    }
}

/// EN-6's safety half of `world`'s one refinement.
pub(crate) fn safety(world: &Refining) -> TimedSafetyOutcome<&'static str> {
    check_timed_refinement(safety_request(world), &Cancel::new())
        .expect("EN-6 takes the refinement")
}

/// FR-245's request for checking a certificate of `world`'s one refinement.
pub(crate) fn certificate_request(world: &Refining) -> ZoneCertificateRequest<'_> {
    ZoneCertificateRequest {
        simulation: world.concrete_request(),
        model: "M",
        max_clocks: qsl_eval::simulation::model::DEFAULT_MAX_CLOCKS,
        item: CertifiedItem::Refinement {
            refinement: the_refinement(world),
            abstract_package: &world.package,
            abstract_initial: std::slice::from_ref(&world.abstract_initial),
        },
    }
}

pub(crate) const CONTRACTS: &str = "post ASend using v on A::Call::send { self.inflight }\n\
     pre MSend using v on M::Call::send { self.x >= 1 ms }\n\
     post MSent using v on M::Call::send { self.inflight }\n";

/// The refinement of `A` by `M` with the field row `inflight = {row}`.
pub(crate) fn refinement(row: &str) -> String {
    format!(
        "refinement Timed using inf abstract A concrete M {{\n\
         population A::calls from M::calls;\n\
         object A::Call from M::Call {{ phase = self.phase; inflight = {row}; late = self.late; retried = self.retried; }}\n\
         step M::Call::send -> A::Call::send(self);\n\
         }}\n"
    )
}

pub(crate) fn unit(row: &str) -> Result<Refining, Vec<(String, String, String)>> {
    refining(
        &req(),
        &treq(),
        &format!("{DENSE}{CONTRACTS}{}", refinement(row)),
    )
}

#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn s3_admits_a_timed_concrete_model_under_an_untimed_abstract_one() {
    let world = unit("self.inflight").unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert_eq!(world.package.graph().refinements().len(), 1);
}

/// FR-430-AC-1's routing half: the timed `TReq`'s refinement record is
/// marked dense and routes to EN-6 alone with EN-1 and EN-6 registered.
/// Through `ModelRequest::derived`, the refinement brings its timed concrete
/// subject a deadlock-freedom and a time-lock-freedom item (FR-232), and
/// both route to EN-6 alone too (FR-243).
#[trace("TC-1170", "FR-430-AC-1")]
#[trace("TC-687", "FR-232-AC-1")]
#[trace("TC-698", "FR-243-AC-1")]
#[test]
fn the_timed_refinement_record_routes_to_en6_alone() {
    use qsl_analyze::zone_check::manifest as en6;
    use qsl_route::deadlock::ModelRefinementItem;
    use qsl_route::request::items_from_requirements;
    use qsl_route::request::ItemKind;

    let world = unit("self.inflight").unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let graph = world.package.graph();
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    let occurrence = refinement
        .requirement_occurrence()
        .expect("the refinement has a record");
    let record = graph.requirements().get(occurrence).expect("its record");
    assert!(record.requirements().timed());
    let registry = crate::dense_route::registry();
    let items = items_from_requirements(graph.requirements(), &registry, None);
    let item = items
        .iter()
        .find(|item| item.occurrence() == occurrence)
        .expect("the record's item");
    assert_eq!(
        crate::dense_route::ids(item.candidates()),
        [en6::BACKEND_ID]
    );

    let subject = "treq";
    let initial = [world.abstract_initial.clone()];
    let derived: Vec<_> = crate::derived_request::derived_items(
        &[],
        &[ModelRefinementItem {
            refinement,
            subject: &subject,
            abstract_initial: &initial,
            package: graph,
        }],
        &[],
        &[],
    )
    .into_iter()
    .map(|(item, obligation)| {
        assert_eq!(obligation.subject, subject);
        (
            item.item_kind(),
            crate::dense_route::ids(item.candidates().expect("its candidates")),
        )
    })
    .collect();
    assert_eq!(
        derived,
        [
            (
                Some(ItemKind::DeadlockFreedom),
                vec![en6::BACKEND_ID.to_owned()]
            ),
            (
                Some(ItemKind::TimeLockFreedom),
                vec![en6::BACKEND_ID.to_owned()]
            )
        ]
    );
}

#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn a_clock_read_in_the_mapping_is_refused_at_the_read() {
    let Err(refusals) = unit("self.x >= 1 ms") else {
        panic!("a clock read in the mapping is refused");
    };
    assert_eq!(
        refusals,
        [(
            "unsupported_construct".to_owned(),
            "expression-form".to_owned(),
            "self.x".to_owned()
        )]
    );
}

/// FR-430-AC-1: a refinement whose abstract model is `TReq` and whose
/// concrete model is a copy `TReq2` of it.
#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn a_timed_abstract_model_is_refused_at_its_reference() {
    let treq2 = renamed_package(&treq(), "test/treq2");
    let Err(refusals) = refining(
        &treq2,
        &treq(),
        &format!(
            "{DENSE}time A dense unit ms;\n{CONTRACTS}\
             pre ASend2 using v on A::Call::send {{ self.x >= 1 ms }}\n{}",
            refinement("self.inflight")
        ),
    ) else {
        panic!("a timed abstract model is refused");
    };
    assert_eq!(
        refusals,
        [(
            "unsupported_construct".to_owned(),
            "declaration-form".to_owned(),
            "A".to_owned()
        )]
    );
}

/// FR-430-AC-1: `TReq` refines `Req`, and EN-6's product holds. Its edges
/// are the discrete steps only: from `sent = false` and `x >= 0`, `send` at
/// `x >= 1` reaches `sent = true` and `x >= 1`, and `send` again is covered
/// by that state; no delay is an edge, so `check_step` runs for `send`
/// alone.
#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn the_safety_half_of_a_timed_refinement_holds_on_the_zone_product() {
    let world = unit("self.inflight").unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let TimedSafetyOutcome::Holds { stats, .. } = safety(&world) else {
        panic!("the safety half holds");
    };
    assert_eq!((stats.symbolic_states, stats.retained_edges), (2, 2));
}

/// FR-430 "Timed refinement": `check_step` runs on each discrete edge. With
/// `send` mapped to `stutter`, its edge changes the mapped `sent`, so the
/// half is violated at position 1, after the least admissible delay 1.
#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn a_discrete_edge_that_fails_check_step_refutes_after_its_delay() {
    let body = refinement("self.inflight").replace("-> A::Call::send(self)", "-> stutter");
    let world = refining(&req(), &treq(), &format!("{DENSE}{CONTRACTS}{body}"))
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let TimedSafetyOutcome::Violated { counterexample, .. } = safety(&world) else {
        panic!("the safety half is violated");
    };
    assert_eq!(
        steps(&counterexample),
        [("send".to_owned(), Rational::from_integer(1_i64.into()))]
    );
    assert!(
        matches!(
            counterexample.refinement,
            Some(RefinementFailure::StutterChanged { position: 1 })
        ),
        "{:?}",
        counterexample.refinement
    );
}

/// FR-430-AC-2's `TReq` with `trap` (here `late`): `send` resets `x`;
/// `enter` (precondition `sent`, guard `x >= 1`) sets `trap` and resets
/// `x`; `undo` (precondition `trap and sent`) clears `sent`. `trap` maps to
/// the abstract `late`, which stays false: the trap is no abstract state.
/// With `trapped`, the time invariant `when self.late { x <= 0 }`.
fn trap_undo(trapped: bool) -> Refining {
    let model = document(
        &["x"],
        &[
            ("send", &["inflight", "x"]),
            ("enter", &["late", "x"]),
            ("undo", &["inflight"]),
        ],
    );
    let contracts = "post ASend using v on A::Call::send { self.inflight }\n\
         pre MSend using v on M::Call::send { self.x >= 1 ms }\n\
         post MSent using v on M::Call::send { self.inflight and self.x = 0 ms }\n\
         pre MEnter using v on M::Call::enter { self.inflight and self.x >= 1 ms }\n\
         post MEntered using v on M::Call::enter { self.late and self.x = 0 ms }\n\
         pre MUndo using v on M::Call::undo { self.late and self.inflight }\n\
         post MUndone using v on M::Call::undo { not self.inflight }\n";
    let body = refinement("self.inflight")
        .replace("late = self.late", "late = false")
        .replace(
            "step M::Call::send -> A::Call::send(self);",
            "step M::Call::send -> A::Call::send(self);\n\
             step M::Call::enter -> stutter;\nstep M::Call::undo -> stutter;",
        );
    let trap = if trapped {
        "time invariant Trap when { self.late } { self.x <= 0 ms }\n"
    } else {
        ""
    };
    refining(&req(), &model, &format!("{DENSE}{contracts}{trap}{body}"))
        .unwrap_or_else(|refusals| panic!("{refusals:?}"))
}

/// FR-145 over time (QSpec FR-430): `world`'s refinement counterexample
/// `payload` replayed through FR-241's exact delays.
fn replay(world: &Refining, payload: &TemporalCounterexample) -> RefinementTraceSettlement {
    let [refinement] = world.package.graph().refinements() else {
        panic!("one refinement");
    };
    let systems = open_systems(
        refinement,
        &world.concrete_request(),
        &world.package,
        vec![world.abstract_initial.clone()],
    )
    .expect("the subjects open");
    replay_timed_refinement_trace(refinement, &systems, payload, &Limits::default())
        .expect("the counterexample replays")
}

/// FR-430-AC-2: without the time invariant, the safety half is violated by
/// `send` after delay 1, `enter` after delay 1 and `undo` after delay 0,
/// failing at position 3, and the counterexample replays through FR-241's
/// delays. With it, the only pre-state points of `undo` (trap set, `x = 0`)
/// reach no time-divergent continuation, so the failing edge does not
/// refute (ADR-026 TV-1) and the half holds, while the time-lock-freedom
/// item of that model is refuted. The same counterexample replayed over the
/// trapped model reproduces the failing step but settles inconclusive,
/// since its pre-state point is time-locked.
#[trace("TC-1170", "FR-430-AC-2")]
#[test]
fn a_failing_undo_refutes_only_where_time_can_diverge() {
    let free = trap_undo(false);
    let TimedSafetyOutcome::Violated { counterexample, .. } = safety(&free) else {
        panic!("without the invariant the half is violated");
    };
    let one = Rational::from_integer(1_i64.into());
    assert_eq!(
        steps(&counterexample),
        [
            ("send".to_owned(), one.clone()),
            ("enter".to_owned(), one),
            ("undo".to_owned(), Rational::from_integer(0_i64.into())),
        ]
    );
    assert_eq!(
        counterexample.refinement,
        Some(RefinementFailure::StutterChanged { position: 3 })
    );
    assert!(
        matches!(
            replay(&free, &counterexample),
            RefinementTraceSettlement::Reproduced {
                failure: RefinementFailure::StutterChanged { position: 3 },
                ..
            }
        ),
        "{:?}",
        replay(&free, &counterexample)
    );

    let trapped = trap_undo(true);
    let outcome = safety(&trapped);
    assert!(
        matches!(outcome, TimedSafetyOutcome::Holds { .. }),
        "{outcome:?}"
    );
    let time_lock = time_lock_of(&trapped);
    assert!(
        matches!(time_lock, ZoneCheckOutcome::Violated { .. }),
        "{time_lock:?}"
    );
    assert!(
        matches!(
            replay(&trapped, &counterexample),
            RefinementTraceSettlement::Inconclusive(_)
        ),
        "{:?}",
        replay(&trapped, &counterexample)
    );
}

/// FR-145 over time: a counterexample whose delay breaks a guard is
/// refused, as FR-237 refuses it, before any refinement check runs.
#[trace("TC-1170", "FR-430-AC-2")]
#[test]
fn a_timed_refinement_counterexample_with_an_early_step_is_refused() {
    let free = trap_undo(false);
    let TimedSafetyOutcome::Violated {
        mut counterexample, ..
    } = safety(&free)
    else {
        panic!("the half is violated");
    };
    counterexample.prefix[0].delay = Some(Rational::from_integer(0_i64.into()));
    let [refinement] = free.package.graph().refinements() else {
        panic!("one refinement");
    };
    let systems = open_systems(
        refinement,
        &free.concrete_request(),
        &free.package,
        vec![free.abstract_initial.clone()],
    )
    .expect("the subjects open");
    assert!(replay_timed_refinement_trace(
        refinement,
        &systems,
        &counterexample,
        &Limits::default()
    )
    .is_err());
}

/// FR-430-AC-1: a clock read in a step row is refused at the read.
#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn a_clock_read_in_a_step_row_is_refused_at_the_read() {
    let body = refinement("self.inflight").replace("A::Call::send(self)", "A::Call::send(self.x)");
    let Err(refusals) = refining(&req(), &treq(), &format!("{DENSE}{CONTRACTS}{body}")) else {
        panic!("a clock read in a step row is refused");
    };
    assert_eq!(
        refusals,
        [(
            "unsupported_construct".to_owned(),
            "expression-form".to_owned(),
            "self.x".to_owned()
        )]
    );
}

/// A field `name: type_ref` of the package record `owner`.
fn record_field(owner: &str, name: &str, type_ref: &str) -> Json {
    let mut field = wire_field(name, type_ref);
    field["identity"] = json!(ident(&format!("{owner}/{name}")));
    field
}

/// `TReq` whose `Call` also holds `t: Timer`, the package record `Timer {
/// x: Clock; n: Boolean }`, and `note: Note`, the record `Note { x: Boolean
/// }` whose `x` is no clock.
fn treq_with_records() -> Vec<u8> {
    const CLOCK: &str = "ix://quire/native/Clock";
    const BOOLEAN: &str = "ix://quire/native/Boolean";
    let mut document: Json = serde_json::from_slice(&treq()).expect("JSON");
    let types = document["types"].as_array_mut().expect("types");
    types.push(wire_type(
        &ident("Timer"),
        "value_record",
        json!({"fields": [record_field("Timer", "x", CLOCK), record_field("Timer", "n", BOOLEAN)]}),
    ));
    types.push(wire_type(
        &ident("Note"),
        "value_record",
        json!({"fields": [record_field("Note", "x", BOOLEAN)]}),
    ));
    let fields = types[1]["fields"].as_array_mut().expect("Call fields");
    fields.push(wire_field("t", &ident("Timer")));
    fields.push(wire_field("note", &ident("Note")));
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(json!({"kind": {"module": PACKAGE, "name": "value_record"},
            "moduleVersion": "1.0.0", "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning::RECORD_VALUE_TYPE}}));
    serde_json::to_vec(&document).expect("JSON")
}

/// The refusals of the unit over [`treq_with_records`] whose mapping row
/// is `inflight = {row}`, with the declarations `declarations`.
fn record_row(declarations: &str, row: &str) -> Result<(), Vec<(String, String, String)>> {
    refining(
        &req(),
        &treq_with_records(),
        &format!("{DENSE}{CONTRACTS}{declarations}{}", refinement(row)),
    )
    .map(|_| ())
}

/// QSpec FR-430 over FR-230's clock places: a row's read is a clock read by
/// its place path from the concrete object, not by its field name. A data
/// field of a clock-bearing record (`self.t.n`) and a record field named
/// like a clock that is no clock place (`self.note.x`) are admitted; the
/// clock place `self.t.x` is refused at the read.
#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn a_rows_clock_read_is_found_by_its_place_path() {
    assert_eq!(record_row("", "self.t.n"), Ok(()));
    assert_eq!(record_row("", "self.note.x"), Ok(()));
    assert_eq!(
        record_row("", "self.t.x >= 1 ms"),
        Err(vec![(
            "unsupported_construct".to_owned(),
            "expression-form".to_owned(),
            "self.t.x".to_owned()
        )])
    );
}

/// QSpec FR-430 over FR-230's clock-reading declarations: a row calling a
/// predicate or a function that reads a clock refuses `expression-form`
/// at the call, the read, never `ill_typed`.
#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn a_clock_read_through_a_predicate_or_function_is_refused_at_the_call() {
    let near = "predicate near using v(c: Clock, b: Quantity<ms>): Boolean { c <= b }\n";
    let pick = "function pick using v(a: Clock, b: Clock, left: Boolean): Clock pure \
                { if left then a else b }\n";
    for (declarations, row, at) in [
        (near, "near(self.x, 3 ms)", "near(self.x, 3 ms)"),
        (
            pick,
            "pick(self.x, self.x, true) >= 1 ms",
            "pick(self.x, self.x, true)",
        ),
    ] {
        assert_eq!(
            record_row(declarations, row),
            Err(vec![(
                "unsupported_construct".to_owned(),
                "expression-form".to_owned(),
                at.to_owned()
            )]),
            "{row}"
        );
    }
}

/// FR-430-AC-6's model: `TReq` with `trap` (here `late`) and `dead` (here
/// `retried`); `send` (precondition `not dead`, guard `x >= 1`) resets
/// `x`; `enter` (precondition `sent and not dead`, guard `x >= 1`) sets
/// `trap` and resets `x`; `undo` (precondition `trap and sent and not
/// dead`, guard `x <= 0`) clears `sent` and, with `dead`, sets `dead` under
/// the time invariant `when self.dead { x <= 0 }`. `trap` and `dead` map
/// to the abstract `late` and `retried`, which stay false.
pub(crate) fn dead_undo(dead: bool) -> Refining {
    undo_world(dead, dead)
}

/// FR-430-AC-6's model with `undo` setting `dead` when `retried`, under
/// the time invariant `when self.dead { x <= 0 }` when `invariant`: with
/// `retried` and no invariant, `undo`'s post-state is time-divergent.
pub(crate) fn undo_world(retried: bool, invariant: bool) -> Refining {
    let model = document(
        &["x"],
        &[
            ("send", &["inflight", "x"]),
            ("enter", &["late", "x"]),
            ("undo", &["inflight", "retried"]),
        ],
    );
    let undone = if retried {
        "not self.inflight and self.retried"
    } else {
        "not self.inflight and not self.retried"
    };
    let invariant = if invariant {
        "time invariant Dead when { self.retried } { self.x <= 0 ms }\n"
    } else {
        ""
    };
    let contracts = format!(
        "post ASend using v on A::Call::send {{ self.inflight }}\n\
         pre MSend using v on M::Call::send {{ not self.retried and self.x >= 1 ms }}\n\
         post MSent using v on M::Call::send {{ self.inflight and self.x = 0 ms }}\n\
         pre MEnter using v on M::Call::enter {{ self.inflight and not self.retried and self.x >= 1 ms }}\n\
         post MEntered using v on M::Call::enter {{ self.late and self.x = 0 ms }}\n\
         pre MUndo using v on M::Call::undo {{ self.late and self.inflight and not self.retried and self.x <= 0 ms }}\n\
         post MUndone using v on M::Call::undo {{ {undone} }}\n{invariant}"
    );
    let body = refinement("self.inflight")
        .replace("late = self.late", "late = false")
        .replace("retried = self.retried", "retried = false")
        .replace(
            "step M::Call::send -> A::Call::send(self);",
            "step M::Call::send -> A::Call::send(self);\n\
             step M::Call::enter -> stutter;\nstep M::Call::undo -> stutter;",
        );
    refining(&req(), &model, &format!("{DENSE}{contracts}{body}"))
        .unwrap_or_else(|refusals| panic!("{refusals:?}"))
}

/// The time-lock-freedom item of `world`'s concrete model.
fn time_lock_of(world: &Refining) -> ZoneCheckOutcome<&'static str> {
    let system = ModelSystem::new(world.concrete_request()).expect("the system admits");
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::TimeLockFreedom,
            obligation: ItemObligation {
                subject: "timed-refinement",
                kind: ObligationKind::TimeLockFreedom,
                reductions: ObligationReductions::default(),
            },
            limits: ZoneCheckLimits::default(),
            reductions: &Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("EN-6 takes the item")
}

/// FR-430-AC-6 (ADR-026 TV-1 read at the step's post-state): the `undo`
/// edge fails from a pre-state (trap and `sent` set, `x = 0`) whose
/// behaviour can still diverge, but its post-state (`dead` set, `x = 0`)
/// has no time-divergent continuation, so the half holds and the
/// time-lock-freedom item is refuted. The variant whose `undo` leaves
/// `dead` false refutes: `send` after delay 1, `enter` after delay 1,
/// `undo` after delay 0, `StutterChanged` at position 3, its last point in
/// the post-state of `undo`, replaying through FR-241 and CT-3.
#[trace("TC-1170", "FR-430-AC-6")]
#[test]
fn a_failing_step_refutes_only_where_its_post_state_can_diverge() {
    let dead = dead_undo(true);
    let outcome = safety(&dead);
    assert!(
        matches!(outcome, TimedSafetyOutcome::Holds { .. }),
        "{outcome:?}"
    );
    assert!(
        matches!(time_lock_of(&dead), ZoneCheckOutcome::Violated { .. }),
        "the time-lock-freedom item is refuted"
    );
    let live = dead_undo(false);
    let TimedSafetyOutcome::Violated { counterexample, .. } = safety(&live) else {
        panic!("the failing undo refutes where time runs on");
    };
    let one = Rational::from_integer(1_i64.into());
    assert_eq!(
        steps(&counterexample),
        [
            ("send".to_owned(), one.clone()),
            ("enter".to_owned(), one),
            ("undo".to_owned(), Rational::from_integer(0_i64.into())),
        ]
    );
    assert_eq!(
        counterexample.refinement,
        Some(RefinementFailure::StutterChanged { position: 3 })
    );
    assert!(matches!(
        replay(&live, &counterexample),
        RefinementTraceSettlement::Reproduced { .. }
    ));
}

/// FR-430-AC-7 (QSpec FR-430 "A failing initial state"): a concrete
/// initial `sent = false` under an abstract initial `sent = true` is
/// `Violated` with `InitialNotAbstract{initial: 0}` and an empty prefix,
/// and replays. With the time invariant `when true { x <= 0 }` and no
/// operation enabled, the initial state is time-locked, the failure does
/// not refute and the half settles TD-6's `NoAdmittedBehaviour`.
#[trace("TC-1170", "FR-430-AC-7")]
#[test]
fn a_failing_initial_state_refutes_only_where_it_can_diverge() {
    let unit = |locked: bool| {
        let invariant = if locked {
            "time invariant Stuck when { true } { self.x <= 0 ms }\n"
        } else {
            ""
        };
        refining_from(
            &req(),
            &treq(),
            &format!(
                "{DENSE}{CONTRACTS}{invariant}{}",
                refinement("self.inflight")
            ),
            true,
        )
        .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    };
    let free = unit(false);
    let TimedSafetyOutcome::Violated { counterexample, .. } = safety(&free) else {
        panic!("the initial state is no abstract one");
    };
    assert!(counterexample.prefix.is_empty());
    assert_eq!(
        counterexample.refinement,
        Some(RefinementFailure::InitialNotAbstract { initial: 0 })
    );
    assert!(matches!(
        replay(&free, &counterexample),
        RefinementTraceSettlement::Reproduced { .. }
    ));
    let outcome = safety(&unit(true));
    assert!(
        matches!(
            outcome,
            TimedSafetyOutcome::Undecided {
                cause: TimedRefinementUndecided::Zone(ZoneUndecided::NoAdmittedBehaviour),
                ..
            }
        ),
        "{outcome:?}"
    );
}

/// FR-430-AC-7: an `Undetermined` step verdict is no refutation and TV-1
/// does not read it. The unread history field `writes: Int[0, 1]`,
/// counting `send`s, leaves the range at the second `send`, and the half
/// returns `Undecided(Undetermined)` both where that step's post-state
/// diverges and where the time invariant `when self.inflight { x <= 0 }`
/// time-locks it.
#[trace("TC-1170", "FR-430-AC-7")]
#[test]
fn an_undetermined_step_is_undecided_whatever_its_post_state() {
    let history =
        "history M::Call.writes : Int[0, 1] = 0 { on M::Call::send : pre(self.writes) + 1; }\n";
    for locked in [false, true] {
        let (guard, invariant) = if locked {
            (
                "(not self.inflight and self.x >= 1 ms) or self.inflight",
                "time invariant Hold when { self.inflight } { self.x <= 0 ms }\n",
            )
        } else {
            ("self.x >= 1 ms", "")
        };
        let contracts = format!(
            "post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre MSend using v on M::Call::send {{ {guard} }}\n\
             post MSent using v on M::Call::send {{ self.inflight and self.x = 0 ms }}\n{invariant}"
        );
        let body = refinement("self.inflight").replace(
            "step M::Call::send",
            &format!("{history}step M::Call::send"),
        );
        let model = document(&["x"], &[("send", &["inflight", "x"])]);
        let world = refining(&req(), &model, &format!("{DENSE}{contracts}{body}"))
            .unwrap_or_else(|refusals| panic!("{refusals:?}"));
        let outcome = safety(&world);
        assert!(
            matches!(
                outcome,
                TimedSafetyOutcome::Undecided {
                    cause: TimedRefinementUndecided::Undetermined(_),
                    ..
                }
            ),
            "locked {locked}: {outcome:?}"
        );
    }
}

/// The record `analyze` settles for `world`'s one refinement on EN-6,
/// through `timed_refinement_item` and FR-145's replay over FR-241's delays.
fn analyzed(world: &Refining) -> qsl_analyze::analyze::AnalyzeRecord {
    use qsl_analyze::analyze::{analyze, timed_refinement_item, Engines, TimedRefinementItem};
    let [refinement] = world.package.graph().refinements() else {
        panic!("one refinement");
    };
    let concrete = world.concrete_request();
    let item = TimedRefinementItem {
        request: safety_request(world),
        certificate: certificate_request(world),
        certificate_limits: qsl_replay::CertificateCheckLimits::default(),
        replay: Box::new(move |payload| {
            let systems = open_systems(
                refinement,
                &concrete,
                &world.package,
                vec![world.abstract_initial.clone()],
            )
            .map_err(|_| {
                qsl_replay::ReplayRefusal::ModelTrace(Box::new(
                    qsl_replay::ModelTraceRefusal::NoRefinementFailure,
                ))
            })?;
            replay_timed_refinement_trace(refinement, &systems, payload, &Limits::default())
        }),
    };
    let mut records = analyze(
        &Engines::qsl().expect("the layer-A manifests register"),
        vec![timed_refinement_item(
            qsl_foundation::RequestIndex::new(0),
            item,
        )],
        &Cancel::new(),
    );
    assert_eq!(records.len(), 1);
    records.remove(0)
}

/// FR-430-AC-1 and FR-144 end to end (TC-1170): the `TReq` refinement with
/// `send` mapped to `stutter` runs on EN-6 through `analyze` and settles
/// `refuted` once its counterexample replays. The holding `TReq` settles
/// `Proved{ZoneCertified, Certified}` on EN-6, once FR-245 accepts its
/// refinement-form certificate (FR-430-AC-8).
#[trace("TC-1170", "FR-430-AC-1")]
#[trace("TC-1170", "FR-430-AC-8")]
#[test]
fn a_timed_refinement_settles_on_en6_through_analyze() {
    use qsl_analyze::analyze::Settlement;
    use qsl_analyze::zone_check::manifest::BACKEND_ID;
    use qsl_replay::{Certification, ProofBasis, TerminalValue};

    let body = refinement("self.inflight").replace("-> A::Call::send(self)", "-> stutter");
    let violated = refining(&req(), &treq(), &format!("{DENSE}{CONTRACTS}{body}"))
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let record = analyzed(&violated);
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(*terminal.value(), TerminalValue::Refuted);
    assert_eq!(
        terminal.backend().map(qsl_foundation::BackendId::as_str),
        Some(BACKEND_ID)
    );

    let holds = unit("self.inflight").unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let record = analyzed(&holds);
    let Settlement::Terminal(terminal) = &record.settlement else {
        panic!("a terminal record: {:?}", record.settlement);
    };
    assert_eq!(
        *terminal.value(),
        TerminalValue::Proved {
            basis: ProofBasis::ZoneCertified,
            certification: Certification::Certified,
        }
    );
    assert_eq!(
        terminal.backend().map(qsl_foundation::BackendId::as_str),
        Some(BACKEND_ID)
    );
}
