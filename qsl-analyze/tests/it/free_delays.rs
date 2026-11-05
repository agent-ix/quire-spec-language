// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-204 (TC-642, TC-631): the post-draw `FreeDelays` action of a digital
//! product where given-delay and free-delay identities race together under a
//! workload, its row choices, the row-factored values and the policy's
//! whole-action choice.
//!
//! The fixture's `Call` stands for FR-204's models: `phase` 0 is `Idle`, 1
//! `Half` and 2 `Done`, and `late` is `b`.
//!
//! These tests assert the product's choices, the exact extrema, the policy's
//! whole `FreeDelays` action, the produced path sets with their merged step
//! probabilities, and the canonical `free-delays` body with its admission at
//! the race state. A claim's `proved`/`refuted` reading is the value against
//! its threshold; the settled outcome item and FR-202's replay through
//! `ModelSystem` are not asserted here (remaining work: QSL-1708).

use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::{
    action_steps, build_product, decide_bounded_event, decide_fair_expected_reward,
    decide_fair_reachability, decide_reachability, free_delays_choice, produce_witness,
    witness_scheduler, ActionChoice, EntryChoice, ExactProbLimits, ExtRational, FhResult,
    FreeDelayRow, MonitorVerdict, ProbBuild, ProbProduct, ProbStateId, ProductAction, ProductKind,
    ProductRefusal, ProductRequest, SchedulerChoice, UnboundedObjective, WitnessError,
    WitnessScope,
};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::ScheduledIdentity;
use qsl_replay::{
    FreeDelayActionRow, FreeDelaysRace, FreeRacer, GivenRacer,
    ProductAction as ReplayProductAction, ProductActionRefusal, Schedule,
};
use qsl_semantics::check::Bound;
use quire_exact::{Cancel, Integer, Rational};
use quire_semantic_value::catalog::CatalogCode;

use crate::timed_world::{self, World};

const PROFILES: &str = "profile ev = \"quire.temporal.timed/v1\";\n\
     profile inf = \"quire.temporal.infinite-trace/v1\";\n";

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

/// A model over `Call` with the clock `x`, the operations `operations` (name,
/// the fields it modifies) and the unit text `body`.
fn model(operations: &[(&str, &[&str])], body: &str) -> World {
    let document = timed_world::document(&["x"], operations);
    timed_world::world(
        &document,
        PROFILES,
        &format!("{}{body}", timed_world::DENSE),
    )
}

/// An operation `name` guarded by `guard` whose post-state is `post`.
fn operation(name: &str, guard: &str, post: &str) -> String {
    format!(
        "pre G{name} using v on M::Call::{name} {{ {guard} }}\n\
         post P{name} using v on M::Call::{name} {{ {post} }}\n"
    )
}

/// A workload `name` weighting each of `operations` 1.
fn workload(name: &str, operations: &[&str]) -> String {
    let weights: String = operations
        .iter()
        .map(|operation| format!("weight Call::{operation} = 1; "))
        .collect();
    format!("workload {name} on M {{ {weights}}}\n")
}

/// A probability claim over `c` under `scheduler`.
fn claim(name: &str, scheduler: &str, bound: &str, event: &str) -> String {
    format!(
        "probabilistic {name} using ev on M over (c: M::Call) {scheduler} on origin \
         {{ probability {bound} [ {event} ] }}\n"
    )
}

/// FR-204-AC-6's `Race` with `A`'s delay `a_delay` and `B`'s guard `x >= b_from`
/// ms, plus `extra` operation text and the `Both` workload over `weighted`.
fn race(a_delay: &str, b_from: u32, extra: &str, weighted: &[&str], claims: &str) -> World {
    let mut operations: Vec<(&str, &[&str])> = vec![("A", &["phase"]), ("B", &["phase", "late"])];
    if weighted.contains(&"C") {
        operations.push(("C", &["phase"]));
    }
    model(
        &operations,
        &format!(
            "{}{}{extra}\
             time invariant Wait when {{ self.phase = 0 }} {{ self.x <= 3 ms }}\n\
             delay M::Call::A ~ discrete {{ {a_delay} }};\n{}{claims}",
            operation("A", "self.phase = 0", "self.phase = 2"),
            operation(
                "B",
                &format!("self.phase = 0 and self.x >= {b_from} ms"),
                "self.phase = 2 and self.late = true",
            ),
            workload("Both", weighted),
        ),
    )
}

/// `Max` (`<= max`) and `Min` (`>= min`) over `eventually[0 ms, D] holds(c.late)`.
fn extrema_claims(deadline: u32, max: &str, min: &str) -> String {
    let event = format!("eventually[0 ms, {deadline} ms] holds(c.late)");
    format!(
        "{}{}",
        claim("Max", "under Both", &format!("<= {max}"), &event),
        claim("Min", "under Both", &format!(">= {min}"), &event),
    )
}

fn build(world: &World, name: &str) -> ProbProduct {
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim(name)
        .expect("the claim checks");
    match build_product(
        ProductRequest {
            subject: ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim,
            limits: ExactProbLimits::default(),
        },
        &Cancel::new(),
    ) {
        Ok(ProbBuild::Built(product)) => product,
        other => panic!("{name}: expected a product, got {other:?}"),
    }
}

fn refusal(world: &World, name: &str) -> ProductRefusal {
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim(name)
        .expect("the claim checks");
    build_product(
        ProductRequest {
            subject: ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim,
            limits: ExactProbLimits::default(),
        },
        &Cancel::new(),
    )
    .expect_err("the build is refused")
}

/// An operation text's operation name.
fn short(world: &World, text: &str) -> String {
    world
        .system()
        .operations()
        .into_iter()
        .find(|(_, _, candidate)| *candidate == text)
        .map(|(_, name, _)| name.to_owned())
        .unwrap_or_else(|| panic!("no operation {text}"))
}

/// A row as `(given, free)`, each `(operation name, delay)`.
type Named = (Vec<(String, u64)>, Vec<(String, u64)>);

fn named(world: &World, row: &FreeDelayRow) -> Named {
    let names = |list: &[(qsl_eval::simulation::model::ScheduledIdentity, u64)]| {
        list.iter()
            .map(|(identity, delay)| (short(world, &identity.operation), *delay))
            .collect()
    };
    (names(&row.given), names(&row.free))
}

fn row(given: &[(&str, u64)], free: &[(&str, u64)]) -> Named {
    let owned = |list: &[(&str, u64)]| {
        list.iter()
            .map(|(name, delay)| ((*name).to_owned(), *delay))
            .collect()
    };
    (owned(given), owned(free))
}

/// The initial state's row choices in edge order, each once, grouped by row.
fn row_choices(world: &World, product: &ProbProduct) -> Vec<Vec<Named>> {
    let start = product.initial[0].2;
    let mut rows: Vec<Vec<Named>> = Vec::new();
    for edge in product.edges.iter().filter(|edge| edge.from == start) {
        let ProductAction::FreeDelays(bodies) = &edge.choice else {
            panic!("a mixed race offers FreeDelays only: {:?}", edge.choice);
        };
        let [body] = bodies.as_slice() else {
            panic!("a row-choice edge names one row: {bodies:?}");
        };
        let choice = named(world, body);
        match rows.last_mut() {
            Some(last) if last[0].0 == choice.0 => {
                if last.last() != Some(&choice) {
                    last.push(choice);
                }
            }
            _ => rows.push(vec![choice]),
        }
    }
    rows
}

