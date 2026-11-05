// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-520: the state and compensation expression slots quire-protocol's
//! compensation engine relied on, evaluated through
//! `CheckedPackageEvaluation::evaluate_protocol_slot` over TC-1070's
//! `Refund` and `Partial` templates.

use super::*;
use qsl_eval::value::{BinderCharges, ChargeAmounts, BINDER_ACCOUNTING_VERSION};
use quire_exact::{Charge, ChargePoint};

const COUNTERS: [LimitKind; 10] = [
    LimitKind::IntegerBits,
    LimitKind::DecimalDigits,
    LimitKind::ScaleExpansion,
    LimitKind::TextInputBytes,
    LimitKind::TextScalars,
    LimitKind::NormalizedScalars,
    LimitKind::UnitEdges,
    LimitKind::ValueOccurrences,
    LimitKind::WorkUnits,
    LimitKind::ResultUnits,
];

fn usage(meter: &Meter) -> [u64; 10] {
    COUNTERS.map(|kind| meter.consumed(kind))
}

fn run(
    package: &CheckedPackage,
    owner: &str,
    member: &str,
    index: Option<usize>,
    values: Vec<(&str, Value)>,
    meter: &mut Meter,
) -> Result<ProtocolSlotEvaluation, CallFailure> {
    package.evaluate_protocol_slot(
        &slot(package, "Refunds", owner, member, index),
        &supply(package, "Refunds", values, None),
        meter,
    )
}

fn boolean(result: ProtocolSlotEvaluation) -> bool {
    match completed(result) {
        Value::Boolean(value) => value,
        other => panic!("a relation slot completes with a Boolean: {other:?}"),
    }
}

fn recovery(package: &CheckedPackage, total: i64, target: i64, activated: bool) -> bool {
    boolean(
        run(
            package,
            "Refund",
            "recovery.predicate",
            None,
            vec![
                ("recovered", receipt(package, total)),
                ("target", int(target)),
                ("activated", Value::Boolean(activated)),
            ],
            &mut Meter::new(UNLIMITED),
        )
        .expect("the recovery slot admits its supply"),
    )
}

fn retry(package: &CheckedPackage, first: i64, second: i64) -> Vec<(&'static str, Value)> {
    vec![
        ("earlier", attempt(package, first)),
        ("later", attempt(package, second)),
        ("target", int(5)),
        ("activated", Value::Boolean(false)),
    ]
}

#[test]
#[trace("FR-520-AC-1", "FR-390-AC-9", "FR-390-AC-10", "TC-1315")]
fn every_outcome_row_has_its_successor_outcome() {
    let package = fixture();
    let mut meter = Meter::new(UNLIMITED);
    for (ready, expected) in [(false, true), (true, false)] {
        let guard = run(
            &package,
            "Refund",
            "trigger.guard",
            None,
            vec![("trigger", flag(&package, ready)), ("target", int(5))],
            &mut meter,
        )
        .expect("the guard admits its supply");
        assert_eq!(boolean(guard), expected, "trigger.guard for ready={ready}");
    }
    for (first, second, expected) in [(0, 1, true), (3, 1, false)] {
        let result = run(
            &package,
            "Refund",
            "retry.relation",
            None,
            retry(&package, first, second),
            &mut meter,
        )
        .expect("the relation admits its supply");
        assert_eq!(boolean(result), expected, "retry.relation {first} {second}");
    }
    assert!(recovery(&package, 5, 5, false), "recovery.predicate true");
    assert!(!recovery(&package, 9, 5, false), "recovery.predicate false");

    let unavailable = run(
        &package,
        "Refund",
        "trigger.guard",
        None,
        Vec::new(),
        &mut meter,
    );
    assert!(
        matches!(unavailable, Err(CallFailure::Admission(_))),
        "a binder the slot reads but the supply omits is refused at admission: {unavailable:?}"
    );

    let halted = package
        .evaluate_protocol_slot(
            &slot(&package, "Refunds", "Refund", "trigger.guard", None),
            &ProtocolSupply {
                binders: vec![
                    (
                        binder(&package, "Refunds", "trigger"),
                        SuppliedValue::Unavailable(CatalogCode::new(
                            "unavailable_observation",
                            "missing-required-artifact",
                        ))
                        .into(),
                    ),
                    (
                        binder(&package, "Refunds", "target"),
                        protocol_value(SuppliedValue::Value(int(5))),
                    ),
                ],
                observation: None,
            },
            &mut meter,
        )
        .expect("an unavailable binder is a slot outcome");
    let ProtocolSlotEvaluation::Unavailable { location, .. } = halted else {
        panic!("an unavailable binder halts the slot: {halted:?}");
    };
    let location = location.expect("an in-process checked read is located");
    assert_eq!(
        format!("{:?}", location.origin),
        "TypeDeclaration { name: \"protocol Refunds compensate Refund trigger-guard 0\" }",
        "the halt names the read inside the trigger guard's own body"
    );

    let wrong_type = run(
        &package,
        "Refund",
        "trigger.guard",
        None,
        vec![("trigger", int(1)), ("target", int(5))],
        &mut Meter::new(UNLIMITED),
    );
    let mut fresh = Meter::new(UNLIMITED);
    let before = fresh.clone();
    let again = run(
        &package,
        "Refund",
        "trigger.guard",
        None,
        vec![("trigger", int(1)), ("target", int(5))],
        &mut fresh,
    );
    assert!(
        matches!(wrong_type, Err(CallFailure::Admission(_))),
        "{wrong_type:?}"
    );
    assert!(matches!(again, Err(CallFailure::Admission(_))), "{again:?}");
    assert_eq!(fresh, before, "a wrong-typed supply charges nothing");

    let mut measured = Meter::new(UNLIMITED);
    let _ = run(
        &package,
        "Refund",
        "retry.relation",
        None,
        retry(&package, 0, 1),
        &mut measured,
    )
    .expect("measuring run");
    let mut denied_any = false;
    for (position, kind) in COUNTERS.into_iter().enumerate() {
        let measured_usage = usage(&measured)[position];
        if measured_usage == 0 {
            continue;
        }
        let mut limits = UNLIMITED;
        set_limit(&mut limits, kind, measured_usage - 1);
        let result = run(
            &package,
            "Refund",
            "retry.relation",
            None,
            retry(&package, 0, 1),
            &mut Meter::new(limits),
        )
        .expect("a resource-incomplete slot is an outcome, not an admission failure");
        assert_incomplete(result, kind);
        denied_any = true;
    }
    assert!(denied_any, "the measured run charged at least one counter");
}

