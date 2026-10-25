// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 "Hyper self-composition queries": the scripts and refusals of
//! `encode_hyper_query` over ADR-023 section 8's vault.

use ix_trace_rs::trace;
use qsl_eval::hyper::{CheckedStepMatch, StepLabel, StepMatch};
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::smt::{Measure, MonitorTable, Refusal};
use qsl_replay::{
    encode_hyper_query, HyperProductIdentity, HyperQuery, HyperQueryRequest, SmtEncodingLimits,
    SmtEncodingRefusal, SmtQueryKind,
};
use quire_exact::{LimitKind, Meter};
use quire_semantic_value::model_transition::ModelTransition;

use crate::script_search::{assert_unsat, assertions, operands, satisfiable};
use crate::vault_world::{
    non_interference, with_body, Build, World, EQUAL_INPUTS, FORALL_EXISTS, LIVENESS, MONITORED,
    SAVES_POWER, SECURE, UNLIMITED,
};

/// FR-176-AC-4's conjunct, which has no value on a joint step whose `a` step
/// is `step(0)`.
const DIVIDED: &str = "match { a.step.op = b.step.op and \
     a.step.Vault::step.i = b.step.Vault::step.i and \
     (if present(a.step.Vault::step.i) then 1 mod value(a.step.Vault::step.i) else 0) = 0 }";

/// A third conjunct that calls a declared function, which a budget that
/// covers the first two conjuncts of `(step(0), step(0))` does not cover.
const CALLING: &str = "match { a.step.op = b.step.op and \
     a.step.Vault::step.i = b.step.Vault::step.i and Noop() }";

/// The vault, `matched` the `match` of its `NonInterference`.
fn secure(matched: &str) -> World {
    World::build(&["v"], false, &non_interference(matched, ""), &SECURE)
}

fn instance(key: &str) -> Vec<(String, String)> {
    vec![("v".to_owned(), key.to_owned())]
}

/// `encode_hyper_query` over `world`'s clause and subject.
fn query(
    world: &World,
    key: &str,
    kind: SmtQueryKind,
    meter: &mut Meter,
    encoding: SmtEncodingLimits,
) -> Result<HyperQuery, SmtEncodingRefusal> {
    let subjects = [world.subject()];
    let instance = instance(key);
    encode_hyper_query(
        HyperQueryRequest {
            clause: world.clause(),
            subjects: &subjects,
            instance: &instance,
            meter,
            encoding,
        },
        kind,
    )
}

fn script(world: &World, kind: SmtQueryKind) -> String {
    let mut meter = Meter::new(UNLIMITED);
    let query = query(world, "v", kind, &mut meter, SmtEncodingLimits::default())
        .expect("the request encodes");
    String::from_utf8(query.script).expect("a script is UTF-8")
}

/// The golden script `name`, recorded from the encoder of this requirement.
fn golden(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/golden/hyper_monitor/{name}.smt2",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|_| panic!("the golden file {path}"))
}

/// FR-315-AC-43: a body that is not `always` over a state predicate is read
/// through a monitor, and its scripts are byte-equal to the golden scripts
/// recorded from the encoder.
#[trace("TC-1264", "FR-315-AC-43")]
#[test]
fn a_monitored_body_gives_the_recorded_golden_scripts() {
    let world = World::build(&["v"], false, &with_body("", "", MONITORED), &SECURE);
    for (name, kind) in [
        ("secure-base-1", SmtQueryKind::InductionBase { depth: 1 }),
        ("secure-step-1", SmtQueryKind::InductionStep { depth: 1 }),
    ] {
        let first = script(&world, kind);
        assert_eq!(first, script(&world, kind), "{name}: byte-equal twice");
        assert_eq!(first.as_bytes(), golden(name).as_slice(), "{name}");
    }
}

// ---------------------------------------------------------------------------
// The script of FR-315-AC-7 and its relatives.
// ---------------------------------------------------------------------------

/// A component's step disjunct between copies of trace variable `t`: `step(i)`
/// sets `l` to `i`, and the stutter keeps both fields.
fn component(t: &str, step: Option<u8>) -> String {
    match step {
        Some(i) => format!("(and (= s1.{t}.v.l {i}) (= s1.{t}.v.h s0.{t}.v.h))"),
        None => format!("(and (= s1.{t}.v.h s0.{t}.v.h) (= s1.{t}.v.l s0.{t}.v.l))"),
    }
}

