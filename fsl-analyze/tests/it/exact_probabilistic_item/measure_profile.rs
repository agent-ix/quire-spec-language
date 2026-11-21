// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-3152: finite-horizon products, independent payoff/rank checking and limits.
use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use fsl_replay::ExactProbabilityCertificate;
use serde_json::{json, Value};

fn retry(claims: &str) -> prob_world::Model {
    prob_world::Model {
        package:"test/retry",object:"Msg",population:"msgs",key:"m",
        integers:vec![("Lost",0,1)],
        fields:vec![("delivered",prob_world::BOOLEAN.into())],
        operations:["fast","slow"].into_iter().map(|name| prob_world::Operation { name,parameters:vec![("lost","ix://test/retry/Lost".into())],modifies:vec!["delivered"],deletes:false }).collect(),
        body:format!("pre Fast using v on M::Msg::fast {{ not self.delivered }}\npost FastDone using v on M::Msg::fast {{ self.delivered = (lost = 1) }}\npre Slow using v on M::Msg::slow {{ not self.delivered }}\npost SlowDone using v on M::Msg::slow {{ self.delivered = (lost = 1) }}\nrandom Msg::fast(lost) ~ {{ 0:1,1:1 }};\nrandom Msg::slow(lost) ~ {{ 0:3,1:1 }};\nworkload Steady on M {{ weight Msg::fast=1; weight Msg::slow=0; }}\nterminal Done using v on M when {{ self.delivered }};\n{claims}"),
        initial:json!({"delivered":false}),
    }
}
fn claim(name: &str, scheduler: &str, body: &str) -> String {
    format!("probabilistic {name} using ev on M over (m:M::Msg) under {scheduler} on origin {{ {body} }}\n")
}
fn steps() -> &'static str {
    "steps from holds(true) until holds(m.delivered) within 4"
}
fn proof(world: &World, name: &str, expected: Rational) -> Value {
    let record = settle(world, name, ModelCheckLimits::default());
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("expected proof: {:?}", record.settlement));
    assert!(
        matches!(terminal.value(), TerminalValue::Proved { .. }),
        "{:?}",
        terminal.value()
    );
    assert_eq!(record.exact_probabilistic.as_ref().unwrap().value, expected);
    serde_json::from_slice(
        terminal
            .exact_certificate_document()
            .expect("a checked document"),
    )
    .unwrap()
}
fn check(
    world: &World,
    name: &str,
    document: &Value,
) -> Result<ExactProbabilityCertificateCheck, fsl_replay::ProbabilityDocumentError> {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
    fsl_replay::check_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        &serde_json::to_vec(document).unwrap(),
        fsl_replay::ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 8_000_000,
            states: 128,
            edges: 1024,
            scalar_bits: 128,
            accounting: fsl_replay::AccountingLimits::default().0,
        },
        &Cancel::new(),
    )
}
fn key(row: &Value) -> Value {
    serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(row["state"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap()
}
fn refuted(world: &World, name: &str) {
    let record = settle(world, name, ModelCheckLimits::default());
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("expected refutation: {:?}", record.settlement));
    assert_eq!(
        terminal.value(),
        &TerminalValue::Refuted,
        "{:?}",
        terminal.value()
    );
    assert!(record.probabilistic_counterexample.is_some());
}

#[trace("TC-3152", "FR-1043-AC-1")]
#[test]
fn workload_quantile_has_independently_checked_numerator_denominator_and_ranks() {
    let world = World::new(&retry(
        &(claim(
            "Q",
            "Steady",
            &format!("quantile 0.5 of ({}) <= 2", steps()),
        ) + &claim(
            "No",
            "Steady",
            &format!("quantile 0.8 of ({}) <= 2", steps()),
        )),
    ));
    let doc = proof(&world, "Q", ratio(3, 4));
    assert_eq!(doc["kind"], "conditional");
    assert_eq!(doc["conditional"]["mode"], "ratio");
    let numerator = &doc["conditional"]["numerator"];
    let denominator = &doc["conditional"]["denominator"];
    assert_eq!(numerator["objective"]["payoff"], "satisfied");
    assert_eq!(denominator["objective"]["payoff"], "activated");
    for (remaining, amount, value) in [
        (4, "0", ratio(3, 4)),
        (3, "1", ratio(1, 2)),
        (2, "2", ratio(0, 1)),
    ] {
        let row = numerator["values"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                let key = key(row);
                key[5][1] == remaining.to_string() && key[5][2] == "active" && key[6][2] == amount
            })
            .expect("an active amount row");
        assert_eq!(
            row["value"],
            json!({"numerator":value.numerator().to_string(),"denominator":value.denominator().to_string()})
        );
        assert!(denominator["values"]
            .as_array()
            .unwrap()
            .iter()
            .any(|other| other["state"] == row["state"]
                && other["value"] == json!({"numerator":"1","denominator":"1"})));
    }
    let above = numerator["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            let state = key(row);
            state[5] == json!(["measure", "1", "active"]) && state[6][2] == json!(["above-bound"])
        })
        .expect("the above-bound amount is a distinct active row");
    assert_eq!(above["value"], json!({"numerator":"0","denominator":"1"}));
    assert!(numerator["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["state"] == above["state"] && row["rank"] == 1));
    assert!(
        numerator["values"].as_array().unwrap().iter().any(|row| {
            let state = key(row);
            state[5][2] == "censored"
                && state[6][2] == json!(["censored"])
                && row["value"] == json!({"numerator":"0","denominator":"1"})
        }),
        "horizon censoring must remain distinct from saturation"
    );
    let accepted = check(&world, "Q", &doc);
    assert!(
        matches!(accepted, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "{accepted:?}"
    );
    let mut altered = doc.clone();
    let ranks = altered["conditional"]["numerator"]["ranking"]
        .as_array_mut()
        .unwrap();
    let row = ranks.iter_mut().find(|row| row["rank"] == 3).unwrap();
    row["rank"] = json!(4);
    let checked = check(&world, "Q", &altered).unwrap();
    assert!(
        matches!(
            checked,
            ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::RankBroken,
                at: ProbabilityCertificateLocus::ProductPart {
                    part: fsl_replay::ProbabilityCertificatePart::Numerator,
                    ..
                }
            })
        ),
        "{checked:?}"
    );
    refuted(&world, "No");
}

#[trace("TC-3152", "FR-1043-AC-2", "FR-1043-AC-3")]
#[test]
fn every_scheduler_checks_shifted_extrema_and_the_strict_zero_edge() {
    let claims = claim(
        "Min",
        "every scheduler",
        &format!("quantile 0.4 of ({}) <= 2", steps()),
    ) + &claim(
        "MinNo",
        "every scheduler",
        &format!("quantile 0.5 of ({}) <= 2", steps()),
    ) + &claim(
        "Max",
        "every scheduler",
        &format!("quantile 0.6 of ({}) >= 2", steps()),
    ) + &claim(
        "MaxNo",
        "every scheduler",
        &format!("quantile 0.5 of ({}) >= 2", steps()),
    );
    let world = World::new(&retry(&claims));
    let doc = proof(&world, "Min", ratio(3, 80));
    assert_eq!(doc["objective"]["payoff"], "shifted");
    assert_eq!(doc["objective"]["optimization"], "minimum");
    for (remaining, amount, expected) in [
        (3, json!("1"), ratio(-3, 20)),
        (2, json!("2"), ratio(-2, 5)),
        (1, json!(["above-bound"]), ratio(-2, 5)),
    ] {
        let row = doc["values"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                let state = key(row);
                state[5][1] == remaining.to_string()
                    && state[5][2] == "active"
                    && state[6][2] == amount
            })
            .unwrap();
        assert_eq!(
            row["value"],
            json!({"numerator":expected.numerator().to_string(),"denominator":expected.denominator().to_string()})
        );
    }
    let maximum = proof(&world, "Max", ratio(-1, 10));
    for row in maximum["values"].as_array().unwrap() {
        let state = key(row);
        if state[5][2] == "active" && state[6][2] != "0" {
            assert_eq!(row["value"], json!({"numerator":"-3","denominator":"5"}));
        }
    }
    refuted(&world, "MinNo");
    refuted(&world, "MaxNo");
    let mut altered = doc.clone();
    let initial = altered["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["rank"] == 4)
        .unwrap()["state"]
        .clone();
    altered["values"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["state"] == initial)
        .unwrap()["value"] = json!({"numerator":"9","denominator":"40"});
    let checked = check(&world, "Min", &altered).unwrap();
    assert!(
        matches!(
            checked,
            ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::InequalityFails,
                ..
            })
        ),
        "{checked:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-5")]
#[test]
fn comparison_event_and_fraction_mean_use_distinct_exact_payoffs() {
    let world = World::new(&retry(
        &(claim(
            "Event",
            "Steady",
            &format!("probability >= 0.7 [{} <= 2]", steps()),
        ) + &claim(
            "Mean",
            "Steady",
            "mean of fraction holds(m.delivered) over [0,2] >= rational(2,5)",
        )),
    ));
    let event = proof(&world, "Event", ratio(3, 4));
    assert_eq!(event["objective"]["payoff"], "event");
    let mean = proof(&world, "Mean", ratio(5, 12));
    assert_eq!(mean["objective"]["payoff"], "fraction");
    let checked = check(&world, "Mean", &mean);
    assert!(
        matches!(checked, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "{checked:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-7")]
#[test]
fn changed_objectives_full_keys_and_shifted_range_never_alias() {
    let world = World::new(&retry(&claim(
        "Q",
        "every scheduler",
        &format!("quantile 0.4 of ({}) <= 2", steps()),
    )));
    let doc = proof(&world, "Q", ratio(3, 80));
    for (member, value) in [
        ("window", json!(5)),
        ("threshold", json!({"numerator":"3","denominator":"1"})),
        ("comparison", json!("lt")),
        ("reward", json!([{}, "duration"])),
        ("unit", json!(["declared", {}])),
        ("level", json!({"numerator":"1","denominator":"2"})),
    ] {
        let mut altered = doc.clone();
        altered["objective"]["measure"][member] = value;
        let actual = check(&world, "Q", &altered);
        assert!(
            !matches!(actual, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
            "{member}: {actual:?}"
        );
    }
    let mut changed = doc.clone();
    changed["objective"]["payoff"] = json!("activated");
    let actual = check(&world, "Q", &changed);
    assert!(
        matches!(
            actual,
            Err(fsl_replay::ProbabilityDocumentError::ObjectiveMismatch)
        ),
        "{actual:?}"
    );
    let mut changed = doc.clone();
    changed["values"][0]["value"] = json!({"numerator":"-1","denominator":"1"});
    let actual = check(&world, "Q", &changed);
    assert!(
        matches!(
            actual,
            Ok(ExactProbabilityCertificateCheck::Rejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::Malformed,
                    at: ProbabilityCertificateLocus::Member("values")
                }
            ))
        ),
        "{actual:?}"
    );
    let mut changed = doc.clone();
    let mut decoded = key(&changed["values"][0]);
    decoded[5][2] = json!("unactivated");
    changed["values"][0]["state"] =
        json!(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&decoded).unwrap()));
    let actual = check(&world, "Q", &changed);
    assert!(
        !matches!(actual, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "{actual:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-7", "FR-196-AC-8", "FR-196-AC-12")]
