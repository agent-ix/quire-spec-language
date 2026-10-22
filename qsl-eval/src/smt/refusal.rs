// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 Outputs: `SmtEncodingRefusal`, the refusal that names the construct
//! the encoding has no form for, with its locus.

use qsl_foundation::InternalFault;

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

/// Why no script was written (FR-315 Outputs). Each variant is one condition
/// a caller distinguishes; none is told apart by its message.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SmtEncodingRefusal {
    /// A field of a universe object has a sort the encoding has no form for
    /// (FR-315-AC-3).
    #[error("field `{object}.{field}` has {sort}, which has no SMT sort")]
    FieldSort {
        /// The object's key.
        object: String,
        /// The field's declared name.
        field: String,
        /// The field's sort, as the refusal names it.
        sort: &'static str,
    },
    /// An expression of a clause or atom is outside the `QF_LIA` fragment
    /// the encoding prints.
    #[error("the {form} at {locus} has no QF_LIA form")]
    Expression {
        /// The construct.
        form: &'static str,
        /// Where it is.
        locus: ExpressionLocus,
    },
    /// An operation parameter has a type with no finite value domain here.
    #[error("parameter `{parameter}` of operation `{operation}` has no finite domain")]
    OperationParameter {
        /// The operation's name.
        operation: String,
        /// The parameter's name.
        parameter: String,
    },
    /// An operation creates or deletes objects, so the universe objects of a
    /// state copy are not fixed.
    #[error("operation `{operation}` creates or deletes objects")]
    OperationLifecycle {
        /// The operation's name.
        operation: String,
    },
    /// A universe object is not held by every initial state, or is held at a
    /// different type.
    #[error("object `{object}` is not held at one type by every initial state")]
    ObjectLifecycle {
        /// The object's key.
        object: String,
    },
    /// Two populations hold an object under one key, so a constant name
    /// `s<i>.<object>.<field>` would name both.
    #[error("two populations hold an object under the key `{object}`")]
    ObjectNameClash {
        /// The key.
        object: String,
    },
    /// A name cannot be printed as an SMT-LIB symbol.
    #[error("the name `{name}` cannot be printed as an SMT-LIB symbol")]
    UnprintableName {
        /// The name.
        name: String,
    },
    /// The subject has no initial state, so it has no object to declare.
    #[error("the subject has no initial state")]
    NoInitialState,
    /// The item has no instance (FR-125 `NoInstance`).
    #[error("the item has no instance")]
    NoInstance,
    /// An instance's object is absent from the universe or has a type the
    /// clause's `over` parameter does not admit.
    #[error("the instance `{key}` binds no object of the `over` type")]
    InstanceBinding {
        /// The instance's key.
        key: String,
    },
    /// The item is of a form the proof queries do not cover.
    #[error("the item is {form}, which FR-315 gives no proof query")]
    ItemForm {
        /// The form.
        form: &'static str,
    },
    /// The item is not encoded for this kind of query.
    #[error("a {form} item has no `{kind}` query")]
    QueryKind {
        /// The query kind's name.
        kind: &'static str,
        /// The item's form.
        form: &'static str,
    },
    /// The item's monitor is larger than the encoding prints.
    #[error("the item's monitor has {atoms} atoms and {states} states; the encoding prints at most {max_atoms} and {max_states}")]
    MonitorBound {
        /// The letter's atoms.
        atoms: usize,
        /// The monitor states reached so far.
        states: usize,
        /// The most atoms printed.
        max_atoms: usize,
        /// The most states printed.
        max_states: usize,
    },
    /// An item of a form that reads one instance at a time has several
    /// instances (a monitor is built for one instance).
    #[error("the {form} item has {count} instances")]
    SeveralInstances {
        /// The item's form.
        form: &'static str,
        /// The number of instances.
        count: usize,
    },
    /// The clause's instances are not finite (FR-125 `RequiresBound`).
    #[error("the clause's instances are not finite: {0:?}")]
    Instances(crate::simulation::NotSimulated),
    /// The query would declare more constants than the encoding writes.
    #[error("depth {depth} declares more than {max} constants")]
    TooLarge {
        /// The requested depth.
        depth: u64,
        /// The most constants a script declares.
        max: usize,
    },
    /// The subject did not admit (FR-120 `ModelSystem::new`).
    #[error("the subject does not admit: {0}")]
    Subject(#[from] qsl_semantics::model::observation::AdmissionFailure),
    /// The property automaton did not build.
    #[error("the property automaton does not build: {0}")]
    Automaton(#[from] crate::automaton::AutomatonError),
    /// An invariant of the subject broke.
    #[error("encoding broke invariant {0:?}")]
    Fault(InternalFault),
}
