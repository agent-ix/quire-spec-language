// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local evaluator adapter for qsl-package's admitted reader projection.

use quire_checked::node::{CheckedLiteral, NodeKind};
use quire_exact::{Value, ValueType};
use quire_semantic_value::location::Location;

use super::node_source::{Callee, NodeSource};

/// One node handle in the package-owned admitted projection.
#[derive(Clone, Copy)]
pub(crate) struct ReaderNode<'arena> {
    projection: &'arena qsl_package::ReaderProtocolProjection,
    index: usize,
}

impl<'arena> ReaderNode<'arena> {
    pub(crate) fn root(projection: &'arena qsl_package::ReaderProtocolProjection) -> Self {
        Self {
            projection,
            index: projection.root(),
        }
    }
}

impl<'arena> NodeSource<'arena> for ReaderNode<'arena> {
    type Child = usize;
    type Literal = CheckedLiteral;

    fn kind(self) -> &'arena NodeKind<Self::Child, Self::Literal> {
        self.projection
            .kind(self.index)
            .expect("projection node index is valid")
    }

    fn literal(literal: &'arena Self::Literal) -> &'arena Value {
        &literal.0
    }

    fn at(self, child: Self::Child) -> Self {
        Self {
            projection: self.projection,
            index: child,
        }
    }

    fn value_type(self) -> &'arena ValueType {
        self.projection
            .value_type(self.index)
            .expect("projection node index is valid")
    }

    fn location(self) -> Option<&'arena Location> {
        self.projection.location(self.index)
    }

    fn is_ordering_part(self) -> bool {
        false
    }

    fn children(self) -> Vec<Self> {
        self.projection
            .children(self.index)
            .expect("projection node index is valid")
            .iter()
            .copied()
            .map(|index| Self {
                projection: self.projection,
                index,
            })
            .collect()
    }

    fn callee(
        self,
        _graph: &'arena qsl_semantics::check::CheckedGraph,
        function: usize,
    ) -> Option<Callee<'arena, Self>> {
        let function = self.projection.function(function)?;
        Some(Callee {
            name: function.name,
            body: Self {
                projection: self.projection,
                index: function.body,
            },
            slots: function.slots,
        })
    }
}
