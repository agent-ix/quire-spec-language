// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-359 / TC-998 to TC-1002: evaluation of a child record value at an
//! ancestor-typed slot, over the FR-359 domain package checked and linked
//! from source.
//!
//! Every assertion compares typed values, declaration keys and outcome
//! shapes, never message text. The child keeps its own identity at every
//! slot (item 1), a field is read by identity (item 2), and equality over a
//! child at a parent-typed slot is a Boolean (item 4).

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{FieldValue, Integer, Meter, NodeKey, Outcome, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;

#[path = "../../../qsl-semantics/tests/support/child_composite.rs"]
mod fixture;

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

/// `declarations` checked and linked as a unit selecting the FR-359 package
/// as model `alias`.
fn package_as(alias: &str, declarations: &str) -> CheckedPackage {
    package_over(&fixture::document(), alias, declarations)
}

fn package_from(document: &serde_json::Value, declarations: &str) -> CheckedPackage {
    package_over(document, "M", declarations)
}

fn package_over(document: &serde_json::Value, alias: &str, declarations: &str) -> CheckedPackage {
    let bytes = serde_json::to_vec(document).expect("the fixture serializes");
    let packages = package_input([bytes.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one model package");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model {alias} = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         {declarations}\n",
        package = fixture::PACKAGE,
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "child-composite", "fixture", "fixture:1"),
        "child-composite.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the source unit");
    assert!(
        parsed.is_admissible(),
        "{:?}\n{source}",
        parsed.diagnostics()
    );
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the source unit");
    let selected = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("I1 admits the fixture package");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("assembly succeeds");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}")),
    )
}

fn package(declarations: &str) -> CheckedPackage {
    package_as("M", declarations)
}

/// The declaration key of the model record `name`.
fn declaration(package: &CheckedPackage, name: &str) -> NodeKey {
    let wanted = format!("M::{name}");
    package
        .graph()
        .scope()
        .types()
        .composites()
        .find(|composite| composite.name() == wanted)
        .unwrap_or_else(|| panic!("{wanted} is declared"))
        .key()
}

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

/// A `name` record holding the given present fields; an omitted optional
/// field is absent.
fn record(package: &CheckedPackage, name: &str, fields: Vec<(&str, Value)>) -> Value {
    package
        .graph()
        .scope()
        .types()
        .record(
            declaration(package, name),
            fields
                .into_iter()
                .map(|(field, value)| (field, FieldValue::Present(value)))
                .collect(),
        )
        .unwrap_or_else(|refusal| panic!("{name} constructs: {refusal:?}"))
}

/// `Money { amount_minor, currency }` of the record `name`, with `extra`.
fn money(package: &CheckedPackage, name: &str, amount: i64, extra: Vec<(&str, Value)>) -> Value {
    let mut fields = vec![("amount_minor", int(amount)), ("currency", int(978))];
    fields.extend(extra);
    record(package, name, fields)
}

fn plain(package: &CheckedPackage, amount: i64) -> Value {
    money(package, "Money", amount, vec![])
}

fn taxed(package: &CheckedPackage, amount: i64, tax: i64) -> Value {
    money(package, "TaxedMoney", amount, vec![("tax_minor", int(tax))])
}

fn gift(package: &CheckedPackage, amount: i64, tax: i64, wrap: i64) -> Value {
    money(
        package,
        "GiftTaxedMoney",
        amount,
        vec![("tax_minor", int(tax)), ("wrap_minor", int(wrap))],
    )
}

fn run(package: &CheckedPackage, function: &str, arguments: Vec<Value>) -> Outcome<Value> {
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified(function).expect("an identifier"),
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .unwrap_or_else(|failure| panic!("{function} runs: {failure:?}"));
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("a kernel outcome, not {other:?}"),
    }
}

/// The value `function` completes with.
fn completed(package: &CheckedPackage, function: &str, arguments: Vec<Value>) -> Value {
    match run(package, function, arguments) {
        Outcome::Completed(value) => value,
        other => panic!("{function} completes, not {other:?}"),
    }
}

fn boolean(package: &CheckedPackage, function: &str, arguments: Vec<Value>) -> bool {
    match completed(package, function, arguments) {
        Value::Boolean(value) => value,
        other => panic!("{function} gives a Boolean, not {other:?}"),
    }
}

fn integer(package: &CheckedPackage, function: &str, arguments: Vec<Value>) -> i64 {
    match completed(package, function, arguments) {
        Value::Integer(value) => value.to_string().parse().expect("a small integer"),
        other => panic!("{function} gives an Integer, not {other:?}"),
    }
}

/// Whether two values are one value: the same declarations and every field
/// equal, by the kernel's canonical key order, not by their printed form.
fn same_value(left: &Value, right: &Value) -> bool {
    quire_exact::compare_keys(left, right) == Some(std::cmp::Ordering::Equal)
}

