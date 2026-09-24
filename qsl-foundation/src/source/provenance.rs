// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 O-07 and O-12: source provenance. The occurrence key
//! (`node id`, `role`, `ordinal`), the source region (`RawSourceRef`, byte
//! start, byte end) and the package source map that keys the one by the
//! other -- exactly FR-322's `source_map` (`SourceMapEntry`: `node_id`,
//! `role`, `ordinal`, `regions`).
//!
//! The map is keyed by the wire node id ([`WireNodeId`]): it is read from
//! `quire.checked-package/v2` bytes, and a node id read from a wire never
//! becomes a `NodeKey` by conversion (R-10). A kernel
//! [`quire_exact::Location`] (the checked occurrence tag, which carries no
//! bytes) resolves through the map in the other, emission, direction
//! ([`OccurrenceKey::from`]).
//!
//! Admission of the wire's own map (every occurrence mapped, every region
//! under a locked, current source, no overlap within one occurrence and
//! document) is IR's `checked_package` reader's job. This module holds the
//! admitted map and checks only what its own shape needs: a key maps once,
//! to at least one region.

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::fmt;

use quire_exact::{Location, Origin, Role};

use crate::digest::{DigestDomain, DigestRecord, WireNodeId};

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
    /// A [`Revision`]'s namespace is empty (QSpec `Nonempty`).
    #[error("the revision namespace is empty")]
    EmptyRevisionNamespace,
    /// A [`Revision`]'s value is empty (QSpec `Nonempty`).
    #[error("the revision value is empty")]
    EmptyRevisionValue,
    /// A [`RawSourceRef`]'s digest is not a `quire.source.bytes/v1` digest
    /// (QSpec `RawSourceRef.digest_domain` is that constant).
    #[error("a raw source digest must be quire.source.bytes/v1, not {0}")]
    NotSourceBytes(DigestDomain),
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

/// QSpec `Revision`: a revision value in a stable namespace.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Revision {
    namespace: String,
    value: String,
}

