---
id: SR-16274
title: "spec-review of FR-315 monitor items: TP-2 on each, bounded TP-2 formulas, guarded clauses and per-instance monitors (quire-spec-language PR 1354)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8025e2d5; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md; spec/test-cases/TC-1201-smt-monitor-items-guarded-and-multi-instance.md; spec/test-cases/TC-900-the-smt-lib-encoding-is-canonical.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1201
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: references
---

# Spec review of FR-315 monitor items (quire-spec-language PR 1354)

## Summary

Ticket: STD-407. This is one spec-review pass over
`git diff origin/main...8025e2d5`. It covers the FR-315 monitor-item
encoding, rules 4, 4a, 4b and 4c, the TP-2 distinctness form, the refusal
table, FR-315-AC-5, AC-21, AC-23 and AC-33 to AC-39, TC-1201 and TC-900
step 5. The oracles were ADR-018 (EN-2, EN-3, DS-3, CX-1, CX-2, UE-3,
V-2), FR-338 "Guarded `on each` activation", FR-126 (the guarded bullet,
AC-2 and AC-16), FR-314 Behavior and FR-315 on main.

Checked and sound:

- The past-the-end flags. The step assertion makes `h` monotone, and a
  copy past the end reads only the end letter. So `(not (and (= h<k> h<j>)
  (or h<k> (and Q...)) M...))` compares the whole future-relevant state,
  and the simple-path shortcut argument of the step case still holds.
- The cover position `p + h` for `on each`. It is at least `a + h` for the
  failing activation `a`, so it meets CX-1. It loses only completeness.
- Per-instance monitors. They share one tabulation, and no letter is
  shared between instances.
- The guarded reading (condition bit 0, the end letter reads no condition)
  matches FR-338 and FR-126.
- The rule-4 bad prefix matches CX-1's infinite-trace shape.
- "TP-4 has no proof query" already holds on main. Row 1 has it, ADR-018
  EN-3 lists only TP-1, TP-3 and TP-2 `on each`, and DS-3 gives EN-2's
  searches to the backend. So it is not a narrowing.
- The several-instances removal and the nested TP-2 restoration are the
  ticket's scope.

The AC-33, AC-34, AC-35, AC-37 (guarded TP-1 part) and AC-38/AC-39
vectors were checked by hand against the monitor semantics, and they hold.
No deferral or narrowing language was found.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | An `on origin` monitor item can be proved below its horizon. Row 3 refuses `query-kind` for `Unrolling{depth < u}` only for the direct `eventually[0,u]` shape. The new monitor items get no such refusal, and FR-314 has no check that the depth reaches the horizon. Take `always[0,3] eventually[0,1] holds(c.value = 3)`, horizon 4, which AC-35 shows is refuted. Its `Unrolling{depth: 0}` is unsatisfiable, because the monitor cannot reject at position 0. A `BoundedComplete{depth: 0}` certificate refuting that query matches FR-314's byte-for-byte rebuild and settles `proved`, `Certified`. The result is unsat=proved for a violated claim. Fix: extend row 3 to refuse `Unrolling{depth}` with `depth < h` for every `on origin` monitor item, or have FR-314 reject a `BoundedComplete` basis below the item's horizon, with an AC either way. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:102, :500-504; spec/functional/FR-314-check-an-smt-proof-certificate.md:150-154 | missing-requirement |
| FND-002 | high | The guarded false-extension vectors cannot be encoded. AC-36, AC-37 (third part) and TC-1201 steps 4 and 5 use the condition `6 mod (2 - c.value) >= 0`, whose divisor is not a literal. Row 15 refuses a non-literal divisor `out-of-fragment`, so the encoder writes no script. The stated script and model, and the undefined-condition path they are meant to test, cannot be produced. Fix: use a condition whose undefinedness is `QF_LIA`, for example a range coercion as in AC-4/AC-31, or a `value` read of an absent optional. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:116, :1019-1020; spec/test-cases/TC-1201-smt-monitor-items-guarded-and-multi-instance.md:28-36 | wrong-requirement |
| FND-003 | medium | Rule 4c's attribution order contradicts rule 4a. Rule 4a reads a state-only failure (a definedness failure) before a monitor violation at `p`. Rule 4c names the instance of the first disjunct in disjunct order, and at each position the violations come before the definedness failures. Suppose instance 1's monitor rejects at `p` and instance 0's atom is undefined there. CG then builds instance 0's UE-3 prefix but names `over_binding` instance 1, and FR-128's replay cannot reproduce that. Fix: have 4c attribute to the instance of the failure that rule 4a/4b actually reads. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:311-315, :337-343, :600-606 | wrong-requirement |
| FND-004 | medium | The Monitors bullet cites FR-338 "Guarded TP-4 clause". That section is not on main; only open #1310 (STD-368) adds it. #1354 neither contains nor depends on #1310, so on merge the reference dangles. Fix: cite what main states (FR-126 "Liveness (TP-4)" and ADR-018 DS-3), or record the merge order after #1310. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:204-207 | wrong-requirement |
| FND-005 | low | TC-1201 does not say how the `on each` and guarded requests are built. FR-126-AC-16 records that S3/S4 refuse an `on each` clause `NotEmitted` (FR-337, QSL-926), so its test builds the checked package itself. TC-1201 steps 1 to 5 encode "over `Counter`" as if from source. Fix: state the test-built package, as FR-126-AC-16 does. | spec/test-cases/TC-1201-smt-monitor-items-guarded-and-multi-instance.md:14-36; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:465 | missing-requirement |

