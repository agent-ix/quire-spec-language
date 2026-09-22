// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-132: the occurrence-key-schema slice retains static source ownership while
//! concrete workflow identities and repeat ordinals remain caller supplied.
//! This file does not claim D's unavailable relationship/component/endpoint
//! producer adapter or the other independently tracked TC-132 obligations.

use crate::support::native_protocol as setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::protocol_artifact::{
    self as artifact, native, occurrence_key_schema, wire as w, Dimension, Error, ExactInteger,
    Invalid, Limits, NodeOccurrenceSchema, NodeRole, OccurrenceKeyError, OccurrenceKeySchema,
    RepeatOrdinalSchema, WorkflowInstanceIdentity,
};
use setup::{Inputs, Unit};

const SOURCE: &str = r#"
predicate Other using S (item: M::Node): Boolean { true }

protocol Keyed using P over (view: M::Node) on origin {
    role Service on M::Node;
    role Provider on M::Node;
    channel Messages from Service to Provider carries M::Plain
        ordering unordered delivery [1,1];
    run repeat Outer by Service visible (true) max 9223372036854775807 while { true }
        sequence OuterBody {
            event Tick by Service as (tick: M::Plain) { tick.ready };
            send Sent via Messages as (sent: M::Plain) { sent.ready };
            receive Received via Messages of Sent as (received: M::Plain) { received.ready };
            repeat Inner by Service visible (true) max 0 while { true }
                event Impossible by Service as (impossible: M::Plain) { impossible.ready };
            exhausted event InnerDone by Service as (innerDone: M::Plain) { innerDone.ready };
        }
    exhausted event OuterDone by Service as (outerDone: M::Plain) { outerDone.ready };
    finish Closed as (closed: M::Node) { true };
}
"#;

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

fn with_admitted(test: impl FnOnce(&artifact::AdmittedPackage, &[u8])) {
    let inputs = Inputs::new(&[Unit {
        name: "occurrence-keys",
        body: SOURCE,
        declarations: &["Other", "Keyed"],
    }]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let native = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("source-derived protocol admission");
            let emitted = native::emit(&native, Limits::default())
                .into_result()
                .expect("canonical source-derived bytes");
            let admitted = inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("public parser-free reader admission");
            test(&admitted, emitted.bytes());
        },
    );
}

fn declaration(package: &w::Package, name: &str) -> u32 {
    u32::try_from(
        package
            .declarations
            .iter()
            .position(|declaration| declaration.name == name)
            .expect("fixture declaration"),
    )
    .expect("bounded declaration index")
}

fn index(value: u32) -> usize {
    usize::try_from(value).expect("fixture index fits usize")
}

fn node<'a>(
    package: &'a w::Package,
    schema: &'a OccurrenceKeySchema,
    name: &str,
) -> &'a NodeOccurrenceSchema {
    schema
        .nodes()
        .iter()
        .find(|node| {
            let declaration = &package.declarations[index(node.node().declaration)];
            let w::Body::Protocol { controls, .. } = &declaration.body else {
                return false;
            };
            controls[index(node.node().index)].name == name
        })
        .expect("fixture control node")
}

fn role(declaration: u32, index: u32) -> NodeRole {
    NodeRole::Role(w::Handle { declaration, index })
}

