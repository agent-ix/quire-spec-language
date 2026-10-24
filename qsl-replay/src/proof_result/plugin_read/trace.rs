// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-290 `solver-inconclusive{trace}`: the `trace` member of a plugin's
//! cause read in QSpec FR-193's "Hybrid trace" form, its names resolved to
//! indices against the claim's `CheckedHybridModel`, and the correspondence
//! failures FR-250 lists refused `Malformed` at their member.

use serde_json::{Map, Value};

use super::{
    at_index, canonical_integer, child, count_of, first_undefined, invalid, malformed, member,
    read_plugin_inconclusive_cause, unknown_member, wrong_kind,
};
use crate::hybrid_replay::{admit, ExactInterval, HybridTrace, TraceBefore, TraceJump, TraceState};
use crate::proof_result::plugin::PluginResultRefusal;
use crate::proof_result::InconclusiveCause;
use qsl_semantics::check::{CheckedHybridModel, ModeIndex};
use quire_exact::Rational;

/// A plugin's `inconclusive` `cause` over an item whose claim is a hybrid
/// claim over `model`, or over any other item when `model` is `None`
/// (FR-290, "Basis and cause"). A `solver-inconclusive` `trace` reads as
/// `Some(HybridTrace)` on a hybrid claim and is `Malformed` at
/// `/cause/trace` on any other item; `insufficient-horizon` is `Malformed`
/// at `/cause/type` on any other item. The trace is kept, neither replayed
/// nor settled from.
///
/// # Errors
///
/// [`PluginResultRefusal`], as for [`read_plugin_inconclusive_cause`], with
/// the trace's own shape and correspondence refusals.
#[ix_trace_rs::string_edge]
pub fn read_plugin_inconclusive_cause_over(
    cause: &Value,
    model: Option<&CheckedHybridModel>,
) -> Result<InconclusiveCause, PluginResultRefusal> {
    const BASE: &str = "/cause";
    let kind = cause
        .as_object()
        .and_then(|members| members.get("type"))
        .and_then(Value::as_str);
    match (kind, model) {
        (Some("insufficient-horizon"), None) => Err(invalid(child(BASE, "type"))),
        (Some("solver-inconclusive"), _) => {
            let Some(members) = cause.as_object() else {
                return read_plugin_inconclusive_cause(cause);
            };
            let Some(trace) = members.get("trace") else {
                return read_plugin_inconclusive_cause(cause);
            };
            let mut rest = members.clone();
            rest.remove("trace");
            read_plugin_inconclusive_cause(&Value::Object(rest))?;
            let pointer = child(BASE, "trace");
            let Some(model) = model else {
                return Err(invalid(pointer));
            };
            Ok(InconclusiveCause::SolverInconclusive {
                trace: Some(Box::new(read_trace(model, trace, &pointer)?)),
                replay: None,
            })
        }
        _ => read_plugin_inconclusive_cause(cause),
    }
}

fn object<'a>(
    value: &'a Value,
    allowed: &[&str],
    pointer: &str,
) -> Result<&'a Map<String, Value>, PluginResultRefusal> {
    let Value::Object(members) = value else {
        return Err(wrong_kind(pointer.to_owned()));
    };
    if let Some(extra) = first_undefined(members, allowed) {
        return Err(unknown_member(child(pointer, extra)));
    }
    Ok(members)
}

/// An FR-331 exact `Rational`, `{numerator, denominator}` in lowest terms.
fn rational(value: &Value, pointer: &str) -> Result<Rational, PluginResultRefusal> {
    let members = object(value, &["denominator", "numerator"], pointer)?;
    let part = |name: &str| {
        members
            .get(name)
            .and_then(Value::as_str)
            .and_then(canonical_integer)
    };
    let (Some(numerator), Some(denominator)) = (part("numerator"), part("denominator")) else {
        return Err(invalid(pointer.to_owned()));
    };
    let reduced = Rational::new(numerator.clone(), denominator.clone())
        .map_err(|_| invalid(pointer.to_owned()))?;
    if reduced.numerator() != &numerator || reduced.denominator() != &denominator {
        return Err(invalid(pointer.to_owned()));
    }
    Ok(reduced)
}

/// An FR-331 `Interval`, `{lower, upper}`.
fn interval(value: &Value, pointer: &str) -> Result<ExactInterval, PluginResultRefusal> {
    let members = object(value, &["lower", "upper"], pointer)?;
    let end = |name: &str| rational(member(members, name, pointer)?, &child(pointer, name));
    Ok(ExactInterval::new(end("lower")?, end("upper")?))
}

