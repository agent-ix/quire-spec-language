// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-104: the `ProtocolClause` family's S3 check of a state clause,
//! against TC-458's `ConfigVersion` fixture package
//! (`crate::model_operations`).
//!
//! **Scope note** (mirrors `model_operations.rs`'s own): `versionNumber` is
//! typed as native `Integer`, not the bound `Int[0, 1000]` scalar FR-104-AC-1
//! describes, since the bound-scalar value-type reader is QSL-289's own
//! scope, not this ticket's (QSL-277). Every assertion below that would
//! otherwise read `Int[0, 1000]` reads `Integer` instead.

use ix_trace_rs::trace;
use qsl_forms::StateClauseKind;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{CheckedGraph, CheckingLimits, Observation, WrongSnapshotCause};
use quire_exact::ValueType;

use crate::model_operations::{
    admit_and_assemble_with_body, archive_population, config_version_document,
    config_version_document_with_population, config_version_identity, empty_frame, operation,
    operation_parameter,
};
use serde_json::json;

/// `attemptUpdate` with `modifies: [versionNumber]`, no parameters, result
/// `Boolean` -- TC-458's own operation, reused verbatim by name.
fn attempt_update_modifies_version_number() -> serde_json::Value {
    operation(
        "attemptUpdate",
        json!([]),
        Some("ix://quire/native/Boolean"),
        json!({
            "modifies": [config_version_identity("versionNumber")],
            "creates": [],
            "deletes": [],
        }),
    )
}

/// `attemptUpdate` with `modifies: [versionNumber, parent]` (TC-459 step 4).
fn attempt_update_modifies_version_and_parent() -> serde_json::Value {
    operation(
        "attemptUpdate",
        json!([]),
        Some("ix://quire/native/Boolean"),
        json!({
            "modifies": [
                config_version_identity("versionNumber"),
                config_version_identity("parent"),
            ],
            "creates": [],
            "deletes": [],
        }),
    )
}

/// `probe(target: ConfigVersion)`, no result, empty frame (TC-459 step 5,
/// TC-466 step 3's fixture variant).
fn probe_operation() -> serde_json::Value {
    let config_version = "ix://example/config-version/ConfigVersion";
    operation(
        "probe",
        json!([operation_parameter("probe", "target", config_version)]),
        None,
        empty_frame(),
    )
}

/// Assembles and checks `body` (the unit's trailing declarations, e.g.
/// state clauses) against `document`, or every refusal's catalog code --
/// an assembly refusal (FR-104 "Resolution", the `using` alias, context and
/// operation, and the population domain) or a check refusal (typing,
/// observations, definedness), whichever stage caught it: both report the
/// same closed catalog codes (`AssemblyCause`/`CheckCause`'s own
/// `code()`/`cause()`), so a test asserts one shape regardless of stage.
fn check(document: &[u8], body: &str) -> Result<CheckedGraph, Vec<(Code, Option<&'static str>)>> {
    let declarations = match admit_and_assemble_with_body(document, body) {
        Ok(declarations) => declarations,
        Err(refusal) => {
            return Err(refusal
                .errors
                .iter()
                .map(|error| (error.cause.code(), Some(error.cause.catalog_code().cause())))
                .collect())
        }
    };
    declarations
        .check(CheckingLimits::default())
        .map_err(|refusals| {
            refusals
                .iter()
                .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
                .collect()
        })
}

const PARENT_ORDER: &str = "invariant ParentOrder using v on Config::ConfigVersion at current { \
    present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }\n";
const NO_CYCLE: &str = "invariant NoCycle using v on Config::ConfigVersion at current { \
    not reaches(self, self, parent) }\n";
const VERSION_UNCHANGED: &str = "post VersionUnchanged using v on \
    Config::ConfigVersion::attemptUpdate { self.versionNumber = pre(self.versionNumber) }\n";

