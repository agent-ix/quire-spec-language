// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-620 and FR-621 (FSpec FR-520): the items of a `native-analyze/1`
//! request document and the `analyze` result document.
//!
//! A document item names its claim by declared name and never carries the
//! members FSL computes (`request_index`, `extent`, `candidates`, the
//! resolved `temporal.fairness`). [`resolve_items`] resolves each item
//! against the checked package into the [`ClaimItem`] `analyze` takes, or
//! settles it on its own (`unknown-claim`, `absent-kind`). The caller
//! negotiates the resolved items ([`Negotiate`]); [`analyze_document`] then
//! runs them and writes the FR-621 document, each item echoing its admitted
//! FR-331 request item ([`ClaimItem::request_item_wire`]) so it can be
//! requested again exactly.

use std::collections::{BTreeMap, BTreeSet};
use std::num::{NonZeroU32, NonZeroU64};

use filament_checked::key::DeclarationKey;
use filament_exact::Cancel;
use filament_semantic_value::catalog::{CatalogCode, Category};
use fsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef};
use fsl_foundation::{BackendId, RequestIndex};
use fsl_package::CheckedPackage;
use fsl_replay::{OutcomeAxes, TerminalRecord, TerminalValue};
use fsl_route::request::{ExtentClassification, ItemKind};
use fsl_route::routing::Disposition;
use fsl_route::{CandidateOutcome, EvidenceKind, ItemEvidence, Registry};
use fsl_semantics::check::Capability;
use fsl_semantics::model::observation::{DocumentProvision, DocumentRef};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::request::ProtocolAnalysisSelection;
use super::{
    AnalyzeLimits, ClaimItem, Diagnostic, Engines, ItemForm, ItemOptions, NegotiatedItem,
    SourceProvision, SubjectBinding, TemporalItem,
};
use fsl_eval::simulation::model::PopulationUniverse;

/// The request document format FR-620 reads.
pub const REQUEST_FORMAT: &str = "native-analyze/1";

fn present_json_value<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<Value>, D::Error> {
    Value::deserialize(decoder).map(Some)
}

/// A `{authority, identity}` reference to a provisioned document.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct DocumentSelection {
    /// The document's declared authority.
    pub authority: String,
    /// The document's declared identity.
    pub identity: String,
}

/// One activated protocol role population supplied with a recording request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActivatedRolePopulation {
    /// The checked protocol role whose activated members are listed.
    pub role: String,
    /// The protocol activation instance, when the protocol is `on each`.
    #[serde(default, deserialize_with = "present_json_value")]
    pub instance: Option<Value>,
    /// The activated role members.
    pub members: Vec<ActivatedRoleMember>,
}

/// One authority-scoped identity in an activated role population.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct ActivatedRoleMember {
    /// The member reference authority.
    pub authority: String,
    /// The member reference identity.
    pub identity: String,
}

impl DocumentSelection {
    /// The document reference this selection names.
    pub fn reference(&self) -> DocumentRef {
        DocumentRef {
            authority: self.authority.clone(),
            identity: self.identity.clone(),
        }
    }

    fn of(reference: &DocumentRef) -> Self {
        Self {
            authority: reference.authority.clone(),
            identity: reference.identity.clone(),
        }
    }

    fn wire(&self) -> Value {
        json!({"authority": self.authority, "identity": self.identity})
    }
}

/// FSpec FR-520's model subject `{initial_states, universes, proof_bounds}`:
/// the snapshots the subject starts from and the snapshots whose complete
/// populations are its universes.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SubjectSelection {
    /// The initial-state snapshots, in request order.
    #[serde(default)]
    pub initial_states: Vec<DocumentSelection>,
    /// The snapshots whose complete populations are the universes.
    #[serde(default)]
    pub universes: Vec<DocumentSelection>,
    /// The substituted proof bounds; FSL reads none yet from a document.
    #[serde(default)]
    pub proof_bounds: Vec<Value>,
    /// FSL FR-1033: the protocol the subject binds, when the claim is decided
    /// over a protocol subject rather than the model subject.
    #[serde(default)]
    pub protocol: Option<ProtocolSelection>,
    /// The request's `memory` member (FR-1033, FR-219), carried with the
    /// subject because it resolves the memory model of the protocol subject's
    /// outermost `parallel`s; never read from a subject document.
    #[serde(skip)]
    pub memory: Vec<fsl_eval::simulation::protocol::MemorySelection>,
}

/// FSL FR-1033: an `over` binding, FSpec FR-433's `{reference}` spelling.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OverSelection {
    /// The object reference.
    pub reference: ReferenceSelection,
}

/// The population identity and key an `over` binding names.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSelection {
    /// The population identity.
    pub population: String,
    /// The object key.
    pub key: String,
}

/// A role's `{type, selection}` binding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RoleSelection {
    /// The binding's type, `model-binding`.
    #[serde(rename = "type")]
    pub kind: String,
    /// The domain package identity bound to the role.
    pub selection: String,
}

/// FSL FR-1033: the `protocol` member of a subject.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProtocolSelection {
    /// The protocol clause's qualified name.
    pub protocol: String,
    /// The `over` binding; the item settles `declined` when absent.
    #[serde(default)]
    pub over: Option<OverSelection>,
    /// Each static role's binding.
    #[serde(default)]
    pub roles: BTreeMap<String, RoleSelection>,
    /// Each symbolic capacity's decimal value.
    #[serde(default)]
    pub capacities: BTreeMap<String, String>,
}

impl ProtocolSelection {
    /// The binding this selection states, over the request's `memory`; `None`
    /// when a capacity is no decimal count.
    pub(super) fn binding(
        &self,
        package: &CheckedPackage,
        memory: &[fsl_eval::simulation::protocol::MemorySelection],
    ) -> Option<super::request::ProtocolBinding> {
        // The selections naming this protocol's `parallel`s; another protocol's
        // change nothing here (FR-1033 "Reading `memory`").
        let memory: Vec<_> = memory
            .iter()
            .filter(|entry| entry.parallel.split("::").next() == Some(self.protocol.as_str()))
            .cloned()
            .collect();
        let memory = memory.as_slice();
        let capacities = self
            .capacities
            .iter()
            .map(|(name, value)| {
                Some(fsl_eval::model_subject::CapacityBinding {
                    name: name.clone(),
                    value: value.parse().ok()?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let effective = package
            .graph()
            .protocol(&self.protocol)
            .and_then(|checked| {
                fsl_eval::simulation::protocol::ResolvedMemory::resolve(checked, memory).ok()
            })
            .map(|resolved| {
                resolved
                    .parallels()
                    .iter()
                    .map(|parallel| (parallel.parallel.clone(), parallel.model))
                    .collect()
            })
            .unwrap_or_default();
        Some(super::request::ProtocolBinding {
            protocol: self.protocol.clone(),
            over: self.over.as_ref().map(|over| {
                (
                    over.reference.population.clone(),
                    over.reference.key.clone(),
                )
            }),
            roles: self
                .roles
                .iter()
                .map(|(role, binding)| fsl_eval::model_subject::RoleBinding {
                    role: role.clone(),
                    model: binding.selection.clone(),
                })
                .collect(),
            capacities,
            memory: memory.to_vec(),
            effective,
        })
    }

    fn wire(&self) -> Value {
        let mut member = Map::new();
        member.insert("protocol".to_owned(), json!(self.protocol));
        if let Some(over) = &self.over {
            member.insert(
                "over".to_owned(),
                json!({"reference": {"population": over.reference.population,
                    "key": over.reference.key}}),
            );
        }
        member.insert(
            "roles".to_owned(),
            Value::Object(
                self.roles
                    .iter()
                    .map(|(role, binding)| {
                        (
                            role.clone(),
                            json!({"type": binding.kind, "selection": binding.selection}),
                        )
                    })
                    .collect(),
            ),
        );
        member.insert("capacities".to_owned(), json!(self.capacities));
        Value::Object(member)
    }
}

impl SubjectSelection {
    /// FR-620's default subject: every provisioned `initialization`
    /// snapshot, in ascending `{authority, identity}` order, as both the
    /// initial states and the universes, with no proof bound.
    pub fn default_over(documents: &DocumentProvision) -> Self {
        let initial: Vec<DocumentSelection> = documents
            .iter()
            .filter(|(_, bytes)| snapshot(bytes).is_some_and(|read| read.initialization()))
            .map(|(reference, _)| DocumentSelection::of(reference))
            .collect();
        Self {
            universes: initial.clone(),
            initial_states: initial,
            proof_bounds: Vec::new(),
            protocol: None,
            memory: Vec::new(),
        }
    }

    /// FR-620: every `universes` entry names a provisioned snapshot, and
    /// every population those snapshots hold is complete, because a universe
    /// is a complete population (FSpec FR-331).
    ///
    /// # Errors
    ///
    /// [`UniverseRefusal::Unprovisioned`] naming the first entry no
    /// provisioned snapshot answers, and [`UniverseRefusal::Incomplete`]
    /// naming the first population a referenced snapshot marks incomplete.
    pub fn check_universes(&self, documents: &DocumentProvision) -> Result<(), UniverseRefusal> {
        for (position, selection) in self.universes.iter().enumerate() {
            let Some(read) = documents
                .get(&selection.reference())
                .and_then(|bytes| snapshot(bytes))
            else {
                return Err(UniverseRefusal::Unprovisioned {
                    position,
                    entry: selection.clone(),
                });
            };
            if let Some(population) = read
                .populations
                .iter()
                .find(|population| !population.complete)
            {
                return Err(UniverseRefusal::Incomplete {
                    population: population.population.clone(),
                });
            }
        }
        Ok(())
    }

    /// The binding `analyze` takes over this subject: the initial states,
    /// and one universe per complete population the universe snapshots
    /// declare, its keys the union of their objects' keys in reference order.
    /// An explicit subject's references were checked with
    /// [`Self::check_universes`]; the default subject leaves an incomplete
    /// population out (FR-620).
    pub fn binding<'a>(
        &self,
        packages: &'a fsl_semantics::model::intake::PackageInput,
        documents: &'a DocumentProvision,
    ) -> SubjectBinding<'a> {
        let mut universes: BTreeMap<DeclarationKey, (Vec<String>, BTreeSet<String>)> =
            BTreeMap::new();
        for read in self.universes.iter().filter_map(|selection| {
            documents
                .get(&selection.reference())
                .and_then(|bytes| snapshot(bytes))
        }) {
            for population in read
                .populations
                .iter()
                .filter(|population| population.complete)
            {
                // A hyper subject can share one snapshot across selected
                // models. Its `model` header names one model, while each
                // population identity names its own declaring package.
                let (keys, seen) = universes
                    .entry(DeclarationKey {
                        package: population
                            .population
                            .strip_prefix("ix://")
                            .and_then(|identity| identity.rsplit_once('/'))
                            .map_or_else(
                                || read.model.identity.clone(),
                                |(package, _)| package.to_owned(),
                            ),
                        node: population.population.clone(),
                    })
                    .or_default();
                for object in &population.objects {
                    if seen.insert(object.key.clone()) {
                        keys.push(object.key.clone());
                    }
                }
            }
        }
        SubjectBinding {
            packages,
            snapshots: documents,
            initial: self
                .initial_states
                .iter()
                .map(DocumentSelection::reference)
                .collect(),
            universes: universes
                .into_iter()
                .map(|(population, (keys, _))| PopulationUniverse { population, keys })
                .collect(),
        }
    }

    fn wire(&self) -> Value {
        let mut subject = json!({
            "initial_states": self.initial_states.iter().map(DocumentSelection::wire).collect::<Vec<_>>(),
            "universes": self.universes.iter().map(DocumentSelection::wire).collect::<Vec<_>>(),
            "proof_bounds": self.proof_bounds,
        });
        if let (Some(protocol), Some(members)) = (&self.protocol, subject.as_object_mut()) {
            members.insert("protocol".to_owned(), protocol.wire());
        }
        subject
    }

    /// This subject with the model parts it leaves out (FSL FR-1033: a
    /// subject may state only its `protocol`) taken from `base`, the request's
    /// subject.
    #[must_use]
    pub fn completed_by(mut self, base: &Self) -> Self {
        self.memory.clone_from(&base.memory);
        if self.initial_states.is_empty()
            && self.universes.is_empty()
            && self.proof_bounds.is_empty()
        {
            self.initial_states.clone_from(&base.initial_states);
            self.universes.clone_from(&base.universes);
            self.proof_bounds.clone_from(&base.proof_bounds);
        }
        self
    }

    /// Whether both subjects have the same model parts, whatever protocol
    /// either binds.
    fn same_model(&self, other: &Self) -> bool {
        self.initial_states == other.initial_states
            && self.universes == other.universes
            && self.proof_bounds == other.proof_bounds
    }
}

/// Why a request's `subject.universes` cannot form the subject's universes
/// (FR-620).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum UniverseRefusal {
    /// The entry at `position` names no provisioned snapshot document
    /// (`invalid-request`).
    #[error("subject.universes[{position}] {{{}, {}}} names no provisioned snapshot", entry.authority, entry.identity)]
    Unprovisioned {
        /// The entry's position in `subject.universes`.
        position: usize,
        /// The entry.
        entry: DocumentSelection,
    },
    /// A referenced snapshot marks `population` incomplete
    /// (`incomplete_population`).
    #[error("population {population} of a universe snapshot is incomplete")]
    Incomplete {
        /// The population's declaration identity.
        population: String,
    },
}

/// The members of a snapshot the default subject reads; the snapshot itself
/// is admitted by the engine's binding.
#[derive(Deserialize)]
struct SnapshotRead {
    model: SnapshotModel,
    anchor: SnapshotAnchor,
    #[serde(default)]
    populations: Vec<SnapshotPopulation>,
}

impl SnapshotRead {
    #[ix_trace_rs::string_edge]
    fn initialization(&self) -> bool {
        self.anchor.kind == "initialization"
    }
}

#[derive(Deserialize)]
struct SnapshotModel {
    identity: String,
}

#[derive(Deserialize)]
struct SnapshotAnchor {
    kind: String,
}

#[derive(Deserialize)]
struct SnapshotPopulation {
    population: String,
    complete: bool,
    #[serde(default)]
    objects: Vec<SnapshotObject>,
}

#[derive(Deserialize)]
struct SnapshotObject {
    key: String,
}

fn snapshot(bytes: &[u8]) -> Option<SnapshotRead> {
    serde_json::from_slice(bytes).ok()
}

/// FSpec FR-368's `item_kind` spellings.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ItemKindSelection {
    /// An authored clause.
    Formula,
    /// The derived deadlock-freedom item.
    DeadlockFreedom,
    /// The derived time-lock-freedom item.
    TimeLockFreedom,
    /// The derived race-freedom item.
    RaceFreedom,
    /// A refinement item.
    Refinement,
    /// A repeat's invariant obligation (FR-730).
    LoopInvariant,
    /// A repeat's termination obligation (FR-730).
    LoopTermination,
}

impl ItemKindSelection {
    const fn kind(self) -> ItemKind {
        match self {
            Self::Formula => ItemKind::Formula,
            Self::DeadlockFreedom => ItemKind::DeadlockFreedom,
            Self::TimeLockFreedom => ItemKind::TimeLockFreedom,
            Self::RaceFreedom => ItemKind::RaceFreedom,
            Self::Refinement => ItemKind::Refinement,
            Self::LoopInvariant => ItemKind::LoopInvariant,
            Self::LoopTermination => ItemKind::LoopTermination,
        }
    }
}

/// The largest count a request member carries: every JSON reader holds it
/// exactly (FSpec FR-520), and it fits the `usize` an explicit-state engine
/// counts in on every supported target.
const MAX_COUNT: u64 = 9_007_199_254_740_991;

/// Read one count member named `member`: a JSON integer from `minimum` to
/// 2^53 - 1 that fits `usize`. A larger value refuses the whole request
/// naming the member, never a silently dropped bound (FR-620).
fn count<'de, D: serde::Deserializer<'de>>(
    decoder: D,
    member: &str,
    minimum: u64,
) -> Result<u64, D::Error> {
    let value = u64::deserialize(decoder)?;
    if value < minimum || value > MAX_COUNT || usize::try_from(value).is_err() {
        return Err(serde::de::Error::custom(format!(
            "{member} {value} is not a count from {minimum} to {MAX_COUNT}"
        )));
    }
    Ok(value)
}

/// Generate the `deserialize_with` reader of each named count member.
macro_rules! count_members {
    ($($reader:ident => $member:literal),+ $(,)?) => {
        $(
            fn $reader<'de, D: serde::Deserializer<'de>>(
                decoder: D,
            ) -> Result<Option<NonZeroU64>, D::Error> {
                count(decoder, $member, 1).map(NonZeroU64::new)
            }
        )+
    };
}

count_members! {
    read_max_states => "temporal.limits.max_states",
    read_max_transitions => "temporal.limits.max_transitions",
    read_max_automaton_states => "temporal.limits.max_automaton_states",
    read_max_walk_steps => "temporal.limits.max_walk_steps",
    read_max_witness_set => "temporal.limits.max_witness_set",
    read_max_relation_tuples => "temporal.limits.max_relation_tuples",
    read_max_candidates => "temporal.limits.max_candidates",
    read_max_clocks => "temporal.limits.max_clocks",
    read_max_task_product_work => "temporal.limits.max_task_product_work",
    read_max_store_buffer => "temporal.limits.max_store_buffer",
    read_max_messages => "temporal.limits.max_messages",
    read_max_live_instances => "temporal.limits.max_live_instances",
    read_max_solver_steps => "temporal.limits.max_solver_steps",
    read_max_solver_time => "temporal.limits.max_solver_time",
}

/// An integer request bound, retaining out-of-range integer tokens until the
/// item kind determines whether they are a request error or an item refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestCount {
    /// The in-range integer, or `None` when the integer is out of range.
    value: Option<u64>,
}

impl RequestCount {
    fn read<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let raw = Box::<serde_json::value::RawValue>::deserialize(decoder)?;
        let token = raw.get();
        let digits = token.strip_prefix('-').unwrap_or(token);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(serde::de::Error::custom("the bound must be a JSON integer"));
        }
        let value = if token == "-0" {
            Some(0)
        } else {
            token
                .parse::<u64>()
                .ok()
                .filter(|value| *value <= MAX_COUNT)
        };
        Ok(Self { value })
    }

    fn value(self) -> Option<u64> {
        self.value
    }
}

