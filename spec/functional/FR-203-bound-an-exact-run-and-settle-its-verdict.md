---
id: FR-203
title: "Bound an exact run and settle its verdict"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-023
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-202
    type: depends_on
---
# FR-203: Bound an exact run and settle its verdict

## Description

EN-5's cost SHALL be bounded only by `ExactProbLimits`, ADR-014 B-5 budgets
that the request sets, each with a published default (ADR-028 LM-1 to
LM-3). Reaching one SHALL stop the run and name the budget, its value and
the request member that raises it. QSL SHALL settle every item routed to
EN-5 as exactly one FR-331 terminal record with ADR-018's verdict kinds and
the `ProofBasis` members `ExactValue` and `ValueBounds` (XV-1 to XV-7),
never with ADR-024's `measured`.

## Use case

An operator's exact check of a large model stops at `max_states`. The result
names `max_states`, its value and `model_check.max_states`; the operator raises
it and reruns. A run that completes settles `proved` with `ExactValue` and
the least favourable initial state, or `refuted` after its witness replays.

## Inputs

```rust
pub struct ExactProbLimits {
        pub model: ModelCheckLimits,      // FR-126: max_states, max_transitions, max_automaton_states, its defaults

    pub max_iterations: u64,          // interval-iteration sweeps; default 1_000_000
    pub precision_bits: u32,          // starting dyadic precision; default 64
    pub max_precision_bits: u32,      // default 4_096
    pub max_rational_bits: u64,       // default 65_536
    pub max_policy_iterations: u64,   // default 10_000
    pub max_witness_paths: u64,       // default 1_024
}
pub enum ExactProbLimit { MaxStates, MaxTransitions, MaxAutomatonStates, MaxIterations, MaxPrecisionBits, MaxRationalBits, MaxPolicyIterations, Time, ClauseMeter, Cancelled }
```

- An EN-5 outcome (FR-197 to FR-200) with its certificate check (FR-201) or
  witness replay (FR-202).

## Outputs

```rust
// ProofBasis gains:
//   ExactValue { value: ExtRational, reductions: Vec<AppliedReduction> },
//   ValueBounds { lower: Rational, upper: Rational, method: BoundsMethod, reductions: Vec<AppliedReduction> },
pub enum BoundsMethod { BackwardInduction { precision_bits: u32 }, IntervalIteration { precision_bits: u32, sweeps: u64 } }
// IncompleteCause gains: PrecisionBudget { lower: Rational, upper: Rational }

// UnsupportedCause gains: ZeroWeightComponent, StrictClockConstraint, DelayDistribution,
//   ZeroDelayCycle, TimedFormShape
pub struct ExactEntry { pub initial: u32, pub binding: Option<Binding>, pub value: ExactOrBounds, pub extremum: Option<Extremum> } // Extremum: Min or Max
```

## Behavior

### Budgets

- Each `ExactProbLimits` member SHALL be set by the request, with the
  default above when omitted. The `model` members SHALL keep FR-126's
  setting names and defaults; every other member's setting name SHALL be
  `exact_probabilistic.` followed by the member name, by FR-255's
  convention. QSpec FR-411 owns each limit and its setting name; QSL
  publishes the default values. No member SHALL be fixed by the language, and none
  enters the obligation identity. The method choices the budgets drive
  SHALL be recorded in the result.
- Reaching a budget before the method completes SHALL settle V-7:
  `Incomplete(PrecisionBudget{lower, upper})` when an interval still
  straddles the threshold at `max_precision_bits`, and
  `Incomplete(LimitReached{limit, value, setting})` naming the budget, its
  value and its setting otherwise, unless partial evidence already settles the
  item (FR-202).

### Settlement

- The map from EN-5's outcome to `TerminalValue` SHALL be exhaustive, with
  no `_` arm:

