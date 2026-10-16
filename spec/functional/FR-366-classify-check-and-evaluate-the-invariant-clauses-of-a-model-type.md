---
id: FR-366
title: "Classify, check and evaluate the invariant clauses of a model type"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-359
    type: references
  - target: ix://agent-ix/quire-specification/FR-208
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-207
    type: depends_on
  - target: ix://agent-ix/quire-specification/TC-235
    type: references
---
# FR-366: Classify, check and evaluate the invariant clauses of a model type

## Description

A clause a domain package attaches to a type (`TypeDefinition.clauses`) is a
type-level clause. Intake keeps its `language` and `text` (FR-056) and
nothing else. This requirement gives that clause its meaning under QSpec
FR-208 and FR-208-AC-10: the engine SHALL classify every type-level clause as
an invariant, a guard or an unevaluated clause, SHALL parse and check each
invariant of a `quire` clause, and SHALL evaluate the invariants of a record
value type or an event type once for every value of the type that an
observation reaches, reporting a false result as a logical violation located
by its QSpec FR-207 runtime value path. QSpec TC-235 M06 run 4, M13 and M14
are the reference rows.

Today a type-level `quire` clause is retained and nothing parses, checks or
evaluates it: only the `requires` and `ensures` of an operation are
(FR-103), and a state clause a unit writes about an object type, state machine
or process is (FR-104, FR-107). FR-103's statement that a domain-package
invariant "evaluates nothing" is replaced by this requirement.

## Inputs

- One admitted domain package (FR-056) with its type-level clauses.
- For checking: each invariant's text parsed by the FR-146 expression parser,
  which the spine `compile` driver calls at S1/S2 as it does for an
  operation's kept clauses (FR-103). The assembler consumes parsed
  expressions and parses no text, so the semantics layer takes no dependency
  on the S1 parser.
- For evaluation: an `AdmittedObservations` value for a value-invariant
  selection (below), and a `Meter`.

## Outputs

- A `ClauseRole` for every type-level clause: `Invariant`, `Guard` or
  `Unevaluated`, and for every type its effective invariants.
- A checked package holding the checked invariants, or a refusal.
- `evaluate_value_invariants`: `Result<ValueInvariantEvaluation, CallFailure>`.

## Behavior

### Classification

- A type-level clause SHALL be classified `Unevaluated` when its language is
  not `quire`. It is kept with its text and source identity, never parsed,
  checked or evaluated (QSpec FR-208-AC-4).
- A `quire` clause SHALL be classified `Guard` when a transition of a state
  machine names it as its `guard`, and `Invariant` otherwise. A guard is not
  an invariant, is not conjoined to any invariant and is not evaluated by this
  requirement (QSpec FR-208-AC-10).
- A state machine specializing another keeps an inherited guard as a guard: a
  clause is `Guard` for the machine that declares it and for every
  specialization that inherits the transition naming it. A subtype
  transition whose guard names an ancestor's invariant is refused as FR-208-AC-12
  states.
- The effective invariants of a type SHALL be its own `Invariant` clauses
  followed by those of every ancestor under FR-208's generalization, each
  group ascending by clause key. A clause an ancestor classifies as `Guard` is
  in no descendant's effective invariants.
- The classification SHALL be a function of the admitted package alone: the
  same package gives the same roles in any document order.

### Parse and check

- `SelectedModel` SHALL gain `with_parsed_type_invariants`, taking the
  caller's FR-146 parser as `with_parsed_operation_clauses` does (FR-103), and
  parsing the text of each clause classified `Invariant`. A text the parser
  refuses stops the walk and is refused with the parser's refusal, located at
  `TypeDefinition.clauses[k]` of the owning type, `k` being the clause's
  position among that type's clauses in the package.
- The `ProtocolClause` family SHALL check each parsed invariant as a Boolean
  under FR-104's invariant rules, with `self` bound to the owner: for a record
  value type or an event type, a value of that type whose effective fields
  are readable by `self.f`; for an object type, a state machine or a process,
  as FR-104 binds `self` for a unit invariant on that context. An invariant
  has no operation parameter and no `result` (FR-104-AC-2).
