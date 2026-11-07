// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-513-AC-16: versioned assessment kind spellings.

use ix_trace_rs::trace;
use qsl_package::protocol_artifact_v1::{ArtifactKind, BindingKind};

#[trace("TC-136", "FR-513-AC-16")]
#[test]
fn v1_assessment_kinds_keep_the_closed_wire_vocabulary() {
    for (kind, spelling) in [
        (ArtifactKind::Population, "population"),
        (ArtifactKind::Window, "window"),
    ] {
        assert_eq!(kind.as_str(), spelling);
        assert_eq!(
            serde_json::to_string(&kind).unwrap(),
            format!("\"{spelling}\"")
        );
        assert_eq!(
            serde_json::from_str::<ArtifactKind>(&format!("\"{spelling}\""))
                .expect("the exact v1 artifact kind is admitted"),
            kind
        );
    }

    for (kind, spelling) in [
        (BindingKind::Population, "population"),
        (BindingKind::Window, "window"),
        (BindingKind::Clock, "clock"),
    ] {
        assert_eq!(kind.as_str(), spelling);
        assert_eq!(
            serde_json::to_string(&kind).unwrap(),
            format!("\"{spelling}\"")
        );
        assert_eq!(
            serde_json::from_str::<BindingKind>(&format!("\"{spelling}\""))
                .expect("the exact v1 binding kind is admitted"),
            kind
        );
    }

    for unknown in ["observation", "model", ""] {
        assert!(serde_json::from_str::<ArtifactKind>(&format!("\"{unknown}\"")).is_err());
        assert!(serde_json::from_str::<BindingKind>(&format!("\"{unknown}\"")).is_err());
    }

    assert_ne!(ArtifactKind::Population, ArtifactKind::Window);
    assert_ne!(BindingKind::Window, BindingKind::Clock);
}
