// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-219, FR-220, FR-221, FR-225, FR-227 and FR-229 (TC-664, TC-665,
//! TC-666, TC-670, TC-672, TC-674): request memory selection, EN-1's
//! access classification and pre-check, and `ProtocolSystem<Tso>` over
//! ADR-025 §10's store buffering (`crate::litmus`).
//!
//! State counts follow ADR-025 §10's convention: states inside the
//! `parallel` (some branch thread live) and joined states (the root at
//! `finish`). The protocol system also has the state before the fork and
//! the states after `finish`, which §10's table does not count.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_protocol, ProtocolCheckRequest};
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ProtocolSubject};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::protocol::{
    classify_accesses, visible, AccessShape, CarriedComponent, ClaimReads, InstanceAccess,
    MemoryModel, MemorySelection, MemorySelectionRefusal, PositionAnchor, ProtocolLocation,
    ProtocolRefusal, ProtocolState, ProtocolSystem, ProtocolTransition, RestKind, Sc, StepKind,
    ThreadPath, Tso, WaitCause, WeakAccesses,
};
use qsl_eval::simulation::{independent, Location, SubjectSystem, TransitionSystem};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{MemoryModelName, SourceIdentity};
use qsl_package::CheckedPackage;
use qsl_replay::{
    compile_package, replay_protocol_trace, settle_protocol_check, DependencyInput,
    InconclusiveCause, MemoryBound, MemoryBoundMember, MemoryCause, ModelCheckItem,
    ModelCheckLimits, ModelCheckOutcome, ModelCheckRecord, ModelTraceRefusal,
    ModelTraceReplayResult, ModelTraceSettlement, ModelTraceSubject, ProtocolCertificateRequest,
    ProtocolTraceSubject, ReplayLimits, ReplayRefusal, ResultLabel, StatedMemoryBound,
    TemporalCounterexample, TerminalValue, UnavailabilityCause,
};
use qsl_semantics::check::{AccessClass, FormulaNodeId};
use quire_exact::{Cancel, Incomplete, Meter};
use serde_json::{json, Value as Json};

use crate::fill::{reachable, Reached};
use crate::litmus::{
    attempt, contracts, core, document, field, package, protocol, protocol_with, sb_run, sb_unit,
    unit_text, Documents, NO_BOTH_ZERO,
};
use crate::model_settle::{envelope_at, request};

/// The request entry selecting `model` for `parallel`.
fn select(parallel: &str, model: MemoryModelName) -> MemorySelection {
    MemorySelection {
        parallel: parallel.to_owned(),
        model,
    }
}

/// One unit, its documents and its checked package.
struct Case {
    body: String,
    documents: Documents,
    checked: CheckedPackage,
    /// The letters of the unit's one claim.
    letters: &'static dyn ModelLetters,
}

impl Case {
    fn new(body: &str) -> Self {
        Self::with_letters(body, &NoBothZero)
    }

    /// The unit with `body`, its claim's atom read by `letters`.
    fn with_letters(body: &str, letters: &'static dyn ModelLetters) -> Self {
        Self {
            body: body.to_owned(),
            documents: Documents::new(),
            checked: package(body),
            letters,
        }
    }

    fn subject(&self, selection: &[MemorySelection]) -> ProtocolSubject<'_> {
        self.documents.subject(&self.checked, "SB", selection)
    }

    /// EN-1's classification of `SB` under `selection`.
    fn accesses(&self, selection: &[MemorySelection]) -> WeakAccesses {
        let subject = self.subject(selection);
        let model = ModelSystem::new(subject.model.simulation.clone()).expect("admits");
        let protocol = self.checked.graph().protocol("SB").expect("SB");
        classify_accesses(&model, protocol, &subject.memory).expect("classifies")
    }

    /// The protocol system of `SB` under `memory`.
    fn system<M: MemoryModel>(
        &self,
        selection: &[MemorySelection],
        memory: M,
    ) -> Result<ProtocolSystem<'_, M>, ProtocolRefusal> {
        let subject = self.subject(selection);
        let model = ModelSystem::new(subject.model.simulation.clone()).expect("admits");
        ProtocolSystem::new(model, &subject, memory)
    }

    /// EN-1 over `SB` with `item`, the selection and the store buffer bound.
    fn check(
        &self,
        selection: &[MemorySelection],
        item: ModelCheckItem<'_>,
        max_store_buffer: Option<u64>,
    ) -> ModelCheckOutcome<ProtocolTransition> {
        check_protocol(
            ProtocolCheckRequest {
                subject: self.subject(selection),
                item,
                max_depth: None,
                max_store_buffer,
                limits: ModelCheckLimits::default(),
            },
            &Cancel::new(),
        )
        .expect("the protocol subject checks")
    }

    /// EN-1 over `SB`'s claim.
    fn claim(
        &self,
        selection: &[MemorySelection],
        max_store_buffer: Option<u64>,
    ) -> ModelCheckOutcome<ProtocolTransition> {
        let clause = self.checked.graph().temporal_clauses()[0].clause();
        self.check(
            selection,
            ModelCheckItem::Clause {
                clause,
                letters: self.letters,
            },
            max_store_buffer,
        )
    }

    /// FR-127's record of `outcome` over the claim (the deadlock-freedom
    /// item when `deadlock`), its counterexample replayed by FR-217.
    fn settle(
        &self,
        selection: &[MemorySelection],
        outcome: &ModelCheckOutcome<ProtocolTransition>,
        deadlock: bool,
        max_store_buffer: Option<u64>,
    ) -> ModelCheckRecord<ProtocolTransition> {
        let item = if deadlock {
            ModelCheckItem::DeadlockFreedom
        } else {
            ModelCheckItem::Clause {
                clause: self.checked.graph().temporal_clauses()[0].clause(),
                letters: self.letters,
            }
        };
        settle_protocol_check(
            outcome,
            &ModelCheckLimits::default(),
            &self.package_id(),
            &ProtocolCertificateRequest {
                subject: self.subject(selection),
                item,
                max_store_buffer,
            },
            &Cancel::new(),
            |payload| self.replay(selection, payload.clone()),
        )
    }

    /// FR-098's `package_id` of the unit.
    fn package_id(&self) -> DigestRecord {
        compile_package(
            SourceIdentity::new("agent-ix", "litmus", "git", "1"),
            "litmus.native",
            unit_text(&self.body).as_bytes(),
            [document().as_slice()],
            &DependencyInput::default(),
            qsl_replay::StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("the unit compiles")
        .package_id()
    }

    /// The subject FR-217's replay reads, its resolved models `selection`'s
    /// (FR-227 Inputs).
    fn trace_subject(&self, selection: &[MemorySelection]) -> ProtocolTraceSubject {
        let subject = self.subject(selection);
        ProtocolTraceSubject {
            model: ModelTraceSubject {
                initial: vec![self.documents.initial.clone()],
                universes: Documents::universes(),
                expansion: subject.model.simulation.expansion,
            },
            protocol: "SB".to_owned(),
            over: core(),
            roles: subject.roles.clone(),
            memory: subject.memory.clone(),
            max_store_buffer: None,
        }
    }

    /// FR-217's replay of `payload`, a counterexample to the claim, with
    /// the subject's resolved models replaced by `selection`'s.
    fn replay(
        &self,
        selection: &[MemorySelection],
        payload: TemporalCounterexample<ProtocolTransition>,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        let unit = unit_text(&self.body).into_bytes();
        let id = self.package_id();
        let node = &self.checked.graph().temporal_clauses()[0];
        let clause_node = WireNodeId::from_digest(*node.identity().as_bytes());
        let occurrence = OccurrenceKey::new(clause_node, node.claim().clone());
        replay_protocol_trace(
            request(&document(), self.documents.snapshots.iter(), &unit, id),
            &self.trace_subject(selection),
            &envelope_at(&unit, id, payload, clause_node, occurrence),
            self.letters,
            ReplayLimits::default(),
        )
    }
}

/// The claim's one atom, `not (k.r1 = 0 and k.r2 = 0)`, read from the
/// bound core's registers.
struct NoBothZero;

impl ModelLetters for NoBothZero {
    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        let both = field(state, "cores", &instance.key, "r1") == 0
            && field(state, "cores", &instance.key, "r2") == 0;
        Ok(if both {
            AtomReading::False
        } else {
            AtomReading::True
        })
    }
}

/// The branch thread `label` of `Both`'s first instance.
fn branch(label: &str) -> ThreadPath {
    ThreadPath::root(0).child("Both", label, 0)
}

/// ADR-025 §10's counts: states with a live branch thread, and states
/// whose root rests at `finish`.
fn counts<M: MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    reached: &[Reached<M>],
) -> (usize, usize) {
    let mut inside = 0;
    let mut joined = 0;
    for state in reached {
        let threads = system.threads(&state.state).expect("threads");
        if threads.iter().any(|thread| !thread.path.steps.is_empty()) {
            inside += 1;
        } else if threads.iter().any(|thread| {
            thread.rest_point.kind == RestKind::Node
                && thread.rest_point.node.as_deref() == Some("Done")
        }) {
            joined += 1;
        }
    }
    (inside, joined)
}

/// The `(object key, field name)` of a location `accesses` has a cell for.
fn named(accesses: &WeakAccesses, location: &Location) -> (String, String) {
    let cell = accesses.cell(location).expect("a touched field has a cell");
    (cell.key.clone(), cell.name.clone())
}

/// The class of each instance of `node` and its shared location's name.
fn classes(accesses: &WeakAccesses, node: &str) -> Vec<(AccessClass, (String, String))> {
    accesses
        .attempt(node)
        .expect("a weak attempt")
        .instances
        .iter()
        .map(|instance| match &instance.access {
            InstanceAccess::Access {
                class,
                location: Some(location),
            } => (*class, named(accesses, location)),
            other => panic!("{node}: one shared location, got {other:?}"),
        })
        .collect()
}

