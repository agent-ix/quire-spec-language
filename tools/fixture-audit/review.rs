// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-012 AC-1/2: historical review bindings and discriminating negative controls.
use crate::{
    error::{ensure, Code, Error, Result},
    input::{self, array, field, text, Input},
};
use qsl_foundation::ByteDigest;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Exact byte integrity; never a semantic/canonical identity.
pub(crate) fn digest(raw: &[u8]) -> String {
    ByteDigest::of(raw).to_string()
}

macro_rules! leaf_ids {
    ($($name:ident),+ $(,)?) => {$(
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        struct $name(Box<str>);
    )+};
}
leaf_ids!(
    Authority,
    ArtifactIdentity,
    RevisionNamespace,
    RevisionValue
);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Key {
    authority: Authority,
    identity: ArtifactIdentity,
    namespace: RevisionNamespace,
    revision: RevisionValue,
}

#[derive(Default)]
struct Registry(BTreeMap<Key, (String, String)>);
impl Registry {
    fn accept(&mut self, reference: &Value, raw: &[u8], payload: &Value) -> Result<()> {
        let claimed = text(field(reference, "digest")?)?;
        ensure(
            digest(raw) == claimed,
            Code::DigestMismatch,
            "selected artifact bytes changed",
        )?;
        let profile = text(field(reference, "profile")?)?;
        ensure(
            text(field(payload, "fixtureVersion")?)? == profile,
            Code::ProfileMismatch,
            "fixture profile mismatch",
        )?;
        let identity = ["identity", "id", "invocation"]
            .iter()
            .find_map(|name| payload.get(name))
            .ok_or_else(|| Error::new(Code::IdentityMismatch, "payload has no identity"))?;
        let revision = field(reference, "revision")?;
        let namespace = text(field(revision, "namespace")?)?;
        let revision_value = text(field(revision, "value")?)?;
        let payload_revision = field(payload, "revision")?;
        ensure(
            text(identity)? == text(field(reference, "identity")?)?
                && text(field(payload_revision, "namespace")?)? == namespace
                && text(field(payload_revision, "value")?)? == revision_value,
            Code::IdentityMismatch,
            "payload identity/revision mismatch",
        )?;
        let key = Key {
            authority: Authority(text(field(reference, "authority")?)?.into()),
            identity: ArtifactIdentity(text(identity)?.into()),
            namespace: RevisionNamespace(namespace.into()),
            revision: RevisionValue(revision_value.into()),
        };
        let binding = (profile.to_owned(), claimed.to_owned());
        if let Some(previous) = self.0.get(&key) {
            ensure(
                previous == &binding,
                Code::IdentityConflict,
                "immutable review key reused for changed content",
            )?;
        } else {
            self.0.insert(key, binding);
        }
        Ok(())
    }
}

fn refused<T>(result: Result<T>, expected: Code) -> Result<()> {
    match result {
        Err(error) if error.code == expected => Ok(()),
        Err(error) => Err(Error::new(
            Code::InvalidFixture,
            format!("negative control expected {expected}, got {error}"),
        )),
        Ok(_) => Err(Error::new(
            Code::InvalidFixture,
            format!("negative control incorrectly accepted: {expected}"),
        )),
    }
}

fn controls(reference: &Value, raw: &[u8], name: &str, changed: Value) -> Result<()> {
    let payload = input::json(raw)?;
    let mut registry = Registry::default();
    registry.accept(reference, raw, &payload)?;
    let mut modified = payload;
    let slot = modified
        .as_object_mut()
        .and_then(|v| v.get_mut(name))
        .ok_or_else(|| {
            Error::new(
                Code::InvalidFixture,
                format!("control field {name} missing"),
            )
        })?;
    ensure(
        *slot != changed,
        Code::InvalidFixture,
        "negative control does not change selected value",
    )?;
    *slot = changed;
    let changed_raw =
        serde_json::to_vec(&modified).map_err(|e| Error::new(Code::InvalidJson, e.to_string()))?;
    ensure(
        raw != changed_raw,
        Code::InvalidFixture,
        "negative control did not change bytes",
    )?;
    refused(
        registry.accept(reference, &changed_raw, &modified),
        Code::DigestMismatch,
    )?;
    let mut new_reference = reference.clone();
    let digest_slot = new_reference
        .as_object_mut()
        .and_then(|v| v.get_mut("digest"))
        .ok_or_else(|| Error::new(Code::InvalidFixture, "digest field missing"))?;
    *digest_slot = Value::String(digest(&changed_raw));
    refused(
        registry.accept(&new_reference, &changed_raw, &modified),
        Code::IdentityConflict,
    )
}

