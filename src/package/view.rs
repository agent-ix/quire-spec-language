// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-019/021: borrowed typed package views, preserving the specified record order.

use std::{collections::BTreeSet, fmt};

use quire_contract_ir as ir;
use serde::{
    ser::{Error, SerializeSeq},
    Serialize, Serializer,
};

use super::NativePackageIdentity;
use crate::checking::{CheckedPackage, RuntimeRequirements, UniverseRequirement};
use crate::linking::{DeclarationKey, DeclarationLocation, ResolutionTarget, ResolvedOccurrence};
use qsl_foundation::{ByteDigest, Span};

pub(super) const FORMAT: &str = qsl_foundation::wire_format::WireFormat::LinkedPackage.as_str();

#[derive(Serialize)]
pub(super) struct Definition {
    pub revision: &'static str,
    pub digest: &'static str,
}

#[derive(Serialize)]
pub(super) struct Semantics {
    pub language: &'static str,
    pub edition: &'static str,
    pub syntax_profile: &'static str,
    pub model_profile: &'static str,
    pub checking_contract: &'static str,
    pub ir_revision: &'static str,
    pub base_definition: Definition,
    pub rules_definition: Definition,
}

impl Semantics {
    pub(super) fn selected() -> Self {
        const STANDARD: &str = "e897f810a7356d4ce8fd19026221ebda7b65596f";
        Self {
            language: crate::syntax::LANGUAGE,
            edition: crate::syntax::EDITION,
            syntax_profile: crate::syntax::PROFILE,
            model_profile: "native-state-model/1",
            checking_contract: "native-checked-clauses/1",
            ir_revision: "690bde7f2dc58662cf9ff0595c2c0e3b17107c6f",
            base_definition: Definition {
                revision: STANDARD,
                digest: "sha256:8bc68a3c7e46d26c7191dfbb662d4d63070af9fedc9ce985fb1885f7efe29429",
            },
            rules_definition: Definition {
                revision: STANDARD,
                digest: "sha256:d9eb316752ac45d7984b355a054cd279ef9749164a92c7f61fbf621fe280588b",
            },
        }
    }
}

struct DisplayValue<T>(T);

impl<T: fmt::Display> Serialize for DisplayValue<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self.0)
    }
}

#[derive(Serialize)]
struct Source<'a> {
    identity: &'a str,
    revision: &'a str,
    digest: DisplayValue<ByteDigest>,
    formal: &'a ir::SourceIdentity,
}

#[derive(Serialize)]
struct CanonicalIdentity {
    domain: &'static str,
    version: &'static str,
    algorithm: &'static str,
    digest: DisplayValue<NativePackageIdentity>,
}

#[derive(Serialize)]
pub(super) struct Manifest<'a, 'model> {
    format: &'static str,
    semantics: Semantics,
    required_features: &'a BTreeSet<&'static str>,
    source: Source<'a>,
    models: Models<'a, 'model>,
    clauses: Clauses<'a, 'model>,
    #[serde(skip_serializing_if = "Option::is_none")]
    canonical_identity: Option<CanonicalIdentity>,
}

impl<'a, 'model> Manifest<'a, 'model> {
    pub fn new(
        checked: &'a CheckedPackage<'model>,
        features: &'a BTreeSet<&'static str>,
        projections: bool,
        identity: Option<NativePackageIdentity>,
    ) -> Self {
        let original = checked.bindings().source.source();
        Self {
            format: FORMAT,
            semantics: Semantics::selected(),
            required_features: features,
            source: Source {
                identity: &original.identity().identity,
                revision: &original.identity().revision,
                digest: DisplayValue(original.digest()),
                formal: checked.bindings().source.identity(),
            },
            models: Models(checked),
            clauses: Clauses {
                checked,
                projections,
            },
            canonical_identity: identity.map(|digest| CanonicalIdentity {
                domain: "quire.native.bound-package",
                version: "v1",
                algorithm: "sha256",
                digest: DisplayValue(digest),
            }),
        }
    }
}

#[derive(Serialize)]
struct Model<'a> {
    alias: &'a str,
    owner: &'a ir::RequirementRef,
    digest: DisplayValue<ByteDigest>,
    artifact: &'a str,
}

struct Models<'a, 'model>(&'a CheckedPackage<'model>);

