// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124: the intended terminal states of a state model, from the unit's
//! `terminal` declarations (ADR-018 DL-1).
//!
//! The assembler resolves each declaration into a
//! [`TerminalClauseDeclaration`] and refuses a second declaration for one
//! model alias. S3 checks a `when` predicate as the one clause checker checks
//! an invariant (FR-104), so a read of a parameter, a result or a pre-state
//! refuses as it does in an invariant. The graph keeps one
//! [`CheckedTerminal`] per selected model: [`TerminalDeclaration::None`] for a
//! model with no declaration.
//!
//! The predicate's `self` is the model's state object: the one object type
//! the model's populations range over, so `self.value = 3` holds at a state
//! exactly when it holds for every object of that population, as an
//! invariant's does.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::string_edge;
use qsl_forms::{ExpressionForm, StateClauseKind};
use qsl_foundation::selection::ProfileSelection;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::Span;
use quire_exact::{EffectiveId, NodeKey, ValueType};

use super::ir::{CheckedBody, CheckedNode};
use super::node_key::{BindingValue, BodyTerm, LeafTerm, LiteralValue, MemberTerm, NodeTag};
use super::refusal::{CheckCause, CheckRefusal};
use super::state_clause::{check_clause, ClauseDeclarations, ClauseView, PopulationDomain};
use super::CheckedGraph;
use super::SemanticNode;
use crate::family::CheckContext;

/// One `terminal` declaration the assembler resolved (FR-124 "Inputs"): its
/// `using` alias to a profile selection and its model alias to a selected
/// model.
#[derive(Clone, Debug)]
pub struct TerminalClauseDeclaration {
    /// The declared name.
    pub name: String,
    /// The model alias the declaration is on.
    pub model: String,
    /// The profile selection the `using` alias resolved to.
    pub selection: ProfileSelection,
    /// The span of the whole declaration.
    pub span: Span,
    /// Which terminal states the declaration marks intended.
    pub kind: TerminalClauseKind,
}

/// The two forms of a `terminal` declaration, resolved.
#[derive(Clone, Debug)]
pub enum TerminalClauseKind {
    /// `terminal any`: every terminal state is intended.
    Any,
    /// `terminal when { P }`: a terminal state satisfying `P` is intended.
    When {
        /// The predicate as S2 built it.
        predicate: ExpressionForm,
        /// The model's state object, the type of `self`.
        context: EffectiveId,
        /// The population the object type ranges over.
        population: Option<PopulationDomain>,
    },
}

/// The checked body of a `terminal when` predicate: a Boolean over `self`,
/// the evaluation reads at each state's observation.
#[derive(Debug)]
pub struct CheckedTerminalPredicate {
    pub(crate) parameters: Vec<(String, ValueType)>,
    pub(crate) body: CheckedBody,
    pub(crate) slots: usize,
    pub(crate) slot_names: Vec<String>,
}

impl CheckedTerminalPredicate {
    /// `self` with its type: the one slot the body is checked over.
    pub fn parameters(&self) -> &[(String, ValueType)] {
        &self.parameters
    }

    /// The checked Boolean body.
    pub fn body(&self) -> CheckedNode<'_> {
        self.body.root()
    }

    /// The number of evaluation slots of the body.
    pub fn slots(&self) -> usize {
        self.slots
    }
}

/// The intended terminal states of a state model (FR-124 "Outputs").
#[derive(Debug)]
pub enum TerminalDeclaration {
    /// The model declares none: it intends no terminal state.
    None,
    /// A terminal state is intended when the predicate holds at it.
    When(CheckedTerminalPredicate),
    /// Every terminal state is intended.
    Any,
}

/// One selected model's checked terminal declaration.
#[derive(Debug)]
pub struct CheckedTerminal {
    pub(crate) model: String,
    pub(crate) name: Option<String>,
    pub(crate) declaration: TerminalDeclaration,
    /// The `claim` occurrence `[node, "claim", ordinal]` of the declaration's
    /// `temporal`/`terminal` node, set when a package is built; `None` for a
    /// model with no declaration.
    pub(crate) occurrence: Option<OccurrenceKey>,
}