/// TC-459 step 1 / FR-104-AC-1: the ConfigVersion invariants and
/// postcondition each check, typing `self`, `self.versionNumber` and
/// `self.parent`, with `ParentOrder`'s reads all `current` and
/// `VersionUnchanged`'s left read `post` and right `pre`.
#[trace("TC-459", "FR-104-AC-1")]
#[test]
fn the_configversion_state_clauses_check() {
    let document = config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let body = format!("{PARENT_ORDER}{NO_CYCLE}{VERSION_UNCHANGED}");
    let graph = check(&document, &body).expect("the three clauses check");

    let parent_order = graph
        .state_clause("ParentOrder")
        .expect("ParentOrder checked");
    assert_eq!(parent_order.kind(), StateClauseKind::Invariant);
    let no_cycle = graph.state_clause("NoCycle").expect("NoCycle checked");
    assert_eq!(no_cycle.kind(), StateClauseKind::Invariant);
    let version_unchanged = graph
        .state_clause("VersionUnchanged")
        .expect("VersionUnchanged checked");
    assert_eq!(version_unchanged.kind(), StateClauseKind::Postcondition);

    // `self` is `Reference<Config::ConfigVersion>`.
    let (self_name, self_type) = &parent_order.parameters()[0];
    assert_eq!(self_name, "self");
    assert_eq!(self_type, &ValueType::Reference(parent_order.context()));

    // `ParentOrder`'s reads are all `current`.
    let parent_order_reads: Vec<Observation> = parent_order
        .reads()
        .map(|(_, observation)| observation)
        .collect();
    assert!(!parent_order_reads.is_empty());
    assert!(parent_order_reads
        .iter()
        .all(|observation| *observation == Observation::Current));

    // `VersionUnchanged`'s left read is `post`, its right `pre`.
    let mut version_unchanged_reads: Vec<Observation> = version_unchanged
        .reads()
        .map(|(_, observation)| observation)
        .collect();
    version_unchanged_reads.sort();
    assert_eq!(
        version_unchanged_reads,
        vec![Observation::Pre, Observation::Post]
    );
}

/// TC-459 step 2 / FR-104-AC-2: `post R using v on
/// Config::ConfigVersion::attemptUpdate { result }` checks with
/// `result: Boolean`.
#[trace("TC-459", "FR-104-AC-2")]
#[test]
fn a_postcondition_reads_result_typed_as_the_operations_result() {
    let document = config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let body = "post R using v on Config::ConfigVersion::attemptUpdate { result }\n";
    let graph = check(&document, body).expect("`result` checks in a postcondition");
    let r = graph.state_clause("R").expect("R checked");
    assert_eq!(r.body().value_type(), &ValueType::Boolean);
}

/// TC-459 step 3 / FR-104-AC-2: `result` in an invariant, and in a `pre`
/// clause of `attemptUpdate`, each refuse `wrong_snapshot`/`wrong-anchor`.
#[trace("TC-459", "FR-104-AC-2")]
#[test]
fn result_outside_a_postcondition_refuses_wrong_anchor() {
    let document = config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    for body in [
        "invariant I using v on Config::ConfigVersion at current { result }\n",
        "pre Q using v on Config::ConfigVersion::attemptUpdate { result }\n",
    ] {
        let refusals =
            check(&document, body).expect_err("`result` refuses outside a postcondition");
        assert_eq!(refusals.len(), 1);
        assert_eq!(
            refusals[0],
            (
                Code::WrongSnapshot,
                Some(WrongSnapshotCause::WrongAnchor.as_str())
            )
        );
    }
}

