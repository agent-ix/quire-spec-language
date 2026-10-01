// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec's `quire.value.definition-lock/v1` catalog, read by reference, and
//! its package selection admission, exercised directly against the
//! compiled-in lock with QSpec's own package-selection vectors.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_semantics::value::{
    native_diagnostics_catalog, CatalogRole, DefinitionLock, LockReadError, SelectionRefusalCode,
    Trigger,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn lock() -> &'static DefinitionLock {
    DefinitionLock::pinned()
}

fn document() -> Value {
    serde_json::from_str(quire_specification::COMPLETE_VALUE_LOCK).unwrap()
}

fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect()
}

/// The lock text with `from` replaced once by `to`, as a `'static` string.
fn mutated(from: &str, to: &str) -> &'static str {
    let lock = quire_specification::COMPLETE_VALUE_LOCK;
    assert!(lock.contains(from), "the lock text holds {from:?}");
    lock.replacen(from, to, 1).leak()
}

#[test]
fn the_catalog_covers_every_role_exactly_once() {
    let lock = lock();
    let roles: BTreeSet<_> = lock.catalog().iter().map(|entry| entry.role).collect();
    assert_eq!(roles, CatalogRole::ALL.into_iter().collect());
    for entry in lock.catalog() {
        assert_eq!(
            CatalogRole::from_code(entry.role.as_str()),
            Some(entry.role)
        );
        assert!(!entry.identity.is_empty(), "{:?}", entry.role);
        assert!(!entry.artifact_path.is_empty(), "{:?}", entry.role);
        assert!(!entry.authority.is_empty(), "{:?}", entry.role);
        assert!(!entry.revision_namespace.is_empty(), "{:?}", entry.role);
        assert!(!entry.revision_value.is_empty(), "{:?}", entry.role);
        assert_eq!(lock.entry(entry.role), Some(entry));
    }
}

