// SPDX-License-Identifier: AGPL-3.0-or-later
//! The JSON codec of FSpec FR-418's `task_product` member and of
//! `TaskTransitionTemplate` (the `TaskProductCertificate` and
//! `TaskTransitionTemplate` definitions of the FR-331 schema, FR-418-AC-11).
//!
//! The reader is strict: it admits exactly the schema's shape and refuses
//! everything else with a [`TaskProductShapeRefusal`], which maps onto the
//! certificate's `malformed` rule at the `task_product` member. It checks
//! wire shape only; recomputing the product is the checker's work.

use filament_exact::{Integer, Rational};
use filament_semantic_value::model_transition::ModelTransition;
use quire_canonical::Limits;
use serde_json::{json, Map, Value};

use super::{
    TaskArrivalKind, TaskClockCeiling, TaskClosureEvidence, TaskClosureMethod,
    TaskDiagonalConstraint, TaskDiagonalOperator, TaskProductCertificate, TaskQueueBound,
    TaskSubtraction, TaskTimeProgress, TaskTransitionTemplate,
};
use crate::certificate::closure::CertificateRejection;
use crate::certificate::{CertificateLocus, CertificateRule};
use crate::TaskTransitionAction;

/// The largest index or count the wire carries (RFC 8785 exact integers).
const MAX_COUNT: u64 = 9_007_199_254_740_991;

/// The byte bound of one transition-identity text.
const IDENTITY_LIMITS: Limits = Limits::new(1 << 20);

/// Why a `task_product` value is not a `TaskProductCertificate`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskProductShapeReason {
    /// A required member is absent.
    Missing,
    /// A member the definition does not name.
    UnknownMember,
    /// The value is of another JSON kind.
    WrongKind,
    /// An integer outside its range, or a string that is not the canonical
    /// spelling of its type, or a rational not in lowest terms.
    NotCanonical,
    /// An enumerated spelling the definition does not list.
    UnknownVariant,
    /// Members that are individually valid but contradict each other, such
    /// as a closure label that does not belong to the method, a cap the
    /// method forbids, or an empty `queue_bounds`.
    Inconsistent,
}

/// A `task_product` value that is not a `TaskProductCertificate`, with the
/// JSON pointer (relative to the member) of the first offence.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("task_product{pointer}: {reason:?}")]
pub struct TaskProductShapeRefusal {
    /// The JSON pointer of the offence, empty for the member itself.
    pub pointer: String,
    /// What is wrong there.
    pub reason: TaskProductShapeReason,
}

impl TaskProductShapeRefusal {
    /// The certificate rejection FSpec FR-418 gives it: `malformed` at the
    /// certificate member `{member: task_product}`.
    pub fn rejection(&self) -> CertificateRejection {
        CertificateRejection {
            rule: CertificateRule::Malformed,
            at: CertificateLocus::CertificateMember(
                filament_contract_model::CertificateMember::TaskProduct,
            ),
        }
    }
}

/// A value that has no wire spelling.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("the task-product value has no wire spelling")]
pub struct TaskProductWireError;

pub(crate) fn refuse<T>(pointer: &str, reason: TaskProductShapeReason) -> Result<T, TaskProductShapeRefusal> {
    Err(TaskProductShapeRefusal {
        pointer: pointer.to_owned(),
        reason,
    })
}

pub(crate) fn child(pointer: &str, name: impl std::fmt::Display) -> String {
    format!("{pointer}/{name}")
}

/// An object whose members are exactly `required` plus any of `optional`.
pub(crate) fn object<'v>(
    value: &'v Value,
    pointer: &str,
    required: &[&str],
    optional: &[&str],
) -> Result<&'v Map<String, Value>, TaskProductShapeRefusal> {
    let Some(map) = value.as_object() else {
        return refuse(pointer, TaskProductShapeReason::WrongKind);
    };
    for name in map.keys() {
        if !required.contains(&name.as_str()) && !optional.contains(&name.as_str()) {
            return refuse(&child(pointer, name), TaskProductShapeReason::UnknownMember);
        }
    }
    for name in required {
        if !map.contains_key(*name) {
            return refuse(&child(pointer, name), TaskProductShapeReason::Missing);
        }
    }
    Ok(map)
}

pub(crate) fn member<'v>(map: &'v Map<String, Value>, name: &str) -> &'v Value {
    // Callers read only members `object` has proved present.
    map.get(name).unwrap_or(&Value::Null)
}

pub(crate) fn array<'v>(value: &'v Value, pointer: &str) -> Result<&'v [Value], TaskProductShapeRefusal> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or(TaskProductShapeRefusal {
            pointer: pointer.to_owned(),
            reason: TaskProductShapeReason::WrongKind,
        })
}

fn text(value: &Value, pointer: &str) -> Result<String, TaskProductShapeRefusal> {
    match value.as_str() {
        Some("") => refuse(pointer, TaskProductShapeReason::NotCanonical),
        Some(text) => Ok(text.to_owned()),
        None => refuse(pointer, TaskProductShapeReason::WrongKind),
    }
}

pub(crate) fn count(value: &Value, pointer: &str) -> Result<u64, TaskProductShapeRefusal> {
    match value.as_u64() {
        Some(count) if count <= MAX_COUNT => Ok(count),
        Some(_) => refuse(pointer, TaskProductShapeReason::NotCanonical),
        None => refuse(
            pointer,
            if value.is_number() {
                TaskProductShapeReason::NotCanonical
            } else {
                TaskProductShapeReason::WrongKind
            },
        ),
    }
}

fn positive_count(value: &Value, pointer: &str) -> Result<u64, TaskProductShapeRefusal> {
    match count(value, pointer)? {
        0 => refuse(pointer, TaskProductShapeReason::NotCanonical),
        count => Ok(count),
    }
}