fn joint(first: Option<u8>, second: Option<u8>) -> String {
    format!("(and {} {})", component("a", first), component("b", second))
}

/// The step assertion's disjuncts: the one assertion that is a disjunction of
/// joint disjuncts.
fn joint_steps(script: &str) -> Vec<String> {
    let steps: Vec<_> = assertions(script)
        .into_iter()
        .filter(|assertion| assertion.starts_with("(or (and (and (= s1.a.v.l"))
        .collect();
    assert_eq!(steps.len(), 1, "one step assertion: {script}");
    operands(&steps[0])
}

/// FR-315-AC-7: the declarations, the two initial-state predicates and the
/// three joint disjuncts `μ_U` keeps; without the `match`, all nine.
#[trace("TC-1120", "FR-315-AC-7")]
#[test]
fn the_base_query_declares_the_copies_and_keeps_the_matched_joint_steps() {
    let world = secure(EQUAL_INPUTS);
    let kind = SmtQueryKind::InductionBase { depth: 1 };
    let text = script(&world, kind);
    assert_eq!(text, script(&world, kind), "byte-equal twice");
    let declared: Vec<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix("(declare-const "))
        .collect();
    let expected = [
        "s0.a.v.h", "s0.a.v.l", "s0.b.v.h", "s0.b.v.l", "s1.a.v.h", "s1.a.v.l", "s1.b.v.h",
        "s1.b.v.l",
    ];
    assert_eq!(
        declared,
        expected.map(|name| format!("{name} Int)")),
        "{text}"
    );
    let initial = |t: &str| {
        format!(
            "(or (and (= s0.{t}.v.h 0) (= s0.{t}.v.l 0)) (and (= s0.{t}.v.h 1) (= s0.{t}.v.l 0)))"
        )
    };
    let asserted = assertions(&text);
    let first = asserted
        .iter()
        .position(|assertion| *assertion == initial("a"))
        .expect("a's initial-state predicate");
    assert_eq!(asserted[first + 1], initial("b"), "then b's");
    assert_eq!(
        joint_steps(&text),
        [
            joint(Some(0), Some(0)),
            joint(Some(1), Some(1)),
            joint(None, None)
        ]
    );
    assert!(!text.contains("(*") && !text.contains("mod"), "linear");
    assert!(!text.contains("m0"), "no monitor");

    let unmatched = script(&secure(""), kind);
    let all = joint_steps(&unmatched);
    assert_eq!(all.len(), 9);
    assert_eq!(all[0], joint(Some(0), Some(0)));
    assert_eq!(all[8], joint(None, None));
}

/// FR-315-AC-8: the step query asserts no initial-state predicate, the
/// property at position 0, pairwise distinct joint states and the violation
/// at the last position; a fairness set changes nothing.
#[trace("TC-1120", "FR-315-AC-8")]
#[test]
fn the_step_query_asserts_the_property_distinctness_and_the_violation() {
    let world = secure(EQUAL_INPUTS);
    let one = script(&world, SmtQueryKind::InductionStep { depth: 1 });
    let asserted = assertions(&one);
    assert!(
        !asserted
            .iter()
            .any(|a| a.starts_with("(or (and (= s0.a.v.h")),
        "no initial-state predicate: {one}"
    );
    let distinct = "(not (and (= s1.a.v.h s0.a.v.h) (= s1.a.v.l s0.a.v.l) \
         (= s1.b.v.h s0.b.v.h) (= s1.b.v.l s0.b.v.l)))";
    let at = asserted
        .iter()
        .position(|a| a == distinct)
        .expect("positions 0 and 1 are distinct");
    assert_eq!(asserted[at + 1], "(not (not (= s0.a.v.l s0.b.v.l)))");
    assert_eq!(asserted[at + 2], "(not (= s1.a.v.l s1.b.v.l))");

    let two = script(&world, SmtQueryKind::InductionStep { depth: 2 });
    let pairs = assertions(&two)
        .iter()
        .filter(|a| a.starts_with("(not (and (= s"))
        .count();
    assert_eq!(pairs, 3, "positions 0, 1 and 2 are pairwise distinct");

    let fair = World::build(
        &["v"],
        false,
        &non_interference(EQUAL_INPUTS, "fair weak V::Vault::step;"),
        &SECURE,
    );
    for kind in [
        SmtQueryKind::InductionBase { depth: 1 },
        SmtQueryKind::InductionStep { depth: 1 },
    ] {
        assert_eq!(script(&world, kind), script(&fair, kind), "{kind:?}");
    }
}

