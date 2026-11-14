// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-571, FR-572, FR-574, FR-575 (TC-1411, TC-1412, TC-1414, TC-1415):
//! deciding realizability over the `qsl-eval` arena (ADR-036 slice 4).

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_analyze::analyze::{analyze_items, AnalyzeRecord, Engines};
use qsl_analyze::protocol_analysis::{
    self, projection_item, realizability_item, realize, realize_tampered, replay_counterexample,
    replay_strategy, ProjectionRequest, RealizabilityOutcome, RealizabilityRefusal,
    RealizabilityRequest,
};
use qsl_eval::model_subject::{ProtocolSubject, RoleBinding};
use qsl_eval::simulation::protocol::StepKind;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_package::CheckedPackage;
use qsl_replay::protocol_analysis::{
    node_text, AnalysisOutcome, AssessedRoot, BoundsDraft, InconclusiveCause, LostAction, Observed,
    Play, ProjectionBoundsDraft, RealizabilityCounterexample, RealizabilityDisagreement,
    RealizabilityWitness, RunBudget, RunLimit, StrategyAction, UnsupportedSlug,
};
use qsl_replay::{
    Certification, DecidingElement, Execution, IncompleteCause, InconclusiveCause as Fr331Cause,
    ModelCheckLimit, ProofBasis, ResultLabel, SeparatingWitnessRecord, SettlementBasis,
    TerminalValue, Truth, UnavailabilityCause,
};
use qsl_route::{
    settle_candidates, Candidate, CandidateOutcome, NegotiationCause, NegotiationSettlement,
    Registry,
};
use qsl_semantics::check::Capability;
use qsl_semantics::model::observation::DocumentRef;
use quire_contract_model::{CheckedNodeId, CheckedPackageV2};
use quire_exact::Cancel;

use crate::fill::{instance, model_subject, package, source_memory, Documents, PACKAGE};
use crate::protocol_projection::{admitted_source, clause, named, root as assessed_root};

const RECORDS: &str = "record Ack { ok: Boolean; }\nrecord PickMsg { ok: Boolean; }\n\
    record CancelMsg { ok: Boolean; }\nrecord Txt { s: String; }\n";

fn channel(name: &str, from: &str, to: &str, carries: &str, delivery: &str) -> String {
    format!(
        "channel {name} from {from} to {to} carries {carries} ordering unordered delivery \
         {delivery} capacity 4 overflow reject;\n"
    )
}

/// The members of the fixtures, with `delivery` on the `Pick` and
/// `Cancelled` channels.
fn members(delivery: &str) -> String {
    [
        "role Buyer on M::Cell;\nrole Seller on M::Cell;\nrole Warehouse on M::Cell;\n".to_owned(),
        channel("Order", "Buyer", "Seller", "Ack", "exactly-once-premise"),
        channel("Tell", "Seller", "Buyer", "Ack", "exactly-once-premise"),
        channel("Pick", "Seller", "Warehouse", "PickMsg", delivery),
        channel("Cancelled", "Seller", "Warehouse", "CancelMsg", delivery),
        channel("Words", "Buyer", "Seller", "Txt", "exactly-once-premise"),
    ]
    .concat()
}

/// One fixture: both package forms and its documents.
struct Fixture {
    checked: CheckedPackage,
    admitted: Box<CheckedPackageV2>,
    documents: Documents,
    initial: Vec<DocumentRef>,
}

impl Fixture {
    fn new(delivery: &str, run: &str) -> Self {
        Self::over(delivery, "", run, &[0])
    }

    /// A fixture whose protocol declares `captures` first, with one initial
    /// state per value of the cell `c` in `values`.
    fn over(delivery: &str, captures: &str, run: &str, values: &[i64]) -> Self {
        Self::activated("on origin", delivery, captures, run, values)
    }

    /// [`Self::over`] with the protocol's `activation`.
    fn activated(
        activation: &str,
        delivery: &str,
        captures: &str,
        run: &str,
        values: &[i64],
    ) -> Self {
        let text = format!(
            "{RECORDS}protocol Flow using pp over (k: M::Cell) {activation} {{\n{captures}{}\nrun {run}\n\
             finish Done as (zz: Boolean) {{ true }};\n}}\n",
            members(delivery)
        );
        let mut documents = Documents::new();
        let initial = values
            .iter()
            .enumerate()
            .map(|(at, value)| documents.start(&format!("s{at}"), &[("c", *value)]))
            .collect();
        Self {
            checked: package(&text),
            admitted: admitted_source(&text),
            documents,
            initial,
        }
    }

