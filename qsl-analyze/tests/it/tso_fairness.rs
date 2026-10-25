// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-228 and FR-227 (TC-673, TC-672): flush fairness under `tso`, its
//! effect on a liveness clause over a weak `parallel`, and the replay of a
//! lasso that keeps a store buffer non-empty.
//!
//! `Spin` is the store-and-poll program of QSL-994 over the `litmus` fixture, named `SB`
//! because the shared case helpers name that protocol: branch `w` has one
//! `relaxed` store of 1 to `x`, and branch `r` a `repeat` with no maximum
//! whose body is one `relaxed` load of `x` into `k.r1`. The fixture's
//! operations return no value, so the loop guard is `true` in place of the
//! load's binder reading 0; the lasso of the AC is the same.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_analyze::model_check::{
    check_protocol, LivenessRefusal, ModelCheckRefusal, ProtocolCheckRequest,
};
use qsl_eval::fairness::{check_loop, EdgeLabel, Identity, LassoFairness, LoopStep};
use qsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::protocol::{
    protocol_constraints, FairnessOrigin, MemorySelection, ProtocolFairness, ProtocolState,
    ProtocolSystem, ProtocolTarget, ProtocolTransition, Sc, StepKind, Tso,
};
use qsl_eval::simulation::{SubjectSystem, TransitionSystem};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{Code, MemoryModelName, SourceIdentity};
use qsl_package::CheckedPackage;
use qsl_replay::{
    compile_package, replay_protocol_trace, CounterexampleKind, DependencyInput, Lasso,
    MemoryBound, MemoryBoundMember, MemoryCause, ModelCheckItem, ModelCheckLimits,
    ModelCheckOutcome, ModelStep, ModelTraceRefusal, ModelTraceSubject, ProtocolTraceSubject,
    ReplayLimits, ReplayRefusal, TemporalCounterexample,
};
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Cancel, Meter};
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

use crate::litmus::{
    attempt, contracts, core, document, field, package, protocol, sb_run, unit_text, Documents,
};
use crate::model_settle::{envelope_at, request};
use crate::weak_memory::select;

/// `eventually holds(k.r1 = 1)`.
const SETTLES: &str = "temporal Settles using t0 over (k: M::Core) clock \"model-steps\" \
                       on origin { eventually holds(k.r1 = 1) }\n";

/// `eventually holds(k.r1 <= 1)` over store buffering.
const AT_MOST_ONE: &str = "temporal AtMostOne using t0 over (k: M::Core) clock \"model-steps\" \
                           on origin { eventually holds(k.r1 <= 1) }\n";

/// The claim's one atom, read from the bound core's `r1`.
struct R1(fn(i64) -> bool);

