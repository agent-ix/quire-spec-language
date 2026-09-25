// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-121 step 11 (FR-042-AC-11): a native package that imports a domain
//! package admitted from filament-core-data#173's architecture bundle checks,
//! emits a compiled protocol whose `Model` names that package directly, and
//! reads back through the strict reader.
//!
//! The bundle runs the real lift, FR-154 admission and FR-152
//! classification (see `composed_domain_models`). Its temp copy also gains
//! `Reading`, a test-authored record value type with one `Boolean` field, so
//! a field read of a domain type reaches emission: the bundle's own record
//! value type, `Count`, has only an `Integer` field, which has no native type
//! here yet.

use crate::composed_domain_models::{
    admitted_with_bytes, architecture_bundle, hex, record_value_type,
};
use crate::support::native_protocol::{Inputs, Unit};

use ix_trace_rs::trace;
use qsl_foundation::ByteDigest;
use qsl_semantics::model::domain_package::DomainPackageRef;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::protocol_artifact::{
    self as artifact, native, wire as w, Error, Invalid, Limits, Unsupported,
    DOMAIN_PACKAGE_PROFILE,
};

const SIMPLE: &str = "protocol Simple using P over (view: M::Node) on origin {\n role Service on M::Node;\n run sequence Main { check Ready using S { true }; }\n finish Closed as (closed: M::Node) { true };\n}";

const READING: &str = "---
id: Reading
title: Reading
object: value_object
type: FR
name: Reading
---

# Reading: Reading

## Description

A test-authored record value type with one Boolean field.

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| ok | Boolean | 1 | |
";