    fn subject(&self) -> ProtocolSubject<'_> {
        ProtocolSubject {
            model: model_subject(&self.checked, &self.documents, self.initial.clone(), &["c"]),
            protocol: "Flow".to_owned(),
            over: instance("c"),
            roles: ["Buyer", "Seller", "Warehouse"]
                .iter()
                .map(|role| RoleBinding {
                    role: (*role).to_owned(),
                    model: PACKAGE.to_owned(),
                })
                .collect(),
            memory: source_memory(&self.checked, "Flow"),
        }
    }

    fn request(&self, max_depth: u64) -> RealizabilityRequest<'_> {
        RealizabilityRequest {
            protocol: clause(&self.admitted),
            subject: self.subject(),
            definition_key: Some("ix://test/Flow".to_owned()),
            strategy_profile: Some("bounded-static".to_owned()),
            role_knowledge: Some(Some(BTreeSet::new())),
            environment: Some(Some(BTreeSet::new())),
            bounds: Some(BoundsDraft {
                max_depth: Some(max_depth),
                max_history: Some(16),
            }),
            assessed_root: Some(assessed_root()),
            budget: RunBudget::default(),
        }
    }

    fn decide(&self, request: RealizabilityRequest<'_>) -> RealizabilityOutcome {
        realize(&self.admitted, request).expect("the request is admitted")
    }

    fn run(&self) -> RealizabilityOutcome {
        self.decide(self.request(64))
    }

    fn named(&self, last: &str) -> CheckedNodeId {
        named(&self.admitted, last)
    }
}

/// `Buyer` places an order; `Seller` reads it and owns `ship | cancel`.
fn worked(delivery: &str, ship: &str, cancel: &str) -> Fixture {
    worked_over(delivery, "", ship, cancel)
}

/// [`worked`] with the protocol declaring `captures` first.
fn worked_over(delivery: &str, captures: &str, ship: &str, cancel: &str) -> Fixture {
    Fixture::over(
        delivery,
        captures,
        &format!(
            "sequence Main {{\n\
             send Placed via Order as (placed: Ack) {{ true }};\n\
             receive Got via Order of Placed as (got: Ack) {{ true }};\n\
             choice Decide by Seller visible (got.ok) {{\n\
               case no when {{ not got.ok }} sequence Cancel {{ {cancel} }}\n\
               case yes when {{ got.ok }} sequence Ship {{ {ship} }}\n\
             }} }}"
        ),
        &[0],
    )
}

const PICK: &str = "send PickIt via Pick as (p: PickMsg) { true };\n\
    receive PickedUp via Pick of PickIt as (q: PickMsg) { true };";
const CANCELLED: &str = "send Cn via Cancelled as (c: CancelMsg) { true };\n\
    receive Cn2 via Cancelled of Cn as (d: CancelMsg) { true };";

fn witness(outcome: RealizabilityOutcome) -> RealizabilityWitness {
    match outcome.outcome {
        AnalysisOutcome::Proved(witness) => witness,
        other => panic!("expected a witness: {other:?}"),
    }
}

fn refuted(outcome: RealizabilityOutcome) -> RealizabilityCounterexample {
    match outcome.outcome {
        AnalysisOutcome::Refuted(counterexample) => counterexample,
        other => panic!("expected a refutation: {other:?}"),
    }
}

fn inconclusive(outcome: RealizabilityOutcome) -> InconclusiveCause {
    match outcome.outcome {
        AnalysisOutcome::Inconclusive(cause) => cause,
        other => panic!("expected an inconclusive result: {other:?}"),
    }
}

fn position(record: &SeparatingWitnessRecord) -> Option<String> {
    record
        .trace_position
        .as_ref()
        .map(|held| held.as_str().to_owned())
}

/// TC-1411 step 2: the worked example's hidden choice.
#[trace(
    "TC-1411",
    "FR-571-AC-2",
    "FR-571-AC-4",
    "FR-574-AC-3",
    "FR-574-AC-4",
    "QSpec-FR-175-AC-2",
    "QSpec-FR-175-AC-4"
)]
#[test]
fn the_hidden_choice_refutes_by_indistinguishable_histories() {
    let fixture = worked("exactly-once-premise", PICK, "");
    let RealizabilityCounterexample::IndistinguishableHistories {
        role,
        history_a,
        history_b,
        action_a,
        action_b,
        obligation,
        separating_witness,
        ..
    } = refuted(fixture.run())
    else {
        panic!("expected indistinguishable histories");
    };
    assert_eq!(role, fixture.named("Warehouse"));
    let mut actions = [action_a, action_b];
    actions.sort();
    assert_eq!(
        actions,
        [
            StrategyAction::Receive {
                peer: fixture.named("Seller")
            },
            StrategyAction::End,
        ]
    );
    assert_ne!(history_a, history_b);
    assert_eq!(obligation, fixture.named("Done"));
    assert_eq!(
        separating_witness.deciding_element,
        DecidingElement::Protocol(node_text(&obligation).expect("the node encodes"))
    );
    let longer = history_a.steps.len().max(history_b.steps.len()) - 1;
    assert_eq!(position(&separating_witness), Some(longer.to_string()));
}

