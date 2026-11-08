// SPDX-License-Identifier: AGPL-3.0-or-later
//! `planner::read_result_entry` (QSL FR-529, QSpec FR-331 "Result entries"):
//! the strict reader of one FR-331 result entry into a [`TerminalRecord`]
//! and its certification, in FR-331's reading order:
//!
//! 1. the entry is an object (`wrong-value-kind` at `""`);
//! 2. `result` is present, a string and one of the nine results;
//! 3. a `measured` entry carries no truth or basis (`measured-with-truth`);
//! 4. every member the result requires is present (`missing-member`);
//! 5. no member the result does not define is present (`unknown-member`);
//! 6. a `proved` entry's `certification` is a string, one of the three
//!    values and the one the Certification table fixes
//!    (`certification-mismatch`);
//! 7. the `basis` or `cause`, then each remaining member in member-name
//!    order, is read as a value.
//!
//! A plugin's entry goes through FR-290's readers
//! ([`read_plugin_basis`], [`read_plugin_certification`],
//! [`read_plugin_inconclusive_cause`]); every other entry through the
//! first-party arms below. A value the planner's types cannot hold (a
//! certificate, a hybrid trace, a statistical verdict, ...) is refused
//! `unknown_required_feature`/`unsupported-feature` at the point in the
//! reading order where it is read.

use qsl_foundation::{BackendId, Code, RequestIndex};
use quire_contract_model::Std001Code;
use quire_exact::CancelCause;
use serde_json::{Map, Value};

use super::json::MAX_COUNT;
use super::key::ClaimKind;
use crate::proof_result::{
    child, name_order, read_plugin_basis, read_plugin_certification,
    read_plugin_inconclusive_cause, Certification, DeclineCode, IncompleteCause, InconclusiveCause,
    PluginResultRefusal, ProofBasis, ProofRefusalCause, TerminalRecord, TerminalValue,
    UnavailabilityCause, ValueFault,
};

/// Who produced an entry. FR-331's Certification table fixes `trusted` for a
/// backend loaded as a plugin (FR-305), so the reader needs to know.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryOrigin {
    /// A first-party backend.
    FirstParty,
    /// A backend loaded as a plugin.
    Plugin,
}

/// The limit a `limit-reached` cause names (QSpec FR-461), kept as written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LimitFacts {
    /// The limit's name.
    pub limit: String,
    /// The value it was set to.
    pub value: u64,
    /// The request setting that raises it.
    pub setting: String,
}

/// One FR-331 result entry as the planner holds it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResultEntry {
    /// The entry's `request_index`.
    pub request_index: u64,
    /// The producing backend.
    pub backend: BackendId,
    /// The entry's FR-290 kind.
    pub kind: ClaimKind,
    /// The entry as a terminal record.
    pub record: TerminalRecord,
    /// A `proved` entry's certification as the entry states it (a plugin's
    /// is `trusted`).
    pub certification: Option<Certification>,
    /// An `unsupported`, `declined` or `failed` entry's FR-272 diagnostic,
    /// kept as written.
    pub diagnostic: Option<Value>,
    /// The limit an `incomplete` entry's `limit-reached` cause names.
    pub limit_reached: Option<LimitFacts>,
}

impl ResultEntry {
    /// This entry as the planner accepts it from request bytes: the planner
    /// checks no certificate, so a `certified` entry is read as
    /// `uncertified` (QSpec FR-492). A `trusted` or `uncertified` entry is
    /// unchanged.
    #[must_use]
    pub fn read_from_bytes(mut self) -> Self {
        if self.certification == Some(Certification::Certified) {
            self.certification = Some(Certification::Uncertified);
            if let TerminalValue::Proved { basis, .. } = self.record.value().clone() {
                self.record = TerminalRecord::with_backend(
                    self.record.request_index(),
                    self.backend.clone(),
                    TerminalValue::Proved {
                        basis,
                        certification: Certification::Uncertified,
                    },
                );
            }
        }
        self
    }
}

