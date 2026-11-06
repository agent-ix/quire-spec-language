// SPDX-License-Identifier: AGPL-3.0-or-later
//! Extension definition validation tests (FR-354 "Definition validation";
//! TC-883 steps 1 to 4; QSpec FR-467).
use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::Code;
use serde_json::{json, Value};

use super::bundle::{Definition, DefinitionCatalog, DefinitionRole, ReaderAuthority};
use super::extension::{
    ConflictKind, ConflictWith, CoreVocabulary, ExtensionCause, ExtensionRefusal, MissingMember,
};

const READER_AUTHORITY: ReaderAuthority = ReaderAuthority::fixture();

fn reference(identity: &str) -> DefinitionRef {
    DefinitionRef::new("agent-ix", identity).unwrap()
}

fn core() -> CoreVocabulary {
    CoreVocabulary::new(["function", "model", "record"], ["model", "function_decl"])
}

/// TC-882's `retention` extension document.
fn retention(keyword: &str, node_kind: &str) -> Value {
    json!({
        "grammar_schema": {"productions": [
            {"keyword": keyword, "clauses": ["subject", "days"]}
        ]},
        "typed_node_schema": {"nodes": [
            {"keyword": keyword, "node_kind": node_kind, "clauses": [
                {"name": "subject", "type": "Text"},
                {"name": "days", "type": "Int[1, 3650]"}
            ]}
        ]}
    })
}

fn extension(identity: &str, dependencies: &[&str], document: &Value) -> Definition {
    Definition::from_exact_bytes(
        &READER_AUTHORITY,
        "agent-ix",
        identity,
        DefinitionRole::Source,
        dependencies.iter().map(|id| reference(id)).collect(),
        BTreeSet::new(),
        document.to_string().as_bytes(),
    )
    .unwrap()
}

fn hold(
    definitions: Vec<Definition>,
    selected: &[&str],
) -> Result<super::extension::HeldExtensions, ExtensionRefusal> {
    let selected: Vec<_> = selected.iter().map(|id| reference(id)).collect();
    DefinitionCatalog::new(definitions)
        .unwrap()
        .hold_extensions(&selected, &core())
}

fn missing(document: &Value) -> MissingMember {
    match hold(vec![extension("ext.a", &[], document)], &["ext.a"]).unwrap_err() {
        ExtensionRefusal::MissingMember { definition, member } => {
            assert_eq!(definition, reference("ext.a"));
            member
        }
        other => panic!("expected missing-member, got {other:?}"),
    }
}

#[trace("TC-883", "QSpec-FR-467-AC-1")]
#[test]
fn a_valid_extension_is_held_with_its_productions_joined_to_node_entries() {
    let held = hold(
        vec![extension(
            "ext.a",
            &[],
            &retention("retention", "retention"),
        )],
        &["ext.a"],
    )
    .unwrap();
    let extension = held.get(&reference("ext.a")).unwrap();
    let production = &extension.productions()[0];
    assert_eq!(production.keyword, "retention");
    assert_eq!(production.node_kind, "retention");
    let clauses: Vec<_> = production
        .clauses
        .iter()
        .map(|c| (c.name.as_str(), c.type_ref.as_str()))
        .collect();
    assert_eq!(clauses, [("subject", "Text"), ("days", "Int[1, 3650]")]);
    assert_eq!(held.len(), 1);
}

