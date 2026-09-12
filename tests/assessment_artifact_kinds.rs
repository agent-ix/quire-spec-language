// SPDX-License-Identifier: AGPL-3.0-only

use quire_spec_language::protocol_artifact::wire::{ArtifactKind, BindingKind};
use ix_trace_rs::trace;

#[trace("TC-136", "FR-049-AC-2", "FR-049-AC-5")]
#[test]
fn population_and_window_documents_remain_distinct_from_clock_and_model_exports() {
    assert_eq!(
        serde_json::to_string(&ArtifactKind::Population).expect("serialize population kind"),
        r#""population""#
    );
    assert_eq!(
        serde_json::to_string(&ArtifactKind::Window).expect("serialize window kind"),
        r#""window""#
    );
    assert_eq!(
        serde_json::from_str::<ArtifactKind>(r#""population""#)
            .expect("read population kind"),
        ArtifactKind::Population
    );
    assert_eq!(
        serde_json::from_str::<ArtifactKind>(r#""window""#).expect("read window kind"),
        ArtifactKind::Window
    );

    assert_eq!(BindingKind::Population.as_str(), "population");
    assert_eq!(BindingKind::Window.as_str(), "window");
    assert_eq!(BindingKind::Clock.as_str(), "clock");
    assert_ne!(BindingKind::Window, BindingKind::Clock);
}
