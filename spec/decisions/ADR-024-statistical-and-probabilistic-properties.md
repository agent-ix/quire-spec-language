---
id: ADR-024
title: "Statistical and probabilistic properties"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-241
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-242
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-243
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
---
# ADR-024: Statistical and probabilistic properties

## Status

Proposed, 2026-10-01. The QSL compiler requirements that implement it are
FR-185 to FR-194, under US-022. §10 records the
owner's rulings on the draft's five questions, folded into the decision
below. The owning ticket and related work are listed under References.

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. Item ids `PM-`, `PF-`, `ST-`, `WA-`, `SV-`, `RP-`, `RX-`, `DS-`,
`QS-` and `RU-` are local to this record. Other artifacts cite them as
`ADR-024 SV-1`. ADR-018 items are cited as `ADR-018 SM-3`.

## Context

QSL states and checks Boolean properties. ADR-018 gives a temporal claim a
verdict over every behaviour of a finite model; ADR-014 A-4 evaluates a
temporal clause over one given trace. Neither can state "p95 response time is
under 5 ms" or "available 99.9% of the time". Those statements are about how
often something happens, so they need a probability measure over behaviours
(for a model) or a statistic over observed events (for a live system).

What exists today:

- **Nondeterministic models.** FR-120's `ModelSystem` gives a checked
  package's successor relation: an operation applied to an argument vector
  from finite parameter domains, enabled when its effective precondition
  holds, times every post-state in its frame that satisfies its
  postcondition. ADR-018 SM-3 reads a behaviour as a maximal path of that
  relation.
- **A seeded sampler.** FR-101's `sample_request` draws one successor per
  step, uniformly over the successors in canonical order, with the
  `quire.simulation.sampler/v1` generator: each draw is SHA-256 over
  `{"draw","seed","step","trace"}`, accepted by rejection so the selection is
  exactly uniform. A sampled trace is identified by
  `SampleProvenance{seed, trace, sampler}` (ADR-014 TR-1, TR-6) and replays
  without the sampler (FR-101-AC-5).
- **Quantities.** QSpec FR-142 gives exact dimensions, units and conversions;
  comparison needs equal dimensions and an explicit common unit.
- **Verdict vocabulary.** ADR-013 O-16's eight categories, QSpec FR-331's
  result values (`proved`, `refuted`, `tested`, `inconclusive`,
  `unsupported`, `declined`, `incomplete`, `failed`), and ADR-018's
  `ProofBasis` on `TerminalValue::Proved`. O-16 already keeps `tested` in the
  success category with the rule that it never counts as proof evidence.
- **Monitors.** tl-mltl evaluates Boolean MLTL over bounded finite traces and
  lists probabilistic semantics as out of its scope. QSpec FR-124 marks
  probabilistic FRETish as a non-preserved mapping.

The question has two halves: what a probabilistic model and property mean,
and how a verdict states that a sampled result is a measurement and not a
proof.

## Decision

### 1. Probabilistic choice in models

A model becomes probabilistic through two declarations: **random parameters**
on operations, which resolve choice by probability, and a **workload**, which
resolves the scheduler's choice of operation by weight. Together with the
model they define a Markov decision process; with a workload chosen they
define a Markov chain.

| ID | Rule |
| --- | --- |
| PM-1 | **Random parameters.** An operation parameter may carry a distribution: a finite list of (value, weight) pairs, each weight a positive exact rational, the values distinct and of the parameter's type. The probability of a value is its weight over the sum of the weights. The value set is the parameter's **support**, and the support is its finite domain for every purpose of FR-120: enumeration, the requires-bound pre-check and the transition identity. A random parameter is read by the operation's postconditions and rewards (PM-5) only; its preconditions read none, so enabledness is decided before any draw. Spelling is QSpec's (QS-1). |
| PM-2 | **Workloads.** A workload is a declaration on a model that gives every operation of the model a positive exact rational weight. A **scheduled identity** is a transition identity without its random arguments: (operation, receiver, non-random argument vector). Under a workload, at a state `s` with enabled operations `O(s)`, each operation `o` in `O(s)` is chosen with probability `w(o) / Σ w(O(s))`, and then each of its enabled scheduled identities `E_o(s)` with probability `1 / |E_o(s)|`. The random arguments are then drawn independently from their distributions in declared order. The workload is a memoryless randomized scheduler. Spelling is QSpec's (QS-2). |
| PM-3 | **The Markov condition.** For a scheduled identity and a drawn random argument vector, the model must give exactly one post-state. When it gives several (residual nondeterminism) or none, the subject is not a Markov chain under the workload at that state, and the item settles `unsupported`, `NotMarkov{state, transition, post_states}` (SV-5). The step probability of a transition is the product of PM-2's scheduler probability and the probabilities of its random arguments. |
| PM-4 | **Behaviours and their measure.** A behaviour is still a maximal path from an initial state (ADR-018 SM-3), with SM-4's terminal stutter step. For an initial state `s0` and a workload `W`, the step probabilities define the probability measure `Pr[s0, W]` on behaviours: a finite prefix has the product of its step probabilities, extended to measurable sets of behaviours in the usual way for Markov chains. A probabilistic claim holds for a subject when it holds under `Pr[s0, W]` for every initial state `s0` of the subject and for every binding of its `over` parameter (ADR-018 QS-5). The subject has no distribution over its initial states. |
| PM-5 | **Rewards.** An operation may declare named rewards, each a non-negative value of an integer or quantity type (QSpec FR-142), computed from the operation's parameters alone, random ones included. A transition carries its operation's reward values; the terminal stutter step carries zero. A reward reads no state, so it adds nothing to the state key and the subject stays finite. `duration`, a reward of a time dimension, is the usual case. Spelling is QSpec's (QS-3). |
| PM-6 | **One graph for both readings.** The support graph of the Markov chain is ADR-018's state graph: random parameters range over their supports and every operation has positive weight, so every transition of the state graph has positive probability under every workload. Two consequences follow. Every ADR-018 claim (TP-1 to TP-4) reads a probabilistic model as a nondeterministic one and ignores weights. A claim ADR-018 proves holds on every behaviour of positive probability under every workload. |
| PM-7 | **Fairness holds almost surely.** In a finite Markov chain where every enabled transition has positive probability, a behaviour reaches a bottom strongly connected component with probability 1 and then takes every transition of it infinitely often. Every ADR-018 and ADR-019 fairness constraint, weak or strong, `whole` or `each`, therefore holds with probability 1 under every workload. A probabilistic claim carries no fairness set; its premise is its workload. |
| PM-8 | **The claim names its scheduler.** A probabilistic claim states `under W` for a workload `W`, and statistical checking (§3) settles it under that workload. The workload is part of the claim, so it is part of the obligation identity (ADR-013 O-09) and of the result's scope. A claim may instead state `under every scheduler`: it then quantifies over every scheduler of the Markov decision process. Statistical checking settles that form `unsupported` (SV-5); it is the seam for exact probabilistic checking (SV-9). |

