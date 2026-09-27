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

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_forms::StateClauseKind;
use qsl_foundation::bound::DomainKey;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    AssemblyCause, AssemblyError, CheckCause, CheckRefusal, CheckedGraph, CheckingLimits, Location,
    NodeKind, Observation, Origin, WrongSnapshotCause,
};
use qsl_semantics::family::{ClaimExtent, DomainKind};
use quire_exact::ValueType;

use crate::model_operations::{
    admit_and_assemble_with_body, ambiguous_operation_document, archive_population,
    config_unit_with_body, config_version_document, config_version_document_with_operations,
    config_version_document_with_population, config_version_identity, empty_frame, operation,
    operation_parameter, subtype_document, subtype_document_with_two_member_population,
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
    assert_eq!(
        located[0].location,
        Location {
            origin: Origin::StateClause {
                clause: "ParentOrder".to_owned(),
                index: 0,
            },
            path: Vec::new(),
        },
        "the first ParentOrder is its own locus"
    );
    assert_eq!(
        located[1].location,
        Location {
            origin: Origin::StateClause {
                clause: "ParentOrder".to_owned(),
                index: 1,
            },
            path: Vec::new(),
        },
        "the second ParentOrder is its own locus"
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
    assert_eq!(
        located[0].location,
        Location {
            origin: Origin::Body {
                function: "ParentOrder".to_owned(),
                index: 0,
            },
            path: Vec::new(),
        },
        "the function's own declaration is its locus"
    );
    assert_eq!(
        located[1].location,
        Location {
            origin: Origin::StateClause {
                clause: "ParentOrder".to_owned(),
                index: 0,
            },
            path: Vec::new(),
        },
        "the clause's own declaration is its locus"
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

    // Row 2: `on Config::ConfigVersion::missing` refuses missing-name, at
    // the operation name span (SR-737 FND-004 leftover).
    let row2 = "pre B using v on Config::ConfigVersion::missing { true }\n";
    let refusals = check(&document, row2).expect_err("row 2 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(
        refusals[0],
        (Code::MissingDeclaration, Some("missing-name"))
    );
    let (unit, _) = config_unit_with_body(&document, row2);
    let errors = assembly_refusals(&document, row2);
    assert_eq!(errors.len(), 1);
    let AssemblyCause::UnresolvedOperation { context, operation } = &errors[0].cause else {
        panic!("expected UnresolvedOperation, got {:?}", errors[0].cause);
    };
    assert_eq!(context, "Config::ConfigVersion");
    assert_eq!(operation, "missing");
    assert_eq!(
        &unit[errors[0].span.start..errors[0].span.end],
        "missing",
        "the assembler's span is the operation name as written, not the whole `on` clause"
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

    // Row 10: `reaches` in a function body refuses operator-ineligible, at
    // the root: the whole body is the `reaches` call (SR-737 FND-004
    // leftover).
    let row10 = "function r using v(x: Config::ConfigVersion, y: Config::ConfigVersion): \
        Boolean pure { reaches(x, y, parent) }\n";
    let refusals = check(&document, row10).expect_err("row 10 refuses");
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0], (Code::IllTyped, Some("operator-ineligible")));
    let located = check_refusals(&document, row10).expect_err("row 10 refuses");
    assert_eq!(located.len(), 1);
    assert_eq!(
        located[0].location,
        Location {
            origin: Origin::Body {
                function: "r".to_owned(),
                index: 0,
            },
            path: Vec::new(),
        },
        "the reaches call is the whole function body"
    );
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

/// SR-736 FND-011: a population that declares two or more member types
/// still has exactly one `DomainKey` -- its canonical member (the least of
/// its declared member types in ascending `DeclarationKey` order), whatever
/// member type a clause's context conforms to. `config_history` here
/// declares both `Sub` and `ConfigVersion` as its own members (not `Sub` by
/// conformance alone, as [`population_domain_key_is_the_populations_own_member_type`]
/// covers): a clause on each still gives one equal key.
#[trace("TC-461", "FR-104-AC-5")]
#[test]
fn population_with_several_members_has_one_canonical_domain_key() {
    let document =
        subtype_document_with_two_member_population(attempt_update_modifies_version_number());
    let body = "invariant SubInvariant using v on Config::Sub at current { true }\n\
        invariant ConfigVersionInvariant using v on Config::ConfigVersion at current { true }\n";
    let graph = check(&document, body).expect("both invariants check");
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
        assert_eq!(domains.len(), 1, "one population domain per clause");
        let (key, kind) = domains.iter().next().expect("one domain");
        assert_eq!(kind, DomainKind::Population);
        domain_keys.push(key.clone());
    }
    assert_eq!(
        domain_keys[0], domain_keys[1],
        "config_history declares two member types (Sub, ConfigVersion); a clause on \
         either still keys its one domain by the same canonical member"
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

// ---------------------------------------------------------------------------
// FR-106 check 11: the frame and delta check, end to end over a precondition
// and a postcondition on `attemptUpdate` (QSL-278's own gap: check 11 was
// previously exercised only by `frame.rs`'s own unit tests, never through
// `admit_observations`'s `Invocation` path). `attemptUpdate`'s frame is
// `modifies: [versionNumber]` ([`attempt_update_modifies_version_number`]),
// so `parent` is a field outside `modifies` (check 11.3's own fixture).
// ---------------------------------------------------------------------------

fn frame_test_document() -> Vec<u8> {
    config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    )
}

const FRAME_CLAUSES: &str = "pre AttemptUpdatePre using v on Config::ConfigVersion::attemptUpdate \
    { true }\npost AttemptUpdatePost using v on Config::ConfigVersion::attemptUpdate { true }\n";

fn frame_document_digest(bytes: &[u8]) -> [u8; 32] {
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("test fixture is JSON");
    let limits = quire_canonical::Limits::new(u64::MAX, quire_canonical::Limits::MAX_DEPTH)
        .expect("MAX_DEPTH is within MAX_DEPTH");
    *quire_canonical::sha256(&value, limits)
        .expect("test fixture is RFC 8785 canonical")
        .as_bytes()
}

fn frame_label(identity: &str) -> qsl_semantics::model::observation::DocumentRef {
    qsl_semantics::model::observation::DocumentRef {
        authority: "test".to_owned(),
        identity: identity.to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    }
}

fn frame_model_header(model_digest_hex: &str) -> serde_json::Value {
    json!({
        "identity": "example/config-version",
        "version": "1.0.0",
        "digest": format!("sha256-jcs:{model_digest_hex}"),
    })
}

fn frame_identity_json(
    label: &qsl_semantics::model::observation::DocumentRef,
) -> serde_json::Value {
    json!({
        "authority": label.authority,
        "identity": label.identity,
        "revision_namespace": label.revision_namespace,
        "revision": label.revision,
    })
}

/// A `config_history` snapshot: `objects`, each `(key, version_number,
/// parent_key)` (`parent_key` `None` means `parent` is absent).
fn frame_snapshot_bytes(
    label: &qsl_semantics::model::observation::DocumentRef,
    model_digest_hex: &str,
    observation: &str,
    objects: &[(&str, i64, Option<&str>)],
) -> Vec<u8> {
    let config_version = "ix://example/config-version/ConfigVersion";
    let population = "ix://example/config-version/config_history";
    let objects: Vec<_> = objects
        .iter()
        .map(|(key, version_number, parent)| {
            let parent_field = match parent {
                Some(parent_key) => {
                    json!({"present": {"reference": {"population": population, "key": parent_key}}})
                }
                None => json!({"absent": null}),
            };
            json!({
                "key": key,
                "type": config_version,
                "fields": {
                    "versionNumber": {"integer": version_number.to_string()},
                    "parent": parent_field,
                },
            })
        })
        .collect();
    let value = json!({
        "format": "quire.state.snapshot/v1",
        "identity": frame_identity_json(label),
        "observation": observation,
        "model": frame_model_header(model_digest_hex),
        "populations": [
            {"population": population, "complete": true, "objects": objects},
        ],
    });
    value.to_string().into_bytes()
}

#[allow(clippy::too_many_arguments)]
fn frame_invocation_bytes(
    label: &qsl_semantics::model::observation::DocumentRef,
    model_digest_hex: &str,
    pre_ref: &qsl_semantics::model::observation::DocumentRef,
    post_ref: &qsl_semantics::model::observation::DocumentRef,
    self_key: &str,
    created: &[&str],
    deleted: &[&str],
) -> Vec<u8> {
    let population = "ix://example/config-version/config_history";
    let object_ref = |key: &str| json!({"population": population, "key": key});
    let value = json!({
        "format": "quire.state.invocation/v1",
        "identity": frame_identity_json(label),
        "model": frame_model_header(model_digest_hex),
        "context": "ix://example/config-version/ConfigVersion",
        "operation": "attemptUpdate",
        "self": {"population": population, "key": self_key},
        "pre": {
            "identity": frame_identity_json(pre_ref),
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&pre_ref.digest)),
        },
        "post": {
            "identity": frame_identity_json(post_ref),
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&post_ref.digest)),
        },
        "parameters": {},
        "result": {"boolean": true},
        "created": created.iter().map(|key| object_ref(key)).collect::<Vec<_>>(),
        "deleted": deleted.iter().map(|key| object_ref(key)).collect::<Vec<_>>(),
    });
    value.to_string().into_bytes()
}

