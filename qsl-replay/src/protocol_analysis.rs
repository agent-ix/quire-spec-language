// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 slice 2: the structural key, bounds, premises and outcome types of
//! a protocol analysis (FR-572, FR-570, FR-571).
//!
//! A [`ProtocolAnalysisKey`] holds the members of QSpec FR-174-AC-6 (a
//! projection) or FR-175-AC-6 (a realizability result) as typed values and
//! never a digest. It is built from a draft whose members are all optional:
//! the builder and the reader of a stored key both refuse the first absent
//! member as `invalid_runtime_input`/`missing-member` naming it, and nothing
//! defaults a member. Display name, path, span, producing run and timestamp
//! are not fields of the types, so they cannot change a key. Run limits
//! (`max_states`, `max_transitions`, the evaluation meter) are outside the
//! key (PA-14).

use std::collections::BTreeSet;

use qsl_package::protocol_view::{
    channel_endpoints, channel_ordering, choice_visibility, control_operation, delivery, fifo_key,
    queue_capacity, queue_channel, queue_overflow, ChannelOrdering, ControlOperation, Delivery,
    LossSelector, Overflow, QueueCapacity, ViewRefusal,
};
use quire_contract_model::{CheckedNodeId, CheckedSemanticNodeV2};
use serde::{Deserialize, Deserializer, Serialize};

/// The refusal of a key with an absent member: `invalid_runtime_input` with
/// cause `missing-member`, naming the member (FR-572, PA-13).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("invalid_runtime_input/missing-member: the analysis key carries no `{member}`")]
pub struct MissingMember {
    /// The absent key member; a nested member is named by its path, such as
    /// `channels[0].delivery`.
    pub member: String,
}

impl MissingMember {
    /// A refusal naming `member`.
    #[must_use]
    pub fn new(member: impl Into<String>) -> Self {
        Self {
            member: member.into(),
        }
    }

    /// The catalog code of the refusal.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        "invalid_runtime_input"
    }

    /// The catalog cause of the refusal.
    #[must_use]
    pub const fn cause(&self) -> &'static str {
        "missing-member"
    }
}

/// Why a stored key did not read.
#[derive(Debug, thiserror::Error)]
pub enum KeyReadError {
    /// A member is absent.
    #[error(transparent)]
    Missing(#[from] MissingMember),
    /// A member is present in a shape the key does not admit, or is a member
    /// the key kind does not have.
    #[error("the analysis key does not read: {0}")]
    Malformed(#[from] serde_json::Error),
    /// The `kind` is present but names neither a projection nor a
    /// realizability key.
    #[error("the analysis key kind `{0}` is neither projection nor realizability")]
    UnknownKind(String),
}

impl KeyReadError {
    /// The catalog code of the refusal.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        "invalid_runtime_input"
    }

    /// The catalog cause of the refusal.
    #[must_use]
    pub const fn cause(&self) -> &'static str {
        match self {
            Self::Missing(_) => "missing-member",
            Self::Malformed(_) | Self::UnknownKind(_) => "invalid-value",
        }
    }
}

/// The two bounds that are key members (PA-14). There is no default.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Bounds {
    /// The horizon, in events along a path or play.
    pub max_depth: u64,
    /// The local history length; a member of a realizability key only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_history: Option<u64>,
}

/// The bounds of a realizability key, whose members may be absent.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundsDraft {
    /// The horizon.
    pub max_depth: Option<u64>,
    /// The local history length.
    pub max_history: Option<u64>,
}

/// The bounds of a projection key, whose `max_depth` may be absent. A
/// projection has no `max_history` member, so a stored one is refused.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionBoundsDraft {
    /// The horizon.
    pub max_depth: Option<u64>,
}

/// A channel's ordering premise (FR-054).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "kebab-case")]
pub enum OrderingPremise {
    /// No order between receives.
    Unordered,
    /// Receives follow the FIFO key.
    Fifo {
        /// The message binder node.
        binder: CheckedNodeId,
        /// The key expression node.
        key: CheckedNodeId,
    },
}

/// A channel's delivery premise (FR-054, FR-509).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "kebab-case")]
pub enum DeliveryPremise {
    /// `unknown`.
    Unknown,
    /// `at-most-once`.
    AtMostOnce,
    /// `at-least-once`.
    AtLeastOnce,
    /// `exactly-once-premise`.
    ExactlyOncePremise,
    /// A declared count interval, both bounds inclusive.
    Between {
        /// The least count.
        lower: u64,
        /// The greatest count.
        upper: u64,
    },
}

