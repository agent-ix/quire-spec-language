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
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-233
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-450
    type: references
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

They then start the vehicle either moving at full speed or already stopped,
and claim that, from every instant at which it moves faster than 10, it slows
below that speed within 10.5 s, under weak fairness on the stopping
transition:

```
hybrid Braking2 using p {
  initial {
    mode Moving when { v = 20 and a >= 1 and a <= 2 and t = 0 };
    mode Stopped when { v = 0 and t = 0 };
  }
    mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }
    flow { v' = -a; a' = 0; t' = 1; }
    transition to Stopped when { v = 0 } reset { v = 0 and a >= 1 };
  mode Stopped invariant { v = 0 }
    flow { v' = 0; a' = 0; t' = 1; }
}
temporal Slows using t over (h: Braking2) clock "model-time"
    on each (q: Braking2) when (q.v > 10) {
  fair Moving::Stopped;
  capture v0: Rational[0, 20; 1, 1] = q.v;
    eventually [0 s, 10500 ms] holds(h.v < v0 and h.v <= 10)
}
temporal Recent using p over (h: Braking2) clock "model-steps" on origin {
  once [0, 2] holds(h.v = 20)
}
```

`t` selects `quire.temporal.timed/v1`, whose intervals take exact rational
bounds such as `10500 ms`; the stopping transition's reset keeps `a` in its
range, as a non-deterministic bound.

`p` selects `quire.temporal.infinite-trace/v1`, so a claim may also write an
operator with no interval and a fairness constraint (FR-123). Every initial
state has `v` at most 20, and `v` only falls, so `SlowEnough` holds. S3 records `v` and `a` as hybrid and
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
    pub initial: Option<Spanned<Vec<InitialEntryForm>>>, // `initial { ... }`, None when unwritten
    pub modes: Vec<HybridModeForm>,           // declared order
}
pub struct InitialEntryForm {                 // `mode M when { c };`
    pub mode: Spanned<Identifier>,
    pub condition: Expression,
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
    pub initial: Vec<InitialEntry>,           // source order; [{mode: 0, condition: Boolean(true)}] when unwritten
    pub modes: Vec<CheckedMode>,              // declared order
}
pub struct InitialEntry { pub mode: ModeIndex, pub condition: HybridTerm }
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
    pub resets: Vec<ResetConstraint>,         // declared order
}
pub struct ResetConstraint {                  // `x ~ e`: the post-jump x against e over the pre-jump state
    pub variable: VariableIndex,
    pub relation: CompareOp,
    pub value: HybridTerm,
}
pub enum HybridTerm {
    Variable(VariableIndex),                  // at the current position; in a reset, the pre-jump value
    ActivationVariable(VariableIndex),        // `q.x` of an `on each (q: D)` activation
    Capture(CaptureIndex),
    Constant(ExactRational),
    Boolean(bool),
    Negate(Box<HybridTerm>),
    Arithmetic(ArithmeticOp, Box<HybridTerm>, Box<HybridTerm>),   // + - * /
    Compare(CompareOp, Box<HybridTerm>, Box<HybridTerm>),         // = != < <= >= >
    Not(Box<HybridTerm>),
    Logic(LogicOp, Box<HybridTerm>, Box<HybridTerm>),             // and or implies
    Conditional(Box<HybridTerm>, Box<HybridTerm>, Box<HybridTerm>), // if c then a else b
    Call(CheckedFunctionRef, Vec<HybridTerm>), // a pure function (FR-062), arguments in order
}
pub struct CheckedHybridClaim {
    pub model: HybridModelIndex,
    pub clock: HybridClock,
    pub activation: HybridActivation,
    pub captures: Vec<HybridCapture>,         // source order
    pub fairness: Vec<HybridFairness>,        // source order of first occurrence
    pub formula: HybridFormula,
    pub horizon: Option<HybridHorizon>,       // None for an unbounded claim
}
pub enum HybridClock { ModelTime, ModelSteps }
pub enum HybridActivation { Origin, Each { guard: Option<HybridTerm> } }
pub struct HybridCapture { pub name: Identifier, pub ty: CheckedTypeRef, pub value: HybridTerm }
pub struct HybridFairness {
    pub kind: FairnessKind,                   // Weak | Strong
    pub granularity: FairnessGranularity,     // Whole | Each
    pub transitions: Vec<(ModeIndex, TransitionIndex)>, // the named transitions, source order
}
pub enum HybridHorizon { Seconds(ExactRational), Positions(u64) }
pub enum HybridFormula {
    Holds(HybridTerm),                        // holds(e), and true and false
    Not(Box<HybridFormula>),
    Logic(LogicOp, Box<HybridFormula>, Box<HybridFormula>),       // and or implies
    Always(Option<HybridInterval>, Box<HybridFormula>),
    Eventually(Option<HybridInterval>, Box<HybridFormula>),
    Until(Option<HybridInterval>, Box<HybridFormula>, Box<HybridFormula>),
    Release(Option<HybridInterval>, Box<HybridFormula>, Box<HybridFormula>),
    Historically(Option<HybridInterval>, Box<HybridFormula>),
    Once(Option<HybridInterval>, Box<HybridFormula>),
    Since(Option<HybridInterval>, Box<HybridFormula>, Box<HybridFormula>),
    Triggered(Option<HybridInterval>, Box<HybridFormula>, Box<HybridFormula>),
}
pub struct HybridInterval {                   // seconds under ModelTime, positions under ModelSteps
    pub lower: ExactRational,
    pub lower_open: bool,
    pub upper: Option<ExactRational>,         // None for `*`
    pub upper_open: bool,
}
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
- S1 SHALL parse an optional `hybrid-initial` as the first member of a
  `hybrid_decl` body (QSpec shared grammar), `initial` being a keyword only
  there, and S2 SHALL build `HybridForm::initial` from it, each entry with
  its span, and `None` when the clause is unwritten.
