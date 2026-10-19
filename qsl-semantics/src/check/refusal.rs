// SPDX-License-Identifier: AGPL-3.0-or-later
//! Located checking refusals and their closed codes and causes.

use qsl_foundation::diagnostic::Code;
use qsl_foundation::source::provenance::SourceRegion;
use qsl_foundation::Setting;
use qsl_foundation::Span;
use quire_exact::Integer;
use quire_exact::{EffectiveId, NodeKey};
use quire_exact::{IllTyped, IllTypedCause};
use quire_semantic_value::location::Location;
use quire_semantic_value::semantic_node::IdentityRefusal;

/// FR-113: why a protocol's `ProtocolClause` checker refused one scoped
/// anchor, or a name it declares. Each variant's `span` is the refusal's
/// own site: the anchor's whole reference for a resolution refusal, or the
/// declaration's own name for a duplicate-declaration refusal (ADR-012
/// §12.2 Check row). `region::DeclarationRegions::refusal_region` reads
/// this span directly rather than through a `Location`'s path: a protocol
/// has no expression tree for a path to walk.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ProtocolAnchorCause {
    /// FR-113 "Refusals": no scope declares the anchor's first segment, or
    /// a later segment names nothing its previous target declares.
    Missing {
        /// The anchor's segments exactly as written (FR-113 "Refusals":
        /// "retaining the segments, the scope, and the first segment that
        /// failed").
        segments: Vec<String>,
        /// The segment that failed to resolve.
        segment: String,
        /// The anchor's own lexical scope (outermost first, empty at the
        /// top level) -- the search's starting point, named for context
        /// regardless of which segment failed.
        scope: Vec<String>,
        /// The whole reference's span.
        span: Span,
    },
    /// FR-113 "Refusals": a scope declares two static nodes of one name, or
    /// an anchor whose resolution decides on that name.
    Ambiguous {
        /// The ambiguous name.
        name: String,
        /// Every declaring locus of the name, in source order.
        loci: Vec<Span>,
        /// The refusal's own site: the anchor's reference, or the
        /// duplicated declaration's own name.
        span: Span,
    },
    /// FR-113 "Refusals": a resolved anchor names a node its site does not
    /// admit.
    WrongKind {
        /// The site the anchor fills, e.g. `"effect-of"`.
        site: &'static str,
        /// The kinds the site admits.
        admitted: Vec<&'static str>,
        /// The resolved target's actual kind.
        actual: &'static str,
        /// The anchor's reference span.
        span: Span,
    },
    /// FR-113 "Refusals": a `receive`'s channel disagrees with the `send`
    /// its `receive-of` anchor names.
    ChannelMismatch {
        /// The `receive`'s own channel.
        receive_channel: String,
        /// The named `send`'s channel.
        send_channel: String,
        /// The anchor's reference span.
        span: Span,
    },
    /// FR-113 "Refusals": a record binder, a capture, a compensation
    /// trigger or a retry or recovery parameter names a model or profile
    /// alias, a native declaration of the package, or another binder
    /// visible where it is declared.
    Shadow {
        /// The shadowing binder's own name.
        name: String,
        /// What the binder shadows.
        shadowed: ShadowedDeclaration,
        /// The shadowing binder's own span.
        span: Span,
    },
    /// FR-114 "Behavior": an `attempt`'s `contracts` entry names no state
    /// clause of the unit.
    MissingContract {
        /// The entry as written.
        entry: String,
        /// The entry's own span.
        span: Span,
    },
    /// FR-114 "Behavior": an `attempt`'s `contracts` entry names an
    /// invariant, or a `pre`/`post` clause anchored at a different
    /// operation than the attempt's own (QSpec: "Listed state
    /// pre/postcondition declarations must match this operation and its
    /// invocation anchors").
    WrongContractAnchor {
        /// The entry as written.
        entry: String,
        /// The attempt's own anchor, `Context::operation`.
        attempt_anchor: String,
        /// The named clause's own anchor: `Context::operation` for a
        /// `pre`/`post` clause anchored elsewhere, or just `Context` for an
        /// invariant (which names no operation).
        clause_anchor: String,
        /// The entry's own span.
        span: Span,
    },
    /// A protocol whose anchors, binders and attempt bindings all check,
    /// but which holds a construct no checker reads yet (a relationship, a
    /// requirement, a replicated role, a `related by` clause, a `predicate`
    /// join or a `terminal when` member). Refused rather than compiled with
    /// unchecked content and silently missing from the emitted package
    /// (SR-753 FND-002, SR-761 FND-001, SR-770 FND-001).
    Unimplemented {
        /// The declared protocol name.
        name: String,
        /// The construct's own keyword, e.g. `"channel"` or `"send"`.
        construct: &'static str,
        /// The construct's own span: the earliest such construct in the
        /// protocol.
        span: Span,
    },
    /// The protocol's `using` alias names no profile selection of
    /// the unit.
    MissingProfile {
        /// The alias as written.
        alias: String,
        /// The alias's own span.
        span: Span,
    },
    /// A role's `on`/`each` object type names no object type of
    /// the package.
    MissingRoleType {
        /// The role's declared name.
        role: String,
        /// The object type as written.
        context: String,
        /// The object type's own span.
        span: Span,
    },
    /// An `attempt`'s `by` role names no role of the protocol.
    MissingRole {
        /// The role as written.
        role: String,
        /// The role reference's own span.
        span: Span,
    },
    /// A binder's declared type does not resolve against the
    /// package scope.
    BinderType {
        /// The binder's declared name.
        binder: String,
        /// Why the type did not resolve.
        fault: super::type_form::TypeFormFault,
        /// The span of the type form (or nested argument form) at fault.
        span: Span,
    },
    /// FR-218 "Channels": a channel's `from` or `to` names no role the
    /// protocol declares. `missing_declaration`/`missing-name`, at the role
    /// name; both ends refuse independently.
    ChannelRole {
        /// The channel's declared name.
        channel: String,
        /// Which end of the channel names the role.
        end: ChannelEnd,
        /// The role name as written.
        name: String,
        /// The role name's own span.
        span: Span,
    },
    /// FR-218 "Channels": a channel's `carries` type does not resolve.
    /// `missing_declaration`/`missing-name` for a missing name, at the type
    /// name.
    ChannelType {
        /// The channel's declared name.
        channel: String,
        /// Why the type did not resolve.
        fault: super::type_form::TypeFormFault,
        /// The span of the type form (or nested argument form) at fault.
        span: Span,
    },
    /// FR-218 "Admission for emission": a channel carries an object type,
    /// which is not a message type. `invalid_model_binding`/
    /// `malformed-declaration`, once, at the type name.
    ChannelCarriesObject {
        /// The channel's declared name.
        channel: String,
        /// The `carries` type as written.
        name: String,
        /// The type name's own span.
        span: Span,
    },
    /// FR-218 "Admission for emission": a role's model is a type that does
    /// not take part in a protocol. `invalid_model_binding`/
    /// `malformed-declaration`, at the model name.
    RoleModel {
        /// The role's declared name.
        role: String,
        /// The model as written.
        context: String,
        /// The model name's own span.
        span: Span,
    },
    /// FR-218 "Admission for emission": a `parallel`'s join list or quorum
    /// count is unsound. `invalid_package`/`invalid-value`, at the list, or
    /// at the count for a quorum.
    JoinList {
        /// The `parallel` control's declared name.
        parallel: String,
        /// What is unsound.
        fault: JoinFault,
        /// The list's span, or the quorum count's.
        span: Span,
    },
    /// FR-218 "Constructs": an `event` or `commit` written `by`, or a
    /// `choice` or `repeat` owned `by`, a name that is no declared role.
    /// `missing_declaration`/`missing-name`, at the role name.
    NodeRole {
        /// The node's declared name.
        node: String,
        /// The role name as written.
        name: String,
        /// The role name's own span.
        span: Span,
    },
    /// FR-218 "Channels": a `send` or `receive` written `via` a name that
    /// is no declared channel. `missing_declaration`/`missing-name`, at the
    /// name.
    MissingChannel {
        /// The node's declared name.
        node: String,
        /// The channel name as written.
        name: String,
        /// The channel name's own span.
        span: Span,
    },
    /// FR-218 "Channels": a `send` or `receive` record binder, or a FIFO
    /// binder, whose declared type is not the channel's `carries` type.
    /// `ill_typed`/`type-mismatch`, at the binder's type.
    CarriesMismatch {
        /// The channel's declared name.
        channel: String,
        /// The channel's `carries` type.
        expected: Box<quire_exact::ValueType>,
        /// The binder's declared type.
        actual: Box<quire_exact::ValueType>,
        /// The binder type's span.
        span: Span,
    },
    /// FR-218 "Channels": a refusal of the clause checker inside a channel's
    /// FIFO key, checked over its binder and constants only. Its code and
    /// cause are the wrapped cause's.
    KeyClause {
        /// The channel's declared name.
        channel: String,
        /// The clause checker's cause.
        cause: Box<CheckCause>,
        /// The located node.
        span: Span,
    },
    /// FR-218 "Channels": a channel's FIFO key reads model state.
    /// `ill_typed`/`operator-ineligible`, at the read, naming the key by its
    /// channel.
    KeyModelState {
        /// The channel's declared name.
        channel: String,
        /// The read.
        span: Span,
    },
    /// FR-218 "Channels": a channel's FIFO key is of a type over which the
    /// clause checker admits no `=` (QSpec FR-054: stable equality).
    /// `ill_typed`/`operator-ineligible`, at the key, retaining the operator
    /// and the type.
    KeyEquality {
        /// The channel's declared name.
        channel: String,
        /// The key's type.
        key_type: Box<quire_exact::ValueType>,
        /// The key.
        span: Span,
    },
    /// FR-218 "Scopes": a refusal inside one slot of a node -- a body, a
    /// `check` condition, a `visible` entry, a guard or a loop proof -- by
    /// the clause checker, or a read of a binder that is not readable at
    /// the slot (`MissingName`, as FR-113 refuses an unresolved name). Its
    /// code and cause are the wrapped cause's.
    SlotClause {
        /// The node's declared name.
        node: String,
        /// Which slot of the node.
        slot: SlotKind,
        /// The cause.
        cause: Box<CheckCause>,
        /// The located read or node.
        span: Span,
    },
    /// FR-218 "Model state": a slot that bars model state (a `check`
    /// condition, a guard, a `visible` entry) reads it, or reads the `over`
    /// input. `ill_typed`/`operator-ineligible`, at the read, naming the
    /// slot.
    SlotModelState {
        /// The node's declared name.
        node: String,
        /// Which slot of the node.
        slot: SlotKind,
        /// The read.
        span: Span,
    },
    /// FR-218 "Choice and repeat decisions": a `visible` entry or a guard
    /// reads a fact that is not an eligible, individually listed fact of the
    /// owner. `wrong_snapshot`/`wrong-observation`, at the read.
    WrongObservation {
        /// The `choice` or `repeat` node's declared name.
        node: String,
        /// The owning role.
        owner: String,
        /// The binder read.
        binder: String,
        /// The field read, when the read is a field read.
        field: Option<String>,
        /// Why the fact is not observable.
        reason: ObservationReason,
        /// The read.
        span: Span,
    },
    /// FR-218 "Constructs": the compiled causal edge relation, without the
    /// `repeat` back edges, has a cycle. `invalid_package`/
    /// `definition-cycle`, naming the controls on the cycle in tree order,
    /// at the first of them.
    CausalCycle {
        /// The names of the controls on the cycle, in tree order.
        nodes: Vec<String>,
        /// The first control's name span.
        span: Span,
    },
    /// FR-218 "Constructs": a `repeat` whose body does not necessarily take
    /// a step. `unsupported_construct`/`declaration-form`, at the `repeat`,
    /// naming it and the selected protocol profile (QSpec FR-052).
    NoProgress {
        /// The `repeat` control's declared name.
        repeat: String,
        /// The identity of the selected protocol profile, when one is
        /// selected.
        profile: Option<String>,
        /// The `repeat`'s name span.
        span: Span,
    },
    /// FR-218 "Choice and repeat decisions": a `choice` whose guards are not
    /// proved exhaustive and exclusive. `undefined_expression`/
    /// `unproved-exhaustiveness`, at the choice.
    Exhaustiveness {
        /// The `choice` control's declared name.
        choice: String,
        /// The names of the cases involved, in source order.
        cases: Vec<String>,
        /// What the valuations show, in the order `missing-arm`,
        /// `duplicate-arm`; never empty.
        failed: Vec<ArmFault>,
        /// The `choice`'s name span.
        span: Span,
    },
    /// QSpec FR-466 "Per-operator checks" 2: an `await` whose matched event
    /// is a `send` or an `attempt`. `unsupported_construct`/
    /// `declaration-form`, at the `await`, naming the form and the selected
    /// protocol profile.
    AwaitedForm {
        /// The `await` control's declared name.
        node: String,
        /// The matched event node's declared name.
        event: String,
        /// The matched node's form keyword (`send` or `attempt`).
        form: &'static str,
        /// The identity of the selected protocol profile, when one is
        /// selected.
        profile: Option<String>,
        /// The `await`'s name span.
        span: Span,
    },
    /// QSpec FR-466 "Per-operator checks" 6: an `await`'s `within` with
    /// `lower > upper` or an open upper bound. `ill_typed`/`type-mismatch`
    /// at the interval (member `within`, expected a closed interval with
    /// `lower <= upper`, actual the interval as written).
    AwaitWindow {
        /// The `await` control's declared name.
        node: String,
        /// The lower bound as written.
        lower: u64,
        /// The upper bound as written; `None` for `*`.
        upper: Option<u64>,
        /// The interval's span.
        span: Span,
    },
    /// QSpec FR-464 "Profiles and laws": an `await` reads a clock role, so
    /// the unit selects a `temporal_profile`; without one,
    /// `missing_declaration`/`missing-selection` at the `await`.
    MissingTemporalProfile {
        /// The `await` control's declared name.
        node: String,
        /// The `await`'s name span.
        span: Span,
    },
}