#[test]
fn a_draw_with_multiple_post_states_retains_its_full_pending_control() {
    let claim = claim(
        "Q",
        "every scheduler",
        "quantile 0.4 of (steps from holds(true) until holds(m.delivered) within 2) >= 1",
    );
    let mut model = retry(&claim);
    model.body = model.body.replace("self.delivered = (lost = 1)", "true");
    let world = World::new(&model);
    let doc = proof(&world, "Q", ratio(-2, 5));
    let pending = doc["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(key)
        .filter(|key| !key[7].is_null())
        .collect::<Vec<_>>();
    assert!(
        !pending.is_empty(),
        "a genuine residual post-state choice must enter the graph"
    );
    assert!(pending
        .iter()
        .all(|key| key[7][0] == "drawn" && key[5][0] == "measure"));
    assert!(
        pending
            .iter()
            .any(|key| key[7][2][0] == json!({"type":"integer","value":"0"})),
        "{pending:?}"
    );
    assert!(
        pending
            .iter()
            .any(|key| key[7][2][0] == json!({"type":"integer","value":"1"})),
        "{pending:?}"
    );
    let checked = check(&world, "Q", &doc);
    assert!(
        matches!(checked, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "{checked:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-5", "FR-196-AC-11")]
#[test]
fn a_completed_fraction_does_not_read_an_outside_window_deleted_object() {
    let model=prob_world::coin("probabilistic Mean using ev on M over (c:M::Coin) under Even on origin { mean of fraction holds(c.v = 0) over [0,0] >= rational(1,2) }");
    let world = World::new(&model);
    proof(&world, "Mean", ratio(1, 1));
}

#[trace("TC-3152", "FR-1043-AC-7", "FR-196-AC-13")]
#[test]
fn measure_and_fraction_undefined_reads_replay_the_unadvanced_control() {
    for form in [
        "probability >= 0.5 [steps from holds(c.v = 0) until holds(c.v = 1) within 2 <= 1]",
        "quantile 0.5 of (steps from holds(c.v = 0) until holds(c.v = 1) within 2) <= 1",
        "mean of fraction holds(c.v = 0) over [0,2] >= rational(1,2)",
    ] {
        let source = format!(
            "probabilistic Q using ev on M over (c:M::Coin) under Even on origin {{ {form} }}"
        );
        let world = World::new(&prob_world::coin(&source));
        let record = settle(&world, "Q", ModelCheckLimits::default());
        let terminal = record
            .terminal()
            .unwrap_or_else(|| panic!("{:?}", record.settlement));
        assert_eq!(
            terminal.value(),
            &TerminalValue::Refuted,
            "{form}: {:?}",
            terminal.value()
        );
        let reproduced = record
            .probabilistic_counterexample
            .as_ref()
            .expect("native undefined evidence");
        let undefined = reproduced
            .undefined()
            .expect("an independently reproduced undefined observation");
        assert_eq!(undefined.evaluation.site.position, 1);
        let decoded: Value = serde_json::from_slice(undefined.product_state.as_bytes()).unwrap();
        assert!(!decoded[5].is_null() && !decoded[6].is_null(), "{decoded}");
        let system = ModelSystem::new(world.subject().simulation).unwrap();
        let replay = fsl_replay::replay_probabilistic_undefined(
            &world.package,
            &system,
            &undefined.path.item,
            undefined,
            &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
            &Cancel::new(),
        );
        assert!(
            matches!(
                replay,
                Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
            ),
            "{replay:?}"
        );
    }
}

#[trace("TC-3152", "FR-1043-AC-7")]
#[test]
fn the_independent_measure_checker_preserves_caller_limits_and_cancellation() {
    let world = World::new(&retry(&claim(
        "Q",
        "every scheduler",
        &format!("quantile 0.4 of ({}) <= 2", steps()),
    )));
    let doc = proof(&world, "Q", ratio(3, 80));
    let bytes = serde_json::to_vec(&doc).unwrap();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("Q")).unwrap();
    let limits = fsl_replay::ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 8_000_000,
        states: 128,
        edges: 1024,
        scalar_bits: 128,
        accounting: fsl_replay::AccountingLimits::default().0,
    };
    let run = |limits, cancel: &Cancel| {
        fsl_replay::check_probability_certificate_document(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &item,
            &bytes,
            limits,
            cancel,
        )
    };
    let actual = run(limits, &Cancel::new());
    assert!(
        matches!(actual, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "valid baseline: {actual:?}"
    );
    for (offered, expected) in [
        (
            fsl_replay::ProbabilityDocumentLimits { bytes: 0, ..limits },
            fsl_replay::ProbabilityDocumentLimit::Bytes,
        ),
        (
            fsl_replay::ProbabilityDocumentLimits {
                storage_bytes: 0,
                ..limits
            },
            fsl_replay::ProbabilityDocumentLimit::StorageBytes,
        ),
        (
            fsl_replay::ProbabilityDocumentLimits {
                states: 0,
                ..limits
            },
            fsl_replay::ProbabilityDocumentLimit::States,
        ),
        (
            fsl_replay::ProbabilityDocumentLimits { edges: 0, ..limits },
            fsl_replay::ProbabilityDocumentLimit::Edges,
        ),
        (
            fsl_replay::ProbabilityDocumentLimits { edges: 1, ..limits },
            fsl_replay::ProbabilityDocumentLimit::Edges,
        ),
        (
            fsl_replay::ProbabilityDocumentLimits {
                scalar_bits: 4,
                ..limits
            },
            fsl_replay::ProbabilityDocumentLimit::RationalBits,
        ),
    ] {
        let actual = run(offered, &Cancel::new());
        assert!(
            matches!(actual,Err(fsl_replay::ProbabilityDocumentError::Limit { limit,.. }) if limit==expected),
            "{expected:?}: {actual:?}"
        );
    }
    for accounting in [
        filament_exact::ScalarLimits {
            work_units: 0,
            ..limits.accounting
        },
        filament_exact::ScalarLimits {
            integer_bits: 0,
            ..limits.accounting
        },
        filament_exact::ScalarLimits {
            result_units: 0,
            ..limits.accounting
        },
    ] {
        let actual = run(
            fsl_replay::ProbabilityDocumentLimits {
                accounting,
                ..limits
            },
            &Cancel::new(),
        );
        assert!(
            matches!(
                actual,
                Err(fsl_replay::ProbabilityDocumentError::Accounting(_))
            ),
            "{actual:?}"
        );
    }
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Deadline);
    let actual = run(limits, &cancel);
    assert!(
        matches!(
            actual,
            Err(fsl_replay::ProbabilityDocumentError::Cancelled(
                CancelCause::Deadline
            ))
        ),
        "{actual:?}"
    );
}

#[trace("TC-2066", "FR-1043-AC-4", "FR-203-AC-1")]
#[test]
fn p95_quantity_ratio_is_accepted_from_its_emitted_package_and_two_ms_refutes() {
    let claims="probabilistic P95 using ev on M over (s:M::Server) under Steady on origin { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) within 10) <= 5 ms }\nprobabilistic TooFast using ev on M over (s:M::Server) under Steady on origin { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) within 10) <= 2 ms }";
    let mut model = prob_world::quantity_service("1 ms:90,3 ms:7,12 ms:3", claims);
    model.body = model.body.replace(
        "random Server::attempt(ok) ~ { true: 1 };",
        "random Server::attempt(ok) ~ { true: 98, false: 2 };",
    );
    let world = World::new(&model);
    let document = proof(&world, "P95", ratio(24233, 25000));
    let numerator = &document["conditional"]["numerator"];
    assert_eq!(numerator["objective"]["measure"]["kind"], "accumulate");
    assert_eq!(numerator["objective"]["measure"]["reward"][1], "duration");
    assert!(!numerator["objective"]["measure"]["unit"].is_null());
    assert_eq!(
        numerator["objective"]["measure"]["threshold"],
        json!({"numerator":"5","denominator":"1"})
    );
    let reward = world
        .package
        .graph()
        .probabilistic_model()
        .rewards()
        .iter()
        .find(|reward| reward.name() == "duration")
        .unwrap();
    let filament_exact::ValueType::Quantity(unit) = reward.value_type() else {
        panic!("the duration reward must retain its actual unit");
    };
    let mut unit_bytes = Vec::new();
    let mut writer =
        quire_canonical::Writer::new(&mut unit_bytes, quire_canonical::Limits::new(4096));
    fsl_foundation::scalar_wire::unit(Some(unit), &mut writer, |integer, writer| {
        fsl_foundation::scalar_wire::integer(integer, 4096, None, writer)
    })
    .unwrap();
    drop(writer);
    let actual_unit: Value = serde_json::from_slice(&unit_bytes).unwrap();
    assert_eq!(numerator["objective"]["measure"]["unit"], actual_unit);
    let denominator = &document["conditional"]["denominator"];
    let initial = denominator["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["rank"] == 10)
        .unwrap();
    let initial = denominator["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["state"] == initial["state"])
        .unwrap();
    assert_eq!(initial["value"], json!({"numerator":"1","denominator":"1"}));
    let emitted = fsl_package::emit_checked(
        &world.package,
        fsl_replay::spine::SpineLimits::default().package.emit(),
    )
    .unwrap();
    let simulation = world.subject().simulation;
    let mut documents = vec![model.document(false)];
    documents.extend(simulation.snapshots.values().cloned());
    let provision = fsl_replay::ByteProvision::admit(
        world
            .provision
            .entries()
            .map(|(authority, identity, bytes)| {
                (authority.to_owned(), identity.to_owned(), bytes.to_vec())
            })
            .collect(),
        documents,
        std::iter::empty(),
    )
    .unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("P95")).unwrap();
    let limits = fsl_replay::ProbabilitySourceLimits {
        spine: fsl_replay::spine::SpineLimits::default(),
        observation: simulation.observation,
        expansion: simulation.expansion,
        scalar: simulation.meter,
        document: fsl_replay::ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 8_000_000,
            states: 128,
            edges: 1024,
            scalar_bits: 128,
            accounting: fsl_replay::AccountingLimits::default().0,
        },
    };
    let check = |document: &Value| {
        let bytes = serde_json::to_vec(document).unwrap();
        fsl_replay::check_probability_certificate_package(
            fsl_replay::ProbabilityPackageRequest {
                package: emitted.package().bytes(),
                package_name: Some("package"),
                provision: &provision,
                item: &item,
                initial: simulation.initial.clone(),
                universes: simulation.universes.clone(),
                certificate: &bytes,
            },
            limits,
            &Cancel::new(),
        )
    };
    let checked = check(&document);
    assert!(
        matches!(checked, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "{checked:?}"
    );
    let mut altered = document.clone();
    altered["conditional"]["numerator"]["values"][0]["value"] =
        json!({"numerator":"1","denominator":"7"});
    let checked = check(&altered);
    assert!(
        matches!(
            checked,
            Ok(ExactProbabilityCertificateCheck::Rejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::InequalityFails,
                    at: ProbabilityCertificateLocus::ProductPart {
                        part: fsl_replay::ProbabilityCertificatePart::Numerator,
                        ..
                    }
                }
            ))
        ),
        "{checked:?}"
    );
    let too_fast = settle(&world, "TooFast", ModelCheckLimits::default());
    assert_eq!(
        too_fast.terminal().unwrap().value(),
        &TerminalValue::Refuted,
        "{:?}",
        too_fast.settlement
    );
    assert_eq!(
        too_fast
            .probabilistic_counterexample
            .as_ref()
            .unwrap()
            .evidence()
            .paths
            .len(),
        1
    );
}