/// The FR-272 `invalid_result` cause of a defect (or the
/// `unknown_required_feature` of a value the planner cannot hold).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntryCause {
    /// A member the result requires is absent.
    MissingMember,
    /// A member the result does not define is present.
    UnknownMember,
    /// A member's JSON kind is not the kind its domain takes.
    WrongValueKind,
    /// A member of the right kind lies outside its domain.
    InvalidValue,
    /// A `proved` entry's certification is not the one the Certification
    /// table fixes.
    CertificationMismatch {
        /// The fixed value, when there is one.
        expected: Option<Certification>,
        /// The received value.
        actual: Certification,
    },
    /// A cause or basis spelling outside the closed set, or outside the
    /// subset the producing backend may send.
    UnknownCause,
    /// A `measured` entry carries a truth or a basis.
    MeasuredWithTruth,
    /// A valid value the planner's types cannot hold: the received spelling.
    UnsupportedFeature {
        /// The received spelling.
        feature: String,
    },
}

impl EntryCause {
    /// The FR-272 cause spelling.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::MissingMember => "missing-member",
            Self::UnknownMember => "unknown-member",
            Self::WrongValueKind => "wrong-value-kind",
            Self::InvalidValue => "invalid-value",
            Self::CertificationMismatch { .. } => "certification-mismatch",
            Self::UnknownCause => "unknown-cause",
            Self::MeasuredWithTruth => "measured-with-truth",
            Self::UnsupportedFeature { .. } => "unsupported-feature",
        }
    }

    /// The FR-272 code the cause belongs to.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedFeature { .. } => "unknown_required_feature",
            _ => "invalid_result",
        }
    }
}

/// A result entry the planner refuses, naming its first defect in FR-331's
/// reading order.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{}/{} at `{pointer}` of result item {item_index}", .cause.code(), .cause.as_str())]
pub struct ResultEntryRefusal {
    /// The result item index the entry answers.
    pub item_index: usize,
    /// The offending member's RFC 6901 pointer relative to the entry (the
    /// empty pointer names the whole entry).
    pub pointer: String,
    /// The cause.
    pub cause: EntryCause,
}

struct Defect {
    pointer: String,
    cause: EntryCause,
}

impl Defect {
    fn new(pointer: impl Into<String>, cause: EntryCause) -> Self {
        Self {
            pointer: pointer.into(),
            cause,
        }
    }

    fn missing(pointer: String) -> Self {
        Self::new(pointer, EntryCause::MissingMember)
    }

    fn unknown(pointer: String) -> Self {
        Self::new(pointer, EntryCause::UnknownMember)
    }

    fn wrong_kind(pointer: String) -> Self {
        Self::new(pointer, EntryCause::WrongValueKind)
    }

    fn invalid(pointer: String) -> Self {
        Self::new(pointer, EntryCause::InvalidValue)
    }

    fn unknown_cause(pointer: String) -> Self {
        Self::new(pointer, EntryCause::UnknownCause)
    }

    fn unsupported(pointer: String, feature: &str) -> Self {
        Self::new(
            pointer,
            EntryCause::UnsupportedFeature {
                feature: feature.to_owned(),
            },
        )
    }
}

impl From<PluginResultRefusal> for Defect {
    fn from(refusal: PluginResultRefusal) -> Self {
        let fault = |cause: ValueFault| match cause {
            ValueFault::MissingMember => EntryCause::MissingMember,
            ValueFault::WrongValueKind => EntryCause::WrongValueKind,
            ValueFault::InvalidValue => EntryCause::InvalidValue,
        };
        match refusal {
            PluginResultRefusal::QslOnlyCause(_) => Self::unknown_cause("/cause/type".to_owned()),
            PluginResultRefusal::CoreOnlyBasis => Self::unknown_cause("/basis/type".to_owned()),
            PluginResultRefusal::MissingCounterexample { pointer, cause } => {
                Self::new(pointer, fault(cause))
            }
            PluginResultRefusal::UnknownSpelling { pointer } => Self::unknown_cause(pointer),
            PluginResultRefusal::MissingMember { pointer } => Self::missing(pointer),
            PluginResultRefusal::UnknownMember { pointer } => Self::unknown(pointer),
            PluginResultRefusal::Malformed { pointer, cause } => Self::new(pointer, fault(cause)),
            PluginResultRefusal::Unsupported { pointer, feature } => {
                Self::unsupported(pointer, &feature)
            }
            PluginResultRefusal::CertificationNotTrusted { received } => Self::new(
                "/certification",
                EntryCause::CertificationMismatch {
                    expected: Some(Certification::Trusted),
                    actual: parse_certification(&received).unwrap_or(Certification::Uncertified),
                },
            ),
        }
    }
}

