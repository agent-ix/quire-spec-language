// SPDX-License-Identifier: AGPL-3.0-only
//! FR-012 AC-3: historical role/source/model correspondence, not a semantic matcher.
use crate::{
    error::{ensure, Code, Error, Result},
    input::{array, equal, field, number, text, Input},
    review::digest,
};
use quire_spec_language::{Source, SourceIdentity};
use serde_json::{json, Map, Value};
use std::{collections::BTreeMap, path::Path};

fn object(value: &Value) -> Result<&Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| Error::new(Code::InvalidFixture, "expected object"))
}

fn bytes<'a>(raws: &'a BTreeMap<String, Vec<u8>>, key: &str) -> Result<&'a [u8]> {
    raws.get(key)
        .map(Vec::as_slice)
        .ok_or_else(|| Error::new(Code::InvalidFixture, format!("missing artifact {key}")))
}

fn region(source: &Source, span: &Value, expected_digest: &Value) -> Result<()> {
    let start = number(field(span, "startByte")?)?;
    let end = number(field(span, "endByte")?)?;
    ensure(
        start < end,
        Code::InvalidFixture,
        "source region must be nonempty",
    )?;
    let raw = source
        .text()
        .as_bytes()
        .get(start..end)
        .ok_or_else(|| Error::new(Code::InvalidFixture, "source region out of bounds"))?;
    ensure(
        digest(raw) == text(expected_digest)?,
        Code::DigestMismatch,
        "source region bytes changed",
    )?;
    for (name, offset) in [("start", start), ("end", end)] {
        let position = source.position(offset).ok_or_else(|| {
            Error::new(
                Code::InvalidFixture,
                "source offset is not a scalar boundary",
            )
        })?;
        ensure(
            position.line == number(field(span, &format!("{name}Line"))?)?
                && position.column == number(field(span, &format!("{name}Column"))?)?,
            Code::InvalidFixture,
            "source coordinates mismatch",
        )?;
    }
    Ok(())
}

fn closure_fields(closure: &Value, lock: &Value) -> Result<()> {
    // Two equally malformed values are not correspondence. Nested producer
    // contents remain the producer contract's authority.
    text(field(lock, "fingerprint")?)?;
    object(field(lock, "canonicalization")?)?;
    array(field(lock, "packages")?)?;
    equal(
        closure,
        "establishedFingerprint",
        field(lock, "fingerprint")?,
    )?;
    equal(
        closure,
        "establishedCanonicalization",
        field(lock, "canonicalization")?,
    )?;
    equal(closure, "packages", field(lock, "packages")?)
}

