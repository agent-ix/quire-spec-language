// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042/TC-121: real native producer fixture; B's IT-001 remains a separate gate.

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

    #[test]
    #[ignore = "requires a stripped release ELF test executable within the producer's 16 MiB binary limit"]
    #[trace("TC-121", "FR-042-AC-1", "FR-042-AC-4", "FR-042-AC-6", "FR-042-AC-7")]
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
        assert_eq!(package.declarations.len(), 6);
        for (name, unit, requirement, clause) in [
            ("Allowed", "predicates", "HandoffPredicates", "allowed"),
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
    }
}
