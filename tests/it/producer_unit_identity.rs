// SPDX-License-Identifier: AGPL-3.0-or-later
//! Producer-origin declared/compound identity and exact-handoff byte controls.

use std::collections::BTreeMap;
use std::fmt::Write;

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_replay::{AdmittedCompilation, CompileUnit, ReplayLimits, StageLimits};
use quire_exact::{Integer, Meter, Outcome, Quantity, Rational, ScalarLimits, UnitDomain};
use quire_semantic_value::quantity::{
    convert_quantity, evaluate_quantity_unit, HandoffRefusal, QuantityOperation, QuantityTarget,
    QuantityUnit, UnitScope,
};
use quire_semantic_value::semantic_node::IdentityRefusal;
use quire_semantic_value::unit::{DimensionNode, NominalUnitForm, UnitGraph, UnitNode};

const ROOTS: usize = 32;

fn source(roots: usize, expose_roots: bool) -> String {
    let mut source = String::from(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         dimension Length;\n\
         unit base : Length = rational(1, 1);\n",
    );
    for index in 0..roots {
        writeln!(
            source,
            "dimension D{index} = Length;\nunit r{index} : D{index} = rational(1, 1);"
        )
        .expect("writing a source string");
        if expose_roots {
            writeln!(source, "function touch{index} using v(x: Quantity<r{index}>): Quantity<r{index}> pure {{ x }}")
                .expect("writing a source function");
        }
    }
    source.push_str("function anchor using v(): Boolean pure { true }\n");
    source
}

