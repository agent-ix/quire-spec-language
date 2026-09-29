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
//! `package_id` and, for that function, its parameters' node ids in
//! declared order, each the same `WireNodeId` a `CanonicalAssignment`'s
//! `parameter` or a witness transcript names. It builds no
//! [`crate::execute::replay`] call and no [`crate::spine::Call`]; it names a
//! call site, it does not make one.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_foundation::SourceIdentity;

use crate::identity::QualifiedName;
use crate::spine;

/// The two facts [`call_site`] returns: a compiled package's own
/// content-addressed identity, and one named function's parameters' node
/// ids in declared order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallSite {
    /// The compiled package's `package_id`, as a `quire.package.semantic/v2`
    /// digest record -- the same shape `ReplayRequestWire::package_id`
    /// carries.
    pub package_id: DigestRecord,
    /// The named function's parameters, in declared order. Each is the
    /// `WireNodeId` a `CanonicalAssignment::parameter` or a witness
    /// transcript names to bind that parameter's argument (ADR-013 O-25,
    /// C-11).
    pub parameters: Vec<WireNodeId>,
}

/// Why [`call_site`] named no call site.
#[derive(Debug, thiserror::Error)]
pub enum CallSiteRefusal {
    /// S1 to S4 refused the source. Carries the refusal's own rendered
    /// message; the stage and region detail live only in
    /// `qsl_replay::spine`, which this facade does not expose.
    #[error("the source did not compile: {0}")]
    Compile(String),
    /// `function` names no function of the compiled package.
    #[error("{0} names no function of the compiled package")]
    UnknownFunction(QualifiedName),
    /// The compiled package's own function node for `function` is not
    /// itself a function node, or its parameter count does not match its
    /// checked signature -- a broken invariant.
    #[error("internal fault in {}: {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
}

/// Compile `bytes` (labelled `source`, displayed as `path`) through S1 to S4
/// with no dependency input and no domain package input, under the default
/// spine stage limits, and locate `function`'s call site in the result. A
/// source that imports a library, selects a domain package, or needs
/// non-default stage limits refuses here (`DependencyInput`); widen this
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

    let unknown = || Box::new(CallSiteRefusal::UnknownFunction(function.clone()));
    let [segment] = function.segments() else {
        return Err(unknown());
    };
    let callable = compiled
        .package
        .graph()
        .callable(segment.as_str())
        .ok_or_else(unknown)?;
    let parameters = compiled
        .package
        .graph()
        .semantic_graph()
        .node(callable.identity)
        .and_then(|node| node.function_parameters())
        .ok_or_else(|| {
            Box::new(CallSiteRefusal::Fault(InternalFault::new(
                "replay",
                "callable-identity-is-a-function-node",
            )))
        })?;
    if parameters.len() != callable.parameters.len() {
        return Err(Box::new(CallSiteRefusal::Fault(InternalFault::new(
            "replay",
            "function-node-parameters-match-signature",
        ))));
    }

    Ok(CallSite {
        package_id: compiled.emitted.package_id().record(),
        parameters: parameters
            .iter()
            .map(|key| WireNodeId::from_digest(*key.as_bytes()))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::{call_site, CallSiteRefusal};
    use crate::identity::QualifiedName;
    use ix_trace_rs::trace;
    use qsl_foundation::SourceIdentity;
    use quire_exact::Identifier;

    /// The TC-452 fixture unit `F`: one function `f` of one parameter `x`.
    const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\" version \"1-draft.2\" digest \
        \"sha256:c8c7ae9fbe783286369ecc83f006190f83be4c3c8fc585766617c90f27a25b16\";\n\
        function f using v(x: Int[0, 9]): Integer pure { x + 5 }\n";

    fn name(segment: &str) -> QualifiedName {
        QualifiedName::new(vec![Identifier::new(segment).unwrap()]).unwrap()
    }

    /// QSL-317: `call_site` returns the same `package_id` `spine::compile`
    /// emits for the identical source, and one parameter node id for `f`'s
    /// one parameter `x`, matching the id `spine::compile`'s own checked
    /// graph assigns that parameter's node.
    #[trace("QSL-317")]
    #[test]
    fn call_site_returns_the_package_id_and_the_named_functions_parameter_node_ids() {
        let source = SourceIdentity::new("a", "u", "git", "1");
        let site = call_site(source.clone(), "unit.native", UNIT.as_bytes(), &name("f"))
            .expect("the fixture unit compiles and names f");

        let compiled = crate::spine::compile(
            source,
            "unit.native",
            UNIT.as_bytes(),
            &std::collections::BTreeMap::new(),
            &crate::spine::DependencyInput::default(),
            crate::spine::SpineLimits::default(),
        )
        .expect("the fixture unit compiles");
        assert_eq!(site.package_id, compiled.emitted.package_id().record());

        let callable = compiled
            .package
            .graph()
            .callable("f")
            .expect("f is callable");
        let expected: Vec<_> = compiled
            .package
            .graph()
            .semantic_graph()
            .node(callable.identity)
            .and_then(|node| node.function_parameters())
            .expect("f is a function node")
            .iter()
            .map(|key| qsl_foundation::digest::WireNodeId::from_digest(*key.as_bytes()))
            .collect();
        assert_eq!(site.parameters, expected);
        assert_eq!(site.parameters.len(), 1);
    }

    /// A name that resolves to nothing in the compiled package refuses
    /// `UnknownFunction`, not a panic or a silent empty result.
    #[trace("QSL-317")]
    #[test]
    fn call_site_refuses_an_unknown_function_name() {
        let source = SourceIdentity::new("a", "u", "git", "1");
        let refusal = call_site(source, "unit.native", UNIT.as_bytes(), &name("nope"))
            .expect_err("nope names no function of the compiled package");
        assert!(matches!(*refusal, CallSiteRefusal::UnknownFunction(_)));
    }
}
