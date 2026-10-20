---
id: SR-12218
title: "Gap analysis of QSL-567 E16c part against FR-228-AC-3, AC-4, FR-226, TC-671 and TC-673 (PR #1080)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c90b688f3a78524650762e8d052cdd585f14220e; spec/functional/FR-228-derive-memory-fairness-constraints.md, spec/functional/FR-226-*.md, spec/test-cases/TC-673-memory-fairness.md, spec/test-cases/TC-671-*.md, spec/decisions/ADR-025-weak-memory-models-for-parallel.md (MF-2 to MF-5); qsl-eval/src/fairness/, qsl-eval/tests/it/fairness_kernels.rs, qsl-replay/src/certificate/scc.rs, qsl-route/src/lib.rs, qsl-route/src/request.rs, qsl-route/tests/it/memory_models.rs, qsl-semantics/tests/it/temporal_fairness.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-228
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-226
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-673
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-671
    type: reviews
---
# SR-12218: Gap analysis of QSL-567 E16c part against FR-228-AC-3, AC-4, FR-226, TC-671 and TC-673 (PR #1080)

## Summary

Ticket: QSL-567 (E16c), partial. PR quire-spec-language#1080 at head c90b688f3. The audit covers the diff, FR-228-AC-3 and AC-4, FR-226-AC-1 to AC-3, TC-671 and TC-673. Plan completion: not assessed. QSpec FR-437 and FR-439 were read from a local read-only QSpec checkout and are cited by id only.

Criterion coverage:

- **FR-228-AC-3 (examined).** The test tagged `TC-673`/`FR-228-AC-3` checks the two SCCs the AC names: stale only is rejected, stale plus fresh is passed, and an SCC with no load owes nothing. The lasso test and the certificate test carry the same tags. The AC is covered as written. But the rule the AC (and ADR-025 MF-2, QSpec FR-437) states for an SCC is unsound, because it rejects an SCC that holds a fair sub-cycle with no load. See SR-12217 FND-001 and FND-001 here.
- **FR-228-AC-4 (examined).** Its second half, `fair weak flush` refusing `missing_declaration`/`missing-name`, is tested and tagged `TC-673` only. That is correct, because the first half (TP-1 verdicts equal with and without the memory constraints) is not built. No tag inflation.
- **FR-228-AC-1, AC-2 (context only).** Not in this PR. They are declared remainder in the PR body.
- **FR-226-AC-3 (examined).** `candidates_for_models` is tested with an EN-1-like descriptor advertising `sc, tso, ra` and an `sc`-only descriptor: `{EN-1}` for `tso` and `ra`, the empty set when the `sc`-only one is alone, and `{test candidate}` for `sc`. The test is tagged `TC-671` with no AC id. That is correct, because the AC speaks of manifests and of the item's resolved model, and neither is wired. The request writer still calls `candidates_for_fairness`, so the production candidate step never filters by model (FND-002).
- **FR-226-AC-2 (examined).** Not met. `descriptor_from_manifest` sets `memory_models` to empty for every manifest, so once routing is wired a manifest-registered EN-1 is no candidate for any weak item (FND-003). This is declared remainder: QSpec FR-331 has no manifest member for memory models yet.
- **FR-226-AC-1 (context only).** Declared remainder (spec gap).

There are no stubs or tautological tests in the diff. Every tag is on a fully covered AC or carries only the TC id.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | MAJOR. FR-228-AC-3 and ADR-025 MF-2 (following QSpec FR-437) define visibility fairness on an SCC as "for each pair loaded on some edge of the SCC, the SCC holds a fresh load". That form is not equivalent to the lasso form ("the loop holds a fresh load for every pair it loads"), and it is unsound: it rejects an SCC that contains a fair accepting sub-cycle that avoids the load edges. The code implements the SCC wording faithfully (SR-12217 FND-001), so the test passes while the verdict can be wrong. The spec (QSpec FR-437 upstream, ADR-025 MF-2, FR-228-AC-3) needs an edge-refinement form, and a test whose SCC holds a sub-cycle with no load and must be passed. | spec/functional/FR-228-derive-memory-fairness-constraints.md:81; spec/decisions/ADR-025-weak-memory-models-for-parallel.md:253; qsl-eval/src/fairness/refine.rs:185-195 |
| FND-002 | medium | FR-226 Behavior ("the route candidate step SHALL include ... only backends whose manifest advertises the subject's resolved model") is not reached in production. `candidates_of` in the request writer calls `candidates_for_fairness`, which passes no resolved models, so a weak item is routed to every capable backend. This is declared remainder (resolved models belong to E16a), but the kernel has no caller. | qsl-route/src/request.rs:506-512 |
| FND-003 | medium | `descriptor_from_manifest` hard-codes `memory_models: BTreeSet::new()`. When FND-002 is wired, every manifest-registered backend, EN-1 included, loses every weak item, and FR-226-AC-2 stays unmet. This is declared remainder, blocked on QSpec FR-331/FR-439 naming the manifest member. | qsl-route/src/lib.rs:232-236 |
| FND-004 | medium | PR #1080 is a declared partial, but its QSL-567 attachment has `linkKind: closes`. Merging it would move QSL-567 to Done while FR-224, FR-226-AC-1/2, FR-227, FR-228-AC-1/2 and the first half of FR-228-AC-4 are still open. The lead keeps QSL-567 open, or files the remainder, at merge. | QSL-567 attachment (PR #1080) |

## Verdict

**FAIL: not mergeable, one major (FND-001).** FR-228-AC-3 is covered as written, but the AC's SCC rule is unsound, and the code inherits the defect. Spec and code have to be fixed together, and a test with a sub-cycle that avoids the loads has to be added. FR-226-AC-3's candidate kernel is correct and correctly tagged only `TC-671`, and FR-228-AC-4's flush refusal is correctly tagged only `TC-673`. FND-002 to FND-004 are declared-remainder or ticket-state items and do not block.
