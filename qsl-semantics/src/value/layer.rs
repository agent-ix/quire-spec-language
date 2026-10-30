// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-110 "Layer selection": the header-selectable layers of QSpec AD-003's
//! state and value/model hierarchy.
//!
//! A [`HeaderLayer`] is one local policy role: its identity, direct
//! requirements, admitted forms, and witness request are defined by exhaustive
//! matches over the closed enum.

/// A header-selectable layer of QSpec AD-003 (QSpec FR-453), in the order
/// of the closed `header_selectable_layers` roles.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum HeaderLayer {
    /// `quire.state.core/v1`: bounded scalars, presence and control, finite
    /// ordered sequences, in state clauses.
    StateCore,
    /// `quire.state.queries/v1`: state core plus named predicates and the
    /// ordered queries.
    StateQueries,
    /// `quire.state.graph/v1`: state queries plus references and
    /// reachability.
    StateGraph,
    /// `quire.value.complete/v1`: state core plus complete values and pure
    /// functions.
    ValueComplete,
    /// `quire.model.complete/v1`: state graph and complete values plus
    /// systems-model structure.
    ModelComplete,
    /// `quire.temporal.complete/v1`: the complete model plus timed models.
    TemporalComplete,
}

impl HeaderLayer {
    /// Every layer, in enum order.
    pub const ALL: [Self; 6] = [
        Self::StateCore,
        Self::StateQueries,
        Self::StateGraph,
        Self::ValueComplete,
        Self::ModelComplete,
        Self::TemporalComplete,
    ];

    /// The layer's definition identity.
    pub fn identity(self) -> &'static str {
        match self {
            Self::StateCore => "quire.state.core/v1",
            Self::StateQueries => "quire.state.queries/v1",
            Self::StateGraph => "quire.state.graph/v1",
            Self::ValueComplete => "quire.value.complete/v1",
            Self::ModelComplete => "quire.model.complete/v1",
            Self::TemporalComplete => "quire.temporal.complete/v1",
        }
    }

    /// The package definition reference associated with this layer role.
    pub fn reference(self) -> super::definition::DefinitionReference {
        super::definition::DefinitionReference {
            authority: "agent-ix".to_owned(),
            identity: self.identity().to_owned(),
        }
    }

    /// Direct layer requirements.
    pub const fn requires(self) -> &'static [Self] {
        match self {
            Self::StateCore => &[],
            Self::StateQueries => &[Self::StateCore],
            Self::StateGraph => &[Self::StateQueries],
            Self::ValueComplete => &[Self::StateCore],
            Self::ModelComplete => &[Self::StateGraph, Self::ValueComplete],
            Self::TemporalComplete => &[Self::ModelComplete],
        }
    }

    /// The witness request this layer requires.
    pub const fn witness_request(self) -> WitnessRequest {
        match self {
            Self::ValueComplete => WitnessRequest {
                kind: WitnessKind::ValueValidity,
                extent: "bounded",
            },
            Self::StateCore
            | Self::StateQueries
            | Self::StateGraph
            | Self::ModelComplete
            | Self::TemporalComplete => WitnessRequest {
                kind: WitnessKind::OperationContract,
                extent: "bounded",
            },
        }
    }

    /// Forms this layer adds to those of its direct requirements.
    pub const fn forms(self) -> &'static [LayerForm] {
        match self {
            Self::StateCore => &[LayerForm::StateClause],
            Self::StateQueries => &[
                LayerForm::PredicateDeclaration,
                LayerForm::NamedPredicateCall,
                LayerForm::SequenceQuery,
            ],
            Self::StateGraph => &[LayerForm::ObjectNavigation],
            Self::ValueComplete => &[
                LayerForm::FunctionDeclaration,
                LayerForm::ValueConstruct,
                LayerForm::PredicateDeclaration,
                LayerForm::NamedPredicateCall,
                LayerForm::SequenceQuery,
            ],
            Self::ModelComplete => &[
                LayerForm::OperationBody,
                LayerForm::DispatchCall,
                LayerForm::Lookup,
            ],
            Self::TemporalComplete => &[],
        }
    }

    /// The selected layers and their transitive requirements.
    pub fn closure(selected: &[Self]) -> Vec<Self> {
        let mut reached = std::collections::BTreeSet::new();
        let mut pending = selected.to_vec();
        while let Some(layer) = pending.pop() {
            if reached.insert(layer) {
                pending.extend_from_slice(layer.requires());
            }
        }
        reached.into_iter().collect()
    }

    /// The forms admitted by this layer and its requirement closure.
    pub fn admitted_forms(self) -> std::collections::BTreeSet<LayerForm> {
        Self::closure(&[self])
            .into_iter()
            .flat_map(|layer| layer.forms().iter().copied())
            .collect()
    }

    /// Whether this layer admits a form through its requirement closure.
    pub fn admits(self, form: LayerForm) -> bool {
        self.admitted_forms().contains(&form)
    }

    /// The layer a header identity names, if it names one.
    #[ix_trace_rs::string_edge]
    pub fn from_identity(identity: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|layer| layer.identity() == identity)
    }
}

