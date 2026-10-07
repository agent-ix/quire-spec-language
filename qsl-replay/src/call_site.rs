// SPDX-License-Identifier: AGPL-3.0-or-later
//! A thin CG-facing cut of the front-end operations in [`crate::spine`]: the facts a client
//! needs to key a `ReplayRequestWire` or a frame or state-clause
//! counterexample by node id
//! and package id (ADR-013 O-25, C-11), without reaching
//! `qsl_replay::spine` itself, which CG may not call (ADR-011 §3 FB-05,
//! T-12 rule (a): `spine` is public only for `command`).
//!
//! [`call_site`] compiles one unit against the supplied domain package
//! documents and dependency input, under the default spine stage limits,
//! and locates one selection in the result: the compiled package's own
//! `package_id` and a named function's parameters, each paired with its own
//! node id, a named operation's anchor, frame and state clause identities,
//! or a named state clause's identities, or a named state field's or
//! population's domain key (ADR-012 §15.4, §15.7). It also returns the compiled package's own
//! `quire.checked-package/v2` bytes. Every node id is the same
//! `WireNodeId` a `CanonicalAssignment`, a witness transcript or a
//! counterexample names, joined by declared identity, never by position (ADR-013 O-25).
//! It builds no [`crate::execute::replay`] call and no
//! [`crate::spine::Call`]; it names a call site, it does not make one.

use qsl_foundation::bound::DomainKey;
use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{Code, SourceIdentity};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedGraph, CheckedOperationFrame, CheckedStateClause};
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::intake::package_input;
use quire_exact::{Cancel, Identifier, NodeKey, Origin, Role};

use crate::execute::callable_parameter_keys;
use crate::identity::QualifiedName;
use crate::spine::{self, CompileRefusal, DependencyInput, DependencyInputRefusal, OperationName};

/// What [`call_site`] can locate in the compiled package, and what it
/// answers for each:
/// - a [`QualifiedName`] selects a function, by the one-segment name
///   FR-098's selection resolves, and locates a [`FunctionSite`];
/// - an [`OperationName`] `M::T::op` selects an operation, resolved as
///   FR-115's `Frame` selection resolves it, and locates an
///   [`OperationSite`];
/// - a [`ClauseName`] selects a state clause -- an invariant, a
///   precondition or a postcondition -- by its declared name, as FR-106's
///   `ClauseSelection` names it, and locates a [`ClauseSite`];
/// - a [`FieldName`] `M::T.f` selects a state field of an object type and
///   locates a [`FieldSite`], its ADR-012 §15.4 domain key;
/// - a [`PopulationName`] `M::P` selects a population of a model and
///   locates a [`PopulationSite`], its ADR-012 §15.7 domain key.
///
/// Sealed: these five are the selections.
pub trait CallSiteSelection: sealed::Locate {}

impl CallSiteSelection for QualifiedName {}
impl CallSiteSelection for OperationName {}
impl CallSiteSelection for ClauseName {}
impl CallSiteSelection for FieldName {}
impl CallSiteSelection for PopulationName {}

/// A population `M::P`: the `model` alias `M` and a population declaration
/// `P` of that model's domain package, by its artifact id (the `P` of
/// `ix://<package>/P`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationName {
    /// The model alias `M`.
    pub model: Identifier,
    /// The population `P`.
    pub population: Identifier,
}

impl std::fmt::Display for PopulationName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.model.as_str(), self.population.as_str())
    }
}

/// A state field `M::T.f`: the `model` alias `M`, an object type `T` of
/// that model, and a field `f` of `T`'s effective attribute set, declared
/// by `T` or inherited from a supertype.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldName {
    /// The model alias `M`.
    pub model: Identifier,
    /// The object type `T` the field is named through.
    pub object: Identifier,
    /// The field `f`.
    pub field: Identifier,
}

impl std::fmt::Display for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}::{}.{}",
            self.model.as_str(),
            self.object.as_str(),
            self.field.as_str()
        )
    }
}

/// A state clause of the unit, by its declared name: the name FR-106's
/// `ClauseSelection` and a state-clause counterexample name it by.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseName(pub Identifier);

impl std::fmt::Display for ClauseName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

mod sealed {
    use qsl_foundation::digest::DigestRecord;
    use qsl_package::CheckedPackage;