/// The enclosure of `model`'s variables: one interval for each, keyed by
/// name.
fn enclosure(
    model: &CheckedHybridModel,
    value: &Value,
    pointer: &str,
) -> Result<Vec<ExactInterval>, PluginResultRefusal> {
    let Value::Object(members) = value else {
        return Err(wrong_kind(pointer.to_owned()));
    };
    let names: Vec<&str> = model
        .variables
        .iter()
        .map(|variable| variable.name.as_str())
        .collect();
    if let Some(extra) = first_undefined(members, &names) {
        return Err(invalid(child(pointer, extra)));
    }
    names
        .iter()
        .map(|name| match members.get(*name) {
            None => Err(invalid(pointer.to_owned())),
            Some(interval_value) => interval(interval_value, &child(pointer, name)),
        })
        .collect()
}

fn mode(
    model: &CheckedHybridModel,
    value: &Value,
    pointer: &str,
) -> Result<ModeIndex, PluginResultRefusal> {
    let Some(name) = value.as_str() else {
        return Err(malformed(false, pointer.to_owned()));
    };
    model
        .modes
        .iter()
        .position(|mode| mode.name.as_str() == name)
        .map(ModeIndex::new)
        .ok_or_else(|| invalid(pointer.to_owned()))
}

fn state(
    model: &CheckedHybridModel,
    value: &Value,
    pointer: &str,
) -> Result<TraceState, PluginResultRefusal> {
    let members = object(value, &["enclosure", "mode", "time"], pointer)?;
    let at = |name: &str| child(pointer, name);
    Ok(TraceState {
        mode: mode(model, member(members, "mode", pointer)?, &at("mode"))?,
        time: interval(member(members, "time", pointer)?, &at("time"))?,
        enclosure: enclosure(
            model,
            member(members, "enclosure", pointer)?,
            &at("enclosure"),
        )?,
    })
}

fn before(
    model: &CheckedHybridModel,
    value: &Value,
    pointer: &str,
) -> Result<TraceBefore, PluginResultRefusal> {
    let members = object(value, &["enclosure", "time"], pointer)?;
    let at = |name: &str| child(pointer, name);
    Ok(TraceBefore {
        time: interval(member(members, "time", pointer)?, &at("time"))?,
        enclosure: enclosure(
            model,
            member(members, "enclosure", pointer)?,
            &at("enclosure"),
        )?,
    })
}

fn jump(
    model: &CheckedHybridModel,
    value: &Value,
    pointer: &str,
) -> Result<TraceJump, PluginResultRefusal> {
    let members = object(value, &["before", "state", "transition"], pointer)?;
    let at = |name: &str| child(pointer, name);
    Ok(TraceJump {
        transition: count_of(
            member(members, "transition", pointer)?,
            false,
            at("transition"),
        )?,
        before: members
            .get("before")
            .map(|value| before(model, value, &at("before")))
            .transpose()?,
        state: state(model, member(members, "state", pointer)?, &at("state"))?,
    })
}

/// A `HybridTrace` in QSpec FR-193's wire form over `model`, after the
/// correspondence refusals of "Hybrid trace".
fn read_trace(
    model: &CheckedHybridModel,
    value: &Value,
    pointer: &str,
) -> Result<HybridTrace, PluginResultRefusal> {
    let members = object(value, &["delta", "jumps", "loop", "start"], pointer)?;
    let at = |name: &str| child(pointer, name);
    let Some(jumps) = member(members, "jumps", pointer)?.as_array() else {
        return Err(wrong_kind(at("jumps")));
    };
    let trace = HybridTrace {
        start: state(model, member(members, "start", pointer)?, &at("start"))?,
        jumps: jumps
            .iter()
            .enumerate()
            .map(|(index, value)| jump(model, value, &at_index(&at("jumps"), index)))
            .collect::<Result<_, _>>()?,
        delta: members
            .get("delta")
            .map(|value| rational(value, &at("delta")))
            .transpose()?,
        loop_to: members
            .get("loop")
            .map(|value| count_of(value, false, at("loop")))
            .transpose()?,
    };
    admit(model, &trace).map_err(|refusal| PluginResultRefusal::Malformed {
        pointer: format!("{pointer}{}", refusal.pointer),
        cause: crate::proof_result::plugin::ValueFault::InvalidValue,
    })?;
    Ok(trace)
}
