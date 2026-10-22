---
id: FR-124
title: "Declare intended terminal states and derive the deadlock-freedom item"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-382
    type: traces_to
---
# FR-124: Declare intended terminal states and derive the deadlock-freedom item

## Description

A package SHALL declare the intended terminal states of a state model with a
`terminal` declaration that names the model's alias, at most one per model
alias (ADR-018 DL-1). The request writer SHALL add one derived
deadlock-freedom item for each distinct model subject among a request's
temporal items, unless that subject's state model declares `terminal any`
(ADR-018 DL-3). The item is checked like a reachable-state invariant
(TP-1) whose predicate `deadlocked` is derived, not authored (ADR-018 DL-2).

## Use case

A verification operator models a counter that counts up to 3 and stops. They
write `terminal when` a predicate true at 3, so the model checker treats
state 3 as an intended halt, while a state where the model is stuck by
mistake is still reported. Another model halts on purpose wherever it halts,
so its author writes `terminal any` and gets no deadlock report.

## Semantic authority and boundary

QSpec owns the surface grammar of the `terminal` declaration, the definition of
a deadlock, and the deadlock-freedom item's wire identity and place in the
QSpec FR-331 request (ADR-018 QS-12; References). This requirement specifies what
S3 checks once the shared grammar has parsed the declaration, how QSL classifies
terminal states, and what the request writer adds. The spellings `terminal
when` and `terminal any` follow ADR-018 DL-1 and are illustrative.

## Inputs

- The parsed `terminal` declaration of a package, which names one model alias,
  in one of its two forms:
  one that carries a state predicate `P` (written `terminal when P`) and one
  that marks every terminal state intended (written `terminal any`).
- For the request writer: the request's temporal items over model subjects
  (FR-125's `ModelSubject`), each with its checked package.

## Outputs

- On the checked state model: `TerminalDeclaration::{None, When(clause),
  Any}`, where `clause` is a checked state predicate.
- A `DeadlockFreedom` request item per subject: requirement (`temporal-
  satisfaction`, `Unbounded{domains}`), property form `ReachableInvariant`,
  and an obligation identity made of the subject and the fixed item kind
  `deadlock-freedom`.
- A typed `CheckRefusal` with a span on refusal.

## Behavior

### The `terminal` declaration

- S1 SHALL parse the `terminal` declaration as the shared grammar's `terminal-decl`,
  `terminal <name> using <alias> on <model> ( when <block> | any ) ;`, into one
  `TerminalDeclaration` node, and `terminal` SHALL be a reserved word of the
  complete grammar. S2 builds that node into a `TerminalDeclarationForm`
  (FR-382).
- The checker SHALL admit at most one `terminal` declaration per model alias.
  S2 builds a second declaration for one model alias (FR-382), and S3
  refuses it (FR-124 terminal check): the checker SHALL refuse
  `ambiguous_declaration`/`ambiguous-name` at its span, naming both
  declarations (QSpec FR-366).
- The checker SHALL check the predicate `P` of a `When` declaration as a state
  predicate over the model's state, through the one clause checker that
  invariants use (FR-104). If `P` reads a parameter, a result or a
  pre-state, then the checker SHALL refuse as FR-104 refuses that read in
  an invariant.
- Where the member types of the model's populations (the types its
  populations cover, not every declared object type) are exactly one object
  type `T`, the checker SHALL type `self` in `P` as `Reference<M::T>`.
- Where those populations cover no object type or several, the checker SHALL
  refuse `self` in `P` with `missing_declaration`/`missing-name` at `self`:
  no single type is the one `self` stands for, so `self` is not bound (the
  ambiguity is the declaration's, not a name resolving to several
  declarations).
- The checker SHALL admit in `P`, for a model with any number of object
  types, a quantifier or population form over a population `M::P` of the
  model (`forall(o in M::P : ...)`, `exists(o in M::P : ...)`, FR-391), and
  SHALL type each bound variable by that population. A `P` that does not read
  `self` SHALL check for every model.
- When `P` reads `self`, the deadlock classifier SHALL evaluate `P` once for
  each object of `T`'s population at the terminal state, and SHALL treat `P`
  as satisfied when it is true for every object, so a terminal state with no
  object of `T` satisfies `P` (as an invariant on `T` holds for no object,
  FR-104). It SHALL treat `P` as not satisfied when it is false for any
  object, and otherwise, when it is `Undefined` for some object, as
  `Undefined`.
- When `P` does not read `self`, the deadlock classifier SHALL evaluate `P`
  once at the terminal state (QSpec FR-366).
- When a state model has no `terminal` declaration, the checker SHALL check it
  to `TerminalDeclaration::None`.
- The checker SHALL include the `TerminalDeclaration` in the checked
  package, so two models that differ only in it have different package
  identities.

### Deadlocked states

- The deadlock classifier SHALL classify a state of a subject as
  **terminal** when no transition identity is enabled at it, which is when
  FR-120's successor relation gives it no successor (ADR-018 FA-2, DL-2).
- The deadlock classifier SHALL classify a terminal state as **intended**
  when the declaration is `Any`, or `When(P)` and `P` evaluates `true` at
  the state's observation by FR-107; as **deadlocked** when the
  declaration is `None`, or `When(P)` and `P` evaluates `false`.