impl<'de> Deserialize<'de> for RequestCount {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        Self::read(decoder)
    }
}

/// FSpec FR-331's `temporal.limits`, each member a positive count.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TemporalLimitsSelection {
    /// `max_states`.
    #[serde(default, deserialize_with = "read_max_states")]
    pub max_states: Option<NonZeroU64>,
    /// `max_transitions`.
    #[serde(default, deserialize_with = "read_max_transitions")]
    pub max_transitions: Option<NonZeroU64>,
    /// `max_automaton_states`.
    #[serde(default, deserialize_with = "read_max_automaton_states")]
    pub max_automaton_states: Option<NonZeroU64>,
    /// `max_walk_steps`.
    #[serde(default, deserialize_with = "read_max_walk_steps")]
    pub max_walk_steps: Option<NonZeroU64>,
    /// `max_witness_set`.
    #[serde(default, deserialize_with = "read_max_witness_set")]
    pub max_witness_set: Option<NonZeroU64>,
    /// `max_relation_tuples`.
    #[serde(default, deserialize_with = "read_max_relation_tuples")]
    pub max_relation_tuples: Option<NonZeroU64>,
    /// `max_candidates` (FR-625): the most successor candidates one expansion enumerates.
    #[serde(default, deserialize_with = "read_max_candidates")]
    pub max_candidates: Option<NonZeroU64>,
    /// `max_clocks` (FSpec FR-415).
    #[serde(default, deserialize_with = "read_max_clocks")]
    pub max_clocks: Option<NonZeroU64>,
    /// `max_task_product_work` (FSpec FR-415).
    #[serde(default, deserialize_with = "read_max_task_product_work")]
    pub max_task_product_work: Option<NonZeroU64>,
    /// `max_store_buffer` (FSpec FR-439).
    #[serde(default, deserialize_with = "read_max_store_buffer")]
    pub max_store_buffer: Option<NonZeroU64>,
    /// `max_messages` (FSpec FR-439).
    #[serde(default, deserialize_with = "read_max_messages")]
    pub max_messages: Option<NonZeroU64>,
    /// `max_live_instances` (FSpec FR-428).
    #[serde(default, deserialize_with = "read_max_live_instances")]
    pub max_live_instances: Option<NonZeroU64>,
    /// `max_solver_steps` (FSpec FR-368).
    #[serde(default, deserialize_with = "read_max_solver_steps")]
    pub max_solver_steps: Option<NonZeroU64>,
    /// `max_solver_time` (FSpec FR-368).
    #[serde(default, deserialize_with = "read_max_solver_time")]
    pub max_solver_time: Option<NonZeroU64>,
}

impl TemporalLimitsSelection {
    /// Whether a weak-memory bound (FR-225) is set, which only a run over a
    /// protocol subject reads.
    fn sets_memory_limits(&self) -> bool {
        self.max_store_buffer.is_some() || self.max_messages.is_some()
    }

    /// Whether a limit no explicit-state EN-1 run reads is set.
    fn sets_other_engine_limits(&self) -> bool {
        [
            self.max_live_instances,
            self.max_solver_steps,
            self.max_solver_time,
        ]
        .iter()
        .any(Option::is_some)
    }
}

/// A refinement item's abstract subject, FSpec FR-368's `{identity,
/// initial_states}`: the abstract model's domain package identity and the
/// snapshots its initial states are read from.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AbstractSubjectSelection {
    /// The abstract model's domain package identity.
    pub identity: String,
    /// The abstract subject's initial-state snapshots, in request order.
    pub initial_states: Vec<DocumentSelection>,
}

/// FSpec FR-331's `temporal` member of a document item, without the
/// `fairness` FSL resolves from the checked clause.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TemporalSelection {
    /// The item kind; `formula` when absent.
    #[serde(default)]
    pub item_kind: Option<ItemKindSelection>,
    /// Protocol-analysis profile (FR-636, FR-575).
    #[serde(default)]
    pub profile: Option<String>,
    /// The item's model subject; the request's when absent.
    #[serde(default)]
    pub subject: Option<SubjectSelection>,
    /// A refinement item's abstract subject (FR-630, FSpec FR-368).
    #[serde(default)]
    pub abstract_subject: Option<AbstractSubjectSelection>,
    /// The stutter-closure premise.
    #[serde(default)]
    pub stutter_closure: Option<bool>,
    /// A bounded search's horizon.
    #[serde(default)]
    pub max_depth: Option<RequestCount>,
    /// Realizability's local-history bound (FR-636, ADR-036 PA-14).
    #[serde(default)]
    pub max_history: Option<RequestCount>,
    /// The item's own budgets.
    #[serde(default)]
    pub limits: Option<TemporalLimitsSelection>,
    /// The SMT encoding budgets.
    #[serde(default)]
    pub smt_encoding: Option<Value>,
    /// The witness walks per initial state (FSpec FR-392).
    #[serde(default)]
    pub witness_samples: Option<u64>,
    /// The witness-sampling seed (FSpec FR-392).
    #[serde(default)]
    pub seed: Option<Value>,
}

impl TemporalSelection {
    /// The witness-sampling seed as an unsigned count: a JSON integer or its
    /// decimal string; `None` when absent or neither.
    fn seed_count(&self) -> Option<u64> {
        match self.seed.as_ref()? {
            Value::Number(number) => number.as_u64(),
            Value::String(text) => text.parse().ok(),
            _ => None,
        }
    }
}

/// FSpec FR-331's `check` member: the case budget of a value-validity or
/// operation-contract item.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CheckSelection {
    /// `limits.max_cases`.
    pub limits: CheckLimitsSelection,
}

/// `check.limits`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CheckLimitsSelection {
    /// The case budget.
    pub max_cases: NonZeroU64,
}

