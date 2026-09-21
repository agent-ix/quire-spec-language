// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-125 group 6: mapping support classification through a strict
//! compiled-protocol version-2 package.
//!
//! FR-045-AC-6. Each declaration is compiled for real and emitted twice: once
//! through the version-1 producer and strict reader, and once through the
//! version-2 producer and a strict reader whose temporal expectation is
//! re-derived from the authored sources and the registered catalog rather than
//! projected from the producer's table. Expected dispositions are spelt here
//! from the reviewed support table, and expected selections from the registered
//! catalog and the emitted bytes, never read back from the classifier.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeLimits};
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::protocol_artifact::{
    native, v2, wire as w, AdmittedPackage, Limits, NumberWire,
};
use quire_spec_language::temporal::{
    self, Closure, Dimension, Error, Refusal, Subject, Support, Unmatched,
};
use quire_spec_language::ByteDigest;
use setup::{definition_bytes, Inputs, TemporalDefinitionExpectation, Unit};

/// One authored declaration per registered profile, each reaching a bounded
/// future operator and no past operator.
const DECLARATIONS: [(&str, R); 3] = [
    ("ByEvent", R::EventPosition),
    ("BySample", R::FixedSample),
    ("ByTimestamp", R::TimestampedWindow),
];

fn inputs() -> Inputs {
    Inputs::new(&[
        Unit {
            name: "event-time",
            body: "temporal ByEvent using T over (view: M::Plain) clock \"event-clock\" on origin { always[0,1] holds(view.ready) }",
            declarations: &["ByEvent"],
        },
        Unit {
            name: "sample-time",
            body: "temporal BySample using F over (view: M::Plain) clock \"sample-clock\" on origin { eventually[0,2] holds(view.ready) }",
            declarations: &["BySample"],
        },
        Unit {
            name: "timestamp-time",
            body: "temporal ByTimestamp using W over (view: M::Plain) clock \"timestamp-clock\" on origin { always[0,1] holds(view.ready) }",
            declarations: &["ByTimestamp"],
        },
        // A compiled package must carry a protocol family; this consumer
        // requires every temporal declaration above.
        Unit {
            name: "consumer",
            body: "protocol Flow using P over (view: M::Plain) on origin {
                role Service on M::Node;
                requires temporal ByEvent;
                requires temporal BySample;
                requires temporal ByTimestamp;
                run sequence Main {
                    event Happened by Service as (happened: M::Plain) { happened.ready };
                }
                finish Closed as (closed: M::Plain) { closed.ready };
            }",
            declarations: &["Flow"],
        },
    ])
}

/// The exact clock configuration each declaration selects, in declaration order.
fn clocks() -> [v2::wire::ClockConfiguration; 3] {
    [
        v2::wire::ClockConfiguration::EventPosition {
            sequence_authority: "orders".into(),
        },
        v2::wire::ClockConfiguration::FixedSample {
            epoch: w::Number(NumberWire::Integer {
                decimal: "0".into(),
            }),
            period: w::Number(NumberWire::Rational {
                numerator: "1".into(),
                denominator: "2".into(),
            }),
            unit: "second".into(),
        },
        v2::wire::ClockConfiguration::TimestampedEvent {
            timestamp_unit: "millisecond".into(),
        },
    ]
}

/// The original definition artifact the caller selected for one registered
/// definition, found by identity and exact bytes.
fn definition_artifact<'a>(
    selected: &'a native::Selections<'a>,
    definition: R,
) -> &'a w::ArtifactRef {
    selected
        .dependencies
        .iter()
        .find(|dependency| {
            dependency.artifact.identity == definition.identity()
                && dependency.bytes == definition_bytes(definition)
        })
        .expect("selected original definition bytes")
        .artifact
}

/// Compile the fixture once and hand the test the strict version-1 and
/// strict version-2 admissions of the same proofs and selections, with the
/// raw-byte digest of the emitted version-2 package computed here.
fn with_both(
    test: impl FnOnce(&native::Selections<'_>, &AdmittedPackage, &v2::AdmittedPackage, ByteDigest),
) {
    let inputs = inputs();
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            let namespace = proofs.types().binding().namespace();
            let spans: Vec<_> = DECLARATIONS
                .iter()
                .map(|(name, _)| {
                    let [id] = namespace.lookup(name) else {
                        panic!("one authored declaration named {name}")
                    };
                    let span = namespace.syntax(*id).expect("authored syntax").span;
                    w::Span {
                        start: u32::try_from(span.start).expect("span start fits"),
                        end: u32::try_from(span.end).expect("span end fits"),
                    }
                })
                .collect();
            let revisions: Vec<_> = DECLARATIONS
                .iter()
                .map(|(_, definition)| w::Revision {
                    namespace: selected.definition_revision_namespace.into(),
                    value: definition.revision().into(),
                })
                .collect();
            let producer_clocks = clocks();
            let producer: Vec<_> = DECLARATIONS
                .iter()
                .enumerate()
                .map(|(index, (_, definition))| native::TemporalSelection {
                    source: &inputs.source_references[index],
                    span: &spans[index],
                    definition_identity: definition.identity(),
                    definition_revision: &revisions[index],
                    definition_artifact: definition_artifact(selected, *definition),
                    clock: &producer_clocks[index],
                })
                .collect();
            // The reader selection is re-derived from the authored sources and the
            // registered catalog, never projected from the producer's table.
            let expected: Vec<_> = DECLARATIONS
                .iter()
                .zip(clocks())
                .enumerate()
                .map(|(index, ((name, definition), clock))| {
                    inputs.temporal_expectation(
                        proofs,
                        index,
                        name,
                        TemporalDefinitionExpectation {
                            identity: definition.identity().into(),
                            revision: w::Revision {
                                namespace: selected.definition_revision_namespace.into(),
                                value: definition.revision().into(),
                            },
                            artifact: definition_artifact(selected, *definition).clone(),
                            clock,
                        },
                    )
                })
                .collect();

            let emitted_v2 = native::admit_v2(proofs, selected, &producer, Limits::default())
                .into_result()
                .expect("version-2 emission");
            let admitted_v2 = inputs
                .read_v2(proofs, &emitted_v2, &expected)
                .into_result()
                .expect("strict version-2 reader");

            let family = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("version-1 admission");
            let emitted_v1 = native::emit(&family, Limits::default())
                .into_result()
                .expect("version-1 emission");
            let admitted_v1 = inputs
                .read(proofs, &emitted_v1)
                .into_result()
                .expect("strict version-1 reader");

            test(
                selected,
                &admitted_v1,
                &admitted_v2,
                ByteDigest::of(emitted_v2.bytes()),
            );
        },
    );
}