- If `P` evaluates `Undefined` at a terminal state, then the classifier
  SHALL classify the state as neither intended nor deadlocked, and the
  deadlock-freedom item's letter at that state is undefined: the item
  settles `refuted` with cause `UndefinedEvaluation{where, cause}`
  (ADR-018 UE-1, FR-126).
- The deadlock classifier SHALL evaluate `P` only at terminal states, so a
  state that satisfies `P` and has a successor is not terminal.
- The deadlock classifier SHALL read the state graph alone; fairness
  constraints play no part in whether a state is deadlocked.

### The deadlock-freedom item

- The request writer SHALL add one `DeadlockFreedom` item per distinct model
  subject among the request's temporal items, so two items over equal
  subjects share one deadlock-freedom item.
- When a subject's state model declares `terminal any`, the request writer
  SHALL add no deadlock-freedom item for it.
- The request writer SHALL give the item the subject and the property
  `always holds(not deadlocked)`, routed, negotiated and settled as a
  `ReachableInvariant` item (FR-126, FR-127).
- The request writer SHALL make the item's obligation identity the subject
  and the kind `deadlock-freedom`, so it differs from every authored
  claim's identity over the same subject.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-124-AC-1 | The `Counter` unit (population `counters`, field `value: Int[0, 3]`, operation `inc` with precondition `self.value < 3` and postcondition `self.value = pre(self.value) + 1`) checks with each of: no `terminal` declaration (`None`), `terminal when` a predicate that holds exactly when every counter is at 3 (`When`), and `terminal any` (`Any`); the three packages have pairwise different identities. A second `terminal` declaration refuses `ambiguous_declaration`/`ambiguous-name` naming both declarations; a `terminal when` predicate that reads `pre(self.value)` refuses as FR-104 refuses it in an invariant. | Test (TC-519) |
| FR-124-AC-2 | A request with two temporal items over the same `Counter` subject (universe `{c}`, initial value 0) and the `None` declaration carries exactly one `DeadlockFreedom` item, with property form `ReachableInvariant`, requirement (`temporal-satisfaction`, `Unbounded`) and an obligation identity distinct from both authored items. With the `When` declaration it carries one; with the `Any` declaration it carries none. | Test (TC-519) |
| FR-124-AC-3 | Over the `Counter` subject, state `value = 3` is terminal; it is deadlocked under `None` and intended under `When` and `Any`. States `value = 0` to `2` are not terminal under any declaration. Adding `fair weak inc` to a request's items changes none of these classifications. | Test (TC-519) |
| FR-124-AC-4 | `terminal Done using Complete on M when { true };` and `terminal Halt using Complete on M any;` each parse to one `TerminalDeclaration` node whose text is the declaration; only the `when` declaration holds a `Block`. `terminal Done using Complete on M;` fails at `;`, `... on M when true;` fails at `true`, `terminal Done using Complete when { true };` fails at `when`, and `terminal using Complete on M any;` fails at `using`, each as `invalid_syntax`. | Test (TC-1060) |
| FR-124-AC-5 | In a model with one object type `Counter` (`value: Int[0, 3]`, population `counters`, universe `{c1, c2}`), `terminal Done using v on M when { self.value = 3 };` checks to `When`, and a terminal state is intended exactly when both counters hold 3. | Test (TC-519) |
| FR-124-AC-6 | In a model with two object types `Counter` (`value: Int[0, 3]`, population `counters`, universe `{c1}`) and `Gate` (`open: Boolean`, population `gates`, universe `{g1}`), operations `inc` on `Counter` (as `Counter` in AC-1) and `release` on `Gate` (precondition `not self.open`, postcondition `self.open`), `terminal Done using v on M when { forall(c in M::counters : c.value = 3) and forall(g in M::gates : g.open) };` checks to `When`; the terminal state with `c1.value = 3` and `g1.open` is intended, and no terminal state is reachable otherwise. | Test (TC-519) |
| FR-124-AC-7 | In the AC-6 model, `terminal Done using v on M when { self.value = 3 };` refuses `missing_declaration`/`missing-name` at `self`; in a model whose populations cover no object type, `terminal Done using v on M when { self.value = 3 };` refuses the same way and `terminal Done using v on M when { true };` checks to `When`. | Test (TC-519) |
| FR-124-AC-8 | In the AC-5 model with universe `{}`, the single state is terminal and intended under `terminal Done using v on M when { self.value = 3 };`; in the AC-5 model with universe `{c1}`, `terminal Done using v on M when { exists(c in M::counters : c.value = 3) };` is evaluated once at each terminal state, and with universe `{}` it is false there, so the state is deadlocked. | Test (TC-519) |

## Dependencies

- ADR-018 §10 DL-1 to DL-3 and DL-6.
- [FR-104](FR-104-check-state-clauses.md) (state predicate checking),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (evaluation at an
  observation), [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (successor relation), [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md)
  (the request writer), [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md)
  (state model members),
  [FR-391](FR-391-name-a-population-in-a-state-clause.md) (a population name
  in a terminal predicate).

## References

- QSpec FR-366 (deadlock freedom: the `terminal` declaration, the deadlock
  definition and the deadlock-freedom item), the QSpec half of ADR-018 QS-12
  (Linear STD-131).
