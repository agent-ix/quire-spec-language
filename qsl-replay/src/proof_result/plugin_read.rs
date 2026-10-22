// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-290 "Reading a plugin result member": the typed readers of a plugin's
//! FR-331 `basis`, `certification`, inconclusive `cause` and canonical
//! assignments. Each takes one wire member as a `serde_json::Value`, accepts
//! only the FR-331 spellings and refuses every value a plugin may not send
//! (ADR-029 PL-7) with an ordered, typed [`PluginResultRefusal`].
//!
//! The `basis` and `cause` readers share one ordered check (FR-290 steps 1 to
//! 8). What differs between them is data: the spellings of each member, the
//! payload members each spelling defines, and the spellings refused for a
//! plugin. Those are the [`BASIS`] and [`CAUSE`] tables below.

use std::cmp::Ordering;

use serde_json::{Map, Value};

use super::plugin::{read_plugin_cause_kind, PluginResultRefusal, ValueFault};
use super::{
    AppliedReduction, InconclusiveCause, InstanceBound, MemoryBound, MemoryBoundMember, ProofBasis,
    Proviso, ReductionKind,
};
use crate::hybrid::{SolverHorizon, SolverMethod};
use crate::witness::{CanonicalAssignment, WitnessValue};
use qsl_foundation::digest::WireNodeId;
use quire_exact::{Integer, Rational};
use quire_semantic_value::fairness::PropertyForm;

/// The largest FR-331 count, 2^53 - 1.
const MAX_COUNT: u64 = (1 << 53) - 1;

/// A spelling FR-331 lists for a member, with the payload members it
/// defines, in member-name order.
struct Spelling {
    name: &'static str,
    members: &'static [&'static str],
}

const fn spelling(name: &'static str, members: &'static [&'static str]) -> Spelling {
    Spelling { name, members }
}

/// FR-331's `basis` spellings a plugin may send.
const BASIS: &[Spelling] = &[
    spelling("exhaustive", &[]),
    spelling("bounded-complete", &["depth"]),
    spelling("inductive", &["depth"]),
    spelling("bounded-solver", &["horizon", "jumps", "method"]),
    spelling("reduced", &["reductions"]),
];

/// The `basis` spellings only the core establishes (FR-290 step 3).
const CORE_ONLY_BASIS: &[&str] = &[
    "bounded-proof",
    "closed-form",
    "witnessed",
    "exact-value",
    "value-bounds",
    "zone-certified",
];

/// FR-331's inconclusive `cause` spellings QSL's type holds for a plugin.
const CAUSE: &[Spelling] = &[
    spelling("undecided-successor", &[]),
    spelling("no-initial-state", &[]),
    spelling("no-instance", &[]),
    spelling("sufficient-test-failed", &[]),
    spelling("solver-inconclusive", &[]),
    spelling("match-undetermined", &[]),
    spelling("vacuous-match", &[]),
    spelling("bound-reached", &["depth"]),
    spelling("induction-not-closed", &["depth"]),
    spelling("reduction-not-preserving", &["form", "reduction"]),
    spelling("constraint-reached", &["boundary_states"]),
    spelling("reduction-horizon", &["max_depth"]),
    spelling("memory-bound-reached", &["limit", "states", "value"]),
    spelling("instance-bound-reached", &["limit", "states", "value"]),
    spelling("mapping-undetermined", &["prefix", "row", "state_key"]),
];

/// FR-331 causes that are valid on the wire but that QSL's type cannot hold
/// for a plugin (FR-290 step 5).
const UNSUPPORTED_CAUSE: &[&str] = &[
    "certificate-rejected",
    "well-definedness-unchecked",
    "symmetry-broken",
    "lasso-not-concretized",
    "no-admitted-behaviour",
    "transfer-not-preserving",
    "premise-unproved",
    "hypotheses-not-established",
];

/// A pointer to `name` under `base`, with `~` and `/` escaped (RFC 6901).
fn child(base: &str, name: &str) -> String {
    format!("{base}/{}", name.replace('~', "~0").replace('/', "~1"))
}

fn at_index(base: &str, index: usize) -> String {
    format!("{base}/{index}")
}

/// A value outside its domain: `wrong-value-kind` when `kind_ok` is false
/// (the JSON kind is not the kind the domain takes), else `invalid-value`.
fn malformed(kind_ok: bool, pointer: String) -> PluginResultRefusal {
    PluginResultRefusal::Malformed {
        pointer,
        cause: if kind_ok {
            ValueFault::InvalidValue
        } else {
            ValueFault::WrongValueKind
        },
    }
}

fn wrong_kind(pointer: String) -> PluginResultRefusal {
    malformed(false, pointer)
}

fn invalid(pointer: String) -> PluginResultRefusal {
    malformed(true, pointer)
}

fn missing(pointer: String) -> PluginResultRefusal {
    PluginResultRefusal::MissingMember { pointer }
}

fn unknown_member(pointer: String) -> PluginResultRefusal {
    PluginResultRefusal::UnknownMember { pointer }
}

/// RFC 8785 member-name order: by UTF-16 code unit.
fn name_order(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

/// The member names of `members` that `allowed` does not list, first in
/// member-name order.
fn first_undefined<'a>(members: &'a Map<String, Value>, allowed: &[&str]) -> Option<&'a str> {
    members
        .keys()
        .map(String::as_str)
        .filter(|name| !allowed.contains(name))
        .min_by(|left, right| name_order(left, right))
}