### 2. Property forms

A probabilistic claim compares a probability, a quantile, the mean of a
fraction or a long-run fraction with a threshold, at confidence parameters it
states (PF-10). Every form is built from an **event**
or a **measure** over one behaviour.

| ID | Rule |
| --- | --- |
| PF-1 | **Events.** An event is a Boolean function of a behaviour that a finite prefix decides. It is one of: a formula under a bounded profile with activation `on origin` (ADR-018 TP-2), whose truth depends on the first `h + 1` positions, `h` its horizon (ADR-014 TR-4); or a comparison `M <= c` or `M >= c` of a bounded measure (PF-2) with a threshold `c` of the measure's type. Interval operators nest in any order inside the bounded formula (QSpec FR-091, FR-092). |
| PF-2 | **Bounded measures.** A measure is a value computed from one behaviour over a finite window `[0, h]` of positions. **Accumulate**: `accumulate R from holds(A) until holds(B) within h` is the sum of reward `R` over the steps from the first position `a <= h` where `A` holds to the first position `b`, `a <= b <= h`, where `B` holds (the steps into positions `a + 1` to `b`). A behaviour with no such `a` is not activated; one with `a` and no such `b` is censored, and its value is `+∞`. **Steps**: `steps from holds(A) until holds(B) within h` is accumulate with reward 1 per step. **Fraction**: `fraction holds(P) over [0, h]` is the number of positions in `[0, h]` where `P` holds over `h + 1`; with `weighted by R` each position `i` counts with the reward of the step that leaves it, and the value is the weighted sum over the total weight. A window reaching a terminal state continues on its stutter steps (ADR-018 SM-4). |
| PF-3 | **Probability bound.** `probability >= θ [E]` or `probability <= θ [E]` for an event `E` and an exact rational `θ` with `0 < θ < 1`. It holds when `Pr[s0, W]({behaviours where E is true}) >= θ` (or `<= θ`) for every `s0` and binding (PM-4). The event's truth on a behaviour is SM-1's value (ADR-018 SM-1). |
| PF-4 | **Quantile.** `quantile q of M <= c` or `quantile q of M >= c` for a bounded accumulate or steps measure `M`, an exact rational `q` with `0 < q < 1` and a threshold `c` of `M`'s type. The distribution is that of `M` under `Pr[s0, W]` conditioned on activation. The `q`-quantile is `inf {x : Pr(M <= x) >= q}`. Since a distribution function is right-continuous, `quantile q of M <= c` holds exactly when `Pr(M <= c) >= q`, and `quantile q of M >= c` exactly when `Pr(M < c) < q`, a strict bound: `M` has finite support, so `Pr(M < c)` is attained at the largest support point below `c`, and with `M` equal to 1 or 3 with probability 1/2 each, `quantile 1/2 of M >= 3` is false (the 1/2-quantile is 1) while `Pr(M < 3) = 1/2`. Statistical tests decide the strict bound as `<= q`, since the boundary lies inside the indifference region. Each quantile claim is therefore checked as a probability bound on an event conditioned on activation. A censored behaviour counts as exceeding every finite threshold. |
| PF-5 | **Mean of a fraction.** `mean of F >= θ` or `<= θ` for a fraction measure `F` and an exact rational `θ` with `0 < θ < 1`. It holds when the expectation of `F` under `Pr[s0, W]` meets the bound for every `s0` and binding. Windowed availability is this form or a probability bound on `F >= θ'`. A mean is stated over a fraction only; a latency is stated as a quantile (PF-4). |
| PF-6 | **Long-run fraction.** `long-run fraction holds(P) >= θ` or `<= θ`, optionally `weighted by R`, for `0 < θ < 1`. On a behaviour, the long-run fraction is the limit as `n → ∞` of the fraction over `[0, n]` (PF-2). In a finite Markov chain the limit exists with probability 1 and is constant on each bottom strongly connected component. The claim holds when the expectation of the limit under `Pr[s0, W]` meets the bound. This is the steady-state reading of long-run availability. Statistical checking regenerates at returns to `s0` (ST-7). |
| PF-7 | **Units.** A probability, a quantile level, a fraction and a mean of a fraction are dimensionless exact rationals. A threshold of an accumulate measure has the reward's dimension; comparing it with a value in another unit of that dimension follows QSpec FR-142, so the claim converts explicitly. Every estimate in a result is an exact rational, in the threshold's unit where it has one. |
| PF-8 | **Bounded and qualitative forms.** Every event and bounded measure is decided on a finite window, so one sample decides it. An interval operator under an unbounded one (ADR-018 IV-1) has no finite window, and the bounded profile states the same property with a window: recovery stability becomes `probability >= 0.99 [eventually[0,100] always[0,10] holds(healthy)]`. A threshold of 0 or 1 is a qualitative claim about the support graph (PM-6): "every behaviour satisfies `E`" is an ADR-018 TP-2 claim, and "some behaviour reaches `p` within `h`" is a possible property (ADR-022). Both get ADR-018's exact verdicts. |
| PF-9 | **Claim kind.** A probabilistic claim requires the QSpec FR-290 kind `probabilistic-satisfaction` (QS-6). Its subject is a model subject (ADR-018 SM-2) with a workload. A probabilistic claim over a trace subject is outside this kind; observed statistics are window aggregates (§4). |
| PF-10 | **Confidence parameters.** A probabilistic claim may state three exact rationals: `α`, the greatest probability of rejecting the claim when its bound holds; `β`, the greatest probability of accepting it when its bound fails; and `ι`, the indifference half-width, with `0 < ι`, `θ − ι > 0` and `θ + ι < 1`. A decision carries its error guarantee when the true value lies outside `(θ − ι, θ + ι)`. When stated, the parameters are part of the claim and of its obligation identity (ADR-013 O-09), beside the workload (PM-8): one property stated at two confidences is two obligations. Every method of §3 reads them from the claim, and statistical evidence requires them: an item naming `statistical` evidence for a claim that states none settles `unsupported`, `MissingConfidence`, at negotiation (SV-5). A claim that states none is checked with exact evidence only (SV-9). |

### 3. Statistical model checking with QSL's simulator