fn declaration_of(value: &Value) -> NodeKey {
    match value {
        Value::Composite(composite) => composite.declaration(),
        other => panic!("a composite, not {other:?}"),
    }
}

/// The present value of the field `name` of the record `value`.
fn field_of(package: &CheckedPackage, value: &Value, name: &str) -> Value {
    let Value::Composite(composite) = value else {
        panic!("a record, not {value:?}");
    };
    let types = package.graph().scope().types();
    let declaration = types
        .composite(composite.declaration())
        .expect("the record is declared");
    let quire_semantic_value::declaration::CompositeShape::Record(fields) = declaration.shape()
    else {
        panic!("a record shape");
    };
    let index = fields
        .iter()
        .position(|field| field.name() == name)
        .unwrap_or_else(|| panic!("{name} is a field"));
    match &composite.slots()[index] {
        FieldValue::Present(value) => value.clone(),
        other => panic!("{name} is present, not {other:?}"),
    }
}

fn elements(value: &Value) -> Vec<Value> {
    match value {
        Value::Collection(collection) => collection.elements().to_vec(),
        other => panic!("a collection, not {other:?}"),
    }
}

const READS: &str = "\
function occurredAt using v(e: M::OrderPlaced): Integer pure { e.occurred_at }\n\
function orderId using v(e: M::OrderPlaced): Integer pure { e.order_id }\n";

fn order_placed(
    package: &CheckedPackage,
    name: &str,
    order_id: i64,
    occurred_at: i64,
    extra: Vec<(&str, i64)>,
) -> Value {
    let mut fields = vec![
        ("order_id", int(order_id)),
        ("occurred_at", int(occurred_at)),
    ];
    fields.extend(extra.into_iter().map(|(field, value)| (field, int(value))));
    record(package, name, fields)
}

/// FR-359-AC-1: a child, and a grandchild, passed at a parent-typed
/// parameter reads its own `occurred_at` and `order_id`, never `promised_by`
/// or `rush_by`, though its fields are ordered `[promised_by, occurred_at,
/// order_id, ...]`.
#[trace("FR-359-AC-1", "TC-998")]
#[test]
fn a_field_read_at_a_parent_typed_parameter_reads_the_childs_field_by_identity() {
    let package = package(READS);
    let cases = [
        (order_placed(&package, "OrderPlaced", 7, 8, vec![]), (8, 7)),
        (
            order_placed(
                &package,
                "ExpressOrderPlaced",
                17,
                18,
                vec![("promised_by", 19)],
            ),
            (18, 17),
        ),
        (
            order_placed(
                &package,
                "RushOrderPlaced",
                27,
                28,
                vec![("promised_by", 29), ("rush_by", 30)],
            ),
            (28, 27),
        ),
    ];
    for (value, (occurred_at, order_id)) in cases {
        assert_eq!(
            integer(&package, "occurredAt", vec![value.clone()]),
            occurred_at
        );
        assert_eq!(integer(&package, "orderId", vec![value]), order_id);
    }
}

/// FR-359-AC-1: the same reads through a record field, an `Option` payload
/// and a `Sequence` element holding the child, which keeps its own type.
#[trace("FR-359-AC-1", "FR-359-AC-6", "TC-998")]
#[test]
fn a_child_is_read_through_a_record_field_an_option_and_a_sequence_element() {
    let package = package(&format!(
        "{READS}\
         type Total = Integer;\n\
         function throughField using v(h: M::Holder): Integer pure {{ h.placed.occurred_at }}\n\
         function throughOption using v(h: M::Holder): Integer pure {{ if present(h.maybe) then value(h.maybe).order_id else 0 }}\n\
         function throughSequence using v(h: M::Holder): Integer pure {{ sum<Total>(e in h.many: e.occurred_at) }}\n\
         function unwrapped using v(h: M::Holder): M::OrderPlaced pure {{ if present(h.maybe) then value(h.maybe) else h.placed }}\n"
    ));
    let rush = order_placed(
        &package,
        "RushOrderPlaced",
        27,
        28,
        vec![("promised_by", 29), ("rush_by", 30)],
    );
    let express = order_placed(
        &package,
        "ExpressOrderPlaced",
        17,
        18,
        vec![("promised_by", 19)],
    );
    let plain = order_placed(&package, "OrderPlaced", 7, 8, vec![]);
    let many = quire_exact::from_admitted(
        quire_exact::CollectionType::new(
            quire_exact::CollectionKind::Sequence,
            quire_exact::ValueType::Composite(declaration(&package, "OrderPlaced")),
            Some(quire_exact::CardinalityBound::new(0, 8).unwrap()),
        ),
        vec![rush.clone(), express.clone(), plain],
    );
    let holder = record(
        &package,
        "Holder",
        vec![
            ("placed", rush.clone()),
            ("maybe", express.clone()),
            ("many", many),
        ],
    );
    assert_eq!(integer(&package, "throughField", vec![holder.clone()]), 28);
    assert_eq!(integer(&package, "throughOption", vec![holder.clone()]), 17);
    assert_eq!(
        integer(&package, "throughSequence", vec![holder.clone()]),
        28 + 18 + 8
    );
    let unwrapped = completed(&package, "unwrapped", vec![holder]);
    assert_eq!(
        declaration_of(&unwrapped),
        declaration(&package, "ExpressOrderPlaced")
    );
    assert!(same_value(&unwrapped, &express));
}