/// TC-1411 steps 1 and 4: the repair by a `cancelled` message.
#[trace(
    "TC-1411",
    "FR-571-AC-1",
    "FR-571-AC-4",
    "QSpec-FR-175-AC-1",
    "QSpec-FR-175-AC-4"
)]
#[test]
fn the_repaired_example_is_realized_by_a_table_the_walk_accepts() {
    let fixture = worked("exactly-once-premise", PICK, CANCELLED);
    let found = witness(fixture.run());
    assert!(found.closure.closed);
    assert!(found.closure.plays >= 2, "both branches are played");
    let warehouse = found
        .strategies
        .iter()
        .find(|strategy| strategy.role == fixture.named("Warehouse"))
        .expect("a Warehouse strategy");
    // One class, before any receive, serves both branches.
    assert_eq!(
        warehouse.table.get(&Vec::new()),
        Some(&StrategyAction::Receive {
            peer: fixture.named("Seller")
        })
    );
    let walked = replay_strategy(&fixture.admitted, fixture.request(64), &found).expect("admitted");
    assert_eq!(witness(walked), found);

    let mut edited = found.clone();
    for strategy in &mut edited.strategies {
        if strategy.role == fixture.named("Warehouse") {
            strategy.table.insert(Vec::new(), StrategyAction::End);
        }
    }
    let failed =
        replay_strategy(&fixture.admitted, fixture.request(64), &edited).expect("admitted");
    assert_eq!(
        inconclusive(failed),
        InconclusiveCause::StrategyReplayFailed
    );
}

/// FR-218: every role holds the protocol captures, bound once when its
/// instance starts, so two initial states that differ in a capture the
/// `Seller`'s guard reads are told apart by the `Warehouse` too: before any
/// message it ends under `big = false` and receives under `big = true`. The
/// end is the `big = false` state's, at the terminal state the `finish`
/// step reaches after the instance has gone, so it shows the captures are
/// held, not re-read from a state.
#[trace("TC-1411", "FR-571-AC-1")]
#[test]
fn a_capture_tells_initial_states_apart_for_every_role() {
    let fixture = Fixture::over(
        "exactly-once-premise",
        "capture big: Boolean = k.v >= 1;\n",
        &format!(
            "sequence Main {{\n\
             choice Decide by Seller visible (big = true) {{\n\
               case no when {{ not (big = true) }} sequence Cancel {{ }}\n\
               case yes when {{ big = true }} sequence Ship {{ {PICK} }}\n\
             }} }}"
        ),
        &[0, 1],
    );
    let found = witness(fixture.run());
    let warehouse = found
        .strategies
        .iter()
        .find(|strategy| strategy.role == fixture.named("Warehouse"))
        .expect("a Warehouse strategy");
    assert!(
        warehouse
            .table
            .keys()
            .all(|class| matches!(class.first(), Some(Observed::Captures { instance: 0, .. }))),
        "every class leads with the bound captures: {:?}",
        warehouse.table.keys()
    );
    let first: Vec<(&Vec<u8>, &StrategyAction)> = warehouse
        .table
        .iter()
        .filter_map(|(class, action)| match &class[..] {
            [Observed::Captures { payload, .. }] => Some((payload, action)),
            _ => None,
        })
        .collect();
    let [(small, end), (big, receive)] = match &first[..] {
        [a, b] if *a.1 == StrategyAction::End => [*a, *b],
        [a, b] => [*b, *a],
        other => panic!("two classes before any message: {other:?}"),
    };
    assert_ne!(small, big);
    assert_eq!(*end, StrategyAction::End);
    assert_eq!(
        *receive,
        StrategyAction::Receive {
            peer: fixture.named("Seller")
        }
    );
}

/// The hidden choice stays refuted when the protocol declares a capture its
/// guard does not read: the `Warehouse` holds the same captures under both
/// branches, at the terminal state too.
#[trace("TC-1411", "FR-571-AC-2", "FR-571-AC-4")]
#[test]
fn a_capture_the_guard_does_not_read_leaves_the_hidden_choice_refuted() {
    let fixture = worked_over(
        "exactly-once-premise",
        "capture big: Boolean = k.v >= 1;\n",
        PICK,
        "",
    );
    let RealizabilityCounterexample::IndistinguishableHistories { role, .. } =
        refuted(fixture.run())
    else {
        panic!("expected indistinguishable histories");
    };
    assert_eq!(role, fixture.named("Warehouse"));
}

