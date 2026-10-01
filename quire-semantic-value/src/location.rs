// SPDX-License-Identifier: AGPL-3.0-or-later
//! Expression locations (ADR-011 §6.1 layer SV): the declaration an
//! expression belongs to ([`Origin`]) and the child-index path from that
//! declaration's root expression ([`Location`]). A checking refusal, an
//! evaluation outcome and a loss record are all located this way.
//!
//! These are not `quire_exact::{Origin, Location}`: those name a checked
//! node's source occurrence (ADR-013 O-07/T-5). These name a position
//! inside a package's expression trees.

use alloc::string::String;
use alloc::vec::Vec;

/// The declaration a location belongs to.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Origin {
    /// The body of the named function, at this zero-based declaration index.
    Body {
        /// The declared name.
        function: String,
        /// The declaration index in the package.
        index: usize,
    },
    /// The `decreases` measure of the named function.
    Measure {
        /// The declared name.
        function: String,
        /// The declaration index in the package.
        index: usize,
    },
    /// A standalone checked expression.
    Expression,
    /// A declared record, tuple or enum type, by its declared name. Its
    /// `declaration` occurrence is located here (FR-322), and so is the
    /// `generated` occurrence of a node that no function body, measure,
    /// state clause or protocol attempt reaches, when this is the least
    /// declared name that reaches it (`lowering::enclosing_declarations`). It
    /// names the declared name's region when the FR-091 assembler read the
    /// name from the unit, and no region for a type declared by hand
    /// (FR-096).
    TypeDeclaration {
        /// The declared name.
        name: String,
    },
    /// The body of the named state clause (FR-104), at this zero-based
    /// index among the unit's state clauses in source order. The clause's
    /// `claim` occurrence is located at its root.
    StateClause {
        /// The declared clause name.
        clause: String,
        /// The clause's index among the package's state clauses.
        index: usize,
    },
    /// One protocol `attempt`'s own operation binding (FR-114), by
    /// the protocol's index among the package's protocols and the
    /// attempt's index among that protocol's own `attempts`, both in
    /// source order. An `operation_anchor`/`frame` node names no position
    /// of its own, so this names a real position of the unit (the
    /// attempt's own declared name) to resolve its generated occurrence's
    /// region, the same way `Origin::StateClause` does for a `pre`/`post`
    /// clause's own anchor (FR-096).
    ProtocolAttempt {
        /// The protocol's index among the package's protocols.
        protocol: usize,
        /// The attempt's index among the protocol's own `attempts`.
        attempt: usize,
    },
}

/// A located expression: its declaration and the child-index path from that
/// declaration's root expression.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Location {
    /// The owning declaration.
    pub origin: Origin,
    /// Child indices from the root, as the parsed form's
    /// `Expression::children` numbers them.
    pub path: Vec<usize>,
}

impl Location {
    /// A child of this location.
    pub fn child(&self, index: usize) -> Self {
        let mut path = self.path.clone();
        path.push(index);
        Self {
            origin: self.origin.clone(),
            path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// A child keeps its origin and extends the path by one index, leaving
    /// the parent unchanged.
    #[test]
    fn child_extends_the_path_and_keeps_the_origin() {
        let parent = Location {
            origin: Origin::Body {
                function: String::from("f"),
                index: 2,
            },
            path: vec![1],
        };
        let child = parent.child(3);
        assert_eq!(child.origin, parent.origin);
        assert_eq!(child.path, vec![1, 3]);
        assert_eq!(parent.path, vec![1]);
    }
}
