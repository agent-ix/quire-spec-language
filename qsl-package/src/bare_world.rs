// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533 "What the bare view reads from the package": the declarations a
//! protocol-slot evaluation runs against, read from the package's own nodes.
//!
//! The source route evaluates against the checked scope of the unit it
//! compiled: its record declarations, enumeration members and IEEE profile.
//! A [`BareWorld`] holds the same declarations as the package's bytes and the
//! identity rows give them, and nothing else: a record is the complete
//! selected record view Contract IR projects (`SelectedRecordView`), the IEEE
//! profile is the lock's selection of the profile law. Source unions retain
//! the original producer's sealed members and positional payload types.

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_model::{
    CheckedNodeId, CheckedNodeKind, CompositeTypeForm, ModelForm, SelectedExactIdentity,
};
use quire_exact::ValueType;
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment, UnionMemberDeclaration,
};
use quire_semantic_value::enumeration::EnumMemberIndex;

use crate::bare::BarePackage;
use crate::bare_slot::BareSelectionFailure;

/// The declarations a bytes-only protocol-slot evaluation runs against.
#[derive(Clone, Debug)]
pub struct BareWorld {
    types: TypeEnvironment,
    enum_members: EnumMemberIndex,
    ieee_profile: bool,
}

impl BareWorld {
    /// The package's record and source union declarations.
    pub fn types(&self) -> &TypeEnvironment {
        &self.types
    }

    /// The enumeration members the package's equality decisions carry; the
    /// index of the declared enumerations the source route holds is not a
    /// property of the bytes (FR-533 "Types the bytes cannot project").
    pub fn enum_members(&self) -> &EnumMemberIndex {
        &self.enum_members
    }

    /// Whether the package's lock selects the IEEE profile law.
    pub fn has_ieee_profile(&self) -> bool {
        self.ieee_profile
    }
}

fn fault(invariant: &'static str) -> BareSelectionFailure {
    BareSelectionFailure::from(crate::protocol_selection::SlotSelectionFailure::Fault(
        InternalFault::new("FR-533", invariant),
    ))
}

/// Keep the candidates whose native dependency closure can be represented.
/// Native references carry the topology; no Exact type is reconstructed from
/// a name or walked by a second type interpreter.
fn closed_composites(
    package: &BarePackage<'_>,
    candidates: Vec<(&CheckedNodeId, CompositeDeclaration)>,
) -> Vec<CompositeDeclaration> {
    let represented_nodes: BTreeSet<_> = candidates.iter().map(|(node, _)| *node).collect();
    let represented_handles: BTreeSet<_> = candidates
        .iter()
        .map(|(_, declaration)| declaration.key())
        .collect();
    let mut reverse: BTreeMap<&CheckedNodeId, Vec<&CheckedNodeId>> = BTreeMap::new();
    let mut blocked = BTreeSet::new();
    let mut pending = Vec::new();
    for node in &package.package().graph().nodes {
        let id = &node.node_id;
        let requires_declaration = matches!(
            package.package().node_kind(id),
            Some(CheckedNodeKind::CompositeType(
                CompositeTypeForm::Record | CompositeTypeForm::Tuple | CompositeTypeForm::Union
            )) | Some(CheckedNodeKind::Model(
                ModelForm::RecordValueType | ModelForm::TupleValueType | ModelForm::ObjectType
            ))
        );
        let missing_handle = package.exact_type_projections().get(id)
            .and_then(|projection| projection.as_ref().ok())
            .is_some_and(|projection| matches!(projection.value_type(), ValueType::Composite(key) if !represented_handles.contains(key)));
        if (requires_declaration && !represented_nodes.contains(id)) || missing_handle {
            blocked.insert(id);
            pending.push(id);
        }
        reverse.entry(&node.semantic_type).or_default().push(id);
        if let Some(term) = package.package().term(id) {
            for dependency in term.reference_targets() {
                reverse.entry(dependency).or_default().push(id);
            }
        }
    }
    while let Some(missing) = pending.pop() {
        for parent in reverse.get(missing).into_iter().flatten() {
            if blocked.insert(*parent) {
                pending.push(*parent);
            }
        }
    }
    candidates
        .into_iter()
        .filter(|(node, _)| !blocked.contains(node))
        .map(|(_, declaration)| declaration)
        .collect()
}

impl BarePackage<'_> {
    /// The declarations this package's slots evaluate against, read from its
    /// nodes and identity rows (FR-533). A composite Contract IR cannot project
    /// is absent: a slot that reads it refuses `MissingTypeRepresentation`
    /// when it is selected, and no record of a different shape is
    /// substituted.
    ///
    /// # Errors
    ///
    /// [`BareSelectionFailure::Identity`] for a missing, foreign or unknown
    /// identity witness; a fault when the projected composites are not a closed
    /// environment.
    #[ix_trace_rs::string_edge]
    pub fn world(&self) -> Result<BareWorld, BareSelectionFailure> {
        let package = self.package();
        let (_, identities) = self.witness()?;
        let mut composites = Vec::new();
        for row in identities.declaration_rows() {
            if !matches!(row.exact_identity, SelectedExactIdentity::Record(_)) {
                continue;
            }
            let Some(Ok(projection)) = self.exact_type_projections().get(&row.structural_node)
            else {
                continue;
            };
            let Some(view) = projection.record_view() else {
                continue;
            };
            let mut ordered_fields: Vec<_> = view.fields().iter().collect();
            // The MODEL binding list is authored order; runtime record
            // slots use the original declaration-key order on both routes.
            ordered_fields.sort_by(|left, right| left.ir_identity().cmp(right.ir_identity()));
            let fields = ordered_fields
                .into_iter()
                .map(|field| {
                    FieldDeclaration::new(
                        field.name(),
                        field.value_type().clone(),
                        field.presence(),
                    )
                })
                .collect();
            composites.push((
                &row.structural_node,
                CompositeDeclaration::new(
                    view.record_key().clone(),
                    row.original_identity.as_ref(),
                    CompositeShape::Record(fields),
                ),
            ));
        }
        for union in identities.source_union_rows() {
            if !matches!(
                self.exact_type_projections().get(&union.structural_node),
                Some(Ok(_))
            ) {
                continue;
            }
            let members = union
                .members
                .iter()
                .map(|member| {
                    UnionMemberDeclaration::from_verified(
                        member.member.clone(),
                        member.payload.clone(),
                    )
                })
                .collect();
            composites.push((
                &union.structural_node,
                CompositeDeclaration::new(
                    union.type_handle.clone(),
                    union.qualified_declaration.join("::"),
                    CompositeShape::Union(members),
                ),
            ));
        }
        let types = TypeEnvironment::new(closed_composites(self, composites), Vec::new())
            .map_err(|_| fault("bare-composites-are-a-closed-environment"))?;
        let ieee = qsl_semantics::value::DefinitionRole::IeeeProfile.identity();
        Ok(BareWorld {
            types,
            enum_members: EnumMemberIndex::default(),
            ieee_profile: package
                .lock()
                .definition_selections
                .iter()
                .any(|selected| selected.identity.as_ref() == ieee),
        })
    }
}
