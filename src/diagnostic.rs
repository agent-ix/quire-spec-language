// SPDX-License-Identifier: AGPL-3.0-only
use crate::source::{LocatedSpan, SourceIdentity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Source,
    Lex,
    Parse,
    Profile,
    Format,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Lex => "lex",
            Self::Parse => "parse",
            Self::Profile => "profile",
            Self::Format => "format",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Code {
    InvalidSourceIdentity,
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
