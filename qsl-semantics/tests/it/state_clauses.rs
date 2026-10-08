// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-104: the `ProtocolClause` family's S3 check of a state clause,
//! against TC-458's `ConfigVersion` fixture package
//! (`crate::model_operations`).
//!
//! `versionNumber` is the package-declared bound `VersionNumber` scalar
//! (`Int[0, 1000]`, FR-104-AC-1), declared by the shared `ConfigVersion`
//! fixture ([`crate::model_operations::version_number_value_type`]) and read
//! by FR-056's `value-type/v1` scalar reader. An out-of-bound
//! `versionNumber` in a snapshot therefore refuses at admission (TC-465 rows
//! 20 and 30).

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_forms::StateClauseKind;
use qsl_foundation::bound::DomainKey;
use qsl_foundation::bound::DomainKind;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    AssemblyCause, AssemblyError, CheckCause, CheckRefusal, CheckedGraph, NodeKind, Observation,
    WrongSnapshotCause,
};
use qsl_semantics::family::ClaimExtent;
use quire_exact::ValueType;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::location::{Location, Origin};

use crate::model_operations::{
    admit_and_assemble_with_body, ambiguous_operation_document, archive_population,
    config_unit_with_body, config_version_document, config_version_document_with_operations,
    config_version_document_with_population, config_version_identity, empty_frame, operation,
    operation_parameter, subtype_document, subtype_document_with_two_member_population,
    version_number_bound, version_number_identity,
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

/// Every node of `root`'s subtree, root included. `CheckedNode::descendants`
/// is crate-private; `CheckedNode::children` is public, so this walks with
/// it.
fn descendants(
    root: qsl_semantics::check::CheckedNode<'_>,
) -> Vec<qsl_semantics::check::CheckedNode<'_>> {
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

    // `self.versionNumber` is an `Attribute` node, typed `Int[0, 1000]`
    // (FR-104-AC-1's bound `VersionNumber` scalar, not the native
    // `Integer`). `self.parent` is an `Attribute` node of
    // `Option<Reference<Config::ConfigVersion>>`.
    let attribute_types: Vec<&ValueType> = descendants(parent_order.body())
        .into_iter()
        .filter(|node| matches!(node.kind(), NodeKind::Attribute { .. }))
        .map(|node| node.value_type())
        .collect();
    let version_number_type = version_number_bound();
    assert!(
        attribute_types.contains(&&version_number_type),
        "self.versionNumber: Int[0, 1000]: {attribute_types:?}"
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
                NodeKind::Pre(operand)
                    if matches!(node.at(*operand).kind(), NodeKind::Attribute { .. })
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
            refusals[0].location.path(),
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
        Location::root(Origin::StateClause {
            clause: "ParentOrder".to_owned(),
            index: 0,
        }),
        "the first ParentOrder is its own locus"
    );
    assert_eq!(
        located[1].location,
        Location::root(Origin::StateClause {
            clause: "ParentOrder".to_owned(),
            index: 1,
        }),
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
        Location::root(Origin::Body {
            function: "ParentOrder".to_owned(),
            index: 0,
        }),
        "the function's own declaration is its locus"
    );
    assert_eq!(
        located[1].location,
        Location::root(Origin::StateClause {
            clause: "ParentOrder".to_owned(),
            index: 0,
        }),
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
        located[0].location.path(),
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
        located[0].location.path(),
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
        located[0].location.path(),
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
        located[0].location.path(),
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
        located[0].location.path(),
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
        Location::root(Origin::Body {
            function: "r".to_owned(),
            index: 0,
        }),
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
// and a postcondition on `attemptUpdate` (a gap: check 11 was
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
    let document = quire_canonical::read(bytes, u64::MAX).expect("test fixture is JSON");
    *quire_canonical::sha256(&document, quire_canonical::Limits::new(u64::MAX))
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
                None => json!({"absent": {}}),
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
    run_frame_clause_editing(
        clause_name,
        pre_objects,
        post_objects,
        self_key,
        created,
        deleted,
        |bytes| bytes,
    )
}

/// [`run_frame_clause`], with the invocation's bytes passed through
/// `edit_invocation` before they are digested and provisioned.
#[allow(clippy::too_many_arguments)]
fn run_frame_clause_editing(
    clause_name: &str,
    pre_objects: &[(&str, i64, Option<&str>)],
    post_objects: &[(&str, i64, Option<&str>)],
    self_key: &str,
    created: &[&str],
    deleted: &[&str],
    edit_invocation: impl FnOnce(Vec<u8>) -> Vec<u8>,
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    run_frame_clause_limited(
        clause_name,
        pre_objects,
        post_objects,
        self_key,
        created,
        deleted,
        edit_invocation,
        qsl_semantics::model::observation::ObservationLimits::default(),
    )
}

/// [`run_frame_clause_editing`] under the given `limits`.
#[allow(clippy::too_many_arguments)]
fn run_frame_clause_limited(
    clause_name: &str,
    pre_objects: &[(&str, i64, Option<&str>)],
    post_objects: &[(&str, i64, Option<&str>)],
    self_key: &str,
    created: &[&str],
    deleted: &[&str],
    edit_invocation: impl FnOnce(Vec<u8>) -> Vec<u8>,
    limits: qsl_semantics::model::observation::ObservationLimits,
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
    let invocation_bytes = edit_invocation(frame_invocation_bytes(
        &invocation,
        &model_digest,
        &pre,
        &post,
        self_key,
        created,
        deleted,
    ));
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
        limits,
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
            assert_eq!(record.code.as_str(), expected_code);
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

/// TC-464 (precondition over an invocation): `root`'s `versionNumber`
/// changes from 1 to 2; a precondition admits the pre side alone, with no
/// post observation.
#[trace("TC-464", "FR-106-AC-9")]
#[test]
fn attempt_update_precondition_admits_an_authorized_change() {
    let pre = [("root", 1, None)];
    let post = [("root", 2, None)];
    let observations = run_frame_clause("AttemptUpdatePre", &pre, &post, "root", &[], &[])
        .expect("a precondition over an invocation admits");
    assert!(observations.pre.is_some());
    assert!(observations.post.is_none());
}

/// TC-464 (FR-106 check 11, postcondition): the same authorized change,
/// admitted for `AttemptUpdatePost` over the same pre/post snapshots.
#[trace("TC-465", "FR-106-AC-5")]
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
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn attempt_update_refuses_an_unauthorized_create() {
    let pre = [("root", 1, None)];
    let post = [("root", 1, None), ("child", 1, None)];
    let result = run_frame_clause("AttemptUpdatePost", &pre, &post, "root", &["child"], &[]);
    assert_frame_refused(
        result,
        "frame_violation",
        "unauthorized-change",
        Some("child"),
    );
}

/// FR-106 check 11.2: the pre snapshot's `child`, absent from post, is not
/// granted by `attemptUpdate`'s (empty) `deletes` -- refuses naming `child`.
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn attempt_update_refuses_an_unauthorized_delete() {
    let pre = [("root", 1, None), ("child", 1, Some("root"))];
    let post = [("root", 1, None)];
    let result = run_frame_clause("AttemptUpdatePost", &pre, &post, "root", &[], &["child"]);
    assert_frame_refused(
        result,
        "frame_violation",
        "unauthorized-change",
        Some("child"),
    );
}

/// FR-106 check 11.3: `root`'s `parent` changes, a field outside
/// `attemptUpdate`'s `modifies [versionNumber]` -- refuses naming `root`.
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn attempt_update_refuses_a_change_outside_modifies() {
    let pre = [("root", 1, None), ("child", 1, None)];
    let post = [("root", 1, Some("child")), ("child", 1, None)];
    let result = run_frame_clause("AttemptUpdatePost", &pre, &post, "root", &[], &[]);
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
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn attempt_update_refuses_a_declared_delta_mismatch() {
    let pre = [("root", 1, None)];
    let post = [("root", 1, None)];
    let result = run_frame_clause("AttemptUpdatePost", &pre, &post, "root", &["child"], &[]);
    assert_frame_refused(
        result,
        "population_delta_mismatch",
        "delta-disagreement",
        None,
    );
}

// ---------------------------------------------------------------------------
// TC-464 (FR-106-AC-1, AC-2, AC-6; SR-751 FND-004): positive read and
// admission of FR-106's documents, digest stability, and that admission
// reads nothing ambient. Reuses TC-465's own ConfigVersion package and
// healthy-parent/changed-version fixtures (defined below in this file).
// ---------------------------------------------------------------------------

/// Step 1 (FR-106-AC-1): the healthy-parent snapshot admits one current
/// observation: `config_history` complete, `root.versionNumber` 1,
/// `root.parent` absent, `child.versionNumber` 2, `child.parent` present
/// naming `root`, `self` is `child`, and the snapshot's identity and digest
/// are retained.
#[trace("TC-464", "FR-106-AC-1")]
#[test]
fn tc464_step1_healthy_parent_admits_one_current_observation() {
    let document = tc465_document();
    let observations =
        run_tc465_current(&document, |_| {}, None).expect("the healthy-parent snapshot admits");
    let current = observations
        .current
        .expect("an invariant admits a current observation");
    assert_eq!(current.identity.identity, "current-snap");
    assert_ne!(current.identity.digest, [0; 32], "the digest is retained");
    assert_eq!(
        current
            .populations
            .get("ix://example/config-version/config_history"),
        Some(&true),
        "config_history is admitted complete"
    );
    assert_eq!(observations.self_object.object().as_str(), "child");
    let rendered = format!("{:?}", current.environment);
    assert!(
        rendered.contains(r#"Integer(1)"#) && rendered.contains(r#"Integer(2)"#),
        "root's and child's versionNumber (1, 2) are both admitted: {rendered}"
    );
    assert!(
        rendered.contains("Absent"),
        "root.parent is admitted absent: {rendered}"
    );
    assert!(
        rendered.contains(r#"object: ObjectId("root")"#),
        "child.parent names root: {rendered}"
    );
}

/// Step 2 (FR-106-AC-1): the same snapshot, re-serialized with other
/// whitespace and reversed member order, admits with the same digest and
/// an equal admitted value.
#[trace("TC-464", "FR-106-AC-1")]
#[test]
fn tc464_step2_a_reserialized_snapshot_gives_the_same_digest_and_an_equal_value() {
    let document = tc465_document();
    let model_digest_hex = tc465_model_digest_hex(&document);
    let label = frame_label("current-snap");
    let canonical = tc465_healthy_parent(&label, &model_digest_hex);

    // Reversed top-level member order and different (pretty) whitespace --
    // still the same RFC 8785 canonical bytes, so the same digest.
    let object = canonical.as_object().expect("an object");
    let mut reversed = serde_json::Map::new();
    for (key, value) in object.iter().rev() {
        reversed.insert(key.clone(), value.clone());
    }
    let reserialized =
        serde_json::to_string_pretty(&serde_json::Value::Object(reversed)).expect("valid JSON");

    let canonical_bytes = canonical.to_string().into_bytes();
    let canonical_digest = frame_document_digest(&canonical_bytes);
    let reserialized_bytes = reserialized.into_bytes();
    let reserialized_digest = frame_document_digest(&reserialized_bytes);
    assert_eq!(
        canonical_digest, reserialized_digest,
        "reordering members and changing whitespace does not change the RFC 8785 digest"
    );

    let selected = qsl_semantics::model::observation::DocumentRef {
        digest: canonical_digest,
        ..label.clone()
    };
    let mut snapshots_a = BTreeMap::new();
    snapshots_a.insert(canonical_digest, canonical_bytes);
    let mut snapshots_b = BTreeMap::new();
    snapshots_b.insert(reserialized_digest, reserialized_bytes);

    let selection = || qsl_semantics::model::observation::ClauseSelectionInput::Current {
        snapshot: selected.clone(),
        anchor: qsl_semantics::model::observation::SelectedAnchor {
            kind: qsl_semantics::model::observation::AnchorKind::Handler,
            name: "validate".to_owned(),
        },
        self_object: qsl_semantics::model::observation::SelectedObject {
            population: "ix://example/config-version/config_history".to_owned(),
            key: "child".to_owned(),
        },
    };
    let from_canonical = run_tc465(
        &document,
        "ParentOrder",
        selection(),
        snapshots_a,
        BTreeMap::new(),
    )
    .expect("the canonical bytes admit");
    let from_reserialized = run_tc465(
        &document,
        "ParentOrder",
        selection(),
        snapshots_b,
        BTreeMap::new(),
    )
    .expect("the re-serialized bytes admit");
    // `usage.document_bytes` (SR-751 FND-002 round 2) is deliberately
    // excluded here: it is the raw byte length actually consumed, which
    // legitimately differs between two representations of the same
    // canonical value (reordered members, pretty-printed whitespace) --
    // "an equal admitted value" is about the clause, observations and
    // environment this step's own name and trace describe, not about
    // admission's resource usage.
    let strip_usage =
        |mut observations: qsl_semantics::model::observation::AdmittedObservations| {
            observations.usage = qsl_semantics::model::observation::AdmissionUsage::default();
            observations
        };
    assert_eq!(
        format!("{:?}", strip_usage(from_canonical)),
        format!("{:?}", strip_usage(from_reserialized)),
        "an equal admitted value, whatever whitespace or member order the bytes used"
    );
}

/// Step 3 (FR-106-AC-2): the changed-version invocation admits for
/// `VersionUnchanged`, with distinct pre and post observations, `self`
/// `child` in both, `result` true, no parameters, and empty created and
/// deleted.
#[trace("TC-464", "FR-106-AC-2")]
#[test]
fn tc464_step3_changed_version_invocation_admits_a_full_observation_set() {
    let document = tc465_document();
    let observations = run_tc465_invocation(&document, |_| {}, |_| {}, |_| {})
        .expect("the changed-version invocation admits");
    let pre = observations
        .pre
        .expect("a postcondition admits a pre observation");
    let post = observations
        .post
        .expect("a postcondition admits a post observation");
    assert_ne!(
        format!("{pre:?}"),
        format!("{post:?}"),
        "pre and post are distinct observations"
    );
    assert_eq!(observations.self_object.object().as_str(), "child");
    assert!(observations.parameters.is_empty(), "no parameters");
    assert!(observations.created.is_empty(), "empty created");
    assert!(observations.deleted.is_empty(), "empty deleted");
    assert!(
        matches!(observations.result, Some(quire_exact::Value::Boolean(true))),
        "result is true: {:?}",
        observations.result
    );
    let pre_rendered = format!("{:?}", pre.environment);
    let post_rendered = format!("{:?}", post.environment);
    assert!(
        pre_rendered.contains("Integer(2)"),
        "pre's child.versionNumber is 2: {pre_rendered}"
    );
    assert!(
        post_rendered.contains("Integer(3)"),
        "post's child.versionNumber is 3: {post_rendered}"
    );
}

/// SR-751 FND-002 round 2: `AdmittedObservations::usage` (FR-109 Outputs'
/// admission-work half of usage) is not left empty for a real,
/// successful admission -- it reports the real byte length, nesting
/// depth and object/value counts this invocation's pre and post
/// snapshots (and the invocation document itself) actually consumed, not
/// a hardcoded zero.
#[trace("TC-464", "FR-106-AC-2")]
#[test]
fn tc464_admission_usage_reports_the_real_documents_consumed() {
    let document = tc465_document();
    let observations = run_tc465_invocation(&document, |_| {}, |_| {}, |_| {})
        .expect("the changed-version invocation admits");
    let usage = observations.usage;
    assert!(
        usage.document_bytes > 0,
        "the sum of every admitted document's byte length is nonzero"
    );
    assert!(
        usage.objects > 0,
        "pre and post each admit at least one object"
    );
    assert!(
        usage.values > 0,
        "pre and post each admit at least one field value"
    );
}

/// Step 4 (FR-106-AC-6): repeating steps 1 and 3 in a process whose
/// working directory is an empty temporary directory gives equal results --
/// admission reads nothing ambient (its provisions are in-memory maps).
/// Serialized against every other test in this binary via a lock, since
/// `std::env::set_current_dir` is process-wide state.
#[trace("TC-464", "FR-106-AC-6")]
#[test]
fn tc464_step4_admission_reads_nothing_ambient() {
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = CWD_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let original_cwd = std::env::current_dir().expect("a current directory exists");
    let document = tc465_document();
    let before_current = run_tc465_current(&document, |_| {}, None)
        .expect("the healthy-parent snapshot admits outside the temp dir");
    let before_invocation = run_tc465_invocation(&document, |_| {}, |_| {}, |_| {})
        .expect("the changed-version invocation admits outside the temp dir");

    let empty_dir = tempfile::tempdir().expect("an empty temporary directory");
    std::env::set_current_dir(empty_dir.path()).expect("chdir into the empty temp dir");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let after_current = run_tc465_current(&document, |_| {}, None)
            .expect("the healthy-parent snapshot admits inside the empty temp dir");
        let after_invocation = run_tc465_invocation(&document, |_| {}, |_| {}, |_| {})
            .expect("the changed-version invocation admits inside the empty temp dir");
        (after_current, after_invocation)
    }));
    std::env::set_current_dir(&original_cwd).expect("restore the original working directory");
    let (after_current, after_invocation) = result.unwrap_or_else(|payload| {
        std::panic::resume_unwind(payload);
    });

    assert_eq!(
        format!("{before_current:?}"),
        format!("{after_current:?}"),
        "an empty working directory admits the same current observation"
    );
    assert_eq!(
        format!("{before_invocation:?}"),
        format!("{after_invocation:?}"),
        "an empty working directory admits the same invocation observation"
    );
}

// ---------------------------------------------------------------------------
// TC-464 step 5 (FR-106-AC-8): a `PreCall` selection over the chain
// `a -> b -> c` pre snapshot, for `ReachesTarget` (`probe`'s precondition).
// ---------------------------------------------------------------------------

/// The chain `a -> b -> c` (`c.parent` absent) as a `config_history`
/// snapshot observed `observation`, with `extra_populations` listed after
/// `config_history`.
fn tc464_chain_snapshot(
    label: &qsl_semantics::model::observation::DocumentRef,
    model_digest_hex: &str,
    observation: &str,
    complete: bool,
    extra_populations: Vec<serde_json::Value>,
) -> Vec<u8> {
    let population = "ix://example/config-version/config_history";
    let config_version = "ix://example/config-version/ConfigVersion";
    let object = |key: &str, parent: Option<&str>| {
        let parent = match parent {
            Some(parent) => {
                json!({"present": {"reference": {"population": population, "key": parent}}})
            }
            None => json!({"absent": {}}),
        };
        json!({"key": key, "type": config_version,
               "fields": {"versionNumber": {"integer": "1"}, "parent": parent}})
    };
    let mut populations = vec![json!({
        "population": population,
        "complete": complete,
        "objects": [object("a", Some("b")), object("b", Some("c")), object("c", None)],
    })];
    populations.extend(extra_populations);
    json!({
        "format": "quire.state.snapshot/v1",
        "identity": frame_identity_json(label),
        "observation": observation,
        "model": frame_model_header(model_digest_hex),
        "populations": populations,
    })
    .to_string()
    .into_bytes()
}

/// `{population, key}` as a snapshot-form reference value.
fn tc464_reference(
    population: &str,
    key: &str,
) -> qsl_semantics::model::observation::SnapshotValue {
    qsl_semantics::model::observation::SnapshotValue::reference(
        qsl_semantics::model::observation::SelectedObject {
            population: population.to_owned(),
            key: key.to_owned(),
        },
    )
}

/// Admits `clause_name` over `document` with `PreCall { snapshot, self: a,
/// parameters }`, the snapshot being `snapshot_bytes`.
fn run_tc464_pre_call(
    document: &[u8],
    clause_name: &str,
    snapshot_bytes: Vec<u8>,
    parameters: Vec<(&str, qsl_semantics::model::observation::SnapshotValue)>,
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    let snapshot = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&snapshot_bytes),
        ..frame_label("pre-call-snap")
    };
    let mut snapshots = BTreeMap::new();
    snapshots.insert(snapshot.digest, snapshot_bytes);
    run_tc465(
        document,
        clause_name,
        qsl_semantics::model::observation::ClauseSelectionInput::PreCall {
            snapshot,
            self_object: qsl_semantics::model::observation::SelectedObject {
                population: "ix://example/config-version/config_history".to_owned(),
                key: "a".to_owned(),
            },
            parameters: parameters
                .into_iter()
                .map(|(name, value)| (quire_exact::Identifier::new(name).unwrap(), value))
                .collect(),
        },
        snapshots,
        BTreeMap::new(),
    )
}