| ID | Rule |
| --- | --- |
| ST-1 | **Engine EN-4.** Statistical model checking samples behaviours of the subject under the workload with FR-101's sampler, decides the claim's event or measure on each sample with the layer-5 evaluator (ADR-018 SM-1), and decides the claim from the counts with a statistical test. It lives in QSL layer A, crate `qsl-analyze`, module `statistical`, above the qualified core (ADR-029 CB-3), and runs in ADR-018's stage S6c over E10. It reads FR-120's `ModelSystem`, the sampler and the bounded evaluator from layer 5, `qsl-eval`, and nothing from EN-1. The verdict types, the witness replay arm and EN-4's settlement map live in layer 6, `qsl-replay`, as does every type CG reads. CG keeps the Kani C-09 map (ADR-013 C-09, ADR-011 T-13). QSL owns settlement only for its native engines, and that settlement lives in `qsl-replay`. EN-4 issues no `proved`: its results are `measured` (SV-1), so the rule that a proof with no core certificate checker settles `proved` labelled `uncertified` does not arise for it. |
| ST-2 | **Scheduler resolution while sampling.** At each step the sampler draws once for the scheduler (PM-2: operation by weight, then scheduled identity uniformly), then once per random parameter in declared order. Each draw selects by exact integer weights: rational weights are scaled by the least common multiple of their denominators. The sampler's preimage gains a choice index within the step, so draws within one step are independent. With unit weights and one choice per step the selection equals FR-101's uniform selection. This is a QSpec revision of `quire.simulation.sampler/v1` (QS-8). |
| ST-3 | **Tests.** The claim holds for every initial state and every `over` binding (PM-4), so EN-4 runs one statistical test per pair, `m` tests in all. Each test uses `α' = α/m` and `β' = β/m` (Bonferroni), so the family-wise errors stay within the claim's `α` and `β` whatever the dependence between tests. Symmetry reduces `m` (RX-1). |
| ST-4 | **Fixed-sample method (Okamoto).** For an event with true probability `p`, `N = ⌈ln(1/min(α', β')) / (2ι²)⌉` independent samples give an estimate `p̂` with `Pr(p̂ − p >= ι)` and `Pr(p − p̂ >= ι)` each at most `min(α', β')` (Okamoto's bound, a Chernoff–Hoeffding bound). The test reports the interval `[p̂ − ι, p̂ + ι]`. For a `>= θ` bound it is **accepted** when the interval lies at or above `θ` (wrong with probability at most `β'`), **rejected** when it lies below `θ` (wrong with probability at most `α'`), and **undecided** otherwise; `<= θ` is symmetric. The same bound with range 1 decides a mean of a fraction (PF-5), since a fraction lies in `[0, 1]` (Hoeffding). |
| ST-5 | **Sequential method (SPRT).** Wald's sequential probability ratio test, as Younes and Simmons apply it to model checking. For `>= θ` it tests `H0: p >= θ + ι` against `H1: p <= θ − ι`. Each sample adds `ln(p1/p0)` when the event holds and `ln((1 − p1)/(1 − p0))` when it does not, `p0 = θ + ι`, `p1 = θ − ι`. The test keeps two sums of rational bounds on these increments: a lower sum, each increment rounded down, and an upper sum, each rounded up. It accepts `H1` (**rejected**) only when the lower sum reaches `ln(1/α')` and `H0` (**accepted**) only when the upper sum reaches `ln(β')`, so it never decides earlier than with the true values, in either direction. With these thresholds Wald's inequalities bound the error probabilities by `α'` (rejecting when `p >= p0`) and `β'` (accepting when `p <= p1`) with no approximation; inside the indifference region either decision may come. The test ends with probability 1; the `max_samples` budget bounds its cost (ST-8). A `<= θ` bound swaps the roles. SPRT decides a probability bound or a quantile; a mean of a fraction, whose samples are not Bernoulli, is decided by ST-4. |
| ST-6 | **Activation.** A quantile claim (PF-4) is conditioned on activation. A sample that is not activated is drawn and not counted: it advances the trace index and counts toward the `max_draws` budget, never toward `N` or the SPRT sum. |
| ST-7 | **Regenerative method for long-run fractions.** For PF-6, EN-4 samples one long behaviour from `s0` and cuts it into regeneration cycles at each return to `s0` (equal state key). Each cycle contributes its weighted count `Y_k` of positions where `P` holds and its total weight `L_k`. The estimate is `Σ Y_k / Σ L_k`, and its interval half-width is `z · S / (L̄ · √n)` for `n` cycles, `S` the sample standard deviation of `Y_k − Â · L_k` and `z` the one-sided `1 − min(α', β')` normal quantile. The interval is asymptotic: its coverage approaches the stated level as `n` grows, by the central limit theorem for regenerative processes. The half-width uses `S² + 1/n` in place of `S²` (Chow and Robbins), so a degenerate sample with `S = 0` gives the half-width `z / (L̄ · n)`, never 0, and cannot stop the run early. The run stops once the half-width is at most `ι` and at least `min_cycles` cycles are complete, and decides as ST-4. The result's coverage is `Asymptotic`. A run that never returns to `s0`, such as one from a transient `s0`, ends its test when a cycle reaches `max_cycle_steps` steps, Undecided with cause `NoRegeneration{max_cycle_steps}`, which names the setting, its value and the request member that raises it. |
| ST-8 | **Method and budgets.** The method (Okamoto, SPRT or regenerative) is a request setting, and so is the regenerative method's `min_cycles`; neither enters the obligation identity, and both are recorded in the result (SV-2). EN-4's budgets are `StatisticalLimits{max_samples, max_draws, max_cycle_steps}`, ADR-014 B-5 budgets of QSL's own provider like ADR-018's `ModelCheckLimits`, each set by the request with a published default, plus time, the clause meter and cancellation. No budget is a modelling limit: reaching one stops the run (SV-4), and the caller raises it and reruns. The number of sampled witnesses kept, `max_witnesses`, is a request setting. |
| ST-9 | **Exact arithmetic.** Counts, estimates, interval ends and thresholds are exact rationals. Every logarithm, square root and normal quantile in ST-4 to ST-7 is replaced by a rational bound computed by QSpec's method to a fixed precision and rounded in the conservative direction: `N` rounds up, SPRT thresholds round away from zero, SPRT increments round into the lower and upper sums of ST-5, and interval half-widths round up. The decision is a function of the counts alone, identical on every platform (QS-9). |
| ST-10 | **Sample stops.** A sample whose expansion returns `ExpansionStop` (FR-101) stops the run with that cause. A contract conjunction evaluated undecided during a step (FR-120 `ContractUndetermined`) settles **undecided**, `UndecidedSuccessor` (ADR-018 V-6). A step that breaks PM-3 settles `NotMarkov`. |

Okamoto's sample count depends only on the claim's `α`, `β` and `ι`; SPRT's depends on how far
the true probability is from `θ`, and is far smaller when it is not close
(§7). Okamoto gives an interval; SPRT gives a decision with bounded errors.
The request picks one.

### 4. Window aggregates in monitors

A live system yields one observed trace. A statistic over a window of it is a
deterministic function of the trace, so a monitor of "p95 over the last five
minutes" needs no probability: it needs aggregate atoms.

| ID | Rule |
| --- | --- |
| WA-1 | **Aggregate terms.** An aggregate term reads values from the positions in a past window of the current position: `count`, `sum`, `min`, `max`, `fraction holds(P)` and `quantile q`, each over a value expression and an optional filter predicate. Example (illustrative spelling, QS-10): `quantile 0.95 of e.latency where holds(e is Response) over past[0, 300 s]`. An aggregate term compares with a threshold to form an atom, and the atom enters any bounded-profile formula. Aggregate terms are admitted over trace subjects; over a model subject the measures of §2 serve. |
| WA-2 | **Windows.** Under event-position false-extension, `past[a, b]` holds the positions `j` with `i − b <= j <= i − a` and `j >= 0`. Under timestamped-event finite-window (QSpec FR-090), it holds the events whose timestamp `t_j` satisfies `t_i − b <= t_j <= t_i − a`, with `a` and `b` durations. A window never reads a future position, so an aggregate atom needs no lookahead and adds nothing to the formula's horizon (ADR-014 TR-4). |
| WA-3 | **Values.** Values are exact: integers, exact rationals and quantities with QSpec FR-142's dimension rules. `quantile q` is the nearest-rank quantile, the `⌈q · n⌉`-th smallest of the `n` values in the window. Comparisons with the threshold follow FR-142. |
| WA-4 | **Insufficient data.** An aggregate term declares `min_count >= 1`. A window holding fewer values evaluates the atom to the kernel's `Undefined`, with cause `InsufficientData{count, min_count}`, so the clause at that position is O-16 undefined, the existing outcome for an atom with no value, never `refuted` and never `UndefinedEvaluation`, as QSpec FR-414 states. |
| WA-5 | **Memory.** The evaluator keeps the window's values. `max_window_values` is an ADR-014 B-2 run limit that the caller sets per run, with a published default; a window that exceeds it yields `Incomplete`, never a truth value. Evaluation charges TR-5 work per value visited. |
| WA-6 | **Meaning.** The truth of an aggregate atom at a position is a deterministic function of the trace. A monitor result over an observed trace reads by ADR-014 A-4 for a finite trace: a violation is a violation of the claim on that trace, and a holding result is `tested`, evidence for that trace only. The result carries evidence kind `Observed{trace, positions}` (SV-1). |
| WA-7 | **Relation to tl-mltl.** An aggregate atom is evaluated at atom evaluation, before temporal evaluation, as `holds` atoms are. The temporal layer stays Boolean MLTL over derived Boolean signals, so tl-mltl consumes the derived signal unchanged and stays free of probabilistic semantics. QSL's layer-5 evaluator defines the meaning (ADR-018 SM-1); the observation layer computes the derived signal for a live system (References). |
| WA-8 | **Relation to model claims.** A model quantile (PF-4) is over behaviours under a workload, one observation per behaviour. A window quantile is over events of one observed trace. A monitor with the same threshold as a model claim checks that the running system stays within the model's assumption; its violation is evidence that the workload does not describe the live system, and changes no verdict of the model claim. |

### 5. Verdicts: a measurement is never a proof

| ID | Rule |
| --- | --- |
| SV-1 | **A distinct verdict family.** A statistical result is a measurement with confidence bounds. It settles as the new FR-331 result value `measured`, carried by a new `TerminalValue::Measured(StatisticalVerdict)`. It is never `proved` and never `refuted`: it is not V-1 to V-4 of ADR-018, its record carries no `ProofBasis`, and proof accounting, which reads result values, never counts it. `ProofBasis` is not extended, because every `ProofBasis` lives on `Proved`, and `Proved` counts as proof. |
| SV-2 | **Content.** `StatisticalVerdict{decision, estimate, interval, basis, provenance, witnesses}`. `decision` is `Accepted`, `Rejected` or `Undecided(cause)`. `estimate` is the exact rational point estimate: `p̂` for a probability, the conditional probability `Pr(M <= c)` and the empirical nearest-rank quantile for a quantile claim, the sample mean of a fraction, or the regenerative ratio. `interval` is the confidence interval for ST-4 and ST-7 and absent for ST-5. `basis` is `StatisticalBasis{method, confidence, samples, draws, tests, coverage}`: the method (`Okamoto`, `Sprt`, `Regenerative{min_cycles}`), the claim's `α`, `β` and `ι`, the samples counted, the draws taken, the test count `m` with the per-test `α'` and `β'`, and `coverage` `FiniteSample` (ST-4, ST-5) or `Asymptotic` (ST-7). `provenance` is RP-1's. The result keeps one entry per test with its own decision and estimate. |
**SV-3 Decisions and categories.**

| Decision | Meaning | FR-331 value | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- |
| Accepted | Every test accepted, each with its method's error guarantee (ST-4, ST-5, ST-7). Verdict "measured: accepted" | `measured` | `Measured{decision: Accepted, …}` | success, never proof evidence, as `tested` |
| Rejected | At least one test rejected, with its method's error guarantee, or a sample evaluated undefined (SV-11). Verdict "measured: rejected". It fails the pipeline exactly as an O-16 violation does | `measured` | `Measured{decision: Rejected, …}` | violation, never `refuted` |
| Undecided | Neither: the interval contains the threshold (`IndifferenceRegion`), a step's contract evaluated undecided (`UndecidedSuccessor`), or the run did not regenerate (`NoRegeneration`, ST-7). Verdict "measured: undecided" | `measured` | `Measured{decision: Undecided(cause), …}` | inconclusive |

| ID | Rule |
| --- | --- |
| SV-4 | **Stopped runs.** A budget (ST-8), other than `max_cycle_steps`, which ends a regenerative test `NoRegeneration` (ST-7), or an `ExpansionStop` stops the run before its method completes: `Incomplete(cause)` naming the budget, O-16 incomplete, as ADR-018 V-7. Undecided is kept for a run that completed its method. |
| SV-5 | **Unsupported.** `NotMarkov` (PM-3), `under every scheduler` (`EveryScheduler`, PM-8), a claim that states no confidence parameters (`MissingConfidence`, PF-10), and a requested evidence kind no candidate advertises (SV-9) settle `Unsupported(cause)`, O-16 unsupported. A request naming a method the claim's form does not admit (SPRT or regeneration for a mean of a fraction, Okamoto or SPRT for a long-run fraction, regeneration for any other form) is refused `NotStatistical`. |
| SV-6 | **Counterexamples are evidence.** A **sampled witness** is a sample on the violating side of the claim's bound: for a `>= θ` bound over an event `E`, a sample on which `E` is false; for a `<= θ` bound, a sample on which `E` is true; for a quantile claim, the same for its PF-4 event; for a mean of a fraction, a sample whose fraction lies below `θ` (`>= θ`) or above it (`<= θ`). A long-run fraction keeps no witness, since no finite sample refutes a limit. A witness carries the test index, the trace index, the steps in ADR-018 CX-2's content, the drawn values, the path probability (the exact product of its step probabilities) and the event's value. The result keeps up to `max_witnesses` of them, the first in trace-index order. A sampled witness never refutes a probability bound: with `θ < 1` a bound allows behaviours where the event is false. |
| SV-7 | **A witness refutes the qualitative claim.** A sampled witness is a behaviour of positive probability on the violating side of its bound, so it is a valid ADR-018 CX-1 counterexample for the qualitative claim of its bound direction over the same subject: "`E` on every behaviour" for a `>= θ` bound, "not `E` on every behaviour" for a `<= θ` bound, and the fraction comparison on every behaviour for a mean of a fraction. It replays through ADR-018 CX-3 as one. |
| SV-8 | **Witness replay.** A sampled witness replays by ADR-018 CX-3 (re-execution through `ModelSystem`, post-state digests, SM-1 evaluation), plus two checks: each random argument is in its support, and each draw recomputed from the seed, trace index, step and choice index selects the recorded scheduled identity and values. Agreement settles the witness reproduced; disagreement settles `inconclusive`, `ReplayParity`, for that witness. The item's value is unchanged either way. |
| SV-9 | **The exact seam.** A request item for `probabilistic-satisfaction` names the evidence it accepts: `statistical` with a method (§3) or `exact`. A provider advertises the evidence kinds it produces, and negotiation routes an item only to a candidate advertising its kind and never substitutes one for the other. EN-4 advertises `statistical`. An exact backend computes the probability over the full finite subject, for a workload or for every scheduler (PM-8), and settles with ADR-018's `proved` and `refuted`, `closed-scope`, under a `ProofBasis` member that its own record defines. With no exact candidate, an `exact` item settles `unsupported-requested-capability` at negotiation. |
| SV-10 | **A separate `measured` axis.** A statistical result is not a truth result. It carries QSpec FR-241's execution disposition (`completed` for every decision) and its own `measured` axis with the closed values `accepted`, `rejected` and `undecided`; it carries no FR-242 truth and no FR-243 settlement basis. No reader can therefore take a measurement for `satisfied` or `violated`, and FR-242's and FR-243's vocabularies keep their meaning for proof and refutation alone. A stopped or unsupported statistical run carries execution `resource-incomplete`, `failed` or `unsupported` and no `measured` value, as FR-241 states for any analysis. |
| SV-11 | **A sampled undefined is a rejection.** ADR-018 UE-1 to UE-6 apply to every sample. When the event, the measure or its comparison evaluates undefined on a sample, the claim fails on that sample: its test decides Rejected at once, the run stops, and the claim is Rejected (SV-3), as QSpec FR-408 states. The test result carries `UndefinedEvaluation{where, cause}` with QSpec FR-408's `where` and `cause`. The sample is kept as a sampled witness (SV-6) and replays by SV-8, with ADR-018 UE-5 in place of the event's evaluation. It refutes the qualitative claim by SV-7. |

**Reproducibility.**

| ID | Rule |
| --- | --- |
| RP-1 | **Provenance.** `provenance` is the seed, the sampler identity, and for each test its index, initial state, binding and trace-index range. Sample `i` of test `j` uses trace index `i · m + j` and starts from test `j`'s initial state, with no draw selecting the start. |
| RP-2 | **Deterministic replay of the run.** Re-running EN-4 with the same subject, claim (its workload and confidence parameters included), method, `min_cycles`, seed and sampler reproduces every sample, every count and the decision, by FR-101's determinism (FR-101-AC-4) and ST-9's exact arithmetic. That re-run is the replay of a statistical result; each sampled witness also replays alone (SV-8). |
| RP-3 | **Independence of samples.** Distinct trace indices give independent draws under the sampler's hash, which is the independence ST-4 and ST-5 assume. The seed is a request member; a request that reruns with a new seed gets a new, independent measurement of the same obligation. |

### 6. Reductions

ADR-021's reductions change which product states EN-1 stores. EN-4 stores no
states, so POR and state constraints have nothing to reduce. Symmetry still
helps: it reduces the number of tests.

| ID | Rule |
| --- | --- |
| RX-1 | **Symmetry reduces tests.** Under an admitted symmetry declaration (ADR-021 SYM-5), when the workload and every random parameter's distribution are invariant under the group `G`, every permutation in `G` preserves step probabilities, so `Pr[π(s0), W]` of a symmetric event under binding `π(o)` equals `Pr[s0, W]` under binding `o`. Workload weights are per operation, so they are invariant; a random parameter of a reference type into a symmetric population is invariant when its weights are equal within each class. EN-4 then runs one test per orbit of (initial state, binding) pairs, the least in key order, and `m` counts orbits. The verdict and the samples per test are unchanged; a smaller `m` loosens each test's `α'` and `β'`. |
**RX-2 Proposed PT-2 row.** ADR-021 PT-2 gains:

| Form | Symmetry (admitted, SYM-5) | POR (C0–C3, POR-6) | State constraint |
| --- | --- | --- | --- |
| Probabilistic claim, statistical (ADR-024 EN-4) | yes, when the workload and every random-parameter distribution are invariant under `G` (RX-1); reduces the test count | no: EN-4 stores no states, and a reduced successor set changes step probabilities | no: a cut changes the measure |

### 7. Worked examples

The spellings are illustrative; QSpec's shared grammar owns them (QS-1 to
QS-5). Each subject has one initial state and no `over` parameter, so
`m = 1`. Every claim states `α = β = 0.01`; the method is the request's. SPRT sample counts are Wald's
approximate expected values, which ignore overshoot.

