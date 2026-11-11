// SPDX-License-Identifier: AGPL-3.0-or-later
//! The adverse mutation corpus of FR-518: each case is a package derived from
//! the written one by one mutation, with the refusal its table row declares.
//! The declared values are written here by hand. Before anything is written,
//! every case is submitted to the stage it names and must get exactly that
//! refusal.
//!
//! The mutations edit package bytes as JSON values; no verdict on a package is
//! made here, only by the reader and [`super::Selection::check`].

use quire_contract_model::{CheckedPackageDispatchResult, CheckedPackageEvidence};
use serde_json::{json, Value};

use super::selection::SelectionAxis;
use super::{Environment, Error, Selection, MUTATIONS_FORMAT, READ_LIMITS};

/// The stage a case is refused at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    /// By `read_checked_package`.
    Reader,
    /// Admitted by the reader, refused by [`Selection::check`].
    Selection,
}

/// The refusal the reader gives a `reader` case.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderRefusal {
    /// The refusal code, as the wire spells it.
    pub code: String,
    /// The cause, when the refusal has one.
    pub cause: Option<String>,
    /// The RFC 6901 pointer of the primary locus.
    pub pointer: String,
}

/// What a case is refused with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expected {
    /// A reader refusal.
    Reader(ReaderRefusal),
    /// A selection axis.
    Selection(SelectionAxis),
}

impl Expected {
    fn stage(&self) -> Stage {
        match self {
            Self::Reader(_) => Stage::Reader,
            Self::Selection(_) => Stage::Selection,
        }
    }

    fn to_json(&self) -> Value {
        match self {
            Self::Reader(refusal) => json!({
                "code": refusal.code,
                "cause": refusal.cause,
                "pointer": refusal.pointer,
            }),
            Self::Selection(axis) => json!({ "axis": axis.name() }),
        }
    }
}

fn reader(code: &str, cause: Option<&str>, pointer: String) -> Expected {
    Expected::Reader(ReaderRefusal {
        code: code.to_owned(),
        cause: cause.map(str::to_owned),
        pointer,
    })
}

fn timed(pointer: String) -> Expected {
    reader("invalid_package", Some("timed-node-reference"), pointer)
}

fn malformed(pointer: String) -> Expected {
    reader("malformed_wire", None, pointer)
}

/// One mutation of the base package and the refusal it must get.
struct Made {
    value: Value,
    expected: Expected,
}

/// One row of the corpus table.
struct Case {
    identity: &'static str,
    axis: &'static str,
    /// The clock kind of the package the case belongs to.
    home: &'static str,
    make: fn(&Value) -> Option<Made>,
}

const EVENT: &str = "event_position";
const FIXED: &str = "fixed_sample";
const STAMPED: &str = "timestamped_event";

fn bindings(v: &Value) -> Option<&Vec<Value>> {
    v.get("temporal_bindings")?.as_array()
}

fn nodes(v: &Value) -> Option<&Vec<Value>> {
    v.get("semantic_graph")?.as_array_nodes()
}

trait AsArrayNodes {
    fn as_array_nodes(&self) -> Option<&Vec<Value>>;
}

impl AsArrayNodes for Value {
    fn as_array_nodes(&self) -> Option<&Vec<Value>> {
        self.get("nodes")?.as_array()
    }
}

/// The ids of the `temporal_clause` nodes, in graph order. Binding rows also
/// name `compensation` nodes, so a row is not necessarily a clause.
fn clause_ids(v: &Value) -> Vec<Value> {
    nodes(v)
        .into_iter()
        .flatten()
        .filter(|node| node.get("semantic_form") == Some(&json!("temporal_clause")))
        .filter_map(|node| node.get("node_id").cloned())
        .collect()
}

/// The position of the first `temporal_clause` node.
fn first_clause_node(v: &Value) -> Option<usize> {
    nodes(v)?
        .iter()
        .position(|node| node.get("semantic_form") == Some(&json!("temporal_clause")))
}

