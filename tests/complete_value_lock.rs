// SPDX-License-Identifier: AGPL-3.0-or-later
//! Package selection follows the closed lock's canonical vectors.
//!
//! QSL-169 (PLAT-887): the two tests that lived here which asserted the
//! vendored `resources/complete-value` artifacts' bytes against a pinned
//! digest, and the JSON lock-parsing error paths, are gone along with the
//! vendored tree and the JSON parser -- their entire subject no longer
//! exists. The canonical selection vectors below were themselves vendored
//! JSON (`complete-value-selection-vectors.json`); they are real behavioural
//! coverage of `admit_selection`, so they are inlined here as Rust data
//! instead of being dropped with the file they used to live in.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use quire_spec_language::value::{DefinitionLock, SelectionRefusalCode, Trigger};

struct Accepted {
    name: &'static str,
    triggers: &'static [&'static str],
    additional_roles: &'static [&'static str],
}

struct Refused {
    name: &'static str,
    triggers: &'static [&'static str],
    additional_roles: &'static [&'static str],
    omitted_always_roles: &'static [&'static str],
    expected_code: &'static str,
}

const ACCEPTED: [Accepted; 7] = [
    Accepted {
        name: "no-trigger",
        triggers: &[],
        additional_roles: &[],
    },
    Accepted {
        name: "text-bearing",
        triggers: &["text_bearing"],
        additional_roles: &["text_profile"],
    },
    Accepted {
        name: "ieee-operation",
        triggers: &["ieee_operation"],
        additional_roles: &["ieee_profile"],
    },
    Accepted {
        name: "div-rem-euclidean",
        triggers: &["integer_div_rem"],
        additional_roles: &["integer_division_euclidean"],
    },
    Accepted {
        name: "div-rem-floor",
        triggers: &["integer_div_rem"],
        additional_roles: &["integer_division_floor"],
    },
    Accepted {
        name: "div-rem-truncating",
        triggers: &["integer_div_rem"],
        additional_roles: &["integer_division_truncating"],
    },
    Accepted {
        name: "every-trigger",
        triggers: &["ieee_operation", "integer_div_rem", "text_bearing"],
        additional_roles: &["ieee_profile", "integer_division_floor", "text_profile"],
    },
];

const REFUSED: [Refused; 12] = [
    Refused {
        name: "two-division-profiles",
        triggers: &["integer_div_rem"],
        additional_roles: &["integer_division_euclidean", "integer_division_truncating"],
        omitted_always_roles: &[],
        expected_code: "selection_alternative_conflict",
    },
    Refused {
        name: "division-missing-for-div-rem",
        triggers: &["integer_div_rem"],
        additional_roles: &[],
        omitted_always_roles: &[],
        expected_code: "selection_trigger_unsatisfied",
    },
    Refused {
        name: "division-without-trigger",
        triggers: &[],
        additional_roles: &["integer_division_floor"],
        omitted_always_roles: &[],
        expected_code: "selection_untriggered_profile",
    },
    Refused {
        name: "ieee-without-trigger",
        triggers: &[],
        additional_roles: &["ieee_profile"],
        omitted_always_roles: &[],
        expected_code: "selection_untriggered_profile",
    },
    Refused {
        name: "unicode-missing-for-text-bearing",
        triggers: &["text_bearing"],
        additional_roles: &[],
        omitted_always_roles: &[],
        expected_code: "selection_trigger_unsatisfied",
    },
    Refused {
        name: "always-role-missing",
        triggers: &[],
        additional_roles: &[],
        omitted_always_roles: &["accounting"],
        expected_code: "selection_required_missing",
    },
    Refused {
        name: "unknown-trigger",
        triggers: &["host_float"],
        additional_roles: &[],
        omitted_always_roles: &[],
        expected_code: "selection_unknown_trigger",
    },
    Refused {
        name: "duplicate-trigger",
        triggers: &["text_bearing", "text_bearing"],
        additional_roles: &["text_profile"],
        omitted_always_roles: &[],
        expected_code: "selection_duplicate_trigger",
    },
    Refused {
        name: "unknown-trigger-before-duplicate-trigger",
        triggers: &["text_bearing", "text_bearing", "host_float"],
        additional_roles: &["text_profile"],
        omitted_always_roles: &[],
        expected_code: "selection_unknown_trigger",
    },
    Refused {
        name: "duplicate-trigger-before-unknown-role",
        triggers: &["ieee_operation", "ieee_operation"],
        additional_roles: &["ieee_profile", "installed_unicode"],
        omitted_always_roles: &[],
        expected_code: "selection_duplicate_trigger",
    },
    Refused {
        name: "unknown-role",
        triggers: &[],
        additional_roles: &["installed_unicode"],
        omitted_always_roles: &[],
        expected_code: "selection_unknown_role",
    },
    Refused {
        name: "duplicate-role",
        triggers: &["text_bearing"],
        additional_roles: &["text_profile", "text_profile"],
        omitted_always_roles: &[],
        expected_code: "selection_duplicate_role",
    },
];