#### 7.1 Probability bound: no fault in 1,000 steps

```text
profile ev = "quire.temporal.event-position.false-extension/v1" …;
model Health = "example/health" …;     // object n: Node { healthy: Bool }, initially healthy

operation Node::tick(random fault: Bool ~ { true: 1, false: 9999999 })
  post self.healthy = (pre(self.healthy) and not fault);

workload Steady on Health { weight Node::tick = 1; }

probabilistic NoFault using ev on Health under Steady on origin
    with alpha 0.01, beta 0.01, indifference 0.0005 {
  probability >= 0.999 [ always[0,1000] holds(n.healthy) ]
}
```

The event is TP-2 with `h = 1000`, so each sample is 1,001 positions. The
exact probability is `(1 − 10⁻⁷)^1000 ≈ 0.9999000`.

- **Okamoto**: `N = ⌈ln(100) / (2 · 0.0005²)⌉ = 9,210,341` samples. The
  interval `p̂ ± 0.0005` lies above 0.999, so the item settles "measured:
  accepted", wrong with probability at most 0.01.
- **SPRT** (`H0: p >= 0.9995`, `H1: p <= 0.9985`): a holding
  sample adds `ln(0.9985/0.9995) ≈ −0.0010010`, a failing one adds
  `ln 3 ≈ 1.0986`, and the thresholds are `±ln 100 ≈ ±4.6052`. With no failing
  sample the test accepts after 4,601 samples; each failing sample costs about
  1,098 more. At `p ≈ 0.9999` the expected count is about 5,168 samples.
  Settles `measured`, Accepted.

