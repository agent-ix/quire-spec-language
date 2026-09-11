// SPDX-License-Identifier: AGPL-3.0-only
//! FR-036/040/042: executable pure values derived from the original typed AST.
//! Symbolic proof witnesses are deliberately not an input to this lowering.

use crate::checking::composed::{DeclarationTypes, ObservationOrigin};
use crate::checking::NativeType;
use crate::linking::composed::scopes::{self, DeclarationScope};
use crate::protocol_artifact::{
    wire as w, work::Work, Dimension, Error, ExactRational, Invalid, NumberWire, Unsupported,
};
use crate::syntax::composed::{self as c, ComposedUnit};
use crate::syntax::{BinaryOp, Builtin, ExprId, ExprKind, UnaryOp};

use super::layout::DeclLayout;
use super::types::{index, integer, text, ValueBuilder};

struct Context<'s, 'm> {
    typed: &'s DeclarationTypes<'m>,
    unit: &'s ComposedUnit,
    scope: &'s DeclarationScope,
    layout: &'s DeclLayout,
}

impl ValueBuilder<'_> {
    pub(super) fn lower_values(
        &mut self,
        typed: &DeclarationTypes<'_>,
        unit: &ComposedUnit,
        scope: &DeclarationScope,
        layout: &DeclLayout,
        profile_indices: &[u32],
        work: &mut Work,
    ) -> Result<Vec<w::Value>, Error> {
        if typed.unit() != scope.unit
            || typed.declaration() != scope.declaration
            || !typed.complete()
            || !typed.causes().is_empty()
        {
            return Err(Error::Invalid(Invalid::Type));
        }
        let context = Context {
            typed,
            unit,
            scope,
            layout,
        };
        let mut result = Vec::new();
        for &at in &layout.values {
            work.visit()?;
            let syntax = unit
                .expression(at)
                .ok_or(Error::Invalid(Invalid::Reference))?;
            let node = typed.node(at).ok_or(Error::Invalid(Invalid::Type))?;
            let locus = layout.locus(syntax.span)?;
            work.locus = Some(locus.clone());
            if node.span != syntax.span {
                return Err(Error::Invalid(Invalid::Locus));
            }
            let value_type =
                self.ty(node.ty.as_ref().ok_or(Error::Invalid(Invalid::Type))?, work)?;
            let profile = *profile_indices
                .get(node.profile)
                .ok_or(Error::Invalid(Invalid::Profile))?;
            let anchor = layout.value_anchor(at)?;
            let origin = match node.origin.ok_or(Error::Invalid(Invalid::Owner))? {
                ObservationOrigin::Independent => w::Origin::Independent {},
                ObservationOrigin::Anchored(anchor) => w::Origin::Anchor {
                    anchor: layout.anchor(anchor)?,
                },
                ObservationOrigin::Selected { unit, expression } => {
                    if unit != typed.unit() {
                        return Err(Error::Invalid(Invalid::Owner));
                    }
                    w::Origin::Selected {
                        value: layout.value(expression)?,
                    }
                }
            };
            let operation = self.operation(&context, at, value_type, work)?;
            work.charge(Dimension::Entries, 1)?;
            result.push(w::Value {
                original_expression: index(at.0)?,
                locus,
                operator_locus: w::Nullable(
                    syntax
                        .operator_span
                        .map(|span| layout.locus(span))
                        .transpose()?,
                ),
                value_type,
                profile,
                scope: layout.scope(at)?,
                anchor,
                origin,
                operation,
            });
        }
        Ok(result)
    }

    fn operation(
        &mut self,
        context: &Context<'_, '_>,
        at: ExprId,
        result: u32,
        work: &mut Work,
    ) -> Result<w::ValueOperation, Error> {
        let layout = context.layout;
        let node = context
            .typed
            .node(at)
            .ok_or(Error::Invalid(Invalid::Type))?;
        let syntax = context
            .unit
            .expression(at)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        Ok(match &syntax.kind {
            c::ValueKind::Shared(kind) => match kind {
                ExprKind::Group { inner } => w::ValueOperation::Group {
                    value: layout.value(*inner)?,
                },
                ExprKind::Boolean(value) => w::ValueOperation::Boolean { value: *value },
                ExprKind::Integer(value) => {
                    work.bytes(value.len())?;
                    let value = value
                        .parse::<i64>()
                        .map_err(|_| Error::Invalid(Invalid::NumericDomain))?;
                    w::ValueOperation::Number {
                        value: w::Number(integer(value, work)?.0),
                    }
                }
                ExprKind::Text(value) => w::ValueOperation::Text {
                    value: text(value, work)?,
                },
                ExprKind::Name(_) | ExprKind::SelfValue | ExprKind::ResultValue => {
                    let binder = node.binder.ok_or(Error::Invalid(Invalid::Binding))?;
                    w::ValueOperation::Read {
                        binder: layout.binder(binder.index())?,
                    }
                }
                ExprKind::EnumValue { variant, .. } => {
                    let NativeType::Enumeration { model, declaration } = context.ty(at)? else {
                        return Err(Error::Invalid(Invalid::Type));
                    };
                    w::ValueOperation::Enum {
                        variant: self.export(
                            model,
                            w::ExportKind::Variant,
                            declaration.name().as_str(),
                            Some(&variant.value),
                            work,
                        )?,
                    }
                }
                ExprKind::Field { base, name } => {
                    let (model, record) = match context.ty(*base)? {
                        NativeType::Record { model, declaration } => {
                            (*model, declaration.name().as_str())
                        }
                        NativeType::Object { model, role } => (*model, role.record.as_str()),
                        NativeType::Boolean
                        | NativeType::Scalar { .. }
                        | NativeType::Enumeration { .. }
                        | NativeType::Reference { .. }
                        | NativeType::Option(_)
                        | NativeType::Sequence { .. } => return Err(Error::Invalid(Invalid::Type)),
                    };
                    w::ValueOperation::Field {
                        base: layout.value(*base)?,
                        field: self.export(
                            model,
                            w::ExportKind::Field,
                            record,
                            Some(&name.value),
                            work,
                        )?,
                    }
                }
                ExprKind::Unary { op, argument } => {
                    if *op == UnaryOp::Negate {
                        if let c::ValueKind::Shared(ExprKind::Integer(value)) = &context
                            .unit
                            .expression(*argument)
                            .ok_or(Error::Invalid(Invalid::Reference))?
                            .kind
                        {
                            work.bytes(value.len())?;
                            let value = value
                                .parse::<i128>()
                                .ok()
                                .and_then(i128::checked_neg)
                                .and_then(|value| i64::try_from(value).ok())
                                .ok_or(Error::Invalid(Invalid::NumericDomain))?;
                            return Ok(w::ValueOperation::Number {
                                value: w::Number(integer(value, work)?.0),
                            });
                        }
                    }
                    w::ValueOperation::Unary {
                        operator: match op {
                            UnaryOp::Not => w::Unary::Not,
                            UnaryOp::Negate => w::Unary::Negate,
                        },
                        value: layout.value(*argument)?,
                    }
                }
                ExprKind::Binary { op, left, right } => w::ValueOperation::Binary {
                    operator: binary(*op)?,
                    left: layout.value(*left)?,
                    right: layout.value(*right)?,
                },
                ExprKind::Call { builtin, argument } => match builtin {
                    Builtin::Pre => {
                        // The actual scope pass selects invocation pre and type
                        // admission checks read eligibility and retained origins.
                        w::ValueOperation::Pre {
                            value: layout.value(*argument)?,
                            anchor: layout.anchor(scopes::Anchor::InvocationPre)?,
                        }
                    }
                    Builtin::Present | Builtin::Value | Builtin::Deref => {
                        w::ValueOperation::Unary {
                            operator: match builtin {
                                Builtin::Present => w::Unary::Present,
                                Builtin::Value => w::Unary::Value,
                                Builtin::Deref => w::Unary::Deref,
                                Builtin::Size | Builtin::Pre => {
                                    return Err(Error::Invalid(Invalid::Type))
                                }
                            },
                            value: layout.value(*argument)?,
                        }
                    }
                    Builtin::Size => w::ValueOperation::Size {
                        collection: layout.value(*argument)?,
                        result,
                    },
                },
                ExprKind::Let { name, value, body } => w::ValueOperation::Let {
                    binder: context.local_binder(name.span, scopes::BinderKind::Let, work)?,
                    initializer: layout.value(*value)?,
                    body: layout.value(*body)?,
                },
                ExprKind::If {
                    condition,
                    then_value,
                    else_value,
                } => w::ValueOperation::If {
                    condition: layout.value(*condition)?,
                    then_value: layout.value(*then_value)?,
                    else_value: layout.value(*else_value)?,
                },
                ExprKind::Quantifier {
                    universal,
                    name,
                    domain,
                    predicate,
                } => w::ValueOperation::Query {
                    operator: if *universal {
                        w::Query::ForAll
                    } else {
                        w::Query::Exists
                    },
                    binder: context.local_binder(name.span, scopes::BinderKind::Query, work)?,
                    collection: layout.value(*domain)?,
                    body: layout.value(*predicate)?,
                    result,
                },
                ExprKind::Reaches {
                    start,
                    target,
                    field,
                } => {
                    if context.typed.node(*start).and_then(|node| node.origin)
                        != context.typed.node(*target).and_then(|node| node.origin)
                    {
                        return Err(Error::Invalid(Invalid::Binding));
                    }
                    let (model, role) = match context.ty(*start)? {
                        NativeType::Object { model, role }
                        | NativeType::Reference { model, role } => (*model, *role),
                        NativeType::Boolean
                        | NativeType::Scalar { .. }
                        | NativeType::Enumeration { .. }
                        | NativeType::Record { .. }
                        | NativeType::Option(_)
                        | NativeType::Sequence { .. } => return Err(Error::Invalid(Invalid::Type)),
                    };
                    w::ValueOperation::Reaches {
                        start: layout.value(*start)?,
                        target: layout.value(*target)?,
                        edge: self.export(
                            model,
                            w::ExportKind::Field,
                            role.record.as_str(),
                            Some(&field.value),
                            work,
                        )?,
                        universe: self.export(
                            model,
                            w::ExportKind::Population,
                            role.record.as_str(),
                            Some(role.universe.as_str()),
                            work,
                        )?,
                    }
                }
            },
            c::ValueKind::Invoke { arguments, .. } => {
                let mut handles = Vec::new();
                for argument in arguments {
                    work.visit()?;
                    work.charge(Dimension::Entries, 1)?;
                    handles.push(layout.value(*argument)?);
                }
                w::ValueOperation::Call {
                    predicate: layout.call_target(at)?,
                    arguments: handles,
                }
            }
            c::ValueKind::Rational { .. } => {
                let (numerator, denominator) = node
                    .normalized_rational
                    .ok_or(Error::Invalid(Invalid::NumericDomain))?;
                // Validate the existing checker's result; never normalize again.
                work.visit()?;
                ExactRational::new(numerator, denominator)?;
                let NumberWire::Integer { decimal: numerator } = integer(numerator, work)?.0 else {
                    return Err(Error::Invalid(Invalid::WrongNumericKind));
                };
                let NumberWire::Integer {
                    decimal: denominator,
                } = integer(denominator, work)?.0
                else {
                    return Err(Error::Invalid(Invalid::WrongNumericKind));
                };
                w::ValueOperation::Number {
                    value: w::Number(NumberWire::Rational {
                        numerator,
                        denominator,
                    }),
                }
            }
            c::ValueKind::Product { op, left, right } => match op.value {
                c::ProductOp::Slash => w::ValueOperation::Binary {
                    operator: w::Binary::RationalDivide,
                    left: layout.value(*left)?,
                    right: layout.value(*right)?,
                },
                c::ProductOp::Mod => return Err(Error::Unsupported(Unsupported::Feature)),
            },
            c::ValueKind::Size { argument, .. } => w::ValueOperation::Size {
                collection: layout.value(*argument)?,
                result,
            },
            c::ValueKind::Contains { collection, member } => w::ValueOperation::Contains {
                collection: layout.value(*collection)?,
                member: layout.value(*member)?,
            },
            c::ValueKind::Query {
                op,
                binder,
                domain,
                body,
                ..
            } => w::ValueOperation::Query {
                operator: match op.value {
                    c::QueryOp::Filter => w::Query::Filter,
                    c::QueryOp::Map => w::Query::Map,
                    c::QueryOp::Count => w::Query::Count,
                    c::QueryOp::Sum => w::Query::Sum,
                },
                binder: context.local_binder(binder.span, scopes::BinderKind::Query, work)?,
                collection: layout.value(*domain)?,
                body: layout.value(*body)?,
                result,
            },
        })
    }
}

