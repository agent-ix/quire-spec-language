// SPDX-License-Identifier: AGPL-3.0-or-later
//! One bounded interpreter for the declarative complete-V1 grammar.
use std::collections::BTreeSet;

use logos::Logos;

use super::cst::{
    self, CstElement, LosslessCst, Production, RawNode, Recovery, RecoveryKind, TokenClass,
    TokenKind,
};
use super::diagnostic::{CompleteCause, HostCause};
use super::grammar::{self, Grammar, Rule, Terminal};
use super::{CompleteCode, CompleteDiagnostic, ParsedSource};
use crate::lexer::Limits;
use crate::token::{Kind, LexError};
use qsl_foundation::selection::{
    DefinitionDigest, DefinitionRef, ImportSelection, InvalidDefinitionComponent,
    InvalidModelComponent, ModelDigest, ModelRef, ModelSelection, ProfileSelection,
    SourceSelections,
};
use qsl_foundation::{Phase, Source, Span};

#[derive(Clone, Debug)]
struct Significant {
    kind: Option<Kind>,
    span: Span,
    spelling: Box<str>,
}

type Scan = (
    Vec<super::CstToken>,
    Vec<Significant>,
    Vec<CompleteDiagnostic>,
    Vec<Recovery>,
);

#[derive(Clone, Debug)]
struct MatchNode {
    production: Production,
    start: usize,
    end: usize,
    children: Vec<MatchNode>,
    node_count: usize,
}

#[derive(Clone, Debug)]
struct RuleMatch {
    end: usize,
    children: Vec<MatchNode>,
    node_count: usize,
}

#[derive(Clone, Debug)]
struct Failure {
    position: usize,
    expected: BTreeSet<String>,
}

impl Failure {
    fn expected(position: usize, expected: String) -> Self {
        Self {
            position,
            expected: BTreeSet::from([expected]),
        }
    }

    fn merge(mut self, other: Self) -> Self {
        match other.position.cmp(&self.position) {
            std::cmp::Ordering::Greater => other,
            std::cmp::Ordering::Equal => {
                self.expected.extend(other.expected);
                self
            }
            std::cmp::Ordering::Less => self,
        }
    }
}

enum Attempt<T> {
    Match(T),
    No(Failure),
    Exhausted(ExhaustedKind),
}

/// Which ceiling a resource_exhausted refusal names (NFR-001 "Nesting
/// level" and "Verification": a work-budget refusal must name the work
/// limit, never nesting, and a nesting refusal must name nesting depth, the
/// ceiling and the opening bracket's span).
#[derive(Clone, Copy, Debug)]
enum ExhaustedKind {
    /// The interpreter step budget (`Engine::maximum_steps`) was exceeded.
    Steps,
    /// The retained syntax-node ceiling (`Limits::nodes`) was exceeded.
    Nodes { bound: usize },
    /// The opening bracket of pair `bound + 1` was reached: `span` is that
    /// bracket's own span, not the farthest token scanned.
    Nesting { bound: usize, span: Span },
}

pub(super) fn parse(
    source: Source,
    limits: Limits,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    let grammar = grammar::complete_v1();
    let compounds = grammar::complete_compound_spellings(&grammar);
    let (tokens, significant, mut diagnostics, mut recoveries) = scan(&source, limits, &compounds)?;
    let prelude_nodes = selection_prelude_nodes(&source, &grammar, &significant, limits);
    let matched = if diagnostics.is_empty() {
        let mut engine = Engine::new(&grammar, &significant, limits);
        match engine.production(Production::CompleteUnit, 0, 0) {
            Attempt::Match(node) => Some(node),
            Attempt::No(failure) => {
                let span = significant.get(failure.position).map_or(
                    Span {
                        start: source.text().len(),
                        end: source.text().len(),
                    },
                    |token| token.span,
                );
                let (code, cause) = selection_code(&significant, failure.position).map_or(
                    (
                        CompleteCode::InvalidSyntax,
                        if failure.position == significant.len() {
                            CompleteCause::UnexpectedEnd
                        } else {
                            CompleteCause::UnexpectedToken
                        },
                    ),
                    |code| (code, CompleteCause::UnsupportedSelection),
                );
                let expected = failure
                    .expected
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(" or ");
                diagnostics.push(*super::diagnostic::error(
                    &source,
                    code,
                    cause,
                    if code == CompleteCode::InvalidSyntax {
                        Phase::Parse
                    } else {
                        Phase::Profile
                    },
                    span.start,
                    span.end,
                    format!("expected {expected}"),
                ));
                recoveries.push(Recovery {
                    kind: if failure.position == significant.len() {
                        RecoveryKind::Insert
                    } else {
                        RecoveryKind::Delete
                    },
                    span,
                    expected,
                });
                None
            }
            Attempt::Exhausted(kind) => {
                let farthest_span = || {
                    significant.get(engine.farthest).map_or(
                        Span {
                            start: source.text().len(),
                            end: source.text().len(),
                        },
                        |token| token.span,
                    )
                };
                let (span, message) = match kind {
                    ExhaustedKind::Steps => (
                        farthest_span(),
                        format!(
                            "complete grammar work budget exhausted: bound {} steps",
                            engine.maximum_steps
                        ),
                    ),
                    ExhaustedKind::Nodes { bound } => (
                        farthest_span(),
                        format!("syntax node budget exhausted: bound {bound} nodes"),
                    ),
                    ExhaustedKind::Nesting { bound, span } => (
                        span,
                        format!("nesting depth exceeds the ceiling of {bound} levels"),
                    ),
                };
                return Err(super::diagnostic::error(
                    &source,
                    CompleteCode::ResourceExhausted,
                    CompleteCause::InsufficientNextCharge,
                    Phase::Parse,
                    span.start,
                    span.end,
                    message,
                ));
            }
        }
    } else {
        None
    };

    let (nodes, root) = if let Some(node) = matched {
        lower_tree(&source, &significant, node)
    } else {
        (
            vec![RawNode {
                production: Production::CompleteUnit,
                span: Span {
                    start: 0,
                    end: source.text().len(),
                },
                children: Vec::new(),
            }],
            0,
        )
    };
    let selection_nodes = if nodes.iter().any(|node| {
        matches!(
            node.production,
            Production::Profile | Production::ImportDeclaration | Production::Model
        )
    }) {
        &nodes
    } else {
        &prelude_nodes
    };
    let selections = extract_selections(&source, &significant, selection_nodes, &mut diagnostics);
    let cst = LosslessCst::new(source.clone(), tokens, nodes, root, recoveries);
    Ok(ParsedSource::from_parts(
        source,
        cst,
        diagnostics,
        selections,
        false,
    ))
}

