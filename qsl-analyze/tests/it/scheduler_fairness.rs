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
use qsl_analyze::model_check::{check_protocol, ModelCheckRefusal};
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ProtocolSubject};
use qsl_eval::simulation::model::{ExpansionLimits, ModelSimState};
use qsl_eval::simulation::protocol::{
    FairnessOrigin, MemorySelection, ProtocolClass, ProtocolFairness, ProtocolTarget,
    ProtocolTransition, ResolvedMemory, StepKind, ThreadPath,
};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::AtomReading;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::MemoryModelName;
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_replay::{
    replay_protocol_trace, settle_protocol_check, ModelCheckItem, ModelCheckLimits,
    ModelCheckOutcome, ModelCheckRecord, ModelTraceRefusal, ModelTraceReplayResult,
    ModelTraceSettlement, ModelTraceSubject, ProofCertificate, ProtocolCertificateRequest,
    ProtocolTraceSubject, ReplayLimits, ReplayRefusal, ResultLabel, TemporalCounterexample,
};
use qsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedProtocolTarget, PackageDeclarations, ProtocolAnchorCause,
    TemporalCause,
};
use qsl_semantics::check::{CheckedTemporalClause, FormulaNodeId};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Cancel, Meter};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use serde_json::Value;

use crate::fill::{
    attempt, contracts, document, fill_unit, instance, package_with, package_with_name, protocol,
    reachable, sc_system, subject, unit_text, universe, v, Documents,
};
use crate::model_settle::{envelope_at, request_named};

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
    ) -> Result<AtomReading, qsl_eval::model_subject::AtomStop> {
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
    package_name: String,
    body: String,
    documents: Documents,
    initial: Vec<DocumentRef>,
    checked: CheckedPackage,
    letters: &'static AtV,
    clause_index: usize,
}

impl Chatter {
    /// The unit with `members` and the clause `claim`, read by `letters`.
    fn new(members: &str, claim: &str, letters: &'static AtV) -> Self {
        let body = chatter_body(members, claim);
        Self::with_body("package", body, 0, letters)
    }

    fn with_prefix(prefix: &str, members: &str, claim: &str, letters: &'static AtV) -> Self {
        let body = chatter_body(members, &format!("{prefix}{claim}"));
        Self::with_body("other-package", body, 1, letters)
    }

    fn with_body(
        package_name: &str,
        body: String,
        clause_index: usize,
        letters: &'static AtV,
    ) -> Self {
        let mut documents = Documents::new();
        let initial = vec![documents.start("s0", &[("c", 0)])];
        let checked = package_with_name(package_name, crate::fill::PROTOCOL_PROFILE, &body);
        Self {
            package_name: package_name.to_owned(),
            body,
            documents,
            initial,
            checked,
            letters,
            clause_index,
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
        self.checked.graph().temporal_clauses()[self.clause_index].clause()
    }

    fn item(&self) -> ModelCheckItem<'_> {
        ModelCheckItem::Clause {
            clause: self.clause(),
            letters: self.letters,
        }
    }

