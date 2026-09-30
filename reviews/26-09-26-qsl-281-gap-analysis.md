---
id: SR-681
title: "Kernel ForeignReference record delivery gaps"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-096-AC-8; FR-100-AC-9; TC-322; TC-428; TC-452; quire-exact/src/outcome.rs; quire-exact/src/equality.rs; qsl-foundation/src/diagnostic.rs; qsl-replay/src/spine/call/tests.rs; src/command/output.rs; qsl-eval/tests/it/model_reference_queries.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
---
## Summary

Ticket: QSL-281. PR: quire-spec-language#479. This checks the
ticket's deliverables, and the ACs they touch, against tagged tests and code.

Delivered and traced:

- The variant carries both universes: TC-322, two tests in `quire-exact/src/equality.rs`.
- `kernel_refusal_record` builds the record: FR-096-AC-8 / TC-428, `a_kernel_foreign_reference_builds_a_record_with_both_universes`, with an independent hex oracle.
- The spine converts it to `CallRefusal::Record` with a locus: FR-100-AC-9 / TC-452 step 4, `tc_452_step_4_outcome_mapping_covers_every_category`.
- The CLI renders it with exit 20: FR-100-AC-9 / TC-452 step 4, `refused_foreign_reference_renders_record_and_exits_20`.
- The old no-record list no longer contains `ForeignReference` in `a_kernel_refusal_builds_a_record_only_where_the_catalog_has_a_code`.

Underspecified code: the left/right to `required`/`supplied` assignment has no
owning requirement (FND-001).

## Verdict

**Changes requested** on the trace gap (FND-001); coverage is otherwise complete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The operand-order rule is code with no owning requirement. No FR or AC says which operand's universe is `required`; only the variant's doc comment does. It is tested (TC-322 and the qsl-eval c10/e16 tests pin it), but against the implementation, not a spec. This is the same root as SR-682 FND-001 and the membership inversion in SR-680 FND-001. | quire-exact/src/outcome.rs:145-156; quire-exact/src/equality.rs:164-170 |
| FND-002 | low | No test raises `ForeignReference` through `Contains` (`x in c`), the second production caller of `member_equal`. The operand order there is untested and follows only from `member_equal`'s argument order. | qsl-eval/src/value/expression/evaluate.rs:1220 |

## Dispositions

Round 1, re-checked.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-096's key table gains a kernel `Refusal::ForeignReference` row owning the operand rule (code order corrected in the fix round; FR-100 states it too) |
| FND-002 | fixed | `c06b_contains_a_foreign_universe_probe_refuses_required_is_the_member` drives a checked `Contains` through `evaluate`; killed the membership-swap mutant |

Round 2, re-checked (spec-only; no code change since round 1). Every round-1 outcome stands; nothing is open.