/// One FSpec FR-520 analysis item, as a document writes it. Closed: an
/// unknown member, and each member FSL computes, refuses the request.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, try_from = "ItemSelectionWire")]
pub struct ItemSelection {
    /// The claim's declared name.
    #[serde(default)]
    pub claim: Option<String>,
    /// The FSpec FR-290 kind, or `simulation`.
    #[serde(default)]
    pub kind: Option<String>,
    /// The backend the caller names.
    #[serde(default)]
    pub backend: Option<String>,
    /// Whether the aggregate counts the item; `true` when absent.
    #[serde(default)]
    pub required: Option<bool>,
    /// FSpec FR-331 `temporal`.
    #[serde(default)]
    pub temporal: Option<TemporalSelection>,
    /// FSpec FR-331 `probabilistic`.
    #[serde(default)]
    pub probabilistic: Option<Value>,
    /// FSpec FR-331 `hybrid`.
    #[serde(default)]
    pub hybrid: Option<Value>,
    /// FSpec FR-331 `check`.
    #[serde(default)]
    pub check: Option<CheckSelection>,
    /// FSpec FR-331 `options`.
    #[serde(default)]
    pub options: Option<BTreeMap<String, String>>,
    /// FSpec FR-382 `reductions`, over the unit's declarations (FR-623).
    #[serde(default)]
    pub reductions: Option<Value>,
    /// FSpec FR-331 `correspondence`.
    #[serde(default)]
    pub correspondence: Option<Value>,
    /// FSpec FR-331 `premises`.
    #[serde(default)]
    pub premises: Option<Value>,
    /// FSpec FR-331 `assessed_root`.
    #[serde(default)]
    pub assessed_root: Option<Value>,
    /// A `finite-replay` item's supplied trace.
    #[serde(default)]
    pub trace: Option<DocumentSelection>,
    /// An FR-739 finite trace population for a finite hyper item.
    #[serde(default)]
    pub traces: Option<DocumentSelection>,
    /// A `global-conformance` item's supplied FR-737 recording.
    #[serde(default)]
    pub recording: Option<DocumentSelection>,
    /// The activated population used to close a recording's fan-out scope.
    #[serde(default, deserialize_with = "present_json_value")]
    pub population: Option<Value>,
    /// A `simulation` item's settings (FR-634).
    #[serde(default)]
    pub simulation: Option<super::SimulationSelection>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemSelectionWire {
    #[serde(default)]
    claim: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    required: Option<bool>,
    #[serde(default)]
    temporal: Option<TemporalSelection>,
    #[serde(default)]
    probabilistic: Option<Value>,
    #[serde(default)]
    hybrid: Option<Value>,
    #[serde(default)]
    check: Option<CheckSelection>,
    #[serde(default)]
    options: Option<BTreeMap<String, String>>,
    #[serde(default)]
    reductions: Option<Value>,
    #[serde(default)]
    correspondence: Option<Value>,
    #[serde(default)]
    premises: Option<Value>,
    #[serde(default)]
    assessed_root: Option<Value>,
    #[serde(default)]
    trace: Option<DocumentSelection>,
    #[serde(default)]
    traces: Option<DocumentSelection>,
    #[serde(default)]
    recording: Option<DocumentSelection>,
    #[serde(default, deserialize_with = "present_json_value")]
    population: Option<Value>,
    #[serde(default)]
    simulation: Option<super::SimulationSelection>,
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
struct ItemSelectionReadError {
    message: &'static str,
}

impl TryFrom<ItemSelectionWire> for ItemSelection {
    type Error = ItemSelectionReadError;

    fn try_from(wire: ItemSelectionWire) -> Result<Self, Self::Error> {
        let protocol_analysis = matches!(
            wire.kind.as_deref(),
            Some("local-projection" | "realizability")
        );
        if !protocol_analysis {
            if let Some(temporal) = wire.temporal.as_ref() {
                if temporal.max_depth.is_some_and(|bound| {
                    bound
                        .value()
                        .is_none_or(|value| usize::try_from(value).is_err())
                }) {
                    return Err(ItemSelectionReadError {
                        message: "temporal.max_depth is out of range",
                    });
                }
                if temporal.profile.is_some() || temporal.max_history.is_some() {
                    return Err(ItemSelectionReadError {
                        message:
                            "a non-protocol item carries an unsupported protocol member or count",
                    });
                }
            }
        }
        Ok(Self {
            claim: wire.claim,
            kind: wire.kind,
            backend: wire.backend,
            required: wire.required,
            temporal: wire.temporal,
            probabilistic: wire.probabilistic,
            hybrid: wire.hybrid,
            check: wire.check,
            options: wire.options,
            reductions: wire.reductions,
            correspondence: wire.correspondence,
            premises: wire.premises,
            assessed_root: wire.assessed_root,
            trace: wire.trace,
            traces: wire.traces,
            recording: wire.recording,
            population: wire.population,
            simulation: wire.simulation,
        })
    }
}

impl ItemSelection {
    /// Whether the item carries a member whose in-process route `analyze`
    /// does not build yet, so the item settles `unsupported` on its own.
    fn unrouted(&self) -> bool {
        self.unrouted_routing_options(false)
    }

    /// [`Self::unrouted`] for an item whose `options` are negotiated against
    /// its candidates' published values (FR-243, FR-635) when it names no
    /// backend, as a temporal claim's `method` is; `options_negotiated`
    /// false sends an option only to the backend the item names (FR-967).
    fn unrouted_routing_options(&self, options_negotiated: bool) -> bool {
        self.unrouted_members(options_negotiated)
            || self.temporal.as_ref().is_some_and(|temporal| {
                temporal.witness_samples.is_some() || temporal.seed.is_some()
            })
    }

    /// [`Self::unrouted`] without the witness sampling members, which a
    /// state-graph claim reads (FR-628). Its backend options are negotiated
    /// against the candidates' published values, as for other temporal
    /// claims (FR-243).
    fn unrouted_beyond_sampling(&self) -> bool {
        self.unrouted_members(true)
    }

    fn unrouted_members(&self, options_negotiated: bool) -> bool {
        let temporal = self.temporal.as_ref();
        let protocol_analysis = matches!(
            self.kind.as_deref(),
            Some("local-projection" | "realizability")
        );
        self.hybrid.is_some()
            || self.correspondence.is_some()
            || (self.premises.is_some() && !protocol_analysis)
            || (self.assessed_root.is_some() && !protocol_analysis)
            || (self.trace.is_some()
                && self.kind.as_deref() != Some(Capability::FiniteReplay.to_wire()))
            || (self.recording.is_some()
                && self.kind.as_deref() != Some(Capability::GlobalConformance.to_wire()))
            || self.simulation.is_some()
            // An option is sent only to the backend the item names (FR-967),
            // unless the item's candidates negotiate it (FR-243).
            || (!options_negotiated
                && self.backend.is_none()
                && !protocol_analysis
                && self
                    .options
                    .as_ref()
                    .is_some_and(|options| !options.is_empty()))
            || temporal.is_some_and(|temporal| {
                (temporal.abstract_subject.is_some()
                    && !matches!(temporal.item_kind, Some(ItemKindSelection::Refinement)))
                    || temporal.smt_encoding.is_some()
                    || temporal
                        .limits
                        .is_some_and(|limits| limits.sets_other_engine_limits())
                    // EN-6's task-product search reads `max_task_product_work`
                    // for a schedulability item; no other item does.
                    || (self
                        .kind
                        .as_deref()
                        .is_some_and(|kind| kind != Capability::Schedulability.to_wire())
                        && temporal
                            .limits
                            .is_some_and(|limits| limits.max_task_product_work.is_some()))
            })
    }
}

/// FR-741's default `min_cycles` of the regenerative method a long-run
/// fraction defaults to.
const DEFAULT_MIN_CYCLES: u64 = 100;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbabilisticSelection {
    #[serde(default)]
    evidence: Option<String>,
    #[serde(default)]
    method: Option<StatisticalMethodSelection>,
    #[serde(default)]
    seed: Option<String>,
    #[serde(default)]
    max_witnesses: Option<u32>,
    #[serde(default)]
    workers: Option<NonZeroU32>,
    #[serde(default)]
    timed_sampling: Option<TimedSamplingSelection>,
    /// The evidence kind's engine budgets, read once the kind is known.
    #[serde(default)]
    limits: Option<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatisticalMethodSelection {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    min_cycles: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimedSamplingSelection {
    #[serde(default)]
    q: Option<u32>,
    #[serde(default)]
    max_sample_steps: Option<u64>,
}

impl TimedSamplingSelection {
    fn resolve(self) -> fsl_replay::statistical::TimedSampling {
        let defaults = fsl_replay::statistical::TimedSampling::default();
        fsl_replay::statistical::TimedSampling {
            q: self.q.unwrap_or(defaults.q),
            max_sample_steps: self.max_sample_steps.unwrap_or(defaults.max_sample_steps),
        }
    }
}

/// FR-191's `StatisticalLimits` members of `probabilistic.limits` (FR-741):
/// each a positive count, the published default when absent.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatisticalLimitsSelection {
    #[serde(default)]
    max_samples: Option<NonZeroU64>,
    #[serde(default)]
    max_draws: Option<NonZeroU64>,
    #[serde(default)]
    max_cycle_steps: Option<NonZeroU64>,
}

impl StatisticalLimitsSelection {
    fn resolve(self) -> fsl_replay::statistical::StatisticalLimits {
        let defaults = fsl_replay::statistical::StatisticalLimits::default();
        fsl_replay::statistical::StatisticalLimits {
            max_samples: self
                .max_samples
                .map_or(defaults.max_samples, NonZeroU64::get),
            max_draws: self.max_draws.map_or(defaults.max_draws, NonZeroU64::get),
            max_cycle_steps: self
                .max_cycle_steps
                .map_or(defaults.max_cycle_steps, NonZeroU64::get),
        }
    }
}

/// FR-203's `ExactProbLimits` members of `probabilistic.limits` (FR-741),
/// in FSpec FR-411's flat spelling: the `model` members sit beside the
/// solver members. Each is a positive count, the published default when
/// absent.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactLimitsSelection {
    #[serde(default)]
    max_states: Option<NonZeroU64>,
    #[serde(default)]
    max_transitions: Option<NonZeroU64>,
    #[serde(default)]
    max_automaton_states: Option<NonZeroU64>,
    #[serde(default)]
    max_iterations: Option<NonZeroU64>,
    #[serde(default)]
    precision_bits: Option<NonZeroU32>,
    #[serde(default)]
    max_precision_bits: Option<NonZeroU32>,
    #[serde(default)]
    max_rational_bits: Option<NonZeroU64>,
    #[serde(default)]
    max_policy_iterations: Option<NonZeroU64>,
}

impl ExactLimitsSelection {
    fn resolve(self) -> Result<crate::exact_probabilistic::ExactProbLimits, ()> {
        let mut limits = crate::exact_probabilistic::ExactProbLimits::default();
        let count = |value: NonZeroU64| usize::try_from(value.get()).map_err(|_| ());
        if let Some(value) = self.max_states {
            limits.model.limits.max_states = count(value)?;
        }
        if let Some(value) = self.max_transitions {
            limits.model.limits.max_transitions = count(value)?;
        }
        if let Some(value) = self.max_automaton_states {
            limits.model.max_automaton_states = value.get();
        }
        if let Some(value) = self.max_iterations {
            limits.max_iterations = value.get();
        }
        if let Some(value) = self.precision_bits {
            limits.precision_bits = value.get();
        }
        if let Some(value) = self.max_precision_bits {
            limits.max_precision_bits = value.get();
        }
        if let Some(value) = self.max_rational_bits {
            limits.max_rational_bits = value.get();
        }
        if let Some(value) = self.max_policy_iterations {
            limits.max_policy_iterations = value.get();
        }
        if limits.precision_bits > limits.max_precision_bits {
            return Err(());
        }
        Ok(limits)
    }
}

/// FR-741's `probabilistic` request member as the item it negotiates: the
/// evidence kind (`default_evidence` when absent), its method, seed and
/// budgets, each absent member at its published default. `long_run` selects
/// the default method of a long-run fraction, `regenerative` with
/// `min_cycles` 100 (FR-190).
fn probabilistic_item(
    value: &Value,
    default_evidence: EvidenceKind,
    long_run: bool,
) -> Result<super::ProbabilisticItem, ()> {
    let selection: ProbabilisticSelection =
        serde_json::from_value(value.clone()).map_err(|_| ())?;
    let evidence = selection
        .evidence
        .as_deref()
        .map(EvidenceKind::from_wire)
        .unwrap_or(Some(default_evidence))
        .ok_or(())?;
    let timed_sampling = selection
        .timed_sampling
        .map(TimedSamplingSelection::resolve);
    let (method, seed, sampler, limits) = match evidence {
        EvidenceKind::Exact => {
            if selection.method.is_some() || selection.seed.is_some() || timed_sampling.is_some() {
                return Err(());
            }
            let limits = match selection.limits {
                None => crate::exact_probabilistic::ExactProbLimits::default(),
                Some(limits) => serde_json::from_value::<ExactLimitsSelection>(limits)
                    .map_err(|_| ())?
                    .resolve()?,
            };
            (None, None, None, super::ProbabilisticLimits::Exact(limits))
        }
        EvidenceKind::Statistical => {
            let method = match selection.method {
                None if long_run => fsl_replay::statistical::StatisticalMethod::Regenerative {
                    min_cycles: DEFAULT_MIN_CYCLES,
                },
                None => fsl_replay::statistical::StatisticalMethod::Okamoto,
                Some(StatisticalMethodSelection { kind, min_cycles }) => match kind.as_str() {
                    "okamoto" if min_cycles.is_none() => {
                        fsl_replay::statistical::StatisticalMethod::Okamoto
                    }
                    "sprt" if min_cycles.is_none() => {
                        fsl_replay::statistical::StatisticalMethod::Sprt
                    }
                    "clopper-pearson" if min_cycles.is_none() => {
                        fsl_replay::statistical::StatisticalMethod::ClopperPearson
                    }
                    "regenerative" => fsl_replay::statistical::StatisticalMethod::Regenerative {
                        min_cycles: min_cycles.ok_or(())?,
                    },
                    _ => return Err(()),
                },
            };
            let seed = selection
                .seed
                .as_deref()
                .unwrap_or("0")
                .parse::<u64>()
                .map_err(|_| ())?;
            let sampler = fsl_foundation::selection::DefinitionRef::new(
                "agent-ix",
                "filament.simulation.sampler/v1",
            )
            .map_err(|_| ())?;
            let limits = match selection.limits {
                None => fsl_replay::statistical::StatisticalLimits::default(),
                Some(limits) => serde_json::from_value::<StatisticalLimitsSelection>(limits)
                    .map_err(|_| ())?
                    .resolve(),
            };
            (
                Some(method),
                Some(seed),
                Some(sampler),
                super::ProbabilisticLimits::Statistical(limits),
            )
        }
    };
    Ok(super::ProbabilisticItem {
        evidence,
        limits,
        method,
        seed,
        sampler,
        max_witnesses: selection.max_witnesses,
        workers: selection.workers,
        symmetry: Vec::new(),
        timed_sampling,
    })
}

/// One item as [`resolve_items`] resolved it.
#[derive(Clone)]
pub enum ResolvedItem {
    /// An item `analyze` runs once it is negotiated.
    Claim(Box<ResolvedClaim>),
    /// A `simulation` item (FR-634), which settles no claim and is not
    /// negotiated.
    Simulation(Box<super::SimulationItem>),
    /// An item settled without running: its terminal record.
    Settled {
        /// The declared name the item used, when it named one.
        claim_name: Option<String>,
        /// The settled record.
        record: Box<TerminalRecord>,
    },
}

/// A resolved claim item with its declared name and its candidate outcome.
#[derive(Clone)]
pub struct ResolvedClaim {
    /// The item `analyze` takes.
    pub item: ClaimItem,
    /// The declared name of the claim, or a derived item's anchor.
    pub claim_name: String,
    /// The candidates the registry gives the item.
    pub candidates: CandidateOutcome,
    /// The item's model subject.
    pub subject: SubjectSelection,
    /// An item whose own subject differs from the request's, or that
    /// carries a member `analyze` has no in-process route for, settles
    /// `unsupported` without being negotiated.
    pub unrouted: bool,
}

/// The caller's negotiation (FR-281 "Negotiated items"): FSL computes the
/// candidates, and the caller settles each item from them, as CG's
/// `negotiate_*` does in the driver.
pub trait Negotiate {
    /// The disposition and its diagnostic for `item` over `candidates`.
    fn negotiate(
        &self,
        item: &ClaimItem,
        candidates: &CandidateOutcome,
    ) -> (Disposition, Option<Diagnostic>);
}

fn unknown_claim(index: RequestIndex, name: &str) -> ResolvedItem {
    let mut detail = Map::new();
    detail.insert("name".to_owned(), json!(name));
    ResolvedItem::Settled {
        claim_name: Some(name.to_owned()),
        record: Box::new(TerminalRecord::disposition_invalid_request(
            index,
            CatalogCode::new("invalid_capability", "unknown-claim"),
            detail,
        )),
    }
}

/// FR-628 "Refusals": `witness_samples` on a claim that is not `possible`,
/// and a `seed` that is no unsigned count, refuse the item `invalid-request`
/// naming the member.
fn sampling_refusal(
    index: RequestIndex,
    name: &str,
    claim: &fsl_semantics::check::CheckedStateGraphClaim,
    temporal: &TemporalSelection,
) -> Option<ResolvedItem> {
    let member = if temporal.witness_samples.is_some()
        && !matches!(
            claim.form(),
            fsl_semantics::check::StateGraphForm::Possible { .. }
        ) {
        "temporal.witness_samples"
    } else if temporal.seed.is_some() && temporal.seed_count().is_none() {
        "temporal.seed"
    } else {
        return None;
    };
    let mut detail = Map::new();
    detail.insert("member".to_owned(), json!(member));
    Some(ResolvedItem::Settled {
        claim_name: Some(name.to_owned()),
        record: Box::new(TerminalRecord::disposition_invalid_request(
            index,
            CatalogCode::new("invalid_runtime_input", "invalid-value"),
            detail,
        )),
    })
}

fn invalid_kind(index: RequestIndex, name: Option<&str>, cause: &'static str) -> ResolvedItem {
    ResolvedItem::Settled {
        claim_name: name.map(str::to_owned),
        record: Box::new(TerminalRecord::disposition_invalid_request(
            index,
            CatalogCode::new("invalid_capability", cause),
            Map::new(),
        )),
    }
}

/// FR-967, FSpec FR-779: an item naming a registered backend with an option
/// that backend's manifest does not publish settles `invalid-request`
/// `invalid_capability`/`unknown-option`, naming the least unpublished
/// name. An item naming no backend, or one the registry does not hold
/// (which settles `unknown-backend`), is not checked.
fn unknown_option(
    index: RequestIndex,
    name: Option<&str>,
    registry: &Registry,
    named: Option<&BackendId>,
    options: &BTreeMap<String, String>,
) -> Option<ResolvedItem> {
    let backend = named?;
    let option = registry
        .descriptor(backend)?
        .unpublished_option(options.keys())?;
    let mut detail = Map::new();
    detail.insert("request_index".to_owned(), json!(index.get()));
    detail.insert("backend".to_owned(), json!(backend.as_str()));
    detail.insert("option".to_owned(), json!(option));
    Some(ResolvedItem::Settled {
        claim_name: name.map(str::to_owned),
        record: Box::new(TerminalRecord::disposition_invalid_request(
            index,
            CatalogCode::new("invalid_capability", "unknown-option"),
            detail,
        )),
    })
}

/// FR-634: the request kind of a simulation item.
pub const SIMULATION_KIND: &str = "simulation";

/// FR-630: the request kind of a refinement item.
const REFINEMENT_KIND: &str = "refinement";

/// The checked declarations by their declared qualified name.
fn declarations(package: &CheckedPackage) -> BTreeMap<String, filament_exact::NodeKey> {
    let graph = package.graph();
    let mut names: BTreeMap<String, filament_exact::NodeKey> = declared_nodes(package).collect();
    names.extend(graph.hyper_clauses().iter().filter_map(|clause| {
        Some((
            clause.name().to_owned(),
            clause.assembled_occurrence()?.node().clone(),
        ))
    }));
    names.extend(
        graph
            .temporal_clauses()
            .iter()
            .map(|clause| (clause.clause().name().to_owned(), clause.identity())),
    );
    // FR-750: a checked hybrid claim is a claim by its declared name.
    names.extend(
        graph
            .hybrid_claim_sites()
            .iter()
            .map(|site| (site.name.clone(), site.identity.clone())),
    );
    // FR-625, FR-489: every state clause is a claim by its declared name,
    // including the pre/post clauses selected as operation-contract items.
    names.extend(
        graph
            .state_clauses()
            .iter()
            .filter(|clause| clause.is_invariant() || clause.operation().is_some())
            .map(|clause| (clause.name().to_owned(), clause.identity())),
    );
    names.extend(
        graph
            .finite_hyper_clauses()
            .iter()
            .filter_map(|clause| Some((clause.name().to_owned(), clause.identity()?.clone()))),
    );
    names.extend(
        graph.state_graph_claims().iter().filter_map(|claim| {
            Some((claim.name().to_owned(), claim.occurrence()?.node().clone()))
        }),
    );
    names.extend(
        graph
            .probabilistic_model()
            .claims()
            .iter()
            .filter_map(|claim| Some((claim.name.clone(), claim.occurrence()?.node().clone()))),
    );
    // FR-630: a refinement is named by its declared name and keyed at the
    // node of its requirement record (FR-144).
    names.extend(graph.refinements().iter().filter_map(|refinement| {
        Some((
            refinement.name().to_owned(),
            refinement.requirement_occurrence()?.node().clone(),
        ))
    }));
    names
}

/// Whether the claim at `occurrence` is a state-graph claim (FR-165).
pub(super) fn is_state_graph(package: &CheckedPackage, occurrence: &OccurrenceKey) -> bool {
    package
        .graph()
        .state_graph_claims()
        .iter()
        .any(|claim| claim.occurrence() == Some(occurrence))
}

/// The semantic graph's declaring nodes by qualified name.
fn declared_nodes(
    package: &CheckedPackage,
) -> impl Iterator<Item = (String, filament_exact::NodeKey)> + '_ {
    package.graph().semantic_graph().nodes().filter_map(|node| {
        node.declaration().map(|name| {
            let name = name
                .iter()
                .map(|segment| segment.as_str())
                .collect::<Vec<_>>()
                .join("::");
            (name, node.key())
        })
    })
}

/// The declared name of the declaration `node`, when it declares one.
fn declared_name(package: &CheckedPackage, node: &filament_exact::NodeKey) -> Option<String> {
    declarations(package)
        .into_iter()
        .find(|(_, key)| key == node)
        .map(|(name, _)| name)
}

/// The occurrence a claim item over the declaration `node` names: the
/// occurrence of its requirement record; for a `Boolean` function, the
/// Boolean value clause its body is (FSpec FR-290, FR-484 "Forms"), at its
/// `claim` or `declaration` occurrence; else its first checked occurrence.
fn claim_occurrence(
    package: &CheckedPackage,
    records: &[fsl_route::request::RequirementItem],
    node: &filament_exact::NodeKey,
    name: Option<&str>,
) -> Option<OccurrenceKey> {
    let graph = package.graph();
    if let Some(clause) = graph.state_clauses().iter().find(|clause| {
        clause.identity() == *node
            && clause.operation().is_some()
            && name.is_none_or(|name| clause.name() == name)
    }) {
        return Some(OccurrenceKey::new(
            clause.identity(),
            clause.claim().clone(),
        ));
    }
    if let Some(record) = records.iter().find(|record| record.node() == node) {
        return Some(record.occurrence().clone());
    }
    if let Some(occurrence) = graph
        .hyper_clauses()
        .iter()
        .filter_map(|clause| clause.assembled_occurrence())
        .find(|occurrence| occurrence.node() == node)
    {
        return Some(occurrence.clone());
    }
    if let Some(clause) = graph
        .temporal_clauses()
        .iter()
        .find(|clause| clause.identity() == *node)
    {
        return Some(OccurrenceKey::new(
            clause.identity(),
            clause.claim().clone(),
        ));
    }
    if let Some(site) = graph
        .hybrid_claim_sites()
        .iter()
        .find(|site| &site.identity == node)
    {
        return Some(OccurrenceKey::new(
            site.identity.clone(),
            site.claim.clone(),
        ));
    }
    if let Some(clause) = graph.state_clauses().iter().find(|clause| {
        clause.identity() == *node && (clause.is_invariant() || clause.operation().is_some())
    }) {
        return Some(OccurrenceKey::new(
            clause.identity(),
            clause.claim().clone(),
        ));
    }
    if let Some(clause) = graph
        .finite_hyper_clauses()
        .iter()
        .find(|clause| clause.identity() == Some(node))
    {
        return clause.occurrence().cloned();
    }
    let body = graph
        .semantic_graph()
        .node(node.clone())
        .and_then(|declaration| declaration.function_body());
    if let Some(body) = body {
        let clause = graph
            .occurrences()
            .filter(|(key, origin, _)| {
                *key == body && matches!(origin.role().as_str(), "claim" | "declaration")
            })
            .map(|(key, origin, _)| OccurrenceKey::new(key, origin))
            .find(|clause| crate::claim_family::boolean_expression(graph, clause));
        if clause.is_some() {
            return clause;
        }
    }
    graph
        .occurrences()
        .find(|(key, _, _)| key == node)
        .map(|(key, origin, _)| OccurrenceKey::new(key, origin))
}

/// The extent of the Boolean value clause `claim` that the function `name`
/// (declared at `node`) is: the domains of its parameters, the arguments
/// the clause ranges over (FSpec FR-290, FR-484). A claim that is not that
/// clause, or whose parameter types do not classify, is `bounded`, and its
/// item is refused by the checker that selects it.
fn value_clause_extent(
    package: &CheckedPackage,
    name: &str,
    node: &filament_exact::NodeKey,
    claim: &OccurrenceKey,
) -> ExtentClassification {
    value_clause_claim_extent(package, name, node, claim)
        .map_or(ExtentClassification::Bounded, |extent| {
            ExtentClassification::of(&extent)
        })
}

/// The domains of the Boolean value clause `claim` that the function `name`
/// (declared at `node`) is, as [`value_clause_extent`] classifies them;
/// `None` when `claim` is not that clause or its parameter types do not
/// classify.
fn value_clause_claim_extent(
    package: &CheckedPackage,
    name: &str,
    node: &filament_exact::NodeKey,
    claim: &OccurrenceKey,
) -> Option<fsl_semantics::family::ClaimExtent> {
    let graph = package.graph();
    let declaration = graph.semantic_graph().node(node.clone());
    let is_clause = declaration
        .as_ref()
        .and_then(|declaration| declaration.function_body())
        .is_some_and(|body| body == *claim.node());
    let (Some(callable), Some(parameters), true) = (
        graph.callable(name),
        declaration.and_then(|declaration| declaration.function_parameters()),
        is_clause,
    ) else {
        return None;
    };
    let roots: Vec<(filament_exact::NodeKey, &filament_exact::ValueType)> = parameters
        .into_iter()
        .zip(callable.parameters.iter().map(|(_, value_type)| value_type))
        .collect();
    fsl_semantics::family::requirements::classify_extent(&roots, graph.scope().types(), u64::MAX)
        .ok()
}

/// The extent of the Boolean value clause at `claim`, found through the
/// declared function whose body it is (FSpec FR-290, FR-484 "Forms");
/// `None` when no declared function's body is `claim`'s node or its
/// parameter types do not classify.
pub(crate) fn boolean_value_clause_extent(
    package: &CheckedPackage,
    claim: &OccurrenceKey,
) -> Option<fsl_semantics::family::ClaimExtent> {
    let graph = package.graph();
    declarations(package).into_iter().find_map(|(name, node)| {
        let body = graph
            .semantic_graph()
            .node(node.clone())
            .and_then(|declaration| declaration.function_body());
        if body.as_ref() == Some(claim.node()) {
            value_clause_claim_extent(package, &name, &node, claim)
        } else {
            None
        }
    })
}

/// Resolve `selections` (FR-620 "Resolving items") over `package`, the
/// request's default `subject` and the caller's `registry`; `None`
/// selections are FR-620's default items: one per checked claim and kind,
/// then each model subject's derived items, the claims ordered by `units`,
/// the request's source units in request order (the program, then each
/// library as listed). Each result is either an item to negotiate and run
/// or an item already settled.
pub fn resolve_items(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    selections: Option<&[ItemSelection]>,
    registry: &Registry,
    units: &[RawSourceRef],
    binding: &SubjectBinding<'_>,
    limits: &AnalyzeLimits,
) -> Vec<ResolvedItem> {
    match selections {
        Some(selections) => selections
            .iter()
            .enumerate()
            .map(|(position, selection)| {
                resolve_one(
                    package,
                    subject,
                    RequestIndex::new(position),
                    selection,
                    registry,
                    binding,
                )
            })
            .collect(),
        None => default_items(package, subject, registry, units, binding, limits),
    }
}

mod package_run;

pub use package_run::{
    settle_package_run, PackageRun, PackageRunError, PendingFamily, PendingPackageFamily,
};

/// One requirement item of a run call (FR-395 "Item of the call") with the
/// backend and options the run names for it (FR-960).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallItem {
    /// The item's record occurrence: its FSpec FR-331 claim.
    pub claim: OccurrenceKey,
    /// The item's FR-395 request index.
    pub request_index: RequestIndex,
    /// The backend the run names for the item, when it names one.
    pub backend: Option<BackendId>,
    /// The named backend's options (FSpec FR-196).
    pub options: BTreeMap<String, String>,
}

/// Resolve the requirement items of a call of `function` (FR-960, FR-961
/// step 4): each item's candidates are FR-075's for its named backend, an
/// unnamed item's narrowed by FR-962's tie rule, and its request item
/// carries the named backend and its options. An option the named
/// backend's manifest does not publish settles the item
/// `invalid_capability`/`unknown-option` (FR-967). A named backend the
/// registry does not hold keeps FR-075's unknown-backend marker, which
/// negotiation settles, and its options are not checked.
pub fn resolve_call_items(
    package: &CheckedPackage,
    function: &str,
    items: &[CallItem],
    registry: &Registry,
) -> Vec<ResolvedItem> {
    items
        .iter()
        .map(|call_item| resolve_call_item(package, function, call_item, registry))
        .collect()
}

