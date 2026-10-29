// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 Outputs: `SmtEncodingRefusal`, the two arms of an encoding that
//! wrote no script: [`Refusal`], which names the construct the encoding has
//! no form for with its locus (each variant is one code of FR-315's table,
//! and [`Refusal::code`] spells it), and an internal fault (FR-096).

use qsl_semantics::model::observation::AdmissionFailure;
use quire_semantic_value::catalog::InternalFault;

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
    /// The trace variable of the component the expression reads, for an atom
    /// of a hyper clause.
    pub component: Option<String>,
}

impl std::fmt::Display for ExpressionLocus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "clause `{}` node {}", self.clause, self.node)?;
        match &self.component {
            Some(component) => write!(formatter, " in component `{component}`"),
            None => Ok(()),
        }
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

    /// The request setting which raises this measure (FR-255).
    pub const fn setting(self) -> &'static str {
        match self {
            Self::Constants => "smt_encoding.max_constants",
            Self::Applications => "smt_encoding.max_applications",
            Self::MonitorAtoms => "smt_encoding.max_monitor_atoms",
            Self::MonitorStates => "smt_encoding.max_monitor_states",
            Self::Candidates => "smt_encoding.max_candidates",
            Self::ScriptBytes => "smt_encoding.max_script_bytes",
        }
    }
}

/// A refusal: the construct the encoding has no form for, or the size limit
/// passed (FR-315 Outputs).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Refusal {
    /// `item-form`: the item has no proof query of this form.
    #[error("the item is {form}, which FR-315 gives no proof query")]
    ItemForm {
        /// The form.
        form: &'static str,
    },
    /// `item-form`: the clause's property-automaton translation refused.
    #[error("the property automaton does not build: {0}")]
    ItemTranslation(#[from] AutomatonError),
    /// `invalid-value`: the race-freedom item (FR-224) names a model subject,
    /// which has no memory model and so no data race; EN-1 refuses the same
    /// request `invalid_runtime_input`/`invalid-value`.
    #[error("the race-freedom item is decided over a protocol subject only")]
    RaceOverModel,
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
    #[error(
        "{} is {value}, over the limit {limit} set by {setting}",
        measure.name()
    )]
    TooLarge {
        /// The measure.
        measure: Measure,
        /// Its value.
        value: u64,
        /// Its limit.
        limit: u64,
        /// The request setting which raises this limit.
        setting: &'static str,
    },
    /// `out-of-fragment`: an expression is outside the `QF_LIA` fragment.
    #[error("the {form} at {locus} has no QF_LIA form")]
    OutOfFragment {
        /// The construct.
        form: &'static str,
        /// Where it is.
        locus: ExpressionLocus,
    },
    /// `refinement-any-row`: a refinement step row maps to `any`, so it has
    /// no per-step query (FR-146).
    #[error("refinement step row {row} maps to `any`, which has no per-step query")]
    RefinementAnyRow {
        /// The row's zero-based position in the refinement's `steps`.
        row: u64,
    },
    /// `refinement-open-argument`: an argument of a refinement step row is
    /// `_`, which the negated rule would quantify universally.
    #[error("argument {argument} of refinement step row {row} is `_`")]
    RefinementOpenArgument {
        /// The row's zero-based position in the refinement's `steps`.
        row: u64,
        /// The argument's zero-based position among the abstract
        /// parameters.
        argument: u64,
    },
    /// `hyper-form`: FR-173 classifies the clause as other than HP-2.
    #[error("hyper clause `{clause}` is {form}, not an HP-2 clause")]
    HyperForm {
        /// The clause's declared name.
        clause: String,
        /// The form the clause has, as the refusal names it.
        form: &'static str,
    },
    /// `hyper-body`: the clause's body is outside the safety fragment.
    #[error("the body of hyper clause `{clause}` is outside the safety fragment")]
    HyperBody {
        /// The clause's declared name.
        clause: String,
    },
    /// `subject-binding`: `subjects` is not one subject per alias, over one
    /// checked package.
    #[error("{}", match alias {
        Some(alias) => format!("alias `{alias}` has no subject over the clause's checked package"),
        None => "the request supplies more subjects than the clause has aliases".to_owned(),
    })]
    SubjectBinding {
        /// The first alias that has no subject or the wrong one; `None` when
        /// every alias has one and the request supplies more subjects.
        alias: Option<String>,
    },
    /// `instance-binding`: `instance` is not one key of the universe of each
    /// object parameter, in parameter order.
    #[error("the instance does not bind parameter `{parameter}` to a key of its universe")]
    InstanceBinding {
        /// The first parameter that breaks the binding.
        parameter: String,
    },
    /// `product-identity`: the clause names no `hyperproperty` node of the
    /// subjects' checked-package emission.
    #[error("hyper clause `{clause}` has no identity in the checked-package emission")]
    ProductIdentity {
        /// The clause's declared name.
        clause: String,
    },
    /// `match-conjunct`: a `match` conjunct evaluated refused, or the meter
    /// ran out during it, on a tuple of joint disjuncts.
    #[error("conjunct {conjunct} of `{clause}` at {site} is refused on tuple ({})", tuple.join(", "))]
    MatchConjunct {
        /// The clause's declared name.
        clause: String,
        /// The conjunct's index in `mu_universal()`.
        conjunct: usize,
        /// The conjunct's site, `hyper <c> match <k>`.
        site: String,
        /// The tuple's component disjuncts, in quantifier order.
        tuple: Vec<String>,
    },
}

