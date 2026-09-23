// SPDX-License-Identifier: AGPL-3.0-or-later
//! The family-owned evaluation-time causes S6a returns in
//! [`crate::family::FamilyResult`] (FR-090, ADR-013 O-16, T-6), defined
//! beside the evaluator that raises them. Layer 5 `value::expression` holds
//! every family's evaluator (ADR-011 §6.1), so the module path does not
//! decide the owning family: [`ProtocolClauseSnapshot`] is `ProtocolClause`'s,
//! [`StateModelUndefined`] is `StateModel`'s, and
//! [`ModelQueryRefusal`] carries `StateModel`'s model-query refusal.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::{
    CatalogCode, CatalogCoded, UndefinedCoded, UndefinedReason, UndefinedRecord,
};
use quire_exact::ObjectReference;

use crate::check::WrongSnapshotCause;
use crate::model::key::hex;
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};

/// The `precondition-false` payload (`native-diagnostics.md`): the called
/// effective operation, the selected method's effective identity, the
/// receiver reference and the call locus. The call locus is the evaluator's
/// own [`crate::value::Evaluation::location`], not repeated here. Moved from
/// the now-deleted `value::outcome` (QSL-131 O2): not a kernel type
/// (ADR-013 T-6 -- FR-151 dispatch resolution is QSL `model`/`check`
/// vocabulary), and narrowed to `pub(crate)` since its only consumers are
/// [`StateModelUndefined::PreconditionFalse`] here and `Machine`'s
/// `DispatchGuard` in `value::expression::evaluate`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct PreconditionFailure {
    /// The called effective operation's unqualified member name.
    pub(crate) operation: String,
    /// The selected method's effective identity: the redefinition candidate
    /// the receiver's most-specific runtime type actually linked to.
    pub(crate) selected: String,
    /// The receiver reference the call was made on.
    pub(crate) receiver: ObjectReference,
}

/// ADR-013 T-6: `ProtocolClause`'s evaluation-time refusal cause, carrying
/// [`WrongSnapshotCause`]. `ProtocolClause` owns `Pre` (ADR-012 §4.3;
/// FR-091-AC-8). The evaluator raises only
/// [`WrongSnapshotCause::WrongAnchor`]; [`WrongSnapshotCause::ForbiddenPreRead`]
/// is a checking-time cause. `catalog_code()` covers both (O-17).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ProtocolClauseSnapshot(pub(crate) WrongSnapshotCause);

impl CatalogCoded for ProtocolClauseSnapshot {
    fn catalog_code(&self) -> CatalogCode {
        match self.0 {
            WrongSnapshotCause::WrongAnchor => CatalogCode::new("wrong_snapshot", "wrong-anchor"),
            WrongSnapshotCause::ForbiddenPreRead => {
                CatalogCode::new("wrong_snapshot", "forbidden-pre-read")
            }
        }
    }
}

/// FR-090-AC-8: the `StateModel` model-query refusal as S6a carries it, in
/// `FamilyResult::Refused`. Holds the refusal's cause and detail and no
/// native-v1 `qsl_foundation::diagnostic::Code` (ADR-013 R-09); its catalog
/// code is the cause's own [`ModelRefusalCause::catalog_code`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ModelQueryRefusal {
    /// The refusal's closed cause.
    pub(crate) cause: ModelRefusalCause,
    /// The refusal's human-readable detail.
    pub(crate) detail: String,
}

impl From<ModelRefusal> for ModelQueryRefusal {
    fn from(refusal: ModelRefusal) -> Self {
        let ModelRefusal {
            code: _,
            cause,
            detail,
        } = refusal;
        Self { cause, detail }
    }
}

impl CatalogCoded for ModelQueryRefusal {
    fn catalog_code(&self) -> CatalogCode {
        self.cause.catalog_code()
    }
}

/// The payload rendering of a member identity. A population member's
/// identity is a JSON string, so a well-formed identity is valid UTF-8 and
/// renders as itself. Bytes that are not UTF-8 render as
/// `identity bytes 0x<lowercase hex> (not UTF-8)`, the same rendering
/// `model::population::lookup` uses: two distinct malformed identities never
/// render alike, and a malformed identity never renders as a lossy decoding
/// that could equal a real member's.
pub(crate) fn identity_string(identity: &[u8]) -> String {
    match std::str::from_utf8(identity) {
        Ok(identity) => identity.to_owned(),
        Err(_) => format!("identity bytes 0x{} (not UTF-8)", hex(identity)),
    }
}

/// ADR-013 O-16: the `StateModel` family's evaluation-time undefined cause
/// (FR-090-AC-11, AC-12). ADR-012 assigns dispatch, dispatch preconditions
/// and population lookup to `StateModel` (§1, §3, §4.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum StateModelUndefined {
    /// FR-151 (TC-196 D06): a dispatched `receiver.member(args)` call's
    /// selected method's effective precondition evaluated to `false`.
    PreconditionFalse(PreconditionFailure),
    /// FR-153: a `lookup<T>(p, r) absent undefined` query's reference `r`
    /// names no member of the population bound to `p`.
    AbsentKey {
        /// The population binding `p` is bound to, rendered for the
        /// catalog payload.
        binding: String,
        /// The requested reference key `r`, rendered by
        /// [`identity_string`].
        key: String,
    },
}