fn row_mut(v: &mut Value, row: usize) -> Option<&mut Value> {
    v.get_mut("temporal_bindings")?.as_array_mut()?.get_mut(row)
}

fn clock_mut(v: &mut Value, row: usize) -> Option<&mut serde_json::Map<String, Value>> {
    row_mut(v, row)?.get_mut("clock")?.as_object_mut()
}

fn selection_mut<'a>(v: &'a mut Value, key: &str) -> Option<&'a mut Vec<Value>> {
    v.get_mut("lock")?.get_mut(key)?.as_array_mut()
}

fn ok(value: Value, expected: Expected) -> Option<Made> {
    Some(Made { value, expected })
}

fn clone(v: &Value) -> Value {
    v.clone()
}

/// Rewrite every occurrence of the node id `from` as `to`.
fn replace_id(v: &mut Value, from: &Value, to: &Value) {
    if v == from {
        *v = to.clone();
        return;
    }
    match v {
        Value::Array(items) => items.iter_mut().for_each(|item| replace_id(item, from, to)),
        Value::Object(members) => members
            .values_mut()
            .for_each(|member| replace_id(member, from, to)),
        _ => {}
    }
}

fn header_wire_version(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    *w.get_mut("contract_version")? = json!("quire.checked-package/v1");
    ok(
        w,
        reader("unknown_contract_version", None, "/contract_version".into()),
    )
}

fn binding_missing(v: &Value) -> Option<Made> {
    let clause = clause_ids(v).first()?.clone();
    let row = bindings(v)?
        .iter()
        .position(|row| row.get("declaration") == Some(&clause))?;
    let node = nodes(v)?
        .iter()
        .position(|node| node.get("node_id") == Some(&clause))?;
    let mut w = clone(v);
    w.get_mut("temporal_bindings")?.as_array_mut()?.remove(row);
    ok(w, timed(format!("/semantic_graph/nodes/{node}/body")))
}

fn binding_surplus(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    let at = bindings(v)?.len();
    let mut row = bindings(v)?.first()?.clone();
    let ordinal = row.get("declaration")?.get("ordinal")?.as_u64()?;
    *row.get_mut("declaration")?.get_mut("ordinal")? = json!(ordinal + 1000);
    w.get_mut("temporal_bindings")?.as_array_mut()?.push(row);
    ok(w, timed(format!("/temporal_bindings/{at}/declaration")))
}

fn binding_duplicate(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    let at = bindings(v)?.len();
    let row = bindings(v)?.first()?.clone();
    w.get_mut("temporal_bindings")?.as_array_mut()?.push(row);
    ok(
        w,
        reader(
            "invalid_package",
            Some("conflicting-definition"),
            format!("/temporal_bindings/{at}"),
        ),
    )
}

fn binding_reordered(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    bindings(v)?.get(1)?;
    w.get_mut("temporal_bindings")?.as_array_mut()?.swap(0, 1);
    ok(w, timed("/temporal_bindings/1/declaration".into()))
}

fn declaration_index(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    let other = nodes(v)?
        .iter()
        .find(|node| node.get("semantic_form") != Some(&json!("temporal_clause")))?
        .get("node_id")?
        .clone();
    *row_mut(&mut w, 0)?.get_mut("declaration")? = other;
    ok(w, timed("/temporal_bindings/0/declaration".into()))
}

fn definition_index(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    selection_mut(&mut w, "profile_selections")?
        .retain(|selection| selection.get("role") != Some(&json!("temporal_profile")));
    let node = nodes(v)?.iter().position(|node| {
        node.get("node_tag") == Some(&json!("protocol"))
            && matches!(
                node.pointer("/body/operation/member/operator")
                    .and_then(Value::as_str),
                Some("await" | "compensate")
            )
    })?;
    ok(
        w,
        reader(
            "missing_declaration",
            Some("missing-selection"),
            format!("/semantic_graph/nodes/{node}"),
        ),
    )
}