/// `name` of `members`, which the ordered checks before it found present.
fn member<'a>(
    members: &'a Map<String, Value>,
    name: &str,
    base: &str,
) -> Result<&'a Value, PluginResultRefusal> {
    members.get(name).ok_or_else(|| missing(child(base, name)))
}

/// An FR-331 count: a JSON integer from 0 to 2^53 - 1, from 1 when
/// `positive`.
fn count_of(value: &Value, positive: bool, pointer: String) -> Result<u64, PluginResultRefusal> {
    match value.as_u64() {
        Some(count) if count <= MAX_COUNT && (!positive || count >= 1) => Ok(count),
        _ => Err(malformed(value.is_u64() || value.is_i64(), pointer)),
    }
}

fn count(
    members: &Map<String, Value>,
    name: &str,
    base: &str,
    positive: bool,
) -> Result<u64, PluginResultRefusal> {
    count_of(member(members, name, base)?, positive, child(base, name))
}

/// A non-empty JSON string.
fn text_of(value: &Value, pointer: String) -> Result<&str, PluginResultRefusal> {
    match value.as_str() {
        Some(text) if !text.is_empty() => Ok(text),
        _ => Err(malformed(value.is_string(), pointer)),
    }
}

/// The ordered check shared by the `basis` and `cause` readers (FR-290 steps
/// 1 to 7): the object and its `type`, the spelling refusals, and the payload
/// members. It returns the payload and the spelling.
fn check_member<'a>(
    value: &'a Value,
    member_name: &str,
    refuse_spelling: impl Fn(&str) -> Result<(), PluginResultRefusal>,
    table: &'static [Spelling],
    unsupported: &[&str],
) -> Result<(&'a Map<String, Value>, &'static Spelling), PluginResultRefusal> {
    let base = child("", member_name);
    let Value::Object(members) = value else {
        return Err(wrong_kind(base));
    };
    let type_pointer = child(&base, "type");
    let kind = match members.get("type") {
        None => return Err(missing(type_pointer)),
        Some(Value::String(kind)) => kind.as_str(),
        Some(_) => return Err(wrong_kind(type_pointer)),
    };
    refuse_spelling(kind)?;
    let Some(entry) = table.iter().find(|entry| entry.name == kind) else {
        return Err(if unsupported.contains(&kind) {
            PluginResultRefusal::Unsupported {
                pointer: type_pointer,
                feature: kind.to_owned(),
            }
        } else {
            PluginResultRefusal::UnknownSpelling {
                pointer: type_pointer,
            }
        });
    };
    if let Some(absent) = entry
        .members
        .iter()
        .find(|name| !members.contains_key(**name))
    {
        return Err(missing(child(&base, absent)));
    }
    let defined: Vec<&str> = entry
        .members
        .iter()
        .copied()
        .chain(std::iter::once("type"))
        .collect();
    if let Some(extra) = first_undefined(members, &defined) {
        return Err(unknown_member(child(&base, extra)));
    }
    Ok((members, entry))
}

