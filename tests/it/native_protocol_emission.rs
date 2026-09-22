// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-121: actual native stages produce emission authority for supported families.
//! Fixture-selected producer/baseline records establish neither public acceptance
//! nor the separately owned quire-protocol integration and full #40 completion.

use crate::support::native_protocol as setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::protocol_artifact::{
    self as artifact, native, wire as w, Dimension, Error, Invalid, Limits, Unsupported,
};
use quire_spec_language::ByteDigest;
use setup::{Inputs, Unit};

const SIMPLE: &str = "protocol Simple using P over (view: M::Node) on origin {\n role Service on M::Node;\n run sequence Main { check Ready using S { true }; }\n finish Closed as (closed: M::Node) { true };\n}";

fn discharged(report: &proofs::ProofReport<'_, '_, '_>) {
    assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
    for declaration in report.declarations() {
        assert_eq!(
            report.types().disposition(declaration.declaration()),
            Some(TypeDisposition::Typed)
        );
        assert_eq!(
            declaration.disposition(),
            proofs::ProofDisposition::Discharged,
            "{:?}",
            declaration.causes()
        );
        assert!(declaration.complete());
    }
}

#[track_caller]
fn failure<T>(report: &artifact::Report<T>, expected: Error) {
    match report.result() {
        Ok(_) => panic!("expected {expected:?}, but admission succeeded"),
        Err(actual) => assert_eq!(actual, &expected),
    }
}

fn integer_value(value: &w::Integer) -> i64 {
    let artifact::ProtocolNumber::Integer(value) = value.checked().unwrap() else {
        panic!("authored integer bound")
    };
    value.value()
}

