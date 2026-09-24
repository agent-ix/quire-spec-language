// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-078/090: independently composed expectations for a real minimal unit.

use quire_spec_language::native_model::NativeModel;
use sha2::{Digest, Sha256};

use super::native_rule_model;

// The physical record boundary is determined by fixture composition, before
// either the model's located decoder or the package compiler sees the input.
const MODEL_PREFIX: &str = concat!(
    "{\n",
    "  \"license\": \"AGPL-3.0-only\",\n",
    "  \"package\": \"example/package-model\",\n",
    "  \"requirement\": \"PackageModel\",\n",
    "  \"revision\": 1,\n",
    "  \"scalars\": [\n",
    "    {\"name\": \"Id\", \"kind\": \"text\", \"max_scalars\": 8}\n",
    "  ],\n",
    "  \"records\": [\n",
    "    "
);
const MODEL_NODE: &str = "{\"name\": \"Node\", \"fields\": []}";
const MODEL_SUFFIX: &str = concat!(
    ",\n",
    "    {\"name\": \"NodeRef\", \"fields\": [\n",
    "      {\"name\": \"id\", \"type\": {\"kind\": \"scalar\", \"name\": \"Id\"}}\n",
    "    ]}\n",
    "  ],\n",
    "  \"values\": [\n",
    "    {\"name\": \"self\", \"kind\": \"state\", \"type\": {\"kind\": \"record\", \"name\": \"Node\"}}\n",
    "  ],\n",
    "  \"objects\": [\n",
    "    {\"record\": \"Node\", \"reference\": \"NodeRef\", \"identity_field\": \"id\", \"universe\": \"nodes\"}\n",
    "  ],\n",
    "  \"operations\": []\n",
    "}\n"
);

pub(super) const CLAUSE: &str = "invariant Rule on M::Node at current { true }";
pub(super) const SECOND_CLAUSE: &str = "invariant Second on M::Node at current { false }";
pub(super) const OWNER: &str =
    r#"{"package":"example/package-model","requirement":"PackageModel","revision":1}"#;
const AUTHOR: &str = r#"{"package":"example/package-rules","requirement":"Rule","revision":2}"#;
const SEMANTICS: &str = r#"{"language":"ix:native","edition":"0-draft","syntax_profile":"state-finite/0-draft","model_profile":"native-state-model/1","checking_contract":"native-checked-clauses/1","ir_revision":"690bde7f2dc58662cf9ff0595c2c0e3b17107c6f","base_definition":{"revision":"e897f810a7356d4ce8fd19026221ebda7b65596f","digest":"sha256:8bc68a3c7e46d26c7191dfbb662d4d63070af9fedc9ce985fb1885f7efe29429"},"rules_definition":{"revision":"e897f810a7356d4ce8fd19026221ebda7b65596f","digest":"sha256:d9eb316752ac45d7984b355a054cd279ef9749164a92c7f61fbf621fe280588b"}}"#;

pub(super) fn model() -> NativeModel {
    let input = format!("{MODEL_PREFIX}{MODEL_NODE}{MODEL_SUFFIX}");
    native_rule_model::from_text(&input, "package-model.json", "1")
        .expect("minimal source-derived model setup")
        .model()
}

pub(super) fn source_prefix(model: &NativeModel) -> String {
    format!(
        "{}model M = \"example/package-model\" version \"1\" digest \"{}\";\n",
        super::HEADER,
        model.digest()
    )
}

pub(super) fn source(model: &NativeModel) -> String {
    format!("{}{CLAUSE}\n", source_prefix(model))
}

pub(super) fn source_multiple(model: &NativeModel) -> String {
    format!("{}{CLAUSE}\n{SECOND_CLAUSE}\n", source_prefix(model))
}

pub(super) struct Expected {
    pub canonical: String,
    pub artifact: String,
    pub digest: String,
}

pub(super) fn expected(
    model: &NativeModel,
    identity_json: &str,
    revision_json: &str,
    formal_revision: u64,
) -> Expected {
    expected_inner(model, identity_json, revision_json, formal_revision, false)
}

pub(super) fn expected_multiple(
    model: &NativeModel,
    identity_json: &str,
    revision_json: &str,
    formal_revision: u64,
) -> Expected {
    expected_inner(model, identity_json, revision_json, formal_revision, true)
}