impl ModelLetters for R1 {
    crate::units::unguarded!();

    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Ok(if (self.0)(field(state, "cores", &instance.key, "r1")) {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

static IS_ONE: R1 = R1(|value| value == 1);
static AT_MOST: R1 = R1(|value| value <= 1);

/// `Spin`'s `run`.
fn spin_run() -> String {
    format!(
        "parallel Both {{ branch w sequence W {{ {} }} \
         branch r repeat Spin by c visible () while {{ true }} sequence R {{ {} }} }} \
         join all [w, r];",
        attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        attempt("Lx", "c", "Core::read1", "relaxed", "ReadsX1, Read1"),
    )
}

/// One unit with its claim.
struct Unit {
    body: String,
    documents: Documents,
    checked: CheckedPackage,
    letters: &'static R1,
}

impl Unit {
    fn new(run: &str, claim: &str, letters: &'static R1) -> Self {
        let body = contracts() + &protocol("SB", run) + claim;
        Self {
            checked: package(&body),
            documents: Documents::new(),
            body,
            letters,
        }
    }

    fn subject(
        &self,
        selection: &[MemorySelection],
    ) -> qsl_eval::model_subject::ProtocolSubject<'_> {
        self.documents.subject(&self.checked, "SB", selection)
    }

    fn clause(&self) -> &qsl_semantics::check::CheckedTemporalClause {
        self.checked.graph().temporal_clauses()[0].clause()
    }

    fn item(&self) -> ModelCheckItem<'_> {
        ModelCheckItem::Clause {
            clause: self.clause(),
            letters: self.letters,
        }
    }

    fn tso(&self) -> Vec<MemorySelection> {
        vec![select("SB::Both", MemoryModelName::Tso)]
    }

    fn check(
        &self,
        selection: &[MemorySelection],
        max_store_buffer: Option<u64>,
    ) -> Result<ModelCheckOutcome<ProtocolTransition>, ModelCheckRefusal> {
        check_protocol(
            ProtocolCheckRequest {
                subject: self.subject(selection),
                item: self.item(),
                max_depth: None,
                max_store_buffer,
                max_messages: None,
                max_live_instances: None,
                limits: ModelCheckLimits::default(),
            },
            &Cancel::new(),
        )
    }

    fn replay(
        &self,
        selection: &[MemorySelection],
        payload: TemporalCounterexample<ProtocolTransition>,
    ) -> Result<qsl_replay::ModelTraceReplayResult, ReplayRefusal> {
        let unit = unit_text(&self.body).into_bytes();
        let id = compile_package(
            SourceIdentity::new("agent-ix", "litmus", "git", "1"),
            "litmus.native",
            &unit,
            [document().as_slice()],
            &DependencyInput::default(),
            qsl_replay::StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("the unit compiles")
        .package_id();
        let node = &self.checked.graph().temporal_clauses()[0];
        let clause_node = WireNodeId::from_digest(*node.identity().as_bytes());
        let occurrence = OccurrenceKey::new(clause_node, node.claim().clone());
        let subject = self.subject(selection);
        replay_protocol_trace(
            request(&document(), self.documents.snapshots.iter(), &unit, id),
            &ProtocolTraceSubject {
                model: ModelTraceSubject {
                    initial: vec![self.documents.initial.clone()],
                    universes: self.documents.universes(),
                    expansion: subject.model.simulation.expansion,
                },
                protocol: "SB".to_owned(),
                over: core(),
                roles: subject.roles.clone(),
                memory: subject.memory.clone(),
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
            },
            &envelope_at(&unit, id, payload, clause_node, occurrence),
            self.letters,
            ReplayLimits::default(),
        )
    }
}

/// The `fair weak each` flush member over the branch `branch` of `Both`.
fn flush(branch: &str) -> ProtocolFairness {
    ProtocolFairness::memory_step(MemoryModelName::Tso, "flush", "Both", branch)
}

/// The system of `unit` under `memory`.
macro_rules! system {
    ($unit:expr, $selection:expr, $memory:expr) => {{
        let subject = $unit.subject($selection);
        let model = ModelSystem::new(subject.model.simulation.clone()).expect("admits");
        ProtocolSystem::new(model, &subject, $memory).expect("builds")
    }};
}

/// ADR-025 MF-1 (TC-673): `eventually holds(k.r1 <= 1)` over
/// `SB` resolved to `tso` carries `fair weak each flush(left)` and
/// `fair weak each flush(right)` with origin `memory` beside the three
/// scheduler constraints; resolved to `sc` it carries no `memory` member.
/// Not covered: the `ra` visibility member, which waits on QSL-933.
#[trace("TC-673")]
#[test]
fn a_tso_clause_carries_flush_fairness_beside_the_scheduler_constraints() {
    let unit = Unit::new(&sb_run("", "", ""), AT_MOST_ONE, &AT_MOST);
    let tso = unit.tso();
    let set = system!(unit, &tso, Tso::new(4))
        .fairness_set(unit.clause().fairness())
        .expect("the set builds");
    let memory: Vec<&ProtocolFairness> = set
        .iter()
        .filter(|member| member.origin() == FairnessOrigin::Memory)
        .collect();
    assert_eq!(memory, [&flush("left"), &flush("right")], "{set:?}");
    for member in &memory {
        assert_eq!(
            (member.kind(), member.granularity()),
            (FairnessKind::Weak, FairnessGranularity::Each)
        );
    }
    let schedulers: Vec<&ProtocolTarget> = set
        .iter()
        .filter(|member| member.origin() == FairnessOrigin::Scheduler)
        .map(ProtocolFairness::target)
        .collect();
    assert_eq!(schedulers.len(), 3, "{set:?}");
    assert_eq!(set.len(), 5, "{set:?}");
    let keys: Vec<Vec<u8>> = set
        .iter()
        .map(|member| {
            quire_canonical::to_vec(member, quire_canonical::Limits::new(4096)).expect("encodes")
        })
        .collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "{keys:?}");

    let sc = system!(unit, &[], Sc)
        .fairness_set(unit.clause().fairness())
        .expect("the set builds");
    assert!(
        sc.iter()
            .all(|member| member.origin() != FairnessOrigin::Memory),
        "{sc:?}"
    );
}

/// A reachable state and its steps, each with the index of its post-state.
type Reach = (ProtocolState<Tso>, Vec<(ProtocolTransition, usize)>);

/// The reachable states of `system` with each state's steps, breadth-first.
fn reached(system: &ProtocolSystem<'_, Tso>) -> Vec<Reach> {
    let mut known: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    let mut states: Vec<Reach> = Vec::new();
    for state in system.initial() {
        let key = system.state_key(&state).expect("a key").0;
        known.insert(key, states.len());
        states.push((state, Vec::new()));
    }
    let mut at = 0;
    while at < states.len() {
        let successors = system
            .successors(&states[at].0)
            .expect("expands")
            .successors;
        for (transition, next) in successors {
            let key = system.state_key(&next).expect("a key").0;
            let index = *known.entry(key).or_insert_with(|| {
                states.push((next, Vec::new()));
                states.len() - 1
            });
            states[at].1.push((transition, index));
        }
        at += 1;
    }
    states
}

/// The unflushed lasso over `Spin`: its stem and the index of the state
/// whose `Lx` self-loop is its loop. The path is the breadth-first one to the
/// first state at which `r`'s load leaves the state as it was while `w`'s
/// buffer holds the store.
fn unflushed_lasso(
    system: &ProtocolSystem<'_, Tso>,
) -> (
    Vec<(ProtocolTransition, usize)>,
    usize,
    Vec<ProtocolTransition>,
) {
    let states = reached(system);
    let (looped, load) = states
        .iter()
        .enumerate()
        .find_map(|(at, (_, steps))| {
            let buffered = steps
                .iter()
                .any(|(transition, _)| transition.memory_step().is_some());
            steps
                .iter()
                .find(|(transition, next)| {
                    buffered
                        && *next == at
                        && transition.node().is_some_and(|node| node.ends_with("Lx"))
                })
                .map(|(transition, _)| (at, transition.clone()))
        })
        .expect("a state whose load returns to it");
    let mut parent: BTreeMap<usize, (usize, ProtocolTransition)> = BTreeMap::new();
    let mut queue = std::collections::VecDeque::from([0usize]);
    while let Some(at) = queue.pop_front() {
        for (transition, next) in &states[at].1 {
            if *next != 0 && !parent.contains_key(next) {
                parent.insert(*next, (at, transition.clone()));
                queue.push_back(*next);
            }
        }
    }
    let mut stem = Vec::new();
    let mut at = looped;
    while at != 0 {
        let (before, transition) = parent[&at].clone();
        stem.push((transition, at));
        at = before;
    }
    stem.reverse();
    stem.push((load, looped));
    let enabled = states[looped]
        .1
        .iter()
        .map(|(transition, _)| transition.clone())
        .collect();
    (stem, looped, enabled)
}

/// The first member of `fairness` that the loop taking `taken` at a state
/// enabling `enabled` is unfair under; `None` when the loop is fair.
fn unfair_member<'f>(
    fairness: &'f [ProtocolFairness],
    enabled: &[ProtocolTransition],
    taken: &ProtocolTransition,
) -> Option<&'f ProtocolFairness> {
    let numbers: BTreeMap<&ProtocolTransition, Identity> = enabled
        .iter()
        .enumerate()
        .map(|(index, transition)| (transition, Identity::new(index as u64)))
        .collect();
    let identity = |transition: &ProtocolTransition| -> Result<Identity, ()> {
        numbers.get(transition).copied().ok_or(())
    };
    let (members, constraints): (Vec<usize>, Vec<_>) =
        protocol_constraints(fairness, enabled, identity)
            .expect("every transition is numbered")
            .into_iter()
            .unzip();
    let step = LoopStep::new(
        enabled.iter().map(|transition| numbers[transition]),
        EdgeLabel::Transition(numbers[taken]),
    );
    match check_loop(&[step], &constraints).expect("a loop has a step") {
        LassoFairness::Fair => None,
        LassoFairness::Unfair { constraint } => Some(&fairness[members[constraint]]),
    }
}

