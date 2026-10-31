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
    use quire_spec_language::protocol_artifact::{
        handoff::{
            MUTATION_MANIFEST_FORMAT, PUBLISHED_MUTATION_MANIFEST_FILE, PUBLISHED_OFFER_FILE,
            PUBLISHED_SELECTION_FILE,
        },
        v2,
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

        let sidecar: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output.join(PUBLISHED_SELECTION_FILE)).unwrap())
                .unwrap();
        assert_eq!(sidecar["temporal"].as_array().unwrap().len(), 3);
        for temporal in sidecar["temporal"].as_array().unwrap() {
            let file = temporal["clock_input"]["file"].as_str().unwrap();
            let clock = std::fs::read(output.join(file)).unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&clock).unwrap(),
                temporal["clock_input"]["configuration"]
            );
        }

        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(output.join(PUBLISHED_MUTATION_MANIFEST_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["format"], MUTATION_MANIFEST_FORMAT);
        let cases = manifest["cases"].as_array().unwrap();
        for case in cases {
            let input = &case["input"];
            let file = input["file"].as_str().unwrap();
            assert!(output.join(file).is_file(), "missing mutation {file}");
            assert!(!case["expected_refusal_code"].as_str().unwrap().is_empty());
        }
    }
}