/// The nine FR-331 result values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResultValue {
    Proved,
    Refuted,
    Tested,
    Measured,
    Inconclusive,
    Unsupported,
    Declined,
    Incomplete,
    Failed,
}

impl ResultValue {
    #[ix_trace_rs::string_edge]
    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "proved" => Self::Proved,
            "refuted" => Self::Refuted,
            "tested" => Self::Tested,
            "measured" => Self::Measured,
            "inconclusive" => Self::Inconclusive,
            "unsupported" => Self::Unsupported,
            "declined" => Self::Declined,
            "incomplete" => Self::Incomplete,
            "failed" => Self::Failed,
            _ => return None,
        })
    }

    /// The members this result requires, may carry, and the group of which
    /// exactly one is carried.
    const fn shape(self) -> Shape {
        match self {
            Self::Proved => Shape {
                required: &["basis", "certification"],
                optional: &["certificate", "cross_check", "evidence"],
                one_of: &[],
            },
            Self::Refuted => Shape {
                required: &["counterexample"],
                optional: &["cause"],
                one_of: &[],
            },
            Self::Tested => Shape {
                required: &[],
                optional: &[],
                one_of: &[],
            },
            Self::Measured => Shape {
                required: &["verdict"],
                optional: &[],
                one_of: &[],
            },
            Self::Inconclusive => Shape {
                required: &["cause"],
                optional: &[],
                one_of: &[],
            },
            Self::Unsupported | Self::Failed => Shape {
                required: &[],
                optional: &[],
                one_of: &["cause", "diagnostic"],
            },
            Self::Declined => Shape {
                required: &["cause", "diagnostic"],
                optional: &[],
                one_of: &[],
            },
            Self::Incomplete => Shape {
                required: &["cause"],
                optional: &["task_search"],
                one_of: &[],
            },
        }
    }
}

struct Shape {
    required: &'static [&'static str],
    optional: &'static [&'static str],
    one_of: &'static [&'static str],
}

/// The members every entry carries.
const COMMON: [&str; 4] = ["backend", "kind", "request_index", "result"];

#[ix_trace_rs::string_edge]
fn parse_certification(text: &str) -> Option<Certification> {
    match text {
        "certified" => Some(Certification::Certified),
        "uncertified" => Some(Certification::Uncertified),
        "trusted" => Some(Certification::Trusted),
        _ => None,
    }
}

/// Read one FR-331 result entry. `item_index` is the result item the entry
/// answers, kept in the refusal.
///
/// # Errors
///
/// [`ResultEntryRefusal`] naming the first defect in FR-331's reading order.
pub fn read_result_entry(
    entry: &Value,
    item_index: usize,
    origin: EntryOrigin,
) -> Result<ResultEntry, ResultEntryRefusal> {
    read_entry(entry, origin).map_err(|defect| ResultEntryRefusal {
        item_index,
        pointer: defect.pointer,
        cause: defect.cause,
    })
}

#[ix_trace_rs::string_edge]
fn read_entry(entry: &Value, origin: EntryOrigin) -> Result<ResultEntry, Defect> {
    // Step 1.
    let Value::Object(members) = entry else {
        return Err(Defect::wrong_kind(String::new()));
    };
    // Step 2.
    let result = match members.get("result") {
        None => return Err(Defect::missing("/result".to_owned())),
        Some(Value::String(text)) => {
            ResultValue::parse(text).ok_or_else(|| Defect::invalid("/result".to_owned()))?
        }
        Some(_) => return Err(Defect::wrong_kind("/result".to_owned())),
    };
    // Step 3.
    if result == ResultValue::Measured {
        for name in ["basis", "truth"] {
            if members.contains_key(name) {
                return Err(Defect::new(child("", name), EntryCause::MeasuredWithTruth));
            }
        }
    }
    let shape = result.shape();
    // Step 4.
    check_required(members, &shape)?;
    // Step 5.
    check_defined(members, &shape)?;
    // Step 6.
    let certification = if result == ResultValue::Proved {
        Some(read_certification(members, origin)?)
    } else {
        None
    };
    // Step 7: the basis or cause, then each remaining member in name order.
    let mut main = Main::None;
    if result == ResultValue::Proved {
        main = Main::Basis(read_basis(member(members, "basis")?, origin)?);
    } else if let Some(cause) = members.get("cause") {
        main = Main::Cause(read_cause(result, cause)?);
    }
    let mut names: Vec<&String> = members.keys().collect();
    names.sort_by(|left, right| name_order(left, right));
    let mut rest = Rest::default();
    for name in names {
        let value = member(members, name)?;
        let pointer = child("", name);
        match name.as_str() {
            "backend" => rest.backend = Some(read_backend(value, &pointer)?),
            "kind" => rest.kind = Some(read_kind(value, &pointer)?),
            "request_index" => rest.request_index = Some(read_count(value, &pointer)?),
            "counterexample" => {
                read_count(value, &pointer)?;
            }
            "diagnostic" => rest.diagnostic = Some(read_diagnostic(value, &pointer)?),
            "certificate" | "cross_check" | "evidence" | "task_search" | "verdict" => {
                return Err(Defect::unsupported(pointer, name));
            }
            // `basis`, `cause`, `certification` and `result` are read above.
            _ => {}
        }
    }
    assemble(result, origin, certification, main, rest)
}