/// Tracing: TC-132; ACs: FR-048-AC-1.
#[test]
#[trace("TC-132", "FR-048-AC-1")]
fn tc_132_public_schema_retains_roles_nested_ordinals_and_caller_workflows() {
    with_admitted(|admitted, emitted| {
        let package = admitted.package();
        let keyed = declaration(package, "Keyed");
        let report = occurrence_key_schema(admitted, keyed, Limits::default());
        let schema = report.result().expect("admitted protocol schema");
        assert_eq!(schema.declaration(), keyed);
        assert_eq!(schema.roles().len(), 2);
        assert_eq!(
            schema.roles()[0].role(),
            &w::Handle {
                declaration: keyed,
                index: 0
            }
        );
        assert_eq!(
            schema.roles()[1].role(),
            &w::Handle {
                declaration: keyed,
                index: 1
            }
        );

        let declaration = &package.declarations[index(keyed)];
        assert_eq!(
            declaration.bindings[index(schema.workflow_binding())].kind,
            w::BindingKind::WorkflowInstance
        );
        let w::Body::Protocol {
            roles, controls, ..
        } = &declaration.body
        else {
            panic!("fixture protocol")
        };
        assert_eq!(schema.roles()[0].instance_binding(), roles[0].instance);
        assert_eq!(schema.roles()[1].instance_binding(), roles[1].instance);

        let outer = node(package, schema, "Outer");
        let outer_body = node(package, schema, "OuterBody");
        let tick = node(package, schema, "Tick");
        let sent = node(package, schema, "Sent");
        let received = node(package, schema, "Received");
        let impossible = node(package, schema, "Impossible");
        let inner_done = node(package, schema, "InnerDone");
        let outer_done = node(package, schema, "OuterDone");
        assert_eq!(outer.role(), &role(keyed, 0));
        assert_eq!(outer_body.role(), &NodeRole::Structural);
        assert_eq!(tick.role(), &role(keyed, 0));
        assert_eq!(sent.role(), &role(keyed, 0));
        assert_eq!(received.role(), &role(keyed, 1));
        assert_eq!(inner_done.role(), &role(keyed, 0));
        assert!(outer.repeats().is_empty());

        let maximum = ExactInteger::new(i64::MAX);
        let zero = ExactInteger::new(0);
        assert_eq!(
            impossible.repeats(),
            &[
                RepeatOrdinalSchema::Iteration {
                    repeat: outer.node().clone(),
                    upper_exclusive: maximum,
                },
                RepeatOrdinalSchema::Iteration {
                    repeat: node(package, schema, "Inner").node().clone(),
                    upper_exclusive: zero,
                },
            ]
        );
        assert_eq!(
            inner_done.repeats(),
            &[
                RepeatOrdinalSchema::Iteration {
                    repeat: outer.node().clone(),
                    upper_exclusive: maximum,
                },
                RepeatOrdinalSchema::Exhaustion {
                    repeat: node(package, schema, "Inner").node().clone(),
                    value: zero,
                },
            ]
        );
        assert_eq!(
            outer_done.repeats(),
            &[RepeatOrdinalSchema::Exhaustion {
                repeat: outer.node().clone(),
                value: maximum,
            }]
        );
        for selected in schema.nodes() {
            let control = &controls[index(selected.node().index)];
            assert_eq!(selected.original_node(), control.original_node);
            assert_eq!(selected.locus(), &control.locus);
        }

        assert!(!emitted.windows(2).any(|bytes| bytes == b"O1"));
        assert!(!emitted.windows(2).any(|bytes| bytes == b"O2"));
        let ordinals = [ExactInteger::new(i64::MAX - 1), zero];
        let o1 = schema
            .bind(
                WorkflowInstanceIdentity::new("O1").unwrap(),
                inner_done.node(),
                role(keyed, 0),
                &ordinals,
            )
            .expect("first downstream workflow key");
        let o2 = schema
            .bind(
                WorkflowInstanceIdentity::new("O2").unwrap(),
                inner_done.node(),
                role(keyed, 0),
                &ordinals,
            )
            .expect("second downstream workflow key");
        assert_ne!(o1, o2);
        assert_eq!(o1.workflow().as_str(), "O1");
        assert_eq!(o2.workflow().as_str(), "O2");
        assert_eq!(o1.node(), inner_done.node());
        assert_eq!(o1.repeat_ordinals(), ordinals);

        assert!(matches!(
            schema.bind(
                WorkflowInstanceIdentity::new("O1").unwrap(),
                inner_done.node(),
                role(keyed, 1),
                &ordinals,
            ),
            Err(OccurrenceKeyError::WrongRole { .. })
        ));
        assert_eq!(
            schema.bind(
                WorkflowInstanceIdentity::new("O1").unwrap(),
                inner_done.node(),
                role(keyed, 0),
                &[zero],
            ),
            Err(OccurrenceKeyError::RepeatCount {
                expected: 2,
                actual: 1,
            })
        );
        assert_eq!(
            schema.bind(
                WorkflowInstanceIdentity::new("O1").unwrap(),
                inner_done.node(),
                role(keyed, 0),
                &[maximum, zero],
            ),
            Err(OccurrenceKeyError::RepeatOrdinal {
                dimension: 0,
                value: maximum,
            })
        );
        assert_eq!(
            schema.bind(
                WorkflowInstanceIdentity::new("O1").unwrap(),
                impossible.node(),
                role(keyed, 0),
                &[ExactInteger::new(i64::MAX - 1), zero],
            ),
            Err(OccurrenceKeyError::RepeatOrdinal {
                dimension: 1,
                value: zero,
            })
        );
        assert!(schema
            .bind(
                WorkflowInstanceIdentity::new("O1").unwrap(),
                outer_done.node(),
                role(keyed, 0),
                &[maximum],
            )
            .is_ok());
        assert_eq!(
            schema.bind(
                WorkflowInstanceIdentity::new("O1").unwrap(),
                &w::Handle {
                    declaration: keyed,
                    index: u32::MAX,
                },
                role(keyed, 0),
                &[],
            ),
            Err(OccurrenceKeyError::UnknownNode {
                node: w::Handle {
                    declaration: keyed,
                    index: u32::MAX,
                },
            })
        );

        assert_eq!(
            WorkflowInstanceIdentity::new(""),
            Err(OccurrenceKeyError::EmptyWorkflowIdentity)
        );
        let largest_identity = "x".repeat(4_096);
        assert_eq!(
            WorkflowInstanceIdentity::new(&largest_identity)
                .expect("exact identity bound")
                .as_str(),
            largest_identity
        );
        let oversized_identity = "x".repeat(4_097);
        assert_eq!(
            WorkflowInstanceIdentity::new(&oversized_identity),
            Err(OccurrenceKeyError::WorkflowIdentityTooLong { bytes: 4_097 })
        );
    });
}