`always holds(n.healthy)` as an ADR-018 TP-1 claim is refuted by EN-1: the
support graph contains the faulting step. Any failing sample is that
counterexample (SV-7).

#### 7.2 Quantile: p95 latency under 5 ms

```text
model Service = "example/service" …;   // object s: Server { phase: Phase, attempts: Int[0, 2] }

operation Server::request
  pre  self.phase = Idle
  post self.phase = Busy and self.attempts = 0;
operation Server::attempt(random d: Quantity<Time> ~ { 1 ms: 90, 3 ms: 7, 12 ms: 3 },
                          random outcome: Outcome ~ { Ok: 98, Fail: 2 })
  pre  self.phase = Busy
  post self.attempts = pre(self.attempts) + 1 and
       self.phase = (if outcome = Ok or pre(self.attempts) = 1 then Done else Busy)
  reward duration = d;
operation Server::reset
  pre  self.phase = Done
  post self.phase = Idle;

workload Steady on Service { weight Server::request = 1; weight Server::attempt = 1; weight Server::reset = 1; }

probabilistic P95 using ev on Service under Steady on origin
    with alpha 0.01, beta 0.01, indifference 0.01 {
  quantile 0.95 of (accumulate duration from holds(s.phase = Busy) until holds(s.phase = Done) within 10) <= 5 ms
}
```