/// TC-1411 step 3: absent premises and the horizon.
#[trace("TC-1411", "FR-571-AC-3", "QSpec-FR-175-AC-3")]
#[test]
fn an_unstated_premise_or_the_horizon_is_inconclusive() {
    let fixture = worked("exactly-once-premise", PICK, CANCELLED);
    let mut request = fixture.request(64);
    request.role_knowledge = Some(None);
    assert_eq!(
        inconclusive(fixture.decide(request)),
        InconclusiveCause::PremiseNotStated {
            member: "role_knowledge"
        }
    );
    let mut request = fixture.request(64);
    request.environment = Some(None);
    assert_eq!(
        inconclusive(fixture.decide(request)),
        InconclusiveCause::PremiseNotStated {
            member: "environment"
        }
    );
    witness(fixture.run());
    assert_eq!(
        inconclusive(fixture.decide(fixture.request(3))),
        InconclusiveCause::Horizon
    );
    let mut request = fixture.request(64);
    request.budget = RunBudget {
        max_states: 1,
        ..RunBudget::default()
    };
    assert_eq!(
        fixture.decide(request).outcome,
        AnalysisOutcome::Failed(RunLimit {
            limit: ModelCheckLimit::MaxStates,
            value: 1
        })
    );
}

/// V1 defines no fact a role-knowledge or environment premise may hold, so
/// a non-empty set is refused rather than carried in the key and ignored.
#[trace("TC-1411")]
#[test]
fn a_premise_that_holds_a_fact_is_refused() {
    let fixture = worked("exactly-once-premise", PICK, CANCELLED);
    let mut request = fixture.request(64);
    request.environment = Some(Some(BTreeSet::from(["loss".to_owned()])));
    assert!(matches!(
        realize(&fixture.admitted, request),
        Err(RealizabilityRefusal::PremiseFact("environment"))
    ));
}

/// TC-1411 steps 2 and 5: an admitted loss defeats the protocol on every
/// run of the environment's choosing, though every lossless run completes.
#[trace(
    "TC-1411",
    "FR-571-AC-2",
    "FR-571-AC-5",
    "FR-571-AC-6",
    "FR-574-AC-3",
    "FR-574-AC-4",
    "QSpec-FR-175-AC-2",
    "QSpec-FR-175-AC-5"
)]
#[test]
fn an_admitted_loss_is_an_environment_defeat() {
    let fixture = worked("at-most-once", PICK, CANCELLED);
    let RealizabilityCounterexample::EnvironmentDefeat {
        defeats_per_branch,
        obligation,
        separating_witness,
        ..
    } = refuted(fixture.run())
    else {
        panic!("expected an environment defeat");
    };
    let [defeat] = &defeats_per_branch[..] else {
        panic!("one branch: the environment fixes the choice");
    };
    assert!(defeat
        .play
        .steps
        .iter()
        .any(|step| step.transition.kind() == StepKind::Lose));
    assert_eq!(obligation, fixture.named("Done"));
    assert_eq!(
        separating_witness.deciding_element,
        DecidingElement::Protocol(node_text(&obligation).expect("the node encodes"))
    );
    assert_eq!(
        position(&separating_witness),
        Some((defeat.play.steps.len() - 1).to_string())
    );
}

/// TC-1411 step 5: an open delivery range is cut, so a passing table is
/// inconclusive.
#[trace("TC-1411", "FR-571-AC-5", "QSpec-FR-175-AC-5")]
#[test]
fn an_open_delivery_range_leaves_a_strategy_inconclusive() {
    let fixture = worked("at-least-once", PICK, CANCELLED);
    assert_eq!(inconclusive(fixture.run()), InconclusiveCause::CutCount);
}

/// TC-1412: two runs on one key return one result.
#[trace("TC-1412", "FR-572-AC-5", "QSpec-FR-175-AC-6")]
#[test]
fn two_runs_on_one_key_return_one_result() {
    for fixture in [
        worked("exactly-once-premise", PICK, CANCELLED),
        worked("exactly-once-premise", PICK, ""),
    ] {
        assert_eq!(fixture.run(), fixture.run());
    }
}