/// Runs `clause_name` (`AttemptUpdatePre`/`AttemptUpdatePost`) over an
/// invocation built from `pre_objects`/`post_objects` ((key, versionNumber,
/// parent) triples), self object `self_key`, declared `created`/`deleted`.
#[allow(clippy::too_many_arguments)]
fn run_frame_clause(
    clause_name: &str,
    pre_objects: &[(&str, i64, Option<&str>)],
    post_objects: &[(&str, i64, Option<&str>)],
    self_key: &str,
    created: &[&str],
    deleted: &[&str],
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    let document = frame_test_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let model_digest = qsl_semantics::model::key::hex(digest);

    let declarations = admit_and_assemble_with_body(&document, FRAME_CLAUSES)
        .unwrap_or_else(|refusal| panic!("assembly refused: {refusal:?}"));
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("check refused: {refusals:?}"));
    let clause = graph.state_clause(clause_name).expect("clause is declared");

    let pre = frame_label("pre-snap");
    let pre_bytes = frame_snapshot_bytes(&pre, &model_digest, "pre", pre_objects);
    let pre = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&pre_bytes),
        ..pre
    };
    let post = frame_label("post-snap");
    let post_bytes = frame_snapshot_bytes(&post, &model_digest, "post", post_objects);
    let post = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&post_bytes),
        ..post
    };
    let invocation = frame_label("invocation");
    let invocation_bytes = frame_invocation_bytes(
        &invocation,
        &model_digest,
        &pre,
        &post,
        self_key,
        created,
        deleted,
    );
    let invocation = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&invocation_bytes),
        ..invocation
    };

    let mut snapshots = BTreeMap::new();
    snapshots.insert(pre.digest, pre_bytes);
    snapshots.insert(post.digest, post_bytes);
    let mut invocations = BTreeMap::new();
    invocations.insert(invocation.digest, invocation_bytes);
    let provisions = qsl_semantics::model::observation::Provisions {
        snapshots: &snapshots,
        invocations: &invocations,
    };
    let selection = qsl_semantics::model::observation::ClauseSelection {
        name: clause_name.to_owned(),
        input: qsl_semantics::model::observation::ClauseSelectionInput::Invocation { invocation },
    };
    // The model -> check edge must stay empty (FR-074-AC-3): the caller
    // (here, this test; in production, `qsl-replay/src/spine/clause.rs`)
    // reads the checked clause's own facts into `ClauseFacts`/
    // `OperationFacts`, `admit_observations`'s own model-level input.
    let clause_facts = qsl_semantics::model::observation::ClauseFacts {
        identity: clause.identity(),
        kind: clause.kind(),
        context: clause.context(),
        operation: clause.operation().map(|operation| {
            qsl_semantics::model::observation::OperationFacts {
                declaring: operation.declaring,
                declaration: operation.declaration.clone(),
            }
        }),
    };
    qsl_semantics::model::observation::admit_observations(
        graph.model_selections(),
        graph.scope().types(),
        &clause_facts,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &provisions,
        &selection,
        qsl_semantics::model::observation::ObservationLimits::default(),
    )
}