/// A queue's capacity premise.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "kebab-case")]
pub enum CapacityPremise {
    /// A written integer capacity.
    Literal {
        /// The capacity.
        value: u64,
    },
    /// A symbolic capacity, by name.
    Symbolic {
        /// The capacity's name.
        name: String,
    },
}

/// A queue's overflow premise.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "kebab-case")]
pub enum OverflowPremise {
    /// The send is refused.
    Reject,
    /// The send waits.
    Block,
    /// A message is dropped.
    Loss {
        /// Which message: `head`, `tail` or `new`.
        selector: LossPremise,
    },
}

/// Which message a lossy queue drops.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LossPremise {
    /// The oldest queued message.
    Head,
    /// The newest queued message.
    Tail,
    /// The arriving message.
    New,
}

/// Premises group (a): one channel's FR-054 members with its queue's capacity
/// and overflow policy (PA-6).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelPremise {
    /// The channel node.
    pub channel: CheckedNodeId,
    /// The sending role node.
    pub sender: CheckedNodeId,
    /// The receiving role node.
    pub receiver: CheckedNodeId,
    /// The message type node.
    pub message_type: CheckedNodeId,
    /// The ordering policy.
    pub ordering: OrderingPremise,
    /// The delivery policy.
    pub delivery: DeliveryPremise,
    /// The queue capacity.
    pub capacity: CapacityPremise,
    /// The queue overflow policy.
    pub overflow: OverflowPremise,
}

/// Premises group (b): one `choice` control's owner and visible set (PA-6).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoicePremise {
    /// The `choice` control node.
    pub choice: CheckedNodeId,
    /// The owning node.
    pub owner: CheckedNodeId,
    /// The visible `expression` nodes, in the order the node carries them.
    pub visible: Vec<CheckedNodeId>,
}

/// The premises of a protocol analysis (PA-6, PA-8). Groups (a) and (b) are
/// built from the linked package; groups (c) and (d) are authored by a
/// realizability claim. `None` is "not stated" and `Some` of an empty set is
/// "declared empty": different premises, both present.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolPremises {
    /// Group (a): the channels, ascending by channel node.
    pub channels: Vec<ChannelPremise>,
    /// Group (b): the choices, ascending by choice node.
    pub choices: Vec<ChoicePremise>,
    /// Group (c): the shared facts a role may condition on.
    pub role_knowledge: Option<BTreeSet<String>>,
    /// Group (d): the admitted external events and channel outcomes.
    pub environment: Option<BTreeSet<String>>,
}

/// Why premises (a) and (b) were not built from a package.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum PremiseRefusal {
    /// A protocol read refused.
    #[error(transparent)]
    View(#[from] ViewRefusal),
    /// A node the view named is not in the package.
    #[error("the package holds no node {0:?}")]
    UnknownNode(CheckedNodeId),
    /// A queue belongs to no channel of the clause.
    #[error("the queue {queue:?} names no channel of the clause")]
    OrphanQueue {
        /// The queue node.
        queue: CheckedNodeId,
    },
    /// A channel of the clause has no queue.
    #[error("the channel {channel:?} has no queue")]
    ChannelWithoutQueue {
        /// The channel node.
        channel: CheckedNodeId,
    },
}

/// Why a string is no canonical node-id text.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum NodeTextError {
    /// The id does not encode.
    #[error("a node id does not encode: {0}")]
    Encoding(String),
    /// The text does not read as a checked node id.
    #[error("the text does not read as a checked node id")]
    NotANodeId,
    /// The text reads as a node id but is not its canonical encoding.
    #[error("the text is not the canonical encoding of the node id it reads as")]
    NotCanonical,
}

/// The canonical string of a checked node id (ADR-036 PA-24): the RFC 8785
/// text of its `CheckedNodeId` V2 wire object, `quire_canonical::to_vec`.
///
/// # Errors
///
/// [`NodeTextError::Encoding`] when the id does not encode.
pub fn node_text(id: &CheckedNodeId) -> Result<String, NodeTextError> {
    let bytes = quire_canonical::to_vec(id, quire_canonical::Limits::new(u64::MAX))
        .map_err(|error| NodeTextError::Encoding(error.to_string()))?;
    String::from_utf8(bytes).map_err(|error| NodeTextError::Encoding(error.to_string()))
}

