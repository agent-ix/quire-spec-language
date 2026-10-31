// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 O-07 and O-12: source provenance. The occurrence key
//! (`node id`, `role`, `ordinal`), the source region (`RawSourceRef`, byte
//! start, byte end) and the package source map that keys the one by the
//! other -- exactly FR-322's `source_map` (`SourceMapEntry`: `node_id`,
//! `role`, `ordinal`, `regions`).
//!
//! The map is keyed by the admitted structural node identity. A kernel
//! [`quire_exact::Location`] resolves through the map by the same node and
//! occurrence, with no digest conversion ([`OccurrenceKey::from`]).
//!
//! Admission of the wire's own map (every occurrence mapped, every region
//! under a locked, current source, no overlap within one occurrence and
//! document) is IR's `checked_package` reader's job. This module holds the
//! admitted map and checks only what its own shape needs: a key maps once,
//! to at least one region.

use alloc::collections::btree_map::Entry;
use alloc::collections::BTreeMap;

use alloc::{boxed::Box, string::String, vec::Vec};
use core::fmt;
use hashbrown::HashMap;

use quire_exact::{Location, NodeKey, Origin, Role};

/// Why a provenance value could not be formed. Each condition a caller must
/// tell apart is its own variant.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum InvalidProvenance {
    /// A [`RawSourceRef`]'s authority is empty (QSpec `Nonempty`).
    #[error("the source authority is empty")]
    EmptyAuthority,
    /// A [`RawSourceRef`]'s identity is empty (QSpec `Nonempty`).
    #[error("the source identity is empty")]
    EmptyIdentity,
    /// A [`SourceRegion`]'s end precedes its start.
    #[error("region end {end} precedes start {start}")]
    ReversedRegion {
        /// The offered start.
        start: u64,
        /// The offered end.
        end: u64,
    },
    /// A source-map entry names no region (FR-322: every occurrence maps to
    /// one or more regions).
    #[error("occurrence {0} names no source region")]
    NoRegion(OccurrenceKey),
    /// Two source-map entries name the same occurrence key.
    #[error("occurrence {0} is mapped twice")]
    DuplicateOccurrence(OccurrenceKey),
}

/// ADR-013 O-07 and QSpec `RawSourceRef`: names one source document by
/// its authority and identity.
///
/// quire:canonical
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RawSourceRef {
    authority: String,
    identity: String,
}

/// QSpec FR-322 `RawSourceRef`, member for member: `authority` and
/// `identity`.
impl serde::Serialize for RawSourceRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut record = serializer.serialize_struct("RawSourceRef", 2)?;
        record.serialize_field("authority", &self.authority)?;
        record.serialize_field("identity", &self.identity)?;
        record.end()
    }
}

impl RawSourceRef {
    /// Name a source document. Refuses an empty authority or identity.
    pub fn new(
        authority: impl Into<String>,
        identity: impl Into<String>,
    ) -> Result<Self, InvalidProvenance> {
        let (authority, identity) = (authority.into(), identity.into());
        if authority.is_empty() {
            return Err(InvalidProvenance::EmptyAuthority);
        }
        if identity.is_empty() {
            return Err(InvalidProvenance::EmptyIdentity);
        }
        Ok(Self {
            authority,
            identity,
        })
    }

    /// The source's authority.
    pub fn authority(&self) -> &str {
        &self.authority
    }

    /// The source's identity within that authority.
    pub fn identity(&self) -> &str {
        &self.identity
    }
}

/// ADR-013 O-07/O-12: a half-open UTF-8 byte interval `[start, end)` under
/// one source document.
///
/// Equality, ordering and hashing are lexical over (`RawSourceRef` authority,
/// identity, start, end): two regions over the same document and offsets are
/// one region.
///
/// An empty region (`start == end`) is a point, which a diagnostic may
/// name. A v2 source-map entry's regions are non-empty; IR's reader
/// enforces that at admission.
#[derive(Clone, Debug)]
pub struct SourceRegion {
    source: RawSourceRef,
    start: u64,
    end: u64,
}

impl SourceRegion {
    /// The region `[start, end)` of `source`. Refuses `end < start`.
    pub fn new(source: RawSourceRef, start: u64, end: u64) -> Result<Self, InvalidProvenance> {
        if end < start {
            return Err(InvalidProvenance::ReversedRegion { start, end });
        }
        Ok(Self { source, start, end })
    }

