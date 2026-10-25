// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-485: in-process checked-closure acquisition and application through the replay facade.

use std::collections::BTreeSet;
use std::sync::Arc;

use qsl_eval::value::PackageClosure;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::CheckedExpression;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::{
    Admission, BodyFactory, CallFailure, Evaluation, Progress, Registry, Session, Signature,
};
use quire_exact::{Meter, NodeKey, Value};
use quire_semantic_value::object_closure::ObjectClosure;

use crate::admitted::{context_of, AdmittedContext, AdmittedRefusal};
use crate::bounds::ReplayLimits;
use crate::compile::{recompile_unit, CompiledPackage};
use crate::request::StageLimits;
use crate::spine::DependencyInput;
use crate::DigestRecord;

type Adapter = PackageClosure<Arc<CheckedPackage>, Arc<CheckedExpression>>;

/// An emitted package, its admitted tables, and its retained checked closure.
pub struct CheckedClosureCompilation {
    package: CompiledPackage,
    context: AdmittedContext,
    handle: CheckedClosureHandle,
}

impl CheckedClosureCompilation {
    /// The same emitted package that `compile_admitted` returns.
    pub fn package(&self) -> &CompiledPackage {
        &self.package
    }

    /// The same admitted tables that `compile_admitted` returns.
    pub fn context(&self) -> &AdmittedContext {
        &self.context
    }

    /// The retained checked closure, valid independently of these public values.
    pub fn handle(&self) -> &CheckedClosureHandle {
        &self.handle
    }

    /// Separate the three independently owned results.
    pub fn into_parts(self) -> (CompiledPackage, AdmittedContext, CheckedClosureHandle) {
        (self.package, self.context, self.handle)
    }
}

/// The checked package and exact linked dependency closure of one acquisition.
#[derive(Clone)]
pub struct CheckedClosureHandle {
    package: Arc<CheckedPackage>,
    package_id: DigestRecord,
    acquisition: Arc<()>,
}

impl CheckedClosureHandle {
    /// The emitted unit's exact package identity.
    pub fn package_id(&self) -> DigestRecord {
        self.package_id
    }

    /// Look up a check-minted callable in the selected owner of this closure.
    pub fn lookup_callable(
        &self,
        owner_package_id: DigestRecord,
        wire_node_id: WireNodeId,
    ) -> Result<CallableSelection, CallableLookupRefusal> {
        let mut packages = vec![(self.package_id, self.package.as_ref())];
        let mut visited = BTreeSet::new();
        let mut selected = None;
        let mut other_owner = None;
        let mut ambiguous = false;
        while let Some((id, package)) = packages.pop() {
            if !visited.insert(id) {
                continue;
            }
            if id == owner_package_id {
                selected = Some(package);
            } else if has_node(package, wire_node_id) {
                if other_owner.replace(id).is_some() {
                    ambiguous = true;
                }
            }
            packages.extend(
                package
                    .dependencies()
                    .iter()
                    .map(|(id, dependency)| (id.record(), dependency.as_ref())),
            );
        }
        let Some(package) = selected else {
            return Err(CallableLookupRefusal::Unknown);
        };
        {
            if let Some(key) = package
                .graph()
                .semantic_graph()
                .nodes()
                .map(|node| node.key())
                .find(|key| key.as_bytes() == wire_node_id.as_bytes())
            {
                if let Some(function) = package.graph().callable_by_identity(key) {
                    return Ok(CallableSelection {
                        owner_package_id,
                        key,
                        signature: Signature::new(
                            function
                                .parameters
                                .iter()
                                .map(|(_, ty)| ty.clone())
                                .collect::<Vec<_>>(),
                            function.result.clone(),
                        ),
                        acquisition: Arc::clone(&self.acquisition),
                    });
                }
                return Err(CallableLookupRefusal::WrongKind);
            }
        }
        match (other_owner, ambiguous) {
            (Some(owner), false) => Err(CallableLookupRefusal::Foreign(owner)),
            _ => Err(CallableLookupRefusal::Unknown),
        }
    }

