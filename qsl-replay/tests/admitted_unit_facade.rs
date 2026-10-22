// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1092 (FR-398-AC-2, FR-398-AC-3): `compile_admitted`'s context holds
//! every dimension and declared unit the check admitted, with the ids the
//! spine's own checked package gives them, and refuses as typed. Only the
//! oracle that produces the expected ids runs the spine directly. FR-398-AC-1
//! (the crate root and the kernel alone) is `admitted_unit_root_only.rs`.

use ix_trace_rs::trace;
use qsl_foundation::Setting;
use qsl_replay::spine::{self, ParseRequest, SpineLimits};
use qsl_replay::{
    compile_admitted, AdmittedCompilation, AdmittedLookupRefusal, DependencyInput, NodeKey,
    ReplayLimits, SourceIdentity, StageLimits, SuppliedLibrary, WireNodeId,
};
use std::collections::BTreeSet;

use qsl_replay::AdmittedUnit;
use qsl_semantics::check::{CheckedGraph, NominalNode};
use quire_exact::{
    Cancel, Integer, Meter, Outcome, Quantity, Rational, ScalarLimits, UnitDomain, UnitId,
    ValueType,
};
use quire_semantic_value::quantity::{convert_quantity, ConvertedValue, QuantityTarget};

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
// Two units of one dimension, one of another, an enum and a record (the nodes
// a unit lookup refuses as the wrong kind), and a function whose parameters
// name each unit as a type.
const UNIT: &str = "dimension Length;\n\
     dimension Time;\n\
     unit m : Length = rational(1, 1);\n\
     unit km : Length = rational(1000, 1) * m;\n\
     unit s : Time = rational(1, 1);\n\
     enum Color { BLUE, RED }\n\
     record Box { a: Boolean; }\n\
     function f using v(a: Quantity<m>, b: Quantity<km>, c: Quantity<s>): Boolean pure { true }\n";
const LIBRARY: &str = "dimension Mass;\nunit kg : Mass = rational(1, 1);\n\
     function g using v(): Boolean pure { true }\n";

fn unit_identity() -> SourceIdentity {
    SourceIdentity::new("a", "u", "git", "1")
}

fn limits() -> StageLimits {
    [(Setting::S1InputBytes, 1 << 20)].into_iter().collect()
}

fn wire(key: NodeKey) -> WireNodeId {
    WireNodeId::from_digest(*key.as_bytes())
}

fn admit(text: &str, dependencies: &DependencyInput) -> AdmittedCompilation {
    compile_admitted(
        unit_identity(),
        "unit.native",
        text.as_bytes(),
        [],
        dependencies,
        limits(),
        ReplayLimits::default(),
    )
    .expect("the unit compiles")
}

fn library(identity: &str, name: &str, body: &str) -> SuppliedLibrary {
    SuppliedLibrary {
        identity: identity.to_owned(),
        source: SourceIdentity::new("a", name, "git", "1"),
        path: format!("{name}.native"),
        bytes: format!("{HEADER}{body}").into_bytes(),
    }
}

/// Run `read` over the spine's own checked graph of `text`.
fn with_spine_graph<T>(text: &str, read: impl FnOnce(&CheckedGraph) -> T) -> T {
    let source = unit_identity();
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("parses")
    .into_value();
    let models = spine::select(&parsed, &Default::default(), limits.model, &cancel)
        .expect("selects")
        .into_value();
    let checked = spine::check(
        &parsed,
        &models,
        &DependencyInput::default(),
        &spine::LockEvidence::default(),
        limits,
        &cancel,
    )
    .expect("checks")
    .into_value();
    read(checked.package().graph())
}

/// What the spine's own checked package says of the unit text, read without
/// the unit table the facade reads: each unit's node id and last name segment
/// from the semantic graph's unit nodes, each dimension's likewise, and the
/// unit id the checker gave each `Quantity<u>` parameter of `f`.
struct Expected {
    units: Vec<(NodeKey, String)>,
    dimensions: Vec<(NodeKey, String)>,
    parameters: Vec<UnitId>,
}

fn spine_expected(graph: &CheckedGraph) -> Expected {
    let (mut units, mut dimensions) = (Vec::new(), Vec::new());
    for node in graph.semantic_graph().nodes() {
        match node.nominal() {
            Some(NominalNode::Unit(Some(preimage))) => units.push((
                node.key(),
                preimage
                    .qualified_declaration()
                    .last()
                    .expect("a name")
                    .clone(),
            )),
            Some(NominalNode::Dimension(Some(preimage))) => dimensions.push((
                node.key(),
                preimage
                    .qualified_declaration()
                    .last()
                    .expect("a name")
                    .clone(),
            )),
            _ => {}
        }
    }
    units.sort();
    dimensions.sort();
    let parameters = graph
        .callable("f")
        .expect("the function")
        .parameters
        .iter()
        .map(|(_, value_type)| match value_type {
            ValueType::Quantity(unit) => *unit,
            other => panic!("a quantity parameter, got {other:?}"),
        })
        .collect();
    Expected {
        units,
        dimensions,
        parameters,
    }
}

