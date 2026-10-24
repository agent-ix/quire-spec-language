// SPDX-License-Identifier: AGPL-3.0-or-later
//! The one refusal type of the fairness kernels.

use quire_exact::Cancel;
use thiserror::Error;

use crate::fairness::graph::Vertex;

/// Why a fairness kernel refused its input or stopped.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum FairnessError {
    /// A vertex is not in the graph.
    #[error("vertex {vertex:?} is not in the graph")]
    UnknownVertex {
        /// The vertex asked for.
        vertex: Vertex,
    },
    /// The caller's `Cancel` handle was cancelled.
    #[error("the run was cancelled")]
    Cancelled,
    /// A lasso's loop has no step.
    #[error("a lasso's loop has no step")]
    EmptyLoop,
}

/// Charge one unit of work against the caller's handle, before the work.
pub(crate) fn charge(cancel: &Cancel) -> Result<(), FairnessError> {
    if cancel.poll() {
        Err(FairnessError::Cancelled)
    } else {
        Ok(())
    }
}
