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
    admitted_with, admitted_with_bytes, admitted_with_populations, architecture_bundle, hex,
    pump_operations, record_value_type,
};
use crate::support::native_protocol::{Inputs, Unit};

use ix_trace_rs::trace;
use qsl_foundation::ByteDigest;
use qsl_semantics::model::domain_package::DomainPackageRef;
use quire_contract_ir as ir;
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

/// A domain object type needs a population input, and the unedited bundle
/// declares no population covering `Pump`: a declaration over `Pump` checks
/// but its emission refuses as unsupported rather than emitting a population
/// it cannot name.
#[trace("TC-121", "FR-042-AC-13")]
#[test]
fn a_domain_object_with_no_covering_population_refuses_emission() {
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
/// input for `Pump`: with no population covering `Pump` its emission refuses
/// as unsupported rather than treating the record as population-free, and
/// with `Plant` covering it the declaration carries exactly `Pump`'s
/// population and closure pair.
#[trace("TC-121", "FR-042-AC-13")]
#[test]
fn a_record_reaching_an_object_through_an_unrepresented_field_needs_its_population() {
    for populations in [&[][..], PLANT] {
        let bundle = architecture_bundle(|root| {
            record_value_type(
                root,
                "Fleet",
                &[("ok", "Boolean", "1"), ("pumps", "Pump", "0..*")],
            );
        });
        let (package, bytes) = admitted_with_populations(bundle.path(), populations);
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
                let report = native::admit(proofs, selected, Limits::default());
                if populations.is_empty() {
                    match report.result() {
                        Ok(_) => panic!("a record reaching an uncovered domain object emitted"),
                        Err(error) => {
                            assert_eq!(error, &Error::Unsupported(Unsupported::Export));
                        }
                    }
                    return;
                }
                let admitted = report.into_result().expect("the covered record emits");
                let package = admitted.package();
                let pairs = population_pairs(declaration(package, "FleetOk"));
                assert_eq!(pairs.len(), 2, "{pairs:?}");
                for binding in pairs {
                    assert_eq!(
                        export(
                            package,
                            binding.model.0.as_ref().expect("population export")
                        ),
                        (w::ExportKind::Population, path(&["Pump", "Plant"]))
                    );
                }
                let emitted = native::emit(&admitted, Limits::default())
                    .into_result()
                    .expect("emits");
                let read = inputs.read(proofs, &emitted);
                assert_eq!(read.result().expect("reads back").package(), package);
            },
        );
    }
}

/// The architecture bundle plus `Reading` and `populations`, admitted, as
/// the inputs of `units`.
fn populated_inputs(units: &[Unit<'_>], populations: &[(&str, &[&str])]) -> Inputs {
    let bundle = architecture_bundle(|root| {
        std::fs::write(root.join("spec/model/Reading.md"), READING).expect("write Reading");
    });
    let (package, bytes) = admitted_with_populations(bundle.path(), populations);
    Inputs::with_domain(units, package, bytes)
}

/// `Plant`, one closed population over `Pump` and `Sys`.
const PLANT: &[(&str, &[&str])] = &[("Plant", &["Pump", "Sys"])];

/// A predicate over a `Pump`, a precondition of `Pump::run`, and a protocol
/// whose role plays `Pump`, attempting `Pump::run` and sending `Pump`s on a
/// channel keyed by the `Pump` itself.
const PUMPS: &str = "predicate PumpUp using S (pump: D::Pump): Boolean { pump.id }\n\
pre PumpReady using S on D::Pump::run { self.id }\n\
protocol Pumping using P over (view: D::Pump) on origin {\n\
 role Operator on D::Pump;\n\
 channel Pumps from Operator to Operator carries D::Pump ordering fifo by (keyed: D::Pump) { keyed } delivery [0,2];\n\
 run sequence Main {\n\
  attempt Ran by Operator on D::Pump::run contracts [] as (ran: D::Pump) { ran.id };\n\
  send Sent via Pumps as (sent: D::Pump) { sent.id };\n\
  receive Got via Pumps of Sent as (got: D::Pump) { got.id };\n\
 }\n\
 finish Closed as (closed: D::Pump) { true };\n\
}";

fn pump_units(body: &'static str, declarations: &'static [&'static str]) -> [Unit<'static>; 2] {
    [
        Unit {
            name: "simple",
            body: SIMPLE,
            declarations: &["Simple"],
        },
        Unit {
            name: "pumps",
            body,
            declarations,
        },
    ]
}

/// `PUMPS` over `PLANT`, `PumpReady` bound to `Pump::run`'s execution
/// anchor, its name, and expected to name `operation`.
fn pump_inputs(operation: w::ExportRef) -> Inputs {
    let mut inputs = populated_inputs(
        &pump_units(PUMPS, &["PumpUp", "PumpReady", "Pumping"]),
        PLANT,
    );
    inputs.execution(
        "PumpReady",
        ir::ExecutionPoint::Pre {
            operation: ir::AnchorName::new("run").expect("anchor name"),
        },
        w::Execution::Pre { operation },
    );
    inputs
}

/// The export `reference` names in `package`, as `(kind, path)`.
fn export(package: &w::Package, reference: &w::ExportRef) -> (w::ExportKind, Vec<String>) {
    let export = &package.models[reference.model as usize].exports[reference.export as usize];
    (export.kind, export.path.clone())
}

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

fn declaration<'p>(package: &'p w::Package, name: &str) -> &'p w::Declaration {
    package
        .declarations
        .iter()
        .find(|declaration| declaration.name == name)
        .unwrap_or_else(|| panic!("{name} emitted"))
}

