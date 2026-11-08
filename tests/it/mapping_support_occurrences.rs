// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked temporal classifications keep node facts separate from occurrence selection.

use ix_trace_rs::trace;
use qsl_package::checked_views::{mapping_support, temporal_subject, Closure, Support, Unmatched};
use quire_contract_model::{
    CheckedNodeKind, CheckedOccurrenceRole, ClockConfiguration, TemporalForm,
};
use quire_spec_language::emit_checked_v2::{emit, ClockSelection, Request};

#[trace("TC-1322", "FR-527-AC-28")]
#[test]
fn shared_classification_preserves_occurrence_keys_and_detects_changed_public_facts() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let original = include_str!("../../examples/config-version/model.semantic-ir.json");
    let auxiliary = original.replace("example/config-version", "example/auxiliary");
    let model = directory.path().join("config.json");
    let other_model = directory.path().join("auxiliary.json");
    std::fs::write(&model, original).expect("Config model");
    std::fs::write(&other_model, &auxiliary).expect("auxiliary model");
    let source = directory.path().join("shared.native");
    std::fs::write(
        &source,
        r#"language "ix:native" edition "1-draft";
profile v = "quire.model.complete/v1";
profile t = "quire.temporal.event-position.false-extension/v1";
model Alpha = "example/config-version";
model Zeta = "example/auxiliary";
invariant Auxiliary using v on Zeta::ConfigVersion at current { self.versionNumber >= 0 }
temporal First using t over (c: Alpha::ConfigVersion) clock "events" on origin {
    eventually[0,2] holds(c.versionNumber = 2)
}
temporal Second using t over (c: Alpha::ConfigVersion) clock "events" on origin {
    eventually[0,2] holds(c.versionNumber = 2)
}
"#,
    )
    .expect("temporal source");
    let clocks =
        ["first-events", "second-events"].map(|authority| ClockConfiguration::EventPosition {
            sequence_authority: authority.into(),
        });
    let bytes = emit(&Request {
        source,
        package_name: "occurrence-classification".into(),
        models: vec![model, other_model],
        libraries: Vec::new(),
        execution_points: None,
        temporal_selections: ["First", "Second"]
            .into_iter()
            .zip(&clocks)
            .map(|(clause, clock)| ClockSelection {
                clause: clause.into(),
                clock: clock.clone(),
            })
            .collect(),
        out: None,
    })
    .expect("the real producer emits the shared clauses");
    let package =
        super::emit_checked_v2::read_back(&bytes, &[original.as_bytes(), auxiliary.as_bytes()]);
    let clauses: Vec<_> = package
        .graph()
        .nodes
        .iter()
        .zip(package.node_kinds())
        .filter(|(_, kind)| **kind == CheckedNodeKind::Temporal(TemporalForm::TemporalClause))
        .map(|(node, _)| &node.node_id)
        .collect();
    let [declaration] = clauses.as_slice() else {
        panic!("the two declarations must share one clause: {clauses:?}");
    };
    let view = temporal_subject(&package, declaration).expect("checked subject");
    let classification = mapping_support(&view, Closure::Complete).expect("classification");
    assert_eq!(&classification.retained.declaration, *declaration);
    assert_eq!(classification.retained.name, None);
    assert_eq!(view.name(), None);
    assert_eq!(view.clock(), None);
    assert_eq!(view.occurrences().len(), 2);
    for (name, expected_clock) in ["First", "Second"].into_iter().zip(&clocks) {
        let occurrence = view
            .occurrences()
            .iter()
            .find(|occurrence| view.name_of(occurrence) == Some(name))
            .expect("the selected original occurrence has its own name");
        assert_eq!(&occurrence.node, *declaration);
        assert_eq!(occurrence.role, CheckedOccurrenceRole::Claim);
        let clock = view.clock_of(occurrence).expect("the occurrence clock");
        assert_eq!(&clock.declaration, *declaration);
        assert_eq!(clock.occurrence, occurrence.ordinal);
        assert_eq!(&clock.clock, expected_clock);
        let mut absent = occurrence.clone();
        absent.ordinal = u64::MAX;
        assert_eq!(view.name_of(&absent), None);
        assert_eq!(view.clock_of(&absent), None);
        let mut wrong_role = occurrence.clone();
        wrong_role.role = CheckedOccurrenceRole::Generated;
        assert_eq!(view.name_of(&wrong_role), None);
        assert_eq!(view.clock_of(&wrong_role), None);
        let mut wrong_declaration = occurrence.clone();
        wrong_declaration.node.ordinal = u64::MAX;
        assert_eq!(view.name_of(&wrong_declaration), None);
        assert_eq!(view.clock_of(&wrong_declaration), None);
    }
    assert_eq!(
        package.selected_model_packages().collect::<Vec<_>>(),
        ["example/auxiliary", "example/config-version"],
        "owner identity order is independent of authored aliases and input order"
    );
    assert!(matches!(classification.support, Support::Supported { .. }));
    let mut changed_support = classification.clone();
    changed_support.support = Support::Unsupported {
        dimensions: vec![Unmatched::TimedProfile],
    };
    let mut changed_premises = classification.clone();
    let Support::Supported { premises, .. } = &mut changed_premises.support else {
        panic!("the fixture must exercise a supported mapping");
    };
    *premises = &[];
    let mut changed_name = classification.clone();
    changed_name.retained.name = Some("First".into());
    for changed in [changed_support, changed_premises, changed_name] {
        assert_eq!(changed.selection(), classification.selection());
        assert!(changed.belongs_to(&package));
        assert_ne!(
            changed,
            mapping_support(&view, Closure::Complete).expect("fresh reclassification"),
            "sealed selection does not authenticate mutable public facts"
        );
    }
}
