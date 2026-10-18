// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-804 (FR-306-AC-1, FR-306-AC-2): the abstraction relation of a checked
//! package is written as one `correspondence`/`abstraction_relation` v2 node
//! (QSpec FR-451) that enters the `package_id`. The relation is bound with
//! `CheckedPackage::bind_abstraction`, as no source compile reads an
//! `abstraction` declaration yet, and the bytes are read back through IR's
//! reader. The expected body is written here from FR-451.

use qsl_semantics::check::{
    AbstractionNodeRefusal, AuthoredBinding, FrameBinding, ObjectBinding, OperationKey,
    PopulationBinding, RustField, RustParameter, RustPath, RustReceiver,
};
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::Identifier;

use super::*;

#[path = "../../../../qsl-semantics/tests/support/abstraction.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "the export tests load the same fixture"
)]
mod fixture;

/// The unit: the `attemptUpdate` postcondition names `ConfigVersion`'s
/// operation, so the package holds its object type, frame and anchor nodes.
const CLAUSES: &str = "post Updated using v on M::ConfigVersion::attemptUpdate { true }\n";

fn declaration(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: fixture::PACKAGE.to_owned(),
        node: fixture::id(suffix),
    }
}

fn identifier(text: &str) -> Identifier {
    Identifier::new(text).expect("an identifier")
}

/// `ConfigVersion` bound to `ConfigVersion`, its `version` field to
/// `version_field` and `note` to `note`.
fn config_object(version_field: &str) -> AuthoredBinding {
    AuthoredBinding::Object {
        key: declaration("ConfigVersion"),
        value: ObjectBinding {
            rust_type: RustPath::new("ConfigVersion"),
            fields: BTreeMap::from([
                (identifier("version"), RustField::new(version_field)),
                (identifier("note"), RustField::new("note")),
            ]),
        },
    }
}

fn configs_population() -> AuthoredBinding {
    AuthoredBinding::Population {
        key: declaration("configs"),
        value: PopulationBinding {
            collection: RustPath::new("state::configs"),
        },
    }
}

fn attempt_update(function: &str) -> AuthoredBinding {
    AuthoredBinding::Frame {
        key: OperationKey {
            declaring: declaration("ConfigVersion"),
            operation: identifier("attemptUpdate"),
        },
        value: FrameBinding {
            function: RustPath::new(function),
            receiver: RustReceiver::SelfValue,
            parameters: BTreeMap::from([
                (identifier("n"), RustParameter::new("count")),
                (identifier("m"), RustParameter::new("limit")),
            ]),
        },
    }
}

/// FR-304-AC-1's relation, with `version` bound to `version_field`.
fn three_bindings(version_field: &str) -> Vec<AuthoredBinding> {
    vec![
        config_object(version_field),
        configs_population(),
        attempt_update("update"),
    ]
}

/// The fixture package: the document the unit is checked against, its digest
/// and the linked package.
fn linked() -> (Vec<u8>, String, CheckedPackage) {
    let document = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         {CLAUSES}",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    (document, digest, CheckedPackage::link(graph))
}

struct Emitted {
    emission: Emission,
    wire: Value,
    evidence: CheckedPackageEvidence,
}

/// The fixture package with `bindings` bound (none when `None`), emitted.
fn emit_with(bindings: Option<&[AuthoredBinding]>) -> Emitted {
    let (document, digest, mut package) = linked();
    if let Some(bindings) = bindings {
        package
            .bind_abstraction(bindings)
            .expect("the relation checks");
    }
    // The relation's occurrence is at the package root, which names no region
    // of the unit until a source declaration places it, so the unit places
    // every occurrence.
    let source = package.graph().source().clone();
    let emission = emit_package(&package, move |_| {
        Some(SourceRegion::new(source.clone(), 0, 1).expect("a nonempty region"))
    })
    .expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(digest.as_str(), document.as_slice())
        .expect("a fresh evidence key");
    Emitted {
        emission,
        wire,
        evidence,
    }
}

impl Emitted {
    fn package_id(&self) -> qsl_semantics::library::PackageId {
        self.emission.package().package_id()
    }

    fn read(&self) -> Read {
        read_with(&self.emission, &self.evidence)
    }

    /// The one node of `tag`/`form` that `matches`.
    fn node(&self, tag: &str, form: &str, matches: impl Fn(&Value) -> bool) -> &Value {
        let found: Vec<&Value> = super::owners::node_where(&self.wire, tag, form)
            .into_iter()
            .filter(|node| matches(node))
            .collect();
        let [node] = found[..] else {
            panic!("one {tag}/{form} node: {found:#?}");
        };
        node
    }