fn refuse_core_only_basis(kind: &str) -> Result<(), PluginResultRefusal> {
    if CORE_ONLY_BASIS.contains(&kind) {
        Err(PluginResultRefusal::CoreOnlyBasis)
    } else {
        Ok(())
    }
}

/// Read a plugin's `proved` entry's `basis` member into a [`ProofBasis`]
/// (FR-290, "Basis and cause"). The checks run in the order of the
/// requirement's steps 1 to 8, and the first refusal is returned.
///
/// # Errors
///
/// [`PluginResultRefusal`]: `Malformed` for a non-object or a value outside
/// its domain, `MissingMember` and `UnknownMember` for payload members,
/// `CoreOnlyBasis` for a basis only the core establishes, `UnknownSpelling`
/// for a `type` outside FR-331 and `Unsupported` for a valid value QSL's type
/// cannot hold.
#[ix_trace_rs::string_edge]
pub fn read_plugin_basis(basis: &Value) -> Result<ProofBasis, PluginResultRefusal> {
    const BASE: &str = "/basis";
    let (members, entry) = check_member(basis, "basis", refuse_core_only_basis, BASIS, &[])?;
    Ok(match entry.name {
        "exhaustive" => ProofBasis::Exhaustive,
        "bounded-complete" => ProofBasis::BoundedComplete {
            depth: count(members, "depth", BASE, false)?,
        },
        "inductive" => ProofBasis::Inductive {
            depth: count(members, "depth", BASE, false)?,
        },
        "bounded-solver" => {
            let horizon = read_horizon(member(members, "horizon", BASE)?, child(BASE, "horizon"))?;
            let jumps = count(members, "jumps", BASE, false)?;
            let method = read_method(member(members, "method", BASE)?, child(BASE, "method"))?;
            ProofBasis::BoundedSolver {
                method,
                horizon,
                jumps,
            }
        }
        _ => ProofBasis::Reduced {
            reductions: read_reductions(
                member(members, "reductions", BASE)?,
                child(BASE, "reductions"),
            )?,
        },
    })
}

#[ix_trace_rs::string_edge]
fn read_method(value: &Value, pointer: String) -> Result<SolverMethod, PluginResultRefusal> {
    let method = text_of(value, pointer.clone())?;
    [
        SolverMethod::DeltaComplete,
        SolverMethod::FlowpipeOutwardRounded,
        SolverMethod::FlowpipeUnrounded,
    ]
    .into_iter()
    .find(|candidate| candidate.as_str() == method)
    .ok_or_else(|| PluginResultRefusal::Unsupported {
        pointer,
        feature: method.to_owned(),
    })
}

/// A canonical decimal integer: an optional `-`, then digits with no leading
/// zero.
#[ix_trace_rs::string_edge]
fn canonical_integer(text: &str) -> Option<Integer> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let canonical = !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && text != "-0";
    if canonical {
        text.parse::<Integer>().ok()
    } else {
        None
    }
}

/// A checked-package v2 `Rational` `{numerator, denominator}` in lowest
/// terms, with a non-negative numerator and a positive denominator.
fn read_horizon(value: &Value, pointer: String) -> Result<SolverHorizon, PluginResultRefusal> {
    let refuse = || invalid(pointer.clone());
    let Value::Object(members) = value else {
        return Err(wrong_kind(pointer));
    };
    if members.len() != 2 {
        return Err(refuse());
    }
    let part = |name: &str| {
        members
            .get(name)
            .and_then(Value::as_str)
            .and_then(canonical_integer)
    };
    let (Some(numerator), Some(denominator)) = (part("numerator"), part("denominator")) else {
        return Err(refuse());
    };
    let reduced = Rational::new(numerator.clone(), denominator.clone()).map_err(|_| refuse())?;
    if reduced.numerator() != &numerator || reduced.denominator() != &denominator {
        return Err(refuse());
    }
    SolverHorizon::new(reduced).map_err(|_| refuse())
}

fn read_reductions(
    value: &Value,
    pointer: String,
) -> Result<Vec<AppliedReduction>, PluginResultRefusal> {
    match value.as_array() {
        Some(entries) if !entries.is_empty() => entries
            .iter()
            .enumerate()
            .map(|(index, entry)| read_reduction(entry, at_index(&pointer, index)))
            .collect(),
        _ => Err(malformed(value.is_array(), pointer)),
    }
}

