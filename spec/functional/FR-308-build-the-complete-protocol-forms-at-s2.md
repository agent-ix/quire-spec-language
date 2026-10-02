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
  - target: ix://agent-ix/quire-specification/FR-052
    type: depends_on
---
# FR-308: Build the complete protocol forms at S2

## Description

When S2 builds the form of a protocol declaration under
`quire.model.complete/v1`, it SHALL accept every surface form of QSpec's
shared grammar "Complete temporal, choreography and analysis forms" that
the protocol system runs, and SHALL build one typed form for each, holding
every member the production writes with its span (ADR-012 §1, §3;
ADR-027 DS-1). S2 records what was written and resolves nothing; S3 checks
it (FR-218).

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
  #173's complete forms; QSpec FR-052 for `repetition`).

## Outputs

In `forms::protocol_clause` (crate `qsl-forms`):

- `ProtocolClauseForm` with its activation (`OnOrigin` or `OnEach{binder,
  guard}`), roles, channels, compensations, an ordered list of
  `terminal` and `scheduling` members, the `run` control and `finish`.
- `RoleForm::{Static{name, model}, Replicated{name, model, population, max,
  lifetime: Workflow | Scope | Until(expr)}}`.
- `ChannelForm{name, from, to, carries, ordering, delivery, capacity:
  Literal(u64) | Symbolic(ident), overflow}`.
- `ParallelForm{name, branches, join: All | Any | Quorum(u64) |
  Predicate(ident), outstanding: Option<Continue | Cancel>}`, and the
  memory part of FR-309.
- `RepeatForm{name, binder, visibility, max: Option<u64>, loop_proof:
  Option<{invariant, variant}>, guard, body, exhausted: Option<control>}`.
- `AwaitForm`, `CompensationForm`, `TerminalForm::{When(block), Any}` and
  `SchedulingForm::Adversarial`, each with its spans.

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
- A well-formed CST adds no S2 refusal. A CST that carries an error or
  recovery node SHALL refuse as `FormsCause::RecoveringCst`, with no
  partial form.
- Under a profile that does not select `quire.model.complete/v1`, S2 SHALL
  build no complete form, and S3 refuses each such construct
  `unsupported_construct` (QSpec shared grammar).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-308-AC-1 | A protocol with `role w each M::Worker from workers max 2 lifetime until({w.done})`, `channel ch from a to b carries M::Ping ordering unordered delivery at-least-once capacity 1 overflow block;` and `activation on each (x: M::Tick) when ({true})` builds a `Replicated` role with population `workers`, max 2 and an `Until` lifetime, a `ChannelForm` with `AtLeastOnce`, `Literal(1)` and `Block`, and an `OnEach` activation with binder `x`; each member's span covers its text. | Test (TC-887) |
| FR-308-AC-2 | `parallel P { branch a …; branch b …; } join quorum(2) [a, b] outstanding cancel;` builds `join: Quorum(2)` and `outstanding: Some(Cancel)`; the same with `join all` and no `outstanding` builds `All` and `None`. | Test (TC-887) |
| FR-308-AC-3 | `repeat R by r visible() invariant {…} variant {…} while {…} …` builds a `RepeatForm` with no maximum, both loop-proof blocks and no `exhausted` control; `repeat R by r visible() max 3 while {…} … exhausted …` builds maximum 3, no loop proof and an `exhausted` control. | Test (TC-887) |
| FR-308-AC-4 | A protocol with `terminal when {k.v = 3};`, then `terminal any;`, then `scheduling adversarial;` builds three members in that order with their spans; a protocol that names an identifier `fork` outside a step-row position builds it as an identifier. | Test (TC-887) |
| FR-308-AC-5 | A protocol whose CST holds a recovery node refuses `FormsCause::RecoveringCst` with no form. Building the same declaration twice gives equal forms. | Test (TC-887) |

## Dependencies

- ADR-012 §1, §3 (forms); ADR-027 §9 DS-1.
- [FR-112](FR-112-build-protocol-scoped-anchor-forms.md) (scoped anchors in
  the same forms), FR-218 (the S3 checks), FR-309 (the memory forms).
- QSpec owns the grammar: the shared grammar's complete choreography
  productions and QSpec FR-052's `repetition`.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-052 and the shared
  grammar's complete forms (Linear STD-140).