fn declaration_name(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    let node = first_clause_node(v)?;
    w.get_mut("semantic_graph")?
        .get_mut("nodes")?
        .get_mut(node)?
        .as_object_mut()?
        .insert("declaration".into(), json!({"qualified_name": ["Renamed"]}));
    ok(
        w,
        reader(
            "invalid_semantic_graph",
            None,
            format!("/semantic_graph/nodes/{node}/declaration"),
        ),
    )
}

fn declaration_requirement(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    let index = nodes(v)?
        .iter()
        .position(|node| node.get("semantic_form") == Some(&json!("protocol_clause")))?;
    let protocol = nodes(v)?.get(index)?.to_string();
    let ids = clause_ids(v);
    let from = ids
        .iter()
        .find(|id| protocol.contains(&id.to_string()))?
        .clone();
    let to = ids.iter().find(|id| **id != from)?.clone();
    let node = w
        .get_mut("semantic_graph")?
        .get_mut("nodes")?
        .get_mut(index)?;
    replace_id(node, &from, &to);
    ok(w, Expected::Selection(SelectionAxis::ClauseRequirement))
}

fn declaration_clause(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    let from = clause_ids(v).first()?.clone();
    let mut to = from.clone();
    *to.get_mut("ordinal")? = json!(to.get("ordinal")?.as_u64()? + 1);
    replace_id(&mut w, &from, &to);
    ok(w, Expected::Selection(SelectionAxis::ClauseNode))
}

fn declaration_execution(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    let node = first_clause_node(v)?;
    let members = w
        .get_mut("semantic_graph")?
        .get_mut("nodes")?
        .get_mut(node)?
        .get_mut("body")?
        .get_mut("arguments")?
        .get_mut(2)?
        .get_mut("members")?
        .as_array_mut()?;
    let kind = members
        .iter_mut()
        .find(|member| member.get("name") == Some(&json!("kind")))?;
    *kind.get_mut("value")?.get_mut("value")? = json!("each");
    ok(w, Expected::Selection(SelectionAxis::ClauseActivation))
}

/// Whether a node is a temporal clause or a compensation template: the two
/// forms whose application carries the temporal profile law.
#[ix_trace_rs::string_edge]
fn carries_profile_law(node: &Value) -> bool {
    let temporal_law = node
        .pointer("/body/operation/laws/0/role")
        .is_some_and(|role| role == "temporal_profile");
    temporal_law
        && ["temporal_clause", "compensation"]
            .iter()
            .any(|form| node.get("semantic_form") == Some(&json!(form)))
}

/// Set the identity, revision or authority of the profile law of every
/// clause and of the lock's matching selection, so they stay consistent.
fn profile_law(v: &Value, value: &str, axis: SelectionAxis) -> Option<Made> {
    let mut w = clone(v);
    let mut rewritten = None;
    for node in w
        .get_mut("semantic_graph")?
        .get_mut("nodes")?
        .as_array_mut()?
    {
        if !carries_profile_law(node) {
            continue;
        }
        let definition = node
            .get_mut("body")?
            .get_mut("operation")?
            .get_mut("laws")?
            .get_mut(0)?
            .get_mut("definition")?;
        rewritten = Some(rewrite_definition(definition, axis, value)?);
    }
    let rewritten = rewritten?;
    if axis != SelectionAxis::DefinitionAuthority {
        // A compensation row of an unregistered profile is refused at its row
        // before the law is classified, so the compensation rows go too.
        let compensations: Vec<Value> = nodes(&w)?
            .iter()
            .filter(|node| node.get("semantic_form") == Some(&json!("compensation")))
            .filter_map(|node| node.get("node_id").cloned())
            .collect();
        w.get_mut("temporal_bindings")?
            .as_array_mut()?
            .retain(|row| {
                row.get("declaration")
                    .is_none_or(|id| !compensations.contains(id))
            });
    }
    let first = first_clause_node(v)?;
    let expected = match axis {
        SelectionAxis::DefinitionAuthority => Expected::Selection(axis),
        _ => reader(
            "unknown_profile",
            Some("unsupported-selection"),
            format!("/semantic_graph/nodes/{first}/body/operation/laws/0/definition"),
        ),
    };
    let selection = selection_mut(&mut w, "profile_selections")?
        .iter_mut()
        .find(|selection| selection.get("role") == Some(&json!("temporal_profile")))?
        .get_mut("definition")?;
    *selection = rewritten;
    ok(w, expected)
}

