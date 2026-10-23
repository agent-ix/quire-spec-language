---
id: FR-447
title: "Check a hybrid declaration and its claims, emit them, and lower them to a hybrid provider request"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-193
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-272
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-481
    type: depends_on
---
# FR-447: Check a hybrid declaration and its claims, emit them, and lower them to a hybrid provider request

## Description

This requirement covers a unit that holds a `hybrid-decl` (QSpec FR-193
"Source form and the clock line"). S2 (`qsl-forms`) builds one declaration
form for it. S3 checks that form into a checked hybrid model on the checked
package. S4 emits the model and each claim over it as the checked-package v2
nodes QSpec FR-481 defines. The request writer lowers each claim to an FR-331
request item that negotiation (FR-075) routes to a hybrid provider, in
process or in a process of its own. The Behavior section states each stage's
rules. The checked hybrid model
records each mode's rate of each continuous variable and classifies each
variable as a clock or as hybrid dynamics, which is ADR-026 HY-1's line
between EN-6 and an external solver. QSpec FR-193 owns the production and its
meaning, and QSpec FR-481 the emitted nodes; this requirement specifies the
forms QSL builds, the checks and refusals S3 applies, the checked record, the
nodes S4 emits and the request it lowers to. FR-250 maps the provider's
result to a verdict.

## Use case

A robotics engineer models a braking vehicle: in mode `Moving` the speed `v`,
at most 20, falls at the deceleration `a`, between 1 and 2, and a timer `t`
runs, and the vehicle jumps to `Stopped` when `v` reaches 0. They claim that
`v` stays below 30 for the first 10 s.

```
hybrid Braking using p {
  mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }
    flow { v' = -a; a' = 0; t' = 1; }
    transition to Stopped when { v = 0 } reset { v = 0 };
  mode Stopped invariant { v = 0 }
    flow { v' = 0; a' = 0; t' = 1; }
}
temporal SlowEnough using p over (h: Braking) clock "model-time" on origin {
  always [0, 10] holds(h.v < 30)
}
```

`p` selects `quire.temporal.infinite-trace/v1`, so a claim may also write an
operator with no interval (FR-123). Every initial state has `v` at most 20,
and `v` only falls, so the claim holds. S3 records `v` and `a` as hybrid and
`t` as a clock, and the claim routes to the registered hybrid provider.
Writing `transition to Halted` names no mode, and S3 refuses it with a
span.

## Inputs

- A recovery-free `LosslessCst` of a unit holding a `hybrid-decl` and
  `temporal` clauses over it (ADR-011 E2).
- The `using` profile alias the declaration selects, resolved as for every
  native declaration.

## Outputs

At S2, `DeclarationForm` gains one variant:

