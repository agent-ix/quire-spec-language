// SPDX-License-Identifier: AGPL-3.0-or-later
//! O-07/T-5 source occurrence identity: `Origin` and `Location`.
//!
//! ADR-013 T-5 states the kernel provenance shape plainly: `Origin`/
//! `Location` "names a node id and occurrence key and carries no bytes".
//! ADR-013 O-07 gives the occurrence key itself: `(NodeKey, role, ordinal)`.
//!
//! This is a **fresh design**, not a port. The existing QSL type of the same
//! name (`value::expression::refusal::{Origin, Location}`) is an
//! expression-tree path used to report where inside an expression evaluation
//! went wrong; it is unrelated to this one and stays in `value::expression`.
//! This module's `Origin`/`Location` are the kernel provenance pair O-07/T-5
//! actually describe: which source occurrence of which checked node a value
//! or outcome traces back to.
//!
//! `Role` is a lexical-string newtype rather than a closed enum: the set of
//! occurrence roles ("declaration", "reference", "default", ...) is QSL's
//! preimage schema's to define and extend, and the kernel takes no position
//! on which roles exist -- it only carries the role QSL attached.
//!
//! **M-3: `Role`'s `String` has no length bound.** This is deliberate, not
//! an oversight left alongside the other `pub(crate)`-or-metered H-5
//! findings: a role's spelling is a small, fixed vocabulary word that
//! QSL's own preimage schema names at check time (`node-identity-
//! preimage.schema.json`), not a value materialized from caller-supplied,
//! runtime-metered evaluation input the way a `Decimal`'s scale or a
//! `Text`'s bytes are. Nothing on any evaluation path constructs a `Role`
//! from adversarial input, so it takes no charge and needs no bound the way
//! [`crate::Meter`]-charged materializations do.

use std::fmt;

use crate::node::NodeKey;

/// A source occurrence's role, as QSL's preimage schema names it (e.g.
/// `"declaration"`, `"reference"`). The kernel does not enumerate roles; it
/// carries whichever lexical role string QSL attached to the occurrence.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Role(String);

impl Role {
    /// Wrap an already-known role string.
    pub fn new(role: impl Into<String>) -> Self {
        Self(role.into())
    }

    /// The role's lexical spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Role {
    fn from(role: &str) -> Self {
        Self::new(role)
    }
}

impl From<String> for Role {
    fn from(role: String) -> Self {
        Self::new(role)
    }
}

/// A source occurrence key within one checked node (ADR-013 O-07): a role
/// and an ordinal disambiguating repeated occurrences of that role on the
/// same node (e.g. the third `"reference"` occurrence).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Origin {
    role: Role,
    ordinal: u64,
}

impl Origin {
    /// Name a source occurrence by role and ordinal.
    pub fn new(role: Role, ordinal: u64) -> Self {
        Self { role, ordinal }
    }

    /// The occurrence's role.
    pub fn role(&self) -> &Role {
        &self.role
    }

    /// The occurrence's ordinal, disambiguating repeated occurrences of the
    /// same role on the same node.
    pub fn ordinal(&self) -> u64 {
        self.ordinal
    }
}

/// A full source location (ADR-013 T-5): which checked node, and which
/// occurrence within it. Names a node id and an occurrence key; carries no
/// bytes of its own.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Location {
    node: NodeKey,
    occurrence: Origin,
}

impl Location {
    /// Name a location by node and occurrence.
    pub fn new(node: NodeKey, occurrence: Origin) -> Self {
        Self { node, occurrence }
    }

    /// The located node.
    pub fn node(&self) -> NodeKey {
        self.node
    }

    /// The occurrence within that node.
    pub fn occurrence(&self) -> &Origin {
        &self.occurrence
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn digest(byte: u8) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        bytes[31] = byte;
        bytes
    }

    /// TC-305: two locations naming the same node and the same
    /// `(role, ordinal)` occurrence are equal; a different ordinal makes
    /// them distinct (ADR-013 O-07).
    #[trace("TC-305")]
    #[test]
    fn tc_305_location_equality_follows_node_and_occurrence() {
        let node = NodeKey::from_digest(digest(1));
        let a = Location::new(node, Origin::new(Role::from("declaration"), 0));
        let b = Location::new(node, Origin::new(Role::from("declaration"), 0));
        let c = Location::new(node, Origin::new(Role::from("declaration"), 1));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    /// TC-306 (H-7/H-8, strengthened): `Origin`'s derived `Ord` orders
    /// first by role, then by ordinal within the same role -- the field
    /// order the derive relies on, exercised rather than merely round-
    /// tripped through the accessors.
    #[trace("TC-306")]
    #[test]
    fn tc_306_origin_orders_by_role_then_ordinal() {
        let declaration_0 = Origin::new(Role::from("declaration"), 0);
        let declaration_1 = Origin::new(Role::from("declaration"), 1);
        let reference_0 = Origin::new(Role::from("reference"), 0);
        assert!(declaration_0 < declaration_1);
        assert!(declaration_1 < reference_0);
        assert_eq!(reference_0.role().as_str(), "reference");
        assert_eq!(reference_0.ordinal(), 0);
    }
}
