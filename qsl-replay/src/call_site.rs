// SPDX-License-Identifier: AGPL-3.0-or-later
//! A thin CG-facing cut of [`crate::spine::compile`]: the facts a client
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
//! or a named state clause's identities. It also returns the compiled
//! package's own `quire.checked-package/v2` bytes. Every node id is the same
//! `WireNodeId` a `CanonicalAssignment`, a witness transcript or a
//! counterexample names, joined by declared identity, never by position (ADR-013 O-25).
//! It builds no [`crate::execute::replay`] call and no
//! [`crate::spine::Call`]; it names a call site, it does not make one.

use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedGraph, CheckedOperationFrame, CheckedStateClause};
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::intake::package_input;
use quire_exact::{Identifier, NodeKey};

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
///   `ClauseSelection` names it, and locates a [`ClauseSite`].
///
/// Sealed: these three are the selections.
pub trait CallSiteSelection: sealed::Locate {}

impl CallSiteSelection for QualifiedName {}
impl CallSiteSelection for OperationName {}
impl CallSiteSelection for ClauseName {}

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
/// [`FunctionSite`], an [`OperationSite`] or a [`ClauseSite`].
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
    /// Carries the refusal's own rendered message; the stage and region
    /// detail live only in `qsl_replay::spine`, which this facade does not
    /// expose.
    #[error("the source did not compile: {0}")]
    Compile(String),
    /// A `model` declaration of the unit selects a domain package that the
    /// supplied `packages` do not admit (I1).
    #[error("domain package intake refused `model {alias}`: {message}")]
    ModelIntake {
        /// The `model` declaration's alias.
        alias: String,
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
    /// cycle or a diamond. Carries the resolution's rendered message.
    #[error("{0}")]
    Import(String),
    /// A supplied library, resolved through the unit's imports, did not
    /// compile.
    #[error("the library {} refused: {message}", display_path(.path))]
    Dependency {
        /// The library identities from the unit's import down to the
        /// library that refused, that library last.
        path: Vec<LibraryName>,
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
    /// A broken invariant: the compiled package's own function node for a
    /// selected function is not itself a function node, or its parameter
    /// count does not match its checked signature, or a declared parameter
    /// or clause name is not itself a valid `Identifier`.
    #[error("internal fault in {}: {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
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
/// #     profile v = \"quire.value.complete/v1\" version \"1-draft.2\" digest \
/// #     \"sha256:c8c7ae9fbe783286369ecc83f006190f83be4c3c8fc585766617c90f27a25b16\";\n\
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
/// #     profile v = \"quire.value.complete/v1\" version \"1-draft.2\" digest \
/// #     \"sha256:c8c7ae9fbe783286369ecc83f006190f83be4c3c8fc585766617c90f27a25b16\";\n\
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
    let compiled = spine::compile(
        source,
        path,
        bytes,
        &package_input(packages),
        dependencies,
        spine::SpineLimits::default(),
    )
    .map_err(|refusal| Box::new(CallSiteRefusal::from(*refusal)))?;
    let package_id = compiled.emitted.package_id().record();
    let site = selection.locate(&compiled.package, package_id)?;
    Ok(CallSite {
        package_id,
        package: compiled.emitted.bytes().to_vec(),
        site,
    })
}

impl From<CompileRefusal> for CallSiteRefusal {
    /// Sort a spine compile refusal by whose input it concerns: the unit's
    /// own source, the domain packages, the dependency input, an import, or
    /// a supplied library.
    fn from(refusal: CompileRefusal) -> Self {
        match refusal {
            CompileRefusal::Intake { refusal, .. } => Self::ModelIntake {
                message: spine::intake_message(&refusal.cause),
                alias: refusal.alias,
            },
            CompileRefusal::DependencyInput(input) => Self::DependencyInput(input),
            CompileRefusal::Import { refusal, .. } => Self::Import(refusal.to_string()),
            CompileRefusal::Dependency { path, refusal } => Self::Dependency {
                path,
                message: refusal.to_string(),
            },
            refusal @ (CompileRefusal::Source(_)
            | CompileRefusal::Forms { .. }
            | CompileRefusal::Profile { .. }
            | CompileRefusal::Assembly { .. }
            | CompileRefusal::Check { .. }
            | CompileRefusal::Link(_)
            | CompileRefusal::Emit(_)
            | CompileRefusal::Omitted(_)) => Self::Compile(refusal.to_string()),
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
        Ok(FunctionSite { parameters })
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
    use crate::request::{ReplayRequestWire, StageLimits, StateEnvironment};
    use crate::spine::DependencyInput;
    use crate::witness::{CanonicalAssignment, ReplaySource};

    /// The TC-452 fixture unit `F`: one function `f` of one parameter `x`.
    const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\" version \"1-draft.2\" digest \
        \"sha256:c8c7ae9fbe783286369ecc83f006190f83be4c3c8fc585766617c90f27a25b16\";\n\
        function f using v(x: Int[0, 9]): Integer pure { x + 5 }\n";

    /// A unit of one Boolean predicate `p` of one parameter `x`. `f` above
    /// returns `Integer`, which `replay` refuses `NotAPredicate` before it
    /// ever joins arguments, so it cannot exercise whether `call_site`'s
    /// node id is the one `replay` actually accepts; `p` can.
    const PREDICATE_UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\" version \"1-draft.2\" digest \
        \"sha256:c8c7ae9fbe783286369ecc83f006190f83be4c3c8fc585766617c90f27a25b16\";\n\
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

    fn stage_limits() -> StageLimits {
        let s1 = ScalarLimits {
            text_input_bytes: u64::try_from(crate::bounds::MAX_ENCODED_BYTES).unwrap(),
            ..scalar_limits(u64::MAX)
        };
        StageLimits {
            s1,
            s2: scalar_limits(u64::MAX),
            s3: scalar_limits(u64::MAX),
            s4: scalar_limits(u64::MAX),
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
            contract_version: "quire.native-runtime/v1".to_owned(),
            capability_vocabulary: Some("quire.capability-kind/v1".to_owned()),
            profile_selections: vec![],
            package_id: (
                Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
                package_id.hex(),
            ),
            package_contract_version: "quire.checked-package/v2".to_owned(),
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
            originating_counterexample_identity: [0; 32],
            backend: (
                "kani-backend-1".to_owned(),
                Some(DigestDomain::ToolManifestJcsV1.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::ToolManifestJcsV1, [0; 32]).hex(),
            ),
            state_environment: StateEnvironment::new(vec![]),
            accounting_limits: scalar_limits(u64::MAX),
            stage_limits: stage_limits(),
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
        let result = crate::replay(request);
        assert!(
            result.is_ok(),
            "call_site's node id for x must be the one replay accepts: {result:?}"
        );
    }

    /// A name that resolves to nothing in the compiled package refuses
    /// `UnknownFunction`, not a panic or a silent empty result.
    #[trace("TC-516", "FR-121-AC-2")]
    #[test]
    fn call_site_refuses_an_unknown_function_name() {
        let compiled = function_site(UNIT.as_bytes(), "f")
            .expect("f names a real function of the compiled unit");
        let refusal = function_site(UNIT.as_bytes(), "nope")
            .expect_err("nope names no function of the compiled package");
        match *refusal {
            CallSiteRefusal::UnknownFunction { selection, package } => {
                assert_eq!(selection, name("nope"));
                assert_eq!(package, compiled.package_id);
            }
            other => panic!("expected UnknownFunction, got {other:?}"),
        }
    }

    /// TC-166 step 3, at `call_site`: a well-formed `QualifiedName` that
    /// names no function of the compiled package refuses `UnknownFunction`,
    /// naming the exact selection and the compiled package -- an undeclared
    /// name, a name equal to `f` but for case, and a qualified name whose
    /// last segment is `f`. None of them falls back to a display-name match
    /// against `f`, which resolves.
    #[trace("TC-166")]
    #[test]
    fn tc_166_call_site_refuses_an_unresolvable_qualified_name() {
        let compiled = function_site(UNIT.as_bytes(), "f").expect("f resolves");
        let qualified = QualifiedName::new(vec![
            Identifier::new("module").unwrap(),
            Identifier::new("f").unwrap(),
        ])
        .unwrap();
        for selection in [name("g"), name("F"), qualified] {
            let refusal = call_site(
                SourceIdentity::new("a", "u", "git", "1"),
                "unit.native",
                UNIT.as_bytes(),
                [],
                &DependencyInput::default(),
                &selection,
            )
            .expect_err("the selection names no function of the package");
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

    /// FR-121-AC-12: a `CallSite` carries the compiled package's
    /// `quire.checked-package/v2` bytes, exactly the S4 emitter's, and the
    /// RFC 8785 bytes of their `identity_preimage` member digest, under
    /// `quire.package.semantic/v2`, to the `CallSite`'s own `package_id`,
    /// which the bytes' own `package_id` member also spells.
    #[trace("TC-516", "FR-121-AC-12")]
    #[test]
    fn call_site_returns_the_checked_package_bytes_its_package_id_names() {
        let site = function_site(UNIT.as_bytes(), "f").expect("f is a declared function");
        let compiled = crate::spine::compile(
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
        let limits = quire_canonical::Limits::new(u64::MAX, quire_canonical::Limits::MAX_DEPTH)
            .expect("MAX_DEPTH is within MAX_DEPTH");
        let digest = quire_canonical::sha256(&wire["identity_preimage"], limits)
            .expect("the identity preimage is canonical JSON");
        assert_eq!(
            DigestRecord::mint(DigestDomain::PackageSemanticV2, *digest.as_bytes()),
            site.package_id
        );
        assert_eq!(wire["package_id"]["digest"], site.package_id.hex());
    }
}
