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
    channel_endpoints, channel_ordering, choice_visibility, delivery, fifo_key, protocol_view,
    queue_capacity, queue_channel, queue_overflow, ChannelOrdering, Delivery, LossSelector,
    Overflow, QueueCapacity, ViewRefusal,
};
use quire_contract_model::{CheckedNodeId, CheckedPackageV2, CheckedSemanticNodeV2};
use serde::{Deserialize, Deserializer, Serialize};

/// The refusal of a key with an absent member: `invalid_runtime_input` with
/// cause `missing-member`, naming the member (FR-572, PA-13).
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("invalid_runtime_input/missing-member: the analysis key carries no `{member}`")]
pub struct MissingMember {
    /// The key member that is absent.
    pub member: &'static str,
}

impl MissingMember {
    /// The catalog code of the refusal.
    #[must_use]
    pub const fn code(self) -> &'static str {
        "invalid_runtime_input"
    }

    /// The catalog cause of the refusal.
    #[must_use]
    pub const fn cause(self) -> &'static str {
        "missing-member"
    }
}

/// Why a stored key did not read.
#[derive(Debug, thiserror::Error)]
pub enum KeyReadError {
    /// A member is absent.
    #[error(transparent)]
    Missing(#[from] MissingMember),
    /// The document is not a key of either kind.
    #[error("the analysis key does not read: {0}")]
    Malformed(#[from] serde_json::Error),
}

/// The two bounds that are key members (PA-14). There is no default.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize)]
pub struct Bounds {
    /// The horizon, in events along a path or play.
    pub max_depth: u64,
    /// The local history length; a member of a realizability key only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_history: Option<u64>,
}

/// A [`Bounds`] whose members may be absent.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundsDraft {
    /// The horizon.
    pub max_depth: Option<u64>,
    /// The local history length.
    pub max_history: Option<u64>,
}

impl BoundsDraft {
    fn depth(self) -> Result<u64, MissingMember> {
        self.max_depth.ok_or(MissingMember {
            member: "max_depth",
        })
    }
}

/// A channel's ordering premise (FR-054).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "kebab-case")]
pub enum OrderingPremise {
    /// No order between receives.
    Unordered,
    /// Receives follow the FIFO key.
    Fifo {
        /// The message binder node, as its canonical string.
        binder: String,
        /// The key expression node, as its canonical string.
        key: String,
    },
}

/// A channel's delivery premise (FR-054, FR-509).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelPremise {
    /// The channel node, as its canonical string.
    pub channel: String,
    /// The sending role node.
    pub sender: String,
    /// The receiving role node.
    pub receiver: String,
    /// The message type node.
    pub message_type: String,
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
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoicePremise {
    /// The `choice` control node.
    pub choice: String,
    /// The owning node.
    pub owner: String,
    /// The visible `expression` nodes, in the order the node carries them.
    pub visible: Vec<String>,
}

/// The premises of a protocol analysis (PA-6, PA-8). Groups (a) and (b) are
/// built from the linked package; groups (c) and (d) are authored by a
/// realizability claim. `None` is "not stated" and `Some` of an empty set is
/// "declared empty": different premises, both present.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
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
    #[error("the package holds no node {0}")]
    UnknownNode(String),
    /// A queue belongs to no channel of the clause.
    #[error("the queue {queue} names no channel of the clause")]
    OrphanQueue {
        /// The queue node, as its canonical string.
        queue: String,
    },
    /// A channel of the clause has no queue.
    #[error("the channel {channel} has no queue")]
    ChannelWithoutQueue {
        /// The channel node, as its canonical string.
        channel: String,
    },
    /// A node id has no canonical string.
    #[error("a node id does not encode: {0}")]
    Encoding(String),
}

/// The canonical string of a checked node id: its canonical JSON object.
///
/// # Errors
///
/// [`PremiseRefusal::Encoding`] when the id does not serialize.
pub fn node_text(id: &CheckedNodeId) -> Result<String, PremiseRefusal> {
    serde_json::to_string(id).map_err(|error| PremiseRefusal::Encoding(error.to_string()))
}