#[test]
#[trace("FR-520-AC-2", "TC-1315")]
fn registration_capture_is_a_pure_function_of_its_supply() {
    let package = fixture();
    let capture = |total: i64| {
        let mut meter = Meter::new(UNLIMITED);
        let value = completed(
            run(
                &package,
                "Refund",
                "registration_captures",
                Some(0),
                vec![("forward", receipt(&package, total))],
                &mut meter,
            )
            .expect("the capture admits its supply"),
        );
        (value, meter)
    };
    let (first, first_meter) = capture(10);
    let (second, second_meter) = capture(10);
    assert_value_eq!(first, int(10));
    assert_value_eq!(second, int(10));
    assert_eq!(
        first_meter, second_meter,
        "equal supply gives equal charges"
    );

    // The later calls read the captured 10 even though the record says 12.
    let (recorded, _) = capture(12);
    assert_value_eq!(recorded, int(12));
    let mut meter = Meter::new(UNLIMITED);
    let retried = run(
        &package,
        "Refund",
        "retry.relation",
        None,
        vec![
            ("earlier", attempt(&package, 0)),
            ("later", attempt(&package, 1)),
            ("target", first.clone()),
            ("activated", Value::Boolean(false)),
        ],
        &mut meter,
    )
    .expect("retry admits the captured value");
    assert!(boolean(retried));
    assert!(recovery(&package, 10, 10, false));
    assert!(!recovery(&package, 12, 10, false));
}

#[test]
#[trace("FR-520-AC-5", "TC-1315")]
fn recovery_requires_the_exact_relation_target_and_achieved_state() {
    let package = fixture();
    assert!(
        recovery(&package, 10, 10, false),
        "the exact triple recovers"
    );
    assert!(
        !recovery(&package, 10, 11, false),
        "a target of 11 does not"
    );
    assert!(
        !recovery(&package, 9, 10, false),
        "another achieved state does not"
    );
    assert!(
        !recovery(&package, 10, 10, true),
        "an activated compensation does not"
    );
    // Another template with the same target and achieved state is its own slot.
    let partial = boolean(
        run(
            &package,
            "Partial",
            "recovery.predicate",
            None,
            vec![
                ("partialRecovered", receipt(&package, 10)),
                ("partialTarget", int(11)),
                ("partialActivated", Value::Boolean(false)),
            ],
            &mut Meter::new(UNLIMITED),
        )
        .expect("the Partial slot admits its supply"),
    );
    assert!(!partial, "Partial reads its own captured target");
    // An operation record with no matching recovered value is not a success.
    let refused = run(
        &package,
        "Refund",
        "recovery.predicate",
        None,
        vec![("target", int(10))],
        &mut Meter::new(UNLIMITED),
    );
    assert!(
        matches!(refused, Err(CallFailure::Admission(_))),
        "a call without the achieved state is refused at admission: {refused:?}"
    );
}