/// FR-315-AC-9: a tuple on which `μ_U` is undefined stays a joint disjunct,
/// and holding between copies 0 and 1 it is a definedness failure at
/// position 1, so the negated property holds.
#[trace("TC-1120", "FR-315-AC-9")]
#[test]
fn an_undefined_match_is_kept_and_is_a_definedness_failure() {
    let world = secure(DIVIDED);
    let text = script(&world, SmtQueryKind::InductionBase { depth: 1 });
    let steps = joint_steps(&text);
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0], joint(Some(0), Some(0)));
    let negated = assertions(&text).pop().expect("the negated property");
    assert!(
        operands(&negated).contains(&steps[0]),
        "the undefined tuple is a disjunct of the negated property: {negated}"
    );
    assert!(!operands(&negated).contains(&steps[1]), "{negated}");
}

/// FR-315-AC-11: over the secure vault the base and step queries are
/// unsatisfiable; over a vault whose post clause is `self.l = self.h` the base
/// query is satisfiable.
#[trace("TC-1120", "FR-315-AC-11")]
#[test]
fn the_secure_vault_is_inductive_and_the_leaky_vault_has_a_matched_bad_prefix() {
    let world = secure(EQUAL_INPUTS);
    for kind in [
        SmtQueryKind::InductionBase { depth: 1 },
        SmtQueryKind::InductionStep { depth: 1 },
    ] {
        assert_unsat(&script(&world, kind));
    }
    let leaky = World::build(
        &["v"],
        false,
        &non_interference(EQUAL_INPUTS, ""),
        &Build {
            post: "self.l = self.h",
            ..SECURE
        },
    );
    assert!(satisfiable(&script(
        &leaky,
        SmtQueryKind::InductionBase { depth: 1 }
    )));
}

/// FR-487-AC-7: the step query asserts the property at position 0 as
/// `(= s0.a.v.l s0.b.v.l)` and its violation at position 1 over `s1.a.v.l`
/// and `s1.b.v.l`.
#[trace("TC-1264", "FR-487-AC-7")]
#[test]
fn the_atom_is_read_from_its_checked_condition_in_each_component() {
    let world = secure(EQUAL_INPUTS);
    let text = script(&world, SmtQueryKind::InductionStep { depth: 1 });
    let asserted = assertions(&text);
    assert!(
        asserted.contains(&"(not (not (= s0.a.v.l s0.b.v.l)))".to_owned()),
        "{text}"
    );
    assert!(
        asserted.contains(&"(not (= s1.a.v.l s1.b.v.l))".to_owned()),
        "{text}"
    );
    let base = script(&world, SmtQueryKind::InductionBase { depth: 1 });
    assert_eq!(
        joint_steps(&base),
        [
            joint(Some(0), Some(0)),
            joint(Some(1), Some(1)),
            joint(None, None)
        ]
    );
}

// ---------------------------------------------------------------------------
// Refusals.
// ---------------------------------------------------------------------------

fn refusal_of(world: &World, kind: SmtQueryKind, meter: &mut Meter) -> SmtEncodingRefusal {
    query(world, "v", kind, meter, SmtEncodingLimits::default()).expect_err("the request refuses")
}

fn conjunct_refusal(refusal: &SmtEncodingRefusal) -> (usize, String, Vec<String>) {
    match refusal {
        SmtEncodingRefusal::Refused(Refusal::MatchConjunct {
            conjunct,
            site,
            tuple,
            ..
        }) => (*conjunct, site.clone(), tuple.clone()),
        other => panic!("expected match-conjunct, got {other:?}"),
    }
}