    /// Create a fresh registry over the designated QSL adapter and leaf scheduler.
    pub fn registry(&self) -> ApplicationRegistry {
        ApplicationRegistry {
            acquisition: Arc::clone(&self.acquisition),
            inner: Registry::new(PackageClosure::new(Arc::clone(&self.package))),
        }
    }
}

fn has_node(package: &CheckedPackage, node: WireNodeId) -> bool {
    package
        .graph()
        .semantic_graph()
        .nodes()
        .any(|candidate| candidate.key().as_bytes() == node.as_bytes())
}

/// A checker-issued callable bound to one acquisition, with no public constructor.
#[derive(Clone)]
pub struct CallableSelection {
    owner_package_id: DigestRecord,
    key: NodeKey,
    signature: Signature,
    acquisition: Arc<()>,
}

impl CallableSelection {
    /// The package that owns this callable.
    pub fn owner_package_id(&self) -> DigestRecord {
        self.owner_package_id
    }
    /// The check-minted key, for generated-body diagnostics only.
    pub fn key(&self) -> NodeKey {
        self.key
    }
    /// The exact admitted signature.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }
}

/// Why a callable lookup did not issue a selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CallableLookupRefusal {
    /// The owner or node is absent, or several other packages own the node.
    #[error("unknown callable")]
    Unknown,
    /// Exactly one other package in the closure owns the node.
    #[error("callable belongs to package {0:?}")]
    Foreign(DigestRecord),
    /// The selected package owns the node but it is not callable by name.
    #[error("node is not callable by name")]
    WrongKind,
}

/// Acquire and retain the checked closure from the one shared compilation path.
pub fn acquire_checked_closure<'a>(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: impl IntoIterator<Item = &'a [u8]>,
    dependencies: &DependencyInput,
    limits: StageLimits,
    replay_limits: ReplayLimits,
) -> Result<CheckedClosureCompilation, AdmittedRefusal> {
    let compiled = recompile_unit(
        source,
        path,
        bytes,
        packages,
        dependencies,
        limits,
        replay_limits,
    )?;
    let package_id = compiled.emitted.package().package_id();
    let context =
        context_of(compiled.checked.package(), package_id).map_err(AdmittedRefusal::Fault)?;
    Ok(CheckedClosureCompilation {
        package: CompiledPackage::new(compiled.emitted),
        context,
        handle: CheckedClosureHandle {
            package: Arc::new(compiled.checked.into_package()),
            package_id: package_id.record(),
            acquisition: Arc::new(()),
        },
    })
}

/// The object closure as admitted by the existing QSL environment constructor.
pub struct ApplicationEnvironment(ObjectEnvironment);

impl ApplicationEnvironment {
    /// Use the supplied object closure without inventing population data.
    pub fn new(objects: ObjectClosure) -> Self {
        Self(ObjectEnvironment::new(objects))
    }
}

/// A leaf registration result or an acquisition-identity refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SelectionRefusal {
    /// The selection was issued by another acquisition.
    #[error("selection belongs to another acquisition")]
    WrongAcquisition,
}

/// A leaf Begin failure or an acquisition-identity refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ApplicationBeginRefusal {
    /// The selection was issued by another acquisition.
    #[error("selection belongs to another acquisition")]
    WrongAcquisition,
    /// The leaf refused admission before starting a session.
    #[error(transparent)]
    Call(#[from] CallFailure),
}

/// A binding set over precisely one retained acquisition.
pub struct ApplicationRegistry {
    acquisition: Arc<()>,
    inner: Registry<Adapter>,
}

impl ApplicationRegistry {
    /// Bind a generated body after checking acquisition identity.
    pub fn register(
        &mut self,
        selection: &CallableSelection,
        signature: &Signature,
        factory: impl BodyFactory + 'static,
    ) -> Result<Admission, SelectionRefusal> {
        if !Arc::ptr_eq(&self.acquisition, &selection.acquisition) {
            return Err(SelectionRefusal::WrongAcquisition);
        }
        Ok(self.inner.register(selection.key, signature, factory))
    }