    /// The `model`/`object_type` node declared as `suffix`.
    fn object_type(&self, suffix: &str) -> &Value {
        self.node("model", "object_type", |node| {
            node["owner"]["node"] == fixture::id(suffix)
        })
    }

    /// The `relation`/`population` node of the population declared as
    /// `suffix`: the one whose `identity` text is its declaration's node.
    fn population(&self, suffix: &str) -> &Value {
        self.node("relation", "population", |node| {
            node["body"]["members"][0]["value"]["value"] == fixture::id(suffix)
        })
    }

    /// The package's one relation node.
    fn relation(&self) -> &Value {
        self.node("correspondence", "abstraction_relation", |_| true)
    }
}

/// FR-306-AC-1 (TC-804 step 1): the unit with FR-304-AC-1's relation emits
/// one relation node whose body holds the three bindings the in-process
/// relation holds -- keyed by the object type's, the population's and the
/// operation's nodes, valued by the Rust spellings split into segments -- in
/// FR-451's canonical order, typed by itself, depending on exactly its
/// targets, with one `declaration` occurrence; IR's reader admits the bytes.
#[trace("TC-804", "FR-306-AC-1")]
#[test]
fn the_relation_is_written_as_one_node_holding_the_three_bindings() {
    let emitted = emit_with(Some(&three_bindings("version")));
    let config = emitted.object_type("ConfigVersion")["node_id"].clone();
    let configs = emitted.population("configs")["node_id"].clone();
    let relation = emitted.relation();

    assert_eq!(
        relation["body"],
        json!({
            "term": "abstraction_relation",
            "objects": [{
                "type": config,
                "rust_type": ["ConfigVersion"],
                "fields": [
                    {"name": "note", "rust_field": "note"},
                    {"name": "version", "rust_field": "version"},
                ],
            }],
            "populations": [{"population": configs, "collection": ["state", "configs"]}],
            "frames": [{
                "context": config,
                "operation": "attemptUpdate",
                "function": ["update"],
                "receiver": "self",
                "parameters": [
                    {"name": "m", "rust_parameter": "limit"},
                    {"name": "n", "rust_parameter": "count"},
                ],
            }],
        })
    );
    let mut targets = vec![config, configs];
    targets.sort_by(|left, right| left["digest"].as_str().cmp(&right["digest"].as_str()));
    assert_eq!(relation["dependencies"], json!(targets));
    assert_eq!(relation["semantic_type"], relation["node_id"]);
    assert_eq!(
        relation["occurrences"],
        json!([{"role": "declaration", "ordinal": 0}])
    );
    assert!(
        matches!(emitted.read(), Read::Verified { .. }),
        "{:?}",
        emitted.read()
    );
}

/// TC-804 step 1: a package that declares no relation holds no relation node.
#[test]
fn a_package_with_no_relation_writes_no_relation_node() {
    let emitted = emit_with(None);
    assert!(
        super::owners::node_where(&emitted.wire, "correspondence", "abstraction_relation")
            .is_empty()
    );
    assert!(super::owners::node_where(&emitted.wire, "relation", "population").is_empty());
    assert!(matches!(emitted.read(), Read::Verified { .. }));
}

/// FR-306-AC-2 (TC-804 step 2): emitting the same unit again gives the same
/// `package_id` and relation node id; changing one `ObjectBinding` field's
/// `RustField` gives a different `package_id` and a different relation node
/// id.
#[trace("TC-804", "FR-306-AC-2")]
#[test]
fn a_rebinding_is_a_new_relation_node_and_a_new_package_id() {
    let first = emit_with(Some(&three_bindings("version")));
    let again = emit_with(Some(&three_bindings("version")));
    assert_eq!(first.package_id(), again.package_id());
    assert_eq!(first.relation()["node_id"], again.relation()["node_id"]);

    let rebound = emit_with(Some(&three_bindings("revision")));
    assert_ne!(first.package_id(), rebound.package_id());
    assert_ne!(first.relation()["node_id"], rebound.relation()["node_id"]);
    assert!(matches!(rebound.read(), Read::Verified { .. }));
}

/// A Rust path with an empty segment has no array of segments, so the
/// emission refuses with the path rather than writing a node IR's reader
/// refuses.
#[test]
fn a_rust_path_with_an_empty_segment_refuses_the_emission() {
    let (_, _, mut package) = linked();
    let mut bindings = three_bindings("version");
    bindings[2] = attempt_update("update::");
    package
        .bind_abstraction(&bindings)
        .expect("the relation checks");
    assert_eq!(
        emit_checked(&package).expect_err("the emission refuses"),
        EmitRefusal::Abstraction(AbstractionNodeRefusal::EmptyRustSegment {
            spelling: "update::".to_owned(),
        })
    );
}
