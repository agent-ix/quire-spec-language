// SPDX-License-Identifier: AGPL-3.0-or-later
//! The family-owned evaluation-time causes S6a returns in
//! [`quire_eval::FamilyResult`] (FR-090, ADR-013 O-16, T-6), defined
//! beside the evaluator that raises them. Layer 5 `value::expression` holds
//! every family's evaluator (ADR-011 §6.1), so the module path does not
//! decide the owning family: [`ProtocolClauseSnapshot`] is `ProtocolClause`'s,
//! [`ModelQueryRefusal`] carries `StateModel`'s model-query refusal. The
//! `Value`-family causes both evaluators raise live in `quire-eval`
//! (FR-403).

use std::collections::BTreeMap;

use quire_exact::ObjectReference;
use quire_semantic_value::catalog::{render_identity, CatalogCode, CatalogCoded};

use qsl_semantics::model::normalize::{ModelRefusal, ModelRefusalCause};
use qsl_semantics::model::observation::AdmissionRecord;
use quire_checked::key::DeclarationKey;
use quire_semantic_value::call::WrongSnapshotCause;

/// ADR-013 T-6: `ProtocolClause`'s evaluation-time refusal cause, carrying
/// [`WrongSnapshotCause`]. `ProtocolClause` owns `Pre` (ADR-012 §4.3;
/// FR-091-AC-8). The evaluator raises only
/// [`WrongSnapshotCause::WrongAnchor`]; [`WrongSnapshotCause::ForbiddenPreRead`]
/// is a checking-time cause. `catalog_code()` covers both (O-17).
///
/// Each variant carries the payload the catalog requires for its cause
/// (FR-096), read by [`CatalogCoded::catalog_fields`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ProtocolClauseSnapshot {
    /// `wrong-anchor`: the required and the supplied anchor selection.
    WrongAnchor {
        /// The required anchor selection.
        required: &'static str,
        /// The supplied anchor selection.
        supplied: &'static str,
    },
    /// `forbidden-pre-read`: the exact prohibited read.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "a checking-time cause: the evaluator raises only wrong-anchor (see the type doc); the checker's own refusal is `CheckCause::WrongSnapshot`"
        )
    )]
    ForbiddenPreRead {
        /// The prohibited read, rendered.
        read: String,
    },
}

impl ProtocolClauseSnapshot {
    /// The closed cause this payload belongs to.
    pub(crate) fn cause(&self) -> WrongSnapshotCause {
        match self {
            Self::WrongAnchor { .. } => WrongSnapshotCause::WrongAnchor,
            Self::ForbiddenPreRead { .. } => WrongSnapshotCause::ForbiddenPreRead,
        }
    }
}

impl CatalogCoded for ProtocolClauseSnapshot {
    /// FR-063-AC-7: `#[deny(...)]` closes the `_ => unsupported(...)` escape
    /// hatch the seam probe alone cannot see.
    #[deny(clippy::wildcard_enum_match_arm)]
    fn catalog_code(&self) -> CatalogCode {
        match self.cause() {
            WrongSnapshotCause::WrongAnchor => CatalogCode::new("wrong_snapshot", "wrong-anchor"),
            WrongSnapshotCause::ForbiddenPreRead => {
                CatalogCode::new("wrong_snapshot", "forbidden-pre-read")
            }
            // FR-063: no arm for the probe variant under `--cfg seam_probe`
            // alone (`E0004`, this seam's evidence).
            //
            // The arm below exists only in the probe's build of the crates
            // above `qsl-eval` (`--cfg seam_probe_eval_downstream`):
            // `qsl-replay` depends on this crate, so it must compile there
            // for the root crate's own seams to be reached at all.
            #[cfg(seam_probe_eval_downstream)]
            WrongSnapshotCause::__SeamProbe => CatalogCode::new("__seam_probe__", "__seam_probe__"),
        }
    }

    /// FR-096's key table: `wrong-anchor` has `required` and `supplied`,
    /// `forbidden-pre-read` has `read`.
    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        Some(match self {
            Self::WrongAnchor { required, supplied } => BTreeMap::from([
                ("required", (*required).to_owned()),
                ("supplied", (*supplied).to_owned()),
            ]),
            Self::ForbiddenPreRead { read } => BTreeMap::from([("read", read.clone())]),
        })
    }
}