fn selection_prelude_nodes(
    source: &Source,
    grammar: &Grammar,
    tokens: &[Significant],
    limits: Limits,
) -> Vec<RawNode> {
    fn raw(source: &Source, tokens: &[Significant], node: MatchNode) -> RawNode {
        let start = tokens
            .get(node.start)
            .map_or(source.text().len(), |token| token.span.start);
        let end = node
            .end
            .checked_sub(1)
            .and_then(|index| tokens.get(index))
            .map_or(start, |token| token.span.end);
        RawNode {
            production: node.production,
            span: Span { start, end },
            children: Vec::new(),
        }
    }

    let mut engine = Engine::new(grammar, tokens, limits);
    let Attempt::Match(header) = engine.production(Production::Header, 0, 0) else {
        return Vec::new();
    };
    let mut position = header.end;
    let mut output = Vec::new();
    for (production, required) in [
        (Production::Profile, true),
        (Production::ImportDeclaration, false),
        (Production::Model, false),
    ] {
        let mut count = 0_usize;
        while let Attempt::Match(node) = engine.production(production, position, 0) {
            if node.end <= position {
                break;
            }
            position = node.end;
            output.push(raw(source, tokens, node));
            count += 1;
        }
        if required && count == 0 {
            return Vec::new();
        }
    }
    output
}

fn extract_selections(
    source: &Source,
    tokens: &[Significant],
    nodes: &[RawNode],
    diagnostics: &mut Vec<CompleteDiagnostic>,
) -> SourceSelections {
    struct SelectionCapture<'a> {
        alias: Option<&'a Significant>,
        identity: &'a Significant,
        version: &'a Significant,
        digest: &'a Significant,
    }

    fn capture<'a>(
        production: Production,
        selected: &'a [Significant],
    ) -> Option<SelectionCapture<'a>> {
        let schema = grammar::selection_schema(production)?;
        let mut cursor = 0_usize;
        let mut alias = None;
        let mut identity = None;
        let mut version = None;
        let mut digest = None;
        for part in schema {
            use grammar::SelectionPart;
            match part {
                SelectionPart::Exact(expected) => {
                    let token = selected.get(cursor)?;
                    if token.spelling.as_ref() != *expected {
                        return None;
                    }
                    cursor += 1;
                }
                SelectionPart::Alias => {
                    alias = Some(selected.get(cursor)?);
                    cursor += 1;
                }
                SelectionPart::Identity => {
                    identity = Some(selected.get(cursor)?);
                    cursor += 1;
                }
                SelectionPart::Version => {
                    version = Some(selected.get(cursor)?);
                    cursor += 1;
                }
                SelectionPart::Digest => {
                    digest = Some(selected.get(cursor)?);
                    cursor += 1;
                }
                SelectionPart::OptionalAlias => {
                    if selected
                        .get(cursor)
                        .is_some_and(|token| token.spelling.as_ref() == "as")
                    {
                        alias = Some(selected.get(cursor + 1)?);
                        cursor += 2;
                    }
                }
            }
        }
        (cursor == selected.len()).then_some(SelectionCapture {
            alias,
            identity: identity?,
            version: version?,
            digest: digest?,
        })
    }

    fn text(token: &Significant) -> Option<&str> {
        match &token.kind {
            Some(Kind::Text(value)) => Some(value),
            _ => None,
        }
    }

    struct InvalidDefinition<'a> {
        token: &'a Significant,
        cause: HostCause,
        message: &'static str,
    }

    fn definition<'a>(
        identity: &'a Significant,
        version: &'a Significant,
        digest: &'a Significant,
    ) -> Result<DefinitionRef, InvalidDefinition<'a>> {
        let invalid_identity = || InvalidDefinition {
            token: identity,
            cause: HostCause::SelectionIdentity,
            message: "profile or import identity must be non-empty and at most 512 bytes",
        };
        let invalid_version = || InvalidDefinition {
            token: version,
            cause: HostCause::SelectionVersion,
            message: "profile or import version must be non-empty and at most 256 bytes",
        };
        let invalid_digest = || InvalidDefinition {
            token: digest,
            cause: HostCause::SelectionDigest,
            message: "profile or import digest must be canonical SHA-256",
        };
        let identity_value = text(identity).ok_or_else(invalid_identity)?;
        let version_value = text(version).ok_or_else(invalid_version)?;
        let invalid_component = |component| match component {
            InvalidDefinitionComponent::Identity => invalid_identity(),
            InvalidDefinitionComponent::Version => invalid_version(),
        };
        // Components first, so an invalid identity or version is located
        // before the digest is read.
        DefinitionRef::validate_components(identity_value, version_value)
            .map_err(invalid_component)?;
        let digest_value = DefinitionDigest::parse(text(digest).ok_or_else(invalid_digest)?)
            .map_err(|_| invalid_digest())?;
        DefinitionRef::new(identity_value, version_value, digest_value).map_err(invalid_component)
    }

    fn model<'a>(
        identity: &'a Significant,
        version: &'a Significant,
        digest: &'a Significant,
    ) -> Result<ModelRef, InvalidDefinition<'a>> {
        let invalid_identity = || InvalidDefinition {
            token: identity,
            cause: HostCause::SelectionIdentity,
            message: "compiled-model identity must be non-empty and at most 512 bytes",
        };
        let invalid_version = || InvalidDefinition {
            token: version,
            cause: HostCause::SelectionVersion,
            message: "compiled-model version must be non-empty and at most 256 bytes",
        };
        let invalid_digest = || InvalidDefinition {
            token: digest,
            cause: HostCause::SelectionDigest,
            message: "compiled-model document digest must be canonical SHA-256",
        };
        let identity_value = text(identity).ok_or_else(invalid_identity)?;
        let version_value = text(version).ok_or_else(invalid_version)?;
        let invalid_component = |component| match component {
            InvalidModelComponent::Identity => invalid_identity(),
            InvalidModelComponent::Version => invalid_version(),
        };
        // Components first, so an invalid identity or version is located
        // before the digest is read.
        ModelRef::validate_components(identity_value, version_value).map_err(invalid_component)?;
        let digest_value = ModelDigest::parse(text(digest).ok_or_else(invalid_digest)?)
            .map_err(|_| invalid_digest())?;
        ModelRef::new(identity_value, version_value, digest_value).map_err(invalid_component)
    }

    fn record_invalid(
        source: &Source,
        invalid: InvalidDefinition<'_>,
        diagnostics: &mut Vec<CompleteDiagnostic>,
    ) {
        diagnostics.push(*super::diagnostic::error(
            source,
            invalid.cause.code(),
            CompleteCause::Host(invalid.cause),
            Phase::Profile,
            invalid.token.span.start,
            invalid.token.span.end,
            invalid.message,
        ));
    }

    let mut selections = SourceSelections::default();
    for node in nodes {
        let start = tokens.partition_point(|token| token.span.end <= node.span.start);
        let end = tokens.partition_point(|token| token.span.start < node.span.end);
        let selected = &tokens[start..end];
        let Some(captured) = capture(node.production, selected) else {
            continue;
        };
        match node.production {
            Production::Profile => {
                let Some(alias) = captured.alias else {
                    continue;
                };
                match definition(captured.identity, captured.version, captured.digest) {
                    Ok(definition) => selections.profiles.push(ProfileSelection {
                        alias: alias.spelling.to_string(),
                        definition,
                        span: node.span,
                        identity_span: captured.identity.span,
                    }),
                    Err(invalid) => record_invalid(source, invalid, diagnostics),
                }
            }
            Production::ImportDeclaration => {
                match definition(captured.identity, captured.version, captured.digest) {
                    Ok(definition) => selections.imports.push(ImportSelection {
                        alias: captured.alias.map(|alias| alias.spelling.to_string()),
                        definition,
                        span: node.span,
                    }),
                    Err(invalid) => record_invalid(source, invalid, diagnostics),
                }
            }
            Production::Model => {
                let Some(alias) = captured.alias else {
                    continue;
                };
                match model(captured.identity, captured.version, captured.digest) {
                    Ok(model) => selections.models.push(ModelSelection {
                        alias: alias.spelling.to_string(),
                        model,
                        span: node.span,
                    }),
                    Err(invalid) => record_invalid(source, invalid, diagnostics),
                }
            }
            _ => {}
        }
    }
    selections
        .profiles
        .sort_by_key(|selection| selection.span.start);
    selections
        .imports
        .sort_by_key(|selection| selection.span.start);
    selections
        .models
        .sort_by_key(|selection| selection.span.start);
    selections
}