#[trace("TC-3152", "FR-1043-AC-7", "FR-196-AC-13")]
#[test]
fn an_undefined_measure_after_merged_draws_replays_a_real_positive_draw() {
    let source="probabilistic Q using ev on M over (c:M::Coin) under Even on origin { probability >= 0.5 [steps from holds(c.v = 0) until holds(false) within 3 <= 2] }";
    let mut model = prob_world::coin("");
    model.operations.push(prob_world::Operation {
        name: "advance",
        parameters: vec![("bit", "ix://test/coin/Bit".into())],
        modifies: vec!["v"],
        deletes: false,
    });
    model.body = model
        .body
        .replace(
            "post Drop using v",
            "pre CanDrop using v on M::Coin::drop { self.v = 1 }\npost Drop using v",
        )
        .replace(
            "weight Coin::drop = 1;",
            "weight Coin::drop = 1; weight Coin::advance = 1;",
        );
    model.body.push_str("\npre CanAdvance using v on M::Coin::advance { self.v = 0 }\npost Advanced using v on M::Coin::advance { self.v = 1 }\nrandom Coin::advance(bit) ~ { 0:1,1:1 };\n");
    model.body.push_str(source);
    let world = World::new(&model);
    let record = settle(&world, "Q", ModelCheckLimits::default());
    assert_eq!(
        record.terminal().unwrap().value(),
        &TerminalValue::Refuted,
        "{:?}",
        record.settlement
    );
    let token = record
        .probabilistic_counterexample
        .as_ref()
        .expect("a native reproduced path");
    let undefined = token.undefined().unwrap();
    assert_eq!(undefined.evaluation.site.position, 2);
    assert_eq!(undefined.path.paths[0].steps.len(), 2);
    assert!(undefined.path.paths[0].steps[0].draw.is_some());
    assert_eq!(token.mass(), &ratio(1, 4));
}

fn assert_vacuity_result_schema(terminal: &TerminalRecord, certificate: &Value) {
    let published = crate::published_fspec::json("proposals/backend-provider-v1/schema.json");
    let package = crate::published_fspec::json("proposals/checked-package-v2/schema.json");
    let schema =
        json!({"$schema":published["$schema"],"$defs":published["$defs"],"$ref":"#/$defs/Result"});
    let schema = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .with_document("urn:filament:checked-package:v2".into(), package)
        .compile(&schema)
        .unwrap();
    let outcome = serde_json::to_value(fsl_replay::OutcomeItem::from_terminal(terminal)).unwrap();
    let result = json!({"request_index":terminal.request_index().get(),
        "backend":terminal.backend().unwrap().as_str(),"kind":"probabilistic-satisfaction",
        "result":outcome["result"],"basis":outcome["basis"],"certification":outcome["certification"],
        "certificate":certificate});
    if let Err(errors) = schema.validate(&result) {
        panic!(
            "actual certified vacuity carrier: {:?}\n{result}",
            errors.map(|error| error.to_string()).collect::<Vec<_>>()
        );
    }
    let mut ordinary = result.clone();
    ordinary["certificate"]["kind"] = json!("exact");
    assert!(
        !schema.is_valid(&ordinary),
        "exhaustive forbids every other certificate kind"
    );
    let mut evidence = result.clone();
    evidence["evidence"] = json!({});
    assert!(
        !schema.is_valid(&evidence),
        "generic evidence remains forbidden"
    );
    let mut scalar_free_exact = result;
    scalar_free_exact["basis"] = json!({"type":"exact-value"});
    assert!(
        !schema.is_valid(&scalar_free_exact),
        "exact-value still needs its actual rational"
    );
}

#[trace("FR-1043-AC-10", "FR-203-AC-10", "FR-203-AC-11")]
#[test]
fn unreachable_activation_has_a_certified_value_free_vacuity_basis_for_each_scheduler() {
    for scheduler in [
        "Steady",
        "every scheduler",
        "every scheduler fair weak Msg::fast",
    ] {
        let world = World::new(&retry(
            &(claim(
                "Le",
                scheduler,
                "quantile 0.9 of (steps from holds(false) until holds(m.delivered) within 4) <= 2",
            ) + &claim(
                "Ge",
                scheduler,
                "quantile 0.9 of (steps from holds(false) until holds(m.delivered) within 4) >= 2",
            )),
        ));
        let system = ModelSystem::new(world.subject().simulation).unwrap();
        for name in ["Le", "Ge"] {
            let record = settle(&world, name, ModelCheckLimits::default());
            let terminal = record
                .terminal()
                .unwrap_or_else(|| panic!("{:?}", record.settlement));
            assert!(
                matches!(
                    terminal.value(),
                    TerminalValue::Proved {
                        basis: ProofBasis::Exhaustive,
                        certification: Certification::Certified
                    }
                ),
                "{scheduler}/{name}: {:?}",
                terminal.value()
            );
            assert!(
                record.exact_probabilistic.is_none(),
                "vacuity has no invented exact value"
            );
            let document: Value =
                serde_json::from_slice(terminal.exact_certificate_document().unwrap()).unwrap();
            assert_eq!(document["kind"], "conditional");
            assert_eq!(document["conditional"], json!({"mode":"vacuous"}));
            assert_eq!(document["values"], json!([]));
            assert_vacuity_result_schema(terminal, &document);
            let checked = check(&world, name, &document);
            assert!(
                matches!(
                    checked,
                    Ok(ExactProbabilityCertificateCheck::AcceptedVacuous(_))
                ),
                "{checked:?}"
            );
            let item = fsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
            let claims = world.package.graph().probabilistic_model().claims();
            let zero_document = |payoff| {
                let mut document = fsl_replay::measure_probability_certificate_document(
                    &system,
                    claims,
                    &item,
                    &ExactProbabilityCertificate { values: Vec::new() },
                    payoff,
                    document_limits(),
                    &Cancel::new(),
                )
                .unwrap();
                document.values = document
                    .ranking
                    .iter()
                    .map(|(state, _)| fsl_replay::ExactProbabilityValue {
                        state: state.clone(),
                        value: ratio(0, 1),
                    })
                    .collect();
                document
            };
            // These are complete, mathematically exact zero vectors. The
            // refusal must be the vacuity role, rather than invalid setup.
            let numeric = if scheduler == "Steady" {
                fsl_replay::conditional_measure_probability_certificate(
                    zero_document(fsl_replay::MeasurePayoffKind::Satisfied),
                    zero_document(fsl_replay::MeasurePayoffKind::Activated),
                )
                .unwrap()
            } else {
                zero_document(fsl_replay::MeasurePayoffKind::Shifted)
            };
            let numeric: Value = serde_json::from_slice(
                &fsl_replay::write_measure_probability_certificate(
                    &numeric,
                    document_limits(),
                    &Cancel::new(),
                )
                .unwrap(),
            )
            .unwrap();
            let part = if scheduler == "Steady" {
                &numeric["conditional"]["numerator"]
            } else {
                &numeric
            };
            let initial = part["ranking"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["rank"] == 4)
                .unwrap();
            let expected = ProbabilityProductKey::from_canonical_bytes(
                URL_SAFE_NO_PAD
                    .decode(initial["state"].as_str().unwrap())
                    .unwrap(),
            );
            let checked = check(&world, name, &numeric);
            assert!(matches!(&checked, Ok(ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::InequalityFails,
                at: ProbabilityCertificateLocus::ProductState(at)
            })) if at == &expected), "numeric no-activation {scheduler}/{name}: {checked:?}; expected initial {expected:?}");
        }
    }
    for scheduler in [
        "Steady",
        "every scheduler",
        "every scheduler fair weak Msg::fast",
    ] {
        let activated = World::new(&retry(&claim(
            "Q",
            scheduler,
            &format!("quantile 0.4 of ({}) <= 2", steps()),
        )));
        let expected = if scheduler == "Steady" {
            ratio(3, 4)
        } else {
            ratio(3, 80)
        };
        let numeric = proof(&activated, "Q", expected);
        assert_ne!(numeric["conditional"]["mode"], "vacuous");
        let system = ModelSystem::new(activated.subject().simulation).unwrap();
        let item = fsl_replay::probabilistic_claim_identity(activated.claim("Q")).unwrap();
        let forged = fsl_replay::vacuous_measure_probability_certificate_document(
            &system,
            activated.package.graph().probabilistic_model().claims(),
            &item,
            document_limits(),
            &Cancel::new(),
        )
        .unwrap();
        let forged: Value = serde_json::from_slice(
            &fsl_replay::write_measure_probability_certificate(
                &forged,
                document_limits(),
                &Cancel::new(),
            )
            .unwrap(),
        )
        .unwrap();
        let checked = check(&activated, "Q", &forged);
        assert!(
            matches!(
                checked,
                Ok(ExactProbabilityCertificateCheck::Rejected(
                    ProbabilityCertificateRejection {
                        rule: ProbabilityCertificateRule::InequalityFails,
                        ..
                    }
                ))
            ),
            "{scheduler}: {checked:?}"
        );
    }
}

fn document_limits() -> fsl_replay::ProbabilityDocumentLimits {
    fsl_replay::ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 8_000_000,
        states: 128,
        edges: 1024,
        scalar_bits: 128,
        accounting: fsl_replay::AccountingLimits::default().0,
    }
}

