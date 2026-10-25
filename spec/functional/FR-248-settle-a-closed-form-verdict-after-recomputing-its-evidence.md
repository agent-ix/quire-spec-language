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
pub struct ClosedFormResult {
    pub originating_obligation: ObligationIdentity, // identity of the obligation analyzed
    pub outcome: ClosedFormOutcome,                 // FR-247's result for that obligation
}

pub fn check_closed_form(
    request: ReplayRequest<'_>,            // FR-098: package, byte provision, limits
    item: &ObligationIdentity,
    claim: &ClaimTriple,                   // FR-454: [node, "claim", ordinal]
    result: &ClosedFormResult,
    cancel: &Cancel,                       // FR-276
) -> Result<ClosedFormCheck, CertificateRefusal>; // Agrees | Disagrees(reason)
```

`ClosedFormResult` carries the originating obligation identity alongside the
outcome FR-247 produced for it. The caller SHALL retain the identity resolved
for the analysis claim; the checker verifies it against both `item` and the
identity of the claim resolved from the recompiled package.

## Outputs

The FR-331 terminal record of the claim.

## Behavior

- The checker SHALL recompile and check the package by FR-098's rules and
  read the task set and the claim from the recompiled package through
  `claim`, the request item's `[node, "claim", ordinal]` triple resolved by
  FR-454's schedulability lookup, never from the caller or the outcome.
- If `claim` does not resolve in the recompiled package (FR-454's
  `clause-root-unresolved`), then the checker SHALL return that refusal and
  check no evidence, and the settlement SHALL settle the item `Failed`: the
  unresolved root is a fault of the request, not of the evidence, as FR-410
  states for a state-graph triple.
- If `result.originating_obligation` differs from `item` or from the identity
  of the claim `claim` resolves to, then the checker SHALL refuse
  `stale_dependency`/`content-mismatch`, naming the originating identity and
  each differing expected identity, as
  [FR-245](./FR-245-check-a-zone-certificate-in-the-qualified-core.md) does.
  Each identity is keyed from its claim's FR-454 node, so two claims with
  different nodes have different identities.
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
| `result.outcome` is `Schedulable` and the checker agrees with it | `proved` | `closed-scope` | `Proved{basis: ClosedForm{analysis}, certification: Certified}` | success |
| `result.outcome` is `Miss` and the checker agrees with it | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| `result.outcome` is `SufficientTestFailed` and the checker agrees with it | `inconclusive` | `unsettled` | `Inconclusive(SufficientTestFailed)` | inconclusive |
| any `result.outcome` the checker disagrees with | `inconclusive` | `unsettled` | `Inconclusive(ReplayParity)` | inconclusive |
| `claim` unresolved in the recompiled package (`clause-root-unresolved`) | `failed` | `unavailable` | `Failed` | failed (ADR-013 O-16 internal failure) |
| `result.outcome` is `Stopped` | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` or `Incomplete(Cancelled{source})` (ADR-018 V-7) | incomplete |

- `analysis` SHALL be `FixedPriorityRta`, `EdfQpa` or `AmcRtb`.
- The record SHALL carry the evidence, and its obligation identity binds
  the task set with every premise.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-248-AC-1 | `schedulable CtlFp on Ctl under fixed_priority;` settles `proved`, `Proved{ClosedForm{FixedPriorityRta}, Certified}`, with fixpoints `(1, 3, 12)` in the record; `schedulable CtlEdf on Ctl under edf;` settles `Proved{ClosedForm{EdfQpa}, Certified}`. With the third WCET 6 both settle `refuted` with their evidence. | Test (TC-703) |
| FR-248-AC-2 | An outcome with the fixpoint 12 replaced by 11 settles `inconclusive`, `ReplayParity`; a miss whose last iterate 13 is replaced by 12 settles `inconclusive`, `ReplayParity`. For `a` (`C = 1`, `T = D = 2`, priority 1) and `b` (`C = 1`, `T = 3`, `D = 1`, blocking 1, priority 2), evidence naming `b`'s job 2 with iterates `4, 6, 7, 8` settles `inconclusive`, `ReplayParity`, naming job 1, whose completion 6 does not exceed `2 · 3`; the evidence naming job 0 with the iterate `2` settles `refuted`. | Test (TC-703) |
| FR-248-AC-3 | FR-247-AC-3's `C(HI) = 6` result settles `inconclusive`, `SufficientTestFailed`; its base result settles `Proved{ClosedForm{AmcRtb}, Certified}`. | Test (TC-703) |
| FR-248-AC-4 | A `ClosedFormResult` carrying `CtlFp`'s obligation identity and fixed-priority outcome, checked against `CtlEdf`'s item and claim triple, refuses `stale_dependency`/`content-mismatch` naming both identities, keyed from their FR-454 nodes whose `policy` differs; after the source bytes in the request are edited to raise a WCET while its expected package identity remains unchanged, the same result refuses by FR-098's package-identity rule; the checker reads the task set from the recompiled package, so a caller cannot supply one. | Test (TC-703) |
| FR-248-AC-5 | A `ClosedFormResult` carrying `CtlFp`'s obligation identity and fixed-priority outcome, checked with `claim` the triple FR-454 returns for `CtlFp`, reads `Ctl` from the recompiled package and settles as AC-1 states; with `claim` naming an unknown NodeId, `CtlFp`'s node with another ordinal, or a `claim`/`state_graph` node, the checker returns `clause-root-unresolved`, checks no evidence, and the item settles `failed`, `unavailable`, `Failed`; with `claim` the triple of `CtlEdf`, it refuses `stale_dependency`/`content-mismatch` naming the identities of `CtlFp` and `CtlEdf`. | Test (TC-703) |

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
