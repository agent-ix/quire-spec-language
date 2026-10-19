---
id: SR-12212
title: "Gap analysis of quire-spec-language PR #1066: each fairness on the annotated quotient, reduced liveness"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0a3c2a78aae79f00dfca2106c36fa09d6229d83a; FR-153-AC-1..4, FR-161-AC-1..3, ADR-021 AQ-1..AQ-8, EI-6, RV-1, RV-7, PT-2 against qsl-analyze/src/model_check/quotient.rs (unit tests), qsl-analyze/tests/it/quotient.rs, qsl-analyze/tests/it/toggle_world.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1066

## Summary

Ticket: QSL-541 (Implements FR-153, FR-161 `each`). A planless gap analysis:
plan completion was not assessed. It is scoped to the PR diff. Each
`#[trace]` was read against its AC. The expected values (18 and 30 product
states, the six `va = 0` states, the generator `(b c)`, the closures) come from
ADR-021 §7.1, not from the engine. Counterexamples are read back by
re-executing them through `ModelSystem::successors`.

Examined:

- FR-153-AC-1 (TC-570, unit and integration): entry, tree word, generator,
  the `W_taken` closure, empty `W_disabled`, the weak failure, 30 product
  states, and equal generators when repeated. The AC says `Holds`, and the
  engine returns `HoldsReduced`, which is RV-1's settlement. Clean.
- FR-153-AC-2 (TC-570): 18 states, one SCC, generator `(b c)`, a `W_taken`
  closure that covers `O`, and a pass. The AQ-7 loop takes all three
  identities concretely and returns to the entry. Clean.
- FR-153-AC-3 (TC-571): the weak run is `Violated` with a flip-only loop, and
  the strong run is `HoldsReduced`. On the quotient, `R` equals the
  fire-enabling states. The claim is bound to a watcher in a non-symmetric
  population, standing in for AC-3's "no `over`" claim, which keeps the full
  group. This is documented in toggle_world.rs. Clean as tagged, but see
  FND-001 for the binding the substitution avoids.
- FR-153-AC-4: not tagged. Declared partial in quotient.rs's header.
- FR-161-AC-1, FR-161-AC-2: not tagged, no test.
- FR-161-AC-3: not tagged. The concretised `each` loop is re-executed; FR-128
  replay is declared waiting.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | No test covers strong fairness under symmetry when the bound key's stabiliser is trivial, and that path is unsound (SR-12211 FND-001). AC-3's test binds `over` to a separate non-symmetric `watchers` population, so its one instance keeps the full group `[a, b]` and the quotient path runs. Binding over `x: M::Toggle` instead gives orbit `[a, b]` with a trivial stabiliser. The product then goes to the weak-only `FairModel`, and the run returns `Violated` (reproduced). PT-2's `each` row and RV-7 cover every binding, so FR-153's strong `each` behaviour, and FR-153-AC-4's agreement, are untested for the commonest binding shape. Add the trivial-stabiliser strong run with the fix. | qsl-analyze/tests/it/quotient.rs:214-262, qsl-analyze/tests/it/toggle_world.rs:9-14 |
| FND-002 | medium | FR-153-AC-4 is not met. Its meter clause is covered by untagged tests. Its agreement clause cannot be met for AC-3, because the unreduced engine still refuses strong fairness (`LivenessRefusal::StrongFairness`), and nothing compares AC-1's or AC-2's reduced verdict to an unreduced verdict as AC-4 states it. AC-1 compares against `ComponentCertificateNotBuilt`, not a verdict. The gap is declared, but the ticket's acceptance is "the tests in this PR back FR-153", so AC-4 stays open. It needs a named follow-up (the unreduced strong filter, ADR-019 / QSL-365) recorded on the ticket. | qsl-analyze/tests/it/quotient.rs:14-21 |
| FND-003 | medium | FR-161 "Loop closure" with more than one pass is implemented (`lasso`'s repeat loop and `order`) but no test exercises it. FR-161-AC-2's two-pass `Token` loop is the AC for it. FR-161-AC-1's `whole` loop under symmetry (the greedy branch of `quotient_phase`) has no test either. The ticket scopes FR-161 to `each`, so these ACs may belong elsewhere, but this PR ships the code they verify: a wrong pass count or closure check would go undetected. | qsl-analyze/src/model_check/liveness.rs:1765-1814, qsl-analyze/src/model_check/liveness.rs:1611-1631 |
| FND-004 | medium | FR-161-AC-3 is half covered. The concretised `each` loop is re-executed (AC-2's test), but FR-128 replay, its `each` re-check (ADR-019 SR-8) and `reproduced-with-evaluated-witness` are not run. The test header declares the wait on FR-128, but no ticket is named for it. | qsl-analyze/tests/it/quotient.rs:19-21 |

## Verdict

One major (FND-001). It is the test-side face of SR-12211 FND-001: the
tagged AC-3 test passes only because its fixture avoids the trivial-stabiliser
binding, where the engine returns a false refutation. AC-1, AC-2 and AC-3 are
otherwise backed by tests that would fail if the behaviour broke. FND-002 to
FND-004 are declared or scope-adjacent gaps, recorded as mediums and not
blocking.

## Dispositions