- If an invariant reads `pre(e)`, then the checker SHALL refuse
  `wrong_snapshot`/`forbidden-pre-read`, naming the clause's declaration key and located at `TypeDefinition.clauses[k]`. No checked
  package is produced.
- If an invariant's root is not Boolean, then the checker SHALL refuse
  `ill_typed`/`non-boolean-root` at `TypeDefinition.clauses[k]`.
- A checked invariant SHALL be typed under the typer's profile-independent
  defaults, as a kept operation clause is (FR-103): no unit profile selection
  applies to it. Equality of two operands of a native `String`, `UUID` or
  `Timestamp` type is admitted as FR-091 states.
- A checked invariant adds no declaration, claim or requirement record.

### Evaluation per reachable value

A value-invariant selection admits one current observation and, optionally,
one invocation (FR-106). `evaluate_value_invariants` takes the
`AdmittedObservations` of that selection.

- A value of a record value type or event type is *reachable* when it is the
  value of a field of an object of the observation, the value of an
  operation parameter or result of the selected invocation, or a value
  contained in a reachable value: an element of a collection, the contained
  value of a present optional, a field of a reachable record value. An object
  reached by a `reference` is not followed: its own fields are read as that
  object's, once, in the observation.
- The evaluator SHALL evaluate each effective invariant of the value's
  most-specific type once for every reachable value, with `self` bound to that
  value and no object universe. A value reached at two locations is evaluated
  at each. The result for a value depends only on that value.
- A result SHALL be the FR-107 `Evaluation` of the invariant: `Completed(true)`
  or `Completed(false)`. A false result is a logical violation, a Boolean
  `false` with no diagnostic code or cause, never a refusal.
- Each result SHALL carry the QSpec FR-207 runtime value path of the value it
  was evaluated for:
  - a value under an object: the root observation is the selected current
    observation and the subject the declared object identity of the object
    (QSpec FR-204); the steps are `member` for the field, then `option_value`
    for a present optional, `index` for a collection element and `field` for
    a field of a record value;
  - a value of a parameter of the invocation: the root observation is the
    invocation's pre observation, the subject the invocation binding of the
    parameter, and the steps start empty;
  - a value of the result: the root observation is the invocation's post
    observation and the subject the result binding.
- The results SHALL be ordered by the FR-207 key order of their paths, then by
  clause key, whatever order the observation lists its objects or the
  invocation its parameters.
- `outcome` SHALL combine the verdicts as FR-107's clause set does: `False`
  when any result is `Completed(false)`, `True` when every result is
  `Completed(true)` and for an empty result list, and `Incomplete` when a
  charge is denied, the evaluation stopping at that result, which is its last.
  A denied charge is `resource_exhausted`/`insufficient-next-charge`.
- A type none of whose values the observation reaches has no result. A
  well-formed invariant of such a type is checked and reports nothing.
- An invariant of an object type, a state machine or a process is evaluated
  as a unit invariant on that context is: over the `Current` or `Context`
  selection of FR-106 by FR-107's `evaluate_clause`, `self` and the receiver
  path as FR-107 states. A `Guard` and an `Unevaluated` clause have no result
  in either evaluation.
- The evaluation SHALL be deterministic: the same package, observations and
  meter budget give the same results and the same charge log.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-366-AC-1 | Over K2 plus the `quire` clause `Extra` (`self.placed or not self.placed`) on `OrderLifecycle`, no transition naming it: `Extra` is `Invariant`, `NotYetPlaced` is `Guard` (of transition `[0]`) and in no effective-invariant list, and the effective invariants of `OrderLifecycle` are `[Extra]`. `Money/NonNegative` in `ocl` is `Unevaluated`. `ExpressLifecycle` (supertype `OrderLifecycle`) has `NotYetPlaced` as `Guard` and effective invariants `[Extra]`; `TaxedMoney` has effective invariants `[NonNegative]`. The classification is equal when the package's clause records are reordered. | Test (TC-1050) |
