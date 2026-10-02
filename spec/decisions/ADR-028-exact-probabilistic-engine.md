---
id: ADR-028
title: "Exact probabilistic checking: the native engine EN-5"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-024
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
  - target: ix://agent-ix/quire-specification/FR-243
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
---
# ADR-028: Exact probabilistic checking: the native engine EN-5

## Status

Proposed, 2026-10-01. §18 records the owner's and the plan lead's rulings
on the draft's six questions, folded into the decision. The QSL compiler
requirements that implement it are FR-195 to FR-204, under US-023. The
owner ruled that exact
probabilistic checking is a native quire engine, EN-5, specified now with
the rest of the QSL language work and built after EN-1 (ADR-018) and EN-4
(ADR-024). The ruling scopes it to claims over every scheduler, exact and
sound-interval bounds with certificates, counterexamples whose witness
scheduler replays exactly, and probabilistic timed automata through digital
clocks; it admits no uncertified floating-point result. A later ruling adds
claims over fair schedulers (§3a). The owning ticket and
the research are listed under References.

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. Item ids `SP-`, `PR-`, `SCH-`, `XF-`, `AR-`, `FH-`, `UR-`, `LR-`,
`FS-`, `XV-`, `WS-`, `CE-`, `LM-`, `TA-`, `RX-`, `DS-`, `QS-` and `RU-` are local to this
record. Other artifacts cite them as `ADR-028 CE-3`. Items of other records
are cited as `ADR-024 PM-8`.

## Context

ADR-024 makes a model probabilistic with random parameters (PM-1) and a
workload (PM-2). Under a workload `W` the model is a discrete-time Markov
chain (DTMC); with the workload left out it is a Markov decision process
(MDP) whose scheduler picks the scheduled identity at each step. ADR-024's
statistical engine EN-4 samples the chain and settles `measured` (SV-1),
never a proof. ADR-024 leaves three things to an exact backend:

- **`under every scheduler`** (PM-8), which sampling cannot settle: scheduler
  sampling gives lower bounds on the maximum only (Legay and Sedwards).
- **The `exact` evidence kind** (SV-9). A request item names `statistical` or
  `exact` evidence; an exact backend settles `proved` or `refuted`,
  `closed-scope`, under a `ProofBasis` member its own record defines, and
  negotiation never substitutes one kind for the other.
- **Forms with weak sampling guarantees.** Thresholds close to the true value
  settle "measured: undecided"; rare-event targets near `10⁻⁹` need on the
  order of `10⁹` samples under SPRT and `10¹⁸` under Okamoto; long-run
  fractions carry `Asymptotic` coverage only (ST-7).

ADR-026 adds timed subjects. Its RU-5 and SS-7 route `exact` evidence over a
timed subject to EN-5 when the subject's constraints are all closed: EN-5
reads it through digital clocks as a probabilistic timed automaton (PTA).
An operation with no `delay` member has a free, nondeterministic delay
(ADR-026 SD-1), which the claim's scheduler resolves (§13). A strict
constraint and an SD-1 delay distribution with no exact digital reading
settle `unsupported` on that route and are measured by EN-4.

What EN-5 builds on:

- **EN-1's product.** ADR-018 EN-1 explores the product of the subject's
  state graph with a deterministic monitor for every bounded and safety form
  (SM-6), retains every product edge, and decomposes the retained graph into
  strongly connected components.
- **Step probabilities.** ADR-024 PM-3 gives every transition its exact
  rational step probability under a workload, and PM-6 states that the
  support graph of the chain is ADR-018's state graph.
- **The verdict vocabulary.** ADR-018 V-1 to V-8, `ProofBasis` on
  `TerminalValue::Proved`, and the FR-331 values `proved` and `refuted` with
  the FR-243 bases `closed-scope` and `decisive-counterexample`.
- **Certified proofs.** ADR-026 CF-1 to CF-6 give EN-6 zone certificates that
  leave S6c over E11 and settle only after a layer-6 checker accepts them;
  EN-5 follows the same path.

## Decision

### 1. Scope, placement and evidence

| ID | Rule |
| --- | --- |
| SP-1 | **Engine EN-5.** EN-5 computes, over the full finite subject, the exact value or a sound interval of a probabilistic claim's quantity: a probability, a conditional probability, the expectation of a fraction, a long-run fraction or an expected reward. It decides the claim from that value, under a workload or over every scheduler, and every verdict carries evidence a checker verifies in exact rationals with no engine present: a certificate for a proof (§11), a witness for a refutation (§10). |
| SP-2 | **Placement.** EN-5 lives in QSL layer A, crate `qsl-analyze`, module `exact_probabilistic`, beside `model_check` and `statistical`, above the qualified core (ADR-029 CB-3), and runs in ADR-018's stage S6c over E10. It reuses `model_check`'s product `TransitionSystem`, its monitor translations, its edge retention and its SCC decomposition, and reads FR-120's `ModelSystem` from layer 5, `qsl-eval`. Its verdict types, its witness replay arm, its certificate checker and its settlement map live in layer 6, `qsl-replay`, beside `replay_model_trace` (§10, §11), as does every type CG reads. Every EN-5 proof carries a certificate that the in-core checker checks (CE-4), so the rule that a proof whose engine has no core certificate checker settles `proved` labelled `uncertified` does not arise for EN-5. |
| SP-3 | **Evidence kind `exact`.** EN-5's provider manifest advertises (`probabilistic-satisfaction`, evidence `exact`) (ADR-024 SV-9). CG's `negotiate_*` arm for `probabilistic-satisfaction` routes an item naming `exact` to EN-5 and an item naming `statistical` to EN-4. One claim may be requested with either kind; the two results are separate items with separate terminal records. |
| SP-4 | **Confidence parameters.** ADR-024 PF-10's `α`, `β` and `ι` are for statistical evidence only (ADR-024 RU-6, RU-2 here). When a claim states them they stay part of its obligation identity; EN-5 decides the claim's bound itself, which entails every stated confidence, so it reads none of them. A claim that states none is exact-only: statistical checking refuses it at negotiation (`MissingConfidence`, ADR-024 SV-5), and EN-5 checks it. |
| SP-5 | **Proof from the engine's evidence.** An EN-5 `proved` leaves S6c with its certificate over ADR-018's E11, as a refutation leaves with its counterexample, and settles only after the layer-6 checker accepts the certificate (CE-5), as ADR-026 CF-4 states for EN-6. An EN-5 `refuted` settles only after its witness replays (WS-5). |
| SP-6 | **Arithmetic.** Every number on the verdict path is an exact rational or a dyadic interval held in integer arithmetic (§5). A result that has no certificate the checker accepts is never admitted. |

### 2. The probabilistic product

| ID | Rule |
| --- | --- |
| PR-1 | **Actions.** At a model state `s`, the actions are the enabled scheduled identities (ADR-024 PM-2): (operation, receiver, non-random argument vector) whose effective precondition holds at `s` and that have at least one post-state for some drawn random vector. An action `a` has the **draw distribution** `D(s, a)`: each random argument vector `r` in the product of its parameters' supports with probability the product of their PM-1 probabilities. Preconditions read no random parameter (PM-1), so the action set is decided before any draw. |
| PR-2 | **The MDP.** A drawn vector `r` of action `a` at `s` leads to the post-states FR-120 gives for `(a, r)`. When there is exactly one, the edge `s → s'` carries probability `D(s, a)(r)`, summed over the vectors that lead to `s'`. When there are several, the choice among them is part of the scheduler's choice (SCH-2). When there are none, the draw loses mass, and the subject settles `unsupported`, `NotMarkov{state, transition, post_states}` (ADR-024 PM-3). The MDP's support graph is ADR-018's state graph (ADR-024 PM-6). |
| PR-3 | **The DTMC under a workload.** Under `W`, each action is chosen with ADR-024 PM-2's probability, so each edge carries PM-3's step probability. Several post-states for one drawn vector settle `NotMarkov` under a workload, as PM-3 states. |
| PR-4 | **The product with a monitor.** For a form whose event is a bounded formula or a comparison of a bounded measure (ADR-024 PF-1, PF-2), EN-5 forms the product of the MDP or DTMC with ADR-018 SM-6's deterministic finite monitor for the formula, read at each position (ADR-018 SM-3), with EN-1's closure at terminal model states. A bounded measure adds its accumulator to the product state: the steps or reward accumulated since activation, saturating at a single value above the threshold `c` (any value above `c` serves, since only `M <= c` and `M < c` are read), and the activation flag. Rewards are non-negative values from finite supports (ADR-024 PM-5), so the accumulator takes finitely many values. The product state key is (model state key, monitor state index, accumulator); the product edge carries the model edge's action, draw and probability. |
| PR-5 | **Decided states.** The monitor of a bounded formula counts positions to its horizon `h` (ADR-014 TR-4), so every product path reaches an accepting or a rejecting monitor state within `h + 1` positions, which EN-5 keeps as absorbing **decided** states. The product restricted to undecided states is acyclic. |
| PR-6 | **Products for unbounded forms.** An unbounded form (XF-4, XF-5, XF-6) needs no monitor: its state predicates label model states and its reward labels edges. Its product is the MDP or DTMC itself, with the target or the long-run predicate as labels. |

### 3. Every scheduler

