// SPDX-License-Identifier: AGPL-3.0-or-later
//! Composed productions on the existing token cursor and value Pratt parser.
use super::{c, Parser, K};
use crate::syntax::{ClauseKind, ExprId, ModelImport, LANGUAGE};
use c::*;
use qsl_foundation::{Code, Diagnostic, Phase, Span, Spanned};

mod protocol;
mod temporal;

impl Parser {
    pub(super) fn native_unit(&mut self) -> Result<NativeUnit, Box<Diagnostic>> {
        self.expect(K::Language)?;
        let language = self.string()?;
        if language.value != LANGUAGE {
            return Err(self.failure(
                Code::UnknownLanguage,
                Phase::Profile,
                language.span,
                "supported language is ix:native",
            ));
        }
        self.expect(K::Edition)?;
        let edition = self.string()?;
        if edition.value == crate::syntax::EDITION {
            for token in &mut self.tokens {
                token.kind = token
                    .kind
                    .clone()
                    .historical(self.source.slice(token.span).expect("token span"));
            }
            self.at = 0;
            return self.unit().map(NativeUnit::Historical);
        }
        if edition.value != c::EDITION {
            return Err(self.failure(
                Code::UnknownEdition,
                Phase::Profile,
                edition.span,
                "supported editions are 0-draft and 1-draft",
            ));
        }
        self.composed = true;
        // Complete-facet and old profile refusals are not reserved words in the
        // composed base grammar. The complete parser keeps its own selected
        // reservation set over these same recognized tokens.
        for token in &mut self.tokens {
            let spelling = self.source.slice(token.span).expect("token span");
            token.kind = token.kind.clone().composed_base(spelling);
            if token.kind == K::Unsupported
                && spelling.bytes().enumerate().all(|(index, byte)| {
                    byte == b'_'
                        || byte.is_ascii_alphabetic()
                        || (index > 0 && byte.is_ascii_digit())
                })
            {
                token.kind = K::Identifier(spelling.into());
            }
        }
        self.expect(K::Semicolon)?;
        let mut profiles: Vec<ModelImport> = vec![self.import(K::Profile)?];
        while self.is(K::Profile) {
            profiles.push(self.import(K::Profile)?);
        }
        let mut models = Vec::new();
        while self.is(K::Model) {
            models.push(self.model()?);
        }
        let mut declarations = vec![self.declaration()?];
        while !self.is(K::End) {
            declarations.push(self.declaration()?);
        }
        Ok(NativeUnit::Composed(ComposedUnit {
            source: self.source.clone(),
            language,
            edition,
            profiles,
            models,
            declarations,
            expressions: std::mem::take(&mut self.values),
            temporal: std::mem::take(&mut self.temporal),
            controls: std::mem::take(&mut self.controls),
        }))
    }

    // Every composed syntax record and arena node shares this inclusive budget.
    // Header/token spans and child handles do not allocate additional syntax nodes.
    pub(super) fn charge(&mut self, span: Span) -> Result<(), Box<Diagnostic>> {
        if self.syntax_nodes >= self.limits.nodes {
            return Err(self.failure(
                Code::ResourceExhausted,
                Phase::Parse,
                span,
                "syntax node budget exhausted",
            ));
        }
        self.syntax_nodes += 1;
        Ok(())
    }

    pub(super) fn add_value(
        &mut self,
        kind: ValueKind,
        span: Span,
    ) -> Result<ExprId, Box<Diagnostic>> {
        self.charge(span)?;
        let id = ExprId(self.values.len());
        let operator_span = match &kind {
            ValueKind::Product { op, .. } => Some(op.span),
            ValueKind::Query { op, .. } => Some(op.span),
            ValueKind::Invoke { name, .. } => Some(name.span),
            _ => None,
        };
        self.values.push(Expression {
            kind,
            span,
            operator_span,
        });
        Ok(id)
    }

    pub(super) fn member(&mut self) -> Result<Spanned<String>, Box<Diagnostic>> {
        if !self.composed {
            return self.identifier();
        }
        if !self.peek().kind.is_word() {
            return Err(self.unexpected("qualified ASCII member name"));
        }
        let token = self.take();
        Ok(Spanned {
            value: self.source.slice(token.span).expect("token span").into(),
            span: token.span,
        })
    }

    fn unsigned(&mut self) -> Result<Spanned<String>, Box<Diagnostic>> {
        if let K::Integer(value) = self.peek().kind.clone() {
            let span = self.take().span;
            Ok(Spanned { value, span })
        } else {
            Err(self.unexpected("unsigned integer"))
        }
    }

