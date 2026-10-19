---
id: SR-12186
title: "Gap analysis of quire-spec-language PR 1026: FR-339 partition check against TC-526 and its tagged tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d839169621a52beaf1ac2b76b8728607f0e35fd3; PR 1026: FR-339-AC-1, FR-339-AC-2, FR-339 Behavior, TC-526, ADR-019 AM-7 against qsl-replay/src/certificate/scc.rs"
review_set: subset
---

## Summary

Ticket: QSL-531 (partial, E8b). This is a planless gap analysis of the PR diff.
Plan completion: not assessed. `quire matrix --format tsv` (quire 0.36.2) was run
on the reviewed head. FR-339-AC-1 is `untagged`. FR-339-AC-2 is `tagged`, with
one binder (`qsl-replay/src/certificate/scc.rs:586`). The `TC-526`-only tags
credit no criterion.

- FR-339-AC-1 is correctly untagged. The AC's subject is `check_components`
  accepting EN-1's certificate, closure included. Neither exists yet (declared
  remainder, E9 / QSL-530). `the_weak_each_partition_is_accepted` checks the
  partition half of that vector over a hand-built product: 15 states, the
  `q0` layer `MissingAcceptance{0}`, and the two accepting layers
  `UnfairWeak{upd(b)}`. Its `TC-526` tag is a fair partial binding of TC-526
  step 1.
- FR-339-AC-2: `a_tampered_partition_is_rejected_at_its_locus` carries all
  four vectors, and each can fail on that vector's behaviour.
  - `UnfairWeak{upd(a)}` on `{(*, 0, q1)}`. Without the taken-inside check
    the certificate would be accepted. The test asserts `WitnessFails` at
    `taken[1].states[0]`, the component's first state.
  - `rotate_left(1)` gives `[{(*,0,q1)}, {(*,1,q1)}, q0]`, so both accepting
    components come before the `q0` layer that reaches them. Without the edge
    check every witness holds, so the certificate would be accepted. The test
    asserts `BackwardEdge` at `(0,0,q0)`, the source of the first edge
    (`upd(a)` into `(1,0,q1)`) in insertion order.
  - A state of `{(*,0,q1)}` pushed into `{(*,1,q1)}` is rejected
    `NotPartition` at that state.
  - `Trivial` over a one-state stutter self-loop. The example has no
    self-edge, so a separate graph is needed. Without the self-edge test it
    would be accepted. The test asserts `WitnessFails`.
- Binding table: see `bindings` in the posted comment. Three `TC-526` bindings
  are wrong (FND-001 to FND-003).
- Out of AC scope, still verified: uncovered and foreign NotPartition loci,
  the AM-7 refinement (accepted, keeps a non-quiet state, a taken strong
  constraint, a weak constraint named in `UnfairStrong`), and pre-cancelled
  `Stopped(Cancelled{Requested})`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Over-tag: `an_uncovered_or_foreign_state_is_not_a_partition` carries `TC-526`. TC-526's procedure (steps 1 and 2, scope FR-339-AC-1 and AC-2) has no uncovered or foreign state vector, so this test cannot fail on any TC-526 step. It verifies FR-339 Behavior's NotPartition rule beyond AC-2. Drop the tag, or add the vector to TC-526 first. | qsl-replay/src/certificate/scc.rs:632-650; spec/test-cases/TC-526-an-en-1-component-certificate-is-checked.md |
| FND-002 | medium | Over-tag: `a_strong_refinement_is_checked_within_its_component` carries `TC-526`. `UnfairStrong` is ADR-019 AM-7, whose vectors TC-531 steps 4 and 5 own (FR-130-AC-4/5, through `check_components`). TC-526 names no strong witness, so the test cannot fail on TC-526's behaviour. Drop the tag. It can take `TC-531`/`FR-130-AC-4` only once those vectors run through `check_components`. | qsl-replay/src/certificate/scc.rs:657-710; spec/test-cases/TC-531-strong-fairness-on-the-explicit-state-product.md |
| FND-003 | medium | Over-tag: `a_cancelled_check_stops_with_its_source` carries `TC-526`. TC-526 has no cancellation step, and the behaviour is FR-339 Behavior's `Stopped`, which no AC states. Drop the tag. | qsl-replay/src/certificate/scc.rs:714-733 |
| FND-004 | medium | Two soundness-critical witness rules have no negative vector. Deleting either check leaves every test green while the checker accepts a bogus certificate. (a) `UnfairWeak`'s "enabled at every state" half: every `UnfairWeak` in the tests names a constraint enabled everywhere, so removing `enabled_everywhere` goes undetected. (b) `MissingAcceptance`'s disjointness: the only use is the passing `q0` layer, so replacing the check with `true` goes undetected (as does an out-of-range `set`). Add the example with `{(*,0,q1)}` witnessed `MissingAcceptance{0}` (it holds accepting states), and a product where the named weak constraint is disabled at one component state. | qsl-replay/src/certificate/scc.rs:331-340,385-394 |
| FND-005 | low | The FR-339-AC-2 tag makes the matrix show AC-2 as `tagged` while its subject, `check_components`, does not exist yet. The PR body's remainder item 4 (the AC-2 vectors through `check_components`) is then invisible as a matrix gap. A future `check_components` that never calls `check_partition` would still pass AC-1 and leave AC-2 apparently covered. The binding is correct for what it tests. The remainder ticket must carry item 4 explicitly. | qsl-replay/src/certificate/scc.rs:584; spec/functional/FR-339-check-an-en-1-component-certificate.md:110 |
| FND-006 | low | FR-339 Behavior branches with no test: an empty component (NotPartition at the fallback locus), a BackwardEdge inside a `sub` level, more than one `UnfairStrong` refinement or nesting below one level (the explicit stack's depth-first order), and an out-of-range `UnfairWeak` index. All are simple and read correct, but none is pinned. | qsl-replay/src/certificate/scc.rs:197-210,255-260,298-311,334-340 |

## Verdict

PASS with three over-tags and one coverage gap, all medium. No major. AC-1 is
honestly untagged. The AC-2 binding is correct, and each of its four vectors
fails if the rule it exercises is removed. The `TC-526` tag on the uncovered,
foreign, strong-refinement and cancellation tests over-claims TC-526's
procedure. It credits no AC in the matrix, so no coverage figure is inflated.
FND-004 is the one that matters most for a certificate checker. The
weak-enabledness and missing-acceptance rules are soundness-critical, and no
test fails if they are deleted. Nothing blocks merge.