pub(crate) fn integer(value: &Value, pointer: &str) -> Result<Integer, TaskProductShapeRefusal> {
    let Some(spelling) = value.as_str() else {
        return refuse(pointer, TaskProductShapeReason::WrongKind);
    };
    spelling
        .parse()
        .or_else(|_| refuse(pointer, TaskProductShapeReason::NotCanonical))
}

fn positive_integer(value: &Value, pointer: &str) -> Result<Integer, TaskProductShapeRefusal> {
    let value = integer(value, pointer)?;
    if value.is_zero() || value.is_negative() {
        return refuse(pointer, TaskProductShapeReason::NotCanonical);
    }
    Ok(value)
}

/// The deepest `TypedValue` nesting the reader admits.
const MAX_TYPED_DEPTH: usize = 64;

/// Checks `value` against the schema's `TypedValue` definition (FSpec
/// FR-181): a closed object per `type`.
fn typed_value(value: &Value, pointer: &str, depth: usize) -> Result<(), TaskProductShapeRefusal> {
    if depth > MAX_TYPED_DEPTH {
        return refuse(pointer, TaskProductShapeReason::NotCanonical);
    }
    let Some(kind) = value.get("type").and_then(Value::as_str) else {
        return refuse(pointer, TaskProductShapeReason::WrongKind);
    };
    let at = |name: &str| child(pointer, name);
    let hex = |name: &str, width: usize, even: bool| -> Result<(), TaskProductShapeRefusal> {
        let spelling = member_str(value, name).unwrap_or("");
        let digits = spelling
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        let sized = if even {
            spelling.len().is_multiple_of(2)
        } else {
            spelling.len() == width
        };
        if digits && sized {
            Ok(())
        } else {
            refuse(&at(name), TaskProductShapeReason::NotCanonical)
        }
    };
    let nested = |value: &Value, name: String| typed_value(value, &name, depth + 1);
    let list = |map: &Map<String, Value>, name: &str| -> Result<(), TaskProductShapeRefusal> {
        for (index, element) in array(member(map, name), &at(name))?.iter().enumerate() {
            nested(element, child(&at(name), index))?;
        }
        Ok(())
    };
    match kind {
        "null" => object(value, pointer, &["type"], &[]).map(drop),
        "integer" => {
            let map = object(value, pointer, &["type", "value"], &[])?;
            integer(member(map, "value"), &at("value")).map(drop)
        }
        "boolean" => {
            let map = object(value, pointer, &["type", "value"], &[])?;
            match member(map, "value") {
                Value::Bool(_) => Ok(()),
                _ => refuse(&at("value"), TaskProductShapeReason::WrongKind),
            }
        }
        "float32" => {
            object(value, pointer, &["type", "bits"], &[])?;
            match member_str(value, "bits") {
                Some(_) => hex("bits", 8, false),
                None => refuse(&at("bits"), TaskProductShapeReason::WrongKind),
            }
        }
        "float64" => {
            object(value, pointer, &["type", "bits"], &[])?;
            match member_str(value, "bits") {
                Some(_) => hex("bits", 16, false),
                None => refuse(&at("bits"), TaskProductShapeReason::WrongKind),
            }
        }
        "rational" => {
            let map = object(
                value,
                pointer,
                &["type", "numerator", "denominator"],
                &["unit"],
            )?;
            integer(member(map, "numerator"), &at("numerator"))?;
            positive_integer(member(map, "denominator"), &at("denominator"))?;
            map.get("unit")
                .map_or(Ok(()), |unit| text(unit, &at("unit")).map(drop))
        }
        "decimal" => {
            let map = object(value, pointer, &["type", "coefficient", "scale"], &["unit"])?;
            integer(member(map, "coefficient"), &at("coefficient"))?;
            integer(member(map, "scale"), &at("scale"))?;
            map.get("unit")
                .map_or(Ok(()), |unit| text(unit, &at("unit")).map(drop))
        }
        "text" => {
            let map = object(value, pointer, &["type", "value"], &[])?;
            match member(map, "value") {
                Value::String(_) => Ok(()),
                _ => refuse(&at("value"), TaskProductShapeReason::WrongKind),
            }
        }
        "option" => {
            let map = object(value, pointer, &["type", "value"], &[])?;
            match member(map, "value") {
                Value::Null => Ok(()),
                inner => nested(inner, at("value")),
            }
        }
        "set" | "bag" | "sequence" | "ordered-set" => {
            let map = object(value, pointer, &["type", "elements"], &[])?;
            list(map, "elements")
        }
        "record" => {
            let map = object(value, pointer, &["type", "name", "fields"], &[])?;
            text(member(map, "name"), &at("name"))?;
            for (index, field) in array(member(map, "fields"), &at("fields"))?
                .iter()
                .enumerate()
            {
                let base = child(&at("fields"), index);
                let row = object(field, &base, &["name", "value"], &[])?;
                text(member(row, "name"), &child(&base, "name"))?;
                match member(row, "value") {
                    Value::Null => {}
                    inner => nested(inner, child(&base, "value"))?,
                }
            }
            Ok(())
        }
        "tuple" => {
            let map = object(value, pointer, &["type", "name", "components"], &[])?;
            text(member(map, "name"), &at("name"))?;
            list(map, "components")
        }
        "enum" => {
            let map = object(value, pointer, &["type", "name", "member"], &[])?;
            text(member(map, "name"), &at("name"))?;
            text(member(map, "member"), &at("member")).map(drop)
        }
        "union" => {
            let map = object(
                value,
                pointer,
                &["type", "name", "member", "components"],
                &[],
            )?;
            text(member(map, "name"), &at("name"))?;
            text(member(map, "member"), &at("member"))?;
            list(map, "components")
        }
        "reference" => {
            let map = object(
                value,
                pointer,
                &["type", "universe", "object_type", "identity"],
                &[],
            )?;
            node_id(member(map, "universe"), &at("universe"))?;
            effective_id(member(map, "object_type"), &at("object_type"))?;
            match member_str(value, "identity") {
                Some(_) => text(member(map, "identity"), &at("identity")).map(drop),
                None => refuse(&at("identity"), TaskProductShapeReason::WrongKind),
            }
        }
        _ => refuse(&at("type"), TaskProductShapeReason::UnknownVariant),
    }
}

