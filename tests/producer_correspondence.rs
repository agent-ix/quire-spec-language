// SPDX-License-Identifier: AGPL-3.0-only
//! Producer 1.2 model admission through the existing compiled contract.

#[allow(dead_code)]
#[path = "support/native_protocol/mod.rs"]
mod setup;

use std::collections::BTreeSet;

use quire_spec_language::checking::composed::{proofs, TypeLimits};
use quire_spec_language::linking::composed::binding_work::{Limits as BindingLimits, Work};
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::linking::composed::models::ModelErrorKind;
use quire_spec_language::linking::composed::producer::{
    admit_producer_model, admit_producer_model_with_limits, CorrespondenceProducer,
    NativeArtifactSelection, ProducerBundleSelection, ProducerCompatibilityInput,
    ProducerCompatibilitySelection, ProducerDigest, ProducerExportKind, ProducerExportSelection,
    ProducerModelRefusal, ProducerObjectSelection, ProducerRevision,
};
use quire_spec_language::linking::composed::subject::ComponentKind;
use quire_spec_language::protocol_artifact::{self as artifact, native, v2, wire as w, Limits};
use quire_spec_language::ByteDigest;
use setup::{Inputs, TemporalDefinitionExpectation, Unit};

fn revision(namespace: &str, value: &str) -> ProducerRevision {
    ProducerRevision {
        namespace: namespace.into(),
        value: value.into(),
    }
}

fn producer_digest(byte: char) -> ProducerDigest {
    ProducerDigest {
        algorithm: "sha256".into(),
        domain: "filament-canonical-json-1".into(),
        version: "1".into(),
        value: format!("sha256:{}", byte.to_string().repeat(64)),
    }
}

fn native_digest(value: ByteDigest) -> ProducerDigest {
    ProducerDigest {
        algorithm: "sha256".into(),
        domain: "quire-native-bytes-1".into(),
        version: "1".into(),
        value: value.to_string(),
    }
}

fn object(identity: &str, byte: char) -> ProducerObjectSelection {
    ProducerObjectSelection {
        identity: identity.into(),
        revision: revision("filament-revision", "1.2.0"),
        digest: producer_digest(byte),
    }
}

fn dependency(kind: w::ArtifactKind, identity: &str, bytes: &[u8]) -> w::ArtifactRef {
    w::ArtifactRef {
        ref_version: "ix.artifact-ref/3-draft".into(),
        kind,
        authority: "test:producer".into(),
        identity: identity.into(),
        revision: w::Revision {
            namespace: "test:producer-revision".into(),
            value: "1".into(),
        },
        digest: ByteDigest::of(bytes),
        wire: w::Wire {
            identity: "test:producer-input".into(),
            version: "1".into(),
        },
    }
}

fn selection(inputs: &Inputs) -> ProducerCompatibilitySelection {
    let model = object("producer:model", '1');
    let profile = object("producer:profile", '2');
    let configuration = object("producer:configuration", '3');
    let source = inputs.model.source().source();
    let locus = w::ForeignLocus {
        source: w::ArtifactRef {
            ref_version: "ix.artifact-ref/3-draft".into(),
            kind: w::ArtifactKind::Source,
            authority: "test:producer".into(),
            identity: source.identity().identity.clone(),
            revision: w::Revision {
                namespace: "test:model-source".into(),
                value: source.identity().revision.clone(),
            },
            digest: source.digest(),
            wire: w::Wire {
                identity: "native-rule-model".into(),
                version: "2".into(),
            },
        },
        formal: w::Formal {
            document: inputs.model.source().identity().document().as_str().into(),
            revision: w::Revision {
                namespace: "test:formal-revision".into(),
                value: inputs
                    .model
                    .source()
                    .identity()
                    .revision()
                    .get()
                    .to_string(),
            },
        },
        span: w::Span { start: 0, end: 1 },
    };
    ProducerCompatibilitySelection {
        interface_version: "1.2.0".into(),
        bundle: ProducerBundleSelection {
            identity: "producer:bundle".into(),
            revision: revision("filament-revision", "1.2.0"),
            digest: producer_digest('4'),
        },
        model: model.clone(),
        profile,
        configuration: configuration.clone(),
        correspondence:
            quire_spec_language::linking::composed::producer::ProducerCorrespondenceSelection {
                relation_identity: "producer:model-native-relation".into(),
                producer: CorrespondenceProducer {
                    kind: "model".into(),
                    authority: "test:producer".into(),
                    selection: model,
                },
                native: NativeArtifactSelection {
                    identity: inputs.model_reference.identity.clone().into(),
                    revision: revision(
                        &inputs.model_reference.revision.namespace,
                        &inputs.model_reference.revision.value,
                    ),
                    digest: native_digest(inputs.model_reference.digest),
                },
                definitions: Vec::new(),
                required_definitions: BTreeSet::new(),
                configuration_identity: configuration.identity.clone(),
                exports: [
                    (ProducerExportKind::Object, "producer:object-node", "Node"),
                    (
                        ProducerExportKind::Component,
                        "producer:campaign-component",
                        "CampaignComponent",
                    ),
                    (
                        ProducerExportKind::Endpoint,
                        "producer:provider-endpoint",
                        "ProviderEndpoint",
                    ),
                    (
                        ProducerExportKind::Relationship,
                        "producer:order-payment",
                        "OrderPayment",
                    ),
                ]
                .into_iter()
                .map(|(kind, identity, path)| ProducerExportSelection {
                    kind,
                    identity: identity.into(),
                    producer_object_identity: "producer:model".into(),
                    path: vec![path.into()],
                    locus: locus.clone(),
                })
                .collect(),
            },
    }
}

