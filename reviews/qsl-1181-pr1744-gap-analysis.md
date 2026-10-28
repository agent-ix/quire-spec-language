---
id: SR-1744002
title: "gap analysis of PR #1744 (QSL-1181 slice 6): refinement adverse corpus"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1744 slice commit 8d1753244; FR-375-AC-21/23/26, FR-432-AC-3/22, the QSL-1181/1186 refusals; the recorded gaps in QSL-1181 comment 59913fe0"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1744002: gap analysis of PR #1744 (QSL-1181 slice 6)

## Summary

Ticket: QSL-1181. Head: 8d1753244.

**Refusal coverage.** The stack introduced these source-reachable refusals:
- `RefinementNotEmitted` for row forms with no node form.
- The five not-written protocol constructs, at the clause and at the refinement.
- The memory-row refusals. FR-432-AC-3/22 are covered with code, cause and locus by `a_parallel_needs_one_memory_drain_row_under_every_source_default_model`.
- `SameMeaningRefinements`, which is on main.

All of them are now covered. The remaining additions are `InternalFault` invariants (`catalog_fault`, `rows_fault`, `add_model_operations`) and `EmitRefusal` arms for checker-guaranteed facts. None of them is reachable from source.

**The two gaps recorded in QSL-1181 comment 59913fe0:**
- **Gap 1, the FR-375-AC-21 out-of-order row, is not blocked (FND-001).**
  - Nothing external stops it. `rows_refinement_with` already edits the ConfigVersion model JSON in-test: it drops `parent` and rewrites `operations`.
  - Adding a second scalar field, or a second population, is the same kind of edit. That gives two rows in one array on the admitted `emit_rows(MAPS)` baseline.
  - The step-row fixtures do not read back (IR-1077), but object and population rows do read back, and this PR's own AC-21 test is built on exactly that baseline.
  - It is deferred work, and it belongs in this PR.
- **Gap 2, the FR-375-AC-23/26 catalog mutations, is really blocked, but the ticket it cites is closed (FND-002).**
  - IR-874 is Done (completed 2026-10-28T10:24Z), and IR #647 "reconstruct refinement catalog classes and paths" merged at 11:13Z.
  - I bumped the lock to IR main ca23abbd, which is 5 commits after #647, as a reverted probe. All five ignored read-backs still refuse:
    - `invalid_semantic_graph` at `class_catalog/concrete/1` (model), `class_catalog/abstract/0` (protocol subject), and `class_catalog/concrete/0` (memory and protocol step rows).
    - `operation_rows_read_back` refuses `invalid_model_binding` at `steps/0/source_class` (IR-1077).
  - The baseline is still refused, so mutations still cannot be told apart from it. But no open IR ticket tracks this. The `#[ignore = "IR-874: ..."]` reasons and the gap text name a closed ticket.

**AC-21 remainder (FND-003).** The clause "an absent direct dependency for an external `abstract` key refuses `missing_declaration`/`missing-selection`" is neither tested nor recorded as a gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Gap 1 (AC-21 out-of-order row) is not blocked: add a second field or population to the in-test model JSON and test it in this PR | FR-375-AC-21 |
| FND-002 | medium | Gap 2 is really blocked at IR main ca23abbd, but it cites IR-874, which is Done: file or reopen an IR ticket and repoint the ignore reasons and the gap | FR-375-AC-23, FR-375-AC-26 |
| FND-003 | low | The AC-21 external-`abstract` `missing-selection` refusal is neither tested nor recorded | FR-375-AC-21 |

## Verdict

Changes requested: FND-001 and FND-002.

## Dispositions

Round 1, at f8925a6b9 (fix 48914b039 plus the IR-1093 follow-up). Every finding is fixed:
- SR-1744001 FND-001: the `body/abstract` locus is now asserted.
- SR-1744002 FND-001: `an_out_of_order_row_refuses_at_its_first_offending_entry` refuses at `objects/0/fields/1`.
- SR-1744002 FND-002: the ignores cite IR-1093, which is open.
- SR-1744002 FND-003: the external-key `MissingSelection` case is tested at `body/abstract`.

Verdict: mergeable.
