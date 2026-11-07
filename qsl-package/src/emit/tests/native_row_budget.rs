// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: real shared-budget row materialization and allocation refusals.

use super::*;
use crate::{NativeRowError, NativeRowErrorCode};
use quire_contract_model::{
    NativeJoinBudget, NativeJoinCharge, NativeJoinChargeKind, NativeJoinLocus,
    NativeJoinLocusSource,
};

fn ample() -> NativeJoinBudget {
    NativeJoinBudget::new(4_000_000, 4_000_000)
}

const ROWS: &str = "\
dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;
enum Status { READY, DONE }
ordered enum Level { LOW, MID, HIGH }
record R { statuses: Sequence<Status>[0, 3]; flags: Set<Boolean>; }
function status using v(s: Status, l: Level, r: R): Boolean pure { s = Status::READY }
function quantity using v(a: Quantity<ms>, b: Quantity<s>): Boolean pure { convert<Quantity<s>>(a) = b }
function text using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a = b }
function large using v(a: Int[0, 680564733841876926926749214863536422912], b: Int[0, 680564733841876926926749214863536422912]): Boolean pure { a = b }
";

fn emission() -> Emission {
    super::converted_equality::emitted(ROWS)
}

fn mixed_emission() -> Emission {
    super::variant_types::emitted_enums(&format!(
        "{ROWS}\nfunction tier using v(a: M::Tier): Boolean pure {{ a = M::Tier::Gold }}\n\
         function accounts using v(a: M::Account, b: M::Account): Boolean pure {{ a = b }}"
    ))
}

fn native(emission: &Emission) -> CheckedPackageV2 {
    super::source_union_rows::admit_bytes(emission, &wire(emission))
}

fn boundaries<T: std::fmt::Debug + PartialEq>(
    mut materialize: impl FnMut(&mut NativeJoinBudget) -> Result<T, NativeRowError>,
) -> NativeJoinCharge {
    let mut measure = ample();
    let expected = materialize(&mut measure).expect("ample allowance materializes real rows");
    let need = measure.consumed();
    assert!(need.work > 0 && need.storage > 0);
    let mut exact = NativeJoinBudget::new(need.work, need.storage);
    assert_eq!(materialize(&mut exact).expect("exact allowance"), expected);
    assert_eq!(exact.consumed(), need);

    // Initialize the measurement machinery before observing the adapter.
    let _ = allocation_counter::measure(|| {});
    for (work, storage, kind) in [
        (need.work - 1, need.storage, NativeJoinChargeKind::Work),
        (need.work, need.storage - 1, NativeJoinChargeKind::Storage),
        (need.work, 0, NativeJoinChargeKind::Storage),
    ] {
        let mut limited = NativeJoinBudget::new(work, storage);
        let mut outcome = None;
        let allocations = allocation_counter::measure(|| {
            outcome = Some(materialize(&mut limited));
        });
        assert_eq!(
            allocations.count_total, 0,
            "preflight refuses before owned materialization"
        );
        let error = outcome
            .expect("adapter was called")
            .expect_err("insufficient allowance");
        assert_eq!(error.denied(), limited.first_unpaid());
        assert_eq!(error.denied().expect("owner denial").kind, kind);
        assert_eq!(error.locus(), limited.first_unpaid_locus());
        assert_eq!(
            error.configured_bound(),
            Some(match kind {
                NativeJoinChargeKind::Work => work,
                NativeJoinChargeKind::Storage => storage,
            })
        );
        assert!(error.locus().is_some());
        assert_eq!(limited.work_limit(), work);
        assert_eq!(limited.storage_limit(), storage);
        assert_eq!(limited.last_success_delta(), None);
    }

    let mut shared = NativeJoinBudget::new(need.work * 2 - 1, need.storage * 2);
    assert_eq!(
        materialize(&mut shared).expect("first materialization"),
        expected
    );
    let prefix = shared.consumed();
    let second = materialize(&mut shared)
        .expect_err("two individually affordable calls share one allowance");
    assert_eq!(
        second.denied().expect("second denied work").kind,
        NativeJoinChargeKind::Work
    );
    assert!(shared.consumed().work >= prefix.work);
    assert!(shared.consumed().storage >= prefix.storage);
    let first = shared.first_unpaid();
    let locus = shared.first_unpaid_locus();
    let consumed = shared.consumed();
    assert_eq!(
        materialize(&mut shared).expect_err("a refusal is not resumed away"),
        second
    );
    assert_eq!(shared.first_unpaid(), first);
    assert_eq!(shared.first_unpaid_locus(), locus);
    assert_eq!(shared.consumed(), consumed);
    assert_eq!(
        shared.accounted_charge(
            NativeJoinChargeKind::Storage,
            1,
            NativeJoinLocus {
                source: NativeJoinLocusSource::Package,
                form: "later_charge"
            }
        ),
        Err(first.expect("first unpaid stays available"))
    );
    assert_eq!(shared.first_unpaid_locus(), locus);
    assert_eq!(shared.consumed(), consumed);
    need
}

