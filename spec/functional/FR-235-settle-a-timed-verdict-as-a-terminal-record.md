---
id: FR-235
title: "Settle a timed verdict as an FR-331 terminal record"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-232
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-236
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-237
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: depends_on
---
# FR-235: Settle a timed verdict as an FR-331 terminal record

## Description

QSL SHALL settle every item over a timed subject as exactly one FR-331
terminal record, extending FR-127's map with ADR-026's bases and causes
(ADR-026 TV-1, CF-4, OV-14). `qsl-replay` owns the additions to
`TerminalValue`; the zone engine's outcome (FR-239), its counterexample's
replay (FR-237) and its certificate's check (FR-245) map to it through one
function.

## Use case

A verification operator reads a run over a timed model. A proof states
whether it was certified by the zone certificate checker or exhaustive over
integer clocks. A proof over a model that admits no time-divergent
behaviour says so instead of reading as proved. A time-lock reads as
refuted, with its trap evidence.

## Inputs

- A `ZoneCheckOutcome` (FR-239).
- For `Violated`: the FR-072 result of its replay (FR-237).
- For `Holds(certificate)`: the result of `check_zone_certificate`
  (FR-245).

## Outputs

`TerminalValue` with these additions to FR-127's types:

```rust
// ProofBasis gains:
//   ZoneCertified,
//   ClosedForm { analysis: ClosedFormAnalysis },          // FR-248
//   BoundedSolver { method: SolverMethod, horizon: ExactRational, jumps: u64 }, // FR-250
// InconclusiveCause gains:
//   NoAdmittedBehaviour, LassoNotConcretized, CertificateRejected,
//   SufficientTestFailed, SolverInconclusive
// UnsupportedCause gains: PunctualInterval, StopwatchRequired
// CounterexampleKind gains: TimeLock
```

## Behavior

- The map SHALL be exhaustive, with no `_` arm, and give each input
  exactly one row:

| Verdict | Input | QSpec FR-341 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| V-1 | `Holds(certificate)` and the checker accepts | `proved` | `closed-scope` | `Proved{basis: ZoneCertified}` | success |
| V-1 | `HoldsDigitized` (FR-243) | `proved` | `closed-scope` | `Proved{basis: Exhaustive}` | success |
| V-4 | `Violated` whose replay reproduces, `kind` `Formula` or `Deadlock` | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-4 | `Violated` with `kind: UndefinedEvaluation{where, cause}` whose replay reproduces the undefined value at `where` | `refuted`, cause `UndefinedEvaluation{where, cause}` | `decisive-counterexample` | `Refuted` | violation |
| V-10 | `Violated` with `kind: TimeLock` whose replay reproduces | `refuted` | `closed-scope` | `Refuted` | violation |
| V-6 | `Holds(certificate)` and the checker rejects | `inconclusive` | `unsettled` | `Inconclusive(CertificateRejected)` | inconclusive |
| V-6 | `Undecided(NoAdmittedBehaviour)` or `Undecided(LassoNotConcretized)`; a replay that settles `inconclusive` or refuses | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-7 | `Stopped(cause, limit)`; a certificate check a budget stopped | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(cause)` | incomplete |
| V-8 | `PunctualInterval` or `StopwatchRequired` disposition; no candidate | `unsupported` | `unavailable` | `Unsupported(cause)` | unsupported |

- A `Holds(certificate)` outcome SHALL settle `proved` only after
  `check_zone_certificate` accepts its certificate. The certificate SHALL
  travel with the outcome from S6c over ADR-018's E11, as a counterexample
  does.
- A `Violated` outcome SHALL settle `refuted` only through its replay.
- `TerminalValue::category` SHALL map `ZoneCertified`, `ClosedForm` and
  `BoundedSolver` to success.
- A counterexample's length SHALL be its number of discrete steps, so V-5's
  definition is unchanged.
- The record SHALL carry the method: `zone-certified`,
  `digitized-explicit-state`, or FR-127's methods.
- A verdict SHALL hold for exactly its subject and claim; the record's
  obligation identity binds both (ADR-013 O-09).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-235-AC-1 | Each table row maps its input to the stated `TerminalValue`, FR-341 label, FR-243 basis and O-16 category; `ZoneCertified`, `ClosedForm{FixedPriorityRta}` and `BoundedSolver{…}` each map to success. | Test (TC-690) |
| FR-235-AC-2 | `Settles` over `Rpc` with `T = 4 ms`, decided by the zone search, settles `proved`, `Proved{ZoneCertified}`; the same outcome with one certificate node removed settles `inconclusive`, `CertificateRejected`. `NoLateReply` over the same subject, decided by digitization (FR-243), settles `Proved{Exhaustive}` with method `digitized-explicit-state`. | Test (TC-690) |
| FR-235-AC-3 | `NoLateReply` with `T = 3 ms` settles `refuted` only after FR-237's replay reproduces; with the `timeout` step's delay changed to `5/2` the replay refuses and the item settles `inconclusive`. The strict-guard variant's time-lock-freedom item settles `refuted`, basis `closed-scope`, with a counterexample whose `kind` is `TimeLock`. | Test (TC-690) |
| FR-235-AC-4 | FR-232-AC-3's `Stall` claim settles `inconclusive`, `NoAdmittedBehaviour`; FR-234-AC-3's punctual liveness claim settles `unsupported`, `PunctualInterval`; a zone search stopped by `max_symbolic_states` settles `failed`, `resource-incomplete`, naming the limit. | Test (TC-690) |
| FR-235-AC-5 | FR-239-AC-5's outcome settles `refuted`, `decisive-counterexample`, category violation, cause `UndefinedEvaluation{where: 2, cause: division-by-zero}`, after FR-237's replay reproduces it. | Test (TC-710) |

## Dependencies

- ADR-026 §7 TV-1, §8.1 CF-4, §16 OV-14; ADR-018 V-1 to V-8 as amended by
  ADR-026; ADR-022 V-10; ADR-013 O-09, O-16 and O-24.
- [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md),
  [FR-069](FR-069-implement-typed-proof-result-envelope.md),
  [FR-232](FR-232-derive-the-time-lock-freedom-item-and-read-deadlocks-over-time.md),
  [FR-236](FR-236-carry-exact-rational-delays-in-a-timed-counterexample.md),
  [FR-237](FR-237-replay-a-timed-counterexample.md),
  [FR-245](FR-245-check-a-zone-certificate-in-the-qualified-core.md).

## References

- QSpec half: Linear STD-139 owns the verdict rows on the wire, the new
  causes and the `TimeLock` kind in QSpec FR-331 (ADR-026 OV-6); this
  requirement cites it until those QSpec FRs merge.
