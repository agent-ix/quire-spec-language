---
id: FR-136
title: "Check a refinement's step rows at S3"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: depends_on
---
# FR-136: Check a refinement's step rows at S3

## Description

The refinement checker (FR-135) SHALL check the declaration's `step` rows
into `CheckedRefinement.steps` (ADR-020 RM-5, as amended by ADR-027 PR-1).
Every concrete step class has exactly one explicit row, and each right side
is written out: an abstract operation application, `stutter` or `any`.
Nothing defaults.

## Use case

A specification author maps each concrete operation to the abstract step it
implements, or to a stutter. When they forget an operation, or write a
right side whose arguments do not fit the abstract operation, the compiler
says so at the row, so no concrete step passes the refinement unchecked.

## Inputs

- The parsed `step` rows, each with its left side (a concrete operation, or
  over a protocol subject a node, template, channel, role, trigger or memory
  step kind) and its right side, with spans; the source is unclassified until
  S3 resolves it (FR-310).
- The concrete and abstract models' admitted operations with their
  parameters, results and frames (FR-103); over a concrete protocol subject,
  its checked protocol clause and memory model (ADR-027 PS-1, SE-1).

## Outputs

```rust
pub struct StepRow { pub class: StepClass, pub target: StepTarget }

pub enum StepClass {
    Operation(DeclarationKey),
    AttemptNode(NodeKey),            // takes precedence over its operation's row
    CompensationTemplate(NodeKey),   // `cattempt` and `cend`
    ProtocolNode { kind: ProtocolStepKind, node: NodeKey }, // event, send, receive, fork, join, timeout, finish
    Channel { kind: ChannelStepKind, channel: NodeKey },    // duplicate, lose
    Role { kind: RoleStepKind, role: NodeKey },             // spawn, retire
    Trigger(NodeKey),                                       // activate
    MemoryStep(MemoryStepKind),
}

pub enum StepTarget {
    Abstract {
        operation: DeclarationKey,
        receiver: CheckedExpr,
        arguments: Vec<Option<CheckedExpr>>,   // None is `_`
        binds_result: bool,
    },
    Stutter,
    Any,
}
```

## Behavior

### Order of the checked rows

- The refinement node's `steps` array that S4 emits for a model subject
  SHALL hold the rows in the order QSpec FR-375 gives: ascending by the
  node-key digest of the row's first named declaration (the concrete
  operation), then by member name as UTF-8 bytes. It is not source order.
- A row's `StepRowRef` (FR-146) SHALL be its zero-based position in that
  emitted order. `CheckedRefinement.steps` itself may keep source order, so
  long as the `StepRowRef` mapping and the emitted node follow FR-375.
- Over a protocol subject the same rule applies with a sort key that
  breaks the ties a protocol's rows have: `MemoryStep` rows name no
  declaration, and rows such as `duplicate` and `lose` of one channel, or
  `fork` and `join` of one `parallel` node, name the same declaration. A
  protocol subject's rows are ordered ascending by the tuple (1) the node-key
  digest of the declaration the row's left side names: the operation, or the
  attempt, event, send, receive, timeout, finish or fence node, the
  `parallel` node, the template, the channel or the role; for `activate` and
  `memory` rows, which name no declaration of their own, the selected protocol
  clause; (2) the row's leading step keyword (`fork`, `join`, `cend`,
  `duplicate`, `lose`, `spawn`, `retire`, `activate`, `memory`) as UTF-8
  bytes, a row with no keyword sorting first; (3) the memory step kind name
  as UTF-8 bytes, for `memory` rows. The tuple is unique per row, because
  coverage admits one row per class. It is the model case's rule (the
  declaration's key, then the step's name bytes) extended to the classes that
  share a declaration, and it is not source order.
- A protocol subject's `StepRowRef` is the row's zero-based position in that
  order, as for a model subject, and reordering the rows in source changes no
  `StepRowRef` and no emitted `steps` array.

### Selecting the concrete protocol

- The concrete alias `<C>` of the header (FR-135) names the concrete model
  and never a protocol. The step rows select the protocol: a row whose left
  side is `<C>::<protocol>::<name>`, `fork`, `join`, `cend`, `duplicate`,
  `lose`, `spawn` or `retire` followed by that form, or `activate
  <C>::<protocol>` (QSpec FR-432), names the protocol clause `<protocol>` of
  the concrete model. S3 classifies a row's source by resolving the segment
  after `<C>::` among the concrete model's object types and protocol clauses
  (FR-310 builds the source unclassified): a type makes it an operation row,
  a protocol clause makes it a protocol row.
- A refinement SHALL have one concrete protocol, or none. When no row names
  a protocol clause, the concrete subject is a model subject. When the rows
  name one, the concrete subject is that protocol subject and the checker
  SHALL record the clause as `CheckedRefinement.concrete_protocol` (FR-135).
  Every protocol clause declares exactly one `finish` node, whose row
  Coverage below requires, so a protocol subject always has at least one row
  that names its clause; a protocol is never selected by the absence of rows.
