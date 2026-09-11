// SPDX-License-Identifier: AGPL-3.0-only
//! FR-036: declaration-owned lexical environments and protocol structural names.
//! Scope resolution preserves authored types; it does not prove expression types.
mod protocol;
mod values;

use super::binding_work::{Dimension, Exhaustion, Work};
use super::models::ModelBindings;
use super::{DeclarationId, SyntaxNamespace, UnitId};
use crate::syntax::composed::{self as c, ComposedUnit, ParameterType, QualifiedName};
use crate::syntax::{ClauseKind, ExprId};
use crate::{Span, Spanned};
use std::collections::BTreeMap;

/// A binder identity local to its owning declaration report.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BinderId(pub(super) usize);
impl BinderId {
    /// Index into this declaration's binders; not a cross-declaration identity.
    pub fn index(self) -> usize {
        self.0
    }
}

/// A structural identity local to its owning protocol declaration report.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SymbolId(pub(super) usize);
impl SymbolId {
    /// Index into this declaration's structural symbols.
    pub fn index(self) -> usize {
        self.0
    }
}

/// The authored anchor of a value; captures keep this anchor on later reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Anchor {
    /// Explicit arguments supplied to a shared predicate.
    Predicate,
    /// State invariant observation.
    Current,
    /// Immutable parameters of the selected invocation.
    InvocationInput,
    /// Selected invocation's pre observation.
    InvocationPre,
    /// Selected invocation's post observation.
    InvocationPost,
    /// Origin or trigger activation of the enclosing declaration.
    Activation,
    /// Current native leaf valuation in a temporal formula.
    TemporalInstant,
    /// Current workflow view at the protocol evaluation instant.
    ProtocolInstant,
    /// Isolated channel key input; index addresses protocol.channels.
    Fifo(usize),
    /// Forward-effect registration; index addresses protocol.requirements.
    Registration(usize),
    /// First eligible compensation activation; same requirement index.
    CompensationActivation(usize),
    /// Immutable earlier/later attempt pair; same requirement index.
    Retry(usize),
    /// Supplied recovery view; same requirement index.
    Recovery(usize),
    /// Original unit-local control, retaining its structural ancestry.
    Control(c::ControlId),
    /// Workflow closure observation.
    Finish,
}

/// Why a value slot exists, separate from its nominal type and anchor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinderKind {
    /// Explicit shared predicate parameter.
    Parameter,
    /// Temporal or protocol current-input slot.
    Input,
    /// Declaration activation trigger, unavailable in its later body.
    Trigger,
    /// Immutable authored capture.
    Capture,
    /// Let result, visible only in its body.
    Let,
    /// One query-domain element, visible only in the query body.
    Query,
    /// Model-owned state context at an explicit anchor.
    SelfValue,
    /// Model-owned invocation result, available only in postconditions.
    ResultValue,
    /// Actual input named by the selected model operation.
    InvocationParameter,
    /// Event or commit record supplied at a control occurrence.
    EventRecord,
    /// Supplied final closure record.
    Finish,
    /// Channel-specific message used by a FIFO key expression.
    Fifo,
    /// Compensation forward record, available only during registration.
    ForwardEffect,
    /// Candidate trigger, available only during compensation activation.
    CompensationTrigger,
    /// Predecessor attempt in the retry relation.
    EarlierAttempt,
    /// Successor attempt in the retry relation.
    LaterAttempt,
    /// Supplied compensation recovery view.
    Recovery,
}

/// Retained type input for the existing model/type stage, not a parallel type system.
#[derive(Clone, Debug)]
pub enum BinderType {
    /// Exact authored parameter type, before model resolution attaches its owner.
    Declared(ParameterType),
    /// Authored model context of an implicit self slot.
    Context(QualifiedName),
    /// Actual producer value declaration for an invocation input/result.
    ModelValue(crate::linking::DeclarationLocation),
    /// Let value whose type is established by the existing checker.
    Initializer(ExprId),
    /// Query domain whose element type is established by the existing checker.
    ElementOf(ExprId),
}