fn control_inputs(name: &str, run: &str) -> Inputs {
    let body = format!(
        "protocol Controls using P over (view: M::Node) on origin {{
        role Service on M::Node;
        run {run}
        finish Closed as (closed: M::Node) {{ true }};
    }}"
    );
    Inputs::new(&[Unit {
        name,
        body: &body,
        declarations: &["Controls"],
    }])
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-3", "FR-042-AC-7")]
fn native_protocol_reaches_private_family_admission_and_independent_reader() {
    let inputs = Inputs::new(&[Unit {
        name: "simple",
        body: SIMPLE,
        declarations: &["Simple"],
    }]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("supported native protocol");
            let package = admitted.package();
            assert_eq!(package.wire, artifact::WIRE);
            assert_eq!(package.contract, inputs.contract);
            assert_eq!(package.baseline, inputs.baseline);
            assert_eq!(package.producer, inputs.producer);
            assert_eq!(package.sources.len(), 1);
            assert_eq!(package.sources[0].artifact, inputs.source_references[0]);
            assert_eq!(package.sources[0].text, inputs.sources[0].text());
            assert_eq!(
                package.sources[0].native.identity,
                inputs.sources[0].identity().identity
            );
            assert_eq!(
                package.sources[0].formal.document,
                inputs.formal[0].identity().document().as_str()
            );
            let declaration = &package.declarations[0];
            assert_eq!(declaration.requirement.package, "test/native-emission");
            assert_eq!(declaration.requirement.identity, "NativeEmission");
            assert_eq!(declaration.clause, "simple");
            let w::Body::Protocol {
                input,
                controls,
                causal_edges,
                finish,
                ..
            } = &declaration.body
            else {
                panic!("native protocol family")
            };
            assert_eq!((controls.len(), causal_edges.len()), (2, 3));
            assert!(matches!(
                controls[0].operation,
                w::ControlOperation::Sequence { .. }
            ));
            assert!(matches!(
                controls[1].operation,
                w::ControlOperation::Check { .. }
            ));
            let input_type = declaration.binders[input.index as usize].value_type;
            let w::Type::Object { export } = &package.types[input_type as usize] else {
                panic!("actual Node type, not the proof's Boolean record abstraction")
            };
            assert_eq!(
                package.models[export.model as usize].exports[export.export as usize].path,
                ["Node"]
            );
            assert_eq!(
                declaration.bindings[finish.closure as usize].kind,
                w::BindingKind::Closure
            );
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            assert_eq!(emitted.digest(), ByteDigest::of(emitted.bytes()));
            let read = inputs.read(proofs, &emitted);
            let read = read.result().expect("reader consumes actual emitted bytes");
            assert_eq!(read.package(), package);
            assert_eq!(read.digest(), emitted.digest());
            assert_eq!(
                native::emit(&admitted, Limits::default())
                    .into_result()
                    .unwrap()
                    .bytes(),
                emitted.bytes()
            );
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-4", "FR-042-AC-7")]
fn multi_unit_native_families_keep_callee_source_ids_and_lexical_provenance() {
    let inputs = Inputs::new(&[
        Unit { name: "state", body: "predicate Positive using S (amount: M::Signed): Boolean { amount >= 0 }\ninvariant Healthy using S on M::Node at current { Positive(self.signed) }", declarations: &["Positive", "Healthy"] },
        Unit { name: "temporal", body: "temporal Due using T over (view: M::Node) clock \"temporal_instant\" on origin { always[0,1] holds(Positive(view.signed)) }", declarations: &["Due"] },
        Unit { name: "protocol", body: "protocol Flow using P over (view: M::Node) on origin { role Service on M::Node; requires temporal Due; run sequence Main { check Ready using S { let saved = view.signed in Positive(saved) }; } finish Closed as (closed: M::Node) { Positive(closed.signed) }; }", declarations: &["Flow"] },
    ]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("supported composed families");
            let package = admitted.package();
            assert_eq!((package.sources.len(), package.declarations.len()), (3, 4));
            let find = |name| {
                package
                    .declarations
                    .iter()
                    .position(|declaration| declaration.name == name)
                    .unwrap() as u32
            };
            let positive = find("Positive");
            let due = find("Due");
            let flow = find("Flow");
            assert_eq!(package.declarations[due as usize].requires, [positive]);
            assert_eq!(package.declarations[flow as usize].requires, {
                let mut targets = vec![positive, due];
                targets.sort_unstable();
                targets
            });
            let w::Body::Protocol {
                temporal_requirements,
                ..
            } = &package.declarations[flow as usize].body
            else {
                unreachable!()
            };
            assert_eq!(temporal_requirements, &[due]);
            assert!(matches!(
                package.declarations[due as usize].body,
                w::Body::Temporal { .. }
            ));
            assert!(matches!(
                package.declarations[find("Healthy") as usize].body,
                w::Body::State { .. }
            ));
            let namespace = proofs.types().binding().namespace();
            let temporal = &package.declarations[due as usize];
            let w::Body::Temporal { clock, .. } = &temporal.body else {
                panic!("original temporal family")
            };
            let clock_binding = &temporal.bindings[*clock as usize];
            assert_eq!(clock_binding.kind, w::BindingKind::Clock);
            assert_eq!(clock_binding.name, "clock:temporal_instant");
            assert_eq!(
                clock_binding.subject,
                w::Subject::Declaration { declaration: due }
            );
            let snapshot = temporal
                .bindings
                .iter()
                .position(|binding| binding.kind == w::BindingKind::Snapshot)
                .unwrap();
            assert_ne!(*clock as usize, snapshot);
            assert_eq!(temporal.bindings[snapshot].name, "temporal_instant");
            assert_eq!(clock_binding.anchor, temporal.bindings[snapshot].anchor);
            assert_eq!(
                temporal.anchors[clock_binding.anchor.index as usize].kind,
                w::AnchorKind::TemporalInstant
            );
            let [original_due] = namespace.lookup("Due") else {
                panic!("one original temporal declaration")
            };
            let quire_spec_language::syntax::composed::DeclarationKind::Temporal {
                clock: original_clock,
                ..
            } = &namespace.syntax(*original_due).unwrap().kind
            else {
                panic!("original temporal syntax")
            };
            assert_eq!(original_clock.value, "temporal_instant");
            assert_eq!(clock_binding.locus.source, temporal.locus.source);
            assert_eq!(
                clock_binding.locus.span,
                w::Span {
                    start: original_clock.span.start as u32,
                    end: original_clock.span.end as u32,
                }
            );
            for kind in [w::BindingKind::Progress, w::BindingKind::Closure] {
                let requirement = temporal
                    .bindings
                    .iter()
                    .find(|binding| binding.kind == kind)
                    .unwrap();
                assert_eq!(requirement.requires, [*clock]);
            }
            let mut cross_unit_calls = 0;
            for declaration in &package.declarations {
                let [id] = namespace.lookup(&declaration.name) else {
                    panic!("original declaration")
                };
                let typed = proofs.types().declaration(*id).unwrap();
                let unit = namespace.unit(typed.unit()).unwrap();
                let source = &package.sources[declaration.locus.source as usize];
                assert_eq!(source.native.identity, unit.source().identity().identity);
                for value in &declaration.values {
                    let original = unit
                        .expressions()
                        .get(value.original_expression as usize)
                        .unwrap();
                    assert_eq!(
                        (
                            value.locus.span.start as usize,
                            value.locus.span.end as usize
                        ),
                        (original.span.start, original.span.end)
                    );
                    if let w::ValueOperation::Call {
                        predicate,
                        arguments,
                    } = &value.operation
                    {
                        assert_eq!(*predicate, positive);
                        assert_eq!(arguments.len(), 1);
                        if declaration.locus.source
                            != package.declarations[positive as usize].locus.source
                        {
                            cross_unit_calls += 1;
                        }
                    }
                }
            }
            assert_eq!(cross_unit_calls, 3);
            let declaration = &package.declarations[flow as usize];
            let saved = declaration
                .binders
                .iter()
                .find(|binder| binder.name == "saved")
                .unwrap();
            assert_eq!(saved.kind, w::BinderKind::Let);
            let initialized = saved
                .initializer
                .0
                .as_ref()
                .expect("actual lexical initializer");
            assert_eq!(initialized.declaration, flow);
            assert!(matches!(
                declaration.values[initialized.index as usize].operation,
                w::ValueOperation::Field { .. }
            ));
            assert!(declaration
                .values
                .iter()
                .any(|value| matches!(value.operation, w::ValueOperation::Let { .. })));
            assert!(package.declarations[positive as usize]
                .values
                .iter()
                .any(|value| matches!(value.operation, w::ValueOperation::Number { .. })));
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            assert_eq!(
                inputs
                    .read(proofs, &emitted)
                    .result()
                    .expect("original selections reader roundtrip")
                    .package(),
                package
            );
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn multi_unit_invalid_role_type_reports_the_role_in_its_own_source() {
    let inputs = Inputs::new(&[
        Unit {
            name: "a-temporal",
            body: "temporal Due using T over (view: M::Node) clock \"temporal_instant\" on origin { always[0,1] holds(true) }",
            declarations: &["Due"],
        },
        Unit {
            name: "z-protocol",
            body: "protocol InvalidRole using P over (view: M::Node) on origin { role Service on M::Plain; run sequence Main {} finish Closed as (closed: M::Node) { true }; }",
            declarations: &["InvalidRole"],
        },
    ]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let report = native::admit(proofs, selected, Limits::default());
            assert_eq!(
                report.result().expect_err("invalid role type is refused"),
                &Error::Invalid(Invalid::Type)
            );
            let locus = report.locus().expect("role refusal retains its locus");
            assert_eq!(locus.source, 1);
            let source = inputs
                .sources
                .get(usize::try_from(locus.source).expect("u32 source index fits usize"))
                .expect("reported source exists");
            let span = quire_spec_language::Span {
                start: usize::try_from(locus.span.start).expect("u32 span start fits usize"),
                end: usize::try_from(locus.span.end).expect("u32 span end fits usize"),
            };
            assert_eq!(source.slice(span), Some("role Service on M::Plain;"));
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-8")]
fn semantic_definition_revision_is_independent_of_exact_source_artifact_revision() {
    let mut inputs = Inputs::new(&[Unit {
        name: "definition-revisions",
        body: SIMPLE,
        declarations: &["Simple"],
    }]);
    let (source, bytes) = inputs
        .dependencies
        .iter_mut()
        .find(|(_, bytes)| bytes.as_slice() == R::Protocol.bytes())
        .expect("original registered protocol source");
    source.revision = w::Revision {
        namespace: "test:repository-object".into(),
        value: "opaque-protocol-source-object".into(),
    };
    let original = source.clone();
    assert_eq!(original.digest, ByteDigest::of(bytes));
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("source artifact revision does not rewrite semantic revision");
            let package = admitted.package();
            let definition = package
                .definitions
                .iter()
                .find(|definition| definition.identity == R::Protocol.identity())
                .unwrap();
            assert_eq!(
                definition.revision.namespace,
                selected.definition_revision_namespace
            );
            assert_eq!(definition.revision.value, R::Protocol.revision());
            assert_ne!(definition.revision.namespace, original.revision.namespace);
            assert_ne!(definition.revision.value, original.revision.value);
            assert_eq!(
                package.dependencies[definition.artifact as usize].artifact,
                original
            );
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            assert_eq!(
                inputs
                    .read(proofs, &emitted)
                    .result()
                    .expect("reader preserves independently selected opaque source revision")
                    .package(),
                package
            );

            let mut changed_bytes = R::Protocol.bytes().to_vec();
            changed_bytes.push(b'\n');
            let mut resealed = original.clone();
            resealed.digest = ByteDigest::of(&changed_bytes);
            // Wrong raw-byte sealing refuses before recognition is ever reached.
            let unsealed: Vec<_> = selected
                .dependencies
                .iter()
                .map(|dependency| {
                    if dependency.artifact == &original {
                        artifact::SuppliedDependency {
                            artifact: &original,
                            bytes: &changed_bytes,
                            requires: dependency.requires,
                        }
                    } else {
                        *dependency
                    }
                })
                .collect();
            failure(
                &native::admit(
                    proofs,
                    &native::Selections {
                        dependencies: &unsealed,
                        ..*selected
                    },
                    Limits::default(),
                ),
                Error::Invalid(Invalid::Seal),
            );
            // Recognition is by identity alone: a resealed artifact — self-consistent
            // but carrying altered content under the same source-object identity —
            // still grants the compiler's Protocol interpretation.
            let resealed_dependencies: Vec<_> = selected
                .dependencies
                .iter()
                .map(|dependency| {
                    if dependency.artifact == &original {
                        artifact::SuppliedDependency {
                            artifact: &resealed,
                            bytes: &changed_bytes,
                            requires: dependency.requires,
                        }
                    } else {
                        *dependency
                    }
                })
                .collect();
            assert!(native::admit(
                proofs,
                &native::Selections {
                    dependencies: &resealed_dependencies,
                    ..*selected
                },
                Limits::default(),
            )
            .into_result()
            .is_ok());
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-4", "FR-042-AC-5", "FR-042-AC-7")]
fn native_operation_contracts_events_effects_and_commit_keep_distinct_authority() {
    let mut inputs = Inputs::new(&[
        Unit {
            name: "operation-contracts",
            body: "pre Ready using S on M::Node::step { delta >= 0 }\npost Done using S on M::Node::step { result }",
            declarations: &["Ready", "Done"],
        },
        Unit {
            name: "operation-flow",
            body: "protocol Work using P over (view: M::Node) on origin {
                role Service on M::Node;
                run sequence Main {
                    event Started by Service as (started: M::Plain) { started.ready };
                    attempt Tried by Service on M::Node::step contracts [Ready,Done]
                        as (attempted: M::Plain) { attempted.ready };
                    effect Applied of Main::Tried as (applied: M::Plain) { applied.ready };
                    effect AppliedAgain of Main::Tried as (alsoApplied: M::Plain) { alsoApplied.ready };
                    commit Committed by Service as (committed: M::Plain) { committed.ready };
                }
                finish Closed as (closed: M::Node) { true };
            }",
            declarations: &["Work"],
        },
    ]);
    let expected_operation = inputs.step_contracts("Ready", "Done");
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("actual operation contracts and events");
            let package = admitted.package();
            let find = |name| {
                package
                    .declarations
                    .iter()
                    .position(|d| d.name == name)
                    .unwrap() as u32
            };
            let work = find("Work");
            let ready = find("Ready");
            let done = find("Done");
            assert_eq!(
                package.declarations[ready as usize].execution,
                w::Execution::Pre {
                    operation: expected_operation.clone()
                }
            );
            assert_eq!(
                package.declarations[done as usize].execution,
                w::Execution::Post {
                    operation: expected_operation.clone()
                }
            );
            assert_eq!(
                package.models[expected_operation.model as usize].exports
                    [expected_operation.export as usize]
                    .path,
                ["Node", "step"]
            );
            let declaration = &package.declarations[work as usize];
            let mut expected_contracts = vec![ready, done];
            expected_contracts.sort_unstable();
            assert_eq!(declaration.requires, expected_contracts);
            let w::Body::Protocol {
                roles,
                controls,
                causal_edges,
                ..
            } = &declaration.body
            else {
                panic!("protocol body")
            };
            let control = |name| w::Handle {
                declaration: work,
                index: controls.iter().position(|c| c.name == name).unwrap() as u32,
            };
            let started = control("Started");
            let tried = control("Tried");
            let committed = control("Committed");
            let w::ControlOperation::Event {
                event:
                    w::Event::Event {
                        owner,
                        instance: start_instance,
                        compensation,
                    },
                ..
            } = &controls[started.index as usize].operation
            else {
                panic!("domain event")
            };
            assert_eq!(
                owner,
                &w::Handle {
                    declaration: work,
                    index: 0
                }
            );
            assert_eq!(compensation.0, None);
            let w::ControlOperation::Event {
                event:
                    w::Event::Attempt {
                        operation,
                        contracts,
                        instance: attempt_instance,
                        ..
                    },
                ..
            } = &controls[tried.index as usize].operation
            else {
                panic!("operation attempt")
            };
            assert_eq!(operation, &expected_operation);
            assert_eq!(contracts, &expected_contracts);
            let w::ControlOperation::Commit {
                instance: commit_instance,
                ..
            } = &controls[committed.index as usize].operation
            else {
                panic!("commit")
            };
            let mut instances = vec![*start_instance, *attempt_instance, *commit_instance];
            for name in ["Applied", "AppliedAgain"] {
                let applied = control(name);
                let w::ControlOperation::Event {
                    event: w::Event::Effect { attempt, instance },
                    ..
                } = &controls[applied.index as usize].operation
                else {
                    panic!("effect")
                };
                assert_eq!(attempt, &tried);
                assert_eq!(
                    declaration.bindings[*instance as usize].kind,
                    w::BindingKind::Effect
                );
                assert_eq!(
                    declaration.bindings[*instance as usize].requires,
                    [*attempt_instance]
                );
                assert!(causal_edges.iter().any(|edge| edge.owner == applied
                    && edge.kind == w::EdgeKind::Sequence
                    && edge.from.node == tried
                    && edge.from.port == w::Port::Exit
                    && edge.to.node == applied
                    && edge.to.port == w::Port::Enter));
                instances.push(*instance);
            }
            for (instance, kind, node) in [
                (*start_instance, w::BindingKind::Invocation, started),
                (*attempt_instance, w::BindingKind::Attempt, tried),
                (*commit_instance, w::BindingKind::Commit, committed),
            ] {
                let requirement = &declaration.bindings[instance as usize];
                assert_eq!(requirement.kind, kind);
                assert_eq!(requirement.subject, w::Subject::Control { control: node });
                assert_eq!(requirement.requires, [roles[0].instance]);
            }
            instances.sort_unstable();
            instances.dedup();
            assert_eq!(
                instances.len(),
                5,
                "domain, attempt, two effects and commit are distinct"
            );
            let namespace = proofs.types().binding().namespace();
            let [original] = namespace.lookup("Work") else {
                panic!("source declaration")
            };
            let unit = namespace
                .unit(namespace.declaration(*original).unwrap().unit())
                .unwrap();
            for control in controls {
                let original = &unit.controls()[control.original_node as usize];
                assert_eq!(control.name, original.name.value);
                assert_eq!(
                    control.locus.span,
                    w::Span {
                        start: original.span.start as u32,
                        end: original.span.end as u32
                    }
                );
                assert_eq!(control.locus.source, declaration.locus.source);
            }
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            assert_eq!(
                inputs
                    .read(proofs, &emitted)
                    .result()
                    .expect("independently authored operation exports")
                    .package(),
                package
            );
        },
    );
}

#[trace(
    "TC-121",
    "TC-132",
    "FR-042-AC-1",
    "FR-042-AC-5",
    "FR-042-AC-6",
    "FR-042-AC-7",
    "FR-048-AC-2"
)]
#[test]
fn native_record_channels_preserve_fifo_keys_send_identity_and_cardinality() {
    let inputs = Inputs::new(&[Unit {
        name: "record-channels",
        body: "protocol Delivery using P over (view: M::Node) on origin {
            role Sender on M::Node;
            role Receiver on M::Node;
            channel Records from Sender to Receiver carries M::Plain
                ordering fifo by (message: M::Plain) { message.ready } delivery [0,2];
            channel Objects from Sender to Receiver carries M::Node ordering unordered delivery [1,1];
            run parallel Both {
                branch ordered sequence Ordered {
                    send SentPlain via Records as (sentPlain: M::Plain) { sentPlain.ready };
                    receive GotPlain via Records of SentPlain as (gotPlain: M::Plain) { gotPlain.ready = sentPlain.ready };
                }
                branch loose sequence Loose {
                    send SentNode via Objects as (sentNode: M::Node) { sentNode.signed >= 0 };
                    receive GotNode via Objects of SentNode as (gotNode: M::Node) { gotNode.signed = sentNode.signed };
                }
            } join all [ordered,loose];
            finish Closed as (closed: M::Node) { true };
        }",
        declarations: &["Delivery"],
    }]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("direct record channel correspondence");
            let package = admitted.package();
            let declaration = &package.declarations[0];
            let w::Body::Protocol {
                channels,
                controls,
                causal_edges,
                ..
            } = &declaration.body
            else {
                panic!("protocol")
            };
            assert_eq!(channels.len(), 2);
            assert_eq!(
                (
                    integer_value(&channels[0].delivery.lower),
                    integer_value(&channels[0].delivery.upper)
                ),
                (0, 2)
            );
            assert_eq!(
                (
                    integer_value(&channels[1].delivery.lower),
                    integer_value(&channels[1].delivery.upper)
                ),
                (1, 1)
            );
            assert!(matches!(
                package.types[channels[0].message_type as usize],
                w::Type::Record { .. }
            ));
            assert!(matches!(
                package.types[channels[1].message_type as usize],
                w::Type::Object { .. }
            ));
            assert!(matches!(channels[1].ordering, w::Ordering::Unordered {}));
            let w::Ordering::Fifo {
                binder,
                key,
                anchor,
            } = &channels[0].ordering
            else {
                panic!("authored FIFO")
            };
            let message = &declaration.binders[binder.index as usize];
            assert_eq!(message.name, "message");
            assert_eq!(message.kind, w::BinderKind::Fifo);
            assert_eq!(message.value_type, channels[0].message_type);
            assert_eq!(message.anchor, *anchor);
            assert_eq!(
                declaration.anchors[anchor.index as usize].kind,
                w::AnchorKind::Fifo
            );
            let key = &declaration.values[key.index as usize];
            assert_eq!(key.anchor, *anchor);
            assert!(matches!(
                package.types[key.value_type as usize],
                w::Type::Boolean {}
            ));
            let w::ValueOperation::Field { field, .. } = &key.operation else {
                panic!("authored message.ready key")
            };
            assert_eq!(
                package.models[field.model as usize].exports[field.export as usize].path,
                ["Plain", "ready"]
            );
            let mut requirements = Vec::new();
            let mut pairs = Vec::new();
            for (index, (channel, (send_name, receive_name))) in channels
                .iter()
                .zip([("SentPlain", "GotPlain"), ("SentNode", "GotNode")])
                .enumerate()
            {
                assert_eq!(
                    channel.from,
                    w::Handle {
                        declaration: 0,
                        index: 0
                    }
                );
                assert_eq!(
                    channel.to,
                    w::Handle {
                        declaration: 0,
                        index: 1
                    }
                );
                let handle = w::Handle {
                    declaration: 0,
                    index: index as u32,
                };
                for (at, kind) in [
                    (channel.message, w::BindingKind::Message),
                    (channel.send, w::BindingKind::Send),
                    (channel.receive, w::BindingKind::Receive),
                    (channel.delivery_instance, w::BindingKind::Delivery),
                ] {
                    let binding = &declaration.bindings[at as usize];
                    assert_eq!(binding.kind, kind);
                    assert_eq!(
                        binding.subject,
                        w::Subject::Channel {
                            channel: handle.clone()
                        }
                    );
                    requirements.push(at);
                }
                let send = w::Handle {
                    declaration: 0,
                    index: controls.iter().position(|c| c.name == send_name).unwrap() as u32,
                };
                let receive = w::Handle {
                    declaration: 0,
                    index: controls
                        .iter()
                        .position(|c| c.name == receive_name)
                        .unwrap() as u32,
                };
                let w::ControlOperation::Event {
                    event:
                        w::Event::Send {
                            channel: selected_channel,
                        },
                    binder: sent,
                    ..
                } = &controls[send.index as usize].operation
                else {
                    panic!("send")
                };
                assert_eq!(selected_channel, &handle);
                let w::ControlOperation::Event {
                    event:
                        w::Event::Receive {
                            channel: selected_channel,
                            send: selected_send,
                        },
                    binder: received,
                    ..
                } = &controls[receive.index as usize].operation
                else {
                    panic!("receive")
                };
                assert_eq!(selected_channel, &handle);
                assert_eq!(selected_send, &send);
                assert_ne!(sent, received);
                assert_eq!(
                    declaration.binders[sent.index as usize].value_type,
                    channel.message_type
                );
                assert_eq!(
                    declaration.binders[received.index as usize].value_type,
                    channel.message_type
                );
                let (send_instance, send_requirement) = declaration
                    .bindings
                    .iter()
                    .enumerate()
                    .find(|(_, binding)| {
                        binding.kind == w::BindingKind::Send
                            && binding.subject
                                == w::Subject::Control {
                                    control: send.clone(),
                                }
                    })
                    .expect("distinct source-owned send requirement");
                let receive_requirement = declaration
                    .bindings
                    .iter()
                    .find(|binding| {
                        binding.kind == w::BindingKind::Receive
                            && binding.subject
                                == w::Subject::Control {
                                    control: receive.clone(),
                                }
                    })
                    .expect("distinct source-owned receive requirement");
                assert_eq!(send_requirement.requires, [channel.send]);
                let mut receive_requires = vec![channel.receive, send_instance as u32];
                receive_requires.sort_unstable();
                assert_eq!(receive_requirement.requires, receive_requires);
                assert_ne!(send_instance as u32, channel.send);
                assert!(causal_edges.iter().any(|edge| edge.owner == receive
                    && edge.kind == w::EdgeKind::Sequence
                    && edge.from.node == send
                    && edge.from.port == w::Port::Exit
                    && edge.to.node == receive
                    && edge.to.port == w::Port::Enter));
                pairs.push((send, receive));
            }
            requirements.sort_unstable();
            requirements.dedup();
            assert_eq!(
                requirements.len(),
                8,
                "message/send/receive/delivery remain distinct per channel"
            );
            for edge in causal_edges {
                let in_pair = |pair: &(w::Handle, w::Handle), node: &w::Handle| {
                    node == &pair.0 || node == &pair.1
                };
                assert!(
                    !(in_pair(&pairs[0], &edge.from.node) && in_pair(&pairs[1], &edge.to.node))
                );
                assert!(
                    !(in_pair(&pairs[1], &edge.from.node) && in_pair(&pairs[0], &edge.to.node)),
                    "FIFO introduces no cross-channel order"
                );
            }
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            assert_eq!(
                inputs
                    .read(proofs, &emitted)
                    .result()
                    .expect("original model/source reader roundtrip")
                    .package(),
                package
            );
        },
    );
}

