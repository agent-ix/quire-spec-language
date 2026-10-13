// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-102, FR-103, FR-302: the unit `operation-body` declarations and the
//! unit `pre`/`post` clauses are retained by the assembler under the resolved
//! operation's `DeclarationKey` (TC-986, TC-987), and `has_body` is set from
//! them. `ConfigVersion` plays `A` and `Sub` plays `B` of the test cases, `B`
//! being a subtype of `A`, each declaring `size` with the domain package's
//! own `requires`/`ensures` text `false`.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_forms::ExprNode;
use qsl_semantics::check::{AssemblyCause, AssemblyRefusal, PackageDeclarations};
use qsl_semantics::model::domain_package::{DomainPackageRecord, OperationMemberRecord};
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::Integer;
use serde_json::{json, Value};

use super::model_operations::{
    add_sub_type, admit_and_assemble_with_body, config_unit_with_body,
    config_version_document_with_operations, empty_frame, operation, PACKAGE_IDENTITY,
};

/// A domain operation `name` on `owner`, returning an `Integer`, whose
/// `requires` and `ensures` are the one `quire` clause `false`.
fn domain_operation(owner: &str, name: &str) -> Value {
    let mut value = operation(
        name,
        json!([]),
        Some("ix://quire/native/Integer"),
        empty_frame(),
    );
    value["identity"] = json!(format!("ix://{PACKAGE_IDENTITY}/{owner}/{name}"));
    let false_clause = json!({
        "language": "quire",
        "text": "false",
        "origin": value["origin"].clone(),
    });
    value["pre"] = json!([false_clause.clone()]);
    value["post"] = json!([false_clause]);
    value
}

/// `ConfigVersion` (`A`) declaring `size` and each of `extra`, and `Sub`
/// (`B`, a subtype of `A`) declaring `size`.
fn document(extra: &[&str]) -> Vec<u8> {
    let operations = std::iter::once("size")
        .chain(extra.iter().copied())
        .map(|name| domain_operation("ConfigVersion", name))
        .collect();
    let document = add_sub_type(config_version_document_with_operations(operations), &[]);
    let mut envelope: Value = serde_json::from_slice(&document).expect("valid JSON");
    envelope["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|node| node["identity"] == format!("ix://{PACKAGE_IDENTITY}/Sub"))
        .expect("Sub is declared")["operations"] = json!([domain_operation("Sub", "size")]);
    serde_json::to_vec(&envelope).expect("valid JSON")
}

fn key(owner: &str, name: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: format!("ix://{PACKAGE_IDENTITY}/{owner}/{name}"),
    }
}

/// TC-986 step 2's declarations: the two bodies (`B`'s first), a `pre` and a
/// `post` clause of `true` on each of `A.size` and `B.size`.
fn unit_declarations(first: &str, second: &str) -> String {
    format!(
        "body {first} using v on Config::Sub::size() {{ 22 }}\n\
         body {second} using v on Config::ConfigVersion::size() {{ 11 }}\n\
         pre preA using v on Config::ConfigVersion::size {{ true }}\n\
         post postA using v on Config::ConfigVersion::size {{ true }}\n\
         pre preB using v on Config::Sub::size {{ true }}\n\
         post postB using v on Config::Sub::size {{ true }}\n"
    )
}

fn assemble(document: &[u8], declarations: &str) -> Result<PackageDeclarations, AssemblyRefusal> {
    admit_and_assemble_with_body(document, declarations)
}

fn assembled(document: &[u8], declarations: &str) -> PackageDeclarations {
    assemble(document, declarations).unwrap_or_else(|refusal| panic!("{refusal:?}"))
}

fn integer_literal(expression: &qsl_forms::Expression) -> Integer {
    let ExprNode::Integer(value) = expression.root_node() else {
        panic!("an integer literal, not {:?}", expression.root_node());
    };
    value.clone()
}

fn is_true(expression: &qsl_forms::Expression) -> bool {
    matches!(expression.root_node(), ExprNode::Boolean(true))
}