/// TC-1415: a step binder `qsl-eval` cannot enumerate is unsupported with
/// its cause.
#[trace("TC-1415", "FR-575-AC-2")]
#[test]
fn an_unenumerable_step_binder_is_unsupported_with_the_evaluator_cause() {
    let fixture = Fixture::new(
        "exactly-once-premise",
        "sequence Main { send Said via Words as (said: Txt) { true };\n\
         receive Heard via Words of Said as (heard: Txt) { true }; }",
    );
    match fixture.run().outcome {
        AnalysisOutcome::Unsupported(cause) => {
            assert_eq!(cause.slug, UnsupportedSlug::EvaluatorUnsupported);
            assert_eq!(
                cause.detail.as_deref(),
                Some("step binder of an unenumerable record type")
            );
        }
        other => panic!("expected unsupported: {other:?}"),
    }
}

/// TC-1415: `activation on each` starts an instance per trigger, outside the
/// profile, so realizability answers `unsupported` at the clause.
#[trace("TC-1415", "FR-575-AC-1")]
#[test]
fn an_activation_on_each_is_unsupported() {
    let fixture = Fixture::activated(
        "on each (t: M::Cell)",
        "exactly-once-premise",
        "",
        &format!("sequence Main {{ {PICK} }}"),
        &[0],
    );
    match fixture.run().outcome {
        AnalysisOutcome::Unsupported(cause) => {
            assert_eq!(cause.slug, UnsupportedSlug::ActivationOnEach);
            assert_eq!(cause.node, clause(&fixture.admitted));
        }
        other => panic!("expected unsupported: {other:?}"),
    }
}

fn replay(
    fixture: &Fixture,
    counterexample: &RealizabilityCounterexample,
) -> Result<(), RealizabilityDisagreement> {
    replay_counterexample(&fixture.admitted, fixture.request(64), counterexample)
        .expect("admitted")
        .expect("the request reaches an arena")
}

/// PA-20 (ADR-018 SM-7): both refutation forms the engine returns replay,
/// including one found over two initial states; each edit of a play, the
/// actions, the obligation, the key or the record disagrees.
#[trace("TC-1413", "FR-573-AC-2", "FR-573-AC-3")]
#[test]
fn the_realizability_replay_arm_reproduces_both_forms_and_refuses_edits() {
    let hidden = worked("exactly-once-premise", PICK, "");
    let found = refuted(hidden.run());
    assert_eq!(replay(&hidden, &found), Ok(()));
    let edit = |change: &dyn Fn(&mut RealizabilityCounterexample)| {
        let mut edited = found.clone();
        change(&mut edited);
        replay(&hidden, &edited)
    };
    // A step whose successor index names another transition.
    assert!(matches!(
        edit(
            &|ce| if let RealizabilityCounterexample::IndistinguishableHistories {
                history_a,
                history_b,
                ..
            } = ce
            {
                let longer = if history_a.steps.len() >= history_b.steps.len() {
                    history_a
                } else {
                    history_b
                };
                longer.steps.remove(0);
            }
        ),
        Err(RealizabilityDisagreement::StepNotSuccessor { .. })
    ));
    // An initial state the system does not have.
    assert_eq!(
        edit(
            &|ce| if let RealizabilityCounterexample::IndistinguishableHistories {
                history_a, ..
            } = ce
            {
                history_a.initial = 7;
            }
        ),
        Err(RealizabilityDisagreement::StepNotSuccessor { at: 0 })
    );
    // Two equal actions are no incompatible pair.
    assert_eq!(
        edit(
            &|ce| if let RealizabilityCounterexample::IndistinguishableHistories {
                action_a,
                action_b,
                ..
            } = ce
            {
                *action_b = action_a.clone();
            }
        ),
        Err(RealizabilityDisagreement::SameAction)
    );
    // The obligation is not the one a hidden choice violates.
    let other = hidden.named("Seller");
    assert_eq!(
        edit(
            &|ce| if let RealizabilityCounterexample::IndistinguishableHistories {
                obligation,
                ..
            } = ce
            {
                *obligation = other.clone();
            }
        ),
        Err(RealizabilityDisagreement::RecordMismatch)
    );
    // A counterexample of another key or with another record.
    let foreign = hidden.decide(hidden.request(63)).key;
    assert_eq!(
        edit(
            &|ce| if let RealizabilityCounterexample::IndistinguishableHistories { key, .. } = ce {
                *key = foreign.clone();
            }
        ),
        Err(RealizabilityDisagreement::KeyMismatch)
    );
    assert_eq!(
        edit(
            &|ce| if let RealizabilityCounterexample::IndistinguishableHistories {
                separating_witness,
                ..
            } = ce
            {
                separating_witness.trace_position = None;
            }
        ),
        Err(RealizabilityDisagreement::RecordMismatch)
    );

    let lossy = worked("at-most-once", PICK, CANCELLED);
    let defeat = refuted(lossy.run());
    assert_eq!(replay(&lossy, &defeat), Ok(()));
    let mut short = defeat.clone();
    if let RealizabilityCounterexample::EnvironmentDefeat {
        defeats_per_branch,
        separating_witness,
        obligation,
        ..
    } = &mut short
    {
        defeats_per_branch[0].play.steps.pop();
        let position = defeats_per_branch[0].play.steps.len() - 1;
        *separating_witness = AssessedRoot::new(Some(assessed_root()))
            .unwrap()
            .record(obligation, position)
            .unwrap();
    }
    assert_eq!(
        replay(&lossy, &short),
        Err(RealizabilityDisagreement::NotTheStatedViolation)
    );
    let mut none = defeat;
    if let RealizabilityCounterexample::EnvironmentDefeat {
        defeats_per_branch, ..
    } = &mut none
    {
        defeats_per_branch.clear();
    }
    assert!(replay(&lossy, &none).is_err());

    // A true refutation over two initial states names its initial state,
    // so the walk takes the right one.
    let two = Fixture::over(
        "exactly-once-premise",
        "",
        &format!(
            "sequence Main {{\n\
             send Placed via Order as (placed: Ack) {{ true }};\n\
             receive Got via Order of Placed as (got: Ack) {{ true }};\n\
             choice Decide by Seller visible (got.ok) {{\n\
               case no when {{ not got.ok }} sequence Cancel {{ }}\n\
               case yes when {{ got.ok }} sequence Ship {{ {PICK} }}\n\
             }} }}"
        ),
        &[0, 1],
    );
    let found = refuted(two.run());
    assert_eq!(replay(&two, &found), Ok(()));
}

