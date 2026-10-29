// SPDX-License-Identifier: AGPL-3.0-or-later
//! The golden vectors of FR-092, FR-093 and FR-094, resolved at test time.
//!
//! The specs publish each vector's exact preimage bytes. A dependency on
//! another vector's key is written `<digest of NAME>` (`<digest of FR-092
//! NAME>` across specs) and a recursion group's digest `<group digest of
//! FIRST>`, where FIRST is the first member named in the group's table. This
//! module recomputes every key from the preimages: a vector's key is the
//! SHA-256 of its resolved preimage bytes, and a group digest is the SHA-256
//! of the RFC 8785 array of its members' group-local preimage digests
//! (FR-092 "Recursive groups"). No key is recorded anywhere, so the spec and
//! the tests cannot drift apart and nothing is a copy of a computed value.
//! A placeholder for a QSpec vector key (`<QSpec NAME key>`) is recomputed
//! from QSpec's hand-written nominal preimage (`local_key`); a test compares
//! it with QSpec's published vector when `QSPEC_DIR` is set.

use std::collections::BTreeMap;

use serde_json::Value;

const SPECS: [(&str, &str); 3] = [
    (
        "FR-092",
        include_str!("../../../../spec/functional/FR-092-key-type-parameter-and-declared-nodes.md"),
    ),
    (
        "FR-093",
        include_str!(
            "../../../../spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md"
        ),
    ),
    (
        "FR-094",
        include_str!(
            "../../../../spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md"
        ),
    ),
];

/// A vector's `(key, preimage)`, both resolved.
pub type Vector = (String, String);

type Name = (String, String);

#[derive(Default)]
struct Raw {
    preimages: BTreeMap<Name, String>,
    /// Each recursion group by its first named member: the members by
    /// ordinal.
    groups: BTreeMap<Name, Vec<String>>,
}

/// `#[string_edge]`: reads the spec's markdown text, whose line markers are
/// the intake of this test-vector table.
#[ix_trace_rs::string_edge]
fn parse(tag: &str, spec: &str, raw: &mut Raw) {
    let mut lines = spec.lines().peekable();
    while let Some(line) = lines.next() {
        if let Some((first, _)) = line
            .split_once(", group digest (")
            .filter(|(label, _)| label.starts_with('G'))
        {
            let first = first.split('-').next().expect("a group label").to_owned();
            let mut members = Vec::new();
            for row in lines.by_ref().skip(3) {
                let Some(row) = row.strip_prefix("| ") else {
                    break;
                };
                let mut cells = row.split(" | ");
                let ordinal: usize = cells
                    .next()
                    .and_then(|cell| cell.parse().ok())
                    .expect("an ordinal");
                let member = cells.next().expect("a member");
                assert_eq!(ordinal, members.len(), "{first}: ordinals run from 0");
                members.push(member.split(' ').next().expect("a name").to_owned());
            }
            raw.groups.insert((tag.to_owned(), first), members);
            continue;
        }
        let Some(rest) = line.strip_prefix("**") else {
            continue;
        };
        let Some((name, _)) = rest.split_once("**: ") else {
            continue;
        };
        if lines.next() != Some("") || lines.next() != Some("```json") {
            continue;
        }
        let preimage = lines.next().expect("a vector preimage line").to_owned();
        assert_eq!(lines.next(), Some("```"), "{name}: one preimage line");
        raw.preimages
            .insert((tag.to_owned(), name.to_owned()), preimage);
    }
}

/// The lower-case hex SHA-256 of the RFC 8785 bytes of the JSON `text`,
/// through `quire-canonical` (the only canonical encoder in QSL). The specs'
/// preimages are RFC 8785 text, so this is also the digest of their bytes.
fn hex(text: &str) -> String {
    let value: Value = serde_json::from_str(text)
        .unwrap_or_else(|error| panic!("a vector preimage is JSON ({error}): {text}"));
    digest_of(&value)
}