/// One `{reduction, parameters, row}` entry of a `reduced` basis.
#[ix_trace_rs::string_edge]
fn read_reduction(value: &Value, pointer: String) -> Result<AppliedReduction, PluginResultRefusal> {
    const MEMBERS: [&str; 3] = ["parameters", "reduction", "row"];
    let Value::Object(members) = value else {
        return Err(wrong_kind(pointer));
    };
    if let Some(absent) = MEMBERS.iter().find(|name| !members.contains_key(**name)) {
        return Err(missing(child(&pointer, absent)));
    }
    if let Some(extra) = first_undefined(members, &MEMBERS) {
        return Err(unknown_member(child(&pointer, extra)));
    }
    let reduction_pointer = child(&pointer, "reduction");
    let parameters_pointer = child(&pointer, "parameters");
    let reduction = text_of(
        member(members, "reduction", &pointer)?,
        reduction_pointer.clone(),
    )?;
    let parameters = member(members, "parameters", &pointer)?;
    let applied = match reduction {
        "symmetry" => {
            return Err(PluginResultRefusal::Unsupported {
                pointer: reduction_pointer,
                feature: reduction.to_owned(),
            })
        }
        "partial-order" => read_proviso(parameters, parameters_pointer)?,
        "copy-swap" => read_classes(parameters, parameters_pointer)?,
        _ => return Err(invalid(reduction_pointer)),
    };
    text_of(member(members, "row", &pointer)?, child(&pointer, "row"))?;
    Ok(applied)
}

#[ix_trace_rs::string_edge]
fn read_proviso(
    parameters: &Value,
    pointer: String,
) -> Result<AppliedReduction, PluginResultRefusal> {
    let Value::Object(members) = parameters else {
        return Err(wrong_kind(pointer));
    };
    let proviso = member(members, "proviso", &pointer)?;
    if let Some(extra) = first_undefined(members, &["proviso"]) {
        return Err(unknown_member(child(&pointer, extra)));
    }
    match proviso.as_str() {
        Some("breadth-first-revisit") => Ok(AppliedReduction::PartialOrder {
            proviso: Proviso::BreadthFirstRevisit,
        }),
        other => Err(malformed(other.is_some(), child(&pointer, "proviso"))),
    }
}

fn read_classes(
    parameters: &Value,
    pointer: String,
) -> Result<AppliedReduction, PluginResultRefusal> {
    let Value::Object(members) = parameters else {
        return Err(wrong_kind(pointer));
    };
    let classes = member(members, "classes", &pointer)?;
    if let Some(extra) = first_undefined(members, &["classes"]) {
        return Err(unknown_member(child(&pointer, extra)));
    }
    let classes_pointer = child(&pointer, "classes");
    let Some(entries) = classes.as_array() else {
        return Err(wrong_kind(classes_pointer));
    };
    let classes = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| read_class(entry, at_index(&classes_pointer, index)))
        .collect::<Result<_, _>>()?;
    Ok(AppliedReduction::CopySwap { classes })
}

/// A class of two or more distinct non-empty strings.
fn read_class(value: &Value, pointer: String) -> Result<Vec<String>, PluginResultRefusal> {
    let Some(entries) = value.as_array() else {
        return Err(wrong_kind(pointer));
    };
    let class = entries
        .iter()
        .map(|entry| match entry.as_str() {
            Some(name) if !name.is_empty() => Ok(name),
            other => Err(malformed(other.is_some(), pointer.clone())),
        })
        .collect::<Result<Vec<&str>, _>>()?;
    let mut distinct = class.clone();
    distinct.sort_unstable();
    distinct.dedup();
    if class.len() >= 2 && distinct.len() == class.len() {
        Ok(class.into_iter().map(str::to_owned).collect())
    } else {
        Err(invalid(pointer))
    }
}

