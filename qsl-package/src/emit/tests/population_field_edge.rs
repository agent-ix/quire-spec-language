// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1054 rows 15 to 26 and 28 (FR-380 "Field-edge reachability"): a
//! field-edge `reaches` whose walked population has an authored bound links
//! its `quire.op.model.reaches_field` application to the one
//! `Population<T>[N]` node of the edge's type `T`, and the walked population
//! is the one covering `T`, not the operands' static type. Every expected
//! value is the TC row's, over TC-1046's fixture R with the added fields.

use qsl_semantics::check::CheckedGraph;
use qsl_semantics::family::ClaimExtent;

use super::population_bound::{self, assert_population_node, population_nodes, type_occurrences};
use super::reachability::{applications, lower_over, model_node, Lowered};
use super::*;

/// The structural key of the unbounded `reaches(self, self, trail)` node over
/// `Order`, as `origin/main` computes it (TC-1047 row 3).
const TRAIL_REACHES_KEY: &str = "NodeKey(pkg:[\"expression\",\"reachability\",node(pkg:[\"value\",\"parameter\",\"name\",node(pkg:[\"scalar_type\",\"text\",]:0),\"self\",\"level\",node(pkg:[\"scalar_type\",\"integer\",]:0),\"0\",node(pkg:[\"composite_type\",\"reference\",node(acme/orders:[\"model\",\"object_type\",\"ix://acme/orders/Order\",]:0),]:0),]:0),\"quire.op.model.reaches_field\",\"reaches\",node(pkg:[\"scalar_type\",\"boolean\",]:0),]:0)";

const BOUND: &str = "bound OrdersBound on M::orders [4]";
const FAV_LOOP: &str = "invariant FavLoop using v on M::Person at current \
                        { not reaches(self.favourite, self.favourite, trail) }";
const RUSH_TRAIL: &str =
    "invariant RushTrail using v on M::Rush at current { not reaches(self, self, trail) }";
const RUSH_FAV: &str = "invariant RushFav using v on M::Person at current \
                        { not reaches(self.favRush, self.favRush, trail) }";

fn loop_over(field: &str) -> String {
    format!("invariant Loop using v on M::Order at current {{ not reaches(self, self, {field}) }}")
}

fn field_record(template: &Value, owner: &str, name: &str, type_name: &str) -> Value {
    let mut field = template.clone();
    field["identity"] = json!(format!("ix://acme/orders/{owner}/{name}"));
    field["name"] = json!(name);
    field["typeRef"] = json!(format!("ix://acme/orders/{type_name}"));
    field["presence"] = json!("required");
    field["multiplicity"] = json!({"lower": 1, "upper": 1, "ordered": false, "unique": true});
    field
}

fn sequence(mut field: Value, lower: u64, upper: u64) -> Value {
    field["multiplicity"] =
        json!({"lower": lower, "upper": upper, "ordered": true, "unique": false});
    field
}

fn population(name: &str, member: &str) -> Value {
    let mut population = serde_json::from_slice::<Value>(&population_bound::document()).unwrap()
        ["populations"][0]
        .clone();
    let identity = format!("ix://acme/orders/{name}");
    population["identity"] = json!(identity);
    population["displayName"] = json!(identity);
    population["members"] = json!([format!("ix://acme/orders/{member}")]);
    population["origin"]["generated"]["generatorIdentity"] = json!(identity);
    population["origin"]["generated"]["inputIdentities"] = json!([identity]);
    population
}

/// How the populations of the document differ from fixture R's.
enum Populations {
    /// R's `orders` and `people`.
    R,
    /// R plus `staff` of member type `Order`.
    WithStaff,
    /// R plus `rushes` of member type `Rush`.
    WithRushes,
    /// `orders` replaced by `rushes` of member type `Rush`.
    RushesInstead,
}

/// Fixture R with `Order` holding `trail`, `chief`, `chain` and `pair`, and
/// `Person` holding `favourite` and `favRush`.
fn document(populations: &Populations) -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&population_bound::document()).unwrap();
    let template = document["types"][0]["fields"][0].clone();
    for entry in document["types"].as_array_mut().unwrap() {
        match entry["displayName"].as_str() {
            Some("Order") => {
                let chief = field_record(&template, "Order", "chief", "Order");
                let chain = sequence(field_record(&template, "Order", "chain", "Order"), 0, 3);
                let pair = sequence(field_record(&template, "Order", "pair", "Order"), 2, 3);
                entry["fields"]
                    .as_array_mut()
                    .unwrap()
                    .extend([chief, chain, pair]);
            }
            Some("Person") => {
                entry["fields"] = json!([
                    field_record(&template, "Person", "favourite", "Order"),
                    field_record(&template, "Person", "favRush", "Rush"),
                ]);
            }
            _ => {}
        }
    }
    let list = document["populations"].as_array_mut().unwrap();
    match populations {
        Populations::R => {}
        Populations::WithStaff => list.push(population("staff", "Order")),
        Populations::WithRushes => list.push(population("rushes", "Rush")),
        Populations::RushesInstead => list[0] = population("rushes", "Rush"),
    }
    crate::emit::tests::wire_bytes(&document).unwrap()
}