| ID | Rule |
| --- | --- |
| SCH-1 | **Meaning.** `under every scheduler` (ADR-024 PM-8) quantifies over every scheduler of PR-2's MDP: any function from the finite history of states and actions to a distribution over the enabled actions, history-dependent and randomized. `probability >= θ [E]` holds when the **minimum** over schedulers of `Pr[s0, σ](E)` is at least `θ`, and `probability <= θ [E]` when the **maximum** is at most `θ`, for every initial state `s0` and every `over` binding (ADR-024 PM-4). Each other form reads its quantity's minimum or maximum the same way. The workload plays no part, and a scheduler may schedule any enabled action, including one a workload would weight low. |
| SCH-2 | **Residual nondeterminism.** When one drawn vector of an action has several post-states, the scheduler picks one after the draw, knowing the drawn vector, as in a standard MDP (RU-4). The MDP models this choice as an intermediate state `(s, a, r)` with one action per post-state, each with probability 1. Intermediate states are not positions: a monitor reads only model states (ADR-018 SM-3). |
| SCH-3 | **Memoryless deterministic schedulers suffice on the product.** On a finite MDP, the minimum and the maximum of a reachability probability, of a finite-horizon expected reward, of an expected reward to a target, and of a long-run average reward are attained by a scheduler that picks one action per state, ignoring history (Puterman; Baier and Katoen). Every form of §4 reduces to one of these on PR-4's product, whose monitor state and accumulator are exactly the finite memory the form needs on the model. EN-5 therefore computes over memoryless deterministic schedulers on the product, and its witness scheduler (§10) is one. |
| SCH-4 | **Fairness.** An every-scheduler claim with no fairness set ranges over every scheduler, including one that starves an enabled action. One that states a fairness set ranges over the fair schedulers of §3a (RU-3). A claim under a workload carries no fairness set: the workload is fair with probability 1 (ADR-024 PM-7). |
| SCH-5 | **Under a workload.** Under `W` the scheduler is PM-2's memoryless randomized scheduler, the product is a DTMC, and its value is one number per initial state and binding. |

### 3a. Fair schedulers

A randomized protocol terminates only against an adversary that eventually
lets every process move; against every scheduler, one that starves a process
forever makes termination fail for a reason the environment excludes. A
fairness set states that premise, and the claim is decided over the
schedulers that meet it with probability 1 (Baier and Kwiatkowska).

| ID | Rule |
| --- | --- |
| FS-1 | **Fairness set.** A claim `under every scheduler` may state a fairness set of ADR-018 weak and ADR-019 strong constraints, `whole` or `each`, over the model's operations, with ADR-018 FA-1's granularity. The set is part of the claim and of its obligation identity (ADR-013 O-09). A constraint is enabled at a model state when one of its scheduled identities is an action there (PR-1), and taken by a step whose scheduled identity belongs to it. Intermediate states (SCH-2), the terminal stutter step and, over a timed subject, delay moves (§13) belong to no constraint and enable none. |
| FS-2 | **Fair schedulers.** A scheduler `σ` is fair for the set `F` from `s0` when the behaviours that satisfy every constraint of `F` have probability 1 under `Pr[s0, σ]`. With a fairness set, SCH-1's minimum and maximum read as the infimum and supremum over fair schedulers, for every initial state and binding. A fair scheduler always exists: the uniform scheduler over the enabled actions is fair with probability 1 (ADR-024 PM-7). |
| FS-3 | **Fair end components.** An end component `(C, A)` of the product, a strongly connected set of states `C` with actions `A` that never leave it, is **fair** for `F` when, for each strong constraint enabled at some state of `C`, `A` holds an action of it, and for each weak constraint enabled at every state of `C`, `A` holds an action of it. A scheduler that stays in a fair end component and takes every action of `A` with positive probability is fair with probability 1; and under a fair scheduler, the states and actions a behaviour takes infinitely often form a fair end component with probability 1 (de Alfaro; Baier and Kwiatkowska). |
| FS-4 | **Computing them.** EN-5 computes the maximal fair end components by refinement: decompose the product into MECs (UR-2); in each, drop the component when a weak constraint is enabled at every state and no action of it is in `A`, and remove the states where a strong constraint is enabled when no action of it is in `A`; decompose what remains into MECs again; repeat until no component changes. Each round removes at least one state or ends, so the cost is `O(|F| · |S| · |E|)` beside MEC decomposition. |
| FS-5 | **Bounded forms.** For XF-1 to XF-3, the infimum and supremum over fair schedulers equal the minimum and maximum over every scheduler: a scheduler's choices up to the horizon extend past it by the uniform scheduler, which is fair (FS-2), and a bounded event reads nothing past its horizon. EN-5 decides them by §6 and records the fairness set in the result. |
| FS-6 | **Reachability.** The supremum over fair schedulers of `Pr(eventually holds(B))` equals the maximum over every scheduler; it is attained by UR-7's scheduler at states of positive value and the uniform choice at every other state. For the infimum, let `U` be the union of the fair end components of the product restricted to states where `B` does not hold; then the infimum is `1 − max Pr(not B until U)`, the maximum over every scheduler (Baier and Kwiatkowska), which §7 computes. It is attained by UR-7's scheduler for that maximum at states of positive value, the uniform choice over each fair end component's actions in `U`, and the uniform choice at every other state. `holds(A) until holds(B)` reads `U` over states where `A` holds and `B` does not, and counts reaching a state where neither holds as missing `B`. `always holds(P)` reads as `1 − Pr(eventually holds(not P))`, minimum and maximum exchanged (XF-4). |
| FS-7 | **Expected rewards.** For XF-5's minimum, fairness changes no value: a scheduler that reaches `B` with probability 1 reaches it in finite time and continues fairly after it. For the maximum, the value is `+∞` at a state where the infimum over fair schedulers of reaching `B` is below 1 (FS-6), and at a state that reaches, among states where that infimum is 1, an end component holding an action of positive reward, since a fair scheduler may stay in it for any finite time; elsewhere EN-5 collapses the zero-reward end components and runs UR-5. |
| FS-8 | **Long-run fractions.** Under a fair scheduler a behaviour ends in a fair end component with probability 1 (FS-3). EN-5 decomposes the product into maximal fair end components and gives each component `C` LR-2's optimum over its actions: a fair scheduler approaches it by taking the actions the optimum leaves out with vanishing frequency, so the value is an infimum or supremum that need not be attained. LR-3 combines the component values with every end component that holds no fair end component collapsed as in UR-2, so no scheduler in the range stays outside the fair end components. |
| FS-9 | **Evidence.** A certificate's objective names the fairness set, and the checker recomputes the maximal fair end components by FS-4's refinement with its own graph code, as it recomputes `Prob0` and `Prob1` (CE-2); a certificate carries no qualitative sets. A witness scheduler for a claim with a fairness set is memoryless and randomized on the product: each entry may give a distribution over identities with exact rational probabilities (WS-1). Replay checks that the witness is fair: every bottom strongly connected component of the chain it induces on the evidence's states meets FS-3's condition. For a long-run refutation, where the infimum need not be attained, EN-5 mixes the actions the optimum leaves out with probability `2^-k`, raising `k` until the induced chain's exact value is past the threshold, and the evidence is that chain's `LongRun` certificate (CE-3). |

### 4. The forms EN-5 decides

EN-5 decides every ADR-024 form with exact evidence, under a workload and
over every scheduler, and three unbounded forms that exact evidence alone
checks.

| ID | Rule |
| --- | --- |
| XF-1 | **Probability bounds and quantiles (ADR-024 PF-3, PF-4).** The event is decided on a finite window, so the value is a finite-horizon reachability probability of the rejecting or accepting decided states on PR-4's product (§6). |
| XF-2 | **Quantile thresholds over every scheduler.** A quantile claim reads a probability conditioned on activation (PF-4). `quantile q of M <= c` holds over every scheduler exactly when `min over σ of E[σ](X) >= 0`, with `X = [activated] · ([M <= c] − q)`, a finite-horizon terminal reward: for a scheduler with `Pr(activated) > 0`, `E(X) >= 0` is `Pr(M <= c | activated) >= q`, and a scheduler that never activates contributes 0. `quantile q of M >= c` holds exactly when `Pr(M < c | activated) < q` under every scheduler with `Pr(activated) > 0`, a strict bound (ADR-024 PF-4). EN-5 computes `v = max over σ of E[σ](Y)`, `Y = [activated] · ([M < c] − q)`: the claim fails when `v > 0` and holds when `v < 0`; when `v = 0`, EN-5 keeps at each product state only the actions that attain `v` and computes the maximum probability of activation, and the claim fails when it is positive, since a scheduler then meets equality while activating, and holds when it is 0. With `M` equal to 1 or 3 with probability 1/2 each, `q = 1/2` and `c = 3`, `v = 0` is attained with activation probability 1, so the false claim is refuted. Under a workload EN-5 computes the conditional probability directly as the ratio of two finite-horizon probabilities. |
| XF-3 | **Mean of a fraction (PF-5).** Unweighted, the expectation of `fraction holds(P) over [0, h]` is `1/(h + 1)` times the expected number of positions in `[0, h]` where `P` holds, a finite-horizon expected reward with reward 1 at each such position. Weighted by `R`, the fraction is a ratio of two sums, so the product accumulates both sums (PR-4) and the value is a finite-horizon expected terminal reward. |
| XF-4 | **Unbounded reachability.** Under the infinite-trace profile, `probability >= θ` or `<= θ` over `[eventually holds(P)]`, `[holds(A) until holds(B)]` or `[always holds(P)]`, with `P`, `A` and `B` state predicates and `0 < θ < 1`. `always holds(P)` reads as `1 − Pr(eventually holds(not P))`, with minimum and maximum exchanged. The form is decided by §7. |
| XF-5 | **Expected reward to a target.** `expected accumulate R until holds(B) <= c` or `>= c`, for a reward `R` (ADR-024 PM-5) and a threshold `c` of its type, from position 0. On a behaviour that reaches `B`, the value is `R` summed over the steps up to the first position where `B` holds; under a scheduler whose probability of reaching `B` is below 1, the expectation is `+∞`, so a `<= c` bound fails and a `>= c` bound holds there. Over a timed subject, `expected elapsed until holds(B)` reads the time stamp of that position (ADR-026 SS-1). The form is decided by §7, with exact evidence only (XF-7, RU-1): only the exact engine detects the `+∞` cases. |
| XF-6 | **Long-run fractions (PF-6).** `long-run fraction holds(P)`, unweighted or weighted by `R`, decided by bottom components (§8). With exact evidence the result is a proof or a refutation, not ADR-024 ST-7's `Asymptotic` measurement. |
| XF-7 | **Exact evidence only.** XF-4 and XF-5, and every form `under every scheduler`, have no finite-window sample or no sampling route, so they are checked with exact evidence; an item that names `statistical` evidence for one settles `unsupported`, `ExactOnlyForm` (or `EveryScheduler`), at negotiation (ADR-024 SV-5). |
| XF-8 | **Thresholds and units.** Thresholds stay strictly between 0 and 1 for probabilities and fractions, and qualitative claims stay ADR-018 claims (ADR-024 PF-8). A threshold of a reward or time has the reward's dimension and converts by QSpec FR-142 (ADR-024 PF-7). For a `>= θ` bound the value meets the claim when it is at least `θ`, and for `<= θ` when it is at most `θ`: equality meets a non-strict bound. The `>=` quantile form reduces to a strict bound (XF-2), which equality does not meet. |

