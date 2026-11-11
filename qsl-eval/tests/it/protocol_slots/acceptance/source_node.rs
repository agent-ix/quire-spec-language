// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1407 fixture E-S (FR-383-AC-67): TC-1070's fixture E with `Node`
//! declared by the unit's own source. The source route evaluates the
//! protocol's slots from the checked package and gives the value and the
//! charge log fixture E gives.

use super::*;

/// Fixture E-S, checked.
fn source_node_fixture() -> CheckedPackage {
    let (text, document) = tc1070::source_node();
    let bytes = serde_json::to_vec(&document).expect("the document encodes");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("protocol", "acceptance-es"),
        "acceptance-es.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .unwrap();
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).unwrap();
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &qsl_semantics::model::intake::package_input([bytes.as_slice()]).unwrap(),
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .unwrap();
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "acceptance-es".to_owned(),
        unit,
        models,
        Vec::new(),
    )
    .unwrap();
    let pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("E-S checks: {:?}", pending.report(refusals))),
    )
}

/// The completed value and the ordered charge log of `slot` over `values`.
fn evaluated(
    package: &CheckedPackage,
    owner: &str,
    member: &str,
    values: Vec<(&str, Value)>,
) -> (String, Vec<String>, Vec<u64>) {
    let mut meter = Meter::new(UNLIMITED);
    let output = package
        .evaluate_protocol_slot(
            &slot(package, "Refunds", owner, member, None),
            &supply(package, "Refunds", values, None),
            &mut meter,
        )
        .expect("the slot admits its supply");
    let value = format!("{:?}", completed(output));
    let log = meter
        .admitted_charges()
        .iter()
        .map(|charge| format!("{charge:?}"))
        .collect();
    let usage = [
        LimitKind::IntegerBits,
        LimitKind::ValueOccurrences,
        LimitKind::WorkUnits,
        LimitKind::ResultUnits,
    ]
    .map(|kind| meter.consumed(kind))
    .to_vec();
    (value, log, usage)
}

/// E-S checks and its slots give fixture E's values and charge logs: the
/// activation guard, the trigger guard, the retry relation and the recovery
/// predicate of TC-1070's rows.
#[test]
#[trace("FR-383-AC-67", "TC-1407")]
fn the_source_route_evaluates_the_slots_of_a_protocol_over_a_source_node() {
    let control = fixture();
    let source = source_node_fixture();
    let rows = |package: &CheckedPackage| {
        let tick = record(
            package,
            "Tick",
            vec![("ready", Value::Boolean(true)), ("count", int(2))],
        );
        vec![
            evaluated(
                package,
                "Refunds",
                "activation.guard",
                vec![("opened", tick)],
            ),
            evaluated(
                package,
                "Refund",
                "trigger.guard",
                vec![("trigger", flag(package, false)), ("target", int(5))],
            ),
            evaluated(
                package,
                "Refund",
                "retry.relation",
                vec![
                    ("earlier", attempt(package, 0)),
                    ("later", attempt(package, 1)),
                    ("target", int(5)),
                    ("activated", Value::Boolean(false)),
                ],
            ),
            evaluated(
                package,
                "Refund",
                "recovery.predicate",
                vec![
                    ("recovered", receipt(package, 5)),
                    ("target", int(5)),
                    ("activated", Value::Boolean(false)),
                ],
            ),
        ]
    };
    let (control, source) = (rows(&control), rows(&source));
    assert_eq!(
        control
            .iter()
            .map(|(value, ..)| value.as_str())
            .collect::<Vec<_>>(),
        [
            "Boolean(true)",
            "Boolean(true)",
            "Boolean(true)",
            "Boolean(true)"
        ],
        "fixture E's rows"
    );
    assert!(
        control
            .iter()
            .any(|(_, _, usage)| usage.iter().any(|used| *used > 0)),
        "the slot evaluations are metered"
    );
    assert_eq!(source, control, "E-S gives E's values and charge logs");
}

/// TC-1407 S3: `Refund`'s recovery predicate under TC-1072's supply, where
/// `target` is `Unavailable`. The slot halts as unavailable, having read
/// `recovered` and `target` only, and E-S gives fixture E's outcome and
/// charge log.
#[test]
#[trace("FR-569-AC-11", "TC-1407")]
fn an_unavailable_target_halts_the_slot_over_a_source_node_as_it_does_over_a_model() {
    let unavailable = |package: &CheckedPackage| {
        let binders = [
            ("recovered", receipt(package, 5)),
            ("target", int(5)),
            ("activated", Value::Boolean(false)),
        ]
        .into_iter()
        .map(|(name, value)| {
            let supplied = if name == "target" {
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
        let mut meter = Meter::new(UNLIMITED);
        let result = package
            .evaluate_protocol_slot(
                &slot(package, "Refunds", "Refund", "recovery.predicate", None),
                &ProtocolSupply {
                    binders,
                    observation: None,
                },
                &mut meter,
            )
            .expect("an unavailable binder is a slot outcome");
        let ProtocolSlotEvaluation::Unavailable { charges, .. } = &result else {
            panic!("an unavailable target halts the slot: {result:?}");
        };
        let read: Vec<bool> = ["recovered", "target", "activated"]
            .iter()
            .map(|name| charges.binder(&binder(package, "Refunds", name)).is_some())
            .collect();
        let log: Vec<String> = meter
            .admitted_charges()
            .iter()
            .map(|charge| format!("{charge:?}"))
            .collect();
        let usage = [
            LimitKind::IntegerBits,
            LimitKind::ValueOccurrences,
            LimitKind::WorkUnits,
            LimitKind::ResultUnits,
        ]
        .map(|kind| meter.consumed(kind))
        .to_vec();
        (read, log, usage)
    };
    let control = unavailable(&fixture());
    let source = unavailable(&source_node_fixture());
    assert_eq!(
        control.0,
        [true, true, false],
        "the slot read `recovered` and `target`, and stopped before `activated`"
    );
    assert_eq!(source, control, "E-S halts as E does, with E's charge log");
}