fn member<'a>(members: &'a Map<String, Value>, name: &str) -> Result<&'a Value, Defect> {
    members
        .get(name)
        .ok_or_else(|| Defect::missing(child("", name)))
}

fn check_required(members: &Map<String, Value>, shape: &Shape) -> Result<(), Defect> {
    let mut absent: Vec<&str> = COMMON
        .iter()
        .chain(shape.required)
        .copied()
        .filter(|name| !members.contains_key(*name))
        .collect();
    if !shape.one_of.is_empty() && !shape.one_of.iter().any(|name| members.contains_key(*name)) {
        absent.push(shape.one_of[0]);
    }
    absent.sort_by(|left, right| name_order(left, right));
    match absent.first() {
        Some(name) => Err(Defect::missing(child("", name))),
        None => Ok(()),
    }
}

fn check_defined(members: &Map<String, Value>, shape: &Shape) -> Result<(), Defect> {
    let mut undefined: Vec<&str> = members
        .keys()
        .map(String::as_str)
        .filter(|name| {
            !COMMON.contains(name)
                && !shape.required.contains(name)
                && !shape.optional.contains(name)
                && !shape.one_of.contains(name)
        })
        .collect();
    // Exactly one of a group: the later one in member-name order is the
    // member the result does not define.
    let carried: Vec<&str> = shape
        .one_of
        .iter()
        .copied()
        .filter(|name| members.contains_key(*name))
        .collect();
    if carried.len() > 1 {
        undefined.extend(carried.iter().skip(1));
    }
    undefined.sort_by(|left, right| name_order(left, right));
    match undefined.first() {
        Some(name) => Err(Defect::unknown(child("", name))),
        None => Ok(()),
    }
}

/// The certification the Certification table fixes for a proof, from the raw
/// basis: `None` when it leaves the choice between `certified` and
/// `uncertified` to a check the entry does not show.
#[ix_trace_rs::string_edge]
fn expected_certification(origin: EntryOrigin, basis: Option<&Value>) -> Option<Certification> {
    if origin == EntryOrigin::Plugin {
        return Some(Certification::Trusted);
    }
    let basis = basis?.as_object()?;
    match basis.get("type")?.as_str()? {
        "bounded-proof" | "exact-value" | "value-bounds" | "zone-certified" | "closed-form"
        | "witnessed" => Some(Certification::Certified),
        "bounded-solver" | "inductive-solver" => Some(Certification::Uncertified),
        "reduced" => {
            let reductions = basis.get("reductions")?.as_array()?;
            let mut copy_swap_only = true;
            for reduction in reductions {
                match reduction.get("reduction")?.as_str()? {
                    "copy-swap" => {}
                    "symmetry" | "partial-order" => copy_swap_only = false,
                    _ => return None,
                }
            }
            Some(if copy_swap_only {
                Certification::Certified
            } else {
                Certification::Uncertified
            })
        }
        _ => None,
    }
}

