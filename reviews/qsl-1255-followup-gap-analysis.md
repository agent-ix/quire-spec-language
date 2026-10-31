---
id: SR-1939003
title: QSL-1255 follow-up gap-analysis
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language; QSL-1255; PR #1943 delta 620f64531...4eea75ef7;
  qsl-eval/src/lib.rs, qsl-eval/tests/it/accounting_boundaries.rs, qsl-eval/tests/it/named_population.rs'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/TC-136
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-137
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-049
  type: reviews
- target: ix://agent-ix/quire-spec-language/NFR-009
  type: reviews
---

## Summary

Ticket: QSL-1255. Independent review of PR #1943 at frozen head 4eea75ef7d99535bf15e65b815072244eed90582, base 620f64531. Three changed Rust files, tests and doctests only. No spec, semantics, CI or dependency change.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-049-AC-1 remains untagged and incomplete: TC-136 requires ProtocolSlot construction to fail, but ProtocolSlot has public fields and no such negative fixture is present. This is an acknowledged pre-existing assurance gap, not a production regression introduced by this PR. | qsl-eval/src/lib.rs:86-213 | correct-requirement-no-evidence |
| FND-002 | medium | TC-136 H3 requires the fault to name the invalid node. The new test correctly asserts Fault, stage, invariant, no Evaluation and an unchanged meter, but the current fault has no node identity and that clause remains unverified. This is an acknowledged pre-existing contract/source discrepancy, not a new production regression. | qsl-eval/tests/it/named_population.rs:769-845 | correct-requirement-no-evidence |
| FND-003 | medium | TC-137 still equates literal operand/element quantities with final usage. The independent schedule oracles correctly require integer_bits 42 versus operand 41, decimal_digits 7 versus operand 3, and value_occurrences 4 versus three elements. The literal equality remains unmet and is explicitly disclaimed; this inherited assurance gap needs an owned disposition, not a false completion claim. | qsl-eval/tests/it/accounting_boundaries.rs:324-405,584-589 | wrong-requirement |

## Verdict

FAIL for normative assurance completion: FR-049-AC-1 remains untagged and TC-136/TC-137 clauses remain unmet. The findings describe acknowledged baseline gaps; they do not identify new production regressions or negate the value of this tests-only delta.

## Review Evidence

Reconciliation: manual requirement-to-test and source inspection, bounded to the assigned delta and owning requirements. The whole-repository quire matrix was not run per dispatch: quire 0.37.1 / engine 0.50.3 is reported by the lead to hang for five minutes at 100% CPU. This reviewer did not reproduce that hang. No computed coverage percentages or whole-repository PASS are claimed. A repository-wide Rust trace search found no FR-049-AC-1 tag. FR-049-AC-7 and NFR-009-AC-3 tags bind to executable, nonignored assertions. The existing relationship_reachability graph-stack case and FR-365-AC-9 were read as context, outside this three-file delta; their measured small-chain calibration is unchanged.

Plan completion: not assessed

Optional separate semantic-analysis pass: not run; the required code-review lane assessed assertion intent. No spec/plan diff exists, so spec-review and its subanalyses are inapplicable. No new behavior without an owning requirement was found in the delta. Full repository reverse-gap/stub inventory and evidence-store reconciliation are outside this assigned scope and unavailable here.

## Scope

