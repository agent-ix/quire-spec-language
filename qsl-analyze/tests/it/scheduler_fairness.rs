// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-212 and FR-217 (TC-657, TC-662): scheduler fairness over protocol
//! threads, the `scheduling adversarial` opt-out, the classes of protocol
//! fairness targets, and the replay of a protocol lasso against them.
//!
//! `Chatter` is FR-212-AC-1's protocol over this suite's `Cell` fixture:
//! its `Done` attempt applies `inc`, which takes `k.v` from 0 to 1, where
//! FR-212's applies `set` to `flag`, so the claim reads `k.v = 1` in place
//! of `k.flag = 1` and the TP-1 claim `k.v <= 1` in place of `k.flag <= 1`.
//! Its branches, channel premises and nodes are FR-212's.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_analyze::model_check::ProtocolCheckRequest;
use qsl_analyze::model_check::{check_protocol, LivenessRefusal, ModelCheckRefusal};
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ProtocolSubject};
use qsl_eval::simulation::model::{ExpansionLimits, ModelSimState};
use qsl_eval::simulation::protocol::{
    FairnessOrigin, ProtocolClass, ProtocolFairness, ProtocolTarget, ProtocolTransition, StepKind,
    ThreadPath,
};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::AtomReading;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_replay::{
    replay_protocol_trace, settle_protocol_check, ModelCheckItem, ModelCheckLimits,
    ModelCheckOutcome, ModelCheckRecord, ModelTraceRefusal, ModelTraceReplayResult,
    ModelTraceSettlement, ModelTraceSubject, ProtocolCertificateRequest, ProtocolTraceSubject,
    ReplayLimits, ReplayRefusal, ResultLabel, TemporalCounterexample,
};
use qsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedProtocolTarget, PackageDeclarations, TemporalCause,
};
use qsl_semantics::check::{CheckedTemporalClause, FormulaNodeId};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Cancel, Meter};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

use crate::fill::{
    attempt, contracts, document, fill_unit, instance, package, package_id, protocol, reachable,
    sc_system, subject, unit_text, universe, v, Documents,
};
use crate::model_settle::{envelope_at, request};

/// `Chatter`'s channel: unordered, of capacity 2, with delivery `unknown`,
/// so its queued entries are duplicated and lost.
const CHANNEL: &str = "channel ch from w to w carries Boolean \
                       ordering unordered delivery unknown capacity 2 overflow block;";

/// `eventually holds(k.v = 1)` under infinite-trace.
const EVENTUALLY: &str = "temporal Eventually using t0 over (k: M::Cell) clock \"model-steps\" \
                          on origin { eventually holds(k.v = 1) }\n";

/// The TP-1 claim `always holds(k.v <= 1)`.
const BOUNDED: &str = "temporal Bounded using t0 over (k: M::Cell) clock \"model-steps\" \
                       on origin { always holds(k.v <= 1) }\n";

/// `Chatter`'s `run`: `s` sends `Tx` then attempts `Done`, `r` receives
/// `Tx`, joined `all`.
fn chatter_run() -> String {
    format!(
        "parallel Both {{ branch s sequence Sender {{ send Tx via ch as (tx: Boolean) {{ tx }}; {} }} \
         branch r receive Rx via ch of s::Sender::Tx as (rx: Boolean) {{ true }}; }} \
         join all [s, r];",
        attempt("Done", "inc", "IncPre, IncPost", "done", "true"),
    )
}

/// The `Chatter` unit with `members` beside its channel and `terminal any`,
/// and the clause `claim`.
fn chatter_body(members: &str, claim: &str) -> String {
    contracts("self.v = 0")
        + &protocol(
            "Chatter",
            &format!("{CHANNEL} terminal any; {members}"),
            &chatter_run(),
            "d",
        )
        + claim
}

/// The one atom of a claim over `k`, as the predicate `atom` of the bound
/// cell's `v`.
struct AtV(fn(i64) -> bool);