#[ix_trace_rs::string_edge]
fn read_certification(
    members: &Map<String, Value>,
    origin: EntryOrigin,
) -> Result<Certification, Defect> {
    const POINTER: &str = "/certification";
    let value = member(members, "certification")?;
    if origin == EntryOrigin::Plugin {
        read_plugin_certification(Some(value))?;
        return Ok(Certification::Trusted);
    }
    let text = value
        .as_str()
        .ok_or_else(|| Defect::wrong_kind(POINTER.to_owned()))?;
    let actual = parse_certification(text).ok_or_else(|| Defect::invalid(POINTER.to_owned()))?;
    let expected = expected_certification(origin, members.get("basis"));
    let admitted = match expected {
        Some(expected) => actual == expected,
        None => actual != Certification::Trusted,
    };
    if admitted {
        Ok(actual)
    } else {
        Err(Defect::new(
            POINTER,
            EntryCause::CertificationMismatch { expected, actual },
        ))
    }
}

/// The value of a `basis` or `cause` member, read before the other members.
enum Main {
    None,
    Basis(ProofBasis),
    Cause(CauseValue),
}

/// The members read after `basis` and `cause`.
#[derive(Default)]
struct Rest {
    backend: Option<BackendId>,
    kind: Option<ClaimKind>,
    request_index: Option<u64>,
    diagnostic: Option<Diagnostic>,
}

struct Diagnostic {
    code: String,
    wire: Value,
}

/// A cause as the planner holds it.
enum CauseValue {
    Inconclusive(InconclusiveCause),
    Unsupported(UnavailabilityCause),
    Incomplete(IncompleteCause, Option<LimitFacts>),
    Declined(ProofRefusalCause),
    /// `refuted`'s optional cause, `failed`'s `tool-failure`: nothing the
    /// planner keeps.
    Plain,
}

#[ix_trace_rs::string_edge]
fn read_basis(value: &Value, origin: EntryOrigin) -> Result<ProofBasis, Defect> {
    if origin == EntryOrigin::FirstParty {
        if let Some(kind) = value.get("type").and_then(Value::as_str) {
            match kind {
                "bounded-proof" => {
                    let (members, _) = typed_object(value, "basis", BOUNDED_PROOF)?;
                    let checks =
                        read_count(member_of(members, "checks", "/basis")?, "/basis/checks")?;
                    return match u32::try_from(checks) {
                        Ok(success_checks) if success_checks > 0 => {
                            Ok(ProofBasis::Checks { success_checks })
                        }
                        _ => Err(Defect::invalid("/basis/checks".to_owned())),
                    };
                }
                // A basis whose certificate or evidence the planner cannot
                // hold.
                "exact-value" | "value-bounds" | "zone-certified" | "closed-form" | "witnessed" => {
                    return Err(Defect::unsupported("/basis/type".to_owned(), kind));
                }
                _ => {}
            }
        }
    }
    Ok(read_plugin_basis(value)?)
}

type Table = &'static [(
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
)];

const BOUNDED_PROOF: Table = &[("bounded-proof", &["checks"], &[])];

const INCOMPLETE: Table = &[
    ("limit-reached", &["limit", "setting", "value"], &[]),
    ("bounded-search-incomplete", &["reason"], &[]),
    ("precision-budget", &["lower", "upper"], &[]),
    ("cancelled", &["source"], &[]),
    ("timed-out", &[], &[]),
];

const UNSUPPORTED: Table = &[
    ("missing-confidence", &[], &[]),
    ("every-scheduler", &[], &[]),
    ("exact-only-form", &[], &[]),
    ("confidence-beyond-precision", &[], &[]),
    ("not-markov", &["post_states", "state", "transition"], &[]),
    ("not-stochastic", &["identity", "state"], &[]),
    ("time-locked-sample", &["state"], &[]),
    ("total-above-draw-range", &["total"], &[]),
    ("zero-weight-component", &["component"], &[]),
    ("delay-distribution", &["operation"], &[]),
    ("strict-clock-constraint", &["locus"], &[]),
    ("zero-delay-cycle", &["states"], &[]),
    ("timed-form-shape", &["locus"], &[]),
    ("punctual-interval", &["locus"], &[]),
    ("stopwatch-required", &[], &[]),
    ("weak-access-shape", &["shape", "site", "transition"], &[]),
    ("recovery-view", &["predicate"], &[]),
];

const DECLINED: Table = &[
    ("refused", &[], &[]),
    ("invalid-input", &[], &[]),
    ("incomplete-input", &[], &[]),
];

