// SPDX-License-Identifier: AGPL-3.0-or-later
//! The one shared index over a [`DomainPackage`].
//!
//! Every model-layer rung used to rebuild its own maps from
//! `domain_package.records` on every call: the `specific -> supertypes[]`
//! map five times, a conformance index once per check, a dispatch index once
//! per link, and a per-call ancestor walk with its own visited set.
//! [`ModelIndex::build`] reads the records once, and every rung reads this
//! index instead. A [`ModelIndex`] owns its package (shared by [`Arc`]) and
//! refers to records by their position in `domain_package.records`, never by
//! a copy:
//!
//! - [`crate::model::normalize`] indexes the package once per normalization
//!   and keeps the index in the [`crate::model::normalize::EffectiveView`],
//!   together with the package, so dispatch linking and population admission
//!   read the view's index.
//! - A caller that checks conformance or systems rules over a package it
//!   has not normalized builds the index itself, once, and passes it to
//!   every check.
//!
//! Declarations are interned as `DeclIdx` in ascending [`DeclarationKey`]
//! order, so iterating indices ascending is iterating keys ascending: every
//! "ascending by declaration key" rule reads the index order directly.
//!
//! Conformance is answered from each type's ancestry, computed once per
//! type the first time a check asks about it and kept for every later check.
//! The ancestry records the exact state of the bounded walk the conformance
//! rule specifies (see `RecordIndex::conforms`), so an answer, including a
//! refusal at the caller's `ancestor_steps` ceiling, is the same one that
//! walk would give.
//!
//! A type's first conformance check walks that type's whole ancestry, to
//! completion, whatever the caller's `ancestor_steps` ceiling: the ceiling
//! decides the answer read from the ancestry, not how much of it is
//! computed. That first walk expands each interned type at most once and
//! keeps one entry per ancestor, so it is bounded by the package's own size,
//! not by the ceiling; a deep chain refused at a small ceiling still costs
//! one pass over the chain, once per start type.
//!
//! The index adds no charge. Its size is a function of the package's own
//! records, which normalization already charges as `normalize.record`, and
//! the conformance decisions it serves were never charged (FR-151 prices
//! `conformance.axis`, and `value-accounting.md` selects a population member
//! "without a charge, exactly when that type conforms to `T`").
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};

use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, FieldMemberRecord, OperationMemberRecord, ValueTypeRef,
};
use crate::model::key::DeclarationKey;
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};
use qsl_foundation::diagnostic::Code;

/// One interned declaration key. Index order is [`DeclarationKey`] order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct DeclIdx(usize);

/// Which record kinds declare one key. A key can carry several flags only in
/// a package that normalization refuses (`conflicting-binding`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DeclFlags {
    object_type: bool,
    /// Some `ObjectType` record under this key is `abstract`.
    abstract_type: bool,
    /// Some `ObjectType` record under this key declares a supertype.
    non_root: bool,
    scalar_type: bool,
    record_value_type: bool,
    field_member: bool,
    operation_member: bool,
}

/// One member's own `redefines` edge, as normalization's phase 4 reads it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Redefiner {
    /// The member's position in `domain_package.records`.
    pub(crate) record: usize,
    /// The redefining member.
    pub(crate) member: DeclIdx,
    /// The member it redefines.
    pub(crate) target: DeclIdx,
}

/// One type's ancestry: the walk [`RecordIndex::conforms`] specifies, run to
/// completion from that type once.
#[derive(Clone, Debug, Default)]
struct Ancestry {
    /// How many `supertypes` edges the walk follows in all: every edge of
    /// the start type's closure, once.
    edges: u64,
    /// Every type some expanded type names as a direct supertype, ascending
    /// by [`DeclIdx`], with the 1-based ordinal of the first edge that names
    /// it.
    first_named_at: Box<[(DeclIdx, u64)]>,
}

/// The shared index over one [`DomainPackage`], together with that package.
/// See the module docs.
///
/// A package and its index are built together and never paired afterwards,
/// so no caller can read one package's records through another's index.
#[derive(Debug, Eq, PartialEq)]
pub struct ModelIndex {
    package: Arc<DomainPackage>,
    records: RecordIndex,
    type_fact_counts: BTreeMap<DeclarationKey, u64>,
}

