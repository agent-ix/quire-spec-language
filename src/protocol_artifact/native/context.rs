// SPDX-License-Identifier: AGPL-3.0-only
//! FR-036/040/042: borrowed declaration inputs already admitted by the compiler.
use super::layout::DeclLayout;
use crate::checking::composed::DeclarationTypes;
use crate::linking::composed::{
    definitions::ProfileUse, models::ModelDeclarationReport, scopes::DeclarationScope,
    DependencyReference,
};
use crate::syntax::composed as c;

pub(super) struct Declaration<'s, 'm> {
    pub typed: &'s DeclarationTypes<'m>,
    pub unit: &'s c::ComposedUnit,
    pub syntax: &'s c::Declaration,
    pub scope: &'s DeclarationScope,
    pub exports: &'s ModelDeclarationReport<'m>,
    pub references: &'s [DependencyReference],
    pub profile_uses: &'s [ProfileUse],
    pub profiles: &'s [u32],
    pub layout: &'s DeclLayout,
}