/// TC-459 step 4 / FR-104-AC-7, over a package whose `attemptUpdate` frame
/// also modifies `parent`.
#[trace("TC-459", "FR-104-AC-7")]
#[test]
fn postconditions_of_an_operation_that_modifies_parent() {
    let document = config_version_document(
        attempt_update_modifies_version_and_parent(),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    // (a) refuses `unproved-presence` at `value(self.parent)`: a fact about
    // `pre(self.parent)` never discharges `value(self.parent)` at `post`.
    let a = "post A using v on Config::ConfigVersion::attemptUpdate { \
        present(pre(self.parent)) implies deref(value(self.parent)).versionNumber > 0 }\n";
    let refusals = check(&document, a).expect_err("(a) refuses unproved-presence");
    assert_eq!(refusals.len(), 1);
    assert_eq!(
        refusals[0],
        (Code::UndefinedExpression, Some("unproved-presence"))
    );

    // (b) checks: the whole condition is under `pre`, every read `pre`.
    let b = "post B using v on Config::ConfigVersion::attemptUpdate { \
        pre(present(self.parent) implies deref(value(self.parent)).versionNumber > 0) }\n";
    let graph = check(&document, b).expect("(b) checks");
    let clause = graph.state_clause("B").expect("B checked");
    assert!(clause
        .reads()
        .all(|(_, observation)| observation == Observation::Pre));

    // (c), (d) refuse forbidden-pre-read: a bare or let-aliased read has no
    // eligible syntax of its own inside the `pre`.
    for (name, body) in [
        (
            "C",
            "post C using v on Config::ConfigVersion::attemptUpdate { \
                let x = self.versionNumber in pre(x) = 1 }\n",
        ),
        (
            "D",
            "post D using v on Config::ConfigVersion::attemptUpdate { \
                let s = self in pre(s.versionNumber) = 1 }\n",
        ),
    ] {
        let refusals = check(&document, body)
            .err()
            .unwrap_or_else(|| panic!("{name} refuses forbidden-pre-read"));
        assert_eq!(refusals.len(), 1, "{name}");
        assert_eq!(
            refusals[0],
            (
                Code::WrongSnapshot,
                Some(WrongSnapshotCause::ForbiddenPreRead.as_str())
            ),
            "{name}"
        );
    }

    // (e) checks: `let s = pre(self) in s.versionNumber = 1`, its read `pre`.
    let e = "post E using v on Config::ConfigVersion::attemptUpdate { \
        let s = pre(self) in s.versionNumber = 1 }\n";
    let graph = check(&document, e).expect("(e) checks");
    let clause = graph.state_clause("E").expect("E checked");
    assert!(clause
        .reads()
        .all(|(_, observation)| observation == Observation::Pre));
}

/// TC-459 step 5 / FR-104-AC-8, over a package variant that adds
/// `probe(target: ConfigVersion)` with no result and an empty frame.
#[trace("TC-459", "FR-104-AC-8")]
#[test]
fn clauses_of_an_operation_with_a_reference_typed_parameter() {
    let document = config_version_document(probe_operation(), Vec::new(), Vec::new(), json!([]));

    // (a) the postcondition `deref(target).versionNumber = 1` reads `post`.
    let a = "post A using v on Config::ConfigVersion::probe { \
        deref(target).versionNumber = 1 }\n";
    let graph = check(&document, a).expect("(a) checks");
    assert!(graph
        .state_clause("A")
        .expect("A checked")
        .reads()
        .all(|(_, observation)| observation == Observation::Post));

    // (b) the precondition, same body, reads `pre`.
    let b = "pre B using v on Config::ConfigVersion::probe { \
        deref(target).versionNumber = 1 }\n";
    let graph = check(&document, b).expect("(b) checks");
    assert!(graph
        .state_clause("B")
        .expect("B checked")
        .reads()
        .all(|(_, observation)| observation == Observation::Pre));

    // (c) the postcondition `pre(deref(target).versionNumber) = 1` reads
    // `pre`.
    let c = "post C using v on Config::ConfigVersion::probe { \
        pre(deref(target).versionNumber) = 1 }\n";
    let graph = check(&document, c).expect("(c) checks");
    assert!(graph
        .state_clause("C")
        .expect("C checked")
        .reads()
        .all(|(_, observation)| observation == Observation::Pre));

    // (d) the postcondition `pre(target) = target` refuses
    // `forbidden-pre-read`: a bare parameter alone is never eligible.
    let d = "post D using v on Config::ConfigVersion::probe { pre(target) = target }\n";
    let refusals = check(&document, d).expect_err("(d) refuses forbidden-pre-read");
    assert_eq!(refusals.len(), 1);
    assert_eq!(
        refusals[0],
        (
            Code::WrongSnapshot,
            Some(WrongSnapshotCause::ForbiddenPreRead.as_str())
        )
    );
}

/// TC-460 / FR-104-AC-3, AC-4: each ill-formed state clause refuses its own
/// catalog code, and no row yields a checked clause.
#[trace("TC-460", "FR-104-AC-3")]
#[test]
fn missing_and_ambiguous_names_refuse_at_their_locus() {
    let document = config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    );

    // Row 1: `on Config::Missing` refuses missing-name.
    let row1 = "invariant A using v on Config::Missing at current { true }\n";
    let refusals = check(&document, row1).expect_err("row 1 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(
        refusals[0],
        (Code::MissingDeclaration, Some("missing-name"))
    );

    // Row 4: a second `ParentOrder` refuses ambiguous-name at both.
    let row4 = format!("{PARENT_ORDER}{PARENT_ORDER}");
    let refusals = check(&document, &row4).expect_err("row 4 refuses");
    assert!(refusals
        .iter()
        .all(|refusal| *refusal == (Code::AmbiguousDeclaration, Some("ambiguous-name"))));
    assert_eq!(refusals.len(), 2);

    // Row 9: `function ParentOrder` beside the clause refuses
    // ambiguous-name at both declarations (FR-109's shared namespace).
    let row9 = format!("{PARENT_ORDER}function ParentOrder using v(): Boolean pure {{ true }}\n");
    let refusals = check(&document, &row9).expect_err("row 9 refuses");
    assert!(refusals
        .iter()
        .all(|refusal| *refusal == (Code::AmbiguousDeclaration, Some("ambiguous-name"))));
    assert_eq!(refusals.len(), 2);
}

