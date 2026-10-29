// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-345 "Where it runs": the real profile-layering corpus, generated at run
//! time. Every unit selects a model by its digest (FR-056), so a committed
//! corpus would carry a frozen digest and churn on every model edit. The
//! corpus therefore is not committed: `make refinement-layering` calls
//! [`generate`], which reads `examples/config-version/model.semantic-ir.json`,
//! builds the timed model the temporal entries select, computes both
//! digests then, and writes every entry into a directory the gate reads.
//!
//! The corpus holds, for each of QSpec FR-453's edges E1 to E6, a `case`
//! both layers refuse with a typed refusal, a `case` over the edge's
//! distinguishing form and a `distinguishing` entry; and for each of the six
//! layers one `witness` entry, whose unit selects only that layer and holds
//! one clause.

use std::fs;
use std::path::Path;

use qsl_semantics::value::HeaderLayer;
use serde_json::{json, Value};

use super::layering::{Edge, EDGES};
use crate::error::{Error, Result};

/// The untimed model every entry but the temporal ones selects, relative to
/// the repository root.
const UNTIMED_MODEL: &str = "examples/config-version/model.semantic-ir.json";
/// The timed model's package identity.
const TIMED_PACKAGE: &str = "example/layering-timer";
const TIMED_VERSION: &str = "1.0.0";
/// The model file's name inside each entry directory.
const MODEL_FILE: &str = "model.semantic-ir.json";
const AUTHORITY: &str = "agent-ix";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// One model an entry selects: its bytes and the three values a `model`
/// statement repeats, each read from or computed over the bytes.
struct Model {
    bytes: Vec<u8>,
    identity: String,
    version: String,
    digest: String,
}

impl Model {
    fn of(bytes: Vec<u8>, path: &str) -> Result<Self> {
        let invalid = |detail: String| Error::RefinementCorpus {
            path: path.to_owned(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, detail),
        };
        let document: Value =
            serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
        let member = |name: &str| {
            document["package"][name]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid(format!("no package.{name}")))
        };
        let identity = member("identity")?;
        let version = member("version")?;
        let packages = qsl_semantics::model::intake::package_input([bytes.as_slice()]);
        let digest = packages
            .keys()
            .next()
            .map(|digest| qsl_semantics::model::key::hex(digest))
            .ok_or_else(|| invalid("the model is no package".to_owned()))?;
        Ok(Self {
            bytes,
            identity,
            version,
            digest,
        })
    }

    /// The unit under `layer` selecting this model as `M`, holding `body`.
    fn unit(&self, layer: HeaderLayer, body: &str) -> String {
        format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"{}\";\n\
             model M = {:?} version {:?} digest \"sha256-jcs:{}\";\n{body}",
            layer.identity(),
            self.identity,
            self.version,
            self.digest
        )
    }
}

/// The time units and the dense time source of `M`, in ms (ADR-026).
const TIME: &str = "dimension Time;\nunit s : Time = rational(1, 1);\n\
     unit ms : Time = rational(1, 1000) * s;\ntime M dense unit ms;\n";

/// A unit both sides of every edge refuse with a typed refusal: the bounded
/// `versionNumber` (`[0, 1000]`) compared with a Boolean.
const REFUSED: &str =
    "invariant Refused using v on M::ConfigVersion at current { self.versionNumber = true }\n";

/// Which model a unit selects.
#[derive(Clone, Copy)]
enum Selects {
    /// The config-version model.
    Untimed,
    /// The generated timed model.
    Timed,
}

/// The form each edge's child adds over its parent (QSpec FR-453 "Each
/// layer is a subset users pick"), and the model it is written over;
/// `None` for a pair that is no FR-453 edge.
fn distinguishing(edge: &Edge) -> Option<(Selects, String)> {
    use HeaderLayer::{
        ModelComplete, StateCore, StateGraph, StateQueries, TemporalComplete, ValueComplete,
    };
    let untimed = |body: &str| Some((Selects::Untimed, body.to_owned()));
    match (edge.parent, edge.child) {
        // A named predicate.
        (StateCore, StateQueries) => untimed(PREDICATE),
        // Object navigation.
        (StateQueries, StateGraph) | (ValueComplete, ModelComplete) => untimed(NAVIGATION),
        // A function, a complete-value declaration.
        (StateCore, ValueComplete) | (StateGraph, ModelComplete) => untimed(FUNCTION),
        // A `time` member of a state model (QSpec FR-453-AC-7).
        (ModelComplete, TemporalComplete) => {
            Some((Selects::Timed, format!("{TIME}{TIMED_INVARIANT}")))
        }
        _ => None,
    }
}