fn member_str<'v>(value: &'v Value, name: &str) -> Option<&'v str> {
    value.get(name).and_then(Value::as_str)
}

/// An effective type id: `{owner_effective_type: null | <effective id>,
/// original: {package, node}}`, as `effective_form` writes it.
fn effective_id(value: &Value, pointer: &str) -> Result<(), TaskProductShapeRefusal> {
    let mut current = value;
    let mut at = pointer.to_owned();
    while !current.is_null() {
        let map = object(current, &at, &["owner_effective_type", "original"], &[])?;
        let original = member(map, "original");
        let original_at = child(&at, "original");
        let inner = object(original, &original_at, &["package", "node"], &[])?;
        text(member(inner, "package"), &child(&original_at, "package"))?;
        text(member(inner, "node"), &child(&original_at, "node"))?;
        current = member(map, "owner_effective_type");
        at = child(&at, "owner_effective_type");
    }
    Ok(())
}

/// A checked-package `NodeId`: `{ordinal, package, path}`.
fn node_id(value: &Value, pointer: &str) -> Result<(), TaskProductShapeRefusal> {
    let map = object(value, pointer, &["ordinal", "package", "path"], &[])?;
    count(member(map, "ordinal"), &child(pointer, "ordinal"))?;
    text(member(map, "package"), &child(pointer, "package"))?;
    array(member(map, "path"), &child(pointer, "path")).map(drop)
}