/// `ProtocolClause`'s own refusal for a construct FR-104 admits into a
/// checked clause body but FR-107 does not specify evaluation for.
///
/// FR-104 ("Observations of reads") gives `allInstances` and `lookup`, like
/// every other model read, one observation, so the checker admits either
/// inside a state clause whose operation declares a `Population<T>[N]`
/// parameter. FR-107's own "Behavior" section (`FR-107-evaluate-state-
/// clauses-at-s6a.md:54-109`) states `reaches`, field reads, `self`,
/// `result`, parameters and `pre(e)` in full, and never once names
/// `allInstances` or `lookup` -- FR-107 does not cover them. Reusing the
/// `Value` family's own population machinery
/// (`Machine::resolve_population`/`select_anchor`) is not an option: it
/// resolves a `Value::Population` argument's *recorded* `PopulationBinding`
/// (`ObjectEnvironment::with_population`), and FR-106's own raw value
/// grammar ("Document forms") has no `population` tag among its six -- a
/// `Population<T>[N]` operation parameter can never be admitted through an
/// invocation document, so no such binding is ever recorded for a
/// `ProtocolClause` evaluation's `ObjectEnvironment`. Reaching either node
/// while evaluating a `ProtocolClause` therefore refuses here, typed and
/// tested, rather than falling through to `resolve_population`'s own
/// internal-fault path (reserved for a broken invariant, ADR-013 T-4, not
/// this scoped gap).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ProtocolClauseUnsupported {
    /// `allInstances<T>(p)`.
    AllInstances,
    /// `lookup<T>(p, r) absent m`.
    Lookup,
}

impl ProtocolClauseUnsupported {
    fn construct(self) -> &'static str {
        match self {
            Self::AllInstances => "allInstances",
            Self::Lookup => "lookup",
        }
    }
}

impl CatalogCoded for ProtocolClauseUnsupported {
    /// `unknown_required_feature`/`unsupported-feature`, naming the
    /// construct (SR-750 FND-012): the same code/cause pair FR-106's own
    /// check 6.2 already uses for an unsupported feature at a construct
    /// (`document.rs`'s set/bag/ordered-set field refusal), not the
    /// invented `unsupported_construct`/`protocol-clause-population-read`
    /// pairing this replaces, which named no registered cause.
    fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("unknown_required_feature", "unsupported-feature")
    }

    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        Some(BTreeMap::from([("construct", self.construct().to_owned())]))
    }
}

/// FR-364 "Runtime navigation reads only the extent of its relationship":
/// the refusals a navigation decides itself, each retaining the relationship
/// by its declaration key and the receiver or target reference by its object
/// identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NavigationRefusal {
    /// `incomplete_population`/`incomplete-scope`: the relationship's extent
    /// is unlisted or incomplete, so it states nothing about the receiver's
    /// links.
    ExtentIncomplete {
        /// The navigated relationship.
        relationship: DeclarationKey,
        /// The receiver reference.
        receiver: ObjectReference,
    },
    /// `incomplete_population`/`incomplete-scope`: a selected target is
    /// missing and the manifest cannot decide whether it is dangling.
    TargetUndecidable {
        /// The navigated relationship.
        relationship: DeclarationKey,
        /// The first selected reference that is not present.
        reference: ObjectReference,
    },
    /// `dangling_reference`/`absent-target-in-complete-population`: a
    /// selected target is missing from populations the manifest closes.
    TargetDangling {
        /// The navigated relationship.
        relationship: DeclarationKey,
        /// The first selected reference that is not present.
        reference: ObjectReference,
    },
}

impl CatalogCoded for NavigationRefusal {
    fn catalog_code(&self) -> CatalogCode {
        match self {
            Self::ExtentIncomplete { .. } | Self::TargetUndecidable { .. } => {
                CatalogCode::new("incomplete_population", "incomplete-scope")
            }
            Self::TargetDangling { .. } => {
                CatalogCode::new("dangling_reference", "absent-target-in-complete-population")
            }
        }
    }

    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        let (relationship, name, object) = match self {
            Self::ExtentIncomplete {
                relationship,
                receiver,
            } => (relationship, "receiver", receiver),
            Self::TargetUndecidable {
                relationship,
                reference,
            }
            | Self::TargetDangling {
                relationship,
                reference,
            } => (relationship, "reference", reference),
        };
        Some(BTreeMap::from([
            ("relationship", relationship.node.clone()),
            (name, render_identity(object.object().as_str().as_bytes())),
        ]))
    }
}