const PREDICATE: &str = "predicate AtMost using v(n: Integer): Boolean { n <= 1000 }\n\
     invariant Bounded using v on M::ConfigVersion at current { AtMost(self.versionNumber) }\n";
const NAVIGATION: &str =
    "invariant NoCycle using v on M::ConfigVersion at current { not reaches(self, self, parent) }\n";
const FUNCTION: &str = "function atMost using v(n: Integer): Boolean pure { n <= 1000 }\n\
     invariant Bounded using v on M::ConfigVersion at current { atMost(self.versionNumber) }\n";
const TIMED_INVARIANT: &str =
    "invariant Armed using v on M::Timer at current { self.armed or not self.armed }\n";

/// The witness unit for `layer`: the forms its row admits, holding one
/// `invariant` clause, the witness item, and the model it is written over.
fn witness_unit(layer: HeaderLayer) -> (Selects, String) {
    let untimed = |body: &str| (Selects::Untimed, body.to_owned());
    match layer {
        // A bounded-scalar invariant.
        HeaderLayer::StateCore => untimed(
            "invariant Bounded using v on M::ConfigVersion at current { self.versionNumber <= 1000 }\n",
        ),
        // A named-predicate query.
        HeaderLayer::StateQueries => untimed(PREDICATE),
        // A reachability invariant over a finite universe.
        HeaderLayer::StateGraph => untimed(NAVIGATION),
        // A validation function over values.
        HeaderLayer::ValueComplete => untimed(FUNCTION),
        // Navigation and a function together.
        HeaderLayer::ModelComplete => untimed(
            "function atMost using v(n: Integer): Boolean pure { n <= 1000 }\n\
             invariant Ordered using v on M::ConfigVersion at current { \
             not reaches(self, self, parent) and atMost(self.versionNumber) }\n",
        ),
        // QSpec FR-453-AC-8: a `time` member, the clock field `x`, a reset,
        // a guard `x >= 1 ms`, a time invariant and one `invariant` clause.
        HeaderLayer::TemporalComplete => (
            Selects::Timed,
            format!(
                "{TIME}post Arm using v on M::Timer::arm {{ self.armed and self.x = 0 ms }}\n\
                 pre Fire using v on M::Timer::fire {{ self.armed and self.x >= 1 ms }}\n\
                 time invariant Deadline when {{ self.armed }} {{ self.x <= 5 ms }}\n\
                 {TIMED_INVARIANT}"
            ),
        ),
    }
}

/// The timed model: the object `Timer` with the Boolean `armed` and the
/// clock `x`, the operations `arm` (modifies both) and `fire` (modifies
/// `armed`), and one closed population.
fn timed_document() -> Vec<u8> {
    let ident = |suffix: &str| format!("ix://{TIMED_PACKAGE}/{suffix}");
    let origin = |identity: &str| {
        json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity]}})
    };
    let field = |name: &str, type_ref: &str| {
        let identity = ident(&format!("Timer/{name}"));
        json!({
            "identity": identity, "name": name, "typeRef": type_ref, "presence": "required",
            "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false,
                "unique": type_ref != "ix://quire/native/Clock"},
            "origin": origin(&identity),
        })
    };
    let operation = |name: &str, modifies: &[&str]| {
        let identity = ident(&format!("Timer/{name}"));
        json!({
            "identity": identity, "name": name, "params": [], "pre": [], "post": [],
            "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
                "startLine": 1, "startColumn": 1}},
            "frame": {
                "modifies": modifies.iter().map(|field| ident(&format!("Timer/{field}"))).collect::<Vec<_>>(),
                "modifies_receiver": [], "creates": [], "deletes": [],
            },
        })
    };
    let construct = |name: &str, meaning: &str| {
        json!({"kind": {"module": TIMED_PACKAGE, "name": name}, "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    };
    let timer = ident("Timer");
    let population = ident("timers");
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": ident("spec"), "version": TIMED_VERSION, "dialect": "spec-bundle",
            "digest": PLACEHOLDER},
        "package": {"identity": TIMED_PACKAGE, "version": TIMED_VERSION,
            "manifestDigest": PLACEHOLDER, "mappingVersions": [], "profileVersions": [],
            "lockDigest": PLACEHOLDER},
        "occurrences": [], "extensions": [],
        "constructs": [
            construct("object_type", qsl_semantics::model::intake::meaning::OBJECT_TYPE),
            construct("population", qsl_semantics::model::intake::meaning::POPULATION),
        ],
        "types": [{
            "identity": timer, "displayName": timer,
            "kind": {"module": TIMED_PACKAGE, "name": "object_type"}, "roles": [],
            "origin": origin(&timer), "constraints": [], "extensions": [],
            "unknownPolicy": "reject", "supertypes": [], "relationships": [],
            "fields": [field("armed", "ix://quire/native/Boolean"),
                field("x", "ix://quire/native/Clock")],
            "operations": [operation("arm", &["armed", "x"]), operation("fire", &["armed"])],
        }],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": TIMED_PACKAGE, "name": "population"},
            "members": [timer], "extent": "closed", "origin": origin(&population),
        }],
    })
    .to_string()
    .into_bytes()
}