/// The names of the locations `node`'s instances touch that are not
/// shared, field locations only.
fn local(accesses: &WeakAccesses, node: &str) -> BTreeSet<(String, String)> {
    accesses
        .attempt(node)
        .expect("a weak attempt")
        .instances
        .iter()
        .flat_map(|instance| instance.local.iter())
        .filter(|location| matches!(location, Location::Field { .. }))
        .map(|location| named(accesses, location))
        .collect()
}

fn name(key: &str, field: &str) -> (String, String) {
    (key.to_owned(), field.to_owned())
}

/// A `parallel` nested in `SB::Both`'s left branch.
fn nested_unit(inner: &str) -> String {
    let run = format!(
        "parallel Both {{ \
         branch left sequence L {{ {} parallel Inner {inner} {{ branch a sequence A {{ }} \
         branch b sequence B {{ }} }} join all [a, b]; {} }} \
         branch right sequence R {{ {} {} }} }} join all [left, right];",
        attempt("Sx", "w", "Cell::set", "release", "IsX, StoreOne"),
        attempt("Ly", "c", "Core::read1", "acquire", "ReadsY1, Read1"),
        attempt("Sy", "w", "Cell::set", "release", "IsY, StoreOne"),
        attempt("Lx", "c", "Core::read2", "acquire", "ReadsX2, Read2"),
    );
    sb_unit(&run)
}

/// FR-219-AC-2 (TC-664 step 2): over `SB` with source default `sc`, no
/// `memory` member resolves `SB::Both` to `sc` and a `tso` entry resolves it,
/// and its nested `parallel`, to `tso`. An entry naming the nested
/// `parallel`, or a control that is not a `parallel` (the attempt `Sx`),
/// refuses `invalid_runtime_input`/`invalid-value` naming the entry; two
/// entries naming `SB::Both` refuse naming both.
#[trace("TC-664", "FR-219-AC-2")]
#[test]
fn the_request_selects_the_model_of_an_outermost_parallel() {
    let case = Case::new(&nested_unit(""));
    let protocol = case.checked.graph().protocol("SB").expect("SB");
    let models = |selection: &[MemorySelection]| {
        let resolved = case.subject(selection).memory;
        let governing = resolved.governing(protocol).expect("governs");
        let mut by_name: Vec<(String, MemoryModelName)> = protocol
            .controls
            .controls
            .iter()
            .filter_map(|control| {
                governing
                    .get(&control.node)
                    .map(|model| (control.name.clone(), *model))
            })
            .collect();
        by_name.sort();
        (resolved.model_of("SB::Both"), by_name)
    };
    let both = |model| vec![("Both".to_owned(), model), ("Inner".to_owned(), model)];
    assert_eq!(
        models(&[]),
        (Some(MemoryModelName::Sc), both(MemoryModelName::Sc))
    );
    assert_eq!(
        models(&[select("SB::Both", MemoryModelName::Tso)]),
        (Some(MemoryModelName::Tso), both(MemoryModelName::Tso))
    );

    let refused = |selection: &[MemorySelection]| {
        qsl_eval::simulation::protocol::ResolvedMemory::resolve(protocol, selection)
            .expect_err("the selection refuses")
    };
    let nested = refused(&[select("SB::Both::left::L::Inner", MemoryModelName::Tso)]);
    assert_eq!(
        nested,
        MemorySelectionRefusal::Nested {
            entry: 0,
            parallel: "SB::Both::left::L::Inner".to_owned()
        }
    );
    let attempt = refused(&[select("SB::Both::left::L::Sx", MemoryModelName::Tso)]);
    assert_eq!(
        attempt,
        MemorySelectionRefusal::NotAParallel {
            entry: 0,
            parallel: "SB::Both::left::L::Sx".to_owned()
        }
    );
    for second in [MemoryModelName::Ra, MemoryModelName::Tso] {
        assert_eq!(
            refused(&[
                select("SB::Both", MemoryModelName::Tso),
                select("SB::Both", second)
            ]),
            MemorySelectionRefusal::Duplicate {
                first: 0,
                second: 1,
                parallel: "SB::Both".to_owned()
            }
        );
    }
    for refusal in [nested, attempt] {
        assert_eq!(
            (refusal.code().as_str(), refusal.cause()),
            ("invalid_runtime_input", "invalid-value")
        );
    }
}

/// TC-664 steps 3 and 5 (FR-219-AC-3, AC-5, `sc` and `tso`; `ra` is not
/// stepped): the `sc` and `tso` subjects of `SB` resolve different models,
/// so their obligations differ, and settle `proved` and `refuted`. A defaulted
/// `sc` is written as explicitly as a selected one, and `SB` with source
/// `memory tso` and no member resolves as `SB` with a `tso` selection.
#[trace("TC-664")]
#[test]
fn the_resolved_model_is_the_identity_and_not_the_source_clause() {
    let sc = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let tso = Case::new(&sb_unit(&sb_run("memory tso", "release", "acquire")));
    let defaulted = sc.subject(&[]).memory;
    assert_eq!(
        defaulted,
        sc.subject(&[select("SB::Both", MemoryModelName::Sc)])
            .memory
    );
    assert_eq!(defaulted.model_of("SB::Both"), Some(MemoryModelName::Sc));
    let selected_tso = sc
        .subject(&[select("SB::Both", MemoryModelName::Tso)])
        .memory;
    assert_ne!(defaulted, selected_tso);
    assert_eq!(tso.subject(&[]).memory, selected_tso);

    assert!(matches!(
        sc.claim(&[], None),
        ModelCheckOutcome::Holds { .. }
    ));
    assert!(matches!(
        sc.claim(&[select("SB::Both", MemoryModelName::Tso)], None),
        ModelCheckOutcome::Violated { .. }
    ));
}

/// A protocol whose `parallel` has no `memory` clause and joins `any`,
/// cancelling the other branch.
fn any_join_unit() -> String {
    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} }} branch right sequence R {{ {} }} }} \
         join any [left, right] outstanding cancel;",
        attempt("Sx", "w", "Cell::set", "", "IsX, StoreOne"),
        attempt("Sy", "w", "Cell::set", "", "IsY, StoreOne"),
    );
    contracts() + &protocol("SB", &run)
}

/// FR-219-AC-4 (TC-664 step 4) and FR-220-AC-5 (TC-665 step 5): a `tso`
/// selection for a `parallel` with no `memory` clause that joins `any`
/// settles V-8 `WeakAccessShape{shape: JoinPolicy}` before any state is
/// explored, written `{"type":"join-policy"}`, and the same request with no
/// `memory` member checks under `sc`. An attempt writing two fields of its
/// receiver, both shared, settles `MultiLocation` under a `tso` selection,
/// naming both locations, and checks under `sc`.
#[trace("TC-664", "FR-219-AC-4")]
#[trace("TC-665", "FR-220-AC-5")]
#[test]
fn a_weak_selection_settles_the_access_shapes_before_exploring() {
    let any = Case::new(&any_join_unit());
    let selected = any.check(
        &[select("SB::Both", MemoryModelName::Tso)],
        ModelCheckItem::DeadlockFreedom,
        None,
    );
    let ModelCheckOutcome::Memory {
        cause: MemoryCause::WeakAccessShape(shape),
        stats,
    } = &selected
    else {
        panic!("{selected:?}");
    };
    assert_eq!(stats.product_states, 0);
    assert_eq!(shape.shape, AccessShape::JoinPolicy);
    assert_eq!(shape.site, "Both");
    assert_eq!(shape.transition, None);
    assert_eq!(
        shape.shape_text().expect("text"),
        r#"{"type":"join-policy"}"#
    );
    let record = any.settle(
        &[select("SB::Both", MemoryModelName::Tso)],
        &selected,
        true,
        None,
    );
    assert_eq!(record.label, ResultLabel::Unsupported);
    assert_eq!(
        record.value,
        TerminalValue::Unsupported(UnavailabilityCause::WeakAccessShape {
            site: "Both".to_owned(),
            transition: None,
            shape: r#"{"type":"join-policy"}"#.to_owned(),
        })
    );
    assert!(
        matches!(
            any.check(&[], ModelCheckItem::DeadlockFreedom, None),
            ModelCheckOutcome::Holds { .. }
        ),
        "the any join checks under sc"
    );

    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} }} \
         branch right sequence R {{ {} {} }} }} join all [left, right];",
        attempt("Bx", "w", "Cell::both", "", "BothX, BothOne"),
        attempt("Lx", "c", "Core::read2", "", "ReadsX2, Read2"),
        attempt("Lw", "c", "Core::read3", "", "ReadsX3, ReadW"),
    );
    let two = Case::new(&(contracts() + &protocol("SB", &run)));
    let outcome = two.check(
        &[select("SB::Both", MemoryModelName::Tso)],
        ModelCheckItem::DeadlockFreedom,
        None,
    );
    let ModelCheckOutcome::Memory {
        cause: MemoryCause::WeakAccessShape(shape),
        ..
    } = &outcome
    else {
        panic!("{outcome:?}");
    };
    assert_eq!(shape.site, "Both/left/L/Bx");
    assert!(shape.transition.is_some());
    let AccessShape::MultiLocation { locations } = &shape.shape else {
        panic!("{shape:?}");
    };
    let accesses = two.accesses(&[select("SB::Both", MemoryModelName::Tso)]);
    let names: BTreeSet<(String, String)> = locations
        .iter()
        .map(|location| named(&accesses, location))
        .collect();
    let receiver = names
        .iter()
        .next()
        .map(|(key, _)| key.clone())
        .expect("a location");
    assert_eq!(
        names,
        BTreeSet::from([name(&receiver, "v"), name(&receiver, "w")])
    );
    let text: Json = serde_json::from_str(&shape.shape_text().expect("text")).expect("JSON");
    assert_eq!(text["type"], json!("multi-location"));
    let fields: BTreeSet<&str> = text["locations"]
        .as_array()
        .expect("locations")
        .iter()
        .map(|location| {
            assert_eq!(location["type"], json!("location"));
            assert!(location["object"].is_object());
            location["field"].as_str().expect("a field")
        })
        .collect();
    assert_eq!(fields, BTreeSet::from(["v", "w"]));
    assert!(matches!(
        two.check(&[], ModelCheckItem::DeadlockFreedom, None),
        ModelCheckOutcome::Holds { .. }
    ));
}

