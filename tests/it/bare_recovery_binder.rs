// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533 and FR-390 (TC-1333, TC-1071): recipe R's
//! `recovery.predicate`, whose body holds the `sum(receipt in ...)`
//! comprehension, evaluated through the bare route over the handoff files.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, ProtocolSlot, ProtocolSupply, SuppliedValue};
use qsl_package::{BarePackage, BareSupply, ProtocolSlotDescriptor};
use quire_exact::{FieldValue, Meter, NodeKey, Value, ValueType};
use quire_spec_language::checked_handoff::Corpus;

use super::checked_handoff::{admitted, integer_request, request, tree};
use quire_spec_language::checked_handoff::{evidence, write};

const UNLIMITED: quire_exact::ScalarLimits = quire_exact::ScalarLimits {
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

fn int(value: i64) -> Value {
    Value::Integer(value.into())
}

/// The recipe R handoff files and the key of its compensation `name`.
struct Recipe {
    _holder: tempfile::TempDir,
    package: Box<quire_contract_model::CheckedPackageV2>,
    identities: qsl_package::ReadIdentityRows,
    equality: qsl_package::ReadEqualityRows,
}

fn recipe() -> Recipe {
    recipe_with(None)
}

/// R's `Full` recover condition as the recipe writes it.
const FULL_RECOVER: &str = "sum<M::Total>(receipt in recoveredFull.receipts: receipt) = targetFull\n            and not activatedFull";

/// R with the `Full` recover condition replaced by `condition`.
fn recipe_with(condition: Option<&str>) -> Recipe {
    let holder = tempfile::tempdir().expect("directory");
    let mut request = integer_request(holder.path());
    if let Some(condition) = condition {
        let text = std::fs::read_to_string(&request.emit.source).expect("R's unit");
        assert!(
            text.contains(FULL_RECOVER),
            "the recipe text is as expected"
        );
        let unit = holder.path().join("unit.native");
        let text = text.replace(FULL_RECOVER, condition);
        std::fs::write(&unit, text).expect("variant");
        request.emit.source = unit;
    }
    read_recipe(holder, &request)
}

fn read_recipe(
    holder: tempfile::TempDir,
    request: &quire_spec_language::checked_handoff::CheckedHandoffRequest,
) -> Recipe {
    let out = holder.path().join("handoff");
    write(request, &out).expect("the handoff is written");
    let files = tree(&out);
    let package = admitted(
        &files["checked-package.json"],
        &evidence(request).expect("evidence"),
    );
    let identities = qsl_package::ReadIdentityRows::read(
        &files["identities.json"],
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("rows");
    let equality =
        qsl_package::ReadEqualityRows::read(&files["equality.json"], &package).expect("rows");
    Recipe {
        _holder: holder,
        package,
        identities,
        equality,
    }
}

impl Recipe {
    fn bare(&self) -> BarePackage<'_> {
        BarePackage::new(&self.package)
            .with_supply(
                BareSupply {
                    identities: &self.identities,
                    equality: Some(&self.equality),
                },
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("the fixture budget admits the supply")
    }

    fn compensation(&self, name: &str) -> NodeKey {
        let nodes = &self.package.graph().nodes;
        let node = nodes
            .iter()
            .find(|node| {
                &*node.semantic_form == "compensation"
                    && node.declaration.as_ref().is_some_and(|declaration| {
                        declaration.qualified_name.last().map(|part| &**part) == Some(name)
                    })
            })
            .expect("compensation node");
        qsl_semantics::check::wire_id::WireGraph::new(nodes.iter().map(|node| &node.node_id))
            .expect("wire ids")
            .key(&node.node_id)
            .expect("key")
    }

    /// `Recovery` carrying `receipts`, built from the package's own declaration.
    fn recovery(&self, bare: &BarePackage<'_>, receipts: &[i64]) -> Value {
        let world = bare.world().expect("world");
        let types = world.types();
        let declaration = types
            .composites()
            .find(|record| record.name().ends_with("Recovery"))
            .expect("Recovery is declared");
        let quire_semantic_value::declaration::CompositeShape::Record(fields) = declaration.shape()
        else {
            panic!("Recovery is a record");
        };
        let ValueType::Collection(collection) = fields
            .iter()
            .find(|field| field.name() == "receipts")
            .expect("receipts")
            .value_type()
        else {
            panic!("receipts is a collection");
        };
        let elements = receipts.iter().copied().map(int).collect();
        types
            .record(
                declaration.key(),
                vec![(
                    "receipts",
                    FieldValue::Present(quire_exact::from_admitted(
                        (**collection).clone(),
                        elements,
                    )),
                )],
            )
            .expect("Recovery")
    }

    /// The `recovery.predicate` of `name` over the three supplied binders.
    fn recover(
        &self,
        name: &str,
        receipts: &[i64],
        target: i64,
        activated: bool,
    ) -> Result<quire_exact::Value, String> {
        let bare = self.bare();
        let node = self.compensation(name);
        let slot = ProtocolSlot {
            node: node.clone(),
            member: "recovery.predicate".to_owned(),
            index: None,
        };
        let selected = bare
            .select_protocol_slot(&ProtocolSlotDescriptor::new(
                node,
                "recovery.predicate",
                None,
            ))
            .map_err(|failure| format!("{failure:?}"))?;
        let values = [
            (format!("recovered{name}"), self.recovery(&bare, receipts)),
            (format!("target{name}"), int(target)),
            (format!("activated{name}"), Value::Boolean(activated)),
        ];
        let given = ProtocolSupply {
            binders: values
                .into_iter()
                .map(|(binder, value)| {
                    let source = selected
                        .sources()
                        .iter()
                        .find(|source| source.name() == binder)
                        .unwrap_or_else(|| panic!("the slot reads {binder}"));
                    (source.binder().clone(), SuppliedValue::Value(value).into())
                })
                .collect(),
            observation: None,
        };
        let mut meter = Meter::new(UNLIMITED);
        match bare.evaluate_protocol_slot(&slot, &given, &mut meter) {
            Ok(qsl_eval::value::ProtocolSlotEvaluation::Evaluated(evaluation, _)) => {
                match evaluation.outcome {
                    quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(
                        value,
                    )) => Ok(value),
                    other => Err(format!("{other:?}")),
                }
            }
            other => Err(format!("{other:?}")),
        }
    }
}

fn holds(result: Result<Value, String>) -> bool {
    match result {
        Ok(Value::Boolean(held)) => held,
        other => panic!("a Boolean outcome: {other:?}"),
    }
}

/// FR-390 "Composed roots from the reader" and FR-517-AC-14: the `sum`
/// comprehension binder a `recover` slot introduces itself projects over the
/// bare route, so R's `Full` recovery predicate
/// (`sum(receipt in recovered.receipts: receipt) = target and not activated`)
/// evaluates from the handoff files alone to the value its text states, for
/// supplies that differ in the receipts, the target and the activation.
#[trace("FR-533-AC-19", "FR-517-AC-14", "TC-1333")]
#[test]
fn full_recovery_predicate_evaluates_through_the_bare_route() {
    let recipe = recipe();
    assert!(holds(recipe.recover("Full", &[2, 3], 5, false)));
    assert!(!holds(recipe.recover("Full", &[2, 3], 6, false)));
    assert!(!holds(recipe.recover("Full", &[2, 3], 5, true)));
    assert!(holds(recipe.recover("Full", &[1, 1, 1, 1], 4, false)));
    assert!(holds(recipe.recover("Full", &[], 0, false)));
}

/// The same for `Partial`, whose predicate holds two comprehensions that share
/// one binder node (`0 < sum(...) and sum(...) < target`), and which reads no
/// `activated` binder.
#[trace("FR-533-AC-19", "FR-517-AC-14", "TC-1333")]
#[test]
fn partial_recovery_predicate_evaluates_through_the_bare_route() {
    let recipe = recipe();
    assert!(holds(recipe.recover("Partial", &[1, 2], 5, false)));
    assert!(!holds(recipe.recover("Partial", &[], 5, false)));
    assert!(!holds(recipe.recover("Partial", &[5], 5, false)));
    assert!(holds(recipe.recover("Partial", &[4], 5, false)));
}

/// A malformed supply is refused, not evaluated: a recovery binder that is not
/// a `Recovery` record, and a supply that omits a binder the predicate reads.
#[trace("FR-533-AC-19", "TC-1333")]
#[test]
fn a_malformed_recovery_supply_is_refused() {
    let recipe = recipe();
    let bare = recipe.bare();
    let node = recipe.compensation("Full");
    let slot = ProtocolSlot {
        node: node.clone(),
        member: "recovery.predicate".to_owned(),
        index: None,
    };
    let selected = bare
        .select_protocol_slot(&ProtocolSlotDescriptor::new(
            node,
            "recovery.predicate",
            None,
        ))
        .expect("selects");
    let supply = |wrong_type: bool, omit: Option<&str>| {
        let values = [
            (
                "recoveredFull",
                if wrong_type {
                    int(5)
                } else {
                    recipe.recovery(&bare, &[2, 3])
                },
            ),
            ("targetFull", int(5)),
            ("activatedFull", Value::Boolean(false)),
        ];
        ProtocolSupply {
            binders: values
                .into_iter()
                .filter(|(name, _)| Some(*name) != omit)
                .map(|(name, value)| {
                    let source = selected
                        .sources()
                        .iter()
                        .find(|source| source.name() == name)
                        .expect("the slot reads the binder");
                    (source.binder().clone(), SuppliedValue::Value(value).into())
                })
                .collect(),
            observation: None,
        }
    };
    let mut meter = Meter::new(UNLIMITED);
    assert!(
        bare.evaluate_protocol_slot(&slot, &supply(false, None), &mut meter)
            .is_ok(),
        "the well-formed supply evaluates"
    );
    for (name, given) in [
        ("a wrongly typed binder", supply(true, None)),
        ("a missing binder", supply(false, Some("targetFull"))),
    ] {
        let mut meter = Meter::new(UNLIMITED);
        let result = bare.evaluate_protocol_slot(&slot, &given, &mut meter);
        assert!(
            matches!(result, Err(quire_eval::CallFailure::Admission(_))),
            "{name} is refused at admission: {result:?}"
        );
    }
}

/// The names of the sources `Full`'s recovery slot lists, in slot order.
fn source_names(recipe: &Recipe) -> Vec<String> {
    let bare = recipe.bare();
    let node = recipe.compensation("Full");
    bare.select_protocol_slot(&ProtocolSlotDescriptor::new(
        node,
        "recovery.predicate",
        None,
    ))
    .expect("selects")
    .sources()
    .iter()
    .map(|source| source.name().to_owned())
    .collect()
}

const FULL_SOURCES: [&str; 3] = ["targetFull", "activatedFull", "recoveredFull"];

/// FR-390 rule 3: the binder a collection query introduces is not a source;
/// the slot lists only the binders the caller supplies.
#[trace("FR-390-AC-68", "TC-1071", "TC-1333")]
#[test]
fn a_query_binder_is_no_source_of_the_slot() {
    assert_eq!(source_names(&recipe()), FULL_SOURCES);
}

/// A `let` binder over the same predicate: not a source, and the predicate
/// evaluates to the same values.
#[trace("FR-390-AC-68", "TC-1071", "TC-1333")]
#[test]
fn a_let_binder_is_no_source_and_the_predicate_evaluates() {
    let recipe = recipe_with(Some(
        "let total = sum<M::Total>(receipt in recoveredFull.receipts: receipt) in\n            total = targetFull and not activatedFull",
    ));
    assert_eq!(source_names(&recipe), FULL_SOURCES);
    assert!(holds(recipe.recover("Full", &[2, 3], 5, false)));
    assert!(!holds(recipe.recover("Full", &[2, 3], 4, false)));
    assert!(!holds(recipe.recover("Full", &[2, 3], 5, true)));
}

/// Nested queries with distinct binder names: both are introduced, neither is
/// a source, and the inner body reads the outer binder.
#[trace("FR-390-AC-68", "TC-1071", "TC-1333")]
#[test]
fn nested_query_binders_are_no_sources_and_the_predicate_evaluates() {
    let recipe = recipe_with(Some(
        "count<M::Total>(a in recoveredFull.receipts: sum<M::Total>(b in recoveredFull.receipts: b) > a) = targetFull\n            and not activatedFull",
    ));
    assert_eq!(source_names(&recipe), FULL_SOURCES);
    // Each of the two receipts is below their sum 5, so the count is 2.
    assert!(holds(recipe.recover("Full", &[2, 3], 2, false)));
    assert!(!holds(recipe.recover("Full", &[2, 3], 3, false)));
    // A receipt equal to the sum is not below it.
    assert!(holds(recipe.recover("Full", &[5], 0, false)));
}

/// FR-533 "Types the bytes cannot project": over R with the unit `U` on
/// `Amount` and `Total` (FR-517-AC-13), Contract IR cannot project the
/// `Recovery` record, because its `receipts` member names the source-owned
/// unit. The bare world therefore holds no `Recovery`, and the slot that reads
/// it refuses `MissingTypeRepresentation` rather than substituting a record of
/// another shape; the other tests of this file read the unit-free variant.
#[trace("FR-533-AC-19", "TC-1333")]
#[test]
fn recovery_over_the_unit_u_is_a_missing_type_representation() {
    let holder = tempfile::tempdir().expect("directory");
    let recipe = read_recipe(holder, &request("event_position", Corpus::None));
    let bare = recipe.bare();
    let world = bare.world().expect("world");
    let records: Vec<String> = world
        .types()
        .composites()
        .map(|record| record.name().to_owned())
        .collect();
    assert!(
        records.iter().all(|name| !name.ends_with("Recovery")),
        "{records:?}"
    );
    let refused = bare
        .select_protocol_slot(&ProtocolSlotDescriptor::new(
            recipe.compensation("Full"),
            "recovery.predicate",
            None,
        ))
        .map(|selected| {
            selected
                .sources()
                .iter()
                .map(|source| format!("{} {:?}", source.name(), source.value_type()))
                .collect::<Vec<_>>()
        })
        .unwrap_err();
    assert!(
        format!("{refused:?}").contains("MissingTypeRepresentation"),
        "the Recovery binder is not typed as a quantity: {refused:?}"
    );
}

/// A binder that has the name of a binder already readable is refused when the
/// unit is checked, so a source and an introduced binder of one slot never
/// share a name and no package holds that pair.
#[trace("FR-390-AC-68", "TC-1071")]
#[test]
fn a_query_binder_named_as_a_source_is_refused_when_the_unit_is_checked() {
    let holder = tempfile::tempdir().expect("directory");
    let mut request = request("event_position", Corpus::None);
    let text = std::fs::read_to_string(&request.emit.source).expect("R's unit");
    let unit = holder.path().join("unit.native");
    std::fs::write(
        &unit,
        text.replace(
            FULL_RECOVER,
            "sum<M::Total>(targetFull in recoveredFull.receipts: targetFull) = targetFull\n            and not activatedFull",
        ),
    )
    .expect("variant");
    request.emit.source = unit;
    let refused = write(&request, &holder.path().join("handoff"));
    let text = format!("{refused:?}");
    assert!(
        text.contains("AmbiguousName") && text.contains("targetFull"),
        "{text}"
    );
}

/// A supply for an introduced binder is a key that is no source of the slot:
/// refused `unknown-member`, naming it.
#[trace("FR-390-AC-68", "TC-1071")]
#[test]
fn a_supply_for_an_introduced_binder_is_refused_as_unknown() {
    let recipe = recipe();
    let bare = recipe.bare();
    let node = recipe.compensation("Full");
    let slot = ProtocolSlot {
        node: node.clone(),
        member: "recovery.predicate".to_owned(),
        index: None,
    };
    let selected = bare
        .select_protocol_slot(&ProtocolSlotDescriptor::new(
            node,
            "recovery.predicate",
            None,
        ))
        .expect("selects");
    let nodes = &recipe.package.graph().nodes;
    let graph =
        qsl_semantics::check::wire_id::WireGraph::new(nodes.iter().map(|node| &node.node_id))
            .expect("wire ids");
    let introduced = nodes
        .iter()
        .filter(|node| &*node.semantic_form == "parameter")
        .find(|node| {
            let key = graph.key(&node.node_id).expect("key");
            let text = format!("{:?}", node.body);
            text.contains("\"receipt\"") && !selected.sources().iter().any(|s| *s.binder() == key)
        })
        .expect("the introduced binder");
    let mut binders: Vec<_> = selected
        .sources()
        .iter()
        .map(|source| {
            let value = match source.name() {
                "recoveredFull" => recipe.recovery(&bare, &[2, 3]),
                "targetFull" => int(5),
                _ => Value::Boolean(false),
            };
            (source.binder().clone(), SuppliedValue::Value(value).into())
        })
        .collect();
    let mut meter = Meter::new(UNLIMITED);
    let given = ProtocolSupply {
        binders: binders.clone().into_iter().collect(),
        observation: None,
    };
    assert!(bare
        .evaluate_protocol_slot(&slot, &given, &mut meter)
        .is_ok());
    binders.push((
        graph.key(&introduced.node_id).expect("key"),
        SuppliedValue::Value(int(1)).into(),
    ));
    let given = ProtocolSupply {
        binders: binders.into_iter().collect(),
        observation: None,
    };
    let mut meter = Meter::new(UNLIMITED);
    let Err(quire_eval::CallFailure::Admission(refusal)) =
        bare.evaluate_protocol_slot(&slot, &given, &mut meter)
    else {
        panic!("the extra supply is refused");
    };
    let quire_eval::CallAdmissionFailure::Refused(refused) = *refusal else {
        panic!("a refusal");
    };
    assert_eq!(refused.record.catalog.code(), "invalid_runtime_input");
    assert_eq!(refused.record.catalog.cause(), "unknown-member");
    for counter in quire_exact::LimitKind::ALL {
        assert_eq!(meter.consumed(counter), 0, "{counter:?}");
    }
}

/// One recover condition of `Full` and the supplies it is evaluated over:
/// (receipts, target, activated, expected).
type Supplies = [(&'static [i64], i64, bool, bool)];

fn assert_form(condition: &str, supplies: &Supplies) {
    let recipe = recipe_with(Some(condition));
    assert_eq!(source_names(&recipe), FULL_SOURCES, "{condition}");
    for (receipts, target, activated, expected) in supplies {
        assert_eq!(
            holds(recipe.recover("Full", receipts, *target, *activated)),
            *expected,
            "{condition} over {receipts:?}, {target}, {activated}"
        );
    }
}

/// Each operation FR-390 rule 3 names, over a well-typed condition: the
/// introduced binder is no source and the predicate evaluates to its text.
#[trace("FR-390-AC-68", "TC-1071", "TC-1333")]
#[test]
fn forall_binder_is_no_source() {
    assert_form(
        "forall(x in recoveredFull.receipts: x <= targetFull) and not activatedFull",
        &[
            (&[2, 3], 3, false, true),
            (&[2, 3], 2, false, false),
            (&[2, 3], 3, true, false),
        ],
    );
}

#[trace("FR-390-AC-68", "TC-1071", "TC-1333")]
#[test]
fn exists_binder_is_no_source() {
    assert_form(
        "exists(x in recoveredFull.receipts: x <= targetFull and x >= targetFull) and not activatedFull",
        &[
            (&[2, 3], 3, false, true),
            (&[2, 3], 4, false, false),
            (&[2, 3], 3, true, false),
        ],
    );
}

#[trace("FR-390-AC-68", "TC-1071", "TC-1333")]
#[test]
fn filter_binder_is_no_source() {
    assert_form(
        "count<M::Total>(y in filter(x in recoveredFull.receipts: x > 2): true) = targetFull and not activatedFull",
        &[
            (&[2, 3, 4], 2, false, true),
            (&[2, 3, 4], 1, false, false),
            (&[1, 2], 0, false, true),
        ],
    );
}

#[trace("FR-390-AC-68", "TC-1071", "TC-1333")]
#[test]
fn map_binder_is_no_source() {
    assert_form(
        "sum<M::Total>(y in map(x in recoveredFull.receipts: x): y) = targetFull and not activatedFull",
        &[
            (&[2, 3], 5, false, true),
            (&[2, 3], 4, false, false),
            (&[2, 3], 5, true, false),
        ],
    );
}

/// A genuine case arm binds its payload inside the recover condition. The
/// handoff admits, the arm binder is no caller source, and the source and
/// bytes routes return the authored Boolean values and equal charge logs.
#[trace("FR-390-AC-68", "TC-1071", "TC-1333")]
#[test]
fn a_case_arm_binder_in_a_recover_condition_is_admitted_and_evaluates() {
    use qsl_foundation::SourceIdentity;
    use qsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits, SuppliedLibrary};

    let holder = tempfile::tempdir().expect("directory");
    let mut request = integer_request(holder.path());
    let text = std::fs::read_to_string(&request.emit.source).expect("R's unit");
    let unit = holder.path().join("unit.native");
    let condition = "let total = (case Shape::Circle(sum<M::Total>(receipt in recoveredFull.receipts: receipt)) { Empty: targetFull; Circle(r): r; }) in total <= targetFull and total >= targetFull and not activatedFull";
    std::fs::write(
        &unit,
        text.replace(FULL_RECOVER, condition).replace(
            "model M = \"agent-ix/checked-handoff\";",
            "model M = \"agent-ix/checked-handoff\";\nunion Shape { Empty, Circle(M::Total) }",
        ),
    )
    .expect("variant");
    request.emit.source = unit;

    let source = SourceIdentity::new("local", format!("source:{}", request.emit.source.display()));
    let bytes = std::fs::read(&request.emit.source).expect("the authored case-arm unit");
    let models = request
        .emit
        .models
        .iter()
        .map(|path| std::fs::read(path).expect("the recipe's model"))
        .collect::<Vec<_>>();
    let packages = qsl_semantics::model::intake::package_input(models.iter().map(Vec::as_slice))
        .expect("the selected model");
    let dependencies = DependencyInput::new(
        request
            .emit
            .libraries
            .iter()
            .map(|(identity, path)| SuppliedLibrary {
                identity: identity.clone(),
                source: SourceIdentity::new("local", format!("library:{}", path.display())),
                path: path.display().to_string(),
                bytes: std::fs::read(path).expect("the recipe's library"),
            })
            .collect::<Vec<_>>(),
    )
    .expect("the selected library");
    let limits = SpineLimits::default();
    let cancel = quire_exact::Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: &request.emit.source.display().to_string(),
            bytes: &bytes,
        },
        limits.source,
        &cancel,
    )
    .expect("the case-arm unit parses")
    .into_value();
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .expect("the model is selected")
        .into_value();
    let checked = spine::check(
        &parsed,
        request.emit.package_name.clone(),
        &models,
        &dependencies,
        limits,
        &cancel,
    )
    .expect("the genuine case-arm unit checks")
    .into_value();
    let source = checked.package();
    let source_protocol = source.graph().protocol("Flow").expect("Flow");
    let full = source_protocol
        .obligations
        .compensations
        .iter()
        .position(|template| template.name == "Full")
        .expect("Full");
    let source_slot = ProtocolSlot {
        node: source
            .graph()
            .protocol_node_keys("Flow")
            .expect("Flow's keys")
            .compensations[&full]
            .clone(),
        member: "recovery.predicate".to_owned(),
        index: None,
    };
    let view = source.evaluation_view();
    let source_selected = view
        .select_protocol_slot(&ProtocolSlotDescriptor::new(
            source_slot.node.clone(),
            "recovery.predicate",
            None,
        ))
        .expect("the source recovery predicate selects");
    let recipe = read_recipe(holder, &request);
    assert_eq!(
        source_names(&recipe),
        FULL_SOURCES,
        "the arm binder is no source"
    );
    let bare = recipe.bare();
    let bare_slot = ProtocolSlot {
        node: recipe.compensation("Full"),
        member: "recovery.predicate".to_owned(),
        index: None,
    };
    let selected = bare
        .select_protocol_slot(&ProtocolSlotDescriptor::new(
            bare_slot.node.clone(),
            "recovery.predicate",
            None,
        ))
        .expect("the admitted recovery predicate selects");
    let bare_names: Vec<&str> = selected
        .sources()
        .iter()
        .map(|source| source.name())
        .collect();
    let source_route_names: Vec<&str> = source_selected
        .sources()
        .iter()
        .map(|source| source.name())
        .collect();
    assert_eq!(
        bare_names, source_route_names,
        "both routes list the same sources"
    );
    for (receipts, target, activated, expected) in [
        (&[2, 3][..], 5, false, true),
        (&[2, 3][..], 4, false, false),
        (&[2, 3][..], 6, false, false),
        (&[2, 3][..], 5, true, false),
        (&[][..], 0, false, true),
    ] {
        let values = [
            ("recoveredFull", recipe.recovery(&bare, receipts)),
            ("targetFull", int(target)),
            ("activatedFull", Value::Boolean(activated)),
        ];
        let supply = ProtocolSupply {
            binders: values
                .iter()
                .map(|(name, value)| {
                    let binder = source_selected
                        .sources()
                        .iter()
                        .find(|source| source.name() == *name)
                        .expect("a caller source")
                        .binder()
                        .clone();
                    (binder, SuppliedValue::Value(value.clone()).into())
                })
                .collect(),
            observation: None,
        };
        let mut source_meter = Meter::new(UNLIMITED);
        let source_result = source
            .evaluate_protocol_slot(&source_slot, &supply, &mut source_meter)
            .expect("the source evaluates");
        let qsl_eval::value::ProtocolSlotEvaluation::Evaluated(source_evaluation, _) =
            source_result
        else {
            panic!("the source returns an evaluation")
        };
        assert_eq!(
            completed_boolean(&source_evaluation.outcome),
            Some(expected),
            "receipts={receipts:?}, target={target}, activated={activated}"
        );
    }
}

/// A binder the body never reads is no source and the predicate evaluates
/// (it is not a refusal), also nested inside another query.
#[trace("FR-390-AC-68", "TC-1071", "TC-1333")]
#[test]
fn a_binder_the_body_never_reads_is_no_source() {
    assert_form(
        "count<M::Total>(unused in recoveredFull.receipts: true) = targetFull and not activatedFull",
        &[(&[2, 3], 2, false, true), (&[2, 3], 3, false, false)],
    );
    assert_form(
        "count<M::Total>(a in recoveredFull.receipts: count<M::Total>(unused in recoveredFull.receipts: true) = 2 and a > 0) = targetFull and not activatedFull",
        &[(&[2, 3], 2, false, true), (&[2, 3], 3, false, false)],
    );
}

fn completed_boolean(outcome: &quire_eval::FamilyOutcome<Value>) -> Option<bool> {
    match outcome {
        quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(
            value,
        ))) => Some(*value),
        _ => None,
    }
}
