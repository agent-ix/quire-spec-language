// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-152 qualified-name binder for the systems-model kinds: the binder
//! request `resolve(kind, M::<artifact id>)`.
//!
//! A qualified name is the model alias followed by the construct's artifact
//! id. It is resolved against the one admitted package the alias selects and
//! returns the construct's declaration key together with its effective
//! declaration in the supplied [`EffectiveView`]: an Interface's effective
//! type, or the effective member owned by the construct's source end-owner
//! type (a Part's owning type, a Port's part's owning type, a Connection's or
//! Allocation's source end-owner type). A Connection or Allocation resolves
//! only when its admission rule holds, as admission applies it. A construct is never named through
//! its owner, an alias never reaches another selection's declarations, and
//! nothing outside the supplied inputs is consulted.
//!
//! `systems.resolve` is charged once per segment after the alias, before the
//! segment is resolved. Runtime navigation (`r.name`, `model.navigate`) is
//! not here: it needs an object population this crate does not build.
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline"
)]

use std::collections::HashMap;

use crate::model::accounting::{Charge, ChargePoint, Incomplete, Meter};
use crate::model::admitted::AdmittedPackage;
use crate::model::domain_package::{DomainPackageRecord, DomainPackageRef};
use crate::model::key::{DeclarationKey, EffectiveId};
use crate::model::normalize::{EffectiveView, ModelRefusal, ModelRefusalCause};
use crate::model::systems::{
    check_allocation, check_connection, resolve_kind, wrong_export, AllocationCheckOutcome,
    ConditionFailure, ConnectionCheckOutcome, ConnectionOutcome, Kind, ResolvedElement,
};
use qsl_foundation::diagnostic::Code;

/// A model alias of the unit being checked and the selection it names.
#[derive(Clone, Copy, Debug)]
pub struct ModelAlias<'a> {
    /// The `M` of `M::pump_out`.
    pub alias: &'a str,
    /// The selection the alias names.
    pub selection: &'a DomainPackageRef,
}

/// A resolved systems-model element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedSystemsElement {
    /// The element's kind, always the requested kind.
    pub kind: Kind,
    /// The construct's declaration key.
    pub key: DeclarationKey,
    /// The effective declaration the name resolves to.
    pub effective: EffectiveId,
    /// The owning effective type; absent for an Interface, which is itself
    /// an effective type.
    pub owner: Option<EffectiveId>,
}

/// Why a qualified name did not resolve.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolveFailure {
    /// The name is refused.
    Refused(Box<ModelRefusal>),
    /// The connection conditions refused the Connection; every failing
    /// condition, in table order.
    Connection(Vec<ConditionFailure>),
    /// A charge was denied.
    Incomplete(Incomplete),
}

fn unresolved(name: &str, segment: usize, reason: &str) -> ResolveFailure {
    ResolveFailure::Refused(Box::new(ModelRefusal {
        code: Code::MissingDeclaration,
        cause: ModelRefusalCause::UnresolvedName {
            name: name.to_owned(),
            segment,
        },
        detail: format!("{name}: {reason}"),
    }))
}

