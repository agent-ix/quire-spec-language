// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-3, PA-9: the global event structure of a checked protocol,
//! read from a checked package only through `qsl_package::protocol_view`.
//! It sits in `spine`, the checked route, because it takes a
//! `CheckedPackageV2` (ADR-032 CK-3).

use std::collections::BTreeMap;

use qsl_package::protocol_view::{
    causal_relation, channel_endpoints, channel_ordering, control_body, decision_reads,
    fifo_key_reads_message, protocol_view, ChannelOrdering, ControlBody, EdgeKind, Port, PortRef,
    ViewRefusal,
};
use quire_contract_model::{CheckedNodeId, CheckedPackageV2, CheckedSemanticNodeV2};

use crate::protocol_analysis::{
    Comm, GlobalEvent, GlobalEventKind, GlobalStructure, RunBudget, RunLimit, StructureInput,
    UniformFifo,
};

/// Why a structure was not built.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum StructureError {
    /// A protocol read refused.
    #[error(transparent)]
    View(#[from] ViewRefusal),
    /// A node the view named is not in the package.
    #[error("the package holds no node {0:?}")]
    UnknownNode(CheckedNodeId),
    /// A run limit stopped the build.
    #[error("the structure build reached {} = {}", .0.limit, .0.value)]
    Limit(RunLimit),
}

struct Reader<'p> {
    package: &'p CheckedPackageV2,
    receives: Vec<(CheckedNodeId, CheckedNodeId)>,
    input: StructureInput,
}

impl<'p> Reader<'p> {
    fn node(&self, id: &CheckedNodeId) -> Result<&'p CheckedSemanticNodeV2, StructureError> {
        self.package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| &node.node_id == id)
            .ok_or_else(|| StructureError::UnknownNode(id.clone()))
    }

    fn comm(&self, channel: &CheckedNodeId) -> Result<Comm, StructureError> {
        let ends = channel_endpoints(self.node(channel)?)?;
        Ok(Comm {
            channel: channel.clone(),
            sender: ends.sender.clone(),
            receiver: ends.receiver.clone(),
            message_type: ends.message_type.clone(),
        })
    }

    fn walk(
        &mut self,
        id: &CheckedNodeId,
        within: &mut Vec<CheckedNodeId>,
    ) -> Result<(), StructureError> {
        within.push(id.clone());
        let enter = PortRef {
            node: id.clone(),
            port: Port::Enter,
        };
        let exit = PortRef {
            node: id.clone(),
            port: Port::Exit,
        };
        let node = self.node(id)?;
        match control_body(node)? {
            ControlBody::Sequence { children } => {
                for child in children {
                    self.walk(child, within)?;
                }
            }
            ControlBody::Choice { owner, cases } => {
                let reads = decision_reads(self.package, node)?;
                let carriers = self
                    .receives
                    .iter()
                    .filter(|(_, binder)| reads.contains(binder))
                    .map(|(receive, _)| receive.clone())
                    .collect();
                self.input.events.push(GlobalEvent {
                    at: enter.clone(),
                    out: enter,
                    kind: GlobalEventKind::Choice {
                        owner: owner.clone(),
                        cases: cases
                            .iter()
                            .map(|(label, body)| ((*label).to_owned(), (*body).clone()))
                            .collect(),
                        carriers,
                    },
                    within: within.clone(),
                });
                for (_, body) in cases {
                    self.walk(body, within)?;
                }
            }
            ControlBody::Parallel { branches, .. } => {
                let bodies: Vec<CheckedNodeId> =
                    branches.iter().map(|(_, body)| (*body).clone()).collect();
                self.input.events.push(GlobalEvent {
                    at: enter.clone(),
                    out: enter,
                    kind: GlobalEventKind::Region {
                        branches: bodies.clone(),
                    },
                    within: within.clone(),
                });
                self.input.events.push(GlobalEvent {
                    at: exit.clone(),
                    out: exit,
                    kind: GlobalEventKind::Join { branches: bodies },
                    within: within.clone(),
                });
                for (_, body) in branches {
                    self.walk(body, within)?;
                }
            }
            ControlBody::Send { channel, .. } => {
                let comm = self.comm(channel)?;
                self.input.events.push(GlobalEvent {
                    at: enter,
                    out: exit,
                    kind: GlobalEventKind::Send(comm),
                    within: within.clone(),
                });
            }
            ControlBody::Receive { channel, of, .. } => {
                self.input.receive_of.insert(id.clone(), of.clone());
                let comm = self.comm(channel)?;
                self.input.events.push(GlobalEvent {
                    at: enter,
                    out: exit,
                    kind: GlobalEventKind::Receive(comm),
                    within: within.clone(),
                });
            }
            ControlBody::Other(_) => {}
        }
        within.pop();
        Ok(())
    }
}

