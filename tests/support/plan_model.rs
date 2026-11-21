// SPDX-License-Identifier: AGPL-3.0-or-later
//! A small authored ConfigVersion model for source planning requests.

const MODEL: &str = r#"language "ix:native" edition "1-draft";
domain "example/config-version";
type VersionNumber = Int[0, 3];
object ConfigVersion {
  versionNumber: VersionNumber;
  parent: ConfigVersion?;
  operation attemptUpdate(): Boolean modifies self.versionNumber;
}
population config_history: ConfigVersion;
"#;

/// Compile the planning model through the admitted model-unit input route.
/// The snapshot's two objects and parent reference use the same declaration
/// identities as the spine; their version domain is actually bounded to four
/// values, rather than filtering a larger domain with an invariant.
pub(crate) fn document() -> Vec<u8> {
    let source = fsl_cst::diagnostic::read_source(
        fsl_foundation::SourceIdentity::new("test", "planning-model"),
        "planning-model.native",
        MODEL.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .expect("the authored planning model source is admitted");
    let packages = fsl_semantics::model::intake::package_input_from_sources([
        ("example/config-version", &source),
    ])
    .expect("the small planning model compiles to package input");
    packages["example/config-version"].clone()
}