    /// Begin a call under the leaf's admission and permanent sealing rules.
    pub fn begin_call<'a>(
        &mut self,
        selection: &CallableSelection,
        arguments: Vec<Value>,
        environment: &'a ApplicationEnvironment,
        meter: &'a mut Meter,
    ) -> Result<CheckedApplicationSession<'a>, ApplicationBeginRefusal> {
        if !Arc::ptr_eq(&self.acquisition, &selection.acquisition) {
            return Err(ApplicationBeginRefusal::WrongAcquisition);
        }
        self.inner
            .begin_call(selection.key, arguments, &environment.0, meter)
            .map(|inner| CheckedApplicationSession { inner })
            .map_err(ApplicationBeginRefusal::Call)
    }

    /// Whether a successful Begin permanently sealed this binding set.
    pub fn is_sealed(&self) -> bool {
        self.inner.is_sealed()
    }
}

/// A live leaf application whose meter belongs to its caller.
pub struct CheckedApplicationSession<'a> {
    inner: Session<'a, Adapter>,
}

impl<'a> CheckedApplicationSession<'a> {
    /// Take one leaf transition, consuming this session.
    pub fn step(self) -> CheckedApplicationProgress<'a> {
        match self.inner.step() {
            Progress::Runnable(inner) => CheckedApplicationProgress::Runnable(Self { inner }),
            Progress::Terminal(result) => CheckedApplicationProgress::Terminal(result),
        }
    }

    /// Drive this leaf session to its terminal result.
    pub fn run(self) -> Result<Evaluation, CallFailure> {
        self.inner.run()
    }
}