#[test]
#[trace("FR-520-AC-6", "TC-1315")]
fn missing_or_unavailable_recovery_evidence_is_never_true() {
    let package = fixture();
    for omitted in ["recovered", "target", "activated"] {
        let values: Vec<(&str, Value)> = [
            ("recovered", receipt(&package, 10)),
            ("target", int(10)),
            ("activated", Value::Boolean(false)),
        ]
        .into_iter()
        .filter(|(name, _)| *name != omitted)
        .collect();
        let result = run(
            &package,
            "Refund",
            "recovery.predicate",
            None,
            values,
            &mut Meter::new(UNLIMITED),
        );
        assert!(
            matches!(result, Err(CallFailure::Admission(_))),
            "omitting {omitted} is an admission failure: {result:?}"
        );
    }
    let unavailable = package
        .evaluate_protocol_slot(
            &slot(&package, "Refunds", "Refund", "recovery.predicate", None),
            &ProtocolSupply {
                binders: vec![
                    (
                        binder(&package, "Refunds", "recovered"),
                        SuppliedValue::Unavailable(CatalogCode::new(
                            "unavailable_observation",
                            "missing-required-artifact",
                        ))
                        .into(),
                    ),
                    (
                        binder(&package, "Refunds", "target"),
                        protocol_value(SuppliedValue::Value(int(10))),
                    ),
                    (
                        binder(&package, "Refunds", "activated"),
                        protocol_value(SuppliedValue::Value(Value::Boolean(false))),
                    ),
                ],
                observation: None,
            },
            &mut Meter::new(UNLIMITED),
        )
        .expect("an unavailable binder is a slot outcome");
    assert!(
        matches!(unavailable, ProtocolSlotEvaluation::Unavailable { .. }),
        "{unavailable:?}"
    );
}

#[test]
#[trace("FR-520-AC-7", "FR-390-AC-9", "FR-390-AC-10", "TC-1315")]
fn three_slot_calls_on_one_meter_equal_three_on_fresh_meters() {
    let package = fixture();
    let calls = |meter: &mut Meter| {
        let _ = run(
            &package,
            "Refund",
            "registration_captures",
            Some(0),
            vec![("forward", receipt(&package, 5))],
            meter,
        )
        .expect("capture");
        let _ = run(
            &package,
            "Refund",
            "retry.relation",
            None,
            retry(&package, 0, 1),
            meter,
        )
        .expect("retry");
        let _ = run(
            &package,
            "Refund",
            "recovery.predicate",
            None,
            vec![
                ("recovered", receipt(&package, 5)),
                ("target", int(5)),
                ("activated", Value::Boolean(false)),
            ],
            meter,
        )
        .expect("recovery");
    };
    let mut shared = Meter::new(UNLIMITED);
    calls(&mut shared);
    let mut sum = [0_u64; 10];
    let mut highest = [0_u64; 10];
    let singles: [fn(&CheckedPackage, &mut Meter); 3] = [
        |package, meter| {
            let _ = run(
                package,
                "Refund",
                "registration_captures",
                Some(0),
                vec![("forward", receipt(package, 5))],
                meter,
            );
        },
        |package, meter| {
            let _ = run(
                package,
                "Refund",
                "retry.relation",
                None,
                retry(package, 0, 1),
                meter,
            );
        },
        |package, meter| {
            let _ = run(
                package,
                "Refund",
                "recovery.predicate",
                None,
                vec![
                    ("recovered", receipt(package, 5)),
                    ("target", int(5)),
                    ("activated", Value::Boolean(false)),
                ],
                meter,
            );
        },
    ];
    for single in singles {
        let mut fresh = Meter::new(UNLIMITED);
        single(&package, &mut fresh);
        for ((total, top), used) in sum.iter_mut().zip(highest.iter_mut()).zip(usage(&fresh)) {
            *total += used;
            *top = (*top).max(used);
        }
    }
    for (position, kind) in COUNTERS.into_iter().enumerate() {
        let expected = if kind.is_cumulative() {
            sum[position]
        } else {
            highest[position]
        };
        assert_eq!(
            usage(&shared)[position],
            expected,
            "{kind:?}: cumulative counters add, high-water counters keep the largest call"
        );
    }
    assert!(sum.iter().any(|used| *used > 0));
}

