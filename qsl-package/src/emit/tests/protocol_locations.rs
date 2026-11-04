// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1063 / TC-1070: the FR-383 `checked_term_data.protocol_locations` rows
//! the emitter writes for TC-1070 fixture E and the E-C controls, and the
//! QSpec FR-322 reader's verdict on the emitted rows and on edits of them
//! (FR-383-AC-32..34, AC-39..41, AC-43..61; FR-390-AC-39, AC-40). Every
//! verdict on an edited package is the reader's.

use super::protocol_controls::{
    assert_refused_at, expression_occurrences_of_node_at, read_protocol_over, tc1070,
    E_C_TRIED_PICKED,
};
use super::*;
use quire_contract_model::{CheckedPackageRefusalCause, CheckedPackageRefusalCode};
use quire_semantic_value::location::{Location, Origin};

const ROWS: &str = "/checked_term_data/protocol_locations";

/// `source` checked over fixture E's domain package, with its emitted wire.
fn checked_and_emitted(source: &str) -> (CheckedPackage, Value) {
    let packages = qsl_semantics::model::intake::package_input([domain().as_slice()]).unwrap();
    let graph = assemble_with_models(source.as_bytes(), &packages)
        .expect("fixture assembles")
        .check(CheckingLimits::default())
        .expect("fixture checks");
    let package = CheckedPackage::link(graph);
    let emission = emit_checked(&package).expect("fixture emits");
    assert!(emission.omitted.is_empty(), "{:?}", emission.omitted);
    (package, wire(&emission))
}

fn domain() -> Vec<u8> {
    serde_json::to_vec(&tc1070::document()).unwrap()
}

fn rows(wire: &Value) -> &[Value] {
    wire["checked_term_data"]["protocol_locations"]
        .as_array()
        .expect("the emitted package carries protocol_locations")
}

fn origin_name(row: &Value) -> &str {
    assert_eq!(row["location"]["origin"]["kind"], "type_declaration");
    row["location"]["origin"]["name"].as_str().unwrap()
}

fn path(row: &Value) -> Vec<u64> {
    row["location"]["path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| step.as_u64().unwrap())
        .collect()
}

/// The one row under origin `name` at child path `at`.
fn row_at<'w>(wire: &'w Value, name: &str, at: &[u64]) -> &'w Value {
    let mut found = rows(wire)
        .iter()
        .filter(|row| origin_name(row) == name && path(row) == at);
    let row = found
        .next()
        .unwrap_or_else(|| panic!("a row under {name} at {at:?}"));
    assert!(found.next().is_none(), "one row under {name} at {at:?}");
    row
}

/// Every row of `application` under origin `name`.
fn rows_of<'w>(wire: &'w Value, application: &Value, name: &str) -> Vec<&'w Value> {
    rows(wire)
        .iter()
        .filter(|row| row["application"] == *application && origin_name(row) == name)
        .collect()
}

fn key(row: &Value) -> (u64, u64) {
    (
        row["occurrence"]["ordinal"].as_u64().unwrap(),
        row["term_ordinal"].as_u64().unwrap(),
    )
}

/// The checked package's `Location` as the wire writes it.
fn wire_path(location: &Location) -> Vec<u64> {
    location
        .path()
        .into_iter()
        .map(|step| step as u64)
        .collect()
}

fn refunds(slot: &str) -> String {
    format!("protocol Refunds compensate Refund {slot}")
}

fn site(name: &str, at: &[usize]) -> Location {
    Location::at(
        Origin::TypeDeclaration {
            name: name.to_owned(),
        },
        at,
    )
}

/// FR-383-AC-58 over every application and origin of `wire`: taken in
/// ascending (occurrence ordinal, `term_ordinal`) order, the rows of one
/// application and origin follow the slot walk, whose pre-order is the
/// lexicographic order of the positions' child paths.
fn assert_rows_follow_the_walk(wire: &Value) {
    let mut groups: BTreeMap<(String, String), Vec<&Value>> = BTreeMap::new();
    for row in rows(wire) {
        groups
            .entry((row["application"].to_string(), origin_name(row).to_owned()))
            .or_default()
            .push(row);
    }
    for (group, mut members) in groups {
        members.sort_by_key(|row| key(row));
        let keys: Vec<_> = members.iter().map(|row| key(row)).collect();
        assert!(
            keys.windows(2).all(|pair| pair[0] < pair[1]),
            "{group:?}: {keys:?}"
        );
        let paths: Vec<_> = members.iter().map(|row| path(row)).collect();
        assert!(
            paths.windows(2).all(|pair| pair[0] < pair[1]),
            "{group:?}: {paths:?}"
        );
    }
}

