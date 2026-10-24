// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-218 "Relationships": the S3 check of a protocol's `relationship r =
//! M::R;` members.
//!
//! `M::R` resolves in the unit's own declaration scope: `M` is the alias of
//! a selected model and `R` the artifact id of one of its exports. A name
//! with no such export refuses `missing_declaration`/`missing-name`; an
//! export that is not a relationship refuses `invalid_model_binding`/
//! `wrong-export`, retaining the required kind, the actual kind and the
//! export's declaration key. A checked relationship is bound to its
//! export's declaration key and the types of its two ends in the order the
//! model declares them (source, then target), never to the written name.
//! A relationship whose ends are not both object types (a systems
//! connection between endpoints) has no end a value can fill, which no
//! requirement types yet: it is refused as not yet implemented. Two
//! relationships of one name refuse `ambiguous_declaration`/
//! `ambiguous-name` at each declaration; relationships and channels have
//! separate namespaces.

use std::collections::BTreeMap;

use qsl_forms::{ProtocolConstructKind, ProtocolDeclarationForm};
use qsl_foundation::Span;
use quire_exact::ValueType;
use quire_semantic_value::location::{Location, Origin};

use super::lowering::AdmittedModel;
use super::refusal::{CheckCause, CheckRefusal, ProtocolAnchorCause};
use crate::model::domain_package::DomainPackageRecord;
use crate::model::key::DeclarationKey;

/// One checked `relationship r = M::R;` member (FR-218 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedRelationship {
    /// The protocol-local name `r`.
    pub name: String,
    /// The declaration key of the model relationship `M::R` names.
    pub key: DeclarationKey,
    /// The types of its source and target ends, in that order.
    pub ends: [ValueType; 2],
    /// The span of the whole member.
    pub span: Span,
}

fn refusal(cause: ProtocolAnchorCause) -> CheckRefusal {
    CheckRefusal {
        // `region::DeclarationRegions::refusal_region` reads the cause's own
        // span.
        location: Location::root(Origin::Expression),
        cause: CheckCause::ProtocolAnchor(Box::new(cause)),
    }
}

/// The keyword phrase of an export's kind, as a wrong-export refusal names
/// it.
fn kind_label(record: &DomainPackageRecord) -> &'static str {
    match record {
        DomainPackageRecord::Enum(_) => "variant type",
        DomainPackageRecord::Variant(_) => "variant",
        DomainPackageRecord::ObjectType(_) => "object type",
        DomainPackageRecord::FieldMember(_) => "field",
        DomainPackageRecord::RecordValueType(_) => "record value type",
        DomainPackageRecord::EventType(_) => "event type",
        DomainPackageRecord::StateMachine(_) => "state machine",
        DomainPackageRecord::Process(_) => "process",
        DomainPackageRecord::Clause(_) => "clause",
        DomainPackageRecord::Namespace(_) => "namespace",
        DomainPackageRecord::PersistenceInterface(_) => "persistence interface",
        DomainPackageRecord::ScalarType(_) => "scalar type",
        DomainPackageRecord::OperationMember(_) => "operation",
        DomainPackageRecord::Component(_) => "part",
        DomainPackageRecord::Endpoint(_) => "port",
        DomainPackageRecord::Relationship(_) => "relationship",
        DomainPackageRecord::Allocation(_) => "allocation",
        DomainPackageRecord::Population(_) => "population",
    }
}

/// Every relationship member of `protocol`, checked against the selected
/// `models`, in source order, or the refusals with their spans.
pub(crate) fn check(
    protocol: &ProtocolDeclarationForm,
    models: &[AdmittedModel],
) -> Result<Vec<CheckedRelationship>, Vec<(Span, CheckRefusal)>> {
    let mut refusals = Vec::new();
    let mut by_name: BTreeMap<&str, Vec<Span>> = BTreeMap::new();
    for relationship in &protocol.relationships {
        by_name
            .entry(relationship.name.name.as_str())
            .or_default()
            .push(relationship.name.span);
    }
    for (name, loci) in &by_name {
        if loci.len() < 2 {
            continue;
        }
        for &span in loci {
            refusals.push((
                span,
                refusal(ProtocolAnchorCause::Ambiguous {
                    name: (*name).to_owned(),
                    loci: loci.clone(),
                    span,
                }),
            ));
        }
    }

    let mut checked = Vec::with_capacity(protocol.relationships.len());
    for relationship in &protocol.relationships {
        let target = &relationship.target;
        let export = target.name.split_once("::").and_then(|(alias, artifact)| {
            let model = models.iter().find(|model| model.alias() == Some(alias))?;
            Some((model, model.export(artifact)?))
        });
        let Some((model, export)) = export else {
            refusals.push((
                target.span,
                refusal(ProtocolAnchorCause::MissingRelationship {
                    relationship: relationship.name.name.clone(),
                    name: target.name.clone(),
                    span: target.span,
                }),
            ));
            continue;
        };
        let DomainPackageRecord::Relationship(record) = export else {
            refusals.push((
                target.span,
                refusal(ProtocolAnchorCause::RelationshipExport {
                    relationship: relationship.name.name.clone(),
                    name: target.name.clone(),
                    required: "relationship",
                    actual: kind_label(export),
                    export: Box::new(export.key().clone()),
                    span: target.span,
                }),
            ));
            continue;
        };
        let end = |key: &DeclarationKey| model.object_type_id(key).map(ValueType::Reference);
        let (Some(source), Some(target_end)) = (
            end(&record.source.type_identity),
            end(&record.target.type_identity),
        ) else {
            refusals.push((
                target.span,
                refusal(ProtocolAnchorCause::Unimplemented {
                    name: protocol.name.name.clone(),
                    construct: ProtocolConstructKind::Relationship.label(),
                    span: target.span,
                }),
            ));
            continue;
        };
        checked.push(CheckedRelationship {
            name: relationship.name.name.clone(),
            key: record.key.clone(),
            ends: [source, target_end],
            span: relationship.span,
        });
    }
    if refusals.is_empty() {
        Ok(checked)
    } else {
        Err(refusals)
    }
}

#[cfg(test)]
mod tests;