#[trace("TC-1331", "FR-531-AC-29")]
#[test]
fn equality_witness_materializes_complete_original_rows_at_exact_shared_bounds() {
    let emission = emission();
    let package = native(&emission);
    boundaries(|budget| emission.equality_witness(&package, budget));
    let witness = emission
        .equality_witness(&package, &mut ample())
        .expect("real witness");
    let applications = emission.equality_producer_identity_rows();
    assert_eq!(witness.applications().len(), applications.len());
    for (row, original) in witness.applications().iter().zip(applications) {
        assert_eq!(&row.application, original.checked_node());
        assert_eq!(&row.occurrence, original.occurrence());
        assert_eq!(&row.source_map_entry, original.source_map_entry());
        assert_eq!(&row.node_key, original.application());
        assert_eq!(&row.location, original.location());
    }
    assert_eq!(witness.source_units(), emission.selected_source_unit_rows());
    let expected: Vec<_> = emission
        .equality_fact_witness_rows()
        .iter()
        .filter(|row| {
            row.checked_application().package == row.structural_node().package
                && emission
                    .wire_nodes
                    .iter()
                    .any(|node| &node.node_id == row.structural_node())
        })
        .collect();
    assert_eq!(witness.facts().len(), expected.len());
    for (row, original) in witness.facts().iter().zip(expected) {
        assert_eq!(&row.application, original.application());
        assert_eq!(&row.checked_application, original.checked_application());
        assert_eq!(&row.structural_node, original.structural_node());
        assert_eq!(&row.structural_key, original.structural_key());
        assert_eq!(row.owner.as_ref(), original.owner());
        assert_eq!(row.value_type.as_ref(), original.value_type());
    }
    let bytes = witness.to_bytes(&package).expect("authentic rows encode");
    assert_eq!(
        crate::ReadEqualityRows::read(&bytes, &package).expect("native witness round-trip"),
        witness
    );
}

#[trace("TC-1331", "FR-531-AC-30")]
#[test]
fn enum_and_collection_conversions_preserve_owner_values_and_one_shared_allowance() {
    let emission = mixed_emission();
    let source_need = boundaries(|budget| emission.selected_source_enum_rows(budget));
    let model_need = boundaries(|budget| emission.selected_ir_model_enum_shape_rows(budget));
    let collection_need = emission
        .selected_source_collection_rows()
        .iter()
        .enumerate()
        .fold(NativeJoinCharge::default(), |mut total, (index, row)| {
            let need = boundaries(|budget| row.to_ir(budget, collection_locus(index)));
            total.work += need.work;
            total.storage += need.storage;
            total
        });
    let mut shared = NativeJoinBudget::new(
        source_need.work + model_need.work + collection_need.work,
        source_need.storage + model_need.storage + collection_need.storage,
    );
    let source = emission
        .selected_source_enum_rows(&mut shared)
        .expect("source rows");
    let model = emission
        .selected_ir_model_enum_shape_rows(&mut shared)
        .expect("model shape rows");
    let collections: Vec<_> = emission
        .selected_source_collection_rows()
        .iter()
        .enumerate()
        .map(|(index, row)| {
            row.to_ir(&mut shared, collection_locus(index))
                .expect("selected collection row")
        })
        .collect();
    assert_eq!(shared.consumed().work, shared.work_limit());
    assert_eq!(shared.consumed().storage, shared.storage_limit());
    for (name, ordered, expected) in [
        ("Status", false, ["DONE", "READY"].as_slice()),
        ("Level", true, ["LOW", "MID", "HIGH"].as_slice()),
    ] {
        let row = source
            .iter()
            .find(|row| {
                row.declaration
                    .qualified_declaration
                    .last()
                    .map(AsRef::as_ref)
                    == Some(name)
            })
            .expect("source declaration");
        assert_eq!(row.declaration.shape.is_ordered(), ordered);
        assert_eq!(
            row.members
                .iter()
                .map(|member| member.case.as_ref())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            row.members
                .iter()
                .map(|member| &member.variant)
                .collect::<Vec<_>>(),
            row.declaration.shape.variants().collect::<Vec<_>>()
        );
    }
    assert_eq!(model.len(), emission.selected_model_enum_shape_rows().len());
    for (row, original) in model.iter().zip(emission.selected_model_enum_shape_rows()) {
        assert_eq!(row.structural_node, original.structural_node);
        assert_eq!(row.node_key, original.node_key);
        assert_eq!(row.selected_document, original.selected_document);
        assert_eq!(row.original_identity, original.original_identity);
        assert_eq!(row.shape, original.shape);
        assert_eq!(row.cases.len(), original.cases.len());
        for (case, original) in row.cases.iter().zip(&original.cases) {
            assert_eq!(case.case, original.case);
            assert_eq!(case.structural_node, original.structural_node);
            assert_eq!(case.member_key, original.member_key);
            assert_eq!(
                case.member_original_identity,
                original.member_original_identity
            );
            assert_eq!(case.variant, original.variant);
        }
    }
    assert_eq!(
        collections.len(),
        emission.selected_source_collection_rows().len()
    );
    for (row, original) in collections
        .iter()
        .zip(emission.selected_source_collection_rows())
    {
        assert_eq!(row.structural_node, original.structural_node);
        assert_eq!(row.base_node, original.base_node);
        assert_eq!(row.element_node, original.element_node);
        assert_eq!(row.value_type, original.value_type);
        assert_eq!(original.key_node, None);
    }
}