/// PA-18 (i): a claimed pair of indistinguishable histories
/// whose two actions are both enabled at both ends, with no evidence that
/// either loses at one end, is refused. Here both histories are the empty
/// play, where the `Buyer` may take either of two parallel sends.
#[trace("TC-1413", "FR-573-AC-3")]
#[test]
fn indistinguishable_histories_without_disjoint_not_lost_sets_are_refused() {
    let fixture = Fixture::new(
        "exactly-once-premise",
        "sequence Main { parallel Par {\n\
           branch a sequence Ay { send Sa via Order as (sa: Ack) { true }; }\n\
           branch b sequence Bee { send Sb via Order as (sb: Ack) { true }; }\n\
         } join all [a, b];\n\
         receive Ra via Order of Sa as (ra: Ack) { true };\n\
         receive Rb via Order of Sb as (rb: Ack) { true }; }",
    );
    let key = fixture.run().key;
    let finish = fixture.named("Done");
    let empty = Play {
        initial: 0,
        steps: Vec::new(),
    };
    let claim = |lost: Vec<LostAction>| RealizabilityCounterexample::IndistinguishableHistories {
        key: key.clone(),
        role: fixture.named("Buyer"),
        history_a: empty.clone(),
        history_b: empty.clone(),
        action_a: StrategyAction::Send {
            node: fixture.named("Sa"),
        },
        action_b: StrategyAction::Send {
            node: fixture.named("Sb"),
        },
        lost,
        separating_witness: AssessedRoot::new(Some(assessed_root()))
            .unwrap()
            .record(&finish, 0)
            .unwrap(),
        obligation: finish.clone(),
    };
    assert_eq!(
        replay(&fixture, &claim(Vec::new())),
        Err(RealizabilityDisagreement::NotDisjoint)
    );
}

/// PA-20: a forced refutation settles `refuted` only through the replay
/// arm; an edited counterexample settles `inconclusive` `replay-disagreed`.
#[trace("TC-1413", "FR-573-AC-3")]
#[test]
fn the_engine_settles_an_edited_refutation_inconclusive() {
    let hidden = worked("exactly-once-premise", PICK, "");
    let untouched = realize_tampered(&hidden.admitted, hidden.request(64), &|_| {}).unwrap();
    assert!(matches!(untouched.outcome, AnalysisOutcome::Refuted(_)));
    let edited = realize_tampered(&hidden.admitted, hidden.request(64), &|ce| {
        if let RealizabilityCounterexample::IndistinguishableHistories {
            action_a, action_b, ..
        } = ce
        {
            *action_b = action_a.clone();
        }
    })
    .unwrap();
    assert_eq!(inconclusive(edited), InconclusiveCause::ReplayDisagreed);
    let lossy = worked("at-most-once", PICK, CANCELLED);
    let edited = realize_tampered(&lossy.admitted, lossy.request(64), &|ce| {
        if let RealizabilityCounterexample::EnvironmentDefeat {
            defeats_per_branch, ..
        } = ce
        {
            defeats_per_branch.clear();
        }
    })
    .unwrap();
    assert_eq!(inconclusive(edited), InconclusiveCause::ReplayDisagreed);
}