/// Which expression slot of a protocol node a refusal is about (FR-218
/// "Scopes").
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SlotKind {
    /// An event node's constraint body.
    Constraint,
    /// A `check`'s condition.
    Condition,
    /// The `finish` constraint.
    Finish,
    /// A `visible` entry of a `choice` or `repeat`.
    Visible,
    /// A `case` guard or a `repeat`'s `while` guard.
    Guard,
    /// A `repeat`'s invariant.
    Invariant,
    /// A `repeat`'s variant.
    Variant,
}

/// Why a decision's read is not an observable fact of its owner (FR-218
/// "Choice and repeat decisions").
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ObservationReason {
    /// `other-role`: the binder is eligible only for a different role.
    OtherRole,
    /// `ineligible-kind`: the binder's kind is never eligible.
    IneligibleKind,
    /// `unlisted`: eligible, but no individual `visible` entry lists it.
    Unlisted,
}

impl ObservationReason {
    /// The `reason` member's value.
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::OtherRole => "other-role",
            Self::IneligibleKind => "ineligible-kind",
            Self::Unlisted => "unlisted",
        }
    }
}

/// What a valuation of a `choice`'s visible facts shows about its guards
/// (FR-218 "Choice and repeat decisions").
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ArmFault {
    /// `missing-arm`: a valuation no guard covers.
    MissingArm,
    /// `duplicate-arm`: a valuation two guards cover.
    DuplicateArm,
}

impl ArmFault {
    /// The `failed` entry's value.
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::MissingArm => "missing-arm",
            Self::DuplicateArm => "duplicate-arm",
        }
    }
}

/// Which end of a channel names a role (FR-218 "Channels").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChannelEnd {
    /// The `from` role.
    From,
    /// The `to` role.
    To,
}

/// Why a `parallel`'s join list or quorum count is refused (FR-218
/// "Admission for emission").
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum JoinFault {
    /// `join all` whose list omits a branch.
    OmitsBranch {
        /// The first omitted branch label, in source order.
        branch: String,
    },
    /// `join quorum(n)` with `n` below one.
    QuorumBelowOne,
    /// `join quorum(n)` with `n` above the number of distinct labels joined.
    QuorumAboveLabels {
        /// The number of distinct labels the list holds.
        labels: usize,
    },
    /// A `join any` or `join predicate` that lists no label.
    NoLabel,
}

/// Why a protocol reference expression that resolved is still refused
/// (FR-218). An unresolved or non-protocol target is `MissingName` and
/// `IllTyped` like any other name.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ProtocolReferenceCause {
    /// A second reference to a target already referred to in the unit: the
    /// carrier is keyed by its body alone and IR admits one `expression`
    /// occurrence per carrier, so two references to one target cannot both
    /// be written. `unsupported_construct`/`expression-form`, at the second
    /// reference.
    Repeated {
        /// The first reference to the same target.
        first: Location,
    },
    /// A protocol, or a reference to it, whose `run` root names no FR-052
    /// control construct (`sequence`, `choice`, `parallel`, `repeat`) to be
    /// the `profile_operator` of its node, while a protocol profile is
    /// selected. `unsupported_construct`/`declaration-form`.
    UnsupportedRoot {
        /// The root's own keyword, such as `attempt` or `commit`.
        root: &'static str,
    },
}

impl ProtocolReferenceCause {
    /// The first reference to the same target, for a repeated reference.
    pub fn first(&self) -> Option<&Location> {
        match self {
            Self::Repeated { first } => Some(first),
            Self::UnsupportedRoot { .. } => None,
        }
    }

    /// This cause's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::Repeated { .. } | Self::UnsupportedRoot { .. } => Code::UnsupportedConstruct,
        }
    }

    /// This cause's catalog tag.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Repeated { .. } => "expression-form",
            Self::UnsupportedRoot { .. } => "declaration-form",
        }
    }
}

/// The catalog code and tag a [`ProtocolAnchorCause::BinderType`] refuses
/// with: the same classes `type_form::TypeFormError`'s own refusal uses
/// (a missing or ambiguous name, else a type mismatch).
pub(super) fn type_fault_class(fault: &super::type_form::TypeFormFault) -> (Code, &'static str) {
    use super::type_form::TypeFormFault as F;
    match fault {
        F::MissingName(_) => (Code::MissingDeclaration, "missing-name"),
        F::AmbiguousName(_) => (Code::AmbiguousDeclaration, "ambiguous-name"),
        F::Malformed
        | F::EmptyInterval
        | F::IntegerOutsideI128(_)
        | F::DenominatorBelowOne
        | F::MalformedDecimal
        | F::EmptyTextBounds
        | F::EmptyCardinality => (Code::IllTyped, "type-mismatch"),
    }
}

/// Which selection alias a [`ShadowedDeclaration::ProfileAlias`] or
/// [`ShadowedDeclaration::ModelAlias`] names (FR-091: `profile p = ...;` and
/// `model M = ...;`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AliasKind {
    /// A `profile p = ...;` selection alias.
    Profile,
    /// A `model M = ...;` selection alias.
    Model,
}

/// What a binder shadows (FR-113 "Refusals"): named as the checker holds
/// it. FR-113 says "naming the declaration it would shadow", and AC-5 says
/// "naming the alias" -- a consumer needs the *kind* of the shadowed
/// declaration, not just that one exists (SR-765 FND-005). The first five
/// variants live outside the protocol form this checker reads (a
/// package-wide alias or native declaration), so none of them carry a
/// span; only [`Self::Binder`] does.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ShadowedDeclaration {
    /// The unit's own `profile p = ...;` selection alias.
    ProfileAlias,
    /// The unit's own `model M = ...;` selection alias.
    ModelAlias,
    /// A native `function` or `predicate` declaration of the package.
    Function,
    /// A native `type`, `record`, `tuple` or `enum` declaration of the
    /// package.
    Type,
    /// A native `dimension` or `unit` declaration of the package.
    Quantity,
    /// Another binder visible where the shadowing binder is declared, by
    /// that binder's own span.
    Binder(Span),
}