fn compile(package_name: String, roots: usize, expose_roots: bool) -> AdmittedCompilation {
    let source = source(roots, expose_roots);
    qsl_replay::compile_admitted(
        CompileUnit {
            source: SourceIdentity::new("test", "producer-unit-identity"),
            package_name,
            path: "producer-unit-identity.native",
            bytes: source.as_bytes(),
        },
        [],
        &qsl_replay::spine::DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("the authentic root-unit source compiles")
}

/// Re-admit the producer's retained nominal facts through QSV's public graph
/// owner. Every key and original declaration comes from the same QSL context.
fn graph(compiled: &AdmittedCompilation) -> UnitGraph {
    let context = compiled.context();
    let table = context.unit_table();
    let mut dimensions = BTreeMap::new();
    let mut units = BTreeMap::new();
    for dimension in context.dimensions() {
        let nominal = table
            .nominal_node(dimension.node())
            .expect("retained dimension");
        let NominalUnitForm::Dimension { base, terms } = &nominal.form else {
            panic!("a dimension has its admitted nominal form")
        };
        dimensions.insert(
            nominal.key.clone(),
            if *base {
                DimensionNode::checked(terms.clone(), nominal.declaration.clone())
            } else {
                DimensionNode::checked_derived(terms.clone(), nominal.declaration.clone())
            }
            .expect("the producer checked the dimension"),
        );
    }
    for unit in context.units() {
        let nominal = table.nominal_node(unit.node()).expect("retained unit");
        let NominalUnitForm::Unit {
            dimension,
            target,
            scale,
            offset,
        } = &nominal.form
        else {
            panic!("a unit has its admitted nominal form")
        };
        units.insert(
            nominal.key.clone(),
            UnitNode::checked(
                dimension.clone(),
                target.clone(),
                scale.clone(),
                offset.clone(),
                nominal.declaration.clone(),
            )
            .expect("the producer checked the unit"),
        );
    }
    UnitGraph::from_checked_nodes(&dimensions, &units).expect("the producer's complete topology")
}

fn meter() -> Meter {
    Meter::new(ScalarLimits {
        integer_bits: 1024,
        decimal_digits: 1024,
        scale_expansion: 1024,
        text_input_bytes: 1024,
        text_scalars: 1024,
        normalized_scalars: 1024,
        unit_edges: 1024,
        value_occurrences: 1024,
        work_units: 100_000,
        result_units: 100_000,
    })
}

fn one() -> Rational {
    Rational::new(Integer::one(), Integer::one()).expect("one is rational")
}

#[trace("TC-411", "FR-088-AC-12")]
#[test]
fn admitted_declared_root_and_its_one_term_compound_keep_distinct_arms() {
    let compiled = compile("producer-unit-identity".to_owned(), 2, true);
    let context = compiled.context();
    let root = context.units().next().expect("an admitted root");
    let declared = Quantity::new(one(), root.id());
    let resolved = context
        .unit_table()
        .resolve(&declared)
        .expect("declared identity resolves");
    let (outcome, formed) = evaluate_quantity_unit(
        QuantityOperation::Power(resolved, &Integer::one()),
        &mut meter(),
    )
    .expect("power one preserves the admitted root");
    let Outcome::Completed(compound) = outcome else {
        panic!("power one completes")
    };
    assert_eq!(declared.unit().domain(), UnitDomain::Declared);
    assert_eq!(compound.unit().domain(), UnitDomain::Compound);
    assert_eq!(
        compound.unit().compound_terms(),
        Some([(root.node().clone(), Integer::one())].as_slice())
    );
    assert_eq!(declared.unit().declared_key(), Some(root.node()));
    assert_ne!(declared.unit(), compound.unit());
    assert!(declared.unit() < compound.unit());
    let mut scope = UnitScope::new(context.unit_table());
    scope.form(formed);
    let resolved = scope.resolve(&compound).expect("the compound arm resolves");
    assert!(matches!(resolved.unit(), QuantityUnit::Compound(_)));
    assert_eq!(compound.magnitude(), declared.magnitude());
}

fn conversion_handoff(compiled: &AdmittedCompilation, oversized: bool) {
    let context = compiled.context();
    let graph = graph(compiled);
    let mut roots: Vec<_> = context
        .units()
        .filter(|unit| {
            unit.qualified_name()
                .last()
                .is_some_and(|name| name.as_str().starts_with('r'))
        })
        .map(|unit| unit.node().clone())
        .collect();
    roots.sort();
    assert_eq!(roots.len(), ROOTS);
    assert!(
        roots
            .iter()
            .all(|root| root.view().package() == context.package_name().as_str()),
        "the admitted package name is retained in every genuine root identity"
    );
    let terms: Vec<_> = roots
        .iter()
        .enumerate()
        .map(|(index, key)| {
            (
                key.clone(),
                Integer::from(if index < ROOTS / 2 { 1_i64 } else { -1_i64 }),
            )
        })
        .collect();
    let target = QuantityUnit::Compound(
        graph
            .compound_unit(&terms)
            .expect("the admitted roots form a target"),
    );
    assert!(
        target.dimension().is_dimensionless(),
        "opposite exponents cancel the common base map"
    );
    let source_unit =
        QuantityUnit::Compound(graph.compound_unit(&[]).expect("dimensionless source"));
    let source_id = source_unit.id().expect("the empty compound identity fits");
    let mut table = context.unit_table().clone();
    table
        .insert(source_unit)
        .expect("dimensionless source retained");
    let source = Quantity::new(one(), source_id);
    let mut scope = UnitScope::new(&table);
    let mut meter = meter();
    let Outcome::Completed(converted) = convert_quantity(
        scope.resolve(&source).expect("the genuine source resolves"),
        &target,
        &QuantityTarget::Exact,
        &mut meter,
    )
    .expect("normalized dimensions agree") else {
        panic!("the exact conversion completes before handoff")
    };
    let before = meter.admission_count();
    let result = converted.identified_quantity();
    assert_eq!(
        meter.admission_count(),
        before,
        "handoff adds no accounting charge"
    );
    if oversized {
        let Err(HandoffRefusal::Identity(IdentityRefusal::InputBytes { bound, required })) = result
        else {
            panic!("the actual completed conversion must refuse the oversized identity")
        };
        assert_eq!(bound, 16_777_216);
        assert!(required > bound);
        eprintln!("native handoff identity refusal: bound={bound}, required={required}");
        assert!(
            scope.into_formed().ids().next().is_none(),
            "refusal yields no unit to retain"
        );
    } else {
        let (quantity, identified) = result.expect("small authentic target has an identity");
        assert_eq!(quantity.magnitude(), &one());
        assert_eq!(quantity.unit(), &identified.id());
        assert_eq!(scope.form(identified), *quantity.unit());
        assert_eq!(
            scope
                .resolve(&quantity)
                .expect("retained compound resolves")
                .unit(),
            &target
        );
    }
}

#[trace("TC-1092", "FR-398-AC-8")]
#[test]
fn small_admitted_root_family_completes_exact_handoff() {
    let compiled = compile("producer-unit-small".to_owned(), ROOTS, true);
    let admitted = crate::emit_checked_v2::read_back(compiled.package().bytes(), &[]);
    for root in compiled.context().units().filter(|unit| {
        unit.qualified_name()
            .last()
            .is_some_and(|name| name.as_str().starts_with('r'))
    }) {
        let id = qsl_semantics::check::wire_id::structural_id(root.node());
        assert!(
            admitted
                .resolved_graph()
                .nodes
                .iter()
                .any(|node| node.node_id == id),
            "the source function forces its real root node into the admitted bytes"
        );
    }
    let roots: Vec<_> = compiled
        .context()
        .units()
        .map(|unit| unit.node().clone())
        .collect();
    let exponent = Integer::one();
    let Err(IdentityRefusal::InputBytes { bound, required }) =
        quire_semantic_value::unit::compound_unit_id(
            roots.iter().map(|root| (root, &exponent)),
            quire_canonical::Limits::new(64),
        )
    else {
        panic!("the native owner counter must refuse this source-backed term encoding")
    };
    assert_eq!(bound, 64);
    assert!(required > bound);
    conversion_handoff(&compiled, false);
}

#[trace("TC-1092", "FR-398-AC-8")]
#[test]
fn long_admitted_package_name_reaches_actual_handoff_identity_refusal() {
    let package_name = "p".repeat(16_777_216 / ROOTS);
    conversion_handoff(&compile(package_name, ROOTS, false), true);
}
