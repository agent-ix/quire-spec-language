// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042/TC-121: real native producer fixture; B's IT-001 remains a separate gate.

#[allow(dead_code, reason = "The shared module also provides the v2 recipe")]
#[path = "protocol-handoff/producer.rs"]
mod producer;

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), producer::Error> {
    let mut arguments = std::env::args_os().skip(1);
    let directory = arguments.next().ok_or(producer::Error::Arguments)?;
    if arguments.next().is_some() {
        return Err(producer::Error::Arguments);
    }
    producer::write(std::path::Path::new(&directory))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_spec_language::protocol_artifact::wire as w;
    use quire_spec_language::syntax::{composed as c, ExprKind};

    #[test]
    #[trace(
        "TC-121",
        "FR-042-AC-1",
        "FR-042-AC-2",
        "FR-042-AC-4",
        "FR-042-AC-5",
        "FR-042-AC-6",
        "FR-042-AC-7"
    )]
    fn stripped_release_producer_keeps_original_owners_and_compensations() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("handoff");
        // The recipe selects this actual test executable and independently reads
        // the emitted bytes before writing. These assertions inspect its output.
        super::producer::write(&output).expect("real producer and independent reader");
        let package: w::Package =
            serde_json::from_slice(&std::fs::read(output.join("compiled-protocol.json")).unwrap())
                .unwrap();
        const SOURCE_PREFIX: &str = "ix://agent-ix/quire-spec-language/examples/protocol-handoff/";
        assert_eq!(package.sources.len(), 4);
        for (unit, document) in [
            ("predicates", "ProtocolHandoffPredicates"),
            ("state", "ProtocolHandoffState"),
            ("temporal", "ProtocolHandoffTemporal"),
            ("workflow", "ProtocolHandoffWorkflow"),
        ] {
            let identity = format!("{SOURCE_PREFIX}{unit}");
            let source = package
                .sources
                .iter()
                .find(|s| s.native.identity == identity)
                .unwrap();
            assert_eq!(source.native.revision, "1");
            assert_eq!(
                source.path,
                format!("examples/protocol-handoff/{unit}.native")
            );
            assert_eq!(source.formal.document, document);
            assert_eq!(
                source.formal.revision.namespace,
                "quire-contract-ir/source-revision"
            );
            assert_eq!(source.formal.revision.value, "1");
        }
        assert_eq!(package.declarations.len(), 7);
        for (name, unit, requirement, clause) in [
            ("Allowed", "predicates", "HandoffPredicates", "allowed"),
            ("Bounded", "predicates", "HandoffPredicates", "bounded"),
            ("Healthy", "state", "HandoffState", "healthy"),
            ("BeforeApply", "state", "HandoffState", "before_apply"),
            ("AfterApply", "state", "HandoffState", "after_apply"),
            ("Due", "temporal", "HandoffTemporal", "due"),
            ("Flow", "workflow", "HandoffWorkflow", "flow"),
        ] {
            let declaration = package
                .declarations
                .iter()
                .find(|d| d.name == name)
                .unwrap();
            let source = &package.sources[declaration.locus.source as usize];
            assert_eq!(source.native.identity, format!("{SOURCE_PREFIX}{unit}"));
            assert_eq!(
                declaration.requirement.package,
                "agent-ix/quire-spec-language"
            );
            assert_eq!(declaration.requirement.identity, requirement);
            assert_eq!(
                declaration.requirement.revision.namespace,
                "quire-contract-ir/requirement-revision"
            );
            assert_eq!(declaration.requirement.revision.value, "1");
            assert_eq!(declaration.clause, clause);
        }
        let owner = package
            .declarations
            .iter()
            .position(|d| d.name == "Flow")
            .unwrap();
        let w::Body::Protocol {
            compensations,
            controls,
            ..
        } = &package.declarations[owner].body
        else {
            panic!("Flow retains its protocol body")
        };
        let [full, partial] = compensations.as_slice() else {
            panic!("two authored compensation obligations")
        };
        assert_eq!(
            (full.name.as_str(), partial.name.as_str()),
            ("Full", "Partial")
        );
        assert_eq!(full.forward_effect, partial.forward_effect);
        assert_eq!(full.forward_effect.declaration as usize, owner);
        assert_eq!(controls[full.forward_effect.index as usize].name, "Applied");
        let commit = full
            .commit
            .0
            .as_ref()
            .expect("Full names its commit boundary");
        assert_eq!(commit.declaration as usize, owner);
        assert_eq!(controls[commit.index as usize].name, "Committed");
        assert!(matches!(
            controls[commit.index as usize].operation,
            w::ControlOperation::Commit { .. }
        ));
        assert!(
            partial.commit.0.is_none(),
            "Partial retains authored commit never"
        );
        signed64_literals(&package);
        received_choice(&package, owner, &output);
    }

    /// FR-042-AC-2: the fixture admits the full signed-64 integer domain and an
    /// exact rational at the denominator ceiling, and carries their literals to
    /// the wire unnarrowed. Every handle below stays inside Bounded's own arena.
    fn signed64_literals(package: &w::Package) {
        use quire_spec_language::protocol_artifact::{ExactInteger, ExactRational, ProtocolNumber};

        let declaration = package
            .declarations
            .iter()
            .find(|d| d.name == "Bounded")
            .expect("authored signed-64 predicate");
        let literals = declaration
            .values
            .iter()
            .filter_map(|value| match &value.operation {
                w::ValueOperation::Number { value } => {
                    Some(value.checked().expect("canonical emitted number"))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            literals,
            [
                ProtocolNumber::Integer(ExactInteger::new(i64::MIN)),
                ProtocolNumber::Integer(ExactInteger::new(i64::MAX)),
                ProtocolNumber::Rational(ExactRational::new(i64::MAX, i64::MAX - 1).unwrap()),
                ProtocolNumber::Rational(ExactRational::new(i64::MIN, i64::MAX).unwrap()),
            ]
        );
        for (name, expected) in [
            ("Wide", vec![i64::MIN, i64::MAX]),
            ("Exact", vec![i64::MIN, i64::MAX, i64::MAX]),
        ] {
            let export = package.models[0]
                .exports
                .iter()
                .position(|export| {
                    export.kind == w::ExportKind::Scalar && export.path == [name.to_owned()]
                })
                .unwrap_or_else(|| panic!("{name} scalar export")) as u32;
            let representation = package
                .types
                .iter()
                .find_map(|ty| match ty {
                    w::Type::Scalar {
                        export: selected,
                        representation,
                        ..
                    } if selected.model == 0 && selected.export == export => Some(representation),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{name} scalar type"));
            let bounds = match representation {
                w::Representation::Integer { minimum, maximum } => vec![minimum, maximum],
                w::Representation::Rational {
                    numerator_minimum,
                    numerator_maximum,
                    maximum_denominator,
                } => vec![numerator_minimum, numerator_maximum, maximum_denominator],
                w::Representation::Text { .. } => panic!("{name} is a numeric scalar"),
            };
            let bounds = bounds
                .into_iter()
                .map(|bound| match bound.checked().expect("canonical bound") {
                    ProtocolNumber::Integer(value) => value.value(),
                    ProtocolNumber::Rational(_) => panic!("{name} bound is an integer position"),
                })
                .collect::<Vec<_>>();
            assert_eq!(bounds, expected, "{name} retains its full domain");
        }
    }

    fn received_choice(package: &w::Package, owner: usize, output: &std::path::Path) {
        let declaration = &package.declarations[owner];
        let source = &package.sources[declaration.locus.source as usize];
        let original_bytes = std::fs::read(output.join("workflow.native")).unwrap();
        assert_eq!(source.text.as_bytes(), original_bytes);
        assert!(source
            .text
            .ends_with(include_str!("protocol-handoff/workflow.body.native")));
        let c::NativeUnit::Composed(original) = quire_spec_language::parse_native(
            quire_spec_language::SourceIdentity {
                identity: source.native.identity.clone(),
                revision: "1".into(),
            },
            "examples/protocol-handoff/workflow.native",
            &original_bytes,
            quire_spec_language::Limits::default(),
        )
        .unwrap() else {
            panic!("original composed source")
        };
        let w::Body::Protocol {
            roles,
            channels,
            controls,
            causal_edges,
            ..
        } = &declaration.body
        else {
            panic!("protocol")
        };
        let handle = |name| w::Handle {
            declaration: owner as u32,
            index: controls.iter().position(|node| node.name == name).unwrap() as u32,
        };
        let [channel] = channels.as_slice() else {
            panic!("one decision channel")
        };
        assert_eq!(channel.name, "Decisions");
        assert_eq!(channel.locus.source, declaration.locus.source);
        assert_eq!(channel.from.declaration as usize, owner);
        assert_eq!(channel.to.declaration as usize, owner);
        assert_eq!(roles[channel.from.index as usize].name, "Provider");
        assert_eq!(roles[channel.to.index as usize].name, "Service");
        assert_ne!(channel.from, channel.to);
        let outcome = handle("Outcome");
        let w::ControlOperation::Choice {
            owner: chooser,
            visible,
            cases,
        } = &controls[outcome.index as usize].operation
        else {
            panic!("received decision")
        };
        assert_eq!(chooser, &channel.to);
        assert_eq!(visible.len(), 2);
        assert_eq!(
            cases
                .iter()
                .map(|case| case.label.as_str())
                .collect::<Vec<_>>(),
            ["ready", "notReady"]
        );
        assert_eq!(cases[0].body, handle("Confirmed"));
        assert_eq!(cases[1].body, handle("Unconfirmed"));

        let mut received = Vec::new();
        for (index, (reply, sent, binder_name)) in [
            ("LeftReply", "LeftSent", "leftReply"),
            ("RightReply", "RightSent", "rightReply"),
        ]
        .into_iter()
        .enumerate()
        {
            let reply = handle(reply);
            let w::ControlOperation::Event {
                event:
                    w::Event::Receive {
                        channel: selected,
                        send,
                    },
                binder,
                ..
            } = &controls[reply.index as usize].operation
            else {
                panic!("receive occurrence")
            };
            assert_eq!(
                *selected,
                w::Handle {
                    declaration: owner as u32,
                    index: 0
                }
            );
            assert_eq!(*send, handle(sent));
            assert_eq!(binder.declaration as usize, owner);
            let bound = &declaration.binders[binder.index as usize];
            assert_eq!(bound.name, binder_name);
            assert_eq!(bound.kind, w::BinderKind::Event);
            assert_eq!(bound.anchor.declaration as usize, owner);
            let anchor = &declaration.anchors[bound.anchor.index as usize];
            assert_eq!(anchor.kind, w::AnchorKind::Control);
            assert_eq!(anchor.owner.0.as_ref(), Some(&reply));
            assert_eq!(bound.locus.source, declaration.locus.source);
            assert_eq!(
                &source.text[bound.locus.span.start as usize..bound.locus.span.end as usize],
                binder_name
            );
            let visible = &visible[index];
            assert_eq!(visible.declaration as usize, owner);
            let value = &declaration.values[visible.index as usize];
            let w::ValueOperation::Field { base, field } = &value.operation else {
                panic!("original received Boolean field")
            };
            assert_eq!(base.declaration as usize, owner);
            assert_eq!(
                declaration.values[base.index as usize].operation,
                w::ValueOperation::Read {
                    binder: binder.clone()
                }
            );
            assert_eq!(
                package.models[field.model as usize].exports[field.export as usize].path,
                ["Notice", "ready"]
            );
            assert_eq!(value.locus.source, declaration.locus.source);
            assert_eq!(
                &source.text[value.locus.span.start as usize..value.locus.span.end as usize],
                format!("{binder_name}.ready")
            );
            let parsed = &original.expressions()[value.original_expression as usize];
            assert!(matches!(
                parsed.kind,
                c::ValueKind::Shared(ExprKind::Field { .. })
            ));
            assert_eq!(
                (
                    value.locus.span.start as usize,
                    value.locus.span.end as usize
                ),
                (parsed.span.start, parsed.span.end)
            );
            received.push(bound);
        }
        assert_eq!(received[0].value_type, received[1].value_type);
        assert_ne!(received[0].anchor, received[1].anchor);
        let guards = [
            "leftReply.ready and rightReply.ready",
            "not (leftReply.ready and rightReply.ready)",
        ];
        for (case, text) in cases.iter().zip(guards) {
            assert_eq!(case.guard.declaration as usize, owner);
            let guard = &declaration.values[case.guard.index as usize];
            assert_eq!(guard.locus.source, declaration.locus.source);
            assert_eq!(
                &source.text[guard.locus.span.start as usize..guard.locus.span.end as usize],
                text
            );
            for (kind, from, to) in [
                (w::EdgeKind::Branch, &outcome, &case.body),
                (w::EdgeKind::Join, &case.body, &outcome),
            ] {
                assert!(causal_edges.iter().any(|edge| edge.owner == outcome
                    && edge.kind == kind
                    && edge.from.node == *from
                    && edge.to.node == *to));
            }
        }
        let gather = handle("Gather");
        let w::ControlOperation::Parallel { branches, join } =
            &controls[gather.index as usize].operation
        else {
            panic!("all-branch join")
        };
        assert_eq!(
            branches
                .iter()
                .map(|branch| branch.label.as_str())
                .collect::<Vec<_>>(),
            ["left", "right"]
        );
        assert_eq!(join, &[0, 1]);
        for (branch, name) in branches.iter().zip(["Left", "Right"]) {
            assert_eq!(branch.body, handle(name));
            for (kind, from, to) in [
                (w::EdgeKind::Branch, &gather, &branch.body),
                (w::EdgeKind::Join, &branch.body, &gather),
            ] {
                assert!(causal_edges.iter().any(|edge| edge.owner == gather
                    && edge.kind == kind
                    && edge.from.node == *from
                    && edge.to.node == *to));
            }
        }
        for name in [
            "Gather",
            "Left",
            "Right",
            "LeftSent",
            "RightSent",
            "LeftReply",
            "RightReply",
            "Outcome",
            "Confirmed",
            "Unconfirmed",
        ] {
            let retained = &controls[handle(name).index as usize];
            let authored = &original.controls()[retained.original_node as usize];
            assert_eq!(authored.name.value, name);
            assert_eq!(retained.locus.source, declaration.locus.source);
            assert_eq!(
                (
                    retained.locus.span.start as usize,
                    retained.locus.span.end as usize
                ),
                (authored.span.start, authored.span.end)
            );
        }
    }
}