/// The checked node id a canonical string names.
///
/// # Errors
///
/// [`NodeTextError::NotANodeId`] when the text does not read as a node id,
/// [`NodeTextError::NotCanonical`] when re-encoding the id does not give the
/// same bytes.
pub fn read_node_text(text: &str) -> Result<CheckedNodeId, NodeTextError> {
    let id: CheckedNodeId = serde_json::from_str(text).map_err(|_| NodeTextError::NotANodeId)?;
    if node_text(&id)? == text {
        Ok(id)
    } else {
        Err(NodeTextError::NotCanonical)
    }
}

/// The premises of each of `channels` with its queue's capacity and
/// overflow policy. A queue naming no channel of `channels`, or a channel
/// with no queue, is refused.
pub(crate) fn pair_channels(
    channels: &[&CheckedSemanticNodeV2],
    queues: &[&CheckedSemanticNodeV2],
) -> Result<Vec<ChannelPremise>, PremiseRefusal> {
    let queues = queues
        .iter()
        .map(|queue| Ok((queue_channel(queue)?, *queue)))
        .collect::<Result<Vec<_>, PremiseRefusal>>()?;
    for (channel, queue) in &queues {
        if !channels.iter().any(|held| &held.node_id == *channel) {
            return Err(PremiseRefusal::OrphanQueue {
                queue: queue.node_id.clone(),
            });
        }
    }
    channels
        .iter()
        .map(|channel| {
            let (_, queue) = queues
                .iter()
                .find(|(held, _)| *held == &channel.node_id)
                .ok_or_else(|| PremiseRefusal::ChannelWithoutQueue {
                    channel: channel.node_id.clone(),
                })?;
            channel_premise(channel, queue)
        })
        .collect()
}

/// The premises of each `choice` among `controls`. A control whose
/// operation does not read is refused, never skipped.
pub(crate) fn choice_premises(
    controls: &[&CheckedSemanticNodeV2],
) -> Result<Vec<ChoicePremise>, PremiseRefusal> {
    let mut choices = Vec::new();
    for node in controls {
        if control_operation(node)? != ControlOperation::Choice {
            continue;
        }
        let found = choice_visibility(node)?;
        choices.push(ChoicePremise {
            choice: found.choice.clone(),
            owner: found.owner.clone(),
            visible: found.visible.iter().map(|id| (*id).clone()).collect(),
        });
    }
    Ok(choices)
}

fn channel_premise(
    channel: &CheckedSemanticNodeV2,
    queue: &CheckedSemanticNodeV2,
) -> Result<ChannelPremise, PremiseRefusal> {
    let ends = channel_endpoints(channel)?;
    let ordering = match channel_ordering(channel)? {
        ChannelOrdering::Unordered => OrderingPremise::Unordered,
        ChannelOrdering::Fifo => {
            let key = fifo_key(channel)?;
            OrderingPremise::Fifo {
                binder: key.binder.clone(),
                key: key.key.clone(),
            }
        }
    };
    Ok(ChannelPremise {
        channel: channel.node_id.clone(),
        sender: ends.sender.clone(),
        receiver: ends.receiver.clone(),
        message_type: ends.message_type.clone(),
        ordering,
        delivery: match delivery(channel)? {
            Delivery::Unknown => DeliveryPremise::Unknown,
            Delivery::AtMostOnce => DeliveryPremise::AtMostOnce,
            Delivery::AtLeastOnce => DeliveryPremise::AtLeastOnce,
            Delivery::ExactlyOncePremise => DeliveryPremise::ExactlyOncePremise,
            Delivery::Between { lower, upper } => DeliveryPremise::Between { lower, upper },
        },
        capacity: match queue_capacity(queue)? {
            QueueCapacity::Literal(value) => CapacityPremise::Literal { value },
            QueueCapacity::Symbolic(name) => CapacityPremise::Symbolic {
                name: name.to_owned(),
            },
        },
        overflow: match queue_overflow(queue)? {
            Overflow::Reject => OverflowPremise::Reject,
            Overflow::Block => OverflowPremise::Block,
            Overflow::Loss(selector) => OverflowPremise::Loss {
                selector: match selector {
                    LossSelector::Head => LossPremise::Head,
                    LossSelector::Tail => LossPremise::Tail,
                    LossSelector::New => LossPremise::New,
                },
            },
        },
    })
}

