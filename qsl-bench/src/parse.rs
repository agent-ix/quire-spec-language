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
}

/// Bytes fed to SHA-256 while building one CST's node identities, split by
/// where they come from.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IdentityHashBytes {
    /// Every preimage byte: [`Self::source_slices`], [`Self::ancestor_paths`]
    /// and the fixed per-node and per-document framing.
    pub total: u64,
    /// Bytes of each node's own source slice, summed over nodes: a byte at
    /// nesting depth `d` is hashed once per enclosing node.
    pub source_slices: u64,
    /// Bytes of each node's ancestor production names, summed over nodes.
    pub ancestor_paths: u64,
}

/// Bytes fed to SHA-256 while building `parsed`'s CST identities, computed
/// from the CST's public node spans and ancestor lists by the preimage
/// layout `qsl_cst::cst::LosslessCst::new` uses at `89326999`: one
/// document-revision preimage, then one `quire.complete.cst-node/1`
/// preimage per node carrying the source identity, the node's production
/// name, **the node's whole source slice**, every ancestor production name
/// and a 4-byte occurrence ordinal.
///
/// This is a model of that code, not an instrumented count: it goes stale
/// when `LosslessCst::new`'s preimage changes, and the probe says so where
/// it prints the figure. Timing (`benches/cst.rs`) is the measured
/// counterpart.
pub fn cst_identity_hashed_bytes(parsed: &ParsedSource) -> IdentityHashBytes {
    let widen = |bytes: usize| u64::try_from(bytes).unwrap_or(u64::MAX);
    let identity = parsed.source().identity();
    let digest_text = parsed.source().digest().to_string();
    let revision = b"quire.complete.document-revision/1\0".len()
        + [
            identity.identity.len(),
            identity.revision.len(),
            digest_text.len(),
        ]
        .iter()
        .map(|length| 8 + length)
        .sum::<usize>();
    let mut counted = IdentityHashBytes {
        total: widen(revision),
        ..IdentityHashBytes::default()
    };
    for node in parsed.cst().nodes() {
        let span = node.span();
        let slice = span.end.saturating_sub(span.start);
        let ancestors: usize = node
            .identity()
            .ancestor_productions
            .iter()
            .map(|production| format!("{production:?}/").len())
            .sum();
        let framing = b"quire.complete.cst-node/1\0".len()
            + identity.identity.len()
            + format!("|{:?}|", node.production()).len()
            + 1
            + 1
            + 4;
        counted.source_slices += widen(slice);
        counted.ancestor_paths += widen(ancestors);
        counted.total += widen(framing + slice + ancestors);
    }
    counted
}
