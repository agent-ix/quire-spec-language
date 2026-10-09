// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `TemporalTrace` family's one S2 production (FR-325, ADR-011 M-3b):
//! a `temporal` clause and its formula.
//!
//! This module reads the CST of one `TemporalClause` declaration and builds
//! its [`TemporalClauseForm`]. It selects no meaning, the same contract
//! `value`'s productions keep (FR-091): the `using` alias, the `over` type
//! and each fairness operation stay spelled and spanned, and an operator's
//! interval is built as written. The formula is an arena built with an
//! explicit work stack, so a formula of any depth builds without recursion.

use qsl_cst::{CstNode, LosslessCst, Production, TokenKind};
use qsl_foundation::Span;

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{
    ActivationForm, CaptureForm, DeclarationForm, ExpressionForm, FairnessConstraintForm,
    FairnessGranularity, FairnessKind, IntervalForm, IntervalUpper, ParameterForm,
    TemporalClauseForm, TemporalFormulaForm, TemporalNodeForm, TemporalNodeId, TemporalOperator,
    TemporalOperatorForm, UsingAlias,
};
use super::value::{
    declared_name, expression, has_token, items, name_form, nodes_of, only, production_node, text,
    tokens_of, type_form, unexpected, Item,
};

/// `temporal Name using p over (x: T) clock "c" activation { fairness*
/// captures formula }` (FR-325 "Outputs").
pub(crate) fn temporal_clause(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let clause_items = items(cst, node);
    // `temporal Name using p ...`: the clause's own name and its `using`
    // alias are its only two direct identifier tokens.
    let [_, alias] = tokens_of(&clause_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(node));
    };
    let activation = only(&clause_items, Production::Activation, node)?;
    let activation_items = items(cst, activation);
    let activation = match nodes_of(&activation_items, Production::Parameter).first() {
        None => ActivationForm::Origin,
        Some(parameter) => ActivationForm::Each {
            parameter: parameter_form(cst, parameter)?,
            when: nodes_of(&activation_items, Production::Expression)
                .first()
                .map(|condition| expression_form(cst, condition))
                .transpose()?,
        },
    };
    let fairness = nodes_of(&clause_items, Production::Fairness)
        .into_iter()
        .map(|constraint| fairness_form(cst, constraint))
        .collect::<Result<_, _>>()?;
    let captures = nodes_of(&clause_items, Production::Capture)
        .into_iter()
        .map(|capture| capture_form(cst, capture))
        .collect::<Result<_, _>>()?;
    let formula = only(&clause_items, Production::TemporalExpression, node)?;
    Ok(DeclarationForm::Temporal(Box::new(TemporalClauseForm {
        name: declared_name(&clause_items, node)?,
        profile: UsingAlias {
            alias: text(alias, node)?,
            span: alias.span(),
        },
        over: parameter_form(cst, only(&clause_items, Production::Parameter, node)?)?,
        activation,
        fairness,
        captures,
        formula: formula_form(cst, formula)?,
    })))
}

/// One `Parameter` node (`ident : ParameterType`) as a parameter form.
fn parameter_form(cst: &LosslessCst, parameter: &CstNode) -> Result<ParameterForm, FormsRefusal> {
    let parameter_items = items(cst, parameter);
    let declared = only(&parameter_items, Production::ParameterType, parameter)?;
    let declared_items = items(cst, declared);
    let reference = only(&declared_items, Production::TypeReference, declared)?;
    Ok(ParameterForm {
        name: declared_name(&parameter_items, parameter)?,
        value_type: type_form(cst, reference)?,
    })
}

/// `capture name: Type = value ;`.
fn capture_form(cst: &LosslessCst, capture: &CstNode) -> Result<CaptureForm, FormsRefusal> {
    let capture_items = items(cst, capture);
    Ok(CaptureForm {
        parameter: parameter_form(cst, only(&capture_items, Production::Parameter, capture)?)?,
        value: expression_form(cst, only(&capture_items, Production::Expression, capture)?)?,
        span: capture.span(),
    })
}