/// ADR-025 MF-1 (TC-673), the filter half: the lasso whose stem is `fork`,
/// `w`'s store and whose loop is `r`'s load with `w`'s buffer `[x := 1]`
/// throughout meets every scheduler constraint, since `w` has no enabled
/// step and `r` steps on every edge, and is rejected by the fairness set only
/// under `flush(w)`'s constraint. Not covered: the AC's `proved` verdict,
/// which waits on FR-339's component certificate, and its `refuted` verdict
/// with the memory constraints ignored, which the engine has no switch for.
#[trace("TC-673")]
#[test]
fn the_unflushed_lasso_is_unfair_only_under_the_flush_constraint() {
    let unit = Unit::new(&spin_run(), SETTLES, &IS_ONE);
    let tso = unit.tso();
    let system = system!(unit, &tso, Tso::new(4));
    let (stem, looped, enabled) = unflushed_lasso(&system);
    let (taken, _) = stem.last().expect("a loop step").clone();
    assert_eq!(
        stem.iter().map(|(t, _)| t.kind()).collect::<Vec<_>>()[..2],
        [StepKind::Fork, StepKind::Attempt]
    );
    assert!(looped > 0);
    let w = qsl_eval::simulation::protocol::ThreadPath::root(0).child("Both", "w", 0);
    assert!(
        enabled
            .iter()
            .any(|t| t.memory_step().is_some_and(|step| step.thread == w)),
        "w's flush is enabled throughout the loop: {enabled:?}"
    );
    assert!(
        enabled.iter().all(|t| t.thread() != Some(&w)),
        "w has no enabled step of its own"
    );

    let full = system
        .fairness_set(unit.clause().fairness())
        .expect("the set builds");
    assert_eq!(
        unfair_member(&full, &enabled, &taken),
        Some(&flush("w")),
        "{full:?}"
    );
    let ignored: Vec<ProtocolFairness> = full
        .into_iter()
        .filter(|member| member.origin() != FairnessOrigin::Memory)
        .collect();
    assert_eq!(unfair_member(&ignored, &enabled, &taken), None);
}