/// One immutable declaration-owned value. For model inputs `span` locates the
/// native operation selector; ModelValue retains the actual formal declaration.
#[derive(Clone, Debug)]
pub struct Binder {
    /// Lexical spelling; implicit self/result slots have no named binding.
    pub name: Option<String>,
    /// Original native binder token or selecting operation token.
    pub span: Span,
    /// Binding role independent of its type.
    pub kind: BinderKind,
    /// Anchor retained even when the immutable value is read later.
    pub anchor: Anchor,
    /// Existing model/type stage's explicit input.
    pub ty: BinderType,
}

/// An exact original value occurrence and its selected immutable binder.
#[derive(Clone, Debug)]
pub struct ValueOccurrence {
    /// Source owner of the handle and span.
    pub unit: UnitId,
    /// Original value-arena handle, never a package-global integer.
    pub expression: ExprId,
    /// Original name, self or result token region.
    pub span: Span,
    /// Immutable binder in this declaration's report.
    pub target: BinderId,
    /// Evaluation context, separate from a capture's retained anchor.
    pub evaluation_anchor: Anchor,
}

/// Static protocol namespaces. Control kinds remain distinct from value binders.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    /// Participant responsibility slot.
    Role,
    /// Declared message channel.
    Channel,
    /// Declared producer-owned relationship.
    Relationship,
    /// Registered compensation template.
    Compensation,
    /// Ordered control group.
    Sequence,
    /// Exclusive decision group.
    Choice,
    /// Named choice case scope.
    Case,
    /// All-branch parallel group.
    Parallel,
    /// Named parallel branch scope.
    Branch,
    /// Explicitly bounded repetition group.
    Repeat,
    /// Event/deadline control group.
    Await,
    /// Semantic send event.
    Send,
    /// Receive tied to an exact send.
    Receive,
    /// Model operation attempt.
    Attempt,
    /// Successful effect tied to an exact attempt.
    Effect,
    /// Typed domain event.
    Event,
    /// Native value constraint control.
    Check,
    /// Observed forbidding commit boundary.
    Commit,
    /// Protocol closure observation.
    Finish,
}

/// Parent links retain canonical group/branch paths without duplicating paths.
#[derive(Clone, Debug)]
pub struct Symbol {
    /// Original declaration token and source spelling.
    pub name: Spanned<String>,
    /// Structural namespace/kind.
    pub kind: SymbolKind,
    /// Enclosing control, branch or case; None selects the protocol root.
    pub parent: Option<SymbolId>,
    /// Original unit-local control handle, absent for metadata declarations.
    pub control: Option<c::ControlId>,
    /// Authored role/relationship/payload selection for model attachment.
    pub model: Option<QualifiedName>,
}

/// Distinct target obligations of structural occurrences.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralKind {
    /// Participant role required by an endpoint or control.
    Role,
    /// Message channel required by a send/receive.
    Channel,
    /// Relationship required by related endpoints.
    Relationship,
    /// Exact send selected by a receive.
    Send,
    /// Exact operation attempt selected by an effect.
    Attempt,
    /// Successful effect selected by compensation registration.
    Effect,
    /// Exact forbidding commit selected by a compensation template.
    Commit,
    /// Exact compensation registration selected by a domain event.
    Compensation,
    /// Preceding event/commit or named compensation activation.
    AwaitAnchor,
    /// Named branch required by an all-branch join.
    Branch,
}

/// An original structural path, resolved within its owning declaration.
#[derive(Clone, Debug)]
pub struct StructuralReference {
    /// Owning control; metadata references belong to the declaration.
    pub site: Option<c::ControlId>,
    /// Complete original path region.
    pub span: Span,
    /// Original path components, including each token's source span.
    pub path: Vec<Spanned<String>>,
    /// Required target kind, independent of equal name spelling.
    pub required: StructuralKind,
    /// Unique selected symbol, retained even when its kind refuses.
    pub target: Option<SymbolId>,
}

