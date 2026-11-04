---
id: SR-1640003
title: "Spec review of PR 2291 edits to FR-520 (AC-11, outcome map) and TC-1315 (QSL-1640)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0bd98250e0bfe4c9bfc353a23534e19126172115; QSL-1640; PR #2291; spec/functional/FR-520-evaluate-state-and-compensation-expressions-through-the-checked-package-evaluation-entry-points.md; spec/test-cases/TC-1315-evaluate-state-and-compensation-expressions-through-the-checked-package-evaluation-entry-points.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-520
    type: reviews
---
# Spec review of PR 2291

## Summary

Ticket: QSL-1640. Read against quire-exact FR-358-AC-16 to AC-21 (IR-1280, Done: the kernel exposes a disabled-by-default ten-counter interval and states that binder routing, read-presence entries, per-call conservation and the semantic `accounting_version` belong to QSL, not the kernel) and quire-protocol FR-045-AC-7 and AC-8 (consumer, merged main; AC-8 consumes the log "as QSL FR-520-AC-11 states it").

The PR removes the IR-1280 gating from AC-11, the outcome-map row, the Dependencies bullet and TC-1315 row 12 (correct: IR-1280 is delivered and PR 2291 implements the routing), and it also rewrites the attribution rule itself from "every charge made while reading or evaluating a binder's supplied value" to "every charge made after a binder's supplied value is read and before the next binder's is read". IR-1280 does not define the attribution, so there is no conflict with IR-1280 itself; the conflict is between the new rule and the criterion's original intent (FND-001).

Recommendation on the spec decision (owner decides): change the implementation, not the criterion. A charge belongs to binder B when it is made by reading B's value or by an operation whose binder-derived operands all come from B; an operation that combines values from two or more binders, and every charge with no binder-derived operand (literal-only subexpressions, branch and short-circuit control), is recorded under none. This keeps the original AC-11 sentence true and makes the per-binder numbers invariant under operand order. Required code change: track on the evaluator's value stack which single binder (if any) each pushed value derives from, propagate it through operator application, and route the interval taken around each task step to that owner instead of to "the binder read last"; `ChargeRouting::read` then only registers read presence. Required spec/test change: keep the AC wording, add a sentence defining "made by a binder", and add TC-1315 procedure and tests for (a) a literal-only subexpression after a read lands in the unattributed entry, (b) an operator over two binders lands in the unattributed entry, (c) commuting the operands of a two-binder operator leaves every per-binder entry unchanged, (d) a single-binder operator after the other binder's read lands on its own binder. If the owner prefers the positional rule, the fallback is to keep the code and the PR's wording but state in FR-520 that the log is a read-ordered partition of the call's charges, not the cost caused by each binder, and keep FR-045 from describing it as per-binder cost.

## Verdict

**Needs a decision before merge** on FND-001; FND-002 and FND-003 are fixable in the same PR.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The outcome-map row now defines ownership positionally ("after a binder's supplied value is read and before the next binder's is read; charges made before the first read under none"). Under that wording an operator over two binders, and any constant subexpression evaluated after a read, is charged to the last binder read, and swapping the operands of a symmetric operator moves the charge to the other binder. The row and AC-11 also still speak of "charges made by no binder", which the positional rule can only ever assign to the prefix before the first read. The criterion's original wording ("while reading or evaluating a binder's supplied value", "charges made by no binder") describes causal ownership; the rewrite changes the requirement to match the implementation. | spec/functional/FR-520-evaluate-state-and-compensation-expressions-through-the-checked-package-evaluation-entry-points.md:78 | wrong-requirement |
| FND-002 | medium | FR-520-AC-11 cannot fail on mis-attribution: it asserts that read binders have an entry, an unread binder has none, the entries combine to the total, equal supplies give equal logs and the version is present. A log that assigns every charge to the first binder read, or to the wrong binder, satisfies every clause. No adverse case is required: a nonzero unattributed entry, operands in the other order, or a binder-free subexpression after a read. TC-1315 row 12 has the same gap. | spec/functional/FR-520-evaluate-state-and-compensation-expressions-through-the-checked-package-evaluation-entry-points.md:107 | missing-requirement |
| FND-003 | medium | Nothing in FR-520 says what happens to a meter interval the caller enabled before the call, although the implementation drains it and discards its amounts and leaves capture enabled and empty. The behavior is a public contract of `evaluate_protocol_slot`; state it (or require a refusal) and add it to AC-11 or a new criterion with a non-empty pre-existing interval in TC-1315. | spec/functional/FR-520-evaluate-state-and-compensation-expressions-through-the-checked-package-evaluation-entry-points.md:78 | missing-requirement |

## Coverage

Examined: FR-520 outcome-map row for per-binder accounting; FR-520-AC-9, AC-11; FR-520 Dependencies; TC-1315 row 12 (AC-11 step) and its scope line. Context only: quire-exact FR-358-AC-16 to AC-21, quire-protocol FR-045-AC-7 and AC-8. `quire validate` of the two edited spec files passes. The Contract-IR dependency edit and the removal of "gated on IR-1280" are correct and need no change.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The rule at call boundaries is not stated, and the implementation departs from the stated rule on dispatch. The outcome-map row says a value derived only from B "through operations and let bindings" belongs to B and an operation over one binder is under that binder, but says nothing about function-call arguments, dispatched calls, or iteration, and a dispatched call with arguments from two binders makes every callee operation unattributed (SR-1640001 FND-006). State the call, dispatch and iteration rules (or that a dispatched call's body is unattributed when its arguments come from two binders) and add the cases to TC-1315 row 12. | spec/functional/FR-520-evaluate-state-and-compensation-expressions-through-the-checked-package-evaluation-entry-points.md:78 | missing-requirement |

## Dispositions

Disposition round 1 at PR head `28eeb158e76a5956b7e99b08332a3bbe38d20b6a` (contains current origin/main 137f723a6, so the trial merge is the head).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The outcome map now defines "made by binder B" by operand provenance (reading B, or an operation whose binder-derived operands all come from B; operations over two binders and literal-only computations are made by no binder). |
| FND-002 | fixed | AC-11 adds the adverse cases: a literal-only subexpression and an operation over two binders are under no binder, commuting the operands leaves the log unchanged, an operation over one binder is under that binder; TC-1315 row 12 lists the matching procedure and expected results. |
| FND-003 | fixed | The outcome-map row states that the call drains an interval the caller had open, does not add it to the log, and leaves capture as found; TC-1315 row 12 includes a caller-enabled interval holding a charge. |
