// SPDX-License-Identifier: AGPL-3.0-only
//! FR-012 AC-4/5: selected producer bytes and real native syntax, not evaluation.
use crate::{
    error::{ensure, Code, Error, Result},
    input::{array, equal, field, text, Input},
    review::digest,
};
use quire_spec_language::{parse, Code as NativeCode, Limits, SourceIdentity};
use serde_json::json;
use std::{collections::BTreeSet, path::Path};

const PRODUCER: &str = "3b75e01c652ba00bb07c352ff5467419401e792b";

/// Verify the selected producer checkpoint bytes without executing its producer.
pub(crate) fn model(root: &Path) -> Result<String> {
    let mut input = Input::new(root)?;
    let provenance = input.json("model-output/provenance.json")?;
    equal(
        &provenance,
        "fixtureVersion",
        &json!("agent-a-model-production/1-draft"),
    )?;
    equal(
        field(&provenance, "producer")?,
        "revision",
        &json!(PRODUCER),
    )?;
    let artifacts = array(field(&provenance, "artifacts")?)?;
    ensure(
        artifacts.len() == 5,
        Code::InvalidFixture,
        "expected five selected checkpoint artifacts",
    )?;
    let mut paths = BTreeSet::new();
    for artifact in artifacts {
        let path = text(field(artifact, "path")?)?;
        ensure(
            paths.insert(path),
            Code::InvalidFixture,
            "duplicate checkpoint artifact",
        )?;
        let raw = input.read(path)?;
        ensure(
            digest(&raw) == text(field(artifact, "digest")?)?,
            Code::DigestMismatch,
            format!("checkpoint bytes changed: {path}"),
        )?;
    }
    Ok("passed: 5 producer checkpoint byte digests and exact producer pin; historical evidence only; no producer or evaluator executed".into())
}

/// Execute actual native syntax checks over the selected FS03 authored cases.
pub(crate) fn syntax(root: &Path) -> Result<String> {
    let mut input = Input::new(root)?;
    let fixture = input.json("fixtures/typing-cases.json")?;
    check_syntax_profile(&mut input, &fixture)?;
    let mut ids = BTreeSet::new();
    let mut report = SyntaxReport::default();
    for case in array(field(&fixture, "cases")?)? {
        let id = text(field(case, "id")?)?;
        ensure(
            ids.insert(id),
            Code::InvalidFixture,
            "duplicate syntax case ID",
        )?;
        let invocation = SyntaxInvocation::read(case, id)?;
        let observed = invocation.parse();
        // The historical contract reads expected syntax after native parsing.
        // Preserve that order, including compound-invalid inputs.
        match check_syntax_outcome(case, id, observed)? {
            SyntaxOutcome::Parsed => report.parsed += 1,
            SyntaxOutcome::Unsupported => report.unsupported += 1,
        }
    }
    Ok(report.to_string())
}

fn check_syntax_profile(input: &mut Input, fixture: &serde_json::Value) -> Result<()> {
    equal(
        fixture,
        "fixtureVersion",
        &json!("agent-a-typing-cases/1-draft"),
    )?;
    equal(
        field(fixture, "modelBinding")?,
        "state",
        &json!("unavailable"),
    )?;
    for (path, expected) in [
        (
            "state-semantics.md",
            field(field(fixture, "ruleContract")?, "digest")?,
        ),
        (
            "profile.md",
            field(field(fixture, "baseProfile")?, "definitionDigest")?,
        ),
    ] {
        ensure(
            digest(&input.read(path)?) == text(expected)?,
            Code::DigestMismatch,
            format!("selected definition changed: {path}"),
        )?;
    }
    Ok(())
}

struct SyntaxInvocation<'a> {
    id: &'a str,
    anchor: &'static str,
    expression: &'a str,
}

impl<'a> SyntaxInvocation<'a> {
    fn read(case: &'a serde_json::Value, id: &'a str) -> Result<Self> {
        let anchor = match text(field(case, "anchor")?)? {
            "post" => "post Case on M::Node::step",
            "current" => "invariant Case on M::Node at current",
            _ => {
                return Err(Error::new(
                    Code::InvalidFixture,
                    "unknown syntax case anchor",
                ))
            }
        };
        Ok(Self {
            id,
            anchor,
            expression: text(field(case, "expression")?)?,
        })
    }

    fn parse(
        &self,
    ) -> std::result::Result<quire_spec_language::ParsedUnit, Box<quire_spec_language::Diagnostic>>
    {
        let anchor = self.anchor;
        let expression = self.expression;
        let source = format!("language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"example/rule-tests\" version \"0.0.0-fixture\" digest \"unresolved-model-package\";\n{anchor} {{ {expression} }}\n");
        parse(
            SourceIdentity {
                identity: format!("fs03:{}", self.id),
                revision: "fixture:1".into(),
            },
            "selected-rule.native",
            source.as_bytes(),
            Limits::default(),
        )
    }
}

enum SyntaxOutcome {
    Parsed,
    Unsupported,
}

fn check_syntax_outcome(
    case: &serde_json::Value,
    id: &str,
    observed: std::result::Result<
        quire_spec_language::ParsedUnit,
        Box<quire_spec_language::Diagnostic>,
    >,
) -> Result<SyntaxOutcome> {
    let expected = text(field(field(case, "expected")?, "syntax")?)?;
    match (expected, observed) {
        ("parsed", Ok(_)) => Ok(SyntaxOutcome::Parsed),
        ("unsupported", Err(error)) if error.code == NativeCode::UnsupportedConstruct => {
            Ok(SyntaxOutcome::Unsupported)
        }
        (_, Err(error)) if error.is_incomplete() => Err(Error::new(
            Code::ResourceExhausted,
            format!("native syntax {id}: {}", error.message),
        )),
        (_, Err(error)) => Err(Error::new(
            Code::InvalidFixture,
            format!("native syntax {id}: {}", error.code.as_str()),
        )),
        (_, Ok(_)) => Err(Error::new(
            Code::InvalidFixture,
            format!("unexpected parsed result for {id}"),
        )),
    }
}

#[derive(Default)]
struct SyntaxReport {
    parsed: usize,
    unsupported: usize,
}

impl std::fmt::Display for SyntaxReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "passed syntax only: {} parsed expressions; {} unsupported refusal; case IDs and rule/profile digests checked; no typechecker or evaluator executed", self.parsed, self.unsupported)
    }
}