#[trace(
    "TC-3152",
    "FR-1043-AC-5",
    "FR-202-AC-27",
    "FR-1043-AC-9",
    "FR-202-AC-29"
)]
#[test]
fn workload_mean_refutation_uses_a_checked_subsystem_and_equality_never_refutes() {
    let claims = claim(
        "No",
        "Steady",
        "mean of fraction holds(m.delivered) over [0,2] >= rational(1,2)",
    ) + &claim(
        "EqualGe",
        "Steady",
        "mean of fraction holds(m.delivered) over [0,2] >= rational(5,12)",
    ) + &claim(
        "EqualLe",
        "Steady",
        "mean of fraction holds(m.delivered) over [0,2] <= rational(5,12)",
    );
    let world = World::new(&retry(&claims));
    let record = settle(&world, "No", ModelCheckLimits::default());
    assert_eq!(
        record.terminal().unwrap().value(),
        &TerminalValue::Refuted,
        "{:?}",
        record.settlement
    );
    let token = record
        .probabilistic_counterexample
        .as_ref()
        .expect("a checked subsystem");
    assert!(token.evidence().paths.is_empty());
    let subsystem = token
        .evidence()
        .subsystem
        .as_ref()
        .expect("fraction expectation certificate");
    let document: Value = serde_json::from_slice(subsystem.as_bytes()).unwrap();
    assert_eq!(document["kind"], "exact");
    assert_eq!(document["objective"]["payoff"], "fraction");
    assert_eq!(
        token.wire.as_ref().unwrap()["evidence"]["type"],
        "subsystem"
    );
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let replay = fsl_replay::replay_probabilistic_witness(
        &world.package,
        &system,
        &token.evidence().item,
        token.evidence(),
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        &Cancel::new(),
    );
    assert!(
        matches!(
            replay,
            Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
        ),
        "{replay:?}"
    );
    for name in ["EqualGe", "EqualLe"] {
        proof(&world, name, ratio(5, 12));
        let mut offered = token.evidence().clone();
        offered.item = fsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
        let mut document = fsl_replay::read_measure_probability_certificate(
            subsystem.as_bytes(),
            document_limits(),
            &Cancel::new(),
        )
        .unwrap();
        document.identity = offered.item.clone();
        offered.subsystem = Some(Box::new(
            fsl_replay::ProbabilitySubsystemCertificate::from_document(
                &document,
                document_limits(),
                &Cancel::new(),
            )
            .unwrap(),
        ));
        let replay = fsl_replay::replay_probabilistic_witness(
            &world.package,
            &system,
            &offered.item,
            &offered,
            &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
            &Cancel::new(),
        );
        assert!(
            matches!(
                replay,
                Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                    ProbabilityCertificateRejection {
                        rule: ProbabilityCertificateRule::ThresholdMissed,
                        ..
                    }
                ))
            ),
            "equality: {replay:?}"
        );
    }
    let mut document = fsl_replay::read_measure_probability_certificate(
        subsystem.as_bytes(),
        document_limits(),
        &Cancel::new(),
    )
    .unwrap();
    document.values[0].value = ratio(1, 7);
    let mut offered = token.evidence().clone();
    offered.subsystem = Some(Box::new(
        fsl_replay::ProbabilitySubsystemCertificate::from_document(
            &document,
            document_limits(),
            &Cancel::new(),
        )
        .unwrap(),
    ));
    let replay = fsl_replay::replay_probabilistic_witness(
        &world.package,
        &system,
        &offered.item,
        &offered,
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        &Cancel::new(),
    );
    assert!(
        matches!(
            replay,
            Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::InequalityFails,
                    ..
                }
            ))
        ),
        "altered exact value: {replay:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-8", "FR-1043-AC-9", "FR-202-AC-29")]
#[test]
fn every_scheduler_exact_mean_witness_attains_the_deciding_extremum() {
    let claims = claim(
        "MinNo",
        "every scheduler",
        "mean of fraction holds(m.delivered) over [0,2] >= rational(1,2)",
    ) + &claim(
        "MaxNo",
        "every scheduler",
        "mean of fraction holds(m.delivered) over [0,2] <= rational(1,3)",
    );
    let world = World::new(&retry(&claims));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let initial = system.initial().remove(0);
    let actions = system.actions(&initial).unwrap().unwrap();
    let [fast, slow] = actions.as_slice() else {
        panic!("expected exactly fast and slow: {actions:?}");
    };
    assert_ne!(fast.identity.operation, slow.identity.operation);
    for (name, expected, action) in [
        ("MinNo", ratio(11, 48), &slow.identity),
        ("MaxNo", ratio(5, 12), &fast.identity),
    ] {
        let record = settle(&world, name, ModelCheckLimits::default());
        assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
        let token = record.probabilistic_counterexample.as_ref().unwrap();
        let witness = token.evidence();
        let document: Value =
            serde_json::from_slice(witness.subsystem.as_ref().unwrap().as_bytes()).unwrap();
        let root = URL_SAFE_NO_PAD.encode(witness.initial_key.as_bytes());
        let value = document["values"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["state"] == root)
            .unwrap();
        assert_eq!(
            value["value"],
            json!({"numerator":expected.numerator().to_string(),"denominator":expected.denominator().to_string()})
        );
        let selected = witness
            .scheduler
            .iter()
            .find(|(key, _)| key == &witness.initial_key)
            .unwrap();
        assert_eq!(
            selected.1,
            fsl_replay::ProductAction::Identity(fsl_replay::Schedule::of(&system, action).unwrap())
        );
    }
    let baseline = settle(&world, "MinNo", ModelCheckLimits::default());
    let token = baseline.probabilistic_counterexample.as_ref().unwrap();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let initial = system.initial().remove(0);
    let fast = system
        .actions(&initial)
        .unwrap()
        .unwrap()
        .remove(0)
        .identity;
    let fast =
        fsl_replay::ProductAction::Identity(fsl_replay::Schedule::of(&system, &fast).unwrap());
    let mut changed = token.evidence().clone();
    let row = changed
        .scheduler
        .iter_mut()
        .find(|(key, _)| key == &changed.initial_key)
        .unwrap();
    assert_ne!(
        row.1, fast,
        "the adverse action must differ from the actual minimizing policy"
    );
    row.1 = fast;
    let replay = fsl_replay::replay_probabilistic_witness(
        &world.package,
        &system,
        &changed.item,
        &changed,
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        &Cancel::new(),
    );
    assert!(
        matches!(replay, Ok(fsl_replay::ProbabilityWitnessReplay::Parity)),
        "{replay:?}"
    );
}

fn mean_bound(
    witness: &fsl_replay::ProbabilisticCounterexample,
    lower: bool,
    every: bool,
) -> Value {
    let mut document: Value =
        serde_json::from_slice(witness.subsystem.as_ref().unwrap().as_bytes()).unwrap();
    let base = document["objective"].take();
    document["objective"] =
        json!({"profile":"filament.probability-objective/value-bounds/v1","base":base});
    document["kind"] = json!(if lower { "lower" } else { "upper" });
    if lower {
        // The least unnamed value is zero. Retain all positive rows, including
        // their actual horizon ranks, without offering an upper counterpart.
        document["values"]
            .as_array_mut()
            .unwrap()
            .retain(|row| row["value"]["numerator"] != "0");
        let states = document["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["state"].clone())
            .collect::<Vec<_>>();
        document["ranking"]
            .as_array_mut()
            .unwrap()
            .retain(|row| states.contains(&row["state"]));
    } else {
        document.as_object_mut().unwrap().remove("ranking");
    }
    if every {
        let states = document["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["state"].clone())
            .collect::<Vec<_>>();
        document["policy"] = Value::Array(
            witness
                .scheduler
                .iter()
                .filter_map(|(key, action)| {
                    let state = json!(URL_SAFE_NO_PAD.encode(key.as_bytes()));
                    if !states.contains(&state) {
                        return None;
                    }
                    let fsl_replay::ProductAction::Identity(schedule) = action else {
                        return None;
                    };
                    let transition = schedule
                        .transition_identity(quire_canonical::Limits::new(document_limits().bytes))
                        .unwrap();
                    let choice: Value = serde_json::from_slice(
                        &transition
                            .canonical_bytes(quire_canonical::Limits::new(document_limits().bytes))
                            .unwrap(),
                    )
                    .unwrap();
                    Some(json!({"state":state,"choice":choice}))
                })
                .collect(),
        );
    }
    document
}

fn replay_mean_document(
    world: &World,
    baseline: &fsl_replay::ProbabilisticCounterexample,
    document: &Value,
) -> Result<fsl_replay::ProbabilityWitnessReplay, fsl_replay::ProbabilityWitnessError> {
    let mut offered = baseline.clone();
    offered.subsystem = Some(Box::new(
        fsl_replay::ProbabilitySubsystemCertificate::from_bytes(
            &serde_json::to_vec(document).unwrap(),
            document_limits(),
            &Cancel::new(),
        )
        .unwrap(),
    ));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    fsl_replay::replay_probabilistic_witness(
        &world.package,
        &system,
        &offered.item,
        &offered,
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        &Cancel::new(),
    )
}

fn settle_mean_document(
    world: &World,
    name: &str,
    document: &Value,
) -> fsl_analyze::analyze::AnalyzeRecord {
    let bytes = serde_json::to_vec(document).unwrap();
    let item = fsl_analyze::analyze::exact_probabilistic_item_witness_tampered(
        RequestIndex::new(0),
        world.request(name, ModelCheckLimits::default()),
        move |witness| {
            witness.subsystem = Some(Box::new(
                fsl_replay::ProbabilitySubsystemCertificate::from_bytes(
                    &bytes,
                    document_limits(),
                    &Cancel::new(),
                )
                .unwrap(),
            ));
        },
    );
    analyze_items(&Engines::fsl().unwrap(), vec![item], &Cancel::new()).remove(0)
}

#[trace("FR-1043-AC-8", "FR-1043-AC-9", "FR-202-AC-28", "FR-202-AC-29")]
#[test]
fn bounded_mean_refutation_roles_accept_upper_minimum_and_partial_lower_maximum() {
    for scheduler in ["Steady", "every scheduler"] {
        let world = World::new(&retry(
            &(claim(
                "Min",
                scheduler,
                "mean of fraction holds(m.delivered) over [0,2] >= rational(1,2)",
            ) + &claim(
                "Max",
                scheduler,
                "mean of fraction holds(m.delivered) over [0,2] <= rational(1,3)",
            )),
        ));
        for (name, lower) in [("Min", false), ("Max", true)] {
            let baseline = settle(&world, name, ModelCheckLimits::default());
            assert_eq!(
                baseline.terminal().unwrap().value(),
                &TerminalValue::Refuted,
                "{:?}",
                baseline.settlement
            );
            let token = baseline.probabilistic_counterexample.as_ref().unwrap();
            let mut document = mean_bound(token.evidence(), lower, scheduler == "every scheduler");
            if scheduler == "every scheduler" {
                let root = URL_SAFE_NO_PAD.encode(token.evidence().initial_key.as_bytes());
                document["values"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|row| row["state"] == root)
                    .unwrap()["value"] = if lower {
                    json!({"numerator":"2","denominator":"5"})
                } else {
                    json!({"numerator":"1","denominator":"4"})
                };
            }
            let replay = replay_mean_document(&world, token.evidence(), &document);
            assert!(
                matches!(
                    replay,
                    Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
                ),
                "{name}/{scheduler}: {replay:?}"
            );
            let root = URL_SAFE_NO_PAD.encode(token.evidence().initial_key.as_bytes());
            let mut equality = document.clone();
            let row = equality["values"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row["state"] == root)
                .expect("the initial bound");
            row["value"] = if lower {
                json!({"numerator":"1","denominator":"3"})
            } else {
                json!({"numerator":"1","denominator":"2"})
            };
            let replay = replay_mean_document(&world, token.evidence(), &equality);
            assert!(
                matches!(
                    replay,
                    Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                        ProbabilityCertificateRejection {
                            rule: ProbabilityCertificateRule::ThresholdMissed,
                            ..
                        }
                    ))
                ),
                "valid non-refuting equality: {replay:?}"
            );
            let mut invalid = document.clone();
            let row = invalid["values"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row["state"] == root)
                .unwrap();
            row["value"] = json!({"numerator":if lower { "1" } else { "0" },"denominator":"1"});
            let replay = replay_mean_document(&world, token.evidence(), &invalid);
            assert!(
                matches!(
                    replay,
                    Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                        ProbabilityCertificateRejection {
                            rule: ProbabilityCertificateRule::InequalityFails,
                            ..
                        }
                    ))
                ),
                "invalid bound: {replay:?}"
            );
            if lower {
                let original: Value =
                    serde_json::from_slice(token.evidence().subsystem.as_ref().unwrap().as_bytes())
                        .unwrap();
                assert!(
                    document["values"].as_array().unwrap().len()
                        < original["values"].as_array().unwrap().len(),
                    "the lower must really omit states"
                );
                let mut changed = document.clone();
                changed["ranking"][0]["rank"] = json!(99);
                let replay = replay_mean_document(&world, token.evidence(), &changed);
                assert!(
                    matches!(
                        replay,
                        Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                            ProbabilityCertificateRejection {
                                rule: ProbabilityCertificateRule::RankBroken,
                                ..
                            }
                        ))
                    ),
                    "{replay:?}"
                );
            } else {
                let mut changed = document.clone();
                changed["values"].as_array_mut().unwrap().pop();
                let replay = replay_mean_document(&world, token.evidence(), &changed);
                assert!(
                    matches!(
                        replay,
                        Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                            ProbabilityCertificateRejection {
                                rule: ProbabilityCertificateRule::Malformed,
                                at: fsl_replay::ProbabilityCertificateLocus::Member("values")
                            }
                        ))
                    ),
                    "{replay:?}"
                );
            }
            let mut changed = document.clone();
            changed["objective"]["base"]["measure"]["window"] = json!(3);
            let replay = replay_mean_document(&world, token.evidence(), &changed);
            assert!(
                matches!(replay, Err(fsl_replay::ProbabilityWitnessError::Refused)),
                "{replay:?}"
            );
        }
    }
}