impl ProtocolAnchorCause {
    /// The refusal's own span (FR-096: read directly, not through a
    /// `Location`'s path).
    pub fn span(&self) -> Span {
        match self {
            Self::Missing { span, .. }
            | Self::Ambiguous { span, .. }
            | Self::WrongKind { span, .. }
            | Self::ChannelMismatch { span, .. }
            | Self::Shadow { span, .. }
            | Self::MissingContract { span, .. }
            | Self::WrongContractAnchor { span, .. }
            | Self::Unimplemented { span, .. }
            | Self::MissingProfile { span, .. }
            | Self::MissingRoleType { span, .. }
            | Self::MissingRole { span, .. }
            | Self::BinderType { span, .. }
            | Self::ChannelRole { span, .. }
            | Self::ChannelType { span, .. }
            | Self::ChannelCarriesObject { span, .. }
            | Self::RoleModel { span, .. }
            | Self::JoinList { span, .. }
            | Self::NodeRole { span, .. }
            | Self::MissingChannel { span, .. }
            | Self::CarriesMismatch { span, .. }
            | Self::CausalCycle { span, .. }
            | Self::NoProgress { span, .. }
            | Self::Exhaustiveness { span, .. }
            | Self::KeyClause { span, .. }
            | Self::KeyModelState { span, .. }
            | Self::KeyEquality { span, .. }
            | Self::SlotClause { span, .. }
            | Self::SlotModelState { span, .. }
            | Self::WrongObservation { span, .. }
            | Self::AwaitedForm { span, .. }
            | Self::AwaitWindow { span, .. }
            | Self::MissingTemporalProfile { span, .. } => *span,
        }
    }

    /// This cause's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::Missing { .. }
            | Self::MissingContract { .. }
            | Self::MissingProfile { .. }
            | Self::MissingRoleType { .. }
            | Self::MissingRole { .. }
            | Self::ChannelRole { .. }
            | Self::NodeRole { .. }
            | Self::MissingChannel { .. }
            | Self::MissingTemporalProfile { .. } => Code::MissingDeclaration,
            Self::Ambiguous { .. } | Self::Shadow { .. } => Code::AmbiguousDeclaration,
            Self::WrongKind { .. }
            | Self::ChannelMismatch { .. }
            | Self::CarriesMismatch { .. }
            | Self::AwaitWindow { .. }
            | Self::KeyModelState { .. }
            | Self::KeyEquality { .. }
            | Self::SlotModelState { .. } => Code::IllTyped,
            Self::KeyClause { cause, .. } | Self::SlotClause { cause, .. } => cause.code(),
            Self::WrongObservation { .. } => Code::WrongSnapshot,
            Self::WrongContractAnchor { .. } => Code::WrongSnapshot,
            Self::Unimplemented { .. } | Self::NoProgress { .. } | Self::AwaitedForm { .. } => {
                Code::UnsupportedConstruct
            }
            Self::CausalCycle { .. } => Code::InvalidPackage,
            Self::Exhaustiveness { .. } => Code::UndefinedExpression,
            Self::BinderType { fault, .. } | Self::ChannelType { fault, .. } => {
                type_fault_class(fault).0
            }
            Self::ChannelCarriesObject { .. } | Self::RoleModel { .. } => Code::InvalidModelBinding,
            Self::JoinList { .. } => Code::InvalidPackage,
        }
    }

    /// This cause's catalog tag (FR-113: reuses the existing
    /// `missing-name`, `ambiguous-name` and `type-mismatch` tags; no
    /// protocol-specific catalog code exists).
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Missing { .. }
            | Self::MissingContract { .. }
            | Self::MissingProfile { .. }
            | Self::MissingRoleType { .. }
            | Self::MissingRole { .. }
            | Self::ChannelRole { .. }
            | Self::NodeRole { .. }
            | Self::MissingChannel { .. } => "missing-name",
            Self::Ambiguous { .. } | Self::Shadow { .. } => "ambiguous-name",
            Self::WrongKind { .. }
            | Self::ChannelMismatch { .. }
            | Self::CarriesMismatch { .. }
            | Self::AwaitWindow { .. } => "type-mismatch",
            Self::KeyModelState { .. } | Self::KeyEquality { .. } | Self::SlotModelState { .. } => {
                "operator-ineligible"
            }
            // Only a cause that has a tag is wrapped
            // (`protocol_controls::check_fifo`, `protocol_slots`).
            Self::KeyClause { cause, .. } | Self::SlotClause { cause, .. } => {
                cause.cause().unwrap_or_default()
            }
            Self::WrongObservation { .. } => "wrong-observation",
            Self::WrongContractAnchor { .. } => "wrong-anchor",
            Self::Unimplemented { .. } => "not-yet-implemented",
            Self::NoProgress { .. } | Self::AwaitedForm { .. } => "declaration-form",
            Self::MissingTemporalProfile { .. } => "missing-selection",
            Self::CausalCycle { .. } => "definition-cycle",
            Self::Exhaustiveness { .. } => "unproved-exhaustiveness",
            Self::BinderType { fault, .. } | Self::ChannelType { fault, .. } => {
                type_fault_class(fault).1
            }
            Self::ChannelCarriesObject { .. } | Self::RoleModel { .. } => "malformed-declaration",
            Self::JoinList { .. } => "invalid-value",
        }
    }
}

/// FR-326: why the `TemporalTrace` family's `check` refused one `temporal`
/// clause. Each variant carries its own span, which
/// `region::DeclarationRegions::refusal_region` reads directly: a clause has
/// no expression tree for a `Location` path to walk.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TemporalCause {
    /// FR-326 "Profile identity": the unit selects no temporal profile, or
    /// more than one. `found` names the identity of each selection it
    /// found, in source order. Located at the clause.
    ProfileSelection {
        /// The identity of each temporal profile the unit selects.
        found: Vec<&'static str>,
        /// The clause's span.
        span: Span,
    },
    /// FR-326 "Bounded profiles": an operator with no interval under a
    /// bounded profile. Located at the operator.
    MissingInterval {
        /// The operator's own token.
        span: Span,
    },
    /// FR-326 "Bounded profiles": an `[a,*]` interval under a bounded
    /// profile. Located at the interval.
    OpenInterval {
        /// The interval's span.
        span: Span,
    },
    /// FR-326 "Bounded profiles": an interval with `a > b`, under every
    /// profile. Located at the interval.
    InvertedInterval {
        /// The written lower bound.
        lower: u64,
        /// The written upper bound.
        upper: u64,
        /// The interval's span.
        span: Span,
    },
    /// A parameter of the clause (`over`, `on each` or a capture) declares
    /// a type that does not resolve.
    ParameterType {
        /// The parameter's name.
        parameter: String,
        /// Why its type form did not resolve.
        fault: super::type_form::TypeFormFault,
        /// The type form's span.
        span: Span,
    },
    /// FR-123 "Fairness constraints": a fairness constraint names no
    /// operation of an admitted model. Located at the operation's name.
    UnresolvedFairnessOperation {
        /// The operation as written.
        name: String,
        /// The operation name's span.
        span: Span,
    },
    /// FR-123 "Fairness constraints": a fairness constraint's operation
    /// names operations of several declaring types, none more derived than
    /// the others. Located at the operation's name.
    AmbiguousFairnessOperation {
        /// The operation as written.
        name: String,
        /// The operation name's span.
        span: Span,
    },
    /// FR-123 "Fairness constraints": a fairness constraint on a clause
    /// whose unit selects a bounded profile. Located at the constraint.
    FairnessUnderBoundedProfile {
        /// The constraint's span.
        span: Span,
    },
    /// A clause whose checked parts all hold, but that no S4 emission
    /// writes yet (FR-337): refused rather than compiled and silently
    /// missing from the emitted package.
    NotEmitted {
        /// The declared clause name.
        name: String,
        /// The clause's span.
        span: Span,
    },
}

impl TemporalCause {
    /// The refusal's own span (FR-096: read directly, not through a
    /// `Location`'s path).
    pub fn span(&self) -> Span {
        match self {
            Self::ProfileSelection { span, .. }
            | Self::MissingInterval { span }
            | Self::OpenInterval { span }
            | Self::InvertedInterval { span, .. }
            | Self::ParameterType { span, .. }
            | Self::UnresolvedFairnessOperation { span, .. }
            | Self::AmbiguousFairnessOperation { span, .. }
            | Self::FairnessUnderBoundedProfile { span }
            | Self::NotEmitted { span, .. } => *span,
        }
    }

    /// This cause's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::ProfileSelection { .. } => Code::UnknownProfile,
            Self::MissingInterval { .. }
            | Self::OpenInterval { .. }
            | Self::FairnessUnderBoundedProfile { .. }
            | Self::NotEmitted { .. } => Code::UnsupportedConstruct,
            Self::UnresolvedFairnessOperation { .. } => Code::MissingDeclaration,
            Self::AmbiguousFairnessOperation { .. } => Code::AmbiguousDeclaration,
            Self::InvertedInterval { .. } => Code::IllTyped,
            Self::ParameterType { fault, .. } => type_fault_class(fault).0,
        }
    }

    /// This cause's catalog tag.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::ProfileSelection { .. } => "unsupported-selection",
            Self::MissingInterval { .. }
            | Self::OpenInterval { .. }
            | Self::FairnessUnderBoundedProfile { .. } => "expression-form",
            Self::UnresolvedFairnessOperation { .. } => "missing-name",
            Self::AmbiguousFairnessOperation { .. } => "ambiguous-name",
            Self::InvertedInterval { .. } => "type-mismatch",
            Self::NotEmitted { .. } => "not-yet-implemented",
            Self::ParameterType { fault, .. } => type_fault_class(fault).1,
        }
    }
}