/// FR-359-AC-2: a child that redefines an inherited field with a narrower
/// type, or under another name, is read at the parent's type through its
/// redefining slot; an unredefined inherited field is read from its own.
#[trace("FR-359-AC-2", "TC-998")]
#[test]
fn a_redefining_child_is_read_through_its_redefining_slot() {
    let package = package(
        "function quant using v(q: M::Quant): Integer pure { q.n }\n\
         function stamped using v(s: M::Stamp): Integer pure { s.at.occurred_at }\n\
         function stampedId using v(s: M::Stamp): Integer pure { s.at.order_id }\n",
    );
    let narrow = record(&package, "NarrowQuant", vec![("n", int(5))]);
    assert_eq!(integer(&package, "quant", vec![narrow]), 5);
    assert_eq!(
        integer(
            &package,
            "quant",
            vec![record(&package, "Quant", vec![("n", int(500))])]
        ),
        500
    );
    let express = order_placed(
        &package,
        "ExpressOrderPlaced",
        17,
        18,
        vec![("promised_by", 19)],
    );
    let express_stamp = record(&package, "ExpressStamp", vec![("at", express.clone())]);
    assert_eq!(
        integer(&package, "stamped", vec![express_stamp.clone()]),
        18
    );
    assert_eq!(integer(&package, "stampedId", vec![express_stamp]), 17);
    let renamed = record(&package, "RenamedStamp", vec![("when", express)]);
    assert_eq!(integer(&package, "stamped", vec![renamed.clone()]), 18);
    assert_eq!(integer(&package, "stampedId", vec![renamed]), 17);
    let parent = record(
        &package,
        "Stamp",
        vec![("at", order_placed(&package, "OrderPlaced", 1, 2, vec![]))],
    );
    assert_eq!(integer(&package, "stamped", vec![parent]), 2);
}

const EQUALITY: &str = "\
function same using v(a: M::Money, b: M::Money): Boolean pure { a = b }\n\
function differ using v(a: M::Money, b: M::Money): Boolean pure { a != b }\n\
function sameTaxed using v(a: M::TaxedMoney, b: M::TaxedMoney): Boolean pure { a = b }\n\
function sameGift using v(a: M::GiftTaxedMoney, b: M::GiftTaxedMoney): Boolean pure { a = b }\n\
function pairSame using v(a: M::Pair, b: M::Pair): Boolean pure { a = b }\n\
function optionSame using v(a: M::Invoice, b: M::Invoice): Boolean pure { a.total = b.total }\n\
function sequenceSame using v(a: M::Lines, b: M::Lines): Boolean pure { a.lines = b.lines }\n\
function setSame using v(a: M::Lines, b: M::Lines): Boolean pure { a.tags = b.tags }\n\
function bagSame using v(a: M::Lines, b: M::Lines): Boolean pure { a.counted = b.counted }\n\
function orderedSame using v(a: M::Lines, b: M::Lines): Boolean pure { a.queue = b.queue }\n\
tuple Priced(M::Money, Integer);\n\
function tupleSame using v(a: Priced, b: Priced): Boolean pure { a = b }\n";

fn lines(package: &CheckedPackage, members: Vec<Value>) -> Value {
    let collection = |kind, bound: (u64, u64)| {
        quire_exact::CollectionType::new(
            kind,
            quire_exact::ValueType::Composite(declaration(package, "Money")),
            Some(quire_exact::CardinalityBound::new(bound.0, bound.1).unwrap()),
        )
    };
    let form = |kind| {
        quire_exact::form(
            &collection(kind, (0, 8)),
            members.clone(),
            &mut Meter::new(UNLIMITED),
        )
    };
    let formed = |kind| match form(kind) {
        Outcome::Completed(value) => value,
        other => panic!("{kind:?} forms: {other:?}"),
    };
    record(
        package,
        "Lines",
        vec![
            ("lines", formed(quire_exact::CollectionKind::Sequence)),
            ("tags", formed(quire_exact::CollectionKind::Set)),
            ("counted", formed(quire_exact::CollectionKind::Bag)),
            ("queue", formed(quire_exact::CollectionKind::OrderedSet)),
        ],
    )
}

fn invoice(package: &CheckedPackage, total: Value) -> Value {
    record(package, "Invoice", vec![("total", total)])
}

