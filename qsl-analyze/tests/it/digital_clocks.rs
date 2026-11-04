// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-204 (TC-639, TC-642): EN-5's digital-clock product over ADR-028
//! §15.5's `Retx`, decided by the finite-horizon solver.

use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::{
    build_product, decide_bounded_event, settle_product_refusal, ExactProbLimits, FhResult,
    ProbBuild, ProbProduct, ProductAction, ProductKind, ProductRefusal, ProductRequest,
};
use qsl_analyze::exact_probabilistic::{DeadlinePhase, MonitorState};
use qsl_eval::model_subject::ModelSubject;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::statistical::SubjectLimits;
use qsl_replay::{OutcomeItem, TerminalValue, UnavailabilityCause};
use qsl_semantics::check::Bound;
use quire_exact::{Cancel, Integer, Rational};
use serde_json::json;

use crate::timed_world::{self, World};

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

/// `Retx`: `late` is `delivered`, `send(lost)` needs `x >= guard` and resets
/// `x`, the invariant `x <= 2 ms` holds until delivery.
fn retx(guard: &str, reset: bool, declarations: &str, claims: &str) -> World {
    retx_with(
        &[("send", &["late", "x"])],
        guard,
        reset,
        declarations,
        claims,
    )
}

/// The text of [`retx`]'s unit: the claim and declaration lines follow it.
fn retx_body(guard: &str, reset: bool, declarations: &str, claims: &str) -> String {
    let reset = if reset { " and self.x = 0 ms" } else { "" };
    format!(
        "{}\
         pre Send using v on M::Call::send {{ not self.late{guard} }}\n\
         post Sent using v on M::Call::send {{ self.late = not lost{reset} }}\n\
         random M::Call::send(lost) ~ {{ true: 1, false: 9 }};\n\
         time invariant Deadline when {{ not self.late }} {{ self.x <= 2 ms }}\n\
         {declarations}{claims}",
        timed_world::DENSE,
    )
}

/// [`retx`] over the operations `operations`, `send` first.
fn retx_with(
    operations: &[(&str, &[&str])],
    guard: &str,
    reset: bool,
    declarations: &str,
    claims: &str,
) -> World {
    let document = retx_document(operations);
    let body = retx_body(guard, reset, declarations, claims);
    timed_world::world(&document, PROFILES, &body)
}

fn retx_document(operations: &[(&str, &[&str])]) -> Vec<u8> {
    let mut document: serde_json::Value =
        serde_json::from_slice(&timed_world::document(&["x"], operations))
            .expect("the timed fixture is JSON");
    let operation = &mut document["types"][1]["operations"][0];
    operation["params"] = json!([{
        "identity": timed_world::ident("Call/send/lost"),
        "name": "lost",
        "typeRef": "ix://quire/native/Boolean",
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"source": {"sourceIdentity": timed_world::ident("spec"),
            "path": "spec.qspec", "startLine": 1, "startColumn": 1}},
    }]);
    serde_json::to_vec(&document).expect("serializes")
}

const PROFILES: &str = "profile ev = \"quire.temporal.timed/v1\";\n\
     profile inf = \"quire.temporal.infinite-trace/v1\";\n";

const GUARD: &str = " and self.x >= 1 ms";

fn deadline_claim(name: &str, scheduler: &str, bound: &str, body: &str) -> String {
    format!(
        "probabilistic {name} using ev on M over (c: M::Call) {scheduler} on origin {{ \
         probability {bound} [ {body}[0 ms, 4 ms] holds(c.late) ] }}\n"
    )
}

fn build(world: &World, claim: &str) -> Result<ProbBuild, ProductRefusal> {
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim(claim)
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
}

fn product(world: &World, claim: &str) -> ProbProduct {
    match build(world, claim) {
        Ok(ProbBuild::Built(product)) => product,
        other => panic!("{claim}: expected a product, got {other:?}"),
    }
}

fn refusal(world: &World, claim: &str) -> ProductRefusal {
    build(world, claim).expect_err("the build is refused")
}

