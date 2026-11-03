// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-511: the exact protocol number codec is a root-level public module.

use ix_trace_rs::trace;
use quire_spec_language::number::{
    ExactInteger, ExactRational, NumberComponent, NumberError, NumberWire, ProtocolNumber,
    NUMERIC_PROFILE,
};

/// FR-511-AC-1 / TC-1306: the root import surface resolves.
#[trace("FR-511-AC-1", "TC-1306")]
#[test]
fn root_number_module_publishes_the_complete_codec_surface() {
    assert_eq!(NUMERIC_PROFILE, "quire.protocol.numeric/1");
    assert_eq!(
        ProtocolNumber::Integer(ExactInteger::new(7)),
        ProtocolNumber::Integer(ExactInteger::new(7))
    );
    assert_eq!(ExactRational::new(1, 3).unwrap().denominator(), 3);
    assert_eq!(
        ExactInteger::from_decimal("01"),
        Err(NumberError::NonCanonicalDecimal {
            component: NumberComponent::Decimal,
        })
    );
    let _: Option<NumberWire> = None;
}

/// FR-511-AC-3 / TC-1306: no `qsl-*` crate's source refers to the codec or to
/// its profile string.
#[trace("FR-511-AC-3", "TC-1306")]
#[test]
fn no_qsl_crate_source_refers_to_the_codec() {
    fn walk(dir: &std::path::Path, found: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                if text.contains("ProtocolNumber") || text.contains("quire.protocol.numeric/1") {
                    found.push(path.display().to_string());
                }
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        let is_qsl = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("qsl-"));
        if is_qsl && path.join("src").is_dir() {
            walk(&path.join("src"), &mut found);
        }
    }
    assert!(
        found.is_empty(),
        "qsl-* sources refer to the codec: {found:?}"
    );
}