/// FR-220-AC-11 (TC-665 step 10): EN-1 classifies `SB` under `tso` and
/// under `ra` with `Sx` and `Sy` as stores, `Ly` and `Lx` as loads, the
/// cells' `v` shared and the core's `r1` and `r2` branch-local.
#[trace("TC-665", "FR-220-AC-11")]
#[test]
fn en1_classifies_store_buffering_into_stores_loads_and_registers() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    for model in [MemoryModelName::Tso, MemoryModelName::Ra] {
        let accesses = case.accesses(&[select("SB::Both", model)]);
        let shared: BTreeSet<(String, String)> = accesses
            .shared()
            .iter()
            .map(|location| named(&accesses, location))
            .collect();
        assert_eq!(shared, BTreeSet::from([name("x", "v"), name("y", "v")]));
        for (node, class) in [
            ("Both/left/L/Sx", AccessClass::Store),
            ("Both/left/L/Ly", AccessClass::Load),
            ("Both/right/R/Sy", AccessClass::Store),
            ("Both/right/R/Lx", AccessClass::Load),
        ] {
            let found = classes(&accesses, node);
            assert!(
                found.iter().all(|(found, _)| *found == class),
                "{node}: {found:?}"
            );
        }
        assert!(local(&accesses, "Both/left/L/Ly").contains(&name("core", "r1")));
        assert!(local(&accesses, "Both/right/R/Lx").contains(&name("core", "r2")));
        let ordering = |node| accesses.attempt(node).expect("weak").ordering;
        assert_eq!(
            ordering("Both/left/L/Sx"),
            Some(qsl_semantics::check::AccessOrdering::Release)
        );
    }
    // Under `sc` no attempt is classified.
    assert_eq!(case.accesses(&[]).attempts().count(), 0);
}

/// FR-220-AC-12 (TC-665 step 11): two branches each storing `relaxed` to
/// `x`, with no reader, are two stores to one shared location.
#[trace("TC-665", "FR-220-AC-12")]
#[test]
fn en1_classifies_two_relaxed_stores_to_one_shared_location() {
    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} }} branch right sequence R {{ {} }} }} \
         join all [left, right];",
        attempt("S1", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        attempt("S2", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
    );
    let case = Case::new(&(contracts() + &protocol("SB", &run)));
    for model in [MemoryModelName::Tso, MemoryModelName::Ra] {
        let accesses = case.accesses(&[select("SB::Both", model)]);
        for node in ["Both/left/L/S1", "Both/right/R/S2"] {
            assert!(classes(&accesses, node).contains(&(AccessClass::Store, name("x", "v"))));
        }
        assert!(accesses
            .shared()
            .iter()
            .any(|location| named(&accesses, location) == name("x", "v")));
    }
}

/// FR-220-AC-13 (TC-665 step 12): a branch that writes the core's `r1` and
/// then reads it again has `r1` branch-local, and its reread is local.
#[trace("TC-665", "FR-220-AC-13")]
#[test]
fn en1_keeps_a_register_read_again_by_its_own_branch_local() {
    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} {} }} branch right sequence R {{ {} {} }} }} \
         join all [left, right];",
        attempt("Ly", "c", "Core::read1", "acquire", "ReadsY1, Read1"),
        attempt("Cp", "c", "Core::copy", "", "CopyR"),
        attempt("Sy", "w", "Cell::set", "release", "IsY, StoreOne"),
        attempt("Lx", "c", "Core::read2", "acquire", "ReadsX2, Read2"),
    );
    let case = Case::new(&(contracts() + &protocol("SB", &run)));
    let accesses = case.accesses(&[select("SB::Both", MemoryModelName::Tso)]);
    let shared: BTreeSet<(String, String)> = accesses
        .shared()
        .iter()
        .map(|location| named(&accesses, location))
        .collect();
    assert!(!shared.contains(&name("core", "r1")));
    assert!(!shared.contains(&name("core", "r3")));
    let copy = accesses.attempt("Both/left/L/Cp").expect("weak");
    assert!(copy.instances.iter().all(|instance| instance.access
        == InstanceAccess::Access {
            class: AccessClass::Local,
            location: None
        }));
    assert!(local(&accesses, "Both/left/L/Cp").contains(&name("core", "r1")));
}

/// FR-220-AC-4 (TC-665 step 4): `SB` under `sc` with every ordering
/// `relaxed`, and with every ordering `seq_cst`, reaches the same states,
/// 13 inside the `parallel` and 3 joined, and settles `proved` both times.
#[trace("TC-665", "FR-220-AC-4")]
#[test]
fn orderings_change_nothing_under_sc() {
    let mut keys = Vec::new();
    for ordering in ["relaxed", "seq_cst"] {
        let case = Case::new(&sb_unit(&sb_run("", ordering, ordering)));
        let system = case.system(&[], Sc).expect("builds");
        let reached = reachable(&system);
        assert_eq!(counts(&system, &reached), (13, 3));
        let mut states: Vec<Vec<u8>> = reached
            .iter()
            .map(|state| system.state_key(&state.state).expect("key").0)
            .collect();
        states.sort();
        keys.push(states);
        assert!(matches!(
            case.claim(&[], None),
            ModelCheckOutcome::Holds { .. }
        ));
    }
    assert_eq!(keys[0], keys[1]);
}

/// The memory member of `state`'s key.
fn memory_member<M: MemoryModel>(system: &ProtocolSystem<'_, M>, state: &ProtocolState<M>) -> Json {
    let (key, _) = system.state_key(state).expect("a key");
    serde_json::from_slice::<Json>(&key).expect("JSON")["memory"].clone()
}

/// Each buffer of `state`, by branch label, as `(object key, value)` pairs.
fn buffers(state: &ProtocolState<Tso>) -> BTreeMap<String, Vec<(String, i64)>> {
    state
        .memory()
        .threads()
        .map(|thread| {
            let label = thread
                .steps
                .last()
                .map(|step| step.branch.clone())
                .unwrap_or_default();
            let entries = state
                .memory()
                .buffer(thread)
                .unwrap_or_default()
                .iter()
                .map(|entry| {
                    let value = match &entry.value {
                        Some(quire_exact::Value::Integer(value)) => {
                            value.to_string().parse().expect("small")
                        }
                        other => panic!("{other:?}"),
                    };
                    (entry.cell.key.clone(), value)
                })
                .collect();
            (label, entries)
        })
        .collect()
}

