// SPDX-License-Identifier: AGPL-3.0-only
//! TC-121: strict wire-reader controls; native family emission is a separate gate.
//! AC-5/6 exercise only static wire branches; choice totality, runtime progress,
//! delivery/recovery semantics and the complete acceptance criteria remain open.

#[path = "support/protocol_artifact/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::protocol_artifact::{
    self as artifact, wire as w, Dimension, Error, Invalid, Limits, NumberComponent, NumberError,
    NumberWire, Unsupported,
};
use quire_spec_language::ByteDigest;
use serde_json::json;
use setup::{handle, Fixture};

type Mutation<T> = (fn(&mut T), Error);

#[track_caller]
fn failure<T>(report: &artifact::Report<T>, expected: Error) {
    match report.result() {
        Ok(_) => panic!("expected {expected:?}, but the offer was admitted"),
        Err(actual) => assert_eq!(actual, &expected),
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-7")]
fn complete_wire_candidate_binds_real_model_and_independent_original_inventory() {
    let fixture = Fixture::new();
    let candidate = fixture.candidate();
    let selected = Fixture::seal(candidate.bytes());
    let report = fixture.read(candidate.bytes(), &selected, Limits::default());
    let admitted = report.result().expect("complete reader fixture");
    assert_eq!(admitted.package(), &fixture.package);
    assert_eq!(admitted.digest(), ByteDigest::of(candidate.bytes()));
    assert_eq!(candidate.digest(), admitted.digest());
    assert_eq!(
        report.accounting_version(),
        "quire.protocol.artifact-work/1"
    );
    let package = admitted.package();
    let model = &package.models[0];
    assert_eq!(
        package.dependencies[model.artifact as usize]
            .artifact
            .digest,
        fixture.model.digest()
    );
    assert_eq!(model.profile, fixture.model.profile().as_str());
    assert_eq!(model.exports[0].kind, w::ExportKind::Object);
    assert_eq!(model.exports[0].path, ["Node"]);
    let span = fixture
        .model
        .source()
        .to_native(&fixture.model.roles().objects[0].source)
        .unwrap();
    assert_eq!(
        model.exports[0].locus.span,
        w::Span {
            start: span.start as u32,
            end: span.end as u32
        }
    );
    assert_eq!(package.sources[0].native.revision, "editable:7");
    assert_eq!(package.sources[0].formal.document, "ProtocolReaderSource");
    assert_eq!(
        package.declarations[0].requirement.package,
        "test/protocol-reader"
    );
    assert_eq!(
        package.declarations[0]
            .bindings
            .iter()
            .map(|binding| binding.kind)
            .collect::<Vec<_>>(),
        [
            w::BindingKind::WorkflowInstance,
            w::BindingKind::Snapshot,
            w::BindingKind::Closure,
            w::BindingKind::Population,
            w::BindingKind::Closure,
            w::BindingKind::Population,
            w::BindingKind::Closure,
        ]
    );
    assert_eq!(model.exports[1].kind, w::ExportKind::Population);
    assert_eq!(model.exports[1].path, ["Node", "nodes"]);
    assert_eq!(model.exports[1].locus, model.exports[0].locus);
    assert!(model.correspondence.0.is_none());
    // Candidate and reader admission carry no native-emitter/family authority.
    assert_eq!(
        artifact::encode_candidate(package, Limits::default())
            .into_result()
            .unwrap()
            .bytes(),
        candidate.bytes()
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-2", "FR-042-AC-7")]
fn fixed_member_order_and_serde_escaping_have_one_selected_byte_spelling() {
    let fixture = Fixture::new();
    let candidate = fixture.candidate();
    let bytes = candidate.bytes();
    let text = std::str::from_utf8(bytes).unwrap();
    assert!(text.starts_with("{\"wire\":\"quire.compiled-protocol/1\",\"media\":\"application/vnd.quire.compiled-protocol+json;version=1\",\"schema\":\"quire.compiled-protocol.schema/1\",\"type\":\"CompiledProtocolPackage\",\"encoding\":\"quire.protocol.compact-json/1\",\"numeric\":\"quire.protocol.numeric/1\",\"contract\":"));
    assert!(text.contains("café / and escaped control: \\t\\n"));
    assert!(!bytes.ends_with(b"\n"));
    let reordered = text.replacen("{\"wire\":\"quire.compiled-protocol/1\",\"media\":\"application/vnd.quire.compiled-protocol+json;version=1\"", "{\"media\":\"application/vnd.quire.compiled-protocol+json;version=1\",\"wire\":\"quire.compiled-protocol/1\"", 1);
    let alternatives = [
        format!("{text}\n"),
        format!(" {text}"),
        reordered,
        text.replacen("café /", "caf\\u00e9 \\/", 1),
    ];
    for offered in alternatives {
        assert_ne!(offered.as_bytes(), bytes);
        let report = fixture.read(
            offered.as_bytes(),
            &Fixture::seal(offered.as_bytes()),
            Limits::default(),
        );
        failure(&report, Error::Invalid(Invalid::Canonical));
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-7")]
fn resealing_a_tampered_candidate_cannot_replace_the_original_expected_seal() {
    let fixture = Fixture::new();
    let original = fixture.candidate();
    let selected = Fixture::seal(original.bytes());
    let mut offered = fixture.package.clone();
    offered.declarations[0].values[0].operation = w::ValueOperation::Boolean { value: false };
    let tampered = artifact::encode_candidate(&offered, Limits::default())
        .into_result()
        .unwrap();
    assert_ne!(tampered.digest(), selected.digest);
    failure(
        &fixture.read(tampered.bytes(), &selected, Limits::default()),
        Error::Invalid(Invalid::Seal),
    );
    for foreign in [
        fixture.model.digest(),
        fixture.package.sources[0].artifact.digest,
        fixture.package.contract.digest,
    ] {
        let mut substituted = selected.clone();
        substituted.digest = foreign;
        failure(
            &fixture.read(original.bytes(), &substituted, Limits::default()),
            Error::Invalid(Invalid::Seal),
        );
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-3")]
fn independent_header_and_source_selections_refuse_single_axis_substitution() {
    let fixture = Fixture::new();
    let mutations: [fn(&mut w::Package); 11] = [
        |p| p.producer.implementation.push_str("-other"),
        |p| p.producer.revision.namespace.push_str("-other"),
        |p| p.producer.binary.digest = ByteDigest::of(b"other binary"),
        |p| p.baseline.revision.value.push_str("-other"),
        |p| p.contract.identity.push_str("-other"),
        |p| p.language.edition.push_str("-other"),
        |p| p.sources[0].native.identity.push_str("-other"),
        |p| p.sources[0].native.revision.push_str("-other"),
        |p| p.sources[0].formal.revision.namespace.push_str("-other"),
        |p| p.sources[0].path.push_str("-other"),
        |p| p.declarations[0].requirement.identity.push_str("-other"),
    ];
    for mutate in mutations {
        let mut offered = fixture.package.clone();
        mutate(&mut offered);
        failure(
            &fixture.offered(&offered),
            Error::Invalid(Invalid::Selection),
        );
    }
    let mut offered = fixture.package.clone();
    offered.sources[0].text.push(' ');
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Seal));
    offered.sources[0].artifact.digest = ByteDigest::of(offered.sources[0].text.as_bytes());
    failure(
        &fixture.offered(&offered),
        Error::Invalid(Invalid::Selection),
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-8")]
fn unknown_interpretations_and_features_are_explicitly_unsupported() {
    let fixture = Fixture::new();
    for mutate in [
        (|p: &mut w::Package| p.wire.push_str("-future")) as fn(&mut w::Package),
        |p| p.numeric.push_str("-future"),
    ] {
        let mut offered = fixture.package.clone();
        mutate(&mut offered);
        failure(
            &fixture.offered(&offered),
            Error::Unsupported(Unsupported::Wire),
        );
    }
    for optional in [false, true] {
        let mut offered = fixture.package.clone();
        let features = if optional {
            &mut offered.features.optional
        } else {
            &mut offered.features.required
        };
        features.push("quire.protocol.zz-unsupported/1".into());
        failure(
            &fixture.offered(&offered),
            Error::Unsupported(Unsupported::Feature),
        );
    }
    let mut offered = fixture.package.clone();
    offered
        .features
        .required
        .retain(|feature| feature != "quire.protocol.bindings/1");
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Feature));
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-7")]
fn unknown_wire_is_classified_before_its_foreign_dependency_inventory() {
    let fixture = Fixture::new();
    let mut offered = fixture.package.clone();
    offered.wire = "quire.compiled-protocol/2".into();
    offered.dependencies.push(w::Dependency {
        artifact: setup::reference(
            w::ArtifactKind::Source,
            "future-input",
            "test:future",
            "2",
            b"additional future input",
        ),
        requires: vec![],
    });
    failure(
        &fixture.offered(&offered),
        Error::Unsupported(Unsupported::Wire),
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-7")]
fn dependency_definition_and_source_inventories_are_complete_exact_and_unique() {
    let fixture = Fixture::new();
    let mut offered = fixture.package.clone();
    offered.sources.clear();
    failure(
        &fixture.offered(&offered),
        Error::Invalid(Invalid::Inventory),
    );
    let mut offered = fixture.package.clone();
    offered.dependencies.pop();
    failure(
        &fixture.offered(&offered),
        Error::Invalid(Invalid::Inventory),
    );
    let mut offered = fixture.package.clone();
    offered.dependencies[1] = offered.dependencies[0].clone();
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Order));
    let mut offered = fixture.package.clone();
    offered.dependencies.swap(0, 1);
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Order));
    let mut offered = fixture.package.clone();
    offered.dependencies[0].requires.push(1);
    failure(
        &fixture.offered(&offered),
        Error::Invalid(Invalid::Dependency),
    );
    let mut offered = fixture.package.clone();
    let definition = offered
        .definitions
        .iter_mut()
        .find(|definition| !definition.rules.is_empty())
        .unwrap();
    definition.rules.pop();
    failure(
        &fixture.offered(&offered),
        Error::Invalid(Invalid::Definition),
    );
    let mut offered = fixture.package.clone();
    let definition = offered
        .definitions
        .iter_mut()
        .find(|definition| !definition.requires.is_empty())
        .unwrap();
    definition.requires.clear();
    failure(
        &fixture.offered(&offered),
        Error::Invalid(Invalid::Definition),
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-7")]
fn object_only_closed_shapes_refuse_missing_duplicate_unknown_and_wrong_fields() {
    let fixture = Fixture::new();
    let base = fixture.json();
    let mut invalid = Vec::new();
    let mut value = base.clone();
    value.as_object_mut().unwrap().remove("models");
    invalid.push(value);
    let mut value = base.clone();
    value["surprise"] = json!(true);
    invalid.push(value);
    let mut value = base.clone();
    value["models"][0]
        .as_object_mut()
        .unwrap()
        .remove("correspondence");
    invalid.push(value);
    let mut value = base.clone();
    value["types"][1]["surprise"] = json!(true);
    invalid.push(value);
    let mut value = base.clone();
    value["types"][1]["kind"] = json!("future-boolean");
    invalid.push(value);
    let mut value = base.clone();
    value["sources"][0]["formal"] =
        json!(["ProtocolReaderSource", {"namespace":"test-authored-revision", "value":"7"}]);
    invalid.push(value);
    let mut value = base.clone();
    value["sources"][0]["artifact"]["kind"] = json!({"source":null});
    invalid.push(value);
    for value in invalid {
        assert!(matches!(
            fixture.raw(&value).result(),
            Err(Error::Json { .. })
        ));
    }
    for structural in [json!(0.0), json!(-1), json!(1_048_577)] {
        let mut value = base.clone();
        value["declarations"][0]["values"][0]["original_expression"] = structural;
        failure(
            &fixture.raw(&value),
            Error::Invalid(Invalid::StructuralInteger),
        );
    }
    let candidate = fixture.candidate();
    let text = std::str::from_utf8(candidate.bytes()).unwrap();
    let duplicate = text.replacen(
        "{\"wire\":",
        "{\"wire\":\"quire.compiled-protocol/1\",\"wire\":",
        1,
    );
    assert!(matches!(
        fixture
            .read(
                duplicate.as_bytes(),
                &Fixture::seal(duplicate.as_bytes()),
                Limits::default()
            )
            .result(),
        Err(Error::Json { .. })
    ));
}

#[test]
#[trace("TC-121", "FR-042-AC-2", "FR-042-AC-7")]
fn raw_numeric_offers_keep_numeric_refusals_including_encoder_rejection() {
    let fixture = Fixture::new();
    for (number, expected) in [
        (
            json!({"kind":"integer", "decimal":"01"}),
            NumberError::NonCanonicalDecimal {
                component: NumberComponent::Decimal,
            },
        ),
        (
            json!({"kind":"integer", "decimal":"9223372036854775808"}),
            NumberError::ComponentOutOfRange {
                component: NumberComponent::Decimal,
            },
        ),
        (
            setup::rational("1", "0"),
            NumberError::NonPositiveDenominator,
        ),
        (setup::rational("2", "4"), NumberError::UnreducedRational),
    ] {
        let mut value = fixture.json();
        value["declarations"][0]["values"][0]["operation"] =
            json!({"kind":"number", "value":number});
        failure(&fixture.raw(&value), Error::Numeric(expected));
    }
    for number in [
        json!(1),
        json!({"kind":"integer", "decimal":1}),
        json!({"kind":"integer", "decimal":"1", "extra":true}),
    ] {
        let mut value = fixture.json();
        value["declarations"][0]["values"][0]["operation"] =
            json!({"kind":"number", "value":number});
        assert!(matches!(
            fixture.raw(&value).result(),
            Err(Error::Json { .. })
        ));
    }
    let mut floating = fixture.json();
    floating["declarations"][0]["values"][0]["operation"] = json!({"kind":"number", "value":1.5});
    failure(
        &fixture.raw(&floating),
        Error::Invalid(Invalid::StructuralInteger),
    );
    let mut offered = fixture.package.clone();
    offered.declarations[0].values[0].operation = w::ValueOperation::Number {
        value: w::Number(NumberWire::Rational {
            numerator: "2".into(),
            denominator: "4".into(),
        }),
    };
    failure(
        &artifact::encode_candidate(&offered, Limits::default()),
        Error::Numeric(NumberError::UnreducedRational),
    );
    let mut value = fixture.json();
    value["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"sequence", "element":0, "maximum":setup::rational("1", "1")}));
    failure(
        &fixture.raw(&value),
        Error::Invalid(Invalid::WrongNumericKind),
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-4", "FR-042-AC-7")]
fn references_preserve_local_owner_type_scope_and_original_utf8_locus() {
    let fixture = Fixture::new();
    let mut offered = fixture.package.clone();
    offered.declarations[0].values[0].scope.declaration = 1;
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Owner));
    let mut offered = fixture.package.clone();
    offered.declarations[0].values[0].scope.index = 99;
    failure(
        &fixture.offered(&offered),
        Error::Invalid(Invalid::Reference),
    );
    let mut offered = fixture.package.clone();
    offered.declarations[0].values[0].value_type = 0;
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Type));
    let mut offered = fixture.package.clone();
    offered.declarations[0].values[0].operation = w::ValueOperation::Group { value: handle(0) };
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Cycle));
    let mut offered = fixture.package.clone();
    offered.declarations[0].scopes[0].parent = w::Nullable(Some(handle(0)));
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Cycle));
    let mut offered = fixture.package.clone();
    let middle = offered.sources[0].text.find('é').unwrap() as u32 + 1;
    offered.declarations[0].values[0].locus.span = w::Span {
        start: middle,
        end: middle + 1,
    };
    let report = fixture.offered(&offered);
    failure(&report, Error::Invalid(Invalid::Locus));
    assert_eq!(
        report.locus(),
        Some(&offered.declarations[0].values[0].locus)
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-8")]
fn actual_export_authority_cannot_be_replaced_by_a_tag_path_or_foreign_locus() {
    let fixture = Fixture::new();
    for kind in [
        w::ExportKind::Component,
        w::ExportKind::Endpoint,
        w::ExportKind::Relationship,
    ] {
        let mut offered = fixture.package.clone();
        let mut unsupported = offered.models[0].exports[0].clone();
        unsupported.kind = kind;
        offered.models[0].exports.push(unsupported);
        offered.models[0]
            .exports
            .sort_by_key(|export| export.kind.as_str());
        let object = offered.models[0]
            .exports
            .iter()
            .position(|export| export.kind == w::ExportKind::Object)
            .unwrap() as u32;
        offered.types[0] = w::Type::Object {
            export: w::ExportRef {
                model: 0,
                export: object,
            },
        };
        for binding in &mut offered.declarations[0].bindings {
            binding.model.0.as_mut().unwrap().export = object;
        }
        failure(
            &fixture.offered(&offered),
            Error::Unsupported(Unsupported::Export),
        );
    }
    let mut offered = fixture.package.clone();
    offered.models[0].exports[0].path[0] = "OtherNode".into();
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Model));
    let mut offered = fixture.package.clone();
    offered.models[0].exports[1].path[1] = "other-population".into();
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Model));
    let mut offered = fixture.package.clone();
    offered.models[0].exports[0].locus.formal.document = "ForeignModelSource".into();
    failure(
        &fixture.offered(&offered),
        Error::Invalid(Invalid::ForeignLocus),
    );
    let mut offered = fixture.package.clone();
    offered.models[0].exports[0].locus.span.end -= 1;
    failure(
        &fixture.offered(&offered),
        Error::Invalid(Invalid::ForeignLocus),
    );
    let mut offered = fixture.package.clone();
    let duplicate = offered.models[0].exports[0].clone();
    offered.models[0].exports.push(duplicate);
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Order));
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-8")]
fn profiles_and_unimplemented_producer_correspondence_cannot_borrow_admission() {
    let fixture = Fixture::new();
    let mut offered = fixture.package.clone();
    offered.declarations[0].profile = offered.package_definition;
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Profile));
    let mut offered = fixture.package.clone();
    offered.models[0].profile = "native-state-model/2".into();
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Model));
    let mut offered = fixture.package.clone();
    offered.definitions[0].revision.value.push_str("-future");
    failure(
        &fixture.offered(&offered),
        Error::Unsupported(Unsupported::Definition),
    );
    let mut offered = fixture.package.clone();
    let model = &mut offered.models[0];
    // This is an offered claim, deliberately lacking an admitted correspondence
    // producer. Matching native bytes cannot make its other digest domain true.
    model.correspondence = w::Nullable(Some(w::Correspondence {
        producer: w::ProducerObject {
            interface: model.artifact,
            kind: "unimplemented-producer-object".into(),
            authority: "test:external-producer".into(),
            identity: "claimed-object".into(),
            revision: fixture.foreign_formal.revision.clone(),
            digest: w::SelectedDigest {
                domain: "test:separate-object-domain".into(),
                version: "1".into(),
                algorithm: "sha256".into(),
                value: "offered-but-unproved".into(),
            },
        },
        native: model.artifact,
        relation: offered.package_definition,
        exports: vec![0],
    }));
    failure(
        &fixture.offered(&offered),
        Error::Unsupported(Unsupported::ProducerCorrespondence),
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-7", "FR-042-AC-9")]
fn selected_name_bounds_are_utf8_bytes_with_no_truncation() {
    let mut fixture = Fixture::new();
    fixture.package.producer.implementation = "é".repeat(2048);
    let candidate = fixture.candidate();
    assert_eq!(fixture.package.producer.implementation.len(), 4096);
    assert!(fixture
        .read(
            candidate.bytes(),
            &Fixture::seal(candidate.bytes()),
            Limits::default()
        )
        .result()
        .is_ok());
    fixture.package.producer.implementation.push('x');
    let candidate = fixture.candidate();
    failure(
        &fixture.read(
            candidate.bytes(),
            &Fixture::seal(candidate.bytes()),
            Limits::default(),
        ),
        Error::Invalid(Invalid::Name),
    );
    fixture.package.producer.implementation.clear();
    let candidate = fixture.candidate();
    failure(
        &fixture.read(
            candidate.bytes(),
            &Fixture::seal(candidate.bytes()),
            Limits::default(),
        ),
        Error::Invalid(Invalid::Name),
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-7")]
fn required_control_edges_and_closure_bindings_cannot_be_omitted_or_cross_wired() {
    let fixture = Fixture::new();
    for duplicate in [false, true] {
        let mut offered = fixture.package.clone();
        let w::Body::Protocol { causal_edges, .. } = &mut offered.declarations[0].body else {
            unreachable!()
        };
        if duplicate {
            causal_edges.push(causal_edges[0].clone());
        } else {
            causal_edges.clear();
        }
        failure(&fixture.offered(&offered), Error::Invalid(Invalid::Control));
    }
    let mut offered = fixture.package.clone();
    let w::Body::Protocol { finish, .. } = &mut offered.declarations[0].body else {
        unreachable!()
    };
    finish.closure = 1; // A snapshot cannot stand in for complete-workflow closure.
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Binding));
    let mut offered = fixture.package.clone();
    offered.declarations[0].bindings[2].subject = w::Subject::Declaration { declaration: 1 };
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Owner));
}

