// SPDX-License-Identifier: AGPL-3.0-only
//! Real public compiler setup for the independently frozen package vectors.

use quire_contract_ir as ir;
use quire_spec_language::checking::{
    check, CheckBindings, CheckLimits, CheckedPackage, ClauseBinding,
};
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::{link_native, parse, Limits, LinkLimits, SourceIdentity};

pub(crate) struct Vector {
    pub name: &'static str,
    pub source: &'static str,
    pub canonical: &'static [u8],
    pub artifact: &'static [u8],
    pub digest: &'static str,
    pub identity: &'static str,
    pub revision: &'static str,
    pub formal_revision: u64,
    pub clauses: &'static [(&'static str, &'static str)],
}

pub(crate) fn cases() -> [Vector; 3] {
    [
        Vector {
            name: "minimal",
            source: include_str!("../fixtures/native-package/minimal.native"),
            canonical: include_bytes!("../fixtures/native-package/minimal.canonical.json"),
            artifact: include_bytes!("../fixtures/native-package/minimal.package.json"),
            digest: include_str!("../fixtures/native-package/minimal.sha256"),
            identity: "test:package",
            revision: "draft:1",
            formal_revision: 1,
            clauses: &[("Rule", "rule")],
        },
        Vector {
            name: "controls",
            source: include_str!("../fixtures/native-package/controls.native"),
            canonical: include_bytes!("../fixtures/native-package/controls.canonical.json"),
            artifact: include_bytes!("../fixtures/native-package/controls.package.json"),
            digest: include_str!("../fixtures/native-package/controls.sha256"),
            identity: "test:\"\\/\0\u{8}\u{c}\n\r\té🦀",
            revision: "draft:é\u{1}",
            formal_revision: 9_007_199_254_740_993,
            clauses: &[("Rule", "rule")],
        },
        Vector {
            name: "multiple",
            source: include_str!("../fixtures/native-package/multiple.native"),
            canonical: include_bytes!("../fixtures/native-package/multiple.canonical.json"),
            artifact: include_bytes!("../fixtures/native-package/multiple.package.json"),
            digest: include_str!("../fixtures/native-package/multiple.sha256"),
            identity: "test:multiple",
            revision: "draft:1",
            formal_revision: 1,
            clauses: &[("Rule", "rule"), ("Second", "second")],
        },
    ]
}

pub(crate) fn checked<'a>(vector: &Vector, models: &'a [NativeModel]) -> CheckedPackage<'a> {
    let unit = parse(
        SourceIdentity {
            identity: vector.identity.into(),
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
                name: ir::AnchorName::new("validate").unwrap(),
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