struct LeafBudget<'a> {
    source: &'a Source,
    compounds: &'a [&'static str],
    pending: Vec<Span>,
    retained: usize,
    limit: usize,
}

impl LeafBudget<'_> {
    fn token(&mut self, span: Span) -> Result<(), Box<CompleteDiagnostic>> {
        self.pending.push(span);
        self.settle(false)
    }

    fn leaf(&mut self, span: Span) -> Result<(), Box<CompleteDiagnostic>> {
        self.settle(true)?;
        self.commit(span)
    }

    fn finish(&mut self) -> Result<(), Box<CompleteDiagnostic>> {
        self.settle(true)
    }

    fn settle(&mut self, boundary: bool) -> Result<(), Box<CompleteDiagnostic>> {
        while let (Some(first), Some(last)) = (self.pending.first(), self.pending.last()) {
            let whole_span = Span {
                start: first.start,
                end: last.end,
            };
            let whole = &self.source.text()[whole_span.start..whole_span.end];
            let exact = self.compounds.contains(&whole);
            let has_longer = self
                .compounds
                .iter()
                .any(|compound| compound.len() > whole.len() && compound.starts_with(whole));
            if !boundary && has_longer {
                break;
            }
            if exact {
                self.pending.clear();
                self.commit(whole_span)?;
                continue;
            }

            let start = first.start;
            let compound_end = self
                .pending
                .iter()
                .enumerate()
                .filter(|(_, part)| {
                    self.compounds
                        .contains(&&self.source.text()[start..part.end])
                })
                .map(|(index, _)| index)
                .next_back();
            let end = compound_end.unwrap_or(0);
            let span = Span {
                start,
                end: self.pending[end].end,
            };
            self.pending.drain(..=end);
            self.commit(span)?;
        }
        Ok(())
    }

    fn commit(&mut self, span: Span) -> Result<(), Box<CompleteDiagnostic>> {
        if self.retained >= self.limit {
            return Err(super::diagnostic::error(
                self.source,
                CompleteCode::ResourceExhausted,
                CompleteCause::InsufficientNextCharge,
                Phase::Lex,
                span.start,
                span.end,
                "CST leaf budget exhausted",
            ));
        }
        self.retained += 1;
        Ok(())
    }
}