impl Refusal {
    /// The refusal's code in FR-315's table.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ItemForm { .. } | Self::ItemTranslation(_) => "item-form",
            Self::RaceOverModel => "invalid-value",
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
            Self::RefinementAnyRow { .. } => "refinement-any-row",
            Self::RefinementOpenArgument { .. } => "refinement-open-argument",
            Self::HyperForm { .. } => "hyper-form",
            Self::HyperBody { .. } => "hyper-body",
            Self::SubjectBinding { .. } => "subject-binding",
            Self::InstanceBinding { .. } => "instance-binding",
            Self::ProductIdentity { .. } => "product-identity",
            Self::MatchConjunct { .. } => "match-conjunct",
        }
    }

    /// The position of this refusal in the order the encoding applies its
    /// checks: rows 1 to 17 of FR-315's refusal table, and 18 to 23 for the
    /// hyper refusals in the order FR-315 "Refusals and their order" states
    /// (the table itself lists no hyper row), and 24 and 25 for the
    /// refinement row refusals, which FR-315 "Refinement queries" checks
    /// before it reads a subject.
    pub const fn row(&self) -> u8 {
        match self {
            Self::ItemForm { .. } | Self::ItemTranslation(_) => 1,
            Self::RaceOverModel => 1,
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
            Self::HyperForm { .. } => 18,
            Self::SubjectBinding { .. } => 19,
            Self::InstanceBinding { .. } => 20,
            Self::ProductIdentity { .. } => 21,
            Self::HyperBody { .. } => 22,
            Self::MatchConjunct { .. } => 23,
            Self::RefinementAnyRow { .. } => 24,
            Self::RefinementOpenArgument { .. } => 25,
        }
    }
}

/// Why no script was written (FR-315 Outputs).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SmtEncodingRefusal {
    /// The item or the subject uses a construct the encoding has no form
    /// for, or a size limit passed.
    #[error("{0}")]
    Refused(Refusal),
    /// An invariant that a well-formed request guarantees did not hold
    /// (FR-096: `runtime_invariant`, `established-invariant-broken`). It
    /// settles `failed`.
    #[error("encoding broke invariant {0:?}")]
    Fault(InternalFault),
}

impl From<Refusal> for SmtEncodingRefusal {
    fn from(refusal: Refusal) -> Self {
        Self::Refused(refusal)
    }
}

impl SmtEncodingRefusal {
    /// A broken invariant of a well-formed request, named by `invariant`.
    pub const fn fault(invariant: &'static str) -> Self {
        Self::Fault(InternalFault::new("smt-encoding", invariant))
    }

    /// The refusal's code in FR-315's table; `None` for a fault.
    pub const fn code(&self) -> Option<&'static str> {
        match self {
            Self::Refused(refusal) => Some(refusal.code()),
            Self::Fault(_) => None,
        }
    }

    /// The row of FR-315's table the refusal is on; a fault precedes every
    /// row.
    pub const fn row(&self) -> u8 {
        match self {
            Self::Refused(refusal) => refusal.row(),
            Self::Fault(_) => 0,
        }
    }
}