/// The population and closure requirements of `declaration`.
fn population_pairs(declaration: &w::Declaration) -> Vec<&w::BindingRequirement> {
    declaration
        .bindings
        .iter()
        .filter(|binding| {
            matches!(
                binding.kind,
                w::BindingKind::Population | w::BindingKind::Closure
            )
        })
        .collect()
}

/// Emits `PUMPS` and returns the operation export `PumpReady`'s execution
/// names, after asserting it is `Pump::run`'s own `operation` export.
fn emitted_run_export() -> w::ExportRef {
    let placeholder = w::ExportRef {
        model: 0,
        export: 0,
    };
    let inputs = pump_inputs(placeholder);
    let mut run = None;
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("domain populations and operations emit");
            let package = admitted.package();
            let w::Execution::Pre { operation } = &declaration(package, "PumpReady").execution
            else {
                panic!("PumpReady is a precondition")
            };
            assert_eq!(
                export(package, operation),
                (w::ExportKind::Operation, path(&["Pump", "run"]))
            );
            run = Some(operation.clone());
        },
    );
    run.expect("emitted")
}

/// A predicate over a domain object type, a precondition of a domain
/// operation and a protocol attempting it check and emit: each domain
/// object input carries exactly one population and one closure requirement,
/// typed by the object type's `object` export and naming its `population`
/// export `[Pump, Plant]`; the operation is named by its own `operation`
/// export `[Pump, run]` in the precondition's execution and body and in the
/// attempt, with `Pump` as its context; the channel keys on the `Pump`
/// itself. The strict reader reads the bytes back unchanged.
#[trace("TC-121", "FR-042-AC-13")]
#[test]
fn domain_object_populations_and_operations_emit_and_read_back() {
    let run = emitted_run_export();
    let inputs = pump_inputs(run.clone());
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("domain populations and operations emit");
            let package = admitted.package();
            let (domain_index, domain) = domain_model(&package.models);
            assert_eq!(run.model, domain_index);
            let populations: Vec<_> = domain
                .exports
                .iter()
                .filter(|export| export.kind == w::ExportKind::Population)
                .map(|export| export.path.clone())
                .collect();
            assert_eq!(
                populations,
                [path(&["Pump", "Plant"]), path(&["Sys", "Plant"])]
            );

            let pump_up = declaration(package, "PumpUp");
            let pairs = population_pairs(pump_up);
            assert_eq!(pairs.len(), 2, "{pairs:?}");
            let population = pairs
                .iter()
                .find(|binding| binding.kind == w::BindingKind::Population)
                .expect("a population requirement");
            let closure = pairs
                .iter()
                .find(|binding| binding.kind == w::BindingKind::Closure)
                .expect("a closure requirement");
            let model = population.model.0.as_ref().expect("population export");
            assert_eq!(
                export(package, model),
                (w::ExportKind::Population, path(&["Pump", "Plant"]))
            );
            assert_eq!(closure.model, population.model);
            assert_eq!(closure.value_type, population.value_type);
            let w::Type::Object { export: object } =
                &package.types[population.value_type.0.expect("typed") as usize]
            else {
                panic!("a population is typed by its object type")
            };
            assert_eq!(
                export(package, object),
                (w::ExportKind::Object, path(&["Pump"]))
            );

            let ready = declaration(package, "PumpReady");
            let w::Body::State {
                context, operation, ..
            } = &ready.body
            else {
                panic!("PumpReady is a state clause")
            };
            assert_eq!(operation.0.as_ref(), Some(&run));
            assert_eq!(
                export(package, context),
                (w::ExportKind::Object, path(&["Pump"]))
            );

            let pumping = declaration(package, "Pumping");
            let w::Body::Protocol {
                roles,
                controls,
                channels,
                ..
            } = &pumping.body
            else {
                panic!("Pumping is a protocol")
            };
            assert_eq!(
                export(package, &roles[0].model),
                (w::ExportKind::Object, path(&["Pump"]))
            );
            let attempted = controls
                .iter()
                .find_map(|control| match &control.operation {
                    w::ControlOperation::Event {
                        event: w::Event::Attempt { operation, .. },
                        ..
                    } => Some(operation),
                    _ => None,
                })
                .expect("an attempt");
            assert_eq!(attempted, &run);
            assert!(matches!(channels[0].ordering, w::Ordering::Fifo { .. }));

            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .expect("emits");
            let read = inputs.read(proofs, &emitted);
            let read = read.result().expect("the strict reader admits the bytes");
            assert_eq!(read.package(), package);
        },
    );
}

