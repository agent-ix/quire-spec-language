---
id: ADR-023
title: "Hyperproperties over every behaviour of a model"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
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
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-191
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-243
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
---
# ADR-023: Hyperproperties over every behaviour of a model

## Context

**What QSpec specifies.** QSpec FR-191 defines relational properties and
hyperproperties over **declared finite trace sets**: a trace variable ranges
over one closed trace population, quantifiers are evaluated left to right,
alignment between traces is an explicit total relation, and no execution is
correlated by local name alone. The shared grammar has two forms:

- `relation <name> using <profile> over (x: Q, y: Q, …) { … }`, comparing
  two or more named executions;
- `hyper <name> using <profile> over t in Q bounded N forall trace a exists
  trace b … { … }`, quantifying over a finite trace domain of at most `N`
  traces.

QSpec's checked-package v2 has a `hyperproperty` claim node form. QSpec
FR-290's claim-form table assigns `temporal-satisfaction` to every temporal
requirement under a finite-trace or infinite-trace profile and has no
hyperproperty row.

**What QSL does today.** `qsl-cst` parses both forms (`RelationClause`,
`HyperClause`, `TraceDomain`, `Quantifier`). The S2 value form builder
refuses each of them as an unrepresented production, so no relation or
hyperproperty reaches S3.

**What ADR-018 provides.** Behaviours of a model subject as maximal paths with
terminal stutter (SM-2 to SM-4), weak fairness per operation (FA-1 to FA-6),
the trace evaluator as the one semantics (SM-1), the explicit-state product
EN-1 with SCC-based fair-cycle detection, verdict kinds V-1 to V-8, and
counterexamples that replay through `ModelSystem` (CX-1 to CX-5), and
EN-1's run limits as `ModelCheckLimits`, ADR-014 B-5 budgets of QSL's own
provider that a request sets (ADR-018 §3, IV-6).

**Why every behaviour.** Security properties such as noninterference compare
two runs: whatever the secret, runs with the same public inputs give the same
public outputs. Efficiency claims do too: "power-saving mode uses no more
power than normal mode for the same inputs". Both quantify over every
behaviour of a model, which a finite trace set does not cover. Writing the
self-composition by hand, as TLA+ authors do, squares the state space inside
the model and hides the property.

**The literature this record draws on.** HyperLTL (Clarkson, Finkbeiner,
Koleini, Micinski, Rabe and Sánchez) quantifies over traces and reads
indexed atoms in lockstep. Alternation-free formulas reduce to LTL over a
self-composition (Barthe, D'Argenio and Rezk; MCHyper, by Finkbeiner, Rabe
and Sánchez). For `∀∃` formulas, the automata-theoretic method checks
language inclusion and needs Büchi complementation (AutoHyper, by Beutner and
Finkbeiner); the strategy-based method lets an existential player answer the
universal player's moves and is sound and incomplete, made complete with
prophecies (Coenen, Finkbeiner, Sánchez and Tentrup; Beutner and Finkbeiner);
bounded model checking unrolls to depth `k` and solves a QBF (HyperQube, by
Hsu, Sánchez and Bonakdarpour). Safety properties have a safety closure
automaton in which every run of a trimmed Büchi automaton is accepting
(Kupferman and Vardi).

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. Item ids `HS-`, `HM-`, `HP-`, `HC-`, `HV-`, `HX-`, `PH-`,
`RU-`, `PA-`, `CS-`, `XC-`, `SE-` and `QS-` are local to this record. Other artifacts cite them as `ADR-023 HM-3`.
ADR-018 ids (`SM-`, `EN-`, `V-`, `FA-`, `CX-`, `DL-`, `IV-`) keep their
meaning and are cited with their record.

All state counts in §8 are hand enumeration; no QSL engine produced them.

## Decision

### 1. Syntax

The compiler builds the self-composition; the author writes the property. The
spellings below are illustrative; QSpec's shared grammar owns them (QS-1).

| ID | Rule |
| --- | --- |
| HS-1 | **Behaviour domain.** A `hyper` clause names its domain after `over`. The FR-191 domain `t in Q bounded N` keeps its meaning: a declared finite trace population. The second domain form is `behaviours`, optionally followed by object parameters (HS-3): the trace variables range over behaviours of model subjects (ADR-018 SM-2, SM-3). |
| HS-2 | **Binding trace variables to models.** Under `behaviours`, every quantifier names a model alias: `forall trace a of V`. Two quantifiers that name the same alias range independently over the same subject, which is the self-composition. Quantifiers that name different aliases range over different subjects, which compares two models. The request supplies one subject (ADR-018 SM-2: initial states, universes, `ProofBound`s) per distinct alias. Under `behaviours` a quantifier without `of`, and under a finite trace domain a quantifier with `of`, refuse at S3, located at the quantifier. |
| HS-3 | **Object parameters.** `behaviours (v: V::Vault)` binds an object of a population of alias `V`'s model. The clause has one instance per key of that population's universe, as ADR-018 QS-5 states for a temporal clause's `over` parameter. Within an instance, `v` denotes the object with that key in every trace of that alias. Objects are correlated across traces by universe key, which is explicit and is the same key in every copy of one subject. A clause over two aliases names one parameter per alias it reads, and its instances range over the product of their universes. |
| HS-4 | **Indexed atoms.** A state atom in the body names the trace each member read comes from: `holds(v.l @ a = v.l @ b)`. Every model member read in an atom carries exactly one trace variable, and one atom may read several variables. A member read through a variable bound to another model refuses `ill_typed` at the read. |
| HS-5 | **Input matching.** An optional `match { μ }` block states which tuples of behaviours are compared. `μ` is a Boolean over **step labels**: `a.step.op` is the operation of the step into the current position, or `stutter` for the terminal stutter step (ADR-018 SM-4); `a.step.receiver` is its receiver key; `a.step.O.p` is the argument `p` of operation `O`, of type `Option<T>` for `p: T`, present exactly when the step is an `O` step. `μ` reads no state. It is written as a conjunction, and S3 splits it into its conjuncts (HM-3). A step label is the FR-181 transition identity of the step, so "same inputs" is the operation and arguments the environment chose; the post-state the model chose is not an input. |
| HS-6 | **Fairness per trace variable.** Each quantifier carries its own fairness set over its own model's operations: `forall trace a of V fair { weak V::Vault::step }`. Each constraint is an ADR-018 FA-1 constraint with ADR-019's kinds, `whole` unmarked (ADR-018 FA-6). The empty set is valid. |
| HS-7 | **Profile.** A `hyper` clause over `behaviours` selects `quire.temporal.infinite-trace/v1`, and its body is an infinite-trace formula over indexed atoms, interval operators admitted (ADR-018 IV-1). A clause over `behaviours` under any other temporal profile settles V-8, `unsupported-requested-capability`, at negotiation, as ADR-018 §1 settles a fixed-sample clause over a model subject. |
| HS-8 | **`relation` over model executions.** An execution binding may name an operation of a model alias: `relation Det using v over (x: V::Vault::step, y: V::Vault::step) { … }`. Each variable ranges over the transitions of that operation from the reachable states of its alias's subject. The body is a Boolean under the value profile that reads, per execution, its arguments (`x.i`), its receiver's pre-state (`pre(x.self.l)`), post-state (`x.self.l`) and result (`x.result`). Every comparison is between named executions (QSpec FR-191). An execution binding to any other qualified name keeps FR-191's finite reading. |
| HS-9 | **Alignment.** A `hyper` clause over `behaviours` aligns its traces in lockstep (HM-2) unless it carries `align skip { O, … }`, which names operations of the clause's model aliases whose steps are skipped: the traces align on their projections onto the remaining steps (§13, PA-1 to PA-7). |

Noninterference over one model, the same model twice:

```text
profile inf = "quire.temporal.infinite-trace/v1" …;
model V = "example/vault" …;

hyper NonInterference using inf over behaviours (v: V::Vault)
  forall trace a of V
  forall trace b of V
  match { a.step.op = b.step.op and a.step.Vault::step.i = b.step.Vault::step.i }
{
  always holds(v.l @ a = v.l @ b)
}
```

Power saving over two models, two different aliases:

```text
model S = "example/device-saving" …;
model N = "example/device-normal" …;

hyper SavesPower using inf over behaviours (d: S::Device, e: N::Device)
  forall trace s of S
  forall trace n of N
  match { s.step.Device::tick.load = n.step.Device::tick.load }
{
  always holds(d.used @ s <= e.used @ n)
}
```

### 2. Semantics

