// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-017: independent original-occurrence controls for real Serde borrowing.

use crate::support::located_json;

use ix_trace_rs::trace;
use qsl_foundation::{Source, SourceIdentity, Span};
use quire_contract_ir as ir;
use quire_spec_language::formal_source::FormalSource;
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Named {
    name: String,
    id: u32,
}

fn source(text: &str) -> FormalSource {
    let native = Source::read(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:located-json".into(),
            revision_namespace: "test".into(),
            revision: "draft:1".into(),
        },
        "occurrences.json",
        text.as_bytes(),
        1_048_576,
    )
    .unwrap();
    FormalSource::new(
        native,
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("JsonOccurrences").unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    )
}

#[test]
#[trace("TC-054", "FR-017-AC-1")]
fn tc_054_locations_follow_occurrences_not_spelling() {
    let cases = [
        ("[", r#"{"name":"NodeRef","id":1}"#, "]", "NodeRef", 1),
        (
            "[\n{\"reference\":\"NodeRef\"},\n  ",
            r#"{"name":"NodeRef","id":1}"#,
            "\n]",
            "NodeRef",
            1,
        ),
        (
            "[{\"name\":\"NodeRef\",\"id\":1},",
            r#"{"name":"NodeRef","id":1}"#,
            "]",
            "NodeRef",
            1,
        ),
        (
            "[\n {\"reference\":\"NodeRef\"},\n ",
            r#"{ "id": 2, "name": "Node\u0052ef" }"#,
            "\n]",
            "NodeRef",
            2,
        ),
        (
            "[\n{\"reference\":\"é😀\"},\n ",
            "{\"id\":3,\"name\":\"é😀\"}",
            "\n]",
            "é😀",
            3,
        ),
    ];
    for (prefix, value, suffix, name, id) in cases {
        let text = format!("{prefix}{value}{suffix}");
        let binding = source(&text);
        let raw: Vec<&RawValue> = located_json::read(&binding, 1_048_576).unwrap();
        let located: located_json::Located<Named> =
            located_json::decode(&binding, raw.last().unwrap()).unwrap();
        assert_eq!(
            located.value,
            Named {
                name: name.into(),
                id
            }
        );
        let expected = Span {
            start: prefix.len(),
            end: prefix.len() + value.len(),
        };
        assert_eq!(binding.to_native(&located.source).unwrap(), expected);
        assert_eq!(
            binding.source().text().get(expected.start..expected.end),
            Some(value)
        );
    }
}

#[test]
#[trace("TC-054", "FR-017-AC-1", "TC-102", "FR-025-AC-3")]
fn tc_054_identical_foreign_buffers_are_not_source_occurrences() {
    let text = r#"{"name":"NodeRef","id":1}"#;
    let original = source(text);
    let foreign = source(text);
    let raw: &RawValue = located_json::read(&foreign, text.len()).unwrap();
    assert!(matches!(
        located_json::decode::<Named>(&original, raw),
        Err(located_json::Error::ForeignOccurrence)
    ));
    let owned = RawValue::from_string(text.to_owned()).unwrap();
    assert!(matches!(
        located_json::decode::<Named>(&original, &owned),
        Err(located_json::Error::ForeignOccurrence)
    ));
    let own: &RawValue = located_json::read(&original, text.len()).unwrap();
    assert_eq!(
        located_json::decode::<Named>(&original, own)
            .unwrap()
            .value
            .id,
        1
    );
}

#[test]
#[trace("TC-054", "FR-017-AC-1", "TC-102", "FR-025-AC-4")]
fn tc_054_malformed_and_ambiguous_typed_input_refuse() {
    for text in ["{", "[] true", "[1,]"] {
        assert!(matches!(
            located_json::read::<&RawValue>(&source(text), text.len()),
            Err(located_json::Error::Json(_))
        ));
        assert!(matches!(
            located_json::read::<&RawValue>(&source(text), text.len() - 1),
            Err(located_json::Error::ByteLimit { actual, maximum })
                if actual == text.len() && maximum == text.len() - 1
        ));
    }
    for text in [
        r#"{"name":"NodeRef","name":"Other","id":1}"#,
        r#"{"name":"NodeRef","\u006eame":"Other","id":1}"#,
        r#"{"name":"NodeRef","id":1,"unknown":true}"#,
        r#"{"name":"NodeRef"}"#,
    ] {
        let binding = source(text);
        let raw: &RawValue = located_json::read(&binding, text.len()).unwrap();
        assert!(matches!(
            located_json::decode::<Named>(&binding, raw),
            Err(located_json::Error::Json(_))
        ));
    }
}