fn tc464_chain(document: &[u8], observation: &str, complete: bool) -> Vec<u8> {
    tc464_chain_snapshot(
        &frame_label("pre-call-snap"),
        &tc465_model_digest_hex(document),
        observation,
        complete,
        Vec::new(),
    )
}

const TC464_CONFIG_HISTORY: &str = "ix://example/config-version/config_history";

/// Step 5 (FR-106-AC-8): `PreCall { snapshot, self: a, parameters: {target:
/// a} }` for `ReachesTarget` admits one pre observation, `self` `a`, the
/// parameter `target` naming `a`, and no post observation, result or delta
/// (no frame check runs: there is no post state to compare).
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn tc464_step5_a_pre_call_admits_one_pre_observation() {
    let document = tc465_document();
    let observations = run_tc464_pre_call(
        &document,
        "ReachesTarget",
        tc464_chain(&document, "pre", true),
        vec![("target", tc464_reference(TC464_CONFIG_HISTORY, "a"))],
    )
    .expect("the pre-call observation admits");
    let pre = observations.pre.expect("one pre observation");
    assert_eq!(pre.identity.identity, "pre-call-snap");
    assert_ne!(pre.identity.digest, [0; 32]);
    assert!(observations.current.is_none());
    assert!(observations.post.is_none(), "no post observation");
    assert!(observations.result.is_none(), "no result");
    assert!(observations.created.is_empty() && observations.deleted.is_empty());
    assert_eq!(observations.self_object.object().as_str(), "a");
    let [(name, quire_exact::Value::Reference(target))] = &observations.parameters[..] else {
        panic!("one reference parameter: {:?}", observations.parameters);
    };
    assert_eq!(name, "target");
    assert_eq!(target.object().as_str(), "a");
    assert_eq!(target, &observations.self_object);
}

/// Step 5 (FR-106-AC-8): the same selection for `VersionUnchanged`, a
/// postcondition, refuses `wrong_snapshot`/`wrong-observation`.
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn tc464_step5_a_pre_call_for_a_postcondition_refuses_wrong_observation() {
    let document = tc465_document();
    let result = run_tc464_pre_call(
        &document,
        "VersionUnchanged",
        tc464_chain(&document, "pre", true),
        vec![("target", tc464_reference(TC464_CONFIG_HISTORY, "a"))],
    );
    assert_tc465_refused(result, "wrong_snapshot", "wrong-observation");
}

/// Step 5 (FR-106-AC-8): a pre-call snapshot that says `post` refuses
/// `wrong_snapshot`/`wrong-observation`.
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn tc464_step5_a_pre_call_snapshot_saying_post_refuses_wrong_observation() {
    let document = tc465_document();
    let result = run_tc464_pre_call(
        &document,
        "ReachesTarget",
        tc464_chain(&document, "post", true),
        vec![("target", tc464_reference(TC464_CONFIG_HISTORY, "a"))],
    );
    assert_tc465_refused(result, "wrong_snapshot", "wrong-observation");
}

/// Step 5 (FR-106-AC-8): no `target` refuses `invalid_runtime_input`/
/// `missing-member`.
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn tc464_step5_a_pre_call_without_target_refuses_missing_member() {
    let document = tc465_document();
    let result = run_tc464_pre_call(
        &document,
        "ReachesTarget",
        tc464_chain(&document, "pre", true),
        Vec::new(),
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "missing-member");
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("target")
    );
}

/// Step 5 (FR-106-AC-8): `target` naming `ghost`, absent from the complete
/// `config_history`, refuses `dangling_reference`/
/// `absent-target-in-complete-population` naming `ghost` and
/// `config_history`.
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn tc464_step5_a_pre_call_target_naming_ghost_refuses_dangling_reference() {
    let document = tc465_document();
    let result = run_tc464_pre_call(
        &document,
        "ReachesTarget",
        tc464_chain(&document, "pre", true),
        vec![("target", tc464_reference(TC464_CONFIG_HISTORY, "ghost"))],
    );
    let record = assert_tc465_refused(
        result,
        "dangling_reference",
        "absent-target-in-complete-population",
    );
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("ghost")
    );
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some(TC464_CONFIG_HISTORY)
    );
}

/// Step 5 (FR-106-AC-8): over the package variant where `Sub` specializes
/// `ConfigVersion` and `archive` has member type `Sub`, `target` naming
/// `a1` in an `archive` listed with no objects and marked `complete:
/// false` -- which no field of `a`, `b` or `c` reaches -- is `Incomplete`
/// with `incomplete_population`/`incomplete-scope` naming `archive`: a
/// declared population a reference parameter names is required.
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn tc464_step5_a_pre_call_target_in_an_incomplete_archive_is_incomplete() {
    let document = tc465_document_with_archive_population();
    let archive = "ix://example/config-version/archive";
    let snapshot = tc464_chain_snapshot(
        &frame_label("pre-call-snap"),
        &tc465_model_digest_hex(&document),
        "pre",
        true,
        vec![json!({"population": archive, "complete": false, "objects": []})],
    );
    let result = run_tc464_pre_call(
        &document,
        "ReachesTarget",
        snapshot,
        vec![("target", tc464_reference(archive, "a1"))],
    );
    let record = assert_tc465_incomplete(result, "incomplete_population", "incomplete-scope");
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some(archive)
    );
}