/// FR-391 "Evaluation": a clause or a `holds` atom names a population the
/// observation's snapshot does not list `complete`. FR-106 requires every
/// population a clause names, so the read stops `incomplete_population`/
/// `incomplete-scope` naming it, whichever path admitted the snapshot.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct NamedPopulationIncomplete {
    /// The population's declaration identity.
    pub(crate) population: String,
}

impl CatalogCoded for NamedPopulationIncomplete {
    fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("incomplete_population", "incomplete-scope")
    }

    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        Some(BTreeMap::from([("population", self.population.clone())]))
    }
}

/// A read of a history field (FR-138) that holds no value at the instant the
/// read observes: the update of the step that reached the instant failed, so
/// the value is not known and a row must not read a stand-in. The read is
/// undetermined, naming the field.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct HistoryUnresolved {
    /// The history field's name.
    pub(crate) field: String,
}

impl CatalogCoded for HistoryUnresolved {
    /// `missing_declaration`/`missing-name`, naming the history field.
    fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("missing_declaration", "missing-name")
    }

    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        Some(BTreeMap::from([("history", self.field.clone())]))
    }
}

/// FR-115: `ProtocolClause`'s refusal when a `Frame` run's check 11 cannot
/// evaluate the frame over the admitted invocation -- a declared delta that
/// disagrees (`population_delta_mismatch`/`delta-disagreement`), or the
/// conformance walk's ceiling. It carries FR-106's own admission record for
/// the finding, whose code and cause are the catalog's. A change outside the
/// frame is never this: it is a violation with a witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProtocolClauseFrameRefusal {
    /// The finding's admission record.
    pub(crate) record: AdmissionRecord,
}

impl CatalogCoded for ProtocolClauseFrameRefusal {
    fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new(self.record.code.as_str(), self.record.cause)
    }

    /// The record's own input path (`population`, and `object` or `field`
    /// where the finding names one).
    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        Some(self.record.fields.clone())
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
        self.cause
            .catalog_code()
            .with_prefix(qsl_semantics::family::FamilyKind::StateModel.catalog_code_prefix())
    }

    /// FR-096's key table rows for this cause (`absent-key` and
    /// `foreign-universe`); `None` for every cause with no row.
    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        self.cause.catalog_fields()
    }
}

#[cfg(test)]
mod tests {
    use super::{ModelQueryRefusal, ProtocolClauseSnapshot};
    use ix_trace_rs::trace;
    use qsl_foundation::diagnostic::{category_of, Code};
    use qsl_semantics::model::normalize::{ModelRefusal, ModelRefusalCause};
    use quire_semantic_value::catalog::{CatalogCode, CatalogCoded, Category};

    fn wrong_anchor() -> ProtocolClauseSnapshot {
        ProtocolClauseSnapshot::WrongAnchor {
            required: "pre",
            supplied: "post",
        }
    }

    fn forbidden_pre_read() -> ProtocolClauseSnapshot {
        ProtocolClauseSnapshot::ForbiddenPreRead {
            read: "pre(p)".to_owned(),
        }
    }