#[trace(
    "TC-121",
    "TC-132",
    "FR-042-AC-1",
    "FR-042-AC-4",
    "FR-042-AC-6",
    "FR-048-AC-2"
)]
#[test]
fn native_scalar_channel_declares_exact_fifo_type_without_invented_event_projection() {
    let inputs = Inputs::new(&[Unit {
        name: "scalar-channel",
        body: "protocol ScalarChannel using P over (view: M::Node) on origin {
            role Service on M::Node;
            channel Samples from Service to Service carries M::Signed
                ordering fifo by (sample: M::Signed) { sample } delivery [0,3];
            run check Ready using S { true };
            finish Closed as (closed: M::Node) { true };
        }",
        declarations: &["ScalarChannel"],
    }]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("static scalar channel and total equality key");
            let package = admitted.package();
            let declaration = &package.declarations[0];
            let w::Body::Protocol {
                channels, controls, ..
            } = &declaration.body
            else {
                panic!("protocol")
            };
            assert_eq!(channels.len(), 1);
            assert_eq!(controls.len(), 1);
            assert!(matches!(
                controls[0].operation,
                w::ControlOperation::Check { .. }
            ));
            let channel = &channels[0];
            let w::Type::Scalar {
                export,
                representation: w::Representation::Integer { minimum, maximum },
                ..
            } = &package.types[channel.message_type as usize]
            else {
                panic!("actual Signed scalar")
            };
            assert_eq!(
                package.models[export.model as usize].exports[export.export as usize].path,
                ["Signed"]
            );
            assert_eq!((integer_value(minimum), integer_value(maximum)), (-10, 10));
            let w::Ordering::Fifo {
                binder,
                key,
                anchor,
            } = &channel.ordering
            else {
                panic!("FIFO")
            };
            let value = &declaration.values[key.index as usize];
            assert_eq!(value.value_type, channel.message_type);
            assert_eq!(value.anchor, *anchor);
            assert_eq!(
                value.operation,
                w::ValueOperation::Read {
                    binder: binder.clone()
                }
            );
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            assert_eq!(
                inputs
                    .read(proofs, &emitted)
                    .result()
                    .expect("scalar channel roundtrip")
                    .package(),
                package
            );
        },
    );
}