/// FR-359-AC-4: `==` and `!=` over two `Money`-typed operands, one a
/// `TaxedMoney`, give `false` and `true` in either order, never a
/// checked-invariant refusal; equal effective fields are equal.
#[trace("FR-359-AC-4", "TC-999")]
#[test]
fn equality_over_a_child_at_a_parent_typed_slot_is_a_boolean() {
    let package = package(EQUALITY);
    let money = plain(&package, 100);
    let taxed_money = taxed(&package, 100, 20);
    for (left, right) in [(&taxed_money, &money), (&money, &taxed_money)] {
        assert!(!boolean(
            &package,
            "same",
            vec![left.clone(), right.clone()]
        ));
        assert!(boolean(
            &package,
            "differ",
            vec![left.clone(), right.clone()]
        ));
    }
    // A grandchild against its ancestors and against another grandchild.
    let gifted = gift(&package, 100, 20, 3);
    assert!(!boolean(
        &package,
        "same",
        vec![gifted.clone(), money.clone()]
    ));
    assert!(!boolean(
        &package,
        "same",
        vec![gifted.clone(), taxed_money.clone()]
    ));
    assert!(boolean(
        &package,
        "same",
        vec![gifted.clone(), gift(&package, 100, 20, 3)]
    ));
    assert!(!boolean(
        &package,
        "same",
        vec![gifted.clone(), gift(&package, 100, 20, 4)]
    ));
    // Equal effective fields are equal; only `tax_minor` differing is not.
    assert!(boolean(
        &package,
        "sameTaxed",
        vec![taxed(&package, 100, 20), taxed(&package, 100, 20)]
    ));
    assert!(!boolean(
        &package,
        "sameTaxed",
        vec![taxed(&package, 100, 20), taxed(&package, 100, 21)]
    ));
    assert!(boolean(
        &package,
        "same",
        vec![taxed(&package, 100, 20), taxed(&package, 100, 20)]
    ));
    assert!(!boolean(
        &package,
        "same",
        vec![taxed(&package, 100, 20), taxed(&package, 100, 21)]
    ));
    assert!(boolean(
        &package,
        "sameGift",
        vec![gift(&package, 1, 2, 3), gift(&package, 1, 2, 3)]
    ));
}

/// FR-359-AC-5: equality reaches a child inside another value by the same
/// rule: a record field, an `Option`, a `Sequence`, a `Set`, a `Bag`, an
/// `OrderedSet` and a tuple position.
#[trace("FR-359-AC-5", "TC-999")]
#[test]
fn equality_reaches_a_child_inside_another_value_by_the_same_rule() {
    let package = package(EQUALITY);
    let pair = |first: Value| {
        record(
            &package,
            "Pair",
            vec![("first", first), ("second", plain(&package, 1))],
        )
    };
    assert!(!boolean(
        &package,
        "pairSame",
        vec![pair(taxed(&package, 100, 20)), pair(plain(&package, 100))]
    ));
    assert!(boolean(
        &package,
        "pairSame",
        vec![
            pair(taxed(&package, 100, 20)),
            pair(taxed(&package, 100, 20))
        ]
    ));
    assert!(!boolean(
        &package,
        "optionSame",
        vec![
            invoice(&package, taxed(&package, 100, 20)),
            invoice(&package, plain(&package, 100))
        ]
    ));
    assert!(boolean(
        &package,
        "optionSame",
        vec![
            invoice(&package, taxed(&package, 100, 20)),
            invoice(&package, taxed(&package, 100, 20))
        ]
    ));
    for function in ["sequenceSame", "setSame", "bagSame", "orderedSame"] {
        assert!(
            !boolean(
                &package,
                function,
                vec![
                    lines(&package, vec![taxed(&package, 100, 20)]),
                    lines(&package, vec![plain(&package, 100)]),
                ]
            ),
            "{function}: a child is not its parent"
        );
        assert!(
            boolean(
                &package,
                function,
                vec![
                    lines(&package, vec![taxed(&package, 100, 20)]),
                    lines(&package, vec![taxed(&package, 100, 20)]),
                ]
            ),
            "{function}: equal children are equal"
        );
    }
    let types = package.graph().scope().types();
    let priced = |money: Value| {
        types
            .tuple(declaration_named(&package, "Priced"), vec![money, int(5)])
            .expect("a tuple of a Money and an Integer")
    };
    assert!(!boolean(
        &package,
        "tupleSame",
        vec![
            priced(taxed(&package, 100, 20)),
            priced(plain(&package, 100))
        ]
    ));
    assert!(boolean(
        &package,
        "tupleSame",
        vec![
            priced(taxed(&package, 100, 20)),
            priced(taxed(&package, 100, 20))
        ]
    ));
}

fn declaration_named(package: &CheckedPackage, name: &str) -> NodeKey {
    package
        .graph()
        .scope()
        .types()
        .composites()
        .find(|composite| composite.name() == name)
        .unwrap_or_else(|| panic!("{name} is declared"))
        .key()
}

