// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-16, PA-19: how a realizability play reads the one `qsl-eval`
//! protocol transition system: which role takes a step, with what action and
//! observation, and what a state's findings decide. The engine's search, its
//! strategy replay walk and the counterexample replay arm (PA-20) all read
//! the arena through these functions, so each computes observation classes
//! with the one `observations` function.

use std::collections::BTreeMap;

use qsl_eval::simulation::model::ModelFinding;
use qsl_eval::simulation::protocol::{
    ProtocolState, ProtocolSystem, ProtocolTransition, Sc, StepKind,
};
use qsl_eval::simulation::ExpansionStop;
use qsl_package::protocol_view::{protocol_view, ViewRefusal};
use quire_contract_model::{CheckedNodeId, CheckedPackageV2};

use super::{ObservationClass, Observed, ProtocolPremises, StrategyAction};

/// Why the arena could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ArenaError {
    /// A protocol read refused.
    #[error(transparent)]
    View(#[from] ViewRefusal),
    /// An expansion stopped: an evaluation broke.
    #[error("the arena expansion stopped: {0:?}")]
    Stopped(ExpansionStop),
    /// A step names a node the linked package does not hold.
    #[error("the step at `{0}` names no node of the linked package")]
    UnnamedNode(String),
}

impl From<ExpansionStop> for ArenaError {
    fn from(stop: ExpansionStop) -> Self {
        Self::Stopped(stop)
    }
}

/// The names the arena's steps resolve to in the linked package.
pub struct ArenaNames {
    /// Each control by its declared name; `None` for a shared name.
    controls: BTreeMap<String, Option<CheckedNodeId>>,
    /// Each channel by its declared name; `None` for a shared name.
    channels: BTreeMap<String, Option<CheckedNodeId>>,
    /// Each channel's (sender, receiver) roles, by channel node.
    ends: BTreeMap<CheckedNodeId, (usize, usize)>,
    /// The roles, ascending.
    roles: Vec<CheckedNodeId>,
    /// The progress and terminal obligation: the `finish` member.
    finish: CheckedNodeId,
    /// The holder of the safety obligations: the clause.
    clause: CheckedNodeId,
}

impl ArenaNames {
    /// The names of the protocol clause `protocol` of `package`.
    ///
    /// # Errors
    ///
    /// [`ArenaError::View`] when a protocol read refuses.
    pub fn new(
        package: &CheckedPackageV2,
        protocol: &CheckedNodeId,
        premises: &ProtocolPremises,
    ) -> Result<Self, ArenaError> {
        let view = protocol_view(package, protocol)?;
        let graph = package.resolved_graph();
        // `qsl-eval` names a control by its path through the `run` tree with
        // case labels (`Main/Decide/yes/Ship/PickIt`) and a channel by its
        // qualified name (`Flow::Pick`); the package declares each under the
        // clause by its own name. A name two nodes share resolves to neither.
        let name_of = |id: &CheckedNodeId| {
            graph
                .nodes
                .iter()
                .find(|node| &node.node_id == id)
                .and_then(|node| node.declaration.as_ref())
                .and_then(|declaration| declaration.qualified_name.last())
                .map(|segment| segment.to_string())
        };
        let mut controls = BTreeMap::new();
        let mut channels = BTreeMap::new();
        for (held, ids) in [
            (&mut controls, view.controls()),
            (&mut channels, view.channels()),
        ] {
            for id in ids {
                if let Some(name) = name_of(id) {
                    held.entry(name)
                        .and_modify(|shared: &mut Option<CheckedNodeId>| *shared = None)
                        .or_insert_with(|| Some((*id).clone()));
                }
            }
        }
        let mut roles: Vec<CheckedNodeId> = view.roles().iter().map(|id| (*id).clone()).collect();
        roles.sort();
        let index = |id: &CheckedNodeId| roles.binary_search(id).ok();
        let mut ends = BTreeMap::new();
        for channel in &premises.channels {
            if let (Some(sender), Some(receiver)) =
                (index(&channel.sender), index(&channel.receiver))
            {
                ends.insert(channel.channel.clone(), (sender, receiver));
            }
        }
        Ok(Self {
            controls,
            channels,
            ends,
            finish: view.finish().clone(),
            clause: view.clause().clone(),
            roles,
        })
    }

    /// The roles, ascending by node.
    #[must_use]
    pub fn roles(&self) -> &[CheckedNodeId] {
        &self.roles
    }

    /// The `finish` member: the progress and terminal obligation.
    #[must_use]
    pub fn finish(&self) -> &CheckedNodeId {
        &self.finish
    }

    /// The clause: the holder of the safety obligations.
    #[must_use]
    pub fn clause(&self) -> &CheckedNodeId {
        &self.clause
    }

    /// The control `qsl-eval` names by the path `path`: `qsl-eval` exposes
    /// a step's control only as this string.
    #[ix_trace_rs::string_edge]
    fn node(&self, path: &str) -> Result<&CheckedNodeId, ArenaError> {
        let name = path.rsplit('/').next().unwrap_or(path);
        self.controls
            .get(name)
            .and_then(Option::as_ref)
            .ok_or_else(|| ArenaError::UnnamedNode(path.to_owned()))
    }

    /// The (sender, receiver) roles of the channel `qsl-eval` names
    /// `qualified`: `qsl-eval` exposes a step's channel only as this string.
    #[ix_trace_rs::string_edge]
    fn channel(&self, qualified: &str) -> Result<(usize, usize), ArenaError> {
        let name = qualified.rsplit("::").next().unwrap_or(qualified);
        self.channels
            .get(name)
            .and_then(Option::as_ref)
            .and_then(|channel| self.ends.get(channel))
            .copied()
            .ok_or_else(|| ArenaError::UnnamedNode(qualified.to_owned()))
    }
}