/// Located scope failures remain typed independently of downstream model errors.
#[derive(Clone, Debug)]
pub enum ScopeIssue {
    /// A declaration reuses a binder; both source locations remain inspectable.
    DuplicateBinder { span: Span, previous: BinderId },
    /// A binder would shadow a model/profile alias or native declaration.
    ReservedBinder { span: Span },
    /// No lexical binding declares this value name.
    MissingValue {
        expression: ExprId,
        span: Span,
        name: String,
    },
    /// A declared binding is unavailable in this expression's environment.
    OutOfScope {
        expression: ExprId,
        span: Span,
        name: String,
    },
    /// self/result/pre is unavailable at this declaration or selected anchor.
    AmbientUnavailable { expression: ExprId, span: Span },
    /// Model refusal prevented establishing actual invocation inputs.
    ModelOperationUnavailable { span: Span },
    /// Two declarations occupy the same structural scope/name.
    DuplicateSymbol {
        symbol: SymbolId,
        previous: SymbolId,
    },
    /// No symbol supplies the original path; index addresses references.
    MissingTarget { reference: usize },
    /// Multiple symbols supply a path component.
    AmbiguousTarget { reference: usize },
    /// The unique symbol has a different required structural kind.
    WrongTargetKind { reference: usize, target: SymbolId },
    /// The referenced event/commit is not necessarily available on this path.
    UnavailableTarget { reference: usize, target: SymbolId },
    /// The all-branch join does not select every branch exactly once.
    InvalidJoin { span: Span },
    /// This protocol profile does not admit waiting on a send or attempt.
    InvalidAwaitEvent { control: c::ControlId, span: Span },
    /// A receive selects a different channel than its exact referenced send.
    IncompatibleReference { reference: usize },
    /// An internal scope result did not extend its inherited frame chain.
    InvalidEnvironment { span: Span },
}

/// Scope completion is independent of definition/model/type/family admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopeDisposition {
    /// Traversal completed without a lexical or structural refusal.
    Resolved,
    /// Complete traversal retained at least one refusal.
    Refused,
    /// This declaration's traversal did not complete.
    Unfinished,
}

/// All IDs, anchors and spans below are qualified by this declaration and unit.
#[derive(Debug)]
pub struct DeclarationScope {
    /// Original package declaration identity.
    pub declaration: DeclarationId,
    /// Source owner of all local syntax handles and spans.
    pub unit: UnitId,
    /// Original immutable value identities.
    pub binders: Vec<Binder>,
    /// Resolved value-name/self/result occurrences.
    pub values: Vec<ValueOccurrence>,
    /// Typed protocol declarations and their structural ancestry.
    pub symbols: Vec<Symbol>,
    /// Original protocol paths, including failed selections.
    pub references: Vec<StructuralReference>,
    /// Typed lexical/structural refusals known before any exhaustion.
    pub issues: Vec<ScopeIssue>,
    complete: bool,
}
impl DeclarationScope {
    /// Scope completion only; retained types still require their owning checker.
    pub fn disposition(&self) -> ScopeDisposition {
        if !self.complete {
            ScopeDisposition::Unfinished
        } else if self.issues.is_empty() {
            ScopeDisposition::Resolved
        } else {
            ScopeDisposition::Refused
        }
    }
}

