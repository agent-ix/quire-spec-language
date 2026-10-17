// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-321 (TC-830): a witness union value converts to the kernel union value
//! its slot's union declares, round-trips through the witness value text, and
//! is bounded by the request's `value_occurrences` at any depth.
//!
//! The conversion's adversarial and deep cases are reached over the sealed
//! fixture unions (`qsl_semantics::check::union_fixture`) through
//! `convert_arguments_in`; `a_union_input_evaluates_a_checked_case_through_replay`
//! runs a union from source, through the spine and `replay`.

use std::cmp::Ordering;

use ix_trace_rs::trace;
use qsl_foundation::digest::WireNodeId;
use qsl_semantics::check::union_fixture as fixture;
use qsl_semantics::value::supplied::{SuppliedFailure, SuppliedLimitKind, SuppliedPhase};
use quire_exact::{compare_keys, ScalarLimits, Value, ValueType};

use super::{
    default_run, name, parameter, refused_parameter, replay, request, spine, typed_input,
    EvaluatedValue, ReplayLimits, ReplayResult, WitnessValue, UNLIMITED,
};
use crate::execute::argument::{convert_arguments_in, ReferenceModel, Stopped};

/// An integer inside a value text, which reads back as an exact integer.
fn int(number: i64) -> WitnessValue {
    WitnessValue::ExactInteger(quire_exact::Integer::from(number))
}

fn union(name: &str, member: &str, components: Vec<WitnessValue>) -> WitnessValue {
    WitnessValue::Union {
        declaration: WireNodeId::from_digest(*fixture::settled(name).as_bytes()),
        member: member.to_owned(),
        components,
    }
}

/// A `Tree` of `depth` levels, matching `fixture::tree(depth)`.
fn tree(depth: usize) -> WitnessValue {
    let mut tree = union("Tree", "Leaf", vec![int(1)]);
    for _ in 1..depth {
        tree = union(
            "Tree",
            "Branch",
            vec![int(1), WitnessValue::Option(Some(Box::new(tree)))],
        );
    }
    tree
}

fn convert(
    declared: &ValueType,
    value: &WitnessValue,
    limits: &ScalarLimits,
) -> Result<Value, Stopped> {
    let types = fixture::types(quire_exact::EffectiveId::from_digest([9; 32]));
    convert_arguments_in(
        &types,
        Vec::new(),
        std::slice::from_ref(declared),
        std::slice::from_ref(value),
        limits,
        &mut default_run(),
        &ReferenceModel::unselected(),
    )
    .map(|mut converted| converted.values.remove(0))
}

fn equal(left: &Value, right: &Value) -> bool {
    compare_keys(left, right) == Some(Ordering::Equal)
}

/// Run `body` on a thread with a 512 KiB stack.
fn on_small_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(body)
        .unwrap()
        .join()
        .unwrap();
}

/// FR-321-AC-3: `Shape::Empty`, `Shape::Circle(4)` and a depth-3 `Tree` each
/// convert from their witness union spelling to an equal kernel value, and
/// through the witness value text and back to the same witness and value.
#[trace("FR-321-AC-3", "TC-830")]
#[test]
fn union_values_round_trip_through_their_witness_spelling() {
    let cases = [
        (
            "Shape",
            union("Shape", "Empty", vec![]),
            fixture::value("Shape", "Empty", vec![]),
        ),
        (
            "Shape",
            union("Shape", "Circle", vec![int(4)]),
            fixture::value("Shape", "Circle", vec![Value::Integer(4_i64.into())]),
        ),
        ("Tree", tree(3), fixture::tree(3)),
    ];
    for (name, witness, expected) in cases {
        let declared = fixture::union_type(name);
        let kernel = convert(&declared, &witness, &UNLIMITED).expect("a member of its union");
        assert!(equal(&kernel, &expected), "{witness:?}");
        let text = witness.to_value_text().expect("a union has a value text");
        let again = WitnessValue::from_value_text(&text).expect("the text reads back");
        assert_eq!(again, witness);
        let kernel_again = convert(&declared, &again, &UNLIMITED).expect("converts again");
        assert!(equal(&kernel_again, &expected));
    }
}