fn position(declarations: &[w::Declaration], name: &str) -> usize {
    declarations
        .iter()
        .position(|declaration| declaration.name == name)
        .expect("authored temporal declaration")
}

/// FR-045-AC-6: a strict version-2 classification returns the same disposition
/// as the version-1 classification of the same declaration under both requested
/// closures, and additionally retains the authenticated package digest,
/// declaration, definition identity, revision and definition artifact digest.
/// The version-1 classification retains no authenticated selection.
#[trace("TC-125", "FR-045-AC-6")]
#[test]
fn a_version_two_classification_retains_its_authenticated_definition_selection() {
    with_both(|selected, v1_package, v2_package, emitted_digest| {
        for (name, definition) in DECLARATIONS {
            let v1_at = position(&v1_package.package().declarations, name);
            let v2_at = position(&v2_package.inherited().declarations, name);
            for closure in [Closure::Closed, Closure::Open] {
                let unauthenticated = temporal::mapping_support(v1_package, v1_at, closure)
                    .expect("a version-1 temporal declaration classifies");
                let authenticated = temporal::mapping_support_v2(v2_package, v2_at, closure)
                    .expect("a version-2 temporal declaration classifies");

                let expected = match (definition, closure) {
                    (R::TimestampedWindow, _) => None,
                    (_, Closure::Closed) => Some("mltl.closed-trace/v1"),
                    (_, Closure::Open) => Some("mltl.online-prefix/v1"),
                };
                match (expected, &authenticated.support) {
                    (None, Support::Unsupported { dimensions }) => assert_eq!(
                        dimensions.as_slice(),
                        [Unmatched::FiniteWindow],
                        "{name}: the finite-window dimension alone",
                    ),
                    (Some(target), Support::Supported { target: named, .. }) => {
                        assert_eq!(named.identity(), target, "{name} {closure:?}: TL target");
                    }
                    (expected, support) => {
                        panic!("{name} {closure:?}: expected {expected:?}, classified {support:?}")
                    }
                }
                assert_eq!(
                    authenticated.support, unauthenticated.support,
                    "{name} {closure:?}: authentication does not change the disposition",
                );
                assert_eq!(
                    authenticated.retained, unauthenticated.retained,
                    "{name} {closure:?}: the same native subject, profile and activation",
                );

                assert!(
                    unauthenticated.authenticated().is_none(),
                    "{name}: a version-1 classification authenticates nothing",
                );
                let selection = authenticated
                    .authenticated()
                    .unwrap_or_else(|| panic!("{name}: a version-2 selection is retained"));
                assert_eq!(
                    selection.package_digest(),
                    emitted_digest,
                    "{name}: the digest of the emitted package bytes",
                );
                assert_eq!(selection.declaration(), v2_at);
                assert_eq!(selection.definition_identity(), definition.identity());
                assert_eq!(
                    selection.definition_revision(),
                    &w::Revision {
                        namespace: selected.definition_revision_namespace.into(),
                        value: definition.revision().into(),
                    },
                    "{name}: the namespaced registered revision",
                );
                assert_eq!(
                    selection.definition_artifact(),
                    definition_artifact(selected, definition).digest,
                    "{name}: the independently selected definition artifact",
                );
            }
        }
    });
}

/// FR-045-AC-6: the version-2 entry point refuses a declaration outside the
/// admitted package, including the largest index a caller can name, and a
/// non-temporal declaration, each with its exact located refusal rather than a
/// classification without an authenticated selection.
#[trace("TC-125", "FR-045-AC-6")]
#[test]
fn a_version_two_request_outside_the_package_or_not_temporal_is_refused() {
    with_both(|_, _, v2_package, _| {
        let at = |declaration| Subject {
            declaration,
            ..Subject::default()
        };
        let declarations = v2_package.inherited().declarations.len();
        assert_eq!(
            temporal::mapping_support_v2(v2_package, declarations, Closure::Closed),
            Err(Error::Refused(Refusal::Reference {
                subject: at(declarations)
            })),
            "a declaration outside the package is refused",
        );
        assert_eq!(
            temporal::mapping_support_v2(v2_package, usize::MAX, Closure::Open),
            Err(Error::Refused(Refusal::Reference {
                subject: at(usize::MAX)
            })),
            "the largest declaration index is outside the package and refused",
        );
        let flow = position(&v2_package.inherited().declarations, "Flow");
        assert_eq!(
            temporal::mapping_support_v2(v2_package, flow, Closure::Closed),
            Err(Error::Refused(Refusal::Binding {
                dimension: Dimension::Profile,
                subject: at(flow),
            })),
            "a protocol declaration has no temporal profile to classify",
        );
    });
}