fn assert_frame_refused(
    result: Result<
        qsl_semantics::model::observation::AdmittedObservations,
        qsl_semantics::model::observation::AdmissionFailure,
    >,
    expected_code: &str,
    expected_cause: &str,
    expected_object: Option<&str>,
) {
    match result {
        Err(qsl_semantics::model::observation::AdmissionFailure::Refused(record)) => {
            assert_eq!(record.code, expected_code);
            assert_eq!(record.cause, expected_cause);
            if let Some(expected_object) = expected_object {
                assert_eq!(
                    record.fields.get("object").map(String::as_str),
                    Some(expected_object)
                );
            }
        }
        other => panic!("expected Err(Refused({expected_code}/{expected_cause})), got {other:?}"),
    }
}

/// TC-464 (FR-106 check 11, precondition): `root`'s `versionNumber` changes
/// from 1 to 2 -- authorized (`attemptUpdate`'s frame `modifies
/// [versionNumber]`) -- no other defect; admission admits.
#[trace("TC-464", "FR-106-AC-5")]
#[test]
fn attempt_update_precondition_admits_an_authorized_change() {
    let pre = [("root", 1, None)];
    let post = [("root", 2, None)];
    let observations = run_frame_clause("AttemptUpdatePre", &pre, &post, "root", &[], &[])
        .expect("an authorized versionNumber change admits");
    assert!(observations.pre.is_some());
    assert!(observations.post.is_some());
}

/// TC-464 (FR-106 check 11, postcondition): the same authorized change,
/// admitted for `AttemptUpdatePost` over the same pre/post snapshots.
#[trace("TC-464", "FR-106-AC-5")]
#[test]
fn attempt_update_postcondition_admits_an_authorized_change() {
    let pre = [("root", 1, None)];
    let post = [("root", 2, None)];
    let observations = run_frame_clause("AttemptUpdatePost", &pre, &post, "root", &[], &[])
        .expect("an authorized versionNumber change admits");
    assert!(observations.pre.is_some());
    assert!(observations.post.is_some());
}

/// FR-106 check 11.1: the post snapshot's `child`, absent from pre, is not
/// granted by `attemptUpdate`'s (empty) `creates` -- refuses naming `child`.
#[trace("TC-464", "FR-106-AC-5")]
#[test]
fn attempt_update_refuses_an_unauthorized_create() {
    let pre = [("root", 1, None)];
    let post = [("root", 1, None), ("child", 1, None)];
    let result = run_frame_clause("AttemptUpdatePre", &pre, &post, "root", &["child"], &[]);
    assert_frame_refused(
        result,
        "frame_violation",
        "unauthorized-change",
        Some("child"),
    );
}

/// FR-106 check 11.2: the pre snapshot's `child`, absent from post, is not
/// granted by `attemptUpdate`'s (empty) `deletes` -- refuses naming `child`.
#[trace("TC-464", "FR-106-AC-5")]
#[test]
fn attempt_update_refuses_an_unauthorized_delete() {
    let pre = [("root", 1, None), ("child", 1, Some("root"))];
    let post = [("root", 1, None)];
    let result = run_frame_clause("AttemptUpdatePre", &pre, &post, "root", &[], &["child"]);
    assert_frame_refused(
        result,
        "frame_violation",
        "unauthorized-change",
        Some("child"),
    );
}

/// FR-106 check 11.3: `root`'s `parent` changes, a field outside
/// `attemptUpdate`'s `modifies [versionNumber]` -- refuses naming `root`.
#[trace("TC-464", "FR-106-AC-5")]
#[test]
fn attempt_update_refuses_a_change_outside_modifies() {
    let pre = [("root", 1, None), ("child", 1, None)];
    let post = [("root", 1, Some("child")), ("child", 1, None)];
    let result = run_frame_clause("AttemptUpdatePre", &pre, &post, "root", &[], &[]);
    assert_frame_refused(
        result,
        "frame_violation",
        "unauthorized-change",
        Some("root"),
    );
}

/// FR-106 check 11.4: nothing is actually created between pre and post, but
/// the invocation declares `created: [child]` -- refuses
/// `population_delta_mismatch`/`delta-disagreement`.
#[trace("TC-464", "FR-106-AC-5")]
#[test]
fn attempt_update_refuses_a_declared_delta_mismatch() {
    let pre = [("root", 1, None)];
    let post = [("root", 1, None)];
    let result = run_frame_clause("AttemptUpdatePre", &pre, &post, "root", &["child"], &[]);
    assert_frame_refused(
        result,
        "population_delta_mismatch",
        "delta-disagreement",
        None,
    );
}

// ---------------------------------------------------------------------------
// TC-465 (FR-106-AC-3, AC-4, AC-5, AC-7; SR-751 FND-001): admission refuses
// each input defect. Shared package: `ConfigVersion` (`versionNumber`,
// `parent`), `attemptUpdate` (modifies `[versionNumber, parent]` unless a
// row's own package variant says otherwise) and
// `probe(target: ConfigVersion)` (no result, empty frame), one closed
// population `config_history`; `ParentOrder` (invariant) and
// `VersionUnchanged` (`attemptUpdate`'s postcondition) are its two clauses.
// ---------------------------------------------------------------------------

const TC465_CLAUSES: &str = "invariant ParentOrder using v on Config::ConfigVersion at current { \
    present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }\n\
    post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate { \
    self.versionNumber = pre(self.versionNumber) }\n";

fn tc465_document_with(operation: serde_json::Value) -> Vec<u8> {
    config_version_document_with_operations(vec![operation, probe_operation()])
}

fn tc465_document() -> Vec<u8> {
    tc465_document_with(attempt_update_modifies_version_and_parent())
}

