// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-197 Y01 and the binder half of Y09 (FR-152-AC-1/3/5): a qualified name
//! `M::<artifact id>` resolves a systems construct to its kind, declaration
//! key and effective declaration, charges `systems.resolve` per segment after
//! the alias, and never resolves through an owner, a foreign alias or a
//! lookup outside the supplied inputs.
//!
//! Fixture Y is TC-197's own, in the `ix://test/orders/<artifact id>` node
//! form FR-154 gives a construct.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::model::accounting::{
    ChargePoint, Incomplete, LimitKind, Meter, ModelNormalizationLimits,
};
use qsl_semantics::model::admitted::AdmittedPackage;
use qsl_semantics::model::domain_package::{
    AllocationRecord, DomainPackage, DomainPackageRecord, DomainPackageRef, OperationEffect,
    OperationMemberRecord, PortDirection, RelationshipDirection, RelationshipRecord,
};
use qsl_semantics::model::key::{DeclarationKey, EffectiveId};
use qsl_semantics::model::normalize::{
    normalize, EffectiveView, ModelRefusal, ModelRefusalCause, NormalizeOutcome,
};
use qsl_semantics::model::qualified::{
    resolve_systems, ModelAlias, ResolveFailure, ResolvedSystemsElement,
};
use qsl_semantics::model::systems::{ConditionFailure, Kind};

use super::model_systems::{
    allocation, component, endpoint, field_member, mult, object_type, one, relationship,
    scalar_type,
};

fn id(artifact: &str) -> String {
    format!("ix://test/orders/{artifact}")
}

fn key(artifact: &str) -> DeclarationKey {
    DeclarationKey::fixture(id(artifact))
}

fn operation_run() -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: key("Pump/run"),
        owner: key("Pump"),
        parameters: vec![],
        result: None,
        effect: OperationEffect::default(),
        requires: Vec::new(),
        ensures: Vec::new(),
        has_body: true,
        redefines: None,
    })
}

fn fixture_y(skip: &[&str]) -> DomainPackage {
    let records = vec![
        scalar_type(&id("Count"), 0, 9),
        object_type(&id("Sys"), None, vec![]),
        object_type(&id("Pump"), None, vec![]),
        object_type(&id("Tank"), None, vec![]),
        object_type(&id("Flow"), Some(vec![&id("Flow/rate")]), vec![]),
        object_type(
            &id("Flow2"),
            Some(vec![&id("Flow/rate")]),
            vec![&id("Flow")],
        ),
        field_member(&id("Flow/rate"), &id("Flow"), &id("Count"), one()),
        component(&id("sys_pump"), &id("Sys"), &id("Pump"), one()),
        component(&id("sys_tank"), &id("Sys"), &id("Tank"), one()),
        endpoint(
            &id("pump_out"),
            &id("sys_pump"),
            &id("Flow"),
            Some(PortDirection::Out),
            one(),
        ),
        endpoint(
            &id("tank_in"),
            &id("sys_tank"),
            &id("Flow"),
            Some(PortDirection::In),
            one(),
        ),
        relationship(
            &id("pipe"),
            &id("pump_out"),
            one(),
            &id("tank_in"),
            one(),
            RelationshipDirection::SourceToTarget,
        ),
        operation_run(),
        allocation(&id("pump_alloc"), &id("Pump/run"), &id("sys_pump")),
    ];
    let records = records
        .into_iter()
        .filter(|record| !skip.contains(&record.key().node.as_str()))
        .collect();
    DomainPackage::new(DomainPackageRef::fixture("y"), records)
}

fn fixture_with(mutate: impl FnOnce(&mut Vec<DomainPackageRecord>)) -> DomainPackage {
    let mut package = fixture_y(&[]);
    mutate(&mut package.records);
    package
}

struct World {
    admitted: AdmittedPackage,
    view: EffectiveView,
    selection: DomainPackageRef,
    admission_charges: Vec<ChargePoint>,
}

