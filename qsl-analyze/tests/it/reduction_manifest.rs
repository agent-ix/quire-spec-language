// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-159 "Capability" (TC-583) and QSpec FR-430: EN-1 and EN-6 advertise
//! the reductions they apply in their provider manifests, and negotiation
//! routes an item that selects reductions only to a candidate advertising
//! every one of them, naming the first one no candidate left out
//! advertises when none does. Negotiation never drops, adds or replaces a
//! selected reduction.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ix_trace_rs::trace;
use qsl_analyze::model_check::manifest as en1;
use qsl_analyze::zone_check::manifest as en6;
use qsl_foundation::bound::DomainKind;
use qsl_foundation::{BackendId, ReductionKind};
use qsl_replay::UnavailabilityCause;
use qsl_route::reductions::Reductions;
use qsl_route::request::{items_with_reductions, EvidenceSelection};
use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, CandidateOutcome, FairnessKindsMember,
    ManifestMembers, Mode, ProviderOrigin, ReductionsDefect, RegistrationCause, Registry,
};
use qsl_semantics::check::Capability;
use serde_json::{json, Value as Json};

use ReductionKind::{PartialOrder, StateConstraint, Symmetry};

/// A temporal backend `id` advertising `reductions` as its manifest spells
/// them.
fn temporal(
    id: &str,
    reductions: Option<&[&str]>,
) -> Result<BackendDescriptor, qsl_route::RegistrationRefusal> {
    descriptor_from_manifest(
        Candidate::new(BackendId::new(id)),
        ProviderOrigin::Linked,
        [(
            Some(Capability::TemporalSatisfaction.to_wire()),
            Some(Mode::Bounded.to_wire()),
        )],
        ManifestMembers {
            domains: Some(&[DomainKind::Population.to_wire()]),
            fairness_kinds: Some(FairnessKindsMember::List(&[])),
            evidence_kinds: None,
            memory_models: None,
            reductions,
        },
    )
}

/// The candidates of a temporal item selecting `selected`, and the
/// reduction that emptied the set, if any.
fn routed(registry: &Registry, selected: &[ReductionKind]) -> (Vec<String>, Option<ReductionKind>) {
    let outcome = registry.restrict_to_reductions(
        registry.candidates_for_temporal_dynamics(
            Capability::TemporalSatisfaction,
            None,
            &BTreeSet::new(),
            &BTreeSet::new(),
            false,
        ),
        &selected.iter().copied().collect(),
    );
    let CandidateOutcome::Candidates(set) = outcome else {
        panic!("a computed candidate set");
    };
    (
        set.candidates()
            .iter()
            .map(|candidate| candidate.id().as_str().to_owned())
            .collect(),
        set.missing_reduction(),
    )
}

/// The V-8 cause an emptied set settles, and its wire spelling.
fn settled(missing: Option<ReductionKind>) -> &'static str {
    UnavailabilityCause::UnsupportedReduction {
        reduction: missing.expect("a reduction emptied the set"),
    }
    .as_str()
}

/// FR-159-AC-3's second half: a registry whose only model-check candidate
/// advertises `symmetry` alone settles a request selecting partial-order
/// reduction V-8, naming it, and routes a request selecting symmetry to
/// that candidate; the first is never routed to it.
#[trace("TC-583", "FR-159-AC-3")]
#[test]
fn a_reduction_no_candidate_advertises_settles_unsupported() {
    let mut registry = Registry::new();
    registry
        .register(temporal("sym-only", Some(&["symmetry"])).expect("admitted"))
        .expect("registered");
    let (candidates, missing) = routed(&registry, &[PartialOrder]);
    assert!(candidates.is_empty());
    assert_eq!(missing, Some(PartialOrder));
    assert_eq!(settled(missing), "unsupported-requested-capability");
    assert_eq!(
        routed(&registry, &[Symmetry]),
        (vec!["sym-only".to_owned()], None)
    );
    assert_eq!(routed(&registry, &[]), (vec!["sym-only".to_owned()], None));
}

