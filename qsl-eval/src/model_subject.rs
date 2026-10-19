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

use qsl_foundation::bound::ProofBound;
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Incomplete, Meter, ObjectReference};

use crate::simulation::model::{ModelSimState, ModelSimulationRequest};
use crate::temporal::AtomReading;

/// FR-125's model subject.
pub struct ModelSubject<'a> {
    /// FR-120's request: package, provisions, initial states, universes,
    /// expansion limits and evaluation meter budget.
    pub simulation: ModelSimulationRequest<'a>,
    /// The ADR-014 B-4 proof bounds the item is qualified by.
    pub proof_bounds: Vec<ProofBound>,
}

/// Reads the letter of a clause over a model subject at one state: the
/// value of each `holds` atom for one binding of the clause's `over`
/// parameter (ADR-018 UE-1, QSpec FR-361's `over` binding).
pub trait ModelLetters {
    /// The instances of the clause's `over` parameter: one per key of each
    /// population that holds the parameter's type (`T` or a subtype), in
    /// FR-125's canonical order: population declaration identity text, then
    /// the population's universe-list order. A binding names its instance by
    /// population and key; its object type only completes the reference and
    /// is not compared.
    fn bindings(&self) -> Vec<ObjectReference>;

    /// The `holds` atom `atom` for the instance `binding` at `state`,
    /// charging `meter`. The implementation matches the state's object to
    /// `binding` by population and key, never by comparing the whole
    /// reference, and reads the bound parameter as the state's own reference
    /// to that object (FR-125 Instancing); where the population has no object
    /// under the key conforming to the parameter's type, the read is
    /// `absent-key` undefined. An exhausted meter is `Err(Incomplete)`, never a
    /// reading.
    fn atom(
        &self,
        atom: FormulaNodeId,
        binding: &ObjectReference,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete>;
}