#[trace("FR-1043-AC-2", "FR-1043-AC-5", "FR-1043-AC-8")]
#[test]
fn bounded_fair_measure_proofs_and_native_refutations_keep_the_same_optimum() {
    let claims = claim(
        "Q",
        "every scheduler fair weak Msg::fast",
        &format!("quantile 0.4 of ({}) <= 3", steps()),
    ) + &claim(
        "No",
        "every scheduler fair weak Msg::fast",
        &format!("quantile 0.5 of ({}) <= 2", steps()),
    ) + &claim(
        "MeanNo",
        "every scheduler fair strong Msg::fast",
        "mean of fraction holds(m.delivered) over [0,2] >= rational(1,2)",
    );
    let world = World::new(&retry(&claims));
    let document = proof(&world, "Q", ratio(57, 320));
    assert_eq!(
        document["objective"]["fairness"].as_array().unwrap().len(),
        1
    );
    let accepted = check(&world, "Q", &document);
    assert!(
        matches!(accepted, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "{accepted:?}"
    );
    refuted(&world, "No");
    refuted(&world, "MeanNo");
}

#[trace("FR-1043-AC-4", "FR-1043-AC-5", "FR-1043-AC-8")]
#[test]
fn weighted_fraction_keeps_exact_sums_and_checks_its_mean_independently() {
    let mut model = retry(
        &(claim(
            "Yes",
            "Steady",
            "mean of fraction holds(m.delivered) over [0,2] weighted by effort >= rational(29,100)",
        ) + &claim(
            "No",
            "Steady",
            "mean of fraction holds(m.delivered) over [0,2] weighted by effort >= rational(3,10)",
        )),
    );
    model.body = model
        .body
        .replace("{ not self.delivered }", "{ true }")
        .replace("terminal Done using v on M when { self.delivered };", "")
        + "\nreward effort on Msg::fast = lost + 1;\nreward effort on Msg::slow = lost + 1;\n";
    let world = World::new(&model);
    // Eight equally likely triples of positive weights, independently summed
    // as (a*(b+1)+b*(c+1))/(a+b+c+3), have this exact average.
    let document = proof(&world, "Yes", ratio(71, 240));
    assert_eq!(
        document["objective"]["measure"]["kind"],
        "weighted-fraction"
    );
    let accepted = check(&world, "Yes", &document);
    assert!(
        matches!(accepted, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "{accepted:?}"
    );
    let accumulators = document["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| key(row)[6].clone())
        .collect::<Vec<_>>();
    assert!(
        accumulators
            .iter()
            .any(|sum| sum.to_string().contains("weighted-fraction")),
        "{accumulators:?}"
    );
    refuted(&world, "No");
}

#[trace(
    "FR-1043-AC-8",
    "FR-1043-AC-9",
    "FR-202-AC-28",
    "FR-1042-AC-13",
    "FR-1042-AC-14"
)]
#[test]
fn mean_bound_policy_hints_constrain_search_and_absence_reconstructs_internal_choices() {
    let claims = claim(
        "Min",
        "every scheduler",
        "mean of fraction holds(m.delivered) over [0,2] >= rational(1,2)",
    ) + &claim(
        "Max",
        "every scheduler",
        "mean of fraction holds(m.delivered) over [0,2] <= rational(1,3)",
    );
    let world = World::new(&retry(&claims));
    let minimum = settle(&world, "Min", ModelCheckLimits::default());
    let maximum = settle(&world, "Max", ModelCheckLimits::default());
    let minimum = minimum
        .probabilistic_counterexample
        .as_ref()
        .unwrap()
        .evidence();
    let maximum = maximum
        .probabilistic_counterexample
        .as_ref()
        .unwrap()
        .evidence();
    let document = mean_bound(minimum, false, true);
    let mut bad_hint = document.clone();
    let opposite = mean_bound(maximum, true, true);
    let root = URL_SAFE_NO_PAD.encode(minimum.initial_key.as_bytes());
    let opposite = opposite["policy"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["state"] == root)
        .unwrap();
    let row = bad_hint["policy"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["state"] == root)
        .unwrap();
    assert_ne!(
        row["choice"], opposite["choice"],
        "a genuinely different enabled action"
    );
    row["choice"] = opposite["choice"].clone();
    let rejected = replay_mean_document(&world, minimum, &bad_hint);
    assert!(
        matches!(
            rejected,
            Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::InequalityFails,
                    ..
                }
            ))
        ),
        "a bad supplied hint must not fall back: {rejected:?}"
    );
    let mut nonexistent = document.clone();
    let hint = nonexistent["policy"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["state"] == root)
        .unwrap();
    let original = hint["choice"]["operation"].as_str().unwrap();
    let missing = original.replace("slow", "missing");
    assert_ne!(original, missing, "the hint must name an absent operation");
    hint["choice"]["operation"] = json!(missing);
    let rejected = replay_mean_document(&world, minimum, &nonexistent);
    assert!(
        matches!(
            rejected,
            Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::InequalityFails,
                    ..
                }
            ))
        ),
        "nonexistent hint: {rejected:?}"
    );
    let mut no_hint = document;
    no_hint.as_object_mut().unwrap().remove("policy");
    let mut offered = minimum.clone();
    offered.scheduler.clear();
    let accepted = replay_mean_document(&world, &offered, &no_hint);
    let Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(token)) = accepted else {
        panic!("{accepted:?}");
    };
    assert!(
        token
            .evidence()
            .scheduler
            .iter()
            .any(|(_, action)| matches!(action, fsl_replay::ProductAction::TerminalStutter)),
        "terminal-only choices must be inferred"
    );

    let mut model = retry(&claims);
    model.body = model.body.replace("self.delivered = (lost = 1)", "true");
    let world = World::new(&model);
    let record = settle(&world, "Min", ModelCheckLimits::default());
    assert_eq!(
        record.terminal().unwrap().value(),
        &TerminalValue::Refuted,
        "{:?}",
        record.settlement
    );
    let mut offered = record
        .probabilistic_counterexample
        .as_ref()
        .unwrap()
        .evidence()
        .clone();
    let mut document = mean_bound(&offered, false, true);
    document.as_object_mut().unwrap().remove("policy");
    offered.scheduler.clear();
    let accepted = replay_mean_document(&world, &offered, &document);
    let Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(token)) = accepted else {
        panic!("{accepted:?}");
    };
    assert!(
        token
            .evidence()
            .scheduler
            .iter()
            .any(|(_, action)| matches!(action, fsl_replay::ProductAction::PostState(_))),
        "genuine residual post-state choices must be inferred"
    );
    // Both residual outcomes contribute zero when the final departing source
    // is false. Their target keys define the canonical action tie break.
    let mut checked_tie = false;
    for (source, action) in &token.evidence().scheduler {
        let source: Value = serde_json::from_slice(source.as_bytes()).unwrap();
        if source[7].is_null() || source[5][1] != "1" || source[6][1] != "0" {
            continue;
        }
        let fsl_replay::ProductAction::PostState(target) = action else {
            continue;
        };
        let target_json: Value = serde_json::from_slice(target.as_bytes()).unwrap();
        let expected = document["values"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                let other = key(row);
                other[7].is_null() && other[5] == target_json[5] && other[6] == target_json[6]
            })
            .map(|row| {
                URL_SAFE_NO_PAD
                    .decode(row["state"].as_str().unwrap())
                    .unwrap()
            })
            .min()
            .unwrap();
        assert_eq!(target.as_bytes(), expected);
        checked_tie = true;
    }
    assert!(
        checked_tie,
        "the fixture must exercise a real satisfying-action tie"
    );
    // This pending state's accumulated true contribution makes every
    // residual action strictly positive. Zero has no satisfying action.
    let pending = document["values"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| {
            let state = key(row);
            !state[7].is_null() && row["value"]["numerator"] != "0"
        })
        .expect("a genuine positive residual choice");
    pending["value"] = json!({"numerator":"0","denominator":"1"});
    let rejected = replay_mean_document(&world, &offered, &document);
    assert!(
        matches!(
            rejected,
            Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::InequalityFails,
                    ..
                }
            ))
        ),
        "{rejected:?}"
    );
}