/// The number of `FreeDelays` actions the row choices make: one row choice
/// per row.
fn actions(rows: &[Vec<Named>]) -> usize {
    rows.iter().map(Vec::len).product()
}

/// The initial value of a deadline claim and the whole `FreeDelays` action
/// its policy takes at the initial state.
fn decide(world: &World, product: &ProbProduct, bound: Bound) -> (Rational, Vec<Named>) {
    let result = decide_bounded_event(product, &bound, &ExactProbLimits::default())
        .expect("the digital product is acyclic");
    let FhResult::Exact { values, policy } = result else {
        panic!("exact values: {result:?}");
    };
    let start = product.initial[0].2;
    (
        values[start.index()].clone(),
        witness(world, product, start, &policy),
    )
}

fn witness(
    world: &World,
    product: &ProbProduct,
    state: qsl_analyze::exact_probabilistic::ProbStateId,
    policy: &[Option<usize>],
) -> Vec<Named> {
    let Some(ProductAction::FreeDelays(rows)) = free_delays_choice(product, state, policy) else {
        panic!("the policy takes a FreeDelays action at the initial state");
    };
    rows.iter().map(|row| named(world, row)).collect()
}

/// FR-204-AC-6 (TC-642 step 3): the one row given `[2]` offers `B` the delays
/// 1, 2 and 3; the minimum `0` takes `B` delay 3 and the maximum `1` delay 1,
/// and the tie at `m = 2` splits `1/2` and `1/2`.
#[trace("TC-642", "FR-204-AC-6")]
#[test]
fn race_offers_three_free_delays_actions() {
    let world = race(
        "2 ms: 1",
        1,
        "",
        &["A", "B"],
        &extrema_claims(5, "0.5", "0.5"),
    );
    let built = build(&world, "Max");
    assert_eq!(built.kind, ProductKind::Mdp);
    let rows = row_choices(&world, &built);
    assert_eq!(
        rows,
        [vec![
            row(&[("A", 2)], &[("B", 1)]),
            row(&[("A", 2)], &[("B", 2)]),
            row(&[("A", 2)], &[("B", 3)]),
        ]]
    );
    assert_eq!(actions(&rows), 3);
    let tie: Vec<_> = built
        .edges
        .iter()
        .filter(|edge| {
            edge.from == built.initial[0].2
                && matches!(&edge.choice, ProductAction::FreeDelays(rows)
                    if rows[0].free[0].1 == 2)
        })
        .map(|edge| edge.probability.clone())
        .collect();
    assert_eq!(tie, [ratio(1, 2), ratio(1, 2)], "the tie at m = 2");

    let (maximum, max_witness) = decide(&world, &built, Bound::AtMost(ratio(1, 2)));
    assert_eq!(maximum, ratio(1, 1));
    assert_eq!(max_witness, [row(&[("A", 2)], &[("B", 1)])]);
    let (minimum, min_witness) = decide(&world, &build(&world, "Min"), Bound::AtLeast(ratio(1, 2)));
    assert_eq!(minimum, ratio(0, 1));
    assert_eq!(min_witness, [row(&[("A", 2)], &[("B", 3)])]);
}

/// FR-204-AC-7 (TC-642 step 4): with `B` guarded by `x >= 2 ms` the maximum
/// is the tie `1/2`, taken by the row given `[2]` to `B` delay 2, and the
/// minimum `0`; under every scheduler the claim is unsupported,
/// `DelayDistribution`, naming `A`.
#[trace("TC-642", "FR-204-AC-7")]
#[test]
fn a_later_guard_leaves_the_tie_as_the_maximum() {
    let world = race(
        "2 ms: 1",
        2,
        "",
        &["A", "B"],
        &format!(
            "{}{}",
            extrema_claims(5, "0.4", "0.5"),
            claim(
                "Every",
                "under every scheduler",
                "<= 0.5",
                "eventually[0 ms, 5 ms] holds(c.late)"
            ),
        ),
    );
    let built = build(&world, "Max");
    assert_eq!(actions(&row_choices(&world, &built)), 2);
    let (maximum, max_witness) = decide(&world, &built, Bound::AtMost(ratio(2, 5)));
    assert_eq!(maximum, ratio(1, 2));
    assert!(maximum > ratio(2, 5), "`<= 0.4` is refuted on the maximum");
    assert!(maximum <= ratio(1, 2), "`<= 0.5` is proved");
    assert_eq!(max_witness, [row(&[("A", 2)], &[("B", 2)])]);
    let (minimum, _) = decide(&world, &build(&world, "Min"), Bound::AtLeast(ratio(1, 2)));
    assert_eq!(minimum, ratio(0, 1));
    let ProductRefusal::DelayDistribution { operation } = refusal(&world, "Every") else {
        panic!("a delay distribution under every scheduler");
    };
    assert_eq!(short(&world, &operation), "A");
}

/// FR-204-AC-8 and AC-9 (TC-642 step 5): two rows, given `[1]` and `[3]`,
/// each offering `B` the delays 1 to 3, make nine actions; the maximum `3/4`
/// and the minimum `1/4` take the least attaining delay of each row.
#[trace("TC-642", "FR-204-AC-8")]
#[trace("TC-642", "FR-204-AC-9")]
#[test]
fn two_rows_take_the_least_attaining_delay_of_each() {
    let world = race(
        "1 ms: 1, 3 ms: 1",
        1,
        "",
        &["A", "B"],
        &extrema_claims(5, "0.7", "0.3"),
    );
    let built = build(&world, "Max");
    let rows = row_choices(&world, &built);
    let delays = |given: u64| -> Vec<Named> {
        (1..=3)
            .map(|delay| row(&[("A", given)], &[("B", delay)]))
            .collect()
    };
    assert_eq!(rows, [delays(1), delays(3)]);
    assert_eq!(actions(&rows), 9);

    let (maximum, max_witness) = decide(&world, &built, Bound::AtMost(ratio(7, 10)));
    assert_eq!(maximum, ratio(3, 4));
    assert!(maximum > ratio(7, 10), "`<= 0.7` is refuted on the maximum");
    assert_eq!(
        max_witness,
        [row(&[("A", 1)], &[("B", 1)]), row(&[("A", 3)], &[("B", 1)])]
    );
    let (minimum, min_witness) =
        decide(&world, &build(&world, "Min"), Bound::AtLeast(ratio(3, 10)));
    assert_eq!(minimum, ratio(1, 4));
    assert!(minimum < ratio(3, 10), "`>= 0.3` is refuted on the minimum");
    assert_eq!(
        min_witness,
        [row(&[("A", 1)], &[("B", 2)]), row(&[("A", 3)], &[("B", 3)])]
    );
}

/// FR-204-AC-10's `Detour`: `B` leads to `Half`, where `G` (sets `b`) and `L`
/// tie at one more millisecond.
fn detour(claims: &str) -> World {
    model(
        &[
            ("A", &["phase", "late"]),
            ("B", &["phase"]),
            ("G", &["phase", "late"]),
            ("L", &["phase"]),
        ],
        &format!(
            "{}{}{}{}\
             time invariant Wait when {{ self.phase = 0 }} {{ self.x <= 3 ms }}\n\
             delay M::Call::A ~ discrete {{ 1 ms: 1, 3 ms: 1 }};\n\
             delay M::Call::G ~ discrete {{ 1 ms: 1 }};\n\
             delay M::Call::L ~ discrete {{ 1 ms: 1 }};\n{}{claims}",
            operation("A", "self.phase = 0", "self.phase = 2 and self.late = true"),
            operation("B", "self.phase = 0 and self.x >= 1 ms", "self.phase = 1"),
            operation("G", "self.phase = 1", "self.phase = 2 and self.late = true"),
            operation("L", "self.phase = 1", "self.phase = 2"),
            workload("All", &["A", "B", "G", "L"]),
        ),
    )
}

