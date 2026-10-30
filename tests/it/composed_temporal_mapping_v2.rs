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

use crate::support::native_protocol as setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeLimits};
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::protocol_artifact::{
    native, v2, wire as w, AdmittedPackage, Limits, NumberWire,
};
use quire_spec_language::temporal::{
    self, Closure, Dimension, Error, Refusal, Subject, Support, Unmatched,
};
use setup::{Inputs, TemporalDefinitionExpectation, Unit};

/// Which operator directions a declaration's body reaches.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// A bounded future operator and no past operator.
    Future,
    /// A bounded past operator and no future operator.
    PastOnly,
    /// Both a bounded past and a bounded future operator.
    Mixed,
}

/// One authored declaration per registered profile and operator direction.
const DECLARATIONS: [(&str, R, Kind); 13] = [
    ("ByEvent", R::EventPosition, Kind::Future),
    ("BySample", R::FixedSample, Kind::Future),
    ("ByTimestamp", R::TimestampedWindow, Kind::Future),
    ("OnceEvent", R::EventPosition, Kind::PastOnly),
    ("HistoricallySample", R::FixedSample, Kind::PastOnly),
    ("SinceEvent", R::EventPosition, Kind::PastOnly),
    ("TriggeredSample", R::FixedSample, Kind::PastOnly),
    ("PastTimestamp", R::TimestampedWindow, Kind::PastOnly),
    ("MixedEvent", R::EventPosition, Kind::Mixed),
    ("MixedTimestamp", R::TimestampedWindow, Kind::Mixed),
    ("MixedUntilEvent", R::EventPosition, Kind::Mixed),
    ("MixedReleaseSample", R::FixedSample, Kind::Mixed),
    ("UntilSample", R::FixedSample, Kind::Future),
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
        Unit {
            name: "once-event-clock",
            body: "temporal OnceEvent using T over (view: M::Plain) clock \"once-event-clock\" on origin { once[0,1] holds(view.ready) }",
            declarations: &["OnceEvent"],
        },
        Unit {
            name: "historically-sample-clock",
            body: "temporal HistoricallySample using F over (view: M::Plain) clock \"historically-sample-clock\" on origin { historically[0,2] holds(view.ready) }",
            declarations: &["HistoricallySample"],
        },
        Unit {
            name: "since-event-clock",
            body: "temporal SinceEvent using T over (view: M::Plain) clock \"since-event-clock\" on origin { holds(view.ready) since[1,2] holds(view.ready) }",
            declarations: &["SinceEvent"],
        },
        Unit {
            name: "triggered-sample-clock",
            body: "temporal TriggeredSample using F over (view: M::Plain) clock \"triggered-sample-clock\" on origin { holds(view.ready) triggered[1,2] holds(view.ready) }",
            declarations: &["TriggeredSample"],
        },
        Unit {
            name: "past-timestamp-clock",
            body: "temporal PastTimestamp using W over (view: M::Plain) clock \"past-timestamp-clock\" on origin { once[0,1] holds(view.ready) }",
            declarations: &["PastTimestamp"],
        },
        Unit {
            name: "mixed-event-clock",
            body: "temporal MixedEvent using T over (view: M::Plain) clock \"mixed-event-clock\" on origin { once[0,1] eventually[0,1] holds(view.ready) }",
            declarations: &["MixedEvent"],
        },
        Unit {
            name: "mixed-timestamp-clock",
            body: "temporal MixedTimestamp using W over (view: M::Plain) clock \"mixed-timestamp-clock\" on origin { once[0,1] eventually[0,1] holds(view.ready) }",
            declarations: &["MixedTimestamp"],
        },
        Unit {
            name: "mixed-until-event-clock",
            body: "temporal MixedUntilEvent using T over (view: M::Plain) clock \"mixed-until-event-clock\" on origin { once[0,1] holds(view.ready) and (holds(view.ready) until[0,1] holds(view.ready)) }",
            declarations: &["MixedUntilEvent"],
        },
        Unit {
            name: "mixed-release-sample-clock",
            body: "temporal MixedReleaseSample using F over (view: M::Plain) clock \"mixed-release-sample-clock\" on origin { once[0,1] holds(view.ready) and (holds(view.ready) release[0,1] holds(view.ready)) }",
            declarations: &["MixedReleaseSample"],
        },
        Unit {
            name: "until-sample-clock",
            body: "temporal UntilSample using F over (view: M::Plain) clock \"until-sample-clock\" on origin { holds(view.ready) until[0,1] holds(view.ready) }",
            declarations: &["UntilSample"],
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
                requires temporal OnceEvent;
                requires temporal HistoricallySample;
                requires temporal SinceEvent;
                requires temporal TriggeredSample;
                requires temporal PastTimestamp;
                requires temporal MixedEvent;
                requires temporal MixedTimestamp;
                requires temporal MixedUntilEvent;
                requires temporal MixedReleaseSample;
                requires temporal UntilSample;
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
fn clocks() -> Vec<v2::wire::ClockConfiguration> {
    DECLARATIONS
        .iter()
        .map(|(_, definition, _)| match definition {
            R::EventPosition => v2::wire::ClockConfiguration::EventPosition {
                sequence_authority: "orders".into(),
            },
            R::FixedSample => v2::wire::ClockConfiguration::FixedSample {
                epoch: w::Number(NumberWire::Integer {
                    decimal: "0".into(),
                }),
                period: w::Number(NumberWire::Rational {
                    numerator: "1".into(),
                    denominator: "2".into(),
                }),
                unit: "second".into(),
            },
            R::TimestampedWindow => v2::wire::ClockConfiguration::TimestampedEvent {
                timestamp_unit: "millisecond".into(),
            },
            other => unreachable!("{other:?} is not a selectable clock profile"),
        })
        .collect()
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
                && dependency.bytes == definition.bytes()
        })
        .expect("selected original definition bytes")
        .artifact
}

