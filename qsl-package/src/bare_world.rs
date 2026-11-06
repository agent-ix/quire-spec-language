// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533 "What the bare view reads from the package": the declarations a
//! protocol-slot evaluation runs against, read from the package's own nodes.
//!
//! The source route evaluates against the checked scope of the unit it
//! compiled: its record declarations, enumeration members and IEEE profile.
//! A [`BareWorld`] holds the same declarations as the package's bytes and the
//! identity rows give them, and nothing else: a record is the complete
//! selected record view Contract IR projects (`SelectedRecordView`), the IEEE
//! profile is the lock's selection of the profile law.

use quire_contract_model::SelectedExactIdentity;
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
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
    /// The package's record declarations.
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

impl BarePackage<'_> {
    /// The declarations this package's slots evaluate against, read from its
    /// nodes and identity rows (FR-533). A record Contract IR cannot project
    /// is absent: a slot that reads it refuses `MissingTypeRepresentation`
    /// when it is selected, and no record of a different shape is
    /// substituted.
    ///
    /// # Errors
    ///
    /// [`BareSelectionFailure::Identity`] for a missing, foreign or unknown
    /// identity witness; a fault when the projected records are not a closed
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
            let Ok(projection) = package.project_exact_type(
                &row.structural_node,
                identities,
                crate::bare_slot::package_work_limit(),
            ) else {
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
            composites.push(CompositeDeclaration::new(
                view.record_key().clone(),
                row.original_identity.as_ref(),
                CompositeShape::Record(fields),
            ));
        }
        let types = TypeEnvironment::new(composites, Vec::new())
            .map_err(|_| fault("bare-records-are-a-closed-environment"))?;
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
