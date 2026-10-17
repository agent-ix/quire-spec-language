// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S3 call-target seam (FR-317, ADR-012 §16.2): the one place an
//! `Expression::Call` or `Expression::Name` is resolved to its target.
//!
//! A qualified reference `q::m` has as candidates the declared unions,
//! declared enums and import aliases named `q` that have a member `m`. This
//! module counts them and refuses more than one; with exactly one union
//! candidate the target is a [`UnionMember`], whose check the `SumCase`
//! family owns; every other reference keeps `Value`'s existing resolution,
//! which the caller supplies.

use quire_exact::{IllTypedCause, NodeKey, ValueType};
use quire_semantic_value::declaration::CompositeShape;
use quire_semantic_value::location::Location;

use super::check::Typer;
use super::imports::ImportedNames;
use super::refusal::{AmbiguousQualified, CheckCause, CheckRefusal, QualifiedCandidate};
use super::sum_case::{UnionMember, UnionRef};

/// What a call or name resolved to.
pub(crate) enum ResolvedTarget<'a, V> {
    /// A member of a declared union, checked by the `SumCase` family.
    UnionMember(UnionMember<'a>),
    /// Any other target, as `Value` resolved it.
    Value(V),
}

/// The declared unions named `qualifier` that have a member `member`, as
/// (declaration, position) pairs, in declaration order.
fn unions<'a>(typer: &Typer<'a>, qualifier: &str, member: &str) -> Vec<(NodeKey, usize)> {
    let scope = typer.scope();
    scope
        .named_types(qualifier)
        .iter()
        .filter_map(|named| match named {
            ValueType::Composite(declaration) => {
                let union = UnionRef::declared(scope.types(), *declaration)?;
                Some((*declaration, union.position_of(member)?))
            }
            _ => None,
        })
        .collect()
}

/// Whether a declared union is named `qualifier`.
fn names_a_union(typer: &Typer<'_>, qualifier: &str) -> bool {
    let scope = typer.scope();
    scope.named_types(qualifier).iter().any(|named| {
        matches!(named, ValueType::Composite(declaration)
        if matches!(
            scope.types().composite(*declaration).map(|declared| declared.shape()),
            Some(CompositeShape::Union(_))
        ))
    })
}

/// Resolve `name` at `location` to exactly one target or refuse it.
///
/// `existing` is `Value`'s own resolution of `name`, run when the target is
/// not a union member; it makes the same refusals it made before unions
/// existed. Every unqualified name takes that path unchanged.
pub(crate) fn resolve_target<'a, V>(
    typer: &Typer<'a>,
    name: &str,
    location: &Location,
    existing: impl FnOnce() -> Result<V, CheckRefusal>,
) -> Result<ResolvedTarget<'a, V>, CheckRefusal> {
    let Some((qualifier, member)) = name.rsplit_once("::") else {
        return existing().map(ResolvedTarget::Value);
    };
    let scope = typer.scope();
    let declared = unions(typer, qualifier, member);
    let enums: Vec<NodeKey> = scope
        .enum_members_named(name)
        .map(|(binding, _)| binding.declaration.key())
        .collect();
    let import = name.split_once("::").and_then(|(alias, imported)| {
        let import = scope.imported(alias)?;
        ImportedNames::of(&import.view)
            .resolve(imported)
            .ok()
            .map(|_| alias.to_owned())
    });
    let candidates = declared.len() + enums.len() + usize::from(import.is_some());
    // A name only enums share is refused as it always was; every other
    // overlap is the FR-317 ambiguity.
    if candidates > 1 && !(declared.is_empty() && import.is_none()) {
        let candidates = declared
            .iter()
            .map(|(declaration, _)| QualifiedCandidate::Union(*declaration))
            .chain(enums.into_iter().map(QualifiedCandidate::Enum))
            .chain(import.map(QualifiedCandidate::Import))
            .collect();
        return Err(CheckRefusal {
            location: location.clone(),
            cause: CheckCause::AmbiguousQualified(Box::new(AmbiguousQualified {
                name: name.to_owned(),
                candidates,
            })),
        });
    }
    if let ([(declaration, position)], true) = (&declared[..], enums.is_empty() && import.is_none())
    {
        let member = UnionRef::declared(scope.types(), *declaration)
            .and_then(|union| UnionMember::of(union, *position));
        if let Some(member) = member {
            return Ok(ResolvedTarget::UnionMember(member));
        }
    }
    // `U::m` where `U` is a union that declares no `m` is an undeclared
    // member, not an unresolved name (QSpec FR-143).
    if candidates == 0 && names_a_union(typer, qualifier) {
        return Err(CheckRefusal::ill_typed(
            location,
            IllTypedCause::TypeMismatch,
        ));
    }
    existing().map(ResolvedTarget::Value)
}
