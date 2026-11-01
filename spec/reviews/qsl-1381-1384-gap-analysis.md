---
id: SR-1957024
title: "Gap analysis — Layer-4 checked temporal and authenticated successors"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1381 FR-043/044, QSL-1382 FR-045, QSL-1383 FR-046-AC-5, QSL-1384 FR-050; full origin/main...HEAD at 95ec6998923cbb58fb831a1422f99505390ad6d9; disposition round 4 reviewed at b365e72bcc6de760b36b40bcdabc35f1699c11ec"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: reviews
---

## Summary

Manually mapped each retained FR-043/044/045/046-AC-5 criterion and FR-050-AC-1..8 to test tags, procedures, and live checked-route code. TC-150 has broad strict reader, authentication, and writer coverage, but misses authenticated activation state, several exact source-locus negative cases, and producer-side limit/retry evidence.

## Verdict

**FAIL** — retained checked-route behaviors are not fully evidenced on the authenticated `/2` path.

## Coverage

- Manually performed AC-to-test and reverse code-to-spec review because `quoin matrix --repo . --json` (Quoin 0.29.1) emitted no stdout/stderr for 30 seconds and was interrupted (exit 130).
- FR-043-AC-1..14 and FR-044-AC-1..9 remain tagged to checked successor tests; FR-045-AC-1..6 map to TC-1279; FR-046-AC-5 maps to TC-1280. TC-122/123/125 and TC-127/128 retain their legacy scopes.
- FR-050-AC-1..8 have TC-150 tags and named procedures. Parameter substitutions, typed refusal dimensions, pre-position refusal, progress watermark preservation, strict cross-version reads, handoff files, determinism, mutation replay, and existing-directory refusal were inspected.
- No aggregate gate or merge was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | [QSL-1381/1384] TC-150 tests `evaluate_checked_with_progress_v2` only with `on origin`; no authenticated checked-route test proves per-instance captures are retained and not reevaluated through progress/replay/restoration. The implementation currently passes no retained-instance map. | tests/it/compiled_protocol_v2.rs:656-671, src/temporal.rs:214-223 |
| FND-002 | medium | [QSL-1384] FR-050-AC-5 and TC-150 claim added `/2` work is exactly bounded with a fresh retry, but the named test exercises only `read_v2_bytes`/`read_v2`. No test applies exact/one-short limits or retry to `native::admit_v2`, so producer-side binding/string/number/output charges are not demonstrated. | spec/functional/FR-050-publish-authenticated-temporal-artifacts.md:237, spec/test-cases/TC-150-preserve-authenticated-temporal-selections-on-checked-route.md:50-54, tests/it/compiled_protocol_v2.rs:1971-2040 |
| FND-003 | medium | [QSL-1384] TC-150's source-locus refusal test changes authority while retaining the source identity; it does not independently test a changed source identity, declaration name, or byte span. A matcher that ignored any of those exact-locus fields could pass the current negative case. | tests/it/compiled_protocol_v2.rs:826-856, src/temporal.rs:502-521 |
| FND-004 | medium | [QSL-1384] Authenticated progress is keyed only by declaration index, definition identity/revision, and clock fields. There is no successor test for two distinct checked clauses/packages sharing one ledger; their retained progress can alias. | src/temporal/progress.rs:23-41, tests/it/compiled_protocol_v2.rs:656-671 |

## Dispositions

Round 4, reviewed at head b365e72bcc6de760b36b40bcdabc35f1699c11ec.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The authenticated progress successor now asserts capture state is retained across replay and restoration, refusal, distinct clause state and eviction. after_excerpt: `assert_eq!(restored_assessment.capture_evaluations, 1); assert_eq!(restored_assessment.captures[0].value, "7");` |
| FND-002 | fixed | The producer successor now tests exact/one-short bounds and repeatable bytes, package and usage. after_excerpt: `let producer = native::admit_v2(proofs, selected, &tables.producer, Limits::default());` |
| FND-003 | fixed | The test now independently substitutes all four source-locus axes. after_excerpt: `("declaration byte span", &wrong_span),` |
| FND-004 | fixed | Distinct checked nodes now get separate checked-authenticated ledger keys under the same admitted binding. after_excerpt: `assert_ne!(first_node.identity(), second_node.identity());` |

## Final disposition

**PASS — clean at `b365e72bcc6de760b36b40bcdabc35f1699c11ec`.** Round 4 verified all findings in the pass-3 record as fixed and found no new findings. The original `## Verdict` and `## Findings` sections above preserve the pass-3 results; the round-4 outcomes are recorded in `## Dispositions`.