/// `definition` with the member `axis` names -- its revision for
/// [`SelectionAxis::DefinitionRevision`], its authority for
/// [`SelectionAxis::DefinitionAuthority`] -- changed to `value`; `None` for
/// any other axis.
fn rewrite_definition(definition: &mut Value, axis: SelectionAxis, value: &str) -> Option<Value> {
    let identity = definition.get("identity")?.as_str()?.to_owned();
    let (name, _revision) = identity.rsplit_once('/')?;
    if axis == SelectionAxis::DefinitionRevision {
        *definition.get_mut("identity")? = json!(format!("{name}/{value}"));
    } else if axis == SelectionAxis::DefinitionAuthority {
        *definition.get_mut("authority")? = json!(value);
    } else {
        return None;
    }
    Some(definition.clone())
}

/// Row 12: the profile law of every clause, the lock's `temporal_profile`
/// selection and every binding row move together to another registered
/// profile, which the reader admits; only the sidecar names the first one.
fn definition_identity(v: &Value) -> Option<Made> {
    let other = "quire.temporal.fixed-sample.false-extension/v1";
    let mut w = clone(v);
    for node in w
        .get_mut("semantic_graph")?
        .get_mut("nodes")?
        .as_array_mut()?
    {
        if node.get("semantic_form") == Some(&json!("temporal_clause")) {
            *node
                .get_mut("body")?
                .get_mut("operation")?
                .get_mut("laws")?
                .get_mut(0)?
                .get_mut("definition")?
                .get_mut("identity")? = json!(other);
        }
    }
    *selection_mut(&mut w, "profile_selections")?
        .iter_mut()
        .find(|selection| selection.get("role") == Some(&json!("temporal_profile")))?
        .get_mut("definition")?
        .get_mut("identity")? = json!(other);
    for row in w.get_mut("temporal_bindings")?.as_array_mut()? {
        *row.get_mut("clock")? = json!({
            "kind": "fixed_sample",
            "epoch": {"kind": "integer", "decimal": "0"},
            "period": {"kind": "integer", "decimal": "1"},
            "unit": "milliseconds",
        });
    }
    ok(w, Expected::Selection(SelectionAxis::DefinitionIdentity))
}

/// Row 13: Contract IR registers one revision of each profile (`/v1`), so a
/// changed revision is an unknown profile; `definition.revision` is compared by
/// `Selection::check` but no admitted package can differ on it.
fn definition_revision(v: &Value) -> Option<Made> {
    profile_law(v, "v2", SelectionAxis::DefinitionRevision)
}

fn definition_authority(v: &Value) -> Option<Made> {
    profile_law(v, "other-authority", SelectionAxis::DefinitionAuthority)
}

fn closed_shape(v: &Value, member: &str) -> Option<Made> {
    let mut w = clone(v);
    w.get_mut("lock")?
        .get_mut("definition_selections")?
        .get_mut(0)?
        .as_object_mut()?
        .insert(member.to_owned(), json!("x"));
    ok(
        w,
        reader(
            "unknown_member",
            None,
            format!("/lock/definition_selections/0/{member}"),
        ),
    )
}

fn digest_member(v: &Value) -> Option<Made> {
    closed_shape(v, "digest")
}

fn digest_domain_member(v: &Value) -> Option<Made> {
    closed_shape(v, "digest_domain")
}

