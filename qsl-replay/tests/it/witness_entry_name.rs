// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-070-AC-19 (TC-1281): witness entry names preserve node identity and
//! escape transcript delimiters.

use ix_trace_rs::trace;
use qsl_replay::{
    witness_entry_name, DecodeRefusal, NodeKey, Witness, WitnessBinding, WitnessValue,
    WitnessValueType,
};
use quire_exact::PathSegment;

fn node(text: &str, ordinal: u64) -> NodeKey {
    NodeKey::new("fixture", vec![PathSegment::Text(text.to_owned())], ordinal)
        .expect("the fixture node is admissible")
}

fn assertion(values: &str) -> String {
    format!("<<<assertion|h|c|{values}>>>")
}

fn boolean_binding(parameter: NodeKey) -> WitnessBinding {
    WitnessBinding {
        parameter,
        value_type: WitnessValueType::Boolean,
    }
}

/// FR-070-AC-19: reserved bytes are escaped in uppercase and other UTF-8 is
/// preserved; similar source strings remain distinct and decode to their
/// original identities.
#[trace("TC-1281", "FR-070-AC-19")]
#[test]
fn entry_names_escape_delimiters_and_round_trip_node_identity() {
    let special = node("a%=;|<>é", 2);
    let name = witness_entry_name(&special).expect("the node has a transcript spelling");
    assert_eq!(
        name,
        r#"{"ordinal":2,"package":"fixture","path":["a%25%3D%3B%7C%3C%3E%7Fé"]}"#
    );
    assert!(!name.contains(['=', ';', '|', '<', '>']));

    let equals = node("=", 0);
    let escape_like = node("%3D", 0);
    let equals_name = witness_entry_name(&equals).expect("the equals node spells");
    let escape_like_name = witness_entry_name(&escape_like).expect("the escape-like node spells");
    assert_eq!(
        equals_name,
        r#"{"ordinal":0,"package":"fixture","path":["%3D"]}"#
    );
    assert_eq!(
        escape_like_name,
        r#"{"ordinal":0,"package":"fixture","path":["%253D"]}"#
    );
    assert_ne!(equals_name, escape_like_name);

    for (parameter, entry_name) in [
        (equals.clone(), equals_name),
        (escape_like.clone(), escape_like_name),
    ] {
        let witness = Witness::parse(assertion(&format!("{entry_name}=1")))
            .expect("the generated entry parses as one witness");
        assert_eq!(
            witness.decode(&[boolean_binding(parameter)]),
            Ok(vec![WitnessValue::Boolean(true)])
        );
    }
}

/// FR-070-AC-19: witness entries follow structural NodeKey order even when
/// their display names sort lexically in the opposite order.
#[trace("TC-1281", "FR-070-AC-19")]
#[test]
fn entry_order_uses_numeric_node_ordinal_not_display_name_text() {
    let ordinal_two = node("same", 2);
    let ordinal_ten = node("same", 10);
    assert!(ordinal_two < ordinal_ten);

    let name_two = witness_entry_name(&ordinal_two).expect("ordinal two spells");
    let name_ten = witness_entry_name(&ordinal_ten).expect("ordinal ten spells");
    assert!(
        name_ten < name_two,
        "display names sort opposite structural ids"
    );

    let bindings = [
        boolean_binding(ordinal_two.clone()),
        boolean_binding(ordinal_ten.clone()),
    ];
    let ascending = Witness::parse(assertion(&format!("{name_two}=0;{name_ten}=1")))
        .expect("the structurally ordered transcript parses");
    assert_eq!(
        ascending.decode(&bindings),
        Ok(vec![
            WitnessValue::Boolean(false),
            WitnessValue::Boolean(true)
        ])
    );

    let reversed = Witness::parse(assertion(&format!("{name_ten}=1;{name_two}=0")))
        .expect("the reversed transcript parses before ordering is checked");
    assert_eq!(
        reversed.decode(&bindings),
        Err(DecodeRefusal::EntryOrder(name_two))
    );
}