/// The producing analyzer's identity and version: a member of a realizability
/// key only (FR-175 "Derived-artifact identity"; PA-15).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzerIdentity {
    /// `qsl-analyze::protocol_analysis`.
    pub identity: String,
    /// The crate version.
    pub version: String,
}

/// The members of a projection key (FR-174-AC-6).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProjectionKey {
    /// The FR-288 qualified definition key.
    pub definition_key: String,
    /// The selected role.
    pub role: String,
    /// The projection profile.
    pub profile: String,
    /// The channel premises as selected.
    pub channels: Vec<ChannelPremise>,
    /// The choice and visibility premises as selected.
    pub choices: Vec<ChoicePremise>,
    /// The bounds; `max_depth` only.
    pub bounds: Bounds,
}

/// The members of a realizability key (FR-175-AC-6).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RealizabilityKey {
    /// The FR-288 qualified definition key.
    pub definition_key: String,
    /// The strategy profile.
    pub strategy_profile: String,
    /// The premises: role knowledge, visibility, channels and environment.
    #[serde(flatten)]
    pub premises: ProtocolPremises,
    /// The bounds; `max_depth` and `max_history`.
    pub bounds: Bounds,
    /// The producing analyzer identity and version.
    pub analyzer: AnalyzerIdentity,
}

/// The structural key of a protocol analysis result (PA-13).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProtocolAnalysisKey {
    /// A projection's key.
    Projection(ProjectionKey),
    /// A realizability result's key.
    Realizability(RealizabilityKey),
}

/// A projection key whose members may be absent.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionKeyDraft {
    /// The FR-288 qualified definition key.
    pub definition_key: Option<String>,
    /// The selected role.
    pub role: Option<String>,
    /// The projection profile.
    pub profile: Option<String>,
    /// The channel premises.
    pub channels: Option<Vec<ChannelPremise>>,
    /// The choice and visibility premises.
    pub choices: Option<Vec<ChoicePremise>>,
    /// The bounds.
    pub bounds: Option<ProjectionBoundsDraft>,
}

/// A realizability key whose members may be absent. The optional-valued
/// premises are double options: the outer layer is the member, the inner the
/// value, so `Some(None)` ("not stated") is present.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealizabilityKeyDraft {
    /// The FR-288 qualified definition key.
    pub definition_key: Option<String>,
    /// The strategy profile.
    pub strategy_profile: Option<String>,
    /// Group (c).
    #[serde(default, deserialize_with = "stated")]
    pub role_knowledge: Option<Option<BTreeSet<String>>>,
    /// The channel premises.
    pub channels: Option<Vec<ChannelPremise>>,
    /// The choice and visibility premises.
    pub choices: Option<Vec<ChoicePremise>>,
    /// Group (d).
    #[serde(default, deserialize_with = "stated")]
    pub environment: Option<Option<BTreeSet<String>>>,
    /// The bounds.
    pub bounds: Option<BoundsDraft>,
    /// The analyzer identity.
    pub analyzer: Option<AnalyzerIdentity>,
}

/// A member present with a value or `null`: `null` is `Some(None)`.
fn stated<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

fn present<T>(member: Option<T>, name: &str) -> Result<T, MissingMember> {
    member.ok_or_else(|| MissingMember::new(name))
}

type Object = serde_json::Map<String, serde_json::Value>;

fn require(object: &Object, path: &str, members: &[&str]) -> Result<(), MissingMember> {
    for member in members {
        if !object.contains_key(*member) {
            return Err(MissingMember::new(format!("{path}.{member}")));
        }
    }
    Ok(())
}

/// Refuse the first absent member of a nested premise or the analyzer,
/// naming its path, before serde reads the document (PA-13).
fn require_nested(document: &Object) -> Result<(), MissingMember> {
    let entries = |name: &str| match document.get(name) {
        Some(serde_json::Value::Array(entries)) => entries.as_slice(),
        _ => &[],
    };
    for (at, channel) in entries("channels").iter().enumerate() {
        let serde_json::Value::Object(channel) = channel else {
            continue;
        };
        let path = format!("channels[{at}]");
        require(
            channel,
            &path,
            &[
                "channel",
                "sender",
                "receiver",
                "message_type",
                "ordering",
                "delivery",
                "capacity",
                "overflow",
            ],
        )?;
        if let Some(serde_json::Value::Object(ordering)) = channel.get("ordering") {
            if ordering.get("kind").and_then(serde_json::Value::as_str) == Some("fifo") {
                require(ordering, &format!("{path}.ordering"), &["binder", "key"])?;
            }
        }
    }
    for (at, choice) in entries("choices").iter().enumerate() {
        if let serde_json::Value::Object(choice) = choice {
            require(
                choice,
                &format!("choices[{at}]"),
                &["choice", "owner", "visible"],
            )?;
        }
    }
    if let Some(serde_json::Value::Object(analyzer)) = document.get("analyzer") {
        require(analyzer, "analyzer", &["identity", "version"])?;
    }
    Ok(())
}