/// Whether two source transitions have one wire spelling. The FR-331 wire
/// carries an operation as one string (a declaration's `name` or `node`
/// identity, or the operation text), so a transition read from a certificate
/// equals the checker's own transition exactly when their wire spellings
/// agree; structural `==` on a read transition is exact only when both sides
/// were built from a string-valued operation.
pub fn same_source_transition(left: &ModelTransition, right: &ModelTransition) -> bool {
    match (
        crate::task_source_transition_wire(left),
        crate::task_source_transition_wire(right),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn rational(value: &Value, pointer: &str) -> Result<Rational, TaskProductShapeRefusal> {
    let map = object(value, pointer, &["numerator", "denominator"], &[])?;
    let numerator = integer(member(map, "numerator"), &child(pointer, "numerator"))?;
    let denominator = integer(member(map, "denominator"), &child(pointer, "denominator"))?;
    let invalid = || TaskProductShapeRefusal {
        pointer: pointer.to_owned(),
        reason: TaskProductShapeReason::NotCanonical,
    };
    if denominator.is_zero() || denominator.is_negative() {
        return Err(invalid());
    }
    let reduced = Rational::new(numerator.clone(), denominator.clone()).map_err(|_| invalid())?;
    if reduced.numerator() != &numerator || reduced.denominator() != &denominator {
        return Err(invalid());
    }
    Ok(reduced)
}

fn rational_wire(value: &Rational) -> Value {
    json!({
        "denominator": value.denominator().to_string(),
        "numerator": value.numerator().to_string(),
    })
}

fn enumerated<T: Copy>(
    value: &Value,
    pointer: &str,
    table: &[(&str, T)],
) -> Result<T, TaskProductShapeRefusal> {
    let Some(spelling) = value.as_str() else {
        return refuse(pointer, TaskProductShapeReason::WrongKind);
    };
    table
        .iter()
        .find(|(name, _)| *name == spelling)
        .map(|(_, variant)| *variant)
        .ok_or(TaskProductShapeRefusal {
            pointer: pointer.to_owned(),
            reason: TaskProductShapeReason::UnknownVariant,
        })
}

const ACTIONS: [(&str, TaskTransitionAction); 8] = [
    ("activate", TaskTransitionAction::Activate),
    ("release", TaskTransitionAction::Release),
    ("dispatch", TaskTransitionAction::Dispatch),
    ("preempt", TaskTransitionAction::Preempt),
    ("resume", TaskTransitionAction::Resume),
    ("complete", TaskTransitionAction::Complete),
    ("budget-change", TaskTransitionAction::BudgetChange),
    ("deadline-miss", TaskTransitionAction::DeadlineMiss),
];

fn action_name(action: TaskTransitionAction) -> &'static str {
    ACTIONS
        .iter()
        .find(|(_, candidate)| *candidate == action)
        .map_or("", |(name, _)| name)
}

/// A source transition identity `{type, operation, receiver, arguments}`.
pub(crate) fn source_transition(
    value: &Value,
    pointer: &str,
) -> Result<ModelTransition, TaskProductShapeRefusal> {
    let map = object(
        value,
        pointer,
        &["type", "operation", "receiver", "arguments"],
        &[],
    )?;
    if member(map, "type").as_str() != Some("transition") {
        return refuse(
            &child(pointer, "type"),
            TaskProductShapeReason::UnknownVariant,
        );
    }
    let operation = text(member(map, "operation"), &child(pointer, "operation"))?;
    let typed = |value: &Value, at: String| -> Result<String, TaskProductShapeRefusal> {
        typed_value(value, &at, 0)?;
        serde_json::to_string(value).or_else(|_| refuse(&at, TaskProductShapeReason::NotCanonical))
    };
    let receiver = match member(map, "receiver") {
        Value::Null => None,
        value => Some(typed(value, child(pointer, "receiver"))?),
    };
    let arguments_at = child(pointer, "arguments");
    let arguments = array(member(map, "arguments"), &arguments_at)?
        .iter()
        .enumerate()
        .map(|(index, value)| typed(value, child(&arguments_at, index)))
        .collect::<Result<Vec<_>, _>>()?;
    let operation = serde_json::to_string(&Value::String(operation))
        .or_else(|_| refuse(pointer, TaskProductShapeReason::NotCanonical))?;
    ModelTransition::new(operation, receiver, arguments, IDENTITY_LIMITS)
        .or_else(|_| refuse(pointer, TaskProductShapeReason::NotCanonical))
}

pub(crate) fn source_transition_wire(transition: &ModelTransition) -> Result<Value, TaskProductWireError> {
    crate::task_source_transition_wire(transition).map_err(|_| TaskProductWireError)
}

impl TaskTransitionTemplate {
    /// Reads a `TaskTransitionTemplate` (FR-418-AC-11): the closed shape
    /// with `type` `task-transition-template`, and no occurrence,
    /// generation, demand, jitter or deadline.
    ///
    /// # Errors
    ///
    /// [`TaskProductShapeRefusal`] naming the first offence.
    pub fn from_wire(value: &Value) -> Result<Self, TaskProductShapeRefusal> {
        Self::read(value, "")
    }

    pub(crate) fn read(value: &Value, pointer: &str) -> Result<Self, TaskProductShapeRefusal> {
        let map = object(
            value,
            pointer,
            &[
                "type",
                "action",
                "taskset",
                "task",
                "job_rank",
                "slot",
                "release",
                "source_transition",
            ],
            &["priority"],
        )?;
        if member(map, "type").as_str() != Some("task-transition-template") {
            return refuse(
                &child(pointer, "type"),
                TaskProductShapeReason::UnknownVariant,
            );
        }
        let source = match member(map, "source_transition") {
            Value::Null => None,
            value => Some(source_transition(
                value,
                &child(pointer, "source_transition"),
            )?),
        };
        let priority = map
            .get("priority")
            .map(|value| count(value, &child(pointer, "priority")))
            .transpose()?;
        Ok(Self {
            action: enumerated(member(map, "action"), &child(pointer, "action"), &ACTIONS)?,
            task_set: text(member(map, "taskset"), &child(pointer, "taskset"))?,
            task: text(member(map, "task"), &child(pointer, "task"))?,
            job_rank: count(member(map, "job_rank"), &child(pointer, "job_rank"))?,
            slot: count(member(map, "slot"), &child(pointer, "slot"))?,
            release: source_transition(member(map, "release"), &child(pointer, "release"))?,
            source_transition: source,
            priority,
        })
    }

    /// The template's wire value, members in ascending order.
    ///
    /// # Errors
    ///
    /// [`TaskProductWireError`] when a source transition has no spelling.
    pub fn to_wire(&self) -> Result<Value, TaskProductWireError> {
        let mut map = Map::new();
        map.insert("action".to_owned(), json!(action_name(self.action)));
        map.insert("job_rank".to_owned(), json!(self.job_rank));
        if let Some(priority) = self.priority {
            map.insert("priority".to_owned(), json!(priority));
        }
        map.insert("release".to_owned(), source_transition_wire(&self.release)?);
        map.insert("slot".to_owned(), json!(self.slot));
        map.insert(
            "source_transition".to_owned(),
            match &self.source_transition {
                Some(transition) => source_transition_wire(transition)?,
                None => Value::Null,
            },
        );
        map.insert("task".to_owned(), json!(self.task));
        map.insert("taskset".to_owned(), json!(self.task_set));
        map.insert("type".to_owned(), json!("task-transition-template"));
        Ok(Value::Object(map))
    }

    /// Whether `other` names the same finite template, comparing source
    /// transitions by their wire spelling ([`same_source_transition`]).
    pub fn same_identity(&self, other: &Self) -> bool {
        self.action == other.action
            && self.task_set == other.task_set
            && self.task == other.task
            && self.job_rank == other.job_rank
            && self.slot == other.slot
            && self.priority == other.priority
            && same_source_transition(&self.release, &other.release)
            && match (&self.source_transition, &other.source_transition) {
                (None, None) => true,
                (Some(left), Some(right)) => same_source_transition(left, right),
                _ => false,
            }
    }
}

const METHODS: [(&str, TaskClosureMethod); 2] = [
    (
        "closed-finite-product/v1",
        TaskClosureMethod::ClosedFiniteProduct,
    ),
    (
        "closed-no-release-product/v1",
        TaskClosureMethod::ClosedNoReleaseProduct,
    ),
];

const CLOSURES: [(&str, TaskClosureEvidence); 2] = [
    (
        "recompute-all-reachable-successors/v1",
        TaskClosureEvidence::RecomputeAllReachableSuccessors,
    ),
    (
        "recompute-all-source-successors-no-task-release/v1",
        TaskClosureEvidence::RecomputeAllSourceSuccessorsNoTaskRelease,
    ),
];

const ARRIVALS: [(&str, TaskArrivalKind); 2] = [
    ("periodic", TaskArrivalKind::Periodic),
    ("sporadic", TaskArrivalKind::Sporadic),
];

const OPERATORS: [(&str, TaskDiagonalOperator); 4] = [
    ("<", TaskDiagonalOperator::Less),
    ("<=", TaskDiagonalOperator::LessOrEqual),
    (">", TaskDiagonalOperator::Greater),
    (">=", TaskDiagonalOperator::GreaterOrEqual),
];

impl TaskProductCertificate {
    /// Reads a `task_product` member (FR-418-AC-11, AC-14): the
    /// `bounded-subtraction-diagonal/v1` abstraction, the structured
    /// `time_progress`, and the four arrays, each row closed. It refuses an
    /// unknown method, a closure label that is not the method's, a zero cap
    /// under `closed-finite-product/v1`, a nonzero cap, clock ceilings,
    /// diagonals or subtractions under `closed-no-release-product/v1`, an
    /// empty `queue_bounds`, and a template or subtraction that carries an
    /// exact task identity.
    ///
    /// # Errors
    ///
    /// [`TaskProductShapeRefusal`] naming the first offence.
    pub fn from_wire(value: &Value) -> Result<Self, TaskProductShapeRefusal> {
        let map = object(
            value,
            "",
            &[
                "abstraction",
                "queue_bounds",
                "clock_ceilings",
                "diagonal_constraints",
                "subtractions",
                "time_progress",
            ],
            &[],
        )?;
        if member(map, "abstraction").as_str() != Some(Self::ABSTRACTION) {
            return refuse("/abstraction", TaskProductShapeReason::UnknownVariant);
        }
        let progress = object(
            member(map, "time_progress"),
            "/time_progress",
            &["method", "live_job_cap", "closure"],
            &[],
        )?;
        let method = enumerated(
            member(progress, "method"),
            "/time_progress/method",
            &METHODS,
        )?;
        let live_job_cap = count(
            member(progress, "live_job_cap"),
            "/time_progress/live_job_cap",
        )?;
        let closure = enumerated(
            member(progress, "closure"),
            "/time_progress/closure",
            &CLOSURES,
        )?;
        let (cap_ok, closure_ok) = match method {
            TaskClosureMethod::ClosedFiniteProduct => (
                live_job_cap > 0,
                closure == TaskClosureEvidence::RecomputeAllReachableSuccessors,
            ),
            TaskClosureMethod::ClosedNoReleaseProduct => (
                live_job_cap == 0,
                closure == TaskClosureEvidence::RecomputeAllSourceSuccessorsNoTaskRelease,
            ),
        };
        if !cap_ok {
            return refuse(
                "/time_progress/live_job_cap",
                TaskProductShapeReason::Inconsistent,
            );
        }
        if !closure_ok {
            return refuse(
                "/time_progress/closure",
                TaskProductShapeReason::Inconsistent,
            );
        }

        let rows = array(member(map, "queue_bounds"), "/queue_bounds")?;
        if rows.is_empty() {
            return refuse("/queue_bounds", TaskProductShapeReason::Inconsistent);
        }
        let queue_bounds = rows
            .iter()
            .enumerate()
            .map(|(index, row)| read_queue_bound(row, &format!("/queue_bounds/{index}")))
            .collect::<Result<Vec<_>, _>>()?;
        let clock_ceilings = array(member(map, "clock_ceilings"), "/clock_ceilings")?
            .iter()
            .enumerate()
            .map(|(index, row)| read_ceiling(row, &format!("/clock_ceilings/{index}")))
            .collect::<Result<Vec<_>, _>>()?;
        let diagonal_constraints =
            array(member(map, "diagonal_constraints"), "/diagonal_constraints")?
                .iter()
                .enumerate()
                .map(|(index, row)| read_diagonal(row, &format!("/diagonal_constraints/{index}")))
                .collect::<Result<Vec<_>, _>>()?;
        let subtractions = array(member(map, "subtractions"), "/subtractions")?
            .iter()
            .enumerate()
            .map(|(index, row)| read_subtraction(row, &format!("/subtractions/{index}")))
            .collect::<Result<Vec<_>, _>>()?;
        if method == TaskClosureMethod::ClosedNoReleaseProduct {
            for (name, empty) in [
                ("clock_ceilings", clock_ceilings.is_empty()),
                ("diagonal_constraints", diagonal_constraints.is_empty()),
                ("subtractions", subtractions.is_empty()),
            ] {
                if !empty {
                    return refuse(&format!("/{name}"), TaskProductShapeReason::Inconsistent);
                }
            }
        }
        Ok(Self {
            abstraction: Self::ABSTRACTION,
            clock_ceilings,
            diagonal_constraints,
            queue_bounds,
            subtractions,
            time_progress: TaskTimeProgress {
                method,
                live_job_cap,
                closure,
            },
        })
    }

    /// The member's wire value, members in ascending order. It writes what
    /// the value holds; whether the value is a correct product is the
    /// checker's question, not the writer's.
    ///
    /// # Errors
    ///
    /// [`TaskProductWireError`] when a source transition has no spelling.
    pub fn to_wire(&self) -> Result<Value, TaskProductWireError> {
        let queue_bounds: Vec<Value> = self
            .queue_bounds
            .iter()
            .map(|row| {
                json!({
                    "arrival": row.arrival.as_str(),
                    "interarrival": rational_wire(&row.interarrival),
                    "max_jitter": rational_wire(&row.max_jitter),
                    "max_jobs": row.max_jobs,
                    "relative_deadline": rational_wire(&row.relative_deadline),
                    "task": row.task,
                    "taskset": row.task_set,
                    "work_lower": rational_wire(&row.work_lower),
                    "work_upper": rational_wire(&row.work_upper),
                })
            })
            .collect();
        let clock_ceilings: Vec<Value> = self
            .clock_ceilings
            .iter()
            .map(|row| json!({"clock": row.clock, "maximum": row.maximum.to_string()}))
            .collect();
        let diagonal_constraints: Vec<Value> = self
            .diagonal_constraints
            .iter()
            .map(|row| {
                json!({
                    "constant": row.constant.to_string(),
                    "left": row.left,
                    "operator": row.operator.as_str(),
                    "right": row.right,
                })
            })
            .collect();
        let subtractions = self
            .subtractions
            .iter()
            .map(|row| {
                Ok(json!({
                    "clock": row.clock,
                    "constant": row.constant.to_string(),
                    "transition": row.transition.to_wire()?,
                }))
            })
            .collect::<Result<Vec<Value>, TaskProductWireError>>()?;
        Ok(json!({
            "abstraction": self.abstraction,
            "clock_ceilings": clock_ceilings,
            "diagonal_constraints": diagonal_constraints,
            "queue_bounds": queue_bounds,
            "subtractions": subtractions,
            "time_progress": {
                "closure": self.time_progress.closure.as_str(),
                "live_job_cap": self.time_progress.live_job_cap,
                "method": self.time_progress.method.as_str(),
            },
        }))
    }
}

fn read_queue_bound(
    value: &Value,
    pointer: &str,
) -> Result<TaskQueueBound, TaskProductShapeRefusal> {
    let map = object(
        value,
        pointer,
        &[
            "taskset",
            "task",
            "arrival",
            "interarrival",
            "relative_deadline",
            "max_jitter",
            "work_lower",
            "work_upper",
            "max_jobs",
        ],
        &[],
    )?;
    let at = |name: &str| child(pointer, name);
    Ok(TaskQueueBound {
        task_set: text(member(map, "taskset"), &at("taskset"))?,
        task: text(member(map, "task"), &at("task"))?,
        arrival: enumerated(member(map, "arrival"), &at("arrival"), &ARRIVALS)?,
        interarrival: rational(member(map, "interarrival"), &at("interarrival"))?,
        relative_deadline: rational(member(map, "relative_deadline"), &at("relative_deadline"))?,
        max_jitter: rational(member(map, "max_jitter"), &at("max_jitter"))?,
        work_lower: rational(member(map, "work_lower"), &at("work_lower"))?,
        work_upper: rational(member(map, "work_upper"), &at("work_upper"))?,
        max_jobs: positive_count(member(map, "max_jobs"), &at("max_jobs"))?,
    })
}

fn read_ceiling(value: &Value, pointer: &str) -> Result<TaskClockCeiling, TaskProductShapeRefusal> {
    let map = object(value, pointer, &["clock", "maximum"], &[])?;
    Ok(TaskClockCeiling {
        clock: text(member(map, "clock"), &child(pointer, "clock"))?,
        maximum: positive_integer(member(map, "maximum"), &child(pointer, "maximum"))?,
    })
}

fn read_diagonal(
    value: &Value,
    pointer: &str,
) -> Result<TaskDiagonalConstraint, TaskProductShapeRefusal> {
    let map = object(
        value,
        pointer,
        &["left", "operator", "right", "constant"],
        &[],
    )?;
    Ok(TaskDiagonalConstraint {
        left: text(member(map, "left"), &child(pointer, "left"))?,
        operator: enumerated(
            member(map, "operator"),
            &child(pointer, "operator"),
            &OPERATORS,
        )?,
        right: text(member(map, "right"), &child(pointer, "right"))?,
        constant: integer(member(map, "constant"), &child(pointer, "constant"))?,
    })
}

fn read_subtraction(
    value: &Value,
    pointer: &str,
) -> Result<TaskSubtraction, TaskProductShapeRefusal> {
    let map = object(value, pointer, &["transition", "clock", "constant"], &[])?;
    Ok(TaskSubtraction {
        transition: TaskTransitionTemplate::read(
            member(map, "transition"),
            &child(pointer, "transition"),
        )?,
        clock: text(member(map, "clock"), &child(pointer, "clock"))?,
        constant: positive_integer(member(map, "constant"), &child(pointer, "constant"))?,
    })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    const VECTORS: &str = "proposals/backend-provider-v1/zone-certificate-vectors.json";

    fn vectors() -> Value {
        let bytes = filament_specification::published_file(VECTORS).expect("FSpec publishes ZC");
        serde_json::from_slice(bytes).expect("the vectors are JSON")
    }

    fn vector(id: &str) -> Value {
        vectors()["vectors"]
            .as_array()
            .expect("vectors")
            .iter()
            .find(|vector| vector["id"] == id)
            .unwrap_or_else(|| panic!("vector {id}"))
            .clone()
    }

    /// A published certificate's `task_product` member.
    fn member_of(id: &str) -> Value {
        vector(id)["document"]["task_product"].clone()
    }

    /// A malformed vector applied to its base document, and the value at its
    /// pointer, which the vector replaces.
    fn malformed(id: &str) -> (Value, String) {
        let all = vectors();
        let row = all["malformed_vectors"]
            .as_array()
            .expect("malformed vectors")
            .iter()
            .find(|row| row["id"] == id)
            .unwrap_or_else(|| panic!("malformed vector {id}"))
            .clone();
        let mut document = vector(row["base"].as_str().expect("base"))["document"].clone();
        let pointer = row["pointer"].as_str().expect("pointer").to_owned();
        *document.pointer_mut(&pointer).expect("pointer") = row["value"].clone();
        (document, pointer)
    }

    fn mutated(base: &Value, edit: impl FnOnce(&mut Value)) -> Value {
        let mut value = base.clone();
        edit(&mut value);
        value
    }

    fn refusal(value: &Value) -> TaskProductShapeRefusal {
        TaskProductCertificate::from_wire(value).expect_err("the shape is refused")
    }

    /// The published members read and write back to the published bytes.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn the_published_members_round_trip_byte_for_byte() {
        for id in ["ZC-19", "ZC-23", "ZC-26", "ZC-30"] {
            let document = vector(id)["document"].clone();
            let canonical = vector(id)["canonical_text"].as_str().map(str::to_owned);
            let read = TaskProductCertificate::from_wire(&document["task_product"])
                .unwrap_or_else(|error| panic!("{id}: {error}"));
            let written = read.to_wire().expect("writes");
            assert_eq!(written, document["task_product"], "{id}");
            if let Some(canonical) = canonical {
                let text = serde_json::to_string(&written).expect("text");
                assert!(canonical.contains(&text), "{id} canonical bytes");
            }
        }
    }

    /// The template is the closed finite shape.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn a_template_round_trips_and_carries_an_optional_priority() {
        let mut value = member_of("ZC-19")["subtractions"][0]["transition"].clone();
        value["priority"] = json!(3);
        let template = TaskTransitionTemplate::from_wire(&value).expect("reads");
        assert_eq!(template.priority, Some(3));
        assert_eq!(template.to_wire().expect("writes"), value);
        let mut other = value.clone();
        other["priority"] = json!(4);
        let other = TaskTransitionTemplate::from_wire(&other).expect("reads");
        assert!(template.same_identity(&template));
        assert!(!template.same_identity(&other));
    }

    /// ZC-20 and ZC-21 as published: an exact identity or an occurrence is
    /// not a template.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn the_published_exact_task_identity_vectors_are_not_templates() {
        for (id, reason) in [
            ("ZC-20", TaskProductShapeReason::UnknownMember),
            ("ZC-21", TaskProductShapeReason::UnknownMember),
        ] {
            let (document, pointer) = malformed(id);
            let refused =
                TaskTransitionTemplate::from_wire(document.pointer(&pointer).expect("transition"))
                    .expect_err("refused");
            assert_eq!(refused.reason, reason, "{id}");
        }
    }

    /// ZC-22, ZC-27 and ZC-29 as published.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn the_published_malformed_members_are_refused_at_task_product() {
        for (id, pointer) in [
            ("ZC-22", "/subtractions/0/transition/deadline"),
            ("ZC-27", "/time_progress/live_job_cap"),
            ("ZC-29", "/queue_bounds"),
        ] {
            let (document, _) = malformed(id);
            let refused = refusal(&document["task_product"]);
            assert_eq!(refused.pointer, pointer, "{id}");
            let rejection = refused.rejection();
            assert_eq!(rejection.rule, CertificateRule::Malformed);
            assert_eq!(
                rejection.at,
                CertificateLocus::CertificateMember(
                    filament_contract_model::CertificateMember::TaskProduct
                )
            );
        }
    }

    /// Occurrence, generation and exact quantities on a template.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn an_occurrence_or_generation_added_to_a_template_is_refused() {
        let base = member_of("ZC-19");
        for name in ["job", "generation", "demand", "jitter", "deadline"] {
            let value = mutated(&base, |value| {
                value["subtractions"][0]["transition"][name] = json!(0);
            });
            assert_eq!(
                refusal(&value).reason,
                TaskProductShapeReason::UnknownMember,
                "{name}"
            );
        }
    }

    /// Unknown method, foreign closure label, and a policy member.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn an_unknown_method_a_foreign_closure_label_or_a_policy_is_refused() {
        let base = member_of("ZC-19");
        let method = mutated(&base, |value| {
            value["time_progress"]["method"] = json!("closed-finite-product/v2");
        });
        assert_eq!(
            refusal(&method).reason,
            TaskProductShapeReason::UnknownVariant
        );
        let label = mutated(&base, |value| {
            value["time_progress"]["closure"] = json!("asserted-closed");
        });
        assert_eq!(
            refusal(&label).reason,
            TaskProductShapeReason::UnknownVariant
        );
        let swapped = mutated(&base, |value| {
            value["time_progress"]["closure"] =
                json!("recompute-all-source-successors-no-task-release/v1");
        });
        assert_eq!(refusal(&swapped).pointer, "/time_progress/closure");
        let zero = mutated(&base, |value| {
            value["time_progress"]["live_job_cap"] = json!(0)
        });
        assert_eq!(refusal(&zero).pointer, "/time_progress/live_job_cap");
        let policy = mutated(&base, |value| value["policy"] = json!("edf"));
        assert_eq!(
            refusal(&policy).reason,
            TaskProductShapeReason::UnknownMember
        );
        let missing = mutated(&base, |value| {
            value
                .as_object_mut()
                .expect("object")
                .remove("subtractions");
        });
        assert_eq!(refusal(&missing).reason, TaskProductShapeReason::Missing);
    }

    /// AC-14 shape: a positive cap, scheduler clocks, diagonals or
    /// subtractions under the no-release method.
    #[trace("TC-371", "FR-418-AC-14")]
    #[test]
    fn the_no_release_method_refuses_a_cap_and_scheduler_metadata() {
        let base = member_of("ZC-26");
        let cap = mutated(&base, |value| {
            value["time_progress"]["live_job_cap"] = json!(1)
        });
        assert_eq!(refusal(&cap).pointer, "/time_progress/live_job_cap");
        let ceiling = mutated(&base, |value| {
            value["clock_ceilings"] = json!([{"clock": "A.service", "maximum": "2"}]);
        });
        assert_eq!(refusal(&ceiling).pointer, "/clock_ceilings");
        let diagonal = mutated(&base, |value| {
            value["diagonal_constraints"] =
                json!([{"left": "a", "operator": "<=", "right": "b", "constant": "-1"}]);
        });
        assert_eq!(refusal(&diagonal).pointer, "/diagonal_constraints");
        let subtraction = mutated(&base, |value| {
            value["subtractions"] = member_of("ZC-19")["subtractions"].clone();
        });
        assert_eq!(refusal(&subtraction).pointer, "/subtractions");
        let label = mutated(&base, |value| {
            value["time_progress"]["closure"] = json!("recompute-all-reachable-successors/v1");
        });
        assert_eq!(refusal(&label).pointer, "/time_progress/closure");
        assert!(TaskProductCertificate::from_wire(&base).is_ok());
    }

    /// Counts and integer strings: the 2^53 bound, floats, `-0`, `+1`,
    /// leading zeros, and zero or negative denominators.
    #[trace("TC-371", "FR-418-AC-7")]
    #[test]
    fn counts_and_integer_strings_follow_the_number_encoding() {
        let base = member_of("ZC-19");
        let jobs = |value: Value| mutated(&base, |doc| doc["queue_bounds"][0]["max_jobs"] = value);
        assert!(TaskProductCertificate::from_wire(&jobs(json!(9_007_199_254_740_991_u64))).is_ok());
        assert_eq!(
            refusal(&jobs(json!(9_007_199_254_740_992_u64))).reason,
            TaskProductShapeReason::NotCanonical
        );
        assert_eq!(
            refusal(&jobs(json!(1.0))).reason,
            TaskProductShapeReason::NotCanonical
        );
        assert_eq!(refusal(&jobs(json!(0))).pointer, "/queue_bounds/0/max_jobs");
        for spelling in ["01", "+1", "-0", "-1", "0", ""] {
            let value = mutated(&base, |doc| {
                doc["subtractions"][0]["constant"] = json!(spelling);
            });
            assert_eq!(
                refusal(&value).reason,
                TaskProductShapeReason::NotCanonical,
                "{spelling:?}"
            );
        }
        let number = mutated(&base, |doc| doc["subtractions"][0]["constant"] = json!(1));
        assert_eq!(refusal(&number).reason, TaskProductShapeReason::WrongKind);
        let diagonal = |constant: &str| {
            mutated(&base, |doc| {
                doc["diagonal_constraints"] =
                    json!([{"constant": constant, "left": "a", "operator": "<", "right": "b"}]);
            })
        };
        assert!(TaskProductCertificate::from_wire(&diagonal("-5")).is_ok());
        assert_eq!(
            refusal(&diagonal("-0")).reason,
            TaskProductShapeReason::NotCanonical
        );
        for (numerator, denominator) in [("4", "2"), ("1", "0"), ("1", "-1"), ("1", "+2")] {
            let value = mutated(&base, |doc| {
                doc["queue_bounds"][0]["interarrival"] =
                    json!({"numerator": numerator, "denominator": denominator});
            });
            assert_eq!(
                refusal(&value).reason,
                TaskProductShapeReason::NotCanonical,
                "{numerator}/{denominator}"
            );
        }
        let priority = |value: Value| {
            let mut template = member_of("ZC-19")["subtractions"][0]["transition"].clone();
            template["priority"] = value;
            TaskTransitionTemplate::from_wire(&template)
        };
        assert!(priority(json!(2)).is_ok());
        assert_eq!(
            priority(json!("2")).expect_err("kind").reason,
            TaskProductShapeReason::WrongKind
        );
        assert_eq!(
            priority(json!(-1)).expect_err("sign").reason,
            TaskProductShapeReason::NotCanonical
        );
    }

    /// Ceilings and subtraction constants carry any size.
    #[trace("TC-371", "FR-418-AC-7")]
    #[test]
    fn ceilings_and_constants_of_any_size_round_trip() {
        let huge = "340282366920938463463374607431768211456123";
        let value = mutated(&member_of("ZC-19"), |doc| {
            doc["clock_ceilings"] = json!([{"clock": "A.service", "maximum": huge}]);
            doc["subtractions"][0]["constant"] = json!(huge);
            doc["diagonal_constraints"] = json!([
                {"constant": "-1", "left": "a", "operator": "<", "right": "b"},
                {"constant": "0", "left": "a", "operator": "<=", "right": "b"},
                {"constant": "2", "left": "a", "operator": ">", "right": "b"},
                {"constant": huge, "left": "a", "operator": ">=", "right": "b"},
            ]);
        });
        let read = TaskProductCertificate::from_wire(&value).expect("reads");
        assert_eq!(read.clock_ceilings[0].maximum.to_string(), huge);
        assert_eq!(read.subtractions[0].constant.to_string(), huge);
        assert_eq!(read.to_wire().expect("writes"), value);
        let zero = mutated(&value, |doc| {
            doc["clock_ceilings"][0]["maximum"] = json!("0")
        });
        assert_eq!(refusal(&zero).reason, TaskProductShapeReason::NotCanonical);
        let bad = mutated(&value, |doc| {
            doc["diagonal_constraints"][0]["operator"] = json!("=")
        });
        assert_eq!(refusal(&bad).reason, TaskProductShapeReason::UnknownVariant);
    }

    fn template_with(release: Value) -> Value {
        let mut template = member_of("ZC-19")["subtractions"][0]["transition"].clone();
        template["release"] = release;
        template
    }

    /// Receivers and arguments are `TypedValue`s.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn a_receiver_or_argument_must_be_a_typed_value() {
        let identity = |receiver: Value, arguments: Value| {
            json!({"type": "transition", "operation": "Cell::set",
                   "receiver": receiver, "arguments": arguments})
        };
        let good = identity(
            json!({"type": "record", "name": "Cell", "fields": [
                {"name": "n", "value": {"type": "integer", "value": "1"}},
                {"name": "m", "value": null}]}),
            json!([{"type": "option", "value": null},
                   {"type": "set", "elements": [{"type": "text", "value": "x"}]},
                   {"type": "boolean", "value": true},
                   {"type": "float32", "bits": "3f800000"}]),
        );
        assert!(TaskTransitionTemplate::from_wire(&template_with(good)).is_ok());
        for bad in [
            identity(json!({"type": "x"}), json!([])),
            identity(json!({"type": "integer"}), json!([])),
            identity(json!({"type": "integer", "value": "01"}), json!([])),
            identity(json!(null), json!([{"type": "boolean", "value": 1}])),
            identity(json!(null), json!([{"type": "float32", "bits": "ZZ"}])),
            identity(
                json!(null),
                json!([{"type": "set", "elements": [{"type": "?"}]}]),
            ),
            identity(
                json!(null),
                json!([{"type": "text", "value": "a", "extra": 1}]),
            ),
        ] {
            assert!(
                TaskTransitionTemplate::from_wire(&template_with(bad.clone())).is_err(),
                "{bad}"
            );
        }
    }

    fn transition(operation: &str) -> ModelTransition {
        ModelTransition::new(operation.to_owned(), None, Vec::new(), IDENTITY_LIMITS)
            .expect("canonical identity")
    }

    /// A checked transition's node-identity operation is spelled on the wire
    /// as one string; comparison is by that spelling, and a plain string and
    /// a `{"name"}` identity are one wire identity.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn source_transitions_compare_by_their_wire_spelling() {
        let node = transition(r#"{"node":"n1","package":"p"}"#);
        let named = transition(r#"{"name":"Task::release"}"#);
        let plain = transition(r#""Task::release""#);
        let wire = crate::task_source_transition_wire(&node).expect("wire");
        assert_eq!(wire["operation"], "n1");
        let read = source_transition(&wire, "").expect("reads");
        assert_ne!(read, node, "structural equality sees the flattening");
        assert!(same_source_transition(&read, &node));
        assert!(same_source_transition(&named, &plain));
        assert!(!same_source_transition(&node, &plain));
        let mut template = TaskTransitionTemplate::from_wire(&template_with(wire)).expect("reads");
        let mut other = template.clone();
        template.release = node;
        other.release = read;
        assert!(template.same_identity(&other));
    }
}