/// FR-204-AC-10 (TC-642 step 6), the separating vector: after the draw the
/// maximum is `3/4` and the minimum `3/8`, so `<= 0.7` and `>= 0.45` are both
/// refuted; a choice made before the draw would give `5/8`, `1/2` and `1/2`
/// for `B` at 1, 2 and 3, proving both claims.
#[trace("TC-642", "FR-204-AC-10")]
#[test]
fn detour_separates_the_post_draw_choice_from_the_pre_draw_one() {
    let event = "eventually[0 ms, 2 ms] holds(c.late)";
    let world = detour(&format!(
        "{}{}",
        claim("Max", "under All", "<= 0.7", event),
        claim("Min", "under All", ">= 0.45", event),
    ));
    let built = build(&world, "Max");
    let rows = row_choices(&world, &built);
    assert_eq!(actions(&rows), 9);

    let result = decide_bounded_event(
        &built,
        &Bound::AtMost(ratio(7, 10)),
        &ExactProbLimits::default(),
    )
    .expect("the digital product is acyclic");
    let FhResult::Exact { values, policy } = result else {
        panic!("exact values: {result:?}");
    };
    let start = built.initial[0].2;
    let maximum = values[start.index()].clone();
    assert_eq!(maximum, ratio(3, 4));
    assert!(maximum > ratio(7, 10), "`<= 0.7` is refuted on the maximum");
    assert_eq!(
        witness(&world, &built, start, &policy),
        [row(&[("A", 1)], &[("B", 2)]), row(&[("A", 3)], &[("B", 1)])]
    );

    // Before the draw the scheduler would fix one delay `d` for both rows:
    // `Σ_g P(g) Q(g, d)`, read from the row-choice edges with `B` at `d`.
    // The race's successors have no further free delay, so their values are
    // the scheduler's under either objective.
    let pre_draw = |delay: u64| {
        built
            .edges
            .iter()
            .filter(|edge| {
                edge.from == start
                    && matches!(&edge.choice, ProductAction::FreeDelays(rows)
                        if rows[0].free[0].1 == delay)
            })
            .fold(ratio(0, 1), |sum, edge| {
                sum.add(&edge.probability.mul(&values[edge.to.index()]))
            })
    };
    assert_eq!(
        [pre_draw(1), pre_draw(2), pre_draw(3)],
        [ratio(5, 8), ratio(1, 2), ratio(1, 2)]
    );
    assert!(
        ratio(5, 8) <= ratio(7, 10),
        "before the draw `<= 0.7` would be proved"
    );

    let (minimum, min_witness) =
        decide(&world, &build(&world, "Min"), Bound::AtLeast(ratio(9, 20)));
    assert_eq!(minimum, ratio(3, 8));
    assert!(
        minimum < ratio(9, 20),
        "`>= 0.45` is refuted on the minimum"
    );
    assert!(
        ratio(1, 2) >= ratio(9, 20),
        "before the draw `>= 0.45` would be proved"
    );
    assert_eq!(
        min_witness,
        [row(&[("A", 1)], &[("B", 1)]), row(&[("A", 3)], &[("B", 2)])]
    );
}

/// FR-204-AC-11 (TC-642 step 7): with a second free operation `C` each row
/// assigns both `B` and `C`; the maximum `1` takes `B` 1, `C` 2 and the
/// minimum `0` takes `B` 2, `C` 1.
#[trace("TC-642", "FR-204-AC-11")]
#[test]
fn every_free_identity_is_in_each_row() {
    let world = race(
        "2 ms: 1",
        1,
        &operation("C", "self.phase = 0 and self.x >= 1 ms", "self.phase = 2"),
        &["A", "B", "C"],
        &extrema_claims(5, "0.5", "0.5"),
    );
    let built = build(&world, "Max");
    let rows = row_choices(&world, &built);
    let expected: Vec<Named> = (1..=3)
        .flat_map(|b| (1..=3).map(move |c| row(&[("A", 2)], &[("B", b), ("C", c)])))
        .collect();
    assert_eq!(rows, [expected]);
    assert_eq!(actions(&rows), 9);

    let (maximum, max_witness) = decide(&world, &built, Bound::AtMost(ratio(1, 2)));
    assert_eq!(maximum, ratio(1, 1));
    assert_eq!(max_witness, [row(&[("A", 2)], &[("B", 1), ("C", 2)])]);
    let (minimum, min_witness) = decide(&world, &build(&world, "Min"), Bound::AtLeast(ratio(1, 2)));
    assert_eq!(minimum, ratio(0, 1));
    assert_eq!(min_witness, [row(&[("A", 2)], &[("B", 2), ("C", 1)])]);
}

/// FR-204-AC-12's `Loop` with `A`'s delay `a_delay`: `B` resets `x` and keeps
/// `Idle`. `Reach` is AC-12's claim; `Cost` is the expected number of `B`
/// steps until `b`.
fn loop_world(a_delay: &str) -> World {
    model(
        &[("A", &["phase", "late"]), ("B", &["x"])],
        &format!(
            "{}{}\
             time invariant Wait when {{ self.phase = 0 }} {{ self.x <= 2 ms }}\n\
             delay M::Call::A ~ discrete {{ {a_delay} }};\n\
             reward steps on M::Call::B = 1;\n{}\
             probabilistic Reach using inf on M over (c: M::Call) under Both on origin \
             {{ probability <= 0.9 [ eventually holds(c.late) ] }}\n\
             probabilistic Cost using inf on M over (c: M::Call) under Both on origin \
             {{ expected accumulate steps until holds(c.late) <= 5 }}\n",
            operation("A", "self.phase = 0", "self.phase = 2 and self.late = true"),
            operation("B", "self.phase = 0 and self.x >= 1 ms", "self.x = 0 ms"),
            workload("Both", &["A", "B"]),
        ),
    )
}

/// A fair scheduler's deterministic choices as an evaluator policy.
fn deterministic(policy: &[Option<SchedulerChoice>]) -> Vec<Option<usize>> {
    policy
        .iter()
        .map(|choice| match choice {
            Some(SchedulerChoice::Deterministic(ActionChoice(index))) => Some(*index),
            _ => None,
        })
        .collect()
}

/// FR-204 "row-factored operator" for FR-200's fair reachability: over
/// AC-12's `Loop` the fair maximum is `1` and its scheduler takes the end
/// component's exit, `B` delay 2, not the row choice `B` delay 1.
#[trace("TC-642", "FR-204-AC-12")]
#[test]
fn fair_reachability_takes_the_row_factored_exit() {
    let world = loop_world("2 ms: 1");
    let built = build(&world, "Reach");
    let start = built.initial[0].2;
    let target: Vec<bool> = built.labels.iter().map(|labels| labels[0]).collect();
    let result = decide_fair_reachability(
        &built,
        None,
        &target,
        UnboundedObjective::Max,
        &ExactProbLimits::default(),
    )
    .expect("the fair maximum is decided");
    assert_eq!(result.values.len(), built.states.len());
    assert_eq!(
        result.values[start.index()],
        ExtRational::Finite(ratio(1, 1))
    );
    assert_eq!(
        witness(&world, &built, start, &deterministic(&result.policy)),
        [row(&[("A", 2)], &[("B", 2)])]
    );
}