#[trace("TC-883", "FR-354-AC-3", "QSpec-FR-467-AC-2", "QSpec-FR-467-AC-10")]
#[test]
fn a_missing_member_refuses_naming_the_definition_and_the_member() {
    let mut no_typed = retention("retention", "retention");
    no_typed
        .as_object_mut()
        .unwrap()
        .remove("typed_node_schema");
    assert_eq!(missing(&no_typed), MissingMember::TypedNodeSchema);

    let mut no_grammar = retention("retention", "retention");
    no_grammar.as_object_mut().unwrap().remove("grammar_schema");
    assert_eq!(missing(&no_grammar), MissingMember::GrammarSchema);

    let mut no_entry = retention("retention", "retention");
    no_entry["typed_node_schema"]["nodes"] = json!([]);
    assert_eq!(
        missing(&no_entry),
        MissingMember::NodeEntry {
            keyword: "retention".into()
        }
    );

    let mut no_kind = retention("retention", "retention");
    no_kind["typed_node_schema"]["nodes"][0]
        .as_object_mut()
        .unwrap()
        .remove("node_kind");
    assert_eq!(
        missing(&no_kind),
        MissingMember::NodeKind {
            keyword: "retention".into()
        }
    );

    let mut no_clause = retention("retention", "retention");
    no_clause["typed_node_schema"]["nodes"][0]["clauses"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    assert_eq!(
        missing(&no_clause),
        MissingMember::ClauseEntry {
            keyword: "retention".into(),
            clause: "days".into()
        }
    );

    let mut no_type = retention("retention", "retention");
    no_type["typed_node_schema"]["nodes"][0]["clauses"][0]
        .as_object_mut()
        .unwrap()
        .remove("type");
    assert_eq!(
        missing(&no_type),
        MissingMember::ClauseType {
            keyword: "retention".into(),
            clause: "subject".into()
        }
    );

    let mut extra_clause = retention("retention", "retention");
    extra_clause["typed_node_schema"]["nodes"][0]["clauses"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "extra", "type": "Text"}));
    assert_eq!(
        missing(&extra_clause),
        MissingMember::UnlistedClause {
            keyword: "retention".into(),
            clause: "extra".into()
        }
    );

    let mut orphan = retention("retention", "retention");
    orphan["typed_node_schema"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"keyword": "orphan", "node_kind": "orphan", "clauses": []}));
    assert_eq!(
        missing(&orphan),
        MissingMember::ProductionForEntry {
            keyword: "orphan".into()
        }
    );

    let mut no_productions = retention("retention", "retention");
    no_productions["grammar_schema"]["productions"] = json!([]);
    assert_eq!(missing(&no_productions), MissingMember::Production);

    let refusal = hold(vec![extension("ext.a", &[], &no_typed)], &["ext.a"]).unwrap_err();
    assert_eq!(refusal.code(), Code::InvalidPackage);
    assert_eq!(refusal.cause(), ExtensionCause::MissingMember);
    assert_eq!(refusal.cause().as_str(), "missing-member");
}

fn conflict(refusal: ExtensionRefusal) -> (DefinitionRef, ConflictKind, String, ConflictWith) {
    assert_eq!(refusal.code(), Code::InvalidPackage);
    assert_eq!(refusal.cause().as_str(), "conflicting-definition");
    match refusal {
        ExtensionRefusal::ConflictingDefinition {
            definition,
            kind,
            name,
            with,
        } => (definition, kind, name, with),
        other => panic!("expected conflicting-definition, got {other:?}"),
    }
}

#[trace("TC-883", "FR-354-AC-3", "QSpec-FR-467-AC-3", "QSpec-FR-467-AC-4")]
#[test]
fn a_core_keyword_or_core_node_kind_refuses_naming_the_core_grammar() {
    let by_keyword = hold(
        vec![extension("ext.a", &[], &retention("function", "retention"))],
        &["ext.a"],
    )
    .unwrap_err();
    assert_eq!(
        conflict(by_keyword),
        (
            reference("ext.a"),
            ConflictKind::Keyword,
            "function".into(),
            ConflictWith::CoreGrammar
        )
    );
    let by_kind = hold(
        vec![extension("ext.a", &[], &retention("retention", "model"))],
        &["ext.a"],
    )
    .unwrap_err();
    assert_eq!(
        conflict(by_kind),
        (
            reference("ext.a"),
            ConflictKind::NodeKind,
            "model".into(),
            ConflictWith::CoreGrammar
        )
    );
}

#[trace("TC-883", "FR-354-AC-3", "QSpec-FR-467-AC-3", "QSpec-FR-467-AC-4")]
#[test]
fn two_selected_extensions_sharing_a_keyword_or_node_kind_refuse_naming_both() {
    let same_keyword = hold(
        vec![
            extension("ext.a", &[], &retention("retention", "kind_a")),
            extension("ext.b", &[], &retention("retention", "kind_b")),
        ],
        &["ext.a", "ext.b"],
    )
    .unwrap_err();
    assert_eq!(
        conflict(same_keyword),
        (
            reference("ext.b"),
            ConflictKind::Keyword,
            "retention".into(),
            ConflictWith::Definition(reference("ext.a"))
        )
    );
    let same_kind = hold(
        vec![
            extension("ext.a", &[], &retention("alpha", "retention")),
            extension("ext.b", &[], &retention("beta", "retention")),
        ],
        &["ext.a", "ext.b"],
    )
    .unwrap_err();
    assert_eq!(
        conflict(same_kind),
        (
            reference("ext.b"),
            ConflictKind::NodeKind,
            "retention".into(),
            ConflictWith::Definition(reference("ext.a"))
        )
    );
}