One operation is enabled at each state, so the workload's weights decide
nothing here; PM-3 holds because each drawn `(d, outcome)` gives one
post-state. Activation is at position 1. A request succeeds on the first
attempt with probability 0.98; otherwise a second attempt follows and the
latency is the sum of two durations.

- **Exact distribution.** `Pr(M <= 1 ms) = 0.882`, `Pr(M <= 2 ms) = 0.8982`,
  `Pr(M <= 3 ms) = 0.9668`, `Pr(M <= 5 ms) = 0.98 · 0.97 + 0.02 · 0.936 =
  0.96932`. The p95 is 3 ms. By PF-4 the claim is `probability >= 0.95
  [M <= 5 ms]`, true with margin 0.019.
- **Okamoto**: `N = ⌈ln(100) / (2 · 0.01²)⌉ = 23,026` samples,
  each at most 11 positions. The interval about 0.9693 ± 0.01 lies above
  0.95: Accepted, with estimate `Pr(M <= 5 ms) ≈ 0.969` and empirical p95
  3 ms.
- **SPRT** (`H0: p >= 0.96`, `H1: p <= 0.94`): a holding sample
  adds `≈ −0.02105`, a failing one `≈ 0.4055`. With no failing sample it
  accepts after 219 samples; at `p = 0.96932` it needs about 578. Accepted.
- **A tighter threshold.** At `<= 2 ms` the event probability is 0.8982,
  below 0.94, and SPRT settles "measured: rejected": the pipeline fails as
  on an O-16 violation, and the value is never `refuted`. A sample with latency 3 ms is a sampled witness (SV-6).

#### 7.3 Availability: long-run and per window

```text
model Avail = "example/avail" …;       // object v: Server { up: Bool }, initially up

operation Server::tick(random fail: Bool ~ { true: 1, false: 1999 })
  pre  self.up
  post self.up = not fail;
operation Server::repair(random ok: Bool ~ { true: 9, false: 1 })
  pre  not self.up
  post self.up = ok;

workload Steady on Avail { weight Server::tick = 1; weight Server::repair = 1; }

probabilistic LongRun using ev on Avail under Steady on origin
    with alpha 0.01, beta 0.01, indifference 0.0002 {
  long-run fraction holds(v.up) >= 0.999
}
probabilistic Monthly using ev on Avail under Steady on origin
    with alpha 0.01, beta 0.01, indifference 0.005 {
  probability >= 0.99 [ fraction holds(v.up) over [0, 10000] >= 0.999 ]
}
```

The chain fails with probability `f = 0.0005` per step and repairs with
probability `r = 0.9`.

- **Long-run.** The exact availability is `r / (r + f) = 0.9 / 0.9005 ≈
  0.999445`, so `LongRun` holds. ST-7 regenerates at each return to `up`. The
  cycle length `L` is 1 with probability `1 − f` and `1 + G` otherwise, `G`
  geometric with mean `1/r`, and each cycle has `Y = 1`. The asymptotic
  variance of the ratio is `Â² · Var(L) / E[L]² ≈ 0.000677`. With the claim's
  `ι = 0.0002` and `z ≈ 2.3263` (one-sided 0.99), the half-width with
  `S² + 1/n` reaches `ι` at the `n` that solves `n = K · 0.000678 + K / n`,
  `K = z² / (ι² · L̄²) ≈ 1.351 · 10⁸`: about 93,080 cycles, about 93,130
  steps. The interval
  about 0.99944 ± 0.0002 lies above 0.999: "measured: accepted",
  `coverage: Asymptotic`. The first 100 cycles have no failure with
  probability about 0.95, and then `S = 0`; the `S² + 1/n` term keeps the
  half-width at `z / (L̄ · n)`, so the run cannot stop before about 11,600
  cycles on a degenerate sample. The claim `long-run fraction holds(v.up)
  <= 0.9995` with the same confidence parameters therefore settles
  "measured: undecided", `IndifferenceRegion`: its interval about
  0.99944 ± 0.0002 contains 0.9995, which lies within `ι` of the exact
  value. It is never Rejected on an interval `[1, 1]`.
- **Per window.** A 10,001-position window has `fraction >= 0.999` exactly
  when at most 10 positions are down. Its probability is about 0.9590, below
  0.99, so `Monthly` fails although the long-run availability passes. SPRT
  (`H0: p >= 0.995`, `H1: p <= 0.985`) rejects after five failing windows in
  a row at the least, and needs about 128 windows at `p ≈ 0.959`: "measured:
  rejected", finite-sample coverage. Okamoto would need 92,104 windows.

The two claims differ by design: the long-run claim reads the mean over all
time, the window claim reads how often a fixed window meets the target.

### 8. Downstream impact and sequencing

| ID | Repository | Change |
| --- | --- | --- |
| DS-1 | QSL | S3: random parameters (PM-1) with the read restriction, workloads (PM-2) with weight completeness, rewards (PM-5), probabilistic claim forms (§2) with their classification, events and measures, aggregate terms (WA-1 to WA-4). FR-120: random parameters as support domains; the step probability. FR-101: weighted selection and the choice index (ST-2). S3: confidence parameters in the claim and its identity (PF-10). Layer 5 `statistical`: EN-4 with the three methods, Bonferroni, exact arithmetic, `StatisticalLimits`, sampled witnesses. Layer-5 evaluator: aggregate atoms with `max_window_values`. `qsl-replay`: `TerminalValue::Measured`, `StatisticalVerdict`, the sampled witness and its replay checks (SV-8). The EN-4 provider manifest with its evidence kind. |
| DS-2 | CG | A `negotiate_*` arm for `probabilistic-satisfaction` that routes by evidence kind and never substitutes (SV-9); the map into `TerminalValue::Measured`. |
| DS-3 | Driver | Runs an item routed to EN-4 in process and writes its terminal record. |
| DS-4 | Observation | Computes aggregate atoms as derived signals for live traces (WA-7). |
| DS-5 | QSpec | §9. |

**Sequencing.** EN-4 depends on FR-120 `ModelSystem`, the bounded layer-5
evaluator from the temporal spine migration, and FR-101 sampling, all already
planned for ADR-018 (its §7 steps 2 and 3). It does not depend on EN-1, so it can follow those
prerequisites in parallel with EN-1. Window aggregates depend only on the
temporal spine. Exact probabilistic checking follows its research (SV-9).