fn projection_request(fixture: &Fixture, max_depth: u64) -> ProjectionRequest {
    ProjectionRequest {
        protocol: clause(&fixture.admitted),
        definition_key: Some("ix://test/Flow".to_owned()),
        role: fixture.named("Warehouse"),
        profile: Some("bounded-static".to_owned()),
        bounds: Some(ProjectionBoundsDraft {
            max_depth: Some(max_depth),
        }),
        assessed_root: Some(assessed_root()),
        budget: RunBudget::default(),
    }
}

/// The FR-360 axes one served item settled with, read from its terminal
/// record and its carried result.
fn axes(
    record: &AnalyzeRecord,
) -> (
    TerminalValue,
    ResultLabel,
    SettlementBasis,
    Execution,
    Truth,
) {
    let settlement = record
        .protocol_analysis
        .as_ref()
        .expect("a served item carries its result")
        .settlement();
    let value = record
        .terminal()
        .expect("a terminal record")
        .value()
        .clone();
    assert_eq!(
        value, settlement.value,
        "the record holds the settlement's value"
    );
    (
        value,
        settlement.label,
        settlement.basis,
        settlement.execution,
        settlement.truth,
    )
}

/// TC-155, ADR-036 PA-4: the linked engines advertise `local-projection`
/// and `realizability` through one manifest, so each kind has one
/// candidate, and naming it settles a single backend; a backend that lacks
/// the kind still settles the unchanged `UnsupportedRequestedCapability`.
#[trace("TC-155", "FR-057-AC-15")]
#[test]
fn the_protocol_kinds_are_served_by_the_linked_manifest() {
    let mut registry = Registry::new();
    for descriptor in Engines::qsl().expect("they register").registrations() {
        registry
            .register(descriptor.clone())
            .expect("a unique backend");
    }
    let engine = BackendId::new(protocol_analysis::manifest::BACKEND_ID);
    for kind in [Capability::LocalProjection, Capability::Realizability] {
        let CandidateOutcome::Candidates(set) = registry.candidates(kind, None) else {
            panic!("{kind:?}: a candidate set");
        };
        assert_eq!(
            set.candidates(),
            [Candidate::new(engine.clone())],
            "{kind:?}"
        );
        assert_eq!(
            settle_candidates(
                &registry.candidates(kind, Some(&engine)),
                kind,
                Some(&engine)
            ),
            NegotiationSettlement::Single(engine.clone())
        );
        let other = BackendId::new("en-1");
        assert_eq!(
            settle_candidates(&registry.candidates(kind, Some(&other)), kind, Some(&other)),
            NegotiationSettlement::Cause(NegotiationCause::UnsupportedRequestedCapability(
                kind,
                Some(other)
            ))
        );
    }
}