/// FR-159-AC-5: EN-1 advertises `symmetry`, `partial-order` and
/// `constraint`. A registry whose only candidate advertises `symmetry` and
/// `partial-order` settles a request selecting the constraint V-8, naming
/// it; with EN-1 registered beside it, the same request routes to EN-1
/// only.
#[trace("TC-583", "FR-159-AC-5")]
#[test]
fn en1_advertises_every_reduction_and_takes_the_constraint() {
    let en1 = en1::descriptor(ProviderOrigin::Linked).expect("EN-1 admits");
    assert_eq!(
        en1.reductions(),
        &BTreeSet::from([Symmetry, PartialOrder, StateConstraint])
    );
    assert_eq!(en1::REDUCTIONS, ["symmetry", "partial-order", "constraint"]);
    let mut registry = Registry::new();
    registry
        .register(temporal("sym-por", Some(&["symmetry", "partial-order"])).expect("admitted"))
        .expect("registered");
    let (candidates, missing) = routed(&registry, &[StateConstraint]);
    assert!(candidates.is_empty());
    assert_eq!(missing, Some(StateConstraint));
    registry.register(en1).expect("registered");
    assert_eq!(
        routed(&registry, &[StateConstraint]),
        (vec![en1::BACKEND_ID.to_owned()], None)
    );
}

/// QSpec FR-430 (FR-430-AC-3): EN-6 advertises `symmetry` and no other
/// reduction, so negotiation settles an item over a dense-time subject
/// that selects partial-order reduction, a state constraint or both V-8,
/// naming partial-order reduction first, with no candidate run; one
/// selecting symmetry routes to EN-6.
#[trace("TC-1170", "FR-430-AC-3")]
#[test]
fn en6_advertises_symmetry_alone() {
    let en6 = en6::descriptor(ProviderOrigin::Linked).expect("EN-6 admits");
    assert_eq!(en6.reductions(), &BTreeSet::from([Symmetry]));
    let mut registry = Registry::new();
    registry.register(en6).expect("registered");
    for (selected, named) in [
        (&[PartialOrder][..], PartialOrder),
        (&[StateConstraint][..], StateConstraint),
        (&[PartialOrder, StateConstraint][..], PartialOrder),
        (&[Symmetry, StateConstraint][..], StateConstraint),
    ] {
        let (candidates, missing) = routed(&registry, selected);
        assert!(candidates.is_empty(), "{selected:?}");
        assert_eq!(missing, Some(named), "{selected:?}");
    }
    assert_eq!(
        routed(&registry, &[Symmetry]),
        (vec![en6::BACKEND_ID.to_owned()], None)
    );
}

/// FR-159 "Capability": a `reductions` member that is empty, holds
/// `state-constraint` (not a spelling of any reduction) or repeats a label
/// refuses the registration, retaining the defect.
#[trace("TC-583", "FR-159-AC-5")]
#[test]
fn a_defective_reductions_member_refuses_the_registration() {
    for (member, defect) in [
        (&[][..], ReductionsDefect::Empty),
        (
            &["symmetry", "state-constraint"][..],
            ReductionsDefect::UnknownLabel("state-constraint".to_owned()),
        ),
        (
            &["symmetry", "symmetry"][..],
            ReductionsDefect::DuplicateLabel("symmetry".to_owned()),
        ),
    ] {
        let refusal = temporal("bad", Some(member)).expect_err("refused");
        assert_eq!(
            refusal.cause(),
            &RegistrationCause::InvalidReductions(defect)
        );
    }
    let none = temporal("none", None).expect("admitted");
    assert!(none.reductions().is_empty());
}

/// EN-1's manifest entry as QSpec's backend-provider schema states one:
/// its published members, with `reductions` as given.
fn en1_entry(reductions: &[&str]) -> Json {
    json!({
        "backend": en1::BACKEND_ID,
        "tool": {
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        },
        "advertises": en1::ADVERTISED
            .iter()
            .map(|(kind, mode)| json!({"kind": kind, "mode": mode}))
            .collect::<Vec<_>>(),
        "domains": en1::DOMAINS,
        "fairness_kinds": ["weak", "strong"],
        "memory_models": en1::MEMORY_MODELS
            .iter()
            .map(|model| model.label())
            .collect::<Vec<_>>(),
        "reductions": reductions,
    })
}

