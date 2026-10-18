// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124: the intended terminal states of a state model, from the unit's
//! `terminal` declarations (ADR-018 DL-1).
//!
//! The assembler resolves each declaration into a
//! [`TerminalClauseDeclaration`] and refuses a second declaration for one
//! model alias. S3 checks a `when` predicate as the one clause checker checks
//! an invariant (FR-104), so a read of a parameter, a result or a pre-state
//! refuses as it does in an invariant. The graph keeps one
//! [`CheckedTerminal`] per selected model: [`TerminalDeclaration::None`] for a
//! model with no declaration.
//!
//! The predicate's `self` is the model's state object: the one object type
//! the model's populations range over, so `self.value = 3` holds at a state
//! exactly when it holds for every object of that population, as an
//! invariant's does.

use qsl_forms::{ExpressionForm, StateClauseKind};
use qsl_foundation::selection::ProfileSelection;
use qsl_foundation::Span;
use quire_exact::{EffectiveId, ValueType};

use super::ir::{CheckedBody, CheckedNode};
use super::refusal::{CheckCause, CheckRefusal};
use super::state_clause::{check_clause, ClauseDeclarations, ClauseView, PopulationDomain};
use crate::family::CheckContext;

/// One `terminal` declaration the assembler resolved (FR-124 "Inputs"): its
/// `using` alias to a profile selection and its model alias to a selected
/// model.
#[derive(Clone, Debug)]
pub struct TerminalClauseDeclaration {
    /// The declared name.
    pub name: String,
    /// The model alias the declaration is on.
    pub model: String,
    /// The profile selection the `using` alias resolved to.
    pub selection: ProfileSelection,
    /// The span of the whole declaration.
    pub span: Span,
    /// Which terminal states the declaration marks intended.
    pub kind: TerminalClauseKind,
}

/// The two forms of a `terminal` declaration, resolved.
#[derive(Clone, Debug)]
pub enum TerminalClauseKind {
    /// `terminal any`: every terminal state is intended.
    Any,
    /// `terminal when { P }`: a terminal state satisfying `P` is intended.
    When {
        /// The predicate as S2 built it.
        predicate: ExpressionForm,
        /// The model's state object, the type of `self`.
        context: EffectiveId,
        /// The population the object type ranges over.
        population: Option<PopulationDomain>,
    },
}

/// The checked body of a `terminal when` predicate: a Boolean over `self`,
/// the evaluation reads at each state's observation.
#[derive(Debug)]
pub struct CheckedTerminalPredicate {
    pub(crate) parameters: Vec<(String, ValueType)>,
    pub(crate) body: CheckedBody,
    pub(crate) slots: usize,
}

impl CheckedTerminalPredicate {
    /// `self` with its type: the one slot the body is checked over.
    pub fn parameters(&self) -> &[(String, ValueType)] {
        &self.parameters
    }

    /// The checked Boolean body.
    pub fn body(&self) -> CheckedNode<'_> {
        self.body.root()
    }

    /// The number of evaluation slots of the body.
    pub fn slots(&self) -> usize {
        self.slots
    }
}

/// The intended terminal states of a state model (FR-124 "Outputs").
#[derive(Debug)]
pub enum TerminalDeclaration {
    /// The model declares none: it intends no terminal state.
    None,
    /// A terminal state is intended when the predicate holds at it.
    When(CheckedTerminalPredicate),
    /// Every terminal state is intended.
    Any,
}

/// One selected model's checked terminal declaration.
#[derive(Debug)]
pub struct CheckedTerminal {
    pub(crate) model: String,
    pub(crate) name: Option<String>,
    pub(crate) declaration: TerminalDeclaration,
}

impl CheckedTerminal {
    /// The model alias.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The declared name; `None` when the model declares none.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The model's intended terminal states.
    pub fn declaration(&self) -> &TerminalDeclaration {
        &self.declaration
    }
}

/// A refusal of a `terminal when` predicate's checking: the declaration it is
/// about and the refusal itself.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TerminalRefusal {
    /// The declaration's index among the package's `terminal` declarations.
    pub declaration: usize,
    /// The checking refusal.
    pub cause: CheckCause,
}

/// Checks the `terminal` declaration `declaration`, the `index`th of the
/// package, and returns it with the package's running node total after it.
pub(crate) fn check_terminal<'a>(
    index: usize,
    declaration: &TerminalClauseDeclaration,
    declarations: &ClauseDeclarations<'a>,
    cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
) -> Result<(TerminalDeclaration, u64), CheckRefusal> {
    let TerminalClauseKind::When {
        predicate,
        context,
        population,
    } = &declaration.kind
    else {
        return Ok((TerminalDeclaration::Any, declarations.nodes_used));
    };
    let view = ClauseView {
        kind: StateClauseKind::Invariant,
        complete_value: super::state_clause::is_complete_value(&declaration.selection),
        context: *context,
        receiver: ValueType::Reference(*context),
        operation: None,
        context_population: *population,
        frame_population: None,
        body: &predicate.expression,
    };
    let typed = check_clause(&view, declarations, &mut cx.reborrow(declarations.scope)).map_err(
        |refusal| CheckRefusal {
            location: refusal.location,
            cause: CheckCause::Terminal(Box::new(TerminalRefusal {
                declaration: index,
                cause: refusal.cause,
            })),
        },
    )?;
    Ok((
        TerminalDeclaration::When(CheckedTerminalPredicate {
            parameters: typed.parameters,
            body: typed.body,
            slots: typed.slots,
        }),
        typed.nodes_used,
    ))
}