const HOLDING: &str = "\
function build using v(m: M::TaxedMoney): M::Invoice pure { M::Invoice { total: m } }\n\
function read using v(i: M::Invoice): Option<M::Money> pure { i.total }\n\
function has using v(i: M::Invoice): Boolean pure { present(i.total) }\n\
function unwrap using v(i: M::Invoice, d: M::Money): M::Money pure { if present(i.total) then value(i.total) else d }\n\
function keep using v(m: M::Money): M::Money pure { m }\n\
function pass using v(m: M::TaxedMoney): M::Money pure { keep(m) }\n\
function asMoney using v(m: M::GiftTaxedMoney): M::Money pure { m }\n";

/// FR-359-AC-6: a record built with an optional field holding a
/// `TaxedMoney` reads back, through `total`, `present` and `value`, as the
/// same `TaxedMoney`, and no `field-option-payload-rejected` fault is
/// reported.
#[trace("FR-359-AC-6", "FR-359-AC-8", "FR-359-AC-10", "TC-1000", "TC-1001")]
#[test]
fn an_optional_field_holding_a_child_reads_back_the_same_child() {
    let package = package(HOLDING);
    let child = taxed(&package, 100, 20);
    let invoice = completed(&package, "build", vec![child.clone()]);
    assert_eq!(declaration_of(&invoice), declaration(&package, "Invoice"));
    let read = completed(&package, "read", vec![invoice.clone()]);
    let Value::Option(option) = &read else {
        panic!("an option, not {read:?}");
    };
    let payload = option.payload().expect("present");
    assert_eq!(declaration_of(payload), declaration(&package, "TaxedMoney"));
    assert!(same_value(payload, &child));
    assert!(boolean(&package, "has", vec![invoice.clone()]));
    let unwrapped = completed(&package, "unwrap", vec![invoice, plain(&package, 0)]);
    assert!(same_value(&unwrapped, &child));
    // A function argument and result keep the child's own type as well.
    let passed = completed(&package, "pass", vec![child.clone()]);
    assert!(same_value(&passed, &child));
    let grand = gift(&package, 1, 2, 3);
    let upcast = completed(&package, "asMoney", vec![grand.clone()]);
    assert_eq!(
        declaration_of(&upcast),
        declaration(&package, "GiftTaxedMoney")
    );
}

/// FR-359-AC-7: a `Sequence`, `Set`, `Bag` and `OrderedSet` of `Money`
/// holding a `TaxedMoney` keeps its identity through iteration and `size`;
/// a `Set` or `Bag` holding a `Money` and a `TaxedMoney` of equal shared
/// fields has two elements, ordered by declaration key bytes in either
/// source order, an `OrderedSet` and a `Sequence` keep their own order.
#[trace("FR-359-AC-7", "TC-1000")]
#[test]
fn collections_of_the_parent_hold_children_and_order_by_declaration_key() {
    let package = package(
        "function sequenceOf using v(a: M::Money, b: M::Money): M::Lines pure { M::Lines { lines: sequence[a, b], tags: set[a, b], counted: bag[a, b], queue: orderedSet[a, b] } }\n\
         function found using v(i: M::Lines, m: M::TaxedMoney): Boolean pure { contains(i.tags, m) }\n\
         function tally using v(i: M::Lines): Integer pure { size(i.tags) }\n",
    );
    let money = plain(&package, 100);
    let child = taxed(&package, 100, 20);
    let (money_key, child_key) = (
        declaration(&package, "Money"),
        declaration(&package, "TaxedMoney"),
    );
    let ascending = if money_key.as_bytes() < child_key.as_bytes() {
        [money_key, child_key]
    } else {
        [child_key, money_key]
    };
    for (first, second) in [
        (money.clone(), child.clone()),
        (child.clone(), money.clone()),
    ] {
        let built = completed(&package, "sequenceOf", vec![first.clone(), second.clone()]);
        let keys = |name: &str| -> Vec<NodeKey> {
            elements(&field_of(&package, &built, name))
                .iter()
                .map(declaration_of)
                .collect()
        };
        assert_eq!(keys("tags"), ascending, "a Set orders by declaration key");
        assert_eq!(
            keys("counted"),
            ascending,
            "a Bag orders by declaration key"
        );
        let supplied = [declaration_of(&first), declaration_of(&second)];
        assert_eq!(keys("lines"), supplied, "a Sequence keeps supply order");
        assert_eq!(
            keys("queue"),
            supplied,
            "an OrderedSet keeps first-occurrence order"
        );
        assert_eq!(integer(&package, "tally", vec![built.clone()]), 2);
        assert!(boolean(
            &package,
            "found",
            vec![built.clone(), child.clone()]
        ));
        // The child inside keeps its own fields.
        let held = elements(&field_of(&package, &built, "tags"))
            .into_iter()
            .find(|element| declaration_of(element) == child_key)
            .expect("the child is kept");
        assert!(same_value(&held, &child));
    }
}