fn tc465_model_digest_hex(document: &[u8]) -> String {
    let packages = qsl_semantics::model::intake::package_input([document]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    qsl_semantics::model::key::hex(digest)
}

/// The healthy-parent current snapshot (FR-106's own worked example):
/// `root` at 1 with no parent, `child` at 2 naming `root`.
fn tc465_healthy_parent(
    label: &qsl_semantics::model::observation::DocumentRef,
    model_digest_hex: &str,
) -> serde_json::Value {
    let population = "ix://example/config-version/config_history";
    let config_version = "ix://example/config-version/ConfigVersion";
    json!({
        "format": "quire.state.snapshot/v1",
        "identity": frame_identity_json(label),
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": frame_model_header(model_digest_hex),
        "populations": [{
            "population": population,
            "complete": true,
            "objects": [
                {"key": "root", "type": config_version,
                 "fields": {"versionNumber": {"integer": "1"}, "parent": {"absent": null}}},
                {"key": "child", "type": config_version,
                 "fields": {"versionNumber": {"integer": "2"},
                            "parent": {"present": {"reference": {"population": population, "key": "root"}}}}},
            ],
        }],
    })
}

/// The changed-version invocation's pre snapshot: `child` at 2, no parent.
fn tc465_pre_snapshot(
    label: &qsl_semantics::model::observation::DocumentRef,
    model_digest_hex: &str,
) -> serde_json::Value {
    let population = "ix://example/config-version/config_history";
    let config_version = "ix://example/config-version/ConfigVersion";
    json!({
        "format": "quire.state.snapshot/v1",
        "identity": frame_identity_json(label),
        "observation": "pre",
        "model": frame_model_header(model_digest_hex),
        "populations": [{
            "population": population,
            "complete": true,
            "objects": [
                {"key": "child", "type": config_version,
                 "fields": {"versionNumber": {"integer": "2"}, "parent": {"absent": null}}},
            ],
        }],
    })
}

/// The changed-version invocation's post snapshot: `child` at 3, no parent.
fn tc465_post_snapshot(
    label: &qsl_semantics::model::observation::DocumentRef,
    model_digest_hex: &str,
) -> serde_json::Value {
    let population = "ix://example/config-version/config_history";
    let config_version = "ix://example/config-version/ConfigVersion";
    json!({
        "format": "quire.state.snapshot/v1",
        "identity": frame_identity_json(label),
        "observation": "post",
        "model": frame_model_header(model_digest_hex),
        "populations": [{
            "population": population,
            "complete": true,
            "objects": [
                {"key": "child", "type": config_version,
                 "fields": {"versionNumber": {"integer": "3"}, "parent": {"absent": null}}},
            ],
        }],
    })
}

fn tc465_invocation(
    label: &qsl_semantics::model::observation::DocumentRef,
    model_digest_hex: &str,
    pre_ref: &qsl_semantics::model::observation::DocumentRef,
    post_ref: &qsl_semantics::model::observation::DocumentRef,
) -> serde_json::Value {
    let population = "ix://example/config-version/config_history";
    json!({
        "format": "quire.state.invocation/v1",
        "identity": frame_identity_json(label),
        "model": frame_model_header(model_digest_hex),
        "context": "ix://example/config-version/ConfigVersion",
        "operation": "attemptUpdate",
        "self": {"population": population, "key": "child"},
        "pre": {
            "identity": frame_identity_json(pre_ref),
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&pre_ref.digest)),
        },
        "post": {
            "identity": frame_identity_json(post_ref),
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&post_ref.digest)),
        },
        "parameters": {},
        "result": {"boolean": true},
        "created": [],
        "deleted": [],
    })
}

/// Runs `clause_name` over `document`, admitting `selection` under
/// `snapshots`/`invocations` -- TC-465's own harness.
fn run_tc465(
    document: &[u8],
    clause_name: &str,
    selection: qsl_semantics::model::observation::ClauseSelectionInput,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    invocations: BTreeMap<[u8; 32], Vec<u8>>,
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    let packages = qsl_semantics::model::intake::package_input([document]);
    let declarations = admit_and_assemble_with_body(document, TC465_CLAUSES)
        .unwrap_or_else(|refusal| panic!("assembly refused: {refusal:?}"));
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("check refused: {refusals:?}"));
    let clause = graph.state_clause(clause_name).expect("clause is declared");
    let provisions = qsl_semantics::model::observation::Provisions {
        snapshots: &snapshots,
        invocations: &invocations,
    };
    let clause_facts = qsl_semantics::model::observation::ClauseFacts {
        identity: clause.identity(),
        kind: clause.kind(),
        context: clause.context(),
        operation: clause.operation().map(|operation| {
            qsl_semantics::model::observation::OperationFacts {
                declaring: operation.declaring,
                declaration: operation.declaration.clone(),
            }
        }),
    };
    let selection = qsl_semantics::model::observation::ClauseSelection {
        name: clause_name.to_owned(),
        input: selection,
    };
    qsl_semantics::model::observation::admit_observations(
        graph.model_selections(),
        graph.scope().types(),
        &clause_facts,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &provisions,
        &selection,
        qsl_semantics::model::observation::ObservationLimits::default(),
    )
}