/// Read a plugin's `proved` entry's `certification` member (FR-290,
/// "Certification"). Only the string `trusted` is accepted, and it carries no
/// value: an accepted entry's record carries `trusted`
/// (`TerminalRecord::from_plugin_proof`). `None` is an absent member.
///
/// # Errors
///
/// `MissingMember` for an absent member, `Malformed` for a non-string and
/// `CertificationNotTrusted` for any other string.
#[ix_trace_rs::string_edge]
pub fn read_plugin_certification(certification: Option<&Value>) -> Result<(), PluginResultRefusal> {
    const POINTER: &str = "/certification";
    match certification {
        None => Err(missing(POINTER.to_owned())),
        Some(Value::String(label)) if label == "trusted" => Ok(()),
        Some(Value::String(label)) if matches!(label.as_str(), "certified" | "uncertified") => {
            Err(PluginResultRefusal::CertificationNotTrusted {
                received: label.clone(),
            })
        }
        Some(other) => Err(malformed(other.is_string(), POINTER.to_owned())),
    }
}

/// Read an `inconclusive` entry's `cause` member into an
/// [`InconclusiveCause`] (FR-290, "Basis and cause"). The checks run in the
/// order of the requirement's steps 1 to 8, and the first refusal is
/// returned.
///
/// # Errors
///
/// [`PluginResultRefusal`]: `Malformed`, `MissingMember` and `UnknownMember`
/// as for [`read_plugin_basis`], `QslOnlyCause` for a cause only QSL settles,
/// `UnknownSpelling` for a `type` outside FR-331 and `Unsupported` for a
/// valid FR-331 cause QSL's type cannot hold for a plugin.
#[ix_trace_rs::string_edge]
pub fn read_plugin_inconclusive_cause(
    cause: &Value,
) -> Result<InconclusiveCause, PluginResultRefusal> {
    const BASE: &str = "/cause";
    let (members, entry) = check_member(
        cause,
        "cause",
        read_plugin_cause_kind,
        CAUSE,
        UNSUPPORTED_CAUSE,
    )?;
    Ok(match entry.name {
        "undecided-successor" => InconclusiveCause::UndecidedSuccessor,
        "no-initial-state" => InconclusiveCause::NoInitialState,
        "no-instance" => InconclusiveCause::NoInstance,
        "sufficient-test-failed" => InconclusiveCause::SufficientTestFailed,
        "solver-inconclusive" => InconclusiveCause::SolverInconclusive,
        "match-undetermined" => InconclusiveCause::MatchUndetermined,
        "vacuous-match" => InconclusiveCause::VacuousMatch,
        "bound-reached" => InconclusiveCause::BoundReached {
            depth: count(members, "depth", BASE, false)?,
        },
        "induction-not-closed" => InconclusiveCause::InductionNotClosed {
            depth: count(members, "depth", BASE, false)?,
        },
        "reduction-not-preserving" => {
            let form = read_form(member(members, "form", BASE)?, child(BASE, "form"))?;
            let reduction = read_reduction_kind(
                member(members, "reduction", BASE)?,
                child(BASE, "reduction"),
            )?;
            InconclusiveCause::ReductionNotPreserving { reduction, form }
        }
        "constraint-reached" => InconclusiveCause::ConstraintReached {
            boundary_states: count(members, "boundary_states", BASE, true)?,
        },
        "reduction-horizon" => InconclusiveCause::ReductionHorizon {
            max_depth: count(members, "max_depth", BASE, false)?,
        },
        "memory-bound-reached" => {
            let member_name = read_memory_limit(members, BASE)?;
            let states = count(members, "states", BASE, true)?;
            let value = count(members, "value", BASE, true)?;
            InconclusiveCause::MemoryBoundReached {
                bound: MemoryBound {
                    member: member_name,
                    value,
                },
                states,
            }
        }
        "instance-bound-reached" => {
            read_instance_limit(members, BASE)?;
            let states = count(members, "states", BASE, true)?;
            let value = count(members, "value", BASE, true)?;
            InconclusiveCause::InstanceBoundReached {
                bound: InstanceBound { value },
                states,
            }
        }
        _ => {
            check_prefix(member(members, "prefix", BASE)?, child(BASE, "prefix"))?;
            text_of(member(members, "row", BASE)?, child(BASE, "row"))?;
            check_state_key(
                member(members, "state_key", BASE)?,
                child(BASE, "state_key"),
            )?;
            InconclusiveCause::MappingUndetermined
        }
    })
}