fn resolve_call_item(
    package: &CheckedPackage,
    function: &str,
    call_item: &CallItem,
    registry: &Registry,
) -> ResolvedItem {
    let index = call_item.request_index;
    let named = call_item.backend.as_ref();
    if let Some(refused) =
        unknown_option(index, Some(function), registry, named, &call_item.options)
    {
        return refused;
    }
    let Some(record) = requirement_items(package, registry, named)
        .into_iter()
        .find(|record| record.occurrence() == &call_item.claim)
    else {
        return ResolvedItem::Settled {
            claim_name: Some(function.to_owned()),
            record: Box::new(TerminalRecord::disposition_failed(
                index,
                filament_semantic_value::catalog::InternalFault::new(
                    "run-requirements",
                    "call-item-is-a-record",
                )
                .catalog_code(),
            )),
        };
    };
    let kind = record.kind();
    let candidates = crate::tie_rule::resolve_tie(
        package,
        &call_item.claim,
        kind,
        named,
        false,
        record.candidates().clone(),
    );
    let item = ClaimItem {
        request_index: index,
        claim: call_item.claim.clone(),
        kind,
        extent: record.extent(),
        backend: call_item.backend.clone(),
        candidates: candidate_list(&candidates),
        options: ItemOptions {
            required: true,
            max_cases: None,
            max_task_product_work: None,
            trace: None,
            traces: None,
            recording: None,
            population: None,
            backend_options: call_item.options.clone(),
            protocol_analysis: None,
        },
        form: None,
    };
    ResolvedItem::Claim(Box::new(ResolvedClaim {
        item,
        claim_name: function.to_owned(),
        candidates,
        subject: SubjectSelection::default(),
        unrouted: false,
    }))
}

fn requirement_items(
    package: &CheckedPackage,
    registry: &Registry,
    named: Option<&BackendId>,
) -> Vec<fsl_route::request::RequirementItem> {
    fsl_route::request::items_from_requirements(package.graph().requirements(), registry, named)
}

fn resolve_one(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    index: RequestIndex,
    selection: &ItemSelection,
    registry: &Registry,
    binding: &SubjectBinding<'_>,
) -> ResolvedItem {
    let name = selection.claim.as_deref();
    let Some(name) = name else {
        // FR-634: a claimless `simulation` item runs on the in-process
        // simulator over the request's subject.
        if let (Some(SIMULATION_KIND), Some(settings)) =
            (selection.kind.as_deref(), selection.simulation.as_ref())
        {
            return ResolvedItem::Simulation(Box::new(super::SimulationItem {
                index,
                settings: settings.clone(),
                subject: selection
                    .temporal
                    .as_ref()
                    .and_then(|temporal| temporal.subject.clone()),
            }));
        }
        return ResolvedItem::Settled {
            claim_name: None,
            record: Box::new(TerminalRecord::disposition_unsupported(
                index,
                fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
                Map::new(),
            )),
        };
    };
    if let Some(resolved) =
        resolve_protocol_freedom(package, subject, index, selection, registry, name)
    {
        return resolved;
    }
    if let Some(resolved) =
        resolve_schedule_claim(package, subject, index, selection, registry, name)
    {
        return resolved;
    }
    let Some(node) = declarations(package).get(name).cloned() else {
        return resolve_derived(package, subject, index, selection, registry, binding, name)
            .unwrap_or_else(|| unknown_claim(index, name));
    };
    let state_graph = package.graph().state_graph_claim(name).filter(|claim| {
        claim
            .occurrence()
            .is_some_and(|found| found.node() == &node)
    });
    let named = selection.backend.as_deref().map(BackendId::new);
    let records = requirement_items(package, registry, named.as_ref());
    // FR-940 "Claim occurrence ordinals": an authored temporal clause is its
    // own `claim` occurrence of a node structurally equal clauses share, so
    // the name keys the whole occurrence, never the node's first.
    let Some(claim) = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|clause| clause.clause().name() == name)
        .map(|clause| OccurrenceKey::new(clause.identity(), clause.claim().clone()))
        .or_else(|| claim_occurrence(package, &records, &node, Some(name)))
    else {
        return unknown_claim(index, name);
    };
    // A record keys the declaration, or the value clause a `Boolean`
    // function's name resolves to.
    let own: Vec<&fsl_route::request::RequirementItem> = records
        .iter()
        .filter(|record| record.node() == &node || record.occurrence() == &claim)
        .collect();
    let kind = match selection.kind.as_deref() {
        // FR-630 "Request item": a refinement item names kind `refinement`,
        // the `temporal-satisfaction` record S3 writes for the declaration
        // (FR-144).
        Some(REFINEMENT_KIND)
            if super::refinement_item::refinement_at(package, &claim).is_some() =>
        {
            Capability::TemporalSatisfaction
        }
        Some(label) => match Capability::from_wire(label) {
            Ok(kind) => kind,
            Err(_) => return invalid_kind(index, Some(name), "unknown-kind"),
        },
        None => {
            let mut kinds: Vec<Capability> = Vec::new();
            for record in &own {
                if !kinds.contains(&record.kind()) {
                    kinds.push(record.kind());
                }
            }
            match kinds.as_slice() {
                [kind] => *kind,
                [] if selection.probabilistic.is_some()
                    || package.graph().probabilistic_model().claim(name).is_some() =>
                {
                    Capability::ProbabilisticSatisfaction
                }
                [] if state_graph.is_some()
                    || package
                        .graph()
                        .hyper_clauses()
                        .iter()
                        .any(|clause| clause.assembled_occurrence() == Some(&claim)) =>
                {
                    Capability::TemporalSatisfaction
                }
                _ => return invalid_kind(index, Some(name), "absent-kind"),
            }
        }
    };
    // FSpec FR-368: a refinement item's `abstract_subject.identity` names
    // the refinement's abstract model package.
    if let (Some(abstract_subject), Some(refinement)) = (
        selection
            .temporal
            .as_ref()
            .and_then(|temporal| temporal.abstract_subject.as_ref()),
        super::refinement_item::refinement_at(package, &claim),
    ) {
        if abstract_subject.identity != refinement.abstract_model_selection().identity {
            let mut detail = Map::new();
            detail.insert(
                "member".to_owned(),
                json!("temporal.abstract_subject.identity"),
            );
            return ResolvedItem::Settled {
                claim_name: Some(name.to_owned()),
                record: Box::new(TerminalRecord::disposition_invalid_request(
                    index,
                    CatalogCode::new("invalid_runtime_input", "invalid-value"),
                    detail,
                )),
            };
        }
    }
    let protocol_analysis_kind = matches!(
        kind,
        Capability::LocalProjection | Capability::Realizability
    );
    let record = own.iter().find(|record| record.kind() == kind);
    if selection.probabilistic.is_some() && kind != Capability::ProbabilisticSatisfaction {
        return invalid_value(index, name, "probabilistic");
    }
    let empty_member = json!({});
    let probabilistic_member = selection
        .probabilistic
        .as_ref()
        .or((kind == Capability::ProbabilisticSatisfaction).then_some(&empty_member));
    let probabilistic = if let Some(value) = probabilistic_member {
        let checked = package
            .graph()
            .probabilistic_model()
            .claims()
            .iter()
            .find(|checked| checked.occurrence() == Some(&claim));
        let default_evidence = match checked {
            Some(checked) if checked.exact_only() => EvidenceKind::Exact,
            _ => EvidenceKind::Statistical,
        };
        let long_run = checked.is_some_and(|checked| {
            matches!(
                checked.form,
                fsl_semantics::check::ProbForm::LongRunFraction { .. }
            )
        });
        match probabilistic_item(value, default_evidence, long_run) {
            Ok(item) => Some(item),
            Err(()) => return invalid_value(index, name, "probabilistic"),
        }
    } else {
        None
    };
    // FR-967: options are checked after kind admission and the
    // inapplicable-kind rule, which `analyze` applies to an item whose kind
    // does not apply to its claim's family, and before candidates.
    let item_kind = selection
        .temporal
        .as_ref()
        .and_then(|temporal| temporal.item_kind)
        .map_or(ItemKind::Formula, |item_kind| item_kind.kind());
    let protocol_claim = package.graph().protocols().iter().any(|protocol| {
        package
            .graph()
            .protocol_node_keys(&protocol.name)
            .and_then(|keys| keys.clause.as_ref())
            == Some(claim.node())
    });
    let protocol_deadlock = protocol_claim
        && kind == Capability::GlobalConformance
        && item_kind == ItemKind::DeadlockFreedom;
    let protocol_temporal_deadlock = protocol_claim
        && kind == Capability::TemporalSatisfaction
        && item_kind == ItemKind::DeadlockFreedom;
    let applies = protocol_deadlock
        || crate::claim_family::claim_family(package, item_kind, &claim)
            .is_some_and(|family| kind.applies_to_declaration(family));
    let mut backend_options = selection.options.clone().unwrap_or_default();
    let projection_role = if kind == Capability::LocalProjection {
        backend_options.remove("role")
    } else {
        None
    };
    if applies {
        if let Some(refused) = unknown_option(
            index,
            Some(name),
            registry,
            named.as_ref(),
            &backend_options,
        ) {
            return refused;
        }
    }
    let extent = match record {
        Some(record) => record.extent(),
        None if protocol_analysis_kind => ExtentClassification::Unbounded {
            finite_bound_available: false,
        },
        None if kind == Capability::ProbabilisticSatisfaction => package
            .graph()
            .probabilistic_model()
            .claims()
            .iter()
            .find(|checked| checked.occurrence() == Some(&claim))
            .map(|checked| {
                ExtentClassification::of(checked.requirements(claim.node().clone()).extent())
            })
            .unwrap_or(ExtentClassification::Bounded),
        None if kind == Capability::ValueValidity => {
            value_clause_extent(package, name, &node, &claim)
        }
        None => ExtentClassification::Bounded,
    };
    let candidates = match &probabilistic {
        Some(item) => registry.candidates_for_evidence(
            kind,
            &ItemEvidence::Kind(item.evidence),
            named.as_ref(),
        ),
        None => record.map_or_else(
            || registry.candidates(kind, named.as_ref()),
            |record| record.candidates().clone(),
        ),
    };
    let candidates = if let Some(claim) = state_graph {
        registry.restrict_to_state_graph_form(
            registry.restrict_to_options_and_timing(candidates, &backend_options, timed(package)),
            fsl_route::StateGraphFormKind::of(claim.form()),
        )
    } else if kind == Capability::TemporalSatisfaction {
        // FR-243, FR-635: `options.method` is negotiated against each
        // candidate's published values, so `digital-clocks` over a timed
        // subject reaches EN-1's digital-clock route and `zones` EN-6.
        registry.restrict_to_options_and_timing(
            candidates,
            &backend_options,
            claim_timed(package, &claim, binding),
        )
    } else {
        candidates
    };
    // FR-962: an unnamed item the linked tie decides reaches the negotiator
    // with the one provider FR-489's claim-form rule selects.
    let model_subject = !selection
        .temporal
        .as_ref()
        .and_then(|temporal| temporal.subject.as_ref())
        .unwrap_or(subject)
        .initial_states
        .is_empty();
    let candidates = crate::tie_rule::resolve_tie(
        package,
        &claim,
        kind,
        named.as_ref(),
        model_subject,
        candidates,
    );
    let temporal = selection.temporal.clone().unwrap_or_default();
    let mut assessed_root_draft = fsl_replay::protocol_analysis::AssessedRootDraft::default();
    if protocol_analysis_kind {
        if temporal.profile.is_none() {
            return invalid_member(index, name, "temporal.profile");
        }
        if temporal.max_depth.is_none() {
            return invalid_member(index, name, "temporal.max_depth");
        }
        if temporal
            .max_depth
            .is_some_and(|bound| bound.value().is_none())
        {
            return invalid_member_cause(index, name, "temporal.max_depth", "invalid-value");
        }
        if temporal
            .max_depth
            .and_then(RequestCount::value)
            .is_some_and(|bound| bound == 0)
        {
            return invalid_member_cause(index, name, "temporal.max_depth", "invalid-value");
        }
        if temporal
            .profile
            .as_deref()
            .is_some_and(|profile| crate::protocol_analysis::Profile::from_wire(profile).is_none())
        {
            return invalid_member_cause(index, name, "temporal.profile", "invalid-value");
        }
        if kind == Capability::Realizability && temporal.max_history.is_none() {
            return invalid_member(index, name, "temporal.max_history");
        }
        if kind == Capability::Realizability
            && temporal
                .max_history
                .is_some_and(|bound| bound.value().is_none_or(|value| value == 0))
        {
            return invalid_member_cause(index, name, "temporal.max_history", "invalid-value");
        }
        if kind == Capability::LocalProjection && temporal.max_history.is_some() {
            return invalid_member_cause(index, name, "temporal.max_history", "unknown-member");
        }
        if selection.assessed_root.is_none() {
            return invalid_member(index, name, "assessed_root");
        }
        let Some(root_wire) = selection.assessed_root.as_ref() else {
            return invalid_member(index, name, "assessed_root");
        };
        assessed_root_draft =
            match fsl_replay::protocol_analysis::AssessedRootDraft::read_wire(root_wire) {
                Ok(root) => root,
                Err(member) => return invalid_member_cause(index, name, member, "invalid-value"),
            };
        if kind == Capability::LocalProjection && projection_role.is_none() {
            return invalid_member(index, name, "options.role");
        }
    }
    let own_subject = temporal
        .subject
        .clone()
        .map_or_else(|| subject.clone(), |own| own.completed_by(subject));
    let protocol = if protocol_deadlock || protocol_temporal_deadlock || protocol_analysis_kind {
        let owner = package.graph().protocols().iter().find(|protocol| {
            package
                .graph()
                .protocol_node_keys(&protocol.name)
                .and_then(|keys| keys.clause.as_ref())
                == Some(claim.node())
        });
        match (owner, own_subject.protocol.as_ref()) {
            (Some(owner), Some(selected)) if selected.protocol == owner.name => {
                match selected.binding(package, &own_subject.memory) {
                    Some(binding) => Some(binding),
                    None => return invalid_value(index, name, "capacities"),
                }
            }
            (Some(_), _) if protocol_analysis_kind => {
                return invalid_member(index, name, "temporal.subject.protocol");
            }
            (Some(_), _) => return invalid_value(index, name, "temporal.subject.protocol"),
            (None, _) => None,
        }
    } else if kind == Capability::TemporalSatisfaction
        && temporal
            .item_kind
            .is_none_or(|item_kind| item_kind.kind() == ItemKind::Formula)
        && package
            .graph()
            .temporal_clauses()
            .iter()
            .any(|clause| clause.identity() == *claim.node())
    {
        match own_subject.protocol.as_ref() {
            Some(selected) => match selected.binding(package, &own_subject.memory) {
                Some(binding) => Some(binding),
                None => return invalid_value(index, name, "capacities"),
            },
            None => None,
        }
    } else {
        None
    };
    if let Some(claim) = state_graph {
        if let Some(refused) = sampling_refusal(index, name, claim, &temporal) {
            return refused;
        }
    }
    let protocol_bound =
        protocol.is_some() || (kind == Capability::GlobalConformance && protocol_claim);
    let hybrid_claim = kind == Capability::TemporalSatisfaction
        && package
            .graph()
            .hybrid_claim_sites()
            .iter()
            .any(|site| &site.identity == claim.node() && &site.claim == claim.origin());
    let hybrid_limits = match selection.hybrid.as_ref().filter(|_| hybrid_claim) {
        Some(member) => match fsl_replay::SolverBudget::read_hybrid(member) {
            Ok(limits) => limits,
            Err(_) => return invalid_member_cause(index, name, "hybrid", "invalid-value"),
        },
        None => fsl_replay::SolverBudget::default(),
    };
    let form = if hybrid_claim {
        Some(ItemForm::Hybrid(super::HybridItem {
            proof_bounds: Vec::new(),
            limits: hybrid_limits,
            options: backend_options.clone(),
        }))
    } else if let Some(probabilistic) = probabilistic {
        Some(ItemForm::Probabilistic(probabilistic))
    } else if let Some(clause) = (kind == Capability::TemporalSatisfaction)
        .then(|| {
            package
                .graph()
                .finite_hyper_clauses()
                .iter()
                .find(|clause| clause.identity() == Some(claim.node()))
                .cloned()
        })
        .flatten()
    {
        Some(ItemForm::FiniteHyper(super::request::FiniteHyperItem {
            clause,
        }))
    } else if kind == Capability::TemporalSatisfaction
        || kind == Capability::GlobalConformance
        || protocol_analysis_kind
    {
        let mut form = temporal_item(package, &claim, &temporal);
        form.pending_reductions.clone_from(&selection.reductions);
        form.protocol = protocol.clone();
        Some(ItemForm::Temporal(form))
    } else {
        None
    };
    let claim_reads_time = claim_timed(package, &claim, binding);
    let item = ClaimItem {
        request_index: index,
        claim,
        kind,
        extent,
        backend: named,
        candidates: candidate_list(&candidates),
        options: ItemOptions {
            required: selection.required.unwrap_or(true),
            max_cases: selection.check.map(|check| check.limits.max_cases),
            max_task_product_work: temporal
                .limits
                .and_then(|limits| limits.max_task_product_work),
            trace: selection.trace.as_ref().map(DocumentSelection::reference),
            traces: selection.traces.as_ref().map(DocumentSelection::reference),
            recording: selection
                .recording
                .as_ref()
                .map(DocumentSelection::reference),
            population: selection.population.clone(),
            backend_options,
            protocol_analysis: protocol_analysis_kind.then(|| ProtocolAnalysisSelection {
                profile: temporal.profile.clone().expect("profile checked above"),
                max_depth: temporal
                    .max_depth
                    .and_then(RequestCount::value)
                    .expect("valid max_depth checked above"),
                max_history: temporal.max_history.and_then(RequestCount::value),
                role: projection_role.clone(),
                assessed_root: selection.assessed_root.clone(),
                assessed_root_draft,
                protocol: protocol.clone(),
                premises: selection.premises.clone(),
            }),
        },
        form,
    };
    ResolvedItem::Claim(Box::new(ResolvedClaim {
        item,
        claim_name: name.to_owned(),
        candidates,
        unrouted: if state_graph.is_some() {
            selection.unrouted_beyond_sampling()
        } else if hybrid_claim {
            // The `hybrid` member is read by the hybrid item itself.
            ItemSelection {
                hybrid: None,
                ..selection.clone()
            }
            .unrouted_routing_options(true)
        } else {
            selection.unrouted_routing_options(kind == Capability::TemporalSatisfaction)
        } || (selection.reductions.is_some() && kind != Capability::TemporalSatisfaction)
            || reads_no_clocks(claim_reads_time, &temporal)
            || (!protocol_bound
                && temporal
                    .limits
                    .is_some_and(|limits| limits.sets_memory_limits()))
            || !own_subject.same_model(subject),
        subject: own_subject,
    }))
}