fn unit_named<'a>(admitted: &'a AdmittedCompilation, name: &str) -> &'a qsl_replay::AdmittedUnit {
    admitted
        .context()
        .units()
        .find(|unit| unit.qualified_name().last().map(|n| n.as_str()) == Some(name))
        .expect("the unit is admitted")
}

/// The context holds exactly the spine's units and dimensions; each unit's id
/// is the one the checker gave its `Quantity<u>` parameter; ids are distinct and stable.
#[trace("TC-1092", "FR-398-AC-2")]
#[test]
fn unit_ids_are_the_spines_distinct_and_stable() {
    let text = format!("{HEADER}{UNIT}");
    let expected = with_spine_graph(&text, spine_expected);
    let names: Vec<_> = expected.units.iter().map(|(_, n)| n.as_str()).collect();
    assert_eq!(names.len(), 3);

    let admitted = admit(&text, &DependencyInput::default());
    let context = admitted.context();
    let table: Vec<(NodeKey, String)> = context
        .units()
        .map(|unit| {
            (
                unit.node(),
                unit.qualified_name()
                    .last()
                    .expect("a name")
                    .as_str()
                    .to_owned(),
            )
        })
        .collect();
    assert_eq!(table, expected.units);
    let dimensions: Vec<(NodeKey, String)> = context
        .dimensions()
        .map(|dimension| {
            (
                dimension.node(),
                dimension
                    .qualified_name()
                    .last()
                    .expect("a name")
                    .as_str()
                    .to_owned(),
            )
        })
        .collect();
    assert_eq!(dimensions, expected.dimensions);
    assert!(context.dimensions().all(|dimension| dimension.is_base()));

    // The ids are the ones the checked signature carries for `Quantity<m>`,
    // `Quantity<km>` and `Quantity<s>`, in that order.
    let ids = ["m", "km", "s"].map(|name| unit_named(&admitted, name).id());
    assert_eq!(ids.as_slice(), expected.parameters.as_slice());
    assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), 3);
    for name in ["m", "km", "s"] {
        let unit = unit_named(&admitted, name);
        // The kernel id wraps the node id's bytes in the declared-unit domain.
        assert_eq!(unit.id().domain(), UnitDomain::Declared, "{name}");
        assert_eq!(unit.id().as_bytes(), unit.node().as_bytes(), "{name}");
    }

    // The dimension each unit names is the dimension node of that name.
    let length = expected
        .dimensions
        .iter()
        .find(|(_, n)| n == "Length")
        .unwrap()
        .0;
    let time = expected
        .dimensions
        .iter()
        .find(|(_, n)| n == "Time")
        .unwrap()
        .0;
    assert_eq!(unit_named(&admitted, "m").dimension(), length);
    assert_eq!(unit_named(&admitted, "km").dimension(), length);
    assert_eq!(unit_named(&admitted, "s").dimension(), time);

    // Conversion: same dimension converts, either way; another does not.
    let (m, km, s) = (
        unit_named(&admitted, "m"),
        unit_named(&admitted, "km"),
        unit_named(&admitted, "s"),
    );
    assert!(m.converts_to(km) && km.converts_to(m));
    assert!(!m.converts_to(s) && !s.converts_to(km));

    for (node, ..) in &expected.units {
        let first = context.unit_declaration(wire(*node)).expect("found");
        assert_eq!(first, context.unit_declaration(wire(*node)).expect("found"));
    }
    let recompiled = admit(&text, &DependencyInput::default());
    assert_eq!(
        recompiled.context().units().cloned().collect::<Vec<_>>(),
        context.units().cloned().collect::<Vec<_>>()
    );
}

/// The context's unit table resolves every admitted unit by its kernel id.
#[trace("TC-1092", "FR-398-AC-2")]
#[test]
fn the_unit_table_holds_every_admitted_unit_by_id() {
    let admitted = admit(&format!("{HEADER}{UNIT}"), &DependencyInput::default());
    let context = admitted.context();
    for unit in context.units() {
        assert_eq!(
            context.unit_table().get(unit.id()),
            Some(unit.quantity_unit())
        );
    }
    assert_eq!(context.unit_table().ids().count(), 3);
}

