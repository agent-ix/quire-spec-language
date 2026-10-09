---
id: TC-522
title: "Model-check outcomes settle as QSpec FR-331 terminal records with their strength"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: verifies
---
# TC-522: Model-check outcomes settle as QSpec FR-331 terminal records with their strength

## Description

Verify the map from EN-1's outcomes and their replay results to
`TerminalValue`, QSpec FR-360 labels, FR-243 bases and O-16 categories; the
category of every `ProofBasis` and new inconclusive cause; each replay
refusal path; a faulting replay; and the named limit of a run that reached it.

Scope: FR-127-AC-1 to FR-127-AC-5 and FR-127-AC-7 to FR-127-AC-10.

## Test Procedure

1. Settle one input of each row of FR-127-AC-1.
2. Read `TerminalValue::category` for each value of FR-127-AC-2.
3. Settle FR-126-AC-1's outcomes: the weak `each` proof; the counterexample
   under the constraint with no granularity, with its replay; that
   counterexample with one post-state digest altered; in an envelope for
   the weak `each` clause; with its last step removed; one whose formula
   evaluates `true` on replay; one whose replay returns `InternalFault`
   through a replay stand-in.
4. Settle FR-126-AC-6's `max_automaton_states` run and FR-126-AC-5's
   `max_depth` 2 and evaluation-meter runs; then a run whose `Cancel`
   handle is cancelled with `CancelCause::Requested`, and one cancelled with
   `CancelCause::Deadline`.
5. Settle FR-126-AC-3's deadlock-freedom violation with its replay.
6. Settle FR-338-AC-1's TP-1 proof with its certificate, and with
   FR-338-AC-2's `(1, 0)`-removed certificate.
7. Settle FR-339-AC-1's weak `each` proof with its certificate, and with
   FR-339-AC-2's `upd(a)` witness.
8. Read the label and category of `Proved{Checks{3}, Certified}`,
   `Proved{BoundedComplete{depth: 5}, Certified}`,
   `Proved{BoundedComplete{depth: 5}, Uncertified}`,
   `Proved{Inductive{depth: 2}, Uncertified}` and
   `Proved{Exhaustive, Trusted}`, and the category of
   `Inconclusive(CertificateRejected)`.
9. Settle a replay result with cause `Verdicts`.

Tag the tests `#[trace("TC-522", "FR-127-AC-n")]`.

## Expected Results

- Step 1: each row exactly as FR-127's table; depth 2 and method
  `explicit-state` on V-5; the limit-reached record names `max_states`, value 2, setting `max_states`.
- Step 2: inconclusive `KaniVacuousProof`; success five times, each value
  keeping its certification; inconclusive four times.
- Step 3: `proved`, `closed-scope`, `Proved{Exhaustive, Certified}`, success;
  `refuted`, `decisive-counterexample`, violation; `inconclusive`,
  `ReplayRefused`; `inconclusive`, `ReplayParity` (the unfair lasso);
  `inconclusive`, `ReplayRefused`; `inconclusive`, `ReplayParity`;
  `failed`, category failed.
- Step 4: `incomplete`, `unavailable`,
  `Incomplete(LimitReached{MaxAutomatonStates, 50, max_automaton_states})`,
  the record stating the three limits used;
  `inconclusive`, `BoundReached{depth: 2}`, execution `completed`, truth
  `pending`; a limit-reached record naming `EvaluationMeter`, value 0;
  `incomplete`, `Incomplete(Cancelled{Requested})`, written
  `cancelled{source: "requested"}`, and `Incomplete(Cancelled{Deadline})`,
  written `cancelled{source: "deadline"}`.
- Step 5: `refuted` with a counterexample of `kind: Deadlock`, and an
  obligation identity distinct from the authored claims'.
- Step 6: `Proved{Exhaustive, Certified}`; `inconclusive`,
  `CertificateRejected{SuccessorMissing, (1, 0)}`.
- Step 7: `Proved{Exhaustive, Certified}`; `inconclusive`,
  `CertificateRejected{WitnessFails, at}`, `at` `{(*, 0, q1)}`'s first state.
- Step 8: `proved`, success, five times, labelled `Certified`, `Certified`,
  `Uncertified`, `Uncertified` and `Trusted`; inconclusive.
- Step 9: `inconclusive`, `ReplayParity`, written `replay-parity`.
