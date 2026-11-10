// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560: the first construct of an entry, in clause-node order, that lies
//! outside the backend's coverage, and the cause the spec gives it.

use super::{BackendCoverage, CoverageOperation, CoverageType};
use qsl_semantics::check::CheckedNode;
use quire_checked::node::{
    Arithmetic, Connective, IntegerDivisionOperator, NodeKind, OrderedKind, Visit,
};
use quire_eval::UntranslatableConstruct;
use quire_exact::{OrderingOperator, ValueType};
use quire_semantic_value::declaration::EqualityOperator;
use quire_semantic_value::location::ExpressionLocation;

/// A construct outside a coverage and where it is.
#[derive(Clone, Debug)]
pub struct Outside {
    /// The construct, as FR-294's `Untranslatable` cause names it.
    pub construct: UntranslatableConstruct,
    /// Where it is.
    pub location: ExpressionLocation,
}

/// The first construct of the function `parameters`/`body` outside
/// `coverage`, or `None` when every construct lies inside it.
///
/// Clause-node order (FR-294) is the pre-order of the checked tree with the
/// declared parameters first. Per construct the rows of FR-560's table apply
/// in the order the spec fixes: an unlisted form wins over an unlisted value
/// type, which wins over an unlisted operation.
#[must_use]
pub fn first_outside(
    parameters: &[(String, ValueType)],
    body: CheckedNode<'_>,
    coverage: &BackendCoverage,
) -> Option<Outside> {
    for (_, value_type) in parameters {
        if !coverage.lists_type(value_type) {
            return Some(Outside {
                construct: UntranslatableConstruct::ValueType(Box::new(value_type.clone())),
                location: body.location().clone(),
            });
        }
    }
    // Pre-order without host recursion: a deep expression must not overflow
    // the stack of the thread that prepares it.
    let mut pending = vec![body];
    while let Some(node) = pending.pop() {
        let children = node.children();
        if let Some(construct) = outside_construct(node, &children, coverage) {
            return Some(Outside {
                construct,
                location: node.location().clone(),
            });
        }
        pending.extend(children.into_iter().rev());
    }
    None
}