/// Tracing: TC-132; ACs: FR-048-AC-1.
#[test]
#[trace("TC-132", "FR-048-AC-1")]
fn tc_132_schema_refuses_wrong_family_and_honors_exact_projection_limits() {
    with_admitted(|admitted, _| {
        let package = admitted.package();
        let other = declaration(package, "Other");
        let keyed = declaration(package, "Keyed");
        assert_eq!(
            occurrence_key_schema(admitted, other, Limits::default()).into_result(),
            Err(Error::Invalid(Invalid::Owner))
        );
        assert_eq!(
            occurrence_key_schema(admitted, u32::MAX, Limits::default()).into_result(),
            Err(Error::Invalid(Invalid::Reference))
        );

        let measured = occurrence_key_schema(admitted, keyed, Limits::default());
        measured.result().expect("measure complete schema");
        let usage = measured.usage();
        assert!(usage.entries > 0);
        assert!(usage.references > 0);
        assert!(usage.depth > 0);
        let exact = Limits {
            entries: usage.entries,
            references: usage.references,
            depth: usage.depth,
            ..Limits::default()
        };
        let exact_report = occurrence_key_schema(admitted, keyed, exact);
        assert_eq!(exact_report.usage(), usage);
        assert_eq!(
            exact_report
                .result()
                .expect("exact limits admit the complete schema")
                .nodes()
                .len(),
            occurrence_key_schema(admitted, keyed, Limits::default())
                .result()
                .unwrap()
                .nodes()
                .len()
        );

        for (dimension, limits) in [
            (
                Dimension::Entries,
                Limits {
                    entries: usage.entries - 1,
                    ..Limits::default()
                },
            ),
            (
                Dimension::References,
                Limits {
                    references: usage.references - 1,
                    ..Limits::default()
                },
            ),
            (
                Dimension::Depth,
                Limits {
                    depth: usage.depth - 1,
                    ..Limits::default()
                },
            ),
        ] {
            let report = occurrence_key_schema(admitted, keyed, limits);
            let Err(Error::Incomplete(exhaustion)) = report.result() else {
                panic!("one-short {dimension:?} limit must be incomplete")
            };
            assert_eq!(exhaustion.dimension, dimension);
            assert_eq!(
                exhaustion.limit,
                match dimension {
                    Dimension::Entries => usage.entries - 1,
                    Dimension::References => usage.references - 1,
                    Dimension::Depth => usage.depth - 1,
                    _ => unreachable!("selected projection dimension"),
                }
            );
            assert!(exhaustion.requested > 0);
            assert!(report.result().is_err());
        }
    });
}