/// FR-321: a witness union that is not a well-formed value of the slot's
/// union refuses `WrongValueKind` naming the parameter.
#[trace("FR-321-AC-1", "TC-829")]
#[test]
fn a_witness_union_that_is_not_a_member_of_its_slots_union_is_refused() {
    let shape = fixture::union_type("Shape");
    let refused = [
        (
            "another union's declaration",
            union("Other", "Only", vec![int(1)]),
        ),
        (
            "a member the union does not declare",
            union("Shape", "Square", vec![int(2)]),
        ),
        ("a short payload", union("Shape", "Rect", vec![int(2)])),
        (
            "a payload of the wrong kind",
            union("Shape", "Circle", vec![WitnessValue::Boolean(true)]),
        ),
    ];
    for (case, witness) in refused {
        let result = convert(&shape, &witness, &UNLIMITED);
        assert_eq!(
            result.as_ref().err().and_then(refused_parameter),
            Some(0),
            "{case}: {result:?}"
        );
    }
}

/// FR-321-AC-4: a `Tree` 10,000 levels deep converts, on a small stack, to
/// the kernel value that equals itself; a `value_occurrences` limit one below
/// its 29,999 occurrences stops the conversion naming that limit.
#[trace("FR-321-AC-4", "TC-830")]
#[test]
fn a_ten_thousand_level_witness_tree_converts_and_is_bounded_by_occurrences() {
    on_small_stack(|| {
        let declared = fixture::union_type("Tree");
        let witness = tree(10_000);
        let kernel = convert(&declared, &witness, &UNLIMITED).expect("any depth converts");
        assert!(equal(&kernel, &fixture::tree(10_000)));
        assert!(equal(&kernel, &kernel.clone()));

        let limits = |value_occurrences| ScalarLimits {
            value_occurrences,
            ..UNLIMITED
        };
        assert!(convert(&declared, &witness, &limits(29_999)).is_ok());
        let Err(Stopped::Supplied(SuppliedFailure::Stop(stop))) =
            convert(&declared, &witness, &limits(29_998))
        else {
            panic!("a supplied-value stop");
        };
        assert_eq!(stop.context.phase, SuppliedPhase::Conversion);
        assert_eq!(stop.context.parameter, Some(0));
        let limit = stop.limit().expect("a limit stop");
        assert_eq!(limit.limit_kind, SuppliedLimitKind::ValueOccurrences);
        assert_eq!(limit.limit, 29_998);
        assert_eq!(limit.next_charge, 29_999);
    });
}

const SHAPE_UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n\
    function big using v(s: Shape): Boolean pure \
    { case s { Empty: false; Circle(r): r > 3; Rect(w, h): w * h > 5; } }\n";

/// A union value supplied to a function compiled from source: the witness
/// names the union by the node id the emitted package gives it, the sealed
/// declaration converts it, and the checked `case` evaluates it.
#[trace("FR-321-AC-3", "FR-320-AC-1", "TC-830")]
#[test]
fn a_union_input_evaluates_a_checked_case_through_replay() {
    let compiled = spine(SHAPE_UNIT, &std::collections::BTreeMap::new());
    let graph = compiled.package.graph();
    let types = graph.scope().types();
    let shape = types
        .composites()
        .find(|declaration| declaration.name() == "Shape")
        .expect("Shape is declared")
        .key();
    let node = types.union_key(shape).expect("check sealed the union");
    let declaration = WireNodeId::from_digest(*node.as_bytes());
    let supplied = |member: &str, components: Vec<WitnessValue>| WitnessValue::Union {
        declaration,
        member: member.to_owned(),
        components,
    };
    for (value, expected) in [
        (supplied("Empty", vec![]), false),
        (supplied("Circle", vec![int(4)]), true),
        (supplied("Circle", vec![int(2)]), false),
        (supplied("Rect", vec![int(2), int(3)]), true),
        (supplied("Rect", vec![int(1), int(2)]), false),
    ] {
        let request = request(
            SHAPE_UNIT.as_bytes(),
            compiled.emitted.package_id(),
            name(&["big"]),
            typed_input(parameter(&compiled, "big", 0), value.clone()),
        );
        let ReplayResult::Input(result) =
            replay(request, ReplayLimits::default()).expect("the replay runs")
        else {
            panic!("an Input-sourced request settles on the Input arm");
        };
        assert_eq!(
            result.value(),
            Some(EvaluatedValue::Boolean(expected)),
            "{value:?}"
        );
    }
}