impl Revision {
    /// A revision `value` in `namespace`. Refuses an empty member.
    pub fn new(
        namespace: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, InvalidProvenance> {
        let (namespace, value) = (namespace.into(), value.into());
        if namespace.is_empty() {
            return Err(InvalidProvenance::EmptyRevisionNamespace);
        }
        if value.is_empty() {
            return Err(InvalidProvenance::EmptyRevisionValue);
        }
        Ok(Self { namespace, value })
    }

    /// The revision namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The revision value.
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// ADR-013 O-07 and QSpec `RawSourceRef`: names one source document by
/// authority, identity, revision and its `quire.source.bytes/v1` digest.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RawSourceRef {
    authority: String,
    identity: String,
    revision: Revision,
    digest: DigestRecord,
}

impl RawSourceRef {
    /// Name a source document. Refuses an empty authority or identity, and
    /// a digest outside `quire.source.bytes/v1`.
    pub fn new(
        authority: impl Into<String>,
        identity: impl Into<String>,
        revision: Revision,
        digest: DigestRecord,
    ) -> Result<Self, InvalidProvenance> {
        let (authority, identity) = (authority.into(), identity.into());
        if authority.is_empty() {
            return Err(InvalidProvenance::EmptyAuthority);
        }
        if identity.is_empty() {
            return Err(InvalidProvenance::EmptyIdentity);
        }
        if digest.domain() != DigestDomain::SourceBytesV1 {
            return Err(InvalidProvenance::NotSourceBytes(digest.domain()));
        }
        Ok(Self {
            authority,
            identity,
            revision,
            digest,
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

    /// The selected revision of that identity.
    pub fn revision(&self) -> &Revision {
        &self.revision
    }

    /// The `quire.source.bytes/v1` digest of the source's bytes.
    pub fn digest(&self) -> DigestRecord {
        self.digest
    }
}

/// ADR-013 O-07/O-12: a half-open UTF-8 byte interval `[start, end)` under
/// one source document.
///
/// Equality, ordering and hashing are lexical over (`RawSourceRef` digest,
/// start, end), exactly O-12's region equality: the document's digest names
/// its bytes, so two regions over the same bytes and offsets are one region
/// whatever authority or revision label named the document.
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

    /// The lexical key O-12's equality compares.
    fn key(&self) -> (DigestRecord, u64, u64) {
        (self.source.digest, self.start, self.end)
    }
}

impl PartialEq for SourceRegion {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for SourceRegion {}

impl std::hash::Hash for SourceRegion {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

impl PartialOrd for SourceRegion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SourceRegion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

/// ADR-013 O-07: an occurrence key, (node id, role, ordinal), exactly
/// FR-322's `source_map` key. The node half is the wire node id; the role
/// and ordinal half is the kernel [`Origin`]. Equality and ordering are
/// lexical over (node id, role, ordinal).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OccurrenceKey {
    node: WireNodeId,
    origin: Origin,
}

impl OccurrenceKey {
    /// Name an occurrence by wire node id, role and ordinal.
    pub fn new(node: WireNodeId, origin: Origin) -> Self {
        Self { node, origin }
    }

    /// The occurrence's wire node id.
    pub fn node(&self) -> WireNodeId {
        self.node
    }

    /// The occurrence's role and ordinal.
    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    /// The first key of `node` in key order: the empty role sorts before
    /// every role, and ordinal zero before every ordinal.
    fn first_of(node: WireNodeId) -> Self {
        Self::new(node, Origin::new(Role::new(""), 0))
    }
}

/// The emission direction: a checked location's node id, as the wire
/// spells it. The reverse is never defined (R-10).
impl From<&Location> for OccurrenceKey {
    fn from(location: &Location) -> Self {
        Self::new(
            WireNodeId::from_digest(*location.node().as_bytes()),
            location.occurrence().clone(),
        )
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
    UnknownNode(WireNodeId),
    /// The package maps the tag's node, but not at the tag's role and
    /// ordinal.
    #[error("the package maps no occurrence {0}")]
    UnknownOccurrence(OccurrenceKey),
}

/// ADR-013 O-12: one package's source map, keyed by the O-07 occurrence
/// key, each key mapping to its ordered, non-empty regions.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageSourceMap {
    entries: BTreeMap<OccurrenceKey, Box<[SourceRegion]>>,
}

impl PackageSourceMap {
    /// The map of `entries`, each an occurrence key and its regions in
    /// wire order. Refuses an entry with no region and a key mapped twice.
    pub fn from_entries(
        entries: impl IntoIterator<Item = (OccurrenceKey, Vec<SourceRegion>)>,
    ) -> Result<Self, InvalidProvenance> {
        let mut map = BTreeMap::new();
        for (key, regions) in entries {
            if regions.is_empty() {
                return Err(InvalidProvenance::NoRegion(key));
            }
            match map.entry(key) {
                Entry::Occupied(occupied) => {
                    return Err(InvalidProvenance::DuplicateOccurrence(
                        occupied.key().clone(),
                    ))
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(regions.into_boxed_slice());
                }
            }
        }
        Ok(Self { entries: map })
    }

    /// ADR-013 C-14: the regions of one occurrence, in wire order, or
    /// `None` when the package maps no such occurrence.
    pub fn regions(&self, key: &OccurrenceKey) -> Option<&[SourceRegion]> {
        self.entries.get(key).map(|regions| &**regions)
    }

    /// ADR-013 O-07: every occurrence of `node`, in (role, ordinal) order,
    /// each with its regions.
    pub fn occurrences(
        &self,
        node: WireNodeId,
    ) -> impl Iterator<Item = (&OccurrenceKey, &[SourceRegion])> {
        self.entries
            .range(OccurrenceKey::first_of(node)..)
            .take_while(move |(key, _)| key.node == node)
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
        if self.occurrences(key.node).next().is_none() {
            Err(UnresolvedOccurrence::UnknownNode(key.node))
        } else {
            Err(UnresolvedOccurrence::UnknownOccurrence(key))
        }
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::NodeKey;

    use super::*;

    fn digest(domain: DigestDomain, fill: u8) -> DigestRecord {
        DigestRecord::mint(domain, [fill; 32])
    }

    fn source(authority: &str, fill: u8) -> RawSourceRef {
        RawSourceRef::new(
            authority,
            "unit",
            Revision::new("git", "abc").unwrap(),
            digest(DigestDomain::SourceBytesV1, fill),
        )
        .unwrap()
    }

    fn region(fill: u8, start: u64, end: u64) -> SourceRegion {
        SourceRegion::new(source("a", fill), start, end).unwrap()
    }

    fn key(node: u8, role: &str, ordinal: u64) -> OccurrenceKey {
        OccurrenceKey::new(
            WireNodeId::from_digest([node; 32]),
            Origin::new(Role::new(role), ordinal),
        )
    }

    /// O-12 region equality is lexical over (digest, start, end): the
    /// authority, identity and revision labels take no part, while each of
    /// the three compared members does.
    #[trace("TC-420", "FR-095-AC-2")]
    #[test]
    fn region_equality_is_over_digest_start_and_end_only() {
        let base = region(1, 4, 9);
        let relabelled = SourceRegion::new(
            RawSourceRef::new(
                "b",
                "other",
                Revision::new("semver", "2").unwrap(),
                digest(DigestDomain::SourceBytesV1, 1),
            )
            .unwrap(),
            4,
            9,
        )
        .unwrap();
        assert_eq!(base, relabelled);
        assert_eq!(base.cmp(&relabelled), std::cmp::Ordering::Equal);
        assert_ne!(base, region(2, 4, 9));
        assert_ne!(base, region(1, 5, 9));
        assert_ne!(base, region(1, 4, 8));
    }

    /// A `RawSourceRef` is QSpec's: non-empty authority, identity and
    /// revision members, and a `quire.source.bytes/v1` digest only. A
    /// region's end never precedes its start; an empty region is a point.
    #[trace("TC-420", "FR-095-AC-1")]
    #[test]
    fn raw_source_refs_and_regions_refuse_malformed_members() {
        let revision = || Revision::new("git", "abc").unwrap();
        let bytes = digest(DigestDomain::SourceBytesV1, 1);
        assert_eq!(
            RawSourceRef::new("", "u", revision(), bytes),
            Err(InvalidProvenance::EmptyAuthority)
        );
        assert_eq!(
            RawSourceRef::new("a", "", revision(), bytes),
            Err(InvalidProvenance::EmptyIdentity)
        );
        assert_eq!(
            Revision::new("", "abc"),
            Err(InvalidProvenance::EmptyRevisionNamespace)
        );
        assert_eq!(
            Revision::new("git", ""),
            Err(InvalidProvenance::EmptyRevisionValue)
        );
        assert_eq!(
            RawSourceRef::new(
                "a",
                "u",
                revision(),
                digest(DigestDomain::DefinitionBytesV1, 1)
            ),
            Err(InvalidProvenance::NotSourceBytes(
                DigestDomain::DefinitionBytesV1
            ))
        );
        assert_eq!(
            SourceRegion::new(source("a", 1), 5, 4).unwrap_err(),
            InvalidProvenance::ReversedRegion { start: 5, end: 4 }
        );
        let point = SourceRegion::new(source("a", 1), 5, 5).unwrap();
        assert_eq!((point.start(), point.end()), (5, 5));
    }

    /// O-07 key equality is lexical over (node id, role, ordinal): each
    /// member alone separates two keys.
    #[trace("TC-420", "FR-095-AC-1")]
    #[test]
    fn occurrence_keys_compare_over_node_role_and_ordinal() {
        assert_eq!(key(1, "expression", 0), key(1, "expression", 0));
        assert_ne!(key(1, "expression", 0), key(2, "expression", 0));
        assert_ne!(key(1, "expression", 0), key(1, "type", 0));
        assert_ne!(key(1, "expression", 0), key(1, "expression", 1));
    }

    fn map() -> PackageSourceMap {
        PackageSourceMap::from_entries([
            (key(1, "declaration", 0), vec![region(1, 0, 4)]),
            (
                key(1, "expression", 0),
                vec![region(1, 10, 12), region(1, 20, 22)],
            ),
            (key(1, "expression", 1), vec![region(1, 30, 31)]),
            (key(2, "type", 0), vec![region(1, 40, 41)]),
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
            Some(&[region(1, 10, 12), region(1, 20, 22)][..])
        );
        assert_eq!(
            map.regions(&key(1, "expression", 1)),
            Some(&[region(1, 30, 31)][..])
        );
        assert_eq!(map.regions(&key(1, "expression", 2)), None);
        let node_one: Vec<_> = map
            .occurrences(WireNodeId::from_digest([1; 32]))
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
        assert_eq!(map.occurrences(WireNodeId::from_digest([3; 32])).count(), 0);
    }

    /// O-12: a location tag resolves through the map by its node id and
    /// occurrence; a tag naming no node of the package, or an occurrence
    /// the package does not map, refuses with its own cause.
    #[trace("TC-421", "FR-095-AC-4")]
    #[test]
    fn a_location_resolves_or_refuses_by_cause() {
        let map = map();
        let location = |node: u8, role: &str, ordinal| {
            Location::new(
                NodeKey::from_digest([node; 32]),
                Origin::new(Role::new(role), ordinal),
            )
        };
        assert_eq!(
            map.resolve(&location(2, "type", 0)),
            Ok(&[region(1, 40, 41)][..])
        );
        assert_eq!(
            map.resolve(&location(3, "type", 0)),
            Err(UnresolvedOccurrence::UnknownNode(WireNodeId::from_digest(
                [3; 32]
            )))
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
                (key(1, "type", 0), vec![region(1, 0, 1)]),
                (key(1, "type", 0), vec![region(1, 2, 3)]),
            ]),
            Err(InvalidProvenance::DuplicateOccurrence(key(1, "type", 0)))
        );
    }
}