/// Runs `ParentOrder` (invariant, `Current`) over `document`, with the
/// healthy-parent snapshot's JSON `serde_json::Value` mutated by `mutate`
/// before it is serialized and digested (`selected_digest`, when given,
/// overrides the document's own digest for the `ClauseSelection` -- a
/// stale-selection or edited-under-original-digest row).
fn run_tc465_current(
    document: &[u8],
    mutate: impl FnOnce(&mut serde_json::Value),
    selected_digest: Option<[u8; 32]>,
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    let model_digest_hex = tc465_model_digest_hex(document);
    let label = frame_label("current-snap");
    let mut value = tc465_healthy_parent(&label, &model_digest_hex);
    mutate(&mut value);
    let bytes = value.to_string().into_bytes();
    let real_digest = frame_document_digest(&bytes);
    let selected = qsl_semantics::model::observation::DocumentRef {
        digest: selected_digest.unwrap_or(real_digest),
        ..label
    };
    let mut snapshots = BTreeMap::new();
    snapshots.insert(selected.digest, bytes);
    run_tc465(
        document,
        "ParentOrder",
        qsl_semantics::model::observation::ClauseSelectionInput::Current {
            snapshot: selected,
            anchor: qsl_semantics::model::observation::SelectedAnchor {
                kind: qsl_semantics::model::observation::AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: qsl_semantics::model::observation::SelectedObject {
                population: "ix://example/config-version/config_history".to_owned(),
                key: "child".to_owned(),
            },
        },
        snapshots,
        BTreeMap::new(),
    )
}

/// Runs `VersionUnchanged` (postcondition, `Invocation`) over `document`,
/// with the changed-version invocation's own JSON mutated by
/// `mutate_invocation`, and its pre/post snapshots by `mutate_pre`/
/// `mutate_post`.
fn run_tc465_invocation(
    document: &[u8],
    mutate_pre: impl FnOnce(&mut serde_json::Value),
    mutate_post: impl FnOnce(&mut serde_json::Value),
    mutate_invocation: impl FnOnce(&mut serde_json::Value),
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    let model_digest_hex = tc465_model_digest_hex(document);
    let pre_label = frame_label("pre-snap");
    let mut pre_value = tc465_pre_snapshot(&pre_label, &model_digest_hex);
    mutate_pre(&mut pre_value);
    let pre_bytes = pre_value.to_string().into_bytes();
    let pre = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&pre_bytes),
        ..pre_label
    };

    let post_label = frame_label("post-snap");
    let mut post_value = tc465_post_snapshot(&post_label, &model_digest_hex);
    mutate_post(&mut post_value);
    let post_bytes = post_value.to_string().into_bytes();
    let post = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&post_bytes),
        ..post_label
    };

    let invocation_label = frame_label("invocation");
    let mut invocation_value = tc465_invocation(&invocation_label, &model_digest_hex, &pre, &post);
    mutate_invocation(&mut invocation_value);
    let invocation_bytes = invocation_value.to_string().into_bytes();
    let invocation = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&invocation_bytes),
        ..invocation_label
    };

    let mut snapshots = BTreeMap::new();
    snapshots.insert(pre.digest, pre_bytes);
    snapshots.insert(post.digest, post_bytes);
    let mut invocations = BTreeMap::new();
    invocations.insert(invocation.digest, invocation_bytes);
    run_tc465(
        document,
        "VersionUnchanged",
        qsl_semantics::model::observation::ClauseSelectionInput::Invocation { invocation },
        snapshots,
        invocations,
    )
}

fn assert_tc465_refused(
    result: Result<
        qsl_semantics::model::observation::AdmittedObservations,
        qsl_semantics::model::observation::AdmissionFailure,
    >,
    expected_code: &str,
    expected_cause: &str,
) -> qsl_semantics::model::observation::AdmissionRecord {
    match result {
        Err(qsl_semantics::model::observation::AdmissionFailure::Refused(record)) => {
            assert_eq!(record.code, expected_code);
            assert_eq!(record.cause, expected_cause);
            record
        }
        other => panic!("expected Err(Refused({expected_code}/{expected_cause})), got {other:?}"),
    }
}

fn assert_tc465_incomplete(
    result: Result<
        qsl_semantics::model::observation::AdmittedObservations,
        qsl_semantics::model::observation::AdmissionFailure,
    >,
    expected_code: &str,
    expected_cause: &str,
) -> qsl_semantics::model::observation::AdmissionRecord {
    match result {
        Err(qsl_semantics::model::observation::AdmissionFailure::Incomplete(record)) => {
            assert_eq!(record.code, expected_code);
            assert_eq!(record.cause, expected_cause);
            record
        }
        other => {
            panic!("expected Err(Incomplete({expected_code}/{expected_cause})), got {other:?}")
        }
    }
}

/// Row 1 (check 1.1): the selected snapshot is missing from the provision.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row1_missing_document_is_incomplete() {
    let document = tc465_document();
    let model_digest_hex = tc465_model_digest_hex(&document);
    let label = frame_label("current-snap");
    let selected = qsl_semantics::model::observation::DocumentRef {
        digest: [9; 32],
        ..label
    };
    let _ = tc465_healthy_parent(&selected, &model_digest_hex); // unused; provision stays empty
    let result = run_tc465(
        &document,
        "ParentOrder",
        qsl_semantics::model::observation::ClauseSelectionInput::Current {
            snapshot: selected,
            anchor: qsl_semantics::model::observation::SelectedAnchor {
                kind: qsl_semantics::model::observation::AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: qsl_semantics::model::observation::SelectedObject {
                population: "ix://example/config-version/config_history".to_owned(),
                key: "child".to_owned(),
            },
        },
        BTreeMap::new(),
        BTreeMap::new(),
    );
    assert_tc465_incomplete(
        result,
        "unavailable_observation",
        "missing-required-artifact",
    );
}

/// Row 2 (check 1.2): a snapshot of 1 MiB + 1 byte.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row2_oversized_document_refuses_input_bytes_exceeded() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["padding"] = json!("x".repeat(1_048_576));
        },
        None,
    );
    assert_tc465_refused(result, "stage_limit_exceeded", "input-bytes-exceeded");
}

/// Row 3 (check 1.2): a field value nested 65 `present` levels deep.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row3_deeply_nested_value_refuses_nesting_depth_exceeded() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            let mut nested = json!({"absent": null});
            for _ in 0..65 {
                nested = json!({"present": nested});
            }
            value["populations"][0]["objects"][1]["fields"]["parent"] = nested;
        },
        None,
    );
    assert_tc465_refused(result, "stage_limit_exceeded", "nesting-depth-exceeded");
}

/// Row 4 (check 1.3): the snapshot's bytes edited, kept under the original
/// digest.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row4_edited_bytes_under_original_digest_refuses_byte_digest_mismatch() {
    let document = tc465_document();
    let model_digest_hex = tc465_model_digest_hex(&document);
    let label = frame_label("current-snap");
    let original = tc465_healthy_parent(&label, &model_digest_hex);
    let original_digest = frame_document_digest(&original.to_string().into_bytes());
    let mut edited = original;
    edited["populations"][0]["objects"][1]["fields"]["versionNumber"] = json!({"integer": "3"});
    let result = run_tc465_current(
        &document,
        |value| *value = edited.clone(),
        Some(original_digest),
    );
    assert_tc465_refused(result, "stale_dependency", "byte-digest-mismatch");
}

/// Row 5 (check 1.4): `format` `native-state-input/1`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row5_wrong_format_refuses_unsupported_wire() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| value["format"] = json!("native-state-input/1"),
        None,
    );
    assert_tc465_refused(result, "unknown_wire", "unsupported-wire");
}