/// The runnable or terminal state of one leaf step.
#[must_use = "dropping a runnable session abandons the application"]
pub enum CheckedApplicationProgress<'a> {
    /// The same application can be stepped again.
    Runnable(CheckedApplicationSession<'a>),
    /// The application ended and its session was consumed.
    Terminal(Result<Evaluation, CallFailure>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine::SuppliedLibrary;
    use ix_trace_rs::trace;
    use quire_eval::{Body, ChildRequest, FamilyOutcome, Resume, Transition};
    use quire_exact::{Cancel, CancelCause, Integer, LimitKind, Outcome, ScalarLimits, ValueType};
    use quire_semantic_value::call::InputRefusal;
    use quire_semantic_value::catalog::InternalFault;
    use quire_semantic_value::location::Location;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const HEADER: &str =
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
    const FUNCTIONS: &str = "function f using v(x: Int[0, 9]): Integer pure { x }\nfunction g using v(x: Int[0, 9]): Integer pure { f(x) }\n";

    fn identity(name: &str) -> SourceIdentity {
        SourceIdentity::new("a", name, "git", "1")
    }
    fn acquire(text: &str, dependencies: &DependencyInput) -> CheckedClosureCompilation {
        acquire_checked_closure(
            identity("unit"),
            "unit.native",
            text.as_bytes(),
            [],
            dependencies,
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("checked acquisition")
    }
    fn wire(package: &CheckedPackage, name: &str) -> WireNodeId {
        WireNodeId::from_digest(
            *package
                .graph()
                .function_identity(name)
                .expect("function")
                .as_bytes(),
        )
    }
    fn meter(work_units: u64) -> Meter {
        let large = u64::from(u32::MAX);
        Meter::new(ScalarLimits {
            integer_bits: large,
            decimal_digits: large,
            scale_expansion: large,
            text_input_bytes: large,
            text_scalars: large,
            normalized_scalars: large,
            unit_edges: large,
            value_occurrences: large,
            work_units,
            result_units: large,
        })
    }
    fn int(value: u64) -> Value {
        Value::Integer(Integer::from(value))
    }
    fn completed(evaluation: &Evaluation) -> Option<u64> {
        match &evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => value.to_u64(),
            _ => None,
        }
    }

    #[derive(Clone)]
    struct Host {
        begun: Arc<AtomicUsize>,
        child: Option<NodeKey>,
    }
    impl BodyFactory for Host {
        fn begin(&self) -> Box<dyn Body> {
            self.begun.fetch_add(1, Ordering::SeqCst);
            Box::new(HostBody { child: self.child })
        }
    }
    struct HostBody {
        child: Option<NodeKey>,
    }
    impl Body for HostBody {
        fn resume(
            &mut self,
            input: Resume,
            _meter: &mut Meter,
        ) -> Result<Transition, InternalFault> {
            Ok(match input {
                Resume::Start(arguments) => match self.child {
                    Some(callee) => Transition::Call(ChildRequest {
                        callee,
                        arguments,
                        site: Location::at(quire_semantic_value::location::Origin::Expression, &[]),
                    }),
                    None => Transition::Done(Evaluation {
                        outcome: FamilyOutcome::Evaluated(Outcome::Completed(int(7))),
                        location: None,
                        losses: vec![],
                    }),
                },
                Resume::Child(result) => Transition::Done(result),
                Resume::Again => Transition::Continue,
            })
        }
    }

    /// One compile yields equal public outputs and a retained owned closure.
    #[trace("TC-1263", "FR-485-AC-1", "FR-485-AC-8")]
    #[test]
    fn acquisition_retains_one_checked_closure() {
        let text =
            format!("{HEADER}import \"test/lib\" as l;\nenum Color {{ RED, BLUE }}\n{FUNCTIONS}");
        let input = DependencyInput::new([SuppliedLibrary {
            identity: "test/lib".to_owned(),
            source: identity("lib"),
            path: "lib.native".to_owned(),
            bytes: format!(
                "{HEADER}function imported using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n"
            )
            .into_bytes(),
        }])
        .expect("dependency input");
        let expected = crate::compile_admitted(
            identity("unit"),
            "unit.native",
            text.as_bytes(),
            [],
            &input,
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("admitted");
        let spine_runs = crate::execute::spine_run_count();
        let acquired = acquire(&text, &input);
        assert_eq!(crate::execute::spine_run_count(), spine_runs + 1);
        assert_eq!(acquired.package().bytes(), expected.package().bytes());
        assert_eq!(
            acquired.package().package_id(),
            expected.package().package_id()
        );
        assert_eq!(
            acquired.context().package_id(),
            expected.context().package_id()
        );
        assert_eq!(
            format!("{:?}", acquired.context()),
            format!("{:?}", expected.context())
        );
        let own = wire(&acquired.handle.package, "f");
        let (bytes, context, handle) = acquired.into_parts();
        drop((bytes, context, text));
        let selection = handle
            .lookup_callable(handle.package_id(), own)
            .expect("retained callable");
        assert_eq!(selection.key().as_bytes(), own.as_bytes());
        assert_eq!(selection.signature().parameters().len(), 1);
        let mut registry = handle.registry();
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut meter = meter(1000);
        let result = registry
            .begin_call(&selection, vec![int(3)], &environment, &mut meter)
            .expect("begin")
            .run()
            .expect("run");
        assert_eq!(completed(&result), Some(3));
        assert!(meter.consumed(LimitKind::WorkUnits) > 0);
    }

    /// Owner selection precedes node lookup, including an imported callable.
    #[trace("TC-1263", "FR-485-AC-2")]
    #[test]
    fn owner_scoped_lookup_refuses_foreign_unknown_and_wrong_kind() {
        let library = SuppliedLibrary {
            identity: "test/lib".to_owned(),
            source: identity("lib"),
            path: "lib.native".to_owned(),
            bytes: format!(
                "{HEADER}function imported using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n"
            )
            .into_bytes(),
        };
        let lib_acquired = acquire_checked_closure(
            library.source.clone(),
            &library.path,
            &library.bytes,
            [],
            &DependencyInput::default(),
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("library");
        let lib_wire = wire(&lib_acquired.handle.package, "imported");
        let dependencies = DependencyInput::new([library]).expect("dependency input");
        let acquired = acquire(
            &format!("{HEADER}import \"test/lib\" as l;\n{FUNCTIONS}"),
            &dependencies,
        );
        let handle = acquired.handle();
        assert_eq!(
            handle.lookup_callable(handle.package_id(), lib_wire).err(),
            Some(CallableLookupRefusal::Foreign(
                lib_acquired.package().package_id()
            ))
        );
        let dependency = handle
            .lookup_callable(lib_acquired.package().package_id(), lib_wire)
            .expect("dependency callable");
        assert_eq!(
            dependency.owner_package_id(),
            lib_acquired.package().package_id()
        );
        assert_eq!(
            handle
                .lookup_callable(handle.package_id(), WireNodeId::from_digest([9; 32]))
                .err(),
            Some(CallableLookupRefusal::Unknown)
        );
        assert_eq!(
            handle
                .lookup_callable(
                    lib_acquired.package().package_id(),
                    wire(&handle.package, "f")
                )
                .err(),
            Some(CallableLookupRefusal::Foreign(handle.package_id()))
        );
        assert_eq!(
            handle
                .lookup_callable(
                    DigestRecord::mint(
                        qsl_foundation::digest::DigestDomain::PackageSemanticV2,
                        [8; 32]
                    ),
                    lib_wire
                )
                .err(),
            Some(CallableLookupRefusal::Unknown)
        );
        let mut registry = handle.registry();
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut meter = meter(1000);
        let result = registry
            .begin_call(&dependency, vec![int(4)], &environment, &mut meter)
            .expect("begin imported")
            .run()
            .expect("run imported");
        assert!(matches!(
            result.outcome,
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
        ));
    }

    /// The facade identity check precedes the leaf's admission and seal checks.
    #[trace("TC-1263", "FR-485-AC-4", "FR-485-AC-5", "FR-485-AC-6")]
    #[test]
    fn bindings_sessions_and_acquisition_identity_follow_leaf_order() {
        let text = format!("{HEADER}{FUNCTIONS}");
        let first = acquire(&text, &DependencyInput::default());
        let second = acquire(&text, &DependencyInput::default());
        let own = wire(&first.handle.package, "f");
        let selection = first
            .handle()
            .lookup_callable(first.package().package_id(), own)
            .expect("selection");
        let foreign = second
            .handle()
            .lookup_callable(second.package().package_id(), own)
            .expect("same key second acquisition");
        assert_eq!(selection.key(), foreign.key());
        let mut registry = first.handle().registry();
        let begun = Arc::new(AtomicUsize::new(0));
        let host = Host {
            begun: Arc::clone(&begun),
            child: None,
        };
        assert_eq!(
            registry.register(&foreign, foreign.signature(), host.clone()),
            Err(SelectionRefusal::WrongAcquisition)
        );
        let wrong = Signature::new(vec![ValueType::Boolean], ValueType::Integer);
        assert_eq!(
            registry.register(&selection, &wrong, host.clone()),
            Ok(Admission::SignatureMismatch)
        );
        assert_eq!(
            registry.register(&selection, selection.signature(), host.clone()),
            Ok(Admission::Accepted)
        );
        assert_eq!(
            registry.register(&selection, selection.signature(), host.clone()),
            Ok(Admission::DuplicateBinding)
        );
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut meter = meter(1000);
        assert!(matches!(
            registry.begin_call(&selection, vec![], &environment, &mut meter),
            Err(ApplicationBeginRefusal::Call(CallFailure::Input(
                InputRefusal::Arity { .. }
            )))
        ));
        assert!(!registry.is_sealed());
        assert_eq!(meter.admission_count(), 0);
        assert!(matches!(
            registry.begin_call(&foreign, vec![int(3)], &environment, &mut meter),
            Err(ApplicationBeginRefusal::WrongAcquisition)
        ));
        let session = registry
            .begin_call(&selection, vec![int(3)], &environment, &mut meter)
            .expect("begin");
        assert!(registry.is_sealed());
        assert_eq!(begun.load(Ordering::SeqCst), 0);
        drop(session);
        assert_eq!(begun.load(Ordering::SeqCst), 0);
        assert_eq!(
            registry.register(&foreign, foreign.signature(), host.clone()),
            Err(SelectionRefusal::WrongAcquisition)
        );
        assert_eq!(
            registry.register(&selection, selection.signature(), host),
            Ok(Admission::RegistrySealed)
        );
        let result = registry
            .begin_call(&selection, vec![int(4)], &environment, &mut meter)
            .expect("second begin")
            .run()
            .expect("host run");
        assert_eq!(completed(&result), Some(7));
        assert_eq!(begun.load(Ordering::SeqCst), 1);
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 1);
    }

    /// Refusals and cancellation never create a handle or charge at Begin.
    #[trace("TC-1263", "FR-485-AC-6", "FR-485-AC-7")]
    #[test]
    fn compile_refusals_and_cancelled_begin_preserve_leaf_results() {
        let malformed = "not a QSL unit";
        let input = DependencyInput::default();
        let expected = crate::compile_admitted(
            identity("unit"),
            "unit.native",
            malformed.as_bytes(),
            [],
            &input,
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .unwrap_err();
        let actual = acquire_checked_closure(
            identity("unit"),
            "unit.native",
            malformed.as_bytes(),
            [],
            &input,
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .err()
        .expect("refusal");
        assert_eq!(
            std::mem::discriminant(&actual),
            std::mem::discriminant(&expected)
        );
        let acquired = acquire(&format!("{HEADER}{FUNCTIONS}"), &input);
        let selected = acquired
            .handle()
            .lookup_callable(
                acquired.package().package_id(),
                wire(&acquired.handle.package, "f"),
            )
            .expect("selection");
        let mut registry = acquired.handle().registry();
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let cancel = Cancel::new();
        cancel.cancel(CancelCause::Requested);
        let mut meter = meter(1000).with_cancel(cancel);
        assert!(matches!(
            registry.begin_call(&selected, vec![int(3)], &environment, &mut meter),
            Err(ApplicationBeginRefusal::Call(CallFailure::Cancelled(
                CancelCause::Requested
            )))
        ));
        assert!(!registry.is_sealed());
        assert_eq!(meter.admission_count(), 0);
    }

    /// A private malformed selection reaches the leaf's own callable admission.
    #[trace("TC-1263", "FR-485-AC-2", "FR-485-AC-4")]
    #[test]
    fn public_lookup_and_private_malformed_selection_have_distinct_refusals() {
        let acquired = acquire(
            &format!("{HEADER}enum Color {{ RED, BLUE }}\n{FUNCTIONS}"),
            &DependencyInput::default(),
        );
        let handle = acquired.handle();
        let own = handle
            .lookup_callable(handle.package_id(), wire(&handle.package, "f"))
            .expect("own selection");
        let enum_key = handle
            .package
            .graph()
            .semantic_graph()
            .nodes()
            .map(|node| node.key())
            .find(|key| handle.package.graph().callable_by_identity(*key).is_none())
            .expect("non-callable node");
        assert_eq!(
            handle
                .lookup_callable(
                    handle.package_id(),
                    WireNodeId::from_digest(*enum_key.as_bytes())
                )
                .err(),
            Some(CallableLookupRefusal::WrongKind)
        );
        let unrelated = acquire(
            &format!("{HEADER}function other using v(x: Int[0, 9]): Integer pure {{ x }}\n"),
            &DependencyInput::default(),
        );
        let missing_key = unrelated
            .handle
            .package
            .graph()
            .function_identity("other")
            .expect("other");
        let missing = WireNodeId::from_digest(*missing_key.as_bytes());
        assert_eq!(
            handle.lookup_callable(handle.package_id(), missing).err(),
            Some(CallableLookupRefusal::Unknown)
        );
        let mut malformed = own.clone();
        malformed.key = missing_key;
        let mut registry = handle.registry();
        let begun = Arc::new(AtomicUsize::new(0));
        assert_eq!(
            registry.register(
                &malformed,
                malformed.signature(),
                Host {
                    begun: Arc::clone(&begun),
                    child: None
                }
            ),
            Ok(Admission::CallableNotAdmitted)
        );
        assert_eq!(begun.load(Ordering::SeqCst), 0);
    }

    /// A registered body requests another callable through the one leaf session.
    #[trace("TC-1263", "FR-485-AC-5")]
    #[test]
    fn generated_body_child_request_uses_the_leaf_session() {
        let acquired = acquire(&format!("{HEADER}{FUNCTIONS}"), &DependencyInput::default());
        let handle = acquired.handle();
        let f = handle
            .lookup_callable(handle.package_id(), wire(&handle.package, "f"))
            .expect("f");
        let g = handle
            .lookup_callable(handle.package_id(), wire(&handle.package, "g"))
            .expect("g");
        let begun = Arc::new(AtomicUsize::new(0));
        let mut registry = handle.registry();
        assert_eq!(
            registry.register(
                &g,
                g.signature(),
                Host {
                    begun: Arc::clone(&begun),
                    child: Some(f.key())
                }
            ),
            Ok(Admission::Accepted)
        );
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut meter = meter(1000);
        let result = registry
            .begin_call(&g, vec![int(3)], &environment, &mut meter)
            .expect("begin")
            .run()
            .expect("child application");
        assert_eq!(completed(&result), Some(3));
        assert_eq!(begun.load(Ordering::SeqCst), 1);
        assert!(meter.admission_count() >= 2, "root and child call charges");
    }

    /// Cancellation after Begin is a located incomplete outcome at the denied charge.
    #[trace("TC-1263", "FR-485-AC-6")]
    #[test]
    fn cancellation_after_begin_and_work_denial_preserve_leaf_accounting() {
        let acquired = acquire(&format!("{HEADER}{FUNCTIONS}"), &DependencyInput::default());
        let handle = acquired.handle();
        let selection = handle
            .lookup_callable(handle.package_id(), wire(&handle.package, "f"))
            .expect("f");
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let cancel = Cancel::new();
        let mut cancelled_meter = meter(1000).with_cancel(cancel.clone());
        let mut registry = handle.registry();
        let session = registry
            .begin_call(&selection, vec![int(3)], &environment, &mut cancelled_meter)
            .expect("begin");
        cancel.cancel(CancelCause::Requested);
        let result = session.run().expect("located incomplete");
        assert!(matches!(
            result.outcome,
            FamilyOutcome::Evaluated(Outcome::Incomplete(_))
        ));
        assert!(result.location.is_some());
        assert_eq!(cancelled_meter.admission_count(), 0);
        assert_eq!(cancel.tripped(), Some(CancelCause::Requested));

        let mut baseline = meter(1000);
        let successful = handle
            .registry()
            .begin_call(&selection, vec![int(3)], &environment, &mut baseline)
            .expect("begin baseline")
            .run()
            .expect("baseline run");
        assert_eq!(completed(&successful), Some(3));
        let spend = baseline.consumed(LimitKind::WorkUnits);
        assert!(spend > 0);
        let mut limited = meter(spend - 1);
        let result = handle
            .registry()
            .begin_call(&selection, vec![int(3)], &environment, &mut limited)
            .expect("begin limited")
            .run()
            .expect("incomplete run");
        assert!(matches!(
            result.outcome,
            FamilyOutcome::Evaluated(Outcome::Incomplete(_))
        ));
        assert!(result.location.is_some());
        assert!(limited.consumed(LimitKind::WorkUnits) < spend);
        let mut exact = meter(spend);
        let result = handle
            .registry()
            .begin_call(&selection, vec![int(3)], &environment, &mut exact)
            .expect("begin exact")
            .run()
            .expect("exact run");
        assert_eq!(completed(&result), Some(3));
    }

    /// Acquisition shares the exact refusal classification of compile_admitted.
    #[trace("TC-1263", "FR-485-AC-7")]
    #[test]
    fn acquisition_refusals_match_the_existing_compile_path() {
        use qsl_foundation::Setting;

        let valid = format!("{HEADER}{FUNCTIONS}");
        let same_owner = DependencyInput::new([SuppliedLibrary {
            identity: "test/same".to_owned(),
            source: SourceIdentity::new("a", "unit", "git", "2"),
            path: "same.native".to_owned(),
            bytes: HEADER.as_bytes().to_vec(),
        }])
        .expect("admissible input");
        let short: StageLimits = [(Setting::S1InputBytes, 4)].into_iter().collect();
        let above: StageLimits = [(Setting::S1InputBytes, 2 << 20)].into_iter().collect();
        let empty = DependencyInput::default();
        for (source, dependencies, stages) in [
            ("not a QSL unit", &empty, StageLimits::default()),
            (valid.as_str(), &same_owner, StageLimits::default()),
            (valid.as_str(), &empty, short),
            (valid.as_str(), &empty, above),
        ] {
            let expected = crate::compile_admitted(
                identity("unit"),
                "unit.native",
                source.as_bytes(),
                [],
                dependencies,
                stages.clone(),
                ReplayLimits::default(),
            )
            .err()
            .expect("existing refusal");
            let actual = acquire_checked_closure(
                identity("unit"),
                "unit.native",
                source.as_bytes(),
                [],
                dependencies,
                stages,
                ReplayLimits::default(),
            )
            .err()
            .expect("facade refusal");
            assert_eq!(
                format!("{actual:?}"),
                format!("{expected:?}"),
                "source: {source}"
            );
        }
    }

    /// An absent population binding fails at Begin before any generated body runs.
    #[trace("TC-1263", "FR-485-AC-7")]
    #[test]
    fn unrecorded_population_refuses_before_application() {
        use qsl_semantics::model::intake::package_input;
        use quire_exact::PopulationId;

        let document = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/fixtures/spine-model.semantic-ir.json"
        ));
        let packages = package_input([document.as_slice()]);
        let digest = packages.keys().next().expect("one model package");
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
            profile v = \"quire.model.complete/v1\";\n\
            model M = \"acme/orders\" version \"1.0.0\" digest \"sha256-jcs:{hex}\";\n\
            function h using v(p: Population<M::Widget>[3]): Boolean pure {{ true }}\n"
        );
        let acquired = acquire_checked_closure(
            identity("unit"),
            "unit.native",
            source.as_bytes(),
            [document.as_slice()],
            &DependencyInput::default(),
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("model function compiles");
        let handle = acquired.handle();
        let selection = handle
            .lookup_callable(handle.package_id(), wire(&handle.package, "h"))
            .expect("h");
        let mut registry = handle.registry();
        let begun = Arc::new(AtomicUsize::new(0));
        assert_eq!(
            registry.register(
                &selection,
                selection.signature(),
                Host {
                    begun: Arc::clone(&begun),
                    child: None,
                }
            ),
            Ok(Admission::Accepted)
        );
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut meter = meter(1000);
        let result = registry.begin_call(
            &selection,
            vec![Value::Population(PopulationId::from_digest([7; 32]))],
            &environment,
            &mut meter,
        );
        assert!(matches!(
            result,
            Err(ApplicationBeginRefusal::Call(CallFailure::Input(
                InputRefusal::WrongValueKind { parameter: 0 }
            )))
        ));
        assert!(!registry.is_sealed());
        assert_eq!(meter.admission_count(), 0);
        assert_eq!(begun.load(Ordering::SeqCst), 0);
    }
}