fn compensation(name: &str, commit: &str, recovery: &str) -> String {
    format!(
        "compensate {name} for O1::Payment::E1 as (forward{name}: M::Node)
            by Provider on M::Node::step using T clock \"recovery-{name}\" {{
          capture target{name}: M::Total = forward{name}.total;
          activate first (trigger{name}: M::Plain) when {{ not trigger{name}.ready }} {{
            capture activated{name}: Boolean = trigger{name}.ready;
          }}
          within [0,30]; attempts 3 of M::Node;
          retry (earlier{name}: M::Node, later{name}: M::Node) {{
            earlier{name}.tally < 3 and later{name}.tally = earlier{name}.tally + 1
            and target{name} >= 0 and not activated{name}
          }};
          commit {commit};
          recover (recovered{name}: M::Node) {{ {recovery} }};
        }}"
    )
}

fn inputs() -> Inputs {
    inputs_with_related("")
}

fn inputs_with_related(related: &str) -> Inputs {
    let full = compensation(
        "Full",
        "O1::Committed",
        "sum<M::Total>(amountFull in recoveredFull.amounts: amountFull) = targetFull and not activatedFull",
    );
    let partial = compensation(
        "Partial",
        "never",
        "let restoredPartial = sum<M::Total>(amountPartial in recoveredPartial.amounts: amountPartial)
         in restoredPartial > 0 and restoredPartial < targetPartial and not activatedPartial",
    );
    let flow = format!(
        "protocol Campaign using P over (view: M::Node) on origin {{
          role Merchant on M::Node;
          role Provider on M::Node;
          relationship OrderPayment = M::OrderPayment;
          channel Shipments from Merchant to Provider carries M::Plain
            ordering unordered delivery [1,2];
          {full}
          requires temporal Deadline;
          {partial}
          run sequence O1 {{
            send Notice via Shipments as (notice: M::Plain) {related} {{ true }};
            parallel Deliveries {{
              branch left sequence First {{
                receive S1 via Shipments of Notice as (shipmentS1: M::Plain) {{ shipmentS1.ready }};
              }}
              branch right sequence Second {{
                receive S2 via Shipments of Notice as (shipmentS2: M::Plain) {{ shipmentS2.ready }};
              }}
            }} join all [left,right];
            choice DeliveryState by Provider visible (shipmentS1.ready,shipmentS2.ready) {{
              case complete when {{ shipmentS1.ready and shipmentS2.ready }} sequence Complete {{}}
              case waiting when {{ not (shipmentS1.ready and shipmentS2.ready) }} sequence Waiting {{}}
            }}
            sequence Payment {{
              attempt A1 by Provider on M::Node::step contracts [Before,After]
                as (attemptA1: M::Plain) {{ attemptA1.ready }};
              effect E1 of Payment::A1 as (effectE1: M::Node) {{ true }};
            }}
            repeat Retry by Provider visible (shipmentS1.ready,shipmentS2.ready) max 2
              while {{ not (shipmentS1.ready and shipmentS2.ready) }}
              sequence Again {{
                event AwaitDelivery by Provider as (awaited: M::Plain) {{ awaited.ready }};
              }}
              exhausted sequence Refunds {{
                event RefundFull by Provider for Full as (refundFull: M::Plain) {{ refundFull.ready }};
                event RefundPartial by Provider for Partial as (refundPartial: M::Plain) {{ refundPartial.ready }};
              }}
            commit Committed by Provider as (committed: M::Plain) {{ committed.ready }};
          }}
          finish Closed as (closed: M::Node) {{ true }};
        }}"
    );
    let mut inputs = Inputs::new(&[
        Unit {
            name: "producer-contracts",
            body: "pre Before using S on M::Node::step { delta >= 0 }
                   post After using S on M::Node::step { result }",
            declarations: &["Before", "After"],
        },
        Unit {
            name: "producer-time",
            body: "temporal Deadline using T over (view: M::Plain) clock \"campaign-clock\" on origin { true }",
            declarations: &["Deadline"],
        },
        Unit {
            name: "producer-flow",
            body: &flow,
            declarations: &["Campaign"],
        },
    ]);
    inputs.step_contracts("Before", "After");
    inputs
}

