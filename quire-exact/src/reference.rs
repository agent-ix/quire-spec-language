// SPDX-License-Identifier: AGPL-3.0-or-later
//! O-13/T-6 terminal `Reference<T>` values.
//!
//! A reference is terminal: its identity is the triple ADR-013 T-6 gives the
//! kernel -- `EffectiveId` (the object type), `UniverseId` and `ObjectId` --
//! and equality never inspects referenced state. This is a fresh cut, not a
//! verbatim port, of QSL `value::reference`: the original `ObjectReference`
//! carries a `NodeKey` object type plus raw `UniverseIdentity`/
//! `ObjectIdentity` byte strings; ADR-013 T-6 replaces all three with opaque
//! digest ids QSL mints from its own preimage (ADR-013 QC-15). The original
//! `ObjectEnvironment` (a closed reference graph checked against a
//! `TypeEnvironment`) is dropped entirely: it is a declaration-registry
//! concern and stays layer 3 in QSL per ADR-013 O-15, not the kernel.

use crate::identity::{EffectiveId, ObjectId, UniverseId};

/// A `Reference<T>` value: its identity triple (ADR-013 T-6).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectReference {
    universe: UniverseId,
    object_type: EffectiveId,
    object: ObjectId,
}

impl ObjectReference {
    /// The reference `(universe, object_type, object)` supplied by a bound
    /// model snapshot.
    pub fn new(universe: UniverseId, object_type: EffectiveId, object: ObjectId) -> Self {
        Self {
            universe,
            object_type,
            object,
        }
    }

    /// The universe identity.
    pub fn universe(&self) -> UniverseId {
        self.universe
    }

    /// The object-type effective identity.
    pub fn object_type(&self) -> EffectiveId {
        self.object_type
    }

    /// The object identity.
    pub fn object(&self) -> ObjectId {
        self.object
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

    /// TC-316: two references built from the same triple are equal; changing
    /// any one component of the triple makes them distinct (ADR-013 T-6:
    /// equality never inspects referenced state -- only the triple).
    #[trace("TC-316")]
    #[test]
    fn tc_316_reference_equality_follows_the_identity_triple() {
        let universe = UniverseId::from_digest(digest(1));
        let object_type = EffectiveId::from_digest(digest(2));
        let object = ObjectId::from_digest(digest(3));
        let a = ObjectReference::new(universe, object_type, object);
        let b = ObjectReference::new(universe, object_type, object);
        let c = ObjectReference::new(universe, object_type, ObjectId::from_digest(digest(4)));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