fn clock_alternative(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    *row_mut(&mut w, 0)?.get_mut("clock")? = json!({
        "kind": "timestamped_event", "timestamp_unit": "other-unit"
    });
    ok(w, timed("/temporal_bindings/0/clock/kind".into()))
}

fn set_clock(v: &Value, member: &str, value: Value, axis: SelectionAxis) -> Option<Made> {
    let mut w = clone(v);
    *clock_mut(&mut w, 0)?.get_mut(member)? = value;
    ok(w, Expected::Selection(axis))
}

fn clock_sequence_authority(v: &Value) -> Option<Made> {
    set_clock(
        v,
        "sequence_authority",
        json!("other-authority"),
        SelectionAxis::ClockSequenceAuthority,
    )
}

fn clock_epoch(v: &Value) -> Option<Made> {
    set_clock(
        v,
        "epoch",
        json!({"kind": "integer", "decimal": "7"}),
        SelectionAxis::ClockEpoch,
    )
}

fn clock_period(v: &Value) -> Option<Made> {
    set_clock(
        v,
        "period",
        json!({"kind": "rational", "numerator": "1", "denominator": "3"}),
        SelectionAxis::ClockPeriod,
    )
}

fn clock_unit(v: &Value) -> Option<Made> {
    set_clock(v, "unit", json!("other-unit"), SelectionAxis::ClockUnit)
}

fn clock_timestamp_unit(v: &Value) -> Option<Made> {
    set_clock(
        v,
        "timestamp_unit",
        json!("other-unit"),
        SelectionAxis::ClockTimestampUnit,
    )
}

fn remove_member(v: &Value, member: &str) -> Option<Made> {
    let mut w = clone(v);
    clock_mut(&mut w, 0)?.remove(member)?;
    ok(w, malformed("/temporal_bindings/0/clock".into()))
}

fn rename_member(v: &Value, member: &str, to: &str) -> Option<Made> {
    let mut w = clone(v);
    let clock = clock_mut(&mut w, 0)?;
    let value = clock.remove(member)?;
    clock.insert(to.to_owned(), value);
    // The renamed member is outside the closed clock schema, so Contract IR
    // refuses it as an unknown member at the member itself (AD-004).
    ok(
        w,
        reader(
            "unknown_member",
            None,
            format!("/temporal_bindings/0/clock/{to}"),
        ),
    )
}

fn missing_sequence_authority(v: &Value) -> Option<Made> {
    remove_member(v, "sequence_authority")
}

fn renamed_sequence_authority(v: &Value) -> Option<Made> {
    rename_member(v, "sequence_authority", "sequenceAuthority")
}

fn missing_period(v: &Value) -> Option<Made> {
    remove_member(v, "period")
}

fn renamed_period(v: &Value) -> Option<Made> {
    rename_member(v, "period", "sample_period")
}

fn missing_timestamp_unit(v: &Value) -> Option<Made> {
    remove_member(v, "timestamp_unit")
}

fn renamed_timestamp_unit(v: &Value) -> Option<Made> {
    rename_member(v, "timestamp_unit", "timestampUnit")
}

fn clock_name_differs(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    *row_mut(&mut w, 0)?.get_mut("clock_name")? = json!("other-clock");
    ok(w, timed("/temporal_bindings/0/clock_name".into()))
}

fn clock_name_empty(v: &Value) -> Option<Made> {
    let mut w = clone(v);
    *row_mut(&mut w, 0)?.get_mut("clock_name")? = json!("");
    ok(w, malformed("/temporal_bindings/0/clock_name".into()))
}

