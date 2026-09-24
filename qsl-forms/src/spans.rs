// SPDX-License-Identifier: AGPL-3.0-or-later
//! The byte spans of a parsed form's expression nodes (FR-091-AC-10).
//!
//! A form's spans are held beside its [`Expression`] tree, one span per
//! node, in an arena shaped exactly as [`Expression::children`] numbers the
//! tree. That shape is what lets a `check::Location` (a declaration origin
//! plus a child-index path) reach the span of the node it names (FR-096):
//! each path step `i` moves to child `i`.
//!
//! The arena is flat, so building, walking and dropping it never recurse,
//! however deep the tree.

use std::fmt;

use qsl_foundation::Span;

use super::Expression;

/// One node of an [`ExpressionSpans`] arena. Minted only by the arena that
/// holds the node.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SpanId(usize);

#[derive(Clone, Debug, Eq, PartialEq)]
struct SpanNode {
    span: Span,
    children: Vec<SpanId>,
}

/// The spans of one expression tree, one per node: the root at
/// [`Self::root`], and each node's children in [`Expression::children`]
/// order. Every child's span lies inside its parent's.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpressionSpans {
    nodes: Vec<SpanNode>,
}

/// Why a span could not be added to an [`ExpressionSpans`] arena.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpanRefusal {
    /// The parent names no node of this arena.
    UnknownParent(SpanId),
    /// The child's span does not lie inside its parent's.
    OutsideParent {
        /// The parent's span.
        parent: Span,
        /// The refused child span.
        child: Span,
    },
}

impl fmt::Display for SpanRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownParent(parent) => write!(f, "span node {} is not in this tree", parent.0),
            Self::OutsideParent { parent, child } => write!(
                f,
                "span {}..{} lies outside its parent {}..{}",
                child.start, child.end, parent.start, parent.end
            ),
        }
    }
}

impl std::error::Error for SpanRefusal {}

impl ExpressionSpans {
    /// A tree holding only its root node's span.
    pub fn new(root: Span) -> Self {
        Self {
            nodes: vec![SpanNode {
                span: root,
                children: Vec::new(),
            }],
        }
    }

    /// The root node.
    pub fn root(&self) -> SpanId {
        SpanId(0)
    }

    /// Append `span` as the next child of `parent`, in
    /// [`Expression::children`] order.
    pub fn push_child(&mut self, parent: SpanId, span: Span) -> Result<SpanId, SpanRefusal> {
        let id = SpanId(self.nodes.len());
        let node = self
            .nodes
            .get_mut(parent.0)
            .ok_or(SpanRefusal::UnknownParent(parent))?;
        if span.start < node.span.start || span.end > node.span.end || span.start > span.end {
            return Err(SpanRefusal::OutsideParent {
                parent: node.span,
                child: span,
            });
        }
        node.children.push(id);
        self.nodes.push(SpanNode {
            span,
            children: Vec::new(),
        });
        Ok(id)
    }

    /// The span of `node`, when it is a node of this tree.
    pub fn span(&self, node: SpanId) -> Option<Span> {
        self.nodes.get(node.0).map(|node| node.span)
    }

    /// The `index`th child of `node`, numbered as [`Expression::children`]
    /// numbers it.
    pub fn child(&self, node: SpanId, index: usize) -> Option<SpanId> {
        self.nodes.get(node.0)?.children.get(index).copied()
    }

    /// The span of the node `path` reaches from the root, one child index
    /// per step (FR-096); `None` when a step names no child.
    pub fn at(&self, path: &[usize]) -> Option<Span> {
        let mut node = self.root();
        for &index in path {
            node = self.child(node, index)?;
        }
        self.span(node)
    }

    /// Whether this tree has exactly `expression`'s shape: one node per
    /// expression node, each with as many children as
    /// [`Expression::children`] lists. Walks both on an explicit stack.
    pub fn fits(&self, expression: &Expression) -> bool {
        let mut stack = vec![(self.root(), expression)];
        while let Some((node, expression)) = stack.pop() {
            let Some(node) = self.nodes.get(node.0) else {
                return false;
            };
            let children = expression.children();
            if node.children.len() != children.len() {
                return false;
            }
            stack.extend(node.children.iter().copied().zip(children));
        }
        true
    }
}

