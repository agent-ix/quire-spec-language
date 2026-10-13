// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-359-AC-9 (TC-1002): a replay argument holding a record whose
//! most-specific type conforms to its parameter's declared record type is
//! decoded by the declaration it names and admitted at the parameter, at the
//! top, as an `Option` payload and as a collection element. A record that
//! does not conform refuses `WrongValueKind` naming the parameter.

use ix_trace_rs::trace;
use qsl_foundation::digest::WireNodeId;
use qsl_package::CheckedPackage;
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, Integer, NodeKey, Presence, Value, ValueType,
};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
};

use super::{WitnessValue, UNLIMITED};
use crate::execute::argument::{convert_arguments, Stopped};
use crate::execute::ReplayRefusal;
use crate::witness::{WitnessField, WitnessSlot};

fn key(label: &str) -> NodeKey {
    NodeKey::from_digest(qsl_foundation::digest::ByteDigest::of(label.as_bytes()).as_bytes())
}

fn record(label: &str, fields: &[&str], supertypes: &[&str]) -> CompositeDeclaration {
    CompositeDeclaration::new(
        key(label),
        label,
        CompositeShape::Record(
            fields
                .iter()
                .map(|name| FieldDeclaration::new(*name, ValueType::Integer, Presence::Required))
                .collect(),
        ),
    )
    .with_supertypes(supertypes.iter().map(|label| key(label)).collect())
}

/// `Money <- TaxedMoney <- GiftTaxedMoney` and the unrelated `Fee`.
fn package() -> CheckedPackage {
    let types = TypeEnvironment::new(
        [
            record("Money", &["amount_minor"], &[]),
            record("TaxedMoney", &["amount_minor", "tax_minor"], &["Money"]),
            record(
                "GiftTaxedMoney",
                &["amount_minor", "tax_minor", "wrap_minor"],
                &["TaxedMoney"],
            ),
            record("Fee", &["fee_minor"], &[]),
        ],
        [],
    )
    .expect("the records admit");
    CheckedPackage::link(
        qsl_semantics::check::PackageDeclarations {
            types,
            ..qsl_semantics::check::PackageDeclarations::new(
                qsl_semantics::check::fixture_source(),
                qsl_foundation::IdentityLimits::default(),
            )
        }
        .check(CheckingLimits::default())
        .expect("the package checks"),
    )
}

fn witness(label: &str, fields: &[(&str, i64)]) -> WitnessValue {
    WitnessValue::Record {
        declaration: WireNodeId::from_digest(*key(label).as_bytes()),
        fields: fields
            .iter()
            .map(|(name, value)| WitnessField {
                name: (*name).to_owned(),
                slot: WitnessSlot::Present(WitnessValue::ExactInteger(Integer::from(*value))),
            })
            .collect(),
    }
}

fn taxed() -> WitnessValue {
    witness("TaxedMoney", &[("amount_minor", 100), ("tax_minor", 20)])
}

fn gift() -> WitnessValue {
    witness(
        "GiftTaxedMoney",
        &[("amount_minor", 100), ("tax_minor", 20), ("wrap_minor", 3)],
    )
}

fn plain() -> WitnessValue {
    witness("Money", &[("amount_minor", 100)])
}

fn composite(label: &str) -> ValueType {
    ValueType::Composite(key(label))
}

/// The one argument `value` converted at the parameter type `declared`.
fn converted(value: WitnessValue, declared: ValueType) -> Result<Value, usize> {
    match convert_arguments(&package(), &[declared], &[value], &UNLIMITED) {
        Ok(mut converted) => Ok(converted.values.remove(0)),
        Err(Stopped::Refusal(refusal)) => match *refusal {
            ReplayRefusal::Input(InputRefusal::WrongValueKind { parameter }) => Err(parameter),
            other => panic!("a wrong-value-kind refusal, not {other:?}"),
        },
        Err(Stopped::Limit(limit)) => panic!("no limit applies: {limit:?}"),
    }
}

fn declaration_of(value: &Value) -> NodeKey {
    match value {
        Value::Composite(composite) => composite.declaration(),
        other => panic!("a record, not {other:?}"),
    }
}