/// TC-1063 L5..L8, FR-383-AC-34/50/52/53/54: fixture E writes one row per
/// visit, each keyed to the `expression` occurrence the checker recorded at
/// that visit's position and carrying that position's `Location`.
#[trace(
    "TC-1063",
    "FR-383-AC-32",
    "FR-383-AC-34",
    "FR-383-AC-50",
    "FR-383-AC-51",
    "FR-383-AC-52",
    "FR-383-AC-53",
    "FR-383-AC-54",
    "FR-383-AC-58"
)]
#[test]
fn fixture_e_keys_each_visit_to_the_occurrence_recorded_at_its_position() {
    let (package, wire) = checked_and_emitted(tc1070::SOURCE);
    let graph = package.graph();
    let retry = refunds("retry 0");
    let guard = refunds("trigger-guard 0");

    // `earlier.tally` in `earlier.tally < 3`, then in `earlier.tally + 1`.
    let in_less = row_at(&wire, &retry, &[0, 0, 0, 0]);
    let in_plus = row_at(&wire, &retry, &[0, 0, 1, 1, 0]);
    let tally = &in_less["application"];
    assert_eq!(in_plus["application"], *tally);
    assert_eq!(rows_of(&wire, tally, &retry).len(), 2);
    assert_eq!(
        rows(&wire)
            .iter()
            .filter(|row| row["application"] == *tally)
            .count(),
        2
    );
    assert_eq!((key(in_less), key(in_plus)), ((0, 0), (1, 0)));
    let recorded = expression_occurrences_of_node_at(graph, &site(&retry, &[0, 0, 0, 0]));
    assert_eq!(
        recorded
            .iter()
            .map(|(ordinal, location)| (*ordinal, wire_path(location)))
            .collect::<Vec<_>>(),
        [(0, path(in_less)), (1, path(in_plus))]
    );

    // `target >= 0`: one node held by the trigger guard and the retry
    // relation, occurrence 0 in the guard and 1 in the retry relation.
    let in_guard = row_at(&wire, &guard, &[1]);
    let in_retry = row_at(&wire, &retry, &[0, 1]);
    assert_eq!(in_guard["application"], in_retry["application"]);
    let target = &in_guard["application"];
    assert_eq!(rows_of(&wire, target, &guard), [in_guard]);
    assert_eq!(rows_of(&wire, target, &retry), [in_retry]);
    assert_eq!((key(in_guard), key(in_retry)), ((0, 0), (1, 0)));
    assert_ne!(in_guard["occurrence"], in_retry["occurrence"]);

    // Partial's own `partialEarlier.tally` has two rows of its own origin.
    let partial = "protocol Refunds compensate Partial retry 0";
    let partial_tally = &row_at(&wire, partial, &[0, 0, 0, 0])["application"];
    assert_ne!(partial_tally, tally);
    assert_eq!(rows_of(&wire, partial_tally, partial).len(), 2);

    assert_rows_follow_the_walk(&wire);
}

/// FR-383-AC-33/39/40/41: every row is under a type-declaration origin that
/// names its slot's site, no row names a `protocol_attempt` origin, and the
/// rows of one origin belong to one slot.
#[trace(
    "TC-1063",
    "FR-383-AC-33",
    "FR-383-AC-39",
    "FR-383-AC-40",
    "FR-383-AC-41"
)]
#[test]
fn every_fixture_e_row_is_under_its_slots_site_name() {
    let (_, wire) = checked_and_emitted(tc1070::SOURCE);
    assert!(!rows(&wire).is_empty());
    let names: BTreeSet<&str> = rows(&wire).iter().map(origin_name).collect();
    for name in &names {
        assert!(
            name.starts_with("protocol Refunds ") || name.starts_with("protocol Pipeline "),
            "{name}"
        );
    }
    // The slots of `Refund` that hold applications, one site name each.
    for slot in ["trigger-guard 0", "retry 0", "recovery 0"] {
        assert!(names.contains(refunds(slot).as_str()), "{slot}: {names:?}");
    }
    assert!(names.contains("protocol Refunds run Main Applied body"));
}

/// TC-1063 rows over fixture E's emitted rows: the QSpec FR-322 reader
/// admits them (FR-383-AC-19's read-back, extended to the populated array)
/// and a package with the occurrence-1 `earlier.tally` row removed
/// (FR-383-AC-43).
#[trace("TC-1063", "FR-383-AC-43")]
#[test]
fn the_reader_admits_the_rows_and_a_package_missing_one_whole_group() {
    let (_, wire) = checked_and_emitted(tc1070::SOURCE);
    let document = domain();
    let admitted = read_protocol_over(&wire, &document);
    assert!(matches!(admitted, Read::Verified { .. }), "{admitted:?}");
    let mut removed = wire.clone();
    let index = occurrence_one_tally(&wire);
    removed["checked_term_data"]["protocol_locations"]
        .as_array_mut()
        .unwrap()
        .remove(index);
    let outcome = read_protocol_over(&removed, &document);
    assert!(matches!(outcome, Read::Verified { .. }), "{outcome:?}");
}