const REFUTED: Table = &[("undefined-evaluation", &["cause", "where"], &[])];

const FAILED: Table = &[("tool-failure", &["defect"], &["limit", "value"])];

/// The ordered check of a typed `cause` object: its kind, `type` member
/// (absence, kind, spelling), then its required and undefined payload
/// members. Returns the payload and the spelling.
#[ix_trace_rs::string_edge]
fn typed_object<'a>(
    value: &'a Value,
    member_name: &str,
    table: Table,
) -> Result<(&'a Map<String, Value>, &'static str), Defect> {
    let base = child("", member_name);
    let Value::Object(members) = value else {
        return Err(Defect::wrong_kind(base));
    };
    let type_pointer = child(&base, "type");
    let kind = match members.get("type") {
        None => return Err(Defect::missing(type_pointer)),
        Some(Value::String(kind)) => kind.as_str(),
        Some(_) => return Err(Defect::wrong_kind(type_pointer)),
    };
    let Some((name, required, optional)) = table.iter().find(|(name, _, _)| *name == kind) else {
        return Err(Defect::unknown_cause(type_pointer));
    };
    let mut absent: Vec<&str> = required
        .iter()
        .copied()
        .filter(|payload| !members.contains_key(*payload))
        .collect();
    absent.sort_by(|left, right| name_order(left, right));
    if let Some(payload) = absent.first() {
        return Err(Defect::missing(child(&base, payload)));
    }
    let mut undefined: Vec<&str> = members
        .keys()
        .map(String::as_str)
        .filter(|payload| {
            *payload != "type" && !required.contains(payload) && !optional.contains(payload)
        })
        .collect();
    undefined.sort_by(|left, right| name_order(left, right));
    if let Some(payload) = undefined.first() {
        return Err(Defect::unknown(child(&base, payload)));
    }
    Ok((members, name))
}

fn member_of<'a>(
    members: &'a Map<String, Value>,
    name: &str,
    base: &str,
) -> Result<&'a Value, Defect> {
    members
        .get(name)
        .ok_or_else(|| Defect::missing(child(base, name)))
}

#[ix_trace_rs::string_edge]
fn read_cause(result: ResultValue, value: &Value) -> Result<CauseValue, Defect> {
    match result {
        ResultValue::Inconclusive => read_inconclusive(value).map(CauseValue::Inconclusive),
        ResultValue::Incomplete => read_incomplete(value),
        ResultValue::Unsupported => {
            let (_, name) = typed_object(value, "cause", UNSUPPORTED)?;
            match name {
                "confidence-beyond-precision" => Ok(CauseValue::Unsupported(
                    UnavailabilityCause::ConfidenceBeyondPrecision,
                )),
                other => Err(Defect::unsupported("/cause/type".to_owned(), other)),
            }
        }
        ResultValue::Declined => {
            let (_, name) = typed_object(value, "cause", DECLINED)?;
            Ok(CauseValue::Declined(match name {
                "refused" => ProofRefusalCause::Refused,
                "invalid-input" => ProofRefusalCause::InvalidInput,
                _ => ProofRefusalCause::IncompleteInput,
            }))
        }
        ResultValue::Refuted => {
            let (_, name) = typed_object(value, "cause", REFUTED)?;
            Err(Defect::unsupported("/cause/type".to_owned(), name))
        }
        ResultValue::Failed => {
            typed_object(value, "cause", FAILED)?;
            Ok(CauseValue::Plain)
        }
        // `proved`, `tested` and `measured` define no `cause`.
        ResultValue::Proved | ResultValue::Tested | ResultValue::Measured => Ok(CauseValue::Plain),
    }
}

#[ix_trace_rs::string_edge]
fn read_inconclusive(value: &Value) -> Result<InconclusiveCause, Defect> {
    // A first-party cause the plugin reader's table leaves out. Every one is
    // read here, before the plugin reader, because that reader refuses the
    // core-only causes as a plugin may not send them.
    if let Some(kind) = value.get("type").and_then(Value::as_str) {
        match kind {
            "kani_vacuous_proof" | "refuted-ignoring-constraints" => {
                const NO_PAYLOAD: Table = &[
                    ("kani_vacuous_proof", &[], &[]),
                    ("refuted-ignoring-constraints", &[], &[]),
                ];
                typed_object(value, "cause", NO_PAYLOAD)?;
                return Ok(if kind == "kani_vacuous_proof" {
                    InconclusiveCause::KaniVacuousProof
                } else {
                    InconclusiveCause::RefutedIgnoringConstraints
                });
            }
            "replay-parity" | "replay-refused" | "cross-check-disagreement" => {
                return Err(Defect::unsupported("/cause/type".to_owned(), kind));
            }
            _ => {}
        }
    }
    Ok(read_plugin_inconclusive_cause(value)?)
}