/// The corpus table, in table order.
fn table() -> Vec<Case> {
    let c = |identity, axis, home, make| Case {
        identity,
        axis,
        home,
        make,
    };
    vec![
        c(
            "header-wire-version-substitution",
            "header.version",
            EVENT,
            header_wire_version,
        ),
        c(
            "binding-missing",
            "temporal_bindings.inventory",
            EVENT,
            binding_missing,
        ),
        c(
            "binding-surplus",
            "temporal_bindings.inventory",
            EVENT,
            binding_surplus,
        ),
        c(
            "binding-duplicate",
            "temporal_bindings.uniqueness",
            EVENT,
            binding_duplicate,
        ),
        c(
            "binding-reordered",
            "temporal_bindings.order",
            EVENT,
            binding_reordered,
        ),
        c(
            "declaration-index",
            "temporal_binding.declaration",
            EVENT,
            declaration_index,
        ),
        c(
            "definition-index",
            "temporal_binding.definition",
            EVENT,
            definition_index,
        ),
        c(
            "declaration-name",
            "declaration.name",
            EVENT,
            declaration_name,
        ),
        c(
            "declaration-requirement",
            "declaration.requirement",
            EVENT,
            declaration_requirement,
        ),
        c(
            "declaration-clause",
            "declaration.clause",
            EVENT,
            declaration_clause,
        ),
        c(
            "declaration-execution",
            "declaration.execution",
            EVENT,
            declaration_execution,
        ),
        c(
            "definition-identity",
            "definition.identity",
            EVENT,
            definition_identity,
        ),
        c(
            "definition-revision",
            "definition.revision",
            EVENT,
            definition_revision,
        ),
        c(
            "definition-artifact-digest-closed-shape",
            "definition_selections.digest",
            EVENT,
            digest_member,
        ),
        c(
            "definition-artifact-digest-domain-closed-shape",
            "definition_selections.digest_domain",
            EVENT,
            digest_domain_member,
        ),
        c("clock-alternative", "clock.kind", EVENT, clock_alternative),
        c(
            "clock-sequence-authority",
            "clock.sequence_authority",
            EVENT,
            clock_sequence_authority,
        ),
        c("clock-epoch", "clock.epoch", FIXED, clock_epoch),
        c("clock-period", "clock.period", FIXED, clock_period),
        c("clock-unit", "clock.unit", FIXED, clock_unit),
        c(
            "clock-timestamp-unit",
            "clock.timestamp_unit",
            STAMPED,
            clock_timestamp_unit,
        ),
        c(
            "clock-missing-sequence-authority",
            "clock.sequence_authority",
            EVENT,
            missing_sequence_authority,
        ),
        c(
            "clock-renamed-sequence-authority",
            "clock.sequence_authority",
            EVENT,
            renamed_sequence_authority,
        ),
        c(
            "clock-missing-period",
            "clock.period",
            FIXED,
            missing_period,
        ),
        c(
            "clock-renamed-period",
            "clock.period",
            FIXED,
            renamed_period,
        ),
        c(
            "clock-missing-timestamp-unit",
            "clock.timestamp_unit",
            STAMPED,
            missing_timestamp_unit,
        ),
        c(
            "clock-renamed-timestamp-unit",
            "clock.timestamp_unit",
            STAMPED,
            renamed_timestamp_unit,
        ),
        c(
            "definition-artifact-wire-identity",
            "definition.authority",
            EVENT,
            definition_authority,
        ),
        c(
            "clock-name",
            "temporal_bindings.clock_name",
            EVENT,
            clock_name_differs,
        ),
        c(
            "clock-name-empty",
            "temporal_bindings.clock_name",
            EVENT,
            clock_name_empty,
        ),
    ]
}

/// The cases of one package and their files.
pub(super) struct Cases {
    manifest: Value,
    files: Vec<(String, Vec<u8>)>,
}

impl Cases {
    /// Every file under `mutations/`.
    pub(super) fn files(&self) -> Result<Vec<(String, Vec<u8>)>, Error> {
        let mut manifest =
            serde_json::to_vec_pretty(&self.manifest).map_err(|source| Error::Encode {
                what: "mutations/manifest.json",
                source,
            })?;
        manifest.push(b'\n');
        let mut files = vec![("mutations/manifest.json".to_owned(), manifest)];
        files.extend(self.files.iter().cloned());
        Ok(files)
    }
}