/// Step 5 (FR-106-AC-8): an undeclared parameter `zextra` naming `a1` in
/// the incomplete `archive` makes no population required: check 10 refuses
/// it `invalid_runtime_input`/`unknown-member`, not check 7 `Incomplete`.
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn tc464_step5_an_undeclared_parameter_requires_no_population() {
    let document = tc465_document_with_archive_population();
    let archive = "ix://example/config-version/archive";
    let snapshot = tc464_chain_snapshot(
        &frame_label("pre-call-snap"),
        &tc465_model_digest_hex(&document),
        "pre",
        true,
        vec![json!({"population": archive, "complete": false, "objects": []})],
    );
    let result = run_tc464_pre_call(
        &document,
        "ReachesTarget",
        snapshot,
        vec![
            ("target", tc464_reference(TC464_CONFIG_HISTORY, "a")),
            ("zextra", tc464_reference(archive, "a1")),
        ],
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "unknown-member");
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("zextra")
    );
}

/// Step 5 (FR-106-AC-8): only a reference-valued parameter makes a
/// population required. Over an operation `flag(b: Boolean)` with the
/// precondition `FlagHolds { b }`, a `b` given a reference to `a1` in the
/// incomplete `archive` refuses check 10 `invalid_runtime_input`/
/// `wrong-value-kind`, not check 7 `Incomplete`.
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn tc464_step5_a_non_reference_parameter_requires_no_population() {
    let flag = operation(
        "flag",
        json!([operation_parameter(
            "flag",
            "b",
            "ix://quire/native/Boolean"
        )]),
        None,
        empty_frame(),
    );
    let document = with_archive_population(config_version_document_with_operations(vec![
        attempt_update_modifies_version_and_parent(),
        probe_operation(),
        flag,
    ]));
    let clauses =
        format!("{TC465_CLAUSES}pre FlagHolds using v on Config::ConfigVersion::flag {{ b }}\n");
    let archive = "ix://example/config-version/archive";
    let snapshot_bytes = tc464_chain_snapshot(
        &frame_label("pre-call-snap"),
        &tc465_model_digest_hex(&document),
        "pre",
        true,
        vec![json!({"population": archive, "complete": false, "objects": []})],
    );
    let snapshot = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&snapshot_bytes),
        ..frame_label("pre-call-snap")
    };
    let result = run_tc465_with_clauses(
        &document,
        &clauses,
        "FlagHolds",
        qsl_semantics::model::observation::ClauseSelectionInput::PreCall {
            snapshot: snapshot.clone(),
            self_object: qsl_semantics::model::observation::SelectedObject {
                population: TC464_CONFIG_HISTORY.to_owned(),
                key: "a".to_owned(),
            },
            parameters: BTreeMap::from([(
                quire_exact::Identifier::new("b").unwrap(),
                tc464_reference(archive, "a1"),
            )]),
        },
        BTreeMap::from([(snapshot.digest, snapshot_bytes)]),
        BTreeMap::new(),
        qsl_semantics::model::observation::ObservationLimits::default(),
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "wrong-value-kind");
    assert_eq!(record.fields.get("field").map(String::as_str), Some("b"));
}

// ---------------------------------------------------------------------------
// Check 6.5/10 subtype admission (QSpec FR-151): a `Reference<T>` parameter
// admits an object whose most-specific type conforms to `T` under the
// declared supertypes graph, and refuses `wrong-value-kind` otherwise.
// ---------------------------------------------------------------------------

/// `document` with `Other`, an object type unrelated to `ConfigVersion` (no
/// supertypes, nothing specializes it), and the population `others` whose
/// only declared member is `Other`.
fn with_unrelated_population(document: Vec<u8>) -> Vec<u8> {
    let mut envelope: serde_json::Value = serde_json::from_slice(&document).expect("valid JSON");
    let other = "ix://example/config-version/Other";
    let others = "ix://example/config-version/others";
    envelope["types"].as_array_mut().unwrap().push(json!({
        "identity": other,
        "displayName": other,
        "kind": {"module": "example/config-version", "name": "object_type"},
        "roles": [],
        "origin": {
            "generated": {
                "generatorIdentity": other,
                "generatorVersion": "1.0.0",
                "inputIdentities": [other],
            }
        },
        "constraints": [],
        "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": [],
        "fields": [],
        "operations": [],
    }));
    envelope["populations"].as_array_mut().unwrap().push(json!({
        "identity": others,
        "displayName": others,
        "kind": {"module": "example/config-version", "name": "population"},
        "members": [other],
        "extent": "closed",
        "origin": {
            "generated": {
                "generatorIdentity": others,
                "generatorVersion": "1.0.0",
                "inputIdentities": [others],
            }
        },
    }));
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// A complete `archive` holding the one `Sub` object `k`.
fn archive_with_sub_object() -> serde_json::Value {
    json!({
        "population": "ix://example/config-version/archive",
        "complete": true,
        "objects": [{
            "key": "k", "type": "ix://example/config-version/Sub",
            "fields": {"versionNumber": {"integer": "1"}, "parent": {"absent": {}}},
        }],
    })
}

/// `probe(target: ConfigVersion)` bound to `k`, a `Sub` object of the
/// complete `archive` (`Sub` specializes `ConfigVersion`), admits, and the
/// admitted `target` is `k` typed by its own most-specific type `Sub`.
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn a_subtype_object_binds_a_supertype_parameter() {
    let document = tc465_document_with_archive_population();
    let snapshot = tc464_chain_snapshot(
        &frame_label("pre-call-snap"),
        &tc465_model_digest_hex(&document),
        "pre",
        true,
        vec![archive_with_sub_object()],
    );
    let observations = run_tc464_pre_call(
        &document,
        "ReachesTarget",
        snapshot,
        vec![(
            "target",
            tc464_reference("ix://example/config-version/archive", "k"),
        )],
    )
    .expect("a Sub object admits for a ConfigVersion parameter");
    let [(name, quire_exact::Value::Reference(target))] = &observations.parameters[..] else {
        panic!("one reference parameter: {:?}", observations.parameters);
    };
    assert_eq!(name, "target");
    assert_eq!(target.object().as_str(), "k");
    let pre = observations.pre.as_ref().expect("one pre observation");
    assert_eq!(
        pre.environment.objects().find(target.universe(), "k"),
        Some(target),
        "target is archive's k, typed by k's own type"
    );
    assert_ne!(
        target.object_type(),
        observations.self_object.object_type(),
        "target keeps Sub, not the declared ConfigVersion"
    );
}

/// `probe(target: ConfigVersion)` bound to `o`, an object of `Other`, a type
/// with no declared-supertype chain to `ConfigVersion`, refuses check 10
/// `invalid_runtime_input`/`wrong-value-kind` at `target`.
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn an_unrelated_type_object_refuses_a_parameter_with_wrong_value_kind() {
    let document = with_unrelated_population(tc465_document());
    let others = "ix://example/config-version/others";
    let snapshot = tc464_chain_snapshot(
        &frame_label("pre-call-snap"),
        &tc465_model_digest_hex(&document),
        "pre",
        true,
        vec![json!({
            "population": others,
            "complete": true,
            "objects": [{"key": "o", "type": "ix://example/config-version/Other", "fields": {}}],
        })],
    );
    let result = run_tc464_pre_call(
        &document,
        "ReachesTarget",
        snapshot,
        vec![("target", tc464_reference(others, "o"))],
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "wrong-value-kind");
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("target")
    );
}

/// `probeSub(target: Sub)` bound to `a`, a `ConfigVersion` object: the
/// supertype does not conform to its subtype, so check 10 refuses
/// `invalid_runtime_input`/`wrong-value-kind` at `target`.
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn a_supertype_object_refuses_a_subtype_parameter_with_wrong_value_kind() {
    let probe_sub = operation(
        "probeSub",
        json!([operation_parameter(
            "probeSub",
            "target",
            "ix://example/config-version/Sub"
        )]),
        None,
        empty_frame(),
    );
    let document = with_archive_population(config_version_document_with_operations(vec![
        attempt_update_modifies_version_and_parent(),
        probe_operation(),
        probe_sub,
    ]));
    let clauses = format!(
        "{TC465_CLAUSES}pre SubTargetHolds using v on Config::ConfigVersion::probeSub {{ true }}\n"
    );
    let snapshot_bytes = tc464_chain(&document, "pre", true);
    let snapshot = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&snapshot_bytes),
        ..frame_label("pre-call-snap")
    };
    let result = run_tc465_with_clauses(
        &document,
        &clauses,
        "SubTargetHolds",
        qsl_semantics::model::observation::ClauseSelectionInput::PreCall {
            snapshot: snapshot.clone(),
            self_object: qsl_semantics::model::observation::SelectedObject {
                population: TC464_CONFIG_HISTORY.to_owned(),
                key: "a".to_owned(),
            },
            parameters: BTreeMap::from([(
                quire_exact::Identifier::new("target").unwrap(),
                tc464_reference(TC464_CONFIG_HISTORY, "a"),
            )]),
        },
        BTreeMap::from([(snapshot.digest, snapshot_bytes)]),
        BTreeMap::new(),
        qsl_semantics::model::observation::ObservationLimits::default(),
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "wrong-value-kind");
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("target")
    );
}

/// Step 5's archive case over an invocation (FR-106 required populations:
/// "of an invocation or a `PreCall` selection, in the pre snapshot"): the
/// `probe` invocation's `target` naming `a1` in an `archive` its pre
/// snapshot lists with no objects and marks `complete: false` is
/// `Incomplete` with `incomplete_population`/`incomplete-scope` naming
/// `archive`.
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn tc464_step5_an_invocation_target_in_an_incomplete_archive_is_incomplete() {
    let document = tc465_document_with_archive_population();
    let archive = "ix://example/config-version/archive";
    let result = run_tc465_probe_with(
        &document,
        "ReachesTarget",
        |pre| {
            pre["populations"]
                .as_array_mut()
                .expect("the snapshot lists its populations")
                .push(json!({"population": archive, "complete": false, "objects": []}));
        },
        |invocation| {
            invocation["parameters"]["target"] =
                json!({"reference": {"population": archive, "key": "a1"}});
        },
    );
    let record = assert_tc465_incomplete(result, "incomplete_population", "incomplete-scope");
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some(archive)
    );
}

/// Step 5 (FR-106-AC-8): the pre-call snapshot marked `complete: false` is
/// `Incomplete` with `incomplete_population`/`incomplete-scope`.
#[trace("TC-464", "FR-106-AC-8")]
#[test]
fn tc464_step5_an_incomplete_pre_call_snapshot_is_incomplete() {
    let document = tc465_document();
    let result = run_tc464_pre_call(
        &document,
        "ReachesTarget",
        tc464_chain(&document, "pre", false),
        vec![("target", tc464_reference(TC464_CONFIG_HISTORY, "a"))],
    );
    let record = assert_tc465_incomplete(result, "incomplete_population", "incomplete-scope");
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some(TC464_CONFIG_HISTORY)
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
    self.versionNumber = pre(self.versionNumber) }\n\
    pre ReachesTarget using v on Config::ConfigVersion::probe { \
    reaches(self, target, parent) }\n\
    post ProbeHolds using v on Config::ConfigVersion::probe { true }\n";

fn tc465_document_with(operation: serde_json::Value) -> Vec<u8> {
    config_version_document_with_operations(vec![operation, probe_operation()])
}

fn tc465_document() -> Vec<u8> {
    tc465_document_with(attempt_update_modifies_version_and_parent())
}

/// [`tc465_document`], with `attemptUpdate` modifying `[parent]` only
/// (row 27: an authorized-elsewhere change outside `modifies` must still
/// refuse).
fn tc465_document_with_frame_modifies_parent_only() -> Vec<u8> {
    tc465_document_with(operation(
        "attemptUpdate",
        json!([]),
        Some("ix://quire/native/Boolean"),
        json!({
            "modifies": ["ix://example/config-version/ConfigVersion/parent"],
            "creates": [],
            "deletes": [],
        }),
    ))
}