#[trace(
    "TC-121",
    "TC-132",
    "FR-042-AC-1",
    "FR-042-AC-6",
    "FR-042-AC-7",
    "FR-048-AC-2"
)]
#[test]
fn zero_delivery_cardinality_is_preserved_by_emission_and_reading() {
    let inputs = Inputs::new(&[Unit {
        name: "zero-delivery",
        body: "protocol ZeroDelivery using P over (view: M::Node) on origin {
            role Service on M::Node;
            channel None from Service to Service carries M::Plain
                ordering unordered delivery [0,0];
            run check Ready using S { true };
            finish Closed as (closed: M::Node) { true };
        }",
        declarations: &["ZeroDelivery"],
    }]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("zero is an authored finite delivery cardinality");
            let package = admitted.package();
            let w::Body::Protocol { channels, .. } = &package.declarations[0].body else {
                panic!("protocol")
            };
            let [channel] = channels.as_slice() else {
                panic!("one channel")
            };
            assert_eq!(
                (
                    integer_value(&channel.delivery.lower),
                    integer_value(&channel.delivery.upper)
                ),
                (0, 0)
            );
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .expect("emit zero delivery cardinality");
            assert_eq!(
                inputs
                    .read(proofs, &emitted)
                    .into_result()
                    .expect("independent reader preserves zero delivery cardinality")
                    .package(),
                package
            );
        },
    );
}