```yaml
scope:
- id: FR-049-AC-1
  path: spec/functional/FR-049-admit-composed-evaluation-inputs.md
  role: examined
  excerpt: Each checked-term evaluation selects an exact checked declaration/expression
    identity; raw bytes, freely constructed wire records and a foreign or out-of-range
    checked handle cannot enter an evaluation session.
- id: FR-049-AC-7
  path: spec/functional/FR-049-admit-composed-evaluation-inputs.md
  role: examined
  excerpt: Each applicable `accounting.<counter>` setting in FR-255's shared `quire.value.accounting/v1`
    table has zero/no-work, exact-usage and one-short evidence; over-budget work produces
    no completed partial value and stops at the owning requirement's charge point.
- id: FR-049-AC-8
  path: spec/functional/FR-049-admit-composed-evaluation-inputs.md
  role: examined
  excerpt: Repeating an evaluation over the same immutable checked package and runtime
    environment with a fresh sufficient meter returns the same result and leaves inputs
    unchanged; sequential protocol-slot calls on one meter accumulate in call order.
- id: FR-049-AC-10
  path: spec/functional/FR-049-admit-composed-evaluation-inputs.md
  role: examined
  excerpt: The compile path retains the selected source, model, profile and configuration
    identities under FR-036/FR-056/FR-110/FR-050; observation identities are admitted
    by FR-106. No runtime API accepts independent `StaticAuthority` members.
- id: NFR-009-AC-3
  path: spec/non-functional/NFR-009-bound-composed-evaluation.md
  role: examined
  excerpt: A counter overflow (a charge that would carry a consumed counter past `u64::MAX`)
    and a charge denied for a reason other than a configured limit (the stand-in for
    an allocator refusal) reached by a checked-term call each return the meter's incomplete
    record naming the affected counter and charge point, with no completed value and
    no wrapped or saturated count; no caller classifies either from diagnostic text.
- id: TC-136
  path: spec/test-cases/TC-136-admit-composed-state-views.md
  role: examined
  excerpt: 'First, a compile-fail public-API fixture attempts to

    pass raw bytes, a freely constructed wire record and a hand-built

    `CheckedExpression` to `CheckedPackageEvaluation::{call, evaluate}` and to

    build a `ProtocolSlot`; each fails to compile.'
- id: TC-136
  path: spec/test-cases/TC-136-admit-composed-state-views.md
  role: examined
  excerpt: '| H3 | `evaluate_holds` | a formula node id that is no `holds` atom of
    the clause, and an id beyond the clause''s node count | an `Err` fault naming
    the node; no `Evaluation` |'
- id: TC-137
  path: spec/test-cases/TC-137-bound-composed-state-evaluation.md
  role: examined
  excerpt: 'Count the fixture''s

    quantity independently from its construction (the bit length of the operand,

    the digits of the literal, the UTF-8 bytes of the text, the number of elements)

    and compare it with the metered usage from a successful run under a sufficient

    meter; the two must be equal.'
- id: TC-137
  path: spec/test-cases/TC-137-bound-composed-state-evaluation.md
  role: examined
  excerpt: 'On a 512 KiB stack, evaluate a 100,000-record-deep input, a

    10,000-call nested predicate chain and a 10,000-object graph chain.'
- id: TC-137
  path: spec/test-cases/TC-137-bound-composed-state-evaluation.md
  role: examined
  excerpt: 'Both controls classify the result from the record''s fields, never from

    diagnostic text, and a retry of C1 and C2 with a fresh meter completes.'
- id: FR-365-AC-9
  path: spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md
  role: context_only
  excerpt: The charges are the `graph.expand`, `graph.edge` and `graph.result-retain`
    schedule in discovery order with the stated `value_occurrences`, a repeated link
    charges one `graph.edge` per occurrence and is discovered once, and the exact
    required `work_units` and `value_occurrences` complete while one less is incomplete
    at the stated charge point.
- id: TC-136
  path: spec/test-cases/TC-136-admit-composed-state-views.md
  role: examined
  excerpt: '| H1 | `call` | a `QualifiedName` the package does not declare | `InputRefusal::UnknownFunction`
    naming it; no charge; no fallback to a display-name comparison |'
- id: TC-136
  path: spec/test-cases/TC-136-admit-composed-state-views.md
  role: examined
  excerpt: '| H2 | `call` | the name of a declaration the package holds that is not
    callable by name | the same `UnknownFunction` refusal as H1 |'
bindings:
- test_id: every_accounting_counter_has_zero_exact_and_one_short_evidence
  ac_id: TC-137
  trace: correct
- test_id: every_accounting_counter_has_zero_exact_and_one_short_evidence
  ac_id: FR-049-AC-7
  trace: correct
- test_id: a_fixture_with_no_work_completes_at_zero_for_every_counter_but_work
  ac_id: TC-137
  trace: correct
- test_id: a_fixture_with_no_work_completes_at_zero_for_every_counter_but_work
  ac_id: FR-049-AC-7
  trace: correct
- test_id: a_checked_constant_completes_with_every_counter_at_zero
  ac_id: TC-137
  trace: correct
- test_id: a_checked_constant_completes_with_every_counter_at_zero
  ac_id: FR-049-AC-7
  trace: correct
- test_id: raising_work_units_above_the_default_completes_work_the_default_stops
  ac_id: TC-137
  trace: correct
- test_id: a_work_units_overflow_is_the_meters_incomplete_record
  ac_id: TC-137
  trace: correct
- test_id: a_work_units_overflow_is_the_meters_incomplete_record
  ac_id: NFR-009-AC-3
  trace: correct
- test_id: an_injected_denial_is_the_meters_incomplete_record
  ac_id: TC-137
  trace: correct
- test_id: an_injected_denial_is_the_meters_incomplete_record
  ac_id: NFR-009-AC-3
  trace: correct
- test_id: a_ten_thousand_call_predicate_chain_completes_on_a_512_kib_stack
  ac_id: TC-137
  trace: correct
- test_id: a_hundred_thousand_record_deep_input_completes_on_a_512_kib_stack
  ac_id: TC-137
  trace: correct
- test_id: invalid_holds_handles_fault_without_spending_the_meter
  ac_id: TC-136
  trace: correct
```