- S1 SHALL parse a `timed-interval`, and its `[q, *]` form, in place of an
  `interval` in every temporal clause, telling it apart by its second token
  (QSpec shared grammar); S1 does not resolve the `over` type, so S3 SHALL
  refuse the form `unsupported_construct`/`expression-form` at its span in a
  clause that is neither a hybrid claim nor under `quire.temporal.timed/v1`.
- S2 SHALL leave names, types, the equation set and the classification
  unchecked for S3.

### Modes and transitions

- S3 SHALL refuse a second mode of one name in a declaration with
  `conflicting_declaration`/`duplicate-declaration` at the second mode's
  span, naming both modes.
- S3 SHALL resolve each transition's `to` name to a mode of the same
  declaration, and SHALL refuse a name that is none with
  `missing_declaration`/`missing-name` at its span.

### Initial states

- S3 SHALL check each initial entry: its mode name SHALL resolve to a mode
  of the declaration, else refuse `missing_declaration`/`missing-name` at
  its span; its condition SHALL be Boolean, else refuse
  `ill_typed`/`non-boolean-root` at its span, and SHALL take the expression
  forms of "Invariants, guards, resets and expression forms".
- S3 SHALL record the entries as `CheckedHybridModel::initial` in source
  order. Two entries may name one mode, and every mode an entry names is an
  initial mode.
- With no `initial` clause, S3 SHALL record the one entry
  `InitialEntry { mode: 0, condition: Boolean(true) }`, the first mode in
  declared order with the condition `true`, which is what
  `initial { mode M when { true }; }` checks to.