/// FR-623: admit the item's pending `reductions` over what the unit declares,
/// so the item runs under FSpec FR-382's wire form and its echo states it. A
/// subject the binding refuses is left for the run to refuse.
fn admit_reductions<'a>(
    input: &fsl_eval::simulation::model::ModelCheckInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: &AnalyzeLimits,
    item: &mut ClaimItem,
) -> Result<(), super::reduction_item::ReductionIssue> {
    let Some(ItemForm::Temporal(form)) = &mut item.form else {
        return Ok(());
    };
    let Some(selection) = form.pending_reductions.take() else {
        return Ok(());
    };
    let simulation = super::request::simulation(input, binding, limits);
    let Ok(system) = fsl_eval::simulation::model::ModelSystem::new(simulation) else {
        form.pending_reductions = Some(selection);
        return Ok(());
    };
    form.reductions = super::reduction_item::admit(&selection, &system, &binding.universes)?;
    Ok(())
}

/// FR-626: a derived item (`deadlock-freedom`, `time-lock-freedom` or
/// `race-freedom`) named by the model alias of its subject, which is the
/// declaration the derived item anchors at (FSpec FR-368 "Claim and
/// anchor"). `None` when the item is no derived item or `name` is no model
/// alias, so the caller settles `unknown-claim`; an alias whose subject
/// derives no such item (a time-lock item over an untimed subject, a
/// deadlock item under `terminal any`) settles `inapplicable-kind`.
fn resolve_derived(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    index: RequestIndex,
    selection: &ItemSelection,
    registry: &Registry,
    binding: &SubjectBinding<'_>,
    name: &str,
) -> Option<ResolvedItem> {
    let temporal = selection.temporal.as_ref()?;
    let kind = temporal.item_kind?.kind();
    if matches!(kind, ItemKind::Formula | ItemKind::Refinement) {
        return None;
    }
    let graph = package.graph();
    let identity = graph.model_package_identity(name)?;
    let root = graph
        .model_roots()
        .find(|(selected, _)| selected.identity == identity)
        .map(|(_, root)| root)?;
    if temporal.max_depth.is_some() {
        // FSpec FR-368-AC-20: a derived item carries no `max_depth`.
        let mut detail = Map::new();
        detail.insert("member".to_owned(), json!("temporal.max_depth"));
        return Some(ResolvedItem::Settled {
            claim_name: Some(name.to_owned()),
            record: Box::new(TerminalRecord::disposition_invalid_request(
                index,
                CatalogCode::new("invalid_runtime_input", "invalid-value"),
                detail,
            )),
        });
    }
    // FSpec FR-368-AC-22: a subject that reads several selections has no
    // anchor for its derived items, so the producer settles the item
    // `unsupported` and writes no request item for it.
    let reads = read_identities(package, binding);
    if fsl_route::deadlock::SubjectReading::read(graph, |identity| reads.contains(identity))
        .anchor()
        .is_err()
    {
        return Some(ResolvedItem::Settled {
            claim_name: Some(name.to_owned()),
            record: Box::new(TerminalRecord::disposition_unsupported(
                index,
                fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
                Map::new(),
            )),
        });
    }
    let derived = derived_items(package, subject, registry, binding, &[], 0)
        .into_iter()
        .find_map(|resolved| match resolved {
            ResolvedItem::Claim(claim)
                if claim.item.claim.node() == &root
                    && matches!(
                        &claim.item.form,
                        Some(ItemForm::Temporal(form)) if form.item_kind == kind
                    ) =>
            {
                Some(claim)
            }
            ResolvedItem::Claim(_) | ResolvedItem::Settled { .. } | ResolvedItem::Simulation(_) => {
                None
            }
        });
    let Some(mut claim) = derived else {
        return Some(invalid_kind(index, Some(name), "inapplicable-kind"));
    };
    let own_subject = temporal
        .subject
        .clone()
        .map_or_else(|| subject.clone(), |own| own.completed_by(subject));
    claim.item.request_index = index;
    let named = selection.backend.as_deref().map(BackendId::new);
    let backend_options = selection.options.clone().unwrap_or_default();
    if let Some(refused) = unknown_option(
        index,
        Some(name),
        registry,
        named.as_ref(),
        &backend_options,
    ) {
        return Some(refused);
    }
    claim.candidates = registry.restrict_to_timing(
        registry.candidates(claim.item.kind, named.as_ref()),
        timed_over(package, &reads),
    );
    claim.item.candidates = candidate_list(&claim.candidates);
    claim.item.backend = named;
    claim.item.options = ItemOptions {
        required: selection.required.unwrap_or(true),
        max_cases: None,
        max_task_product_work: None,
        trace: None,
        traces: None,
        recording: None,
        population: None,
        backend_options,
        protocol_analysis: None,
    };
    let mut form = temporal_item(package, &claim.item.claim, temporal);
    form.pending_reductions.clone_from(&selection.reductions);
    claim.item.form = Some(ItemForm::Temporal(form));
    claim.claim_name = name.to_owned();
    claim.unrouted = selection.unrouted()
        || reads_no_clocks(timed_over(package, &reads), temporal)
        || !own_subject.same_model(subject);
    claim.subject = own_subject;
    Some(ResolvedItem::Claim(claim))
}

/// FR-246, FR-633: a `schedulable` or `response` claim named by its declared
/// name. A task-set claim's node is minted beside the semantic graph (FSpec
/// FR-482), so its name resolves through the checked task sets and its
/// record's occurrence, not through [`declarations`]. `None` when `name` is
/// no task-set claim.
fn resolve_schedule_claim(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    index: RequestIndex,
    selection: &ItemSelection,
    registry: &Registry,
    name: &str,
) -> Option<ResolvedItem> {
    let claim = package
        .graph()
        .task_sets()
        .claims()
        .iter()
        .find(|checked| checked.name == name)?
        .occurrence()?;
    if selection
        .kind
        .as_deref()
        .is_some_and(|label| label != Capability::Schedulability.to_wire())
    {
        return Some(invalid_kind(index, Some(name), "inapplicable-kind"));
    }
    let named = selection.backend.as_deref().map(BackendId::new);
    let backend_options = selection.options.clone().unwrap_or_default();
    if let Some(refused) = unknown_option(
        index,
        Some(name),
        registry,
        named.as_ref(),
        &backend_options,
    ) {
        return Some(refused);
    }
    let records = requirement_items(package, registry, named.as_ref());
    let candidates = records
        .iter()
        .find(|record| record.occurrence() == &claim)
        .map_or_else(
            || registry.candidates(Capability::Schedulability, named.as_ref()),
            |record| record.candidates().clone(),
        );
    let candidates = crate::tie_rule::resolve_tie(
        package,
        &claim,
        Capability::Schedulability,
        named.as_ref(),
        false,
        candidates,
    );
    Some(ResolvedItem::Claim(Box::new(ResolvedClaim {
        item: ClaimItem {
            request_index: index,
            claim,
            kind: Capability::Schedulability,
            extent: ExtentClassification::Bounded,
            backend: named,
            candidates: candidate_list(&candidates),
            options: ItemOptions {
                required: selection.required.unwrap_or(true),
                max_cases: None,
                max_task_product_work: selection
                    .temporal
                    .as_ref()
                    .and_then(|temporal| temporal.limits)
                    .and_then(|limits| limits.max_task_product_work),
                trace: None,
                traces: None,
                recording: None,
                population: selection.population.clone(),
                backend_options,
                protocol_analysis: None,
            },
            form: None,
        },
        claim_name: name.to_owned(),
        candidates,
        subject: subject.clone(),
        unrouted: selection.unrouted(),
    })))
}

/// A request member the item cannot read: the item settles `declined`
/// `invalid-input` `invalid_runtime_input`/`invalid-value` naming `member`.
fn invalid_value(index: RequestIndex, name: &str, member: &str) -> ResolvedItem {
    let mut detail = Map::new();
    detail.insert("member".to_owned(), json!(member));
    ResolvedItem::Settled {
        claim_name: Some(name.to_owned()),
        record: Box::new(
            TerminalRecord::disposition_declined(
                index,
                fsl_replay::ProofRefusalCause::InvalidInput,
                fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                CatalogCode::new("invalid_runtime_input", "invalid-value"),
            )
            .with_diagnostic_detail(detail),
        ),
    }
}

/// A required protocol-analysis request member was absent.
fn invalid_member(index: RequestIndex, name: &str, member: &str) -> ResolvedItem {
    invalid_member_cause(index, name, member, "missing-member")
}

fn invalid_member_cause(
    index: RequestIndex,
    name: &str,
    member: &str,
    cause: &'static str,
) -> ResolvedItem {
    let mut detail = Map::new();
    detail.insert("member".to_owned(), json!(member));
    ResolvedItem::Settled {
        claim_name: Some(name.to_owned()),
        record: Box::new(TerminalRecord::disposition_invalid_request(
            index,
            CatalogCode::new("invalid_runtime_input", cause),
            detail,
        )),
    }
}

/// FSL FR-1033 and FR-730: a protocol freedom or loop-proof item whose
/// `claim` names the scoped control path is decided over that protocol.
/// `None` when the selection names no eligible control.
fn resolve_protocol_freedom(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    index: RequestIndex,
    selection: &ItemSelection,
    registry: &Registry,
    name: &str,
) -> Option<ResolvedItem> {
    let temporal = selection.temporal.as_ref()?;
    let kind = temporal.item_kind?.kind();
    let owner = name.split("::").next()?;
    let checked = package.graph().protocol(owner)?;
    let repeat = match kind {
        ItemKind::RaceFreedom => {
            let outermost =
                fsl_eval::simulation::protocol::ResolvedMemory::resolve(checked, &[]).ok()?;
            if !outermost
                .parallels()
                .iter()
                .any(|parallel| parallel.parallel == name)
            {
                return None;
            }
            None
        }
        ItemKind::LoopInvariant | ItemKind::LoopTermination => {
            let (_, path) = name.split_once("::")?;
            let path = path.split("::").collect::<Vec<_>>();
            let control = checked.controls.at_path(&path)?;
            let repeat = checked
                .slots
                .repeats
                .iter()
                .find(|repeat| repeat.node == control.node && repeat.loop_proof.is_some())?;
            if kind == ItemKind::LoopTermination && repeat.max.is_some() {
                return Some(invalid_kind(index, Some(name), "inapplicable-kind"));
            }
            if temporal.max_depth.is_some() {
                return Some(invalid_value(index, name, "temporal.max_depth"));
            }
            Some(repeat.node)
        }
        ItemKind::Formula
        | ItemKind::DeadlockFreedom
        | ItemKind::TimeLockFreedom
        | ItemKind::Refinement => return None,
    };
    let own_subject = temporal
        .subject
        .clone()
        .map_or_else(|| subject.clone(), |own| own.completed_by(subject));
    let Some(selected) = own_subject.protocol.as_ref() else {
        let mut detail = Map::new();
        detail.insert("member".to_owned(), json!("temporal.subject.protocol"));
        return Some(ResolvedItem::Settled {
            claim_name: Some(name.to_owned()),
            record: Box::new(
                TerminalRecord::disposition_declined(
                    index,
                    fsl_replay::ProofRefusalCause::InvalidInput,
                    fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                    CatalogCode::new("invalid_runtime_input", "missing-member"),
                )
                .with_diagnostic_detail(detail),
            ),
        });
    };
    if selected.protocol != owner {
        return Some(unknown_claim(index, name));
    }
    let Some(binding) = selected.binding(package, &own_subject.memory) else {
        return Some(invalid_value(index, name, "capacities"));
    };
    let Some(node) = checked.node() else {
        return Some(invalid_kind(index, Some(name), "inapplicable-kind"));
    };
    let named = selection.backend.as_deref().map(BackendId::new);
    let backend_options = selection.options.clone().unwrap_or_default();
    if let Some(refused) = unknown_option(
        index,
        Some(name),
        registry,
        named.as_ref(),
        &backend_options,
    ) {
        return Some(refused);
    }
    let declaration = OccurrenceKey::declaration(node);
    let claim = match repeat {
        Some(repeat) => fsl_route::deadlock::ProtocolTemporalItem {
            occurrence: &declaration,
            subject: &binding,
            protocol: checked,
            models: &binding.effective,
            non_atomic_access: false,
        }
        .reading()
        .protocol_loop_anchor(repeat)?,
        None => declaration,
    };
    let candidates = registry.restrict_to_timing(
        registry.candidates(Capability::TemporalSatisfaction, named.as_ref()),
        false,
    );
    let mut form = temporal_item(package, &claim, temporal);
    form.protocol = Some(binding);
    Some(ResolvedItem::Claim(Box::new(ResolvedClaim {
        item: ClaimItem {
            request_index: index,
            claim,
            kind: Capability::TemporalSatisfaction,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: false,
            },
            backend: named,
            candidates: candidate_list(&candidates),
            options: ItemOptions {
                required: selection.required.unwrap_or(true),
                max_cases: None,
                max_task_product_work: None,
                trace: None,
                traces: selection.traces.as_ref().map(DocumentSelection::reference),
                recording: None,
                population: selection.population.clone(),
                backend_options,
                protocol_analysis: None,
            },
            form: Some(ItemForm::Temporal(form)),
        },
        claim_name: name.to_owned(),
        candidates,
        subject: own_subject.clone(),
        unrouted: selection.unrouted() || !own_subject.same_model(subject),
    })))
}

/// Whether the item sets `max_clocks`, which only a timed subject reads
/// (FSpec FR-415): on an untimed subject no route reads it.
fn reads_no_clocks(timed: bool, temporal: &TemporalSelection) -> bool {
    !timed
        && temporal
            .limits
            .is_some_and(|limits| limits.max_clocks.is_some())
}

/// Whether the model subject the package's selections give is timed
/// (FR-243): a selected model declares a time source.
pub(super) fn timed(package: &CheckedPackage) -> bool {
    timed_over(package, &model_identities(package))
}

/// Whether the clause `claim` reads a timed model (FR-243): a model selection
/// its subject reads (the models its universes cover, every selection when
/// they cover none, as derived items read them) declares a time source. A
/// clause over an untimed model whose subject reads no timed model stays on
/// EN-1 beside a timed model it does not read; one whose subject also reads a
/// timed model is timed. A clause whose `over` type is no selection of the
/// package, or that is no clause, reads every selection.
pub(super) fn claim_timed(
    package: &CheckedPackage,
    claim: &OccurrenceKey,
    binding: &SubjectBinding<'_>,
) -> bool {
    let owner = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|clause| &OccurrenceKey::new(clause.identity(), clause.claim().clone()) == claim)
        .and_then(|clause| match &clause.clause().over().1 {
            filament_exact::ValueType::Reference(object) => {
                Some(object.original().package().to_owned())
            }
            _ => None,
        });
    let selected = owner
        .as_deref()
        .is_some_and(|owner| model_identities(package).contains(owner));
    claim_routing(
        selected,
        || timed_over(package, &read_identities(package, binding)),
        || timed(package),
    )
}

/// The routing decision of [`claim_timed`]: a clause over a model the package
/// selects is timed when a model its subject reads is (`reads_timed`); any
/// other clause reads every selection, so it is timed when the package is
/// (`package_timed`).
fn claim_routing(
    owner_selected: bool,
    reads_timed: impl FnOnce() -> bool,
    package_timed: impl FnOnce() -> bool,
) -> bool {
    if owner_selected {
        reads_timed()
    } else {
        package_timed()
    }
}

#[cfg(test)]
mod claim_routing_tests {
    use super::claim_routing;

    /// QSL-2654, FR-243: a clause over an untimed model whose subject reads a
    /// timed model is timed, whatever the package says of the models it does
    /// not read; one whose subject reads only untimed models is not, though
    /// the package holds a timed one.
    #[test]
    fn a_subject_reading_a_timed_model_is_timed_whatever_its_over_model() {
        assert!(claim_routing(true, || true, || false));
        assert!(claim_routing(true, || true, || true));
        assert!(!claim_routing(true, || false, || true));
        assert!(!claim_routing(true, || false, || false));
    }

    /// QSL-2654: a clause whose `over` type is no model selection reads every
    /// selection, so it is timed exactly when the package is, and never reads
    /// the subject's models.
    #[test]
    fn a_clause_over_no_selected_model_falls_back_to_the_package() {
        assert!(claim_routing(
            false,
            || panic!("the subject's reads are not consulted"),
            || true
        ));
        assert!(!claim_routing(
            false,
            || panic!("the subject's reads are not consulted"),
            || false
        ));
    }
}

/// [`timed`] for a subject that reads the model selections `reads` holds:
/// only a model it reads can make it timed (FR-243).
fn timed_over(package: &CheckedPackage, reads: &BTreeSet<&str>) -> bool {
    fsl_route::deadlock::SubjectReading::read(package.graph(), |identity| reads.contains(identity))
        .timed()
}

/// The identity of every model selection of `package`.
fn model_identities(package: &CheckedPackage) -> BTreeSet<&str> {
    package
        .graph()
        .model_selections()
        .iter()
        .map(|selection| selection.identity.as_str())
        .collect()
}