    fn signed(&mut self) -> Result<Spanned<String>, Box<Diagnostic>> {
        let start = self.peek().span.start;
        let negative = self.eat(K::Minus);
        let mut integer = self.unsigned()?;
        if negative {
            integer.value.insert(0, '-');
        }
        integer.span.start = start;
        Ok(integer)
    }

    fn range_from(&self, start: usize) -> Span {
        Span {
            start,
            end: self.tokens[self.at - 1].span.end,
        }
    }

    fn qualified(&mut self) -> Result<QualifiedName, Box<Diagnostic>> {
        let model = self.identifier()?;
        self.expect(K::Qualify)?;
        let name = self.member()?;
        Ok(QualifiedName { model, name })
    }

    fn operation(&mut self) -> Result<Operation, Box<Diagnostic>> {
        let context = self.qualified()?;
        self.expect(K::Qualify)?;
        let name = self.member()?;
        Ok(Operation { context, name })
    }

    fn parameter(&mut self) -> Result<Parameter, Box<Diagnostic>> {
        self.charge(self.peek().span)?;
        let name = self.identifier()?;
        let start = name.span.start;
        self.expect(K::Colon)?;
        let ty = if self.is(K::BooleanType) {
            ParameterType::Boolean(self.take().span)
        } else {
            ParameterType::Model(self.qualified()?)
        };
        Ok(Parameter {
            name,
            ty,
            span: self.range_from(start),
        })
    }

    fn bound_parameter(&mut self) -> Result<Parameter, Box<Diagnostic>> {
        self.expect(K::OpenParen)?;
        let parameter = self.parameter()?;
        self.expect(K::CloseParen)?;
        Ok(parameter)
    }

    fn value_block(&mut self) -> Result<ExprId, Box<Diagnostic>> {
        self.expect(K::OpenBrace)?;
        let expression = self.expression()?;
        self.expect(K::CloseBrace)?;
        Ok(expression)
    }

    fn activation(&mut self) -> Result<Activation, Box<Diagnostic>> {
        self.charge(self.peek().span)?;
        let start = self.expect(K::On)?.span.start;
        if self.eat(K::Origin) {
            return Ok(Activation::Origin {
                span: self.range_from(start),
            });
        }
        self.expect(K::Each)?;
        let trigger = self.bound_parameter()?;
        let guard = if self.eat(K::When) {
            self.expect(K::OpenParen)?;
            let expression = self.expression()?;
            self.expect(K::CloseParen)?;
            Some(expression)
        } else {
            None
        };
        Ok(Activation::Each {
            trigger,
            guard,
            span: self.range_from(start),
        })
    }

    fn captures(&mut self) -> Result<Vec<Capture>, Box<Diagnostic>> {
        let mut captures = Vec::new();
        while self.is(K::Capture) {
            self.charge(self.peek().span)?;
            let start = self.take().span.start;
            let parameter = self.parameter()?;
            self.expect(K::Equal)?;
            let value = self.expression()?;
            self.expect(K::Semicolon)?;
            captures.push(Capture {
                parameter,
                value,
                span: self.range_from(start),
            });
        }
        Ok(captures)
    }

    fn interval(&mut self) -> Result<Interval, Box<Diagnostic>> {
        self.charge(self.peek().span)?;
        let start = self.expect(K::OpenBracket)?.span.start;
        let lower = self.unsigned()?;
        self.expect(K::Comma)?;
        let upper = self.unsigned()?;
        self.expect(K::CloseBracket)?;
        Ok(Interval {
            lower,
            upper,
            span: self.range_from(start),
        })
    }