/// Check every selected artifact and the original source/model/run compositions.
pub(crate) fn audit(root: &Path) -> Result<String> {
    let mut input = Input::new(root)?;
    let packet = input.json("role-compositions.json")?;
    equal(
        &packet,
        "fixtureVersion",
        &json!("agent-a-role-compositions/1-draft"),
    )?;
    let refs = field(&packet, "artifactRefs")?;
    let semantics = field(&packet, "semanticRefs")?;
    let locators = field(&packet, "artifactLocators")?;
    let mut raws = BTreeMap::new();
    for (key, reference) in object(refs)? {
        let raw = input.read(text(field(locators, key)?)?)?;
        equal(reference, "refVersion", &json!("ix.artifact-ref/2-draft"))?;
        text(field(reference, "identity")?)?;
        let revision = field(reference, "revision")?;
        text(field(revision, "namespace")?)?;
        text(field(revision, "value")?)?;
        let expected = text(field(reference, "digest")?)?;
        ensure(
            digest(&raw) == expected,
            Code::DigestMismatch,
            format!("role artifact {key} changed"),
        )?;
        let mut changed = raw.clone();
        changed.push(b' ');
        ensure(
            digest(&changed) != expected,
            Code::InvalidFixture,
            "changed-byte control failed",
        )?;
        raws.insert(key.clone(), raw);
    }
    let profile_digest = digest(&input.profile()?);
    for (key, source_key) in [
        ("property-current", "current-source"),
        ("property-post", "operation-source"),
    ] {
        let payload = input.decode(bytes(&raws, key)?)?;
        let source = field(&payload, "source")?;
        equal(source, "artifact", field(refs, source_key)?)?;
        ensure(
            number(field(field(source, "irRevisionMapping")?, "revision")?)? > 0,
            Code::InvalidFixture,
            "source IR revision must be positive",
        )?;
        let original = Source::read(
            SourceIdentity {
                identity: text(field(field(refs, source_key)?, "identity")?)?.into(),
                revision: "selected-review-region".into(),
            },
            source_key,
            bytes(&raws, source_key)?,
            quire_spec_language::source::MAX_SOURCE_BYTES,
        )
        .map_err(|e| {
            Error::new(
                if e.is_incomplete() {
                    Code::ResourceExhausted
                } else {
                    Code::InvalidFixture
                },
                e.message,
            )
        })?;
        region(
            &original,
            field(source, "clauseSpan")?,
            field(source, "clauseDigest")?,
        )?;
        region(
            &original,
            field(source, "expressionSpan")?,
            field(source, "expressionDigest")?,
        )?;
        let semantic = field(semantics, key)?;
        equal(semantic, "artifact", field(refs, key)?)?;
        let features = array(field(semantic, "requiredFeatures")?)?
            .iter()
            .map(text)
            .collect::<Result<Vec<_>>>()?;
        ensure(
            features.windows(2).all(|pair| pair[0] < pair[1]),
            Code::InvalidFixture,
            "historical feature list must be sorted and unique",
        )?;
        equal(
            field(semantic, "semanticProfile")?,
            "definitionDigest",
            &json!(profile_digest),
        )?;
    }
    let ir = input.decode(bytes(&raws, "model")?)?;
    let lock = input.decode(bytes(&raws, "lock")?)?;
    let manifest = input.decode(bytes(&raws, "manifest")?)?;
    equal(&ir, "contractVersion", &json!("1.1.0"))?;
    let package = field(&ir, "package")?;
    let manifest_package = field(&manifest, "package")?;
    let identity = text(field(package, "identity")?)?;
    ensure(
        identity == text(field(manifest_package, "identity")?)?
            && identity == text(field(&lock, "rootPackage")?)?,
        Code::IdentityMismatch,
        "model package identity mismatch",
    )?;
    ensure(
        text(field(package, "version")?)? == text(field(manifest_package, "version")?)?,
        Code::InvalidFixture,
        "model package version mismatch",
    )?;
    equal(
        package,
        "manifestDigest",
        &json!(digest(bytes(&raws, "manifest")?)),
    )?;
    equal(package, "lockDigest", &json!(digest(bytes(&raws, "lock")?)))?;
    let closure = input.decode(bytes(&raws, "closure")?)?;
    closure_fields(&closure, &lock)?;
    equal(
        field(&closure, "nativeQualification")?,
        "state",
        &json!("unavailable"),
    )?;
    let output = input.decode(bytes(&raws, "native-output")?)?;
    equal(&output, "status", &json!("parsed"))?;
    let source = field(&output, "source")?;
    equal(
        source,
        "digest",
        &json!(digest(bytes(&raws, "native-source")?)),
    )?;
    let native_ref = field(refs, "native-source")?;
    equal(source, "identity", field(native_ref, "identity")?)?;
    equal(
        field(native_ref, "revision")?,
        "namespace",
        &json!("quire-spec.cli-source-label"),
    )?;
    equal(
        source,
        "revision",
        field(field(native_ref, "revision")?, "value")?,
    )?;
    let run = input.decode(bytes(&raws, "run")?)?;
    equal(&run, "runIdentity", field(field(refs, "run")?, "identity")?)?;
    equal(&run, "observedExitCode", &json!(0))?;
    equal(&run, "logicalOutcome", &json!("not-evaluated"))?;
    for (name, key) in [
        ("source", "native-source"),
        ("nativeArtifact", "native-output"),
        ("scopeExample", "environment"),
    ] {
        equal(&run, name, field(refs, key)?)?;
    }
    let arguments = array(field(&run, "arguments")?)?;
    ensure(
        arguments.get(1) == Some(field(source, "identity")?)
            && arguments.get(2) == Some(field(source, "revision")?),
        Code::InvalidFixture,
        "recorded native arguments mismatch",
    )?;
    for case in array(field(&packet, "cases")?)? {
        for selected in object(field(case, "roles")?)?.values() {
            ensure(
                object(refs)?
                    .values()
                    .chain(object(semantics)?.values())
                    .any(|reference| reference == selected),
                Code::InvalidFixture,
                "role selects an unlisted reference",
            )?;
        }
    }
    Ok(format!("passed: {} exact artifacts and changed-byte controls; four source regions/coordinates; model wrapper consistency; historical syntax output; no semantic matcher or evaluator executed",raws.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    #[trace("TC-003", "FR-012-AC-3", "FR-012-AC-6")]
    #[test]
    fn equal_but_malformed_closure_fields_refuse() {
        let lock = json!({"fingerprint":"sha256:selected","canonicalization":{},"packages":[]});
        let closure = json!({"establishedFingerprint":"sha256:selected","establishedCanonicalization":{},"packages":[]});
        closure_fields(&closure, &lock).unwrap();
        for (lock_name, closure_name) in [
            ("fingerprint", "establishedFingerprint"),
            ("canonicalization", "establishedCanonicalization"),
            ("packages", "packages"),
        ] {
            for invalid in [Value::Null, json!(42), json!(true)] {
                let mut bad_lock = lock.clone();
                let mut bad_closure = closure.clone();
                bad_lock[lock_name] = invalid.clone();
                bad_closure[closure_name] = invalid;
                assert_eq!(
                    closure_fields(&bad_closure, &bad_lock).unwrap_err().code,
                    Code::InvalidFixture
                );
            }
        }
    }
    #[trace("TC-003", "FR-012-AC-3", "FR-012-AC-6")]
    #[test]
    fn original_scalar_coordinates_and_mutations() {
        let source = Source::read(
            SourceIdentity {
                identity: "test:unicode".into(),
                revision: "1".into(),
            },
            "original",
            "aé\nxyz".as_bytes(),
            100,
        )
        .unwrap();
        let mut span = json!({"startByte":1,"endByte":3,"startLine":1,"startColumn":2,"endLine":1,"endColumn":3});
        let expected = json!(digest("é".as_bytes()));
        region(&source, &span, &expected).unwrap();
        span["endColumn"] = json!(4);
        assert_eq!(
            region(&source, &span, &expected).unwrap_err().code,
            Code::InvalidFixture
        );
        span["endColumn"] = json!(3);
        assert_eq!(
            region(&source, &span, &json!(digest(b"x")))
                .unwrap_err()
                .code,
            Code::DigestMismatch
        );
        span["startByte"] = json!(2);
        assert!(region(&source, &span, &expected).is_err());
        span["endByte"] = json!(100);
        assert_eq!(
            region(&source, &span, &expected).unwrap_err().code,
            Code::InvalidFixture
        );
    }
}
