// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-122 (ADR-013 O-25 to O-27): the state-clause family's own
//! [`FamilyPayload`], [`StateClauseCounterexample`] -- an observation under
//! which an invariant, a precondition or a postcondition evaluates `false`,
//! carried on the FR-070 envelope as
//! `WitnessEnvelope<StateClauseCounterexample>`.
//!
//! The payload names the clause and its one observation. The clause's
//! checked identities travel once, as the envelope's `clause_node` and
//! `occurrence_key` (ADR-017 PF-4, G-2). The replay reads it in
//! `crate::execute` (`replay_state_clause`).

use quire_exact::Identifier;

use super::FamilyPayload;
use crate::result::SeparatingWitnessRecord;
use qsl_semantics::model::observation::ClauseSelectionInput;

/// FR-122: the state-clause counterexample payload (FR-070-AC-5). Every
/// member is typed; none is a `String`-keyed map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateClauseCounterexample {
    /// The state clause's declared name, as FR-106's `ClauseSelection`
    /// names it.
    pub clause: Identifier,
    /// The observation, by the clause's kind: `PreCall` for a
    /// precondition, `Invocation` for a postcondition, `Current` for an
    /// invariant. Its documents are read from the replay request's byte
    /// provision by their `sha256-jcs` digests.
    pub observation: ClauseSelectionInput,
    /// FR-268 (ADR-031 SW-8): the clause's separating witness record,
    /// present exactly when the producing run's settlement basis was
    /// decisive (FR-265).
    pub witness: Option<SeparatingWitnessRecord>,
}

impl FamilyPayload for StateClauseCounterexample {
    /// The inline size plus the witness record's own measured size
    /// (ADR-031 SW-14), so an oversized record refuses at decode.
    fn measured_bytes(&self) -> usize {
        std::mem::size_of_val(self)
            + self
                .witness
                .as_ref()
                .map_or(0, SeparatingWitnessRecord::measured_bytes)
    }
}
