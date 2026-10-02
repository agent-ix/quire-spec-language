---
id: FR-154
title: "Scope a modifies entry to the receiver and enforce it through check_frame"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
---
# FR-154: Scope a modifies entry to the receiver and enforce it through check_frame

## Description

A `modifies` entry has a scope: `receiver`, written `modifies self.f`, grants
writes to field `f` on the operation's receiver only; `every-object`,
written `modifies f`, grants them on every object whose effective type has
`f`, QSpec FR-013's meaning (ADR-021 POR-3, RU-1). QSL SHALL read the scope
at intake, carry it through the checked package and the S4 frame node, and
enforce it wherever a frame is decided: FR-120's `check_frame` and candidate
generation, and FR-106 check 11 for an invocation (FR-115). A change to `f`
on any object other than the receiver under a `receiver` entry is
`frame_violation`.

## Use case

A verification operator scopes `bump()`'s frame to its own counter. The
model checker then knows two bumps on different counters cannot interfere,
which partial-order reduction needs, and any step that writes another
counter's `v` is reported as a frame violation (US-019).

## Inputs

- A domain package whose operation frames carry a scope on each `modifies`
  entry (QSpec FR-013 with the receiver-scope amendment; the frame entry
  shape is QSpec's, see References).

## Outputs

```rust
pub struct ModifiesEntry {
    pub field: DeclarationKey,
    pub scope: FrameScope,
}

pub enum FrameScope { Receiver, EveryObject }

// OperationEffect.modifies: Vec<ModifiesEntry>

impl StateModel {
    pub fn check_frame(
        &self,
        effect: &OperationEffect,
        receiver: &ObjectReference,
        pre: &ModelState,
        post: &ModelState,
    ) -> Result<StateDelta, AdmissionFailure>;
}
```

## Behavior

- **Intake.** Intake SHALL read each `modifies` entry's scope, `receiver`
  or `every-object`, into `ModifiesEntry.scope` and SHALL resolve its field
  as FR-103 does. When an operation's frame holds both an `every-object` entry `modifies f`
  and a `receiver` entry `modifies self.f` for one field, intake SHALL
  refuse it with `invalid_model_binding`/`conflicting-binding`, naming both
  entries and the operation, as QSpec FR-013 states; intake never merges
  them.
- **Emission.** S4 SHALL emit each `modifies` entry of a `state`/`frame`
  node (FR-105) with its scope, as the checked-package v2 frame entry
  carries it (QSpec, see References). Two frames that differ only by an
  entry's scope SHALL have different node content.
- **Decision.** `check_frame` SHALL take the application's receiver. For an
  object `o` and field `f`, a change of `f` from pre to post SHALL be
  granted when an `EveryObject` entry names `f` and `o`'s effective type has
  `f` (FR-013), or when a `Receiver` entry names `f` and `o` is the
  receiver. Every other change of a field of a surviving object SHALL be
  `frame_violation`/`unauthorized-change`, naming the object and the field.
  `creates` keeps its meaning. A `deletes` entry carries its QSpec FR-013
  scope, read at intake and emitted with the frame node: a type-wide
  `deletes T` grants deleting any subset of the conforming objects; a
  `deletes self` grants deleting the receiver only; a `deletes p` grants
  deleting the argument bound to reference parameter `p` only. Deleting any
  other object under a scoped entry SHALL be `frame_violation`/
  `unauthorized-change`, naming the object.
- **One decision.** `population::decide_frame` SHALL take the receiver and
  apply this rule, so FR-120's `check_frame`, `enforce_frame` and FR-106's
  check 11 (and through it FR-115's `Frame` run) give the same verdict for
  the same effect, receiver, pre and post. FR-106 check 11 SHALL read the
  receiver from the invocation's `self` binding.
- **Candidates.** FR-120's post-state construction SHALL range a field that
  a `Receiver` entry grants over its domain on the receiver only, and keep
  its value on every other object.
- **Domains.** A field that only `Receiver` entries grant SHALL still be a
  root of `ModelSystem::domains()`, as FR-120 lists granted fields.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-154-AC-1 | The `test/counters` package of FR-120 with `increment()` framed `modifies self.value`, state `w` holding `c1` and `c2` at `value` 0, and no clause: `increment` on receiver `c1` has three successors, each changing only `c1.value`, and `c2.value` is 0 in every one. With the frame `modifies value` (every-object), the same application has nine successors, ranging both objects' `value`. | Test (TC-572) |
| FR-154-AC-2 | `check_frame` with effect `modifies self.value`, receiver `c1`, pre `w` and a post with `c2.value` 1 returns `frame_violation`/`unauthorized-change` naming `c2` and `value`; the same post with receiver `c2` returns a delta with no creation or deletion; with effect `modifies value` both return a delta. | Test (TC-572) |
| FR-154-AC-3 | FR-115's `Frame` run of an invocation of `increment` on `self = c1` whose post snapshot changes `c2.value` settles `violation` with the frame witness naming `c2.value` under `modifies self.value`, and `success` under `modifies value`. The FR-106 clause-run admission of the same invocation refuses `frame_violation` under `modifies self.value`. | Test (TC-573) |
| FR-154-AC-4 | The S4 frame node of `increment` under `modifies self.value` carries the scope `receiver`, and under `modifies value` the scope `every-object`; the two nodes' contents differ. A frame with both `modifies self.value` and `modifies value` is refused at intake with `invalid_model_binding`/`conflicting-binding` naming both entries and `increment`, and no frame node is emitted for it. | Test (TC-573) |
| FR-154-AC-5 | An operation `drop()` framed `deletes self` on receiver `c1`: `check_frame` admits a post without `c1` and refuses a post without `c2` as `frame_violation`/`unauthorized-change` naming `c2`; framed `deletes Counter`, both posts and a post without either are admitted; the S4 frame nodes of the two frames differ. | Test (TC-572) |

## Dependencies

- ADR-021 POR-2 (writes), POR-3, RU-1, TX-1 (1) and (2).
- [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md)
  (intake of frames), [FR-105](FR-105-emit-state-nodes.md)
  (the S4 frame node), [FR-106](FR-106-admit-snapshots-and-invocations.md)
  (check 11), [FR-115](FR-115-run-an-operation-frame-over-an-invocation.md)
  (the `Frame` run), [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`check_frame`, post-states and domains).
- QSpec FR-013 owns frame semantics; the receiver-scope amendment, its
  surface syntax and the scope member of the checked-package v2 frame entry
  are QSpec's (ADR-021 QS-4). The domain package's operation frame shape is
  the model contract's.

## References

- QSpec half: QSpec FR-013 and FR-340 (Linear STD-134; ADR-021 §9 QS-4). Owning ticket: Linear
  QSL-368.