/// Row 6 (check 1.5): `populations` removed.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row6_missing_populations_refuses_missing_member() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value.as_object_mut().unwrap().remove("populations");
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "missing-member");
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("populations")
    );
}

/// Row 7 (check 1.6): an extra top-level member `note`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row7_unknown_member_refuses_unknown_member() {
    let document = tc465_document();
    let result = run_tc465_current(&document, |value| value["note"] = json!(true), None);
    let record = assert_tc465_refused(result, "invalid_runtime_input", "unknown-member");
    assert_eq!(record.fields.get("field").map(String::as_str), Some("note"));
}

/// Row 8 (check 1.7): blank `authority` in document and selection.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row8_blank_authority_refuses_blank_label() {
    let document = tc465_document();
    let model_digest_hex = tc465_model_digest_hex(&document);
    let label = frame_label("current-snap");
    let mut blank = tc465_healthy_parent(&label, &model_digest_hex);
    blank["identity"]["authority"] = json!("");
    let bytes = blank.to_string().into_bytes();
    let digest = frame_document_digest(&bytes);
    let selected = qsl_semantics::model::observation::DocumentRef {
        authority: String::new(),
        digest,
        ..label
    };
    let mut snapshots = BTreeMap::new();
    snapshots.insert(digest, bytes);
    let result = run_tc465(
        &document,
        "ParentOrder",
        qsl_semantics::model::observation::ClauseSelectionInput::Current {
            snapshot: selected,
            anchor: qsl_semantics::model::observation::SelectedAnchor {
                kind: qsl_semantics::model::observation::AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: qsl_semantics::model::observation::SelectedObject {
                population: "ix://example/config-version/config_history".to_owned(),
                key: "child".to_owned(),
            },
        },
        snapshots,
        BTreeMap::new(),
    );
    let record = assert_tc465_refused(result, "invalid_source_identity", "blank-label");
    assert_eq!(
        record.fields.get("label").map(String::as_str),
        Some("authority")
    );
}

/// Row 9 (check 1.8): the selection's `revision` differs from the
/// document's own.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row9_revision_mismatch_refuses_revision_mismatch() {
    let document = tc465_document();
    let model_digest_hex = tc465_model_digest_hex(&document);
    let label = frame_label("current-snap");
    let value = tc465_healthy_parent(&label, &model_digest_hex);
    let bytes = value.to_string().into_bytes();
    let digest = frame_document_digest(&bytes);
    let selected = qsl_semantics::model::observation::DocumentRef {
        revision: "2".to_owned(),
        digest,
        ..label
    };
    let mut snapshots = BTreeMap::new();
    snapshots.insert(digest, bytes);
    let result = run_tc465(
        &document,
        "ParentOrder",
        qsl_semantics::model::observation::ClauseSelectionInput::Current {
            snapshot: selected,
            anchor: qsl_semantics::model::observation::SelectedAnchor {
                kind: qsl_semantics::model::observation::AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: qsl_semantics::model::observation::SelectedObject {
                population: "ix://example/config-version/config_history".to_owned(),
                key: "child".to_owned(),
            },
        },
        snapshots,
        BTreeMap::new(),
    );
    assert_tc465_refused(result, "stale_dependency", "revision-mismatch");
}

/// Row 10 (check 2): `Current` selecting `VersionUnchanged` (a
/// postcondition).
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row10_current_selecting_a_postcondition_refuses_wrong_observation() {
    let document = tc465_document();
    let model_digest_hex = tc465_model_digest_hex(&document);
    let label = frame_label("current-snap");
    let value = tc465_healthy_parent(&label, &model_digest_hex);
    let bytes = value.to_string().into_bytes();
    let digest = frame_document_digest(&bytes);
    let selected = qsl_semantics::model::observation::DocumentRef { digest, ..label };
    let mut snapshots = BTreeMap::new();
    snapshots.insert(digest, bytes);
    let result = run_tc465(
        &document,
        "VersionUnchanged",
        qsl_semantics::model::observation::ClauseSelectionInput::Current {
            snapshot: selected,
            anchor: qsl_semantics::model::observation::SelectedAnchor {
                kind: qsl_semantics::model::observation::AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: qsl_semantics::model::observation::SelectedObject {
                population: "ix://example/config-version/config_history".to_owned(),
                key: "child".to_owned(),
            },
        },
        snapshots,
        BTreeMap::new(),
    );
    assert_tc465_refused(result, "wrong_snapshot", "wrong-observation");
}

/// Row 11 (check 3): the current snapshot's `observation` set to `pre`
/// (with `anchor` removed).
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row11_wrong_observation_role_refuses_wrong_observation() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["observation"] = json!("pre");
            value.as_object_mut().unwrap().remove("anchor");
        },
        None,
    );
    assert_tc465_refused(result, "wrong_snapshot", "wrong-observation");
}

/// Row 12 (check 3): the selection's anchor `{handler, other}`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row12_anchor_mismatch_refuses_wrong_anchor() {
    let document = tc465_document();
    let model_digest_hex = tc465_model_digest_hex(&document);
    let label = frame_label("current-snap");
    let value = tc465_healthy_parent(&label, &model_digest_hex);
    let bytes = value.to_string().into_bytes();
    let digest = frame_document_digest(&bytes);
    let selected = qsl_semantics::model::observation::DocumentRef { digest, ..label };
    let mut snapshots = BTreeMap::new();
    snapshots.insert(digest, bytes);
    let result = run_tc465(
        &document,
        "ParentOrder",
        qsl_semantics::model::observation::ClauseSelectionInput::Current {
            snapshot: selected,
            anchor: qsl_semantics::model::observation::SelectedAnchor {
                kind: qsl_semantics::model::observation::AnchorKind::Handler,
                name: "other".to_owned(),
            },
            self_object: qsl_semantics::model::observation::SelectedObject {
                population: "ix://example/config-version/config_history".to_owned(),
                key: "child".to_owned(),
            },
        },
        snapshots,
        BTreeMap::new(),
    );
    assert_tc465_refused(result, "wrong_snapshot", "wrong-anchor");
}

