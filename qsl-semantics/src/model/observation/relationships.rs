// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106's `relationships` member: the read of a snapshot's entries, the
//! checks 6.6 to 6.9 over them, the instance ends checks 7 and 8 decide, and
//! the hand-off of the admitted entries to the extent store (FR-364).
//!
//! A defect in the shape of an entry or an instance is kept with the entry
//! at read and refused when the check walk reaches it, so an earlier object's
//! defect (checks 6.1 to 6.5) is reported before a later entry's, as FR-106's
//! walk order says.

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::diagnostic::Code;
use quire_exact::{EffectiveId, ObjectReference};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::relationship_extent::{
    EndSide, NavigationRelationship, PopulationEntry, PopulationEnumerator, RelationshipEnd,
    RelationshipExtent,
};

use super::document::{find_declaration, RawPopulation};
use super::extents::{end_name, PopulationEnumerationFailure, RelationshipExtents};
use super::helpers::object_reference;
use super::tree::{Json, Object};
use super::{
    fault, incomplete, refuse, AdmissionFailure, AdmissionRecord, LinkDifference, ModelView,
    ReadMeter, SelectedObject,
};
use crate::model::domain_package::{DomainPackageRecord, OperationEffect};
use crate::model::key::DeclarationKey;

// ---------------------------------------------------------------------------
// The read
// ---------------------------------------------------------------------------

/// One end of a snapshot instance: `{"population": ..., "key": ...}`.
pub(super) type RawEnd = SelectedObject;

/// One instance of an entry, or the defect in its shape.
#[derive(Clone, Debug)]
pub(super) struct RawInstance {
    source: RawEnd,
    target: RawEnd,
}

/// One entry of a snapshot's `relationships` member that has the three
/// members of FR-106 in their kinds.
#[derive(Clone, Debug)]
pub(super) struct RawRelationship {
    relationship: String,
    complete: bool,
    instances: Vec<Result<RawInstance, AdmissionRecord>>,
}

/// One entry, or the shape defect check 6.6 refuses it for.
#[derive(Clone, Debug)]
pub(super) struct RawEntry(Result<RawRelationship, AdmissionRecord>);

/// A member the entry names, converted once at read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntryMember {
    Relationship,
    Complete,
    Instances,
}

impl EntryMember {
    const ALL: [Self; 3] = [Self::Relationship, Self::Complete, Self::Instances];

    #[qsl_attrs::string_edge]
    fn read(name: &str) -> Option<Self> {
        match name {
            "relationship" => Some(Self::Relationship),
            "complete" => Some(Self::Complete),
            "instances" => Some(Self::Instances),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Relationship => "relationship",
            Self::Complete => "complete",
            Self::Instances => "instances",
        }
    }
}

/// A member of an instance, converted once at read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstanceMember {
    Source,
    Target,
}

impl InstanceMember {
    const ALL: [Self; 2] = [Self::Source, Self::Target];

    #[qsl_attrs::string_edge]
    fn read(name: &str) -> Option<Self> {
        match name {
            "source" => Some(Self::Source),
            "target" => Some(Self::Target),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Target => "target",
        }
    }
}

/// A member of an end, converted once at read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EndMember {
    Population,
    Key,
}

impl EndMember {
    const ALL: [Self; 2] = [Self::Population, Self::Key];

    #[qsl_attrs::string_edge]
    fn read(name: &str) -> Option<Self> {
        match name {
            "population" => Some(Self::Population),
            "key" => Some(Self::Key),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Population => "population",
            Self::Key => "key",
        }
    }
}

fn defect(cause: &'static str, member: &'static str) -> AdmissionRecord {
    AdmissionRecord::new(Code::InvalidRuntimeInput, cause).with("member", member)
}

/// Reads a snapshot's optional `relationships` member, charging one value per
/// instance against `observation.values`.
///
/// # Errors
///
/// A limit refusal, or a `relationships` member that is not an array.
pub(super) fn read_relationships(
    object: Object<'_>,
    meter: &mut ReadMeter,
) -> Result<Vec<RawEntry>, AdmissionFailure> {
    let Some(member) = object.member("relationships") else {
        return Ok(Vec::new());
    };
    let items = member.as_array().ok_or_else(|| {
        refuse(
            AdmissionRecord::new(Code::InvalidRuntimeInput, "wrong-value-kind")
                .with("field", "relationships"),
        )
    })?;
    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        entries.push(read_entry(item, meter)?);
    }
    Ok(entries)
}