/// FR-204: with `A` drawing 1 or 2 ms, `Loop`'s initial state has two rows.
/// FR-200's fair reachability and fair expected reward decide it through the
/// row-factored operator: maximum reachability `1`; minimum expected `B`
/// steps `1/4` (row `[1]` takes `B` delay 2, where `A` wins alone; row `[2]`
/// takes the tie, `1/2`, over returning at cost 1); maximum `3/2` (both rows
/// take `B` delay 1).
#[trace("TC-642", "FR-204")]
#[test]
fn fair_decisions_read_two_rows_through_the_row_factored_operator() {
    let world = loop_world("1 ms: 1, 2 ms: 1");
    let built = build(&world, "Reach");
    let start = built.initial[0].2;
    assert_eq!(actions(&row_choices(&world, &built)), 4);
    let target: Vec<bool> = built.labels.iter().map(|labels| labels[0]).collect();
    let limits = ExactProbLimits::default();
    let reach = decide_fair_reachability(&built, None, &target, UnboundedObjective::Max, &limits)
        .expect("the fair maximum is decided");
    assert_eq!(
        reach.values[start.index()],
        ExtRational::Finite(ratio(1, 1))
    );

    let built = build(&world, "Cost");
    let start = built.initial[0].2;
    let target: Vec<bool> = built.labels.iter().map(|labels| labels[0]).collect();
    let minimum =
        decide_fair_expected_reward(&built, &target, "steps", UnboundedObjective::Min, &limits)
            .expect("the fair minimum is decided");
    assert_eq!(minimum.values.len(), built.states.len());
    assert_eq!(
        minimum.values[start.index()],
        ExtRational::Finite(ratio(1, 4))
    );
    let policy = deterministic(minimum.policy.as_deref().expect("a fair scheduler"));
    assert_eq!(
        witness(&world, &built, start, &policy),
        [row(&[("A", 1)], &[("B", 2)]), row(&[("A", 2)], &[("B", 2)])]
    );
    let maximum =
        decide_fair_expected_reward(&built, &target, "steps", UnboundedObjective::Max, &limits)
            .expect("the fair maximum is decided");
    assert_eq!(
        maximum.values[start.index()],
        ExtRational::Finite(ratio(3, 2))
    );
}

/// FR-204-AC-12 (TC-642 step 8): `Loop`'s `B` delay 1 returns to the initial
/// state and delay 2 ties with `A`. Both attain the maximum's upper iterate
/// 1; the maximum `1` takes the end component's exit, `B` delay 2, and the
/// chain `B` delay 1 induces never reaches `b`: it is the minimum's choice,
/// with value `0`.
#[trace("TC-642", "FR-204-AC-12")]
#[test]
fn loop_witness_takes_the_end_component_exit() {
    let world = loop_world("2 ms: 1");
    let built = build(&world, "Reach");
    let start = built.initial[0].2;
    assert_eq!(
        row_choices(&world, &built),
        [vec![
            row(&[("A", 2)], &[("B", 1)]),
            row(&[("A", 2)], &[("B", 2)])
        ]]
    );
    let returns = built.edges.iter().any(|edge| {
        edge.from == start
            && edge.to == start
            && matches!(&edge.choice, ProductAction::FreeDelays(rows) if rows[0].free[0].1 == 1)
    });
    assert!(returns, "`B` delay 1 wins and returns to the initial state");
    let target: Vec<bool> = built.labels.iter().map(|labels| labels[0]).collect();
    // ADR-026 SD-3 takes tied identities in order at one instant, each while
    // still enabled: with `B` first, `A` is still enabled and sets `b`, so
    // every outcome of the tie reaches the target.
    let tie: Vec<_> = built
        .edges
        .iter()
        .filter(|edge| {
            edge.from == start
                && matches!(&edge.choice, ProductAction::FreeDelays(rows) if rows[0].free[0].1 == 2)
        })
        .collect();
    assert_eq!(tie.len(), 2, "one edge per order of the tie");
    assert!(tie.iter().all(|edge| target[edge.to.index()]));

    let limits = ExactProbLimits::default();
    let maximum = decide_reachability(&built, &target, UnboundedObjective::Max, &limits)
        .expect("the labelled product is solved");
    assert_eq!(
        maximum.values[start.index()],
        ExtRational::Finite(ratio(1, 1))
    );
    assert!(
        ratio(1, 1) > ratio(9, 10),
        "`<= 0.9` is refuted on the maximum"
    );
    assert_eq!(
        witness(&world, &built, start, &maximum.policy),
        [row(&[("A", 2)], &[("B", 2)])]
    );
    let minimum = decide_reachability(&built, &target, UnboundedObjective::Min, &limits)
        .expect("the labelled product is solved");
    assert_eq!(
        minimum.values[start.index()],
        ExtRational::Finite(ratio(0, 1))
    );
    assert_eq!(
        witness(&world, &built, start, &minimum.policy),
        [row(&[("A", 2)], &[("B", 1)])]
    );
}

/// FR-196-AC-14 (TC-631 step 12): a given `A` drawing 9 or 10 digital units
/// and a free `B` in `[1, 2]` make two rows, given `[9]` before `[10]`
/// (numeric order), each assigning `B` alone; four actions, and no state key
/// carries a pending given draw.
#[trace("TC-631", "FR-196-AC-14")]
#[test]
fn rows_are_in_numeric_order_and_add_no_key_member() {
    let world = model(
        &[("A", &["phase"]), ("B", &["phase", "late"])],
        &format!(
            "{}{}\
             time invariant Wait when {{ self.phase = 0 }} {{ self.x <= 10 ms }}\n\
             delay M::Call::A ~ discrete {{ 10 ms: 1, 9 ms: 1 }};\n{}{}",
            operation("A", "self.phase = 0", "self.phase = 2"),
            operation(
                "B",
                "self.phase = 0 and self.x >= 1 ms and self.x <= 2 ms",
                "self.phase = 2 and self.late = true",
            ),
            workload("Both", &["A", "B"]),
            claim(
                "Rows",
                "under Both",
                ">= 0.5",
                "eventually[0 ms, 12 ms] holds(c.late)"
            ),
        ),
    );
    let built = build(&world, "Rows");
    let rows = row_choices(&world, &built);
    assert_eq!(
        rows,
        [
            vec![row(&[("A", 9)], &[("B", 1)]), row(&[("A", 9)], &[("B", 2)])],
            vec![
                row(&[("A", 10)], &[("B", 1)]),
                row(&[("A", 10)], &[("B", 2)])
            ],
        ]
    );
    assert_eq!(actions(&rows), 4);
    assert!(built.states.iter().all(|key| key.pending.is_none()));
    let start = built.initial[0].2;
    let mass = |given: u64, free: u64| {
        built
            .edges
            .iter()
            .filter(|edge| {
                edge.from == start
                    && matches!(&edge.choice, ProductAction::FreeDelays(rows)
                        if rows[0].given[0].1 == given && rows[0].free[0].1 == free)
            })
            .fold(ratio(0, 1), |sum, edge| sum.add(&edge.probability))
    };
    for given in [9, 10] {
        for free in [1, 2] {
            assert_eq!(mass(given, free), ratio(1, 2), "P(g) folded into the row");
        }
    }
}