/// FR-159-AC-5: EN-1's manifest entry, carrying `reductions` `["symmetry",
/// "partial-order", "constraint"]`, validates against QSpec's
/// backend-provider schema, read in place from `$QSPEC_DIR` (which the
/// test requires) with the
/// checked-package schema it refers to; the same
/// entry with `state-constraint` in place of `constraint` fails it.
#[trace("TC-583", "FR-159-AC-5")]
#[test]
fn en1_manifest_entry_validates_against_the_backend_provider_schema() {
    let qspec = std::env::var_os("QSPEC_DIR")
        .expect("QSPEC_DIR names the QSpec checkout whose schema this reads");
    let read = |relative: &str| -> Json {
        let path = PathBuf::from(&qspec).join(relative);
        serde_json::from_slice(&std::fs::read(&path).expect("the schema reads"))
            .expect("the schema is JSON")
    };
    let schema = read("proposals/backend-provider-v1/schema.json");
    // The backend-provider schema refers to the checked-package schema by
    // its `$id`.
    let package = read("proposals/checked-package-v2/schema.json");
    let descriptor = json!({
        "$schema": schema["$schema"],
        "$defs": schema["$defs"],
        "$ref": "#/$defs/BackendDescriptor",
    });
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .with_document("urn:quire:checked-package:v2".to_owned(), package)
        .compile(&descriptor)
        .expect("the schema compiles");
    let entry = en1_entry(&en1::REDUCTIONS);
    if let Err(errors) = compiled.validate(&entry) {
        let errors: Vec<_> = errors.map(|error| error.to_string()).collect();
        panic!("EN-1's entry violates the schema: {errors:?}");
    }
    assert!(!compiled.is_valid(&en1_entry(&[
        "symmetry",
        "partial-order",
        "state-constraint"
    ])));
}

/// FR-159 "Capability" through the request writer's real path: the items
/// of a checked package's requirement records, for a request selecting
/// reductions, are restricted to the candidates advertising every one of
/// them. With EN-6 alone registered, the §7.1 claim's item selecting
/// partial-order reduction has no candidate and settles V-8 naming it;
/// with EN-1 beside it, the item routes to EN-1 only; selecting symmetry
/// keeps both.
#[trace("TC-583", "FR-159-AC-3")]
#[test]
fn the_request_writer_restricts_each_item_by_its_reductions() {
    let world = crate::reduction::cycle(
        true,
        &[[0, 0, 0]],
        &[crate::reduction::infinite("always holds(x.v <= 2)")],
    );
    let records = world.package.graph().requirements();
    let items = |registry: &Registry, reductions: Reductions| {
        let items = items_with_reductions(
            records,
            registry,
            None,
            EvidenceSelection::Record,
            &reductions,
        );
        let temporal: Vec<_> = items
            .iter()
            .filter(|item| item.kind() == Capability::TemporalSatisfaction)
            .collect();
        let [item] = &temporal[..] else {
            panic!("one temporal record, one item: {items:?}");
        };
        let CandidateOutcome::Candidates(set) = item.candidates() else {
            panic!("a computed candidate set");
        };
        let candidates: Vec<String> = set
            .candidates()
            .iter()
            .map(|candidate| candidate.id().as_str().to_owned())
            .collect();
        (candidates, item.unsupported_reduction())
    };
    let por = Reductions {
        partial_order: true,
        ..Reductions::default()
    };
    let mut registry = Registry::new();
    registry
        .register(en6::descriptor(ProviderOrigin::Linked).expect("EN-6 admits"))
        .expect("registered");
    let (candidates, missing) = items(&registry, por.clone());
    assert!(candidates.is_empty());
    assert_eq!(missing, Some(PartialOrder));
    assert_eq!(settled(missing), "unsupported-requested-capability");
    registry
        .register(en1::descriptor(ProviderOrigin::Linked).expect("EN-1 admits"))
        .expect("registered");
    assert_eq!(
        items(&registry, por),
        (vec![en1::BACKEND_ID.to_owned()], None)
    );
    let symmetric = Reductions {
        symmetry: vec![qsl_route::reductions::SymmetryDeclaration {
            population: crate::reduction_world::counters(),
            classes: vec![vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]],
        }],
        ..Reductions::default()
    };
    assert_eq!(
        items(&registry, symmetric),
        (
            vec![en1::BACKEND_ID.to_owned(), en6::BACKEND_ID.to_owned()],
            None
        )
    );
}