impl CheckedTerminal {
    /// The model alias.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The declared name; `None` when the model declares none.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The occurrence of the declaration's emitted node (FR-411): how a
    /// reader of the emitted package finds the declaration.
    pub fn occurrence(&self) -> Option<&OccurrenceKey> {
        self.occurrence.as_ref()
    }

    /// The model's intended terminal states.
    pub fn declaration(&self) -> &TerminalDeclaration {
        &self.declaration
    }
}

/// A refusal of a `terminal when` predicate's checking: the declaration it is
/// about and the refusal itself.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TerminalRefusal {
    /// The declaration's index among the package's `terminal` declarations.
    pub declaration: usize,
    /// The checking refusal.
    pub cause: CheckCause,
}

/// Checks the `terminal` declaration `declaration`, the `index`th of the
/// package, and returns it with the package's running node total after it.
pub(crate) fn check_terminal<'a>(
    index: usize,
    declaration: &TerminalClauseDeclaration,
    declarations: &ClauseDeclarations<'a>,
    cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
) -> Result<(TerminalDeclaration, u64), CheckRefusal> {
    let TerminalClauseKind::When {
        predicate,
        context,
        population,
    } = &declaration.kind
    else {
        return Ok((TerminalDeclaration::Any, declarations.nodes_used));
    };
    let view = ClauseView {
        kind: StateClauseKind::Invariant,
        complete_value: super::state_clause::is_complete_value(&declaration.selection),
        context: Some(context.clone()),
        receiver: Some(("self", ValueType::Reference(context.clone()))),
        operation: None,
        context_population: population.clone(),
        frame_population: None,
        binders: &[],
        presence_places: &[],
        body: &predicate.expression,
    };
    let typed = check_clause(&view, declarations, &mut cx.reborrow(declarations.scope)).map_err(
        |refusal| CheckRefusal {
            location: refusal.location,
            cause: CheckCause::Terminal(Box::new(TerminalRefusal {
                declaration: index,
                cause: refusal.cause,
            })),
        },
    )?;
    Ok((
        TerminalDeclaration::When(CheckedTerminalPredicate {
            parameters: typed.parameters,
            body: typed.body,
            slots: typed.slots,
            slot_names: typed.slot_names,
        }),
        typed.nodes_used,
    ))
}

/// The mode of a `temporal`/`terminal` node (QSpec FR-476).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalMode {
    /// `when`: a terminal state is intended when the condition holds.
    When,
    /// `any`: every terminal state is intended.
    Any,
}

impl TerminalMode {
    /// The mode's spelling in the node's `mode` literal (QSpec FR-476).
    pub(crate) const fn wire(self) -> &'static str {
        match self {
            Self::When => "when",
            Self::Any => "any",
        }
    }

    /// The mode a node's `mode` literal spells. `#[string_edge]`: the one
    /// place the written text is read (FR-064).
    #[string_edge]
    fn from_wire(text: &str) -> Option<Self> {
        [Self::When, Self::Any]
            .into_iter()
            .find(|mode| mode.wire() == text)
    }
}

/// Whether `node` is a `temporal`/`terminal` node. `#[string_edge]`: the node
/// form is read by its spelling (FR-064).
#[string_edge]
fn is_terminal_node(node: &SemanticNode) -> bool {
    node.node_tag() == NodeTag::Temporal && node.semantic_form() == "terminal"
}

/// One `temporal`/`terminal` node of a checked graph (FR-411), read from the
/// graph's own semantic nodes: no emission is needed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalNode {
    /// The node.
    pub node: NodeKey,
    /// The `model` node it is on.
    pub model: NodeKey,
    /// The node's mode.
    pub mode: TerminalMode,
}