impl ProtocolAnalysisKey {
    /// Build a projection key, refusing the first absent member (PA-13).
    ///
    /// # Errors
    ///
    /// [`MissingMember`] naming the first absent member in key order.
    pub fn projection(draft: ProjectionKeyDraft) -> Result<Self, MissingMember> {
        let definition_key = present(draft.definition_key, "definition_key")?;
        let role = present(draft.role, "role")?;
        let profile = present(draft.profile, "profile")?;
        let mut channels = present(draft.channels, "channels")?;
        let mut choices = present(draft.choices, "choices")?;
        let max_depth = present(
            present(draft.bounds, "bounds")?.max_depth,
            "bounds.max_depth",
        )?;
        channels.sort();
        choices.sort();
        Ok(Self::Projection(ProjectionKey {
            definition_key,
            role,
            profile,
            channels,
            choices,
            bounds: Bounds {
                max_depth,
                max_history: None,
            },
        }))
    }

    /// Build a realizability key, refusing the first absent member (PA-13).
    ///
    /// # Errors
    ///
    /// [`MissingMember`] naming the first absent member in key order.
    pub fn realizability(draft: RealizabilityKeyDraft) -> Result<Self, MissingMember> {
        let definition_key = present(draft.definition_key, "definition_key")?;
        let strategy_profile = present(draft.strategy_profile, "strategy_profile")?;
        let role_knowledge = present(draft.role_knowledge, "role_knowledge")?;
        let mut channels = present(draft.channels, "channels")?;
        let mut choices = present(draft.choices, "choices")?;
        let environment = present(draft.environment, "environment")?;
        let bounds = present(draft.bounds, "bounds")?;
        let max_depth = present(bounds.max_depth, "bounds.max_depth")?;
        let max_history = present(bounds.max_history, "bounds.max_history")?;
        let analyzer = present(draft.analyzer, "analyzer")?;
        channels.sort();
        choices.sort();
        Ok(Self::Realizability(RealizabilityKey {
            definition_key,
            strategy_profile,
            premises: ProtocolPremises {
                channels,
                choices,
                role_knowledge,
                environment,
            },
            bounds: Bounds {
                max_depth,
                max_history: Some(max_history),
            },
            analyzer,
        }))
    }

    /// Read a stored key, rebuilding it through the builder so an absent
    /// member is refused exactly as the builder refuses it (PA-13).
    ///
    /// # Errors
    ///
    /// [`KeyReadError::Missing`] naming the absent member (a nested one by
    /// its path), [`KeyReadError::UnknownKind`] for a `kind` of no key,
    /// [`KeyReadError::Malformed`] for a member in a shape the key does not
    /// admit, such as `bounds.max_history` on a projection key.
    pub fn from_json(text: &str) -> Result<Self, KeyReadError> {
        let mut document: serde_json::Map<String, serde_json::Value> = serde_json::from_str(text)?;
        let kind = document
            .remove("kind")
            .ok_or_else(|| MissingMember::new("kind"))?;
        require_nested(&document)?;
        match kind.as_str() {
            Some("projection") => {
                let draft = serde_json::from_value(document.into())?;
                Ok(Self::projection(draft)?)
            }
            Some("realizability") => {
                let draft = serde_json::from_value(document.into())?;
                Ok(Self::realizability(draft)?)
            }
            _ => Err(KeyReadError::UnknownKind(kind.to_string())),
        }
    }

    /// This key's stored structured value.
    ///
    /// # Errors
    ///
    /// A serialization error, which no key value produces.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// A run limit that stopped a run before its method completed, stating the
/// limit and the value used (PA-14).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunLimit {
    /// `max_states` or `max_transitions`.
    pub limit: crate::ModelCheckLimit,
    /// The value in force.
    pub value: u64,
}