fn initial_value(product: &ProbProduct, bound: Bound) -> Rational {
    let result = decide_bounded_event(product, &bound, &ExactProbLimits::default())
        .expect("the digital product is acyclic");
    let FhResult::Exact { values, .. } = result else {
        panic!("exact values: {result:?}");
    };
    values[product.initial[0].2.index()].clone()
}

fn every_claims() -> String {
    format!(
        "{}{}{}",
        deadline_claim("Deadline", "under every scheduler", ">= 0.99", "eventually"),
        deadline_claim("Max", "under every scheduler", "<= 0.999", "eventually"),
        deadline_claim("Always", "under every scheduler", ">= 0.99", "always"),
    )
}

fn every(guard: &str, reset: bool) -> World {
    retx(guard, reset, "", &every_claims())
}

/// FR-204-AC-1 (TC-639 step 1): over every scheduler the minimum is `99/100`
/// and the maximum `9999/10000`. This is the values half of FR-204-AC-1; the
/// proved/refuted settlement and the refutation path with its replay are not
/// asserted here.
#[trace("TC-639", "FR-204-AC-1")]
#[test]
fn deadline_extrema_over_every_scheduler() {
    let world = every(GUARD, true);
    let built = product(&world, "Deadline");
    assert_eq!(built.kind, ProductKind::Mdp);
    assert_eq!(
        initial_value(&built, Bound::AtLeast(ratio(99, 100))),
        ratio(99, 100)
    );
    assert_eq!(
        initial_value(&built, Bound::AtMost(ratio(999, 1000))),
        ratio(9999, 10000)
    );
    // The scheduler's delays are named unit delays, not an operation.
    assert!(built
        .edges
        .iter()
        .any(|edge| matches!(edge.choice, ProductAction::UnitDelay) && edge.action.is_none()));
}

/// FR-204-AC-4 (TC-639 step 4): a `send` without guard or reset is a
/// zero-delay cycle, and an `always` event has no digital reading.
#[trace("TC-639", "FR-204-AC-4")]
#[test]
fn zero_delay_cycle_and_form_shape_are_unsupported() {
    let cyclic = every("", false);
    let cycle_refusal = refusal(&cyclic, "Deadline");
    let ProductRefusal::ZeroDelayCycle { states } = &cycle_refusal else {
        panic!("a zero-delay cycle");
    };
    // Without a guard or a reset `send` loops at the initial state: x = 0,
    // elapsed 0, the deadline still pending.
    assert_eq!(states.len(), 1);
    assert_eq!(digital_clocks_of(&states[0]), (vec![0], Some(0)));
    assert!(matches!(
        states[0].monitor,
        Some(MonitorState::Deadline {
            phase: DeadlinePhase::Pending,
            ..
        })
    ));
    let cycle_ids = states
        .iter()
        .map(|state| state.digital_cycle_identity())
        .collect::<Vec<_>>();
    let simulation = cyclic.request();
    let settled = settle_product_refusal(
        cycle_refusal,
        SubjectLimits {
            expansion: simulation.expansion,
            meter: &simulation.meter,
        },
    )
    .expect("the actual digital refusal settles");
    assert_eq!(
        settled.value(),
        &TerminalValue::Unsupported(UnavailabilityCause::ZeroDelayCycle {
            states: cycle_ids.clone(),
        })
    );
    let terminal = settled.into_terminal(RequestIndex::new(0), BackendId::new("qsl-en-5"));
    let wire = serde_json::to_value(OutcomeItem::from_terminal(&terminal))
        .expect("the settled digital cycle writes to FR-331");
    assert_eq!(wire["cause"]["kind"], "zero-delay-cycle");
    assert_eq!(wire["cause"]["states"], serde_json::json!(cycle_ids));
    let identity = wire["cause"]["states"][0]
        .as_str()
        .expect("a cycle state identity is a string");
    assert!(identity.starts_with("quire.probability-digital-cycle-state/v1;"));

    let world = every(GUARD, true);
    let shape_refusal = refusal(&world, "Always");
    let ProductRefusal::TimedFormShape { span, claim } = shape_refusal else {
        panic!("an always event has no digital timed reading");
    };
    let simulation = world.request();
    let settled = settle_product_refusal(
        ProductRefusal::TimedFormShape {
            span,
            claim: claim.clone(),
        },
        SubjectLimits {
            expansion: simulation.expansion,
            meter: &simulation.meter,
        },
    )
    .expect("the unsupported form settles");
    assert_eq!(
        settled.value(),
        &TerminalValue::Unsupported(UnavailabilityCause::TimedFormShape {
            span,
            claim: claim.clone()
        })
    );
    let terminal = settled.into_terminal(RequestIndex::new(1), BackendId::new("qsl-en-5"));
    let wire = serde_json::to_value(OutcomeItem::from_terminal(&terminal))
        .expect("the form-shape refusal writes to FR-331");
    assert_eq!(wire["cause"]["kind"], "timed-form-shape");
    // An assembled claim is located by its identity, never by its span.
    assert!(claim.is_some());
    assert!(wire["cause"].get("where").is_none());
    assert_eq!(wire["cause"]["locus"]["occurrence"]["role"], "claim");
}