/// Adverse payloads over the emitted population and operation bytes: a
/// changed package digest and an export the package does not declare (a
/// population declaration, or an operation) refuse as `Invalid::Model`; a
/// population pair keyed to another object type of the same declaration
/// refuses as `Invalid::Binding`; and a precondition whose context is not
/// its operation's owner refuses as `Invalid::Type`.
#[trace("TC-121", "FR-042-AC-13")]
#[test]
fn substituted_domain_population_and_operation_keys_refuse() {
    let run = emitted_run_export();
    let inputs = pump_inputs(run);
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
                .expect("emits");
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .expect("encodes");
            let text = std::str::from_utf8(emitted.bytes()).expect("UTF-8 payload");
            let digest = hex(&selection.digest);
            let mut changed = selection.digest;
            changed[0] ^= 1;
            let naming = format!("\"digest\":\"{digest}\"}}");
            assert_eq!(text.matches(&naming).count(), 1, "{text}");
            for (mutated, expected) in [
                (
                    text.replace(&naming, &format!("\"digest\":\"{}\"}}", hex(&changed))),
                    Invalid::Model,
                ),
                (
                    text.replace("[\"Pump\",\"Plant\"]", "[\"Pump\",\"Pond\"]"),
                    Invalid::Model,
                ),
                (
                    text.replace("[\"Pump\",\"run\"]", "[\"Pump\",\"halt\"]"),
                    Invalid::Model,
                ),
            ] {
                assert_ne!(mutated, text);
                let bytes = mutated.into_bytes();
                refused(
                    &inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes)),
                    Error::Invalid(expected),
                );
            }

            let package = admitted.package();
            let (domain_index, domain) = domain_model(&package.models);
            let find = |kind: w::ExportKind, parts: &[&str]| w::ExportRef {
                model: domain_index,
                export: u32::try_from(
                    domain
                        .exports
                        .iter()
                        .position(|export| export.kind == kind && export.path == path(parts))
                        .expect("declared export"),
                )
                .unwrap(),
            };
            let sys_population = find(w::ExportKind::Population, &["Sys", "Plant"]);
            let sys_object = find(w::ExportKind::Object, &["Sys"]);
            let at = |package: &w::Package, name: &str| {
                package
                    .declarations
                    .iter()
                    .position(|declaration| declaration.name == name)
                    .expect("declared")
            };

            // `PumpUp`'s pair, keyed to `Sys` under the same declaration.
            let mut wrong_population = package.clone();
            let pump_up = at(package, "PumpUp");
            for binding in &mut wrong_population.declarations[pump_up].bindings {
                if matches!(
                    binding.kind,
                    w::BindingKind::Population | w::BindingKind::Closure
                ) {
                    binding.model = w::Nullable(Some(sys_population.clone()));
                }
            }
            // `PumpReady`'s context, `Sys` rather than `run`'s owner `Pump`.
            let mut wrong_context = package.clone();
            let ready = at(package, "PumpReady");
            let w::Body::State { context, .. } = &mut wrong_context.declarations[ready].body else {
                panic!("PumpReady is a state clause")
            };
            *context = sys_object;
            for (mutated, expected) in [
                (wrong_population, Invalid::Binding),
                (wrong_context, Invalid::Type),
            ] {
                let candidate = artifact::encode_candidate(&mutated, Limits::default())
                    .into_result()
                    .expect("the mutated package encodes");
                refused(
                    &inputs.read_bytes(proofs, candidate.bytes(), candidate.digest()),
                    Error::Invalid(expected),
                );
            }
        },
    );
}