/// The positional index [`ModelIndex`] keeps over its package's records.
/// Normalization builds it over a borrowed package, before a completed
/// view takes a shared copy of that package.
///
/// Built from the records alone and never mutated after [`Self::build`],
/// apart from each type's ancestry, which is computed on first use.
#[derive(Debug)]
pub(crate) struct RecordIndex {
    /// Every key a record declares or references, ascending.
    keys: Vec<DeclarationKey>,
    positions: HashMap<DeclarationKey, DeclIdx>,
    flags: Vec<DeclFlags>,
    /// Each type's declared `supertypes[]`, in record order, concatenated
    /// over every `ObjectType` record under the key.
    generals: Vec<Vec<DeclIdx>>,
    /// The position of the last field record under each key.
    fields: HashMap<DeclIdx, usize>,
    /// The position of the last operation record under each key, ascending
    /// by key.
    operations: BTreeMap<DeclIdx, usize>,
    /// The last scalar record's `[lower, upper]` under each key.
    scalars: HashMap<DeclIdx, (i128, i128)>,
    /// Each member's owning type, from the last member record under its key.
    member_owner: HashMap<DeclIdx, DeclIdx>,
    /// Each owner's field members, in record order.
    fields_by_owner: HashMap<DeclIdx, Vec<DeclIdx>>,
    /// Each owner's operation members, in record order.
    operations_by_owner: HashMap<DeclIdx, Vec<DeclIdx>>,
    /// Each owner's redefining members, in record order.
    redefiners_by_owner: HashMap<DeclIdx, Vec<Redefiner>>,
    /// Each member's `redefines` target, from the first member record under
    /// its key that declares one.
    redefines: HashMap<DeclIdx, DeclIdx>,
    record_count: usize,
    ancestry: Vec<OnceLock<Ancestry>>,
}

impl PartialEq for RecordIndex {
    /// Compares what [`Self::build`] read from the records. The ancestries
    /// are a cache of values derived from `generals`, so two indexes with
    /// equal `generals` have equal ancestries whichever were computed.
    fn eq(&self, other: &Self) -> bool {
        self.keys == other.keys
            && self.flags == other.flags
            && self.generals == other.generals
            && self.fields == other.fields
            && self.operations == other.operations
            && self.scalars == other.scalars
            && self.member_owner == other.member_owner
            && self.fields_by_owner == other.fields_by_owner
            && self.operations_by_owner == other.operations_by_owner
            && self.redefiners_by_owner == other.redefiners_by_owner
            && self.redefines == other.redefines
            && self.record_count == other.record_count
    }
}

impl Eq for RecordIndex {}

/// Every key `record` declares or references, for interning.
fn referenced_keys(record: &DomainPackageRecord) -> Vec<&DeclarationKey> {
    match record {
        DomainPackageRecord::ObjectType(object) => std::iter::once(&object.key)
            .chain(&object.supertypes)
            .collect(),
        DomainPackageRecord::FieldMember(field) => std::iter::once(&field.key)
            .chain([&field.owner])
            .chain(&field.redefines)
            .collect(),
        DomainPackageRecord::OperationMember(operation) => std::iter::once(&operation.key)
            .chain([&operation.owner])
            .chain(&operation.redefines)
            .collect(),
        DomainPackageRecord::RecordValueType(_)
        | DomainPackageRecord::ScalarType(_)
        | DomainPackageRecord::Component(_)
        | DomainPackageRecord::Endpoint(_)
        | DomainPackageRecord::Relationship(_)
        | DomainPackageRecord::Allocation(_)
        | DomainPackageRecord::Population(_) => vec![record.key()],
    }
}

impl ModelIndex {
    /// Reads `domain_package`'s records once into an index that keeps the
    /// package. Passing an `Arc<DomainPackage>` shares that package; passing
    /// a `DomainPackage` moves it; neither copies its records.
    pub fn build(domain_package: impl Into<Arc<DomainPackage>>) -> Self {
        let package = domain_package.into();
        let records = RecordIndex::build(&package);
        let type_fact_counts = records.type_fact_counts();
        Self { package, records, type_fact_counts }
    }

    /// `package` with `records`, the index normalization already built over
    /// it.
    pub(crate) fn from_parts(package: Arc<DomainPackage>, records: RecordIndex,
        type_fact_counts: BTreeMap<DeclarationKey, u64>) -> Self {
        debug_assert_eq!(
            records.record_count,
            package.records.len(),
            "the record index was built over this package's records"
        );
        Self { package, records, type_fact_counts }
    }

    /// Actual qualify/inherit fact count for an object type; non-object
    /// value-domain conversion is priced as one work unit.
    pub(crate) fn type_fact_count(&self, value_type: &ValueTypeRef) -> u64 {
        value_type.as_package().and_then(|key| self.type_fact_counts.get(key)).copied().unwrap_or(1)
    }