| ID | Rule |
| --- | --- |
| HM-1 | **Domain of a trace variable.** A variable `a of M` ranges over the behaviours of `M`'s subject (ADR-018 SM-3, SM-4) that satisfy `a`'s fairness set (ADR-018 FA-3, ADR-019 SF-3). Fairness is read on the variable's own behaviour: enabledness and taking come from its own model states and steps, whatever the other traces do. |
| HM-2 | **Lockstep alignment.** A tuple of behaviours is read position by position: joint position `j` is position `j` of every component, and joint step `j` is the tuple of the components' steps into position `j`. Every behaviour is infinite, because a terminal state is stutter-extended (ADR-018 SM-4), so every joint position exists for every component. Lockstep is the explicit total alignment QSpec FR-191 requires: it pairs positions by their index in the model's step sequence, which is the event-position sequence authority of the profile (ADR-018 SM-3). |
| HM-3 | **Quantifiers and match.** For a prefix with universal variables `u` and existential variables `e` (in that order, HP-3), S3 splits `μ` into `μ_U`, the conjuncts that read universal variables only, and `μ_E`, the conjuncts that read an existential variable. The clause holds exactly when, for every tuple of behaviours of `u` on which `μ_U` holds at every joint step, there is a tuple of behaviours of `e` on which `μ_E` holds at every joint step and the body holds on the joint behaviour. With no existential variable this is: every tuple matched by `μ_U` satisfies the body. `μ` constrains joint steps `j >= 1`; position 0 has no step, and a premise on initial states is written in the body. Quantifier order is preserved (QSpec FR-191-AC-4). |
| HM-4 | **Body.** The body's truth on a joint behaviour is the value SM-1's evaluator returns, with each indexed atom reading its component's observation at the joint position (ADR-018 SM-3). On a tuple of lassos the joint behaviour is ultimately periodic: its stem is the longest component stem and its period the least common multiple of the loop lengths, under ADR-014 TR-2's prefix-then-loop indexing per component, each component
read by ADR-018 SM-8, so past operators see the same history at a loop
position and at its unrolled copies. Evaluation charges ADR-014 TR-5 work per (temporal node, joint position) visit. |
| HM-5 | **Terminal stutter.** A component at a terminal state takes its stutter step at every later joint step, with step label `stutter` (HS-5); the stutter step is outside every fairness constraint (ADR-018 SM-4). A `μ` that requires equal operations matches a stuttering component only with another stuttering one. |
| HM-6 | **Deadlocks.** A hyper or relation item reports no deadlock. The request writer adds the deadlock-freedom item of each distinct subject the item names (ADR-018 DL-3), so a reachable deadlock is reported once per subject, by its own item. |
| HM-7 | **Vacuity.** A clause with a non-empty `μ_U` and no tuple of fair universal behaviours on which `μ_U` holds at every joint step is vacuous. It settles V-6 `VacuousMatch` (HV-4), never `proved`. With `μ_U` empty the universal domain is never empty, because a finite subject with an initial state always has a fair behaviour (ADR-018 §4); a subject with no initial state settles V-6 `NoInitialState` as ADR-018 states. |
| HM-8 | **Interval operators.** An interval operator in the body counts joint positions (ADR-018 IV-2, IV-7). ADR-018 IV-3's translation applies over indexed atoms unchanged. |
| HM-9 | **Step relation.** A `relation` over model executions (HS-8) holds exactly when its body is true for every tuple `(t_1, …, t_n)`, where `t_i` is a transition of variable `i`'s operation that leaves a reachable state of its subject: an edge of the reachable graph, with its pre-state, arguments, result and post-state. Every reachable transition lies on some behaviour, so this is the hyperproperty "for every tuple of behaviours and every tuple of positions at which each takes its named operation, the body holds", with positions chosen independently per trace. |

### 3. Property forms

S3 classifies each clause into one form from its quantifier prefix, its body
and its fairness sets, and records the form beside the requirement record, as
ADR-018 §1 records a temporal form. The requirement's kind is
`temporal-satisfaction`, with extent `Unbounded` (ADR-014 A-3).

| ID | Form | Shape | Engines (§4) |
| --- | --- | --- | --- |
| HP-1 | Step relation | `relation` over model executions (HS-8, HM-9) | model claim: EN-1 (exhaustive). Code claim, when the operations are bound to code: a two-call Kani harness on the IR/CG side (§14) |
| HP-2 | Universal hyperproperty | `hyper` over `behaviours`, every quantifier `forall` (`∀^n`, `n >= 1`), lockstep, any infinite-trace body, any fairness sets | EN-1 (exhaustive); EN-2 (refutation, bounded-to-`k`); EN-3 (inductive, safety body) |
| HP-3 | `∀∃` safety hyperproperty | prefix `∀^n ∃^m` with `n >= 0`, `m >= 1` and `n + m >= 2`; body in ADR-014 A-4's safety fragment, interval operators included (ADR-018 IV-4); every existential variable has the empty fairness set | EN-1 (exhaustive) |
| HP-4 | Other hyperproperty | an existential variable before a universal one; an existential variable, with a universal or a second existential beside it, whose body is outside the safety fragment or whose fairness set is non-empty; `align skip` with an existential variable or a body outside the safety fragment | none: V-8, `unsupported-requested-capability` |
| HP-5 | Single-existential claim | prefix of exactly one `exists`, no `align skip`, any infinite-trace body, any fairness set; with `align skip` the clause is HP-4 | ADR-022's possible family (§15) |
| HP-6 | Projection-aligned universal hyperproperty | every quantifier `forall`, with `align skip` (HS-9), a body in ADR-014 A-4's safety fragment and no fairness set on any variable (PA-1, PA-3, §13) | EN-1 (exhaustive) |

**The admitted fragment is HP-1, HP-2, HP-3, HP-5 and HP-6.** HP-2 is
alternation-free and any body; HP-3 is `∀*∃*` with a safety body and
unconstrained existential behaviours; HP-5 is a possibility claim; HP-6 is
HP-2 with a safety body aligned on projections. The restriction on HP-3 is what lets
one existential tuple be found by subset construction and König's lemma
(HC-2), with no Büchi complementation.

### 4. Checking

#### 4.1 Constructions

| ID | Rule |
| --- | --- |
| HC-1 | **Self-composition product (HP-2).** A `HyperProduct` `TransitionSystem` whose state is the tuple of component model states and the state of the generalized Büchi automaton for the negated body (ADR-018 SM-6), keyed by the component state keys in quantifier order and the automaton state index. Initial states: every tuple of initial states, each component from its own subject's initial list (the same subject twice gives every ordered pair, the pair of a state with itself included), with the automaton reading joint position 0. Successors: every tuple of component steps (each an FR-120 transition identity enabled at the component, or the stutter step at a terminal component) on which `μ_U` holds, each with every joint post-state and automaton successor. A joint step's identity is the tuple of component identities. `μ_U` is evaluated through the one clause evaluator (ADR-016 FE-3) over the step labels. |
| HC-2 | **Witness-set product (HP-3).** A `TransitionSystem` whose state is (the universal tuple, `X`), where `X` is a set of pairs (existential tuple of model states, state of the safety automaton for the body). The safety automaton is the Büchi automaton for the body with every state of empty language removed, every remaining state accepting; for a safety body, an infinite word has a run in it exactly when it satisfies the body (Kupferman and Vardi). Initial: for each initial universal tuple, `X_0` holds every (initial existential tuple, automaton state reached by reading joint position 0). Successors: each universal joint step on which `μ_U` holds; `X'` holds every (existential post-tuple, automaton successor) reached from a member of `X` by an existential joint step on which `μ_E` holds against that universal step, the automaton reading the joint post-position. `X` is stored as the sorted sequence of its members' keys, so equal sets coalesce. **Violation:** a reachable state with `X` empty from which a cycle exists that is fair for every universal variable. `X` empty is absorbing. **Correctness:** if `X` stays non-empty along a fair universal behaviour, the existential prefixes that survive form an infinite, finitely branching tree, and König's lemma gives an infinite branch: an existential tuple matched at every step on which the automaton has an infinite run, so the body holds. If `X` empties, every matched existential tuple has left the safety closure by that position, so none satisfies the body. With `n = 0` there is one universal tuple, the empty one, and the clause holds exactly when `X` never empties. This is the construction ADR-020 AX-2 uses for hidden abstract fields; both use one `WitnessSet` component in `model_check`. |
| HC-3 | **Tuple enumeration (HP-1).** EN-1's first phase explores each subject's reachable graph once, retaining edges. For each tuple of edges labelled with the variables' operations, in the product of the canonical edge orders (FR-181), the body is evaluated through the one clause evaluator (ADR-016 FE-3) over the executions' observations. The first tuple on which it is false is the canonical counterexample. |
| HC-4 | **Every hyper item runs both EN-1 phases.** A safety body under `μ_U` or a universal fairness set is not machine-closed: a bad prefix of a matched tuple counts only when it extends to a fair matched tuple. EN-1 therefore decides HP-2 by ADR-018's SCC phase for every body; for a safety body the automaton's rejecting monitor state is an accepting sink with a self-loop, so a violation is a fair accepting cycle reachable through it. HP-3 runs the SCC phase over the states with `X` empty. Fairness constraints are scoped to their component: the filter (ADR-018 FA-4, ADR-019 SR-2) reads a constraint of variable `k` as enabled and taken on component `k` of the joint state and joint step, and is otherwise unchanged. |