/// The operation record under `key` in the selected `Config` model's view.
fn record(package: &PackageDeclarations, key: &DeclarationKey) -> OperationMemberRecord {
    package.selected_views["Config"]
        .domain_package()
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(operation) if &operation.key == key => {
                Some(operation.clone())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("the view declares {key:?}"))
}

/// What the unit retained, as comparable data, independent of body labels
/// and declaration order: each body's key and literal, each key's unit
/// clauses' kinds and trivial bodies.
fn retained(package: &PackageDeclarations) -> Vec<(DeclarationKey, Integer, Vec<String>)> {
    let mut rows: Vec<_> = package
        .operation_bodies
        .iter()
        .map(|body| {
            let clauses = package
                .unit_clauses_for(&body.key)
                .into_iter()
                .map(|clause| format!("{:?}:{}", clause.kind, is_true(&clause.body)))
                .collect();
            (body.key.clone(), integer_literal(&body.body), clauses)
        })
        .collect();
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    rows
}

/// TC-986 steps 1 to 3: a single body for each of `A.size` and `B.size`
/// joins, with the retained expressions and the unit `pre`/`post` clauses,
/// to the admitted operations by resolved `DeclarationKey`. `has_body` is
/// true for those operations; the domain package's own `false` texts stay
/// each operation's `requires`/`ensures` and supply neither the unit clauses
/// nor a body.
#[trace("TC-986", "FR-302-AC-7", "FR-103-AC-9")]
#[test]
fn bodies_and_unit_clauses_join_the_admitted_operations_by_resolved_key() {
    let document = document(&[]);
    let declarations = unit_declarations("forB", "forA");
    let package = assembled(&document, &declarations);
    let text = config_unit_with_body(&document, &declarations).0;
    let (a, b) = (key("ConfigVersion", "size"), key("Sub", "size"));

    assert_eq!(package.operation_bodies.len(), 2);
    let for_a = package.operation_body(&a).expect("A.size has a body");
    let for_b = package.operation_body(&b).expect("B.size has a body");
    assert_eq!(integer_literal(&for_a.body), Integer::from(11_i64));
    assert_eq!(integer_literal(&for_b.body), Integer::from(22_i64));
    assert_eq!(for_a.label.name, "forA");
    assert_eq!(for_b.label.name, "forB");

    // Each unit clause is associated with its operation's key and keeps the
    // source identity of the unit declaration that wrote it.
    for (key, suffix) in [(&a, "A"), (&b, "B")] {
        let clauses = package.unit_clauses_for(key);
        let [pre, post] = clauses.as_slice() else {
            panic!("one pre and one post clause for {key:?}, got {clauses:?}");
        };
        assert_eq!(pre.name, format!("pre{suffix}"));
        assert_eq!(post.name, format!("post{suffix}"));
        assert!(is_true(&pre.body) && is_true(&post.body));
        for (clause, keyword) in [(pre, "pre"), (post, "post")] {
            let span = clause.spans.declaration;
            assert!(
                text[span.start..span.end].starts_with(&format!("{keyword} {}", clause.name)),
                "{:?}",
                &text[span.start..span.end]
            );
        }
    }

    // `has_body` follows the unit's bodies; the domain `false` texts stay
    // the operations' own requires and ensures.
    for key in [&a, &b] {
        let operation = record(&package, key);
        assert!(operation.has_body, "{key:?}");
        let texts =
            |clauses: &[qsl_semantics::model::domain_package::OperationContractClause]| {
                clauses
                    .iter()
                    .map(|clause| clause.text.clone())
                    .collect::<Vec<_>>()
            };
        assert_eq!(texts(&operation.requires), ["false"]);
        assert_eq!(texts(&operation.ensures), ["false"]);
    }
}