fn digest_of(value: &Value) -> String {
    quire_canonical::sha256(value, quire_canonical::Limits::new(1 << 20))
        .expect("a preimage encodes canonically")
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The key QSpec publishes for its nominal vector `name`, read from
/// `$QSPEC_DIR`; `None` while it is unset. Test-only (`cfg(test)`): the
/// qualified core reads no ambient environment or filesystem in shipped
/// source, and `test-support` is shipped source.
#[cfg(test)]
pub fn published_key(name: &str) -> Option<String> {
    let dir = std::env::var_os("QSPEC_DIR")
        .unwrap_or_else(|| panic!("QSPEC_DIR must point to an available QSpec checkout"));
    let path =
        std::path::Path::new(&dir).join("proposals/checked-package-v2/node-identity-vectors.json");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let document: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parsing {}: {error}", path.display()));
    document["vectors"]
        .as_array()?
        .iter()
        .find(|vector| vector["name"] == name)
        .and_then(|vector| vector["sha256"].as_str())
        .map(str::to_owned)
}

fn owner_json() -> Value {
    // Members in ascending key order (RFC 8785 for these ASCII keys).
    serde_json::json!({"authority": "agent-ix", "identity": "example-model", "kind": "definition"})
}

/// The SHA-256 of the RFC 8785 bytes of `preimage`, which the callers below
/// write by hand: every object lists its members in ascending key order and
/// every number is a decimal string, so `serde_json` emits the canonical
/// bytes. No production identity type is used, which keeps this recomputation
/// independent of the code under test when `QSPEC_DIR` is unset.
fn key_of(preimage: &Value) -> String {
    digest_of(preimage)
}

/// The key of QSpec's nominal vector `name` (`dimension-length`,
/// `dimension-time`, `unit-metre`, `unit-second`), recomputed from the
/// hand-written preimage; no published key is recorded.
#[ix_trace_rs::string_edge]
pub fn local_key(name: &str) -> Option<String> {
    let dimension = |declared: &str| {
        key_of(&serde_json::json!({
            "owner": owner_json(),
            "qualified_declaration": ["Example", declared],
            "terms": [],
            "version": "quire.dimension-node/v1",
        }))
    };
    let unit = |declared: &str, of: &str| {
        Some(key_of(&serde_json::json!({
            "dimension_node_id": {
                "digest": local_key(of)?,
                "domain": "quire.checked-semantic-node/v1",
            },
            "offset": {"denominator": "1", "numerator": "0"},
            "owner": owner_json(),
            "qualified_declaration": ["Example", declared],
            "scale": {"denominator": "1", "numerator": "1"},
            "target_unit_node_id": null,
            "version": "quire.unit-node/v1",
        })))
    };
    match name {
        "dimension-length" => Some(dimension("Length")),
        "dimension-time" => Some(dimension("Time")),
        "unit-metre" => unit("metre", "dimension-length"),
        "unit-second" => unit("second", "dimension-time"),
        _ => None,
    }
}

struct Resolver<'a> {
    raw: &'a Raw,
    keys: BTreeMap<Name, Option<String>>,
    group_digests: BTreeMap<Name, Option<String>>,
}