    /// The source document.
    pub fn source(&self) -> &RawSourceRef {
        &self.source
    }

    /// The inclusive start byte.
    pub fn start(&self) -> u64 {
        self.start
    }

    /// The exclusive end byte.
    pub fn end(&self) -> u64 {
        self.end
    }

    /// The lexical key a region's equality compares.
    fn key(&self) -> (&str, &str, u64, u64) {
        (
            &self.source.authority,
            &self.source.identity,
            self.start,
            self.end,
        )
    }
}

impl PartialEq for SourceRegion {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for SourceRegion {}

impl core::hash::Hash for SourceRegion {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

impl PartialOrd for SourceRegion {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SourceRegion {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

/// ADR-013 O-07: an occurrence key, (node id, role, ordinal), exactly
/// FR-322's `source_map` key. The node half is the admitted node id; the role
/// and ordinal half is the kernel [`Origin`]. Equality and ordering are
/// lexical over (node id, role, ordinal).
///
/// quire:canonical
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OccurrenceKey {
    node: NodeKey,
    origin: Origin,
}

impl OccurrenceKey {
    /// Name an occurrence by structural node id, role and ordinal.
    pub fn new(node: NodeKey, origin: Origin) -> Self {
        Self { node, origin }
    }

    /// The `generated` occurrence of `node` at ordinal 0: the one
    /// occurrence of a node the producer writes with no source construct of
    /// its own, such as a `model_root` node (QSpec FR-322-AC-132).
    pub fn generated(node: NodeKey) -> Self {
        Self::new(node, Origin::new(Role::new("generated"), 0))
    }

    /// The `declaration` occurrence of `node` at ordinal 0, such as a
    /// protocol clause node's (QSpec FR-368 "Claim and anchor").
    pub fn declaration(node: NodeKey) -> Self {
        Self::new(node, Origin::new(Role::new("declaration"), 0))
    }

    /// The occurrence's structural node id.
    pub fn node(&self) -> &NodeKey {
        &self.node
    }

    /// The occurrence's role and ordinal.
    pub fn origin(&self) -> &Origin {
        &self.origin
    }
}

/// A checked location's admitted structural node and occurrence.
impl From<&Location> for OccurrenceKey {
    fn from(location: &Location) -> Self {
        Self::new(location.node().clone(), location.occurrence().clone())
    }
}

impl fmt::Display for OccurrenceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}, {}, {})",
            self.node,
            self.origin.role(),
            self.origin.ordinal()
        )
    }
}

/// Why a location did not resolve through a [`PackageSourceMap`]
/// (ADR-013 O-12: a tag naming no node in the package refuses at replay).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum UnresolvedOccurrence {
    /// The package maps no occurrence of the tag's node.
    #[error("the package names no node {0}")]
    UnknownNode(NodeKey),
    /// The package maps the tag's node, but not at the tag's role and
    /// ordinal.
    #[error("the package maps no occurrence {0}")]
    UnknownOccurrence(OccurrenceKey),
}

/// ADR-013 O-12: one package's source map, keyed by the O-07 occurrence
/// key, each key mapping to its ordered, non-empty regions.
///
/// Entries are grouped by node under a hash of the node's key, so building
/// and looking up the map never orders two node keys: ordering keys that
/// share a deep chain of nested operands costs that chain's depth.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageSourceMap {
    entries: HashMap<NodeKey, NodeOccurrences>,
}

/// One node's mapped occurrences, in (role, ordinal) order, each with its
/// full key and its regions.
type NodeOccurrences = BTreeMap<Origin, (OccurrenceKey, Box<[SourceRegion]>)>;

impl PackageSourceMap {
    /// The map of `entries`, each an occurrence key and its regions in
    /// wire order. Refuses an entry with no region and a key mapped twice.
    pub fn from_entries(
        entries: impl IntoIterator<Item = (OccurrenceKey, Vec<SourceRegion>)>,
    ) -> Result<Self, InvalidProvenance> {
        let mut map: HashMap<NodeKey, NodeOccurrences> = HashMap::new();
        for (key, regions) in entries {
            if regions.is_empty() {
                return Err(InvalidProvenance::NoRegion(key));
            }
            match map
                .entry(key.node.clone())
                .or_default()
                .entry(key.origin.clone())
            {
                Entry::Occupied(_) => return Err(InvalidProvenance::DuplicateOccurrence(key)),
                Entry::Vacant(vacant) => {
                    vacant.insert((key, regions.into_boxed_slice()));
                }
            }
        }
        Ok(Self { entries: map })
    }