/// FR-359-AC-11: two units that select the same model under different
/// aliases give the same declaration keys, so a `Set<Money>` orders its
/// elements the same way and a value built in one unit equals the value
/// built in the other.
#[trace("FR-359-AC-11", "TC-1000")]
#[test]
fn two_aliases_of_one_model_agree_on_keys_order_and_equality() {
    let build = |alias: &str| {
        let package = package_as(
            alias,
            &format!(
                "function made using v(a: {alias}::Money, b: {alias}::Money): {alias}::Lines pure {{ {alias}::Lines {{ lines: sequence[a, b], tags: set[a, b], counted: bag[a, b], queue: orderedSet[a, b] }} }}\n"
            ),
        );
        let types = package.graph().scope().types();
        let key_of = |name: &str| {
            types
                .composites()
                .find(|composite| composite.name() == format!("{alias}::{name}"))
                .unwrap()
                .key()
        };
        let money = types
            .record(
                key_of("Money"),
                vec![
                    ("amount_minor", FieldValue::Present(int(100))),
                    ("currency", FieldValue::Present(int(978))),
                ],
            )
            .unwrap();
        let taxed = types
            .record(
                key_of("TaxedMoney"),
                vec![
                    ("amount_minor", FieldValue::Present(int(100))),
                    ("currency", FieldValue::Present(int(978))),
                    ("tax_minor", FieldValue::Present(int(20))),
                ],
            )
            .unwrap();
        let money_copy = money.clone();
        let built = completed(&package, "made", vec![taxed.clone(), money]);
        let tags: Vec<NodeKey> = {
            let Value::Composite(lines) = &built else {
                panic!("a record");
            };
            let quire_semantic_value::declaration::CompositeShape::Record(fields) =
                types.composite(lines.declaration()).unwrap().shape()
            else {
                panic!("a record shape");
            };
            let index = fields.iter().position(|f| f.name() == "tags").unwrap();
            let FieldValue::Present(set) = &lines.slots()[index] else {
                panic!("tags are present");
            };
            elements(set).iter().map(declaration_of).collect()
        };
        let names: BTreeMap<String, NodeKey> = ["Money", "TaxedMoney", "GiftTaxedMoney"]
            .into_iter()
            .map(|name| (name.to_owned(), key_of(name)))
            .collect();
        (names, tags, taxed, money_copy)
    };
    let (m_names, m_tags, m_taxed, m_money) = build("M");
    let (n_names, n_tags, n_taxed, n_money) = build("N");
    assert_eq!(m_names, n_names, "every model record has one key");
    assert_eq!(m_tags, n_tags, "a Set orders its elements identically");
    assert!(
        same_value(&m_taxed, &n_taxed),
        "a TaxedMoney built in one unit equals the same value built in the other"
    );
    // The oracle can fail: a child is not its parent, nor equal across
    // the two units' parents.
    assert!(!same_value(&m_taxed, &m_money));
    assert!(!same_value(&m_taxed, &n_money));
}

/// FR-359-AC-5, TC-999 step 3: comparing a child with a different
/// most-specific type decides by type first and performs no text-leaf
/// comparison. A `StampedTicket` and a `Ticket` with the same `label` are
/// unequal, where an implementation that compared the parent's text leaves
/// first would call them equal. Needs PR #789 (String leaves lower).
#[trace("FR-359-AC-5", "TC-999")]
#[test]
#[ignore = "PR #789 (QSL-723): String leaves do not lower yet"]
fn a_mixed_pair_with_equal_text_leaves_is_unequal() {
    let document = fixture::document_with_native_leaves();
    let package = package_from(
        &document,
        "function same using v(a: M::Ticket, b: M::Ticket): Boolean pure { a = b }",
    );
    let ticket = |name: &str, extra: Vec<(&str, Value)>| {
        let mut fields = vec![("label", Value::String("EUR".to_owned()))];
        fields.extend(extra);
        record(&package, name, fields)
    };
    let child = ticket("StampedTicket", vec![("stamp", int(1))]);
    let parent = ticket("Ticket", vec![]);
    assert!(!boolean(&package, "same", vec![child, parent]));
}

// ---------------------------------------------------------------------------
// FR-359-AC-8 / TC-1001: a model operation's parameter and result
// ---------------------------------------------------------------------------

use qsl_semantics::model::observation::{
    admit_observations, AdmissionFailure, AdmittedObservations, ClauseFacts, ClauseSelection,
    ClauseSelectionInput, DocumentRef, ObservationLimits, OperationFacts, Provisions,
};
use serde_json::json;