/// TC-666 step 1 (FR-221-AC-1 but the step order): `SB` under `tso`
/// reaches 34 states inside the `parallel` and 4 joined states, and the
/// claim settles `refuted` with a counterexample of the fork and the four
/// attempts: at its last position memory holds `x = y = 0`, `left`'s buffer
/// `[x := 1]`, `right`'s `[y := 1]`, and both registers 0. FR-181's
/// canonical order puts `Sy` (no argument) before `Ly`, so the first
/// violation is `Sx`, `Sy`, `Lx`, `Ly`, not §10's `Sx`, `Ly`, `Sy`, `Lx`.
#[trace("TC-666")]
#[test]
fn store_buffering_under_tso_reaches_38_states_and_is_refuted() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    let system = case.system(&tso, Tso::default()).expect("builds");
    let reached = reachable(&system);
    assert_eq!(counts(&system, &reached), (34, 4));

    let outcome = case.claim(&tso, None);
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    // The first step forks; the other four are the four attempts, each
    // once, in FR-181's canonical transition order.
    let steps: Vec<(StepKind, String)> = counterexample
        .prefix
        .iter()
        .map(|step| {
            (
                step.transition.kind(),
                step.transition.node().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    assert_eq!(steps.first(), Some(&(StepKind::Fork, "Both".to_owned())));
    let attempts: BTreeSet<&(StepKind, String)> = steps.iter().skip(1).collect();
    let attempt = |node: &str| (StepKind::Attempt, node.to_owned());
    let expected = [
        attempt("Both/left/L/Sx"),
        attempt("Both/left/L/Ly"),
        attempt("Both/right/R/Sy"),
        attempt("Both/right/R/Lx"),
    ];
    assert_eq!(steps.len(), 5);
    assert_eq!(attempts, expected.iter().collect());
    // Walk the counterexample's steps through the system to its last state.
    let mut state = system.initial().remove(0);
    for step in &counterexample.prefix {
        state = system
            .successors(&state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| *transition == step.transition)
            .map(|(_, next)| next)
            .expect("the step is enabled");
    }
    let model = state.model();
    assert_eq!(
        (
            field(model, "cells", "x", "v"),
            field(model, "cells", "y", "v")
        ),
        (0, 0)
    );
    assert_eq!(
        (
            field(model, "cores", "core", "r1"),
            field(model, "cores", "core", "r2")
        ),
        (0, 0)
    );
    assert_eq!(
        buffers(&state),
        BTreeMap::from([
            ("left".to_owned(), vec![("x".to_owned(), 1)]),
            ("right".to_owned(), vec![("y".to_owned(), 1)]),
        ])
    );
}

/// `SB` with a `fence` of `left` and of `right` between each store and its
/// load.
fn fenced_sb(left: &str, right: &str) -> String {
    format!(
        "parallel Both {{ \
         branch left sequence L {{ {} fence Fl ordering {left}; {} }} \
         branch right sequence R {{ {} fence Fr ordering {right}; {} }} }} join all [left, right];",
        attempt("Sx", "w", "Cell::set", "release", "IsX, StoreOne"),
        attempt("Ly", "c", "Core::read1", "acquire", "ReadsY1, Read1"),
        attempt("Sy", "w", "Cell::set", "release", "IsY, StoreOne"),
        attempt("Lx", "c", "Core::read2", "acquire", "ReadsX2, Read2"),
    )
}

/// FR-221-AC-2 (TC-666): `SB` with `fence F ordering seq_cst;` between each
/// store and load settles `proved` under `tso`, and so does `SB` with every
/// store and load `seq_cst` and no fence.
#[trace("TC-666", "FR-221-AC-2")]
#[test]
fn seq_cst_fences_or_accesses_forbid_store_buffering_under_tso() {
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    for run in [
        fenced_sb("seq_cst", "seq_cst"),
        sb_run("", "seq_cst", "seq_cst"),
    ] {
        let case = Case::new(&sb_unit(&run));
        let outcome = case.claim(&tso, None);
        assert!(
            matches!(outcome, ModelCheckOutcome::Holds { .. }),
            "{run}: {outcome:?}"
        );
        let record = case.settle(&tso, &outcome, false, None);
        assert_eq!(record.label, ResultLabel::Proved, "{record:?}");
    }
}

/// ADR-025 TSO-6 and MR-1 (ADR-027 SE-5): under `tso` a `seq_cst` fence is
/// enabled only once its thread's buffer is empty, so after `left`'s store
/// its fence waits, with cause `Gate`, until `flush(left)`; an `acquire`
/// fence is enabled with a store buffered. The `seq_cst` fence's footprint
/// reads and is enabled by `buf(left)`; the `acquire` fence's names no
/// buffer. This is FR-206-AC-8's `tso` half.
#[trace("TC-666", "TC-651", "FR-206-AC-8")]
#[test]
fn a_seq_cst_fence_waits_for_its_store_buffer_under_tso() {
    let case = Case::new(&sb_unit(&fenced_sb("seq_cst", "acquire")));
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    let system = case.system(&tso, Tso::default()).expect("builds");
    let reached = reachable(&system);
    let at_fence = |state: &Reached<Tso>, label: &str, node: &str| {
        system
            .threads(&state.state)
            .expect("threads")
            .into_iter()
            .find(|thread| {
                thread
                    .path
                    .steps
                    .first()
                    .is_some_and(|step| step.branch == label)
                    && thread
                        .rest_point
                        .node
                        .as_deref()
                        .is_some_and(|at| at.ends_with(node))
            })
            .map(|thread| thread.path)
    };
    let fence_step = |state: &Reached<Tso>, path: &ThreadPath| -> Option<ProtocolTransition> {
        state
            .steps
            .iter()
            .find(|(transition, _)| {
                transition.kind() == StepKind::Fence && transition.thread() == Some(path)
            })
            .map(|(transition, _)| transition.clone())
    };
    let (mut waited, mut passed, mut buffered_acquire) = (0, 0, 0);
    for state in &reached {
        if let Some(left) = at_fence(state, "left", "/Fl") {
            let buffered = !state
                .state
                .memory()
                .buffer(&left)
                .unwrap_or_default()
                .is_empty();
            match fence_step(state, &left) {
                Some(fence) => {
                    assert!(!buffered, "a seq_cst fence waits for its buffer");
                    let footprint = system.footprint(&state.state, &fence).expect("a footprint");
                    let own = Location::Protocol(ProtocolLocation::Buffer(left.clone()));
                    assert!(footprint.reads.contains(&own) && footprint.enabling.contains(&own));
                    passed += 1;
                }
                None => {
                    assert!(buffered, "an unbuffered seq_cst fence is enabled");
                    let blocked = system.blocked(&state.state, &[]).expect("blocked");
                    assert!(blocked
                        .iter()
                        .filter(|thread| thread.path == left)
                        .all(|thread| thread.cause == WaitCause::Gate));
                    // `flush(left)` drains the one store, and the fence is
                    // then enabled.
                    let (_, flushed) = state
                        .steps
                        .iter()
                        .find(|(transition, _)| {
                            transition
                                .memory_step()
                                .is_some_and(|step| step.thread == left)
                        })
                        .expect("a flush of left's buffer");
                    let after = &reached[*flushed];
                    assert!(after
                        .state
                        .memory()
                        .buffer(&left)
                        .unwrap_or_default()
                        .is_empty());
                    assert!(fence_step(after, &left).is_some());
                    waited += 1;
                }
            }
        }
        if let Some(right) = at_fence(state, "right", "/Fr") {
            let fence = fence_step(state, &right).expect("an acquire fence passes");
            let footprint = system.footprint(&state.state, &fence).expect("a footprint");
            assert!(!footprint.reads.iter().any(|location| matches!(
                location,
                Location::Protocol(ProtocolLocation::Buffer(_))
            )));
            if !state
                .state
                .memory()
                .buffer(&right)
                .unwrap_or_default()
                .is_empty()
            {
                buffered_acquire += 1;
            }
        }
    }
    assert!(waited > 0 && passed > 0 && buffered_acquire > 0);
}

/// FR-221-AC-3 (TC-666): a branch storing `x := 1` then `y := 1`, beside a
/// branch that loads `x` and then `y`: after both stores the storing
/// thread's buffer is `[x := 1, y := 1]` and the first `flush` writes `x`
/// only; that thread's load of `x` before any flush reads 1 and the other
/// thread's reads 0; its `fetch_add` is not enabled until both flushes.
#[trace("TC-666", "FR-221-AC-3")]
#[test]
fn a_store_buffer_is_fifo_and_a_locked_access_waits_for_it() {
    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} {} {} {} }} \
         branch right sequence R {{ {} {} }} }} join all [left, right];",
        attempt("S1", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        attempt("S2", "w", "Cell::set", "relaxed", "IsY, StoreOne"),
        attempt("L3", "c", "Core::read3", "relaxed", "ReadsX3, Read3"),
        attempt("Ad", "w", "Cell::add", "acq_rel", "AddX, AddOne"),
        attempt("Lx", "c", "Core::read2", "relaxed", "ReadsX2, Read2"),
        attempt("Ly", "c", "Core::read1", "relaxed", "ReadsY1, Read1"),
    );
    let case = Case::new(&(contracts() + &protocol("SB", &run)));
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    let accesses = case.accesses(&tso);
    assert!(classes(&accesses, "Both/left/L/Ad").contains(&(AccessClass::Rmw, name("x", "v"))));
    let system = case.system(&tso, Tso::default()).expect("builds");
    let next = |state: &ProtocolState<Tso>, node: &str| {
        system
            .successors(state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| transition.node() == Some(node))
            .map(|(_, next)| next)
    };
    let flush = |state: &ProtocolState<Tso>| {
        system
            .successors(state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| transition.memory_step().is_some())
            .map(|(_, next)| next)
            .expect("a flush")
    };
    let forked = next(&system.initial()[0], "Both").expect("fork");
    let stored = next(
        &next(&forked, "Both/left/L/S1").expect("S1"),
        "Both/left/L/S2",
    )
    .expect("S2");
    assert_eq!(
        buffers(&stored)["left"],
        vec![("x".to_owned(), 1), ("y".to_owned(), 1)]
    );
    let flushed = flush(&stored);
    assert_eq!(
        (
            field(flushed.model(), "cells", "x", "v"),
            field(flushed.model(), "cells", "y", "v")
        ),
        (1, 0)
    );
    let own = next(&stored, "Both/left/L/L3").expect("L3");
    assert_eq!(field(own.model(), "cores", "core", "r3"), 1);
    let other = next(&stored, "Both/right/R/Lx").expect("Lx");
    assert_eq!(field(other.model(), "cores", "core", "r2"), 0);
    // At `Ad` with the buffer full, the locked access waits for both flushes.
    assert!(next(&own, "Both/left/L/Ad").is_none());
    let once = flush(&own);
    assert!(next(&once, "Both/left/L/Ad").is_none());
    let twice = flush(&once);
    assert!(next(&twice, "Both/left/L/Ad").is_some());
}

/// TC-666 step 4 (FR-221-AC-4's buffer half): in `SB` under `tso`, `join`
/// is not enabled while a branch buffer is non-empty, and every joined
/// state has no buffer.
#[trace("TC-666")]
#[test]
fn the_join_waits_for_empty_buffers() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let system = case
        .system(&[select("SB::Both", MemoryModelName::Tso)], Tso::default())
        .expect("builds");
    let reached = reachable(&system);
    for state in &reached {
        let joins = state
            .steps
            .iter()
            .any(|(transition, _)| transition.kind() == StepKind::Join);
        let buffered = state.state.memory().threads().any(|thread| {
            !state
                .state
                .memory()
                .buffer(thread)
                .unwrap_or_default()
                .is_empty()
        });
        assert!(!(joins && buffered));
        let threads = system.threads(&state.state).expect("threads");
        if threads.iter().all(|thread| thread.path.steps.is_empty()) {
            assert_eq!(state.state.memory().threads().count(), 0);
        }
    }
}