/// The cause slug of a construct outside the `bounded-static` profile
/// (FR-575), under the `unsupported_projection` code.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UnsupportedSlug {
    /// A protocol activated `on each` trigger rather than `on origin`.
    ActivationOnEach,
    /// A role with a dynamic population.
    DynamicRolePopulation,
    /// A queue with a symbolic capacity.
    SymbolicQueueCapacity,
    /// A `join` that is not `all`, or carries an `outstanding` policy.
    UnsupportedJoin,
    /// A `repeat`.
    Repeat,
    /// A compensation.
    Compensation,
    /// A timed `await`.
    TimedAwait,
    /// A control outside the profile's list: `check`, `attempt`, `effect`,
    /// `event`, `commit` or `fence`.
    UnsupportedControl,
    /// Realizability only: a construct `qsl-eval` refuses to step.
    EvaluatorUnsupported,
}

impl UnsupportedSlug {
    /// The slug's kebab-case spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ActivationOnEach => "activation-on-each",
            Self::DynamicRolePopulation => "dynamic-role-population",
            Self::SymbolicQueueCapacity => "symbolic-queue-capacity",
            Self::UnsupportedJoin => "unsupported-join",
            Self::Repeat => "repeat",
            Self::Compensation => "compensation",
            Self::TimedAwait => "timed-await",
            Self::UnsupportedControl => "unsupported-control",
            Self::EvaluatorUnsupported => "evaluator-unsupported",
        }
    }

    /// The catalog code every slug sits under.
    #[must_use]
    pub const fn code(self) -> &'static str {
        "unsupported_projection"
    }
}

/// A construct outside the bounded static profile (FR-575): its slug and
/// the checked node id of the construct.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedCause {
    /// The construct's slug.
    pub slug: UnsupportedSlug,
    /// The construct's checked node id.
    pub node: CheckedNodeId,
    /// The named policy of an `unsupported-join`, or the `qsl-eval` cause.
    pub detail: Option<String>,
}

/// Why a search is inconclusive (FR-573).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InconclusiveCause {
    /// The method completed up to the horizon without a counterexample.
    Horizon,
    /// A realizability premise the claim did not state (PA-8).
    PremiseNotStated {
        /// `role_knowledge` or `environment`.
        member: &'static str,
    },
    /// The counterexample's replay disagreed with the checked graph
    /// (ADR-018 SM-7).
    ReplayDisagreed,
    /// Realizability: a role's observation class has no common winning
    /// action and no refutation is forced (PA-18).
    Unforced,
    /// Realizability: the strategy table failed the in-engine replay walk
    /// (PA-19).
    StrategyReplayFailed,
    /// Realizability: a channel's open delivery range was cut at its
    /// capacity and the horizon, and the result depended on it (PA-7).
    CutCount,
    /// Realizability: the result depended on an evaluation `qsl-eval` left
    /// undecided: an undetermined clause or protocol expression, or a
    /// terminal predicate undefined or refused at a terminal state.
    Undecided,
}

impl InconclusiveCause {
    /// The cause's FR-331 wire spelling (FR-573).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Horizon => "bound-reached",
            Self::PremiseNotStated { .. } => "premise-not-stated",
            Self::ReplayDisagreed => "replay-disagreed",
            Self::Unforced => "unforced-refutation",
            Self::StrategyReplayFailed => "strategy-replay-failed",
            Self::CutCount => "cut-count",
            Self::Undecided => "undecided-evaluation",
        }
    }
}

/// The settlement of one analysis, by the FR-360 labels (PA-20). `A` is the
/// artifact of a `proved` result, `C` the counterexample of a `refuted` one.
/// A run that a limit stopped is `Failed` and carries neither.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnalysisOutcome<A, C> {
    /// Sound, closed-scope, `Uncertified`.
    Proved(A),
    /// A decisive counterexample.
    Refuted(C),
    /// Undecided: a horizon or an unstated premise.
    Inconclusive(InconclusiveCause),
    /// A construct outside the profile.
    Unsupported(UnsupportedCause),
    /// A run limit stopped the run: `resource-incomplete`.
    Failed(RunLimit),
}

/// A result: its key, stored as a structured value, and its outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolAnalysisResult<A, C> {
    /// The key the result is a function of.
    pub key: ProtocolAnalysisKey,
    /// The settlement.
    pub outcome: AnalysisOutcome<A, C>,
}

pub mod arena;
mod artifact;
mod realizability_replay;
mod settle;
mod strategy;
mod structure;
pub use artifact::*;
pub use realizability_replay::*;
pub use settle::*;
pub use strategy::*;
pub use structure::*;

#[cfg(test)]
mod tests;
