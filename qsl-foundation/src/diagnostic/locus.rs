// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 T-5 (DA-13): the foundation diagnostic locus, and the JSON
//! pointer its artifact variant carries.
//!
//! A stage converts its own position into a [`Locus`] when it emits a
//! diagnostic (ADR-011 §6.1): S0 to S2 name a [`SourceRegion`], S3 on name
//! a kernel [`Location`] tag, and a wire reader names a digest-addressed
//! artifact and a pointer into it.

use std::fmt;
use std::str::FromStr;

use quire_exact::Location;

use crate::digest::DigestRecord;
use crate::source::provenance::{PackageSourceMap, SourceRegion, UnresolvedOccurrence};

/// ADR-013 T-5: where a diagnostic points.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Locus {
    /// An O-07 source region, used by S0 to S2.
    Region(SourceRegion),
    /// The kernel location tag (node id and occurrence key, O-12), used
    /// from S3 on and resolved to regions through the package source map
    /// when rendered ([`Locus::regions`]).
    Occurrence(Location),
    /// A digest-addressed artifact (O-18) and a JSON pointer into it, used
    /// by wire readers.
    Artifact {
        /// The artifact's digest record.
        digest: DigestRecord,
        /// The pointer into that artifact.
        pointer: JsonPointer,
    },
}

/// Why a [`Locus`] names no source region.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum UnresolvedLocus {
    /// The occurrence tag did not resolve through the package source map.
    #[error(transparent)]
    Occurrence(#[from] UnresolvedOccurrence),
    /// The locus names an artifact position, which no source region
    /// denotes.
    #[error("an artifact locus names no source region")]
    Artifact,
}

impl Locus {
    /// The source regions this locus denotes: a region locus is its own
    /// region, and an occurrence locus resolves through `map` (ADR-013
    /// T-5). An artifact locus names no source region.
    pub fn regions<'a>(
        &'a self,
        map: &'a PackageSourceMap,
    ) -> Result<&'a [SourceRegion], UnresolvedLocus> {
        match self {
            Self::Region(region) => Ok(std::slice::from_ref(region)),
            Self::Occurrence(location) => Ok(map.resolve(location)?),
            Self::Artifact { .. } => Err(UnresolvedLocus::Artifact),
        }
    }
}

/// An RFC 6901 JSON pointer: empty (the whole document) or a sequence of
/// `/`-prefixed reference tokens in which `~` is only ever `~0` or `~1`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct JsonPointer(String);

/// Why text is not an RFC 6901 JSON pointer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum InvalidJsonPointer {
    /// A non-empty pointer does not start with `/`.
    #[error("a non-empty JSON pointer starts with '/'")]
    MissingLeadingSlash,
    /// A `~` at this byte offset is not followed by `0` or `1`.
    #[error("'~' at byte {0} is not followed by '0' or '1'")]
    BadEscape(usize),
}

impl JsonPointer {
    /// The empty pointer: the whole document.
    pub fn root() -> Self {
        Self(String::new())
    }

    /// This pointer extended by one object member name, escaped per RFC
    /// 6901 (`~` as `~0`, `/` as `~1`).
    #[must_use]
    pub fn key(mut self, key: &str) -> Self {
        self.0.push('/');
        for character in key.chars() {
            match character {
                '~' => self.0.push_str("~0"),
                '/' => self.0.push_str("~1"),
                other => self.0.push(other),
            }
        }
        self
    }

    /// The pointer's text, exactly as parsed.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for JsonPointer {
    type Err = InvalidJsonPointer;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if !text.is_empty() && !text.starts_with('/') {
            return Err(InvalidJsonPointer::MissingLeadingSlash);
        }
        let bytes = text.as_bytes();
        for (at, byte) in bytes.iter().enumerate() {
            if *byte == b'~' && !matches!(bytes.get(at + 1), Some(b'0' | b'1')) {
                return Err(InvalidJsonPointer::BadEscape(at));
            }
        }
        Ok(Self(text.to_owned()))
    }
}

impl fmt::Display for JsonPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::{NodeKey, Origin, Role};

    use super::*;
    use crate::digest::{DigestDomain, WireNodeId};
    use crate::source::provenance::{OccurrenceKey, RawSourceRef, Revision};

    fn region(start: u64, end: u64) -> SourceRegion {
        SourceRegion::new(
            RawSourceRef::new(
                "a",
                "u",
                Revision::new("git", "abc").unwrap(),
                DigestRecord::mint(DigestDomain::SourceBytesV1, [7; 32]),
            )
            .unwrap(),
            start,
            end,
        )
        .unwrap()
    }

    fn origin(ordinal: u64) -> Origin {
        Origin::new(Role::new("expression"), ordinal)
    }

    /// T-5: a region locus is its own region; an occurrence locus resolves
    /// through the source map to the regions its key maps, and refuses with
    /// the map's cause when the key is unmapped; an artifact locus names no
    /// region.
    #[trace("TC-422", "FR-095-AC-5")]
    #[test]
    fn each_locus_variant_resolves_to_regions_by_its_own_rule() {
        let node = [9; 32];
        let map = PackageSourceMap::from_entries([(
            OccurrenceKey::new(WireNodeId::from_digest(node), origin(0)),
            vec![region(3, 8), region(12, 14)],
        )])
        .unwrap();
        assert_eq!(
            Locus::Region(region(1, 2)).regions(&map),
            Ok(&[region(1, 2)][..])
        );
        let located = Locus::Occurrence(Location::new(NodeKey::from_digest(node), origin(0)));
        assert_eq!(
            located.regions(&map),
            Ok(&[region(3, 8), region(12, 14)][..])
        );
        let unmapped = Location::new(NodeKey::from_digest(node), origin(1));
        assert_eq!(
            Locus::Occurrence(unmapped.clone()).regions(&map),
            Err(UnresolvedLocus::Occurrence(
                UnresolvedOccurrence::UnknownOccurrence(OccurrenceKey::from(&unmapped))
            ))
        );
        let artifact = Locus::Artifact {
            digest: DigestRecord::mint(DigestDomain::PackageSemanticV2, [1; 32]),
            pointer: "/source_map/0".parse().unwrap(),
        };
        assert_eq!(artifact.regions(&map), Err(UnresolvedLocus::Artifact));
    }

    /// T-5's artifact pointer is RFC 6901: the empty pointer and escaped
    /// tokens parse; a pointer without a leading `/` or with a bare `~`
    /// refuses.
    #[trace("TC-422", "FR-095-AC-6")]
    #[test]
    fn json_pointers_follow_rfc_6901() {
        for text in ["", "/", "/source_map/0/regions", "/a~1b/c~0d"] {
            assert_eq!(text.parse::<JsonPointer>().unwrap().as_str(), text);
        }
        assert_eq!(
            "source_map".parse::<JsonPointer>(),
            Err(InvalidJsonPointer::MissingLeadingSlash)
        );
        assert_eq!(
            "/a~2b".parse::<JsonPointer>(),
            Err(InvalidJsonPointer::BadEscape(2))
        );
        assert_eq!(
            "/a~".parse::<JsonPointer>(),
            Err(InvalidJsonPointer::BadEscape(2))
        );
        let built = JsonPointer::root().key("lock").key("a/b~c");
        assert_eq!(built.as_str(), "/lock/a~1b~0c");
        assert_eq!(built.as_str().parse::<JsonPointer>(), Ok(built.clone()));
        assert_eq!(JsonPointer::root().as_str(), "");
    }
}