fn lower(populations: &Populations, clauses: &str) -> Lowered {
    lower_over(document(populations), clauses)
}

fn check(
    populations: &Populations,
    clauses: &str,
) -> Result<CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>> {
    let packages =
        qsl_semantics::model::intake::package_input([document(populations).as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model M = \"acme/orders\";\n{clauses}\n"
    );
    assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
}

fn checked(populations: &Populations, clauses: &str) -> CheckedGraph {
    check(populations, clauses).unwrap_or_else(|refusals| panic!("{clauses}: {refusals:?}"))
}

/// The unbounded-domain count of each `claim` record, `0` for a bounded one.
fn unbounded_domains(graph: &CheckedGraph) -> Vec<usize> {
    graph
        .requirements()
        .iter()
        .filter(|(key, _)| key.origin().role().as_str() == "claim")
        .map(|(_, record)| match record.requirements().extent() {
            ClaimExtent::Unbounded(domains) => domains.len(),
            ClaimExtent::Bounded => 0,
        })
        .collect()
}

/// The label and maximum of each bounded population the clause `name`'s
/// requirements record names (its `Population(Some(N))` domains).
fn record_bounds(graph: &CheckedGraph, name: &str) -> Vec<(String, u64)> {
    graph
        .state_clause(name)
        .expect("the clause is checked")
        .population_bounds()
        .iter()
        .map(|bound| (bound.label.clone(), bound.maximum))
        .collect()
}

/// The one `reaches_field` application of `lowered`.
fn field_application(lowered: &Lowered) -> &Value {
    let fields = applications(&lowered.wire, "quire.op.model.reaches_field");
    let [field] = fields.as_slice() else {
        panic!("one reaches_field application");
    };
    field
}

fn field_member(lowered: &Lowered) -> &Value {
    &field_application(lowered)["body"]["operation"]["member"]
}

/// The one population node of `lowered`, shaped as `Population<Order>[4]`.
fn order_population(lowered: &Lowered) -> &Value {
    let populations = population_nodes(&lowered.wire);
    let [population] = populations.as_slice() else {
        panic!("one population node, got {}", populations.len());
    };
    assert_population_node(&lowered.wire, population, "4");
    population
}

/// Rows 15 to 18 (FR-380-AC-8 to AC-11): the member holds `population`, the
/// `Population<Order>[4]` node, which the application's `dependencies` hold.
fn assert_linked(field: &str, trace_ac: &str) {
    let lowered = lower(&Populations::R, &format!("{BOUND}\n{}", loop_over(field)));
    let population = order_population(&lowered);
    assert_eq!(
        field_member(&lowered),
        &json!({
            "kind": "field",
            "declaration": model_node(&lowered.wire, "Order")["node_id"],
            "name": field,
            "population": population["node_id"],
        }),
        "{trace_ac}"
    );
    assert!(
        field_application(&lowered)["dependencies"]
            .as_array()
            .unwrap()
            .contains(&population["node_id"]),
        "{trace_ac}: the dependencies hold the population node"
    );
    assert_eq!(type_occurrences(&lowered, population), [(0, "[4]")]);
}

#[trace("TC-1054", "FR-380-AC-8")]
#[test]
fn a_reference_field_edge_links_the_bounded_population() {
    assert_linked("chief", "FR-380-AC-8");
}

#[trace("TC-1054", "FR-380-AC-9")]
#[test]
fn an_optional_reference_field_edge_links_the_bounded_population() {
    assert_linked("trail", "FR-380-AC-9");
}

#[trace("TC-1054", "FR-380-AC-10")]
#[test]
fn a_sequence_field_edge_links_the_bounded_population() {
    assert_linked("chain", "FR-380-AC-10");
}

/// The bound `4` is unequal to the sequence's `[2, 3]`, so a node derived
/// from the sequence's bounds is told apart.
#[trace("TC-1054", "FR-380-AC-11")]
#[test]
fn a_sequence_bounds_supply_no_population() {
    assert_linked("pair", "FR-380-AC-11");
    let lowered = lower(&Populations::R, &format!("{BOUND}\n{}", loop_over("pair")));
    let maxima: Vec<&Value> = population_nodes(&lowered.wire)
        .iter()
        .map(|node| &node["body"]["members"][0]["value"]["value"])
        .collect();
    assert_eq!(maxima, ["4"], "no node of maximum 2 or 3");
}

/// Rows 20 (FR-380-AC-13): without the bound the member holds exactly
/// `kind`, `declaration` and `name`.
#[trace("TC-1054", "FR-380-AC-13")]
#[test]
fn an_unbounded_field_edge_keeps_its_member_and_node_identity() {
    for field in ["chief", "trail", "chain", "pair"] {
        let lowered = lower(&Populations::R, &loop_over(field));
        assert_eq!(
            field_member(&lowered),
            &json!({
                "kind": "field",
                "declaration": model_node(&lowered.wire, "Order")["node_id"],
                "name": field,
            })
        );
        assert_eq!(population_nodes(&lowered.wire), Vec::<&Value>::new());
    }
    // The application's node id is the one TC-1047 row 3 records: the
    // structural key `origin/main` gives the same clause, pinned here, so a
    // change that re-keys field members fails.
    let lowered = lower(&Populations::R, &loop_over("trail"));
    let wire = &lowered.wire;
    let id = &applications(wire, "quire.op.model.reaches_field")[0]["node_id"];
    let key = qsl_semantics::check::wire_id::WireGraph::of_package_value(wire)
        .unwrap()
        .key_of_value(id)
        .unwrap();
    assert_eq!(format!("{key:?}"), TRAIL_REACHES_KEY);
}

/// Row 21 (FR-380-AC-14): two populations covering `T` refuse at the
/// `reaches`, outside the `StateModel` family.
#[trace("TC-1054", "FR-380-AC-14")]
#[test]
fn two_populations_covering_the_edge_type_refuse_ambiguous_name() {
    let clauses = format!("{BOUND}\n{FAV_LOOP}");
    let Err(refusals) = check(&Populations::WithStaff, &clauses) else {
        panic!("{clauses}: refused");
    };
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    assert_eq!(
        refusal.cause.code(),
        qsl_foundation::diagnostic::Code::AmbiguousDeclaration
    );
    assert_eq!(refusal.cause.cause(), Some("ambiguous-name"));
    // At the `reaches`: the one operand of the clause body's root `not`.
    assert_eq!(refusal.location.path(), [0]);
    let qsl_semantics::check::CheckCause::AmbiguousName { name, .. } = &refusal.cause else {
        panic!("an ambiguous name outside the StateModel family: {refusal:?}");
    };
    assert_eq!(
        name, "M::Order (ix://acme/orders/orders, ix://acme/orders/staff)",
        "orders and staff in ascending DeclarationKey order"
    );
}

/// Row 22 (FR-380-AC-20): a field edge and a relationship end walking one
/// bounded population name one node holding one `Type` occurrence.
#[trace("TC-1054", "FR-380-AC-20")]
#[test]
fn a_field_edge_and_an_end_share_one_population_node() {
    let lowered = lower(
        &Populations::R,
        &format!(
            "{BOUND}\n{}\n\
             invariant NoLoop using v on M::Order at current {{ not reaches(self, self, next) }}",
            loop_over("trail")
        ),
    );
    let population = order_population(&lowered);
    let end = applications(&lowered.wire, "quire.op.model.reaches");
    let [end] = end.as_slice() else {
        panic!("one end reachability");
    };
    assert_eq!(
        end["body"]["operation"]["member"]["population"],
        field_member(&lowered)["population"]
    );
    assert_eq!(field_member(&lowered)["population"], population["node_id"]);
    assert_eq!(type_occurrences(&lowered, population), [(0, "[4]")]);
}

/// Row 23 (FR-380-AC-15): the link is the edge type's node, not the
/// operand's subtype's.
#[trace("TC-1054", "FR-380-AC-15")]
#[test]
fn a_subtype_operand_links_the_edge_types_node() {
    let lowered = lower(&Populations::R, &format!("{BOUND}\n{RUSH_TRAIL}"));
    let population = order_population(&lowered);
    let member = field_member(&lowered);
    assert_eq!(
        member["declaration"],
        model_node(&lowered.wire, "Rush")["node_id"]
    );
    assert_eq!(member["population"], population["node_id"]);
}

/// Row 19 (FR-380-AC-12): IR's strict v2 reader admits each package and
/// selects the `Population<Order>[4]` node and its least-ordinal `Type`
/// occurrence, in the row 23 package with the `Rush` node as `declaration`.
#[trace("TC-1054", "FR-380-AC-12")]
#[test]
#[ignore = "QSpec STD-663 and IR-1533: IR's reader does not yet admit `population` on a `field` member of `reaches_field`"]
fn ir_reads_each_field_edge_package_and_selects_its_population() {
    let units = [
        loop_over("chief"),
        loop_over("trail"),
        loop_over("chain"),
        loop_over("pair"),
        RUSH_TRAIL.to_owned(),
    ];
    for unit in units {
        let lowered = lower(&Populations::R, &format!("{BOUND}\n{unit}"));
        assert!(
            lowered.verified,
            "{unit}: IR's v2 reader admits the package"
        );
        let ir = lowered
            .ir
            .as_ref()
            .expect("IR's v2 reader admits the package");
        let application = field_application(&lowered);
        let id: quire_contract_model::CheckedNodeId =
            serde_json::from_str(&application["node_id"].to_string()).unwrap();
        let selector = ir
            .reaches_population(&id)
            .unwrap_or_else(|error| panic!("{unit}: the reader cannot resolve it: {error:?}"))
            .expect("the application names a population");
        let population = order_population(&lowered);
        assert_eq!(
            serde_json::to_value(&selector.binding).unwrap(),
            population["node_id"],
            "{unit}"
        );
        assert_eq!(selector.occurrence.ordinal, 0, "{unit}");
    }
}

/// Row 24 (FR-380-AC-16): no population covers `Order`, and `rushes`, which
/// covers the operand's type, is not walked.
#[trace("TC-1054", "FR-380-AC-16")]
#[test]
fn a_population_covering_only_the_operand_type_is_not_linked() {
    let lowered = lower(
        &Populations::RushesInstead,
        &format!("bound RushesBound on M::rushes [4]\n{RUSH_TRAIL}"),
    );
    assert_eq!(
        field_member(&lowered),
        &json!({
            "kind": "field",
            "declaration": model_node(&lowered.wire, "Rush")["node_id"],
            "name": "trail",
        })
    );
    assert_eq!(population_nodes(&lowered.wire), Vec::<&Value>::new());
}

/// Row 25 (FR-380-AC-17): the clause is admitted, and its only population
/// domain is the context domain `rushes`.
#[trace("TC-1054", "FR-380-AC-17")]
#[test]
fn an_uncovered_edge_type_adds_no_domain_and_no_refusal() {
    let bounded = checked(
        &Populations::RushesInstead,
        &format!("bound RushesBound on M::rushes [4]\n{RUSH_TRAIL}"),
    );
    assert_eq!(unbounded_domains(&bounded), [0], "rushes alone, bounded");
    assert_eq!(
        record_bounds(&bounded, "RushTrail"),
        [("RushesBound".to_owned(), 4)],
        "the context domain is the only bounded domain"
    );
    let unbounded = checked(&Populations::RushesInstead, RUSH_TRAIL);
    assert_eq!(
        unbounded_domains(&unbounded),
        [1],
        "rushes alone, unbounded"
    );
}

/// Row 26 (FR-380-AC-18): the record holds the unbounded context domain
/// `people` and the walked domain `orders` as `Population(Some(4))`.
#[trace("TC-1054", "FR-380-AC-18")]
#[test]
fn a_walked_field_edge_population_is_a_bounded_domain_of_the_record() {
    let bounded = checked(&Populations::R, &format!("{BOUND}\n{FAV_LOOP}"));
    assert_eq!(
        unbounded_domains(&bounded),
        [1],
        "people alone is unbounded"
    );
    assert_eq!(
        record_bounds(&bounded, "FavLoop"),
        [("OrdersBound".to_owned(), 4)]
    );
    let unbounded = checked(&Populations::R, FAV_LOOP);
    assert_eq!(unbounded_domains(&unbounded), [2], "people and orders");
    assert_eq!(record_bounds(&unbounded, "FavLoop"), []);
}

/// Row 28 (FR-380-AC-21): the walked population covers the edge type
/// `Order`, not the operand's `Rush`, so there is no `rushes` domain and no
/// ambiguity.
#[trace("TC-1054", "FR-380-AC-21")]
#[test]
fn the_edge_type_not_the_operand_type_names_the_walked_population() {
    let plain = checked(&Populations::WithRushes, RUSH_FAV);
    assert_eq!(unbounded_domains(&plain), [2], "people and orders");
    let orders = checked(&Populations::WithRushes, &format!("{BOUND}\n{RUSH_FAV}"));
    assert_eq!(unbounded_domains(&orders), [1], "orders is bounded");
    assert_eq!(
        record_bounds(&orders, "RushFav"),
        [("OrdersBound".to_owned(), 4)]
    );
    let rushes = checked(
        &Populations::WithRushes,
        &format!("bound RushesBound on M::rushes [4]\n{RUSH_FAV}"),
    );
    assert_eq!(
        unbounded_domains(&rushes),
        [2],
        "rushes is not walked: people and the unbounded orders stay unbounded"
    );
    assert_eq!(
        record_bounds(&rushes, "RushFav"),
        [],
        "the record holds no rushes(Some 4) domain"
    );
}