#[test]
fn related_occurrences_remain_fail_closed_without_typed_producer_endpoints() {
    let mut inputs = inputs_with_related("related by OrderPayment(view, notice)");
    let interface_bytes = b"producer-interface-1.2.0";
    let relation_bytes = b"producer:model-native-relation";
    let interface = dependency(
        w::ArtifactKind::Source,
        "producer-interface",
        interface_bytes,
    );
    let relation = dependency(
        w::ArtifactKind::Binding,
        "producer:model-native-relation",
        relation_bytes,
    );
    inputs.dependencies.extend([
        (interface.clone(), interface_bytes.to_vec()),
        (relation.clone(), relation_bytes.to_vec()),
    ]);
    inputs.dependencies.sort_by(|left, right| {
        (left.0.kind.as_str(), &left.0.identity).cmp(&(right.0.kind.as_str(), &right.0.identity))
    });
    let selected = selection(&inputs);
    let mut operation = inputs.step_contracts("Before", "After");
    operation.export += 2;
    inputs.remap_operation_contracts("Before", "After", operation);
    let admitted = admit_producer_model(
        ProducerCompatibilityInput {
            selection: &selected,
            native: &inputs.model,
        },
        &selected,
    )
    .expect("producer relationship declaration");
    inputs.with_producer_proofs(
        &admitted,
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selections| {
            let producer = native::ProducerSelection {
                model: &admitted,
                interface: &interface,
                relation: &relation,
            };
            let report =
                native::admit_with_producers(proofs, selections, &[producer], Limits::default());
            assert_eq!(
                report.result().err(),
                Some(&artifact::Error::Unsupported(artifact::Unsupported::Export))
            );
        },
    );
}

fn integer(value: &w::Integer) -> i64 {
    let artifact::ProtocolNumber::Integer(value) = value.checked().expect("canonical integer")
    else {
        panic!("integer field")
    };
    value.value()
}