/// A child, and a grandchild, is admitted at a parameter naming an ancestor
/// and keeps its own declaration and fields.
#[trace("FR-359-AC-9", "TC-1002")]
#[test]
fn a_child_record_is_admitted_at_an_ancestor_typed_parameter() {
    let child = converted(taxed(), composite("Money")).expect("a TaxedMoney at Money");
    assert_eq!(declaration_of(&child), key("TaxedMoney"));
    let Value::Composite(held) = &child else {
        unreachable!();
    };
    assert_eq!(held.slots().len(), 2, "the child's own two fields");
    for slot in ["Money", "TaxedMoney"] {
        let grandchild = converted(gift(), composite(slot)).expect("a Gift at an ancestor");
        assert_eq!(declaration_of(&grandchild), key("GiftTaxedMoney"));
    }
    let same = converted(plain(), composite("Money")).expect("a Money at Money");
    assert_eq!(declaration_of(&same), key("Money"));
}

/// A record that is only an ancestor of the slot's type, an unrelated
/// record, or a record missing a field of its own type refuses
/// `WrongValueKind` naming the parameter.
#[trace("FR-359-AC-9", "TC-1002")]
#[test]
fn a_record_that_does_not_conform_refuses_wrong_value_kind() {
    assert_eq!(
        converted(taxed(), composite("GiftTaxedMoney")).err(),
        Some(0)
    );
    assert_eq!(converted(plain(), composite("TaxedMoney")).err(), Some(0));
    assert_eq!(
        converted(witness("Fee", &[("fee_minor", 1)]), composite("Money")).err(),
        Some(0)
    );
    assert_eq!(
        converted(
            witness("TaxedMoney", &[("amount_minor", 100)]),
            composite("Money")
        )
        .err(),
        Some(0),
        "a TaxedMoney without tax_minor is not a TaxedMoney"
    );
}

/// The same records as the payload of an `Option` and as an element of a
/// `Sequence`, a `Set` and a `Bag`, each keeping its own declaration.
#[trace("FR-359-AC-9", "TC-1002")]
#[test]
fn a_child_record_is_admitted_as_a_payload_and_as_an_element() {
    let money = composite("Money");
    let option = converted(
        WitnessValue::Option(Some(Box::new(taxed()))),
        ValueType::option(money.clone()),
    )
    .expect("a TaxedMoney in an Option<Money>");
    let Value::Option(option) = &option else {
        panic!("an option");
    };
    assert_eq!(
        declaration_of(option.payload().expect("present")),
        key("TaxedMoney")
    );
    assert!(converted(
        WitnessValue::Option(Some(Box::new(witness("Fee", &[("fee_minor", 1)])))),
        ValueType::option(money.clone()),
    )
    .is_err());
    for kind in [
        CollectionKind::Sequence,
        CollectionKind::Set,
        CollectionKind::Bag,
        CollectionKind::OrderedSet,
    ] {
        let declared = ValueType::collection(CollectionType::new(
            kind,
            money.clone(),
            Some(CardinalityBound::new(0, 4).unwrap()),
        ));
        let elements = vec![taxed(), plain()];
        let value = match kind {
            CollectionKind::Sequence => WitnessValue::Sequence(elements),
            CollectionKind::Set => WitnessValue::Set(elements),
            CollectionKind::Bag => WitnessValue::Bag(elements),
            CollectionKind::OrderedSet => WitnessValue::OrderedSet(elements),
        };
        let formed = converted(value, declared.clone()).unwrap_or_else(|_| panic!("{kind:?}"));
        let Value::Collection(collection) = &formed else {
            panic!("a collection");
        };
        let mut keys: Vec<NodeKey> = collection.elements().iter().map(declaration_of).collect();
        assert_eq!(
            keys.len(),
            2,
            "{kind:?} holds both: a child is not its parent"
        );
        keys.sort_by_key(|key| *key.as_bytes());
        let mut expected = [key("Money"), key("TaxedMoney")];
        expected.sort_by_key(|key| *key.as_bytes());
        assert_eq!(keys, expected);
        let bad = converted(
            match kind {
                CollectionKind::Sequence => {
                    WitnessValue::Sequence(vec![witness("Fee", &[("fee_minor", 1)])])
                }
                CollectionKind::Set => WitnessValue::Set(vec![witness("Fee", &[("fee_minor", 1)])]),
                CollectionKind::Bag => WitnessValue::Bag(vec![witness("Fee", &[("fee_minor", 1)])]),
                CollectionKind::OrderedSet => {
                    WitnessValue::OrderedSet(vec![witness("Fee", &[("fee_minor", 1)])])
                }
            },
            declared,
        );
        assert!(bad.is_err(), "{kind:?}: an unrelated record is refused");
    }
}