/// Compile the fixture once and hand the test the strict version-1 and
/// strict version-2 admissions of the same proofs and selections, with the
/// strict version-1 and strict version-2 admissions.
fn with_both(test: impl FnOnce(&native::Selections<'_>, &AdmittedPackage, &v2::AdmittedPackage)) {
    let inputs = inputs();
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            let namespace = proofs.types().binding().namespace();
            let spans: Vec<_> = DECLARATIONS
                .iter()
                .map(|(name, _, _)| {
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
                .map(|(_, definition, _)| w::Revision {
                    namespace: selected.definition_revision_namespace.into(),
                    value: definition.revision().into(),
                })
                .collect();
            let producer_clocks = clocks();
            let producer: Vec<_> = DECLARATIONS
                .iter()
                .enumerate()
                .map(|(index, (_, definition, _))| native::TemporalSelection {
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
                .map(|(index, ((name, definition, _), clock))| {
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

            test(selected, &admitted_v1, &admitted_v2);
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
/// closures, and additionally retains its declaration and definition selection.
/// The version-1 classification retains no authenticated selection.
#[trace("TC-125", "FR-045-AC-6")]
#[test]
fn a_version_two_classification_retains_its_authenticated_definition_selection() {
    with_both(|selected, v1_package, v2_package| {
        for (name, definition, kind) in DECLARATIONS {
            let v1_at = position(&v1_package.package().declarations, name);
            let v2_at = position(&v2_package.inherited().declarations, name);
            for closure in [Closure::Closed, Closure::Open] {
                let unauthenticated = temporal::mapping_support(v1_package, v1_at, closure)
                    .expect("a version-1 temporal declaration classifies");
                let authenticated = temporal::mapping_support_v2(v2_package, v2_at, closure)
                    .expect("a version-2 temporal declaration classifies");

                // Spelt from FR-045's table, not read back from the classifier.
                let (expected, unmatched): (_, &[Unmatched]) = match (definition, kind, closure) {
                    (R::TimestampedWindow, Kind::Mixed, _) => {
                        (None, &[Unmatched::FiniteWindow, Unmatched::MixedDirection])
                    }
                    (R::TimestampedWindow, _, _) => (None, &[Unmatched::FiniteWindow]),
                    (_, Kind::Mixed, _) => (None, &[Unmatched::MixedDirection]),
                    (_, Kind::PastOnly, _) => (Some("mltl.origin-complete-history/v1"), &[]),
                    (_, Kind::Future, Closure::Closed) => (Some("mltl.closed-trace/v1"), &[]),
                    (_, Kind::Future, Closure::Open) => (Some("mltl.online-prefix/v1"), &[]),
                };
                match (expected, &authenticated.support) {
                    (None, Support::Unsupported { dimensions }) => assert_eq!(
                        dimensions.as_slice(),
                        unmatched,
                        "{name}: every unmatched dimension",
                    ),
                    (
                        Some(target),
                        Support::Supported {
                            target: named,
                            total_sample_valuation,
                            ..
                        },
                    ) => {
                        assert_eq!(named.identity(), target, "{name} {closure:?}: TL target");
                        assert_eq!(
                            *total_sample_valuation,
                            definition == R::FixedSample && matches!(kind, Kind::Future),
                            "{name} {closure:?}: the total-sample-valuation condition",
                        );
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
    with_both(|_, _, v2_package| {
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