/// Missing result slots mean unfinished work, never successful empty declarations.
#[derive(Debug)]
pub struct ScopeReport {
    declarations: Vec<Option<DeclarationScope>>,
    /// First unaffordable operation; earlier declaration results remain intact.
    pub exhaustion: Option<Exhaustion>,
}
impl ScopeReport {
    /// Retained partial/completed result, absent if this declaration never began.
    pub fn declaration(&self, id: DeclarationId) -> Option<&DeclarationScope> {
        self.declarations.get(id.index()).and_then(Option::as_ref)
    }
    /// Slots follow namespace declaration order; None means unfinished work.
    pub fn declarations(&self) -> &[Option<DeclarationScope>] {
        &self.declarations
    }
    /// Missing results are unfinished, never an implicit successful empty scope.
    pub fn disposition(&self, id: DeclarationId) -> ScopeDisposition {
        self.declaration(id)
            .map_or(ScopeDisposition::Unfinished, DeclarationScope::disposition)
    }
}

#[derive(Clone, Copy)]
struct Environment {
    frame: Option<usize>,
    anchor: Anchor,
    self_value: Option<BinderId>,
    pre_self: Option<BinderId>,
    result: Option<BinderId>,
}
impl Environment {
    fn new(anchor: Anchor) -> Self {
        Self {
            frame: None,
            anchor,
            self_value: None,
            pre_self: None,
            result: None,
        }
    }
}
struct Frame {
    // Frames are appended, so every parent precedes its child in the arena.
    parent: Option<usize>,
    binder: BinderId,
}

struct Resolver<'a, 'w> {
    namespace: &'a SyntaxNamespace,
    unit: &'a ComposedUnit,
    output: &'w mut DeclarationScope,
    work: &'w mut Work,
    frames: Vec<Frame>,
    names: BTreeMap<String, BinderId>,
}

/// Resolve syntax scopes against exact supplied model operation inputs. This
/// establishes lexical/static target ownership, never checked family semantics.
pub fn resolve(
    namespace: &SyntaxNamespace,
    models: &ModelBindings<'_>,
    work: &mut Work,
) -> ScopeReport {
    let mut report = ScopeReport {
        declarations: (0..namespace.declarations().len()).map(|_| None).collect(),
        exhaustion: None,
    };
    for (index, entry) in namespace.declarations().iter().enumerate() {
        let result = (|| {
            work.charge(Dimension::Bindings, 1)?;
            let id = DeclarationId(index);
            let output = report.declarations[index].insert(DeclarationScope {
                declaration: id,
                unit: entry.unit(),
                binders: Vec::new(),
                values: Vec::new(),
                symbols: Vec::new(),
                references: Vec::new(),
                issues: Vec::new(),
                complete: false,
            });
            let unit = namespace.unit(entry.unit()).expect("namespace unit");
            let declaration = &unit.declarations()[entry.declaration_index()];
            let mut resolver = Resolver {
                namespace,
                unit,
                output,
                work,
                frames: Vec::new(),
                names: BTreeMap::new(),
            };
            resolver.declaration(declaration, models)?;
            resolver.finish_names()?;
            resolver.output.complete = true;
            Ok(())
        })();
        if let Err(exhaustion) = result {
            report.exhaustion = Some(exhaustion);
            break;
        }
    }
    report
}