    /// The clause's fairness set over the subject (FR-212).
    fn fairness(&self) -> Vec<ProtocolFairness> {
        let targets: Vec<_> = self
            .clause()
            .protocol_fairness()
            .map(|(constraint, _)| constraint.clone())
            .collect();
        sc_system(&self.subject())
            .fairness_set_with_targets(self.clause().fairness(), &targets)
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
                max_messages: None,
                max_live_instances: None,
                limits: ModelCheckLimits::default(),
                ignore_memory_constraints: false,
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
        let node = &self.checked.graph().temporal_clauses()[self.clause_index];
        let clause_node = node.identity().clone();
        let occurrence = OccurrenceKey::new(clause_node.clone(), node.claim().clone());
        let subject = self.subject();
        replay_protocol_trace(
            request_named(
                &self.package_name,
                &document(),
                self.documents.snapshots.iter(),
                &unit,
            ),
            &ProtocolTraceSubject {
                model: ModelTraceSubject {
                    timed: None,
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
                max_messages: None,
                max_live_instances: None,
            },
            &envelope_at(payload, clause_node, occurrence),
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
            &ProtocolCertificateRequest {
                subject: self.subject(),
                item: self.item(),
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
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
/// `s`'s scheduler constraint, so the run holds with a component certificate
/// after exploring the whole product.
#[trace("TC-657", "FR-212-AC-1")]
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
    let outcome = chatter.check().expect("the subject checks");
    let ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Component(_),
        stats,
    } = &outcome
    else {
        panic!("{outcome:?}");
    };
    assert!(stats.product_states > 0, "{stats:?}");
    let record = chatter.settle(&outcome);
    assert_eq!(record.label, ResultLabel::Proved, "{record:?}");
}

/// Part of FR-212-AC-2 (TC-657), and FR-217-AC-4 (TC-662): with `scheduling
/// adversarial` the fairness set is empty and the same claim is refuted by
/// the lasso whose stem is `fork(Both)`, `send(Tx)` and whose loop is
/// `duplicate(ch, Tx)`, `lose(ch, Tx)`, `s` resting at `Done` throughout;
/// the lasso replays against the adversarial package, so the claim settles
/// `refuted`. The two compiled clause identities differ, and the exact
/// `Done` attempt identity stays enabled at every loop state without
/// appearing in the loop. The same lasso with the default package's scheduler
/// constraints as its fairness set, replayed against the default package,
/// refuses `invalid_runtime_input`/`invalid-value` as unfair: `s` is enabled
/// at every loop state and takes no step, and so is `r`, whose constraint
/// comes first in the set.
#[trace("TC-657", "TC-662", "FR-212-AC-2", "FR-217-AC-4")]
#[test]
fn scheduling_adversarial_admits_the_lasso_that_starves_a_branch() {
    let adversarial = Chatter::new("scheduling adversarial;", EVENTUALLY, &IS_ONE);
    let fair = Chatter::with_prefix(BOUNDED, "", EVENTUALLY, &IS_ONE);
    let adversarial_package = adversarial
        .checked
        .graph()
        .protocol_node_keys("Chatter")
        .and_then(|keys| keys.clause.clone())
        .expect("emitted protocol clause");
    let fair_package = fair
        .checked
        .graph()
        .protocol_node_keys("Chatter")
        .and_then(|keys| keys.clause.clone())
        .expect("emitted protocol clause");
    assert_ne!(
        adversarial_package, fair_package,
        "distinct package identities"
    );
    let adversarial_node = &adversarial.checked.graph().temporal_clauses()[0];
    let fair_node = &fair.checked.graph().temporal_clauses()[1];
    assert_ne!(adversarial_node.identity(), fair_node.identity());
    assert_ne!(
        OccurrenceKey::new(
            adversarial_node.identity(),
            adversarial_node.claim().clone()
        ),
        OccurrenceKey::new(fair_node.identity(), fair_node.claim().clone()),
        "distinct obligation identities from different clause occurrences"
    );
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
            transition.kind() == StepKind::Attempt
                && transition.thread() == Some(&s)
                && transition.node() == Some("Both/s/Sender/Done")
        }));
    }
    assert!(cycle
        .iter()
        .all(|step| step.transition.node() != Some("Both/s/Sender/Done")));

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
pub(crate) fn check_refusal(body: &str) -> CheckRefusal {
    let document = document();
    let text = unit_text(body);
    let packages = package_input([document.as_slice()]).unwrap();
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
        "package".to_owned(),
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
pub(crate) fn temporal(refusal: &CheckRefusal) -> &TemporalCause {
    match &refusal.cause {
        CheckCause::Temporal(cause) => cause,
        other => panic!("{other:?}"),
    }
}

/// `Chatter`'s liveness clause with the fairness constraint `fairness`.
fn with_constraint(members: &str, fairness: &str) -> String {
    chatter_body(members, &eventually_with(fairness))
}

/// `EVENTUALLY` with the fairness constraint `fairness`.
fn eventually_with(fairness: &str) -> String {
    format!(
        "temporal Eventually using t0 over (k: M::Cell) clock \"model-steps\" \
         on origin {{ {fairness} eventually holds(k.v = 1) }}\n"
    )
}

/// FR-212-AC-3 (TC-657): over `Chatter`, branch and attempt targets survive
/// S3 and S4 with their checked node identities. `fair weak Missing` refuses
/// `missing_declaration`/`missing-name`, and a second `scheduling` member
/// `ambiguous_declaration`/`ambiguous-name`.
#[trace("TC-657", "FR-212-AC-3")]
#[test]
fn protocol_targets_resolve_and_classes_hold_their_threads_steps() {
    let branch = Chatter::new(
        "",
        &eventually_with("fair weak whole Chatter::Both::s;"),
        &IS_ONE,
    );
    let protocol = branch
        .checked
        .graph()
        .protocol("Chatter")
        .expect("protocol");
    let parallel = protocol
        .controls
        .controls
        .iter()
        .find(|control| control.name == "Both")
        .expect("Both");
    let fairness: Vec<_> = branch.clause().protocol_fairness().collect();
    assert_eq!(fairness.len(), 1);
    assert_eq!(
        fairness[0].0.target,
        CheckedProtocolTarget::Branch {
            parallel: parallel.node,
            branch: "s".to_owned(),
        }
    );
    assert!(branch.fairness().iter().any(|member| matches!(
        member.target(),
        ProtocolTarget::Branch { parallel: path, branch } if path == "Both" && branch == "s"
    )));
    let branch_emission =
        qsl_package::emit_checked(&branch.checked).expect("typed branch fairness emits");
    let branch_wire = branch_emission.package().bytes();
    assert!(String::from_utf8_lossy(branch_wire).contains("parallel_branch"));

    let attempt = Chatter::new(
        "",
        &eventually_with("fair weak each Chatter::Done;"),
        &IS_ONE,
    );
    let protocol = attempt
        .checked
        .graph()
        .protocol("Chatter")
        .expect("protocol");
    let done = protocol
        .controls
        .controls
        .iter()
        .find(|control| control.name == "Done")
        .expect("Done");
    assert_eq!(
        attempt
            .clause()
            .protocol_fairness()
            .next()
            .expect("target")
            .0
            .target,
        CheckedProtocolTarget::AttemptNode { node: done.node }
    );
    assert!(attempt.fairness().iter().any(|member| matches!(
        member.target(),
        ProtocolTarget::AttemptNode { node } if node == "Both/s/Sender/Done"
    )));
    let attempt_emission =
        qsl_package::emit_checked(&attempt.checked).expect("typed attempt fairness emits");
    let attempt_wire = attempt_emission.package().bytes();
    assert!(String::from_utf8_lossy(attempt_wire).contains("attempt_node"));
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
    let CheckCause::ProtocolAnchor(anchor) = &twice.cause else {
        panic!("{twice:?}");
    };
    let ProtocolAnchorCause::Ambiguous { name, loci, span } = anchor.as_ref() else {
        panic!("{twice:?}");
    };
    let text = unit_text(&chatter_body(
        "scheduling adversarial; scheduling adversarial;",
        EVENTUALLY,
    ));
    let first = text.find("scheduling").expect("a first member");
    assert_eq!(name, "scheduling");
    assert_eq!(loci.len(), 2, "{loci:?}");
    assert_eq!(loci[0].start, first, "the first member is named");
    assert_eq!(loci[1], *span, "the refusal sits at the second");
    assert!(span.start > first);

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

/// FR-123: operation and protocol-target fairness members lower in the
/// source order of their first occurrence when the forms are interleaved.
#[trace("TC-657", "FR-123")]
#[test]
fn interleaved_operation_and_protocol_fairness_preserves_source_order() {
    let chatter = Chatter::new(
        "",
        &eventually_with(
            "fair weak each Chatter::Done; fair strong each M::Cell::inc; fair weak whole Chatter::Both::s;",
        ),
        &IS_ONE,
    );
    let emitted = qsl_package::emit_checked(&chatter.checked).expect("mixed fairness emits");
    let wire: Value = serde_json::from_slice(emitted.package().bytes()).expect("wire JSON");
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes");
    let clause = nodes
        .iter()
        .find(|node| node["semantic_form"] == "temporal_clause")
        .expect("temporal clause");
    let members = clause["body"]["arguments"][4]["members"]
        .as_array()
        .expect("fairness members");
    let targets: Vec<_> = members
        .iter()
        .map(|reference| {
            let target_id = &reference["target"];
            let node = nodes
                .iter()
                .find(|node| &node["node_id"] == target_id)
                .expect("fairness node");
            let target = &node["body"]["operation"]["member"]["target"];
            match target["kind"].as_str().expect("target kind") {
                "attempt_node" => "attempt:Done".to_owned(),
                "operation" => format!("operation:{}", target["name"].as_str().unwrap()),
                "parallel_branch" => "parallel_branch".to_owned(),
                other => panic!("unexpected target kind {other}"),
            }
        })
        .collect();
    assert_eq!(
        targets,
        ["attempt:Done", "operation:inc", "parallel_branch",]
    );
}

/// FR-212: compensation targets resolve from the authored template, emit as
/// a typed target, and classify actual compensation transitions.
#[trace("TC-657", "FR-212")]
#[test]
fn compensation_fairness_target_emits_and_covers_transitions() {
    let template =
        crate::protocol_compensation::refund("Main::Paid", "dec", 1, "false", "never", "true");
    let run = crate::protocol_compensation::pay_run("");
    let claim = eventually_with("fair weak each Pay::Refund;");
    let body = contracts("self.v = 0")
        + crate::protocol_compensation::REFUND
        + &protocol("Pay", &template, &run, "d")
        + &claim;
    let mut documents = Documents::new();
    let initial = vec![documents.start("s0", &[("c", 0)])];
    let checked = package_with(crate::fill::PROTOCOL_PROFILE, &body);
    let pay_subject = subject(&checked, &documents, initial.clone(), "Pay", "c");
    let clause = checked.graph().temporal_clauses()[0].clause();
    let targets: Vec<_> = clause
        .protocol_fairness()
        .map(|(constraint, _)| constraint.clone())
        .collect();
    assert!(matches!(
        targets.first().map(|target| &target.target),
        Some(CheckedProtocolTarget::CompensationTemplate { .. })
    ));
    let system = sc_system(&pay_subject);
    let fairness = system
        .fairness_set_with_targets(clause.fairness(), &targets)
        .expect("the compensation target resolves");
    assert!(fairness.iter().any(|member| matches!(
        member.target(),
        ProtocolTarget::CompensationTemplate { template } if template == "Pay::Refund"
    ) && member.origin() == FairnessOrigin::Authored));
    assert!(fairness.iter().any(|member| matches!(
        member.target(),
        ProtocolTarget::CompensationTemplate { template } if template == "Pay::Refund"
    ) && member.origin() == FairnessOrigin::Scheduler));
    let emission = qsl_package::emit_checked(&checked).expect("typed compensation fairness emits");
    let wire = emission.package().bytes();
    let wire: Value = serde_json::from_slice(wire).expect("emitted checked-package JSON");
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("emitted semantic graph");
    let fairness_node = nodes
        .iter()
        .find(|node| {
            node["semantic_form"] == "fairness"
                && node["body"]["operation"]["member"]["target"]["kind"] == "compensation_template"
        })
        .expect("typed compensation target in the package");
    let target = &fairness_node["body"]["operation"]["member"]["target"];
    assert!(nodes.iter().any(|node| node["node_id"] == target["node"]));
    let compensation = fairness
        .iter()
        .find(|member| {
            member.origin() == FairnessOrigin::Authored
                && matches!(member.target(), ProtocolTarget::CompensationTemplate { .. })
        })
        .expect("authored compensation fairness");
    let scheduler_compensation = fairness
        .iter()
        .find(|member| {
            member.origin() == FairnessOrigin::Scheduler
                && matches!(member.target(), ProtocolTarget::CompensationTemplate { .. })
        })
        .expect("scheduler compensation fairness");
    let reached = reachable(&system);
    assert!(reached
        .iter()
        .any(|state| state.steps.iter().any(|(step, _)| {
            matches!(step.kind(), StepKind::CAttempt | StepKind::CEnd)
                && compensation.class_of(step).is_some()
                && scheduler_compensation.class_of(step).is_some()
        })));
}

/// Exercise compensation fairness in liveness filtering and replay. IR
/// admission requires the fairness-bearing temporal claim in the protocol's
/// requirements, but QSL lowering currently refuses the resulting reference
/// cycle because that claim targets a node inside the same protocol.
#[trace("TC-657", "FR-212", "FR-217-AC-4")]
#[ignore = "blocked by cyclic protocol requirement/fairness-target admission join"]
#[test]
fn authored_compensation_fairness_filters_and_replay_rejects_a_lasso() {
    let template =
        crate::protocol_compensation::refund("Main::Paid", "dec", 1, "false", "never", "true");
    let name = "counter";
    let channel = "channel ch from w to w carries Boolean ordering unordered \
                   delivery unknown capacity 2 overflow block;";
    let members = format!("{channel}\n{template}\nscheduling adversarial;");
    let run = format!(
        "sequence Main {{ {} {} send Tx via ch as (tx: Boolean) {{ tx }}; {} }}",
        crate::protocol_compensation::CHARGE,
        crate::protocol_compensation::paid("Main::C"),
        crate::protocol_compensation::FAIL
    );
    let liveness = "temporal Unfair using t0 over (k: M::Cell) clock \"model-steps\" \
                    on origin { always eventually holds(k.v = 0) }\n\
                    temporal Fair using t0 over (k: M::Cell) clock \"model-steps\" \
                    on origin { fair weak each Pay::Refund; always eventually holds(k.v = 0) }\n";
    let body = contracts("self.v = 0")
        + crate::protocol_compensation::REFUND
        + &protocol("Pay", &members, &run, "true")
        + liveness;
    let mut documents = Documents::new();
    let initial = vec![documents.start("s0", &[("c", 0)])];
    let checked = package_with(crate::fill::PROTOCOL_PROFILE, &body);
    let clauses = checked.graph().temporal_clauses();
    assert_eq!(clauses.len(), 2);
    let unfair_clause = clauses[0].clause();
    let fair_node = &clauses[1];
    let fair_clause = fair_node.clause();
    let letters = AtV(|value| value == 0);
    let run_clause = |clause: &CheckedTemporalClause| {
        check_protocol(
            ProtocolCheckRequest {
                subject: subject(&checked, &documents, initial.clone(), "Pay", "c"),
                item: ModelCheckItem::Clause {
                    clause,
                    letters: &letters,
                },
                max_depth: None,
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
                limits: ModelCheckLimits::default(),
                ignore_memory_constraints: false,
            },
            &Cancel::new(),
        )
    };
    let ModelCheckOutcome::Violated {
        counterexample: lasso,
        ..
    } = run_clause(unfair_clause).expect("the unauthored claim checks")
    else {
        panic!("an unfair compensation lasso should refute the claim");
    };
    assert!(lasso.lasso.is_some(), "{lasso:?}");
    let ModelCheckOutcome::Holds { .. } = run_clause(fair_clause).expect("the fair claim checks")
    else {
        panic!("authored compensation fairness should filter its lasso");
    };

    let package_subject = subject(&checked, &documents, initial.clone(), "Pay", "c");
    let replay_targets: Vec<_> = fair_clause
        .protocol_fairness()
        .map(|(constraint, _)| constraint.clone())
        .collect();
    let replay_fairness = sc_system(&package_subject)
        .fairness_set_with_targets(fair_clause.fairness(), &replay_targets)
        .expect("the authored target fairness set builds");
    let mut replay_payload = lasso.as_ref().clone();
    replay_payload.fairness = replay_fairness;
    let occurrence = OccurrenceKey::new(fair_node.identity().clone(), fair_node.claim().clone());
    let envelope = envelope_at(replay_payload, fair_node.identity().clone(), occurrence);
    let unit = unit_text(&body).into_bytes();
    let replayed = replay_protocol_trace(
        request_named(name, &document(), documents.snapshots.iter(), &unit),
        &ProtocolTraceSubject {
            model: ModelTraceSubject {
                timed: None,
                initial,
                universes: universe(&["c"]),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
            },
            protocol: "Pay".to_owned(),
            over: instance("c"),
            roles: package_subject.roles.clone(),
            memory: package_subject.memory.clone(),
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        },
        &envelope,
        &letters,
        ReplayLimits::default(),
    );
    let Err(ReplayRefusal::ModelTrace(refusal)) = replayed else {
        panic!("the lasso is refused by authored compensation fairness: {replayed:?}");
    };
    assert!(matches!(
        refusal.as_ref(),
        ModelTraceRefusal::UnfairProtocolLasso { .. }
    ));
}

/// FR-212 Outputs (TC-657): an authored operation constraint over a protocol
/// subject enters the fairness set beside the three scheduler constraints,
/// with origin `authored` and the written strength and granularity, and the
/// set stays in ascending JCS order.
#[trace("TC-657")]
#[test]
fn an_authored_operation_constraint_joins_the_scheduler_constraints() {
    let chatter = Chatter::new(
        "",
        &eventually_with("fair strong each M::Cell::inc;"),
        &IS_ONE,
    );
    let fairness = chatter.fairness();
    assert_eq!(fairness.len(), 4, "{fairness:?}");
    let [authored] = fairness
        .iter()
        .filter(|member| member.origin() == FairnessOrigin::Authored)
        .collect::<Vec<_>>()[..]
    else {
        panic!("one authored member: {fairness:?}");
    };
    assert_eq!(
        (authored.kind(), authored.granularity()),
        (FairnessKind::Strong, FairnessGranularity::Each)
    );
    assert!(
        matches!(authored.target(), ProtocolTarget::Operation { .. }),
        "{authored:?}"
    );
    let keys: Vec<Vec<u8>> = fairness
        .iter()
        .map(|member| {
            quire_canonical::to_vec(member, quire_canonical::Limits::new(4096))
                .expect("a member encodes")
        })
        .collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "{keys:?}");
}

/// FR-228: a liveness clause over a protocol whose `Both` runs under `ra`
/// refuses `unsupported_construct`/`not-yet-implemented`: the engine reads
/// no visibility fairness, and a stale read would give a false
/// counterexample.
#[trace("TC-657")]
#[test]
fn liveness_over_an_ra_parallel_refuses() {
    let chatter = Chatter::new("", EVENTUALLY, &IS_ONE);
    let mut subject = chatter.subject();
    let checked = chatter
        .checked
        .graph()
        .protocol("Chatter")
        .expect("the protocol");
    subject.memory = ResolvedMemory::resolve(
        checked,
        &[MemorySelection {
            parallel: "Chatter::Both".to_owned(),
            model: MemoryModelName::Ra,
        }],
    )
    .expect("the selection resolves");
    let refusal = check_protocol(
        ProtocolCheckRequest {
            subject,
            item: chatter.item(),
            max_depth: None,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
            limits: ModelCheckLimits::default(),
            ignore_memory_constraints: false,
        },
        &Cancel::new(),
    )
    .expect_err("an ra liveness clause refuses");
    assert!(
        matches!(refusal, ModelCheckRefusal::WeakLiveness),
        "{refusal:?}"
    );
}