fn assert_campaign(package: &w::Package) {
    let declaration = package
        .declarations
        .iter()
        .find(|declaration| declaration.name == "Campaign")
        .expect("composed campaign declaration");
    let w::Body::Protocol {
        roles,
        relationships,
        channels,
        compensations,
        temporal_requirements,
        controls,
        causal_edges,
        ..
    } = &declaration.body
    else {
        panic!("campaign protocol")
    };
    assert_eq!(
        roles
            .iter()
            .map(|role| role.name.as_str())
            .collect::<Vec<_>>(),
        ["Merchant", "Provider"]
    );
    let [relationship] = relationships.as_slice() else {
        panic!("one producer-authoritative relationship")
    };
    assert_eq!(relationship.name, "OrderPayment");
    assert_eq!(
        package.models[relationship.model.model as usize].exports
            [relationship.model.export as usize]
            .kind,
        w::ExportKind::Relationship
    );
    let relationship_binding = &declaration.bindings[relationship.binding as usize];
    assert_eq!(relationship_binding.kind, w::BindingKind::Relationship);
    assert_eq!(
        relationship_binding.model.0,
        Some(relationship.model.clone())
    );
    assert!(relationship_binding.value_type.0.is_none());
    assert!(relationship_binding.relation.0.is_some());
    let [channel] = channels.as_slice() else {
        panic!("one shipment channel")
    };
    assert_eq!(
        (
            integer(&channel.delivery.lower),
            integer(&channel.delivery.upper)
        ),
        (1, 2)
    );
    for name in [
        "Notice",
        "S1",
        "S2",
        "DeliveryState",
        "A1",
        "E1",
        "Retry",
        "RefundFull",
        "RefundPartial",
        "Committed",
    ] {
        let control = controls
            .iter()
            .find(|control| control.name == name)
            .unwrap_or_else(|| panic!("authored campaign control {name}"));
        assert_eq!(control.locus.source, declaration.locus.source);
    }
    let choice = controls
        .iter()
        .find(|control| control.name == "DeliveryState")
        .expect("owned delivery choice");
    let w::ControlOperation::Choice { owner, visible, .. } = &choice.operation else {
        panic!("owned choice")
    };
    assert_eq!(roles[owner.index as usize].name, "Provider");
    assert_eq!(visible.len(), 2);
    let retry = controls
        .iter()
        .find(|control| control.name == "Retry")
        .expect("bounded retry");
    let w::ControlOperation::Repeat {
        owner,
        visible,
        maximum,
        ..
    } = &retry.operation
    else {
        panic!("bounded retry")
    };
    assert_eq!(roles[owner.index as usize].name, "Provider");
    assert_eq!(visible.len(), 2);
    assert_eq!(integer(maximum), 2);
    assert!(causal_edges.iter().any(|edge| {
        edge.kind == w::EdgeKind::RepeatProgress
            && edge
                .maximum
                .0
                .as_ref()
                .is_some_and(|bound| integer(bound) == 2)
    }));
    let [full, partial] = compensations.as_slice() else {
        panic!("full and partial refund obligations")
    };
    assert_eq!(
        (full.name.as_str(), partial.name.as_str()),
        ("Full", "Partial")
    );
    assert_eq!(full.forward_effect, partial.forward_effect);
    assert!(full.commit.0.is_some());
    assert!(partial.commit.0.is_none());
    assert!(!full.registration_captures.is_empty());
    assert!(!partial.registration_captures.is_empty());
    assert!(!full.recovery_bindings.is_empty());
    assert!(!partial.recovery_bindings.is_empty());
    assert_eq!(temporal_requirements.len(), 1);
    let control_count = |kind| {
        declaration
            .bindings
            .iter()
            .filter(|binding| {
                binding.kind == kind && matches!(binding.subject, w::Subject::Control { .. })
            })
            .count()
    };
    assert_eq!(control_count(w::BindingKind::Send), 1);
    assert_eq!(control_count(w::BindingKind::Receive), 2);
    assert_eq!(control_count(w::BindingKind::Attempt), 1);
    assert_eq!(control_count(w::BindingKind::Effect), 1);
    assert_eq!(
        declaration
            .bindings
            .iter()
            .filter(|binding| binding.kind == w::BindingKind::CompensationAttempt)
            .count(),
        2
    );
    assert_eq!(
        declaration
            .bindings
            .iter()
            .filter(|binding| binding.kind == w::BindingKind::CompensationEffect)
            .count(),
        2
    );
}