fn world_of(admitted_package: DomainPackage, viewed_package: &DomainPackage) -> World {
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    let selection = admitted_package.model_selection.clone();
    let admitted =
        AdmittedPackage::admit(admitted_package, &mut meter).expect("fixture Y is admitted");
    let NormalizeOutcome::Completed(view) =
        normalize(viewed_package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("fixture Y normalizes");
    };
    World {
        admitted,
        view,
        selection,
        admission_charges: meter.admitted_charges().to_vec(),
    }
}

fn world() -> World {
    let package = fixture_y(&[]);
    world_of(package.clone(), &package)
}

fn resolve(
    world: &World,
    aliases: &[ModelAlias<'_>],
    required: Kind,
    name: &str,
    limits: ModelNormalizationLimits,
) -> (Result<ResolvedSystemsElement, ResolveFailure>, Meter) {
    let mut meter = Meter::new(limits);
    let result = resolve_systems(
        &world.admitted,
        &world.view,
        aliases,
        required,
        name,
        &mut meter,
    );
    (result, meter)
}

fn resolve_m(
    world: &World,
    required: Kind,
    name: &str,
) -> (Result<ResolvedSystemsElement, ResolveFailure>, Meter) {
    let aliases = [ModelAlias {
        alias: "M",
        selection: &world.selection,
    }];
    resolve(
        world,
        &aliases,
        required,
        name,
        ModelNormalizationLimits::UNLIMITED,
    )
}

fn refused(result: Result<ResolvedSystemsElement, ResolveFailure>) -> Box<ModelRefusal> {
    match result {
        Err(ResolveFailure::Refused(refusal)) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The effective member of `original` owned by effective type `owner`,
/// found by the test directly in the view's declarations.
fn member_entry(view: &EffectiveView, owner: &str, original: &str) -> EffectiveId {
    let owner = view.type_identities()[&key(owner)];
    let entries: Vec<_> = view
        .declarations()
        .iter()
        .filter(|entry| {
            entry.preimage.owner_effective_type == Some(owner)
                && entry.preimage.original == key(original)
        })
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "{original} has one entry owned by {owner:?}"
    );
    entries[0].effective_id
}

/// TC-197 Y01: each supported kind resolves with its exact declaration key
/// and effective declaration. The Connection and the Allocation resolve as
/// the member owned by their source end-owner type: the Allocation is
/// qualified into both `Pump` and `Sys` and resolves to `Pump`'s.
///
/// Mutation: resolving a member at the target end-owner (`Sys` for the
/// Allocation) or returning the declaration key as the effective identity
/// fails the `effective` and `owner` assertions.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-1")]
#[test]
fn y01_each_systems_kind_resolves_with_declaration_key_and_effective_identity() {
    let world = world();
    assert_eq!(
        world.admission_charges,
        vec![ChargePoint::SystemsKind; 8],
        "kind mapping charges eight systems.kind"
    );
    let flow = world.view.type_identities()[&key("Flow")];
    let sys = world.view.type_identities()[&key("Sys")];
    let pump = world.view.type_identities()[&key("Pump")];
    let cases = [
        (
            Kind::Part,
            "sys_pump",
            Some(sys),
            member_entry(&world.view, "Sys", "sys_pump"),
        ),
        (
            Kind::Port,
            "pump_out",
            Some(sys),
            member_entry(&world.view, "Sys", "pump_out"),
        ),
        (Kind::Interface, "Flow", None, flow),
        (
            Kind::Connection,
            "pipe",
            Some(sys),
            member_entry(&world.view, "Sys", "pipe"),
        ),
        (
            Kind::Allocation,
            "pump_alloc",
            Some(pump),
            member_entry(&world.view, "Pump", "pump_alloc"),
        ),
    ];
    for (kind, artifact, owner, effective) in cases {
        let (result, meter) = resolve_m(&world, kind, &format!("M::{artifact}"));
        let resolved = result.unwrap_or_else(|failure| panic!("{artifact}: {failure:?}"));
        assert_eq!(
            resolved,
            ResolvedSystemsElement {
                kind,
                key: key(artifact),
                effective,
                owner,
            },
            "{artifact}"
        );
        // The resolve charge, then the admission rule's own charges: three
        // connection conditions for a Connection, one for an Allocation.
        let mut charges = vec![ChargePoint::SystemsResolve];
        match kind {
            Kind::Connection => {
                charges.extend([ChargePoint::SystemsConnectionCondition; 3]);
            }
            Kind::Allocation => charges.push(ChargePoint::SystemsAllocation),
            Kind::Part | Kind::Port | Kind::Interface | Kind::None => {}
        }
        assert_eq!(meter.admitted_charges(), charges, "{artifact}");
    }
    assert_ne!(
        member_entry(&world.view, "Pump", "pump_alloc"),
        member_entry(&world.view, "Sys", "pump_alloc"),
        "the Allocation's two owner entries are distinct declarations"
    );
}

/// TC-197 Y01: `M::Sys::pump_alloc` and `M::Sys::sys_pump` name nothing,
/// because a construct is named by its artifact id and never through an
/// owner. Both segments are charged before the second is refused.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-1", "QSpec-FR-152-AC-3")]
#[test]
fn y01_a_construct_is_never_named_through_its_owner() {
    let world = world();
    for (kind, name) in [
        (Kind::Allocation, "M::Sys::pump_alloc"),
        (Kind::Part, "M::Sys::sys_pump"),
    ] {
        let (result, meter) = resolve_m(&world, kind, name);
        let refusal = refused(result);
        assert_eq!(refusal.code, Code::MissingDeclaration, "{name}");
        assert_eq!(
            refusal.cause,
            ModelRefusalCause::UnresolvedName {
                name: name.to_owned(),
                segment: 2,
            }
        );
        assert_eq!(
            meter.admitted_charges(),
            [ChargePoint::SystemsResolve; 2],
            "{name}"
        );
    }
}

/// FR-152-AC-3: a name that no declaration answers, and a construct the
/// supplied effective view does not hold, refuse without falling back to
/// another owner's entry or to a same-named declaration.
///
/// Mutation: falling back to any entry whose original matches regardless of
/// owner resolves `pump_alloc` through `Sys`'s entry and fails the refusal.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-3")]
#[test]
fn an_unresolvable_name_refuses_without_ambient_lookup() {
    let world = world();
    let (result, meter) = resolve_m(&world, Kind::Part, "M::sys_pumpp");
    assert_eq!(
        refused(result).cause,
        ModelRefusalCause::UnresolvedName {
            name: "M::sys_pumpp".to_owned(),
            segment: 1,
        }
    );
    assert_eq!(meter.admitted_charges(), [ChargePoint::SystemsResolve]);

    let (result, _) = resolve_m(&world, Kind::Part, "pump_out");
    assert_eq!(
        refused(result).cause,
        ModelRefusalCause::UnresolvedName {
            name: "pump_out".to_owned(),
            segment: 0,
        },
        "an unqualified name has no alias to resolve under"
    );

    // The view holds no entry for the Allocation at all.
    let full = fixture_y(&[]);
    let without = fixture_y(&[&id("pump_alloc")]);
    let world = world_of(full, &without);
    let (result, _) = resolve_m(&world, Kind::Allocation, "M::pump_alloc");
    assert_eq!(
        refused(result).cause,
        ModelRefusalCause::UnresolvedName {
            name: "M::pump_alloc".to_owned(),
            segment: 1,
        }
    );
}

/// An alias that is not one of the unit's model aliases binds nothing.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-3")]
#[test]
fn an_unbound_alias_resolves_nothing_and_charges_nothing() {
    let world = world();
    let (result, meter) = resolve_m(&world, Kind::Port, "X::pump_out");
    assert_eq!(
        refused(result).cause,
        ModelRefusalCause::UnresolvedName {
            name: "X::pump_out".to_owned(),
            segment: 0,
        }
    );
    assert!(meter.admitted_charges().is_empty());
}

/// FR-152-AC-5: a qualified name whose alias names another ModelSelection
/// than the one that owns the declarations is refused before any segment is
/// charged, even though the other selection's artifact id equals one the
/// owning selection declares.
///
/// Mutation: resolving the artifact id without comparing the alias's
/// selection resolves `N::pump_out` against the owning package and fails
/// the refusal.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-5")]
#[test]
fn an_alias_naming_another_model_selection_is_foreign() {
    let world = world();
    let other = DomainPackageRef::fixture("other");
    assert_ne!(other, world.selection);
    let aliases = [
        ModelAlias {
            alias: "M",
            selection: &world.selection,
        },
        ModelAlias {
            alias: "N",
            selection: &other,
        },
    ];
    let (result, meter) = resolve(
        &world,
        &aliases,
        Kind::Port,
        "N::pump_out",
        ModelNormalizationLimits::UNLIMITED,
    );
    let refusal = refused(result);
    assert_eq!(refusal.code, Code::ForeignReference);
    assert_eq!(
        refusal.cause,
        ModelRefusalCause::ForeignAlias {
            alias: "N".to_owned(),
            offered: Box::new(other.clone()),
            expected: Box::new(world.selection.clone()),
        }
    );
    assert!(meter.admitted_charges().is_empty());

    // The same artifact id under the owning alias still resolves.
    let (result, _) = resolve(
        &world,
        &aliases,
        Kind::Port,
        "M::pump_out",
        ModelNormalizationLimits::UNLIMITED,
    );
    assert_eq!(result.expect("M::pump_out resolves").key, key("pump_out"));
}

/// FR-152-AC-5: identity is the construct's declaration, never its name. A
/// request for a kind the named construct does not have refuses
/// `wrong-export` even though the name resolves.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-2")]
#[test]
fn a_name_resolving_to_another_kind_is_wrong_export() {
    let world = world();
    for (required, name) in [(Kind::Port, "M::sys_pump"), (Kind::Allocation, "M::pipe")] {
        let (result, meter) = resolve_m(&world, required, name);
        let refusal = refused(result);
        assert_eq!(refusal.code, Code::InvalidModelBinding, "{name}");
        assert_eq!(refusal.cause, ModelRefusalCause::WrongExport, "{name}");
        assert_eq!(meter.admitted_charges(), [ChargePoint::SystemsResolve]);
    }
}

/// The `systems.resolve` charge is made before the segment is resolved, so a
/// denied charge resolves nothing.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-1")]
#[test]
fn a_denied_systems_resolve_charge_is_incomplete() {
    let world = world();
    let aliases = [ModelAlias {
        alias: "M",
        selection: &world.selection,
    }];
    let limits = ModelNormalizationLimits {
        work_units: 0,
        ..ModelNormalizationLimits::UNLIMITED
    };
    let (result, _) = resolve(&world, &aliases, Kind::Part, "M::sys_pump", limits);
    assert_eq!(
        result,
        Err(ResolveFailure::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 0,
            consumed: 0,
            next_charge: 1,
            charge_point: ChargePoint::SystemsResolve,
        }))
    );
}