impl TerminalNode {
    /// The node read from `body`, the aggregate `lowering` writes: `model`
    /// and `mode` first. `None` for a body of another shape, which no
    /// lowering writes under this tag and form.
    fn of(node: NodeKey, body: &BodyTerm) -> Option<Self> {
        let BodyTerm::Aggregate(aggregate) = body else {
            return None;
        };
        let mut members = aggregate.members.iter().map(|member| match member {
            MemberTerm::Binding(binding) => match &binding.value {
                BindingValue::Leaf(leaf) => Some((binding.name.as_str(), leaf)),
                _ => None,
            },
            _ => None,
        });
        let (("model", LeafTerm::Reference { target }), ("mode", LeafTerm::Literal { value, .. })) =
            (members.next()??, members.next()??)
        else {
            return None;
        };
        let LiteralValue::Text(mode) = value else {
            return None;
        };
        let mode = TerminalMode::from_wire(mode)?;
        Some(Self {
            node,
            model: target.admitted()?.clone(),
            mode,
        })
    }
}

/// FR-411 "Deadlock-freedom verdict join": each selected model alias's
/// terminal node key, or `None` for a model with no declaration.
///
/// The node key carries the declaration's mode, model and predicate and not
/// its written name, so two packages that differ only in a model's
/// declaration have different keys here although the deadlock-freedom item's
/// obligation identity is the same (ADR-013 O-09). QSL settlement compares
/// the keys the request was written under with those a run, a certificate
/// check or a replay read.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TerminalKeys(BTreeMap<String, Option<NodeKey>>);

impl TerminalKeys {
    /// The first model alias, in ascending order, whose declaration differs
    /// between `self`, the declarations a request was written under, and
    /// `read`, the declarations a run, check or replay read. An alias one
    /// side does not list has no declaration there.
    pub fn mismatch(&self, read: &Self) -> Option<TerminalMismatch> {
        let key = |keys: &Self, model: &str| keys.0.get(model).cloned().flatten();
        self.0
            .keys()
            .chain(read.0.keys())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .find_map(|model| {
                let requested = key(self, model);
                let recompiled = key(read, model);
                (requested != recompiled).then(|| TerminalMismatch {
                    model: model.clone(),
                    requested,
                    recompiled,
                })
            })
    }
}

/// A model alias whose terminal declaration differs between a request and
/// what a run, check or replay read (FR-411 "Deadlock-freedom verdict
/// join"): `stale_dependency`/`content-mismatch`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalMismatch {
    /// The model alias.
    pub model: String,
    /// The terminal node key the request was written under; `None` for no
    /// declaration.
    pub requested: Option<NodeKey>,
    /// The terminal node key that was read; `None` for no declaration.
    pub recompiled: Option<NodeKey>,
}

impl std::fmt::Display for TerminalMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key = |key: &Option<NodeKey>| {
            key.as_ref()
                .map_or_else(|| "none".to_owned(), ToString::to_string)
        };
        write!(
            f,
            "model {}: requested terminal {}, read terminal {}",
            self.model,
            key(&self.requested),
            key(&self.recompiled)
        )
    }
}

impl CheckedGraph {
    /// FR-411 "Deadlock-freedom verdict join": each selected model's
    /// terminal node key, the node its declaration's `claim` occurrence
    /// names, or `None` for a model with no declaration.
    pub fn terminal_keys(&self) -> TerminalKeys {
        TerminalKeys(
            self.terminals()
                .iter()
                .map(|terminal| {
                    (
                        terminal.model().to_owned(),
                        terminal
                            .occurrence()
                            .map(|occurrence| occurrence.node().clone()),
                    )
                })
                .collect(),
        )
    }

    /// FR-411: every `temporal`/`terminal` node of this graph, ascending by
    /// key. The node, not the S3 value, says whether a model declares a
    /// terminal and in which mode.
    pub fn terminal_nodes(&self) -> Vec<TerminalNode> {
        self.semantic_graph()
            .nodes()
            .filter(|node| is_terminal_node(node))
            .filter_map(|node| TerminalNode::of(node.key(), node.body()))
            .collect()
    }
}