impl Resolver<'_, '_> {
    fn issue(&mut self, issue: ScopeIssue) -> Result<(), Exhaustion> {
        self.work.charge(Dimension::Bindings, 1)?;
        self.output.issues.push(issue);
        Ok(())
    }
    fn binder(&mut self, binder: Binder) -> Result<BinderId, Exhaustion> {
        self.work.charge(Dimension::Bindings, 1)?;
        if let Some(name) = &binder.name {
            self.work.charge(Dimension::References, 1)?;
            if let Some(previous) = self.names.get(name).copied() {
                self.issue(ScopeIssue::DuplicateBinder {
                    span: binder.span,
                    previous,
                })?;
            }
            let mut reserved = !self.namespace.lookup(name).is_empty();
            for import in self.unit.profiles().iter().chain(self.unit.models()) {
                self.work.charge(Dimension::References, 1)?;
                reserved |= import.alias.value == *name;
            }
            if reserved {
                self.issue(ScopeIssue::ReservedBinder { span: binder.span })?;
            }
        }
        let id = BinderId(self.output.binders.len());
        if let Some(name) = &binder.name {
            self.names.entry(name.clone()).or_insert(id);
        }
        self.output.binders.push(binder);
        Ok(id)
    }
    fn extend(
        &mut self,
        mut environment: Environment,
        binder: BinderId,
    ) -> Result<Environment, Exhaustion> {
        self.work.charge(Dimension::Bindings, 1)?;
        let index = self.frames.len();
        self.frames.push(Frame {
            parent: environment.frame,
            binder,
        });
        environment.frame = Some(index);
        Ok(environment)
    }
    fn parameter(
        &mut self,
        parameter: &c::Parameter,
        kind: BinderKind,
        anchor: Anchor,
    ) -> Result<BinderId, Exhaustion> {
        self.binder(Binder {
            name: Some(parameter.name.value.clone()),
            span: parameter.name.span,
            kind,
            anchor,
            ty: BinderType::Declared(parameter.ty.clone()),
        })
    }
    fn captures(
        &mut self,
        captures: &[c::Capture],
        mut env: Environment,
        anchor: Anchor,
    ) -> Result<Environment, Exhaustion> {
        for capture in captures {
            self.work.charge(Dimension::References, 1)?;
            self.expression(capture.value, env)?;
            let binder = self.parameter(&capture.parameter, BinderKind::Capture, anchor)?;
            env = self.extend(env, binder)?;
        }
        Ok(env)
    }
    fn activation(
        &mut self,
        activation: &c::Activation,
        env: Environment,
        captures: &[c::Capture],
    ) -> Result<Environment, Exhaustion> {
        let mut active = Environment {
            anchor: Anchor::Activation,
            ..env
        };
        let (trigger, span) = match activation {
            c::Activation::Origin { span } => (None, *span),
            c::Activation::Each {
                trigger,
                guard,
                span,
            } => {
                let id = self.parameter(trigger, BinderKind::Trigger, Anchor::Activation)?;
                active = self.extend(active, id)?;
                if let Some(guard) = guard {
                    self.expression(*guard, active)?;
                }
                (Some(id), *span)
            }
        };
        let captured = self.captures(captures, active, Anchor::Activation)?;
        // Export captures in their authored order, leaving the activation trigger behind.
        self.export_frames(captured, env, env, trigger, span)
    }

    // Every control result must extend its inherited environment (possibly by
    // zero frames). Validate the complete suffix before exporting anything;
    // malformed ancestry refuses at its authored owner and retains only `output`.
    fn export_frames(
        &mut self,
        result: Environment,
        base: Environment,
        mut output: Environment,
        excluded: Option<BinderId>,
        span: Span,
    ) -> Result<Environment, Exhaustion> {
        let mut exports = Vec::new();
        let mut cursor = result.frame;
        while cursor != base.frame {
            self.work.charge(Dimension::Edges, 1)?;
            let Some(frame) = cursor.and_then(|index| {
                self.frames
                    .get(index)
                    .filter(|frame| frame.parent.is_none_or(|parent| parent < index))
            }) else {
                self.issue(ScopeIssue::InvalidEnvironment { span })?;
                return Ok(output);
            };
            if Some(frame.binder) != excluded {
                exports.push(frame.binder);
            }
            cursor = frame.parent;
        }
        for binder in exports.into_iter().rev() {
            output = self.extend(output, binder)?;
        }
        Ok(output)
    }
    fn finish_names(&mut self) -> Result<(), Exhaustion> {
        for issue in &mut self.output.issues {
            self.work.charge(Dimension::References, 1)?;
            if let ScopeIssue::MissingValue {
                expression,
                span,
                name,
            } = issue
            {
                if self.names.contains_key(name) {
                    *issue = ScopeIssue::OutOfScope {
                        expression: *expression,
                        span: *span,
                        name: name.clone(),
                    };
                }
            }
        }
        Ok(())
    }
}