impl ModelLetters for AtV {
    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, quire_exact::Incomplete> {
        Ok(if (self.0)(v(state, &instance.key)) {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

/// `k.v = 1`.
const IS_ONE: AtV = AtV(|value| value == 1);

/// `k.v <= 1`.
const AT_MOST_ONE: AtV = AtV(|value| value <= 1);

/// A `Chatter` unit from `c.v = 0`, with its checked package.
struct Chatter {
    body: String,
    documents: Documents,
    initial: Vec<DocumentRef>,
    checked: CheckedPackage,
    letters: &'static AtV,
}

impl Chatter {
    /// The unit with `members` and the clause `claim`, read by `letters`.
    fn new(members: &str, claim: &str, letters: &'static AtV) -> Self {
        let body = chatter_body(members, claim);
        let mut documents = Documents::new();
        let initial = vec![documents.start("s0", &[("c", 0)])];
        let checked = package(&body);
        Self {
            body,
            documents,
            initial,
            checked,
            letters,
        }
    }

    fn subject(&self) -> ProtocolSubject<'_> {
        subject(
            &self.checked,
            &self.documents,
            self.initial.clone(),
            "Chatter",
            "c",
        )
    }

    fn clause(&self) -> &CheckedTemporalClause {
        self.checked.graph().temporal_clauses()[0].clause()
    }

    fn item(&self) -> ModelCheckItem<'_> {
        ModelCheckItem::Clause {
            clause: self.clause(),
            letters: self.letters,
        }
    }

    /// The clause's fairness set over the subject (FR-212).
    fn fairness(&self) -> Vec<ProtocolFairness> {
        sc_system(&self.subject())
            .fairness_set(self.clause().fairness())
            .expect("the fairness set builds")
    }

    /// EN-1 over the clause.
    fn check(&self) -> Result<ModelCheckOutcome<ProtocolTransition>, ModelCheckRefusal> {
        check_protocol(
            ProtocolCheckRequest {
                subject: self.subject(),
                item: self.item(),
                max_depth: None,
                max_store_buffer: None,
                limits: ModelCheckLimits::default(),
            },
            &Cancel::new(),
        )
    }

    /// FR-217's replay of `payload` against this unit.
    fn replay(
        &self,
        payload: TemporalCounterexample<ProtocolTransition>,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        let unit = unit_text(&self.body).into_bytes();
        let id = package_id(&self.body);
        let node = &self.checked.graph().temporal_clauses()[0];
        let clause_node = WireNodeId::from_digest(*node.identity().as_bytes());
        let occurrence = OccurrenceKey::new(clause_node, node.claim().clone());
        let subject = self.subject();
        replay_protocol_trace(
            request(&document(), self.documents.snapshots.iter(), &unit, id),
            &ProtocolTraceSubject {
                model: ModelTraceSubject {
                    initial: self.initial.clone(),
                    universes: universe(&["c"]),
                    expansion: ExpansionLimits {
                        max_candidates: 1_000_000,
                    },
                },
                protocol: "Chatter".to_owned(),
                over: instance("c"),
                roles: subject.roles.clone(),
                memory: subject.memory.clone(),
                max_store_buffer: None,
            },
            &envelope_at(&unit, id, payload, clause_node, occurrence),
            self.letters,
            ReplayLimits::default(),
        )
    }

    /// FR-127's record of `outcome`, its counterexample replayed by FR-217.
    fn settle(
        &self,
        outcome: &ModelCheckOutcome<ProtocolTransition>,
    ) -> ModelCheckRecord<ProtocolTransition> {
        settle_protocol_check(
            outcome,
            &ModelCheckLimits::default(),
            &package_id(&self.body),
            &ProtocolCertificateRequest {
                subject: self.subject(),
                item: self.item(),
                max_store_buffer: None,
            },
            &Cancel::new(),
            |payload| self.replay(payload.clone()),
        )
    }
}

/// The three scheduler constraints of `Chatter`: over `s`, `r` and the root.
fn chatter_schedulers() -> Vec<ProtocolTarget> {
    vec![
        ProtocolTarget::branch("Both", "s"),
        ProtocolTarget::branch("Both", "r"),
        ProtocolTarget::Root,
    ]
}

/// Each step's kind and node, in order; a channel step has no node.
fn shape(steps: &[qsl_replay::ModelStep<ProtocolTransition>]) -> Vec<(StepKind, Option<String>)> {
    steps
        .iter()
        .map(|step| {
            (
                step.transition.kind(),
                step.transition
                    .node()
                    .map(|node| node.rsplit('/').next().unwrap_or(node).to_owned()),
            )
        })
        .collect()
}

/// Part of FR-212-AC-1 (TC-657): `Chatter`'s fairness set for the
/// liveness claim holds exactly three `fair weak whole` constraints with
/// origin `scheduler`, over `s`, `r` and the root, and EN-1 finds no fair
/// accepting cycle: the lasso that starves `s` at `Done` is not fair under
/// `s`'s scheduler constraint. Not covered: `proved` itself, which waits on
/// FR-339's component certificate, so EN-1 refuses the run as
/// `ComponentCertificateNotBuilt` after exploring the whole product.
#[trace("TC-657")]
#[test]
fn chatter_carries_three_scheduler_constraints_and_no_fair_counterexample() {
    let chatter = Chatter::new("", EVENTUALLY, &IS_ONE);
    let fairness = chatter.fairness();
    assert_eq!(fairness.len(), 3, "{fairness:?}");
    let targets: BTreeSet<ProtocolTarget> = fairness
        .iter()
        .map(|member| {
            assert_eq!(
                (member.kind(), member.granularity(), member.origin()),
                (
                    FairnessKind::Weak,
                    FairnessGranularity::Whole,
                    FairnessOrigin::Scheduler
                ),
            );
            member.target().clone()
        })
        .collect();
    assert_eq!(
        targets,
        chatter_schedulers().into_iter().collect::<BTreeSet<_>>()
    );
    let refusal = chatter.check().expect_err("the proof waits on FR-339");
    let ModelCheckRefusal::Liveness(LivenessRefusal::ComponentCertificateNotBuilt { stats }) =
        refusal
    else {
        panic!("{refusal:?}");
    };
    assert!(stats.product_states > 0, "{stats:?}");
}

/// Part of FR-212-AC-2 (TC-657), and FR-217-AC-4 (TC-662): with `scheduling
/// adversarial` the fairness set is empty and the same claim is refuted by
/// the lasso whose stem is `fork(Both)`, `send(Tx)` and whose loop is
/// `duplicate(ch, Tx)`, `lose(ch, Tx)`, `s` resting at `Done` throughout;
/// the lasso replays against the adversarial package, so the claim settles
/// `refuted`. The same lasso with the default package's scheduler
/// constraints as its fairness set, replayed against the default package,
/// refuses `invalid_runtime_input`/`invalid-value` as unfair: `s` is enabled
/// at every loop state and takes no step, and so is `r`, whose constraint
/// comes first in the set. Not
/// covered: the two packages' different identities. The compiled
/// `package_id` does not bind the `scheduling` member until FR-383 emits
/// it, so today the two identities are equal, and replay tells the two
/// packages' counterexamples apart by their fairness sets alone.
#[trace("TC-657", "TC-662", "FR-217-AC-4")]
#[test]
fn scheduling_adversarial_admits_the_lasso_that_starves_a_branch() {
    let adversarial = Chatter::new("scheduling adversarial;", EVENTUALLY, &IS_ONE);
    let fair = Chatter::new("", EVENTUALLY, &IS_ONE);
    assert!(adversarial.fairness().is_empty());
    let outcome = adversarial.check().expect("the protocol subject checks");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    let lasso = counterexample.lasso.expect("a lasso");
    assert!(!lasso.terminal_stutter);
    assert!(counterexample.fairness.is_empty());
    let (stem, cycle) = counterexample.prefix.split_at(lasso.entry);
    assert_eq!(
        shape(stem),
        [
            (StepKind::Fork, Some("Both".to_owned())),
            (StepKind::Send, Some("Tx".to_owned())),
        ]
    );
    assert_eq!(
        shape(cycle),
        [(StepKind::Duplicate, None), (StepKind::Lose, None)]
    );
    for step in cycle {
        assert_eq!(step.transition.channel_queue(), Some(("Chatter::ch", 0)));
    }
    // `s` rests at `Done` throughout: its attempt is enabled at every loop
    // state and no loop step is its.
    let subject = adversarial.subject();
    let system = sc_system(&subject);
    let s = ThreadPath::root(0).child("Both", "s", 0);
    let mut state = system.initial().remove(0);
    let mut loop_states = Vec::new();
    for (index, step) in counterexample.prefix.iter().enumerate() {
        if index >= lasso.entry {
            loop_states.push(state.clone());
        }
        let (_, next) = system
            .successors(&state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, next)| {
                *transition == step.transition
                    && qsl_eval::simulation::SubjectSystem::state_key(&system, next)
                        .expect("a key")
                        .1
                        == step.post_state
            })
            .expect("the step replays");
        state = next;
    }
    for at in &loop_states {
        let enabled = system.successors(at).expect("expands").successors;
        assert!(enabled.iter().any(|(transition, _)| {
            transition.kind() == StepKind::Attempt && transition.thread() == Some(&s)
        }));
    }

    let record = adversarial.settle(&outcome);
    assert_eq!(record.label, ResultLabel::Refuted, "{record:?}");
    let replayed = adversarial
        .replay(counterexample.as_ref().clone())
        .expect("the lasso replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );

    let mut scheduled = counterexample.as_ref().clone();
    scheduled.fairness = fair.fairness();
    let Err(ReplayRefusal::ModelTrace(refusal)) = fair.replay(scheduled) else {
        panic!("an unfair lasso refuses");
    };
    let ModelTraceRefusal::UnfairProtocolLasso { constraint } = refusal.as_ref() else {
        panic!("{refusal:?}");
    };
    // `r`, whose `receive` the queued `Tx` keeps enabled, starves too; its
    // member is the first of the set in JCS order that the loop fails.
    assert_eq!(
        **constraint,
        ProtocolFairness::scheduler(ProtocolTarget::branch("Both", "r"))
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, "invalid-value")
    );
}

/// FR-217 "Behavior": a payload whose fairness set differs from the one the
/// recompiled clause and protocol give, here the adversarial lasso with no
/// constraint replayed against the default package, refuses
/// `stale_dependency`/`content-mismatch`, naming both sets.
#[trace("TC-662")]
#[test]
fn a_protocol_payload_without_the_scheduler_constraints_refuses() {
    let adversarial = Chatter::new("scheduling adversarial;", EVENTUALLY, &IS_ONE);
    let fair = Chatter::new("", EVENTUALLY, &IS_ONE);
    let ModelCheckOutcome::Violated { counterexample, .. } =
        adversarial.check().expect("the subject checks")
    else {
        panic!("a violation");
    };
    let Err(ReplayRefusal::ModelTrace(refusal)) = fair.replay(*counterexample) else {
        panic!("a stale fairness set refuses");
    };
    assert_eq!(
        *refusal,
        ModelTraceRefusal::ProtocolFairnessSet {
            payload: Vec::new(),
            recompiled: fair.fairness(),
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::StaleDependency, "content-mismatch")
    );
}

/// FR-212-AC-4 (TC-657): the TP-1 claim `always holds(k.v <= 1)` over
/// `Chatter` settles `proved` with and without `scheduling adversarial`,
/// over the same reachable states: the scheduler constraints change no
/// safety verdict.
#[trace("TC-657", "FR-212-AC-4")]
#[test]
fn a_safety_verdict_is_the_same_with_and_without_the_scheduler() {
    let mut explored = Vec::new();
    for members in ["", "scheduling adversarial;"] {
        let chatter = Chatter::new(members, BOUNDED, &AT_MOST_ONE);
        let outcome = chatter.check().expect("the subject checks");
        let ModelCheckOutcome::Holds { certificate, stats } = &outcome else {
            panic!("{members}: {outcome:?}");
        };
        let record = chatter.settle(&outcome);
        assert_eq!(record.label, ResultLabel::Proved, "{members}: {record:?}");
        explored.push((certificate.clone(), *stats));
    }
    assert_eq!(explored[0], explored[1]);
}

/// The S3 refusal of the unit `body`, which must not check.
fn check_refusal(body: &str) -> CheckRefusal {
    let document = document();
    let text = unit_text(body);
    let packages = package_input([document.as_slice()]);
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "counter", "git", "1"),
        "counter.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses the unit");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the model selection admits")
    .with_parsed_operation_clauses(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    })
    .expect("the operation clauses parse");
    let mut refusals = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(CheckingLimits::default())
    .expect_err("the unit refuses");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    refusals.remove(0)
}