#[trace("FR-1043-AC-9", "FR-202-AC-29")]
#[test]
fn valid_mean_bounds_on_the_threshold_and_wrong_bound_roles_are_rejected() {
    let world = World::new(&retry(
        &(claim(
            "Ge",
            "Steady",
            "mean of fraction holds(m.delivered) over [0,2] >= rational(1,2)",
        ) + &claim(
            "Le",
            "Steady",
            "mean of fraction holds(m.delivered) over [0,2] <= rational(2,5)",
        )),
    ));
    for (name, lower, value) in [
        ("Ge", false, json!({"numerator":"1","denominator":"2"})),
        ("Le", true, json!({"numerator":"2","denominator":"5"})),
    ] {
        let record = settle(&world, name, ModelCheckLimits::default());
        assert_eq!(
            record.terminal().unwrap().value(),
            &TerminalValue::Refuted,
            "{:?}",
            record.settlement
        );
        let witness = record
            .probabilistic_counterexample
            .as_ref()
            .unwrap()
            .evidence();
        let mut document = mean_bound(witness, lower, false);
        let root = URL_SAFE_NO_PAD.encode(witness.initial_key.as_bytes());
        document["values"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["state"] == root)
            .unwrap()["value"] = value;
        let replay = replay_mean_document(&world, witness, &document);
        assert!(
            matches!(
                replay,
                Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                    ProbabilityCertificateRejection {
                        rule: ProbabilityCertificateRule::ThresholdMissed,
                        ..
                    }
                ))
            ),
            "a valid nonrefuting bound: {replay:?}"
        );
        let record = settle_mean_document(&world, name, &document);
        assert!(
            matches!(
                record.terminal().unwrap().value(),
                TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
                    rule: CertificateRule::ThresholdMissed,
                    ..
                })
            ),
            "{:?}",
            record.settlement
        );
        assert!(
            record.probabilistic_counterexample.is_none() && record.exact_probabilistic.is_none()
        );
    }
    let world = World::new(&retry(
        &(claim(
            "Min",
            "every scheduler",
            "mean of fraction holds(m.delivered) over [0,2] >= rational(1,2)",
        ) + &claim(
            "Max",
            "every scheduler",
            "mean of fraction holds(m.delivered) over [0,2] <= rational(1,3)",
        )),
    ));
    for (name, correct_lower) in [("Min", false), ("Max", true)] {
        let record = settle(&world, name, ModelCheckLimits::default());
        let witness = record
            .probabilistic_counterexample
            .as_ref()
            .unwrap()
            .evidence();
        // Construct a valid bound for the opposite role, retaining complete
        // coverage and the ranks required by that role. Only its direction
        // is inappropriate for refuting this checked claim.
        let mut document: Value =
            serde_json::from_slice(witness.subsystem.as_ref().unwrap().as_bytes()).unwrap();
        let base = document["objective"].take();
        document["objective"] =
            json!({"profile":"filament.probability-objective/value-bounds/v1","base":base});
        document["kind"] = json!(if correct_lower { "upper" } else { "lower" });
        if correct_lower {
            document.as_object_mut().unwrap().remove("ranking");
        }
        let root = URL_SAFE_NO_PAD.encode(witness.initial_key.as_bytes());
        document["values"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["state"] == root)
            .unwrap()["value"] = if correct_lower {
            json!({"numerator":"1","denominator":"2"})
        } else {
            json!({"numerator":"1","denominator":"5"})
        };
        let replay = replay_mean_document(&world, witness, &document);
        assert!(
            matches!(
                replay,
                Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                    ProbabilityCertificateRejection {
                        rule: ProbabilityCertificateRule::Malformed,
                        at: fsl_replay::ProbabilityCertificateLocus::Member("kind")
                    }
                ))
            ),
            "wrong deciding bound role: {replay:?}"
        );
        let record = settle_mean_document(&world, name, &document);
        assert!(
            matches!(
                record.terminal().unwrap().value(),
                TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
                    rule: CertificateRule::Malformed,
                    at: CertificateLocus::CertificateMember(fsl_replay::CertificateMember::Kind)
                })
            ),
            "{:?}",
            record.settlement
        );
        assert!(
            record.probabilistic_counterexample.is_none() && record.exact_probabilistic.is_none()
        );
    }
}

#[trace("FR-1043-AC-2", "FR-1043-AC-7", "FR-202-AC-10")]
#[test]
fn residual_measure_paths_replay_one_positive_physical_draw_and_reject_wrong_posts() {
    let claims = claim(
        "Event",
        "every scheduler",
        "probability >= 0.5 [steps from holds(true) until holds(m.delivered) within 1 <= 1]",
    ) + &claim(
        "Quantile",
        "every scheduler",
        "quantile 0.5 of (steps from holds(true) until holds(m.delivered) within 1) <= 1",
    );
    let mut model = retry(&claims);
    model.body = model.body.replace("self.delivered = (lost = 1)", "true");
    let world = World::new(&model);
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    for name in ["Event", "Quantile"] {
        let record = settle(&world, name, ModelCheckLimits::default());
        assert_eq!(
            record.terminal().unwrap().value(),
            &TerminalValue::Refuted,
            "{:?}",
            record.settlement
        );
        let token = record.probabilistic_counterexample.as_ref().unwrap();
        let witness = token.evidence();
        assert!(token.mass() > &ratio(1, 2));
        assert!(witness.paths.iter().all(|path| path.steps.len() == 1
            && path.steps[0].draw.is_some()
            && path.steps[0].transition.is_some()));
        assert!(witness.scheduler.iter().any(|(key, action)| {
            let state: Value = serde_json::from_slice(key.as_bytes()).unwrap();
            !state[7].is_null() && matches!(action, fsl_replay::ProductAction::PostState(_))
        }));
        let replay = |offered: &fsl_replay::ProbabilisticCounterexample| {
            fsl_replay::replay_probabilistic_witness(
                &world.package,
                &system,
                &offered.item,
                offered,
                &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
                &Cancel::new(),
            )
        };
        let baseline = replay(witness);
        assert!(
            matches!(
                baseline,
                Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
            ),
            "{baseline:?}"
        );
        let mut forged = witness.clone();
        forged
            .scheduler
            .iter_mut()
            .find(|(_, action)| matches!(action, fsl_replay::ProductAction::PostState(_)))
            .unwrap()
            .1 = fsl_replay::ProductAction::PostState(witness.initial_key.clone());
        let rejected = replay(&forged);
        assert!(
            !matches!(
                rejected,
                Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
            ),
            "{rejected:?}"
        );
        let first = &witness.paths[0].steps[0];
        let original = system.state_key(&first.post).unwrap().0;
        let alternate = system
            .actions_with_cancel(&witness.initial, &Cancel::new())
            .unwrap()
            .unwrap()
            .into_iter()
            .filter(|action| Some(&action.identity) == first.action.as_ref())
            .flat_map(|action| action.draws)
            .filter(|draw| {
                fsl_eval::simulation::model::same_values(&draw.random, first.draw.as_ref().unwrap())
            })
            .flat_map(|draw| draw.successors)
            .map(|(_, post)| post)
            .find(|post| system.state_key(post).unwrap().0 != original)
            .expect("an actual alternative post-state");
        let mut forged = witness.clone();
        forged.paths[0].steps[0].post = alternate;
        let rejected = replay(&forged);
        assert!(
            !matches!(
                rejected,
                Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
            ),
            "{rejected:?}"
        );
    }
}

#[trace("FR-1043-AC-7", "FR-202-AC-26")]
#[test]
fn a_residual_undefined_measure_keeps_its_pending_policy_and_unadvanced_control() {
    let model = prob_world::tossed_coin("probabilistic Q using ev on M over (c:M::Coin) under every scheduler on origin { quantile 0.5 of (steps from holds(c.v = 0) until holds(c.v = 1) within 2) <= 1 }");
    let world = World::new(&model);
    let record = settle(&world, "Q", ModelCheckLimits::default());
    assert_eq!(
        record.terminal().unwrap().value(),
        &TerminalValue::Refuted,
        "{:?}",
        record.settlement
    );
    let token = record.probabilistic_counterexample.as_ref().unwrap();
    let undefined = token.undefined().unwrap();
    assert_eq!(undefined.evaluation.site.position, 1);
    assert_eq!(undefined.path.paths[0].steps.len(), 1);
    assert_eq!(token.mass(), &ratio(1, 2));
    assert!(undefined
        .path
        .scheduler
        .iter()
        .any(|(_, action)| matches!(action, fsl_replay::ProductAction::PostState(_))));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let replay = |offered: &fsl_replay::ProbabilityUndefinedWitness| {
        fsl_replay::replay_probabilistic_undefined(
            &world.package,
            &system,
            &offered.path.item,
            offered,
            &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
            &Cancel::new(),
        )
    };
    let baseline = replay(undefined);
    assert!(
        matches!(
            baseline,
            Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
        ),
        "{baseline:?}"
    );
    let mut wrong = undefined.clone();
    wrong
        .path
        .scheduler
        .iter_mut()
        .find(|(_, action)| matches!(action, fsl_replay::ProductAction::PostState(_)))
        .unwrap()
        .1 = fsl_replay::ProductAction::PostState(wrong.path.initial_key.clone());
    let rejected = replay(&wrong);
    assert!(
        !matches!(
            rejected,
            Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
        ),
        "{rejected:?}"
    );
}