#[trace("TC-1331", "FR-531-AC-30")]
#[test]
fn adapters_preserve_an_earlier_owner_denial_and_maps_never_lose_their_keys() {
    let emission = emission();
    let original = NativeJoinLocus {
        source: NativeJoinLocusSource::ReachedRow(0),
        form: "earlier_selection",
    };
    let mut budget = NativeJoinBudget::new(0, 0);
    let denied = budget
        .accounted_charge(NativeJoinChargeKind::Work, 9, original)
        .expect_err("earlier caller charge");
    let _ = allocation_counter::measure(|| {});
    let mut error = None;
    let allocations = allocation_counter::measure(|| {
        error = Some(
            emission
                .selected_source_enum_rows(&mut budget)
                .expect_err("prior denial"),
        );
    });
    assert_eq!(allocations.count_total, 0);
    let error = error.expect("adapter ran");
    assert_eq!(error.denied(), Some(denied));
    assert_eq!(error.locus(), Some(original));
    assert_eq!(budget.first_unpaid_locus(), Some(original));
    assert_eq!(budget.consumed(), NativeJoinCharge::default());

    let map_package = mixed_collection_package();
    let maps = emit_checked(&map_package).expect("real source map emission");
    assert!(maps
        .selected_source_collection_rows()
        .iter()
        .any(|row| row.key_node.is_some()));
    let (supported_index, supported) = maps
        .selected_source_collection_rows()
        .iter()
        .enumerate()
        .find(|(_, row)| row.key_node.is_none())
        .expect("same emission retains a representable collection");
    let mut conversion_budget = ample();
    let converted = supported
        .to_ir(&mut conversion_budget, collection_locus(supported_index))
        .expect("unrelated map must not poison selected collection conversion");
    assert_eq!(converted.value_type, supported.value_type);
    assert_eq!(converted.element_node, supported.element_node);
    let package = native(&maps);
    let joined = quire_contract_model::SelectedExactIdentities::empty()
        .with_source_collection_rows(&package, [converted], &mut conversion_budget)
        .expect("IR joins only the requested supported collection despite the retained map");
    let projected = package
        .project_exact_type(&supported.structural_node, &joined, &mut conversion_budget)
        .expect("IR projects the authentic converted supported collection");
    assert_eq!(projected.value_type(), &supported.value_type);
    assert!(conversion_budget.consumed().work > 0);

    let (map_index, map_row) = maps
        .selected_source_collection_rows()
        .iter()
        .enumerate()
        .find(|(_, row)| row.key_node.is_some())
        .expect("same emission retains map");
    let mut budget = ample();
    let mut result = None;
    let allocations = allocation_counter::measure(|| {
        result = Some(map_row.to_ir(&mut budget, collection_locus(map_index)));
    });
    assert_eq!(allocations.count_total, 0);
    let error = result
        .expect("adapter ran")
        .expect_err("target IR row cannot carry a map key");
    assert_eq!(error.code(), NativeRowErrorCode::UnsupportedMap);
    assert_eq!(error.denied(), None);
    assert_eq!(budget.first_unpaid(), None);
    assert!(maps
        .selected_source_collection_rows()
        .iter()
        .any(|row| row.key_node.is_some()));
}