impl Serialize for Models<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let linked = self.0.linked();
        let imports = linked.unit().imports();
        if imports.len() != linked.models().len() {
            return Err(S::Error::custom(
                "checked import inventory differs from original source",
            ));
        }
        let mut seq = serializer.serialize_seq(Some(imports.len()))?;
        for (import, selected) in imports.iter().zip(linked.models()) {
            let model = selected
                .native_model()
                .ok_or_else(|| S::Error::custom("checked import has no native model"))?;
            let artifact = std::str::from_utf8(model.artifact_bytes()).map_err(S::Error::custom)?;
            seq.serialize_element(&Model {
                alias: &import.alias.value,
                owner: model.environment().owner(),
                digest: DisplayValue(model.digest()),
                artifact,
            })?;
        }
        seq.end()
    }
}

#[derive(Serialize)]
struct NativeSpan {
    start: usize,
    end: usize,
}

impl From<Span> for NativeSpan {
    fn from(span: Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Key<'a> {
    Type {
        name: &'a ir::SymbolName,
    },
    Value {
        name: &'a ir::SymbolName,
    },
    Scalar {
        name: &'a ir::SymbolName,
    },
    Field {
        record: &'a ir::SymbolName,
        field: &'a ir::SymbolName,
    },
    Variant {
        enumeration: &'a ir::SymbolName,
        variant: &'a ir::SymbolName,
    },
    Operation {
        context: &'a ir::SymbolName,
        name: &'a ir::SymbolName,
    },
}

impl<'a> From<&'a DeclarationKey> for Key<'a> {
    fn from(key: &'a DeclarationKey) -> Self {
        match key {
            DeclarationKey::Type(name) => Self::Type { name },
            DeclarationKey::Value(name) => Self::Value { name },
            DeclarationKey::Scalar(name) => Self::Scalar { name },
            DeclarationKey::Field { record, field } => Self::Field { record, field },
            DeclarationKey::Variant {
                enumeration,
                variant,
            } => Self::Variant {
                enumeration,
                variant,
            },
            DeclarationKey::Operation { context, name } => Self::Operation { context, name },
        }
    }
}

#[derive(Serialize)]
struct DeclarationIdentity<'a> {
    owner: &'a ir::RequirementRef,
    key: Key<'a>,
}

#[derive(Serialize)]
struct Location<'a> {
    identity: DeclarationIdentity<'a>,
    source: &'a ir::SourceSpan,
}

impl<'a> From<&'a DeclarationLocation> for Location<'a> {
    fn from(location: &'a DeclarationLocation) -> Self {
        Self {
            identity: DeclarationIdentity {
                owner: &location.identity.owner,
                key: (&location.identity.key).into(),
            },
            source: &location.source,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Target<'a> {
    Formal { declaration: Location<'a> },
    Local { binding: NativeSpan },
}

#[derive(Serialize)]
struct Occurrence<'a> {
    expression: Option<usize>,
    span: NativeSpan,
    target: Target<'a>,
}

struct Occurrences<'a>(&'a [ResolvedOccurrence]);

impl Serialize for Occurrences<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for occurrence in self.0 {
            seq.serialize_element(&Occurrence {
                expression: occurrence.expression.map(|id| id.0),
                span: occurrence.span.into(),
                target: match &occurrence.target {
                    ResolutionTarget::Formal(location) => Target::Formal {
                        declaration: location.into(),
                    },
                    ResolutionTarget::Local(span) => Target::Local {
                        binding: (*span).into(),
                    },
                },
            })?;
        }
        seq.end()
    }
}

struct Observations<'a>(&'a [ir::StateObservation]);

impl Serialize for Observations<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let order = [
            ir::StateObservation::Current,
            ir::StateObservation::Pre,
            ir::StateObservation::Post,
        ];
        let required = order.into_iter().filter(|o| self.0.contains(o));
        if required.clone().count() != self.0.len() {
            return Err(S::Error::custom("duplicate checked observations"));
        }
        serializer.collect_seq(required)
    }
}

#[derive(Serialize)]
struct Universe<'a> {
    model: &'a ir::RequirementRef,
    record: &'a ir::SymbolName,
    universe: &'a ir::SymbolName,
    observations: Observations<'a>,
}

struct Universes<'a, 'model>(&'a [UniverseRequirement<'model>]);

