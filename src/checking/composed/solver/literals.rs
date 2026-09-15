// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact contextual literals using the existing IR rational normalizer.
use super::*;
use std::collections::BTreeSet;

impl Solver<'_, '_, '_, '_> {
    pub(super) fn literals(&mut self) -> Result<()> {
        let mut negative = BTreeSet::new();
        for index in self.range.clone() {
            self.work
                .charge(D::Constraints, 1, self.site(ExprId(index)))?;
            if let c::ValueKind::Shared(ExprKind::Unary {
                op: UnaryOp::Negate,
                argument,
            }) = &self.unit.expressions()[index].kind
            {
                if matches!(
                    self.unit.expressions()[argument.0].kind,
                    c::ValueKind::Shared(ExprKind::Integer(_))
                ) {
                    self.work.charge(D::Records, 1, self.site(*argument))?;
                    negative.insert(argument.0);
                }
            }
        }
        for index in self.range.clone() {
            let at = ExprId(index);
            let ty = self.get(self.var(at), at)?;
            match &self.unit.expressions()[index].kind {
                c::ValueKind::Shared(ExprKind::Integer(text)) => {
                    if negative.contains(&index) {
                        continue;
                    }
                    self.integer_literal(text, false, ty.as_ref(), at)?;
                }
                c::ValueKind::Shared(ExprKind::Unary {
                    op: UnaryOp::Negate,
                    argument,
                }) => {
                    if let c::ValueKind::Shared(ExprKind::Integer(text)) =
                        &self.unit.expressions()[argument.0].kind
                    {
                        self.integer_literal(text, true, ty.as_ref(), at)?;
                    }
                }
                c::ValueKind::Shared(ExprKind::Text(text)) => {
                    self.work.charge(D::Bytes, text.len(), self.site(at))?;
                    if let Some(ty) = ty {
                        match ty {
                            NativeType::Scalar { role, .. } => match role.kind {
                                ScalarKind::Text { max_scalars } => {
                                    if text.chars().count()
                                        > usize::try_from(max_scalars).unwrap_or(usize::MAX)
                                    {
                                        self.cause(at, CauseKind::LiteralDomain)?;
                                    }
                                }
                                ScalarKind::Integer { .. } | ScalarKind::Rational { .. } => {
                                    self.cause(at, CauseKind::TypeMismatch)?
                                }
                            },
                            NativeType::Boolean
                            | NativeType::Enumeration { .. }
                            | NativeType::Record { .. }
                            | NativeType::Object { .. }
                            | NativeType::Reference { .. }
                            | NativeType::Option(_)
                            | NativeType::Sequence { .. } => {
                                self.cause(at, CauseKind::TypeMismatch)?
                            }
                        }
                    }
                }
                c::ValueKind::Rational {
                    numerator,
                    denominator,
                } => {
                    self.work.charge(
                        D::Bytes,
                        numerator.value.len() + denominator.value.len(),
                        self.site(at),
                    )?;
                    let parsed_denominator = denominator.value.parse::<i64>();
                    if parsed_denominator == Ok(0) {
                        self.cause(at, CauseKind::ZeroDenominator)?;
                        continue;
                    }
                    let (Ok(raw_numerator), Ok(raw_denominator)) =
                        (numerator.value.parse::<i64>(), parsed_denominator)
                    else {
                        self.cause(
                            at,
                            CauseKind::UnsupportedPrerequisite(Prerequisite::RationalNormalization),
                        )?;
                        continue;
                    };
                    let Some(ty) = ty else { continue };
                    let Some(domain) = ty.rational() else {
                        self.cause(at, CauseKind::TypeMismatch)?;
                        continue;
                    };
                    let Some(formal) = self.formal else {
                        self.cause(
                            at,
                            CauseKind::UnsupportedPrerequisite(Prerequisite::FormalSource),
                        )?;
                        continue;
                    };
                    let Ok(source) = formal.to_ir(self.unit.source(), self.site(at).span) else {
                        self.cause(
                            at,
                            CauseKind::UnsupportedPrerequisite(Prerequisite::FormalSource),
                        )?;
                        continue;
                    };
                    let denominator = if raw_numerator == 0 {
                        1
                    } else {
                        let Some(positive) = raw_denominator.checked_abs() else {
                            self.cause(
                                at,
                                CauseKind::UnsupportedPrerequisite(
                                    Prerequisite::RationalNormalization,
                                ),
                            )?;
                            continue;
                        };
                        positive
                    };
                    self.work.charge(D::Normalization, 128, self.site(at))?;
                    let normalized = ir::Expression::new(
                        ir::ExpressionKind::RationalLiteral {
                            numerator: raw_numerator,
                            denominator,
                            value_type: domain.clone(),
                        },
                        source,
                    );
                    let ir::ExpressionKind::RationalLiteral {
                        numerator,
                        denominator,
                        ..
                    } = normalized.kind()
                    else {
                        unreachable!("literal constructor")
                    };
                    let numerator = if raw_denominator < 0 {
                        let Some(value) = numerator.checked_neg() else {
                            self.cause(
                                at,
                                CauseKind::UnsupportedPrerequisite(
                                    Prerequisite::RationalNormalization,
                                ),
                            )?;
                            continue;
                        };
                        value
                    } else {
                        *numerator
                    };
                    if numerator < domain.numerator_minimum()
                        || numerator > domain.numerator_maximum()
                        || u64::try_from(*denominator)
                            .ok()
                            .is_none_or(|value| value > domain.maximum_denominator())
                    {
                        self.cause(at, CauseKind::LiteralDomain)?;
                    }
                    let local = self.var(at);
                    self.output.nodes[local].normalized_rational = Some((numerator, *denominator));
                }
                c::ValueKind::Shared(
                    ExprKind::Group { .. }
                    | ExprKind::Boolean(_)
                    | ExprKind::Name(_)
                    | ExprKind::SelfValue
                    | ExprKind::ResultValue
                    | ExprKind::EnumValue { .. }
                    | ExprKind::Field { .. }
                    | ExprKind::Unary {
                        op: UnaryOp::Not, ..
                    }
                    | ExprKind::Binary { .. }
                    | ExprKind::Call { .. }
                    | ExprKind::Let { .. }
                    | ExprKind::If { .. }
                    | ExprKind::Quantifier { .. }
                    | ExprKind::Reaches { .. },
                )
                | c::ValueKind::Invoke { .. }
                | c::ValueKind::Product { .. }
                | c::ValueKind::Size { .. }
                | c::ValueKind::Contains { .. }
                | c::ValueKind::Query { .. } => {}
            }
        }
        Ok(())
    }
    fn integer_literal(
        &mut self,
        text: &str,
        negative: bool,
        ty: Option<&NativeType<'_>>,
        at: ExprId,
    ) -> Result<()> {
        self.work.charge(D::Bytes, text.len(), self.site(at))?;
        let Some(ty) = ty else { return Ok(()) };
        let Some(domain) = ty.integer() else {
            self.cause(at, CauseKind::TypeMismatch)?;
            return Ok(());
        };
        let parsed = text.parse::<i128>().ok().and_then(|value| {
            if negative {
                value.checked_neg()
            } else {
                Some(value)
            }
        });
        if !parsed.is_some_and(|value| {
            value >= i128::from(domain.minimum()) && value <= i128::from(domain.maximum())
        }) {
            self.cause(at, CauseKind::LiteralDomain)?;
        }
        Ok(())
    }
}