/// The index of the retry-origin `earlier.tally` row keyed to occurrence 1.
fn occurrence_one_tally(wire: &Value) -> usize {
    let row = row_at(wire, &refunds("retry 0"), &[0, 0, 1, 1, 0]);
    assert_eq!(key(row), (1, 0));
    rows(wire).iter().position(|found| found == row).unwrap()
}

/// FR-383-AC-44 / FR-390-AC-39 (TC-1070 C7): a group with no ordinal-zero
/// row is refused at I2.
#[trace("TC-1063", "TC-1070", "FR-383-AC-44", "FR-390-AC-39")]
#[test]
fn a_group_with_no_ordinal_zero_row_is_refused_at_i2() {
    let (_, mut wire) = checked_and_emitted(tc1070::SOURCE);
    let index = occurrence_one_tally(&wire);
    wire["checked_term_data"]["protocol_locations"][index]["term_ordinal"] = json!(1);
    assert_refused_at(
        read_protocol_over(&wire, &domain()),
        CheckedPackageRefusalCode::InvalidPackage,
        Some(CheckedPackageRefusalCause::InvalidValue),
        ROWS,
    );
}

/// FR-383-AC-45: the occurrence-1 row rekeyed to occurrence 0 at
/// `term_ordinal` 2 leaves that group the ordinals 0 and 2.
#[trace("TC-1063", "FR-383-AC-45")]
#[test]
fn a_group_that_skips_an_ordinal_is_refused_at_i2() {
    let (_, mut wire) = checked_and_emitted(tc1070::SOURCE);
    let index = occurrence_one_tally(&wire);
    let row = &mut wire["checked_term_data"]["protocol_locations"][index];
    row["occurrence"]["ordinal"] = json!(0);
    row["term_ordinal"] = json!(2);
    assert_refused_at(
        read_protocol_over(&wire, &domain()),
        CheckedPackageRefusalCode::InvalidPackage,
        Some(CheckedPackageRefusalCause::InvalidValue),
        ROWS,
    );
}

/// FR-383-AC-46 / FR-390-AC-40 (TC-1070 C7a): a duplicated row is refused at
/// I2 at the duplicate's index.
#[trace("TC-1063", "TC-1070", "FR-383-AC-46", "FR-390-AC-40")]
#[test]
fn a_duplicated_row_is_refused_at_i2_at_the_duplicate() {
    let (_, mut wire) = checked_and_emitted(tc1070::SOURCE);
    let index = occurrence_one_tally(&wire);
    let rows = wire["checked_term_data"]["protocol_locations"]
        .as_array_mut()
        .unwrap();
    let duplicate = rows[index].clone();
    rows.insert(index + 1, duplicate);
    assert_refused_at(
        read_protocol_over(&wire, &domain()),
        CheckedPackageRefusalCode::InvalidPackage,
        Some(CheckedPackageRefusalCause::ConflictingDefinition),
        &format!("{ROWS}/{}", index + 1),
    );
}

/// E-C's `Tried` and `Picked` controls (TC-1070 E-C) with its `Composed.if`
/// shape, over fixture E's `M::Tick{ready, count}`; `LET` is where
/// [`e_c_with_let`] adds the `Composed.let` shape.
const E_C_LET_IF: &str = r#"
language "ix:native" edition "1-draft";
profile P = "quire.protocol.finite-global/v1";
model M = "test/orders";
protocol Composed using P over (view: M::Node) on origin {
  capture order: M::Total = view.total;
  role Service on M::Node;
  run sequence Main {
    attempt Tried by Service on M::Node::step contracts []
        as (t: M::Flag) { order >= 0 };
    attempt Picked by Service on M::Node::step contracts []
        as (p: M::Flag) { order >= 0 };
    LET
    attempt Branch by Service on M::Node::step contracts []
        as (im: M::Tick) { if im.ready then im.count >= 1 else im.count + 1 >= 2 };
  }
  finish Closed as (closed: M::Flag) { true };
}
"#;

/// [`E_C_LET_IF`] with its `Composed.let` control `Bound`, or without it.
fn e_c_with_let(with: bool) -> String {
    E_C_LET_IF.replace(
        "    LET\n",
        if with {
            "attempt Bound by Service on M::Node::step contracts [] \
             as (lm: M::Tick) { let c = lm.count in c + c >= 1 };\n"
        } else {
            ""
        },
    )
}

