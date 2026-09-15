// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-033: one target-name catalog for library parsing, display and CLI selection.

/// A requested lowering target is outside the published catalog.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("unknown lowering target: {name}")]
pub struct UnknownProjectionTarget {
    name: Box<str>,
}

impl UnknownProjectionTarget {
    /// The rejected target spelling.
    pub fn name(&self) -> &str {
        &self.name
    }
}

macro_rules! targets {
    ($($(#[$doc:meta])* $variant:ident => $name:literal),+ $(,)?) => {
        /// Explicit lowering domain; IR binding and backend acceptance are separate.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        #[non_exhaustive]
        pub enum ProjectionTarget {
            $($(#[$doc])* $variant),+
        }

        impl ProjectionTarget {
            /// Every published target, in stable help-display order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// Stable spelling shared by parsing, display and failure reports.
            pub const fn name(self) -> &'static str {
                match self { $(Self::$variant => $name),+ }
            }
        }
    };
}

targets! {
    /// The existing generated Boolean backend's qualified domain.
    BooleanOracleV1 => "boolean-oracle/v1",
    /// Bounded primitive integer expressions for the strict IR binder.
    IntegerIrV1 => "integer-ir/v1",
    /// Primitive context fields and pre/post inputs, retaining native object authority.
    StateScalarIrV1 => "state-scalar-ir/v1",
}

impl std::fmt::Display for ProjectionTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

impl std::str::FromStr for ProjectionTarget {
    type Err = UnknownProjectionTarget;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|target| target.name() == name)
            .ok_or_else(|| UnknownProjectionTarget { name: name.into() })
    }
}
