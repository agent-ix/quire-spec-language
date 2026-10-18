// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124 (TC-519 step 1): the S3 check of a `terminal` declaration over
//! TC-458's `ConfigVersion` fixture package (`crate::model_operations`),
//! whose one population ranges over `ConfigVersion`, so the predicate's
//! `self` is a `ConfigVersion`.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::Span;
use qsl_semantics::check::{
    AssemblyCause, AssemblyError, CheckRefusal, CheckedGraph, TerminalDeclaration,
};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_body, attempt_update_modifies_version_number, config_unit_with_body,
    config_version_document,
};

const NONE: &str = "function noop using v(): Boolean pure { true }\n";
const WHEN: &str = "terminal Done using v on Config when { self.versionNumber = 1000 };\n";
const ANY: &str = "terminal Halt using v on Config any;\n";

fn document() -> Vec<u8> {
    config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    )
}

fn checked(body: &str) -> CheckedGraph {
    let declarations = admit_and_assemble_with_body(&document(), body)
        .unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"));
    declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{body}: the unit checks: {refusals:?}"))
}

fn declaration(graph: &CheckedGraph) -> &TerminalDeclaration {
    graph
        .terminal("Config")
        .expect("the selected model has a terminal declaration")
        .declaration()
}

/// FR-124-AC-1: a model with no `terminal` declaration checks to `None`,
/// `terminal when` to `When` with a Boolean body over `self`, and `terminal
/// any` to `Any`; the three differ.
#[trace("TC-519", "FR-124-AC-1")]
#[test]
fn a_model_checks_to_none_when_or_any() {
    let none = checked(NONE);
    let when = checked(WHEN);
    let any = checked(ANY);
    assert!(matches!(declaration(&none), TerminalDeclaration::None));
    assert_eq!(none.terminal("Config").and_then(|t| t.name()), None);
    let TerminalDeclaration::When(predicate) = declaration(&when) else {
        panic!("when: {:?}", declaration(&when));
    };
    assert_eq!(
        predicate
            .parameters()
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["self"]
    );
    assert_eq!(when.terminal("Config").and_then(|t| t.name()), Some("Done"));
    assert!(matches!(declaration(&any), TerminalDeclaration::Any));
    assert_eq!(any.terminal("Config").and_then(|t| t.name()), Some("Halt"));
    assert!(none.terminal("Absent").is_none());
}

fn span_of(text: &str, needle: &str) -> Span {
    let start = text.find(needle).expect("the needle is in the unit");
    Span {
        start,
        end: start + needle.len(),
    }
}

/// FR-124-AC-1: a second `terminal` declaration for one model alias refuses
/// `ambiguous_declaration`/`ambiguous-name` at itself, naming both.
#[trace("TC-519", "FR-124-AC-1")]
#[test]
fn a_second_declaration_for_one_model_refuses_naming_both() {
    let body = format!("{ANY}{WHEN}");
    let refusal = admit_and_assemble_with_body(&document(), &body)
        .expect_err("two declarations for one alias");
    let (text, _) = config_unit_with_body(&document(), &body);
    let (first, second) = (
        span_of(&text, ANY.trim_end()),
        span_of(&text, WHEN.trim_end()),
    );
    assert_eq!(
        refusal.errors,
        [AssemblyError {
            cause: AssemblyCause::AmbiguousTerminal {
                model: "Config".into(),
                spans: vec![first, second],
            },
            span: second,
        }]
    );
    let cause = &refusal.errors[0].cause;
    assert_eq!(cause.code(), Code::AmbiguousDeclaration);
    assert_eq!(cause.catalog_code().cause(), "ambiguous-name");
}

/// The code and cause of each refusal, and the checker's own refusals.
fn refusals_of(declarations: &str) -> Vec<CheckRefusal> {
    let declarations =
        admit_and_assemble_with_body(&document(), declarations).expect("the unit assembles");
    declarations
        .check(CheckingLimits::default())
        .expect_err("the unit refuses")
}

fn code_and_cause(refusals: &[CheckRefusal]) -> Vec<(Code, Option<&'static str>)> {
    refusals
        .iter()
        .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
        .collect()
}

/// FR-124-AC-1: a `terminal when` predicate that reads `pre(self.v)` refuses
/// as an invariant does, and the refusal resolves to the read in the unit.
#[trace("TC-519", "FR-124-AC-1")]
#[test]
fn a_predicate_reading_a_pre_state_refuses_as_an_invariant_does() {
    let terminal = "terminal Done using v on Config when { pre(self.versionNumber) = 1000 };\n";
    let invariant = "invariant Same using v on Config::ConfigVersion at current { \
         pre(self.versionNumber) = 1000 }\n";
    let in_terminal = refusals_of(terminal);
    assert_eq!(
        code_and_cause(&in_terminal),
        [(Code::WrongSnapshot, Some("forbidden-pre-read"))]
    );
    assert_eq!(
        code_and_cause(&refusals_of(invariant)),
        code_and_cause(&in_terminal)
    );

    let declarations =
        admit_and_assemble_with_body(&document(), terminal).expect("the unit assembles");
    let regions = declarations.regions();
    let (text, _) = config_unit_with_body(&document(), terminal);
    let region = regions
        .refusal_region(&in_terminal[0])
        .expect("the refusal names a region of the unit");
    let read = span_of(&text, "pre(self.versionNumber)");
    assert_eq!(
        (region.start(), region.end()),
        (read.start as u64, read.end as u64)
    );
}
