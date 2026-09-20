// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL #138: the closed set of vendored resource trees this command knows.
use crate::error::Error;
use qsl_attrs::string_edge;
use std::path::{Path, PathBuf};

/// One vendored resource tree this command knows how to revendor or check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tree {
    /// `resources/native-v1`, the native-syntax historical selection.
    NativeV1,
    /// `resources/complete-value`, the complete-value historical selection.
    CompleteValue,
    /// `tests/fixtures/architecture`, vendored from FCD's own
    /// `crates/extraction-frontend/fixtures/architecture` (QSL #131 PR 3).
    TestFixturesArchitecture,
    /// `tests/fixtures/modules`, vendored from FCD's own
    /// `crates/extraction-frontend/fixtures/modules` (QSL #131 PR 3).
    TestFixturesModules,
}

impl Tree {
    /// Every known tree, in the order `--tree all` processes them.
    pub const ALL: [Self; 4] = [
        Self::NativeV1,
        Self::CompleteValue,
        Self::TestFixturesArchitecture,
        Self::TestFixturesModules,
    ];

    /// This tree's `--tree`/directory-name spelling.
    pub fn dir_name(self) -> &'static str {
        match self {
            Self::NativeV1 => "native-v1",
            Self::CompleteValue => "complete-value",
            Self::TestFixturesArchitecture => "test-fixtures-architecture",
            Self::TestFixturesModules => "test-fixtures-modules",
        }
    }

    /// FR-064's own edge function shape: one total conversion from a CLI
    /// `--tree` value to this closed enum, or a typed refusal.
    #[string_edge]
    pub fn from_arg(text: &str) -> Result<Self, Error> {
        match text {
            "native-v1" => Ok(Self::NativeV1),
            "complete-value" => Ok(Self::CompleteValue),
            "test-fixtures-architecture" => Ok(Self::TestFixturesArchitecture),
            "test-fixtures-modules" => Ok(Self::TestFixturesModules),
            _ => Err(Error::Usage(
                "--tree must be native-v1, complete-value, test-fixtures-architecture, \
                 test-fixtures-modules or all",
            )),
        }
    }

    /// The vendored tree's own root under the given workspace root:
    /// `resources/<tree>` for the two resource trees, `tests/fixtures/<name>`
    /// for the two test-fixture trees.
    pub fn root(self, workspace_root: &Path) -> PathBuf {
        match self {
            Self::NativeV1 | Self::CompleteValue => {
                workspace_root.join("resources").join(self.dir_name())
            }
            Self::TestFixturesArchitecture => workspace_root
                .join("tests")
                .join("fixtures")
                .join("architecture"),
            Self::TestFixturesModules => workspace_root
                .join("tests")
                .join("fixtures")
                .join("modules"),
        }
    }

    /// `<tree root>/VENDOR.json` under the given workspace root.
    pub fn manifest_path(self, workspace_root: &Path) -> PathBuf {
        self.root(workspace_root).join("VENDOR.json")
    }
}
