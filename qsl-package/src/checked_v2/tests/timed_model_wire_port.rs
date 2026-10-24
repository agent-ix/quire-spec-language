// SPDX-License-Identifier: AGPL-3.0-or-later
//! Optional raw-output receipts for the existing timed-model conformance tests.
//! The tests always retain their typed roundtrip and strict admission assertions.
//! Qualification can compare an immutable old-writer baseline without vendoring
//! QSpec's input corpus or adding a production codec.

use std::io::Write;
use std::path::Path;

use crate::checked_v2::timed_model::CheckedTimedModel;

pub(super) fn observe(label: &str, models: &[CheckedTimedModel], package_bytes: &[u8]) {
    let baseline = std::env::var_os("QSL_TIMED_WIRE_BASELINE");
    let capture = std::env::var_os("QSL_TIMED_WIRE_CAPTURE");
    if baseline.is_none() && capture.is_none() {
        return;
    }
    let nodes = models
        .iter()
        .map(|model| {
            model
                .write_nodes()
                .expect("the admitted fixture writes nodes")
        })
        .collect::<Vec<_>>();
    let value = serde_json::to_value(nodes).expect("the owner node wire serializes");
    let node_bytes = quire_canonical::to_vec(&value, crate::checked::EMIT_LIMITS)
        .expect("the fixture node wire canonically encodes");
    for (kind, bytes) in [("nodes", node_bytes.as_slice()), ("package", package_bytes)] {
        let name = format!("{label}-{kind}.json");
        if let Some(directory) = &baseline {
            let expected = std::fs::read(Path::new(directory).join(&name))
                .expect("the immutable baseline receipt exists");
            let first_difference = bytes
                .iter()
                .zip(&expected)
                .position(|(actual, expected)| actual != expected)
                .unwrap_or(bytes.len().min(expected.len()));
            assert!(
                bytes == expected,
                "{name} changed at byte {first_difference}: actual {} bytes, baseline {} bytes",
                bytes.len(),
                expected.len()
            );
        }
        if let Some(directory) = &capture {
            std::fs::create_dir_all(directory).expect("the receipt directory can be created");
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(Path::new(directory).join(&name))
                .expect("capture never overwrites an existing receipt");
            file.write_all(bytes)
                .expect("the complete receipt is written");
        }
    }
}