/// [`tc465_document`], with `ConfigVersion` also declaring `tags`, a set
/// of `ConfigVersion` (row 17: check 6.2's set/bag/ordered-set refusal).
/// Mutated directly at the JSON level (`config_version_document_with_
/// operations` has no set-field builder), matching the ad hoc envelope
/// patching [`config_version_document_with_population`] already uses for
/// its own extra population.
fn tc465_document_with_set_field() -> Vec<u8> {
    let document = tc465_document();
    let mut envelope: serde_json::Value = serde_json::from_slice(&document).expect("valid JSON");
    let config_version = "ix://example/config-version/ConfigVersion";
    let tags_identity = format!("{config_version}/tags");
    envelope["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["identity"] == config_version)
        .expect("ConfigVersion is declared")["fields"]
        .as_array_mut()
        .expect("ConfigVersion declares fields")
        .push(json!({
            "identity": tags_identity,
            "name": "tags",
            "typeRef": config_version,
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 0, "upper": 5, "ordered": false, "unique": true},
            "origin": {
                "generated": {
                    "generatorIdentity": tags_identity,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [tags_identity],
                }
            },
        }));
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// [`tc465_document`], with an extra object type `Note` declared but named
/// by no population's `members` (row 32: check 6.1's wrong-role-mapping
/// for an object whose type is not a member type of its population).
fn tc465_document_with_note_type() -> Vec<u8> {
    let document = tc465_document();
    let mut envelope: serde_json::Value = serde_json::from_slice(&document).expect("valid JSON");
    let note = "ix://example/config-version/Note";
    envelope["types"].as_array_mut().unwrap().push(json!({
        "identity": note,
        "displayName": note,
        "kind": {"module": "example/config-version", "name": "object_type"},
        "roles": [],
        "origin": {
            "generated": {
                "generatorIdentity": note,
                "generatorVersion": "1.0.0",
                "inputIdentities": [note],
            }
        },
        "constraints": [],
        "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": [],
        "fields": [],
        "operations": [],
    }));
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// [`tc465_document_with`]'s `attemptUpdate` modifying `[versionNumber]`
/// only, plus an independent object type `Tag` (no supertype relation to
/// `ConfigVersion`) that declares its *own* field also named
/// `versionNumber` -- same display name, unrelated declaring type. `Tag` is
/// declared a member of `config_history` too (SR-750 FND-007 round 2's own
/// cross-type test: a grant on `ConfigVersion::versionNumber` must never
/// authorize a write to `Tag::versionNumber`, a same-named field of an
/// unrelated type).
fn tc465_document_with_tag_type_sharing_a_field_name() -> Vec<u8> {
    let document = tc465_document_with(attempt_update_modifies_version_number());
    let mut envelope: serde_json::Value = serde_json::from_slice(&document).expect("valid JSON");
    let tag = "ix://example/config-version/Tag";
    let tag_version_number = format!("{tag}/versionNumber");
    envelope["types"].as_array_mut().unwrap().push(json!({
        "identity": tag,
        "displayName": tag,
        "kind": {"module": "example/config-version", "name": "object_type"},
        "roles": [],
        "origin": {
            "generated": {
                "generatorIdentity": tag,
                "generatorVersion": "1.0.0",
                "inputIdentities": [tag],
            }
        },
        "constraints": [],
        "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": [],
        "fields": [{
            "identity": tag_version_number,
            "name": "versionNumber",
            "typeRef": "ix://quire/native/Integer",
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": {
                "generated": {
                    "generatorIdentity": tag_version_number.clone(),
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [tag_version_number],
                }
            },
        }],
        "operations": [],
    }));
    envelope["populations"][0]["members"]
        .as_array_mut()
        .unwrap()
        .push(json!(tag));
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// SR-750 FND-007 round 2: a grant on `ConfigVersion::versionNumber` (the
/// invocation's own `attemptUpdate` modifies exactly that field) never
/// authorizes a write to `Tag::versionNumber` -- a same-named field of an
/// unrelated type, present as a surviving object across the same
/// invocation's pre and post.
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn a_same_named_field_of_an_unrelated_type_is_never_authorized() {
    let document = tc465_document_with_tag_type_sharing_a_field_name();
    let tag = "ix://example/config-version/Tag";
    let population = "ix://example/config-version/config_history";
    let result = run_tc465_invocation(
        &document,
        move |value| {
            value["populations"][0]["objects"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "key": "t1", "type": tag,
                    "fields": {"versionNumber": {"integer": "1"}},
                }));
        },
        move |value| {
            value["populations"][0]["objects"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "key": "t1", "type": tag,
                    "fields": {"versionNumber": {"integer": "2"}},
                }));
        },
        |_| {},
    );
    let record = assert_tc465_refused(result, "frame_violation", "unauthorized-change");
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some(population)
    );
    assert_eq!(record.fields.get("object").map(String::as_str), Some("t1"));
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("versionNumber")
    );
}

/// SR-750 FND-007 round 3: check 11 authorizes a creation by conformance
/// to a `creates` grant, as `enforce_frame` does. `attemptUpdate` creates
/// `ConfigVersion`; the post snapshot adds `s1` of type `Sub` (a subtype
/// of `ConfigVersion`, a member of `config_history`), declared in
/// `created`. This admits, with `s1` created as a `Sub`.
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn a_creation_of_a_subtype_of_a_creates_grant_admits() {
    let config_version = "ix://example/config-version/ConfigVersion";
    let document = add_sub_type(
        tc465_document_with(operation(
            "attemptUpdate",
            json!([]),
            Some("ix://quire/native/Boolean"),
            json!({
                "modifies": [config_version_identity("versionNumber")],
                "creates": [config_version],
                "deletes": [],
            }),
        )),
        &["ix://example/config-version/config_history"],
    );
    let population = "ix://example/config-version/config_history";
    let sub = "ix://example/config-version/Sub";
    let result = run_tc465_invocation(
        &document,
        |_| {},
        move |value| {
            value["populations"][0]["objects"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "key": "s1", "type": sub,
                    "fields": {"versionNumber": {"integer": "1"}, "parent": {"absent": {}}},
                }));
        },
        move |value| {
            value["created"] = json!([{"population": population, "key": "s1"}]);
        },
    );
    let admitted = result.unwrap_or_else(|failure| panic!("expected admission, got {failure:?}"));
    assert_eq!(admitted.created.len(), 1);
    assert_eq!(admitted.created[0].object().as_str(), "s1");
    let post = admitted.post.as_ref().expect("post observation");
    assert!(post.environment.objects().contains(&admitted.created[0]));
}

/// SR-750 FND-007 round 3: check 11 authorizes a field write through the
/// field's `redefines` chain, as `enforce_frame` does, never by display
/// name. `Sub` declares `version`, which redefines
/// `ConfigVersion::versionNumber`; `attemptUpdate` modifies only
/// `versionNumber`. A surviving `Sub` object whose `version` changes is
/// covered by that grant, so this admits.
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn a_write_to_a_field_that_redefines_a_modifies_grant_admits() {
    let document = tc465_document_with_sub_redefining_version_number();
    let sub = "ix://example/config-version/Sub";
    let sub_object = move |version: &str| {
        json!({
            "key": "s1", "type": sub,
            "fields": {"version": {"integer": version}, "parent": {"absent": {}}},
        })
    };
    let result = run_tc465_invocation(
        &document,
        move |value| {
            value["populations"][0]["objects"]
                .as_array_mut()
                .unwrap()
                .push(sub_object("1"));
        },
        move |value| {
            value["populations"][0]["objects"]
                .as_array_mut()
                .unwrap()
                .push(sub_object("2"));
        },
        |_| {},
    );
    if let Err(failure) = result {
        panic!("expected admission, got {failure:?}");
    }
}