| Outcome | FR-331 value | FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- |
| Exact value on the claim's side, certificate accepted | `proved` | `closed-scope` | `Proved{basis: ExactValue{…}}` | success |
| Sound interval on the claim's side, certificate accepted | `proved` | `closed-scope` | `Proved{basis: ValueBounds{…}}` | success |
| Bound fails, witness replays | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| Undefined evaluation at a state of positive probability, `Undefined` path replays | `refuted`, cause `UndefinedEvaluation{where, cause}` | `decisive-counterexample` | `Refuted` | violation |
| Interval straddles at the budget | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(PrecisionBudget{…})` | incomplete |
| Another limit reached | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` (ADR-018 V-7) | incomplete |
| Cancelled through the `Cancel` handle | `incomplete`, `cancelled` | `unavailable` | `Incomplete(Cancelled)` (ADR-018 V-7) | incomplete |
| Certificate rejected; replay disagrees or refuses | `inconclusive` | `unsettled` | `Inconclusive(CertificateRejected{rule, state})`, `Inconclusive(ReplayParity)`, `Inconclusive(ReplayRefused)` | inconclusive |
| `NotMarkov`, `ZeroWeightComponent`, a timed cause (FR-204) | `unsupported` | `unavailable` | `Unsupported(cause)` | unsupported |

- `TerminalValue::category` SHALL map both new `ProofBasis` members to
  success, and proof accounting SHALL count them as proof.
- The record SHALL carry FR-242 truth and an FR-243 basis, and no `measured`
  value. The basis SHALL state the least favourable (initial state, binding)
  pair; the result SHALL keep one entry per pair with its value or interval
  and, over every scheduler, whether it is a minimum or a maximum, and, with
  a fairness set, the set.
- The obligation identity SHALL bind the subject, the claim, its scheduler or
  `every`, its fairness set and its stated confidence parameters; the
  evidence kind SHALL enter the item's request, not the claim.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-203-AC-1 | §15.2 at `5 ms` settles `proved`, `closed-scope`, `Proved{ExactValue{24233/25000}}`, success, with one entry. §15.1 with `max_rational_bits` 1,024 settles `Proved{ValueBounds{…, BackwardInduction{64}}}`. §15.4's `Deliver` settles `refuted`, `decisive-counterexample`, after FR-202's replay, with an entry marked `Min`. | Test (TC-638) |
| FR-203-AC-2 | FR-197-AC-4's straddling run settles `Incomplete(PrecisionBudget{lower, upper})` with `lower < 99/100 < upper`. §15.3's per-window claim with `max_states` 1,000 stops before the product completes and no partial evidence settles it: `Incomplete(ResourceExhausted)` naming `limits.max_states` and 1,000. A request omitting every limit runs with the defaults above. | Test (TC-638) |
| FR-203-AC-3 | FR-201-AC-1's rejected certificate settles `inconclusive`, `CertificateRejected`; FR-202-AC-2's prefix-duplicated path set settles `inconclusive`, `ReplayParity`; FR-199-AC-3's zero-weight variant settles `unsupported`, `ZeroWeightComponent`. None of these records carries a `measured` value. | Test (TC-638) |
| FR-203-AC-4 | A request whose proof summary holds AC-1's two proofs counts 2 proved items. `P95` with and without stated confidence parameters have different obligation identities, and the same claim requested with `exact` and with a changed `precision_bits` has one obligation identity. | Test (TC-638) |
| FR-203-AC-5 | FR-196-AC-5's refutation settles `refuted`, `decisive-counterexample`, `Refuted`, category violation, with cause `UndefinedEvaluation{where: 1, cause: division-by-zero}`, after FR-202's replay; the claim's threshold plays no part. | Test (TC-641) |

## Dependencies

- ADR-028 LM-1 to LM-3, XV-1 to XV-7, AR-4, SP-5; ADR-014 B-5 and ADR-013
  O-16, O-24 as amended by ADR-028.
- [FR-069](FR-069-implement-typed-proof-result-envelope.md),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the terminal record and `ProofBasis` it extends),
  [FR-201](FR-201-check-a-probability-certificate.md),
  [FR-202](FR-202-replay-a-probabilistic-witness.md).

## References

- QSpec half, which owns `ExactProbLimits` and its setting names in the
  request, while QSL publishes the default values, and the result content
  onto FR-331 and FR-243: QSpec FR-411 (Linear STD-137).
- Owning ticket: Linear QSL-371.