    /// FR-151 conformance after its owning axis charge has admitted.
    /// Every walk has its own edge ceiling; its denial does not refund work.
    pub(crate) fn conformance_walk(&self, s: &DeclarationKey, t: &DeclarationKey,
        limit: u64) -> Result<bool, crate::model::accounting::Incomplete> {
        if s == t { return Ok(true); }
        let Some(start) = self.records.position(s) else { return Ok(false); };
        let mut stack = vec![start];
        let mut expanded = std::collections::HashSet::new();
        let mut consumed = 0;
        while let Some(current) = stack.pop() {
            if !expanded.insert(current) { continue; }
            for general in &self.records.generals[current.0] {
                if consumed == limit {
                    return Err(crate::model::accounting::Incomplete {
                        limit_kind: crate::model::accounting::LimitKind::AncestorSteps,
                        limit, consumed, next_charge: 1,
                        charge_point: crate::model::accounting::ChargePoint::ModelAncestorEdge,
                    });
                }
                consumed += 1;
                if self.records.key(*general) == t { return Ok(true); }
                stack.push(*general);
            }
        }
        Ok(false)
    }

    /// Object reachability or exact native/scalar domain conversion.
    pub(crate) fn value_conformance_walk(&self, s: &ValueTypeRef, t: &ValueTypeRef,
        limit: u64) -> Result<bool, crate::model::accounting::Incomplete> {
        match (s, t) {
            (ValueTypeRef::Native(a), ValueTypeRef::Native(b)) => Ok(a == b),
            (ValueTypeRef::Package(a), ValueTypeRef::Package(b)) => {
                match (self.scalar_bounds(a), self.scalar_bounds(b)) {
                    (Some((lo, hi)), Some((parent_lo, parent_hi))) =>
                        Ok(parent_lo <= lo && hi <= parent_hi),
                    (Some(_), None) | (None, Some(_)) => Ok(false),
                    (None, None) => self.conformance_walk(a, b, limit),
                }
            }
            _ => Ok(false),
        }
    }

    /// All declared field redefinition links followed by an effect test,
    /// including the written member itself; cycles remain bounded by records.
    pub(crate) fn field_lineage(&self, field: &DeclarationKey) -> Vec<&DeclarationKey> {
        let Some(mut current) = self.records.position(field) else { return Vec::new(); };
        let mut path = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while seen.insert(current) {
            path.push(self.records.key(current));
            let Some(next) = self.records.redefines.get(&current) else { break; };
            current = *next;
        }
        path
    }

    /// The indexed package.
    pub(crate) fn package(&self) -> &DomainPackage {
        &self.package
    }

    /// Whether some `ObjectType` record declares `key`.
    pub(crate) fn is_object_type(&self, key: &DeclarationKey) -> bool {
        self.records.is_object_type(key)
    }

    /// Whether some `ObjectType` record declares `key` abstract (D05,
    /// `model-complete.md:156`).
    pub(crate) fn is_abstract(&self, key: &DeclarationKey) -> bool {
        self.records.is_abstract(key)
    }

    /// The field record declared under `key` (the last one, if several).
    pub(crate) fn field(&self, key: &DeclarationKey) -> Option<&FieldMemberRecord> {
        self.records.field(&self.package.records, key)
    }

    /// The operation record declared under `key` (the last one, if several).
    pub(crate) fn operation(&self, key: &DeclarationKey) -> Option<&OperationMemberRecord> {
        self.records.operation(&self.package.records, key)
    }

    /// Every operation record, ascending by key.
    pub(crate) fn operations(&self) -> impl Iterator<Item = &OperationMemberRecord> {
        self.records.operations(&self.package.records)
    }

    /// The `[lower, upper]` of the scalar type declared under `key`.
    pub(crate) fn scalar_bounds(&self, key: &DeclarationKey) -> Option<(i128, i128)> {
        self.records.scalar_bounds(key)
    }

    /// The owning type of the field or operation member `key`.
    pub(crate) fn member_owner(&self, key: &DeclarationKey) -> Option<&DeclarationKey> {
        self.records.member_owner(key)
    }

    /// See [`RecordIndex::redefinition_reaches`].
    pub(crate) fn redefinition_reaches(
        &self,
        field: &DeclarationKey,
        admits: impl Fn(&DeclarationKey) -> bool,
    ) -> bool {
        self.records.redefinition_reaches(field, admits)
    }

    /// See [`RecordIndex::conforms`].
    pub(crate) fn conforms(
        &self,
        s: &DeclarationKey,
        t: &DeclarationKey,
        max_steps: u64,
        setting: qsl_foundation::Setting,
    ) -> Result<bool, ModelRefusal> {
        self.records.conforms(s, t, max_steps, setting)
    }

    /// See [`RecordIndex::value_type_conforms`].
    pub(crate) fn value_type_conforms(
        &self,
        s: &ValueTypeRef,
        t: &ValueTypeRef,
        max_steps: u64,
        setting: qsl_foundation::Setting,
    ) -> Result<bool, ModelRefusal> {
        self.records.value_type_conforms(s, t, max_steps, setting)
    }
}

/// The record at `position`, when it is a field member.
fn field_at(records: &[DomainPackageRecord], position: usize) -> Option<&FieldMemberRecord> {
    match records.get(position)? {
        DomainPackageRecord::FieldMember(field) => Some(field),
        _ => None,
    }
}