impl Resolver<'_> {
    /// `text` with every placeholder replaced, or `None` when one needs a
    /// QSpec key that is unavailable.
    #[ix_trace_rs::string_edge]
    fn resolve(&mut self, tag: &str, text: &str) -> Option<String> {
        let mut out = String::new();
        let mut rest = text;
        while let Some(start) = rest.find('<') {
            out.push_str(&rest[..start]);
            let tail = &rest[start + 1..];
            let Some(end) = tail.find('>') else {
                out.push_str(&rest[start..]);
                rest = "";
                break;
            };
            let inner = &tail[..end];
            let replacement = if let Some(target) = inner.strip_prefix("digest of ") {
                let (target_tag, name) = match target.split_once(' ') {
                    Some((target_tag, name)) if target_tag.starts_with("FR-") => (target_tag, name),
                    _ => (tag, target),
                };
                Some(self.key(target_tag, name)?)
            } else if let Some(first) = inner.strip_prefix("group digest of ") {
                Some(self.group_digest(tag, first)?)
            } else if let Some(name) = inner
                .strip_prefix("QSpec ")
                .and_then(|name| name.strip_suffix(" key"))
            {
                Some(local_key(name)?)
            } else {
                None
            };
            match replacement {
                Some(replacement) => {
                    out.push_str(&replacement);
                    rest = &tail[end + 1..];
                }
                None => {
                    out.push('<');
                    rest = tail;
                }
            }
        }
        out.push_str(rest);
        Some(out)
    }

    fn key(&mut self, tag: &str, name: &str) -> Option<String> {
        let id = (tag.to_owned(), name.to_owned());
        if let Some(known) = self.keys.get(&id) {
            return known.clone();
        }
        let preimage = self.preimage(tag, name);
        let key = preimage.map(|preimage| hex(&preimage));
        self.keys.insert(id, key.clone());
        key
    }

    fn preimage(&mut self, tag: &str, name: &str) -> Option<String> {
        let raw = self
            .raw
            .preimages
            .get(&(tag.to_owned(), name.to_owned()))
            .unwrap_or_else(|| panic!("{tag} publishes no vector {name}"));
        self.resolve(tag, raw)
    }

    /// The group-local preimage digests of the group first named `first`,
    /// by ordinal: each member's preimage without its `group` member.
    fn member_signatures(&mut self, tag: &str, first: &str) -> Option<Vec<String>> {
        let members = self
            .raw
            .groups
            .get(&(tag.to_owned(), first.to_owned()))
            .unwrap_or_else(|| panic!("{tag} publishes no group {first}"))
            .clone();
        let marker = format!("\"group\":\"<group digest of {first}>\",");
        let mut locals = Vec::new();
        for member in &members {
            let raw = &self.raw.preimages[&(tag.to_owned(), member.clone())];
            let local = raw.replacen(&marker, "", 1);
            locals.push(self.resolve(tag, &local).map(|local| hex(&local)));
        }
        locals.into_iter().collect()
    }

    fn group_digest(&mut self, tag: &str, first: &str) -> Option<String> {
        let id = (tag.to_owned(), first.to_owned());
        if let Some(known) = self.group_digests.get(&id) {
            return known.clone();
        }
        let digest = self.member_signatures(tag, first).map(|locals| {
            let array = format!(
                "[{}]",
                locals
                    .iter()
                    .map(|local| format!("\"{local}\""))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            hex(&array)
        });
        self.group_digests.insert(id, digest.clone());
        digest
    }
}

fn raw() -> Raw {
    let mut raw = Raw::default();
    for (tag, spec) in SPECS {
        parse(tag, spec, &mut raw);
    }
    raw
}

/// `tag`'s vectors by name, `(key, preimage)`, resolved. A vector that
/// names a QSpec key has it recomputed.
pub fn vectors(tag: &str) -> BTreeMap<String, Vector> {
    let raw = raw();
    let mut resolver = Resolver {
        raw: &raw,
        keys: BTreeMap::new(),
        group_digests: BTreeMap::new(),
    };
    raw.preimages
        .keys()
        .filter(|(vector_tag, _)| vector_tag == tag)
        .filter_map(|(_, name)| {
            let preimage = resolver.preimage(tag, name)?;
            Some((name.clone(), (hex(&preimage), preimage)))
        })
        .collect()
}

/// The raw `(first member, members by ordinal)` recursion groups `tag`'s
/// tables publish.
pub fn group_tables(tag: &str) -> Vec<(String, Vec<String>)> {
    raw()
        .groups
        .iter()
        .filter(|((group_tag, _), _)| group_tag == tag)
        .map(|((_, first), members)| (first.clone(), members.clone()))
        .collect()
}

/// A vector's preimage as the spec writes it, placeholders unresolved.
pub fn raw_preimage(tag: &str, name: &str) -> String {
    raw().preimages[&(tag.to_owned(), name.to_owned())].clone()
}

/// The digest of the recursion group `tag` publishes first naming `first`,
/// recomputed from its members' preimages; `None` while a member needs an
/// unavailable QSpec key.
pub fn group_digest(tag: &str, first: &str) -> Option<String> {
    let raw = raw();
    let mut resolver = Resolver {
        raw: &raw,
        keys: BTreeMap::new(),
        group_digests: BTreeMap::new(),
    };
    resolver.group_digest(tag, first)
}