#[test]
#[trace("FR-520-AC-8", "TC-1315")]
fn the_evaluation_crates_depend_on_no_io_crate() {
    let cargo = std::env::var("CARGO").expect("cargo sets CARGO for tests");
    let output = std::process::Command::new(cargo)
        .args([
            "tree",
            "--locked",
            "--offline",
            "-e",
            "normal",
            "--prefix",
            "none",
            "-p",
            "qsl-eval",
            "-p",
            "qsl-package",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8(output.stdout).expect("utf-8 tree");
    for name in [
        "tokio", "reqwest", "hyper", "sqlx", "rusqlite", "postgres", "mio", "socket2", "openssl",
        "rustls",
    ] {
        assert!(
            !tree
                .lines()
                .any(|line| line.split(' ').next() == Some(name)),
            "{name} is in the evaluation dependency tree"
        );
    }
}

#[test]
#[trace("FR-520-AC-10", "TC-1315")]
fn the_retired_state_module_and_its_exports_do_not_exist() {
    // Other modules declare their own `evaluate_v2` (temporal result
    // authentication) and `ACCOUNTING_VERSION` (composed binding work); the
    // retired items are the ones of the deleted `state` module.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    assert!(!root.join("src/state.rs").exists() && !root.join("src/state").exists());
    let lib = std::fs::read_to_string(root.join("src/lib.rs")).expect("lib.rs reads");
    assert!(
        !lib.contains("mod state;"),
        "the root crate declares no state module"
    );
    let mut stack = vec![root];
    let mut scanned = 0_usize;
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("directory reads") {
            let path = entry.expect("entry reads").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if path.is_dir() {
                if !matches!(name, "target" | ".git" | "fuzz") {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && path.components().any(|c| c.as_os_str() == "src")
                && name != "state_compensation.rs"
            {
                let text = std::fs::read_to_string(&path).expect("source reads");
                scanned += 1;
                for retired in ["StateView", "EvaluationReport"] {
                    assert!(
                        !text.contains(retired),
                        "{} names {retired}",
                        path.display()
                    );
                }
                for path_form in ["state::evaluate_v2", "state::ACCOUNTING_VERSION"] {
                    assert!(
                        !text.contains(path_form),
                        "{} names {path_form}",
                        path.display()
                    );
                }
            }
        }
    }
    assert!(
        scanned > 50,
        "the scan covered the workspace sources: {scanned}"
    );
}

fn doc_blocks(text: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut open: Option<(String, String)> = None;
    for line in text.lines() {
        let Some(doc) = line.trim_start().strip_prefix("///") else {
            continue;
        };
        let doc = doc.trim();
        match (&mut open, doc.strip_prefix("```")) {
            (None, Some(info)) => open = Some((info.to_owned(), String::new())),
            (Some(_), Some(_)) => blocks.extend(open.take()),
            (Some((_, body)), None) => {
                body.push_str(doc);
                body.push('\n');
            }
            (None, None) => {}
        }
    }
    blocks
}

#[test]
#[trace("FR-520-AC-12", "TC-1315")]
fn no_boolean_coercion_is_pinned_by_compile_fail_doctests_with_a_compiling_control() {
    // Stable rustdoc does not enforce the error code in `compile_fail,EXXXX`, so
    // this test pins that the doctests exist with the intended code and body;
    // the doctests themselves pin only that each coercion fails to compile.
    let path = format!(
        "{}/src/value/expression/protocol_slot.rs",
        env!("CARGO_MANIFEST_DIR")
    );
    let blocks = doc_blocks(&std::fs::read_to_string(&path).expect("protocol_slot.rs reads"));
    for (info, convert, ty) in [
        (
            "compile_fail,E0599",
            "result.as_bool()",
            "qsl_eval::value::ProtocolSlotEvaluation",
        ),
        (
            "compile_fail,E0599",
            "result.as_bool()",
            "quire_eval::Evaluation",
        ),
        (
            "compile_fail,E0277",
            "result.into()",
            "qsl_eval::value::ProtocolSlotEvaluation",
        ),
        (
            "compile_fail,E0277",
            "result.into()",
            "quire_eval::Evaluation",
        ),
    ] {
        assert!(
            blocks.iter().any(|(block, body)| block == info
                && body.contains(convert)
                && body.contains(&format!("result: {ty}"))),
            "no `{info}` doctest for `{convert}` on {ty}"
        );
    }
    assert!(
        blocks.iter().any(|(info, body)| info == "no_run"
            && body.contains("ProtocolSlotEvaluation::Unavailable")
            && body.contains("quire_eval::Evaluation")),
        "the compiling control is missing"
    );
}

#[test]
#[trace("FR-520-AC-13", "TC-1315")]
fn no_evaluation_result_is_coerced_with_unwrap_or_false() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let mut stack = vec![
        std::path::PathBuf::from(format!("{manifest}/src")),
        std::path::PathBuf::from(format!("{manifest}/../qsl-package/src")),
    ];
    let mut scanned = 0_usize;
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("directory reads") {
            let path = entry.expect("entry reads").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            scanned += 1;
            let text = std::fs::read_to_string(&path).expect("source reads");
            let lines: Vec<&str> = text.lines().collect();
            for (index, line) in lines.iter().enumerate() {
                if !line.contains("unwrap_or(false)") {
                    continue;
                }
                // Other uses (bit-vector and map lookups) are not evaluation
                // results; the few lines before the call are the statement.
                let statement = lines[index.saturating_sub(4)..=index].join("\n");
                for word in [
                    "Evaluation",
                    "evaluate_protocol_slot",
                    "Outcome::Completed",
                    "FamilyOutcome",
                ] {
                    assert!(
                        !statement.contains(word),
                        "{}:{} coerces an evaluation result with unwrap_or(false)",
                        path.display(),
                        index + 1
                    );
                }
            }
        }
    }
    assert!(
        scanned > 20,
        "the scan covered the evaluation crates: {scanned}"
    );
}

fn charges_of(result: &ProtocolSlotEvaluation) -> &BinderCharges {
    match result {
        ProtocolSlotEvaluation::Evaluated(_, charges)
        | ProtocolSlotEvaluation::Unavailable { charges, .. } => charges,
    }
}

fn amounts(charges: &ChargeAmounts) -> [u64; 10] {
    COUNTERS.map(|kind| charges.amount(kind))
}

fn recovery_supply(package: &CheckedPackage, unavailable: Option<&str>) -> ProtocolSupply<'static> {
    let binders = [
        ("recovered", receipt(package, 5)),
        ("target", int(5)),
        ("activated", Value::Boolean(false)),
    ]
    .into_iter()
    .map(|(name, value)| {
        let supplied = if Some(name) == unavailable {
            SuppliedValue::Unavailable(CatalogCode::new(
                "unavailable_observation",
                "missing-required-artifact",
            ))
        } else {
            SuppliedValue::Value(value)
        };
        (binder(package, "Refunds", name), supplied.into())
    })
    .collect();
    ProtocolSupply {
        binders,
        observation: None,
    }
}