## Verdict

**Not mergeable.** FND-001 is a soundness hole on the restored `on origin`
monitor items. FND-002 makes two ACs unrealizable. The rest of the
encoding is sound as reviewed.

Ids: FR-315-AC-33 to AC-39 and TC-1201 appear on no other open PR head.

Merge: the PR body says "No open PR touches FR-315", but #1356 does.
`git merge-tree` of #1354 with #1356 conflicts only at the FR-315 AC
table, where both append after AC-32; keep both PRs' rows in id order.
#1354 merges cleanly with #1312, #1310 and #1342.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The new FR-314 bullet sets a rule for FR-127's settlement map: a `BoundedComplete` result with no certificate that is below the horizon, or not for a TP-2 `on origin` item, settles `inconclusive` `CertificateRejected{ShapeMismatch}`. FR-127 itself is not edited, and open #1312 rewrites that path: `settle_smt_proof` "with no certificate SHALL run no check" and settles `Uncertified`. Once both land, FR-127 says the uncertified path checks nothing while FR-314 requires the horizon check there. Fix: state the horizon rule in FR-127, or in `settle_smt_proof` if #1312 lands first, and record the merge order with #1312. | spec/functional/FR-314-check-an-smt-proof-certificate.md (new ShapeMismatch bullet); #1312 spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit 97ba934d1 (head 49de7405). Row 3 refuses `query-kind` for `Unrolling{depth < h}` for every `on origin` monitor item. FR-314 rejects `ShapeMismatch` at `Query { part: Unrolling }`, before encoding, any `BoundedComplete` basis below the horizon or for an item that is not TP-2 `on origin`. Hand-checked: AC-35 depths 0, 1 and 3 are below horizon 4 and refused. FR-314-AC-8 depth 0 (h 4), `always[0,3]` depth 2 (h 3) and the TP-1 basis are rejected; AC-1 depth 3 is still accepted. |
| FND-002 | fixed | The condition is now `c.value + 2 = c.value`. FR-104 coerces the sum into `Int[0, 3]` (as AC-31), so it is defined at 0 and 1, where it is false, and undefined at 2. The printed term is linear. The clause never activates. Hand-checked: at depth 2 the only failure is the condition at position 2, so the prefix is 0, 1, 2 and the result is UndefinedEvaluation at 2. `inc`'s postcondition never fails on the path. |
| FND-003 | fixed | Rule 4c now names the instance of the failure that rules 3, 4, 4a and 4b read. A definedness failure is attributed to its own instance and never to a violation's. |
| FND-004 | fixed | The citation of FR-338 "Guarded TP-4 clause" is gone. The bullet now cites ADR-018 DS-3 and FR-126's letter-seam reading, which are on main. |
| FND-005 | fixed | TC-1201 now states that `on each` and guarded packages are test-built, in FR-370's shape, as FR-126-AC-16 does. |