## Validation

Reviewer reran cargo fmt --all -- --check (exit 0), cargo clippy --locked -p qsl-eval --all-targets --all-features -- -D warnings (exit 0), cargo test --locked -p qsl-eval --doc (23 passed), and the accounting_boundaries (8 passed) and named_population (9 passed) integration filters. Rust runs used CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0 and CARGO_BUILD_JOBS=3. Logs: /tmp/qsl1255-reviewer-{doc,clippy,accounting,named}.log. Candidate HEAD stayed 4eea75ef7d99535bf15e65b815072244eed90582 and git status remained clean. No branch edits, commits or pushes were made. Full aggregate and cargo-deny gates belong to the lead; no dependency diff exists. The lead supplied a current-main C14 MalformedWire control (/tmp/codex1-qsl-1359-main-control.log); this review does not call that failure a new delta regression.

## Coverage

Reconciliation: manual requirement-to-test and source inspection, bounded to the assigned delta and owning requirements. The whole-repository quire matrix was not run per dispatch: quire 0.37.1 / engine 0.50.3 is reported by the lead to hang for five minutes at 100% CPU. This reviewer did not reproduce that hang. No computed coverage percentages or whole-repository PASS are claimed. A repository-wide Rust trace search found no FR-049-AC-1 tag. FR-049-AC-7 and NFR-009-AC-3 tags bind to executable, nonignored assertions. The existing relationship_reachability graph-stack case and FR-365-AC-9 were read as context, outside this three-file delta; their measured small-chain calibration is unchanged.

Plan completion: not assessed

Optional separate semantic-analysis pass: not run; the required code-review lane assessed assertion intent. No spec/plan diff exists, so spec-review and its subanalyses are inapplicable. No new behavior without an owning requirement was found in the delta. Full repository reverse-gap/stub inventory and evidence-store reconciliation are outside this assigned scope and unavailable here.

The inherited SR-1939001 FND-006 tracker discrepancy remains open and is owned by the lead; this review does not silently close or rewrite it.

## Dispositions

Round 1.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Deferred to [QSL-1363](https://linear.app/agent-ix/issue/QSL-1363/resolve-tc-136-protocolslot-construction-contract-against-the-public) (Specify); code lead adopted this pre-existing contract/production gap outside the user-authorized tests-only PR. The gap remains unresolved; no production or specification fix is claimed. |
| FND-002 | deferred | Deferred to [QSL-1364](https://linear.app/agent-ix/issue/QSL-1364/name-the-rejected-holds-node-in-the-tc-136-h3-internal-fault) (Specced); code lead adopted this pre-existing contract/production gap outside the user-authorized tests-only PR. The gap remains unresolved; no production or specification fix is claimed. |
| FND-003 | deferred | Deferred to [QSL-1365](https://linear.app/agent-ix/issue/QSL-1365/reconcile-tc-137-operand-quantities-with-intermediate-and-retained) (Specify); code lead adopted this pre-existing contract/production gap outside the user-authorized tests-only PR. The gap remains unresolved; no production or specification fix is claimed. |

## Disposition verdict (round 1)

At head `907cd223883c3798475d07ad783daf51483f457e`, the only change since reviewed `4eea75ef7` is the report-only commit copying SR-1939002 and SR-1939003 verbatim into reviews/. Byte comparison passed, and git diff over qsl-eval is empty. No new code review or aggregate run was needed. No new finding or regression was identified.

Live tracker verification: QSL-1255 is In Progress, assigned to Peter Krenesky, with has-review retained. Comment f3bbb401-f26f-4833-888a-c1254f7d07ec, created 2026-10-31T16:59:53.155Z, explicitly retains the remaining scope and defers the three gaps to child tickets QSL-1363 (Specify), QSL-1364 (Specced), and QSL-1365 (Specify). All three child-parent links were verified. The children currently have no individual assignee; the dispatching code lead has adopted their routing and the parent remains assigned. This is tracked deferral, not a claim of completed implementation.

The original finding rows and verdict remain unchanged as historical review evidence. The disposition below is current: partial tests-only delivery is mergeable from the review standpoint, subject to the lead's remaining gates and existing baseline policy. Normative assurance completion still FAILS until the linked work is resolved. The inherited completion-state discrepancy SR-1939001 FND-006 is separately fixed by the reopened parent and recorded remaining scope; none of the production gaps is fixed by that administrative repair.