fn charge_log(
    package: &CheckedPackage,
    supply: &ProtocolSupply<'_>,
) -> (ProtocolSlotEvaluation, [u64; 10]) {
    let mut meter = Meter::new(UNLIMITED);
    let result = package
        .evaluate_protocol_slot(
            &slot(package, "Refunds", "Refund", "recovery.predicate", None),
            supply,
            &mut meter,
        )
        .expect("the recovery slot evaluates");
    (result, usage(&meter))
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn a_slot_call_logs_each_binder_it_read_with_the_accounting_version() {
    let package = fixture();
    let supply = recovery_supply(&package, Some("target"));
    let (result, used) = charge_log(&package, &supply);
    assert!(
        matches!(result, ProtocolSlotEvaluation::Unavailable { .. }),
        "{result:?}"
    );
    let charges = charges_of(&result);
    assert_eq!(charges.accounting_version(), BINDER_ACCOUNTING_VERSION);
    let read = |name: &str| charges.binder(&binder(&package, "Refunds", name));
    assert!(read("recovered").is_some(), "the first read has an entry");
    assert!(
        read("target").is_some(),
        "the unavailable binder was read and has an entry"
    );
    assert!(
        read("activated").is_none(),
        "the evaluation stopped before it read `activated`"
    );
    assert_eq!(charges.binders().count(), 2);
    assert_eq!(amounts(&charges.total()), used);
    let (again, _) = charge_log(&package, &supply);
    assert_eq!(
        charges_of(&again),
        charges,
        "equal supplies give equal logs"
    );
}

const NOTHING: [u64; 10] = [0; 10];

/// The TC-1070 package with its source rewritten by `edit`.
fn variant(edit: impl Fn(String) -> String) -> CheckedPackage {
    fixture_from_source(&edit(SOURCE.to_owned())).0
}

/// The Refund slot `recovery.predicate` over `recovered`, `target` and
/// `activated` with `body` as its expression, beside the pure functions
/// `plus` and `add`.
fn recovery_variant(body: &str) -> CheckedPackage {
    let original =
        "recover (recovered: M::Receipt) { recovered.total = target and not activated };";
    variant(|source| {
        source
            .replace(
                "profile S =",
                "profile v = \"quire.value.complete/v1\";\nprofile S =",
            )
            .replace(
                original,
                &format!("recover (recovered: M::Receipt) {{ {body} }};"),
            )
            + "function plus using v(a: Integer): Integer pure { a + 1 }\n\
               function add using v(a: Integer, b: Integer): Integer pure { a + b }\n\
               function bump using v(a: Integer): Integer pure { let x = a + 1 in x + 1 }\n\
               function three using v(n: Integer): Sequence<Integer>[0, 16] pure { sequence[n, n, n] }\n"
    })
}

fn recovery_log(package: &CheckedPackage) -> (ProtocolSlotEvaluation, [u64; 10]) {
    charge_log(package, &recovery_supply(package, None))
}

fn entry(package: &CheckedPackage, charges: &BinderCharges, name: &str) -> [u64; 10] {
    amounts(
        charges
            .binder(&binder(package, "Refunds", name))
            .expect("every binder the expression reads has an entry"),
    )
}

// Counter order: [integer_bits, decimal_digits, scale_expansion,
// text_input_bytes, text_scalars, normalized_scalars, unit_edges,
// value_occurrences, work_units, result_units].

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn an_operation_over_two_binders_is_unattributed_and_a_later_single_binder_operation_is_its_own() {
    let package = fixture();
    let (result, used) = recovery_log(&package);
    let charges = charges_of(&result);
    assert_eq!(entry(&package, charges, "recovered"), NOTHING);
    assert_eq!(entry(&package, charges, "target"), NOTHING);
    assert_eq!(
        entry(&package, charges, "activated"),
        [0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
        "`not activated` reads one binder after others were read; the `and` over two binders is not its"
    );
    assert_eq!(
        amounts(charges.unattributed()),
        [0, 0, 0, 0, 0, 0, 0, 1, 6, 2],
        "`recovered.total = target` reads two binders, and the `and` joins two binders"
    );
    assert_eq!(
        amounts(&charges.total()),
        used,
        "work and results sum, the eight high-water counters take the maximum"
    );
    let (again, _) = recovery_log(&package);
    assert_eq!(
        charges_of(&again),
        charges,
        "equal supplies give equal logs"
    );
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn operations_over_one_binder_each_are_charged_to_that_binder() {
    let package = recovery_variant("recovered.total + 1 = target + 1 and not activated");
    let (result, used) = recovery_log(&package);
    let charges = charges_of(&result);
    let one_operand = [4, 0, 0, 0, 0, 0, 0, 2, 3, 1];
    assert_eq!(entry(&package, charges, "recovered"), one_operand);
    assert_eq!(entry(&package, charges, "target"), one_operand);
    assert_eq!(
        entry(&package, charges, "activated"),
        [0, 0, 0, 0, 0, 0, 0, 0, 1, 1]
    );
    assert_eq!(
        amounts(charges.unattributed()),
        [0, 0, 0, 0, 0, 0, 0, 1, 6, 2],
        "only the equality over both sums and the `and` joining two binders are unattributed"
    );
    assert_eq!(amounts(&charges.total()), used);
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn a_literal_only_subexpression_is_unattributed_wherever_it_runs() {
    let package = recovery_variant("(1 + 2 = 3) and recovered.total = target and not activated");
    let (result, used) = recovery_log(&package);
    let charges = charges_of(&result);
    let base = fixture();
    let (base_result, _) = recovery_log(&base);
    for name in ["recovered", "target", "activated"] {
        assert_eq!(
            entry(&package, charges, name),
            entry(&base, charges_of(&base_result), name),
            "{name}: the literal prefix adds nothing to a binder"
        );
    }
    assert_eq!(
        amounts(charges.unattributed()),
        [3, 0, 0, 0, 0, 0, 0, 2, 15, 5],
        "the literal prefix runs before the first binder read"
    );
    assert_eq!(amounts(&charges.total()), used);
    let late = recovery_variant("recovered.total = target and (1 + 2 = 3) and not activated");
    let (late_result, late_used) = recovery_log(&late);
    let late_charges = charges_of(&late_result);
    assert_eq!(
        entry(&late, late_charges, "activated"),
        [0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
        "a literal-only subexpression after a binder read is not that binder's"
    );
    assert_eq!(amounts(&late_charges.total()), late_used);
    assert!(
        late_charges.unattributed().amount(LimitKind::WorkUnits)
            > charges_of(&base_result)
                .unattributed()
                .amount(LimitKind::WorkUnits),
        "the literal subexpression's work is unattributed"
    );
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn a_connective_over_two_binders_is_unattributed_and_leaves_the_right_binder_its_own_operation() {
    let (alone, alone_result, _) = recovery_log_of("not activated");
    let (joined, joined_result, used) = recovery_log_of("target = 5 and not activated");
    let charges = charges_of(&joined_result);
    assert_eq!(
        entry(&joined, charges, "activated"),
        entry(&alone, charges_of(&alone_result), "activated"),
        "`and` over `target` and `activated` is made by neither"
    );
    assert_eq!(
        entry(&joined, charges, "activated"),
        [0, 0, 0, 0, 0, 0, 0, 0, 1, 1]
    );
    assert!(charges.unattributed().amount(LimitKind::WorkUnits) >= 1);
    assert_eq!(amounts(&charges.total()), used);
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn every_element_of_a_query_over_one_binder_is_charged_to_that_binder() {
    let (with_double, double_result, used) =
        recovery_log_of("forall(x in three(target): x * 2 + recovered.total > 0)");
    let (without, plain_result, _) =
        recovery_log_of("forall(x in three(target): x + recovered.total > 0)");
    let double = charges_of(&double_result);
    let plain = charges_of(&plain_result);
    let work =
        |charges: &BinderCharges, package: &CheckedPackage| entry(package, charges, "target")[8];
    let elements = 3;
    assert!(
        work(double, &with_double) >= work(plain, &without) + elements,
        "`x * 2` over each of the {elements} elements is made by `target`, not only the first"
    );
    for kind in [LimitKind::WorkUnits, LimitKind::ResultUnits] {
        assert_eq!(
            double.unattributed().amount(kind),
            plain.unattributed().amount(kind),
            "no element's `x * 2` falls to unattributed after a result over `recovered`"
        );
    }
    assert_eq!(amounts(&double.total()), used);
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn commuting_the_operands_of_a_two_binder_operation_leaves_the_log_unchanged() {
    let forward = recovery_variant("recovered.total + target = 10 and not activated");
    let backward = recovery_variant("target + recovered.total = 10 and not activated");
    let (forward_result, used) = recovery_log(&forward);
    let (backward_result, _) = recovery_log(&backward);
    let charges = charges_of(&forward_result);
    assert_eq!(charges, charges_of(&backward_result));
    assert_eq!(
        amounts(charges.unattributed()),
        [4, 0, 0, 0, 0, 0, 0, 2, 9, 3]
    );
    assert_eq!(amounts(&charges.total()), used);
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn a_call_made_before_any_read_registers_no_binder_in_the_callee_frame() {
    let package = recovery_variant("plus(1) = 3 and recovered.total = target and not activated");
    let (result, used) = recovery_log(&package);
    let charges = charges_of(&result);
    assert_eq!(
        charges.binders().count(),
        0,
        "the callee's parameter slot is no binder read, and the short-circuit reads none"
    );
    assert_eq!(
        amounts(charges.unattributed()),
        [2, 0, 0, 0, 0, 0, 0, 2, 11, 4]
    );
    assert_eq!(amounts(&charges.total()), used);
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn a_high_water_counter_is_the_largest_binder_entry_not_the_sum() {
    let package = fixture();
    let mut meter = Meter::new(UNLIMITED);
    let result = run(
        &package,
        "Refund",
        "retry.relation",
        None,
        retry(&package, 0, 1),
        &mut meter,
    )
    .expect("retry");
    let charges = charges_of(&result);
    assert!(charges.binders().count() > 1);
    for kind in COUNTERS {
        let entries = charges
            .binders()
            .map(|(_, held)| held.amount(kind))
            .chain([charges.unattributed().amount(kind)]);
        let expected = if kind.is_cumulative() {
            entries.sum::<u64>()
        } else {
            entries.max().unwrap_or(0)
        };
        assert_eq!(charges.total().amount(kind), expected, "{kind:?}");
        assert_eq!(
            charges.total().amount(kind),
            meter.consumed(kind),
            "{kind:?}"
        );
    }
    assert!(
        COUNTERS
            .iter()
            .any(|kind| !kind.is_cumulative() && charges.total().amount(*kind) > 0),
        "a high-water counter was charged"
    );
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn a_call_restores_the_meters_capture_state_and_drains_a_caller_interval_it_found() {
    let package = fixture();
    let supply = recovery_supply(&package, None);
    let slot = slot(&package, "Refunds", "Refund", "recovery.predicate", None);
    let mut off = Meter::new(UNLIMITED);
    package
        .evaluate_protocol_slot(&slot, &supply, &mut off)
        .expect("evaluates");
    assert!(
        off.take_charge_accounting().is_none(),
        "capture stays disabled for a caller that never enabled it"
    );
    let mut on = Meter::new(UNLIMITED);
    on.enable_charge_accounting();
    on.charge(Charge::new(ChargePoint::CollectionVisit))
        .expect("the caller's own charge");
    let before = on.consumed(LimitKind::WorkUnits);
    assert!(before > 0);
    let result = package
        .evaluate_protocol_slot(&slot, &supply, &mut on)
        .expect("evaluates");
    assert_eq!(
        charges_of(&result).total().amount(LimitKind::WorkUnits),
        on.consumed(LimitKind::WorkUnits) - before,
        "the returned log is the call's own total, without the caller's interval"
    );
    let drained = on
        .take_charge_accounting()
        .expect("capture stays enabled for a caller that enabled it");
    assert_eq!(
        COUNTERS.map(|kind| drained.amount(kind)),
        NOTHING,
        "the caller's interval before the call and the call's own charges are not in it"
    );
}

/// The log of the Refund `recovery.predicate` slot with `body` as its
/// expression, supplying only the binders `body` names.
fn recovery_log_of(body: &str) -> (CheckedPackage, ProtocolSlotEvaluation, [u64; 10]) {
    let package = recovery_variant(body);
    let values = [
        ("recovered", receipt(&package, 5)),
        ("target", int(5)),
        ("activated", Value::Boolean(false)),
    ]
    .into_iter()
    .filter(|(name, _)| body.contains(name))
    .collect();
    let mut meter = Meter::new(UNLIMITED);
    let result = run(
        &package,
        "Refund",
        "recovery.predicate",
        None,
        values,
        &mut meter,
    )
    .expect("the recovery slot evaluates");
    (package, result, usage(&meter))
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn a_one_argument_call_charges_its_body_to_the_binder_of_its_argument() {
    let (package, result, used) = recovery_log_of("plus(target) = 6 and not activated");
    let charges = charges_of(&result);
    assert_eq!(
        entry(&package, charges, "target"),
        [4, 0, 0, 0, 0, 0, 0, 2, 9, 2]
    );
    assert_eq!(
        amounts(charges.unattributed()),
        [0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
        "only the `and` over `target` and `activated`"
    );
    assert_eq!(amounts(&charges.total()), used);
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn a_one_argument_call_after_a_two_argument_call_takes_its_own_argument_owner() {
    let (package, result, used) =
        recovery_log_of("add(target, 1) = 6 and plus(recovered.total) = 6 and not activated");
    let charges = charges_of(&result);
    assert_eq!(
        entry(&package, charges, "target"),
        [4, 0, 0, 0, 0, 0, 0, 2, 9, 2],
        "`add(target, 1)` and its body belong to `target` and nothing after it does"
    );
    assert_eq!(
        entry(&package, charges, "recovered"),
        [4, 0, 0, 0, 0, 0, 0, 2, 9, 2],
        "`plus(recovered.total)` and its body belong to `recovered`"
    );
    assert_eq!(
        amounts(charges.unattributed()),
        [0, 0, 0, 0, 0, 0, 0, 0, 2, 2],
        "the two `and`s each join operands of different binders"
    );
    assert_eq!(amounts(&charges.total()), used);
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn a_call_over_two_binders_charges_its_body_to_no_binder() {
    let (package, result, used) =
        recovery_log_of("add(recovered.total, target) = 10 and not activated");
    let charges = charges_of(&result);
    assert_eq!(entry(&package, charges, "recovered"), NOTHING);
    assert_eq!(entry(&package, charges, "target"), NOTHING);
    assert_eq!(
        amounts(charges.unattributed()),
        [4, 0, 0, 0, 0, 0, 0, 2, 10, 3]
    );
    assert_eq!(amounts(&charges.total()), used);
}

#[test]
#[trace("FR-520-AC-11", "TC-1315")]
fn a_let_bound_value_keeps_the_binder_it_derives_from() {
    let (package, result, used) = recovery_log_of("bump(target) = 7 and not activated");
    let charges = charges_of(&result);
    assert_eq!(
        entry(&package, charges, "target"),
        [4, 0, 0, 0, 0, 0, 0, 2, 12, 3],
        "`x + 1` over the let-bound `x = a + 1` is made by `target`"
    );
    assert_eq!(
        amounts(charges.unattributed()),
        [0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
        "only the `and` over `target` and `activated`"
    );
    assert_eq!(amounts(&charges.total()), used);
}

/// The emitted compensation node whose `forward_binder` is the parameter the
/// template declared under `forward`: the template's identity read from the
/// graph itself, never from a position in any list.
fn template_node(package: &CheckedPackage, forward: &str) -> NodeKey {
    use qsl_semantics::check::{BindingValue, BodyTerm, LeafTerm, MemberTerm};
    let keys = package.graph().protocol_node_keys("Refunds").unwrap();
    let binder = binder(package, "Refunds", forward);
    let mut owners = keys.compensations.values().filter(|key| {
        let node = package
            .graph()
            .semantic_graph()
            .node((*key).clone())
            .unwrap();
        let BodyTerm::Application(application) = node.body() else {
            return false;
        };
        application.arguments.iter().any(|argument| match argument {
            MemberTerm::Binding(binding) if binding.name == "forward_binder" => matches!(
                &binding.value,
                BindingValue::Leaf(LeafTerm::Reference { target })
                    if target.admitted() == Some(&binder)
            ),
            _ => false,
        })
    });
    let owner = owners.next().expect("one template declares this forward");
    assert!(owners.next().is_none(), "exactly one template owns it");
    owner.clone()
}

/// QSL-1705: a compensation node selects its own template's slots even when
/// the declaration order differs from the node-key order. `Refund` is
/// declared first and `Partial` second; each node found by graph identity
/// evaluates only its own template's binders.
#[test]
#[trace("FR-390-AC-9", "TC-1315")]
fn each_compensation_node_selects_its_own_template_slots() {
    let package = fixture();
    let checked = package.graph().protocol("Refunds").unwrap();
    let declared: Vec<&str> = checked
        .obligations
        .compensations
        .iter()
        .map(|template| template.name.as_str())
        .collect();
    assert_eq!(declared, ["Refund", "Partial"], "source declaration order");
    let refund = template_node(&package, "forward");
    let partial = template_node(&package, "partialForward");
    assert_ne!(refund, partial);
    assert!(
        partial < refund,
        "the fixture's node-key order differs from its declaration order"
    );
    let evaluate = |node: &NodeKey, values: Vec<(&str, Value)>| {
        boolean(
            package
                .evaluate_protocol_slot(
                    &ProtocolSlot {
                        node: node.clone(),
                        member: "recovery.predicate".to_owned(),
                        index: None,
                    },
                    &supply(&package, "Refunds", values, None),
                    &mut Meter::new(UNLIMITED),
                )
                .expect("a template's own slot admits its own binders"),
        )
    };
    let partial_values = |target| {
        vec![
            ("partialRecovered", receipt(&package, 10)),
            ("partialTarget", int(target)),
            ("partialActivated", Value::Boolean(false)),
        ]
    };
    assert!(evaluate(&partial, partial_values(10)), "Partial recovers");
    assert!(!evaluate(&partial, partial_values(11)), "Partial target 11");
    let refund_values = |target| {
        vec![
            ("recovered", receipt(&package, 10)),
            ("target", int(target)),
            ("activated", Value::Boolean(false)),
        ]
    };
    assert!(evaluate(&refund, refund_values(10)), "Refund recovers");
    assert!(!evaluate(&refund, refund_values(11)), "Refund target 11");
}