#[test]
fn admitted_producer_model_emits_and_reads_exact_correspondence() {
    let mut inputs = inputs();
    let interface_bytes = b"producer-interface-1.2.0";
    let relation_bytes = b"producer:model-native-relation";
    let interface = dependency(
        w::ArtifactKind::Source,
        "producer-interface",
        interface_bytes,
    );
    let relation = dependency(
        w::ArtifactKind::Binding,
        "producer:model-native-relation",
        relation_bytes,
    );
    inputs.dependencies.extend([
        (interface.clone(), interface_bytes.to_vec()),
        (relation.clone(), relation_bytes.to_vec()),
    ]);
    inputs.dependencies.sort_by(|left, right| {
        (left.0.kind.as_str(), &left.0.identity).cmp(&(right.0.kind.as_str(), &right.0.identity))
    });
    let expected = selection(&inputs);
    let producer_exports_before_operation = expected
        .correspondence
        .exports
        .iter()
        .filter(|export| {
            matches!(
                export.kind,
                ProducerExportKind::Component | ProducerExportKind::Endpoint
            )
        })
        .count() as u32;
    let mut operation = inputs.step_contracts("Before", "After");
    operation.export += producer_exports_before_operation;
    inputs.remap_operation_contracts("Before", "After", operation);
    let offered = expected.clone();
    let admitted = admit_producer_model(
        ProducerCompatibilityInput {
            selection: &offered,
            native: &inputs.model,
        },
        &expected,
    )
    .expect("typed producer selection");

    inputs.with_producer_proofs(
        &admitted,
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            assert!(proofs
                .types()
                .binding()
                .models()
                .unwrap()
                .inputs()
                .iter()
                .any(|input| input
                    .producer_model()
                    .is_some_and(|value| std::ptr::eq(value, &admitted))));
            let producer = native::ProducerSelection {
                model: &admitted,
                interface: &interface,
                relation: &relation,
            };
            let duplicate = native::admit_with_producers(
                proofs,
                selected,
                &[producer, producer],
                Limits::default(),
            );
            assert_eq!(
                duplicate.result().err(),
                Some(&artifact::Error::Invalid(artifact::Invalid::Duplicate))
            );
            let admission =
                native::admit_with_producers(proofs, selected, &[producer], Limits::default())
                    .into_result()
                    .expect("producer-backed family admission");
            let package = admission.package();
            assert_campaign(package);
            let correspondence = package.models[0]
                .correspondence
                .0
                .as_ref()
                .expect("existing correspondence record");
            assert_eq!(correspondence.producer.identity, "producer:model");
            assert_eq!(correspondence.native, package.models[0].artifact);
            assert_eq!(correspondence.exports.len(), 4);
            for (kind, path) in [
                (w::ExportKind::Component, "CampaignComponent"),
                (w::ExportKind::Endpoint, "ProviderEndpoint"),
                (w::ExportKind::Object, "Node"),
                (w::ExportKind::Relationship, "OrderPayment"),
            ] {
                assert!(correspondence.exports.iter().any(|&index| {
                    let export = &package.models[0].exports[index as usize];
                    export.kind == kind && export.path == [path]
                }));
            }

            let emitted = native::emit(&admission, Limits::default())
                .into_result()
                .expect("canonical producer bytes");
            let expected_producer = artifact::ExpectedProducerModel {
                model: &admitted,
                interface: &interface,
                relation: &relation,
            };
            let read = inputs.read_with_producers(proofs, &emitted, &[expected_producer]);
            assert!(
                read.result().is_ok(),
                "strict producer reader: {:?}, locus: {:?}, usage: {:?}",
                read.result(),
                read.locus(),
                read.usage(),
            );

            let base = admission.package();
            let model_index = 0;
            let correspondence = base.models[model_index].correspondence.0.as_ref().unwrap();
            let interface_index = correspondence.producer.interface;
            let native_index = correspondence.native;
            let relation_index = correspondence.relation;
            let export_index = correspondence.exports[0] as usize;
            let mut changed_model = base.clone();
            changed_model.models[model_index]
                .correspondence
                .0
                .as_mut()
                .unwrap()
                .producer
                .identity
                .push_str("-other");
            let mut changed_domain = base.clone();
            changed_domain.models[model_index]
                .correspondence
                .0
                .as_mut()
                .unwrap()
                .producer
                .digest
                .domain = "quire-native-bytes-1".into();
            let mut changed_interface = base.clone();
            changed_interface.models[model_index]
                .correspondence
                .0
                .as_mut()
                .unwrap()
                .producer
                .interface = relation_index;
            let mut changed_native = base.clone();
            changed_native.models[model_index]
                .correspondence
                .0
                .as_mut()
                .unwrap()
                .native = relation_index;
            let mut changed_relation = base.clone();
            changed_relation.models[model_index]
                .correspondence
                .0
                .as_mut()
                .unwrap()
                .relation = interface_index;
            for declaration in &mut changed_relation.declarations {
                for binding in &mut declaration.bindings {
                    if binding.kind == w::BindingKind::Relationship {
                        binding.relation = w::Nullable(Some(interface_index));
                    }
                }
            }
            let mut missing_export = base.clone();
            missing_export.models[model_index]
                .correspondence
                .0
                .as_mut()
                .unwrap()
                .exports
                .clear();
            let mut reordered_exports = base.clone();
            reordered_exports.models[model_index]
                .correspondence
                .0
                .as_mut()
                .unwrap()
                .exports
                .swap(0, 1);
            let mut recanonicalized_source = base.clone();
            recanonicalized_source.models[model_index].exports[export_index]
                .locus
                .source
                .digest = ByteDigest::of(b"presentation-only re-encoding");
            for (offered, error) in [
                (
                    changed_model,
                    artifact::Error::Producer(ProducerModelRefusal::Model),
                ),
                (
                    changed_domain,
                    artifact::Error::Producer(ProducerModelRefusal::ProducerDigest),
                ),
                (
                    changed_interface,
                    artifact::Error::Producer(ProducerModelRefusal::Interface),
                ),
                (
                    changed_native,
                    artifact::Error::Producer(ProducerModelRefusal::NativeBytes),
                ),
                (
                    changed_relation,
                    artifact::Error::Producer(ProducerModelRefusal::Correspondence),
                ),
                (
                    missing_export,
                    artifact::Error::Producer(ProducerModelRefusal::Exports),
                ),
                (
                    reordered_exports,
                    artifact::Error::Invalid(artifact::Invalid::Order),
                ),
                (
                    recanonicalized_source,
                    artifact::Error::Producer(ProducerModelRefusal::Exports),
                ),
            ] {
                let candidate = artifact::encode_candidate(&offered, Limits::default())
                    .into_result()
                    .expect("typed adverse candidate");
                let report = inputs.read_producer_bytes(
                    proofs,
                    candidate.bytes(),
                    candidate.digest(),
                    &[expected_producer],
                );
                assert_eq!(report.result().err(), Some(&error));
            }
            let mut cross_wired_relationship = base.clone();
            cross_wired_relationship
                .declarations
                .iter_mut()
                .flat_map(|declaration| &mut declaration.bindings)
                .find(|binding| binding.kind == w::BindingKind::Relationship)
                .expect("relationship binding")
                .relation = w::Nullable(Some(interface_index));
            let candidate =
                artifact::encode_candidate(&cross_wired_relationship, Limits::default())
                    .into_result()
                    .expect("structurally encoded relationship mutation");
            let report = inputs.read_producer_bytes(
                proofs,
                candidate.bytes(),
                candidate.digest(),
                &[expected_producer],
            );
            assert_eq!(
                report.result().err(),
                Some(&artifact::Error::Invalid(artifact::Invalid::Binding))
            );
            assert_ne!(interface_index, relation_index);
            assert_ne!(native_index, relation_index);

            let mut injected_related = base.clone();
            let declaration_index = injected_related
                .declarations
                .iter()
                .position(|declaration| declaration.name == "Campaign")
                .expect("campaign declaration");
            let declaration = &mut injected_related.declarations[declaration_index];
            assert!(declaration.values.len() >= 2);
            let w::Body::Protocol { controls, .. } = &mut declaration.body else {
                panic!("campaign protocol")
            };
            let control = controls
                .iter_mut()
                .find(|control| control.name == "Notice")
                .expect("send control");
            let locus = control.locus.clone();
            let w::ControlOperation::Event { related, .. } = &mut control.operation else {
                panic!("send event")
            };
            related.push(w::Related {
                relationship: 0,
                from: w::Handle {
                    declaration: declaration_index as u32,
                    index: 0,
                },
                to: w::Handle {
                    declaration: declaration_index as u32,
                    index: 1,
                },
                locus,
            });
            let candidate = artifact::encode_candidate(&injected_related, Limits::default())
                .into_result()
                .expect("structurally encoded related occurrence");
            let report = inputs.read_producer_bytes(
                proofs,
                candidate.bytes(),
                candidate.digest(),
                &[expected_producer],
            );
            assert_eq!(
                report.result().err(),
                Some(&artifact::Error::Unsupported(artifact::Unsupported::Export))
            );

            let endpoint_export = base.models[model_index]
                .exports
                .iter()
                .position(|export| export.kind == w::ExportKind::Endpoint)
                .expect("producer endpoint export") as u32;
            let mut endpoint_role = base.clone();
            let declaration = endpoint_role
                .declarations
                .iter_mut()
                .find(|declaration| declaration.name == "Campaign")
                .expect("campaign declaration");
            let (instance, model) = {
                let w::Body::Protocol { roles, .. } = &mut declaration.body else {
                    panic!("campaign protocol")
                };
                let role = roles.first_mut().expect("merchant role");
                role.model.export = endpoint_export;
                (role.instance, role.model.clone())
            };
            declaration.bindings[instance as usize].model = w::Nullable(Some(model));
            let candidate = artifact::encode_candidate(&endpoint_role, Limits::default())
                .into_result()
                .expect("structurally encoded endpoint role substitution");
            let report = inputs.read_producer_bytes(
                proofs,
                candidate.bytes(),
                candidate.digest(),
                &[expected_producer],
            );
            assert_eq!(
                report.result().err(),
                Some(&artifact::Error::Unsupported(artifact::Unsupported::Export))
            );

            let [deadline] = proofs.types().binding().namespace().lookup("Deadline") else {
                panic!("one temporal declaration")
            };
            let span = proofs
                .types()
                .binding()
                .namespace()
                .syntax(*deadline)
                .unwrap()
                .span;
            let span = w::Span {
                start: span.start as u32,
                end: span.end as u32,
            };
            let definition = R::EventPosition;
            let definition_artifact = selected
                .dependencies
                .iter()
                .find(|dependency| {
                    dependency.artifact.identity == definition.identity()
                        && dependency.bytes == definition.bytes()
                })
                .unwrap()
                .artifact;
            let definition_revision = w::Revision {
                namespace: selected.definition_revision_namespace.into(),
                value: definition.revision().into(),
            };
            let clock = v2::wire::ClockConfiguration::EventPosition {
                sequence_authority: "campaign-events".into(),
            };
            let temporal = native::TemporalSelection {
                source: &inputs.source_references[1],
                span: &span,
                definition_identity: definition.identity(),
                definition_revision: &definition_revision,
                definition_artifact,
                clock: &clock,
            };
            let expected_temporal = inputs.temporal_expectation(
                proofs,
                1,
                "Deadline",
                TemporalDefinitionExpectation {
                    identity: definition.identity().into(),
                    revision: definition_revision.clone(),
                    artifact: definition_artifact.clone(),
                    clock: clock.clone(),
                },
            );
            let v2_emitted = native::admit_v2_with_producers(
                proofs,
                selected,
                &[producer],
                &[temporal],
                Limits::default(),
            )
            .into_result()
            .expect("producer-backed version-2 admission");
            let v2_read = inputs.read_v2_with_producers(
                proofs,
                &v2_emitted,
                &[expected_temporal],
                &[expected_producer],
            );
            assert!(
                v2_read.result().is_ok(),
                "strict version-2 producer reader: {:?}",
                v2_read.result()
            );
        },
    );

    let mut ghost = expected.clone();
    let ghost_locus = ghost.correspondence.exports[0].locus.clone();
    ghost.correspondence.exports.push(ProducerExportSelection {
        kind: ProducerExportKind::Scalar,
        identity: "producer:ghost-scalar".into(),
        producer_object_identity: "producer:model".into(),
        path: vec!["Ghost".into()],
        locus: ghost_locus,
    });
    let mut wrong_kind = expected.clone();
    wrong_kind
        .correspondence
        .exports
        .iter_mut()
        .find(|export| export.kind == ProducerExportKind::Object)
        .expect("object export")
        .kind = ProducerExportKind::Record;
    for invalid in [&ghost, &wrong_kind] {
        let admitted = admit_producer_model(
            ProducerCompatibilityInput {
                selection: invalid,
                native: &inputs.model,
            },
            invalid,
        )
        .expect("structurally valid producer adapter input");
        inputs.with_producer_proofs(
            &admitted,
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                let producer = native::ProducerSelection {
                    model: &admitted,
                    interface: &interface,
                    relation: &relation,
                };
                let report =
                    native::admit_with_producers(proofs, selected, &[producer], Limits::default());
                assert_eq!(
                    report.result().err(),
                    Some(&artifact::Error::Producer(ProducerModelRefusal::Exports))
                );
            },
        );
    }
}

