// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-138 "S3": the check of a refinement's `history` rows.
//!
//! A history row adds one field to every object of a concrete object type,
//! for the refinement only: it has a bounded declared type, an initial
//! literal and at most one update per concrete operation of that type. The
//! checker resolves the type and each operation, rejects a name the type
//! already declares, decides the literal against the declared type's value
//! set statically, and types each update against the declared type with
//! `self` bound to the step's receiver. The field enters the typer's scope
//! as a [`ScopedHistory`], so a field row, a step-row argument or an update
//! reads it; no concrete clause is checked with that scope, so none can.

use std::collections::BTreeMap;

use qsl_forms::{
    ExactNumberKind, HistoryLiteralForm, HistoryRowForm, RefinementForm, RefinementRowForm,
};
use qsl_foundation::Span;
use quire_exact::{
    admit_text, Decimal, EffectiveId, IllTypedCause, Meter, OptionValue, Outcome, Rational,
    TextPayload, Value, ValueType,
};
use quire_semantic_value::location::{Location, Origin};

use super::{CheckedRow, Checker, RefinementCause, RowContext};
use crate::check::check::ScopedHistory;
use crate::check::refusal::CheckCause;
use crate::check::state_clause::{ClauseDeclarations, ClauseOperation};
use crate::check::type_form::resolve_type_form;
use crate::family::CheckContext;
use crate::model::key::DeclarationKey;

/// One concrete operation's update of a history field, resolved.
#[derive(Debug)]
pub struct HistoryUpdate {
    /// The concrete operation.
    pub operation: DeclarationKey,
    /// The new value over the step: `self` is the receiver, then the
    /// operation's parameters.
    pub expression: CheckedRow,
}

/// `history C::T.field : Type = initial { on C::T::op : e; ... }`, resolved
/// (FR-138 `HistoryRow`).
#[derive(Debug)]
pub struct HistoryRow {
    /// The concrete object type the field is added to.
    pub object_type: DeclarationKey,
    /// The field's name.
    pub field: String,
    /// The field's declared type.
    pub declared: ValueType,
    /// The value every object starts with, and an object a step creates
    /// takes.
    pub initial: Value,
    /// The updates, in source order.
    pub updates: Vec<HistoryUpdate>,
}

/// A history row whose type, name and initial literal checked.
pub(super) struct Header<'f> {
    form: &'f HistoryRowForm,
    object: EffectiveId,
    object_name: String,
    object_key: DeclarationKey,
    declared: ValueType,
    initial: Value,
}

/// Whether `declared` has a finite domain a history literal can name
/// (FR-138 "S3": a bounded value type).
fn bounded(declared: &ValueType) -> bool {
    match declared {
        ValueType::Boolean
        | ValueType::Int(_)
        | ValueType::Enum(_)
        | ValueType::Text(_)
        | ValueType::Rational(_)
        | ValueType::Decimal(_) => true,
        ValueType::Option(payload) => bounded(payload),
        ValueType::Integer
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::String
        | ValueType::Uuid
        | ValueType::Timestamp
        | ValueType::Composite(_)
        | ValueType::Collection(_)
        | ValueType::Reference(_)
        | ValueType::Population(_) => false,
    }
}

fn mismatch() -> RefinementCause {
    RefinementCause::Check(CheckCause::IllTyped(IllTypedCause::TypeMismatch))
}

