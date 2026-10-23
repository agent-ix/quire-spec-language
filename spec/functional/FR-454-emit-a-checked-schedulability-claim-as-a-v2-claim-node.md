---
id: FR-454
title: "Emit a checked schedulability claim as a v2 claim node"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-248
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: depends_on
---
# FR-454: Emit a checked schedulability claim as a v2 claim node

## Description

The S4 package emitter (ADR-011 layer 4 `package`, edge E4) SHALL emit each
checked `schedulable` or `response` claim (FR-246) as one
`quire.checked-semantic-graph/v2` node with `node_tag: "claim"` and
`semantic_form: "schedulability"`, whose body is the aggregate QSpec FR-482
states with the claim's whole task set inside it, and QSL's I2 reader SHALL
read that node back. QSpec FR-482 owns the body, the obligation identity and
the reader's refusals; this requirement states what QSL's emitter writes
from its checked `TaskSet` and `ScheduleClaim` and what its I2 reader
returns, so that the obligation identity FR-246 and FR-248-AC-4 rely on is
keyed from a node.

## Use case

An embedded engineer compiles a unit holding `Ctl` and the claims `CtlFp`
and `CtlEdf`. The package carries each claim as one schedulability claim
node, so a provider request names each item by `claim {node, occurrence}`,
FR-248's checker reads the task set back from the recompiled package, and a
WCET edit in the source gives a different node and so a different
obligation.

## Inputs

- A checked task set and a checked `ScheduleClaim` over it (FR-246), with
  every time value in the task set's unit.
- The node keys FR-092 mints and FR-093's literal spellings.

## Outputs

- One v2 node per distinct checked claim, body as QSpec FR-482 states, with
  `semantic_type` the package's `scalar_type`/`boolean` node, no
  `declaration`, no `owner`, `dependencies` `[]` and one occurrence of role
  `claim` per declaration. Two declarations with equal checked content share
  one node and differ by occurrence ordinal.
- One identity triple `[node, "claim", ordinal]` per `claim` occurrence, as
  FR-337 states for a temporal clause.
- From the I2 reader: the claim's kind, policy, `response` task and bound,
  and its task set, or FR-482's refusal with its locus.

## Behavior

- The emitter SHALL write `kind` as `schedulable` for
  `ScheduleClaim::Schedulable` and `response` for `ScheduleClaim::Response`,
  and `policy` as `fixed-priority`, `edf` or `amc` for `FixedPriority`,
  `Edf` and `Amc`; a `response` claim's policy is `fixed-priority`.
- The emitter SHALL write `task` as the text literal of the `response`
  task's name and `bound` as a `quantity` literal of `d` in the task set's
  unit, and both as the empty aggregate for a `schedulable` claim.
- The emitter SHALL write `tasks` as one task aggregate per task, ascending by
  task name in Unicode scalar value order: `name`; `arrival` `periodic` or
  `sporadic`; `period`, `deadline` and `wcet` (`C`, or `C(LO)` for a dual
  budget); `wcet_hi` (`C(HI)`, or the empty aggregate for a single budget);
  `criticality` `lo` or `hi`; `priority` as an `integer` literal, or the
  empty aggregate when the task declares none; `jitter` and `blocking`, `0`
  when the task declares none. Every time value SHALL be a `quantity`
  literal typed by the task set's unit node, its value the exact rational
  magnitude in that unit.
- The emitter SHALL key the node by FR-322's rules over its body, so the
  kind, the policy, the `response` task and bound, and every task's premises
  enter the node id, and the claim's and the task set's names do not.
- S4 SHALL NOT refuse a checked schedulability claim as not emitted.
- The I2 reader SHALL admit an emitted schedulability claim node and return
  its kind, policy, task, bound and task set, and SHALL refuse a node that
  breaks an FR-482 rule with FR-482's code at FR-482's locus, in FR-482's
  reader order.
- The I2 reader SHALL resolve a schedulability claim triple through a
  schedulability lookup that is separate from FR-337's temporal clause
  lookup and FR-410's state-graph lookup. Each lookup accepts only its own
  node form.
