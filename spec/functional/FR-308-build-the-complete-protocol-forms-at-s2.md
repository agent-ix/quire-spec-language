---
id: FR-308
title: "Build the complete protocol forms at S2"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-112
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-325
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-052
    type: depends_on
---
# FR-308: Build the complete protocol forms at S2

## Description

When S2 builds the form of a protocol declaration, it SHALL accept every surface form of QSpec's
shared grammar "Complete temporal, choreography and analysis forms" that
the protocol system runs, and SHALL build one typed form for each, holding
every member the production writes with its span (ADR-012 §1, §3;
ADR-027 DS-1). That includes the protocol's captures, relationship
declarations, `requires temporal` requirements, each event node's `related by`
clauses and every member of a compensation template. S2 records what was
written and resolves nothing; S3 checks it (FR-218, FR-381).

## Use case

A verification operator writes a protocol with replicated roles, a bounded
channel, a `parallel` joined by quorum, a `repeat` with an invariant and
variant, a `terminal when` member and `scheduling adversarial`. Each
construct reaches the checker as a typed form with its parts and spans, so
every later refusal points at the text the operator wrote.

## Inputs

- The CST of one protocol declaration, parsed by QSpec's shared-grammar
  productions `protocol-clause`, `activation`, `role`, `role-lifetime`,
  `channel`, `delivery-policy`, `capacity`, `overflow-policy`, `parallel`,
  `join-policy`, `repetition`, `loop-proof`, `await-control`,
  `compensation`, `protocol-terminal` and `protocol-scheduling` (QSpec
  #173's complete forms; QSpec FR-052 for `repetition`), and the same
  grammar's `capture`, `relationship`, `protocol-requirement`, `related`,
  `event-node` and `node-ref`.

## Outputs

In crate `qsl-forms`, the protocol form is `ProtocolDeclarationForm`. The
fields below name its members, written `ProtocolClauseForm` where they are
the target model. Today it builds `roles` (`RoleForm{name, context}`),
`activation` (`ActivationForm::{Origin, Each{parameter, when}}`), `captures`,
`relationships`, `requirements`, `compensations`, and `related` on each node
declaration; the control constructs, channels and replicated roles below are
the target for their owning slices:

- `ProtocolClauseForm` with its activation (`OnOrigin` or `OnEach{binder,
  guard}`), roles, channels, compensations, an ordered list of
  `terminal` and `scheduling` members, the `run` control and `finish`.
- `RoleForm::{Static{name, model}, Replicated{name, model, population, max,
  lifetime: Workflow | Scope | Until(expr)}}`.
- `ChannelForm{name, from, to, carries, ordering, delivery, capacity:
  Literal(u64) | Symbolic(ident), overflow: Reject | Block | Loss(Head |
  Tail | New)}`.
- `ParallelForm{name, branches, join: All | Any | Quorum(u64) |
  Predicate(ident), outstanding: Option<Continue | Cancel>}`, and the
  memory part of FR-309.
- `RepeatForm{name, binder, visibility, max: Option<u64>, loop_proof:
  Option<{invariant, variant}>, guard, body, exhausted: Option<control>}`.
- `AwaitForm`, `TerminalForm::{When(block), Any}` and
  `SchedulingForm::Adversarial`, each with its spans.
- `ProtocolClauseForm.captures`: one `CaptureForm{parameter, value, span}`
  (FR-325's form) per `capture` the clause writes before its roles.
- `ProtocolClauseForm.relationships`: one `RelationshipForm{name, target:
  QualifiedName, span}` per `relationship` member.
- `ProtocolClauseForm.requirements`: one `RequirementForm{temporal: Name,
  span}` per `requires temporal` member.
- On each `send`, `receive`, `attempt`, `effect` and `event` node form,
  `related: Vec<RelatedForm{relationship: DeclaredName, ends:
  [ExpressionForm; 2], span}>`, one per `related by` clause. The node form
  is `ProtocolNodeDeclaration`, which carries `related`.
- `CompensationForm{name, target: NameForm, forward: ParameterForm, role:
  DeclaredName, operation: NameForm, using: UsingAlias, clock: ClockForm,
  registration_captures: Vec<CaptureForm>, trigger: ParameterForm, guard:
  ExpressionForm, activation_captures: Vec<CaptureForm>, within:
  IntervalForm, attempts: CompensationAttempts{count: u64, count_span,
  record: TypeForm}, retry: CompensationRetry{earlier: ParameterForm, later:
  ParameterForm, relation: ExpressionForm}, commit:
  CompensationCommit::Node(NameForm) | CompensationCommit::Never(Span),
  recover: CompensationRecover{binder: ParameterForm, predicate:
  ExpressionForm}, span}`. A `NameForm` is the name as spelled with its
  span. `guard`, `relation` and `predicate` are the `ExpressionForm` of the
  body expression, with the braces outside its span. `attempts.record` is
  `TypeForm::name` of the written `M::T`. `Never` carries the span of
  `never`.
- `ExprNode::ProtocolReference{target}` (the span is the expression's own span in the S2 span tree), the form
  of a protocol reference expression (Protocol reference expression).

## Behavior

- S2 SHALL build one form per production instance in the CST, in source
  order, with each member's text and span.
- S2 SHALL keep every `terminal` and `scheduling` member it finds, in
  source order, so that S3 can refuse a second one at its span (FR-211,
  FR-212).
- S2 SHALL record `repeat`'s maximum, invariant and variant exactly as
  written, each optional, and an `exhausted` control only when a maximum is
  written.
- S2 SHALL read `fork`, `join`, `scheduling`, `terminal`, `adversarial`
  and the other complete-grammar keywords only in the positions the shared
  grammar reserves them for; elsewhere they are identifiers.
- S2 SHALL build each `CaptureForm` of the clause, of a compensation's
  registration stage (written before `activate`) and of its activation stage
  (written in the block after `when`), in source order, into the field of its
  own position. It keeps the declared type and the initializer as written and
  decides no capture's visibility or stage (FR-381).
- S2 SHALL build one `RelationshipForm` per `relationship`, one
  `RequirementForm` per `requires temporal` and one `RelatedForm` per
  `related by`, each in source order, keeping every name and both ends as
  written. A target or relationship named twice builds twice; S2 merges and
  resolves nothing.
- S2 SHALL build a compensation's `commit` member as `Never` for `commit
  never` and as `Node` for a node reference, and its `attempts`
  count and `within` interval as written, with no S2 diagnostic for a zero
  count or an inverted interval. A count that is not a decimal `u64` fails at
  S1 with an `invalid_syntax` diagnostic at the count (FR-325).
- S2 SHALL build a channel's overflow policy as written: `overflow reject`
  as `Reject`, `overflow block` as `Block` and `overflow loss head`,
  `overflow loss tail` and `overflow loss new` as `Loss(Head)`, `Loss(Tail)`
  and `Loss(New)`. The selector token follows `loss` and no other overflow
  policy takes one, so `overflow loss` with no selector, and `overflow block
  head`, are not in the grammar and fail at the parser with its existing
  syntax refusal, before any form is built.
- A well-formed CST adds no S2 refusal. A CST that carries an error or
  recovery node SHALL refuse as `FormsCause::RecoveringCst`, with no
  partial form.
- S2 builds each complete form whether or not the unit selects
  `quire.protocol.complete/v1`. QSpec's facet rule (an unselected facet gives
  `unsupported_construct`) applies to it, but QSL has no header layer for that
  profile, so the gate is not enforced (FR-361 item 8).

### Protocol reference expression

- A protocol reference is the `primary` alternative
  `protocol-reference = 'protocol', qualified-name`. The target is a
  qualified name that S3 resolves (FR-218); the expression carries no
  package selector, alias or key. `protocol` leads a reference only in
  expression position, where no declaration can begin, so it never changes
  how a `protocol-clause` declaration parses.
- A protocol reference is admitted wherever the grammar admits a `primary`.
  Its checked type is the target's semantic type (today the Boolean type
  node the protocol clause carries; FR-218). Its value is the value of the
  target protocol clause: a pure reference that denotes the clause it names
  and is evaluated exactly as that clause is.
- S2 SHALL build one `ExprNode::ProtocolReference` per occurrence in
  source order, with the written target and the span of the whole
  expression, and SHALL resolve nothing. A target that is not a
  `qualified-name` is a parse refusal, not an S2 form.
- QSpec owns the shared grammar. The `protocol-reference` production and
  its meaning are QSpec FR-463's.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-308-AC-1 | A protocol with `role w each M::Worker from workers max 2 lifetime until(w.done)`, `channel ch from a to b carries M::Ping ordering unordered delivery at-least-once capacity 1 overflow block;` and `activation on each (x: M::Tick) when (true)` builds a `Replicated` role with population `workers`, max 2 and an `Until` lifetime, a `ChannelForm` with `AtLeastOnce`, `Literal(1)` and `Block`, and an `OnEach` activation with binder `x`; each member's span covers its text. | Test (TC-887) |
| FR-308-AC-2 | `parallel P { branch a …; branch b …; } join quorum(2) [a, b] outstanding cancel;` builds `join: Quorum(2)` and `outstanding: Some(Cancel)`; the same with `join all` and no `outstanding` builds `All` and `None`. | Test (TC-887) |
| FR-308-AC-3 | `repeat R by r visible() invariant {…} variant {…} while {…} …` builds a `RepeatForm` with no maximum, both loop-proof blocks and no `exhausted` control; `repeat R by r visible() max 3 while {…} … exhausted …` builds maximum 3, no loop proof and an `exhausted` control. | Test (TC-887) |
| FR-308-AC-4 | A protocol with `terminal when {k.v = 3};`, then `terminal any;`, then `scheduling adversarial;` builds three members in that order with their spans; a protocol that names an identifier `fork` outside a step-row position builds it as an identifier. | Test (TC-887) |
| FR-308-AC-5 | A protocol whose CST holds a recovery node refuses `FormsCause::RecoveringCst` with no form. Building the same declaration twice gives equal forms. | Test (TC-887) |
| FR-308-AC-6 | `protocol Fill` written as an expression builds one `ExprNode::ProtocolReference` with target `Fill` and the span of the two tokens; `protocol M::Fill` builds the target `M::Fill`; two references in one unit build two forms in source order. A unit that holds a `protocol-clause` declaration and a reference builds both and the declaration parses as before. | Test (TC-1021) |
| FR-308-AC-7 | A protocol that writes `capture order: M::OrderRef = opened.order;` before its roles, and a compensation that writes `capture target: M::Total = forward.total;` before `activate` and `capture activated: Boolean = trigger.ready;` in its activation block, builds one `CaptureForm` in `ProtocolClauseForm.captures`, one in the compensation's `registration_captures` and one in its `activation_captures`, each with the written name, type, initializer text and its own span. Two registration captures keep their source order. | Test (TC-1055) |
| FR-308-AC-8 | `relationship Pipe = M::pipe;` builds a `RelationshipForm` with name `Pipe` and target `M::pipe`, each span slicing to its text; two relationship members build two forms in source order. | Test (TC-1055) |
| FR-308-AC-9 | `requires temporal Due;`, `requires temporal Late;` and a second `requires temporal Due;` build three `RequirementForm`s in source order with the written names and no resolution. | Test (TC-1055) |
| FR-308-AC-10 | An `effect` node with `related by Pays (applied.order, opened.order)` builds one `RelatedForm` with relationship `Pays` and both ends as full expression forms in written order; a node with two `related by` clauses, one naming `Pays` twice, builds three forms in source order. | Test (TC-1055) |
| FR-308-AC-11 | A compensation builds every member of its `CompensationForm`: `commit Main::Committed` builds `Node` and `commit never` builds `Never`; `attempts 0` builds count 0 and `within [31,30]` builds the interval as written, each with no S2 diagnostic; `attempts 18446744073709551616` fails at S1 with a parse diagnostic at the count. | Test (TC-1055) |
| FR-308-AC-12 | A protocol whose CST holds a recovery node inside a `capture`, a `relationship`, a `requires temporal`, a `related by` or a compensation refuses `FormsCause::RecoveringCst` for each, with no partial form. A protocol that writes all five builds equal forms when built twice. | Test (TC-1055) |
| FR-308-AC-13 | Channels written `overflow reject`, `overflow block`, `overflow loss head`, `overflow loss tail` and `overflow loss new` build `Reject`, `Block`, `Loss(Head)`, `Loss(Tail)` and `Loss(New)`, each span slicing to its text; `overflow loss` with no selector and `overflow block head` fail at the parser with a syntax diagnostic and build no form. | Test (TC-1065) |

## Dependencies

- ADR-012 §1, §3 (forms); ADR-027 §9 DS-1.
- [FR-112](FR-112-build-protocol-scoped-anchor-forms.md) (scoped anchors in
  the same forms), FR-218 and FR-381 (the S3 checks), FR-309 (the memory
  forms), FR-325 (`CaptureForm`, `IntervalForm` and the `u64` bound rule).
- QSpec owns the grammar: the shared grammar's complete choreography
  productions and QSpec FR-052's `repetition`.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-052 and the shared
  grammar's complete forms (Linear STD-140).