### 9. What QSpec must specify

| ID | Item | Where |
| --- | --- | --- |
| QS-1 | Random parameters (PM-1): syntax, weights as positive exact rationals, support as the parameter's domain, the read restriction, and the support in the FR-181 successor relation | shared grammar; QSpec FR-405, FR-013, FR-181 |
| QS-2 | Workloads (PM-2): declaration syntax, one weight per operation, the scheduling rule, `under W` and `under every scheduler` on a claim (PM-8), and the workload in the obligation identity | shared grammar; QSpec FR-405, FR-406 |
| QS-3 | Rewards (PM-5): syntax, non-negativity, parameter-only reads, integer or quantity type | shared grammar; QSpec FR-405, FR-142 |
| QS-4 | The probabilistic semantics: step probability, the Markov condition and `NotMarkov` (PM-3), the measure per initial state and binding (PM-4), the support graph (PM-6), almost-sure fairness (PM-7) | QSpec FR-406 |
| QS-5 | Property forms (§2): events, bounded measures with activation and censoring, probability bounds, quantiles and their reduction, mean of a fraction, long-run fractions, units, thresholds strictly between 0 and 1, and the confidence parameters `α`, `β`, `ι` in the claim and its obligation identity (PF-10) | QSpec FR-407, FR-090, FR-142 |
| QS-6 | FR-290 kind `probabilistic-satisfaction`; the advertised evidence kinds `statistical` and `exact`; the request item's evidence kind, method and `StatisticalLimits`; the rule that negotiation never substitutes one evidence kind for the other | QSpec FR-290, FR-331, FR-408 |
| QS-7 | Results: the FR-331 result value `measured` and its members (SV-2), carried as SV-10 states | QSpec FR-409, FR-410, FR-331 |
| QS-8 | Sampler revision: weighted selection by exact integer weights with rejection, the choice index in the preimage, the trace-index assignment `i · m + j` (RP-1), and vectors including the uniform case equal to TC-210's | `quire.simulation.sampler/v1` definition; QSpec TC-210 |
| QS-9 | Statistical methods over the claim's `α`, `β`, `ι`: Okamoto's `N`, SPRT with the thresholds `ln(1/α')` and `ln(β')`, Bonferroni over tests, the regenerative estimator and its asymptotic interval, and the method that computes rational bounds for logarithms, square roots and normal quantiles with conservative rounding (ST-9) | QSpec FR-408 |
| QS-10 | Window aggregates (§4): grammar, windows under event-position and timestamped-event profiles, nearest-rank quantile, `min_count` and `InsufficientData` as `Undefined`, `max_window_values` | QSpec FR-414, FR-090, FR-091, FR-092 and the shared grammar |
| QS-11 | The sampled witness wire (SV-6) and its replay checks (SV-8) | QSpec FR-410 and the counterexample contract |
| QS-12 | Conformance vectors, each with a fixed seed so the decision and sample count are exact: (a) §7.1, its exact probability and both methods' decisions; (b) §7.2, the exact distribution function at 1, 2, 3 and 5 ms, Accepted at 5 ms, Rejected at 2 ms, and a sampled witness that replays; (c) §7.3, Accepted long-run with `Asymptotic` coverage and Rejected per window; (d) a model with residual nondeterminism settling `NotMarkov`; (e) two initial states with Bonferroni, and a symmetric pair reduced to one test (RX-1); (f) weighted sampler vectors; (g) aggregate monitors: nearest-rank quantiles, `InsufficientData`, a unit conversion, `max_window_values`; (h) witness replay refusals: a value outside the support, a draw that selects another identity | QSpec TC-350 to TC-359 and TC-366, beside TC-200 and TC-210 |

### 10. Rulings on the draft's questions

The owner ruled on the five questions the draft left open, on 2026-10-01, and on the confidence parameters (RU-6).

| ID | Question | Ruling | Rationale | Where it lands |
| --- | --- | --- | --- | --- |
| RU-1 | Whether long-run fractions, checked by regeneration with asymptotic coverage, are admitted | **Admitted.** They are checked by the regenerative method, and the `measured` verdict labels their coverage `Asymptotic` | Long-run availability is the steady-state statement users mean by "available 99.9% of the time"; the label keeps its weaker guarantee visible beside finite-sample results | PF-6, ST-7, SV-2 |
| RU-2 | Whether a Rejected decision fails the pipeline | **Rejected fails the pipeline,** gating as an O-16 violation does. The verdict is "measured: rejected", never `refuted`. Undecided is inconclusive | A bound the measurement rejects at the claim's error rates is a failed requirement for anyone running the check; keeping the value `measured` keeps it out of refutation accounting | SV-3; ADR-013 O-16, amended with this record |
| RU-3 | Whether the mean of an accumulated measure (mean latency) is a form | **Dropped.** Latency is stated as a quantile; the mean of a fraction stays | A censored sample makes a mean `+∞`, and a quantile states the latency requirement without it | PF-5, WA-1 |
| RU-4 | Where the confidence parameters live | **In the claim and its obligation identity:** `α`, `β` and the indifference half-width `ι`. The method (Okamoto, SPRT, regenerative) is a request setting | The confidence is part of what the claim asserts; the method is how a run meets it, and any method meets the same parameters | PF-10, ST-8, §7 |
| RU-5 | How QSpec encodes a statistical result | **A separate `measured` axis,** not a new FR-243 basis | FR-242 and FR-243 describe Boolean truth and its settlement; a measurement is neither, and its own axis keeps both vocabularies exact | SV-10 |
| RU-6 | Whether a claim may omit its confidence parameters (ruled 2026-10-01) | **Confidence parameters are for statistical evidence only.** When a claim states them they stay in the claim and its obligation identity (RU-4); a claim that states none is exact-only, and statistical checking refuses it | An exact verdict decides the bound itself and reads no confidence; a measurement has no meaning without one | PF-10, SV-5 |

## Consequences

- QSL states p95 latency, probability bounds and availability over a model
  with an explicit workload, and checks them with its own simulator.
- Every statistical result says what it is: a `measured` value with its
  method, samples, confidence parameters and coverage. It never reads as
  proof, and a rejected bound never reads as a refutation.
- A probabilistic model is still a nondeterministic model for every ADR-018
  claim, so exact verdicts and statistical measurements apply to one model.
- Statistical results reproduce bit for bit from their seed, and each
  violating sample replays as an ordinary model trace.
- Live-system statistics are deterministic window aggregates; monitors gain
  quantiles and fractions without probabilistic semantics.
- A rejected measurement fails the pipeline as a violation does, and is
  still never a refutation.
- Fixed-sample checking of probabilities near 1 needs millions of samples;
  SPRT needs thousands when the true probability is not close to the
  threshold. Long-run claims carry asymptotic coverage only.
- Exact probabilistic checking has a place to land: the same claim kind with
  evidence kind `exact`, settling `proved` or `refuted`.