#[test]
#[trace(
    "TC-121",
    "TC-132",
    "FR-042-AC-5",
    "FR-042-AC-6",
    "FR-042-AC-8",
    "FR-048-AC-2"
)]
fn native_channels_refuse_missing_correspondence_bad_keys_and_reversed_cardinality() {
    for (channel, run, expected) in [
        (
            "channel Samples from Service to Service carries M::Signed ordering unordered delivery [0,2];",
            "send Sent via Samples as (sent: M::Node) { sent.signed >= 0 };",
            Error::Unsupported(Unsupported::Export),
        ),
        (
            "channel Samples from Service to Service carries M::Plain ordering fifo by (sample: M::Plain) { sample } delivery [0,2];",
            "check Ready using S { true };",
            Error::Invalid(Invalid::Type),
        ),
        (
            "channel Samples from Service to Service carries M::Plain ordering fifo by (sample: M::Node) { sample.signed } delivery [0,2];",
            "check Ready using S { true };",
            Error::Invalid(Invalid::Type),
        ),
        (
            "channel Samples from Service to Service carries M::Plain ordering unordered delivery [2,1];",
            "check Ready using S { true };",
            Error::Invalid(Invalid::NumericDomain),
        ),
        (
            "channel Samples from Service to Service carries M::Plain ordering unordered delivery [0,9223372036854775808];",
            "check Ready using S { true };",
            Error::Invalid(Invalid::NumericDomain),
        ),
        (
            "",
            "event Happened by Service as (amount: M::Signed) { amount >= 0 };",
            Error::Invalid(Invalid::Type),
        ),
    ] {
        let body = format!("protocol RefusedChannel using P over (view: M::Node) on origin {{
            role Service on M::Node;
            {channel}
            run {run}
            finish Closed as (closed: M::Node) {{ true }};
        }}");
        let inputs = Inputs::new(&[Unit {
            name: "refused-channel",
            body: &body,
            declarations: &["RefusedChannel"],
        }]);
        inputs.with_proofs(TypeLimits::default(), proofs::ProofLimits::default(), |proofs, selected| {
            discharged(proofs);
            failure(&native::admit(proofs, selected, Limits::default()), expected);
        });
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn native_receive_preserves_the_original_same_channel_refusal() {
    use quire_spec_language::linking::composed::scopes::{ScopeIssue, StructuralKind};
    let inputs = Inputs::new(&[Unit {
        name: "crossed-channel",
        body: "protocol Crossed using P over (view: M::Node) on origin {
            role Service on M::Node;
            channel Original from Service to Service carries M::Plain ordering unordered delivery [0,1];
            channel Other from Service to Service carries M::Plain ordering unordered delivery [0,1];
            run sequence Main {
                send Sent via Original as (sent: M::Plain) { sent.ready };
                receive Received via Other of Main::Sent as (received: M::Plain) { received.ready };
            }
            finish Closed as (closed: M::Node) { true };
        }",
        declarations: &["Crossed"],
    }]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            let binding = proofs.types().binding();
            let [id] = binding.namespace().lookup("Crossed") else {
                panic!("original Crossed declaration")
            };
            let scope = binding.scopes().unwrap().declaration(*id).unwrap();
            let [ScopeIssue::IncompatibleReference { reference }] = scope.issues.as_slice() else {
                panic!("one exact send/channel incompatibility: {:?}", scope.issues)
            };
            let selected_send = &scope.references[*reference];
            assert_eq!(selected_send.required, StructuralKind::Send);
            assert_eq!(
                scope.symbols[selected_send.target.unwrap().index()]
                    .name
                    .value,
                "Sent"
            );
            assert_eq!(
                proofs.types().disposition(*id),
                Some(TypeDisposition::Refused)
            );
            assert_eq!(
                proofs.disposition(*id),
                Some(proofs::ProofDisposition::Refused)
            );
            failure(
                &native::admit(proofs, selected, Limits::default()),
                Error::Unsupported(Unsupported::FamilyProof),
            );
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-8")]
fn refused_types_or_definedness_never_grant_native_family_authority() {
    for (expression, type_disposition) in [
        ("view.signed + true", TypeDisposition::Refused),
        ("view.signed + 1 <= 10", TypeDisposition::Typed),
    ] {
        let body = format!("protocol Refused using P over (view: M::Node) on origin {{ role Service on M::Node; run check Risk using S {{ {expression} }}; finish Closed as (closed: M::Node) {{ true }}; }}");
        let inputs = Inputs::new(&[Unit {
            name: "refused",
            body: &body,
            declarations: &["Refused"],
        }]);
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                let [id] = proofs.types().binding().namespace().lookup("Refused") else {
                    unreachable!()
                };
                assert_eq!(proofs.types().disposition(*id), Some(type_disposition));
                assert_eq!(
                    proofs.disposition(*id),
                    Some(proofs::ProofDisposition::Refused)
                );
                failure(
                    &native::admit(proofs, selected, Limits::default()),
                    Error::Unsupported(Unsupported::FamilyProof),
                );
            },
        );
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn nonprogressing_native_repeat_is_refused_after_real_value_discharge() {
    let inputs = Inputs::new(&[Unit { name: "no-progress", body: "protocol NoProgress using P over (view: M::Node) on origin { role Service on M::Node; run repeat Retry by Service visible (true) max 1 while { true } sequence Again { check Nothing using S { true }; } exhausted sequence Stopped {} finish Closed as (closed: M::Node) { true }; }", declarations: &["NoProgress"] }]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            failure(
                &native::admit(proofs, selected, Limits::default()),
                Error::Invalid(Invalid::Control),
            );
        },
    );
}

#[test]
#[trace(
    "TC-121",
    "TC-133",
    "FR-042-AC-1",
    "FR-042-AC-5",
    "FR-042-AC-7",
    "FR-048-AC-3"
)]
fn native_owned_choice_proves_nonliteral_constant_guards_and_preserves_both_branches() {
    let guard = "((not false) and (false or true)) and ((false implies false) = (true != false))
        and (if true then true else false) and (if false then false else true)";
    let run = format!("choice Decide by Service visible ((true or false), (false and true)) {{
        case yes when {{ {guard} }} event Accepted by Service as (accepted: M::Plain) {{ accepted.ready }};
        case no when {{ not ({guard}) }} event Rejected by Service as (rejected: M::Plain) {{ rejected.ready }};
    }}");
    let inputs = control_inputs("constant-choice", &run);
    inputs.with_proofs(TypeLimits::default(), proofs::ProofLimits::default(), |proofs, selected| {
        discharged(proofs);
        let admitted = native::admit(proofs, selected, Limits::default()).into_result().expect("one true nonliteral choice guard");
        let package = admitted.package();
        let declaration = &package.declarations[0];
        let w::Body::Protocol { controls, causal_edges, run, .. } = &declaration.body else { panic!("protocol") };
        let w::ControlOperation::Choice { owner, visible, cases } = &controls[run.index as usize].operation else { panic!("owned choice") };
        assert_eq!(owner, &w::Handle { declaration: 0, index: 0 });
        assert_eq!(visible.len(), 2);
        assert_eq!(cases.iter().map(|case| case.label.as_str()).collect::<Vec<_>>(), ["yes", "no"]);
        assert_eq!(controls[cases[0].body.index as usize].name, "Accepted");
        assert_eq!(controls[cases[1].body.index as usize].name, "Rejected");
        for case in cases {
            assert_eq!(case.guard.declaration, 0);
            assert!(causal_edges.iter().any(|edge| edge.owner == *run && edge.kind == w::EdgeKind::Branch && edge.from.node == *run && edge.from.port == w::Port::Enter && edge.to.node == case.body && edge.to.port == w::Port::Enter));
            assert!(causal_edges.iter().any(|edge| edge.owner == *run && edge.kind == w::EdgeKind::Join && edge.from.node == case.body && edge.from.port == w::Port::Exit && edge.to.node == *run && edge.to.port == w::Port::Exit));
        }
        for operator in [w::Binary::And, w::Binary::Or, w::Binary::Implies, w::Binary::Equal, w::Binary::NotEqual] {
            assert!(declaration.values.iter().any(|value| matches!(&value.operation, w::ValueOperation::Binary { operator: actual, .. } if *actual == operator)), "original {operator:?} expression retained");
        }
        assert!(declaration.values.iter().any(|value| matches!(value.operation, w::ValueOperation::Unary { operator: w::Unary::Not, .. })));
        assert!(declaration.values.iter().any(|value| matches!(value.operation, w::ValueOperation::Group { .. })));
        assert_eq!(declaration.values.iter().filter(|value| matches!(value.operation, w::ValueOperation::If { .. })).count(), 4);
        let emitted = native::emit(&admitted, Limits::default()).into_result().unwrap();
        assert_eq!(inputs.read(proofs, &emitted).result().expect("independent reader accepts source-produced choice").package(), package);
    });
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn native_choice_refuses_overlap_uncovered_and_unproved_dynamic_decisions() {
    for (visible, yes, no, expected) in [
        // The received-Boolean admission rule checks every original operand.
        // These formerly folded inputs cannot acquire visibility from truth.
        (
            "(true or view.plain.ready), (false and view.plain.ready)",
            "if false then view.plain.ready else true",
            "false",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "true",
            "true or false",
            "not false",
            Error::Invalid(Invalid::Control),
        ),
        (
            "true",
            "false and true",
            "true implies false",
            Error::Invalid(Invalid::Control),
        ),
        (
            "true",
            "view.plain.ready",
            "not view.plain.ready",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "view.plain.ready",
            "true",
            "false",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "true",
            "if view.plain.ready then true else false",
            "false",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
    ] {
        let run = format!(
            "choice Decide by Service visible ({visible}) {{
            case yes when {{ {yes} }} check Accept using S {{ true }};
            case no when {{ {no} }} check Reject using S {{ true }};
        }}"
        );
        let inputs = control_inputs("refused-choice", &run);
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                discharged(proofs);
                failure(
                    &native::admit(proofs, selected, Limits::default()),
                    expected,
                );
            },
        );
    }
}

