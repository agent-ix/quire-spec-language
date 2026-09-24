// SPDX-License-Identifier: AGPL-3.0-or-later
//! S1 parser inputs: complete-V1 source text for `qsl_cst::parse`, and the
//! classification of what one parse returned.

use qsl_cst::{Limits, ParsedSource};
use qsl_foundation::SourceIdentity;

/// The two-line prelude every generated source starts with: the language
/// line and the `Complete` profile selection every `function ... using
/// Complete` declaration names.
pub const PRELUDE: &str = concat!(
    "language \"ix:native\" edition \"1-draft\";\n",
    "profile Complete = \"quire.value.complete/v1\" version \"1\" digest ",
    "\"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n",
);

/// The source identity every generated source is parsed under.
pub const IDENTITY: &str = "bench:qsl-196";

/// One function whose body nests `depth` parenthesis pairs:
/// `depth = 3` is `((( x + x) + x) + x)`. `depth = 0` is the bare `x`.
pub fn nested_source(depth: usize) -> String {
    let mut body = String::from("x");
    for _ in 0..depth {
        body = format!("({body} + x)");
    }
    format!("{PRELUDE}function nested using Complete (x: Integer): Integer pure {{ {body} }}\n")
}

/// `functions` one-line functions, `f0` .. `f{functions - 1}`, each
/// `function fI using Complete (x: Integer): Integer pure { x }`.
pub fn volume_source(functions: usize) -> String {
    let mut text = String::from(PRELUDE);
    for index in 0..functions {
        text.push_str(&format!(
            "function f{index} using Complete (x: Integer): Integer pure {{ x }}\n"
        ));
    }
    text
}

/// Parse `text` under [`IDENTITY`] at the default [`Limits`].
///
/// The outer `Err` is `qsl_cst::parse`'s own fatal source or resource
/// refusal, returned as is.
pub fn parse(text: &str) -> Result<ParsedSource, Box<qsl_cst::CompleteDiagnostic>> {
    qsl_cst::parse(
        SourceIdentity {
            identity: IDENTITY.to_owned(),
            revision: "1".to_owned(),
        },
        "bench.native",
        text.as_bytes(),
        Limits::default(),
    )
}

/// What one parse returned, reduced to what the baseline records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseOutcome {
    /// Parsed with no diagnostic and no recovery.
    Admitted {
        /// CST token (leaf) count.
        tokens: usize,
        /// CST node (interior) count.
        nodes: usize,
    },
    /// Parsed, but recovered or diagnosed: never admissible.
    Recovered {
        /// The first diagnostic's stable code, if any was recorded.
        first_code: Option<&'static str>,
    },
    /// `qsl_cst::parse` refused outright.
    Refused {
        /// The refusal's stable code.
        code: &'static str,
        /// The refusal's stable cause tag.
        cause: &'static str,
        /// The refusal's human-readable detail.
        message: String,
    },
}

impl ParseOutcome {
    /// Classify one [`parse`] result.
    pub fn of(result: &Result<ParsedSource, Box<qsl_cst::CompleteDiagnostic>>) -> Self {
        match result {
            Ok(parsed) if parsed.is_admissible() => Self::Admitted {
                tokens: parsed.cst().tokens().len(),
                nodes: parsed.cst().nodes().len(),
            },
            Ok(parsed) => Self::Recovered {
                first_code: parsed
                    .diagnostics()
                    .first()
                    .map(|diagnostic| diagnostic.code.as_str()),
            },
            Err(diagnostic) => Self::Refused {
                code: diagnostic.code.as_str(),
                cause: diagnostic.cause.as_str(),
                message: diagnostic.message.clone(),
            },
        }
    }

    /// Whether the parse was admitted.
    pub fn is_admitted(&self) -> bool {
        matches!(self, Self::Admitted { .. })
    }

    /// The outcome's name in a benchmark id (`parser/depth/refused/5`), so
    /// a benchmark whose input changes from refused to admitted gets a new
    /// id instead of reading as a regression or a speed-up.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Admitted { .. } => "admitted",
            Self::Recovered { .. } => "recovered",
            Self::Refused { .. } => "refused",
        }
    }
}

/// The benchmark id for `input` under `outcome`: `<outcome>/<input>`.
pub fn outcome_id(outcome: &ParseOutcome, input: impl std::fmt::Display) -> String {
    format!("{}/{input}", outcome.label())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smallest_inputs_parse() {
        for text in [nested_source(0), volume_source(1)] {
            assert!(ParseOutcome::of(&parse(&text)).is_admitted(), "{text}");
        }
    }
}
