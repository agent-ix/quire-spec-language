---
id: SR-20261104231203
title: "spec-review of QSL-1667 post-draw FreeDelays (quire-spec-language PR 2325)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language PR 2325 head b3d90793c51500e2f5edd5c302ff654b0e4db409; git diff origin/main...HEAD (ADR-028, FR-091, FR-093, FR-196, FR-202, FR-204, TC-631, TC-637, TC-642)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-202
    type: references
---

# Spec review of QSL-1667 post-draw FreeDelays (quire-spec-language PR 2325)

## Summary

Ticket: QSL-1667 (with the QSL-1664 coordination edits to FR-091-OQ-15 and FR-093). One pass, with
EARS, integrity, failure-domain and criterion-strength checks done by hand. `quire validate` (quire
0.36.2, engine 0.50.2) exits 0 on all nine files; `git diff --check` is clean.

Re-derived by hand:

- Parity proof (FR-204 Behavior). The FreeDelays action set is the product over rows of the per-row
  assignment sets, the one-step operator of every admitted timed objective (TA-4: finite-horizon
  deadline reach, unbounded Reach/until, expected elapsed) is linear in the successor distribution,
  and every `P(g) > 0`, so `opt_f Σ_g P(g)·Q(g,f(g)) = Σ_g P(g)·opt_a Q(g,a)` holds for max and min.
  The action form's Bellman operator equals the composed chance-then-decision operator of the SCH-2
  form, so least/greatest fixpoints (Reach min/max, expected reward) agree too. The randomized-policy
  remark holds (the outcome law depends only on per-row marginals). The reopen conditions are real
  (a nonlinear one-step operator; enumeration size). The value claims are sound; the witness
  tie-break is not (FND-001).
- `Detour` (FR-204-AC-10): Q(1,1)=1/2+1/2·1/2=3/4, Q(1,2)=Q(1,3)=1, Q(3,1)=1/2, Q(3,2)=Q(3,3)=0.
  Post-draw max (1+1/2)/2=3/4, min (3/4+0)/2=3/8; pre-draw B=1: 5/8, B=2: 1/2, B=3: 1/2, so max 5/8,
  min 1/2. `<= 0.7` and `>= 0.45` separate the forms. The least attaining rows are [1]→2, [3]→1 (max)
  and [1]→1, [3]→2 (min). The max path set is {A@1: 1/2, B@1·G@2: 1/4} = 3/4 > 7/10; it is prefix-free;
  the merged first steps come from one row each, as FR-202-AC-8 states.
- FR-204-AC-11: (B,C) in [1,3]^2 gives 9 one-row actions; the max of 1 has least row (1,2); the min of 0
  has least row (2,1), since every B=1 row ties or wins. AC-6/7/8 values and verdicts are unchanged
  (0/1, 1/2 and 0, 3/4 and 1/4). AC-8's prose correction ("delay 1 or 2 wins") is right, and its witness
  rows moved to AC-9, so nothing was weakened.
- FR-196: the canonical body `["free-delays",[[given,free],...]]` uses numeric row order (AC-14: 9
  before 10), the refusals are complete, `pending` stays null, no new ProductKey member is added, and
  merged outcome edges are consistent with ADR-028 WS-1/TA-6 and FR-202's merged-step replay.
- Stale-text grep over spec/: `WitnessChoice` survives only in the FR-204 rulings row that records its
  removal. Every `WorkloadResolve` reference is either all-free (FR-196, FR-204-AC-5) or states its
  absence from mixed states. ADR-028 WS-2 is stale (FND-003).
- Traceability: each new AC (FR-196-AC-14, FR-202-AC-8, FR-204-AC-10/11) has a TC step with expected
  results, and every TC scope line names it. The rulings table is labelled as relayed lead defaults
  with a veto and per-row reopen conditions. No ceremony was added.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The per-row "least attaining assignment" witness rule contradicts FR-198's policy extraction. FR-198 picks an action by canonical transition order (JCS bytes, where "10" sorts before "9") and, for a maximum, by the MEC exit rule. For unbounded Reach (TA-4), a free `B` that resets `x` and loops back to `Idle` against given `A ~ {2}` gives row options B=1 (loop) and B=2 (tie, 1/2 to target). Both attain the upper iterate 1, the least is B=1, and that witness never reaches the target, so replay settles inconclusive. Fix: the witness takes FR-198's chosen action. Define FreeDelays canonical action order numerically (row by row) so that "least" and FR-198's order agree, and let the MEC rule override within rows. | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:150; spec/functional/FR-202-replay-a-probabilistic-witness.md:96; spec/functional/FR-198 (witness policy bullets) | wrong-requirement |
