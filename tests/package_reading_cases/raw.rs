// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-020: exact integer spelling and raw duplicate members before any host map.
use super::*;
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize, Serializer,
};
use sha2::{Digest, Sha256};

#[test]
#[trace("TC-083", "FR-020-AC-2")]
fn wire_inventory_cardinality_is_closed_before_rebinding() {
    let models = [model()];
    let [vector, _, _] = package_vector_setup::cases();
    let bindings = package_vector_setup::checked(&vector, &models)
        .bindings()
        .clone();
    let original: Value = serde_json::from_slice(vector.artifact).unwrap();
    for (field, maximum) in [("models", 64), ("clauses", 256)] {
        for count in [maximum, maximum + 1] {
            let mut changed = original.clone();
            let first = changed[field][0].clone();
            changed[field] = json!(vec![first; count]);
            let error = read(&changed, bindings.clone(), &models).unwrap_err();
            if count == maximum {
                assert!(matches!(
                    error.stage,
                    PackageStage::Rebind | PackageStage::Compare
                ));
            } else {
                assert_eq!(
                    (error.code, error.stage),
                    (Code::InvalidPackage, PackageStage::Decode)
                );
                assert!(error.usage.decode.is_some());
            }
        }
    }
}

#[test]
#[trace("TC-091", "FR-021-AC-5")]
fn raw_ir_and_jcs_digests_cannot_replace_native_static_identity() {
    let models = [model()];
    let [vector, _, _] = package_vector_setup::cases();
    let bindings = package_vector_setup::checked(&vector, &models)
        .bindings()
        .clone();
    let original: Value = serde_json::from_slice(vector.artifact).unwrap();
    let declaration = models[0]
        .environment()
        .canonical_declaration(ir::CanonicalProfile::V1)
        .unwrap();
    // This independent JCS document contains only one integer and one ASCII key,
    // so its canonical bytes are fixed without another canonicalizer.
    for digest in [
        format!("{:x}", Sha256::digest(vector.artifact)),
        declaration.digest().to_string(),
        format!("{:x}", Sha256::digest(br#"{"a":1}"#)),
    ] {
        assert_ne!(digest, vector.digest.trim_end());
        let mut changed = original.clone();
        changed["canonical_identity"]["digest"] = json!(digest);
        let error = read(&changed, bindings.clone(), &models).unwrap_err();
        assert_eq!(
            (error.code, error.stage),
            (Code::InvalidPackage, PackageStage::Compare)
        );
        assert!(error.usage.checking.is_some());
    }
}

struct Duplicate<'a> {
    value: &'a Value,
    target: &'a str,
    here: String,
}
impl Serialize for Duplicate<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.value {
            Value::Object(object) => {
                let duplicate = self.here == self.target;
                let mut out =
                    serializer.serialize_map(Some(object.len() + usize::from(duplicate)))?;
                for (key, value) in object {
                    out.serialize_entry(
                        key,
                        &Duplicate {
                            value,
                            target: self.target,
                            here: format!("{}/{key}", self.here),
                        },
                    )?;
                }
                if duplicate {
                    let (key, value) = object
                        .iter()
                        .next()
                        .expect("nonempty closed fixture record");
                    out.serialize_entry(key, value)?;
                }
                out.end()
            }
            Value::Array(array) => {
                let mut out = serializer.serialize_seq(Some(array.len()))?;
                for (index, value) in array.iter().enumerate() {
                    out.serialize_element(&Duplicate {
                        value,
                        target: self.target,
                        here: format!("{}/{index}", self.here),
                    })?;
                }
                out.end()
            }
            value => value.serialize(serializer),
        }
    }
}

#[test]
#[trace("TC-083", "FR-020-AC-2")]
fn every_closed_record_rejects_raw_duplicate_members_before_typed_decoding() {
    let models = [native_rule_model::parts().model()];
    let package = rule(&models, "post", "result");
    let original: Value = serde_json::from_slice(package.bytes()).unwrap();
    let bindings = package.checked().bindings().clone();
    let mut paths = Vec::new();
    objects(&original, String::new(), &mut paths);
    for target in paths {
        let bytes = serde_json::to_vec(&Duplicate {
            value: &original,
            target: &target,
            here: String::new(),
        })
        .unwrap();
        let error = NativePackage::read_verified(
            &bytes,
            NativePackageRef::new(ByteDigest::of(&bytes)),
            bindings.clone(),
            &models,
            &PackageSupport::default(),
            PackageReadLimits::default(),
        )
        .err()
        .unwrap_or_else(|| panic!("accepted duplicate at {target}"));
        assert_eq!(
            (error.code, error.stage),
            (Code::InvalidPackage, PackageStage::Decode)
        );
        assert!(error.usage.recognition.is_some());
        assert!(error.usage.decode.is_none());
    }
}

#[test]
#[trace("TC-083", "TC-082", "FR-020-AC-2", "FR-020-AC-10")]
fn full_u64_revision_survives_and_fraction_exponent_or_overflow_spelling_refuses() {
    let models = [model()];
    let [mut vector, _, _] = package_vector_setup::cases();
    vector.formal_revision = u64::MAX;
    let package = NativePackage::new(
        package_vector_setup::checked(&vector, &models),
        PackageLimits::default(),
    )
    .unwrap();
    let bindings = package.checked().bindings().clone();
    let success = NativePackage::read_verified(
        package.bytes(),
        package.reference(),
        bindings.clone(),
        &models,
        &PackageSupport::default(),
        PackageReadLimits::default(),
    )
    .unwrap();
    assert_eq!(
        success
            .checked()
            .bindings()
            .source
            .identity()
            .revision()
            .get(),
        u64::MAX
    );
    let text = std::str::from_utf8(package.bytes()).unwrap();
    let marker = "\"revision\":18446744073709551615";
    assert_eq!(
        text.matches(marker).count(),
        1,
        "unique raw numeric test occurrence"
    );
    for number in ["1.0", "1e0", "-0", "18446744073709551616", "0"] {
        let raw = text.replace(marker, &format!("\"revision\":{number}"));
        let error = NativePackage::read_verified(
            raw.as_bytes(),
            NativePackageRef::new(ByteDigest::of(raw.as_bytes())),
            bindings.clone(),
            &models,
            &PackageSupport::default(),
            PackageReadLimits::default(),
        )
        .err()
        .unwrap_or_else(|| panic!("accepted invalid revision spelling {number}"));
        assert_eq!(
            (error.code, error.stage),
            (Code::InvalidPackage, PackageStage::Decode),
            "{number}"
        );
        assert!(error.usage.decode.is_some());
        assert!(error.usage.derive.is_none());
    }
}