/// One `Expression` node as a condition with its spans.
fn expression_form(cst: &LosslessCst, node: &CstNode) -> Result<ExpressionForm, FormsRefusal> {
    let (expression, spans) = expression(cst, node)?;
    Ok(ExpressionForm { expression, spans })
}

/// `fair [weak | strong] [whole | each] Operation ;`: an unwritten kind is
/// `weak` (FR-362) and an unwritten granularity stays `None`, for S3 to
/// read as `whole` (FR-123).
fn fairness_form(
    cst: &LosslessCst,
    constraint: &CstNode,
) -> Result<FairnessConstraintForm, FormsRefusal> {
    let constraint_items = items(cst, constraint);
    let granularity = match (
        has_token(&constraint_items, b"whole"),
        has_token(&constraint_items, b"each"),
    ) {
        (true, false) => Some(FairnessGranularity::Whole),
        (false, true) => Some(FairnessGranularity::Each),
        (false, false) => None,
        (true, true) => return Err(unexpected(constraint)),
    };
    Ok(FairnessConstraintForm {
        kind: if has_token(&constraint_items, b"strong") {
            FairnessKind::Strong
        } else {
            FairnessKind::Weak
        },
        operation: name_form(
            cst,
            only(&constraint_items, Production::QualifiedName, constraint)?,
        )?,
        granularity,
        span: constraint.span(),
    })
}

/// One `Interval` node (`[a, b]` or `[a, *]`) as written. S1 admits only
/// bounds that fit `u64`, so a bound that does not parse here is a broken
/// invariant between the two stages.
fn interval_form(cst: &LosslessCst, interval: &CstNode) -> Result<IntervalForm, FormsRefusal> {
    let interval_items = items(cst, interval);
    let bound = |token: &qsl_cst::CstToken| {
        text(token, interval)?
            .parse::<u64>()
            .map_err(|_| unexpected(interval))
    };
    let bounds = tokens_of(&interval_items, TokenKind::Integer);
    let (lower, upper) = match (&bounds[..], has_token(&interval_items, b"*")) {
        ([lower, upper], false) => (bound(lower)?, IntervalUpper::Finite(bound(upper)?)),
        ([lower], true) => (bound(lower)?, IntervalUpper::Open),
        _ => return Err(unexpected(interval)),
    };
    Ok(IntervalForm {
        lower,
        upper,
        span: interval.span(),
    })
}

/// A formula CST node (`TemporalExpression` or any production under it) as
/// a formula arena.
fn formula_form(cst: &LosslessCst, root: &CstNode) -> Result<TemporalFormulaForm, FormsRefusal> {
    let mut build = Formula {
        cst,
        nodes: Vec::new(),
        work: vec![Task {
            node: root,
            parent: None,
        }],
    };
    while let Some(task) = build.work.pop() {
        build.map(task)?;
    }
    // The root is the first node added: every level above it passes through.
    if build.nodes.is_empty() {
        return Err(unexpected(root));
    }
    Ok(TemporalFormulaForm::new(build.nodes, TemporalNodeId(0)))
}

/// One CST node still to map, with the arena index of the operator node
/// that takes the result as an operand.
#[derive(Clone, Copy)]
struct Task<'c> {
    node: &'c CstNode,
    parent: Option<usize>,
}

struct Formula<'c> {
    cst: &'c LosslessCst,
    nodes: Vec<TemporalNodeForm>,
    work: Vec<Task<'c>>,
}

/// The operator a temporal keyword spells.
fn operator_of(spelling: &[u8]) -> Option<TemporalOperator> {
    match spelling {
        b"eventually" => Some(TemporalOperator::Eventually),
        b"always" => Some(TemporalOperator::Always),
        b"once" => Some(TemporalOperator::Once),
        b"historically" => Some(TemporalOperator::Historically),
        b"until" => Some(TemporalOperator::Until),
        b"release" => Some(TemporalOperator::Release),
        b"since" => Some(TemporalOperator::Since),
        b"triggered" => Some(TemporalOperator::Triggered),
        _ => None,
    }
}