| FND-002 | medium | One FreeDelays action per function is `(w^k)^s` actions (w window size, k free identities, s support size), each with built outcome edges counted against `max_transitions`: 2 given values × 10 free units gives 100 actions, and 4 × 10 × 2 free identities gives 10^8. The factorization the PR proves lets EN-5 optimize per row (s × w^k) and materialize only the policy's action. As written, the builder must enumerate every action, so ordinary models hit LM-1 limits that the SCH-2 form would not. Fix: state that the product's value iteration uses the row-factored operator and that only the row edges count against `max_transitions`. Keep enumeration (and the "nine actions" listing ACs) as the semantic choice set, not as built edges. Alternatively, tighten the reopen condition to a concrete size. | spec/functional/FR-196-build-the-probabilistic-product.md:246-256; spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:121,206 | wrong-requirement |
| FND-003 | medium | ADR-028 WS-2 still says the witness scheduler is "absent under a workload", but this PR makes it load-bearing there. WS-1 and TA-6 (edited here), FR-202's `scheduler` comment, FR-202-AC-8 and FR-204-AC-6..11 all require a scheduler with FreeDelays entries under a workload. Fix: WS-2 should say "absent under a workload with no free delay". | spec/decisions/ADR-028-exact-probabilistic-engine.md:234 | wrong-requirement |
| FND-004 | low | FR-091's "Integer power spelling" body still says the production "extends QSpec `shared-grammar.md`'s value expression", while the edited OQ-15 now says QSL takes QSpec's own production. OQ-15 also claims parity without noting that QSL caps the exponent literal at i128 ("within the integer-literal range"), while QSpec FR-322 admits any magnitude. Fix: reword line 388 to "is QSpec's complete-V1 `power` production", and add the i128 ceiling as QSL's stated narrowing (a QSL exponent above i128 refuses `IntegerOutsideI128`). Every QSpec vector, including 2^64, fits. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:388,404,1310 | wrong-requirement |
| FND-005 | low | FR-093 and FR-091-OQ-15 cite QSpec FR-322-AC-383 and TC-187 QP-01..QP-10, which exist only on QSpec PR 622. Merging this PR first leaves dangling citations. Fix: merge after QSpec PR 622, or hold the two citation edits. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:250; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1310 | wrong-requirement |

## Verdict

CONDITIONAL. The FreeDelays representation, the parity proof (value level), both separating vectors,
AC-11, FR-196's canonical body and its refusals, and the WitnessChoice removal are correct and
hand-checked. FND-001 must be fixed before merge, because the witness tie-break can emit a
non-replayable maximum witness. FND-002 and FND-003 should be fixed in this PR. FND-004 is wording.
FND-005 is merge ordering after QSpec PR 622.

## Dispositions (pass 1)

| FND | Outcome | Evidence |
| --- | --- | --- |
| FND-001 | fixed | FR-204 Behavior: the witness entry is the action chosen by FR-198's witness policy rules (the MEC exit rule first for a maximum, then the attaining action with ties broken by canonical transition order). FR-196 defines the canonical order between two `FreeDelays` actions numerically, row by row, so the least attaining action is the least attaining assignment of each row outside the MEC case. FR-202 witness production now cites FR-198's rules instead of "least attaining". New FR-204-AC-12 (`Loop`, TC-642 step 8) is the reviewer's counter-example: the witness is `B` delay 2, the exit. AC-6 to AC-11 re-checked: none of their race states lies in an end component (`Done` is absorbing and no edge returns to `Idle`), so their rows are unchanged. |
| FND-002 | fixed | FR-196: the set of all functions is the semantic choice set. The product builds row-choice edges per row and assignment (`s · w^k`), each with `P(g)` folded in, with one-row choice bodies that are not enabled actions by themselves. Only built row-choice edges count against `max_transitions`. FR-204 Behavior: EN-5 evaluates through the row-factored operator, and graph precomputations read an action's support as the union of its row choices' supports. Ruling (a) now reopens only for a nonlinear objective; size is no longer a reason, because the SCH-2 form builds the same `s · w^k`. The "nine actions" ACs list the semantic choice set. |
| FND-003 | fixed | ADR-028 WS-2 now reads "absent under a workload with no free delay". |
| FND-004 | fixed | FR-091 "Integer power spelling" now says S1 parses by QSpec's own complete-V1 `power` production. FR-091-OQ-15 states the conformant narrowing: an exponent literal outside i128 refuses `IntegerOutsideI128`, while QSpec admits any magnitude, and every QSpec vector, 2^64 included, fits. |
| FND-005 | deferred to merge order | Coordinator handles it: QSpec PR 622 merges before this PR. |
