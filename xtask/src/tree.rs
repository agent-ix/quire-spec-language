// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL #138: the closed set of vendored resource trees this command knows.
use crate::error::Error;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tree {
    NativeV1,
    CompleteValue,
}

impl Tree {
    pub const ALL: [Self; 2] = [Self::NativeV1, Self::CompleteValue];

    pub fn dir_name(self) -> &'static str {
        match self {
            Self::NativeV1 => "native-v1",
            Self::CompleteValue => "complete-value",
        }
    }

    pub fn from_arg(text: &str) -> Result<Self, Error> {
        match text {
            "native-v1" => Ok(Self::NativeV1),
            "complete-value" => Ok(Self::CompleteValue),
            _ => Err(Error::Usage(
                "--tree must be native-v1, complete-value or all",
            )),
        }
    }

    /// `resources/<tree>` under the given workspace root.
    pub fn root(self, workspace_root: &Path) -> PathBuf {
        workspace_root.join("resources").join(self.dir_name())
    }

    /// `resources/<tree>/VENDOR.json` under the given workspace root.
    pub fn manifest_path(self, workspace_root: &Path) -> PathBuf {
        self.root(workspace_root).join("VENDOR.json")
    }
}
