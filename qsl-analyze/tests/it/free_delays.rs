// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-204 (TC-642, TC-631): the post-draw `FreeDelays` action of a digital
//! product where given-delay and free-delay identities race together under a
//! workload, its row choices, the row-factored values and the policy's
//! whole-action choice.
//!
//! The fixture's `Call` stands for FR-204's models: `phase` 0 is `Idle`, 1
//! `Half` and 2 `Done`, and `late` is `b`.

use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::{
    build_product, decide_bounded_event, decide_reachability, free_delays_choice, ExactProbLimits,
    ExtRational, FhResult, FreeDelayRow, ProbBuild, ProbProduct, ProductAction, ProductKind,
    ProductRequest, UnboundedObjective,
};
use qsl_eval::model_subject::ModelSubject;
use qsl_semantics::check::Bound;
use quire_exact::{Cancel, Integer, Rational};

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
/// 1, 2 and 3; the minimum `0` takes `B` delay 3 and the maximum `1` delay 1.
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
/// minimum `0`.
#[trace("TC-642", "FR-204-AC-7")]
#[test]
fn a_later_guard_leaves_the_tie_as_the_maximum() {
    let world = race(
        "2 ms: 1",
        2,
        "",
        &["A", "B"],
        &extrema_claims(5, "0.4", "0.5"),
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

/// FR-204-AC-12 (TC-642 step 8): `Loop`'s `B` delay 1 returns to the initial
/// state and delay 2 ties with `A`. Both attain the maximum's upper iterate
/// 1; the maximum `1` takes the end component's exit, `B` delay 2, and the
/// chain `B` delay 1 induces never reaches `b`: it is the minimum's choice,
/// with value `0`.
#[trace("TC-642", "FR-204-AC-12")]
#[test]
fn loop_witness_takes_the_end_component_exit() {
    let world = model(
        &[("A", &["phase", "late"]), ("B", &["x"])],
        &format!(
            "{}{}\
             time invariant Wait when {{ self.phase = 0 }} {{ self.x <= 2 ms }}\n\
             delay M::Call::A ~ discrete {{ 2 ms: 1 }};\n{}\
             probabilistic Reach using inf on M over (c: M::Call) under Both on origin \
             {{ probability <= 0.9 [ eventually holds(c.late) ] }}\n",
            operation("A", "self.phase = 0", "self.phase = 2 and self.late = true"),
            operation("B", "self.phase = 0 and self.x >= 1 ms", "self.x = 0 ms"),
            workload("Both", &["A", "B"]),
        ),
    );
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