impl UndefinedCoded for StateModelUndefined {
    fn undefined_record(&self) -> UndefinedRecord {
        match self {
            Self::PreconditionFalse(failure) => UndefinedRecord {
                reason: UndefinedReason::PreconditionFalse,
                fields: BTreeMap::from([
                    ("operation", failure.operation.clone()),
                    ("selected", failure.selected.clone()),
                    (
                        "receiver",
                        identity_string(failure.receiver.object().as_str().as_bytes()),
                    ),
                ]),
            },
            Self::AbsentKey { binding, key } => UndefinedRecord {
                reason: UndefinedReason::AbsentKey,
                fields: BTreeMap::from([("binding", binding.clone()), ("key", key.clone())]),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{identity_string, ModelQueryRefusal, ProtocolClauseSnapshot};
    use crate::check::WrongSnapshotCause;
    use crate::model::normalize::{ModelRefusal, ModelRefusalCause};
    use ix_trace_rs::trace;
    use qsl_foundation::diagnostic::{category_of, CatalogCode, CatalogCoded, Category, Code};

    /// TC-387 (FR-090-AC-6): each `WrongSnapshotCause` maps to
    /// `wrong_snapshot` with its own catalog cause tag, never through the
    /// native-v1 `Code`.
    #[trace("FR-090-AC-6", "TC-387")]
    #[test]
    fn protocol_clause_snapshot_catalog_code() {
        assert_eq!(
            ProtocolClauseSnapshot(WrongSnapshotCause::WrongAnchor).catalog_code(),
            CatalogCode::new("wrong_snapshot", "wrong-anchor")
        );
        assert_eq!(
            ProtocolClauseSnapshot(WrongSnapshotCause::ForbiddenPreRead).catalog_code(),
            CatalogCode::new("wrong_snapshot", "forbidden-pre-read")
        );
    }

    /// TC-386 (FR-090-AC-5), steps 1 and 2: F `diagnostic`'s category map
    /// files every code the `ProtocolClause` snapshot cause and
    /// `ModelRefusal` return under `Category::Refusal`. The model-query
    /// causes come from `model::refusal`'s `exhaustive_samples`, which
    /// fails to compile when a cause variant is missing from it.
    #[trace("FR-090-AC-5", "TC-386")]
    #[test]
    fn family_refusal_codes_map_to_category_refusal() {
        for cause in [
            WrongSnapshotCause::WrongAnchor,
            WrongSnapshotCause::ForbiddenPreRead,
        ] {
            let code = ProtocolClauseSnapshot(cause).catalog_code();
            assert_eq!(category_of(&code), Some(Category::Refusal), "{code}");
        }
        let samples = crate::model::refusal::tests::exhaustive_samples();
        assert!(!samples.is_empty());
        for cause in samples {
            let code = cause.catalog_code();
            assert_eq!(
                category_of(&code),
                Some(Category::Refusal),
                "{cause:?} -> {code}"
            );
            // The catalog code is a native code spelling, and the cause tag
            // is the cause's own.
            assert!(Code::from_code(code.code()).is_some(), "{code}");
            assert_eq!(code.cause(), cause.as_str());
        }
    }

    /// TC-389 (FR-090-AC-8) step 4: the refusal S6a carries holds no
    /// native-v1 `Code`. The destructuring names every field and has no
    /// `..`, so a field added to `ModelQueryRefusal` fails to compile here;
    /// the field types are pinned by the annotations.
    #[trace("FR-090-AC-8", "TC-389")]
    #[test]
    fn model_query_refusal_carries_no_native_code() {
        let refusal = ModelRefusal {
            code: Code::CardinalityOutOfBound,
            cause: ModelRefusalCause::AboveMaximum {
                selected: 2,
                maximum: 1,
            },
            detail: "two members above a maximum of one".to_owned(),
        };
        let expected = refusal.catalog_code();
        let carried = ModelQueryRefusal::from(refusal);
        assert_eq!(carried.catalog_code(), expected);
        assert_eq!(
            expected,
            CatalogCode::new("cardinality_out_of_bound", "above-maximum")
        );
        let ModelQueryRefusal { cause, detail } = carried;
        let _: ModelRefusalCause = cause;
        let _: String = detail;
    }

    /// FR-090-AC-12: a malformed identity renders as its hex bytes, so two distinct
    /// malformed identities never render alike.
    #[test]
    fn identity_string_renders_non_utf8_bytes_as_hex() {
        assert_eq!(identity_string(b"c9"), "c9");
        assert_eq!(
            identity_string(&[0xFF, 0xFE]),
            "identity bytes 0xfffe (not UTF-8)"
        );
        assert_ne!(identity_string(&[0xFF]), identity_string(&[0xFE]));
    }
}