#[test]
#[trace(
    "TC-121",
    "TC-133",
    "FR-042-AC-1",
    "FR-042-AC-5",
    "FR-042-AC-7",
    "FR-048-AC-3",
    "FR-048-AC-5"
)]
fn native_repeat_emits_event_progress_and_respects_zero_or_false_guard_paths() {
    for (maximum, guard, body, observable) in [
        (
            2_i64,
            "true = true",
            "event Advanced by Service as (advanced: M::Plain) { advanced.ready };",
            true,
        ),
        (
            1_i64,
            "true = true",
            "event Advanced by Service as (advanced: M::Plain) { advanced.ready };",
            true,
        ),
        (
            i64::MAX,
            "true = true",
            "event Advanced by Service as (advanced: M::Plain) { advanced.ready };",
            true,
        ),
        (0, "not false", "check Skipped using S { true };", false),
        (
            2,
            "false or false",
            "check Skipped using S { true };",
            false,
        ),
    ] {
        let run = format!(
            "repeat Loop by Service visible (not false) max {maximum} while {{ {guard} }}
            {body} exhausted sequence Stopped {{ check Exhausted using S {{ true }}; }}"
        );
        let inputs = control_inputs("bounded-repeat", &run);
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                discharged(proofs);
                let admitted = native::admit(proofs, selected, Limits::default())
                    .into_result()
                    .expect("bounded event progress or explicit body bypass");
                let package = admitted.package();
                let w::Body::Protocol {
                    controls,
                    causal_edges,
                    run,
                    ..
                } = &package.declarations[0].body
                else {
                    panic!("protocol")
                };
                let w::ControlOperation::Repeat {
                    owner,
                    maximum: limit,
                    body,
                    exhausted,
                    ..
                } = &controls[run.index as usize].operation
                else {
                    panic!("repeat")
                };
                assert_eq!(
                    owner,
                    &w::Handle {
                        declaration: 0,
                        index: 0
                    }
                );
                assert_eq!(integer_value(limit), maximum);
                assert_eq!(
                    matches!(
                        controls[body.index as usize].operation,
                        w::ControlOperation::Event { .. }
                    ),
                    observable
                );
                assert_eq!(controls[exhausted.index as usize].name, "Stopped");
                let back: Vec<_> = causal_edges
                    .iter()
                    .filter(|edge| edge.kind == w::EdgeKind::RepeatProgress)
                    .collect();
                assert_eq!(back.len(), 1);
                assert_eq!(back[0].owner, *run);
                assert_eq!(back[0].from.node, *body);
                assert_eq!(back[0].from.port, w::Port::Exit);
                assert_eq!(back[0].to.node, *run);
                assert_eq!(back[0].to.port, w::Port::Enter);
                assert_eq!(integer_value(back[0].maximum.0.as_ref().unwrap()), maximum);
                let emitted = native::emit(&admitted, Limits::default())
                    .into_result()
                    .unwrap();
                assert_eq!(
                    inputs
                        .read(proofs, &emitted)
                        .result()
                        .expect("original bounded control graph")
                        .package(),
                    package
                );
            },
        );
    }
}

