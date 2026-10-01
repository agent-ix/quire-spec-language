// SPDX-License-Identifier: AGPL-3.0-or-later
//! The call-admission refusal (ADR-011 §6.1 layer SV): a runtime input a
//! call or evaluation refuses before any charge. `qsl-eval`'s admission
//! code produces it; a backend admitting arguments to checked code refuses
//! the same way.
//!
//! The F-layer diagnostic `Code` each refusal carries is mapped above this
//! leaf, in `qsl-eval` (`input_refusal_code`): this crate depends on
//! `quire-exact` only, so it names the closed cause tag ([`InputRefusal::cause`])
//! and leaves the catalog code to the layer that owns the catalog.

use alloc::string::String;

/// A runtime input a call or evaluation refuses before any charge.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum InputRefusal {
    /// No function of this name: `missing_declaration` / `missing-name`.
    #[error("no function named {0}")]
    UnknownFunction(String),
    /// The argument count differs from the parameter count:
    /// `invalid_runtime_input` / `wrong-value-kind`.
    #[error("{supplied} arguments for {declared} parameters")]
    Arity {
        /// Declared parameters.
        declared: usize,
        /// Supplied arguments.
        supplied: usize,
    },
    /// An argument is not a value of its parameter type:
    /// `invalid_runtime_input` / `wrong-value-kind`.
    #[error("argument {parameter} is not a value of its declared type")]
    WrongValueKind {
        /// The parameter index.
        parameter: usize,
    },
    /// An argument holds a reference with no object in the complete
    /// population: `dangling_reference` / `absent-target-in-complete-population`.
    #[error("argument {parameter} holds a reference with no target object")]
    DanglingReference {
        /// The parameter index.
        parameter: usize,
    },
    /// FR-107: `evaluate_clause`'s name resolves to no state clause of this
    /// package: `missing_declaration` / `missing-name`.
    #[error("no state clause named {0}")]
    UnknownClause(String),
    /// FR-107: `evaluate_clause`'s observations were admitted for a
    /// different clause than the one named, or (FR-115) `evaluate_frame`'s
    /// invocation for a different frame: `invalid_runtime_input` /
    /// `wrong-role-mapping`.
    #[error("the observations were admitted for another clause or frame")]
    ObservationsMismatch,
}

impl InputRefusal {
    /// The closed cause tag.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::UnknownFunction(_) | Self::UnknownClause(_) => "missing-name",
            Self::Arity { .. } | Self::WrongValueKind { .. } => "wrong-value-kind",
            Self::DanglingReference { .. } => "absent-target-in-complete-population",
            Self::ObservationsMismatch => "wrong-role-mapping",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each refusal names its closed cause tag.
    #[test]
    fn each_refusal_names_its_cause() {
        let cases = [
            (
                InputRefusal::UnknownFunction(String::from("f")),
                "missing-name",
            ),
            (
                InputRefusal::UnknownClause(String::from("c")),
                "missing-name",
            ),
            (
                InputRefusal::Arity {
                    declared: 1,
                    supplied: 2,
                },
                "wrong-value-kind",
            ),
            (
                InputRefusal::WrongValueKind { parameter: 0 },
                "wrong-value-kind",
            ),
            (
                InputRefusal::DanglingReference { parameter: 0 },
                "absent-target-in-complete-population",
            ),
            (InputRefusal::ObservationsMismatch, "wrong-role-mapping"),
        ];
        for (refusal, cause) in cases {
            assert_eq!(refusal.cause(), cause, "{refusal:?}");
        }
    }
}