/// The compiled-in QSpec lock reads, so `DefinitionLock::pinned` cannot
/// panic, and it is QSpec's document: every row member and every
/// package-selection rule equals an independent parse of the same bytes.
#[test]
fn the_compiled_in_lock_reads() {
    let read = DefinitionLock::read(quire_specification::COMPLETE_VALUE_LOCK)
        .expect("QSpec's complete-value-lock.json reads");
    assert_eq!(&read, lock());
    let document = document();
    assert_eq!(read.revision(), document["revision"]);
    let rows = document["qualification_catalog"].as_array().unwrap();
    assert_eq!(rows.len(), read.catalog().len());
    for (row, entry) in rows.iter().zip(read.catalog()) {
        let definition = &row["definition"];
        assert_eq!(row["role"], entry.role.as_str());
        assert_eq!(row["artifact_path"], entry.artifact_path);
        assert_eq!(definition["authority"], entry.authority);
        assert_eq!(definition["identity"], entry.identity);
        assert_eq!(
            definition["revision"]["namespace"],
            entry.revision_namespace
        );
        assert_eq!(definition["revision"]["value"], entry.revision_value);
        assert_eq!(definition["digest_domain"], entry.digest_domain);
        assert_eq!(definition["digest"], entry.digest);
    }
    let selection = &document["package_selection"];
    let always: Vec<&str> = read.always_roles().iter().map(|r| r.as_str()).collect();
    assert_eq!(always, strings(&selection["always"]));
    let conditional: Vec<(&str, &str)> = read
        .conditional_roles()
        .iter()
        .map(|(trigger, role)| (trigger.as_str(), role.as_str()))
        .collect();
    let expected: Vec<(&str, &str)> = selection["conditional"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rule| {
            (
                rule["trigger"].as_str().unwrap(),
                rule["role"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(conditional, expected);
    let exactly_one: Vec<(&str, Vec<&str>)> = read
        .exactly_one_roles()
        .iter()
        .map(|(trigger, roles)| (trigger.as_str(), roles.iter().map(|r| r.as_str()).collect()))
        .collect();
    let expected: Vec<(&str, Vec<&str>)> = selection["exactly_one"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rule| (rule["trigger"].as_str().unwrap(), strings(&rule["roles"])))
        .collect();
    assert_eq!(exactly_one, expected);
}

/// A lock naming a role QSL has no `CatalogRole` for, a role twice, or no
/// row for a role does not read.
#[test]
fn a_lock_with_an_unknown_duplicated_or_missing_role_does_not_read() {
    assert!(matches!(
        DefinitionLock::read(mutated("\"role\": \"edition\"", "\"role\": \"editio\"")),
        Err(LockReadError::UnknownRole(role)) if role == "editio"
    ));
    assert!(matches!(
        DefinitionLock::read(mutated("\"role\": \"root\"", "\"role\": \"edition\"")),
        Err(LockReadError::RoleRowCount {
            role: CatalogRole::Edition,
            count: 2
        })
    ));
    let mut document = document();
    document["qualification_catalog"]
        .as_array_mut()
        .unwrap()
        .retain(|row| row["role"] != "accounting");
    let missing = serde_json::to_string(&document).unwrap().leak();
    assert!(matches!(
        DefinitionLock::read(missing),
        Err(LockReadError::RoleRowCount {
            role: CatalogRole::Accounting,
            count: 0
        })
    ));
}

/// A selection rule naming a trigger outside the closed vocabulary, or a
/// vocabulary other than `Trigger::ALL`, does not read.
#[test]
fn a_lock_with_an_unknown_trigger_or_vocabulary_does_not_read() {
    assert!(matches!(
        DefinitionLock::read(mutated(
            "{\"trigger\": \"text_bearing\", \"role\"",
            "{\"trigger\": \"text_bearin\", \"role\""
        )),
        Err(LockReadError::UnknownTrigger(trigger)) if trigger == "text_bearin"
    ));
    let missing = mutated(
        "[\"ieee_operation\", \"integer_div_rem\", \"text_bearing\"]",
        "[\"ieee_operation\", \"integer_div_rem\"]",
    );
    assert!(matches!(
        DefinitionLock::read(missing),
        Err(LockReadError::TriggerVocabulary(_))
    ));
    let extra = mutated(
        "[\"ieee_operation\", \"integer_div_rem\", \"text_bearing\"]",
        "[\"ieee_operation\", \"integer_div_rem\", \"text_bearing\", \"host_float\"]",
    );
    assert!(matches!(
        DefinitionLock::read(extra),
        Err(LockReadError::TriggerVocabulary(_))
    ));
}

/// A `selection_refusal_codes` set missing a code, or naming one twice, does
/// not read.
#[test]
fn a_lock_with_another_refusal_code_set_does_not_read() {
    let dropped = mutated(", \"selection_untriggered_profile\"", "");
    assert!(matches!(
        DefinitionLock::read(dropped),
        Err(LockReadError::RefusalCodes(codes)) if codes.len() == 7
    ));
    let duplicated = mutated(
        "\"selection_untriggered_profile\"]",
        "\"selection_untriggered_profile\", \"selection_untriggered_profile\"]",
    );
    assert!(matches!(
        DefinitionLock::read(duplicated),
        Err(LockReadError::RefusalCodes(codes)) if codes.len() == 9
    ));
}

/// The diagnostics catalog's `DefinitionRef` is QSpec's document: its
/// identity and revision from the document's header, its digest the SHA-256
/// of the document's bytes.
#[test]
fn the_native_diagnostics_catalog_reads_its_header() {
    let catalog = native_diagnostics_catalog();
    let document = quire_specification::NATIVE_DIAGNOSTICS;
    assert_eq!(catalog.identity, "quire.native.diagnostics/v1");
    assert!(document.contains(&format!(
        "Interpretation identity: `{}`; revision: `{}`.",
        catalog.identity, catalog.revision.value
    )));
    assert_eq!(
        catalog.digest,
        format!("{:x}", Sha256::digest(document.as_bytes()))
    );
    assert_eq!(catalog.digest_domain, "quire.definition.bytes/v1");
}

fn selection_vectors() -> Value {
    serde_json::from_str(quire_specification::COMPLETE_VALUE_SELECTION_VECTORS).unwrap()
}

/// The vector's selected roles: the always roles less any omitted, then its
/// additional roles.
fn vector_roles<'a>(vectors: &'a Value, vector: &'a Value) -> Vec<&'a str> {
    let omitted = vector
        .get("omitted_always_roles")
        .map(strings)
        .unwrap_or_default();
    let mut roles: Vec<&str> = strings(&vectors["always_roles"])
        .into_iter()
        .filter(|role| !omitted.contains(role))
        .collect();
    roles.extend(strings(&vector["additional_roles"]));
    roles
}

/// Every accepted QSpec vector admits, with exactly its triggers and roles,
/// and a division law exactly when `integer_div_rem` is present.
#[trace("QSpec-TC-192")]
#[test]
fn admit_selection_accepts_every_accepted_qspec_vector() {
    let vectors = selection_vectors();
    let accepted = vectors["accepted"].as_array().unwrap();
    assert!(!accepted.is_empty());
    for vector in accepted {
        let name = &vector["name"];
        let triggers = strings(&vector["triggers"]);
        let roles = vector_roles(&vectors, vector);
        let admitted = lock()
            .admit_selection(&triggers, &roles)
            .unwrap_or_else(|code| panic!("{name}: {code:?}"));
        let expected: BTreeSet<Trigger> = triggers
            .iter()
            .map(|code| Trigger::from_code(code).unwrap())
            .collect();
        assert_eq!(admitted.triggers(), &expected, "{name}");
        let roles: BTreeSet<CatalogRole> = roles
            .iter()
            .map(|code| CatalogRole::from_code(code).unwrap())
            .collect();
        assert_eq!(admitted.roles(), &roles, "{name}");
        assert_eq!(
            admitted.division_profile().is_some(),
            expected.contains(&Trigger::IntegerDivRem),
            "{name}"
        );
    }
}

/// Every refused QSpec vector, including the two ordering vectors, refuses
/// with exactly its `expected_code`, and the vectors together reach every
/// closed refusal code.
#[trace("QSpec-TC-192")]
#[test]
fn admit_selection_refuses_every_refused_qspec_vector_with_its_code() {
    let vectors = selection_vectors();
    let refused = vectors["refused"].as_array().unwrap();
    let mut codes = BTreeSet::new();
    for vector in refused {
        let name = &vector["name"];
        let expected = SelectionRefusalCode::from_code(vector["expected_code"].as_str().unwrap())
            .unwrap_or_else(|| panic!("{name}: unknown expected_code"));
        let code = lock()
            .admit_selection(
                &strings(&vector["triggers"]),
                &vector_roles(&vectors, vector),
            )
            .expect_err(&format!("{name} admitted"));
        assert_eq!(code, expected, "{name}");
        codes.insert(code);
    }
    assert_eq!(codes, SelectionRefusalCode::ALL.into_iter().collect());
}
