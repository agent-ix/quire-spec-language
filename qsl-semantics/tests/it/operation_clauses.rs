// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-103: a domain-package operation's kept `requires` and `ensures` are
//! checked as Boolean clauses (TC-993) over the ConfigVersion package.

use ix_trace_rs::trace;
use qsl_semantics::check::{CheckCause, CheckRefusal, WrongSnapshotCause};
use qsl_semantics::model::refusal::{MemberPath, MemberRoot};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::location::{OperationClauseKind, Origin};
use serde_json::json;

use super::model_operations::{
    admit_and_assemble_with_body, config_version_document_with_operations, empty_frame, operation,
    PACKAGE_IDENTITY,
};

/// The one function the unit declares: a unit needs a declaration, and no
/// unit state clause names the operation.
const UNIT_BODY: &str = "function same using v(a: Boolean): Boolean pure { a }";

/// Checks a package whose `bump` operation holds `requires` and `ensures`
/// (`language`, `text`) and no unit clause.
fn check(
    requires: &[(&str, &str)],
    ensures: &[(&str, &str)],
) -> Result<qsl_semantics::check::CheckedGraph, Vec<CheckRefusal>> {
    let mut bump = operation("bump", json!([]), None, empty_frame());
    let clauses = |entries: &[(&str, &str)]| {
        entries
            .iter()
            .map(|(language, text)| {
                json!({
                    "language": language,
                    "text": text,
                    "origin": bump["origin"].clone(),
                })
            })
            .collect::<Vec<_>>()
    };
    let (pre, post) = (clauses(requires), clauses(ensures));
    bump["pre"] = json!(pre);
    bump["post"] = json!(post);
    let document = config_version_document_with_operations(vec![bump]);
    admit_and_assemble_with_body(&document, UNIT_BODY)
        .expect("the package assembles")
        .check(CheckingLimits::default())
}

fn bump_node() -> String {
    format!("ix://{PACKAGE_IDENTITY}/ConfigVersion/bump")
}

/// Clauses that check yield a package, add no state clause, and a clause in
/// another language is kept and neither parsed nor checked.
#[trace("FR-103-AC-7", "FR-103-AC-8", "TC-993")]
#[test]
fn kept_operation_clauses_that_check_add_no_state_clause() {
    let graph = check(
        &[
            ("quire", "self.versionNumber >= 0"),
            ("acme:rules", "not an expression ((("),
        ],
        &[("quire", "self.versionNumber = pre(self.versionNumber)")],
    )
    .expect("a Boolean requires and an ensures that reads pre check");
    assert!(
        graph.state_clauses().is_empty(),
        "a kept clause adds no checked state clause"
    );
}

/// A `requires` that reads `pre(e)` refuses `wrong_snapshot`/
/// `forbidden-pre-read`, located at `Operation.pre[k]` and naming the
/// operation.
#[trace("FR-103-AC-8", "TC-993")]
#[test]
fn a_requires_that_reads_pre_refuses_at_its_position() {
    let refusals = check(
        &[
            ("quire", "self.versionNumber >= 0"),
            ("quire", "pre(self.versionNumber) = self.versionNumber"),
        ],
        &[],
    )
    .expect_err("the second requires reads pre");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, got {refusals:?}");
    };
    assert!(matches!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    ));
    assert_eq!(refusal.operation_node(), Some(bump_node().as_str()));
    assert_eq!(
        refusal.operation_member(),
        Some(MemberPath::new(MemberRoot::Operation).name("pre").index(1))
    );
    assert!(matches!(
        &refusal.location.origin,
        Origin::ModelOperationClause {
            kind: OperationClauseKind::Requires,
            position: 1,
            ..
        }
    ));
}

/// An `ensures` that does not check is located at `Operation.post[k]`.
#[trace("FR-103-AC-8", "TC-993")]
#[test]
fn an_ensures_that_is_not_boolean_refuses_at_its_position() {
    let refusals =
        check(&[], &[("quire", "self.versionNumber")]).expect_err("an ensures must be Boolean");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, got {refusals:?}");
    };
    assert_eq!(refusal.operation_node(), Some(bump_node().as_str()));
    assert_eq!(
        refusal.operation_member(),
        Some(MemberPath::new(MemberRoot::Operation).name("post").index(0))
    );
}

/// A kept clause names no `using` profile, so the unit's selected
/// complete-value profile plays no part: integer `div` refuses
/// `profile_prohibited_operator`.
#[trace("FR-103-AC-8", "TC-993")]
#[test]
fn a_kept_clause_is_checked_under_the_profile_independent_defaults() {
    let refusals = check(&[("quire", "self.versionNumber div 2 = 0")], &[])
        .expect_err("integer div is profile prohibited in a kept clause");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, got {refusals:?}");
    };
    assert!(matches!(
        refusal.cause,
        CheckCause::ProfileProhibitedOperator(_)
    ));
    assert_eq!(
        refusal.operation_member(),
        Some(MemberPath::new(MemberRoot::Operation).name("pre").index(0))
    );
}