    /// ADR-013 C-14: the regions of one occurrence, in wire order, or
    /// `None` when the package maps no such occurrence.
    pub fn regions(&self, key: &OccurrenceKey) -> Option<&[SourceRegion]> {
        self.entries
            .get(&key.node)?
            .get(&key.origin)
            .map(|(_, regions)| &**regions)
    }

    /// ADR-013 O-07: every occurrence of `node`, in (role, ordinal) order,
    /// each with its regions.
    pub fn occurrences(
        &self,
        node: &NodeKey,
    ) -> impl Iterator<Item = (&OccurrenceKey, &[SourceRegion])> {
        self.entries
            .get(node)
            .into_iter()
            .flat_map(BTreeMap::values)
            .map(|(key, regions)| (key, &**regions))
    }

    /// ADR-013 O-12: the regions a checked location tag names. Refuses a
    /// tag whose node the package does not map, and one whose node it maps
    /// at no such role and ordinal.
    pub fn resolve(&self, location: &Location) -> Result<&[SourceRegion], UnresolvedOccurrence> {
        let key = OccurrenceKey::from(location);
        if let Some(regions) = self.regions(&key) {
            return Ok(regions);
        }
        if self.occurrences(&key.node).next().is_none() {
            Err(UnresolvedOccurrence::UnknownNode(key.node))
        } else {
            Err(UnresolvedOccurrence::UnknownOccurrence(key))
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};
    use ix_trace_rs::trace;
    use quire_exact::PathSegment;

    use super::*;

    fn source(authority: &str) -> RawSourceRef {
        RawSourceRef::new(authority, "unit").unwrap()
    }

    fn region(authority: &str, start: u64, end: u64) -> SourceRegion {
        SourceRegion::new(source(authority), start, end).unwrap()
    }

    fn node(value: u8) -> NodeKey {
        NodeKey::new("test", vec![PathSegment::Text(value.to_string())], 0)
            .expect("test node has a valid structural identity")
    }

    fn key(node: u8, role: &str, ordinal: u64) -> OccurrenceKey {
        OccurrenceKey::new(self::node(node), Origin::new(Role::new(role), ordinal))
    }

    /// A region's equality is lexical over (authority, identity, start,
    /// end): each of the four compared members separates two regions.
    #[trace("TC-420", "FR-095-AC-2")]
    #[test]
    fn region_equality_is_over_source_start_and_end() {
        let base = region("a", 4, 9);
        assert_eq!(base, region("a", 4, 9));
        assert_eq!(base.cmp(&region("a", 4, 9)), core::cmp::Ordering::Equal);
        assert_ne!(base, region("b", 4, 9));
        assert_ne!(
            base,
            SourceRegion::new(RawSourceRef::new("a", "other").unwrap(), 4, 9).unwrap()
        );
        assert_ne!(base, region("a", 5, 9));
        assert_ne!(base, region("a", 4, 8));
    }

    /// A `RawSourceRef` is QSpec's: non-empty authority and identity
    /// members. A region's end never precedes its start; an empty region is a
    /// point.
    #[trace("TC-420", "FR-095-AC-1")]
    #[test]
    fn raw_source_refs_and_regions_refuse_malformed_members() {
        assert_eq!(
            RawSourceRef::new("", "u"),
            Err(InvalidProvenance::EmptyAuthority)
        );
        assert_eq!(
            RawSourceRef::new("a", ""),
            Err(InvalidProvenance::EmptyIdentity)
        );
        assert_eq!(
            SourceRegion::new(source("a"), 5, 4).unwrap_err(),
            InvalidProvenance::ReversedRegion { start: 5, end: 4 }
        );
        let point = SourceRegion::new(source("a"), 5, 5).unwrap();
        assert_eq!((point.start(), point.end()), (5, 5));
    }

    /// O-07 key equality is lexical over (node id, role, ordinal): each
    /// member alone separates two keys.
    #[trace("TC-420", "FR-095-AC-1")]
    #[test]
    fn occurrence_keys_compare_over_node_role_and_ordinal() {
        assert_eq!(key(1, "expression", 0), key(1, "expression", 0));
        assert_ne!(key(1, "expression", 0), key(2, "expression", 0));
        let same_path_other_package =
            NodeKey::new("other", vec![PathSegment::Text("1".into())], 0).unwrap();
        assert_ne!(
            key(1, "expression", 0),
            OccurrenceKey::new(
                same_path_other_package,
                Origin::new(Role::new("expression"), 0)
            )
        );
        let same_path_other_ordinal =
            NodeKey::new("test", vec![PathSegment::Text("1".into())], 1).unwrap();
        assert_ne!(
            key(1, "expression", 0),
            OccurrenceKey::new(
                same_path_other_ordinal,
                Origin::new(Role::new("expression"), 0)
            )
        );
        assert_ne!(key(1, "expression", 0), key(1, "type", 0));
        assert_ne!(key(1, "expression", 0), key(1, "expression", 1));
    }

    fn map() -> PackageSourceMap {
        PackageSourceMap::from_entries([
            (key(1, "declaration", 0), vec![region("a", 0, 4)]),
            (
                key(1, "expression", 0),
                vec![region("a", 10, 12), region("a", 20, 22)],
            ),
            (key(1, "expression", 1), vec![region("a", 30, 31)]),
            (key(2, "type", 0), vec![region("a", 40, 41)]),
        ])
        .unwrap()
    }

    /// C-14 and O-07: an occurrence key maps to its regions in wire order;
    /// a node's occurrences are exactly its own keys, in (role, ordinal)
    /// order.
    #[trace("TC-421", "FR-095-AC-3")]
    #[test]
    fn keys_map_to_their_regions_and_nodes_to_their_occurrences() {
        let map = map();
        assert_eq!(
            map.regions(&key(1, "expression", 0)),
            Some(&[region("a", 10, 12), region("a", 20, 22)][..])
        );
        assert_eq!(
            map.regions(&key(1, "expression", 1)),
            Some(&[region("a", 30, 31)][..])
        );
        assert_eq!(map.regions(&key(1, "expression", 2)), None);
        let node_one: Vec<_> = map
            .occurrences(&node(1))
            .map(|(key, _)| key.clone())
            .collect();
        assert_eq!(
            node_one,
            [
                key(1, "declaration", 0),
                key(1, "expression", 0),
                key(1, "expression", 1)
            ]
        );
        assert_eq!(map.occurrences(&node(3)).count(), 0);
    }

    /// O-12: a location tag resolves through the map by its node id and
    /// occurrence; a tag naming no node of the package, or an occurrence
    /// the package does not map, refuses with its own cause.
    #[trace("TC-421", "FR-095-AC-4")]
    #[test]
    fn a_location_resolves_or_refuses_by_cause() {
        let map = map();
        let location = |node: u8, role: &str, ordinal| {
            Location::new(self::node(node), Origin::new(Role::new(role), ordinal))
        };
        assert_eq!(
            map.resolve(&location(2, "type", 0)),
            Ok(&[region("a", 40, 41)][..])
        );
        assert_eq!(
            map.resolve(&location(3, "type", 0)),
            Err(UnresolvedOccurrence::UnknownNode(node(3)))
        );
        assert_eq!(
            map.resolve(&location(2, "type", 1)),
            Err(UnresolvedOccurrence::UnknownOccurrence(key(2, "type", 1)))
        );
    }

    /// The map's own shape: every key maps once, to at least one region.
    #[trace("TC-421", "FR-095-AC-3")]
    #[test]
    fn the_map_refuses_an_empty_or_repeated_entry() {
        assert_eq!(
            PackageSourceMap::from_entries([(key(1, "type", 0), Vec::new())]),
            Err(InvalidProvenance::NoRegion(key(1, "type", 0)))
        );
        assert_eq!(
            PackageSourceMap::from_entries([
                (key(1, "type", 0), vec![region("a", 0, 1)]),
                (key(1, "type", 0), vec![region("a", 2, 3)]),
            ]),
            Err(InvalidProvenance::DuplicateOccurrence(key(1, "type", 0)))
        );
    }
}