    /// TC-428 (FR-096-AC-7): each cause in FR-096's key table gives exactly
    /// the keys the table lists for it, valued from its own variant.
    #[trace("TC-428", "FR-096-AC-7")]
    #[test]
    fn catalog_fields_hold_exactly_the_keys_of_the_key_table() {
        fn keys(cause: &dyn CatalogCoded) -> Vec<(&'static str, String)> {
            cause
                .catalog_fields()
                .expect("a key-table row")
                .into_iter()
                .collect()
        }
        assert_eq!(
            keys(&wrong_anchor()),
            [
                ("required", "pre".to_owned()),
                ("supplied", "post".to_owned())
            ]
        );
        assert_eq!(keys(&forbidden_pre_read()), [("read", "pre(p)".to_owned())]);
        let absent = ModelQueryRefusal {
            cause: ModelRefusalCause::AbsentKey {
                binding: "pop-1".to_owned(),
                key: b"c9".to_vec(),
            },
            detail: String::new(),
        };
        assert_eq!(
            keys(&absent),
            [("binding", "pop-1".to_owned()), ("key", "c9".to_owned())]
        );
        let foreign = ModelQueryRefusal {
            cause: ModelRefusalCause::ForeignUniverse {
                actual: crate::test_ids::node_key([0xAB]),
                expected: crate::test_ids::universe_id([0x11]),
            },
            detail: String::new(),
        };
        assert_eq!(
            keys(&foreign),
            [
                ("required", crate::test_ids::node_key([0x11]).to_string()),
                ("supplied", crate::test_ids::node_key([0xAB]).to_string())
            ]
        );
        // A cause with no key-table row has no fields, not an empty map,
        // and builds no record.
        let mismatch = ModelQueryRefusal {
            cause: ModelRefusalCause::TypeMismatch,
            detail: String::new(),
        };
        assert_eq!(mismatch.catalog_fields(), None);
        assert_eq!(
            qsl_foundation::diagnostic::refusal_record(&mismatch, None),
            None
        );
    }

    /// TC-387 (FR-090-AC-6): each `WrongSnapshotCause` maps to
    /// `wrong_snapshot` with its own catalog cause tag, never through the
    /// native-v1 `Code`.
    #[trace("FR-090-AC-6", "TC-387")]
    #[test]
    fn protocol_clause_snapshot_catalog_code() {
        assert_eq!(
            wrong_anchor().catalog_code(),
            CatalogCode::new("wrong_snapshot", "wrong-anchor")
        );
        assert_eq!(
            forbidden_pre_read().catalog_code(),
            CatalogCode::new("wrong_snapshot", "forbidden-pre-read")
        );
    }

    /// TC-386 (FR-090-AC-5), steps 1 and 2: F `diagnostic`'s category map
    /// files every code the `ProtocolClause` snapshot cause and
    /// `ModelRefusal` return under `Category::Refusal`, except the
    /// incomplete-evidence codes, which it files under `Category::Incomplete`. The model-query
    /// causes come from `model::refusal`'s `exhaustive_samples`, which
    /// fails to compile when a cause variant is missing from it.
    #[trace("FR-090-AC-5", "TC-386")]
    #[test]
    fn family_refusal_codes_map_to_category_refusal() {
        for cause in [wrong_anchor(), forbidden_pre_read()] {
            let code = cause.catalog_code();
            assert_eq!(category_of(&code), Some(Category::Refusal), "{code}");
        }
        let samples = qsl_semantics::model::refusal::fixtures::exhaustive_samples();
        assert!(!samples.is_empty());
        for cause in samples {
            let code = cause.catalog_code();
            // The two incomplete-evidence codes are incomplete (FR-106,
            // ADR-012's admission table); every other cause is a refusal.
            let expected =
                if ["incomplete_population", "unavailable_observation"].contains(&code.code()) {
                    Category::Incomplete
                } else {
                    Category::Refusal
                };
            assert_eq!(category_of(&code), Some(expected), "{cause:?} -> {code}");
            // The catalog code is a native code spelling, and the cause tag
            // is the cause's own, except that a walk ceiling is a denied
            // charge (QSpec FR-150-AC-11).
            assert!(Code::from_code(code.code()).is_some(), "{code}");
            let tag = match cause {
                ModelRefusalCause::AncestorSteps { .. } | ModelRefusalCause::FamilySteps { .. } => {
                    "insufficient-next-charge"
                }
                _ => cause.as_str(),
            };
            assert_eq!(code.cause(), tag);
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
        assert_eq!(carried.catalog_code().prefix(), Some("state-model"));
        assert_eq!(
            expected,
            CatalogCode::new("cardinality_out_of_bound", "above-maximum")
        );
        let ModelQueryRefusal { cause, detail } = carried;
        let _: ModelRefusalCause = cause;
        let _: String = detail;
    }
}