#[trace("TC-192")]
#[test]
fn canonical_package_selection_vectors_admit_and_refuse_exactly() {
    let lock = DefinitionLock::pinned();
    let always: Vec<_> = lock
        .always_roles()
        .iter()
        .map(|role| role.as_str())
        .collect();

    let accepted: BTreeSet<_> = ACCEPTED.iter().map(|v| v.name).collect();
    assert_eq!(
        accepted,
        BTreeSet::from([
            "no-trigger",
            "text-bearing",
            "ieee-operation",
            "div-rem-euclidean",
            "div-rem-floor",
            "div-rem-truncating",
            "every-trigger",
        ])
    );
    for vector in &ACCEPTED {
        let mut roles = always.clone();
        roles.extend(vector.additional_roles.iter().copied());
        let admitted = lock
            .admit_selection(vector.triggers, &roles)
            .unwrap_or_else(|code| panic!("{}: {code:?}", vector.name));
        let triggers: BTreeSet<_> = vector
            .triggers
            .iter()
            .map(|code| Trigger::from_code(code).unwrap())
            .collect();
        assert_eq!(admitted.triggers(), &triggers, "{}", vector.name);
        assert_eq!(admitted.roles().len(), roles.len(), "{}", vector.name);
        assert_eq!(
            admitted.division_profile().is_some(),
            triggers.contains(&Trigger::IntegerDivRem),
            "{}",
            vector.name
        );
    }

    let refused: BTreeSet<_> = REFUSED.iter().map(|v| v.name).collect();
    assert_eq!(
        refused,
        BTreeSet::from([
            "two-division-profiles",
            "division-missing-for-div-rem",
            "division-without-trigger",
            "ieee-without-trigger",
            "unicode-missing-for-text-bearing",
            "always-role-missing",
            "unknown-trigger",
            "duplicate-trigger",
            "unknown-trigger-before-duplicate-trigger",
            "duplicate-trigger-before-unknown-role",
            "unknown-role",
            "duplicate-role",
        ])
    );
    let mut codes = BTreeSet::new();
    for vector in &REFUSED {
        let mut roles: Vec<_> = always
            .iter()
            .copied()
            .filter(|role| {
                !vector
                    .omitted_always_roles
                    .iter()
                    .any(|omitted| omitted == role)
            })
            .collect();
        roles.extend(vector.additional_roles.iter().copied());
        let expected = SelectionRefusalCode::from_code(vector.expected_code).unwrap();
        assert_eq!(
            lock.admit_selection(vector.triggers, &roles),
            Err(expected),
            "{}",
            vector.name
        );
        codes.insert(expected);
    }
    assert_eq!(codes, SelectionRefusalCode::ALL.into_iter().collect());
}