/// The identities of the model selections the subject `binding` reads: those
/// whose domain package one of its universes covers (FR-124-AC-12, FSpec
/// FR-368-AC-22). A subject whose universes cover none reads every selection,
/// as [`fsl_eval::simulation::model::ModelSystem::new_over_read_identities`]
/// reads it.
fn read_identities<'p>(
    package: &'p CheckedPackage,
    binding: &SubjectBinding<'_>,
) -> BTreeSet<&'p str> {
    let all = model_identities(package);
    let read: BTreeSet<&str> = all
        .iter()
        .copied()
        .filter(|identity| {
            binding
                .universes
                .iter()
                .any(|universe| universe.population.package == *identity)
        })
        .collect();
    if read.is_empty() {
        all
    } else {
        read
    }
}

fn candidate_list(candidates: &CandidateOutcome) -> Vec<fsl_route::Candidate> {
    match candidates {
        CandidateOutcome::Candidates(set) => set.candidates().to_vec(),
        CandidateOutcome::UnknownBackend(_) => Vec::new(),
    }
}

/// The temporal form of an item over `claim`: the clause's resolved
/// fairness, and each limit the item sets over the published default.
fn temporal_item(
    package: &CheckedPackage,
    claim: &OccurrenceKey,
    selection: &TemporalSelection,
) -> TemporalItem {
    let mut item = temporal_settings(selection);
    if item.item_kind == ItemKind::Formula {
        if let Some(clause) = package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|clause| clause.identity() == *claim.node())
        {
            item.fairness = clause.clause().fairness().to_vec();
        }
    }
    item
}

/// The temporal form an item's `selection` sets: its item kind and each
/// limit it sets over the published default, with no fairness, which the
/// claim's clause resolves.
fn temporal_settings(selection: &TemporalSelection) -> TemporalItem {
    let mut item = TemporalItem {
        item_kind: selection
            .item_kind
            .map_or(ItemKind::Formula, ItemKindSelection::kind),
        stutter_closure: selection.stutter_closure,
        // Protocol-analysis items validate their bound before reaching an
        // engine; other items retain the existing `usize` conversion rule.
        max_depth: selection
            .max_depth
            .and_then(RequestCount::value)
            .and_then(|depth| usize::try_from(depth).ok()),
        witness_samples: selection.witness_samples,
        seed: selection.seed_count(),
        abstract_initial: selection.abstract_subject.as_ref().map(|subject| {
            subject
                .initial_states
                .iter()
                .map(DocumentSelection::reference)
                .collect()
        }),
        ..TemporalItem::default()
    };
    if let Some(limits) = &selection.limits {
        let set = |value: Option<NonZeroU64>, slot: &mut u64| {
            if let Some(value) = value {
                *slot = value.get();
            }
        };
        // The reader admits only counts that fit `usize` (`count`).
        let set_usize = |value: Option<NonZeroU64>, slot: &mut usize| {
            if let Some(value) = value {
                *slot = usize::try_from(value.get()).unwrap_or(usize::MAX);
            }
        };
        set_usize(limits.max_states, &mut item.limits.limits.max_states);
        set_usize(
            limits.max_transitions,
            &mut item.limits.limits.max_transitions,
        );
        set(
            limits.max_automaton_states,
            &mut item.limits.max_automaton_states,
        );
        set(limits.max_walk_steps, &mut item.limits.max_walk_steps);
        item.max_clocks = limits.max_clocks;
        item.max_store_buffer = limits.max_store_buffer.map(NonZeroU64::get);
        item.max_messages = limits.max_messages.map(NonZeroU64::get);
        set(limits.max_witness_set, &mut item.limits.max_witness_set);
        set(
            limits.max_relation_tuples,
            &mut item.limits.max_relation_tuples,
        );
        set(limits.max_candidates, &mut item.max_candidates);
    }
    item
}

/// FR-620's default items: one per checked claim and kind, the claims in
/// source order across every declaration family, followed by the derived
/// items of the model subject.
fn default_items(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    registry: &Registry,
    units: &[RawSourceRef],
    binding: &SubjectBinding<'_>,
    limits: &AnalyzeLimits,
) -> Vec<ResolvedItem> {
    let graph = package.graph();
    let records = requirement_items(package, registry, None);
    let mut nodes: Vec<filament_exact::NodeKey> = graph
        .temporal_clauses()
        .iter()
        .map(|clause| clause.identity())
        .collect();
    for record in &records {
        if !nodes.contains(record.node()) {
            nodes.push(record.node().clone());
        }
    }
    let position = |node: &filament_exact::NodeKey| -> (usize, u64) {
        let region = claim_occurrence(package, &records, node, None)
            .and_then(|occurrence| {
                graph.occurrence_region(occurrence.node().clone(), occurrence.origin())
            })
            .map(|region| (region.source().clone(), region.start()))
            .or_else(|| {
                // A temporal clause's own declaration span lies in the
                // program unit when its occurrence records no region.
                let clause = graph
                    .temporal_clauses()
                    .iter()
                    .find(|clause| clause.identity() == *node)?;
                let start = u64::try_from(clause.clause().span().start).ok()?;
                Some((units.first()?.clone(), start))
            });
        region.map_or((usize::MAX, u64::MAX), |(source, start)| {
            let unit = units
                .iter()
                .position(|candidate| *candidate == source)
                .unwrap_or(usize::MAX);
            (unit, start)
        })
    };
    // FR-620: source order across every declaration family, the units in
    // request order and each unit's declarations by the start byte of their
    // span; the sort is stable, so a node with no recorded region keeps its
    // place after those with one.
    nodes.sort_by_cached_key(|node| position(node));
    let mut resolved = Vec::new();
    for node in &nodes {
        let Some(name) = declared_name(package, node) else {
            continue;
        };
        let kinds: Vec<Capability> = records
            .iter()
            .filter(|record| record.node() == node)
            .map(|record| record.kind())
            .collect();
        for kind in Capability::ALL.iter().filter(|kind| kinds.contains(kind)) {
            let selection = ItemSelection {
                claim: Some(name.clone()),
                kind: Some(kind.to_wire().to_owned()),
                ..ItemSelection::default()
            };
            resolved.push(resolve_one(
                package,
                subject,
                RequestIndex::new(resolved.len()),
                &selection,
                registry,
                binding,
            ));
        }
    }
    // FR-246, FR-620: a task-set claim's record is a default item too, the
    // claims in source order.
    for claim in graph.task_sets().claims() {
        let selection = ItemSelection {
            claim: Some(claim.name.clone()),
            kind: Some(Capability::Schedulability.to_wire().to_owned()),
            ..ItemSelection::default()
        };
        resolved.push(resolve_one(
            package,
            subject,
            RequestIndex::new(resolved.len()),
            &selection,
            registry,
            binding,
        ));
    }
    let derived = if subject.protocol.is_some() {
        protocol_derived_items(
            package,
            subject,
            registry,
            (binding, limits),
            &resolved,
            resolved.len(),
        )
    } else {
        derived_items(
            package,
            subject,
            registry,
            binding,
            &resolved,
            resolved.len(),
        )
    };
    resolved.extend(derived);
    resolved
}

/// FSpec FR-368's derived items of the subject the authored temporal items
/// of `authored` are over, numbered from `first`.
fn derived_items(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    registry: &Registry,
    binding: &SubjectBinding<'_>,
    authored: &[ResolvedItem],
    first: usize,
) -> Vec<ResolvedItem> {
    let graph = package.graph();
    // FR-124-AC-12: the subject's items are over the models its universes
    // read, not over every model the unit selects.
    let selections = read_identities(package, binding);
    let timed = timed_over(package, &selections);
    let mut request = fsl_route::model_request::ModelRequest::<()>::new();
    // FSpec FR-368: a model subject has its derived items whether or not the
    // unit authors a clause, so the subject is joined to the derivation
    // before any authored item is written.
    request.state_graph(
        &fsl_route::deadlock::ModelStateGraphItem {
            subject: &(),
            package: graph,
        },
        &selections,
    );
    let occurrences: Vec<OccurrenceKey> = authored
        .iter()
        .filter_map(|resolved| match resolved {
            ResolvedItem::Claim(claim)
                if claim.item.kind == Capability::TemporalSatisfaction
                    && graph
                        .temporal_clauses()
                        .iter()
                        .any(|clause| clause.identity() == *claim.item.claim.node()) =>
            {
                Some(claim.item.claim.clone())
            }
            ResolvedItem::Claim(_) | ResolvedItem::Settled { .. } | ResolvedItem::Simulation(_) => {
                None
            }
        })
        .collect();
    let requirements = fsl_semantics::family::Requirements::new(
        Capability::TemporalSatisfaction,
        fsl_semantics::family::ClaimExtent::Bounded,
    );
    for occurrence in &occurrences {
        request.authored(
            &fsl_route::deadlock::ModelTemporalItem {
                occurrence,
                subject: &(),
                package: graph,
            },
            &requirements,
            &selections,
        );
    }
    let written = request.derived(
        registry,
        None,
        &fsl_route::request::DerivedLimits::default(),
    );
    written
        .items()
        .iter()
        .filter(|item| {
            item.item_kind()
                .is_some_and(|kind| kind != ItemKind::Formula)
        })
        .enumerate()
        .map(|(offset, item)| {
            derived_claim(
                package,
                subject,
                registry,
                item,
                (RequestIndex::new(first + offset), timed),
                None,
            )
        })
        .collect()
}

/// The resolved claim of the derived request item `item` over `subject`, at
/// the request index and with the timing of the subject the first member of
/// `(index, timed)` and its second give, run over the protocol subject
/// `protocol` when it has one.
fn derived_claim(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    registry: &Registry,
    item: &fsl_route::request::RequestItem,
    (index, timed): (RequestIndex, bool),
    protocol: Option<&super::request::ProtocolBinding>,
) -> ResolvedItem {
    let candidates = registry.restrict_to_timing(
        item.candidates()
            .cloned()
            .unwrap_or_else(|| registry.candidates(item.kind(), None)),
        timed,
    );
    let claim_name = declared_name(package, item.node()).unwrap_or_else(|| {
        protocol
            .map(|binding| binding.protocol.clone())
            .unwrap_or_default()
    });
    ResolvedItem::Claim(Box::new(ResolvedClaim {
        item: ClaimItem {
            request_index: index,
            claim: item.occurrence().clone(),
            kind: item.kind(),
            extent: item.extent(),
            backend: None,
            candidates: candidate_list(&candidates),
            options: ItemOptions {
                required: true,
                max_cases: None,
                max_task_product_work: None,
                trace: None,
                traces: None,
                recording: None,
                population: None,
                backend_options: BTreeMap::new(),
                protocol_analysis: None,
            },
            form: Some(ItemForm::Temporal(TemporalItem {
                item_kind: item.item_kind().unwrap_or(ItemKind::Formula),
                protocol: protocol.cloned(),
                ..TemporalItem::default()
            })),
        },
        claim_name,
        candidates,
        subject: subject.clone(),
        unrouted: false,
    }))
}

/// FSL FR-1033 and FSpec FR-429, FR-438: the derived items of the protocol
/// subject `subject` binds, written beside the authored items of `authored`
/// that are decided over it: a `deadlock-freedom` item, and, when a resolved
/// model is weak and an access is non-atomic, a `race-freedom` item.
fn protocol_derived_items(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    registry: &Registry,
    (admission, limits): (&SubjectBinding<'_>, &AnalyzeLimits),
    authored: &[ResolvedItem],
    first: usize,
) -> Vec<ResolvedItem> {
    let graph = package.graph();
    let Some(binding) = subject
        .protocol
        .as_ref()
        .and_then(|selected| selected.binding(package, &subject.memory))
    else {
        return Vec::new();
    };
    let Some(checked) = graph.protocol(&binding.protocol) else {
        return Vec::new();
    };
    let non_atomic =
        super::protocol_item::has_non_atomic_access(package, admission, limits, &binding);
    let occurrences: Vec<OccurrenceKey> = authored
        .iter()
        .filter_map(|resolved| match resolved {
            ResolvedItem::Claim(claim)
                if matches!(
                    &claim.item.form,
                    Some(ItemForm::Temporal(form)) if form.protocol.as_ref() == Some(&binding)
                        && form.item_kind == ItemKind::Formula
                ) =>
            {
                Some(claim.item.claim.clone())
            }
            ResolvedItem::Claim(_) | ResolvedItem::Settled { .. } | ResolvedItem::Simulation(_) => {
                None
            }
        })
        .collect();
    let requirements = fsl_semantics::family::Requirements::new(
        Capability::TemporalSatisfaction,
        fsl_semantics::family::ClaimExtent::Bounded,
    );
    let mut request =
        fsl_route::model_request::ModelRequest::<super::request::ProtocolBinding>::new();
    for occurrence in &occurrences {
        request.protocol(
            &fsl_route::deadlock::ProtocolTemporalItem {
                occurrence,
                subject: &binding,
                protocol: checked,
                models: &binding.effective,
                non_atomic_access: non_atomic,
            },
            &requirements,
        );
    }
    let written = request.derived(
        registry,
        None,
        &fsl_route::request::DerivedLimits::default(),
    );
    written
        .items()
        .iter()
        .filter(|item| {
            item.item_kind()
                .is_some_and(|kind| kind != ItemKind::Formula)
        })
        .enumerate()
        .map(|(offset, item)| {
            derived_claim(
                package,
                subject,
                registry,
                item,
                (RequestIndex::new(first + offset), timed(package)),
                Some(&binding),
            )
        })
        .collect()
}

impl ClaimItem {
    /// FR-620's canonical FSpec FR-331 request item of this item over
    /// `subject`: `request_index`, the `claim` occurrence key, the `kind`,
    /// the `extent`, the `candidates`, a named `backend`, and for a temporal
    /// item its `subject`, `item_kind`, resolved `fairness`, `max_depth`
    /// and every `limits` member made explicit.
    ///
    /// # Errors
    ///
    /// When the claim's node key or a fairness constraint has no wire
    /// spelling.
    pub fn request_item_wire(
        &self,
        subject: &SubjectSelection,
    ) -> Result<Value, serde_json::Error> {
        let mut item = Map::new();
        item.insert("request_index".to_owned(), json!(self.request_index.get()));
        item.insert("kind".to_owned(), json!(self.kind.to_wire()));
        item.insert(
            "claim".to_owned(),
            fsl_replay::claim_locus_wire(&self.claim)?,
        );
        item.insert(
            "extent".to_owned(),
            match self.extent {
                ExtentClassification::Bounded => {
                    json!({"classification": "bounded", "bounds": []})
                }
                ExtentClassification::Unbounded {
                    finite_bound_available,
                } => json!({"classification": "unbounded",
                    "finite_bound_available": finite_bound_available}),
            },
        );
        item.insert(
            "candidates".to_owned(),
            json!({"type": "set", "backends": self
                .candidates
                .iter()
                .map(|candidate| candidate.id().as_str())
                .collect::<Vec<_>>()}),
        );
        if let Some(backend) = &self.backend {
            item.insert("backend".to_owned(), json!(backend.as_str()));
        }
        if let Some(trace) = &self.options.trace {
            item.insert(
                "trace".to_owned(),
                json!({"authority": trace.authority, "identity": trace.identity}),
            );
        }
        if let Some(traces) = &self.options.traces {
            item.insert(
                "traces".to_owned(),
                json!({"authority": traces.authority, "identity": traces.identity}),
            );
        }
        if let Some(recording) = &self.options.recording {
            item.insert(
                "recording".to_owned(),
                json!({"authority": recording.authority, "identity": recording.identity}),
            );
        }
        if let Some(population) = &self.options.population {
            item.insert("population".to_owned(), population.clone());
        }
        if let Some(max_cases) = self.options.max_cases {
            item.insert(
                "check".to_owned(),
                json!({"limits": {"max_cases": max_cases.get()}}),
            );
        }
        if let Some(ItemForm::Hybrid(hybrid)) = &self.form {
            if !hybrid.options.is_empty() {
                item.insert("options".to_owned(), json!(hybrid.options));
            }
        }
        if let Some(protocol) = &self.options.protocol_analysis {
            if let Some(root) = &protocol.assessed_root {
                item.insert("assessed_root".to_owned(), root.clone());
            }
            let mut options = self.options.backend_options.clone();
            if let Some(role) = &protocol.role {
                options.insert("role".to_owned(), role.clone());
            }
            if !options.is_empty() {
                item.insert("options".to_owned(), json!(options));
            }
        }
        if let Some(ItemForm::Temporal(temporal)) = &self.form {
            let mut member = Map::new();
            member.insert("subject".to_owned(), subject.wire());
            member.insert("item_kind".to_owned(), json!(temporal.item_kind.to_wire()));
            if let Some(protocol) = &self.options.protocol_analysis {
                member.insert("profile".to_owned(), json!(protocol.profile));
                if let Some(max_history) = protocol.max_history {
                    member.insert("max_history".to_owned(), json!(max_history));
                }
            }
            member.insert(
                "fairness".to_owned(),
                Value::Array(
                    temporal
                        .fairness
                        .iter()
                        .map(fairness_wire)
                        .collect::<Result<_, _>>()?,
                ),
            );
            if let Some(protocol) = &temporal.protocol {
                item.insert(
                    "memory".to_owned(),
                    Value::Array(
                        protocol
                            .effective
                            .iter()
                            .map(|(parallel, model)| {
                                json!({"parallel": parallel, "model": model.label()})
                            })
                            .collect(),
                    ),
                );
            }
            if !temporal.reductions.is_empty() {
                item.insert(
                    "reductions".to_owned(),
                    super::reduction_item::wire(&temporal.reductions),
                );
            }
            if let Some(stutter) = temporal.stutter_closure {
                member.insert("stutter_closure".to_owned(), json!(stutter));
            }
            if let Some(depth) = temporal.max_depth {
                member.insert("max_depth".to_owned(), json!(depth));
            }
            if let Some(samples) = temporal.witness_samples {
                member.insert("witness_samples".to_owned(), json!(samples));
            }
            if let Some(seed) = temporal.seed {
                member.insert("seed".to_owned(), json!(seed));
            }
            let limits = &temporal.limits;
            member.insert(
                "limits".to_owned(),
                json!({
                    "max_states": limits.limits.max_states,
                    "max_transitions": limits.limits.max_transitions,
                    "max_automaton_states": limits.max_automaton_states,
                    "max_walk_steps": limits.max_walk_steps,
                    "max_witness_set": limits.max_witness_set,
                    "max_relation_tuples": limits.max_relation_tuples,
                }),
            );
            item.insert("temporal".to_owned(), Value::Object(member));
        } else if self.kind == Capability::Schedulability {
            if let Some(max_work) = self.options.max_task_product_work {
                item.insert(
                    "temporal".to_owned(),
                    json!({"limits": {"max_task_product_work": max_work.get()}}),
                );
            }
        }
        if let Some(ItemForm::Probabilistic(probabilistic)) = &self.form {
            let mut member = Map::new();
            member.insert(
                "evidence".to_owned(),
                json!(probabilistic.evidence.to_wire()),
            );
            match probabilistic.evidence {
                EvidenceKind::Statistical => {
                    let method = match probabilistic.method {
                        Some(fsl_replay::statistical::StatisticalMethod::Okamoto) => {
                            json!({"type": "okamoto"})
                        }
                        Some(fsl_replay::statistical::StatisticalMethod::Sprt) => {
                            json!({"type": "sprt"})
                        }
                        Some(fsl_replay::statistical::StatisticalMethod::ClopperPearson) => {
                            json!({"type": "clopper-pearson"})
                        }
                        Some(fsl_replay::statistical::StatisticalMethod::Regenerative {
                            min_cycles,
                        }) => json!({"type": "regenerative", "min_cycles": min_cycles}),
                        None => {
                            return Err(serde::ser::Error::custom("missing statistical method"));
                        }
                    };
                    member.insert("method".to_owned(), method);
                    member.insert(
                        "seed".to_owned(),
                        json!(probabilistic.seed.unwrap_or_default().to_string()),
                    );
                    member.insert(
                        "max_witnesses".to_owned(),
                        json!(probabilistic
                            .max_witnesses
                            .unwrap_or(crate::statistical::DEFAULT_MAX_WITNESSES)),
                    );
                    member.insert(
                        "workers".to_owned(),
                        json!(probabilistic.workers.map_or(1, NonZeroU32::get)),
                    );
                    let timed_sampling = probabilistic.timed_sampling.unwrap_or_default();
                    member.insert(
                        "timed_sampling".to_owned(),
                        json!({
                            "q": timed_sampling.q,
                            "max_sample_steps": timed_sampling.max_sample_steps,
                        }),
                    );
                }
                EvidenceKind::Exact => {}
            }
            // FR-741: FSL writes every budget the caller left out, at its
            // published default.
            let limits = match probabilistic.limits {
                super::ProbabilisticLimits::Statistical(limits) => json!({
                    "max_samples": limits.max_samples,
                    "max_draws": limits.max_draws,
                    "max_cycle_steps": limits.max_cycle_steps,
                }),
                super::ProbabilisticLimits::Exact(limits) => json!({
                    "max_states": limits.model.limits.max_states,
                    "max_transitions": limits.model.limits.max_transitions,
                    "max_automaton_states": limits.model.max_automaton_states,
                    "max_iterations": limits.max_iterations,
                    "precision_bits": limits.precision_bits,
                    "max_precision_bits": limits.max_precision_bits,
                    "max_rational_bits": limits.max_rational_bits,
                    "max_policy_iterations": limits.max_policy_iterations,
                }),
            };
            member.insert("limits".to_owned(), limits);
            item.insert("probabilistic".to_owned(), Value::Object(member));
        }
        Ok(Value::Object(item))
    }
}

/// FSpec FR-364's `{kind, operation, granularity}` fairness constraint.
///
/// # Errors
///
/// When the constraint has no canonical spelling, which the caller reports
/// as a fault rather than writing a placeholder entry.
fn fairness_wire(
    constraint: &filament_semantic_value::fairness::FairnessConstraint,
) -> Result<Value, serde_json::Error> {
    let bytes = quire_canonical::to_vec(constraint, quire_canonical::Limits::new(u64::MAX))
        .map_err(|error| <serde_json::Error as serde::ser::Error>::custom(error.to_string()))?;
    serde_json::from_slice(&bytes)
}

/// What one request settles for the FR-621 document.
pub struct AnalyzeRun<'a> {
    /// The checked package.
    pub package: &'a CheckedPackage,
    /// The request's shared subject binding.
    pub binding: SubjectBinding<'a>,
    /// Source bytes under every label the package names.
    pub provision: SourceProvision,
    /// The resolved items, in request order.
    pub items: Vec<ResolvedItem>,
    /// The engines `analyze` may run.
    pub engines: &'a Engines,
    /// The shared limits.
    pub limits: AnalyzeLimits,
}