- Rows that name two different protocol clauses of `<C>` SHALL refuse
  `invalid_model_binding`/`conflicting-binding` at the first row, in source
  order, that names the second protocol, naming both rows. A refinement
  checks one protocol; a second protocol is a second refinement.
- A protocol name that is no protocol clause of the concrete model's package
  SHALL refuse `missing_declaration`/`missing-name` at that segment. A
  clause whose roles act only on a model other than `<C>` SHALL refuse
  `invalid_model_binding`/`malformed-declaration` at that segment, as QSpec
  FR-375 states for the abstract side.
- A `memory <kind>` row names no protocol. Its kind SHALL resolve among the
  memory step kinds of the memory model of the selected protocol (ADR-027
  SE-4), and with no protocol selected, or with a kind the model lacks, it
  SHALL refuse `missing_declaration`/`missing-name` at the kind.
- S3 coverage reads the selected clause and its resolved memory model only.
  The clause's `over` binding, its role bindings and the replicated roles'
  universes are inputs of the check request (ADR-027 PS-1; FR-142), which
  takes the clause from `concrete_protocol` and supplies those; none changes
  which rows are required.

### Order of the checks

- A refinement's header, population, object, fairness and history rows and
  a model subject's step rows depend on the models only and SHALL be
  checked wherever the refinement is reached.
- A protocol subject's step-row coverage depends on the selected clause being
  checked: its nodes, templates, channels, roles, trigger and memory model
  are what the classes are drawn from (FR-218, FR-215). S3 SHALL therefore
  check the protocol clauses of the unit before it checks the coverage of a
  refinement over one of them; equivalently, it defers the coverage of a
  protocol subject to a pass that has the checked clause, and a coverage check
  that runs before the clause is checked is a defect.
- When the selected clause was refused, S3 SHALL report the clause's own
  refusals and SHALL NOT report coverage refusals for classes drawn from it,
  and SHALL produce no `CheckedRefinement` for the refinement.
- A refinement over a model subject has no such dependency and its checks do
  not wait for protocols.

### Coverage over a model subject

- For each operation of the concrete model, the checker SHALL require
  exactly one row. An operation with no row SHALL refuse
  `missing_declaration`/`missing-name`, naming the operation. An operation
  with two rows SHALL refuse `invalid_model_binding`/`conflicting-binding`,
  naming both rows.
- A row whose left side names no concrete operation SHALL refuse
  `missing_declaration`/`missing-name` at its span.

### Coverage over a protocol subject

- When the rows select a concrete protocol (above), the checker SHALL
  require one row for every concrete step class of that protocol
  (ADR-027 ST-1 to ST-15):
  - an `attempt` step SHALL be covered by its operation's row or by a row
    naming its attempt node, and the node row SHALL take precedence for that
    node;
  - a `cattempt` step SHALL be covered by its operation's row or by a row
    naming its compensation template, and a `cend` step by the template's
    row;
  - each `event`, `send`, `receive`, `fork`, `join`, `timeout` and `finish`
    node SHALL have a node row;
  - each channel whose delivery policy admits `duplicate` or `lose` SHALL
    have a channel row for each step kind it admits;
  - each replicated role SHALL have a role row for `spawn`, and, when its
    lifetime is `until`, for `retire`;
  - each `activation on each` trigger SHALL have a trigger row;
  - each memory step kind the memory model has SHALL have a memory-step row.
- An operation row whose operation no attempt of the protocol uses decides
  nothing and is admitted; an operation row is required only where an
  attempt of that operation has no node row.
- A class with no row SHALL refuse `missing_declaration`/`missing-name`,
  naming the node, template, channel, role, trigger or memory step kind.
  Two rows for one class SHALL refuse
  `invalid_model_binding`/`conflicting-binding`.

### Right sides

- `stutter` and `any` SHALL be accepted as written. A row with no right side
  is a parse error at S2.
- For an abstract operation application, the operation SHALL resolve to an
  operation of the abstract model, refusing
  `missing_declaration`/`missing-name` otherwise.
- The receiver and each argument SHALL be checked as FR-135 checks a field
  row, with `self` bound to the concrete receiver, the concrete step's
  arguments in scope by parameter name, `pre(...)` reads of the concrete
  pre-state, history fields (FR-138) and population-valued forms (FR-139).
  The receiver SHALL conform to a reference to the abstract operation's
  receiver type after reference lifting, and each argument to its abstract
  parameter type; a mismatch SHALL refuse `ill_typed`/`type-mismatch` at the
  expression.
- The argument count SHALL equal the abstract operation's parameter count,
  with `_` for an argument the concrete step leaves open; a different count
  SHALL refuse `ill_typed`/`operator-ineligible` at the row, as QSpec FR-375
  states.