/// Resolves the qualified `name` (`M::<artifact id>`) as a `required`
/// systems-model element of `admitted`, whose effective declarations are
/// `view`'s.
///
/// Refuses `foreign_reference` when the alias names another selection than
/// `admitted`'s, `missing_declaration` when the alias is unbound, a segment
/// names nothing (including any segment past the artifact id) or the view
/// holds no effective declaration for the construct, and
/// `invalid_model_binding`/`wrong-export` when the construct is another kind.
pub fn resolve_systems(
    admitted: &AdmittedPackage,
    view: &EffectiveView,
    aliases: &[ModelAlias<'_>],
    required: Kind,
    name: &str,
    meter: &mut Meter,
) -> Result<ResolvedSystemsElement, ResolveFailure> {
    let mut segments = name.split("::");
    let alias = segments.next().unwrap_or_default();
    let Some(bound) = aliases.iter().find(|bound| bound.alias == alias) else {
        return Err(unresolved(name, 0, "the model alias is not bound"));
    };
    if bound.selection != admitted.selection() {
        return Err(ResolveFailure::Refused(Box::new(ModelRefusal {
            code: Code::ForeignReference,
            cause: ModelRefusalCause::ForeignAlias {
                alias: alias.to_owned(),
                offered: Box::new(bound.selection.clone()),
                expected: Box::new(admitted.selection().clone()),
            },
            detail: format!(
                "{name}: alias {alias} names model {}, not the selection that owns the declarations",
                bound.selection.identity
            ),
        })));
    }
    let mut construct = None;
    let mut position = 0;
    for segment in segments {
        position += 1;
        meter
            .charge(Charge::new(ChargePoint::SystemsResolve))
            .map_err(ResolveFailure::Incomplete)?;
        if construct.is_some() {
            return Err(unresolved(
                name,
                position,
                "a construct is named by its artifact id, never through an owner",
            ));
        }
        let Some(declaration) = admitted.declaration(segment) else {
            return Err(unresolved(
                name,
                position,
                "no declaration has this artifact id",
            ));
        };
        construct = Some(declaration);
    }
    let Some(construct) = construct else {
        return Err(unresolved(name, 1, "the name has no artifact id"));
    };
    let resolved = resolve_kind(admitted.systems(), required, construct.key)
        .map_err(|refusal| ResolveFailure::Refused(Box::new(refusal)))?;
    admit_construct(admitted, view, &resolved, meter)?;
    let owner = effective_owner(admitted, view, resolved.kind, &resolved.key, name)?;
    let effective = view
        .declarations()
        .iter()
        .find(|entry| {
            entry.visible
                && entry.preimage.original == resolved.key
                && entry.preimage.owner_effective_type == owner
        })
        .map(|entry| entry.effective_id)
        .ok_or_else(|| unresolved(name, 1, "the view holds no effective declaration for it"))?;
    Ok(ResolvedSystemsElement {
        kind: resolved.kind,
        key: resolved.key,
        effective,
        owner,
    })
}

/// Applies the admission rule of the construct's kind, as admission does:
/// the connection conditions for a Connection, the target rule for an
/// Allocation. A construct admission would refuse does not resolve.
fn admit_construct(
    admitted: &AdmittedPackage,
    view: &EffectiveView,
    resolved: &ResolvedElement,
    meter: &mut Meter,
) -> Result<(), ResolveFailure> {
    match resolved.kind {
        Kind::Connection => {
            match check_connection(view.model_index(), admitted.systems(), &resolved.key, meter) {
                ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted) => Ok(()),
                ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
                    Err(ResolveFailure::Connection(failures))
                }
                ConnectionCheckOutcome::Refused(refusal) => {
                    Err(ResolveFailure::Refused(Box::new(refusal)))
                }
                ConnectionCheckOutcome::Incomplete(incomplete) => {
                    Err(ResolveFailure::Incomplete(incomplete))
                }
            }
        }
        Kind::Allocation => match check_allocation(admitted.systems(), &resolved.key, meter) {
            AllocationCheckOutcome::Admitted => Ok(()),
            AllocationCheckOutcome::Refused(refusal) => {
                Err(ResolveFailure::Refused(Box::new(refusal)))
            }
            AllocationCheckOutcome::Incomplete(incomplete) => {
                Err(ResolveFailure::Incomplete(incomplete))
            }
        },
        Kind::Part | Kind::Port | Kind::Interface | Kind::None => Ok(()),
    }
}

/// The owner of a Port or operation source: a Port belongs to its Part's
/// owning type, an operation to its declaring type.
fn element_owner<'a>(
    key: &DeclarationKey,
    records: &HashMap<&DeclarationKey, &'a DomainPackageRecord>,
) -> Option<&'a DeclarationKey> {
    match records.get(key)? {
        DomainPackageRecord::Component(part) => Some(&part.owning_type),
        DomainPackageRecord::Endpoint(port) => match records.get(&port.owning_component)? {
            DomainPackageRecord::Component(part) => Some(&part.owning_type),
            _ => None,
        },
        DomainPackageRecord::OperationMember(operation) => Some(&operation.owner),
        _ => None,
    }
}

/// The effective type that owns `key`'s effective declaration: none for an
/// Interface, else the effective identity of the construct's source
/// end-owner type. The source end's owner is computed on its own: a source
/// that has none (a Port whose owner is not a Part, or an element that is
/// not a Part, Port or operation) is `wrong-export`, never replaced by the
/// target end's owner.
fn effective_owner(
    admitted: &AdmittedPackage,
    view: &EffectiveView,
    kind: Kind,
    key: &DeclarationKey,
    name: &str,
) -> Result<Option<EffectiveId>, ResolveFailure> {
    let refuse = |refusal| ResolveFailure::Refused(Box::new(refusal));
    if kind == Kind::Interface {
        return Ok(None);
    }
    let records: HashMap<&DeclarationKey, &DomainPackageRecord> = admitted
        .package()
        .records
        .iter()
        .map(|record| (record.key(), record))
        .collect();
    let missing = || {
        refuse(ModelRefusal {
            code: Code::MissingDeclaration,
            cause: ModelRefusalCause::UnresolvedName {
                name: name.to_owned(),
                segment: 1,
            },
            detail: format!(
                "{}: the view holds no effective declaration for it",
                key.node
            ),
        })
    };
    let record = records.get(key).ok_or_else(missing)?;
    let source = match record {
        DomainPackageRecord::Component(part) => Some(&part.owning_type),
        DomainPackageRecord::Endpoint(_) => element_owner(key, &records),
        DomainPackageRecord::Relationship(connection) => {
            element_owner(&connection.source.type_identity, &records)
        }
        DomainPackageRecord::Allocation(allocation) => {
            element_owner(&allocation.source_element, &records)
        }
        _ => None,
    };
    let Some(source) = source else {
        let end = match record {
            DomainPackageRecord::Relationship(connection) => &connection.source.type_identity,
            DomainPackageRecord::Allocation(allocation) => &allocation.source_element,
            _ => key,
        };
        return Err(refuse(wrong_export(
            Kind::Part,
            admitted.systems().actual_kind(end),
            &end.node,
        )));
    };
    view.type_identities()
        .get(source)
        .copied()
        .map(Some)
        .ok_or_else(missing)
}
