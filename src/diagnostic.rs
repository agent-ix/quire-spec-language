// SPDX-License-Identifier: AGPL-3.0-only
use crate::source::{LocatedSpan, Source, SourceIdentity, Span};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Source,
    Lex,
    Parse,
    Profile,
    Format,
    SourceMap,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Lex => "lex",
            Self::Parse => "parse",
            Self::Profile => "profile",
            Self::Format => "format",
            Self::SourceMap => "source_map",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Code {
    InvalidSourceIdentity,
    InvalidSourceMap,
    SourceDigestMismatch,
    InvalidUtf8,
    InvalidSyntax,
    UnsupportedConstruct,
    UnknownLanguage,
    UnknownEdition,
    UnknownProfile,
    ResourceExhausted,
}

impl Code {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidSourceIdentity => "invalid_source_identity",
            Self::InvalidSourceMap => "invalid_source_map",
            Self::SourceDigestMismatch => "source_digest_mismatch",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::InvalidSyntax => "invalid_syntax",
            Self::UnsupportedConstruct => "unsupported_construct",
            Self::UnknownLanguage => "unknown_language",
            Self::UnknownEdition => "unknown_edition",
            Self::UnknownProfile => "unknown_profile",
            Self::ResourceExhausted => "resource_exhausted",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub phase: Phase,
    pub code: Code,
    pub source: SourceIdentity,
    pub path: String,
    pub span: LocatedSpan,
    pub message: String,
}

impl Diagnostic {
    pub fn is_incomplete(&self) -> bool {
        self.code == Code::ResourceExhausted
    }
}

pub(crate) fn error(
    source: &Source,
    code: Code,
    phase: Phase,
    start: usize,
    end: usize,
    message: impl Into<String>,
) -> Box<Diagnostic> {
    Box::new(Diagnostic {
        code,
        phase,
        source: source.identity().clone(),
        path: source.path().into(),
        span: source
            .locate(Span { start, end })
            .expect("internal offsets are UTF-8 boundaries"),
        message: message.into(),
    })
}
