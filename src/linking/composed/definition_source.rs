// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036: compiler-owned interpretations of native definition artifacts.
//!
//! This closed registry names, for each recognized interpretation, the
//! authority, identity, revision and artifact path a caller-supplied
//! definition is checked against, and the normative rules it selects. Every
//! entry is a forward reference: an authority, an identity string, a
//! revision string and a path telling a reader where to resolve the real
//! artifact from. None of it is embedded content, and recognizing a
//! caller-supplied definition is by identity and revision alone.
//!
//! This closed registry binds reviewed metadata to those references. It
//! neither interprets arbitrary Markdown nor discovers omitted invocation
//! inputs. Recognition alone establishes no model binding, type checking or
//! execution.

use super::definitions::Selection;
use crate::ByteDigest;

/// A compiler-known interpretation, unavailable through caller-defined metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RegisteredDefinition {
    /// The composed language edition and common source rules.
    Edition,
    /// State core expressions and declaration forms.
    StateCore,
    /// Reusable predicates and ordered queries.
    StateQueries,
    /// Finite identity-bearing graph expressions.
    StateGraph,
    /// Shared temporal interface, not a selectable concrete clock profile.
    TemporalFacet,
    /// Event-position clock with closed-scope false extension of atoms.
    EventPosition,
    /// Exact fixed-sample clock with closed-scope false extension of atoms.
    FixedSample,
    /// Timestamped finite-window interpretation.
    TimestampedWindow,
    /// Finite global protocol interpretation.
    Protocol,
    /// Observation binding interpretation, not a clause profile.
    ObservationBinding,
    /// Scoped observation progress and restoration interpretation.
    Progress,
    /// Observation range correspondence with an assessment-selected clock.
    Range,
    /// Linked-package container interpretation, not an inner clause profile.
    Package,
    /// Native diagnostic interpretation, not a clause profile.
    Diagnostics,
}

/// One normative rule selected by a registered definition: an inventory key,
/// never an instruction to retrieve content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleArtifact {
    /// This repository's own path, or the original URL for an external rule.
    pub path: &'static str,
}

struct RegisteredSource {
    authority: &'static str,
    identity: &'static str,
    revision: &'static str,
    path: &'static str,
    requirements: &'static [RegisteredDefinition],
    rules: &'static [RuleArtifact],
}

/// This registry's publishing authority for every entry.
const AUTHORITY: &str = "agent-ix";

macro_rules! rule {
    ($path:literal) => {
        RuleArtifact { path: $path }
    };
}

macro_rules! definition {
    ($file:literal, $identity:literal, $revision:literal, [$($requirement:ident),*], [$($rule:expr),* $(,)?]) => {
        RegisteredSource {
            authority: AUTHORITY,
            identity: $identity,
            revision: $revision,
            path: concat!("proposals/quire-v1/definitions/", $file, ".md"),
            requirements: &[$(RegisteredDefinition::$requirement),*],
            rules: &[$($rule),*],
        }
    };
}

