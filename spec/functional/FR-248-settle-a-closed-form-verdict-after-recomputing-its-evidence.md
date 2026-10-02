---
id: FR-248
title: "Settle a closed-form verdict after recomputing its evidence"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-247
    type: depends_on
---
# FR-248: Settle a closed-form verdict after recomputing its evidence

## Description

`qsl_replay::check_closed_form`, a layer-6 facade entry, SHALL recompute
EN-7's evidence in exact rationals before a schedulability verdict settles,
as the zone certificate checker checks a certificate (ADR-026 RT-7). A
schedulable result settles `proved` with basis `ClosedForm{analysis}`; a
miss settles `refuted` with its evidence; a disagreement settles
`inconclusive`. Every verdict is conditional on the task set's stated
premises, which the subject carries.

## Use case

An assessor accepts a schedulability proof because the core recomputes
each fixpoint, iterate or demand point from the task set itself, without
trusting the analysis that produced them.

## Inputs

```rust
pub fn check_closed_form(
    request: ReplayRequest<'_>,            // FR-098: package, byte provision, limits
    item: &ObligationIdentity,
    outcome: &ClosedFormOutcome,           // FR-247
    cancel: &Cancel,                       // FR-276
) -> Result<ClosedFormCheck, CertificateRefusal>; // Agrees | Disagrees(reason)
```

## Outputs

The FR-331 terminal record of the claim.

## Behavior

- The checker SHALL recompile and check the package by FR-098's rules and
  read the task set and the claim from the recompiled item, never from the
  caller.
- If the outcome's obligation identity differs from `item` or from the
  recompiled item's identity, then the checker SHALL refuse
  `stale_dependency`/`content-mismatch`, naming both, as FR-245 does.
- For `Schedulable` with fixpoints, the checker SHALL check that each
  stated `R` satisfies the recurrence exactly, that no smaller value does,
  and that `R + J_i <= D_i`.
- For `Schedulable` under EDF, the checker SHALL recompute the utilization,
  the busy-period length and `dbf` at every point of the QPA sequence.
- For a fixed-priority `Miss`, the checker SHALL recompute each stated
  iterate from the previous one by the recurrence for job `q`,
  `R = (q + 1) · C_i + B_i + Σ_{j ∈ hp(i)} ⌈(R + J_j) / T_j⌉ · C_j`,
  starting from `(q + 1) · C_i + B_i`, and check that only the last gives a
  response time `R + J_i − q · T_i` above `D_i`. For `q > 0` it SHALL also
  compute each earlier job's completion `w_p`, the fixpoint of the same
  recurrence for job `p`, and check that every job `p < q` keeps the level-`i`
  busy period open, `w_p > (p + 1) · T_i − J_i`; otherwise job `q` lies past
  the busy period, its recurrence overstates its completion, and the
  checker SHALL disagree, naming the first job `p` that closes it. For an
  EDF `Miss`, it SHALL recompute the utilization, or `dbf(t) > t`.
- The checker's work SHALL be linear in the size of the evidence, which
  EN-7's `max_demand_points` already bounds, so the check always completes.
- For `SufficientTestFailed`, the checker SHALL recompute the low-mode
  fixpoints and the failing high-mode criterion.
- The map SHALL be:

| Input | QSpec FR-360 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- |
| `Schedulable` that the checker agrees with | `proved` | `closed-scope` | `Proved{basis: ClosedForm{analysis}}` | success |
| `Miss` that the checker agrees with | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| `SufficientTestFailed` that the checker agrees with | `inconclusive` | `unsettled` | `Inconclusive(SufficientTestFailed)` | inconclusive |
| any outcome the checker disagrees with | `inconclusive` | `unsettled` | `Inconclusive(ReplayParity)` | inconclusive |
| `Stopped` | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` or `Incomplete(Cancelled)` (ADR-018 V-7) | incomplete |

- `analysis` SHALL be `FixedPriorityRta`, `EdfQpa` or `AmcRtb`.
- The record SHALL carry the evidence, and its obligation identity binds
  the task set with every premise.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-248-AC-1 | `schedulable Ctl under fixed-priority` settles `proved`, `Proved{ClosedForm{FixedPriorityRta}}`, with fixpoints `(1, 3, 12)` in the record; `under edf` settles `Proved{ClosedForm{EdfQpa}}`. With the third WCET 6 both settle `refuted` with their evidence. | Test (TC-703) |
| FR-248-AC-2 | An outcome with the fixpoint 12 replaced by 11 settles `inconclusive`, `ReplayParity`; a miss whose last iterate 13 is replaced by 12 settles `inconclusive`, `ReplayParity`. For `a` (`C = 1`, `T = D = 2`, priority 1) and `b` (`C = 1`, `T = 3`, `D = 1`, blocking 1, priority 2), evidence naming `b`'s job 2 with iterates `4, 6, 7, 8` settles `inconclusive`, `ReplayParity`, naming job 1, whose completion 6 does not exceed `2 · 3`; the evidence naming job 0 with the iterate `2` settles `refuted`. | Test (TC-703) |
| FR-248-AC-3 | FR-247-AC-3's `C(HI) = 6` result settles `inconclusive`, `SufficientTestFailed`; its base result settles `Proved{ClosedForm{AmcRtb}}`. | Test (TC-703) |
| FR-248-AC-4 | `Ctl`'s fixed-priority outcome checked against the identity of `schedulable Ctl under edf` refuses `stale_dependency`/`content-mismatch` naming both; after a source edit that raises a WCET in the package, the same outcome refuses by FR-098's rule; the checker reads the task set from the recompiled package, so a caller cannot supply one. | Test (TC-703) |

## Dependencies

- ADR-026 §12 RT-7; ADR-013 O-09 and O-16.
- [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md),
  [FR-235](FR-235-settle-a-timed-verdict-as-a-terminal-record.md),
  [FR-246](FR-246-check-task-sets-and-schedulability-claims.md),
  [FR-247](FR-247-analyse-a-task-set-in-closed-form.md).

## References

- The checker as an in-core entry beside `replay`, in `qsl-replay`:
  ADR-029 CB-2, Linear QSL-390.
- QSpec half: QSpec FR-419 (Linear STD-139) owns the closed-form evidence
  wire (ADR-026 OV-10).