/// The spans of one function declaration form (FR-091-AC-1, FR-091-AC-10):
/// the whole declaration's, and one per node of its body and of its
/// `decreases` measure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclarationSpans {
    /// The whole declaration.
    pub declaration: Span,
    /// The body's nodes.
    pub body: ExpressionSpans,
    /// The `decreases` measure's nodes, when the declaration writes one.
    pub measure: Option<ExpressionSpans>,
}

/// Which part of a declaration its spans do not fit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpansMismatch {
    /// The body spans do not have the body's shape.
    Body,
    /// The measure spans do not have the measure's shape, or one of the two
    /// is absent.
    Measure,
}

impl fmt::Display for SpansMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Body => f.write_str("the body spans do not fit the body"),
            Self::Measure => f.write_str("the measure spans do not fit the measure"),
        }
    }
}

impl std::error::Error for SpansMismatch {}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    #[test]
    fn a_child_outside_its_parent_is_refused() {
        let mut spans = ExpressionSpans::new(span(2, 5));
        let refused = spans.push_child(spans.root(), span(1, 3));
        assert_eq!(
            refused,
            Err(SpanRefusal::OutsideParent {
                parent: span(2, 5),
                child: span(1, 3),
            })
        );
        assert_eq!(spans.child(spans.root(), 0), None);
    }

    #[test]
    fn a_parent_from_another_tree_is_refused() {
        let mut other = ExpressionSpans::new(span(0, 9));
        let foreign = other.push_child(other.root(), span(0, 1)).unwrap();
        let mut spans = ExpressionSpans::new(span(0, 9));
        assert_eq!(
            spans.push_child(foreign, span(0, 1)),
            Err(SpanRefusal::UnknownParent(foreign))
        );
    }

    #[test]
    fn a_path_past_the_tree_reaches_no_span() {
        let mut spans = ExpressionSpans::new(span(0, 5));
        spans.push_child(spans.root(), span(0, 1)).unwrap();
        assert_eq!(spans.at(&[]), Some(span(0, 5)));
        assert_eq!(spans.at(&[0]), Some(span(0, 1)));
        assert_eq!(spans.at(&[1]), None);
        assert_eq!(spans.at(&[0, 0]), None);
    }

    /// A declaration's spans must have its body's and its measure's shape:
    /// a body span tree missing a node, or a measure span with no measure,
    /// is refused rather than carried to resolve a path to the wrong node.
    #[test]
    fn spans_that_do_not_fit_the_form_are_refused() {
        use crate::{BuiltinType, FunctionDeclaration, TypeForm};

        let declaration = || {
            FunctionDeclaration::new(
                "f",
                Vec::new(),
                TypeForm::builtin(BuiltinType::Boolean, span(0, 0)),
                None,
                Expression::Not(Box::new(Expression::Boolean(true))),
            )
        };
        let mut body = ExpressionSpans::new(span(0, 8));
        body.push_child(body.root(), span(4, 8)).unwrap();

        let bare = DeclarationSpans {
            declaration: span(0, 8),
            body: ExpressionSpans::new(span(0, 8)),
            measure: None,
        };
        assert_eq!(
            declaration().with_spans(bare).err(),
            Some(SpansMismatch::Body)
        );
        let stray_measure = DeclarationSpans {
            declaration: span(0, 8),
            body: body.clone(),
            measure: Some(ExpressionSpans::new(span(0, 1))),
        };
        assert_eq!(
            declaration().with_spans(stray_measure).err(),
            Some(SpansMismatch::Measure)
        );
        let fitting = DeclarationSpans {
            declaration: span(0, 8),
            body,
            measure: None,
        };
        let carried = declaration().with_spans(fitting.clone()).unwrap();
        assert_eq!(carried.spans(), Some(&fitting));
    }

    /// A chain far deeper than any recursion would survive is built,
    /// walked, shape-checked and dropped without overflowing the stack.
    #[test]
    fn a_deep_chain_is_walked_and_dropped_without_recursion() {
        const DEPTH: usize = 200_000;
        let mut spans = ExpressionSpans::new(span(0, DEPTH));
        let mut node = spans.root();
        for _ in 0..DEPTH {
            node = spans.push_child(node, span(0, DEPTH)).unwrap();
        }
        let path = vec![0; DEPTH];
        assert_eq!(spans.at(&path), Some(span(0, DEPTH)));
        assert!(!spans.fits(&Expression::Boolean(true)));
    }
}
