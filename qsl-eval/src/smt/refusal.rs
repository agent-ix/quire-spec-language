// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 Outputs: `SmtEncodingRefusal`, the refusal that names the construct
//! the encoding has no form for, with its locus. Each variant is one code of
//! FR-315's refusal table, and [`SmtEncodingRefusal::code`] spells it.

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

/// A size measure of FR-315 "Size limits", in the order they are checked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Measure {
    /// `declare-const` commands.
    Constants,
    /// Applications of one step.
    Applications,
    /// Atoms of a TP-3 item's monitor letter.
    MonitorAtoms,
    /// Monitor states the tabulation reaches.
    MonitorStates,
    /// Candidates of one application the query expands.
    Candidates,
    /// Bytes of the canonical script.
    ScriptBytes,
}

impl Measure {
    /// The measure's name in FR-315.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Constants => "constants",
            Self::Applications => "applications",
            Self::MonitorAtoms => "monitor-atoms",
            Self::MonitorStates => "monitor-states",
            Self::Candidates => "candidates",
            Self::ScriptBytes => "script-bytes",
        }
    }
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
    /// `field-domain`: the query expands the candidates of an application
    /// and a field the application may write has no finite domain.
    #[error("field `{object}.{field}` has no finite domain, and an application may write it")]
    FieldDomain {
        /// The object's key.
        object: String,
        /// The field's name.
        field: String,
    },
    /// `requires-bound`: a population that admits the `over` type has no
    /// universe.
    #[error("the clause's instances are not finite: {0:?}")]
    RequiresBound(NotSimulated),
    /// `no-instance`: the clause has zero instances.
    #[error("the item has no instance")]
    NoInstance,
    /// `instance-type`: an instance's object does not conform to the `over`
    /// type in an initial state.
    #[error("the instance `{key}` is no object of the `over` type")]
    InstanceType {
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
    #[error("{} is {value}, over the limit {limit}", measure.name())]
    TooLarge {
        /// The measure.
        measure: Measure,
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
}

impl SmtEncodingRefusal {
    /// The refusal's code in FR-315's table.
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
            Self::InstanceType { .. } => "instance-type",
            Self::FieldDomain { .. } => "field-domain",
            Self::SeveralInstances { .. } => "several-instances",
            Self::TooLarge { .. } => "too-large",
            Self::OutOfFragment { .. } => "out-of-fragment",
        }
    }

    /// The row of FR-315's refusal table this refusal is on, 1 to 17: the
    /// order the encoding applies the checks in.
    pub const fn row(&self) -> u8 {
        match self {
            Self::ItemForm { .. } | Self::ItemTranslation(_) => 1,
            Self::Profile => 2,
            Self::QueryKind { .. } => 3,
            Self::SubjectAdmission(_) => 4,
            Self::NoInitialState => 5,
            Self::ObjectLifecycle(_) => 6,
            Self::ConstantName { .. } => 7,
            Self::FieldSort { .. } => 8,
            Self::ParameterDomain { .. } => 9,
            Self::FieldDomain { .. } => 10,
            Self::RequiresBound(_) => 11,
            Self::NoInstance => 12,
            Self::InstanceType { .. } => 13,
            Self::SeveralInstances { .. } => 14,
            Self::TooLarge {
                measure: Measure::ScriptBytes,
                ..
            } => 17,
            Self::TooLarge { .. } => 15,
            Self::OutOfFragment { .. } => 16,
        }
    }

    /// An invariant the encoding relies on did not hold for this item: the
    /// item is refused as one the encoding cannot read.
    pub const fn unreadable(invariant: &'static str) -> Self {
        Self::ItemForm { form: invariant }
    }
}