/// FR-225 (TC-670 steps 2 and 3): `SB` under `tso` with `max_store_buffer`
/// 1 still settles `refuted`; two stores in one branch beside their loads,
/// with the deadlock-freedom item, settle V-6 `MemoryBoundReached` under
/// `max_store_buffer` 1, naming the member and its value, and `proved`
/// under the default 4 with the bound not reached.
#[trace("TC-670")]
#[test]
fn a_store_buffer_bound_cuts_stores_and_settles_memory_bound_reached() {
    let sb = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    assert!(matches!(
        sb.claim(&tso, Some(1)),
        ModelCheckOutcome::Violated { .. }
    ));

    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} {} }} \
         branch right sequence R {{ {} {} }} }} join all [left, right];",
        attempt("S1", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        attempt("S2", "w", "Cell::set", "relaxed", "IsY, StoreOne"),
        attempt("Lx", "c", "Core::read2", "relaxed", "ReadsX2, Read2"),
        attempt("Ly", "c", "Core::read1", "relaxed", "ReadsY1, Read1"),
    );
    let two = Case::new(&(contracts() + &protocol("SB", &run) + NO_BOTH_ZERO));
    let bounded = two.check(&tso, ModelCheckItem::DeadlockFreedom, Some(1));
    let ModelCheckOutcome::Memory {
        cause: MemoryCause::BoundReached { bound, states },
        ..
    } = &bounded
    else {
        panic!("{bounded:?}");
    };
    assert_eq!(
        *bound,
        MemoryBound {
            member: MemoryBoundMember::MaxStoreBuffer,
            value: 1
        }
    );
    assert!(*states >= 1);
    let record = two.settle(&tso, &bounded, true, Some(1));
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::MemoryBoundReached {
            bound: *bound,
            states: *states,
        })
    );
    assert!(matches!(
        two.check(&tso, ModelCheckItem::DeadlockFreedom, None),
        ModelCheckOutcome::Holds { .. }
    ));
}

/// FR-229-AC-5 (TC-674 step 5): every terminal state of `SB` under `tso`
/// has empty buffers, and its deadlock-freedom item settles `proved`.
#[trace("TC-674", "FR-229-AC-5")]
#[test]
fn every_terminal_state_under_tso_has_empty_buffers() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    let system = case.system(&tso, Tso::default()).expect("builds");
    let reached = reachable(&system);
    let terminal: Vec<&Reached<Tso>> = reached
        .iter()
        .filter(|state| state.steps.is_empty())
        .collect();
    assert!(!terminal.is_empty());
    for state in terminal {
        assert!(state.state.memory().threads().all(|thread| state
            .state
            .memory()
            .buffer(thread)
            .unwrap_or_default()
            .is_empty()));
    }
    assert!(matches!(
        case.check(&tso, ModelCheckItem::DeadlockFreedom, None),
        ModelCheckOutcome::Holds { .. }
    ));
}

/// TC-674 steps 1 and 2 (FR-229-AC-1 and AC-2, `sc` and `tso`): under
/// `tso` the memory member after `Sx` maps `left`'s store buffer to
/// `[x := 1]` and `right`'s to the empty sequence, and is empty in every
/// joined state; under `sc` it is empty at every state. After `Sx` alone
/// memory still holds `x = 0`, and `flush(left)` writes it.
#[trace("TC-674")]
#[test]
fn the_memory_member_holds_the_buffers_while_the_parallel_is_active() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    let system = case.system(&tso, Tso::default()).expect("builds");
    let next = |state: &ProtocolState<Tso>, pick: &dyn Fn(&ProtocolTransition) -> bool| {
        system
            .successors(state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| pick(transition))
            .map(|(_, next)| next)
            .expect("enabled")
    };
    let forked = next(&system.initial()[0], &|transition| {
        transition.kind() == StepKind::Fork
    });
    let stored = next(&forked, &|transition| {
        transition.node() == Some("Both/left/L/Sx")
    });
    let member = memory_member(&system, &stored);
    assert_eq!(member["type"], json!("map"));
    let entries = member["entries"].as_array().expect("entries");
    assert_eq!(entries.len(), 2);
    let mut lengths = BTreeMap::new();
    for entry in entries {
        let key = &entry["key"];
        assert_eq!(key["type"], json!("store-buffer"));
        let label = key["thread"]["steps"][0]["branch"]
            .as_str()
            .expect("a branch")
            .to_owned();
        let elements = entry["value"]["elements"].as_array().expect("a sequence");
        lengths.insert(label, elements.len());
    }
    assert_eq!(
        lengths,
        BTreeMap::from([("left".to_owned(), 1), ("right".to_owned(), 0)])
    );
    assert_eq!(field(stored.model(), "cells", "x", "v"), 0);
    let flushed = next(&stored, &|transition| {
        transition
            .memory_step()
            .is_some_and(|step| step.thread == branch("left") && step.step == "flush")
    });
    assert_eq!(field(flushed.model(), "cells", "x", "v"), 1);
    for state in reachable(&system) {
        let threads = system.threads(&state.state).expect("threads");
        if threads.iter().all(|thread| thread.path.steps.is_empty()) {
            assert_eq!(
                memory_member(&system, &state.state),
                json!({"entries": [], "type": "map"})
            );
        }
    }
    let sc = case.system(&[], Sc).expect("builds");
    for state in reachable(&sc) {
        assert_eq!(
            memory_member(&sc, &state.state),
            json!({"entries": [], "type": "map"})
        );
    }
}

/// `always holds(not (c.v = 1))` over every cell.
const NEVER_ONE: &str = "temporal NeverOne using t0 over (c: M::Cell) clock \"model-steps\" \
     on origin { always holds(not (c.v = 1)) }\n";

/// [`NEVER_ONE`]'s atom, read from the bound cell at the state the engine
/// and replay hand the letters: the atom values (ADR-025 MS-2).
struct NeverOne;

impl ModelLetters for NeverOne {
    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        Ok(if field(state, "cells", &instance.key, "v") == 1 {
            AtomReading::False
        } else {
            AtomReading::True
        })
    }
}

/// `always holds(not (k.r1 = 0))`: `left`'s register never reads 0.
const R1_NEVER_ZERO: &str = "temporal R1NeverZero using t0 over (k: M::Core) clock \
     \"model-steps\" on origin { always holds(not (k.r1 = 0)) }\n";

/// [`R1_NEVER_ZERO`]'s atom, read from the bound core's register.
struct R1NeverZero;

impl ModelLetters for R1NeverZero {
    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        Ok(if field(state, "cores", &instance.key, "r1") == 0 {
            AtomReading::False
        } else {
            AtomReading::True
        })
    }
}

/// FR-229-AC-2 (TC-674 step 2): under `tso`, the claims' `holds` atoms
/// are evaluated by EN-1 and replay at every position. `holds(x.v = 1)`
/// is false after a store alone and true only after its flush: the
/// shortest counterexample to `always holds(not (c.v = 1))` is a fork, a
/// store and the flush, whose position carries the `memory` anchor naming
/// the step, and it replays. `holds(c1.r = 0)` reads `left`'s register:
/// `always holds(not (k.r1 = 0))` is refuted right after `Ly`. A store's
/// position carries its operation's anchor and the fork's the `protocol`
/// anchor naming its node. A load of `x` by `left` whose precondition
/// requires `x.v = 1` is enabled after `Sx`, since `left` sees its own
/// buffered store; the same load by `right` is not, until `flush(left)`.
#[trace("TC-674", "FR-229-AC-2")]
#[test]
fn a_flush_is_a_position_with_a_memory_anchor_and_atoms_read_memory() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    let system = case.system(&tso, Tso::default()).expect("builds");
    let step = |state: &ProtocolState<Tso>, pick: &dyn Fn(&ProtocolTransition) -> bool| {
        system
            .successors(state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| pick(transition))
    };
    let (fork, forked) = step(&system.initial()[0], &|transition| {
        transition.kind() == StepKind::Fork
    })
    .expect("the fork");
    assert_eq!(
        fork.anchor(),
        PositionAnchor::Protocol {
            step: StepKind::Fork,
            node: Some("Both"),
            channel: None,
        }
    );
    let (sx, _) = step(&forked, &|transition| {
        transition.node() == Some("Both/left/L/Sx")
    })
    .expect("Sx");
    assert!(
        matches!(sx.anchor(), PositionAnchor::Operation(operation)
            if operation.contains("\"ix://test/litmus/Cell/set\"")),
        "{:?}",
        sx.anchor()
    );
    // The claims' `holds` atoms, evaluated by EN-1 and replayed by FR-217
    // at every position of `SB` under `tso`: `holds(c.v = 1)` first holds
    // after `flush(left)` or `flush(right)`, never right after a store, so
    // the shortest counterexample to `always holds(not (c.v = 1))` ends
    // with the flush, whose position carries the `memory` anchor.
    let run = sb_run("", "release", "acquire");
    let never_one = Case::with_letters(
        &(contracts() + &protocol("SB", &run) + NEVER_ONE),
        &NeverOne,
    );
    let outcome = never_one.claim(&tso, None);
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("a flush makes c.v = 1: {outcome:?}");
    };
    let kinds: Vec<StepKind> = counterexample
        .prefix
        .iter()
        .map(|step| step.transition.kind())
        .collect();
    assert_eq!(
        kinds,
        [StepKind::Fork, StepKind::Attempt, StepKind::Memory],
        "{counterexample:?}"
    );
    let flush = &counterexample.prefix[2].transition;
    let stored = &counterexample.prefix[1].transition;
    let PositionAnchor::Memory(anchor) = flush.anchor() else {
        panic!("{:?}", flush.anchor());
    };
    let thread = stored.thread().expect("a store's thread").clone();
    assert_eq!(
        (anchor.model, anchor.step, &anchor.thread),
        (MemoryModelName::Tso, "flush", &thread)
    );
    let record = never_one.settle(&tso, &outcome, false, None);
    assert_eq!(record.label, ResultLabel::Refuted, "{record:?}");
    // `holds(c1.r = 0)` reads `left`'s register: `always holds(not
    // (k.r1 = 0))` is refuted right after `left`'s `Ly` (after its `Sx`),
    // which loads `y`'s 0 into the register that starts at 3.
    let register = Case::with_letters(
        &(contracts() + &protocol("SB", &run) + R1_NEVER_ZERO),
        &R1NeverZero,
    );
    let outcome = register.claim(&tso, None);
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("Ly reads 0: {outcome:?}");
    };
    let nodes: Vec<Option<&str>> = counterexample
        .prefix
        .iter()
        .map(|step| step.transition.node())
        .collect();
    assert_eq!(
        nodes,
        [Some("Both"), Some("Both/left/L/Sx"), Some("Both/left/L/Ly")]
    );
    let record = register.settle(&tso, &outcome, false, None);
    assert_eq!(record.label, ResultLabel::Refuted, "{record:?}");

    let sees = contracts()
        + "pre SeesX3 using v on M::Core::read3 { c.v = 1 }\n\
           pre SeesX4 using v on M::Core::read4 { c.v = 1 }\n";
    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} {} }} \
         branch right sequence R {{ {} }} }} join all [left, right];",
        store("Sx", "x", "StoreOne"),
        attempt(
            "L3",
            "c",
            "Core::read3",
            "relaxed",
            "ReadsX3, SeesX3, Read3"
        ),
        attempt(
            "L4",
            "c",
            "Core::read4",
            "relaxed",
            "ReadsX4, SeesX4, Read4"
        ),
    );
    let gated = Case::new(&(sees + &protocol("SB", &run)));
    let system = gated.system(&tso, Tso::default()).expect("builds");
    let step = |state: &ProtocolState<Tso>, pick: &dyn Fn(&ProtocolTransition) -> bool| {
        system
            .successors(state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| pick(transition))
    };
    let node =
        |name: &'static str| move |transition: &ProtocolTransition| transition.node() == Some(name);
    let (_, forked) = step(&system.initial()[0], &|transition| {
        transition.kind() == StepKind::Fork
    })
    .expect("the fork");
    assert!(step(&forked, &node("Both/right/R/L4")).is_none());
    let (_, stored) = step(&forked, &node("Both/left/L/Sx")).expect("Sx");
    assert!(step(&stored, &node("Both/left/L/L3")).is_some());
    assert!(step(&stored, &node("Both/right/R/L4")).is_none());
    let (_, flushed) =
        step(&stored, &|transition| transition.memory_step().is_some()).expect("flush(left)");
    assert!(step(&flushed, &node("Both/right/R/L4")).is_some());
}

