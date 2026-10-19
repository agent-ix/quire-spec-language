---
id: FR-381
title: "Check protocol captures, activation and compensation recovery at S3"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-308
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-326
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-055
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-057
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-058
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-172
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-173
    type: depends_on
---
# FR-381: Check protocol captures, activation and compensation recovery at S3

## Description

When S3 checks a protocol declaration, the `ProtocolClause` family SHALL check
the protocol's `activation`, every `capture` it writes, and every
`compensate` template with its registration, activation, attempt bound,
retry relation, commit boundary and recovery predicate, and SHALL record each
in the `CheckedProtocolClause` (FR-218). A capture's initializer, a guard, a
retry relation and a recovery predicate are checked through the one clause
checker (FR-104), over the names their stage may read and no others.

## Use case

A verification operator writes a refund template that registers when a payment
effect occurs, captures the refundable total, activates on the first failed
shipment, retries a bounded number of times, stops at a commit and recovers
when the refunded amounts equal the captured total. They need a refund guard
that reads the payment record, a retry relation that reads a capture before it
exists, or a capture that reads itself, refused at the name they wrote, so
that the protocol system only ever runs an obligation whose every input is
fixed before it starts.

## Inputs

- The S2 forms of a protocol declaration (FR-308): its `activation`, its
  `captures`, and its `CompensationForm`s with their stage fields.
- The scoped anchors resolved (FR-113), the attempts bound (FR-114) and the
  control tree and causal edge relation FR-218 compiles.

## Outputs

On the `CheckedProtocolClause` (FR-218):

- `activation: Origin | OnEach{binder, record_type, guard}`.
- `captures`: each capture as `CheckedCapture{name, declared_type, stage,
  initializer, owner}`, in source order within its stage, with `stage`
  `Protocol`, `Registration` or `Activation` and `owner` the clause or the
  template.
- `compensations`: each template as `CheckedCompensation{name, effect, role,
  operation, forward, registration_captures, trigger, guard,
  activation_captures, within, maximum_attempts, attempt_type, retry, commit:
  Never | Node(id), recover}`, in source order, where `effect` is the id of
  the one forward `effect` node, `operation` the identity of the operation's
  anchor node and frame node (FR-114), `maximum_attempts` a `u64` of at least
  one, and `within` a closed interval `[lower, upper]`.
- A typed `CheckRefusal` with a span on refusal, and no checked clause. Every
  refusal holds the S3 stage, the `ProtocolClause` family, the code and cause
  named at it, the located span named at it and the members named at it, so a
  consumer reads each as a field and never from a message.

## Behavior

### Names a stage may read