    use super::CallSiteRefusal;

    /// How a selection locates itself in a compiled package.
    pub trait Locate {
        /// What the selection locates.
        type Site;

        /// Locate `self` in `package`, whose `package_id` is `package_id`.
        fn locate(
            &self,
            package: &CheckedPackage,
            package_id: DigestRecord,
        ) -> Result<Self::Site, Box<CallSiteRefusal>>;
    }
}

/// What [`call_site`] returns: the compiled package's own content-addressed
/// identity, its lowered bytes, and what the selection located, a
/// [`FunctionSite`], an [`OperationSite`], a [`ClauseSite`], a
/// [`FieldSite`] or a [`PopulationSite`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallSite<S> {
    /// The compiled package's `package_id`, as a `quire.package.semantic/v2`
    /// digest record -- the same shape `ReplayRequestWire::package_id` and
    /// a witness envelope's package identity carry.
    pub package_id: DigestRecord,
    /// The compiled package's `quire.checked-package/v2` bytes, exactly as
    /// the S4 emitter wrote them when it minted `package_id`: their
    /// `identity_preimage` member's RFC 8785 bytes digest to `package_id`.
    pub package: Vec<u8>,
    /// The located function or operation.
    pub site: S,
}

/// A function's parameters in the compiled package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionSite {
    /// The compiled graph's `function` node whose `declaration` is the
    /// selected function: the function node id member of the function
    /// contract obligation's ADR-013 O-09 preimage (FR-121-AC-15).
    pub function: WireNodeId,
    /// The function node's `declaration` occurrence at ordinal 0, the
    /// declaration occurrence member of that preimage. Its source span is
    /// not part of it.
    pub declaration: OccurrenceKey,
    /// The parameters, in declared order. Each pair is the parameter's
    /// declared name and the `WireNodeId` a `CanonicalAssignment::parameter`
    /// or a witness transcript names to bind that parameter's argument --
    /// joined by declared identity, never by position (ADR-013 O-25, C-11).
    pub parameters: Vec<(Identifier, WireNodeId)>,
}

/// An operation's identities in the compiled package (FR-104, FR-105): the
/// members a frame counterexample carries (FR-116) and the node and
/// occurrence each state clause naming the operation is recorded under.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationSite {
    /// The operation's `state`/`operation_anchor` node.
    pub anchor: WireNodeId,
    /// The operation's `state`/`frame` node.
    pub frame: WireNodeId,
    /// The frame node's own `generated` occurrence (FR-104-AC-5): the key
    /// the frame's `operation-contract` requirement is recorded under and
    /// a frame counterexample names.
    pub frame_occurrence: OccurrenceKey,
    /// Every `pre` and `post` clause of the unit whose operation resolves
    /// to this operation, in declaration order.
    pub clauses: Vec<ClauseSite>,
}

/// A state field's domain in the compiled package (ADR-012 §15.4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldSite {
    /// The field's domain key, always a [`DomainKey::Node`]: the
    /// `model`/`object_type` node of the type that declares the field, and
    /// one path element, the field's ordinal among that type's own field
    /// declarations in ascending field-name UTF-8 byte order. A domain
    /// inside the field's type extends this path with the child-index path
    /// into that type. The key a `DeclaredDomain` bounding the field names.
    pub domain: DomainKey,
}

/// A population's domain in the compiled package (ADR-012 §15.7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationSite {
    /// The population's domain key, always a [`DomainKey::Population`]:
    /// its canonical member type's `model`/`object_type` node and its
    /// ordinal among the package's population declarations in ascending
    /// `DeclarationKey` order -- the key the package's own requirement
    /// records carry for it, and the one a `DeclaredDomain` bounding it
    /// names.
    pub domain: DomainKey,
}

/// One state clause's identities in the compiled package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseSite {
    /// The clause's declared name.
    pub name: Identifier,
    /// The clause's `state_clause` node.
    pub node: WireNodeId,
    /// The clause's `claim` occurrence of its node (ADR-013 O-07).
    pub occurrence: OccurrenceKey,
}

