// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-045: native-to-TL mapping support classification.
//!
//! The disposition is a total function of three inputs and nothing else: the
//! admitted declaration's selected profile, its reachable operator kinds, and the
//! surrounding-execution closure named in the request. The owner ruling on
//! `quire-contract-ir#64` fixes surrounding-execution closure as the axis that
//! selects the TL row; decision-scope closure stays a separate native result axis
//! and is never substituted for it.
//! It consults no backend capability report, no installed TL version, no syntax
//! match and no historical result. It emits no TL formula, no valuation request
//! and no correspondence record: the emission half of the bridge remains blocked
//! on `quire-contract-ir#63`, `quire-contract-ir#64` and actual TL capability.
//!
//! A classification made through a strict version-2 package also retains the
//! authenticated definition selection it was made against. One made through a
//! version-1 package retains none, so the two are never mistaken for each other.

use crate::protocol_artifact::wire as w;
use crate::ByteDigest;

use super::profile::Profile;
use super::result::Subject;
use super::trace::Closure;

/// Reviewed correspondence source this table restates.
pub const SUPPORT_TABLE: &str =
    "quire-specification/FR-095 @ 782c1ce39a197cd52b8b35b50adf2e5e3ecedd0f";

/// A TL target named by the reviewed support table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    /// `mltl.closed-trace/v1`.
    ClosedTrace,
    /// `mltl.online-prefix/v1`.
    OnlinePrefix,
}

impl Target {
    /// Exact TL profile identity named by the reviewed table.
    pub fn identity(self) -> &'static str {
        match self {
            Self::ClosedTrace => "mltl.closed-trace/v1",
            Self::OnlinePrefix => "mltl.online-prefix/v1",
        }
    }
}

/// A semantic dimension the current TL targets cannot express.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unmatched {
    /// The finite-window boundary rule; index-based TL profiles extend atoms
    /// false beyond closure.
    FiniteWindow,
    /// Any bounded past operator, until a separately reviewed TL past profile
    /// exists.
    PastOperator,
}

/// Premises a supported mapping would still have to discharge. Naming them is
/// not discharging them, and this crate discharges none of them.
pub const OUTSTANDING_PREMISES: &[&str] = &[
    "total Boolean predicate projection",
    "source and clause identity",
    "model, type and predicate bindings",
    "evaluation anchor and immutable capture environment",
    "clock and observation binding",
    "interval",
    "closure and history premises",
    "result dimensions the selected TL wire does not encode",
];

/// The classification of one mapping request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Support {
    /// The reviewed table names a TL target for this request.
    Supported {
        /// The TL target this request maps to.
        target: Target,
        /// Reviewed source table this disposition was read from, so a later
        /// revision of that table is a visible change rather than silent
        /// staleness.
        table: &'static str,
        /// Extra condition the fixed-sample row attaches, when it applies.
        total_sample_valuation: bool,
        /// Premises a supported mapping would still have to discharge.
        premises: &'static [&'static str],
    },
    /// No current TL target can express this request.
    Unsupported {
        /// Every unmatched dimension, not one summary cause.
        dimensions: Vec<Unmatched>,
    },
}

/// What a classification retains from the native declaration. Nothing is
/// substituted or reduced: the subject, the selected profile identity and
/// revision, and the activation record are the admitted declaration's own.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Retained {
    /// The native declaration subject the request was made against.
    pub subject: Subject,
    /// The declaration's admitted name.
    pub name: String,
    /// The selected profile, retained by its registered identity.
    pub profile: Profile,
    /// The exact admitted profile revision, distinct from the language edition.
    pub profile_revision: String,
    /// The declaration's admitted activation record.
    pub activation: w::Activation,
}

/// The authenticated definition selection a strict version-2 classification
/// was made against.
///
/// This is retained evidence, not an assertion. It names the admitted package,
/// the declaration, and the registered definition the package's temporal binding
/// selects for it. It carries no clock parameters: classification takes no
/// trace, so it authenticates no parameter map. A later bridge reads the admitted
/// clock configuration from the package this digest names.
///
/// Only the compiler constructs one; private fields prevent a caller from
/// fabricating the evidence:
///
/// ```compile_fail,E0451
/// use quire_spec_language::protocol_artifact::wire::Revision;
/// use quire_spec_language::temporal::AuthenticatedSelection;
/// use quire_spec_language::ByteDigest;
/// let forged = AuthenticatedSelection {
///     package_digest: ByteDigest::of(b"forged"),
///     declaration: 0,
///     definition_identity: String::new(),
///     definition_revision: Revision { namespace: String::new(), value: String::new() },
///     definition_artifact: ByteDigest::of(b"forged"),
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedSelection {
    package_digest: ByteDigest,
    declaration: usize,
    definition_identity: String,
    definition_revision: w::Revision,
    definition_artifact: ByteDigest,
}