```rust
pub struct HybridForm {                       // hybrid-decl
    pub name: Spanned<Identifier>,
    pub profile: Spanned<Identifier>,         // the `using` alias
    pub modes: Vec<HybridModeForm>,           // declared order
}
pub struct HybridModeForm {
    pub name: Spanned<Identifier>,
    pub invariant: Expression,
    pub equations: Vec<(Spanned<QualifiedName>, Expression)>, // `x' = e;`, declared order
    pub transitions: Vec<TransitionForm>,     // declared order
}
pub struct TransitionForm {
    pub target: Spanned<Identifier>,
    pub guard: Expression,
    pub reset: Expression,
}
```

At S3, on the checked package:

```rust
pub struct CheckedHybridModel {
    pub name: Identifier,
    pub variables: Vec<ContinuousVariable>,   // order of first appearance as an equation's left side
    pub modes: Vec<CheckedMode>,              // declared order; modes[0] is the initial mode
}
pub struct ContinuousVariable { pub name: Identifier, pub class: VariableClass }
pub enum VariableClass { Clock, Hybrid }
pub struct CheckedMode {
    pub name: Identifier,
    pub invariant: HybridTerm,
    pub rates: Vec<HybridTerm>,               // rates[i] is the rate of variables[i] in this mode
    pub transitions: Vec<CheckedTransition>,  // declared order
}
pub struct CheckedTransition {
    pub target: ModeIndex,
    pub guard: HybridTerm,
    pub resets: Vec<(VariableIndex, HybridTerm)>, // declared order
}
pub enum HybridTerm {
    Variable(VariableIndex),
    Constant(ExactRational),
    Boolean(bool),
    Negate(Box<HybridTerm>),
    Arithmetic(ArithmeticOp, Box<HybridTerm>, Box<HybridTerm>),   // + - * /
    Compare(CompareOp, Box<HybridTerm>, Box<HybridTerm>),         // < <= = >= >
    Not(Box<HybridTerm>),
    Logic(LogicOp, Box<HybridTerm>, Box<HybridTerm>),             // and or implies
}
pub struct CheckedHybridClaim {
    pub model: HybridModelIndex,
    pub formula: HybridFormula,
    pub horizon: Option<u64>,                 // whole seconds; None for an unbounded claim
}
pub enum HybridFormula {
    Holds(HybridTerm),                        // holds(e), and true and false
    Not(Box<HybridFormula>),
    Logic(LogicOp, Box<HybridFormula>, Box<HybridFormula>),       // and or implies
    Always(Option<HybridInterval>, Box<HybridFormula>),
    Eventually(Option<HybridInterval>, Box<HybridFormula>),
    Until(Option<HybridInterval>, Box<HybridFormula>, Box<HybridFormula>),
    Release(Option<HybridInterval>, Box<HybridFormula>, Box<HybridFormula>),
}
pub struct HybridInterval { pub lower: u64, pub upper: Option<u64> } // seconds; None for `*`
```

`CheckedPackage::hybrid_models: Vec<CheckedHybridModel>` and
`CheckedPackage::hybrid_claims: Vec<CheckedHybridClaim>`, each in source
order. A typed `CheckRefusal` with a span on refusal.

At S4, the emitted v2 package carries each checked hybrid model and claim as
QSpec FR-481's `hybrid` nodes and `temporal_clause` node.

## Behavior

### S2 forms

- `LeadingTokenKind` SHALL gain `Hybrid`, selecting exactly the production
  `hybrid_decl`.
- S2 SHALL build `HybridForm` from the production's CST, keeping every mode,
  equation and transition in declared order with its span.
- S2 SHALL leave names, types, the equation set and the classification
  unchecked for S3.

### Modes and transitions

- S3 SHALL refuse a second mode of one name in a declaration with
  `conflicting_declaration`/`duplicate-declaration` at the second mode's
  span, naming both modes.
- S3 SHALL resolve each transition's `to` name to a mode of the same
  declaration, and SHALL refuse a name that is none with
  `missing_declaration`/`missing-name` at its span.
- The first mode in declared order SHALL be the initial mode, and the
  initial continuous states SHALL be the states that satisfy its invariant
  at time 0.

### Continuous variables and flows

- The continuous variables of a declaration SHALL be the names on the left
  of its equations, across all its modes.
- An equation's left side SHALL be a single identifier. A qualified name
  with a `::` segment SHALL refuse `invalid_model_binding`/
  `malformed-declaration` at its span.
- Each mode SHALL give exactly one equation for each continuous variable. A
  second equation for one variable in a mode SHALL refuse
  `conflicting_declaration`/`duplicate-declaration` at its span, naming both
  equations; a mode with no equation for a continuous variable SHALL refuse
  `invalid_model_binding`/`malformed-declaration` at the mode's span, naming
  the mode and the variable.
- The right side of a variable's equation in a mode SHALL be that
  variable's rate in that mode, and S3 SHALL record it as
  `CheckedMode::rates`.
- A continuous variable's values SHALL be exact rationals; a rate, a reset
  value and an arithmetic operand SHALL be numeric, and a numeric expression
  of another type SHALL refuse `ill_typed`/`type-mismatch` at its span.

### The clock line

- S3 SHALL classify a continuous variable as `Clock` when its rate is the
  numeric literal `1` in every mode, and every other continuous variable as
  `Hybrid` (ADR-026 HY-1; QSpec FR-193 "Source form and the clock line").
  S3 folds no constant: a rate written `2 - 1` or `rational(2, 2)` is not
  the literal `1`, so its variable is `Hybrid`.
- If every continuous variable of a declaration is a `Clock`, then S3 SHALL
  refuse the declaration `invalid_timed_model`/`rate-one-hybrid` at its
  name's span, naming the timed model form (`time dense` with `Clock`
  fields, QSpec FR-415) that declares such a model (QSpec FR-193-AC-7).
  So every checked hybrid model has at least one `Hybrid` variable.

### Invariants, guards, resets and expression forms

- A mode's invariant and a transition's guard SHALL be Boolean; one of
  another type SHALL refuse `ill_typed`/`non-boolean-root` at its span.
- A reset SHALL be `true`, which keeps every variable, or a conjunction of
  equalities `x = e`, each `x` a continuous variable; after the jump `x`
  takes the value of `e` read over the values before the jump, and a
  variable no conjunct names keeps its value. Any other reset SHALL refuse
  `invalid_model_binding`/`malformed-declaration` at its span, and a second
  conjunct for one variable SHALL refuse
  `conflicting_declaration`/`duplicate-declaration`, naming both.
- In an invariant, a flow, a guard and a reset, a bare name SHALL resolve to
  a continuous variable of the declaration; any other name SHALL refuse
  `missing_declaration`/`missing-name` at its span.
- S3 SHALL admit in these expressions exactly the forms `HybridTerm` holds:
  continuous variables, numeric literals and `exact-value`s read exactly,
  `true` and `false`, unary `-`, `+`, `-`, `*`, `/`, the comparisons `<`,
  `<=`, `=`, `>=` and `>`, and `not`, `and`, `or` and `implies`. Any other
  expression form SHALL refuse `unsupported_construct`/`expression-form` at
  its span, naming the form and the declaration.
- S3 SHALL lower each admitted expression to its `HybridTerm`, a variable by
  its index in `CheckedHybridModel::variables`.

### Hybrid claims

- A `temporal` clause whose `over` parameter's type is the name of a hybrid
  declaration of the unit SHALL be a hybrid claim over that declaration's
  checked model, and S3 SHALL check it by this section in place of the
  model-subject rules (FR-123, FR-233).
- A hybrid claim's `using` profile SHALL be
  `quire.temporal.infinite-trace/v1`, the profile whose integer intervals
  QSpec FR-481 emits; a hybrid claim whose profile is another SHALL refuse
  `unsupported_construct`/`declaration-form` at the `using` alias's span,
  naming the profile.
- A hybrid claim SHALL write `clock "model-time"`, and its interval bounds
  SHALL count seconds. Another clock string SHALL refuse
  `unsupported_construct`/`declaration-form` at the clock clause's span.
- A hybrid claim SHALL activate `on origin` and carry no `fair` and no
  capture. An `on each` activation, a `fair` clause and a capture SHALL each
  refuse `unsupported_construct`/`declaration-form` at its span.
- In a hybrid claim's body, `h.x`, with `h` the `over` parameter, SHALL read
  the continuous variable `x`; a member that is no continuous variable SHALL
  refuse `missing_declaration`/`missing-name` at its span. The expression of
  each `holds(...)` SHALL take the expression forms of the previous section
  and lower to a `HybridTerm`; `true` and `false` lower to
  `Holds(Boolean(..))`.
- S3 SHALL lower a hybrid claim's body to a `HybridFormula`: `not`, `and`,
  `or` and `implies`, and the future operators `always`, `eventually`,
  `until` and `release`, each with its interval when it writes one. The past
  operators `once`, `historically`, `since` and `triggered`, and any
  `holds(...)` over a hyper expression, SHALL refuse
  `unsupported_construct`/`expression-form` at the operator's span, naming
  the operator.
- An operator with no interval is admitted only where the unit's profile
  admits it (FR-123), and its subformula is unbounded.
- S3 SHALL compute the claim's horizon as the reach `r` of its formula,
  the latest time in seconds whose state the formula reads:
  `r(Holds(_))` is 0; `r(Not(f))` is `r(f)`; `r(Logic(_, f, g))` is the
  greater of `r(f)` and `r(g)`; `r(Always(i, f))` and `r(Eventually(i, f))`
  are `u + r(f)`; and `r(Until(i, f, g))` and `r(Release(i, f, g))` are
  `u` plus the greater of `r(f)` and `r(g)`, where `u` is `i`'s upper bound.
  The reach is unbounded when any operator on the way has no interval or
  the upper bound `*`, or any operand's reach is unbounded. The horizon SHALL
  be `Some(r)` when the reach is bounded and `None` otherwise. So
  `always [0, 10] eventually [0, 5] holds(h.v < 30)` has horizon 15.

### The checked record

- S3 SHALL record each checked hybrid model and each checked hybrid claim on
  the checked package, and the checked package identity (FR-003, FR-259)
  SHALL take them into its preimage: each `CheckedHybridModel`, then each
  `CheckedHybridClaim`, in source order, as one quire-canonical (FR-259) JSON
  object of its members in the order this requirement declares them, each
  enum value an object whose `type` member is the variant name in
  kebab-case and whose other members are its fields. So two packages that
  differ in a mode, a rate, a guard, a reset, a classification or a claim
  have different checked package identities.
- The `CheckedHybridModel` SHALL be the one form the request writer and an
  in-process hybrid provider read, and its emitted nodes (Package emission)
  the one form a process hybrid provider reads; none re-reads source.

### The provider request

- The request writer SHALL write one FR-331 request item for each hybrid
  claim: its kind `temporal-satisfaction` (QSpec FR-290), its claim locus,
  its extent `bounded` when the claim's horizon is `Some` and `unbounded`
  when it is `None`, its `hybrid.limits` by FR-250, and the hybrid-dynamics
  requirement QSpec FR-193 names, for negotiation (FR-075) to route.
- The request writer SHALL give that requirement to every hybrid claim,
  since every checked hybrid model has a `Hybrid` variable (The clock line).
- The item's `claim` SHALL name the claim's emitted `temporal_clause` node
  and its `claim` occurrence (QSpec FR-331 "Items", FR-481 "Hybrid claims"),
  so the item has one locus for every provider.
- Negotiation SHALL route the item to any registered provider that
  advertises the item's kind and requirement, in process or in a process of
  its own; no rule restricts a hybrid item to either.
- An in-process hybrid provider (QSpec FR-331 "In-process providers";
  FR-280) receives the request's manifest, `deadline_ms` and items as typed
  values, together with the checked package, and reads the claim's
  `CheckedHybridClaim` and its `CheckedHybridModel` from that package. QSL's
  built-in hybrid solver is such a provider, and FR-250's descriptor bounds
  are its bounds.
- A process hybrid provider, such as one that runs dReal or Flow*, receives
  the request frame with the emitted package's v2 bytes (QSpec FR-331
  "Package bytes") and reads the claim's `temporal_clause` node and its
  model's `hybrid` nodes from those bytes (QSpec FR-481).

### Package emission

- S4 SHALL emit a checked package that holds a checked hybrid model, with
  each model and each claim over it as the nodes QSpec FR-481 defines:
  - each `CheckedHybridModel` as one `hybrid`/`hybrid_model` node declared by
    its name, with each `ContinuousVariable` a `value`/`parameter` node named
    by it at the level of its index, typed `scalar_type`/`rational`, and its
    `class` the text `clock` or `hybrid`;
  - each `CheckedMode` as one `hybrid`/`hybrid_mode` node declared by the
    model's name and the mode's name, its `invariant`, its `rates` in
    variable order and its `transitions` in declared order;
  - each `CheckedTransition` as one `hybrid`/`hybrid_transition` node with
    its target `ModeIndex`, its guard and its resets in declared order, each
    by its `VariableIndex`;
  - each `HybridTerm` as a term node: `Variable(i)` a `reference` to
    variable `i`'s node; `Constant(q)` a `value`/`literal` node of `q` typed
    `scalar_type`/`rational`; `Boolean(b)` a `value`/`literal` node of `b`;
    and `Negate`, `Arithmetic`, `Compare`, `Not` and `Logic` an `expression`
    node applying the catalogued `quire.op.rational.*` or
    `quire.op.boolean.*` operation of that operator to the nodes of its
    operands;
  - each `CheckedHybridClaim` as the `temporal`/`temporal_clause` node of
    its `temporal` clause, whose `over` parameter is typed by its model's
    `hybrid_model` node, with the `temporal_profile` law
    `quire.temporal.infinite-trace/v1`, clock `model-time`, activation
    `origin` and its `HybridFormula` as QSpec FR-370 formula nodes, each
    `HybridInterval` in the integer form. `Holds(Boolean(true))` and
    `Holds(Boolean(false))` SHALL be the formula nodes of
    `quire.op.temporal.true` and `quire.op.temporal.false`, and every other
    `Holds(t)` the node of `quire.op.temporal.holds` over `t`'s term node,
    so one claim has one encoding and one `package_id`. The horizon is not emitted: a reader derives it from
    the formula by the reach rule of "Hybrid claims" above.
- QSL's I2 reader SHALL admit an emitted package's hybrid nodes and read
  each `hybrid_model` node and its modes and transitions back into the
  `CheckedHybridModel` it was emitted from, and each hybrid claim's
  `temporal_clause` node into its `CheckedHybridClaim`, the horizon derived
  by the reach rule of "Hybrid claims".
- The I2 reader SHALL refuse a hybrid node or hybrid claim that breaks a
  QSpec FR-481 rule with FR-481's code at FR-481's locus, in FR-481's
  reader order, and a layout outside QSpec's published schema `malformed_wire`
  at strict wire validation.
- The emitted package's `package_id` SHALL take every hybrid node, so two
  packages that differ in a mode, a rate, a guard, a reset, a
  classification or a claim have different `package_id`s.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-447-AC-1 | The use case's `Braking` builds one `HybridForm` with modes `Moving` and `Stopped` in that order, `Moving` holding three equations and one transition to `Stopped`, each with its span; S3 checks it to a `CheckedHybridModel` whose variables are `v`, `a` and `t` in that order, classified `Hybrid`, `Hybrid` and `Clock`, whose `Moving` rates are `Negate(Variable(1))`, `Constant(0)` and `Constant(1)`, whose initial mode is `Moving`, and whose transition's reset is `[(0, Constant(0))]`. | Test (TC-1187) |
| FR-447-AC-2 | Each mutation of `Braking` refuses at its span with the cause given: a second mode `Moving`, `conflicting_declaration`/`duplicate-declaration`; `transition to Halted`, `missing_declaration`/`missing-name`; `Stopped` without `t' = 1;`, `invalid_model_binding`/`malformed-declaration` naming `Stopped` and `t`; a second `v' = 0;` in `Moving`, `conflicting_declaration`/`duplicate-declaration`; `P::v' = 0;`, `invalid_model_binding`/`malformed-declaration`; an invariant `{ v }`, `ill_typed`/`non-boolean-root`; a reset `{ v > 0 }`, `invalid_model_binding`/`malformed-declaration`; a reset `{ v = 0 and v = 1 }`, `conflicting_declaration`/`duplicate-declaration`; a guard reading `w`, `missing_declaration`/`missing-name`; a guard `when { v + 1 }`, `ill_typed`/`non-boolean-root`; a flow `v' = true;`, `ill_typed`/`type-mismatch`; a flow `v' = f(v)`, `unsupported_construct`/`expression-form`; and a declaration whose only equations are `t' = 1;` in each mode, `invalid_timed_model`/`rate-one-hybrid`. | Test (TC-1187) |
| FR-447-AC-3 | `SlowEnough` checks to a `CheckedHybridClaim` over `Braking` with the formula `Always(Some([0, 10]), Holds(Compare(Lt, Variable(0), Constant(30))))` and horizon 10; with the body `always [0, 10] eventually [0, 5] holds(h.v < 30)` the horizon is 15, with `holds(h.v < 30) until [0, 4] holds(h.v = 0)` it is 4, and with `always holds(h.v < 30)` or `always [0, *] holds(h.v < 30)` it is `None`. `once [0, 2] holds(h.v = 0)` is refused `unsupported_construct`/`expression-form`; `clock "model-steps"`, `on each`, a `fair` clause and a capture are each refused `unsupported_construct`/`declaration-form`; and `h.w` is refused `missing_declaration`/`missing-name`. | Test (TC-1187) |
| FR-447-AC-4 | The request writer writes for `SlowEnough` one item of kind `temporal-satisfaction`, extent `bounded`, carrying the hybrid-dynamics requirement, whose `claim` names `SlowEnough`'s emitted `temporal_clause` node and its `claim` occurrence; with an in-process fixture hybrid provider registered the item routes to it, which reads `Braking` from the checked package; with only a process fixture hybrid provider registered the item routes to it, which receives the package's v2 bytes and reads `Braking` and `SlowEnough` from their QSpec FR-481 nodes; with none registered it settles `unsupported`, `unsupported-requested-capability`. Two packages differing only in `Moving`'s rate of `v` have different checked package identities. | Test (TC-1187) |
| FR-447-AC-5 | S4 emits the package holding `Braking` and `SlowEnough`, and QSpec's v2 reader admits it: one `hybrid`/`hybrid_model` node declared `[Braking]` whose variables are the parameter nodes of `v`, `a` and `t` at levels 0 to 2 classed `hybrid`, `hybrid` and `clock`, whose modes are `[Braking, Moving]` and `[Braking, Stopped]` in that order, `Moving`'s rates referencing the nodes of `-a`, `0` and `1` and its one transition node `target` 1 with the reset `(0, 0)`; and one `temporal_clause` node over a parameter typed by the model node, clock `model-time`. Replacing `Moving`'s `v' = -a` by `v' = -(2 * a)` changes the `package_id`. | Test (TC-1187) |
| FR-447-AC-6 | QSL's I2 reader admits AC-5's emitted package and reads back `Braking`'s `CheckedHybridModel` and `SlowEnough`'s `CheckedHybridClaim` with horizon 10, each equal to the checked one; it admits QSpec's `positive-all-families.json`, whose `Tank` model has a bare-variable rate and reset and a `hybrid` variable whose rates are all the literal `1`; and, with `dependencies`, keys and `package_id` rebuilt after each edit, it refuses `Moving` with a fourth rate `invalid_model_binding`/`malformed-declaration` at `rates`, every variable classed `clock` `invalid_package`/`invalid-value` at `v`'s class literal, and `SlowEnough`'s clock `model-steps` `invalid_package`/`invalid-value` at `/body/arguments/1`, each as QSpec FR-481 orders them; a model body with `modes` before `variables` refuses `malformed_wire`. | Test (TC-1187) |
| FR-447-AC-7 | `SlowEnough` written `using` a profile alias selecting `quire.temporal.timed/v1` refuses at S3 `unsupported_construct`/`declaration-form` at the alias's span, naming the profile; a claim body `always [0, 10] true` emits a `quire.op.temporal.true` formula node under the `always` node, and no `quire.op.temporal.holds` node over a Boolean literal. | Test (TC-1187) |

## Dependencies

- QSpec FR-193 (the `hybrid-decl` production, its meaning, the clock line and
  the solver contract); QSpec FR-290 (the item kind); QSpec FR-331 (the
  request item, `hybrid.limits`, the package bytes and the in-process form);
  QSpec FR-322 and FR-481 (the emitted nodes and their reader order);
  QSpec FR-272 (the causes).
- ADR-026 HY-1 to HY-3.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md),
  [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md),
  [FR-250](FR-250-map-a-hybrid-solver-result-to-a-verdict.md),
  [FR-280](FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md).
