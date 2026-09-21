// SPDX-License-Identifier: AGPL-3.0-or-later
//! The closed `quire.value.definition-lock/v1` catalog and its package
//! selection admission, exercised directly against the compiled-in lock with
//! synthetic trigger/role vectors.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use quire_spec_language::value::{CatalogRole, DefinitionLock, SelectionRefusalCode, Trigger};

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

#[trace("Task-048")]
#[test]
fn the_catalog_covers_every_role_exactly_once() {
    let lock = lock();
    assert_eq!(lock.revision(), "1-draft.1");
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

#[trace("TC-192")]
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
    assert_eq!(
        text_bearing.triggers(),
        &BTreeSet::from([Trigger::TextBearing])
    );
    assert_eq!(text_bearing.division_profile(), None);

    let ieee_operation = accept(&["ieee_operation"], &["ieee_profile"]);
    assert_eq!(
        ieee_operation.triggers(),
        &BTreeSet::from([Trigger::IeeeOperation])
    );

    for role in [
        "integer_division_euclidean",
        "integer_division_floor",
        "integer_division_truncating",
    ] {
        let division = accept(&["integer_div_rem"], &[role]);
        assert_eq!(
            division.triggers(),
            &BTreeSet::from([Trigger::IntegerDivRem]),
            "{role}"
        );
        assert!(division.division_profile().is_some(), "{role}");
    }

    let every_trigger = accept(
        &["text_bearing", "ieee_operation", "integer_div_rem"],
        &[
            "text_profile",
            "ieee_profile",
            "integer_division_euclidean",
        ],
    );
    assert_eq!(
        every_trigger.triggers(),
        &BTreeSet::from([
            Trigger::TextBearing,
            Trigger::IeeeOperation,
            Trigger::IntegerDivRem
        ])
    );
    assert!(every_trigger.division_profile().is_some());
}

#[trace("TC-192")]
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