impl Serialize for Universes<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // The checker emits a BTreeMap keyed by owner/record; admission permits
        // one universe for that identity. Validate rather than sort/copy it.
        if self.0.windows(2).any(|pair| {
            let a = &pair[0];
            let b = &pair[1];
            (
                a.model.environment().owner(),
                &a.object.record,
                &a.object.universe,
            ) >= (
                b.model.environment().owner(),
                &b.object.record,
                &b.object.universe,
            )
        }) {
            return Err(S::Error::custom(
                "checked universe inventory is not ordered and unique",
            ));
        }
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for requirement in self.0 {
            seq.serialize_element(&Universe {
                model: requirement.model.environment().owner(),
                record: &requirement.object.record,
                universe: &requirement.object.universe,
                observations: Observations(&requirement.observations),
            })?;
        }
        seq.end()
    }
}

#[derive(Serialize)]
struct Operation<'a> {
    model: &'a ir::RequirementRef,
    context: &'a ir::SymbolName,
    name: &'a ir::SymbolName,
}

#[derive(Serialize)]
struct Runtime<'a, 'model> {
    context: Location<'a>,
    context_observations: Observations<'a>,
    universes: Universes<'a, 'model>,
    operation: Option<Operation<'a>>,
    validate_frame: bool,
}

impl<'a, 'model> From<&'a RuntimeRequirements<'model>> for Runtime<'a, 'model> {
    fn from(runtime: &'a RuntimeRequirements<'model>) -> Self {
        Self {
            context: (&runtime.context).into(),
            context_observations: Observations(&runtime.context_observations),
            universes: Universes(&runtime.universes),
            operation: runtime.operation.map(|role| Operation {
                model: runtime.model.environment().owner(),
                context: &role.context,
                name: &role.name,
            }),
            validate_frame: runtime.validate_frame,
        }
    }
}

#[derive(Serialize)]
struct NativeProjection {
    target: &'static str,
    status: &'static str,
    cost_model: &'static str,
}

#[derive(Serialize)]
struct IrProjection {
    target: &'static str,
    status: &'static str,
    code: &'static str,
    span: NativeSpan,
}

struct Projections(Span);

impl Serialize for Projections {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&NativeProjection {
            target: "native-reference/1",
            status: "available",
            cost_model: "native-ref-cost/1-draft",
        })?;
        seq.serialize_element(&IrProjection {
            target: "quire.contract.executable-projection/v1",
            status: "unlowered",
            code: "unsupported_construct",
            span: self.0.into(),
        })?;
        seq.end()
    }
}

#[derive(Serialize)]
struct Clause<'a, 'model> {
    name: &'a str,
    owner: &'a ir::RequirementRef,
    clause: &'a ir::ClauseId,
    kind: &'static str,
    execution_point: &'a ir::ExecutionPoint,
    span: NativeSpan,
    expression: usize,
    context: Location<'a>,
    operation: Option<Location<'a>>,
    occurrences: Occurrences<'a>,
    runtime: Runtime<'a, 'model>,
    #[serde(skip_serializing_if = "Option::is_none")]
    projections: Option<Projections>,
}

struct Clauses<'a, 'model> {
    checked: &'a CheckedPackage<'model>,
    projections: bool,
}

impl Serialize for Clauses<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let linked = self.checked.linked();
        let original = linked.unit().clauses();
        if original.len() != linked.clauses().len()
            || original.len() != self.checked.clauses().len()
        {
            return Err(S::Error::custom(
                "checked clause inventory differs from original source",
            ));
        }
        let mut seq = serializer.serialize_seq(Some(original.len()))?;
        for ((syntax, resolved), checked) in original
            .iter()
            .zip(linked.clauses())
            .zip(self.checked.clauses())
        {
            let root = linked.unit().expression(syntax.expression).ok_or_else(|| {
                S::Error::custom("checked clause has no original root expression")
            })?;
            let binding = checked.binding();
            seq.serialize_element(&Clause {
                name: &syntax.name.value,
                owner: &binding.requirement,
                clause: &binding.clause,
                kind: match syntax.kind {
                    crate::syntax::ClauseKind::Invariant => "invariant",
                    crate::syntax::ClauseKind::Precondition => "precondition",
                    crate::syntax::ClauseKind::Postcondition => "postcondition",
                },
                execution_point: &binding.execution_point,
                span: syntax.span.into(),
                expression: syntax.expression.0,
                context: resolved.context().into(),
                operation: resolved.operation().map(Into::into),
                occurrences: Occurrences(resolved.occurrences()),
                runtime: checked.runtime_requirements().into(),
                projections: self.projections.then_some(Projections(root.span)),
            })?;
        }
        seq.end()
    }
}
