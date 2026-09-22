// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-035/036/040: declaration ownership shared by binding and type admission.

use qsl_foundation::Span;
use std::ops::Range;

/// Parser nodes are contiguous by declaration, with every node contained in its
/// owner's span. Inside that region nodes may be postorder rather than sorted by
/// source position. Only declaration boundaries partition the arena monotonically.
/// The real-parser controls in linking::composed::arena test this invariant for
/// value, temporal and control arenas through the binding adapter to this helper.
/// Each caller charges its own work dimension before an inspected node is read.
pub(crate) fn owned_range<T, E>(
    nodes: &[T],
    owner: Span,
    span: impl Fn(&T) -> Span,
    mut inspect: impl FnMut() -> Result<(), E>,
) -> Result<Range<usize>, E> {
    let mut boundary = |at| -> Result<usize, E> {
        let (mut low, mut high) = (0, nodes.len());
        while low < high {
            inspect()?;
            let middle = low + (high - low) / 2;
            if span(&nodes[middle]).start < at {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        Ok(low)
    };
    Ok(boundary(owner.start)?..boundary(owner.end)?)
}
