// SPDX-License-Identifier: AGPL-3.0-only
//! FR-050/TC-138: release-capable strict v2 producer handoff for B.

#[allow(dead_code, reason = "The shared module also preserves the v1 recipe")]
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
    let directory = arguments.next().ok_or(producer::Error::ArgumentsV2)?;
    if arguments.next().is_some() {
        return Err(producer::Error::ArgumentsV2);
    }
    producer::write_v2(std::path::Path::new(&directory))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_spec_language::protocol_artifact::{v2, wire as w};
    use quire_spec_language::ByteDigest;

    #[trace("TC-138", "FR-050-AC-1", "FR-050-AC-4")]
    #[test]
    #[ignore = "requires a stripped release ELF test executable within the producer's 16 MiB binary limit"]
    fn stripped_release_v2_producer_writes_the_independently_read_handoff() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("handoff-v2");
        super::producer::write_v2(&output).expect("real v2 producer and independent strict reader");

        let bytes = std::fs::read(output.join("compiled-protocol-v2.json")).unwrap();
        let package: v2::wire::Package = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(package.inherited.wire, v2::WIRE);
        assert_eq!(package.temporal_bindings.len(), 3);
        assert!(package
            .temporal_bindings
            .windows(2)
            .all(|pair| pair[0].declaration < pair[1].declaration));
        for (binding, name) in
            package
                .temporal_bindings
                .iter()
                .zip(["Due", "DueSample", "DueTimestamp"])
        {
            let declaration = &package.inherited.declarations[binding.declaration as usize];
            assert_eq!(declaration.name, name);
            assert_eq!(binding.definition, declaration.profile);
        }
        assert_eq!(
            package.temporal_bindings[0].clock,
            v2::wire::ClockConfiguration::EventPosition {
                sequence_authority: "workflow-events".into()
            }
        );

        let reference: w::ArtifactRef = serde_json::from_slice(
            &std::fs::read(output.join("compiled-protocol-v2.ref.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(reference.wire.identity, "quire.compiled-protocol");
        assert_eq!(reference.wire.version, "2");
        assert_eq!(reference.digest, ByteDigest::of(&bytes));

        let sidecar: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output.join("expected-v2.json")).unwrap())
                .unwrap();
        assert_eq!(sidecar["temporal"].as_array().unwrap().len(), 3);
        for temporal in sidecar["temporal"].as_array().unwrap() {
            let file = temporal["clock_input"]["file"].as_str().unwrap();
            let clock = std::fs::read(output.join(file)).unwrap();
            assert_eq!(
                ByteDigest::of(&clock).to_string(),
                temporal["clock_input"]["digest"].as_str().unwrap()
            );
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&clock).unwrap(),
                temporal["clock_input"]["configuration"]
            );
        }
        let dependencies = sidecar["inherited"]["dependencies"].as_array().unwrap();
        assert!(!dependencies.is_empty());
        for dependency in dependencies {
            let file = dependency["file"].as_str().unwrap();
            let original = std::fs::read(output.join(file)).unwrap();
            assert_eq!(
                ByteDigest::of(&original).to_string(),
                dependency["artifact"]["digest"].as_str().unwrap(),
                "{file}"
            );
        }
        for source in sidecar["inherited"]["sources"].as_array().unwrap() {
            let file = source["file"].as_str().unwrap();
            let original = std::fs::read(output.join(file)).unwrap();
            assert_eq!(
                ByteDigest::of(&original).to_string(),
                source["source"]["artifact"]["digest"].as_str().unwrap(),
                "{file}"
            );
        }
        let model_source = &sidecar["inherited"]["model"]["source"];
        let model_source_file = sidecar["inherited"]["model"]["source_file"]
            .as_str()
            .unwrap();
        assert_eq!(
            ByteDigest::of(&std::fs::read(output.join(model_source_file)).unwrap()).to_string(),
            model_source["artifact"]["digest"].as_str().unwrap()
        );

        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output.join("mutations/manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["format"], "quire.protocol.v2-mutations/1");
        let cases = manifest["cases"].as_array().unwrap();
        assert!(cases.len() >= 13);
        for case in cases {
            let input = &case["input"];
            let file = input["file"].as_str().unwrap();
            assert!(output.join(file).is_file(), "missing mutation {file}");
            assert!(!case["expected_refusal_code"].as_str().unwrap().is_empty());
        }
    }
}