#### 4.2 Algorithm families and engines

HP-2 is the self-composition reduction of alternation-free HyperLTL to LTL
(MCHyper's method), so it reuses EN-1's product, SCC phase, fairness filter
and lasso unchanged. HP-3 is the automata-theoretic method for `∀*∃*`
(AutoHyper's), specialised to safety bodies with unconstrained existential
behaviours: the existential side is resolved by subset construction over the
safety closure, which is exact on a finite subject and needs no Büchi
complementation. HP-1 is a 2-safety (in general `n`-safety) check over the
reachable graph (Clarkson and Schneider's `k`-safety).

| ID | Engine | Forms | Verdicts |
| --- | --- | --- | --- |
| HC-5 | **EN-1** (ADR-018 §3), module `model_check` | HP-1 by HC-3; HP-2 by HC-1; HP-3 by HC-2 | V-1, V-4, V-5, V-6, V-7 |
| HC-6 | **EN-2**, SMT unrolling of `n` copies of the transition relation over `k` joint steps, with `μ_U` as a constraint on each joint step and a joint loop back-edge that closes every component at the same step | HP-2 refutation | V-4, V-5 |
| HC-7 | **EN-3**, `k`-induction over the self-composed transition relation | HP-2 with a safety body | V-3, V-6 `InductionNotClosed` |

EN-2 and EN-3 are the SMT backend's (ADR-018 RU-1). HP-1's
model claim, HP-3 and HP-6 are EN-1's; HP-1's code claim is the IR/CG side's
(§14).

#### 4.3 Tractability and run limits

| ID | Rule |
| --- | --- |
| HC-8 | **Pre-check.** Before building any product, EN-1 classifies every root of every subject (ADR-018 §3 pre-check) and explores each subject's reachable graph once under the request's `ModelCheckLimits`. A subject that alone reaches a limit settles the item V-7 before the product is built. HP-4 settles V-8 in negotiation and reaches no engine. HP-5 classifies every root as above but runs no separate subject exploration: ADR-022's phase 0 runs first, then SE-3's product exploration, whose limits count product states (§15). |
| HC-9 | **Cost per form.** With `R_i` the reachable states of component `i`, `d_i` its greatest out-degree, `E_i` the edges of the operation variable `i` names, `A` the automaton for the negated body and `Q` the safety automaton: HP-1 evaluates at most `∏ \|E_i\|` tuples. HP-2 stores at most `∏ \|R_i\| × \|A\|` product states with at most `∏ d_i` joint steps each; matching on inputs usually removes most of them, since a matched joint step needs equal labels. HP-3 stores at most `∏ \|R_u\| × 2^(∏ \|R_e\| × \|Q\|)` states, and one `X` holds at most `∏ \|R_e\| × \|Q\|` members. When the existential model's step is determined by the matched inputs, as in a model whose only nondeterminism is the environment's choice, every `X` holds at most (existential initial tuples) × `\|Q\|` members. |
| HC-10 | **Run limits.** Every limit is a caller-set resource budget, never a modelling limit: no form, formula size, arity or depth is capped. Each is counted with checked arithmetic as the run reaches it, and reaching one stops the run and settles V-7, `Incomplete(ResourceExhausted)` naming the limit, so the run never continues past it and the caller raises it and reruns. `max_states` counts product states, `max_transitions` joint steps, and `max_automaton_states` automaton states (ADR-018 IV-6). Two budgets join `ModelCheckLimits`, ADR-014 B-5 budgets of QSL's own provider that a request sets like the others, each with a published default that applies only when the request sets none: **`max_witness_set`**, the members one HP-3 witness set may reach before the run stops, default 2^16 (65,536); **`max_relation_tuples`**, the tuples an HP-1 run may evaluate before it stops, default 2^24 (16,777,216). `max_depth` is a method parameter (ADR-018 §1 "Depth") and settles V-5 (HV-3). Meter, time and cancellation settle V-7 as ADR-018 §3 states. |
| HC-11 | **Where checking stops being tractable.** For HP-2 with two copies of one subject, the product is quadratic in the reachable states: at a `max_states` budget of 2^20 an unpruned pair product fits components of about 1,000 reachable states, and with three copies about 100; copy-swap symmetry (§16) halves a symmetric pair product. HP-3 is exponential in the existential reachable states in the worst case and linear in them when the existential step is determined by its matched inputs (HC-9). HP-4's general `∀∃` needs Büchi complementation, `2^O(s log s)` in the automaton size `s`. |

### 5. State-space reduction

The rows below are proposed for ADR-021 PT-2, under the same normative rule
(ADR-021 PT-1): a reduction applies exactly when its row reads "yes".

| ID | Form | Symmetry (ADR-021 SYM-5) | POR (ADR-021 POR-6) | State constraint (ADR-021 SC-2) |
| --- | --- | --- | --- | --- |
| PH-1 | HP-1 step relation, one subject | yes: the first execution ranges over orbit representatives, the others over full orbits | no: the relation reads every reachable state | refutation real; pass only when unreached (ADR-021 RV-4) |
| PH-2 | HP-2, every variable over one subject, every fairness constraint `whole` | yes, with the diagonal action: one permutation on every component, the object parameters' stabiliser applied (ADR-021 SYM-6) | no: lockstep pairs positions across traces | lasso real; pass only when unreached |
| PH-3 | HP-2 over two subjects, or with any `each` constraint | no | no | as PH-2 |
| PH-4 | HP-6, every variable over one subject | yes, diagonal, as PH-2 | no: skipped and visible steps interleave per component, and the reduction changes which visible steps align | refutation real (a bad projected prefix of explored paths); pass only when unreached |
| PH-5 | Copy-swap (§16): HP-2 or HP-6 with variables S3 finds swappable (CS-1) | yes, any fairness granularity: the swap renames no key, so an `each` constraint maps to the same constraint on the other copy | not applicable: the swap is a symmetry, not an interleaving reduction | as the form's own row |
| PH-6 | HP-3, every variable over one subject | yes, diagonal, with `X` renamed member by member and re-sorted, as ADR-021 renames ADR-020's AX-2 set | no | no: a cut existential state removes a witness, so a refutation would not be real |

Why each entry holds:

- **Diagonal symmetry.** Under ADR-021 SYM-5 every permutation `π` of the
  group is an automorphism of the subject, and indexed atoms and `μ` are
  identity-transparent (ADR-021 SYM-3, read over the body and `μ`). Applying
  the same `π` to every component maps a matched tuple to a matched tuple
  with the same atom values at every joint position, so the product
  quotients by the diagonal group. A separate permutation per component
  would break an atom that compares one key across traces, such as
  `v.l @ a = v.l @ b`. Two subjects have unrelated universes, so no diagonal
  exists.
- **POR.** Reordering independent steps inside one component changes which
  of its steps pairs with another component's step, which changes indexed
  atoms that compare the two. No lockstep hyperproperty is invariant under
  that.
- **Copy-swap.** §16, CS-3.
- **State constraint on HP-3.** `X` is an existential over the existential
  subject's behaviours. A cut removes members, `X` can empty only because of
  the cut, and the resulting refutation is not a refutation of the subject.

### 6. Verdicts

Every verdict is one of ADR-018 V-1 to V-8. No label, basis, category or
`ProofBasis` member is added; a `proved` record carries its certification
(HX-7, ADR-029 RU-2).

| ID | Rule |
| --- | --- |
| HV-1 | **Proved.** EN-1 completes with no violation: HP-1 every tuple evaluated true, HP-2 no fair accepting cycle, HP-3 no reachable empty `X` with a fair universal cycle, HP-6 no reachable rejecting monitor state. V-1, `proved`, `closed-scope`, `Proved{basis: Exhaustive, certification: Certified}`, or `Proved{basis: Reduced{…}, certification}` under ADR-021 RV-1, which names copy-swap when it applied (CS-5): `Certified` when copy-swap is its only reduction, `Uncertified` when symmetry or partial-order reduction applied. EN-3 proves HP-2 as V-3. Every V-1 `proved` from EN-1 settles only after the product-closure certificate checker (HX-7) accepts its certificate. A proof whose method has no certificate checker, such as EN-3's induction, settles `proved` labelled `Uncertified` in its terminal record. A rejected certificate settles V-6 `CertificateRejected`. |
| HV-2 | **Refuted.** A counterexample (§7) that replay reproduces, including one that ends where the claim evaluated undefined (HV-8). V-4, `refuted`, `decisive-counterexample`, `Refuted`. |
| HV-3 | **Bounded.** A run that completes every depth up to `max_depth` with no violation settles V-5 `BoundReached{depth}`. For HP-2 and HP-3 this is "no counterexample whose universal lasso has length at most `k`", lockstep length being the shared number of joint steps (ADR-018 §1 "Length"). For HP-1 it is "no counterexample whose executions all leave states at depth below `k`". |
| HV-4 | **Inconclusive causes.** `InconclusiveCause` gains `MatchUndetermined`: `μ_U` or `μ_E` evaluated refused or incomplete at a reachable joint step, for HP-2, HP-3 and HP-6 alike. It gains `VacuousMatch`: the HM-7 case. `UndecidedSuccessor`, `NoInitialState`, `ReplayParity` and `ReplayRefused` apply as ADR-018 V-6 states. All are V-6. |
| HV-5 | **Stopped.** Reaching a run limit of HC-10 or the meter settles V-7, FR-360 `failed` with execution `resource-incomplete`, result `incomplete`, `LimitReached{limit, value, setting}` (ADR-018 V-7); cancellation settles V-7, FR-360 `failed` with execution `resource-incomplete`, result `incomplete`, `unavailable`, `Incomplete(Cancelled{source})`, written `cancelled{source}` (FR-127, QSpec FR-399). |
| HV-6 | **Unsupported.** HP-4, and a clause over `behaviours` under a profile other than infinite-trace (HS-7), settle V-8, `unsupported-requested-capability`. |
| HV-7 | **Identity.** The obligation identity (ADR-013 O-09) binds every subject in alias order, the quantifier prefix with each variable's alias in order, each variable's fairness set, `μ`, and the object parameters' instance. Reversing two quantifiers, or moving a constraint from one variable to another, gives a different obligation (QSpec FR-191-AC-2, AC-4). |
| HV-8 | **Undefined evaluation.** ADR-018 UE-1 to UE-6 apply to every hyper form EN-1 decides. When the body or `μ` evaluates undefined on a tuple, the item settles `refuted`, V-4, with cause `UndefinedEvaluation{where, cause}`. `where` names the tuple: one trace per quantified variable, ending at the joint position where the claim evaluated undefined, which is `trace_position`; for HP-1, the tuple of executions. `cause` is the evaluator's undefined cause. The engine reports the first refuting evidence in canonical order: for HP-1 the first tuple, in the product of canonical edge orders, on which the body is false or undefined; for HP-2, HP-3, HP-5 and HP-6 the first product state, in canonical breadth-first order, at which a letter is undefined or a violation is reached, as ADR-018 UE-4 states. The counterexample is QSpec FR-400's `undefined` kind: one finite path per trace or execution variable, universal and existential, ending at the state the undefined evaluation reads, so an undefined `μ_E` or body letter on an HP-3 existential tuple is carried and replayed like a universal one. |

### 7. Counterexamples and replay

| ID | Rule |
| --- | --- |
| HX-1 | **Content.** A hyper counterexample is QSpec FR-400's `HyperCounterexample`, with its five kinds: `lockstep` (HP-2), `witness-exhausted` (HP-3), `projected` (HP-6, PA-6), `step-tuple` (HP-1) and `undefined` (any form, HV-8), and FR-400's reader validation. QSL's type mirrors it member for member (FR-183). It travels in `WitnessEnvelope<HyperCounterexample>` with a new `ReplaySource::ModelTraceTuple` source arm, replayed by a layer-6 facade entry `qsl_replay::replay_model_trace_tuple` beside ADR-018 CX-3's `replay_model_trace`, `trace_position` naming the failing joint position (ADR-014 TR-2). |
| HX-2 | **Replay of `Lockstep`.** E9 recompiles the package (FR-098) and re-admits each subject. For each trace it re-executes the steps through its subject's `ModelSystem` with FR-101 `replay`, with ADR-018 CX-3's refusals (no successor of a step's identity with the recorded post-state, a step not enabled, a loop that does not close at `loop_entry`). It then checks `μ_U` at every joint step, the loop's closing step included, and each trace's fairness set on its own loop (ADR-018 CX-3, ADR-019 SR-8), refusing an unmatched tuple or an unfair trace with `invalid_runtime_input`/`invalid-value`. It evaluates the body over the tuple by HM-4. False settles `reproduced-with-evaluated-witness` and the item `refuted`; true settles `inconclusive`, `ReplayParity`. |
| HX-3 | **Replay of `WitnessExhausted`.** Replay re-executes and checks the universal traces as HX-2 does, without evaluating the body. It then recomputes `X_0` to `X_position` along the universal traces from the existential subjects' `ModelSystem`s and the safety automaton, by HC-2's rule, under the request's `max_witness_set`. `X_position` empty and every earlier `X` non-empty settles `reproduced-with-evaluated-witness` and the item `refuted`. A non-empty `X_position` settles `inconclusive`, `ReplayParity`; reaching `max_witness_set` during replay settles `inconclusive`, `ReplayRefused`. |
| HX-4 | **What an HP-3 refutation proves.** It names a fair tuple of universal behaviours and a joint position by which every existential tuple matched with it has violated the body. `X` is a function of the universal prefix, the existential subjects and the body, so replay recomputes it with no engine present and checks the absence of a witness exhaustively over the existential subject, along a finite prefix. That is why the refutation's basis is `decisive-counterexample`: the evidence is the universal tuple, and the existential half is a closed computation over it. |
| HX-5 | **Replay of `StepTuple`.** Replay re-executes each prefix, checks the final transition is enabled with the recorded post-state, and evaluates the body over the tuple of executions through the one clause evaluator. False reproduces; true settles `ReplayParity`. A counterexample whose `undefined` member is set replays, for every kind, by ADR-018 UE-5 over the tuple: it reproduces when the first undefined evaluation along the tuple is at `where` with an equal cause. |
| HX-6 | **Engine independence.** EN-2 reads each component's transition identities from selector variables in its encoding and produces a `Lockstep` counterexample that replays by HX-2 with no solver present (ADR-018 CX-4). A counterexample found under symmetry is concretised by ADR-021 EI-6 per component with the one diagonal permutation, so no counterexample carries a canonical state. |
| HX-7 | **Product-closure certificate.** An EN-1 `proved` for HP-1, HP-2, HP-3, HP-5 or HP-6 writes a product-closure certificate: the form, the instance, and the sorted list of reachable product-state keys the run stored (for HP-1, each subject's reachable state keys). The layer-6 checker in `qsl-replay` recompiles the package and rebuilds each subject's `ModelSystem`, as replay does, and reads nothing from the engine but the certificate. It accepts when the recomputed initial product states are members, every member's recomputed successors are members (canonicalised by CS-2 when the record names copy-swap), no member's letters or `μ` evaluate undefined, refused or incomplete, and the closed set holds no violation by the form's own rule, recomputed by the checker: no fair accepting SCC (HP-2), no reachable empty `X` with a fair universal cycle (HP-3), no rejecting monitor state (HP-6), no false tuple (HP-1), no fair accepting cycle missing from an initial state's part (HP-5). FR-163 states the format and the check, reusing ADR-018 PC-3 and PC-4 (ADR-029 RU-2). |

### 8. Worked examples

**The subject.** Model `V` has object type `Vault` with fields `h: Int[0, 1]`
(the secret) and `l: Int[0, 1]` (the public output), population `vaults` and
one operation `step(i: Int[0, 1])`, the public input, with frame `modifies
[l]` and no precondition. Universe `vaults = {v}`. Two initial snapshots:
`(h, l) = (0, 0)` and `(1, 0)`. A state is written `(h, l)`, a step
`step(i)`. Two post clauses are compared:

- **leaky:** `self.l = (i + self.h) mod 2`;
- **secure:** `self.l = i`.

Each model has 4 reachable states, every `(h, l)`, each with two always
enabled steps and no terminal state, so each subject's deadlock-freedom item
settles `proved` (V-1).

#### 8.1 `∀∀`: noninterference (HP-2)

The clause is §1's `NonInterference`: every quantifier `forall`, an
infinite-trace body, so HP-2. `μ_U` matches equal inputs.

**Product.** The negated body is `eventually not holds(v.l @ a = v.l @ b)`, a
two-state Büchi automaton: `q0` loops on every joint position and moves to
accepting `q1` on a position with `l_a != l_b`; `q1` loops on every position.
A joint step is `(step(i), step(i))`, two per product state. `h` never
changes, so a product state keeps its pair `(h_a, h_b)`.

- **leaky.** With `h_a = h_b`, matched inputs give equal `l`, so the
  reachable `l` pairs are `(0,0)` and `(1,1)`, both with `q0`: 2 product
  states for each of the 2 such `h` pairs, 4 in all. With `h_a != h_b`,
  every step gives unequal `l`: the `l` pairs are `(0,0)` with `q0`, and
  `(0,1)` and `(1,0)` each with `q0` and with `q1`, 5 product states for each
  of the 2 such `h` pairs, 10 in all. The reachable product has 14 states
  over 10 model pairs. It has two accepting SCCs, `{(0,1) q1, (1,0) q1}` for
  each unequal `h` pair, each closed under both joint steps. The empty
  fairness set admits them, and the item settles `refuted` (V-4) with a
  `Lockstep` counterexample of this shape:

  | Joint position | `a` (`h`, `l`) | `b` (`h`, `l`) | Joint step into the next position |
  | --- | --- | --- | --- |
  | 0 | `(0, 0)` | `(1, 0)` | `(step(0), step(0))` |
  | 1 (loop entry) | `(0, 0)` | `(1, 1)` | `(step(0), step(0))`, back to position 1 |

  Replay (HX-2) re-executes both traces from their initial snapshots, checks
  the inputs match at both joint steps, and evaluates the body: `l_a = 0` and
  `l_b = 1` at position 1, so it is false, `trace_position` 1. The canonical
  lasso is fixed by ADR-018 §3 "Determinism" and CX-5's greedy loop; any lasso
  EN-1 returns has this form.
- **secure.** Every joint step gives `l_a = l_b = i`. The reachable product
  has 8 states: the 4 `h` pairs times the `l` pairs `(0,0)` and `(1,1)`, all
  with `q0`. No accepting state is reachable, and the item settles `proved`,
  `Proved{basis: Exhaustive, certification: Certified}` (V-1).
- **copy-swap.** The body and `μ` are symmetric in `a` and `b` and both
  fairness sets are empty, so S3 admits copy-swap (CS-1) and EN-1 stores one
  state per swap class (§16). The counts above are unreduced. The swap fixes a
  product state whose two components are equal and pairs the rest, so the
  leaky product stores 9 classes (the 4 states with equal components, and the
  10 with unequal `h` in 5 pairs) and the secure product 6 (4 fixed states and
  2 pairs). The leaky verdict and the secure verdict are unchanged.
- **without `match`.** Joint steps pair `step(0)` with `step(1)`, `l` differs
  after one step even in the secure model, and the item settles `refuted`.
  The match is what makes the clause noninterference.

#### 8.2 `∀∃`: the secret is not revealed (HP-3)

```text
hyper Opaque using inf over behaviours (v: V::Vault)
  forall trace a of V
  exists trace b of V
  match { a.step.Vault::step.i = b.step.Vault::step.i }
{
  holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b)
}
```

For every behaviour there is another, with the other secret and the same
inputs, that an observer of `l` cannot tell apart from it. The body is in the
safety fragment and `b` has the empty fairness set, so HP-3, with `μ_E` the
one conjunct. The safety automaton has two states: `init`, which reads joint
position 0 and moves to `g` when `h_a != h_b` and `l_a = l_b`, and `g`, which
loops while `l_a = l_b`. After position 0 every member of `X` is at `g`.

- **leaky.** From `a = (0, 0)`, `X_0 = {((1, 0), g)}`. After `step(i)`, `a`
  has `l = i` and the matched `b` step gives `l = (i + 1) mod 2`, so every
  member leaves the safety automaton and `X_1` is empty; `a = (1, 0)` is
  symmetric. The reachable product has 6 states: the 2 initial universal
  states with their singleton `X`, and the 4 universal states with `X`
  empty. Every universal state has a successor, so a cycle is reachable from
  an empty `X`, and the item settles `refuted` (V-4) with a
  `WitnessExhausted{position: 1}` counterexample whose one trace is `a`'s
  lasso `(0, 0) -step(0)-> (0, 0)`, loop entry 0. Replay (HX-3) re-executes
  `a`, recomputes `X_0 = {((1, 0), g)}` and `X_1` empty from `V`'s
  `ModelSystem`, and reproduces.
- **secure.** After every matched step `l_a = l_b = i`, so for `a` at
  `(h, l)` the set is `X = {((1 - h, l), g)}`. The reachable product has 4
  states, one per universal state, each with a singleton `X`. No `X` empties,
  and the item settles `proved` (V-1).

### 9. Interactions with the sibling records

- **Strong fairness (ADR-019).** A universal variable's fairness set may hold
  strong constraints. ADR-019's SCC refinement runs on the HP-2 product and
  on the empty-`X` part of the HP-3 product with constraints scoped to their
  component (HC-4). An existential variable with any constraint is HP-4.
- **Refinement (ADR-020).** Refinement with hidden abstract fields is a
  `∀∃` check between two subjects: for every concrete behaviour there is an
  abstract behaviour consistent with it. ADR-020 AX-2's set of abstract
  states and HC-2's `X` are one construction, built once as `WitnessSet`.
  The two differ in alignment: a refinement step map absorbs `stutter` rows,
  so concrete and abstract positions are aligned by the step map; a hyper
  clause aligns in lockstep (HM-2). ADR-020 AX-5 settles a liveness half with
  hidden fields `unsupported` for the reason HP-4 does: an existential side
  with a liveness obligation needs Büchi complementation.
- **State-space reduction (ADR-021).** §5's rows PH-1 to PH-6 for PT-2.
  ADR-021 EI-4's hooks delegate from the hyper product to each component; the
  automaton component and `X`'s automaton states are never reduced.
- **Possible properties (ADR-022).** A clause whose prefix is one existential
  variable asks whether some behaviour satisfies the body: a possibility
  claim. It is routed to ADR-022's family and settles by its verdict rows
  GV-1 (V-9), GV-2 (V-10) and GV-5 (§15).

### 10. Downstream impact

**Stage DAG.** Hyper and relation model claims run at ADR-018's S6c, reading
the S4 package over E10; their refutations leave S6c over E11 as S7 typed
witnesses, reach S8 through E9, and settle their terminal records only after
replay, as ADR-018 §7 states for a model counterexample. A relation's code
claim runs on the existing S5 → E7 → S6b Kani path (§14). Following ADR-029
CB-3 and ADR-018 LA-1 to LA-4, layer A `qsl-analyze` holds the engines:
the exploration of the hyper, projected and witness-set products, the SCC
phase, tuple enumeration and certificate emission. Layer 5 `qsl-eval` holds
what the engines and the core checker both use: the products' successor
functions and state numbering, the `WitnessSet` step, the property-automaton
translation, component-scoped fairness enabledness and the copy-swap
canonicaliser. Layer 6 `qsl-replay` holds the verdict types, the
settlement map, the facade entry `replay_model_trace_tuple` and the
product-closure certificate checker (HX-7, FR-163). Types CG reads live in
`qsl-replay` or `quire-semantic-value` (ADR-018 LA-4).

| ID | Repository | Change |
| --- | --- | --- |
| DS-1 | QSL | S2: forms for `hyper` over `behaviours` and `relation` over model executions, in place of today's refusal. S3, in the `TemporalTrace` family: the HS-2 alias binding, object parameters, indexed atoms, step labels and the `μ` split, per-variable fairness, the execution-binding check, the `align skip` list (PA-1), the copy-swap test (CS-1), form classification HP-1 to HP-6, and the HP-1 code-claim requirement record (XC-1). Layer-5 evaluator: tuple-of-lassos evaluation (HM-4) and three-valued evaluation of a projected prefix tuple (PA-6). `qsl-eval`: the successor functions and state numbering of `HyperProduct` (HC-1), the projected product (PA-4) and `WitnessSet` shared with ADR-020 (HC-2), component-scoped fairness enabledness and the copy-swap canonicaliser (CS-2). `qsl-analyze`: the exploration of those products, tuple enumeration (HC-3), the two new budgets, `VacuousMatch` detection and certificate emission; HP-5 reaches ADR-022's engine (SE-2). `qsl-replay`: `HyperCounterexample`, `ReplaySource::ModelTraceTuple` and its result arm, the verdict settlement with `MatchUndetermined` and `VacuousMatch`, and the product-closure certificate checker (HX-7). The request writer: one subject per alias and the deadlock-freedom item per subject. |
| DS-2 | CG | The EN-1 `negotiate_*` arm reads the HP form; HP-4 settles V-8. The SMT arm takes HP-2 only. The two-call Kani harness for an HP-1 code claim (XC-2 to XC-5). Obligation identity per HV-7. |
| DS-3 | IR | EN-2 and EN-3 over `n` copies of the transition relation with `μ_U` as a step constraint, after ADR-018 DS-3. Intake of the HP-1 code-claim obligation (XC-1). |
| DS-4 | Driver | Supplies one subject per alias to S6c and to E9 replay. |
| DS-5 | QSpec | §11. |

### 11. What QSpec must specify

| ID | Item | Where |
| --- | --- | --- |
| QS-1 | Surface syntax: the `behaviours` domain with object parameters (HS-1, HS-3), `of <alias>` on a quantifier (HS-2), per-variable fairness (HS-6), indexed atoms (HS-4), the `match` block and step labels with the `stutter` label (HS-5), execution bindings to model operations and the pre, post, argument and result reads (HS-8), `align skip { … }` (HS-9, PA-1) | QSpec FR-395 and FR-402 |
| QS-2 | FR-191 over model subjects: a trace variable's domain may be the fair behaviours of a model subject (HM-1); lockstep as the explicit total alignment over behaviours (HM-2); the `μ_U`/`μ_E` relativisation of quantifiers (HM-3); tuple evaluation (HM-4); terminal stutter (HM-5); vacuity (HM-7); the step relation (HM-9); projection alignment and its prefix reading (PA-2, PA-3) | QSpec FR-395, FR-397, FR-398 and the amended FR-191 |
| QS-3 | The forms HP-1 to HP-6, the admitted fragment, V-8 for HP-4, and the routing of HP-5 to the possible family with its witness, trap and verdict rules (SE-1 to SE-5) | QSpec FR-396; FR-390 to FR-394 for HP-5 |
| QS-4 | Claim-form rows: a hyperproperty over behaviours and a relation's model claim require `temporal-satisfaction`; a relation's code claim requires `operation-contract`, one claim per relation, with every declared invariant of each execution's model as its pre-state premise (XC-1, XC-2) | QSpec FR-290 |
| QS-5 | Verdicts: the HV table onto QSpec FR-360 and FR-243; the `MatchUndetermined` and `VacuousMatch` inconclusive causes; `max_witness_set` and `max_relation_tuples` with their defaults and V-7 settlement | QSpec FR-399, FR-360, FR-331 |
| QS-6 | Counterexample wire and replay: `HyperCounterexample` with its five kinds, the `undefined` kind carrying a trace for every variable, equal trace lengths and the shared loop entry for `Lockstep` and `WitnessExhausted`, per-step skip marks for `Projected`, the replay rules HX-2, HX-3, HX-5 and PA-7, HX-4's evidence for an HP-3 refutation, and the relation code claim's Kani counterexample replay (XC-4) | QSpec FR-400, FR-401, FR-331 |
| QS-7 | Obligation identity: subjects, prefix with aliases, per-variable fairness, `μ`, instance (HV-7); the checked-package v2 `hyperproperty` node's members for them | QSpec FR-402, FR-331 |
| QS-8 | Preservation rows PH-1 to PH-6 beside ADR-021's PT-2, and copy-swap: its detection rule, its admission without a request selection, and `CopySwap` in `ProofBasis::Reduced` (CS-1 to CS-5) | QSpec FR-403 |
| QS-9 | Conformance vectors, each with expected verdict and, for a refutation, a counterexample that must replay: (a) §8.1 leaky refuted, secure proved, unmatched refuted; (b) §8.2 leaky refuted with exhaustion at position 1, secure proved; (c) a `μ_U` no tuple satisfies, settling `VacuousMatch`; (d) a model where one copy reaches a terminal state, with `μ` matching `stutter`; (e) an HP-2 liveness body that holds only under one variable's fairness set; (f) HP-4 prefixes and bodies settling V-8; (g) a step relation for determinism, refuted on a model with a nondeterministic post-state and proved on a deterministic one; (h) replay refusals: traces of unequal length, `μ_U` false at a joint step, an unfair component, a recomputed non-empty `X`; (i) an HP-3 run that reaches `max_witness_set` and settles V-7; (j) a body with an interval operator nested under `always`; (k) §13's projection example, refuted in lockstep and proved with `align skip`, and a projected counterexample that replays; (l) copy-swap on §8.1, with the reduced and unreduced runs agreeing, and a body that fails the CS-1 test; (m) an HP-5 claim proved by a lasso witness and one refuted by a trap; (n) a relation code claim whose Kani counterexample replays | QSpec TC-213 and new TCs |

### 12. Rulings on the draft's questions

The owner ruled on the four questions the draft left open, and on undefined
tuples (RU-5), on 2026-10-01.

| ID | Question | Ruling | Rationale | Where it lands |
| --- | --- | --- | --- | --- |
| RU-1 | Whether v1 admits an alignment other than lockstep | **Projection alignment is admitted beside lockstep**, for universal clauses with a safety body: the clause names the operations whose steps are skipped, and the traces align on their remaining steps | A secret that changes how many internal steps a run takes makes lockstep pair an internal step of one run with a visible step of the other, and report a leak that no observer of the visible steps can see. Safety bodies keep the check exact: a safety violation is a finite prefix, so the projected product decides it in one phase, while a projection of an infinite behaviour can be finite and asynchronous hyperproperties with liveness bodies are undecidable in general | HS-9, HP-6, §13 (PA-1 to PA-7), PH-4 |
| RU-2 | Reachable or declared-type step relations | **Both, by subject.** A model claim runs on EN-1 over reachable transitions; a code claim runs as a two-call Kani harness over every pre-state that satisfies the declared invariants | The model claim covers what the model can reach and needs exploration. Code has no explored state graph, so its premise is the declared invariants, which every reachable state satisfies once they are proved. The two are separate obligations: each settles on its own evidence | HP-1, §14 (XC-1 to XC-5) |
| RU-3 | Who owns single-existential prefixes, and the basis of an exhaustive refutation | **Routed to ADR-022's possible family.** A witness proves the claim once exploration has ruled out a reachable undefined evaluation (ADR-022 GV-1, V-9, RU-5); an exhaustive search of a trap's closure that finds none refutes it (ADR-022 GV-2, V-10); a partial search stays inconclusive (ADR-022 GV-5). The refutation's basis is `closed-scope`, settled by exhaustive exploration, with trap evidence that replay re-explores | A single existential asks whether some behaviour exists, which is a possibility question; one family keeps one witness shape, one trap shape and one verdict map for every possibility claim. A refutation shows that no trace exists, so it has no trace to carry: its evidence is the closed scope it examined | HP-5, §15 (SE-1 to SE-5) |
| RU-4 | Copy-swap symmetry in v1 | **Admitted when the compiler verifies it; authors never declare it** | Swapping two universal runs over one subject leaves a symmetric body and match unchanged, so the pair product holds every pair twice. S3 checks the swap on the checked clause, so a symmetry the clause does not have is never applied | §16 (CS-1 to CS-5), PH-5 |
| RU-5 | How an undefined tuple settles | **Refuted**, cause `UndefinedEvaluation{where, cause}` naming the tuple, as ADR-018 RU-5 rules for every proof engine; the tuple is the first refuting evidence in canonical order | One reading of an undefined claim across every engine: an undefined claim does not hold, the tuple replays, and no result kind is added | HV-2, HV-4, HV-8, HX-5 |

### 13. Projection alignment

| ID | Rule |
| --- | --- |
| PA-1 | **Syntax.** `align skip { O, … }` follows the quantifiers. Each `O` names an operation of a model alias the clause binds; its steps are skipped in every trace of that alias. S3 refuses an operation of no bound alias, an operation named twice and an empty list, located at the entry. A clause with `align skip` and an existential variable or a body outside ADR-014 A-4's safety fragment is HP-4 (V-8). A fairness set on a variable of an `align skip` clause refuses at S3 (PA-3). The spelling is illustrative; QSpec's shared grammar owns it (QS-1). |
| PA-2 | **Projection.** A step is **skipped** when its operation is in the list and **visible** otherwise. The projection of a finite path is the sequence of its positions that are position 0 or are entered by a visible step; projected position `j` is the `j`-th of them, with position 0 the initial state. At a projected position an atom reads the state the visible step entered (ADR-018 SM-3), and `μ` reads that visible step's label. Skipped steps never reach `μ` or an atom. Interval operators count projected positions. |
| PA-3 | **Meaning: every matched projected prefix is good.** An HP-6 clause holds exactly when, for every tuple of finite paths from initial states, one per variable of its subject, whose projections have one length `L + 1` and on which `μ_U` holds at every joint visible step `1` to `L`, the joint projected prefix is not a bad prefix of the body: ADR-014 A-4's three-valued evaluation does not return false for it. Projected positions correspond by index: projected position `j` of every trace. A path that ends at a terminal state, or whose continuations are all skipped steps, takes part in the prefixes up to its last visible step. Every finite path of a subject extends to a fair behaviour (ADR-018 §4, ADR-019 SF-7), so a fairness set would change no HP-6 verdict, which is why S3 refuses one, as ADR-022 GM-7 refuses fairness on a state-graph claim. |
| PA-4 | **Check: the projected product.** A `TransitionSystem` whose state is the tuple of component model states and the state of the body's safety monitor (ADR-018 SM-6), keyed as HC-1 keys a state. Initial states are HC-1's, the monitor reading joint position 0. It has two kinds of move. A **skip move**: one component takes an enabled skipped step; every other component and the monitor stay. A **visible move**: every component takes an enabled visible step, `μ_U` holds on the joint step, and the monitor reads the joint post-position. A violation is a reachable rejecting monitor state, found in EN-1's first phase. **Exactness:** skip moves of different components commute, so the reachable product states are exactly the tuples of finite paths with equal projected length, each followed by any number of skipped steps, paired with the monitor state of their joint projected prefix. A rejecting state is reachable exactly when PA-3 fails. |
| PA-5 | **Cost.** The product stores at most `∏ \|R_i\| × \|M\|` states, `M` the monitor, as HC-1 does. A state has at most `Σ s_i` skip moves and `∏ v_i` visible moves, with `s_i` and `v_i` component `i`'s greatest skipped and visible out-degrees. `max_depth` counts moves of either kind, so V-5 means no counterexample of at most `k` moves. The HC-10 budgets apply. |
| PA-6 | **Counterexample.** `HyperCounterexample` of kind `Projected`: one finite trace per variable, each step marked skipped or visible, every trace with the same number `L` of visible steps, the traces of unequal total length. It has no `loop_entry`, and `trace_position` names projected position `L`. EN-1's canonical counterexample is the first path in FR-181 breadth-first order to a rejecting state, split into its components' steps. |
| PA-7 | **Replay.** Each trace re-executes through its subject's `ModelSystem` with ADR-018 CX-3's refusals. Replay checks that each skipped mark names a listed operation and each visible mark an unlisted one, that the visible counts are equal, and that `μ_U` holds at every joint visible step, refusing otherwise with `invalid_runtime_input`/`invalid-value`. It then evaluates the body three-valued over the joint projected prefix (ADR-014 A-4). False reproduces and settles the item `refuted`; any other value settles `inconclusive`, `ReplayParity`. |

**Example.** A variant of §8's vault adds a field `busy: Boolean` and an
internal operation `mix`. `step(i)` requires `not busy`, sets `l = i` and sets
`busy` to `h = 1`; `mix` requires `busy`, clears it and leaves `l`. A run with
secret 1 takes a `mix` after every `step`; a run with secret 0 never does.
With `μ` matching the arguments of two `step` steps and nothing else, lockstep
pairs `a`'s second `step` with `b`'s `mix`: `a` sets `l` to its new input
while `b` keeps the old one, and the clause is refuted although every visible
`step` sets `l = i` in both runs. With `align skip { V::Vault::mix }` the
projections are the `step` sequences, matched inputs give equal `l` at every
projected position, and the clause holds. This example is reasoned by hand;
it has no state count.

### 14. Step relations on code: the IR/CG overlap

A relation's code claim is checked by a Kani harness that CG generates. QSL
states what it hands over and what the harness must prove; the harness itself
is CG's.

| ID | Rule |
| --- | --- |
| XC-1 | **What QSL hands over.** When every execution variable's operation is bound to an implementation by the abstraction relation (ADR-017 AR-1 to AR-4; QSpec FR-353), S3 records, beside the model claim's `temporal-satisfaction` requirement, one `operation-contract` requirement for the code claim, with its extent per ADR-014 §4. The checked package carries what the harness reads: the relation node with its execution variables in order, each naming its operation; the body lowered to FR-322 terms over each execution's pre-state, arguments, result and post-state; each execution model's declared invariants; each operation's effective precondition, frame and postcondition; and the abstraction-relation binding of each operation. QSL names no harness, backend or Kani construct. |
| XC-2 | **What the harness must prove.** For all pre-states `s_1 … s_n` of the declared types, each satisfying every declared invariant of its model, and all argument vectors in the operations' parameter domains with each operation's effective precondition true at its `s_i`: running each operation's bound implementation once from its own `s_i`, the calls sharing no state, gives results and post-states on which the body holds. Every outcome of a nondeterministic implementation is covered. The pre-states range over the declared types, not the reachable states. |
| XC-3 | **Separate obligations.** The code claim takes the invariants as its premise and says nothing about whether they hold; the invariant and operation-contract claims say that. The model claim and the code claim have different identities and different evidence, and neither discharges the other. A harness whose premise no state satisfies makes zero checks and settles `inconclusive`, `KaniVacuousProof` (ADR-013 C-09). |
| XC-4 | **Counterexample and replay.** A Kani counterexample gives, per execution, a pre-state, an argument vector, a result and a post-state. It enters E9 by ADR-013 C-09's Kani path. QSL's half of the replay admits each pre-state as an FR-106 snapshot, checks it satisfies the declared invariants, and evaluates the body over the tuple through the one clause evaluator (ADR-016 FE-3). False reproduces; true settles `ReplayParity`. No reachability prefix is involved. |
| XC-5 | **Boundary.** Harness construction, unwinding and state representation are CG's (CG ADR-003). The rows above are the whole of the overlap: XC-1 is what IR admits and CG reads, XC-2 is what the generated harness proves. |

### 15. Single-existential claims

| ID | Rule |
| --- | --- |
| SE-1 | **Routing.** A `hyper` clause over `behaviours` whose prefix is exactly one `exists trace b of M` is HP-5, a possibility claim, and joins ADR-022's family as a form with a temporal body. It records `Requirements{kind: temporal-satisfaction, extent: Unbounded{domains}}` as ADR-022 SG-4 does, and the candidate's arm reads the form. A `match` block on it refuses at S3, since it has one trace. Its fairness set is admitted: a temporal body's witness is an infinite behaviour, and fairness decides which infinite behaviours count. |
| SE-2 | **Meaning.** HP-5 holds when, from every initial state of the subject, some fair behaviour satisfies the body. Every initial state is ADR-022 RU-1's quantifier for `possible`, and an author who means one start lists that one initial snapshot. |
| SE-3 | **Check.** EN-1 builds the product of the subject with the generalized Büchi automaton for the body itself, not its negation, with the clause's fairness, and searches each initial state's part of the product for a fair accepting cycle by ADR-018's SCC phase. ADR-022's phase 0 sampling (`witness_samples`, published default 64) runs first: a sampled walk that revisits a state gives a candidate lasso. |
| SE-4 | **Verdicts.** **Proved:** every initial state has a witness, a fair lasso on which the body is true, and each replays: ADR-022 GV-1, V-9, `proved`, `decisive-witness`, `Proved{basis: Witness{sources}, certification: Certified}` once `check_state_graph` accepts its certificate, each source `Sampled`, `Explored` or `Unrolled{depth}`, and only when the product exploration completed with no open node and no letter of the body evaluated undefined at a reachable product state (ADR-022 GV-1, RU-5). A witness for every initial state on a run that a limit stopped, or that ended with open nodes, settles V-6, `inconclusive`, `WellDefinednessUnchecked`. **Undefined:** a letter of the body that evaluates undefined at a reachable product state refutes the item as HV-8 states, with cause `UndefinedEvaluation{where, cause}`, even when every initial state has a witness. **Refuted:** an initial state whose part of the product is closed (every node expanded, ADR-022 GM-6) and holds no fair accepting cycle is a trap: ADR-022 GV-2, V-10, `refuted`, `closed-scope`, `TerminalValue::Refuted`. **Otherwise:** ADR-022 GV-5, which maps a stopped run to V-7 and a completed run with open nodes to V-6 or V-5. Every result states how it was settled, as ADR-022's settlement-method rule requires. |
| SE-5 | **Evidence, and the basis of an exhaustive refutation.** A witness is ADR-022's `Witness` arm with one ADR-018 CX-2 model lasso per initial state; it replays by ADR-018 CX-3, with fairness checked and the body evaluated true. A trap is ADR-022's `Trap` arm naming the initial state, with an empty stem; replay re-explores that initial state's part of the product, unreduced, under the request's limits, and runs the SCC phase with the fairness filter, so it trusts no engine claim about the closure (ADR-022 GX-3). No fair accepting cycle reproduces the refutation; one found settles `ReplayParity`; a stopped replay settles V-7. An exhaustive refutation of a possibility claim has no single counterexample trace, because what it shows is that no trace exists. Its basis is `closed-scope` (QSpec FR-243), because the trap's closure, which is the whole decision scope, was examined completely, and its settlement method is exhaustive exploration. `decisive-counterexample` is the basis of refutations that carry a trace, as HP-3's `WitnessExhausted` does with its universal traces (HX-4). |

### 16. Copy-swap symmetry

| ID | Rule |
| --- | --- |
| CS-1 | **Detection.** S3 tests every pair of universal variables `a`, `b` of an HP-2 or HP-6 clause. The pair is **swappable** when both name the same alias, their fairness sets are equal (with FA-6's unmarked reading), and the swap `σ`, which renames `a` to `b` and `b` to `a` in every indexed atom and step label, leaves `μ_U` and the body unchanged in S3's normal form. The normal form sorts the operands of `and`, `or`, `=` and `!=` by their checked encoding, flattens nested `and` and `or`, and leaves every other node in place; each rewrite is an equivalence. The test compares the normal form of `σ(φ)` with that of `φ` node by node, and the same for `μ_U`. It is sound, since equal normal forms have equal truth on every joint behaviour, and incomplete: a clause that is symmetric for a deeper reason is not detected, and then no swap applies. Swappability is an equivalence on variables, so the swappable variables fall into classes, and the **copy group** is the product of the symmetric groups of the classes. No request selects it and no author declares it. |
| CS-2 | **Halving the product.** The canonical form of a product state is the member of its orbit under the copy group with the least key bytes, the component keys in quantifier order and then the automaton state index, as ADR-021 SYM-7 orders keys. S3 builds the body's automaton from its normal form, so `σ` permutes the automaton's states by renaming the subformulas each state records. EN-1 canonicalises every successor and stores canonical states only, as ADR-021 EI-3 does. With two copies an orbit has one or two members: a state whose two components are equal is fixed and every other state pairs with its swap, so the stored states are the fixed states plus half the rest. The copy group commutes with ADR-021's diagonal key symmetry, and the run's group is their product. |
| CS-3 | **Soundness.** `σ` is an automorphism of the product. Both variables range over one subject with one fairness set, so a tuple is in the domain exactly when its swap is. `μ_U` is unchanged by `σ`, so a tuple is matched exactly when its swap is. The body is unchanged, so it has the same truth on a tuple and its swap, and `σ` maps the automaton's acceptance sets and the monitor's rejecting state onto themselves. Each fairness constraint of `a` maps to the same constraint of `b`, and `σ` renames no key, so the fairness filter is invariant for `whole` and `each` constraints alike. The quotient is therefore bisimilar to the product on acceptance and fairness (Emerson and Sistla), and has a fair accepting cycle, or a reachable rejecting state, exactly when the product does. |
| CS-4 | **Counterexamples.** A counterexample found on the quotient is concretised as ADR-021 EI-6 concretises one, with the copy permutation in place of the key permutation: replay tracks which permutation maps each stored state to the concrete tuple, and a loop that closes on a permuted state is repeated until the concrete tuple closes, at most the order of that permutation. The result is an ordinary `HyperCounterexample` with its traces in quantifier order, and it replays by HX-2 or PA-7. |
| CS-5 | **Verdicts and identity.** A proof under copy-swap settles `Proved{basis: Reduced{reductions}, certification: Certified}` with `CopySwap{classes}` among its reductions (ADR-021 RV-1), once HX-7's product-closure checker accepts its certificate. A refutation names no reduction (ADR-021 RV-7). Swappability is a function of the checked clause, which the obligation identity binds (HV-7), so it adds no identity member; ADR-021 RV-8's identity rule covers request-selected reductions. |

## Consequences

- Noninterference, input-matched efficiency comparisons and other
  alternation-free hyperproperties over every behaviour of a finite model get
  `proved` or `refuted` from QSL's own explicit-state engine, with weak and
  strong fairness per trace variable and mixed interval formulas.
- `∀∃` properties with a safety body and unconstrained existential
  behaviours are decided exactly, and their refutations replay with no
  engine: the universal traces re-execute and the absence of a witness is
  recomputed.
- The author writes the property and the input match; the compiler builds the
  self-composition and the model stays as written.
- The product is quadratic for two copies and exponential in the number of
  copies; HP-3 is exponential in the existential side in the worst case.
  Every run limit is a caller-set budget, and reaching one stops the run as
  V-7 with the limit named.
- One `WitnessSet` construction serves hyperproperties and refinement with
  hidden fields.
- A reduction applies to a hyper item only by the PH rows: diagonal symmetry
  over one subject and compiler-verified copy-swap, never POR.
- A secret that changes the number of internal steps no longer reads as a
  leak: a universal safety clause can align on its visible steps.
- A relation over model operations gives two claims, one on the model's
  reachable transitions and one on the code under the declared invariants,
  each with its own evidence.
- A single-existential claim is a possibility claim, proved by a witness and
  refuted by a trap, with ADR-022's verdict map.

## Amendments made with this record

- ADR-014 §5 A-4: lasso evaluation covers a tuple of lassos in lockstep for a
  hyperproperty (HM-4).
- ADR-012 §3, the `TemporalTrace` row of the per-family assignment: the family
  parses and checks `hyper` clauses over `behaviours` and `relation` clauses
  over model executions.

## Amendments to make on acceptance

- ADR-021 PT-2: rows PH-1 to PH-6 and their reasons (§5); `CopySwap` in
  `ProofBasis::Reduced` (CS-5).
- ADR-022: HP-5 as a possibility form with a temporal body, its fairness set,
  its lasso witness and its product trap (SE-1 to SE-5).
- ADR-020 AX-2: the abstract-state set is the shared `WitnessSet` (HC-2).
- ADR-013 O-16 and the `TerminalValue` row: `MatchUndetermined` and
  `VacuousMatch`, both inconclusive.
- ADR-011 §6.1: layer A `qsl-analyze` `model_check` holds the exploration of
  the hyper, projected and witness-set products and tuple enumeration; layer
  5 `qsl-eval` their successor functions, the `WitnessSet` step, fairness
  enabledness and the copy-swap canonicaliser; layer 6 `qsl-replay` the
  facade entry `replay_model_trace_tuple`, the settlement map and the
  product-closure certificate checker.
- `spec/spec.md`: index row.

## Alternatives Considered

- **An undefined tuple settling `undefined`, counting as neither pass nor
  refutation.** Rejected by RU-5: every proof engine refutes an undefined
  claim (ADR-018 RU-5).

- **Hand-written self-composition in the model.** Rejected. It doubles every
  field and operation, hides the property, and its counterexample is a
  single trace of a model nobody wrote.
- **Match as a premise of the body only** (`always μ implies φ`). Rejected.
  It pairs every tuple and filters late, so the product keeps every unmatched
  joint step, and for `∀∃` it states a different property: an existential
  tuple that diverges in inputs would satisfy it vacuously. `μ_U` and `μ_E`
  restrict the domains, as FR-191 restricts a trace population before
  quantification.
- **Correlating objects across traces by name.** Rejected by QSpec FR-191.
  Universe keys correlate them explicitly (HS-3).
- **General asynchronous alignment with liveness bodies** (stuttering
  HyperLTL). Not taken. It is undecidable in general; HP-6 aligns on an
  author-named projection with a safety body, where the prefix reading keeps
  the check exact (RU-1).
- **Strategy-based `∀∃` checking.** Not taken for v1. It proves with a winning
  strategy and is incomplete without prophecies, so it refutes nothing on
  its own; HC-2 is exact on HP-3.
- **AutoHyper-style complementation for every `∀∃` body.** Not taken for v1.
  It is complete and costs a complementation exponential in the automaton
  size; HP-3's restriction avoids it.
- **Bounded QBF unrolling (HyperQube) for `∀∃`.** Not taken for v1. It
  belongs to the SMT lane and its sound refutations need the pessimistic
  bounded semantics; EN-2 covers HP-2 refutation.
- **Copy-swap as an author declaration.** Rejected. A declared symmetry the
  clause does not have would hide counterexamples; S3 verifies it (RU-4).
- **An exhaustive refutation of a possibility claim as
  `decisive-counterexample`.** Rejected. It carries no trace; its evidence is
  the closed scope, so its basis is `closed-scope` (SE-5).
- **Only reachable or only declared-type step relations.** Rejected. A model
  claim and a code claim answer different questions (RU-2).
- **A hyperproperty capability kind.** Rejected. QSpec FR-290's
  `temporal-satisfaction` already covers a temporal requirement over the
  traces of its profile, and the form beside the requirement routes it.

## References

- Owning ticket: Linear QSL-370. Related: VER-6, RES-18. Its QSpec half,
  QS-1 to QS-9, is QSpec FR-395 to FR-403 (Linear STD-136).
- The owner's rulings of §12 are recorded on the owning ticket.
- ADR-018 (Linear QSL-366; QSpec half QSpec FR-360 to FR-370, Linear
  STD-131). Sibling records: ADR-019
  (QSL-365, strong fairness), ADR-020 (QSL-367, refinement mappings), ADR-021
  (QSL-368, state-space reduction), ADR-022 (QSL-369, possible properties).
- M. R. Clarkson and F. B. Schneider, Hyperproperties (2010).
- M. R. Clarkson, B. Finkbeiner, M. Koleini, K. K. Micinski, M. N. Rabe and
  C. Sánchez, Temporal logics for hyperproperties (2014).
- G. Barthe, P. R. D'Argenio and T. Rezk, Secure information flow by
  self-composition (2004).
- B. Finkbeiner, M. N. Rabe and C. Sánchez, Algorithms for model checking
  HyperLTL and HyperCTL* (2015), MCHyper.
- N. Coenen, B. Finkbeiner, C. Sánchez and L. Tentrup, Verifying hyperliveness
  (2019).
- R. Beutner and B. Finkbeiner, Prophecy variables for hyperproperty
  verification (2022); AutoHyper: explicit-state model checking for HyperLTL
  (2023).
- T.-H. Hsu, C. Sánchez and B. Bonakdarpour, Bounded model checking for
  hyperproperties (2021), HyperQube.
- O. Kupferman and M. Y. Vardi, Model checking of safety properties (2001).
- J. A. Goguen and J. Meseguer, Security policies and security models (1982).
- E. A. Emerson and A. P. Sistla, Symmetry and model checking (1996).
- J. Baumeister, N. Coenen, B. Bonakdarpour, B. Finkbeiner and C. Sánchez, A
  temporal logic for asynchronous hyperproperties (2021).