#[trace("TC-883", "FR-354-AC-3", "QSpec-FR-467-AC-8", "QSpec-FR-467-AC-10")]
#[test]
fn collisions_within_one_definition_name_both_indices() {
    let mut document = retention("alpha", "kind_a");
    document["grammar_schema"]["productions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"keyword": "alpha", "clauses": ["subject", "days"]}));
    let by_keyword = hold(vec![extension("ext.a", &[], &document)], &["ext.a"]).unwrap_err();
    assert_eq!(
        conflict(by_keyword),
        (
            reference("ext.a"),
            ConflictKind::Keyword,
            "alpha".into(),
            ConflictWith::SameDefinition {
                first: 0,
                second: 1
            }
        )
    );

    let mut document = retention("alpha", "kind_a");
    document["grammar_schema"]["productions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"keyword": "beta", "clauses": []}));
    document["typed_node_schema"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"keyword": "beta", "node_kind": "kind_a", "clauses": []}));
    let by_kind = hold(vec![extension("ext.a", &[], &document)], &["ext.a"]).unwrap_err();
    assert_eq!(
        conflict(by_kind),
        (
            reference("ext.a"),
            ConflictKind::NodeKind,
            "kind_a".into(),
            ConflictWith::SameDefinition {
                first: 0,
                second: 1
            }
        )
    );

    let mut document = retention("alpha", "kind_a");
    let entry = document["typed_node_schema"]["nodes"][0].clone();
    document["typed_node_schema"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(entry);
    let by_entry = hold(vec![extension("ext.a", &[], &document)], &["ext.a"]).unwrap_err();
    assert_eq!(
        conflict(by_entry),
        (
            reference("ext.a"),
            ConflictKind::EntryKeyword,
            "alpha".into(),
            ConflictWith::SameDefinition {
                first: 0,
                second: 1
            }
        )
    );
}

#[trace("TC-883", "FR-354-AC-3", "QSpec-FR-467-AC-5")]
#[test]
fn extensions_depending_on_each_other_refuse_naming_the_cycle_in_path_order() {
    let refusal = hold(
        vec![
            extension("ext.a", &["ext.b"], &retention("alpha", "kind_a")),
            extension("ext.b", &["ext.a"], &retention("beta", "kind_b")),
        ],
        &["ext.a", "ext.b"],
    )
    .unwrap_err();
    assert_eq!(refusal.code(), Code::InvalidPackage);
    assert_eq!(refusal.cause().as_str(), "definition-cycle");
    assert_eq!(
        refusal,
        ExtensionRefusal::DefinitionCycle(vec![
            reference("ext.a"),
            reference("ext.b"),
            reference("ext.a")
        ])
    );
}

#[trace("TC-883", "FR-354-AC-3", "QSpec-FR-467-AC-6")]
#[test]
fn only_the_first_violated_rule_is_reported() {
    // Missing member, keyword collision and cycle: missing-member only.
    let mut broken = retention("function", "kind_b");
    broken.as_object_mut().unwrap().remove("typed_node_schema");
    let refusal = hold(
        vec![
            extension("ext.a", &["ext.b"], &retention("alpha", "kind_a")),
            extension("ext.b", &["ext.a"], &broken),
        ],
        &["ext.a", "ext.b"],
    )
    .unwrap_err();
    assert_eq!(refusal.cause(), ExtensionCause::MissingMember);

    // A collision and a cycle: conflicting-definition only.
    let refusal = hold(
        vec![
            extension("ext.a", &["ext.b"], &retention("function", "kind_a")),
            extension("ext.b", &["ext.a"], &retention("beta", "kind_b")),
        ],
        &["ext.a", "ext.b"],
    )
    .unwrap_err();
    assert_eq!(refusal.cause(), ExtensionCause::ConflictingDefinition);
}

#[trace("TC-883", "FR-354-AC-3")]
#[test]
fn only_selected_extensions_are_validated_and_held() {
    // An unselected invalid definition neither refuses nor is held.
    let mut broken = retention("retention", "kind_b");
    broken.as_object_mut().unwrap().remove("typed_node_schema");
    let held = hold(
        vec![
            extension("ext.a", &[], &retention("alpha", "kind_a")),
            extension("ext.b", &[], &broken),
        ],
        &["ext.a"],
    )
    .unwrap();
    assert_eq!(held.len(), 1);
    assert!(held.get(&reference("ext.b")).is_none());

    let refusal = hold(vec![], &["ext.a"]).unwrap_err();
    assert_eq!(refusal.code(), Code::MissingImport);
}
