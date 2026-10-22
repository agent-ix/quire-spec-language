// SPDX-License-Identifier: AGPL-3.0-or-later
//! The facade's same-run plugin replay context (FR-404): run one call as
//! [`run_with_requirements`](crate::run_with_requirements) does and return,
//! from that same compile, a [`CallReplayContext`] bound to the run.
//!
//! The context wraps a [`PluginReplayContext`] built from the run's own
//! units and limits, and names an item by FR-395's pairing key, the run's
//! `package_id` and a member's request index. A consumer reaches the entry
//! and the context through the `qsl_replay` root and never names
//! [`crate::spine`] (FR-060 T12-A).

use std::collections::BTreeMap;

use qsl_foundation::bound::ProofBound;
use qsl_foundation::{BackendId, RequestIndex, SourceIdentity};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::Cancel;

use crate::call_requirements::{associate, CallRequirement, CallRequirements};
use crate::limits::CallerLimits;
use crate::proof_result::{PluginReplayContext, PluginReplayRefusal};
use crate::request::ReplayRequestWire;
use crate::spine::{run_reading, Call, CallOutcome, DependencyInput, RunRefusal};
use crate::witness::CanonicalAssignment;

/// The plugin replay context of one run (FR-404): it builds a replay request
/// only for an item of its own run's call. It has no public constructor, no
/// public field and no way to reach the wrapped [`PluginReplayContext`], so a
/// consumer obtains one only from [`run_with_replay_context`] (FR-404-AC-9).
/// None of these compiles:
///
/// ```compile_fail
/// let _ = qsl_replay::CallReplayContext { package_id: todo!(), context: todo!(), members: todo!() };
/// ```
///
/// ```compile_fail
/// let _ = qsl_replay::CallReplayContext::new();
/// ```
///
/// ```compile_fail
/// let _ = qsl_replay::CallReplayContext::default();
/// ```
///
/// ```compile_fail
/// let _ = serde_json::from_str::<qsl_replay::CallReplayContext>("{}");
/// ```
///
/// ```compile_fail
/// fn unbound(context: qsl_replay::CallReplayContext) {
///     let _: &qsl_replay::PluginReplayContext = &*context;
/// }
/// ```
///
/// ```compile_fail
/// fn unbound(context: qsl_replay::CallReplayContext) {
///     let _: &qsl_replay::PluginReplayContext = context.as_ref();
/// }
/// ```
#[derive(Clone, Debug)]
pub struct CallReplayContext {
    package_id: PackageId,
    context: PluginReplayContext,
    members: Vec<CallRequirement>,
}

impl CallReplayContext {
    /// The `package_id` of the run that produced this context.
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }

    /// The `ReplayRequestWire` for the item at the pairing key
    /// (`package_id`, `request_index`): FR-290's
    /// [`PluginReplayContext::request`] at the member's occurrence key.
    ///
    /// # Errors
    ///
    /// The first [`PluginReplayRefusal`], in this order: `ForeignRun` when
    /// `package_id` is not the run's; `NotACallItem` when `request_index` is
    /// no member's; then FR-290's `NotReplayable`, `UnresolvedDependency`,
    /// `DuplicateBoundKey` and `Fault`.
    pub fn request(
        &self,
        package_id: PackageId,
        request_index: RequestIndex,
        proof_bounds: &[ProofBound],
        plugin: &BackendId,
        assignments: Vec<CanonicalAssignment>,
    ) -> Result<ReplayRequestWire, PluginReplayRefusal> {
        if package_id != self.package_id {
            return Err(PluginReplayRefusal::ForeignRun {
                received: package_id,
                run: self.package_id,
            });
        }
        let member = self
            .members
            .iter()
            .find(|member| member.request_index() == request_index)
            .ok_or(PluginReplayRefusal::NotACallItem { request_index })?;
        self.context
            .request(member.occurrence(), proof_bounds, plugin, assignments)
    }
}

/// What [`run_with_replay_context`] returns: `run`'s `package_id` and
/// outcome, the selected call's [`CallRequirements`] and the run's
/// [`CallReplayContext`], all from one compile.
#[derive(Debug)]
pub struct RunWithReplayContext {
    package_id: PackageId,
    outcome: CallOutcome,
    requirements: CallRequirements,
    context: CallReplayContext,
}

