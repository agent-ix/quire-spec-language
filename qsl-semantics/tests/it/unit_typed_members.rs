// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Unit-typed members": `Quantity<u>` is a type in every position
//! that takes a type, each typed by the unit's own nominal node.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    AggregateTerm, AssemblyCause, BindingValue, BodyTerm, CheckCause, CheckedGraph, GroupMember,
    LeafTerm, MemberTerm, SemanticNode, TupleMember,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::admit_unit;
use quire_checked::key::DeclarationKey;
use quire_exact::{IllTypedCause, NodeKey, ValueType};
use serde_json::{json, Value};

use crate::model_operations::{
    admit_and_assemble_with_body, config_unit, config_unit_with_profiles,
    config_version_document_with_operations, empty_frame, operation, operation_parameter,
    parse_and_build,
};

/// A structural node key for a fixture, distinct per `tag`.
fn fixture_key(tag: u8) -> quire_exact::NodeKey {
    quire_exact::NodeKey::new(
        "fixture",
        vec![quire_exact::PathSegment::Text(format!("node{tag}"))],
        0,
    )
    .expect("a valid fixture key")
}

const UNITS: &str = "dimension Time;\nunit s : Time = rational(1, 1);\n\
                     unit ms : Time = rational(1, 1000) * s;\n";

/// `ConfigVersion` with the operation `measure(d: ms): ms`, its parameter
/// and result typed `ix://quire/unit/ms`.
fn document() -> Vec<u8> {
    config_version_document_with_operations(vec![operation(
        "measure",
        json!([operation_parameter("measure", "d", "ix://quire/unit/ms")]),
        Some("ix://quire/unit/ms"),
        empty_frame(),
    )])
}

/// The node a leaf references, when it references one.
fn leaf_target(leaf: &LeafTerm, found: &mut Vec<NodeKey>) {
    if let LeafTerm::Reference { target } = leaf {
        found.extend(target.admitted().cloned());
    }
}

/// The referenced node of each reference leaf under `term`, in order.
fn targets(term: &MemberTerm) -> Vec<NodeKey> {
    let mut found = Vec::new();
    let group = |group: &qsl_semantics::check::GroupTerm, found: &mut Vec<NodeKey>| {
        for member in &group.members {
            match member {
                GroupMember::Leaf(leaf) => leaf_target(leaf, found),
                GroupMember::Binding(binding) => leaf_target(&binding.value, found),
            }
        }
    };
    match term {
        MemberTerm::Leaf(leaf) => leaf_target(leaf, &mut found),
        MemberTerm::Group(inner) => group(inner, &mut found),
        MemberTerm::Binding(binding) => match &binding.value {
            BindingValue::Leaf(leaf) => leaf_target(leaf, &mut found),
            BindingValue::Group(inner) => group(inner, &mut found),
            BindingValue::Tuple(tuple) => {
                for member in &tuple.members {
                    match member {
                        TupleMember::Leaf(leaf) => leaf_target(leaf, &mut found),
                        TupleMember::Group(inner) => group(inner, &mut found),
                    }
                }
            }
            _ => {}
        },
    }
    found
}

/// The members of a node whose body is an aggregate.
fn members(node: &SemanticNode) -> &[MemberTerm] {
    let BodyTerm::Aggregate(AggregateTerm { members }) = node.body() else {
        panic!("an aggregate body: {:?}", node.body());
    };
    members
}

/// The referenced nodes of an `Option` or `Set` node's body.
fn body_targets(node: &SemanticNode) -> Vec<NodeKey> {
    match node.body() {
        BodyTerm::Leaf(leaf) => {
            let mut found = Vec::new();
            leaf_target(leaf, &mut found);
            found
        }
        BodyTerm::Aggregate(AggregateTerm { members }) => {
            members.iter().flat_map(targets).collect()
        }
        other => panic!("a leaf or aggregate body: {other:?}"),
    }
}