#[ix_trace_rs::string_edge]
fn read_form(value: &Value, pointer: String) -> Result<PropertyForm, PluginResultRefusal> {
    let form = text_of(value, pointer.clone())?;
    match form {
        "reachable-invariant" => Ok(PropertyForm::ReachableInvariant),
        "bounded-mltl" => Ok(PropertyForm::BoundedMltl),
        "safety" => Ok(PropertyForm::Safety),
        "liveness" => Ok(PropertyForm::Liveness),
        other => Err(PluginResultRefusal::Unsupported {
            pointer,
            feature: other.to_owned(),
        }),
    }
}

#[ix_trace_rs::string_edge]
fn read_reduction_kind(
    value: &Value,
    pointer: String,
) -> Result<ReductionKind, PluginResultRefusal> {
    match value.as_str() {
        Some("symmetry") => Ok(ReductionKind::Symmetry),
        Some("partial-order") => Ok(ReductionKind::PartialOrder),
        Some("constraint") => Ok(ReductionKind::StateConstraint),
        other => Err(malformed(other.is_some(), pointer)),
    }
}

#[ix_trace_rs::string_edge]
fn read_memory_limit(
    members: &Map<String, Value>,
    base: &str,
) -> Result<MemoryBoundMember, PluginResultRefusal> {
    match member(members, "limit", base)?.as_str() {
        Some("max_store_buffer") => Ok(MemoryBoundMember::MaxStoreBuffer),
        Some("max_messages") => Ok(MemoryBoundMember::MaxMessages),
        other => Err(malformed(other.is_some(), child(base, "limit"))),
    }
}

#[ix_trace_rs::string_edge]
fn read_instance_limit(
    members: &Map<String, Value>,
    base: &str,
) -> Result<(), PluginResultRefusal> {
    match member(members, "limit", base)?.as_str() {
        Some("max_live_instances") => Ok(()),
        other => Err(malformed(other.is_some(), child(base, "limit"))),
    }
}

/// An FR-331 `StateKey`: 64 lowercase hex digits.
fn check_state_key(value: &Value, pointer: String) -> Result<(), PluginResultRefusal> {
    match value.as_str().and_then(WireNodeId::from_hex) {
        Some(_) => Ok(()),
        None => Err(malformed(value.is_string(), pointer)),
    }
}

/// The `prefix` of a `mapping-undetermined` cause: FR-331 `ModelStep`s
/// (FR-364): `transition` and `post_state`, plus `delay` when present.
fn check_prefix(value: &Value, pointer: String) -> Result<(), PluginResultRefusal> {
    let Some(steps) = value.as_array() else {
        return Err(wrong_kind(pointer));
    };
    steps
        .iter()
        .enumerate()
        .try_for_each(|(index, step)| check_model_step(step, at_index(&pointer, index)))
}

#[ix_trace_rs::string_edge]
fn check_model_step(step: &Value, pointer: String) -> Result<(), PluginResultRefusal> {
    let Value::Object(members) = step else {
        return Err(wrong_kind(pointer));
    };
    for required in ["post_state", "transition"] {
        if !members.contains_key(required) {
            return Err(missing(child(&pointer, required)));
        }
    }
    if let Some(extra) = first_undefined(members, &["delay", "post_state", "transition"]) {
        return Err(unknown_member(child(&pointer, extra)));
    }
    if let Some(delay) = members.get("delay") {
        check_delay(delay, child(&pointer, "delay"))?;
    }
    check_state_key(
        member(members, "post_state", &pointer)?,
        child(&pointer, "post_state"),
    )?;
    let transition_pointer = child(&pointer, "transition");
    let transition = member(members, "transition", &pointer)?;
    let well_formed = transition.as_object().is_some_and(|fields| {
        match fields.get("type").and_then(Value::as_str) {
            Some("terminal-stutter") => fields.len() == 1,
            Some("transition") => {
                fields.len() == 4
                    && fields.get("operation").is_some_and(Value::is_string)
                    && fields.contains_key("receiver")
                    && fields.get("arguments").is_some_and(Value::is_array)
            }
            _ => false,
        }
    });
    if well_formed {
        Ok(())
    } else {
        Err(malformed(transition.is_object(), transition_pointer))
    }
}

