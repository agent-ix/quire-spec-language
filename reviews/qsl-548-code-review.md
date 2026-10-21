---
id: SR-12278
title: "Code review (with rust-review) of QSL-548 step simulation occurrence keys (PR #1164)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f9e64bcfa (qsl-548-impl, diff origin/main...HEAD); qsl-semantics/src/check/step_simulation.rs, qsl-semantics/src/check/family.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/tests/it/refinement_declarations.rs"
review_set: subset
---

## Summary

Ticket: QSL-548 (PR quire-spec-language#1164). Code review with the
rust-review lane folded in, of the FR-146 record half that ADR-013 O-07 and
O-09 (as amended by #1157) unblocked: `step_simulation_records` now returns
`StepSimulationEntry`, which pairs each QSV `StepSimulationRecord` with its
O-07 occurrence key (refinement node, role `generated`, ordinal 0 for
`Initial`, row position + 1 for `Step`).

Checks run (own target dir, through `locked-build.sh`):
`cargo test -p qsl-semantics --features test-support --test it
refinement_declarations` (28 passed); `cargo test -p qsl-replay --test it
refinement_history` (28 passed); `cargo clippy -p qsl-semantics -p
qsl-replay --all-targets --all-features -- -D warnings` (clean).

The PR body's "Remaining work: QSL-548" items (refinement node minting,
protocol-subject rows, FR-314/FR-468 settlement, package refusals) are known
and are not findings here.

## Verdict

PASS. No high or medium finding; one low.

The occurrence key matches O-07 exactly. The node is `wire(refinement_key)`,
the same digest bytes as the record's `refinement` member, which O-09 says
must equal the owning node key. The role is `OccurrenceRole::Generated`
through its one kernel spelling (`kernel_role`, widened to `pub(crate)`), so
no new role string is typed. The ordinal and the `StepRowRef` come from one
`position` value, so the O-09 coupling (`Initial` at ordinal 0, `Step { row:
i }` at ordinal `i + 1`) holds by construction. `u64::from(position) + 1`
cannot overflow, because `position` is a `u32`. An ineligible refinement
(hidden field, history field, `any` row) still returns an empty vector, so it
carries no generated occurrences, as FR-322-AC-91 requires. The key stays
outside `StepSimulationRecord`, so it is outside the content identity CG
hashes, as O-09 requires.

FR-375 ordering concern, checked; nothing is wrong. `CheckedRefinement::steps()`
is in source order, and `step_simulation_records` never reads a row's
content: it emits positions `0..steps().len()`, which are FR-375 positions by
definition, and the record set does not depend on source order.
`step_row_refs` (unchanged here) maps each source row to its FR-375 position,
sorted by the operation's node-key bytes. Its tie-break compares the full
qualified name (`operation.node`) where FR-375 says "member name". The
tie-break cannot be reached: a model subject has one row per concrete
operation, and content-addressed operation node keys never tie. So the
ordinal `i + 1` names the row at FR-375 position `i`, as O-07 requires.

API shape: `StepSimulationEntry` has a private constructor and read-only
accessors, so no caller can build a key whose ordinal disagrees with the
record's obligation. O-09 forbids QSL from "inventing another record type". A
QSL-side carrier that pairs the O-07 key with the QSV record is not a record
type in that sense: it adds no wire type and no identity member. The one
external caller (`qsl-replay` `refinement_history`) compiles unchanged
against the new return type.

## Examined Scope

```yaml
scope:
  - {id: FR-146, path: spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md, role: examined, excerpt: "The record's content identity is specified by ADR-013 O-09; its occurrence key follows QSpec FR-322-AC-88 through AC-91, as described below."}
  - {id: ADR-013 O-07, path: spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, role: examined, excerpt: "For a QSV StepSimulationRecord, the occurrence key names the owning checked relation/refinement node, with role generated; the record does not mint a second checked node id. Only a proof-eligible refinement carries these generated occurrences. ... The initial record has ordinal 0. A step record for zero-based row i has ordinal i + 1, where i is the row's position in the refinement's FR-375 steps total order (FR-375-AC-19), not its source order."}
  - {id: ADR-013 O-09, path: spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, role: examined, excerpt: "The O-07 occurrence key is carried separately and excluded from this digest. ... The refinement member must equal the owning node key. obligation is Initial for ordinal 0, or Step { row: i } for ordinal i + 1"}
  - {id: qsl-semantics/src/check/step_simulation.rs, path: qsl-semantics/src/check/step_simulation.rs, role: examined, excerpt: "pub struct StepSimulationEntry { occurrence: OccurrenceKey, record: StepSimulationRecord }"}
  - {id: qsl-semantics/src/check/family.rs, path: qsl-semantics/src/check/family.rs, role: examined, excerpt: "pub(crate) fn kernel_role(self) -> Role"}
  - {id: qsl-semantics/tests/it/refinement_declarations.rs, path: qsl-semantics/tests/it/refinement_declarations.rs, role: examined, excerpt: "fn each_record_has_its_generated_occurrence_key(); fn a_hypothesis_is_its_own_operation_contract_record()"}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `OccurrenceRole::Generated` is documented as "A node no source region denotes". This PR makes it the role of the per-step record occurrences. ADR-013 O-07 gives each of those a `source_map` region: `Initial` names the refinement declaration region and `Step` names its authored step-row region. The variant's doc now contradicts the only use of the variant. Reword it to cover a generated occurrence of an owner node, which can carry regions. | qsl-semantics/src/check/family.rs:1333-1334 |

## Coverage

Not applicable to this method; see SR-12279 (gap-analysis).

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed | HEADSHA |