/// Execute the original three-role controls and strict duplicate-key refusal.
pub(crate) fn self_test() -> Result<String> {
    for (name, original, changed) in [
        ("objects", json!([1]), json!([2])),
        ("frame", json!({"allowed":[]}), json!({"allowed":["x"]})),
        ("result", json!(true), json!(false)),
    ] {
        let identity = format!("test:{name}");
        let mut payload = json!({"identity":identity,"revision":{"namespace":"test","value":"1"},"fixtureVersion":"test/1"});
        let object = payload
            .as_object_mut()
            .ok_or_else(|| Error::new(Code::InvalidFixture, "control construction failed"))?;
        object.insert(name.into(), original);
        let raw = serde_json::to_vec(&payload)
            .map_err(|e| Error::new(Code::InvalidJson, e.to_string()))?;
        let reference = json!({"authority":"test","identity":identity,"revision":{"namespace":"test","value":"1"},"profile":"test/1","digest":digest(&raw)});
        controls(&reference, &raw, name, changed)?;
    }
    refused(input::json(br#"{"x":1,"x":2}"#), Code::InvalidJson)?;
    // Exercise the stable code vocabulary as part of this tool's own audit,
    // not by parsing human diagnostic messages on failure paths.
    for code in Code::all() {
        ensure(
            Code::from_code(code.as_str()) == Some(*code),
            Code::InvalidFixture,
            "ambiguous diagnostic catalog",
        )?;
    }
    Ok("passed: 6 content/digest negative controls for snapshot/frame/invocation; duplicate keys refused; no evaluator executed".into())
}

struct Review {
    input: Input,
    registry: Registry,
    inspected: BTreeMap<String, (Value, Vec<u8>)>,
}
impl Review {
    fn resolve(&mut self, entry: &Value) -> Result<Value> {
        let path = text(field(entry, "path")?)?;
        let reference = field(entry, "artifactRef")?;
        let raw = self.input.read(path)?;
        let payload = self.input.decode(&raw)?;
        self.registry.accept(reference, &raw, &payload)?;
        self.inspected.insert(path.into(), (reference.clone(), raw));
        Ok(payload)
    }
}

/// Audit the selected historical invocation manifest without interpreting outcomes.
pub(crate) fn audit(root: &Path) -> Result<String> {
    let mut run = Review {
        input: Input::new(root)?,
        registry: Registry::default(),
        inspected: BTreeMap::new(),
    };
    let manifest = run.input.json("invocation-cases.json")?;
    ensure(
        text(field(&manifest, "fixtureVersion")?)? == "agent-a-invocation-cases/1-draft",
        Code::ProfileMismatch,
        "review manifest version",
    )?;
    let cases = array(field(&manifest, "cases")?)?;
    for case in cases {
        let invocation = run.resolve(field(case, "input")?)?;
        for role in ["pre", "post"] {
            let observation = field(&invocation, role)?;
            if observation.get("availability").map(text).transpose()? != Some("unavailable") {
                run.resolve(observation)?;
            }
        }
        run.resolve(field(&invocation, "frameBinding")?)?;
    }
    for frame in array(field(&manifest, "frameBindings")?)? {
        run.resolve(frame)?;
    }
    let mut exercised = BTreeSet::new();
    for (reference, raw) in run.inspected.values() {
        let profile = text(field(reference, "profile")?)?;
        if exercised.contains(profile) {
            continue;
        }
        let (name, changed) = match profile {
            "agent-a-snapshot/1-draft" => {
                let payload = input::json(raw)?;
                let mut objects = array(field(&payload, "objects")?)?.to_vec();
                objects.push(json!({"identity":"audit:changed-object"}));
                ("objects", json!(objects))
            }
            "agent-a-frame-binding/1-draft" => (
                "frame",
                json!({"allowedChangedFields":["changed"],"allowedCreatedTypes":[],"allowedDeletedTypes":[]}),
            ),
            "agent-a-invocation/1-draft" => {
                let payload = input::json(raw)?;
                let previous = field(&payload, "result")?.as_bool().ok_or_else(|| {
                    Error::new(Code::InvalidFixture, "expected Boolean invocation result")
                })?;
                ("result", json!(!previous))
            }
            _ => return Err(Error::new(Code::ProfileMismatch, "unknown fixture profile")),
        };
        controls(reference, raw, name, changed)?;
        exercised.insert(profile);
    }
    ensure(
        exercised.len() == 3,
        Code::InvalidFixture,
        "missing role negative control",
    )?;
    Ok(format!("passed: {} exact artifact files, {} invocation cases; 6 content/digest negative controls; no evaluator executed",run.inspected.len(),cases.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    #[trace("TC-001", "FR-012-AC-1", "FR-017-AC-4")]
    #[test]
    fn identity_negative_controls() {
        let summary = self_test().unwrap();
        assert!(summary.contains("6 content/digest negative controls"));
        let payload = json!({"identity":"original","revision":{"namespace":"n","value":"1"},"fixtureVersion":"v"});
        let raw = serde_json::to_vec(&payload).unwrap();
        let mut reference = json!({"authority":"a","identity":"wrong","revision":{"namespace":"n","value":"1"},"profile":"v","digest":digest(&raw)});
        assert_eq!(
            Registry::default()
                .accept(&reference, &raw, &payload)
                .unwrap_err()
                .code,
            Code::IdentityMismatch
        );
        reference["identity"] = json!("original");
        reference["profile"] = json!("wrong");
        assert_eq!(
            Registry::default()
                .accept(&reference, &raw, &payload)
                .unwrap_err()
                .code,
            Code::ProfileMismatch
        );
    }
}