fn world_with(mutate: impl FnOnce(&mut Vec<DomainPackageRecord>)) -> World {
    let package = fixture_with(mutate);
    world_of(package.clone(), &package)
}

fn pipe(records: &mut [DomainPackageRecord]) -> &mut RelationshipRecord {
    records
        .iter_mut()
        .find_map(|record| match record {
            DomainPackageRecord::Relationship(r) if r.key == key("pipe") => Some(r),
            _ => None,
        })
        .expect("fixture Y has pipe")
}

fn pump_alloc(records: &mut [DomainPackageRecord]) -> &mut AllocationRecord {
    records
        .iter_mut()
        .find_map(|record| match record {
            DomainPackageRecord::Allocation(a) if a.key == key("pump_alloc") => Some(a),
            _ => None,
        })
        .expect("fixture Y has pump_alloc")
}

fn connection_failures(
    result: Result<ResolvedSystemsElement, ResolveFailure>,
) -> Vec<ConditionFailure> {
    match result {
        Err(ResolveFailure::Connection(failures)) => failures,
        other => panic!("expected the connection conditions to refuse, got {other:?}"),
    }
}

/// TC-197 Y02: an Allocation whose target is a Port, not a Part, does not
/// resolve: admission's allocation rule refuses it `wrong-export`.
///
/// Mutation: resolving on the kind mapping alone returns the Allocation.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-1", "QSpec-FR-152-AC-2")]
#[test]
fn y02_an_allocation_targeting_a_port_does_not_resolve() {
    let world = world_with(|records| pump_alloc(records).target_element = key("pump_out"));
    let (result, meter) = resolve_m(&world, Kind::Allocation, "M::pump_alloc");
    let refusal = refused(result);
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert_eq!(refusal.cause, ModelRefusalCause::WrongExport);
    assert_eq!(
        meter.admitted_charges(),
        [ChargePoint::SystemsResolve, ChargePoint::SystemsAllocation]
    );
}

