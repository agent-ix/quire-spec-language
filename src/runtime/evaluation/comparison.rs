// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-008: type-directed equality and exact Unicode scalar ordering under budgets.

use super::*;
use crate::native_model::ScalarSite;
use std::cmp::Ordering;

impl<'a, 'checked, 'model, P: FnMut() -> bool> Evaluator<'a, 'checked, 'model, P> {
    pub(super) fn equal(
        &mut self,
        ty: &NativeType<'model>,
        a: Value<'a>,
        b: Value<'a>,
        span: Span,
        depth: usize,
    ) -> Result<bool> {
        self.budget.comparison(span, depth)?;
        self.equal_admitted(ty, a, b, span, depth)
    }

    fn equal_admitted(
        &mut self,
        ty: &NativeType<'model>,
        a: Value<'a>,
        b: Value<'a>,
        span: Span,
        depth: usize,
    ) -> Result<bool> {
        match (ty, a, b) {
            (NativeType::Boolean, Value::Boolean(a), Value::Boolean(b)) => Ok(a == b),
            (
                NativeType::Scalar {
                    representation: ir::ValueType::Integer { .. },
                    ..
                },
                Value::Integer(a),
                Value::Integer(b),
            ) => Ok(a == b),
            (
                NativeType::Scalar {
                    representation: ir::ValueType::Text,
                    ..
                },
                Value::Text(a),
                Value::Text(b),
            ) => Ok(self.text_order(a, b, span)?.is_eq()),
            (
                NativeType::Enumeration { model, declaration },
                Value::Enumeration {
                    owner: ao,
                    name: an,
                    variant: av,
                },
                Value::Enumeration {
                    owner: bo,
                    name: bn,
                    variant: bv,
                },
            ) => {
                if ao != model.environment().owner()
                    || bo != ao
                    || an != declaration.name()
                    || bn != an
                {
                    return Err(self.invariant(
                        span,
                        "checked enum values have different nominal identities",
                    ));
                }
                Ok(av == bv)
            }
            (
                NativeType::Object { .. },
                Value::Object { identity: a, .. },
                Value::Object { identity: b, .. },
            )
            | (
                NativeType::Reference { .. },
                Value::Reference { identity: a, .. },
                Value::Reference { identity: b, .. },
            ) => Ok(a == b),
            (
                NativeType::Record { model, declaration },
                Value::Record { .. },
                Value::Record { .. },
            ) => {
                let catalog = self.catalog(model.environment().owner(), span)?;
                let fields = catalog
                    .ordered_fields
                    .get(declaration.name())
                    .ok_or_else(|| self.invariant(span, "checked record field index is absent"))?;
                for field in fields {
                    self.budget.poll(span)?;
                    // Charge before fetching/inspecting the child pair or expanding its type.
                    self.budget.comparison(span, depth + 1)?;
                    let ty = catalog
                        .formal(
                            field.value_type(),
                            &ScalarSite::Field {
                                record: declaration.name().clone(),
                                field: field.name().clone(),
                            },
                        )
                        .ok_or_else(|| {
                            self.invariant(span, "checked comparison field type is absent")
                        })?;
                    let a = self.field(a, field.name().as_str(), span)?;
                    let b = self.field(b, field.name().as_str(), span)?;
                    if !self.equal_admitted(&ty, a, b, span, depth + 1)? {
                        return Ok(false);
                    }
                }
                self.budget.poll(span)?;
                Ok(true)
            }
            _ => Err(self.invariant(span, "checked equality eligibility or value shape failed")),
        }
    }

    pub(super) fn order(
        &mut self,
        ty: &NativeType<'model>,
        a: Value<'a>,
        b: Value<'a>,
        span: Span,
    ) -> Result<Ordering> {
        self.budget.comparison(span, 1)?;
        match (ty, a, b) {
            (
                NativeType::Scalar {
                    representation: ir::ValueType::Integer { .. },
                    ..
                },
                Value::Integer(a),
                Value::Integer(b),
            ) => Ok(a.cmp(&b)),
            (
                NativeType::Scalar {
                    representation: ir::ValueType::Text,
                    ..
                },
                Value::Text(a),
                Value::Text(b),
            ) => self.text_order(a, b, span),
            _ => Err(self.invariant(span, "checked ordering eligibility or value shape failed")),
        }
    }

    fn text_order(&mut self, a: &str, b: &str, span: Span) -> Result<Ordering> {
        let mut a = a.chars();
        let mut b = b.chars();
        loop {
            self.budget.poll(span)?;
            self.budget.text(span)?;
            let left = a.next();
            self.budget.text(span)?;
            let right = b.next();
            match left.cmp(&right) {
                Ordering::Equal if left.is_some() => {}
                ordering => return Ok(ordering),
            }
        }
    }
}