/// An id nobody admitted is `Unknown`; a dimension, an enum and a record of
/// this package are `WrongKind` for a unit lookup, and a unit is `WrongKind`
/// for a dimension lookup.
#[trace("TC-1092", "FR-398-AC-3")]
#[test]
fn unit_lookups_refuse_unknown_and_wrong_kind_ids() {
    let admitted = admit(&format!("{HEADER}{UNIT}"), &DependencyInput::default());
    let context = admitted.context();

    let nobody = WireNodeId::from_digest([7; 32]);
    let unknown = Err(AdmittedLookupRefusal::Unknown {
        declaration: nobody,
        case: None,
    });
    assert_eq!(
        context.unit_declaration(nobody),
        unknown.clone().map(|_: ()| unreachable!())
    );
    assert!(matches!(
        context.dimension_declaration(nobody),
        Err(AdmittedLookupRefusal::Unknown { .. })
    ));

    let dimension = wire(context.dimensions().next().expect("a dimension").node());
    let color = wire(context.enums().next().expect("the enum").key());
    let record = wire(context.composites().next().expect("the record").node());
    for id in [dimension, color, record] {
        assert!(
            matches!(
                context.unit_declaration(id),
                Err(AdmittedLookupRefusal::WrongKind { declaration }) if declaration == id
            ),
            "{id:?}"
        );
    }
    let unit = wire(unit_named(&admitted, "m").node());
    assert!(matches!(
        context.dimension_declaration(unit),
        Err(AdmittedLookupRefusal::WrongKind { declaration }) if declaration == unit
    ));
    // The record and enum lookups refuse a unit as the wrong kind too.
    assert!(matches!(
        context.composite_declaration(unit),
        Err(AdmittedLookupRefusal::WrongKind { .. })
    ));
}

/// A unit a dependency admitted is `Foreign`, naming that dependency; the
/// unit's own units are found.
#[trace("TC-1092", "FR-398-AC-3")]
#[test]
fn a_dependencys_unit_is_foreign() {
    let mass = library("test/mass", "mass", LIBRARY);
    let alone = compile_admitted(
        mass.source.clone(),
        &mass.path,
        &mass.bytes,
        [],
        &DependencyInput::default(),
        limits(),
        ReplayLimits::default(),
    )
    .expect("the library compiles");
    let kg = alone.context().units().next().expect("the unit");
    let kg_id = wire(kg.node());
    let mass_dimension = wire(
        alone
            .context()
            .dimensions()
            .next()
            .expect("dimension")
            .node(),
    );
    let package = alone.context().package_id();
    let mid = library(
        "test/mid",
        "mid",
        "import \"test/mass\" as p;\nfunction h using v(): Boolean pure { true }\n",
    );

    for (import, dependencies) in [
        ("test/mass", vec![mass.clone()]),
        ("test/mid", vec![mass.clone(), mid.clone()]),
    ] {
        let text = format!("{HEADER}import \"{import}\" as d;\n{UNIT}");
        let admitted = admit(
            &text,
            &DependencyInput::new(dependencies).expect("a dependency input"),
        );
        let context = admitted.context();
        assert_eq!(
            context.unit_declaration(kg_id).map(|_| ()),
            Err(AdmittedLookupRefusal::Foreign {
                declaration: kg_id,
                package,
            })
        );
        assert_eq!(
            context.dimension_declaration(mass_dimension).map(|_| ()),
            Err(AdmittedLookupRefusal::Foreign {
                declaration: mass_dimension,
                package,
            })
        );
        assert_eq!(context.units().count(), 3);
        for own in context.units() {
            assert_eq!(context.unit_declaration(wire(own.node())), Ok(own));
        }
    }
}

fn find_unit<'a>(admitted: &'a AdmittedCompilation, name: &str) -> &'a AdmittedUnit {
    unit_named(admitted, name)
}

