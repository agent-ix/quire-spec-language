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
use qsl_foundation::{Phase, Source, Span, SyntaxLimit};

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
        let matched = match engine.production(Production::CompleteUnit, 0) {
            // A top-level match always leaves its root pending. Were that
            // invariant broken, the unit is reported as a syntax failure
            // with a recovery, never lowered to a silent one-node CST.
            Outcome::Match(_) => engine.into_forest().ok_or_else(|| {
                debug_assert!(false, "a matched unit leaves its root pending");
                Failure::expected(0, format!("{:?}", Production::CompleteUnit))
            }),
            Outcome::No(failure) => Err(failure),
            Outcome::Exhausted(refusal) => {
                let span = significant.get(refusal.position).map_or(
                    Span {
                        start: source.text().len(),
                        end: source.text().len(),
                    },
                    |token| token.span,
                );
                return Err(super::diagnostic::resource_exhausted(
                    &source,
                    Phase::Parse,
                    span,
                    refusal.limit,
                ));
            }
        };
        match matched {
            Ok(forest) => Some(forest),
            Err(failure) => {
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
        }
    } else {
        None
    };

    let (nodes, root) = if let Some(forest) = matched {
        lower_tree(&source, &significant, &forest)
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
        limits,
    ))
}

fn selection_prelude_nodes(
    source: &Source,
    grammar: &Grammar,
    tokens: &[Significant],
    limits: Limits,
) -> Vec<RawNode> {
    fn raw(source: &Source, tokens: &[Significant], node: &Matched) -> RawNode {
        RawNode {
            production: node.production,
            span: token_span(source, tokens, node.start, node.end),
            children: Vec::new(),
        }
    }

    let mut engine = Engine::new(grammar, tokens, limits);
    let Outcome::Match(mut position) = engine.production(Production::Header, 0) else {
        return Vec::new();
    };
    let mut output = Vec::new();
    for (production, required) in [
        (Production::Profile, true),
        (Production::ImportDeclaration, false),
        (Production::Model, false),
    ] {
        let mut count = 0_usize;
        while let Outcome::Match(end) = engine.production(production, position) {
            if end <= position {
                break;
            }
            position = end;
            let Some(node) = engine.last_match() else {
                break;
            };
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
            return Err(super::diagnostic::resource_exhausted(
                self.source,
                Phase::Lex,
                span,
                SyntaxLimit::Tokens { bound: self.limit },
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
        return Err(super::diagnostic::resource_exhausted(
            source,
            Phase::Lex,
            excess.span(),
            SyntaxLimit::Tokens {
                bound: limits.tokens,
            },
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

/// One matched production in the engine's append-only match store.
///
/// A stored match is immutable once made, and names its direct children by
/// store index. A memoized match is therefore shared, not copied, by every
/// later attempt that reuses it.
#[derive(Clone, Debug)]
struct Matched {
    production: Production,
    /// First significant-token index covered.
    start: usize,
    /// One past the last significant-token index covered.
    end: usize,
    /// This match's direct children, left to right, as a range of the
    /// store's child links.
    children: std::ops::Range<usize>,
    /// Matches in this subtree, itself included: the CST nodes it lowers to.
    size: usize,
    /// Bracket depth the production was entered at; with `production` and
    /// `start`, the memo key.
    depth: usize,
    /// The match stored before this one at the same `start`: the memo is
    /// one list per position, threaded through the store.
    earlier: Option<usize>,
}

/// A completed match not yet adopted by the production that encloses it.
#[derive(Clone, Copy, Debug)]
struct Pending {
    /// Store index of the match.
    node: usize,
    /// Subtree sizes of this and every earlier pending match, summed, so the
    /// matches made since any pending length are counted in O(1).
    total: usize,
}

/// The engine's result for a matched unit: the match store and its root.
struct Forest {
    matches: Vec<Matched>,
    links: Vec<usize>,
    root: usize,
}

/// A resource ceiling hit while matching, and the significant-token index
/// its refusal is located at.
#[derive(Clone, Copy, Debug)]
struct Refusal {
    limit: SyntaxLimit,
    position: usize,
}

/// Result of matching one rule at one position.
#[derive(Debug)]
enum Outcome {
    /// Matched; the value is the position after the match.
    Match(usize),
    /// Did not match; the failure is recoverable by an enclosing choice,
    /// option or repetition.
    No(Failure),
    /// A resource ceiling was hit; fatal for the whole parse.
    Exhausted(Refusal),
}

/// A rule suspended on the engine's explicit stack, waiting for the outcome
/// of the sub-rule it started. Each variant holds the state the recursive
/// formulation would have kept in its Rust stack frame.
enum Frame<'g> {
    /// A production whose rule is being matched.
    Production {
        production: Production,
        position: usize,
        depth: usize,
        first: usize,
    },
    /// `rules[index]` is being matched.
    Sequence {
        rules: &'g [Rule],
        index: usize,
        first: usize,
        depth: usize,
    },
    /// Alternative `rules[index]` is being matched at `position`.
    Choice {
        rules: &'g [Rule],
        index: usize,
        position: usize,
        first: usize,
        depth: usize,
        failure: Option<Failure>,
    },
    /// The optional rule is being matched at `position`.
    Optional { position: usize, first: usize },
    /// One more repetition of `rule` is being matched at `end`.
    Repeat {
        rule: &'g Rule,
        minimum: usize,
        commit_on_progress: bool,
        end: usize,
        count: usize,
        first: usize,
        iteration: usize,
        depth: usize,
    },
}

/// What the engine does next.
enum Step<'g> {
    /// Start matching `rule` at a position and bracket depth.
    Rule(&'g Rule, usize, usize),
    /// Start matching a production at a position and bracket depth.
    Production(Production, usize, usize),
    /// Hand an outcome to the innermost suspended frame.
    Return(Outcome),
}

/// Interpreter steps the work budget allows per significant token.
///
/// Derivation. The engine charges one step per rule entry and one per
/// production entry. Without backtracking that re-reads input, the steps a
/// unit takes grow linearly with its significant tokens, so the budget is a
/// per-token constant. Measured steps per significant token at default
/// ceilings (`tests::measured_work_leaves_headroom_under_the_budget`, which
/// re-measures these inputs and fails if any exceeds a third of the budget):
///
/// | input (function body unless noted)   | steps / token |
/// | ------------------------------------ | ------------- |
/// | `x = x and x = x ...`                | 69.9          |
/// | 63 nested parentheses                | 69.4          |
/// | `x + x + ...`                        | 65.2          |
/// | `g(x, x) + ...`                      | 61.2          |
/// | `x * x * ...`                        | 59.8          |
/// | `x[x][x]...`                         | 59.4          |
/// | `set[x, x] + ...`                    | 58.7          |
/// | `forall(v in x: v) and ...`          | 52.8          |
/// | `if true then x else ...`            | 51.2          |
/// | `M::T(x, x) + ...`                   | 48.0          |
/// | `let v = x in ...`                   | 36.1          |
/// | `true implies true ...`              | 33.8          |
/// | `M::R { a: x, b: x } + ...`          | 33.5          |
/// | `size<M::T>(x) + ...`                | 33.0          |
/// | 63 nested `Option<...>` (alias)      | 7.1           |
/// | `not not ... true`                   | 6.0           |
/// | `always [0,1] ...` (temporal clause) | 3.0           |
///
/// Measured with production memoization (QSL-213). The same inputs store
/// 0.03 to 0.17 matches per step.
///
/// An identifier operand is the costliest token: `Primary` tries every
/// alternative before `QualifiedName`. 256 is over three times the worst
/// measured case.
///
/// Alternatives that share a prefix re-read it: a failed `TupleValue` is
/// parsed again as a call argument list, and in `f(g(f(g(x,)),))` each
/// level repeats the whole level inside it. The engine memoizes every
/// production that matched, so such a re-read costs one step per production
/// it reuses and work stays linear in bracket depth
/// (`tests::nested_trailing_comma_work_is_linear_in_depth`).
const WORK_PER_TOKEN: usize = 256;

/// A bounded interpreter for the declarative grammar table.
///
/// It runs on explicit heap stacks: `frames` holds suspended rules,
/// `matches` holds every completed production match, and `pending` holds the
/// completed matches not yet adopted by an enclosing production. No Rust call
/// recursion depends on the input, so neither a long
/// operator/prefix/`let`/`if`/temporal chain nor a deeply bracketed unit can
/// overflow the thread's stack (NFR-001). Every structure is flat, so
/// discarding a failed attempt truncates `pending` and dropping a result
/// never recurses.
///
/// A production's outcome is a function of the production, position and
/// bracket depth alone, so a match is memoized under that key and reused by
/// reference.
///
/// Memory. `matches` and `links` are append-only for the engine's lifetime,
/// so they are bounded by the work budget, not by the node ceiling: a failed
/// attempt's matches stay stored. A new match costs at least two charged
/// steps (its production entry and its rule entry) and takes 72 bytes; a
/// child link costs at least one step and takes 8 bytes; `pending` holds at
/// most the links of the productions still open. With 2 × matches + reuses
/// ≤ steps, the store takes at most 40 bytes per step of the budget, plus 16
/// bytes per token for the memo heads. At the default ceilings
/// (100,000 tokens, so 25,600,256 steps) that is about 1 GiB in the worst
/// case. The measured inputs at [`WORK_PER_TOKEN`] store at most 0.17
/// matches and 0.17 links per step, about 14 bytes per step.
struct Engine<'a> {
    grammar: &'a Grammar,
    reserved_words: BTreeSet<&'static str>,
    tokens: &'a [Significant],
    limits: Limits,
    frames: Vec<Frame<'a>>,
    matches: Vec<Matched>,
    links: Vec<usize>,
    pending: Vec<Pending>,
    /// The latest stored match starting at each position `0..=tokens`, the
    /// head of that position's memo list.
    memo: Vec<Option<usize>>,
    steps: usize,
    maximum_steps: usize,
    farthest: usize,
    /// The farthest-position failure any attempt hit during the current
    /// [`Self::production`] call, kept even where the frame that hit it
    /// (`Optional`, a backtracking `Repeat`, a `Choice` that picks a
    /// different alternative) goes on to discard its own copy because that
    /// arm no longer needs it. Consulted only if the call's overall outcome
    /// is [`Outcome::No`]: a successful parse never reports it, so this
    /// never changes what a valid input produces.
    farthest_failure: Option<Failure>,
}

impl<'a> Engine<'a> {
    /// The work budget is [`WORK_PER_TOKEN`] steps per significant token,
    /// plus one token's worth for the end of input, so parsing work is
    /// linear in the input.
    fn new(grammar: &'a Grammar, tokens: &'a [Significant], limits: Limits) -> Self {
        Self {
            grammar,
            reserved_words: grammar::complete_reserved_words(grammar),
            tokens,
            limits,
            frames: Vec::new(),
            matches: Vec::new(),
            links: Vec::new(),
            pending: Vec::new(),
            memo: vec![None; tokens.len().saturating_add(1)],
            steps: 0,
            maximum_steps: tokens
                .len()
                .saturating_add(1)
                .saturating_mul(WORK_PER_TOKEN),
            farthest: 0,
            farthest_failure: None,
        }
    }

    fn charge(&mut self, position: usize) -> bool {
        self.farthest = self.farthest.max(position.min(self.tokens.len()));
        self.steps = self.steps.saturating_add(1);
        self.steps <= self.maximum_steps
    }

    /// Record `failure` as a farthest-failure candidate, then return it as
    /// an [`Outcome::No`]. Every fresh failure (a mismatched terminal, an
    /// unmatched production or alternative) is created through this method,
    /// so `farthest_failure` always holds the deepest position any attempt
    /// reached this call, independent of which frame later discards its own
    /// copy of the same failure.
    fn fail(&mut self, failure: Failure) -> Outcome {
        self.farthest_failure = Some(match self.farthest_failure.take() {
            Some(farthest) => farthest.merge(failure.clone()),
            None => failure.clone(),
        });
        Outcome::No(failure)
    }

    fn work_exhausted(&self) -> Outcome {
        Outcome::Exhausted(Refusal {
            limit: SyntaxLimit::Work {
                bound: self.maximum_steps,
            },
            position: self.farthest,
        })
    }

    fn nodes_exhausted(&self) -> Outcome {
        Outcome::Exhausted(Refusal {
            limit: SyntaxLimit::Nodes {
                bound: self.limits.nodes,
            },
            position: self.farthest,
        })
    }

    /// Matches, descendants included, made since pending length `first`.
    fn matched_since(&self, first: usize) -> usize {
        let total = |length: usize| {
            length
                .checked_sub(1)
                .and_then(|index| self.pending.get(index))
                .map_or(0, |pending| pending.total)
        };
        total(self.pending.len()).saturating_sub(total(first))
    }

    /// Queue stored match `node` for adoption by the enclosing production.
    fn adopt_later(&mut self, node: usize) {
        let before = self.pending.last().map_or(0, |pending| pending.total);
        let size = self.matches.get(node).map_or(0, |matched| matched.size);
        debug_assert!(size > 0, "pending match {node} is stored");
        self.pending.push(Pending {
            node,
            total: before.saturating_add(size),
        });
    }

    /// Match `production` at `position`. On [`Outcome::Match`], the
    /// production's own match is the last pending match.
    ///
    /// The store and memo persist across calls on one engine: the tokens are
    /// the same, so a memoized match stays valid, and every call charges the
    /// one work budget. Clearing them per call would cost O(tokens) each time
    /// `selection_prelude_nodes` reads one more declaration.
    ///
    /// `farthest_failure` is scoped to this one call: it is reset on entry
    /// and, when the call's own outcome is [`Outcome::No`], consulted before
    /// returning. It replaces the bubbled failure only when it sits strictly
    /// farther, so the reported failure sits at the deepest position any
    /// attempt reached rather than wherever the outermost frame happened to
    /// be looking when the whole match gave up. A tie keeps the bubbled
    /// failure exactly as it was (including its own `expected` set):
    /// `farthest_failure` is a whole-call accumulator, so two rules that
    /// happen to fail at the same position for unrelated reasons must not
    /// blend their `expected` sets the way two alternatives of the same
    /// `Choice` deliberately do.
    fn production(&mut self, production: Production, position: usize) -> Outcome {
        self.frames.clear();
        self.pending.clear();
        self.farthest_failure = None;
        let mut step = Step::Production(production, position, 0);
        loop {
            step = match step {
                Step::Production(production, position, depth) => {
                    self.enter_production(production, position, depth)
                }
                Step::Rule(rule, position, depth) => self.enter_rule(rule, position, depth),
                Step::Return(outcome) => match self.frames.pop() {
                    None => {
                        return match outcome {
                            Outcome::No(failure) => {
                                Outcome::No(match self.farthest_failure.take() {
                                    Some(farthest) if farthest.position > failure.position => {
                                        farthest
                                    }
                                    _ => failure,
                                })
                            }
                            other => other,
                        };
                    }
                    Some(frame) => self.resume(frame, outcome),
                },
            };
        }
    }

    /// The match the last successful [`Self::production`] call produced.
    fn last_match(&self) -> Option<&Matched> {
        let pending = self.pending.last()?;
        self.matches.get(pending.node)
    }

    /// The store and root of the last successful [`Self::production`] call.
    fn into_forest(self) -> Option<Forest> {
        let root = self.pending.last()?.node;
        Some(Forest {
            matches: self.matches,
            links: self.links,
            root,
        })
    }

    fn enter_production(
        &mut self,
        production: Production,
        position: usize,
        depth: usize,
    ) -> Step<'a> {
        if !self.charge(position) {
            return Step::Return(self.work_exhausted());
        }
        // Reuse, not copy: the memoized match joins this attempt by
        // reference, one step for the whole subtree. The walk is not charged
        // per entry. It is bounded: a (production, depth) pair is stored at
        // most once per position, because once stored it is never matched
        // again there, so the list holds at most one entry per production
        // per bracket depth (`Production` count × (nesting ceiling + 1)),
        // and in practice the few productions that can start at one token.
        let mut cursor = self.memo.get(position).copied().flatten();
        while let Some(node) = cursor {
            let Some(matched) = self.matches.get(node) else {
                break;
            };
            if matched.production == production && matched.depth == depth {
                let end = matched.end;
                self.adopt_later(node);
                return Step::Return(Outcome::Match(end));
            }
            cursor = matched.earlier;
        }
        let grammar = self.grammar;
        let Some(rule) = grammar.get(&production) else {
            return Step::Return(self.fail(Failure::expected(position, format!("{production:?}"))));
        };
        self.frames.push(Frame::Production {
            production,
            position,
            depth,
            first: self.pending.len(),
        });
        Step::Rule(rule, position, depth)
    }

    fn enter_rule(&mut self, rule: &'a Rule, position: usize, depth: usize) -> Step<'a> {
        if !self.charge(position) {
            return Step::Return(self.work_exhausted());
        }
        let first = self.pending.len();
        match rule {
            Rule::Terminal(terminal) => Step::Return(self.terminal(terminal, position, depth)),
            Rule::Production(production) => Step::Production(*production, position, depth),
            Rule::Sequence(rules) => {
                let Some(head) = rules.first() else {
                    return Step::Return(Outcome::Match(position));
                };
                self.frames.push(Frame::Sequence {
                    rules,
                    index: 0,
                    first,
                    depth,
                });
                Step::Rule(head, position, depth)
            }
            Rule::Choice(rules) => {
                let Some(head) = rules.first() else {
                    return Step::Return(
                        self.fail(Failure::expected(position, "alternative".into())),
                    );
                };
                self.frames.push(Frame::Choice {
                    rules,
                    index: 0,
                    position,
                    first,
                    depth,
                    failure: None,
                });
                Step::Rule(head, position, depth)
            }
            Rule::Optional(inner) => {
                self.frames.push(Frame::Optional { position, first });
                Step::Rule(inner, position, depth)
            }
            Rule::Repeat {
                rule: inner,
                minimum,
                commit_on_progress,
            } => {
                self.frames.push(Frame::Repeat {
                    rule: inner,
                    minimum: *minimum,
                    commit_on_progress: *commit_on_progress,
                    end: position,
                    count: 0,
                    first,
                    iteration: first,
                    depth,
                });
                Step::Rule(inner, position, depth)
            }
        }
    }

    /// Continue the suspended `frame` with the outcome of the sub-rule it
    /// started. Every recovery point (a later alternative, an absent option,
    /// the end of a repetition) truncates `pending` to where the failed
    /// attempt began, so the attempt's matches leave the tree. They stay in
    /// the store, where a memoized one can still be reused.
    fn resume(&mut self, frame: Frame<'a>, outcome: Outcome) -> Step<'a> {
        match frame {
            Frame::Production {
                production,
                position,
                depth,
                first,
            } => match outcome {
                Outcome::Match(end) => {
                    let descendants = self.matched_since(first);
                    if descendants >= self.limits.nodes {
                        return Step::Return(self.nodes_exhausted());
                    }
                    let links = self.links.len();
                    debug_assert!(first <= self.pending.len(), "a frame's pending start");
                    self.links.extend(
                        self.pending
                            .get(first..)
                            .unwrap_or_default()
                            .iter()
                            .map(|pending| pending.node),
                    );
                    self.pending.truncate(first);
                    let node = self.matches.len();
                    let earlier = self
                        .memo
                        .get_mut(position)
                        .and_then(|head| head.replace(node));
                    self.matches.push(Matched {
                        production,
                        start: position,
                        end,
                        children: links..self.links.len(),
                        size: descendants + 1,
                        depth,
                        earlier,
                    });
                    self.adopt_later(node);
                    Step::Return(Outcome::Match(end))
                }
                other => Step::Return(other),
            },
            Frame::Sequence {
                rules,
                index,
                first,
                depth,
            } => match outcome {
                Outcome::Match(end) => {
                    if self.matched_since(first) > self.limits.nodes {
                        return Step::Return(self.nodes_exhausted());
                    }
                    // A bracket pair opens and closes within one sequence
                    // (`tc_197_brackets_are_direct_sequence_elements`), so
                    // the sequence alone tracks the depth of what follows.
                    let depth = match &rules[index] {
                        Rule::Terminal(Terminal::Open(_)) => depth.saturating_add(1),
                        Rule::Terminal(Terminal::Close(_)) => depth.saturating_sub(1),
                        _ => depth,
                    };
                    let index = index + 1;
                    let Some(next) = rules.get(index) else {
                        return Step::Return(Outcome::Match(end));
                    };
                    self.frames.push(Frame::Sequence {
                        rules,
                        index,
                        first,
                        depth,
                    });
                    Step::Rule(next, end, depth)
                }
                other => Step::Return(other),
            },
            Frame::Choice {
                rules,
                index,
                position,
                first,
                depth,
                failure,
            } => match outcome {
                Outcome::No(next) => {
                    let failure = match failure {
                        None => next,
                        Some(current) => current.merge(next),
                    };
                    self.pending.truncate(first);
                    let index = index + 1;
                    let Some(alternative) = rules.get(index) else {
                        return Step::Return(Outcome::No(failure));
                    };
                    self.frames.push(Frame::Choice {
                        rules,
                        index,
                        position,
                        first,
                        depth,
                        failure: Some(failure),
                    });
                    Step::Rule(alternative, position, depth)
                }
                other => Step::Return(other),
            },
            Frame::Optional { position, first } => match outcome {
                Outcome::No(failure) if failure.position > position => {
                    Step::Return(Outcome::No(failure))
                }
                Outcome::No(_) => {
                    self.pending.truncate(first);
                    Step::Return(Outcome::Match(position))
                }
                other => Step::Return(other),
            },
            Frame::Repeat {
                rule,
                minimum,
                commit_on_progress,
                end,
                count,
                first,
                iteration,
                depth,
            } => {
                let failure = match outcome {
                    Outcome::Match(next) if next > end => {
                        if self.matched_since(first) > self.limits.nodes {
                            return Step::Return(self.nodes_exhausted());
                        }
                        self.frames.push(Frame::Repeat {
                            rule,
                            minimum,
                            commit_on_progress,
                            end: next,
                            count: count + 1,
                            first,
                            iteration: self.pending.len(),
                            depth,
                        });
                        return Step::Rule(rule, next, depth);
                    }
                    // A repetition that consumed nothing ends the loop and
                    // keeps none of its matches.
                    Outcome::Match(_) => {
                        self.pending.truncate(iteration);
                        None
                    }
                    Outcome::No(next) if commit_on_progress && next.position > end => {
                        return Step::Return(Outcome::No(next));
                    }
                    Outcome::No(next) => {
                        self.pending.truncate(iteration);
                        Some(next)
                    }
                    exhausted @ Outcome::Exhausted(_) => return Step::Return(exhausted),
                };
                if count >= minimum {
                    Step::Return(Outcome::Match(end))
                } else {
                    Step::Return(match failure {
                        Some(failure) => Outcome::No(failure),
                        None => self.fail(Failure::expected(end, "repeated production".into())),
                    })
                }
            }
        }
    }

    fn terminal(&mut self, terminal: &Terminal, position: usize, depth: usize) -> Outcome {
        let accepted = match terminal {
            Terminal::End => {
                return if position == self.tokens.len() {
                    Outcome::Match(position)
                } else {
                    self.fail(Failure::expected(position, "end of source".into()))
                };
            }
            _ => {
                let Some(token) = self.tokens.get(position) else {
                    return self.fail(Failure::expected(position, terminal.description()));
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
            return self.fail(Failure::expected(position, terminal.description()));
        }
        // NFR-001 "Nesting level": the opening bracket of pair `nesting + 1`
        // is refused at its own span.
        if matches!(terminal, Terminal::Open(_)) && depth >= self.limits.nesting {
            return Outcome::Exhausted(Refusal {
                limit: SyntaxLimit::NestingDepth {
                    bound: self.limits.nesting,
                },
                position,
            });
        }
        Outcome::Match(position + 1)
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

/// Source span of significant tokens `start..end`; empty at `start`'s
/// offset when the range is empty.
fn token_span(source: &Source, tokens: &[Significant], start: usize, end: usize) -> Span {
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

/// Lower the matched tree under the forest's root into CST raw nodes in
/// post-order (every descendant before its ancestor, siblings left to
/// right). Each reference to a stored match lowers to its own CST node, so a
/// memoized match reused in the tree is never a shared CST node. The walk
/// keeps its path on a heap stack, so it is iterative and linear in the
/// nodes it emits.
fn lower_tree(source: &Source, tokens: &[Significant], forest: &Forest) -> (Vec<RawNode>, usize) {
    /// A stored match being lowered: its next child link and the CST
    /// children lowered so far.
    struct Visit<'f> {
        matched: &'f Matched,
        next: usize,
        children: Vec<CstElement>,
    }

    let visit = |node: usize| {
        forest.matches.get(node).map(|matched| Visit {
            matched,
            next: matched.children.start,
            children: Vec::new(),
        })
    };
    let capacity = forest.matches.get(forest.root).map_or(0, |root| root.size);
    let mut nodes = Vec::with_capacity(capacity);
    let mut path: Vec<Visit<'_>> = visit(forest.root).into_iter().collect();
    while let Some(top) = path.last_mut() {
        if top.next < top.matched.children.end {
            let child = forest.links.get(top.next).and_then(|&child| visit(child));
            debug_assert!(child.is_some(), "child link {} is stored", top.next);
            top.next += 1;
            path.extend(child);
            continue;
        }
        let Some(done) = path.pop() else {
            break;
        };
        let matched = done.matched;
        let span = if matched.production == Production::CompleteUnit {
            Span {
                start: 0,
                end: source.text().len(),
            }
        } else {
            token_span(source, tokens, matched.start, matched.end)
        };
        let index = nodes.len();
        nodes.push(RawNode {
            production: matched.production,
            span,
            children: done.children,
        });
        if let Some(parent) = path.last_mut() {
            parent.children.push(CstElement::Node(index));
        }
    }
    let root = nodes.len().saturating_sub(1);
    (nodes, root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qsl_foundation::SourceIdentity;

    const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

    fn source(text: &str) -> Source {
        Source::read(
            SourceIdentity {
                authority: "test".into(),
                identity: "test:engine".into(),
                revision_namespace: "test".into(),
                revision: "1".into(),
            },
            "engine.native",
            text.as_bytes(),
            qsl_foundation::source::MAX_SOURCE_BYTES,
        )
        .expect("test source")
    }

    fn function(body: &str) -> String {
        format!("{HEADER}function f using Complete (x: Integer): Boolean pure {{ {body} }}\n")
    }

    /// Interpreter steps and significant tokens for a unit that must parse.
    fn work(text: &str) -> (usize, usize) {
        let source = source(text);
        let grammar = grammar::complete_v1();
        let compounds = grammar::complete_compound_spellings(&grammar);
        let (_, significant, diagnostics, _) =
            scan(&source, Limits::default(), &compounds).expect("scan");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let mut engine = Engine::new(&grammar, &significant, Limits::default());
        let outcome = engine.production(Production::CompleteUnit, 0);
        assert!(matches!(outcome, Outcome::Match(_)), "{outcome:?}");
        (engine.steps, significant.len())
    }

    // Re-measures the table recorded at `WORK_PER_TOKEN`.
    #[test]
    fn measured_work_leaves_headroom_under_the_budget() {
        let temporal = |formula: String| {
            format!(
                "{HEADER}temporal W using Complete over (s: Integer) clock \"t\" on origin {{ {formula} }}\n"
            )
        };
        let inputs = [
            function(&["x = x"; 500].join(" and ")),
            function(&["x"; 1000].join(" + ")),
            function(&format!("{}x{}", "(".repeat(63), ")".repeat(63))),
            function(&["g(x, x)"; 250].join(" + ")),
            function(&["x"; 1000].join(" * ")),
            function(&format!("x{}", "[x]".repeat(750))),
            function(&["set[x, x]"; 200].join(" + ")),
            function(&["forall(v in x: v)"; 200].join(" and ")),
            function(&["M::T(x, x)"; 200].join(" + ")),
            function(&["M::R { a: x, b: x }"; 125].join(" + ")),
            function(&["size<M::T>(x)"; 200].join(" + ")),
            function(&format!("{}x", "if true then x else ".repeat(400))),
            function(&format!("{}x", "let v = x in ".repeat(400))),
            function(&["true"; 500].join(" implies ")),
            function(&format!("{}true", "not ".repeat(2000))),
            format!(
                "{HEADER}type T = {}Integer{};\n",
                "Option<".repeat(63),
                ">".repeat(63)
            ),
            temporal("always [0,1] ".repeat(750) + "true"),
        ];
        for text in inputs {
            let (steps, tokens) = work(&text);
            let budget_third = (tokens + 1) * WORK_PER_TOKEN / 3;
            assert!(
                steps <= budget_third,
                "{steps} steps for {tokens} tokens exceeds a third of the budget: {}",
                &text[HEADER.len()..HEADER.len() + 80]
            );
        }
    }

    /// `f(g(f(...x,)...,),)` with `depth` calls, each with a trailing comma
    /// the argument list does not admit.
    fn nested_trailing_comma(depth: usize) -> String {
        let open: String = (0..depth)
            .map(|level| if level % 2 == 0 { "f(" } else { "g(" })
            .collect();
        function(&format!("{open}x{}", ",)".repeat(depth)))
    }

    // Each level fails as a tuple and is re-read as a call argument list; the
    // memoized argument expression makes the re-read one step, not a
    // re-parse of every level inside it. Without memoization the work
    // doubles per level and depth 4 already exhausts the budget.
    #[test]
    fn nested_trailing_comma_work_is_linear_in_depth() {
        let grammar = grammar::complete_v1();
        let compounds = grammar::complete_compound_spellings(&grammar);
        let steps = |depth: usize| {
            let source = source(&nested_trailing_comma(depth));
            let (_, significant, _, _) =
                scan(&source, Limits::default(), &compounds).expect("scan");
            let mut engine = Engine::new(&grammar, &significant, Limits::default());
            let outcome = engine.production(Production::CompleteUnit, 0);
            assert!(
                matches!(outcome, Outcome::No(_)),
                "depth {depth}: {outcome:?}"
            );
            engine.steps
        };
        let counts: Vec<usize> = (2..=12).map(steps).collect();
        let increments: Vec<usize> = counts.windows(2).map(|pair| pair[1] - pair[0]).collect();
        // Linear: every added level costs the same number of steps.
        assert!(
            increments.windows(2).all(|pair| pair[0] == pair[1]),
            "steps by depth 2..=12: {counts:?}"
        );
    }

    /// Run `run` on a 512 KiB thread; overflowing it aborts the test process.
    fn on_bounded_stack<F: FnOnce() + Send + 'static>(run: F) {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(run)
            .expect("spawn a 512 KiB thread")
            .join()
            .expect("the parser must not overflow a 512 KiB stack");
    }

    // A caller may raise the nesting ceiling far past the default; brackets
    // nested that deep still never recurse, and are refused by the token or
    // node ceiling (or parse). `parse` here is the engine entry below the
    // public entry points' own ceiling.
    #[test]
    fn brackets_under_a_raised_nesting_ceiling_never_overflow_the_stack() {
        on_bounded_stack(|| {
            let limits = Limits {
                nesting: 100_000,
                ..Limits::default()
            };
            // Deeper than the node ceiling can hold for parentheses.
            let depth = 6_000;
            let temporal = format!(
                "{HEADER}temporal W using Complete over (s: Integer) clock \"t\" on origin {{ {}true{} }}\n",
                "(".repeat(depth),
                ")".repeat(depth)
            );
            for text in [
                function(&format!("{}x{}", "(".repeat(depth), ")".repeat(depth))),
                function(&format!("{}x{}", "x[".repeat(depth), "]".repeat(depth))),
                format!(
                    "{HEADER}type T = {}Integer{};\n",
                    "Option<".repeat(depth),
                    ">".repeat(depth)
                ),
                temporal,
            ] {
                match parse(source(&text), limits) {
                    Ok(parsed) => assert!(parsed.is_admissible()),
                    Err(refusal) => {
                        let defaults = Limits::default();
                        assert!(
                            refusal.limit()
                                == Some(SyntaxLimit::Nodes {
                                    bound: defaults.nodes
                                })
                                || refusal.limit()
                                    == Some(SyntaxLimit::Tokens {
                                        bound: defaults.tokens
                                    }),
                            "{refusal:?}"
                        );
                    }
                }
            }
            // Within the node ceiling, deep brackets parse.
            let parsed = parse(
                source(&format!(
                    "{HEADER}type T = {}Integer{};\n",
                    "Option<".repeat(2_000),
                    ">".repeat(2_000)
                )),
                limits,
            )
            .expect("2000 nested Option<...> fit the node ceiling");
            assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        });
    }
}