const OPERATION_CLAUSES: &str = "\
pre PayHolds using v on M::Ledger::pay { m.amount_minor > 0 }\n\
post PayReturns using v on M::Ledger::pay { result.amount_minor = m.amount_minor }\n\
pre PayTaxedHolds using v on M::Ledger::payTaxed { m.tax_minor >= 0 }\n";

fn record_json(name: &str, fields: &[(&str, i64)]) -> serde_json::Value {
    let fields: serde_json::Map<String, serde_json::Value> = fields
        .iter()
        .map(|(field, value)| ((*field).to_owned(), json!({"integer": value.to_string()})))
        .collect();
    json!({"record": {"type": fixture::id(name), "fields": fields}})
}

fn taxed_json(amount: i64) -> serde_json::Value {
    record_json(
        "TaxedMoney",
        &[
            ("amount_minor", amount),
            ("currency", 978),
            ("tax_minor", 20),
        ],
    )
}

fn money_json(amount: i64) -> serde_json::Value {
    record_json("Money", &[("amount_minor", amount), ("currency", 978)])
}

fn document_ref(label: &str, bytes: &[u8]) -> DocumentRef {
    let document = quire_canonical::read(bytes, u64::MAX).expect("the document is JSON");
    let digest = *quire_canonical::sha256(&document, quire_canonical::Limits::new(u64::MAX))
        .expect("the document is canonical")
        .as_bytes();
    DocumentRef {
        authority: "test".to_owned(),
        identity: label.to_owned(),
        revision_namespace: "fixture".to_owned(),
        revision: "1".to_owned(),
        digest,
    }
}

fn identity_json(reference: &DocumentRef) -> serde_json::Value {
    json!({
        "authority": reference.authority, "identity": reference.identity,
        "revision_namespace": reference.revision_namespace, "revision": reference.revision,
    })
}

/// `clause` admitted over a `Ledger.<operation>` invocation whose parameter
/// `m` and result are the given values (`None` result is `null`).
fn admit_invocation(
    package: &CheckedPackage,
    clause: &str,
    operation: &str,
    parameter: serde_json::Value,
    result: Option<serde_json::Value>,
) -> Result<AdmittedObservations, AdmissionFailure> {
    let document = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let packages = package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one model package");
    };
    let model = json!({
        "identity": fixture::PACKAGE, "version": "1",
        "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(digest)),
    });
    let snapshot = |label: &str, observation: &str| {
        let identity = DocumentRef {
            authority: "test".to_owned(),
            identity: label.to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "1".to_owned(),
            digest: [0; 32],
        };
        let bytes = serde_json::to_vec(&json!({
            "format": "quire.state.snapshot/v1",
            "identity": identity_json(&identity),
            "observation": observation,
            "model": model,
            "populations": [{
                "population": fixture::id("ledgers"), "complete": true,
                "objects": [{"key": "l1", "type": fixture::id("Ledger"),
                             "fields": {"ledger_id": {"integer": "1"}}}],
            }],
        }))
        .expect("the snapshot serializes");
        (document_ref(label, &bytes), bytes)
    };
    let (pre, pre_bytes) = snapshot("pre", "pre");
    let (post, post_bytes) = snapshot("post", "post");
    let invocation_identity = DocumentRef {
        authority: "test".to_owned(),
        identity: "call".to_owned(),
        revision_namespace: "fixture".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let reference = |document: &DocumentRef| {
        json!({
            "identity": identity_json(document),
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&document.digest)),
        })
    };
    let invocation_bytes = serde_json::to_vec(&json!({
        "format": "quire.state.invocation/v1",
        "identity": identity_json(&invocation_identity),
        "model": model,
        "context": fixture::id("Ledger"),
        "operation": operation,
        "self": {"population": fixture::id("ledgers"), "key": "l1"},
        "pre": reference(&pre),
        "post": reference(&post),
        "parameters": {"m": parameter},
        "result": result,
        "created": [],
        "deleted": [],
    }))
    .expect("the invocation serializes");
    let invocation = document_ref("call", &invocation_bytes);
    let snapshots = BTreeMap::from([(pre.digest, pre_bytes), (post.digest, post_bytes)]);
    let invocations = BTreeMap::from([(invocation.digest, invocation_bytes)]);
    let checked = package.graph().state_clause(clause).expect("the clause");
    let facts = ClauseFacts {
        identity: checked.identity(),
        kind: checked.kind(),
        context: checked.context(),
        operation: checked.operation().map(|operation| OperationFacts {
            declaring: operation.declaring,
            declaration: operation.declaration.clone(),
        }),
    };
    admit_observations(
        package.graph().model_selections(),
        package.graph().scope().types(),
        &facts,
        &packages,
        ModelNormalizationLimits::default(),
        &Provisions {
            snapshots: &snapshots,
            invocations: &invocations,
        },
        &ClauseSelection {
            name: clause.to_owned(),
            input: ClauseSelectionInput::Invocation { invocation },
        },
        ObservationLimits::default(),
    )
}