## Amendments made with this record

- ADR-018 SM-2: a probabilistic claim's subject adds its workload (PM-8).
  §1 `ProofBasis` paragraph: statistical results are not a `ProofBasis`
  member (SV-1).
- ADR-014 §3 TR-1: a statistical run is identified by its provenance (RP-1).
  TR-5: aggregate atoms charge per value visited (WA-5). TR-6: EN-4 runs
  carry a seed.
- ADR-013 O-16 proof column: "measured: accepted" is success and never
  proof evidence; "measured: rejected" is violation, fails the pipeline as
  one, and is never `refuted`; "measured: undecided" is inconclusive (SV-3).
- ADR-016 §6: a statistical result reaches proof accounting never; it settles
  through negotiation as `measured` (SV-1).

## Amendments to make on acceptance

- ADR-021 PT-2: the row of RX-2, and RX-1's symmetry condition beside SYM-6.
- ADR-011 §1 and §6.1: S6c runs EN-4 as well as EN-1; layer A, crate
  `qsl-analyze`, gains `statistical` (ADR-029 CB-3); layer 5, `qsl-eval`,
  gains the weighted sampler; layer 6, `qsl-replay`, gains the `Measured`
  settlement and `replay_sampled_witness`.
- `spec/spec.md`: index row.

## Alternatives Considered

- **A `ProofBasis::Statistical{samples, confidence, error}` on `Proved`.**
  Rejected. `Proved` counts as proof evidence, so a statistical acceptance
  would count as proof wherever a consumer reads the value.
- **A new FR-243 basis `statistical` with truth `unavailable`.** Rejected
  (RU-5). It would stretch FR-242 and FR-243, whose values describe Boolean
  truth, to carry a measurement; a separate axis (SV-10) keeps both
  vocabularies exact.
- **Confidence parameters as method parameters outside the identity,** like
  ADR-018's depth. Rejected (RU-4): the confidence is part of what the claim
  asserts, so two confidences are two obligations.
- **Mean of an accumulated measure (mean latency).** Rejected (RU-3): a
  censored sample makes the mean `+∞`, and a quantile states latency without
  it.
- **Settling statistical results as `inconclusive` or `tested`.** Rejected.
  `inconclusive` loses the decision, and `tested` has no reading for a
  rejection. One value with three decisions keeps the decision and the
  category together.
- **The uniform sampler as the default scheduler.** Rejected. A p95 under a
  scheduler nobody declared has no meaning for a real workload. The claim
  names its workload; a workload with equal weights is one choice among many.
- **Scheduler sampling to estimate maximum and minimum probabilities over
  schedulers** (Legay and Sedwards' lightweight scheduler sampling). Not
  adopted: it gives lower bounds on the maximum only. Every-scheduler claims
  go to exact checking (SV-9).
- **Unbounded path formulas under a probability bound,** checked by detecting
  bottom components during sampling (Younes, Clarke and Zuliani). Not
  adopted: every event and measure has a finite window (PF-8), so one sample
  decides it without graph analysis.
- **Clopper–Pearson exact binomial intervals.** Tighter than Okamoto at the
  same coverage. Deferred: it needs the incomplete beta function under ST-9's
  exact arithmetic.
- **Batch means for long-run fractions.** Rejected in favour of regeneration,
  whose cycles are independent by the Markov property; batch means relies on
  batch length to approximate independence.
- **A distribution over initial states.** Rejected. ADR-018 claims hold from
  every initial state; probabilistic claims keep that reading (PM-4), and
  Bonferroni keeps the error bound.
- **Probabilistic choice by weighted post-states.** Rejected in favour of
  random parameters: a parameter names the outcome, keeps the transition
  identity exact and replayable, and reuses FR-120's enumeration.
- **Approximate quantile sketches in monitors.** Rejected. A sketch adds an
  approximation error to a monitor verdict; exact nearest-rank over a window
  bounded by `max_window_values` keeps the verdict a function of the trace.
- **Statistical properties as hyperproperties checked by self-composition**
  (ADR-023). Rejected for these forms. A quantile is a property of the
  measure over behaviours, not a relation between a fixed number of them.

## References

- Owning ticket: Linear QSL-371. Its QSpec half, QS-1 to QS-12, is Linear
  STD-137 (QSpec FR-405 to FR-414). The implementation ticket follows the merged spec.
- Crate layout (layer A `qsl-analyze` for engines, `qsl-eval` for the
  model system and sampler, `qsl-replay` for verdicts, replay and
  settlement) and the uncertified-proof rule: ADR-029 CB-2 and CB-3, owner
  ruling relayed by the plan lead; Linear QSL-390, on its own draft branch.
- Exact probabilistic checking of Markov decision processes with PRISM or
  Storm: Linear RES-53.
- Related: OBS-24 (SLO trace assertions kept apart from workload synthesis;
  the monitor side of §4), RES-28 (statistical soundness of a measurement
  relative to a pre-declared design; the claim's confidence parameters,
  PF-10, are that design).
- Sibling records: ADR-018 (QSL-366), ADR-019 (QSL-365, strong fairness),
  ADR-020 (QSL-367, refinement mappings), ADR-021 (QSL-368, state-space
  reduction), ADR-022 (QSL-369, possible properties), ADR-023 (QSL-370,
  hyperproperties).
- tl-mltl `spec/spec.md` "Out of Scope"; QSpec FR-124-AC-3 (probabilistic
  FRETish non-preserved).
- A. Wald, *Sequential Analysis*, 1947; and "Sequential tests of statistical
  hypotheses", *Annals of Mathematical Statistics*, 1945.
- M. Okamoto, "Some inequalities relating to the partial sum of binomial
  probabilities", *Annals of the Institute of Statistical Mathematics*, 1959.
- W. Hoeffding, "Probability inequalities for sums of bounded random
  variables", *Journal of the American Statistical Association*, 1963.
- H. L. S. Younes and R. G. Simmons, "Probabilistic verification of discrete
  event systems using acceptance sampling", CAV 2002; and "Statistical
  probabilistic model checking with a focus on time-bounded properties",
  *Information and Computation*, 2006.
- A. Legay, B. Delahaye and S. Bensalem, "Statistical model checking: an
  overview", RV 2010.
- A. Legay and S. Sedwards, "Lightweight Monte Carlo algorithm for Markov
  decision processes", 2014.
- H. L. S. Younes, E. M. Clarke and P. Zuliani, "Statistical verification of
  probabilistic properties with unbounded until", SBMF 2010.
- C. Baier and J.-P. Katoen, *Principles of Model Checking*, 2008 (Markov
  chains, bottom strongly connected components, long-run averages).
- P. W. Glynn and D. L. Iglehart, "Simulation methods for queues: an
  overview", *Queueing Systems*, 1988 (regenerative simulation).
- E. Ábrahám and B. Bonakdarpour, "HyperPCTL: a temporal logic for
  probabilistic hyperproperties", QEST 2018.
