---
id: SR-1738002
title: "Gap analysis of quire-spec-language PR #1738: memory-class catalog entries and memory rows (QSL-1181 slice 5)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2f3178357; PR #1738 slice commit against FR-375-AC-19, AC-23, AC-26, the FR-375 steps table and FR-432-AC-22 (STD-524/525)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-specification/FR-375
    type: reviews
---
# Gap analysis of quire-spec-language PR #1738

## Summary

Ticket: QSL-1181 (slice 5). Plan completion: not assessed.

**Covered:**

- **FR-375-AC-19, memory half.** `memory_rows_go_last_drain_before_flush` covers memory rows last, drain before flush, no `source_node`, and order-invariant ids.
- **FR-375-AC-23 and AC-26, emitted half.** `memory_catalog_entries_are_one_drain_per_model_then_flush` covers drain(ra), drain(sc), drain(tso) with `store-buffer-flush`, then flush(tso), and one entry per model across two `sc` parallels.
- **FR-432-AC-22** (the mandatory drain row) was covered by earlier slices.
- **FR-375-AC-26's refusal half** (reordered or repeated drains) is a reader check, with IR.

**A spec gap.** One `step memory drain` row decides the drain class for every source-default model. FR-432's class table gives the row no model, and the test writes one drain row over `sc`, `tso` and `ra` parallels. Yet the catalog holds one `memory drain` entry per model. FR-375 says the reader "SHALL compare every `steps[i].source_class` and `source_node` to one concrete entry", but a memory step has neither a `source_node` nor a `memory_model`, so its spelling matches up to three entries. QSL's emission is consistent with the row grammar. What the spec does not state is how one memory step entry joins a multi-model catalog: does it decide every entry of its class, or must the step carry `memory_model`? That leaves IR's reader (IR-874's AC-325 work) free to read it either way.

**Untracked ignore.** `memory_rows_read_back` is a third test ignored under IR-874. QSL-1181 comment 3b349555 names only two.

## Verdict

**The slice's ACs are met for emission.** There are two low items.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Spec gap, for STD-493 (or STD-525's owner): state how a memory step row joins the catalog when several source-default models share a memory class spelling. Either one `memory drain` step decides every `(memory drain, model)` entry, or the step carries `memory_model`, giving one step per entry. FR-375's "compare … to one concrete entry" does not fit one row over three drain entries. | QSpec FR-375 `steps` table and "Checked class catalog"; FR-432 class table, FR-432-AC-22; FR-375-AC-26 |
| FND-002 | low | Add `memory_rows_read_back` to QSL-1181's completion item 3b349555 (remove the IR-874 ignores when the IR dependency carries B2/B3), which today lists only `a_model_refinement_class_catalog_reads_back` and `a_row_empty_protocol_subject_reads_back`. | qsl-package/src/emit/tests/admission_corpus.rs (`memory_rows_read_back`); QSL-1181 comment 3b349555; IR-874 |