/// TC-460 rows 2, 3, 5-8, 10 / FR-104-AC-3, AC-4.
#[trace("TC-460", "FR-104-AC-4")]
#[test]
fn ill_typed_and_operator_ineligible_clauses_refuse() {
    let document = config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    );

    // Row 2: `on Config::ConfigVersion::missing` refuses missing-name.
    let row2 = "pre B using v on Config::ConfigVersion::missing { true }\n";
    let refusals = check(&document, row2).expect_err("row 2 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(
        refusals[0],
        (Code::MissingDeclaration, Some("missing-name"))
    );

    // Row 3: a non-Boolean invariant body refuses `non-boolean-root`.
    let row3 = "invariant C using v on Config::ConfigVersion at current { self.versionNumber }\n";
    let refusals = check(&document, row3).expect_err("row 3 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0], (Code::IllTyped, Some("non-boolean-root")));

    // Row 5: `pre(self.versionNumber)` in an invariant refuses
    // forbidden-pre-read (`pre` is legal only in a postcondition).
    let row5 = "invariant D using v on Config::ConfigVersion at current { \
        pre(self.versionNumber) = 1 }\n";
    let refusals = check(&document, row5).expect_err("row 5 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(
        refusals[0],
        (
            Code::WrongSnapshot,
            Some(WrongSnapshotCause::ForbiddenPreRead.as_str())
        )
    );

    // Row 6: `pre(result)` in a postcondition refuses forbidden-pre-read.
    let row6 = "post E using v on Config::ConfigVersion::attemptUpdate { pre(result) }\n";
    let refusals = check(&document, row6).expect_err("row 6 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(
        refusals[0],
        (
            Code::WrongSnapshot,
            Some(WrongSnapshotCause::ForbiddenPreRead.as_str())
        )
    );

    // Row 7: `reaches(self, self, versionNumber)` refuses
    // operator-ineligible: `versionNumber` is not reference-typed.
    let row7 = "invariant F using v on Config::ConfigVersion at current { \
        reaches(self, self, versionNumber) }\n";
    let refusals = check(&document, row7).expect_err("row 7 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0], (Code::IllTyped, Some("operator-ineligible")));

    // Row 8: `deref(value(self.parent)).versionNumber < 5` unguarded refuses
    // unproved-presence at the `value`.
    let row8 = "invariant G using v on Config::ConfigVersion at current { \
        deref(value(self.parent)).versionNumber < 5 }\n";
    let refusals = check(&document, row8).expect_err("row 8 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(
        refusals[0],
        (Code::UndefinedExpression, Some("unproved-presence"))
    );

    // Row 10: `reaches` in a function body refuses operator-ineligible.
    let row10 = "function r using v(x: Config::ConfigVersion, y: Config::ConfigVersion): \
        Boolean pure { reaches(x, y, parent) }\n";
    let refusals = check(&document, row10).expect_err("row 10 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0], (Code::IllTyped, Some("operator-ineligible")));
}

/// TC-461 / FR-104-AC-5, AC-6: one `operation-contract` requirement per
/// clause and per named frame, and stable clause identity.
#[trace("TC-461", "FR-104-AC-5")]
#[test]
fn one_requirement_record_per_clause_and_frame() {
    let document = config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let body = format!("{PARENT_ORDER}{NO_CYCLE}{VERSION_UNCHANGED}");

    // Step 1: four records.
    let graph = check(&document, &body).expect("the unit checks");
    assert_eq!(graph.requirements().len(), 4);
    for record in graph.requirements().values() {
        assert_eq!(
            record.requirements().kind(),
            qsl_semantics::check::Capability::OperationContract
        );
    }

    // Step 2: without `VersionUnchanged`, two records, no frame record.
    let without = format!("{PARENT_ORDER}{NO_CYCLE}");
    let graph = check(&document, &without).expect("the unit checks without VersionUnchanged");
    assert_eq!(graph.requirements().len(), 2);
}

/// TC-461 step 3 / FR-104-AC-6: checking the same unit twice, and with its
/// clauses reordered, gives each clause the same node identity and the same
/// requirement record keys.
#[trace("TC-461", "FR-104-AC-6")]
#[test]
fn clause_identity_and_requirement_keys_are_stable() {
    let document = config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let forward = format!("{PARENT_ORDER}{NO_CYCLE}{VERSION_UNCHANGED}");
    let reverse = format!("{VERSION_UNCHANGED}{NO_CYCLE}{PARENT_ORDER}");

    let a = check(&document, &forward).expect("forward order checks");
    let b = check(&document, &forward).expect("forward order checks again");
    let c = check(&document, &reverse).expect("reverse order checks");

    for name in ["ParentOrder", "NoCycle", "VersionUnchanged"] {
        let identity_a = a.state_clause(name).expect("checked").identity();
        let identity_b = b.state_clause(name).expect("checked").identity();
        let identity_c = c.state_clause(name).expect("checked").identity();
        assert_eq!(identity_a, identity_b, "{name}");
        assert_eq!(identity_a, identity_c, "{name}");
    }
    let mut keys_a: Vec<_> = a.requirements().keys().collect();
    let mut keys_b: Vec<_> = b.requirements().keys().collect();
    let mut keys_c: Vec<_> = c.requirements().keys().collect();
    keys_a.sort();
    keys_b.sort();
    keys_c.sort();
    assert_eq!(keys_a, keys_b);
    assert_eq!(keys_a, keys_c);
}

/// TC-461 step 4 / FR-104-AC-6: `ParentOrder2` (an added clause with
/// `ParentOrder`'s own body) shares `ParentOrder`'s node identity, and the
/// unit yields five records.
#[trace("TC-461", "FR-104-AC-6")]
#[test]
fn two_clauses_of_equal_kind_anchor_and_body_share_identity() {
    let document = config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let parent_order_2 = PARENT_ORDER.replacen("ParentOrder", "ParentOrder2", 1);
    let body = format!("{PARENT_ORDER}{NO_CYCLE}{VERSION_UNCHANGED}{parent_order_2}");
    let graph = check(&document, &body).expect("the unit checks");

    let parent_order = graph.state_clause("ParentOrder").expect("checked");
    let parent_order_2 = graph.state_clause("ParentOrder2").expect("checked");
    assert_eq!(parent_order.identity(), parent_order_2.identity());
    assert_eq!(graph.requirements().len(), 5);
}

/// TC-461 step 5 / FR-104 "Requirements": over a package with a second,
/// unbounded population `archive` over `ConfigVersion`, each clause refuses
/// `ambiguous_declaration`/`ambiguous-name` at its `on`, naming `archive`
/// and `config_history`.
///
/// **Spec question**: FR-104's Behavior section ("population domain key")
/// also describes `archive` *declaring a maximum of 10* as the unambiguous,
/// unchanged-four-records branch. The domain package's own population
/// record carries no static maximum -- only `PopulationBinding::
/// declared_maximum` does, a per-invocation/evaluation fact
/// (`qsl-semantics/src/model/population.rs:551-560`), never a domain
/// package declaration (`qsl-semantics/src/model/domain_package.rs:465-479`,
/// confirmed by `AdmittedModel::populations_of`'s own doc,
/// `qsl-semantics/src/check/lowering/model.rs:114-124`: "A domain package
/// population declares no maximum ... so each one is an unbounded
/// `Population(None)` domain"). That branch is therefore not
/// representable at check time today and is not exercised here.
#[trace("TC-461", "FR-104-AC-5")]
#[test]
fn two_no_maximum_populations_of_one_type_refuse_ambiguous_name() {
    let document = config_version_document_with_population(
        attempt_update_modifies_version_number(),
        archive_population(),
    );
    let body = format!("{PARENT_ORDER}{NO_CYCLE}{VERSION_UNCHANGED}");
    let refusals = check(&document, &body).expect_err("two no-maximum populations refuse");
    assert_eq!(refusals.len(), 3, "one refusal per clause");
    for refusal in &refusals {
        assert_eq!(
            *refusal,
            (Code::AmbiguousDeclaration, Some("ambiguous-name"))
        );
    }
}