/// [`tc465_document_with`]'s `attemptUpdate` modifying `[versionNumber]`
/// only, plus `Sub` (a subtype of `ConfigVersion` and a member of
/// `config_history`) declaring `version`, which redefines
/// `ConfigVersion::versionNumber` under another name.
fn tc465_document_with_sub_redefining_version_number() -> Vec<u8> {
    let document = add_sub_type(
        tc465_document_with(attempt_update_modifies_version_number()),
        &["ix://example/config-version/config_history"],
    );
    let mut envelope: serde_json::Value = serde_json::from_slice(&document).expect("valid JSON");
    let sub = "ix://example/config-version/Sub";
    let version = format!("{sub}/version");
    let sub_type = envelope["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["identity"] == sub)
        .expect("Sub is declared");
    sub_type["fields"] = json!([{
        "identity": version,
        "name": "version",
        // The same bound `VersionNumber` (`Int[0, 1000]`) that
        // `ConfigVersion::versionNumber` declares, so this redefinition does
        // not widen it and the package admits. A native `Integer` here would
        // widen it and refuse `RedefinitionWidens` at assembly, which is
        // covered in `type_environment_model.rs`, not here.
        "typeRef": version_number_identity(),
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "redefines": config_version_identity("versionNumber"),
        "origin": {
            "generated": {
                "generatorIdentity": version.clone(),
                "generatorVersion": "1.0.0",
                "inputIdentities": [version],
            }
        },
    }]);
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// [`tc465_document`], with an extra object type `Sub` (`supertypes:
/// [ConfigVersion]`, no fields/operations of its own), declared as
/// `config_history`'s own second member type (row 39: `Sub` "is a member
/// type of `config_history`", `TC-465-admission-refuses-each-input-defect.md:67`
/// -- not merely covered by conformance, so check 6.1 never refuses a `Sub`
/// object as an undeclared member type).
fn tc465_document_with_sub_subtype() -> Vec<u8> {
    add_sub_type(
        tc465_document(),
        &["ix://example/config-version/config_history"],
    )
}

/// Appends object type `Sub` (`supertypes: [ConfigVersion]`, no fields or
/// operations of its own) to `document`, and declares it a member of every
/// population in `populations` (by identity).
fn add_sub_type(document: Vec<u8>, populations: &[&str]) -> Vec<u8> {
    let mut envelope: serde_json::Value = serde_json::from_slice(&document).expect("valid JSON");
    let config_version = "ix://example/config-version/ConfigVersion";
    let sub = "ix://example/config-version/Sub";
    envelope["types"].as_array_mut().unwrap().push(json!({
        "identity": sub,
        "displayName": sub,
        "kind": {"module": "example/config-version", "name": "object_type"},
        "roles": [],
        "origin": {
            "generated": {
                "generatorIdentity": sub,
                "generatorVersion": "1.0.0",
                "inputIdentities": [sub],
            }
        },
        "constraints": [],
        "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": [config_version],
        "fields": [],
        "operations": [],
    }));
    for population in envelope["populations"].as_array_mut().unwrap() {
        if populations.contains(&population["identity"].as_str().unwrap()) {
            population["members"]
                .as_array_mut()
                .unwrap()
                .push(json!(sub));
        }
    }
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// [`tc465_document`], with `Sub` (see [`add_sub_type`]) and a second,
/// unbounded population `archive` whose only declared member is `Sub`
/// (rows 40, 41). FR-104 (`FR-104-check-state-clauses.md:194-195,216-221`,
/// as amended): a population declaration never has a maximum,
/// so `archive`, like `config_history`, is simply an unbounded
/// `Population(None)` -- no wire field expresses a maximum at all.
/// `archive` never covers a clause on `ConfigVersion` itself: population
/// coverage is by conformance downward only (a clause on a *subtype*
/// reaches its supertype's population, never the reverse,
/// `population_coverage_is_by_conformance_and_absent_when_none_covers`),
/// so declaring `archive`'s member as `Sub` (not `ConfigVersion`) keeps
/// `config_history` the sole population `ConfigVersion`'s own clauses
/// resolve against -- no S3 `ambiguous_declaration`/`ambiguous-name`.
fn tc465_document_with_archive_population() -> Vec<u8> {
    with_archive_population(tc465_document())
}

/// `document` with `Sub` (see [`add_sub_type`]) and the unbounded
/// population `archive` whose only declared member is `Sub`
/// ([`tc465_document_with_archive_population`]).
fn with_archive_population(document: Vec<u8>) -> Vec<u8> {
    let document = add_sub_type(document, &[]);
    let mut envelope: serde_json::Value = serde_json::from_slice(&document).expect("valid JSON");
    let sub = "ix://example/config-version/Sub";
    let archive = "ix://example/config-version/archive";
    envelope["populations"].as_array_mut().unwrap().push(json!({
        "identity": archive,
        "displayName": archive,
        "kind": {"module": "example/config-version", "name": "population"},
        "members": [sub],
        "extent": "closed",
        "origin": {
            "generated": {
                "generatorIdentity": archive,
                "generatorVersion": "1.0.0",
                "inputIdentities": [archive],
            }
        },
    }));
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// [`tc465_document_with_archive_population`], with `attemptUpdate`
/// modifying `[versionNumber]` only (row 40: a `parent` change anywhere,
/// in any population, must be unauthorized).
fn tc465_document_with_archive_population_and_narrow_frame() -> Vec<u8> {
    let document = tc465_document_with_archive_population();
    let mut envelope: serde_json::Value = serde_json::from_slice(&document).expect("valid JSON");
    let config_version = "ix://example/config-version/ConfigVersion";
    envelope["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["identity"] == config_version)
        .expect("ConfigVersion is declared")["operations"][0]["frame"]["modifies"] =
        json!(["ix://example/config-version/ConfigVersion/versionNumber"]);
    serde_json::to_vec(&envelope).expect("valid JSON")
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
                 "fields": {"versionNumber": {"integer": "1"}, "parent": {"absent": {}}}},
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
                 "fields": {"versionNumber": {"integer": "2"}, "parent": {"absent": {}}}},
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
                 "fields": {"versionNumber": {"integer": "3"}, "parent": {"absent": {}}}},
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
/// `snapshots`/`invocations` -- TC-465's own harness, `ObservationLimits::
/// default()`.
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
    run_tc465_with_limits(
        document,
        clause_name,
        selection,
        snapshots,
        invocations,
        qsl_semantics::model::observation::ObservationLimits::default(),
    )
}

/// [`run_tc465`], with the `ObservationLimits` given explicitly (SR-750
/// FND-011's `objects_per_document`/`values_per_document` rows).
fn run_tc465_with_limits(
    document: &[u8],
    clause_name: &str,
    selection: qsl_semantics::model::observation::ClauseSelectionInput,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    invocations: BTreeMap<[u8; 32], Vec<u8>>,
    limits: qsl_semantics::model::observation::ObservationLimits,
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    run_tc465_with_clauses(
        document,
        TC465_CLAUSES,
        clause_name,
        selection,
        snapshots,
        invocations,
        limits,
    )
}

/// [`run_tc465_with_limits`] over the unit declaring `clauses`.
fn run_tc465_with_clauses(
    document: &[u8],
    clauses: &str,
    clause_name: &str,
    selection: qsl_semantics::model::observation::ClauseSelectionInput,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    invocations: BTreeMap<[u8; 32], Vec<u8>>,
    limits: qsl_semantics::model::observation::ObservationLimits,
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    run_tc465_with_clauses_under(
        document,
        clauses,
        clause_name,
        selection,
        snapshots,
        invocations,
        limits,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
}

/// [`run_tc465_with_clauses`] under the model normalization limits
/// `model_limits`, which also bound observation admission's conformance
/// walks.
#[allow(clippy::too_many_arguments)]
fn run_tc465_with_clauses_under(
    document: &[u8],
    clauses: &str,
    clause_name: &str,
    selection: qsl_semantics::model::observation::ClauseSelectionInput,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    invocations: BTreeMap<[u8; 32], Vec<u8>>,
    limits: qsl_semantics::model::observation::ObservationLimits,
    model_limits: qsl_semantics::model::accounting::ModelNormalizationLimits,
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    let packages = qsl_semantics::model::intake::package_input([document]);
    let declarations = admit_and_assemble_with_body(document, clauses)
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
    // Admission runs on a 512 KiB stack: reading a document never uses the
    // host stack in proportion to its depth (FR-261).
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn_scoped(scope, || {
                qsl_semantics::model::observation::admit_observations(
                    graph.model_selections(),
                    graph.scope().types(),
                    &clause_facts,
                    &packages,
                    model_limits,
                    &provisions,
                    &selection,
                    limits,
                )
            })
            .expect("spawn the admission thread")
            .join()
            .expect("admission does not panic")
    })
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

/// [`run_tc465_current`], with the `ObservationLimits` given explicitly
/// (SR-750 FND-011's `objects_per_document`/`values_per_document` rows).
fn run_tc465_current_with_limits(
    document: &[u8],
    mutate: impl FnOnce(&mut serde_json::Value),
    limits: qsl_semantics::model::observation::ObservationLimits,
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    let model_digest_hex = tc465_model_digest_hex(document);
    let label = frame_label("current-snap");
    let mut value = tc465_healthy_parent(&label, &model_digest_hex);
    mutate(&mut value);
    let bytes = value.to_string().into_bytes();
    let selected = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&bytes),
        ..label
    };
    let mut snapshots = BTreeMap::new();
    snapshots.insert(selected.digest, bytes);
    run_tc465_with_limits(
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
        limits,
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

/// The healthy-parent object population, observed `pre` or `post` (no
/// `anchor`, unlike [`tc465_healthy_parent`]'s `current` shape) -- rows
/// 35 to 38's own pre/post snapshot, since a `probe` invocation's pre and
/// post are checked as `pre`/`post` observations (check 3), never
/// `current`.
fn tc465_healthy_parent_observed(
    label: &qsl_semantics::model::observation::DocumentRef,
    model_digest_hex: &str,
    observation: &str,
) -> serde_json::Value {
    let population = "ix://example/config-version/config_history";
    let config_version = "ix://example/config-version/ConfigVersion";
    json!({
        "format": "quire.state.snapshot/v1",
        "identity": frame_identity_json(label),
        "observation": observation,
        "model": frame_model_header(model_digest_hex),
        "populations": [{
            "population": population,
            "complete": true,
            "objects": [
                {"key": "root", "type": config_version,
                 "fields": {"versionNumber": {"integer": "1"}, "parent": {"absent": {}}}},
                {"key": "child", "type": config_version,
                 "fields": {"versionNumber": {"integer": "2"},
                            "parent": {"present": {"reference": {"population": population, "key": "root"}}}}},
            ],
        }],
    })
}

/// TC-465's own `probe` invocation (rows 35-38): `operation` `probe`,
/// `self` `{config_history, child}`, `parameters` naming `root` as
/// `target`, `result` `null`, healthy-parent as both pre and post.
fn tc465_probe_invocation(
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
        "operation": "probe",
        "self": {"population": population, "key": "child"},
        "pre": {
            "identity": frame_identity_json(pre_ref),
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&pre_ref.digest)),
        },
        "post": {
            "identity": frame_identity_json(post_ref),
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&post_ref.digest)),
        },
        "parameters": {"target": {"reference": {"population": population, "key": "root"}}},
        "result": null,
        "created": [],
        "deleted": [],
    })
}

/// Runs `ReachesTarget` (precondition) over `document`'s `probe`
/// invocation, healthy-parent as both pre and post, `mutate_invocation`
/// applied before digesting -- TC-465 rows 35 to 38's own harness.
fn run_tc465_probe(
    document: &[u8],
    mutate_invocation: impl FnOnce(&mut serde_json::Value),
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    run_tc465_probe_with(document, "ReachesTarget", |_| {}, mutate_invocation)
}

/// [`run_tc465_probe`], with the pre snapshot's JSON mutated by
/// `mutate_pre` before it is digested.
fn run_tc465_probe_with(
    document: &[u8],
    clause: &str,
    mutate_pre: impl FnOnce(&mut serde_json::Value),
    mutate_invocation: impl FnOnce(&mut serde_json::Value),
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    let model_digest_hex = tc465_model_digest_hex(document);
    let pre_label = frame_label("pre-snap");
    let mut pre_value = tc465_healthy_parent_observed(&pre_label, &model_digest_hex, "pre");
    mutate_pre(&mut pre_value);
    let pre_bytes = pre_value.to_string().into_bytes();
    let pre = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&pre_bytes),
        ..pre_label
    };

    let post_label = frame_label("post-snap");
    let post_value = tc465_healthy_parent_observed(&post_label, &model_digest_hex, "post");
    let post_bytes = post_value.to_string().into_bytes();
    let post = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&post_bytes),
        ..post_label
    };

    let invocation_label = frame_label("invocation");
    let mut invocation_value =
        tc465_probe_invocation(&invocation_label, &model_digest_hex, &pre, &post);
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
        clause,
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
            assert_eq!(record.code.as_str(), expected_code);
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
            assert_eq!(record.code.as_str(), expected_code);
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
#[trace("TC-720", "FR-255-AC-1")]
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
    let record = assert_tc465_refused(result, "stage_limit_exceeded", "input-bytes-exceeded");
    assert_eq!(
        record.fields.get("setting").map(String::as_str),
        Some("observation.input_bytes")
    );
    assert_eq!(
        record.fields.get("bound").map(String::as_str),
        Some("1048576")
    );
}

/// FR-261 B4, FR-255: `objects_per_document` is charged as the reader walks
/// the document: the healthy-parent snapshot's two objects (`root`, `child`)
/// exceed a ceiling of 1 at the second, naming `observation.objects`, the
/// bound and the count reached; the builder raises it.
#[trace("TC-465", "FR-106-AC-3")]
#[trace("TC-720", "FR-255-AC-1")]
#[trace("TC-721", "FR-255-AC-4")]
#[test]
fn objects_per_document_limit_refuses_naming_observation_objects() {
    let document = tc465_document();
    let limits = qsl_semantics::model::observation::ObservationLimits::default()
        .with_objects_per_document(1);
    let result = run_tc465_current_with_limits(&document, |_| {}, limits);
    let record = assert_tc465_refused(result, "stage_limit_exceeded", "node-count-exceeded");
    assert_eq!(
        record.fields.get("setting").map(String::as_str),
        Some("observation.objects")
    );
    assert_eq!(record.fields.get("bound").map(String::as_str), Some("1"));
    assert_eq!(record.fields.get("actual").map(String::as_str), Some("2"));
    let raised = limits.with_objects_per_document(2);
    assert!(run_tc465_current_with_limits(&document, |_| {}, raised).is_ok());
}

/// FR-261 B4, FR-255: `values_per_document` counts every value form of the
/// document as the reader walks it: a ceiling of 0 refuses at the first,
/// naming `observation.values`.
#[trace("TC-465", "FR-106-AC-3")]
#[trace("TC-720", "FR-255-AC-1")]
#[test]
fn values_per_document_limit_refuses_naming_observation_values() {
    let document = tc465_document();
    let limits =
        qsl_semantics::model::observation::ObservationLimits::default().with_values_per_document(0);
    let result = run_tc465_current_with_limits(&document, |_| {}, limits);
    let record = assert_tc465_refused(result, "stage_limit_exceeded", "node-count-exceeded");
    assert_eq!(
        record.fields.get("setting").map(String::as_str),
        Some("observation.values")
    );
    assert_eq!(record.fields.get("bound").map(String::as_str), Some("0"));
    assert_eq!(record.fields.get("actual").map(String::as_str), Some("1"));
}

/// `run_tc465_current_with_limits` over snapshot text built by `build`
/// (which may nest to any depth, as `serde_json::Value` cannot), digested
/// under its own digest unless `selected` gives another.
fn run_tc465_current_text(
    document: &[u8],
    build: impl FnOnce(&str) -> String,
    selected: Option<[u8; 32]>,
    limits: qsl_semantics::model::observation::ObservationLimits,
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    let model_digest_hex = tc465_model_digest_hex(document);
    let label = frame_label("current-snap");
    let mut value = tc465_healthy_parent(&label, &model_digest_hex);
    value["populations"][0]["objects"][1]["fields"]["parent"] = json!("DEEP");
    let bytes = build(&value.to_string()).into_bytes();
    let selected = qsl_semantics::model::observation::DocumentRef {
        digest: selected.unwrap_or_else(|| frame_document_digest(&bytes)),
        ..label
    };
    let mut snapshots = BTreeMap::new();
    snapshots.insert(selected.digest, bytes);
    run_tc465_with_limits(
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
        limits,
    )
}

/// `"DEEP"` replaced by a `present` value nested `depth` levels.
fn nest_present(text: &str, depth: usize) -> String {
    let nested = format!(
        "{}{{\"absent\":{{}}}}{}",
        "{\"present\":".repeat(depth),
        "}".repeat(depth)
    );
    text.replacen("\"DEEP\"", &nested, 1)
}

/// FR-261-AC-2 (TC-733 step 2): on a 512 KiB stack (the harness admits on
/// one), digest admission over a snapshot holding a field value nested
/// 100,000 deep, within `observation.input_bytes`, reads the document's
/// digest and proceeds to FR-106's later checks -- here check 6's kind
/// judgement of the field -- and under another digest refuses
/// `stale_dependency`/`content-mismatch`.
#[trace("TC-733", "FR-261-AC-2")]
#[test]
fn a_snapshot_nested_100_000_deep_is_digested_on_content() {
    let document = tc465_document();
    let limits = qsl_semantics::model::observation::ObservationLimits::default()
        .with_document_bytes(4_000_000)
        .with_values_per_document(1_000_000);
    let own = run_tc465_current_text(&document, |text| nest_present(text, 100_000), None, limits);
    // The digest matched: admission went on to the field's own content
    // check, which refuses the nested value against the declared type.
    assert_tc465_refused(own, "invalid_runtime_input", "wrong-value-kind");
    let other = run_tc465_current_text(
        &document,
        |text| nest_present(text, 100_000),
        Some([7; 32]),
        limits,
    );
    assert_tc465_refused(other, "stale_dependency", "content-mismatch");
}

