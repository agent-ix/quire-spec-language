// SPDX-License-Identifier: AGPL-3.0-or-later
//! Model object-type operations (FR-103, FR-104): each object type's own
//! declared operations, keyed by its effective identity, and their
//! resolution through a [`TypeEnvironment`]'s effective view.
//!
//! This is a layer-3 side table beside the type environment, not part of
//! it: an operation carries the domain package's [`OperationEffect`], a
//! `model` type, so it stays in this crate while the registry
//! (`quire_semantic_value::declaration`) is a layer-SV leaf (ADR-011 §6.1).

use std::collections::BTreeMap;

use quire_exact::{EffectiveId, ValueType};
use quire_semantic_value::declaration::TypeEnvironment;

use crate::model::domain_package::OperationEffect;

/// A declared operation of an object type (FR-103, ADR-012 §15.2
/// `StateModel`): its name, its parameters and result value types (typed by
/// a field's own FR-056 rule, with no `Option` wrapping -- the domain
/// package's own `OperationParameterRecord`/`OperationResult` carry no
/// independent presence flag the way a field's `presence` does), and its
/// producer-declared effect frame, carried unchanged from the domain
/// package: the assembler resolves none of its keys further.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationDeclaration {
    name: String,
    parameters: Vec<(String, ValueType)>,
    result: Option<ValueType>,
    effect: OperationEffect,
}

impl OperationDeclaration {
    /// `name(parameters): result` with `effect`, exactly as the domain
    /// package's own operation member declares them.
    pub fn new(
        name: impl Into<String>,
        parameters: Vec<(String, ValueType)>,
        result: Option<ValueType>,
        effect: OperationEffect,
    ) -> Self {
        Self {
            name: name.into(),
            parameters,
            result,
            effect,
        }
    }

    /// The declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The declared parameters, in declaration order.
    pub fn parameters(&self) -> &[(String, ValueType)] {
        &self.parameters
    }

    /// The declared result type, or `None` when the operation has no
    /// result.
    pub fn result(&self) -> Option<&ValueType> {
        self.result.as_ref()
    }

    /// The declared effect frame.
    pub fn effect(&self) -> &OperationEffect {
        &self.effect
    }
}

/// Where an operation name resolves in an object type's effective view
/// ([`OperationTable::resolve`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationLookup<'a> {
    /// The operation, and the object type that declares it.
    Declared {
        /// The declaring object type.
        declaring: EffectiveId,
        /// The operation.
        operation: &'a OperationDeclaration,
    },
    /// No type of the view declares an operation of that name.
    Missing,
    /// Several declarations, none of whose declaring types is more derived
    /// than every other: the declaring types, in key order.
    Ambiguous(Vec<EffectiveId>),
}

/// Every object type's own declared operations, in declaration order, keyed
/// by the declaring type's effective identity. Visible on a subtype through
/// FR-081's effective view, as a field is; the declaring type stays its
/// owning type.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationTable(BTreeMap<EffectiveId, Vec<OperationDeclaration>>);

impl OperationTable {
    /// Declares `operations` as the object type `owner`'s own operations,
    /// replacing any it declared before.
    pub fn declare(&mut self, owner: EffectiveId, operations: Vec<OperationDeclaration>) {
        self.0.insert(owner, operations);
    }

    /// The object type `owner`'s own declared operations, in declaration
    /// order; empty when it declares none.
    pub fn declared_by(&self, owner: EffectiveId) -> &[OperationDeclaration] {
        self.0.get(&owner).map_or(&[], Vec::as_slice)
    }

    /// The operation `name` resolves to in the object type's effective view
    /// under `types` (FR-103, FR-104): declared by the type itself or by an
    /// ancestor, visible on a subtype as a field is, and keeping its
    /// declaring type. A declaration an ancestor makes is hidden by one a
    /// more derived type of the view makes; two declarations neither of
    /// whose types is more derived are ambiguous. Only an object type
    /// `types` admits declares an operation.
    pub fn resolve(
        &self,
        types: &TypeEnvironment,
        object_type: EffectiveId,
        name: &str,
    ) -> OperationLookup<'_> {
        let candidates: Vec<(EffectiveId, &OperationDeclaration)> = self
            .0
            .iter()
            .filter(|(declaring, _)| {
                types.object_type(**declaring).is_some() && types.conforms(object_type, **declaring)
            })
            .flat_map(|(declaring, operations)| {
                operations
                    .iter()
                    .filter(|operation| operation.name() == name)
                    .map(move |operation| (*declaring, operation))
            })
            .collect();
        let nearest: Vec<(EffectiveId, &OperationDeclaration)> = candidates
            .iter()
            .filter(|(declaring, _)| {
                !candidates
                    .iter()
                    .any(|(other, _)| other != declaring && types.conforms(*other, *declaring))
            })
            .copied()
            .collect();
        match nearest.as_slice() {
            [] => OperationLookup::Missing,
            [(declaring, operation)] => OperationLookup::Declared {
                declaring: *declaring,
                operation,
            },
            many => {
                OperationLookup::Ambiguous(many.iter().map(|(declaring, _)| *declaring).collect())
            }
        }
    }
}
