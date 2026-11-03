// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-520: the state and compensation expression slots quire-protocol's
//! compensation engine relied on, evaluated through
//! `CheckedPackageEvaluation::evaluate_protocol_slot` over TC-1070's
//! `Refund` and `Partial` templates.

use super::*;

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

// FR-520-AC-11 (per-binder charge accounting with `accounting_version`) has no
// test here: it needs QSL's binder routing onto the quire-exact interval
// accounting API, which is not built yet. Until it is, the matrix shows AC-11
// uncovered rather than covered by a test that does not run.