#[trace("FR-202-AC-30", "FR-1043-AC-9")]
#[test]
fn a_slack_mean_bound_emits_the_checked_policy_and_refuses_the_old_optimizer_policy() {
    let world = World::new(&retry(&claim(
        "Q",
        "every scheduler",
        "mean of fraction holds(m.delivered) over [0,2] >= rational(1,2)",
    )));
    let record = settle(&world, "Q", ModelCheckLimits::default());
    let old = record
        .probabilistic_counterexample
        .as_ref()
        .unwrap()
        .evidence();
    let mut document = mean_bound(old, false, true);
    let root = URL_SAFE_NO_PAD.encode(old.initial_key.as_bytes());
    document["values"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["state"] == root)
        .unwrap()["value"] = json!({"numerator":"49","denominator":"100"});
    let hinted = document.clone();
    document["policy"]
        .as_array_mut()
        .unwrap()
        .retain(|row| row["state"] != root);
    let mismatch = replay_mean_document(&world, old, &document);
    assert!(
        matches!(mismatch, Ok(fsl_replay::ProbabilityWitnessReplay::Parity)),
        "{mismatch:?}"
    );
    let mut empty = old.clone();
    empty.scheduler.clear();
    let selected = replay_mean_document(&world, &empty, &document);
    let Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(selected)) = selected else {
        panic!("{selected:?}");
    };
    let selected_root = selected
        .evidence()
        .scheduler
        .iter()
        .find(|(key, _)| key == &old.initial_key)
        .unwrap();
    let old_root = old
        .scheduler
        .iter()
        .find(|(key, _)| key == &old.initial_key)
        .unwrap();
    assert_ne!(
        selected_root.1, old_root.1,
        "both actions must really satisfy the slack bound"
    );
    let accepted = replay_mean_document(&world, selected.evidence(), &document);
    assert!(
        matches!(
            accepted,
            Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
        ),
        "{accepted:?}"
    );
    let hint_accepted = replay_mean_document(&world, old, &hinted);
    assert!(
        matches!(
            hint_accepted,
            Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
        ),
        "{hint_accepted:?}"
    );
    let policy = selected.evidence().scheduler.clone();
    let bytes = serde_json::to_vec(&document).unwrap();
    let item = fsl_analyze::analyze::exact_probabilistic_item_witness_tampered(
        RequestIndex::new(0),
        world.request("Q", ModelCheckLimits::default()),
        move |witness| {
            witness.scheduler = policy.clone();
            witness.subsystem = Some(Box::new(
                fsl_replay::ProbabilitySubsystemCertificate::from_bytes(
                    &bytes,
                    document_limits(),
                    &Cancel::new(),
                )
                .unwrap(),
            ));
        },
    );
    let result = analyze_items(&Engines::fsl().unwrap(), vec![item], &Cancel::new()).remove(0);
    assert_eq!(
        result.terminal().unwrap().value(),
        &TerminalValue::Refuted,
        "{:?}",
        result.settlement
    );
    let evidence = result.probabilistic_counterexample.as_ref().unwrap();
    assert_eq!(evidence.evidence().scheduler, selected.evidence().scheduler);
    let wire = evidence.wire.as_ref().unwrap();
    let row = wire["scheduler"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["state"]["key"] == root)
        .unwrap();
    assert!(
        row["choice"]["operation"].to_string().contains("fast"),
        "the wire must use the checked selection: {row}"
    );
    let mismatch = settle_mean_document(&world, "Q", &document);
    assert!(
        matches!(
            mismatch.terminal().unwrap().value(),
            TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
        ),
        "{:?}",
        mismatch.settlement
    );
    assert!(mismatch.probabilistic_counterexample.is_none());
}

#[trace("FR-1043-AC-9", "FR-202-AC-29")]
#[test]
fn a_live_mean_bound_checker_limit_or_cancellation_cannot_produce_a_token() {
    let world = World::new(&retry(&claim(
        "Q",
        "Steady",
        "mean of fraction holds(m.delivered) over [0,2] >= rational(1,2)",
    )));
    let record = settle(&world, "Q", ModelCheckLimits::default());
    let mut offered = record
        .probabilistic_counterexample
        .as_ref()
        .unwrap()
        .evidence()
        .clone();
    let document = mean_bound(&offered, false, false);
    offered.subsystem = Some(Box::new(
        fsl_replay::ProbabilitySubsystemCertificate::from_bytes(
            &serde_json::to_vec(&document).unwrap(),
            document_limits(),
            &Cancel::new(),
        )
        .unwrap(),
    ));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let replay = |limits: &fsl_analyze::exact_probabilistic::ExactProbLimits, cancel: &Cancel| {
        fsl_replay::replay_probabilistic_witness(
            &world.package,
            &system,
            &offered.item,
            &offered,
            limits,
            cancel,
        )
    };
    let limits = fsl_analyze::exact_probabilistic::ExactProbLimits::default();
    let accepted = replay(&limits, &Cancel::new());
    assert!(
        matches!(
            accepted,
            Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
        ),
        "valid control: {accepted:?}"
    );
    let mut exhausted = limits;
    exhausted.model.limits.max_states = 1;
    let stopped = replay(&exhausted, &Cancel::new());
    assert!(
        matches!(
            stopped,
            Err(fsl_replay::ProbabilityWitnessError::Limit {
                limit: fsl_replay::ModelCheckLimit::ExactProbMaxStates,
                value: 1
            })
        ),
        "live checking limit: {stopped:?}"
    );
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Deadline);
    let stopped = replay(&limits, &cancel);
    assert!(
        matches!(
            stopped,
            Err(fsl_replay::ProbabilityWitnessError::Graph(
                fsl_replay::ExactProbabilityGraphError::Cancelled(CancelCause::Deadline)
            ))
        ),
        "{stopped:?}"
    );
}

#[trace("FR-1043-AC-11", "FR-202-AC-26")]
#[test]
fn zero_weight_fraction_replays_its_tentative_value_before_an_outside_window_predicate() {
    for scheduler in ["Even", "every scheduler"] {
        let source = format!("probabilistic Q using ev on M over (c:M::Coin) under {scheduler} on origin {{ mean of fraction holds(c.v = 0) over [0,0] weighted by duration >= rational(1,2) }}");
        let mut model = prob_world::coin(&source);
        model.body += "\nreward duration on Coin::drop = 0;\n";
        let world = World::new(&model);
        let record = settle(&world, "Q", ModelCheckLimits::default());
        assert_eq!(
            record.terminal().unwrap().value(),
            &TerminalValue::Refuted,
            "{:?}",
            record.settlement
        );
        let token = record.probabilistic_counterexample.as_ref().unwrap();
        let undefined = token
            .undefined()
            .expect("the value's own undefined evidence");
        assert_eq!(undefined.evaluation.site.position, 1);
        assert_eq!(
            undefined.evaluation.site.locus,
            fsl_replay::UndefinedLocus::FractionValue
        );
        assert_eq!(undefined.path.paths[0].steps.len(), 1);
        assert_eq!(token.mass(), &ratio(1, 2));
        let context: Value = serde_json::from_slice(undefined.product_state.as_bytes()).unwrap();
        assert_eq!(context[5], json!(["fraction", "1", "collecting"]));
        assert_eq!(context[6][0], "weighted-fraction");
        assert_eq!(context[6][3], json!({"numerator":"0","denominator":"1"}));
        assert_eq!(context[6][4], json!({"numerator":"0","denominator":"1"}));
        let wire = token.wire.as_ref().unwrap();
        assert_eq!(wire["evidence"]["undefined"]["cause"], "division-by-zero");
        let system = ModelSystem::new(world.subject().simulation).unwrap();
        let post = &undefined.path.paths[0].steps[0].post;
        let readings = system
            .state_predicates_with_cancel(
                post,
                &[(
                    match &world.claim("Q").form {
                        fsl_semantics::check::ProbForm::MeanOfFraction { fraction, .. } => {
                            &fraction.predicate
                        }
                        _ => unreachable!(),
                    },
                    undefined.path.binding.as_ref(),
                )],
                &Cancel::new(),
            )
            .unwrap();
        assert!(
            matches!(
                readings.as_slice(),
                [fsl_eval::simulation::model::PredicateReading::Undefined { .. }]
            ),
            "the actual outside-window object must really be absent: {readings:?}"
        );
        let replay = |offered: &fsl_replay::ProbabilityUndefinedWitness| {
            fsl_replay::replay_probabilistic_undefined(
                &world.package,
                &system,
                &offered.path.item,
                offered,
                &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
                &Cancel::new(),
            )
        };
        let baseline = replay(undefined);
        assert!(
            matches!(
                baseline,
                Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
            ),
            "{baseline:?}"
        );
        let mut wrong = undefined.clone();
        wrong.evaluation.site.locus =
            fsl_replay::UndefinedLocus::ClaimPredicate(fsl_replay::ClaimPredicate::Target);
        let rejected = replay(&wrong);
        assert!(
            matches!(rejected, Ok(fsl_replay::ProbabilityWitnessReplay::Parity)),
            "{rejected:?}"
        );
        let mut wrong = undefined.clone();
        wrong.evaluation.site.position = 0;
        let rejected = replay(&wrong);
        assert!(
            matches!(rejected, Ok(fsl_replay::ProbabilityWitnessReplay::Parity)),
            "{rejected:?}"
        );
        for forged in [
            {
                let mut key = context.clone();
                key[5] = json!(["fraction", "0", "complete"]);
                key
            },
            {
                let mut key = context.clone();
                key[6][3] = json!({"numerator":"1","denominator":"1"});
                key
            },
        ] {
            let mut wrong = undefined.clone();
            wrong.product_state =
                ProbabilityProductKey::from_canonical_bytes(serde_json::to_vec(&forged).unwrap());
            let rejected = replay(&wrong);
            assert!(
                matches!(rejected, Ok(fsl_replay::ProbabilityWitnessReplay::Parity)),
                "{rejected:?}"
            );
        }
        let mut wrong = undefined.clone();
        wrong.evaluation.cause = fsl_eval::temporal::UndefinedCause::Family(
            filament_semantic_value::catalog::UndefinedRecord {
                reason: filament_semantic_value::catalog::UndefinedReason::AbsentKey,
                fields: std::collections::BTreeMap::new(),
            },
        );
        let rejected = replay(&wrong);
        assert!(
            matches!(rejected, Ok(fsl_replay::ProbabilityWitnessReplay::Parity)),
            "{rejected:?}"
        );
        let mut positive = model;
        positive.body = positive.body.replace(
            "reward duration on Coin::drop = 0",
            "reward duration on Coin::drop = 1",
        );
        let positive = World::new(&positive);
        proof(&positive, "Q", ratio(1, 1));
    }
}

/// A skipped finite object domain does not skip the reward's mandatory
/// typing/definedness and parameter-only-context contracts.
#[trace("FR-185-AC-4")]
#[test]
fn object_parameter_reward_candidates_are_checked_before_runtime() {
    use filament_exact::IllTypedCause;
    use fsl_semantics::check::{CheckCause, Obligation};
    let source = |expression: &str| {
        let mut model = retry(&claim("Q", "Steady", "probability >= 0.5 [accumulate duration from holds(true) until holds(m.delivered) within 2 <= 1]"));
        model
            .operations
            .iter_mut()
            .find(|operation| operation.name == "fast")
            .unwrap()
            .parameters
            .push(("peer", "ix://test/retry/Msg".into()));
        model.body += &format!("\nreward duration on Msg::fast = {expression};\n");
        model
    };
    for (expression, expected) in [
        ("if peer = peer then 1 mod 0 else 0", 0),
        ("if peer.delivered then 1 else 0", 1),
        ("if deref(peer).delivered then 1 else 0", 2),
    ] {
        let Err(refusals) = World::try_new(&source(expression)) else {
            panic!("must refuse before runtime: {expression}");
        };
        assert!(
            refusals.iter().any(|refusal| match expected {
                0 => matches!(refusal.cause, CheckCause::Unproved(Obligation::Nonzero)),
                1 => matches!(
                    refusal.cause,
                    CheckCause::IllTyped(IllTypedCause::TypeMismatch)
                ),
                _ => refusal.cause.cause() == Some("state-read-in-reward"),
            }),
            "{expression}: {refusals:?}"
        );
    }
    let admitted = World::new(&source("if peer = peer then 1 else 0"));
    let system = ModelSystem::new(admitted.subject().simulation).unwrap();
    let initial = system.initial().remove(0);
    let actions = system.actions(&initial).unwrap().unwrap();
    assert!(!actions.is_empty());
    assert!(actions.iter().flat_map(|action| &action.draws).any(|draw| draw.rewards.iter().any(|(reward, value)| reward.as_str() == "duration" && matches!(value, filament_exact::Value::Integer(value) if value == &Integer::one()))));
}