/// TC-672 step 1 (FR-227-AC-1 but §10's step order, SR-12240 FND-008):
/// `SB`'s `tso` counterexample's subject names `SB::Both` resolved to
/// `tso`, and every step carries its post-state's memory component, both
/// buffers, equal to the component of the state the step reaches: empty
/// buffers after the fork and `left: [x := 1]`, `right: [y := 1]` at the
/// last position. It replays to `reproduced-with-evaluated-witness`; a
/// carried component that differs from the recomputed one refuses
/// `stale_dependency`/`content-mismatch` naming its step, and so do the
/// steps replayed under `sc`, at the fork, the first step whose post-state
/// differs. Over `sc` no step carries a component.
#[trace("TC-672")]
#[test]
fn a_tso_counterexample_replays_only_under_tso() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    assert_eq!(
        case.trace_subject(&tso).memory.model_of("SB::Both"),
        Some(MemoryModelName::Tso)
    );
    let outcome = case.claim(&tso, None);
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("{outcome:?}");
    };
    // Each carried component is the memory member of the state its step
    // reaches.
    let system = case.system(&tso, Tso::default()).expect("builds");
    let mut state = system.initial().remove(0);
    let mut carried = Vec::new();
    for step in &counterexample.prefix {
        state = system
            .successors(&state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| *transition == step.transition)
            .map(|(_, next)| next)
            .expect("the step is enabled");
        let component = step
            .transition
            .memory_component()
            .expect("a step over a weak parallel carries its memory component");
        let text: Json = serde_json::from_str(component.as_str()).expect("JSON");
        assert_eq!(text, memory_member(&system, &state));
        carried.push(buffers(&state));
    }
    let empty = BTreeMap::from([("left".to_owned(), vec![]), ("right".to_owned(), vec![])]);
    assert_eq!(carried.first(), Some(&empty));
    assert_eq!(
        carried.last(),
        Some(&BTreeMap::from([
            ("left".to_owned(), vec![("x".to_owned(), 1)]),
            ("right".to_owned(), vec![("y".to_owned(), 1)]),
        ]))
    );
    // A carried component that is not the recomputed one is refused.
    let mut forged = counterexample.as_ref().clone();
    let first = forged.prefix[0].transition.memory_component().cloned();
    forged.prefix[2].transition = forged.prefix[2].transition.clone().carrying_memory(first);
    let Err(ReplayRefusal::ModelTrace(refusal)) = case.replay(&tso, forged) else {
        panic!("a forged memory component refuses");
    };
    assert!(
        matches!(*refusal, ModelTraceRefusal::MemoryComponent { step: 2, .. }),
        "{refusal:?}"
    );
    assert_eq!(
        (refusal.code().as_str(), refusal.cause()),
        ("stale_dependency", "content-mismatch")
    );
    let mut missing = counterexample.as_ref().clone();
    missing.prefix[1].transition = missing.prefix[1].transition.clone().carrying_memory(None);
    let Err(ReplayRefusal::ModelTrace(refusal)) = case.replay(&tso, missing) else {
        panic!("a missing memory component refuses");
    };
    assert!(
        matches!(*refusal, ModelTraceRefusal::MemoryComponent { step: 1, .. }),
        "{refusal:?}"
    );
    // Over `sc` no step carries a memory component.
    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} }} \
         branch right sequence R {{ {} {} }} }} join all [left, right];",
        store("S1", "x", "StoreOne"),
        load("Lx", "x", 2),
        load("Ly", "y", 1),
    );
    let sc = Case::new(&(contracts() + &protocol("SB", &run) + NO_BOTH_ZERO));
    let ModelCheckOutcome::Violated {
        counterexample: under_sc,
        ..
    } = sc.claim(&[], None)
    else {
        panic!("both loads read 0 under sc");
    };
    assert!(under_sc
        .prefix
        .iter()
        .all(|step| step.transition.memory_component().is_none()));
    // An `sc` step that carries a component, its post-state digest right,
    // is refused on the component alone.
    let mut forged_sc = under_sc.as_ref().clone();
    forged_sc.prefix[1].transition =
        forged_sc.prefix[1]
            .transition
            .clone()
            .carrying_memory(Some(CarriedComponent::from_text(
                r#"{"entries":[],"type":"map"}"#,
            )));
    let Err(ReplayRefusal::ModelTrace(refusal)) = sc.replay(&[], forged_sc) else {
        panic!("a component on an sc step refuses");
    };
    assert!(
        matches!(
            &*refusal,
            ModelTraceRefusal::MemoryComponent {
                step: 1,
                recorded: Some(_),
                replayed: None,
            }
        ),
        "{refusal:?}"
    );
    assert!(sc.replay(&[], under_sc.as_ref().clone()).is_ok());
    let record = case.settle(
        &tso,
        &ModelCheckOutcome::Violated {
            counterexample: counterexample.clone(),
            stats: Default::default(),
        },
        false,
        None,
    );
    assert_eq!(record.label, ResultLabel::Refuted, "{record:?}");
    let replayed = case
        .replay(&tso, counterexample.as_ref().clone())
        .expect("replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
    let Err(ReplayRefusal::ModelTrace(refusal)) = case.replay(&[], counterexample.as_ref().clone())
    else {
        panic!("replaying under sc refuses");
    };
    assert!(
        matches!(*refusal, ModelTraceRefusal::PostState { step: 0, .. }),
        "{refusal:?}"
    );
    assert_eq!(
        (refusal.code().as_str(), refusal.cause()),
        ("stale_dependency", "content-mismatch"),
        "{refusal:?}"
    );
}

/// SR-12240 FND-010 (FR-225, ADR-025 MB-4; TC-670): a run that cut a store
/// and found a counterexample settles `refuted` and states its bound as
/// reached; under the default bound, which cuts nothing, the same claim's
/// record states it not reached.
#[trace("TC-670")]
#[test]
fn a_refuted_run_that_cut_a_store_states_its_bound_reached() {
    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} {} }} \
         branch right sequence R {{ {} {} }} }} join all [left, right];",
        store("S1", "x", "StoreOne"),
        store("S2", "y", "StoreOne"),
        load("Lx", "x", 2),
        load("Ly", "y", 1),
    );
    let two = Case::new(&(contracts() + &protocol("SB", &run) + NO_BOTH_ZERO));
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    let bound = |value| MemoryBound {
        member: MemoryBoundMember::MaxStoreBuffer,
        value,
    };
    for (max_store_buffer, value, reached) in [(Some(1), 1, true), (None, 4, false)] {
        let outcome = two.claim(&tso, max_store_buffer);
        let ModelCheckOutcome::Violated { stats, .. } = &outcome else {
            panic!("both loads read 0: {outcome:?}");
        };
        assert_eq!(stats.bound_limited > 0, reached, "{stats:?}");
        let record = two.settle(&tso, &outcome, false, max_store_buffer);
        assert_eq!(record.label, ResultLabel::Refuted, "{record:?}");
        assert_eq!(
            record.memory_bound,
            Some(StatedMemoryBound {
                bound: bound(value),
                reached,
            })
        );
    }
}

