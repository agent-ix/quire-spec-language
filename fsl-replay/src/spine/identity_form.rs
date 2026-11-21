// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-650 "Identity spellings": the structural JSON spellings of a universe
//! and of a most-specific effective type, never a digest or a hex rendering.
//! One writer and one reader per identity, so a written identity reads back
//! to the identity it was written from.

use filament_exact::{
    EffectiveId, NodeKey, NodeKeyView, OriginalDeclarationKey, PathSegment, PathSegmentView,
    UniverseId, MAX_NODE_ORDINAL,
};
use fsl_foundation::diagnostic::Code;
use serde_json::{json, Map, Value as Json};

use super::run_value::{defect, defect_below, wrong_kind, wrong_kind_at, Path, ReadStop};

/// The universe as `{"package", "path", "ordinal"}`: the node id of the
/// declaration of the universe's least root type (FSpec FR-181's
/// `<node id>`).
pub fn universe_form(universe: &UniverseId) -> Json {
    node_form(universe.root_type().view())
}

/// A node id as `{"package": S, "path": [S | node id, ...], "ordinal": N}`.
fn node_form(node: NodeKeyView<'_>) -> Json {
    let path = node
        .path()
        .map(|segment| match segment {
            PathSegmentView::Text(text) => Json::String(text.to_owned()),
            PathSegmentView::Node(nested) => node_form(nested),
        })
        .collect::<Vec<_>>();
    json!({"package": node.package(), "path": path, "ordinal": node.ordinal()})
}

/// The effective id as `{"owner_effective_type": E | null, "original":
/// {"package", "node"}}`, `E` nested in FSpec FR-201's member structure.
pub fn effective_form(id: &EffectiveId) -> Json {
    let mut form = Json::Null;
    for original in id.owner_chain() {
        form = json!({
            "owner_effective_type": form,
            "original": {"package": original.package(), "node": original.node()},
        });
    }
    form
}

pub(crate) fn closed<'a>(
    form: &'a Json,
    members: &[&str],
    path: &Path,
) -> Result<&'a Map<String, Json>, ReadStop> {
    let Some(object) = form.as_object() else {
        return Err(wrong_kind(path));
    };
    closed_object(object, members, path)?;
    Ok(object)
}

/// `object` has exactly `members`: an extra one is `unknown-member`, a
/// missing one `missing-member`.
pub(crate) fn closed_object(
    object: &Map<String, Json>,
    members: &[&str],
    path: &Path,
) -> Result<(), ReadStop> {
    if let Some(extra) = object.keys().find(|name| !members.contains(&name.as_str())) {
        return Err(defect_below(
            Code::InvalidRuntimeInput,
            "unknown-member",
            path,
            &[extra],
        ));
    }
    if members.iter().any(|member| !object.contains_key(*member)) {
        return Err(defect(Code::InvalidRuntimeInput, "missing-member", path));
    }
    Ok(())
}

/// The universe `form` spells, at `path`.
pub(crate) fn read_universe(form: &Json, path: &mut Path) -> Result<UniverseId, ReadStop> {
    read_node(form, path).map(UniverseId::new)
}

fn read_node(form: &Json, path: &mut Path) -> Result<NodeKey, ReadStop> {
    let object = closed(form, &["package", "path", "ordinal"], path)?;
    let package = match &object["package"] {
        Json::String(package) => package.clone(),
        _ => return Err(wrong_kind_at(path, "package")),
    };
    let Some(segments) = object["path"].as_array() else {
        return Err(wrong_kind_at(path, "path"));
    };
    let ordinal = match object["ordinal"].as_u64() {
        Some(ordinal) if ordinal <= MAX_NODE_ORDINAL => ordinal,
        Some(_) => {
            return Err(defect_below(
                Code::InvalidRuntimeInput,
                "invalid-value",
                path,
                &["ordinal"],
            ))
        }
        None => return Err(wrong_kind_at(path, "ordinal")),
    };
    let mut path_segments = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        let position = index.to_string();
        path.push("path");
        path.push(&position);
        let read = match segment {
            Json::String(text) => Ok(PathSegment::Text(text.clone())),
            Json::Object(_) => read_node(segment, path).map(PathSegment::Node),
            _ => Err(wrong_kind(path)),
        };
        path.pop();
        path.pop();
        path_segments.push(read?);
    }
    NodeKey::new(package, path_segments, ordinal)
        .map_err(|_| defect(Code::InvalidRuntimeInput, "invalid-value", path))
}

/// The effective id `form` spells, at `path`.
pub(crate) fn read_effective(form: &Json, path: &Path) -> Result<EffectiveId, ReadStop> {
    // Read from the leaf down to the root, then fold root first, so the
    // nesting depth costs no call stack.
    let mut originals = Vec::new();
    let mut here = path.clone();
    let mut current = form;
    loop {
        let object = closed(current, &["owner_effective_type", "original"], &here)?;
        let mut below = here.clone();
        below.push("original");
        let original = closed(&object["original"], &["package", "node"], &below)?;
        let (Json::String(package), Json::String(node)) = (&original["package"], &original["node"])
        else {
            return Err(wrong_kind(&below));
        };
        originals.push(OriginalDeclarationKey::new(package.clone(), node.clone()));
        match &object["owner_effective_type"] {
            Json::Null => break,
            owner @ Json::Object(_) => {
                here.push("owner_effective_type");
                current = owner;
            }
            _ => return Err(wrong_kind_at(&here, "owner_effective_type")),
        }
    }
    let mut identity: Option<EffectiveId> = None;
    for original in originals.into_iter().rev() {
        identity = Some(EffectiveId::new(identity, original));
    }
    identity.ok_or(ReadStop::Fault("effective-id-has-an-original"))
}

/// Whether `form` spells an effective id: `effective_form`'s object, never
/// its `null` of an empty chain.
pub(crate) fn is_effective_form(form: &Json) -> bool {
    !form.is_null() && read_effective(form, &Path::new()).is_ok()
}

/// A node id in the universe spelling, for a refusal that holds the node of
/// a universe.
pub fn node_key_form(node: &NodeKey) -> Json {
    node_form(node.view())
}
