// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-317: a thin CG-facing cut of [`crate::spine::compile`] -- the two
//! facts a client needs to key a `ReplayRequestWire` by parameter node id
//! and package id (ADR-013 O-25, C-11), without reaching `qsl_replay::spine`
//! itself, which CG may not call (ADR-011 §3 FB-05, T-12 rule (a): `spine`
//! is public only for `command`).
//!
//! [`call_site`] compiles a standalone unit -- no dependency input, no
//! domain package input, the default spine stage limits -- and locates one
//! named function's call site in the result: the compiled package's own
//! `package_id` and, for that function, its parameters in declared order,
//! each paired with its own node id -- the same `WireNodeId` a
//! `CanonicalAssignment`'s `parameter` or a witness transcript names, joined
//! by declared identity, never by position (ADR-013 O-25). It builds no
//! [`crate::execute::replay`] call and no [`crate::spine::Call`]; it names a
//! call site, it does not make one.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_foundation::SourceIdentity;
use quire_exact::Identifier;

use crate::execute::callable_parameter_keys;
use crate::identity::QualifiedName;
use crate::spine;

/// The two facts [`call_site`] returns: a compiled package's own
/// content-addressed identity, and one named function's parameters, each
/// paired with its own node id in declared order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallSite {
    /// The compiled package's `package_id`, as a `quire.package.semantic/v2`
    /// digest record -- the same shape `ReplayRequestWire::package_id`
    /// carries.
    pub package_id: DigestRecord,
    /// The named function's parameters, in declared order. Each pair is the
    /// parameter's declared name and the `WireNodeId` a
    /// `CanonicalAssignment::parameter` or a witness transcript names to
    /// bind that parameter's argument -- joined by declared identity, never
    /// by position (ADR-013 O-25, C-11).
    pub parameters: Vec<(Identifier, WireNodeId)>,
}

/// Why [`call_site`] named no call site.
#[derive(Debug, thiserror::Error)]
pub enum CallSiteRefusal {
    /// S1 to S4 refused the source. Carries the refusal's own rendered
    /// message; the stage and region detail live only in
    /// `qsl_replay::spine`, which this facade does not expose.
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
    /// The compiled package's own function node for `function` is not
    /// itself a function node, its parameter count does not match its
    /// checked signature, or one of its declared parameter names is not
    /// itself a valid `Identifier` -- a broken invariant.
    #[error("internal fault in {}: {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
}

/// Compile `bytes` (labelled `source`, displayed as `path`) through S1 to S4
/// with no dependency input and no domain package input, under the default
/// spine stage limits, and locate `function`'s call site in the result. A
/// source that imports a library, selects a domain package, or needs
/// non-default stage limits refuses here as `Compile`, carrying the
/// import's or the compile's own rendered refusal message; widen this
/// entry, not the CG call site, when a client needs one of those.
pub fn call_site(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    function: &QualifiedName,
) -> Result<CallSite, Box<CallSiteRefusal>> {
    let compiled = spine::compile(
        source,
        path,
        bytes,
        &BTreeMap::new(),
        &spine::DependencyInput::default(),
        spine::SpineLimits::default(),
    )
    .map_err(|refusal| Box::new(CallSiteRefusal::Compile(refusal.to_string())))?;
    let package_id = compiled.emitted.package_id().record();

    let unknown = || {
        Box::new(CallSiteRefusal::UnknownFunction {
            selection: function.clone(),
            package: package_id,
        })
    };
    let [segment] = function.segments() else {
        return Err(unknown());
    };
    let callable = compiled
        .package
        .graph()
        .callable(segment.as_str())
        .ok_or_else(unknown)?;
    let keys = callable_parameter_keys(&compiled.package, &callable)
        .map_err(|fault| Box::new(CallSiteRefusal::Fault(fault)))?;

    let parameters = callable
        .parameters
        .iter()
        .zip(keys.iter())
        .map(|((name, _value_type), key)| {
            let identifier = Identifier::new(name).map_err(|_| {
                Box::new(CallSiteRefusal::Fault(InternalFault::new(
                    "replay",
                    "declared-parameter-name-is-a-valid-identifier",
                )))
            })?;
            Ok((identifier, WireNodeId::from_digest(*key.as_bytes())))
        })
        .collect::<Result<Vec<_>, Box<CallSiteRefusal>>>()?;

    Ok(CallSite {
        package_id,
        parameters,
    })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::digest::{ByteDigest, DigestDomain, DigestRecord};
    use qsl_foundation::SourceIdentity;
    use quire_exact::{Identifier, ScalarLimits};

    use super::{call_site, CallSiteRefusal};
    use crate::identity::QualifiedName;
    use crate::request::{ReplayRequestWire, StageLimits, StateEnvironment};
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

    /// QSL-317 (SR-780 FND-001): `call_site` names `p`'s one parameter `x`
    /// paired with its declared name, and the node id of that pair is the
    /// one the real replay executor accepts -- proved by driving `replay`
    /// with a request keyed by that exact pair, not by re-running the same
    /// lookup `call_site` uses and comparing.
    #[trace("TC-516", "FR-121-AC-1")]
    #[test]
    fn call_site_names_the_parameter_pair_the_real_replay_executor_accepts() {
        let source = SourceIdentity::new("a", "u", "git", "1");
        let bytes = PREDICATE_UNIT.as_bytes();
        let site = call_site(source, "unit.native", bytes, &name("p"))
            .expect("the fixture unit compiles and names p");
        assert_eq!(site.parameters.len(), 1);
        let (identifier, node_id) = site.parameters[0].clone();
        assert_eq!(identifier.as_str(), "x");

        let request = wire(
            bytes,
            site.package_id,
            name("p"),
            vec![CanonicalAssignment {
                parameter: node_id,
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
        let source = SourceIdentity::new("a", "u", "git", "1");
        let compiled = call_site(source.clone(), "unit.native", UNIT.as_bytes(), &name("f"))
            .expect("f names a real function of the compiled unit");
        let refusal = call_site(source, "unit.native", UNIT.as_bytes(), &name("nope"))
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