fn outside_construct(
    node: CheckedNode<'_>,
    children: &[CheckedNode<'_>],
    coverage: &BackendCoverage,
) -> Option<UntranslatableConstruct> {
    if let Some(form) = form_name(node) {
        if !coverage.lists_form(&form) {
            return Some(UntranslatableConstruct::ExpressionForm(form));
        }
    }
    let mut types =
        std::iter::once(node.value_type()).chain(children.iter().map(|c| c.value_type()));
    if let Some(unlisted) = types.find(|t| !coverage.lists_type(t)) {
        return Some(UntranslatableConstruct::ValueType(Box::new(
            unlisted.clone(),
        )));
    }
    let operation = operation_of(node, children)?;
    if coverage.lists_operation(&operation) {
        return None;
    }
    // FR-560 gives `Operation(k)` with `k` the operation's node key. The
    // checked representation exposes no per-node key (`CheckedNode` has a
    // `Location` and no `NodeKey`), so the key cannot be produced; the entry
    // is still listed unsupported, with its form as the nearest truthful
    // cause. Spec gap: see the QSL-602 step 4b report.
    Some(UntranslatableConstruct::ExpressionForm(
        form_name(node).unwrap_or_else(|| "operator".to_owned()),
    ))
}

/// The ADR-012 form name of a node, `None` for a local read (an operand
/// reference, not a form of FR-560's table).
fn form_name(node: CheckedNode<'_>) -> Option<String> {
    let name = match node.kind() {
        NodeKind::Local(_) => return None,
        NodeKind::Literal(_) => "literal",
        NodeKind::Let { .. } => "let",
        NodeKind::If { .. } => "if",
        NodeKind::Coerce(..)
        | NodeKind::ConvertScalar(..)
        | NodeKind::Arithmetic(..)
        | NodeKind::IntegerDivision { .. }
        | NodeKind::Negate(_)
        | NodeKind::Divide { .. }
        | NodeKind::Rational { .. }
        | NodeKind::RationalNegate(..)
        | NodeKind::Decimal { .. }
        | NodeKind::DecimalNegate(..)
        | NodeKind::ConvertDecimal(..)
        | NodeKind::Order(..)
        | NodeKind::Equality(..)
        | NodeKind::Connective(..)
        | NodeKind::Not(_) => "operator",
        NodeKind::Call { .. } | NodeKind::ImportedCall { .. } | NodeKind::Dispatch { .. } => "call",
        NodeKind::Record { .. } | NodeKind::Tuple { .. } => "record",
        NodeKind::Collection { .. } => "collection",
        NodeKind::Query { visit, .. } => match visit {
            Visit::Forall => "forall",
            Visit::Exists => "exists",
            _ => "collection",
        },
        NodeKind::Present(_) | NodeKind::PlacePresence { .. } | NodeKind::SlotPresence { .. } => {
            "present"
        }
        NodeKind::Attribute { .. } | NodeKind::Navigate { .. } => "deref",
        NodeKind::Field { .. } => {
            let operand_is_record = node
                .children()
                .first()
                .is_some_and(|operand| matches!(operand.value_type(), ValueType::Composite(_)));
            if operand_is_record {
                "record"
            } else {
                "field"
            }
        }
        NodeKind::Reaches { .. } | NodeKind::ReachesEnd { .. } => "reaches",
        NodeKind::Case { .. } => "case",
        other => return Some(variant_name(other)),
    };
    Some(name.to_owned())
}

/// The lower-cased variant name of a checked node kind: FR-560 names the
/// forms outside its table by the variant names of the checked syntax.
fn variant_name<C: std::fmt::Debug>(kind: &NodeKind<C>) -> String {
    format!("{kind:?}")
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

/// The `Value`-family operation of an `operator` node, as a coverage entry.
/// An operator this backend does not implement gets the name `other`, which
/// no coverage lists.
fn operation_of(node: CheckedNode<'_>, children: &[CheckedNode<'_>]) -> Option<CoverageOperation> {
    let operator = match node.kind() {
        NodeKind::Arithmetic(Arithmetic::Add, ..) => "+",
        NodeKind::Arithmetic(Arithmetic::Subtract, ..) => "-",
        NodeKind::Arithmetic(Arithmetic::Multiply, ..) => "*",
        NodeKind::Negate(_) => "negate",
        NodeKind::IntegerDivision { operator, .. } => match operator {
            IntegerDivisionOperator::Div => "div",
            IntegerDivisionOperator::Rem => "rem",
            IntegerDivisionOperator::Mod => "mod",
        },
        NodeKind::Order(operator, OrderedKind::Integers, ..) => match operator {
            OrderingOperator::Less => "<",
            OrderingOperator::LessOrEqual => "<=",
            OrderingOperator::Greater => ">",
            OrderingOperator::GreaterOrEqual => ">=",
        },
        NodeKind::Coerce(..) => "coerce",
        NodeKind::ConvertScalar(..) => "convert",
        NodeKind::Equality(operator, ..) => match operator {
            EqualityOperator::Equal => "=",
            EqualityOperator::NotEqual => "!=",
        },
        NodeKind::Connective(connective, ..) => match connective {
            Connective::And => "and",
            Connective::Or => "or",
            Connective::Implies => "implies",
        },
        NodeKind::Not(_) => "not",
        NodeKind::Divide { .. }
        | NodeKind::Rational { .. }
        | NodeKind::RationalNegate(..)
        | NodeKind::Decimal { .. }
        | NodeKind::DecimalNegate(..)
        | NodeKind::ConvertDecimal(..)
        | NodeKind::Order(..) => "other",
        _ => return None,
    };
    let operand = children
        .first()
        .and_then(|child| CoverageType::of(child.value_type()))
        .unwrap_or(CoverageType::Boolean);
    Some(CoverageOperation { operator, operand })
}

/// The cause for a construct that lies in the coverage but that this backend
/// cannot yet translate (an operation whose FR-562 fast-path condition fails,
/// which would call the kernel through a value handle this backend does not
/// yet have). FR-560 gives such a construct `Operation(k)` with `k` its node
/// key, which the checked representation does not expose (see
/// [`outside_construct`]), so the cause is the construct's form.
#[must_use]
pub fn untranslated(node: CheckedNode<'_>) -> Outside {
    Outside {
        construct: UntranslatableConstruct::ExpressionForm(
            form_name(node).unwrap_or_else(|| "operator".to_owned()),
        ),
        location: node.location().clone(),
    }
}