### 5. Exact arithmetic

| ID | Rule |
| --- | --- |
| AR-1 | **Exact rationals.** Probabilities, weights, rewards, thresholds and values are rationals held as pairs of arbitrary-precision integers in lowest terms. ADR-024's PM-1 and PM-2 probabilities are exact rationals by construction. |
| AR-2 | **Dyadic intervals.** Iterative methods hold each value as an interval `[l, u]` of dyadic rationals `m · 2^-p`, `m` an arbitrary-precision integer and `p` the working precision. Every operation rounds `l` down and `u` up by integer floor and ceiling division, so the interval always contains the true value of the operation on its inputs. This bounds the bit length of every value by `p` and contains the growth of exact rationals, whose denominators grow with every step. |
| AR-3 | **No floating point.** No IEEE-754 value and no hardware rounding mode enters the verdict path. Results are identical on every platform and every build, and the decision is a function of the subject, the claim and the limits alone, as ADR-018 §3 states for EN-1. |
| AR-4 | **Certified output.** EN-5 emits a value only with a certificate that it has first checked with the checker of §11. A candidate the check rejects is not emitted; EN-5 raises precision or moves to its exact method, and a run that runs out of budget settles V-7 (LM-2). |

### 6. Finite-horizon forms: exact backward induction

| ID | Rule |
| --- | --- |
| FH-1 | **Method.** On PR-4's product, undecided states form a DAG layered by position (PR-5). EN-5 computes each state's value in reverse topological order: a decided state has its terminal value (1 or 0 for an event, the terminal reward of XF-2 or XF-3); an undecided state's value is the probability-weighted sum over its successors, under a workload, or the minimum or maximum over its actions of that sum, over every scheduler. The value at the initial product state is exact. There is no convergence question and no iteration count. |
| FH-2 | **Order of precision.** EN-5 first runs FH-1 in exact rationals. When a value's bit length passes `max_rational_bits` (LM-1), it reruns FH-1 over dyadic intervals at `precision_bits`; each layer adds at most one rounding unit to each end, so after `h + 1` layers the interval width is at most `2 (h + 1) · 2^-p`. When the interval still contains the threshold, EN-5 doubles `p` up to `max_precision_bits`. An interval that straddles the threshold at the budget settles V-7, `PrecisionBudget{lower, upper}`. |
| FH-3 | **Cost.** One pass visits each product edge once: `O(|E|)` arithmetic operations, `|E|` the product's edges, which grows with the horizon because the monitor counts positions. In exact rationals the bit length of a value grows linearly in the depth; over dyadic intervals each operation costs `O(p)`-bit integer arithmetic. |
| FH-4 | **Witness scheduler.** Over every scheduler, the action attaining the minimum or maximum at each product state, ties broken by canonical transition order (ADR-018 §3, Determinism), is the memoryless deterministic witness scheduler of §10. |

### 7. Unbounded reachability and expected rewards

| ID | Rule |
| --- | --- |
| UR-1 | **Qualitative precomputation.** EN-5 first computes, by graph algorithms alone, the states whose value is 0 or 1: `Prob0` and `Prob1` under a workload; `Prob0A`, `Prob0E`, `Prob1A` and `Prob1E` over every scheduler (de Alfaro; Baier and Katoen §10.6). Values at these states are exact without arithmetic, and the remaining states have values strictly between 0 and 1. |
| UR-2 | **End components.** Over every scheduler, EN-5 decomposes the MDP into maximal end components (MECs). For a maximum it collapses each MEC with no target state into one state, keeping the actions that leave it; for a minimum, `Prob0E` already holds the states that can stay away from the target forever. On the resulting MDP the reachability operator has a unique fixed point, so iteration from both ends converges to the value (Haddad and Monmege; Baier, Klein, Leuschner, Parker and Wunderlich). |
| UR-3 | **Interval iteration.** EN-5 iterates the Bellman operator from 0 and from 1 at once, over dyadic intervals with outward rounding (AR-2): the lower sequence rises and the upper sequence falls, and each iterate of each is a valid lower or upper certificate (CE-3). It stops when the interval at every initial product state lies on one side of the threshold. Plain value iteration with a convergence threshold has no stopping guarantee and is never run. |
| UR-4 | **Exact policy iteration.** When the interval still contains the threshold after `max_iterations` sweeps or at `max_precision_bits`, EN-5 runs policy iteration in exact rationals: it fixes a memoryless deterministic scheduler, evaluates it by an exact sparse linear solve (fraction-free elimination, Bareiss), improves it action by action, and stops at a scheduler no action improves. The value is exact, so a value equal to the threshold is decided, which no interval method decides. Under a workload it is one exact linear solve. |
| UR-5 | **Expected rewards.** For XF-5, the qualitative step finds the states that reach `B` with probability 1 under every scheduler (for a maximum) or under some scheduler (for a minimum); elsewhere the value is `+∞`, decided by the graph alone. On the rest, interval iteration needs an upper starting vector, which EN-5 obtains as in sound value iteration (Quatmann and Katoen), and end components of zero reward are collapsed as in UR-2. Exact policy iteration is the fallback, as in UR-4. |
| UR-6 | **Cost.** Qualitative precomputation is linear in the product for a DTMC and `O(|S| · |E|)` for an MDP; MEC decomposition is `O(|S| · |E|)`. Each interval sweep costs `O(|E|)` operations at `p` bits; the number of sweeps depends on how fast the model mixes, and `max_iterations` bounds it. Each policy evaluation costs a sparse elimination whose integers stay within the Hadamard bound, polynomial in the states and the bit length of the step probabilities; policy iteration ends after finitely many improvements, exponentially many in the worst case (Fearnley) and few in practice, and `max_policy_iterations` bounds them. |
| UR-7 | **Canonical result.** The witness scheduler is the final policy of UR-4 or, from interval iteration, the action that attains the bound at each state in the final upper (for a maximum) or lower (for a minimum) iterate, ties broken by canonical transition order. For a maximum, the policy is chosen on UR-2's collapsed MDP and expanded on the product: at each collapsed MEC, the chosen leaving action at the MEC state that owns it, and at every other state of the MEC the first action, in canonical order, on a shortest path inside the MEC to that state. An action that stays inside a MEC attains the same iterate value as the leaving action, so choosing per state would let the induced chain stay in the MEC forever; the expansion leaves it and attains the maximum. |

### 8. Long-run fractions

| ID | Rule |
| --- | --- |
| LR-1 | **Bottom components.** Under a workload, EN-5 decomposes the DTMC into bottom strongly connected components (BSCCs). Over every scheduler, it decomposes the MDP into MECs. A behaviour ends in one such component with probability 1 (ADR-024 PM-7), and the long-run fraction is constant within it (PF-6). |
| LR-2 | **Value per component.** In a BSCC `C`, the long-run fraction is `π_C(R · [P]) / π_C(R)`, `π_C` the unique stationary distribution of `C`, found by an exact linear solve; unweighted, `R = 1`. The weighted form is kept over every scheduler (RU-5): time-weighted availability over a timed subject weights by delay (ADR-026 SS-1). In a MEC `C`, the optimal fraction over schedulers that stay in `C` is a ratio of mean payoffs; EN-5 finds it by Dinkelbach's parametric method, each round solving a mean-payoff problem for the reward `R · ([P] − ρ)` by exact policy iteration, and stopping at the `ρ` where the optimal gain is 0. |
| LR-3 | **Combination.** The claim's value is the expected component value reached: under a workload, `Σ_C Pr(reach C) · ρ_C`; over every scheduler, the minimum or maximum over schedulers of that sum, an expected terminal reward on the MDP with `ρ_C` at each component (de Alfaro). Both are §7 problems. |
| LR-4 | **Zero weight.** A component in which every cycle has total weight 0 under `R` has no defined fraction. The item settles `unsupported`, `ZeroWeightComponent{component}` (V-8). |
| LR-5 | **Relation to EN-4.** ADR-024 ST-7 measures the same quantity by regeneration with `Asymptotic` coverage. With exact evidence the item settles `proved` or `refuted`, and `coverage` does not apply. |

