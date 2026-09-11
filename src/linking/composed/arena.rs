// SPDX-License-Identifier: AGPL-3.0-only
//! FR-036: bounded lookup of a declaration's contiguous parser-arena region.

use super::binding_work::{Dimension, Exhaustion, Work};
use crate::Span;

/// Parser arenas are postorder within each source-ordered declaration (the same
/// invariant used by dependencies::declaration_at). Nodes inside a declaration
/// need not be source-ordered, but every node of an earlier declaration precedes
/// every node of a later one. A declaration boundary therefore partitions the
/// arena. Binary lookup avoids rescanning other declarations and needs no copied
/// AST or separately allocated ownership index. Charge every inspected node.
pub(super) fn owned<'a, T>(
    nodes: &'a [T],
    owner: Span,
    span: impl Fn(&T) -> Span,
    work: &mut Work,
) -> Result<&'a [T], Exhaustion> {
    let mut boundary = |at| -> Result<usize, Exhaustion> {
        let mut low = 0;
        let mut high = nodes.len();
        while low < high {
            work.charge(Dimension::References, 1)?;
            let middle = low + (high - low) / 2;
            if span(&nodes[middle]).start < at {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        Ok(low)
    };
    let start = boundary(owner.start)?;
    let end = boundary(owner.end)?;
    Ok(&nodes[start..end])
}