/// Why the `StateGraph` family refused a state-graph claim (FR-165). Each
/// cause carries the span it is located at, read directly, since a claim's
/// predicates are not the expression tree of a declaration a `Location`
/// path resolves in.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum StateGraphCause {
    /// A predicate reads an operation anchor (`self`), a result or a
    /// pre-state (`pre(e)`), not the state alone (ADR-022 GM-3). Located at
    /// the read.
    ReadsBeyondState {
        /// The read's span.
        span: Span,
    },
    /// A predicate contains a temporal operator. Located at the operator.
    TemporalOperator {
        /// The operator's span.
        span: Span,
    },
    /// The claim carries a fairness constraint (ADR-022 GM-7). Located at
    /// the constraint.
    Fairness {
        /// The constraint's span.
        span: Span,
    },
    /// A claim whose parts all check, but that no S4 emission writes yet:
    /// refused rather than compiled and silently missing from the package.
    /// Located at the claim.
    NotEmitted {
        /// The declared claim name.
        name: String,
        /// The claim's span.
        span: Span,
    },
    /// The state clause checker refused a predicate, as it refuses an
    /// invariant body.
    Predicate {
        /// The refused predicate's role in the claim.
        role: PredicateRole,
        /// The span of the node the refusal names, or of the whole predicate
        /// when its spans do not reach that node.
        span: Span,
        /// The clause checker's refusal.
        cause: Box<CheckCause>,
    },
}

/// Which predicate of a state-graph claim a refusal is about.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PredicateRole {
    /// `possible`'s and `always possible`'s `P`.
    Target,
    /// `always possible`'s `Q` and `unique path`'s `X`.
    From,
    /// `unique path`'s `Y`.
    To,
}

impl StateGraphCause {
    /// The refusal's own span (FR-096), when it has one.
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::ReadsBeyondState { span }
            | Self::TemporalOperator { span }
            | Self::Fairness { span }
            | Self::NotEmitted { span, .. }
            | Self::Predicate { span, .. } => Some(*span),
        }
    }

    /// This cause's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::ReadsBeyondState { .. }
            | Self::TemporalOperator { .. }
            | Self::Fairness { .. }
            | Self::NotEmitted { .. } => Code::UnsupportedConstruct,
            Self::Predicate { cause, .. } => cause.code(),
        }
    }

    /// This cause's catalog tag.
    pub fn tag(&self) -> Option<&'static str> {
        match self {
            Self::ReadsBeyondState { .. }
            | Self::TemporalOperator { .. }
            | Self::Fairness { .. } => Some("expression-form"),
            Self::NotEmitted { .. } => Some("not-yet-implemented"),
            Self::Predicate { cause, .. } => cause.cause(),
        }
    }
}

/// An inclusive mathematical integer interval whose missing ends are
/// unbounded.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct ProvedInterval {
    /// The lower end, if bounded.
    pub lower: Option<Integer>,
    /// The upper end, if bounded.
    pub upper: Option<Integer>,
}

/// The definedness obligation that could not be proved.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Obligation {
    /// A divisor may be zero: `unproved-nonzero`.
    Nonzero,
    /// `value(e)` without a proved `present(e)`: `unproved-presence`.
    Presence,
    /// An integer value may lie outside its required interval:
    /// `unproved-range`.
    Range {
        /// The required interval.
        required: Box<ProvedInterval>,
        /// The interval the accepted proof forms establish.
        proved: Box<ProvedInterval>,
    },
    /// A rational quotient may lie outside its `Rational[..]` domain:
    /// `unproved-range`.
    RationalRange,
    /// `reduce` without a proof of `size(c) >= 1`: `unproved-range`.
    NonemptyReduction {
        /// The proved `size(c)` interval.
        size: Box<ProvedInterval>,
    },
}

/// The FR-146 component obligation that failed, in check order.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MeasureObligation {
    /// `missing-measure`.
    MissingMeasure,
    /// `measure-arity`.
    MeasureArity,
    /// `measure-kind`.
    MeasureKind,
    /// `nonnegative`.
    Nonnegative,
    /// `decrease`.
    Decrease,
}

impl MeasureObligation {
    /// The obligation spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingMeasure => "missing-measure",
            Self::MeasureArity => "measure-arity",
            Self::MeasureKind => "measure-kind",
            Self::Nonnegative => "nonnegative",
            Self::Decrease => "decrease",
        }
    }
}

/// The checking stage a declared checking limit bounds.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CheckingStage {
    /// Name resolution and typing.
    Typing,
}

/// Which declared checking limit was reached.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CheckingLimitKind {
    /// Admitted expression nodes per checked package.
    Nodes,
    /// A checked-family declaration's own preimage byte length
    /// (`qsl_foundation::diagnostic::LimitKind::InputBytes`).
    InputBytes,
    /// Cumulative checking work: the checked-family contract meter's
    /// spend across every declaration checked against it
    /// (`qsl_foundation::diagnostic::LimitKind::WorkBudget`), and lowering's
    /// `declaration.check` charges.
    WorkBudget,
}

impl CheckingLimitKind {
    /// The setting that raises this limit (FR-255): the one mapping from a
    /// checking limit to its name, and through the setting to its limit kind.
    pub const fn setting(self) -> Setting {
        match self {
            Self::Nodes => Setting::S3Nodes,
            Self::InputBytes => Setting::S3InputBytes,
            Self::WorkBudget => Setting::S3WorkUnits,
        }
    }
}

impl TryFrom<Setting> for CheckingLimitKind {
    type Error = Setting;

    /// The reverse of [`CheckingLimitKind::setting`], for `check::mod`'s
    /// `StageFailure::Limit` arm. Matched exhaustively rather than with a
    /// `_` catch-all, so a new setting forces a decision here. Every other
    /// setting names a limit of a stage other than checking, and refuses.
    fn try_from(setting: Setting) -> Result<Self, Setting> {
        match setting {
            Setting::S3Nodes => Ok(Self::Nodes),
            Setting::S3InputBytes => Ok(Self::InputBytes),
            Setting::S3WorkUnits => Ok(Self::WorkBudget),
            Setting::S1InputBytes
            | Setting::S1Tokens
            | Setting::S1Nodes
            | Setting::S1WorkUnits
            | Setting::S3DecimalScale
            | Setting::DependencyLibraries
            | Setting::DependencyImportEdges
            | Setting::DependencySourceBytes
            | Setting::EnvironmentAncestorSteps
            | Setting::EnvironmentWorkUnits
            | Setting::ModelDeclarationRecords
            | Setting::ModelDerivationFacts
            | Setting::ModelEffectiveDeclarations
            | Setting::ModelDispatchCandidates
            | Setting::ModelHashedBytes
            | Setting::ModelWorkUnits
            | Setting::ModelAncestorSteps
            | Setting::ModelFamilySteps
            | Setting::AdmissionPopulationMembers
            | Setting::AdmissionWorkUnits
            | Setting::AdmissionAncestorSteps
            | Setting::ObservationInputBytes
            | Setting::ObservationObjects
            | Setting::ObservationValues
            | Setting::LibraryDefinitions
            | Setting::LibraryDependencyEdges
            | Setting::LibraryArtifactBytes
            | Setting::LibrarySingleArtifactBytes
            | Setting::IdentityInputBytes
            | Setting::IntakeInputBytes
            | Setting::ReplayInputBytes
            | Setting::FormatOutputBytes
            | Setting::I2InputBytes
            | Setting::I2Nodes
            | Setting::I2Edges
            | Setting::I2Occurrences
            | Setting::I2Diagnostics
            | Setting::I2WorkUnits
            | Setting::ExploreStates
            | Setting::ExploreTransitions => Err(setting),
        }
    }
}

/// FR-272's closed `wrong_snapshot` cause list this crate decides for
/// `pre(...)` (native-diagnostics.md: `wrong-observation`, `wrong-invocation`,
/// `wrong-anchor` or `forbidden-pre-read`). Only the two causes `pre(...)`
/// itself can name are represented; `wrong-observation` and
/// `wrong-invocation` are catalogued for the native runtime's own
/// snapshot-selection refusals (`src/runtime`), not this crate.
///
/// The two represented causes split by *layer*, not by clause: every
/// checking-time refusal this crate's `Typer` decides over `pre(...)`'s own
/// syntax -- wrong clause, ineligible operand, or a captured `let` alias
/// (FR-208 applies FR-042 to invariants/preconditions and names this same
/// cause explicitly) -- is [`Self::ForbiddenPreRead`]. [`Self::WrongAnchor`]
/// is reserved for the one case the checker cannot see at all: a
/// postcondition it did admit, evaluated at runtime
/// (`evaluate.rs`'s `select_anchor`) over a population value with no
/// attached pre binding.
///
/// FR-104 adds a second checking-time `wrong_snapshot`/`wrong-anchor`
/// refusal -- `result` named anywhere but an admitting postcondition -- but
/// it is not a third variant of this enum: [`super::refusal::CheckCause`]'s
/// own [`super::refusal::CheckCause::UnanchoredResult`] carries the payload
/// FR-104-AC-2 needs (the clause kind and the operation), which this type
/// cannot hold without widening every runtime consumer of the FR-090
/// evaluation-cause contract. The two share one catalog cause
/// (`wrong-anchor`) and nothing else: `UnanchoredResult` is `check`'s own
/// checking-time path to it, [`Self::WrongAnchor`] stays the runtime-only
/// path, and this enum's two-cause closure is otherwise unchanged.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WrongSnapshotCause {
    /// A postcondition's `pre(...)` evaluated over a `Value::Population`
    /// with no attached pre anchor -- the caller supplied a population
    /// admitted through `admit_binding` directly rather than through
    /// `admit_invocation` (the only constructor that attaches one). A
    /// runtime-only cause: the checker has no way to see, at checking time,
    /// which admission path a caller's runtime argument will take.
    WrongAnchor,
    /// `pre(...)` written where the checked declaration does not admit it
    /// at all (anywhere but an operation's postcondition -- shared-
    /// grammar.md's "self, result and pre(...) are caller-side anchor
    /// operations, unavailable as implicit ambient state inside a reusable
    /// predicate"), or admitted but refused over its own operand: a bare
    /// `Name`, literal, or a value already captured/computed outside this
    /// `pre(...)`'s own operand (FR-042's Behavior clause: "Pre is refused
    /// ... on bare parameters/constants/captures"; FR-042-AC-3's `let s =
    /// self in pre(s.version)` analogue for a `let`-aliased state root) --
    /// re-anchoring any of these would silently do nothing rather than
    /// replay a real read under the pre observation.
    ForbiddenPreRead,
    /// FR-063, TC-387: exists only so `--cfg seam_probe` makes the
    /// `ProtocolClause` snapshot cause's `catalog_code()` match
    /// non-exhaustive. Never constructed outside the probe build.
    #[cfg(seam_probe)]
    __SeamProbe,
}