- If a triple given to the schedulability lookup names a NodeId the package
  does not hold, a node that is not a `claim`/`schedulability` node, or an
  occurrence whose role or ordinal differs, then the I2 reader SHALL return
  `clause-root-unresolved` at the looked-up triple.
- FR-248's checker SHALL read the claim and its task set from the
  recompiled package through the request item's triple, its `claim`
  argument, never from the caller or the outcome.
- If the triple does not resolve in the recompiled package, then the
  settlement SHALL settle the item `Failed` (FR-248's map) with no evidence
  checked, as FR-410 settles an unresolved state-graph triple.
- An outcome checked against an item whose triple names another node (for
  example the `under edf` claim's node for a fixed-priority outcome) SHALL
  refuse `stale_dependency`/`content-mismatch` naming both obligation
  identities, each keyed from its node, which is FR-248-AC-4's identity
  mismatch.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-454-AC-1 | Compiling ADR-026 §12's `Ctl` (in `ms`, tasks `a`, `b`, `c` with priorities 1, 2, 3) with `CtlFp`, `CtlEdf` and `response R on Ctl.c <= 12 ms under fixed_priority;` emits three `claim`/`schedulability` nodes: `CtlFp` with `kind` `schedulable`, `policy` `fixed-priority`, `task` and `bound` the empty aggregate and `tasks` the three task aggregates in name order, every time value a `quantity` literal typed by the `ms` unit node and `jitter` and `blocking` `0`; `CtlEdf` the same with `policy` `edf`; `R` with `kind` `response`, `task` `c` and `bound` `12`. Each has `dependencies` `[]` and occurrences of role `claim`; the I2 reader admits each and returns its kind, policy, task, bound and task set. No checked claim is refused as not emitted. | Test (TC-1202) |
| FR-454-AC-2 | `CtlFp` and `CtlEdf` emit different node ids and different obligation identities; `Ctl` with the third WCET 6 emits a different `CtlFp` node id and `package_id`; `Ctl` with its tasks listed in another order, or with `jitter 0 ms` written on one task, emits `CtlFp`'s node id; one unit that declares `CtlFp`'s claim twice under two names emits one node with two `claim` occurrences, ordinals 0 and 1. | Test (TC-1202) |
| FR-454-AC-3 | The I2 reader returns for `CtlFp` the triple `[node, "claim", ordinal]`, and resolving it in the recompiled package returns the claim and its task set; a triple with an unknown NodeId, with a `claim`/`state_graph` node's NodeId, or with another ordinal returns `clause-root-unresolved` from the schedulability lookup, and `CtlFp`'s triple given to FR-337's temporal lookup or FR-410's state-graph lookup returns `clause-root-unresolved`; a request whose triple is unresolved checks no evidence and settles the item `Failed`. `CtlFp`'s outcome checked against `CtlEdf`'s triple refuses `stale_dependency`/`content-mismatch` naming both obligation identities. | Test (TC-1202) |
| FR-454-AC-4 | Fed to QSL's I2 reader, AC-1's `CtlFp` node with `policy` `rate-monotonic` refuses `invalid_package`/`invalid-value`, with its tasks in the order `b`, `a`, `c` refuses `invalid_semantic_graph` at `tasks`, and with the first task's `period` `0` refuses `invalid_timed_model`/`invalid-task-set` at that task's aggregate, each at FR-482's locus, with the node key and `package_id` recomputed after each edit. | Test (TC-1202) |

## Dependencies

- ADR-011 §6.1 layer 4 `package` and §2.1 E4; ADR-026 RT-1 to RT-3, RT-7.
- [FR-246](FR-246-check-task-sets-and-schedulability-claims.md) (the checked
  task set, its unit and the claims), FR-092 (node keys),
  [FR-337](FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md)
  (the identity triple and `clause-root-unresolved`),
  [FR-410](FR-410-emit-a-checked-state-graph-claim-as-a-v2-claim-node.md)
  (the sibling state-graph lookup),
  [FR-248](FR-248-settle-a-closed-form-verdict-after-recomputing-its-evidence.md)
  (the checker that reads the node).

## References

- QSpec FR-482 (checked-package schedulability claim body and obligation
  identity), FR-322 (the artifact and its reader order), FR-419.
- Owning ticket: Linear STD-375; code: QSL-861.