#[ix_trace_rs::string_edge]
fn read_incomplete(value: &Value) -> Result<CauseValue, Defect> {
    let (members, name) = typed_object(value, "cause", INCOMPLETE)?;
    match name {
        "limit-reached" => {
            let limit = read_text(member_of(members, "limit", "/cause")?, "/cause/limit")?;
            let setting = read_text(member_of(members, "setting", "/cause")?, "/cause/setting")?;
            let value = read_count(member_of(members, "value", "/cause")?, "/cause/value")?;
            Ok(CauseValue::Incomplete(
                IncompleteCause::ResourceExhausted,
                Some(LimitFacts {
                    limit,
                    value,
                    setting,
                }),
            ))
        }
        "timed-out" => Ok(CauseValue::Incomplete(IncompleteCause::TimedOut, None)),
        "cancelled" => {
            let source = member_of(members, "source", "/cause")?;
            let source = match source.as_str() {
                Some("requested") => CancelCause::Requested,
                Some("deadline") => CancelCause::Deadline,
                Some(_) => return Err(Defect::invalid("/cause/source".to_owned())),
                None => return Err(Defect::wrong_kind("/cause/source".to_owned())),
            };
            Ok(CauseValue::Incomplete(
                IncompleteCause::Cancelled { source },
                None,
            ))
        }
        other => Err(Defect::unsupported("/cause/type".to_owned(), other)),
    }
}

fn read_text(value: &Value, pointer: &str) -> Result<String, Defect> {
    match value.as_str() {
        Some(text) if !text.is_empty() => Ok(text.to_owned()),
        Some(_) => Err(Defect::invalid(pointer.to_owned())),
        None => Err(Defect::wrong_kind(pointer.to_owned())),
    }
}

fn read_count(value: &Value, pointer: &str) -> Result<u64, Defect> {
    match value.as_u64() {
        Some(count) if count <= MAX_COUNT => Ok(count),
        _ if value.is_u64() || value.is_i64() => Err(Defect::invalid(pointer.to_owned())),
        _ => Err(Defect::wrong_kind(pointer.to_owned())),
    }
}

fn read_backend(value: &Value, pointer: &str) -> Result<BackendId, Defect> {
    read_text(value, pointer).map(BackendId::new)
}

fn read_kind(value: &Value, pointer: &str) -> Result<ClaimKind, Defect> {
    let text = read_text(value, pointer)?;
    ClaimKind::parse(&text).ok_or_else(|| Defect::invalid(pointer.to_owned()))
}

/// An FR-272 typed cause `{code, cause, detail}`.
fn read_diagnostic(value: &Value, pointer: &str) -> Result<Diagnostic, Defect> {
    let Value::Object(members) = value else {
        return Err(Defect::wrong_kind(pointer.to_owned()));
    };
    for name in ["cause", "code", "detail"] {
        if !members.contains_key(name) {
            return Err(Defect::missing(child(pointer, name)));
        }
    }
    let mut undefined: Vec<&str> = members
        .keys()
        .map(String::as_str)
        .filter(|name| !["cause", "code", "detail"].contains(name))
        .collect();
    undefined.sort_by(|left, right| name_order(left, right));
    if let Some(name) = undefined.first() {
        return Err(Defect::unknown(child(pointer, name)));
    }
    let code = read_text(
        member_of(members, "code", pointer)?,
        &child(pointer, "code"),
    )?;
    read_text(
        member_of(members, "cause", pointer)?,
        &child(pointer, "cause"),
    )?;
    let detail_pointer = child(pointer, "detail");
    if !member_of(members, "detail", pointer)?.is_object() {
        return Err(Defect::wrong_kind(detail_pointer));
    }
    Ok(Diagnostic {
        code,
        wire: value.clone(),
    })
}