/// One entry's files: its `entry.json` and the unit and model files it
/// names.
struct Entry {
    directory: String,
    entry: Value,
    files: Vec<(&'static str, Vec<u8>)>,
}

fn source(path: &str, name: &str) -> Value {
    json!({
        "path": path,
        "authority": AUTHORITY,
        "identity": format!("ix://example/refinement-layering/{name}"),
    })
}

/// A `case` or `distinguishing` entry over `edge`: one unit under the
/// edge's parent and the same unit under its child.
fn pair(kind: &str, name: &str, edge: &Edge, model: &Model, body: &str) -> Entry {
    Entry {
        directory: name.to_owned(),
        entry: json!({
            "kind": kind,
            "parent": source("parent.native", &format!("{name}/parent")),
            "child": source("child.native", &format!("{name}/child")),
            "packages": [MODEL_FILE],
            "dependencies": [],
        }),
        files: vec![
            ("parent.native", model.unit(edge.parent, body).into_bytes()),
            ("child.native", model.unit(edge.child, body).into_bytes()),
            (MODEL_FILE, model.bytes.clone()),
        ],
    }
}

fn witness(layer: HeaderLayer, model: &Model, body: &str) -> Entry {
    let name = format!("witness-{}", layer_slug(layer));
    Entry {
        entry: json!({
            "kind": "witness",
            "unit": source("unit.native", &name),
            "layer": layer.identity(),
            "packages": [MODEL_FILE],
            "dependencies": [],
        }),
        files: vec![
            ("unit.native", model.unit(layer, body).into_bytes()),
            (MODEL_FILE, model.bytes.clone()),
        ],
        directory: name,
    }
}

/// The layer identity's name between `quire.` and `/v1`, for a directory.
fn layer_slug(layer: HeaderLayer) -> String {
    layer
        .identity()
        .trim_start_matches("quire.")
        .trim_end_matches("/v1")
        .replace('.', "-")
}

/// Every entry of the real corpus, over the untimed model `untimed` and the
/// timed model `timed`.
fn entries(untimed: &Model, timed: &Model) -> Result<Vec<Entry>> {
    let model = |selects| match selects {
        Selects::Untimed => untimed,
        Selects::Timed => timed,
    };
    let mut entries = Vec::new();
    for edge in &EDGES {
        let name = edge.name.to_ascii_lowercase();
        entries.push(pair(
            "case",
            &format!("{name}-refused"),
            edge,
            untimed,
            REFUSED,
        ));
        let (selects, body) = distinguishing(edge).ok_or_else(|| Error::RefinementCorpus {
            path: edge.name.to_owned(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "the edge has no distinguishing form",
            ),
        })?;
        let over = model(selects);
        entries.push(pair("case", &format!("{name}-form"), edge, over, &body));
        entries.push(pair(
            "distinguishing",
            &format!("{name}-distinguishing"),
            edge,
            over,
            &body,
        ));
    }
    for layer in HeaderLayer::ALL {
        let (selects, body) = witness_unit(layer);
        entries.push(witness(layer, model(selects), &body));
    }
    Ok(entries)
}

/// Write the real layering corpus into `out`, with both model digests
/// computed from the model bytes. Nothing it writes is committed.
///
/// # Errors
/// [`Error::RefinementCorpus`] when the untimed model cannot be read or a
/// corpus file cannot be written.
pub fn generate(root: &Path, out: &Path) -> Result<()> {
    let path = root.join(UNTIMED_MODEL);
    let bytes = fs::read(&path).map_err(|source| Error::RefinementCorpus {
        path: path.display().to_string(),
        source,
    })?;
    let untimed = Model::of(bytes, &path.display().to_string())?;
    let timed = Model::of(timed_document(), TIMED_PACKAGE)?;
    for entry in entries(&untimed, &timed)? {
        let directory = out.join(&entry.directory);
        let write_error = |path: &Path, source| Error::RefinementCorpus {
            path: path.display().to_string(),
            source,
        };
        fs::create_dir_all(&directory).map_err(|source| write_error(&directory, source))?;
        let entry_path = directory.join("entry.json");
        fs::write(&entry_path, entry.entry.to_string())
            .map_err(|source| write_error(&entry_path, source))?;
        for (name, bytes) in entry.files {
            let path = directory.join(name);
            fs::write(&path, bytes).map_err(|source| write_error(&path, source))?;
        }
    }
    Ok(())
}