#[test]
fn producer_axes_refuse_independently_before_model_linking() {
    let inputs = inputs();
    let expected = selection(&inputs);
    let mut interface = expected.clone();
    interface.interface_version = "1.3.0".into();
    let mut model = expected.clone();
    model.model.identity = "producer:other-model".into();
    let mut configuration = expected.clone();
    configuration.configuration.identity = "producer:other-configuration".into();
    let mut domain = expected.clone();
    domain.bundle.digest.domain = "quire-native-bytes-1".into();
    let mut source = expected.clone();
    source.correspondence.exports[0].locus.source.digest =
        ByteDigest::of(b"presentation-only re-encoding");
    for (offered, refusal) in [
        (interface, ProducerModelRefusal::Interface),
        (model, ProducerModelRefusal::Model),
        (configuration, ProducerModelRefusal::Configuration),
        (domain, ProducerModelRefusal::ProducerDigest),
        (source, ProducerModelRefusal::Exports),
    ] {
        assert_eq!(
            admit_producer_model(
                ProducerCompatibilityInput {
                    selection: &offered,
                    native: &inputs.model,
                },
                &expected,
            )
            .unwrap_err(),
            refusal,
        );
    }
    assert!(matches!(
        admit_producer_model_with_limits(
            ProducerCompatibilityInput {
                selection: &expected,
                native: &inputs.model,
            },
            &expected,
            BindingLimits {
                bytes: 0,
                ..BindingLimits::default()
            },
        ),
        Err(ProducerModelRefusal::ResourceExhausted(exhaustion))
            if exhaustion.dimension
                == quire_spec_language::linking::composed::binding_work::Dimension::Bytes
                && exhaustion.used == 0
    ));

    let baseline = admit_producer_model(
        ProducerCompatibilityInput {
            selection: &expected,
            native: &inputs.model,
        },
        &expected,
    )
    .expect("baseline producer model");
    let mut changed_profile = expected.clone();
    changed_profile.profile.identity = "producer:changed-profile".into();
    let changed = admit_producer_model(
        ProducerCompatibilityInput {
            selection: &changed_profile,
            native: &inputs.model,
        },
        &changed_profile,
    )
    .expect("independently admitted changed profile");
    assert_eq!(
        inputs
            .producer_subject(&baseline)
            .differences(&inputs.producer_subject(&changed)),
        [ComponentKind::Models]
    );
}