#[trace("TC-121", "TC-133", "FR-042-AC-5", "FR-042-AC-8", "FR-048-AC-3")]
#[test]
fn repeat_maximum_above_signed64_refuses_before_artifact_emission() {
    let run = "repeat Loop by Service visible (not false) max 9223372036854775808 while { true }
        event Advanced by Service as (advanced: M::Plain) { advanced.ready };
        exhausted check Stopped using S { true };";
    let inputs = control_inputs("repeat-maximum-overflow", run);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            failure(
                &native::admit(proofs, selected, Limits::default()),
                Error::Invalid(Invalid::NumericDomain),
            );
        },
    );
}

#[test]
#[trace(
    "TC-121",
    "TC-133",
    "FR-042-AC-1",
    "FR-042-AC-5",
    "FR-042-AC-6",
    "FR-042-AC-7",
    "FR-042-AC-8",
    "FR-048-AC-3"
)]
fn native_await_emits_clock_authority_but_a_deadline_alone_cannot_prove_repeat_progress() {
    let await_node = "await Response after Main::Started using T clock \"reply-clock\" within [0,2]
        match event Reply by Service as (reply: M::Plain) { reply.ready };
        then check Received using S { reply.ready };
        timeout check TimedOut using S { true };";
    let run = format!("sequence Main {{ event Started by Service as (started: M::Plain) {{ started.ready }}; {await_node} }}");
    let inputs = control_inputs("await-response", &run);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("explicit await with selected static clock and progress authorities");
            let package = admitted.package();
            let declaration = &package.declarations[0];
            let w::Body::Protocol {
                controls,
                causal_edges,
                ..
            } = &declaration.body
            else {
                panic!("protocol")
            };
            let handle = |name| w::Handle {
                declaration: 0,
                index: controls.iter().position(|c| c.name == name).unwrap() as u32,
            };
            let response = handle("Response");
            let started = handle("Started");
            let w::ControlOperation::Await {
                after,
                profile,
                clock,
                within,
                event,
                then_body,
                timeout,
            } = &controls[response.index as usize].operation
            else {
                panic!("await")
            };
            assert_eq!(
                after,
                &w::AwaitAnchor::Event {
                    node: started.clone()
                }
            );
            assert_eq!(
                package.definitions[*profile as usize].identity,
                R::EventPosition.identity()
            );
            assert_eq!(
                (integer_value(&within.lower), integer_value(&within.upper)),
                (0, 2)
            );
            let clock_binding = &declaration.bindings[*clock as usize];
            assert_eq!(clock_binding.kind, w::BindingKind::Clock);
            assert_eq!(
                clock_binding.subject,
                w::Subject::Control {
                    control: response.clone()
                }
            );
            assert_eq!(
                declaration.anchors[clock_binding.anchor.index as usize].kind,
                w::AnchorKind::Control
            );
            let namespace = proofs.types().binding().namespace();
            let [original] = namespace.lookup("Controls") else {
                panic!("original protocol declaration")
            };
            let unit = namespace
                .unit(namespace.declaration(*original).unwrap().unit())
                .unwrap();
            let quire_spec_language::syntax::composed::ControlKind::Await {
                clock: authored_clock,
                ..
            } = &unit.controls()[controls[response.index as usize].original_node as usize].kind
            else {
                panic!("original await node")
            };
            assert_eq!(authored_clock.value, "reply-clock");
            assert_eq!(clock_binding.locus.source, declaration.locus.source);
            assert_eq!(
                clock_binding.locus.span,
                w::Span {
                    start: authored_clock.span.start as u32,
                    end: authored_clock.span.end as u32,
                }
            );
            for kind in [w::BindingKind::Progress, w::BindingKind::Closure] {
                let requirement = declaration
                    .bindings
                    .iter()
                    .find(|b| {
                        b.kind == kind
                            && b.subject
                                == w::Subject::Control {
                                    control: response.clone(),
                                }
                    })
                    .unwrap();
                assert_eq!(requirement.requires, [*clock]);
                assert_eq!(requirement.anchor, clock_binding.anchor);
            }
            for (kind, from, from_port, to, to_port) in [
                (
                    w::EdgeKind::Sequence,
                    &started,
                    w::Port::Exit,
                    &response,
                    w::Port::Enter,
                ),
                (
                    w::EdgeKind::AwaitSuccess,
                    &response,
                    w::Port::Enter,
                    event,
                    w::Port::Enter,
                ),
                (
                    w::EdgeKind::AwaitSuccess,
                    event,
                    w::Port::Exit,
                    then_body,
                    w::Port::Enter,
                ),
                (
                    w::EdgeKind::AwaitTimeout,
                    &response,
                    w::Port::Enter,
                    timeout,
                    w::Port::Enter,
                ),
                (
                    w::EdgeKind::Join,
                    then_body,
                    w::Port::Exit,
                    &response,
                    w::Port::Exit,
                ),
                (
                    w::EdgeKind::Join,
                    timeout,
                    w::Port::Exit,
                    &response,
                    w::Port::Exit,
                ),
            ] {
                assert!(causal_edges.iter().any(|edge| edge.owner == response
                    && edge.kind == kind
                    && edge.from.node == *from
                    && edge.from.port == from_port
                    && edge.to.node == *to
                    && edge.to.port == to_port));
            }
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            assert_eq!(
                inputs
                    .read(proofs, &emitted)
                    .result()
                    .expect("original timed static requirements")
                    .package(),
                package
            );
        },
    );
    let repeated = format!(
        "sequence Main {{ event Started by Service as (started: M::Plain) {{ started.ready }};
        repeat WaitAgain by Service visible (true) max 2 while {{ true }}
            {await_node} exhausted check Stopped using S {{ true }};
    }}"
    );
    let inputs = control_inputs("deadline-needs-authority", &repeated);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            failure(
                &native::admit(proofs, selected, Limits::default()),
                Error::Unsupported(Unsupported::FamilyProof),
            );
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-8", "FR-042-AC-9")]
fn missing_selected_model_and_independent_limits_cannot_emit_partial_packages() {
    let inputs = Inputs::new(&[Unit {
        name: "bounded",
        body: SIMPLE,
        declarations: &["Simple"],
    }]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let dependencies: Vec<_> = selected
                .dependencies
                .iter()
                .copied()
                .filter(|dependency| dependency.artifact != &inputs.model_reference)
                .collect();
            let missing = native::Selections {
                dependencies: &dependencies,
                ..*selected
            };
            failure(
                &native::admit(proofs, &missing, Limits::default()),
                Error::Invalid(Invalid::Dependency),
            );
            let refused = native::admit(
                proofs,
                selected,
                Limits {
                    declarations: 0,
                    ..Limits::default()
                },
            );
            let Err(Error::Incomplete(exhaustion)) = refused.result() else {
                panic!("zero declarations must refuse")
            };
            assert_eq!(
                (exhaustion.dimension, exhaustion.used, exhaustion.limit),
                (Dimension::Declarations, 0, 0)
            );
            let admitted = native::admit(
                proofs,
                selected,
                Limits {
                    declarations: 1,
                    ..Limits::default()
                },
            )
            .into_result()
            .expect("independently counted one native declaration");
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            let exact = serde_json::to_vec(admitted.package()).unwrap().len();
            // The expected size is independently serialized from the admitted graph;
            // the work counter is never used to manufacture an exact boundary.
            assert_eq!(emitted.bytes().len(), exact);
            for ceiling in [0, exact - 1] {
                let report = native::emit(
                    &admitted,
                    Limits {
                        output_bytes: ceiling,
                        ..Limits::default()
                    },
                );
                let Err(Error::Incomplete(exhaustion)) = report.result() else {
                    panic!("incomplete emission must expose no package")
                };
                assert_eq!(exhaustion.dimension, Dimension::OutputBytes);
                assert_eq!(exhaustion.limit, ceiling);
                assert!(report.usage().output_bytes <= ceiling);
            }
            assert_eq!(
                native::emit(
                    &admitted,
                    Limits {
                        output_bytes: exact,
                        ..Limits::default()
                    }
                )
                .into_result()
                .unwrap()
                .bytes(),
                emitted.bytes()
            );
        },
    );
}