/// TC-197 Y03(a): a Connection whose source is the `in` port and whose
/// target is the `out` port does not resolve: `port-direction`.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-1", "QSpec-FR-152-AC-4")]
#[test]
fn y03a_a_connection_against_port_direction_does_not_resolve() {
    let world = world_with(|records| {
        let pipe = pipe(records);
        pipe.source.type_identity = key("tank_in");
        pipe.target.type_identity = key("pump_out");
    });
    let (result, meter) = resolve_m(&world, Kind::Connection, "M::pipe");
    let failures = connection_failures(result);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].code, Code::InvalidModelBinding);
    assert_eq!(
        failures[0].cause,
        ModelRefusalCause::PortDirection {
            source: key("tank_in"),
            target: key("pump_out"),
        }
    );
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .filter(|point| **point == ChargePoint::SystemsConnectionCondition)
            .count(),
        3
    );
}

/// TC-197 Y05: a Connection whose target end multiplicity `{0,2}` does not
/// conform to its port's `{1,1}` does not resolve: `multiplicity-narrowing`.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-1", "QSpec-FR-152-AC-4")]
#[test]
fn y05_a_connection_narrowing_its_port_multiplicity_does_not_resolve() {
    let world = world_with(|records| pipe(records).target.multiplicity = Some(mult(0, Some(2))));
    let (result, _) = resolve_m(&world, Kind::Connection, "M::pipe");
    let failures = connection_failures(result);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].code, Code::IllTyped);
    assert!(matches!(
        failures[0].cause,
        ModelRefusalCause::MultiplicityNarrowing { .. }
    ));
}