#[test]
fn relationship_declarations_require_the_exact_producer_export_kind() {
    for (mut selected, expected) in [
        {
            let inputs = inputs();
            let mut selected = selection(&inputs);
            selected
                .correspondence
                .exports
                .retain(|export| export.kind != ProducerExportKind::Relationship);
            (selected, ModelErrorKind::MissingExport)
        },
        {
            let inputs = inputs();
            let mut selected = selection(&inputs);
            selected
                .correspondence
                .exports
                .iter_mut()
                .find(|export| export.kind == ProducerExportKind::Relationship)
                .expect("relationship fixture")
                .kind = ProducerExportKind::Component;
            (selected, ModelErrorKind::WrongExportKind)
        },
    ] {
        let inputs = inputs();
        // Rebind the native selection to this fixture's identical admitted model bytes.
        selected.correspondence.native.identity = inputs.model_reference.identity.clone().into();
        selected.correspondence.native.revision = revision(
            &inputs.model_reference.revision.namespace,
            &inputs.model_reference.revision.value,
        );
        selected.correspondence.native.digest = native_digest(inputs.model_reference.digest);
        let admitted = admit_producer_model(
            ProducerCompatibilityInput {
                selection: &selected,
                native: &inputs.model,
            },
            &selected,
        )
        .expect("well-formed producer selection");
        inputs.with_producer_proofs(
            &admitted,
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, _| {
                let binding = proofs.types().binding();
                let [campaign] = binding.namespace().lookup("Campaign") else {
                    panic!("campaign declaration")
                };
                let report = binding
                    .models()
                    .expect("completed model inventory")
                    .resolve_declaration(
                        binding.namespace(),
                        *campaign,
                        &mut Work::new(BindingLimits::default()),
                    )
                    .expect("model report");
                assert!(report.refusals.iter().any(|error| error.kind == expected));
            },
        );
    }
}
