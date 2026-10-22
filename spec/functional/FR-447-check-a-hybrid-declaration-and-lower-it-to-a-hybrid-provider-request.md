---
id: FR-447
title: "Check a hybrid declaration and its claims, and lower them to a hybrid provider request"
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
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: depends_on
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
---
# FR-447: Check a hybrid declaration and its claims, and lower them to a hybrid provider request

## Description

When a unit holds a `hybrid-decl` (QSpec FR-193 "Source form and the clock
line"), S2 (`qsl-forms`) SHALL build one declaration form for it, S3 SHALL
check it into a checked hybrid model on the checked package, and the request
writer SHALL lower each claim over that model to an FR-331 request item that
negotiation (FR-075) routes to a hybrid provider. The checked hybrid model
records each mode's rate of each continuous variable and classifies each
variable as a clock or as hybrid dynamics, which is ADR-026 HY-1's line
between EN-6 and an external solver. QSpec FR-193 owns the production and its
meaning; this requirement specifies the forms QSL builds, the checks and
refusals S3 applies, the checked record, and the request it lowers to.
FR-250 maps the provider's result to a verdict.

## Use case

A robotics engineer models a braking vehicle: in mode `Moving` the speed `v`
falls at the deceleration `a` and a timer `t` runs, and the vehicle jumps to
`Stopped` when `v` reaches 0. They claim that `v` stays below 30 for the
first 10 s.

```
hybrid Braking using p {
  mode Moving invariant { v >= 0 }
    flow { v' = -a; a' = 0; t' = 1; }
    transition to Stopped when { v = 0 } reset { v = 0 };
  mode Stopped invariant { v = 0 }
    flow { v' = 0; a' = 0; t' = 1; }
}
temporal SlowEnough using p over (h: Braking) clock "model-time" on origin {
  always [0, 10] h.v < 30
}
```

S3 records `v` and `a` as hybrid and `t` as a clock, and the claim routes to
the registered hybrid provider. Writing `transition to Halted` names no mode,
and S3 refuses it with a span.

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
    pub formula: HybridFormula,               // the temporal expression, atoms as HybridTerm
    pub horizon: Option<ExactRational>,       // seconds; None for an unbounded claim
}
```

`CheckedPackage::hybrid_models: Vec<CheckedHybridModel>` and
`CheckedPackage::hybrid_claims: Vec<CheckedHybridClaim>`, each in source
order. A typed `CheckRefusal` with a span on refusal.

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

- S3 SHALL classify a continuous variable as `Clock` when its rate is a
  numeric literal of exact value 1 in every mode, and every other
  continuous variable as `Hybrid` (ADR-026 HY-1).
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
- A hybrid claim SHALL write `clock "model-time"`, and its interval bounds
  SHALL count seconds. Another clock string SHALL refuse
  `unsupported_construct`/`declaration-form` at the clock clause's span.
- A hybrid claim SHALL activate `on origin` and carry no `fair` and no
  capture. An `on each` activation, a `fair` clause and a capture SHALL each
  refuse `unsupported_construct`/`declaration-form` at its span.
- In a hybrid claim's body, `h.x`, with `h` the `over` parameter, SHALL read
  the continuous variable `x`; a member that is no continuous variable SHALL
  refuse `missing_declaration`/`missing-name` at its span. Every atom SHALL
  take the expression forms of the previous section and lower to a
  `HybridTerm`.
- A hybrid claim SHALL have the horizon `Some(n)`, `n` the greatest upper
  bound of its intervals, when every temporal operator of its body carries
  an interval with a finite upper bound, and `None` otherwise.

### The checked record

- S3 SHALL record each checked hybrid model and each checked hybrid claim on
  the checked package, so two packages that differ in a mode, a rate, a
  guard, a reset, a classification or a claim have different package
  identities.
- The `CheckedHybridModel` SHALL be the one form the request writer and a
  hybrid provider read; neither re-reads source.

### The provider request

- The request writer SHALL write one FR-331 request item for each hybrid
  claim: its kind `temporal-satisfaction` (QSpec FR-290), its claim locus,
  its extent `bounded` when the claim's horizon is `Some` and `unbounded`
  when it is `None`, its `hybrid.limits` by FR-250, and the hybrid-dynamics
  requirement QSpec FR-193 names, for negotiation (FR-075) to route.
- The request writer SHALL give that requirement to every hybrid claim,
  since every checked hybrid model has a `Hybrid` variable (The clock line).
- A hybrid provider SHALL be an in-process provider (QSpec FR-331
  "In-process providers"; FR-280): it receives the request's manifest,
  `deadline_ms` and items as typed values, together with the checked
  package, and reads the claim's `CheckedHybridClaim` and its
  `CheckedHybridModel` from that package. QSL's built-in hybrid solver is
  such a provider, and FR-250's descriptor bounds are its bounds.

### Package emission

- If a checked package holds a checked hybrid model, then S4 SHALL refuse
  the package `unsupported_construct`/`not-yet-implemented`, naming the first
  hybrid declaration in source order, the feature name `hybrid-model` and
  the selected profile identity (QSpec FR-272), as FR-185 does for a
  probabilistic package. The checked package is unaffected, and an item over
  it routed to a process provider settles `unsupported` with a diagnostic
  (QSpec FR-331 "In-process providers").

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-447-AC-1 | The use case's `Braking` builds one `HybridForm` with modes `Moving` and `Stopped` in that order, `Moving` holding three equations and one transition to `Stopped`, each with its span; S3 checks it to a `CheckedHybridModel` whose variables are `v`, `a` and `t` in that order, classified `Hybrid`, `Hybrid` and `Clock`, whose `Moving` rates are `Negate(Variable(1))`, `Constant(0)` and `Constant(1)`, whose initial mode is `Moving`, and whose transition's reset is `[(0, Constant(0))]`. | Test (TC-1187) |
| FR-447-AC-2 | Each mutation of `Braking` refuses at its span with the cause given: a second mode `Moving`, `conflicting_declaration`/`duplicate-declaration`; `transition to Halted`, `missing_declaration`/`missing-name`; `Stopped` without `t' = 1;`, `invalid_model_binding`/`malformed-declaration` naming `Stopped` and `t`; a second `v' = 0;` in `Moving`, `conflicting_declaration`/`duplicate-declaration`; `P::v' = 0;`, `invalid_model_binding`/`malformed-declaration`; an invariant `{ v }`, `ill_typed`/`non-boolean-root`; a reset `{ v > 0 }`, `invalid_model_binding`/`malformed-declaration`; a reset `{ v = 0 and v = 1 }`, `conflicting_declaration`/`duplicate-declaration`; a guard reading `w`, `missing_declaration`/`missing-name`; a flow `v' = f(v)`, `unsupported_construct`/`expression-form`; and a declaration whose only equations are `t' = 1;` in each mode, `invalid_timed_model`/`rate-one-hybrid`. | Test (TC-1187) |
| FR-447-AC-3 | `SlowEnough` checks to a `CheckedHybridClaim` over `Braking` with horizon 10 and the atom `Compare(Lt, Variable(0), Constant(30))`; the same claim with `always h.v < 30` has horizon `None`. `clock "model-steps"`, `on each`, a `fair` clause and a capture are each refused `unsupported_construct`/`declaration-form`, and `h.w` is refused `missing_declaration`/`missing-name`. | Test (TC-1187) |
| FR-447-AC-4 | The request writer writes for `SlowEnough` one item of kind `temporal-satisfaction`, extent `bounded`, carrying the hybrid-dynamics requirement, and with an in-process fixture hybrid provider registered the item routes to it, which reads `Braking` from the checked package; with none registered it settles `unsupported`, `unsupported-requested-capability`. Two packages differing only in `Moving`'s rate of `v` have different package identities. | Test (TC-1187) |
| FR-447-AC-5 | S4 refuses the package holding `Braking` `unsupported_construct`/`not-yet-implemented` naming `Braking` and the feature `hybrid-model`, while the checked package remains and serves AC-4's in-process provider; the same item routed to a process provider settles `unsupported` with a diagnostic. | Test (TC-1187) |

## Dependencies

- QSpec FR-193 (the `hybrid-decl` production, its meaning, the clock line and
  the solver contract); QSpec FR-290 (the item kind); QSpec FR-331 (the
  request item, `hybrid.limits` and the in-process form); QSpec FR-272 (the
  causes).
- ADR-026 HY-1 to HY-3.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md),
  [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md),
  [FR-250](FR-250-map-a-hybrid-solver-result-to-a-verdict.md),
  [FR-280](FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md).