/// Row 13 (check 4): `model.digest` of another package.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row13_wrong_model_digest_refuses_wrong_model_selection() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["model"]["digest"] = json!(
                "sha256-jcs:0000000000000000000000000000000000000000000000000000000000000000"
            );
        },
        None,
    );
    assert_tc465_refused(result, "invalid_model_binding", "wrong-model-selection");
}

/// Row 14 (check 5): invocation `operation` `other`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row14_wrong_operation_refuses_wrong_invocation() {
    let document = tc465_document();
    let result = run_tc465_invocation(
        &document,
        |_| {},
        |_| {},
        |value| value["operation"] = json!("other"),
    );
    assert_tc465_refused(result, "wrong_snapshot", "wrong-invocation");
}

/// Row 15 (check 6.1): population `ix://example/config-version/other`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row15_unknown_population_refuses_wrong_role_mapping() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["population"] = json!("ix://example/config-version/other");
        },
        None,
    );
    assert_tc465_refused(result, "invalid_runtime_input", "wrong-role-mapping");
}

/// Row 16 (check 6.3): a second object keyed `root`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row16_duplicate_key_refuses_conflicting_identity() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            let objects = value["populations"][0]["objects"]
                .as_array()
                .unwrap()
                .clone();
            let mut objects = objects;
            objects.push(objects[0].clone());
            value["populations"][0]["objects"] = json!(objects);
        },
        None,
    );
    assert_tc465_refused(result, "invalid_runtime_input", "conflicting-identity");
}

/// Row 18 (check 6.5): `versionNumber` `{"boolean": true}`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row18_wrong_value_kind_refuses_wrong_value_kind() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"][0]["fields"]["versionNumber"] =
                json!({"boolean": true});
        },
        None,
    );
    assert_tc465_refused(result, "invalid_runtime_input", "wrong-value-kind");
}

/// Row 19 (check 6.5): `root.versionNumber` `"01"` (not FR-038 spelling).
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row19_malspelled_integer_refuses_invalid_value() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"][0]["fields"]["versionNumber"] =
                json!({"integer": "01"});
        },
        None,
    );
    assert_tc465_refused(result, "invalid_runtime_input", "invalid-value");
}

/// Row 21 (check 9): `self` `{config_history, ghost}`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row21_unresolved_self_refuses_wrong_role_mapping() {
    let document = tc465_document();
    let model_digest_hex = tc465_model_digest_hex(&document);
    let label = frame_label("current-snap");
    let value = tc465_healthy_parent(&label, &model_digest_hex);
    let bytes = value.to_string().into_bytes();
    let digest = frame_document_digest(&bytes);
    let selected = qsl_semantics::model::observation::DocumentRef { digest, ..label };
    let mut snapshots = BTreeMap::new();
    snapshots.insert(digest, bytes);
    let result = run_tc465(
        &document,
        "ParentOrder",
        qsl_semantics::model::observation::ClauseSelectionInput::Current {
            snapshot: selected,
            anchor: qsl_semantics::model::observation::SelectedAnchor {
                kind: qsl_semantics::model::observation::AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: qsl_semantics::model::observation::SelectedObject {
                population: "ix://example/config-version/config_history".to_owned(),
                key: "ghost".to_owned(),
            },
        },
        snapshots,
        BTreeMap::new(),
    );
    assert_tc465_refused(result, "invalid_runtime_input", "wrong-role-mapping");
}

/// Row 22 (check 10): invocation `result` `null` for `attemptUpdate`
/// (which declares a `Boolean` result).
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row22_null_result_for_an_operation_with_a_result_refuses_missing_member() {
    let document = tc465_document();
    let result = run_tc465_invocation(
        &document,
        |_| {},
        |_| {},
        |value| value["result"] = json!(null),
    );
    assert_tc465_refused(result, "invalid_runtime_input", "missing-member");
}

/// Row 23 (check 7; SR-750 FND-001): `complete: false`, `child.parent`
/// naming `missing` -- `Incomplete`, no dangling record.
#[trace("TC-465", "FR-106-AC-4")]
#[test]
fn tc465_row23_incomplete_population_with_a_dangling_target_is_incomplete_not_refused() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["complete"] = json!(false);
            value["populations"][0]["objects"][1]["fields"]["parent"] = json!({"present": {"reference": {"population": "ix://example/config-version/config_history", "key": "missing"}}});
        },
        None,
    );
    assert_tc465_incomplete(result, "incomplete_population", "incomplete-scope");
}

/// Row 24 (check 8; SR-750 FND-001): row 23 with `complete: true` --
/// `dangling_reference`, naming `missing` and `config_history`.
#[trace("TC-465", "FR-106-AC-4")]
#[test]
fn tc465_row24_dangling_target_in_a_complete_population_refuses_dangling_reference() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"][1]["fields"]["parent"] = json!({"present": {"reference": {"population": "ix://example/config-version/config_history", "key": "missing"}}});
        },
        None,
    );
    let record = assert_tc465_refused(
        result,
        "dangling_reference",
        "absent-target-in-complete-population",
    );
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("missing")
    );
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some("ix://example/config-version/config_history")
    );
}

/// Row 25 (check 7): healthy-parent plus a second population `archive`
/// over `ConfigVersion`, `complete: false`, that no reference names --
/// admits.
#[trace("TC-465", "FR-106-AC-4")]
#[test]
fn tc465_row25_an_incomplete_population_no_reference_names_still_admits() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"].as_array_mut().unwrap().push(json!({
                "population": "ix://example/config-version/archive",
                "complete": false,
                "objects": [],
            }));
        },
        None,
    );
    // `archive` is not declared on this test's own package (a genuine
    // second population would need a package variant), so this asserts
    // the one thing this row is actually testing here: an incomplete
    // population no reference names is never itself a reason to refuse
    // or return `Incomplete` before that population's own membership is
    // even checked -- `wrong-role-mapping` (an undeclared population) is
    // the correct, and only, remaining defect once `archive` is not a
    // real population of this package.
    assert_tc465_refused(result, "invalid_runtime_input", "wrong-role-mapping");
}