- When both the concrete step's operation and the abstract operation
  declare a result, the concrete result type SHALL conform to the abstract
  result type after reference lifting, refusing `ill_typed`/`type-mismatch`
  otherwise, and the row SHALL record `binds_result: true`. Otherwise it
  SHALL record `false`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-136-AC-1 | ADR-020 §8's `CasRefinesCounter` checks seven step rows: `commitA` and `commitB` as `Abstract{inc, receiver self, arguments [], binds_result: false}`, and the five others as `Stutter`. `RingIsQueue`'s `put` row checks as `enq(self.ring, x)` and its `take` row as `deq(self.ring)` with `binds_result: true`. | Test (TC-541) |
| FR-136-AC-2 | Removing the `peek` row refuses `missing_declaration`/`missing-name` naming `Impl::Counter::peek`; writing `peek` twice, once `-> stutter` and once `-> any`, refuses `invalid_model_binding`/`conflicting-binding` naming both rows. A row `step Impl::Counter::nope -> stutter` refuses `missing_declaration`/`missing-name`. | Test (TC-541) |
| FR-136-AC-3 | `step Impl::Counter::commitA -> Spec::Counter::inc(self, 1)` refuses `ill_typed`/`operator-ineligible` (one argument too many). In `RingIsQueue`, `put -> enq(self.ring, self.index = 0)` refuses `ill_typed`/`type-mismatch` at the argument; `put -> enq(self.ring, _)` checks with `arguments [None]`; `take` mapped to an abstract operation whose result is `Boolean` refuses `ill_typed`/`type-mismatch`. | Test (TC-541) |
| FR-136-AC-4 | Over a concrete protocol subject whose model has the two operations `incA` and `incB`, each mapped `-> Spec::Counter::inc(self)`, and whose `run` is a `parallel` with branches `A` and `B` holding one attempt of `incA` and one of `incB`, with rows for both operations and no `fork`, `join` or `finish` row, the checker refuses `missing_declaration`/`missing-name` naming the `fork` node first in source order. With node rows for `fork`, `join` and `finish` written `-> stutter` it checks; a further row naming branch `A`'s attempt node `-> any` checks and takes precedence over `incA`'s row for that node; two rows for the `join` node refuse `invalid_model_binding`/`conflicting-binding`. | Test (TC-541) |
| FR-136-AC-5 | Over a model subject, `CasRefinesCounter` checked with its seven `step` rows in source order and in the reverse source order emits the same `steps` array in ascending node-key digest order of each row's concrete operation, and gives each row the same `StepRowRef`. | Test (TC-541) |
| FR-136-AC-6 | With FR-136-AC-4's protocol, named `Run`, the concrete alias `M` names the model and the rows naming `M::Run::…` select the protocol, recorded as `concrete_protocol`; a refinement whose rows name no protocol clause has a model subject and `concrete_protocol` absent. A row `step M::Other::fin -> stutter` naming a second protocol clause `Other` of `M` beside `Run` refuses `invalid_model_binding`/`conflicting-binding` at that row naming both rows; `step M::Nope::fin -> stutter` refuses `missing_declaration`/`missing-name` at `Nope`; a clause of the package acting only on another model refuses `invalid_model_binding`/`malformed-declaration` at the protocol name. A `step memory flush` row under a model subject, and one naming a kind the selected memory model lacks, each refuse `missing_declaration`/`missing-name` at the kind. An operation row for an operation no attempt uses checks. | Test (TC-541) |
| FR-136-AC-7 | Over FR-136-AC-4's protocol subject, with `fork`, `join` and `finish` rows, a `duplicate` and a `lose` row for one channel and a `memory flush` row written in source order and again in reverse, the emitted `steps` array and every `StepRowRef` are equal, ordered by the tuple of FR-136's protocol sort key: the two channel rows share the channel's digest and `duplicate` precedes `lose`; the `memory` row carries the clause's digest and sorts by the kind name; the `fork` and `join` rows of the `parallel` node share its digest and `fork` precedes `join`. | Test (TC-541) |
| FR-136-AC-8 | A protocol clause that fails its own checks, named by a refinement's rows, reports the clause's refusals alone: no coverage refusal for a class drawn from it and no `CheckedRefinement` for the refinement. The same clause checked cleanly gives FR-136-AC-4's coverage refusals when a row is missing, whatever the declaration order of the clause and the refinement in the source. | Test (TC-541) |

## Dependencies

- ADR-020 §1 RM-5 and §11 RU-1; ADR-027 §2 ST-1 to ST-15, PR-1 and RU-5.
- [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md),
  [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md).
- FR-218 (the checked protocol clause) and FR-215 (memory step kinds):
  coverage over a protocol subject reads them, which fixes the order of the
  checks above.
- FR-310 builds the unclassified row sources this requirement classifies.
- FR-141 reads each row when it decides a step.

## References

- The QSpec half (grammar and step semantics): QSpec FR-375 and FR-377 (Linear STD-133).
