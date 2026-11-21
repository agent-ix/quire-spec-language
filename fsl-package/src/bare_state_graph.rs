// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940 "Package form of a state-graph claim": a `claim`/`state_graph`
//! node read from its admitted package bytes and its `claim_names` row
//! (FSpec FR-475, FR-322), never from source.

use filament_contract_model::{
    CheckedLiteralValue, CheckedOccurrenceRole, CheckedSemanticNodeV2, LiteralKind,
};
use filament_exact::NodeKey;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::{
    PackageStateGraphForm, PackageStateGraphKind, PackageStatePredicate, PredicateRole,
};

use crate::bare::BarePackage;
use crate::bare_slot::BareSelectionFailure;
use crate::bare_term::{fault, walk_failure, BareClauseRefusal};
use crate::checked_v2::{resolve_state_graph_root, wire_graph, StateGraphKind, StateGraphRoot};
use crate::protocol_selection::SlotSelectionFailure;
use crate::protocol_view::{group_member, text};
use crate::reader_protocol_projection::{BareWalker, ReaderProtocolProjection, UnsupportedView};

/// One state-graph predicate read for evaluation: its parameters in order,
/// the `over` parameter first, each with the local slot the body reads it
/// through, and the predicate's evaluator nodes.
pub struct BareStatePredicate {
    parameters: Vec<(String, usize)>,
    projection: ReaderProtocolProjection,
}

impl BareStatePredicate {
    /// The predicate's parameters in declaration order with their slots.
    pub fn parameters(&self) -> &[(String, usize)] {
        &self.parameters
    }

    /// The predicate's evaluator nodes.
    pub fn projection(&self) -> &ReaderProtocolProjection {
        &self.projection
    }
}

/// What a claim's occurrence names in the package.
struct Parts<'p> {
    root: StateGraphRoot,
    row: &'p filament_contract_model::ClaimNameRow,
    node: &'p CheckedSemanticNodeV2,
}

/// A predicate node's shape.
enum Shape {
    Expression,
    ConstantTrue,
}

