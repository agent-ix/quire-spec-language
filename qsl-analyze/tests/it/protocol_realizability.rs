// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-571, FR-572, FR-574, FR-575 (TC-1411, TC-1412, TC-1414, TC-1415):
//! deciding realizability over the `qsl-eval` arena (ADR-036 slice 4).

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_analyze::protocol_analysis::{
    realize, replay_strategy, RealizabilityOutcome, RealizabilityRefusal, RealizabilityRequest,
};
use qsl_eval::model_subject::{ProtocolSubject, RoleBinding};
use qsl_eval::simulation::protocol::StepKind;
use qsl_package::CheckedPackage;
use qsl_replay::protocol_analysis::{
    node_text, AnalysisOutcome, BoundsDraft, InconclusiveCause, Observed,
    RealizabilityCounterexample, RealizabilityWitness, RunBudget, RunLimit, StrategyAction,
    UnsupportedSlug,
};
use qsl_replay::{DecidingElement, SeparatingWitnessRecord};
use qsl_semantics::model::observation::DocumentRef;
use quire_contract_model::{CheckedNodeId, CheckedPackageV2};

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
        let text = format!(
            "{RECORDS}protocol Flow using pp over (k: M::Cell) on origin {{\n{captures}{}\nrun {run}\n\
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
    Fixture::new(
        delivery,
        &format!(
            "sequence Main {{\n\
             send Placed via Order as (placed: Ack) {{ true }};\n\
             receive Got via Order of Placed as (got: Ack) {{ true }};\n\
             choice Decide by Seller visible (got.ok) {{\n\
               case no when {{ not got.ok }} sequence Cancel {{ {cancel} }}\n\
               case yes when {{ got.ok }} sequence Ship {{ {ship} }}\n\
             }} }}"
        ),
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
    let longer = history_a.len().max(history_b.len()) - 1;
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

/// FR-218: every role holds the protocol captures, so two initial states
/// that differ in a capture the `Seller`'s guard reads are told apart by
/// the `Warehouse` too: it ends under one and receives under the other.
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
    let mut first: Vec<&StrategyAction> = warehouse
        .table
        .iter()
        .filter(|(class, _)| matches!(class[..], [Observed::Captures { .. }]))
        .map(|(_, action)| action)
        .collect();
    first.sort();
    assert_eq!(
        first,
        [
            &StrategyAction::Receive {
                peer: fixture.named("Seller")
            },
            &StrategyAction::End,
        ]
    );
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
            limit: "max_states",
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
    assert!(defeat.play.iter().any(|step| step.kind() == StepKind::Lose));
    assert_eq!(obligation, fixture.named("Done"));
    assert_eq!(
        separating_witness.deciding_element,
        DecidingElement::Protocol(node_text(&obligation).expect("the node encodes"))
    );
    assert_eq!(
        position(&separating_witness),
        Some((defeat.play.len() - 1).to_string())
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