/// The record at `position`, when it is an operation member.
fn operation_at(
    records: &[DomainPackageRecord],
    position: usize,
) -> Option<&OperationMemberRecord> {
    match records.get(position)? {
        DomainPackageRecord::OperationMember(operation) => Some(operation),
        _ => None,
    }
}

impl RecordIndex {
    /// Count every declared inheritance path using one explicit-stack
    /// dynamic program. Structural normalization refuses cycles before its
    /// resulting view is passed to conformance.
    fn type_fact_counts(&self) -> BTreeMap<DeclarationKey, u64> {
        let mut counts: HashMap<DeclIdx, u64> = HashMap::new();
        for key in self.object_types() {
            let Some(start) = self.position(key) else { continue; };
            let mut active = std::collections::HashSet::new();
            let mut stack = vec![(start, false)];
            while let Some((current, closing)) = stack.pop() {
                if counts.contains_key(&current) { continue; }
                if closing {
                    let count = self.generals[current.0].iter().fold(1_u64,
                        |sum, general| sum.saturating_add(counts.get(general).copied().unwrap_or(u64::MAX)));
                    counts.insert(current, count);
                    active.remove(&current);
                } else if active.insert(current) {
                    stack.push((current, true));
                    for general in self.generals[current.0].iter().rev() {
                        if !counts.contains_key(general) { stack.push((*general, false)); }
                    }
                } else {
                    counts.insert(current, u64::MAX);
                }
            }
        }
        counts.into_iter().map(|(key, count)| (self.key(key).clone(), count)).collect()
    }

    /// Reads `domain_package`'s records once into the index.
    pub(crate) fn build(domain_package: &DomainPackage) -> Self {
        let mut keys: Vec<DeclarationKey> = domain_package
            .records
            .iter()
            .flat_map(referenced_keys)
            .cloned()
            .collect();
        keys.sort_unstable();
        keys.dedup();
        let positions: HashMap<DeclarationKey, DeclIdx> = keys
            .iter()
            .enumerate()
            .map(|(position, key)| (key.clone(), DeclIdx(position)))
            .collect();
        let at = |key: &DeclarationKey| positions[key];

        let mut index = Self {
            flags: vec![DeclFlags::default(); keys.len()],
            generals: vec![Vec::new(); keys.len()],
            ancestry: std::iter::repeat_with(OnceLock::new)
                .take(keys.len())
                .collect(),
            fields: HashMap::new(),
            operations: BTreeMap::new(),
            scalars: HashMap::new(),
            member_owner: HashMap::new(),
            fields_by_owner: HashMap::new(),
            operations_by_owner: HashMap::new(),
            redefiners_by_owner: HashMap::new(),
            redefines: HashMap::new(),
            record_count: domain_package.records.len(),
            keys: Vec::new(),
            positions: HashMap::new(),
        };
        for (record_position, record) in domain_package.records.iter().enumerate() {
            let own = at(record.key());
            match record {
                DomainPackageRecord::ObjectType(object) => {
                    let flags = &mut index.flags[own.0];
                    flags.object_type = true;
                    flags.abstract_type |= object.abstract_type;
                    flags.non_root |= !object.supertypes.is_empty();
                    index.generals[own.0].extend(object.supertypes.iter().map(at));
                }
                DomainPackageRecord::FieldMember(field) => {
                    let owner = at(&field.owner);
                    index.flags[own.0].field_member = true;
                    index.member_owner.insert(own, owner);
                    index.fields_by_owner.entry(owner).or_default().push(own);
                    if let Some(target) = &field.redefines {
                        index.note_redefiner(owner, record_position, own, at(target), true);
                    }
                    index.fields.insert(own, record_position);
                }
                DomainPackageRecord::OperationMember(operation) => {
                    let owner = at(&operation.owner);
                    index.flags[own.0].operation_member = true;
                    index.member_owner.insert(own, owner);
                    index
                        .operations_by_owner
                        .entry(owner)
                        .or_default()
                        .push(own);
                    if let Some(target) = &operation.redefines {
                        index.note_redefiner(owner, record_position, own, at(target), false);
                    }
                    index.operations.insert(own, record_position);
                }
                DomainPackageRecord::RecordValueType(_) => {
                    index.flags[own.0].record_value_type = true;
                }
                DomainPackageRecord::ScalarType(scalar) => {
                    index.flags[own.0].scalar_type = true;
                    index.scalars.insert(own, (scalar.lower, scalar.upper));
                }
                // FR-152 systems-model records and FR-153 population
                // declarations are read by `crate::model::systems` and
                // `crate::model::normalize` from the records themselves.
                DomainPackageRecord::Component(_)
                | DomainPackageRecord::Endpoint(_)
                | DomainPackageRecord::Relationship(_)
                | DomainPackageRecord::Allocation(_)
                | DomainPackageRecord::Population(_) => {}
            }
        }
        index.keys = keys;
        index.positions = positions;
        index
    }