/// The work units that cover the first two conjuncts of `(step(0), step(0))`
/// and no more: what the plain `match` consumes there.
fn two_conjuncts_budget() -> Meter {
    let world = secure(EQUAL_INPUTS);
    let (zero, _) = steps(&world);
    let mut meter = Meter::new(UNLIMITED);
    CheckedStepMatch::new(world.package(), world.clause())
        .universal(
            &[binding(&world)],
            &[StepLabel::Operation(&zero), StepLabel::Operation(&zero)],
            &mut meter,
        )
        .expect("complete");
    let mut limits = UNLIMITED;
    limits.work_units = meter.consumed(LimitKind::WorkUnits);
    Meter::new(limits)
}

fn binding(world: &World) -> qsl_eval::hyper::ParameterBinding {
    let system = ModelSystem::new(world.subject().simulation).expect("admits");
    let parameter = &world.clause().parameters()[0];
    let instance = system
        .instances(&quire_exact::ValueType::Reference(
            parameter.population.object(),
        ))
        .expect("lists")
        .into_iter()
        .next()
        .expect("the universe holds one vault");
    qsl_eval::hyper::ParameterBinding {
        parameter: parameter.name.clone(),
        instance,
    }
}

/// The `step(0)` and `step(1)` transitions out of the first initial state.
fn steps(world: &World) -> (ModelTransition, ModelTransition) {
    let system = ModelSystem::new(world.subject().simulation).expect("admits");
    let state = system.initial()[0].clone();
    let mut found = [None, None];
    for (transition, _) in system.successors(&state).expect("expands").successors {
        let [argument] = transition.arguments() else {
            continue;
        };
        if argument.contains("\"value\":\"0\"") {
            found[0] = Some(transition);
        } else if argument.contains("\"value\":\"1\"") {
            found[1] = Some(transition);
        }
    }
    let [Some(zero), Some(one)] = found else {
        panic!("step(0) and step(1) are enabled");
    };
    (zero, one)
}

/// FR-315-AC-10: each request is refused with the code and locus given, and
/// an equal request with an equal budget gives the same refusal.
#[trace("TC-1120", "FR-315-AC-10")]
#[test]
fn each_refusal_names_its_code_and_construct() {
    let mut meter = Meter::new(UNLIMITED);
    let base = SmtQueryKind::InductionBase { depth: 1 };

    let hp3 = World::new(&["v"], false, FORALL_EXISTS);
    assert_eq!(
        refusal_of(&hp3, base, &mut meter).code(),
        Some("hyper-form")
    );

    let liveness = World::new(&["v"], false, LIVENESS);
    assert_eq!(
        refusal_of(&liveness, base, &mut meter).code(),
        Some("hyper-body")
    );

    let world = secure(EQUAL_INPUTS);
    assert_eq!(
        refusal_of(&world, SmtQueryKind::Unrolling { depth: 1 }, &mut meter).code(),
        Some("query-kind")
    );

    let saves = World::new(&["v"], true, SAVES_POWER);
    let subjects = [saves.subject()];
    let both = vec![
        ("d".to_owned(), "v".to_owned()),
        ("e".to_owned(), "v".to_owned()),
    ];
    let refusal = encode_hyper_query(
        HyperQueryRequest {
            clause: saves.clause(),
            subjects: &subjects,
            instance: &both,
            meter: &mut meter,
            encoding: SmtEncodingLimits::default(),
        },
        base,
    )
    .expect_err("one subject for two aliases");
    assert_eq!(refusal.code(), Some("subject-binding"));
    assert!(refusal.to_string().contains("`N`"), "{refusal}");

    let calling = secure(CALLING);
    let first = refusal_of(&calling, base, &mut two_conjuncts_budget());
    let (conjunct, site, tuple) = conjunct_refusal(&first);
    assert_eq!(conjunct, 2);
    assert!(site.ends_with(" match 2"), "{site}");
    assert!(site.starts_with("hyper "), "{site}");
    assert_eq!(tuple, ["step(0)", "step(0)"]);
    assert_eq!(
        first,
        refusal_of(&calling, base, &mut two_conjuncts_budget())
    );
}