impl<'c> Formula<'c> {
    /// Add one node, as the next operand of operator node `parent`.
    fn add(&mut self, form: TemporalNodeForm, parent: Option<usize>) -> usize {
        let index = self.nodes.len();
        if let Some(TemporalNodeForm::Operator(operator)) =
            parent.and_then(|parent| self.nodes.get_mut(parent))
        {
            operator.operands.push(TemporalNodeId(index));
        }
        self.nodes.push(form);
        index
    }

    fn operator_node(
        &mut self,
        operator: TemporalOperator,
        interval: Option<IntervalForm>,
        span: Span,
        operator_span: Span,
        parent: Option<usize>,
    ) -> usize {
        self.add(
            TemporalNodeForm::Operator(TemporalOperatorForm {
                operator,
                interval,
                operands: Vec::new(),
                span,
                operator_span,
            }),
            parent,
        )
    }

    /// Queue `children` under operator node `parent`, so the first is
    /// mapped first.
    fn queue(&mut self, parent: usize, children: &[&'c CstNode]) {
        self.work.extend(children.iter().rev().map(|&node| Task {
            node,
            parent: Some(parent),
        }));
    }

    /// Continue with `child` in `task`'s place.
    fn pass(&mut self, task: Task<'c>, child: &'c CstNode) {
        self.work.push(Task {
            node: child,
            parent: task.parent,
        });
    }

    fn map(&mut self, task: Task<'c>) -> Result<(), FormsRefusal> {
        let node = task.node;
        let items = items(self.cst, node);
        match node.production() {
            Production::TemporalExpression => self.single(task, &items),
            Production::TemporalImplication => self.implication(task, &items),
            Production::TemporalDisjunction => self.chain(task, &items, TemporalOperator::Or),
            Production::TemporalConjunction => self.chain(task, &items, TemporalOperator::And),
            Production::TemporalRelation => self.relation(task, &items),
            Production::TemporalUnary => self.unary(task, &items),
            Production::TemporalPrimary => self.primary(task, &items),
            _ => Err(unexpected(node)),
        }
    }

    /// A level with one operand and no operator.
    fn single(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsRefusal> {
        let [Item::Node(operand)] = items else {
            return Err(unexpected(task.node));
        };
        self.pass(task, operand);
        Ok(())
    }

    /// `d` or `d implies i`: right-associative, as the CST nests it.
    fn implication(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsRefusal> {
        let [Item::Node(left), Item::Token(token), Item::Node(right)] = items else {
            return self.single(task, items);
        };
        if token.spelling() != b"implies" {
            return Err(unexpected(task.node));
        }
        let index = self.operator_node(
            TemporalOperator::Implies,
            None,
            task.node.span(),
            token.span(),
            task.parent,
        );
        self.queue(index, &[left, right]);
        Ok(())
    }

    /// `o0 op o1 op ... op on`: the left-associative chain
    /// `op(... op(o0, o1) ..., on)`, links added root first.
    fn chain(
        &mut self,
        task: Task<'c>,
        items: &[Item<'c>],
        operator: TemporalOperator,
    ) -> Result<(), FormsRefusal> {
        let node = task.node;
        let mut operands = Vec::new();
        let mut tokens = Vec::new();
        for item in items {
            match item {
                Item::Node(operand) => operands.push(*operand),
                Item::Token(token) => tokens.push(*token),
            }
        }
        if tokens.is_empty() {
            return self.single(task, items);
        }
        if operands.len() != tokens.len() + 1 {
            return Err(unexpected(node));
        }
        let start = node.span().start;
        let mut parent = task.parent;
        let mut links = Vec::with_capacity(tokens.len());
        for (token, right) in tokens.iter().zip(&operands[1..]).rev() {
            let span = Span {
                start,
                end: right.span().end,
            };
            let index = self.operator_node(operator, None, span, token.span(), parent);
            links.push(index);
            parent = Some(index);
        }
        // `links` runs from the root link (applying the last operator) to
        // link 1 (applying the first, to `o0` and `o1`).
        let Some((&first, upper)) = links.split_last() else {
            return Err(unexpected(node));
        };
        self.queue(first, &[operands[0], operands[1]]);
        for (link, operand) in upper.iter().rev().zip(&operands[2..]) {
            self.queue(*link, &[operand]);
        }
        Ok(())
    }

    /// `u` or `u op [interval]? u` for a binary temporal operator.
    fn relation(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsRefusal> {
        if items.len() == 1 {
            return self.single(task, items);
        }
        self.apply(task, items, 2)
    }

    /// `not u`, `op [interval]? u` for a unary temporal operator, or a
    /// primary.
    fn unary(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsRefusal> {
        match items.first() {
            Some(Item::Node(_)) => self.single(task, items),
            Some(Item::Token(token)) if token.spelling() == b"not" => {
                let [_, Item::Node(operand)] = items else {
                    return Err(unexpected(task.node));
                };
                let index = self.operator_node(
                    TemporalOperator::Not,
                    None,
                    task.node.span(),
                    token.span(),
                    task.parent,
                );
                self.queue(index, &[operand]);
                Ok(())
            }
            Some(Item::Token(_)) => self.apply(task, items, 1),
            None => Err(unexpected(task.node)),
        }
    }

    /// A temporal operator keyword with an optional interval, applied to
    /// `arity` operands among `items`.
    fn apply(
        &mut self,
        task: Task<'c>,
        items: &[Item<'c>],
        arity: usize,
    ) -> Result<(), FormsRefusal> {
        let node = task.node;
        let token = items
            .iter()
            .find_map(|item| match item {
                Item::Token(token) => Some(*token),
                Item::Node(_) => None,
            })
            .ok_or_else(|| unexpected(node))?;
        let operator = operator_of(token.spelling()).ok_or_else(|| unexpected(node))?;
        let interval = nodes_of(items, Production::Interval)
            .first()
            .map(|interval| interval_form(self.cst, interval))
            .transpose()?;
        let operands: Vec<&CstNode> = items
            .iter()
            .filter_map(|item| match item {
                Item::Node(operand) if operand.production() != Production::Interval => {
                    Some(*operand)
                }
                Item::Node(_) | Item::Token(_) => None,
            })
            .collect();
        if operands.len() != arity {
            return Err(unexpected(node));
        }
        let index = self.operator_node(operator, interval, node.span(), token.span(), task.parent);
        self.queue(index, &operands);
        Ok(())
    }

    /// `true`, `false`, `( e )` or `holds ( x )`.
    fn primary(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsRefusal> {
        let node = task.node;
        match items {
            [Item::Token(token)] if token.spelling() == b"true" || token.spelling() == b"false" => {
                self.add(
                    TemporalNodeForm::Constant {
                        value: token.spelling() == b"true",
                        span: node.span(),
                    },
                    task.parent,
                );
                Ok(())
            }
            [Item::Token(open), Item::Node(inner), Item::Token(_)] if open.spelling() == b"(" => {
                self.pass(task, inner);
                Ok(())
            }
            [Item::Token(holds), Item::Token(_), Item::Node(condition), Item::Token(_)]
                if holds.spelling() == b"holds" =>
            {
                let condition = expression_form(self.cst, condition)?;
                self.add(
                    TemporalNodeForm::Holds {
                        condition,
                        span: node.span(),
                    },
                    task.parent,
                );
                Ok(())
            }
            _ => Err(unexpected(node)),
        }
    }
}
