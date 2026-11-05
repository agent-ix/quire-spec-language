// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`RequirementMap`]: a package's requirement records, keyed by occurrence
//! and capability and held in occurrence order without comparing node keys.
//!
//! `OccurrenceKey` orders by its node key first, and two node keys that
//! share a deep chain of nested operands cost that chain's depth to compare.
//! A `BTreeMap` keyed by the records of a deep body (one per addition of a
//! 100,000-term sum) compares neighbouring chain keys on every build, which
//! costs the body's length times its depth (QSL-381: depth is unbounded).
//! This map is ranked instead: once by origin, then stably by node through
//! `super::super::key_order`, whose ranking costs each key's own width,
//! never its depth. Lookups go through a hash index.

use std::collections::HashMap;
use std::fmt;

use qsl_foundation::source::node_table::{standalone, DebugTable, DebugWith, With};
use qsl_foundation::source::provenance::OccurrenceKey;

use super::RequirementRecord;
use crate::check::key_order;
use crate::check::Capability;

/// Requirement records keyed by occurrence and capability, iterated in
/// ascending occurrence order.
#[derive(Clone, Default)]
pub struct RequirementMap {
    /// Every record, ascending by occurrence, then capability label.
    entries: Vec<(OccurrenceKey, RequirementRecord)>,
    /// Each occurrence's positions in `entries`.
    index: HashMap<OccurrenceKey, Vec<usize>>,
}

impl RequirementMap {
    /// An empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// `records` ranked by occurrence, then capability label. Later duplicate
    /// occurrence and capability pairs replace earlier records.
    fn ranked(records: impl IntoIterator<Item = (OccurrenceKey, RequirementRecord)>) -> Self {
        let mut entries: Vec<(OccurrenceKey, RequirementRecord)> = Vec::new();
        let mut positions: HashMap<(OccurrenceKey, Capability), usize> = HashMap::new();
        for (occurrence, record) in records {
            let pair = (occurrence.clone(), record.requirements().kind());
            if let Some(&at) = positions.get(&pair) {
                entries[at].1 = record;
            } else {
                positions.insert(pair, entries.len());
                entries.push((occurrence, record));
            }
        }
        entries.sort_by(|(left_key, left), (right_key, right)| {
            left_key.origin().cmp(right_key.origin()).then_with(|| {
                left.requirements()
                    .kind()
                    .to_wire()
                    .cmp(right.requirements().kind().to_wire())
            })
        });
        let entries = key_order::sorted_by_key(entries, |(occurrence, _)| occurrence.node());
        let mut index: HashMap<OccurrenceKey, Vec<usize>> = HashMap::new();
        for (at, (occurrence, _)) in entries.iter().enumerate() {
            index.entry(occurrence.clone()).or_default().push(at);
        }
        Self { entries, index }
    }

    /// The map with `records` added, ranked once, or the map unchanged as
    /// the error when an occurrence and capability pair already exists.
    pub(crate) fn extended(
        self,
        records: impl IntoIterator<Item = (OccurrenceKey, RequirementRecord)>,
    ) -> Result<Self, Self> {
        let records: Vec<_> = records.into_iter().collect();
        let mut pairs = std::collections::HashSet::new();
        for (occurrence, record) in &records {
            let kind = record.requirements().kind();
            if !pairs.insert((occurrence.clone(), kind))
                || self.get_kind(occurrence, kind).is_some()
            {
                return Err(self);
            }
        }
        let mut all = self.entries;
        all.extend(records);
        Ok(Self::ranked(all))
    }

    /// The sole record at `key`, or `None` when absent or ambiguous by kind.
    pub fn get(&self, key: &OccurrenceKey) -> Option<&RequirementRecord> {
        let [at] = self.index.get(key)?.as_slice() else {
            return None;
        };
        Some(&self.entries[*at].1)
    }

    /// The record of `kind` at `key`.
    pub fn get_kind(&self, key: &OccurrenceKey, kind: Capability) -> Option<&RequirementRecord> {
        self.index
            .get(key)?
            .iter()
            .map(|&at| &self.entries[at].1)
            .find(|record| record.requirements().kind() == kind)
    }

    /// Whether a record is at `key`.
    pub fn contains_key(&self, key: &OccurrenceKey) -> bool {
        self.index.contains_key(key)
    }

    /// The number of records.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the map holds no record.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every key and record, ascending by key.
    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&OccurrenceKey, &RequirementRecord)> + ExactSizeIterator
    {
        self.entries.iter().map(|(key, record)| (key, record))
    }

    /// Every key, ascending.
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &OccurrenceKey> + ExactSizeIterator {
        self.entries.iter().map(|(key, _)| key)
    }

    /// Every record, by ascending key, to change in place; the keys and
    /// their order stay.
    pub fn values_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut RequirementRecord> {
        self.entries.iter_mut().map(|(_, record)| record)
    }

    /// Every record, by ascending key.
    pub fn values(
        &self,
    ) -> impl DoubleEndedIterator<Item = &RequirementRecord> + ExactSizeIterator {
        self.entries.iter().map(|(_, record)| record)
    }
}

/// A later record for one occurrence and capability replaces its earlier
/// record. Different capabilities may share the occurrence.
impl FromIterator<(OccurrenceKey, RequirementRecord)> for RequirementMap {
    fn from_iter<I: IntoIterator<Item = (OccurrenceKey, RequirementRecord)>>(records: I) -> Self {
        Self::ranked(records)
    }
}

impl From<HashMap<OccurrenceKey, RequirementRecord>> for RequirementMap {
    fn from(records: HashMap<OccurrenceKey, RequirementRecord>) -> Self {
        Self::ranked(records)
    }
}

