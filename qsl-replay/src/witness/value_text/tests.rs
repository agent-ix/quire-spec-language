// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-070 (TC-905, TC-736): the witness value text encodes and decodes
//! exactly, every malformed form refuses, and a value of any depth walks on
//! a small native stack.

use ix_trace_rs::trace;
use qsl_foundation::digest::WireNodeId;
use quire_exact::Integer;

use super::super::envelope_tests::full_packet;
use super::super::{
    measured_encoded_bytes, DecodeRefusal, QuantityMagnitude, ReplaySource, Witness,
    WitnessBinding, WitnessEnvelope, WitnessField, WitnessRefusal, WitnessSlot, WitnessValue,
    WitnessValueType,
};
use super::EntryFault;
use crate::bounds::BoundExceeded;

fn id(fill: u8) -> WireNodeId {
    WireNodeId::from_digest([fill; 32])
}

fn int(value: i64) -> WitnessValue {
    WitnessValue::ExactInteger(Integer::from(value))
}

fn field(name: &str, slot: WitnessSlot) -> WitnessField {
    WitnessField {
        name: name.to_owned(),
        slot,
    }
}

fn present(name: &str, value: WitnessValue) -> WitnessField {
    field(name, WitnessSlot::Present(value))
}

fn canonical(parameter: WireNodeId) -> WitnessBinding {
    WitnessBinding {
        parameter,
        value_type: WitnessValueType::Canonical,
    }
}

fn scalar(parameter: WireNodeId) -> WitnessBinding {
    WitnessBinding {
        parameter,
        value_type: WitnessValueType::I128,
    }
}

/// A witness over `entries` (`parameter node id`, `entry text`), in the
/// order given.
fn witness(entries: &[(WireNodeId, &str)]) -> Witness {
    let values: Vec<String> = entries
        .iter()
        .map(|(parameter, text)| format!("{parameter}={text}"))
        .collect();
    Witness::parse(format!("<<<assertion|h|c|{}>>>", values.join(";"))).unwrap()
}

/// `value` written as an entry text and read back through `decode`.
fn round_trip(value: &WitnessValue) -> WitnessValue {
    let text = value.to_value_text().unwrap();
    let mut decoded = witness(&[(id(1), &text)])
        .decode(&[canonical(id(1))])
        .unwrap();
    assert_eq!(decoded.len(), 1);
    decoded.remove(0)
}

/// The fault `text` refuses with as the one `Canonical` entry of a witness.
fn fault(text: &str) -> EntryFault {
    match witness(&[(id(1), text)]).decode(&[canonical(id(1))]) {
        Err(DecodeRefusal::Malformed {
            parameter,
            value_type: WitnessValueType::Canonical,
            fault,
        }) => {
            assert_eq!(parameter, id(1), "the refusal names the parameter");
            fault
        }
        other => panic!("{text}: expected a malformed refusal, got {other:?}"),
    }
}

fn h(fill: u8) -> String {
    id(fill).to_string()
}

/// FR-070-AC-8 (TC-905): a transcript with one `Canonical` entry and one
/// `I64` entry decodes to two values in binding order, and each slot state,
/// option state, element order, member identifier and declaration node id
/// decodes exactly as written.
#[trace("TC-905", "FR-070-AC-8")]
#[test]
fn tc_905_a_nested_composite_decodes_exactly_beside_a_scalar() {
    let value = WitnessValue::Record {
        declaration: id(0xa1),
        fields: vec![
            present(
                "i",
                WitnessValue::Record {
                    declaration: id(0xa2),
                    fields: vec![
                        present("a", int(3)),
                        field("b", WitnessSlot::Absent),
                        field("c", WitnessSlot::Null),
                    ],
                },
            ),
            present("o1", WitnessValue::Option(Some(Box::new(int(5))))),
            present("o2", WitnessValue::Option(None)),
            present("s", WitnessValue::Sequence(vec![int(3), int(1), int(2)])),
            present("st", WitnessValue::Set(vec![int(1), int(2), int(3)])),
            present("bg", WitnessValue::Bag(vec![int(1), int(1), int(2)])),
            present("os", WitnessValue::OrderedSet(vec![int(9), int(4)])),
            present(
                "u1",
                WitnessValue::Union {
                    declaration: id(0xa3),
                    member: "Circle".to_owned(),
                    components: vec![int(4)],
                },
            ),
            present(
                "u2",
                WitnessValue::Union {
                    declaration: id(0xa3),
                    member: "Empty".to_owned(),
                    components: vec![],
                },
            ),
        ],
    };
    let text = value.to_value_text().unwrap();
    let decoded = witness(&[(id(1), &text), (id(2), "7")])
        .decode(&[canonical(id(1)), scalar(id(2))])
        .unwrap();
    assert_eq!(decoded, vec![value, WitnessValue::Integer(7)]);
}