impl<'p> BarePackage<'p> {
    fn state_graph_parts(&self, claim: &OccurrenceKey) -> Result<Parts<'p>, BareSelectionFailure> {
        let package = self.package();
        self.witness()?;
        let missing =
            || BareSelectionFailure::from(SlotSelectionFailure::MissingNode(claim.node().clone()));
        let root = resolve_state_graph_root(package, claim)
            .map_err(|failure| BareSelectionFailure::from(SlotSelectionFailure::Fault(failure)))?
            .map_err(|_| missing())?;
        let node = self.node_by_key(claim.node())?.ok_or_else(missing)?;
        let ordinal = claim.origin().ordinal();
        let row = package
            .claim_names()
            .unwrap_or_default()
            .iter()
            .find(|row| {
                row.declaration == node.node_id
                    && row.occurrence.role == CheckedOccurrenceRole::Claim
                    && row.occurrence.ordinal == ordinal
            })
            .ok_or_else(missing)?;
        Ok(Parts { root, row, node })
    }

    pub(crate) fn node_by_key(
        &self,
        key: &NodeKey,
    ) -> Result<Option<&'p CheckedSemanticNodeV2>, BareSelectionFailure> {
        let package = self.package();
        let graph = wire_graph(package).map_err(|_| fault("bare-package-has-structural-keys"))?;
        for node in &package.resolved_graph().nodes {
            let found = graph
                .key(&node.node_id)
                .map_err(|_| fault("bare-package-has-structural-keys"))?;
            if found == *key {
                return Ok(Some(node));
            }
        }
        Ok(None)
    }

    fn predicate_shape(&self, node: &CheckedSemanticNodeV2) -> Option<Shape> {
        if &*node.node_tag == "expression" {
            return Some(Shape::Expression);
        }
        let read = node.body.literal_read()?;
        matches!(
            (read.kind, read.value),
            (LiteralKind::Boolean, CheckedLiteralValue::Boolean(true))
        )
        .then_some(Shape::ConstantTrue)
    }

    /// FR-940: the package form of the state-graph claim declared at the
    /// `claim` occurrence `claim`, read from the package's admitted nodes,
    /// `claim_names` row and type projections: its name, span (the row's
    /// `declaration_region`), occurrence, form, `over` and predicates.
    /// Charges nothing.
    ///
    /// # Errors
    ///
    /// [`BareClauseRefusal::Selection`] for a claim the package does not
    /// select, a body that does not read as FR-475 writes it, or a predicate
    /// whose projection is refused.
    #[ix_trace_rs::string_edge]
    pub fn state_graph_claim_form(
        &self,
        claim: &OccurrenceKey,
    ) -> Result<PackageStateGraphForm, BareClauseRefusal> {
        let parts = self.state_graph_parts(claim)?;
        let malformed = || {
            BareClauseRefusal::from(BareSelectionFailure::from(
                SlotSelectionFailure::MalformedNode(claim.node().clone()),
            ))
        };
        let over = match parts.root.over() {
            None => None,
            Some(key) => {
                let parameter = self.node_by_key(key)?.ok_or_else(malformed)?;
                let name = group_member(&parameter.body, "name")
                    .and_then(text)
                    .ok_or_else(malformed)?;
                let object = self
                    .exact_type_projections()
                    .get(&parameter.node_id)
                    .and_then(|projected| projected.as_ref().ok())
                    .and_then(|projected| match projected.value_type() {
                        filament_exact::ValueType::Reference(object) => Some(object.clone()),
                        _ => None,
                    })
                    .ok_or_else(malformed)?;
                Some((name.to_owned(), object))
            }
        };
        let (kind, wanted) = match parts.root.kind() {
            StateGraphKind::Possible => (PackageStateGraphKind::Possible, false),
            StateGraphKind::AlwaysPossible => (PackageStateGraphKind::AlwaysPossible, true),
            StateGraphKind::UniquePath => (PackageStateGraphKind::UniquePath, true),
        };
        if wanted != parts.root.from().is_some() {
            return Err(malformed());
        }
        let mut predicates = Vec::new();
        let roles = parts
            .root
            .from()
            .map(|from| (PredicateRole::From, from))
            .into_iter()
            .chain([(PredicateRole::Target, parts.root.target())]);
        for (role, key) in roles {
            let node = self.node_by_key(key)?.ok_or_else(malformed)?;
            predicates.push(match self.predicate_shape(node).ok_or_else(malformed)? {
                Shape::ConstantTrue => PackageStatePredicate::ConstantTrue { role },
                Shape::Expression => {
                    let read = self.project_state_graph_predicate(claim, role)?;
                    PackageStatePredicate::Written {
                        role,
                        node: key.clone(),
                        slots: read.projection().frame_slots(),
                    }
                }
            });
        }
        let region = &parts.row.declaration_region;
        Ok(PackageStateGraphForm {
            name: parts.row.name.as_ref().to_owned(),
            span: fsl_foundation::Span {
                start: usize::try_from(region.start).unwrap_or(usize::MAX),
                end: usize::try_from(region.end).unwrap_or(usize::MAX),
            },
            occurrence: claim.clone(),
            kind,
            over,
            predicates,
        })
    }

    /// FR-940: the predicate of `role` of the state-graph claim at `claim`,
    /// its term projected over the claim's `over` parameter at slot 0 and
    /// located under the claim's state-graph site. Charges nothing.
    ///
    /// # Errors
    ///
    /// [`BareSelectionFailure`]: a witness refusal, `MissingNode`, or
    /// `MalformedNode` for a role the claim has no written predicate for,
    /// and the first unsupported projection.
    #[ix_trace_rs::string_edge]
    pub fn project_state_graph_predicate(
        &self,
        claim: &OccurrenceKey,
        role: PredicateRole,
    ) -> Result<BareStatePredicate, BareSelectionFailure> {
        let package = self.package();
        let parts = self.state_graph_parts(claim)?;
        let malformed = || {
            BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(claim.node().clone()))
        };
        let key = match role {
            PredicateRole::From => parts.root.from(),
            PredicateRole::Target => Some(parts.root.target()),
        }
        .ok_or_else(malformed)?;
        let predicate = self.node_by_key(key)?.ok_or_else(malformed)?;
        if !matches!(self.predicate_shape(predicate), Some(Shape::Expression)) {
            return Err(malformed());
        }
        let mut keys = Vec::new();
        let mut names = Vec::new();
        if let Some(over) = parts.root.over() {
            let parameter = self.node_by_key(over)?.ok_or_else(malformed)?;
            names.push(
                group_member(&parameter.body, "name")
                    .and_then(text)
                    .ok_or_else(|| fault("bare-binder-has-a-name"))?
                    .to_owned(),
            );
            keys.push(over.clone());
        }
        let index =
            crate::bare_term::declaration_index(package, &parts.node.node_id, "state_graph")
                .ok_or_else(malformed)?;
        let (_supply, identities) = self.witness()?;
        let walker = BareWalker::new(
            package,
            identities,
            self.equality_decisions(),
            self.exact_type_projections(),
            self.dependency_views(),
        )
        .map_err(SlotSelectionFailure::Fault)?;
        let projection = walker
            .project_term(
                key,
                &keys,
                filament_semantic_value::location::ExpressionOwner::TypeDeclaration {
                    name: fsl_semantics::check::state_graph_predicate_site(index, role),
                },
            )
            .map_err(walk_failure)?;
        let world = self.world()?;
        if projection.has_unrepresented_types(world.types()) {
            return Err(BareSelectionFailure::Unsupported(Box::new(
                UnsupportedView::missing_type(package.name(), key.clone()),
            )));
        }
        let parameters = names
            .into_iter()
            .zip(&keys)
            .map(|(name, key)| {
                projection
                    .local_slot(key)
                    .map(|slot| (name, slot))
                    .ok_or_else(|| fault("bare-clause-binder-has-a-local-slot"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(BareStatePredicate {
            parameters,
            projection,
        })
    }
}