/// FR-498-AC-3 (TC-1288 step 3): two claims with the same name whose
/// `always` event has no digital reading each settle `timed-form-shape`
/// carrying their own `{declaration, occurrence}`, not the other's.
#[trace("FR-498-AC-3", "TC-1288")]
#[test]
fn timed_form_shape_names_the_selected_claim_among_equal_names() {
    let claims = format!(
        "{}{}",
        deadline_claim("Same", "under every scheduler", ">= 0.99", "always"),
        deadline_claim("Same", "under every scheduler", ">= 0.5", "always"),
    );
    let world = retx(GUARD, true, "", &claims);
    let checked = world.package.graph().probabilistic_model().claims();
    assert_eq!(checked.len(), 2);
    let simulation = world.request();
    let mut wires = Vec::new();
    for claim in checked {
        let refusal = build_product(
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
        .expect_err("an always event has no digital timed reading");
        let ProductRefusal::TimedFormShape { claim: located, .. } = &refusal else {
            panic!("a timed form shape refusal: {refusal:?}");
        };
        assert_eq!(located.as_ref(), claim.occurrence());
        let settled = settle_product_refusal(
            refusal,
            SubjectLimits {
                expansion: simulation.expansion,
                meter: &simulation.meter,
            },
        )
        .expect("the unsupported form settles");
        let terminal = settled.into_terminal(RequestIndex::new(0), BackendId::new("qsl-en-5"));
        let wire = serde_json::to_value(OutcomeItem::from_terminal(&terminal))
            .expect("the form-shape refusal writes to FR-331");
        assert_eq!(wire["cause"]["kind"], "timed-form-shape");
        assert!(wire["cause"].get("where").is_none(), "no span locus");
        assert_eq!(wire["cause"]["locus"]["occurrence"]["role"], "claim");
        wires.push(wire["cause"]["locus"].clone());
    }
    assert_ne!(wires[0], wires[1], "each cause names its own claim");
}

/// FR-204-AC-4: digital unbounded products have no monitor, but their actual
/// zero-delay cycles still settle with complete stable product-key identities.
#[trace("TC-639", "FR-204-AC-4")]
#[test]
fn unbounded_digital_cycles_settle_with_monitorless_state_identity() {
    let world = retx(
        "",
        false,
        "reward sends on M::Call::send = 1;\n",
        concat!(
            "probabilistic Reach using inf on M over (c: M::Call) under every scheduler on origin ",
            "{ probability >= 0.5 [ eventually holds(c.late) ] }\n",
            "probabilistic Cost using inf on M over (c: M::Call) under every scheduler on origin ",
            "{ expected accumulate sends until holds(c.late) <= 5 }\n",
        ),
    );

    for (index, name) in ["Reach", "Cost"].into_iter().enumerate() {
        let cycle_refusal = refusal(&world, name);
        let ProductRefusal::ZeroDelayCycle { states } = &cycle_refusal else {
            panic!("{name} has a digital zero-delay cycle: {cycle_refusal:?}");
        };
        assert!(!states.is_empty(), "{name}");
        assert!(states.iter().all(|state| state.monitor.is_none()), "{name}");
        let cycle_ids = states
            .iter()
            .map(|state| state.digital_cycle_identity())
            .collect::<Vec<_>>();

        let simulation = world.request();
        let settled = settle_product_refusal(
            cycle_refusal,
            SubjectLimits {
                expansion: simulation.expansion,
                meter: &simulation.meter,
            },
        )
        .expect("the actual unbounded digital refusal settles");
        assert_eq!(
            settled.value(),
            &TerminalValue::Unsupported(UnavailabilityCause::ZeroDelayCycle {
                states: cycle_ids.clone(),
            })
        );
        let terminal = settled.into_terminal(RequestIndex::new(index), BackendId::new("qsl-en-5"));
        let wire = serde_json::to_value(OutcomeItem::from_terminal(&terminal))
            .expect("the settled digital cycle writes to FR-331");
        assert_eq!(wire["cause"]["kind"], "zero-delay-cycle");
        assert_eq!(wire["cause"]["states"], serde_json::json!(cycle_ids));
        assert!(wire["cause"]["states"][0]
            .as_str()
            .expect("a cycle state identity is a string")
            .starts_with("quire.probability-digital-cycle-state/v1;"));
    }
}

/// FR-204-AC-2 (TC-639 step 2): a strict guard has no digital reading.
#[trace("TC-639", "FR-204-AC-2")]
#[test]
fn a_strict_guard_is_unsupported() {
    let world = every(" and self.x > 1 ms", true);
    let ProductRefusal::StrictClockConstraint { span } = refusal(&world, "Deadline") else {
        panic!("a strict clock constraint");
    };
    let guard = " and self.x > 1 ms";
    let unit = timed_world::unit_text(
        timed_world::PACKAGE,
        PROFILES,
        &retx_body(guard, true, "", &every_claims()),
    );
    assert_eq!(&unit[span.start..span.end], "self.x > 1 ms");
    assert_eq!(unit.matches("self.x > 1 ms").count(), 1);
}

fn one_workload(declarations: &str) -> World {
    retx(
        GUARD,
        true,
        &format!("{declarations}workload One on M {{ weight Call::send = 1; }}\n"),
        &format!(
            "{}{}",
            deadline_claim("Under", "under One", ">= 0.99", "eventually"),
            deadline_claim("UnderMax", "under One", "<= 0.999", "eventually"),
        ),
    )
}

/// FR-204-AC-3 (TC-639 step 3): a `discrete` delay under a workload is a DTMC
/// of exact value `159129/160000`; the same declaration over every scheduler
/// and an `exponential` one under the workload are unsupported.
#[trace("TC-639", "FR-204-AC-3")]
#[test]
fn discrete_delays_under_a_workload_are_a_chain() {
    let world = one_workload("delay M::Call::send ~ discrete { 1 ms: 1, 2 ms: 1 };\n");
    let built = product(&world, "Under");
    assert_eq!(built.kind, ProductKind::Dtmc);
    assert_eq!(
        initial_value(&built, Bound::AtLeast(ratio(99, 100))),
        ratio(159_129, 160_000)
    );

    let scheduler = retx(
        GUARD,
        true,
        "delay M::Call::send ~ discrete { 1 ms: 1, 2 ms: 1 };\n",
        &deadline_claim("Deadline", "under every scheduler", ">= 0.99", "eventually"),
    );
    let ProductRefusal::DelayDistribution { operation } = refusal(&scheduler, "Deadline") else {
        panic!("a delay distribution");
    };
    assert!(operation.contains("send"), "{operation}");

    let exponential = one_workload("delay M::Call::send ~ exponential(1);\n");
    assert!(matches!(
        refusal(&exponential, "Under"),
        ProductRefusal::DelayDistribution { .. }
    ));
}

/// FR-204-AC-3 (TC-639): delay admission is scoped to the selected subject
/// model, so its own distribution is refused while another model's is ignored.
#[trace("FR-204-AC-3", "TC-639")]
#[test]
fn delay_admission_is_scoped_to_the_selected_subject_model() {
    const OTHER_PACKAGE: &str = "test/other-timed";
    let subject_delay = retx(
        GUARD,
        true,
        "delay M::Call::send ~ exponential(1);\n",
        &deadline_claim(
            "SubjectDelay",
            "under every scheduler",
            ">= 0.99",
            "eventually",
        ),
    );
    let ProductRefusal::DelayDistribution { operation } = refusal(&subject_delay, "SubjectDelay")
    else {
        panic!("the subject model's delay distribution is unsupported");
    };
    assert!(operation.contains("send"), "{operation}");

    let primary = retx_document(&[("send", &["late", "x"])]);
    let other = String::from_utf8(timed_world::document(&["x"], &[("send", &["late", "x"])]))
        .expect("fixture is UTF-8")
        .replace(timed_world::PACKAGE, OTHER_PACKAGE)
        .into_bytes();
    let body = format!(
        "model Other = \"{OTHER_PACKAGE}\";\ntime Other dense unit ms;\n{}",
        retx_body(
            GUARD,
            true,
            "delay Other::Call::send ~ exponential(1);\n",
            &deadline_claim("Deadline", "under every scheduler", ">= 0.99", "eventually",),
        )
        .replace(
            "time invariant Deadline when { not self.late } { self.x <= 2 ms }\n",
            ""
        ),
    );
    let world = timed_world::world_built_with_packages(&primary, &[&other], PROFILES, &body);
    let built = product(&world, "Deadline");
    assert_eq!(built.kind, ProductKind::Mdp);
    assert!(!built.states.is_empty());
}

/// FR-204-AC-5 (TC-642): under a workload that gives no delay distribution the
/// delay stays the scheduler's: minimum `99/100`, maximum `9999/10000`.
#[trace("TC-642", "FR-204-AC-5")]
#[test]
fn free_delays_under_a_workload_are_decided_by_minimum_and_maximum() {
    let world = one_workload("");
    let built = product(&world, "Under");
    assert_eq!(built.kind, ProductKind::Mdp);
    assert_eq!(
        initial_value(&built, Bound::AtLeast(ratio(99, 100))),
        ratio(99, 100)
    );
    assert_eq!(
        initial_value(&built, Bound::AtMost(ratio(999, 1000))),
        ratio(9999, 10000)
    );
}

/// FR-243-AC-2 (TC-698): over a timed subject, a probabilistic claim that
/// names `exact` evidence routes to EN-5 (the digital-clock route) and one that
/// names `statistical` evidence to EN-4; neither is marked a timed item, which
/// would restrict it to EN-6. The route is a function of the capability kind
/// and the evidence kind alone.
#[trace("TC-698", "FR-243-AC-2")]
#[test]
fn exact_timed_claims_route_to_en5_and_statistical_ones_to_en4() {
    use qsl_foundation::source::provenance::OccurrenceKey;
    use qsl_route::request::{items_with_evidence, EvidenceSelection};
    use qsl_semantics::check::{RequirementMap, RequirementRecord};
    use quire_exact::{Origin, Role};

    let world = retx(
        GUARD,
        true,
        "workload One on M { weight Call::send = 1; }\n",
        &format!(
            "{}probabilistic Sampled using ev on M over (c: M::Call) under One on origin \
             with alpha 0.1, beta 0.1, indifference 0.005 \
             {{ probability >= 0.99 [ eventually[0 ms, 4 ms] holds(c.late) ] }}\n",
            deadline_claim("Deadline", "under every scheduler", ">= 0.99", "eventually")
        ),
    );
    let claims = world.package.graph().probabilistic_model();
    let node = |fill: u8| crate::fixture_ids::node_key([fill]);
    let record = |name: &str, fill: u8| {
        let claim = claims.claim(name).expect("the claim checks");
        RequirementRecord::model_claim(claim.requirements(node(fill)), node(99))
    };
    let exact = record("Deadline", 1);
    let sampled = record("Sampled", 2);
    assert!(!exact.requirements().timed() && !sampled.requirements().timed());
    let records: RequirementMap = [
        (
            OccurrenceKey::new(node(1), Origin::new(Role::new("claim"), 0)),
            exact,
        ),
        (
            OccurrenceKey::new(node(2), Origin::new(Role::new("claim"), 0)),
            sampled,
        ),
    ]
    .into_iter()
    .collect();
    let registry = crate::probabilistic_request_items::registry();
    let items = items_with_evidence(&records, &registry, None, EvidenceSelection::Record);
    let routed: Vec<_> = items
        .iter()
        .map(crate::probabilistic_request_items::candidate_ids)
        .collect();
    assert_eq!(routed, [vec!["en-5".to_owned()], vec!["en-4".to_owned()]]);
}

/// The second operation of the mixed races: it modifies `retried`, has no
/// guard, and is enabled until it has run.
const TICK: &str = "pre Tick using v on M::Call::tick { not self.retried }\n\
     post Ticked using v on M::Call::tick { self.retried = true }\n";

fn with_tick(declarations: &str, claims: &str) -> World {
    retx_with(
        &[("send", &["late", "x"]), ("tick", &["retried"])],
        GUARD,
        true,
        &format!(
            "{TICK}{declarations}workload Both on M {{ weight Call::send = 1; \
             weight Call::tick = 1; }}\n"
        ),
        claims,
    )
}

fn claim_over(name: &str, scheduler: &str, bound: &str, event: &str) -> String {
    format!(
        "probabilistic {name} using ev on M over (c: M::Call) {scheduler} on origin \
         {{ probability {bound} [ {event} ] }}\n"
    )
}

/// The digital clocks and the deadline clock a product key carries, scaled
/// back to model units: `(clock values, deadline clock)`.
fn digital_clocks_of(key: &qsl_analyze::exact_probabilistic::ProbKey) -> (Vec<u64>, Option<u64>) {
    let bytes = &key.model;
    let word = |at: usize| {
        let eight: [u8; 8] = bytes[at..at + 8].try_into().expect("a word");
        u64::from_be_bytes(eight)
    };
    let prefix = b"quire.simulation.state-key/v1/digital\n".len();
    let model = usize::try_from(word(prefix)).expect("a length");
    let mut at = prefix + 8 + model;
    let scale = word(at);
    let count = usize::try_from(word(at + 8)).expect("a count");
    at += 16;
    let mut clocks = Vec::new();
    for _ in 0..count {
        clocks.push(word(at) / scale);
        at += 16;
    }
    let deadline = (bytes[at] == 1).then(|| word(at + 1) / scale);
    (clocks, deadline)
}

/// FR-204-AC-3 and AC-5 (TC-639, TC-642): a given delay and a free one race as
/// the scheduler's choices. Every `WorkloadResolve` choice is a distribution
/// of mass one on its own, so the minimum and maximum stay the free delay's
/// `99/100` and `9999/10000`, and a deadline of 0 where the capped deadline
/// clock makes every choice's targets coincide still solves.
#[trace("TC-639", "FR-204-AC-3")]
#[trace("TC-642", "FR-204-AC-5")]
#[test]
fn mixed_given_and_free_races_keep_each_choice_a_distribution() {
    let world = with_tick(
        "delay M::Call::tick ~ discrete { 3 ms: 1 };\n",
        &format!(
            "{}{}{}",
            claim_over(
                "Now",
                "under Both",
                ">= 0.5",
                "eventually[0 ms, 0 ms] holds(c.late)"
            ),
            deadline_claim("Under", "under Both", ">= 0.99", "eventually"),
            deadline_claim("UnderMax", "under Both", "<= 0.999", "eventually"),
        ),
    );
    for name in ["Now", "Under"] {
        let built = product(&world, name);
        let mut mass: std::collections::BTreeMap<(usize, String), Rational> =
            std::collections::BTreeMap::new();
        for edge in &built.edges {
            if let ProductAction::WorkloadResolve { delay, .. } = &edge.choice {
                let total = mass
                    .entry((edge.from.index(), format!("{:?}", edge.choice)))
                    .or_insert_with(|| ratio(0, 1));
                *total = total.add(&edge.probability);
                assert!(*delay >= 1, "a resolve of the send waits for its guard");
            }
        }
        assert!(!mass.is_empty(), "{name}: the free delays are choices");
        for (choice, total) in &mass {
            assert_eq!(*total, ratio(1, 1), "{name}: {choice:?}");
        }
    }
    let now = product(&world, "Now");
    assert_eq!(
        initial_value(&now, Bound::AtLeast(ratio(1, 2))),
        ratio(0, 1)
    );
    let built = product(&world, "Under");
    assert_eq!(built.kind, ProductKind::Mdp);
    assert_eq!(
        initial_value(&built, Bound::AtLeast(ratio(99, 100))),
        ratio(99, 100)
    );
    assert_eq!(
        initial_value(
            &product(&world, "UnderMax"),
            Bound::AtMost(ratio(999, 1000))
        ),
        ratio(9999, 10000)
    );
}

/// FR-204-AC-3 (TC-639): identities that tie at one delay run in sequence on
/// one edge, and the edge carries the sum of their rewards. FR-204 owns the
/// race; no acceptance criterion states this behaviour.
#[trace("TC-639", "FR-204")]
#[test]
fn tied_identities_sum_their_rewards_on_one_edge() {
    let world = with_tick(
        "delay M::Call::send ~ discrete { 1 ms: 1 };\n\
         delay M::Call::tick ~ discrete { 1 ms: 1 };\n\
         reward cost on M::Call::send = 1;\n\
         reward cost on M::Call::tick = 2;\n",
        &deadline_claim("Tie", "under Both", ">= 0.5", "eventually"),
    );
    let built = product(&world, "Tie");
    let start = built.initial[0].2;
    let race: Vec<_> = built
        .edges
        .iter()
        .filter(|edge| edge.from == start)
        .collect();
    assert!(!race.is_empty());
    for edge in race {
        let cost: Vec<_> = edge
            .rewards
            .iter()
            .filter(|(name, _)| name.as_str() == "cost")
            .map(|(_, value)| value.clone())
            .collect();
        assert!(
            matches!(cost.as_slice(), [quire_exact::Value::Integer(sum)] if *sum == Integer::from(3_i64)),
            "{edge:?}"
        );
    }
}

/// FR-204 "Admission" (TC-639): a `Reach` and an `ExpectedReward` claim over a
/// timed subject label the digital product. The send is forced by the
/// invariant and delivers with probability 9/10, so delivery is certain and
/// the expected number of sends is `10/9` under every scheduler.
#[trace("TC-639", "FR-204")]
#[test]
fn reach_and_expected_reward_label_the_digital_product() {
    use qsl_analyze::exact_probabilistic::{
        decide_expected_reward, decide_reachability, ExtRational, UnboundedObjective,
    };
    let world = retx(
        GUARD,
        true,
        "reward sends on M::Call::send = 1;\n",
        &format!(
            "{}{}",
            "probabilistic Reach using inf on M over (c: M::Call) under every scheduler on origin \
             { probability >= 0.5 [ eventually holds(c.late) ] }\n",
            "probabilistic Cost using inf on M over (c: M::Call) under every scheduler on origin \
             { expected accumulate sends until holds(c.late) <= 5 }\n",
        ),
    );
    for name in ["Reach", "Cost"] {
        let built = product(&world, name);
        let target: Vec<bool> = built.labels.iter().map(|labels| labels[0]).collect();
        assert!(target.iter().any(|reached| *reached), "{name}");
        let start = built.initial[0].2.index();
        for objective in [UnboundedObjective::Min, UnboundedObjective::Max] {
            let limits = ExactProbLimits::default();
            let result = if name == "Reach" {
                decide_reachability(&built, &target, objective, &limits)
            } else {
                decide_expected_reward(&built, &target, "sends", objective, &limits)
            }
            .expect("the labelled product is solved");
            let expected = if name == "Reach" {
                ratio(1, 1)
            } else {
                ratio(10, 9)
            };
            assert_eq!(
                result.values[start],
                ExtRational::Finite(expected),
                "{name}"
            );
        }
    }
}

/// FR-204 "Admission" (TC-639): `holds(A) until[0 ms, D] holds(B)` is a deadline
/// event with the extrema of the `eventually` form.
#[trace("TC-639", "FR-204")]
#[test]
fn an_until_deadline_has_the_same_extrema() {
    let world = retx(
        GUARD,
        true,
        "",
        &format!(
            "{}{}",
            deadline_claim(
                "Until",
                "under every scheduler",
                ">= 0.99",
                "holds(not c.late) until"
            ),
            deadline_claim(
                "UntilMax",
                "under every scheduler",
                "<= 0.999",
                "holds(not c.late) until"
            ),
        ),
    );
    assert_eq!(
        initial_value(&product(&world, "Until"), Bound::AtLeast(ratio(99, 100))),
        ratio(99, 100)
    );
    assert_eq!(
        initial_value(
            &product(&world, "UntilMax"),
            Bound::AtMost(ratio(999, 1000))
        ),
        ratio(9999, 10000)
    );
}

/// FR-204 "Admission" (TC-639): a state where no identity races and delay is
/// bounded is unsupported `NotStochastic`. The `send` is enabled at `x = 1`,
/// but its only delay, 5 ms, lies outside its window.
#[trace("TC-639", "FR-204")]
#[test]
fn a_bounded_state_where_nothing_races_is_not_stochastic() {
    let world = one_workload("delay M::Call::send ~ discrete { 5 ms: 1 };\n");
    assert!(matches!(
        refusal(&world, "Under"),
        ProductRefusal::NotStochastic { .. }
    ));
}

/// FR-204-AC-1 (TC-639 step 1): the clocks span the AC's ranges, `x` from 0
/// to 2 and the deadline clock from 0 to 5 (one past `D = 4 ms`). This is the
/// clock-range clause of FR-204-AC-1 only.
#[trace("TC-639", "FR-204-AC-1")]
#[test]
fn clocks_span_the_declared_ranges() {
    let world = every(GUARD, true);
    let built = product(&world, "Deadline");
    let mut x = std::collections::BTreeSet::new();
    let mut deadline = std::collections::BTreeSet::new();
    for key in &built.states {
        let (clocks, elapsed) = digital_clocks_of(key);
        x.insert(clocks[0]);
        deadline.extend(elapsed);
    }
    assert_eq!(x.into_iter().collect::<Vec<_>>(), [0, 1, 2]);
    assert_eq!(deadline.into_iter().collect::<Vec<_>>(), [0, 1, 2, 3, 4, 5]);
}

/// FR-204 (TC-642): at the quiescent idle-delayed key at `D`, `IdleObserve`
/// reads the predicates at `D` with elapsed time `D`, and `IdleDelay` crosses
/// `D` to elapsed `D + 1`, which rejects.
/// The predicate never holds, so the success-at-`D` half of FR-204-AC-5 is not
/// asserted: this checks the distinct actions, post-keys and phases only.
#[trace("TC-642", "FR-204")]
#[test]
fn idle_observe_reads_at_the_deadline_and_idle_delay_crosses_it() {
    let world = retx(
        GUARD,
        true,
        "",
        &claim_over(
            "Never",
            "under every scheduler",
            ">= 0.5",
            "eventually[0 ms, 4 ms] holds(c.retried)",
        ),
    );
    let built = product(&world, "Never");
    let at_deadline = built.edges.iter().find(|edge| {
        matches!(edge.choice, ProductAction::IdleObserve)
            && digital_clocks_of(&built.states[edge.from.index()]).1 == Some(4)
    });
    let observe = at_deadline.expect("an observation at D");
    let source = observe.from;
    let delay = built
        .edges
        .iter()
        .find(|edge| edge.from == source && matches!(edge.choice, ProductAction::IdleDelay))
        .expect("a delay across D from the same key");
    let observed = &built.states[observe.to.index()];
    let crossed = &built.states[delay.to.index()];
    assert_eq!(digital_clocks_of(observed).1, Some(4));
    assert_eq!(digital_clocks_of(crossed).1, Some(5));
    let phase = |key: &qsl_analyze::exact_probabilistic::ProbKey| match key.monitor {
        Some(MonitorState::Deadline { phase, .. }) => phase,
        other => panic!("a deadline monitor: {other:?}"),
    };
    assert_eq!(phase(observed), DeadlinePhase::Pending);
    assert_eq!(phase(crossed), DeadlinePhase::Reject);
}