impl RunWithReplayContext {
    /// The compiled package's id, as `run` returns it for the same inputs.
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }

    /// The call's outcome, as `run` returns it for the same inputs.
    pub fn outcome(&self) -> &CallOutcome {
        &self.outcome
    }

    /// The selected call's requirement items.
    pub fn requirements(&self) -> &CallRequirements {
        &self.requirements
    }

    /// The run's replay context.
    pub fn context(&self) -> &CallReplayContext {
        &self.context
    }

    /// The package id, the outcome, the requirement items and the context.
    pub fn into_parts(self) -> (PackageId, CallOutcome, CallRequirements, CallReplayContext) {
        (
            self.package_id,
            self.outcome,
            self.requirements,
            self.context,
        )
    }
}

/// [`run_with_requirements`](crate::run_with_requirements) that also returns
/// the run's [`CallReplayContext`] (FR-404). The source compiles once; the
/// outcome, `package_id`, members and context come from that compile. The
/// run's `SpineLimits` are `limits.spine`; the context copies the rest of
/// `limits` into each request it builds.
///
/// The refusals are `run_with_requirements`' for `limits.spine`, and a
/// refusal returns no context and no members.
#[allow(
    clippy::too_many_arguments,
    reason = "run's signature (FR-100): the request's fields, the object context and the cancel handle"
)]
pub fn run_with_replay_context(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    limits: CallerLimits,
    call: &Call,
    objects: &ObjectEnvironment,
    cancel: &Cancel,
) -> Result<RunWithReplayContext, Box<RunRefusal>> {
    let (package_id, outcome, (members, context)) = run_reading(
        source,
        path,
        bytes,
        packages,
        dependencies,
        limits.spine,
        call,
        objects,
        cancel,
        |checked, emitted, models| {
            let graph = checked.package().graph();
            let members = associate(graph.requirements(), graph, &call.function)
                .map_err(|fault| Box::new(RunRefusal::Fault(fault)))?;
            let context = PluginReplayContext::new(checked, emitted, models, limits);
            Ok((members, context))
        },
    )?;
    Ok(RunWithReplayContext {
        package_id,
        outcome,
        requirements: CallRequirements::new(package_id, call, members.clone()),
        context: CallReplayContext {
            package_id,
            context,
            members,
        },
    })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBound};
    use qsl_foundation::digest::WireNodeId;
    use qsl_foundation::source::provenance::OccurrenceKey;
    use quire_exact::{Integer, Origin, Role, Value};

    use super::*;
    use crate::execute::callable_parameter_keys;
    use crate::spine::{self, default_accounting, ParseRequest};
    use crate::witness::WitnessValue;
    use crate::ReplayLimits;
    use crate::{TerminalRecord, TerminalValue};
    use qsl_foundation::SettingLimits;

    const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\";\n\
        function p using v(x: Int[0, 9]): Boolean pure { x > 3 }\n\
        function q using v(x: Int[0, 9]): Integer pure { x + 1 }\n";

    fn plugin() -> BackendId {
        BackendId::new("acme")
    }

    fn run(source: &str, limits: CallerLimits) -> RunWithReplayContext {
        let call = Call {
            function: "q".to_owned(),
            arguments: vec![crate::CallArgument {
                parameter: "x".to_owned(),
                value: Value::Integer(Integer::from(3_i64)),
            }],
            accounting: default_accounting(1_000_000),
            supplied: crate::SuppliedLimits::default(),
        };
        run_with_replay_context(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            source.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            limits,
            &call,
            &ObjectEnvironment::default(),
            &Cancel::new(),
        )
        .expect("the call runs")
    }

    /// `p`'s declaration occurrence and its parameter's node, read through
    /// a separate compile of `source`.
    fn declaration(source: &str, function: &str) -> (OccurrenceKey, WireNodeId) {
        let identity = SourceIdentity::new("a", "u", "git", "1");
        let limits = CallerLimits::default().spine;
        let cancel = Cancel::new();
        let parsed = spine::parse(
            &ParseRequest {
                source: &identity,
                path: "unit.native",
                bytes: source.as_bytes(),
            },
            limits.source,
            &cancel,
        )
        .expect("parses")
        .into_value();
        let models = spine::select(&parsed, &BTreeMap::new(), limits.model, &cancel)
            .expect("selects")
            .into_value();
        let checked = spine::check(
            &parsed,
            &models,
            &DependencyInput::default(),
            &spine::LockEvidence::default(),
            limits,
            &cancel,
        )
        .expect("checks")
        .into_value();
        let callable = checked
            .package()
            .graph()
            .callable(function)
            .expect("declared");
        let parameter =
            callable_parameter_keys(checked.package(), &callable).expect("a function node")[0];
        (
            OccurrenceKey::new(
                WireNodeId::from_digest(*callable.identity.as_bytes()),
                Origin::new(Role::new("declaration"), 0),
            ),
            WireNodeId::from_digest(*parameter.as_bytes()),
        )
    }

    fn bound_at(node: WireNodeId) -> ProofBound {
        ProofBound::new(
            DomainKey::Node { node, path: vec![] },
            Some(DomainKind::Integer),
            FiniteBound::integer_range(Integer::from(0_i64), Integer::from(9_i64)).unwrap(),
        )
        .unwrap()
    }

    fn input(parameter: WireNodeId, x: i128) -> Vec<CanonicalAssignment> {
        vec![CanonicalAssignment {
            parameter,
            value: WitnessValue::Integer(x),
        }]
    }

    /// The run's context with its one member moved to `p`'s function-contract
    /// item, the only kind of item `PluginReplayContext::request` replays
    /// until application claims do (QSL-980).
    fn at_p(result: RunWithReplayContext, key: OccurrenceKey) -> CallReplayContext {
        let mut context = result.into_parts().3;
        context.members = vec![CallRequirement::new(key, RequestIndex::new(0))];
        context
    }

    /// FR-404-AC-2, -5, -6: a member's request carries the run's package, its
    /// source and the run's limits, and replays and settles.
    #[trace("TC-1173", "FR-404-AC-2")]
    #[trace("TC-1173", "FR-404-AC-5")]
    #[trace("TC-1173", "FR-404-AC-6")]
    #[test]
    fn a_member_request_carries_the_run_and_replays() {
        let limits = CallerLimits::from_operands([
            "s1.tokens=200000",
            "library.definitions=9000",
            "work_units=5000",
        ])
        .unwrap();
        let result = run(SOURCE, limits);
        let package_id = result.package_id();
        let (key, parameter) = declaration(SOURCE, "p");
        let context = at_p(result, key.clone());
        let bounds = [bound_at(parameter)];
        let wire = context
            .request(
                package_id,
                RequestIndex::new(0),
                &bounds,
                &plugin(),
                input(parameter, 0),
            )
            .expect("a function-contract item");
        assert_eq!(wire.package_id.1, package_id.hex());
        assert_eq!(wire.replay_target.occurrence_key.as_ref(), Some(&key));
        assert_eq!(wire.byte_provision[0].2, SOURCE.as_bytes());
        assert_eq!(wire.stage_limits["s1.tokens"], 200_000);
        assert_eq!(wire.stage_limits["library.definitions"], 9000);
        assert_eq!(wire.accounting_limits.work_units, 5000);
        for (setting, bound) in CallerLimits::default().bounds() {
            let name = setting.name();
            if !matches!(
                name,
                "s1.tokens" | "library.definitions" | "replay.input_bytes" | "format.output_bytes"
            ) {
                assert_eq!(wire.stage_limits.get(name), Some(&bound), "{name}");
            }
        }
        let replayed = crate::replay(wire, ReplayLimits::default());
        let refuted =
            TerminalRecord::from_plugin_counterexample(RequestIndex::new(0), plugin(), &replayed);
        assert_eq!(refuted.value(), &TerminalValue::Refuted);
        assert_eq!(refuted.backend(), Some(&plugin()));

        // Another package's id is refused before the item is looked at.
        let other = SOURCE.replace("x + 1", "x + 2");
        let foreign = run(&other, CallerLimits::default()).package_id();
        assert_ne!(foreign, package_id);
        assert!(matches!(
            context
                .request(
                    foreign,
                    RequestIndex::new(0),
                    &bounds,
                    &plugin(),
                    Vec::new()
                )
                .err(),
            Some(PluginReplayRefusal::ForeignRun { .. })
        ));
    }

    /// FR-404-AC-5: two packages that hold `p` hold equal keys for it, and
    /// each context's request carries its own package and source.
    #[trace("TC-1173", "FR-404-AC-5")]
    #[test]
    fn equal_keys_in_two_packages_carry_each_their_own_package() {
        let other = format!("{SOURCE}function s using v(x: Int[0, 9]): Integer pure {{ x + 2 }}\n");
        let (key, parameter) = declaration(SOURCE, "p");
        assert_eq!((key.clone(), parameter), declaration(&other, "p"));
        let mut digests = Vec::new();
        for source in [SOURCE, other.as_str()] {
            let result = run(source, CallerLimits::default());
            let package_id = result.package_id();
            let context = at_p(result, key.clone());
            let wire = context
                .request(
                    package_id,
                    RequestIndex::new(0),
                    &[bound_at(parameter)],
                    &plugin(),
                    input(parameter, 0),
                )
                .expect("a request");
            assert_eq!(wire.package_id.1, package_id.hex());
            assert_eq!(wire.byte_provision[0].2, source.as_bytes());
            digests.push(wire.source_digests[0].clone());
        }
        assert_ne!(digests[0], digests[1]);
    }
}