/// FR-315-AC-47: a conjunct the kernel refuses names `match-conjunct` at
/// that conjunct, its site and the first tuple on which the conjuncts before
/// it hold; so does the exhausting conjunct, and `HyperProductIdentity::of`
/// returns its identity and charges nothing.
#[trace("TC-1120", "FR-315-AC-47")]
#[test]
fn a_refused_conjunct_names_the_first_tuple_on_which_the_earlier_ones_hold() {
    let calling = secure(CALLING);
    for kind in [
        SmtQueryKind::InductionBase { depth: 1 },
        SmtQueryKind::InductionStep { depth: 1 },
    ] {
        let refusal = refusal_of(&calling, kind, &mut two_conjuncts_budget());
        let (conjunct, site, tuple) = conjunct_refusal(&refusal);
        assert_eq!(conjunct, 2);
        assert!(site.ends_with(" match 2"), "{site}");
        assert_eq!(tuple, ["step(0)", "step(0)"]);
        assert_eq!(
            refusal,
            refusal_of(&calling, kind, &mut two_conjuncts_budget()),
            "equal budgets, equal refusals"
        );
    }
    let subjects = [calling.subject()];
    let instance = instance("v");
    let mut meter = two_conjuncts_budget();
    HyperProductIdentity::of(&HyperQueryRequest {
        clause: calling.clause(),
        subjects: &subjects,
        instance: &instance,
        meter: &mut meter,
        encoding: SmtEncodingLimits::default(),
    })
    .expect("identified");
    assert_eq!(meter.admission_count(), 0);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);
}

/// FR-487-AC-8: a conjunct placed before the calling one makes the refusal
/// name the calling conjunct and the tuple `(step(1), step(1))`.
#[trace("TC-1264", "FR-487-AC-8")]
#[test]
fn a_conjunct_before_the_calling_one_moves_the_refused_tuple() {
    let guarded = "match { a.step.op = b.step.op and \
         a.step.Vault::step.i = b.step.Vault::step.i and \
         (if present(a.step.Vault::step.i) then value(a.step.Vault::step.i) = 1 else true) \
         and Noop() }";
    let world = secure(guarded);
    let base = SmtQueryKind::InductionBase { depth: 1 };
    // A budget that covers the three leading conjuncts on the tuples up to
    // and including `(step(1), step(1))`, and no more.
    let leading = secure(
        "match { a.step.op = b.step.op and \
         a.step.Vault::step.i = b.step.Vault::step.i and \
         (if present(a.step.Vault::step.i) then value(a.step.Vault::step.i) = 1 else true) }",
    );
    let (zero, one) = steps(&leading);
    let (zero, one) = (StepLabel::Operation(&zero), StepLabel::Operation(&one));
    let mut measured = Meter::new(UNLIMITED);
    for labels in [
        [zero, zero],
        [zero, one],
        [zero, StepLabel::Stutter],
        [one, zero],
        [one, one],
    ] {
        CheckedStepMatch::new(leading.package(), leading.clause())
            .universal(&[binding(&leading)], &labels, &mut measured)
            .expect("complete");
    }
    let mut limits = UNLIMITED;
    limits.work_units = measured.consumed(LimitKind::WorkUnits);
    let mut meter = Meter::new(limits);
    let refusal = refusal_of(&world, base, &mut meter);
    let (conjunct, _, tuple) = conjunct_refusal(&refusal);
    assert_eq!(conjunct, 3);
    assert_eq!(tuple, ["step(1)", "step(1)"]);
}