/// FR-261-AC-3 (TC-733 step 3): on a 512 KiB stack (the harness admits on
/// one), a field value nested 100,000 levels deep is read, counted as
/// `observation.values` as it is walked and judged on its content. With the
/// limits raised the field's declared type refuses it as `wrong-value-kind`
/// at that field. With `observation.values` at 50,000, which the nested
/// value alone crosses, it refuses naming `observation.values`, bound 50,000
/// and the count reached, 50,001.
#[trace("TC-733", "FR-261-AC-3")]
#[test]
fn a_value_nested_100_000_deep_is_counted_as_observation_values() {
    let document = tc465_document();
    let raised = qsl_semantics::model::observation::ObservationLimits::default()
        .with_document_bytes(4_000_000)
        .with_values_per_document(1_000_000);
    let counted = |limits| {
        run_tc465_current_text(&document, |text| nest_present(text, 100_000), None, limits)
    };
    // A bound of 50,000 is crossed by the nested value alone, at count 50,001.
    let refused = counted(raised.with_values_per_document(50_000));
    let record = assert_tc465_refused(refused, "stage_limit_exceeded", "node-count-exceeded");
    assert_eq!(
        record.fields.get("setting").map(String::as_str),
        Some("observation.values")
    );
    assert_eq!(
        record.fields.get("bound").map(String::as_str),
        Some("50000")
    );
    assert_eq!(
        record.fields.get("actual").map(String::as_str),
        Some("50001")
    );
    let judged = assert_tc465_refused(counted(raised), "invalid_runtime_input", "wrong-value-kind");
    assert_eq!(
        judged.fields.get("field").map(String::as_str),
        Some("parent")
    );
}

/// Row 4 (check 1.3): the snapshot's bytes edited, kept under the original
/// digest.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row4_edited_bytes_under_original_digest_refuses_content_mismatch() {
    let document = tc465_document();
    let model_digest_hex = tc465_model_digest_hex(&document);
    let label = frame_label("current-snap");
    let original = tc465_healthy_parent(&label, &model_digest_hex);
    let original_digest = frame_document_digest(&original.to_string().into_bytes());
    let mut edited = original;
    edited["populations"][0]["objects"][1]["fields"]["versionNumber"] = json!({"integer": "3"});
    let edited_digest = frame_document_digest(&edited.to_string().into_bytes());
    let result = run_tc465_current(
        &document,
        |value| *value = edited.clone(),
        Some(original_digest),
    );
    let record = assert_tc465_refused(result, "stale_dependency", "content-mismatch");
    assert_eq!(
        record.fields.get("selected"),
        Some(&qsl_semantics::model::key::hex(&original_digest))
    );
    assert_eq!(
        record.fields.get("recomputed"),
        Some(&qsl_semantics::model::key::hex(&edited_digest))
    );
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

/// SR-750 FND-015 round 2: a document that parses as JSON but isn't an
/// object holds no `format` member at all -- check 1.4 (format) comes
/// before check 1.5 (member presence) in FR-106's own order, so this
/// refuses `unknown_wire`/`unsupported-wire`, never 1.5's
/// `invalid_runtime_input`/`missing-member`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn a_non_object_document_refuses_unsupported_wire_before_missing_member() {
    let document = tc465_document();
    let result = run_tc465_current(&document, |value| *value = json!([1, 2, 3]), None);
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

/// SR-750 FND-004 round 2: `"model": "x"` (a string, not an object) refuses
/// `invalid_runtime_input`/`wrong-value-kind` at `model`, never
/// `AdmissionFailure::Fault` -- untrusted input, settled at admission.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn a_model_member_that_is_not_an_object_refuses_wrong_value_kind() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["model"] = json!("x");
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "wrong-value-kind");
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("model")
    );
}

/// SR-750 FND-004 round 2: an object keyed `""` refuses
/// `invalid_runtime_input`/`invalid-value` at `key`, never
/// `AdmissionFailure::Fault`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn an_empty_object_key_refuses_invalid_value() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"][0]["key"] = json!("");
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "invalid-value");
    assert_eq!(record.fields.get("field").map(String::as_str), Some("key"));
}

/// SR-750 FND-013 round 2: FR-106 line 107 spells the `absent` tag's
/// payload `{}`; a non-empty payload (`5`, not an object at all) is not
/// this shape and refuses `wrong-value-kind` at the field, rather than
/// being accepted the way any payload used to be.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn an_absent_tag_with_a_non_empty_payload_refuses_wrong_value_kind() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"][0]["fields"]["parent"] = json!({"absent": 5});
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "wrong-value-kind");
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("parent")
    );
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

/// SR-750 FND-005 round 2 (two defects): an object of an undeclared member
/// type (`Note`, check 6.1) *and* a duplicate key (of the population's
/// existing `root`, check 6.3) -- 6.1 must be named, not 6.3.
#[trace("TC-465", "FR-106-AC-7")]
#[test]
fn wrong_role_mapping_beats_a_duplicate_key_at_the_same_object() {
    let document = tc465_document_with_note_type();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "key": "root", "type": "ix://example/config-version/Note",
                    "fields": {},
                }));
        },
        None,
    );
    assert_tc465_refused(result, "invalid_runtime_input", "wrong-role-mapping");
}

/// SR-750 FND-005 round 2 (two defects): an undeclared field (`label`,
/// check 6.4) *and* a wrong-kind value on a declared field
/// (`versionNumber`, check 6.5) on the same object -- 6.4 must be named,
/// not 6.5.
#[trace("TC-465", "FR-106-AC-7")]
#[test]
fn unknown_member_beats_a_wrong_value_kind_at_the_same_object() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"][0]["fields"]["label"] = json!({"boolean": true});
            value["populations"][0]["objects"][0]["fields"]["versionNumber"] =
                json!({"boolean": true});
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "unknown-member");
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("label")
    );
}

/// SR-750 FND-017: check 6.4 lists a missing declared field before an
/// undeclared one, so an object with both defects (no `versionNumber`, an
/// extra `label`) refuses `missing-member` naming `versionNumber`.
#[trace("TC-465", "FR-106-AC-7")]
#[test]
fn a_missing_field_beats_an_unknown_field_at_the_same_object() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            let fields = value["populations"][0]["objects"][0]["fields"]
                .as_object_mut()
                .unwrap();
            fields.insert("label".to_owned(), json!({"boolean": true}));
            fields.remove("versionNumber");
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "missing-member");
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("versionNumber")
    );
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

/// Row 20 (check 6.5), part 1: `root.versionNumber` `"-1"`, below
/// `VersionNumber`'s bound `Int[0, 1000]`, refuses `invalid-value` naming
/// `root` and `versionNumber`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row20_root_version_number_below_bound_refuses_invalid_value() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"][0]["fields"]["versionNumber"] =
                json!({"integer": "-1"});
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "invalid-value");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("root")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("versionNumber")
    );
}

/// Row 20 (check 6.5), part 2: `child.versionNumber` `"1001"`, above
/// `VersionNumber`'s bound `Int[0, 1000]`, refuses `invalid-value` naming
/// `child` and `versionNumber`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row20_child_version_number_above_bound_refuses_invalid_value() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"][1]["fields"]["versionNumber"] =
                json!({"integer": "1001"});
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "invalid-value");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("child")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("versionNumber")
    );
}

/// Row 30 (check 6.5 walk order): both `root.versionNumber` `"-1"` and
/// `child.versionNumber` `"1001"` out of `VersionNumber`'s bound
/// `Int[0, 1000]` at once -- check 6.5 walks objects in declared order, so
/// only `root`'s violation is reported.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row30_both_out_of_bound_refuses_at_root_first() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"][0]["fields"]["versionNumber"] =
                json!({"integer": "-1"});
            value["populations"][0]["objects"][1]["fields"]["versionNumber"] =
                json!({"integer": "1001"});
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "invalid-value");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("root")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("versionNumber")
    );
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
/// admits (SR-751 FND-007 round 2: `archive` is `tc465_document_with_
/// archive_population`'s own genuine, declared population, not a bare
/// name pushed onto a package that never declares it; that undeclared
/// name previously made this row wrongly assert a `wrong-role-mapping`
/// refusal, when TC-465 states this row admits).
#[trace("TC-465", "FR-106-AC-4")]
#[test]
fn tc465_row25_an_incomplete_population_no_reference_names_still_admits() {
    let document = tc465_document_with_archive_population();
    let archive = "ix://example/config-version/archive";
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"].as_array_mut().unwrap().push(json!({
                "population": archive,
                "complete": false,
                "objects": [],
            }));
        },
        None,
    );
    result.expect("an incomplete population no reference names still admits");
}

/// The healthy-parent current snapshot with `child.parent` naming `{archive,
/// k}`, `k` a `Sub` object of the complete `archive`.
fn tc465_current_with_parent_in_archive(
    document: &[u8],
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    run_tc465_current(
        document,
        |value| {
            value["populations"][0]["objects"][1]["fields"]["parent"] = json!({"present": {
                "reference": {"population": "ix://example/config-version/archive", "key": "k"}
            }});
            value["populations"]
                .as_array_mut()
                .unwrap()
                .push(archive_with_sub_object());
        },
        None,
    )
}

/// SR-750 FND-016: a reference resolves against the population the wire
/// names, not only the field's declared type. `child.parent` names
/// `{archive, k}`; `archive` is complete and holds `k` of type `Sub`, and
/// `config_history` holds no `k`. `k` exists, so check 8 does not apply,
/// and `Sub` specializes `ConfigVersion`, so the `Reference<ConfigVersion>`
/// field admits it by conformance (QSpec FR-151, check 6.5): the admitted
/// `child.parent` is `k`, keeping its own type `Sub`.
#[trace("TC-464", "FR-106-AC-1")]
#[test]
fn a_field_reference_to_an_admitted_subtype_object_admits() {
    let document = tc465_document_with_archive_population();
    let observations = tc465_current_with_parent_in_archive(&document)
        .expect("a Sub object admits into a Reference<ConfigVersion> field");
    let graph = admit_and_assemble_with_body(&document, TC465_CLAUSES)
        .expect("the package assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    let types = graph.scope().types();
    let current = observations.current.expect("one current observation");
    let child = &observations.self_object;
    let parent = types
        .attribute(child.object_type(), "parent")
        .expect("ConfigVersion declares parent");
    let field = quire_semantic_value::declaration::FieldRef::new(parent.owner(), "parent");
    let Some(quire_exact::FieldValue::Present(quire_exact::Value::Reference(target))) = current
        .environment
        .objects()
        .attribute(types, child, &field)
    else {
        panic!("child.parent is a present reference");
    };
    assert_eq!(target.object().as_str(), "k");
    assert_eq!(
        current.environment.objects().find(target.universe(), "k"),
        Some(target),
        "child.parent is archive's k, typed by k's own type"
    );
    assert_ne!(
        target.object_type(),
        child.object_type(),
        "k keeps Sub, not the declared ConfigVersion"
    );
}

/// Check 6.5's refusal half for an object field: `child.parent` names `o`,
/// an admitted object of `Other`, a type with no declared-supertype chain to
/// `ConfigVersion`. Admission refuses `invalid_runtime_input`/
/// `wrong-value-kind` at `child`'s `parent`, never an internal fault.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn a_field_reference_to_an_unrelated_type_object_refuses_wrong_value_kind() {
    let document = with_unrelated_population(tc465_document());
    let others = "ix://example/config-version/others";
    let result = run_tc465_current(
        &document,
        move |value| {
            value["populations"][0]["objects"][1]["fields"]["parent"] =
                json!({"present": {"reference": {"population": others, "key": "o"}}});
            value["populations"].as_array_mut().unwrap().push(json!({
                "population": others,
                "complete": true,
                "objects": [{"key": "o", "type": "ix://example/config-version/Other", "fields": {}}],
            }));
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "wrong-value-kind");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("child")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("parent")
    );
}

/// SR-750 FND-019: only a population the snapshot lists as incomplete
/// tolerates a dangling reference. `child.parent` names `{bogus, missing}`,
/// a population the snapshot does not list, so the reference is not
/// tolerated and refuses `dangling_reference`.
#[trace("TC-465", "FR-106-AC-4")]
#[test]
fn a_reference_into_an_unlisted_population_refuses_dangling_reference() {
    let document = tc465_document();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"][1]["fields"]["parent"] =
                json!({"present": {"reference": {"population": "bogus", "key": "missing"}}});
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
}