impl ProtocolPremises {
    /// Groups (a) and (b) of the clause `protocol`, read only through
    /// `qsl_package::protocol_view` (PA-3). Groups (c) and (d) are `None`:
    /// no premise is inferred (PA-8).
    ///
    /// # Errors
    ///
    /// [`PremiseRefusal`] when a view function refuses, a queue and a channel
    /// do not pair, or a node is not in the package.
    pub fn from_view(
        package: &CheckedPackageV2,
        protocol: &CheckedNodeId,
    ) -> Result<Self, PremiseRefusal> {
        let view = protocol_view(package, protocol)?;
        let queues = view
            .queues()
            .iter()
            .map(|queue| {
                let node = node_of(package, queue)?;
                Ok((queue_channel(node)?, node))
            })
            .collect::<Result<Vec<_>, PremiseRefusal>>()?;
        for (channel, node) in &queues {
            if !view.channels().iter().any(|held| held == channel) {
                return Err(PremiseRefusal::OrphanQueue {
                    queue: node_text(&node.node_id)?,
                });
            }
        }
        let mut channels = Vec::with_capacity(view.channels().len());
        for id in view.channels() {
            let node = node_of(package, id)?;
            let (_, queue) = queues
                .iter()
                .find(|(channel, _)| channel == id)
                .ok_or_else(|| PremiseRefusal::ChannelWithoutQueue {
                    channel: node_text(id).unwrap_or_default(),
                })?;
            channels.push(channel_premise(node, queue)?);
        }
        let mut choices = Vec::new();
        for id in view.controls() {
            let node = node_of(package, id)?;
            if qsl_package::protocol_view::control_operation(node)
                != Ok(qsl_package::protocol_view::ControlOperation::Choice)
            {
                continue;
            }
            let found = choice_visibility(node)?;
            choices.push(ChoicePremise {
                choice: node_text(found.choice)?,
                owner: node_text(found.owner)?,
                visible: found
                    .visible
                    .iter()
                    .map(|id| node_text(id))
                    .collect::<Result<_, _>>()?,
            });
        }
        channels.sort();
        choices.sort();
        Ok(Self {
            channels,
            choices,
            role_knowledge: None,
            environment: None,
        })
    }
}

fn node_of<'p>(
    package: &'p CheckedPackageV2,
    id: &CheckedNodeId,
) -> Result<&'p CheckedSemanticNodeV2, PremiseRefusal> {
    package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &node.node_id == id)
        .ok_or_else(|| PremiseRefusal::UnknownNode(node_text(id).unwrap_or_default()))
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
                binder: node_text(key.binder)?,
                key: node_text(key.key)?,
            }
        }
    };
    Ok(ChannelPremise {
        channel: node_text(&channel.node_id)?,
        sender: node_text(ends.sender)?,
        receiver: node_text(ends.receiver)?,
        message_type: node_text(ends.message_type)?,
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
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzerIdentity {
    /// `qsl-analyze::protocol_analysis`.
    pub identity: String,
    /// The crate version.
    pub version: String,
}

/// The members of a projection key (FR-174-AC-6).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize)]
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
    pub bounds: Option<BoundsDraft>,
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

fn present<T>(member: Option<T>, name: &'static str) -> Result<T, MissingMember> {
    member.ok_or(MissingMember { member: name })
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
        let max_depth = present(draft.bounds, "bounds")?.depth()?;
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
        let max_depth = bounds.depth()?;
        let max_history = present(bounds.max_history, "max_history")?;
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
    /// [`KeyReadError::Missing`] naming the absent member,
    /// [`KeyReadError::Malformed`] for a document of no key kind.
    pub fn from_json(text: &str) -> Result<Self, KeyReadError> {
        let mut document: serde_json::Map<String, serde_json::Value> = serde_json::from_str(text)?;
        let kind = document.remove("kind");
        match kind.as_ref().and_then(serde_json::Value::as_str) {
            Some("projection") => {
                let draft = serde_json::from_value(document.into())?;
                Ok(Self::projection(draft)?)
            }
            Some("realizability") => {
                let draft = serde_json::from_value(document.into())?;
                Ok(Self::realizability(draft)?)
            }
            _ => Err(MissingMember { member: "kind" }.into()),
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
    /// `max_states`, `max_transitions` or the evaluation meter.
    pub limit: &'static str,
    /// The value in force.
    pub value: u64,
}

/// A construct outside the bounded static profile (FR-575), by the cause the
/// engine states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedCause(pub String);

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

#[cfg(test)]
mod tests;