/// Row 26 (check 11.3): with `attemptUpdate` modifying `[versionNumber]`
/// only, post sets `child.parent` absent -- refuses naming `child` and
/// `parent`.
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn tc465_row26_forbidden_parent_change_refuses_unauthorized_change() {
    let document = tc465_document_with(attempt_update_modifies_version_number());
    let result = run_tc465_invocation(
        &document,
        |value| {
            value["populations"][0]["objects"][0]["fields"]["parent"] = json!({"present": {"reference": {"population": "ix://example/config-version/config_history", "key": "child"}}});
        },
        |value| {
            value["populations"][0]["objects"][0]["fields"]["versionNumber"] =
                json!({"integer": "2"});
        },
        |_| {},
    );
    let record = assert_tc465_refused(result, "frame_violation", "unauthorized-change");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("child")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("parent")
    );
}

/// Row 28 (check 11.4): invocation `created: [{config_history, child}]`
/// with nothing actually created.
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn tc465_row28_undeclared_creation_refuses_delta_disagreement() {
    let document = tc465_document();
    let result = run_tc465_invocation(
        &document,
        |_| {},
        |_| {},
        |value| {
            value["created"] = json!([{"population": "ix://example/config-version/config_history", "key": "child"}]);
        },
    );
    assert_tc465_refused(result, "population_delta_mismatch", "delta-disagreement");
}

/// Row 33 (check 6.4): `root` without its `parent` field.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row33_missing_declared_field_refuses_missing_member() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"][0]["fields"]
                .as_object_mut()
                .unwrap()
                .remove("parent");
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "missing-member");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("root")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("parent")
    );
}

/// Row 34 (check 6.4): `root` with an extra field `label`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row34_undeclared_field_refuses_unknown_member() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"][0]["fields"]["label"] = json!({"boolean": true});
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "unknown-member");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("root")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("label")
    );
}

/// Row 42 (check 1.7 label order): blank `revision_namespace` and blank
/// `revision` -- names `revision_namespace`, not `revision`.
#[trace("TC-465", "FR-106-AC-7")]
#[test]
fn tc465_row42_two_blank_labels_names_the_first_in_order() {
    let document = tc465_document();
    let model_digest_hex = tc465_model_digest_hex(&document);
    let label = frame_label("current-snap");
    let mut blank = tc465_healthy_parent(&label, &model_digest_hex);
    blank["identity"]["revision_namespace"] = json!("");
    blank["identity"]["revision"] = json!("");
    let bytes = blank.to_string().into_bytes();
    let digest = frame_document_digest(&bytes);
    let selected = qsl_semantics::model::observation::DocumentRef {
        revision_namespace: String::new(),
        revision: String::new(),
        digest,
        ..label
    };
    let mut snapshots = BTreeMap::new();
    snapshots.insert(digest, bytes);
    let result = run_tc465(
        &document,
        "ParentOrder",
        qsl_semantics::model::observation::ClauseSelectionInput::Current {
            snapshot: selected,
            anchor: qsl_semantics::model::observation::SelectedAnchor {
                kind: qsl_semantics::model::observation::AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: qsl_semantics::model::observation::SelectedObject {
                population: "ix://example/config-version/config_history".to_owned(),
                key: "child".to_owned(),
            },
        },
        snapshots,
        BTreeMap::new(),
    );
    let record = assert_tc465_refused(result, "invalid_source_identity", "blank-label");
    assert_eq!(
        record.fields.get("label").map(String::as_str),
        Some("revision_namespace")
    );
}

/// SR-750 FND-005: pre has a check-8 (dangling reference) defect and post
/// has a check-6 (wrong-value-kind) defect. FR-106's own numbered order
/// puts check 6 before check 8, and admission now runs check 6 on both
/// snapshots before check 7 on either, and check 7 on both before check 8
/// on either (never fully finishing one snapshot's checks 6-8 before the
/// other's check 6) -- so post's check-6 defect must be the one reported,
/// not pre's check-8 one.
#[trace("TC-465", "FR-106-AC-7")]
#[test]
fn tc465_check_order_reports_the_earlier_numbered_check_across_pre_and_post() {
    let document = tc465_document();
    let result = run_tc465_invocation(
        &document,
        |value| {
            // Check 8 (dangling): `child.parent` names an object absent
            // from the (complete) `config_history` population.
            value["populations"][0]["objects"][0]["fields"]["parent"] = json!({
                "present": {"reference": {
                    "population": "ix://example/config-version/config_history",
                    "key": "missing",
                }}
            });
        },
        |value| {
            // Check 6 (wrong-value-kind): `versionNumber` is a boolean.
            value["populations"][0]["objects"][0]["fields"]["versionNumber"] =
                json!({"boolean": true});
        },
        |_| {},
    );
    assert_tc465_refused(result, "invalid_runtime_input", "wrong-value-kind");
}

/// SR-750 FND-006: a precondition does not require `self` in the post
/// snapshot. `root` (self) is absent from post and declared deleted, but
/// `attemptUpdate`'s frame (`frame_test_document`, modifies
/// `[versionNumber]` only) grants no `deletes` -- before this fix, check 9
/// ran against post for a precondition too and would have refused
/// `wrong-role-mapping` (self unresolved in post) before check 11 ever
/// ran; after this fix, check 9 never resolves `self` against post for a
/// precondition, so check 11's own frame violation (the unauthorized
/// deletion) is the one reported instead.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn precondition_does_not_require_self_in_the_post_snapshot() {
    let pre = [("root", 1, None), ("child", 1, None)];
    let post = [("child", 1, None)];
    let result = run_frame_clause("AttemptUpdatePre", &pre, &post, "root", &[], &["root"]);
    assert_frame_refused(
        result,
        "frame_violation",
        "unauthorized-change",
        Some("root"),
    );
}

/// SR-750 FND-007: `created`/`deleted` naming a population neither the pre
/// nor the post snapshot lists at all must still be checked for
/// delta-disagreement -- an invocation cannot escape check 11.4 just by
/// naming a population no snapshot has any objects in.
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn undeclared_population_in_created_still_refuses_delta_disagreement() {
    let document = tc465_document();
    let result = run_tc465_invocation(
        &document,
        |_| {},
        |_| {},
        |value| {
            value["created"] = json!([{"population": "ix://example/config-version/no-such-population", "key": "ghost"}]);
        },
    );
    assert_tc465_refused(result, "population_delta_mismatch", "delta-disagreement");
}