/// A set and a bag decode in ascending JCS order wherever the writer put
/// them: the text of an unsorted value is the sorted one.
#[trace("TC-905", "FR-070-AC-8")]
#[test]
fn tc_905_a_set_and_a_bag_are_written_ascending_by_their_jcs_bytes() {
    let unsorted = WitnessValue::Set(vec![int(3), int(1), int(2)]);
    assert_eq!(
        round_trip(&unsorted),
        WitnessValue::Set(vec![int(1), int(2), int(3)])
    );
    let bag = WitnessValue::Bag(vec![int(2), int(1), int(2)]);
    assert_eq!(
        round_trip(&bag),
        WitnessValue::Bag(vec![int(1), int(2), int(2)])
    );
}

/// FR-070-AC-9 (TC-905): each malformed entry refuses `Malformed` naming the
/// parameter, with no partial result, for the cause its form has.
#[trace("TC-905", "FR-070-AC-9")]
#[test]
fn tc_905_each_malformed_entry_refuses_naming_the_parameter() {
    let ten = h(0x10);
    let boolean = r#"{"type":"boolean","value":true}"#;
    let one = r#"{"type":"integer","value":"1"}"#;
    let two = r#"{"type":"integer","value":"2"}"#;
    let cases: Vec<(String, EntryFault)> = vec![
        (format!("{boolean}x"), EntryFault::Json),
        (r#"{"type":"boolean", "value":true}"#.to_owned(), EntryFault::NotJcs),
        (
            format!(r#"{{"type":"record","name":"{ten}","fields":[]}}"#),
            EntryFault::NotJcs,
        ),
        (r#"{"type":"map"}"#.to_owned(), EntryFault::Shape),
        (
            format!(r#"{{"fields":[{{"name":"a"}}],"name":"{ten}","type":"record"}}"#),
            EntryFault::Shape,
        ),
        (r#"{"type":"integer","value":"007"}"#.to_owned(), EntryFault::Spelling),
        (r#"{"type":"integer","value":7}"#.to_owned(), EntryFault::Shape),
        (
            r#"{"denominator":"4","numerator":"2","type":"rational"}"#.to_owned(),
            EntryFault::Spelling,
        ),
        (
            r#"{"denominator":"-1","numerator":"2","type":"rational"}"#.to_owned(),
            EntryFault::Spelling,
        ),
        (
            r#"{"bits":"7ff000000000000","type":"float64"}"#.to_owned(),
            EntryFault::Spelling,
        ),
        (
            r#"{"bits":"7FF0000000000000","type":"float64"}"#.to_owned(),
            EntryFault::Spelling,
        ),
        (
            format!(
                r#"{{"identity":"abc","object_type":"{ten}","type":"reference","universe":"{ten}"}}"#
            ),
            EntryFault::Spelling,
        ),
        (
            r#"{"elements":[{"type":"null"}],"type":"sequence"}"#.to_owned(),
            EntryFault::Null,
        ),
        (
            format!(r#"{{"elements":[{one},{one}],"type":"set"}}"#),
            EntryFault::Elements,
        ),
        (
            format!(r#"{{"elements":[{two},{one}],"type":"set"}}"#),
            EntryFault::Elements,
        ),
        (
            format!(r#"{{"elements":[{one},{one}],"type":"ordered-set"}}"#),
            EntryFault::Elements,
        ),
        (r#"{"type":"text","value":"50%"}"#.to_owned(), EntryFault::Escape),
        (r#"{"type":"text","value":"a%3bb"}"#.to_owned(), EntryFault::Escape),
        (
            format!(
                "{{\"fields\":[{{\"name\":\"\\u0061\",\"value\":null}}],\"name\":\"{ten}\",\"type\":\"record\"}}"
            ),
            EntryFault::NotJcs,
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(fault(&text), expected, "{text}");
    }
}

/// FR-070-AC-9: a refused entry gives no partial result even beside a good
/// one.
#[trace("TC-905", "FR-070-AC-9")]
#[test]
fn tc_905_a_refused_entry_leaves_no_partial_result() {
    let witness = witness(&[
        (id(1), r#"{"type":"boolean","value":true}"#),
        (id(2), r#"{"type":"map"}"#),
    ]);
    let refused = witness.decode(&[canonical(id(1)), canonical(id(2))]);
    assert!(
        matches!(
            refused,
            Err(DecodeRefusal::Malformed { parameter, .. }) if parameter == id(2)
        ),
        "{refused:?}"
    );
}

/// FR-070-AC-11 (TC-905): one entry per leaf family decodes exactly as
/// written.
#[trace("TC-905", "FR-070-AC-11")]
#[test]
fn tc_905_each_leaf_family_decodes_exactly() {
    let integer =
        WitnessValue::ExactInteger("-170141183460469231731687303715884105728".parse().unwrap());
    assert_eq!(round_trip(&integer), integer);
    let enum_member = WitnessValue::Enum {
        declaration: id(0xe1),
        member: "RED".to_owned(),
    };
    assert_eq!(round_trip(&enum_member), enum_member);
    let rational = WitnessValue::Rational {
        numerator: Integer::from(-3_i64),
        denominator: Integer::from(4_i64),
    };
    assert_eq!(round_trip(&rational), rational);
    // The decimal keeps its retained coefficient and scale.
    let decimal = WitnessValue::Decimal {
        coefficient: Integer::from(1050_i64),
        scale: Integer::from(2_i64),
    };
    assert_eq!(round_trip(&decimal), decimal);
    // A NaN payload and a signed zero stay distinct.
    let nan = WitnessValue::Float32(0x7fc0_0001);
    assert_eq!(round_trip(&nan), nan);
    let negative_zero = WitnessValue::Float64(0x8000_0000_0000_0000);
    assert_eq!(round_trip(&negative_zero), negative_zero);
    let quantity = WitnessValue::Quantity {
        magnitude: QuantityMagnitude::Decimal {
            coefficient: Integer::from(15_i64),
            scale: Integer::from(1_i64),
        },
        unit: [0x44; 32],
    };
    assert_eq!(round_trip(&quantity), quantity);
    let reference = WitnessValue::Reference {
        universe: [0x01; 32],
        object_type: [0x02; 32],
        identity: b"order-17".to_vec(),
    };
    assert_eq!(round_trip(&reference), reference);
}

/// FR-070-AC-11: the text `a;b=c<<<d>>>e%f` is written with exactly four
/// characters replaced, its escaped bytes count toward the entry's size, and
/// the transcript parses as one block whose entry splits at its own
/// delimiters only. A sequence of texts holding `;` and `>` decodes in order.
#[trace("TC-905", "FR-070-AC-11")]
#[test]
fn tc_905_a_text_with_delimiters_is_escaped_and_decodes_exactly() {
    let text = WitnessValue::Text("a;b=c<<<d>>>e%f".to_owned());
    let escaped = text.to_value_text().unwrap();
    assert_eq!(
        escaped,
        r#"{"type":"text","value":"a%3Bb=c%3C%3C%3Cd%3E%3E%3Ee%25f"}"#
    );
    assert_eq!(text.value_text_len(), escaped.len());
    assert_eq!(round_trip(&text), text);
    let sequence = WitnessValue::Sequence(vec![
        WitnessValue::Text("x;y".to_owned()),
        WitnessValue::Text("p>q".to_owned()),
        WitnessValue::Text(">>>".to_owned()),
    ]);
    assert_eq!(round_trip(&sequence), sequence);
}

/// FR-070-AC-11 (the size claim): an `Input` assignment counts its value at
/// its escaped text's length, a scalar at 8 bytes.
#[trace("TC-905", "FR-070-AC-11")]
#[test]
fn tc_905_an_input_value_counts_at_its_escaped_text_length() {
    let text = WitnessValue::Text("a;b".to_owned());
    let expected = 32 + text.to_value_text().unwrap().len();
    let source = ReplaySource::Input(vec![crate::CanonicalAssignment {
        parameter: id(1),
        value: text,
    }]);
    assert_eq!(source.measured_bytes(), expected);
    let scalar_source = ReplaySource::Input(vec![crate::CanonicalAssignment {
        parameter: id(1),
        value: WitnessValue::Integer(1),
    }]);
    // An i128 integer is charged its 16 bytes.
    assert_eq!(scalar_source.measured_bytes(), 32 + 16);
}

/// FR-070-AC-12 (TC-905): entries for `0a..` and `0b..` in that order
/// decode; the same entries with `0b..` first refuse `EntryOrder` naming the
/// `0a..` entry. A repeated node id stays `Duplicate`.
#[trace("TC-905", "FR-070-AC-12")]
#[test]
fn tc_905_entries_must_ascend_by_parameter_node_id() {
    let (a, b) = (id(0x0a), id(0x0b));
    let ascending = witness(&[(a, "1"), (b, "2")]);
    assert_eq!(
        ascending.decode(&[scalar(a), scalar(b)]).unwrap(),
        vec![WitnessValue::Integer(1), WitnessValue::Integer(2)]
    );
    let descending = witness(&[(b, "2"), (a, "1")]);
    assert_eq!(
        descending.decode(&[scalar(a), scalar(b)]),
        Err(DecodeRefusal::EntryOrder(a.to_string()))
    );
    let repeated = witness(&[(a, "1"), (a, "2")]);
    assert_eq!(
        repeated.decode(&[scalar(a), scalar(b)]),
        Err(DecodeRefusal::Duplicate(a))
    );
}

/// A record of `depth` nested lists: `record List { head: Int[0, 9];
/// tail?: List; }`, built from the inside out.
fn list(depth: usize, head_name: &str, tail_name: &str) -> WitnessValue {
    let declaration = id(0x77);
    let mut value = WitnessValue::Record {
        declaration,
        fields: vec![
            present(head_name, int(1)),
            field(tail_name, WitnessSlot::Absent),
        ],
    };
    for _ in 1..depth {
        value = WitnessValue::Record {
            declaration,
            fields: vec![present(head_name, int(1)), present(tail_name, value)],
        };
    }
    value
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

/// FR-070-AC-10 (TC-736): on a thread with a 512 KiB stack, a transcript
/// entry holding a 100,000-long recursive list decodes, with the envelope's
/// reader bound raised to fit it through `replay.input_bytes`. The decoded
/// value clones, compares equal to itself, renders its redacted `Debug` and
/// drops with no stack overflow. With the bound one byte below the
/// envelope's size, construction refuses `BoundExceeded` naming
/// `replay.input_bytes`.
#[trace("TC-736", "FR-070-AC-10")]
#[test]
fn tc_736_a_100000_long_list_decodes_and_walks_on_a_small_stack() {
    on_small_stack(|| {
        let value = list(100_000, "head", "tail");
        let text = value.to_value_text().unwrap();
        let transcript = format!("<<<assertion|h|c|{}={text}>>>", id(1));
        let witness = Witness::parse(transcript).unwrap();

        let mut packet = full_packet(0);
        packet.source = Some(ReplaySource::Witness(witness.clone()));
        let size = measured_encoded_bytes(&packet);
        assert!(
            size > crate::DEFAULT_REPLAY_INPUT_BYTES as usize,
            "the value needs the raised bound"
        );
        assert!(matches!(
            WitnessEnvelope::reconstruct(packet.clone(), crate::ReplayLimits::default()),
            Err(WitnessRefusal::BoundExceeded(BoundExceeded { bound, .. }))
                if bound == crate::DEFAULT_REPLAY_INPUT_BYTES
        ));
        let below = WitnessEnvelope::reconstruct(
            packet.clone(),
            crate::ReplayLimits::default().with_input_bytes(size as u64 - 1),
        )
        .unwrap_err();
        let WitnessRefusal::BoundExceeded(exceeded) = below else {
            panic!("expected BoundExceeded, got {below:?}");
        };
        assert_eq!(exceeded.actual, size as u128);
        assert_eq!(exceeded.bound, size as u64 - 1);
        assert!(exceeded.to_string().contains("replay.input_bytes"));
        let envelope = WitnessEnvelope::reconstruct(
            packet,
            crate::ReplayLimits::default().with_input_bytes(size as u64),
        )
        .unwrap();

        let ReplaySource::Witness(carried) = envelope.source() else {
            panic!("a witness source");
        };
        let decoded = carried.decode(&[canonical(id(1))]).unwrap().remove(0);
        assert_eq!(decoded.node_count(), 200_000);
        let copy = decoded.clone();
        assert_eq!(copy, decoded);
        let rendered = format!("{decoded:?}");
        assert!(rendered.contains("record"));
        assert!(!rendered.contains("head"));
        // The value is the one that was written.
        assert_eq!(decoded, value);
        drop(copy);
        drop(decoded);
        drop(value);
    });
}

/// `depth` nested sets, each holding the next one and a text leaf, so every
/// level has two elements to order.
fn nested_sets(depth: usize) -> WitnessValue {
    let mut value = WitnessValue::Set(vec![WitnessValue::Text("leaf".to_owned()), int(0)]);
    for _ in 1..depth {
        value = WitnessValue::Set(vec![value, WitnessValue::Text("leaf".to_owned())]);
    }
    value
}

/// The work `value` takes to encode, and the length of its output.
fn work_of(value: &WitnessValue) -> (u64, usize) {
    let (arena, work) = super::encode_pieces(value).unwrap();
    let length = super::Chunks::new(&arena, 0).map(<[u8]>::len).sum();
    (work.written + work.compared, length)
}

fn nested_options(depth: usize) -> WitnessValue {
    let mut value = int(0);
    for _ in 0..depth {
        value = WitnessValue::Option(Some(Box::new(value)));
    }
    value
}

fn nested_sequences(depth: usize) -> WitnessValue {
    let mut value = int(0);
    for _ in 0..depth {
        value = WitnessValue::Sequence(vec![value, int(1)]);
    }
    value
}

/// FR-070-AC-13 (QSL-647): encoding does work proportional to the output, at
/// any depth. Doubling the depth of nested sets, nested options and nested
/// sequences doubles the work (a ratio near 2); an encoder that copies each
/// subtree into every ancestor would quadruple it. The text decodes, order
/// check included, on a 512 KiB stack and round-trips.
#[trace("TC-905", "FR-070-AC-13")]
#[test]
fn tc_905_encoding_work_is_linear_in_depth() {
    on_small_stack(|| {
        const DEPTH: usize = 5_000;
        let shapes: [(&str, fn(usize) -> WitnessValue); 3] = [
            ("sets", nested_sets),
            ("options", nested_options),
            ("sequences", nested_sequences),
        ];
        for (name, build) in shapes {
            let (single, single_len) = work_of(&build(DEPTH));
            let (double, double_len) = work_of(&build(2 * DEPTH));
            assert!(
                single >= single_len as u64,
                "{name}: work counts at least the output"
            );
            assert!(
                double <= 3 * single,
                "{name}: work {single} at depth {DEPTH} became {double} at {}",
                2 * DEPTH
            );
            assert!(
                double <= 3 * double_len as u64,
                "{name}: work {double} for {double_len} bytes of output"
            );
        }
        let value = nested_sets(DEPTH);
        let text = value.to_value_text().unwrap();
        assert_eq!(value.value_text_len(), text.len());
        let decoded = WitnessValue::from_value_text(&text).unwrap();
        assert_eq!(decoded.to_value_text().unwrap(), text);
        drop(decoded);
        drop(value);
    });
}

/// FR-070-AC-9 (QSL-647): the decode order check still refuses out-of-order
/// and duplicate elements found by span, in a nested set, a bag and an
/// ordered set.
#[trace("TC-905", "FR-070-AC-9")]
#[test]
fn tc_905_the_element_order_check_reads_spans() {
    let text = |value: &str| format!(r#"{{"type":"text","value":"{value}"}}"#);
    let a = text("a");
    let b = text("b");
    let collection = |kind: &str, elements: &[&str]| {
        format!(r#"{{"elements":[{}],"type":"{kind}"}}"#, elements.join(","))
    };
    let decodes = |json: &str| WitnessValue::from_value_text(json).map(|_| ());
    assert_eq!(decodes(&collection("set", &[&a, &b])), Ok(()));
    assert_eq!(decodes(&collection("bag", &[&a, &a, &b])), Ok(()));
    assert_eq!(decodes(&collection("ordered-set", &[&b, &a])), Ok(()));
    assert_eq!(
        decodes(&collection("set", &[&b, &a])),
        Err(EntryFault::Elements)
    );
    assert_eq!(
        decodes(&collection("set", &[&a, &a])),
        Err(EntryFault::Elements)
    );
    assert_eq!(
        decodes(&collection("bag", &[&b, &a])),
        Err(EntryFault::Elements)
    );
    assert_eq!(
        decodes(&collection("ordered-set", &[&a, &a])),
        Err(EntryFault::Elements)
    );
    // A bad collection nested inside a good one is found.
    let bad = collection("set", &[&b, &a]);
    assert_eq!(
        decodes(&collection("set", &[&a, &bad])),
        Err(EntryFault::Elements)
    );
}