/// FR-106 check 7's own last sentence ("admission SHALL skip the dangling
/// check over an incomplete population",
/// `FR-106-admit-snapshots-and-invocations.md:216`), SR-750 FND-001 round 2:
/// `archive` (a genuine second, declared population, `tc465_document_with_
/// archive_population`) is `complete: false` and holds `a1`, whose `parent`
/// names a key (`missing`) absent from `archive` itself -- nothing else
/// requires `archive`, so this must admit, not refuse `dangling_reference`.
/// `finish_populations` passes exactly this reference to
/// `ObjectClosure::new` as a tolerated dangling target.
#[trace("TC-465", "FR-106-AC-4")]
#[test]
fn a_dangling_reference_into_an_incomplete_population_still_admits() {
    let document = tc465_document_with_archive_population();
    let archive = "ix://example/config-version/archive";
    let sub = "ix://example/config-version/Sub";
    let result = run_tc465_current(
        &document,
        move |value| {
            value["populations"].as_array_mut().unwrap().push(json!({
                "population": archive,
                "complete": false,
                "objects": [{
                    "key": "a1", "type": sub,
                    "fields": {
                        "versionNumber": {"integer": "1"},
                        "parent": {"present": {"reference": {"population": archive, "key": "missing"}}},
                    },
                }],
            }));
        },
        None,
    );
    result.expect("a dangling reference into an incomplete population admits");
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

/// Row 27 (check 11.3): `attemptUpdate` modifying `[parent]` only, over
/// the changed-version invocation (post `child.versionNumber` 3) --
/// refuses naming `child` and `versionNumber`.
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn tc465_row27_change_outside_a_narrower_modifies_refuses_unauthorized_change() {
    let document = tc465_document_with_frame_modifies_parent_only();
    let result = run_tc465_invocation(&document, |_| {}, |_| {}, |_| {});
    let record = assert_tc465_refused(result, "frame_violation", "unauthorized-change");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("child")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("versionNumber")
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

/// Row 29 (check 1.3 over 1.6): row 4's edit plus row 7's extra member,
/// kept under the original digest -- `content-mismatch` wins (check
/// 1.3 runs before 1.6).
#[trace("TC-465", "FR-106-AC-7")]
#[test]
fn tc465_row29_a_digest_mismatch_beats_an_unknown_member() {
    let document = tc465_document();
    let model_digest_hex = tc465_model_digest_hex(&document);
    let label = frame_label("current-snap");
    let original = tc465_healthy_parent(&label, &model_digest_hex);
    let original_digest = frame_document_digest(&original.to_string().into_bytes());
    let mut edited = original;
    edited["populations"][0]["objects"][1]["fields"]["versionNumber"] = json!({"integer": "3"});
    edited["note"] = json!(true);
    let result = run_tc465_current(
        &document,
        |value| *value = edited.clone(),
        Some(original_digest),
    );
    assert_tc465_refused(result, "stale_dependency", "content-mismatch");
}

/// Row 31 (check 11.2 over 11.3): post deletes `root` and sets
/// `child.parent` absent -- the deletion (11.2) is named, not the field
/// change (11.3).
#[trace("TC-465", "FR-106-AC-7")]
#[test]
fn tc465_row31_an_unauthorized_deletion_beats_a_field_change_in_the_same_population() {
    // `attemptUpdate` modifies `[versionNumber]` only, so `child.parent`
    // changing below is a real 11.3 candidate; `deletes` stays empty, so
    // `root`'s deletion is a real 11.2 candidate in the same population.
    let document = tc465_document_with(attempt_update_modifies_version_number());
    let result = run_tc465_invocation(
        &document,
        |value| {
            // Pre gains `root` (absent from the default changed-version
            // pre snapshot): its deletion from post below is only a real
            // deletion, over a real pre-existing object, if pre actually
            // holds it.
            value["populations"][0]["objects"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "key": "root", "type": "ix://example/config-version/ConfigVersion",
                    "fields": {"versionNumber": {"integer": "1"}, "parent": {"absent": {}}},
                }));
        },
        |value| {
            // `root` stays deleted (post already omits it); `child.parent`
            // also changes outside `modifies` in the same population, to
            // a target that still exists in post (`child` itself, so this
            // stays a frame check, never a dangling-reference one) --
            // 11.2 (the deletion) must be named, not 11.3.
            value["populations"][0]["objects"][0]["fields"]["parent"] = json!({"present": {
                "reference": {"population": "ix://example/config-version/config_history", "key": "child"}
            }});
        },
        |_| {},
    );
    let record = assert_tc465_refused(result, "frame_violation", "unauthorized-change");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("root")
    );
}

/// Row 32 (check 6.1): a package variant that adds object type `Note`, a
/// member type of no population, and an object `n1` of type `Note` in
/// `config_history`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row32_an_object_of_an_undeclared_member_type_refuses_wrong_role_mapping() {
    let document = tc465_document_with_note_type();
    let result = run_tc465_current(
        &document,
        |value| {
            value["populations"][0]["objects"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "key": "n1",
                    "type": "ix://example/config-version/Note",
                    "fields": {},
                }));
        },
        None,
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "wrong-role-mapping");
    assert_eq!(record.fields.get("object").map(String::as_str), Some("n1"));
}

/// FR-106 check 6.1 and the model views admission re-derives bound their
/// conformance walks by the caller's `ancestor_steps`, with no fixed ceiling
/// of their own. An object whose type sits 70 edges below its population's
/// member type is admitted under the default `model.ancestor_steps` (a fixed
/// 64 refused it as `wrong-role-mapping`), and under `ancestor_steps` 60 the
/// same document stops with `stage_limit_exceeded` naming
/// `model.ancestor_steps` and its bound, never `wrong-role-mapping`.
#[trace("TC-465", "FR-106-AC-3")]
#[trace("TC-720", "FR-255-AC-1")]
#[test]
fn a_deep_hierarchy_is_judged_by_the_callers_ancestor_steps_not_a_fixed_ceiling() {
    const DEPTH: usize = 70;
    let config_version = "ix://example/config-version/ConfigVersion";
    let type_name = |level: usize| format!("ix://example/config-version/Deep{level}");
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&tc465_document()).expect("valid JSON");
    for level in 1..=DEPTH {
        let identity = type_name(level);
        let parent = if level == 1 {
            config_version.to_owned()
        } else {
            type_name(level - 1)
        };
        envelope["types"].as_array_mut().unwrap().push(json!({
            "identity": identity,
            "displayName": identity,
            "kind": {"module": "example/config-version", "name": "object_type"},
            "roles": [],
            "origin": {
                "generated": {
                    "generatorIdentity": identity,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [identity],
                }
            },
            "constraints": [],
            "extensions": [],
            "unknownPolicy": "reject",
            "supertypes": [parent],
            "fields": [],
            "operations": [],
        }));
    }
    let document = serde_json::to_vec(&envelope).expect("valid JSON");
    let run = |model_limits: qsl_semantics::model::accounting::ModelNormalizationLimits| {
        let model_digest_hex = tc465_model_digest_hex(&document);
        let label = frame_label("current-snap");
        let mut value = tc465_healthy_parent(&label, &model_digest_hex);
        value["populations"][0]["objects"][1]["type"] = json!(type_name(DEPTH));
        let bytes = value.to_string().into_bytes();
        let selected = qsl_semantics::model::observation::DocumentRef {
            digest: frame_document_digest(&bytes),
            ..label
        };
        let mut snapshots = BTreeMap::new();
        snapshots.insert(selected.digest, bytes);
        run_tc465_with_clauses_under(
            &document,
            TC465_CLAUSES,
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
            qsl_semantics::model::observation::ObservationLimits::default(),
            model_limits,
        )
    };
    run(qsl_semantics::model::accounting::ModelNormalizationLimits::default())
        .expect("a 70-edge hierarchy is admitted under the default ancestor_steps");
    let tight = qsl_semantics::model::accounting::ModelNormalizationLimits {
        ancestor_steps: 60,
        ..qsl_semantics::model::accounting::ModelNormalizationLimits::default()
    };
    let record = assert_tc465_refused(run(tight), "stage_limit_exceeded", "edge-count-exceeded");
    assert_eq!(
        record.fields.get("setting").map(String::as_str),
        Some("model.ancestor_steps")
    );
    assert_eq!(record.fields.get("bound").map(String::as_str), Some("60"));
}

/// Row 39 (check 11.1 over 11.2): `Sub` specializes `ConfigVersion` and is
/// a member type of `config_history`; post changes `child`'s type to `Sub`,
/// sets `child.parent` absent (already true of the default post) and
/// deletes `root` (added to pre, already absent from the default post) --
/// the retype (11.1) must be named, not `root`'s deletion (11.2)
/// (`resolve_self`, `document.rs`, now resolves by conformance per
/// QSL-277's membership ruling, so both pre's `child`/`ConfigVersion` and
/// post's `child`/`Sub` resolve at check 9, reaching check 11's own
/// exact-type retype refusal, `frame.rs`'s module doc, 11.1).
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row39_a_retype_beats_an_unauthorized_deletion() {
    let document = tc465_document_with_sub_subtype();
    let result = run_tc465_invocation(
        &document,
        |value| {
            // Pre gains `root`, so its absence from post below is a real
            // deletion, over a real pre-existing object.
            value["populations"][0]["objects"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "key": "root", "type": "ix://example/config-version/ConfigVersion",
                    "fields": {"versionNumber": {"integer": "1"}, "parent": {"absent": {}}},
                }));
        },
        |value| {
            // `child` retypes to `Sub`; `root` stays deleted (the default
            // post snapshot already omits it).
            value["populations"][0]["objects"][0]["type"] =
                json!("ix://example/config-version/Sub");
        },
        |_| {},
    );
    let record = assert_tc465_refused(result, "frame_violation", "unauthorized-change");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("child"),
        "the retype (11.1) is named, not root's deletion (11.2)"
    );
}

/// Row 40 (check 11, population order): pre and post list `archive` before
/// `config_history`; post sets `a1.parent` (in `archive`) to a present
/// reference (`config_history`'s `child`; see the substitution note below)
/// and `child.parent` (in `config_history`) absent -- `archive` is checked
/// to completion (11.1 to 11.4) before `config_history` even starts, so
/// `a1`'s unauthorized `parent` change refuses first (`attemptUpdate` here
/// modifies `versionNumber` only, so neither population's `parent` change
/// is ever authorized).
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn tc465_row40_population_order_checks_archive_before_config_history() {
    let document = tc465_document_with_archive_population_and_narrow_frame();
    let archive = "ix://example/config-version/archive";
    let sub = "ix://example/config-version/Sub";
    let a1_absent = json!({
        "population": archive,
        "complete": true,
        "objects": [{
            "key": "a1", "type": sub,
            "fields": {"versionNumber": {"integer": "1"}, "parent": {"absent": {}}},
        }],
    });
    // `a1.parent` targets `config_history`'s `child` (exact type
    // `ConfigVersion`), not `a1` itself: a `Reference<ConfigVersion>`
    // field's target is admitted under the field's own declared type, by
    // design (`document.rs`'s `admit_scalar`, the `Reference` arm --
    // "matching `crate::model::normalize` exactly", not the referenced
    // object's own possibly-subtyped admitted type), so a self-reference
    // from a `Sub`-typed `a1` would never resolve regardless of frame
    // authorization -- this substitutes a real, resolvable cross-population
    // target while still exercising the same unauthorized field change on
    // `a1` in `archive`.
    let a1_present = json!({
        "population": archive,
        "complete": true,
        "objects": [{
            "key": "a1", "type": sub,
            "fields": {
                "versionNumber": {"integer": "1"},
                "parent": {"present": {"reference": {
                    "population": "ix://example/config-version/config_history", "key": "child"
                }}},
            },
        }],
    });
    let result = run_tc465_invocation(
        &document,
        move |value| {
            // `archive` is listed before `config_history`.
            let config_history_entry = value["populations"][0].clone();
            value["populations"] = json!([a1_absent, config_history_entry]);
        },
        move |value| {
            let config_history_entry = value["populations"][0].clone();
            value["populations"] = json!([a1_present, config_history_entry]);
        },
        |_| {},
    );
    let record = assert_tc465_refused(result, "frame_violation", "unauthorized-change");
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some(archive)
    );
    assert_eq!(record.fields.get("object").map(String::as_str), Some("a1"));
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("parent")
    );
}