impl GlobalStructure {
    /// The structure of the protocol clause `protocol`, from its FR-519
    /// view and causal relation (PA-3, PA-9). The caller has already
    /// refused every construct outside the `bounded-static` profile.
    ///
    /// # Errors
    ///
    /// [`StructureError`] when a view function refuses, a node is absent, or
    /// `budget` stops the build.
    pub fn from_package(
        package: &CheckedPackageV2,
        protocol: &CheckedNodeId,
        budget: RunBudget,
    ) -> Result<Self, StructureError> {
        let view = protocol_view(package, protocol)?;
        let relation = causal_relation(package, protocol)?;
        let mut reader = Reader {
            package,
            receives: Vec::new(),
            input: StructureInput::default(),
        };
        for id in view.controls() {
            if let ControlBody::Receive { binder, .. } = control_body(reader.node(id)?)? {
                reader.receives.push(((*id).clone(), binder.clone()));
            }
        }
        reader.input.edges = relation
            .edges()
            .iter()
            .filter(|edge| edge.kind != EdgeKind::RepeatBack)
            .map(|edge| (edge.from.clone(), edge.to.clone()))
            .collect();
        let mut fifo = BTreeMap::new();
        for id in view.channels() {
            let node = reader.node(id)?;
            if channel_ordering(node)? == ChannelOrdering::Fifo
                && !fifo_key_reads_message(package, node)?
            {
                fifo.insert(
                    (*id).clone(),
                    UniformFifo {
                        channel: (*id).clone(),
                    },
                );
            }
        }
        reader.input.uniform_fifo = fifo.into_values().collect();
        reader.walk(view.run(), &mut Vec::new())?;
        GlobalStructure::build(reader.input, budget).map_err(StructureError::Limit)
    }
}

/// Why the protocol analysis replay arm did not run.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProtocolReplayRefusal {
    /// The source is no `ProtocolAnalysis` source.
    #[error("invalid_runtime_input/invalid-value at /replay/source: protocol analysis replay requires a ProtocolAnalysis source")]
    WrongSource,
    /// The evidence's key is not a projection key.
    #[error("invalid_runtime_input/invalid-value: the evidence key is not a projection key")]
    NotAProjectionKey,
    /// The structure did not build.
    #[error(transparent)]
    Structure(#[from] StructureError),
}

/// The `ReplaySource::ProtocolAnalysis` arm (ADR-036 PA-20): re-walk the
/// counterexample's stated paths against the checked graph of `protocol`,
/// check each step is an admitted successor and that the walk ends in the
/// stated divergence. An arm that disagrees settles `inconclusive`.
///
/// # Errors
///
/// [`ProtocolReplayRefusal`] when the source is another arm or the graph
/// does not build.
pub fn replay_protocol_analysis(
    package: &CheckedPackageV2,
    protocol: &CheckedNodeId,
    source: &crate::witness::ReplaySource,
    budget: RunBudget,
) -> Result<crate::result::ReplayResult, ProtocolReplayRefusal> {
    use crate::protocol_analysis::{
        replay_projection_counterexample, ProtocolAnalysisKey, ProtocolReplayResult,
        ProtocolReplaySettlement,
    };
    let crate::witness::ReplaySource::ProtocolAnalysis(evidence) = source else {
        return Err(ProtocolReplayRefusal::WrongSource);
    };
    let ProtocolAnalysisKey::Projection(key) = &evidence.counterexample.key else {
        return Err(ProtocolReplayRefusal::NotAProjectionKey);
    };
    let structure = GlobalStructure::from_package(package, protocol, budget)?;
    let settlement = match replay_projection_counterexample(
        &structure,
        &evidence.counterexample,
        key.bounds.max_depth,
    ) {
        Ok(()) => ProtocolReplaySettlement::Reproduced,
        Err(disagreement) => ProtocolReplaySettlement::Inconclusive(disagreement),
    };
    Ok(crate::result::ReplayResult::ProtocolAnalysis(
        ProtocolReplayResult { settlement },
    ))
}
