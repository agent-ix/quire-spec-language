// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-026–028, FR-031: closed records; existing constructors own identifier validity.

use qsl_foundation::serde_object::{deserialize_objects, from_object};
use qsl_foundation::AccountingSetting;
use quire_contract_model as ir;
use serde::Deserialize;
use serde_json::value::RawValue;

/// The selected request shape owns its admitted envelope format.
pub(super) trait RequestKind: serde::de::DeserializeOwned {
    const FORMAT: qsl_foundation::wire_format::WireFormat;
}

impl RequestKind for Request {
    const FORMAT: qsl_foundation::wire_format::WireFormat =
        qsl_foundation::wire_format::WireFormat::RunRequest;
}

impl RequestKind for CompileRequest {
    const FORMAT: qsl_foundation::wire_format::WireFormat =
        qsl_foundation::wire_format::WireFormat::CompileRequest;
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Envelope<'a> {
    pub format: String,
    #[serde(borrow)]
    pub request: &'a RawValue,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompileRequest {
    #[serde(deserialize_with = "deserialize_objects")]
    pub models: Vec<Model>,
    #[serde(deserialize_with = "from_object")]
    pub program: Program,
    /// FR-147/FR-099: the unit's unchecked internal-list value.
    #[serde(default, deserialize_with = "present_json_value")]
    pub refinement_internals: Option<serde_json::Value>,
    /// A `1-draft` program's supplied libraries (FR-027, ADR-015 D-1); none
    /// when absent.
    #[serde(default, deserialize_with = "deserialize_objects")]
    pub libraries: Vec<Library>,
}

/// One supplied library: its identity and source selection.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Library {
    pub identity: String,
    /// FR-147/FR-099: this library's unchecked internal-list value.
    #[serde(default, deserialize_with = "present_json_value")]
    pub refinement_internals: Option<serde_json::Value>,
    #[serde(deserialize_with = "from_object")]
    pub source: SourceFile,
}

fn present_json_value<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<serde_json::Value>, D::Error> {
    serde_json::Value::deserialize(decoder).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    #[serde(deserialize_with = "deserialize_objects")]
    pub models: Vec<Model>,
    #[serde(deserialize_with = "from_object")]
    pub program: Program,
    #[serde(default, deserialize_with = "from_object")]
    pub limits: Limits,
    /// FR-100: a `1-draft` program's named call. Absent for a `0-draft`
    /// program.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub call: Option<Call>,
    /// FR-312: a clause run, mutually exclusive with `call` for 1-draft.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub clause: Option<ClauseRun>,
    /// FR-100/ADR-015 D-1: a `1-draft` program's supplied libraries; none
    /// when absent.
    #[serde(default, deserialize_with = "deserialize_objects")]
    pub libraries: Vec<Library>,
}

fn from_object_opt<'de, D: serde::Deserializer<'de>, T: serde::Deserialize<'de>>(
    decoder: D,
) -> Result<Option<T>, D::Error> {
    from_object(decoder).map(Some)
}

/// FR-312's closed clause-run selection and observation file table.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ClauseRun {
    #[serde(deserialize_with = "from_object")]
    pub selection: qsl_replay::spine::ClauseRunSelectionWire,
    #[serde(deserialize_with = "from_object")]
    pub observations: std::collections::BTreeMap<String, String>,
    /// FR-312: optional clause-run accounting limits, using FR-100's closed
    /// counter object and defaults.
    #[serde(default, deserialize_with = "from_object")]
    pub accounting: Accounting,
    #[serde(default, deserialize_with = "from_object_opt")]
    pub observation_limits: Option<qsl_replay::spine::ObservationLimitsWire>,
}

/// The `call` member of a `1-draft` native-run/1 request (FR-100): which
/// function to call, its arguments, and an optional closed `accounting`
/// object. A top-level `work_units` is no member: `deny_unknown_fields`
/// refuses it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Call {
    pub function: String,
    #[serde(deserialize_with = "deserialize_objects")]
    pub arguments: Vec<Argument>,
    /// FR-100: the call's accounting limits by counter name; absent is `{}`.
    #[serde(default, deserialize_with = "from_object")]
    pub accounting: Accounting,
}

