// SPDX-License-Identifier: AGPL-3.0-or-later
//! Version 1 assessment-kind vocabulary for typed protocol consumers.

use serde::{Deserialize, Serialize};

/// Artifact categories accepted by the protocol assessment boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArtifactKind {
    /// An admitted model population.
    #[serde(rename = "population")]
    Population,
    /// A finite temporal observation window.
    #[serde(rename = "window")]
    Window,
}

impl ArtifactKind {
    /// Returns the exact version 1 wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Population => "population",
            Self::Window => "window",
        }
    }
}

/// Binding categories accepted by the protocol assessment boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BindingKind {
    /// A population binding.
    #[serde(rename = "population")]
    Population,
    /// A finite-window binding.
    #[serde(rename = "window")]
    Window,
    /// A clock binding.
    #[serde(rename = "clock")]
    Clock,
}

impl BindingKind {
    /// Returns the exact version 1 wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Population => "population",
            Self::Window => "window",
            Self::Clock => "clock",
        }
    }
}