/// A producer-owned configuration object, authored with member order and
/// whitespace that are deliberately not RFC 8785/JCS canonical.
const CONFIG_AUTHORED: &[u8] =
    b"{\n  \"profile\": \"native-state-model\",\n  \"edition\": \"1-draft\",\n  \"declarations\": 1\n}\n";

/// The same configuration object written out literally in RFC 8785/JCS form.
/// This repository registers no JCS canonicalizer and must not grow one; the
/// constant exists only to name the other domain's byte sequence.
const CONFIG_JCS: &[u8] =
    b"{\"declarations\":1,\"edition\":\"1-draft\",\"profile\":\"native-state-model\"}";

const CONFIG_IDENTITY: &str = "fixture-producer-config";

/// The producer's external byte selection for the configuration object. Only the
/// digest varies across the cases below; every other selector is held fixed.
fn config_reference(digest: ByteDigest) -> w::ArtifactRef {
    w::ArtifactRef {
        ref_version: "ix.artifact-ref/3-draft".into(),
        kind: w::ArtifactKind::Environment,
        authority: "test:native-emission".into(),
        identity: CONFIG_IDENTITY.into(),
        revision: w::Revision {
            namespace: "test:producer-config-revision".into(),
            value: "1".into(),
        },
        digest,
        wire: w::Wire {
            identity: "test:producer-config".into(),
            version: "1".into(),
        },
    }
}

/// Real native source with the producer-owned config artifact added to the exact
/// dependency inventory under the given reference digest and supplied bytes.
fn config_inputs(digest: ByteDigest, bytes: &[u8]) -> Inputs {
    let mut inputs = Inputs::new(&[Unit {
        name: "digest-domains-config",
        body: SIMPLE,
        declarations: &["Simple"],
    }]);
    inputs
        .dependencies
        .push((config_reference(digest), bytes.to_vec()));
    inputs
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-3")]
fn recanonicalized_producer_bytes_do_not_satisfy_the_authored_config_selection() {
    // The two spellings denote the same object and differ only in
    // canonicalization, so no structural comparison can tell the cases apart.
    assert_ne!(CONFIG_AUTHORED, CONFIG_JCS);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(CONFIG_AUTHORED).unwrap(),
        serde_json::from_slice::<serde_json::Value>(CONFIG_JCS).unwrap()
    );
    let authored = ByteDigest::of(CONFIG_AUTHORED);
    let canonical = ByteDigest::of(CONFIG_JCS);
    assert_ne!(authored, canonical);
    // Positive controls: each spelling admits when the reference selects its own
    // bytes, so neither spelling is refused on its own account and no unrelated
    // guard is armed by this fixture.
    for (name, digest, bytes) in [
        (
            "authored reference over authored bytes",
            authored,
            CONFIG_AUTHORED,
        ),
        (
            "canonical reference over canonical bytes",
            canonical,
            CONFIG_JCS,
        ),
    ] {
        let inputs = config_inputs(digest, bytes);
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selections| {
                discharged(proofs);
                let admitted = native::admit(proofs, selections, Limits::default())
                    .into_result()
                    .unwrap_or_else(|error| panic!("{name}: {error:?}"));
                let dependency = admitted
                    .package()
                    .dependencies
                    .iter()
                    .find(|dependency| dependency.artifact.identity == CONFIG_IDENTITY)
                    .expect("the exact producer-selected dependency is retained");
                assert_eq!(dependency.artifact, config_reference(digest), "{name}");
            },
        );
    }
    // Crossing the same two cells is the only change: an equal-meaning
    // re-encoding of the producer's object does not satisfy the authored byte
    // selection, and the authored spelling does not satisfy a canonicalized one.
    // Admission refuses outright, so no artifact is emitted in either direction.
    for (name, digest, bytes) in [
        (
            "authored reference over recanonicalized bytes",
            authored,
            CONFIG_JCS,
        ),
        (
            "canonical reference over authored bytes",
            canonical,
            CONFIG_AUTHORED,
        ),
    ] {
        let inputs = config_inputs(digest, bytes);
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selections| {
                discharged(proofs);
                let mismatches: Vec<_> = selections
                    .dependencies
                    .iter()
                    .filter(|dependency| {
                        ByteDigest::of(dependency.bytes) != dependency.artifact.digest
                    })
                    .map(|dependency| dependency.artifact.identity.as_str())
                    .collect();
                assert_eq!(mismatches, [CONFIG_IDENTITY], "{name}: one changed axis");
                let report = native::admit(proofs, selections, Limits::default());
                assert_eq!(
                    report.result().err(),
                    Some(&Error::Invalid(Invalid::Seal)),
                    "{name}"
                );
                // Native metadata verifies dependency seals before visiting
                // source or model tables. Together with the isolated mismatch
                // and matched controls, this distinguishes this refusal from
                // a later reader/source seal failure.
                assert_eq!(report.usage().sources, 0, "{name}");
                assert_eq!(report.usage().models, 0, "{name}");
                assert_eq!(report.locus(), None, "{name}");
            },
        );
    }
}