/// The sole record at `key`; panics when absent or ambiguous by capability.
impl std::ops::Index<&OccurrenceKey> for RequirementMap {
    type Output = RequirementRecord;

    fn index(&self, key: &OccurrenceKey) -> &RequirementRecord {
        self.get(key).expect("a requirement record at the key")
    }
}

impl<'a> IntoIterator for &'a RequirementMap {
    type Item = (&'a OccurrenceKey, &'a RequirementRecord);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, (OccurrenceKey, RequirementRecord)>,
        fn(&'a (OccurrenceKey, RequirementRecord)) -> (&'a OccurrenceKey, &'a RequirementRecord),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter().map(|(key, record)| (key, record))
    }
}

/// Two maps are equal when their ordered occurrence and capability records
/// match.
impl PartialEq for RequirementMap {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Eq for RequirementMap {}

/// Formatted as a map in key order, the same as a `BTreeMap`; each key's node
/// and each record's nodes join the enclosing table.
impl DebugWith for RequirementMap {
    fn fmt_with(&self, formatter: &mut fmt::Formatter<'_>, table: &DebugTable) -> fmt::Result {
        formatter
            .debug_map()
            .entries(
                self.iter()
                    .map(|(key, record)| (With(key, table), With(record, table))),
            )
            .finish()
    }
}

impl fmt::Debug for RequirementMap {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        standalone(formatter, self)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use quire_exact::{NodeKey, Origin, PathSegment, Role};

    use super::*;
    use crate::check::Capability;
    use crate::family::{ClaimExtent, Requirements};
    use ix_trace_rs::trace;
    use quire_exact::ValueType;

    fn record(fill: u64) -> RequirementRecord {
        record_for(Capability::ValueValidity, fill)
    }

    fn record_for(kind: Capability, fill: u64) -> RequirementRecord {
        let node = NodeKey::new("p", vec![PathSegment::Text(format!("t{fill}"))], fill).unwrap();
        RequirementRecord::unguarded(
            Requirements::new(kind, ClaimExtent::Bounded),
            ValueType::Boolean,
            node,
            Default::default(),
        )
    }

    /// Keys over two origins and a nested chain, so node order, origin
    /// order and deep nesting all decide some pair.
    fn keys() -> Vec<OccurrenceKey> {
        let mut chain = vec![NodeKey::new("p", vec![PathSegment::Text("x".into())], 0).unwrap()];
        for _ in 0..200 {
            let inner = chain.last().cloned().unwrap();
            chain.push(
                NodeKey::new(
                    "p",
                    vec![PathSegment::Text("add".into()), PathSegment::Node(inner)],
                    0,
                )
                .unwrap(),
            );
        }
        let mut keys = Vec::new();
        for (at, node) in chain.into_iter().enumerate().rev() {
            keys.push(OccurrenceKey::new(
                node.clone(),
                Origin::new(Role::new("expression"), 0),
            ));
            if at % 3 == 0 {
                keys.push(OccurrenceKey::new(node, Origin::new(Role::new("claim"), 1)));
            }
        }
        keys
    }

    /// The map iterates, looks up and formats exactly as a `BTreeMap` built
    /// from the same records.
    #[test]
    fn agrees_with_a_btree_map() {
        let records: Vec<(OccurrenceKey, RequirementRecord)> = keys()
            .into_iter()
            .zip(0..)
            .map(|(key, fill)| (key, record(fill)))
            .collect();
        let expected: BTreeMap<_, _> = records.iter().cloned().collect();
        let map: RequirementMap = records.into_iter().collect();
        assert_eq!(map.len(), expected.len());
        assert!(map.iter().eq(expected.iter()));
        assert!(map.keys().eq(expected.keys()));
        assert!(map.values().eq(expected.values()));
        assert!((&map).into_iter().eq(expected.iter()));
        for (key, value) in &expected {
            assert_eq!(map.get(key), Some(value));
            assert!(map.contains_key(key));
        }
        assert_eq!(format!("{map:?}"), format!("{expected:?}"));
    }

    /// Extending ranks the added records among the present ones and refuses
    /// a key already present.
    #[test]
    fn extending_ranks_and_refuses_a_present_key() {
        let all: Vec<(OccurrenceKey, RequirementRecord)> = keys()
            .into_iter()
            .zip(0..)
            .map(|(key, fill)| (key, record(fill)))
            .collect();
        let (first, second) = all.split_at(all.len() / 2);
        let map: RequirementMap = first.iter().cloned().collect();
        let map = map
            .extended(second.iter().cloned())
            .unwrap_or_else(|_| panic!("distinct keys extend"));
        let expected: RequirementMap = all.iter().cloned().collect();
        assert_eq!(map, expected);
        let present: HashMap<_, _> = [all[0].clone()].into_iter().collect();
        let refused = map.extended(present).expect_err("a present key refuses");
        assert_eq!(refused, expected, "a refused extension adds nothing");
    }

    /// Different capability records share an occurrence, while a duplicate
    /// occurrence and capability pair replaces its earlier record.
    #[trace("FR-507-AC-7")]
    #[test]
    fn one_occurrence_can_hold_records_of_distinct_kinds() {
        let key = keys().remove(0);
        let temporal = record_for(Capability::TemporalSatisfaction, 1);
        let operation = record_for(Capability::OperationContract, 2);
        let map: RequirementMap = [
            (key.clone(), temporal.clone()),
            (key.clone(), operation.clone()),
        ]
        .into_iter()
        .collect();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&key), None, "an unqualified lookup is ambiguous");
        assert_eq!(
            map.get_kind(&key, Capability::TemporalSatisfaction),
            Some(&temporal)
        );
        assert_eq!(
            map.get_kind(&key, Capability::OperationContract),
            Some(&operation)
        );
    }
}