/// FR-315-AC-48: a body whose divisors are not literals is refused
/// `out-of-fragment` at the atom's site naming the component copy, and a
/// `match` that exhausts the meter is refused first; the scripts of
/// FR-315-AC-7, AC-9 and AC-43 are in the fragment the checker reads.
#[trace("TC-1120", "FR-315-AC-48")]
#[test]
fn a_divisor_outside_the_fragment_is_refused_after_the_match_conjuncts() {
    let body = "always holds(1 mod (1 - v.h @ a) = 1 mod (1 - v.h @ b))";
    let world = World::build(&["v"], false, &with_body(EQUAL_INPUTS, "", body), &SECURE);
    for kind in [
        SmtQueryKind::InductionBase { depth: 1 },
        SmtQueryKind::InductionStep { depth: 1 },
    ] {
        let refusal = refusal_of(&world, kind, &mut Meter::new(UNLIMITED));
        assert_eq!(refusal.code(), Some("out-of-fragment"), "{refusal}");
        let text = refusal.to_string();
        assert!(text.contains("`hyper 0 holds "), "{text}");
        assert!(
            text.contains("component `a`") || text.contains("component `b`"),
            "{text}"
        );
        assert_eq!(
            refusal,
            refusal_of(&world, kind, &mut Meter::new(UNLIMITED))
        );
    }
    let exhausting = World::build(&["v"], false, &with_body(CALLING, "", body), &SECURE);
    let refusal = refusal_of(
        &exhausting,
        SmtQueryKind::InductionBase { depth: 1 },
        &mut two_conjuncts_budget(),
    );
    assert_eq!(refusal.code(), Some("match-conjunct"));

    for text in [
        script(
            &secure(EQUAL_INPUTS),
            SmtQueryKind::InductionBase { depth: 1 },
        ),
        script(&secure(DIVIDED), SmtQueryKind::InductionBase { depth: 1 }),
        script(
            &World::build(&["v"], false, &with_body("", "", MONITORED), &SECURE),
            SmtQueryKind::InductionBase { depth: 1 },
        ),
    ] {
        satisfiable(&text);
    }
}

/// FR-315-AC-46: the meter ends with the charges of the `μ_U` evaluations of
/// the nine tuples in order, whatever the kind, depth or instance, and no
/// other charge.
#[trace("TC-1120", "FR-315-AC-46")]
#[test]
fn the_meter_is_charged_for_the_nine_tuples_and_nothing_else() {
    for key in ["v", "w"] {
        let world = World::build(&[key], false, &non_interference(EQUAL_INPUTS, ""), &SECURE);
        let (zero, one) = steps(&world);
        let labels = [
            StepLabel::Operation(&zero),
            StepLabel::Operation(&one),
            StepLabel::Stutter,
        ];
        let matcher = CheckedStepMatch::new(world.package(), world.clause());
        let mut expected = Meter::new(UNLIMITED);
        let bound = [binding_for(&world, key)];
        for first in labels {
            for second in labels {
                matcher
                    .universal(&bound, &[first, second], &mut expected)
                    .expect("complete");
            }
        }
        assert!(expected.consumed(LimitKind::WorkUnits) > 0);
        for kind in [
            SmtQueryKind::InductionBase { depth: 1 },
            SmtQueryKind::InductionBase { depth: 3 },
            SmtQueryKind::InductionStep { depth: 1 },
        ] {
            let mut meter = Meter::new(UNLIMITED);
            query(&world, key, kind, &mut meter, SmtEncodingLimits::default()).expect("encodes");
            assert_eq!(
                meter.consumed(LimitKind::WorkUnits),
                expected.consumed(LimitKind::WorkUnits),
                "{key} {kind:?}"
            );
            assert_eq!(meter.admission_count(), expected.admission_count());
        }
    }
}

fn binding_for(world: &World, key: &str) -> qsl_eval::hyper::ParameterBinding {
    let bound = binding(world);
    assert_eq!(bound.instance.key, key);
    bound
}

/// FR-315-AC-43: `max_monitor_states` 1 refuses `too-large` naming
/// `monitor-states`.
#[trace("TC-1264", "FR-315-AC-43")]
#[test]
fn a_monitor_over_the_state_limit_is_too_large() {
    let world = World::build(&["v"], false, &with_body("", "", MONITORED), &SECURE);
    let encoding = SmtEncodingLimits {
        max_monitor_states: 1,
        ..SmtEncodingLimits::default()
    };
    let refusal = query(
        &world,
        "v",
        SmtQueryKind::InductionBase { depth: 1 },
        &mut Meter::new(UNLIMITED),
        encoding,
    )
    .expect_err("too many monitor states");
    assert!(
        matches!(
            refusal,
            SmtEncodingRefusal::Refused(Refusal::TooLarge {
                measure: Measure::MonitorStates,
                ..
            })
        ),
        "{refusal:?}"
    );
}