impl WrongSnapshotCause {
    /// The closed FR-272 cause tag. Not a seam-probe location: it owns the
    /// probe variant's own arm.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WrongAnchor => "wrong-anchor",
            Self::ForbiddenPreRead => "forbidden-pre-read",
            #[cfg(seam_probe)]
            Self::__SeamProbe => "__seam_probe__",
        }
    }
}

/// [`CheckCause::ResourceExhausted`]'s payload: the checking
/// stage, the limit kind reached, its declared bound and the counter value
/// the refused step would have reached.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StageLimitCause {
    /// The checking stage.
    pub stage: CheckingStage,
    /// The limit reached.
    pub kind: CheckingLimitKind,
    /// The declared limit.
    pub limit: u64,
    /// The counter value the refused step would have reached.
    pub actual: u128,
    /// FR-096: the region where a family `check` reached the limit, when
    /// the refusal's `location` cannot name it: a declaration's whole span,
    /// or the node whose entry `Typer`'s node stop failed at. `None` when `location`
    /// names the position, or when no region of the unit does.
    pub region: Option<SourceRegion>,
}

/// One candidate of an ambiguous `r.name` (FR-363).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum MemberCandidate {
    /// A field of the receiver's object type.
    Field(quire_semantic_value::declaration::FieldRef),
    /// A relationship end navigable from the receiver, named by the role of
    /// the end on `side` (the end nearest the receiver).
    RelationshipEnd {
        /// The relationship's declaration.
        relationship: crate::model::key::DeclarationKey,
        /// The end whose role the name matched.
        side: super::navigation::EndSide,
    },
}

/// One candidate of an ambiguous qualified reference `q::m` (FR-317).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum QualifiedCandidate {
    /// A declared union named `q` that has a member `m`.
    Union(NodeKey),
    /// A declared enum named `q` that has a member `m`.
    Enum(NodeKey),
    /// The import alias `q`, whose package exports `m`.
    Import(String),
}

/// [`CheckCause::AmbiguousQualified`]'s payload.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AmbiguousQualified {
    /// The ambiguous reference as written, `q::m`.
    pub name: String,
    /// Every candidate it matched, unions then enums then the import.
    pub candidates: Vec<QualifiedCandidate>,
}

/// [`CheckCause::AmbiguousMember`]'s payload.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AmbiguousMember {
    /// The ambiguous name.
    pub name: String,
    /// Every candidate the name matched.
    pub candidates: Vec<MemberCandidate>,
}

/// Why a destination end's multiplicity types no navigation (FR-363).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NavigationBound {
    /// The destination end declares no multiplicity, and none is substituted.
    Undeclared,
    /// The destination end's upper bound is unbounded.
    Unbounded,
    /// The destination end's multiplicity is ordered.
    Ordered,
}