/// What kind of witness a layer asks for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WitnessKind {
    /// `operation-contract`.
    OperationContract,
    /// `value-validity`.
    ValueValidity,
}

impl WitnessKind {
    /// Normative spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OperationContract => "operation-contract",
            Self::ValueValidity => "value-validity",
        }
    }
}

/// The witness a layer's qualification asks for: its
/// target kind and the extent its tier bounds it to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WitnessRequest {
    /// The witness target.
    pub kind: WitnessKind,
    /// The extent, `bounded` for every layer.
    pub extent: &'static str,
}

/// A form of the unit language a layer's admitted-form set holds or lacks.
/// The first three are declaration forms; the rest are expression forms.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LayerForm {
    /// An `invariant`, `pre` or `post` state clause.
    StateClause,
    /// A `predicate` declaration.
    PredicateDeclaration,
    /// A `function` declaration.
    FunctionDeclaration,
    /// A `body` operation-body declaration (FR-361 item 5).
    OperationBody,
    /// A call of a named predicate.
    NamedPredicateCall,
    /// `filter`, `map`, `flatMap`, `count` or `sum` over a sequence.
    SequenceQuery,
    /// `deref` or `reaches`.
    ObjectNavigation,
    /// A value construct beyond state core: a record or collection literal,
    /// `convert`, `flatten`, `fold`, `reduce`, a union `case`, a function
    /// call, integer `div`, `rem` or `mod`.
    ValueConstruct,
    /// A postfix `receiver.member(args)` operation call (FR-361 item 5).
    DispatchCall,
    /// The `lookup<T>(p, r) absent m` expression (FR-361 item 5).
    Lookup,
}

impl LayerForm {
    /// Whether the form is a declaration form, so its refusal is
    /// `unsupported_construct`/`declaration-form`; an expression form's is
    /// `unsupported_construct`/`expression-form`.
    pub fn is_declaration(self) -> bool {
        match self {
            Self::StateClause
            | Self::PredicateDeclaration
            | Self::FunctionDeclaration
            | Self::OperationBody => true,
            Self::NamedPredicateCall
            | Self::SequenceQuery
            | Self::ObjectNavigation
            | Self::ValueConstruct
            | Self::DispatchCall
            | Self::Lookup => false,
        }
    }

    /// Whether the form is a model construct (FR-361 item 5): one the model
    /// layer adds, and one the unit's own selection of that layer gates.
    pub fn is_model_construct(self) -> bool {
        match self {
            Self::OperationBody | Self::DispatchCall | Self::Lookup => true,
            Self::StateClause
            | Self::PredicateDeclaration
            | Self::FunctionDeclaration
            | Self::NamedPredicateCall
            | Self::SequenceQuery
            | Self::ObjectNavigation
            | Self::ValueConstruct => false,
        }
    }

    /// The `quire.native.diagnostics/v1` cause tag of a refusal of this form.
    pub fn refusal_cause(self) -> &'static str {
        if self.is_declaration() {
            "declaration-form"
        } else {
            "expression-form"
        }
    }
}

// `value.complete` adds complete values and pure functions to state core.
// Predicate declarations and ordered queries are pure value forms, but object
// navigation belongs to `state.graph` (FR-361 item 17).