/// Why no FR-621 document could be written.
#[derive(Debug, thiserror::Error)]
pub enum AnalyzeDocumentError {
    /// The shared subject or source provision was refused before any item
    /// ran (FR-624).
    #[error(transparent)]
    Refused(#[from] super::AnalyzeRefusal),
    /// An internal fault stopped the run.
    #[error("the analyze run faulted")]
    Fault,
}

/// Negotiate `run`'s items with `negotiate`, analyze them under `cancel`
/// and write FR-621's `filament-outcome/1` document with its FR-285 exit
/// code.
///
/// # Errors
///
/// [`AnalyzeDocumentError::Refused`] for a whole-request refusal (the
/// caller writes FR-624's refusal document), and
/// [`AnalyzeDocumentError::Fault`] for an internal fault.
pub fn analyze_document(
    run: AnalyzeRun<'_>,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> Result<(Value, u8), AnalyzeDocumentError> {
    settle_run(run, negotiate, cancel).map(|settled| (settled.document, settled.exit_code))
}

/// One FR-635 `registry` entry: a registered backend and its FSpec FR-331
/// tool record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryEntry {
    /// The backend.
    pub backend: BackendId,
    /// Its tool record.
    pub tool: super::BackendTool,
}

impl RegistryEntry {
    /// The entry's `{backend, tool}` wire spelling.
    pub fn wire(&self) -> Value {
        json!({"backend": self.backend.as_str(),
            "tool": {"name": self.tool.name(), "version": self.tool.version()}})
    }
}

/// What [`settle_run`] settled: the typed records and reproduced
/// counterexamples, each item's FR-621 entry, the registry, the FR-621
/// document over them and its FR-285 exit code.
pub struct SettledRun {
    /// One terminal record per item, in request-index order.
    pub records: Vec<TerminalRecord>,
    /// Exactly one reproduced counterexample per `refuted` record.
    pub counterexamples: Vec<super::ItemCounterexample>,
    /// Each item's FR-621 entry, in [`Self::records`] order.
    pub items: Vec<Value>,
    /// FR-635's registry, sorted by backend.
    pub registry: Vec<RegistryEntry>,
    /// The FR-621 document.
    pub document: Value,
    /// The most severe item's FR-285 exit code; 0 with no item.
    pub exit_code: u8,
}

impl SettledRun {
    /// The canonical RFC 8785 bytes of the FR-621 document.
    pub fn to_bytes(&self) -> Result<Vec<u8>, fsl_foundation::canonical_json::CanonicalJsonError> {
        fsl_foundation::canonical_json::encode(&self.document)
    }
}

/// Negotiate `run`'s items with `negotiate`, run them under `cancel`
/// (sending each external provider its one request after every item is
/// negotiated, FR-950) and settle each as one record: the one settlement
/// path `analyze` and `run` share (FR-961, FR-963).
///
/// # Errors
///
/// As [`analyze_document`].
pub fn settle_run(
    run: AnalyzeRun<'_>,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> Result<SettledRun, AnalyzeDocumentError> {
    let AnalyzeRun {
        package,
        binding,
        provision,
        items,
        engines,
        limits,
    } = run;
    let source_input = fsl_eval::simulation::model::ModelCheckInput::source(package);
    let Negotiation {
        negotiated,
        unrouted_items,
        settled,
        echoes,
        simulations,
    } = negotiate_items(
        items,
        negotiate,
        &ItemReads {
            admit: &|item| admit_reductions(&source_input, &binding, &limits, item),
            owner: &|claim| owning_model(package, claim).map(str::to_owned),
            family: &|kind, claim| crate::claim_family::claim_family(package, kind, claim),
        },
    )?;
    let outcome = super::request::analyze_bound(
        &super::RunInput::source(package),
        &binding,
        &provision,
        &negotiated,
        engines,
        &limits,
        cancel,
    )
    .map_err(|failure| match failure {
        fsl_foundation::diagnostic::StageFailure::Refused(refusal) => {
            AnalyzeDocumentError::Refused(refusal)
        }
        _ => AnalyzeDocumentError::Fault,
    })?
    .into_value();
    let (targets, routes) = negotiated_routes(&negotiated, &unrouted_items, engines);
    // FR-634: each simulation item's document at its request index; it
    // settles no claim, so it is no aggregate gap, and its category ranks
    // in the exit code.
    let mut simulated = Vec::new();
    for item in &simulations {
        match super::simulation_item::simulate(package, &binding, &limits, item, cancel) {
            Ok(written) => simulated.push(written),
            Err(super::simulation_item::SimulationFault::Binding(failure)) => {
                return Err(AnalyzeDocumentError::Refused(
                    super::AnalyzeRefusal::Binding(failure),
                ));
            }
            Err(super::simulation_item::SimulationFault::Fault) => {
                return Err(AnalyzeDocumentError::Fault);
            }
        }
    }
    write_run(
        SettledParts {
            records: outcome.records,
            counterexamples: outcome.counterexamples,
            details: outcome.details,
            protocol_conformance: outcome.protocol_conformance,
        },
        &settled,
        &echoes,
        &targets,
        &routes,
        &simulated,
        engines,
    )
}

/// What a run reads of its package while it negotiates its items: the
/// source route reads its checked package, the package route (FR-850) the
/// package forms of its items' claims.
struct ItemReads<'r> {
    /// FR-623's admission of an item's pending `reductions`.
    admit: &'r dyn Fn(&mut ClaimItem) -> Result<(), super::reduction_item::ReductionIssue>,
    /// The model package owning an item's claim, which its certificate
    /// identity names.
    owner: &'r dyn Fn(&OccurrenceKey) -> Option<String>,
    /// The declaration family of an item's claim (FR-507).
    family: &'r dyn Fn(ItemKind, &OccurrenceKey) -> Option<fsl_semantics::check::DeclarationFamily>,
}

/// The resolved items of a run, negotiated: the items to analyze, the
/// items negotiation ran for that no route reads, the items settled
/// without running, each claim item's echo and the simulation items.
struct Negotiation {
    negotiated: Vec<NegotiatedItem>,
    unrouted_items: Vec<RequestIndex>,
    settled: BTreeMap<RequestIndex, (Option<String>, TerminalRecord)>,
    echoes: BTreeMap<RequestIndex, Echo>,
    simulations: Vec<super::SimulationItem>,
}

/// Negotiate `items` with `negotiate`, reading the package through `reads`:
/// the one negotiation both routes share.
fn negotiate_items(
    items: Vec<ResolvedItem>,
    negotiate: &dyn Negotiate,
    reads: &ItemReads<'_>,
) -> Result<Negotiation, AnalyzeDocumentError> {
    let mut negotiated = Vec::new();
    // Items negotiation ran for but no route reads: settled by its
    // `unsupported` disposition.
    let mut unrouted_items: Vec<RequestIndex> = Vec::new();
    let mut settled: BTreeMap<RequestIndex, (Option<String>, TerminalRecord)> = BTreeMap::new();
    let mut echoes: BTreeMap<RequestIndex, Echo> = BTreeMap::new();
    let mut simulations: Vec<super::SimulationItem> = Vec::new();
    for resolved in items {
        match resolved {
            ResolvedItem::Simulation(item) => simulations.push(*item),
            ResolvedItem::Settled { claim_name, record } => {
                settled.insert(record.request_index(), (claim_name, *record));
            }
            ResolvedItem::Claim(claim) => {
                let ResolvedClaim {
                    mut item,
                    claim_name,
                    candidates,
                    subject,
                    unrouted,
                } = *claim;
                let admitted = (reads.admit)(&mut item);
                let echo = item
                    .request_item_wire(&subject)
                    .map_err(|_| AnalyzeDocumentError::Fault)?;
                let identity = (reads.owner)(&item.claim)
                    .map(|model| fsl_replay::certificate_identity_wire(&item.claim, &model))
                    .transpose()
                    .map_err(|_| AnalyzeDocumentError::Fault)?;
                echoes.insert(
                    item.request_index,
                    Echo {
                        claim_name,
                        request: echo,
                        locus: fsl_replay::claim_locus_wire(&item.claim).ok(),
                        identity,
                        required: item.options.required,
                        scoped: has_activation_scope(item.kind),
                    },
                );
                if let Err(refused) = admitted {
                    let record = match refused {
                        super::reduction_item::ReductionIssue::UnknownConstraint(name) => {
                            let mut detail = Map::new();
                            detail.insert("name".to_owned(), json!(name));
                            TerminalRecord::disposition_invalid_request(
                                item.request_index,
                                CatalogCode::new("invalid_runtime_input", "invalid-value"),
                                detail,
                            )
                        }
                        super::reduction_item::ReductionIssue::Malformed(_) => {
                            TerminalRecord::disposition_declined(
                                item.request_index,
                                fsl_replay::ProofRefusalCause::InvalidInput,
                                fsl_replay::DeclineCode::Fsl(
                                    fsl_foundation::Code::InvalidRuntimeInput,
                                ),
                                CatalogCode::new("invalid_runtime_input", "invalid-value"),
                            )
                        }
                    };
                    settled.insert(item.request_index, (None, record));
                    continue;
                }
                let (disposition, diagnostic) = negotiate.negotiate(&item, &candidates);
                // A member no route reads settles `unsupported`, except where
                // negotiation refused the item as invalid first: a kind its
                // claim's family does not admit (FR-507, FR-627-AC-4).
                let temporal_kind = match &item.form {
                    Some(ItemForm::Temporal(form)) => form.item_kind,
                    _ => ItemKind::Formula,
                };
                let inapplicable = (reads.family)(temporal_kind, &item.claim)
                    .is_none_or(|family| !item.kind.applies_to_declaration(family));
                if unrouted
                    && !inapplicable
                    && !matches!(disposition, fsl_route::routing::Disposition::InvalidRequest)
                {
                    let mut detail = Map::new();
                    detail.insert("kind".to_owned(), json!(item.kind.to_wire()));
                    unrouted_items.push(item.request_index);
                    settled.insert(
                        item.request_index,
                        (
                            None,
                            TerminalRecord::disposition_unsupported(
                                item.request_index,
                                fsl_route::NegotiationOutcome::UnsupportedRequestedCapability
                                    .code(),
                                detail,
                            ),
                        ),
                    );
                    continue;
                }
                negotiated.push(NegotiatedItem {
                    item,
                    disposition,
                    diagnostic,
                });
            }
        }
    }
    Ok(Negotiation {
        negotiated,
        unrouted_items,
        settled,
        echoes,
        simulations,
    })
}

/// FR-621: the backend each negotiated item was negotiated to, and each
/// item's route, which the document's `last_stage` reads.
fn negotiated_routes(
    negotiated: &[NegotiatedItem],
    unrouted_items: &[RequestIndex],
    engines: &Engines,
) -> (
    BTreeMap<RequestIndex, BackendId>,
    BTreeMap<RequestIndex, super::ItemRoute>,
) {
    // FR-621 "The record's `backend` member": the backend each item was
    // negotiated to, which produced its record when no disposition did.
    let targets: BTreeMap<RequestIndex, BackendId> = negotiated
        .iter()
        .filter_map(|item| {
            item.disposition
                .target()
                .map(|target| (item.item.request_index, target.id().clone()))
        })
        .collect();
    // FR-621 "Last stage": each negotiated item's route. An item negotiated
    // to an FSL engine settles at S6c, one negotiated to an outside provider
    // or settled by its negotiation's disposition at S6b.
    let mut routes: BTreeMap<RequestIndex, super::ItemRoute> = negotiated
        .iter()
        .map(|item| {
            let in_process = item.disposition.target().is_some_and(|target| {
                engines
                    .get(target.id())
                    .is_some_and(|engine| engine.provider.is_none())
            });
            let route = if in_process {
                super::ItemRoute::InProcess
            } else {
                super::ItemRoute::Negotiated
            };
            (item.item.request_index, route)
        })
        .collect();
    routes.extend(
        unrouted_items
            .iter()
            .map(|index| (*index, super::ItemRoute::Negotiated)),
    );
    (targets, routes)
}

/// What the engines settled for a run's negotiated items.
struct SettledParts {
    records: Vec<TerminalRecord>,
    counterexamples: Vec<super::ItemCounterexample>,
    details: Vec<super::ItemDetail>,
    protocol_conformance: BTreeMap<RequestIndex, fsl_replay::ProtocolConformanceOutput>,
}