/// A declared unit the package never uses is not a node of the checked graph,
/// yet it is listed and found, and every other kind's lookup refuses it
/// `WrongKind` (the enum lookups, which never refuse `WrongKind`, say
/// `Unknown`). A dimension no unit names is not listed.
#[trace("TC-1092", "FR-398-AC-3")]
#[test]
fn an_unused_unit_is_found_and_a_dimension_no_unit_names_is_not_listed() {
    let text = format!(
        "{HEADER}dimension Length;\ndimension Spare;\n\
         unit m : Length = rational(1, 1);\n\
         unit unused : Length = rational(2, 1) * m;\n\
         function f using v(a: Quantity<m>): Boolean pure {{ true }}\n"
    );
    let expected = with_spine_graph(&text, spine_expected);
    let used: Vec<_> = expected
        .units
        .iter()
        .map(|(_, name)| name.as_str())
        .collect();
    assert_eq!(used, ["m"], "the spine's graph holds only the used unit");

    let admitted = admit(&text, &DependencyInput::default());
    let context = admitted.context();
    let unused = find_unit(&admitted, "unused");
    assert!(expected
        .units
        .iter()
        .all(|(node, _)| *node != unused.node()));
    let id = wire(unused.node());
    assert_eq!(context.unit_declaration(id), Ok(unused));
    assert_eq!(context.units().count(), 2);
    assert!(matches!(
        context.composite_declaration(id),
        Err(AdmittedLookupRefusal::WrongKind { .. })
    ));
    assert!(matches!(
        context.dimension_declaration(id),
        Err(AdmittedLookupRefusal::WrongKind { .. })
    ));
    assert!(matches!(
        context.enum_declaration(id),
        Err(AdmittedLookupRefusal::Unknown { .. })
    ));
    let names: Vec<_> = context
        .dimensions()
        .map(|dimension| dimension.qualified_name().last().expect("a name").as_str())
        .collect();
    assert_eq!(names, ["Length"], "Spare is named by no unit");
}

/// A derived dimension is listed with its parts, a chain of scales converts
/// through the root, and an offset unit converts with its offset.
#[trace("TC-1092", "FR-398-AC-2")]
#[test]
fn derived_dimensions_chains_and_offsets_convert() {
    let text = format!(
        "{HEADER}dimension Length;\ndimension Time;\ndimension Temperature;\n\
         dimension Speed = Length / Time;\n\
         unit m : Length = rational(1, 1);\n\
         unit km : Length = rational(1000, 1) * m;\n\
         unit mm : Length = rational(1, 1000) * m;\n\
         unit s : Time = rational(1, 1);\n\
         unit mps : Speed = rational(1, 1);\n\
         unit K : Temperature = rational(1, 1);\n\
         unit C : Temperature = rational(1, 1) * K + decimal(27315, 2);\n\
         function f using v(a: Quantity<km>, b: Quantity<mm>, c: Quantity<mps>, \
         d: Quantity<C>, e: Quantity<K>, g: Quantity<s>): Boolean pure {{ true }}\n"
    );
    let admitted = admit(&text, &DependencyInput::default());
    let context = admitted.context();

    let dimension = |name: &str| {
        context
            .dimensions()
            .find(|d| d.qualified_name().last().map(|n| n.as_str()) == Some(name))
            .expect("the dimension is listed")
    };
    let speed = dimension("Speed");
    assert!(!speed.is_base());
    let parts: Vec<_> = speed
        .terms()
        .iter()
        .map(|(node, exponent)| {
            let base = context
                .dimension_declaration(wire(*node))
                .expect("a part is a listed dimension");
            (
                base.qualified_name()
                    .last()
                    .expect("a name")
                    .as_str()
                    .to_owned(),
                exponent.to_string(),
            )
        })
        .collect();
    assert_eq!(
        parts.iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([
            &("Length".to_owned(), "1".to_owned()),
            &("Time".to_owned(), "-1".to_owned())
        ]),
    );
    assert!(dimension("Length").is_base() && dimension("Length").terms().is_empty());
    assert_eq!(context.dimensions().count(), 4);

    let convert = |from: &str, magnitude: Rational, to: &str| {
        let (from, to) = (find_unit(&admitted, from), find_unit(&admitted, to));
        let quantity = Quantity::new(magnitude, from.id());
        let source = context.unit_table().resolve(&quantity).expect("resolves");
        let Outcome::Completed(done) = convert_quantity(
            source,
            to.quantity_unit(),
            &QuantityTarget::Exact,
            &mut meter(),
        )
        .expect("compatible") else {
            panic!("completes");
        };
        done.value().clone()
    };
    let exact = |n: i64, d: i64| {
        ConvertedValue::Exact(Rational::new(Integer::from(n), Integer::from(d)).unwrap())
    };
    let int = |n: i64| Rational::from_integer(Integer::from(n));
    // km -> mm passes through the root m: 2 km = 2,000,000 mm.
    assert_eq!(convert("km", int(2), "mm"), exact(2_000_000, 1));
    // 0 C = 5463/20 K.
    assert_eq!(convert("C", int(0), "K"), exact(5463, 20));
    assert!(!find_unit(&admitted, "mps").converts_to(find_unit(&admitted, "m")));
}

fn meter() -> Meter {
    Meter::new(ScalarLimits {
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
    })
}
