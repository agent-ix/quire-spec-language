// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-103, FR-302: the operation bodies a unit supplies, retained for
//! dispatch linking.
//!
//! The FR-091 assembler resolves each `body L using p on M::T::op (a, b) { e }`
//! (`qsl_forms::OperationBodyForm`, FR-102) through FR-104's selected-model
//! and effective-operation rules to the resolved operation's
//! [`DeclarationKey`], and keeps the body expression, its parameter names and
//! its source spans under that key. The unit's `pre` and `post` state clauses
//! are keyed by the same resolution
//! ([`PackageDeclarations::unit_clauses_for`]). Nothing joins by the body's
//! local label, a display spelling or declaration order.
//!
//! [`PackageDeclarations::unit_clauses_for`]: super::PackageDeclarations::unit_clauses_for

use qsl_forms::{DeclarationSpans, DeclaredName, Expression};
use qsl_foundation::selection::ProfileSelection;
use quire_exact::EffectiveId;

use super::state_clause::ClauseOperation;
use crate::model::key::DeclarationKey;

/// One `operation-body` the FR-091 assembler admitted: its target resolved
/// to an operation of an admitted domain package, its body as S2 built it.
#[derive(Clone, Debug)]
pub struct OperationBodyDeclaration {
    /// The resolved operation's `DeclarationKey`: the identity the body is
    /// associated with and the only key anything joins it by.
    pub key: DeclarationKey,
    /// The alias of the `model` declaration the target's first segment
    /// names.
    pub model: String,
    /// The body's declared label. Local to the unit; not the operation's
    /// identity.
    pub label: DeclaredName,
    /// The profile selection the body's `using` alias resolved to.
    pub selection: ProfileSelection,
    /// The object type `M::T` the target names, by its effective identity.
    pub context: EffectiveId,
    /// The operation the target resolved to, and the type declaring it.
    pub operation: ClauseOperation,
    /// The receiver-less parameter names, in written order; bound to the
    /// operation's parameters positionally, in signature order.
    pub parameters: Vec<DeclaredName>,
    /// The body.
    pub body: Expression,
    /// The form's spans: the declaration's, and one per body node.
    pub spans: DeclarationSpans,
}