### 9. Verdicts

An exact result is a proof or a refutation. It settles with ADR-018's verdict
kinds and its own `ProofBasis` members, never with ADR-024's `measured`.

| ID | Rule |
| --- | --- |
| XV-1 | **`ProofBasis` members.** `ExactValue{value, reductions}` for a value computed exactly (FH-1 in exact rationals, UR-4, LR-2), and `ValueBounds{lower, upper, method, reductions}` for a sound interval that lies on the claim's side of the threshold (FH-2, UR-3), with `method` one of `BackwardInduction{precision_bits}` and `IntervalIteration{precision_bits, sweeps}`. `reductions` is ADR-021 RV-1's list of applied reductions, empty when none applies. The basis states the least favourable (initial state, binding) pair; the result keeps one entry per pair with its value or interval and, over every scheduler, whether it is a minimum or a maximum. `TerminalValue::category` maps both members to success, and both count as proof evidence. |
| XV-2 | **Distinct from `measured`.** An EN-5 result carries FR-242 truth and an FR-243 basis, as every proof and refutation does, and no ADR-024 SV-10 `measured` value. An EN-4 result carries `measured` and no truth. The evidence kind the request named decides which record an item gets, and no reader can take one for the other. |
| XV-3 | **Refutation.** A refutation carries a `ProbabilisticCounterexample` (§10) that replays before the item settles: `refuted`, `decisive-counterexample`, `TerminalValue::Refuted`. |
| XV-4 | **New causes.** `IncompleteCause::PrecisionBudget{lower, upper}`, the interval at the budget (V-7). `InconclusiveCause::CertificateRejected`, ADR-026 CF-4's cause, for a proof certificate the checker rejects (V-6). V-8 `Unsupported` causes: `ZeroWeightComponent` (LR-4), `StrictClockConstraint`, `DelayDistribution`, `ZeroDelayCycle` and `TimedFormShape` (§13), and ADR-026's `NotStochastic` on the digital route under a workload (TA-1a). ADR-024's `NotMarkov` (PR-2, PR-3) and ADR-018's `UndecidedSuccessor`, `NoInitialState`, `ReplayParity` and `ReplayRefused` keep their meaning. |
| XV-5 | **Evidence on a partial product.** A lower certificate (CE-3) and a path set (WS-3) read only the states they name and those states' successors, and an unexplored successor counts as value 0, which keeps a lower bound sound. So evidence of that kind found before a limit stops exploration settles the item: a refutation of a `<= θ` bound or a proof of a `>= θ` bound, as ADR-022 GV-1 lets a witness settle a run that a limit later stopped. Every other verdict needs the complete product. |

**XV-6 Verdict table.**

