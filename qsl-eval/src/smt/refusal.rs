// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 Outputs: `SmtEncodingRefusal`, the refusal that names the construct
//! the encoding has no form for, with its locus. Each variant is one code of
//! FR-315's refusal table, and [`SmtEncodingRefusal::code`] spells it.

use qsl_foundation::InternalFault;
use qsl_semantics::model::observation::AdmissionFailure;

use crate::automaton::AutomatonError;
use crate::simulation::NotSimulated;

/// Where in a checked clause body a refused expression is: the clause and the
/// node's index in the clause's checked body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpressionLocus {
    /// The clause's declared name.
    pub clause: String,
    /// The node's index in the clause's body.
    pub node: usize,
}

impl std::fmt::Display for ExpressionLocus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "clause `{}` node {}", self.clause, self.node)
    }
}

/// What an `object-lifecycle` refusal is about.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LifecycleLocus {
    /// An operation whose frame creates or deletes objects.
    Operation(String),
    /// A universe object that is not held at one type by every initial state.
    Object(String),
}

/// Why no script was written (FR-315 Outputs).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SmtEncodingRefusal {
    /// `item-form`: the item has no proof query of this form.
    #[error("the item is {form}, which FR-315 gives no proof query")]
    ItemForm {
        /// The form.
        form: &'static str,
    },
    /// `item-form`: the clause's property-automaton translation refused.
    #[error("the property automaton does not build: {0}")]
    ItemTranslation(#[from] AutomatonError),
    /// `profile`: the clause binds a profile with no proof query.
    #[error("the clause binds a profile with no proof query")]
    Profile,
    /// `query-kind`: the item has no query of this kind.
    #[error("a {form} item has no `{kind}` query")]
    QueryKind {
        /// The query kind's name.
        kind: &'static str,
        /// The item's form.
        form: &'static str,
    },
    /// `subject-admission`: FR-120's admission of the subject refused.
    #[error("the subject does not admit: {0}")]
    SubjectAdmission(AdmissionFailure),
    /// `no-initial-state`: the subject has no initial state.
    #[error("the subject has no initial state")]
    NoInitialState,
    /// `object-lifecycle`: the objects of a state copy are not fixed.
    #[error("the objects of a state copy are not fixed at {0:?}")]
    ObjectLifecycle(LifecycleLocus),
    /// `constant-name`: a name cannot be printed, or two constants would
    /// print the same name.
    #[error("the name `{name}` cannot be printed as one distinct constant")]
    ConstantName {
        /// The name.
        name: String,
    },
    /// `field-sort`: a field has a sort with no SMT form (FR-315-AC-3).
    #[error("field `{object}.{field}` has {sort}, which has no SMT sort")]
    FieldSort {
        /// The object's key.
        object: String,
        /// The field's declared name.
        field: String,
        /// The field's sort, as the refusal names it.
        sort: &'static str,
    },
    /// `parameter-domain`: an operation parameter has no finite domain.
    #[error("parameter `{parameter}` of operation `{operation}` has no finite domain")]
    ParameterDomain {
        /// The operation's name.
        operation: String,
        /// The parameter's name.
        parameter: String,
    },
    /// `requires-bound`: a population that admits the `over` type has no
    /// universe.
    #[error("the clause's instances are not finite: {0:?}")]
    RequiresBound(NotSimulated),
    /// `no-instance`: the clause has zero instances.
    #[error("the item has no instance")]
    NoInstance,
    /// `instance-binding`: an instance's object does not conform to the
    /// `over` type.
    #[error("the instance `{key}` binds no object of the `over` type")]
    InstanceBinding {
        /// The instance's key.
        key: String,
    },
    /// `several-instances`: a TP-3 item has more than one instance.
    #[error("the monitored item has {count} instances")]
    SeveralInstances {
        /// The number of instances.
        count: usize,
    },
    /// `too-large`: a size measure passes its limit.
    #[error("{measure} is {value}, over the limit {limit}")]
    TooLarge {
        /// The measure.
        measure: &'static str,
        /// Its value.
        value: u64,
        /// Its limit.
        limit: u64,
    },
    /// `out-of-fragment`: an expression is outside the `QF_LIA` fragment.
    #[error("the {form} at {locus} has no QF_LIA form")]
    OutOfFragment {
        /// The construct.
        form: &'static str,
        /// Where it is.
        locus: ExpressionLocus,
    },
    /// An invariant of the subject broke; not a code of the table.
    #[error("encoding broke invariant {0:?}")]
    Fault(InternalFault),
}

impl SmtEncodingRefusal {
    /// The refusal's code in FR-315's table; `internal-fault` for a broken
    /// invariant.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ItemForm { .. } | Self::ItemTranslation(_) => "item-form",
            Self::Profile => "profile",
            Self::QueryKind { .. } => "query-kind",
            Self::SubjectAdmission(_) => "subject-admission",
            Self::NoInitialState => "no-initial-state",
            Self::ObjectLifecycle(_) => "object-lifecycle",
            Self::ConstantName { .. } => "constant-name",
            Self::FieldSort { .. } => "field-sort",
            Self::ParameterDomain { .. } => "parameter-domain",
            Self::RequiresBound(_) => "requires-bound",
            Self::NoInstance => "no-instance",
            Self::InstanceBinding { .. } => "instance-binding",
            Self::SeveralInstances { .. } => "several-instances",
            Self::TooLarge { .. } => "too-large",
            Self::OutOfFragment { .. } => "out-of-fragment",
            Self::Fault(_) => "internal-fault",
        }
    }
}