fn read_entry(item: Json<'_>, meter: &mut ReadMeter) -> Result<RawEntry, AdmissionFailure> {
    let Some(entry) = item.as_object() else {
        return Ok(RawEntry(Err(AdmissionRecord::new(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
        )
        .with("field", "relationships"))));
    };
    for member in EntryMember::ALL {
        if !entry.contains_key(member.name()) {
            return Ok(RawEntry(Err(defect("missing-member", member.name()))));
        }
    }
    for (name, _) in entry.iter() {
        if EntryMember::read(name).is_none() {
            return Ok(RawEntry(Err(AdmissionRecord::new(
                Code::InvalidRuntimeInput,
                "unknown-member",
            )
            .with("member", name))));
        }
    }
    let relationship = entry
        .member(EntryMember::Relationship.name())
        .and_then(Json::as_str);
    let complete = entry
        .member(EntryMember::Complete.name())
        .and_then(Json::as_bool);
    let instances = entry
        .member(EntryMember::Instances.name())
        .and_then(Json::as_array);
    let present = [
        (EntryMember::Relationship, relationship.is_some()),
        (EntryMember::Complete, complete.is_some()),
        (EntryMember::Instances, instances.is_some()),
    ];
    let (Some(relationship), Some(complete), Some(instances)) = (relationship, complete, instances)
    else {
        let wrong = present
            .into_iter()
            .find_map(|(member, ok)| (!ok).then_some(member))
            .map_or("relationship", EntryMember::name);
        let mut record = defect("wrong-value-kind", wrong);
        if let Some(name) = entry
            .member(EntryMember::Relationship.name())
            .and_then(Json::as_str)
        {
            record = record.with("relationship", name);
        }
        return Ok(RawEntry(Err(record)));
    };
    let mut read = Vec::with_capacity(instances.len());
    for instance in instances {
        meter.value()?;
        read.push(read_instance(instance));
    }
    Ok(RawEntry(Ok(RawRelationship {
        relationship: relationship.to_owned(),
        complete,
        instances: read,
    })))
}

fn read_instance(value: Json<'_>) -> Result<RawInstance, AdmissionRecord> {
    let Some(instance) = value.as_object() else {
        return Err(AdmissionRecord::new(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
        ));
    };
    for member in InstanceMember::ALL {
        if !instance.contains_key(member.name()) {
            return Err(defect("missing-member", member.name()));
        }
    }
    for (name, _) in instance.iter() {
        if InstanceMember::read(name).is_none() {
            return Err(
                AdmissionRecord::new(Code::InvalidRuntimeInput, "unknown-member")
                    .with("member", name),
            );
        }
    }
    let end = |member: InstanceMember| {
        let value = instance
            .member(member.name())
            .ok_or_else(|| defect("missing-member", member.name()))?;
        read_end(value).map_err(|record| record.with("end", member.name()))
    };
    Ok(RawInstance {
        source: end(InstanceMember::Source)?,
        target: end(InstanceMember::Target)?,
    })
}

fn read_end(value: Json<'_>) -> Result<RawEnd, AdmissionRecord> {
    let Some(end) = value.as_object() else {
        return Err(AdmissionRecord::new(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
        ));
    };
    for member in EndMember::ALL {
        if !end.contains_key(member.name()) {
            return Err(defect("missing-member", member.name()));
        }
    }
    for (name, _) in end.iter() {
        if EndMember::read(name).is_none() {
            return Err(
                AdmissionRecord::new(Code::InvalidRuntimeInput, "unknown-member")
                    .with("member", name),
            );
        }
    }
    let text = |member: EndMember| {
        end.member(member.name())
            .and_then(Json::as_str)
            .map(str::to_owned)
            .ok_or_else(|| defect("wrong-value-kind", member.name()))
    };
    Ok(SelectedObject {
        population: text(EndMember::Population)?,
        key: text(EndMember::Key)?,
    })
}

// ---------------------------------------------------------------------------
// The model's facts
// ---------------------------------------------------------------------------

/// A population declaration of the package, as the manifest reads it.
#[derive(Clone, Debug)]
pub(super) struct PopulationDeclaration {
    key: DeclarationKey,
    covered: Vec<EffectiveId>,
}

/// What admission reads of the package declarations: the navigation
/// relationships with their ends, and the population declarations.
#[derive(Clone, Debug)]
pub(super) struct ModelFacts {
    navigation: Vec<NavigationRelationship<DeclarationKey>>,
    populations: Vec<PopulationDeclaration>,
}