impl AuthenticatedSelection {
    pub(super) fn new(
        package_digest: ByteDigest,
        declaration: usize,
        definition: &w::Definition,
        definition_artifact: ByteDigest,
    ) -> Self {
        Self {
            package_digest,
            declaration,
            definition_identity: definition.identity.clone(),
            definition_revision: definition.revision.clone(),
            definition_artifact,
        }
    }

    /// Raw-byte digest of the admitted version-2 package.
    pub fn package_digest(&self) -> ByteDigest {
        self.package_digest
    }

    /// Declaration index in that package.
    pub fn declaration(&self) -> usize {
        self.declaration
    }

    /// Registered temporal definition identity the binding selects.
    pub fn definition_identity(&self) -> &str {
        &self.definition_identity
    }

    /// Registered semantic revision of that definition, with its namespace.
    pub fn definition_revision(&self) -> &w::Revision {
        &self.definition_revision
    }

    /// Raw-byte digest of the exact definition artifact the package selects.
    pub fn definition_artifact(&self) -> ByteDigest {
        self.definition_artifact
    }
}

/// One classification request: its disposition, and everything the request
/// retains from the native declaration it was made against.
///
/// `support` and `retained` stay public fields for the existing version-1
/// callers, so they are plain data a caller may copy or change. Only the
/// authenticated selection is sealed. It vouches for itself — the package,
/// declaration and definition it names — and a consumer that needs the
/// disposition bound to that evidence re-classifies from the package the
/// selection names rather than trusting a copied value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Classification {
    /// The disposition read from the reviewed support table.
    pub support: Support,
    /// The retained native subject, profile and activation record.
    pub retained: Retained,
    authenticated: Option<AuthenticatedSelection>,
}

impl Classification {
    pub(super) fn new(
        support: Support,
        retained: Retained,
        authenticated: Option<AuthenticatedSelection>,
    ) -> Self {
        Self {
            support,
            retained,
            authenticated,
        }
    }

    /// The definition selection a strict version-2 entry point authenticated
    /// before classifying. `None` for a version-1 classification, which
    /// authenticates nothing.
    pub fn authenticated(&self) -> Option<&AuthenticatedSelection> {
        self.authenticated.as_ref()
    }
}

/// Which operator kinds a declaration's temporal graph reaches.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Operators {
    /// A bounded or unbounded future operator is reachable.
    pub future: bool,
    /// A bounded or unbounded past operator is reachable.
    pub past: bool,
}

impl Operators {
    /// Walk the emitted temporal arena. Every node is inspected once; the graph
    /// is a flat arena, so no traversal is required.
    pub fn of(nodes: &[w::Temporal]) -> Self {
        let mut result = Self::default();
        for node in nodes {
            match &node.operation {
                w::TemporalOperation::Unary { operator, .. } => match operator {
                    w::TemporalUnary::Eventually | w::TemporalUnary::Always => result.future = true,
                    w::TemporalUnary::Once | w::TemporalUnary::Historically => result.past = true,
                    w::TemporalUnary::Not => {}
                },
                w::TemporalOperation::Binary { operator, .. } => match operator {
                    w::TemporalBinary::Until | w::TemporalBinary::Release => result.future = true,
                    w::TemporalBinary::Since | w::TemporalBinary::Triggered => result.past = true,
                    w::TemporalBinary::And | w::TemporalBinary::Or | w::TemporalBinary::Implies => {
                    }
                },
                w::TemporalOperation::Constant { .. }
                | w::TemporalOperation::Holds { .. }
                | w::TemporalOperation::Group { .. } => {}
            }
        }
        result
    }
}

/// Classify one mapping request against the reviewed support table.
///
/// Unmatched dimensions accumulate: a bounded past operator under the
/// finite-window profile names both.
pub fn classify(profile: Profile, operators: Operators, surrounding_execution: Closure) -> Support {
    let mut dimensions = Vec::new();
    if profile == Profile::TimestampedWindow {
        dimensions.push(Unmatched::FiniteWindow);
    }
    if operators.past {
        dimensions.push(Unmatched::PastOperator);
    }
    if !dimensions.is_empty() {
        return Support::Unsupported { dimensions };
    }
    Support::Supported {
        target: match surrounding_execution {
            Closure::Closed => Target::ClosedTrace,
            Closure::Open => Target::OnlinePrefix,
        },
        table: SUPPORT_TABLE,
        // The fixed-sample row attaches its condition only where a bounded
        // future operator is reachable; a formula reaching no bounded operator
        // at all matches the last row instead and imposes no such obligation.
        total_sample_valuation: profile == Profile::FixedSample && operators.future,
        premises: OUTSTANDING_PREMISES,
    }
}