impl<'c, 'a> Checker<'c, 'a> {
    /// The history rows of `form` whose type, name and initial literal
    /// check. Their fields are in the typer's scope from here on, so every
    /// row and update is typed with every field in view.
    pub(super) fn history_headers(&mut self, form: &'c RefinementForm) -> Vec<Header<'c>> {
        let types = self.types();
        let concrete_types = {
            let found = self.concrete_side.object_types(types);
            self.or_fault(found)
        };
        // The history fields declared so far, by object type and name.
        let mut seen: Vec<(EffectiveId, &str, Span)> = Vec::new();
        let mut headers = Vec::new();
        for row in &form.rows {
            let RefinementRowForm::History(row) = row else {
                continue;
            };
            let Some(object) = concrete_types
                .iter()
                .find(|object| object.name == row.object.name)
            else {
                self.findings
                    .unresolved(&row.object.name, row.object.span, None);
                continue;
            };
            let name = row.field.name.as_str();
            let declared_by_type = types.attribute(object.id, name).is_some();
            // A field of a type and a field of its ancestor or subtype with one
            // name would be one name on the subtype's objects.
            let earlier = seen
                .iter()
                .find(|(other, other_name, _)| {
                    *other_name == name
                        && (types.conforms(*other, object.id) || types.conforms(object.id, *other))
                })
                .map(|(_, _, span)| *span);
            if declared_by_type || earlier.is_some() {
                self.findings.refuse(
                    row.field.span,
                    RefinementCause::Conflicting {
                        name: format!("{}.{name}", object.name),
                        first: earlier.unwrap_or(row.field.span),
                    },
                );
                continue;
            }
            seen.push((object.id, name, row.field.span));
            let location = Location::root(Origin::Expression);
            let declared = match resolve_type_form(self.declarations.scope, &row.ty, &location) {
                Ok(declared) => declared,
                Err(refusal) => {
                    self.findings
                        .refuse(row.ty.span, RefinementCause::Check(refusal.cause));
                    continue;
                }
            };
            if !bounded(&declared) {
                self.findings.refuse(row.ty.span, mismatch());
                continue;
            }
            let initial = match self.literal(&row.initial, &declared) {
                Ok(initial) if declared.admits(&initial) => initial,
                Ok(_) => {
                    self.findings.refuse(row.initial_span, mismatch());
                    continue;
                }
                Err(cause) => {
                    self.findings.refuse(row.initial_span, cause);
                    continue;
                }
            };
            self.history.push(ScopedHistory {
                object_type: object.id,
                name: name.to_owned(),
                value_type: declared.clone(),
            });
            headers.push(Header {
                form: row,
                object: object.id,
                object_name: object.name.clone(),
                object_key: object.key.clone(),
                declared,
                initial,
            });
        }
        headers
    }

    /// `literal` as a value of `declared`, or the refusal that says why it
    /// is not one (QSpec FR-375's history-literal rule). Whether the value
    /// is a member of the declared type's value set is the caller's check.
    fn literal(
        &self,
        literal: &HistoryLiteralForm,
        declared: &ValueType,
    ) -> Result<Value, RefinementCause> {
        match (literal, declared) {
            (HistoryLiteralForm::Integer(integer), ValueType::Int(_)) => {
                Ok(Value::Integer(integer.clone()))
            }
            (HistoryLiteralForm::Boolean(value), ValueType::Boolean) => Ok(Value::Boolean(*value)),
            (HistoryLiteralForm::None, ValueType::Option(payload)) => {
                Ok(OptionValue::none((**payload).clone()))
            }
            (HistoryLiteralForm::Text(text), ValueType::Text(text_type)) => {
                let payload = TextPayload::from_source_literal(text.as_str(), format!("{text:?}"));
                match admit_text(
                    &payload,
                    text_type,
                    &mut Meter::new(crate::check::UNBOUNDED_SCALAR_LIMITS),
                ) {
                    Outcome::Completed(text) => Ok(Value::Text(text)),
                    _ => Err(mismatch()),
                }
            }
            (HistoryLiteralForm::Exact(number), ValueType::Rational(_))
                if number.kind == ExactNumberKind::Rational =>
            {
                Rational::new(number.first.clone(), number.second.clone())
                    .map(Value::Rational)
                    .map_err(|_| mismatch())
            }
            (HistoryLiteralForm::Exact(number), ValueType::Decimal(_))
                if number.kind == ExactNumberKind::Decimal =>
            {
                let scale = number
                    .second
                    .to_u64()
                    .and_then(|scale| u32::try_from(scale).ok())
                    .ok_or_else(mismatch)?;
                Ok(Value::Decimal(Decimal::new(number.first.clone(), scale)))
            }
            (HistoryLiteralForm::EnumMember(name), ValueType::Enum(shape)) => {
                let scope = self.declarations.scope;
                // The path is written with the model's alias; the scope
                // names an enum by its declared name.
                let members: Vec<_> = scope.enum_members_named(&name.name).collect();
                match members.as_slice() {
                    [] => Err(RefinementCause::Unresolved {
                        name: name.name.clone(),
                        held_by: None,
                    }),
                    [(binding, member)] if binding.shape() == *shape => {
                        Ok(crate::check::check::enum_value(binding, member))
                    }
                    _ => Err(mismatch()),
                }
            }
            (literal, ValueType::Option(payload)) => {
                let inner = self.literal(literal, payload)?;
                OptionValue::present((**payload).clone(), inner).map_err(|_| mismatch())
            }
            _ => Err(mismatch()),
        }
    }

    /// The history rows of `headers`, each with its updates checked.
    pub(super) fn history_rows(
        &mut self,
        headers: Vec<Header<'c>>,
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> Vec<HistoryRow> {
        let types = self.types();
        let concrete_types = {
            let found = self.concrete_side.object_types(types);
            self.or_fault(found)
        };
        let mut rows = Vec::new();
        for header in headers {
            let mut firsts: BTreeMap<String, Span> = BTreeMap::new();
            let mut updates = Vec::new();
            let mut complete = true;
            for update in &header.form.updates {
                let Some((object, declaring, concrete)) =
                    self.operation_named(&update.operation, &concrete_types, None)
                else {
                    complete = false;
                    continue;
                };
                if object.id != header.object {
                    // An operation of another type is no operation of this
                    // row's type.
                    self.findings
                        .malformed(&update.operation.name, update.operation.span);
                    complete = false;
                    continue;
                }
                if let Some(first) = firsts.get(concrete.name()) {
                    self.findings.refuse(
                        update.span,
                        RefinementCause::Conflicting {
                            name: format!("{}::{}", header.object_name, concrete.name()),
                            first: *first,
                        },
                    );
                    complete = false;
                    continue;
                }
                firsts.insert(concrete.name().to_owned(), update.span);
                let key = self.concrete_side.model.operation_key(&ClauseOperation {
                    declaring,
                    declaration: concrete.clone(),
                });
                let Some(key) = key.cloned() else {
                    self.fault("concrete-operation-has-a-declaration-key");
                    complete = false;
                    continue;
                };
                let mut parameters = vec![("self".to_owned(), ValueType::Reference(object.id))];
                parameters.extend(concrete.parameters().iter().cloned());
                let expression = self.check_row(
                    &update.value,
                    &header.declared,
                    RowContext {
                        parameters,
                        kind: qsl_forms::StateClauseKind::Postcondition,
                        operation: Some(concrete.name().to_owned()),
                        label: format!("{}.{}", header.object_name, header.form.field.name),
                        range_agnostic: true,
                    },
                    cx,
                );
                match expression {
                    Some(expression) => updates.push(HistoryUpdate {
                        operation: key,
                        expression,
                    }),
                    None => complete = false,
                }
            }
            if complete {
                rows.push(HistoryRow {
                    object_type: header.object_key,
                    field: header.form.field.name.clone(),
                    declared: header.declared,
                    initial: header.initial,
                    updates,
                });
            }
        }
        rows
    }
}
