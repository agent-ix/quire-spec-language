// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-125 (ADR-018 SM-2, SM-3, LA-2): the model subject of a temporal item,
//! and the seam through which a model check reads a clause's letter.
//!
//! A subject is FR-120's simulation request (the S4 package, its domain
//! packages and snapshot provision, the initial states, the universes and
//! the expansion limits and evaluation meter) together with any proof
//! bounds, which qualify the item's identity and make no root finite
//! (ADR-018 "EN-1 pre-check").
//!
//! A behaviour's position `i` is the state its `i`-th step reached, observed
//! by FR-120's synthesized state observation. The `holds` atoms of a clause
//! are read at a position through [`ModelLetters`], the seam to the one
//! clause evaluator (FR-107), as the trace evaluator reads them through
//! [`crate::temporal::TemporalLetters`].

use std::collections::BTreeSet;

use qsl_foundation::bound::ProofBound;
use qsl_semantics::check::footprint::{FootprintRefusal, SymbolicLocation};
use qsl_semantics::check::FormulaNodeId;
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::{Incomplete, Meter};

use crate::simulation::model::{ModelSimState, ModelSimulationRequest};
use crate::simulation::protocol::ResolvedMemory;
use crate::temporal::AtomReading;

/// FR-125's model subject.
pub struct ModelSubject<'a> {
    /// FR-120's request: package, provisions, initial states, universes,
    /// expansion limits and evaluation meter budget.
    pub simulation: ModelSimulationRequest<'a>,
    /// The ADR-014 B-4 proof bounds the item is qualified by.
    pub proof_bounds: Vec<ProofBound>,
}

/// FR-205 (ADR-027 PS-1): a protocol subject, a model subject together with
/// one checked protocol clause of its package and the clause's bindings.
pub struct ProtocolSubject<'a> {
    /// The model subject the protocol's roles act on.
    pub model: ModelSubject<'a>,
    /// The declared name of one protocol clause of the subject's package.
    pub protocol: String,
    /// The object bound to the clause's `over` parameter, by population and
    /// key.
    pub over: ModelInstance,
    /// A binding for each static role.
    pub roles: Vec<RoleBinding>,
    /// The resolved memory model of each outermost `parallel` (FR-219,
    /// ADR-027 SE-4): part of the subject, and so of the obligation
    /// identity.
    pub memory: ResolvedMemory,
}

/// One static role's binding (ADR-027 PS-1): the role and the identity of
/// the domain package whose model plays it.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RoleBinding {
    /// The role's declared name.
    pub role: String,
    /// The domain package identity of the model bound to it.
    pub model: String,
}

/// FR-125 Instancing: one instance of a clause's `over` parameter, a key
/// of a population that admits the parameter's type. The pair is the
/// instance in the obligation identity and the name of a refuted binding;
/// it carries no object type, since a key has no type until a state holds
/// its object.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModelInstance {
    /// The population's declaration key.
    pub population: DeclarationKey,
    /// The key in the population's universe.
    pub key: String,
}

/// Reads the letter of a clause over a model subject at one state: the
/// value of each `holds` atom for one instance of the clause's `over`
/// parameter (ADR-018 UE-1, QSpec FR-361's `over` binding). The instances
/// are derived from the subject's universes
/// ([`crate::simulation::model::ModelSystem::instances`]), never supplied
/// here.
pub trait ModelLetters {
    /// The `holds` atom `atom` for `instance` at `state`, charging `meter`.
    /// The implementation reads the bound parameter as the state's own
    /// reference to the object under the instance's population and key
    /// (FR-125 Instancing); where the population has no object under the
    /// key conforming to the parameter's type, the read is `absent-key`
    /// undefined. An exhausted meter is `Err(Incomplete)`, never a reading.
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete>;

    /// The `when` condition of the clause's guarded `on each` activation for
    /// `instance` at `state`, charging `meter`: true where the position
    /// activates the clause, false where it does not, undefined where the
    /// condition has no value. The engine calls it only for a clause whose
    /// activation has a condition, at each position the clause's monitor
    /// reads. An exhausted meter is `Err(Incomplete)`, never a reading. The
    /// reading is as invariant under a symmetry of the subject as a `holds`
    /// atom's (FR-151).
    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete>;

    /// FR-156 "Atom footprints" for the guarded activation's condition: the
    /// locations it reads, with the clause's `over` parameter as the
    /// receiver. A model check under partial-order reduction reads them for
    /// a guarded clause alone.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] when the condition reads through a form QSpec
    /// FR-383 gives no footprint rule.
    fn activation_reads(&self) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal>;

    /// FR-156 "Atom footprints": the locations `atom` reads, with the
    /// clause's `over` parameter as the receiver. A model
    /// check under partial-order reduction instantiates them per instance;
    /// no other check reads them.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] when the atom reads through a form QSpec FR-383
    /// gives no footprint rule.
    fn reads(&self, atom: FormulaNodeId) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal>;
}