/// Row 41 (check 11, one-sided population): post adds population `archive`
/// (absent from pre), holding object `a1` (`parent` absent), with
/// `created` unchanged (empty) -- `archive`'s pre side admits as empty
/// (SR-750's own population-order rule, `frame.rs`'s `enforce`: "any
/// population only in the post snapshot" is appended to `order` after
/// every pre population), so `a1` is a post-only object no `creates`
/// grant or declared `created` entry authorizes.
#[trace("TC-465", "FR-106-AC-5")]
#[test]
fn tc465_row41_a_population_only_in_post_admits_its_pre_side_as_empty() {
    let document = tc465_document_with_archive_population();
    let archive = "ix://example/config-version/archive";
    let sub = "ix://example/config-version/Sub";
    let result = run_tc465_invocation(
        &document,
        |_| {},
        move |value| {
            value["populations"].as_array_mut().unwrap().push(json!({
                "population": archive,
                "complete": true,
                "objects": [{
                    "key": "a1", "type": sub,
                    "fields": {"versionNumber": {"integer": "1"}, "parent": {"absent": {}}},
                }],
            }));
        },
        |_| {},
    );
    let record = assert_tc465_refused(result, "frame_violation", "unauthorized-change");
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some(archive)
    );
    assert_eq!(record.fields.get("object").map(String::as_str), Some("a1"));
}

/// Row 17 (check 6.2): a package variant whose `ConfigVersion` has field
/// `tags` typed a set of `ConfigVersion`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row17_set_typed_field_refuses_unsupported_feature() {
    let document = tc465_document_with_set_field();
    let result = run_tc465_current(&document, |_| {}, None);
    let record = assert_tc465_refused(result, "unknown_required_feature", "unsupported-feature");
    assert_eq!(record.fields.get("field").map(String::as_str), Some("tags"));
}

/// Row 35 (check 10): probe invocation with `parameters` `{}` (missing
/// `target`).
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row35_missing_parameter_refuses_missing_member() {
    let document = tc465_document();
    let result = run_tc465_probe(&document, |value| value["parameters"] = json!({}));
    let record = assert_tc465_refused(result, "invalid_runtime_input", "missing-member");
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("target")
    );
}

/// Row 36 (check 10): probe invocation with an extra parameter `other`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row36_unknown_parameter_refuses_unknown_member() {
    let document = tc465_document();
    let result = run_tc465_probe(&document, |value| {
        value["parameters"]["other"] = json!({
            "reference": {"population": "ix://example/config-version/config_history", "key": "root"}
        });
    });
    let record = assert_tc465_refused(result, "invalid_runtime_input", "unknown-member");
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("other")
    );
}

/// Row 37 (check 10): probe invocation with `target` `{"integer": "1"}`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row37_wrong_parameter_kind_refuses_wrong_value_kind() {
    let document = tc465_document();
    let result = run_tc465_probe(&document, |value| {
        value["parameters"]["target"] = json!({"integer": "1"});
    });
    assert_tc465_refused(result, "invalid_runtime_input", "wrong-value-kind");
}

/// Row 38 (check 10): probe invocation with `result` `{"boolean": true}`
/// for an operation with no declared result. Check 10's result rule is a
/// postcondition's (a precondition reads no result, FR-106 "Behavior"), so
/// it runs for `probe`'s postcondition `ProbeHolds`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn tc465_row38_result_for_a_no_result_operation_refuses_unknown_member() {
    let document = tc465_document();
    let result = run_tc465_probe_with(
        &document,
        "ProbeHolds",
        |_| {},
        |value| value["result"] = json!({"boolean": true}),
    );
    assert_tc465_refused(result, "invalid_runtime_input", "unknown-member");
}

/// Row 43 (check 8 over a check-10 value; SR-771 FND-001): probe
/// invocation with `target` naming `missing`, a key absent from the
/// complete pre `config_history` -- `dangling_reference`, naming `missing`
/// and `config_history`, exactly as row 24 does for an object field. Before
/// the fix this admitted and `ReachesTarget` evaluated `false`.
#[trace("TC-465", "FR-106-AC-4")]
#[test]
fn tc465_row43_dangling_reference_parameter_refuses_dangling_reference() {
    let document = tc465_document();
    let population = "ix://example/config-version/config_history";
    let result = run_tc465_probe(&document, |value| {
        value["parameters"]["target"] =
            json!({"reference": {"population": population, "key": "missing"}});
    });
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
        Some(population)
    );
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

/// The precondition `AttemptUpdatePre` over the authorized-change
/// invocation, with the invocation's `member` replaced by the JSON text
/// `raw`.
fn run_precondition_with_member(
    member: &str,
    raw: &str,
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    let pre = [("root", 1, None)];
    let post = [("root", 2, None)];
    run_frame_clause_editing("AttemptUpdatePre", &pre, &post, "root", &[], &[], |bytes| {
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value[member] = json!("@raw@");
        value.to_string().replace("\"@raw@\"", raw).into_bytes()
    })
}

/// FR-106-AC-11: a precondition reads no further than check 1.3 into an
/// invocation's `member`, yet a number there with no exact RFC 8785
/// spelling refuses `noncanonical_wire` with `cause` and `pointer`, before
/// the digest is compared; the same member holding `exact` admits.
fn assert_member_number_refused(member: &str, raw: &str, exact: &str, cause: &str, pointer: &str) {
    match run_precondition_with_member(member, raw) {
        Err(qsl_semantics::model::observation::AdmissionFailure::Refused(record)) => {
            assert_eq!(record.code, Code::NoncanonicalWire, "{member}");
            assert_eq!(record.cause, cause, "{member}");
            assert_eq!(
                record.fields.get("document_pointer").map(String::as_str),
                Some(pointer),
                "{member}"
            );
        }
        other => panic!("{member}: expected Refused(noncanonical_wire/{cause}), got {other:?}"),
    }
    run_precondition_with_member(member, exact)
        .unwrap_or_else(|failure| panic!("{member} holding {exact}: {failure:?}"));
}

#[trace("TC-465", "FR-106-AC-11")]
#[test]
fn an_inexact_number_in_an_unread_post_member_refuses() {
    assert_member_number_refused("post", "9007199254740993", "2", "inexact-integer", "/post");
}

#[trace("TC-465", "FR-106-AC-11")]
#[test]
fn an_inexact_number_in_an_unread_result_member_refuses() {
    assert_member_number_refused(
        "result",
        "0.1000000000000000000001",
        "0.1",
        "inexact-number",
        "/result",
    );
}

#[trace("TC-465", "FR-106-AC-11")]
#[test]
fn an_inexact_number_in_an_unread_created_member_refuses() {
    assert_member_number_refused(
        "created",
        "[0,1e-400]",
        "[0,1e-300]",
        "inexact-number",
        "/created/1",
    );
}

#[trace("TC-465", "FR-106-AC-11")]
#[test]
fn an_inexact_number_in_an_unread_deleted_member_refuses() {
    assert_member_number_refused(
        "deleted",
        r#"[{"a/b":18446744073709551616}]"#,
        r#"[{"a/b":9007199254740992}]"#,
        "inexact-integer",
        "/deleted/0/a~1b",
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

/// A precondition does not require `self` in the post snapshot, nor run
/// check 11: `root` (self) is absent from post and declared deleted, which
/// `attemptUpdate`'s frame (modifies `[versionNumber]` only) does not
/// grant, yet the precondition admits the pre side alone.
#[trace("TC-464", "FR-106-AC-9")]
#[test]
fn precondition_does_not_require_self_in_the_post_snapshot() {
    let pre = [("root", 1, None), ("child", 1, None)];
    let post = [("child", 1, None)];
    let observations = run_frame_clause("AttemptUpdatePre", &pre, &post, "root", &[], &["root"])
        .expect("a precondition reads no post side");
    assert!(observations.post.is_none());
    assert!(observations.deleted.is_empty());
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

// ---------------------------------------------------------------------------
// FR-106 check 6.5 (FR-106-AC-12, FR-106-AC-13): inputs are refused, outputs
// are evidence.
// ---------------------------------------------------------------------------

fn witness_limits() -> qsl_semantics::model::observation::ObservationLimits {
    qsl_semantics::model::observation::ObservationLimits {
        post_state: qsl_semantics::model::observation::PostStateRange::Witness,
        ..qsl_semantics::model::observation::ObservationLimits::default()
    }
}

/// The wrapping debit: `child.versionNumber` 0 to -1 under `AttemptUpdatePost`.
fn run_wrapping_debit(
    limits: qsl_semantics::model::observation::ObservationLimits,
) -> Result<
    qsl_semantics::model::observation::AdmittedObservations,
    qsl_semantics::model::observation::AdmissionFailure,
> {
    run_frame_clause_limited(
        "AttemptUpdatePost",
        &[("root", 1, None), ("child", 0, Some("root"))],
        &[("root", 1, None), ("child", -1, Some("root"))],
        "child",
        &[],
        &[],
        |bytes| bytes,
        limits,
    )
}

/// FR-106-AC-12: under `Witness` the wrapping debit's post snapshot admits
/// and reports the exact -1 with its object, field and declared range.
#[trace("TC-465", "FR-106-AC-12")]
#[test]
fn a_witnessed_post_state_value_is_admitted_and_reported_exactly() {
    let admitted = run_wrapping_debit(witness_limits()).expect("the post snapshot admits");
    let post = admitted
        .post
        .expect("a postcondition admits a post snapshot");
    let [violation] = &post.out_of_range[..] else {
        panic!("expected one range violation, got {:?}", post.out_of_range);
    };
    assert_eq!(violation.object.object().as_str(), "child");
    assert_eq!(violation.field.name, "versionNumber");
    assert_eq!(violation.index, None);
    assert_eq!(violation.range.lower().to_string(), "0");
    assert_eq!(violation.range.upper().to_string(), "1000");
    assert_eq!(violation.observed.to_string(), "-1");
    let pre = admitted.pre.expect("a postcondition admits a pre snapshot");
    assert!(pre.out_of_range.is_empty());
}

/// FR-106-AC-13: the default limits refuse the same post snapshot.
#[trace("TC-465", "FR-106-AC-13")]
#[test]
fn the_default_limits_refuse_an_out_of_range_post_state_value() {
    let result =
        run_wrapping_debit(qsl_semantics::model::observation::ObservationLimits::default());
    let record = assert_tc465_refused(result, "invalid_runtime_input", "invalid-value");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("child")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("versionNumber")
    );
}

/// FR-106-AC-12: witnessing never reaches the pre snapshot.
#[trace("TC-465", "FR-106-AC-12")]
#[test]
fn an_out_of_range_pre_state_value_refuses_under_witnessing() {
    let result = run_frame_clause_limited(
        "AttemptUpdatePost",
        &[("root", 1, None), ("child", 1001, Some("root"))],
        &[("root", 1, None), ("child", 2, Some("root"))],
        "child",
        &[],
        &[],
        |bytes| bytes,
        witness_limits(),
    );
    let record = assert_tc465_refused(result, "invalid_runtime_input", "invalid-value");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("child")
    );
}

/// FR-106-AC-12: an out-of-range operation argument refuses
/// `invalid_runtime_input`/`invalid-value` naming the parameter, under the
/// default limits and under `Witness`, while `1000` admits.
#[trace("TC-465", "FR-106-AC-12")]
#[test]
fn an_out_of_range_argument_refuses_whatever_the_limits() {
    let probe_int = operation(
        "probeInt",
        json!([operation_parameter(
            "probeInt",
            "amount",
            &version_number_identity()
        )]),
        None,
        empty_frame(),
    );
    let document = with_archive_population(config_version_document_with_operations(vec![
        attempt_update_modifies_version_and_parent(),
        probe_operation(),
        probe_int,
    ]));
    let clauses = format!(
        "{TC465_CLAUSES}pre AmountHolds using v on Config::ConfigVersion::probeInt {{ true }}\n"
    );
    let snapshot_bytes = tc464_chain(&document, "pre", true);
    let snapshot = qsl_semantics::model::observation::DocumentRef {
        digest: frame_document_digest(&snapshot_bytes),
        ..frame_label("pre-call-snap")
    };
    let run = |amount: &str, limits| {
        run_tc465_with_clauses(
            &document,
            &clauses,
            "AmountHolds",
            qsl_semantics::model::observation::ClauseSelectionInput::PreCall {
                snapshot: snapshot.clone(),
                self_object: qsl_semantics::model::observation::SelectedObject {
                    population: TC464_CONFIG_HISTORY.to_owned(),
                    key: "a".to_owned(),
                },
                parameters: BTreeMap::from([(
                    quire_exact::Identifier::new("amount").unwrap(),
                    qsl_semantics::model::observation::SnapshotValue::integer(amount),
                )]),
            },
            BTreeMap::from([(snapshot.digest, snapshot_bytes.clone())]),
            BTreeMap::new(),
            limits,
        )
    };
    for limits in [
        qsl_semantics::model::observation::ObservationLimits::default(),
        witness_limits(),
    ] {
        let record =
            assert_tc465_refused(run("-1", limits), "invalid_runtime_input", "invalid-value");
        assert_eq!(
            record.fields.get("field").map(String::as_str),
            Some("amount")
        );
        assert!(run("1000", limits).is_ok());
    }
}
