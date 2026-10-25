// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-487 "Evaluating a match conjunct": the value a conjunct's slot takes
//! from the step label of its trace variable.

use std::str::FromStr;

use quire_exact::{Integer, OptionValue, Value, ValueType};
use qsl_semantics::check::{operation_text, StepRead, STUTTER_TEXT};

use crate::hyper::StepLabel;
use crate::simulation::model::encode::parse_reference;

/// Why a label gives its read no value: the label's text does not decode at
/// the type the read declares.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Undecodable;

/// The value `read` takes over `label` (FR-487): the step's operation text or
/// `stutter` for an operation, and for a receiver or an argument an option
/// that is none at the stutter, at a step with no receiver and at a step of
/// another operation, and otherwise holds the decoded text.
pub(crate) fn slot_value(read: &StepRead, label: &StepLabel<'_>) -> Result<Value, Undecodable> {
    match (read, label) {
        (StepRead::Operation { .. }, StepLabel::Stutter) => {
            Ok(Value::String(STUTTER_TEXT.to_owned()))
        }
        (StepRead::Operation { .. }, StepLabel::Operation(step)) => {
            Ok(Value::String(step.operation().to_owned()))
        }
        (StepRead::Receiver { object, .. }, label) => {
            let payload = ValueType::Reference(*object);
            let present = match label {
                StepLabel::Stutter => None,
                StepLabel::Operation(step) => match step.receiver() {
                    None => None,
                    Some(text) => Some(Value::Reference(
                        parse_reference(text).ok_or(Undecodable)?,
                    )),
                },
            };
            Ok(OptionValue::from_admitted(payload, present))
        }
        (StepRead::Argument(argument), label) => {
            let payload = argument.parameter_type.clone();
            let present = match label {
                StepLabel::Stutter => None,
                StepLabel::Operation(step) if step.operation() != operation_text(&argument.operation) => {
                    None
                }
                StepLabel::Operation(step) => {
                    let text = step.arguments().get(argument.index).ok_or(Undecodable)?;
                    Some(decode(text, &payload).ok_or(Undecodable)?)
                }
            };
            Ok(OptionValue::from_admitted(payload, present))
        }
        (StepRead::Stutter, _) => Err(Undecodable),
    }
}

/// The value of declared type `value_type` that `text` writes in FR-181's
/// typed canonical form, for the types a step argument is written in here:
/// Boolean, integers, strings, references and options of them. Any other type
/// has no decoding, and `None` stands for it as for a malformed text.
/// `#[string_edge]`: reads the canonical text the model's own encoder wrote.
#[ix_trace_rs::string_edge]
pub(crate) fn decode(text: &str, value_type: &ValueType) -> Option<Value> {
    let value = match value_type {
        ValueType::Boolean => match text {
            r#"{"type":"boolean","value":true}"# => Value::Boolean(true),
            r#"{"type":"boolean","value":false}"# => Value::Boolean(false),
            _ => return None,
        },
        ValueType::Integer | ValueType::Int(_) => {
            let digits = text
                .strip_prefix(r#"{"type":"integer","value":""#)?
                .strip_suffix(r#""}"#)?;
            Value::Integer(Integer::from_str(digits).ok()?)
        }
        ValueType::String => {
            let literal = text
                .strip_prefix(r#"{"type":"text","value":"#)?
                .strip_suffix('}')?;
            Value::String(unquote(literal)?)
        }
        ValueType::Reference(_) => Value::Reference(parse_reference(text)?),
        ValueType::Option(payload) => {
            let inner = text
                .strip_prefix(r#"{"type":"option","value":"#)?
                .strip_suffix('}')?;
            if inner == "null" {
                return Some(OptionValue::none((**payload).clone()));
            }
            let value = decode(inner, payload)?;
            return OptionValue::present((**payload).clone(), value).ok();
        }
        _ => return None,
    };
    value_type.admits(&value).then_some(value)
}

/// The string that the RFC 8785 string literal `literal` writes.
fn unquote(literal: &str) -> Option<String> {
    let body = literal.strip_prefix('"')?.strip_suffix('"')?;
    let mut text = String::with_capacity(body.len());
    let mut characters = body.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            text.push(character);
            continue;
        }
        match characters.next()? {
            '"' => text.push('"'),
            '\\' => text.push('\\'),
            'b' => text.push('\u{8}'),
            'f' => text.push('\u{c}'),
            'n' => text.push('\n'),
            'r' => text.push('\r'),
            't' => text.push('\t'),
            'u' => {
                let digits: String = characters.by_ref().take(4).collect();
                if digits.len() != 4 {
                    return None;
                }
                text.push(char::from_u32(u32::from_str_radix(&digits, 16).ok()?)?);
            }
            _ => return None,
        }
    }
    Some(text)
}