/// A role's step: the role (an index into [`ArenaNames::roles`]), its action
/// and what it observes.
#[derive(Clone, Debug)]
pub struct RoleStep {
    /// The role's index.
    pub role: usize,
    /// The action.
    pub action: StrategyAction,
    /// What the role observes.
    pub observed: Observed,
}

/// What a play has shown its roles: each started instance's captures, as
/// bound when the play first reached the instance live, by ordinal, and
/// each role's own sends and receives with their records, in step order.
#[derive(Clone)]
pub struct Seen {
    captures: BTreeMap<u64, Vec<u8>>,
    steps: Vec<ObservationClass>,
}

impl Seen {
    /// Nothing shown to any of `roles` roles.
    #[must_use]
    pub fn new(roles: usize) -> Self {
        Self {
            captures: BTreeMap::new(),
            steps: vec![Vec::new(); roles],
        }
    }

    /// This play extended by `step`, when a role takes it.
    #[must_use]
    pub fn extended(&self, step: Option<&RoleStep>) -> Self {
        let mut next = self.clone();
        if let Some(step) = step {
            next.steps[step.role].push(step.observed.clone());
        }
        next
    }

    /// Whether some role's own steps exceed `max_history`.
    #[must_use]
    pub fn exceeds(&self, max_history: u64) -> bool {
        self.steps
            .iter()
            .any(|own| u64::try_from(own.len()).unwrap_or(u64::MAX) > max_history)
    }

    /// The observation of every role after this play: the bound captures
    /// every role holds (FR-218), then the role's own sends and receives in
    /// step order (PA-16). The one function the search, the strategy walk
    /// and the counterexample replay compute classes with.
    #[must_use]
    pub fn observations(&self) -> Vec<ObservationClass> {
        self.steps
            .iter()
            .map(|own| {
                self.captures
                    .iter()
                    .map(|(instance, payload)| Observed::Captures {
                        instance: *instance,
                        payload: payload.clone(),
                    })
                    .chain(own.iter().cloned())
                    .collect()
            })
            .collect()
    }
}

/// What a state's findings decide about a play.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindingValue {
    /// A violated invariant loses the play.
    Lose,
    /// An undecided evaluation, or a weak-access shape (which `qsl-eval`
    /// reports with no successor, so the terminal class would otherwise blame
    /// `finish`), leaves it open.
    Undecided,
}

/// The findings' value; the FR-220 deferred and shared records classify
/// accesses without violating an obligation.
#[must_use]
pub fn finding_value(findings: &[ModelFinding]) -> Option<FindingValue> {
    let mut value = None;
    for finding in findings {
        match finding {
            ModelFinding::InvariantViolated { .. } => return Some(FindingValue::Lose),
            ModelFinding::InvariantUndetermined { .. }
            | ModelFinding::ProtocolExpressionUndetermined { .. }
            | ModelFinding::ContractUndetermined { .. }
            | ModelFinding::WeakAccessShape(_) => {
                value.get_or_insert(FindingValue::Undecided);
            }
            ModelFinding::WeakAccessDeferred { .. } | ModelFinding::WeakAccessShared { .. } => {}
        }
    }
    value
}

/// The arena: the one `qsl-eval` protocol system and the names its steps
/// resolve to.
pub struct Arena<'s, 'a> {
    /// The protocol system.
    pub system: &'s ProtocolSystem<'a, Sc>,
    /// The names.
    pub names: ArenaNames,
}

impl Arena<'_, '_> {
    /// The role that takes `transition`, with its action and observation;
    /// `None` for an environment step (PA-16).
    ///
    /// # Errors
    ///
    /// [`ArenaError`] when the step names no node or its record does not
    /// encode.
    pub fn classify(
        &self,
        transition: &ProtocolTransition,
    ) -> Result<Option<RoleStep>, ArenaError> {
        let unnamed = |what: &str| ArenaError::UnnamedNode(what.to_owned());
        let path = transition.node().unwrap_or_default();
        let (role, receive_from) = match transition.kind() {
            StepKind::Send => {
                let channel = transition
                    .send_record()
                    .map(|record| record.channel())
                    .ok_or_else(|| unnamed(path))?;
                (self.names.channel(channel)?.0, None)
            }
            StepKind::Receive => {
                let (channel, _, _) = transition.received().ok_or_else(|| unnamed(path))?;
                let (sender, receiver) = self.names.channel(channel)?;
                (receiver, Some(sender))
            }
            _ => return Ok(None),
        };
        let node = self.names.node(path)?.clone();
        let payload = self
            .system
            .binder_record_bytes(transition)?
            .unwrap_or_default();
        Ok(Some(match receive_from {
            Some(sender) => RoleStep {
                role,
                action: StrategyAction::Receive {
                    peer: self.names.roles[sender].clone(),
                },
                observed: Observed::Received { node, payload },
            },
            None => RoleStep {
                role,
                action: StrategyAction::Send { node: node.clone() },
                observed: Observed::Sent { node, payload },
            },
        }))
    }

    /// `seen` with the captures of every instance live at `state` that it
    /// has not bound yet. An instance's captures are bound once, when the
    /// play first reaches it live, and never re-read: they are frozen while
    /// it lives and gone once it finishes.
    ///
    /// # Errors
    ///
    /// [`ArenaError::Stopped`] when a capture does not evaluate.
    pub fn bind(&self, state: &ProtocolState<Sc>, mut seen: Seen) -> Result<Seen, ArenaError> {
        for (instance, payload) in self.system.capture_bytes(state)? {
            seen.captures.entry(instance).or_insert(payload);
        }
        Ok(seen)
    }
}