/// Part of FR-230-AC-15 (TC-685 step 17): with dimension `Time`, root `s` and `ms`
/// of scale `1/1000`, a record field, a tuple element, a union payload, an
/// `Option` and a `Set` of `Quantity<ms>`, a function parameter and result,
/// a predicate parameter of the alias `Span = Quantity<ms>`, and an
/// operation parameter and result typed `ix://quire/unit/ms` each check.
/// Every position is typed by the one nominal unit node of `ms`; the alias
/// has no node, the tuple element and union payload reference the unit node
/// itself, and `Option`'s composite node holds the unit node. `f(3 s)`
/// refuses `ill_typed`/`type-mismatch` at `3 s`, and `record R { d: Quantity<Time> }`
/// `missing_declaration`/`missing-name`. Not covered: the AC's
/// `Map<Text, Quantity<ms>>`, which source cannot write (STD-493 item 12).
#[trace("TC-685")]
#[test]
fn quantity_types_check_in_every_position_over_the_one_unit_node() {
    let body = format!(
        "{UNITS}record Window {{ d: Quantity<ms>; n: Integer; o: Option<Quantity<ms>>; \
             marks: Set<Quantity<ms>>; }}\n\
         type Span = Quantity<ms>;\n\
         tuple Pair(Quantity<ms>, Integer);\n\
         union Bound {{ Within(Quantity<ms>), Open }}\n\
         function f using v(d: Quantity<ms>): Quantity<ms> pure {{ d }}\n\
         predicate p using v(d: Span): Boolean {{ d = 3 ms }}\n"
    );
    let document = document();
    let graph = admit_and_assemble_with_body(&document, &body)
        .expect("every position assembles")
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("every position checks");
    let semantic = graph.semantic_graph();
    let named = |name: &str| {
        semantic.nodes().find(|node| {
            node.declaration()
                .is_some_and(|path| path.len() == 1 && path[0].as_str() == name)
        })
    };
    let unit = named("ms").expect("the nominal unit node of ms").key();
    assert!(named("Span").is_none(), "an alias has no node");
    let window = named("Window").expect("the Window record");
    let fields = members(window);
    assert_eq!(
        targets(&fields[0]),
        std::slice::from_ref(&unit),
        "d is typed by ms"
    );
    let option = targets(&fields[2]);
    let [option] = option.as_slice() else {
        panic!("o is typed by one composite: {option:?}");
    };
    let option = semantic.node(option.clone()).expect("the Option node");
    assert_eq!(option.semantic_form(), "option");
    assert_eq!(body_targets(option), std::slice::from_ref(&unit));
    let set = targets(&fields[3]);
    let [set] = set.as_slice() else {
        panic!("marks is typed by one composite: {set:?}");
    };
    let set = semantic.node(set.clone()).expect("the Set node");
    assert_eq!(body_targets(set), std::slice::from_ref(&unit));
    for (name, position) in [("Pair", 0), ("Bound", 0)] {
        let node = named(name).expect("the composite");
        assert_eq!(
            targets(&members(node)[position]),
            std::slice::from_ref(&unit),
            "{name}"
        );
    }
    for function in ["f", "p"] {
        let node = named(function).expect("the function");
        let parameters = node.function_parameters().expect("a function node");
        let parameter = semantic
            .node(parameters[0].clone())
            .expect("the parameter node");
        assert_eq!(
            parameter.semantic_type(),
            Some(unit.clone()),
            "{function}'s parameter is typed by ms"
        );
    }

    // The operation's parameter and result read as `Quantity<ms>`.
    let (unit_text, packages) = config_unit(&document);
    let built = parse_and_build(&unit_text);
    let views = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package admits");
    let owner = views[0].view.type_identities()[&DeclarationKey {
        package: "example/config-version".to_owned(),
        node: "ix://example/config-version/ConfigVersion".to_owned(),
    }];
    let measure = graph
        .scope()
        .operations()
        .declared_by(owner)
        .iter()
        .find(|operation| operation.name() == "measure")
        .expect("measure is declared");
    let [(_, parameter)] = measure.parameters() else {
        panic!("one parameter");
    };
    // The operation's parameter and result are the one type `f`'s
    // parameter is, `Quantity` of the `ms` unit node (above).
    let f = graph.callable("f").expect("f is callable");
    assert_eq!(
        parameter, &f.parameters[0].1,
        "measure's parameter is Quantity<ms>"
    );
    assert!(matches!(parameter, ValueType::Quantity(_)), "{parameter:?}");
    assert_eq!(measure.result(), Some(parameter));

    // A quantity of another unit of the same dimension does not stand for
    // `Quantity<ms>`: refused at the argument `3 s`.
    let mismatched = format!(
        "{UNITS}function f using v(d: Quantity<ms>): Quantity<ms> pure {{ d }}\n\
         function g using v(): Quantity<ms> pure {{ f(3 s) }}\n"
    );
    let declarations = admit_and_assemble_with_body(&document, &mismatched).expect("assembles");
    let regions = declarations.regions();
    let (text, _) = config_unit_with_profiles(&document, "", &mismatched);
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("3 s is not a Quantity<ms>");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
    let region = regions.refusal_region(refusal).expect("a region");
    assert_eq!(&text[region.start() as usize..region.end() as usize], "3 s");

    // `Time` is a dimension, not a unit.
    let refusal = admit_and_assemble_with_body(
        &document,
        &format!("{UNITS}record R {{ d: Quantity<Time>; }}\n"),
    )
    .expect_err("Quantity<Time> names no unit");
    let [error] = refusal.errors.as_slice() else {
        panic!("one error: {:?}", refusal.errors);
    };
    assert_eq!(
        error.cause,
        AssemblyCause::UnresolvedTypeName {
            name: "Time".to_owned()
        }
    );
    assert_eq!(error.cause.code(), Code::MissingDeclaration);
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "missing_declaration/missing-name"
    );
}

