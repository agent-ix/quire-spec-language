---
id: SR-16150
title: "Spec review — FR-059 checks qsl-replay is CG's only QSL dependency"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-059-check-backend-dependency-direction.md, spec/test-cases/TC-156-check-backend-dependency-direction.md at 2ba8fd583"
review_set: subset
evaluated_revision: "2ba8fd583b46b9edaba9aeb38d2533a45c7e7f42"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-156
    type: reviews
---

## Summary

Ticket: STD-316. PR quire-spec-language#1278 names the FB-05 exception's
package (`qsl-replay`), makes every other CG normal dependency on a QSL-classified package a
violation, states the `agent-ix/quire-eval` crates are no FB-05 edge, and adds FR-059-AC-10
(synthetic), FR-059-AC-11 (real CG manifest) and TC-156 steps 9-10. Reviewed
`git diff origin/main...HEAD` at 2ba8fd583 against ADR-011 FB-05 and the C/E/EC crate rows,
FR-060 (T12-A and the evaluator corpus), FR-356-AC-1, and `tools/arch-lint/graph.rs` on main
(`tc_arch_lint_direction_001`, `_004`).

The quire-eval-crate clause agrees with FR-059's classification rule (only packages sourced
from the QSL repository classify as QSL) and with FR-060. AC-10 and AC-11 are unique across
main and all 24 open PR heads. `make check-spec-validation` exits 0. Wording is
behaviour-only. No textual conflict with any open PR.

## Verdict

NOT MERGEABLE as written: one medium finding (the unrefined AC-1 and Outputs now
disagree with the new rule), one low. Both are small edits in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new clauses make only CG's normal dependency on `qsl-replay` the exception, but FR-059-AC-1 ("a graph with only CG's normal dependency on QSL ... reports FB-05 clean") and the Outputs line ("edges into QSL that are not CG's normal dependency on QSL") still speak of any CG normal dependency on QSL. Under the new rule a CG normal edge via `qsl-semantics` is a violation, so AC-1 as worded no longer fixes the outcome. Say "CG's normal dependency on `qsl-replay`" in AC-1, the Outputs line and TC-156 step 1 (the step-1 test on main already uses `qsl-replay`). | spec/functional/FR-059-check-backend-dependency-direction.md:34-35,92; spec/test-cases/TC-156-check-backend-dependency-direction.md step 1 |
| FND-002 | low | FR-356-AC-1 on main already states that the direction check refuses an edge "to any `qsl-*` crate other than CG's normal edge to `qsl-replay` (FB-05)", and `tc_arch_lint_direction_004` (tools/arch-lint/graph.rs:315) tests it, so the gap was the FR-059 statement and the real-CG run, not enforcement. Cite FR-356-AC-1 from FR-059 so the two ACs are read as one rule, not two. | spec/functional/FR-059-check-backend-dependency-direction.md:100; tools/arch-lint/graph.rs:310-330 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | "`qsl-replay` does not re-export them" is false on main and on the open heads: `qsl-replay` re-exports SV items (`qsl-replay/src/lib.rs:183` `EqualityOperator`, `:252` `InputRefusal`, `src/spine.rs:43` `CheckingLimits`), and main still has `qsl_replay::Category`, which #1280 (STD-335) removes. Scope the sentence to the types FR-401 moves (for example "`qsl-replay` re-exports none of the types FR-401 moves, such as `Category`", citing FR-401) or drop it. | spec/functional/FR-059-check-backend-dependency-direction.md:73-82; qsl-replay/src/lib.rs:183,252; qsl-replay/src/spine.rs:43 |
| FND-004 | low | "CG's allowed set of QSL-owned dependencies is ... `qsl-replay` ... plus the shared leaves `quire-semantic-value` and the `agent-ix/quire-eval` crates, and nothing else" calls `quire-semantic-value` (its own repository) QSL-owned while leaving out `quire-exact` and `quire-walk`, which ADR-011 FB-05 lists in the same leaf class with CG edges allowed. Read literally, "nothing else" forbids CG -> `quire-exact`/`quire-walk`. AC-11 is scoped to three sources and is fine; name the allowed set by source the same way, or say the other FB-05 leaves stay allowed. | spec/functional/FR-059-check-backend-dependency-direction.md:73-82 |

## Dispositions

Round 1, checked against fix commits c6c832ddd (SR-16150 fixes) and 234eea163 (shared-leaf allowance). `make check-spec-validation` exits 0; main is an ancestor; no textual conflict with any current open PR. The shared-leaf allowance agrees with FR-059's classification rule and ADR-011 FB-05, but its rationale sentence carries two new findings, FND-003 and FND-004 (section above).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-1, the Outputs line and TC-156 (description and step 1) say CG's normal dependency on `qsl-replay`. |
| FND-002 | fixed | FR-059 cites FR-356-AC-1 and `tc_arch_lint_direction_004` as the same rule, adds a `references` edge and a Dependencies entry. |