#[trace("TC-1331", "FR-531-AC-29")]
#[test]
fn final_row_membership_matches_the_original_filter_after_carry_and_rekey() {
    let emission = emission();
    assert!(!emission.equality_fact_witness_rows.is_empty());
    let mut facts = emission.equality_fact_witness_rows.clone();
    let mut foreign = facts[0].clone();
    foreign.checked_application.package = "different-application-package".into();
    facts.push(foreign);
    let omitted = facts[0].structural_node.clone();
    let remaining: Vec<_> = emission
        .wire_nodes
        .iter()
        .filter(|node| node.node_id != omitted)
        .cloned()
        .collect();
    super::super::monitor_plan::refresh_equality_membership(&mut facts, &remaining);
    for row in &facts {
        assert_eq!(
            row.selected_for_emission,
            row.checked_application.package == row.structural_node.package
                && remaining
                    .iter()
                    .any(|node| node.node_id == row.structural_node)
        );
    }
    assert!(!facts.last().expect("foreign control").selected_for_emission);
    assert!(
        !facts[0].selected_for_emission,
        "omitted actual node is excluded"
    );
    let mut retained = emission.clone();
    retained.equality_fact_witness_rows = facts;
    let renamed = CheckedNodeId {
        package: omitted.package.clone(),
        path: omitted.path.clone(),
        ordinal: omitted.ordinal + 1,
    };
    let mut carried =
        super::super::monitor_plan::carry_rows(&retained, &[(omitted.clone(), renamed.clone())]);
    let mut final_nodes = emission.wire_nodes.clone();
    for node in &mut final_nodes {
        if node.node_id == omitted {
            node.node_id = renamed.clone();
        }
    }
    super::super::monitor_plan::refresh_equality_membership(
        &mut carried.equality_fact_witness_rows,
        &final_nodes,
    );
    for row in &carried.equality_fact_witness_rows {
        assert_eq!(
            row.selected_for_emission,
            row.checked_application.package == row.structural_node.package
                && final_nodes
                    .iter()
                    .any(|node| node.node_id == row.structural_node)
        );
    }
    assert!(
        carried.equality_fact_witness_rows[0].selected_for_emission,
        "final emitted rekey refreshes a stale omission hint"
    );
}

fn collection_locus(index: usize) -> NativeJoinLocus {
    NativeJoinLocus {
        source: NativeJoinLocusSource::ReachedRow(index),
        form: "selected_source_collection_row",
    }
}

#[trace("TC-1331", "FR-531-AC-30")]
#[test]
fn second_retained_enum_row_keeps_its_first_unpaid_locus() {
    let emission = emission();
    let mut complete = ample();
    emission
        .selected_source_enum_rows(&mut complete)
        .expect("complete real population");
    let second = NativeJoinLocus {
        source: NativeJoinLocusSource::ReachedRow(1),
        form: "source_enum_declaration_row",
    };
    let (mut budget, error) = (0..complete.consumed().work)
        .find_map(|limit| {
            let mut budget = NativeJoinBudget::new(limit, complete.consumed().storage);
            let error = emission
                .selected_source_enum_rows(&mut budget)
                .expect_err("short work allowance");
            (error.locus() == Some(second)).then_some((budget, error))
        })
        .expect("a real charge in the second retained declaration exhausts");
    let consumed = budget.consumed();
    let denied = error.denied().expect("owner denial");
    assert_eq!(denied.kind, NativeJoinChargeKind::Work);
    assert_eq!(
        budget.accounted_charge(NativeJoinChargeKind::Storage, 1, collection_locus(9)),
        Err(denied)
    );
    assert_eq!(budget.first_unpaid_locus(), Some(second));
    assert_eq!(budget.consumed(), consumed);
    assert_eq!(emission.selected_source_enum_rows(&mut budget), Err(error));
    assert_eq!(budget.first_unpaid_locus(), Some(second));
    assert_eq!(budget.consumed(), consumed);
}