fn set_limit(limits: &mut Limits, dimension: Dimension, value: usize) {
    match dimension {
        Dimension::PayloadBytes => limits.payload_bytes = value,
        Dimension::OutputBytes => limits.output_bytes = value,
        Dimension::SourceBytes => limits.source_bytes = value,
        Dimension::ContentBytes => limits.content_bytes = value,
        Dimension::Sources => limits.sources = value,
        Dimension::Dependencies => limits.dependencies = value,
        Dimension::Definitions => limits.definitions = value,
        Dimension::Models => limits.models = value,
        Dimension::Declarations => limits.declarations = value,
        Dimension::Entries => limits.entries = value,
        Dimension::References => limits.references = value,
        Dimension::ByteWork => limits.byte_work = value,
        Dimension::Depth => limits.depth = value,
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-9")]
fn independently_counted_peak_limits_distinguish_exact_from_one_short() {
    let fixture = Fixture::new();
    // This oracle uses authored bytes and table lengths, never observed usage.
    let bytes = serde_json::to_vec(&fixture.package).unwrap();
    let selected = Fixture::seal(&bytes);
    for (dimension, exact) in [
        (Dimension::PayloadBytes, bytes.len()),
        (Dimension::OutputBytes, bytes.len()),
        (
            Dimension::SourceBytes,
            fixture.package.sources[0]
                .text
                .len()
                .max(fixture.model.source().source().text().len()),
        ),
        (Dimension::Sources, 1),
        (Dimension::Dependencies, fixture.package.dependencies.len()),
        // Seven language/family/package definitions plus Range,
        // ObservationBinding and Progress for the population/closure pairs.
        (Dimension::Definitions, 10),
        (Dimension::Models, 1),
        (Dimension::Declarations, 1),
    ] {
        let mut limits = Limits::default();
        set_limit(&mut limits, dimension, exact);
        assert!(
            fixture.read(&bytes, &selected, limits).result().is_ok(),
            "exact {dimension:?}"
        );
        set_limit(&mut limits, dimension, exact - 1);
        let refused = fixture.read(&bytes, &selected, limits);
        let Err(Error::Incomplete(exhaustion)) = refused.result() else {
            panic!("one-short {dimension:?}: {refused:?}")
        };
        assert_eq!(exhaustion.dimension, dimension);
        assert_eq!(exhaustion.limit, exact - 1);
        if dimension == Dimension::OutputBytes {
            assert_eq!(refused.locus(), None);
            assert_eq!(exhaustion.locus, None);
        }
        assert!(exhaustion.requested > 0);
        assert!(exhaustion.used <= exhaustion.limit);
        assert!(fixture
            .read(&bytes, &selected, Limits::default())
            .result()
            .is_ok());
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-9")]
fn every_zero_limit_refuses_without_disabling_the_limit_and_fresh_retry_is_clean() {
    let fixture = Fixture::new();
    let bytes = serde_json::to_vec(&fixture.package).unwrap();
    let selected = Fixture::seal(&bytes);
    for dimension in [
        Dimension::PayloadBytes,
        Dimension::OutputBytes,
        Dimension::SourceBytes,
        Dimension::ContentBytes,
        Dimension::Sources,
        Dimension::Dependencies,
        Dimension::Definitions,
        Dimension::Models,
        Dimension::Declarations,
        Dimension::Entries,
        Dimension::References,
        Dimension::ByteWork,
        Dimension::Depth,
    ] {
        let mut limits = Limits::default();
        set_limit(&mut limits, dimension, 0);
        let refused = fixture.read(&bytes, &selected, limits);
        let Err(Error::Incomplete(exhaustion)) = refused.result() else {
            panic!("zero {dimension:?}: {refused:?}")
        };
        assert_eq!(exhaustion.dimension, dimension);
        assert_eq!((exhaustion.used, exhaustion.limit), (0, 0));
        assert!(exhaustion.requested > 0);
        assert_eq!(refused.limits(), limits);
    }
    assert!(fixture
        .read(&bytes, &selected, Limits::default())
        .result()
        .is_ok());
    assert_eq!(serde_json::to_vec(&fixture.package).unwrap(), bytes);
    let mut requested = Limits::default();
    for dimension in [
        Dimension::PayloadBytes,
        Dimension::OutputBytes,
        Dimension::SourceBytes,
        Dimension::ContentBytes,
        Dimension::Sources,
        Dimension::Dependencies,
        Dimension::Definitions,
        Dimension::Models,
        Dimension::Declarations,
        Dimension::Entries,
        Dimension::References,
        Dimension::ByteWork,
        Dimension::Depth,
    ] {
        set_limit(&mut requested, dimension, usize::MAX);
    }
    let bounded = fixture.read(&bytes, &selected, requested);
    assert_eq!(bounded.limits(), Limits::default());
    assert!(bounded.result().is_ok());
}

#[test]
#[trace("TC-121", "FR-042-AC-2", "FR-042-AC-9")]
fn canonical_encoder_output_ceiling_is_exact_and_reports_no_partial_candidate() {
    let fixture = Fixture::new();
    let expected = serde_json::to_vec(&fixture.package).unwrap();
    let exact = Limits {
        output_bytes: expected.len(),
        ..Limits::default()
    };
    let complete = artifact::encode_candidate(&fixture.package, exact)
        .into_result()
        .unwrap();
    assert_eq!(complete.bytes(), expected);
    for limit in [0, expected.len() - 1] {
        let report = artifact::encode_candidate(
            &fixture.package,
            Limits {
                output_bytes: limit,
                ..Limits::default()
            },
        );
        let Err(Error::Incomplete(exhaustion)) = report.result() else {
            panic!("{report:?}")
        };
        assert_eq!(exhaustion.dimension, Dimension::OutputBytes);
        assert_eq!(exhaustion.limit, limit);
        assert!(report.usage().output_bytes <= limit);
    }
    assert_eq!(fixture.candidate().bytes(), expected);
}

#[test]
#[trace("TC-121", "FR-042-AC-7", "FR-042-AC-9")]
fn known_refusals_survive_zero_entries_without_a_diagnostic_allocation() {
    let fixture = Fixture::new();
    let candidate = fixture.candidate();
    let mut selected = Fixture::seal(candidate.bytes());
    selected.digest = ByteDigest::of(b"a different independently selected artifact");
    // Seal checking precedes JSON census and requires exactly zero entries.
    // The terminal Error and optional locus are inline report fields.
    let report = fixture.read(
        candidate.bytes(),
        &selected,
        Limits {
            entries: 0,
            ..Limits::default()
        },
    );
    failure(&report, Error::Invalid(Invalid::Seal));
    assert_eq!(report.usage().entries, 0);
    assert_eq!(report.limits().entries, 0);
    assert_eq!(report.locus(), None);

    let mut offered = fixture.package.clone();
    offered.declarations[0].values[0].operation = w::ValueOperation::Number {
        value: w::Number(NumberWire::Rational {
            numerator: "2".into(),
            denominator: "4".into(),
        }),
    };
    // Raw-number checking also precedes the writer's entry census.
    let report = artifact::encode_candidate(
        &offered,
        Limits {
            entries: 0,
            ..Limits::default()
        },
    );
    failure(&report, Error::Numeric(NumberError::UnreducedRational));
    assert_eq!(report.usage().entries, 0);
    assert_eq!(report.locus(), Some(&offered.declarations[0].locus));
}

#[test]
#[trace("TC-121", "FR-042-AC-2", "FR-042-AC-7")]
fn writer_structural_integers_have_the_same_closed_cap_as_the_reader() {
    let fixture = Fixture::new();
    for index in [0, 1_048_576] {
        let mut offered = fixture.package.clone();
        offered.declarations[0].values[0].original_expression = index;
        let candidate = artifact::encode_candidate(&offered, Limits::default())
            .into_result()
            .unwrap();
        let decoded: serde_json::Value = serde_json::from_slice(candidate.bytes()).unwrap();
        assert_eq!(
            decoded["declarations"][0]["values"][0]["original_expression"],
            json!(index)
        );
    }
    for index in [1_048_577, u32::MAX] {
        let mut offered = fixture.package.clone();
        offered.declarations[0].values[0].original_expression = index;
        failure(
            &artifact::encode_candidate(&offered, Limits::default()),
            Error::Invalid(Invalid::StructuralInteger),
        );
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-4", "FR-042-AC-5", "FR-042-AC-7")]
fn repeated_control_names_keep_distinct_branch_paths_and_selected_owners() {
    let fixture = Fixture::branched();
    let candidate = fixture.candidate();
    let report = fixture.read(
        candidate.bytes(),
        &Fixture::seal(candidate.bytes()),
        Limits::default(),
    );
    let package = report.result().expect("separate branch scopes").package();
    let w::Body::Protocol {
        controls,
        causal_edges,
        ..
    } = &package.declarations[0].body
    else {
        unreachable!()
    };
    assert_eq!((controls.len(), causal_edges.len()), (5, 10));
    assert_eq!(
        (controls[1].name.as_str(), controls[3].name.as_str()),
        ("Nested", "Nested")
    );
    assert_eq!(
        (controls[2].name.as_str(), controls[4].name.as_str()),
        ("Same", "Same")
    );
    for (left, right) in [(1, 3), (2, 4)] {
        assert_ne!(controls[left].original_node, controls[right].original_node);
        assert_ne!(controls[left].locus, controls[right].locus);
    }
    assert_eq!(
        controls[1].operation,
        w::ControlOperation::Sequence {
            children: vec![handle(2)]
        }
    );
    assert_eq!(
        controls[3].operation,
        w::ControlOperation::Sequence {
            children: vec![handle(4)]
        }
    );
    for control in controls {
        let span = &control.locus.span;
        assert!(
            package.sources[0].text[span.start as usize..span.end as usize].contains(&control.name)
        );
    }
    let mut offered = fixture.package.clone();
    let w::Body::Protocol { controls, .. } = &mut offered.declarations[0].body else {
        unreachable!()
    };
    controls[3].operation = w::ControlOperation::Sequence {
        children: vec![handle(2)],
    };
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Owner));
    let mut offered = fixture.package.clone();
    let w::Body::Protocol { causal_edges, .. } = &mut offered.declarations[0].body else {
        unreachable!()
    };
    causal_edges[0].owner = handle(1);
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Control));
    let mut offered = fixture.package.clone();
    let w::Body::Protocol { causal_edges, .. } = &mut offered.declarations[0].body else {
        unreachable!()
    };
    causal_edges[0].owner.declaration = 1;
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Owner));
}