/// With two population declarations covering `Pump`, the source selects
/// neither, so a declaration over a `Pump` refuses emission as unsupported.
#[trace("TC-121", "FR-042-AC-13")]
#[test]
fn a_domain_object_covered_by_two_populations_refuses_emission() {
    let inputs = populated_inputs(
        &pump_units(
            "predicate PumpUp using S (pump: D::Pump): Boolean { pump.id }",
            &["PumpUp"],
        ),
        &[("Plant", &["Pump"]), ("Yard", &["Pump", "Sys"])],
    );
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            match native::admit(proofs, selected, Limits::default()).result() {
                Ok(_) => panic!("an ambiguous domain population emitted"),
                Err(error) => assert_eq!(error, &Error::Unsupported(Unsupported::Export)),
            }
        },
    );
}

/// A FIFO channel keyed by a record value type has no identity to key on:
/// emission refuses it as `Invalid::Type`, as it does a native record key.
#[trace("TC-121", "FR-042-AC-13")]
#[test]
fn a_record_value_type_channel_key_refuses_emission() {
    let inputs = populated_inputs(
        &pump_units(
            "protocol Readings using P over (view: D::Pump) on origin {\n\
             role Operator on D::Pump;\n\
             channel Values from Operator to Operator carries D::Reading ordering fifo by (keyed: D::Reading) { keyed } delivery [0,2];\n\
             run sequence Main {\n\
              send Sent via Values as (sent: D::Reading) { sent.ok };\n\
             }\n\
             finish Closed as (closed: D::Pump) { true };\n\
             }",
            &["Readings"],
        ),
        PLANT,
    );
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            match native::admit(proofs, selected, Limits::default()).result() {
                Ok(_) => panic!("a record value type channel key emitted"),
                Err(error) => assert_eq!(error, &Error::Invalid(Invalid::Type)),
            }
        },
    );
}

/// `BigPump`, an object type whose only supertype is `Pump`, beside the
/// populations `populations`, as the inputs of a predicate over a `BigPump`.
fn big_pump_inputs(populations: &[(&str, &[&str])]) -> Inputs {
    let bundle = architecture_bundle(|_| {});
    let (package, bytes) = admitted_with(bundle.path(), &[("BigPump", "Pump")], populations);
    Inputs::with_domain(
        &pump_units(
            "predicate BigPumpUp using S (pump: D::BigPump): Boolean { true }",
            &["BigPumpUp"],
        ),
        package,
        bytes,
    )
}