/// `ix://example/config-version/<name>`.
fn node(name: &str) -> String {
    format!("ix://example/config-version/{name}")
}

/// A `constraints[]` entry `keyword` `value` on `applies`, owned by `owner`.
fn bound(owner: &str, keyword: &str, value: &str, applies: &str) -> Value {
    let identity = format!("{}/{keyword}", node(owner));
    json!({
        "identity": identity.clone(), "keyword": keyword,
        "operands": {"value": value}, "appliesTo": applies,
        "diagnosticCode": format!("bound.{keyword}"),
        "origin": {"generated": {"generatorIdentity": identity.clone(),
            "generatorVersion": "1.0.0", "inputIdentities": [identity]}},
    })
}

/// [`document`] with an alias `D` of `ms` carrying `on_alias`, the
/// `ConfigVersion` constraints `on_type`, and the optional fields `a: ms`,
/// `d: D`, `ds: Sequence<D>` (multiplicity `[0, 2]`) and `t: s`.
fn bounded_document(on_alias: Vec<Value>, on_type: Vec<Value>) -> Vec<u8> {
    use crate::model_operations::{wire_field, wire_type};
    let mut document: Value = serde_json::from_slice(&document()).expect("JSON");
    let types = document["types"].as_array_mut().expect("types");
    let mut alias = wire_type(
        &node("D"),
        "unused",
        json!({"target": "ix://quire/unit/ms", "constraints": on_alias}),
    );
    alias["kind"] = json!("alias");
    let owner = types
        .iter_mut()
        .find(|definition| definition["identity"] == node("ConfigVersion"))
        .expect("ConfigVersion");
    owner["constraints"] = json!(on_type);
    let fields = owner["fields"].as_array_mut().expect("fields");
    let member = |name: &str, type_ref: &str| {
        wire_field(
            &node(&format!("ConfigVersion/{name}")),
            name,
            type_ref,
            "optional",
            0,
        )
    };
    fields.push(member("a", "ix://quire/unit/ms"));
    fields.push(member("d", &node("D")));
    let mut ds = member("ds", &node("D"));
    ds["presence"] = json!("required");
    ds["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
    fields.push(ds);
    fields.push(member("t", "ix://quire/unit/s"));
    types.push(alias);
    serde_json::to_vec(&document).expect("JSON")
}

/// Part of FR-230-AC-26 (TC-685) and QSpec FR-154-AC-29, the intake and
/// run-time halves of "Unit bounds": a `min` `"5"` on the alias `D` of `ms`
/// and a `max` `"3"` on `ms` itself (on `ConfigVersion`) are admitted; the
/// `ms` bound bounds `a`, `d` and `ds`, the `D` bound only `d` and `ds`,
/// and neither bounds `t: s`. A sequence of `D` is checked at each element:
/// `[6 ms, 2 ms]` violates the `min` at element 1, the `max` at element 0;
/// `3 ms` satisfies the `max` `3` and `4 ms` violates it.
/// `operands.value` `"1/0"`, `"0.5"`, `"2/4"` and `""`, a keyword
/// `minLength` on `D`, and a `min` `"5"` after a `max` `"3"` on `D` each
/// refuse `invalid_model_binding`/`malformed-declaration` at the
/// constraint. Not covered: the bounded quantity type and its static
/// literal refusal (QSpec FR-154-AC-31, which needs quire-exact's bounded
/// quantity value type, IR-1051), and the initial-state and successor
/// checks end to end, which wait on FR-106's Quantity snapshot form and on
/// EN-1 writing a quantity field.
#[trace("TC-685")]
#[test]
fn unit_bounds_bound_the_members_of_their_unit_or_alias() {
    use qsl_semantics::model::observation::PathStep;
    use qsl_semantics::model::state::StateModel;

    let admitted = bounded_document(
        vec![bound("D", "min", "5", &node("D"))],
        vec![bound("ConfigVersion", "max", "3", "ix://quire/unit/ms")],
    );
    let (unit_text, packages) = config_unit(&admitted);
    let built = parse_and_build(&unit_text);
    admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("the bounds admit: {refusal:?}"));
    let graph = admit_and_assemble_with_body(&admitted, UNITS)
        .expect("assembles")
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("checks");
    let model = StateModel::new(
        graph.model_selections(),
        graph.scope().types(),
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model views");
    let bounds = model.member_bounds();
    let mut read: Vec<(String, String, &str, String)> = bounds
        .iter()
        .map(|member| {
            (
                member
                    .member
                    .node
                    .rsplit('/')
                    .next()
                    .expect("a name")
                    .to_owned(),
                member.constraint.node.clone(),
                member.keyword(),
                member.canonical_bound(),
            )
        })
        .collect();
    read.sort();
    let on_ms = node("ConfigVersion/max");
    let on_d = node("D/min");
    let expected: Vec<(String, String, &str, String)> = vec![
        ("a".to_owned(), on_ms.clone(), "max", "3".to_owned()),
        // `measure`'s parameter `d` and its result, typed `ms`.
        ("d".to_owned(), on_ms.clone(), "max", "3".to_owned()),
        ("d".to_owned(), on_ms.clone(), "max", "3".to_owned()),
        ("d".to_owned(), on_d.clone(), "min", "5".to_owned()),
        ("ds".to_owned(), on_ms.clone(), "max", "3".to_owned()),
        ("ds".to_owned(), on_d.clone(), "min", "5".to_owned()),
        ("measure".to_owned(), on_ms.clone(), "max", "3".to_owned()),
    ];
    assert_eq!(read, expected);

    let ms = quire_exact::UnitId::declared(fixture_key(9));
    let quantity = |magnitude: i64| {
        quire_exact::Value::Quantity(quire_exact::Quantity::new(
            quire_exact::Rational::new(
                quire_exact::Integer::from(magnitude),
                quire_exact::Integer::from(1_i64),
            )
            .expect("a rational"),
            ms,
        ))
    };
    let sequence = quire_exact::from_admitted(
        quire_exact::CollectionType::new(
            quire_exact::CollectionKind::Sequence,
            ValueType::Quantity(ms),
            None,
        ),
        vec![quantity(6), quantity(2)],
    );
    let at = |constraint: &str| {
        bounds
            .iter()
            .find(|member| {
                member.constraint.node == constraint && member.member.node.ends_with("/ds")
            })
            .expect("a ds bound")
            .violation(&sequence, graph.scope().types())
    };
    assert_eq!(at(&on_d), Some(vec![PathStep::Index(1)]));
    assert_eq!(at(&on_ms), Some(vec![PathStep::Index(0)]));
    let on_a = bounds
        .iter()
        .find(|member| member.member.node.ends_with("/a"))
        .expect("a's bound");
    assert_eq!(
        on_a.violation(&quantity(3), graph.scope().types()),
        None,
        "equality satisfies"
    );
    assert_eq!(
        on_a.violation(&quantity(4), graph.scope().types()),
        Some(vec![])
    );

    let malformed = |on_alias: Vec<Value>| {
        let document = bounded_document(on_alias, vec![]);
        let (unit_text, packages) = config_unit(&document);
        let built = parse_and_build(&unit_text);
        let refusal = admit_unit(
            &built.selections().models,
            &packages,
            ModelNormalizationLimits::default(),
        )
        .expect_err("the constraint refuses");
        format!("{refusal:?}")
    };
    for value in ["1/0", "0.5", "2/4", ""] {
        let refusal = malformed(vec![bound("D", "min", value, &node("D"))]);
        assert!(
            refusal.contains("InvalidModelBinding")
                && refusal.contains("constraints")
                && refusal.contains("operands"),
            "{value:?}: {refusal}"
        );
    }
    let refusal = malformed(vec![bound("D", "minLength", "3", &node("D"))]);
    assert!(
        refusal.contains("InvalidModelBinding") && refusal.contains("keyword"),
        "{refusal}"
    );
    let refusal = malformed(vec![
        bound("D", "max", "3", &node("D")),
        bound("D", "min", "5", &node("D")),
    ]);
    assert!(
        refusal.contains("InvalidModelBinding")
            && refusal.contains("constraints[1]")
            && refusal.contains("operands"),
        "{refusal}"
    );
}

/// Part of FR-230-AC-26 (TC-685) and QSpec FR-154-AC-29, SR-1681001 CR
/// FND-001, FND-003, FND-004 and SR-1681002 GA FND-002, FND-003: a `max`
/// `"3"` on `ms` bounds a quantity a record holds (`ConfigVersion.sp: Span
/// { d: ms }`, at the path `d`), and a `Span { d: 4 ms }` violates it at
/// `d`; a `min` `"5"` on `ms` on `ConfigVersion` and a `max` `"3"` on `ms`
/// on another object type `Other` refuse `malformed-declaration` at the
/// later constraint; with the alias `E` of the alias `D` of `ms`, a bound on
/// `D` bounds the members typed `D` and `E`, and one on `E` only those
/// typed `E`; an alias whose chain ends at a record type refuses
/// `malformed-declaration` at its `target`.
#[trace("TC-685")]
#[test]
fn unit_bounds_reach_records_aliases_of_aliases_and_every_node() {
    use crate::model_operations::{wire_field, wire_type};
    use qsl_semantics::model::observation::PathStep;
    use qsl_semantics::model::state::StateModel;

    let document = |edit: &dyn Fn(&mut Vec<Value>)| {
        let mut document: Value = serde_json::from_slice(&document()).expect("JSON");
        let types = document["types"].as_array_mut().expect("types");
        edit(types);
        let records = types
            .iter()
            .any(|definition| definition["kind"]["name"] == "value_record");
        if records {
            document["constructs"]
                .as_array_mut()
                .expect("constructs")
                .push(crate::model_operations::wire_construct(
                    "value_record",
                    qsl_semantics::model::intake::meaning::RECORD_VALUE_TYPE,
                    json!({}),
                ));
        }
        serde_json::to_vec(&document).expect("JSON")
    };
    let owner = |types: &mut Vec<Value>| -> usize {
        types
            .iter()
            .position(|definition| definition["identity"] == node("ConfigVersion"))
            .expect("ConfigVersion")
    };
    let optional = |name: &str, type_ref: &str| {
        wire_field(
            &node(&format!("ConfigVersion/{name}")),
            name,
            type_ref,
            "optional",
            0,
        )
    };
    let alias = |name: &str, target: &str, constraints: Vec<Value>| {
        let mut alias = wire_type(
            &node(name),
            "unused",
            json!({"target": target, "constraints": constraints}),
        );
        alias["kind"] = json!("alias");
        alias
    };
    let admitted = |document: &[u8]| {
        let (unit_text, packages) = config_unit(document);
        let built = parse_and_build(&unit_text);
        admit_unit(
            &built.selections().models,
            &packages,
            ModelNormalizationLimits::default(),
        )
        .map(|_| packages)
        .map_err(|refusal| format!("{refusal:?}"))
    };
    let bounds_of = |document: &[u8]| {
        let packages = admitted(document).unwrap_or_else(|refusal| panic!("admits: {refusal}"));
        let graph = admit_and_assemble_with_body(document, UNITS)
            .expect("assembles")
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .expect("checks");
        let model = StateModel::new(
            graph.model_selections(),
            graph.scope().types(),
            &packages,
            ModelNormalizationLimits::default(),
        )
        .expect("model views");
        (graph, model.member_bounds())
    };

    // A quantity a record holds.
    let in_record = document(&|types: &mut Vec<Value>| {
        let span = node("Span");
        let d = wire_field(
            &format!("{span}/d"),
            "d",
            "ix://quire/unit/ms",
            "required",
            1,
        );
        types.push(wire_type(&span, "value_record", json!({"fields": [d]})));
        let at = owner(types);
        types[at]["constraints"] =
            json!([bound("ConfigVersion", "max", "3", "ix://quire/unit/ms")]);
        types[at]["fields"]
            .as_array_mut()
            .expect("fields")
            .push(optional("sp", &span));
    });
    let (graph, bounds) = bounds_of(&in_record);
    let [on_sp] = &bounds
        .iter()
        .filter(|bound| bound.member.node.ends_with("/sp"))
        .collect::<Vec<_>>()[..]
    else {
        panic!("one bound on sp: {bounds:?}");
    };
    assert_eq!(on_sp.path, ["d".to_owned()]);
    let config = graph
        .scope()
        .types()
        .object_types()
        .map(|declaration| declaration.key())
        .find(|key| graph.scope().types().attribute(*key, "sp").is_some())
        .expect("ConfigVersion");
    let sp = graph
        .scope()
        .types()
        .attribute(config, "sp")
        .expect("sp")
        .field()
        .value_type()
        .clone();
    let span = match &sp {
        ValueType::Composite(span) => *span,
        ValueType::Option(inner) => match &**inner {
            ValueType::Composite(span) => *span,
            other => panic!("sp is a record: {other:?}"),
        },
        ValueType::Collection(collection) => match collection.element() {
            ValueType::Composite(span) => *span,
            other => panic!("sp is a record: {other:?}"),
        },
        other => panic!("sp is a record: {other:?}"),
    };
    let ms = quire_exact::UnitId::declared(fixture_key(9));
    let quantity = |magnitude: i64| {
        quire_exact::Value::Quantity(quire_exact::Quantity::new(
            quire_exact::Rational::new(
                quire_exact::Integer::from(magnitude),
                quire_exact::Integer::from(1_i64),
            )
            .expect("a rational"),
            ms,
        ))
    };
    let held = |magnitude: i64| {
        quire_exact::from_admitted_slots(
            span,
            vec![quire_exact::FieldValue::Present(quantity(magnitude))].into_boxed_slice(),
        )
    };
    let types = graph.scope().types();
    assert_eq!(
        on_sp.violation(&held(4), types),
        Some(vec![PathStep::Field("d".to_owned())])
    );
    assert_eq!(on_sp.violation(&held(3), types), None);

    // A crossed pair on two nodes.
    let crossed = document(&|types: &mut Vec<Value>| {
        let at = owner(types);
        types[at]["constraints"] =
            json!([bound("ConfigVersion", "min", "5", "ix://quire/unit/ms")]);
        types.push(wire_type(
            &node("Other"),
            "object_type",
            json!({"supertypes": [], "fields": [], "operations": [], "relationships": [],
                "constraints": [bound("Other", "max", "3", "ix://quire/unit/ms")]}),
        ));
    });
    let refusal = admitted(&crossed).expect_err("a crossed pair refuses");
    assert!(
        refusal.contains("InvalidModelBinding")
            && refusal.contains("operands")
            && refusal.contains(&node("Other")),
        "{refusal}"
    );

    // An alias of an alias.
    let chained = |on_d: Vec<Value>, on_e: Vec<Value>| {
        document(&move |types: &mut Vec<Value>| {
            types.push(alias("D", "ix://quire/unit/ms", on_d.clone()));
            types.push(alias("E", &node("D"), on_e.clone()));
            let at = owner(types);
            let fields = types[at]["fields"].as_array_mut().expect("fields");
            fields.push(optional("d", &node("D")));
            fields.push(optional("e", &node("E")));
        })
    };
    let members = |document: Vec<u8>| {
        let (_, bounds) = bounds_of(&document);
        let mut members: Vec<String> = bounds
            .iter()
            .map(|bound| {
                bound
                    .member
                    .node
                    .rsplit('/')
                    .next()
                    .expect("a name")
                    .to_owned()
            })
            .collect();
        members.sort();
        members
    };
    assert_eq!(
        members(chained(vec![bound("D", "max", "3", &node("D"))], vec![])),
        ["d", "e"]
    );
    assert_eq!(
        members(chained(vec![], vec![bound("E", "max", "3", &node("E"))])),
        ["e"]
    );
    let dangling = document(&|types: &mut Vec<Value>| {
        let span = node("Span");
        types.push(wire_type(&span, "value_record", json!({"fields": []})));
        types.push(alias("D", &span, vec![]));
    });
    let refusal = admitted(&dangling).expect_err("an alias of a record refuses");
    assert!(
        refusal.contains("InvalidModelBinding") && refusal.contains("target"),
        "{refusal}"
    );
}
