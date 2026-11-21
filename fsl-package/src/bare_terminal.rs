// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-411, FR-940: the package form of a model `terminal` declaration, read
//! from the admitted package's terminal rows (FR-060-AC-6) and nodes, never
//! from source.

use filament_exact::{EffectiveId, NodeKey};

use crate::bare::BarePackage;
use crate::bare_slot::BareSelectionFailure;
use crate::bare_term::{declaration_index, fault, walk_failure};
use crate::checked_v2::wire_graph;
use crate::reader_protocol_projection::{BareWalker, ReaderProtocolProjection, UnsupportedView};

/// A `terminal when` condition read for evaluation: its projection and the
/// object type `self` ranges over.
pub struct BareTerminalPredicate {
    context: EffectiveId,
    parameter: bool,
    projection: ReaderProtocolProjection,
}

impl BareTerminalPredicate {
    /// The object type the predicate's `self` ranges over.
    pub fn context(&self) -> &EffectiveId {
        &self.context
    }

    /// Whether the condition reads `self`, which is then bound at slot 0.
    pub fn reads_self(&self) -> bool {
        self.parameter
    }

    /// The condition's evaluator nodes.
    pub fn projection(&self) -> &ReaderProtocolProjection {
        &self.projection
    }
}

/// A package terminal's declaration.
pub enum BareTerminalDeclaration {
    /// Every terminal state is intended.
    Any,
    /// A terminal state is intended when the condition holds at it.
    When(BareTerminalPredicate),
}

/// One `terminal` declaration of a package, read from its node.
pub struct BareTerminal {
    /// The declaration's ordinal among the declarations of its identity, in
    /// terminal node key order.
    pub ordinal: usize,
    /// The declaration's `temporal`/`terminal` node.
    pub node: NodeKey,
    /// The package identity of the model the declaration is on (FR-940: a
    /// model is named by identity on the package route, never by alias).
    pub model: String,
    /// The declaration.
    pub declaration: BareTerminalDeclaration,
}

impl BarePackage<'_> {
    /// FR-411, FR-940: the package's `terminal` declarations, in the
    /// admitted terminal rows' structural order.
    ///
    /// # Errors
    ///
    /// [`BareSelectionFailure`] when a row's nodes, model type or condition
    /// do not read.
    pub fn terminal_declarations(&self) -> Result<Vec<BareTerminal>, BareSelectionFailure> {
        let package = self.package();
        let rows = package.terminal_rows();
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let graph = wire_graph(package).map_err(|_| fault("bare-package-has-structural-keys"))?;
        let key =
            |id: &filament_contract_model::CheckedNodeId| -> Result<NodeKey, BareSelectionFailure> {
                graph
                    .key(id)
                    .map_err(|_| fault("bare-package-has-structural-keys"))
            };
        let (_supply, identities) = self.witness()?;
        let walker = BareWalker::new(
            package,
            identities,
            self.equality_decisions(),
            self.exact_type_projections(),
            self.dependency_views(),
        )
        .map_err(crate::protocol_selection::SlotSelectionFailure::Fault)?
        .with_populations(self);
        let world = self.world()?;
        let objects = self
            .object_declarations()
            .map_err(|()| fault("terminal-object-declarations"))?;
        let mut terminals = Vec::with_capacity(rows.len());
        for row in rows {
            let model_key = key(&row.model)?;
            let context = objects
                .iter()
                .map(|object| object.key())
                .find(|effective| {
                    let original = effective.original();
                    let declaration = filament_checked::key::DeclarationKey {
                        package: original.package().to_owned(),
                        node: original.node().to_owned(),
                    };
                    fsl_semantics::model::key::object_type_node_key(&declaration)
                        .is_ok_and(|node| node == model_key)
                })
                .ok_or_else(|| fault("terminal-model-type"))?;
            // The selection that owns the type names the model, as the source
            // route's `model.selection().identity` does; a model with no
            // population falls back to the selection that declares the type.
            let model = match self
                .selection_of_object(&context)
                .map_err(crate::protocol_selection::SlotSelectionFailure::Fault)?
            {
                Some(identity) => identity.to_owned(),
                None => {
                    let declaring = context.original().package();
                    self.model_selections()
                        .into_iter()
                        .find(|selection| selection.identity == declaring)
                        .map(|selection| selection.identity)
                        .ok_or_else(|| fault("terminal-model-selection"))?
                }
            };
            let declaration = match &row.declaration {
                filament_contract_model::TerminalDeclaration::Any => BareTerminalDeclaration::Any,
                filament_contract_model::TerminalDeclaration::When {
                    parameter,
                    condition,
                } => {
                    let condition = key(condition)?;
                    let keys: Vec<NodeKey> =
                        parameter.iter().map(&key).collect::<Result<_, _>>()?;
                    let node = self
                        .node_by_key(&key(&row.node)?)?
                        .ok_or_else(|| fault("terminal-node"))?;
                    let index = declaration_index(package, &node.node_id, "terminal")
                        .ok_or_else(|| fault("terminal-declaration-index"))?;
                    let projection = walker
                        .project_term(
                            &condition,
                            &keys,
                            filament_semantic_value::location::ExpressionOwner::TypeDeclaration {
                                name: fsl_semantics::check::terminal_predicate_site(index),
                            },
                        )
                        .map_err(walk_failure)?;
                    if projection.has_unrepresented_types(world.types()) {
                        return Err(BareSelectionFailure::Unsupported(Box::new(
                            UnsupportedView::missing_type(package.name(), condition),
                        )));
                    }
                    BareTerminalDeclaration::When(BareTerminalPredicate {
                        context,
                        parameter: !keys.is_empty(),
                        projection,
                    })
                }
            };
            terminals.push(BareTerminal {
                node: key(&row.node)?,
                ordinal: 0,
                model,
                declaration,
            });
        }
        // FR-124-AC-11: every reader derives the ordinal from the same
        // terminal node key order, independent of package row placement.
        terminals
            .sort_by(|left, right| (&left.model, &left.node).cmp(&(&right.model, &right.node)));
        let mut ordinals = std::collections::BTreeMap::<String, usize>::new();
        for terminal in &mut terminals {
            let ordinal = ordinals.entry(terminal.model.clone()).or_default();
            terminal.ordinal = *ordinal;
            *ordinal += 1;
        }
        Ok(terminals)
    }
}