| Outcome | ADR-018 kind | FR-331 value | FR-243 basis | `TerminalValue` | O-16 category | What the checker reads |
| --- | --- | --- | --- | --- | --- | --- |
| Exact value on the claim's side of the threshold, equality counting for a non-strict bound | V-1 style | `proved` | `closed-scope` | `Proved{basis: ExactValue{…}}` | success | An exact certificate (CE-3) over the re-enumerated product |
| Sound interval on the claim's side | V-1 style | `proved` | `closed-scope` | `Proved{basis: ValueBounds{…}}` | success | A lower or upper certificate (CE-3) |
| Bound fails over every scheduler | V-4 | `refuted` | `decisive-counterexample` | `Refuted` | violation | Witness scheduler and its path set or subsystem certificate (WS-3, WS-4) |
| Bound fails under a workload | V-4 | `refuted` | `decisive-counterexample` | `Refuted` | violation | Path set or subsystem certificate |
| Interval straddles the threshold at the budget | V-7 | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(PrecisionBudget{lower, upper})` | incomplete | Nothing; the caller raises the budget and reruns |
| A run limit reached first | V-7 | as above | `unavailable` | `Incomplete(ResourceExhausted)` naming the `ExactProbLimits` member | incomplete | Nothing |
| Certificate rejected; witness replay disagrees | V-6 | `inconclusive` | `unsettled` | `Inconclusive(CertificateRejected)`; `Inconclusive(ReplayParity)` | inconclusive | — |
| Strict clock constraint, delay distribution, zero-delay cycle, unsupported timed form shape, zero-weight component, `NotMarkov` | V-8 | `unsupported` | `unavailable` | `Unsupported(cause)` | unsupported | Nothing |

| ID | Rule |
| --- | --- |
| XV-7 | **Scope.** A verdict holds for its subject (ADR-018 §1 Scope), its claim, the claim's workload or `every scheduler`, and every initial state and binding. The obligation identity binds all of them (ADR-013 O-09) and the confidence parameters the claim states (SP-4); the evidence kind is a request member and enters the item's request, not the claim. |
| XV-8 | **Undefined evaluation.** ADR-018 UE-1 to UE-6 apply. When an atom, a state predicate, a measure or a reward the claim reads evaluates undefined at a product state reached with positive probability (under the workload, or under some scheduler over every scheduler), the item settles `refuted`, XV-3, with cause `UndefinedEvaluation{where, cause}`, `where` the position of that state on the evidence path. A claim undefined on a behaviour of positive probability does not hold whatever its threshold, so no probability sum applies. The evidence is an `Undefined` path (WS-3): the canonical breadth-first path to the first such product state, every step of positive probability, with the witness scheduler's choices along it over every scheduler. An undefined position ranks like any other violation (ADR-018 UE-4). |

### 10. Counterexamples and the witness scheduler

| ID | Rule |
| --- | --- |
| WS-1 | **Witness scheduler.** `WitnessScheduler{entries}`: for each product state the evidence names, its key (PR-4) and its choice: a scheduled identity; after a draw with several post-states, the post-state digest per drawn vector (SCH-2); over a timed subject, a unit `Delay` or an identity (§13). It is memoryless and deterministic on the product (SCH-3), or memoryless and randomized for a claim with a fairness set (FS-9), and is defined on the evidence's states only. |
| WS-2 | **`ProbabilisticCounterexample`.** `{initial_state, binding, bound, scheduler, evidence}`: the index of the initial state in the subject and the `over` binding (ADR-018 CX-2), the claim's bound with its threshold, the witness scheduler (absent under a workload), and the evidence. It travels in `WitnessEnvelope<ProbabilisticCounterexample>` with a new `ReplaySource::ProbabilisticWitness` arm, and the envelope's obligation identity binds the subject and claim. |
| WS-3 | **Path-set evidence.** A finite list of finite product paths from the initial state, each as ADR-018 CX-2's step content plus each step's drawn random vector, on which the event is false (for a `>= θ` bound) or true (for a `<= θ` bound), pairwise prefix-free so their cylinders are disjoint, whose exact path probabilities sum past the bound: above `1 − θ` for `>= θ`, above `θ` for `<= θ`. A path's probability is the product of its step probabilities under the workload (ADR-024 PM-3), or of its draw probabilities and the witness scheduler's choice probabilities. For a quantile claim the paths witness XF-2's transformed reward: their activated, violating mass exceeds `1 − q`, which suffices because `Pr(activated) <= 1`. EN-5 emits a path set when at most `max_witness_paths` paths reach the bound, most probable first. An undefined evaluation (XV-8) is one path ending at the undefined state, with its `UndefinedEvaluation`, and carries no bound sum. |
| WS-4 | **Subsystem evidence.** Otherwise the evidence is a lower certificate (CE-3), on the chain the witness scheduler induces or under the workload, for the violating quantity (`Pr(not E)` for `>= θ`, `Pr(E)` for `<= θ`) with value past the bound. The certificate's support is a critical subsystem (Funke, Jantsch and Baier): the checker explores only its states and their successors. Long-run and expected-reward refutations use the same shape with their own certificate kinds (CE-3). |
| WS-5 | **Replay.** `qsl_replay::replay_probabilistic_witness`, a layer-6 facade entry beside `replay_model_trace`, recompiles the package (FR-098) and re-admits the subject's initial state and universes from the byte provision, as ADR-018 CX-3 does. For a path set it re-executes each path through `ModelSystem` with FR-101 `replay`, selects each step's successor by its post-state digest, checks that each drawn value lies in its support (ADR-024 SV-8) and, over every scheduler, that each step takes a choice the witness scheduler gives positive probability at its product state and, with a fairness set, that the witness is fair (FS-9); it recomputes each path probability exactly, evaluates the event with SM-1 (ADR-018 SM-1), checks prefix-freeness and checks the sum against the bound. For a subsystem it applies the witness scheduler to rebuild the induced chain over the support and runs the certificate checker (CE-5). An `Undefined` path replays as a path of a path set, checking each step's positive probability, and then by ADR-018 UE-5 in place of the event and the sum. Agreement settles `reproduced-with-evaluated-witness` and the item `refuted`; disagreement settles `inconclusive`, `ReplayParity`. It refuses as CX-3 refuses: `stale_dependency`/`revision-mismatch` for a digest with no matching successor, `invalid_runtime_input`/`invalid-value` for a step that is not enabled, a value outside its support or a choice that is not the scheduler's. |
| WS-6 | **What the witness scheduler gives a user.** It is an adversary that meets the violation: replaying it through `ModelSystem` with ADR-024's sampler draws behaviours of the violating chain, and any one of its paths on which the event is false is an ADR-018 CX-1 counterexample for the qualitative claim (ADR-024 SV-7). |

### 11. Certificates and the checker

| ID | Rule |
| --- | --- |
| CE-1 | **`ProbabilityCertificate`.** `{identity, objective, kind, values, ranking, policy, components}`: the item's obligation identity (ADR-013 O-09, as ADR-026 CF-5); the objective (the target or decided states, the reward, minimum, maximum or workload, and the fairness set, FS-9); the kind `Lower`, `Upper`, `Exact` or `LongRun`; a value per product state, each an exact rational (a dyadic value from interval iteration is an exact rational); for `Lower` and `Exact`, a natural-number ranking per state; for a lower bound on a maximum, or an upper bound on a minimum, a memoryless deterministic policy; and for `LongRun`, per component its value and a gain–bias pair. States are named by product state key (PR-4). The certificate names its states; it carries no qualitative sets, which the checker recomputes. |
| CE-2 | **The checker re-enumerates.** `qsl_replay::check_probability_certificate`, a layer-6 facade entry, recompiles the package, re-admits the subject and re-enumerates the product through `ModelSystem` and the monitor: every reachable product state for an `Upper`, `Exact` or `LongRun` certificate, the support and its successors for a `Lower` one. It recomputes the decided states, the `Prob0`/`Prob1` sets, the components and, with a fairness set, the maximal fair end components (FS-4) by graph algorithms, and checks the conditions of CE-3 in exact rationals. It solves no equation and runs no iteration. |
| CE-3 | **Conditions.** With `F` the one-step operator of the objective (the workload's average, the minimum or maximum over actions, or the policy's action) and values fixed at decided, target and `Prob0` states: **Upper**: `F(y) <= y` at every state, so the least fixed point, which is the value, is at most `y` (Knaster–Tarski). **Lower**: `x <= F(x)` at every state of the support, and every non-target state with `x > 0` has, under the workload, under the policy, or for a minimum under every action, a successor with positive probability, `x > 0` and a smaller rank; then no end component inside the support avoids the target, and `x` is at most the value. **Exact**: both, with equality, so `x` is the value. On a finite-horizon product the ranking is the remaining horizon, which the checker reads from the monitor state. **Expected reward**: the same conditions with the reward added to `F`, and the `+∞` states recomputed by the graph check. **LongRun**: per component, the gain–bias pair satisfies Puterman's multichain optimality equations for the reward `R · ([P] − ρ_C)` with gain 0, as equalities under the component's policy (a value attained) and as inequalities over every action (a value no scheduler exceeds), plus an `Exact`, `Lower` or `Upper` certificate for LR-3's expected component value. |
| CE-4 | **The bound follows.** The checker accepts a certificate that meets its conditions and puts the initial state's value on the claim's side of the threshold: `Lower` for a proof of `>= θ` or a refutation of `<= θ`; `Upper` for a proof of `<= θ`, or for a refutation of `>= θ` by an upper bound on `Pr(E)` below `θ`; `Exact` for either. For the `>=` quantile form (XF-2) the side is strict: an `Upper` certificate `y` for `max E[Y]` proves it when `y(s0) < 0`. When `y(s0) = 0`, the checker takes the actions tight under `y`, those with `Q_y(s, a) = y(s)`, `Q_y(s, a)` the one-step value of action `a` under `y`, and checks by graph search that no activated product state is reachable from the initial state using tight actions alone. A scheduler with `E[Y] = 0` has zero slack at every state it reaches, so it uses only tight actions; when activation is unreachable under them, no scheduler meets equality while activating, and the strict bound holds. Otherwise the certificate is rejected. A dyadic interval from interval iteration yields a `Lower` and an `Upper` certificate at once (UR-3). |
| CE-5 | **Verdict path.** EN-5 checks its own certificate before emitting it (AR-4). The driver then runs `check_probability_certificate` on the certificate that left S6c over E11 and settles `proved` only when it accepts; a rejection settles `inconclusive`, `CertificateRejected`; a checker stopped by a budget settles V-7. The checker refuses a certificate whose identity differs from the item's (ADR-026 CF-5). |
| CE-6 | **Size and cost.** A certificate holds one value per state it names, plus a rank and a policy choice where CE-3 needs them. The checker's cost is the re-enumeration plus one pass over the edges in exact rationals, linear in the product. It runs under the request's B-5 budgets. |
| CE-7 | **A small trusted base.** The checker lives in the qualified core, as the owner ruled for certificate checkers (ADR-029, References). The checker's arithmetic and its graph algorithms are separate code from EN-5's iterative and linear-algebra methods and are kept small. The product construction is shared with EN-1, whose translations already answer to SM-1 (ADR-018 SM-7). |

### 12. Limits

| ID | Rule |
| --- | --- |
| LM-1 | **`ExactProbLimits`.** EN-5's budgets are ADR-014 B-5 budgets of QSL's own provider: an embedded `ModelCheckLimits` (ADR-018), with its members, setting names and defaults unchanged (`max_states`, `max_transitions`, `max_automaton_states`) for the product; `max_iterations`, the interval-iteration sweeps; `precision_bits`, the starting dyadic precision, and `max_precision_bits`, the most it doubles to; `max_rational_bits`, the longest exact rational FH-1 and UR-4 keep; `max_policy_iterations`; and `max_witness_paths` (WS-3); plus time, the clause meter and cancellation. Each is set by the request, with a published default. No budget is a modelling limit, and the language fixes no cap on states, horizon, precision or iterations. |
| LM-2 | **Reaching a budget.** A budget reached before the method completes settles V-7: `Incomplete(PrecisionBudget{lower, upper})` when the interval still straddles the threshold, `Incomplete(ResourceExhausted)` naming the budget otherwise, unless XV-5's partial evidence already settles the item. The caller raises the budget and reruns. |
| LM-3 | **Identity.** Budgets and the method choices they drive are request settings recorded in the result; none enters the obligation identity. |

### 13. Probabilistic timed automata through digital clocks

ADR-026 RU-5 and SS-7 route `exact` evidence over a timed subject to EN-5.
EN-5 reads it through digital clocks when its constraints are closed; the
claim's scheduler resolves each delay (RU-6).

| ID | Rule |
| --- | --- |
| TA-1 | **The PTA reading.** A probabilistic claim over a timed subject (ADR-026 TS-1) is read on the digital route (TA-3) as a probabilistic timed automaton, and the claim's scheduler resolves each delay. **Under every scheduler** the scheduler chooses each delay and each scheduled identity, and random arguments are drawn by ADR-024 PM-1; a model that declares a `delay` distribution (ADR-026 SD-1) settles `unsupported`, `DelayDistribution{operation}`, on this form. **Under a workload `W`**, `W` resolves the scheduled identity (ADR-024 PM-2) and, through ADR-026 SD-3's race, every delay a `discrete` distribution gives: when every racing identity's delay is so resolved, the subject is a DTMC over digital timed states, checked exactly by §6 to §8. A delay no distribution gives stays the scheduler's choice, and EN-5 decides the claim over every resolution of those delays, the bound read on their minimum or maximum as SCH-1 reads it. A `uniform`, `uniform[a, b]` or `exponential` distribution has no exact digital reading and settles `unsupported`, `DelayDistribution{operation}`; EN-4 measures it (ADR-026 SS-7). |
| TA-1a | **Discrete delays on the grid.** A `discrete` distribution's delays scale to integers with the model's constants (TA-3). Under `W`, the race of ADR-026 SD-3 at a digital timed state draws each racing identity's delay from its distribution conditioned on its window, takes the smallest with ties ordered by `W`'s weights, and each such outcome is one probabilistic edge whose probability is an exact rational. A digital timed state with no racing identity and a bounded delay settles `unsupported`, `NotStochastic`, as ADR-026 SD-4 states. |
| TA-2 | **The closed condition.** Every atomic clock constraint in a guard, every time invariant and every timed interval of the claim is non-strict, and no constraint compares two clocks (ADR-026 CK-4). A strict constraint settles `unsupported`, `StrictClockConstraint{locus}`. |
| TA-3 | **Digital clocks.** EN-5 scales every constant to an integer by the common denominator of the model's and the claim's constants, as ADR-026 EZ-10 does, and explores the digital MDP of Kwiatkowska, Norman, Parker and Sproston: a timed state is the discrete state with an integer value per clock, capped at the clock's largest compared constant plus one; a delay move adds one unit to every clock below its cap and is admissible when the time invariants hold after it and the state is not urgent (ADR-026 CK-7); a discrete move is ADR-026 TS-2's step with integer clocks. For a closed, diagonal-free PTA the minimum and maximum of a reachability probability and of an expected time to a target are equal on the digital and the dense semantics, so the verdict is a verdict on the dense model, and every delay a witness takes is a valid dense delay. |
| TA-4 | **Admitted forms.** XF-4 reachability and until over state predicates; deadline events under the timed profile, `eventually[0, D] holds(P)` and `holds(A) until[0, D] holds(B)` with `D` closed, by one extra digital clock that no step resets, capped at `D + 1`; and XF-5's `expected elapsed until holds(B)`, with each delay move carrying one unit of time as its reward. Another TT-2 shape settles `unsupported`, `TimedFormShape`. |
| TA-5 | **Time divergence.** On the digital MDP every infinite path diverges exactly when no reachable cycle consists of discrete moves alone, which EN-5 checks by an SCC decomposition of the discrete-move subgraph. A reachable zero-delay cycle settles `unsupported`, `ZeroDelayCycle{states}`, because a scheduler that follows it is Zeno and lies outside ADR-026 TS-4's admitted behaviours. Time-locks are reported by ADR-026 TD-2's item, as for every timed subject. |
| TA-6 | **Evidence.** A witness scheduler chooses between a unit delay and an identity at each digital state (WS-1); under `W` it chooses only the delays no distribution gives. A timed path in a path set carries ADR-026 CT-1 delays and replays by CT-3; its last timed state has no admissible discrete step before the event's time horizon passes, which replay checks from the guards and time invariants. A certificate names digital product states, and the checker re-enumerates the digital MDP. |
| TA-7 | **Size.** The digital MDP has the discrete states times the product over clocks of (cap + 1) values. Its cost grows with the scaled constants, and `max_states` bounds it (LM-1). |

### 14. Reductions

| ID | Rule |
| --- | --- |
| RX-1 | **Symmetry.** Under an admitted symmetry declaration (ADR-021 SYM-5), when every random parameter's distribution and, under a workload, the workload are invariant under the group `G` (ADR-024 RX-1), and the claim's event is invariant under the binding's stabiliser, the quotient MDP has the same minimum and maximum, and the quotient chain the same value, as the original (Kwiatkowska, Norman and Parker). EN-5 then computes on the quotient, and the checker re-enumerates the quotient with the same canonicalisation; `ProofBasis` records the reduction (XV-1). A witness path on the quotient maps to a concrete path as ADR-021 states for counterexamples. Over a timed subject, a permutation moves objects with their clocks (ADR-026 §15). |

**RX-2 Proposed PT-2 row.** ADR-021 PT-2 gains:

| Form | Symmetry (admitted, SYM-5) | POR (C0–C3, POR-6) | State constraint |
| --- | --- | --- | --- |
| Probabilistic claim, exact (ADR-028 EN-5), under a workload or every scheduler | yes, when every random-parameter distribution and the workload are invariant under `G` and the event is invariant under the binding stabiliser (RX-1); certificates and witnesses over the quotient | no: a bounded event counts positions; under a workload a reduced successor set changes step probabilities; over every scheduler it removes schedulers from the range | no: a cut changes the measure |

### 15. Worked examples

ADR-024 §7's examples are finite-state, so EN-5 recomputes them exactly.
Spellings are illustrative, as in ADR-024. Each subject has one initial state
and no `over` parameter. Values are exact; decimals are rounded for reading.

#### 15.1 Probability bound: no fault in 1,000 steps (ADR-024 §7.1)

The product has the health flag at positions 0 to 1,000 and the two decided
states, about 1,003 product states. The exact value is
`(9,999,999 / 10,000,000)^1000 ≈ 0.99990000499483`, a rational whose
denominator is `10^7000`, 23,254 bits. One operation is enabled at every
state, so the value under the workload `Steady` and the minimum and maximum
over every scheduler coincide.

- With `max_rational_bits` at or above 23,254, FH-1 computes the exact value:
  `proved`, `ExactValue`. Below it, FH-2 reruns over dyadic intervals; at 64
  bits the interval width is at most `2 · 1001 · 2^-64 ≈ 1.1 · 10^-16`, the
  interval lies above 0.999, and the item settles `proved`,
  `ValueBounds{…, BackwardInduction{64}}`. The certificate is the dyadic lower
  vector, whose ranking is the remaining horizon.
- One pass over about 2,000 product edges replaces ADR-024's 9,210,341
  Okamoto samples or about 5,168 SPRT samples of 1,001 positions each, and
  the claim's `α`, `β` and `ι` are not read (SP-4).

#### 15.2 Quantile: p95 latency under 5 ms (ADR-024 §7.2)

Activation is at position 1 on every behaviour, so `Pr(activated) = 1` and
the conditional probability is the unconditional one. The product
accumulates `duration` from 0, saturating above the threshold.

- **At 5 ms.** `Pr(M <= 5 ms) = 24,233/25,000 = 0.96932`, at least 0.95:
  `proved`, `ExactValue{24233/25000}`, from FH-1 in exact rationals.
- **At 2 ms.** `Pr(M <= 2 ms) = 4,491/5,000 = 0.8982`, below 0.95: `refuted`.
  The evidence is a path set of one path: `request`, then `attempt` with
  `d = 3 ms` and `outcome = Ok`, probability `7/100 · 98/100 = 343/5,000 =
  0.0686`, on which `M = 3 ms > 2 ms`. Its probability exceeds
  `1 − 0.95 = 0.05`, so replay (WS-5) re-executes the two steps, checks both
  drawn values against their supports, recomputes `343/5000` and evaluates
  the event false. ADR-024 settles the same claim "measured: rejected"; EN-5
  settles it `refuted`.

#### 15.3 Availability: long-run and per window (ADR-024 §7.3)

- **Long-run.** The chain has one BSCC, `{up, down}`. Its stationary
  distribution is `π = (1,800/1,801, 1/1,801)`: `π_up · 1,999/2,000 + π_down ·
  9/10 = π_up`. The long-run availability is `1,800/1,801 ≈ 0.99944475`, at
  least 0.999: `proved`, `ExactValue{1800/1801}`. The `LongRun` certificate is in CE-3's form: component value
  `ρ_C = 1,800/1,801`, the reward `[up] − ρ_C` (`1/1,801` at `up`,
  `−1,800/1,801` at `down`), gain 0, bias 0 at `up` and `−2,000/1,801` at
  `down`; the checker verifies `b(up) = 1/1,801 + (1,999/2,000) · b(up) +
  (1/2,000) · b(down)` and `b(down) = −1,800/1,801 + (9/10) · b(up) +
  (1/10) · b(down)` exactly. ADR-024 measured this
  with `Asymptotic` coverage after about 93,080 regeneration cycles.
- **Per window.** The product carries the position and the count of down
  positions, saturating at 11, so at most 240,024 product states. The event
  "at most 10 of 10,001 positions down" has probability `≈ 0.95898`, below
  0.99: `refuted`. The violating mass, `≈ 0.04102`, is spread over very many
  paths, so the evidence is a subsystem certificate: a dyadic lower vector
  for `Pr(not E)` with value above 0.01, which the checker verifies over its
  support.

#### 15.4 A claim over every scheduler

```text
model Link = "example/link" …;   // object m: Msg { delivered: Bool, attempts: Int[0, 2] }