/// The typed cause of a checking refusal.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CheckCause {
    /// A value operator prohibited by the declaration's selected profile.
    ProfileProhibitedOperator(qsl_forms::BinaryOperator),
    /// FR-300: a nested model form retains StateModel ownership.
    StateModel(Box<super::state_model::StateModelRefusal>),
    /// FR-366: a refusal in a type-level invariant clause, with the clause
    /// it is about.
    TypeInvariant(Box<super::type_invariant::TypeInvariantRefusal>),
    /// A `terminal when` predicate's checking refused (FR-124), with the
    /// declaration it is about.
    Terminal(Box<super::terminal::TerminalRefusal>),
    /// A `terminal` declaration that checks but that no S4 emission writes
    /// yet (FR-124): refused rather than compiled and silently missing.
    TerminalNotEmitted {
        /// The declared name.
        name: String,
        /// The span of the whole declaration.
        span: Span,
    },
    /// A refinement declaration's checking refused (FR-135 to FR-137).
    Refinement(Box<super::refinement::RefinementRefusal>),
    /// A `refinement` declaration that checks but that no S4 emission writes
    /// yet (FR-135): refused rather than compiled and silently missing.
    RefinementNotEmitted {
        /// The declared name.
        name: String,
        /// The span of the whole declaration.
        span: Span,
    },
    /// A completed model conformance stage retains every axis failure.
    ModelConformance(Box<super::model_conformance::ModelConformanceFailure>),
    /// An unfinished model stage exposes its exact five-field denial.
    ModelIncomplete(crate::model::accounting::Incomplete),
    /// Completing the staged type admission after a conformance stage that
    /// found nothing refused, or reading it while pending, reached a type
    /// environment refusal or ceiling.
    TypeAdmission {
        /// The stable code of the type admission's refusal or ceiling.
        code: Code,
        /// Its catalog cause.
        cause: &'static str,
        /// The type-environment ceiling that was reached, when the cause is
        /// a reached limit; `None` for every other type admission refusal. Boxed to
        /// keep `CheckCause`, and so `CheckRefusal`, small.
        limit: Option<Box<qsl_foundation::diagnostic::LimitExceeded>>,
    },
    /// `refused { code: ill_typed }` with its FR-272 cause.
    IllTyped(IllTypedCause),
    /// `refused { code: wrong_snapshot }` with its FR-272 cause: `pre(...)`
    /// written outside its admitted clause, or over an ineligible operand.
    WrongSnapshot(WrongSnapshotCause),
    /// `missing_declaration` / `missing-name`.
    MissingName(String),
    /// `missing_declaration` / `missing-name` for `self` written where no
    /// state clause binds it (FR-104): a typed cause, not a `MissingName`
    /// whose text is `"self"`.
    MissingSelf,
    /// `ambiguous_declaration` / `ambiguous-name`, listing every locus of the
    /// name.
    AmbiguousName {
        /// The ambiguous name.
        name: String,
        /// Every declaring locus.
        loci: Vec<Location>,
    },
    /// FR-363: `r.name` names more than one candidate, a field and a
    /// relationship end or two ends: `ambiguous_declaration` /
    /// `ambiguous-name`, naming every candidate. Boxed like the other
    /// payload-bearing causes, to keep `CheckCause` small.
    AmbiguousMember(Box<AmbiguousMember>),
    /// FR-317: `q::m` names more than one candidate, a union, an enum or an
    /// import alias: `ambiguous_declaration` / `ambiguous-name`, naming every
    /// candidate.
    AmbiguousQualified(Box<AmbiguousQualified>),
    /// FR-363: `r.name` names a relationship end whose destination multiplicity
    /// types no navigation: `unsupported_construct` / `expression-form`.
    UnsupportedNavigation(NavigationBound),
    /// FR-366: the invariant of a record value type or an event type reads
    /// through a reference, navigates a relationship or ranges over a
    /// population. A record or event value is evaluated with no object
    /// universe, so there is nothing for the read to observe.
    ObjectReadInValueInvariant,
    /// `ill_typed` / `type-mismatch` (FR-091): an integer literal outside
    /// `i128::MIN..=i128::MAX`, at the literal. Never a nonzero-magnitude
    /// rewrite: a literal is refused, not truncated.
    IntegerOutsideI128(super::type_form::IntegerOutsideI128),
    /// `ill_typed` / `type-mismatch` (FR-091 "Native scalar literals"): the
    /// text of a `UUID("...")` literal is not the canonical lowercase
    /// `8-4-4-4-12` spelling, at the call.
    MalformedUuidLiteral {
        /// The argument text as written (decoded).
        argument: String,
    },
    /// `ill_typed` / `non-boolean-root` (FR-104): a state clause body whose
    /// type is not `Boolean`, at the body.
    NonBooleanRoot,
    /// `wrong_snapshot` / `wrong-anchor` (FR-104): `result` written anywhere
    /// but a postcondition of an operation that declares a result. Names the
    /// state clause kind and the operation it was written under: both are
    /// `None` outside a state clause, and the operation is `None` in an
    /// invariant.
    UnanchoredResult {
        /// The state clause kind, or `None` outside a state clause.
        clause: Option<qsl_forms::StateClauseKind>,
        /// The operation the clause names, or `None` when it names none.
        operation: Option<String>,
    },
    /// `undefined_expression` with the cause of its obligation.
    Unproved(Obligation),
    /// The `SumCase` family's one failed `case` exhaustiveness obligation
    /// (FR-318): `undefined_expression`/`unproved-exhaustiveness`. Boxed like
    /// [`Self::ProtocolAnchor`], to keep `CheckCause` small.
    SumCase(Box<super::SumCaseCause>),
    /// `undefined_expression` / `unproved-decrease` for one recursive
    /// component.
    UnprovedDecrease {
        /// The component cycle, first and last entries equal. The refusal
        /// location is the call edge the obligation is about.
        cycle: Vec<String>,
        /// The failed obligation.
        obligation: MeasureObligation,
    },
    /// `stage_limit_exceeded`/`<kind>-exceeded`: a declared
    /// checking limit was reached at this node. Boxed: `actual`'s `u128`
    /// would otherwise make every `CheckCause` pay for this one variant's
    /// widest field (clippy `result_large_err` on `CheckRefusal`), the same
    /// reason `InternalFault` below is boxed.
    ResourceExhausted(Box<StageLimitCause>),
    /// An IEEE conversion or arithmetic in a package with no admitted IEEE
    /// profile:
    /// `invalid_package`.
    IeeeProfileNotAdmitted,
    /// A derived collection bound exceeds the representable cardinality range.
    UnrepresentableBound,
    /// `invalid_package` / `definition-cycle` (FR-151, TC-196 D08): an
    /// FR-146 call-graph strongly connected component containing an FR-151
    /// dispatch edge. Operation bodies and contract clauses have no
    /// `decreases` form, so such a component is refused outright, never
    /// measure-checked.
    DefinitionCycle {
        /// Every dispatch edge in the component, ascending by caller then
        /// callee source-declaration order (FR-151's own listing order).
        edges: Vec<(String, String)>,
    },
    /// `invalid_package` / `invalid-value`: a supplied [`super::DispatchTable`]
    /// or [`super::DispatchOperation`] is malformed — an out-of-range table
    /// or function index, or a candidate whose signature does not match its
    /// dispatch operation's declared arity or types. Checked upfront in
    /// [`super::PackageDeclarations::check`], before any node is typed, so
    /// `facts`'s own call-graph walk can treat every table index it
    /// reads as already valid; reuses the already-catalogued `invalid-value`
    /// tag rather than minting a new one. Boxed: this was already
    /// `CheckCause`'s widest variant at 56 bytes; adding `ResourceExhausted`'s
    /// `StageLimitCause` payload cost `CheckCause` its niche-packed
    /// discriminant, so `CheckRefusal` crossed clippy's `result_large_err`
    /// threshold. Boxing this pre-existing variant (not `ResourceExhausted`,
    /// already boxed) is the fix, same precedent as `InternalFault` below.
    InvalidDispatchDeclaration(Box<InvalidDispatchDeclaration>),
    /// `missing_declaration` / `missing-selection` (FR-093): a lowered
    /// operation or leaf needs a profile law whose `DefinitionRef` the
    /// package's lock evidence does not supply. `check` writes no law from a
    /// constant (ADR-011 §2.4), so the node is not built.
    MissingSelection {
        /// The law role the lock evidence does not select.
        role: super::node_key::LawRole,
    },
    /// `unknown_required_feature` / `unsupported-feature` (FR-092 "Recursion
    /// groups", FR-092-OQ-1): the nodes of a recursion group cannot be keyed
    /// apart from another group's, so `check` refuses the group instead of
    /// keying it. `loci` names every in-group declaration, in declaration
    /// order.
    UnsupportedFeature {
        /// Every in-group declaration's region.
        loci: Vec<Location>,
    },
    /// `runtime_invariant` / `established-invariant-broken` (FR-094
    /// "Refusals", ADR-013 T-4): keying a node needed a fact that type
    /// admission, intake or the check stage's own unit scope guarantees, and
    /// it was not there. A fault in `check`, not in the input; the node gets
    /// no key. Boxed: a fault names a `DeclarationKey` or a `UnitId`, larger
    /// than every other cause.
    InternalFault(Box<KeyFault>),
    /// `invalid_package`: a node preimage that cannot be encoded (an empty
    /// or non-identifier name, a number RFC 8785 cannot render exactly).
    NodePreimage(super::node_key::NodeKeyRefusal),
    /// A formed compound unit's identity could not be encoded (FR-259
    /// Behavior 4): its bytes reached `identity.input_bytes`
    /// (`stage_limit_exceeded`), a heap reservation failed
    /// (`resource_exhausted`), or, a broken invariant, it has no RFC 8785
    /// encoding.
    Identity(IdentityRefusal),
    /// FR-113: a scoped anchor's resolution or a protocol's declaration
    /// collection refused. Boxed for the same reason as
    /// `InvalidDispatchDeclaration` above: this would otherwise be
    /// `CheckCause`'s widest variant.
    ProtocolAnchor(Box<ProtocolAnchorCause>),
    /// FR-218: a protocol reference expression refused after it resolved.
    /// Boxed for the same reason as `ProtocolAnchor` above.
    ProtocolReference(Box<ProtocolReferenceCause>),
    /// FR-326: the `TemporalTrace` family refused a `temporal` clause.
    /// Boxed for the same reason as `ProtocolAnchor` above.
    Temporal(Box<TemporalCause>),
    /// FR-165: the `StateGraph` family refused a state-graph claim. Boxed
    /// for the same reason as `ProtocolAnchor` above.
    StateGraph(Box<StateGraphCause>),
    /// FR-381, FR-218: S3 refused a protocol's role, activation, capture or
    /// compensation template. Boxed for the same reason as `ProtocolAnchor`
    /// above.
    ProtocolObligation(Box<super::protocol_obligation::ObligationCause>),
    /// FR-302: a called operation's dispatch family did not link at S3.
    /// Boxed for the same reason as `ProtocolAnchor` above.
    DispatchLink(Box<DispatchLinkCause>),
    /// FR-062-AC-8/TC-161/FR-063: exists only so `--cfg seam_probe` makes
    /// `CheckCause::code`'s match below non-exhaustive. Never constructed
    /// outside the probe build.
    #[cfg(seam_probe)]
    __SeamProbe,
}

/// FR-302: why one called operation's dispatch family did not link. An
/// ambiguous family has one value for each subtype that did not link, as the
/// QSpec FR-272 `ambiguous_dispatch` record names one closed subtype; any other
/// failure has one value for the family. A compile that fails several
/// families holds all of them in its one refusal.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DispatchLinkCause {
    /// The called operation: the root of the family that failed.
    pub operation: crate::model::key::DeclarationKey,
    /// The refusal's code.
    pub code: Code,
    /// The refusal's catalog pair: `resource_exhausted`/
    /// `insufficient-next-charge` for a ceiling, `ambiguous_dispatch`/
    /// `no-applicable` or `multiple-undominated` for an ambiguous family.
    pub catalog: qsl_foundation::diagnostic::CatalogCode,
    /// The caller-budget ceiling the family stopped on, when it did: the
    /// setting that raises it, the limit, the count the refused step would
    /// have reached and the retained charge (FR-255 Behavior 1).
    pub denial: Option<qsl_foundation::diagnostic::BudgetDenial>,
    /// The subtype that did not link, with its candidates and dominance
    /// pairs; `None` unless the family was ambiguous.
    pub subtype: Option<DispatchSubtypeFailure>,
}

/// One subtype that did not link in an ambiguous family (FR-083): its
/// FR-151 cause, the applicable candidates and the strict dominance pairs
/// among them.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DispatchSubtypeFailure {
    /// The subtype.
    pub subtype: crate::model::key::DeclarationKey,
    /// `no-applicable` or `multiple-undominated`.
    pub cause: &'static str,
    /// Every applicable candidate.
    pub candidates: Vec<crate::model::key::DeclarationKey>,
    /// Every `(dominant, dominated)` pair among them.
    pub dominance_pairs: Vec<(
        crate::model::key::DeclarationKey,
        crate::model::key::DeclarationKey,
    )>,
}

/// The invariant [`CheckCause::InternalFault`] names, with the value that
/// broke it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum KeyFault {
    /// The private intake owner lost its corresponding assembled model.
    MissingAdmittedModel(String),
    /// A staged-admission read the conformance stage made for an admitted
    /// member pair was refused, which the admitted records cannot cause.
    PendingField(String),
    /// A `Reference<T>`'s `EffectiveId` is no `type_identities` value of an
    /// admitted domain package's effective view.
    UnknownEffectiveId(EffectiveId),
    /// A `DeclarationKey`'s `package` is no admitted domain package's
    /// identity.
    UnadmittedPackage(crate::model::key::DeclarationKey),
    /// A `DeclarationKey`'s `node` is empty.
    EmptyNode(crate::model::key::DeclarationKey),
    /// A `DeclarationKey` names no record of its admitted domain package.
    UnknownDeclaration(crate::model::key::DeclarationKey),
    /// A domain package record of a kind no checked node names.
    UnnamedRecordKind(crate::model::key::DeclarationKey),
    /// A compound `UnitId` the check stage's unit scope does not hold.
    UnheldUnit(quire_exact::UnitId),
    /// A declared unit node, or a dimension or target unit node a held unit
    /// names, whose nominal preimage the unit table does not hold.
    UnheldNominal(quire_exact::NodeKey),
    /// A `Population<T>[N]`-typed node whose object type `T` no population
    /// binding names.
    UntargetedPopulation,
    /// A literal of a value kind the `Value` family's checker never builds
    /// as a literal.
    UnbuiltLiteral,
    /// A `ValueType::Composite` handle names no declaration of the check's
    /// type environment.
    UnknownComposite(quire_exact::NodeKey),
    /// A recursion group handed to the group keying is empty, repeats a
    /// member, or names no member at a group position.
    InvalidGroup,
    /// A node names a draft that dependency order never keyed.
    UnresolvedDraft,
    /// A record value's slots and its declaration's fields differ in
    /// number.
    RecordSlotCount,
    /// An IEEE arithmetic node whose left operand is not an IEEE type.
    UntypedIeeeOperand,
    /// Two admitted domain packages share this identity: a check selects
    /// one version of each (FR-094, ADR-013 O-01).
    DuplicateModelSelection(String),
    /// An admitted enum declaration or member whose nominal preimage has no
    /// RFC 8785 encoding.
    NonCanonicalNominal(quire_exact::NodeKey),
    /// A family `check` reported a stage limit whose setting no checking
    /// limit has.
    UncheckedLimit(Setting),
    /// A claim (ADR-012 §13.5) could not be keyed by occurrence: its site
    /// has no one scalar application `expression` occurrence at its
    /// location, a guard, root or result bound names no lowered node, or a
    /// scalar application occurrence in a function body is no claim's.
    /// Claims are never silently dropped.
    UnkeyableRequirements,
    /// A claim's extent could not be classified: a checked type names a
    /// composite the type environment does not hold, or the body reads a
    /// slot no binder bound (FR-097-AC-2).
    UnclassifiedExtent(qsl_foundation::InternalFault),
    /// A family `check` stopped on an internal fault of its own, which names
    /// the stage and invariant.
    StageFault(qsl_foundation::InternalFault),
    /// A model correspondence entry would give a node a second declaration
    /// or a declaration a second node (FR-303): names the recorded pair and
    /// the offered one.
    CorrespondenceConflict(Box<super::identity::CorrespondenceConflict>),
}