/// FR-153 coverage through a supertype: `Plant` lists only `Pump`, and
/// `BigPump` conforms to `Pump`, so `Plant` covers `BigPump`. The model
/// exports `[BigPump, Plant]`, a declaration over a `BigPump` carries exactly
/// that pair, and it reads back. When the only population lists `Sys`,
/// neither `BigPump` nor its supertype is a member, and emission refuses.
#[trace("TC-121", "FR-042-AC-13")]
#[test]
fn a_domain_object_is_covered_through_its_supertype_only() {
    let inputs = big_pump_inputs(&[("Plant", &["Pump"])]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("a subtype of a member type emits");
            let package = admitted.package();
            let (_, domain) = domain_model(&package.models);
            let populations: Vec<_> = domain
                .exports
                .iter()
                .filter(|export| export.kind == w::ExportKind::Population)
                .map(|export| export.path.clone())
                .collect();
            assert_eq!(
                populations,
                [path(&["BigPump", "Plant"]), path(&["Pump", "Plant"])]
            );
            let pairs = population_pairs(declaration(package, "BigPumpUp"));
            assert_eq!(pairs.len(), 2, "{pairs:?}");
            for binding in pairs {
                assert_eq!(
                    export(
                        package,
                        binding.model.0.as_ref().expect("population export")
                    ),
                    (w::ExportKind::Population, path(&["BigPump", "Plant"]))
                );
            }
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .expect("emits");
            let read = inputs.read(proofs, &emitted);
            assert_eq!(read.result().expect("reads back").package(), package);
        },
    );
    let uncovered = big_pump_inputs(&[("Yard", &["Sys"])]);
    uncovered.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            match native::admit(proofs, selected, Limits::default()).result() {
                Ok(_) => panic!("an uncovered subtype emitted"),
                Err(error) => assert_eq!(error, &Error::Unsupported(Unsupported::Export)),
            }
        },
    );
}

/// `package` with `declaration`'s population and closure requirements
/// removed, re-encoded. They are the declaration's last requirements and no
/// other requirement names them, so no other index moves.
fn without_population_pairs(package: &w::Package, name: &str) -> artifact::Candidate {
    let mut mutated = package.clone();
    let declaration = mutated
        .declarations
        .iter_mut()
        .find(|declaration| declaration.name == name)
        .expect("declared");
    let kept = declaration
        .bindings
        .iter()
        .position(|binding| {
            matches!(
                binding.kind,
                w::BindingKind::Population | w::BindingKind::Closure
            )
        })
        .expect("a population pair");
    assert!(declaration.bindings[kept..].iter().all(|binding| matches!(
        binding.kind,
        w::BindingKind::Population | w::BindingKind::Closure
    )));
    declaration.bindings.truncate(kept);
    artifact::encode_candidate(&mutated, Limits::default())
        .into_result()
        .expect("the mutated package encodes")
}

/// Emits `name` over a `Pump` input (`PumpUp`) or a `Fleet` record reaching
/// `Pump` through a field (`FleetOk`), under `PLANT`, drops its population
/// pair and requires the reader to refuse as `Invalid::Binding`.
fn dropped_pair_refuses(name: &'static str) {
    let bundle = architecture_bundle(|root| {
        record_value_type(
            root,
            "Fleet",
            &[("ok", "Boolean", "1"), ("pumps", "Pump", "0..*")],
        );
    });
    let (package, bytes) = admitted_with_populations(bundle.path(), PLANT);
    let inputs = Inputs::with_domain(
        &pump_units(
            "predicate PumpUp using S (pump: D::Pump): Boolean { pump.id }\n\
             predicate FleetOk using S (fleet: D::Fleet): Boolean { fleet.ok }",
            &["PumpUp", "FleetOk"],
        ),
        package,
        bytes,
    );
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("emits");
            let candidate = without_population_pairs(admitted.package(), name);
            refused(
                &inputs.read_bytes(proofs, candidate.bytes(), candidate.digest()),
                Error::Invalid(Invalid::Binding),
            );
        },
    );
}

