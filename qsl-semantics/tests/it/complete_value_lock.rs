// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec's `quire.value.definition-lock/v1` catalog, read by reference, and
//! its package selection admission, exercised directly against the
//! compiled-in lock with synthetic trigger/role vectors.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_semantics::value::{
    native_diagnostics_catalog, CatalogRole, DefinitionLock, LockReadError, SelectionRefusalCode,
};
use sha2::{Digest, Sha256};

fn lock() -> &'static DefinitionLock {
    DefinitionLock::pinned()
}

fn always_roles() -> Vec<&'static str> {
    lock()
        .always_roles()
        .iter()
        .map(|role| role.as_str())
        .collect()
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
/// panic, and its rows are QSpec's own: each row's identity and digest are
/// the ones QSpec's document records for that role.
#[test]
fn the_compiled_in_lock_reads() {
    let read = DefinitionLock::read(quire_specification::COMPLETE_VALUE_LOCK)
        .expect("QSpec's complete-value-lock.json reads");
    assert_eq!(&read, lock());
    let document: serde_json::Value =
        serde_json::from_str(quire_specification::COMPLETE_VALUE_LOCK).unwrap();
    let rows = document["qualification_catalog"].as_array().unwrap();
    assert_eq!(rows.len(), read.catalog().len());
    for (row, entry) in rows.iter().zip(read.catalog()) {
        assert_eq!(row["role"], entry.role.as_str());
        assert_eq!(row["definition"]["identity"], entry.identity);
        assert_eq!(row["definition"]["digest"], entry.digest);
    }
    assert_eq!(read.revision(), document["revision"]);
}

/// A lock naming a role QSL has no `CatalogRole` for, or missing a role,
/// does not read.
#[test]
fn a_lock_with_an_unknown_or_missing_role_does_not_read() {
    let unknown = quire_specification::COMPLETE_VALUE_LOCK
        .replacen("\"role\": \"edition\"", "\"role\": \"editio\"", 1)
        .leak();
    assert!(matches!(
        DefinitionLock::read(unknown),
        Err(LockReadError::UnknownRole(role)) if role == "editio"
    ));
    let duplicated = quire_specification::COMPLETE_VALUE_LOCK
        .replacen("\"role\": \"root\"", "\"role\": \"edition\"", 1)
        .leak();
    assert!(matches!(
        DefinitionLock::read(duplicated),
        Err(LockReadError::RoleRowCount {
            role: CatalogRole::Edition,
            count: 2
        })
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

#[trace("QSpec-TC-192")]
#[test]
fn admit_selection_accepts_every_trigger_combination_exactly_once() {
    let lock = lock();
    let always = always_roles();

    let accept = |triggers: &[&str], extra_roles: &[&str]| {
        let mut roles = always.clone();
        roles.extend_from_slice(extra_roles);
        lock.admit_selection(triggers, &roles)
            .unwrap_or_else(|code| panic!("triggers {triggers:?}, roles {roles:?}: {code:?}"))
    };

    let no_trigger = accept(&[], &[]);
    assert!(no_trigger.triggers().is_empty());
    assert_eq!(no_trigger.roles().len(), always.len());
    assert_eq!(no_trigger.division_profile(), None);

    let text_bearing = accept(&["text_bearing"], &["text_profile"]);
    assert_eq!(text_bearing.triggers(), &BTreeSet::from(["text_bearing"]));
    assert_eq!(text_bearing.division_profile(), None);

    let ieee_operation = accept(&["ieee_operation"], &["ieee_profile"]);
    assert_eq!(
        ieee_operation.triggers(),
        &BTreeSet::from(["ieee_operation"])
    );

    for role in [
        "integer_division_euclidean",
        "integer_division_floor",
        "integer_division_truncating",
    ] {
        let division = accept(&["integer_div_rem"], &[role]);
        assert_eq!(
            division.triggers(),
            &BTreeSet::from(["integer_div_rem"]),
            "{role}"
        );
        assert!(division.division_profile().is_some(), "{role}");
    }

    let every_trigger = accept(
        &["text_bearing", "ieee_operation", "integer_div_rem"],
        &["text_profile", "ieee_profile", "integer_division_euclidean"],
    );
    assert_eq!(
        every_trigger.triggers(),
        &BTreeSet::from(["text_bearing", "ieee_operation", "integer_div_rem"])
    );
    assert!(every_trigger.division_profile().is_some());
}

#[trace("QSpec-TC-192")]
#[test]
fn admit_selection_refuses_each_closed_code_exactly_once() {
    let lock = lock();
    let always = always_roles();
    let mut codes = BTreeSet::new();

    let refuse =
        |triggers: &[&str], roles: &[&str]| lock.admit_selection(triggers, roles).unwrap_err();

    let code = refuse(&["not_a_trigger"], &always);
    assert_eq!(code, SelectionRefusalCode::SelectionUnknownTrigger);
    codes.insert(code);

    let code = refuse(&["text_bearing", "text_bearing"], &[]);
    assert_eq!(code, SelectionRefusalCode::SelectionDuplicateTrigger);
    codes.insert(code);

    let code = refuse(&[], &["not_a_role"]);
    assert_eq!(code, SelectionRefusalCode::SelectionUnknownRole);
    codes.insert(code);

    let code = refuse(&[], &["edition", "edition"]);
    assert_eq!(code, SelectionRefusalCode::SelectionDuplicateRole);
    codes.insert(code);

    let missing: Vec<_> = always.iter().skip(1).copied().collect();
    let code = refuse(&[], &missing);
    assert_eq!(code, SelectionRefusalCode::SelectionRequiredMissing);
    codes.insert(code);

    let mut conflicting = always.clone();
    conflicting.extend(["integer_division_euclidean", "integer_division_floor"]);
    let code = refuse(&["integer_div_rem"], &conflicting);
    assert_eq!(code, SelectionRefusalCode::SelectionAlternativeConflict);
    codes.insert(code);

    let code = refuse(&["text_bearing"], &always);
    assert_eq!(code, SelectionRefusalCode::SelectionTriggerUnsatisfied);
    codes.insert(code);

    let mut untriggered = always.clone();
    untriggered.push("text_profile");
    let code = refuse(&[], &untriggered);
    assert_eq!(code, SelectionRefusalCode::SelectionUntriggeredProfile);
    codes.insert(code);

    assert_eq!(codes, SelectionRefusalCode::ALL.into_iter().collect());
}