/// The racing identity `name` among the initial state's row choices.
fn racer(world: &World, product: &ProbProduct, name: &str) -> ScheduledIdentity {
    product
        .outgoing(product.initial[0].2)
        .iter()
        .find_map(|edge| match &edge.choice {
            ProductAction::FreeDelays(rows) => rows
                .iter()
                .flat_map(|row| row.given.iter().chain(&row.free))
                .find(|(identity, _)| short(world, &identity.operation) == name)
                .map(|(identity, _)| identity.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no racer {name}"))
}

fn schedule(world: &World, product: &ProbProduct, name: &str) -> Schedule {
    Schedule::of(&world.system(), &racer(world, product, name)).expect("the schedule encodes")
}

/// The initial state's race as the spec states it: each given racer's
/// conditioned support and each free racer's window, in digital units.
fn stated_race(
    world: &World,
    product: &ProbProduct,
    given: &[(&str, &[u64])],
    free: &[(&str, &[u64])],
) -> FreeDelaysRace {
    let given = given
        .iter()
        .map(|(name, support)| GivenRacer {
            identity: racer(world, product, name),
            schedule: schedule(world, product, name),
            support: support.iter().copied().collect(),
        })
        .collect();
    let free = free
        .iter()
        .map(|(name, window)| FreeRacer {
            identity: racer(world, product, name),
            schedule: schedule(world, product, name),
            window: window.iter().copied().collect(),
        })
        .collect();
    FreeDelaysRace::new(given, free).expect("a mixed race")
}

fn body_of(world: &World, rows: &[FreeDelayRow]) -> Vec<FreeDelayActionRow> {
    match ReplayProductAction::free_delays(&world.system(), rows).expect("the body encodes") {
        ReplayProductAction::FreeDelays(rows) => rows,
        other => panic!("a free-delays body: {other:?}"),
    }
}

/// Write `body` as JCS octets, read it back and admit it at `race`.
fn admit(
    race: &FreeDelaysRace,
    body: ReplayProductAction,
) -> Result<Vec<FreeDelayRow>, ProductActionRefusal> {
    let bytes = body.to_bytes().expect("the body writes");
    race.admit(&ReplayProductAction::parse(&bytes)?)
}

/// Every refusal of `body` at `race` is `cause`, `invalid_runtime_input`/
/// `invalid-value`.
fn refuses(race: &FreeDelaysRace, rows: Vec<FreeDelayActionRow>, cause: ProductActionRefusal) {
    let refusal = admit(race, ReplayProductAction::FreeDelays(rows)).expect_err("the body refuses");
    assert_eq!(refusal, cause);
    assert_eq!(
        refusal.code(),
        CatalogCode::new("invalid_runtime_input", "invalid-value")
    );
}

fn accepted(product: &ProbProduct) -> Vec<bool> {
    let mut accepted = vec![false; product.states.len()];
    for (state, verdict) in &product.decided {
        if *verdict == MonitorVerdict::Accept {
            accepted[state.index()] = true;
        }
    }
    accepted
}

/// The whole action a witness step or entry names.
fn action_rows(world: &World, action: &ProductAction) -> Vec<Named> {
    let ProductAction::FreeDelays(rows) = action else {
        panic!("a FreeDelays action: {action:?}");
    };
    rows.iter().map(|row| named(world, row)).collect()
}

/// The mass the row choice `row` gives the initial state's successor `to`.
fn row_mass(product: &ProbProduct, row: &FreeDelayRow, to: ProbStateId) -> Rational {
    product
        .outgoing(product.initial[0].2)
        .iter()
        .filter(|edge| {
            edge.to == to
                && matches!(&edge.choice, ProductAction::FreeDelays(body)
                    if body.as_slice() == std::slice::from_ref(row))
        })
        .fold(ratio(0, 1), |sum, edge| sum.add(&edge.probability))
}

/// FR-204-AC-10 (TC-637, TC-642 step 6): `Detour`'s maximum
/// witness entry is the `FreeDelays` action given `[1]` to `B` delay 2 and
/// given `[3]` to `B` delay 1, and its path set is `A` at 1 ms (`1/2`) then
/// `B` at 1 ms followed by `G` (`1/4`): `3/4 > 7/10`. The first step's merged
/// `1/2` comes from row `[1]` alone and the `B` step's `1/2` from row `[3]`.
/// One path alone does not pass the bound.
#[trace("TC-637")]
#[trace("TC-642", "FR-204-AC-10")]
#[test]
fn detour_maximum_witness_is_a_two_path_set_of_merged_steps() {
    let world = detour(&claim(
        "Max",
        "under All",
        "<= 0.7",
        "eventually[0 ms, 2 ms] holds(c.late)",
    ));
    let built = build(&world, "Max");
    let FhResult::Exact { policy, .. } = decide_bounded_event(
        &built,
        &Bound::AtMost(ratio(7, 10)),
        &ExactProbLimits::default(),
    )
    .expect("the digital product is acyclic") else {
        panic!("exact values");
    };
    let start = built.initial[0].2;
    let produce = |max_witness_paths| {
        produce_witness(
            &built,
            &policy,
            WitnessScope::Workload,
            start,
            &accepted(&built),
            &ratio(7, 10),
            &ExactProbLimits {
                max_witness_paths,
                ..ExactProbLimits::default()
            },
        )
        .expect("the policy induces a chain")
    };
    let witness = produce(2).expect("two paths pass the bound");
    let expected = [row(&[("A", 1)], &[("B", 2)]), row(&[("A", 3)], &[("B", 1)])];
    // FR-204 "Evidence": the race state's entry, and the entry of the
    // continuation state the chain also reaches (a unit delay no emitted path
    // leaves).
    let [(entry_state, entry), (continuation, delay)] = witness.entries.as_slice() else {
        panic!(
            "the race state and one continuation state: {:?}",
            witness.entries
        );
    };
    assert_eq!(*entry_state, start);
    assert_eq!(action_rows(&world, entry), expected);
    assert!(matches!(delay, ProductAction::UnitDelay));
    assert!(witness.paths.iter().all(|path| path
        .steps
        .iter()
        .take(path.steps.len() - 1)
        .all(|step| step.to != *continuation)));

    let probabilities: Vec<Rational> = witness
        .paths
        .iter()
        .map(|path| path.probability.clone())
        .collect();
    assert_eq!(probabilities, [ratio(1, 2), ratio(1, 4)]);
    let sum = probabilities.iter().fold(ratio(0, 1), |sum, p| sum.add(p));
    assert_eq!(sum, ratio(3, 4));
    assert!(sum > ratio(7, 10));

    let [a_wins] = witness.paths[0].steps.as_slice() else {
        panic!("A at 1 ms is one step");
    };
    let [b_wins, g_wins] = witness.paths[1].steps.as_slice() else {
        panic!("B at 1 ms then G is two steps");
    };
    for merged in [a_wins, b_wins] {
        assert_eq!(action_rows(&world, &merged.action), expected);
        assert!(merged.draw.is_none(), "a merged step carries no draw");
        assert_eq!(merged.probability, ratio(1, 2));
    }
    assert!(!matches!(g_wins.action, ProductAction::FreeDelays(_)));
    assert_eq!(g_wins.probability, ratio(1, 2));
    let ProductAction::FreeDelays(rows) = entry else {
        unreachable!("asserted above");
    };
    assert_eq!(row_mass(&built, &rows[0], a_wins.to), ratio(1, 2));
    assert_eq!(row_mass(&built, &rows[1], a_wins.to), ratio(0, 1));
    assert_eq!(row_mass(&built, &rows[0], b_wins.to), ratio(0, 1));
    assert_eq!(row_mass(&built, &rows[1], b_wins.to), ratio(1, 2));

    assert!(produce(1).is_none(), "`1/2` alone does not pass `7/10`");

    // A path probability wider than `max_rational_bits` is unproducible.
    let narrow = ExactProbLimits {
        max_rational_bits: 1,
        ..ExactProbLimits::default()
    };
    let produce_with = |limits: &ExactProbLimits| {
        produce_witness(
            &built,
            &policy,
            WitnessScope::Workload,
            start,
            &accepted(&built),
            &ratio(7, 10),
            limits,
        )
        .expect("the policy induces a chain")
    };
    assert!(produce_with(&narrow).is_none(), "`1/2` needs two bits");
    // Frontier pushes count against `max_transitions`: the two expansions
    // fit in 2, the four pushes do not.
    let mut few = ExactProbLimits::default();
    few.model.limits.max_transitions = 2;
    assert!(produce_with(&few).is_none());
    assert!(produce_with(&ExactProbLimits::default()).is_some());
}

/// TC-637: the `Detour` witness action's canonical body admits
/// at the race state (`A` drawing 1 or 3, `B` in `[1, 3]`); with its rows
/// swapped, with the row given `[3]` removed, with `B` delay 4, or as
/// `WorkloadResolve{B, 1}` it refuses `invalid_runtime_input`/`invalid-value`.
#[trace("TC-637")]
#[test]
fn detour_witness_body_admits_and_its_variants_refuse() {
    let world = detour(&claim(
        "Max",
        "under All",
        "<= 0.7",
        "eventually[0 ms, 2 ms] holds(c.late)",
    ));
    let built = build(&world, "Max");
    let FhResult::Exact { policy, .. } = decide_bounded_event(
        &built,
        &Bound::AtMost(ratio(7, 10)),
        &ExactProbLimits::default(),
    )
    .expect("the digital product is acyclic") else {
        panic!("exact values");
    };
    let start = built.initial[0].2;
    let Some(ProductAction::FreeDelays(rows)) = free_delays_choice(&built, start, &policy) else {
        panic!("a FreeDelays witness");
    };
    let race = stated_race(&world, &built, &[("A", &[1, 3])], &[("B", &[1, 2, 3])]);
    let body = body_of(&world, &rows);
    assert_eq!(
        admit(&race, ReplayProductAction::FreeDelays(body.clone())).expect("the witness admits"),
        rows
    );

    let mut swapped = body.clone();
    swapped.reverse();
    refuses(&race, swapped, ProductActionRefusal::RowOrder { row: 1 });
    let mut removed = body.clone();
    removed.truncate(1);
    refuses(&race, removed, ProductActionRefusal::RowsMissing);
    let mut late = body;
    late[1].free[0].1 = 4;
    refuses(
        &race,
        late,
        ProductActionRefusal::FreeOutsideWindow { row: 1 },
    );
    let resolve = admit(
        &race,
        ReplayProductAction::WorkloadResolve {
            schedule: schedule(&world, &built, "B"),
            delay: 1,
        },
    )
    .expect_err("WorkloadResolve is not offered at a mixed race");
    assert_eq!(resolve, ProductActionRefusal::Disabled);
    assert_eq!(
        resolve.code(),
        CatalogCode::new("invalid_runtime_input", "invalid-value")
    );
}

/// FR-204-AC-11 (TC-642 step 7): the maximum's row `B` delay 1, `C` delay 2
/// admits at the race state; the same row naming `B` alone refuses
/// `invalid_runtime_input`/`invalid-value`.
#[trace("TC-642", "FR-204-AC-11")]
#[test]
fn a_witness_row_naming_b_alone_refuses() {
    let world = race(
        "2 ms: 1",
        1,
        &operation("C", "self.phase = 0 and self.x >= 1 ms", "self.phase = 2"),
        &["A", "B", "C"],
        &extrema_claims(5, "0.5", "0.5"),
    );
    let built = build(&world, "Max");
    let FhResult::Exact { policy, .. } = decide_bounded_event(
        &built,
        &Bound::AtMost(ratio(1, 2)),
        &ExactProbLimits::default(),
    )
    .expect("the digital product is acyclic") else {
        panic!("exact values");
    };
    let Some(ProductAction::FreeDelays(rows)) =
        free_delays_choice(&built, built.initial[0].2, &policy)
    else {
        panic!("a FreeDelays witness");
    };
    let race = stated_race(
        &world,
        &built,
        &[("A", &[2])],
        &[("B", &[1, 2, 3]), ("C", &[1, 2, 3])],
    );
    let body = body_of(&world, &rows);
    assert_eq!(
        admit(&race, ReplayProductAction::FreeDelays(body.clone()))
            .expect("both free delays admit"),
        rows
    );
    let c = schedule(&world, &built, "C");
    let mut alone = body;
    alone[0].free.retain(|(schedule, _)| *schedule != c);
    assert_eq!(alone[0].free.len(), 1, "`B` alone");
    refuses(
        &race,
        alone,
        ProductActionRefusal::FreeIdentities { row: 0 },
    );
}

/// FR-204-AC-12 (TC-642 step 8): over `Loop` the maximum's witness entry is
/// `B` delay 2, the end component's exit, and its path set reaches `b` with
/// probability `1 > 9/10`; the minimum's entry `B` delay 1 induces a chain
/// that never reaches `b`, so it has no path set. The entry's body admits.
#[trace("TC-642", "FR-204-AC-12")]
#[test]
fn loop_witness_reaches_b_through_the_exit() {
    let world = loop_world("2 ms: 1");
    let built = build(&world, "Reach");
    let start = built.initial[0].2;
    let target: Vec<bool> = built.labels.iter().map(|labels| labels[0]).collect();
    let limits = ExactProbLimits::default();
    let produce = |objective| {
        let solved = decide_reachability(&built, &target, objective, &limits)
            .expect("the labelled product is solved");
        produce_witness(
            &built,
            &solved.policy,
            WitnessScope::Workload,
            start,
            &target,
            &ratio(9, 10),
            &limits,
        )
        .expect("the policy induces a chain")
    };
    let witness = produce(UnboundedObjective::Max).expect("the exit reaches b");
    let exit = [row(&[("A", 2)], &[("B", 2)])];
    let [(entry_state, entry)] = witness.entries.as_slice() else {
        panic!("one entry: {:?}", witness.entries);
    };
    assert_eq!(*entry_state, start);
    assert_eq!(action_rows(&world, entry), exit);
    let sum = witness
        .paths
        .iter()
        .fold(ratio(0, 1), |sum, path| sum.add(&path.probability));
    assert_eq!(sum, ratio(1, 1));
    assert!(witness
        .paths
        .iter()
        .all(|path| path.steps.len() == 1 && action_rows(&world, &path.steps[0].action) == exit));
    assert!(
        produce(UnboundedObjective::Min).is_none(),
        "`B` delay 1 never reaches b"
    );

    let ProductAction::FreeDelays(rows) = entry else {
        unreachable!("asserted above");
    };
    let race = stated_race(&world, &built, &[("A", &[2])], &[("B", &[1, 2])]);
    assert_eq!(
        admit(
            &race,
            ReplayProductAction::FreeDelays(body_of(&world, rows))
        )
        .expect("the exit admits"),
        *rows
    );
}

/// FR-196-AC-14 (TC-631 step 12): each of the four `FreeDelays` actions over
/// `A` drawing 9 or 10 units and a free `B` in `[1, 2]` has the canonical
/// body listing the row given `["9"]` before `["10"]`, although `"10"`
/// precedes `"9"` as bytes, with `given` naming `A` and `free` `B` only. The
/// body in byte order, with a row missing or repeated, with `free` empty or
/// naming `A`, or with `B` delay 3 refuses.
#[trace("TC-631", "FR-196-AC-14")]
#[test]
fn free_delays_bodies_are_in_numeric_row_order_and_malformed_ones_refuse() {
    let world = model(
        &[("A", &["phase"]), ("B", &["phase", "late"])],
        &format!(
            "{}{}\
             time invariant Wait when {{ self.phase = 0 }} {{ self.x <= 10 ms }}\n\
             delay M::Call::A ~ discrete {{ 10 ms: 1, 9 ms: 1 }};\n{}{}",
            operation("A", "self.phase = 0", "self.phase = 2"),
            operation(
                "B",
                "self.phase = 0 and self.x >= 1 ms and self.x <= 2 ms",
                "self.phase = 2 and self.late = true",
            ),
            workload("Both", &["A", "B"]),
            claim(
                "Rows",
                "under Both",
                ">= 0.5",
                "eventually[0 ms, 12 ms] holds(c.late)"
            ),
        ),
    );
    let built = build(&world, "Rows");
    let mut choices: Vec<FreeDelayRow> = Vec::new();
    for edge in built.outgoing(built.initial[0].2) {
        let ProductAction::FreeDelays(body) = &edge.choice else {
            panic!("a mixed race offers FreeDelays only");
        };
        if let [only] = body.as_slice() {
            if !choices.contains(only) {
                choices.push(only.clone());
            }
        }
    }
    let (nine, ten): (Vec<_>, Vec<_>) = choices.into_iter().partition(|row| row.given[0].1 == 9);
    let race = stated_race(&world, &built, &[("A", &[9, 10])], &[("B", &[1, 2])]);
    let (a, b) = (schedule(&world, &built, "A"), schedule(&world, &built, "B"));
    let text =
        |schedule: &Schedule| String::from_utf8(schedule.as_bytes().to_vec()).expect("UTF-8");
    let mut actions = 0;
    for first in &nine {
        for second in &ten {
            actions += 1;
            // Handed over in byte order; the body is in numeric order.
            let rows = [second.clone(), first.clone()];
            let body =
                ReplayProductAction::free_delays(&world.system(), &rows).expect("the body encodes");
            let bytes = String::from_utf8(body.to_bytes().expect("writes")).expect("UTF-8");
            assert_eq!(
                bytes,
                format!(
                    r#"["free-delays",[[[[{a},"9"]],[[{b},"{}"]]],[[[{a},"10"]],[[{b},"{}"]]]]]"#,
                    first.free[0].1,
                    second.free[0].1,
                    a = text(&a),
                    b = text(&b),
                )
            );
            assert_eq!(
                admit(&race, body).expect("each action admits"),
                [first.clone(), second.clone()]
            );
        }
    }
    assert_eq!(actions, 4);

    // `B` at delay 1 wins at 1 unit after either draw: the two rows reach one
    // post-state with one elapsed amount, one merged step of mass `1/2 + 1/2`.
    let b_first = |rows: &[FreeDelayRow]| {
        rows.iter()
            .find(|row| row.free[0].1 == 1)
            .cloned()
            .expect("B at delay 1")
    };
    let both = ProductAction::FreeDelays(vec![b_first(&nine), b_first(&ten)]);
    let steps = action_steps(&built, built.initial[0].2, &both).expect("an offered action");
    let [merged] = steps.as_slice() else {
        panic!("one merged step: {steps:?}");
    };
    assert_eq!(merged.probability, ratio(1, 1));
    assert!(merged.draw.is_none());

    let body = body_of(&world, &[nine[0].clone(), ten[0].clone()]);
    let mut byte_order = body.clone();
    byte_order.reverse();
    refuses(&race, byte_order, ProductActionRefusal::RowOrder { row: 1 });
    refuses(
        &race,
        vec![body[0].clone()],
        ProductActionRefusal::RowsMissing,
    );
    refuses(
        &race,
        vec![body[0].clone(), body[0].clone()],
        ProductActionRefusal::RowRepeated { row: 1 },
    );
    let mut empty = body.clone();
    empty[0].free.clear();
    refuses(
        &race,
        empty,
        ProductActionRefusal::FreeIdentities { row: 0 },
    );
    let mut names_a = body.clone();
    names_a[1].free[0].0 = a.clone();
    refuses(
        &race,
        names_a,
        ProductActionRefusal::FreeIdentities { row: 1 },
    );
    let mut late = body;
    late[0].free[0].1 = 3;
    refuses(
        &race,
        late,
        ProductActionRefusal::FreeOutsideWindow { row: 0 },
    );
}

/// `Detour`'s maximum policy over the `Max` claim.
fn detour_maximum(world: &World) -> (ProbProduct, Vec<Option<usize>>) {
    let built = build(world, "Max");
    let FhResult::Exact { policy, .. } = decide_bounded_event(
        &built,
        &Bound::AtMost(ratio(7, 10)),
        &ExactProbLimits::default(),
    )
    .expect("the digital product is acyclic") else {
        panic!("exact values");
    };
    (built, policy)
}

fn detour_max_claim() -> World {
    detour(&claim(
        "Max",
        "under All",
        "<= 0.7",
        "eventually[0 ms, 2 ms] holds(c.late)",
    ))
}

/// FR-204 "Evidence" (TC-642): the witness scheduler has an entry at every
/// digital evidence and continuation state, not only at the states the
/// emitted paths leave. Under the workload `All` `Detour`'s maximum scheduler
/// also names a continuation state beside the paths.
#[trace("TC-642", "FR-204")]
#[test]
fn witness_entries_cover_continuation_states_beside_the_paths() {
    let world = detour_max_claim();
    let (built, policy) = detour_maximum(&world);
    let start = built.initial[0].2;
    let witness = produce_witness(
        &built,
        &policy,
        WitnessScope::Workload,
        start,
        &accepted(&built),
        &ratio(7, 10),
        &ExactProbLimits::default(),
    )
    .expect("the policy induces a chain")
    .expect("two paths pass the bound");
    let mut left: Vec<ProbStateId> = Vec::new();
    for path in &witness.paths {
        let sources = std::iter::once(start).chain(path.steps.iter().map(|step| step.to));
        left.extend(sources.take(path.steps.len()));
    }
    let entered: Vec<ProbStateId> = witness.entries.iter().map(|(state, _)| *state).collect();
    assert!(
        entered.contains(&start),
        "the race state has an entry: {entered:?}"
    );
    assert!(
        entered.iter().any(|state| !left.contains(state)),
        "a continuation state without an emitted path has an entry: {entered:?}"
    );
    assert!(entered.windows(2).all(|pair| pair[0] < pair[1]));
}

/// A randomized policy over `Detour`'s product: `first` and `second` are the
/// choices of the race state's two rows, and every other node takes choice 0.
fn randomized_policy(
    built: &ProbProduct,
    first: SchedulerChoice,
    second: SchedulerChoice,
) -> Vec<Option<SchedulerChoice>> {
    let mut policy: Vec<Option<SchedulerChoice>> =
        vec![Some(SchedulerChoice::Deterministic(ActionChoice(0))); built.states.len()];
    policy.push(Some(first));
    policy.push(Some(second));
    policy
}

fn weights(list: &[(usize, Rational)]) -> SchedulerChoice {
    SchedulerChoice::Randomized(
        list.iter()
            .map(|(index, mass)| (ActionChoice(*index), mass.clone()))
            .collect(),
    )
}

fn entries_of(
    built: &ProbProduct,
    policy: &[Option<SchedulerChoice>],
    limits: &ExactProbLimits,
) -> Result<Vec<(ProbStateId, EntryChoice)>, WitnessError> {
    witness_scheduler(
        built,
        policy,
        WitnessScope::Workload,
        built.initial[0].2,
        &accepted(built),
        limits,
    )
}

/// The entry at the race state, which is the first.
fn race_entry(entries: &[(ProbStateId, EntryChoice)]) -> Vec<(&ProductAction, Rational)> {
    let Some((_, EntryChoice::Randomized(alternatives))) = entries.first() else {
        panic!("a randomized entry at the race state: {entries:?}");
    };
    alternatives
        .iter()
        .map(|(action, mass)| (action, mass.clone()))
        .collect()
}

/// FR-202 (TC-642, FR-204): the policy randomizes row `[1]` between `B`
/// delay 1 and 2 with `1/2` each and takes `B` delay 1 on row `[3]`; the
/// witness scheduler's entry at the race state is the exact randomization
/// over the two whole actions, and the continuation state has its own entry.
#[trace("TC-642", "FR-204")]
#[test]
fn randomized_rows_give_an_exact_randomized_entry() {
    let world = detour_max_claim();
    let (built, _) = detour_maximum(&world);
    let start = built.initial[0].2;
    let half = ratio(1, 2);
    let policy = randomized_policy(
        &built,
        weights(&[(0, half.clone()), (1, half.clone())]),
        SchedulerChoice::Deterministic(ActionChoice(0)),
    );
    let entries = entries_of(&built, &policy, &ExactProbLimits::default())
        .expect("the policy gives an entry at each reached state");
    let [(state, EntryChoice::Randomized(_)), (continuation, EntryChoice::Action(delay))] =
        entries.as_slice()
    else {
        panic!("a randomized entry at the race state, then a continuation: {entries:?}");
    };
    assert!(*continuation > *state);
    assert!(matches!(delay, ProductAction::UnitDelay));
    assert_eq!(*state, start);
    let alternatives = race_entry(&entries);
    let [(first, p), (second, q)] = alternatives.as_slice() else {
        panic!("two whole actions: {alternatives:?}");
    };
    assert_eq!((p, q), (&half, &half));
    assert_eq!(
        action_rows(&world, first),
        [row(&[("A", 1)], &[("B", 1)]), row(&[("A", 3)], &[("B", 1)])]
    );
    assert_eq!(
        action_rows(&world, second),
        [row(&[("A", 1)], &[("B", 2)]), row(&[("A", 3)], &[("B", 1)])]
    );
}

/// FR-202 (TC-642, FR-204): with two randomized rows the entry is the product
/// measure, `1/2 * 1/4`, `1/2 * 3/4`, ...: a sum of the row masses, or a
/// dropped row, changes a value; alternatives are in row-major canonical
/// order.
#[trace("TC-642", "FR-204")]
#[test]
fn two_randomized_rows_give_the_product_measure() {
    let world = detour_max_claim();
    let (built, _) = detour_maximum(&world);
    let policy = randomized_policy(
        &built,
        weights(&[(0, ratio(1, 2)), (1, ratio(1, 2))]),
        weights(&[(0, ratio(1, 4)), (2, ratio(3, 4))]),
    );
    let entries = entries_of(&built, &policy, &ExactProbLimits::default())
        .expect("two randomized rows give an entry");
    let alternatives = race_entry(&entries);
    let masses: Vec<Rational> = alternatives.iter().map(|(_, mass)| mass.clone()).collect();
    assert_eq!(masses, [ratio(1, 8), ratio(3, 8), ratio(1, 8), ratio(3, 8)]);
    let rows: Vec<Vec<Named>> = alternatives
        .iter()
        .map(|(action, _)| action_rows(&world, action))
        .collect();
    assert_eq!(
        rows[1],
        [row(&[("A", 1)], &[("B", 1)]), row(&[("A", 3)], &[("B", 3)])]
    );
    assert_eq!(
        rows[2],
        [row(&[("A", 1)], &[("B", 2)]), row(&[("A", 3)], &[("B", 1)])]
    );
}

/// FR-202 (TC-642, FR-204): alternatives are in canonical choice order, not
/// the policy's order.
#[trace("TC-642", "FR-204")]
#[test]
fn alternatives_are_in_canonical_choice_order() {
    let world = detour_max_claim();
    let (built, _) = detour_maximum(&world);
    let half = ratio(1, 2);
    let policy = randomized_policy(
        &built,
        weights(&[(1, half.clone()), (0, half.clone())]),
        SchedulerChoice::Deterministic(ActionChoice(0)),
    );
    let entries = entries_of(&built, &policy, &ExactProbLimits::default())
        .expect("a reordered distribution is the same entry");
    let alternatives = race_entry(&entries);
    let delays: Vec<Vec<Named>> = alternatives
        .iter()
        .map(|(action, _)| action_rows(&world, action))
        .collect();
    assert_eq!(delays[0][0], row(&[("A", 1)], &[("B", 1)]));
    assert_eq!(delays[1][0], row(&[("A", 1)], &[("B", 2)]));
}

/// FR-202 (TC-642, FR-204): zero or negative mass, a repeated choice, a total
/// other than 1 and a choice the row lacks give no entry.
#[trace("TC-642", "FR-204")]
#[test]
fn malformed_distributions_give_no_entry() {
    let world = detour_max_claim();
    let (built, _) = detour_maximum(&world);
    let start = built.initial[0].2;
    let half = ratio(1, 2);
    let det = SchedulerChoice::Deterministic(ActionChoice(0));
    let malformed = [
        weights(&[(0, ratio(1, 1)), (1, ratio(-1, 2))]),
        weights(&[(0, ratio(1, 1)), (1, ratio(0, 1))]),
        weights(&[(0, half.clone()), (0, half.clone())]),
        weights(&[(0, ratio(1, 3))]),
        weights(&[(0, half.clone()), (9, half.clone())]),
    ];
    for bad in malformed {
        for in_first_row in [true, false] {
            let policy = if in_first_row {
                randomized_policy(&built, bad.clone(), det.clone())
            } else {
                randomized_policy(&built, det.clone(), bad.clone())
            };
            assert!(
                matches!(
                    entries_of(&built, &policy, &ExactProbLimits::default()),
                    Err(WitnessError::NoChoice { state }) if state == start.index()
                ),
                "{bad:?} refuses"
            );
        }
    }
}

/// FR-202 (TC-642, FR-204): the entry's alternatives are bounded by the
/// transition limit; one more than the bound refuses, the bound itself
/// admits.
#[trace("TC-642", "FR-204")]
#[test]
fn randomized_entries_are_bounded_by_the_transition_limit() {
    let world = detour_max_claim();
    let (built, _) = detour_maximum(&world);
    let start = built.initial[0].2;
    let policy = randomized_policy(
        &built,
        weights(&[(0, ratio(1, 2)), (1, ratio(1, 2))]),
        weights(&[(0, ratio(1, 2)), (1, ratio(1, 2))]),
    );
    let capped = |max: usize| {
        let mut limits = ExactProbLimits::default();
        limits.model.limits.max_transitions = max;
        limits
    };
    assert!(matches!(
        entries_of(&built, &policy, &capped(3)),
        Err(WitnessError::TooManyAlternatives { state }) if state == start.index()
    ));
    assert_eq!(
        race_entry(&entries_of(&built, &policy, &capped(4)).expect("four alternatives fit")).len(),
        4
    );
}