#[trace("TC-3152", "FR-1043-AC-1", "FR-203-AC-12")]
#[test]
fn mixed_initial_quantile_settles_only_activating_initial_one() {
    let model = retry(&claim("Mixed", "Steady", "quantile 0.5 of (steps from holds(not m.delivered) until holds(m.delivered) within 4) <= 2"));
    let world = World::with_initials(
        &model,
        &[json!({"delivered":true}), json!({"delivered":false})],
    );
    let record = settle(&world, "Mixed", ModelCheckLimits::default());
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("{:?}", record.settlement));
    let TerminalValue::Proved {
        basis: fsl_replay::ProofBasis::ExactValue { value, entries, .. },
        ..
    } = terminal.value()
    else {
        panic!("expected checked numeric proof, got {:?}", terminal.value());
    };
    assert_eq!(value.value(), &ratio(3, 4));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].initial, 1);
    let document: Value =
        serde_json::from_slice(terminal.exact_certificate_document().unwrap()).unwrap();
    let accepted = check(&world, "Mixed", &document).unwrap();
    let ExactProbabilityCertificateCheck::Accepted(accepted) = accepted else {
        panic!("{accepted:?}");
    };
    assert_eq!(accepted.value(), &ratio(3, 4));
    // Total certificate coverage still includes the nonactivating initial.
    let mut forged = document;
    for part in ["numerator", "denominator"] {
        let rows = forged["conditional"][part]["values"]
            .as_array_mut()
            .unwrap();
        let old = rows.len();
        rows.retain(|row| key(row)[5][2] != "inactive");
        assert!(
            rows.len() < old,
            "fixture must contain nonactivating control keys"
        );
    }
    let rejected = check(&world, "Mixed", &forged);
    assert!(
        matches!(rejected, Ok(ExactProbabilityCertificateCheck::Rejected(_))),
        "{rejected:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-8", "FR-1043-AC-9", "FR-202-AC-28")]
#[test]
fn terminal_initial_undecided_fraction_synthesizes_stutter_without_a_hint() {
    let claims = claim(
        "Min",
        "every scheduler",
        "mean of fraction holds(false) over [0,2] >= rational(1,2)",
    ) + &claim(
        "Max",
        "every scheduler",
        "mean of fraction holds(false) over [0,2] <= 0",
    );
    let mut model = retry(&claims);
    model.initial = json!({"delivered":true});
    let world = World::new(&model);
    let record = settle(&world, "Min", ModelCheckLimits::default());
    assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
    let witness = record
        .probabilistic_counterexample
        .as_ref()
        .unwrap()
        .evidence();
    let root: Value = serde_json::from_slice(witness.initial_key.as_bytes()).unwrap();
    assert_eq!(root[5], json!(["fraction", "3", "collecting"]));
    let mut upper = mean_bound(witness, false, true);
    upper.as_object_mut().unwrap().remove("policy");
    assert!(upper["values"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["value"] == json!({"numerator":"0","denominator":"1"})));
    let mut no_policy = witness.clone();
    no_policy.scheduler.clear();
    let replay = replay_mean_document(&world, &no_policy, &upper);
    let Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(token)) = replay else {
        panic!("{replay:?}");
    };
    assert!(token
        .evidence()
        .scheduler
        .iter()
        .any(|(key, action)| key == &witness.initial_key
            && matches!(action, fsl_replay::ProductAction::TerminalStutter)));

    // Lower(Maximum)=0 satisfies every local inequality/rank condition. At
    // the admitted threshold zero it cannot strictly refute the claim.
    let mut lower = proof(&world, "Max", ratio(0, 1));
    let base = lower["objective"].take();
    lower["objective"] =
        json!({"profile":"filament.probability-objective/value-bounds/v1", "base":base});
    lower["kind"] = json!("lower");
    lower.as_object_mut().unwrap().remove("policy");
    no_policy.item = fsl_replay::probabilistic_claim_identity(world.claim("Max")).unwrap();
    let rejected = replay_mean_document(&world, &no_policy, &lower);
    assert!(
        matches!(
            rejected,
            Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::ThresholdMissed,
                    ..
                }
            ))
        ),
        "valid local Lowermax0 equality: {rejected:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-9", "FR-202-AC-28")]
#[test]
fn a_partial_lower_expands_only_named_states_and_their_immediate_successors() {
    let world = World::new(&retry(&claim(
        "Q",
        "every scheduler",
        "mean of fraction holds(m.delivered) over [0,2] <= rational(1,4)",
    )));
    let baseline = settle(&world, "Q", ModelCheckLimits::default());
    assert_eq!(
        baseline.terminal().unwrap().value(),
        &TerminalValue::Refuted
    );
    let mut offered = baseline
        .probabilistic_counterexample
        .as_ref()
        .unwrap()
        .evidence()
        .clone();
    let full: Value =
        serde_json::from_slice(offered.subsystem.as_ref().unwrap().as_bytes()).unwrap();
    let mut document = mean_bound(&offered, true, true);
    let root = URL_SAFE_NO_PAD.encode(offered.initial_key.as_bytes());
    document["values"].as_array_mut().unwrap().retain(|row| {
        row["state"] == root || row["value"] == json!({"numerator":"2","denominator":"3"})
    });
    document["values"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["state"] == root)
        .unwrap()["value"] = json!({"numerator":"1","denominator":"3"});
    let named = document["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["state"].clone())
        .collect::<Vec<_>>();
    document["ranking"]
        .as_array_mut()
        .unwrap()
        .retain(|row| named.contains(&row["state"]));
    document.as_object_mut().unwrap().remove("policy");
    let cap = named.len() + 1;
    assert!(
        full["values"].as_array().unwrap().len() > cap,
        "the complete graph must exceed the partial frontier"
    );
    offered.scheduler.clear();
    offered.subsystem = Some(Box::new(
        fsl_replay::ProbabilitySubsystemCertificate::from_bytes(
            &serde_json::to_vec(&document).unwrap(),
            document_limits(),
            &Cancel::new(),
        )
        .unwrap(),
    ));
    let mut limits = fsl_analyze::exact_probabilistic::ExactProbLimits::default();
    limits.model.limits.max_states = cap;
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let accepted = fsl_replay::replay_probabilistic_witness(
        &world.package,
        &system,
        &offered.item,
        &offered,
        &limits,
        &Cancel::new(),
    );
    assert!(
        matches!(
            accepted,
            Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
        ),
        "named plus immediate frontier fits cap{cap}: {accepted:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-7", "FR-203-AC-11")]
#[test]
fn measure_and_vacuity_limits_settle_incomplete_without_certification() {
    let world = World::new(&retry(
        &(claim(
            "Mean",
            "every scheduler",
            "mean of fraction holds(m.delivered) over [0,2] >= 0",
        ) + &claim(
            "Vacuous",
            "Steady",
            "quantile 0.9 of (steps from holds(false) until holds(m.delivered) within 4) <= 2",
        )),
    ));
    let engines = Engines::fsl().unwrap();
    for (name, rational) in [("Mean", true), ("Mean", false), ("Vacuous", false)] {
        let mut request = world.request(name, ModelCheckLimits::default());
        let (limit, value) = if rational {
            request.limits.max_rational_bits = 3;
            (ModelCheckLimit::ExactProbMaxRationalBits, 3)
        } else {
            request.limits.model.limits.max_states = 1;
            (ModelCheckLimit::ExactProbMaxStates, 1)
        };
        let record = analyze_items(
            &engines,
            vec![exact_probabilistic_item(RequestIndex::new(0), request)],
            &Cancel::new(),
        )
        .remove(0);
        let terminal = record
            .terminal()
            .unwrap_or_else(|| panic!("{name}: {:?}", record.settlement));
        assert_eq!(
            terminal.value(),
            &TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, value }),
            "{name}: {:?}",
            record.settlement
        );
        assert!(terminal.exact_certificate_document().is_none());
        assert!(
            record.exact_probabilistic.is_none() && record.probabilistic_counterexample.is_none()
        );
    }
}

#[trace("FR-1043-AC-10", "FR-203-AC-11")]
#[test]
fn vacuity_cancellation_at_the_checker_boundary_retains_its_typed_cause() {
    let world = World::new(&retry(&claim(
        "Q",
        "Steady",
        "quantile 0.9 of (steps from holds(false) until holds(m.delivered) within 4) <= 2",
    )));
    let engines = Engines::fsl().unwrap();
    let run = |cancel: &Cancel| {
        analyze_items(
            &engines,
            vec![exact_probabilistic_item(
                RequestIndex::new(0),
                world.request("Q", ModelCheckLimits::default()),
            )],
            cancel,
        )
        .remove(0)
    };
    let live = Cancel::new();
    let _counting = live.count_charges();
    let control = run(&live);
    assert!(matches!(
        control.terminal().unwrap().value(),
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified
        }
    ));
    let last_poll = live.charges();
    assert!(last_poll > 1);
    for cutoff in [1, last_poll] {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let polls = Arc::new(AtomicU32::new(0));
        let seen = Arc::clone(&polls);
        let cancel = Cancel::observing(move || {
            if seen.fetch_add(1, Ordering::Relaxed) + 1 == cutoff {
                weak.upgrade()
                    .unwrap()
                    .get()
                    .unwrap()
                    .cancel(CancelCause::Deadline);
            }
        });
        slot.set(cancel.clone()).unwrap();
        let stopped = run(&cancel);
        let terminal = stopped
            .terminal()
            .unwrap_or_else(|| panic!("{:?}", stopped.settlement));
        assert_eq!(
            terminal.value(),
            &TerminalValue::Incomplete(IncompleteCause::Cancelled {
                source: CancelCause::Deadline
            }),
            "cutoff{cutoff}: {:?}",
            stopped.settlement
        );
        assert!(terminal.exact_certificate_document().is_none());
        assert!(
            stopped.exact_probabilistic.is_none() && stopped.probabilistic_counterexample.is_none()
        );
    }
}