/// The temporal cause of `refusal`.
fn temporal(refusal: &CheckRefusal) -> &TemporalCause {
    match &refusal.cause {
        CheckCause::Temporal(cause) => cause,
        other => panic!("{other:?}"),
    }
}

/// `Chatter`'s liveness clause with the fairness constraint `fairness`.
fn with_constraint(members: &str, fairness: &str) -> String {
    chatter_body(
        members,
        &format!(
            "temporal Eventually using t0 over (k: M::Cell) clock \"model-steps\" \
             on origin {{ {fairness} eventually holds(k.v = 1) }}\n"
        ),
    )
}

/// FR-212-AC-3 (TC-657): over `Chatter`, `fair weak whole Chatter::Both::s`
/// names the branch `s` of `Both` and `fair weak each Chatter::Done` the
/// attempt node `Done` (QSpec FR-430's spelling of FR-212-AC-3's `s` and
/// `Done`); S4 has no member to emit a protocol target with, so each
/// refuses `unsupported_construct`/`not-yet-implemented` at the constraint,
/// naming the target it resolved to. `fair weak Missing` refuses
/// `missing_declaration`/`missing-name`, and a second `scheduling` member
/// `ambiguous_declaration`/`ambiguous-name`. Over ADR-027 §7's `Fill`, the
/// root's `whole` class holds `fork(Both)`, `join(Both)` and `finish`, and
/// `left`'s holds `attempt(A)`.
#[trace("TC-657", "FR-212-AC-3")]
#[test]
fn protocol_targets_resolve_and_classes_hold_their_threads_steps() {
    for (fairness, expected, granularity) in [
        (
            "fair weak whole Chatter::Both::s;",
            CheckedProtocolTarget::Branch {
                parallel: "Both".to_owned(),
                branch: "s".to_owned(),
            },
            FairnessGranularity::Whole,
        ),
        (
            "fair weak each Chatter::Done;",
            CheckedProtocolTarget::AttemptNode {
                node: "Both/s/Sender/Done".to_owned(),
            },
            FairnessGranularity::Each,
        ),
    ] {
        let refusal = check_refusal(&with_constraint("", fairness));
        let TemporalCause::ProtocolFairnessNotEmitted { constraint, .. } = temporal(&refusal)
        else {
            panic!("{fairness}: {refusal:?}");
        };
        assert_eq!(
            (
                &constraint.protocol,
                &constraint.target,
                constraint.granularity
            ),
            (&"Chatter".to_owned(), &expected, granularity),
            "{fairness}"
        );
        assert_eq!(constraint.kind, FairnessKind::Weak);
        assert_eq!(
            (refusal.cause.code(), refusal.cause.cause()),
            (Code::UnsupportedConstruct, Some("not-yet-implemented"))
        );
    }
    let missing = check_refusal(&with_constraint("", "fair weak Missing;"));
    assert!(
        matches!(temporal(&missing), TemporalCause::UnresolvedFairnessOperation { name, .. } if name == "Missing"),
        "{missing:?}"
    );
    assert_eq!(
        (missing.cause.code(), missing.cause.cause()),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    let twice = check_refusal(&chatter_body(
        "scheduling adversarial; scheduling adversarial;",
        EVENTUALLY,
    ));
    assert_eq!(
        (twice.cause.code(), twice.cause.cause()),
        (Code::AmbiguousDeclaration, Some("ambiguous-name")),
        "{twice:?}"
    );

    let fill = crate::protocol_system::Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let subject = fill.subject("Fill");
    let system = sc_system(&subject);
    let root = ProtocolFairness::scheduler(ProtocolTarget::Root);
    let left = ProtocolFairness::scheduler(ProtocolTarget::branch("Main/Both", "left"));
    let mut classes: [BTreeSet<(StepKind, String)>; 2] = Default::default();
    for state in reachable(&system) {
        for (transition, _) in &state.steps {
            for (class, member) in classes.iter_mut().zip([&root, &left]) {
                if member.class_of(transition) == Some(ProtocolClass::Whole) {
                    class.insert((
                        transition.kind(),
                        transition.node().unwrap_or_default().to_owned(),
                    ));
                }
            }
        }
    }
    let [root_class, left_class] = classes;
    assert_eq!(
        root_class,
        BTreeSet::from([
            (StepKind::Fork, "Main/Both".to_owned()),
            (StepKind::Join, "Main/Both".to_owned()),
            (StepKind::Finish, "Done".to_owned()),
        ])
    );
    assert_eq!(
        left_class,
        BTreeSet::from([(StepKind::Attempt, "Main/Both/left/A".to_owned())])
    );
}