/// The `accounting` member of a [`Call`] (FR-100): any subset of the ten
/// `quire.value.accounting/v1` counters, each a JSON integer from 0 to
/// `u64::MAX`. A member that names no counter, or is named twice, refuses at
/// decode (serde's derive rejects both).
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Accounting {
    #[serde(default, deserialize_with = "counter")]
    integer_bits: Option<u64>,
    #[serde(default, deserialize_with = "counter")]
    decimal_digits: Option<u64>,
    #[serde(default, deserialize_with = "counter")]
    scale_expansion: Option<u64>,
    #[serde(default, deserialize_with = "counter")]
    text_input_bytes: Option<u64>,
    #[serde(default, deserialize_with = "counter")]
    text_scalars: Option<u64>,
    #[serde(default, deserialize_with = "counter")]
    normalized_scalars: Option<u64>,
    #[serde(default, deserialize_with = "counter")]
    unit_edges: Option<u64>,
    #[serde(default, deserialize_with = "counter")]
    value_occurrences: Option<u64>,
    #[serde(default, deserialize_with = "counter")]
    work_units: Option<u64>,
    #[serde(default, deserialize_with = "counter")]
    result_units: Option<u64>,
}

impl Accounting {
    /// The counters the object names, as their settings with their bounds.
    pub fn entries(&self) -> impl Iterator<Item = (AccountingSetting, u64)> {
        [
            (AccountingSetting::IntegerBits, self.integer_bits),
            (AccountingSetting::DecimalDigits, self.decimal_digits),
            (AccountingSetting::ScaleExpansion, self.scale_expansion),
            (AccountingSetting::TextInputBytes, self.text_input_bytes),
            (AccountingSetting::TextScalars, self.text_scalars),
            (
                AccountingSetting::NormalizedScalars,
                self.normalized_scalars,
            ),
            (AccountingSetting::UnitEdges, self.unit_edges),
            (AccountingSetting::ValueOccurrences, self.value_occurrences),
            (AccountingSetting::WorkUnits, self.work_units),
            (AccountingSetting::ResultUnits, self.result_units),
        ]
        .into_iter()
        .filter_map(|(setting, bound)| bound.map(|bound| (setting, bound)))
    }
}

/// A present counter is a JSON integer from 0 to `u64::MAX`. This
/// deserializes `u64` directly, so serde calls it only for a present key and
/// a present `null` fails `u64`'s own visitor rather than becoming absent.
fn counter<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<Option<u64>, D::Error> {
    u64::deserialize(decoder).map(Some)
}

/// One `{parameter, value}` argument of a [`Call`] (FR-100): `value` is a
/// JSON integer in the signed 64-bit range, read as `i64` so an out-of-range
/// or non-integer JSON value refuses at decode (`invalid-request`), never
/// reaching `qsl_replay::spine::run`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Argument {
    pub parameter: String,
    pub value: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceFile {
    pub file: String,
    #[serde(flatten)]
    pub identity: Identity,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Model {
    pub format: String,
    #[serde(deserialize_with = "from_object")]
    pub source: SourceFile,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Program {
    #[serde(deserialize_with = "from_object")]
    pub source: SourceFile,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Identity {
    pub authority: String,
    pub identity: String,
    pub document: String,
    pub formal_revision: u64,
}

impl Identity {
    pub fn bind(&self) -> Result<crate::formal_source::SourceIdentities, ir::Diagnostic> {
        Ok(crate::formal_source::SourceIdentities {
            native: qsl_foundation::SourceIdentity::new(
                self.authority.clone(),
                self.identity.clone(),
            ),
            formal: ir::SourceIdentity::new(
                ir::SourceDocumentId::new(&self.document)?,
                ir::SourceRevision::new(self.formal_revision)?,
            ),
        })
    }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct Limits {
    /// QSpec FR-323 `limits.supplied`: the supplied-value helper ceilings.
    /// Absent resolves to each published default; a present object is closed
    /// and complete with each ceiling an exact `u64`, or the request is
    /// refused before anything runs.
    #[serde(default, deserialize_with = "supplied_limits")]
    pub supplied: Option<quire_semantic_value::supplied_stop::SuppliedLimits>,
}

/// `limits.supplied`, read by the one strict reader the replay request also
/// uses ([`qsl_replay::read_supplied_limits`]).
fn supplied_limits<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<quire_semantic_value::supplied_stop::SuppliedLimits>, D::Error> {
    let value = serde_json::Value::deserialize(decoder)?;
    qsl_replay::read_supplied_limits(&value)
        .map(Some)
        .map_err(serde::de::Error::custom)
}