fn mixed_collection_package() -> CheckedPackage {
    use qsl_forms::TypeForm;
    use quire_exact::{CardinalityBound, MapType, Presence};
    use quire_semantic_value::declaration::{
        CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
    };

    let handle = NodeKey::new("pkg", vec![quire_exact::PathSegment::Text("R".into())], 0)
        .expect("a well formed key");
    let map = |values, bound: Option<(u64, u64)>| {
        ValueType::map(MapType::new(
            values,
            bound.map(|(lower, upper)| CardinalityBound::new(lower, upper).expect("a bound")),
        ))
    };
    let collection = emission()
        .selected_source_collection_rows()
        .iter()
        .find(|row| matches!(&row.value_type, ValueType::Collection(value) if value.element() == &ValueType::Boolean))
        .expect("real sequence row")
        .value_type
        .clone();
    let record = CompositeDeclaration::new(
        handle,
        "R",
        CompositeShape::Record(vec![
            FieldDeclaration::new(
                "m",
                map(ValueType::Boolean, Some((1, 4))),
                Presence::Required,
            ),
            FieldDeclaration::new("n", collection, Presence::Required),
        ]),
    );
    let span = qsl_foundation::Span { start: 0, end: 0 };
    let mut declarations = qsl_semantics::check::depth_forms::declarations(
        qsl_semantics::check::depth_forms::Form::And,
        1,
    );
    declarations.functions = vec![qsl_semantics::check::depth_forms::function(
        "f",
        &[("r", TypeForm::name("R", span))],
        qsl_semantics::check::depth_forms::boolean(),
        None,
        qsl_forms::Expression::boolean(true),
    )];
    declarations.package_name = "pkg".to_owned();
    declarations.types = TypeEnvironment::new([record], []).expect("R is admitted");
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    CheckedPackage::link(graph)
}

#[trace("TC-1331", "FR-531-AC-33")]
#[test]
fn cold_integer_payloads_refuse_inside_the_real_equality_adapter_without_allocating() {
    let emission = super::converted_equality::emitted(
        "function same using v(a: Int[0, 340282366920938463463374607431768211457], b: Int[0, 340282366920938463463374607431768211457]): Boolean pure { a = b }",
    );
    let package = native(&emission);
    let row_index = emission
        .equality_fact_witness_rows()
        .iter()
        .position(|row| {
            matches!(row.value_type(), Some(ValueType::Int(_)))
                && matches!(row.fact(), EqualityFactWitness::OperandType { .. })
        })
        .expect("the genuine emission retains the integer operand type");
    let locus = NativeJoinLocus {
        source: NativeJoinLocusSource::ReachedRow(row_index),
        form: "equality_fact_row",
    };
    let mut measure = ample();
    let expected = emission
        .clone()
        .equality_witness(&package, &mut measure)
        .expect("independent complete materialization");
    let need = measure.consumed();
    // Select the last denied charge in this actual operand row, so the
    // observer reaches its numeric copy precharge rather than an earlier path
    // string. Each probe uses a separate copy; none warms the final input.
    let storage = (0..need.storage)
        .filter(|storage| {
            let candidate = emission.clone();
            let mut budget = NativeJoinBudget::new(need.work, *storage);
            candidate
                .equality_witness(&package, &mut budget)
                .expect_err("one resource remains short")
                .locus()
                == Some(locus)
        })
        .last()
        .expect("an owned charge in the retained integer row");
    let cold = emission.clone();
    let mut budget = NativeJoinBudget::new(need.work, storage);
    let _ = allocation_counter::measure(|| {});
    let mut result = None;
    let allocations = allocation_counter::measure(|| {
        result = Some(cold.equality_witness(&package, &mut budget));
    });
    let error = result
        .expect("real adapter ran")
        .expect_err("numeric materialization storage denied");
    assert_eq!(allocations.count_total, 0);
    assert_eq!(error.code(), NativeRowErrorCode::StorageLimit);
    assert_eq!(error.locus(), Some(locus));
    assert_eq!(error.configured_bound(), Some(storage));
    assert_eq!(error.denied(), budget.first_unpaid());
    let mut exact = NativeJoinBudget::new(need.work, need.storage);
    assert_eq!(
        emission
            .clone()
            .equality_witness(&package, &mut exact)
            .expect("cold payloads copy at their exact full allowance"),
        expected
    );
}
