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
  step kind) and its right side, with spans.
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

### Coverage over a model subject

- For each operation of the concrete model, the checker SHALL require
  exactly one row. An operation with no row SHALL refuse
  `missing_declaration`/`missing-name`, naming the operation. An operation
  with two rows SHALL refuse `invalid_model_binding`/`conflicting-binding`,
  naming both rows.
- A row whose left side names no concrete operation SHALL refuse
  `missing_declaration`/`missing-name` at its span.

### Coverage over a protocol subject

- When the concrete model is checked as a protocol subject, the checker
  SHALL require one row for every concrete step class of the protocol
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
  SHALL refuse `invalid_model_binding`/`malformed-declaration` at the row.
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
| FR-136-AC-3 | `step Impl::Counter::commitA -> Spec::Counter::inc(self, 1)` refuses `invalid_model_binding`/`malformed-declaration` (one argument too many). In `RingIsQueue`, `put -> enq(self.ring, self.index = 0)` refuses `ill_typed`/`type-mismatch` at the argument; `put -> enq(self.ring, _)` checks with `arguments [None]`; `take` mapped to an abstract operation whose result is `Boolean` refuses `ill_typed`/`type-mismatch`. | Test (TC-541) |
| FR-136-AC-4 | Over a concrete protocol subject whose model has the two operations `incA` and `incB`, each mapped `-> Spec::Counter::inc(self)`, and whose `run` is a `parallel` with branches `A` and `B` holding one attempt of `incA` and one of `incB`, with rows for both operations and no `fork`, `join` or `finish` row, the checker refuses `missing_declaration`/`missing-name` naming the `fork` node first in source order. With node rows for `fork`, `join` and `finish` written `-> stutter` it checks; a further row naming branch `A`'s attempt node `-> any` checks and takes precedence over `incA`'s row for that node; two rows for the `join` node refuse `invalid_model_binding`/`conflicting-binding`. | Test (TC-541) |

## Dependencies

- ADR-020 §1 RM-5 and §11 RU-1; ADR-027 §2 ST-1 to ST-15, PR-1 and RU-5.
- [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md),
  [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md).
- FR-141 reads each row when it decides a step.

## References

- The QSpec half (grammar and step semantics): QSpec FR-375 and FR-377 (Linear STD-133).
