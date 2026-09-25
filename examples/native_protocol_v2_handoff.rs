// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-050/TC-138: thin CLI over `protocol_artifact::handoff::write_v2`, the strict v2 producer handoff for B.

use quire_spec_language::protocol_artifact::handoff;

fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let (Some(directory), None) = (arguments.next(), arguments.next()) else {
        eprintln!("usage: native_protocol_v2_handoff <new-output-directory>");
        return std::process::ExitCode::FAILURE;
    };
    match handoff::write_v2(std::path::Path::new(&directory)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::ByteDigest;
    use quire_spec_language::protocol_artifact::{
        handoff::{
            MUTATION_MANIFEST_FORMAT, PUBLISHED_ARTIFACT_REFERENCE_FILE,
            PUBLISHED_MUTATION_MANIFEST_FILE, PUBLISHED_OFFER_FILE, PUBLISHED_SELECTION_FILE,
        },
        v2, wire as w,
    };

    #[trace("TC-138", "FR-050-AC-1", "FR-050-AC-4")]
    #[test]
    fn stripped_release_v2_producer_writes_the_independently_read_handoff() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("handoff-v2");
        quire_spec_language::protocol_artifact::handoff::write_v2(&output)
            .expect("real v2 producer and independent strict reader");

        let bytes = std::fs::read(output.join(PUBLISHED_OFFER_FILE)).unwrap();
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
            &std::fs::read(output.join(PUBLISHED_ARTIFACT_REFERENCE_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(reference.wire.identity, "quire.compiled-protocol");
        assert_eq!(reference.wire.version, "2");
        assert_eq!(reference.digest, ByteDigest::of(&bytes));

        let sidecar: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output.join(PUBLISHED_SELECTION_FILE)).unwrap())
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

        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(output.join(PUBLISHED_MUTATION_MANIFEST_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["format"], MUTATION_MANIFEST_FORMAT);
        let cases = manifest["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 28);
        for case in cases {
            let input = &case["input"];
            let file = input["file"].as_str().unwrap();
            assert!(output.join(file).is_file(), "missing mutation {file}");
            assert!(!case["expected_refusal_code"].as_str().unwrap().is_empty());
        }
    }
}