/// TC-1413: one `analyze` run over the linked engines settles an item of
/// each kind under every FR-360 label that can arise for it, by the FR-573
/// map; every `proved` is `Uncertified`.
#[trace(
    "TC-1413",
    "FR-573-AC-1",
    "FR-573-AC-2",
    "FR-573-AC-4",
    "FR-573-AC-5",
    "FR-573-AC-6",
    "FR-573-AC-7"
)]
#[test]
fn a_driver_run_settles_each_kind_under_each_label() {
    const METER_WORK_UNITS: u64 = 200;
    let sound = worked("exactly-once-premise", PICK, CANCELLED);
    let hidden = worked("exactly-once-premise", PICK, "");
    let lossy = worked("at-most-once", PICK, CANCELLED);
    let open = worked("at-least-once", PICK, CANCELLED);
    let each = Fixture::activated(
        "on each (t: M::Cell)",
        "exactly-once-premise",
        "",
        &format!("sequence Main {{ {PICK} }}"),
        &[0],
    );
    let limited = RunBudget {
        max_states: 1,
        ..RunBudget::default()
    };
    let (sound, hidden, lossy, open, each) = (&sound, &hidden, &lossy, &open, &each);
    let at = RequestIndex::new;
    let mut items = Vec::new();
    // Projection: proved, refuted, inconclusive (horizon), unsupported, failed.
    let short = projection_request(sound, 1);
    let mut stopped = projection_request(sound, 64);
    stopped.budget = limited;
    for (fixture, request) in [
        (sound, projection_request(sound, 64)),
        (hidden, projection_request(hidden, 64)),
        (sound, short),
        (each, projection_request(each, 64)),
        (sound, stopped),
    ] {
        items.push(projection_item(at(items.len()), &fixture.admitted, request));
    }
    // Realizability: proved, both refutations, inconclusive, unsupported, failed.
    let mut unstated = sound.request(64);
    unstated.role_knowledge = Some(None);
    let mut stopped = sound.request(64);
    stopped.budget = limited;
    for (fixture, request) in [
        (sound, sound.request(64)),
        (hidden, hidden.request(64)),
        (lossy, lossy.request(64)),
        (sound, unstated),
        (each, each.request(64)),
        (sound, stopped),
    ] {
        items.push(realizability_item(
            at(items.len()),
            &fixture.admitted,
            request,
        ));
    }
    // A projection stopped by `max_transitions`, a realizability result
    // decided on a cut count, and one stopped by the evaluation meter.
    let mut transitions = projection_request(sound, 64);
    transitions.budget = RunBudget {
        max_transitions: 1,
        ..RunBudget::default()
    };
    items.push(projection_item(
        at(items.len()),
        &sound.admitted,
        transitions,
    ));
    items.push(realizability_item(
        at(items.len()),
        &open.admitted,
        open.request(64),
    ));
    let mut metered = sound.request(64);
    metered.subject.model.simulation.meter = quire_exact::ScalarLimits {
        work_units: METER_WORK_UNITS,
        ..crate::counter::UNLIMITED
    };
    items.push(realizability_item(
        at(items.len()),
        &sound.admitted,
        metered,
    ));
    let records = analyze_items(
        &Engines::qsl().expect("they register"),
        items,
        &Cancel::new(),
    );
    assert_eq!(records.len(), 14);

    let proved = (
        ResultLabel::Proved,
        SettlementBasis::ClosedScope,
        Execution::Completed,
        Truth::Satisfied,
    );
    let refuted_axes = (
        ResultLabel::Refuted,
        SettlementBasis::DecisiveCounterexample,
        Execution::Completed,
        Truth::Violated,
    );
    let pending = (
        ResultLabel::Inconclusive,
        SettlementBasis::Unsettled,
        Execution::Completed,
        Truth::Pending,
    );
    let unsupported = (
        ResultLabel::Unsupported,
        SettlementBasis::Unavailable,
        Execution::Unsupported,
        Truth::Unavailable,
    );
    let stopped = (
        ResultLabel::Failed,
        SettlementBasis::Unavailable,
        Execution::ResourceIncomplete,
        Truth::Unavailable,
    );
    let uncertified = TerminalValue::Proved {
        basis: ProofBasis::Exhaustive,
        certification: Certification::Uncertified,
    };
    let limit = TerminalValue::Incomplete(IncompleteCause::LimitReached {
        limit: ModelCheckLimit::MaxStates,
        value: 1,
    });
    for (at, (value, label, basis, execution, truth)) in records.iter().map(axes).enumerate() {
        let (want, axes) = match at {
            0 | 5 => (Some(uncertified.clone()), proved),
            1 | 6 | 7 => (Some(TerminalValue::Refuted), refuted_axes),
            2 => (
                Some(TerminalValue::Inconclusive(Fr331Cause::BoundReached {
                    depth: 1,
                })),
                pending,
            ),
            8 => (
                Some(TerminalValue::Inconclusive(Fr331Cause::ProtocolAnalysis(
                    InconclusiveCause::PremiseNotStated {
                        member: "role_knowledge",
                    },
                ))),
                pending,
            ),
            3 | 9 => (None, unsupported),
            4 | 10 => (Some(limit.clone()), stopped),
            11 => (
                Some(TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::MaxTransitions,
                    value: 1,
                })),
                stopped,
            ),
            12 => (
                Some(TerminalValue::Inconclusive(Fr331Cause::ProtocolAnalysis(
                    InconclusiveCause::CutCount,
                ))),
                pending,
            ),
            _ => {
                assert!(
                    matches!(
                        &value,
                        TerminalValue::Incomplete(IncompleteCause::LimitReached {
                            limit: ModelCheckLimit::EvaluationMeter { .. },
                            ..
                        })
                    ),
                    "item {at}: {value:?}"
                );
                (Some(value.clone()), stopped)
            }
        };
        assert_eq!((label, basis, execution, truth), axes, "item {at}");
        match want {
            Some(want) => assert_eq!(value, want, "item {at}"),
            None => assert!(
                matches!(
                    &value,
                    TerminalValue::Unsupported(UnavailabilityCause::ProtocolAnalysis(cause))
                        if cause.slug == UnsupportedSlug::ActivationOnEach
                ),
                "item {at}: {value:?}"
            ),
        }
        assert!(!matches!(
            value,
            TerminalValue::Proved {
                certification: Certification::Certified | Certification::Trusted,
                ..
            }
        ));
    }
}