fn enforce_leaf_budget(
    source: &Source,
    limit: usize,
    compounds: &[&'static str],
) -> Result<(), Box<CompleteDiagnostic>> {
    // Count the final retained leaves before allocating one CST record per leaf.
    // `pending` is bounded by the longest grammar-owned compound spelling.
    let mut budget = LeafBudget {
        source,
        compounds,
        pending: Vec::new(),
        retained: 0,
        limit,
    };
    let mut cursor = 0;
    for (result, range) in Kind::lexer(source.text()).spanned() {
        if range.start > cursor {
            budget.leaf(Span {
                start: cursor,
                end: range.start,
            })?;
        }
        let span = Span {
            start: range.start,
            end: range.end,
        };
        match result {
            Ok(Kind::Comment) | Err(_) => budget.leaf(span)?,
            Ok(Kind::Hex(_)) => {
                budget.token(Span {
                    start: span.start,
                    end: span.start + 2,
                })?;
                for offset in 0..span.end - span.start - 2 {
                    budget.token(Span {
                        start: span.start + 2 + offset,
                        end: span.start + 3 + offset,
                    })?;
                }
            }
            Ok(_) => budget.token(span)?,
        }
        cursor = range.end;
    }
    budget.finish()?;
    if cursor < source.text().len() {
        budget.commit(Span {
            start: cursor,
            end: source.text().len(),
        })?;
    }
    Ok(())
}

fn scan(
    source: &Source,
    limits: Limits,
    compounds: &[&'static str],
) -> Result<Scan, Box<CompleteDiagnostic>> {
    enforce_leaf_budget(source, limits.tokens, compounds)?;
    let mut tokens = Vec::new();
    let mut significant = Vec::new();
    let mut diagnostics = Vec::new();
    let mut recoveries = Vec::new();
    let mut cursor = 0;
    for (result, range) in Kind::lexer(source.text()).spanned() {
        if range.start > cursor {
            let span = Span {
                start: cursor,
                end: range.start,
            };
            tokens.push(cst::token(
                TokenClass::Whitespace,
                TokenKind::Whitespace,
                span,
                &source.text().as_bytes()[cursor..range.start],
            ));
        }
        let span = Span {
            start: range.start,
            end: range.end,
        };
        let spelling = &source.text()[range.clone()];
        let (class, kind) = match result {
            Ok(Kind::Comment) => (TokenClass::Comment, Some(Kind::Comment)),
            Ok(kind) => (TokenClass::Token, Some(kind)),
            Err(reason) => {
                // The string token pattern admits no raw control character, so a
                // recognized quoted region fails JSON decoding only at an escape.
                let (cause, message) = match reason {
                    LexError::Character => (
                        CompleteCause::InvalidToken,
                        "unexpected source character or unterminated string",
                    ),
                    LexError::LeadingZero => (
                        CompleteCause::InvalidToken,
                        "integer literals cannot have leading zeros",
                    ),
                    LexError::String => (CompleteCause::InvalidEscape, "invalid JSON string"),
                };
                diagnostics.push(*super::diagnostic::error(
                    source,
                    CompleteCode::InvalidSyntax,
                    cause,
                    Phase::Lex,
                    span.start,
                    span.end,
                    message,
                ));
                recoveries.push(Recovery {
                    kind: RecoveryKind::Delete,
                    span,
                    expected: "valid complete-V1 token".into(),
                });
                (TokenClass::Invalid, None)
            }
        };
        let token_kind = match kind.as_ref() {
            Some(Kind::Comment) => TokenKind::Comment,
            Some(Kind::Identifier(_)) => TokenKind::Identifier,
            Some(Kind::Integer(_)) => TokenKind::Integer,
            Some(Kind::Text(_)) => TokenKind::Text,
            Some(Kind::Hex(_)) => TokenKind::HexPrefix,
            Some(_) => TokenKind::Grammar,
            None => TokenKind::Invalid,
        };
        if matches!(kind, Some(Kind::Hex(_))) {
            tokens.push(cst::token(
                class,
                TokenKind::HexPrefix,
                Span {
                    start: span.start,
                    end: span.start + 2,
                },
                &spelling.as_bytes()[..2],
            ));
            for (offset, byte) in spelling.as_bytes()[2..].iter().enumerate() {
                tokens.push(cst::token(
                    class,
                    TokenKind::HexDigit,
                    Span {
                        start: span.start + 2 + offset,
                        end: span.start + 3 + offset,
                    },
                    std::slice::from_ref(byte),
                ));
            }
        } else {
            tokens.push(cst::token(class, token_kind, span, spelling.as_bytes()));
        }
        if !matches!(class, TokenClass::Whitespace | TokenClass::Comment) {
            let parts = if matches!(kind, Some(Kind::Hex(_))) {
                std::iter::once(Significant {
                    kind: None,
                    span: Span {
                        start: span.start,
                        end: span.start + 2,
                    },
                    spelling: "0x".into(),
                })
                .chain(
                    spelling[2..]
                        .char_indices()
                        .map(|(offset, character)| Significant {
                            kind: None,
                            span: Span {
                                start: span.start + 2 + offset,
                                end: span.start + 3 + offset,
                            },
                            spelling: character.to_string().into(),
                        }),
                )
                .collect::<Vec<_>>()
            } else {
                vec![Significant {
                    kind,
                    span,
                    spelling: spelling.into(),
                }]
            };
            significant.extend(parts);
        }
        cursor = range.end;
    }
    if cursor < source.text().len() {
        let span = Span {
            start: cursor,
            end: source.text().len(),
        };
        tokens.push(cst::token(
            TokenClass::Whitespace,
            TokenKind::Whitespace,
            span,
            &source.text().as_bytes()[cursor..],
        ));
    }
    let tokens = coalesce_complete_cst_tokens(tokens, compounds);
    let significant = coalesce_complete_compounds(significant, compounds);
    if let Some(excess) = tokens.get(limits.tokens) {
        let span = excess.span();
        return Err(super::diagnostic::error(
            source,
            CompleteCode::ResourceExhausted,
            CompleteCause::InsufficientNextCharge,
            Phase::Lex,
            span.start,
            span.end,
            "CST leaf budget exhausted",
        ));
    }
    Ok((tokens, significant, diagnostics, recoveries))
}

fn coalesce_complete_cst_tokens(
    tokens: Vec<super::CstToken>,
    compounds: &[&'static str],
) -> Vec<super::CstToken> {
    let mut output = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        let matched = compounds
            .iter()
            .filter_map(|compound| {
                let mut spelling = Vec::new();
                let mut end = index;
                while end < tokens.len()
                    && tokens[end].class() == TokenClass::Token
                    && (end == index || tokens[end - 1].span().end == tokens[end].span().start)
                    && spelling.len() < compound.len()
                {
                    spelling.extend_from_slice(tokens[end].spelling());
                    end += 1;
                }
                (spelling == compound.as_bytes()).then_some((compound, end))
            })
            .max_by_key(|(compound, _)| compound.len());
        if let Some((compound, end)) = matched {
            output.push(cst::token(
                TokenClass::Token,
                TokenKind::Grammar,
                Span {
                    start: tokens[index].span().start,
                    end: tokens[end - 1].span().end,
                },
                compound.as_bytes(),
            ));
            index = end;
        } else {
            output.push(tokens[index].clone());
            index += 1;
        }
    }
    output
}

fn coalesce_complete_compounds(
    tokens: Vec<Significant>,
    compounds: &[&'static str],
) -> Vec<Significant> {
    let mut output = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        let matched = compounds
            .iter()
            .filter_map(|compound| {
                let mut spelling = String::new();
                let mut end = index;
                while end < tokens.len()
                    && (end == index || tokens[end - 1].span.end == tokens[end].span.start)
                    && spelling.len() < compound.len()
                {
                    spelling.push_str(&tokens[end].spelling);
                    end += 1;
                }
                (spelling == *compound).then_some((compound, end))
            })
            .max_by_key(|(compound, _)| compound.len());
        if let Some((compound, end)) = matched {
            output.push(Significant {
                kind: None,
                span: Span {
                    start: tokens[index].span.start,
                    end: tokens[end - 1].span.end,
                },
                spelling: (*compound).into(),
            });
            index = end;
        } else {
            output.push(tokens[index].clone());
            index += 1;
        }
    }
    output
}

fn selection_code(tokens: &[Significant], position: usize) -> Option<CompleteCode> {
    match position {
        1 if tokens
            .first()
            .is_some_and(|token| token.spelling.as_ref() == "language") =>
        {
            Some(CompleteCode::UnknownLanguage)
        }
        3 if tokens
            .get(2)
            .is_some_and(|token| token.spelling.as_ref() == "edition") =>
        {
            Some(CompleteCode::UnknownEdition)
        }
        _ => None,
    }
}

struct Engine<'a> {
    grammar: &'a Grammar,
    reserved_words: BTreeSet<&'static str>,
    tokens: &'a [Significant],
    limits: Limits,
    steps: usize,
    maximum_steps: usize,
    farthest: usize,
}

/// Extracts the matched value from an [`Attempt`], propagating `No`/
/// `Exhausted` out of the enclosing function (which must itself return
/// `Attempt<_>`). Used by the hand-written chain parsers below in place of
/// `?`, since `Attempt` is not `Try`-compatible `Result`.
macro_rules! attempt {
    ($expr:expr) => {
        match $expr {
            Attempt::Match(value) => value,
            Attempt::No(failure) => return Attempt::No(failure),
            Attempt::Exhausted(kind) => return Attempt::Exhausted(kind),
        }
    };
}

impl<'a> Engine<'a> {
    fn new(grammar: &'a Grammar, tokens: &'a [Significant], limits: Limits) -> Self {
        Self {
            grammar,
            reserved_words: grammar::complete_reserved_words(grammar),
            tokens,
            limits,
            steps: 0,
            // Work-budget derivation (FR item 7): `charge()` is called at
            // least once per terminal matched, per production entered and
            // per Sequence/Choice/Repeat element attempted. A single
            // significant token is charged at most once per level of the
            // declarative expression ladder it is matched under (Expression
            // through Primary is ten levels, including the three the hand
            // written `parse_expression`/`parse_implication`/`parse_unary`
            // chain parsers replace), plus a small constant for surrounding
            // Sequence/Choice bookkeeping and the keyword lookaheads those
            // chain parsers add. That worst case stays comfortably under 32
            // charges per token across the whole grammar (the expression
            // ladder is the deepest chain of productions any token passes
            // through). Doubling to 64 leaves headroom for future grammar
            // growth without letting a legitimate token/node-bounded parse
            // exhaust the work budget before it hits an actual ceiling.
            maximum_steps: limits
                .tokens
                .saturating_add(limits.nodes)
                .saturating_mul(64),
            farthest: 0,
        }
    }

    fn charge(&mut self, position: usize) -> bool {
        self.farthest = self.farthest.max(position.min(self.tokens.len()));
        self.steps = self.steps.saturating_add(1);
        self.steps <= self.maximum_steps
    }

    /// Match one terminal directly, without a `Rule` allocation. Used by the
    /// hand-written chain parsers, which need to try a keyword or symbol
    /// without going through `rule()`'s `Rule::Terminal` indirection. Only
    /// for non-bracket terminals (keywords, `=`, identifiers): the fixed
    /// `depth` of `0` passed to `terminal()` is never inspected except for
    /// `Terminal::Open`, which no caller here passes.
    fn expect_terminal(&mut self, terminal: Terminal, position: usize) -> Attempt<usize> {
        if !self.charge(position) {
            return Attempt::Exhausted(ExhaustedKind::Steps);
        }
        match self.terminal(&terminal, position, 0) {
            Attempt::Match(matched) => Attempt::Match(matched.end),
            Attempt::No(failure) => Attempt::No(failure),
            Attempt::Exhausted(kind) => Attempt::Exhausted(kind),
        }
    }

    /// Whether the token at `position` is the reserved keyword `spelling`.
    /// Used only to decide which chain-parser branch to take; the actual
    /// consumption goes through `expect_terminal` so charging and failure
    /// reporting stay uniform.
    fn peek_is(&self, position: usize, spelling: &str) -> bool {
        self.tokens
            .get(position)
            .is_some_and(|token| token.spelling.as_ref() == spelling)
    }

    fn production(
        &mut self,
        production: Production,
        position: usize,
        depth: usize,
    ) -> Attempt<MatchNode> {
        if !self.charge(position) {
            return Attempt::Exhausted(ExhaustedKind::Steps);
        }
        // NFR-001 "Nesting level": `Expression`, `Implication` and `Unary`
        // are right-recursive chains of unbounded length (`let … in`/
        // `if … else`, right-associative `implies`, prefix `not`/`-`).
        // Interpreting their declarative `Rule` with plain Rust recursion
        // would grow one native stack frame per chain element; these three
        // are instead built iteratively below, producing the identical node
        // shape the declarative table above still documents for inventory
        // and reserved-word purposes (see the comment there).
        match production {
            Production::Expression => return self.parse_expression(position, depth),
            Production::Implication => return self.parse_implication(position, depth),
            Production::Unary => return self.parse_unary(position, depth),
            _ => {}
        }
        // Copies the `&'a Grammar` reference (not its contents): no `Rule`
        // is cloned on the production path (QSL-197 AC-4).
        let grammar = self.grammar;
        let Some(rule) = grammar.get(&production) else {
            return Attempt::No(Failure::expected(position, format!("{production:?}")));
        };
        let matched = self.rule(rule, position, depth);
        match matched {
            Attempt::Match(matched) => {
                if matched.node_count >= self.limits.nodes {
                    return Attempt::Exhausted(ExhaustedKind::Nodes {
                        bound: self.limits.nodes,
                    });
                }
                Attempt::Match(MatchNode {
                    production,
                    start: position,
                    end: matched.end,
                    children: matched.children,
                    node_count: matched.node_count + 1,
                })
            }
            Attempt::No(failure) => Attempt::No(failure),
            Attempt::Exhausted(kind) => Attempt::Exhausted(kind),
        }
    }

    /// Parse `Expression = ("let" Identifier "=" Expression "in" Expression
    /// | "if" Expression "then" Expression "else" Expression |
    /// Implication)`, unwinding the `let …in`/`if … else` continuation
    /// chain with an explicit loop instead of Rust recursion (NFR-001:
    /// these chains add no nesting depth and must not overflow the stack).
    fn parse_expression(&mut self, position: usize, depth: usize) -> Attempt<MatchNode> {
        if !self.charge(position) {
            return Attempt::Exhausted(ExhaustedKind::Steps);
        }
        enum Pending {
            Let {
                start: usize,
                value: MatchNode,
            },
            If {
                start: usize,
                condition: MatchNode,
                then_branch: MatchNode,
            },
        }
        let mut stack: Vec<Pending> = Vec::new();
        let mut cursor = position;
        loop {
            let start = cursor;
            if self.peek_is(cursor, "let") {
                cursor = attempt!(self.expect_terminal(Terminal::Exact("let"), cursor));
                cursor = attempt!(self.expect_terminal(Terminal::Identifier, cursor));
                cursor = attempt!(self.expect_terminal(Terminal::Exact("="), cursor));
                let value = attempt!(self.production(Production::Expression, cursor, depth));
                cursor = value.end;
                cursor = attempt!(self.expect_terminal(Terminal::Exact("in"), cursor));
                stack.push(Pending::Let { start, value });
                continue;
            }
            if self.peek_is(cursor, "if") {
                cursor = attempt!(self.expect_terminal(Terminal::Exact("if"), cursor));
                let condition = attempt!(self.production(Production::Expression, cursor, depth));
                cursor = condition.end;
                cursor = attempt!(self.expect_terminal(Terminal::Exact("then"), cursor));
                let then_branch = attempt!(self.production(Production::Expression, cursor, depth));
                cursor = then_branch.end;
                cursor = attempt!(self.expect_terminal(Terminal::Exact("else"), cursor));
                stack.push(Pending::If {
                    start,
                    condition,
                    then_branch,
                });
                continue;
            }
            break;
        }
        let implication = attempt!(self.production(Production::Implication, cursor, depth));
        // As in `parse_implication`/`parse_unary`: check the total node
        // budget once, against the still-shallow `implication` subtree and
        // each pending `let`/`if` wrapper's (already fully built, but not
        // yet nested) `value`/`condition`/`then_branch`, before folding
        // them into the nested `Expression` chain below. A mid-loop check
        // would risk dropping a partially-built deep chain on an early
        // `Exhausted` return.
        let mut total_node_count = implication.node_count.saturating_add(1);
        for pending in &stack {
            let extra = match pending {
                Pending::Let { value, .. } => value.node_count,
                Pending::If {
                    condition,
                    then_branch,
                    ..
                } => condition.node_count.saturating_add(then_branch.node_count),
            };
            total_node_count = total_node_count.saturating_add(extra).saturating_add(1);
        }
        if total_node_count >= self.limits.nodes {
            return Attempt::Exhausted(ExhaustedKind::Nodes {
                bound: self.limits.nodes,
            });
        }
        let mut node_count = implication.node_count.saturating_add(1);
        let mut acc = MatchNode {
            production: Production::Expression,
            start: implication.start,
            end: implication.end,
            node_count,
            children: vec![implication],
        };
        for pending in stack.into_iter().rev() {
            let end = acc.end;
            let (start, extra, children) = match pending {
                Pending::Let { start, value } => {
                    let extra = value.node_count;
                    (start, extra, vec![value, acc])
                }
                Pending::If {
                    start,
                    condition,
                    then_branch,
                } => {
                    let extra = condition.node_count.saturating_add(then_branch.node_count);
                    (start, extra, vec![condition, then_branch, acc])
                }
            };
            node_count = node_count.saturating_add(extra).saturating_add(1);
            acc = MatchNode {
                production: Production::Expression,
                start,
                end,
                node_count,
                children,
            };
        }
        Attempt::Match(acc)
    }

    /// Parse `Implication = Disjunction ("implies" Implication)?`
    /// iteratively: a right-associative chain built by folding collected
    /// operands from the right, rather than by Rust recursion per element.
    fn parse_implication(&mut self, position: usize, depth: usize) -> Attempt<MatchNode> {
        if !self.charge(position) {
            return Attempt::Exhausted(ExhaustedKind::Steps);
        }
        let mut operands = Vec::new();
        let mut cursor = position;
        loop {
            let operand = attempt!(self.production(Production::Disjunction, cursor, depth));
            cursor = operand.end;
            operands.push(operand);
            if self.peek_is(cursor, "implies") {
                cursor = attempt!(self.expect_terminal(Terminal::Exact("implies"), cursor));
                continue;
            }
            break;
        }
        // Check the total node budget against the flat (shallow) `operands`
        // list *before* folding it into the right-nested `Implication`
        // chain below. Checking mid-fold instead would risk an early
        // `Exhausted` return while `acc` already holds a deep chain -- and
        // dropping that partially-built chain would recurse just as deep as
        // building it iteratively was meant to avoid (QSL-197: this crashed
        // a 512 KiB stack in exactly this way during development).
        let total_node_count = operands
            .iter()
            .fold(0_usize, |total, operand| {
                total.saturating_add(operand.node_count)
            })
            .saturating_add(operands.len());
        if total_node_count >= self.limits.nodes {
            return Attempt::Exhausted(ExhaustedKind::Nodes {
                bound: self.limits.nodes,
            });
        }
        let mut iter = operands.into_iter().rev();
        let last = iter.next().expect("at least one Disjunction operand");
        let mut node_count = last.node_count.saturating_add(1);
        let mut acc = MatchNode {
            production: Production::Implication,
            start: last.start,
            end: last.end,
            node_count,
            children: vec![last],
        };
        for operand in iter {
            node_count = node_count
                .saturating_add(operand.node_count)
                .saturating_add(1);
            acc = MatchNode {
                production: Production::Implication,
                start: operand.start,
                end: acc.end,
                node_count,
                children: vec![operand, acc],
            };
        }
        Attempt::Match(acc)
    }

    /// Parse `Unary = ("not" | "-") Unary | Postfix` iteratively: a run of
    /// `k` prefix operators wraps one `Postfix` operand in `k + 1` nested
    /// `Unary` nodes (matching the declarative shape, where even the
    /// zero-prefix base case wraps once), built with an explicit loop
    /// instead of Rust recursion per operator.
    fn parse_unary(&mut self, position: usize, depth: usize) -> Attempt<MatchNode> {
        if !self.charge(position) {
            return Attempt::Exhausted(ExhaustedKind::Steps);
        }
        let mut prefix_count = 0_usize;
        let mut cursor = position;
        while self.peek_is(cursor, "not") || self.peek_is(cursor, "-") {
            if !self.charge(cursor) {
                return Attempt::Exhausted(ExhaustedKind::Steps);
            }
            cursor += 1;
            prefix_count += 1;
        }
        let postfix = attempt!(self.production(Production::Postfix, cursor, depth));
        // As in `parse_implication`: check the total node budget once,
        // against the still-shallow `postfix` subtree, before building the
        // `prefix_count + 1`-deep nested `Unary` chain. A mid-loop check
        // would risk dropping a partially-built deep chain on an early
        // `Exhausted` return.
        let total_node_count = postfix
            .node_count
            .saturating_add(prefix_count)
            .saturating_add(1);
        if total_node_count >= self.limits.nodes {
            return Attempt::Exhausted(ExhaustedKind::Nodes {
                bound: self.limits.nodes,
            });
        }
        let mut acc = postfix;
        for offset in (0..=prefix_count).rev() {
            acc = MatchNode {
                production: Production::Unary,
                start: position + offset,
                end: acc.end,
                node_count: acc.node_count.saturating_add(1),
                children: vec![acc],
            };
        }
        Attempt::Match(acc)
    }

    fn rule(&mut self, rule: &Rule, position: usize, depth: usize) -> Attempt<RuleMatch> {
        if !self.charge(position) {
            return Attempt::Exhausted(ExhaustedKind::Steps);
        }
        match rule {
            Rule::Terminal(terminal) => self.terminal(terminal, position, depth),
            Rule::Production(production) => match self.production(*production, position, depth) {
                Attempt::Match(node) => Attempt::Match(RuleMatch {
                    end: node.end,
                    node_count: node.node_count,
                    children: vec![node],
                }),
                Attempt::No(failure) => Attempt::No(failure),
                Attempt::Exhausted(kind) => Attempt::Exhausted(kind),
            },
            Rule::Sequence(rules) => {
                let mut end = position;
                let mut current_depth = depth;
                let mut children = Vec::new();
                let mut node_count = 0_usize;
                for rule in rules {
                    match self.rule(rule, end, current_depth) {
                        Attempt::Match(matched) => {
                            node_count = node_count.saturating_add(matched.node_count);
                            if node_count > self.limits.nodes {
                                return Attempt::Exhausted(ExhaustedKind::Nodes {
                                    bound: self.limits.nodes,
                                });
                            }
                            end = matched.end;
                            children.extend(matched.children);
                            // Depth is otherwise invariant across a
                            // Sequence's own elements: every bracket this
                            // grammar authors opens and closes within the
                            // same Sequence, so only the two terminal kinds
                            // that mark a bracket boundary shift it.
                            current_depth = match rule {
                                Rule::Terminal(Terminal::Open(_)) => current_depth + 1,
                                Rule::Terminal(Terminal::Close(_)) => {
                                    current_depth.saturating_sub(1)
                                }
                                _ => current_depth,
                            };
                        }
                        Attempt::No(failure) => return Attempt::No(failure),
                        Attempt::Exhausted(kind) => return Attempt::Exhausted(kind),
                    }
                }
                Attempt::Match(RuleMatch {
                    end,
                    children,
                    node_count,
                })
            }
            Rule::Choice(rules) => {
                let mut failure = None;
                for rule in rules {
                    match self.rule(rule, position, depth) {
                        Attempt::Match(matched) => return Attempt::Match(matched),
                        Attempt::No(next) => {
                            failure = Some(
                                failure
                                    .map_or(next.clone(), |current: Failure| current.merge(next)),
                            );
                        }
                        Attempt::Exhausted(kind) => return Attempt::Exhausted(kind),
                    }
                }
                Attempt::No(
                    failure.unwrap_or_else(|| Failure::expected(position, "alternative".into())),
                )
            }
            Rule::Optional(rule) => match self.rule(rule, position, depth) {
                Attempt::Match(matched) => Attempt::Match(matched),
                Attempt::No(failure) if failure.position > position => Attempt::No(failure),
                Attempt::No(_) => Attempt::Match(RuleMatch {
                    end: position,
                    children: Vec::new(),
                    node_count: 0,
                }),
                Attempt::Exhausted(kind) => Attempt::Exhausted(kind),
            },
            Rule::Repeat {
                rule,
                minimum,
                commit_on_progress,
            } => {
                let mut end = position;
                let mut children = Vec::new();
                let mut node_count = 0_usize;
                let mut count = 0;
                let mut failure = None;
                loop {
                    match self.rule(rule, end, depth) {
                        Attempt::Match(matched) if matched.end > end => {
                            node_count = node_count.saturating_add(matched.node_count);
                            if node_count > self.limits.nodes {
                                return Attempt::Exhausted(ExhaustedKind::Nodes {
                                    bound: self.limits.nodes,
                                });
                            }
                            end = matched.end;
                            children.extend(matched.children);
                            count += 1;
                        }
                        Attempt::Match(_) => break,
                        Attempt::No(next) if *commit_on_progress && next.position > end => {
                            return Attempt::No(next);
                        }
                        Attempt::No(next) => {
                            failure = Some(next);
                            break;
                        }
                        Attempt::Exhausted(kind) => return Attempt::Exhausted(kind),
                    }
                }
                if count >= *minimum {
                    Attempt::Match(RuleMatch {
                        end,
                        children,
                        node_count,
                    })
                } else {
                    Attempt::No(
                        failure.unwrap_or_else(|| {
                            Failure::expected(end, "repeated production".into())
                        }),
                    )
                }
            }
        }
    }

    fn terminal(&self, terminal: &Terminal, position: usize, depth: usize) -> Attempt<RuleMatch> {
        let accepted = match terminal {
            Terminal::End => {
                return if position == self.tokens.len() {
                    Attempt::Match(RuleMatch {
                        end: position,
                        children: Vec::new(),
                        node_count: 0,
                    })
                } else {
                    Attempt::No(Failure::expected(position, "end of source".into()))
                };
            }
            _ => {
                let Some(token) = self.tokens.get(position) else {
                    return Attempt::No(Failure::expected(position, terminal.description()));
                };
                match terminal {
                    Terminal::Exact(value)
                    | Terminal::Literal(value)
                    | Terminal::Open(value)
                    | Terminal::Close(value) => token.spelling.as_ref() == *value,
                    Terminal::Identifier => {
                        crate::token::identifier_spelling(&token.spelling)
                            && !self.reserved_words.contains(token.spelling.as_ref())
                    }
                    Terminal::MemberName => crate::token::identifier_spelling(&token.spelling),
                    Terminal::Text => matches!(token.kind, Some(Kind::Text(_))),
                    Terminal::TextValue(expected) => {
                        matches!(&token.kind, Some(Kind::Text(value)) if value == expected)
                    }
                    Terminal::UnsignedInteger => matches!(token.kind, Some(Kind::Integer(_))),
                    Terminal::End => false,
                }
            }
        };
        if !accepted {
            return Attempt::No(Failure::expected(position, terminal.description()));
        }
        // NFR-001 "Nesting level": the opening bracket of pair `bound + 1`
        // refuses here, naming nesting depth, the ceiling and this bracket's
        // own span (never the farthest token scanned).
        if let Terminal::Open(_) = terminal {
            if depth >= self.limits.nesting {
                let span = self
                    .tokens
                    .get(position)
                    .map_or(Span { start: 0, end: 0 }, |token| token.span);
                return Attempt::Exhausted(ExhaustedKind::Nesting {
                    bound: self.limits.nesting,
                    span,
                });
            }
        }
        Attempt::Match(RuleMatch {
            end: position + 1,
            children: Vec::new(),
            node_count: 0,
        })
    }
}

impl Terminal {
    fn description(&self) -> String {
        match self {
            Self::Exact(value) | Self::Literal(value) | Self::Open(value) | Self::Close(value) => {
                format!("`{value}`")
            }
            Self::Identifier => "identifier".into(),
            Self::MemberName => "qualified ASCII member name".into(),
            Self::Text => "quoted string".into(),
            Self::TextValue(value) => format!("`\"{value}\"`"),
            Self::UnsignedInteger => "unsigned integer".into(),
            Self::End => "end of source".into(),
        }
    }
}

/// One partially-lowered `MatchNode` on the iterative work stack below: its
/// own production/span inputs, the remaining children still to descend
/// into, and the `CstElement`s already produced for children visited so far.
struct LowerFrame {
    production: Production,
    start: usize,
    end: usize,
    remaining: std::vec::IntoIter<MatchNode>,
    collected: Vec<CstElement>,
}

fn node_span(
    source: &Source,
    tokens: &[Significant],
    production: Production,
    start: usize,
    end: usize,
) -> Span {
    if production == Production::CompleteUnit {
        return Span {
            start: 0,
            end: source.text().len(),
        };
    }
    let span_start = tokens
        .get(start)
        .map_or(source.text().len(), |token| token.span.start);
    let span_end = end
        .checked_sub(1)
        .and_then(|index| tokens.get(index))
        .map_or(span_start, |token| token.span.end);
    Span {
        start: span_start,
        end: span_end,
    }
}

/// Convert the parsed `MatchNode` tree into the flat `RawNode` arena with an
/// explicit work stack rather than Rust recursion. `let … in`/`if … else`
/// chains and other deep spines are real nested `MatchNode` values (NFR-001
/// only exempts them from the nesting *ceiling*, not from existing as
/// nested productions); a naive recursive walk here — or the recursive
/// `Drop` glue for `Vec<MatchNode>` a large owned tree would trigger — would
/// grow one native stack frame per chain element regardless of how the tree
/// was built. This walk, and the `into_iter()` below that hands out one
/// child `MatchNode` at a time, never holds the whole tree as a value poised
/// to be dropped recursively.
fn lower_tree(source: &Source, tokens: &[Significant], root: MatchNode) -> (Vec<RawNode>, usize) {
    let mut output: Vec<RawNode> = Vec::new();
    let mut stack: Vec<LowerFrame> = Vec::new();
    let mut next = Some(root);
    let mut finished: Option<usize> = None;
    loop {
        if let Some(node) = next.take() {
            let mut remaining = node.children.into_iter();
            let first_child = remaining.next();
            stack.push(LowerFrame {
                production: node.production,
                start: node.start,
                end: node.end,
                remaining,
                collected: Vec::new(),
            });
            if first_child.is_some() {
                next = first_child;
                continue;
            }
        }
        let mut frame = stack.pop().expect("lower_tree: non-empty frame stack");
        if let Some(id) = finished.take() {
            frame.collected.push(CstElement::Node(id));
        }
        if let Some(sibling) = frame.remaining.next() {
            next = Some(sibling);
            stack.push(frame);
            continue;
        }
        let span = node_span(source, tokens, frame.production, frame.start, frame.end);
        let id = output.len();
        output.push(RawNode {
            production: frame.production,
            span,
            children: frame.collected,
        });
        if stack.is_empty() {
            return (output, id);
        }
        finished = Some(id);
    }
}