impl KeyFault {
    /// The violated invariant's stable identifier (ADR-013 T-4).
    pub fn invariant(&self) -> &'static str {
        match self {
            Self::MissingAdmittedModel(_) => "intake-owner-has-assembled-model",
            Self::PendingField(_) => "conformance-reads-admitted-member-pair",
            Self::UnknownEffectiveId(_) => "reference-target-admitted",
            Self::UnadmittedPackage(_) => "declaration-package-admitted",
            Self::EmptyNode(_) => "declaration-node-nonempty",
            Self::UnknownDeclaration(_) => "declaration-record-present",
            Self::UnnamedRecordKind(_) => "record-kind-named",
            Self::UnheldUnit(_) => "compound-unit-held",
            Self::UnheldNominal(_) => "unit-nominal-preimage-held",
            Self::UntargetedPopulation => "population-target-bound",
            Self::UnbuiltLiteral => "literal-kind-built",
            Self::UnknownComposite(_) => "composite-declared",
            Self::InvalidGroup => "recursion-group-well-formed",
            Self::UnresolvedDraft => "draft-keyed-in-dependency-order",
            Self::RecordSlotCount => "record-slots-match-fields",
            Self::UntypedIeeeOperand => "ieee-operand-typed",
            Self::DuplicateModelSelection(_) => "one-model-version-per-identity",
            Self::NonCanonicalNominal(_) => "nominal-preimage-canonical",
            Self::UncheckedLimit(_) => "family-limit-is-a-checking-limit",
            Self::UnkeyableRequirements => "requirements-item-keyed",
            Self::UnclassifiedExtent(fault) | Self::StageFault(fault) => fault.invariant(),
            Self::CorrespondenceConflict(_) => "model-correspondence-one-to-one",
        }
    }
}

/// Which dispatch-table function slot [`InvalidDispatchDeclaration`] names.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DispatchFunctionRole {
    /// A candidate's own operation body.
    Body,
    /// A candidate's own runtime-evaluated effective precondition.
    Precondition,
    /// A precondition clause reachable from a candidate's static FR-146
    /// redefinition ancestry.
    PreconditionClause,
}

/// The typed detail of [`CheckCause::InvalidDispatchDeclaration`]: what
/// exactly is malformed about a supplied [`super::DispatchTable`] or
/// [`super::DispatchOperation`], with the actual field/index path
/// (`native-diagnostics.md`'s `invalid_package`/`invalid-value` row) rather
/// than a free-form summary.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum InvalidDispatchDeclaration {
    /// A `NodeKind::Dispatch` node's own `operation` index names no
    /// operation in the package's own `dispatch_operations`.
    OperationOutOfRange {
        /// The out-of-range operation index.
        operation: usize,
    },
    /// A dispatch operation names a `table` index past the package's own
    /// `dispatch_tables`.
    TableOutOfRange {
        /// The dispatch operation's own member name.
        member: String,
        /// The out-of-range table index.
        table: usize,
    },
    /// A dispatch-table function index names no function in the package's
    /// own `functions`.
    FunctionOutOfRange {
        /// Which slot this index was read from.
        role: DispatchFunctionRole,
        /// The out-of-range function index.
        index: usize,
    },
    /// A candidate function's declared parameter count does not match the
    /// dispatch operation's own arity (the receiver plus every declared
    /// argument).
    Arity {
        /// Which slot this index was read from.
        role: DispatchFunctionRole,
        /// The function index, valid in `functions` but wrongly shaped.
        index: usize,
        /// The function's own declared parameter count.
        declared: usize,
        /// The receiver plus the dispatch operation's own argument count.
        expected: usize,
    },
    /// A candidate function's parameter types (after the receiver) do not
    /// match the dispatch operation's declared argument types.
    ParameterType {
        /// Which slot this index was read from.
        role: DispatchFunctionRole,
        /// The function index, valid in `functions` but wrongly shaped.
        index: usize,
    },
    /// A candidate function's declared result type does not match the
    /// result its role requires (the operation's own result for a body,
    /// `Boolean` for a precondition or precondition clause).
    ResultType {
        /// Which slot this index was read from.
        role: DispatchFunctionRole,
        /// The function index, valid in `functions` but wrongly shaped.
        index: usize,
    },
    /// Two dispatch operations in the package's own `dispatch_operations`
    /// share a `(receiver_type, member)` pair. A well-formed package exposes
    /// each static receiver type/member name once; a duplicate means two
    /// bridge calls (see `crate::check::checked_dispatch`) were merged for
    /// overlapping roots, and name lookup at a call site would otherwise
    /// silently pick whichever entry happens to come first.
    DuplicateOperation {
        /// The receiver type both operations declare, by its effective
        /// identity.
        receiver_type: EffectiveId,
        /// The shared member name.
        member: String,
    },
    /// A [`super::ResolvedSignatures`] entry is keyed by an index past the
    /// package's own `functions`, so it stands in for no declaration.
    ResolvedSignatureOutOfRange {
        /// The out-of-range function index.
        index: usize,
    },
    /// A `PackageDeclarations::model_clauses` entry is keyed by an index
    /// past the package's own `functions`, so it owns no clause function.
    ModelClauseOutOfRange {
        /// The out-of-range function index.
        index: usize,
    },
}

impl CheckCause {
    /// This cause without the type-level invariant it was found in (FR-366):
    /// a reader that classifies a cause by its kind (a reached limit, an
    /// internal fault, a denied dispatch link) reads through the wrapper.
    #[must_use]
    pub fn unwrapped(&self) -> &Self {
        match self {
            Self::TypeInvariant(refusal) => refusal.cause.unwrapped(),
            Self::Terminal(refusal) => refusal.cause.unwrapped(),
            Self::Refinement(refusal) => match &refusal.cause {
                super::refinement::RefinementCause::Check(cause) => cause.unwrapped(),
                _ => self,
            },
            other => other,
        }
    }

    /// The attributed catalog pair for model causes, preserving raw code().
    pub fn catalog_code(&self) -> Option<qsl_foundation::diagnostic::CatalogCode> {
        match self {
            Self::StateModel(cause) => cause.catalog_code(),
            Self::TypeInvariant(cause) => cause.cause.catalog_code(),
            Self::Terminal(cause) => cause.cause.catalog_code(),
            Self::Refinement(cause) => match &cause.cause {
                super::refinement::RefinementCause::Check(inner) => inner.catalog_code(),
                other => Some(qsl_foundation::diagnostic::CatalogCode::new(
                    other.code().as_str(),
                    other.cause()?,
                )),
            },
            Self::DispatchLink(cause) => Some(cause.catalog),
            _ => Some(qsl_foundation::diagnostic::CatalogCode::new(
                self.code().as_str(),
                self.cause()?,
            )),
        }
    }

