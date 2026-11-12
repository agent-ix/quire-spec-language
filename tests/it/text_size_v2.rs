// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1409 step 1: the checked-package bytes of FR-093-AC-63's `n` read back
//! through IR's strict v2 reader, and carry one `quire.op.text.size` call.

use std::path::Path;

use ix_trace_rs::trace;
use serde_json::Value;

use crate::emit_checked_v2::{producer_bytes, read_back};

/// Every `quire.op.text.size` application in `wire`, anywhere in it.
fn size_nodes<'w>(wire: &'w Value, found: &mut Vec<&'w Value>) {
    match wire {
        Value::Object(object) => {
            if object
                .get("operation")
                .is_some_and(|operation| operation["identity"] == "quire.op.text.size")
                && object.get("term").is_some_and(|term| term == "application")
            {
                found.push(wire);
            }
            object.values().for_each(|value| size_nodes(value, found));
        }
        Value::Array(items) => items.iter().for_each(|value| size_nodes(value, found)),
        _ => {}
    }
}

/// FR-093-AC-63 (TC-1409 step 1): one `call` node over `[ref(t)]` with the
/// `text_profile` law and mode `nfc`, no member and no leaves, and the
/// `Int[0, 8]` result type.
#[trace("FR-093-AC-63", "TC-1409")]
#[test]
fn text_size_lowers_to_one_text_size_call_with_the_profile_law_and_mode() {
    let source = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function n using v(t: Text[0, 8; nfc]): Int[0, 8] pure { size(t) }\n";
    let bytes = producer_bytes(Path::new("text-size.native"), source, &[]);
    read_back(&bytes, &[]);
    let wire: Value = serde_json::from_slice(&bytes).expect("JSON");
    let mut found = Vec::new();
    size_nodes(&wire, &mut found);
    assert_eq!(found.len(), 1, "{found:#?}");
    let node = found[0];
    assert_eq!(node["operator"], "call");
    let operation = &node["operation"];
    assert_eq!(operation["member"], Value::Null);
    assert_eq!(operation["leaves"], serde_json::json!([]));
    assert_eq!(
        operation["laws"],
        serde_json::json!([{
            "role": "text_profile",
            "definition": {
                "authority": "agent-ix",
                "identity": "quire.value.text.unicode-17.0.0/v1"
            }
        }])
    );
    assert_eq!(
        operation["mode"],
        serde_json::json!({"kind": "text_profile", "value": "nfc"})
    );
    let arguments = node["arguments"].as_array().expect("arguments");
    assert_eq!(arguments.len(), 1);
    assert_eq!(arguments[0]["term"], "reference");
    // The result type is the `Int[0, 8]` bounded-domain node.
    let result = node["result_type"]["path"]
        .as_array()
        .expect("a keyed path");
    assert_eq!(result[..2], ["bounded_domain", "integer_range"]);
    assert_eq!(result[result.len() - 3..], ["0", "max", "8"]);
}