/// FR-315-AC-50: the table of the hyper body is the constructor's table of
/// the same formula, with the numbering of the TP-3 item of FR-315-AC-45.
#[trace("TC-1264", "FR-315-AC-50")]
#[test]
fn the_hyper_body_table_is_the_constructors_table() {
    let world = World::build(&["v"], false, &with_body("", "", MONITORED), &SECURE);
    let limits = SmtEncodingLimits::default();
    let table = MonitorTable::over(
        world.clause().body(),
        world.clause().profile(),
        limits.max_monitor_atoms,
        limits.max_monitor_states,
    )
    .expect("tabulates");
    assert_eq!(table.atoms().len(), 2);
    assert_eq!((table.states(), table.initial()), (3, 0));
    assert!(table.rejecting().is_some());
    let moves: Vec<(usize, u64, usize)> = table
        .moves()
        .iter()
        .map(|step| (step.from, step.letter, step.to))
        .collect();
    assert_eq!(moves, IMPLIES_ALWAYS_MOVES);
}

/// The numbered moves of `always (A implies always B)` over two atoms.
const IMPLIES_ALWAYS_MOVES: [(usize, u64, usize); 12] = [
    (0, 0, 0),
    (0, 1, 1),
    (0, 2, 0),
    (0, 3, 2),
    (1, 0, 1),
    (1, 1, 1),
    (1, 2, 1),
    (1, 3, 1),
    (2, 0, 1),
    (2, 1, 1),
    (2, 2, 2),
    (2, 3, 2),
];

// ---------------------------------------------------------------------------
// TC-1190: the encoder half of the identity and its four first refusals.
// ---------------------------------------------------------------------------