operation Msg::send_a(random lost: Bool ~ { true: 1, false: 9 })
  pre  not self.delivered and self.attempts < 2
  post self.attempts = pre(self.attempts) + 1 and self.delivered = not lost;
operation Msg::send_b(random lost: Bool ~ { true: 1, false: 4 })
  pre  not self.delivered and self.attempts < 2
  post self.attempts = pre(self.attempts) + 1 and self.delivered = not lost;
terminal when self.delivered or self.attempts = 2;

workload Even on Link { weight Msg::send_a = 1; weight Msg::send_b = 1; }

probabilistic Deliver using ev on Link under every scheduler on origin {
  probability >= 0.97 [ eventually[0,2] holds(m.delivered) ]
}
```

At the state after one lost message, `send_a` delivers with `9/10` and
`send_b` with `4/5`. Backward induction gives a minimum of `4/5` there and
`min(9/10 + 1/10 · 4/5, 4/5 + 1/5 · 4/5) = min(49/50, 24/25) = 24/25` at the
start; the maximum is `99/100`. Under `Even` the value is `17/20` after one
loss and `391/400 = 0.9775` at the start.

- `Deliver` settles `refuted`. The witness scheduler takes `send_b` at both
  live states; under it, the path `send_b` (lost), `send_b` (lost) has
  probability `1/25 = 0.04 > 1 − 0.97`, so a path set of one path suffices.
  Replay checks that each step takes the scheduler's choice and recomputes
  `1/25`.
- With threshold 0.95 it settles `proved`, `ExactValue{24/25}`, with an
  `Exact` certificate: values `24/25`, `4/5`, 1 at delivered states and 0 at
  the two-loss state; the checker verifies the minimum over both actions at
  each live state.
- The same claim `under Even` settles `proved`, `ExactValue{391/400}`. The
  claim over every scheduler fails where the workload's claim holds, because
  an adversary may always choose the worse link.

#### 15.5 A closed probabilistic timed automaton

```text
model Retx = "example/retx" …;   // object m: Msg { delivered: Bool, x: Clock }
time dense unit ms;

operation Msg::send(random lost: Bool ~ { true: 1, false: 9 })
  pre  not self.delivered and self.x >= 1 ms
  post self.delivered = not lost and self.x = 0;
time invariant when not self.delivered { self.x <= 2 ms }
terminal when self.delivered;