/// ADR-025 MF-1 (TC-673), the engine half: with flush fairness in the set,
/// EN-1 over `Spin` finds no fair counterexample to `eventually
/// holds(k.r1 = 1)`: it explores the product and, having no component
/// certificate to return, refuses `ComponentCertificateNotBuilt`. Not
/// covered: `proved` itself, which waits on FR-339's component certificate.
#[trace("TC-673")]
#[test]
fn eventually_the_load_reads_the_flushed_store() {
    let unit = Unit::new(&spin_run(), SETTLES, &IS_ONE);
    let refusal = unit
        .check(&unit.tso(), None)
        .expect_err("the proof waits on FR-339");
    let ModelCheckRefusal::Liveness(LivenessRefusal::ComponentCertificateNotBuilt { stats }) =
        refusal
    else {
        panic!("{refusal:?}");
    };
    assert!(stats.product_states > 0, "{stats:?}");
}

/// ADR-025 MF-1 (TC-673): a liveness clause over a `tso` `parallel` that is
/// false even with flush fairness is refuted, and the counterexample carries
/// the memory constraints in its fairness set. `k.r1` never reads 2.
#[trace("TC-673")]
#[test]
fn a_false_liveness_claim_over_tso_is_refuted_with_the_memory_constraints() {
    static IS_TWO: R1 = R1(|value| value == 2);
    let claim = SETTLES.replace("= 1", "= 2");
    let unit = Unit::new(&spin_run(), &claim, &IS_TWO);
    let outcome = unit.check(&unit.tso(), None).expect("the subject checks");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    assert!(counterexample.lasso.is_some());
    assert!(
        counterexample.fairness.contains(&flush("w")),
        "{:?}",
        counterexample.fairness
    );
    unit.replay(&unit.tso(), counterexample.as_ref().clone())
        .expect("the fair lasso replays");
}