#[ix_trace_rs::string_edge]
fn assemble(
    result: ResultValue,
    origin: EntryOrigin,
    certification: Option<Certification>,
    main: Main,
    rest: Rest,
) -> Result<ResultEntry, Defect> {
    // Steps 2 to 4 found the common members present, so a missing one here
    // is a reader defect; report it as the member's absence.
    let backend = rest
        .backend
        .ok_or_else(|| Defect::missing("/backend".to_owned()))?;
    let kind = rest
        .kind
        .ok_or_else(|| Defect::missing("/kind".to_owned()))?;
    let request_index = rest
        .request_index
        .ok_or_else(|| Defect::missing("/request_index".to_owned()))?;
    let index = RequestIndex::new(
        usize::try_from(request_index).map_err(|_| Defect::invalid("/request_index".to_owned()))?,
    );
    let diagnostic_wire = rest
        .diagnostic
        .as_ref()
        .map(|diagnostic| diagnostic.wire.clone());
    let mut limit_reached = None;
    let record = match (result, main) {
        (ResultValue::Proved, Main::Basis(basis)) => match origin {
            EntryOrigin::Plugin => {
                TerminalRecord::from_plugin_proof(index, backend.clone(), basis)?
            }
            EntryOrigin::FirstParty => TerminalRecord::with_backend(
                index,
                backend.clone(),
                TerminalValue::Proved {
                    basis,
                    certification: certification.unwrap_or(Certification::Uncertified),
                },
            ),
        },
        (ResultValue::Tested, _) => TerminalRecord::tested(index, backend.clone()),
        (ResultValue::Refuted, _) => {
            TerminalRecord::with_backend(index, backend.clone(), TerminalValue::Refuted)
        }
        (ResultValue::Inconclusive, Main::Cause(CauseValue::Inconclusive(cause))) => match origin {
            EntryOrigin::Plugin => {
                TerminalRecord::from_plugin_inconclusive(index, backend.clone(), cause)?
            }
            EntryOrigin::FirstParty => TerminalRecord::inconclusive(index, backend.clone(), cause),
        },
        (ResultValue::Incomplete, Main::Cause(CauseValue::Incomplete(cause, facts))) => {
            limit_reached = facts;
            TerminalRecord::incomplete(index, backend.clone(), cause)
        }
        (ResultValue::Unsupported, Main::Cause(CauseValue::Unsupported(cause))) => {
            TerminalRecord::unsupported(index, backend.clone(), cause)
        }
        (ResultValue::Unsupported, Main::None) => {
            let diagnostic = rest
                .diagnostic
                .as_ref()
                .ok_or_else(|| Defect::missing("/diagnostic".to_owned()))?;
            if diagnostic.code != "unsupported_projection" {
                return Err(Defect::unsupported(
                    "/diagnostic/code".to_owned(),
                    &diagnostic.code,
                ));
            }
            TerminalRecord::unsupported(index, backend.clone(), UnavailabilityCause::BackendAbsent)
        }
        (ResultValue::Declined, Main::Cause(CauseValue::Declined(cause))) => {
            let diagnostic = rest
                .diagnostic
                .as_ref()
                .ok_or_else(|| Defect::missing("/diagnostic".to_owned()))?;
            let code = match Code::from_code(&diagnostic.code) {
                Some(code) => DeclineCode::Qsl(code),
                None => DeclineCode::Std001(
                    Std001Code::new(&diagnostic.code)
                        .map_err(|_| Defect::invalid("/diagnostic/code".to_owned()))?,
                ),
            };
            TerminalRecord::declined(index, backend.clone(), cause, code)
        }
        (ResultValue::Failed, _) => TerminalRecord::failed(index, backend.clone()),
        // A `measured` entry's `verdict` is refused unsupported above, so no
        // measured value reaches here; any other pairing is a reader defect.
        (
            ResultValue::Measured
            | ResultValue::Proved
            | ResultValue::Inconclusive
            | ResultValue::Incomplete
            | ResultValue::Unsupported
            | ResultValue::Declined,
            _,
        ) => {
            return Err(Defect::invalid("/result".to_owned()));
        }
    };
    Ok(ResultEntry {
        request_index,
        backend,
        kind,
        record,
        certification,
        diagnostic: diagnostic_wire,
        limit_reached,
    })
}
