// SPDX-License-Identifier: AGPL-3.0-or-later
//! Model object-type operations (FR-103, FR-104): each object type's own
//! declared operations, keyed by its effective identity, and their
//! resolution through a [`TypeEnvironment`]'s effective view.
//!
//! This is a `model` table beside the type environment, not part of it: an
//! operation carries the domain package's [`OperationEffect`], so it stays
//! in `model` while the registry (`quire_semantic_value::declaration`) is a
//! layer-SV leaf (ADR-011 §6.1).

use std::collections::BTreeMap;

use quire_exact::{EffectiveId, ValueType};
use quire_semantic_value::declaration::TypeEnvironment;

use super::domain_package::OperationEffect;

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
    /// The `Clock` parameters (FR-230 "Operation parameters"): each a
    /// place, not a data value, by its position among every declared
    /// parameter and its name. They are not in `parameters`.
    clock_parameters: Vec<(usize, String)>,
    /// Whether the result is a `Clock` (FR-230 "Results"): a place, so no
    /// data `result`.
    clock_result: bool,
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
            clock_parameters: Vec::new(),
            clock_result: false,
        }
    }

    /// This operation with the `Clock` parameters `clock_parameters`, each
    /// by its position among every declared parameter and its name, and a
    /// `Clock` result when `clock_result`.
    #[must_use]
    pub fn with_clocks(
        mut self,
        clock_parameters: Vec<(usize, String)>,
        clock_result: bool,
    ) -> Self {
        self.clock_parameters = clock_parameters;
        self.clock_result = clock_result;
        self
    }

    /// The `Clock` parameters, each by its position among every declared
    /// parameter and its name, in declaration order.
    pub fn clock_parameters(&self) -> &[(usize, String)] {
        &self.clock_parameters
    }

    /// Whether the result is a `Clock`.
    pub fn clock_result(&self) -> bool {
        self.clock_result
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

    /// Every operation of the object type's effective view under `types`, by
    /// name in ascending UTF-8 byte order: each name the type or one of its
    /// ancestors declares that [`Self::resolve`] resolves to one
    /// declaration. A name whose declarations are ambiguous has no effective
    /// operation.
    pub fn effective(
        &self,
        types: &TypeEnvironment,
        object_type: EffectiveId,
    ) -> Vec<(EffectiveId, &OperationDeclaration)> {
        let names: std::collections::BTreeSet<&str> = self
            .0
            .iter()
            .filter(|(declaring, _)| types.conforms(object_type.clone(), (*declaring).clone()))
            .flat_map(|(_, operations)| operations.iter().map(OperationDeclaration::name))
            .collect();
        names
            .into_iter()
            .filter_map(|name| match self.resolve(types, object_type.clone(), name) {
                OperationLookup::Declared {
                    declaring,
                    operation,
                } => Some((declaring, operation)),
                OperationLookup::Missing | OperationLookup::Ambiguous(_) => None,
            })
            .collect()
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
                types.object_type((*declaring).clone()).is_some() && types.conforms(object_type.clone(), (*declaring).clone())
            })
            .flat_map(|(declaring, operations)| {
                operations
                    .iter()
                    .filter(|operation| operation.name() == name)
                    .map(move |operation| (declaring.clone(), operation))
            })
            .collect();
        let nearest: Vec<(EffectiveId, &OperationDeclaration)> = candidates
            .iter()
            .filter(|(declaring, _)| {
                !candidates
                    .iter()
                    .any(|(other, _)| other != declaring && types.conforms(other.clone(), declaring.clone()))
            })
            .cloned()
            .collect();
        match nearest.as_slice() {
            [] => OperationLookup::Missing,
            [(declaring, operation)] => OperationLookup::Declared {
                declaring: declaring.clone(),
                operation,
            },
            many => {
                OperationLookup::Ambiguous(many.iter().map(|(declaring, _)| declaring.clone()).collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quire_semantic_value::declaration::ObjectTypeDeclaration;

    fn ship() -> OperationDeclaration {
        OperationDeclaration::new("ship", Vec::new(), None, OperationEffect::default())
    }

    /// An operation filed under a key the environment admits as no object
    /// type is never visible: resolving it from that same key is `Missing`,
    /// though `conforms` is reflexive.
    #[test]
    fn an_operation_of_an_unadmitted_type_resolves_missing() {
        let unadmitted = EffectiveId::from_digest([7; 32]);
        let mut table = OperationTable::default();
        table.declare(unadmitted, vec![ship()]);
        let types = TypeEnvironment::new([], []).expect("an empty environment admits");
        assert_eq!(
            table.resolve(&types, unadmitted, "ship"),
            OperationLookup::Missing
        );
    }

    /// The same operation filed under an admitted object type resolves to
    /// it, declared by that type.
    #[test]
    fn an_operation_of_an_admitted_type_resolves_declared() {
        let admitted = EffectiveId::from_digest([8; 32]);
        let mut table = OperationTable::default();
        table.declare(admitted, vec![ship()]);
        let types =
            TypeEnvironment::new([], [ObjectTypeDeclaration::new(admitted, "M::A", vec![])])
                .expect("one object type admits");
        assert_eq!(
            table.resolve(&types, admitted, "ship"),
            OperationLookup::Declared {
                declaring: admitted,
                operation: &ship(),
            }
        );
    }
}