impl<'m> Context<'_, 'm> {
    fn ty(&self, at: ExprId) -> Result<&NativeType<'m>, Error> {
        self.typed
            .node(at)
            .and_then(|node| node.ty.as_ref())
            .ok_or(Error::Invalid(Invalid::Type))
    }

    fn local_binder(
        &self,
        span: crate::Span,
        kind: scopes::BinderKind,
        work: &mut Work,
    ) -> Result<w::Handle, Error> {
        let mut found = None;
        for (index, binder) in self.scope.binders.iter().enumerate() {
            work.visit()?;
            if binder.span == span && binder.kind == kind {
                if found.is_some() {
                    return Err(Error::Invalid(Invalid::Duplicate));
                }
                found = Some(index);
            }
        }
        self.layout
            .binder(found.ok_or(Error::Invalid(Invalid::Binding))?)
    }
}

fn binary(value: BinaryOp) -> Result<w::Binary, Error> {
    Ok(match value {
        BinaryOp::Implies => w::Binary::Implies,
        BinaryOp::Or => w::Binary::Or,
        BinaryOp::And => w::Binary::And,
        BinaryOp::Equal => w::Binary::Equal,
        BinaryOp::NotEqual => w::Binary::NotEqual,
        BinaryOp::Less => w::Binary::Less,
        BinaryOp::LessEqual => w::Binary::LessEqual,
        BinaryOp::Greater => w::Binary::Greater,
        BinaryOp::GreaterEqual => w::Binary::GreaterEqual,
        BinaryOp::Add => w::Binary::Add,
        BinaryOp::Subtract => w::Binary::Subtract,
        BinaryOp::Multiply => w::Binary::Multiply,
        BinaryOp::Divide | BinaryOp::Remainder => {
            return Err(Error::Unsupported(Unsupported::Feature))
        }
    })
}
