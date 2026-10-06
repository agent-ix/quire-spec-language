// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-357: where a scalar node sits in a recompiled package. It reads the
//! checked nodes' preimages as JSON values to find the operation a node
//! applies and the nodes a function's body reaches; it encodes nothing and
//! computes no digest.

use qsl_semantics::check::{NodeTag, SemanticGraph, SemanticNode};
use quire_exact::NodeKey;

use crate::scalar::ScalarOperator;

/// Whether `target` is `function`'s node or one its body reaches. The walk
/// follows every node reference of a node's preimage, and stops at another
/// function's node: a call to another function is not that function's body.
#[qsl_attrs::string_edge]
pub(super) fn in_body(graph: &SemanticGraph, function: NodeKey, target: NodeKey) -> bool {
    let keys: std::collections::BTreeMap<String, NodeKey> = graph
        .nodes()
        .map(SemanticNode::key)
        .map(|key| (key.to_string(), key))
        .collect();
    let mut seen = std::collections::BTreeSet::from([function]);
    let mut pending = vec![function];
    while let Some(key) = pending.pop() {
        if key == target {
            return true;
        }
        let Some(node) = graph.node(key) else {
            continue;
        };
        if key != function && node.node_tag() == NodeTag::Function {
            continue;
        }
        let Ok(preimage) = serde_json::from_slice::<serde_json::Value>(node.preimage()) else {
            continue;
        };
        let mut digests = Vec::new();
        node_digests(&preimage, &mut digests);
        for digest in digests {
            let referenced = keys.get(&digest).copied();
            if let Some(referenced) = referenced {
                if seen.insert(referenced) {
                    pending.push(referenced);
                }
            }
        }
    }
    false
}

/// Every `digest` string of a preimage's node ids (`{domain, digest}`).
#[qsl_attrs::string_edge]
fn node_digests(value: &serde_json::Value, out: &mut Vec<String>) {
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(serde_json::Value::String(digest)) = map.get("digest") {
                    out.push(digest.clone());
                }
                pending.extend(map.values());
            }
            serde_json::Value::Array(items) => pending.extend(items),
            serde_json::Value::Null
            | serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_) => {}
        }
    }
}

/// Whether `node` is an application of `operator`: its body's operation
/// identity is the operator's catalog identity.
#[qsl_attrs::string_edge]
pub(super) fn applies(node: &SemanticNode, operator: ScalarOperator) -> bool {
    serde_json::from_slice::<serde_json::Value>(node.preimage())
        .ok()
        .is_some_and(|preimage| {
            preimage["body"]["operation"]["identity"] == operator.catalog_identity()
        })
}