/// The FR-621 document of a run: the engines' records beside the items
/// settled without running, each item's entry, the aggregate, the registry
/// and the exit code. The source route and the package route (FR-850)
/// write their documents through this one function.
fn write_run(
    outcome: SettledParts,
    settled: &BTreeMap<RequestIndex, (Option<String>, TerminalRecord)>,
    echoes: &BTreeMap<RequestIndex, Echo>,
    targets: &BTreeMap<RequestIndex, BackendId>,
    routes: &BTreeMap<RequestIndex, super::ItemRoute>,
    simulated: &[(Value, Category)],
    engines: &Engines,
) -> Result<SettledRun, AnalyzeDocumentError> {
    let mut records: Vec<TerminalRecord> = outcome.records;
    records.extend(settled.values().map(|(_, record)| record.clone()));
    records.sort_by_key(TerminalRecord::request_index);
    let base = super::analyze_document_routed(&records, &outcome.counterexamples, routes)
        .map_err(|_| AnalyzeDocumentError::Fault)?;
    let mut document = serde_json::to_value(&base).map_err(|_| AnalyzeDocumentError::Fault)?;
    let written: Vec<Value> = document["items"].as_array().cloned().unwrap_or_default();
    let mut items = Vec::with_capacity(records.len());
    let mut gaps = Vec::new();
    for (record, mut written) in records.iter().zip(written) {
        let index = record.request_index();
        if let Some(bytes) = record.exact_certificate_document() {
            if let Ok(document) = serde_json::from_slice::<Value>(bytes) {
                written["certificate"] = exact_certificate_member(
                    echoes.get(&index).and_then(|echo| echo.identity.clone()),
                    document,
                );
            }
        }
        if let Some(output) = outcome.protocol_conformance.get(&index) {
            if let (Some(record), Ok(Value::Object(members))) =
                (written.as_object_mut(), serde_json::to_value(output))
            {
                record.extend(members);
            }
        }
        let detail = outcome
            .details
            .iter()
            .find(|detail| detail.request_index == index);
        let echo = echoes.get(&index);
        if let Some(locus) = echo.and_then(|echo| echo.locus.as_ref()) {
            if let Some(verdict) = written.get_mut("verdict") {
                name_rejection_locus(verdict, locus);
            }
        }
        let claim_name = match echo {
            Some(echo) => Some(echo.claim_name.clone()),
            None => settled.get(&index).and_then(|(name, _)| name.clone()),
        };
        let required = echo.is_none_or(|echo| echo.required);
        let disposed = record.settled_by_disposition();
        if required
            && !matches!(
                record.value(),
                TerminalValue::Proved { .. } | TerminalValue::Measured(_)
            )
        {
            gaps.push(json!({"request_index": index.get(), "reason": "not-proved",
                "result": written["result"].as_str().unwrap_or("invalid-request")}));
        }
        let replay_was_published = outcome
            .protocol_conformance
            .get(&index)
            .is_none_or(|conformance| conformance.replay.is_some());
        let target = replay_was_published
            .then(|| {
                outcome
                    .counterexamples
                    .iter()
                    .find(|found| found.request_index == index)
                    .and_then(|found| {
                        claim_name
                            .as_deref()
                            .and_then(|name| replay_target(found, name))
                    })
            })
            .flatten();
        items.push(item_document(
            written,
            ItemMembers {
                claim_name,
                request: echo.map(|echo| echo.request.clone()),
                identity: echo.and_then(|echo| echo.identity.clone()),
                target,
                detail: detail.cloned(),
                scoped: echo.is_some_and(|echo| echo.scoped),
                axes: OutcomeAxes::of_record(record.value(), disposed),
                producer: record
                    .producer(targets.get(&index))
                    .map(|backend| backend.as_str().to_owned()),
                disposed,
            },
        ));
    }
    items.extend(simulated.iter().map(|(written, _)| written.clone()));
    items.sort_by_key(|item| item["request_index"].as_u64().unwrap_or(u64::MAX));
    document["items"] = Value::Array(items.clone());
    document["aggregate"] = json!({
        "status": if gaps.is_empty() { "complete" } else { "incomplete" },
        "gaps": gaps,
    });
    // FR-635 "Outputs": `{backend, tool}` of every registered backend,
    // sorted by backend.
    let mut registry: Vec<RegistryEntry> = engines
        .tools()
        .map(|(descriptor, tool)| RegistryEntry {
            backend: descriptor.id().clone(),
            tool: tool.clone(),
        })
        .collect();
    registry.sort_unstable_by(|left, right| {
        (left.backend.as_str(), &left.tool).cmp(&(right.backend.as_str(), &right.tool))
    });
    document["registry"] = Value::Array(registry.iter().map(RegistryEntry::wire).collect());
    let exit_code = Category::most_severe(
        records
            .iter()
            .map(|record| record.value().category().exit_code())
            .chain(simulated.iter().map(|(_, category)| category.exit_code())),
    )
    .unwrap_or(0);
    Ok(SettledRun {
        records,
        counterexamples: outcome.counterexamples,
        items,
        registry,
        document,
        exit_code,
    })
}

/// Whether an item of `kind` has an activation scope, so it carries
/// FR-622's `axes`.
const fn has_activation_scope(kind: Capability) -> bool {
    !matches!(
        kind,
        Capability::ValueValidity
            | Capability::OperationContract
            | Capability::Schedulability
            | Capability::ProbabilisticSatisfaction
    )
}

/// FR-621: beside each state key of a counterexample, the state it names,
/// whose values are FSpec FR-181's typed canonical forms. A state key is the
/// unpadded base64url spelling of the state's canonical bytes, so the state
/// is read back from the key and nothing is recomputed.
#[ix_trace_rs::string_edge]
fn spell_states(value: &mut Value) {
    use base64::Engine;
    match value {
        Value::Object(members) => {
            let state = members
                .get("key")
                .and_then(Value::as_str)
                .and_then(|key| {
                    base64::engine::general_purpose::URL_SAFE_NO_PAD
                        .decode(key)
                        .ok()
                })
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .filter(|state| state["type"] == "simulation-state");
            if let Some(state) = state {
                members.insert("state".to_owned(), state);
            }
            for member in members.values_mut() {
                spell_states(member);
            }
        }
        Value::Array(entries) => entries.iter_mut().for_each(spell_states),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

/// What the run knows of an item it resolved, beside its terminal record.
struct Echo {
    /// The declared name the item resolved to.
    claim_name: String,
    /// The admitted FSpec FR-331 request item.
    request: Value,
    /// FSpec FR-412's certificate identity of the item's obligation;
    /// `None` when no one domain package owns the claim.
    identity: Option<Value>,
    /// The `{node, occurrence}` of the item's claim, which FSpec FR-410's
    /// rejection `where` names as its `locus`.
    locus: Option<Value>,
    /// Whether the aggregate requires the item.
    required: bool,
    /// Whether the item has an activation scope (FR-622).
    scoped: bool,
}

/// The FR-621 members [`item_document`] adds to an FR-286 item.
struct ItemMembers {
    claim_name: Option<String>,
    request: Option<Value>,
    identity: Option<Value>,
    target: Option<Value>,
    detail: Option<super::ItemDetail>,
    scoped: bool,
    axes: OutcomeAxes,
    /// The backend that produced the record, when a result settled it.
    producer: Option<String>,
    /// Whether a disposition settled the record.
    disposed: bool,
}

/// The identity of the domain package that owns the obligation `claim`
/// names, FR-621's certificate `model` (FSpec FR-201, FR-412): the package
/// of a temporal clause's `over` object type, else the package's one model
/// selection. `None` when the claim names no single owning package: a
/// package with no model selection, or several and a claim with no `over`
/// object type.
fn owning_model<'p>(package: &'p CheckedPackage, claim: &'p OccurrenceKey) -> Option<&'p str> {
    let graph = package.graph();
    let over = graph
        .temporal_clauses()
        .iter()
        .find(|clause| clause.identity() == *claim.node())
        .and_then(|clause| match &clause.clause().over().1 {
            filament_exact::ValueType::Reference(object_type) => {
                Some(object_type.original().package())
            }
            _ => None,
        });
    let hybrid = graph
        .hybrid_claim_sites()
        .iter()
        .any(|site| &site.identity == claim.node())
        .then(|| claim.node().package());
    over.or(hybrid).or_else(|| match graph.model_selections() {
        [selection] => Some(selection.identity.as_str()),
        _ => None,
    })
}

/// FSpec FR-323's replay target member object for the reproduced
/// counterexample `found` of the claim declared `name`: a temporal clause
/// (over a model or a protocol subject, and a probabilistic one) and an
/// operation contract's violated state clause name their clause by
/// `state_clause`, and a value-validity assignment names the function its
/// parameters belong to by `function`. The in-process engines produce no
/// protocol-refinement counterexample, so FR-071's `protocol_refinement`
/// target has no variant to come from here.
fn replay_target(found: &super::ItemCounterexample, name: &str) -> Option<Value> {
    let qualified = name.split("::").collect::<Vec<_>>().join(".");
    Some(match &found.counterexample {
        super::Counterexample::Assignments(_) => json!({"function": qualified}),
        super::Counterexample::Temporal { .. }
        | super::Counterexample::Hyper(_)
        | super::Counterexample::Protocol { .. }
        | super::Counterexample::Probabilistic(_)
        | super::Counterexample::Contract(_)
        | super::Counterexample::StateGraph(_)
        | super::Counterexample::GlobalConformance(_)
        | super::Counterexample::Hybrid(_)
        | super::Counterexample::TaskProduct(_) => json!({"state_clause": qualified}),
        super::Counterexample::FiniteHyper(_) => json!({"hyperproperty": qualified}),
        super::Counterexample::ScheduleMiss(_) => return None,
    })
}

/// FR-621's `certificate` of a hybrid item (FSpec FR-652): `identity` and
/// `form`, and for a certificate FR-751 checked `checker`, `accepted` and,
/// when rejected, `rule` and `at`.
fn hybrid_certificate_member(identity: Value, recorded: &fsl_replay::CertificateRecord) -> Value {
    let mut member = Map::new();
    member.insert("identity".to_owned(), identity);
    member.insert("form".to_owned(), recorded.form.clone());
    match &recorded.outcome {
        fsl_replay::CertificateStatus::Unchecked => {}
        fsl_replay::CertificateStatus::Accepted => {
            member.insert(
                "checker".to_owned(),
                fsl_replay::HYBRID_CERTIFICATE_CHECKER.into(),
            );
            member.insert("accepted".to_owned(), true.into());
        }
        fsl_replay::CertificateStatus::Rejected { rule, at } => {
            member.insert(
                "checker".to_owned(),
                fsl_replay::HYBRID_CERTIFICATE_CHECKER.into(),
            );
            member.insert("accepted".to_owned(), false.into());
            member.insert("rule".to_owned(), rule.spelling().into());
            member.insert("at".to_owned(), at.clone().into());
        }
    }
    Value::Object(member)
}

/// FR-621's `certificate` of an exact `proved` item (FR-741): `identity`,
/// `checker` and `accepted`, `kind` (`long-run` for a long-run objective,
/// else `exact`) and the `document`.
fn exact_certificate_member(identity: Option<Value>, document: Value) -> Value {
    let long_run = document
        .pointer("/objective/profile")
        .and_then(Value::as_str)
        .is_some_and(|profile| profile.contains("long-run"));
    let mut member = Map::new();
    if let Some(identity) = identity {
        member.insert("identity".to_owned(), identity);
    }
    member.insert(
        "checker".to_owned(),
        fsl_replay::CertificateChecker::ProbabilityCertificate
            .as_str()
            .into(),
    );
    member.insert("accepted".to_owned(), true.into());
    member.insert(
        "kind".to_owned(),
        (if long_run { "long-run" } else { "exact" }).into(),
    );
    member.insert("document".to_owned(), document);
    Value::Object(member)
}

/// Write `locus` into every `undefined-evaluation` rejection cause of a
/// statistical `verdict`: the replay layer holds no claim, and FSpec FR-410's
/// `where` requires its `locus`.
fn name_rejection_locus(verdict: &mut Value, locus: &Value) {
    let fill = |decision: &mut Value| {
        if let Some(at) = decision
            .pointer_mut("/cause/where")
            .and_then(Value::as_object_mut)
        {
            at.insert("locus".to_owned(), locus.clone());
        }
    };
    fill(&mut verdict["decision"]);
    if let Some(tests) = verdict["tests"].as_array_mut() {
        for test in tests {
            fill(&mut test["decision"]);
        }
    }
}

/// One FR-621 item from the FR-286 item `written`.
fn item_document(written: Value, members: ItemMembers) -> Value {
    let ItemMembers {
        claim_name,
        request,
        identity,
        target,
        detail,
        scoped,
        axes,
        producer,
        disposed,
    } = members;
    let Value::Object(mut record) = written else {
        return written;
    };
    let index = record.remove("request_index").unwrap_or(Value::Null);
    let category = record.remove("category").unwrap_or(Value::Null);
    let counterexample = record.remove("counterexample");
    let certificate_document = record.remove("certificate_document");
    let certificate = record.remove("certificate");
    let mut item = Map::new();
    item.insert("request_index".to_owned(), index);
    item.insert(
        "claim_name".to_owned(),
        claim_name.map_or(Value::Null, Value::String),
    );
    item.insert("request".to_owned(), request.unwrap_or(Value::Null));
    item.insert("category".to_owned(), category);
    let task_search_identity = identity.clone();
    if let Some(certificate) = certificate {
        item.insert("certificate".to_owned(), certificate);
    } else if let (Some((wire, accepted)), Some(_)) = (
        detail.as_ref().and_then(|detail| detail.task_certificate.clone()),
        identity.clone(),
    ) {
        // FSpec FR-418: the task-product certificate the symbolic pass wrote
        // and the checker read, spelled whole, beside the checker's verdict.
        let mut certificate = wire;
        if let Some(members) = certificate.as_object_mut() {
            members.insert(
                "checker".to_owned(),
                fsl_replay::CertificateChecker::Zone.as_str().into(),
            );
            members.insert("accepted".to_owned(), accepted.into());
        }
        item.insert("certificate".to_owned(), certificate);
    } else if let (Some(recorded), Some(identity)) = (
        detail
            .as_ref()
            .and_then(|detail| detail.hybrid_certificate.as_ref()),
        identity.clone(),
    ) {
        item.insert(
            "certificate".to_owned(),
            hybrid_certificate_member(identity, recorded),
        );
    } else if let (Some(checker), Some(identity), Some(result)) = (
        detail.as_ref().and_then(|detail| detail.checker),
        identity,
        record.get("result").and_then(Value::as_str),
    ) {
        if matches!(result, "proved" | "refuted") {
            let mut certificate = json!({
                "identity": identity,
                "checker": checker.as_str(),
                "accepted": result == "proved"
            });
            if let Some(document) = certificate_document {
                if let Some(certificate) = certificate.as_object_mut() {
                    certificate.insert("document".to_owned(), document);
                }
            }
            item.insert("certificate".to_owned(), certificate);
        }
    }
    let mut spelled = None;
    if let Some(mut counterexample) = counterexample {
        spell_states(&mut counterexample);
        spelled = Some(counterexample.clone());
        if let Some(target) = target {
            item.insert(
                "replay".to_owned(),
                json!({"target": target, "reproduced": true}),
            );
        }
        item.insert("counterexample".to_owned(), counterexample);
    }
    for member in [
        "obligations",
        "closure",
        "global",
        "projections",
        "recording",
        "verdict",
    ] {
        if let Some(value) = record.get(member) {
            item.insert(member.to_owned(), value.clone());
        }
    }
    if let Some(stats) = detail.as_ref().and_then(|detail| detail.exploration) {
        item.insert(
            "exploration".to_owned(),
            json!({
                "states": stats.product_states.to_string(),
                "transitions": stats
                    .product_edges
                    .saturating_sub(stats.stutter_edges)
                    .to_string(),
                "automaton_states": stats.automaton_states.to_string(),
            }),
        );
    }
    if let Some(non_zeno) = detail.as_ref().and_then(|detail| detail.non_zeno) {
        record.insert("non_zeno".to_owned(), json!(non_zeno.as_str()));
    }
    // FSpec FR-439: the record states the memory budget the run used and
    // whether it reached it.
    let memory_bound = detail
        .as_ref()
        .and_then(|detail| detail.memory_bound)
        .map(|stated| {
            json!({
                "bound": stated.bound.member.as_str(),
                "value": stated.bound.value,
                "reached": stated.reached,
            })
        });
    if scoped {
        item.insert(
            "axes".to_owned(),
            serde_json::to_value(axes).unwrap_or(Value::Null),
        );
    }
    // FR-941 "Bare claim": a proof over a supplied trace names the trace and
    // the steps it replayed; it is about that trace only.
    if let Some(steps) = detail.as_ref().and_then(|detail| detail.replayed_steps) {
        if let Some(trace) = item.get("request").and_then(|request| request.get("trace")) {
            record.insert("trace".to_owned(), trace.clone());
        }
        record.insert("steps".to_owned(), json!(steps));
    }
    // FR-941: a claim's required clauses are read by the temporal evaluator,
    // whose reading is the item's FSpec FR-352 `disposition.temporal`; a
    // protocol expression undefined at a replayed state names where.
    if let Some(finite) = detail
        .as_ref()
        .and_then(|detail| detail.finite_replay.as_ref())
    {
        if let Some(evaluation) = &finite.evaluation {
            if let Ok(disposition) =
                fsl_replay::temporal_disposition_json(evaluation.as_ref(), spelled.clone())
            {
                if let (Some(axes), Some(basis)) = (item.get_mut("axes"), disposition.get("basis"))
                {
                    axes["settlement"] = basis.clone();
                }
                item.insert("disposition".to_owned(), disposition);
            }
        }
        if let (Some(at), Some(Value::Object(cause))) =
            (&finite.undefined_where, record.get_mut("cause"))
        {
            cause.insert("where".to_owned(), at.clone());
        }
    }
    // FR-450 "Settlement": a `hybrid-trace` disagreement retains the complete
    // counterexample replay did not agree with.
    if let Some(found) = detail
        .as_ref()
        .and_then(|detail| detail.hybrid_disagreement.as_ref())
    {
        if let Some(Value::Object(disagreement)) = record
            .get_mut("cause")
            .and_then(|cause| cause.get_mut("parity"))
            .and_then(|parity| parity.get_mut("disagreement"))
        {
            disagreement.insert("counterexample".to_owned(), found.clone());
        }
    }
    // FSpec FR-331: a bounded task search's frontier evidence rides on its
    // incomplete result, bound to the item's certificate identity.
    if let (Some(Value::Object(mut evidence)), Some(identity)) = (
        detail
            .as_ref()
            .and_then(|detail| detail.task_search.clone()),
        task_search_identity,
    ) {
        evidence.insert("identity".to_owned(), identity);
        record.insert("task_search".to_owned(), Value::Object(evidence));
    }
    // FR-621: `settled_by`, and `backend` exactly when a result settled
    // the record.
    if let Some(stated) = memory_bound {
        record.insert("memory_bound".to_owned(), stated);
    }
    record.insert(
        "settled_by".to_owned(),
        json!(if disposed { "disposition" } else { "result" }),
    );
    if let Some(backend) = producer {
        record.insert("backend".to_owned(), json!(backend));
    }
    item.insert("record".to_owned(), Value::Object(record));
    Value::Object(item)
}