/// The architecture bundle plus `Reading`, admitted, with its document bytes.
fn domain_inputs(units: &[Unit<'_>]) -> Inputs {
    let bundle = architecture_bundle(|root| {
        std::fs::write(root.join("spec/model/Reading.md"), READING).expect("write Reading");
    });
    let (package, bytes) = admitted_with_bytes(bundle.path());
    Inputs::with_domain(units, package, bytes)
}

fn units() -> [Unit<'static>; 2] {
    [
        Unit {
            name: "simple",
            body: SIMPLE,
            declarations: &["Simple"],
        },
        Unit {
            name: "reading",
            body: "predicate ReadingOk using S (reading: D::Reading): Boolean { reading.ok }\n\
                   predicate Metered using S (tally: D::Count): Boolean { true }",
            declarations: &["ReadingOk", "Metered"],
        },
    ]
}

fn discharged(report: &proofs::ProofReport<'_, '_, '_>) {
    assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
    for declaration in report.declarations() {
        assert_eq!(
            report.types().disposition(declaration.declaration()),
            Some(TypeDisposition::Typed),
            "{:?}",
            report
                .types()
                .declaration(declaration.declaration())
                .map(|typed| typed.causes().to_vec())
        );
        assert_eq!(
            declaration.disposition(),
            proofs::ProofDisposition::Discharged,
            "{:?}",
            declaration.causes()
        );
    }
}

#[track_caller]
fn refused<T>(report: &artifact::Report<T>, expected: Error) {
    match report.result() {
        Ok(_) => panic!("expected {expected:?}, but the reader admitted"),
        Err(actual) => assert_eq!(actual, &expected),
    }
}

/// The domain-package model among `models`, and its index.
fn domain_model(models: &[w::Model]) -> (u32, &w::Model) {
    let [(index, model)] = models
        .iter()
        .enumerate()
        .filter(|(_, model)| model.domain_package.0.is_some())
        .collect::<Vec<_>>()[..]
    else {
        panic!("exactly one model names a domain package")
    };
    (u32::try_from(index).unwrap(), model)
}

/// A domain-typed declaration checks and emits; the compiled protocol's
/// `Model` for the package names its identity, version and `sha256-jcs`
/// digest directly, equal to the selection FR-056 admitted, while the native
/// model beside it keeps an explicit `null`; field reads and parameter types
/// name the package's own exports; and the strict reader reads the bytes
/// back unchanged.
#[trace("TC-121", "FR-042-AC-11", "FR-042-AC-12")]
#[test]
fn domain_typed_declarations_emit_a_model_naming_the_domain_package() {
    let inputs = domain_inputs(&units());
    let selection: DomainPackageRef = inputs
        .domain
        .as_ref()
        .expect("domain input")
        .package
        .selection()
        .clone();
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("domain-typed declarations emit");
            let package = admitted.package();
            assert_eq!(package.models.len(), 2);
            let (domain_index, domain) = domain_model(&package.models);
            let naming = domain.domain_package.0.as_ref().expect("named");
            assert_eq!(naming.identity, selection.identity);
            assert_eq!(naming.version, selection.version);
            assert_eq!(naming.digest.0, selection.digest);
            assert_eq!(domain.profile, DOMAIN_PACKAGE_PROFILE);
            assert_eq!(
                package.dependencies[domain.artifact as usize].artifact,
                inputs.domain.as_ref().unwrap().reference
            );
            assert!(package
                .models
                .iter()
                .filter(|model| !std::ptr::eq(*model, domain))
                .all(|model| model.domain_package.0.is_none()));

            // `reading` is typed by the package's `Reading` record export and
            // `reading.ok` reads its `ok` field export.
            let reading_ok = package
                .declarations
                .iter()
                .find(|declaration| declaration.name == "ReadingOk")
                .expect("ReadingOk emitted");
            let w::Type::Record { export } =
                &package.types[reading_ok.binders[0].value_type as usize]
            else {
                panic!("reading is a record value")
            };
            assert_eq!(export.model, domain_index);
            let record = &domain.exports[export.export as usize];
            assert_eq!(
                (record.kind, record.path.as_slice()),
                (w::ExportKind::Record, ["Reading".to_owned()].as_slice())
            );
            let field = reading_ok
                .values
                .iter()
                .find_map(|value| match &value.operation {
                    w::ValueOperation::Field { field, .. } => Some(field),
                    _ => None,
                })
                .expect("reading.ok is a field read");
            assert_eq!(field.model, domain_index);
            let exported = &domain.exports[field.export as usize];
            assert_eq!(exported.kind, w::ExportKind::Field);
            assert_eq!(exported.path, ["Reading", "ok"]);

            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .expect("emits");
            // Decoded from the bytes, the member reproduces the admitted
            // selection with no producer or relation object in between.
            let decoded: serde_json::Value =
                serde_json::from_slice(emitted.bytes()).expect("JSON payload");
            let member = &decoded["models"][domain_index as usize]["domain_package"];
            assert_eq!(member["identity"], selection.identity.as_str());
            assert_eq!(member["version"], selection.version.as_str());
            assert_eq!(member["digest"], hex(&selection.digest).as_str());
            assert_eq!(member.as_object().expect("an object").len(), 3, "{member}");
            let read = inputs.read(proofs, &emitted);
            let read = read.result().expect("the strict reader admits the bytes");
            assert_eq!(read.package(), package);
            assert_eq!(read.digest(), emitted.digest());
        },
    );
}