/// `of` and `encode_hyper_query` over one request, each with a fresh meter.
fn both(
    clause: &qsl_semantics::check::CheckedHyperClause,
    subjects: &[qsl_eval::model_subject::ModelSubject<'_>],
    instance: &[(String, String)],
) -> (
    Result<HyperProductIdentity, SmtEncodingRefusal>,
    Result<HyperQuery, SmtEncodingRefusal>,
    Meter,
) {
    let mut meter = Meter::new(UNLIMITED);
    let identity = HyperProductIdentity::of(&HyperQueryRequest {
        clause,
        subjects,
        instance,
        meter: &mut meter,
        encoding: SmtEncodingLimits::default(),
    });
    let query = encode_hyper_query(
        HyperQueryRequest {
            clause,
            subjects,
            instance,
            meter: &mut meter,
            encoding: SmtEncodingLimits::default(),
        },
        SmtQueryKind::InductionBase { depth: 1 },
    );
    (identity, query, meter)
}

/// FR-315-AC-12: the base and step queries of one request carry equal
/// identities, each equal to `HyperProductIdentity::of`.
#[trace("TC-1190", "FR-315-AC-12")]
#[test]
fn every_query_of_a_request_carries_the_identity_of_the_request() {
    let world = World::build(
        &["v", "w"],
        false,
        &non_interference(EQUAL_INPUTS, ""),
        &SECURE,
    );
    let subjects = [world.subject()];
    let identity = HyperProductIdentity::of(&HyperQueryRequest {
        clause: world.clause(),
        subjects: &subjects,
        instance: &instance("v"),
        meter: &mut Meter::new(UNLIMITED),
        encoding: SmtEncodingLimits::default(),
    })
    .expect("identified");
    for kind in [
        SmtQueryKind::InductionBase { depth: 1 },
        SmtQueryKind::InductionStep { depth: 1 },
        SmtQueryKind::InductionStep { depth: 2 },
    ] {
        let query = query(
            &world,
            "v",
            kind,
            &mut Meter::new(UNLIMITED),
            SmtEncodingLimits::default(),
        )
        .expect("encodes");
        assert_eq!(query.product, identity, "{kind:?}");
        assert_eq!(query.kind, kind);
    }
}

/// FR-315-AC-13: the requests `of` refuses are refused with the same
/// refusal by `encode_hyper_query`, before any other check.
#[trace("TC-1190", "FR-315-AC-13")]
#[test]
fn the_encoder_refuses_what_the_identity_refuses_in_the_same_words() {
    let plain = World::new(&["v"], false, &non_interference(EQUAL_INPUTS, ""));
    let subjects = [plain.subject()];
    for (bad, code) in [
        (Vec::new(), "instance-binding"),
        (instance("x"), "instance-binding"),
    ] {
        let (identity, query, _) = both(plain.clause(), &subjects, &bad);
        let refusal = identity.expect_err("refused");
        assert_eq!(refusal.code(), Some(code));
        assert_eq!(query.expect_err("refused"), refusal);
    }
    let hp3 = World::new(&["v"], false, FORALL_EXISTS);
    let (identity, query, _) = both(hp3.clause(), &[hp3.subject()], &[]);
    let refusal = identity.expect_err("refused");
    assert_eq!(refusal.code(), Some("hyper-form"));
    assert_eq!(query.expect_err("refused"), refusal);

    let liveness = World::new(&["v"], false, LIVENESS);
    let (identity, query, _) = both(liveness.clause(), &[liveness.subject()], &[]);
    let refusal = identity.expect_err("refused");
    assert_eq!(refusal.code(), Some("instance-binding"));
    assert_eq!(query.expect_err("refused"), refusal);

    let saves = World::new(&["v"], true, SAVES_POWER);
    let pairs = vec![
        ("d".to_owned(), "v".to_owned()),
        ("e".to_owned(), "v".to_owned()),
    ];
    let elsewhere = World::new(&["v"], true, SAVES_POWER);
    for subjects in [
        vec![saves.subject()],
        vec![saves.subject(), elsewhere.subject()],
    ] {
        let (identity, query, _) = both(saves.clause(), &subjects, &pairs);
        let refusal = identity.expect_err("refused");
        assert_eq!(refusal.code(), Some("subject-binding"));
        assert_eq!(query.expect_err("refused"), refusal);
    }
}

/// FR-315-AC-49: the clause as S3 checked it names `product-identity` in
/// both calls and charges nothing; the assembled clause is identified.
#[trace("TC-1190", "FR-315-AC-49")]
#[test]
fn an_unassembled_clause_is_refused_by_the_encoder_too() {
    let world = World::new(&["v"], false, &non_interference(EQUAL_INPUTS, ""));
    let (identity, query, meter) = both(&world.unassembled, &[world.subject()], &instance("v"));
    let refusal = identity.expect_err("refused");
    assert_eq!(refusal.code(), Some("product-identity"));
    assert_eq!(query.expect_err("refused"), refusal);
    assert_eq!(meter.admission_count(), 0);
    let (identity, query, _) = both(world.clause(), &[world.subject()], &instance("v"));
    assert_eq!(
        query.expect("encodes").product,
        identity.expect("identified")
    );
}

/// FR-315-AC-51: a subject FR-120 does not admit is identified, and then
/// refused `subject-binding` at its alias by the encoder, after the four
/// identity checks and before `hyper-body`.
#[trace("TC-1190", "FR-315-AC-51")]
#[test]
fn a_subject_that_fr_120_does_not_admit_is_refused_at_its_alias() {
    let world = secure(EQUAL_INPUTS);
    let (identity, query, _) = both(
        world.clause(),
        &[world.unadmitted_subject()],
        &instance("v"),
    );
    identity.expect("the identity is returned");
    let refusal = query.expect_err("the subject is not admitted");
    assert_eq!(refusal.code(), Some("subject-binding"));
    assert!(refusal.to_string().contains("`V`"), "{refusal}");

    // Before `hyper-body`: a liveness body over the same subject.
    let liveness = World::build(&["v"], false, LIVENESS, &SECURE);
    let (identity, query, _) = both(
        liveness.clause(),
        &[liveness.unadmitted_subject()],
        &instance("v"),
    );
    identity.expect("identified");
    assert_eq!(query.expect_err("refused").code(), Some("subject-binding"));
}

/// FR-315-AC-52: a parameter that names an alias the clause does not have is
/// a fault in both calls, not `instance-binding`.
#[trace("TC-1190", "FR-315-AC-52")]
#[test]
fn an_unknown_alias_is_the_same_fault_in_both_calls() {
    let world = secure(EQUAL_INPUTS);
    let altered = world.clause().clone().with_parameter_alias_after_s3("Z");
    let (identity, query, _) = both(&altered, &[world.subject()], &instance("v"));
    let fault = identity.expect_err("a fault");
    assert!(matches!(fault, SmtEncodingRefusal::Fault(_)), "{fault:?}");
    assert_eq!(query.expect_err("a fault"), fault);
}