| FR-366-AC-2 | `Money/NonNegative` as `self.amount_minor >= 0` checks; with `pre(self.amount_minor) >= 0` the compile refuses `wrong_snapshot`/`forbidden-pre-read` naming `ix://test/orders/Money/NonNegative` at `TypeDefinition.clauses[0]` and produces no checked package; `OrderPlaced/Occurred` as `self.occurred_at = self.occurred_at` checks, and as `pre(self.order_id) = self.order_id` refuses the same, naming `ix://test/orders/OrderPlaced/Occurred` at `TypeDefinition.clauses[0]`. | Test (TC-1050) |
| FR-366-AC-3 | `NonNegative` as `self.amount_minor` refuses `ill_typed`/`non-boolean-root` at `TypeDefinition.clauses[0]`; as `self.amount_minor >=` the parser's refusal is returned at `TypeDefinition.clauses[0]`; a clause in `ocl` is not parsed whatever its text. An invariant in a second position of a type with two clauses is located at `TypeDefinition.clauses[1]`. | Test (TC-1050) |
| FR-366-AC-4 | TC-235 M13 run 1: with `Money/NonNegative` (`self.amount_minor >= 0`) and `Order` given `quoted: Money`, `history: Money {0, 3}`, optional `discount: Money` and operation `requote(amount: Money)`, `evaluate_value_invariants` over observation `s1` (`o1` with `total` 5, `quoted` -5, `history` [1, -2], `discount` present -1) and invocation `i1` of `requote` with `amount` -7 reports six results of `NonNegative`, `Completed(true)` at `o1.total` and at `o1.history[0]`, and `Completed(false)`, a Boolean with no code or cause, at exactly: `(s1, object o1)` `[member discount, option_value]`; `(s1, object o1)` `[member history, index 1]`; `(s1, object o1)` `[member quoted]`; `(pre observation of i1, invocation i1 parameter amount)` with no steps. The outcome is `False`. | Test (TC-1051) |
| FR-366-AC-5 | In AC-4 the six results are in the FR-207 key order of their paths, and listing `o1`'s fields or `i1`'s parameters in another order gives equal results; the result for a value is equal when the rest of the observation changes; two evaluations give equal results and charge logs. | Test (TC-1051) |
| FR-366-AC-6 | TC-235 M14 run 1: `OrderPlaced/Occurred` (`self.occurred_at = self.occurred_at`) is checked as an invariant over every `OrderPlaced` value, and over an observation holding one `Order` object and no `OrderPlaced` value `evaluate_value_invariants` has no result for `Occurred` and outcome `True`. | Test (TC-1051) |
| FR-366-AC-7 | A `TaxedMoney` value at the `Money` slot `quoted` is evaluated against `TaxedMoney`'s effective invariants; a `Money` record nested in a record `Pair { first: Money; second: Money }` field of an object is located by `[member pair, field first]` and `[member pair, field second]`, each evaluated; a value of an event type reached through a parameter is evaluated at the parameter binding. A `reference` field is not followed: the referenced object's values are reached once, as a member of its population. | Test (TC-1051) |
| FR-366-AC-8 | With a work budget that denies the third charge, the evaluation returns the results for the values before it and `Incomplete` with `resource_exhausted`/`insufficient-next-charge`, and no result for the later values. An evaluation of a `Guard` or an `Unevaluated` clause is not an `evaluate_value_invariants` result. | Test (TC-1051) |

## Dependencies

- FR-056 (intake keeps the clause records), FR-103 (the kept clause path whose
  parser seam this reuses), FR-104 (the invariant rules and receiver
  binding), FR-106 (admission of the current observation and the
  invocation), FR-107 (the `Evaluation`, the clause-set combination and the
  receiver path of an object, state machine or process invariant), FR-091
  (equality on native `String`, `UUID` and `Timestamp`) and FR-359 (a subtype
  value at an ancestor slot).
- QSpec FR-208 and FR-208-AC-10 (invariant meaning), FR-207 (the runtime value
  path), FR-204 (declared object identity) and TC-235 M06, M13, M14.