/// A step's `delay`: an exact rational `{numerator, denominator}`.
fn check_delay(value: &Value, pointer: String) -> Result<(), PluginResultRefusal> {
    let Some(members) = value.as_object() else {
        return Err(wrong_kind(pointer));
    };
    let part = |name: &str| {
        members
            .get(name)
            .and_then(Value::as_str)
            .and_then(canonical_integer)
    };
    match (members.len(), part("numerator"), part("denominator")) {
        (2, Some(_), Some(denominator)) if !denominator.is_zero() && !denominator.is_negative() => {
            Ok(())
        }
        _ => Err(invalid(pointer)),
    }
}

/// Read an item's `counterexamples` entry into the canonical assignments
/// `ReplaySource::Input` replays (FR-290, "Assignments"). The entry's
/// `request_index` and `backend` are not checked: the driver joins the entry
/// to its item by `request_index`.
///
/// # Errors
///
/// [`PluginResultRefusal`]: `Malformed` at `""` for a non-object entry,
/// `MissingCounterexample` at `/counterexample` or `/counterexample/type`
/// for an entry with no assignments to replay, `UnknownMember`,
/// `MissingMember` and `Malformed` for the members of the counterexample and
/// its values, in the order QSpec FR-331 reads them.
#[ix_trace_rs::string_edge]
pub fn read_plugin_assignments(
    entry: &Value,
) -> Result<Vec<CanonicalAssignment>, PluginResultRefusal> {
    const BASE: &str = "/counterexample";
    let Value::Object(members) = entry else {
        return Err(wrong_kind(String::new()));
    };
    let Some(counterexample) = members.get("counterexample") else {
        return Err(PluginResultRefusal::MissingCounterexample {
            pointer: BASE.to_owned(),
            cause: ValueFault::MissingMember,
        });
    };
    if let Some(extra) = first_undefined(members, &["backend", "counterexample", "request_index"]) {
        return Err(unknown_member(child("", extra)));
    }
    let not_assignments = |cause| PluginResultRefusal::MissingCounterexample {
        pointer: child(BASE, "type"),
        cause,
    };
    let Value::Object(counterexample) = counterexample else {
        return Err(PluginResultRefusal::MissingCounterexample {
            pointer: BASE.to_owned(),
            cause: ValueFault::WrongValueKind,
        });
    };
    match counterexample.get("type") {
        None => return Err(not_assignments(ValueFault::MissingMember)),
        Some(Value::String(kind)) if kind == "assignments" => {}
        Some(Value::String(_)) => return Err(not_assignments(ValueFault::InvalidValue)),
        Some(_) => return Err(not_assignments(ValueFault::WrongValueKind)),
    }
    let values_pointer = child(BASE, "values");
    let Some(values) = counterexample.get("values") else {
        return Err(missing(values_pointer));
    };
    if let Some(extra) = first_undefined(counterexample, &["type", "values"]) {
        return Err(unknown_member(child(BASE, extra)));
    }
    let Value::Array(values) = values else {
        return Err(wrong_kind(values_pointer));
    };
    let mut assignments: Vec<CanonicalAssignment> = Vec::with_capacity(values.len());
    for (index, element) in values.iter().enumerate() {
        let pointer = at_index(&values_pointer, index);
        let assignment = read_assignment(element, &pointer)?;
        if assignments
            .iter()
            .any(|earlier| earlier.parameter == assignment.parameter)
        {
            return Err(invalid(child(&pointer, "parameter")));
        }
        assignments.push(assignment);
    }
    Ok(assignments)
}

fn read_assignment(
    element: &Value,
    pointer: &str,
) -> Result<CanonicalAssignment, PluginResultRefusal> {
    let Value::Object(members) = element else {
        return Err(wrong_kind(pointer.to_owned()));
    };
    for required in ["parameter", "value"] {
        if !members.contains_key(required) {
            return Err(missing(child(pointer, required)));
        }
    }
    if let Some(extra) = first_undefined(members, &["parameter", "value"]) {
        return Err(unknown_member(child(pointer, extra)));
    }
    let parameter_pointer = child(pointer, "parameter");
    let text = member(members, "parameter", pointer)?;
    let parameter = text
        .as_str()
        .and_then(WireNodeId::from_hex)
        .ok_or_else(|| malformed(text.is_string(), parameter_pointer))?;
    let value = WitnessValue::from_wire_json(member(members, "value", pointer)?)
        .map_err(|_| invalid(child(pointer, "value")))?;
    Ok(CanonicalAssignment { parameter, value })
}