fn expected_inner(
    model: &NativeModel,
    identity_json: &str,
    revision_json: &str,
    formal_revision: u64,
    multiple: bool,
) -> Expected {
    let native_prefix = source_prefix(model);
    let original = if multiple {
        source_multiple(model)
    } else {
        source(model)
    };
    let original_digest = format!("sha256:{:x}", Sha256::digest(original.as_bytes()));
    let record_start = MODEL_PREFIX.len();
    let record_end = record_start + MODEL_NODE.len();
    let record_end_column = 5 + MODEL_NODE.len();
    let location = format!(
        concat!(
            "{{\"identity\":{{\"owner\":{OWNER},\"key\":{{\"kind\":\"type\",\"name\":\"Node\"}}}},",
            "\"source\":{{\"start\":{{\"source\":{{\"document\":\"RuleModelSource\",\"revision\":1}},\"line\":10,\"column\":5,\"byte_offset\":{record_start}}},",
            "\"end\":{{\"source\":{{\"document\":\"RuleModelSource\",\"revision\":1}},\"line\":10,\"column\":{record_end_column},\"byte_offset\":{record_end}}}}}}}"
        ),
        OWNER = OWNER,
        record_start = record_start,
        record_end = record_end,
        record_end_column = record_end_column,
    );
    let mut clause_start = native_prefix.len();
    let mut canonical_clauses = Vec::new();
    let mut full_clauses = Vec::new();
    for (expression, (text, name, id, literal)) in [
        (CLAUSE, "Rule", "rule", "true"),
        (SECOND_CLAUSE, "Second", "second", "false"),
    ]
    .into_iter()
    .take(if multiple { 2 } else { 1 })
    .enumerate()
    {
        let clause_end = clause_start + text.len();
        let context_start = clause_start + format!("invariant {name} on M::").len();
        let context_end = context_start + "Node".len();
        let root_start = clause_start + format!("invariant {name} on M::Node at current {{ ").len();
        let root_end = root_start + literal.len();
        let clause = format!(
        concat!(
            "{{\"name\":\"{name}\",\"owner\":{AUTHOR},\"clause\":\"{id}\",\"kind\":\"invariant\",",
            "\"execution_point\":{{\"kind\":\"handler\",\"name\":\"validate\"}},\"span\":{{\"start\":{clause_start},\"end\":{clause_end}}},",
            "\"expression\":{expression},\"context\":{location},\"operation\":null,",
            "\"occurrences\":[{{\"expression\":null,\"span\":{{\"start\":{context_start},\"end\":{context_end}}},\"target\":{{\"kind\":\"formal\",\"declaration\":{location}}}}}],",
            "\"runtime\":{{\"context\":{location},\"context_observations\":[\"current\"],",
            "\"universes\":[{{\"model\":{OWNER},\"record\":\"Node\",\"universe\":\"nodes\",\"observations\":[\"current\"]}}],",
            "\"operation\":null,\"validate_frame\":false}}}}"
        ),
        AUTHOR = AUTHOR,
        OWNER = OWNER,
        name = name,
        id = id,
        expression = expression,
        clause_start = clause_start,
        clause_end = clause_end,
        context_start = context_start,
        context_end = context_end,
        location = location,
        );
        let clause_prefix = clause.strip_suffix('}').unwrap();
        full_clauses.push(format!(
            concat!(
                "{clause_prefix},\"projections\":[",
                "{{\"target\":\"native-reference/1\",\"status\":\"available\",\"cost_model\":\"native-ref-cost/1-draft\"}},",
                "{{\"target\":\"quire.contract.executable-projection/v1\",\"status\":\"unlowered\",\"code\":\"unsupported_construct\",\"span\":{{\"start\":{root_start},\"end\":{root_end}}}}}]}}"
            ),
            clause_prefix = clause_prefix,
            root_start = root_start,
            root_end = root_end,
        ));
        canonical_clauses.push(clause);
        clause_start = clause_end + 1;
    }
    // The complete admitted model is an input to the package under test. Only
    // that opaque input's JSON string quoting is delegated to existing Serde;
    // package fields/order, locations and control-label escaping are literals.
    let model_string = serde_json::to_string(
        std::str::from_utf8(model.artifact_bytes()).expect("admitted UTF-8 model"),
    )
    .unwrap();
    let prefix = format!(
        concat!(
            "{{\"format\":\"native-linked-package/1\",\"semantics\":{SEMANTICS},",
            "\"required_features\":[\"boolean\",\"object\",\"reference\",\"text\"],",
            "\"source\":{{\"authority\":\"agent-ix\",\"identity\":{identity_json},\"revision_namespace\":\"draft\",\"revision\":{revision_json},\"digest\":\"{original_digest}\",\"formal\":{{\"document\":\"PackageSource\",\"revision\":{formal_revision}}}}},",
            "\"models\":[{{\"alias\":\"M\",\"owner\":{OWNER},\"digest\":\"{model_digest}\",\"artifact\":{model_string}}}],\"clauses\":["
        ),
        SEMANTICS = SEMANTICS,
        OWNER = OWNER,
        identity_json = identity_json,
        revision_json = revision_json,
        original_digest = original_digest,
        formal_revision = formal_revision,
        model_digest = model.digest(),
        model_string = model_string,
    );
    let canonical = format!("{prefix}{}]}}", canonical_clauses.join(","));
    let mut hash = Sha256::new();
    hash.update(b"quire-spec-language\0quire.native.bound-package/v1\0linked-package\0");
    hash.update(canonical.as_bytes());
    let digest = format!("{:x}", hash.finalize());
    let artifact = format!(
        concat!(
            "{prefix}{clauses}],",
            "\"canonical_identity\":{{\"domain\":\"quire.native.bound-package\",\"version\":\"v1\",\"algorithm\":\"sha256\",\"digest\":\"{digest}\"}}}}"
        ),
        prefix = prefix,
        clauses = full_clauses.join(","),
        digest = digest,
    );
    Expected {
        canonical,
        artifact,
        digest,
    }
}
