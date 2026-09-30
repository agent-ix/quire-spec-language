// SPDX-License-Identifier: AGPL-3.0-or-later
//! A thin CG-facing cut of [`crate::spine::compile`]: the facts a client
//! needs to key a `ReplayRequestWire` or a frame counterexample by node id
//! and package id (ADR-013 O-25, C-11), without reaching
//! `qsl_replay::spine` itself, which CG may not call (ADR-011 §3 FB-05,
//! T-12 rule (a): `spine` is public only for `command`).
//!
//! [`call_site`] compiles one unit against the supplied domain package
//! documents and dependency input, under the default spine stage limits,
//! and locates one selection in the result: the compiled package's own
//! `package_id` and either a named function's parameters, each paired with
//! its own node id, or a named operation's anchor, frame and state clause
//! identities. Every node id is the same `WireNodeId` a
//! `CanonicalAssignment`, a witness transcript or a frame counterexample
//! names, joined by declared identity, never by position (ADR-013 O-25).
//! It builds no [`crate::execute::replay`] call and no
//! [`crate::spine::Call`]; it names a call site, it does not make one.

use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedGraph, CheckedOperationFrame};
use qsl_semantics::model::intake::package_input;
use quire_exact::{Identifier, NodeKey};

use crate::execute::callable_parameter_keys;
use crate::identity::QualifiedName;
use crate::spine::{self, DependencyInput, OperationName};

/// What [`call_site`] locates in the compiled package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CallSiteSelection {
    /// A function, by the one-segment name FR-098's selection resolves.
    Function(QualifiedName),
    /// An operation `M::T::op`, resolved as FR-115's `Frame` selection
    /// resolves it: model alias `M` to its domain package, `T` to that
    /// package's object type, and `op` in `T`'s effective view.
    Operation(OperationName),
}

/// What [`call_site`] returns: the compiled package's own content-addressed
/// identity and the located selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallSite {
    /// The compiled package's `package_id`, as a `quire.package.semantic/v2`
    /// digest record -- the same shape `ReplayRequestWire::package_id` and
    /// a witness envelope's package identity carry.
    pub package_id: DigestRecord,
    /// The located function or operation.
    pub target: CallSiteTarget,
}

/// The located selection, one arm per [`CallSiteSelection`] arm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CallSiteTarget {
    /// A [`CallSiteSelection::Function`]'s parameters, in declared order.
    /// Each pair is the parameter's declared name and the `WireNodeId` a
    /// `CanonicalAssignment::parameter` or a witness transcript names to
    /// bind that parameter's argument (ADR-013 O-25, C-11).
    Function {
        /// The parameters, in declared order.
        parameters: Vec<(Identifier, WireNodeId)>,
    },
    /// A [`CallSiteSelection::Operation`]'s identities.
    Operation(OperationSite),
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
    /// S1 to S4 refused the source, a domain package selection, an import
    /// or the dependency input. Carries the refusal's own rendered message;
    /// the stage and region detail live only in `qsl_replay::spine`, which
    /// this facade does not expose.
    #[error("the source did not compile: {0}")]
    Compile(String),
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
pub fn call_site<'a>(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: impl IntoIterator<Item = &'a [u8]>,
    dependencies: &DependencyInput,
    selection: &CallSiteSelection,
) -> Result<CallSite, Box<CallSiteRefusal>> {
    let compiled = spine::compile(
        source,
        path,
        bytes,
        &package_input(packages),
        dependencies,
        spine::SpineLimits::default(),
    )
    .map_err(|refusal| Box::new(CallSiteRefusal::Compile(refusal.to_string())))?;
    let package_id = compiled.emitted.package_id().record();
    let target = match selection {
        CallSiteSelection::Function(function) => {
            function_site(&compiled.package, function, package_id)?
        }
        CallSiteSelection::Operation(operation) => {
            let graph = compiled.package.graph();
            let (_, frame) = graph
                .resolve_operation(&operation.model, &operation.object, &operation.operation)
                .and_then(|resolved| graph.operation_frame(&resolved))
                .ok_or_else(|| {
                    Box::new(CallSiteRefusal::UnknownOperation {
                        selection: operation.clone(),
                        package: package_id,
                    })
                })?;
            CallSiteTarget::Operation(operation_site(graph, frame)?)
        }
    };
    Ok(CallSite { package_id, target })
}

/// `function`'s parameters in `package`, each paired with its own node id,
/// by the one-segment name lookup and node-key derivation `replay`'s own
/// selection uses.
fn function_site(
    package: &CheckedPackage,
    function: &QualifiedName,
    package_id: DigestRecord,
) -> Result<CallSiteTarget, Box<CallSiteRefusal>> {
    let unknown = || {
        Box::new(CallSiteRefusal::UnknownFunction {
            selection: function.clone(),
            package: package_id,
        })
    };
    let [segment] = function.segments() else {
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
    Ok(CallSiteTarget::Function { parameters })
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
        .map(|clause| {
            let node = wire_id(clause.identity());
            Ok(ClauseSite {
                name: identifier(clause.name(), "declared-clause-name-is-a-valid-identifier")?,
                node,
                occurrence: OccurrenceKey::new(node, clause.claim().clone()),
            })
        })
        .collect::<Result<Vec<_>, Box<CallSiteRefusal>>>()?;
    let frame_node = wire_id(frame.frame());
    Ok(OperationSite {
        anchor: wire_id(frame.anchor()),
        frame: frame_node,
        frame_occurrence: OccurrenceKey::new(frame_node, frame.frame_origin().clone()),
        clauses,
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
    use qsl_foundation::digest::{ByteDigest, DigestDomain, DigestRecord, WireNodeId};
    use qsl_foundation::SourceIdentity;
    use quire_exact::{Identifier, ScalarLimits};

    use super::{call_site, CallSite, CallSiteRefusal, CallSiteSelection, CallSiteTarget};
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
    fn function_site(bytes: &[u8], function: &str) -> Result<CallSite, Box<CallSiteRefusal>> {
        call_site(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            bytes,
            [],
            &DependencyInput::default(),
            &CallSiteSelection::Function(name(function)),
        )
    }

    /// A function selection's parameters.
    fn parameters(site: &CallSite) -> &[(Identifier, WireNodeId)] {
        match &site.target {
            CallSiteTarget::Function { parameters } => parameters,
            CallSiteTarget::Operation(operation) => {
                panic!("a function selection locates a function, got {operation:?}")
            }
        }
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
        let [(identifier, node_id)] = parameters(&site) else {
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
}