/// A protocol system under `Sc` refuses a subject that resolves a
/// `parallel` to `tso`, and every system refuses `ra`, which is not
/// stepped: `unsupported_construct`/`not-yet-implemented`.
#[trace("TC-666")]
#[test]
fn a_memory_model_refuses_a_resolved_model_it_does_not_implement() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    for (selection, refused) in [
        (
            MemoryModelName::Tso,
            case.system(&[select("SB::Both", MemoryModelName::Tso)], Sc)
                .err(),
        ),
        (
            MemoryModelName::Ra,
            case.system(&[select("SB::Both", MemoryModelName::Ra)], Tso::default())
                .err(),
        ),
    ] {
        let refusal = refused.unwrap_or_else(|| panic!("{selection:?} refuses"));
        assert_eq!(
            (refusal.code().as_str(), refusal.cause()),
            ("unsupported_construct", "not-yet-implemented")
        );
    }
}

/// A claim's one atom over the bound core's registers: the weak outcome
/// `0` of a litmus test does not hold.
struct Forbid(fn(&ModelSimState, &str) -> bool);

impl ModelLetters for Forbid {
    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        Ok(if (self.0)(state, &instance.key) {
            AtomReading::False
        } else {
            AtomReading::True
        })
    }
}

/// Register `name` of the core `key`.
fn register(state: &ModelSimState, key: &str, name: &str) -> i64 {
    field(state, "cores", key, name)
}

/// MP: `r1 = 1` (saw `y := 1`) and `r2 = 0` (missed the earlier `x := 1`).
fn message_passing(state: &ModelSimState, key: &str) -> bool {
    register(state, key, "r1") == 1 && register(state, key, "r2") == 0
}

/// LB: each load saw the other thread's later store.
fn load_buffering(state: &ModelSimState, key: &str) -> bool {
    register(state, key, "r1") == 1 && register(state, key, "r2") == 1
}

/// IRIW: the two readers saw the two independent stores in opposite orders.
fn independent_reads(state: &ModelSimState, key: &str) -> bool {
    register(state, key, "r1") == 1
        && register(state, key, "r2") == 0
        && register(state, key, "r3") == 1
        && register(state, key, "r4") == 0
}

/// 2+2W: after the join, memory holds each location's first store.
fn two_plus_two_writes(state: &ModelSimState, key: &str) -> bool {
    register(state, key, "r1") == 1 && register(state, key, "r2") == 1
}

/// CoRR: a later load of `x` read an older value than an earlier one.
fn coherent_reads(state: &ModelSimState, key: &str) -> bool {
    register(state, key, "r1") == 1 && register(state, key, "r2") == 0
}

static MP: Forbid = Forbid(message_passing);
static LB: Forbid = Forbid(load_buffering);
static IRIW: Forbid = Forbid(independent_reads);
static TWO_PLUS_TWO_W: Forbid = Forbid(two_plus_two_writes);
static CORR: Forbid = Forbid(coherent_reads);

/// A litmus `parallel Both` with `branches` (label, control), inside
/// `sequence Main` and followed by `after`.
fn litmus_run(branches: &[(&str, String)], after: &str) -> String {
    let labels: Vec<&str> = branches.iter().map(|(label, _)| *label).collect();
    let bodies: String = branches
        .iter()
        .map(|(label, body)| format!("branch {label} {body} "))
        .collect();
    format!(
        "sequence Main {{ parallel Both {{ {bodies}}} join all [{}]; {after} }}",
        labels.join(", ")
    )
}

/// A branch body of the attempts `attempts`, in a sequence `name`.
fn steps(name: &str, attempts: &[String]) -> String {
    format!("sequence {name} {{ {} }}", attempts.concat())
}

/// The store `node` of `value` (`StoreOne` or `StoreTwo`) to `cell`.
fn store(node: &str, cell: &str, value: &str) -> String {
    let select = if cell == "x" { "IsX" } else { "IsY" };
    attempt(
        node,
        "w",
        "Cell::set",
        "relaxed",
        &format!("{select}, {value}"),
    )
}

/// The `relaxed` load `node` of `cell` into register `n`.
fn load(node: &str, cell: &str, n: u8) -> String {
    load_ordered(node, cell, n, "relaxed")
}

/// The load `node` of `cell` into register `n` with `ordering` (empty for
/// none, as a local access must be).
fn load_ordered(node: &str, cell: &str, n: u8, ordering: &str) -> String {
    let select = if cell == "x" { "ReadsX" } else { "ReadsY" };
    attempt(
        node,
        "c",
        &format!("Core::read{n}"),
        ordering,
        &format!("{select}{n}, Read{n}"),
    )
}

/// ADR-025 §3.3's litmus tests whose weak outcome `tso` forbids, each with
/// its claim's letters.
fn forbidden_litmus() -> Vec<(&'static str, String, &'static Forbid)> {
    vec![
        (
            "MP",
            litmus_run(
                &[
                    (
                        "a",
                        steps(
                            "A",
                            &[store("Sx", "x", "StoreOne"), store("Sy", "y", "StoreOne")],
                        ),
                    ),
                    ("b", steps("B", &[load("Ly", "y", 1), load("Lx", "x", 2)])),
                ],
                "",
            ),
            &MP,
        ),
        (
            "LB",
            litmus_run(
                &[
                    (
                        "a",
                        steps("A", &[load("Lx", "x", 1), store("Sy", "y", "StoreOne")]),
                    ),
                    (
                        "b",
                        steps("B", &[load("Ly", "y", 2), store("Sx", "x", "StoreOne")]),
                    ),
                ],
                "",
            ),
            &LB,
        ),
        (
            "IRIW",
            litmus_run(
                &[
                    ("a", store("Sx", "x", "StoreOne")),
                    ("b", store("Sy", "y", "StoreOne")),
                    ("c", steps("C", &[load("L1", "x", 1), load("L2", "y", 2)])),
                    ("d", steps("D", &[load("L3", "y", 3), load("L4", "x", 4)])),
                ],
                "",
            ),
            &IRIW,
        ),
        (
            "2+2W",
            litmus_run(
                &[
                    (
                        "a",
                        steps(
                            "A",
                            &[store("Sx1", "x", "StoreOne"), store("Sy2", "y", "StoreTwo")],
                        ),
                    ),
                    (
                        "b",
                        steps(
                            "B",
                            &[store("Sy1", "y", "StoreOne"), store("Sx2", "x", "StoreTwo")],
                        ),
                    ),
                ],
                &(load_ordered("Fx", "x", 1, "") + &load_ordered("Fy", "y", 2, "")),
            ),
            &TWO_PLUS_TWO_W,
        ),
        (
            "CoRR",
            litmus_run(
                &[
                    ("a", store("Sx", "x", "StoreOne")),
                    ("b", steps("B", &[load("L1", "x", 1), load("L2", "x", 2)])),
                ],
                "",
            ),
            &CORR,
        ),
    ]
}

/// The core's registers at every terminal state of `system`.
fn outcomes<M: MemoryModel>(system: &ProtocolSystem<'_, M>) -> BTreeSet<Vec<i64>> {
    reachable(system)
        .iter()
        .filter(|state| state.steps.is_empty())
        .map(|state| {
            ["r1", "r2", "r3", "r4"]
                .iter()
                .map(|name| register(state.state.model(), "core", name))
                .collect()
        })
        .collect()
}

/// FR-221-AC-4's litmus half (TC-666): under `tso`, the message-passing,
/// load-buffering, IRIW, 2+2W and CoRR tests each settle `proved`: the
/// weak outcome is unreachable, and the terminal outcomes are exactly the
/// `sc` ones. Each `tso` record states `max_store_buffer` 4, not reached;
/// the `sc` record states no memory bound.
#[trace("TC-666", "FR-221-AC-4")]
#[test]
fn tso_forbids_the_weak_outcomes_of_mp_lb_iriw_2_2w_and_corr() {
    let tso = [select("SB::Main::Both", MemoryModelName::Tso)];
    for (name, run, letters) in forbidden_litmus() {
        let case = Case::with_letters(
            &(contracts() + &protocol("SB", &run) + NO_BOTH_ZERO),
            letters,
        );
        let under_tso = outcomes(&case.system(&tso, Tso::default()).expect("builds"));
        let under_sc = outcomes(&case.system(&[], Sc).expect("builds"));
        assert!(under_sc.len() > 1, "{name}: {under_sc:?}");
        assert_eq!(under_tso, under_sc, "{name}");
        let outcome = case.claim(&tso, None);
        assert!(
            matches!(outcome, ModelCheckOutcome::Holds { .. }),
            "{name}: {outcome:?}"
        );
        let record = case.settle(&tso, &outcome, false, None);
        assert_eq!(record.label, ResultLabel::Proved, "{name}: {record:?}");
        assert_eq!(
            record.memory_bound,
            Some(StatedMemoryBound {
                bound: MemoryBound {
                    member: MemoryBoundMember::MaxStoreBuffer,
                    value: 4,
                },
                reached: false,
            }),
            "{name}"
        );
        let sc = case.settle(&[], &case.claim(&[], None), false, None);
        assert_eq!(sc.label, ResultLabel::Proved, "{name}: {sc:?}");
        assert_eq!(sc.memory_bound, None, "{name}");
    }
}

/// A `send` after a store, beside a branch that receives it and then
/// loads the stored location.
fn send_unit() -> String {
    let run = format!(
        "parallel Both {{ branch s sequence Sender {{ {} send Tx via ch as (tx: Boolean) {{ tx }}; }} \
         branch r sequence Recv {{ receive Rx via ch of s::Sender::Tx as (rx: Boolean) {{ true }}; {} }} }} \
         join all [s, r];",
        store("Sx", "x", "StoreOne"),
        load("Lx", "x", 2),
    );
    let channel = "channel ch from w to w carries Boolean \
                   ordering unordered delivery unknown capacity 2 overflow block;";
    contracts() + &protocol_with("SB", channel, &run) + NO_BOTH_ZERO
}