/// ADR-025 MK-2 (TC-672): the unflushed lasso over `Spin`, in which `w`'s buffer
/// stays `[x := 1]` through `r`'s repeated load of `x`, refuses
/// `invalid_runtime_input`/`invalid-value` as unfair under `flush(w)`'s
/// constraint. Not covered: FR-227-AC-4's `ra` clause, which waits on QSL-933.
#[trace("TC-672")]
#[test]
fn replay_refuses_the_lasso_that_never_flushes_w() {
    let unit = Unit::new(&spin_run(), SETTLES, &IS_ONE);
    let tso = unit.tso();
    let system = system!(unit, &tso, Tso::new(4));
    let (stem, looped, _) = unflushed_lasso(&system);
    let states = reached(&system);
    let prefix: Vec<ModelStep<ProtocolTransition>> = stem
        .iter()
        .map(|(transition, at)| ModelStep {
            transition: system
                .reported(transition.clone(), &states[*at].0)
                .expect("the step reports"),
            post_state: system.state_key(&states[*at].0).expect("a key").1,
        })
        .collect();
    assert_eq!(stem.last().map(|(_, at)| *at), Some(looped));
    let payload = TemporalCounterexample {
        initial_state: 0,
        lasso: Some(Lasso {
            entry: prefix.len() - 1,
            terminal_stutter: false,
        }),
        prefix,
        over_binding: Some(core()),
        fairness: system
            .fairness_set(unit.clause().fairness())
            .expect("the set builds"),
        kind: CounterexampleKind::Formula,
        blocked: Vec::new(),
        refinement: None,
    };
    let Err(ReplayRefusal::ModelTrace(refusal)) = unit.replay(&tso, payload) else {
        panic!("an unfair lasso refuses");
    };
    let ModelTraceRefusal::UnfairProtocolLasso { constraint } = refusal.as_ref() else {
        panic!("{refusal:?}");
    };
    assert_eq!(**constraint, flush("w"));
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, "invalid-value")
    );
}

/// The S3 refusal of the `Spin` unit whose clause carries `fairness`.
fn refusal_of(fairness: &str) -> qsl_semantics::check::CheckRefusal {
    let claim = format!(
        "temporal Settles using t0 over (k: M::Core) clock \"model-steps\" \
         on origin {{ {fairness} eventually holds(k.r1 = 1) }}\n"
    );
    let body = contracts() + &protocol("SB", &spin_run()) + &claim;
    let document = document();
    let text = unit_text(&body);
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "litmus", "git", "1"),
        "litmus.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses the unit");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the model selection admits")
    .with_parsed_operation_clauses(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    })
    .expect("the operation clauses parse");
    let mut refusals = qsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(quire_semantic_value::checking::CheckingLimits::default())
    .expect_err("the unit refuses");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    refusals.remove(0)
}

/// ADR-025 MF-1 (TC-673): a `fair weak` constraint naming
/// `flush` refuses `missing_declaration`/`missing-name`, since `flush(b)` is
/// no declared operation or thread target. Not covered: the TP-1 claim's
/// verdicts, whose `ra` half waits on QSL-933 and whose "filter ignores them"
/// half the engine has no switch for.
#[trace("TC-673")]
#[test]
fn a_fair_constraint_naming_flush_is_a_missing_declaration() {
    let refusal = refusal_of("fair weak flush;");
    assert_eq!(
        (refusal.cause.code(), refusal.cause.cause()),
        (Code::MissingDeclaration, Some("missing-name")),
        "{refusal:?}"
    );
}