/// An Allocation whose source element is not a Part, Port or operation has
/// no source end-owner. It refuses `wrong-export` rather than taking the
/// target end's owner, which the view also holds an entry for.
///
/// Mutation: taking the first of the source and target owners resolves it
/// through the target's `Sys` entry.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-1")]
#[test]
fn an_allocation_without_a_source_owner_never_takes_the_target_owner() {
    let world = world_with(|records| pump_alloc(records).source_element = key("Count"));
    let (result, _) = resolve_m(&world, Kind::Allocation, "M::pump_alloc");
    let refusal = refused(result);
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert_eq!(refusal.cause, ModelRefusalCause::WrongExport);
}

/// FR-152-AC-3: when the view holds only another owner's entry for the
/// construct (here `Sys`'s, from a package whose Allocation starts at
/// `sys_pump`), the name refuses; it never matches an entry of any owner.
///
/// Mutation: dropping the owner from the entry match resolves it.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-3")]
#[test]
fn a_view_holding_only_another_owners_entry_refuses() {
    let admitted = fixture_y(&[]);
    let viewed = fixture_with(|records| pump_alloc(records).source_element = key("sys_pump"));
    let sys = viewed_owner_ids(&viewed);
    assert_eq!(
        sys, 1,
        "the viewed package qualifies the Allocation once, under Sys"
    );
    let world = world_of(admitted, &viewed);
    let (result, _) = resolve_m(&world, Kind::Allocation, "M::pump_alloc");
    assert_eq!(
        refused(result).cause,
        ModelRefusalCause::UnresolvedName {
            name: "M::pump_alloc".to_owned(),
            segment: 1,
        }
    );
}

fn viewed_owner_ids(package: &DomainPackage) -> usize {
    let NormalizeOutcome::Completed(view) = normalize(package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("normalizes");
    };
    view.declarations()
        .iter()
        .filter(|entry| entry.preimage.original == key("pump_alloc"))
        .count()
}