    fn declaration(&mut self) -> Result<Declaration, Box<Diagnostic>> {
        self.charge(self.peek().span)?;
        let token = self.take();
        if !matches!(
            token.kind,
            K::Predicate | K::Invariant | K::Pre | K::Post | K::Temporal | K::Protocol
        ) {
            return Err(self.failure(
                Code::InvalidSyntax,
                Phase::Parse,
                token.span,
                "expected predicate, state, temporal or protocol declaration",
            ));
        }
        let name = self.identifier()?;
        self.expect(K::Using)?;
        let profile = self.identifier()?;
        let kind = match token.kind {
            K::Predicate => {
                self.expect(K::OpenParen)?;
                let mut parameters = Vec::new();
                if !self.is(K::CloseParen) {
                    parameters.push(self.parameter()?);
                    while self.eat(K::Comma) {
                        parameters.push(self.parameter()?);
                    }
                }
                self.expect(K::CloseParen)?;
                self.expect(K::Colon)?;
                let result = self.expect(K::BooleanType)?.span;
                let body = self.value_block()?;
                DeclarationKind::Predicate {
                    parameters,
                    result,
                    body,
                }
            }
            K::Invariant | K::Pre | K::Post => {
                self.expect(K::On)?;
                let context = self.qualified()?;
                let kind = match token.kind {
                    K::Invariant => ClauseKind::Invariant,
                    K::Pre => ClauseKind::Precondition,
                    _ => ClauseKind::Postcondition,
                };
                let operation = if kind == ClauseKind::Invariant {
                    self.expect(K::At)?;
                    self.expect(K::Current)?;
                    None
                } else {
                    self.expect(K::Qualify)?;
                    Some(self.member()?)
                };
                let body = self.value_block()?;
                DeclarationKind::State {
                    kind,
                    context,
                    operation,
                    body,
                }
            }
            K::Temporal => {
                self.expect(K::Over)?;
                let input = self.bound_parameter()?;
                self.expect(K::Clock)?;
                let clock = self.string()?;
                let activation = Box::new(self.activation()?);
                self.expect(K::OpenBrace)?;
                let captures = self.captures()?;
                let formula = self.temporal_expression(0)?;
                self.expect(K::CloseBrace)?;
                DeclarationKind::Temporal {
                    input,
                    clock,
                    activation,
                    captures,
                    formula,
                }
            }
            K::Protocol => DeclarationKind::Protocol(Box::new(self.protocol()?)),
            _ => unreachable!("declaration kind checked"),
        };
        Ok(Declaration {
            name,
            profile,
            kind,
            span: self.range_from(token.span.start),
        })
    }

    pub(super) fn extended_primary(&mut self) -> Result<Option<ExprId>, Box<Diagnostic>> {
        let token = self.peek().clone();
        let kind = match token.kind.clone() {
            K::Identifier(_)
                if self
                    .tokens
                    .get(self.at + 1)
                    .is_some_and(|t| t.kind == K::OpenParen) =>
            {
                let name = self.identifier()?;
                self.expect(K::OpenParen)?;
                let arguments = self.value_list(K::CloseParen)?;
                self.expect(K::CloseParen)?;
                ValueKind::Invoke { name, arguments }
            }
            K::Rational => {
                self.take();
                self.expect(K::OpenParen)?;
                let numerator = self.signed()?;
                self.expect(K::Comma)?;
                let denominator = self.signed()?;
                self.expect(K::CloseParen)?;
                ValueKind::Rational {
                    numerator,
                    denominator,
                }
            }
            K::Size
                if self
                    .tokens
                    .get(self.at + 1)
                    .is_some_and(|t| t.kind == K::Less) =>
            {
                self.take();
                self.expect(K::Less)?;
                let domain = self.qualified()?;
                self.expect(K::Greater)?;
                self.expect(K::OpenParen)?;
                let argument = self.expression()?;
                self.expect(K::CloseParen)?;
                ValueKind::Size { domain, argument }
            }
            K::Contains => {
                self.take();
                self.expect(K::OpenParen)?;
                let collection = self.expression()?;
                self.expect(K::Comma)?;
                let member = self.expression()?;
                self.expect(K::CloseParen)?;
                ValueKind::Contains { collection, member }
            }
            K::Filter | K::Map | K::Count | K::Sum => {
                self.take();
                let op = match token.kind {
                    K::Filter => QueryOp::Filter,
                    K::Map => QueryOp::Map,
                    K::Count => QueryOp::Count,
                    _ => QueryOp::Sum,
                };
                let result = if matches!(op, QueryOp::Count | QueryOp::Sum) {
                    self.expect(K::Less)?;
                    let ty = self.qualified()?;
                    self.expect(K::Greater)?;
                    Some(ty)
                } else {
                    None
                };
                self.expect(K::OpenParen)?;
                let binder = self.identifier()?;
                self.expect(K::In)?;
                let domain = self.expression()?;
                self.expect(K::Colon)?;
                let body = self.expression()?;
                self.expect(K::CloseParen)?;
                ValueKind::Query {
                    op: Spanned {
                        value: op,
                        span: token.span,
                    },
                    result,
                    binder,
                    domain,
                    body,
                }
            }
            _ => return Ok(None),
        };
        let id = self.add_value(kind, self.range_from(token.span.start))?;
        self.values[id.0].operator_span = Some(token.span);
        Ok(Some(id))
    }

    fn value_list(&mut self, close: K) -> Result<Vec<ExprId>, Box<Diagnostic>> {
        let mut values = Vec::new();
        if !self.is(close) {
            values.push(self.expression()?);
            while self.eat(K::Comma) {
                values.push(self.expression()?);
            }
        }
        Ok(values)
    }
}