fn holds(package: &CheckedPackage, clause: &str, observations: &AdmittedObservations) -> bool {
    let evaluated = package
        .evaluate_clause(
            &QualifiedName::unqualified(clause).unwrap(),
            observations,
            &mut Meter::new(UNLIMITED),
        )
        .expect("admitted observations evaluate");
    match evaluated.evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(holds))) => holds,
        other => panic!("{clause} completes with a Boolean, not {other:?}"),
    }
}

fn parameter_declaration(observations: &AdmittedObservations) -> NodeKey {
    let [(name, value)] = &observations.parameters[..] else {
        panic!("one parameter");
    };
    assert_eq!(name, "m");
    declaration_of(value)
}

/// FR-359-AC-8, TC-1001 steps 1 and 3 over the operation path: a
/// `Ledger.pay(m: Money): Money` invocation whose argument and result are a
/// `TaxedMoney` is admitted with both keeping their own type, and the
/// precondition and postcondition over `m` and `result` hold. The same
/// argument at `payTaxed(m: TaxedMoney)` admits; a `Money` or an unrelated
/// `Fee` there refuses `wrong-value-kind` naming parameter `m`.
#[trace("FR-359-AC-8", "TC-1001")]
#[test]
fn a_model_operation_takes_and_returns_a_child_record() {
    let package = package(OPERATION_CLAUSES);
    let key_of = |name: &str| declaration(&package, name);

    let pre = admit_invocation(
        &package,
        "PayHolds",
        "pay",
        taxed_json(100),
        Some(taxed_json(100)),
    )
    .expect("a TaxedMoney argument admits at pay(m: Money)");
    assert_eq!(parameter_declaration(&pre), key_of("TaxedMoney"));
    assert!(holds(&package, "PayHolds", &pre));

    let post = admit_invocation(
        &package,
        "PayReturns",
        "pay",
        taxed_json(100),
        Some(taxed_json(100)),
    )
    .expect("a TaxedMoney result admits at a Money result");
    assert_eq!(parameter_declaration(&post), key_of("TaxedMoney"));
    let result = post.result.as_ref().expect("the declared result");
    assert_eq!(declaration_of(result), key_of("TaxedMoney"));
    assert!(holds(&package, "PayReturns", &post));

    // The child is also the parameter of a TaxedMoney-typed operation.
    let taxed_only = admit_invocation(
        &package,
        "PayTaxedHolds",
        "payTaxed",
        taxed_json(7),
        Some(money_json(7)),
    )
    .expect("a TaxedMoney admits at payTaxed(m: TaxedMoney)");
    assert!(holds(&package, "PayTaxedHolds", &taxed_only));

    for refused in [money_json(7), record_json("Fee", &[("fee_minor", 1)])] {
        let Err(AdmissionFailure::Refused(record)) = admit_invocation(
            &package,
            "PayTaxedHolds",
            "payTaxed",
            refused,
            Some(money_json(7)),
        ) else {
            panic!("a Money or a Fee at a TaxedMoney parameter refuses");
        };
        assert_eq!(record.cause, "wrong-value-kind");
        assert_eq!(record.fields.get("field").map(String::as_str), Some("m"));
    }
}

/// FR-106-AC-15: a record defect in a parameter or the result names the
/// invocation binding as the root of its path: `m` or `result`, then the
/// nested field.
#[trace("FR-359-AC-8", "FR-106-AC-15", "TC-1001")]
#[test]
fn an_operation_record_defect_names_the_binding_and_the_nested_field() {
    let package = package(OPERATION_CLAUSES);
    let without_tax = record_json("TaxedMoney", &[("amount_minor", 1), ("currency", 978)]);
    let refusal = |result: Result<AdmittedObservations, AdmissionFailure>| match result {
        Err(AdmissionFailure::Refused(record)) => (
            record.cause.to_owned(),
            record.fields.get("field").cloned().unwrap_or_default(),
            record.fields.get("path").cloned().unwrap_or_default(),
        ),
        other => panic!("a refusal, not {other:?}"),
    };
    let expected = |cause: &str, field: &str, path: &str| {
        (cause.to_owned(), field.to_owned(), path.to_owned())
    };
    assert_eq!(
        refusal(admit_invocation(
            &package,
            "PayTaxedHolds",
            "payTaxed",
            without_tax.clone(),
            Some(money_json(1))
        )),
        expected("missing-member", "m", "tax_minor")
    );
    assert_eq!(
        refusal(admit_invocation(
            &package,
            "PayReturns",
            "pay",
            money_json(1),
            Some(without_tax)
        )),
        expected("missing-member", "result", "tax_minor")
    );
    assert_eq!(
        refusal(admit_invocation(
            &package,
            "PayReturns",
            "pay",
            money_json(1),
            Some(record_json("Fee", &[("fee_minor", 1)]))
        )),
        expected("wrong-value-kind", "result", "")
    );
}
