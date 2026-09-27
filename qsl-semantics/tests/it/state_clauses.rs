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
use qsl_foundation::bound::DomainKey;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    AssemblyCause, AssemblyError, CheckCause, CheckRefusal, CheckedGraph, CheckingLimits, NodeKind,
    Observation, WrongSnapshotCause,
};
use qsl_semantics::family::{ClaimExtent, DomainKind};
use quire_exact::ValueType;

use crate::model_operations::{
    admit_and_assemble_with_body, ambiguous_operation_document, archive_population,
    config_unit_with_body, config_version_document, config_version_document_with_operations,
    config_version_document_with_population, config_version_identity, empty_frame, operation,
    operation_parameter, subtype_document,
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

/// Like [`check`], but keeps the raw [`CheckRefusal`] of a check-stage
/// refusal (never an assembly one -- callers use this only where the
/// refusal it wants is `check`'s own: `CheckCause::UnanchoredResult`'s
/// payload (FR-104-AC-2), or a refusal's own `location.path` (FR-096;
/// empty at the checked declaration's root, non-empty at a subexpression).
fn check_refusals(document: &[u8], body: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let declarations = admit_and_assemble_with_body(document, body)
        .unwrap_or_else(|refusal| panic!("assembly refused: {refusal:?}"));
    declarations.check(CheckingLimits::default())
}

/// Like [`check`], but keeps every assembly-stage refusal's raw
/// [`AssemblyError`] (SR-737 FND-002): its typed `AssemblyCause` payload
/// (e.g. `AmbiguousPopulation`'s `context`/`populations`) and its exact
/// `span`, rather than `check`'s packed catalog-code tuple.
fn assembly_refusals(document: &[u8], body: &str) -> Vec<AssemblyError> {
    admit_and_assemble_with_body(document, body)
        .expect_err("assembly refused")
        .errors
}

/// Every node of `root`'s subtree, root included. `Node::descendants` is
/// crate-private; `Node::children` is public, so this walks with it.
fn descendants(root: &qsl_semantics::check::Node) -> Vec<&qsl_semantics::check::Node> {
    let mut all = vec![root];
    let mut stack = root.children();
    while let Some(node) = stack.pop() {
        all.push(node);
        stack.extend(node.children());
    }
    all
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

    // `self.versionNumber` is an `Attribute` node. Its type is `Integer`,
    // this module's own stand-in for FR-104-AC-1's `Int[0, 1000]`: the
    // bound-scalar value-type reader is QSL-289's scope, not this
    // ticket's -- see this file's own header note. `self.parent` is an
    // `Attribute` node of `Option<Reference<Config::ConfigVersion>>`.
    let attribute_types: Vec<&ValueType> = descendants(parent_order.body())
        .into_iter()
        .filter(|node| matches!(node.kind(), NodeKind::Attribute { .. }))
        .map(|node| node.value_type())
        .collect();
    assert!(
        attribute_types.contains(&&ValueType::Integer),
        "self.versionNumber: Integer (QSL-289 unverified: Int[0, 1000]): {attribute_types:?}"
    );
    let parent_type = ValueType::Option(Box::new(ValueType::Reference(parent_order.context())));
    assert!(
        attribute_types.contains(&&parent_type),
        "self.parent: Option<Reference<Config::ConfigVersion>>: {attribute_types:?}"
    );

    // `ParentOrder`'s reads are all `current`, and it has a `Reaches` read
    // (`not reaches(self, self, parent)`) -- `NoCycle`'s own body, checked
    // alongside it here since both share this fixture and this assertion.
    let parent_order_reads: Vec<Observation> = parent_order
        .reads()
        .map(|(_, observation)| observation)
        .collect();
    assert!(!parent_order_reads.is_empty());
    assert!(parent_order_reads
        .iter()
        .all(|observation| *observation == Observation::Current));
    assert!(
        descendants(no_cycle.body())
            .into_iter()
            .any(|node| matches!(node.kind(), NodeKind::Reaches { .. })),
        "NoCycle's body has a Reaches node"
    );
    let no_cycle_reads: Vec<Observation> = no_cycle
        .reads()
        .map(|(_, observation)| observation)
        .collect();
    assert!(!no_cycle_reads.is_empty());
    assert!(no_cycle_reads
        .iter()
        .all(|observation| *observation == Observation::Current));

    // `pre(self.versionNumber)` (`VersionUnchanged`'s right operand) is a
    // `Pre` node over that same `Attribute`.
    assert!(
        descendants(version_unchanged.body())
            .into_iter()
            .any(|node| matches!(
                node.kind(),
                NodeKind::Pre(operand) if matches!(operand.kind(), NodeKind::Attribute { .. })
            )),
        "pre(self.versionNumber) is a Pre node over an Attribute node"
    );

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
    let cases = [
        (
            "invariant I using v on Config::ConfigVersion at current { result }\n",
            Some(StateClauseKind::Invariant),
            None,
        ),
        (
            "pre Q using v on Config::ConfigVersion::attemptUpdate { result }\n",
            Some(StateClauseKind::Precondition),
            Some("attemptUpdate"),
        ),
    ];
    for (body, expected_kind, expected_operation) in cases {
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

        // FR-104-AC-2: the refusal names the clause kind and the operation,
        // at `result`, the whole body here, so the locus is the root.
        let refusals = check_refusals(&document, body).expect_err("check-stage refusal");
        assert_eq!(refusals.len(), 1);
        assert_eq!(
            refusals[0].cause,
            CheckCause::UnanchoredResult {
                clause: expected_kind,
                operation: expected_operation.map(str::to_owned),
            }
        );
        assert_eq!(
            refusals[0].location.path,
            Vec::<usize>::new(),
            "{:?}",
            refusals[0].location
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

    // Row 4: a second `ParentOrder` refuses ambiguous-name at both -- one
    // refusal per declaration, each at its own locus.
    let row4 = format!("{PARENT_ORDER}{PARENT_ORDER}");
    let refusals = check(&document, &row4).expect_err("row 4 refuses");
    assert!(refusals
        .iter()
        .all(|refusal| *refusal == (Code::AmbiguousDeclaration, Some("ambiguous-name"))));
    assert_eq!(refusals.len(), 2);
    let located = check_refusals(&document, &row4).expect_err("row 4 refuses");
    assert_eq!(located.len(), 2);
    assert_ne!(
        located[0].location, located[1].location,
        "each declaration of the ambiguous name is its own locus"
    );

    // Row 9: `function ParentOrder` beside the clause refuses
    // ambiguous-name at both declarations (FR-109's shared namespace),
    // again each at its own locus.
    let row9 = format!("{PARENT_ORDER}function ParentOrder using v(): Boolean pure {{ true }}\n");
    let refusals = check(&document, &row9).expect_err("row 9 refuses");
    assert!(refusals
        .iter()
        .all(|refusal| *refusal == (Code::AmbiguousDeclaration, Some("ambiguous-name"))));
    assert_eq!(refusals.len(), 2);
    let located = check_refusals(&document, &row9).expect_err("row 9 refuses");
    assert_eq!(located.len(), 2);
    assert_ne!(
        located[0].location, located[1].location,
        "the clause and the function are each their own locus"
    );
}

/// TC-460 rows 2, 3, 5-8, 10 / FR-104-AC-3, AC-4.
#[trace("TC-460", "FR-104-AC-3", "FR-104-AC-4")]
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

    // Row 3: a non-Boolean invariant body refuses `non-boolean-root`, at
    // the body itself (its whole condition is the ill-typed root, so the
    // refusal's locus is the declaration's root, an empty child-index path).
    let row3 = "invariant C using v on Config::ConfigVersion at current { self.versionNumber }\n";
    let refusals = check(&document, row3).expect_err("row 3 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0], (Code::IllTyped, Some("non-boolean-root")));
    let located = check_refusals(&document, row3).expect_err("row 3 refuses");
    assert_eq!(located.len(), 1);
    assert_eq!(
        located[0].location.path,
        Vec::<usize>::new(),
        "at the body: {:?}",
        located[0].location
    );

    // Row 5: `pre(self.versionNumber)` in an invariant refuses
    // forbidden-pre-read (`pre` is legal only in a postcondition), at the
    // `pre` itself: a proper subexpression of the `= 1` root, a non-empty
    // child-index path.
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
    let located = check_refusals(&document, row5).expect_err("row 5 refuses");
    assert_eq!(located.len(), 1);
    // `= 1`'s left child (index 0) is the `pre`.
    assert_eq!(
        located[0].location.path,
        vec![0],
        "{:?}",
        located[0].location
    );

    // Row 6: `pre(result)` in a postcondition refuses forbidden-pre-read.
    // Here the `pre` call is the whole body, so its locus is the root.
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
    let located = check_refusals(&document, row6).expect_err("row 6 refuses");
    assert_eq!(located.len(), 1);
    assert_eq!(
        located[0].location.path,
        Vec::<usize>::new(),
        "the pre is the body root here: {:?}",
        located[0].location
    );

    // Row 7: `reaches(self, self, versionNumber)` refuses
    // operator-ineligible: `versionNumber` is not reference-typed. The
    // whole body is the `reaches` call, so the refusal is at the root.
    let row7 = "invariant F using v on Config::ConfigVersion at current { \
        reaches(self, self, versionNumber) }\n";
    let refusals = check(&document, row7).expect_err("row 7 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0], (Code::IllTyped, Some("operator-ineligible")));
    let located = check_refusals(&document, row7).expect_err("row 7 refuses");
    assert_eq!(located.len(), 1);
    assert_eq!(
        located[0].location.path,
        Vec::<usize>::new(),
        "at the reaches, which is the body root here: {:?}",
        located[0].location
    );

    // Row 8: `deref(value(self.parent)).versionNumber < 5` unguarded refuses
    // unproved-presence at `value(self.parent)`, a proper subexpression of
    // the `<` root.
    let row8 = "invariant G using v on Config::ConfigVersion at current { \
        deref(value(self.parent)).versionNumber < 5 }\n";
    let refusals = check(&document, row8).expect_err("row 8 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(
        refusals[0],
        (Code::UndefinedExpression, Some("unproved-presence"))
    );
    let located = check_refusals(&document, row8).expect_err("row 8 refuses");
    assert_eq!(located.len(), 1);
    // `<`'s left child (0) is the `Attribute`, whose reference operand (0)
    // is `deref(value(self.parent))`, whose own operand (0) is
    // `value(self.parent)`.
    assert_eq!(
        located[0].location.path,
        vec![0, 0, 0],
        "{:?}",
        located[0].location
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

    // Step 1: four records -- one extent, one domain, one `DomainKey`, and
    // one occurrence key apiece (SR-737 FND-002).
    let graph = check(&document, &body).expect("the unit checks");
    assert_eq!(graph.requirements().len(), 4);
    let mut domain_keys: Vec<DomainKey> = Vec::new();
    let mut roles: Vec<(String, u64)> = Vec::new();
    for (key, record) in graph.requirements() {
        assert_eq!(
            record.requirements().kind(),
            qsl_semantics::check::Capability::OperationContract
        );
        let ClaimExtent::Unbounded(domains) = record.requirements().extent() else {
            panic!("every record names config_history's unbounded population");
        };
        assert_eq!(
            domains.len(),
            1,
            "one population domain, no other unbounded position"
        );
        let (domain_key, kind) = domains.iter().next().expect("one domain");
        assert_eq!(
            kind,
            DomainKind::Population,
            "the one domain is a population"
        );
        assert_eq!(
            kind.finite_kind(),
            Some(qsl_foundation::bound::FiniteBoundKind::Cardinality),
            "a population domain is boundable by Cardinality"
        );
        domain_keys.push(domain_key.clone());
        roles.push((
            key.origin().role().as_str().to_owned(),
            key.origin().ordinal(),
        ));
    }
    for domain_key in &domain_keys[1..] {
        assert_eq!(
            domain_key, &domain_keys[0],
            "one population (config_history), one equal DomainKey across every record"
        );
    }
    roles.sort();
    let claims = roles.iter().filter(|(role, _)| role == "claim").count();
    let generated = roles.iter().filter(|(role, _)| role == "generated").count();
    assert_eq!(
        (claims, generated),
        (3, 1),
        "three clauses' own claim occurrences, and attemptUpdate's one generated frame \
         occurrence: {roles:?}"
    );

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

    // The shared node carries two `claim` occurrences, ordinals 0
    // (`ParentOrder`, first in source order) and 1 (`ParentOrder2`).
    assert_eq!(parent_order.claim().role().as_str(), "claim");
    assert_eq!(parent_order.claim().ordinal(), 0);
    assert_eq!(parent_order_2.claim().role().as_str(), "claim");
    assert_eq!(parent_order_2.claim().ordinal(), 1);
}

/// TC-461 step 5 / FR-104 "Requirements": over a package with a second,
/// unbounded population `archive` over `ConfigVersion`, each clause refuses
/// `ambiguous_declaration`/`ambiguous-name` at its `on`, naming `archive`
/// and `config_history`. A domain package population is always unbounded
/// (`Population(None)`, no domain package population ever declares a
/// maximum -- `qsl-semantics/src/model/domain_package.rs:451-479`, QSpec
/// FR-153), so TC-461 step 5's other, bounded-`archive` half is not a real
/// case: FR-104 and TC-461 were amended to drop it rather than describe an
/// unrepresentable state (SR-737 FND-003).
#[trace("TC-461", "FR-104-AC-5")]
#[test]
fn two_no_maximum_populations_of_one_type_refuse_ambiguous_name() {
    let document = config_version_document_with_population(
        attempt_update_modifies_version_number(),
        archive_population(),
    );
    let body = format!("{PARENT_ORDER}{NO_CYCLE}{VERSION_UNCHANGED}");
    let (unit, _) = config_unit_with_body(&document, &body);
    let errors = assembly_refusals(&document, &body);
    assert_eq!(errors.len(), 3, "one refusal per clause");
    for error in &errors {
        assert_eq!(error.cause.code(), Code::AmbiguousDeclaration);
        let AssemblyCause::AmbiguousPopulation {
            context,
            populations,
        } = &error.cause
        else {
            panic!("expected AmbiguousPopulation, got {:?}", error.cause);
        };
        assert_eq!(context, "Config::ConfigVersion");
        let names: Vec<&str> = populations
            .iter()
            .map(|key| key.node.rsplit('/').next().expect("a node identity"))
            .collect();
        assert_eq!(
            names,
            vec!["archive", "config_history"],
            "the two populations, in ascending DeclarationKey order"
        );
        assert_eq!(
            &unit[error.span.start..error.span.end],
            "Config::ConfigVersion",
            "the span is the clause's `on` context"
        );
    }
}

/// SR-736 FND-001: two operations with equal frame content (both an empty
/// `modifies`/`creates`/`deletes`) still key two distinct `operation-contract`
/// frame records, one per operation, in source order -- never merged into
/// one and never an internal fault, whether their own records happen to
/// differ (probe 1: `Integer` vs `Boolean` results) or coincide (probe 2:
/// both `Boolean`).
#[trace("TC-461", "FR-104-AC-5")]
#[test]
fn two_operations_with_equal_frames_each_keep_their_own_frame_record() {
    let integer_and_boolean = config_version_document_with_operations(vec![
        operation(
            "isStable",
            json!([]),
            Some("ix://quire/native/Boolean"),
            empty_frame(),
        ),
        operation(
            "versionTotal",
            json!([]),
            Some("ix://quire/native/Integer"),
            empty_frame(),
        ),
    ]);
    let body = "post A using v on Config::ConfigVersion::isStable { result }\n\
        post B using v on Config::ConfigVersion::versionTotal { true }\n";
    let graph = check(&integer_and_boolean, body).expect("two equal-frame operations both check");
    assert_eq!(
        graph.requirements().len(),
        4,
        "two clause records and two distinct frame records"
    );

    let two_boolean = config_version_document_with_operations(vec![
        operation(
            "isStable",
            json!([]),
            Some("ix://quire/native/Boolean"),
            empty_frame(),
        ),
        operation(
            "isFresh",
            json!([]),
            Some("ix://quire/native/Boolean"),
            empty_frame(),
        ),
    ]);
    let body = "post A using v on Config::ConfigVersion::isStable { result }\n\
        post B using v on Config::ConfigVersion::isFresh { result }\n";
    let graph = check(&two_boolean, body).expect("two Boolean operations both check");
    assert_eq!(
        graph.requirements().len(),
        4,
        "content-identical frame records still stay separate, one per operation"
    );
}

/// SR-736 FND-008: the frame occurrence ordinal is minted from the
/// operations' own identity (ascending `DeclarationKey`, then operation
/// name as UTF-8 bytes), over every operation the unit's clauses name --
/// never from which clause happens to name an operation first. Checking
/// two operations with equal (empty) frames, then reversing the clauses
/// that name them, gives the very same occurrence key -> record mapping in
/// both orders: not merely the same set of keys (which reordering alone
/// could not disturb), but the same record at each key.
#[trace("TC-461", "FR-104-AC-6")]
#[test]
fn frame_occurrence_ordinal_is_independent_of_clause_order() {
    let document = config_version_document_with_operations(vec![
        operation(
            "isStable",
            json!([]),
            Some("ix://quire/native/Boolean"),
            empty_frame(),
        ),
        operation(
            "versionTotal",
            json!([]),
            Some("ix://quire/native/Integer"),
            empty_frame(),
        ),
    ]);
    let forward = "post A using v on Config::ConfigVersion::isStable { result }\n\
        post B using v on Config::ConfigVersion::versionTotal { true }\n";
    let reverse = "post B using v on Config::ConfigVersion::versionTotal { true }\n\
        post A using v on Config::ConfigVersion::isStable { result }\n";
    let forward_graph = check(&document, forward).expect("forward order checks");
    let reverse_graph = check(&document, reverse).expect("reverse order checks");

    assert_eq!(
        forward_graph.requirements(),
        reverse_graph.requirements(),
        "the same occurrence key names the same record in both clause orders"
    );

    let generated: Vec<u64> = forward_graph
        .requirements()
        .keys()
        .filter(|key| key.origin().role().as_str() == "generated")
        .map(|key| key.origin().ordinal())
        .collect();
    assert_eq!(
        generated.len(),
        2,
        "one frame occurrence per operation, role `generated` (SR-736 FND-009)"
    );
    let mut ordinals = generated;
    ordinals.sort_unstable();
    assert_eq!(
        ordinals,
        vec![0, 1],
        "the two operations' frame ordinals are 0 and 1, whichever clause names them first"
    );
}

/// SR-736 FND-002: a clause over a subtype still names its supertype's
/// unbounded population, by conformance (FR-084's `allInstances<T>`), not
/// exact identity; a clause over a type no population covers gets no
/// population domain at all -- its extent is `Bounded` when nothing else
/// contributes an unbounded domain.
#[trace("TC-461", "FR-104-AC-5")]
#[test]
fn population_coverage_is_by_conformance_and_absent_when_none_covers() {
    let subtype = subtype_document(attempt_update_modifies_version_number());
    let body = "invariant SubInvariant using v on Config::Sub at current { true }\n";
    let graph = check(&subtype, body).expect("Sub's invariant checks");
    let record = graph
        .requirements()
        .values()
        .next()
        .expect("one requirement record");
    assert_eq!(
        record.requirements().extent().to_wire(),
        "unbounded",
        "Sub's clause still names config_history's population, by conformance"
    );

    let uncovered = ambiguous_operation_document();
    let body = "invariant LeftInvariant using v on Config::Left at current { true }\n";
    let graph = check(&uncovered, body).expect("Left's invariant checks");
    let record = graph
        .requirements()
        .values()
        .next()
        .expect("one requirement record");
    assert_eq!(
        record.requirements().extent().to_wire(),
        "bounded",
        "no population covers Left, so the clause names no population domain"
    );
}

/// SR-736 FND-010: a population's `DomainKey` names the object-type node of
/// the population's own declared member type that covers the clause's
/// context, never the clause's own (possibly proper-subtype) context type.
/// `config_history` declares exactly one member, `ConfigVersion`; an
/// invariant over `Sub` (a proper subtype, covered only by conformance) and
/// one over `ConfigVersion` itself therefore key `config_history`'s domain
/// identically.
#[trace("TC-461", "FR-104-AC-5")]
#[test]
fn population_domain_key_is_the_populations_own_member_type() {
    let subtype = subtype_document(attempt_update_modifies_version_number());
    let body = "invariant SubInvariant using v on Config::Sub at current { true }\n\
        invariant ConfigVersionInvariant using v on Config::ConfigVersion at current { true }\n";
    let graph = check(&subtype, body).expect("both invariants check");
    assert_eq!(
        graph.requirements().len(),
        2,
        "one record per clause, neither names an operation"
    );

    let mut domain_keys: Vec<DomainKey> = Vec::new();
    for record in graph.requirements().values() {
        let ClaimExtent::Unbounded(domains) = record.requirements().extent() else {
            panic!("both clauses name config_history's unbounded population");
        };
        assert_eq!(
            domains.len(),
            1,
            "config_history's population is the clause's only unbounded domain"
        );
        let (key, kind) = domains.iter().next().expect("one domain");
        assert_eq!(kind, DomainKind::Population);
        domain_keys.push(key.clone());
    }
    assert_eq!(
        domain_keys[0], domain_keys[1],
        "Sub's and ConfigVersion's clauses key config_history's population identically: \
         the DomainKey names config_history's own declared member type (ConfigVersion), \
         never Sub itself"
    );
}

/// SR-723 FND-007 / SR-736 FND-006: an operation a supertype declares is
/// visible on a subtype's effective view, keeping the declaring type
/// (`declaring != context`); two unrelated ancestors that each declare an
/// operation of the same name, neither more derived than the other, are
/// ambiguous.
#[trace("TC-458", "FR-103-AC-1")]
#[test]
fn operation_visibility_on_subtypes_inherits_or_refuses_ambiguous() {
    let subtype = subtype_document(attempt_update_modifies_version_number());
    let body = "post P using v on Config::Sub::attemptUpdate { result }\n";
    let graph = check(&subtype, body).expect("Sub inherits attemptUpdate");
    let clause = graph.state_clause("P").expect("P checked");
    let operation = clause.operation().expect("P names an operation");
    assert_ne!(
        clause.context(),
        operation.declaring,
        "attemptUpdate's declaring type (ConfigVersion) is not Sub itself"
    );

    let ambiguous = ambiguous_operation_document();
    let body = "post Q using v on Config::Both::dup { result }\n";
    let refusals = check(&ambiguous, body).expect_err("Both::dup is ambiguous");
    assert_eq!(refusals.len(), 1);
    assert_eq!(
        refusals[0],
        (Code::AmbiguousDeclaration, Some("ambiguous-name"))
    );
}
