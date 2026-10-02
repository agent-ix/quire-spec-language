// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-108 (TC-469): the ConfigVersion spine unit text and the domain
//! package it selects. `spine.rs` builds its spine files from these, and
//! `qsl-package`'s emission-to-admission corpus (FR-093-AC-19) emits the
//! same unit, so both read one datum. Nothing here depends on a crate.

/// `example/config-version`'s Semantic IR 2.0.0 domain package (FR-103-AC-1
/// declarations: `ConfigVersion` with `versionNumber: Int[0, 1000]` and
/// optional `parent`, population `config_history`, operation `attemptUpdate`
/// with frame `modifies [versionNumber]`). AGPL-3.0-only, like `model.json`
/// and the generated unit below: one example, one licence (its own
/// `model.semantic-ir.json.license` sidecar carries the SPDX header, since
/// the Semantic IR 2.0.0 schema (`agent-ix-semantic-ir`) forbids an extra
/// top-level member such as `license` -- confirmed against the pinned
/// `filament-core-data` revision's `schema.rs::semantic_ir`, which
/// `forbid_extra`s every member outside its own closed `IR_OPTIONAL_MEMBERS`
/// list).
pub const DOMAIN_PACKAGE: &str = include_str!("model.semantic-ir.json");

/// The package identity FR-108's own unit text selects: `model Config =
/// "example/config-version" version "1" digest ...`. The version a `model`
/// statement names must equal the domain package's own declared
/// `package.version` (I1 admission, `WrongModelSelection`) -- confirmed by
/// running this fixture: FR-108's prose "`1`" is illustrative, not the
/// package's real semver, so [`unit_text`] below spells the version this
/// package actually declares, `1.0.0`.
pub const PACKAGE_IDENTITY: &str = "example/config-version";

/// The domain package's own declared `package.version`.
pub const PACKAGE_VERSION: &str = "1.0.0";

/// FR-108's own `1-draft` unit text, selecting [`DOMAIN_PACKAGE`] by
/// `model_digest_hex`, its `sha256-jcs` digest: one fixed unit, shared by
/// every case (the corpus table selects different clauses and functions
/// *within* it, never a different unit).
pub fn unit_text(model_digest_hex: &str) -> String {
    format!(
        "// SPDX-License-Identifier: AGPL-3.0-only\n\
         language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model Config = {PACKAGE_IDENTITY:?} version {PACKAGE_VERSION:?} digest \"sha256-jcs:{model_digest_hex}\";\n\
         invariant ParentOrder using v on Config::ConfigVersion at current {{ \
         present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }}\n\
         invariant NoCycle using v on Config::ConfigVersion at current {{ \
         not reaches(self, self, parent) }}\n\
         post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate {{ \
         self.versionNumber = pre(self.versionNumber) }}\n\
         function sameIdentity using v(a: Config::ConfigVersion, b: Config::ConfigVersion): \
         Boolean pure {{ a = b }}\n",
    )
}
