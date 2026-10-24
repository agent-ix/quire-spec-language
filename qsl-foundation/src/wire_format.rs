// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-024–027: one catalog for native source, input, package and command formats.

use serde::{Serialize, Serializer};

macro_rules! formats {
    ($( $(#[$doc:meta])* $variant:ident => $name:literal ),+ $(,)?) => {
        /// Exact native format selector; profiles and cost models are separate contracts.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        #[non_exhaustive]
        pub enum WireFormat { $( $(#[$doc])* $variant, )+ }

        impl WireFormat {
            /// All native formats named by this catalog, independent of a command's admission.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// Stable byte spelling used by producers and consumers.
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $name),+ }
            }
        }
    };
}

formats! {
    /// File-driven native execution request.
    RunRequest => "native-run/1",
    /// Source-only native compilation request.
    CompileRequest => "native-compile/1",
    /// Native clause-run result or command-error envelope.
    RunResult => "native-run-result/2",
    /// Source-aware rule-model authoring profile.
    RuleModel => "native-rule-model/1",
    /// Rule-model authoring profile with exact IR rational scalar roles.
    RuleModelV2 => "native-rule-model/2",
    /// Selected snapshot or invocation artifact.
    RuntimeInput => "native-state-input/1",
    /// Linked native package artifact.
    LinkedPackage => "native-linked-package/1",
    /// FR-100: the spine `run` outcome document for a `1-draft` program's
    /// named function call.
    SpineRunResult => "spine-run-result/1",
}

impl core::fmt::Display for WireFormat {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for WireFormat {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