Each stage below is checked through the clause checker over the names the row
lists, constants and pure functions. A name a stage may not read is refused as
FR-113 refuses an unresolved name: `missing_declaration`/`missing-name` at the
read, retaining `name`, `stage` (the identifier in the table's first column),
`owner` and `read`, where `read` is `forward` for a capture of the same stage
written later than the read or the capture itself, `foreign` for a name that a
template declares, read outside that template (by another template or by a
protocol-level stage), `stage` for any other name that exists but that the
reading stage may not read, and `unbound` for a name nothing declares.

| Stage | Names it may read |
| --- | --- |
| `protocol-activation` (`activation on each ... when`) | the activation binder |
| `protocol-capture` | the `over` input, the activation binder, protocol captures written earlier |
| `registration-capture` | the forward binder, the `over` input, the activation binder, protocol captures, registration captures of the template written earlier |
| `trigger-guard` (`activate first ... when`) | the trigger binder, protocol captures, the template's registration captures |
| `activation-capture` | the trigger binder, the `over` input, protocol captures, the template's registration captures, activation captures of the template written earlier |
| `retry` | the `earlier` and `later` binders, protocol captures, the template's registration and activation captures |
| `recovery` | the recovery binder, protocol captures, the template's registration and activation captures |

- Every node body in the `run` tree and in `finish` may read the protocol's
  captures.
- The protocol's `over` input is model state (FR-218). The protocol
  activation guard, the trigger guard, the retry relation and the recovery
  predicate read none: S3 SHALL refuse a read of the `over` input by any of
  them with `ill_typed`/`operator-ineligible` at the read, naming the stage.
  This rule has precedence over the table: the table's `missing-name` refusal
  applies to binders and captures only. Protocol captures and registration and
  activation captures may read the `over` input.
- The protocol activation guard, the trigger guard, the retry relation and the
  recovery predicate bar every read of model state, as FR-218 defines it: a
  field read of a binder whose type is an object type, a field read through a
  reference-typed value and an explicit dereference are such reads. S3 SHALL
  refuse one in any of them with `ill_typed`/`operator-ineligible` at the read,
  naming the stage, as it refuses a read of the `over` input. The binder of
  each of these stages is read through its fields only when its type is a
  record type or an event type. A binder read as a whole value, and any
  model-state read in a registration or activation capture, are not refused by
  this rule.
- Evaluation order is source order, so a capture is available to the captures,
  guards and relations written after it and to none written before it. A
  capture's initializer that reads the capture itself or a later capture is
  refused as a `forward` read, so a cycle among captures cannot be written. A
  name that another template declares is a `foreign` read.

### Captures

- S3 SHALL resolve a capture's declared type as FR-104 resolves a declared
  type, and SHALL refuse an unresolved name with `missing_declaration`/
  `missing-name` at the type name, retaining `capture` and `name`.
- S3 SHALL check the initializer through the clause checker, and SHALL refuse
  an initializer whose type is not the declared type with `ill_typed`/
  `type-mismatch` at the initializer, retaining `capture`, `expected` and
  `actual`. An initializer is total: S3 SHALL refuse one with an undischarged
  definedness obligation with the `undefined_expression` cause FR-104 gives it
  (`unproved-range`, `unproved-nonzero`, `unproved-presence`) at the operation.
- A capture's name is unique across the protocol and a capture never shadows a
  binder (FR-113); a repeated name refuses as FR-113 states.

### Activation

- `on origin` binds nothing and has no guard. S3 SHALL record `Origin`.
- For `on each (x: T) when (g)` S3 SHALL resolve `T` as FR-104 resolves a
  declared type, refusing an unresolved name with `missing_declaration`/
  `missing-name` at the type name, SHALL check `g` as a Boolean through the
  clause checker, refusing a non-Boolean with `ill_typed`/`non-boolean-root` at
  the guard, and SHALL record `OnEach{binder, record_type, guard}`. An absent
  guard is the guard that is always true, and S3 records it as absent: a
  protocol that writes `when (true)` and one that writes no guard are
  different checked clauses (FR-383).

### Compensation registration

A `compensate N for E as (forward: T) by R on M::X::op using A clock "c" {...}`
template registers one obligation for each occurrence of the forward `effect`
node `E` (QSpec FR-056).

- S3 SHALL check that `E` names an `effect` node and that `commit` names a
  `commit` node as FR-113's site table states, and SHALL refuse a template whose
  `for` anchor names no node or another kind as FR-113 refuses.
- S3 SHALL refuse a forward binder `T` that is not the record type of the effect
  `E` names, by type identity, with `ill_typed`/`type-mismatch` at `T`,
  retaining `compensation`, `expected` (the effect's record type) and `actual`.
- S3 SHALL refuse a `by` role that the protocol does not declare with
  `missing_declaration`/`missing-name` at the role name, retaining
  `compensation` and `name`.
- S3 SHALL resolve `M::X::op` as FR-114 resolves an attempt's operation and
  SHALL bind the template to the operation's one anchor node and one frame node,
  the nodes every other attempt and template naming the operation share. It
  adds no second anchor, no second frame and no second frame record.
- Two templates may name the same forward effect; each registers its own
  obligation. Two templates of one name refuse `ambiguous_declaration`/
  `ambiguous-name` at both, as FR-113 states.
- The template's registration captures are checked in source order at the
  registration stage, and its activation captures at the activation stage.

### Activation, attempts and deadline

- S3 SHALL resolve the trigger type of `activate first (t: U)` as FR-104
  resolves a declared type, check the guard as a Boolean over the names its
  stage may read, and check the activation captures in source order.
- `attempts n of T`: `n` is the maximum number of attempts, counting the
  initial attempt (QSpec FR-058). S3 SHALL refuse an `n` of zero with
  `ill_typed`/`type-mismatch` at the count, retaining `compensation`, `member`
  (`attempts`), `expected` (a count of at least one) and `actual` (`0`). S3 SHALL admit every
  `n` from one to the greatest `u64` and SHALL record it exactly.
- S3 SHALL resolve `T` as FR-104 resolves a declared type, and SHALL refuse a
  `T` that is not a record type or an event type with `invalid_model_binding`/
  `malformed-declaration` at `T`, retaining `compensation`, `member`
  (`attempts`) and `name`, as FR-218 refuses a channel's `carries` type that is
  not admitted. The attempt record is a record, so an object type, a scalar
  type, a composite type and a variant type are refused. S3 SHALL NOT compare
  the `retry` binders with a refused `T`.
- S3 SHALL refuse a `retry` binder whose declared type is not an admitted `T`
  with `ill_typed`/`type-mismatch` at the binder's type, retaining
  `compensation`, `binder`, `expected` (`T`) and `actual`. The `earlier` binder
  is checked before the `later` binder.
- `within [a,b]` is a closed interval. S3 SHALL refuse an interval with `a > b`,
  as FR-326 refuses it in a temporal clause, and an open upper bound, with
  `ill_typed`/`type-mismatch` at the interval, retaining `compensation`,
  `member` (`within`), `expected` (a closed interval with `a <= b`) and `actual`
  (the interval as written).

### Retry, commit and recovery

- S3 SHALL check the `retry` relation as a Boolean through the clause checker
  over the names its stage may read, refusing a non-Boolean with `ill_typed`/
  `non-boolean-root` at the relation. The relation is total (QSpec FR-058): S3
  SHALL refuse an operation with an undischarged definedness obligation with the
  `undefined_expression` cause FR-104 gives, at the operation. The relation
  constrains a retry and never forces one.
- A `commit` member is either `commit never` or `commit N` naming a `commit`
  node. S3 SHALL record `Never` for the first, which selects no earlier cutoff,
  and `Node(id)` for the second (QSpec FR-057). S3 SHALL refuse a `commit` node
  that dominates the forward effect. S3 decides dominance on the control tree
  only, which is conservative and sound: a commit dominates an effect when a
  `sequence` holds, in order, a child that passes the commit and a later child
  that contains the effect. A control passes a commit when it is the commit;
  when it is a `sequence` with a child that passes it; or when it is a
  `parallel` whose join requires a branch that passes it. A join requires
  branch `L` when every set of completed branches that satisfies its policy
  contains `L`: a `join all` requires every branch it lists, which is every
  branch of the `parallel`; a `quorum(n)` whose `n` equals the number of
  distinct labels it joins requires each of those labels, and a `join any`
  that lists one label requires that label. A `join any` or `quorum(n)` with a
  smaller `n` over several labels, and every `join predicate`, require no
  branch, because its settle condition is not evaluated statically; a branch
  the join does not list is never required. A `choice` passes a commit in no
  case, and a `repeat` passes none, because S3 treats every `repeat` as
  skippable whatever its guard, a constant-true guard included. The causal
  edge relation does not decide dominance: a commit and an effect in sibling
  branches of one `parallel` have no common `sequence` order, so the commit
  does not dominate the effect even when a `send` and a `receive` order them
  across the branches. Deciding dominance through such cross-branch causal
  edges is a later extension, and until then S3 admits that commit. The refusal is
  `unsupported_construct`/`declaration-form` at the commit anchor, as QSpec
  FR-052 refuses a structurally prohibited form, retaining the selected
  protocol profile that prohibits it, `compensation`, `member` (`commit`),
  `commit`, `effect` and `reason` (`commit-precedes-effect`). A commit that
  follows the effect, a commit in a parallel branch whose sibling branch holds
  the effect, or a commit that lies in one `case` of a `choice` before the
  effect, is admitted.
- S3 SHALL resolve the recovery binder's type as FR-104 resolves a declared
  type, and SHALL check the recovery predicate as a Boolean through the clause
  checker over the names its stage may read, refusing a non-Boolean with
  `ill_typed`/`non-boolean-root` at the predicate and an undischarged
  definedness obligation as the retry relation. Operation success does not stand
  for recovery: S3 SHALL NOT derive the predicate from the attempt.
- S3 SHALL keep each template a separate obligation with its own captures,
  maximum, commit and predicate: a `Full` and a `Partial` recovery of one effect
  are two `CheckedCompensation`s even when their predicates are equal, and
  S3 SHALL NOT merge them.
- A refusal inside one template SHALL NOT stop S3 from checking the other
  templates, the captures and the activation. S3 reports every refusal, ordered
  by builder state and then by source position (ADR-012 §4.2).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-381-AC-1 | The refund template of TC-1057 checks: each of its seven stages reads the names its row of the table allows, and the checked clause holds the three capture stages, the activation and the one compensation. A registration capture that reads the recovery binder, a trigger guard that reads the forward binder, a retry relation that reads the trigger binder and a recovery predicate that reads `earlier` each refuse `missing_declaration`/`missing-name` at the read, retaining name, stage, owner and `read` `stage`. | Test (TC-1057) |
| FR-381-AC-2 | A capture whose initializer reads itself, a capture that reads a capture written later, and a registration capture that reads a capture of another template each refuse `missing_declaration`/`missing-name` at the read with `read` `forward`, `forward` and `foreign`; two captures that read each other give exactly one refusal, at the read inside the earlier capture, and the later capture checks. | Test (TC-1057) |
| FR-381-AC-3 | A capture with an unresolved type refuses `missing_declaration`/`missing-name` at the type; an initializer of another type refuses `ill_typed`/`type-mismatch` at the initializer, retaining capture, expected and actual; an initializer `view.tally + 1` declared as `M::Tally` refuses `undefined_expression`/`unproved-range` at the operation; a capture named like a visible binder refuses `ambiguous_declaration`/`ambiguous-name` as FR-113-AC-5. | Test (TC-1057) |
| FR-381-AC-4 | `activation on each (x: M::Tick) when (x.ready)` checks and the checked clause holds `OnEach` with binder `x`; `on origin` holds `Origin`. A guard that reads `view.total` refuses `ill_typed`/`operator-ineligible` at the read, naming the guard; a guard that reads a protocol capture refuses `missing_declaration`/`missing-name` with `read` `stage`; a non-Boolean guard refuses `ill_typed`/`non-boolean-root` at the guard; `M::Absent` as the binder type refuses `missing_declaration`/`missing-name`. | Test (TC-1057) |
| FR-381-AC-5 | A template whose forward binder type is not its effect's record type refuses `ill_typed`/`type-mismatch` at the type, retaining compensation, expected and actual; `by Absent` refuses `missing_declaration`/`missing-name` at `Absent`; `on M::Node::missing` refuses `missing_declaration`/`missing-name` at `missing`. The checked compensation holds the effect id, the operation's anchor node and frame node, and a protocol with an attempt and a template naming one operation holds one anchor node and one frame node. | Test (TC-1058) |
| FR-381-AC-6 | Two templates for one effect check and give two registrations; two templates of one name refuse `ambiguous_declaration`/`ambiguous-name` at both declarations. A template `for` an `attempt`, `send` or `event` node, and a `commit` that names an `effect`, each refuse as FR-113-AC-7 states. | Test (TC-1058) |
| FR-381-AC-7 | `attempts 0` refuses `ill_typed`/`type-mismatch` at the `0`, retaining compensation, member `attempts`, expected a count of at least one and actual `0`. `attempts 1` and `attempts 18446744073709551615` check and the checked compensation holds exactly that count; `attempts 18446744073709551616` fails at S1. | Test (TC-1058) |
| FR-381-AC-8 | `within [31,30]` and `within [0,*]` each refuse `ill_typed`/`type-mismatch` at the interval, as FR-326 refuses `[31,30]` in a temporal clause, retaining compensation, member `within`, expected a closed interval with `a <= b` and actual the interval as written; `within [0,0]` and `within [0,30]` check. | Test (TC-1058) |
| FR-381-AC-9 | With `attempts 3 of M::Flag` and retry binders `M::Attempt`, `earlier` and then `later` each refuse `ill_typed`/`type-mismatch` at the binder's type, retaining compensation, binder, expected `M::Flag` and actual `M::Attempt`; with the binders `M::Flag` it checks. `attempts 3 of M::Pulse`, an event type, with binders `M::Pulse` checks. | Test (TC-1058) |
| FR-381-AC-10 | A retry relation `earlier.tally < 3 and later.tally = earlier.tally + 1` checks; with `earlier.tally < 3 and` removed it refuses `undefined_expression`/`unproved-range` at `earlier.tally + 1`. A non-Boolean relation refuses `ill_typed`/`non-boolean-root`; a relation that reads a registration capture and an activation capture checks. | Test (TC-1058) |
| FR-381-AC-11 | `commit never` and `commit Main::Committed` check and the checked compensation holds `Never` and `Node` of the commit's id. A `commit` node that dominates the forward effect on the control tree refuses `unsupported_construct`/`declaration-form` at the commit anchor, retaining the selected protocol profile, compensation, member `commit`, commit, effect and reason `commit-precedes-effect`; a commit inside a `join all` branch, inside a branch of `quorum(2) [a, b]` and inside the one branch of `join any [a]`, each with the effect after the parallel, refuses the same way; a commit that follows the effect, a commit in a parallel branch whose sibling branch holds the effect (including when a `send` in the commit's branch and a `receive` in the effect's branch order them), a commit in a branch of `join any [a, b]` or of `quorum(1) [a, b]` with the effect after the parallel, a commit in the body of a `repeat` before the effect, and a commit in one `case` of a `choice` before the effect, check. | Test (TC-1058) |
| FR-381-AC-12 | A recovery predicate that is a non-Boolean refuses `ill_typed`/`non-boolean-root`; one with an undischarged range obligation refuses `undefined_expression`/`unproved-range`; an unresolved binder type refuses `missing_declaration`/`missing-name`; one that reads a registration and an activation capture checks. A `Full` template with commit `Main::Committed` and a `Partial` template with commit `never`, of one effect, give two checked compensations with their own captures, maxima and commits, in source order, and a refusal in `Full` and one in `Partial` are both reported, in source order. | Test (TC-1058) |
| FR-381-AC-13 | `attempts 3 of M::Node`, an object type, refuses `invalid_model_binding`/`malformed-declaration` at `M::Node`, retaining compensation, member `attempts` and name, and gives no refusal at the retry binders, whatever their type; the same refusal holds for the scalar type `M::Total`, a composite type and a variant type, and `attempts 3 of M::Absent` refuses `missing_declaration`/`missing-name` at `Absent`. | Test (TC-1058) |
| FR-381-AC-14 | A trigger guard and a recovery predicate that read a field of a binder of the object type `M::Node`, and an activation guard that reads a field of an `on each` binder of that type, each refuse `ill_typed`/`operator-ineligible` at the read, naming their stage; a retry relation over object binders refuses the same way; the same reads over a record-type binder check, and a registration capture that reads a field of an object binder checks. | Test (TC-1057) |
| FR-381-AC-15 | A trigger guard that reads `trigger.node.n` through the reference-typed field `node` of a record binder, and a recovery predicate that reads `deref(recovered.node).n`, each refuse `ill_typed`/`operator-ineligible` at the read, naming their stage; an activation capture that reads the same field through the reference checks. | Test (TC-1057) |

## Dependencies

- ADR-027 CP-1 to CP-5 (registration, activation, attempts, commit, recovery)
  and AE-1 (activation); ADR-012 §3, §4.2.
- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md) (the control
  tree, channels and decisions; the checked clause), [FR-308](FR-308-build-the-complete-protocol-forms-at-s2.md)
  (the forms), [FR-113](FR-113-resolve-scoped-anchors-in-nested-control-scopes.md)
  (anchors, kinds, no shadowing), [FR-114](FR-114-bind-a-protocol-attempt-to-its-operation-frame.md)
  (operation anchor and frame), [FR-104](FR-104-check-state-clauses.md) (the
  clause checker, definedness).
- QSpec owns the grammar and meaning of every construct: QSpec FR-055,
  FR-056, FR-057, FR-058, FR-172, FR-173 and the choreography surface contract
  (its compensation rows).