    fn note_redefiner(
        &mut self,
        owner: DeclIdx,
        record: usize,
        member: DeclIdx,
        target: DeclIdx,
    ) {
        self.redefiners_by_owner
            .entry(owner)
            .or_default()
            .push(Redefiner {
                record,
                member,
                target,
            });
        self.redefines.entry(member).or_insert(target);
    }

    /// `key`'s interned index, when some record declares or references it.
    pub(crate) fn position(&self, key: &DeclarationKey) -> Option<DeclIdx> {
        self.positions.get(key).copied()
    }

    /// The key interned at `index`.
    pub(crate) fn key(&self, index: DeclIdx) -> &DeclarationKey {
        &self.keys[index.0]
    }

    fn flags(&self, key: &DeclarationKey) -> DeclFlags {
        self.position(key)
            .map(|index| self.flags[index.0])
            .unwrap_or_default()
    }

    /// Whether some `ObjectType` record declares `key`.
    pub(crate) fn is_object_type(&self, key: &DeclarationKey) -> bool {
        self.flags(key).object_type
    }

    /// Whether some `ObjectType` record declares `key` abstract (D05,
    /// `model-complete.md:156`).
    pub(crate) fn is_abstract(&self, key: &DeclarationKey) -> bool {
        self.flags(key).abstract_type
    }

    /// Whether `key`'s own `supertypes[]` is non-empty, so it is not a root.
    pub(crate) fn is_non_root(&self, key: &DeclarationKey) -> bool {
        self.flags(key).non_root
    }

    /// Whether some `ScalarType` record declares `key`.
    pub(crate) fn is_scalar_type(&self, key: &DeclarationKey) -> bool {
        self.flags(key).scalar_type
    }

    /// Whether some `RecordValueType` record declares `key`.
    pub(crate) fn is_record_value_type(&self, key: &DeclarationKey) -> bool {
        self.flags(key).record_value_type
    }

    /// Whether some `FieldMember` record declares `key`.
    pub(crate) fn is_field_member(&self, key: &DeclarationKey) -> bool {
        self.flags(key).field_member
    }

    /// Whether some `OperationMember` record declares `key`.
    pub(crate) fn is_operation_member(&self, key: &DeclarationKey) -> bool {
        self.flags(key).operation_member
    }

    /// Every declared object type, ascending by key.
    pub(crate) fn object_types(&self) -> impl Iterator<Item = &DeclarationKey> {
        self.keys
            .iter()
            .zip(&self.flags)
            .filter(|(_, flags)| flags.object_type)
            .map(|(key, _)| key)
    }

    /// `specific`'s declared `supertypes[]`, ascending by key.
    pub(crate) fn sorted_generals(&self, specific: &DeclarationKey) -> Vec<DeclarationKey> {
        let Some(specific) = self.position(specific) else {
            return Vec::new();
        };
        let mut generals = self.generals[specific.0].clone();
        generals.sort_unstable();
        generals
            .into_iter()
            .map(|general| self.key(general).clone())
            .collect()
    }

    /// Every `(specific, general)` supertype edge, specifics ascending, each
    /// specific's generals in record order.
    pub(crate) fn generalization_edges(
        &self,
    ) -> impl Iterator<Item = (&DeclarationKey, &DeclarationKey)> {
        self.keys
            .iter()
            .zip(&self.generals)
            .flat_map(move |(specific, generals)| {
                generals
                    .iter()
                    .map(move |general| (specific, self.key(*general)))
            })
    }