- The initial states SHALL be those QSpec FR-193 "Initial states" defines:
  the states at time 0 that satisfy an entry's condition and its mode's
  invariant.

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
- A continuous variable SHALL range over the real numbers, and every
  constant SHALL be an exact rational (QSpec FR-193 "Continuous values and
  expressions"); a rate, a reset value and an arithmetic operand SHALL be
  numeric, and a numeric expression of another type SHALL refuse
  `ill_typed`/`type-mismatch` at its span.

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
- A reset SHALL be `true`, which keeps every variable, or a conjunction,
  possibly under `let` binders, of conjuncts `x ~ e` (QSpec FR-193
  "Resets"): `x` a continuous variable naming its value after the jump, `~`
  one of `=`, `!=`, `<`, `<=`, `>=` and `>`, and `e` any expression of this
  section. Every variable read in `e`, and `pre(x)`, SHALL read the value
  before the jump (the pre-jump reading rule), and S3 SHALL lower `pre(x)`
  in a reset to `Variable(x)`, which a reset reads before the jump.
- S3 SHALL record each conjunct as one `ResetConstraint` in declared order.
  After the jump a variable with an `=` constraint takes its value, a
  variable with only other constraints takes any real value that satisfies
  them all, and a variable with none keeps its value.
- A variable with an `=` conjunct and a second conjunct SHALL refuse
  `conflicting_declaration`/`duplicate-declaration` at the second, naming
  both. A conjunct whose left operand is not a continuous variable, and a
  reset that is not such a conjunction, SHALL refuse
  `invalid_model_binding`/`malformed-declaration` at its span. `pre(...)`
  outside a reset SHALL refuse `invalid_model_binding`/
  `malformed-declaration` at its span.
- In an invariant, a flow, a guard, a reset and an initial condition, a bare
  name SHALL resolve to a continuous variable of the declaration or to a
  `let` binder in scope, and a call's name to a pure function of the unit or
  its libraries (FR-062); any other name SHALL refuse
  `missing_declaration`/`missing-name` at its span.
- S3 SHALL admit in these expressions the forms QSpec FR-193 "Continuous
  values and expressions" admits: continuous variables, numeric literals,
  `exact-value`s and `rational(n, d)` read exactly, `true` and `false`,
  unary `-`, `+`, `-`, `*` and `/` (products and quotients of variables
  included), the comparisons `=`, `!=`, `<`, `<=`, `>=` and `>`, `not`,
  `and`, `or` and `implies`, `let`, `if ... then ... else`, and a call of a
  pure function whose parameters and result are `Boolean` or numeric and
  whose body takes these forms, checked by FR-062 with its own `decreases`
  rule. A form whose operand or result is neither real nor Boolean, an
  integer operation (`div`, `rem`, `mod`) over a real operand, and a real
  argument to an `Integer` or `Int` parameter SHALL refuse
  `ill_typed`/`type-mismatch` at its span; a call whose function body holds
  another form SHALL refuse the same way at the call, naming the form.
- S3 SHALL lower each admitted expression to its `HybridTerm`: a variable by
  its index in `CheckedHybridModel::variables`, `let x = e in b` by
  substituting `e`'s term for `x` in `b`, `if` to `Conditional`, and a call
  to `Call` with the checked function and its argument terms. S3 folds no
  constant and expands no call.

### Hybrid claims

- A `temporal` clause whose `over` parameter's type is the name of a hybrid
  declaration of the unit SHALL be a hybrid claim over that declaration's
  checked model, and S3 SHALL check it by this section in place of the
  model-subject rules (FR-123, FR-233), with the meaning QSpec FR-193
  "Hybrid claims" gives it.
- **Clock.** A hybrid claim with no `clock` member, or with
  `clock "model-time"`, SHALL bind `HybridClock::ModelTime`, and one with
  `clock "model-steps"` SHALL bind `HybridClock::ModelSteps` (QSpec FR-252:
  a hybrid model is a timed subject in unit second). A model subject has
  only these two bindings, so any other clock string SHALL refuse
  `unsupported_construct`/`declaration-form` at the clock clause's span.
- **Activation.** `on origin` SHALL check to `HybridActivation::Origin`.
  `on each (q: D)` SHALL check to `Each { guard: None }` and
  `on each (q: D) when (g)` to `Each { guard: Some(g) }`, `g` Boolean and in
  the expression forms above. `D` SHALL be the `over` parameter's
  declaration, else S3 SHALL refuse `ill_typed`/`type-mismatch` at its
  type. `q.x` SHALL lower to `ActivationVariable(x)`.
- **Captures.** Each `capture n: T = e;` SHALL check to a `HybridCapture`:
  `T` `Boolean` or a numeric type, else `ill_typed`/`type-mismatch` at `T`;
  `e` of `T`'s kind and in the expression forms above, over `h`, `q` and the
  earlier captures. A capture name SHALL lower to `Capture(i)`, `i` its
  position. A second capture of one name SHALL refuse
  `conflicting_declaration`/`duplicate-declaration`, naming both.
- **Fairness.** Each fairness constraint SHALL check to a `HybridFairness`
  with its kind and granularity as FR-123 resolves them (`Weak` and `Whole`
  when unwritten), its name resolved to hybrid transitions: a mode `S` to
  every transition of `S`, and `S::T` to the transitions of `S` whose target
  is `T`. A name that resolves to no transition SHALL refuse
  `missing_declaration`/`missing-name` at its span. A constraint equal to an
  earlier one in kind, granularity and transitions SHALL be held once. A
  fairness constraint on a clause whose profile is bounded SHALL refuse
  `unsupported_construct`/`expression-form` at its span, as FR-123 refuses it.
- **Body.** In a hybrid claim's body, `h.x`, with `h` the `over` parameter,
  SHALL read the continuous variable `x` and lower to `Variable(x)`; a member
  that is no continuous variable SHALL refuse
  `missing_declaration`/`missing-name` at its span. The expression of each
  `holds(...)` SHALL take the expression forms above and lower to a
  `HybridTerm`; `true` and `false` lower to `Holds(Boolean(..))`.
- S3 SHALL lower a hybrid claim's body to a `HybridFormula`: `not`, `and`,
  `or` and `implies`, the future operators `always`, `eventually`, `until`
  and `release`, and the past operators `historically`, `once`, `since` and
  `triggered`, each with its interval when it writes one. A `holds(...)` over
  a hyper expression SHALL refuse `unsupported_construct`/`expression-form`
  at its span, a hyper expression reading traces that a temporal clause does
  not bind.
- An operator with no interval, and an interval with the upper bound `*`,
  are admitted only where the unit's profile admits them (FR-123); the
  subformula of such a future operator is unbounded.
- **Intervals.** Under `ModelTime`, an `interval`'s `uint` bounds SHALL
  count seconds and a `timed-interval`'s quantity bounds SHALL convert
  exactly to exact rational seconds, each end open or closed as written;
  under `ModelSteps`, bounds SHALL count positions and a `timed-interval`
  SHALL refuse `ill_typed`/`type-mismatch` at its span. A bound of a
  dimension other than time SHALL refuse `ill_typed`/`type-mismatch` at its
  span, and an interval whose lower bound exceeds its upper bound, or whose
  equal bounds have an open end, SHALL refuse
  `invalid_timed_model`/`invalid-interval` at its span (QSpec FR-255).
  S3 SHALL record each bound as an exact rational. The forms follow the
  profile (QSpec FR-193 "Intervals", FR-370 "Placement and profile fit"):
  under `quire.temporal.timed/v1` every rational bound, either end open or
  closed, `[q, *]`, which a hybrid claim admits under that profile, and an
  operator with no interval;
  under `quire.temporal.infinite-trace/v1` whole-second bounds with both ends
  closed, `[a, *]`, and an operator with no interval; and under
  `quire.temporal.event-position.false-extension/v1`,
  `quire.temporal.fixed-sample.false-extension/v1` and
  `quire.temporal.timestamped-event.finite-window/v1` whole-second closed
  `[a, b]` only. A bound, end, `*` or missing interval the profile does not
  admit SHALL refuse `unsupported_construct`/`expression-form` at the
  interval's span, or at the operator's span for a missing interval, naming
  the profile.
- **Horizon.** S3 SHALL compute the claim's horizon as the reach `r` of its
  formula, the latest time in seconds (positions under `ModelSteps`) after
  the activation position whose state the formula reads:
  `r(Holds(_))` is 0; `r(Not(f))` is `r(f)`; `r(Logic(_, f, g))` is the
  greater of `r(f)` and `r(g)`; `r(Always(i, f))` and `r(Eventually(i, f))`
  are `u + r(f)`; `r(Until(i, f, g))` and `r(Release(i, f, g))` are `u` plus
  the greater of `r(f)` and `r(g)`, where `u` is `i`'s upper bound;
  `r(Historically(i, f))` and `r(Once(i, f))` are the greater of 0 and
  `r(f) - l`; and `r(Since(i, f, g))` and `r(Triggered(i, f, g))` are the
  greater of `r(f)` and of 0 and `r(g) - l`, where `l` is `i`'s lower bound.
  A past operator reads only earlier positions, so it is bounded even with no
  interval or the upper bound `*`, with `l` 0 when it writes no interval. The
  reach is unbounded when a future operator on the way has no interval or
  the upper bound `*`, or any operand's reach is unbounded. The horizon SHALL
  be `None` for an `Each` activation, which creates an obligation at every
  position, and otherwise `Some(Seconds(r))` under `ModelTime` and
  `Some(Positions(r))` under `ModelSteps` when the reach is bounded, and
  `None` when it is not. So `always [0, 10] eventually [0, 5] holds(h.v < 30)`
  has horizon 15 s, and `always [0, 10] once [2, 3] holds(h.v < 30)` 10 s.

### The checked record

- S3 SHALL record each checked hybrid model and each checked hybrid claim on
  the checked package, and the checked package identity (FR-003, FR-259)
  SHALL take them into its preimage: each `CheckedHybridModel`, then each
  `CheckedHybridClaim`, in source order, as one quire-canonical (FR-259) JSON
  object of its members in the order this requirement declares them, each
  enum value an object whose `type` member is the variant name in
  kebab-case and whose other members are its fields. So two packages that
  differ in a mode, a rate, a guard, a reset, a classification or a claim
  have different checked package identities, as do two that differ in an
  initial entry, a reset constraint's relation, or a claim's clock,
  activation, capture or fairness constraint. A `Call` enters the preimage
  as its function's checked declaration identity (FR-062), so a change to
  the called function's body changes the package identity.
- The `CheckedHybridModel` SHALL be the one form the request writer and an
  in-process hybrid provider read, and its emitted nodes (Package emission)
  the one form a process hybrid provider reads; none re-reads source.

### The provider request

- The request writer SHALL write one FR-331 request item for each hybrid
  claim: its kind `temporal-satisfaction` (QSpec FR-290), its claim locus,
  its extent `bounded` when the claim's horizon is `Some` and `unbounded`
  when it is `None`, its `hybrid.limits` by FR-250, and the hybrid-dynamics
  requirement QSpec FR-193 "Routing a hybrid item" names, the item member
  `"requirements": ["hybrid-dynamics"]`, for negotiation (FR-075) to route.
- The request writer SHALL give that requirement to every hybrid claim,
  since every checked hybrid model has a `Hybrid` variable (The clock line),
  and SHALL write no `requirements` member on the item of any other claim.
  The requirement is part of the request identity (QSpec FR-306).
- The item's `claim` SHALL name the claim's emitted `temporal_clause` node
  and its `claim` occurrence (QSpec FR-331 "Items", FR-481 "Hybrid claims"),
  so the item has one locus for every provider.
- Negotiation SHALL route the item to any registered provider that
  advertises the item's kind and whose descriptor `dynamics` holds `hybrid`
  (FR-075), in process or in a process of its own; no rule restricts a
  hybrid item to either. EN-6, whose descriptor holds no `dynamics`, is never
  a candidate for it, so registering a hybrid provider beside EN-6 makes no
  item ambiguous.
- An in-process hybrid provider (QSpec FR-331 "In-process providers";
  FR-280) receives the request's manifest, `deadline_ms` and items as typed
  values, together with the checked package, and reads the claim's
  `CheckedHybridClaim` and its `CheckedHybridModel` from that package. QSL
  has no built-in hybrid solver (ADR-026 HY-3): every hybrid provider, in
  process or not, is a registered backend that publishes its own solver
  budget defaults in its descriptor `bounds` (FR-250).
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
    `class` the text `clock` or `hybrid`, and its `initial` entries as
    FR-481's `initial` binding, each by its `ModeIndex` and its condition's
    term node, the binding omitted when the entries are exactly the default
    `InitialEntry { mode: 0, condition: Boolean(true) }`;
  - each `CheckedMode` as one `hybrid`/`hybrid_mode` node declared by the
    model's name and the mode's name, its `invariant`, its `rates` in
    variable order and its `transitions` in declared order;
  - each `CheckedTransition` as one `hybrid`/`hybrid_transition` node with
    its target `ModeIndex`, its guard and its `ResetConstraint`s in declared
    order, each by its `VariableIndex` and its value's term node, with the
    `relation` `ne`, `lt`, `le`, `ge` or `gt` when its relation is not `Eq`;
  - each `HybridTerm` as a term node: `Variable(i)` a `reference` to
    variable `i`'s node; `Constant(q)` a `value`/`literal` node of `q` typed
    `scalar_type`/`rational`; `Boolean(b)` a `value`/`literal` node of `b`;
    `Negate`, `Arithmetic`, `Compare` (`Ne` included), `Not` and `Logic` an
    `expression` node applying the catalogued `quire.op.rational.*` or
    `quire.op.boolean.*` operation of that operator to the nodes of its
    operands; `Conditional` an `expression`/`conditional` node and `Call` an
    `expression`/`call` node applying the called function's node (FR-322);
    and, in a claim over a model of `n` variables, `ActivationVariable(i)` a
    `reference` to the `value`/`parameter` node named by variable `i` at
    level `n + i` and `Capture(j)` a `reference` to the `value`/`parameter`
    node named by capture `j` at level `2n + j`, typed by its type;
  - each `CheckedHybridClaim` as the `temporal`/`temporal_clause` node of
    its `temporal` clause, whose `over` parameter is typed by its model's
    `hybrid_model` node at level 0, with the `temporal_profile` law the
    claim's `using` profile selects; its clock `model-time` or
    `model-steps`; its activation `origin`, or `each` with the `on each`
    parameter's node at level 1 as `trigger` and its guard's term node; its
    captures, each named by the capture and holding its initializer's term
    node; its fairness as QSpec FR-481 hybrid fairness members, `declaration`
    the model node and `name` the mode for `fair S`, and `declaration` the
    mode node of `S` and `name` the target for `fair S::T`; and its
    `HybridFormula`, future and past operators alike, as QSpec FR-370
    formula nodes, each `HybridInterval` in the one form QSpec FR-370 admits
    under the claim's profile: under `quire.temporal.timed/v1` the timed
    form, `lower` and `upper` exact rational seconds with `lower_end` and
    `upper_end` `open` or `closed` as the interval's ends are, and `upper`
    `null` with `upper_end` `open` for `*`; under every other profile the
    integer form. `Holds(Boolean(true))` and
    `Holds(Boolean(false))` SHALL be the formula nodes of
    `quire.op.temporal.true` and `quire.op.temporal.false`, and every other
    `Holds(t)` the node of `quire.op.temporal.holds` over `t`'s term node,
    so one claim has one encoding and one `package_id`. The horizon is not emitted: a reader derives it from
    the formula by the reach rule of "Hybrid claims" above.
- QSL's I2 reader SHALL admit an emitted package's hybrid nodes and read
  each `hybrid_model` node and its modes and transitions back into the
  `CheckedHybridModel` it was emitted from, and each hybrid claim's
  `temporal_clause` node into its `CheckedHybridClaim`, the horizon derived
  by the reach rule of "Hybrid claims": every restored member included, the
  initial entries, each reset constraint's relation, the `Conditional`,
  `Call`, `ActivationVariable` and `Capture` terms, and each claim's clock,
  activation, captures, fairness, past operators and intervals.
- The I2 reader SHALL refuse a hybrid node or hybrid claim that breaks a
  QSpec FR-481 rule with FR-481's code at FR-481's locus, in FR-481's
  reader order, and a layout outside QSpec's published schema `malformed_wire`
  at strict wire validation.
- The emitted package's `package_id` SHALL take every hybrid node, so two
  packages that differ in a mode, an initial entry, a rate, a guard, a reset
  or its relation, a classification or a claim, its clock, activation,
  captures and fairness included, have different `package_id`s.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-447-AC-1 | The use case's `Braking` builds one `HybridForm` with modes `Moving` and `Stopped` in that order, `Moving` holding three equations and one transition to `Stopped`, each with its span; S3 checks it to a `CheckedHybridModel` whose variables are `v`, `a` and `t` in that order, classified `Hybrid`, `Hybrid` and `Clock`, whose `Moving` rates are `Negate(Variable(1))`, `Constant(0)` and `Constant(1)`, whose `initial` is `[InitialEntry { mode: 0, condition: Boolean(true) }]`, the first mode `Moving` with the condition `true`, and whose transition's reset is `[ResetConstraint { variable: 0, relation: Eq, value: Constant(0) }]`. | Test (TC-1187) |
| FR-447-AC-2 | Each mutation of `Braking` refuses at its span with the cause given: a second mode `Moving`, `conflicting_declaration`/`duplicate-declaration`; `transition to Halted`, `missing_declaration`/`missing-name`; `Stopped` without `t' = 1;`, `invalid_model_binding`/`malformed-declaration` naming `Stopped` and `t`; a second `v' = 0;` in `Moving`, `conflicting_declaration`/`duplicate-declaration`; `P::v' = 0;`, `invalid_model_binding`/`malformed-declaration`; an invariant `{ v }`, `ill_typed`/`non-boolean-root`; a reset `{ v + a = 1 }`, `invalid_model_binding`/`malformed-declaration`; a reset `{ v = 0 and v = 1 }`, `conflicting_declaration`/`duplicate-declaration`; a guard reading `w`, `missing_declaration`/`missing-name`; a guard `when { v + 1 }`, `ill_typed`/`non-boolean-root`; a flow `v' = true;`, `ill_typed`/`type-mismatch`; a flow `v' = f(v)` with no function `f` in scope, `missing_declaration`/`missing-name`; a flow `v' = v div 2`, `ill_typed`/`type-mismatch`; and a declaration whose only equations are `t' = 1;` in each mode, `invalid_timed_model`/`rate-one-hybrid`. | Test (TC-1187) |
| FR-447-AC-3 | `SlowEnough` checks to a `CheckedHybridClaim` over `Braking` with the clock `ModelTime`, the activation `Origin`, no capture and no fairness, the formula `Always(Some([0, 10]), Holds(Compare(Lt, Variable(0), Constant(30))))` and horizon `Seconds(10)`; with the body `always [0, 10] eventually [0, 5] holds(h.v < 30)` the horizon is `Seconds(15)`, with `holds(h.v < 30) until [0, 4] holds(h.v = 0)` it is `Seconds(4)`, and with `always holds(h.v < 30)` or `always [0, *] holds(h.v < 30)` it is `None`. `clock "wall"` is refused `unsupported_construct`/`declaration-form`, and `h.w` is refused `missing_declaration`/`missing-name`. | Test (TC-1187) |
| FR-447-AC-4 | The request writer writes for `SlowEnough` one item of kind `temporal-satisfaction`, extent `bounded`, carrying the hybrid-dynamics requirement, whose `claim` names `SlowEnough`'s emitted `temporal_clause` node and its `claim` occurrence; with an in-process fixture hybrid provider registered the item routes to it, which reads `Braking` from the checked package; with only a process fixture hybrid provider registered the item routes to it, which receives the package's v2 bytes and reads `Braking` and `SlowEnough` from their QSpec FR-481 nodes; with none registered it settles `unsupported`, `unsupported-requested-capability`. Two packages differing only in `Moving`'s rate of `v` have different checked package identities. | Test (TC-1187) |
| FR-447-AC-5 | S4 emits the package holding `Braking`, `SlowEnough` and `MovingFair`, and QSpec's v2 reader admits it: one `hybrid`/`hybrid_model` node declared `[Braking]` whose variables are the parameter nodes of `v`, `a` and `t` at levels 0 to 2 classed `hybrid`, `hybrid` and `clock`, whose modes are `[Braking, Moving]` and `[Braking, Stopped]` in that order, `Moving`'s rates referencing the nodes of `-a`, `0` and `1` and its one transition node `target` 1 with the reset `(0, 0)`; `SlowEnough` is one `temporal_clause` node over a parameter typed by the model node, clock `model-time`; and `MovingFair`, whose body contains `fair Moving`, has one fairness node whose `quire.op.temporal.fair` application has a `fairness` member with `declaration` referencing the `[Braking]` `hybrid_model` node and `name` the text literal `Moving`. Replacing `Moving`'s `v' = -a` by `v' = -(2 * a)` changes the `package_id`. | Test (TC-1187) |
| FR-447-AC-6 | `Braking2` builds a `HybridForm` whose `initial` holds two entries with their spans and checks to `initial` `[InitialEntry { mode: 0, condition: .. }, InitialEntry { mode: 1, condition: .. }]` in source order, so `Moving` and `Stopped` are both initial modes; `Braking` with `initial { mode Moving when { true }; }` checks to the same `CheckedHybridModel` and checked package identity as `Braking`; `initial { mode Halted when { true }; }` refuses `missing_declaration`/`missing-name` and `initial { mode Moving when { v }; }` `ill_typed`/`non-boolean-root`, each at its span; `initial` as a variable name elsewhere in the unit stays an identifier; and `Braking2` and `Braking` have different checked package identities. | Test (TC-1220) |
| FR-447-AC-7 | Each as one mutation of `Braking`'s `Moving`, the flow `v' = -a * v`, the guard `v * v >= 2`, the rate `a' = if v > 1 then 0 else -1`, `v' = let k = a * v in -k`, a guard `v != 3` and a flow `v' = drag(v)` calling `function drag using p(s: Rational[-100, 100; 1, 1]): Rational[-1000, 0; 1, 10] pure { - s * s / 10 }`, S3 checks each to the `HybridTerm` the rules give: `Arithmetic(Mul, Negate(Variable(1)), Variable(0))`, `Compare(Ge, Arithmetic(Mul, Variable(0), Variable(0)), Constant(2))`, a `Conditional`, the substituted `Negate(Arithmetic(Mul, Variable(1), Variable(0)))`, `Compare(Ne, ..)` and `Call(drag, [Variable(0)])`; a call passing `v` to an `Integer` parameter refuses `ill_typed`/`type-mismatch` at the call; `pre(v)` in a guard refuses `invalid_model_binding`/`malformed-declaration`; and changing `drag`'s body changes the checked package identity. | Test (TC-1220) |
| FR-447-AC-8 | The transition's reset `{ v = pre(v) / 2 and a >= 1 and a <= 2 }` checks to `[ResetConstraint { variable: 0, relation: Eq, value: Arithmetic(Div, Variable(0), Constant(2)) }, ResetConstraint { variable: 1, relation: Ge, value: Constant(1) }, ResetConstraint { variable: 1, relation: Le, value: Constant(2) }]`; `{ v = v + 1 }` checks to `[ResetConstraint { variable: 0, relation: Eq, value: Arithmetic(Add, Variable(0), Constant(1)) }]`, read over the pre-jump `v`; `{ let k = 2 in v = k }` checks to `value: Constant(2)`; `{ v = 0 and v >= 0 }` refuses `conflicting_declaration`/`duplicate-declaration` at the second conjunct, naming both; and `{ v = 0 or v = 1 }` refuses `invalid_model_binding`/`malformed-declaration`. | Test (TC-1220) |
| FR-447-AC-9 | `Slows` checks to a `CheckedHybridClaim` with the clock `ModelTime`, the activation `Each { guard: Some(Compare(Gt, ActivationVariable(0), Constant(10))) }`, one capture `v0` of value `ActivationVariable(0)`, the fairness `[HybridFairness { kind: Weak, granularity: Whole, transitions: [(0, 0)] }]`, a body reading `Capture(0)`, and horizon `None`; the same clause with no `clock` member, and with `clock "model-time"`, checks equal; `fair strong each Moving;` resolves to `(Strong, Each, [(0, 0)])`; `fair Moving::Halted;` and `on each (q: Braking)` over `(h: Braking2)` refuse `missing_declaration`/`missing-name` and `ill_typed`/`type-mismatch`; a second capture `v0` refuses `conflicting_declaration`/`duplicate-declaration`; a capture typed `Text[0, 3; nfc]` refuses `ill_typed`/`type-mismatch`; and a `fair` constraint under a bounded profile refuses `unsupported_construct`/`expression-form`. | Test (TC-1220) |
| FR-447-AC-10 | Over `Braking`, `always [0, 10] (holds(h.v = 0) implies once [0, 2] holds(h.v <= 1))` lowers to `Always(.., Logic(Implies, .., Once(Some([0, 2]), ..)))` with horizon `Seconds(10)`; `always [0, 10] once [2, 3] holds(h.v < 30)` has horizon `Seconds(10)`; `holds(h.v > 0) since [0, 3] eventually [0, 4] holds(h.v = 20)` has horizon `Seconds(4)`; `historically holds(h.v <= 20)` and `holds(h.v = 0) triggered holds(h.v <= 20)`, each without an interval, are admitted under the infinite-trace profile with horizon `Seconds(0)`; and with `clock "model-steps"`, `eventually [0, 1] holds(h.v = 0)` has horizon `Positions(1)`. | Test (TC-1220) |
| FR-447-AC-11 | Under `ModelTime` and a unit selecting `quire.temporal.timed/v1`, `eventually [0 s, 1500 ms] holds(h.v < 19)` checks to the interval `{lower: 0, lower_open: false, upper: Some(3/2), upper_open: false}` and horizon `Seconds(3/2)`, `always (0.5 s, 2 s] holds(h.v <= 20)` to `{1/2, true, Some(2), false}`, `eventually [0, 2]` to `{0, false, Some(2), false}`, and `always [1/4 s, *] holds(h.v <= 20)` to `upper: None` with horizon `None`; under the infinite-trace profile `eventually [0 s, 1500 ms]` and `always (0.5 s, 2 s]` each refuse `unsupported_construct`/`expression-form` naming that profile, while `eventually [0 s, 2 s]` and `always [2, *]` check to whole-second intervals; `[2 kg, 3 kg]` refuses `ill_typed`/`type-mismatch`, `(1 s, 1 s]` refuses `invalid_timed_model`/`invalid-interval`, and under `clock "model-steps"` `eventually [0 s, 2 s]` refuses `ill_typed`/`type-mismatch`; a `timed-interval` in a temporal clause over an untimed state model under the infinite-trace profile refuses `unsupported_construct`/`expression-form`. | Test (TC-1220) |
| FR-447-AC-12 | S4 emits the package holding the use case's `Braking2`, `Slows` and `Recent`, and QSpec's v2 reader admits it, its nodes equal to QSpec FR-481's `restored_nodes` up to the source map: the model's `initial` of two entries, mode 0 and mode 1, each condition one Boolean term node of its full conjunction; the resets `(0, 0)` and `(1, 1, ge)`; `Slows` with an `each` activation over the `q` node at level 1, its guard reading `v`'s activation read node at level 3, the capture `v0` at level 6 typed by the `rational_range` node of `Rational[0, 20; 1, 1]`, the `temporal_profile` law `quire.temporal.timed/v1`, a fairness member naming `[Braking2, Moving]` and `Stopped`, and the timed interval `[0, 21/2]`; `Recent` with clock `model-steps` and an integer `once` interval `[0, 2]`; and `Braking`'s model node with no `initial` binding. | Test (TC-1220) |
| FR-447-AC-13 | QSL's I2 reader admits AC-5's emitted package and reads back `Braking`'s `CheckedHybridModel` and `SlowEnough`'s `CheckedHybridClaim` with horizon `Seconds(10)`, each equal to the checked one; it admits QSpec's `positive-all-families.json`, whose `Tank` model has a bare-variable rate and reset and a `hybrid` variable whose rates are all the literal `1`; and, with `dependencies`, keys and `package_id` rebuilt after each edit, it refuses `Moving` with a fourth rate `invalid_model_binding`/`malformed-declaration` at `rates`, every variable classed `clock` `invalid_package`/`invalid-value` at `v`'s class literal, and `SlowEnough`'s clock `wall` `invalid_package`/`invalid-value` at `/body/arguments/1`, each as QSpec FR-481 orders them; a model body with `modes` before `variables` refuses `malformed_wire`. | Test (TC-1187) |
| FR-447-AC-14 | A claim body `always [0, 10] true` emits a `quire.op.temporal.true` formula node under the `always` node and no `quire.op.temporal.holds` node over a Boolean literal; `SlowEnough` emits the `temporal_profile` law its `using` alias selects. | Test (TC-1187) |
| FR-447-AC-15 | `SlowEnough` written `using` a profile alias selecting `quire.temporal.timed/v1` checks with the interval `{lower: 0, lower_open: false, upper: Some(10), upper_open: false}` and horizon `Some(Seconds(10))`, and S4 emits its `always` node with the timed interval `{lower: 0, upper: 10, lower_end: closed, upper_end: closed}` in exact rational seconds and the `timed/v1` law, which QSpec's v2 reader admits; under the infinite-trace profile the same claim emits the integer `[0, 10]`. Under `timed/v1`, `always [0, *] holds(h.v < 30)` is admitted, a hybrid claim admitting `[q, *]` there, with `upper: None`, horizon `None` and the emitted timed interval `upper` `null`, `upper_end` `open`; `always holds(h.v < 30)`, with no interval, is admitted under `timed/v1` with horizon `None`, and under `quire.temporal.event-position.false-extension/v1` it refuses `unsupported_construct`/`expression-form` at the operator's span, naming the profile. | Test (TC-1187) |
| FR-447-AC-16 | QSL's I2 reader admits AC-12's emitted package and reads back `Braking2`'s `CheckedHybridModel`, with its two initial entries and its reset constraints `Eq` and `Ge`, and `Slows`'s and `Recent`'s `CheckedHybridClaim`s, with their clocks, the `Each` activation and its guard, the capture `v0` and its type, the fairness constraint, the `Once` operator and the intervals, each equal to the checked one; and it refuses a written default `initial` `invalid_semantic_graph` at `initial` and a fairness member naming `Halted` `missing_declaration`/`missing-name`, at QSpec FR-481's loci. | Test (TC-1220) |
| FR-447-AC-17 | The request writer writes `SlowEnough`'s item with the member `"requirements": ["hybrid-dynamics"]` beside `"kind": "temporal-satisfaction"`, and writes no `requirements` on the item of a temporal clause over a model without hybrid dynamics in the same unit; with EN-6 and an in-process fixture hybrid provider whose descriptor `dynamics` is `["hybrid"]` both registered, `SlowEnough`'s item routes to the fixture alone and the other item to EN-6 alone, neither settling `ambiguous-backend`; two requests that differ only in `SlowEnough`'s `requirements` have different request identities. | Test (TC-1187) |

## Dependencies

- QSpec FR-193 (the `hybrid-decl` production, its meaning, the clock line and
  the solver contract); QSpec FR-290 (the item kind); QSpec FR-331 (the
  request item, `hybrid.limits`, the package bytes and the in-process form);
  QSpec FR-322 and FR-481 (the emitted nodes and their reader order);
  QSpec FR-272 (the causes).
- ADR-026 HY-1 to HY-3.
- [FR-062](FR-062-implement-checked-family-contract.md) (pure functions),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md),
  [FR-233](FR-233-bind-a-model-claim-to-model-time-or-model-steps.md),
  [FR-450](FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md)
  (trace replay).
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md),
  [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md),
  [FR-250](FR-250-map-a-hybrid-solver-result-to-a-verdict.md),
  [FR-280](FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md).
