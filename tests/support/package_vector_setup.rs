// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real public compiler setup for the independently composed package vectors.

use qsl_foundation::SourceIdentity;
use quire_contract_model as ir;
use quire_spec_language::checking::{
    check, CheckBindings, CheckLimits, CheckedPackage, ClauseBinding,
};
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::{link_native, parse, Limits, LinkLimits};

#[path = "../package_construction_cases/vectors.rs"]
pub(crate) mod vectors;

// `vectors` reads the header and the model setup from its parent.
const HEADER: &str = include_str!("../fixtures/native-package/header.native");
use super::native_rule_model;

/// One positive package vector: its native source, the expected canonical
/// and package bytes and the expected identity, each composed at test time
/// by the independent recipe in `vectors` from the offered `model`, so no
/// digest of the model, the source or the package is frozen in a file.
pub(crate) struct Vector {
    pub name: &'static str,
    pub source: String,
    pub canonical: Vec<u8>,
    pub artifact: Vec<u8>,
    pub digest: String,
    pub identity: &'static str,
    pub revision: &'static str,
    pub formal_revision: u64,
    pub clauses: &'static [(&'static str, &'static str)],
}

/// The `minimal`, `controls` and `multiple` vectors over `model`.
pub(crate) fn cases(model: &NativeModel) -> [Vector; 3] {
    let vector = |name,
                  source: String,
                  expected: vectors::Expected,
                  identity,
                  revision,
                  formal_revision,
                  clauses| Vector {
        name,
        source,
        canonical: expected.canonical.into_bytes(),
        artifact: expected.artifact.into_bytes(),
        digest: expected.digest,
        identity,
        revision,
        formal_revision,
        clauses,
    };
    [
        vector(
            "minimal",
            vectors::source(model),
            vectors::expected(model, r#""test:package""#, r#""draft:1""#, 1),
            "test:package",
            "draft:1",
            1,
            &[("Rule", "rule")],
        ),
        vector(
            "controls",
            vectors::source(model),
            vectors::expected(
                model,
                r#""test:\"\\/\u0000\b\f\n\r\té🦀""#,
                r#""draft:é\u0001""#,
                9_007_199_254_740_992,
            ),
            "test:\"\\/\0\u{8}\u{c}\n\r\té🦀",
            "draft:é\u{1}",
            9_007_199_254_740_992,
            &[("Rule", "rule")],
        ),
        vector(
            "multiple",
            vectors::source_multiple(model),
            vectors::expected_multiple(model, r#""test:multiple""#, r#""draft:1""#, 1),
            "test:multiple",
            "draft:1",
            1,
            &[("Rule", "rule"), ("Second", "second")],
        ),
    ]
}

pub(crate) fn checked<'a>(vector: &Vector, models: &'a [NativeModel]) -> CheckedPackage<'a> {
    let unit = parse(
        SourceIdentity {
            authority: "agent-ix".into(),
            identity: vector.identity.into(),
            revision_namespace: "draft".into(),
            revision: vector.revision.into(),
        },
        vector.name,
        vector.source.as_bytes(),
        Limits::default(),
    )
    .expect("fixed source must pass the actual parser");
    let source = FormalSource::new(
        unit.source().clone(),
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("PackageSource").unwrap(),
            ir::SourceRevision::new(vector.formal_revision).unwrap(),
        ),
    );
    let linked = link_native(unit, models, LinkLimits::default())
        .expect("fixed imports must resolve against the actual offered models");
    let clauses = vector
        .clauses
        .iter()
        .map(|(name, clause)| ClauseBinding {
            name: (*name).into(),
            requirement: ir::RequirementRef::parse("example/package-rules", "Rule", 2).unwrap(),
            clause: ir::ClauseId::new(*clause).unwrap(),
            execution_point: ir::ExecutionPoint::Handler {
                name: ir::AnchorName::new("validate").unwrap().into(),
            },
        })
        .collect();
    check(
        linked,
        CheckBindings { source, clauses },
        CheckLimits::default(),
    )
    .expect("fixed source must pass the actual checker with external bindings")
}