    /// The field record declared under `key` (the last one, if several),
    /// read from `records`, the indexed package's own records.
    fn field<'r>(
        &self,
        records: &'r [DomainPackageRecord],
        key: &DeclarationKey,
    ) -> Option<&'r FieldMemberRecord> {
        field_at(records, *self.fields.get(&self.position(key)?)?)
    }

    /// The operation record declared under `key` (the last one, if several),
    /// read from `records`, the indexed package's own records.
    fn operation<'r>(
        &self,
        records: &'r [DomainPackageRecord],
        key: &DeclarationKey,
    ) -> Option<&'r OperationMemberRecord> {
        operation_at(records, *self.operations.get(&self.position(key)?)?)
    }

    /// Every operation record, ascending by key, read from `records`.
    fn operations<'r>(
        &'r self,
        records: &'r [DomainPackageRecord],
    ) -> impl Iterator<Item = &'r OperationMemberRecord> {
        self.operations
            .values()
            .filter_map(|position| operation_at(records, *position))
    }

    /// The `[lower, upper]` of the scalar type declared under `key`.
    pub(crate) fn scalar_bounds(&self, key: &DeclarationKey) -> Option<(i128, i128)> {
        self.scalars.get(&self.position(key)?).copied()
    }

    /// The owning type of the field or operation member `key`.
    pub(crate) fn member_owner(&self, key: &DeclarationKey) -> Option<&DeclarationKey> {
        let owner = self.member_owner.get(&self.position(key)?)?;
        Some(self.key(*owner))
    }

    /// `owner`'s directly declared field members, ascending by key, read
    /// from `records`, the indexed package's own records.
    pub(crate) fn sorted_direct_fields<'r>(
        &self,
        records: &'r [DomainPackageRecord],
        owner: &DeclarationKey,
    ) -> Vec<&'r FieldMemberRecord> {
        let Some(owner) = self.position(owner) else {
            return Vec::new();
        };
        let mut members: Vec<DeclIdx> = self
            .fields_by_owner
            .get(&owner)
            .cloned()
            .unwrap_or_default();
        members.sort();
        members
            .into_iter()
            .filter_map(|member| field_at(records, *self.fields.get(&member)?))
            .collect()
    }

    /// Both kinds of directly owned model feature, ascending by original key.
    pub(crate) fn sorted_direct_member_keys<'r>(&self, records: &'r [DomainPackageRecord],
        owner: &DeclarationKey) -> Vec<&'r DeclarationKey> {
        let mut keys: Vec<_> = self.sorted_direct_fields(records, owner).into_iter()
            .map(|field| &field.key).collect();
        keys.extend(self.direct_operations(owner).iter().filter_map(|member|
            operation_at(records, *self.operations.get(member)?).map(|operation| &operation.key)));
        keys.sort();
        keys
    }

    /// `owner`'s directly declared operation members, in record order.
    pub(crate) fn direct_operations(&self, owner: &DeclarationKey) -> &[DeclIdx] {
        self.position(owner)
            .and_then(|owner| self.operations_by_owner.get(&owner))
            .map_or(&[], Vec::as_slice)
    }

    /// `owner`'s own redefining members, in record order.
    pub(crate) fn redefiners_of_owner(&self, owner: &DeclarationKey) -> &[Redefiner] {
        self.position(owner)
            .and_then(|owner| self.redefiners_by_owner.get(&owner))
            .map_or(&[], Vec::as_slice)
    }

    /// Walks `field`'s redefinition chain one member's own `redefines` hop
    /// at a time (`model-complete.md`:162), returning `true` as soon as
    /// `admits` accepts `field` or a member the chain reaches, and `false`
    /// once the chain ends. A chain (`C.x -> B.x -> A.x`, with no direct
    /// `C.x -> A.x` edge) is legal (`model-complete.md`:56/:64), so this
    /// follows every hop, not only the first.
    ///
    /// Bounded by the package's record count, since an acyclic chain visits
    /// no more members than there are records: past that bound the chain is
    /// a cycle and the walk returns `false` rather than looping.
    pub(crate) fn redefinition_reaches(
        &self,
        field: &DeclarationKey,
        admits: impl Fn(&DeclarationKey) -> bool,
    ) -> bool {
        if admits(field) {
            return true;
        }
        let Some(mut current) = self.position(field) else {
            return false;
        };
        for _ in 0..self.record_count {
            let Some(&redefined) = self.redefines.get(&current) else {
                return false;
            };
            if admits(self.key(redefined)) {
                return true;
            }
            current = redefined;
        }
        false // Cycle: exceeded the maximum possible acyclic chain length.
    }

    /// `index`'s ancestry, computed on first use.
    ///
    /// The walk: an explicit stack seeded with `index`; pop a type, skip it
    /// if already expanded, otherwise expand it and follow each of its
    /// declared supertypes in record order, one edge at a time. Every
    /// supertype an edge names is recorded with that edge's ordinal the
    /// first time it is named, whether or not it was already expanded.
    fn ancestry(&self, index: DeclIdx) -> &Ancestry {
        self.ancestry[index.0].get_or_init(|| {
            let mut stack = vec![index];
            let mut expanded: std::collections::HashSet<DeclIdx> = std::collections::HashSet::new();
            let mut first_named_at: HashMap<DeclIdx, u64> = HashMap::new();
            let mut edges: u64 = 0;
            while let Some(current) = stack.pop() {
                if !expanded.insert(current) {
                    continue;
                }
                for general in &self.generals[current.0] {
                    edges += 1;
                    first_named_at.entry(*general).or_insert(edges);
                    stack.push(*general);
                }
            }
            let mut first_named_at: Vec<(DeclIdx, u64)> = first_named_at.into_iter().collect();
            first_named_at.sort_unstable();
            Ancestry {
                edges,
                first_named_at: first_named_at.into_boxed_slice(),
            }
        })
    }

    /// Whether `s` conforms to `t`: the same key, or a chain of declared
    /// supertypes from `s` to `t`.
    ///
    /// The rule is a bounded depth-first walk from `s` (an explicit stack, a
    /// visited set, supertypes pushed in record order) that stops as soon as
    /// an edge names `t`. `max_steps` is the caller's `ancestor_steps`
    /// ceiling
    /// ([`crate::model::accounting::ModelNormalizationLimits::ancestor_steps`]),
    /// used as given: it bounds how many `supertypes` edges the walk follows,
    /// each charged before it is followed, so under multiple supertypes it
    /// bounds the walk's closure, never a chain depth. A target `n` edges
    /// above `s` along a chain is reached at `max_steps == n`. Following one
    /// edge more refuses [`ModelRefusalCause::AncestorSteps`] naming `s` and
    /// `max_steps`; the walk is never truncated into a verdict, so a cycle
    /// refuses rather than loops.
    ///
    /// The answer is read from `s`'s ancestry, not by walking again: `t` is
    /// found at the first edge that names it, so the walk admits it when
    /// that edge's ordinal is within `max_steps`, and a walk that never finds
    /// `t` follows every edge of the closure.
    pub(crate) fn conforms(
        &self,
        s: &DeclarationKey,
        t: &DeclarationKey,
        max_steps: u64,
        setting: qsl_foundation::Setting,
    ) -> Result<bool, ModelRefusal> {
        if s == t {
            return Ok(true);
        }
        let exceeded = || ModelRefusal {
            code: Code::ResourceExhausted,
            cause: ModelRefusalCause::ancestor_steps(s.clone(), max_steps, setting),
            detail: format!(
                "conformance check from {} exceeded the ancestor_steps limit of {max_steps}",
                s.node
            ),
        };
        let Some(start) = self.position(s) else {
            // An uninterned `s` declares no supertype: the walk follows no
            // edge.
            return Ok(false);
        };
        let ancestry = self.ancestry(start);
        let found = self.position(t).and_then(|target| {
            ancestry
                .first_named_at
                .binary_search_by_key(&target, |(ancestor, _)| *ancestor)
                .ok()
                .map(|slot| ancestry.first_named_at[slot].1)
        });
        match found {
            Some(ordinal) if ordinal <= max_steps => Ok(true),
            None if ancestry.edges <= max_steps => Ok(false),
            Some(_) | None => Err(exceeded()),
        }
    }

    /// [`Self::conforms`], lifted to [`ValueTypeRef`]: a native value type
    /// conforms only to itself (QSL declares no generalization among native
    /// value types), and a native and a package value type never conform to
    /// one another.
    pub(crate) fn value_type_conforms(
        &self,
        s: &ValueTypeRef,
        t: &ValueTypeRef,
        max_steps: u64,
        setting: qsl_foundation::Setting,
    ) -> Result<bool, ModelRefusal> {
        match (s, t) {
            (ValueTypeRef::Native(a), ValueTypeRef::Native(b)) => Ok(a == b),
            (ValueTypeRef::Package(s_key), ValueTypeRef::Package(t_key)) => {
                self.conforms(s_key, t_key, max_steps, setting)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use ix_trace_rs::trace;

    use super::*;
    use crate::model::domain_package::{DomainPackageRef, ObjectTypeRecord};

    fn key(node: &str) -> DeclarationKey {
        DeclarationKey::fixture(node)
    }

    fn package(types: &[(&str, &[&str])]) -> DomainPackage {
        DomainPackage::new(
            DomainPackageRef::fixture("bundle.qsl202"),
            types
                .iter()
                .map(|(name, supertypes)| {
                    DomainPackageRecord::ObjectType(ObjectTypeRecord {
                        key: key(name),
                        interface_features: None,
                        abstract_type: false,
                        supertypes: supertypes.iter().map(|general| key(general)).collect(),
                    })
                })
                .collect(),
        )
    }

    /// The reference edge-charging walk of FR-082, the oracle
    /// [`RecordIndex::conforms`] must match: a depth-first walk over the
    /// records with a visited set, charging each `supertypes` edge before it
    /// is followed and refusing at the first edge past `max_steps`.
    fn walked(
        domain_package: &DomainPackage,
        s: &DeclarationKey,
        t: &DeclarationKey,
        max_steps: u64,
    ) -> Result<bool, ModelRefusal> {
        let mut generals_by_specific: HashMap<DeclarationKey, Vec<DeclarationKey>> = HashMap::new();
        for record in &domain_package.records {
            if let DomainPackageRecord::ObjectType(object_type) = record {
                if !object_type.supertypes.is_empty() {
                    generals_by_specific
                        .entry(object_type.key.clone())
                        .or_default()
                        .extend(object_type.supertypes.iter().cloned());
                }
            }
        }
        if s == t {
            return Ok(true);
        }
        let mut stack: Vec<DeclarationKey> = vec![s.clone()];
        let mut visited: HashSet<DeclarationKey> = HashSet::new();
        let mut steps: u64 = 0;
        let found = 'walk: {
            while let Some(current) = stack.pop() {
                if !visited.insert(current.clone()) {
                    continue;
                }
                for general in generals_by_specific.get(&current).into_iter().flatten() {
                    if steps >= max_steps {
                        return Err(ModelRefusal {
                            code: Code::ResourceExhausted,
                            cause: ModelRefusalCause::ancestor_steps(
                                s.clone(),
                                max_steps,
                                qsl_foundation::Setting::ModelAncestorSteps,
                            ),
                            detail: format!(
                                "conformance check from {} exceeded the ancestor_steps limit of {max_steps}",
                                s.node
                            ),
                        });
                    }
                    steps += 1;
                    if general == t {
                        break 'walk true;
                    }
                    stack.push(general.clone());
                }
            }
            false
        };
        Ok(found)
    }

    /// Interning assigns [`DeclIdx`] in ascending
    /// [`DeclarationKey`] order, whatever the record order, so reading the
    /// index ascending reads keys ascending.
    ///
    /// Mutation used: interning in record order (dropping the sort in
    /// `build`) fails the first assertion.
    #[test]
    #[trace("QSpec-TC-195", "QSpec-FR-150-AC-4")]
    fn decl_idx_order_is_declaration_key_order() {
        let domain_package = package(&[
            ("model.Z", &["model.M"]),
            ("model.A", &[]),
            ("model.M", &["model.A", "model.Q"]),
        ]);
        let index = RecordIndex::build(&domain_package);
        let interned: Vec<&DeclarationKey> = (0..index.keys.len())
            .map(|position| index.key(DeclIdx(position)))
            .collect();
        let mut ascending = interned.clone();
        ascending.sort();
        assert_eq!(interned, ascending);
        assert_eq!(
            interned,
            [
                &key("model.A"),
                &key("model.M"),
                &key("model.Q"),
                &key("model.Z")
            ],
            "every declared and referenced key is interned once"
        );
        for (position, interned_key) in interned.iter().enumerate() {
            assert_eq!(index.position(interned_key), Some(DeclIdx(position)));
        }
        assert_eq!(
            index.object_types().collect::<Vec<_>>(),
            [&key("model.A"), &key("model.M"), &key("model.Z")],
            "a referenced but undeclared supertype is not an object type"
        );
    }

    /// [`RecordIndex::conforms`] answers from the ancestry exactly
    /// as the per-call walk did, for every ordered pair of keys (declared,
    /// referenced and unknown) and every `ancestor_steps` ceiling from 0 to
    /// past each walk's length, over a chain, a diamond, a cycle, a
    /// self-loop, a repeated supertype, a dangling supertype and a type
    /// declared twice.
    ///
    /// Mutation used: comparing the found ordinal with `<` instead of `<=`
    /// fails the chain's exact-ceiling cases.
    #[test]
    #[trace("QSpec-TC-196", "QSpec-FR-151-AC-2", "QSpec-FR-151-AC-5")]
    fn conforms_matches_the_bounded_walk_everywhere() {
        let packages = [
            package(&[
                ("model.C3", &["model.C2"]),
                ("model.C2", &["model.C1"]),
                ("model.C1", &["model.C0"]),
                ("model.C0", &[]),
            ]),
            package(&[
                ("model.D", &["model.B", "model.C"]),
                ("model.B", &["model.A"]),
                ("model.C", &["model.A"]),
                ("model.A", &[]),
            ]),
            package(&[
                ("model.X", &["model.Y"]),
                ("model.Y", &["model.Z"]),
                ("model.Z", &["model.X", "model.W"]),
                ("model.W", &[]),
            ]),
            package(&[
                ("model.S", &["model.S", "model.R", "model.R"]),
                ("model.R", &["model.Gone"]),
                ("model.T", &["model.R"]),
                ("model.T", &["model.S"]),
            ]),
        ];
        let mut checked = 0usize;
        for domain_package in &packages {
            let index = RecordIndex::build(domain_package);
            let mut names: Vec<DeclarationKey> = index.keys.clone();
            names.push(key("model.Unknown"));
            for s in &names {
                for t in &names {
                    for max_steps in 0..=6 {
                        assert_eq!(
                            index.conforms(
                                s,
                                t,
                                max_steps,
                                qsl_foundation::Setting::ModelAncestorSteps
                            ),
                            walked(domain_package, s, t, max_steps),
                            "{} conforms to {} under ancestor_steps {max_steps}",
                            s.node,
                            t.node
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 500, "{checked} cases compared");
    }
}