probabilistic Deadline using timed on Retx under every scheduler on origin {
  probability >= 0.99 [ eventually[0 ms, 4 ms] holds(m.delivered) ]
}
probabilistic MeanTime using timed on Retx under every scheduler on origin {
  expected elapsed until holds(m.delivered) <= 3 ms
}
```

Every constraint is closed and no operation declares a delay distribution,
so EN-5 takes the digital route (TA-1 to TA-3): `x` takes values 0 to 2, the
deadline clock 0 to 5, and `send` needs one delay unit after each reset, so
no zero-delay cycle exists (TA-5). The scheduler chooses when to send in
`[1, 2]` ms after each reset.

- **`Deadline`.** The minimum waits until `x = 2` each time, so two attempts
  fit by 4 ms: `1 − (1/10)² = 99/100`. The maximum sends at `x = 1`, four
  attempts: `9,999/10,000`. The minimum equals the threshold, which an
  interval method never decides; FH-1 in exact rationals settles `proved`,
  `ExactValue{99/100}`. At threshold 0.995 the item settles `refuted`: the
  witness scheduler delays to `x = 2` and sends, and the path with two losses
  at 2 ms and 4 ms has probability `1/100 > 1 − 0.995`; at its last state the
  next `send` needs `x >= 1`, after 4 ms, which replay checks (TA-6).
- **`MeanTime`.** Every attempt succeeds with probability `9/10`, and the
  time invariant forces an attempt by `x = 2`, so the target is reached with
  probability 1 under every scheduler. The maximum expected time is
  `2 ms · 10/9 = 20/9 ms ≈ 2.22 ms` and the minimum `10/9 ms`: `proved`,
  `ExactValue{20/9 ms}`.
- **A strict variant.** With `self.x > 1 ms` in the guard the item settles
  `unsupported`, `StrictClockConstraint`, and EN-4 measures it. ADR-026 §11's
  `RareLate` states `under Even` with `exponential` delays, which have no
  exact digital reading, so on EN-5 it settles `unsupported`,
  `DelayDistribution`, and stays EN-4's. With every delay `discrete` on the
  grid it would be a DTMC under `Even` and exact on EN-5 (TA-1a).

#### 15.6 Termination against a fair adversary

```text
model Coin = "example/coin" …;   // object p: Proc { done: Bool }, initially not done

operation Proc::flip(random heads: Bool ~ { true: 1, false: 1 })
  pre  not self.done
  post self.done = heads;
operation Proc::wait
  pre  not self.done
  post self.done = pre(self.done);
terminal when self.done;