#[test]
#[trace("TC-121", "FR-042-AC-9")]
fn caller_owned_string_is_reserved_before_json_escaping_or_output() {
    let fixture = Fixture::new();
    let mut offered = fixture.package.clone();
    // The first key is four bytes; the next raw string is 4096 bytes although
    // its escaped JSON spelling would occupy 8192 bytes, plus delimiters.
    // This invalid header tests candidate preflight, not reader admission.
    offered.wire = "\n".repeat(4096);
    for dimension in [Dimension::ContentBytes, Dimension::ByteWork] {
        let mut limits = Limits::default();
        set_limit(&mut limits, dimension, 4099);
        let report = artifact::encode_candidate(&offered, limits);
        let Err(Error::Incomplete(exhaustion)) = report.result() else {
            panic!("expected preflight exhaustion")
        };
        assert_eq!(exhaustion.dimension, dimension);
        assert_eq!(
            (exhaustion.used, exhaustion.requested, exhaustion.limit),
            (4, 4096, 4099)
        );
        assert_eq!(report.usage().output_bytes, 0);
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-4", "FR-042-AC-7")]
fn real_model_values_and_all_declaration_families_preserve_exact_references() {
    let fixture = Fixture::families();
    let candidate = fixture.candidate();
    let report = fixture.read(
        candidate.bytes(),
        &Fixture::seal(candidate.bytes()),
        Limits::default(),
    );
    let package = report
        .result()
        .unwrap_or_else(|error| {
            panic!(
                "four family reader data: {error:?}, locus {:?}",
                report.locus()
            )
        })
        .package();
    assert_eq!(package.declarations.len(), 5);
    assert!(matches!(
        package.declarations[1].body,
        w::Body::Predicate { result: 1, .. }
    ));
    assert!(matches!(
        package.declarations[2].body,
        w::Body::State {
            clause_kind: w::ClauseKind::Invariant,
            ..
        }
    ));
    assert!(matches!(
        package.declarations[3].body,
        w::Body::Temporal { clock: 0, .. }
    ));
    assert_eq!(package.declarations[1].values.len(), 14);
    assert_eq!(package.declarations[2].requires, [1]);
    assert_eq!(package.declarations[3].requires, [1]);
    assert_eq!(
        package.declarations[4].values[2].operation,
        w::ValueOperation::Text { value: "é".into() }
    );
    let mutations: [Mutation<w::Package>; 8] = [
        (
            |p| {
                p.declarations[1].values[1].operation = w::ValueOperation::Number {
                    value: w::Number(NumberWire::Integer {
                        decimal: "11".into(),
                    }),
                }
            },
            Error::Invalid(Invalid::NumericDomain),
        ),
        (
            |p| p.declarations[1].binders[1].initializer = w::Nullable(None),
            Error::Invalid(Invalid::Binding),
        ),
        (
            |p| {
                p.declarations[2].values[0].operation = w::ValueOperation::Call {
                    predicate: 1,
                    arguments: vec![],
                }
            },
            Error::Invalid(Invalid::Call),
        ),
        (
            |p| {
                p.declarations[2].values[0].operation = w::ValueOperation::Call {
                    predicate: 2,
                    arguments: vec![setup::owned(2, 2)],
                }
            },
            Error::Invalid(Invalid::Call),
        ),
        (
            |p| {
                p.declarations[2].values[2].operation = w::ValueOperation::Field {
                    base: setup::owned(1, 2),
                    field: w::ExportRef {
                        model: 0,
                        export: 1,
                    },
                }
            },
            Error::Invalid(Invalid::Owner),
        ),
        (
            |p| p.declarations[3].bindings[0].kind = w::BindingKind::Progress,
            Error::Invalid(Invalid::Binding),
        ),
        (
            |p| {
                p.declarations[3].temporal[0].operation = w::TemporalOperation::Unary {
                    operator: w::TemporalUnary::Eventually,
                    interval: w::Nullable(None),
                    value: setup::owned(3, 1),
                }
            },
            Error::Invalid(Invalid::Profile),
        ),
        (
            |p| {
                p.declarations[4].values[2].operation = w::ValueOperation::Text {
                    value: "é".repeat(257),
                }
            },
            Error::Invalid(Invalid::NumericDomain),
        ),
    ];
    for (mutate, expected) in mutations {
        let mut offered = fixture.package.clone();
        mutate(&mut offered);
        failure(&fixture.offered(&offered), expected);
    }
}

/// Tracing: TC-131.
#[trace("TC-131", "FR-047-AC-8")]
#[test]
fn composed_reader_refuses_parent_instead_of_inventing_graph_semantics() {
    let fixture = Fixture::families();
    let mut offered = fixture.package.clone();
    offered.declarations[2].values[0].profile = fixture.definition(R::StateGraph);
    offered.declarations[2].requires.clear();
    offered.declarations[2].values[0].operation = w::ValueOperation::Parent {
        reference: setup::owned(2, 1),
        edge: fixture.export(w::ExportKind::Field, &["Node", "signed"]),
        universe: fixture.export(w::ExportKind::Population, &["Node", "nodes"]),
    };
    failure(
        &fixture.offered(&offered),
        Error::Unsupported(Unsupported::Feature),
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-4", "FR-042-AC-7")]
fn selected_provenance_marker_is_distinct_from_an_evaluation_cycle() {
    let fixture = Fixture::families();
    let mut offered = fixture.package.clone();
    offered.declarations[1].values[0].origin = w::Origin::Selected {
        value: setup::owned(1, 0),
    };
    let report = fixture.offered(&offered);
    assert_eq!(
        report
            .result()
            .expect("self provenance marker")
            .package()
            .declarations[1]
            .values[0]
            .origin,
        w::Origin::Selected {
            value: setup::owned(1, 0)
        }
    );
    offered.declarations[1].values[0].origin = w::Origin::Selected {
        value: setup::owned(0, 0),
    };
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Owner));
    offered.declarations[1].values[0].origin = w::Origin::Selected {
        value: setup::owned(1, 10_000),
    };
    failure(
        &fixture.offered(&offered),
        Error::Invalid(Invalid::Reference),
    );
    offered.declarations[1].values[0].origin = w::Origin::Selected {
        value: setup::owned(1, 1),
    };
    offered.declarations[1].values[1].origin = w::Origin::Selected {
        value: setup::owned(1, 0),
    };
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Cycle));
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-6", "FR-042-AC-7")]
fn choice_repeat_await_and_operation_events_preserve_static_edges_and_binding_kinds() {
    let fixture = Fixture::controlled();
    let candidate = fixture.candidate();
    let report = fixture.read(
        candidate.bytes(),
        &Fixture::seal(candidate.bytes()),
        Limits::default(),
    );
    let package = report
        .result()
        .unwrap_or_else(|error| {
            panic!(
                "finite static control records: {error:?}, locus {:?}",
                report.locus()
            )
        })
        .package();
    let declaration = &package.declarations[0];
    let w::Body::Protocol {
        roles,
        controls,
        causal_edges,
        ..
    } = &declaration.body
    else {
        unreachable!()
    };
    assert_eq!(
        (roles.len(), controls.len(), causal_edges.len()),
        (1, 14, 33)
    );
    assert!(matches!(
        controls[1].operation,
        w::ControlOperation::Event {
            event: w::Event::Attempt { instance: 5, .. },
            ..
        }
    ));
    assert!(matches!(
        controls[2].operation,
        w::ControlOperation::Event {
            event: w::Event::Effect { instance: 6, .. },
            ..
        }
    ));
    assert!(matches!(
        controls[3].operation,
        w::ControlOperation::Choice { .. }
    ));
    let w::ControlOperation::Repeat { maximum, .. } = &controls[6].operation else {
        unreachable!()
    };
    assert_eq!(maximum, &setup::integer(0));
    assert!(matches!(
        controls[9].operation,
        w::ControlOperation::Await { clock: 4, .. }
    ));
    assert!(matches!(
        controls[13].operation,
        w::ControlOperation::Commit { instance: 9, .. }
    ));
    assert_eq!(declaration.bindings[5].kind, w::BindingKind::Attempt);
    assert_eq!(declaration.bindings[6].kind, w::BindingKind::Effect);
    assert_eq!(declaration.bindings[9].kind, w::BindingKind::Commit);
    assert_eq!(declaration.bindings[10].kind, w::BindingKind::Progress);
    assert_eq!(
        declaration.bindings[10].subject,
        w::Subject::Control { control: handle(9) }
    );
    assert_eq!(declaration.bindings[10].requires, [4]);
    // These are independent future input requirements, not runtime identities.
    assert_ne!(
        declaration.bindings[5].subject,
        declaration.bindings[6].subject
    );
    let mutants: [Mutation<Vec<w::Control>>; 7] = [
        (
            |controls| {
                let w::ControlOperation::Choice { cases, .. } = &mut controls[3].operation else {
                    unreachable!()
                };
                cases[1].label = "yes".into();
            },
            Error::Invalid(Invalid::Duplicate),
        ),
        (
            |controls| {
                let w::ControlOperation::Choice { cases, .. } = &mut controls[3].operation else {
                    unreachable!()
                };
                cases[1].body = handle(4);
            },
            Error::Invalid(Invalid::Owner),
        ),
        (
            |controls| {
                let w::ControlOperation::Repeat { maximum, .. } = &mut controls[6].operation else {
                    unreachable!()
                };
                *maximum = setup::integer(-1);
            },
            Error::Invalid(Invalid::NumericDomain),
        ),
        (
            |controls| {
                let w::ControlOperation::Await { after, .. } = &mut controls[9].operation else {
                    unreachable!()
                };
                *after = w::AwaitAnchor::Event { node: handle(4) };
            },
            Error::Invalid(Invalid::Control),
        ),
        (
            |controls| {
                let w::ControlOperation::Await { clock, .. } = &mut controls[9].operation else {
                    unreachable!()
                };
                *clock = 5;
            },
            Error::Invalid(Invalid::Binding),
        ),
        (
            |controls| {
                let w::ControlOperation::Event {
                    event: w::Event::Effect { attempt, .. },
                    ..
                } = &mut controls[2].operation
                else {
                    unreachable!()
                };
                *attempt = handle(4);
            },
            Error::Invalid(Invalid::Control),
        ),
        (
            |controls| {
                let w::ControlOperation::Commit { instance, .. } = &mut controls[13].operation
                else {
                    unreachable!()
                };
                *instance = 6;
            },
            Error::Invalid(Invalid::Binding),
        ),
    ];
    for (mutate, expected) in mutants {
        let mut offered = fixture.package.clone();
        let w::Body::Protocol { controls, .. } = &mut offered.declarations[0].body else {
            unreachable!()
        };
        mutate(controls);
        failure(&fixture.offered(&offered), expected);
    }
    let mut offered = fixture.package.clone();
    let w::Body::Protocol { causal_edges, .. } = &mut offered.declarations[0].body else {
        unreachable!()
    };
    causal_edges
        .iter_mut()
        .find(|edge| edge.kind == w::EdgeKind::RepeatProgress)
        .unwrap()
        .maximum = w::Nullable(Some(setup::integer(1)));
    failure(&fixture.offered(&offered), Error::Invalid(Invalid::Control));
    let await_mutants: [fn(&mut Vec<w::BindingRequirement>); 3] = [
        |bindings| {
            bindings.remove(10);
            for binding in bindings {
                for required in &mut binding.requires {
                    if *required > 10 {
                        *required -= 1;
                    }
                }
            }
        },
        |bindings| bindings[10].subject = w::Subject::Control { control: handle(6) },
        |bindings| bindings[10].requires.clear(),
    ];
    for mutate in await_mutants {
        let mut offered = fixture.package.clone();
        mutate(&mut offered.declarations[0].bindings);
        failure(&fixture.offered(&offered), Error::Invalid(Invalid::Binding));
    }
}

/// RFC 8785 (JCS) canonical bytes of a small consumer protocol-result document.
/// Sorted ASCII member names, no insignificant whitespace and bare safe integers
/// fix this spelling without importing a second canonicalizer into the producer.
const RESULT_JCS: &[u8] = br#"{"accepted":true,"protocolResult":"ok","workflow":1}"#;

/// Digest of the consumer's RFC 8785/JCS protocol-result domain. The producer
/// neither computes nor accepts this identity for any of its own references.
fn result_jcs_digest() -> ByteDigest {
    // Independently spelled members reach the same canonical bytes, so the
    // vector above is the document's canonical form and not one chosen spelling.
    assert_eq!(
        serde_json::to_vec(&json!({"workflow": 1, "protocolResult": "ok", "accepted": true}))
            .unwrap(),
        RESULT_JCS
    );
    ByteDigest::of(RESULT_JCS)
}

/// Every producer-owned digest domain the artifact retains: raw model package
/// bytes, the model's canonical IR object, original native source text and the
/// selected producer implementation artifact.
fn producer_digests(fixture: &Fixture) -> [ByteDigest; 4] {
    let canonical = fixture
        .model
        .environment()
        .canonical_declaration(ir::CanonicalProfile::V1)
        .unwrap();
    [
        fixture.model.digest(),
        ByteDigest::of(canonical.bytes().as_slice()),
        fixture.package.sources[0].artifact.digest,
        fixture.package.producer.binary.digest,
    ]
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-7")]
fn a_producer_source_model_or_config_digest_is_never_reinterpreted_as_the_compiled_artifact_seal() {
    let fixture = Fixture::new();
    let candidate = fixture.candidate();
    let accepted = Fixture::seal(candidate.bytes());
    // Positive control: the same producer references admit under the real seal,
    // so each refusal below is attributable to the substituted digest alone.
    fixture
        .read(candidate.bytes(), &accepted, Limits::default())
        .result()
        .expect("complete reader fixture");
    for producer in producer_digests(&fixture) {
        assert_ne!(producer, accepted.digest);
        // The compiled-artifact byte domain refuses a producer-owned digest on
        // its own seal cause, with no producer reference consulted first.
        let mut substituted = accepted.clone();
        substituted.digest = producer;
        failure(
            &fixture.read(candidate.bytes(), &substituted, Limits::default()),
            Error::Invalid(Invalid::Seal),
        );
        // Offering the same producer digest as an accepted contract reference
        // refuses independently, on the selection cause rather than the seal.
        let mut offered = fixture.package.clone();
        offered.contract.digest = producer;
        failure(
            &fixture.offered(&offered),
            Error::Invalid(Invalid::Selection),
        );
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-7")]
fn a_protocol_result_jcs_digest_is_never_accepted_in_a_producer_owned_reference() {
    let jcs = result_jcs_digest();
    // The selected producer implementation artifact keeps its own byte identity.
    let fixture = Fixture::new();
    let mut offered = fixture.package.clone();
    offered.producer.binary.digest = jcs;
    failure(
        &fixture.offered(&offered),
        Error::Invalid(Invalid::Selection),
    );
    // The model package byte authority refuses even when the offered bytes are
    // the result document itself, so its own seal cause cannot mask the model
    // cause: the admitted native model's raw-byte digest remains the authority.
    let mut fixture = Fixture::new();
    let model = fixture.package.models[0].artifact as usize;
    assert_ne!(fixture.package.dependencies[model].artifact.digest, jcs);
    fixture.package.dependencies[model].artifact.digest = jcs;
    fixture.dependency_bytes[model] = RESULT_JCS.to_vec();
    // Every embedded copy of that producer reference is substituted too, so the
    // reader's separate binding-authority selection cannot mask the model cause.
    let identity = fixture.package.dependencies[model]
        .artifact
        .identity
        .clone();
    for declaration in &mut fixture.package.declarations {
        for binding in &mut declaration.bindings {
            if binding.authority.identity == identity {
                binding.authority.digest = jcs;
            }
        }
    }
    let candidate = fixture.candidate();
    failure(
        &fixture.read(
            candidate.bytes(),
            &Fixture::seal(candidate.bytes()),
            Limits::default(),
        ),
        Error::Invalid(Invalid::Model),
    );
    // The model's original producer source keeps its foreign-locus cause.
    let mut fixture = Fixture::new();
    assert_ne!(fixture.foreign_source.digest, jcs);
    fixture.foreign_source.digest = jcs;
    let candidate = fixture.candidate();
    failure(
        &fixture.read(
            candidate.bytes(),
            &Fixture::seal(candidate.bytes()),
            Limits::default(),
        ),
        Error::Invalid(Invalid::ForeignLocus),
    );
    // The native source text retains its exact raw-byte seal cause.
    let mut fixture = Fixture::new();
    assert_ne!(fixture.package.sources[0].artifact.digest, jcs);
    fixture.package.sources[0].artifact.digest = jcs;
    let candidate = fixture.candidate();
    failure(
        &fixture.read(
            candidate.bytes(),
            &Fixture::seal(candidate.bytes()),
            Limits::default(),
        ),
        Error::Invalid(Invalid::Seal),
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-7")]
fn an_admitted_producer_reference_is_retained_byte_exactly_without_recanonicalization() {
    let fixture = Fixture::new();
    let candidate = fixture.candidate();
    let report = fixture.read(
        candidate.bytes(),
        &Fixture::seal(candidate.bytes()),
        Limits::default(),
    );
    let admitted = report.result().expect("complete reader fixture");
    let package = admitted.package();
    let canonical = fixture
        .model
        .environment()
        .canonical_declaration(ir::CanonicalProfile::V1)
        .unwrap();
    // The retained model reference hashes the producer's exact artifact bytes;
    // no canonical projection of the same model is substituted for them.
    let model = &package.dependencies[package.models[0].artifact as usize].artifact;
    assert_eq!(model.digest, ByteDigest::of(fixture.model.artifact_bytes()));
    assert_eq!(model.digest, fixture.model.digest());
    assert_ne!(model.digest, ByteDigest::of(canonical.bytes().as_slice()));
    // Original native source bytes and their producer digest survive unparsed.
    let source = &package.sources[0];
    assert_eq!(source.text, fixture.package.sources[0].text);
    assert_eq!(
        source.artifact.digest,
        ByteDigest::of(source.text.as_bytes())
    );
    assert_eq!(
        fixture.foreign_source.digest,
        ByteDigest::of(fixture.model.source().source().text().as_bytes())
    );
    // The compiled-artifact digest stays in the consumer's own byte domain and
    // never collapses onto a producer reference or the result JCS domain.
    assert_eq!(admitted.digest(), ByteDigest::of(candidate.bytes()));
    for producer in producer_digests(&fixture) {
        assert_ne!(admitted.digest(), producer);
        assert_ne!(result_jcs_digest(), producer);
    }
    assert_ne!(admitted.digest(), result_jcs_digest());
    // Re-encoding the admitted package reproduces the exact offered bytes, so
    // admission recanonicalized no retained reference.
    assert_eq!(
        artifact::encode_candidate(package, Limits::default())
            .into_result()
            .unwrap()
            .bytes(),
        candidate.bytes()
    );
}
