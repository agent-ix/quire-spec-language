// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-249 (QSpec FR-419): S3 checks the `release S.t on M::o;`
//! declarations of a unit against its task sets and the operations of its
//! models, and records each on the checked task sets. The lowering itself
//! (FR-249-AC-1 and AC-2) is not under test here. Run over TC-458's
//! `ConfigVersion` fixture package with a second operation, `command`
//! (`crate::model_operations`).

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{CheckCause, CheckRefusal, CheckedGraph, TemporalCause, TimedConstruct};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_profiles, ambiguous_operation_document,
    attempt_update_modifies_version_number, config_unit_with_profiles,
    config_version_document_with_operations, empty_frame, operation,
};

const PROFILES: &str = "profile t = \"quire.temporal.complete/v1\";\n";

const SET: &str = "dimension Time;\n
     unit s : Time = rational(1, 1);\n
     unit ms : Time = rational(1, 1000) * s;\n
     taskset Ctl {\n
     task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n
     task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n
     }\n";

fn document() -> Vec<u8> {
    config_version_document_with_operations(vec![
        attempt_update_modifies_version_number(),
        operation("command", json!([]), None, empty_frame()),
    ])
}

fn unit_text(declarations: &str) -> String {
    format!("{SET}{declarations}\n")
}

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    admit_and_assemble_with_profiles(&document(), PROFILES, &unit_text(declarations))
        .expect("S2 keeps the release declarations")
        .check(CheckingLimits::default())
}

fn checked(declarations: &str) -> CheckedGraph {
    check(declarations).unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"))
}

/// The code, tag and span text of each refusal of `declarations`.
fn refused(declarations: &str) -> Vec<(Code, &'static str, String)> {
    let text = config_unit_with_profiles(&document(), PROFILES, &unit_text(declarations)).0;
    check(declarations)
        .expect_err("S3 refuses")
        .iter()
        .map(|refusal| {
            let CheckCause::Temporal(cause) = &refusal.cause else {
                panic!("a temporal refusal: {refusal:?}");
            };
            let span = cause.span();
            (
                cause.code(),
                cause.tag(),
                text[span.start..span.end].to_owned(),
            )
        })
        .collect()
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn a_release_is_recorded_on_its_operation_and_a_second_task_is_allowed() {
    let graph = checked(
        "release Ctl.a on ConfigVersion::attemptUpdate;\n
         release Ctl.b on ConfigVersion::attemptUpdate;",
    );
    let sets = graph.task_sets();
    let [first, second] = sets.releases() else {
        panic!("two releases: {:?}", sets.releases());
    };
    assert_eq!((first.task_set(), first.task()), ("Ctl", "a"));
    assert_eq!((second.task_set(), second.task()), ("Ctl", "b"));
    assert_eq!(first.operation().name(), "attemptUpdate");
    let released: Vec<_> = sets
        .released_by(first.operation().key())
        .map(|release| release.task())
        .collect();
    assert_eq!(released, ["a", "b"]);
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn an_operation_that_releases_nothing_records_no_release() {
    let graph = checked("release Ctl.a on ConfigVersion::attemptUpdate;");
    let [release] = graph.task_sets().releases() else {
        panic!("one release");
    };
    let other = check("release Ctl.a on ConfigVersion::command;")
        .expect("command releases a")
        .task_sets()
        .releases()
        .first()
        .map(|release| release.operation().key().clone())
        .expect("a release");
    assert_ne!(&other, release.operation().key());
    assert_eq!(graph.task_sets().released_by(&other).count(), 0);
    // A unit with a task set and no release records none.
    let bare = check("").expect("a unit with a task set and no release checks");
    assert!(bare.task_sets().releases().is_empty());
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn a_second_release_of_one_task_on_one_operation_refuses_at_the_second() {
    let reported = refused(
        "release Ctl.a on ConfigVersion::attemptUpdate;\n
         release Ctl.a on ConfigVersion::attemptUpdate;",
    );
    assert_eq!(
        reported,
        [(
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "release Ctl.a on ConfigVersion::attemptUpdate;".to_owned()
        )]
    );
    // The same task on another operation is not a duplicate.
    checked(
        "release Ctl.a on ConfigVersion::attemptUpdate;\n
         release Ctl.a on ConfigVersion::command;",
    );
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn an_unresolved_task_set_task_or_operation_refuses_at_the_name() {
    for (declaration, at) in [
        ("release Nope.a on ConfigVersion::attemptUpdate;", "Nope"),
        ("release Ctl.z on ConfigVersion::attemptUpdate;", "z"),
        (
            "release Ctl.a on ConfigVersion::nope;",
            "ConfigVersion::nope",
        ),
    ] {
        assert_eq!(
            refused(declaration),
            [(Code::MissingDeclaration, "missing-name", at.to_owned())],
            "{declaration}"
        );
    }
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn a_task_of_another_task_set_is_not_a_task_of_the_named_one() {
    let other = "taskset Other {\n
         task z periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n
         }\n";
    let declarations = format!("{other}release Ctl.z on ConfigVersion::attemptUpdate;");
    assert_eq!(
        refused(&declarations),
        [(Code::MissingDeclaration, "missing-name", "z".to_owned())]
    );
    checked(&format!(
        "{other}release Other.z on ConfigVersion::attemptUpdate;"
    ));
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn every_unresolved_name_of_a_release_is_reported() {
    let reported = refused("release Nope.a on ConfigVersion::nope;");
    assert_eq!(
        reported,
        [
            (Code::MissingDeclaration, "missing-name", "Nope".to_owned()),
            (
                Code::MissingDeclaration,
                "missing-name",
                "ConfigVersion::nope".to_owned()
            ),
        ]
    );
}

#[trace("TC-704", "FR-230-AC-7")]
#[test]
fn a_release_is_refused_unless_the_timed_facet_is_selected() {
    let text = format!("{SET}release Ctl.a on ConfigVersion::attemptUpdate;\n");
    let refusals = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.value.complete/v1\";\n",
        &text,
    )
    .expect("S2 keeps the forms")
    .check(CheckingLimits::default())
    .expect_err("the facet gate refuses");
    let found = refusals.iter().any(|refusal| {
        matches!(
            &refusal.cause,
            CheckCause::Temporal(cause) if matches!(
                &**cause,
                TemporalCause::TimedFacetRequired { construct: TimedConstruct::Release, .. }
            ) && cause.code() == Code::UnsupportedConstruct && cause.tag() == "declaration-form"
        )
    });
    assert!(found, "{refusals:?}");
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn releases_check_over_a_time_dense_model() {
    let graph = checked(
        "time Config dense unit ms;\n
         release Ctl.a on ConfigVersion::attemptUpdate;\n
         release Ctl.b on ConfigVersion::command;",
    );
    assert_eq!(graph.task_sets().releases().len(), 2);
}

/// `Both` inherits `dup` from two unrelated types, so `Both::dup` names no one
/// operation: `ambiguous_declaration`/`ambiguous-name` at the operation name.
#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn an_operation_inherited_from_two_unrelated_types_is_ambiguous() {
    let declarations = "release Ctl.a on Config::Both::dup;";
    let text = unit_text(declarations);
    let refusals =
        admit_and_assemble_with_profiles(&ambiguous_operation_document(), PROFILES, &text)
            .expect("S2 keeps the release")
            .check(CheckingLimits::default())
            .expect_err("the operation is ambiguous");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::Temporal(cause) = &refusal.cause else {
        panic!("a temporal refusal: {refusal:?}");
    };
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::AmbiguousDeclaration, "ambiguous-name")
    );
}