/// `PumpUp` over a `Pump`, with its population pair dropped, refuses.
#[trace("TC-121", "FR-042-AC-13")]
#[test]
fn a_dropped_domain_object_population_pair_refuses() {
    dropped_pair_refuses("PumpUp");
}

/// `FleetOk` over a `Fleet` record reaching `Pump`, with `Pump`'s population
/// pair dropped, refuses.
#[trace("TC-121", "FR-042-AC-13")]
#[test]
fn a_dropped_reached_object_population_pair_refuses() {
    dropped_pair_refuses("FleetOk");
}

/// A precondition and two postconditions of domain operations with
/// parameters and a result: `FillReady` reads `fill`'s Boolean parameter,
/// `Filled` its result and both parameters, and `Swapped` `swap`'s `Pump`
/// parameter.
const FILLS: &str = "pre FillReady using S on D::Pump::fill { forced }\n\
post Filled using S on D::Pump::fill { result and level.ok and forced }\n\
post Swapped using S on D::Pump::swap { peer.id }";

/// `FILLS` over the bundle with `Reading`, `Pump`'s added operations and
/// `PLANT`, each clause bound to its operation's execution anchor, its name,
/// and expected to name `fill` or `swap`.
fn fill_inputs(fill: &w::ExportRef, swap: &w::ExportRef) -> Inputs {
    let bundle = architecture_bundle(|root| {
        std::fs::write(root.join("spec/model/Reading.md"), READING).expect("write Reading");
        pump_operations(root);
    });
    let (package, bytes) = admitted_with_populations(bundle.path(), PLANT);
    let mut inputs = Inputs::with_domain(
        &pump_units(FILLS, &["FillReady", "Filled", "Swapped"]),
        package,
        bytes,
    );
    let anchor = |name: &str| ir::AnchorName::new(name).expect("anchor name");
    inputs.execution(
        "FillReady",
        ir::ExecutionPoint::Pre {
            operation: anchor("fill"),
        },
        w::Execution::Pre {
            operation: fill.clone(),
        },
    );
    inputs.execution(
        "Filled",
        ir::ExecutionPoint::Post {
            operation: anchor("fill"),
        },
        w::Execution::Post {
            operation: fill.clone(),
        },
    );
    inputs.execution(
        "Swapped",
        ir::ExecutionPoint::Post {
            operation: anchor("swap"),
        },
        w::Execution::Post {
            operation: swap.clone(),
        },
    );
    inputs
}

/// Each binder of `declaration` as `(name, kind, anchor kind, type)`.
fn binder_rows(
    package: &w::Package,
    declaration: &w::Declaration,
) -> Vec<(String, w::BinderKind, w::AnchorKind, w::Type)> {
    declaration
        .binders
        .iter()
        .map(|binder| {
            (
                binder.name.clone(),
                binder.kind,
                declaration.anchors[binder.anchor.index as usize].kind,
                package.types[binder.value_type as usize].clone(),
            )
        })
        .collect()
}