/// FR-221-AC-4's `send` half (TC-666): a `send` by a thread whose buffer
/// is not empty is not enabled; once its store is flushed, it is.
#[trace("TC-666", "FR-221-AC-4")]
#[test]
fn a_send_waits_for_its_threads_buffer() {
    let case = Case::new(&send_unit());
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    let system = case.system(&tso, Tso::default()).expect("builds");
    let mut waited = 0;
    for state in reachable(&system) {
        let sender = ThreadPath::root(0).child("Both", "s", 0);
        let buffered = !state
            .state
            .memory()
            .buffer(&sender)
            .unwrap_or_default()
            .is_empty();
        let sends = state
            .steps
            .iter()
            .any(|(transition, _)| transition.kind() == StepKind::Send);
        assert!(!(buffered && sends));
        let rests_at_send = system
            .threads(&state.state)
            .expect("threads")
            .iter()
            .any(|thread| {
                thread.path == sender
                    && thread.rest_point.node.as_deref() == Some("Both/s/Sender/Tx")
            });
        if buffered && rests_at_send {
            waited += 1;
            let flushed = state
                .steps
                .iter()
                .find(|(transition, _)| transition.memory_step().is_some())
                .map(|(_, next)| *next)
                .expect("a flush");
            let after = &reachable(&system)[flushed];
            assert!(after
                .steps
                .iter()
                .any(|(transition, _)| transition.kind() == StepKind::Send));
        }
    }
    assert!(waited > 0);
}

/// ADR-025 MR-1 and MR-2 under `tso`: from `SB` after `Sx`, the plain store
/// `Sx` writes `left`'s buffer and not `x`'s memory value, so it is
/// invisible to an atom that reads `x`; `flush(left)` reads and writes the
/// buffer and writes `x`, so it is visible; `Ly` reads `left`'s buffer and
/// `y`, and is independent of `Sy`; `flush(left)` and `Lx` are dependent
/// through `mem(x)` (FR-229-AC-4's `tso` half); `join` is enabled by both
/// buffers.
#[trace("TC-666")]
#[test]
fn tso_steps_carry_store_buffer_footprints() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    let system = case.system(&tso, Tso::default()).expect("builds");
    let shared = |key: &str| {
        system
            .weak_accesses()
            .shared()
            .iter()
            .find(|location| {
                system
                    .weak_accesses()
                    .cell(location)
                    .is_some_and(|cell| cell.key == key)
            })
            .cloned()
            .expect("a shared location")
    };
    let (x, y) = (shared("x"), shared("y"));
    let buffer = |label: &str| Location::Protocol(ProtocolLocation::Buffer(branch(label)));
    let step_from = |state: &ProtocolState<Tso>, pick: &dyn Fn(&ProtocolTransition) -> bool| {
        system
            .successors(state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| pick(transition))
            .expect("enabled")
    };
    let (_, forked) = step_from(&system.initial()[0], &|transition| {
        transition.kind() == StepKind::Fork
    });
    let (sx, stored) = step_from(&forked, &|transition| {
        transition.node() == Some("Both/left/L/Sx")
    });
    let store = system.footprint(&forked, &sx).expect("a footprint");
    assert!(store.writes.contains(&buffer("left")), "{store:?}");
    assert!(!store.writes.contains(&x), "{store:?}");
    assert!(store.enabling.contains(&buffer("left")), "{store:?}");
    let (flush, _) = step_from(&stored, &|transition| transition.memory_step().is_some());
    let flushed = system.footprint(&stored, &flush).expect("a footprint");
    assert!(flushed.reads.contains(&buffer("left")), "{flushed:?}");
    assert!(flushed.writes.contains(&buffer("left")), "{flushed:?}");
    assert!(flushed.writes.contains(&x), "{flushed:?}");
    let (ly, _) = step_from(&stored, &|transition| {
        transition.node() == Some("Both/left/L/Ly")
    });
    let loaded = system.footprint(&stored, &ly).expect("a footprint");
    assert!(loaded.reads.contains(&buffer("left")) && loaded.reads.contains(&y));
    // FR-229-AC-4's `tso` half: `Ly` and `Sy` are independent; `flush(left)`
    // and `Lx` are dependent through `mem(x)`.
    let footprint_of = |state: &ProtocolState<Tso>, node: &str| {
        let (step, _) = step_from(state, &|transition| transition.node() == Some(node));
        system.footprint(state, &step).expect("a footprint")
    };
    assert!(independent(
        &loaded,
        &footprint_of(&stored, "Both/right/R/Sy")
    ));
    let (_, both) = step_from(&stored, &|transition| {
        transition.node() == Some("Both/right/R/Sy")
    });
    let (flush_left, _) = step_from(&both, &|transition| {
        transition
            .memory_step()
            .is_some_and(|step| step.thread == branch("left"))
    });
    let flush_left = system.footprint(&both, &flush_left).expect("a footprint");
    assert!(!independent(
        &flush_left,
        &footprint_of(&both, "Both/right/R/Lx")
    ));
    let claim = ClaimReads {
        locations: BTreeSet::from([x]),
        anchors: BTreeSet::new(),
    };
    assert!(!visible(&sx, &store, &claim));
    assert!(visible(&flush, &flushed, &claim));
    // The join's footprint names every branch buffer it waits for.
    let joinable = reachable(&system)
        .into_iter()
        .find_map(|state| {
            state
                .steps
                .iter()
                .find(|(transition, _)| transition.kind() == StepKind::Join)
                .map(|(transition, _)| (state.state.clone(), transition.clone()))
        })
        .expect("a join");
    let join = system
        .footprint(&joinable.0, &joinable.1)
        .expect("a footprint");
    assert!(join.enabling.contains(&buffer("left")) && join.enabling.contains(&buffer("right")));
}

/// ADR-027 FT-4 over `ProtocolSystem<Tso>` (ADR-025 MR-1): over `SB`, the
/// FIFO fixture, message passing and the `send` fixture under `tso`, every
/// pair of steps the footprints report independent commutes, and some
/// independent pairs hold a `flush`.
#[trace("TC-661", "TC-666")]
#[test]
fn every_independent_pair_of_tso_steps_commutes() {
    let fifo = format!(
        "parallel Both {{ branch left sequence L {{ {} {} {} }} \
         branch right sequence R {{ {} {} }} }} join all [left, right];",
        store("S1", "x", "StoreOne"),
        store("S2", "y", "StoreOne"),
        load("L3", "x", 3),
        load("Lx", "x", 2),
        load("Ly", "y", 1),
    );
    let (_, mp, _) = forbidden_litmus().remove(0);
    let units = [
        (sb_unit(&sb_run("", "release", "acquire")), "SB::Both"),
        (contracts() + &protocol("SB", &fifo), "SB::Both"),
        (contracts() + &protocol("SB", &mp), "SB::Main::Both"),
        (send_unit(), "SB::Both"),
        (sb_unit(&fenced_sb("seq_cst", "acquire")), "SB::Both"),
    ];
    let mut flushes = 0;
    for (body, parallel) in &units {
        let case = Case::new(body);
        let system = case
            .system(&[select(parallel, MemoryModelName::Tso)], Tso::default())
            .expect("builds");
        let reached = reachable(&system);
        assert!(
            crate::protocol_footprints::assert_independent_steps_commute(&system, &reached) > 0,
            "{body}"
        );
        for state in &reached {
            for (first, _) in &state.steps {
                for (second, _) in &state.steps {
                    if first.memory_step().is_some() && second.memory_step().is_none() {
                        let left = system.footprint(&state.state, first).expect("a footprint");
                        let right = system.footprint(&state.state, second).expect("a footprint");
                        if independent(&left, &right) {
                            flushes += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(flushes > 0);
}

/// SR-12239 FND-001 (FR-225, ADR-025 MB-4; TC-670): the closure checker
/// re-expands under the request's bound. A `Holds` certificate from a run
/// under the default bound, checked under `max_store_buffer` 1, which cuts
/// stores, settles V-6 `MemoryBoundReached` with as many bound-limited
/// states as the engine's own run under 1 reports, never `proved`; checked
/// under the default it settles `proved`, the bound not reached.
#[trace("TC-670")]
#[test]
fn a_closure_certificate_closed_only_under_a_cutting_bound_is_not_proved() {
    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} {} }} \
         branch right sequence R {{ {} {} }} }} join all [left, right];",
        store("S1", "x", "StoreOne"),
        store("S2", "y", "StoreOne"),
        load("Lx", "x", 2),
        load("Ly", "y", 1),
    );
    let two = Case::new(&(contracts() + &protocol("SB", &run) + NO_BOTH_ZERO));
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    let holds = two.check(&tso, ModelCheckItem::DeadlockFreedom, None);
    assert!(
        matches!(holds, ModelCheckOutcome::Holds { .. }),
        "{holds:?}"
    );
    let ModelCheckOutcome::Memory {
        cause: MemoryCause::BoundReached { states, .. },
        ..
    } = two.check(&tso, ModelCheckItem::DeadlockFreedom, Some(1))
    else {
        panic!("the engine cuts under 1");
    };
    let bound = |value| MemoryBound {
        member: MemoryBoundMember::MaxStoreBuffer,
        value,
    };
    let cut = two.settle(&tso, &holds, true, Some(1));
    assert_eq!(
        cut.value,
        TerminalValue::Inconclusive(InconclusiveCause::MemoryBoundReached {
            bound: bound(1),
            states,
        }),
        "{cut:?}"
    );
    assert_eq!(
        cut.memory_bound,
        Some(StatedMemoryBound {
            bound: bound(1),
            reached: true,
        })
    );
    let proved = two.settle(&tso, &holds, true, None);
    assert_eq!(proved.label, ResultLabel::Proved, "{proved:?}");
    assert_eq!(
        proved.memory_bound,
        Some(StatedMemoryBound {
            bound: bound(4),
            reached: false,
        })
    );
}