impl RegisteredDefinition {
    /// Every interpretation this compiler registry recognizes, in stable order.
    pub fn all() -> &'static [Self] {
        &[
            Self::Edition,
            Self::StateCore,
            Self::StateQueries,
            Self::StateGraph,
            Self::TemporalFacet,
            Self::EventPosition,
            Self::FixedSample,
            Self::TimestampedWindow,
            Self::Protocol,
            Self::ObservationBinding,
            Self::Progress,
            Self::Range,
            Self::Package,
            Self::Diagnostics,
        ]
    }

    /// The authority that publishes this definition's identity.
    pub fn authority(self) -> &'static str {
        self.source().authority
    }

    /// Exact definition identity, distinct from a source-local alias.
    pub fn identity(self) -> &'static str {
        self.source().identity
    }

    /// Exact artifact revision, distinct from the selected language edition.
    pub fn revision(self) -> &'static str {
        self.source().revision
    }

    /// Definition path relative to the selected standard snapshot root.
    pub fn path(self) -> &'static str {
        self.source().path
    }

    /// A synthetic byte payload derived from this definition's own identity
    /// string, never real document content. Not gated: it derives nothing
    /// but the identity already public via [`Self::identity`], so there is
    /// no production-caller boundary to enforce here.
    pub fn bytes(self) -> &'static [u8] {
        self.identity().as_bytes()
    }

    /// A selection whose digest covers [`Self::bytes`]'s synthetic
    /// placeholder, never real document content. See [`Self::bytes`].
    pub fn selection(self) -> Selection {
        Selection {
            identity: self.identity().into(),
            revision: self.revision().into(),
            digest: ByteDigest::of(self.bytes()),
        }
    }

    /// Direct required definitions, retaining the artifact's declared order.
    /// Parameterized producer inputs and concrete observation clock selections
    /// are not invented as static definition dependencies.
    pub fn requirements(self) -> &'static [Self] {
        self.source().requirements
    }

    /// Directly selected rules; transitive requirements retain their own rules.
    /// Background links do not become implicit dependencies.
    pub fn rules(self) -> &'static [RuleArtifact] {
        self.source().rules
    }

    fn source(self) -> RegisteredSource {
        match self {
            Self::Edition => definition!("edition", "ix:native", "1-draft.2", [], [
                rule!("proposals/quire-v1/shared-grammar.md"),
                rule!("proposals/quire-v1/package-contract.md"),
                rule!("spec/functional/FR-030-bind-composed-definitions.md"),
                rule!("spec/functional/FR-031-report-requested-capabilities.md"),
                rule!("spec/functional/FR-032-preserve-language-evolution.md"),
                rule!("spec/functional/FR-035-bind-ecosystem-subjects.md"),
                rule!("spec/functional/FR-036-retain-lexical-source-locations.md"),
                rule!("spec/functional/FR-037-parse-shared-native-expressions.md"),
                rule!("spec/functional/FR-038-resolve-predicate-scopes.md"),
                rule!("spec/functional/FR-040-admit-explicit-state-extensions.md"),
                rule!("spec/functional/FR-047-emit-typed-located-causes.md"),
                rule!("spec/non-functional/NFR-010-bound-composed-processing.md"),
            ]),
            Self::StateCore => definition!("state-core", "quire.state.core/v1", "1-draft.3", [Edition], [
                rule!("proposals/quire-v1/state-contract.md"),
                rule!("spec/functional/FR-039-normalize-exact-rational-literals.md"),
                rule!("spec/functional/FR-044-check-composed-numeric-definedness.md"),
                rule!("spec/functional/FR-045-preserve-control-and-presence-facts.md"),
                rule!("spec/functional/FR-046-validate-state-invocation-inputs.md"),
            ]),
            Self::StateQueries => definition!("state-queries", "quire.state.queries/v1", "1-draft.3", [StateCore], [
                rule!("spec/functional/FR-033-admit-reusable-predicates.md"),
                rule!("spec/functional/FR-034-bind-cross-family-predicates.md"),
                rule!("spec/functional/FR-041-evaluate-ordered-queries.md"),
                rule!("spec/functional/FR-042-select-pre-state-reads.md"),
            ]),
            Self::StateGraph => definition!("state-graph", "quire.state.graph/v1", "1-draft.3", [StateQueries], [
                rule!("spec/functional/FR-043-evaluate-finite-graph-relations.md"),
            ]),
            Self::TemporalFacet => definition!("temporal-common", "quire.temporal.bounded-facet/v1", "1-draft.3", [StateGraph], [
                rule!("spec/functional/FR-048-bind-native-temporal-syntax.md"),
                rule!("spec/functional/FR-090-select-temporal-profile-and-clock.md"),
                rule!("spec/functional/FR-091-evaluate-bounded-future.md"),
                rule!("spec/functional/FR-092-evaluate-bounded-past.md"),
                rule!("spec/functional/FR-093-bind-temporal-activation-and-captures.md"),
                rule!("spec/functional/FR-094-interpret-progress-history-and-closure.md"),
                rule!("spec/functional/FR-095-preserve-native-tl-correspondence.md"),
                rule!("spec/non-functional/NFR-040-bound-temporal-state.md"),
                rule!("spec/functional/FR-061-report-orthogonal-results.md"),
            ]),
            Self::EventPosition => definition!("temporal-event-position", "quire.temporal.event-position.false-extension/v1", "1-draft.3", [TemporalFacet], []),
            Self::FixedSample => definition!("temporal-fixed-sample", "quire.temporal.fixed-sample.false-extension/v1", "1-draft.3", [TemporalFacet], []),
            Self::TimestampedWindow => definition!("temporal-timestamped-window", "quire.temporal.timestamped-event.finite-window/v1", "1-draft.3", [TemporalFacet], []),
            Self::Protocol => definition!("protocol-finite", "quire.protocol.finite-global/v1", "1-draft.4", [StateGraph, TemporalFacet], [
                rule!("spec/functional/FR-049-bind-native-choreography-syntax.md"),
                rule!("spec/functional/FR-050-bind-protocol-instances.md"),
                rule!("spec/functional/FR-051-preserve-communication-identities.md"),
                rule!("spec/functional/FR-052-represent-bounded-control.md"),
                rule!("spec/functional/FR-053-enforce-choice-visibility.md"),
                rule!("spec/functional/FR-054-bind-channel-premises.md"),
                rule!("spec/functional/FR-055-activate-protocol-obligations.md"),
                rule!("spec/functional/FR-056-register-compensation.md"),
                rule!("spec/functional/FR-057-enforce-commit-recovery.md"),
                rule!("spec/functional/FR-058-preserve-retry-and-partial-recovery.md"),
                rule!("spec/functional/FR-059-assess-finite-global-conformance.md"),
                rule!("spec/functional/FR-060-separate-protocol-claims.md"),
                rule!("spec/functional/FR-061-report-orthogonal-results.md"),
                rule!("spec/non-functional/NFR-020-bound-protocol-processing.md"),
                rule!("spec/non-functional/NFR-021-reproduce-protocol-results.md"),
                rule!("proposals/quire-v1/choreography-surface.md"),
                rule!("proposals/quire-v1/protocol-contract.md"),
            ]),
            Self::ObservationBinding => definition!("observation-binding", "quire.observation.binding/v1", "1-draft.3", [Package, Range, TemporalFacet, Protocol], [
                rule!("proposals/quire-v1/observation-contract.md"),
                rule!("spec/functional/FR-061-report-orthogonal-results.md"),
                rule!("spec/functional/FR-115-report-activation-participation-and-adequacy.md"),
                rule!("spec/functional/FR-116-map-observation-results-to-consumers.md"),
                rule!("proposals/quire-v1/observation-output-mapping-contract.md"),
            ]),
            Self::Progress => definition!("observation-progress", "quire.observation.progress/v1", "1-draft.3", [ObservationBinding, Range, TemporalFacet], [
                rule!("proposals/quire-v1/observation-contract.md"),
                rule!("spec/functional/FR-061-report-orthogonal-results.md"),
            ]),
            Self::Range => definition!("observation-range", "quire.observation.range/v1", "1-draft.1", [], [
                rule!("proposals/quire-v1/observation-contract.md"),
                rule!("spec/functional/FR-090-select-temporal-profile-and-clock.md"),
            ]),
            Self::Package => definition!("package-reference", "quire.package.composed/v1", "1-draft.2", [Edition], [
                rule!("proposals/shared-reference-2-draft/schema.json"),
                rule!("proposals/shared-reference-2-draft/README.md"),
            ]),
            Self::Diagnostics => definition!("native-diagnostics", "quire.native.diagnostics/v1", "1-draft.1", [], [
                rule!("https://github.com/agent-ix/quire-spec-language/blob/f444d03c06539a6cd0ada6be4ae099b54466d9d9/src/diagnostic.rs"),
                rule!("https://github.com/agent-ix/quire-spec-language/blob/f444d03c06539a6cd0ada6be4ae099b54466d9d9/docs/native-error-codes.md"),
            ]),
        }
    }
}
