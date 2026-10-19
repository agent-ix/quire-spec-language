// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-110 "Layer selection": the header-selectable layers of QSpec AD-003's
//! state and value/model hierarchy, as the `header_selectable_layers` rows
//! of QSpec's `complete-value-lock.json` name them.
//!
//! A [`HeaderLayer`] is one row: its identity, the layers it `requires`, the
//! definition file that states its admitted forms, and the witness request
//! it names. [`DefinitionLock::layer_row`](super::DefinitionLock::layer_row)
//! reads the row; [`DefinitionLock::closure`](super::DefinitionLock::closure)
//! computes a selection's `requires` closure; and
//! [`DefinitionLock::admits`](super::DefinitionLock::admits) answers whether a
//! [`LayerForm`] is in the admitted-form set of a closure. QSpec states the
//! admitted-form sets in prose (`state-core.md`, `state-queries.md`,
//! `state-graph.md`, `value-complete.md`, `model-complete.md`); each
//! [`LayerRow::forms`] is this build's reading of its row's file, over the
//! forms the unit language has today.

/// A header-selectable layer of QSpec AD-003 (QSpec FR-453), in the order
/// of the lock's `header_selectable_layers` rows.
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
}

impl HeaderLayer {
    /// Every layer, in the lock's row order.
    pub const ALL: [Self; 5] = [
        Self::StateCore,
        Self::StateQueries,
        Self::StateGraph,
        Self::ValueComplete,
        Self::ModelComplete,
    ];

    /// The layer's definition identity.
    pub fn identity(self) -> &'static str {
        match self {
            Self::StateCore => "quire.state.core/v1",
            Self::StateQueries => "quire.state.queries/v1",
            Self::StateGraph => "quire.state.graph/v1",
            Self::ValueComplete => "quire.value.complete/v1",
            Self::ModelComplete => "quire.model.complete/v1",
        }
    }

    /// The layer a header identity names, if it names one.
    #[qsl_attrs::string_edge]
    pub fn from_identity(identity: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|layer| layer.identity() == identity)
    }
}

/// What kind of witness a layer asks for (`witness_request.kind`).
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

/// The witness a layer's qualification asks for (`witness_request`): its
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

/// One `header_selectable_layers` row of the `Value` family's lock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayerRow {
    /// The row's layer.
    pub layer: HeaderLayer,
    /// The layers it requires, directly.
    pub requires: &'static [HeaderLayer],
    /// The definition file that states its admitted forms.
    pub admitted_forms: &'static str,
    /// The witness request it names.
    pub witness_request: WitnessRequest,
    /// The forms this layer adds to those of the layers it requires.
    pub forms: &'static [LayerForm],
}

const OPERATION_CONTRACT: WitnessRequest = WitnessRequest {
    kind: WitnessKind::OperationContract,
    extent: "bounded",
};

const VALUE_VALIDITY: WitnessRequest = WitnessRequest {
    kind: WitnessKind::ValueValidity,
    extent: "bounded",
};

/// The five rows, in the lock's order.
pub(super) const LAYER_ROWS: [LayerRow; 5] = [
    LayerRow {
        layer: HeaderLayer::StateCore,
        requires: &[],
        admitted_forms: "state-core.md",
        witness_request: OPERATION_CONTRACT,
        forms: &[LayerForm::StateClause],
    },
    LayerRow {
        layer: HeaderLayer::StateQueries,
        requires: &[HeaderLayer::StateCore],
        admitted_forms: "state-queries.md",
        witness_request: OPERATION_CONTRACT,
        forms: &[
            LayerForm::PredicateDeclaration,
            LayerForm::NamedPredicateCall,
            LayerForm::SequenceQuery,
        ],
    },
    LayerRow {
        layer: HeaderLayer::StateGraph,
        requires: &[HeaderLayer::StateQueries],
        admitted_forms: "state-graph.md",
        witness_request: OPERATION_CONTRACT,
        forms: &[LayerForm::ObjectNavigation],
    },
    // `value.complete` adds complete values and pure functions to state core.
    // A predicate is a pure Boolean function and the ordered queries are
    // collection operations, so its set holds them although it does not
    // require `state.queries`. It holds no object navigation: `deref` and
    // `reaches` are `state.graph`'s (FR-361 item 17).
    LayerRow {
        layer: HeaderLayer::ValueComplete,
        requires: &[HeaderLayer::StateCore],
        admitted_forms: "value-complete.md",
        witness_request: VALUE_VALIDITY,
        forms: &[
            LayerForm::FunctionDeclaration,
            LayerForm::ValueConstruct,
            LayerForm::PredicateDeclaration,
            LayerForm::NamedPredicateCall,
            LayerForm::SequenceQuery,
        ],
    },
    // The model constructs (FR-361 item 5): an `operation-body`, a dispatched
    // call and `lookup`.
    LayerRow {
        layer: HeaderLayer::ModelComplete,
        requires: &[HeaderLayer::StateGraph, HeaderLayer::ValueComplete],
        admitted_forms: "model-complete.md",
        witness_request: OPERATION_CONTRACT,
        forms: &[
            LayerForm::OperationBody,
            LayerForm::DispatchCall,
            LayerForm::Lookup,
        ],
    },
];