fn mismatch(identity: &str) -> Error {
    Error::CaseMismatch {
        identity: identity.to_owned(),
    }
}

/// The refusal the reader gave, as a table value.
fn refusal_of(result: &CheckedPackageDispatchResult) -> Option<ReaderRefusal> {
    let CheckedPackageDispatchResult::Refused(refusal) = result else {
        return None;
    };
    let spelled = |value: &dyn erased::Spell| value.spell();
    Some(ReaderRefusal {
        code: spelled(&refusal.code),
        cause: refusal.cause.as_ref().map(|cause| spelled(cause)),
        pointer: refusal
            .loci
            .first()
            .and_then(|locus| locus.path.as_ref())
            .map(ToString::to_string)
            .unwrap_or_default(),
    })
}

mod erased {
    /// A refusal code or cause spelled as the wire spells it.
    pub(super) trait Spell {
        fn spell(&self) -> String;
    }

    impl<T: serde::Serialize> Spell for T {
        fn spell(&self) -> String {
            serde_json::to_value(self)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_default()
        }
    }
}

/// Build, verify and encode the cases that belong to `base`.
pub(super) fn build(
    base: &[u8],
    selection: &Selection,
    evidence: &CheckedPackageEvidence,
    environment: &dyn Environment,
) -> Result<Cases, Error> {
    build_cases(table(), base, selection, evidence, environment)
}

/// [`build`] over `cases`, the rows of one table.
fn build_cases(
    cases: Vec<Case>,
    base: &[u8],
    selection: &Selection,
    evidence: &CheckedPackageEvidence,
    environment: &dyn Environment,
) -> Result<Cases, Error> {
    let mut value: Value = serde_json::from_slice(base).map_err(|source| Error::Encode {
        what: "checked-package.json",
        source,
    })?;
    // FR-322 "Operand index segments": the cases are authored against the
    // resolved form, whose node ids name graph positions, and written back
    // with operand slots and rows.
    qsl_semantics::check::wire_id::resolve_operand_slots(&mut value)
        .map_err(|detail| Error::ReadBackDiffers { detail })?;
    let home = bindings(&value)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("clock")?.get("kind")?.as_str())
        .unwrap_or_default()
        .to_owned();
    let mut manifest_cases = Vec::new();
    let mut files = Vec::new();
    for case in cases.into_iter().filter(|case| case.home == home) {
        let made = (case.make)(&value).ok_or_else(|| mismatch(case.identity))?;
        let mut slotted = made.value.clone();
        qsl_semantics::check::wire_id::write_operand_rows(&mut slotted);
        let bytes = serde_json::to_vec(&slotted).map_err(|source| Error::Encode {
            what: "mutations case",
            source,
        })?;
        verify(&case, &made, &bytes, base, selection, evidence, environment)?;
        let file = format!("mutations/{}.json", case.identity);
        manifest_cases.push(json!({
            "identity": case.identity,
            "axis": case.axis,
            "input": { "file": file },
            "stage": match made.expected.stage() {
                Stage::Reader => "reader",
                Stage::Selection => "selection",
            },
            "expected": made.expected.to_json(),
        }));
        files.push((file, bytes));
    }
    Ok(Cases {
        manifest: json!({
            "format": MUTATIONS_FORMAT,
            "base_package": "checked-package.json",
            "selection": "selection.json",
            "cases": manifest_cases,
        }),
        files,
    })
}