/// Adverse domain-package `Model`s over the emitted bytes: a changed digest,
/// the naming replaced by `null`, a changed profile and an export the
/// package does not declare each refuse as `Invalid::Model`, and a locus
/// other than the package document as `Invalid::ForeignLocus`, rather than
/// being read into another package.
#[trace("TC-121", "FR-042-AC-11")]
#[test]
fn a_substituted_domain_package_naming_refuses() {
    let inputs = domain_inputs(&units());
    let selection = inputs
        .domain
        .as_ref()
        .expect("domain input")
        .package
        .selection()
        .clone();
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("domain-typed declarations emit");
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .expect("emits");
            let text = std::str::from_utf8(emitted.bytes()).expect("UTF-8 payload");
            let digest = hex(&selection.digest);
            let mut changed = selection.digest;
            changed[0] ^= 1;
            let naming = format!(
                "\"domain_package\":{{\"identity\":\"{}\",\"version\":\"{}\",\"digest\":\"{digest}\"}}",
                selection.identity, selection.version
            );
            assert_eq!(text.matches(&naming).count(), 1, "{text}");
            let profile = format!("\"profile\":\"{DOMAIN_PACKAGE_PROFILE}\"");
            assert_eq!(text.matches(&profile).count(), 1);
            let document_len = inputs.domain.as_ref().unwrap().bytes.len();
            let span = format!("\"span\":{{\"start\":0,\"end\":{document_len}}}");
            assert!(text.contains(&span));
            for (mutated, expected) in [
                // Only the naming's digest: the dependency seal over the same
                // canonical bytes spells the same hex.
                (
                    text.replace(&naming, &naming.replace(&digest, &hex(&changed))),
                    Invalid::Model,
                ),
                (
                    text.replace(&naming, "\"domain_package\":null"),
                    Invalid::Model,
                ),
                (
                    text.replace(&profile, "\"profile\":\"semantic-ir/1.0.0\""),
                    Invalid::Model,
                ),
                // An export the package does not declare.
                (
                    text.replace("[\"Reading\",\"ok\"]", "[\"Reading\",\"no\"]"),
                    Invalid::Model,
                ),
                // A locus other than the package document.
                (
                    text.replace(
                        &span,
                        &format!("\"span\":{{\"start\":0,\"end\":{}}}", document_len - 1),
                    ),
                    Invalid::ForeignLocus,
                ),
            ] {
                assert_ne!(mutated, text);
                let bytes = mutated.into_bytes();
                refused(
                    &inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes)),
                    Error::Invalid(expected),
                );
            }
        },
    );
}

/// A domain object type needs a population input, which has no
/// compiled-protocol export yet: a declaration over `Pump` checks but its
/// emission refuses as unsupported rather than emitting a population it
/// cannot name.
#[trace("TC-121", "FR-042-AC-11")]
#[test]
fn a_domain_object_population_refuses_emission_as_unsupported() {
    let inputs = domain_inputs(&[
        Unit {
            name: "simple",
            body: SIMPLE,
            declarations: &["Simple"],
        },
        Unit {
            name: "pumps",
            body: "predicate PumpUp using S (pump: D::Pump): Boolean { pump.id }",
            declarations: &["PumpUp"],
        },
    ]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let report = native::admit(proofs, selected, Limits::default());
            match report.result() {
                Ok(_) => panic!("a domain object population emitted"),
                Err(error) => assert_eq!(error, &Error::Unsupported(Unsupported::Export)),
            }
        },
    );
}

/// A record value type that reaches a domain object type through a field
/// with no native type here (`pumps: Pump [0..*]`) still needs a population
/// input for `Pump`: emission refuses it as unsupported rather than treating
/// the record as population-free.
#[trace("TC-121", "FR-042-AC-11")]
#[test]
fn a_record_reaching_an_object_through_an_unrepresented_field_refuses_emission() {
    let bundle = architecture_bundle(|root| {
        record_value_type(
            root,
            "Fleet",
            &[("ok", "Boolean", "1"), ("pumps", "Pump", "0..*")],
        );
    });
    let (package, bytes) = admitted_with_bytes(bundle.path());
    let inputs = Inputs::with_domain(
        &[
            Unit {
                name: "simple",
                body: SIMPLE,
                declarations: &["Simple"],
            },
            Unit {
                name: "fleet",
                body: "predicate FleetOk using S (fleet: D::Fleet): Boolean { fleet.ok }",
                declarations: &["FleetOk"],
            },
        ],
        package,
        bytes,
    );
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            match native::admit(proofs, selected, Limits::default()).result() {
                Ok(_) => panic!("a record reaching a domain object type emitted"),
                Err(error) => assert_eq!(error, &Error::Unsupported(Unsupported::Export)),
            }
        },
    );
}