/// A precondition reading a domain operation's parameter and postconditions
/// reading its parameters and result check, emit and read back: each
/// parameter is an `invocation_parameter` binder at the `invocation_input`
/// anchor, typed by the package's own exports (`level` by the `Reading`
/// record export) or `Boolean`, the result a `result` binder at
/// `invocation_post`, and each clause names its operation's own export
/// `[Pump, fill]` or `[Pump, swap]`. `swap`'s `Pump` parameter carries its
/// population and closure pair at the `invocation_input` anchor.
#[trace("TC-121", "FR-042-AC-14")]
#[test]
fn domain_operation_parameters_and_result_emit_and_read_back() {
    let placeholder = w::ExportRef {
        model: 0,
        export: 0,
    };
    let mut exports = None;
    fill_inputs(&placeholder, &placeholder).with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("domain operation clauses emit");
            let package = admitted.package();
            let operation = |name: &str| {
                let (w::Execution::Pre { operation } | w::Execution::Post { operation }) =
                    &declaration(package, name).execution
                else {
                    panic!("{name} is an operation clause")
                };
                operation.clone()
            };
            exports = Some((operation("FillReady"), operation("Swapped")));
        },
    );
    let (fill, swap) = exports.expect("emitted");
    let inputs = fill_inputs(&fill, &swap);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("domain operation clauses emit");
            let package = admitted.package();
            let (domain_index, domain) = domain_model(&package.models);
            assert_eq!(
                export(package, &fill),
                (w::ExportKind::Operation, path(&["Pump", "fill"]))
            );
            assert_eq!(
                export(package, &swap),
                (w::ExportKind::Operation, path(&["Pump", "swap"]))
            );
            let w::Execution::Post { operation } = &declaration(package, "Filled").execution else {
                panic!("Filled is a postcondition")
            };
            assert_eq!(operation, &fill);
            let reading = domain
                .exports
                .iter()
                .position(|export| {
                    export.kind == w::ExportKind::Record && export.path == ["Reading"]
                })
                .expect("the Reading record export");
            let reading = w::Type::Record {
                export: w::ExportRef {
                    model: domain_index,
                    export: u32::try_from(reading).unwrap(),
                },
            };
            let boolean = w::Type::Boolean {};
            let parameter = |name: &str, ty: &w::Type| {
                (
                    name.to_owned(),
                    w::BinderKind::InvocationParameter,
                    w::AnchorKind::InvocationInput,
                    ty.clone(),
                )
            };

            let ready = binder_rows(package, declaration(package, "FillReady"));
            assert!(ready.contains(&parameter("forced", &boolean)), "{ready:?}");
            assert!(ready.contains(&parameter("level", &reading)), "{ready:?}");
            assert!(ready
                .iter()
                .all(|(_, kind, _, _)| *kind != w::BinderKind::Result));

            let filled = binder_rows(package, declaration(package, "Filled"));
            assert!(
                filled.contains(&parameter("forced", &boolean)),
                "{filled:?}"
            );
            assert!(filled.contains(&parameter("level", &reading)), "{filled:?}");
            assert!(
                filled.contains(&(
                    "result".to_owned(),
                    w::BinderKind::Result,
                    w::AnchorKind::InvocationPost,
                    boolean.clone(),
                )),
                "{filled:?}"
            );
            // `result` and `level.ok` are read from those binders.
            let filled_declaration = declaration(package, "Filled");
            let read_binders: Vec<&str> = filled_declaration
                .values
                .iter()
                .filter_map(|value| match &value.operation {
                    w::ValueOperation::Read { binder } => Some(
                        filled_declaration.binders[binder.index as usize]
                            .name
                            .as_str(),
                    ),
                    _ => None,
                })
                .collect();
            for name in ["result", "level", "forced"] {
                assert!(read_binders.contains(&name), "{name}: {read_binders:?}");
            }

            let swapped = declaration(package, "Swapped");
            let pump = binder_rows(package, swapped)
                .into_iter()
                .find(|(name, ..)| name == "peer")
                .expect("peer is bound");
            let w::Type::Object { export: object } = &pump.3 else {
                panic!("peer is a Pump")
            };
            assert_eq!(
                export(package, object),
                (w::ExportKind::Object, path(&["Pump"]))
            );
            assert_eq!(
                (pump.1, pump.2),
                (
                    w::BinderKind::InvocationParameter,
                    w::AnchorKind::InvocationInput
                )
            );
            let input_pairs: Vec<_> = population_pairs(swapped)
                .into_iter()
                .filter(|binding| {
                    swapped.anchors[binding.anchor.index as usize].kind
                        == w::AnchorKind::InvocationInput
                })
                .collect();
            assert_eq!(input_pairs.len(), 2, "{input_pairs:?}");
            for binding in input_pairs {
                assert_eq!(
                    export(
                        package,
                        binding.model.0.as_ref().expect("population export")
                    ),
                    (w::ExportKind::Population, path(&["Pump", "Plant"]))
                );
            }

            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .expect("emits");
            let read = inputs.read(proofs, &emitted);
            let read = read.result().expect("the strict reader admits the bytes");
            assert_eq!(read.package(), package);
        },
    );
}