/// TC-986 step 3: reversing the body declaration order or changing the local
/// labels leaves every association unchanged.
#[trace("TC-986", "FR-302-AC-7")]
#[test]
fn reordering_and_relabelling_leave_the_associations_unchanged() {
    let document = document(&[]);
    let original = assembled(&document, &unit_declarations("forB", "forA"));
    let relabelled = assembled(&document, &unit_declarations("x1", "y2"));
    let reversed = assembled(
        &document,
        "body forA using v on Config::ConfigVersion::size() { 11 }\n\
         body forB using v on Config::Sub::size() { 22 }\n\
         post postB using v on Config::Sub::size { true }\n\
         pre preB using v on Config::Sub::size { true }\n\
         post postA using v on Config::ConfigVersion::size { true }\n\
         pre preA using v on Config::ConfigVersion::size { true }\n",
    );
    let baseline = retained(&original);
    assert_eq!(baseline.len(), 2);
    assert_eq!(retained(&relabelled), baseline);
    // The reversed unit lists each key's clauses post-first, so compare the
    // keys and literals, and the clause sets.
    let flatten = |rows: Vec<(DeclarationKey, Integer, Vec<String>)>| {
        rows.into_iter()
            .map(|(key, literal, mut clauses)| {
                clauses.sort();
                (key, literal, clauses)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(flatten(retained(&reversed)), flatten(baseline));
}

/// TC-986 step 4 (the `has_body = false` control): an operation without a
/// `body` declaration has `has_body = false` and no retained body, whether
/// or not the unit writes clauses about it and whatever its domain texts say.
#[trace("TC-986", "FR-302-AC-7", "FR-103-AC-9")]
#[test]
fn an_operation_without_a_body_declaration_has_no_body() {
    let document = document(&["empty"]);
    let package = assembled(
        &document,
        "body forB using v on Config::Sub::size() { 22 }\n\
         pre preEmpty using v on Config::ConfigVersion::empty { true }\n",
    );
    let (a, b, empty) = (
        key("ConfigVersion", "size"),
        key("Sub", "size"),
        key("ConfigVersion", "empty"),
    );
    assert!(record(&package, &b).has_body);
    for bodyless in [&a, &empty] {
        assert!(!record(&package, bodyless).has_body, "{bodyless:?}");
        assert!(package.operation_body(bodyless).is_none(), "{bodyless:?}");
    }
    // A unit clause about the bodyless operation is still retained under
    // its key and sets nothing.
    assert_eq!(package.unit_clauses_for(&empty).len(), 1);
    assert_eq!(package.operation_bodies.len(), 1);

    // No body declaration at all: every operation's `has_body` is false.
    let none = assembled(
        &document,
        "function same using v(a: Boolean): Boolean pure { a }",
    );
    assert!(none.operation_bodies.is_empty());
    for name in [&a, &b, &empty] {
        assert!(!record(&none, name).has_body, "{name:?}");
    }
}

/// FR-103: a body resolves through the target type's effective view, so a
/// body written on a subtype for an operation it inherits is associated with
/// the declaring type's operation.
#[trace("TC-986", "FR-302-AC-7")]
#[test]
fn a_body_on_an_inherited_operation_is_keyed_by_the_declaring_operation() {
    let document = document(&["empty"]);
    let package = assembled(
        &document,
        "body viaSub using v on Config::Sub::empty() { 5 }\n",
    );
    let declaring = key("ConfigVersion", "empty");
    let body = package
        .operation_body(&declaring)
        .expect("keyed by the declaring operation");
    assert_eq!(integer_literal(&body.body), Integer::from(5_i64));
    assert!(record(&package, &declaring).has_body);
    assert!(!record(&package, &key("ConfigVersion", "size")).has_body);
}

/// The keys of the operations with `has_body`, in the selected view.
fn bodied(package: &PackageDeclarations) -> BTreeSet<DeclarationKey> {
    package.selected_views["Config"]
        .domain_package()
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::OperationMember(operation) if operation.has_body => {
                Some(operation.key.clone())
            }
            _ => None,
        })
        .collect()
}

/// TC-986: the view `has_body` is derived from the retained bodies alone.
#[trace("TC-986", "FR-103-AC-9")]
#[test]
fn has_body_is_exactly_the_set_of_retained_body_keys() {
    let document = document(&["empty"]);
    let package = assembled(&document, &unit_declarations("forB", "forA"));
    let retained_keys: BTreeSet<_> = package
        .operation_bodies
        .iter()
        .map(|body| body.key.clone())
        .collect();
    assert_eq!(bodied(&package), retained_keys);
    assert_eq!(retained_keys.len(), 2);
}

/// TC-987: two body forms resolving to one `DeclarationKey` refuse as
/// `conflicting_declaration`/`duplicate-declaration`, naming both
/// declarations, with no package; source order and labels do not change the
/// outcome, and either body alone assembles.
#[trace("TC-987", "FR-302-AC-7")]
#[test]
fn two_bodies_resolving_to_one_key_refuse_as_a_duplicate_declaration() {
    let document = document(&[]);
    for (first, second) in [
        ("firstBody", "secondBody"),
        ("secondBody", "firstBody"),
        ("same", "other"),
    ] {
        let declarations = format!(
            "body {first} using v on Config::ConfigVersion::size() {{ 11 }}\n\
             body {second} using v on Config::ConfigVersion::size() {{ 22 }}\n"
        );
        let refusal = assemble(&document, &declarations)
            .expect_err("two bodies for one operation do not assemble");
        let [error] = refusal.errors.as_slice() else {
            panic!("exactly one refusal, got {:?}", refusal.errors);
        };
        let AssemblyCause::DuplicateOperationBody { operation, bodies } = &error.cause else {
            panic!("a duplicate body, not {:?}", error.cause);
        };
        assert_eq!(operation, &key("ConfigVersion", "size"));
        let text = config_unit_with_body(&document, &declarations).0;
        let named: Vec<&str> = bodies
            .iter()
            .map(|span| &text[span.start..span.end])
            .collect();
        assert_eq!(named, [first, second], "both declarations, in source order");
        let code = error.cause.catalog_code();
        assert_eq!(code.code(), "conflicting_declaration");
        assert_eq!(code.cause(), "duplicate-declaration");
    }

    // The single-body control (TC-986) assembles, for either body.
    for declarations in [
        "body firstBody using v on Config::ConfigVersion::size() { 11 }\n",
        "body secondBody using v on Config::ConfigVersion::size() { 22 }\n",
    ] {
        let package = assembled(&document, declarations);
        let body = package
            .operation_body(&key("ConfigVersion", "size"))
            .expect("the one body is keyed to A.size");
        assert_eq!(package.operation_bodies.len(), 1);
        assert!(record(&package, &body.key).has_body);
    }
}

/// TC-987: bodies for the same name on distinct types, or on a type and its
/// subtype that declares its own, are not duplicates: they resolve to
/// distinct keys.
#[trace("TC-987", "FR-302-AC-7")]
#[test]
fn bodies_for_distinct_resolved_keys_are_not_duplicates() {
    let document = document(&[]);
    let package = assembled(
        &document,
        "body one using v on Config::ConfigVersion::size() { 1 }\n\
         body two using v on Config::Sub::size() { 2 }\n",
    );
    assert_eq!(package.operation_bodies.len(), 2);
}

/// FR-302: a body declaration whose target names no operation refuses
/// `missing_declaration` under FR-104's resolution rule, with no package.
#[trace("TC-986", "FR-302-AC-7")]
#[test]
fn a_body_with_no_matching_operation_refuses_missing_declaration() {
    let document = document(&[]);
    let refusal = assemble(
        &document,
        "body nothing using v on Config::ConfigVersion::absent() { 1 }\n",
    )
    .expect_err("no operation `absent` exists");
    let [error] = refusal.errors.as_slice() else {
        panic!("exactly one refusal, got {:?}", refusal.errors);
    };
    assert!(
        matches!(&error.cause, AssemblyCause::UnresolvedOperation { operation, .. } if operation == "absent"),
        "{:?}",
        error.cause
    );
    assert_eq!(error.cause.catalog_code().code(), "missing_declaration");
}