    fn rendered_code(&self) -> String {
        self.catalog_code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| self.code().as_str().to_owned())
    }
    /// The refusal code.
    ///
    /// FR-062-AC-8/TC-161/FR-063 seam: adding a `CheckCause` variant with no arm
    /// here fails `--cfg seam_probe` with `E0004` (`xtask::seam_probe`'s
    /// checked-in `CheckCause::code` location) -- this is the one family
    /// `Cause` enum's `catalog_code()`-shaped mapping (ADR-012 §5.1 S4) QSL
    /// has today.
    #[deny(clippy::wildcard_enum_match_arm)]
    #[deny(clippy::match_wildcard_for_single_variants)]
    pub fn code(&self) -> Code {
        match self {
            Self::ProfileProhibitedOperator(_) => Code::UnsupportedConstruct,
            Self::StateModel(cause) => cause.cause.code(),
            Self::TypeInvariant(cause) => cause.cause.code(),
            Self::Terminal(cause) => cause.cause.code(),
            Self::TerminalNotEmitted { .. } | Self::RefinementNotEmitted { .. } => {
                Code::UnsupportedConstruct
            }
            Self::Refinement(cause) => cause.cause.code(),
            Self::ModelConformance(failure) => failure.code,
            Self::ModelIncomplete(_) => Code::ResourceExhausted,
            Self::TypeAdmission { code, .. } => *code,
            Self::IllTyped(_)
            | Self::NonBooleanRoot
            | Self::IntegerOutsideI128(_)
            | Self::MalformedUuidLiteral { .. } => Code::IllTyped,
            Self::WrongSnapshot(_) | Self::UnanchoredResult { .. } => Code::WrongSnapshot,
            Self::MissingName(_) | Self::MissingSelf => Code::MissingDeclaration,
            Self::AmbiguousName { .. } | Self::AmbiguousMember(_) | Self::AmbiguousQualified(_) => {
                Code::AmbiguousDeclaration
            }
            Self::UnsupportedNavigation(_) | Self::ObjectReadInValueInvariant => {
                Code::UnsupportedConstruct
            }
            Self::Unproved(_) | Self::UnprovedDecrease { .. } => Code::UndefinedExpression,
            Self::SumCase(cause) => cause.code(),
            Self::ResourceExhausted(_) => Code::StageLimitExceeded,
            Self::IeeeProfileNotAdmitted
            | Self::DefinitionCycle { .. }
            | Self::InvalidDispatchDeclaration(_)
            | Self::NodePreimage(_) => Code::InvalidPackage,
            Self::UnrepresentableBound => Code::UnrepresentableConstraint,
            Self::MissingSelection { .. } => Code::MissingDeclaration,
            Self::UnsupportedFeature { .. } => Code::UnknownRequiredFeature,
            Self::InternalFault(_) => Code::RuntimeInvariant,
            Self::Identity(IdentityRefusal::InputBytes { .. }) => Code::StageLimitExceeded,
            Self::Identity(IdentityRefusal::Allocation { .. }) => Code::ResourceExhausted,
            Self::Identity(IdentityRefusal::NonCanonical) => Code::RuntimeInvariant,
            Self::ProtocolAnchor(cause) => cause.code(),
            Self::ProtocolReference(cause) => cause.code(),
            Self::Temporal(cause) => cause.code(),
            Self::StateGraph(cause) => cause.code(),
            Self::ProtocolObligation(cause) => cause.code(),
            Self::DispatchLink(cause) => cause.code,
            // Downstream crates (`qsl-eval`, `qsl-route`, the root crate)
            // have their own real seams over other enums, probed in the
            // same `--cfg seam_probe_downstream` build -- this arm exists
            // only so *this* match keeps compiling in that build, letting
            // rustc reach and report those crates' own E0004s, the same
            // "arm under downstream, none under seam_probe alone" shape
            // `FamilyKind::catalog_code_prefix` already uses. `Code`'s
            // choice is arbitrary: `__SeamProbe` is never constructed
            // outside the probe build, so this arm is never actually
            // reached.
            #[cfg(seam_probe_downstream)]
            Self::__SeamProbe => Code::RuntimeInvariant,
        }
    }

    /// The closed cause tag, where the code has one.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::StateModel(cause) => cause.cause.cause(),
            Self::TypeInvariant(cause) => cause.cause.cause(),
            Self::Terminal(cause) => cause.cause.cause(),
            Self::TerminalNotEmitted { .. } | Self::RefinementNotEmitted { .. } => {
                Some("not-yet-implemented")
            }
            Self::Refinement(cause) => cause.cause.cause(),
            Self::ModelConformance(failure) => Some(failure.cause.as_str()),
            // The model ceilings are the caller's budget (FR-255), so a denied
            // charge is an insufficient next charge, never a stage limit.
            Self::ModelIncomplete(_) => Some("insufficient-next-charge"),
            Self::TypeAdmission { cause, .. } => Some(cause),
            Self::IllTyped(cause) => cause.tag(),
            Self::WrongSnapshot(cause) => Some(cause.as_str()),
            Self::NonBooleanRoot => Some("non-boolean-root"),
            Self::IntegerOutsideI128(_) | Self::MalformedUuidLiteral { .. } => {
                Some("type-mismatch")
            }
            Self::UnanchoredResult { .. } => Some("wrong-anchor"),
            Self::MissingName(_) | Self::MissingSelf => Some("missing-name"),
            Self::AmbiguousName { .. } | Self::AmbiguousMember(_) | Self::AmbiguousQualified(_) => {
                Some("ambiguous-name")
            }
            Self::UnsupportedNavigation(_) | Self::ObjectReadInValueInvariant => {
                Some("expression-form")
            }
            Self::Unproved(Obligation::Nonzero) => Some("unproved-nonzero"),
            Self::Unproved(Obligation::Presence) => Some("unproved-presence"),
            Self::Unproved(
                Obligation::Range { .. }
                | Obligation::RationalRange
                | Obligation::NonemptyReduction { .. },
            ) => Some("unproved-range"),
            Self::UnprovedDecrease { .. } => Some("unproved-decrease"),
            Self::SumCase(cause) => Some(cause.tag()),
            Self::ResourceExhausted(cause) => Some(cause.kind.setting().kind().catalog_cause()),
            Self::DefinitionCycle { .. } => Some("definition-cycle"),
            Self::InvalidDispatchDeclaration(_) => Some("invalid-value"),
            Self::MissingSelection { .. } => Some("missing-selection"),
            Self::UnsupportedFeature { .. } => Some("unsupported-feature"),
            Self::ProfileProhibitedOperator(_) => Some("expression-form"),
            Self::InternalFault(_) | Self::Identity(IdentityRefusal::NonCanonical) => {
                Some("established-invariant-broken")
            }
            Self::Identity(IdentityRefusal::InputBytes { .. }) => {
                Some(Setting::IdentityInputBytes.kind().catalog_cause())
            }
            Self::Identity(IdentityRefusal::Allocation { .. }) => Some("allocation-failed"),
            Self::IeeeProfileNotAdmitted | Self::UnrepresentableBound | Self::NodePreimage(_) => {
                None
            }
            Self::ProtocolAnchor(cause) => Some(cause.tag()),
            Self::ProtocolReference(cause) => Some(cause.tag()),
            Self::Temporal(cause) => Some(cause.tag()),
            Self::StateGraph(cause) => cause.tag(),
            Self::ProtocolObligation(cause) => cause.tag(),
            Self::DispatchLink(cause) => Some(cause.catalog.cause()),
            // Not a seam-probe location (unlike `code()`, above --
            // deliberately left non-exhaustive there): this arm keeps
            // `cause()` compiling under `--cfg seam_probe`, so `code()`'s
            // own E0004 is the one and only probe failure this variant
            // causes, matching the checked-in list in `xtask::seam_probe`.
            #[cfg(seam_probe)]
            Self::__SeamProbe => None,
        }
    }
}

/// A located checking refusal, made before any charge.
///
/// quire:canonical
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("{} at {location:?}: {cause:?}", cause.rendered_code())]
pub struct CheckRefusal {
    /// Where the refusal originates.
    pub location: Location,
    /// The typed cause.
    pub cause: CheckCause,
}

impl CheckRefusal {
    pub(crate) fn ill_typed(location: &Location, cause: IllTypedCause) -> Self {
        Self {
            location: location.clone(),
            cause: CheckCause::IllTyped(cause),
        }
    }

    /// A kernel type refusal as its FR-272 cause: an unordered enum ordering
    /// is `operator-ineligible`, every other incompatibility `type-mismatch`.
    pub(crate) fn from_ill_typed(location: &Location, refusal: IllTyped) -> Self {
        let cause = match refusal.cause {
            IllTypedCause::TypeMismatch
            | IllTypedCause::OperatorIneligible
            | IllTypedCause::AmbiguousLiteral => refusal.cause,
            IllTypedCause::UnorderedEnumOrdering => IllTypedCause::OperatorIneligible,
            IllTypedCause::DistinctTextProfiles
            | IllTypedCause::DistinctEnumDeclarations
            | IllTypedCause::IncompatibleDimensions
            | IllTypedCause::AffineUnitArithmetic
            | IllTypedCause::DistinctUnits
            | IllTypedCause::MalformedDecimalType
            | IllTypedCause::DistinctIeeeWidths
            | IllTypedCause::IeeeWithExactOperand
            | IllTypedCause::IeeeToNonRationalExact => IllTypedCause::TypeMismatch,
        };
        Self::ill_typed(location, cause)
    }
}

#[cfg(test)]
mod tests {
    use super::{CheckCause, CheckingLimitKind, CheckingStage, KeyFault, Setting, StageLimitCause};
    use qsl_foundation::diagnostic::Code;

    /// Every `CheckingLimitKind` reports `stage_limit_exceeded`
    /// with its own `<kind>-exceeded` cause, and carries the bound and
    /// actual counter it was built with.
    #[test]
    fn resource_exhausted_reports_stage_limit_exceeded_per_kind() {
        let cases = [
            (CheckingLimitKind::Nodes, "node-count-exceeded"),
            (CheckingLimitKind::InputBytes, "input-bytes-exceeded"),
            (CheckingLimitKind::WorkBudget, "work-budget-exceeded"),
        ];
        for (kind, cause) in cases {
            let refused = CheckCause::ResourceExhausted(Box::new(StageLimitCause {
                stage: CheckingStage::Typing,
                kind,
                limit: 10,
                actual: 11,
                region: None,
            }));
            assert_eq!(refused.code(), Code::StageLimitExceeded, "{kind:?}");
            assert_eq!(refused.cause(), Some(cause), "{kind:?}");
            let CheckCause::ResourceExhausted(exceeded) = refused else {
                unreachable!()
            };
            assert_eq!(exceeded.limit, 10);
            assert_eq!(exceeded.actual, 11);
        }
    }

    /// The three settings a checking limit names convert back; every other
    /// setting names a limit of another stage, refuses, and names the
    /// `KeyFault` `check::mod` raises for it.
    #[test]
    fn only_checking_settings_convert_from_a_setting() {
        for setting in Setting::ALL.iter().copied() {
            let expected = match setting {
                Setting::S3Nodes => Ok(CheckingLimitKind::Nodes),
                Setting::S3InputBytes => Ok(CheckingLimitKind::InputBytes),
                Setting::S3WorkUnits => Ok(CheckingLimitKind::WorkBudget),
                other => Err(other),
            };
            assert_eq!(CheckingLimitKind::try_from(setting), expected, "{setting}");
        }
        assert_eq!(
            KeyFault::UncheckedLimit(Setting::S1Tokens).invariant(),
            "family-limit-is-a-checking-limit"
        );
    }
}