/// ADR-025 MB-3 and MB-4 over a liveness clause: `w` stores twice, so under
/// `max_store_buffer` 1 its second store is cut at the state where the first
/// is buffered. The run settles `MemoryBoundReached` naming the bound and the
/// bound-limited states in place of a proof, and under the default bound,
/// which cuts nothing, it reaches the proof's component certificate.
#[trace("TC-673")]
#[test]
fn a_liveness_run_that_cut_a_store_settles_inconclusive() {
    let run = spin_run().replace(
        &attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        &format!(
            "{} {}",
            attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
            attempt("Sy", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        ),
    );
    let unit = Unit::new(&run, SETTLES, &IS_ONE);
    let outcome = unit
        .check(&unit.tso(), Some(1))
        .expect("the subject checks");
    let ModelCheckOutcome::Memory {
        cause: MemoryCause::BoundReached { bound, states },
        ..
    } = outcome
    else {
        panic!("{outcome:?}");
    };
    assert_eq!(
        bound,
        MemoryBound {
            member: MemoryBoundMember::MaxStoreBuffer,
            value: 1
        }
    );
    assert!(states > 0);
    let refusal = unit.check(&unit.tso(), None).expect_err("no bound reached");
    assert!(
        matches!(
            refusal,
            ModelCheckRefusal::Liveness(LivenessRefusal::ComponentCertificateNotBuilt { .. })
        ),
        "{refusal:?}"
    );
}

/// `Spin` whose `w` stores twice, so `max_store_buffer` 1 cuts its second
/// store at the state where the first is buffered.
fn double_store_run() -> String {
    spin_run().replace(
        &attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        &format!(
            "{} {}",
            attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
            attempt("Sy", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        ),
    )
}

/// ADR-025 MB-4 over a liveness clause: the claim `eventually holds(k.r1 =
/// 2)` is false (no load reads 2), and its lasso exists under the default
/// bound, but under `max_store_buffer` 1 a store is cut, so the run settles
/// `MemoryBoundReached` in place of the counterexample.
#[trace("TC-673")]
#[test]
fn a_false_liveness_claim_with_a_cut_store_settles_inconclusive() {
    static IS_TWO: R1 = R1(|value| value == 2);
    let claim = SETTLES.replace("= 1", "= 2");
    let unit = Unit::new(&double_store_run(), &claim, &IS_TWO);
    let found = unit.check(&unit.tso(), None).expect("the subject checks");
    assert!(
        matches!(found, ModelCheckOutcome::Violated { .. }),
        "{found:?}"
    );
    let cut = unit
        .check(&unit.tso(), Some(1))
        .expect("the subject checks");
    let ModelCheckOutcome::Memory {
        cause: MemoryCause::BoundReached { bound, states },
        ..
    } = cut
    else {
        panic!("{cut:?}");
    };
    assert_eq!(bound.value, 1);
    assert!(states > 0);
}

/// The model-only pass over a clause that is dead at its initial state still
/// meets the cut store: `eventually holds(k.r1 <= 3)` holds at once, so the
/// product has one state and the cut store is met only by the model-only
/// walk of the dead root, which settles `MemoryBoundReached` all the same.
#[trace("TC-673")]
#[test]
fn a_cut_store_met_only_by_the_model_only_pass_settles_inconclusive() {
    static ANY: R1 = R1(|value| value <= 3);
    let claim = SETTLES.replace("k.r1 = 1", "k.r1 <= 3");
    let unit = Unit::new(&double_store_run(), &claim, &ANY);
    let cut = unit
        .check(&unit.tso(), Some(1))
        .expect("the subject checks");
    let ModelCheckOutcome::Memory {
        cause: MemoryCause::BoundReached { states, .. },
        ..
    } = cut
    else {
        panic!("{cut:?}");
    };
    assert!(states > 0);
    let uncut = unit.check(&unit.tso(), None);
    assert!(
        !matches!(uncut, Ok(ModelCheckOutcome::Memory { .. })),
        "{uncut:?}"
    );
}
