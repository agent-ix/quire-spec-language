// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-361: S3 refuses each model construct a unit writes unless the unit's
//! header selects `quire.model.complete/v1`.
//!
//! A model construct is an `operation-body` declaration, an `abstraction`
//! declaration, a dispatched `receiver.member(args)` call or a
//! `lookup<T>(p, r) absent m` expression (FR-361 item 5). The selection is
//! the header and nothing else (item 1): the gate reads the unit's header profile identities, never a construct, a
//! `model` declaration or an alias. Every construct is reported, in source
//! order, and a construct in a declaration nothing uses is refused too
//! (items 11, 13).

use std::collections::BTreeSet;

use qsl_forms::{
    AbstractionDeclarationForm, FunctionDeclaration, OperationBodyForm, StateClauseForm,
};
use qsl_foundation::selection::ProfileSelection;

use super::assemble::{AssemblyCause, AssemblyError};
use super::layer::{expression_form, visit_located};
use crate::value::layer::{HeaderLayer, LayerForm};

/// The family of a refused model construct (FR-361 item 5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelFamily {
    /// An `abstraction` declaration (`abstraction-decl`).
    Abstraction,
    /// A `body` operation-body declaration.
    OperationBody,
    /// A postfix `receiver.member(args)` operation call.
    DispatchCall,
    /// A `lookup<T>(p, r) absent m` expression.
    Lookup,
}

impl ModelFamily {
    /// The family of a model construct form, if `form` is one.
    pub(super) fn of(form: LayerForm) -> Option<Self> {
        match form {
            LayerForm::OperationBody => Some(Self::OperationBody),
            LayerForm::DispatchCall => Some(Self::DispatchCall),
            LayerForm::Lookup => Some(Self::Lookup),
            LayerForm::StateClause
            | LayerForm::PredicateDeclaration
            | LayerForm::FunctionDeclaration
            | LayerForm::NamedPredicateCall
            | LayerForm::SequenceQuery
            | LayerForm::ObjectNavigation
            | LayerForm::ValueConstruct => None,
        }
    }

    /// The spelling of the family in FR-361 item 5.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Abstraction => "abstraction",
            Self::OperationBody => "operation-body",
            Self::DispatchCall => "dispatch-call",
            Self::Lookup => "lookup",
        }
    }

    /// The `quire.native.diagnostics/v1` cause tag of the family's refusal.
    pub fn refusal_cause(self) -> &'static str {
        match self {
            Self::Abstraction | Self::OperationBody => "declaration-form",
            Self::DispatchCall | Self::Lookup => "expression-form",
        }
    }
}

/// The kind of declaration that owns a refused construct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnerKind {
    /// An `abstraction` declaration.
    Abstraction,
    /// A `function` or `predicate` declaration.
    Function,
    /// An `invariant`, `pre` or `post` state clause.
    StateClause,
    /// A `body` operation-body declaration.
    OperationBody,
}

/// What a model-construct refusal retains (FR-361 item 11): the recognised
/// form's family, the owning declaration, the header layers the unit selected
/// in source order, and the layer it needed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelGateRefusal {
    /// The family of the refused construct.
    pub family: ModelFamily,
    /// The kind of the owning declaration.
    pub owner_kind: OwnerKind,
    /// The owning declaration's declared name.
    pub owner: String,
    /// The header layers the unit selected, in source order.
    pub selected: Vec<HeaderLayer>,
}

impl ModelGateRefusal {
    /// The layer the unit needed: `quire.model.complete/v1`.
    pub const fn required_layer(&self) -> HeaderLayer {
        HeaderLayer::ModelComplete
    }
}

/// The header layers `profiles` select, each once, in source order.
fn selected_layers(profiles: &[ProfileSelection]) -> Vec<HeaderLayer> {
    let mut layers = Vec::new();
    for layer in profiles
        .iter()
        .filter_map(|profile| HeaderLayer::from_identity(&profile.identity))
    {
        if !layers.contains(&layer) {
            layers.push(layer);
        }
    }
    layers
}

/// Whether the unit's header names the model profile (FR-361 item 1).
pub(super) fn selects_model(profiles: &[ProfileSelection]) -> bool {
    profiles.iter().any(|profile| {
        HeaderLayer::from_identity(&profile.identity) == Some(HeaderLayer::ModelComplete)
    })
}

/// The declarations of a unit the gate reads.
pub(super) struct Declarations<'a> {
    pub(super) abstractions: &'a [AbstractionDeclarationForm],
    pub(super) functions: &'a [FunctionDeclaration],
    pub(super) state_clauses: &'a [StateClauseForm],
    pub(super) operation_bodies: &'a [OperationBodyForm],
}

/// Every model construct `declarations` write, as a refusal at its span, in
/// source order. Empty when the unit selects the model profile.
pub(super) fn refusals(
    profiles: &[ProfileSelection],
    declarations: &Declarations<'_>,
) -> Vec<AssemblyError> {
    if selects_model(profiles) {
        return Vec::new();
    }
    let selected = selected_layers(profiles);
    let none = BTreeSet::new();
    let mut errors = Vec::new();
    let mut refuse = |family, owner_kind, owner: &str, span| {
        errors.push(AssemblyError {
            cause: AssemblyCause::UnselectedModelConstruct(ModelGateRefusal {
                family,
                owner_kind,
                owner: owner.to_owned(),
                selected: selected.clone(),
            }),
            span,
        });
    };
    for abstraction in declarations.abstractions {
        refuse(
            ModelFamily::Abstraction,
            OwnerKind::Abstraction,
            &abstraction.name.name,
            abstraction.span,
        );
    }
    for function in declarations.functions {
        let Some(spans) = function.spans() else {
            continue;
        };
        let whole = spans.declaration;
        let bodies = std::iter::once((&function.body, &spans.body))
            .chain(function.measure.as_ref().zip(spans.measure.as_ref()));
        for (body, body_spans) in bodies {
            visit_located(body, body_spans, whole, |node, span| {
                let family = expression_form(node, &none).and_then(ModelFamily::of);
                if let Some(family) = family {
                    refuse(family, OwnerKind::Function, &function.name, span);
                }
                None
            });
        }
    }
    for clause in declarations.state_clauses {
        visit_located(
            &clause.body,
            &clause.spans.body,
            clause.spans.declaration,
            |node, span| {
                let family = expression_form(node, &none).and_then(ModelFamily::of);
                if let Some(family) = family {
                    refuse(family, OwnerKind::StateClause, &clause.name.name, span);
                }
                None
            },
        );
    }
    for body in declarations.operation_bodies {
        let name = &body.label.name;
        refuse(
            ModelFamily::OperationBody,
            OwnerKind::OperationBody,
            name,
            body.spans.declaration,
        );
        visit_located(
            &body.body,
            &body.spans.body,
            body.spans.declaration,
            |node, span| {
                let family = expression_form(node, &none).and_then(ModelFamily::of);
                if let Some(family) = family {
                    refuse(family, OwnerKind::OperationBody, name, span);
                }
                None
            },
        );
    }
    errors.sort_by_key(|error| (error.span.start, error.span.end));
    errors
}
