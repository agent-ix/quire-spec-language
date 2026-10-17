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
---
# FR-308: Build the complete protocol forms at S2

## Description

When S2 builds the form of a protocol declaration, it SHALL build one typed
form for each `capture`, `relationship`, `requires temporal`, `related by`
clause and `compensate` template the protocol writes, holding every member
the production writes with its span (ADR-012 §1, §3; ADR-027 DS-1). S2
records what was written and resolves nothing; S3 checks it (FR-218,
FR-381).

## Use case

A verification operator writes a protocol with a capture, a relationship, a
required temporal clause, an `effect` node related to another by a
relationship and a compensation template. Each member reaches the checker
as a typed form with its parts and spans, so every later refusal points at
the text the operator wrote.

## Inputs

- The CST of one protocol declaration, parsed by QSpec's shared-grammar
  productions `protocol-clause`, `capture`, `relationship`,
  `protocol-requirement`, `related`, `event-node`, `node-ref` and
  `compensation`.

## Outputs

In `forms` (crate `qsl-forms`), on `ProtocolDeclarationForm`:

- `roles: Vec<RoleForm{name, context}>`, the declared role name and its
  `M::T` object type, as spelled.
- `captures: Vec<CaptureForm{parameter, value, span}>` (FR-325's form), one
  per `capture` the clause writes before its roles.
- `relationships: Vec<RelationshipForm{name, target: NameForm, span}>`, one
  per `relationship` member.
- `requirements: Vec<RequirementForm{temporal: DeclaredName, span}>`, one per
  `requires temporal` member.
- `compensations: Vec<CompensationForm>`, one per `compensate` template.
- `declarations: Vec<ProtocolNodeDeclaration>`: each static node with its
  kind, name, scope and channel, and `related: Vec<RelatedForm{relationship:
  DeclaredName, ends: [ExpressionForm; 2], span}>`, one per `related by`
  clause of a `send`, `receive`, `attempt`, `effect` or `event` node (empty
  for every other kind).
- `constructs`: every capture, `activation on each`, replicated role,
  relationship, channel, requirement and `related by` clause, by kind and
  span.

`CompensationForm{name, target: NameForm, forward: ParameterForm, role:
DeclaredName, operation: NameForm, using: UsingAlias, clock: ClockForm,
registration_captures: Vec<CaptureForm>, trigger: ParameterForm, guard:
ExpressionForm, activation_captures: Vec<CaptureForm>, within: IntervalForm,
attempts: CompensationAttempts{count: u64, count_span, record: TypeForm},
retry: CompensationRetry{earlier: ParameterForm, later: ParameterForm,
relation: ExpressionForm}, commit: CompensationCommit, recover:
CompensationRecover{binder: ParameterForm, predicate: ExpressionForm},
span}`.

- `target`, `operation` and a `Node` commit are `NameForm`s: the name as
  spelled, with its span.
- `using` is the `UsingAlias` of the alias written.
- `attempts.record` is `TypeForm::name` of the written `M::T`, so its head is
  a name.
- `guard`, `relation` and `predicate` are the `ExpressionForm` of the body
  expression; the braces are not in its span.
- `commit` is `CompensationCommit::Node(NameForm)` or
  `CompensationCommit::Never(Span)`, the span being that of `never`.
- `ExprNode::ProtocolReference{target}` (the span is the expression's own
  span in the S2 span tree) is the form of a protocol reference expression
  (Protocol reference expression).

## Behavior

- S2 SHALL build one form per production instance in the CST, in source
  order, with each member's text and span.
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
  never` and as `Node` for a node reference, and its `attempts` count and
  `within` interval as written, with no S2 diagnostic for a zero count or an
  inverted interval. A count that is not a decimal `u64` fails at S1 with an
  `invalid_syntax` diagnostic at the count (FR-325).
- A well-formed CST adds no S2 refusal. A CST that carries an error or
  recovery node SHALL refuse as `FormsCause::RecoveringCst`, with no
  partial form.
- S2 builds each form whether or not the unit selects
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
| FR-308-AC-5 | A protocol whose CST holds a recovery node refuses `FormsCause::RecoveringCst` with no form. Building the same declaration twice gives equal forms. | Test (TC-887) |
| FR-308-AC-6 | `protocol Fill` written as an expression builds one `ExprNode::ProtocolReference` with target `Fill` and the span of the two tokens; `protocol M::Fill` builds the target `M::Fill`; two references in one unit build two forms in source order. A unit that holds a `protocol-clause` declaration and a reference builds both and the declaration parses as before. | Test (TC-1021) |
| FR-308-AC-7 | A protocol that writes `capture order: M::OrderRef = opened.order;` before its roles, and a compensation that writes `capture target: M::Total = forward.total;` before `activate` and `capture activated: Boolean = trigger.ready;` in its activation block, builds one `CaptureForm` in `ProtocolClauseForm.captures`, one in the compensation's `registration_captures` and one in its `activation_captures`, each with the written name, type, initializer text and its own span. Two registration captures keep their source order. | Test (TC-1055) |
| FR-308-AC-8 | `relationship Pipe = M::pipe;` builds a `RelationshipForm` with name `Pipe` and target `M::pipe`, each span slicing to its text; two relationship members build two forms in source order. | Test (TC-1055) |
| FR-308-AC-9 | `requires temporal Due;`, `requires temporal Late;` and a second `requires temporal Due;` build three `RequirementForm`s in source order with the written names and no resolution. | Test (TC-1055) |
| FR-308-AC-10 | An `effect` node with `related by Pays (applied.order, opened.order)` builds one `RelatedForm` with relationship `Pays` and both ends as full expression forms in written order; a node with two `related by` clauses, one naming `Pays` twice, builds three forms in source order. | Test (TC-1055) |
| FR-308-AC-11 | A compensation builds every member of its `CompensationForm`: `commit Main::Committed` builds `Node` and `commit never` builds `Never`; `attempts 0` builds count 0 and `within [31,30]` builds the interval as written, each with no S2 diagnostic; `attempts 18446744073709551616` fails at S1 with a parse diagnostic at the count. | Test (TC-1055) |
| FR-308-AC-12 | A protocol whose CST holds a recovery node inside a `capture`, a `relationship`, a `requires temporal`, a `related by` or a compensation refuses `FormsCause::RecoveringCst` for each, with no partial form. A protocol that writes all five builds equal forms when built twice. | Test (TC-1055) |

## Dependencies

- ADR-012 §1, §3 (forms); ADR-027 §9 DS-1.
- [FR-112](FR-112-build-protocol-scoped-anchor-forms.md) (scoped anchors in
  the same forms), FR-218 and FR-381 (the S3 checks), FR-325 (`CaptureForm`, `IntervalForm` and the `u64` bound rule).
- QSpec owns the grammar: the shared grammar's `capture`, `relationship`,
  `protocol-requirement`, `related` and `compensation` productions.

## References

- Owning ticket: Linear QSL-396. QSpec half: the shared
  grammar's complete forms (Linear STD-140).