probabilistic Terminates using inf on Coin under every scheduler on origin {
  probability >= 0.99 [ eventually holds(p.done) ]
}
probabilistic FairTerminates using inf on Coin under every scheduler
    fair strong Proc::flip on origin {
  probability >= 0.99 [ eventually holds(p.done) ]
}
```

- **`Terminates`.** The scheduler that always takes `wait` never reaches
  `done`: the minimum is 0 and the item settles `refuted`. The witness
  scheduler takes `wait` at the one live state, and the evidence is an
  `Upper` certificate with value 0 on the chain it induces.
- **`FairTerminates`.** The live state with action `wait` is an end
  component, but `flip` is enabled there and not in it, so FS-4 removes the
  state and no fair end component remains: `U` is empty and the infimum is
  `1 − 0 = 1` (FS-6). The item settles `proved`, `ExactValue{1}`, with an
  `Exact` certificate whose objective names the fairness set; the checker
  recomputes that no fair end component exists. `fair weak Proc::flip` gives
  the same value, since `flip` is enabled at every state of that component.
- The same reading decides termination of a randomized consensus protocol
  against a fair adversary: the adversary schedules processes as it likes,
  but cannot starve one forever.

### 16. Downstream impact and sequencing

| ID | Repository | Change |
| --- | --- | --- |
| DS-1 | QSL | S3: the forms XF-4 and XF-5 with their classification, fairness sets on every-scheduler claims (FS-1), claims without confidence parameters (SP-4), the exact-only rule (XF-7), the timed route's closed-condition and shape checks (TA-2, TA-4). Layer 5 `exact_probabilistic`: the probabilistic product (§2), backward induction, qualitative precomputation, MEC and BSCC decomposition, maximal fair end components and fair schedulers (§3a), interval iteration, exact policy iteration and linear solves, long-run values, the digital MDP, dyadic and exact arithmetic, `ExactProbLimits`, witness schedulers, path sets and certificates. `qsl-replay`: `ProofBasis::ExactValue` and `ValueBounds`, `PrecisionBudget`, `CertificateRejected`, the unsupported causes, `ProbabilisticCounterexample`, `ReplaySource::ProbabilisticWitness`, `replay_probabilistic_witness` and `check_probability_certificate`. The EN-5 provider manifest with evidence kind `exact`. |
| DS-2 | CG | The `probabilistic-satisfaction` arm routes `exact` items to EN-5 (SP-3); the map from EN-5's outcome into `TerminalValue`. |
| DS-3 | Driver | Runs an item routed to EN-5 in process, runs the certificate check or the witness replay over E11 before writing the terminal record (SP-5). |
| DS-4 | QSpec | §17. |

**Sequencing.** EN-5 is built after EN-1 and EN-4. It needs EN-1's product,
monitors, edge retention and SCC decomposition (ADR-018 §7 step 3), and the
S3 and FR-120 work EN-4 brings: random parameters, workloads, rewards, the
probabilistic forms and the step probability (ADR-024 DS-1). The timed route
needs ADR-026's timed subjects and its digitization (EZ-10). Exact checking
follows v1.

### 17. What QSpec must specify

| ID | Item | Where |
| --- | --- | --- |
| QS-1 | The evidence kind `exact` for `probabilistic-satisfaction`, its advertisement and the no-substitution rule (ADR-024 QS-6), and claims without confidence parameters as exact-only (SP-4, XF-7) | QSpec FR-290, FR-331, FR-407, FR-408 |
| QS-2 | `under every scheduler`: minimum and maximum over every history-dependent randomized scheduler, the scheduler's choice after a draw with several post-states, and the statement that memoryless deterministic schedulers on the product attain both (SCH-1 to SCH-3) | QSpec FR-406 |
| QS-3 | The forms with exact evidence: the probabilistic product with a bounded monitor and accumulators (PR-4, PR-5), the quantile threshold transform (XF-2), the mean of a fraction (XF-3), unbounded reachability, until and always over state predicates (XF-4), expected reward to a target with `+∞` (XF-5), long-run fractions by bottom components with `ZeroWeightComponent` (§8) | QSpec FR-411, FR-407, FR-090 |
| QS-4 | Result content: `ExactValue` and `ValueBounds` onto FR-331 `proved` with FR-243 `closed-scope`, one entry per initial state and binding with minimum or maximum, `PrecisionBudget`, `CertificateRejected` and the unsupported causes (XV-1 to XV-6) | QSpec FR-411, FR-331, FR-243 |
| QS-5 | The witness wire: `WitnessScheduler`, `ProbabilisticCounterexample` with path-set and subsystem evidence, and the replay rules of WS-5 | QSpec FR-413 |
| QS-6 | The certificate wire: `ProbabilityCertificate`, its kinds and the checker's conditions (CE-1 to CE-4), including what the checker recomputes | QSpec FR-412 |
| QS-7 | `ExactProbLimits` in the request (LM-1) | QSpec FR-411 |
| QS-8 | The digital route over timed subjects: the closed condition, the digital MDP, delays resolved by the claim's scheduler with `discrete` delays under a workload as a DTMC (TA-1, TA-1a), the admitted forms and the zero-delay-cycle check (TA-2 to TA-5), in step with ADR-026's overlap items | QSpec FR-421, FR-420, FR-161, FR-181 |
| QS-9 | Conformance vectors, each with its exact value, verdict and evidence that must check (the exact witness is not compared, since engines may return different valid ones): (a) §15.1, the exact rational and a dyadic `ValueBounds` proof; (b) §15.2, `24233/25000` proved at 5 ms and `4491/5000` refuted at 2 ms with a one-path set; (c) §15.3, `1800/1801` proved with its gain–bias certificate, and the per-window claim refuted with a subsystem certificate; (d) §15.4, minimum `24/25`, maximum `99/100`, workload `391/400`, the refutation with its witness scheduler, and the proof with its `Exact` certificate; (e) §15.5, `99/100` proved at equality, `20/9 ms`, the refutation at 0.995, and the strict variant `unsupported`; (f) checker refusals: a certificate that violates one inequality (`CertificateRejected`), a missing rank on a lower certificate, a path set that is not prefix-free, a witness step that departs from its scheduler, a drawn value outside its support; (g) `NotMarkov` under a workload, a delay distribution and a zero-delay cycle on the timed route; (h) a `PrecisionBudget` stop | QSpec TC-360 to TC-365 and TC-374, beside TC-200 and TC-210 |
| QS-10 | Fair schedulers: the fairness set on every-scheduler claims, fair schedulers as probability-1 fairness, fair end components, the infimum and supremum per form, the randomized witness scheduler and its fairness check, and a vector for §15.6 (FS-1 to FS-9) | QSpec FR-406, FR-411, FR-413 |

### 18. Rulings on the draft's questions

Ruled on 2026-10-01: RU-2 and RU-3 by the owner, the others by the plan
lead, consistent with the owner's earlier rulings.

| ID | Question | Ruling | Rationale | Where it lands |
| --- | --- | --- | --- | --- |
| RU-1 | Whether expected reward and expected time to a target have a statistical route | **Exact-only.** | The exact engine detects an infinite expectation from the graph; a sample of finite behaviours cannot | XF-5, XF-7 |
| RU-2 | Whether a claim may omit its confidence parameters | **Confidence parameters are for statistical evidence only.** When present they stay in the claim and its obligation identity; a claim without them is exact-only, and statistical checking refuses it (ADR-024 PF-10, RU-6) | An exact verdict decides the bound and reads no confidence | SP-4; ADR-024 PF-10, SV-5 |
| RU-3 | Whether every-scheduler claims may assume fairness | **Specified now,** over fair schedulers by Baier and Kwiatkowska's fair end components | Termination of a randomized protocol holds only against a fair adversary | SCH-4, §3a, FS-1 to FS-9, §15.6 |
| RU-4 | When the scheduler resolves residual nondeterminism | **After the draw,** as in a standard MDP | The scheduler sees the drawn vector, as the model's nondeterminism follows the draw | SCH-2 |
| RU-5 | Whether the weighted long-run fraction is kept over every scheduler | **Kept.** | Time-weighted availability needs it | XF-6, LR-2 |
| RU-6 | Who resolves delays over a timed subject under a workload | **The workload resolves both the action and the delay choice** when it gives the delays: the subject is then a DTMC, checked exactly on EN-5. Delay nondeterminism left over is decided by its minimum and maximum | One claim under one workload has one measure; the exact engine computes it when it is rational | TA-1, TA-1a; ADR-026 SS-7 and RU-5 on acceptance |

## Consequences

- Claims over every scheduler, rare-event bounds, thresholds close to the
  true value and long-run availability get proofs and refutations, not
  measurements.
- Every EN-5 proof carries a certificate and every refutation a witness, and
  a small checker verifies either in exact rationals with no engine present.
  A proof settles only after its certificate is accepted.
- Exact and statistical results for one claim are separate items with
  separate records: `proved` or `refuted` with a `ProofBasis`, or `measured`
  with its method and coverage.
- No floating point enters a verdict; results are identical on every
  platform.
- A value equal to its threshold is decided, by exact backward induction or
  exact policy iteration.
- Closed probabilistic timed automata get exact deadline probabilities and
  expected times over every scheduler; strict constraints and delay
  distributions stay statistical.
- EN-5's cost is the reachable product, which it retains, plus exact or
  dyadic arithmetic over it; every limit is a caller-set budget.

## Amendments made with this record

Each amended text carries an "Amended by ADR-028" note.

- ADR-024 PM-3: under every scheduler, several post-states for one draw are a
  scheduler choice (SCH-2). PM-8: EN-5 settles `under every scheduler`.
  PF-8: the unbounded forms XF-4 and XF-5 with exact evidence. PM-7: a
  claim `under every scheduler` may state a fairness set (FS-1).
  SV-5: exact-only forms under `statistical` evidence. SV-9: EN-5 is the
  exact backend, with `ExactValue` and `ValueBounds`. §8 sequencing: EN-5
  after EN-1 and EN-4.
- ADR-018 §1 `ProofBasis` paragraph: `ExactValue`, `ValueBounds`,
  `PrecisionBudget` and `CertificateRejected` (XV-1, XV-4). §5 CX-4: EN-5's
  witness and certificate replay with no engine present.
- ADR-014 §1 B-5: EN-5's budget type is `ExactProbLimits` (LM-1, LM-2).
- ADR-013 O-16 proof column: the EN-5 verdicts and causes. O-24 public type:
  the `ProofBasis` members and causes.
- `spec/spec.md`: the index row for this record.

## Amendments to make on acceptance

- ADR-021 PT-2: RX-2's row.
- ADR-011 §1 and §6.1: S6c runs EN-5; layer A, crate `qsl-analyze`, gains
  `exact_probabilistic` (ADR-029 CB-3); E11 carries EN-5 certificates and
  probabilistic witnesses; layer 6 `replay` gains
  `replay_probabilistic_witness` and `check_probability_certificate`.
- ADR-026 SS-7, RU-5 and §15: the digital route as TA-1 to TA-7 state it.
  Under a workload that gives the delays, the workload resolves the action
  and the delay choice, so the subject is a DTMC checked exactly on EN-5;
  delay nondeterminism left over is decided by its minimum and maximum; a
  `uniform`, `uniform[a, b]` or `exponential` delay stays EN-4's. The causes
  `DelayDistribution`, `StrictClockConstraint`, `ZeroDelayCycle` and
  `TimedFormShape`.
- ADR-026 SD-1: an operation that declares no `delay` member has a free,
  nondeterministic delay, never an implied distribution, so the leftover
  delay nondeterminism of TA-1 is defined in one place (team-leader
  decision; see References).

## Alternatives Considered

- **Lowering to PRISM or Storm.** Not adopted. Their default results are
  floating point with no certificate, and their exact and sound modes give a
  correct number QSL cannot check without the tool. QSL needs its own witness
  replay and certificate checker in either case, and that checker is about
  half of a native engine. The Modest Toolset's PTA engine is closed source,
  so it cannot be a product dependency.
- **Plain value iteration with a convergence threshold.** Rejected. It
  stops with no bound on its error and can stop far from the value
  (Haddad and Monmege); interval iteration gives both bounds at every sweep.
- **Floating point with directed rounding.** Rejected. It depends on the
  platform's rounding modes and needs care in every operation (Hartmanns);
  dyadic intervals in integer arithmetic give the same soundness with
  results identical everywhere.
- **Exact rationals only.** Rejected. Denominators grow with every step
  (§15.1's value has 23,254 bits), so dyadic intervals decide most claims at
  a fixed width and exact arithmetic is kept for the cases that need it.
- **A linear-programming solver in exact rationals.** Not adopted as the
  primary method. Policy iteration needs only linear solves, which the
  fraction-free elimination provides; an exact simplex adds a component and
  gives the same values.
- **Zone-based PTA analysis.** Not adopted for EN-5. Forward zone
  reachability bounds only the maximum from above; backward reachability
  needs non-convex zone sets; game-based abstraction refinement needs a
  stochastic-game solver. A strict constraint settles `unsupported`, and
  ADR-026's EN-6 owns the zone code.
- **Exact checking of real-valued delay distributions.** Not adopted. Under
  exponential or general delays, time-bounded probabilities are not rational,
  and only bounds are available; those claims stay statistical (ADR-026
  SS-7).
- **No fairness on every-scheduler claims.** Rejected (RU-3). An adversary
  that starves an enabled action forever refutes the termination of every
  randomized protocol, for a reason the protocol's environment excludes.
- **Fairness as a restriction to memoryless schedulers.** Rejected. A
  memoryless deterministic scheduler can starve an action, and restricting
  to memoryless randomized schedulers with full support removes adversaries
  a fair environment allows; probability-1 fairness (FS-2) is the standard
  reading.
- **Counterexamples as the full induced chain recomputed by replay.**
  Rejected. Replay would solve the chain again; a path set or a subsystem
  certificate is checked in one pass.
- **`ProofBasis` members named after probabilities.** Not taken. The same
  members carry expected rewards and expected times, so they are named after
  what they state: an exact value or sound bounds.

## References

- Owning ticket: Linear QSL-371, which records the rulings of §18. QSpec
  half: Linear STD-137 (QSpec FR-405 to FR-414) and, for the digital
  route, Linear STD-139 (QSpec FR-421). Research and the owner's ruling that EN-5
  is a native engine specified now: Linear RES-53. Zone-based timed analysis: Linear RES-54.
- ADR-029, the qualified core, its certificate checkers and the crate
  layout (RU-2, CB-2, CB-3), with the uncertified-proof rule the owner
  ruled: Linear QSL-390, on its own draft branch.
- Team-leader decision of 2026-10-01 on ADR-026 SD-1: an operation with no
  `delay` member has a nondeterministic delay, resolved by its minimum or
  maximum under a workload (RU-6), never an implied distribution.
- Sibling records: ADR-018 (QSL-366), ADR-021 (QSL-368, state-space
  reduction), ADR-022 (QSL-369, possible properties), ADR-024 (QSL-371,
  statistical and probabilistic properties), ADR-026 (QSL-373, dense time;
  its RU-5 and SS-7 route exact claims over timed models here).
- M. L. Puterman, *Markov Decision Processes*, Wiley, 1994 (memoryless
  deterministic optimality; multichain optimality equations, chapter 9).
- C. Baier and J.-P. Katoen, *Principles of Model Checking*, MIT Press, 2008
  (chapter 10: DTMCs, MDPs, `Prob0`/`Prob1`, end components).
- L. de Alfaro, *Formal Verification of Probabilistic Systems*, PhD thesis,
  Stanford, 1997 (end components; long-run averages in MDPs).
- S. Haddad and B. Monmege, "Interval iteration algorithm for MDPs and IMDPs",
  *Theoretical Computer Science*, 2018.
- C. Baier, J. Klein, L. Leuschner, D. Parker and S. Wunderlich, "Ensuring the
  reliability of your model checker: interval iteration for Markov decision
  processes", CAV 2017.
- T. Quatmann and J.-P. Katoen, "Sound value iteration", CAV 2018.
- A. Hartmanns, "Correct probabilistic model checking with floating-point
  arithmetic", TACAS 2022.
- K. Chatterjee, T. Quatmann et al., fixed-point certificates for MDP model
  checking with a verified checker, TACAS 2025.
- F. Funke, S. Jantsch and C. Baier, "Farkas certificates and minimal
  witnesses for probabilistic reachability constraints", TACAS 2020.
- E. H. Bareiss, "Sylvester's identity and multistep integer-preserving
  Gaussian elimination", *Mathematics of Computation*, 1968.
- J. Fearnley, "Exponential lower bounds for policy iteration", ICALP 2010.
- W. Dinkelbach, "On nonlinear fractional programming", *Management Science*,
  1967.
- C. Baier and M. Kwiatkowska, "Model checking for a probabilistic branching
  time logic with fairness", *Distributed Computing*, 1998 (fair
  adversaries; fair end components).
- M. Kwiatkowska, G. Norman, D. Parker and J. Sproston, "Performance analysis
  of probabilistic timed automata using digital clocks", *Formal Methods in
  System Design*, 2006.
- M. Kwiatkowska, G. Norman and D. Parker, "Symmetry reduction for
  probabilistic model checking", CAV 2006.
- M. Kwiatkowska, G. Norman and D. Parker, "Probabilistic model checking and
  autonomy", *Annual Review of Control, Robotics, and Autonomous Systems*,
  2022 (arXiv 2111.10630).
- A. Legay and S. Sedwards, "Lightweight Monte Carlo algorithm for Markov
  decision processes", 2014.