/// Why [`call_site`] named no call site.
#[derive(Debug, thiserror::Error)]
pub enum CallSiteRefusal {
    /// The unit's own source did not compile: S1, S2, a header profile, the
    /// assembler, the checker, the link step or the emitter refused it.
    /// Carries the refusal's catalog code and rendered message; the stage
    /// and region detail live only in `qsl_replay::spine`, which this
    /// facade does not expose.
    #[error("the source did not compile: {message}")]
    Compile {
        /// The compile refusal's catalog code.
        code: Code,
        /// The compile refusal's rendered message.
        message: String,
    },
    /// A `model` declaration of the unit selects a domain package that the
    /// supplied `packages` do not admit (I1).
    #[error("domain package intake refused `model {alias}`: {message}")]
    ModelIntake {
        /// The `model` declaration's alias.
        alias: String,
        /// The intake refusal's catalog code.
        code: Code,
        /// The intake refusal's rendered message.
        message: String,
    },
    /// The supplied `dependencies` refused against the unit: a library
    /// shares the unit's source owner (ADR-013 O-04).
    #[error("the dependency input refused: {0}")]
    DependencyInput(DependencyInputRefusal),
    /// One of the unit's own `import`s did not resolve against the supplied
    /// `dependencies` (ADR-015 D-1): no library is supplied under its
    /// identity, its version or digest disagrees, or the imports form a
    /// cycle or a diamond. Carries the resolution's catalog code and
    /// rendered message.
    #[error("{message}")]
    Import {
        /// The import resolution refusal's catalog code.
        code: Code,
        /// The import resolution refusal's rendered message.
        message: String,
    },
    /// A supplied library, resolved through the unit's imports, did not
    /// compile.
    #[error("the library {} refused: {message}", display_path(.path))]
    Dependency {
        /// The library identities from the unit's import down to the
        /// library that refused, that library last.
        path: Vec<LibraryName>,
        /// The library's own refusal's catalog code.
        code: Code,
        /// The library's own refusal's rendered message.
        message: String,
    },
    /// `selection` names no function of the compiled `package`. Never a
    /// bare `QualifiedName` (FR-088-AC-6, TC-258): paired with the package
    /// it was looked up in, mirroring `ReplayRefusal::UnknownFunction`.
    #[error("missing_declaration/missing-name: package {} declares no function {selection}", .package.hex())]
    UnknownFunction {
        /// The name `call_site` was given.
        selection: QualifiedName,
        /// The compiled package's own `package_id`.
        package: DigestRecord,
    },
    /// `selection` names no operation frame of the compiled `package`: its
    /// model alias or object type does not resolve, the operation resolves
    /// to no single operation, or no clause or attempt of the unit names it
    /// (FR-105 emits no frame for it). Paired with the package, as
    /// [`Self::UnknownFunction`] is (FR-088-AC-6).
    #[error("missing_declaration/missing-name: package {} holds no operation frame {selection}", .package.hex())]
    UnknownOperation {
        /// The operation `call_site` was given.
        selection: OperationName,
        /// The compiled package's own `package_id`.
        package: DigestRecord,
    },
    /// `selection` names no state clause of the compiled `package`. Paired
    /// with the package, as [`Self::UnknownFunction`] is (FR-088-AC-6).
    #[error("missing_declaration/missing-name: package {} declares no state clause {selection}", .package.hex())]
    UnknownClause {
        /// The clause name `call_site` was given.
        selection: ClauseName,
        /// The compiled package's own `package_id`.
        package: DigestRecord,
    },
    /// `selection` names no state field of the compiled `package`: its model
    /// alias or object type does not resolve, or the object type's
    /// effective attribute set has no field of that name. Paired with the
    /// package, as [`Self::UnknownFunction`] is (FR-088-AC-6).
    #[error("missing_declaration/missing-name: package {} declares no field {selection}", .package.hex())]
    UnknownField {
        /// The field `call_site` was given.
        selection: FieldName,
        /// The compiled package's own `package_id`.
        package: DigestRecord,
    },
    /// `selection` names no population of the compiled `package`: its model
    /// alias does not resolve, or its domain package declares no population
    /// of that artifact id. Paired with the package, as
    /// [`Self::UnknownFunction`] is (FR-088-AC-6).
    #[error("missing_declaration/missing-name: package {} declares no population {selection}", .package.hex())]
    UnknownPopulation {
        /// The population `call_site` was given.
        selection: PopulationName,
        /// The compiled package's own `package_id`.
        package: DigestRecord,
    },
    /// A broken invariant: the compiled package's own function node for a
    /// selected function is not itself a function node, or its parameter
    /// count does not match its checked signature, or a declared parameter
    /// or clause name is not itself a valid `Identifier`.
    #[error("internal fault in {}: {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
}