/// The navigation relationships (both ends object types of the model) and the
/// population declarations of every admitted domain package.
pub(super) fn model_facts(views: &[ModelView], types: &TypeEnvironment) -> ModelFacts {
    let mut navigation = Vec::new();
    let mut populations = Vec::new();
    for view in views {
        let identities = view.view.type_identities();
        let object = |key: &DeclarationKey| {
            let id = *identities.get(key)?;
            types.object_type(id).map(|_| id)
        };
        for record in &view.view.domain_package().records {
            match record {
                DomainPackageRecord::Relationship(relationship) => {
                    let (Some(source), Some(target)) = (
                        object(&relationship.source.type_identity),
                        object(&relationship.target.type_identity),
                    ) else {
                        continue;
                    };
                    // An end that declares no multiplicity declares no
                    // `unique: false`, so it is unique (FR-364 condition 4).
                    let end =
                        |end: &crate::model::domain_package::RelationshipEnd,
                         object_type: EffectiveId| RelationshipEnd {
                            object_type,
                            unique: end.multiplicity.is_none_or(|bound| bound.unique),
                        };
                    navigation.push(NavigationRelationship {
                        key: relationship.key.clone(),
                        source: end(&relationship.source, source),
                        target: end(&relationship.target, target),
                    });
                }
                DomainPackageRecord::Population(population) => {
                    populations.push(PopulationDeclaration {
                        key: population.key.clone(),
                        covered: population
                            .member_types
                            .iter()
                            .filter_map(|member| identities.get(member).copied())
                            .collect(),
                    });
                }
                _ => {}
            }
        }
    }
    populations.sort_by(|left, right| left.key.cmp(&right.key));
    ModelFacts {
        navigation,
        populations,
    }
}

impl ModelFacts {
    fn relationship(&self, node: &str) -> Option<&NavigationRelationship<DeclarationKey>> {
        self.navigation
            .iter()
            .find(|relationship| relationship.key.node == node)
    }
}

/// The traversal of the package's population declarations: one row per
/// declaration, `complete` as the snapshot lists it.
struct PackagePopulations<'a> {
    rows: std::slice::Iter<'a, PopulationDeclaration>,
    completeness: &'a BTreeMap<String, bool>,
}