/// Submit `case` to the stage it names; it must get exactly its refusal.
fn verify(
    case: &Case,
    made: &Made,
    bytes: &[u8],
    base: &[u8],
    selection: &Selection,
    evidence: &CheckedPackageEvidence,
    environment: &dyn Environment,
) -> Result<(), Error> {
    if bytes == base {
        return Err(mismatch(case.identity));
    }
    let result = environment.read(bytes, READ_LIMITS, evidence);
    match &made.expected {
        Expected::Reader(declared) => {
            if refusal_of(&result).as_ref() == Some(declared) {
                Ok(())
            } else {
                Err(mismatch(case.identity))
            }
        }
        Expected::Selection(axis) => {
            let CheckedPackageDispatchResult::AdmittedV2(package) = result else {
                return Err(mismatch(case.identity));
            };
            match environment.check(selection, &package) {
                Err(refusal) if refusal.axis == *axis => Ok(()),
                _ => Err(mismatch(case.identity)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use std::path::Path;

    use quire_contract_model::{
        CheckedPackageDispatchResult, CheckedPackageEvidence, CheckedPackageReadLimits,
        CheckedPackageRefusal, CheckedPackageRefusalCode, CheckedPackageV2,
    };
    use serde_json::{json, Value};

    use super::{build_cases, clone, ok, reader, Case, Made, EVENT};
    use crate::checked_handoff::{
        Environment, Error, Selection, SelectionRefusal, SELECTION_FORMAT,
    };

    /// A case whose mutation leaves the base bytes as they are.
    fn unchanged(v: &Value) -> Option<Made> {
        ok(clone(v), reader("invalid_package", None, String::new()))
    }

    /// A reader that gives every package exactly the refusal `unchanged`
    /// declares, so only the unchanged-bytes guard can fail the case.
    struct Agreeing;

    impl Environment for Agreeing {
        fn read(
            &self,
            _bytes: &[u8],
            _limits: CheckedPackageReadLimits,
            _evidence: &CheckedPackageEvidence,
        ) -> CheckedPackageDispatchResult {
            CheckedPackageDispatchResult::Refused(CheckedPackageRefusal {
                code: CheckedPackageRefusalCode::InvalidPackage,
                loci: vec![],
                cause: None,
                locus: None,
                contract_version: None,
                document_pointer: None,
                extension_detail: None,
                protocol_detail: None,
                terminal_duplicate: None,
            })
        }

        fn check(
            &self,
            _selection: &Selection,
            _package: &CheckedPackageV2,
        ) -> Result<(), SelectionRefusal> {
            Ok(())
        }

        fn write_file(&self, _index: usize, _path: &Path, _contents: &[u8]) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn empty_selection() -> Selection {
        let limits = json!({
            "bytes": 0, "nodes": 0, "edges": 0,
            "occurrences": 0, "diagnostics": 0, "work": 0,
        });
        let sidecar = json!({
            "format": SELECTION_FORMAT,
            "package": {
                "contract_version": "",
                "read_limits": limits,
                "evidence": { "models": [], "libraries": [] },
                "node_counts": {},
            },
            "clauses": [],
            "compensations": [],
        });
        Selection::read(sidecar.to_string().as_bytes()).expect("a well-formed sidecar")
    }

    /// FR-518-AC-5: a case whose bytes equal the base is a `CaseMismatch`
    /// naming it, before any stage sees it.
    #[trace("FR-518-AC-5", "TC-1313")]
    #[test]
    fn a_case_that_leaves_the_base_bytes_unchanged_is_a_mismatch() {
        let base = json!({ "semantic_graph": { "nodes": [], "operand_rows": [] }, "temporal_bindings": [{ "clock": { "kind": EVENT } }] });
        let base_bytes = serde_json::to_vec(&base).expect("the base encodes");
        let cases = vec![Case {
            identity: "unchanged-case",
            axis: "none",
            home: EVENT,
            make: unchanged,
        }];
        let result = build_cases(
            cases,
            &base_bytes,
            &empty_selection(),
            &CheckedPackageEvidence::default(),
            &Agreeing,
        );
        match result {
            Err(Error::CaseMismatch { identity }) => assert_eq!(identity, "unchanged-case"),
            other => panic!("expected CaseMismatch, got {:?}", other.map(|_| ())),
        }
    }
}