impl CallSiteRefusal {
    /// The catalog code of this refusal, from the closed catalog
    /// [`crate::ReplayRefusal::code`] draws on: the same code `replay`
    /// gives the same refusal. A compile, intake, import or library refusal
    /// keeps the code of the refusal it was built from, as
    /// `ReplayRefusal::Recompile` does.
    pub fn code(&self) -> Code {
        match self {
            Self::Compile { code, .. }
            | Self::ModelIntake { code, .. }
            | Self::Import { code, .. }
            | Self::Dependency { code, .. } => *code,
            Self::DependencyInput(refusal) => refusal.code(),
            Self::UnknownFunction { .. }
            | Self::UnknownOperation { .. }
            | Self::UnknownClause { .. }
            | Self::UnknownField { .. }
            | Self::UnknownPopulation { .. } => Code::MissingDeclaration,
            Self::Fault(_) => Code::RuntimeInvariant,
        }
    }
}

/// Compile `bytes` (labelled `source`, displayed as `path`) through S1 to S4
/// under the default spine stage limits, with `packages` -- each a domain
/// package document, keyed by its own `sha256-jcs` digest as `replay` keys
/// the documents of its byte provision -- as I1's package input and
/// `dependencies` as the dependency input, and locate `selection` in the
/// result.
///
/// # Function selection (TC-166)
///
/// A function is selected by a typed [`QualifiedName`]; a name that
/// resolves to no function refuses [`CallSiteRefusal::UnknownFunction`].
/// No selection takes a bare `&str`: this call, which differs from the one
/// after it only in its last argument, does not compile.
///
/// ```compile_fail,E0277
/// # use qsl_replay::{call_site, DependencyInput, Identifier, QualifiedName, SourceIdentity};
/// # let unit = "language \"ix:native\" edition \"1-draft\";\n\
/// #     profile v = \"quire.value.complete/v1\";\n\
/// #     function f using v(x: Int[0, 9]): Integer pure { x + 5 }\n";
/// let f = QualifiedName::new(vec![Identifier::new("f").unwrap()]).unwrap();
/// let site = call_site(
///     SourceIdentity::new("a", "u", "git", "1"),
///     "u",
///     unit.as_bytes(),
///     [],
///     &DependencyInput::default(),
///     "f",
/// )
/// .unwrap();
/// assert_eq!(site.site.parameters[0].0.as_str(), "x");
/// ```
///
/// The same call selecting `f` by its `QualifiedName` compiles and locates
/// `f`'s parameter:
///
/// ```
/// # use qsl_replay::{call_site, DependencyInput, Identifier, QualifiedName, SourceIdentity};
/// # let unit = "language \"ix:native\" edition \"1-draft\";\n\
/// #     profile v = \"quire.value.complete/v1\";\n\
/// #     function f using v(x: Int[0, 9]): Integer pure { x + 5 }\n";
/// let f = QualifiedName::new(vec![Identifier::new("f").unwrap()]).unwrap();
/// let site = call_site(
///     SourceIdentity::new("a", "u", "git", "1"),
///     "u",
///     unit.as_bytes(),
///     [],
///     &DependencyInput::default(),
///     &f,
/// )
/// .unwrap();
/// assert_eq!(site.site.parameters[0].0.as_str(), "x");
/// ```
pub fn call_site<'a, S: CallSiteSelection>(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: impl IntoIterator<Item = &'a [u8]>,
    dependencies: &DependencyInput,
    selection: &S,
) -> Result<CallSite<S::Site>, Box<CallSiteRefusal>> {
    let cancel = Cancel::new();
    let refusal = |failure| match spine::refusal_or_fault(failure) {
        Ok(refusal) => Box::new(CallSiteRefusal::from(*refusal)),
        Err(fault) => Box::new(CallSiteRefusal::Fault(fault)),
    };
    let limits = spine::SpineLimits::default();
    let packages = package_input(packages);
    let parsed = spine::parse(
        &spine::ParseRequest {
            source: &source,
            path,
            bytes,
        },
        limits.source,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(refusal)?
        .into_value();
    let checked = spine::check(
        &parsed,
        &models,
        dependencies,
        &spine::LockEvidence::default(),
        limits,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let emitted = spine::package(&checked, spine::PackageLimits::default(), &cancel)
        .map_err(refusal)?
        .into_value();
    let package_id = emitted.package().package_id().record();
    let site = selection.locate(checked.package(), package_id)?;
    Ok(CallSite {
        package_id,
        package: emitted.package().bytes().to_vec(),
        site,
    })
}

impl From<CompileRefusal> for CallSiteRefusal {
    /// Sort a spine compile refusal by whose input it concerns: the unit's
    /// own source, the domain packages, the dependency input, an import, or
    /// a supplied library.
    fn from(refusal: CompileRefusal) -> Self {
        let code = refusal.code();
        match refusal {
            CompileRefusal::Intake { refusal, .. } => Self::ModelIntake {
                message: spine::intake_message(&refusal.cause),
                alias: refusal.alias,
                code,
            },
            CompileRefusal::DependencyInput(input) => Self::DependencyInput(input),
            CompileRefusal::Import { refusal, .. } => Self::Import {
                code,
                message: refusal.to_string(),
            },
            CompileRefusal::Dependency { path, refusal } => Self::Dependency {
                path,
                code,
                message: refusal.to_string(),
            },
            refusal @ (CompileRefusal::Source(_)
            | CompileRefusal::Forms { .. }
            | CompileRefusal::Profile { .. }
            | CompileRefusal::Assembly { .. }
            | CompileRefusal::Check { .. }
            | CompileRefusal::Link(_)
            | CompileRefusal::Emit(_)
            | CompileRefusal::Limit(_)
            | CompileRefusal::Omitted(_)) => Self::Compile {
                code,
                message: refusal.to_string(),
            },
        }
    }
}

/// A dependency path written `a -> b -> c`.
fn display_path(path: &[LibraryName]) -> String {
    path.iter()
        .map(LibraryName::as_str)
        .collect::<Vec<_>>()
        .join(" -> ")
}

impl sealed::Locate for QualifiedName {
    type Site = FunctionSite;

    /// `self`'s parameters in `package`, each paired with its own node id,
    /// by the one-segment name lookup and node-key derivation `replay`'s own
    /// selection uses.
    fn locate(
        &self,
        package: &CheckedPackage,
        package_id: DigestRecord,
    ) -> Result<FunctionSite, Box<CallSiteRefusal>> {
        let unknown = || {
            Box::new(CallSiteRefusal::UnknownFunction {
                selection: self.clone(),
                package: package_id,
            })
        };
        let [segment] = self.segments() else {
            return Err(unknown());
        };
        let callable = package
            .graph()
            .callable(segment.as_str())
            .ok_or_else(unknown)?;
        let keys = callable_parameter_keys(package, &callable)
            .map_err(|fault| Box::new(CallSiteRefusal::Fault(fault)))?;
        let parameters = callable
            .parameters
            .iter()
            .zip(keys)
            .map(|((name, _value_type), key)| {
                Ok((
                    identifier(name, "declared-parameter-name-is-a-valid-identifier")?,
                    wire_id(key),
                ))
            })
            .collect::<Result<Vec<_>, Box<CallSiteRefusal>>>()?;
        let function = wire_id(callable.identity);
        Ok(FunctionSite {
            function,
            declaration: OccurrenceKey::new(function, Origin::new(Role::new("declaration"), 0)),
            parameters,
        })
    }
}

impl sealed::Locate for OperationName {
    type Site = OperationSite;

    /// `self`'s frame in `package`, resolved as FR-115's `Frame` selection
    /// resolves it, with its identities.
    fn locate(
        &self,
        package: &CheckedPackage,
        package_id: DigestRecord,
    ) -> Result<OperationSite, Box<CallSiteRefusal>> {
        let graph = package.graph();
        let (_, frame) = graph
            .resolve_operation(&self.model, &self.object, &self.operation)
            .and_then(|resolved| graph.operation_frame(&resolved))
            .ok_or_else(|| {
                Box::new(CallSiteRefusal::UnknownOperation {
                    selection: self.clone(),
                    package: package_id,
                })
            })?;
        operation_site(graph, frame)
    }
}

impl sealed::Locate for ClauseName {
    type Site = ClauseSite;

    /// The state clause of `package` declared `self`, with its identities.
    fn locate(
        &self,
        package: &CheckedPackage,
        package_id: DigestRecord,
    ) -> Result<ClauseSite, Box<CallSiteRefusal>> {
        let clause = package
            .graph()
            .state_clause(self.0.as_str())
            .ok_or_else(|| {
                Box::new(CallSiteRefusal::UnknownClause {
                    selection: self.clone(),
                    package: package_id,
                })
            })?;
        clause_site(clause)
    }
}

impl sealed::Locate for FieldName {
    type Site = FieldSite;

    /// `self`'s domain key in `package` (ADR-012 §15.4).
    fn locate(
        &self,
        package: &CheckedPackage,
        package_id: DigestRecord,
    ) -> Result<FieldSite, Box<CallSiteRefusal>> {
        package
            .graph()
            .field_domain(&self.model, &self.object, &self.field)
            .map_err(|fault| Box::new(CallSiteRefusal::Fault(fault)))?
            .map(|domain| FieldSite { domain })
            .ok_or_else(|| {
                Box::new(CallSiteRefusal::UnknownField {
                    selection: self.clone(),
                    package: package_id,
                })
            })
    }
}

impl sealed::Locate for PopulationName {
    type Site = PopulationSite;

    /// `self`'s domain key in `package` (ADR-012 §15.7).
    fn locate(
        &self,
        package: &CheckedPackage,
        package_id: DigestRecord,
    ) -> Result<PopulationSite, Box<CallSiteRefusal>> {
        package
            .graph()
            .population_domain(&self.model, &self.population)
            .map_err(|fault| Box::new(CallSiteRefusal::Fault(fault)))?
            .map(|domain| PopulationSite { domain })
            .ok_or_else(|| {
                Box::new(CallSiteRefusal::UnknownPopulation {
                    selection: self.clone(),
                    package: package_id,
                })
            })
    }
}

/// `frame`'s identities in `graph`, with every state clause whose operation
/// is `frame`'s operation -- the same (declaring type, operation) identity
/// [`CheckedGraph::operation_frame`] selects the frame by.
fn operation_site(
    graph: &CheckedGraph,
    frame: &CheckedOperationFrame,
) -> Result<OperationSite, Box<CallSiteRefusal>> {
    let clauses = graph
        .state_clauses()
        .iter()
        .filter(|clause| clause.operation() == Some(frame.operation()))
        .map(clause_site)
        .collect::<Result<Vec<_>, Box<CallSiteRefusal>>>()?;
    let frame_node = wire_id(frame.frame());
    Ok(OperationSite {
        anchor: wire_id(frame.anchor()),
        frame: frame_node,
        frame_occurrence: OccurrenceKey::new(frame_node, frame.frame_origin().clone()),
        clauses,
    })
}

/// `clause`'s declared name, `state_clause` node and `claim` occurrence.
fn clause_site(clause: &CheckedStateClause) -> Result<ClauseSite, Box<CallSiteRefusal>> {
    let node = wire_id(clause.identity());
    Ok(ClauseSite {
        name: identifier(clause.name(), "declared-clause-name-is-a-valid-identifier")?,
        node,
        occurrence: OccurrenceKey::new(node, clause.claim().clone()),
    })
}

/// A declared name as an `Identifier`; a checked declaration's name always
/// is one, so a refusal is the broken `invariant`.
fn identifier(name: &str, invariant: &'static str) -> Result<Identifier, Box<CallSiteRefusal>> {
    Identifier::new(name).map_err(|_| {
        Box::new(CallSiteRefusal::Fault(InternalFault::new(
            "replay", invariant,
        )))
    })
}

/// A checked node's key as the wire spells it.
fn wire_id(key: NodeKey) -> WireNodeId {
    WireNodeId::from_digest(*key.as_bytes())
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::digest::{ByteDigest, DigestDomain, DigestRecord};
    use qsl_foundation::SourceIdentity;
    use quire_exact::{Identifier, ScalarLimits};

    use super::{call_site, CallSite, CallSiteRefusal, FunctionSite};
    use crate::identity::QualifiedName;
    use crate::request::{ReplayRequestWire, StateEnvironment};
    use crate::spine::DependencyInput;
    use crate::witness::{CanonicalAssignment, ReplaySource};

    /// The TC-452 fixture unit `F`: one function `f` of one parameter `x`.
    const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\";\n\
        function f using v(x: Int[0, 9]): Integer pure { x + 5 }\n";

    /// A unit of one Boolean predicate `p` of one parameter `x`. `f` above
    /// returns `Integer`, which `replay` refuses `NotAPredicate` before it
    /// ever joins arguments, so it cannot exercise whether `call_site`'s
    /// node id is the one `replay` actually accepts; `p` can.
    const PREDICATE_UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\";\n\
        function p using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";

    fn name(segment: &str) -> QualifiedName {
        QualifiedName::new(vec![Identifier::new(segment).unwrap()]).unwrap()
    }

    /// `call_site` over `bytes`, a standalone unit with no domain package
    /// and no dependency, selecting the function `function`.
    fn function_site(
        bytes: &[u8],
        function: &str,
    ) -> Result<CallSite<FunctionSite>, Box<CallSiteRefusal>> {
        call_site(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            bytes,
            [],
            &DependencyInput::default(),
            &name(function),
        )
    }

    fn scalar_limits(seed: u64) -> ScalarLimits {
        ScalarLimits {
            integer_bits: seed,
            decimal_digits: seed,
            scale_expansion: seed,
            text_input_bytes: seed,
            text_scalars: seed,
            normalized_scalars: seed,
            unit_edges: seed,
            value_occurrences: seed,
            work_units: seed,
            result_units: seed,
        }
    }

    /// A `ReplayRequestWire` replaying `function` over `bytes`, keyed by
    /// `assignments` -- built entirely from `call_site`'s own
    /// `package_id`/parameter pairs plus fixed constants, never by
    /// re-running the node lookup `call_site` itself runs.
    fn wire(
        bytes: &[u8],
        package_id: DigestRecord,
        function: QualifiedName,
        assignments: Vec<CanonicalAssignment>,
    ) -> ReplayRequestWire {
        let source_digest_record = DigestRecord::mint(
            DigestDomain::SourceBytesV1,
            ByteDigest::of(bytes).as_bytes(),
        );
        ReplayRequestWire {
            profile_selections: vec![],
            package_id: (
                Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
                package_id.hex(),
            ),
            source_digests: vec![(
                "a".to_owned(),
                "u".to_owned(),
                "git".to_owned(),
                "1".to_owned(),
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                source_digest_record.hex(),
            )],
            dependencies: Vec::new(),
            selected_function: function,
            source: ReplaySource::Input(assignments),
            obligation_identity: [0; 32],
            backend: "kani-backend-1".to_owned(),
            state_environment: StateEnvironment::new(vec![]),
            accounting_limits: scalar_limits(u64::MAX),
            stage_limits: std::collections::BTreeMap::new(),
            declared_domains: Vec::new(),
            byte_provision: vec![(
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                source_digest_record.hex(),
                bytes.to_vec(),
            )],
        }
    }

    /// `call_site` names `p`'s one parameter `x`
    /// paired with its declared name, and the node id of that pair is the
    /// one the real replay executor accepts -- proved by driving `replay`
    /// with a request keyed by that exact pair, not by re-running the same
    /// lookup `call_site` uses and comparing.
    #[trace("TC-516", "FR-121-AC-1")]
    #[test]
    fn call_site_names_the_parameter_pair_the_real_replay_executor_accepts() {
        let bytes = PREDICATE_UNIT.as_bytes();
        let site = function_site(bytes, "p").expect("the fixture unit compiles and names p");
        let [(identifier, node_id)] = &site.site.parameters[..] else {
            panic!("p declares one parameter: {site:?}");
        };
        assert_eq!(identifier.as_str(), "x");

        let request = wire(
            bytes,
            site.package_id,
            name("p"),
            vec![CanonicalAssignment {
                parameter: *node_id,
                value: crate::witness::WitnessValue::Integer(3),
            }],
        );
        let result = crate::replay(request, crate::ReplayLimits::default());
        assert!(
            result.is_ok(),
            "call_site's node id for x must be the one replay accepts: {result:?}"
        );
    }

    /// A well-formed name that resolves to nothing in the compiled package
    /// refuses `UnknownFunction`, not a panic or a silent empty result,
    /// naming the exact selection and the compiled package (TC-166 step 3,
    /// at `call_site`): an undeclared name, a name equal to `f` but for
    /// case, and a qualified name whose last segment is `f`. None of them
    /// falls back to a display-name match against `f`, which resolves.
    #[trace("TC-516", "TC-166", "FR-121-AC-2")]
    #[test]
    fn call_site_refuses_an_unknown_function_name() {
        let compiled = function_site(UNIT.as_bytes(), "f")
            .expect("f names a real function of the compiled unit");
        let qualified = QualifiedName::new(vec![
            Identifier::new("module").unwrap(),
            Identifier::new("f").unwrap(),
        ])
        .unwrap();
        for selection in [name("nope"), name("F"), qualified] {
            let refusal = call_site(
                SourceIdentity::new("a", "u", "git", "1"),
                "unit.native",
                UNIT.as_bytes(),
                [],
                &DependencyInput::default(),
                &selection,
            )
            .expect_err("the selection names no function of the compiled package");
            let CallSiteRefusal::UnknownFunction {
                selection: named,
                package,
            } = *refusal
            else {
                panic!("expected UnknownFunction for {selection}, got {refusal:?}");
            };
            assert_eq!(named, selection);
            assert_eq!(package, compiled.package_id);
        }
    }

    /// TC-166 step 2, at `call_site`: a unit declaring `f(x)` and `F(y)`,
    /// whose names differ only by case. Each selection locates its own
    /// function: `f`'s parameter is `x`, `F`'s is `y`, at distinct node ids.
    #[trace("TC-166")]
    #[test]
    fn tc_166_call_site_locates_each_case_variant_function() {
        let unit = "language \"ix:native\" edition \"1-draft\";\n\
            profile v = \"quire.value.complete/v1\";\n\
            function f using v(x: Int[0, 9]): Integer pure { x + 5 }\n\
            function F using v(y: Int[0, 9]): Integer pure { y + 6 }\n";
        let lower = function_site(unit.as_bytes(), "f").expect("f is declared");
        let upper = function_site(unit.as_bytes(), "F").expect("F is declared");
        let [(lower_name, lower_id)] = &lower.site.parameters[..] else {
            panic!("f declares one parameter: {lower:?}");
        };
        let [(upper_name, upper_id)] = &upper.site.parameters[..] else {
            panic!("F declares one parameter: {upper:?}");
        };
        assert_eq!(lower_name.as_str(), "x");
        assert_eq!(upper_name.as_str(), "y");
        assert_ne!(lower_id, upper_id);
    }

    /// FR-121-AC-12: a `CallSite` carries the compiled package's
    /// `quire.checked-package/v2` bytes, exactly the S4 emitter's, and the
    /// RFC 8785 bytes of their `identity_preimage` member digest, under
    /// `quire.package.semantic/v2`, to the `CallSite`'s own `package_id`,
    /// which the bytes' own `package_id` member also spells.
    #[trace("TC-516", "FR-121-AC-12")]
    #[test]
    fn call_site_returns_the_checked_package_bytes_its_package_id_names() {
        let site = function_site(UNIT.as_bytes(), "f").expect("f is a declared function");
        let compiled = crate::spine::compose(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            UNIT.as_bytes(),
            &qsl_semantics::model::intake::package_input([]),
            &DependencyInput::default(),
            crate::spine::SpineLimits::default(),
        )
        .expect("the fixture unit compiles");
        assert_eq!(site.package, compiled.emitted.bytes());

        let wire: serde_json::Value =
            serde_json::from_slice(&site.package).expect("the package bytes are JSON");
        assert_eq!(wire["contract_version"], "quire.checked-package/v2");
        let document =
            quire_canonical::read(&site.package, u64::MAX).expect("the package bytes read");
        let identity_preimage = document
            .root()
            .get("identity_preimage")
            .expect("the package carries an identity preimage");
        let digest =
            quire_canonical::sha256(&identity_preimage, quire_canonical::Limits::new(u64::MAX))
                .expect("the identity preimage is canonical JSON");
        assert_eq!(
            DigestRecord::mint(DigestDomain::PackageSemanticV2, *digest.as_bytes()),
            site.package_id
        );
        assert_eq!(wire["package_id"]["digest"], site.package_id.hex());
    }
}