impl PopulationEnumerator<DeclarationKey> for PackagePopulations<'_> {
    type Error = PopulationEnumerationFailure;

    fn next_row(
        &mut self,
    ) -> Result<Option<PopulationEntry<DeclarationKey>>, PopulationEnumerationFailure> {
        let Some(declaration) = self.rows.next() else {
            return Ok(None);
        };
        let complete = self.completeness.get(&declaration.key.node) == Some(&true);
        PopulationEntry::try_new(
            declaration.key.clone(),
            declaration.covered.iter().copied(),
            complete,
        )
        .map(Some)
        .map_err(|_| PopulationEnumerationFailure {
            declaration: declaration.key.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Check 6.6 to 6.9
// ---------------------------------------------------------------------------

/// One end of an admitted instance: the document's name for it, and the
/// reference when the snapshot lists the object.
#[derive(Clone, Debug)]
pub(super) struct CheckedEnd {
    population: String,
    key: String,
    reference: Option<ObjectReference>,
}

/// One admitted instance of an entry.
#[derive(Clone, Debug)]
pub(super) struct CheckedInstance {
    source: CheckedEnd,
    target: CheckedEnd,
}

/// One entry that passed checks 6.6 to 6.9.
#[derive(Clone, Debug)]
pub(super) struct CheckedExtent {
    relationship: DeclarationKey,
    complete: bool,
    instances: Vec<CheckedInstance>,
}

/// The objects a snapshot lists, by population and key.
pub(super) type Listed = BTreeMap<String, BTreeMap<String, ObjectReference>>;

fn record(cause: &'static str, relationship: &str, instance: usize) -> AdmissionRecord {
    AdmissionRecord::new(Code::InvalidRuntimeInput, cause)
        .with("relationship", relationship)
        .with("instance", instance.to_string())
}

/// FR-106 checks 6.6 to 6.9, entry by entry and instance by instance, over a
/// snapshot's entries. `listed` is the objects checks 6.1 to 6.5 admitted.
///
/// # Errors
///
/// The first failing condition in walk order.
pub(super) fn check_entries(
    types: &TypeEnvironment,
    facts: &ModelFacts,
    entries: &[RawEntry],
    listed: &Listed,
) -> Result<Vec<CheckedExtent>, AdmissionFailure> {
    let mut admitted: Vec<CheckedExtent> = Vec::with_capacity(entries.len());
    let declared_populations: BTreeSet<&str> = facts
        .populations
        .iter()
        .map(|population| population.key.node.as_str())
        .collect();
    for entry in entries {
        // 6.6: the entry's shape.
        let raw = entry.0.as_ref().map_err(|record| refuse(record.clone()))?;
        // 6.7: a navigation relationship of the package, listed once.
        let Some(declared) = facts.relationship(&raw.relationship) else {
            return Err(refuse(
                AdmissionRecord::new(Code::InvalidRuntimeInput, "wrong-role-mapping")
                    .with("relationship", raw.relationship.clone()),
            ));
        };
        if admitted
            .iter()
            .any(|held| held.relationship == declared.key)
        {
            return Err(refuse(
                AdmissionRecord::new(Code::InvalidRuntimeInput, "conflicting-identity")
                    .with("relationship", raw.relationship.clone()),
            ));
        }
        let repeatable = !declared.source.unique && !declared.target.unique;
        let mut instances: Vec<CheckedInstance> = Vec::with_capacity(raw.instances.len());
        let mut seen: BTreeSet<[&str; 4]> = BTreeSet::new();
        for (index, instance) in raw.instances.iter().enumerate() {
            let instance = instance.as_ref().map_err(|defect| {
                refuse(
                    defect
                        .clone()
                        .with("relationship", raw.relationship.clone())
                        .with("instance", index.to_string()),
                )
            })?;
            // 6.8: the ends name populations of the package, and a listed
            // object conforms to its end's type.
            for (side, end) in [
                (EndSide::Source, &instance.source),
                (EndSide::Target, &instance.target),
            ] {
                if !declared_populations.contains(end.population.as_str()) {
                    return Err(refuse(
                        record("wrong-role-mapping", &raw.relationship, index)
                            .with("end", end_name(side))
                            .with("population", end.population.clone()),
                    ));
                }
            }
            let source = checked_end(&instance.source, listed);
            let target = checked_end(&instance.target, listed);
            for (side, end, declared_end) in [
                (EndSide::Source, &source, declared.source),
                (EndSide::Target, &target, declared.target),
            ] {
                let Some(reference) = &end.reference else {
                    continue;
                };
                if !types.conforms(reference.object_type(), declared_end.object_type) {
                    return Err(refuse(
                        record("wrong-value-kind", &raw.relationship, index)
                            .with("end", end_name(side))
                            .with("object", end.key.clone())
                            .with("population", end.population.clone()),
                    ));
                }
            }
            // 6.9: a repeat of an earlier instance of the entry.
            if !repeatable
                && !seen.insert([
                    instance.source.population.as_str(),
                    instance.source.key.as_str(),
                    instance.target.population.as_str(),
                    instance.target.key.as_str(),
                ])
            {
                return Err(refuse(record(
                    "conflicting-identity",
                    &raw.relationship,
                    index,
                )));
            }
            instances.push(CheckedInstance { source, target });
        }
        admitted.push(CheckedExtent {
            relationship: declared.key.clone(),
            complete: raw.complete,
            instances,
        });
    }
    Ok(admitted)
}

fn checked_end(end: &RawEnd, listed: &Listed) -> CheckedEnd {
    CheckedEnd {
        population: end.population.clone(),
        key: end.key.clone(),
        reference: listed
            .get(&end.population)
            .and_then(|objects| objects.get(&end.key))
            .cloned(),
    }
}

// ---------------------------------------------------------------------------
// Checks 7 and 8
// ---------------------------------------------------------------------------

/// The populations an instance of an entry makes required: the population of
/// an end whose other end is in a required population (FR-106 "required
/// population").
pub(super) fn extend_required(
    required: &mut std::collections::BTreeSet<String>,
    extents: &[CheckedExtent],
) -> bool {
    let mut grew = false;
    for extent in extents {
        for instance in &extent.instances {
            let (source, target) = (&instance.source.population, &instance.target.population);
            if required.contains(source) && required.insert(target.clone()) {
                grew = true;
            }
            if required.contains(target) && required.insert(source.clone()) {
                grew = true;
            }
        }
    }
    grew
}

/// The population of every instance end, in walk order.
pub(super) fn end_populations(extents: &[CheckedExtent]) -> impl Iterator<Item = &str> {
    extents.iter().flat_map(|extent| {
        extent.instances.iter().flat_map(|instance| {
            [
                instance.source.population.as_str(),
                instance.target.population.as_str(),
            ]
        })
    })
}

fn ends_of(instance: &CheckedInstance) -> [(EndSide, &CheckedEnd); 2] {
    [
        (EndSide::Source, &instance.source),
        (EndSide::Target, &instance.target),
    ]
}

fn end_record(
    record: AdmissionRecord,
    extent: &CheckedExtent,
    index: usize,
    side: EndSide,
    end: &CheckedEnd,
) -> AdmissionRecord {
    record
        .with("relationship", extent.relationship.node.clone())
        .with("instance", index.to_string())
        .with("end", end_name(side))
        .with("population", end.population.clone())
        .with("object", end.key.clone())
}

/// FR-106 check 7, last rule: an instance end that names a key its
/// population does not list, where the population is not complete, has no
/// object to give its reference key a type, so the observation is incomplete.
///
/// # Errors
///
/// `Incomplete` naming the first such end's population.
pub(super) fn check_unlisted_ends(
    extents: &[CheckedExtent],
    completeness: &BTreeMap<String, bool>,
) -> Result<(), AdmissionFailure> {
    for extent in extents {
        for (index, instance) in extent.instances.iter().enumerate() {
            for (side, end) in ends_of(instance) {
                if end.reference.is_none() && completeness.get(&end.population) != Some(&true) {
                    return Err(incomplete(end_record(
                        AdmissionRecord::new(Code::IncompletePopulation, "incomplete-scope"),
                        extent,
                        index,
                        side,
                        end,
                    )));
                }
            }
        }
    }
    Ok(())
}

/// FR-106 check 8, instance ends: an end that names a key absent from its
/// complete population is a dangling reference.
///
/// # Errors
///
/// `dangling_reference` naming the relationship, the instance, the end, the
/// key and the population.
pub(super) fn check_dangling_ends(extents: &[CheckedExtent]) -> Result<(), AdmissionFailure> {
    for extent in extents {
        for (index, instance) in extent.instances.iter().enumerate() {
            for (side, end) in ends_of(instance) {
                if end.reference.is_none() {
                    return Err(refuse(end_record(
                        AdmissionRecord::new(
                            Code::DanglingReference,
                            "absent-target-in-complete-population",
                        ),
                        extent,
                        index,
                        side,
                        end,
                    )));
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The hand-off to the store
// ---------------------------------------------------------------------------

/// Hands the admitted entries to the extent store (FR-364), with the
/// traversal of the package's population declarations.
///
/// # Errors
///
/// The store's own refusal of a condition, or a `Fault` when an end of an
/// admitted instance still names no object, which checks 7 and 8 exclude.
pub(super) fn build_extents(
    types: &TypeEnvironment,
    facts: ModelFacts,
    extents: &[CheckedExtent],
    completeness: &BTreeMap<String, bool>,
) -> Result<RelationshipExtents, AdmissionFailure> {
    let mut supplied = Vec::with_capacity(extents.len());
    for extent in extents {
        let mut instances = Vec::with_capacity(extent.instances.len());
        for instance in &extent.instances {
            let (Some(source), Some(target)) =
                (&instance.source.reference, &instance.target.reference)
            else {
                return Err(fault("instance-end-unresolved-after-checks-7-and-8"));
            };
            instances.push((source.clone(), target.clone()));
        }
        supplied.push(RelationshipExtent {
            relationship: extent.relationship.clone(),
            complete: extent.complete,
            instances,
        });
    }
    let row_limit = facts.populations.len();
    let mut enumerator = PackagePopulations {
        rows: facts.populations.iter(),
        completeness,
    };
    RelationshipExtents::admit(
        types,
        facts.navigation.iter().cloned(),
        supplied,
        &mut enumerator,
        row_limit,
    )
}

// ---------------------------------------------------------------------------
// Check 11.5: the frame over relationship instances
// ---------------------------------------------------------------------------

/// The instance FR-106 check 11.5 found changed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct LinkFinding {
    pub(super) relationship: DeclarationKey,
    pub(super) difference: LinkDifference,
    pub(super) source: SelectedObject,
    pub(super) target: SelectedObject,
}

/// One instance of an extent as the frame compares it: its ends by
/// reference key, with the document's names for them.
type Link = (
    (ObjectReference, SelectedObject),
    (ObjectReference, SelectedObject),
);

/// The extents one snapshot lists, by relationship, with whether each is
/// complete.
type Extents = BTreeMap<DeclarationKey, (bool, Vec<Link>)>;

fn side_extents(
    views: &[ModelView],
    facts: &ModelFacts,
    populations: &[RawPopulation],
    entries: &[RawEntry],
) -> Result<Extents, AdmissionFailure> {
    let mut listed: BTreeMap<(&str, &str), ObjectReference> = BTreeMap::new();
    for population in populations {
        for object in &population.objects {
            let (view, declaration) = find_declaration(views, object.type_identity.as_str())
                .ok_or_else(|| fault("frame-link-object-type-unresolved"))?;
            let effective_type = *view
                .view
                .type_identities()
                .get(&declaration)
                .ok_or_else(|| fault("frame-link-object-type-has-no-effective-id"))?;
            listed.insert(
                (population.population.as_str(), object.key.as_str()),
                object_reference(views, effective_type, &object.key)?,
            );
        }
    }
    let end = |raw: &RawEnd| -> Result<(ObjectReference, SelectedObject), AdmissionFailure> {
        let reference = listed
            .get(&(raw.population.as_str(), raw.key.as_str()))
            .ok_or_else(|| fault("frame-link-end-unlisted"))?;
        Ok((reference.clone(), raw.clone()))
    };
    let mut extents = Extents::new();
    for entry in entries {
        let raw = entry
            .0
            .as_ref()
            .map_err(|_| fault("frame-link-entry-unreadable"))?;
        let Some(declared) = facts.relationship(&raw.relationship) else {
            return Err(fault("frame-link-relationship-undeclared"));
        };
        let mut links = Vec::with_capacity(raw.instances.len());
        for instance in &raw.instances {
            let instance = instance
                .as_ref()
                .map_err(|_| fault("frame-link-instance-unreadable"))?;
            links.push((end(&instance.source)?, end(&instance.target)?));
        }
        extents.insert(declared.key.clone(), (raw.complete, links));
    }
    Ok(extents)
}

/// FR-106 check 11.5: of a relationship both snapshots list complete and the
/// frame's `modifies` does not name, the first instance, in ascending order
/// of source then target reference key, whose count differs between the pre
/// and the post extent. Relationships are walked in ascending
/// `DeclarationKey` order; an instance with an end at an object the
/// invocation deletes is left out.
///
/// # Errors
///
/// A `Fault` when an admitted snapshot's entry cannot be read again.
pub(super) fn first_link_change(
    views: &[ModelView],
    types: &TypeEnvironment,
    pre: (&[RawPopulation], &[RawEntry]),
    post: (&[RawPopulation], &[RawEntry]),
    effect: &OperationEffect,
    deleted: &[SelectedObject],
) -> Result<Option<LinkFinding>, AdmissionFailure> {
    let facts = model_facts(views, types);
    let pre = side_extents(views, &facts, pre.0, pre.1)?;
    let post = side_extents(views, &facts, post.0, post.1)?;
    let deleted: BTreeSet<(&str, &str)> = deleted
        .iter()
        .map(|object| (object.population.as_str(), object.key.as_str()))
        .collect();
    let is_deleted =
        |end: &SelectedObject| deleted.contains(&(end.population.as_str(), end.key.as_str()));
    for (relationship, (pre_complete, pre_links)) in &pre {
        let Some((post_complete, post_links)) = post.get(relationship) else {
            continue;
        };
        if !(*pre_complete && *post_complete) || effect.modifies.contains(relationship) {
            continue;
        }
        // Per instance: its count before and after, and the ends' names.
        let mut counts: BTreeMap<(&ObjectReference, &ObjectReference), (usize, usize, &Link)> =
            BTreeMap::new();
        for (links, after) in [(pre_links, false), (post_links, true)] {
            for link in links {
                let ((source, source_name), (target, target_name)) = link;
                if is_deleted(source_name) || is_deleted(target_name) {
                    continue;
                }
                let held = counts.entry((source, target)).or_insert((0, 0, link));
                if after {
                    held.1 += 1;
                } else {
                    held.0 += 1;
                }
            }
        }
        for (before, after, link) in counts.values() {
            if before == after {
                continue;
            }
            let ((_, source), (_, target)) = link;
            return Ok(Some(LinkFinding {
                relationship: relationship.clone(),
                difference: if after > before {
                    LinkDifference::Added
                } else {
                    LinkDifference::Removed
                },
                source: source.clone(),
                target: target.clone(),
            }));
        }
    }
    Ok(None)
}