fn composed(attempt: &str) -> String {
    format!("protocol Composed run Main {attempt} body")
}

/// FR-383-AC-55/56/57: `order >= 0` has one row under `Tried`'s origin keyed
/// to occurrence 0 and one under `Picked`'s keyed to occurrence 1, each
/// alone in its group with `term_ordinal` 0.
#[trace("TC-1063", "FR-383-AC-55", "FR-383-AC-56", "FR-383-AC-57")]
#[test]
fn e_c_shared_constraint_rows_are_keyed_tried_zero_picked_one() {
    let (_, wire) = checked_and_emitted(E_C_TRIED_PICKED);
    let tried = row_at(&wire, &composed("Tried"), &[]);
    let picked = row_at(&wire, &composed("Picked"), &[]);
    assert_eq!(tried["application"], picked["application"]);
    assert_eq!((key(tried), key(picked)), ((0, 0), (1, 0)));
    assert_eq!(
        rows(&wire)
            .iter()
            .filter(|row| row["application"] == tried["application"])
            .count(),
        2
    );
    let admitted = read_protocol_over(&wire, &domain());
    assert!(matches!(admitted, Read::Verified { .. }), "{admitted:?}");
}

/// FR-383-AC-47/48/49/59: in `Composed.if` the attribute read `im.count` has
/// a row at each of its two positions, the unselected-at-runtime else-branch
/// included, each carrying the checked clause's `Location`; in
/// `Composed.let` `lm.count` has one row. No row names a `Local` read, a
/// literal, the `let` or the conditional.
#[trace(
    "TC-1063",
    "FR-383-AC-47",
    "FR-383-AC-48",
    "FR-383-AC-49",
    "FR-383-AC-58",
    "FR-383-AC-59"
)]
#[test]
fn e_c_let_and_if_rows_cover_every_term_and_no_non_term() {
    let (package, wire) = checked_and_emitted(&e_c_with_let(true));
    let graph = package.graph();
    let branch = composed("Branch");
    // `if im.ready then im.count >= 1 else im.count + 1 >= 2`: the
    // condition, then `>=` over `im.count`, then `>=` over `+` over
    // `im.count`.
    let then_count = row_at(&wire, &branch, &[1, 0]);
    let else_count = row_at(&wire, &branch, &[2, 0, 0]);
    let count = &then_count["application"];
    assert_eq!(else_count["application"], *count);
    assert_eq!(rows_of(&wire, count, &branch).len(), 2);
    for row in [then_count, else_count] {
        let at = site(
            &branch,
            &path(row)
                .iter()
                .map(|step| *step as usize)
                .collect::<Vec<_>>(),
        );
        assert!(
            expression_occurrences_of_node_at(graph, &at)
                .iter()
                .any(|(ordinal, location)| *location == at && key(row).0 == *ordinal),
            "{row}"
        );
    }
    // The else-branch's `+` and its `>=` have rows too.
    row_at(&wire, &branch, &[2, 0]);
    row_at(&wire, &branch, &[2]);
    // The condition `im.ready` is a term; the conditional at the root is not.
    row_at(&wire, &branch, &[0]);
    assert!(rows(&wire)
        .iter()
        .all(|row| !(origin_name(row) == branch && path(row).is_empty())));

    // `let c = lm.count in c + c >= 1`: `lm.count` once, the comparison and
    // the sum; the `let` at the root and the two `c` reads have none.
    let bound = composed("Bound");
    let bound_rows: Vec<_> = rows(&wire)
        .iter()
        .filter(|row| origin_name(row) == bound)
        .map(path)
        .collect();
    assert_eq!(bound_rows.len(), 3, "{bound_rows:?}");
    assert!(!bound_rows.contains(&Vec::new()), "{bound_rows:?}");

    // Every row names an application node, never a literal, a parameter
    // (the `Local` reads) or the conditional.
    for row in rows(&wire) {
        let node = nodes(&wire)
            .iter()
            .find(|node| node["node_id"] == row["application"])
            .expect("the row names a node of the package");
        assert_eq!(node["node_tag"], "expression", "{node}");
        assert_eq!(node["body"]["term"], "application", "{node}");
        assert_ne!(
            node["body"]["operation"]["identity"], "quire.op.control.if",
            "{node}"
        );
    }
    assert_rows_follow_the_walk(&wire);
    // The reader refuses a `let` binder in a protocol slot (`BinderScope`),
    // whatever its rows, so the read-back runs without `Composed.let`.
    let (_, without_let) = checked_and_emitted(&e_c_with_let(false));
    let admitted = read_protocol_over(&without_let, &domain());
    assert!(matches!(admitted, Read::Verified { .. }), "{admitted:?}");
}
